use crate::error::{AppError, Result};
use tauri::{AppHandle, Manager as _};

// === Window ===

#[tauri::command]
pub fn set_tray_panel_height(height: f64, app_handle: AppHandle) {
    crate::tray_panel::set_height(&app_handle, height);
}

#[tauri::command]
pub fn hide_tray_panel(app_handle: AppHandle) {
    crate::tray_panel::hide_panel(&app_handle);
}

#[tauri::command]
pub fn notify_tray_panel_ready(app_handle: AppHandle) {
    crate::tray_panel::mark_ready(&app_handle);
}

#[tauri::command]
pub fn quit_app(app_handle: AppHandle) {
    app_handle.exit(0);
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<()> {
    // Invoked by the tray panel, so it is an explicit user gesture: it must
    // override the autostart minimize-to-tray policy that hides the window on
    // a normal reveal.
    let outcome = crate::setup::show_main_window_from_user(&app)
        .map_err(|error| AppError::other(format!("Failed to show main window: {}", error)))?;

    match outcome {
        crate::setup::RevealOutcome::Shown => crate::setup::mark_frontend_ready(&app),
        crate::setup::RevealOutcome::HiddenByAutostart => {
            crate::setup::mark_startup_hidden(&app);
        }
    }

    Ok(())
}

#[tauri::command]
pub fn set_window_theme(dark: bool, app_handle: AppHandle) -> Result<()> {
    let window = app_handle
        .get_webview_window("main")
        .ok_or_else(|| AppError::other("Failed to set the window theme: main window not found"))?;
    apply_window_theme(&window, dark)
}

/// Mica takes its tint from the immersive dark-mode attribute, not the system
/// setting, so the window follows the app theme.
#[cfg(target_os = "windows")]
fn apply_window_theme(window: &tauri::WebviewWindow, dark: bool) -> Result<()> {
    use std::ffi::c_void;
    use windows::core::BOOL;
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};

    let hwnd = window.hwnd().map_err(|error| {
        AppError::other(format!("Failed to read the main window handle: {}", error))
    })?;
    let value: BOOL = dark.into();
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            std::ptr::from_ref(&value).cast::<c_void>(),
            std::mem::size_of::<BOOL>() as u32,
        )
    }
    .map_err(|error| AppError::other(format!("Failed to set the immersive dark mode: {}", error)))
}

#[cfg(target_os = "macos")]
fn apply_window_theme(window: &tauri::WebviewWindow, dark: bool) -> Result<()> {
    let theme = if dark {
        tauri::Theme::Dark
    } else {
        tauri::Theme::Light
    };
    window.set_theme(Some(theme)).map_err(|error| {
        AppError::other(format!("Failed to set the native window theme: {}", error))
    })
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn apply_window_theme(_window: &tauri::WebviewWindow, _dark: bool) -> Result<()> {
    Ok(())
}
