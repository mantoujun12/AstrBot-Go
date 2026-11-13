use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, LazyLock, Mutex};

use redb::{Database, ReadableDatabase as _, TableDefinition};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::error::{AppError, Result};
use crate::network_config::apply_legacy_mainland_presets;
use crate::utils::paths::{config_json_path, data_db_path, ensure_data_dirs, manifest_json_path};

static STORE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

// Legacy `data.redb` table definitions, kept for the one-time migration into
// the JSON config files.
const CONFIG_TABLE: TableDefinition<u8, &[u8]> = TableDefinition::new("app_config");
const MANIFEST_TABLE: TableDefinition<u8, &[u8]> = TableDefinition::new("app_manifest");
const ROOT_ROW_KEY: u8 = 0;

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

fn map_redb_error(err: impl std::fmt::Display) -> AppError {
    AppError::config(err.to_string())
}

fn deserialize_value<T: DeserializeOwned>(raw: &[u8]) -> Result<T> {
    serde_json::from_slice(raw).map_err(|e| AppError::config(e.to_string()))
}

/// Serialize and write a JSON record atomically (temp file + rename).
fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    ensure_data_dirs()?;
    let payload = serde_json::to_vec_pretty(value).map_err(|e| AppError::config(e.to_string()))?;
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, payload)
        .map_err(|e| AppError::config(format!("write {}: {}", path.display(), e)))?;
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

/// Migrate records stored in the legacy `data.redb` database into
/// `config.json` / `manifest.json`.
///
/// Idempotent: each JSON file is only written when it does not exist yet, and
/// the whole step is skipped when the legacy database is absent.
fn migrate_legacy_redb_records() -> Result<()> {
    let config_path = config_json_path();
    let manifest_path = manifest_json_path();
    if config_path.exists() && manifest_path.exists() {
        return Ok(());
    }
    let db_path = data_db_path();
    if !db_path.exists() {
        return Ok(());
    }

    let db = Database::create(&db_path).map_err(map_redb_error)?;
    let read_txn = db.begin_read().map_err(map_redb_error)?;

    if !config_path.exists() {
        let config = {
            let table = read_txn.open_table(CONFIG_TABLE).map_err(map_redb_error)?;
            let raw = table
                .get(&ROOT_ROW_KEY)
                .map_err(map_redb_error)?
                .map(|guard| guard.value().to_vec());
            let text = raw
                .as_deref()
                .and_then(|bytes| std::str::from_utf8(bytes).ok().map(|text| text.to_string()));
            match classify_config_str(text.as_deref()) {
                StoredConfig::Ready(config) => config,
                StoredConfig::NeedsMigration(mut config) => {
                    apply_legacy_mainland_presets(&mut config);
                    config
                }
                StoredConfig::Corrupted(error) => {
                    log::warn!(
                        "Config record is corrupted JSON, resetting to default value: {}",
                        error
                    );
                    AppConfig::default()
                }
                StoredConfig::Missing => AppConfig::default(),
            }
        };
        write_json_atomic(&config_path, &config)?;
    }

    if !manifest_path.exists() {
        let manifest = {
            let table = read_txn
                .open_table(MANIFEST_TABLE)
                .map_err(map_redb_error)?;
            match table
                .get(&ROOT_ROW_KEY)
                .map_err(map_redb_error)?
                .map(|guard| deserialize_value::<AppManifest>(guard.value()))
            {
                Some(Ok(manifest)) => manifest,
                Some(Err(error)) => {
                    log::warn!(
                        "Manifest record is corrupted JSON, resetting to default value: {}",
                        error
                    );
                    AppManifest::default()
                }
                None => AppManifest::default(),
            }
        };
        write_json_atomic(&manifest_path, &manifest)?;
    }

    log::info!("Migrated legacy data.redb records into config.json and manifest.json");
    Ok(())
}

#[derive(Debug, Default, Deserialize)]
struct LegacyNetworkConfig {
    #[serde(default)]
    mainland_acceleration: bool,
}

/// Stored config record as classified by one transaction.
enum StoredConfig {
    Ready(AppConfig),
    NeedsMigration(AppConfig),
    Corrupted(String),
    Missing,
}

fn classify_config_str(raw: Option<&str>) -> StoredConfig {
    let Some(raw) = raw else {
        return StoredConfig::Missing;
    };

    let config = match serde_json::from_str::<AppConfig>(raw) {
        Ok(config) => config,
        Err(error) => return StoredConfig::Corrupted(error.to_string()),
    };
    let legacy = serde_json::from_str::<LegacyNetworkConfig>(raw).unwrap_or_default();

    if legacy.mainland_acceleration {
        StoredConfig::NeedsMigration(config)
    } else {
        StoredConfig::Ready(config)
    }
}

