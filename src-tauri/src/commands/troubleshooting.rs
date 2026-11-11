use super::instance_task_spec;
use super::AppState;
use crate::backup;
use crate::component;
use crate::error::{AppError, Result};
use crate::instance;
use crate::tasks::{self, TaskKind};
use crate::utils::lock_check::{collect_files_for_lock_check, ensure_target_not_locked};
use crate::utils::paths::get_instance_core_dir;
use crate::utils::validation::validate_instance_id;
use tauri::{AppHandle, State};

// === Troubleshooting ===

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LockCheckTarget {
    InstanceData,
    BackupCreate,
    BackupRestore,
    InstanceUpgrade,
}

fn run_instance_data_lock_check(instance_id: &str) -> Result<()> {
    validate_instance_id(instance_id)?;
    let data_dir = get_instance_core_dir(instance_id).join("data");
    if !data_dir.exists() {
        return Ok(());
    }
    let target_files = collect_files_for_lock_check(&data_dir)?;
    ensure_target_not_locked(&target_files)
}

#[tauri::command]
pub async fn check_lock(
    target: LockCheckTarget,
    instance_id: Option<String>,
    backup_path: Option<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    match target {
        LockCheckTarget::InstanceData
        | LockCheckTarget::BackupCreate
        | LockCheckTarget::InstanceUpgrade => {
            let instance_id = instance_id
                .as_deref()
                .ok_or_else(|| AppError::other("check_lock 缺少 instance_id"))?;
            let _guard = state.process_manager.acquire_guard(instance_id)?;
            run_instance_data_lock_check(instance_id)
        }
        LockCheckTarget::BackupRestore => {
            let backup_path = backup_path
                .as_deref()
                .ok_or_else(|| AppError::other("check_lock 缺少 backup_path"))?;
            let (_, metadata) = backup::resolve_restore_backup_input(backup_path)?;
            let _guard = state.process_manager.acquire_guard(&metadata.instance_id)?;
            run_instance_data_lock_check(&metadata.instance_id)
        }
    }
}

#[tauri::command]
pub async fn clear_instance_data(instance_id: String, state: State<'_, AppState>) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    instance::clear_instance_data(&instance_id)
}

#[tauri::command]
pub async fn clear_instance_venv(instance_id: String, state: State<'_, AppState>) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    instance::clear_instance_venv(&instance_id)
}

#[tauri::command]
pub async fn clear_uv_cache() -> Result<()> {
    component::clear_uv_cache().await
}

#[tauri::command]
pub async fn clear_pycache(instance_id: String, state: State<'_, AppState>) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    instance::clear_pycache(&instance_id)
}

#[tauri::command]
pub async fn repair_instance(
    app_handle: AppHandle,
    instance_id: String,
    preserve_scope: instance::RepairPreserveScope,
    state: State<'_, AppState>,
) -> Result<()> {
    let guard = state.process_manager.acquire_guard(&instance_id)?;
    let spec = instance_task_spec(
        TaskKind::Repair,
        "tasks.titles.repairInstance",
        &instance_id,
    );
    tasks::execute(app_handle, spec, move |context| async move {
        context.check_cancelled()?;
        let outcome =
            instance::repair_instance(&instance_id, preserve_scope, &context.app_handle).await;
        drop(guard);
        outcome
    })
    .await
}

#[tauri::command]
pub async fn rebuild_instance_manifest(
    state: State<'_, AppState>,
) -> Result<instance::RebuildInstanceManifestResult> {
    if !state.process_manager.get_active_ids().is_empty() {
        return Err(AppError::instance_running());
    }

    let result = tokio::task::spawn_blocking(instance::rebuild_instance_manifest_from_disk)
        .await
        .map_err(|e| {
            AppError::process(format!("Rebuild instance manifest task panicked: {}", e))
        })??;
    Ok(result)
}
