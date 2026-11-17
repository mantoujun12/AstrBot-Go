use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::utils::log_redact;

/// Maximum time to wait for a `<exe> --version` subprocess before killing it.
/// A hung runtime must not stall the snapshot task forever.
const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

/// Poll interval while waiting for the probe subprocess to exit.
const PROBE_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Cache of detected runtime versions, keyed by `<component>/<runtime>`.
/// Avoids launching `--version` subprocesses on every app snapshot rebuild;
/// invalidated on component install/uninstall/reinstall. A cached `None`
/// marks an executable whose version could not be detected.
static VERSION_CACHE: Mutex<Option<HashMap<String, Option<String>>>> = Mutex::new(None);

/// Detect and cache a runtime version. `probe` runs only on cache misses.
pub(super) fn cached_runtime_version(
    key: &str,
    probe: impl FnOnce() -> Option<String>,
) -> Option<String> {
    {
        let guard = VERSION_CACHE.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(version) = guard.as_ref().and_then(|cache| cache.get(key)) {
            return version.clone();
        }
    } // cache guard dropped before probing: `probe` may launch subprocesses

    let detected = probe();

    let mut guard = VERSION_CACHE.lock().unwrap_or_else(|p| p.into_inner());
    guard
        .get_or_insert_with(HashMap::new)
        .insert(key.to_string(), detected.clone());
    detected
}

/// Drop all cached versions after a component mutation.
pub(super) fn invalidate_version_cache() {
    *VERSION_CACHE.lock().unwrap_or_else(|p| p.into_inner()) = None;
}

/// Run `<exe> --version` synchronously and return the trimmed stdout, or
/// `None` when the executable is missing, cannot be launched, hangs past the
/// timeout, or exits with an error (all failures are logged). Intended for
/// the components snapshot, which already runs on a blocking thread. Callers
/// parse the raw banner into a clean version.
pub(super) fn probe_exe_version(exe: &Path) -> Option<String> {
    if !exe.exists() {
        return None;
    }

    let mut cmd = Command::new(exe);
    cmd.arg("--version").stdout(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt as _;
        use windows::Win32::System::Threading::CREATE_NO_WINDOW;
        cmd.creation_flags(CREATE_NO_WINDOW.0);
    }

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            log::warn!(
                "Failed to launch version probe for {}: {}",
                log_redact::path_name(exe),
                e
            );
            return None;
        }
    };

    // Poll for exit with a deadline; `output()` would wait forever on a hung
    // runtime and stall the snapshot task.
    // shortcut: the deadline loop has no unit test (needs a real hanging
    // process); if a version chip ever goes missing, check the probe logs.
    let deadline = Instant::now() + PROBE_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(PROBE_POLL_INTERVAL);
            }
            _ => break None,
        }
    };

    if status.is_none() {
        let _ = child.kill();
        let _ = child.wait();
        log::warn!("Version probe timed out for {}", log_redact::path_name(exe));
        return None;
    }

    if !status.map(|s| s.success()).unwrap_or(false) {
        log::warn!(
            "Version probe exited with failure for {}",
            log_redact::path_name(exe)
        );
        return None;
    }

    // stdout is a tiny `--version` banner; reading after exit cannot deadlock.
    let mut stdout = String::new();
    if let Some(stream) = child.stdout.as_mut() {
        use std::io::Read as _;
        if stream.read_to_string(&mut stdout).is_err() {
            log::warn!(
                "Failed to read version probe output for {}",
                log_redact::path_name(exe)
            );
            return None;
        }
    }

    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Strip a known banner prefix (e.g. `Python `, `v`, `uv `) from a version
/// output and return the clean version string.
pub(super) fn parse_version_output(raw: &str, prefix: &str) -> Option<String> {
    let trimmed = raw.trim();
    let version = trimmed.strip_prefix(prefix).unwrap_or(trimmed).trim();
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::parse_version_output;

    #[test]
    fn parses_python_banner() {
        assert_eq!(
            parse_version_output("Python 3.12.7\n", "Python ").as_deref(),
            Some("3.12.7")
        );
    }

    #[test]
    fn parses_node_banner() {
        assert_eq!(
            parse_version_output("v22.11.0", "v").as_deref(),
            Some("22.11.0")
        );
    }

    #[test]
    fn parses_uv_banner() {
        assert_eq!(
            parse_version_output("uv 0.5.11", "uv ").as_deref(),
            Some("0.5.11")
        );
    }

    #[test]
    fn rejects_empty_output() {
        assert_eq!(parse_version_output("   ", "Python "), None);
        assert_eq!(parse_version_output("", "uv "), None);
    }

    #[test]
    fn keeps_unprefixed_version() {
        assert_eq!(
            parse_version_output("3.10.11", "Python ").as_deref(),
            Some("3.10.11")
        );
    }

    #[test]
    fn caches_probe_result_until_invalidated() {
        let key = "test/probe";
        super::invalidate_version_cache();
        let mut calls = 0;
        let first = super::cached_runtime_version(key, || {
            calls += 1;
            Some("1.2.3".to_string())
        });
        let second = super::cached_runtime_version(key, || {
            calls += 1;
            Some("9.9.9".to_string())
        });
        assert_eq!(first.as_deref(), Some("1.2.3"));
        assert_eq!(second.as_deref(), Some("1.2.3"));
        assert_eq!(calls, 1);

        super::invalidate_version_cache();
        let third = super::cached_runtime_version(key, || {
            calls += 1;
            None
        });
        assert_eq!(third, None);
        assert_eq!(calls, 2);
        // A cached miss is not re-probed until the next invalidation.
        let fourth = super::cached_runtime_version(key, || {
            calls += 1;
            Some("4.4.4".to_string())
        });
        assert_eq!(fourth, None);
        assert_eq!(calls, 2);
        super::invalidate_version_cache();
    }
}
