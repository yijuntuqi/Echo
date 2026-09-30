//! Encrypted backup and restore.
//!
//! A backup is a SQLCipher snapshot: copying the database with the same
//! password yields a working, still-encrypted copy. There is no separate
//! re-encryption step, so the password the app generated is the only key
//! anyone ever needs — and the user never has to type it.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

pub struct BackupManager;

impl BackupManager {
    pub fn new() -> Self {
        Self
    }

    /// Default location: `<app data>/backups/`.
    pub fn default_dir(app: &AppHandle) -> Result<PathBuf, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("backups");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(dir)
    }

    /// Keep the `keep` most recent automatic backups. User-chosen exports are
    /// named differently and are never swept up by this.
    pub fn prune(dir: &Path, keep: usize) -> Result<usize, String> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().starts_with("echo-auto-"))
            .collect();
        if entries.len() <= keep {
            return Ok(0);
        }
        entries.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
        let excess = entries.len() - keep;
        for entry in entries.into_iter().take(excess) {
            std::fs::remove_file(entry.path()).map_err(|e| e.to_string())?;
        }
        Ok(excess)
    }
}

impl Default for BackupManager {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
pub async fn export_backup(app: AppHandle) -> Result<String, String> {
    let pool = crate::pool()?;
    let dir = BackupManager::default_dir(&app)?;

    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let dest = dir.join(format!("echo-auto-{stamp}.echo.db"));

    // `VACUUM INTO` writes a consistent snapshot even with writes in flight.
    let escaped = dest.to_string_lossy().replace('\'', "''");
    sqlx::query(&format!("VACUUM INTO '{escaped}'"))
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    BackupManager::prune(&dir, 30)?;
    Ok(dest.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn import_backup(app: AppHandle, path: String) -> Result<(), String> {
    let src = PathBuf::from(path);
    if !src.exists() {
        return Err(format!("找不到备份文件：{}", src.display()));
    }

    // Ask the keychain for the password this database was created with, and
    // prove it opens the backup before touching the live file.
    let password = crate::onboarding::retrieve_password()?;
    crate::db::DbManager::open_at(src.clone(), &password)
        .await
        .map_err(|_| "备份密码不匹配，无法恢复".to_string())?;

    let dest = crate::state()
        .db_path
        .get()
        .cloned()
        .ok_or_else(|| "数据库尚未初始化".to_string())?;

    let pool = crate::pool()?;
    pool.close().await;

    std::fs::copy(&src, &dest).map_err(|e| e.to_string())?;

    // Reopen so the pool serves the restored data.
    let manager = crate::db::DbManager::open_at(dest, &password)
        .await
        .map_err(|e| e.to_string())?;
    let _ = crate::state().db.set(manager.pool().clone());

    crate::window::show_main_window(&app);
    Ok(())
}
