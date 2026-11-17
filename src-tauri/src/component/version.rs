use std::path::Path;
use std::process::Command;

/// Run `<exe> --version` synchronously and return the trimmed stdout.
/// Returns `None` when the executable is missing, fails to launch, or exits
/// with an error. Intended for the components snapshot, which already runs on
/// a blocking thread. Callers parse the raw banner into a clean version.
pub(super) fn probe_exe_version(exe: &Path) -> Option<String> {
    if !exe.exists() {
        return None;
    }

    let mut cmd = Command::new(exe);
    cmd.arg("--version");

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt as _;
        use windows::Win32::System::Threading::CREATE_NO_WINDOW;
        cmd.creation_flags(CREATE_NO_WINDOW.0);
    }

    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
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
}
