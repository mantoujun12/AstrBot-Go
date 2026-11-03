use std::cmp::Ordering;
use std::sync::RwLock;

use reqwest::Client;
use tauri::{AppHandle, State};
use tauri_plugin_opener::open_path;

use crate::backup;
use crate::component;
use crate::component::{ComponentActionResult, ComponentsSnapshot};
use crate::config::{
    load_config, load_manifest, reload_config, reload_manifest, with_config_mut, AppConfig,
    AppManifest, BackupInfo, InstalledVersion, LanguagePreference, ThemePreference,
};
use crate::download;
use crate::download_control;
use crate::error::{AppError, Result};
use crate::github::{self, GitHubRelease};
use crate::instance::{self, InstanceStatus};
use crate::network_config::{self, NetworkSourceKind};
use crate::platform;
use crate::process::ProcessManager;
use crate::utils::index_url::normalize_default_index;
use crate::utils::lock_check::{collect_files_for_lock_check, ensure_target_not_locked};
use crate::utils::net::{
    build_http_client_with_proxy, check_url, probe_endpoint, probe_pypi, ProbeResult, ProbeStage,
};
use crate::utils::paths::{
    default_data_dir, ensure_data_dirs, get_data_dir, get_instance_core_dir, set_data_dir_override,
    validate_data_dir,
};
use crate::utils::proxy::{
    build_single_url_proxy_settings, ProxyFields, ProxySource, DEFAULT_NO_PROXY_VALUE,
};
use crate::utils::sync::{read_lock_recover, write_lock_recover};
use crate::utils::validation::validate_instance_id;

fn sort_installed_versions_semver(versions: &mut [InstalledVersion]) {
    versions.sort_by(|a, b| {
        let av = semver::Version::parse(a.version.trim_start_matches('v')).ok();
        let bv = semver::Version::parse(b.version.trim_start_matches('v')).ok();

        match (av, bv) {
            (Some(va), Some(vb)) => vb.cmp(&va),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => b.version.cmp(&a.version),
        }
    });
}

pub struct AppState {
    pub client: RwLock<Client>,
    pub process_manager: ProcessManager,
}

impl AppState {
    fn client(&self) -> Client {
        read_lock_recover(&self.client, "AppState.client").clone()
    }

    fn replace_client(&self, client: Client) {
        *write_lock_recover(&self.client, "AppState.client") = client;
    }

    /// Client for large downloads, built from the current proxy settings.
    ///
    /// Downloads must not keep the 30 s total timeout used by API requests,
    /// otherwise long transfers (and paused ones) would be aborted.
    fn download_client(&self) -> Result<Client> {
        let config = load_config()?;
        network_config::build_download_client_from_config(config.as_ref())
    }
}

macro_rules! define_save_config_command {
    ($fn_name:ident, $param:ident : $ty:ty, $field:ident) => {
        #[tauri::command]
        pub async fn $fn_name($param: $ty) -> Result<()> {
            with_config_mut(move |config| {
                config.$field = $param;
                Ok(())
            })
        }
    };
}

fn apply_uv_fallback(config: &mut AppConfig) {
    if config.use_uv_for_deps && !component::is_uv_installed() {
        config.use_uv_for_deps = false;
        if let Err(e) = with_config_mut(|cfg| {
            if cfg.use_uv_for_deps {
                cfg.use_uv_for_deps = false;
            }
            Ok(())
        }) {
            log::warn!("Failed to persist uv fallback to pip: {}", e);
        }
    }
}

pub(crate) fn build_app_snapshot_with(
    process_manager: &ProcessManager,
    load_config_fn: fn() -> Result<std::sync::Arc<AppConfig>>,
    load_manifest_fn: fn() -> Result<std::sync::Arc<AppManifest>>,
) -> Result<AppSnapshot> {
    let config = load_config_fn()?;
    let manifest = load_manifest_fn()?;
    let instances = instance::list_instances(process_manager, manifest.as_ref());
    let backups = backup::list_backups()?;
    let mut config_for_snapshot = (*config).clone();
    apply_uv_fallback(&mut config_for_snapshot);
    let mut versions = manifest.installed_versions.clone();
    sort_installed_versions_semver(&mut versions);

    Ok(AppSnapshot {
        instances,
        versions,
        backups,
        components: component::build_components_snapshot(),
        config: config_for_snapshot,
        data_dir: get_data_dir().display().to_string(),
    })
}

