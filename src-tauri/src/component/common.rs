use std::fs;
use std::path::Path;

use reqwest::Client;
use tauri::AppHandle;

use crate::archive::{extract_tar_gz_flat, extract_zip_flat, ArchiveFormat};
use crate::download::{download_file, emit_download_progress, DownloadMessage, DownloadOptions};
use crate::error::{AppError, Result};
use crate::utils::fs_ops::unique_sibling_path;
use crate::utils::log_redact;

/// Download and install a component archive into `target_dir`.
///
/// The archive is downloaded and extracted into a unique staging sibling of
/// `target_dir`; only a fully extracted runtime is swapped in. A network or
/// extraction failure leaves any existing runtime untouched. `archive_path`
/// supplies the archive file name; the archive itself is downloaded inside
/// the staging directory and removed after extraction.
pub(super) async fn install_from_archive_with_progress(
    client: &Client,
    download_url: &str,
    target_dir: &Path,
    archive_path: &Path,
    archive_format: ArchiveFormat,
    progress_id: &'static str,
    app_handle: Option<&AppHandle>,
) -> Result<()> {
    let parent = target_dir.parent().ok_or_else(|| {
        AppError::io(format!(
            "Failed to resolve parent directory of {:?}",
            target_dir
        ))
    })?;
    fs::create_dir_all(parent).map_err(|e| {
        AppError::io(format!(
            "Failed to create component parent dir {:?}: {}",
            parent, e
        ))
    })?;

    let staging_dir = unique_sibling_path(target_dir, "staging")?;
    let backup_dir = unique_sibling_path(target_dir, "backup")?;
    let archive_file_name = archive_path
        .file_name()
        .ok_or_else(|| AppError::io(format!("Archive path {:?} has no file name", archive_path)))?;
    let staged_archive = staging_dir.join(archive_file_name);

    // Clean staging and backup leftovers from an interrupted previous run
    // and recover an orphaned backup before touching the target. Keep this
    // off snapshot probes (Windows locks in-use executables).
    {
        let _guard = super::COMPONENT_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        clear_stale_stage_dirs(parent, target_dir)?;
    }

    let opts = app_handle.map(|ah| DownloadOptions {
        app_handle: ah,
        id: progress_id,
        control: None,
        task_id: None,
    });

    if let Err(e) = download_file(client, download_url, &staged_archive, opts.as_ref()).await {
        remove_stage_dir(&staging_dir);
        return Err(e);
    }

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

    // Extraction must not overlap snapshot probes: a half-written exe the
    // probe launches would block file replacement on Windows.
    {
        let _guard = super::COMPONENT_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if let Err(e) = extract_staged_and_swap(
            &staged_archive,
            &staging_dir,
            target_dir,
            &backup_dir,
            archive_format,
        ) {
            remove_stage_dir(&staging_dir);
            return Err(e);
        }
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

/// Extract the staged archive and swap the staging directory over
/// `target_dir`.
fn extract_staged_and_swap(
    staged_archive: &Path,
    staging_dir: &Path,
    target_dir: &Path,
    backup_dir: &Path,
    archive_format: ArchiveFormat,
) -> Result<()> {
    match archive_format {
        ArchiveFormat::Zip => extract_zip_flat(staged_archive, staging_dir)?,
        ArchiveFormat::TarGz => extract_tar_gz_flat(staged_archive, staging_dir)?,
    }

    // The archive must not land in the final component directory.
    if let Err(e) = fs::remove_file(staged_archive) {
        log::warn!(
            "Failed to remove archive {}: {}",
            log_redact::path_name(staged_archive),
            e
        );
    }

    replace_component_dir(target_dir, staging_dir, backup_dir)
}

/// Move `staging_dir` over `target_dir`, preserving the previous runtime on
/// failure. Mirrors `instance::deploy::replace_webui_dist`.
fn replace_component_dir(target_dir: &Path, staging_dir: &Path, backup_dir: &Path) -> Result<()> {
    let backup_created = if target_dir.exists() {
        fs::rename(target_dir, backup_dir).map_err(|e| {
            AppError::io(format!(
                "Failed to move existing component dir {:?} to backup {:?}: {}",
                target_dir, backup_dir, e
            ))
        })?;
        true
    } else {
        false
    };

    match fs::rename(staging_dir, target_dir) {
        Ok(()) => {
            if backup_created {
                // The install succeeded; a stuck backup only wastes space and
                // is retried by the stale cleanup of the next install.
                if let Err(e) = fs::remove_dir_all(backup_dir) {
                    log::warn!(
                        "Failed to remove component backup {}: {}",
                        log_redact::path_name(backup_dir),
                        e
                    );
                }
            }
            Ok(())
        }
        Err(replace_error) => {
            if backup_created {
                if target_dir.exists() {
                    let _ = fs::remove_dir_all(target_dir);
                }
                if let Err(rollback_error) = fs::rename(backup_dir, target_dir) {
                    return Err(AppError::io(format!(
                        "Failed to replace component dir {:?} with {:?}: {}. Rollback from {:?} also failed: {}",
                        target_dir, staging_dir, replace_error, backup_dir, rollback_error
                    )));
                }
            }

            Err(AppError::io(format!(
                "Failed to replace component dir {:?} with {:?}: {}",
                target_dir, staging_dir, replace_error
            )))
        }
    }
}

/// Remove staging leftovers from an interrupted install and recover an
/// orphaned backup: a crash between the two swap renames leaves the
/// previous runtime only in the backup, so it is moved back into place.
fn clear_stale_stage_dirs(parent: &Path, target_dir: &Path) -> Result<()> {
    let name = target_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("component");
    let staging_prefix = format!(".{name}.staging-");
    let backup_prefix = format!(".{name}.backup-");

    let entries = match fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(AppError::io(format!(
                "Failed to read component parent dir {}: {}",
                log_redact::path_name(parent),
                e
            )))
        }
    };

    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };
        let path = entry.path();

        if file_name.starts_with(&staging_prefix) {
            remove_stage_dir(&path);
        } else if file_name.starts_with(&backup_prefix) {
            if target_dir.exists() {
                remove_stage_dir(&path);
            } else if let Err(e) = fs::rename(&path, target_dir) {
                log::warn!(
                    "Failed to restore component backup {} to {}: {}",
                    log_redact::path_name(&path),
                    log_redact::path_name(target_dir),
                    e
                );
            }
        }
    }

    Ok(())
}

