//! First-run setup.
//!
//! Onboarding collects a nickname and birthday, picks a database password, and
//! writes the initial profile. Everything after this point assumes the database
//! is open, so the main window stays hidden until it finishes.

use serde::Deserialize;
use tauri::AppHandle;

use crate::db::{DbManager, Profile, Settings};

/// How the database password is kept. The user never types it: onboarding
/// generates a random one and hands it to the OS credential vault, so the
/// database is encrypted at rest without adding a prompt anyone has to remember.
const KEYCHAIN_SERVICE: &str = "com.yijuntuqi.echo";
const KEYCHAIN_USER: &str = "echo.db-password";

#[derive(Debug, Deserialize)]
pub struct OnboardingRequest {
    pub profile: Profile,
    pub settings: Settings,
}

async fn profile_exists() -> Result<bool, String> {
    let Some(pool) = crate::state().db.get() else {
        // No database yet means a first run, which is exactly "not onboarded".
        return Ok(false);
    };
    sqlx::query("SELECT 1 FROM profiles WHERE id = 1")
        .fetch_optional(pool)
        .await
        .map(|row| row.is_some())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_onboarding_status() -> Result<bool, String> {
    profile_exists().await
}

#[tauri::command]
pub async fn complete_onboarding(
    app: AppHandle,
    profile: Profile,
    settings: Settings,
) -> Result<(), String> {
    if profile.nickname.trim().is_empty() {
        return Err("nickname is required".into());
    }
    if profile.birthday.len() != 10 {
        return Err("birthday must be YYYY-MM-DD".into());
    }
    if crate::db::parse_date(&profile.birthday).is_none() {
        return Err("birthday is not a valid date".into());
    }

    // Open the database for the first time. The password is random and stored
    // in the OS keychain; a fresh install has nothing to decrypt yet.
    if crate::state().db.get().is_none() {
        let generated = generate_password();
        let manager = DbManager::open_at(app_data_path(&app)?, &generated)
            .await
            .map_err(|e| e.to_string())?;
        let _ = crate::state().db.set(manager.pool().clone());
        let _ = crate::state().db_path.set(manager.path().to_path_buf());
        // Wire the capability probe's result into the vector engine. This
        // flag gates every semantic-search branch, and nothing ever set it —
        // search silently ran keyword-only forever.
        crate::state()
            .vectors
            .set_vec_available(manager.vec_available());
        store_password(&generated)?;
    }

    let pool = crate::state()
        .db
        .get()
        .ok_or_else(|| "database not initialised".to_string())?;

    let settings_json =
        serde_json::to_string(&settings).map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO profiles (id, nickname, birthday, install_date, settings_json)
         VALUES (1, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
           nickname = excluded.nickname,
           birthday = excluded.birthday,
           settings_json = excluded.settings_json,
           updated_at = datetime('now')",
    )
    .bind(profile.nickname.trim())
    .bind(&profile.birthday)
    .bind(&profile.install_date)
    .bind(&settings_json)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Background services need the pool, so start them only now.
    let _ = crate::state().scheduler.start(&app).await;

    // The key the user just typed (or left blank) becomes live immediately.
    apply_chat_key_from_db(pool).await;
    refresh_nickname_cache().await;
    sync_autostart(&app).await;

    crate::window::show_main_window(&app);
    Ok(())
}

fn app_data_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    use tauri::Manager;
    app.path()
        .app_data_dir()
        .map_err(|e| e.to_string())
        .map(|d| {
            std::fs::create_dir_all(&d).ok();
            d.join("echo.db")
        })
}

/// 32 hex characters of OS randomness — enough entropy that guessing the
/// database password is hopeless even if the keychain is later compromised.
fn generate_password() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn store_password(password: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER)
        .map_err(|e| format!("系统密码库不可用: {e}"))?;
    entry
        .set_password(password)
        .map_err(|e| format!("无法写入系统密码库: {e}"))
}

pub fn retrieve_password() -> Result<String, String> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER)
        .map_err(|e| format!("系统密码库不可用: {e}"))?;
    entry
        .get_password()
        .map_err(|e| format!("无法读取系统密码库: {e}"))
}

/// Forget the stored password. Losing it means losing the database: there is no
/// recovery path, so this is only called when the user explicitly resets.
pub fn forget_password() {
    if let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER) {
        let _ = entry.delete_credential();
    }
}

