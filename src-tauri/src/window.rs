//! Window management: the transparent pet overlay, the system tray, and the
//! global hotkey. The pet lives in its own always-on-top window; the main
//! window hosts the dashboard and the chat panel.

use std::sync::Mutex;

use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

pub struct WindowManager {
    pub pet_window: Option<WebviewWindow>,
    pub tray: Option<tauri::tray::TrayIcon>,
    click_through: Mutex<bool>,
}

/// Label of the always-on-top pet window, as declared in `tauri.conf.json`.
const PET_WINDOW: &str = "pet";
/// Label of the main window.
const MAIN_WINDOW: &str = "main";

impl WindowManager {
    pub fn init(app: &AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let pet_window = build_pet_window(app)?;
        let tray = build_tray(app)?;
        register_hotkey(app)?;

        // A tray app must not lose its windows to the X button: closing the
        // main window hides it instead, so the tray's "设置" entry can still
        // surface it later (recreating a destroyed webview is not possible).
        if let Some(main) = app.get_webview_window(MAIN_WINDOW) {
            let window_for_close = main.clone();
            main.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window_for_close.hide();
                }
            });
        }

        Ok(Self {
            pet_window: Some(pet_window),
            tray: Some(tray),
            // The pet starts interactive; click-through is opt-in via the
            // `set_click_through` command.
            click_through: Mutex::new(false),
        })
    }

    /// Toggle whether mouse input passes through the pet to whatever is beneath.
    pub fn set_click_through(&self, enabled: bool) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        if let Some(win) = &self.pet_window {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::{
                GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TRANSPARENT,
            };
            let Ok(hwnd) = win.hwnd() else {
                return Err("failed to read pet window handle".into());
            };
            let hwnd = HWND(hwnd.0 as *mut core::ffi::c_void);
            // SAFETY: called on the UI thread with a live HWND owned by this window.
            unsafe {
                let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                let updated = if enabled {
                    current | WS_EX_TRANSPARENT.0 as isize
                } else {
                    current & !(WS_EX_TRANSPARENT.0 as isize)
                };
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, updated);
            }
        }

        *self
            .click_through
            .lock()
            .map_err(|_| "click-through lock poisoned".to_string())? = enabled;
        Ok(())
    }

    pub fn click_through(&self) -> bool {
        self.click_through.lock().map(|v| *v).unwrap_or(true)
    }
}

fn build_pet_window(app: &AppHandle) -> Result<WebviewWindow, Box<dyn std::error::Error>> {
    // `tauri.conf.json` already declares this window; reuse it if it exists.
    if let Some(existing) = app.get_webview_window(PET_WINDOW) {
        return Ok(existing);
    }

    let win = WebviewWindowBuilder::new(app, PET_WINDOW, WebviewUrl::App("pet.html".into()))
        .title("Echo Pet")
        .inner_size(200.0, 200.0)
        .min_inner_size(160.0, 160.0)
        // No max: the window grows to ~592x560 while the chat panel is
        // docked beside the pet, then shrinks back programmatically.
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .visible(false)
        .build()?;

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, GWL_STYLE,
            SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_CAPTION, WS_EX_LAYERED,
            WS_EX_TOPMOST, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
        };
        if let Ok(raw) = win.hwnd() {
            let hwnd = HWND(raw.0 as *mut core::ffi::c_void);
            // SAFETY: called on the UI thread with a live HWND owned by this window.
            // WS_EX_LAYERED is required for per-pixel alpha on a transparent window.
            // WS_EX_TRANSPARENT is deliberately NOT set here: it makes every
            // mouse event fall through the pet, killing drag and click. The
            // initial style is interactive; `set_click_through` toggles the
            // flag at runtime when the user asks for pass-through.
            unsafe {
                let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(
                    hwnd,
                    GWL_EXSTYLE,
                    current | WS_EX_LAYERED.0 as isize | WS_EX_TOPMOST.0 as isize,
                );
                // Strip every frame-bearing style bit: some DWM combinations
                // paint a caption (title bar) on a borderless window once it
                // carries a hit-test region. With the style bits gone there is
                // no frame for DWM to resurrect. SWP_FRAMECHANGED makes the
                // style change take effect immediately.
                let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
                let frame_bits = (WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX
                    | WS_SYSMENU)
                    .0 as isize;
                SetWindowLongPtrW(hwnd, GWL_STYLE, style & !frame_bits);
                let _ = SetWindowPos(
                    hwnd,
                    HWND::default(),
                    0,
                    0,
                    0,
                    0,
                    SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
                );
            }
        }
    }

    Ok(win)
}

