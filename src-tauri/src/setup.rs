use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::Emitter as _;
use tauri::Manager as _;
#[cfg(target_os = "linux")]
use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt as _, WebViewExt as _};

use crate::commands::{self, AppState};
use crate::config::{load_config, load_manifest, with_manifest_mut, ThemePreference};
use crate::tray;
use crate::utils::log_bus as log_channel;

const WINDOW_SHOW_FALLBACK_DELAY: Duration = Duration::from_secs(5);
const WINDOW_SHOW_RETRY_DELAY: Duration = Duration::from_secs(1);
const WINDOW_SHOW_MAX_ATTEMPTS: usize = 3;

const LIGHT_WINDOW_BACKGROUND: tauri::window::Color = tauri::window::Color(255, 255, 255, 255);
const DARK_WINDOW_BACKGROUND: tauri::window::Color = tauri::window::Color(36, 36, 36, 255);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RevealOutcome {
    Shown,
    HiddenByAutostart,
}

pub(crate) struct StartupWindowState {
    frontend_ready: AtomicBool,
    startup_handled: AtomicBool,
    show_pending: AtomicBool,
}

impl StartupWindowState {
    pub(crate) fn new() -> Self {
        Self {
            frontend_ready: AtomicBool::new(false),
            startup_handled: AtomicBool::new(false),
            show_pending: AtomicBool::new(false),
        }
    }

    fn mark_frontend_ready(&self) {
        self.frontend_ready.store(true, Ordering::Release);
        self.startup_handled.store(true, Ordering::Release);
    }

    fn mark_startup_hidden(&self) {
        self.startup_handled.store(true, Ordering::Release);
    }

    fn is_startup_handled(&self) -> bool {
        self.startup_handled.load(Ordering::Acquire)
    }

    fn can_reveal_now(&self) -> bool {
        self.frontend_ready.load(Ordering::Acquire) || self.startup_handled.load(Ordering::Acquire)
    }

    fn request_show(&self) {
        self.show_pending.store(true, Ordering::Release);
    }

    fn is_show_pending(&self) -> bool {
        self.show_pending.load(Ordering::Acquire)
    }

    fn clear_show_pending(&self) {
        self.show_pending.store(false, Ordering::Release);
    }

    fn prepare_recreated_window(&self) {
        self.frontend_ready.store(false, Ordering::Release);
        self.startup_handled.store(false, Ordering::Release);
        self.request_show();
    }
}

pub fn create_main_window(app: &tauri::AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    let window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .cloned()
        .unwrap_or_default();
    let preference = load_config()
        .map(|config| config.theme_preference)
        .unwrap_or_default();
    let initialization_script = format!(
        "window.__ASTRBOT_THEME_PREFERENCE__ = \"{}\";",
        theme_preference_value(preference)
    );

    // The config disables decorations for the custom title bar on every
    // platform. macOS still needs the native title bar so its traffic lights
    // stay reachable, and `titleBarStyle: Overlay` keeps the content below it.
    #[cfg(target_os = "macos")]
    let builder = tauri::WebviewWindowBuilder::from_config(app, &window_config)?.decorations(true);
    #[cfg(not(target_os = "macos"))]
    let builder = tauri::WebviewWindowBuilder::from_config(app, &window_config)?;

    let window = builder
        .initialization_script(initialization_script)
        .build()?;
    configure_main_window(&window);
    Ok(window)
}

fn theme_preference_value(preference: ThemePreference) -> &'static str {
    match preference {
        ThemePreference::System => "system",
        ThemePreference::Light => "light",
        ThemePreference::Dark => "dark",
    }
}

pub fn on_setup(app: &tauri::App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    hide_window_for_autostart(app);
    spawn_window_show_fallback(app.handle().clone());

    let state: tauri::State<'_, AppState> = app.state();
    state.process_manager.start_monitor(app.handle().clone());

    spawn_event_forwarder(app);
    spawn_log_forwarder(app);
    spawn_resource_alert_forwarder(app);
    tray::build_tray(app)?;
    restore_instances(app);

    Ok(())
}

