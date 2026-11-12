//! Discovery source for AstrBot instances deployed through the official
//! `uv tool` one-shot script, which installs the `astrbot` CLI on PATH.
//!
//! The installation is confirmed through `uv tool list` (so other ways of
//! putting an `astrbot` executable on PATH are not misclassified), the
//! deployment directory is resolved via `uv tool dir`, and the dashboard
//! port and enable switch are read from its `data/cmd_config.json`. The
//! `astrbot` executable itself is never executed; only `uv` subcommands run,
//! each with a timeout and killed by PID when it hangs.
//!
//! This source is read-only: it reports the deployment without touching it.

use std::path::{Path, PathBuf};

use super::discovery::{
    locate_program, run_capped, verify_webui, DiscoveredInstance, DiscoveryContext,
    InstanceCapabilities, InstanceSource,
};
use super::discovery_dirscan::{read_dashboard_config, resolve_dashboard_url};
use crate::error::Result;
use crate::process::InstanceState;

/// Default WebUI port used by AstrBot when `cmd_config.json` does not
/// override it.
pub const DEFAULT_WEBUI_PORT: u16 = 6185;

/// Discovery source reporting uv tool (`astrbot` CLI) deployments.
pub struct UvToolSource;

impl InstanceSource for UvToolSource {
    fn id(&self) -> &'static str {
        "uv_tool"
    }

    fn title_key(&self) -> &'static str {
        "settings.instances.sources.uv_tool"
    }

    fn discover(&self, _ctx: &DiscoveryContext) -> Result<Vec<DiscoveredInstance>> {
        // The uv tool deployment requires `uv` itself; without it there is
        // nothing to confirm and we degrade silently to an empty list.
        let Some(uv) = locate_program("uv") else {
            return Ok(Vec::new());
        };

        // Confirm the AstrBot tool installation and read its version from
        // `uv tool list`; PATH binaries other than `uv` are not executed.
        let Some(listing) = run_capped(uv.to_string_lossy().as_ref(), &["tool", "list"]) else {
            return Ok(Vec::new());
        };
        let Some(version) = parse_tool_list(&listing) else {
            // `uv` exists but no astrbot tool installed.
            return Ok(Vec::new());
        };

        // Resolve the deployment directory and read its dashboard config so
        // a customized port gets a correct WebUI link and probe.
        let deployment: Option<PathBuf> =
            run_capped(uv.to_string_lossy().as_ref(), &["tool", "dir"])
                .and_then(|output| parse_tool_dir_output(&output))
                .map(|tool_dir| tool_dir.join("astrbot"));
        let (port, enabled) = deployment
            .as_deref()
            .map(read_dashboard_config)
            .unwrap_or((DEFAULT_WEBUI_PORT, true));

        let running = enabled && verify_webui(port);

        Ok(vec![DiscoveredInstance {
            source: self.id().to_string(),
            id: instance_id(deployment.as_deref(), &version),
            name: instance_name(&version),
            webui_url: resolve_dashboard_url(enabled, port),
            state: if running {
                InstanceState::Running
            } else {
                InstanceState::Stopped
            },
            capabilities: InstanceCapabilities { read_only: true },
        }])
    }
}

/// Stable instance id derived from the deployment directory, falling back
/// to the version when the directory could not be resolved.
fn instance_id(deployment: Option<&Path>, version: &str) -> String {
    deployment
        .map(|dir| dir.to_string_lossy().to_string())
        .unwrap_or_else(|| format!("uv-tool:{version}"))
}

/// Display name, enriched with the tool version when it could be read.
fn instance_name(version: &str) -> String {
    if version.is_empty() {
        "AstrBot CLI".to_string()
    } else {
        format!("AstrBot CLI ({version})")
    }
}

/// Extract the astrbot entry (and its version) from `uv tool list` output.
///
/// Entries look like:
/// ```text
/// astrbot v4.29.4
/// - astrbot
/// ```
fn parse_tool_list(listing: &str) -> Option<String> {
    listing.lines().find_map(|line| {
        let line = line.trim();
        let rest = line.strip_prefix("astrbot")?;
        if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
            // Another tool whose name merely starts with "astrbot".
            return None;
        }
        let version = rest.trim().trim_start_matches('v');
        Some(version.to_string())
    })
}

/// Parse `uv tool dir` output into a single path line.
fn parse_tool_dir_output(output: &str) -> Option<PathBuf> {
    output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tool_list_takes_astrbot_entry() {
        let listing = "astrbot v4.29.4\n- astrbot\n- astrbot-cli\n\nruff v0.1.0\n- ruff\n";
        assert_eq!(parse_tool_list(listing), Some("4.29.4".to_string()));
        assert_eq!(parse_tool_list("astrbot v0.1.2"), Some("0.1.2".to_string()));
        // Only tools whose name is exactly astrbot count.
        assert_eq!(parse_tool_list("astrbot-dev v1.0.0\n"), None);
        assert_eq!(parse_tool_list("ruff v0.1.0\n- ruff\n"), None);
        assert_eq!(parse_tool_list(""), None);
    }

    #[test]
    fn instance_state_and_metadata() {
        assert_eq!(instance_name("4.29.4"), "AstrBot CLI (4.29.4)".to_string());
        assert_eq!(instance_name(""), "AstrBot CLI".to_string());
        assert_eq!(
            instance_id(
                Some(Path::new("C:\\Users\\u\\uv\\tools\\astrbot")),
                "4.29.4"
            ),
            "C:\\Users\\u\\uv\\tools\\astrbot"
        );
        assert_eq!(instance_id(None, "4.29.4"), "uv-tool:4.29.4");
    }
}
