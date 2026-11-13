//! Filesystem helpers that keep downloads and backups from damaging existing data.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use crate::error::{AppError, Result};

/// Build a unique hidden sibling path such as `.name.part-<uuid>`.
pub(crate) fn unique_sibling_path(path: &Path, tag: &str) -> Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::io("Path has no parent directory"))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file");
    Ok(parent.join(format!(".{file_name}.{tag}-{}", uuid::Uuid::new_v4())))
}

/// Create a new unique sibling file, retrying if a UUID collides.
pub(crate) fn create_unique_sibling_file(path: &Path, tag: &str) -> Result<(PathBuf, File)> {
    for _ in 0..8 {
        let candidate = unique_sibling_path(path, tag)?;
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(AppError::io(format!(
                    "Failed to create temporary file: {}",
                    error
                )));
            }
        }
    }

    Err(AppError::io(
        "Failed to create a unique temporary file after repeated attempts",
    ))
}

/// Verify that a finished download has the expected byte count.
pub(crate) fn validate_file_length(
    path: &Path,
    expected_total: Option<u64>,
    written: u64,
) -> Result<()> {
    let actual = fs::metadata(path)
        .map_err(|error| AppError::network(format!("无法读取下载文件信息：{}", error)))?
        .len();

    if actual != written {
        return Err(AppError::network(format!(
            "下载文件写入不完整：记录 {} 字节，实际 {} 字节",
            written, actual
        )));
    }

    if let Some(expected) = expected_total {
        if expected != actual {
            return Err(AppError::network(format!(
                "下载文件大小与服务器声明不一致：期望 {} 字节，实际 {} 字节",
                expected, actual
            )));
        }
    }

    if actual == 0 {
        return Err(AppError::network("下载文件为空"));
    }

    Ok(())
}

fn destination_exists(dest: &Path) -> Result<bool> {
    match fs::symlink_metadata(dest) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(AppError::io(format!(
            "Failed to inspect destination path: {}",
            error
        ))),
    }
}

/// Move `temp` over `dest`, preserving the previous file if the swap fails.
///
/// Windows refuses to rename over an existing file, so the old file is first
/// moved to a unique sibling backup. Only the temporary file is removed on
/// failure; the old file is restored or kept at the backup path.
pub(crate) fn replace_file_atomically(temp: &Path, dest: &Path) -> Result<()> {
    if !destination_exists(dest)? {
        return fs::rename(temp, dest).map_err(|error| {
            AppError::io(format!(
                "Failed to move downloaded file into place: {}",
                error
            ))
        });
    }

    let backup_path = unique_sibling_path(dest, "bak")?;
    fs::rename(dest, &backup_path).map_err(|error| {
        AppError::io(format!(
            "Failed to preserve the existing file before replacement: {}",
            error
        ))
    })?;

    match fs::rename(temp, dest) {
        Ok(()) => {
            if let Err(error) = fs::remove_file(&backup_path) {
                log::warn!(
                    "Replaced file but failed to remove its temporary backup: {}",
                    error
                );
            }
            Ok(())
        }
        Err(replace_error) => match fs::rename(&backup_path, dest) {
            Ok(()) => Err(AppError::io(format!(
                "Failed to replace the existing file; the previous file was restored: {}",
                replace_error
            ))),
            Err(rollback_error) => {
                let backup_name = backup_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("backup file");
                Err(AppError::io(format!(
                    "Failed to replace the existing file ({}), and restoring the previous file also failed ({}). The previous file is kept as {} in the same directory",
                    replace_error, rollback_error, backup_name
                )))
            }
        },
    }
}

