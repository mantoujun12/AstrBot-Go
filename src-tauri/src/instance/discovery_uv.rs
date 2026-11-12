//! Discovery source for AstrBot instances deployed through the official
//! `uv tool` one-shot script, which installs the `astrbot` CLI on PATH.
//!
//! This source is read-only: it locates the CLI, best-effort reads its
//! version, probes the default WebUI port, and reports the result without
//! touching the deployment.

use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::error::Result;
use crate::process::InstanceState;

use super::discovery::{
    DiscoveredInstance, DiscoveryContext, InstanceCapabilities, InstanceSource,
};

/// Default WebUI port used by AstrBot; the uv tool deployment does not
/// expose a way to read a customized port from the outside, so a non-default
/// deployment would still be reported on this URL (known limitation).
pub const DEFAULT_WEBUI_PORT: u16 = 6185;

/// Upper bound for each subprocess (`where`/`which`, `astrbot --version`).
const COMMAND_TIMEOUT: Duration = Duration::from_secs(3);

/// TCP connect timeout for the WebUI liveness probe.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);

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
        let Some(exe) = locate_astrbot() else {
            // astrbot (and therefore the uv tool deployment) is absent:
            // degrade silently to an empty list.
            return Ok(Vec::new());
        };

        let version = read_version(&exe).unwrap_or_default();
        let running = probe_webui();

        Ok(vec![DiscoveredInstance {
            source: self.id().to_string(),
            id: instance_id(&exe),
            name: instance_name(&version),
            webui_url: Some(format!("http://127.0.0.1:{DEFAULT_WEBUI_PORT}")),
            state: if running {
                InstanceState::Running
            } else {
                InstanceState::Stopped
            },
            capabilities: InstanceCapabilities { read_only: true },
        }])
    }
}

/// Stable instance id derived from the resolved executable path.
fn instance_id(exe: &Path) -> String {
    exe.to_string_lossy().to_string()
}

/// Display name, enriched with the CLI version when it could be read.
fn instance_name(version: &str) -> String {
    if version.is_empty() {
        "AstrBot CLI".to_string()
    } else {
        format!("AstrBot CLI ({version})")
    }
}

/// Resolve the `astrbot` executable on PATH, or `None` when it is absent.
/// Uses `where` on Windows and `which` elsewhere.
fn locate_astrbot() -> Option<PathBuf> {
    let (program, arg) = locate_command();
    let text = run_capped(program, &[arg])?;
    parse_locate_output(&text)
}

/// Parse `where`/`which` stdout into the first non-empty path line.
/// Expects the output of a successful invocation; returns `None` when no
/// usable line is present.
fn parse_locate_output(text: &str) -> Option<PathBuf> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(PathBuf::from)
}

/// Best-effort `astrbot --version`; `None` when the CLI exits non-zero,
/// times out, or prints no recognizable version.
fn read_version(exe: &Path) -> Option<String> {
    let text = run_capped(exe.to_string_lossy().as_ref(), &["--version"])?;
    parse_version_output(&text)
}

/// Extract the first `dotted-number` version token from CLI output
/// (e.g. `astrbot 4.29.4` or a bare `4.29.4` line).
fn parse_version_output(text: &str) -> Option<String> {
    let mut token = String::new();
    let mut has_digit = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() || (ch == '.' && has_digit) {
            token.push(ch);
            if ch.is_ascii_digit() {
                has_digit = true;
            }
        } else if has_digit {
            break;
        } else {
            token.clear();
        }
    }
    // Trim a trailing dot left over from a dangling separator.
    let token = token.trim_end_matches('.');
    if has_digit {
        Some(token.to_string())
    } else {
        None
    }
}

/// Probe the default WebUI port; `true` when something is listening.
fn probe_webui() -> bool {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, DEFAULT_WEBUI_PORT));
    TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).is_ok()
}

/// Command used to resolve executables on PATH for the current platform.
fn locate_command() -> (&'static str, &'static str) {
    if cfg!(windows) {
        ("where", "astrbot")
    } else {
        ("which", "astrbot")
    }
}

/// Run a short-lived command with an upper bound on waiting. Output is
/// captured with pipes and no inherited console; a hang beyond
/// [`COMMAND_TIMEOUT`] is killed and reported as unavailable.
fn run_capped(program: &str, args: &[&str]) -> Option<String> {
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
            log::debug!("uv-tool discovery: `{program}` unavailable: {error}");
            return None;
        }
    };
    let mut stdout = child.stdout.take()?;

    let handle = Arc::new(Mutex::new(child));
    let waiter = Arc::clone(&handle);
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let join = std::thread::spawn(move || {
        if let Ok(mut child) = waiter.lock() {
            let _ = child.wait();
        }
        let _ = tx.send(());
    });

    let timed_out = matches!(
        rx.recv_timeout(COMMAND_TIMEOUT),
        Err(RecvTimeoutError::Timeout) | Err(RecvTimeoutError::Disconnected)
    );

    if timed_out {
        // Best-effort kill of the hung child before it is reaped.
        if let Ok(mut child) = handle.lock() {
            let _ = child.kill();
        }
        let _ = join.join();
        return None;
    }
    let _ = join.join();

    // The child has exited, so both pipes reach EOF promptly.
    let success = handle
        .lock()
        .ok()
        .and_then(|mut child| child.wait().ok())
        .is_some_and(|status| status.success());
    if !success {
        return None;
    }
    let mut text = String::new();
    let _ = std::io::Read::read_to_string(&mut stdout, &mut text);
    if text.trim().is_empty() {
        return None;
    }
    Some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_locate_output_takes_first_non_empty_line() {
        let text = "\n  C:\\Users\\u\\.local\\bin\\astrbot.exe  \nC:\\Windows\\astrbot.exe\n";
        assert_eq!(
            parse_locate_output(text),
            Some(PathBuf::from("C:\\Users\\u\\.local\\bin\\astrbot.exe"))
        );
        assert_eq!(parse_locate_output("   \n\t\n"), None);
        assert_eq!(parse_locate_output(""), None);
    }

    #[test]
    fn parse_version_output_extracts_dotted_version() {
        assert_eq!(
            parse_version_output("astrbot 4.29.4\n"),
            Some("4.29.4".to_string())
        );
        assert_eq!(parse_version_output("4.29.4"), Some("4.29.4".to_string()));
        assert_eq!(
            parse_version_output("AstrBot CLI\nversion: 1.2.3-beta"),
            Some("1.2.3".to_string())
        );
        assert_eq!(parse_version_output("no version here"), None);
        assert_eq!(parse_version_output(""), None);
    }

    #[test]
    fn instance_state_and_metadata_follow_probe() {
        assert_eq!(
            super::instance_name("4.29.4"),
            "AstrBot CLI (4.29.4)".to_string()
        );
        assert_eq!(super::instance_name(""), "AstrBot CLI".to_string());
        assert_eq!(
            super::instance_id(&PathBuf::from("C:\\bin\\astrbot.exe")),
            "C:\\bin\\astrbot.exe"
        );
    }

    #[test]
    fn locate_command_matches_platform() {
        let (program, _) = locate_command();
        if cfg!(windows) {
            assert_eq!(program, "where");
        } else {
            assert_eq!(program, "which");
        }
    }
}
