//! Tauri commands for managing AstrBot instance data offline (issue #56).
//!
//! Reads run on a blocking thread without guards; every write acquires the
//! instance lifecycle guard first so a start/stop cannot race the edit, and
//! the instance must be stopped because AstrBot caches this data in memory.

use super::AppState;
use crate::error::{AppError, Result};
use crate::instance::data_manage;
use crate::instance::plugins;
use crate::instance::skills;
use crate::instance::{
    CopyInstanceDataRequest, InstanceConfigFile, InstanceCronJob, InstanceKnowledgeBase,
    InstancePersona, InstancePersonaFolder, InstancePlugin, InstancePluginSkill, InstanceSkill,
    PluginInstallRequest, PluginMarketPage, SkillDirEntry, SkillFileRequest, SkillZipInstall,
};
use crate::process::InstanceGuard;
use crate::utils::paths::get_instance_core_dir;
use std::path::PathBuf;
use tauri::State;

async fn off_runtime<T, F>(work: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| AppError::process(format!("数据管理后台任务失败：{error}")))?
}

fn data_dir_of(instance_id: &str) -> Result<PathBuf> {
    data_manage::get_instance_data_dir(instance_id)
}

/// AstrBot's builtin star plugins, read-only skill providers.
fn builtin_stars_dir_of(instance_id: &str) -> PathBuf {
    get_instance_core_dir(instance_id)
        .join("astrbot")
        .join("builtin_stars")
}

// === Instance Data ===

#[tauri::command]
pub async fn list_instance_personas(instance_id: String) -> Result<Vec<InstancePersona>> {
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        data_manage::list_personas(&data_dir)
    })
    .await
}

#[tauri::command]
pub async fn save_instance_personas(
    instance_id: String,
    upserts: Vec<InstancePersona>,
    deletes: Vec<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        data_manage::save_personas(&data_dir, &upserts, &deletes)
    })
    .await
}

#[tauri::command]
pub async fn list_instance_cron_jobs(instance_id: String) -> Result<Vec<InstanceCronJob>> {
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        data_manage::list_cron_jobs(&data_dir)
    })
    .await
}

#[tauri::command]
pub async fn save_instance_cron_jobs(
    instance_id: String,
    upserts: Vec<InstanceCronJob>,
    deletes: Vec<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        data_manage::save_cron_jobs(&data_dir, &upserts, &deletes)
    })
    .await
}

#[tauri::command]
pub async fn list_instance_config_files(instance_id: String) -> Result<Vec<InstanceConfigFile>> {
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        data_manage::list_config_files(&data_dir)
    })
    .await
}

#[tauri::command]
pub async fn read_instance_config_file(instance_id: String, name: String) -> Result<String> {
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        data_manage::read_config_file(&data_dir, &name)
    })
    .await
}

#[tauri::command]
pub async fn write_instance_config_file(
    instance_id: String,
    name: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        data_manage::write_config_file(&data_dir, &name, &content)
    })
    .await
}

#[tauri::command]
pub async fn list_instance_persona_folders(
    instance_id: String,
) -> Result<Vec<InstancePersonaFolder>> {
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        data_manage::list_persona_folders(&data_dir)
    })
    .await
}

#[tauri::command]
pub async fn list_instance_knowledge_bases(
    instance_id: String,
) -> Result<Vec<InstanceKnowledgeBase>> {
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        data_manage::list_knowledge_bases(&data_dir)
    })
    .await
}

#[tauri::command]
pub async fn delete_instance_knowledge_base(
    instance_id: String,
    kb_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        data_manage::delete_knowledge_base(&data_dir, &kb_id)
    })
    .await
}

#[tauri::command]
pub async fn copy_instance_data(
    request: CopyInstanceDataRequest,
    state: State<'_, AppState>,
) -> Result<()> {
    // Every touched instance gets a lifecycle guard for the whole copy so a
    // start/stop cannot race the write.
    let mut guards: Vec<InstanceGuard> = Vec::new();
    guards.push(
        state
            .process_manager
            .acquire_guard(&request.source_instance_id)?,
    );
    for target_id in &request.target_instance_ids {
        guards.push(state.process_manager.acquire_guard(target_id)?);
    }
    let result = off_runtime(move || data_manage::copy_instance_data(&request)).await;
    drop(guards);
    result
}

