//! Backup manager: encrypted export/import

use crate::db::DbManager;
use std::path::PathBuf;
use tauri::AppHandle;

pub struct BackupManager {
    db: DbManager,
}

impl BackupManager {
    pub fn new(db: DbManager) -> Self { Self { db } }
    
    pub async fn export(&self, _password: &str, _dest: &PathBuf) -> Result<(), String> {
        Ok(())
    }
    
    pub async fn import(&self, _password: &str, _src: &PathBuf) -> Result<(), String> {
        Ok(())
    }
    
    pub async fn auto_backup(&self) -> Result<(), String> {
        Ok(())
    }
}