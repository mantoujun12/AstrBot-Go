use std::collections::HashMap;
use std::ffi::OsString;

use reqwest::Client;
use semver::Version;
use serde::Deserialize;

use crate::config::AppConfig;
use crate::error::{AppError, Result};
use crate::github::{build_api_url, build_download_url, GitHubAsset, GitHubRelease};
use crate::platform::{get_nodejs_os_arch, get_python_arch_target, get_uv_archive_name};
use crate::utils::index_url::{join_url, normalize_default_index, wrap_with_proxy};
use crate::utils::net::{
    build_download_client_with_proxy, build_http_client_with_proxy, fetch_json_checked,
};
use crate::utils::proxy::{build_proxy_env_vars, resolve_proxy_from_config, ProxySettings};

const GITHUB_PROBE_URL: &str =
    "https://github.com/AstrBotDevs/AstrBot/archive/refs/heads/master.zip";
const PYTHON_RELEASES_API: &str =
    "https://api.github.com/repos/astral-sh/python-build-standalone/releases?per_page=10";
const NODE_DEFAULT_MIRROR: &str = "https://nodejs.org/dist";
const NPM_DEFAULT_REGISTRY: &str = "https://registry.npmjs.org";
const NODE_INDEX_FILE: &str = "index.json";
const NPM_PROBE_PACKAGE: &str = "vue";
const NODE_ARCHIVE_EXT: &str = if cfg!(windows) { "zip" } else { "tar.gz" };
const PYTHON_PROBE_ASSET_SUFFIX: &str = "-install_only_stripped.tar.gz";

const LEGACY_MAINLAND_GITHUB_PROXY: &str = "https://gh-proxy.org/";
const LEGACY_MAINLAND_PYPI_MIRROR: &str = "https://mirrors.cernet.edu.cn/pypi/web/simple";
const LEGACY_MAINLAND_NODEJS_MIRROR: &str = "https://mirrors.cernet.edu.cn/nodejs-release/";
const LEGACY_MAINLAND_NPM_REGISTRY: &str = "https://registry.npmmirror.com/";
const LEGACY_MAINLAND_PYTHON_MIRROR: &str =
    "https://mirrors.ustc.edu.cn/github-release/astral-sh/python-build-standalone/LatestRelease/";
const LEGACY_MAINLAND_UV_MIRROR: &str =
    "https://mirrors.ustc.edu.cn/github-release/astral-sh/uv/LatestRelease/";

/// Source kinds exposed to the network connectivity test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkSourceKind {
    Github,
    Pypi,
    Nodejs,
    Npm,
    Python,
    Uv,
}

/// Map the removed global mainland switch to the equivalent per-source presets.
pub(crate) fn apply_legacy_mainland_presets(config: &mut AppConfig) {
    config.github_proxy = LEGACY_MAINLAND_GITHUB_PROXY.to_string();
    config.pypi_mirror = LEGACY_MAINLAND_PYPI_MIRROR.to_string();
    config.nodejs_mirror = LEGACY_MAINLAND_NODEJS_MIRROR.to_string();
    config.npm_registry = LEGACY_MAINLAND_NPM_REGISTRY.to_string();
    config.python_mirror = LEGACY_MAINLAND_PYTHON_MIRROR.to_string();
    config.uv_mirror = LEGACY_MAINLAND_UV_MIRROR.to_string();
}

pub(crate) fn build_http_client_from_config(config: &AppConfig) -> Result<Client> {
    build_http_client_with_proxy(proxy_settings(config)?)
}

pub(crate) fn build_download_client_from_config(config: &AppConfig) -> Result<Client> {
    build_download_client_with_proxy(proxy_settings(config)?)
}

pub(crate) fn proxy_settings(config: &AppConfig) -> Result<Option<ProxySettings>> {
    resolve_proxy_from_config(config)
}

pub(crate) fn proxy_env_vars(config: &AppConfig) -> Result<Vec<(OsString, OsString)>> {
    build_proxy_env_vars(config)
}

pub(crate) fn default_index(config: &AppConfig) -> String {
    normalize_default_index(&config.pypi_mirror)
}

pub(crate) fn nodejs_mirror_root(config: &AppConfig) -> String {
    nodejs_mirror_root_from(&config.nodejs_mirror)
}

