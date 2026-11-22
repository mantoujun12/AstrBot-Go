//! Offline plugin management for AstrBot instances.
//!
//! Mirrors upstream AstrBot: plugins live in `data/plugins/<dir>` with
//! `metadata.yaml` and an entry module (`main.py` or `<dir>.py`), and the
//! enabled state is the module path stored in the `inactivated_plugins`
//! preference of `data_v4.db` (scope `global`). All edits are offline: every
//! write command holds the instance lifecycle guard, so a running instance
//! rejects the operation before it can be clobbered on exit.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};

use crate::archive::{extract_zip_flat, validate_zip_archive};
use crate::error::{AppError, Result};
use crate::instance::data_manage::{db_err, ensure_table, open_db};
use crate::utils::log_redact;
use crate::utils::net::{ensure_success_status, fetch_json, send_get};

/// Official AstrBot plugin market (AstrBot Cloud).
const MARKET_URL: &str = "https://cloud.astrbot.app/api/v1/market/plugins";
/// Plugins are small source archives; anything larger is rejected.
const DOWNLOAD_MAX_BYTES: u64 = 256 * 1024 * 1024;
const MARKET_MAX_PAGE_SIZE: i64 = 100;
/// Staging prefix matched by upstream's plugin discovery, which skips it.
const STAGING_PREFIX: &str = ".plugin-install-";
const BACKUP_PREFIX: &str = ".plugin-backup-";

// === Market ==================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMarketCategory {
    pub name: String,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMarketItem {
    pub plugin_id: String,
    pub author: String,
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub repository_url: Option<String>,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub logo_url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub astrbot_version: Option<String>,
    #[serde(default)]
    pub official: bool,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub downloads: i64,
    #[serde(default)]
    pub github_stars: i64,
    #[serde(default)]
    pub latest_version: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMarketPage {
    pub categories: Vec<PluginMarketCategory>,
    #[serde(default)]
    pub category_counts: std::collections::BTreeMap<String, i64>,
    pub items: Vec<PluginMarketItem>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
struct MarketEnvelope {
    code: i64,
    #[serde(default)]
    msg: String,
    data: PluginMarketPage,
}

/// Fetch one page of the official plugin market.
pub async fn fetch_market_page(
    client: &Client,
    page: i64,
    page_size: i64,
    search: Option<&str>,
    category: Option<&str>,
) -> Result<PluginMarketPage> {
    let page = page.max(1);
    let page_size = page_size.clamp(1, MARKET_MAX_PAGE_SIZE);

    let mut url = reqwest::Url::parse(MARKET_URL)
        .map_err(|e| AppError::network(format!("插件市场地址无效：{e}")))?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("page", &page.to_string());
        pairs.append_pair("page_size", &page_size.to_string());
        if let Some(search) = search.map(str::trim).filter(|s| !s.is_empty()) {
            pairs.append_pair("search", search);
        }
        if let Some(category) = category.map(str::trim).filter(|s| !s.is_empty()) {
            pairs.append_pair("category", category);
        }
    }

    let envelope: MarketEnvelope = fetch_json(client, url.as_str()).await?;
    if envelope.code != 200 {
        return Err(AppError::network(format!(
            "插件市场返回错误 {}：{}",
            envelope.code,
            envelope.msg.trim()
        )));
    }
    Ok(envelope.data)
}

// === Installed plugins =======================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstancePlugin {
    /// On-disk directory name under `data/plugins`.
    pub dir_name: String,
    /// `metadata.yaml.name`; falls back to the directory name.
    pub name: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub desc: Option<String>,
    #[serde(default)]
    pub repo: Option<String>,
    #[serde(default)]
    pub astrbot_version: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub has_metadata: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginInstallRequest {
    /// Market download archive URL. Identity is decided by the archive's own
    /// `metadata.yaml`, matching the upstream install flow.
    pub download_url: String,
}

/// Minimal `metadata.yaml` subset used by the launcher.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct PluginMetadata {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    desc: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    repo: Option<String>,
    #[serde(default)]
    astrbot_version: Option<String>,
}

