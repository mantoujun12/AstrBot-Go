//! Discovery source for AstrBot instances deployed via Docker.
//!
//! Runs `docker ps` with array-form arguments (never shell concatenation),
//! filters for AstrBot images, and maps the container that exposes port
//! 6185 to a read-only discovered instance. Docker may be absent or its
//! daemon may be down; in that case the source silently degrades to an
//! empty list instead of failing the whole scan.

use super::discovery::{
    locate_program, run_capped, DiscoveredInstance, DiscoveryContext, InstanceCapabilities,
    InstanceSource,
};

use crate::error::Result;
use crate::process::InstanceState;

/// Docker source id.
pub const SOURCE_ID: &str = "docker";

/// The AstrBot WebUI port inside the container.
const WEBUI_PORT: u16 = 6185;

/// Image fragment identifying an official AstrBot image. Also matches
/// registry-prefixed mirrors such as `m.daocloud.io/docker.io/soulter/astrbot`.
const IMAGE_MARK: &str = "soulter/astrbot";

/// One row of `docker ps --format {{.ID}}\t{{.Image}}\t{{.Names}}\t{{.Ports}}\t{{.State}}`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DockerContainerRow {
    id: String,
    image: String,
    names: String,
    ports: String,
    state: String,
}

/// Parse tab-separated `docker ps --format` output rows.
fn parse_rows(output: &str) -> Vec<DockerContainerRow> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut fields = line.splitn(5, '\t');
            let take = |field: Option<&str>| field.unwrap_or_default().trim().to_string();
            DockerContainerRow {
                id: take(fields.next()),
                image: take(fields.next()),
                names: take(fields.next()),
                ports: take(fields.next()),
                state: take(fields.next()),
            }
        })
        .collect()
}

/// Whether the row plausibly represents an AstrBot deployment.
///
/// The image is the primary signal; the container name only acts as a
/// supporting hint so unrelated images (and unrelated names) are not
/// reported by mistake.
fn row_matches(row: &DockerContainerRow) -> bool {
    if row.image.contains(IMAGE_MARK) {
        return true;
    }
    // Auxiliary check: a non-official image must agree with the container
    // name before we accept it as an AstrBot deployment.
    row.image.contains("astrbot") && row.names.contains("astrbot")
}

/// Extract the host-side address and port of the first port mapping that
/// forwards the container WebUI port.
///
/// Port entries look like `0.0.0.0:6185->6185/tcp, [::]:6185->6185/tcp`
/// (mapped) or `6185/tcp` (exposed but not published, hence no host port).
/// The published host address is retained so a port published on a specific
/// interface gets a URL that can actually reach it; wildcard bindings map
/// to loopback.
fn host_port_for(ports: &str, target: u16) -> Option<(String, u16)> {
    ports.split(',').find_map(|entry| {
        let entry = entry.trim();
        let (host, container) = entry.split_once("->")?;
        if !container.starts_with(&format!("{target}/")) {
            return None;
        }
        // Host side may be `0.0.0.0:6185`, `[::]:6185`, or `6185`.
        let (address, port) = match host.rsplit_once(':') {
            Some((address, port)) => (address, port),
            None => ("127.0.0.1", host),
        };
        let port = port.parse::<u16>().ok()?;
        let address = match address {
            "0.0.0.0" | "" | "[::]" => "127.0.0.1".to_string(),
            other => other.to_string(),
        };
        Some((address, port))
    })
}

/// Map the Docker container state to the launcher instance state.
fn map_state(state: &str) -> InstanceState {
    if state.eq_ignore_ascii_case("running") {
        InstanceState::Running
    } else {
        InstanceState::Stopped
    }
}

/// Discovery source backed by the local Docker daemon.
pub struct DockerSource;

