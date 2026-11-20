use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};
use crate::utils::paths::{config_json_path, ensure_data_dirs, manifest_json_path};

static STORE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn default_true() -> bool {
    true
}

pub const DEFAULT_INSTANCE_HOST: &str = "127.0.0.1";

fn default_instance_host() -> String {
    DEFAULT_INSTANCE_HOST.to_string()
}

fn default_auto_restart_max_retries() -> u32 {
    3
}

fn default_auto_restart_notify() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

/// How the tray icon reacts to a right click.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TrayStyle {
    NativeMenu,
    #[default]
    Panel,
}

impl<'de> Deserialize<'de> for TrayStyle {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Malformed values (null, numbers, objects) and unknown strings from
        // another build must not invalidate the whole config record, which
        // would be reset as corrupted, so they degrade to the default style.
        let value = serde_json::Value::deserialize(deserializer)?;
        Ok(match value.as_str() {
            Some("native_menu") => Self::NativeMenu,
            _ => Self::Panel,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReduceMotionPreference {
    #[default]
    System,
    On,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LanguagePreference {
    #[default]
    #[serde(rename = "system")]
    System,
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
    #[serde(rename = "ru-RU")]
    RuRu,
    #[serde(rename = "ja-JP")]
    JaJp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub github_proxy: String,
    #[serde(default)]
    pub proxy_url: String,
    #[serde(default)]
    pub proxy_port: String,
    #[serde(default)]
    pub proxy_username: String,
    #[serde(default)]
    pub proxy_password: String,
    #[serde(default)]
    pub pypi_mirror: String,
    #[serde(default)]
    pub nodejs_mirror: String,
    #[serde(default)]
    pub npm_registry: String,
    #[serde(default)]
    pub python_mirror: String,
    #[serde(default)]
    pub uv_mirror: String,
    #[serde(default)]
    pub allow_local_dns_probe: bool,
    #[serde(default)]
    pub use_uv_for_deps: bool,
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
    #[serde(default)]
    pub tray_style: TrayStyle,
    /// Persisted tray panel height in logical pixels; `None` until the panel
    /// has reported a height once.
    #[serde(default)]
    pub tray_panel_height: Option<f64>,
    #[serde(default)]
    pub autostart_minimize_to_tray: bool,
    #[serde(default = "default_true")]
    pub check_instance_update: bool,
    /// Check the launcher's own releases when the app starts.
    #[serde(default = "default_true")]
    pub auto_check_launcher_update: bool,
    /// Download and install a launcher update without asking. Only applies to
    /// the startup check; a manual check always asks before installing.
    #[serde(default)]
    pub auto_update_launcher: bool,
    #[serde(default)]
    pub persist_instance_state: bool,
    #[serde(default)]
    pub ignore_external_path: bool,
    #[serde(default)]
    pub use_short_instance_id: bool,
    #[serde(default)]
    pub lock_check_extension_whitelist: bool,
    /// Discovery toggles per registered instance source; a missing entry means
    /// the source is enabled. Only explicit opt-outs are stored.
    #[serde(default)]
    pub instance_source_enabled: HashMap<String, bool>,
    /// Roots scanned by the directory-scan discovery source. Empty means the
    /// user's home directory is scanned.
    #[serde(default)]
    pub instance_scan_paths: Vec<String>,
    #[serde(default)]
    pub theme_preference: ThemePreference,
    #[serde(default)]
    pub language_preference: LanguagePreference,
    #[serde(default)]
    pub reduce_motion_preference: ReduceMotionPreference,
    #[serde(default)]
    pub ignored_launcher_version: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            github_proxy: String::new(),
            proxy_url: String::new(),
            proxy_port: String::new(),
            proxy_username: String::new(),
            proxy_password: String::new(),
            pypi_mirror: String::new(),
            nodejs_mirror: String::new(),
            npm_registry: String::new(),
            python_mirror: String::new(),
            uv_mirror: String::new(),
            allow_local_dns_probe: false,
            use_uv_for_deps: false,
            close_to_tray: true,
            tray_style: TrayStyle::Panel,
            tray_panel_height: None,
            autostart_minimize_to_tray: false,
            check_instance_update: true,
            auto_check_launcher_update: true,
            auto_update_launcher: false,
            persist_instance_state: false,
            ignore_external_path: false,
            use_short_instance_id: false,
            lock_check_extension_whitelist: false,
            instance_source_enabled: HashMap::new(),
            instance_scan_paths: Vec::new(),
            theme_preference: ThemePreference::System,
            language_preference: LanguagePreference::System,
            reduce_motion_preference: ReduceMotionPreference::System,
            ignored_launcher_version: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppManifest {
    #[serde(default)]
    pub instances: HashMap<String, InstanceConfig>,
    #[serde(default)]
    pub installed_versions: Vec<InstalledVersion>,
    #[serde(default)]
    pub tracked_instances_snapshot: Vec<String>,
}

/// Keep a launcher record readable by its owner only. The manifest stores the
/// per-instance dashboard password, so group/other read access must be dropped
/// on Unix. Windows keeps the default ACL: the record already lives in the
/// per-user profile directory.
#[cfg(unix)]
fn restrict_record_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt as _;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|e| AppError::config(format!("chmod {}: {}", path.display(), e)))
}

/// Serialize and write a JSON record atomically (temp file + rename).
fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    ensure_data_dirs()?;
    let payload = serde_json::to_vec_pretty(value).map_err(|e| AppError::config(e.to_string()))?;
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, payload)
        .map_err(|e| AppError::config(format!("write {}: {}", path.display(), e)))?;
    // Set the mode on the temp file so the rename publishes it already private.
    #[cfg(unix)]
    restrict_record_permissions(&temp)?;
    fs::rename(&temp, path)
        .map_err(|e| AppError::config(format!("rename {}: {}", path.display(), e)))?;
    Ok(())
}

/// Read a JSON record file, returning `None` when the file does not exist.
fn read_json_text(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    fs::read_to_string(path)
        .map(Some)
        .map_err(|e| AppError::config(format!("read {}: {}", path.display(), e)))
}

fn write_config_record(path: &Path, config: AppConfig) -> Result<AppConfig> {
    write_json_atomic(path, &config)?;
    Ok(config)
}

/// Delete every `openapi_key` field left under `instances` by builds that
/// authenticated with an API key. The values are never read or logged; the
/// rewrite happens only when at least one stale field was found.
fn scrub_openapi_keys_at(path: &Path) -> Result<usize> {
    let Some(text) = read_json_text(path)? else {
        return Ok(0);
    };
    let mut value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            log::warn!(
                "Manifest record is not valid JSON, skipping the OpenAPI key scrub: {error}"
            );
            return Ok(0);
        }
    };

    let mut scrubbed = 0usize;
    if let Some(instances) = value
        .get_mut("instances")
        .and_then(serde_json::Value::as_object_mut)
    {
        for instance in instances.values_mut() {
            if instance
                .as_object_mut()
                .is_some_and(|record| record.remove("openapi_key").is_some())
            {
                scrubbed += 1;
            }
        }
    }

    if scrubbed > 0 {
        write_json_atomic(path, &value)?;
        log::info!("Removed stored OpenAPI keys from {scrubbed} instance record(s)");
    }
    Ok(scrubbed)
}