fn read_metadata(plugin_dir: &Path) -> Result<Option<PluginMetadata>> {
    for file in ["metadata.yaml", "metadata.yml"] {
        let path = plugin_dir.join(file);
        if !path.is_file() {
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| AppError::io(format!("读取插件元数据失败：{e}")))?;
        let metadata: PluginMetadata = serde_yaml_ng::from_str(&content)
            .map_err(|e| AppError::io(format!("插件元数据格式错误：{e}")))?;
        let mut metadata = metadata;
        // Upstream accepts `description` as an alias for `desc`.
        if metadata.desc.is_none() {
            metadata.desc = metadata.description.take();
        }
        return Ok(Some(metadata));
    }
    Ok(None)
}

/// Plugin directory names must stay importable module names, matching
/// upstream's `main.py`/`<dir>.py` discovery and `metadata.yaml.name` rules.
pub(crate) fn validate_plugin_dir_name(name: &str) -> Result<()> {
    let invalid = || {
        AppError::io(format!(
            "插件目录名不合法：{name:?}（应为字母或下划线开头的字母、数字、下划线组合）"
        ))
    };
    if name.is_empty()
        || name.starts_with('.')
        || name.contains(['/', '\\', ':'])
        || name.contains('\0')
    {
        return Err(invalid());
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => (),
        _ => return Err(invalid()),
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(invalid());
    }
    Ok(())
}

/// The module path upstream persists in `inactivated_plugins`.
fn module_path_for(plugin_dir: &Path, dir_name: &str) -> String {
    let entry = if plugin_dir.join("main.py").exists() {
        "main"
    } else {
        dir_name
    };
    format!("data.plugins.{dir_name}.{entry}")
}

/// List the installed plugins of an instance. Staged and backup directories
/// are invisible to upstream's loader and stay invisible here.
pub fn list_plugins(data_dir: &Path) -> Result<Vec<InstancePlugin>> {
    let plugins_dir = data_dir.join("plugins");
    let mut plugins = Vec::new();
    if !plugins_dir.is_dir() {
        return Ok(plugins);
    }

    let inactivated = read_inactivated(data_dir);

    for entry in
        fs::read_dir(&plugins_dir).map_err(|e| AppError::io(format!("读取插件目录失败：{e}")))?
    {
        let entry = entry.map_err(|e| AppError::io(format!("读取插件目录失败：{e}")))?;
        let dir_name = entry.file_name().to_string_lossy().into_owned();
        if dir_name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        // Upstream only loads dirs with main.py or <dir>.py.
        if !path.join("main.py").is_file() && !path.join(format!("{dir_name}.py")).is_file() {
            continue;
        }
        let module_path = module_path_for(&path, &dir_name);
        let metadata = read_metadata(&path)?;
        let name = metadata
            .as_ref()
            .and_then(|m| m.name.clone())
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| dir_name.clone());
        let author = metadata
            .as_ref()
            .and_then(|m| m.author.clone())
            .filter(|a| !a.trim().is_empty());
        plugins.push(InstancePlugin {
            dir_name,
            name,
            author,
            version: metadata.as_ref().and_then(|m| m.version.clone()),
            desc: metadata.as_ref().and_then(|m| m.desc.clone()),
            repo: metadata.as_ref().and_then(|m| m.repo.clone()),
            astrbot_version: metadata.as_ref().and_then(|m| m.astrbot_version.clone()),
            enabled: !inactivated.contains(&module_path),
            has_metadata: metadata.is_some(),
        });
    }

    plugins.sort_by_key(|a| a.name.to_lowercase());
    Ok(plugins)
}

fn find_plugin_dir(data_dir: &Path, dir_name: &str) -> Result<PathBuf> {
    validate_plugin_dir_name(dir_name)?;
    let dir = data_dir.join("plugins").join(dir_name);
    if !dir.is_dir() {
        return Err(AppError::io(format!("插件 {dir_name} 不存在。")));
    }
    Ok(dir)
}

/// Enable or disable a plugin by updating the `inactivated_plugins` preference.
pub fn set_plugin_enabled(data_dir: &Path, dir_name: &str, enabled: bool) -> Result<()> {
    let dir = find_plugin_dir(data_dir, dir_name)?;
    let module_path = module_path_for(&dir, dir_name);
    let conn = open_db(data_dir)?;
    ensure_table(&conn, "preferences")?;

    let inactivated = read_inactivated_with(&conn)?;
    let updated = if enabled {
        inactivated
            .into_iter()
            .filter(|path| path != &module_path)
            .collect::<Vec<_>>()
    } else if inactivated.contains(&module_path) {
        inactivated
    } else {
        let mut updated = inactivated;
        updated.push(module_path);
        updated
    };
    if updated.is_empty() {
        conn.execute(
            "DELETE FROM preferences WHERE scope = 'global' AND scope_id = 'global' AND key = 'inactivated_plugins'",
            [],
        )
        .map_err(db_err)?;
    } else {
        write_inactivated(&conn, &updated)?;
    }
    Ok(())
}

