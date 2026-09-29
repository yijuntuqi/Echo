//! Embedding service: Candle + bge-small-zh-v1.5

use candle_core::{Device, Tensor, Result as CResult};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EmbeddingError {
    #[error("Model not found: {0}")] NotFound(String),
    #[error("Download failed: {0}")] Download(String),
    #[error("Tokenizer error: {0}")] Tokenizer(String),
    #[error("Candle error: {0}")] Candle(#[from] candle_core::Error),
    #[error("IO error: {0}")] Io(#[from] std::io::Error),
}

pub struct EmbeddingService {
    // model: BertModel,
    // tokenizer: Tokenizer,
    device: Device,
}

impl EmbeddingService {
    pub async fn new(model_dir: &Path, device: Device) -> Result<Self, EmbeddingError> {
        Ok(Self { device })
    }
    
    pub fn encode(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        // TODO: 实现推理
        Ok(texts.iter().map(|_| vec![0.0; 384]).collect())
    }
    
    pub fn encode_one(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        self.encode(&[text]).map(|v| v.into_iter().next().unwrap())
    }
}

pub struct ModelManager {
    models_dir: std::path::PathBuf,
}

impl ModelManager {
    pub fn new(app: &tauri::AppHandle) -> Result<Self, EmbeddingError> {
        let models_dir = app.path().app_data_dir()?.join("models");
        std::fs::create_dir_all(&models_dir)?;
        Ok(Self { models_dir })
    }
    
    pub async fn ensure_bge_small_zh(&self, _progress_tx: tokio::sync::mpsc::Sender<DownloadProgress>) -> Result<std::path::PathBuf, EmbeddingError> {
        Ok(self.models_dir.join("bge-small-zh-v1.5"))
    }
}

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub filename: String,
    pub downloaded: u64,
    pub total: u64,
}