/// Recursively copy the contents of `src` into `dst`, creating `dst` when
/// missing. Existing destination files are overwritten by the source
/// ("source wins"), which matches a migration where the source directory
/// holds the authoritative data. Symbolic links are skipped deliberately:
/// launcher data directories must not contain them.
pub(crate) fn copy_dir_contents(src: &Path, dst: &Path) -> Result<u64> {
    fs::create_dir_all(dst)
        .map_err(|error| AppError::io(format!("Failed to create {}: {}", dst.display(), error)))?;

    let mut copied = 0u64;
    let entries = fs::read_dir(src)
        .map_err(|error| AppError::io(format!("Failed to read {}: {}", src.display(), error)))?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            AppError::io(format!("Failed to list {}: {}", src.display(), error))
        })?;
        let file_type = entry.file_type().map_err(|error| {
            AppError::io(format!(
                "Failed to inspect {}: {}",
                entry.path().display(),
                error
            ))
        })?;
        let from = entry.path();
        let to = dst.join(entry.file_name());

        if file_type.is_dir() {
            copied += copy_dir_contents(&from, &to)?;
        } else if file_type.is_file() {
            fs::copy(&from, &to).map_err(|error| {
                AppError::io(format!("Failed to copy {}: {}", from.display(), error))
            })?;
            copied += 1;
        }
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(label: &str) -> Result<Self> {
            let path = std::env::temp_dir().join(format!(
                "astrbot-launcher-fs-ops-{}-{}",
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
        let mut file = File::create(path)?;
        file.write_all(content)?;
        Ok(())
    }

    #[test]
    fn copy_dir_contents_copies_tree_and_source_wins() -> Result<()> {
        let src = TestDir::new("copy-src")?;
        let dst = TestDir::new("copy-dst")?;

        write_file(&src.path().join("config.json"), b"legacy")?;
        fs::create_dir_all(src.path().join("instances/main"))?;
        write_file(&src.path().join("instances/main/data.txt"), b"legacy-data")?;

        write_file(&dst.path().join("config.json"), b"default")?;

        let copied = copy_dir_contents(src.path(), dst.path())?;

        assert_eq!(copied, 2);
        assert_eq!(fs::read(dst.path().join("config.json"))?, b"legacy");
        assert_eq!(
            fs::read(dst.path().join("instances/main/data.txt"))?,
            b"legacy-data"
        );
        // The source directory must be kept intact.
        assert!(src.path().join("config.json").exists());
        assert!(src.path().join("instances/main/data.txt").exists());
        Ok(())
    }

    #[test]
    fn unique_temporary_files_do_not_collide() -> Result<()> {
        let dir = TestDir::new("unique")?;
        let dest = dir.path().join("artifact.zip");

        let (first_path, first_file) = create_unique_sibling_file(&dest, "part")?;
        let (second_path, second_file) = create_unique_sibling_file(&dest, "part")?;

        assert_ne!(first_path, second_path);
        assert!(first_path.exists());
        assert!(second_path.exists());
        drop(first_file);
        drop(second_file);
        Ok(())
    }

    #[test]
    fn length_validation_rejects_mismatch_and_empty_files() -> Result<()> {
        let dir = TestDir::new("length")?;
        let path = dir.path().join("payload.bin");
        write_file(&path, b"abc")?;

        validate_file_length(&path, Some(3), 3)?;
        validate_file_length(&path, None, 3)?;
        assert!(validate_file_length(&path, Some(4), 3).is_err());
        assert!(validate_file_length(&path, None, 4).is_err());

        let empty = dir.path().join("empty.bin");
        write_file(&empty, b"")?;
        assert!(validate_file_length(&empty, None, 0).is_err());
        Ok(())
    }

    #[test]
    fn atomic_replacement_installs_new_content() -> Result<()> {
        let dir = TestDir::new("replace-ok")?;
        let dest = dir.path().join("artifact.bin");
        let temp = dir.path().join(".artifact.part-test");
        write_file(&dest, b"old")?;
        write_file(&temp, b"new")?;

        replace_file_atomically(&temp, &dest)?;

        assert_eq!(fs::read(&dest)?, b"new");
        assert!(!temp.exists());
        assert_eq!(fs::read_dir(dir.path())?.count(), 1);
        Ok(())
    }

    #[test]
    fn atomic_replacement_restores_old_content_on_failure() -> Result<()> {
        let dir = TestDir::new("replace-rollback")?;
        let dest = dir.path().join("artifact.bin");
        let missing_temp = dir.path().join(".artifact.part-missing");
        write_file(&dest, b"old")?;

        assert!(replace_file_atomically(&missing_temp, &dest).is_err());

        assert_eq!(fs::read(&dest)?, b"old");
        assert_eq!(fs::read_dir(dir.path())?.count(), 1);
        Ok(())
    }
}
