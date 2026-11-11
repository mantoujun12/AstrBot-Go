use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use reqwest::Url;
use serde::Serialize;
use tauri::{AppHandle, Emitter as _, State};
use tauri_plugin_updater::{Update, Updater as TauriUpdater, UpdaterExt as _};

use crate::config::{load_config, AppConfig};
use crate::error::{AppError, Result};
use crate::network_config;
use crate::utils::index_url::wrap_with_proxy;
use crate::utils::log_redact;
use crate::utils::proxy::ProxySource;
use crate::utils::sync::lock_mutex_recover;

const PROGRESS_EVENT_NAME: &str = "launcher-update-progress";
const PROGRESS_THROTTLE: Duration = Duration::from_millis(100);

struct PendingUpdate {
    update: Update,
    bytes: Vec<u8>,
}

#[derive(Default)]
pub(crate) struct LauncherUpdateState {
    pending: Mutex<Option<PendingUpdate>>,
    operation_in_progress: AtomicBool,
}

impl LauncherUpdateState {
    fn begin_operation(&self) -> bool {
        self.operation_in_progress
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    fn finish_operation(&self) {
        self.operation_in_progress.store(false, Ordering::Release);
    }

    fn store_pending(&self, pending: PendingUpdate) {
        *lock_mutex_recover(&self.pending, "LauncherUpdateState.pending") = Some(pending);
    }

    fn take_pending(&self) -> Option<PendingUpdate> {
        lock_mutex_recover(&self.pending, "LauncherUpdateState.pending").take()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct LauncherUpdateInfo {
    supported: bool,
    available: bool,
    current_version: String,
    latest_version: String,
    notes: Option<String>,
    published_at: Option<String>,
}

impl LauncherUpdateInfo {
    fn not_available(current_version: String, supported: bool) -> Self {
        Self {
            supported,
            available: false,
            latest_version: current_version.clone(),
            current_version,
            notes: None,
            published_at: None,
        }
    }

    fn from_update(update: &Update, available: bool) -> Self {
        Self {
            supported: true,
            available,
            current_version: update.current_version.clone(),
            latest_version: update.version.clone(),
            notes: update.body.clone(),
            published_at: update.date.map(|date| date.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
struct LauncherUpdateProgress {
    downloaded: u64,
    total: Option<u64>,
    progress: Option<f64>,
    speed: Option<f64>,
}

#[tauri::command]
pub(crate) async fn check_launcher_update(app: AppHandle) -> Result<LauncherUpdateInfo> {
    if !cfg!(windows) {
        return Ok(LauncherUpdateInfo::not_available(
            current_app_version(&app),
            false,
        ));
    }

    let config = load_config()?;
    match check_for_update(&app, config.as_ref()).await? {
        Some(update) => Ok(LauncherUpdateInfo::from_update(&update, true)),
        None => Ok(LauncherUpdateInfo::not_available(
            current_app_version(&app),
            true,
        )),
    }
}

#[tauri::command]
pub(crate) async fn download_launcher_update(
    app: AppHandle,
    update_state: State<'_, LauncherUpdateState>,
) -> Result<LauncherUpdateInfo> {
    if !cfg!(windows) {
        return Ok(LauncherUpdateInfo::not_available(
            current_app_version(&app),
            false,
        ));
    }
    if !update_state.begin_operation() {
        return Err(AppError::launcher_update());
    }

    let download: Result<Option<(LauncherUpdateInfo, Update, Vec<u8>)>> = async {
        let config = load_config()?;
        let Some(update) = check_for_update(&app, config.as_ref()).await? else {
            return Ok(None);
        };

        let downloaded = std::sync::Arc::new(AtomicU64::new(0));
        let total = std::sync::Arc::new(AtomicU64::new(0));
        let last_emit_elapsed = std::sync::Arc::new(AtomicU64::new(0));
        let start = Instant::now();

        emit_progress(&app, 0, None, Duration::ZERO, false);

        let chunk_app = app.clone();
        let chunk_downloaded = std::sync::Arc::clone(&downloaded);
        let chunk_total = std::sync::Arc::clone(&total);
        let chunk_last_emit = std::sync::Arc::clone(&last_emit_elapsed);
        let on_chunk = move |chunk_length: usize, content_length: Option<u64>| {
            let length = u64::try_from(chunk_length).unwrap_or(0);
            let previous = chunk_downloaded.fetch_add(length, Ordering::Relaxed);
            let downloaded_now = previous.saturating_add(length);

            if let Some(content_length) = content_length {
                chunk_total.store(content_length, Ordering::Relaxed);
            }

            let now = Instant::now();
            let elapsed = now.duration_since(start);
            let elapsed_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
            let last_ms = chunk_last_emit.load(Ordering::Relaxed);
            if elapsed_ms >= last_ms.saturating_add(PROGRESS_THROTTLE.as_millis() as u64) {
                chunk_last_emit.store(elapsed_ms, Ordering::Relaxed);
                emit_progress(
                    &chunk_app,
                    downloaded_now,
                    normalized_total(content_length),
                    elapsed,
                    false,
                );
            }
        };

        let finish_app = app.clone();
        let finish_downloaded = std::sync::Arc::clone(&downloaded);
        let finish_total = std::sync::Arc::clone(&total);
        let bytes = update
            .download(on_chunk, move || {
                let downloaded_now = finish_downloaded.load(Ordering::Relaxed);
                let total = normalized_total(Some(finish_total.load(Ordering::Relaxed)));
                emit_progress(&finish_app, downloaded_now, total, start.elapsed(), true);
            })
            .await
            .map_err(map_updater_error)?;

        Ok(Some((
            LauncherUpdateInfo::from_update(&update, true),
            update,
            bytes,
        )))
    }
    .await;

    match download {
        Ok(Some((info, update, bytes))) => {
            update_state.store_pending(PendingUpdate { update, bytes });
            update_state.finish_operation();
            Ok(info)
        }
        Ok(None) => {
            update_state.finish_operation();
            Ok(LauncherUpdateInfo::not_available(
                current_app_version(&app),
                true,
            ))
        }
        Err(error) => {
            update_state.finish_operation();
            Err(error)
        }
    }
}

#[tauri::command]
pub(crate) fn install_launcher_update(update_state: State<'_, LauncherUpdateState>) -> Result<()> {
    if !cfg!(windows) {
        return Err(AppError::launcher_update());
    }
    if !update_state.begin_operation() {
        return Err(AppError::launcher_update());
    }

    let Some(pending) = update_state.take_pending() else {
        update_state.finish_operation();
        return Err(AppError::launcher_update());
    };

    let PendingUpdate { update, bytes } = pending;
    if let Err(error) = update.install(&bytes).map_err(map_updater_error) {
        update_state.store_pending(PendingUpdate { update, bytes });
        update_state.finish_operation();
        return Err(error);
    }

    // The updater exits the app after launching the Windows installer.
    Ok(())
}

#[tauri::command]
pub(crate) fn restart_launcher(app: AppHandle) {
    app.restart();
}

async fn check_for_update(app: &AppHandle, config: &AppConfig) -> Result<Option<Update>> {
    let updater = build_updater(app, config)?;
    let mut update = updater.check().await.map_err(map_updater_error)?;
    if let Some(update) = update.as_mut() {
        update.download_url = proxied_url(&config.github_proxy, update.download_url.as_str())?;
    }
    Ok(update)
}

fn build_updater(app: &AppHandle, config: &AppConfig) -> Result<TauriUpdater> {
    let mut builder = app.updater_builder();

    if !config.github_proxy.trim().is_empty() {
        let endpoints = configured_update_endpoints(app, &config.github_proxy)?;
        builder = builder.endpoints(endpoints).map_err(map_updater_error)?;
    }

    if let Some(proxy) = network_config::proxy_settings(config)?
        .filter(|proxy| proxy.source == ProxySource::AppConfig)
    {
        if let Some(proxy_url) = proxy.all_proxy.or(proxy.https_proxy).or(proxy.http_proxy) {
            let url =
                Url::parse(&proxy_url).map_err(|error| AppError::config(error.to_string()))?;
            builder = builder.proxy(url);
        }
    }

    builder.build().map_err(map_updater_error)
}

fn configured_update_endpoints(app: &AppHandle, github_proxy: &str) -> Result<Vec<Url>> {
    let endpoints = app
        .config()
        .plugins
        .0
        .get("updater")
        .and_then(|config| config.get("endpoints"))
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| AppError::config("Updater endpoints are not configured"))?;

    let mut proxied = Vec::with_capacity(endpoints.len());
    for endpoint in endpoints {
        let raw = endpoint
            .as_str()
            .ok_or_else(|| AppError::config("Updater endpoint must be a URL string"))?;
        proxied.push(proxied_url(github_proxy, raw)?);
    }

    if proxied.is_empty() {
        return Err(AppError::config("Updater endpoints are not configured"));
    }
    Ok(proxied)
}

fn proxied_url(github_proxy: &str, raw: &str) -> Result<Url> {
    Url::parse(&wrap_with_proxy(github_proxy, raw))
        .map_err(|error| AppError::config(error.to_string()))
}

fn current_app_version(app: &AppHandle) -> String {
    app.package_info().version.to_string()
}

fn normalized_total(total: Option<u64>) -> Option<u64> {
    total.filter(|value| *value > 0)
}

fn progress_percent(downloaded: u64, total: Option<u64>, complete: bool) -> Option<f64> {
    if complete {
        return Some(100.0);
    }

    normalized_total(total)
        .map(|total| (downloaded.min(total) as f64 / total as f64 * 100.0).clamp(0.0, 100.0))
}

fn emit_progress(
    app: &AppHandle,
    downloaded: u64,
    total: Option<u64>,
    elapsed: Duration,
    complete: bool,
) {
    let progress = progress_percent(downloaded, total, complete);
    let speed = if elapsed.as_secs_f64() > 0.0 {
        Some(downloaded as f64 / elapsed.as_secs_f64())
    } else {
        Some(0.0)
    };

    let _ = app.emit(
        PROGRESS_EVENT_NAME,
        LauncherUpdateProgress {
            downloaded,
            total,
            progress,
            speed,
        },
    );
}

fn map_updater_error(error: tauri_plugin_updater::Error) -> AppError {
    log::warn!(
        "Launcher update operation failed: {}",
        log_redact::redact_text(&error.to_string())
    );
    AppError::launcher_update()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_progress_handles_unknown_and_zero_totals() {
        assert_eq!(progress_percent(10, None, false), None);
        assert_eq!(progress_percent(10, Some(0), false), None);
    }

    #[test]
    fn update_progress_is_bounded_and_can_finish() {
        assert_eq!(progress_percent(25, Some(100), false), Some(25.0));
        assert_eq!(progress_percent(120, Some(100), false), Some(100.0));
        assert_eq!(progress_percent(12, None, true), Some(100.0));
    }

    #[test]
    fn only_one_update_operation_can_run_at_a_time() {
        let state = LauncherUpdateState::default();

        assert!(state.begin_operation());
        assert!(!state.begin_operation());

        state.finish_operation();
        assert!(state.begin_operation());
    }

    #[test]
    fn github_proxy_wraps_update_urls() -> Result<()> {
        assert_eq!(
            proxied_url("", "https://github.com/example/latest.json")?.as_str(),
            "https://github.com/example/latest.json"
        );
        assert_eq!(
            proxied_url(
                "https://gh-proxy.example/",
                "https://github.com/example/latest.json"
            )?
            .as_str(),
            "https://gh-proxy.example/https://github.com/example/latest.json"
        );
        Ok(())
    }
}