/// Uninstall a plugin; config and persistent data are optional side deletes.
pub fn uninstall_plugin(
    data_dir: &Path,
    dir_name: &str,
    delete_config: bool,
    delete_data: bool,
) -> Result<()> {
    let dir = find_plugin_dir(data_dir, dir_name)?;
    let metadata = read_metadata(&dir)?;
    let data_name = metadata
        .as_ref()
        .and_then(|m| m.name.clone())
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| dir_name.to_string());

    fs::remove_dir_all(&dir).map_err(|e| AppError::io(format!("删除插件失败：{e}")))?;

    if delete_config {
        let config = data_dir
            .join("config")
            .join(format!("{dir_name}_config.json"));
        if config.is_file() {
            fs::remove_file(&config).map_err(|e| AppError::io(format!("删除插件配置失败：{e}")))?;
        }
    }
    if delete_data {
        let plugin_data = data_dir.join("plugin_data").join(&data_name);
        if plugin_data.is_dir() {
            fs::remove_dir_all(&plugin_data)
                .map_err(|e| AppError::io(format!("删除插件数据失败：{e}")))?;
        }
    }

    // Drop the disabled marker so a later reinstall starts from a clean state.
    forget_disabled_marker(data_dir, dir_name);
    Ok(())
}

fn forget_disabled_marker(data_dir: &Path, dir_name: &str) {
    if !conn_supported(data_dir) {
        return;
    }
    let Ok(conn) = open_db(data_dir) else {
        return;
    };
    if ensure_table(&conn, "preferences").is_err() {
        return;
    }
    let Ok(mut inactivated) = read_inactivated_with(&conn) else {
        return;
    };
    let before = inactivated.len();
    inactivated.retain(|path| {
        path != &format!("data.plugins.{dir_name}.main")
            && path != &format!("data.plugins.{dir_name}.{dir_name}")
    });
    if inactivated.len() != before {
        let _ = write_inactivated(&conn, &inactivated);
    }
}

