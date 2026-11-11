use super::instance_task_spec;
use super::AppState;
use crate::config::load_manifest;
use crate::error::{AppError, Result};
use crate::instance;
use crate::process::InstanceRuntimeInfo;
use crate::tasks::{self, TaskKind};
use crate::utils::paths::get_instance_core_dir;
use crate::utils::validation::validate_instance_id;
use tauri::{AppHandle, State};
use tauri_plugin_opener::open_path;

// === Instance Management ===

#[tauri::command]
pub async fn open_instance_core_folder(instance_id: String) -> Result<()> {
    validate_instance_id(&instance_id)?;
    let core_dir = get_instance_core_dir(&instance_id);

    if !core_dir.is_dir() {
        return Err(AppError::other(
            "实例 core 文件夹不存在，可以尝试先运行一次实例后再打开。",
        ));
    }

    open_path(&core_dir, None::<&str>).map_err(|e| AppError::io(e.to_string()))
}

#[tauri::command]
pub async fn create_instance(name: String, version: String, port: u16) -> Result<()> {
    instance::create_instance(&name, &version, port)
}

#[tauri::command]
pub async fn delete_instance(instance_id: String, state: State<'_, AppState>) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    let result = instance::delete_instance(&instance_id);
    // Do not leak the crash tracker for a removed instance; it otherwise keeps
    // growing the in-memory map for the manager lifetime.
    state.process_manager.forget_restart_tracking(&instance_id);
    result
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInstanceRequest {
    pub name: Option<String>,
    pub version: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub check_update_enabled: Option<bool>,
    pub auto_restart_enabled: Option<bool>,
    pub auto_restart_max_retries: Option<u32>,
    pub auto_restart_notify: Option<bool>,
}

#[tauri::command]
pub async fn update_instance(
    app_handle: AppHandle,
    instance_id: String,
    request: UpdateInstanceRequest,
    state: State<'_, AppState>,
) -> Result<()> {
    let guard = state.process_manager.acquire_guard(&instance_id)?;
    let version_change = request.version.as_deref().is_some_and(|version| {
        load_manifest()
            .ok()
            .and_then(|manifest| {
                manifest
                    .instances
                    .get(&instance_id)
                    .map(|instance| instance.version != version)
            })
            .unwrap_or(false)
    });

    if !version_change {
        return instance::update_instance(
            &instance_id,
            request.name.as_deref(),
            request.version.as_deref(),
            request.host.as_deref(),
            request.port,
            request.check_update_enabled,
            request.auto_restart_enabled,
            request.auto_restart_max_retries,
            request.auto_restart_notify,
            &app_handle,
        )
        .await;
    }

    let spec = instance_task_spec(
        TaskKind::Deploy,
        "tasks.titles.deployInstance",
        &instance_id,
    );
    tasks::execute(app_handle, spec, move |context| async move {
        context.check_cancelled()?;
        let outcome = instance::update_instance(
            &instance_id,
            request.name.as_deref(),
            request.version.as_deref(),
            request.host.as_deref(),
            request.port,
            request.check_update_enabled,
            request.auto_restart_enabled,
            request.auto_restart_max_retries,
            request.auto_restart_notify,
            &context.app_handle,
        )
        .await;
        drop(guard);
        outcome
    })
    .await
}

#[tauri::command]
pub async fn start_instance(
    app_handle: AppHandle,
    instance_id: String,
    state: State<'_, AppState>,
) -> Result<u16> {
    let spec = instance_task_spec(
        TaskKind::Deploy,
        "tasks.titles.deployInstance",
        &instance_id,
    );
    tasks::execute(app_handle, spec, move |context| async move {
        let result = state
            .process_manager
            .start_instance(&instance_id, context.app_handle)
            .await;
        // A manual start abandons the previous crash trail: the instance now
        // runs and any retained reason/counts would show a stale warning.
        if result.is_ok() {
            state.process_manager.clear_crash_signals(&instance_id);
        }
        result
    })
    .await
}

#[tauri::command]
pub async fn stop_instance(instance_id: String, state: State<'_, AppState>) -> Result<()> {
    state.process_manager.stop_instance(&instance_id).await
}

#[tauri::command]
pub async fn restart_instance(
    app_handle: AppHandle,
    instance_id: String,
    state: State<'_, AppState>,
) -> Result<u16> {
    let spec = instance_task_spec(
        TaskKind::Deploy,
        "tasks.titles.deployInstance",
        &instance_id,
    );
    tasks::execute(app_handle, spec, move |context| async move {
        let result = state
            .process_manager
            .restart_instance(&instance_id, context.app_handle)
            .await;
        // A manual restart abandons the previous crash trail: the instance
        // now runs and any retained reason/counts would show a stale warning.
        if result.is_ok() {
            state.process_manager.clear_crash_signals(&instance_id);
        }
        result
    })
    .await
}

#[tauri::command]
pub async fn get_instance_port(instance_id: String, state: State<'_, AppState>) -> Result<u16> {
    state
        .process_manager
        .get_port(&instance_id)
        .ok_or_else(AppError::instance_not_running)
}

#[tauri::command]
pub async fn check_instance_health(
    instance_id: String,
    state: State<'_, AppState>,
) -> Result<instance::InstanceHealthReport> {
    validate_instance_id(&instance_id)?;
    let manifest = load_manifest()?;
    let instance_config = manifest
        .instances
        .get(&instance_id)
        .ok_or_else(|| AppError::instance_not_found(&instance_id))?
        .clone();
    let runtime_info = state.process_manager.get_runtime_info();
    let (running_port, is_transitioning) = match runtime_info.get(&instance_id) {
        Some(InstanceRuntimeInfo::Live { port, .. }) => (Some(*port), false),
        Some(InstanceRuntimeInfo::Starting | InstanceRuntimeInfo::Stopping { .. }) => (None, true),
        None => (None, false),
    };

    Ok(instance::check_instance_health(
        &instance_id,
        &instance_config,
        running_port,
        is_transitioning,
    )
    .await)
}
