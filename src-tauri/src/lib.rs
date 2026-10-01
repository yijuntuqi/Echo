//! Echo — an AI electronic pet that lives on your desktop and phone.
//!
//! This library owns every stateful service. The two webviews are thin views:
//! the main window renders the dashboard and the chat panel, the pet window
//! renders the always-on-top character.

pub mod backup;
pub mod chat;
pub mod db;
pub mod embedding;
pub mod emotion;
pub mod evolution;
pub mod model;
pub mod onboarding;
pub mod scheduler;
pub mod vector;
pub mod window;

use std::path::PathBuf;
use std::sync::OnceLock;

use backup::BackupManager;
use chat::{ChatConfig, ChatEngine};
use db::{DbError, DbPool};
use embedding::EmbeddingService;
use emotion::HybridAnalyzer;
use evolution::EvolutionEngine;
use scheduler::Scheduler;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use vector::VectorEngine;
use window::WindowManager;

pub type SharedResult<T> = Result<T, String>;

/// Long-lived services.
///
/// The database opens lazily: onboarding generates its password, so before
/// that there is no pool to hand out. `OnceLock` makes that a one-way door —
/// once set, the pool lives for the rest of the process.
pub struct AppState {
    pub db: OnceLock<DbPool>,
    pub db_path: OnceLock<PathBuf>,
    pub chat: ChatEngine,
    pub embedding: OnceLock<EmbeddingService>,
    pub vectors: VectorEngine,
    pub emotion: HybridAnalyzer,
    pub evolution: EvolutionEngine,
    pub scheduler: Scheduler,
    pub backup: BackupManager,
}

impl AppState {
    fn new() -> Self {
        let chat = ChatEngine::new(ChatConfig::default());
        Self {
            db: OnceLock::new(),
            db_path: OnceLock::new(),
            emotion: HybridAnalyzer::new(&chat),
            evolution: EvolutionEngine::new(),
            chat,
            embedding: OnceLock::new(),
            vectors: VectorEngine::new(),
            scheduler: Scheduler::new(),
            backup: BackupManager::new(),
        }
    }
}

static STATE: OnceLock<AppState> = OnceLock::new();

/// Access the process-wide state. Safe before the database exists; callers that
/// need the pool check [`AppState::db`].
pub fn state() -> &'static AppState {
    STATE.get_or_init(AppState::new)
}

/// The open database pool, or a message explaining that setup has not run.
pub fn pool() -> SharedResult<&'static DbPool> {
    state()
        .db
        .get()
        .ok_or_else(|| "数据库尚未初始化，请先完成首次设置".to_string())
}

pub type PoolResult<T> = Result<T, DbError>;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load a gitignored local `.env` if present (secrets such as the shared
    // API key stay out of the repository). Line-by-line and tolerant: one
    // unparsable line (e.g. GBK-encoded Chinese written by cmd.exe) must not
    // poison the whole file — that silently cost us the shared key once.
    match dotenvy::dotenv_iter() {
        Ok(iter) => {
            let mut loaded = 0usize;
            for item in iter.flatten() {
                let (key, value) = item;
                std::env::set_var(key, value);
                loaded += 1;
            }
            tracing::debug!(vars = loaded, "loaded local .env");
        }
        Err(e) => tracing::debug!(error = %e, "no .env loaded"),
    }

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false).with_writer(std::io::stderr))
        .with(
            EnvFilter::try_from_env("ECHO_LOG")
                .unwrap_or_else(|_| EnvFilter::new("echo=info,warn")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .setup(|app| {
            use tauri::Manager;

            let handle = app.handle().clone();

            // Touch the state so every later command finds it initialised.
            state();

            // Point the embedding service at its cache directory so the
            // first chat message can trigger the model download.
            model::init(&handle);

            // The pet overlay, tray, and hotkey come up first so the app stays
            // reachable even before the database exists.
            let wm = WindowManager::init(&handle)?;
            app.manage(wm);

            // On a returning launch the password is already in the OS keychain,
            // so the database reopens without prompting. A first run leaves it
            // closed and the main window hidden, waiting for onboarding.
            let boot = handle.clone();
            tauri::async_runtime::spawn(async move {
                match onboarding::open_existing(&boot).await {
                    Ok(()) => {
                        window::show_main_window(&boot);
                        // Keep greetings on their own schedule rather than
                        // firing the moment the app restarts.
                        if let Err(e) = state().scheduler.start(&boot).await {
                            tracing::warn!("scheduler did not start: {e}");
                        }
                    }
                    Err(_) => {
                        // No stored password, or it no longer opens the file:
                        // the frontend routes to onboarding.
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // windows
            window::show_pet_overlay,
            window::hide_pet_overlay,
            window::set_click_through,
            window::get_click_through,
            window::get_pet_position,
            window::set_pet_position,
            window::place_pet_window,
            window::open_chat,
            window::set_pet_window_shape,
            // chat
            chat::send_message,
            chat::get_history,
            chat::get_quota_remaining,
            // memory
            vector::search_memory,
            vector::add_event,
            vector::get_timeline,
            // evolution
            evolution::get_evolution_state,
            evolution::force_evolve,
            // settings
            db::get_settings,
            db::update_settings,
            backup::export_backup,
            backup::import_backup,
            // system
            system::check_updates,
            system::get_system_info,
            // onboarding
            onboarding::complete_onboarding,
            onboarding::get_onboarding_status,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start Echo");
}

pub mod system {
    use serde::Serialize;
    use tauri::command;

    #[derive(Serialize)]
    pub struct SystemInfo {
        pub os: String,
        pub arch: String,
        pub version: String,
    }

    #[command]
    pub fn check_updates() -> crate::chat::UpdateInfo {
        // Update checking is wired up in the release pipeline; until a signed
        // build exists there is nothing newer to offer.
        crate::chat::UpdateInfo {
            available: false,
            version: env!("CARGO_PKG_VERSION").to_string(),
            notes: String::new(),
        }
    }

    #[command]
    pub fn get_system_info() -> SystemInfo {
        SystemInfo {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}
