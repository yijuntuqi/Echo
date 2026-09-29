//! Window management: transparent pet overlay, system tray, global hotkey

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WebviewWindow};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use std::sync::Mutex;

pub struct WindowManager {
    pub pet_window: Option<WebviewWindow>,
    pub tray: Option<tauri::tray::TrayIcon>,
    pub click_through: Mutex<bool>,
}

impl WindowManager {
    pub async fn init(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        // 创建透明宠物窗口
        let pet_window = WebviewWindowBuilder::new(
            app,
            "pet",
            WebviewUrl::App("pet.html".into()),
        )
        .title("Echo Pet")
        .inner_size(200.0, 200.0)
        .min_inner_size(160.0, 160.0)
        .max_inner_size(300.0, 300.0)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()?;
        
        // Windows: 设置点击穿透
        #[cfg(target_os = "windows")]
        {
            use tauri::webview::WebviewExt;
            let hwnd = pet_window.hwnd()?;
            unsafe {
                use windows::Win32::UI::WindowsAndMessaging::{
                    SetWindowLongPtrW, GetWindowLongPtrW, GWL_EXSTYLE,
                    WS_EX_LAYERED, WS_EX_TRANSPARENT, WS_EX_TOPMOST
                };
                let ex_style = GetWindowLongPtrW(hwnd as _, GWL_EXSTYLE);
                SetWindowLongPtrW(
                    hwnd as _, 
                    GWL_EXSTYLE, 
                    ex_style | WS_EX_LAYERED as isize | WS_EX_TRANSPARENT as isize | WS_EX_TOPMOST as isize
                );
            }
        }
        
        // 创建系统托盘
        let show_item = MenuItemBuilder::new("显示宠物").id("show").build(app)?;
        let hide_item = MenuItemBuilder::new("隐藏宠物").id("hide").build(app)?;
        let settings_item = MenuItemBuilder::new("设置").id("settings").build(app)?;
        let quit_item = MenuItemBuilder::new("退出").id("quit").build(app)?;
        let separator = PredefinedMenuItem::separator(app)?;
        
        let menu = MenuBuilder::new(app)
            .items(&[&show_item, &hide_item, &separator, &settings_item, &separator, &quit_item])
            .build()?;
        
        let tray = TrayIconBuilder::new()
            .icon(app.default_window_icon()?.clone())
            .menu(&menu)
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("pet") {
                        let _ = window.set_visible(!window.is_visible().unwrap_or(false));
                    }
                }
            })
            .build(app)?;
        
        // 注册全局热键 Ctrl+Alt+E
        let shortcut = Shortcut::new(Some(tauri_plugin_global_shortcut::Modifiers::CONTROL | tauri_plugin_global_shortcut::Modifiers::ALT), tauri_plugin_global_shortcut::Key::KeyE);
        app.global_shortcut().register(shortcut)?;

        // 监听热键触发
        let app_handle = app.clone();
        app.listen_global("global-shortcut", move |_| {
            if let Some(window) = app_handle.get_webview_window("pet") {
                let _ = window.set_visible(!window.is_visible().unwrap_or(false));
            }
        });
        
        // 处理菜单事件
        let app_handle = app.clone();
        app.on_menu_event(move |app, event| {
            match event.id().as_ref() {
                "show" => {
                    if let Some(w) = app.get_webview_window("pet") { let _ = w.show(); }
                }
                "hide" => {
                    if let Some(w) = app.get_webview_window("pet") { let _ = w.hide(); }
                }
                "settings" => {
                    if let Some(w) = app.get_webview_window("main") { 
                        let _ = w.show(); 
                        let _ = w.set_focus(); 
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        });
        
        Ok(Self {
            pet_window: Some(pet_window),
            tray: Some(tray),
            click_through: Mutex::new(true),
        })
    }
}

#[tauri::command]
pub async fn show_pet_overlay(window_mgr: tauri::State<'_, WindowManager>) -> Result<(), String> {
    if let Some(w) = &window_mgr.pet_window {
        w.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn hide_pet_overlay(window_mgr: tauri::State<'_, WindowManager>) -> Result<(), String> {
    if let Some(w) = &window_mgr.pet_window {
        w.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn set_click_through(window_mgr: tauri::State<'_, WindowManager>, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(w) = &window_mgr.pet_window {
            let hwnd = w.hwnd().map_err(|e| e.to_string())?;
            unsafe {
                use windows::Win32::UI::WindowsAndMessaging::{
                    SetWindowLongPtrW, GetWindowLongPtrW, GWL_EXSTYLE,
                    WS_EX_LAYERED, WS_EX_TRANSPARENT
                };
                let ex_style = GetWindowLongPtrW(hwnd as _, GWL_EXSTYLE);
                let new_style = if enabled {
                    ex_style | WS_EX_TRANSPARENT as isize
                } else {
                    ex_style & !(WS_EX_TRANSPARENT as isize)
                };
                SetWindowLongPtrW(hwnd as _, GWL_EXSTYLE, new_style);
            }
        }
    }
    *window_mgr.click_through.lock().unwrap() = enabled;
    Ok(())
}

#[tauri::command]
pub async fn get_pet_position(window_mgr: tauri::State<'_, WindowManager>) -> Result<Option<(f64, f64)>, String> {
    if let Some(w) = &window_mgr.pet_window {
        let pos = w.outer_position().map_err(|e| e.to_string())?;
        Ok(Some((pos.x as f64, pos.y as f64)))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn set_pet_position(window_mgr: tauri::State<'_, WindowManager>, x: f64, y: f64) -> Result<(), String> {
    if let Some(w) = &window_mgr.pet_window {
        w.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x: x as i32, y: y as i32 }))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}