fn build_tray(app: &AppHandle) -> Result<tauri::tray::TrayIcon, Box<dyn std::error::Error>> {
    let show = MenuItemBuilder::with_id("show", "显示宠物").build(app)?;
    let hide = MenuItemBuilder::with_id("hide", "隐藏宠物").build(app)?;
    let main_window = MenuItemBuilder::with_id("main_window", "主窗口").build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "设置").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&show, &hide, &sep1, &main_window, &settings, &sep2, &quit])
        .build()?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("missing default window icon")?;

    let tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("Echo")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(|tray, event| {
            // Left click toggles the pet; the menu is reserved for right click.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                toggle_pet_window(app);
            }
        })
        .build(app)?;

    Ok(tray)
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "show" => show_pet_window(app),
        "hide" => hide_pet_window(app),
        // Both surface the main window, but land on different pages. For
        // 设置 the route is set directly in the webview (eval on the hash)
        // rather than via an event round-trip: the router's hashchange
        // listener is registered at module init, so it cannot miss. The
        // `nav:goto` event stays as a redundant second path.
        "main_window" => {
            show_main_window(app);
            let _ = app.emit("nav:goto", "/");
        }
        "settings" => {
            show_main_window(app);
            if let Some(win) = app.get_webview_window(MAIN_WINDOW) {
                let _ = win.eval("window.location.hash = '#/settings';");
            }
            let _ = app.emit("nav:goto", "/settings");
        }
        // "quit" is distinct from "hide": it tears the process down.
        "quit" => app.exit(0),
        _ => {}
    }
}

fn register_hotkey(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyE);
    app.global_shortcut()
        .on_shortcut(shortcut, |app, _sc, _event| {
            // Surface the main window: that is what users expect a global
            // hotkey to do. The pet itself is reachable via tray and click.
            show_main_window(app);
        })
        .map_err(|e| format!("could not register Ctrl+Alt+E: {e}"))?;
    Ok(())
}

fn toggle_pet_window(app: &AppHandle) {
    let Some(win) = app.get_webview_window(PET_WINDOW) else {
        tracing::warn!("tray toggle: pet window not found");
        return;
    };
    match win.is_visible() {
        Ok(true) => {
            tracing::info!("tray toggle: hiding pet window");
            let _ = win.hide();
        }
        state => {
            tracing::info!(?state, "tray toggle: showing pet window");
            let _ = win.show();
            let _ = win.set_always_on_top(true);
        }
    }
}

fn show_pet_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(PET_WINDOW) {
        let _ = win.show();
        let _ = win.set_always_on_top(true);
    }
}

