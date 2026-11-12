//! Instance source discovery framework.
//!
//! Instances may come from different deployment sources (launcher-managed
//! instances, Docker containers, uv tool environments, plain directory
//! scans). Each source registers an [`InstanceSource`] implementation in the
//! [`InstanceSourceRegistry`], and the unified discovery command returns the
//! merged instance list together with the registered source metadata.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::LazyLock;
use std::time::Duration;

use serde::Serialize;

use crate::config::AppManifest;
use crate::error::Result;
use crate::process::{InstanceState, ProcessManager};

use super::list_instances;

/// Capability flags describing what the launcher may do with an instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub struct InstanceCapabilities {
    /// Instances from external sources are observed only: start, stop, and
    /// update remain reserved for launcher-managed instances until dedicated
    /// integrations land.
    pub read_only: bool,
}

/// A unified instance record produced by a discovery source.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredInstance {
    /// Stable id of the source that reported this instance.
    pub source: String,
    /// Stable instance id within its source.
    pub id: String,
    pub name: String,
    /// WebUI origin, when the instance exposes a dashboard.
    pub webui_url: Option<String>,
    pub state: InstanceState,
    pub capabilities: InstanceCapabilities,
}

/// Metadata of a registered discovery source, used to render the settings.
#[derive(Debug, Clone, Serialize)]
pub struct InstanceSourceInfo {
    pub id: String,
    /// i18n key resolving the source display name.
    pub title_key: String,
}

/// Snapshot returned by the unified discovery command.
#[derive(Debug, Clone, Serialize, Default)]
pub struct DiscoveredInstancesSnapshot {
    pub sources: Vec<InstanceSourceInfo>,
    pub instances: Vec<DiscoveredInstance>,
}

/// Resources a discovery source may consult while scanning.
pub struct DiscoveryContext<'a> {
    pub process_manager: &'a ProcessManager,
    pub manifest: &'a AppManifest,
    /// Configured roots for the directory-scan source; empty = home dir.
    pub scan_paths: Vec<String>,
}

/// A provider of instances from one deployment mechanism.
pub trait InstanceSource: Send + Sync {
    /// Stable source id, also used as the settings toggle key.
    fn id(&self) -> &'static str;

    /// i18n key resolving the source display name.
    fn title_key(&self) -> &'static str;

    fn discover(&self, ctx: &DiscoveryContext) -> Result<Vec<DiscoveredInstance>>;
}

/// Registry of all instance sources; the list exit iterates it.
pub struct InstanceSourceRegistry {
    sources: Vec<Box<dyn InstanceSource>>,
}

impl InstanceSourceRegistry {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Append a source; later sources appear after earlier ones.
    pub fn register(&mut self, source: Box<dyn InstanceSource>) {
        self.sources.push(source);
    }

    pub fn source_infos(&self) -> Vec<InstanceSourceInfo> {
        self.sources
            .iter()
            .map(|source| InstanceSourceInfo {
                id: source.id().to_string(),
                title_key: source.title_key().to_string(),
            })
            .collect()
    }

    /// Run every enabled source. A failing source is logged and skipped so
    /// one broken integration cannot hide the remaining instances.
    pub fn discover_all(
        &self,
        ctx: &DiscoveryContext,
        is_enabled: &dyn Fn(&str) -> bool,
    ) -> Vec<DiscoveredInstance> {
        let mut all = Vec::new();
        for source in &self.sources {
            if !is_enabled(source.id()) {
                continue;
            }
            match source.discover(ctx) {
                Ok(instances) => all.extend(instances),
                Err(error) => {
                    log::warn!("Instance source {} failed: {}", source.id(), error);
                }
            }
        }
        all
    }
}

impl Default for InstanceSourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// The built-in source: instances deployed and managed by the launcher itself.
struct LauncherManagedSource;

impl InstanceSource for LauncherManagedSource {
    fn id(&self) -> &'static str {
        "launcher"
    }

    fn title_key(&self) -> &'static str {
        "settings.instances.sources.launcher"
    }

    fn discover(&self, ctx: &DiscoveryContext) -> Result<Vec<DiscoveredInstance>> {
        Ok(list_instances(ctx.process_manager, ctx.manifest)
            .into_iter()
            .map(|status| DiscoveredInstance {
                source: self.id().to_string(),
                id: status.id,
                name: status.name,
                webui_url: status.dashboard_enabled.then(|| {
                    // Match the list-view URL logic: a live instance may run
                    // on a runtime port different from its configured one.
                    format!("http://{}:{}", status.configured_host, status.port)
                }),
                state: status.state,
                capabilities: InstanceCapabilities { read_only: false },
            })
            .collect())
    }
}

