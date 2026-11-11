use crate::error::{AppError, Result};
use tauri::AppHandle;

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
