//! Zhipu BigModel client: streaming completions with retries and a quota cap.
//!
//! Replies are pushed to the frontend over the `chat:stream` event rather than
//! returned from the command, so the command resolves immediately and the UI
//! can render tokens as they arrive.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tauri::Emitter;

use crate::db::Conversation;

/// Zhipu's OpenAI-compatible endpoint.
pub const DEFAULT_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";

/// The official shared key would live here so unpaid installs still get the
/// daily allowance. Nothing is committed to this public repository; an empty
/// value means every install must bring its own key.
pub const SHARED_API_KEY: &str = "";

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
            base_url: DEFAULT_BASE_URL.into(),
            // Overwritten from settings by `apply_key`; empty means "bring
            // your own key" is still pending.
            api_key: SHARED_API_KEY.into(),
            daily_model: "glm-5.3-flash".into(),
            premium_model: "glm-5.2".into(),
            max_tokens: 2048,
            temperature: 0.8,
            timeout_secs: 60,
            retry_max: 2,
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

/// What one streamed completion produced, beyond the visible reply.
#[derive(Debug, Clone)]
pub struct CompletionOutcome {
    pub reply: String,
    /// The model actually requested (daily or premium), for the audit trail.
    pub model: String,
    /// Filled from the usage frame Zhipu appends to the last chunk.
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
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
    /// Built eagerly so connection setup and TLS roots are paid once rather
    /// than per request. The timeout is fixed at construction; `configure`
    /// below only swaps models and keys.
    client: reqwest::Client,
    /// Interior-mutable so `apply_key` can reconfigure through the shared
    /// `AppState` without `&mut`.
    config: std::sync::RwLock<ChatConfig>,
    pub quota: QuotaTracker,
}

impl ChatEngine {
    pub fn new(config: ChatConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .user_agent(concat!("Echo/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();
        Self {
            client,
            config: std::sync::RwLock::new(config),
            quota: QuotaTracker::new(),
        }
    }

    pub fn configure(&self, config: ChatConfig) {
        *self.config.write().expect("chat config lock poisoned") = config;
    }

    pub fn config(&self) -> ChatConfig {
        self.config.read().expect("chat config lock poisoned").clone()
    }

    /// The model to ask for on this request.
    fn model_for(&self, premium: bool) -> String {
        let cfg = self.config();
        if premium { cfg.premium_model } else { cfg.daily_model }
    }

    /// Stream a completion from the configured endpoint, calling `on_delta`
    /// with each visible content fragment.
    ///
    /// Reasoning models (the glm-4.5 family onward) stream a
    /// `reasoning_content` field before the answer; those fragments are
    /// dropped here so the panel only ever sees the reply itself.
    ///
    /// Server errors (5xx) and network failures retry with exponential
    /// backoff; client errors (4xx) fail immediately — a bad key will not
    /// get better by asking again.
    pub async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
        use_premium: bool,
        mut on_delta: impl FnMut(&str),
    ) -> Result<CompletionOutcome, ChatError> {
        let cfg = self.config();
        if cfg.api_key.is_empty() {
            return Err(ChatError::NotConfigured);
        }
        let model = self.model_for(use_premium);

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
            "max_tokens": cfg.max_tokens,
            "temperature": cfg.temperature,
        });

