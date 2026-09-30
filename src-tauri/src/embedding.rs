//! Local text embeddings for semantic memory search.
//!
//! The model is downloaded on first run and cached in the app data directory.
//! Until it lands, memory search falls back to keyword matching so the feature
//! degrades rather than disappearing.

use std::path::{Path, PathBuf};

use thiserror::Error;

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
}

impl EmbeddingService {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn is_ready(&self) -> bool {
        self.dir.join("model.safetensors").exists()
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Download the model files if they are not already cached.
    ///
    /// Reports progress as `(filename, downloaded_bytes, total_bytes)` so the
    /// UI can show a determinate bar.
    pub async fn ensure_model(
        &self,
        progress: tokio::sync::mpsc::Sender<(String, u64, u64)>,
    ) -> Result<(), EmbeddingError> {
        if self.is_ready() {
            return Ok(());
        }
        std::fs::create_dir_all(&self.dir)?;

        for file in ["config.json", "tokenizer.json", "model.safetensors"] {
            let url = format!("{MODEL_BASE_URL}/{MODEL_ID}/resolve/main/{file}");
            let dest = self.dir.join(file);
            let bytes = reqwest::get(&url)
                .await
                .map_err(|e| EmbeddingError::Download(format!("{file}: {e}")))?
                .bytes()
                .await
                .map_err(|e| EmbeddingError::Download(format!("{file}: {e}")))?;

            // Write to a temp name so an interrupted download never leaves a
            // truncated file that later looks "ready".
            let tmp = dest.with_extension("part");
            std::fs::write(&tmp, &bytes)?;
            std::fs::rename(&tmp, &dest)?;

            let _ = progress.send((file.to_string(), bytes.len() as u64, bytes.len() as u64)).await;
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
