//! Database layer: SQLCipher + sqlx + migrations + repositories

use sqlx::{sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqlitePool}, Pool, Sqlite, Row};
use std::path::PathBuf;
use tauri::AppHandle;
use thiserror::Error;
use once_cell::sync::Lazy;
use std::sync::Mutex;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLx error: {0}")] Sqlx(#[from] sqlx::Error),
    #[error("Migration failed: {0}")] Migration(String),
    #[error("Invalid password or corrupted database")] InvalidPassword,
    #[error("Backup failed: {0}")] Backup(String),
    #[error("Vector extension not loaded")] VecExtMissing,
    #[error("IO error: {0}")] Io(#[from] std::io::Error),
    #[error("Key derivation failed: {0}")] KeyDerivation(String),
}

pub type DbPool = Pool<Sqlite>;

pub struct DbManager {
    pub pool: DbPool,
    db_path: PathBuf,
    key: [u8; 32],
}

static DEVICE_SALT: Lazy<Mutex<Option<[u8; 16]>>> = Lazy::new(|| Mutex::new(None));

fn get_or_create_device_salt(db_dir: &PathBuf) -> Result<[u8; 16], DbError> {
    let mut salt_guard = DEVICE_SALT.lock().unwrap();
    if let Some(salt) = *salt_guard {
        return Ok(salt);
    }
    
    let salt_path = db_dir.join(".device_salt");
    let salt = if salt_path.exists() {
        let bytes = std::fs::read(&salt_path)?;
        if bytes.len() == 16 {
            let mut arr = [0u8; 16];
            arr.copy_from_slice(&bytes);
            arr
        } else {
            return Err(DbError::KeyDerivation("Invalid salt length".into()));
        }
    } else {
        use rand::RngCore;
        let mut salt = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut salt);
        std::fs::write(&salt_path, &salt)?;
        salt
    };
    
    *salt_guard = Some(salt);
    Ok(salt)
}

fn derive_key(password: &str, salt: &[u8; 16]) -> [u8; 32] {
    use pbkdf2::pbkdf2_hmac_array;
    use sha2::Sha256;
    pbkdf2_hmac_array::<Sha256, 32>(password.as_bytes(), salt, 100_000)
}

impl DbManager {
    pub async fn new(app: &AppHandle, password: &str) -> Result<Self, DbError> {
        let db_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&db_dir)?;
        let db_path = db_dir.join("echo.db");
        
        let salt = get_or_create_device_salt(&db_dir)?;
        let key = derive_key(password, &salt);
        
        let key_hex = hex::encode(key);
        
        let opts = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .pragma("key", format!("x'{}'", key_hex))
            .pragma("cipher_page_size", "4096")
            .pragma("kdf_iter", "100000")
            .pragma("cipher", "aes-256-cbc")
            .pragma("journal_mode", "WAL")
            .pragma("synchronous", "NORMAL")
            .pragma("foreign_keys", "ON");
        
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await?;
        
        // 尝试加载 sqlite-vec 扩展
        let _ = sqlx::query("SELECT load_extension('vec0')").execute(&pool).await;
        
        run_migrations(&pool).await?;
        
        Ok(Self { pool, db_path, key })
    }
    
    pub fn pool(&self) -> &DbPool { &self.pool }
    
    pub async fn rekey(&self, new_password: &str) -> Result<(), DbError> {
        let salt = get_or_create_device_salt(&self.db_path.parent().unwrap().to_path_buf())?;
        let new_key = derive_key(new_password, &salt);
        let new_key_hex = hex::encode(new_key);
        sqlx::query(&format!("PRAGMA rekey = \"x'{}'\"", new_key_hex))
            .execute(&self.pool).await?;
        Ok(())
    }
    
    pub async fn backup_to(&self, dest_path: &PathBuf) -> Result<(), DbError> {
        use sqlx::Connection;
        let mut conn = self.pool.acquire().await?;
        let backup_sql = format!("VACUUM INTO '{}'", dest_path.to_string_lossy().replace('\\', "\\\\"));
        sqlx::query(&backup_sql).execute(&mut *conn).await?;
        Ok(())
    }
}

async fn run_migrations(pool: &DbPool) -> Result<(), DbError> {
    sqlx::migrate!("./migrations").run(pool).await.map_err(|e| DbError::Migration(e.to_string()))?;
    Ok(())
}

// Repository traits
#[async_trait::async_trait]
pub trait ProfileRepo: Send + Sync {
    async fn get(&self) -> Result<Option<Profile>, DbError>;
    async fn upsert(&self, profile: &Profile) -> Result<(), DbError>;
    async fn update_settings(&self, settings: &Settings) -> Result<(), DbError>;
}