/// Upper bound for each discovery subprocess (`where`/`which`, `docker ps`,
/// `uv tool list`).
pub(crate) const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

/// Default TCP connect timeout for WebUI liveness probes.
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);

/// Run a short-lived command with an upper bound on waiting. Output is
/// captured with pipes and no inherited console; a hang beyond
/// [`COMMAND_TIMEOUT`] is killed by PID and reported as unavailable.
///
/// The child is owned by the waiter thread, so the timeout path can kill it
/// by PID without contending with `wait()` for a shared mutex.
pub(crate) fn run_capped(program: &str, args: &[&str]) -> Option<String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        use windows::Win32::System::Threading::CREATE_NO_WINDOW;
        command.creation_flags(CREATE_NO_WINDOW.0);
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            log::debug!("discovery: `{program}` unavailable: {error}");
            return None;
        }
    };
    let pid = child.id();
    let stdout = child.stdout.take()?;

    // Reader thread drains stdout concurrently so a chatty child cannot
    // deadlock on a full pipe while we wait for its exit status.
    let reader = std::thread::spawn(move || {
        let mut text = String::new();
        let mut stdout = stdout;
        let _ = std::io::Read::read_to_string(&mut stdout, &mut text);
        text
    });
    let (tx, rx) = std::sync::mpsc::channel::<std::io::Result<std::process::ExitStatus>>();
    let join = std::thread::spawn(move || {
        let _ = tx.send(child.wait());
    });

    let received = match rx.recv_timeout(COMMAND_TIMEOUT) {
        Ok(status) => Some(status),
        Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected) => {
            kill_process(pid);
            let _ = join.join();
            None
        }
    };
    let success =
        received.is_some_and(|status| status.as_ref().is_ok_and(|status| status.success()));
    let text = reader.join().unwrap_or_default();
    if !success || text.trim().is_empty() {
        return None;
    }
    Some(text)
}

/// Forcefully terminate a spawned discovery child by PID. Uses the platform
/// termination tool, which lives in system directories and cannot be
/// shadowed by user PATH entries.
fn kill_process(pid: u32) {
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("taskkill");
        command.args(["/F", "/T", "/PID", &pid.to_string()]);
        command
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut command = Command::new("kill");
        command.args(["-9", &pid.to_string()]);
        command
    };
    command.stdout(Stdio::null()).stderr(Stdio::null());
    let _ = command.spawn();
}

/// Resolve an external program to an absolute path, or `None` when it is
/// absent. Uses `where` on Windows and `which` elsewhere.
///
/// Matches inside the launcher's current working directory are rejected:
/// on Windows relative PATH entries are searched implicitly, so a dropped-in
/// executable next to the launcher must not be executed automatically.
pub(crate) fn locate_program(name: &str) -> Option<PathBuf> {
    let (locator, arg) = if cfg!(windows) {
        ("where", name)
    } else {
        ("which", name)
    };
    let text = run_capped(locator, &[arg])?;
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .find(|path| is_trusted_location(path))
}

/// Reject relative paths and matches inside the launcher's working
/// directory (the implicit Windows search target).
fn is_trusted_location(path: &std::path::Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    if let Ok(cwd) = std::env::current_dir() {
        if path.starts_with(&cwd) {
            return false;
        }
    }
    true
}