/// One-time startup scrub for manifests saved before dashboard login replaced
/// the OpenAPI key, so the unused secret does not stay on disk.
pub fn scrub_openapi_keys() -> Result<usize> {
    let _guard = STORE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    scrub_openapi_keys_at(&manifest_json_path())
}

fn load_or_init_config_value() -> Result<AppConfig> {
    let _guard = STORE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let path = config_json_path();
    match read_json_text(&path)?.map(|text| serde_json::from_str::<AppConfig>(&text)) {
        Some(Ok(config)) => Ok(config),
        Some(Err(error)) => {
            log::warn!(
                "Config record is corrupted JSON, resetting to default value: {}",
                error
            );
            write_config_record(&path, AppConfig::default())
        }
        None => write_config_record(&path, AppConfig::default()),
    }
}

fn load_or_init_manifest_value() -> Result<AppManifest> {
    let _guard = STORE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let path = manifest_json_path();
    let stored = read_json_text(&path)?.map(|text| serde_json::from_str::<AppManifest>(&text));

    if let Some(Ok(manifest)) = stored {
        return Ok(manifest);
    }

    if let Some(Err(error)) = stored {
        log::warn!(
            "Manifest record is corrupted JSON, resetting to default value: {}",
            error
        );
    }

    let default = AppManifest::default();
    write_json_atomic(&path, &default)?;
    Ok(default)
}

