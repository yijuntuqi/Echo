//! Chat engine: ChatAnywhere API client with streaming, retry, structured output

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChatError {
    #[error("Network error: {0}")] Network(String),
    #[error("API error: {status} - {body}")] Api { status: u16, body: String },
    #[error("Quota exhausted")] QuotaExhausted,
    #[error("Serialization error: {0}")] Serde(#[from] serde_json::Error),
    #[error("Stream error: {0}")] Stream(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    pub base_url: String,
    pub api_key: String,
    pub daily_model: String,
    pub premium_model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout_secs: u64,
    pub retry_max: u32,
    pub retry_base_ms: u64,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.chatanywhere.tech/v1".into(),
            api_key: String::new(),
            daily_model: "gpt-4o-mini".into(),
            premium_model: "gpt-4o".into(),
            max_tokens: 2048,
            temperature: 0.7,
            timeout_secs: 30,
            retry_max: 3,
            retry_base_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole { System, User, Assistant, Tool }

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    pub temperature: f32,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

#[derive(Debug, Serialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub type_: String,
    pub json_schema: Option<serde_json::Value>,
}

impl ResponseFormat {
    pub fn json_schema(schema: &str) -> Self {
        Self {
            type_: "json_schema".into(),
            json_schema: Some(serde_json::from_str(schema).unwrap()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ChatChunk {
    pub id: String,
    pub choices: Vec<ChunkChoice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub struct ChunkChoice {
    pub index: u32,
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    pub content: Option<String>,
    pub role: Option<MessageRole>,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Serialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub version: String,
    pub notes: String,
}

pub struct ChatEngine {
    client: Client,
    config: ChatConfig,
    quota_tracker: QuotaTracker,
}

impl ChatEngine {
    pub fn new(config: ChatConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .unwrap();
        Self {
            client,
            config,
            quota_tracker: QuotaTracker::default(),
        }
    }
    
    pub async fn stream_chat(
        &self,
        messages: Vec<ChatMessage>,
        use_premium: bool,
    ) -> Result<mpsc::Receiver<Result<ChatChunk, ChatError>>, ChatError> {
        let (tx, rx) = mpsc::channel(32);
        // TODO: 实现流式请求
        Ok(rx)
    }
}

#[derive(Default)]
pub struct QuotaTracker {
    inner: std::sync::Arc<tokio::sync::RwLock<QuotaState>>,
}

#[derive(Default)]
struct QuotaState {
    used_today: u32,
    limit: u32,
    last_reset: chrono::DateTime<chrono::Utc>,
    user_key_mode: bool,
}

impl QuotaTracker {
    pub async fn try_consume(&self, n: u32) -> bool {
        let mut state = self.inner.write().await;
        if state.user_key_mode { return true; }
        let now = chrono::Utc::now();
        if now.date_naive() != state.last_reset.date_naive() {
            state.used_today = 0;
            state.last_reset = now;
        }
        if state.used_today + n > state.limit { false } else { state.used_today += n; true }
    }
}

#[tauri::command]
pub async fn send_message(message: String) -> Result<String, String> {
    Ok("Stub response".into())
}

#[tauri::command]
pub async fn get_history(limit: usize, offset: usize) -> Result<Vec<crate::db::Conversation>, String> {
    Ok(vec![])
}

#[tauri::command]
pub async fn get_quota_remaining() -> Result<u32, String> {
    Ok(200)
}