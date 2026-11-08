use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use serde_json::json;
use tauri::menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter as _, Manager as _};

use crate::config::{load_config, load_manifest, TrayStyle};
use crate::error::AppError;
use crate::i18n::Locale;
use crate::instance::{list_instances, InstanceStatus};
use crate::process::InstanceState;

pub(crate) const TRAY_ID: &str = "main-tray";
const MENU_REBUILD_DEBOUNCE: Duration = Duration::from_millis(300);
/// How often the rendered running durations are refreshed while an instance
/// is running. Rebuilding more often would churn the native menu.
const MENU_REFRESH_INTERVAL: Duration = Duration::from_secs(60);

struct TrayMenuItems {
    show: MenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
}

#[derive(Debug, Clone)]
struct TrayInstanceEntry {
    id: String,
    name: String,
    state: InstanceState,
    port: u16,
    duration: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct TrayMenuData {
    instances: Vec<TrayInstanceEntry>,
    running: usize,
    total: usize,
}

impl TrayMenuData {
    fn from_instances(locale: Locale, instances: &[InstanceStatus]) -> Self {
        let entries = instances
            .iter()
            .map(|instance| TrayInstanceEntry {
                id: instance.id.clone(),
                name: instance.name.clone(),
                state: instance.state,
                port: instance.port,
                duration: format_duration(locale, instance.started_at_ms),
            })
            .collect::<Vec<_>>();
        let running = entries
            .iter()
            .filter(|entry| entry.state == InstanceState::Running)
            .count();
        Self {
            total: entries.len(),
            instances: entries,
            running,
        }
    }
}

#[derive(Default)]
struct TrayStateInner {
    style: TrayStyle,
    legacy_menu: Option<TrayMenuItems>,
    data: TrayMenuData,
}

#[derive(Default)]
pub struct TrayState {
    inner: Mutex<TrayStateInner>,
    epoch: AtomicU64,
}

fn lock_state(state: &TrayState) -> MutexGuard<'_, TrayStateInner> {
    state.inner.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) fn ensure_state(app: &AppHandle) -> tauri::State<'_, TrayState> {
    if app.try_state::<TrayState>().is_none() {
        let _ = app.manage(TrayState::default());
    }
    app.state::<TrayState>()
}

pub(crate) fn current_style(app: &AppHandle) -> TrayStyle {
    app.try_state::<TrayState>()
        .map(|state| lock_state(&state).style)
        .unwrap_or_default()
}

/// AppIndicator on Linux does not deliver right-click events, so the panel
/// style is not reachable there and the native menu is used instead.
#[cfg(target_os = "linux")]
fn effective_style(_style: TrayStyle) -> TrayStyle {
    TrayStyle::NativeMenu
}

#[cfg(not(target_os = "linux"))]
fn effective_style(style: TrayStyle) -> TrayStyle {
    style
}

/// Linux AppIndicator cannot reliably rebuild its menu while it is attached,
/// so the dynamic instance menu is only used on the other platforms.
fn supports_dynamic_menu() -> bool {
    !cfg!(target_os = "linux")
}

fn current_locale() -> Locale {
    load_config()
        .map(|config| Locale::resolve(config.language_preference))
        .unwrap_or_else(|_| Locale::detect())
}

fn format_duration(locale: Locale, started_at_ms: Option<i64>) -> Option<String> {
    let started_at = started_at_ms?;
    let elapsed_ms = chrono::Utc::now()
        .timestamp_millis()
        .saturating_sub(started_at)
        .max(0);
    let minutes = elapsed_ms / 60_000;
    if minutes < 1 {
        return Some(locale.tray_duration_less_minute().to_string());
    }
    let hours = minutes / 60;
    Some(if hours == 0 {
        locale.tray_duration_minutes(minutes)
    } else if minutes % 60 == 0 {
        locale.tray_duration_hours(hours)
    } else {
        locale.tray_duration_hours_minutes(hours, minutes % 60)
    })
}

fn instance_title(entry: &TrayInstanceEntry, locale: Locale) -> String {
    let state = locale.instance_state(entry.state);
    match entry.duration.as_deref() {
        Some(duration) => format!(
            "{} · {} · :{} · {}",
            entry.name, state, entry.port, duration
        ),
        None => format!("{} · {} · :{}", entry.name, state, entry.port),
    }
}

