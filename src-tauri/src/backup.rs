use std::ffi::OsStr;
use std::fs::{self, File};
use std::io;
use std::io::Read as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use flate2::read::GzDecoder;
use once_cell::sync::Lazy;
use tar::Archive;
use walkdir::WalkDir;

use crate::archive::{extract_tar_gz_mapped, extract_zip_mapped, validate_zip_archive};
use crate::config::{load_manifest, with_manifest_mut, BackupInfo, BackupMetadata, InstanceConfig};
use crate::error::{AppError, Result};
use crate::utils::archive_path::parse_entry_rel_path;
use crate::utils::fs_ops;
use crate::utils::log_redact;
use crate::utils::paths::{get_backups_dir, get_instance_core_dir, get_instance_dir};
use crate::utils::sync::lock_mutex_recover;
use crate::utils::validation::validate_instance_id;
use crate::validation::resolve_backup_path;

const RESTORE_STAGING_PREFIX: &str = ".restore-staging-";
const RESTORE_ROLLBACK_PREFIX: &str = ".restore-rollback-";
/// Longest accepted backup note, counted in characters rather than bytes.
const BACKUP_NOTE_MAX_CHARS: usize = 200;

static BACKUP_ARCHIVE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// Check if a backup path is in tar.gz format.
fn is_tar_gz(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.ends_with(".tar.gz"))
        .unwrap_or(false)
}

/// Trim a user-provided backup note and reject oversized input.
fn normalize_backup_note(note: Option<&str>) -> Result<String> {
    let trimmed = note.unwrap_or("").trim();
    if trimmed.chars().count() > BACKUP_NOTE_MAX_CHARS {
        return Err(AppError::backup(format!(
            "Backup note must not exceed {} characters",
            BACKUP_NOTE_MAX_CHARS
        )));
    }
    Ok(trimmed.to_string())
}

fn path_contains_pycache(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str() == OsStr::new("__pycache__"))
}

fn append_data_dir_to_zip_excluding_pycache<W: io::Write + io::Seek>(
    writer: &mut zip::ZipWriter<W>,
    data_dir: &Path,
    options: zip::write::SimpleFileOptions,
) -> Result<()> {
    let mut iter = WalkDir::new(data_dir).into_iter();
    while let Some(entry) = iter.next() {
        let entry = entry.map_err(|e| AppError::io(e.to_string()))?;
        let path = entry.path();

        if entry.file_type().is_dir() && entry.file_name() == OsStr::new("__pycache__") {
            iter.skip_current_dir();
            continue;
        }

        let relative = path
            .strip_prefix(data_dir)
            .map_err(|e| AppError::io(e.to_string()))?;
        // Use forward slashes in archive paths per ZIP spec, regardless of OS.
        let rel_parts: Vec<_> = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect();
        let archive_path = format!("data/{}", rel_parts.join("/"));

        if entry.file_type().is_dir() {
            if path != data_dir {
                writer
                    .add_directory(&archive_path, options)
                    .map_err(|e| AppError::io(e.to_string()))?;
            }
            continue;
        }

        if entry.file_type().is_file() {
            writer
                .start_file(&archive_path, options)
                .map_err(|e| AppError::io(e.to_string()))?;
            let mut f = fs::File::open(path).map_err(|e| AppError::io(e.to_string()))?;
            io::copy(&mut f, writer).map_err(|e| AppError::io(e.to_string()))?;
        }
    }

    Ok(())
}

fn backup_display_name(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("backup")
}

