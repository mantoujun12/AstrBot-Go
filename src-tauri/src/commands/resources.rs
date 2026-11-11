use super::AppState;
use crate::config::{load_manifest, with_manifest_mut};
use crate::error::{AppError, ErrorKind, Result};
use crate::utils::validation::validate_instance_id;
use tauri::State;

// === Resource Monitoring ===

#[tauri::command]
pub async fn get_instance_resource_history(
    instance_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::process::ResourceSample>> {
    Ok(state.process_manager.get_resource_history(&instance_id))
}

#[tauri::command]
pub async fn get_resources_snapshot(
    state: State<'_, AppState>,
) -> Result<std::collections::HashMap<String, crate::process::ResourceSample>> {
    Ok(state.process_manager.get_all_latest_resources())
}

fn validate_resource_thresholds(thresholds: &crate::process::ResourceAlertThreshold) -> Result<()> {
    if !thresholds.cpu_percent.is_finite()
        || thresholds.cpu_percent <= 0.0
        || thresholds.cpu_percent > 100.0
    {
        return Err(AppError::config(
            "CPU threshold must be between 0 and 100 percent",
        ));
    }
    if thresholds.memory_bytes == 0 {
        return Err(AppError::config(
            "Memory threshold must be greater than zero",
        ));
    }
    if thresholds.disk_bytes == 0 {
        return Err(AppError::config("Disk threshold must be greater than zero"));
    }
    Ok(())
}

#[tauri::command]
pub async fn set_resource_alert_threshold(
    instance_id: String,
    thresholds: crate::process::ResourceAlertThreshold,
    state: State<'_, AppState>,
) -> Result<()> {
    validate_instance_id(&instance_id)?;
    validate_resource_thresholds(&thresholds)?;

    let stored = thresholds.clone();
    let id = instance_id.clone();
    with_manifest_mut(move |manifest| {
        let instance = manifest
            .instances
            .get_mut(&id)
            .ok_or_else(|| AppError::instance_not_found(&id))?;
        instance.resource_alert_enabled = stored.enabled;
        instance.resource_alert_cpu_percent = Some(stored.cpu_percent);
        instance.resource_alert_memory_bytes = Some(stored.memory_bytes);
        instance.resource_alert_disk_bytes = Some(stored.disk_bytes);
        Ok(())
    })?;

    if state
        .process_manager
        .get_resource_thresholds(&instance_id)
        .is_some()
    {
        match state
            .process_manager
            .set_resource_thresholds(&instance_id, thresholds)
        {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::InstanceNotRunning => {}
            Err(error) => return Err(error),
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn get_resource_alert_threshold(
    instance_id: String,
) -> Result<crate::process::ResourceAlertThreshold> {
    validate_instance_id(&instance_id)?;
    let manifest = load_manifest()?;
    let instance = manifest
        .instances
        .get(&instance_id)
        .ok_or_else(|| AppError::instance_not_found(&instance_id))?;

    let defaults = crate::process::ResourceAlertThreshold::default();
    Ok(crate::process::ResourceAlertThreshold {
        enabled: instance.resource_alert_enabled,
        cpu_percent: instance
            .resource_alert_cpu_percent
            .unwrap_or(defaults.cpu_percent),
        memory_bytes: instance
            .resource_alert_memory_bytes
            .unwrap_or(defaults.memory_bytes),
        disk_bytes: instance
            .resource_alert_disk_bytes
            .unwrap_or(defaults.disk_bytes),
    })
}
