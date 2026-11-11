use super::AppState;
use crate::component;
use crate::component::ComponentActionResult;
use crate::error::{AppError, Result};
use tauri::{AppHandle, State};

// === Components ===

enum ComponentCommandAction {
    Install,
    Reinstall,
    Uninstall,
}

async fn run_component_command(
    app_handle: &AppHandle,
    state: &State<'_, AppState>,
    component_id: &str,
    action: ComponentCommandAction,
) -> Result<ComponentActionResult> {
    let client = state.client();
    let id = component::ComponentId::from_str_id(component_id)
        .ok_or_else(|| AppError::other(format!("Unknown component: {}", component_id)))?;

    match action {
        ComponentCommandAction::Install => {
            component::install_component(&client, id, Some(app_handle)).await
        }
        ComponentCommandAction::Reinstall => {
            component::reinstall_component(&client, id, Some(app_handle)).await
        }
        ComponentCommandAction::Uninstall => {
            tokio::task::spawn_blocking(move || component::uninstall_component(id))
                .await
                .map_err(|e| AppError::process(format!("Uninstall task panicked: {}", e)))?
        }
    }
}

#[tauri::command]
pub async fn install_component(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    component_id: String,
) -> Result<ComponentActionResult> {
    run_component_command(
        &app_handle,
        &state,
        &component_id,
        ComponentCommandAction::Install,
    )
    .await
}

#[tauri::command]
pub async fn reinstall_component(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    component_id: String,
) -> Result<ComponentActionResult> {
    run_component_command(
        &app_handle,
        &state,
        &component_id,
        ComponentCommandAction::Reinstall,
    )
    .await
}

#[tauri::command]
pub async fn uninstall_component(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    component_id: String,
) -> Result<ComponentActionResult> {
    run_component_command(
        &app_handle,
        &state,
        &component_id,
        ComponentCommandAction::Uninstall,
    )
    .await
}