/// Download a market plugin archive and move it into `data/plugins/<name>`,
/// matching upstream: the archive's own `metadata.yaml` decides the plugin
/// identity, and an existing directory is only replaced when it is the same
/// plugin.
pub async fn install_plugin(
    client: &Client,
    data_dir: &Path,
    request: &PluginInstallRequest,
) -> Result<InstancePlugin> {
    let download_url = request.download_url.trim();
    let parsed =
        reqwest::Url::parse(download_url).map_err(|_| AppError::network("插件下载地址无效。"))?;
    if parsed.scheme() != "https" {
        return Err(AppError::network("插件下载地址必须为 HTTPS。"));
    }

    let staging = data_dir
        .join("plugins")
        .join(format!("{STAGING_PREFIX}{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&staging).map_err(|e| AppError::io(format!("创建暂存目录失败：{e}")))?;
    let zip_path = staging.join("plugin.zip");
    download_archive(client, download_url, &zip_path).await?;

    // Extraction and the directory swap are pure blocking work.
    let blocking_data_dir = data_dir.to_path_buf();
    let blocking_staging = staging.clone();
    let join = tokio::task::spawn_blocking(move || {
        finalize_install(&blocking_data_dir, &blocking_staging)
    })
    .await
    .map_err(|e| AppError::process(format!("插件安装后台任务失败：{e}")));

    let result = match join {
        Ok(inner) => inner,
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
    };
    let _ = fs::remove_dir_all(&staging);
    result
}

/// Mirrors upstream's install guard: only replace a `data/plugins/<name>`
/// directory whose own metadata identifies the same plugin.
fn ensure_replaceable(target: &Path, new_name: &str) -> Result<()> {
    if !target.is_dir() {
        return Ok(());
    }
    let existing_name = read_metadata(target)?
        .and_then(|metadata| metadata.name)
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty());
    if existing_name.as_deref() == Some(new_name) {
        return Ok(());
    }
    Err(AppError::io(format!(
        "安装失败：目录 {new_name} 已存在，且不是同一插件。"
    )))
}

fn finalize_install(data_dir: &Path, staging: &Path) -> Result<InstancePlugin> {
    let zip_path = staging.join("plugin.zip");
    validate_zip_archive(&zip_path)?;
    let extracted = staging.join("extracted");
    extract_zip_flat(&zip_path, &extracted)?;
    let plugin_root = locate_plugin_root(&extracted)?;

    let metadata = read_metadata(&plugin_root)?
        .ok_or_else(|| AppError::io("压缩包内缺少 metadata.yaml，不是有效的 AstrBot 插件。"))?;
    let name = metadata
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .ok_or_else(|| AppError::io("插件 metadata.yaml 缺少 name 字段。"))?;
    validate_plugin_dir_name(name)?;

    let target = data_dir.join("plugins").join(name);
    // `data/plugins/<name>` always has a parent, but stay defensive.
    let Some(parent) = target.parent() else {
        return Err(AppError::io("插件目录路径无效。"));
    };
    fs::create_dir_all(parent).map_err(|e| AppError::io(format!("创建插件目录失败：{e}")))?;
    ensure_replaceable(&target, name)?;

    // Replace an existing install behind a backup so a failure can roll back.
    let backup = data_dir
        .join("plugins")
        .join(format!("{BACKUP_PREFIX}{}", uuid::Uuid::new_v4()));
    let had_previous = target.exists();
    if had_previous {
        fs::rename(&target, &backup).map_err(|e| AppError::io(format!("备份旧插件失败：{e}")))?;
    }
    if let Err(error) = fs::rename(plugin_root, &target) {
        if had_previous {
            let _ = fs::rename(&backup, &target);
        }
        return Err(AppError::io(format!("移动插件目录失败：{error}")));
    }
    if had_previous {
        let _ = fs::remove_dir_all(&backup);
    }

    let inactivated = read_inactivated(data_dir);
    let author = metadata
        .author
        .as_deref()
        .map(str::trim)
        .filter(|author| !author.is_empty())
        .map(str::to_string);
    Ok(InstancePlugin {
        dir_name: name.to_string(),
        name: name.to_string(),
        author,
        version: metadata.version,
        desc: metadata.desc,
        repo: metadata.repo,
        astrbot_version: metadata.astrbot_version,
        enabled: !inactivated.contains(&module_path_for(&target, name)),
        has_metadata: true,
    })
}

async fn download_archive(client: &Client, url: &str, dest: &Path) -> Result<()> {
    log::info!(
        "Downloading plugin archive from {}",
        log_redact::url_for_log(url)
    );
    let resp = send_get(client, url, false)
        .await
        .map_err(|e| AppError::network(format!("下载插件失败：{}", e)))?;
    ensure_success_status(&resp, |detail| {
        AppError::network(format!("下载插件失败：HTTP {detail}"))
    })?;

    let mut file =
        fs::File::create(dest).map_err(|e| AppError::io(format!("创建下载文件失败：{e}")))?;
    let mut downloaded: u64 = 0;
    let mut resp = resp;
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| AppError::network(format!("下载插件失败：{e}")))?
    {
        downloaded += chunk.len() as u64;
        if downloaded > DOWNLOAD_MAX_BYTES {
            return Err(AppError::network("插件压缩包超出大小限制。"));
        }
        file.write_all(&chunk)
            .map_err(|e| AppError::io(format!("写入下载文件失败：{e}")))?;
    }
    if downloaded == 0 {
        return Err(AppError::network("插件压缩包为空。"));
    }
    Ok(())
}

/// Accept both archive layouts: `metadata.yaml` at the root (the top dir was
/// already stripped) or a single wrapping directory.
fn locate_plugin_root(extracted: &Path) -> Result<PathBuf> {
    if extracted.join("metadata.yaml").is_file() || extracted.join("metadata.yml").is_file() {
        return Ok(extracted.to_path_buf());
    }
    let candidates: Vec<PathBuf> = fs::read_dir(extracted)
        .map_err(|e| AppError::io(format!("读取解压结果失败：{e}")))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && (path.join("metadata.yaml").is_file() || path.join("metadata.yml").is_file())
        })
        .collect();
    match candidates.as_slice() {
        [only] => Ok(only.clone()),
        [] => Err(AppError::io(
            "压缩包内缺少 metadata.yaml，不是有效的 AstrBot 插件。",
        )),
        _ => Err(AppError::io("压缩包内包含多个插件目录，无法确定安装目标。")),
    }
}

// === Preference storage ======================================================