/// Pick a backup filename that does not overwrite an existing archive.
///
/// Timestamps only have second resolution, so repeated backups within the same
/// second get a numeric suffix. Callers must hold `BACKUP_ARCHIVE_LOCK` while
/// checking and renaming to keep the selection race-free within the process.
fn unique_backup_path(backups_dir: &Path, stem: &str) -> Result<PathBuf> {
    let base = backups_dir.join(format!("{stem}.zip"));
    if !base.exists() {
        return Ok(base);
    }

    for suffix in 1..=9999_u32 {
        let candidate = backups_dir.join(format!("{stem}-{suffix}.zip"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(AppError::backup(
        "Too many backups share the same timestamp, please retry",
    ))
}

fn write_backup_archive(file: File, metadata: &BackupMetadata, data_dir: &Path) -> Result<()> {
    let mut writer = zip::ZipWriter::new(file);

    let metadata_toml = toml::to_string_pretty(metadata)
        .map_err(|e| AppError::backup(format!("Failed to serialize metadata: {}", e)))?;
    let options = zip::write::SimpleFileOptions::default();
    writer
        .start_file("backup.toml", options)
        .map_err(|e| AppError::backup(format!("Failed to add metadata: {}", e)))?;
    writer
        .write_all(metadata_toml.as_bytes())
        .map_err(|e| AppError::backup(format!("Failed to write metadata: {}", e)))?;

    if data_dir.exists() {
        append_data_dir_to_zip_excluding_pycache(&mut writer, data_dir, options)
            .map_err(|e| AppError::backup(format!("Failed to add data dir: {}", e)))?;
    }

    let file = writer
        .finish()
        .map_err(|e| AppError::backup(format!("Failed to finalize backup archive: {}", e)))?;
    file.sync_all()
        .map_err(|e| AppError::backup(format!("Failed to flush backup archive: {}", e)))?;
    Ok(())
}

/// Create a backup archive in a caller-provided directory.
///
/// The zip is written to a unique temporary sibling, validated, and then
/// renamed to a non-conflicting `.zip` name. A failed write or validation
/// removes only the temporary file.
fn create_backup_archive_at(
    instance: &InstanceConfig,
    instance_id: &str,
    auto_generated: bool,
    note: &str,
    backups_dir: &Path,
    data_dir: &Path,
) -> Result<PathBuf> {
    fs::create_dir_all(backups_dir)
        .map_err(|e| AppError::backup(format!("Failed to create backups dir: {}", e)))?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let stem = if auto_generated {
        format!("{}-{}-auto", instance_id, timestamp)
    } else {
        format!("{}-{}", instance_id, timestamp)
    };

    let metadata = BackupMetadata {
        created_at: chrono::Utc::now().to_rfc3339(),
        instance_name: instance.name.clone(),
        instance_id: instance_id.to_string(),
        version: instance.version.clone(),
        includes_venv: false,
        includes_data: true,
        arch_target: String::new(),
        auto_generated,
        note: note.to_string(),
    };

    let (temp_path, file) =
        fs_ops::create_unique_sibling_file(&backups_dir.join(format!("{stem}.zip")), "part")?;

    if let Err(error) = write_backup_archive(file, &metadata, data_dir) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    if let Err(error) = validate_zip_archive(&temp_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(AppError::backup(format!(
            "Backup archive validation failed: {}",
            error
        )));
    }

    let _lock = lock_mutex_recover(&BACKUP_ARCHIVE_LOCK, "BACKUP_ARCHIVE_LOCK");
    let backup_path = match unique_backup_path(backups_dir, &stem) {
        Ok(path) => path,
        Err(error) => {
            let _ = fs::remove_file(&temp_path);
            return Err(error);
        }
    };
    if let Err(error) = fs::rename(&temp_path, &backup_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(AppError::backup(format!(
            "Failed to move backup archive into place: {}",
            error
        )));
    }

    Ok(backup_path)
}

/// Common backup creation logic.
fn create_backup_archive(
    instance: &InstanceConfig,
    instance_id: &str,
    auto_generated: bool,
    note: &str,
) -> Result<String> {
    let backups_dir = get_backups_dir();
    let data_dir = get_instance_core_dir(instance_id).join("data");
    let backup_path = create_backup_archive_at(
        instance,
        instance_id,
        auto_generated,
        note,
        &backups_dir,
        &data_dir,
    )?;

    backup_path
        .to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::io("backup path is not valid UTF-8"))
}

/// Create a backup of an instance.
///
/// When `auto_generated` is `true`, the backup is tagged in metadata so UI can
/// show it as an auto-generated backup. `note` is an optional user annotation
/// stored in the archive metadata.
pub fn create_backup(
    instance_id: &str,
    auto_generated: bool,
    note: Option<&str>,
) -> Result<String> {
    log::info!("Creating backup for instance {}", instance_id);
    let note = normalize_backup_note(note)?;
    let manifest = load_manifest()?;
    let instance = manifest
        .instances
        .get(instance_id)
        .ok_or_else(|| AppError::instance_not_found(instance_id))?;

    let data_dir = get_instance_core_dir(instance_id).join("data");
    if !data_dir.exists() {
        return Err(AppError::backup("No data directory to back up"));
    }

    create_backup_archive(instance, instance_id, auto_generated, &note)
}

/// List all backups
pub fn list_backups() -> Result<Vec<BackupInfo>> {
    let backups_dir = get_backups_dir();
    if !backups_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();

    for entry in fs::read_dir(&backups_dir)
        .map_err(|e| AppError::backup(format!("Failed to read backups dir: {}", e)))?
    {
        let entry = entry.map_err(|e| AppError::backup(e.to_string()))?;
        let path = entry.path();

        let fname = match path.file_name().and_then(|n| n.to_str()) {
            Some(s) => s.to_string(),
            None => {
                log::warn!(
                    "Skipping backup with non-UTF-8 filename: {}",
                    log_redact::path_name(&path)
                );
                continue;
            }
        };

        // Accept both .tar.gz and .zip
        if !(fname.ends_with(".tar.gz") || fname.ends_with(".zip")) {
            continue;
        }

        let path_str = match path.to_str() {
            Some(s) => s.to_string(),
            None => {
                log::warn!(
                    "Skipping backup with non-UTF-8 path: {}",
                    log_redact::path_name(&path)
                );
                continue;
            }
        };

        match read_backup_metadata(&path) {
            Ok(metadata) => {
                backups.push(BackupInfo {
                    filename: fname,
                    path: path_str,
                    metadata,
                    corrupted: false,
                    parse_error: None,
                });
            }
            Err(err) => {
                log::warn!(
                    "Backup metadata parse failed for {}: {}",
                    log_redact::path_name(&path),
                    err
                );
                backups.push(BackupInfo {
                    filename: fname.clone(),
                    path: path_str,
                    metadata: BackupMetadata {
                        created_at: String::new(),
                        instance_name: "（损坏备份）".to_string(),
                        instance_id: String::new(),
                        version: String::new(),
                        includes_venv: false,
                        includes_data: false,
                        arch_target: String::new(),
                        auto_generated: false,
                        note: String::new(),
                    },
                    corrupted: true,
                    parse_error: Some(err.to_string()),
                });
            }
        }
    }

    // Sort by created_at descending
    backups.sort_by(|a, b| b.metadata.created_at.cmp(&a.metadata.created_at));

    Ok(backups)
}

/// Find the latest auto-generated backup for the specified instance, if any.
///
/// This is used to detect unfinished upgrade/downgrade recovery flows.
pub fn find_pending_auto_backup(instance_id: &str) -> Result<Option<BackupInfo>> {
    validate_instance_id(instance_id)?;

    let backups_dir = get_backups_dir();
    if !backups_dir.exists() {
        return Ok(None);
    }

    let filename_prefix = format!("{instance_id}-");
    let mut latest: Option<(BackupInfo, std::time::SystemTime)> = None;

    for entry in fs::read_dir(&backups_dir)
        .map_err(|e| AppError::backup(format!("Failed to read backups dir: {}", e)))?
    {
        let entry = entry.map_err(|e| AppError::backup(e.to_string()))?;
        let path = entry.path();

        let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
            log::warn!(
                "Skipping backup with non-UTF-8 filename: {}",
                log_redact::path_name(&path)
            );
            continue;
        };
        let is_supported = filename.ends_with(".zip") || filename.ends_with(".tar.gz");
        let is_auto_candidate =
            filename.starts_with(&filename_prefix) && filename.contains("-auto");
        if !is_supported || !is_auto_candidate {
            continue;
        }

        let Some(mtime) = fs::metadata(&path)
            .ok()
            .and_then(|metadata| metadata.modified().ok())
        else {
            continue;
        };

        let metadata = match read_backup_metadata(&path) {
            Ok(metadata) => metadata,
            Err(err) => {
                log::warn!(
                    "Backup metadata parse failed for {}: {}",
                    log_redact::path_name(&path),
                    err
                );
                continue;
            }
        };
        if !metadata.auto_generated || metadata.instance_id != instance_id {
            continue;
        }

        let Some(path_str) = path.to_str() else {
            log::warn!(
                "Skipping backup with non-UTF-8 path: {}",
                log_redact::path_name(&path)
            );
            continue;
        };

        let info = BackupInfo {
            filename: filename.to_string(),
            path: path_str.to_string(),
            metadata,
            corrupted: false,
            parse_error: None,
        };

        match &latest {
            None => latest = Some((info, mtime)),
            Some((current, current_mtime)) => {
                let is_newer = mtime
                    .cmp(current_mtime)
                    .then_with(|| info.metadata.created_at.cmp(&current.metadata.created_at))
                    .is_gt();
                if is_newer {
                    latest = Some((info, mtime));
                }
            }
        }
    }

    Ok(latest.map(|(backup, _)| backup))
}

fn read_backup_metadata(backup_path: &Path) -> Result<BackupMetadata> {
    if is_tar_gz(backup_path) {
        read_backup_metadata_tar_gz(backup_path)
    } else {
        read_backup_metadata_zip(backup_path)
    }
}

fn read_backup_metadata_tar_gz(backup_path: &Path) -> Result<BackupMetadata> {
    let file = File::open(backup_path)
        .map_err(|e| AppError::backup(format!("Failed to open backup: {}", e)))?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    for entry in archive
        .entries()
        .map_err(|e| AppError::backup(e.to_string()))?
    {
        let mut entry = entry.map_err(|e| AppError::backup(e.to_string()))?;
        let path = entry.path().map_err(|e| AppError::backup(e.to_string()))?;

        if path.to_str() == Some("backup.toml") {
            let mut content = String::new();
            entry
                .read_to_string(&mut content)
                .map_err(|e| AppError::backup(e.to_string()))?;
            return toml::from_str(&content)
                .map_err(|e| AppError::backup(format!("Failed to parse metadata: {}", e)));
        }
    }

    Err(AppError::backup("backup.toml not found in tar.gz backup"))
}

fn read_backup_metadata_zip(backup_path: &Path) -> Result<BackupMetadata> {
    let file = File::open(backup_path)
        .map_err(|e| AppError::backup(format!("Failed to open backup: {}", e)))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::backup(format!("Failed to read zip: {}", e)))?;

    let mut entry = archive
        .by_name("backup.toml")
        .map_err(|e| AppError::backup(format!("backup.toml not found: {}", e)))?;

    let mut content = String::new();
    entry
        .read_to_string(&mut content)
        .map_err(|e| AppError::backup(e.to_string()))?;

    toml::from_str(&content)
        .map_err(|e| AppError::backup(format!("Failed to parse metadata: {}", e)))
}