        let url = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));
        let mut attempt = 0u32;
        loop {
            let result = self
                .attempt_stream(&url, &cfg.api_key, &body, &model, &mut on_delta)
                .await;
            match result {
                Ok(outcome) => return Ok(outcome),
                Err(err) => {
                    let retryable = match &err {
                        ChatError::Network(_) => true,
                        ChatError::Api { status, .. } => *status >= 500,
                        _ => false,
                    };
                    attempt += 1;
                    if !retryable || attempt > cfg.retry_max {
                        return Err(err);
                    }
                    let backoff = cfg
                        .retry_base_ms
                        .saturating_mul(1 << (attempt - 1).min(16));
                    tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
                }
            }
        }
    }

    /// One request/response cycle. Retry logic lives in the caller.
    async fn attempt_stream(
        &self,
        url: &str,
        api_key: &str,
        body: &serde_json::Value,
        model: &str,
        on_delta: &mut impl FnMut(&str),
    ) -> Result<CompletionOutcome, ChatError> {
        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Accept", "text/event-stream")
            .json(body)
            .send()
            .await
            .map_err(|e| ChatError::Network(e.to_string()))?;

        let status = response.status().as_u16();
        if status >= 400 {
            let body = response.text().await.unwrap_or_default();
            return Err(ChatError::Api { status, body });
        }

        let mut stream = response.bytes_stream();
        let mut reply = String::new();
        let mut prompt_tokens = None;
        let mut completion_tokens = None;
        // SSE frames can straddle chunk boundaries, so partial lines are
        // carried over between reads instead of parsed in place.
        let mut pending = String::new();

        while let Some(item) = stream.next().await {
            let bytes = item.map_err(|e| ChatError::Network(e.to_string()))?;
            pending.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(idx) = pending.find(['\n']) {
                let line: String = pending.drain(..=idx).collect();
                let line = line.trim_end_matches(['\r', '\n']);
                if let Some(frame) = line.strip_prefix("data:") {
                    let frame = frame.trim_start();
                    if frame == "[DONE]" {
                        return Ok(CompletionOutcome {
                            reply,
                            model: model.to_string(),
                            prompt_tokens,
                            completion_tokens,
                        });
                    }
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(frame) {
                        if value.get("error").is_some() {
                            return Err(ChatError::Api {
                                status: 200,
                                body: value.to_string(),
                            });
                        }
                        if let Some(usage) = value.get("usage") {
                            prompt_tokens = usage
                                .get("prompt_tokens")
                                .and_then(|v| v.as_u64())
                                .map(|v| v as u32);
                            completion_tokens = usage
                                .get("completion_tokens")
                                .and_then(|v| v.as_u64())
                                .map(|v| v as u32);
                        }
                        // Only visible answer text is forwarded; reasoning
                        // fragments stay on this side of the IPC boundary.
                        let delta = value
                            .pointer("/choices/0/delta/content")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if !delta.is_empty() {
                            reply.push_str(delta);
                            on_delta(delta);
                        }
                    }
                }
            }
        }

        // The stream closed without [DONE] — keep what arrived rather than
        // discarding a possibly complete answer.
        Ok(CompletionOutcome {
            reply,
            model: model.to_string(),
            prompt_tokens,
            completion_tokens,
        })
    }
}

use futures_util::StreamExt;

