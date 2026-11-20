//! Standalone chat window: creation and instance target tracking.
//!
//! The chat commands themselves (`chat_send_message` and friends) live in the
//! backend chat module; this file only owns the window and the instance the
//! window is currently talking to.

use std::sync::{Mutex, MutexGuard, PoisonError};

use tauri::{
    AppHandle, Emitter as _, Manager as _, State, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use crate::error::{AppError, Result};

pub const CHAT_LABEL: &str = "chat";

const CHAT_WIDTH: f64 = 1280.0;
const CHAT_HEIGHT: f64 = 720.0;
const CHAT_MIN_WIDTH: f64 = 480.0;
const CHAT_MIN_HEIGHT: f64 = 560.0;

/// The instance the chat window talks to. `open_chat_window` stores it before
/// showing the window; an already-open window follows `chat-target-changed`.
#[derive(Default)]
pub struct ChatWindowState {
    target: Mutex<Option<String>>,
}

fn lock_target(state: &ChatWindowState) -> MutexGuard<'_, Option<String>> {
    state.target.lock().unwrap_or_else(PoisonError::into_inner)
}

#[tauri::command]
// Async so the handler runs off the main thread: window creation dispatches
// through the main event loop, which would otherwise be blocked inside this
// very handler — a deadlock that leaves a bare unconfigured white window.
pub async fn open_chat_window(
    app: AppHandle,
    state: State<'_, ChatWindowState>,
    instance_id: Option<String>,
) -> Result<()> {
    // Validate the target before touching the window so an invalid id never
    // focuses a window that still holds the previous conversation.
    let target = match instance_id {
        Some(id) if !id.trim().is_empty() => {
            let known = crate::config::load_manifest()
                .map(|manifest| manifest.instances.contains_key(&id))
                .unwrap_or(false);
            if !known {
                return Err(AppError::instance_not_found(&id));
            }
            Some(id)
        }
        _ => None,
    };

    // A targeted open updates the stored target; an untargeted open keeps the
    // current conversation and only shows and focuses the window.
    if let Some(id) = &target {
        *lock_target(&state) = Some(id.clone());
    }

    if let Some(window) = app.get_webview_window(CHAT_LABEL) {
        log::info!("Chat window already open; showing and focusing it");
        if let Err(error) = window.show() {
            log::warn!("Failed to show the chat window: {error}");
        }
        if let Err(error) = window.set_focus() {
            log::warn!("Failed to focus the chat window: {error}");
        }
    } else {
        log::info!("Creating the chat window");
        let window = build_chat_window(&app, target.as_deref()).map_err(|error| {
            AppError::other(format!("Failed to create the chat window: {error}"))
        })?;
        configure_chat_window(&window);
        log::info!("Chat window created");
    }

    // Emitted on targeted opens so an already-mounted window switches
    // instances even when the window itself was reused.
    if let Some(id) = target {
        if let Err(error) = app.emit_to(CHAT_LABEL, "chat-target-changed", id) {
            log::warn!("Failed to notify the chat window: {error}");
        }
    }
    Ok(())
}

#[tauri::command]
pub fn chat_window_target(state: State<'_, ChatWindowState>) -> String {
    // An empty string means "no target stored"; the webview then picks a
    // default instance on its own.
    lock_target(&state).clone().unwrap_or_default()
}

fn build_chat_window(app: &AppHandle, target: Option<&str>) -> tauri::Result<WebviewWindow> {
    // The hash is injected before any page script runs: the hash router then
    // boots directly on the chat route, with no label sniffing and no
    // post-mount redirect flash. The same script carries the initial target
    // instance so the view never races the target command or event.
    let initialization_script = format!(
        "window.location.hash = '#/chat'; window.__ASTRBOT_CHAT_TARGET__ = {};",
        serde_json::to_string(target.unwrap_or("")).unwrap_or_else(|_| "\"\"".to_string())
    );
    let builder = WebviewWindowBuilder::new(app, CHAT_LABEL, WebviewUrl::App("index.html".into()))
        .title("AstrBot Chat")
        .initialization_script(initialization_script)
        .inner_size(CHAT_WIDTH, CHAT_HEIGHT)
        .min_inner_size(CHAT_MIN_WIDTH, CHAT_MIN_HEIGHT)
        .center();

    // The island's toolbar doubles as the title bar (drag region + window
    // controls), so the native frame is dropped on Windows/Linux, matching
    // the main window. macOS keeps the overlay title bar style so the
    // traffic lights sit over the content like the main window.
    #[cfg(target_os = "macos")]
    let builder = builder
        .decorations(true)
        .title_bar_style(tauri::TitleBarStyle::Overlay);
    #[cfg(not(target_os = "macos"))]
    let builder = builder.decorations(false);

    let window = builder.build()?;

    // Deliberately no `create_overlay_titlebar()` here: decorum's overlay script
    // injects a full-width `data-tauri-drag-region` strip into the page when no
    // `[data-tauri-decorum-tb]` marker exists, which covered the top band and
    // starved the sidebar buttons of their click area. Dragging is handled by
    // the island's explicit `startDragging()` surfaces instead.

    Ok(window)
}

fn configure_chat_window(window: &WebviewWindow) {
    // Opaque theme-coloured background: no native material has to show
    // through, so the window can never fall back to a white void.
    crate::setup::apply_window_background(window, false);

    #[cfg(target_os = "windows")]
    {
        // The backdrop must come first: setting a system backdrop can reset
        // the corner preference, so rounding has to come after it.
        crate::setup::apply_windows_backdrop(window);
        crate::setup::apply_windows_corner_preference(window);
        crate::setup::apply_crisp_window_icon(window);
    }
}