fn collect_menu_data(app: &AppHandle) -> TrayMenuData {
    let Some(state) = app.try_state::<crate::commands::AppState>() else {
        return TrayMenuData::default();
    };
    let Ok(manifest) = load_manifest() else {
        return TrayMenuData::default();
    };
    let instances = list_instances(&state.process_manager, manifest.as_ref());
    TrayMenuData::from_instances(current_locale(), &instances)
}

fn current_menu_data(app: &AppHandle) -> TrayMenuData {
    app.try_state::<TrayState>()
        .map(|state| lock_state(&state).data.clone())
        .unwrap_or_default()
}

fn update_tooltip(app: &AppHandle, data: &TrayMenuData) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let locale = current_locale();
    let text = format!(
        "AstrBot Launcher · {}",
        locale.tray_status(data.running, data.total)
    );
    if let Err(error) = tray.set_tooltip(Some(text)) {
        log::warn!("Failed to update tray tooltip: {}", error);
    }
}

fn item_refs(items: &[Box<dyn IsMenuItem<tauri::Wry>>]) -> Vec<&dyn IsMenuItem<tauri::Wry>> {
    items.iter().map(|item| item.as_ref()).collect()
}

fn build_legacy_menu(app: &AppHandle) -> tauri::Result<(Menu<tauri::Wry>, TrayMenuItems)> {
    let locale = current_locale();
    let show = MenuItem::with_id(app, "show", locale.tray_show(), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", locale.tray_quit(), true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    Ok((menu, TrayMenuItems { show, quit }))
}

fn build_instances_entry(
    app: &AppHandle,
    data: &TrayMenuData,
    locale: Locale,
) -> tauri::Result<Box<dyn IsMenuItem<tauri::Wry>>> {
    if data.instances.is_empty() {
        return Ok(Box::new(MenuItem::with_id(
            app,
            "no-instances",
            locale.tray_no_instances(),
            false,
            None::<&str>,
        )?));
    }

    let mut entries: Vec<Box<dyn IsMenuItem<tauri::Wry>>> =
        Vec::with_capacity(data.instances.len());
    for entry in &data.instances {
        let mut actions: Vec<Box<dyn IsMenuItem<tauri::Wry>>> = Vec::new();
        match entry.state {
            InstanceState::Running => {
                actions.push(Box::new(MenuItem::with_id(
                    app,
                    format!("open:{}", entry.id),
                    locale.tray_open_webui(),
                    true,
                    None::<&str>,
                )?));
                actions.push(Box::new(MenuItem::with_id(
                    app,
                    format!("restart:{}", entry.id),
                    locale.tray_restart(),
                    true,
                    None::<&str>,
                )?));
            }
            InstanceState::Stopped => {
                actions.push(Box::new(MenuItem::with_id(
                    app,
                    format!("start:{}", entry.id),
                    locale.tray_start(),
                    true,
                    None::<&str>,
                )?));
            }
            InstanceState::Starting | InstanceState::Stopping => {
                actions.push(Box::new(MenuItem::with_id(
                    app,
                    format!("busy:{}", entry.id),
                    locale.instance_state(entry.state),
                    false,
                    None::<&str>,
                )?));
            }
        }
        let refs = item_refs(&actions);
        let submenu = Submenu::with_items(app, instance_title(entry, locale), true, &refs)?;
        entries.push(Box::new(submenu));
    }

    let refs = item_refs(&entries);
    let instances = Submenu::with_items(app, locale.tray_instances(data.total), true, &refs)?;
    Ok(Box::new(instances))
}

fn build_full_menu(app: &AppHandle, data: &TrayMenuData) -> tauri::Result<Menu<tauri::Wry>> {
    let locale = current_locale();
    let show = MenuItem::with_id(app, "show", locale.tray_show(), true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", locale.tray_settings(), true, None::<&str>)?;
    let instances = build_instances_entry(app, data, locale)?;
    let quit = MenuItem::with_id(app, "quit", locale.tray_quit(), true, None::<&str>)?;

    let items: Vec<Box<dyn IsMenuItem<tauri::Wry>>> = vec![
        Box::new(show),
        Box::new(settings),
        Box::new(PredefinedMenuItem::separator(app)?),
        instances,
        Box::new(PredefinedMenuItem::separator(app)?),
        Box::new(quit),
    ];
    let refs = item_refs(&items);
    Menu::with_items(app, &refs)
}

fn rebuild_menu(app: &AppHandle) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    let data = current_menu_data(app);
    let menu = build_full_menu(app, &data)?;
    tray.set_menu(Some(menu))?;
    Ok(())
}

/// The dynamic menu renders the running duration as static text, and
/// [`sync_tray_menu`] only runs on instance state transitions, so an instance
/// that keeps running would show the duration captured when it started.
/// Recompute the menu model periodically instead of polling every second.
fn spawn_menu_refresh_task(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(MENU_REFRESH_INTERVAL).await;

            // Managed state is dropped while the app shuts down; the loop then
            // ends with the runtime instead of keeping the process alive.
            if app.try_state::<TrayState>().is_none() {
                break;
            }
            if current_style(&app) != TrayStyle::NativeMenu || !supports_dynamic_menu() {
                continue;
            }
            if current_menu_data(&app).running == 0 {
                continue;
            }

            refresh_menu_data(&app);
            schedule_menu_rebuild(&app);
        }
    });
}