fn cleanup_auto_backup(backup_path: &Path, operation_name: &str) {
    if let Err(e) = fs::remove_file(backup_path) {
        log::warn!(
            "{} succeeded, but failed to delete auto-generated backup {}: {}",
            operation_name,
            log_redact::path_name(backup_path),
            e
        );
    } else {
        log::info!(
            "Deleted auto-generated backup after successful {}: {}",
            operation_name,
            log_redact::path_name(backup_path)
        );
    }
}

pub fn resolve_restore_backup_input(backup_path: &str) -> Result<(PathBuf, BackupMetadata)> {
    log::info!(
        "Restoring backup from {}",
        log_redact::path_name(Path::new(backup_path))
    );
    let backup_path = resolve_backup_path(backup_path, true)?;

    // Read metadata
    let metadata = read_backup_metadata(&backup_path)?;
    Ok((backup_path, metadata))
}

/// Route an archive entry to the correct destination directory.
fn route_backup_entry(relative: &Path, destination_root: &Path) -> Option<PathBuf> {
    // Skip backup.toml
    if relative == Path::new("backup.toml") {
        return None;
    }

    if relative.starts_with("data") {
        if path_contains_pycache(relative) {
            return None;
        }
        return Some(destination_root.join(relative));
    }
    if relative.starts_with("venv") {
        return None;
    }

    log::warn!(
        "Ignoring unsupported backup entry during restore: {}",
        relative.display()
    );
    None
}