fn with_config_value_mut<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&mut AppConfig) -> Result<R>,
{
    let _guard = STORE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let path = config_json_path();
    let mut config =
        match read_json_text(&path)?.map(|text| serde_json::from_str::<AppConfig>(&text)) {
            Some(Ok(config)) => config,
            Some(Err(error)) => {
                log::warn!(
                    "Config record is corrupted JSON, replacing with default before update: {}",
                    error
                );
                AppConfig::default()
            }
            None => AppConfig::default(),
        };

    let result = f(&mut config)?;
    write_json_atomic(&path, &config)?;

    Ok(result)
}

fn with_manifest_value_mut<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&mut AppManifest) -> Result<R>,
{
    let _guard = STORE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let path = manifest_json_path();
    let mut manifest =
        match read_json_text(&path)?.map(|text| serde_json::from_str::<AppManifest>(&text)) {
            Some(Ok(manifest)) => manifest,
            Some(Err(error)) => {
                log::warn!(
                    "Manifest record is corrupted JSON, replacing with default before update: {}",
                    error
                );
                AppManifest::default()
            }
            None => AppManifest::default(),
        };

    let result = f(&mut manifest)?;
    write_json_atomic(&path, &manifest)?;

    Ok(result)
}

/// Execute a transactional read-modify-write operation on app config.
pub fn with_config_mut<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&mut AppConfig) -> Result<T>,
{
    with_config_value_mut(f)
}

/// Execute a transactional read-modify-write operation on app manifest.
pub fn with_manifest_mut<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&mut AppManifest) -> Result<T>,
{
    with_manifest_value_mut(f)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub name: String,
    pub version: String,
    #[serde(default = "default_instance_host")]
    pub host: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub created_at: String,
    #[serde(default = "default_true")]
    pub check_update_enabled: bool,
    /// `None` means "not customised yet", filled from the alert defaults.
    #[serde(default)]
    pub resource_alert_cpu_percent: Option<f32>,
    #[serde(default)]
    pub resource_alert_memory_bytes: Option<u64>,
    #[serde(default)]
    pub resource_alert_disk_bytes: Option<u64>,
    #[serde(default = "default_true")]
    pub resource_alert_enabled: bool,
    #[serde(default)]
    pub auto_restart_enabled: bool,
    #[serde(default = "default_auto_restart_max_retries")]
    pub auto_restart_max_retries: u32,
    #[serde(default = "default_auto_restart_notify")]
    pub auto_restart_notify: bool,
    /// AstrBot dashboard account used by the built-in chat. The credentials are
    /// exchanged for a JWT on demand and never reach the webview.
    #[serde(default)]
    pub dashboard_username: String,
    #[serde(default)]
    pub dashboard_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InstalledVersion {
    pub version: String,
    pub zip_path: String,
}

/// Backup metadata stored in backup.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub created_at: String,
    pub instance_name: String,
    pub instance_id: String,
    pub version: String,
    #[serde(default)]
    pub includes_venv: bool,
    #[serde(default = "default_true")]
    pub includes_data: bool,
    #[serde(default)]
    pub arch_target: String,
    #[serde(default)]
    pub auto_generated: bool,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub filename: String,
    pub path: String,
    pub metadata: BackupMetadata,
    #[serde(default)]
    pub corrupted: bool,
    #[serde(default)]
    pub parse_error: Option<String>,
}

pub fn load_config() -> Result<Arc<AppConfig>> {
    let config = load_or_init_config_value()?;
    Ok(Arc::new(config))
}

