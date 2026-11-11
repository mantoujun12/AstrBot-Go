use super::AppState;
use crate::config::{load_config, load_manifest};
use crate::error::Result;
use crate::github::{self, GitHubRelease};
use crate::update_check::{self, InstanceUpdateCheckResult};
use tauri::State;

// === GitHub ===

#[tauri::command]
pub async fn fetch_releases(
    state: State<'_, AppState>,
    force_refresh: Option<bool>,
) -> Result<Vec<GitHubRelease>> {
    let client = state.client();
    github::fetch_releases(&client, force_refresh.unwrap_or(false)).await
}

#[tauri::command]
pub async fn check_instance_updates(
    state: State<'_, AppState>,
    force_refresh: Option<bool>,
) -> Result<InstanceUpdateCheckResult> {
    let config = load_config()?;
    let manifest = load_manifest()?;
    let global_check_enabled = config.check_instance_update;
    let latest_release = if global_check_enabled {
        let releases =
            github::fetch_releases(&state.client(), force_refresh.unwrap_or(false)).await?;
        update_check::latest_stable_release(&releases)
    } else {
        None
    };

    Ok(update_check::build_update_check_result(
        global_check_enabled,
        manifest.as_ref(),
        latest_release,
    ))
}

#[tauri::command]
pub async fn fetch_launcher_release_notes(
    state: State<'_, AppState>,
    version: String,
) -> Result<Option<String>> {
    let client = state.client();
    github::fetch_launcher_release_notes(&client, &version).await
}