fn restore_safety_backup_needed(data_dir: &Path) -> bool {
    data_dir.is_dir()
}

fn safety_backup_note(path: Option<&str>) -> String {
    match path
        .and_then(|value| Path::new(value).file_name())
        .and_then(|name| name.to_str())
    {
        Some(name) => format!("恢复前安全备份：{}。", name),
        None => "实例原本没有数据目录，未创建恢复前安全备份。".to_string(),
    }
}

fn remove_dir_if_exists(path: &Path, label: &str) {
    match fs::remove_dir_all(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            log::warn!("Failed to remove {}: {}", label, error);
        }
    }
}

/// Extracts a backup's `data/` entries into a staging directory.
fn extract_backup_data_to(backup_path: &Path, staging_root: &Path) -> Result<()> {
    let routing = |raw_path: &str| -> Option<PathBuf> {
        let relative = parse_entry_rel_path(raw_path)?;
        route_backup_entry(&relative, staging_root)
    };

    if is_tar_gz(backup_path) {
        extract_tar_gz_mapped(backup_path, staging_root, routing)
    } else {
        extract_zip_mapped(backup_path, staging_root, routing)
    }
    .map_err(|e| AppError::backup(format!("Failed to extract backup: {}", e)))
}

/// Owns the directory swap performed while restoring instance data.
///
/// `rollback` puts the previous `core/data` back; `commit` discards it.
struct DataSwap {
    core_data_dir: PathBuf,
    staging_root: PathBuf,
    rollback_dir: PathBuf,
    had_previous_data: bool,
    installed: bool,
}