fn schedule_menu_rebuild(app: &AppHandle) {
    let state = ensure_state(app);
    let epoch = state.epoch.fetch_add(1, Ordering::SeqCst).wrapping_add(1);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(MENU_REBUILD_DEBOUNCE).await;
        {
            let state = ensure_state(&app);
            if state.epoch.load(Ordering::SeqCst) != epoch {
                return;
            }
        }
        if current_style(&app) != TrayStyle::NativeMenu || !supports_dynamic_menu() {
            return;
        }
        if let Err(error) = rebuild_menu(&app) {
            log::warn!("Failed to rebuild tray menu: {}", error);
        }
    });
}

fn refresh_menu_data(app: &AppHandle) {
    let data = collect_menu_data(app);
    let state = ensure_state(app);
    {
        let mut inner = lock_state(&state);
        inner.data = data.clone();
    }
    update_tooltip(app, &data);
}

/// Refreshes the menu model after an instance snapshot so the tray reflects
/// process changes without polling.
pub fn sync_tray_menu(app: &AppHandle, instances: &[InstanceStatus]) {
    let data = TrayMenuData::from_instances(current_locale(), instances);
    let state = ensure_state(app);
    {
        let mut inner = lock_state(&state);
        inner.data = data.clone();
    }
    update_tooltip(app, &data);
    if current_style(app) == TrayStyle::NativeMenu && supports_dynamic_menu() {
        schedule_menu_rebuild(app);
    }
}

fn spawn_instance_action(app: &AppHandle, instance_id: &str, restart: bool) {
    let app = app.clone();
    let instance_id = instance_id.to_string();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<crate::commands::AppState>();
        let result = if restart {
            crate::commands::restart_instance(app.clone(), instance_id, state).await
        } else {
            crate::commands::start_instance(app.clone(), instance_id, state).await
        };
        if let Err(error) = result {
            log::warn!("Tray instance action failed: {}", error);
        }
    });
}

fn handle_menu_event(app: &AppHandle, id: &str) {
    match id {
        "show" => crate::setup::request_show_main_window(app),
        "settings" => {
            crate::setup::request_show_main_window(app);
            let _ = app.emit_to("main", "tray-panel-action", json!({ "kind": "settings" }));
        }
        "quit" => app.exit(0),
        _ => {
            if let Some(instance_id) = id.strip_prefix("open:") {
                crate::setup::request_show_main_window(app);
                let _ = app.emit_to(
                    "main",
                    "tray-panel-action",
                    json!({ "kind": "instance", "instanceId": instance_id }),
                );
            } else if let Some(instance_id) = id.strip_prefix("start:") {
                spawn_instance_action(app, instance_id, false);
            } else if let Some(instance_id) = id.strip_prefix("restart:") {
                spawn_instance_action(app, instance_id, true);
            }
        }
    }
}

