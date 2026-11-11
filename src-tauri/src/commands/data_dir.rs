use super::AppState;
use crate::error::{AppError, Result};
use crate::utils::paths::{
    default_data_dir, ensure_data_dirs, get_data_dir, set_data_dir_override, validate_data_dir,
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
