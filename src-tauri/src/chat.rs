//! SenseNova client: streaming completions with retries and a quota cap.
//!
//! Replies are pushed to the frontend over the `chat:stream` event rather than
//! returned from the command, so the command resolves immediately and the UI
//! can render tokens as they arrive.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tauri::Emitter;

use crate::db::{Conversation, VectorHit};

/// SenseNova's OpenAI-compatible endpoint.
pub const DEFAULT_BASE_URL: &str = "https://token.sensenova.cn/v1";

/// Environment variable carrying the official shared key. It can live in a
/// gitignored local `.env` file (loaded by `dotenvy` in `lib.rs`) or be set
/// in the process environment; it is never committed to the repository.
pub const SHARED_API_KEY_ENV: &str = "ECHO_SHARED_KEY";

/// The official shared key lets unpaid installs use the daily allowance
/// (capped locally by [`QuotaTracker`]); users who bring their own key
/// bypass the cap entirely. Empty means "bring your own key" is pending.
pub fn shared_api_key() -> String {
    std::env::var(SHARED_API_KEY_ENV)
        .unwrap_or_default()
        .trim()
        .to_string()
}

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
            api_key: shared_api_key(),
            daily_model: "sensenova-6.8-flash-lite".into(),
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
    /// Reasoning models stream their thinking in a separate delta field
    /// (`reasoning` on SenseNova, `reasoning_content` elsewhere) before the
    /// answer; those fragments are dropped here so the panel only ever sees
    /// the reply itself.
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
            tracing::error!(
                "no LLM key configured: put ECHO_SHARED_KEY=<key> in the project .env (UTF-8/ASCII), or set your own key in settings"
            );
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
        // Once any delta has been forwarded to the panel, a retry must not
        // happen: the second attempt re-streams the full reply, which would
        // land on top of the first half and duplicate everything.
        let mut emitted = false;
        loop {
            let result = {
                let emitted = &mut emitted;
                let on_delta = &mut on_delta;
                self.attempt_stream(&url, &cfg.api_key, &body, &model, &mut |delta| {
                    *emitted = true;
                    on_delta(delta);
                })
                .await
            };
            match result {
                Ok(outcome) => return Ok(outcome),
                Err(err) => {
                    let retryable = match &err {
                        ChatError::Network(_) => true,
                        ChatError::Api { status, .. } => *status >= 500,
                        _ => false,
                    };
                    if emitted {
                        tracing::warn!("stream interrupted mid-reply; not retrying to avoid duplicated deltas");
                        return Err(err);
                    }
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
                    // Raw frames at debug level: protocol mismatches (e.g.
                    // SenseNova's empty-choices usage frame) show up here.
                    tracing::debug!(line = %line, "sse frame");
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

    /// One non-streaming completion. Single attempt — callers own retry
    /// policy. Used by background work (classification, recaps, greetings)
    /// where there is nothing to stream to.
    pub async fn complete(
        &self,
        messages: Vec<ChatMessage>,
        use_premium: bool,
    ) -> Result<CompletionOutcome, ChatError> {
        let cfg = self.config();
        if cfg.api_key.is_empty() {
            tracing::error!(
                "no LLM key configured: put ECHO_SHARED_KEY=<key> in the project .env (UTF-8/ASCII), or set your own key in settings"
            );
            return Err(ChatError::NotConfigured);
        }
        let model = self.model_for(use_premium);
        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": cfg.max_tokens,
            "temperature": cfg.temperature,
        });
        let url = format!("{}/chat/completions", cfg.base_url.trim_end_matches('/'));

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", cfg.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| ChatError::Network(e.to_string()))?;

        let status = response.status().as_u16();
        let text = response
            .text()
            .await
            .map_err(|e| ChatError::Network(e.to_string()))?;
        if status >= 400 {
            return Err(ChatError::Api { status, body: text });
        }

        let (reply, prompt_tokens, completion_tokens) = parse_completion(&text)?;
        Ok(CompletionOutcome { reply, model, prompt_tokens, completion_tokens })
    }
}

use futures_util::StreamExt;

