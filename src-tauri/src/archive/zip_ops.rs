use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use crate::error::{AppError, Result};
use crate::utils::archive_path::parse_entry_rel_path;

use super::extract::write_entry;
use super::links::{create_queued_symlinks, queue_symlink, QueuedSymlink};
use super::path::{detect_common_top_dir, resolve_within_dir, strip_common_top_dir};

/// Extract zip entries using a caller-provided destination resolver.
///
/// Returning `None` from `destination_for` skips the entry.
pub(crate) fn extract_zip_mapped<F>(
    archive_path: &Path,
    dest_dir: &Path,
    mut destination_for: F,
) -> Result<()>
where
    F: FnMut(&str) -> Option<PathBuf>,
{
    fs::create_dir_all(dest_dir).map_err(|e| AppError::io(e.to_string()))?;
    let file = fs::File::open(archive_path).map_err(|e| AppError::io(e.to_string()))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| AppError::io(e.to_string()))?;
    let mut pending_symlinks: Vec<QueuedSymlink> = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|e| AppError::io(e.to_string()))?;

        let raw_name = entry
            .name()
            .map_err(|e| AppError::io(e.to_string()))?
            .to_string();

        if parse_entry_rel_path(&raw_name).is_none() {
            return Err(AppError::io(format!(
                "archive contains unsafe zip path: {raw_name:?}"
            )));
        }

        let Some(out_path) = destination_for(&raw_name) else {
            continue;
        };
        let resolved_out_path = resolve_within_dir(dest_dir, &out_path)?;

        if entry.is_symlink() {
            let mut target = String::new();
            entry
                .read_to_string(&mut target)
                .map_err(|e| AppError::io(e.to_string()))?;
            let pending = queue_symlink(&resolved_out_path, Path::new(&target), dest_dir)?;
            pending_symlinks.push(pending);
        } else {
            let is_dir = entry.is_dir();
            let unix_mode = entry.unix_mode();
            let declared_size = if is_dir { None } else { Some(entry.size()) };
            write_entry(
                &resolved_out_path,
                is_dir,
                &mut entry,
                unix_mode,
                declared_size,
            )?;
        }
    }

    create_queued_symlinks(pending_symlinks)?;

    Ok(())
}

/// Verify that a zip archive is structurally readable and not truncated.
///
/// Opening the archive parses its central directory; visiting every entry also
/// forces each local header to be read and bounds-checks the compressed data.
pub(crate) fn validate_zip_archive(archive_path: &Path) -> Result<()> {
    let file = fs::File::open(archive_path)
        .map_err(|e| AppError::io(format!("Failed to open zip: {e}")))?;
    let file_len = file
        .metadata()
        .map_err(|e| AppError::io(format!("Failed to read zip metadata: {e}")))?
        .len();
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::io(format!("Zip archive is unreadable or truncated: {e}")))?;

    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|e| AppError::io(format!("Zip entry {index} is unreadable: {e}")))?;
        if let Some(data_start) = entry.data_start() {
            let data_end = data_start.saturating_add(entry.compressed_size());
            if data_end > file_len {
                return Err(AppError::io(format!(
                    "Zip entry {index} extends beyond the end of the archive"
                )));
            }
        }
    }

    Ok(())
}

/// Extract zip archive to dest_dir, stripping the top-level directory from the archive.
pub(crate) fn extract_zip_flat(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let top_dir = {
        let file = fs::File::open(archive_path).map_err(|e| AppError::io(e.to_string()))?;
        let archive = zip::ZipArchive::new(file).map_err(|e| AppError::io(e.to_string()))?;
        let names = archive
            .file_names()
            .map(|name| {
                name.map(|name| name.into_owned())
                    .map_err(|e| AppError::io(e.to_string()))
            })
            .collect::<Result<Vec<String>>>()?;
        detect_common_top_dir(names.iter().map(String::as_str))
    };

    extract_zip_mapped(archive_path, dest_dir, |raw_path| {
        let relative = parse_entry_rel_path(raw_path)?;
        let stripped = strip_common_top_dir(&relative, top_dir.as_deref())?;
        Some(dest_dir.join(stripped))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, OpenOptions};
    use std::io::Write as _;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(label: &str) -> Result<Self> {
            let path = std::env::temp_dir().join(format!(
                "astrbot-launcher-zip-{}-{}",
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

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) -> Result<()> {
        let file = fs::File::create(path)?;
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
    fn zip_validation_accepts_readable_archive() -> Result<()> {
        let dir = TestDir::new("valid")?;
        let zip_path = dir.path().join("valid.zip");
        write_zip(
            &zip_path,
            &[("data/config.json", b"{}"), ("data/nested/a.txt", b"a")],
        )?;

        validate_zip_archive(&zip_path)?;
        Ok(())
    }

    #[test]
    fn zip_validation_rejects_truncated_archive() -> Result<()> {
        let dir = TestDir::new("truncated")?;
        let zip_path = dir.path().join("truncated.zip");
        write_zip(&zip_path, &[("data/config.json", b"{}")])?;

        let len = fs::metadata(&zip_path)?.len();
        OpenOptions::new()
            .write(true)
            .open(&zip_path)?
            .set_len(len - 8)?;

        assert!(validate_zip_archive(&zip_path).is_err());
        Ok(())
    }

    #[test]
    fn extraction_rejects_parent_traversal_entry() -> Result<()> {
        let dir = TestDir::new("escape")?;
        let zip_path = dir.path().join("evil.zip");
        write_zip(
            &zip_path,
            &[("data/ok.txt", b"ok"), ("../escape.txt", b"escaped")],
        )?;
        let dest = dir.path().join("dest");
        fs::create_dir_all(&dest)?;

        let result = extract_zip_mapped(&zip_path, &dest, |raw_path| {
            parse_entry_rel_path(raw_path).map(|relative| dest.join(relative))
        });

        assert!(result.is_err());
        assert!(!dir.path().join("escape.txt").exists());
        Ok(())
    }
}
