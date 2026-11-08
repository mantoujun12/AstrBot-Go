use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use tauri::{
    AppHandle, Emitter as _, LogicalSize, Manager as _, PhysicalPosition, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder,
};

use crate::config::TrayStyle;

pub const PANEL_LABEL: &str = "tray-panel";

/// The card is 360 logical pixels wide; the window adds 6px of transparent
/// padding on each side so the CSS shadow is not clipped by the window edge.
const PANEL_WIDTH: f64 = 372.0;
const PANEL_MIN_HEIGHT: f64 = 160.0;
const PANEL_MAX_HEIGHT: f64 = 600.0;
const PANEL_DEFAULT_HEIGHT: f64 = 320.0;
const PANEL_GAP: f64 = 8.0;
const PANEL_MARGIN: f64 = 8.0;
/// A click on the tray icon blurs the panel before the tray event arrives.
/// Ignoring a toggle right after that blur keeps the click from reopening it.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

pub struct TrayPanelState {
    visible: AtomicBool,
    height: AtomicU32,
    last_blur_hide: Mutex<Option<Instant>>,
}

impl Default for TrayPanelState {
    fn default() -> Self {
        Self {
            visible: AtomicBool::new(false),
            height: AtomicU32::new(PANEL_DEFAULT_HEIGHT as u32),
            last_blur_hide: Mutex::new(None),
        }
    }
}

#[derive(Clone, Copy)]
struct Placement {
    x: i32,
    y: i32,
    height: f64,
}

pub fn prewarm(app: &AppHandle) {
    if crate::tray::current_style(app) != TrayStyle::Panel {
        return;
    }
    if let Err(error) = ensure_panel(app) {
        log::warn!("Failed to create the tray panel window: {}", error);
    }
}

pub fn ensure_panel(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    if let Some(window) = app.get_webview_window(PANEL_LABEL) {
        return Ok(window);
    }

    let window = WebviewWindowBuilder::new(
        app,
        PANEL_LABEL,
        WebviewUrl::App("src/tray-panel.html".into()),
    )
    .title("AstrBot Launcher")
    .inner_size(PANEL_WIDTH, PANEL_DEFAULT_HEIGHT)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .decorations(false)
    // Transparent windows need macOSPrivateApi, so only Windows opts in.
    .transparent(cfg!(target_os = "windows"))
    // The native shadow follows the window rectangle rather than the
    // rounded card, which leaves a ghost outline around the transparent
    // padding. The card draws its own CSS shadow instead.
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .focused(false)
    .visible(false)
    .build()?;

    if let Err(error) = window.set_background_color(Some(tauri::window::Color(0, 0, 0, 0))) {
        log::warn!("Failed to set the tray panel background: {}", error);
    }

    Ok(window)
}

pub fn toggle_if_panel(app: &AppHandle) {
    if crate::tray::current_style(app) != TrayStyle::Panel {
        return;
    }
    toggle(app);
}

fn toggle(app: &AppHandle) {
    let window = match ensure_panel(app) {
        Ok(window) => window,
        Err(error) => {
            log::warn!("Failed to open the tray panel: {}", error);
            return;
        }
    };

    if window.is_visible().unwrap_or(false) {
        hide_panel(app);
        return;
    }

    if take_recent_blur_hide(app) {
        return;
    }

    show_panel(app, &window);
}

fn take_recent_blur_hide(app: &AppHandle) -> bool {
    let state = app.state::<TrayPanelState>();
    let mut last = state
        .last_blur_hide
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let recent = matches!(*last, Some(at) if at.elapsed() < REOPEN_GUARD);
    *last = None;
    recent
}

pub fn hide_panel(app: &AppHandle) {
    let state = app.state::<TrayPanelState>();
    state.visible.store(false, Ordering::Release);
    if let Some(window) = app.get_webview_window(PANEL_LABEL) {
        if let Err(error) = window.hide() {
            log::warn!("Failed to hide the tray panel: {}", error);
        }
    }
}

pub fn hide_if_visible(app: &AppHandle) {
    let state = app.state::<TrayPanelState>();
    if state.visible.load(Ordering::Acquire) {
        hide_panel(app);
    }
}

/// Called from the window event handler when the panel loses focus.
pub fn handle_blur(window: &tauri::Window) {
    let app = window.app_handle();
    let state = app.state::<TrayPanelState>();
    if !state.visible.swap(false, Ordering::AcqRel) {
        return;
    }

    if let Err(error) = window.hide() {
        log::warn!("Failed to hide the tray panel: {}", error);
    }
    let mut last = state
        .last_blur_hide
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    *last = Some(Instant::now());
}

pub fn set_height(app: &AppHandle, height: f64) {
    if !height.is_finite() {
        return;
    }
    let clamped = height.clamp(PANEL_MIN_HEIGHT, PANEL_MAX_HEIGHT);
    let state = app.state::<TrayPanelState>();
    state
        .height
        .store(clamped.round() as u32, Ordering::Relaxed);

    // Apply immediately while visible so the panel can shrink after loading.
    let Some(window) = app.get_webview_window(PANEL_LABEL) else {
        return;
    };
    if !window.is_visible().unwrap_or(false) {
        return;
    }
    if let Some(placement) =
        compute_placement(app, clamped).or_else(|| fallback_placement(app, clamped))
    {
        apply_placement(&window, placement);
    }
}