/// Reopen the database at boot using the stored password.
pub async fn open_existing(app: &AppHandle) -> Result<(), String> {
    let password = retrieve_password()?;
    let manager = DbManager::open_at(app_data_path(app)?, &password)
        .await
        .map_err(|e| e.to_string())?;
    let _ = crate::state().db.set(manager.pool().clone());
    let _ = crate::state().db_path.set(manager.path().to_path_buf());
    crate::state()
        .vectors
        .set_vec_available(manager.vec_available());
    apply_chat_key_from_db(manager.pool()).await;
    refresh_nickname_cache().await;
    sync_autostart(app).await;
    Ok(())
}

/// Push the stored `user_api_key` / `user_base_url` / `model_preference` (if
/// any) into the chat engine. One function owns "settings blob -> engine" so
/// startup, onboarding, and settings edits all behave the same.
///
/// Best-effort: a malformed settings blob must not block startup.
pub async fn apply_chat_key_from_db(pool: &crate::db::DbPool) {
    let profile: Option<(String, Option<String>, String)> =
        sqlx::query("SELECT settings_json FROM profiles WHERE id = 1")
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .and_then(|row| {
                use sqlx::Row;
                let json: String = row.get("settings_json");
                serde_json::from_str::<Settings>(&json).ok().map(|s| {
                    (
                        s.user_api_key.unwrap_or_default(),
                        s.user_base_url,
                        s.model_preference,
                    )
                })
            });
    let (key, base_url, preference) = match profile {
        Some((k, url, pref)) => (Some(k), url, pref),
        None => (None, None, "auto".to_string()),
    };
    crate::chat::apply_key(&crate::state().chat, key.as_deref(), base_url.as_deref()).await;
    // The preference rides the same push; apply_key leaves the model fields
    // alone, so this is the only writer of the daily model.
    crate::state().chat.set_model_preference(&preference);
}

/// Mirror the stored `auto_start` setting into the OS autostart entry. The
/// setting used to be saved but never applied — the plugin was initialised
/// with nothing ever calling enable/disable. Best-effort: a registry failure
/// must not block startup or saving.
pub async fn sync_autostart(app: &AppHandle) {
    let enabled = match crate::pool() {
        Ok(pool) => {
            let row: Option<(String,)> =
                sqlx::query_as("SELECT settings_json FROM profiles WHERE id = 1")
                    .fetch_optional(pool)
                    .await
                    .ok()
                    .flatten();
            match row.and_then(|(json,)| serde_json::from_str::<Settings>(&json).ok()) {
                Some(s) => s.auto_start,
                None => return,
            }
        }
        // No database yet (first run): nothing to mirror.
        Err(_) => return,
    };
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    let current = autolaunch.is_enabled().unwrap_or(false);
    if current == enabled {
        return;
    }
    let outcome = if enabled {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    if let Err(e) = outcome {
        tracing::warn!(enabled, error = %e, "autostart sync failed");
    }
}

/// Delete the encrypted database and its keychain password, then restart the
/// app. The escape hatch for a file that can no longer be opened (keychain
/// entry lost or replaced, file damaged): without it the onboarding loop
/// dead-ends on "wrong password" with no way forward. Destructive by
/// definition — the frontend must confirm first.
///
/// The restart is structural, not cosmetic: the pool lives in a `OnceLock` (a
/// one-way door), so only a fresh process gets a fresh door. After the
/// restart there is no stored password, `open_existing` fails, and the
/// frontend routes to a clean onboarding.
#[tauri::command]
pub async fn reset_database(app: AppHandle) -> Result<(), String> {
    use tauri::Manager;

    // Release SQLCipher's file handles first, or Windows refuses the delete.
    if let Some(pool) = crate::state().db.get() {
        pool.close().await;
    }
    forget_password();
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    for name in ["echo.db", "echo.db-wal", "echo.db-shm"] {
        let path = dir.join(name);
        match std::fs::remove_file(&path) {
            Ok(()) => tracing::warn!(path = %path.display(), "reset removed database file"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("无法删除 {}: {e}", path.display())),
        }
    }
    tracing::warn!("database reset by user request; restarting");
    app.restart()
}

/// Cache `profiles.nickname` for prompt building. Blank means "none": the
/// system prompt must never render an empty 「」. Called whenever the database
/// opens and whenever the nickname changes, so reads never touch the table.
pub async fn refresh_nickname_cache() {
    let Ok(pool) = crate::pool() else {
        return;
    };
    let nickname: Option<String> = sqlx::query("SELECT nickname FROM profiles WHERE id = 1")
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .and_then(|row| {
            use sqlx::Row;
            let raw: String = row.get("nickname");
            let trimmed = raw.trim().to_string();
            (!trimmed.is_empty()).then_some(trimmed)
        });
    if let Ok(mut guard) = crate::state().nickname.write() {
        *guard = nickname;
    }
}