/// Check whether an AstrBot WebUI is actually listening on `port`:
/// a TCP connect followed by an HTTP `GET /` whose response looks like the
/// AstrBot dashboard. A plain TCP probe alone would accept any unrelated
/// listener on the same port (issue #81 false-positive guard).
pub(crate) fn verify_webui(port: u16) -> bool {
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    let Ok(mut stream) = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(CONNECT_TIMEOUT));
    let _ = stream.set_write_timeout(Some(CONNECT_TIMEOUT));
    let request = format!("GET / HTTP/1.0\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    if std::io::Write::write_all(&mut stream, request.as_bytes()).is_err() {
        return false;
    }
    let mut response = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        match std::io::Read::read(&mut stream, &mut chunk) {
            Ok(0) => break,
            Ok(read) => {
                response.extend_from_slice(&chunk[..read]);
                if response.len() >= 16 * 1024 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let text = String::from_utf8_lossy(&response);
    let Some(head_end) = text.find("\r\n\r\n") else {
        return false;
    };
    let (head, body) = text.split_at(head_end);
    // Any 2xx/3xx status plus an AstrBot marker in the response counts as
    // the dashboard; unrelated services are reported as stopped.
    let status_ok = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .is_some_and(|code| (200..400).contains(&code));
    status_ok && body.to_ascii_lowercase().contains("astrbot")
}

fn builtin_registry() -> InstanceSourceRegistry {
    let mut registry = InstanceSourceRegistry::new();
    registry.register(Box::new(LauncherManagedSource));
    registry.register(Box::new(super::discovery_docker::DockerSource));
    registry.register(Box::new(super::discovery_uv::UvToolSource));
    registry.register(Box::new(super::discovery_dirscan::DirScanSource));
    registry
}

static GLOBAL_REGISTRY: LazyLock<InstanceSourceRegistry> = LazyLock::new(builtin_registry);

/// Registry used by the app; later integrations register additional sources
/// in [`builtin_registry`].
pub fn global_registry() -> &'static InstanceSourceRegistry {
    &GLOBAL_REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    struct StaticSource {
        source_id: &'static str,
        fail: bool,
    }

    impl InstanceSource for StaticSource {
        fn id(&self) -> &'static str {
            self.source_id
        }

        fn title_key(&self) -> &'static str {
            "settings.instances.sources.launcher"
        }

        fn discover(&self, _ctx: &DiscoveryContext) -> Result<Vec<DiscoveredInstance>> {
            if self.fail {
                return Err(AppError::other("probe failure"));
            }
            Ok(vec![DiscoveredInstance {
                source: self.source_id.to_string(),
                id: format!("{}-1", self.source_id),
                name: "Probe".to_string(),
                webui_url: None,
                state: InstanceState::Stopped,
                capabilities: InstanceCapabilities::default(),
            }])
        }
    }

    fn with_context<T>(run: impl FnOnce(&DiscoveryContext) -> T) -> T {
        let process_manager = ProcessManager::new();
        let manifest = AppManifest::default();
        let ctx = DiscoveryContext {
            process_manager: &process_manager,
            manifest: &manifest,
            scan_paths: Vec::new(),
        };
        run(&ctx)
    }

    fn registry_with(fail_second: bool) -> InstanceSourceRegistry {
        let mut registry = InstanceSourceRegistry::new();
        registry.register(Box::new(StaticSource {
            source_id: "alpha",
            fail: false,
        }));
        registry.register(Box::new(StaticSource {
            source_id: "beta",
            fail: fail_second,
        }));
        registry
    }

    #[test]
    fn discover_all_skips_disabled_and_failing_sources() {
        let (all, only_alpha, degraded) = with_context(|ctx| {
            let all = registry_with(false).discover_all(ctx, &|_| true);
            let only_alpha = registry_with(false).discover_all(ctx, &|id| id == "alpha");
            // A failing source is skipped instead of failing the whole scan.
            let degraded = registry_with(true).discover_all(ctx, &|_| true);
            (all, only_alpha, degraded)
        });
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].source, "alpha");
        assert_eq!(all[1].source, "beta");
        assert_eq!(only_alpha.len(), 1);
        assert_eq!(only_alpha[0].source, "alpha");
        assert_eq!(degraded.len(), 1);
        assert_eq!(degraded[0].source, "alpha");
    }

    #[test]
    fn source_infos_follow_registration_order() {
        let registry = registry_with(false);
        let infos = registry.source_infos();
        assert_eq!(infos.len(), 2);
        assert_eq!(infos[0].id, "alpha");
        assert_eq!(infos[1].id, "beta");
        assert!(!infos[0].title_key.is_empty());
    }

    #[test]
    fn global_registry_exposes_builtin_sources() {
        let registry = global_registry();
        let infos = registry.source_infos();
        assert_eq!(infos.len(), 4);
        assert_eq!(infos[0].id, "launcher");
        assert_eq!(infos[0].title_key, "settings.instances.sources.launcher");
        assert_eq!(infos[1].id, "docker");
        assert_eq!(infos[1].title_key, "settings.instances.sources.docker");
        assert_eq!(infos[2].id, "uv_tool");
        assert_eq!(infos[2].title_key, "settings.instances.sources.uv_tool");
        assert_eq!(infos[3].id, "dir_scan");
        assert_eq!(infos[3].title_key, "settings.instances.sources.dir_scan");

        // Deterministic subset: only the dir-scan source with an empty scan
        // root. Docker/uv sources depend on the host and are covered by
        // their own unit tests with injected commands.
        let ctx = DiscoveryContext {
            process_manager: &ProcessManager::new(),
            manifest: &AppManifest::default(),
            scan_paths: vec![],
        };
        let dirscan_only = registry.discover_all(&ctx, &|id| id == "dir_scan");
        // Empty default scan root on a bare environment, but whatever is
        // found must be an external read-only dir-scan instance.
        assert!(dirscan_only
            .iter()
            .all(|instance| instance.source == "dir_scan" && instance.capabilities.read_only));
    }
}