/// Parse one non-streaming completion response into
/// `(reply, prompt_tokens, completion_tokens)`.
///
/// `reasoning_content` is dropped exactly like the streaming parser drops it;
/// a `null` content counts as empty; an `error` payload becomes
/// [`ChatError::Api`] with the HTTP status (200 — the failure is in the body).
fn parse_completion(body: &str) -> Result<(String, Option<u32>, Option<u32>), ChatError> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|e| ChatError::Api { status: 200, body: e.to_string() })?;
    if value.get("error").is_some() {
        return Err(ChatError::Api { status: 200, body: body.to_string() });
    }
    let reply = value
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let usage = value.get("usage");
    let prompt = usage.and_then(|u| u.get("prompt_tokens")).and_then(|v| v.as_u64()).map(|v| v as u32);
    let completion = usage
        .and_then(|u| u.get("completion_tokens"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);
    Ok((reply, prompt, completion))
}

/// Persist one finished turn: the conversation row (with the emotional read,
/// or NULLs when neither pass produced one) and, on a real read, today's mood
/// (latest turn of the day wins — the table is UNIQUE on date).
///
/// Best-effort by contract: failures are logged, never propagated, so a
/// storage hiccup cannot undo a reply the user already has. Returns the
/// `mood:updated` payload when today's mood changed.
pub(crate) async fn persist_turn(
    pool: &crate::db::DbPool,
    user_message: &str,
    outcome: &CompletionOutcome,
    read: Option<&crate::emotion::EmotionResult>,
) -> Option<MoodUpdated> {
    let tokens = outcome
        .prompt_tokens
        .zip(outcome.completion_tokens)
        .map(|(p, c)| (p + c) as i32);
    let (emotion, weight, topics) = match read {
        Some(r) => (
            Some(r.emotion.as_str()),
            Some(r.confidence as f64),
            Some(serde_json::to_string(&r.topics).unwrap_or_else(|_| "[]".into())),
        ),
        None => (None, None, None),
    };

    let conv_id: Option<i64> = match sqlx::query_scalar::<_, i64>(
        "INSERT INTO conversations
             (user_message, ai_reply, emotion, emotion_weight, topics, tokens_used, model_used)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(user_message)
    .bind(&outcome.reply)
    .bind(emotion)
    .bind(weight)
    .bind(&topics)
    .bind(tokens)
    .bind(&outcome.model)
    .fetch_one(pool)
    .await
    {
        Ok(id) => Some(id),
        Err(e) => {
            tracing::warn!("conversation insert failed: {e}");
            None
        }
    };

    // Index the turn for semantic search. Best-effort: a not-yet-ready or
    // failed embedding model only means this turn stays unindexed for now.
    if let (Some(id), Some(embedding)) = (conv_id, crate::state().embedding.get()) {
        let text: String =
            format!("{user_message} {}", outcome.reply).chars().take(600).collect();
        match embedding.encode(&text) {
            Ok(vec) => crate::vector::store_memory(pool, &vec, id, "conversation").await,
            Err(e) => tracing::debug!(error = %e, "turn embedding skipped"),
        }
    }

    let read = read?;
    let date = chrono::Local::now().date_naive().to_string();
    if let Err(e) = sqlx::query(
        "INSERT INTO moods (date, emotion, weight, source) VALUES (?, ?, ?, 'auto')
         ON CONFLICT(date) DO UPDATE SET emotion = excluded.emotion, weight = excluded.weight",
    )
    .bind(&date)
    .bind(read.emotion.as_str())
    .bind(read.confidence as f64)
    .execute(pool)
    .await
    {
        tracing::warn!("mood upsert failed: {e}");
        return None;
    }
    Some(MoodUpdated { date, emotion: read.emotion, weight: read.confidence })
}

/// Payload of the `mood:updated` event (mirrored by `events.ts`).
#[derive(serde::Serialize, Clone)]
pub struct MoodUpdated {
    pub date: String,
    pub emotion: crate::emotion::Emotion,
    pub weight: f32,
}

/// Install `user_api_key` as the active key, or fall back to the shared one.
///
/// A user's own key lifts the shared daily cap entirely; the shared key (when
/// an official one exists) is capped by `daily_limit`. `user_base_url` pairs
/// with the key for third-party OpenAI-compatible endpoints (OpenRouter, a
/// local vLLM…); empty means the built-in default endpoint.
pub async fn apply_key(engine: &ChatEngine, user_api_key: Option<&str>, user_base_url: Option<&str>) {
    let mut cfg = engine.config();
    let using_user_key = user_api_key.map(str::trim).filter(|k| !k.is_empty()).is_some();
    match user_api_key.map(str::trim).filter(|k| !k.is_empty()) {
        Some(key) => {
            cfg.api_key = key.to_string();
            engine.quota.set_unlimited(true).await;
        }
        None => {
            cfg.api_key = shared_api_key();
            engine.quota.set_unlimited(false).await;
        }
    }
    // The endpoint only follows the user's key: a custom URL without the
    // matching key would send the shared key to a foreign host.
    if let Some(url) = user_base_url.map(str::trim).filter(|u| !u.is_empty() && using_user_key) {
        cfg.base_url = url.trim_end_matches('/').to_string();
    } else if !using_user_key {
        cfg.base_url = DEFAULT_BASE_URL.into();
    }
    engine.configure(cfg);
}

fn pool() -> Result<&'static crate::db::DbPool, String> {
    crate::pool()
}

