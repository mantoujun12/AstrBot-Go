use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::OnceCell;
use redb::{Database, ReadableDatabase as _, ReadableTable as _, TableDefinition};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::error::{AppError, Result};
use crate::network_config::apply_legacy_mainland_presets;
use crate::utils::paths::{data_db_path, ensure_data_dirs};

static CONFIG_DB: OnceCell<Database> = OnceCell::new();
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
        let value = String::deserialize(deserializer)?;
        // Unknown values from a newer build must not invalidate the whole
        // config record, so they degrade to the default style.
        Ok(match value.as_str() {
            "native_menu" => Self::NativeMenu,
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
            autostart_minimize_to_tray: false,
            check_instance_update: true,
            persist_instance_state: false,
            ignore_external_path: false,
            use_short_instance_id: false,
            lock_check_extension_whitelist: false,
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

fn open_config_db() -> Result<Database> {
    ensure_data_dirs()?;
    let db = Database::create(data_db_path()).map_err(map_redb_error)?;
    let write_txn = db.begin_write().map_err(map_redb_error)?;
    {
        write_txn.open_table(CONFIG_TABLE).map_err(map_redb_error)?;
        write_txn
            .open_table(MANIFEST_TABLE)
            .map_err(map_redb_error)?;
    }
    write_txn.commit().map_err(map_redb_error)?;
    Ok(db)
}

fn config_db() -> Result<&'static Database> {
    CONFIG_DB.get_or_try_init(open_config_db)
}

fn serialize_value<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|e| AppError::config(e.to_string()))
}

fn deserialize_value<T: DeserializeOwned>(raw: &[u8]) -> Result<T> {
    serde_json::from_slice(raw).map_err(|e| AppError::config(e.to_string()))
}

fn insert_manifest_value(value: &AppManifest) -> Result<()> {
    let payload = serialize_value(value)?;
    let db = config_db()?;
    let write_txn = db.begin_write().map_err(map_redb_error)?;
    {
        let mut table = write_txn
            .open_table(MANIFEST_TABLE)
            .map_err(map_redb_error)?;
        table
            .insert(&ROOT_ROW_KEY, payload.as_slice())
            .map_err(map_redb_error)?;
    }
    write_txn.commit().map_err(map_redb_error)?;
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

fn classify_config_record(raw: Option<&[u8]>) -> StoredConfig {
    let Some(raw) = raw else {
        return StoredConfig::Missing;
    };

    let config = match deserialize_value::<AppConfig>(raw) {
        Ok(config) => config,
        Err(error) => return StoredConfig::Corrupted(error.to_string()),
    };
    let legacy = serde_json::from_slice::<LegacyNetworkConfig>(raw).unwrap_or_default();

    if legacy.mainland_acceleration {
        StoredConfig::NeedsMigration(config)
    } else {
        StoredConfig::Ready(config)
    }
}

fn read_config_record(db: &Database) -> Result<Option<Vec<u8>>> {
    let read_txn = db.begin_read().map_err(map_redb_error)?;
    let table = read_txn.open_table(CONFIG_TABLE).map_err(map_redb_error)?;
    let raw = table.get(&ROOT_ROW_KEY).map_err(map_redb_error)?;
    Ok(raw.map(|raw| raw.value().to_vec()))
}

/// Migrate the legacy mainland switch or initialize a missing record.
///
/// The record is read again inside the write transaction so a concurrent save
/// cannot be overwritten by a stale migration.
fn rewrite_config_record(db: &Database) -> Result<AppConfig> {
    let write_txn = db.begin_write().map_err(map_redb_error)?;

    let config = {
        let mut table = write_txn.open_table(CONFIG_TABLE).map_err(map_redb_error)?;
        let raw = table
            .get(&ROOT_ROW_KEY)
            .map_err(map_redb_error)?
            .map(|raw| raw.value().to_vec());

        let (config, should_write) = match classify_config_record(raw.as_deref()) {
            StoredConfig::Ready(config) => (config, false),
            StoredConfig::NeedsMigration(mut config) => {
                apply_legacy_mainland_presets(&mut config);
                log::info!("Migrated legacy mainland acceleration to per-source settings");
                (config, true)
            }
            StoredConfig::Corrupted(error) => {
                log::warn!(
                    "Config record is corrupted JSON, resetting to default value: {}",
                    error
                );
                (AppConfig::default(), true)
            }
            StoredConfig::Missing => (AppConfig::default(), true),
        };

        if should_write {
            let payload = serialize_value(&config)?;
            table
                .insert(&ROOT_ROW_KEY, payload.as_slice())
                .map_err(map_redb_error)?;
        }

        config
    };

    write_txn.commit().map_err(map_redb_error)?;
    Ok(config)
}

fn load_or_init_config_value() -> Result<AppConfig> {
    let db = config_db()?;

    match classify_config_record(read_config_record(db)?.as_deref()) {
        StoredConfig::Ready(config) => Ok(config),
        StoredConfig::NeedsMigration(_) | StoredConfig::Corrupted(_) | StoredConfig::Missing => {
            rewrite_config_record(db)
        }
    }
}

fn load_or_init_manifest_value() -> Result<AppManifest> {
    let stored = {
        let db = config_db()?;
        let read_txn = db.begin_read().map_err(map_redb_error)?;
        let table = read_txn
            .open_table(MANIFEST_TABLE)
            .map_err(map_redb_error)?;
        table
            .get(&ROOT_ROW_KEY)
            .map_err(map_redb_error)?
            .map(|raw| deserialize_value::<AppManifest>(raw.value()))
    };

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
    insert_manifest_value(&default)?;
    Ok(default)
}

fn with_config_value_mut<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&mut AppConfig) -> Result<R>,
{
    let db = config_db()?;
    let write_txn = db.begin_write().map_err(map_redb_error)?;

    let mut config = {
        let table = write_txn.open_table(CONFIG_TABLE).map_err(map_redb_error)?;
        let existing = table.get(&ROOT_ROW_KEY).map_err(map_redb_error)?;
        let raw = existing.as_ref().map(|raw| raw.value());
        match classify_config_record(raw) {
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
        }
    };

    let result = f(&mut config)?;
    let payload = serialize_value(&config)?;

    {
        let mut table = write_txn.open_table(CONFIG_TABLE).map_err(map_redb_error)?;
        table
            .insert(&ROOT_ROW_KEY, payload.as_slice())
            .map_err(map_redb_error)?;
    }
    write_txn.commit().map_err(map_redb_error)?;

    Ok(result)
}

