//! ChatAnywhere client: streaming completions with retries and a quota cap.
//!
//! Replies are pushed to the frontend over the `chat:stream` event rather than
//! returned from the command, so the command resolves immediately and the UI
//! can render tokens as they arrive.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tauri::Emitter;

use crate::db::Conversation;

#[derive(Debug, Error)]
pub enum ChatError {
    #[error("network unreachable: {0}")]
    Network(String),
    #[error("api returned {status}: {body}")]
    Api { status: u16, body: String },
    #[error("daily quota exhausted")]
    QuotaExhausted,
    #[error("not configured yet")]
    NotConfigured,
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
    /// Requests allowed per day on the shared key. Bypassed when the user
    /// supplies their own key.
    pub daily_limit: u32,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.chatanywhere.tech/v1".into(),
            // Overwritten from settings on first run; empty means "not set up".
            api_key: String::new(),
            daily_model: "gpt-4o-mini".into(),
            premium_model: "gpt-4o".into(),
            max_tokens: 2048,
            temperature: 0.8,
            timeout_secs: 60,
            retry_max: 3,
            retry_base_ms: 800,
            daily_limit: 200,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub version: String,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: MessageRole::System, content: content.into() }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self { role: MessageRole::User, content: content.into() }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: MessageRole::Assistant, content: content.into() }
    }
}

/// Counts requests against the daily allowance. Resets at UTC midnight.
pub struct QuotaTracker {
    inner: tokio::sync::Mutex<QuotaState>,
}

#[derive(Default)]
struct QuotaState {
    used: u32,
    day: Option<chrono::NaiveDate>,
    /// When true the user brought their own key, so the shared cap does not apply.
    unlimited: bool,
}

impl QuotaTracker {
    pub fn new() -> Self {
        Self { inner: tokio::sync::Mutex::new(QuotaState::default()) }
    }

    /// Returns `false` when the day's allowance is used up.
    pub async fn try_consume(&self, limit: u32) -> bool {
        let mut st = self.inner.lock().await;
        let today = chrono::Utc::now().date_naive();
        if st.day != Some(today) {
            st.day = Some(today);
            st.used = 0;
        }
        if st.unlimited {
            return true;
        }
        if st.used >= limit {
            return false;
        }
        st.used += 1;
        true
    }

    pub async fn set_unlimited(&self, v: bool) {
        self.inner.lock().await.unlimited = v;
    }

    pub async fn remaining(&self, limit: u32) -> u32 {
        let st = self.inner.lock().await;
        if st.unlimited {
            return limit;
        }
        limit.saturating_sub(st.used)
    }
}

pub struct ChatEngine {
    /// Kept for the streaming implementation; built eagerly so connection
    /// setup and TLS roots are paid once rather than per request.
    #[allow(dead_code)]
    client: reqwest::Client,
    config: ChatConfig,
    pub quota: QuotaTracker,
}

impl ChatEngine {
    pub fn new(config: ChatConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .user_agent(concat!("Echo/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();
        Self { client, config, quota: QuotaTracker::new() }
    }

    pub fn configure(&mut self, config: ChatConfig) {
        self.config = config;
    }

    pub fn config(&self) -> &ChatConfig {
        &self.config
    }

    /// Stream a completion, emitting `chat:stream` deltas to `app`.
    ///
    /// Returns the assembled reply. Implemented in the chat task; until then the
    /// command reports that the feature is not yet available rather than
    /// pretending to answer.
    pub async fn stream_completion(
        &self,
        _app: &tauri::AppHandle,
        _messages: Vec<ChatMessage>,
        _use_premium: bool,
    ) -> Result<String, ChatError> {
        if self.config.api_key.is_empty() {
            return Err(ChatError::NotConfigured);
        }
        Err(ChatError::NotConfigured)
    }
}

fn pool() -> Result<&'static crate::db::DbPool, String> {
    crate::pool()
}

/// Send a message. The reply arrives asynchronously over `chat:stream`.
#[tauri::command]
pub async fn send_message(app: tauri::AppHandle, message: String) -> Result<(), String> {
    let engine = &crate::state().chat;
    let history = recent_history(20).await;

    // Oldest-first so the model reads the conversation in order.
    let messages: Vec<ChatMessage> = std::iter::once(ChatMessage::system(SYSTEM_PROMPT))
        .chain(history.into_iter().map(ChatMessage::assistant))
        .chain(std::iter::once(ChatMessage::user(message)))
        .collect();

    let reply = engine
        .stream_completion(&app, messages, false)
        .await
        .map_err(|e| e.to_string())?;

    app.emit(
        "chat:stream",
        StreamChunk { delta: reply, done: true, emotion: None },
    )
    .map_err(|e| e.to_string())
}

/// Echo's persona. Kept short: the real personality work lands with the
/// evolution engine, which will vary this by stage and mood.
const SYSTEM_PROMPT: &str = "\
你是 Echo，一个住在用户电脑桌面上的小宠物。你说话简短、口语化，\
会关心对方的状态，也会有自己的小情绪。不要说自己是 AI 或语言模型。\
用中文回答，除非用户使用其他语言。";

#[derive(serde::Serialize, Clone)]
pub struct StreamChunk {
    pub delta: String,
    pub done: bool,
    pub emotion: Option<EmotionPayload>,
}

#[derive(serde::Serialize, Clone)]
pub struct EmotionPayload {
    pub emotion: crate::emotion::Emotion,
    pub weight: f32,
    pub topics: Vec<String>,
}

/// Recent assistant turns, oldest-first, to give the model conversational
/// continuity. Silently empty before onboarding has created the database.
async fn recent_history(limit: i64) -> Vec<String> {
    let Ok(pool) = pool() else {
        return Vec::new();
    };
    sqlx::query_scalar::<_, String>(
        "SELECT ai_reply FROM conversations ORDER BY id DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map(|rows| rows.into_iter().rev().collect())
    .unwrap_or_default()
}

#[tauri::command]
pub async fn get_history(limit: usize, offset: usize) -> Result<Vec<Conversation>, String> {
    let pool = pool()?;
    sqlx::query_as::<_, Conversation>(
        "SELECT id, timestamp, user_message, ai_reply, emotion, emotion_weight,
                tokens_used, model_used
         FROM conversations ORDER BY id DESC LIMIT ? OFFSET ?",
    )
    .bind(limit as i64)
    .bind(offset as i64)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_quota_remaining() -> Result<u32, String> {
    let engine = &crate::state().chat;
    let limit = engine.config().daily_limit;
    Ok(engine.quota.remaining(limit).await)
}