pub fn configure_main_window(window: &tauri::WebviewWindow) {
    apply_initial_background(window);

    #[cfg(target_os = "linux")]
    let _ = window.with_webview(|webview| {
        if let Some(settings) = webview.inner().settings() {
            settings.set_hardware_acceleration_policy(HardwareAccelerationPolicy::Never);
        }
    });

    #[cfg(not(target_os = "macos"))]
    let _ = window.set_decorations(false);

    let _ = window.set_min_size(Some(tauri::LogicalSize::new(
        crate::MIN_WINDOW_WIDTH as f64,
        crate::MIN_WINDOW_HEIGHT as f64,
    )));
}

fn apply_initial_background(window: &tauri::WebviewWindow) {
    let preference = load_config()
        .map(|config| config.theme_preference)
        .unwrap_or_default();
    let dark = match preference {
        ThemePreference::Dark => true,
        ThemePreference::Light => false,
        ThemePreference::System => window
            .theme()
            .map(|theme| theme == tauri::Theme::Dark)
            .unwrap_or(false),
    };
    let color = if dark {
        DARK_WINDOW_BACKGROUND
    } else {
        LIGHT_WINDOW_BACKGROUND
    };

    if let Err(error) = window.set_background_color(Some(color)) {
        log::warn!(
            "Failed to set the initial window background color: {}",
            error
        );
    }
}

pub(crate) fn should_start_hidden() -> bool {
    let launched_from_autostart = std::env::args_os().any(|arg| arg == "--autostart");
    launched_from_autostart
        && load_config()
            .map(|config| config.autostart_minimize_to_tray)
            .unwrap_or(false)
}

fn hide_window_for_autostart(app: &tauri::App) {
    if !should_start_hidden() {
        return;
    }

    if let Some(main_window) = app.get_webview_window("main") {
        let _ = main_window.hide();
    }
}

fn main_window_unavailable_error() -> tauri::Error {
    tauri::Error::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "main window is unavailable",
    ))
}

fn show_main_window_now(app: &tauri::AppHandle) -> tauri::Result<()> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(main_window_unavailable_error)?;

    let _ = window.unminimize();
    window.show()?;
    if let Err(error) = window.set_focus() {
        log::warn!("Failed to focus the main window: {}", error);
    }

    Ok(())
}

/// Reveals the main window once the frontend has signaled it is ready.
///
/// A pending show request from the tray, single-instance activation, or a
/// recreated window overrides the autostart minimize-to-tray policy.
pub(crate) fn reveal_main_window(app: &tauri::AppHandle) -> tauri::Result<RevealOutcome> {
    let state = app.state::<StartupWindowState>();
    if should_start_hidden() && !state.is_show_pending() {
        return Ok(RevealOutcome::HiddenByAutostart);
    }

    show_main_window_now(app)?;
    state.clear_show_pending();
    Ok(RevealOutcome::Shown)
}

/// Handles a user-initiated activation such as the tray icon or a second launch.
pub(crate) fn request_show_main_window(app: &tauri::AppHandle) {
    let state = app.state::<StartupWindowState>();
    state.request_show();

    if !state.can_reveal_now() {
        return;
    }

    match show_main_window_now(app) {
        Ok(()) => state.clear_show_pending(),
        Err(error) => log::warn!("Failed to show the main window on request: {}", error),
    }
}

pub(crate) fn mark_frontend_ready(app: &tauri::AppHandle) {
    app.state::<StartupWindowState>().mark_frontend_ready();
}

pub(crate) fn mark_startup_hidden(app: &tauri::AppHandle) {
    app.state::<StartupWindowState>().mark_startup_hidden();
}

pub(crate) fn prepare_recreated_window(app: &tauri::AppHandle) {
    app.state::<StartupWindowState>().prepare_recreated_window();
}

