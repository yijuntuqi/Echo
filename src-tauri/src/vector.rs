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
        // sqlite-vec accepts little-endian f32 blobs. kNN uses the LIMIT
        // form only: sqlite-vec rejects a query that combines `k = ?` with
        // a LIMIT (an outer LIMIT gets pushed down into the vec0 scan).
        // The LEFT JOINs cannot fan out (both join columns are primary
        // keys), so no outer LIMIT is needed after the kNN subquery.
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
                   WHERE embedding MATCH ?
                   LIMIT ?) v
             LEFT JOIN conversations c
                    ON v.memory_type = 'conversation' AND c.id = v.memory_id
             LEFT JOIN events e
                    ON v.memory_type = 'event' AND e.id = v.memory_id
             ORDER BY v.distance",
        )
        .bind(blob)
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
        return;
    }
    // Record the successful indexing so the backfill pass can tell which
    // memories still need encoding.
    if let Err(e) = sqlx::query(
        "INSERT OR REPLACE INTO vec_indexed(memory_type, memory_id) VALUES (?, ?)",
    )
    .bind(memory_type)
    .bind(memory_id)
    .execute(pool)
    .await
    {
        tracing::debug!(error = %e, "vec_indexed bookkeeping failed");
    }
}

/// Encode and index every memory that predates a working model.
///
/// The stub era left the vector table empty: everything the user ever said
/// was only findable by keyword. One bounded pass per call (newest first);
/// called in the background whenever the model becomes ready — after a
/// download, or at startup when it was already cached. If more than the
/// limit remains, the next trigger picks up the rest.
pub async fn backfill_memories() {
    let pool = match crate::pool() {
        Ok(p) => p,
        Err(e) => {
            tracing::debug!(error = %e, "backfill skipped: database not open yet");
            return;
        }
    };
    let state = crate::state();
    let Some(embedding) = state.embedding.get() else {
        tracing::debug!("backfill skipped: embedding service unavailable");
        return;
    };
    if !state.vectors.vec_available() || !embedding.is_ready() {
        tracing::debug!(
            vec_available = state.vectors.vec_available(),
            model_ready = embedding.is_ready(),
            "backfill skipped: pipeline not ready"
        );
        return;
    }

    let mut indexed = 0usize;
    let turns: Vec<(i64, String)> = match sqlx::query_as(
        "SELECT c.id, TRIM(c.user_message || ' ' || c.ai_reply)
         FROM conversations c
         LEFT JOIN vec_indexed v ON v.memory_type = 'conversation' AND v.memory_id = c.id
         WHERE v.memory_id IS NULL
         ORDER BY c.id DESC
         LIMIT 500",
    )
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "memory backfill: conversation query failed");
            return;
        }
    };
    for (id, text) in turns {
        // Same shape as the per-turn indexer in chat.rs.
        let text: String = text.chars().take(600).collect();
        match embedding.encode(&text) {
            Ok(vec) => {
                store_memory(pool, &vec, id, "conversation").await;
                indexed += 1;
            }
            Err(e) => tracing::debug!(error = %e, id, "backfill: encode failed"),
        }
    }

    let events: Vec<(i64, String)> = match sqlx::query_as(
        "SELECT e.id, e.description
         FROM events e
         LEFT JOIN vec_indexed v ON v.memory_type = 'event' AND v.memory_id = e.id
         WHERE v.memory_id IS NULL
         ORDER BY e.id DESC
         LIMIT 500",
    )
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "memory backfill: event query failed");
            return;
        }
    };
    for (id, text) in events {
        match embedding.encode(&text) {
            Ok(vec) => {
                store_memory(pool, &vec, id, "event").await;
                indexed += 1;
            }
            Err(e) => tracing::debug!(error = %e, id, "backfill: encode failed"),
        }
    }

    if indexed > 0 {
        tracing::info!(indexed, "memory backfill pass indexed memories");
    } else {
        tracing::debug!("memory backfill pass found nothing to index");
    }
}