fn apply_placement(window: &WebviewWindow, placement: Placement) {
    if let Err(error) = window.set_size(LogicalSize::new(PANEL_WIDTH, placement.height)) {
        log::warn!("Failed to resize the tray panel: {}", error);
    }
    if let Err(error) = window.set_position(PhysicalPosition::new(placement.x, placement.y)) {
        log::warn!("Failed to position the tray panel: {}", error);
    }
}

fn show_panel(app: &AppHandle, window: &WebviewWindow) {
    let state = app.state::<TrayPanelState>();
    let height = state.height.load(Ordering::Relaxed) as f64;

    if let Some(placement) =
        compute_placement(app, height).or_else(|| fallback_placement(app, height))
    {
        apply_placement(window, placement);
    }

    if let Err(error) = window.show() {
        log::warn!("Failed to show the tray panel: {}", error);
        return;
    }

    state.visible.store(true, Ordering::Release);
    *state
        .last_blur_hide
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = None;

    if let Err(error) = window.set_focus() {
        log::warn!("Failed to focus the tray panel: {}", error);
    }
    if let Err(error) = app.emit("tray-panel-shown", ()) {
        log::warn!("Failed to notify the tray panel: {}", error);
    }
}

fn compute_placement(app: &AppHandle, requested_height: f64) -> Option<Placement> {
    let tray = app.tray_by_id(crate::tray::TRAY_ID)?;
    let rect = tray.rect().ok().flatten()?;
    let monitor = monitor_for_rect(app, &rect)?;
    let scale = monitor.scale_factor();

    let position = rect.position.to_physical::<f64>(scale);
    let size = rect.size.to_physical::<f64>(scale);

    let work = monitor.work_area();
    let left = work.position.x as f64;
    let top = work.position.y as f64;
    let right = left + work.size.width as f64;
    let bottom = top + work.size.height as f64;

    let width_px = PANEL_WIDTH * scale;
    let available_height = (bottom - top - PANEL_MARGIN * 2.0) / scale;
    let height = requested_height
        .clamp(PANEL_MIN_HEIGHT, PANEL_MAX_HEIGHT)
        .min(available_height)
        .max(PANEL_MIN_HEIGHT);
    let height_px = height * scale;

    let tray_left = position.x;
    let tray_top = position.y;
    let tray_right = tray_left + size.width;
    let tray_bottom = tray_top + size.height;

    let distance_top = (tray_top - top).max(0.0);
    let distance_bottom = (bottom - tray_bottom).max(0.0);
    let distance_left = (tray_left - left).max(0.0);
    let distance_right = (right - tray_right).max(0.0);

    let (x, y) = if distance_bottom <= distance_top
        && distance_bottom <= distance_left
        && distance_bottom <= distance_right
    {
        // Taskbar at the bottom: open above the tray icon.
        (
            tray_left + size.width / 2.0 - width_px / 2.0,
            tray_top - height_px - PANEL_GAP,
        )
    } else if distance_top <= distance_left && distance_top <= distance_right {
        (
            tray_left + size.width / 2.0 - width_px / 2.0,
            tray_bottom + PANEL_GAP,
        )
    } else if distance_left <= distance_right {
        (
            tray_right + PANEL_GAP,
            tray_top + size.height / 2.0 - height_px / 2.0,
        )
    } else {
        (
            tray_left - width_px - PANEL_GAP,
            tray_top + size.height / 2.0 - height_px / 2.0,
        )
    };

    let max_x = (right - width_px - PANEL_MARGIN).max(left + PANEL_MARGIN);
    let max_y = (bottom - height_px - PANEL_MARGIN).max(top + PANEL_MARGIN);
    Some(Placement {
        x: x.clamp(left + PANEL_MARGIN, max_x).round() as i32,
        y: y.clamp(top + PANEL_MARGIN, max_y).round() as i32,
        height,
    })
}

/// Used when the tray icon has no rectangle yet: anchor to the bottom-right
/// corner of the primary work area instead of leaving the window unplaced.
fn fallback_placement(app: &AppHandle, requested_height: f64) -> Option<Placement> {
    let monitor = app.primary_monitor().ok().flatten()?;
    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    let left = work.position.x as f64;
    let top = work.position.y as f64;
    let right = left + work.size.width as f64;
    let bottom = top + work.size.height as f64;

    let width_px = PANEL_WIDTH * scale;
    let available_height = (bottom - top - PANEL_MARGIN * 2.0) / scale;
    let height = requested_height
        .clamp(PANEL_MIN_HEIGHT, PANEL_MAX_HEIGHT)
        .min(available_height)
        .max(PANEL_MIN_HEIGHT);
    let height_px = height * scale;

    let x = (right - width_px - PANEL_MARGIN).max(left + PANEL_MARGIN);
    let y = (bottom - height_px - PANEL_MARGIN).max(top + PANEL_MARGIN);
    Some(Placement {
        x: x.round() as i32,
        y: y.round() as i32,
        height,
    })
}

fn monitor_for_rect(app: &AppHandle, rect: &tauri::Rect) -> Option<tauri::Monitor> {
    let primary_scale = app
        .primary_monitor()
        .ok()
        .flatten()
        .map(|monitor| monitor.scale_factor())
        .unwrap_or(1.0);
    let position = rect.position.to_physical::<f64>(primary_scale);
    let size = rect.size.to_physical::<f64>(primary_scale);
    let center_x = position.x + size.width / 2.0;
    let center_y = position.y + size.height / 2.0;

    app.monitor_from_point(center_x, center_y)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())
}
