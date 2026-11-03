use std::ffi::OsString;

use reqwest::Client;
use serde::Deserialize;

use crate::config::AppConfig;
use crate::error::{AppError, Result};
use crate::github::{build_api_url, build_download_url};
use crate::platform::get_uv_archive_name;
use crate::utils::index_url::{join_url, normalize_default_index, wrap_with_proxy};
use crate::utils::net::{build_download_client_with_proxy, build_http_client_with_proxy};
use crate::utils::proxy::{build_proxy_env_vars, resolve_proxy_from_config, ProxySettings};

const GITHUB_PROBE_URL: &str =
    "https://github.com/AstrBotDevs/AstrBot/archive/refs/heads/master.zip";
const PYTHON_RELEASES_API: &str =
    "https://api.github.com/repos/astral-sh/python-build-standalone/releases?per_page=10";
const NODE_DEFAULT_MIRROR: &str = "https://nodejs.org/dist";
const NPM_DEFAULT_REGISTRY: &str = "https://registry.npmjs.org";

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

/// Build the URL that represents a source in the connectivity test.
///
/// PyPI is the only kind whose speed test needs an extra step: the returned
/// URL is the index page, which the caller parses to find a real wheel.
pub(crate) fn source_probe_url(
    kind: NetworkSourceKind,
    value: &str,
    config: &AppConfig,
) -> Result<String> {
    let value = value.trim();

    Ok(match kind {
        NetworkSourceKind::Github => wrap_with_proxy(value, GITHUB_PROBE_URL),
        NetworkSourceKind::Pypi => {
            let index = normalize_default_index(value);
            join_url(&index, "pip/")
        }
        NetworkSourceKind::Nodejs => join_url(&nodejs_mirror_root_from(value), "index.json"),
        NetworkSourceKind::Npm => join_url(
            npm_registry_from(value)
                .as_deref()
                .unwrap_or(NPM_DEFAULT_REGISTRY),
            "vue",
        ),
        NetworkSourceKind::Python => {
            if value.is_empty() {
                wrap_with_proxy(&config.github_proxy, PYTHON_RELEASES_API)
            } else {
                join_url(value, "")
            }
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

    #[test]
    fn probe_urls_use_official_defaults() {
        let config = AppConfig::default();

        assert_eq!(
            source_probe_url(NetworkSourceKind::Github, "", &config).unwrap_or_default(),
            GITHUB_PROBE_URL
        );
        assert_eq!(
            source_probe_url(NetworkSourceKind::Pypi, "", &config).unwrap_or_default(),
            "https://pypi.org/simple/pip/"
        );
        assert_eq!(
            source_probe_url(NetworkSourceKind::Nodejs, "", &config).unwrap_or_default(),
            "https://nodejs.org/dist/index.json"
        );
        assert_eq!(
            source_probe_url(NetworkSourceKind::Npm, "", &config).unwrap_or_default(),
            "https://registry.npmjs.org/vue"
        );
    }

    #[test]
    fn probe_urls_respect_presets_and_normalization() {
        let config = AppConfig {
            github_proxy: "https://gh-proxy.com/".to_string(),
            ..AppConfig::default()
        };

        assert_eq!(
            source_probe_url(
                NetworkSourceKind::Github,
                "https://gh-proxy.org/",
                &config
            )
            .unwrap_or_default(),
            "https://gh-proxy.org/https://github.com/AstrBotDevs/AstrBot/archive/refs/heads/master.zip"
        );
        assert_eq!(
            source_probe_url(
                NetworkSourceKind::Pypi,
                "https://mirror.example.com/simple/",
                &config
            )
            .unwrap_or_default(),
            "https://mirror.example.com/simple/pip/"
        );
        assert_eq!(
            source_probe_url(
                NetworkSourceKind::Nodejs,
                "https://node.example.com/",
                &config
            )
            .unwrap_or_default(),
            "https://node.example.com/index.json"
        );
        assert_eq!(
            source_probe_url(NetworkSourceKind::Npm, "https://npm.example.com/", &config)
                .unwrap_or_default(),
            "https://npm.example.com/vue"
        );
    }

    #[test]
    fn python_and_uv_probe_urls_follow_github_or_mirror() {
        let config = AppConfig {
            github_proxy: "https://gh-proxy.com/".to_string(),
            ..AppConfig::default()
        };

        assert_eq!(
            source_probe_url(NetworkSourceKind::Python, "", &config).unwrap_or_default(),
            "https://gh-proxy.com/https://api.github.com/repos/astral-sh/python-build-standalone/releases?per_page=10"
        );
        assert_eq!(
            source_probe_url(
                NetworkSourceKind::Python,
                "https://mirror.example.com/LatestRelease/",
                &config
            )
            .unwrap_or_default(),
            "https://mirror.example.com/LatestRelease/"
        );
        assert!(source_probe_url(
            NetworkSourceKind::Uv,
            "https://mirror.example.com/LatestRelease/",
            &config
        )
        .unwrap_or_default()
        .starts_with("https://mirror.example.com/LatestRelease/uv-"));
    }
}
