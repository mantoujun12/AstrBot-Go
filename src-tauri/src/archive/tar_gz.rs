use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, Result};
use crate::utils::archive_path::parse_entry_rel_path;

use super::extract::write_entry;
use super::links::{create_hard_link_entry, create_queued_symlinks, queue_symlink, QueuedSymlink};
use super::path::{
    finalize_common_top_dir, resolve_within_dir, scan_common_top_dir, strip_common_top_dir,
    validate_rel_link_target,
};

/// Extract tar.gz entries using a caller-provided destination resolver.
///
/// Returning `None` from `destination_for` skips the entry.
pub(crate) fn extract_tar_gz_mapped<F>(
    archive_path: &Path,
    dest_dir: &Path,
    mut destination_for: F,
) -> Result<()>
where
    F: FnMut(&str) -> Option<PathBuf>,
{
    fs::create_dir_all(dest_dir).map_err(|e| AppError::io(e.to_string()))?;
    let file = fs::File::open(archive_path).map_err(|error| AppError::io(error.to_string()))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let mut extracted_files = HashSet::new();
    let mut pending_symlinks: Vec<QueuedSymlink> = Vec::new();

    for entry in archive
        .entries()
        .map_err(|error| AppError::io(error.to_string()))?
    {
        let mut entry = entry.map_err(|error| AppError::io(error.to_string()))?;

        let raw_path = {
            let entry_path = entry
                .path()
                .map_err(|error| AppError::io(error.to_string()))?;
            let s = entry_path.as_ref().to_str().ok_or_else(|| {
                AppError::io(format!(
                    "archive entry path is not valid UTF-8: {:?}",
                    entry_path
                ))
            })?;
            s.to_string()
        };

        if parse_entry_rel_path(&raw_path).is_none() {
            return Err(AppError::io(format!(
                "archive contains unsafe entry path: {raw_path:?}"
            )));
        }

        let Some(out_path) = destination_for(&raw_path) else {
            continue;
        };
        let resolved_out_path = resolve_within_dir(dest_dir, &out_path)?;

        let entry_type = entry.header().entry_type();
        match entry_type {
            tar::EntryType::Symlink => {
                let target = entry
                    .link_name()
                    .map_err(|e| AppError::io(e.to_string()))?
                    .ok_or_else(|| AppError::io("symlink entry missing link target"))?;
                let pending = queue_symlink(&resolved_out_path, target.as_ref(), dest_dir)?;
                pending_symlinks.push(pending);
            }
            tar::EntryType::Link => {
                let target = entry
                    .link_name()
                    .map_err(|e| AppError::io(e.to_string()))?
                    .ok_or_else(|| AppError::io("hard link entry missing link target"))?;
                let target_str = target
                    .as_ref()
                    .to_str()
                    .ok_or_else(|| AppError::io("hard link target is not valid UTF-8"))?;
                let target_path = target.as_ref();
                validate_rel_link_target(target_path, "hard link")?;

                let mut candidates = Vec::new();
                if let Some(mapped) = destination_for(target_str) {
                    candidates.push(resolve_within_dir(dest_dir, &mapped)?);
                }
                let parent = resolved_out_path
                    .parent()
                    .ok_or_else(|| AppError::io("hard link entry has no parent directory"))?;
                candidates.push(resolve_within_dir(dest_dir, &parent.join(target_path))?);

                let resolved_target = candidates
                    .into_iter()
                    .find(|candidate| extracted_files.contains(candidate))
                    .ok_or_else(|| {
                        AppError::io(
                            "hard link target is unsafe or was not extracted earlier in the archive",
                        )
                    })?;

                create_hard_link_entry(&resolved_out_path, &resolved_target)?;

                extracted_files.insert(resolved_out_path);
            }
            _ => {
                if !entry_type.is_dir() && !entry_type.is_file() {
                    return Err(AppError::io(format!(
                        "unsupported tar entry type at {raw_path:?}: {entry_type:?}"
                    )));
                }
                let unix_mode = entry.header().mode().ok();
                let declared_size = if entry_type.is_file() {
                    Some(
                        entry
                            .header()
                            .size()
                            .map_err(|error| AppError::io(error.to_string()))?,
                    )
                } else {
                    None
                };
                write_entry(
                    &resolved_out_path,
                    entry_type.is_dir(),
                    &mut entry,
                    unix_mode,
                    declared_size,
                )?;
                if entry_type.is_file() {
                    extracted_files.insert(resolved_out_path);
                }
            }
        }
    }

    create_queued_symlinks(pending_symlinks)?;

    Ok(())
}