fn nodejs_mirror_root_from(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        NODE_DEFAULT_MIRROR.to_string()
    } else {
        trimmed.to_string()
    }
}

pub(crate) fn npm_registry(config: &AppConfig) -> Option<String> {
    npm_registry_from(&config.npm_registry)
}

fn npm_registry_from(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub(crate) fn python_mirror(config: &AppConfig) -> Option<&str> {
    let trimmed = config.python_mirror.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

pub(crate) fn uv_mirror(config: &AppConfig) -> Option<&str> {
    let trimmed = config.uv_mirror.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

pub(crate) fn astrbot_releases_api_url(config: &AppConfig) -> String {
    build_api_url(&config.github_proxy)
}

pub(crate) fn astrbot_source_archive_urls(config: &AppConfig, tag: &str) -> Vec<String> {
    vec![build_download_url(&config.github_proxy, tag)]
}

pub(crate) fn astrbot_dashboard_archive_urls(config: &AppConfig, tag: &str) -> Vec<String> {
    let tag = tag.trim();
    let registry_url =
        format!("https://astrbot-registry.soulter.top/download/astrbot-dashboard/{tag}/dist.zip");
    let github_url = format!(
        "https://github.com/AstrBotDevs/AstrBot/releases/download/{tag}/AstrBot-{tag}-dashboard.zip"
    );
    let github_fallback_url = wrap_with_proxy(&config.github_proxy, &github_url);

    vec![registry_url, github_fallback_url]
}

pub(crate) fn build_uv_download_url(config: &AppConfig, archive_name: &str) -> String {
    if let Some(base) = uv_mirror(config) {
        join_url(base, archive_name)
    } else {
        let raw_url = format!(
            "https://github.com/astral-sh/uv/releases/latest/download/{}",
            archive_name
        );
        wrap_with_proxy(&config.github_proxy, &raw_url)
    }
}

pub(crate) fn build_mirror_python_asset_download_url(base: &str, asset_name: &str) -> String {
    join_url(base, asset_name)
}

pub(crate) fn build_github_python_asset_download_url(
    config: &AppConfig,
    github_url: &str,
) -> String {
    wrap_with_proxy(&config.github_proxy, github_url)
}

#[derive(Debug, Deserialize)]
struct NodeIndexEntry {
    version: String,
}

#[derive(Debug, Deserialize)]
struct NpmMetadata {
    #[serde(rename = "dist-tags", default)]
    dist_tags: HashMap<String, String>,
}

/// Pick the highest stable version from a Node.js release index.
fn latest_stable_node_version(entries: &[NodeIndexEntry]) -> Option<String> {
    entries
        .iter()
        .filter_map(|entry| {
            let raw = entry.version.trim();
            let version = raw.strip_prefix('v').unwrap_or(raw);
            let parsed = Version::parse(version).ok()?;
            if !parsed.pre.is_empty() {
                return None;
            }
            Some((parsed, version.to_string()))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, version)| version)
}

fn nodejs_archive_name(version: &str, os: &str, arch: &str) -> String {
    format!("node-v{version}-{os}-{arch}.{NODE_ARCHIVE_EXT}")
}

fn nodejs_archive_url(root: &str, version: &str) -> Result<String> {
    let (os, arch) = get_nodejs_os_arch()
        .map_err(|error| AppError::io(format!("Unsupported platform: {error}")))?;
    let archive_name = nodejs_archive_name(version, os, arch);
    Ok(join_url(root, &format!("v{version}/{archive_name}")))
}

fn npm_tarball_url(registry: &str, version: &str) -> String {
    let path = format!("{NPM_PROBE_PACKAGE}/-/{NPM_PROBE_PACKAGE}-{version}.tgz");
    join_url(registry, &path)
}

fn select_python_probe_asset<'a>(
    releases: &'a [GitHubRelease],
    arch_target: &str,
) -> Option<&'a GitHubAsset> {
    let suffix = format!("-{arch_target}{PYTHON_PROBE_ASSET_SUFFIX}");

    releases
        .iter()
        .filter(|release| !release.prerelease)
        .find_map(|release| {
            release
                .assets
                .iter()
                .find(|asset| asset.name.starts_with("cpython-") && asset.name.ends_with(&suffix))
        })
}

fn python_probe_asset_url(config: &AppConfig, mirror: Option<&str>, asset: &GitHubAsset) -> String {
    match mirror {
        Some(base) => build_mirror_python_asset_download_url(base, &asset.name),
        None => build_github_python_asset_download_url(config, &asset.browser_download_url),
    }
}

async fn python_probe_url(
    value: &str,
    config: &AppConfig,
    client: &Client,
    allow_local_dns: bool,
) -> Result<String> {
    let arch_target = get_python_arch_target().map_err(AppError::python)?;
    let api_url = wrap_with_proxy(&config.github_proxy, PYTHON_RELEASES_API);
    let releases: Vec<GitHubRelease> =
        fetch_json_checked(client, &api_url, allow_local_dns).await?;

    let asset = match select_python_probe_asset(&releases, arch_target) {
        Some(asset) => asset,
        None => {
            let message = format!("No Python build found for platform {arch_target}");
            return Err(AppError::python(message));
        }
    };

    Ok(python_probe_asset_url(
        config,
        (!value.is_empty()).then_some(value),
        asset,
    ))
}

/// Build the URL that represents a source in the connectivity test.
///
/// Node.js, npm, and Python resolve a real archive first so the speed test does
/// not measure metadata. PyPI is the other kind with an extra step: the
/// returned URL is the index page, which the caller parses to find a real
/// wheel.
pub(crate) async fn source_probe_url(
    kind: NetworkSourceKind,
    value: &str,
    config: &AppConfig,
    client: &Client,
    allow_local_dns: bool,
) -> Result<String> {
    let value = value.trim();

    Ok(match kind {
        NetworkSourceKind::Github => wrap_with_proxy(value, GITHUB_PROBE_URL),
        NetworkSourceKind::Pypi => {
            let index = normalize_default_index(value);
            join_url(&index, "pip/")
        }
        NetworkSourceKind::Nodejs => {
            let root = nodejs_mirror_root_from(value);
            let index_url = join_url(&root, NODE_INDEX_FILE);
            let entries: Vec<NodeIndexEntry> =
                fetch_json_checked(client, &index_url, allow_local_dns).await?;
            let version = match latest_stable_node_version(&entries) {
                Some(version) => version,
                None => {
                    let message = format!("No stable Node.js release found at {index_url}");
                    return Err(AppError::network(message));
                }
            };
            nodejs_archive_url(&root, &version)?
        }
        NetworkSourceKind::Npm => {
            let registry = npm_registry_from(value);
            let registry = registry.as_deref().unwrap_or(NPM_DEFAULT_REGISTRY);
            let metadata_url = join_url(registry, NPM_PROBE_PACKAGE);
            let metadata: NpmMetadata =
                fetch_json_checked(client, &metadata_url, allow_local_dns).await?;
            let version = match metadata.dist_tags.get("latest") {
                Some(version) => version.clone(),
                None => {
                    let message = format!("No latest version in npm metadata from {registry}");
                    return Err(AppError::network(message));
                }
            };
            npm_tarball_url(registry, &version)
        }
        NetworkSourceKind::Python => {
            python_probe_url(value, config, client, allow_local_dns).await?
        }
        NetworkSourceKind::Uv => {
            let archive_name = get_uv_archive_name()
                .map_err(|error| AppError::io(format!("Unsupported platform: {error}")))?;
            if value.is_empty() {
                let raw_url = format!(
                    "https://github.com/astral-sh/uv/releases/latest/download/{}",
                    archive_name
                );
                wrap_with_proxy(&config.github_proxy, &raw_url)
            } else {
                join_url(value, archive_name)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_presets_migrate_all_sources() {
        let mut config = AppConfig::default();
        apply_legacy_mainland_presets(&mut config);

        assert_eq!(config.github_proxy, LEGACY_MAINLAND_GITHUB_PROXY);
        assert_eq!(config.pypi_mirror, LEGACY_MAINLAND_PYPI_MIRROR);
        assert_eq!(config.nodejs_mirror, LEGACY_MAINLAND_NODEJS_MIRROR);
        assert_eq!(config.npm_registry, LEGACY_MAINLAND_NPM_REGISTRY);
        assert_eq!(config.python_mirror, LEGACY_MAINLAND_PYTHON_MIRROR);
        assert_eq!(config.uv_mirror, LEGACY_MAINLAND_UV_MIRROR);
    }

    fn sample_python_asset(name: &str) -> GitHubAsset {
        GitHubAsset {
            name: name.to_string(),
            browser_download_url: format!(
                "https://github.com/astral-sh/python-build-standalone/releases/download/20260101/{name}"
            ),
            size: 1,
        }
    }

    fn sample_release(prerelease: bool, assets: Vec<GitHubAsset>) -> GitHubRelease {
        GitHubRelease {
            tag_name: "20260101".to_string(),
            name: "20260101".to_string(),
            published_at: String::new(),
            prerelease,
            assets,
            html_url: String::new(),
            body: None,
        }
    }

    #[tokio::test]
    async fn probe_urls_for_github_and_pypi_need_no_network() {
        let config = AppConfig::default();
        let client = Client::new();

        assert_eq!(
            source_probe_url(NetworkSourceKind::Github, "", &config, &client, false)
                .await
                .unwrap_or_default(),
            GITHUB_PROBE_URL
        );
        assert_eq!(
            source_probe_url(NetworkSourceKind::Pypi, "", &config, &client, false)
                .await
                .unwrap_or_default(),
            "https://pypi.org/simple/pip/"
        );
        assert_eq!(
            source_probe_url(
                NetworkSourceKind::Pypi,
                "https://mirror.example.com/simple/",
                &config,
                &client,
                false
            )
            .await
            .unwrap_or_default(),
            "https://mirror.example.com/simple/pip/"
        );
    }

    #[test]
    fn node_probe_url_targets_archive() {
        let entries = [
            NodeIndexEntry {
                version: "v24.0.0-rc.1".to_string(),
            },
            NodeIndexEntry {
                version: "v22.9.0".to_string(),
            },
            NodeIndexEntry {
                version: "v22.10.0".to_string(),
            },
            NodeIndexEntry {
                version: "garbage".to_string(),
            },
        ];

        let version = latest_stable_node_version(&entries).unwrap_or_default();
        assert_eq!(version, "22.10.0");
        assert_eq!(
            nodejs_archive_name(&version, "win", "x64"),
            format!("node-v22.10.0-win-x64.{NODE_ARCHIVE_EXT}")
        );
        assert_eq!(
            nodejs_archive_name(&version, "linux", "arm64"),
            format!("node-v22.10.0-linux-arm64.{NODE_ARCHIVE_EXT}")
        );
        assert!(latest_stable_node_version(&[]).is_none());
    }

    #[test]
    fn npm_probe_url_targets_tarball() {
        assert_eq!(
            npm_tarball_url("https://npm.example.com/", "3.5.17"),
            "https://npm.example.com/vue/-/vue-3.5.17.tgz"
        );
    }

    #[test]
    fn python_probe_selects_stable_asset_and_builds_url() {
        let linux_asset =
            "cpython-3.13.1+20260101-x86_64-unknown-linux-gnu-install_only_stripped.tar.gz";
        let linux_asset_name = linux_asset.to_string();
        let releases = vec![
            sample_release(
                true,
                vec![sample_python_asset(
                    "cpython-3.14.0+20260101-x86_64-unknown-linux-gnu-install_only_stripped.tar.gz",
                )],
            ),
            sample_release(
                false,
                vec![sample_python_asset(
                    "cpython-3.13.1+20260101-aarch64-apple-darwin-install_only.tar.gz",
                )],
            ),
            sample_release(false, vec![sample_python_asset(linux_asset)]),
        ];

        let asset = select_python_probe_asset(&releases, "x86_64-unknown-linux-gnu");
        assert!(asset.is_some());
        let Some(asset) = asset else {
            return;
        };
        assert_eq!(asset.name, linux_asset_name);
        assert!(select_python_probe_asset(&releases, "aarch64-apple-darwin").is_none());

        let config = AppConfig {
            github_proxy: "https://gh-proxy.com/".to_string(),
            ..AppConfig::default()
        };
        assert_eq!(
            python_probe_asset_url(&config, None, asset),
            format!("https://gh-proxy.com/{}", asset.browser_download_url)
        );
        assert_eq!(
            python_probe_asset_url(
                &config,
                Some("https://mirror.example.com/LatestRelease/"),
                asset
            ),
            format!("https://mirror.example.com/LatestRelease/{linux_asset_name}")
        );
    }
}