#[allow(clippy::expect_used)]
fn create_tray(app: &AppHandle, style: TrayStyle) -> tauri::Result<()> {
    let style = effective_style(style);
    let data = current_menu_data(app);
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().expect("no default icon").clone())
        .tooltip("AstrBot Launcher")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu_event(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            let TrayIconEvent::Click {
                button,
                button_state,
                ..
            } = event
            else {
                return;
            };
            if button_state != MouseButtonState::Up {
                return;
            }
            match button {
                MouseButton::Left => crate::setup::request_show_main_window(tray.app_handle()),
                MouseButton::Right => crate::tray_panel::toggle_if_panel(tray.app_handle()),
                _ => {}
            }
        });

    let mut legacy_menu = None;
    match style {
        TrayStyle::NativeMenu if supports_dynamic_menu() => {
            builder = builder.menu(&build_full_menu(app, &data)?);
        }
        TrayStyle::NativeMenu => {
            let (menu, items) = build_legacy_menu(app)?;
            builder = builder.menu(&menu);
            legacy_menu = Some(items);
        }
        TrayStyle::Panel => {}
    }

    let _tray = builder.build(app)?;
    update_tooltip(app, &data);

    let state = ensure_state(app);
    let mut inner = lock_state(&state);
    inner.style = style;
    inner.legacy_menu = legacy_menu;
    drop(inner);
    Ok(())
}

#[allow(clippy::expect_used)]
pub fn build_tray(app: &tauri::App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    refresh_menu_data(app.handle());
    let style = load_config()
        .map(|config| config.tray_style)
        .unwrap_or_default();
    create_tray(app.handle(), style)?;
    spawn_menu_refresh_task(app.handle());
    Ok(())
}

/// Replaces the tray icon so the new style takes effect immediately.
pub fn apply_tray_style(app: &AppHandle, style: TrayStyle) -> crate::Result<()> {
    let state = ensure_state(app);
    {
        let mut inner = lock_state(&state);
        inner.style = effective_style(style);
    }
    // Invalidate rebuilds that are still waiting for their debounce, otherwise
    // one of them could attach the previous menu to the icon created below.
    state.epoch.fetch_add(1, Ordering::SeqCst);

    app.remove_tray_by_id(TRAY_ID);
    refresh_menu_data(app);
    create_tray(app, style)
        .map_err(|error| AppError::other(format!("Failed to rebuild tray icon: {}", error)))?;

    if effective_style(style) == TrayStyle::Panel {
        crate::tray_panel::prewarm(app);
    } else {
        crate::tray_panel::hide_if_visible(app);
    }

    Ok(())
}

/// Rebuilds the dynamic menu after a language change. Linux keeps the fixed
/// two-item menu, so its labels are updated in place.
pub fn refresh_tray_menu(app: &AppHandle) {
    // Menu labels embed the rendered duration and the tooltip embeds the
    // running count, so refresh the cached data with the new locale before
    // rebuilding instead of reusing the previous language.
    refresh_menu_data(app);

    if current_style(app) == TrayStyle::NativeMenu && supports_dynamic_menu() {
        if let Err(error) = rebuild_menu(app) {
            log::warn!("Failed to rebuild tray menu: {}", error);
        }
        return;
    }

    let Some(state) = app.try_state::<TrayState>() else {
        return;
    };
    let inner = lock_state(&state);
    let Some(items) = inner.legacy_menu.as_ref() else {
        return;
    };

    let locale = current_locale();
    if let Err(error) = items.show.set_text(locale.tray_show()) {
        log::warn!("Failed to update tray show label: {}", error);
    }
    if let Err(error) = items.quit.set_text(locale.tray_quit()) {
        log::warn!("Failed to update tray quit label: {}", error);
    }
    drop(inner);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_covers_minutes_and_hours() {
        let now = chrono::Utc::now().timestamp_millis();
        assert_eq!(
            format_duration(Locale::EnUs, Some(now - 30_000)),
            Some("< 1 min".to_string())
        );
        assert_eq!(
            format_duration(Locale::EnUs, Some(now - 90_000)),
            Some("1 min".to_string())
        );
        assert_eq!(
            format_duration(Locale::EnUs, Some(now - 2 * 3_600_000)),
            Some("2 h".to_string())
        );
        assert_eq!(
            format_duration(Locale::EnUs, Some(now - (2 * 3_600_000 + 5 * 60_000))),
            Some("2 h 5 min".to_string())
        );
        assert_eq!(format_duration(Locale::EnUs, None), None);
    }
}