/// Verify that a tar.gz archive can be decompressed and parsed to the end.
///
/// Reading every entry and then draining the decoder also verifies the gzip
/// trailer (CRC and original size), so truncated downloads are rejected.
pub(crate) fn validate_tar_gz_archive(archive_path: &Path) -> Result<()> {
    let file = fs::File::open(archive_path)
        .map_err(|e| AppError::io(format!("Failed to open tar.gz: {e}")))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let mut sink = std::io::sink();

    for entry in archive
        .entries()
        .map_err(|e| AppError::io(format!("Tar.gz archive is unreadable: {e}")))?
    {
        let mut entry =
            entry.map_err(|e| AppError::io(format!("Tar.gz entry is unreadable: {e}")))?;
        std::io::copy(&mut entry, &mut sink)
            .map_err(|e| AppError::io(format!("Tar.gz entry is truncated: {e}")))?;
    }

    let mut decoder = archive.into_inner();
    std::io::copy(&mut decoder, &mut sink)
        .map_err(|e| AppError::io(format!("Tar.gz archive is truncated: {e}")))?;

    Ok(())
}

/// Extract tar.gz archive to dest_dir, stripping the top-level directory from the archive.
pub(crate) fn extract_tar_gz_flat(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    // First pass: find the common top-level directory.
    let file = fs::File::open(archive_path).map_err(|error| AppError::io(error.to_string()))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    let mut top_dir_candidate = None;
    let mut top_dir_saw_nested = false;
    let mut top_dir_valid = true;
    for entry in archive
        .entries()
        .map_err(|error| AppError::io(error.to_string()))?
    {
        let entry = entry.map_err(|error| AppError::io(error.to_string()))?;
        let entry_path = entry
            .path()
            .map_err(|error| AppError::io(error.to_string()))?;
        if let Some(path_str) = entry_path.as_ref().to_str() {
            if let Some(relative) = parse_entry_rel_path(path_str) {
                if let Some(s) = relative.to_str() {
                    scan_common_top_dir(
                        s,
                        &mut top_dir_candidate,
                        &mut top_dir_saw_nested,
                        &mut top_dir_valid,
                    );
                }
            }
        }
    }

    let top_dir = finalize_common_top_dir(top_dir_candidate, top_dir_saw_nested, top_dir_valid);

    extract_tar_gz_mapped(archive_path, dest_dir, |raw_path| {
        let relative = parse_entry_rel_path(raw_path)?;
        let stripped = strip_common_top_dir(&relative, top_dir.as_deref())?;
        Some(dest_dir.join(stripped))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, OpenOptions};

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(label: &str) -> Result<Self> {
            let path = std::env::temp_dir().join(format!(
                "astrbot-launcher-tar-{}-{}",
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

    fn write_tar_gz(path: &Path) -> Result<()> {
        let file = fs::File::create(path)?;
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut builder = tar::Builder::new(encoder);
        let data = b"hello";
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder.append_data(&mut header, "data/hello.txt", &data[..])?;
        let encoder = builder.into_inner()?;
        encoder.finish()?;
        Ok(())
    }

    #[test]
    fn tar_gz_validation_accepts_readable_archive() -> Result<()> {
        let dir = TestDir::new("valid")?;
        let archive = dir.path().join("valid.tar.gz");
        write_tar_gz(&archive)?;

        validate_tar_gz_archive(&archive)?;
        Ok(())
    }

    #[test]
    fn tar_gz_validation_rejects_truncated_archive() -> Result<()> {
        let dir = TestDir::new("truncated")?;
        let archive = dir.path().join("truncated.tar.gz");
        write_tar_gz(&archive)?;

        let len = fs::metadata(&archive)?.len();
        OpenOptions::new()
            .write(true)
            .open(&archive)?
            .set_len(len - 8)?;

        assert!(validate_tar_gz_archive(&archive).is_err());
        Ok(())
    }
}
