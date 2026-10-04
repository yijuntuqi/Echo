//! Local text embeddings for semantic memory search.
//!
//! The model is downloaded on first run and cached in the app data directory.
//! Until it lands, memory search falls back to keyword matching so the feature
//! degrades rather than disappearing.
//!
//! Inference is Candle on CPU (pure Rust): `BertModel` loads the downloaded
//! `model.safetensors`, and HF's `tokenizers` consumes the downloaded
//! `tokenizer.json`. The session is loaded lazily on the first `encode`, then
//! cached for the life of the process.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use thiserror::Error;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Error)]
pub enum EmbeddingError {
    #[error("model not downloaded yet: {0}")]
    NotDownloaded(String),
    #[error("download failed: {0}")]
    Download(String),
    #[error("inference failed: {0}")]
    Inference(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Dimensionality of `bge-small-zh-v1.5`.
///
/// 512, not 384: 384 is the *English* small model (bge-small-en-v1.5). The
/// Chinese small model has hidden_size 512 and CLS-pools its last hidden
/// state without a projection head, so the output dim equals 512. The vector
/// table in `db.rs` is sized from this constant — they must stay in sync.
pub const EMBEDDING_DIM: usize = 512;

pub const MODEL_ID: &str = "BAAI/bge-small-zh-v1.5";
/// Mirror first: the canonical host is slow or blocked on some networks.
pub const MODEL_BASE_URL: &str = "https://hf-mirror.com";

/// bge-zh-v1.5 was trained with this instruction prepended to *queries*
/// (never to stored passages) for retrieval tasks; prepending it measurably
/// improves recall.
const QUERY_INSTRUCTION: &str = "为这个句子生成表示以用于检索相关文章：";

/// BERT's position budget; inputs longer than this cannot be forwarded.
const MAX_TOKENS: usize = 512;

/// A loaded model, kept for the process lifetime.
struct Session {
    tokenizer: tokenizers::Tokenizer,
    model: BertModel,
    device: Device,
}

pub struct EmbeddingService {
    dir: PathBuf,
    base_url: String,
    /// Lazily initialised on the first encode; `None` until the model files
    /// are on disk. Locking loads the model at most once, and a concurrent
    /// encode simply waits out that load.
    session: Mutex<Option<Session>>,
}

impl EmbeddingService {
    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            base_url: MODEL_BASE_URL.into(),
            session: Mutex::new(None),
        }
    }

    /// Override the mirror, for tests that serve the files locally.
    pub fn with_base_url(dir: PathBuf, base_url: &str) -> Self {
        Self {
            dir,
            base_url: base_url.into(),
            session: Mutex::new(None),
        }
    }