pub(crate) fn spawn_window_show_fallback(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(WINDOW_SHOW_FALLBACK_DELAY).await;

        let state = app_handle.state::<StartupWindowState>();
        if state.is_startup_handled() {
            return;
        }

        log::warn!(
            "Frontend did not signal readiness within {:?}; showing the main window",
            WINDOW_SHOW_FALLBACK_DELAY
        );

        for attempt in 1..=WINDOW_SHOW_MAX_ATTEMPTS {
            match reveal_main_window(&app_handle) {
                Ok(RevealOutcome::Shown) => {
                    state.mark_frontend_ready();
                    return;
                }
                Ok(RevealOutcome::HiddenByAutostart) => return,
                Err(error) => {
                    log::warn!(
                        "Failed to show the main window during fallback (attempt {}/{}): {}",
                        attempt,
                        WINDOW_SHOW_MAX_ATTEMPTS,
                        error
                    );
                    if attempt < WINDOW_SHOW_MAX_ATTEMPTS {
                        tokio::time::sleep(WINDOW_SHOW_RETRY_DELAY).await;
                    }
                }
            }
        }

        log::error!(
            "Failed to show the main window after {} fallback attempts",
            WINDOW_SHOW_MAX_ATTEMPTS
        );
    });
}

fn spawn_event_forwarder(app: &tauri::App) {
    let app_handle = app.handle().clone();
    let state: tauri::State<'_, AppState> = app.state();
    let pm = state.process_manager.clone();
    let mut rx = pm.subscribe_runtime_events();

    tauri::async_runtime::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(_event) => {
                    let pm_clone = pm.clone();
                    match tokio::task::spawn_blocking(move || {
                        commands::build_app_snapshot_with(&pm_clone, load_config, load_manifest)
                    })
                    .await
                    {
                        Ok(Ok(snapshot)) => {
                            let _ = app_handle.emit("app-snapshot", &snapshot);
                        }
                        Ok(Err(e)) => {
                            log::warn!("Failed to build app snapshot for event: {}", e);
                        }
                        Err(e) => {
                            log::warn!("Snapshot task panicked: {}", e);
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    log::warn!("Runtime event listener lagged, skipped {} events", skipped);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

fn spawn_log_forwarder(app: &tauri::App) {
    let app_handle = app.handle().clone();
    let mut log_rx = log_channel::get_log_sender().subscribe();

    tauri::async_runtime::spawn(async move {
        loop {
            match log_rx.recv().await {
                Ok(entry) => {
                    let _ = app_handle.emit("log-entry", entry);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

fn spawn_resource_alert_forwarder(app: &tauri::App) {
    let app_handle = app.handle().clone();
    let state: tauri::State<'_, AppState> = app.state();
    let pm = state.process_manager.clone();
    let mut rx = pm.subscribe_resource_alerts();

    tauri::async_runtime::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(alert) => {
                    let _ = app_handle.emit("resource-alert", &alert);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    log::warn!("Resource alert listener lagged, skipped {} events", skipped);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

fn restore_instances(app: &tauri::App) {
    if let (Ok(cfg), Ok(manifest)) = (load_config(), load_manifest()) {
        if cfg.persist_instance_state && !manifest.tracked_instances_snapshot.is_empty() {
            let ids = manifest.tracked_instances_snapshot.clone();
            let restore_handle = app.handle().clone();
            let state: tauri::State<'_, AppState> = app.state();
            let pm = state.process_manager.clone();

            tauri::async_runtime::spawn(async move {
                for id in &ids {
                    log::info!("Restoring instance: {}", id);
                    if let Err(e) = pm.start_instance(id, restore_handle.clone()).await {
                        log::error!("Failed to restore instance {}: {:?}", id, e);
                    }
                }
                // Clear the snapshot after restoration attempt
                let _ = with_manifest_mut(|manifest| {
                    manifest.tracked_instances_snapshot.clear();
                    Ok(())
                });
            });
        }
    }
}
