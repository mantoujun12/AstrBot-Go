use super::AppState;
use crate::error::{AppError, Result};
use crate::utils::fs_ops::copy_dir_contents;
use crate::utils::paths::{
    default_data_dir, ensure_data_dirs, get_data_dir, legacy_default_data_dir,
    legacy_migration_dismissed_marker_path, set_data_dir_override, validate_data_dir,
};
use tauri::State;
use tauri_plugin_opener::open_path;

// === Data Directory ===

#[derive(Debug, Clone, serde::Serialize)]
pub struct DataDirChangeResult {
    pub old_dir: String,
    pub new_dir: String,
}

/// Compare data directory paths case-insensitively on case-insensitive
/// filesystems by canonicalizing paths that already exist.
fn comparable_data_dir(dir: &std::path::Path) -> std::path::PathBuf {
    std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf())
}

#[tauri::command]
pub async fn set_data_dir(
    new_dir: String,
    state: State<'_, AppState>,
) -> Result<DataDirChangeResult> {
    if !state.process_manager.get_active_ids().is_empty() {
        return Err(AppError::other("请先停止所有运行中的实例，再更改数据目录"));
    }

    let old_dir = get_data_dir();
    let trimmed = new_dir.trim();

    let new_path = if trimmed.is_empty() {
        // Reset to the default location.
        default_data_dir()
    } else {
        let path = std::path::PathBuf::from(trimmed);
        validate_data_dir(&path)?;
        path
    };

    if comparable_data_dir(&new_path) == comparable_data_dir(&old_dir) {
        return Err(AppError::other("新数据目录与当前数据目录相同"));
    }

    set_data_dir_override(if trimmed.is_empty() {
        None
    } else {
        Some(&new_path)
    })?;
    ensure_data_dirs()?;

    Ok(DataDirChangeResult {
        old_dir: old_dir.display().to_string(),
        new_dir: new_path.display().to_string(),
    })
}

#[tauri::command]
pub async fn open_folder(path: String) -> Result<()> {
    let dir = std::path::PathBuf::from(path);
    if !dir.is_dir() {
        return Err(AppError::other("目标文件夹不存在"));
    }
    open_path(&dir, None::<&str>).map_err(|e| AppError::io(e.to_string()))
}

// === Legacy Data Directory Migration ===

#[derive(Debug, Clone, serde::Serialize)]
pub struct LegacyDataDirPrompt {
    pub legacy_dir: String,
    pub data_dir: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LegacyDataDirMigrationResult {
    pub copied_entries: u64,
    pub legacy_dir: String,
    pub data_dir: String,
}

fn legacy_migration_dismissed() -> bool {
    legacy_migration_dismissed_marker_path()
        .map(|marker| marker.exists())
        .unwrap_or(false)
}

fn write_legacy_migration_dismissed_marker() -> Result<()> {
    let Some(marker) = legacy_migration_dismissed_marker_path() else {
        return Ok(());
    };
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::io(format!("创建目录失败：{}", e)))?;
    }
    std::fs::write(&marker, b"1").map_err(|e| AppError::io(format!("写入标记失败：{}", e)))
}

/// Detect whether the legacy AstrBot Launcher data directory should be
/// offered for migration. Returns `None` when there is nothing to migrate:
/// the legacy directory is absent or empty, the user already declined, or a
/// custom data directory override is in effect.
#[tauri::command]
pub async fn detect_legacy_data_dir() -> Result<Option<LegacyDataDirPrompt>> {
    let Some(legacy_dir) = legacy_default_data_dir() else {
        return Ok(None);
    };
    if !legacy_dir.is_dir() || legacy_migration_dismissed() {
        return Ok(None);
    }

    // Only offer the wizard while the default data directory is in use;
    // a custom override means the user already manages their own location.
    let data_dir = get_data_dir();
    if comparable_data_dir(&data_dir) != comparable_data_dir(&default_data_dir()) {
        return Ok(None);
    }

    let has_content = std::fs::read_dir(&legacy_dir)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false);
    if !has_content {
        return Ok(None);
    }

    log::info!(
        "Detected legacy data directory {}, offering migration wizard",
        crate::utils::log_redact::path_name(&legacy_dir)
    );
    Ok(Some(LegacyDataDirPrompt {
        legacy_dir: legacy_dir.display().to_string(),
        data_dir: data_dir.display().to_string(),
    }))
}

/// Copy the legacy AstrBot Launcher data directory into the current data
/// directory. The legacy directory is kept intact so the predecessor app
/// continues to work with its own data.
#[tauri::command]
pub async fn migrate_legacy_data_dir(
    state: State<'_, AppState>,
) -> Result<LegacyDataDirMigrationResult> {
    if !state.process_manager.get_active_ids().is_empty() {
        return Err(AppError::other("请先停止所有运行中的实例，再迁移数据"));
    }

    let Some(legacy_dir) = legacy_default_data_dir() else {
        return Err(AppError::other("未找到旧版数据目录"));
    };
    if !legacy_dir.is_dir() {
        return Err(AppError::other("未找到旧版数据目录"));
    }
    let data_dir = get_data_dir();
    if comparable_data_dir(&data_dir) == comparable_data_dir(&legacy_dir) {
        return Err(AppError::other("旧版数据目录与当前数据目录相同"));
    }

    let legacy_dir_for_task = legacy_dir.clone();
    let data_dir_for_task = data_dir.clone();
    let result =
        tauri::async_runtime::spawn_blocking(move || -> Result<LegacyDataDirMigrationResult> {
            let copied_entries = copy_dir_contents(&legacy_dir_for_task, &data_dir_for_task)?;

            // When the legacy directory still stores its config in the
            // `data.redb` database, the freshly created default JSON records
            // must not shadow the migrated records: drop them and regenerate
            // from the copied database.
            if data_dir_for_task.join("data.redb").is_file() {
                let _ = std::fs::remove_file(crate::utils::paths::config_json_path());
                let _ = std::fs::remove_file(crate::utils::paths::manifest_json_path());
            }

            // Re-run the startup migrations so legacy TOML records and the
            // copied `data.redb` are imported into the JSON files.
            crate::migration::run_startup_migrations();

            log::info!(
                "Migrated {} entries from the legacy data directory {}",
                copied_entries,
                crate::utils::log_redact::path_name(&legacy_dir_for_task)
            );
            Ok(LegacyDataDirMigrationResult {
                copied_entries,
                legacy_dir: legacy_dir_for_task.display().to_string(),
                data_dir: data_dir_for_task.display().to_string(),
            })
        })
        .await
        .map_err(|e| AppError::other(format!("迁移任务执行失败：{}", e)))??;

    // The legacy directory still exists after a copy-based migration; record
    // the decision so the wizard does not prompt again.
    write_legacy_migration_dismissed_marker()?;

    Ok(result)
}

/// Record that the user declined migrating the legacy data directory.
#[tauri::command]
pub async fn dismiss_legacy_data_dir_migration() -> Result<()> {
    write_legacy_migration_dismissed_marker()
}