pub fn load_manifest() -> Result<Arc<AppManifest>> {
    let manifest = load_or_init_manifest_value()?;
    Ok(Arc::new(manifest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_written_before_auto_restart_fields_still_loads() -> Result<()> {
        let manifest: AppManifest = serde_json::from_str(
            r#"{"instances":{"id":{"name":"main","version":"v4.29.4"}},"installed_versions":[{"version":"v4.29.4","zip_path":"C:\\versions\\v4.29.4.zip"}],"tracked_instances_snapshot":[]}"#,
        )?;
        let instance = manifest.instances.get("id").ok_or_else(|| {
            AppError::other("manifest saved by an older build must keep its instances")
        })?;
        assert!(!instance.auto_restart_enabled);
        assert_eq!(instance.auto_restart_max_retries, 3);
        assert!(instance.auto_restart_notify);
        assert!(instance.resource_alert_enabled);
        assert_eq!(instance.resource_alert_cpu_percent, None);
        assert_eq!(instance.resource_alert_memory_bytes, None);
        assert_eq!(instance.resource_alert_disk_bytes, None);
        Ok(())
    }

    #[test]
    fn tray_style_tolerates_unknown_values() -> Result<()> {
        let menu: TrayStyle = serde_json::from_str(r#""native_menu""#)?;
        assert_eq!(menu, TrayStyle::NativeMenu);

        let panel: TrayStyle = serde_json::from_str(r#""panel""#)?;
        assert_eq!(panel, TrayStyle::Panel);

        // A value written by a newer build must not invalidate the whole
        // config record, which would then be reset to defaults.
        let unknown: TrayStyle = serde_json::from_str(r#""compact_cards""#)?;
        assert_eq!(unknown, TrayStyle::Panel);

        assert_eq!(
            serde_json::to_string(&TrayStyle::NativeMenu)?,
            r#""native_menu""#
        );
        Ok(())
    }

    #[test]
    fn app_config_without_tray_style_defaults_to_panel() -> Result<()> {
        let config: AppConfig = serde_json::from_str("{}")?;
        assert_eq!(config.tray_style, TrayStyle::Panel);
        Ok(())
    }

    #[test]
    fn launcher_update_preferences_default_to_auto_check_only() -> Result<()> {
        let config: AppConfig = serde_json::from_str("{}")?;
        assert!(config.auto_check_launcher_update);
        assert!(!config.auto_update_launcher);

        let stored: AppConfig = serde_json::from_str(
            r#"{"auto_check_launcher_update":false,"auto_update_launcher":true}"#,
        )?;
        assert!(!stored.auto_check_launcher_update);
        assert!(stored.auto_update_launcher);
        Ok(())
    }

    #[test]
    fn tray_style_tolerates_malformed_values() -> Result<()> {
        // Null, numbers and objects must not fail deserialization: a failed
        // AppConfig parse makes classify_config_record treat the record as
        // corrupted and reset every other setting with it.
        for raw in ["null", "42", "{\"style\":\"panel\"}", "[]", "true"] {
            let style: TrayStyle = serde_json::from_str(raw)?;
            assert_eq!(style, TrayStyle::Panel, "raw value: {raw}");
        }
        Ok(())
    }

    #[test]
    fn app_config_keeps_other_fields_when_tray_style_is_malformed() -> Result<()> {
        let config: AppConfig = serde_json::from_str(
            r#"{
                "tray_style": {"unexpected": true},
                "github_proxy": "https://gh.example/",
                "close_to_tray": false,
                "tray_panel_height": 412.0
            }"#,
        )?;

        assert_eq!(config.tray_style, TrayStyle::Panel);
        assert_eq!(config.github_proxy, "https://gh.example/");
        assert!(!config.close_to_tray);
        assert_eq!(config.tray_panel_height, Some(412.0));

        let numeric: AppConfig =
            serde_json::from_str(r#"{"tray_style": 7, "github_proxy": "https://gh.example/"}"#)?;
        assert_eq!(numeric.tray_style, TrayStyle::Panel);
        assert_eq!(numeric.github_proxy, "https://gh.example/");
        Ok(())
    }

    #[test]
    fn tray_panel_height_defaults_to_none() -> Result<()> {
        let config: AppConfig = serde_json::from_str("{}")?;
        assert_eq!(config.tray_panel_height, None);
        assert_eq!(AppConfig::default().tray_panel_height, None);
        Ok(())
    }

    #[test]
    fn backup_metadata_applies_legacy_defaults() -> Result<()> {
        let metadata: BackupMetadata = toml::from_str(
            r#"
created_at = "2026-01-01T00:00:00Z"
instance_name = "Test"
instance_id = "inst1"
version = "v4.0.0"
"#,
        )?;

        assert!(!metadata.includes_venv);
        assert!(metadata.includes_data);
        assert!(metadata.arch_target.is_empty());
        assert!(!metadata.auto_generated);
        assert!(metadata.note.is_empty());
        Ok(())
    }

    #[test]
    fn backup_info_defaults_and_corrupted_marker_are_preserved() -> Result<()> {
        let info: BackupInfo = serde_json::from_str(
            r#"{
                "filename": "broken.zip",
                "path": "backups/broken.zip",
                "metadata": {
                    "created_at": "",
                    "instance_name": "（损坏备份）",
                    "instance_id": "",
                    "version": ""
                }
            }"#,
        )?;

        assert!(!info.corrupted);
        assert!(info.parse_error.is_none());
        assert_eq!(info.metadata.instance_name, "（损坏备份）");
        assert!(info.metadata.includes_data);

        let corrupted: BackupInfo = serde_json::from_str(
            r#"{
                "filename": "broken.zip",
                "path": "backups/broken.zip",
                "metadata": {
                    "created_at": "",
                    "instance_name": "（损坏备份）",
                    "instance_id": "",
                    "version": ""
                },
                "corrupted": true,
                "parse_error": "invalid backup.toml"
            }"#,
        )?;

        assert!(corrupted.corrupted);
        assert_eq!(
            corrupted.parse_error.as_deref(),
            Some("invalid backup.toml")
        );
        Ok(())
    }

    #[test]
    fn backup_metadata_round_trip_preserves_all_fields() -> Result<()> {
        let metadata = BackupMetadata {
            created_at: "2026-01-01T00:00:00Z".to_string(),
            instance_name: "Test".to_string(),
            instance_id: "inst1".to_string(),
            version: "v4.0.0".to_string(),
            includes_venv: true,
            includes_data: false,
            arch_target: "x86_64-pc-windows-msvc".to_string(),
            auto_generated: true,
            note: "升级前备份".to_string(),
        };

        let decoded: BackupMetadata = toml::from_str(&toml::to_string(&metadata)?)?;

        assert_eq!(decoded.created_at, metadata.created_at);
        assert_eq!(decoded.instance_name, metadata.instance_name);
        assert_eq!(decoded.instance_id, metadata.instance_id);
        assert_eq!(decoded.version, metadata.version);
        assert_eq!(decoded.includes_venv, metadata.includes_venv);
        assert_eq!(decoded.includes_data, metadata.includes_data);
        assert_eq!(decoded.arch_target, metadata.arch_target);
        assert_eq!(decoded.auto_generated, metadata.auto_generated);
        assert_eq!(decoded.note, metadata.note);
        Ok(())
    }

    #[test]
    fn dashboard_credentials_default_to_empty_and_round_trip() -> Result<()> {
        // Manifests saved before these fields existed must keep loading.
        let manifest: AppManifest =
            serde_json::from_str(r#"{"instances":{"id":{"name":"main","version":"v4.29.4"}}}"#)?;
        let instance = manifest
            .instances
            .get("id")
            .ok_or_else(|| AppError::other("instance missing"))?;
        assert_eq!(instance.dashboard_username, "");
        assert_eq!(instance.dashboard_password, "");

        let stored: AppManifest = serde_json::from_str(
            r#"{"instances":{"id":{"name":"main","version":"v4.29.4","dashboard_username":"admin","dashboard_password":"p@ss"}}}"#,
        )?;
        let encoded = serde_json::to_string(&stored)?;
        let decoded: AppManifest = serde_json::from_str(&encoded)?;
        let instance = decoded
            .instances
            .get("id")
            .ok_or_else(|| AppError::other("instance missing"))?;
        assert_eq!(instance.dashboard_username, "admin");
        assert_eq!(instance.dashboard_password, "p@ss");
        Ok(())
    }

    struct ScrubDir(std::path::PathBuf);

    impl ScrubDir {
        fn new(label: &str) -> Result<Self> {
            let path = std::env::temp_dir().join(format!(
                "astrbot-launcher-manifest-scrub-{}-{}",
                label,
                uuid::Uuid::new_v4()
            ));
            fs::create_dir_all(&path)?;
            Ok(Self(path))
        }

        fn file(&self, label: &str) -> std::path::PathBuf {
            self.0.join(format!("{label}.json"))
        }
    }

    impl Drop for ScrubDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn openapi_key_scrub_removes_stale_secrets_and_keeps_other_fields() -> Result<()> {
        let dir = ScrubDir::new("strip")?;
        let path = dir.file("manifest");
        fs::write(
            &path,
            r#"{"instances":{
                "a":{"name":"main","version":"v4.29.4","openapi_key":"abk_a","dashboard_username":"admin"},
                "b":{"name":"alt","version":"v4.30.0","openapi_key":"abk_b","dashboard_password":"p"}
            },"installed_versions":[],"tracked_instances_snapshot":["a"]}"#,
        )?;

        assert_eq!(scrub_openapi_keys_at(&path)?, 2);

        let rewritten: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path)?)?;
        let instances = rewritten
            .get("instances")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| AppError::other("instances missing"))?;
        for instance in instances.values() {
            assert!(instance.get("openapi_key").is_none(), "stale key survived");
        }
        assert_eq!(instances["a"]["name"], "main");
        assert_eq!(instances["a"]["dashboard_username"], "admin");
        assert_eq!(instances["b"]["dashboard_password"], "p");
        assert_eq!(
            rewritten["tracked_instances_snapshot"],
            serde_json::json!(["a"])
        );
        Ok(())
    }

    #[test]
    fn openapi_key_scrub_leaves_clean_manifests_untouched() -> Result<()> {
        let dir = ScrubDir::new("clean")?;
        let path = dir.file("manifest");
        let original = r#"{"instances":{"a":{"name":"main","version":"v4.29.4"}}}"#;
        fs::write(&path, original)?;

        assert_eq!(scrub_openapi_keys_at(&path)?, 0);
        assert_eq!(fs::read_to_string(&path)?, original);
        Ok(())
    }

    #[test]
    fn network_source_fields_default_to_empty() -> Result<()> {
        let config: AppConfig = serde_json::from_str("{}")?;
        assert!(config.python_mirror.is_empty());
        assert!(config.uv_mirror.is_empty());
        assert!(!config.allow_local_dns_probe);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn launcher_records_are_readable_by_the_owner_only() -> Result<()> {
        use std::os::unix::fs::PermissionsExt as _;

        let path = std::env::temp_dir().join(format!(
            "astrbot-launcher-record-{}.json",
            uuid::Uuid::new_v4()
        ));
        fs::write(&path, b"{}")?;

        restrict_record_permissions(&path)?;

        let mode = fs::metadata(&path)?.permissions().mode() & 0o777;
        let _ = fs::remove_file(&path);
        assert_eq!(mode, 0o600);
        Ok(())
    }

    #[test]
    fn language_preference_defaults_to_system() -> Result<()> {
        let config: AppConfig = serde_json::from_str("{}")?;
        assert_eq!(config.language_preference, LanguagePreference::System);
        Ok(())
    }

    #[test]
    fn language_preference_round_trips_exact_tags() -> Result<()> {
        let cases = [
            (LanguagePreference::System, "system"),
            (LanguagePreference::ZhCn, "zh-CN"),
            (LanguagePreference::EnUs, "en-US"),
            (LanguagePreference::RuRu, "ru-RU"),
            (LanguagePreference::JaJp, "ja-JP"),
        ];

        for (preference, tag) in cases {
            let encoded = serde_json::to_string(&preference)?;
            assert_eq!(encoded, format!("\"{tag}\""));
            let decoded: LanguagePreference = serde_json::from_str(&encoded)?;
            assert_eq!(decoded, preference);
        }
        Ok(())
    }
}