#[tauri::command]
pub async fn get_app_snapshot(state: State<'_, AppState>) -> Result<AppSnapshot> {
    let pm = state.process_manager.clone();
    tokio::task::spawn_blocking(move || build_app_snapshot_with(&pm, load_config, load_manifest))
        .await
        .map_err(|e| AppError::process(format!("Snapshot task panicked: {}", e)))?
}

#[tauri::command]
pub async fn rebuild_app_snapshot(state: State<'_, AppState>) -> Result<AppSnapshot> {
    let pm = state.process_manager.clone();
    tokio::task::spawn_blocking(move || {
        build_app_snapshot_with(&pm, reload_config, reload_manifest)
    })
    .await
    .map_err(|e| AppError::process(format!("Snapshot task panicked: {}", e)))?
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AppSnapshot {
    pub instances: Vec<InstanceStatus>,
    pub versions: Vec<InstalledVersion>,
    pub backups: Vec<BackupInfo>,
    pub components: ComponentsSnapshot,
    pub config: AppConfig,
    pub data_dir: String,
}

// === Config ===

#[tauri::command]
pub fn is_macos() -> bool {
    platform::is_macos()
}

#[tauri::command]
pub async fn save_github_proxy(github_proxy: String, state: State<'_, AppState>) -> Result<()> {
    let github_proxy = github_proxy.trim().to_string();

    // An empty value means "use github.com directly". Testing the default
    // source here would trap users behind an unreachable network or a broken
    // mirror they are trying to remove, so only custom proxies are tested.
    if !github_proxy.is_empty() {
        let url = github::build_api_url(&github_proxy);
        let client = state.client();
        check_url(&client, &url).await?;
    }

    with_config_mut(move |config| {
        config.github_proxy = github_proxy;
        Ok(())
    })
}

#[tauri::command]
pub async fn save_proxy(
    proxy_url: String,
    proxy_port: String,
    proxy_username: String,
    proxy_password: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let proxy_fields = ProxyFields::new(proxy_url, proxy_port, proxy_username, proxy_password);

    // Use a raw client (no system-proxy fallback) purely for connectivity tests.
    let configured_proxy = build_single_url_proxy_settings(
        ProxySource::AppConfig,
        &proxy_fields,
        Some(DEFAULT_NO_PROXY_VALUE.to_string()),
    )?;
    let test_proxy = configured_proxy.clone();
    let test_client = build_http_client_with_proxy(test_proxy)?;

    if !proxy_fields.url.is_empty()
        && check_url(&test_client, "https://cloudflare.com/cdn-cgi/trace")
            .await
            .is_err()
        && check_url(&test_client, "https://baidu.com").await.is_err()
    {
        return Err(AppError::network(
            "代理配置错误，无法连接 cloudflare.com 或 baidu.com",
        ));
    }

    let ProxyFields {
        url,
        port,
        username,
        password,
    } = proxy_fields;

    let next_client = with_config_mut(move |config| {
        config.proxy_url = url;
        config.proxy_port = port;
        config.proxy_username = username;
        config.proxy_password = password;
        network_config::build_http_client_from_config(config)
    })?;

    state.replace_client(next_client);

    Ok(())
}

#[tauri::command]
pub async fn save_pypi_mirror(pypi_mirror: String, state: State<'_, AppState>) -> Result<()> {
    let trimmed = pypi_mirror.trim();

    // An empty value means "use pypi.org directly" (see
    // `normalize_default_index`); testing the default source would block users
    // from removing a mirror they can no longer reach.
    let normalized_for_save = if trimmed.is_empty() {
        String::new()
    } else {
        let normalized_default_index = normalize_default_index(trimmed);
        let check_url_value = format!("{}/", normalized_default_index);
        let client = state.client();
        check_url(&client, &check_url_value).await?;
        normalized_default_index
    };

    with_config_mut(move |config| {
        config.pypi_mirror = normalized_for_save;
        Ok(())
    })
}

define_save_config_command!(save_close_to_tray, close_to_tray: bool, close_to_tray);
define_save_config_command!(
    save_autostart_minimize_to_tray,
    autostart_minimize_to_tray: bool,
    autostart_minimize_to_tray
);
define_save_config_command!(save_nodejs_mirror, nodejs_mirror: String, nodejs_mirror);
define_save_config_command!(save_npm_registry, npm_registry: String, npm_registry);

#[tauri::command]
pub async fn save_python_mirror(python_mirror: String) -> Result<()> {
    let python_mirror = python_mirror.trim().to_string();
    with_config_mut(move |config| {
        config.python_mirror = python_mirror;
        Ok(())
    })
}

#[tauri::command]
pub async fn save_uv_mirror(uv_mirror: String) -> Result<()> {
    let uv_mirror = uv_mirror.trim().to_string();
    with_config_mut(move |config| {
        config.uv_mirror = uv_mirror;
        Ok(())
    })
}

#[tauri::command]
pub async fn save_allow_local_dns_probe(allow_local_dns_probe: bool) -> Result<()> {
    with_config_mut(move |config| {
        config.allow_local_dns_probe = allow_local_dns_probe;
        Ok(())
    })
}

#[tauri::command]
pub async fn test_network_source(
    kind: NetworkSourceKind,
    value: String,
    stage: ProbeStage,
    state: State<'_, AppState>,
) -> Result<ProbeResult> {
    let config = load_config()?;
    let allow_local_dns = config.allow_local_dns_probe;
    let client = state.client();
    let url =
        network_config::source_probe_url(kind, &value, config.as_ref(), &client, allow_local_dns)
            .await?;

    if kind == NetworkSourceKind::Pypi {
        probe_pypi(&client, &url, stage, allow_local_dns).await
    } else {
        probe_endpoint(&client, &url, stage, allow_local_dns).await
    }
}

#[tauri::command]
pub async fn test_http_proxy(
    proxy_url: String,
    proxy_port: String,
    proxy_username: String,
    proxy_password: String,
) -> Result<ProbeResult> {
    let proxy_fields = ProxyFields::new(proxy_url, proxy_port, proxy_username, proxy_password);
    let proxy = build_single_url_proxy_settings(
        ProxySource::AppConfig,
        &proxy_fields,
        Some(DEFAULT_NO_PROXY_VALUE.to_string()),
    )?;
    let client = build_http_client_with_proxy(proxy)?;
    const SPEED_URL: &str = "https://speed.cloudflare.com/__down?bytes=1048576";

    match probe_endpoint(&client, SPEED_URL, ProbeStage::Both, false).await {
        Ok(result) => Ok(result),
        Err(_) => {
            let mut latency =
                probe_endpoint(&client, "https://www.baidu.com", ProbeStage::Latency, false)
                    .await?;
            latency.speed_bps = None;
            Ok(latency)
        }
    }
}

#[tauri::command]
pub async fn save_use_uv_for_deps(use_uv_for_deps: bool) -> Result<()> {
    if use_uv_for_deps && !component::is_uv_installed() {
        return Err(AppError::other("uv 组件未安装，无法启用 uv 安装依赖"));
    }

    with_config_mut(move |config| {
        config.use_uv_for_deps = use_uv_for_deps;
        Ok(())
    })
}

#[tauri::command]
pub fn compare_versions(a: String, b: String) -> i32 {
    match (
        semver::Version::parse(a.trim_start_matches('v')),
        semver::Version::parse(b.trim_start_matches('v')),
    ) {
        (Ok(va), Ok(vb)) => va.cmp(&vb) as i32,
        _ => 0,
    }
}

define_save_config_command!(
    save_check_instance_update,
    check_instance_update: bool,
    check_instance_update
);
define_save_config_command!(
    save_persist_instance_state,
    persist_instance_state: bool,
    persist_instance_state
);
define_save_config_command!(
    save_use_short_instance_id,
    use_short_instance_id: bool,
    use_short_instance_id
);
define_save_config_command!(
    save_ignore_external_path,
    ignore_external_path: bool,
    ignore_external_path
);
define_save_config_command!(
    save_lock_check_extension_whitelist,
    lock_check_extension_whitelist: bool,
    lock_check_extension_whitelist
);
define_save_config_command!(
    save_theme_preference,
    theme_preference: ThemePreference,
    theme_preference
);

#[tauri::command]
pub fn save_language_preference(
    language_preference: LanguagePreference,
    app_handle: AppHandle,
) -> Result<()> {
    with_config_mut(move |config| {
        config.language_preference = language_preference;
        Ok(())
    })?;

    crate::tray::refresh_tray_menu(&app_handle);
    Ok(())
}

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
pub async fn fetch_launcher_release_notes(
    state: State<'_, AppState>,
    version: String,
) -> Result<Option<String>> {
    let client = state.client();
    github::fetch_launcher_release_notes(&client, &version).await
}

// === Version Management ===

#[tauri::command]
pub async fn install_version(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    release: GitHubRelease,
) -> Result<()> {
    let client = state.download_client()?;
    download::download_version(&client, &release, Some(&app_handle)).await
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
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    instance::repair_instance(&instance_id, preserve_scope, &app_handle).await
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
    instance::delete_instance(&instance_id)
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInstanceRequest {
    pub name: Option<String>,
    pub version: Option<String>,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub check_update_enabled: Option<bool>,
}

#[tauri::command]
pub async fn update_instance(
    app_handle: AppHandle,
    instance_id: String,
    request: UpdateInstanceRequest,
    state: State<'_, AppState>,
) -> Result<()> {
    let _guard = state.process_manager.acquire_guard(&instance_id)?;
    instance::update_instance(
        &instance_id,
        request.name.as_deref(),
        request.version.as_deref(),
        request.host.as_deref(),
        request.port,
        request.check_update_enabled,
        &app_handle,
    )
    .await
}

#[tauri::command]
pub async fn start_instance(
    app_handle: AppHandle,
    instance_id: String,
    state: State<'_, AppState>,
) -> Result<u16> {
    state
        .process_manager
        .start_instance(&instance_id, app_handle)
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
    state
        .process_manager
        .restart_instance(&instance_id, app_handle)
        .await
}

#[tauri::command]
pub async fn get_instance_port(instance_id: String, state: State<'_, AppState>) -> Result<u16> {
    state
        .process_manager
        .get_port(&instance_id)
        .ok_or_else(AppError::instance_not_running)
}

// === Backup ===

#[tauri::command]
pub async fn create_backup(
    instance_id: String,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<String> {
    let guard = state.process_manager.acquire_guard(&instance_id)?;
    tokio::task::spawn_blocking(move || {
        let result = backup::create_backup(&instance_id, false, note.as_deref());
        drop(guard);
        result
    })
    .await
    .map_err(|e| AppError::process(format!("Create backup task panicked: {}", e)))?
}

#[tauri::command]
pub async fn restore_backup(backup_path: String, state: State<'_, AppState>) -> Result<()> {
    let (resolved_path, metadata) = backup::resolve_restore_backup_input(&backup_path)?;
    let guard = state.process_manager.acquire_guard(&metadata.instance_id)?;
    tokio::task::spawn_blocking(move || {
        let result = backup::restore_backup_with_input(resolved_path, metadata);
        drop(guard);
        result
    })
    .await
    .map_err(|e| AppError::process(format!("Restore backup task panicked: {}", e)))?
}

#[tauri::command]
pub async fn delete_backup(backup_path: String) -> Result<()> {
    tokio::task::spawn_blocking(move || backup::delete_backup(&backup_path))
        .await
        .map_err(|e| AppError::process(format!("Delete backup task panicked: {}", e)))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed(version: &str) -> InstalledVersion {
        InstalledVersion {
            version: version.to_string(),
            zip_path: format!("/versions/{version}.zip"),
        }
    }

    #[test]
    fn compare_versions_handles_prefixes_and_prereleases() {
        assert_eq!(
            compare_versions("v4.0.0".to_string(), "4.0.0".to_string()),
            0
        );
        assert_eq!(
            compare_versions("4.1.0".to_string(), "4.0.9".to_string()),
            1
        );
        assert_eq!(
            compare_versions("4.0.0".to_string(), "4.0.0-beta.1".to_string()),
            1
        );
        assert_eq!(
            compare_versions("4.0.0-beta.2".to_string(), "4.0.0-beta.1".to_string()),
            1
        );
        assert_eq!(
            compare_versions("4.0.0+build.1".to_string(), "4.0.0+build.1".to_string()),
            0
        );
        assert_eq!(
            compare_versions("4.0.0+build.2".to_string(), "4.0.0+build.1".to_string()),
            1
        );
    }

    #[test]
    fn compare_versions_returns_zero_for_invalid_input() {
        assert_eq!(
            compare_versions("not-a-version".to_string(), "4.0.0".to_string()),
            0
        );
        assert_eq!(compare_versions("4.0.0".to_string(), String::new()), 0);
    }

    #[test]
    fn installed_versions_sort_newest_first_with_invalid_entries_last() {
        let mut versions = vec![
            installed("v4.0.0"),
            installed("invalid"),
            installed("4.2.0-beta.1"),
            installed("4.10.0"),
            installed("v4.2.0"),
        ];

        sort_installed_versions_semver(&mut versions);

        let order: Vec<&str> = versions.iter().map(|v| v.version.as_str()).collect();
        assert_eq!(
            order,
            vec!["4.10.0", "v4.2.0", "4.2.0-beta.1", "v4.0.0", "invalid"]
        );
    }
}
