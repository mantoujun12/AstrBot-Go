//! Rebuild instance and version manifest entries by scanning the data directory.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::config::{with_manifest_mut, InstalledVersion, InstanceConfig, DEFAULT_INSTANCE_HOST};
use crate::error::{AppError, Result};
use crate::utils::log_redact;
use crate::utils::paths::{get_data_dir, get_versions_dir};
use crate::utils::validation::validate_instance_id;

#[derive(Debug, Clone, serde::Serialize)]
pub struct RebuildInstanceManifestResult {
    pub instances: usize,
    pub versions: usize,
}

pub fn rebuild_instance_manifest_from_disk() -> Result<RebuildInstanceManifestResult> {
    let installed_versions = scan_installed_versions()?;
    let instances = scan_instances()?;
    let instance_count = instances.len();
    let version_count = installed_versions.len();

    with_manifest_mut(move |manifest| {
        manifest.installed_versions = installed_versions;
        manifest.instances = merge_recovered_instances(instances, &manifest.instances);
        let rebuilt_ids: HashSet<String> = manifest.instances.keys().cloned().collect();
        manifest
            .tracked_instances_snapshot
            .retain(|id| rebuilt_ids.contains(id));
        Ok(())
    })?;

    Ok(RebuildInstanceManifestResult {
        instances: instance_count,
        versions: version_count,
    })
}

/// Keep the dashboard login of every instance that survives a rebuild; the
/// recovered record cannot read it back from the instance directory.
fn merge_recovered_instances(
    recovered: HashMap<String, InstanceConfig>,
    existing: &HashMap<String, InstanceConfig>,
) -> HashMap<String, InstanceConfig> {
    recovered
        .into_iter()
        .map(|(id, mut instance)| {
            if let Some(saved) = existing.get(&id) {
                instance.dashboard_username = saved.dashboard_username.clone();
                instance.dashboard_password = saved.dashboard_password.clone();
            }
            (id, instance)
        })
        .collect()
}

