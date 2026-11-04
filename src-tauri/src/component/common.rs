use std::path::Path;

use reqwest::Client;
use tauri::AppHandle;

use crate::archive::{extract_tar_gz_flat, extract_zip_flat, ArchiveFormat};
use crate::download::{download_file, emit_download_progress, DownloadMessage, DownloadOptions};
use crate::error::{AppError, Result};
use crate::utils::log_redact;

pub(super) async fn install_from_archive_with_progress(
    client: &Client,
    download_url: &str,
    target_dir: &Path,
    archive_path: &Path,
    archive_format: ArchiveFormat,
    progress_id: &'static str,
    app_handle: Option<&AppHandle>,
) -> Result<()> {
    if target_dir.exists() {
        std::fs::remove_dir_all(target_dir).map_err(|e| {
            AppError::io(format!(
                "Failed to clean existing target dir {:?}: {}",
                target_dir, e
            ))
        })?;
    }
    std::fs::create_dir_all(target_dir).map_err(|e| {
        AppError::io(format!(
            "Failed to create target dir {:?}: {}",
            target_dir, e
        ))
    })?;

    let opts = app_handle.map(|ah| DownloadOptions {
        app_handle: ah,
        id: progress_id,
        control: None,
        task_id: None,
    });

    download_file(client, download_url, archive_path, opts.as_ref()).await?;

    if let Some(o) = &opts {
        emit_download_progress(
            o,
            0,
            None,
            Some(99),
            None,
            "extracting",
            DownloadMessage::Extracting,
        )?;
    }

    match archive_format {
        ArchiveFormat::Zip => extract_zip_flat(archive_path, target_dir)?,
        ArchiveFormat::TarGz => extract_tar_gz_flat(archive_path, target_dir)?,
    }

    if let Err(e) = std::fs::remove_file(archive_path) {
        log::warn!(
            "Failed to remove archive {}: {}",
            log_redact::path_name(archive_path),
            e
        );
    }

    if let Some(o) = &opts {
        emit_download_progress(
            o,
            0,
            None,
            Some(100),
            None,
            "done",
            DownloadMessage::Installed,
        )?;
    }

    Ok(())
}