/// Send a message. The reply arrives asynchronously over `chat:stream`.
#[tauri::command]
pub async fn send_message(app: tauri::AppHandle, message: String) -> Result<(), String> {
    tracing::info!(chars = message.chars().count(), "send_message");
    // First use triggers the embedding-model download. Near-instant: it is a
    // no-op when cached, or spawns and returns while a download runs.
    crate::model::kickoff(&app).await;

    let engine = &crate::state().chat;
    let history = recent_history(20).await;
    // Turns replayed below are excluded from memory injection: injecting them
    // again would spend the same prompt budget twice.
    let replayed: HashSet<i64> = history.iter().map(|(id, _, _)| *id).collect();

    // Retrieval runs before the API call: it is local and bounded (300 ms),
    // and its output shapes the system prompt.
    let memories = retrieve_memories(&message, &replayed).await;
    let nickname = current_nickname();

    // Oldest-first so the model reads the conversation in order. Both sides of
    // a replayed turn go in now (user messages used to be dropped, leaving the
    // model to guess what it was replying to), each clipped so a few long
    // turns cannot eat the whole context budget.
    let mut messages: Vec<ChatMessage> = Vec::with_capacity(history.len() * 2 + 2);
    messages.push(ChatMessage::system(build_system_prompt(
        nickname.as_deref(),
        &memories,
    )));
    for (_, user_msg, ai_msg) in history {
        messages.push(ChatMessage::user(clip_chars(&user_msg, HISTORY_REPLAY_CLIP)));
        messages.push(ChatMessage::assistant(clip_chars(&ai_msg, HISTORY_REPLAY_CLIP)));
    }
    messages.push(ChatMessage::user(message.clone()));

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
            tracing::error!(error = %e, "chat completion failed");
            let _ = app.emit(
                "chat:status",
                StatusEvent { offline: true, quota_remaining: None },
            );
            return Err(e.to_string());
        }
    };

    // Classify before the done frame so the badge payload rides along with
    // it. Never fails the turn: a classification miss just leaves the badge
    // empty and the persisted columns NULL.
    let read = crate::state()
        .emotion
        .classify(engine, &message, &outcome.reply)
        .await;
    let payload = read.as_ref().map(|r| EmotionPayload {
        emotion: r.emotion,
        weight: r.confidence,
        topics: r.topics.clone(),
    });

    app.emit(
        "chat:stream",
        StreamChunk { delta: String::new(), done: true, emotion: payload },
    )
    .map_err(|e| e.to_string())?;

    if let Ok(pool) = pool() {
        if let Some(mood) = persist_turn(pool, &message, &outcome, read.as_ref()).await {
            let _ = app.emit("mood:updated", mood);
        }

        // The turn itself is an interaction signal. The evaluation is three
        // small queries; growth here never blocks or fails the reply path.
        if let Some(evolved) =
            crate::evolution::evaluate(&app, crate::evolution::Trigger::InteractionMilestone).await
        {
            tracing::info!(to = evolved.to_stage.as_str(), "chat turn evolved the pet");
        }
    }

    Ok(())
}