fn with_manifest_value_mut<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&mut AppManifest) -> Result<R>,
{
    let db = config_db()?;
    let write_txn = db.begin_write().map_err(map_redb_error)?;

    let mut manifest = {
        let table = write_txn
            .open_table(MANIFEST_TABLE)
            .map_err(map_redb_error)?;
        let existing = table.get(&ROOT_ROW_KEY).map_err(map_redb_error)?;
        if let Some(raw) = existing {
            match deserialize_value::<AppManifest>(raw.value()) {
                Ok(manifest) => manifest,
                Err(error) => {
                    log::warn!(
                        "Manifest record is corrupted JSON, replacing with default before update: {}",
                        error
                    );
                    AppManifest::default()
                }
            }
        } else {
            AppManifest::default()
        }
    };

    let result = f(&mut manifest)?;
    let payload = serialize_value(&manifest)?;

    {
        let mut table = write_txn
            .open_table(MANIFEST_TABLE)
            .map_err(map_redb_error)?;
        table
            .insert(&ROOT_ROW_KEY, payload.as_slice())
            .map_err(map_redb_error)?;
    }
    write_txn.commit().map_err(map_redb_error)?;

    Ok(result)
}

fn has_config_record_value() -> Result<bool> {
    let db = config_db()?;
    let read_txn = db.begin_read().map_err(map_redb_error)?;
    let table = read_txn.open_table(CONFIG_TABLE).map_err(map_redb_error)?;
    let value = table.get(&ROOT_ROW_KEY).map_err(map_redb_error)?;
    Ok(value.is_some())
}

fn has_manifest_record_value() -> Result<bool> {
    let db = config_db()?;
    let read_txn = db.begin_read().map_err(map_redb_error)?;
    let table = read_txn
        .open_table(MANIFEST_TABLE)
        .map_err(map_redb_error)?;
    let value = table.get(&ROOT_ROW_KEY).map_err(map_redb_error)?;
    Ok(value.is_some())
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
        // config record, which classify_config_record would reset.
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
        let legacy = br#"{"mainland_acceleration":true,"nodejs_mirror":"https://custom.example/"}"#;
        let needs_migration = classify_config_record(Some(legacy));
        assert!(matches!(needs_migration, StoredConfig::NeedsMigration(_)));
        if let StoredConfig::NeedsMigration(config) = needs_migration {
            assert!(config.github_proxy.is_empty());
            assert_eq!(config.nodejs_mirror, "https://custom.example/");
        }

        let migrated =
            br#"{"mainland_acceleration":false,"nodejs_mirror":"https://custom.example/"}"#;
        let ready = classify_config_record(Some(migrated));
        assert!(matches!(ready, StoredConfig::Ready(_)));
        if let StoredConfig::Ready(config) = ready {
            assert_eq!(config.nodejs_mirror, "https://custom.example/");
        }

        assert!(matches!(
            classify_config_record(None),
            StoredConfig::Missing
        ));
        assert!(matches!(
            classify_config_record(Some(b"not json")),
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