fn conn_supported(data_dir: &Path) -> bool {
    data_dir.join("data_v4.db").is_file()
}

/// `inactivated_plugins` from the preferences table; a missing database or
/// table simply means "nothing disabled".
fn read_inactivated(data_dir: &Path) -> Vec<String> {
    if !conn_supported(data_dir) {
        return Vec::new();
    }
    let Ok(conn) = open_db(data_dir) else {
        return Vec::new();
    };
    if ensure_table(&conn, "preferences").is_err() {
        return Vec::new();
    }
    read_inactivated_with(&conn).unwrap_or_default()
}

fn read_inactivated_with(conn: &rusqlite::Connection) -> Result<Vec<String>> {
    use rusqlite::OptionalExtension as _;

    let raw: Option<String> = conn
        .query_row(
            "SELECT value FROM preferences WHERE scope = 'global' AND scope_id = 'global' AND key = 'inactivated_plugins'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(db_err)?;
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    let parsed: JsonValue = serde_json::from_str(&raw).unwrap_or(JsonValue::Null);
    let Some(items) = parsed.get("val").and_then(JsonValue::as_array) else {
        return Ok(Vec::new());
    };
    Ok(items
        .iter()
        .filter_map(JsonValue::as_str)
        .map(str::to_string)
        .collect())
}

fn write_inactivated(conn: &rusqlite::Connection, paths: &[String]) -> Result<()> {
    let value = json!({ "val": paths }).to_string();
    let now = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S%.6f")
        .to_string();
    conn.execute(
        "INSERT INTO preferences (scope, scope_id, key, value, created_at, updated_at)
         VALUES ('global', 'global', 'inactivated_plugins', ?1, ?2, ?2)
         ON CONFLICT (scope, scope_id, key)
         DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        rusqlite::params![value, now],
    )
    .map_err(db_err)?;
    Ok(())
}

// === Tests ===================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::data_manage::BUSY_TIMEOUT;
    use rusqlite::Connection;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Result<Self> {
            let dir = std::env::temp_dir()
                .join(format!("astrbot-go-plugins-{tag}-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&dir).map_err(|error| AppError::io(error.to_string()))?;
            Ok(Self(dir))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn open_test_db(path: &Path) -> Result<Connection> {
        let conn = Connection::open(path.join("data_v4.db")).map_err(db_err)?;
        conn.execute_batch(
            "CREATE TABLE preferences (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                scope TEXT NOT NULL,
                scope_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                created_at TEXT,
                updated_at TEXT,
                UNIQUE (scope, scope_id, key)
            );",
        )
        .map_err(db_err)?;
        conn.busy_timeout(BUSY_TIMEOUT).map_err(db_err)?;
        Ok(conn)
    }

    fn write_plugin(data_dir: &Path, name: &str, metadata: &str) -> Result<PathBuf> {
        let dir = data_dir.join("plugins").join(name);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("main.py"), "")?;
        fs::write(dir.join("metadata.yaml"), metadata)?;
        Ok(dir)
    }

    #[test]
    fn dir_names_are_validated() {
        for name in ["my_plugin", "_x", "A1"] {
            assert!(validate_plugin_dir_name(name).is_ok(), "{name}");
        }
        for name in ["", ".hidden", "a/b", "a\\b", "1abc", "a-b", "..", "a:b"] {
            assert!(validate_plugin_dir_name(name).is_err(), "{name}");
        }
    }

    #[test]
    fn module_path_follows_entry_file() -> Result<()> {
        let dir = TempDir::new("module-path")?;
        let plain = dir.path().join("plugins/a_plugin");
        fs::create_dir_all(&plain)?;
        assert_eq!(
            module_path_for(&plain, "a_plugin"),
            "data.plugins.a_plugin.a_plugin"
        );
        fs::write(plain.join("main.py"), "")?;
        assert_eq!(
            module_path_for(&plain, "a_plugin"),
            "data.plugins.a_plugin.main"
        );
        Ok(())
    }

    #[test]
    fn list_reports_metadata_and_enabled_state() -> Result<()> {
        let dir = TempDir::new("list")?;
        write_plugin(
            dir.path(),
            "my_plugin",
            "name: my_plugin\nauthor: anka\nversion: 1.0.0\ndesc: hello\n",
        )?;
        let conn = open_test_db(dir.path())?;
        write_inactivated(&conn, &["data.plugins.my_plugin.main".to_string()])?;

        let plugins = list_plugins(dir.path())?;
        assert_eq!(plugins.len(), 1);
        let plugin = &plugins[0];
        assert_eq!(plugin.name, "my_plugin");
        assert_eq!(plugin.author.as_deref(), Some("anka"));
        assert_eq!(plugin.version.as_deref(), Some("1.0.0"));
        assert!(!plugin.enabled);
        Ok(())
    }

    #[test]
    fn staging_dirs_are_invisible() -> Result<()> {
        let dir = TempDir::new("staging")?;
        write_plugin(
            dir.path(),
            &format!("{STAGING_PREFIX}abc"),
            "name: x\nauthor: y\n",
        )?;
        assert!(list_plugins(dir.path())?.is_empty());
        Ok(())
    }

    #[test]
    fn enable_disable_round_trips_through_preferences() -> Result<()> {
        let dir = TempDir::new("toggle")?;
        write_plugin(dir.path(), "my_plugin", "name: my_plugin\nauthor: anka\n")?;
        let conn = open_test_db(dir.path())?;

        set_plugin_enabled(dir.path(), "my_plugin", false)?;
        assert!(!read_inactivated_with(&conn)?.is_empty());
        assert!(!list_plugins(dir.path())?[0].enabled);

        set_plugin_enabled(dir.path(), "my_plugin", false)?;
        assert_eq!(read_inactivated_with(&conn)?.len(), 1);

        set_plugin_enabled(dir.path(), "my_plugin", true)?;
        assert!(read_inactivated_with(&conn)?.is_empty());
        assert!(list_plugins(dir.path())?[0].enabled);
        Ok(())
    }

    #[test]
    fn uninstall_removes_plugin_and_optional_side_data() -> Result<()> {
        let dir = TempDir::new("uninstall")?;
        write_plugin(dir.path(), "my_plugin", "name: my_plugin\nauthor: anka\n")?;
        fs::create_dir_all(dir.path().join("plugin_data").join("my_plugin"))?;
        fs::create_dir_all(dir.path().join("config"))?;
        fs::write(
            dir.path().join("config").join("my_plugin_config.json"),
            "{}",
        )?;

        uninstall_plugin(dir.path(), "my_plugin", true, true)?;
        assert!(!dir.path().join("plugins/my_plugin").exists());
        assert!(!dir.path().join("plugin_data/my_plugin").exists());
        assert!(!dir.path().join("config/my_plugin_config.json").exists());
        Ok(())
    }

    #[test]
    fn path_traversal_is_rejected() -> Result<()> {
        let dir = TempDir::new("traversal")?;
        fs::create_dir_all(dir.path().join("plugins").join("real"))?;
        for bad in ["..", "real/../other", "../outside"] {
            assert!(find_plugin_dir(dir.path(), bad).is_err(), "{bad}");
        }
        Ok(())
    }

    #[test]
    fn replacement_requires_the_same_plugin() -> Result<()> {
        let dir = TempDir::new("replace")?;
        let target = dir.path().join("plugins").join("my_plugin");
        fs::create_dir_all(&target)?;

        // Same plugin by metadata name: replaceable.
        fs::write(target.join("metadata.yaml"), "name: my_plugin\n")?;
        assert!(ensure_replaceable(&target, "my_plugin").is_ok());

        // A different plugin lives in that directory already.
        assert!(ensure_replaceable(&target, "other_plugin").is_err());

        // A directory without metadata cannot be verified, matching upstream.
        let bare = dir.path().join("plugins").join("bare");
        fs::create_dir_all(&bare)?;
        assert!(ensure_replaceable(&bare, "bare").is_err());

        // Fresh installs are always fine.
        assert!(ensure_replaceable(&dir.path().join("plugins").join("new"), "new").is_ok());
        Ok(())
    }

    #[test]
    fn locate_plugin_root_accepts_wrapper_and_flat_layouts() -> Result<()> {
        let dir = TempDir::new("locate")?;
        let flat = dir.path().join("flat");
        fs::create_dir_all(&flat)?;
        fs::write(flat.join("metadata.yaml"), "name: x\n")?;
        assert!(locate_plugin_root(&flat)? == flat);

        let wrapped = dir.path().join("wrapped");
        let inner = wrapped.join("repo-main");
        fs::create_dir_all(&inner)?;
        fs::write(inner.join("metadata.yaml"), "name: x\n")?;
        assert!(locate_plugin_root(&wrapped)? == inner);
        Ok(())
    }
}