fn remove_stage_dir(path: &Path) {
    if let Err(e) = fs::remove_dir_all(path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            log::warn!(
                "Failed to remove staging directory {}: {}",
                log_redact::path_name(path),
                e
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read as _;
    use std::path::PathBuf;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(label: &str) -> Result<Self> {
            let path = std::env::temp_dir().join(format!(
                "astrbot-launcher-component-{}-{}",
                label,
                uuid::Uuid::new_v4()
            ));
            fs::create_dir_all(&path)?;
            Ok(Self(path))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_tar_gz(path: &Path, entry_name: &str, data: &[u8]) -> Result<()> {
        let file = fs::File::create(path)?;
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append_data(&mut header, entry_name, data)?;
        builder.into_inner()?.finish()?;
        Ok(())
    }

    fn read_file(path: &Path) -> Result<Vec<u8>> {
        let mut content = Vec::new();
        fs::File::open(path)?.read_to_end(&mut content)?;
        Ok(content)
    }

    fn root_entries(root: &Path) -> Result<Vec<String>> {
        let mut names: Vec<String> = fs::read_dir(root)?
            .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
            .collect::<std::io::Result<_>>()?;
        names.sort();
        Ok(names)
    }

    #[tokio::test]
    async fn failed_download_keeps_existing_target() -> Result<()> {
        let root = TestDir::new("download-failure")?;
        let target = root.path().join("python");
        fs::create_dir_all(&target)?;
        fs::write(target.join("python.exe"), b"old-runtime")?;

        // Port 1 on localhost refuses connections without real network use.
        let client = Client::builder().no_proxy().build()?;
        let result = install_from_archive_with_progress(
            &client,
            "http://127.0.0.1:1/python.tar.gz",
            &target,
            &target.join("python.tar.gz"),
            ArchiveFormat::TarGz,
            "python",
            None,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            read_file(&target.join("python.exe"))?,
            b"old-runtime".to_vec()
        );
        assert_eq!(root_entries(root.path())?, vec!["python".to_string()]);
        Ok(())
    }

    #[test]
    fn failed_extraction_keeps_existing_target() -> Result<()> {
        let root = TestDir::new("extract-failure")?;
        let target = root.path().join("python");
        fs::create_dir_all(&target)?;
        fs::write(target.join("python.exe"), b"old-runtime")?;

        let staging = root.path().join(".python.staging-test");
        fs::create_dir_all(&staging)?;
        let staged_archive = staging.join("python.tar.gz");
        fs::write(&staged_archive, b"not a tar.gz")?;

        let result = extract_staged_and_swap(
            &staged_archive,
            &staging,
            &target,
            &root.path().join(".python.backup-test"),
            ArchiveFormat::TarGz,
        );

        assert!(result.is_err());
        assert_eq!(
            read_file(&target.join("python.exe"))?,
            b"old-runtime".to_vec()
        );
        assert_eq!(
            root_entries(root.path())?,
            vec![".python.staging-test".to_string(), "python".to_string()]
        );
        Ok(())
    }

    #[test]
    fn successful_extract_and_swap_replaces_target() -> Result<()> {
        let root = TestDir::new("swap-success")?;
        let target = root.path().join("uv");
        fs::create_dir_all(&target)?;
        fs::write(target.join("uv.exe"), b"old-runtime")?;

        let staging = root.path().join(".uv.staging-test");
        fs::create_dir_all(&staging)?;
        let staged_archive = staging.join("uv.tar.gz");
        write_tar_gz(&staged_archive, "runtime/uv.exe", b"new-runtime")?;

        extract_staged_and_swap(
            &staged_archive,
            &staging,
            &target,
            &root.path().join(".uv.backup-test"),
            ArchiveFormat::TarGz,
        )?;

        assert_eq!(read_file(&target.join("uv.exe"))?, b"new-runtime".to_vec());
        assert!(!staging.exists());
        assert!(!root.path().join(".uv.backup-test").exists());
        Ok(())
    }

    #[test]
    fn clear_stale_stage_dirs_recovers_orphaned_backup() -> Result<()> {
        let root = TestDir::new("stale-recovery")?;
        let target = root.path().join("nodejs");

        let staging = root.path().join(".nodejs.staging-stale");
        fs::create_dir_all(&staging)?;
        fs::write(staging.join("junk"), b"junk")?;

        // Simulate a crash between the two swap renames: the previous
        // runtime survives only in the backup.
        let backup = root.path().join(".nodejs.backup-stale");
        fs::create_dir_all(&backup)?;
        fs::write(backup.join("node.exe"), b"old-runtime")?;

        clear_stale_stage_dirs(root.path(), &target)?;

        assert!(!staging.exists());
        assert_eq!(
            read_file(&target.join("node.exe"))?,
            b"old-runtime".to_vec()
        );
        assert!(!backup.exists());

        // With the target present again, a leftover backup is just removed.
        let stale_backup = root.path().join(".nodejs.backup-extra");
        fs::create_dir_all(&stale_backup)?;
        clear_stale_stage_dirs(root.path(), &target)?;
        assert!(!stale_backup.exists());
        assert!(target.join("node.exe").exists());
        Ok(())
    }
}
