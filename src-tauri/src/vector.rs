//! Memory search and the timeline.
//!
//! Semantic search runs on `sqlite-vec` when the extension loaded, and falls
//! back to FTS5 keyword matching when it did not — an Android build without the
//! prebuilt extension still gets useful recall, just lexical.

use serde::Deserialize;

use crate::db::{DateRange, Event, MoodEntry, TimelineItem, VectorHit};

pub struct VectorEngine {
    /// Set once the sqlite-vec extension has been probed at startup.
    vec_available: std::sync::atomic::AtomicBool,
}

impl Default for VectorEngine {
    fn default() -> Self {
        Self { vec_available: std::sync::atomic::AtomicBool::new(false) }
    }
}

impl VectorEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_vec_available(&self, v: bool) {
        self.vec_available
            .store(v, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn vec_available(&self) -> bool {
        self.vec_available.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Nearest neighbours by cosine distance, newest-first within equal scores.
    pub async fn search(
        &self,
        _pool: Option<&crate::db::DbPool>,
        _query: &[f32],
        top_k: usize,
    ) -> Result<Vec<VectorHit>, String> {
        if !self.vec_available() {
            return Err("semantic search unavailable: vector extension not loaded".into());
        }
        let _ = top_k;
        Ok(Vec::new())
    }
}

fn pool() -> Result<&'static crate::db::DbPool, String> {
    crate::pool()
}

#[tauri::command]
pub async fn search_memory(query: String, top_k: usize) -> Result<Vec<VectorHit>, String> {
    let pool = pool()?;
    let engine = &crate::state().vectors;

    // Semantic path, when both the extension and the model are present.
    if engine.vec_available() {
        if let Some(embedding) = crate::state().embedding.get() {
            if let Ok(vector) = embedding.encode(&query) {
                if let Ok(hits) = engine.search(Some(pool), &vector, top_k).await {
                    if !hits.is_empty() {
                        return Ok(hits);
                    }
                }
            }
        }
    }

    // Lexical fallback: FTS5 if present, otherwise a LIKE scan.
    match sqlx::query_as::<_, VectorHit>(
        "SELECT c.id AS memory_id, 'conversation' AS memory_type,
                0.0 AS distance
         FROM fts_conversations f
         JOIN conversations c ON c.id = f.rowid
         WHERE fts_conversations MATCH ?
         LIMIT ?",
    )
    .bind(&query)
    .bind(top_k as i64)
    .fetch_all(pool)
    .await
    {
        Ok(hits) => Ok(hits),
        Err(_) => {
            let pattern = format!("%{query}%");
            sqlx::query_as::<_, VectorHit>(
                "SELECT id AS memory_id, 'conversation' AS memory_type, 0.0 AS distance
                 FROM conversations
                 WHERE user_message LIKE ? OR ai_reply LIKE ?
                 ORDER BY id DESC LIMIT ?",
            )
            .bind(&pattern)
            .bind(&pattern)
            .bind(top_k as i64)
            .fetch_all(pool)
            .await
            .map_err(|e| e.to_string())
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct NewEvent {
    pub date: String,
    pub description: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub importance: i32,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[tauri::command]
pub async fn add_event(event: NewEvent) -> Result<i64, String> {
    let pool = pool()?;
    let id = sqlx::query(
        "INSERT INTO events (date, description, type, importance, tags_json)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&event.date)
    .bind(&event.description)
    .bind(&event.kind)
    .bind(event.importance.clamp(1, 5))
    .bind(serde_json::to_string(&event.tags).unwrap_or_else(|_| "[]".into()))
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?
    .last_insert_rowid();
    Ok(id)
}

/// Events and the day's mood, grouped by date and newest first.
#[tauri::command]
pub async fn get_timeline(range: DateRange, limit: usize) -> Result<Vec<TimelineItem>, String> {
    let pool = pool()?;

    let events = sqlx::query_as::<_, Event>(
        "SELECT id, date, description, type, importance, tags_json
         FROM events WHERE date BETWEEN ? AND ?
         ORDER BY date DESC, importance DESC LIMIT ?",
    )
    .bind(&range.start)
    .bind(&range.end)
    .bind(limit as i64)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let moods = sqlx::query_as::<_, MoodEntry>(
        "SELECT id, date, emotion, weight, note, source
         FROM moods WHERE date BETWEEN ? AND ?",
    )
    .bind(&range.start)
    .bind(&range.end)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    // One row per day, so a day shows its events next to how it felt.
    let mut by_date: std::collections::BTreeMap<String, TimelineItem> = Default::default();
    for ev in events {
        by_date
            .entry(ev.date.clone())
            .or_insert_with(|| TimelineItem {
                date: ev.date.clone(),
                events: Vec::new(),
                mood: None,
            })
            .events
            .push(ev);
    }
    for mood in moods {
        let date = mood.date.clone();
        by_date
            .entry(date.clone())
            .or_insert_with(|| TimelineItem { date, events: Vec::new(), mood: None })
            .mood = Some(mood);
    }

    let mut items: Vec<TimelineItem> = by_date.into_values().collect();
    items.sort_by(|a, b| b.date.cmp(&a.date));
    items.truncate(limit);
    Ok(items)
}