/// Live status of the semantic-search pipeline, for diagnosing why a search
/// fell back to keywords. Row counts read the real tables; `null` counts
/// mean the table does not exist (extension never became available).
#[tauri::command]
pub async fn vector_debug() -> Result<serde_json::Value, String> {
    use sqlx::Row;

    let state = crate::state();
    let model_ready = state.embedding.get().map(|e| e.is_ready()).unwrap_or(false);
    let mut out = serde_json::json!({
        "vec_available": state.vectors.vec_available(),
        "model_ready": model_ready,
    });
    if let Ok(pool) = crate::pool() {
        let count = |sql: &'static str| {
            let pool = pool.clone();
            async move {
                sqlx::query(sql)
                    .fetch_one(&pool)
                    .await
                    .ok()
                    .and_then(|r| r.try_get::<i64, _>(0).ok())
            }
        };
        out["vec_memories_rows"] = match count("SELECT COUNT(*) FROM vec_memories").await {
            Some(n) => serde_json::json!(n),
            None => serde_json::Value::Null,
        };
        out["vec_indexed_rows"] = match count("SELECT COUNT(*) FROM vec_indexed").await {
            Some(n) => serde_json::json!(n),
            None => serde_json::Value::Null,
        };
        out["conversations"] = serde_json::json!(
            count("SELECT COUNT(*) FROM conversations").await.unwrap_or(-1)
        );
        out["events"] = serde_json::json!(count("SELECT COUNT(*) FROM events").await.unwrap_or(-1));
    } else {
        out["database"] = serde_json::json!("closed (onboarding not finished)");
    }
    Ok(out)
}

fn pool() -> Result<&'static crate::db::DbPool, String> {
    crate::pool()
}

#[tauri::command]
pub async fn search_memory(query: String, top_k: usize) -> Result<Vec<VectorHit>, String> {
    let pool = pool()?;
    let engine = &crate::state().vectors;

    // Semantic path, when both the extension and the model are present.
    // Queries go through encode_query: bge-zh-v1.5 expects the retrieval
    // instruction on the query side only. Every fallback reason is logged —
    // a silent keyword-only degradation cost us weeks.
    if !engine.vec_available() {
        tracing::warn!("semantic search skipped: vector extension unavailable");
    } else if let Some(embedding) = crate::state().embedding.get() {
        match embedding.encode_query(&query) {
            Ok(vector) => match engine.search(Some(pool), &vector, top_k).await {
                Ok(hits) if !hits.is_empty() => {
                    tracing::info!(hits = hits.len(), "semantic search hit");
                    return Ok(hits);
                }
                Ok(_) => tracing::info!("semantic search empty; trying keyword fallback"),
                Err(e) => {
                    tracing::warn!(error = %e, "semantic search query failed; trying keyword fallback")
                }
            },
            Err(e) => tracing::warn!(error = %e, "query encode failed; trying keyword fallback"),
        }
    } else {
        tracing::warn!("semantic search skipped: embedding service unavailable");
    }

    // Lexical fallback: FTS5 if present, otherwise a LIKE scan over both
    // conversations and events. These hits have no query vector behind them
    // (the memory may not even be indexed), so their "distance" is a rank
    // proxy, deliberately conservative: a keyword match must not dress up
    // as a 95% semantic match. 0.4 ≈ 80% / 0.5 ≈ 75% in the UI's mapping.
    match sqlx::query_as::<_, VectorHit>(
        "SELECT c.id AS memory_id, 'conversation' AS memory_type,
                0.4 AS distance,
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
                        CASE WHEN user_message LIKE ? THEN 0.35 ELSE 0.5 END AS distance,
                        timestamp AS created_at,
                        TRIM(user_message || ' ' || ai_reply) AS content
                 FROM conversations
                 WHERE user_message LIKE ? OR ai_reply LIKE ?
                 UNION ALL
                 SELECT id, 'event', 0.5 AS distance, date AS created_at,
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