// === Instance plugins ===

#[tauri::command]
pub async fn fetch_plugin_market(
    page: Option<i64>,
    page_size: Option<i64>,
    search: Option<String>,
    category: Option<String>,
    state: State<'_, AppState>,
) -> Result<PluginMarketPage> {
    let client = state.client();
    plugins::fetch_market_page(
        &client,
        page.unwrap_or(1),
        page_size.unwrap_or(30),
        search.as_deref(),
        category.as_deref(),
    )
    .await
}

#[tauri::command]
pub async fn list_instance_plugins(instance_id: String) -> Result<Vec<InstancePlugin>> {
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        plugins::list_plugins(&data_dir)
    })
    .await
}

#[tauri::command]
pub async fn install_instance_plugin(
    instance_id: String,
    request: PluginInstallRequest,
    state: State<'_, AppState>,
) -> Result<InstancePlugin> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    let data_dir = data_dir_of(&instance_id)?;
    let client = state.download_client()?;
    plugins::install_plugin(&client, &data_dir, &request).await
}

#[tauri::command]
pub async fn set_instance_plugin_enabled(
    instance_id: String,
    dir_name: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        plugins::set_plugin_enabled(&data_dir, &dir_name, enabled)
    })
    .await
}

#[tauri::command]
pub async fn uninstall_instance_plugin(
    instance_id: String,
    dir_name: String,
    delete_config: bool,
    delete_data: bool,
    state: State<'_, AppState>,
) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        plugins::uninstall_plugin(&data_dir, &dir_name, delete_config, delete_data)
    })
    .await
}

// === Instance skills ===

#[tauri::command]
pub async fn list_instance_skills(instance_id: String) -> Result<Vec<InstanceSkill>> {
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        Ok(skills::list_skills(&data_dir))
    })
    .await
}

#[tauri::command]
pub async fn list_instance_plugin_skills(instance_id: String) -> Result<Vec<InstancePluginSkill>> {
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        let builtin_root = builtin_stars_dir_of(&instance_id);
        Ok(skills::list_plugin_skills(&data_dir, &builtin_root))
    })
    .await
}

#[tauri::command]
pub async fn list_instance_skill_files(
    instance_id: String,
    request: SkillFileRequest,
) -> Result<Vec<SkillDirEntry>> {
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        let builtin_root = builtin_stars_dir_of(&instance_id);
        skills::list_skill_files(&data_dir, &builtin_root, &request)
    })
    .await
}

#[tauri::command]
pub async fn read_instance_skill_file(
    instance_id: String,
    request: SkillFileRequest,
) -> Result<String> {
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        let builtin_root = builtin_stars_dir_of(&instance_id);
        skills::read_skill_file(&data_dir, &builtin_root, &request)
    })
    .await
}

#[tauri::command]
pub async fn write_instance_skill_file(
    instance_id: String,
    request: SkillFileRequest,
    content: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        let builtin_root = builtin_stars_dir_of(&instance_id);
        skills::write_skill_file(&data_dir, &builtin_root, &request, &content)
    })
    .await
}

#[tauri::command]
pub async fn set_instance_skill_enabled(
    instance_id: String,
    name: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        skills::set_skill_active(&data_dir, &name, enabled)
    })
    .await
}

#[tauri::command]
pub async fn delete_instance_skill(
    instance_id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        skills::delete_skill(&data_dir, &name)
    })
    .await
}

#[tauri::command]
pub async fn install_instance_skill_zip(
    instance_id: String,
    request: SkillZipInstall,
    state: State<'_, AppState>,
) -> Result<Vec<String>> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    off_runtime(move || {
        let data_dir = data_dir_of(&instance_id)?;
        skills::install_skill_zip(&data_dir, &request)
    })
    .await
}