impl DataSwap {
    fn install(staging_root: PathBuf, core_dir: &Path, rollback_dir: PathBuf) -> Result<Self> {
        let staged_data_dir = staging_root.join("data");
        let core_data_dir = core_dir.join("data");
        let mut swap = Self {
            core_data_dir: core_data_dir.clone(),
            staging_root,
            rollback_dir,
            had_previous_data: false,
            installed: false,
        };

        match fs::symlink_metadata(&staged_data_dir) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => {
                return Err(AppError::backup(
                    "备份的 data 根路径不是普通目录，已拒绝恢复",
                ));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                // The archive contains no data entries; preserve the current
                // data instead of replacing it with nothing.
                return Ok(swap);
            }
            Err(error) => {
                return Err(AppError::backup(format!(
                    "Failed to inspect staged data directory: {}",
                    error
                )));
            }
        }

        fs::create_dir_all(core_dir).map_err(|e| {
            AppError::backup(format!("Failed to create instance core directory: {}", e))
        })?;

        if core_data_dir.exists() {
            fs::rename(&core_data_dir, &swap.rollback_dir).map_err(|e| {
                AppError::backup(format!("Failed to move existing data aside: {}", e))
            })?;
            swap.had_previous_data = true;
        }

        match fs::rename(&staged_data_dir, &core_data_dir) {
            Ok(()) => {
                swap.installed = true;
                Ok(swap)
            }
            Err(error) => {
                if swap.had_previous_data {
                    if let Err(rollback_error) = fs::rename(&swap.rollback_dir, &core_data_dir) {
                        return Err(AppError::backup(format!(
                            "Failed to install restored data ({}), and restoring the previous data also failed ({}). The previous data is kept in a temporary instance directory",
                            error, rollback_error
                        )));
                    }
                    swap.had_previous_data = false;
                }
                let _ = fs::remove_dir_all(&swap.staging_root);
                Err(AppError::backup(format!(
                    "Failed to install restored data: {}",
                    error
                )))
            }
        }
    }

    fn commit(&mut self) {
        if self.had_previous_data {
            remove_dir_if_exists(&self.rollback_dir, "previous instance data");
            self.had_previous_data = false;
        }
        remove_dir_if_exists(&self.staging_root, "restore staging directory");
        self.installed = false;
    }

    fn rollback(&mut self) -> Result<()> {
        if !self.installed {
            remove_dir_if_exists(&self.staging_root, "restore staging directory");
            return Ok(());
        }

        let displaced = self.staging_root.join("restored-data");
        if self.had_previous_data {
            fs::rename(&self.core_data_dir, &displaced).map_err(|error| {
                AppError::backup(format!(
                    "Failed to move restored data aside during rollback: {}",
                    error
                ))
            })?;

            match fs::rename(&self.rollback_dir, &self.core_data_dir) {
                Ok(()) => {
                    self.had_previous_data = false;
                    self.installed = false;
                    remove_dir_if_exists(&self.staging_root, "restore staging directory");
                    Ok(())
                }
                Err(rollback_error) => {
                    let restore_error = fs::rename(&displaced, &self.core_data_dir).err();
                    let detail = match restore_error {
                        Some(error) => format!(
                            "Failed to restore previous data from the temporary directory ({}), and the restored data could not be moved back either ({}). Both directories are kept",
                            rollback_error, error
                        ),
                        None => format!(
                            "Failed to restore previous data from the temporary directory ({}). The restored data was moved back into place",
                            rollback_error
                        ),
                    };
                    Err(AppError::backup(detail))
                }
            }
        } else {
            fs::rename(&self.core_data_dir, &displaced).map_err(|error| {
                AppError::backup(format!(
                    "Failed to remove restored data during rollback: {}",
                    error
                ))
            })?;
            self.installed = false;
            remove_dir_if_exists(&self.staging_root, "restore staging directory");
            Ok(())
        }
    }
}

