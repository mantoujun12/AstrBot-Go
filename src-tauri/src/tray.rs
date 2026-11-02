use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager as _};

use crate::config::load_config;
use crate::i18n::Locale;

const TRAY_ID: &str = "main-tray";

pub struct TrayMenuState {
    show: MenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
}

fn current_locale() -> Locale {
    load_config()
        .map(|config| Locale::resolve(config.language_preference))
        .unwrap_or_else(|_| Locale::detect())
}

#[allow(clippy::expect_used)]
pub fn build_tray(app: &tauri::App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let locale = current_locale();
    let show = MenuItem::with_id(app, "show", locale.tray_show(), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", locale.tray_quit(), true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().expect("no default icon").clone())
        .tooltip("AstrBot Launcher")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click { button, .. } = event {
                if button == tauri::tray::MouseButton::Left {
                    if let Some(w) = tray.app_handle().get_webview_window("main") {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    app.manage(TrayMenuState { show, quit });
    Ok(())
}

/// Updates the tray labels in place. This is safe on Linux, where replacing
/// the whole menu is not supported by every desktop environment.
pub fn refresh_tray_menu(app: &AppHandle) {
    let Some(state) = app.try_state::<TrayMenuState>() else {
        return;
    };
    let locale = current_locale();
    if let Err(error) = state.show.set_text(locale.tray_show()) {
        log::warn!("Failed to update tray show label: {}", error);
    }
    if let Err(error) = state.quit.set_text(locale.tray_quit()) {
        log::warn!("Failed to update tray quit label: {}", error);
    }
}
