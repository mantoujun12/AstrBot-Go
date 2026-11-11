use super::instance_task_spec;
use super::AppState;
use crate::backup;
use crate::error::{AppError, Result};
use crate::tasks::{self, TaskKind};
use tauri::{AppHandle, State};

// === Backup ===

#[tauri::command]
pub async fn create_backup(
    app_handle: AppHandle,
    instance_id: String,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<String> {
    let guard = state.process_manager.acquire_guard(&instance_id)?;
    let spec = instance_task_spec(TaskKind::Backup, "tasks.titles.createBackup", &instance_id);
    tasks::execute_blocking(app_handle, spec, move |context| {
        context.progress(
            Some(0),
            "backup",
            "progress.deploy.backingUpData",
            "正在备份数据…",
        )?;
        let result = backup::create_backup(&instance_id, false, note.as_deref());
        if result.is_ok() {
            context.progress(
                Some(100),
                "done",
                "progress.deploy.dataBackupDone",
                "数据备份完成",
            )?;
        }
        drop(guard);
        result
    })
    .await
}

#[tauri::command]
pub async fn restore_backup(
    app_handle: AppHandle,
    backup_path: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let (resolved_path, metadata) = backup::resolve_restore_backup_input(&backup_path)?;
    let instance_id = metadata.instance_id.clone();
    let guard = state.process_manager.acquire_guard(&instance_id)?;
    let spec = instance_task_spec(TaskKind::Backup, "tasks.titles.restoreBackup", &instance_id);
    tasks::execute_blocking(app_handle, spec, move |context| {
        context.progress(
            Some(0),
            "restore",
            "progress.deploy.restoringData",
            "正在还原数据…",
        )?;
        let result = backup::restore_backup_with_input(resolved_path, metadata);
        if result.is_ok() {
            context.progress(
                Some(100),
                "done",
                "progress.deploy.dataRestored",
                "数据还原完成",
            )?;
        }
        drop(guard);
        result
    })
    .await
}

#[tauri::command]
pub async fn delete_backup(backup_path: String) -> Result<()> {
    tokio::task::spawn_blocking(move || backup::delete_backup(&backup_path))
        .await
        .map_err(|e| AppError::process(format!("Delete backup task panicked: {}", e)))?
}