fn scan_installed_versions() -> Result<Vec<InstalledVersion>> {
    let versions_dir = get_versions_dir();
    if !versions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut versions = Vec::new();
    for entry in fs::read_dir(&versions_dir)
        .map_err(|e| AppError::io(format!("Failed to read versions dir: {}", e)))?
    {
        let entry = entry.map_err(|e| AppError::io(e.to_string()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(filename) = path.file_name().and_then(|name| name.to_str()) else {
            log::warn!(
                "Skipping version archive with non-UTF-8 filename: {}",
                log_redact::path_name(&path)
            );
            continue;
        };
        let Some(version) = filename.strip_suffix(".zip") else {
            continue;
        };
        let Some(zip_path) = path.to_str() else {
            log::warn!(
                "Skipping version archive with non-UTF-8 path: {}",
                log_redact::path_name(&path)
            );
            continue;
        };

        versions.push(InstalledVersion {
            version: version.to_string(),
            zip_path: zip_path.to_string(),
        });
    }

    Ok(versions)
}

fn scan_instances() -> Result<HashMap<String, InstanceConfig>> {
    let instances_dir = get_data_dir().join("instances");
    if !instances_dir.exists() {
        return Ok(HashMap::new());
    }

    let mut instances = HashMap::new();

    for entry in fs::read_dir(&instances_dir)
        .map_err(|e| AppError::io(format!("Failed to read instances dir: {}", e)))?
    {
        let entry = entry.map_err(|e| AppError::io(e.to_string()))?;
        if !entry
            .file_type()
            .map_err(|e| AppError::io(e.to_string()))?
            .is_dir()
        {
            continue;
        }

        let Some(instance_id) = entry.file_name().to_str().map(str::to_string) else {
            log::warn!(
                "Skipping instance directory with non-UTF-8 name: {}",
                log_redact::path_name(&entry.path())
            );
            continue;
        };
        if let Err(error) = validate_instance_id(&instance_id) {
            log::warn!(
                "Skipping invalid instance directory {}: {}",
                instance_id,
                error
            );
            continue;
        }

        let instance_dir = entry.path();
        if !looks_like_instance_dir(&instance_dir) {
            log::warn!(
                "Skipping instance directory without recognizable content: {}",
                log_redact::path_name(&instance_dir)
            );
            continue;
        }

        let Some(version) = read_pyproject_version(&instance_dir) else {
            log::warn!(
                "Skipping instance {} because core/pyproject.toml has no project.version",
                instance_id
            );
            continue;
        };

        let defaults = crate::process::ResourceAlertThreshold::default();
        let instance = InstanceConfig {
            name: default_instance_name(&instance_id),
            version,
            host: DEFAULT_INSTANCE_HOST.to_string(),
            port: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
            check_update_enabled: true,
            resource_alert_cpu_percent: Some(defaults.cpu_percent),
            resource_alert_memory_bytes: Some(defaults.memory_bytes),
            resource_alert_disk_bytes: Some(defaults.disk_bytes),
            resource_alert_enabled: defaults.enabled,
            auto_restart_enabled: false,
            auto_restart_max_retries: 3,
            auto_restart_notify: true,
            dashboard_username: String::new(),
            dashboard_password: String::new(),
        };

        instances.insert(instance_id, instance);
    }

    Ok(instances)
}

fn looks_like_instance_dir(instance_dir: &Path) -> bool {
    let core_dir = instance_dir.join("core");
    instance_dir.join("venv").exists()
        || core_dir.join("main.py").exists()
        || core_dir.join("data").exists()
        || core_dir.join("pyproject.toml").exists()
        || core_dir.join("requirements.txt").exists()
}

fn read_pyproject_version(instance_dir: &Path) -> Option<String> {
    let pyproject_path = instance_dir.join("core").join("pyproject.toml");
    let content = fs::read_to_string(&pyproject_path).ok()?;
    let value = match toml::from_str::<toml::Value>(&content) {
        Ok(value) => value,
        Err(error) => {
            log::warn!(
                "Failed to parse {}: {}",
                log_redact::path_name(&pyproject_path),
                error
            );
            return None;
        }
    };
    let version = value
        .get("project")
        .and_then(|project| project.get("version"))
        .and_then(toml::Value::as_str)?
        .trim();

    if version.is_empty() {
        return None;
    }
    if version.starts_with('v') {
        Some(version.to_string())
    } else {
        Some(format!("v{}", version))
    }
}

fn default_instance_name(instance_id: &str) -> String {
    let short_id: String = instance_id.chars().take(8).collect();
    format!("Rev {}", short_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::ResourceAlertThreshold;

    fn instance_config(name: &str, username: &str, password: &str) -> InstanceConfig {
        let defaults = ResourceAlertThreshold::default();
        InstanceConfig {
            name: name.to_string(),
            version: "v4.29.4".to_string(),
            host: DEFAULT_INSTANCE_HOST.to_string(),
            port: 0,
            created_at: String::new(),
            check_update_enabled: true,
            resource_alert_cpu_percent: Some(defaults.cpu_percent),
            resource_alert_memory_bytes: Some(defaults.memory_bytes),
            resource_alert_disk_bytes: Some(defaults.disk_bytes),
            resource_alert_enabled: defaults.enabled,
            auto_restart_enabled: false,
            auto_restart_max_retries: 3,
            auto_restart_notify: true,
            dashboard_username: username.to_string(),
            dashboard_password: password.to_string(),
        }
    }

    #[test]
    fn rebuild_keeps_saved_credentials_for_surviving_instances() {
        let existing = HashMap::from([
            (
                "keep".to_string(),
                instance_config("saved", "admin", "secret"),
            ),
            (
                "gone".to_string(),
                instance_config("removed", "ghost", "gone"),
            ),
        ]);
        let recovered = HashMap::from([
            ("keep".to_string(), instance_config("Rev keep", "", "")),
            ("new".to_string(), instance_config("Rev new", "", "")),
        ]);

        let merged = merge_recovered_instances(recovered, &existing);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged["keep"].dashboard_username, "admin");
        assert_eq!(merged["keep"].dashboard_password, "secret");
        assert_eq!(merged["keep"].name, "Rev keep");
        assert_eq!(merged["new"].dashboard_username, "");
        assert_eq!(merged["new"].dashboard_password, "");
        assert!(!merged.contains_key("gone"));
    }
}
