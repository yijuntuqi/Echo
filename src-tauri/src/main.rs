//! Echo - AI Electronic Pet
//! Main entry point

use tauri::Manager;
use tracing_subscriber::{fmt, EnvFilter};

mod window;
mod db;
mod chat;
mod embedding;
mod vector;
mod emotion;
mod evolution;
mod scheduler;
mod backup;
mod onboarding;

use window::WindowManager;
use db::DbManager;
use chat::ChatEngine;
use embedding::EmbeddingService;
use vector::VectorEngine;
use emotion::HybridAnalyzer;
use evolution::EvolutionEngine;
use scheduler::Scheduler;
use backup::BackupManager;
use onboarding::OnboardingManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("echo=debug".parse().unwrap()))
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_stronghold::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let handle = app.handle().clone();
            
            // 初始化窗口管理器（透明宠物窗口、托盘、热键）
            let window_mgr = tauri::async_runtime::block_on(async {
                WindowManager::init(&handle).await.expect("Failed to init window manager")
            });
            app.manage(window_mgr);
            
            // 初始化数据库（稍后在 onboarding 完成后用密码初始化）
            // 这里先不初始化，等待用户完成向导
            
            // 初始化其他服务（延迟到 onboarding 完成后）
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 窗口/托盘
            window::show_pet_overlay,
            window::hide_pet_overlay,
            window::set_click_through,
            window::get_pet_position,
            window::set_pet_position,
            
            // 对话
            chat::send_message,
            chat::get_history,
            chat::get_quota_remaining,
            
            // 记忆/检索
            vector::search_memory,
            vector::add_event,
            vector::get_timeline,
            
            // 进化
            evolution::get_evolution_state,
            evolution::force_evolve,
            
            // 设置/备份
            db::get_settings,
            db::update_settings,
            db::export_backup,
            db::import_backup,
            
            // 系统
            system::check_updates,
            system::get_system_info,
            
            // 引导
            onboarding::complete_onboarding,
            onboarding::get_onboarding_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

mod system {
    use tauri::command;
    
    #[command]
    pub async fn check_updates() -> Result<crate::chat::UpdateInfo, String> {
        // TODO: 实现更新检查
        Ok(crate::chat::UpdateInfo {
            available: false,
            version: env!("CARGO_PKG_VERSION").to_string(),
            notes: String::new(),
        })
    }
    
    #[command]
    pub async fn get_system_info() -> Result<SystemInfo, String> {
        Ok(SystemInfo {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
    
    #[derive(serde::Serialize)]
    pub struct SystemInfo {
        pub os: String,
        pub arch: String,
        pub version: String,
    }
}