fn hide_pet_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(PET_WINDOW) {
        let _ = win.hide();
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(MAIN_WINDOW) {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn show_pet_overlay(app: AppHandle) -> Result<(), String> {
    show_pet_window(&app);
    Ok(())
}

#[tauri::command]
pub fn hide_pet_overlay(app: AppHandle) -> Result<(), String> {
    hide_pet_window(&app);
    Ok(())
}

#[tauri::command]
pub fn set_click_through(
    manager: tauri::State<'_, WindowManager>,
    enabled: bool,
) -> Result<(), String> {
    manager.set_click_through(enabled)
}

#[tauri::command]
pub fn get_click_through(manager: tauri::State<'_, WindowManager>) -> bool {
    manager.click_through()
}

#[tauri::command]
pub fn get_pet_position(app: AppHandle) -> Option<(f64, f64)> {
    let win = app.get_webview_window(PET_WINDOW)?;
    let pos = win.outer_position().ok()?;
    Some((pos.x as f64, pos.y as f64))
}

#[tauri::command]
pub fn set_pet_position(app: AppHandle, x: f64, y: f64) -> Result<(), String> {
    let win = app
        .get_webview_window(PET_WINDOW)
        .ok_or_else(|| "pet window not found".to_string())?;
    win.set_position(PhysicalPosition::new(x as i32, y as i32))
        .map_err(|e| e.to_string())
}

/// Resize and move the pet window in one atomic OS call. Issuing size and
/// position from the frontend takes two round trips; between them the window
/// can paint with the pet drawn outside the (already shrunk) bounds and the
/// pet visibly blinks. One `SetWindowPos`, one frame.
#[cfg(target_os = "windows")]
#[tauri::command]
pub fn place_pet_window(app: AppHandle, x: i32, y: i32, width: i32, height: i32) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER};

    let win = app
        .get_webview_window(PET_WINDOW)
        .ok_or_else(|| "pet window not found".to_string())?;
    let Ok(raw) = win.hwnd() else {
        return Err("failed to read pet window handle".into());
    };
    let hwnd = HWND(raw.0 as *mut core::ffi::c_void);
    // SAFETY: live HWND owned by this window; flags preserve z-order/focus.
    unsafe {
        SetWindowPos(
            hwnd,
            HWND::default(),
            x,
            y,
            width,
            height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Non-Windows fallback: two calls, same net effect.
#[cfg(not(target_os = "windows"))]
#[tauri::command]
pub fn place_pet_window(app: AppHandle, x: i32, y: i32, width: i32, height: i32) -> Result<(), String> {
    let win = app
        .get_webview_window(PET_WINDOW)
        .ok_or_else(|| "pet window not found".to_string())?;
    win.set_size(PhysicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    win.set_position(PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())
}

/// Open the chat panel next to the pet. The panel lives in the pet window
/// (so it can follow the pet around); this only raises the `chat:open`
/// event, which the pet window answers by expanding and showing the panel.
#[tauri::command]
pub fn open_chat(app: AppHandle) -> Result<(), String> {
    app.emit("chat:open", ())
        .map_err(|e| e.to_string())
}

/// Hit-test region for the pet window, in physical px relative to the window.
#[derive(serde::Deserialize)]
pub struct WindowRegion {
    /// `[x, y, width, height]` rectangles; mouse input outside the union of
    /// these falls through to whatever is beneath the window.
    #[serde(default)]
    pub rects: Vec<[i32; 4]>,
    /// Optional `[x, y, width, height]` ellipse (the pet's egg silhouette).
    /// The collapsed window uses it so the square's transparent corners let
    /// clicks through to windows underneath.
    #[serde(default)]
    pub ellipse: Option<[i32; 4]>,
}

/// Restrict where the (transparent) pet window accepts mouse input.
///
/// While the chat panel is docked the window grows to ~592x560, almost all of
/// it empty transparency — without a region that invisible rectangle blocks
/// clicks aimed at windows underneath (e.g. the main window's navigation).
/// Even collapsed, the 200x200 square's corners would block, so the frontend
/// always keeps a region set: rects for panel + pet, ellipse for the pet.
#[tauri::command]
pub fn set_pet_window_shape(app: AppHandle, region: Option<WindowRegion>) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::Graphics::Gdi::{
            CombineRgn, CreateEllipticRgn, CreateRectRgn, DeleteObject, HGDIOBJ, HRGN, RGN_OR,
            SetWindowRgn,
        };

        let Some(win) = app.get_webview_window(PET_WINDOW) else {
            return Err("pet window not found".into());
        };
        let Ok(hwnd) = win.hwnd() else {
            return Err("failed to read pet window handle".into());
        };
        let hwnd = HWND(hwnd.0 as *mut core::ffi::c_void);

        let Some(region) = region else {
            // A null region removes the clip: the whole (small) window is hit-testable.
            // SAFETY: live HWND owned by this window.
            unsafe {
                SetWindowRgn(hwnd, HRGN::default(), true);
            }
            return Ok(());
        };

        let mut combined = HRGN::default();
        for [x, y, w, h] in region.rects {
            if w <= 0 || h <= 0 {
                continue;
            }
            // SAFETY: plain GDI object creation; freed below or handed to the OS.
            let r = unsafe { CreateRectRgn(x, y, x + w, y + h) };
            if combined.is_invalid() {
                combined = r;
            } else {
                unsafe {
                    CombineRgn(combined, combined, r, RGN_OR);
                    let _ = DeleteObject(HGDIOBJ(r.0));
                }
            }
        }
        if let Some([x, y, w, h]) = region.ellipse.filter(|[_, _, w, h]| *w > 0 && *h > 0) {
            // SAFETY: plain GDI object creation; freed below or handed to the OS.
            let r = unsafe { CreateEllipticRgn(x, y, x + w, y + h) };
            if combined.is_invalid() {
                combined = r;
            } else {
                unsafe {
                    CombineRgn(combined, combined, r, RGN_OR);
                    let _ = DeleteObject(HGDIOBJ(r.0));
                }
            }
        }
        if combined.is_invalid() {
            return Err("failed to create window region".into());
        }
        // SAFETY: the OS takes ownership of `combined` once set — never freed here.
        unsafe {
            SetWindowRgn(hwnd, combined, true);
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, region);
    }
    Ok(())
}