fn remove_stale_restore_staging_dirs(instance_dir: &Path) {
    let entries = match fs::read_dir(instance_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return,
        Err(error) => {
            log::warn!(
                "Failed to read instance directory before restore: {}",
                error
            );
            return;
        }
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if name.starts_with(RESTORE_STAGING_PREFIX) {
            remove_dir_if_exists(&entry.path(), "stale restore staging directory");
        }
    }
}

/// Extract a backup into a unique staging directory and swap it into place.
fn prepare_data_swap(backup_path: &Path, instance_dir: &Path, core_dir: &Path) -> Result<DataSwap> {
    remove_stale_restore_staging_dirs(instance_dir);

    let staging_root =
        instance_dir.join(format!("{RESTORE_STAGING_PREFIX}{}", uuid::Uuid::new_v4()));
    if let Err(error) = extract_backup_data_to(backup_path, &staging_root) {
        let _ = fs::remove_dir_all(&staging_root);
        return Err(error);
    }

    let rollback_dir =
        instance_dir.join(format!("{RESTORE_ROLLBACK_PREFIX}{}", uuid::Uuid::new_v4()));
    match DataSwap::install(staging_root.clone(), core_dir, rollback_dir) {
        Ok(swap) => Ok(swap),
        Err(error) => {
            let _ = fs::remove_dir_all(&staging_root);
            Err(error)
        }
    }
}

/// Restore backup.
pub fn restore_backup_with_input(backup_path: PathBuf, metadata: BackupMetadata) -> Result<()> {
    let instance_id = metadata.instance_id.clone();
    validate_instance_id(&instance_id)?;

    // Check if version is installed
    let manifest = load_manifest()?;
    if !manifest
        .installed_versions
        .iter()
        .any(|v| v.version == metadata.version)
    {
        return Err(AppError::version_not_found(&metadata.version));
    }

    // Validate original instance still exists
    let instance = manifest
        .instances
        .get(&instance_id)
        .cloned()
        .ok_or_else(|| AppError::instance_not_found(&instance_id))?;

    let instance_dir = get_instance_dir(&instance_id);
    let core_dir = get_instance_core_dir(&instance_id);
    let data_dir = core_dir.join("data");

    // A failed safety backup aborts the restore before any instance data is
    // touched. It is a normal backup so startup will not treat it as a pending
    // auto-generated upgrade backup.
    let safety_backup = if restore_safety_backup_needed(&data_dir) {
        let path = create_backup_archive(&instance, &instance_id, false, "").map_err(|error| {
            AppError::backup(format!("恢复前安全备份失败，已中止恢复：{}", error))
        })?;
        log::info!(
            "Created pre-restore safety backup {}",
            backup_display_name(&path)
        );
        Some(path)
    } else {
        None
    };

    let mut swap = prepare_data_swap(&backup_path, &instance_dir, &core_dir)?;

    let update_result = with_manifest_mut(|manifest| {
        let instance = manifest
            .instances
            .get_mut(&instance_id)
            .ok_or_else(|| AppError::instance_not_found(&instance_id))?;
        instance.version = metadata.version.clone();
        Ok(())
    });

    if let Err(error) = update_result {
        let rollback_result = swap.rollback();
        let safety_note = safety_backup_note(safety_backup.as_deref());
        return match rollback_result {
            Ok(()) => Err(AppError::backup(format!(
                "恢复后更新实例版本失败，已回滚数据：{}。{}",
                error, safety_note
            ))),
            Err(rollback_error) => Err(AppError::backup(format!(
                "恢复后更新实例版本失败（{}），且回滚数据失败（{}）。{}",
                error, rollback_error, safety_note
            ))),
        };
    }

    swap.commit();

    if metadata.auto_generated {
        cleanup_auto_backup(&backup_path, "restore");
    }

    Ok(())
}

/// Delete a backup
pub fn delete_backup(backup_path: &str) -> Result<()> {
    log::info!(
        "Deleting backup {}",
        log_redact::path_name(Path::new(backup_path))
    );
    let path = resolve_backup_path(backup_path, false)?;
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| AppError::backup(format!("Failed to delete backup: {}", e)))?;
    }
    Ok(())
}

