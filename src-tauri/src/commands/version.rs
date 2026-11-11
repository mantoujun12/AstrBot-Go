use super::download_task_spec;
use super::AppState;
use crate::download;
use crate::download_control;
use crate::error::Result;
use crate::github::GitHubRelease;
use crate::tasks;
use tauri::{AppHandle, State};

// === Version Management ===

#[tauri::command]
pub async fn install_version(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    release: GitHubRelease,
) -> Result<()> {
    let client = state.download_client()?;
    let spec = download_task_spec(&release);
    tasks::execute(app_handle, spec, move |context| async move {
        download::download_version(
            &client,
            &release,
            Some(&context.app_handle),
            Some(&context.id),
        )
        .await
    })
    .await
}

#[tauri::command]
pub fn pause_download(id: String) -> Result<()> {
    download_control::pause(&id)
}

#[tauri::command]
pub fn resume_download(id: String) -> Result<()> {
    download_control::resume(&id)
}

#[tauri::command]
pub fn cancel_download(id: String) {
    download_control::cancel(&id);
}

#[tauri::command]
pub async fn uninstall_version(version: String) -> Result<()> {
    download::remove_version(&version)
}
