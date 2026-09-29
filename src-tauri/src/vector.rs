//! Vector engine: sqlite-vec wrapper

use sqlx::SqlitePool;
use crate::db::DbError;

pub struct VectorEngine {
    pool: SqlitePool,
}

impl VectorEngine {
    pub fn new(pool: SqlitePool) -> Self { Self { pool } }
    
    pub async fn index_memory(&self, _memory_id: i64, _memory_type: &str, _embedding: &[f32]) -> Result<(), DbError> {
        Ok(())
    }
    
    pub async fn search(&self, _query_vec: &[f32], _top_k: usize, _filter: Option<VectorFilter>) -> Result<Vec<VectorHit>, DbError> {
        Ok(vec![])
    }
    
    pub async fn rebuild_all(&self) -> Result<(), DbError> {
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct VectorHit {
    pub memory_id: i64,
    pub memory_type: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub distance: f32,
}

#[derive(Debug, Clone, Default)]
pub struct VectorFilter {
    pub memory_type: Option<String>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
}

#[tauri::command]
pub async fn search_memory(query: String, top_k: usize) -> Result<Vec<VectorHit>, String> {
    Ok(vec![])
}

#[tauri::command]
pub async fn add_event(event: crate::db::Event) -> Result<i64, String> {
    Ok(0)
}

#[tauri::command]
pub async fn get_timeline(range: crate::db::DateRange, limit: usize) -> Result<Vec<crate::db::TimelineItem>, String> {
    Ok(vec![])
}