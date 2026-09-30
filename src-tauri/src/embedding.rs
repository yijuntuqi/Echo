//! Local text embeddings for semantic memory search.
//!
//! The model is downloaded on first run and cached in the app data directory.
//! Until it lands, memory search falls back to keyword matching so the feature
//! degrades rather than disappearing.

use std::path::{Path, PathBuf};

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

/// Dimensionality of `bge-small-zh-v1.5`, the model we ship with.
pub const EMBEDDING_DIM: usize = 384;

pub const MODEL_ID: &str = "BAAI/bge-small-zh-v1.5";
/// Mirror first: the canonical host is slow or blocked on some networks.
pub const MODEL_BASE_URL: &str = "https://hf-mirror.com";

pub struct EmbeddingService {
    dir: PathBuf,
    base_url: String,
}

impl EmbeddingService {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir, base_url: MODEL_BASE_URL.into() }
    }

    /// Override the mirror, for tests that serve the files locally.
    pub fn with_base_url(dir: PathBuf, base_url: &str) -> Self {
        Self { dir, base_url: base_url.into() }
    }

    pub fn is_ready(&self) -> bool {
        self.dir.join("model.safetensors").exists()
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

    /// Encode one string. Implemented in the Candle task; until then this
    /// reports why it cannot run rather than returning a zero vector, which
    /// would silently poison every similarity search.
    pub fn encode(&self, _text: &str) -> Result<Vec<f32>, EmbeddingError> {
        Err(EmbeddingError::Inference(
            "embedding inference is not wired up yet".into(),
        ))
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
        std::fs::write(d.join("model.safetensors"), b"cached").unwrap();
        let svc = service(port, &d);

        let (tx, mut rx) = mpsc::channel(8);
        svc.ensure_model(tx).await.unwrap();

        assert!(rx.recv().await.is_none(), "no download should run");
        std::fs::remove_dir_all(&d).ok();
    }
}