/// Migrate the legacy mainland switch or initialize a missing record.
fn write_config_record(path: &Path, config: AppConfig) -> Result<AppConfig> {
    write_json_atomic(path, &config)?;
    Ok(config)
}

fn load_or_init_config_value() -> Result<AppConfig> {
    migrate_legacy_redb_records()?;
    let _guard = STORE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let path = config_json_path();
    match classify_config_str(read_json_text(&path)?.as_deref()) {
        StoredConfig::Ready(config) => Ok(config),
        StoredConfig::NeedsMigration(mut config) => {
            apply_legacy_mainland_presets(&mut config);
            log::info!("Migrated legacy mainland acceleration to per-source settings");
            write_config_record(&path, config)
        }
        StoredConfig::Corrupted(error) => {
            log::warn!(
                "Config record is corrupted JSON, resetting to default value: {}",
                error
            );
            write_config_record(&path, AppConfig::default())
        }
        StoredConfig::Missing => write_config_record(&path, AppConfig::default()),
    }
}

fn load_or_init_manifest_value() -> Result<AppManifest> {
    migrate_legacy_redb_records()?;
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
    migrate_legacy_redb_records()?;
    let _guard = STORE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let path = config_json_path();
    let mut config = match classify_config_str(read_json_text(&path)?.as_deref()) {
        StoredConfig::Ready(config) => config,
        StoredConfig::NeedsMigration(mut config) => {
            apply_legacy_mainland_presets(&mut config);
            log::info!("Migrated legacy mainland acceleration to per-source settings");
            config
        }
        StoredConfig::Corrupted(error) => {
            log::warn!(
                "Config record is corrupted JSON, replacing with default before update: {}",
                error
            );
            AppConfig::default()
        }
        StoredConfig::Missing => AppConfig::default(),
    };

    let result = f(&mut config)?;
    write_json_atomic(&path, &config)?;

    Ok(result)
}

fn with_manifest_value_mut<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&mut AppManifest) -> Result<R>,
{
    migrate_legacy_redb_records()?;
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

fn has_config_record_value() -> Result<bool> {
    migrate_legacy_redb_records()?;
    Ok(config_json_path().exists())
}

fn has_manifest_record_value() -> Result<bool> {
    migrate_legacy_redb_records()?;
    Ok(manifest_json_path().exists())
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

pub fn reload_config() -> Result<Arc<AppConfig>> {
    load_config()
}

pub fn load_manifest() -> Result<Arc<AppManifest>> {
    let manifest = load_or_init_manifest_value()?;
    Ok(Arc::new(manifest))
}

pub fn reload_manifest() -> Result<Arc<AppManifest>> {
    load_manifest()
}

pub(crate) fn has_config_record() -> Result<bool> {
    has_config_record_value()
}

pub(crate) fn has_manifest_record() -> Result<bool> {
    has_manifest_record_value()
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
        // config record, which classify_config_str would reset.
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
    fn network_source_fields_default_to_empty() -> Result<()> {
        let config: AppConfig = serde_json::from_str("{}")?;
        assert!(config.python_mirror.is_empty());
        assert!(config.uv_mirror.is_empty());
        assert!(!config.allow_local_dns_probe);
        Ok(())
    }

    #[test]
    fn legacy_mainland_flag_is_detected_from_json() -> Result<()> {
        let legacy: LegacyNetworkConfig =
            serde_json::from_str(r#"{"mainland_acceleration":true}"#)?;
        assert!(legacy.mainland_acceleration);
        Ok(())
    }

    #[test]
    fn stored_config_is_classified_for_migration() {
        let legacy = r#"{"mainland_acceleration":true,"nodejs_mirror":"https://custom.example/"}"#;
        let needs_migration = classify_config_str(Some(legacy));
        assert!(matches!(needs_migration, StoredConfig::NeedsMigration(_)));
        if let StoredConfig::NeedsMigration(config) = needs_migration {
            assert!(config.github_proxy.is_empty());
            assert_eq!(config.nodejs_mirror, "https://custom.example/");
        }

        let migrated =
            r#"{"mainland_acceleration":false,"nodejs_mirror":"https://custom.example/"}"#;
        let ready = classify_config_str(Some(migrated));
        assert!(matches!(ready, StoredConfig::Ready(_)));
        if let StoredConfig::Ready(config) = ready {
            assert_eq!(config.nodejs_mirror, "https://custom.example/");
        }

        assert!(matches!(classify_config_str(None), StoredConfig::Missing));
        assert!(matches!(
            classify_config_str(Some("not json")),
            StoredConfig::Corrupted(_)
        ));
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
