use std::fs;
use std::io::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};

use futures_util::StreamExt as _;
use reqwest::Client;
use tauri::{AppHandle, Emitter as _};

use crate::config::{with_manifest_mut, InstalledVersion};
use crate::download_control::{self, ControlState, DownloadControl};
use crate::error::{AppError, ErrorKind, Result};
use crate::github::{get_source_archive_urls, GitHubRelease};
use crate::utils::net::{ensure_success_status, send_get};
use crate::utils::paths::get_versions_dir;
use crate::validation::resolve_version_zip_path;

#[derive(Debug, Clone, serde::Serialize)]
pub struct DownloadProgress {
    pub id: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    /// Backend-computed progress percentage (0-100). `None` means unknown.
    pub progress: Option<u8>,
    /// Smoothed transfer rate in bytes per second. `None` when unknown.
    pub speed: Option<u64>,
    pub step: String,
    pub message: String,
}

pub struct DownloadOptions<'a> {
    pub app_handle: &'a AppHandle,
    pub id: &'a str,
    /// Present for downloads the user may pause or cancel.
    pub control: Option<&'a DownloadControl>,
}

pub fn emit_download_progress(
    opts: &DownloadOptions,
    downloaded: u64,
    total: Option<u64>,
    progress: Option<u8>,
    speed: Option<u64>,
    step: &str,
    message: &str,
) {
    let _ = opts.app_handle.emit(
        "download-progress",
        DownloadProgress {
            id: opts.id.to_string(),
            downloaded,
            total,
            progress,
            speed,
            step: step.to_string(),
            message: message.to_string(),
        },
    );
}

/// Smoothed transfer rate derived from byte deltas between progress events.
struct SpeedMeter {
    last_bytes: u64,
    last_at: Instant,
    smoothed: f64,
}

impl SpeedMeter {
    fn new() -> Self {
        Self {
            last_bytes: 0,
            last_at: Instant::now(),
            smoothed: 0.0,
        }
    }

    /// Update the meter; returns the smoothed bytes-per-second rate.
    ///
    /// Windows shorter than 100 ms are ignored, otherwise a burst of
    /// percentage-triggered events would produce noisy spikes.
    fn update(&mut self, bytes: u64, now: Instant) -> Option<u64> {
        let elapsed = now.saturating_duration_since(self.last_at).as_secs_f64();
        if elapsed < 0.1 {
            return if self.smoothed > 0.0 {
                Some(self.smoothed.round() as u64)
            } else {
                None
            };
        }

        let instant = bytes.saturating_sub(self.last_bytes) as f64 / elapsed;
        self.smoothed = if self.smoothed > 0.0 {
            self.smoothed * 0.6 + instant * 0.4
        } else {
            instant
        };
        self.last_bytes = bytes;
        self.last_at = now;
        Some(self.smoothed.round() as u64)
    }
}

/// Emit paused/resumed state changes and block while the user has paused.
async fn checkpoint(
    control: &DownloadControl,
    opts: &DownloadOptions<'_>,
    downloaded: u64,
    total: Option<u64>,
) -> Result<()> {
    let mut paused = false;
    loop {
        match control.state() {
            ControlState::Running => {
                if paused {
                    emit_download_progress(
                        opts,
                        downloaded,
                        total,
                        compute_percent_0_99(downloaded, total),
                        None,
                        "downloading",
                        "继续下载",
                    );
                }
                return Ok(());
            }
            ControlState::Cancelled => {
                emit_download_progress(
                    opts,
                    downloaded,
                    total,
                    compute_percent_0_99(downloaded, total),
                    Some(0),
                    "cancelled",
                    "下载已取消",
                );
                return Err(AppError::download_cancelled());
            }
            ControlState::Paused => {
                if !paused {
                    emit_download_progress(
                        opts,
                        downloaded,
                        total,
                        compute_percent_0_99(downloaded, total),
                        Some(0),
                        "paused",
                        "已暂停",
                    );
                    paused = true;
                }
                control.wait_for_change().await;
            }
        }
    }
}

