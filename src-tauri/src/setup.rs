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
const TRANSPARENT_WINDOW_BACKGROUND: tauri::window::Color = tauri::window::Color(0, 0, 0, 0);

const WINDOWS_11_FIRST_BUILD: u32 = 22000;
const WINDOWS_11_22H2_BUILD: u32 = 22621;

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
    crate::tray_panel::prewarm(app.handle());
    restore_instances(app);

    Ok(())
}

pub fn configure_main_window(window: &tauri::WebviewWindow) {
    apply_initial_background(window);
    #[cfg(target_os = "windows")]
    {
        // The backdrop must be applied first: setting a system backdrop can
        // reset the corner preference, so rounding has to come after it.
        apply_windows_backdrop(window);
        apply_windows_corner_preference(window);
    }

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
    let color = if window_backdrop_is_transparent() {
        TRANSPARENT_WINDOW_BACKGROUND
    } else if dark {
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

/// Whether the main window backdrop must stay fully transparent so the native
/// material shows through the webview: Mica on Windows 11, vibrancy on macOS.
/// Anything opaque would cover the material, so Windows 10 (no Mica) keeps the
/// opaque theme colour instead.
fn window_backdrop_is_transparent() -> bool {
    #[cfg(target_os = "windows")]
    {
        windows_supports_mica()
    }
    #[cfg(target_os = "macos")]
    {
        true
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        false
    }
}

/// Mica requires Windows 11 (build 22000+). `RtlGetVersion` reports the real
/// build number, while `GetVersionEx` is shimmed by the application manifest.
#[cfg(target_os = "windows")]
fn windows_supports_mica() -> bool {
    windows_build_number() >= WINDOWS_11_FIRST_BUILD
}

/// Mica Alt (`DWMSBT_TABBEDWINDOW`) needs the system backdrop attribute that
/// shipped with Windows 11 22H2 (build 22621+).
#[cfg(target_os = "windows")]
fn windows_supports_mica_alt() -> bool {
    windows_build_number() >= WINDOWS_11_22H2_BUILD
}

/// Reads the real Windows build number via `RtlGetVersion`.
#[cfg(target_os = "windows")]
fn windows_build_number() -> u32 {
    use windows::Wdk::System::SystemServices::RtlGetVersion;
    use windows::Win32::System::SystemInformation::OSVERSIONINFOW;

    // SAFETY: OSVERSIONINFOW is a plain C struct for which all-zero is a valid
    // value; its size field is set as RtlGetVersion requires before the call.
    let mut info: OSVERSIONINFOW = unsafe { std::mem::zeroed() };
    info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOW>() as u32;
    if unsafe { RtlGetVersion(&mut info) }.is_ok() {
        info.dwBuildNumber
    } else {
        0
    }
}

/// Applies the Windows 11 backdrop material directly, logging the outcome:
/// Tauri's config-driven application discards every error silently, which made
/// Mica failures invisible. Mica Alt tints the window with the wallpaper much
/// more strongly than plain Mica; builds 22000-22620 only support the
/// undocumented Mica attribute (1029), as used by `window-vibrancy`.
#[cfg(target_os = "windows")]
fn apply_windows_backdrop(window: &tauri::WebviewWindow) {
    use std::ffi::c_void;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMSBT_TABBEDWINDOW, DWMWA_SYSTEMBACKDROP_TYPE, DWMWINDOWATTRIBUTE,
        DWM_SYSTEMBACKDROP_TYPE,
    };

    const DWMWA_MICA_EFFECT: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(1029);
    const MICA_EFFECT_ON: u32 = 1;
    const PLAIN_MICA: &str = "Mica";
    const MICA_ALT: &str = "Mica Alt";

    let material = if windows_supports_mica_alt() {
        MICA_ALT
    } else {
        PLAIN_MICA
    };

    let result = window
        .hwnd()
        .map_err(|error| error.to_string())
        .and_then(|hwnd| {
            if windows_supports_mica_alt() {
                // SAFETY: hwnd is a valid top-level window handle and both the
                // attribute constant and the value type match the DWM contract.
                let backdrop = DWMSBT_TABBEDWINDOW;
                unsafe {
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_SYSTEMBACKDROP_TYPE,
                        std::ptr::from_ref(&backdrop).cast::<c_void>(),
                        std::mem::size_of::<DWM_SYSTEMBACKDROP_TYPE>() as u32,
                    )
                }
                .map_err(|error| error.to_string())
            } else if windows_supports_mica() {
                // SAFETY: hwnd is a valid top-level window handle; attribute
                // 1029 takes a BOOL-like u32 value.
                let enable = MICA_EFFECT_ON;
                unsafe {
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_MICA_EFFECT,
                        std::ptr::from_ref(&enable).cast::<c_void>(),
                        std::mem::size_of::<u32>() as u32,
                    )
                }
                .map_err(|error| error.to_string())
            } else {
                Err("Windows 10 has no Mica material".to_string())
            }
        });

    match result {
        Ok(()) => log::info!(
            "Applied the {material} backdrop to the main window (Windows build {})",
            windows_build_number()
        ),
        Err(error) => log::warn!("Failed to apply the {material} backdrop: {error}"),
    }
}

/// Windows 11 draws a frameless window with square corners. Asking DWM for
/// rounded corners restores them; builds that ignore the attribute just return
/// an error, which is only logged.
#[cfg(target_os = "windows")]
fn apply_windows_corner_preference(window: &tauri::WebviewWindow) {
    use std::ffi::c_void;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
        DWM_WINDOW_CORNER_PREFERENCE,
    };

    let result = window
        .hwnd()
        .map_err(|error| error.to_string())
        .and_then(|hwnd| {
            let preference: DWM_WINDOW_CORNER_PREFERENCE = DWMWCP_ROUND;
            unsafe {
                DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    std::ptr::from_ref(&preference).cast::<c_void>(),
                    std::mem::size_of::<DWM_WINDOW_CORNER_PREFERENCE>() as u32,
                )
            }
            .map_err(|error| error.to_string())
        });

    if let Err(error) = result {
        log::warn!("Failed to round the main window corners: {}", error);
    } else {
        log::info!("Rounded the main window corners (DWMWCP_ROUND)");
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
    crate::tray_panel::hide_if_visible(app);
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

/// Reveals the main window for an explicit user gesture coming from the
/// frontend (tray panel "open" actions).
///
/// Unlike [`reveal_main_window`] this always overrides the autostart
/// minimize-to-tray policy, because a pending show request is registered
/// first.
pub(crate) fn show_main_window_from_user(app: &tauri::AppHandle) -> tauri::Result<RevealOutcome> {
    app.state::<StartupWindowState>().request_show();
    reveal_main_window(app)
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
                            crate::tray::sync_tray_menu(&app_handle, &snapshot.instances);
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