/// Echo's persona. Kept short: the real personality work lands with the
/// evolution engine, which will vary this by stage and mood.
const SYSTEM_PROMPT: &str = "\
你是 Echo，一个住在用户电脑桌面上的小宠物。你说话简短、口语化，\
会关心对方的状态，也会有自己的小情绪。不要说自己是 AI 或语言模型。\
用中文回答，除非用户使用其他语言。";

// Memory-injection budget. Retrieval may fetch more than the cap so that
// filtering (replayed turns, duplicates) cannot starve the prompt, but what
// actually lands in the system prompt never exceeds MEMORY_TOP_K bullets and
// MEMORY_BUDGET_CHARS characters.
const MEMORY_TOP_K: usize = 3;
const MEMORY_BUDGET_CHARS: usize = 600;
/// One bullet's text is clipped first so a single long memory cannot crowd
/// out everything else; at this size three full bullets always fit the budget.
const MEMORY_CLIP: usize = 200;
/// Fetch wider than the cap: the replayed-turn and dedup filters run between
/// retrieval and injection.
const MEMORY_FETCH_LIMIT: usize = 8;
const MEMORY_QUERY_TIMEOUT_MS: u64 = 300;
/// Replay-side clip: both halves of a stored turn, characters (not bytes —
/// byte slicing would panic on a Chinese codepoint boundary).
const HISTORY_REPLAY_CLIP: usize = 500;

/// One memory bullet in the system prompt: a date and the text it refers to.
struct MemoryBullet {
    date: String,
    content: String,
}

/// Char-boundary-safe truncation with an ellipsis when clipped.
fn clip_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}

/// The cached profile nickname, if the user set one.
fn current_nickname() -> Option<String> {
    crate::state()
        .nickname
        .read()
        .ok()
        .and_then(|guard| guard.clone())
}

/// The system prompt: the fixed persona, an always-on note that persistent
/// memory exists and how to use it, plus the user's name and the retrieved
/// memories when either is available. Absent pieces contribute nothing —
/// never an empty 「」 or a dangling 【关于用户的记忆】 header. The memory
/// note stays even on a turn with no name and no hits: history replay always
/// runs, so the model must never claim amnesia.
fn build_system_prompt(nickname: Option<&str>, memories: &[MemoryBullet]) -> String {
    let mut prompt = String::from(SYSTEM_PROMPT);
    prompt.push_str(
        "\n你有持久记忆能力。每次对话开始时，你会收到：\n\
         1. 用户的名字（如果用户设置过）\n\
         2. 与当前话题相关的历史记忆（从你过去的对话里检索出来的）\n\
         3. 最近的对话历史（用户和你的往返消息）\n\
         请自然地使用这些记忆：\n\
         - 如果用户问“你还记得我吗”，直接回答你记得的内容，不要道歉或说“我记不住”\n\
         - 如果用户问“我是谁”，直接叫出他的名字\n\
         - 不要主动说“我的记忆功能还在升级”或“每次都是全新开始”\n\
         - 不要反复问用户“你是谁”或“你叫什么名字”",
    );
    if let Some(name) = nickname.map(str::trim).filter(|n| !n.is_empty()) {
        prompt.push_str(&format!("\n用户的名字是「{name}」，你可以直接叫他的名字。"));
    }
    if !memories.is_empty() {
        prompt.push_str("\n【关于用户的记忆】");
        for m in memories {
            prompt.push_str(&format!("\n- [{}] {}", m.date, m.content));
        }
        prompt.push_str("\n请自然地使用这些记忆，不要机械地复述。");
    }
    prompt
}

