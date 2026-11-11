use crate::component;
use crate::component::ComponentsSnapshot;
use crate::config::{
    load_config, load_manifest, reload_config, reload_manifest, with_config_mut, AppConfig,
    AppManifest, BackupInfo, InstalledVersion,
};
use crate::error::{AppError, Result};
use crate::github::GitHubRelease;
use crate::instance::InstanceStatus;
use crate::network_config;
use crate::process::ProcessManager;
use crate::tasks::{TaskKind, TaskSpec};
use crate::utils::paths::get_data_dir;
use crate::utils::sync::{read_lock_recover, write_lock_recover};
use reqwest::Client;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::sync::RwLock;
use tauri::State;

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

fn task_args(pairs: [(&str, String); 1]) -> BTreeMap<String, String> {
    pairs
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

fn download_task_spec(release: &GitHubRelease) -> TaskSpec {
    let mut spec = TaskSpec::new(TaskKind::Download, "tasks.titles.downloadVersion");
    spec.title_args = task_args([("version", release.tag_name.clone())]);
    spec.target_id = Some(release.tag_name.clone());
    spec.cancellable = true;
    spec.pausable = true;
    spec.dedupe_target = true;
    spec.control_ref = Some(release.tag_name.clone());
    spec.initial_message_key = "progress.download.starting".to_string();
    spec.initial_message = "开始下载".to_string();
    spec
}

fn instance_task_spec(kind: TaskKind, title_key: &'static str, instance_id: &str) -> TaskSpec {
    let mut spec = TaskSpec::new(kind, title_key);
    let instance_name = load_manifest()
        .ok()
        .and_then(|manifest| {
            manifest
                .instances
                .get(instance_id)
                .map(|instance| instance.name.clone())
        })
        .unwrap_or_else(|| instance_id.to_string());
    spec.title_args = task_args([("instance", instance_name)]);
    spec.target_id = Some(instance_id.to_string());
    spec.dedupe_target = true;
    // Instance work clears and refills instance data mid-flight, so a cancellation
    // would abort between the two halves; only downloads can be cancelled safely.
    spec.cancellable = false;
    spec.initial_message_key = "tasks.status.running".to_string();
    spec.initial_message = "运行中".to_string();
    spec
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
    let instances = crate::instance::list_instances(process_manager, manifest.as_ref());
    let backups = crate::backup::list_backups()?;
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

pub(crate) mod backup;
pub(crate) mod components;
pub(crate) mod config;
pub(crate) mod data_dir;
pub(crate) mod github;
pub(crate) mod instance;
pub(crate) mod resources;
pub(crate) mod tasks_center;
pub(crate) mod troubleshooting;
pub(crate) mod version;
pub(crate) mod window;

pub use backup::*;
pub use components::*;
pub use config::*;
pub use data_dir::*;
pub use github::*;
pub use instance::*;
pub use resources::*;
pub use tasks_center::*;
pub use troubleshooting::*;
pub use version::*;
pub use window::*;
