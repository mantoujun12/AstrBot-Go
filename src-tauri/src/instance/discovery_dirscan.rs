//! Directory-scan instance source.
//!
//! Discovers source-code deployed AstrBot instances (and installations made
//! by the old launcher) by walking configured scan roots for directories
//! that contain `data/cmd_config.json` with AstrBot signature fields.
//! Discovered instances are read-only: the launcher only shows their WebUI
//! availability and never manages their lifecycle.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::discovery::{
    verify_webui, DiscoveredInstance, DiscoveryContext, InstanceCapabilities, InstanceSource,
};
use super::types::CmdConfig;
use crate::error::{AppError, Result};
use crate::process::InstanceState;

const SOURCE_ID: &str = "dir_scan";
const SOURCE_TITLE_KEY: &str = "settings.instances.sources.dir_scan";

/// Maximum directory depth below each scan root.
const MAX_SCAN_DEPTH: usize = 3;

/// Default dashboard port of AstrBot.
const DEFAULT_DASHBOARD_PORT: u16 = 6185;

/// Directory names that never contain instances and make scans fast.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    "target",
    "dist",
    "build",
    "out",
    "venv",
    ".venv",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".idea",
    ".vscode",
    "AppData",
    "Windows",
    "Program Files",
    "Program Files (x86)",
    "$Recycle.Bin",
    "System Volume Information",
];

/// The built-in directory-scan source.
pub struct DirScanSource;

impl InstanceSource for DirScanSource {
    fn id(&self) -> &'static str {
        SOURCE_ID
    }

    fn title_key(&self) -> &'static str {
        SOURCE_TITLE_KEY
    }

    fn discover(&self, ctx: &DiscoveryContext) -> Result<Vec<DiscoveredInstance>> {
        let roots = resolve_scan_roots(&ctx.scan_paths);
        // Launcher-managed instance directories are excluded so a scan root
        // containing the launcher data dir does not list every managed
        // instance again as an external read-only row.
        let managed = managed_instance_roots();
        let mut instances = Vec::new();
        let mut seen: HashSet<PathBuf> = HashSet::new();
        for root in roots {
            for dir in scan_root(&root) {
                // The same instance may be reachable from several roots; only
                // the first occurrence is reported.
                let key = std::fs::canonicalize(&dir).unwrap_or_else(|_| dir.clone());
                if !seen.insert(key.clone()) {
                    continue;
                }
                if managed
                    .iter()
                    .any(|root| key.starts_with(root) || dir.starts_with(root))
                {
                    continue;
                }
                if let Some(instance) = build_instance(&dir) {
                    instances.push(instance);
                }
            }
        }
        Ok(instances)
    }
}

/// Canonicalized directories holding launcher-managed instances
/// (`<data dir>/instances/<id>`); any scan match inside one is skipped.
fn managed_instance_roots() -> Vec<PathBuf> {
    let instances_dir = crate::utils::paths::get_data_dir().join("instances");
    let Ok(entries) = std::fs::read_dir(&instances_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| std::fs::canonicalize(entry.path()).ok())
        .map(normalize_canonical)
        .collect()
}

/// Turn the configured scan paths into actual roots. When nothing is
/// configured the user's home directory is used so the feature works out of
/// the box.
pub(crate) fn resolve_scan_roots(configured: &[String]) -> Vec<PathBuf> {
    if configured.is_empty() {
        return dirs::home_dir().map(|home| vec![home]).unwrap_or_default();
    }
    configured.iter().map(PathBuf::from).collect()
}

/// Validate user-provided scan paths and return the canonicalized,
/// de-duplicated list for persistence. Every path must exist and be a
/// directory; anything else is rejected instead of silently dropped.
pub fn validate_scan_paths(paths: &[String]) -> Result<Vec<String>> {
    let mut validated: Vec<String> = Vec::new();
    for raw in paths {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let path = PathBuf::from(trimmed);
        if !path.exists() {
            return Err(AppError::config(format!(
                "Scan directory does not exist: {}",
                trimmed
            )));
        }
        if !path.is_dir() {
            return Err(AppError::config(format!(
                "Scan path is not a directory: {}",
                trimmed
            )));
        }
        let canonical =
            normalize_canonical(std::fs::canonicalize(&path).map_err(|e| {
                AppError::config(format!("Failed to resolve scan directory: {}", e))
            })?);
        let value = canonical.to_string_lossy().to_string();
        if !validated.contains(&value) {
            validated.push(value);
        }
    }
    Ok(validated)
}

/// Strip the Windows verbatim prefix (`\\?\C:\...` → `C:\...`) so stored
/// scan paths stay readable in the UI.
pub(crate) fn normalize_canonical(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy().to_string();
    if let Some(stripped) = text.strip_prefix(r"\\?\") {
        return PathBuf::from(stripped);
    }
    path
}

/// Directories under one scan root that contain `data/cmd_config.json`.
pub(crate) fn scan_root(root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    collect_candidate_dirs(root, 0, &mut candidates);
    candidates
}