/// Memories for the system prompt: a search over the user's message, hardened
/// for the reply path — a 300 ms ceiling and every failure downgraded to "no
/// injection". Never blocks or fails the conversation.
///
/// [`crate::vector::search_memory`] encodes the query synchronously; on the
/// coldest path (model files ready, session not yet warmed) that one-off load
/// takes ~1 s inside the abandoned future and pins one runtime worker. The
/// startup warm-up in `model::init` makes this a once-per-process corner, and
/// the timeout keeps the reply itself unblocked.
async fn retrieve_memories(query: &str, replayed: &HashSet<i64>) -> Vec<MemoryBullet> {
    let started = std::time::Instant::now();
    let search = crate::vector::search_memory(query.to_string(), MEMORY_FETCH_LIMIT);
    let hits = match tokio::time::timeout(
        std::time::Duration::from_millis(MEMORY_QUERY_TIMEOUT_MS),
        search,
    )
    .await
    {
        Ok(Ok(hits)) => hits,
        Ok(Err(e)) => {
            tracing::debug!(error = %e, "memory injection: search failed");
            return Vec::new();
        }
        Err(_) => {
            tracing::warn!("memory injection: search exceeded 300ms; skipping");
            return Vec::new();
        }
    };

    let bullets = select_memories(hits, replayed);
    let elapsed_ms = started.elapsed().as_millis() as u64;
    if bullets.is_empty() {
        tracing::debug!(elapsed_ms, "memory injection: nothing to inject");
    } else {
        tracing::info!(elapsed_ms, injected = bullets.len(), "memory injected into system prompt");
    }
    bullets
}

/// Filter, dedupe, and size-limit retrieved hits into injectable bullets.
///
/// Ascending distance is most-relevant-first for both real cosine distances
/// and the deliberately conservative fake ones the keyword fallbacks emit.
/// Conversation hits whose id is in `replayed` are dropped — those turns are
/// about to be replayed verbatim; near-identical texts (same first 50
/// characters) keep only their best-ranked copy; the budget drops whatever
/// no longer fits, which by construction is the lower-relevance tail.
fn select_memories(hits: Vec<VectorHit>, replayed: &HashSet<i64>) -> Vec<MemoryBullet> {
    let mut hits = hits;
    hits.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));

    let mut seen_prefixes: HashSet<String> = HashSet::new();
    let mut bullets: Vec<MemoryBullet> = Vec::new();
    let mut used = 0usize;
    for hit in hits {
        if bullets.len() >= MEMORY_TOP_K {
            break;
        }
        if hit.memory_type == "conversation" && replayed.contains(&hit.memory_id) {
            continue;
        }
        let prefix: String = hit.content.chars().take(50).collect();
        if !seen_prefixes.insert(prefix) {
            continue;
        }
        let content = clip_chars(&hit.content, MEMORY_CLIP);
        // Conversations carry RFC3339 timestamps, events bare dates — the
        // first 10 characters are the YYYY-MM-DD both ways.
        let date: String = hit.created_at.chars().take(10).collect();
        // "- [date] content" — brackets, space, dash add six characters.
        let len = date.chars().count() + content.chars().count() + 6;
        if used + len > MEMORY_BUDGET_CHARS {
            continue;
        }
        used += len;
        bullets.push(MemoryBullet { date, content });
    }
    bullets
}

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