/// Restore only data from a backup to an existing instance.
pub fn restore_data_to_instance(backup_path: &str, instance_id: &str) -> Result<()> {
    validate_instance_id(instance_id)?;

    let backup_path = resolve_backup_path(backup_path, true)?;
    let metadata = read_backup_metadata(&backup_path)?;
    if metadata.instance_id != instance_id {
        return Err(AppError::backup("备份与目标实例不匹配，已中止恢复"));
    }
    let instance_dir = get_instance_dir(instance_id);
    let core_dir = get_instance_core_dir(instance_id);

    // The version update flow creates an auto-generated backup before clearing
    // the instance, so that backup is the recovery point for this data restore.
    let mut swap = prepare_data_swap(&backup_path, &instance_dir, &core_dir)?;
    swap.commit();

    if metadata.auto_generated {
        cleanup_auto_backup(&backup_path, "data restore");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(label: &str) -> Result<Self> {
            let path = std::env::temp_dir().join(format!(
                "astrbot-launcher-backup-{}-{}",
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

    fn write_file(path: &Path, content: &[u8]) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    fn test_instance() -> InstanceConfig {
        InstanceConfig {
            name: "Test instance".to_string(),
            version: "4.0.0".to_string(),
            host: "127.0.0.1".to_string(),
            port: 6185,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            check_update_enabled: true,
            resource_alert_cpu_percent: None,
            resource_alert_memory_bytes: None,
            resource_alert_disk_bytes: None,
            resource_alert_enabled: true,
            auto_restart_enabled: false,
            auto_restart_max_retries: 3,
            auto_restart_notify: true,
        }
    }

    fn write_backup_zip(path: &Path, entries: &[(&str, &[u8])]) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (name, data) in entries {
            writer.start_file(*name, options)?;
            writer.write_all(data)?;
        }
        let file = writer.finish()?;
        file.sync_all()?;
        Ok(())
    }

    #[test]
    fn backup_filenames_use_unique_suffix_on_conflict() -> Result<()> {
        let dir = TestDir::new("conflict")?;
        write_file(&dir.path().join("inst1-20260101_010101.zip"), b"first")?;
        write_file(&dir.path().join("inst1-20260101_010101-1.zip"), b"second")?;

        let path = unique_backup_path(dir.path(), "inst1-20260101_010101")?;

        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("inst1-20260101_010101-2.zip")
        );
        Ok(())
    }

    #[test]
    fn backup_archive_is_valid_and_keeps_metadata() -> Result<()> {
        let dir = TestDir::new("metadata")?;
        let backups_dir = dir.path().join("backups");
        let data_dir = dir.path().join("core").join("data");
        write_file(&data_dir.join("config.json"), b"{}")?;

        let path = create_backup_archive_at(
            &test_instance(),
            "inst1",
            false,
            "手动备份",
            &backups_dir,
            &data_dir,
        )?;

        assert!(path.exists());
        validate_zip_archive(&path)?;

        let file = File::open(&path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let mut entry = archive.by_name("backup.toml")?;
        let mut content = String::new();
        entry.read_to_string(&mut content)?;
        let metadata: BackupMetadata = toml::from_str(&content)?;

        assert_eq!(metadata.instance_id, "inst1");
        assert_eq!(metadata.instance_name, "Test instance");
        assert_eq!(metadata.version, "4.0.0");
        assert!(metadata.includes_data);
        assert!(!metadata.auto_generated);
        assert_eq!(metadata.note, "手动备份");
        assert!(!metadata.created_at.is_empty());
        Ok(())
    }

    #[test]
    fn backup_note_is_trimmed_and_length_limited() -> Result<()> {
        assert_eq!(normalize_backup_note(None)?, "");
        assert_eq!(normalize_backup_note(Some("  手动备份 \n"))?, "手动备份");

        let at_limit = "备".repeat(BACKUP_NOTE_MAX_CHARS);
        assert_eq!(
            normalize_backup_note(Some(&at_limit))?,
            at_limit,
            "multi-byte notes should be counted by characters"
        );

        let too_long = "备".repeat(BACKUP_NOTE_MAX_CHARS + 1);
        assert!(normalize_backup_note(Some(&too_long)).is_err());
        Ok(())
    }

    #[test]
    fn prepare_data_swap_rejects_path_escape_and_keeps_current_data() -> Result<()> {
        let dir = TestDir::new("prepare-escape")?;
        let instance_dir = dir.path().join("instance");
        let core_dir = instance_dir.join("core");
        write_file(&core_dir.join("data").join("config.json"), b"old")?;

        let backup_path = dir.path().join("backup.zip");
        write_backup_zip(
            &backup_path,
            &[("data/config.json", b"new"), ("../escape.txt", b"escaped")],
        )?;

        let result = prepare_data_swap(&backup_path, &instance_dir, &core_dir);
        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(core_dir.join("data").join("config.json"))?,
            "old"
        );
        assert!(!instance_dir.join("escape.txt").exists());
        Ok(())
    }

    #[test]
    fn restore_safety_backup_is_skipped_without_a_data_directory() -> Result<()> {
        let dir = TestDir::new("safety-decision")?;
        let missing = dir.path().join("missing");
        assert!(!restore_safety_backup_needed(&missing));

        let file_path = dir.path().join("not-a-directory");
        write_file(&file_path, b"x")?;
        assert!(!restore_safety_backup_needed(&file_path));

        let data_dir = dir.path().join("data");
        fs::create_dir_all(&data_dir)?;
        assert!(restore_safety_backup_needed(&data_dir));
        Ok(())
    }

    #[test]
    fn staged_data_swap_restores_previous_data_on_rollback() -> Result<()> {
        let dir = TestDir::new("swap-rollback")?;
        let core_dir = dir.path().join("core");
        write_file(&core_dir.join("data").join("config.json"), b"old")?;

        let staging_root = dir.path().join(".restore-staging-test");
        write_file(&staging_root.join("data").join("config.json"), b"new")?;
        let rollback_dir = dir.path().join(".restore-rollback-test");

        let mut swap = DataSwap::install(staging_root.clone(), &core_dir, rollback_dir.clone())?;
        assert_eq!(
            fs::read_to_string(core_dir.join("data").join("config.json"))?,
            "new"
        );

        swap.rollback()?;

        assert_eq!(
            fs::read_to_string(core_dir.join("data").join("config.json"))?,
            "old"
        );
        assert!(!staging_root.exists());
        assert!(!rollback_dir.exists());
        Ok(())
    }

    #[test]
    fn staged_data_swap_commit_discards_previous_data() -> Result<()> {
        let dir = TestDir::new("swap-commit")?;
        let core_dir = dir.path().join("core");
        write_file(&core_dir.join("data").join("config.json"), b"old")?;

        let staging_root = dir.path().join(".restore-staging-test");
        write_file(&staging_root.join("data").join("config.json"), b"new")?;
        let rollback_dir = dir.path().join(".restore-rollback-test");

        let mut swap = DataSwap::install(staging_root.clone(), &core_dir, rollback_dir.clone())?;
        swap.commit();

        assert_eq!(
            fs::read_to_string(core_dir.join("data").join("config.json"))?,
            "new"
        );
        assert!(!staging_root.exists());
        assert!(!rollback_dir.exists());
        Ok(())
    }

    #[test]
    fn staged_data_swap_rejects_non_directory_data_root() -> Result<()> {
        let dir = TestDir::new("swap-bad-root")?;
        let core_dir = dir.path().join("core");
        write_file(&core_dir.join("data").join("config.json"), b"old")?;

        let staging_root = dir.path().join(".restore-staging-test");
        write_file(&staging_root.join("data"), b"not a directory")?;
        let rollback_dir = dir.path().join(".restore-rollback-test");

        let result = DataSwap::install(staging_root, &core_dir, rollback_dir);
        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(core_dir.join("data").join("config.json"))?,
            "old"
        );
        Ok(())
    }

    #[test]
    fn staged_data_swap_keeps_current_data_when_archive_has_no_data() -> Result<()> {
        let dir = TestDir::new("swap-empty")?;
        let core_dir = dir.path().join("core");
        write_file(&core_dir.join("data").join("config.json"), b"old")?;

        let staging_root = dir.path().join(".restore-staging-test");
        fs::create_dir_all(&staging_root)?;
        let rollback_dir = dir.path().join(".restore-rollback-test");

        let mut swap = DataSwap::install(staging_root, &core_dir, rollback_dir)?;
        assert_eq!(
            fs::read_to_string(core_dir.join("data").join("config.json"))?,
            "old"
        );

        swap.commit();
        assert_eq!(
            fs::read_to_string(core_dir.join("data").join("config.json"))?,
            "old"
        );
        Ok(())
    }
}
