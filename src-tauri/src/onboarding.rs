//! Onboarding wizard: first-run setup

use crate::db::{DbManager, Profile, Settings};
use chrono::NaiveDate;
use tauri::AppHandle;

pub struct OnboardingManager {
    db: DbManager,
}

impl OnboardingManager {
    pub fn new(db: DbManager) -> Self { Self { db } }
    
    pub async fn complete(&self, profile: Profile, settings: Settings) -> Result<(), String> {
        sqlx::query(
            "INSERT INTO profiles (id, nickname, birthday, install_date, settings_json) VALUES (1, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET nickname=?, birthday=?, settings_json=?, updated_at=datetime('now')"
        )
        .bind(&profile.nickname)
        .bind(&profile.birthday)
        .bind(&profile.install_date)
        .bind(&profile.settings_json)
        .bind(&profile.nickname)
        .bind(&profile.birthday)
        .bind(&profile.settings_json)
        .execute(&self.db.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }
    
    pub async fn is_completed(&self) -> Result<bool, String> {
        let row = sqlx::query("SELECT 1 FROM profiles WHERE id = 1")
            .fetch_optional(&self.db.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.is_some())
    }
}

#[tauri::command]
pub async fn complete_onboarding(
    profile: crate::db::Profile,
    settings: crate::db::Settings,
) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn get_onboarding_status() -> Result<bool, String> {
    Ok(false)
}