fn collect_candidate_dirs(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > MAX_SCAN_DEPTH {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        // Unreadable directories are skipped so one broken branch cannot
        // fail the whole scan, but the skip is logged: without it, missed
        // instances below the branch look like a complete scan.
        Ok(entries) => entries,
        Err(error) => {
            log::debug!("Directory scan skipped unreadable {dir:?}: {error}");
            return;
        }
    };
    if dir.join("data").join("cmd_config.json").is_file() {
        out.push(dir.to_path_buf());
    }
    if depth == MAX_SCAN_DEPTH {
        return;
    }
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || is_skipped_dir(&entry.file_name().to_string_lossy()) {
            continue;
        }
        collect_candidate_dirs(&path, depth + 1, out);
    }
}

pub(crate) fn is_skipped_dir(name: &str) -> bool {
    if name.starts_with('.') {
        return true;
    }
    SKIP_DIRS.contains(&name)
}

/// Build a discovered instance from a directory holding
/// `data/cmd_config.json`, or `None` when the file is not a valid AstrBot
/// configuration (guards against false positives).
pub(crate) fn build_instance(dir: &Path) -> Option<DiscoveredInstance> {
    let content = std::fs::read_to_string(dir.join("data").join("cmd_config.json")).ok()?;
    let (port, dashboard_enabled) = validate_cmd_config(&content)?;

    let name = dir
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    if name.is_empty() {
        return None;
    }
    let id = std::fs::canonicalize(dir)
        .unwrap_or_else(|_| dir.to_path_buf())
        .to_string_lossy()
        .to_string();

    // A plain TCP probe alone would mistake an unrelated listener on the
    // port for a running instance, so the WebUI is verified over HTTP
    // (issue #81 false-positive guard).
    let state = if dashboard_enabled && verify_webui(port) {
        InstanceState::Running
    } else {
        InstanceState::Stopped
    };

    Some(DiscoveredInstance {
        source: SOURCE_ID.to_string(),
        id,
        name,
        webui_url: resolve_dashboard_url(dashboard_enabled, port),
        state,
        capabilities: InstanceCapabilities { read_only: true },
    })
}

/// Read the dashboard port and enable switch from a deployment's
/// `data/cmd_config.json`; falls back to the defaults when the file is
/// absent or does not carry the values.
pub(crate) fn read_dashboard_config(deployment: &Path) -> (u16, bool) {
    std::fs::read_to_string(deployment.join("data").join("cmd_config.json"))
        .ok()
        .and_then(|content| validate_cmd_config(&content))
        .unwrap_or((DEFAULT_DASHBOARD_PORT, true))
}

/// Build the WebUI URL for a dashboard: `None` when the dashboard is
/// disabled, otherwise `http://127.0.0.1:<port>`.
pub(crate) fn resolve_dashboard_url(enabled: bool, port: u16) -> Option<String> {
    enabled.then(|| format!("http://127.0.0.1:{port}"))
}

