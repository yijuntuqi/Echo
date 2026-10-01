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

    /// Nearest neighbours by cosine distance. Uses sqlite-vec's kNN query
    /// against `vec_memories` and joins the source tables so each hit carries
    /// its timestamp and a text excerpt for display.
    pub async fn search(
        &self,
        pool: Option<&crate::db::DbPool>,
        query: &[f32],
        top_k: usize,
    ) -> Result<Vec<VectorHit>, String> {
        if !self.vec_available() {
            return Err("semantic search unavailable: vector extension not loaded".into());
        }
        let pool = pool.ok_or("no database pool")?;
        // sqlite-vec accepts little-endian f32 blobs.
        let blob: Vec<u8> = query.iter().flat_map(|f| f.to_le_bytes()).collect();
        sqlx::query_as::<_, VectorHit>(
            "SELECT v.memory_id, v.memory_type, v.distance,
                    COALESCE(c.timestamp, e.date, '') AS created_at,
                    COALESCE(
                        NULLIF(TRIM(c.user_message || ' ' || c.ai_reply), ''),
                        e.description, ''
                    ) AS content
             FROM (SELECT memory_id, memory_type, distance
                   FROM vec_memories
                   WHERE embedding MATCH ? AND k = ?) v
             LEFT JOIN conversations c
                    ON v.memory_type = 'conversation' AND c.id = v.memory_id
             LEFT JOIN events e
                    ON v.memory_type = 'event' AND e.id = v.memory_id
             ORDER BY v.distance
             LIMIT ?",
        )
        .bind(blob)
        .bind(top_k as i64)
        .bind(top_k as i64)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())
    }
}

/// Index one memory for semantic search. Best-effort: without the vector
/// extension the memory simply stays unindexed (lexical search still finds it).
pub async fn store_memory(
    pool: &crate::db::DbPool,
    embedding: &[f32],
    memory_id: i64,
    memory_type: &str,
) {
    let engine = &crate::state().vectors;
    if !engine.vec_available() {
        return;
    }
    let blob: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
    // Upsert by (type, id): a re-encoded memory replaces its old vector.
    if let Err(e) = sqlx::query("DELETE FROM vec_memories WHERE memory_id = ? AND memory_type = ?")
        .bind(memory_id)
        .bind(memory_type)
        .execute(pool)
        .await
    {
        tracing::debug!(error = %e, "vector upsert delete failed");
        return;
    }
    if let Err(e) = sqlx::query(
        "INSERT INTO vec_memories(embedding, memory_id, memory_type) VALUES (?, ?, ?)",
    )
    .bind(blob)
    .bind(memory_id)
    .bind(memory_type)
    .execute(pool)
    .await
    {
        tracing::debug!(error = %e, "vector index insert failed");
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

    // Lexical fallback: FTS5 if present, otherwise a LIKE scan over both
    // conversations and events. Each hit carries its timestamp and a text
    // excerpt; the pseudo-distance reflects where the match landed so the
    // UI's relevance percentage is not uniformly 100%.
    match sqlx::query_as::<_, VectorHit>(
        "SELECT c.id AS memory_id, 'conversation' AS memory_type,
                0.1 AS distance,
                c.timestamp AS created_at,
                TRIM(c.user_message || ' ' || c.ai_reply) AS content
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
                "SELECT id AS memory_id, 'conversation' AS memory_type,
                        CASE WHEN user_message LIKE ? THEN 0.05 ELSE 0.2 END AS distance,
                        timestamp AS created_at,
                        TRIM(user_message || ' ' || ai_reply) AS content
                 FROM conversations
                 WHERE user_message LIKE ? OR ai_reply LIKE ?
                 UNION ALL
                 SELECT id, 'event', 0.2 AS distance, date AS created_at,
                        description AS content
                 FROM events
                 WHERE description LIKE ?
                 ORDER BY distance, created_at DESC
                 LIMIT ?",
            )
            .bind(&pattern)
            .bind(&pattern)
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

    // Index the event for semantic search; skipped silently when the
    // embedding model is not ready (lexical search still finds it).
    if let Some(embedding) = crate::state().embedding.get() {
        if let Ok(vec) = embedding.encode(&event.description) {
            store_memory(pool, &vec, id, "event").await;
        }
    }
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
