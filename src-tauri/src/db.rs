//! Encrypted local storage.
//!
//! Everything the pet remembers lives in one SQLCipher database inside the
//! app's data directory. There is no server and no sync: the file *is* the
//! user's data, and the backup/restore commands move it around.

use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use tauri::{AppHandle, Manager};
use thiserror::Error;

pub type DbPool = SqlitePool;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migration failed: {0}")]
    Migration(String),
    #[error("wrong password, or the database is damaged")]
    BadPassword,
    #[error("vector extension unavailable: {0}")]
    VectorExt(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("path error: {0}")]
    Path(#[from] tauri::Error),
    #[error("not initialised yet")]
    NotReady,
}

/// Open (creating if needed) the encrypted database and run migrations.
pub async fn open(app: &AppHandle, password: &str) -> Result<DbManager, DbError> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("echo.db");
    DbManager::open_at(path, password).await
}

pub struct DbManager {
    pool: DbPool,
    path: PathBuf,
    vec_available: bool,
}

impl Clone for DbManager {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            path: self.path.clone(),
            vec_available: self.vec_available,
        }
    }
}

/// Load the sqlite-vec extension and create the vector table if it is there.
///
/// Returns `false` on builds without the extension (currently Android without
/// the prebuilt `.so`). Callers fall back to FTS5 keyword search rather than
/// failing, so this is a capability probe, not a hard requirement.
pub async fn ensure_vector_table(pool: &DbPool) -> bool {
    // The desktop build compiles sqlite-vec in; `vec_version()` is the cheapest
    // way to find out whether the extension is actually usable.
    if sqlx::query("SELECT vec_version()")
        .fetch_one(pool)
        .await
        .is_err()
    {
        return false;
    }

    let created = sqlx::query(
        "CREATE VIRTUAL TABLE IF NOT EXISTS vec_memories USING vec0(
            embedding FLOAT[384],
            memory_id INTEGER,
            memory_type TEXT
        )",
    )
    .execute(pool)
    .await;

    if let Err(e) = created {
        tracing::warn!("sqlite-vec present but vec_memories failed: {e}");
        return false;
    }

    // Shadow-table indexes make the time/type filters used by mixed queries
    // cheap. Best-effort: an older sqlite-vec may not expose them.
    for stmt in [
        "CREATE INDEX IF NOT EXISTS idx_vec_type ON vec_memories(memory_type)",
    ] {
        if let Err(e) = sqlx::query(stmt).execute(pool).await {
            tracing::debug!("optional vector index skipped: {e}");
        }
    }

    true
}