fn compute_percent_0_99(downloaded: u64, total: Option<u64>) -> Option<u8> {
    let t = total?;
    if t == 0 {
        return Some(0);
    }
    let p = (downloaded.saturating_mul(99)).saturating_div(t);
    Some(p.min(99) as u8)
}

/// Download a file from `url` and stream it to `dest`.
///
/// On failure the partially-written file is removed so callers never see a
/// truncated / corrupt artifact.
pub async fn download_file(
    client: &Client,
    url: &str,
    dest: &Path,
    opts: Option<&DownloadOptions<'_>>,
) -> Result<()> {
    log::debug!("Downloading {} -> {:?}", url, dest);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(e.to_string()))?;
    }

    let result = download_file_inner(client, url, dest, opts).await;

    if result.is_err() {
        let _ = fs::remove_file(dest);
    }

    result
}

pub async fn download_file_with_fallbacks(
    client: &Client,
    urls: &[String],
    dest: &Path,
    opts: Option<&DownloadOptions<'_>>,
) -> Result<()> {
    let mut last_error = None;

    for (index, url) in urls.iter().enumerate() {
        if index > 0 {
            log::warn!("Retrying download with fallback URL: {}", url);
            if let Some(o) = opts {
                emit_download_progress(
                    o,
                    0,
                    None,
                    None,
                    None,
                    "downloading",
                    "加速下载失败，正在回退直连",
                );
            }
        }

        match download_file(client, url, dest, opts).await {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == ErrorKind::DownloadCancelled => return Err(error),
            Err(error) => {
                log::warn!("Download failed for {}: {}", url, error);
                last_error = Some(error);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| AppError::network("没有可用的下载地址".to_string())))
}

async fn download_file_inner(
    client: &Client,
    url: &str,
    dest: &Path,
    opts: Option<&DownloadOptions<'_>>,
) -> Result<()> {
    let resp = send_get(client, url, false)
        .await
        .map_err(|e| AppError::network(e.to_string()))?;
    ensure_success_status(&resp, AppError::network)?;

    let total = resp.content_length();
    let mut downloaded: u64 = 0;
    let mut last_emit = Instant::now();
    let mut last_percent: u8 = 0;
    let mut meter = SpeedMeter::new();

    if let Some(o) = opts {
        emit_download_progress(
            o,
            0,
            total,
            compute_percent_0_99(0, total),
            None,
            "downloading",
            "开始下载",
        );
    }

    let mut file = fs::File::create(dest).map_err(|e| AppError::io(e.to_string()))?;

    let mut stream = resp.bytes_stream();

    loop {
        if let Some(o) = opts {
            if let Some(control) = o.control {
                checkpoint(control, o, downloaded, total).await?;
            }
        }

        let next = match opts.and_then(|o| o.control) {
            Some(control) => tokio::select! {
                item = stream.next() => item,
                _ = control.wait_for_change() => continue,
            },
            None => stream.next().await,
        };

        let Some(chunk) = next else {
            break;
        };
        let chunk = chunk.map_err(|e| AppError::network(e.to_string()))?;
        file.write_all(&chunk)
            .map_err(|e| AppError::io(e.to_string()))?;

        downloaded += chunk.len() as u64;

        if let Some(o) = opts {
            let now = Instant::now();
            let current_percent = compute_percent_0_99(downloaded, total).unwrap_or(0);
            if now.duration_since(last_emit) >= Duration::from_millis(100)
                || current_percent > last_percent
            {
                emit_download_progress(
                    o,
                    downloaded,
                    total,
                    compute_percent_0_99(downloaded, total),
                    meter.update(downloaded, now),
                    "downloading",
                    "下载中",
                );
                last_emit = now;
                last_percent = current_percent;
            }
        }
    }

    if let Some(o) = opts {
        // Keep 100 reserved for the terminal "done" event.
        emit_download_progress(
            o,
            downloaded,
            total,
            compute_percent_0_99(downloaded, total).or(Some(99)),
            meter.update(downloaded, Instant::now()),
            "downloading",
            "下载完成",
        );
    }

    Ok(())
}

/// Download and register an AstrBot version archive.
pub async fn download_version(
    client: &Client,
    release: &GitHubRelease,
    app_handle: Option<&AppHandle>,
) -> Result<()> {
    let version = &release.tag_name;
    log::info!("Downloading version {}", version);
    // Register before touching the target file so a duplicate install cannot
    // delete the archive that the active download is writing.
    let guard = download_control::acquire(version)?;
    let versions_dir = get_versions_dir();
    let zip_path = resolve_version_zip_path(version)?;

    std::fs::create_dir_all(&versions_dir)
        .map_err(|e| AppError::io(format!("Failed to create versions dir: {}", e)))?;

    if zip_path.exists() {
        if let Err(e) = std::fs::remove_file(&zip_path) {
            log::warn!("Failed to remove old zip {:?}: {}", zip_path, e);
        }
    }

    let opts = app_handle.map(|ah| DownloadOptions {
        app_handle: ah,
        id: version,
        control: Some(guard.control()),
    });

    let core_archive_urls = get_source_archive_urls(version);
    download_file_with_fallbacks(client, &core_archive_urls, &zip_path, opts.as_ref()).await?;

    if let Some(o) = &opts {
        let size = std::fs::metadata(&zip_path).map(|m| m.len()).ok();
        emit_download_progress(
            o,
            size.unwrap_or(0),
            size,
            Some(100),
            None,
            "done",
            "下载完成",
        );
    }

    let zip_path_str = zip_path
        .to_str()
        .ok_or_else(|| {
            AppError::io(format!(
                "Version zip path is not valid UTF-8: {:?}",
                zip_path
            ))
        })?
        .to_string();

    let installed = InstalledVersion {
        version: version.to_string(),
        zip_path: zip_path_str,
    };

    let version_owned = version.to_string();
    with_manifest_mut(move |manifest| {
        manifest
            .installed_versions
            .retain(|v| v.version != version_owned.as_str());
        manifest.installed_versions.push(installed);
        Ok(())
    })?;

    Ok(())
}

/// Unregister and remove an AstrBot version archive.
pub fn remove_version(version: &str) -> Result<()> {
    log::info!("Removing version {}", version);
    let zip_path = resolve_version_zip_path(version)?;

    let version_owned = version.to_string();
    with_manifest_mut(|manifest| {
        for inst in manifest.instances.values() {
            if inst.version == version_owned.as_str() {
                return Err(AppError::version_in_use(&version_owned, &inst.name));
            }
        }

        manifest
            .installed_versions
            .retain(|v| v.version != version_owned.as_str());
        Ok(())
    })?;

    if zip_path.exists() {
        if let Err(e) = std::fs::remove_file(&zip_path) {
            log::warn!("Failed to remove zip {:?}: {}", zip_path, e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meter_at(base: Instant) -> SpeedMeter {
        SpeedMeter {
            last_bytes: 0,
            last_at: base,
            smoothed: 0.0,
        }
    }

    #[test]
    fn speed_meter_smooths_samples() {
        let base = Instant::now();
        let mut meter = meter_at(base);

        assert_eq!(
            meter.update(1_000_000, base + Duration::from_secs(1)),
            Some(1_000_000)
        );
        assert_eq!(
            meter.update(3_000_000, base + Duration::from_secs(2)),
            Some(1_400_000)
        );
    }

    #[test]
    fn speed_meter_ignores_short_windows() {
        let base = Instant::now();
        let mut meter = meter_at(base);

        assert_eq!(meter.update(100, base + Duration::from_millis(10)), None);
        assert_eq!(
            meter.update(1_000_000, base + Duration::from_secs(1)),
            Some(1_000_000)
        );
    }

    #[test]
    fn percent_reserves_hundred_for_terminal_event() {
        assert_eq!(compute_percent_0_99(0, None), None);
        assert_eq!(compute_percent_0_99(0, Some(100)), Some(0));
        assert_eq!(compute_percent_0_99(50, Some(100)), Some(49));
        assert_eq!(compute_percent_0_99(100, Some(100)), Some(99));
    }
}