#[async_trait::async_trait]
pub trait MoodRepo: Send + Sync {
    async fn upsert_daily(&self, mood: &MoodEntry) -> Result<(), DbError>;
    async fn get_range(&self, start: chrono::NaiveDate, end: chrono::NaiveDate) -> Result<Vec<MoodEntry>, DbError>;
}

#[async_trait::async_trait]
pub trait EventRepo: Send + Sync {
    async fn insert(&self, event: &Event) -> Result<i64, DbError>;
    async fn get_by_date(&self, date: chrono::NaiveDate) -> Result<Vec<Event>, DbError>;
    async fn get_timeline(&self, range: DateRange, limit: usize) -> Result<Vec<TimelineItem>, DbError>;
}

#[async_trait::async_trait]
pub trait ConversationRepo: Send + Sync {
    async fn append(&self, conv: &Conversation) -> Result<i64, DbError>;
    async fn list_paginated(&self, limit: usize, offset: usize) -> Result<Vec<Conversation>, DbError>;
    async fn search_fts(&self, query: &str, limit: usize) -> Result<Vec<Conversation>, DbError>;
}

#[async_trait::async_trait]
pub trait VectorRepo: Send + Sync {
    async fn upsert_embedding(&self, memory_id: i64, memory_type: &str, embedding: &[f32]) -> Result<(), DbError>;
    async fn search_similar(&self, query_vec: &[f32], top_k: usize, filter: Option<VectorFilter>) -> Result<Vec<VectorHit>, DbError>;
    async fn delete_by_memory_id(&self, memory_id: i64, memory_type: &str) -> Result<(), DbError>;
}

// Data structures
use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Profile {
    pub id: i64,
    pub nickname: String,
    pub birthday: NaiveDate,
    pub install_date: NaiveDate,
    pub settings_json: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub theme: String,
    pub notifications: bool,
    pub auto_start: bool,
    pub user_api_key: Option<String>,
    pub model_preference: String,
    pub backup_password_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MoodEntry {
    pub id: i64,
    pub date: NaiveDate,
    pub emotion: String,
    pub weight: f64,
    pub note: Option<String>,
    pub source: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Event {
    pub id: i64,
    pub date: NaiveDate,
    pub description: String,
    pub type_: String,
    pub importance: i32,
    pub tags_json: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineItem {
    pub date: NaiveDate,
    pub events: Vec<Event>,
    pub mood: Option<MoodEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Conversation {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub user_message: String,
    pub ai_reply: String,
    pub emotion: Option<String>,
    pub emotion_weight: Option<f64>,
    pub topics: Option<String>,
    pub tokens_used: Option<i32>,
    pub model_used: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorHit {
    pub memory_id: i64,
    pub memory_type: String,
    pub created_at: DateTime<Utc>,
    pub distance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFilter {
    pub memory_type: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

// Settings commands
#[tauri::command]
pub async fn get_settings(db: tauri::State<'_, DbManager>) -> Result<Settings, String> {
    let row = sqlx::query("SELECT settings_json FROM profiles WHERE id = 1")
        .fetch_optional(&db.pool)
        .await
        .map_err(|e| e.to_string())?;
    
    if let Some(row) = row {
        let json: String = row.get("settings_json");
        serde_json::from_str(&json).map_err(|e| e.to_string())
    } else {
        Ok(Settings {
            theme: "auto".into(),
            notifications: true,
            auto_start: true,
            user_api_key: None,
            model_preference: "auto".into(),
            backup_password_hash: None,
        })
    }
}

#[tauri::command]
pub async fn update_settings(db: tauri::State<'_, DbManager>, patch: serde_json::Value) -> Result<(), String> {
    let current = get_settings(db.clone()).await?;
    let mut merged = serde_json::to_value(current).map_err(|e| e.to_string())?;
    json_patch::merge(&mut merged, &patch);
    let json = serde_json::to_string(&merged).map_err(|e| e.to_string())?;
    
    sqlx::query("UPDATE profiles SET settings_json = ?, updated_at = datetime('now') WHERE id = 1")
        .bind(&json)
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn export_backup(db: tauri::State<'_, DbManager>, password: String, dest_path: String) -> Result<String, String> {
    let path = PathBuf::from(dest_path);
    db.backup_to(&path).await.map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn import_backup(db: tauri::State<'_, DbManager>, password: String, src_path: String) -> Result<(), String> {
    // TODO: 实现备份恢复
    Err("Not implemented yet".into())
}