//! Instance update checks against AstrBot GitHub releases.

use serde::{Deserialize, Serialize};

use crate::config::AppManifest;
use crate::github::GitHubRelease;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceUpdateState {
    Available,
    UpToDate,
    Disabled,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceUpdateInfo {
    pub instance_id: String,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub state: InstanceUpdateState,
    pub check_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceUpdateCheckResult {
    pub update_check_enabled: bool,
    pub latest_release: Option<GitHubRelease>,
    pub instances: Vec<InstanceUpdateInfo>,
}

fn parse_version(value: &str) -> Option<semver::Version> {
    let trimmed = value.trim();
    let without_prefix = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    semver::Version::parse(without_prefix).ok()
}

/// Return the newest release that is neither marked as a prerelease nor carries
/// a semantic-version prerelease suffix. AstrBot has published beta tags with
/// the API prerelease flag unset, so both signals are checked.
pub fn latest_stable_release(releases: &[GitHubRelease]) -> Option<GitHubRelease> {
    releases
        .iter()
        .filter(|release| !release.prerelease)
        .filter_map(|release| {
            let version = parse_version(&release.tag_name)?;
            if version.pre.is_empty() {
                Some((version, release))
            } else {
                None
            }
        })
        .max_by(|(left, _), (right, _)| left.cmp(right))
        .map(|(_, release)| release.clone())
}

pub fn evaluate_instance_update(
    current_version: &str,
    latest_version: Option<&str>,
    global_check_enabled: bool,
    instance_check_enabled: bool,
) -> InstanceUpdateState {
    if !global_check_enabled || !instance_check_enabled {
        return InstanceUpdateState::Disabled;
    }

    let Some(latest_version) = latest_version else {
        return InstanceUpdateState::Unknown;
    };
    let Some(current) = parse_version(current_version) else {
        return InstanceUpdateState::Unknown;
    };
    let Some(latest) = parse_version(latest_version) else {
        return InstanceUpdateState::Unknown;
    };

    if latest > current {
        InstanceUpdateState::Available
    } else {
        InstanceUpdateState::UpToDate
    }
}

pub fn build_update_check_result(
    global_check_enabled: bool,
    manifest: &AppManifest,
    latest_release: Option<GitHubRelease>,
) -> InstanceUpdateCheckResult {
    let latest_version = latest_release
        .as_ref()
        .map(|release| release.tag_name.as_str());
    let mut instances: Vec<InstanceUpdateInfo> = manifest
        .instances
        .iter()
        .map(|(instance_id, instance)| {
            let check_enabled = global_check_enabled && instance.check_update_enabled;
            let state = evaluate_instance_update(
                &instance.version,
                latest_version,
                global_check_enabled,
                instance.check_update_enabled,
            );
            InstanceUpdateInfo {
                instance_id: instance_id.clone(),
                current_version: instance.version.clone(),
                latest_version: latest_version.map(ToOwned::to_owned),
                state,
                check_enabled,
            }
        })
        .collect();

    instances.sort_by(|left, right| left.instance_id.cmp(&right.instance_id));

    InstanceUpdateCheckResult {
        update_check_enabled: global_check_enabled,
        latest_release,
        instances,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::InstanceConfig;

    fn release(tag_name: &str, prerelease: bool) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag_name.to_string(),
            name: tag_name.to_string(),
            published_at: "2026-11-04T00:00:00Z".to_string(),
            prerelease,
            assets: Vec::new(),
            html_url: format!("https://example.com/{tag_name}"),
            body: None,
        }
    }

    fn instance(version: &str, check_update_enabled: bool) -> InstanceConfig {
        InstanceConfig {
            name: format!("Instance {version}"),
            version: version.to_string(),
            host: "127.0.0.1".to_string(),
            port: 6185,
            created_at: String::new(),
            check_update_enabled,
            auto_restart_enabled: false,
            auto_restart_max_retries: 3,
            auto_restart_notify: true,
        }
    }

    #[test]
    fn latest_stable_release_uses_semver_and_skips_prereleases() {
        let releases = vec![
            release("v4.9.0", false),
            release("v4.10.0-beta.1", false),
            release("v4.10.0", true),
            release("v4.10.0", false),
        ];

        assert_eq!(
            latest_stable_release(&releases).map(|release| release.tag_name),
            Some("v4.10.0".to_string())
        );
    }

    #[test]
    fn update_state_respects_switches_and_version_order() {
        assert_eq!(
            evaluate_instance_update("4.28.2", Some("v4.29.4"), false, true),
            InstanceUpdateState::Disabled
        );
        assert_eq!(
            evaluate_instance_update("4.28.2", Some("v4.29.4"), true, false),
            InstanceUpdateState::Disabled
        );
        assert_eq!(
            evaluate_instance_update("v4.28.2", Some("v4.29.4"), true, true),
            InstanceUpdateState::Available
        );
        assert_eq!(
            evaluate_instance_update("4.29.4", Some("v4.29.4"), true, true),
            InstanceUpdateState::UpToDate
        );
        assert_eq!(
            evaluate_instance_update("4.30.0", Some("v4.29.4"), true, true),
            InstanceUpdateState::UpToDate
        );
    }

    #[test]
    fn update_state_is_unknown_when_versions_cannot_be_compared() {
        assert_eq!(
            evaluate_instance_update("custom-build", Some("v4.29.4"), true, true),
            InstanceUpdateState::Unknown
        );
        assert_eq!(
            evaluate_instance_update("4.28.2", None, true, true),
            InstanceUpdateState::Unknown
        );
    }

    #[test]
    fn result_is_sorted_and_carries_each_instance_switch() {
        let mut manifest = AppManifest::default();
        manifest
            .instances
            .insert("b".to_string(), instance("4.28.2", true));
        manifest
            .instances
            .insert("a".to_string(), instance("4.28.2", false));

        let result = build_update_check_result(true, &manifest, Some(release("v4.29.4", false)));

        assert!(result.update_check_enabled);
        assert_eq!(
            result
                .instances
                .iter()
                .map(|info| info.instance_id.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b"]
        );
        assert_eq!(result.instances[0].state, InstanceUpdateState::Disabled);
        assert!(!result.instances[0].check_enabled);
        assert_eq!(result.instances[1].state, InstanceUpdateState::Available);
        assert!(result.instances[1].check_enabled);
    }
}