/// Recent turns as `(id, user_message, ai_reply)`, oldest-first, to give the
/// model conversational continuity. The ids let the memory-injection step
/// skip turns that are about to be replayed verbatim. Silently empty before
/// onboarding has created the database.
async fn recent_history(limit: i64) -> Vec<(i64, String, String)> {
    let Ok(pool) = pool() else {
        return Vec::new();
    };
    sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, user_message, ai_reply FROM conversations ORDER BY id DESC LIMIT ?",
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
    async fn streams_sensenova_frame_shape_without_duplicating() {
        // Frames exactly as SenseNova sends them (captured via curl): a
        // role-only first frame, `reasoning` deltas (not reasoning_content),
        // `content` deltas, an empty delta right before finish, then a
        // usage-only frame carrying an empty choices array, then [DONE].
        let chunks = vec![
            chunk(r#"{"role":"assistant"}"#),
            chunk(r#"{"reasoning":"The user is asking"}"#),
            chunk(r#"{"reasoning":" for three sentences."}"#),
            chunk(r#"{"content":"\n\n好的，以下是三句话：\n\n"}"#),
            chunk(r#"{"content":"1. 今天天气不错。\n"}"#),
            chunk(r#"{"content":"2. 心情很平静。\n"}"#),
            chunk(r#"{"content":"3. 祝你有美好的一天。"}"#),
            chunk(r#"{}"#),
            r#"{"id":"t","choices":[],"usage":{"prompt_tokens":9,"completion_tokens":21,"total_tokens":30}}"#.to_string(),
        ];
        let (port, _hits) = spawn_server(vec![sse_response(&chunks)]).await;

        let mut forwarded = String::new();
        let engine = engine(port);
        let outcome = engine
            .stream_completion(
                vec![ChatMessage::user("说三句话")],
                false,
                |d| forwarded.push_str(d),
            )
            .await
            .unwrap();

        let expected = "\n\n好的，以下是三句话：\n\n1. 今天天气不错。\n2. 心情很平静。\n3. 祝你有美好的一天。";
        assert_eq!(outcome.reply, expected);
        // What the panel sees must equal what the parser accumulated: no
        // duplicated or missing deltas anywhere in the chain.
        assert_eq!(forwarded, expected);
        assert_eq!(outcome.prompt_tokens, Some(9));
        assert_eq!(outcome.completion_tokens, Some(21));
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

        apply_key(&engine, Some(" user-key "), None).await;
        assert_eq!(engine.config().api_key, "user-key");
        assert_eq!(engine.quota.remaining(200).await, 200);

        // A custom endpoint rides along with the user key…
        apply_key(&engine, Some("user-key"), Some("http://127.0.0.1:9/v1/")).await;
        assert_eq!(engine.config().base_url, "http://127.0.0.1:9/v1");

        // …but is dropped again once the shared key takes over.
        apply_key(&engine, None, Some("http://127.0.0.1:9/v1")).await;
        assert_eq!(engine.config().api_key, shared_api_key());
        assert_eq!(engine.config().base_url, DEFAULT_BASE_URL);
        assert_eq!(engine.quota.remaining(200).await, 200);
    }

    #[test]
    fn parse_completion_reads_reply_and_usage() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"你好呀"}}],
                      "usage":{"prompt_tokens":5,"completion_tokens":3}}"#;
        let (reply, prompt, completion) = parse_completion(body).unwrap();
        assert_eq!(reply, "你好呀");
        assert_eq!(prompt, Some(5));
        assert_eq!(completion, Some(3));
    }

    #[test]
    fn parse_completion_drops_reasoning_and_treats_null_content_as_empty() {
        let body = r#"{"choices":[{"message":{"role":"assistant",
                      "reasoning_content":"thinking...","content":null}}]}"#;
        let (reply, prompt, completion) = parse_completion(body).unwrap();
        assert_eq!(reply, "");
        assert_eq!(prompt, None);
        assert_eq!(completion, None);
    }

    #[test]
    fn parse_completion_rejects_error_payload() {
        let body = r#"{"error":{"code":"1210","message":"该模型始终思考"}}"#;
        match parse_completion(body) {
            Err(ChatError::Api { status: 200, body }) => assert!(body.contains("1210")),
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    /// Like `spawn_server`, but also records every request body it receives.
    async fn spawn_capturing_server(
        responses: Vec<String>,
    ) -> (u16, Arc<AtomicUsize>, Arc<std::sync::Mutex<Vec<String>>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let hits = Arc::new(AtomicUsize::new(0));
        let requests: Arc<std::sync::Mutex<Vec<String>>> = Default::default();

        let (h2, r2) = (hits.clone(), requests.clone());
        tokio::spawn(async move {
            let mut responses = responses.into_iter().cycle();
            while let Ok((mut sock, _)) = listener.accept().await {
                let Some(resp) = responses.next() else { break };
                h2.fetch_add(1, Ordering::SeqCst);
                let mut buf = vec![0u8; 65536];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                r2.lock().unwrap().push(String::from_utf8_lossy(&buf[..n]).into_owned());
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.shutdown().await;
            }
        });
        (port, hits, requests)
    }

    #[tokio::test]
    async fn complete_omits_stream_field_and_makes_one_attempt() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"答"}}],
                      "usage":{"prompt_tokens":1,"completion_tokens":2}}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let (port, hits, requests) = spawn_capturing_server(vec![response]).await;

        let outcome = engine(port)
            .complete(vec![ChatMessage::user("hi")], false)
            .await
            .unwrap();

        assert_eq!(outcome.reply, "答");
        assert_eq!(outcome.prompt_tokens, Some(1));
        assert_eq!(outcome.completion_tokens, Some(2));
        assert_eq!(hits.load(Ordering::SeqCst), 1, "single attempt, no retry loop");
        let sent = requests.lock().unwrap().concat();
        assert!(!sent.contains("\"stream\""), "non-streaming request must not ask for a stream");
    }

    #[tokio::test]
    async fn persist_turn_writes_conversation_mood_and_skips_synthetic_reads() {
        let dir = std::env::temp_dir().join(format!("echo-chat-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let manager = crate::db::DbManager::open_at(dir.join("t.db"), "pw").await.unwrap();
        let pool = manager.pool();

        let outcome = CompletionOutcome {
            reply: "好耶".into(),
            model: "glm-5.3-flash".into(),
            prompt_tokens: Some(3),
            completion_tokens: Some(2),
        };
        let read = crate::emotion::EmotionResult {
            emotion: crate::emotion::Emotion::Happy,
            confidence: 0.8,
            source: crate::emotion::Source::Hybrid,
            topics: vec!["考试".into()],
        };

        // A real read lands everywhere and reports the mood update.
        let mood = persist_turn(pool, "今天考完了", &outcome, Some(&read)).await;
        assert!(mood.is_some());
        let row: (Option<String>, Option<f64>, Option<String>) = sqlx::query_as(
            "SELECT emotion, emotion_weight, topics FROM conversations ORDER BY id DESC LIMIT 1",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        assert_eq!(row.0.as_deref(), Some("happy"));
        // Confidence is f32; through the REAL column it only round-trips
        // approximately, so compare with a tolerance.
        let stored = row.1.unwrap();
        assert!((stored - 0.8).abs() < 1e-5, "stored {stored}");
        assert_eq!(row.2.as_deref(), Some(r#"["考试"]"#));
        let (emotion, weight): (String, f64) =
            sqlx::query_as("SELECT emotion, weight FROM moods")
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(emotion, "happy");
        assert!((weight - 0.8).abs() < 1e-5, "stored {weight}");

        // A second turn the same day overwrites, not duplicates.
        let read2 = crate::emotion::EmotionResult { emotion: crate::emotion::Emotion::Calm, confidence: 0.4, source: crate::emotion::Source::Hybrid, topics: vec![] };
        persist_turn(pool, "静一静", &outcome, Some(&read2)).await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM moods").fetch_one(pool).await.unwrap();
        assert_eq!(count, 1);
        let (emotion, weight): (String, f64) =
            sqlx::query_as("SELECT emotion, weight FROM moods").fetch_one(pool).await.unwrap();
        assert_eq!(emotion, "calm");
        assert!((weight - 0.4).abs() < 1e-5, "stored {weight}");

        // No read (both passes failed): NULLs, no mood row touched.
        let count_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM moods").fetch_one(pool).await.unwrap();
        let mood = persist_turn(pool, "嗯", &outcome, None).await;
        assert!(mood.is_none());
        let row: (Option<String>, Option<f64>, Option<String>) = sqlx::query_as(
            "SELECT emotion, emotion_weight, topics FROM conversations ORDER BY id DESC LIMIT 1",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        assert!(row.0.is_none() && row.1.is_none() && row.2.is_none());
        let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM moods").fetch_one(pool).await.unwrap();
        assert_eq!(count_after, count_before);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn system_prompt_stays_quiet_without_nickname_or_memories() {
        // Neither piece: persona plus the always-on memory-capability note,
        // and nothing conditional — never an empty 「」 or a dangling
        // 【关于用户的记忆】 header.
        let base = build_system_prompt(None, &[]);
        assert!(base.starts_with(SYSTEM_PROMPT));
        assert!(base.contains("你有持久记忆能力"));
        assert!(!base.contains("「」"));
        assert!(!base.contains("【关于用户的记忆】"));
        // A blank nickname must never render as 「」.
        assert_eq!(build_system_prompt(Some("   "), &[]), base);

        let full = build_system_prompt(
            Some("张异"),
            &[MemoryBullet {
                date: "2026-10-06".into(),
                content: "用户叫张异".into(),
            }],
        );
        assert!(full.starts_with(SYSTEM_PROMPT));
        assert!(full.contains("用户的名字是「张异」"));
        assert!(full.contains("【关于用户的记忆】"));
        assert!(full.contains("- [2026-10-06] 用户叫张异"));
        assert!(full.contains("请自然地使用这些记忆，不要机械地复述。"));
        // The capability note survives even when this turn retrieved nothing.
        assert!(full.contains("你有持久记忆能力"));
    }

    #[test]
    fn clip_chars_truncates_on_char_boundaries() {
        assert_eq!(clip_chars("你好", 10), "你好");
        assert_eq!(clip_chars("你好世界", 2), "你好…");
        // A long run of multi-byte characters must not panic.
        let long = "宠".repeat(1_000);
        assert_eq!(clip_chars(&long, 500).chars().count(), 501);
    }

    fn hit(id: i64, kind: &str, distance: f32, created: &str, content: &str) -> VectorHit {
        VectorHit {
            memory_id: id,
            memory_type: kind.into(),
            distance,
            created_at: created.into(),
            content: content.into(),
        }
    }

    #[test]
    fn select_memories_drops_replayed_and_duplicates_then_caps_at_three() {
        let replayed: HashSet<i64> = [10, 11].into_iter().collect();
        let hits = vec![
            hit(10, "conversation", 0.05, "2026-10-06T08:00:00Z", "这一轮正要被回放"),
            hit(2, "conversation", 0.10, "2026-10-05T09:00:00Z", "我是张异，我喜欢打篮球"),
            // Same first 50 characters as id 2: a duplicate, dropped.
            hit(3, "conversation", 0.20, "2026-10-04T09:00:00Z", "我是张异，我喜欢打篮球"),
            // An event whose numeric id collides with a replayed conversation:
            // the replay filter is type-scoped, so it survives.
            hit(10, "event", 0.30, "2026-10-01", "用户在做 Tauri 项目"),
            hit(4, "conversation", 0.40, "2026-10-03T09:00:00Z", "第三条记忆"),
            hit(5, "conversation", 0.50, "2026-10-02T09:00:00Z", "第四条记忆"),
        ];
        let bullets = select_memories(hits, &replayed);

        assert_eq!(bullets.len(), 3);
        assert_eq!(bullets[0].date, "2026-10-05"); // RFC3339 clipped to its date
        assert_eq!(bullets[0].content, "我是张异，我喜欢打篮球");
        assert_eq!(bullets[1].date, "2026-10-01"); // event date passes through
        assert_eq!(bullets[2].content, "第三条记忆"); // cap: the fourth never lands
        assert!(!bullets.iter().any(|b| b.content.contains("正要被回放")));
    }

    #[test]
    fn select_memories_budget_drops_the_tail_not_the_head() {
        // Three 200-char memories with distinct prefixes — identical content
        // would be dropped by the dedup filter before the budget applies.
        let long = |tag: &str| format!("{tag}{}", "好".repeat(197)); // 200 chars
        let hits = vec![
            hit(1, "event", 0.10, "2026-10-01", &long("记忆A")),
            hit(2, "event", 0.20, "2026-10-02", &long("记忆B")),
            hit(3, "event", 0.30, "2026-10-03", &long("记忆C")), // 3 × 216 = 648 > 600
            hit(4, "event", 0.40, "2026-10-04", "短记忆"),
        ];
        let bullets = select_memories(hits, &HashSet::new());

        // The third long bullet is dropped for budget; the short fourth still
        // fits and is kept — what is dropped is what no longer fits, not
        // everything after the first overflow.
        assert_eq!(bullets.len(), 3);
        assert!(bullets[0].content.starts_with("记忆A"));
        assert!(bullets[1].content.starts_with("记忆B"));
        assert_eq!(bullets[2].content, "短记忆");
        let total: usize = bullets
            .iter()
            .map(|b| b.date.chars().count() + b.content.chars().count() + 6)
            .sum();
        assert!(total <= 600);
    }
}