    pub fn is_ready(&self) -> bool {
        ["config.json", "tokenizer.json", "model.safetensors"]
            .iter()
            .all(|f| self.dir.join(f).exists())
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Download the model files if they are not already cached.
    ///
    /// Streams each file, reporting progress as
    /// `(filename, downloaded_bytes, total_bytes)` per chunk so the UI can
    /// show a determinate bar. When the server omits `Content-Length`, the
    /// running total stands in so the event shape stays valid.
    ///
    /// Files land atomically: streamed into a `.part` sibling, renamed only
    /// once complete. Any failure (bad status, mid-stream error) removes the
    /// partial file, leaving nothing that could later look "ready".
    pub async fn ensure_model(
        &self,
        progress: tokio::sync::mpsc::Sender<(String, u64, u64)>,
    ) -> Result<(), EmbeddingError> {
        if self.is_ready() {
            return Ok(());
        }
        std::fs::create_dir_all(&self.dir)?;

        for file in ["config.json", "tokenizer.json", "model.safetensors"] {
            let url = format!("{}/{MODEL_ID}/resolve/main/{file}", self.base_url);
            let dest = self.dir.join(file);
            // Write to a temp name so an interrupted download never leaves a
            // truncated file that later looks "ready".
            let tmp = dest.with_extension("part");

            let mut resp = reqwest::get(&url)
                .await
                .map_err(|e| EmbeddingError::Download(format!("{file}: {e}")))?;
            if !resp.status().is_success() {
                return Err(EmbeddingError::Download(format!(
                    "{file}: HTTP {}",
                    resp.status()
                )));
            }
            let total = resp.content_length();

            let mut out = tokio::fs::File::create(&tmp).await?;
            let mut downloaded = 0u64;
            loop {
                match resp.chunk().await {
                    Ok(Some(chunk)) => {
                        if chunk.is_empty() {
                            continue;
                        }
                        out.write_all(&chunk).await?;
                        downloaded += chunk.len() as u64;
                        let _ = progress
                            .send((file.to_string(), downloaded, total.unwrap_or(downloaded)))
                            .await;
                    }
                    Ok(None) => break,
                    Err(e) => {
                        drop(out);
                        let _ = tokio::fs::remove_file(&tmp).await;
                        return Err(EmbeddingError::Download(format!("{file}: {e}")));
                    }
                }
            }
            out.flush().await?;
            drop(out);
            tokio::fs::rename(&tmp, &dest).await?;
        }
        Ok(())
    }

    /// Encode a stored memory (passage side — no retrieval instruction).
    ///
    /// Loads the model on first call (~1s), then caches it. Synchronous CPU
    /// inference of this 24M-parameter model takes a few tens of
    /// milliseconds — cheap enough that callers can use it inline.
    pub fn encode(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        self.encode_inner(text)
    }

    /// Load the inference session if the model files are ready. A no-op once
    /// loaded; callers use it to move the one-off model load out of the first
    /// real encode (e.g. right after the download finishes).
    pub fn warm_up(&self) {
        if !self.is_ready() {
            return;
        }
        if let Ok(mut guard) = self.session.lock() {
            if guard.is_none() {
                *guard = self.load_session().ok();
            }
        }
    }

    /// Encode a search query, with the retrieval instruction bge-zh-v1.5
    /// expects on the query side. Pair with [`Self::encode`], never encode
    /// both sides the same way.
    pub fn encode_query(&self, query: &str) -> Result<Vec<f32>, EmbeddingError> {
        let prefixed = format!("{QUERY_INSTRUCTION}{query}");
        self.encode_inner(&prefixed)
    }

    /// Never returns a zero vector on failure: a silent zero would poison
    /// every similarity search with confident nonsense, so errors propagate.
    fn encode_inner(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        if !self.is_ready() {
            return Err(EmbeddingError::NotDownloaded(self.dir.display().to_string()));
        }

        let mut guard = self
            .session
            .lock()
            .map_err(|_| EmbeddingError::Inference("session lock poisoned".into()))?;
        if guard.is_none() {
            *guard = Some(self.load_session()?);
        }
        let session = guard.as_ref().expect("session just initialised");

        let enc = session
            .tokenizer
            .encode(text, true)
            .map_err(|e| EmbeddingError::Inference(format!("tokenize: {e}")))?;
        let mut ids = enc.get_ids().to_vec();
        let mut types = enc.get_type_ids().to_vec();
        if ids.is_empty() {
            return Err(EmbeddingError::Inference("empty input".into()));
        }
        ids.truncate(MAX_TOKENS);
        types.truncate(MAX_TOKENS);

        let ct = |r: candle_core::Result<Tensor>| {
            r.map_err(|e| EmbeddingError::Inference(e.to_string()))
        };
        // Batch of one: (1, seq) ids and type ids, exactly what BERT expects.
        let ids = ct(Tensor::new(&ids[..], &session.device))?.unsqueeze(0).map_err(|e| EmbeddingError::Inference(e.to_string()))?;
        let types = ct(Tensor::new(&types[..], &session.device))?
            .unsqueeze(0)
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

        // No attention mask needed: batch of one has no padding to mask out.
        let hidden = ct(session.model.forward(&ids, &types, None))?;
        // CLS pooling: bge-zh-v1.5 represents a sentence with its [CLS] token.
        let cls = ct(hidden.get(0))?;
        let cls = ct(cls.get(0))?;
        // L2-normalise (the `+ eps` guards the all-zero case).
        let inv_norm = ct(cls.sqr())?;
        let inv_norm = ct(inv_norm.sum_all())?;
        let inv_norm = ct(inv_norm + 1e-12)?;
        let inv_norm = ct(inv_norm.sqrt())?;
        let inv_norm = ct(inv_norm.recip())?;
        let normalized = ct(cls.broadcast_mul(&inv_norm))?;
        let out: Vec<f32> = normalized
            .to_vec1()
            .map_err(|e| EmbeddingError::Inference(e.to_string()))?;

        if out.len() != EMBEDDING_DIM {
            return Err(EmbeddingError::Inference(format!(
                "model output has dim {}, expected {EMBEDDING_DIM}",
                out.len()
            )));
        }
        Ok(out)
    }

    /// Load config + tokenizer + weights from the cache directory. Called
    /// under the session lock, at most once per process.
    fn load_session(&self) -> Result<Session, EmbeddingError> {
        let device = Device::Cpu;
        let config: BertConfig = serde_json::from_slice(&std::fs::read(self.dir.join("config.json"))?)
            .map_err(|e| EmbeddingError::Inference(format!("config parse: {e}")))?;
        let tokenizer = tokenizers::Tokenizer::from_file(self.dir.join("tokenizer.json"))
            .map_err(|e| EmbeddingError::Inference(format!("tokenizer load: {e}")))?;
        // Non-mmap load on purpose: the weights sit in our own cache dir and
        // `ensure_model` could otherwise swap the file under the mapping.
        let tensors = candle_core::safetensors::load(self.dir.join("model.safetensors"), &device)
            .map_err(|e| EmbeddingError::Inference(format!("weights load: {e}")))?;
        let vs = VarBuilder::from_tensors(tensors, DType::F32, &device);
        let model = BertModel::load(vs, &config)
            .map_err(|e| EmbeddingError::Inference(format!("model load: {e}")))?;
        tracing::info!(dim = EMBEDDING_DIM, "embedding model loaded");
        Ok(Session { tokenizer, model, device })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::mpsc;

    /// Serves the three model files from an in-memory map on a loopback port.
    /// `missing` names a file to 404 instead.
    async fn spawn_file_server(files: Vec<(&'static str, Vec<u8>)>, missing: &'static str) -> u16 {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let map: std::collections::HashMap<String, Vec<u8>> =
            files.into_iter().map(|(n, b)| (n.to_string(), b)).collect();
        tokio::spawn(async move {
            while let Ok((mut sock, _)) = listener.accept().await {
                let map = map.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 65536];
                    let n = sock.read(&mut buf).await.unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]);
                    let path = req
                        .split_whitespace()
                        .nth(1)
                        .and_then(|p| p.rsplit('/').next())
                        .unwrap_or("")
                        .to_string();

                    let (head, body): (&str, Vec<u8>) = if path == missing || !map.contains_key(&path) {
                        (
                            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                            Vec::new(),
                        )
                    } else {
                        let body = map.get(&path).cloned().unwrap_or_default();
                        (
                            Box::leak(
                                format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\n\
                                     Content-Length: {}\r\nConnection: close\r\n\r\n",
                                    body.len()
                                )
                                .into_boxed_str(),
                            ),
                            body,
                        )
                    };
                    let _ = sock.write_all(head.as_bytes()).await;
                    let _ = sock.write_all(&body).await;
                    let _ = sock.shutdown().await;
                });
            }
        });
        port
    }

    fn service(port: u16, dir: &std::path::Path) -> EmbeddingService {
        EmbeddingService::with_base_url(dir.to_path_buf(), &format!("http://127.0.0.1:{port}"))
    }

    fn dir() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("echo-embed-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Everything is_ready() looks at — used by the "already cached" test.
    fn write_all_model_files(d: &std::path::Path) {
        std::fs::write(d.join("config.json"), b"{}").unwrap();
        std::fs::write(d.join("tokenizer.json"), b"{}").unwrap();
        std::fs::write(d.join("model.safetensors"), b"cached").unwrap();
    }

    #[test]
    fn encode_requires_the_model() {
        let d = dir();
        let svc = EmbeddingService::new(d.clone());
        // Not a zero vector, not a panic: an explicit error the callers skip on.
        assert!(matches!(svc.encode("你好"), Err(EmbeddingError::NotDownloaded(_))));
        assert!(matches!(svc.encode_query("你好"), Err(EmbeddingError::NotDownloaded(_))));
        std::fs::remove_dir_all(&d).ok();
    }

    #[tokio::test]
    async fn progress_is_incremental_and_files_land() {
        let big = vec![0xABu8; 4 * 1024 * 1024];
        let port = spawn_file_server(
            vec![
                ("config.json", b"{}".to_vec()),
                ("tokenizer.json", b"{}".to_vec()),
                ("model.safetensors", big.clone()),
            ],
            "",
        )
        .await;
        let d = dir();
        let svc = service(port, &d);

        let (tx, mut rx) = mpsc::channel(1024);
        svc.ensure_model(tx).await.unwrap();

        let mut events: Vec<(String, u64, u64)> = Vec::new();
        while let Some(ev) = rx.recv().await {
            events.push(ev);
        }

        assert!(svc.is_ready());
        assert_eq!(std::fs::read(d.join("model.safetensors")).unwrap(), big);

        // The big file must report progress as it streams, not once at the end.
        let big_events: Vec<&(String, u64, u64)> =
            events.iter().filter(|(f, _, _)| f == "model.safetensors").collect();
        assert!(big_events.len() >= 2, "expected incremental progress, got {events:?}");
        let mut last = 0u64;
        for (_, downloaded, total) in &big_events {
            assert_eq!(total, &(big.len() as u64));
            assert!(*downloaded > last, "progress must be monotonic");
            last = *downloaded;
        }
        assert_eq!(last, big.len() as u64);

        std::fs::remove_dir_all(&d).ok();
    }

    #[tokio::test]
    async fn missing_file_fails_and_stays_not_ready() {
        let port = spawn_file_server(
            vec![("config.json", b"{}".to_vec()), ("tokenizer.json", b"{}".to_vec())],
            "model.safetensors",
        )
        .await;
        let d = dir();
        let svc = service(port, &d);

        let (tx, _rx) = mpsc::channel(64);
        let result = svc.ensure_model(tx).await;

        assert!(result.is_err());
        assert!(!svc.is_ready());
        // No half-written file that later looks "ready".
        assert!(!d.join("model.safetensors").exists());
        assert!(!d.join("model.safetensors.part").exists());

        std::fs::remove_dir_all(&d).ok();
    }

    #[tokio::test]
    async fn already_ready_skips_download() {
        let port = spawn_file_server(vec![], "").await;
        let d = dir();
        write_all_model_files(&d);
        let svc = service(port, &d);

        let (tx, mut rx) = mpsc::channel(8);
        svc.ensure_model(tx).await.unwrap();

        assert!(rx.recv().await.is_none(), "no download should run");
        std::fs::remove_dir_all(&d).ok();
    }
}