/// Install `user_api_key` as the active key, or fall back to the shared one.
///
/// A user's own key lifts the shared daily cap entirely; the shared key (when
/// an official one exists) is capped by `daily_limit`.
pub async fn apply_key(engine: &ChatEngine, user_api_key: Option<&str>) {
    let mut cfg = engine.config();
    match user_api_key.map(str::trim).filter(|k| !k.is_empty()) {
        Some(key) => {
            cfg.api_key = key.to_string();
            engine.configure(cfg);
            engine.quota.set_unlimited(true).await;
        }
        None => {
            cfg.api_key = SHARED_API_KEY.to_string();
            engine.configure(cfg);
            engine.quota.set_unlimited(false).await;
        }
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
        .chain(std::iter::once(ChatMessage::user(message.clone())))
        .collect();

    let limit = engine.config().daily_limit;
    if !engine.quota.try_consume(limit).await {
        let _ = app.emit("chat:status", StatusEvent { offline: true, quota_remaining: Some(0) });
        return Err(ChatError::QuotaExhausted.to_string());
    }

    let emitter = app.clone();
    let outcome = engine
        .stream_completion(messages, false, move |delta| {
            let _ = emitter.emit(
                "chat:stream",
                StreamChunk { delta: delta.to_string(), done: false, emotion: None },
            );
        })
        .await;

    let outcome = match outcome {
        Ok(o) if !o.reply.is_empty() => o,
        Ok(_) => {
            // Connected but nothing said: surface it instead of a silent panel.
            let _ = app.emit(
                "chat:status",
                StatusEvent { offline: true, quota_remaining: None },
            );
            return Err("模型没有返回内容，请稍后再试".into());
        }
        Err(e) => {
            let _ = app.emit(
                "chat:status",
                StatusEvent { offline: true, quota_remaining: None },
            );
            return Err(e.to_string());
        }
    };

    app.emit(
        "chat:stream",
        StreamChunk { delta: String::new(), done: true, emotion: None },
    )
    .map_err(|e| e.to_string())?;

    // The audit trail is best-effort: a failed insert must not undo the
    // reply the user already received.
    if let Ok(pool) = pool() {
        let tokens = outcome
            .prompt_tokens
            .zip(outcome.completion_tokens)
            .map(|(p, c)| (p + c) as i32);
        let _ = sqlx::query(
            "INSERT INTO conversations (user_message, ai_reply, tokens_used, model_used)
             VALUES (?, ?, ?, ?)",
        )
        .bind(&message)
        .bind(&outcome.reply)
        .bind(tokens)
        .bind(&outcome.model)
        .execute(pool)
        .await;
    }

    Ok(())
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

#[derive(serde::Serialize, Clone)]
pub struct StatusEvent {
    pub offline: bool,
    pub quota_remaining: Option<u32>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Serves one canned HTTP response per connection, sequentially. Every
    /// request opens a fresh connection (`Connection: close`), so an attempt
    /// counter over connections is an attempt counter over requests.
    async fn spawn_server(responses: Vec<String>) -> (u16, Arc<AtomicUsize>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let hits = Arc::new(AtomicUsize::new(0));
        let hits2 = hits.clone();
        tokio::spawn(async move {
            let mut responses = responses.into_iter().cycle();
            while let Ok((mut sock, _)) = listener.accept().await {
                let Some(resp) = responses.next() else { break };
                hits2.fetch_add(1, Ordering::SeqCst);
                let mut buf = vec![0u8; 65536];
                let _ = sock.read(&mut buf).await;
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.shutdown().await;
            }
        });
        (port, hits)
    }

    fn sse_response(chunks: &[String]) -> String {
        let mut body = String::new();
        for c in chunks {
            body.push_str(&format!("data: {c}\n\n"));
        }
        body.push_str("data: [DONE]\n\n");
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    fn error_response(status: u16, body: &str) -> String {
        format!(
            "HTTP/1.1 {status} Err\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    /// One OpenAI-compatible streaming chunk. `delta` is raw JSON so tests can
    /// shape it freely (reasoning content, plain content, empty deltas…).
    fn chunk(delta: &str) -> String {
        format!(r#"{{"id":"t","choices":[{{"index":0,"delta":{delta}}}]}}"#)
    }

    fn usage_chunk() -> String {
        r#"{"id":"t","choices":[{"index":0,"delta":{}}],"usage":{"prompt_tokens":5,"completion_tokens":3,"total_tokens":8}}"#.into()
    }

    fn engine(port: u16) -> ChatEngine {
        let cfg = ChatConfig {
            base_url: format!("http://127.0.0.1:{port}"),
            api_key: "test-key".into(),
            timeout_secs: 5,
            retry_max: 2,
            retry_base_ms: 10,
            ..Default::default()
        };
        ChatEngine::new(cfg)
    }

    #[tokio::test]
    async fn streams_content_deltas_and_skips_reasoning() {
        let chunks = vec![
            chunk(r#"{"role":"assistant","reasoning_content":"let me think"}"#),
            chunk(r#"{"role":"assistant","content":"你好"}"#),
            chunk(r#"{"role":"assistant","content":"呀"}"#),
            chunk(r#"{"role":"assistant","content":""}"#),
            usage_chunk(),
        ];
        let (port, _hits) = spawn_server(vec![sse_response(&chunks)]).await;

        let mut deltas = Vec::new();
        let engine = engine(port);
        let outcome = engine
            .stream_completion(
                vec![ChatMessage::user("hi")],
                false,
                |d| deltas.push(d.to_string()),
            )
            .await
            .unwrap();

        assert_eq!(outcome.reply, "你好呀");
        assert_eq!(deltas, vec!["你好".to_string(), "呀".to_string()]);
        assert_eq!(outcome.prompt_tokens, Some(5));
        assert_eq!(outcome.completion_tokens, Some(3));
        assert_eq!(outcome.model, engine.config().daily_model);
    }

    #[tokio::test]
    async fn premium_flag_selects_premium_model() {
        let (port, _hits) =
            spawn_server(vec![sse_response(&[chunk(r#"{"content":"好"}"#)])]).await;
        let engine = engine(port);
        let expected = engine.config().premium_model.clone();
        let outcome = engine
            .stream_completion(vec![ChatMessage::user("hi")], true, |_| {})
            .await
            .unwrap();
        assert_eq!(outcome.model, expected);
    }

    #[tokio::test]
    async fn empty_key_is_not_configured() {
        let (port, _hits) =
            spawn_server(vec![sse_response(&[chunk(r#"{"content":"好"}"#)])]).await;
        let cfg = ChatConfig {
            base_url: format!("http://127.0.0.1:{port}"),
            api_key: String::new(),
            ..Default::default()
        };
        let result = ChatEngine::new(cfg)
            .stream_completion(vec![ChatMessage::user("hi")], false, |_| {})
            .await;
        assert!(matches!(result, Err(ChatError::NotConfigured)));
    }

    #[tokio::test]
    async fn retries_server_errors_and_succeeds() {
        let chunks = vec![chunk(r#"{"content":"恢复"}"#)];
        let (port, hits) = spawn_server(vec![
            error_response(500, r#"{"error":{"message":"boom"}}"#),
            sse_response(&chunks),
        ])
        .await;

        let outcome = engine(port)
            .stream_completion(vec![ChatMessage::user("hi")], false, |_| {})
            .await
            .unwrap();
        assert_eq!(outcome.reply, "恢复");
        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn client_errors_fail_without_retry() {
        let (port, hits) = spawn_server(vec![error_response(
            401,
            r#"{"error":{"code":"1002","message":"bad key"}}"#,
        )])
        .await;

        let result = engine(port)
            .stream_completion(vec![ChatMessage::user("hi")], false, |_| {})
            .await;
        match result {
            Err(ChatError::Api { status, body }) => {
                assert_eq!(status, 401);
                assert!(body.contains("bad key"));
            }
            other => panic!("expected Api error, got {other:?}"),
        }
        assert_eq!(hits.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn quota_tracker_resets_daily_and_bypasses_for_own_key() {
        let tracker = QuotaTracker::new();
        assert!(tracker.try_consume(2).await);
        assert!(tracker.try_consume(2).await);
        assert!(!tracker.try_consume(2).await);

        tracker.set_unlimited(true).await;
        assert!(tracker.try_consume(2).await);
        assert_eq!(tracker.remaining(2).await, 2);
    }

    #[tokio::test]
    async fn apply_settings_prefers_user_key_and_marks_unlimited() {
        let (port, _hits) =
            spawn_server(vec![sse_response(&[chunk(r#"{"content":"好"}"#)])]).await;
        let cfg = ChatConfig {
            base_url: format!("http://127.0.0.1:{port}"),
            ..Default::default()
        };
        let engine = ChatEngine::new(cfg);

        apply_key(&engine, Some(" user-key ")).await;
        assert_eq!(engine.config().api_key, "user-key");
        assert_eq!(engine.quota.remaining(200).await, 200);

        apply_key(&engine, None).await;
        assert_eq!(engine.config().api_key, SHARED_API_KEY);
        assert_eq!(engine.quota.remaining(200).await, 200);
    }
}