impl InstanceSource for DockerSource {
    fn id(&self) -> &'static str {
        SOURCE_ID
    }

    fn title_key(&self) -> &'static str {
        "settings.instances.sources.docker"
    }

    fn discover(&self, _ctx: &DiscoveryContext) -> Result<Vec<DiscoveredInstance>> {
        // Resolve the docker CLI to an absolute path; relative/working-dir
        // matches on PATH are rejected so a dropped-in `docker` executable
        // is never executed automatically.
        let Some(docker) = locate_program("docker") else {
            // Docker CLI absent: degrade silently to an empty list.
            return Ok(Vec::new());
        };
        // `-a` also lists stopped containers so they keep appearing with a
        // stopped state instead of vanishing from the list.
        // Arguments are passed as an array; the format string is never
        // concatenated into a shell command. The command is capped so an
        // unresponsive daemon cannot hang the discovery load.
        let Some(output) = run_capped(
            docker.to_string_lossy().as_ref(),
            &[
                "ps",
                "-a",
                "--format",
                "{{.ID}}\t{{.Image}}\t{{.Names}}\t{{.Ports}}\t{{.State}}",
            ],
        ) else {
            // Silent degradation: docker CLI missing, daemon not running, or
            // any other failure yields an empty list.
            return Ok(Vec::new());
        };

        let instances = parse_rows(&output)
            .into_iter()
            .filter(row_matches)
            .map(|row| {
                let webui = host_port_for(&row.ports, WEBUI_PORT);
                DiscoveredInstance {
                    source: SOURCE_ID.to_string(),
                    id: format!("docker:{}", row.id),
                    name: row.names.split(',').next().unwrap_or_default().to_string(),
                    webui_url: webui.map(|(host, port)| format!("http://{host}:{port}")),
                    state: map_state(&row.state),
                    // Docker deployments are observed only; lifecycle stays
                    // with the container runtime.
                    capabilities: InstanceCapabilities { read_only: true },
                }
            })
            .collect();
        Ok(instances)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(image: &str, names: &str, ports: &str, state: &str) -> DockerContainerRow {
        DockerContainerRow {
            id: "abc123".to_string(),
            image: image.to_string(),
            names: names.to_string(),
            ports: ports.to_string(),
            state: state.to_string(),
        }
    }

    #[test]
    fn parses_tab_separated_rows() {
        let rows = parse_rows(
            "a1\tm.daocloud.io/docker.io/soulter/astrbot:latest\tastrbot-web\t0.0.0.0:6185->6185/tcp\trunning\nb2\tnginx:latest\tweb\t80/tcp\texited\n",
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, "a1");
        assert_eq!(
            rows[0].image,
            "m.daocloud.io/docker.io/soulter/astrbot:latest"
        );
        assert_eq!(rows[0].ports, "0.0.0.0:6185->6185/tcp");
        assert_eq!(rows[1].state, "exited");
        assert!(parse_rows("").is_empty());
    }

    #[test]
    fn matches_official_and_mirrored_images_only() {
        assert!(row_matches(&row(
            "soulter/astrbot:latest",
            "misc",
            "",
            "running"
        )));
        assert!(row_matches(&row(
            "m.daocloud.io/docker.io/soulter/astrbot:v4",
            "astrbot",
            "",
            "running"
        )));
        // Auxiliary path: uncommon astrbot image plus matching name.
        assert!(row_matches(&row(
            "registry.local/astrbot:dev",
            "astrbot-1",
            "",
            "running"
        )));
        // Unrelated image with an astrbot-looking name is rejected.
        assert!(!row_matches(&row(
            "nginx:latest",
            "astrbot-proxy",
            "",
            "running"
        )));
        // Non-AstrBot image without an agreeing name is rejected.
        assert!(!row_matches(&row("nginx:latest", "web", "", "running")));
    }

    #[test]
    fn extracts_host_port_for_webui_mapping() {
        assert_eq!(
            host_port_for("0.0.0.0:6185->6185/tcp, [::]:6185->6185/tcp", 6185),
            Some(("127.0.0.1".to_string(), 6185))
        );
        assert_eq!(
            host_port_for("0.0.0.0:7000->6185/tcp", 6185),
            Some(("127.0.0.1".to_string(), 7000))
        );
        // A port published on a specific interface keeps that address.
        assert_eq!(
            host_port_for("192.168.1.7:6185->6185/tcp", 6185),
            Some(("192.168.1.7".to_string(), 6185))
        );
        // Exposed without publishing: no host port.
        assert_eq!(host_port_for("6185/tcp", 6185), None);
        // Only other ports are mapped.
        assert_eq!(host_port_for("0.0.0.0:8080->8080/tcp", 6185), None);
        assert_eq!(host_port_for("", 6185), None);
    }

    #[test]
    fn maps_container_state() {
        assert_eq!(map_state("running"), InstanceState::Running);
        assert_eq!(map_state("RUNNING"), InstanceState::Running);
        assert_eq!(map_state("exited"), InstanceState::Stopped);
        assert_eq!(map_state("paused"), InstanceState::Stopped);
        assert_eq!(map_state(""), InstanceState::Stopped);
    }
}