impl DbManager {
    pub async fn open_at(path: PathBuf, password: &str) -> Result<Self, DbError> {
        let key = derive_key(password);

        let opts = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            // SQLCipher's raw-key form needs the double quotes: the pragma
            // value grammar accepts no blob literals, and the quotes are what
            // tell SQLCipher the x'…' text is a raw key, not a passphrase.
            // Same shape as the `rekey` statement below.
            .pragma("key", format!(r#""x'{}'""#, hex_encode(&key)))
            // SQLCipher v4 defaults otherwise (kdf_iter 256000, page 4096).
            .pragma("cipher_page_size", "4096")
            .pragma("journal_mode", "WAL")
            .pragma("foreign_keys", "ON")
            .pragma("busy_timeout", "5000");

        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await
            .map_err(|e| {
                // SQLCipher reports a wrong key as a generic "file is not a database".
                if e.to_string().contains("not a database") {
                    DbError::BadPassword
                } else {
                    DbError::Sqlx(e)
                }
            })?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| DbError::Migration(e.to_string()))?;
        let vec_available = ensure_vector_table(&pool).await;
        tracing::info!(
            vec_available,
            path = %path.display(),
            "database ready"
        );
        Ok(Self { pool, path, vec_available })
    }

    /// Whether `sqlite-vec` loaded, which decides if semantic search is usable.
    pub fn vec_available(&self) -> bool {
        self.vec_available
    }

    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Change the encryption password in place (`PRAGMA rekey`).
    pub async fn rekey(&self, new_password: &str) -> Result<(), DbError> {
        let key = hex_encode(&derive_key(new_password));
        sqlx::query(&format!(r#"PRAGMA rekey = "x'{key}'""#))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Write a consistent snapshot to `dest` using SQLite's online backup, so
    /// the copy is valid even if writes are in flight.
    pub async fn backup_to(&self, dest: &Path) -> Result<(), DbError> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let escaped = dest.to_string_lossy().replace('\'', "''");
        sqlx::query(&format!("VACUUM INTO '{escaped}'"))
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

/// PBKDF2-HMAC-SHA256 with a fixed application salt.
///
/// A per-installation salt would have to be stored next to the database, which
/// gives an attacker who reads the data directory no extra protection. The
/// threat model here is "someone gets a copy of the file", so a constant salt
/// still forces an offline brute force of the password.
fn derive_key(password: &str) -> [u8; 32] {
    const SALT: &[u8] = b"echo.pet.v1.static.salt";
    const ITERATIONS: u32 = 100_000;
    pbkdf2::pbkdf2_hmac_array::<sha2::Sha256, 32>(password.as_bytes(), SALT, ITERATIONS)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------------------
// Domain types (mirrored by `src/lib/api/types.ts`)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub nickname: String,
    pub birthday: String,
    pub theme: String,
    pub notifications: bool,
    pub auto_start: bool,
    pub user_api_key: Option<String>,
    /// OpenAI-compatible endpoint to pair with a user key (e.g. an
    /// OpenRouter or local vLLM URL). Empty = the built-in default.
    #[serde(default)]
    pub user_base_url: Option<String>,
    pub model_preference: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            nickname: String::new(),
            birthday: String::new(),
            theme: "auto".into(),
            notifications: true,
            auto_start: true,
            user_api_key: None,
            user_base_url: None,
            model_preference: "auto".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub nickname: String,
    pub birthday: String,
    pub install_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MoodEntry {
    pub id: i64,
    pub date: String,
    pub emotion: String,
    pub weight: f64,
    pub note: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Event {
    pub id: i64,
    pub date: String,
    pub description: String,
    #[sqlx(rename = "type")]
    pub kind: String,
    pub importance: i32,
    pub tags_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Conversation {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub user_message: String,
    pub ai_reply: String,
    pub emotion: Option<String>,
    pub emotion_weight: Option<f64>,
    pub tokens_used: Option<i32>,
    pub model_used: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineItem {
    pub date: String,
    pub events: Vec<Event>,
    pub mood: Option<MoodEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct VectorHit {
    pub memory_id: i64,
    pub memory_type: String,
    pub distance: f32,
    /// When the memory happened: conversations carry their RFC3339 timestamp,
    /// events their local date.
    pub created_at: String,
    /// A short text excerpt of the memory, for display in search results.
    pub content: String,
}

pub fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn pool() -> Result<&'static DbPool, String> {
    crate::pool()
}

#[tauri::command]
pub async fn get_settings() -> Result<Settings, String> {
    let pool = pool()?;
    let json: Option<String> = sqlx::query("SELECT settings_json FROM profiles WHERE id = 1")
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .map(|r| r.get("settings_json"));

    Ok(json
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default())
}

#[tauri::command]
pub async fn update_settings(patch: serde_json::Value) -> Result<(), String> {
    let pool = pool()?;
    let current = get_settings().await?;
    let mut merged = serde_json::to_value(current).map_err(|e| e.to_string())?;
    json_patch::merge(&mut merged, &patch);
    let json = serde_json::to_string(&merged).map_err(|e| e.to_string())?;

    sqlx::query("UPDATE profiles SET settings_json = ?, updated_at = datetime('now') WHERE id = 1")
        .bind(json)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    // A swapped key takes effect on the next message without a restart.
    if patch.get("user_api_key").is_some() {
        crate::onboarding::apply_chat_key_from_db(pool).await;
    }
    Ok(())
}