/// Validate that parsed `cmd_config.json` content is an AstrBot
/// configuration and return the dashboard port (default
/// [`DEFAULT_DASHBOARD_PORT`] when absent) plus the `enable` switch.
pub(crate) fn validate_cmd_config(content: &str) -> Option<(u16, bool)> {
    let value: serde_json::Value =
        serde_json::from_str(content.trim_start_matches('\u{feff}')).ok()?;
    let dashboard = value.get("dashboard")?.as_object()?;
    // AstrBot signature: a dashboard object with an `enable` switch and a
    // `username` field. Files without these fields are other applications.
    let enable = dashboard
        .get("enable")
        .and_then(|enable| enable.as_bool())?;
    dashboard.get("username")?;
    // Reuse the shared CmdConfig parser so launcher-managed and discovered
    // instances read the dashboard identically.
    let parsed: CmdConfig = serde_json::from_value(value).ok()?;
    let port = parsed
        .dashboard
        .and_then(|dashboard| dashboard.port)
        .unwrap_or(DEFAULT_DASHBOARD_PORT);
    Some((port, enable))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;
    use std::fs;

    fn write_config(dir: &Path, content: &str) -> Result<()> {
        let data = dir.join("data");
        fs::create_dir_all(&data)?;
        fs::write(data.join("cmd_config.json"), content)?;
        Ok(())
    }

    const VALID_CONFIG: &str =
        r#"{"dashboard":{"enable":true,"username":"astrbot","password":"x","port":6185}}"#;

    #[test]
    fn is_skipped_dir_matches_noise() {
        for name in ["node_modules", ".git", "target", "AppData"] {
            assert!(is_skipped_dir(name));
        }
        for name in ["AstrBot", "my bot", "data"] {
            assert!(!is_skipped_dir(name));
        }
    }

    #[test]
    fn scan_root_finds_config_at_depth_and_skips_noise() -> Result<()> {
        let temp = std::env::temp_dir().join(format!("dirscan-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let deep = temp.join("a").join("b").join("deep3");
        write_config(&deep, VALID_CONFIG)?;
        write_config(&temp.join("node_modules").join("fake"), VALID_CONFIG)?;
        write_config(&temp.join("my-bot"), VALID_CONFIG)?;

        let mut found = scan_root(&temp);
        found.sort();
        let found_names: Vec<String> = found
            .iter()
            .filter_map(|path| path.file_name().map(|n| n.to_string_lossy().to_string()))
            .collect();
        assert!(found_names.contains(&"my-bot".to_string()));
        assert!(found_names.contains(&"deep3".to_string()));
        assert!(!found_names.contains(&"fake".to_string()));
        fs::remove_dir_all(&temp)?;
        Ok(())
    }

    #[test]
    fn validate_cmd_config_accepts_astrbot_and_rejects_other_files() {
        assert_eq!(validate_cmd_config(VALID_CONFIG), Some((6185, true)));
        // Port falls back to the AstrBot default when absent.
        assert_eq!(
            validate_cmd_config(r#"{"dashboard":{"enable":true,"username":"x"}}"#),
            Some((DEFAULT_DASHBOARD_PORT, true))
        );
        // A disabled dashboard is still an AstrBot deployment, but without
        // a usable WebUI.
        assert_eq!(
            validate_cmd_config(r#"{"dashboard":{"enable":false,"username":"x","port":7000}}"#),
            Some((7000, false))
        );
        // Missing signature fields → skipped.
        assert_eq!(validate_cmd_config(r#"{"other":1}"#), None);
        assert_eq!(validate_cmd_config(r#"{"dashboard":{"port":1}}"#), None);
        assert_eq!(validate_cmd_config("not json"), None);
    }

    #[test]
    fn build_instance_uses_directory_name_and_probes_port() -> Result<()> {
        let temp = std::env::temp_dir().join(format!("dirscan-build-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let dir = temp.join("AstrBot");
        // Reserve an ephemeral port then release it: nothing listens, and
        // the fixed 6185 default would race a real WebUI on this machine.
        let probe = std::net::TcpListener::bind(("127.0.0.1", 0))
            .map_err(|error| AppError::other(error.to_string()))?;
        let port = probe
            .local_addr()
            .map_err(|error| AppError::other(error.to_string()))?
            .port();
        drop(probe);
        write_config(
            &dir,
            &format!(r#"{{"dashboard":{{"enable":true,"username":"astrbot","port":{port}}}}}"#),
        )?;

        let instance = build_instance(&dir).ok_or_else(|| AppError::other("instance missing"))?;
        assert_eq!(instance.source, "dir_scan");
        assert_eq!(instance.name, "AstrBot");
        assert_eq!(
            instance.webui_url.as_deref(),
            Some(&*format!("http://127.0.0.1:{port}"))
        );
        assert!(instance.capabilities.read_only);
        // Nothing is listening; the instance must be reported stopped.
        assert_eq!(instance.state, InstanceState::Stopped);
        assert!(instance.id.contains("AstrBot"));

        assert!(build_instance(&temp).is_none());
        fs::remove_dir_all(&temp)?;
        Ok(())
    }

    #[test]
    fn disabled_dashboard_gets_no_webui_url() -> Result<()> {
        let temp = std::env::temp_dir().join(format!("dirscan-disabled-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let dir = temp.join("AstrBot");
        write_config(
            &dir,
            r#"{"dashboard":{"enable":false,"username":"astrbot","port":6185}}"#,
        )?;

        let instance = build_instance(&dir).ok_or_else(|| AppError::other("instance missing"))?;
        // No WebUI link for a disabled dashboard, and always stopped.
        assert_eq!(instance.webui_url, None);
        assert_eq!(instance.state, InstanceState::Stopped);
        fs::remove_dir_all(&temp)?;
        Ok(())
    }

    #[test]
    fn dashboard_config_falls_back_to_defaults() {
        // Missing file → defaults.
        let (port, enabled) = read_dashboard_config(Path::new("definitely-missing-dir"));
        assert_eq!(port, DEFAULT_DASHBOARD_PORT);
        assert!(enabled);
    }

    #[test]
    fn validate_scan_paths_rejects_missing_and_deduplicates() -> Result<()> {
        let temp = std::env::temp_dir().join(format!("dirscan-paths-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp)?;
        let temp_str = temp.to_string_lossy().to_string();

        // Missing path is rejected, not dropped.
        let missing = temp.join("nope");
        assert!(validate_scan_paths(&[missing.to_string_lossy().to_string()]).is_err());

        // A file path is rejected.
        let file = temp.join("file.txt");
        fs::write(&file, b"x")?;
        assert!(validate_scan_paths(&[file.to_string_lossy().to_string()]).is_err());

        let result = validate_scan_paths(&[temp_str.clone(), format!("  {}  ", temp_str)])?;
        assert_eq!(result, vec![temp_str]);
        fs::remove_dir_all(&temp)?;
        Ok(())
    }
}
