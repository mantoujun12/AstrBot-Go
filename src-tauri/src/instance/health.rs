use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::time::Duration;

use serde::Serialize;
use tokio::process::Command;

use crate::component;
use crate::config::InstanceConfig;
use crate::utils::paths::{
    get_instance_core_dir, get_instance_dir, get_instance_venv_dir, get_venv_python,
};

const DISK_WARNING_BYTES: u64 = 1024 * 1024 * 1024;
const DISK_CRITICAL_BYTES: u64 = 256 * 1024 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
const REQUIREMENTS_AUDIT_SCRIPT: &str = r#"
import importlib.metadata
import json
import sys

from pip._internal.network.session import PipSession
from pip._internal.req.req_file import parse_requirements
from pip._vendor.packaging.markers import default_environment
from pip._vendor.packaging.requirements import Requirement
from pip._vendor.packaging.utils import canonicalize_name

session = PipSession()
try:
    requirements = parse_requirements(sys.argv[1], session=session)
    installed = {
        canonicalize_name(distribution.metadata["Name"]): distribution.version
        for distribution in importlib.metadata.distributions()
        if distribution.metadata.get("Name")
    }
    missing = set()
    for parsed in requirements:
        if not parsed.requirement or parsed.constraint:
            continue
        requirement = Requirement(parsed.requirement)
        if requirement.marker and not requirement.marker.evaluate(default_environment()):
            continue
        version = installed.get(canonicalize_name(requirement.name))
        if version is None or (
            requirement.specifier
            and not requirement.specifier.contains(version, prereleases=True)
        ):
            missing.add(requirement.name)
    print(json.dumps(sorted(missing)))
finally:
    session.close()
"#;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstanceHealthGrade {
    Healthy,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstanceHealthCheckKey {
    Python,
    Venv,
    Dependencies,
    Port,
    DiskSpace,
    Configuration,
    Webui,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstanceHealthCheckStatus {
    Pass,
    Warning,
    Fail,
    Skipped,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InstanceHealthDetailCode {
    RuntimeReady,
    RuntimeMissing,
    RuntimeFailed,
    VenvReady,
    VenvMissing,
    VenvFailed,
    DependenciesReady,
    DependenciesIssues,
    DependenciesCheckFailed,
    RequirementsMissing,
    PipUnavailable,
    VenvRequired,
    DependenciesTimeout,
    PortAvailable,
    PortOccupied,
    PortListening,
    PortUnreachable,
    PortInvalid,
    PortCheckFailed,
    SpaceSufficient,
    SpaceLow,
    SpaceCritical,
    SpaceUnavailable,
    ConfigValid,
    ConfigInvalid,
    ConfigMissing,
    WebuiReachable,
    WebuiUnreachable,
    WebuiNotRunning,
    WebuiDisabled,
    WebuiHostUnsupported,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstanceHealthCheck {
    pub key: InstanceHealthCheckKey,
    pub status: InstanceHealthCheckStatus,
    pub detail_code: InstanceHealthDetailCode,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstanceHealthReport {
    pub overall: InstanceHealthGrade,
    pub checks: Vec<InstanceHealthCheck>,
}

impl InstanceHealthCheck {
    fn new(
        key: InstanceHealthCheckKey,
        status: InstanceHealthCheckStatus,
        detail_code: InstanceHealthDetailCode,
        value: Option<String>,
    ) -> Self {
        Self {
            key,
            status,
            detail_code,
            value,
        }
    }

    fn pass(
        key: InstanceHealthCheckKey,
        detail_code: InstanceHealthDetailCode,
        value: Option<String>,
    ) -> Self {
        Self::new(key, InstanceHealthCheckStatus::Pass, detail_code, value)
    }

    fn warning(
        key: InstanceHealthCheckKey,
        detail_code: InstanceHealthDetailCode,
        value: Option<String>,
    ) -> Self {
        Self::new(key, InstanceHealthCheckStatus::Warning, detail_code, value)
    }

    fn fail(
        key: InstanceHealthCheckKey,
        detail_code: InstanceHealthDetailCode,
        value: Option<String>,
    ) -> Self {
        Self::new(key, InstanceHealthCheckStatus::Fail, detail_code, value)
    }

    fn skipped(
        key: InstanceHealthCheckKey,
        detail_code: InstanceHealthDetailCode,
        value: Option<String>,
    ) -> Self {
        Self::new(key, InstanceHealthCheckStatus::Skipped, detail_code, value)
    }
}

pub(crate) async fn check_instance_health(
    instance_id: &str,
    instance: &InstanceConfig,
    running_port: Option<u16>,
    is_transitioning: bool,
) -> InstanceHealthReport {
    let checks = vec![
        check_python(&instance.version).await,
        check_venv(instance_id).await,
        check_dependencies(instance_id).await,
        check_port(
            instance.host.as_str(),
            running_port.unwrap_or(instance.port),
            running_port.is_some(),
            is_transitioning,
        ),
        check_disk_space(instance_id),
        check_configuration(instance_id),
        check_webui(
            instance.host.as_str(),
            running_port.unwrap_or(instance.port),
            running_port.is_some(),
            super::crud::is_dashboard_enabled(instance_id),
        )
        .await,
    ];

    InstanceHealthReport {
        overall: overall_grade(&checks),
        checks,
    }
}

async fn check_python(version: &str) -> InstanceHealthCheck {
    let executable = match component::get_python_for_version(version) {
        Ok(path) => path,
        Err(_) => {
            return InstanceHealthCheck::fail(
                InstanceHealthCheckKey::Python,
                InstanceHealthDetailCode::RuntimeMissing,
                None,
            );
        }
    };

    match run_command(&executable, &["--version"]).await {
        Ok(output) if output.status.success() => InstanceHealthCheck::pass(
            InstanceHealthCheckKey::Python,
            InstanceHealthDetailCode::RuntimeReady,
            python_version(&output),
        ),
        _ => InstanceHealthCheck::fail(
            InstanceHealthCheckKey::Python,
            InstanceHealthDetailCode::RuntimeFailed,
            None,
        ),
    }
}

async fn check_venv(instance_id: &str) -> InstanceHealthCheck {
    let venv_dir = get_instance_venv_dir(instance_id);
    if !venv_dir.is_dir() {
        return InstanceHealthCheck::fail(
            InstanceHealthCheckKey::Venv,
            InstanceHealthDetailCode::VenvMissing,
            None,
        );
    }

    let executable = get_venv_python(&venv_dir);
    if !executable.is_file() {
        return InstanceHealthCheck::fail(
            InstanceHealthCheckKey::Venv,
            InstanceHealthDetailCode::VenvMissing,
            None,
        );
    }

    match run_command(&executable, &["--version"]).await {
        Ok(output) if output.status.success() => InstanceHealthCheck::pass(
            InstanceHealthCheckKey::Venv,
            InstanceHealthDetailCode::VenvReady,
            python_version(&output),
        ),
        _ => InstanceHealthCheck::fail(
            InstanceHealthCheckKey::Venv,
            InstanceHealthDetailCode::VenvFailed,
            None,
        ),
    }
}

async fn check_dependencies(instance_id: &str) -> InstanceHealthCheck {
    let core_dir = get_instance_core_dir(instance_id);
    let requirements_path = core_dir.join("requirements.txt");
    if !requirements_path.is_file() {
        return InstanceHealthCheck::fail(
            InstanceHealthCheckKey::Dependencies,
            InstanceHealthDetailCode::RequirementsMissing,
            None,
        );
    }

    let executable = get_venv_python(&get_instance_venv_dir(instance_id));
    if !executable.is_file() {
        return InstanceHealthCheck::skipped(
            InstanceHealthCheckKey::Dependencies,
            InstanceHealthDetailCode::VenvRequired,
            None,
        );
    }

    let pip_check = match run_command(&executable, &["-m", "pip", "check"]).await {
        Ok(output) => output,
        Err(CommandFailure::TimedOut) => {
            return InstanceHealthCheck::warning(
                InstanceHealthCheckKey::Dependencies,
                InstanceHealthDetailCode::DependenciesTimeout,
                None,
            );
        }
        Err(CommandFailure::Unavailable) => {
            return InstanceHealthCheck::fail(
                InstanceHealthCheckKey::Dependencies,
                InstanceHealthDetailCode::DependenciesCheckFailed,
                None,
            );
        }
    };

    let pip_diagnostics = format!(
        "{}{}",
        String::from_utf8_lossy(&pip_check.stdout),
        String::from_utf8_lossy(&pip_check.stderr)
    );
    if pip_diagnostics.contains("No module named pip") {
        return InstanceHealthCheck::warning(
            InstanceHealthCheckKey::Dependencies,
            InstanceHealthDetailCode::PipUnavailable,
            None,
        );
    }
    let pip_issue_count = if pip_check.status.success() {
        0
    } else {
        pip_diagnostics
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count()
            .max(1)
    };

    let Some(requirements_path) = requirements_path.to_str() else {
        return InstanceHealthCheck::fail(
            InstanceHealthCheckKey::Dependencies,
            InstanceHealthDetailCode::DependenciesCheckFailed,
            None,
        );
    };
    let audit = match run_command(
        &executable,
        &["-c", REQUIREMENTS_AUDIT_SCRIPT, requirements_path],
    )
    .await
    {
        Ok(output) => output,
        Err(CommandFailure::TimedOut) => {
            return InstanceHealthCheck::warning(
                InstanceHealthCheckKey::Dependencies,
                InstanceHealthDetailCode::DependenciesTimeout,
                None,
            );
        }
        Err(CommandFailure::Unavailable) => {
            return InstanceHealthCheck::fail(
                InstanceHealthCheckKey::Dependencies,
                InstanceHealthDetailCode::DependenciesCheckFailed,
                None,
            );
        }
    };
    if !audit.status.success() {
        return InstanceHealthCheck::fail(
            InstanceHealthCheckKey::Dependencies,
            InstanceHealthDetailCode::DependenciesCheckFailed,
            None,
        );
    }

    let Some(missing_count) = missing_requirement_count(&audit.stdout) else {
        return InstanceHealthCheck::fail(
            InstanceHealthCheckKey::Dependencies,
            InstanceHealthDetailCode::DependenciesCheckFailed,
            None,
        );
    };
    dependency_check_result(pip_issue_count, missing_count)
}

fn dependency_check_result(pip_issue_count: usize, missing_count: usize) -> InstanceHealthCheck {
    let issue_count = pip_issue_count.max(missing_count);
    if issue_count > 0 {
        InstanceHealthCheck::fail(
            InstanceHealthCheckKey::Dependencies,
            InstanceHealthDetailCode::DependenciesIssues,
            Some(issue_count.to_string()),
        )
    } else {
        InstanceHealthCheck::pass(
            InstanceHealthCheckKey::Dependencies,
            InstanceHealthDetailCode::DependenciesReady,
            None,
        )
    }
}

fn missing_requirement_count(output: &[u8]) -> Option<usize> {
    serde_json::from_slice::<Vec<String>>(output)
        .ok()
        .map(|missing| missing.len())
}

fn check_port(
    host: &str,
    port: u16,
    is_running: bool,
    is_transitioning: bool,
) -> InstanceHealthCheck {
    if port == 0 {
        return InstanceHealthCheck::fail(
            InstanceHealthCheckKey::Port,
            InstanceHealthDetailCode::PortInvalid,
            None,
        );
    }
    if is_transitioning {
        return InstanceHealthCheck::warning(
            InstanceHealthCheckKey::Port,
            InstanceHealthDetailCode::PortCheckFailed,
            Some(port.to_string()),
        );
    }

    let Some(probe_ip) = local_probe_ip(host) else {
        return InstanceHealthCheck::warning(
            InstanceHealthCheckKey::Port,
            InstanceHealthDetailCode::PortCheckFailed,
            Some(port.to_string()),
        );
    };
    let address = SocketAddr::new(probe_ip, port);
    if is_running {
        return match TcpStream::connect_timeout(&address, CONNECT_TIMEOUT) {
            Ok(_) => InstanceHealthCheck::pass(
                InstanceHealthCheckKey::Port,
                InstanceHealthDetailCode::PortListening,
                Some(port.to_string()),
            ),
            Err(_) => InstanceHealthCheck::fail(
                InstanceHealthCheckKey::Port,
                InstanceHealthDetailCode::PortUnreachable,
                Some(port.to_string()),
            ),
        };
    }

    match TcpListener::bind(address) {
        Ok(_) => InstanceHealthCheck::pass(
            InstanceHealthCheckKey::Port,
            InstanceHealthDetailCode::PortAvailable,
            Some(port.to_string()),
        ),
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => InstanceHealthCheck::fail(
            InstanceHealthCheckKey::Port,
            InstanceHealthDetailCode::PortOccupied,
            Some(port.to_string()),
        ),
        Err(_) => InstanceHealthCheck::warning(
            InstanceHealthCheckKey::Port,
            InstanceHealthDetailCode::PortCheckFailed,
            Some(port.to_string()),
        ),
    }
}

fn check_disk_space(instance_id: &str) -> InstanceHealthCheck {
    let available = match available_disk_bytes(&get_instance_dir(instance_id)) {
        Ok(bytes) => bytes,
        Err(_) => {
            return InstanceHealthCheck::warning(
                InstanceHealthCheckKey::DiskSpace,
                InstanceHealthDetailCode::SpaceUnavailable,
                None,
            );
        }
    };

    check_disk_space_bytes(available)
}

fn check_disk_space_bytes(available: u64) -> InstanceHealthCheck {
    let key = InstanceHealthCheckKey::DiskSpace;
    let value = Some(available.to_string());
    if available < DISK_CRITICAL_BYTES {
        InstanceHealthCheck::fail(key, InstanceHealthDetailCode::SpaceCritical, value)
    } else if available < DISK_WARNING_BYTES {
        InstanceHealthCheck::warning(key, InstanceHealthDetailCode::SpaceLow, value)
    } else {
        InstanceHealthCheck::pass(key, InstanceHealthDetailCode::SpaceSufficient, value)
    }
}

fn check_configuration(instance_id: &str) -> InstanceHealthCheck {
    let data_dir = get_instance_core_dir(instance_id).join("data");
    let yaml_path = data_dir.join("config.yaml");
    let command_config_path = data_dir.join("cmd_config.json");
    let mut found_configuration = false;

    if yaml_path.exists() {
        found_configuration = true;
        let valid_yaml = std::fs::read_to_string(&yaml_path)
            .ok()
            .and_then(|content| {
                serde_yaml_ng::from_str::<serde_yaml_ng::Value>(
                    content.trim_start_matches('\u{feff}'),
                )
                .ok()
            })
            .is_some_and(|value| matches!(value, serde_yaml_ng::Value::Mapping(_)));
        if !valid_yaml {
            return InstanceHealthCheck::fail(
                InstanceHealthCheckKey::Configuration,
                InstanceHealthDetailCode::ConfigInvalid,
                None,
            );
        }
    }

    if command_config_path.exists() {
        found_configuration = true;
        let valid_json = std::fs::read_to_string(&command_config_path)
            .ok()
            .and_then(|content| {
                serde_json::from_str::<super::types::CmdConfig>(
                    content.trim_start_matches('\u{feff}'),
                )
                .ok()
            })
            .is_some();
        if !valid_json {
            return InstanceHealthCheck::fail(
                InstanceHealthCheckKey::Configuration,
                InstanceHealthDetailCode::ConfigInvalid,
                None,
            );
        }
    }

    if found_configuration {
        InstanceHealthCheck::pass(
            InstanceHealthCheckKey::Configuration,
            InstanceHealthDetailCode::ConfigValid,
            None,
        )
    } else {
        InstanceHealthCheck::warning(
            InstanceHealthCheckKey::Configuration,
            InstanceHealthDetailCode::ConfigMissing,
            None,
        )
    }
}

async fn check_webui(
    host: &str,
    port: u16,
    is_running: bool,
    dashboard_enabled: bool,
) -> InstanceHealthCheck {
    let key = InstanceHealthCheckKey::Webui;
    if !dashboard_enabled {
        return InstanceHealthCheck::skipped(key, InstanceHealthDetailCode::WebuiDisabled, None);
    }
    if !is_running {
        return InstanceHealthCheck::skipped(key, InstanceHealthDetailCode::WebuiNotRunning, None);
    }
    let Some(host) = local_probe_ip(host) else {
        return InstanceHealthCheck::warning(
            key,
            InstanceHealthDetailCode::WebuiHostUnsupported,
            None,
        );
    };

    let host = match host {
        IpAddr::V4(address) => address.to_string(),
        IpAddr::V6(address) => format!("[{address}]"),
    };
    let url = format!("http://{host}:{port}/");
    let client = match reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(4))
        .build()
    {
        Ok(client) => client,
        Err(_) => {
            return InstanceHealthCheck::fail(
                key,
                InstanceHealthDetailCode::WebuiUnreachable,
                None,
            );
        }
    };

    match client.get(url).send().await {
        Ok(response) if !response.status().is_server_error() => InstanceHealthCheck::pass(
            key,
            InstanceHealthDetailCode::WebuiReachable,
            Some(response.status().as_u16().to_string()),
        ),
        _ => InstanceHealthCheck::fail(key, InstanceHealthDetailCode::WebuiUnreachable, None),
    }
}

fn local_probe_ip(host: &str) -> Option<IpAddr> {
    let host = host.trim().trim_start_matches('[').trim_end_matches(']');
    if host.eq_ignore_ascii_case("localhost") || host.is_empty() {
        return Some(IpAddr::V4(Ipv4Addr::LOCALHOST));
    }

    let ip = host.parse::<IpAddr>().ok()?;
    if ip.is_loopback() {
        Some(ip)
    } else if ip.is_unspecified() {
        Some(match ip {
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(_) => IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
        })
    } else {
        None
    }
}

enum CommandFailure {
    Unavailable,
    TimedOut,
}

async fn run_command(
    executable: &Path,
    args: &[&str],
) -> std::result::Result<std::process::Output, CommandFailure> {
    let mut command = Command::new(executable);
    command.args(args).kill_on_drop(true);

    match tokio::time::timeout(COMMAND_TIMEOUT, command.output()).await {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(_)) => Err(CommandFailure::Unavailable),
        Err(_) => Err(CommandFailure::TimedOut),
    }
}

fn python_version(output: &std::process::Output) -> Option<String> {
    [output.stdout.as_slice(), output.stderr.as_slice()]
        .into_iter()
        .flat_map(|bytes| {
            String::from_utf8_lossy(bytes)
                .lines()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .find_map(|line| line.strip_prefix("Python ").map(ToOwned::to_owned))
}

fn available_disk_bytes(path: &Path) -> std::io::Result<u64> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt as _;
        use windows::core::PCWSTR;
        use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;

        let mut path: Vec<u16> = path.as_os_str().encode_wide().collect();
        path.push(0);
        let mut available = 0u64;
        unsafe { GetDiskFreeSpaceExW(PCWSTR(path.as_ptr()), Some(&mut available), None, None) }
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        Ok(available)
    }

    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt as _;

        let path = CString::new(path.as_os_str().as_bytes())
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;
        let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        let result = unsafe { libc::statvfs(path.as_ptr(), stats.as_mut_ptr()) };
        if result != 0 {
            return Err(std::io::Error::last_os_error());
        }
        let stats = unsafe { stats.assume_init() };
        Ok(stats.f_bavail.saturating_mul(stats.f_frsize))
    }

    #[cfg(not(any(target_os = "windows", unix)))]
    {
        let _ = path;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Disk space checks are unsupported on this platform",
        ))
    }
}

fn overall_grade(checks: &[InstanceHealthCheck]) -> InstanceHealthGrade {
    if checks
        .iter()
        .any(|check| check.status == InstanceHealthCheckStatus::Fail)
    {
        InstanceHealthGrade::Critical
    } else if checks
        .iter()
        .any(|check| check.status == InstanceHealthCheckStatus::Warning)
    {
        InstanceHealthGrade::Warning
    } else {
        InstanceHealthGrade::Healthy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_grade_ignores_skipped_checks() {
        let checks = [InstanceHealthCheck::skipped(
            InstanceHealthCheckKey::Webui,
            InstanceHealthDetailCode::WebuiNotRunning,
            None,
        )];
        assert_eq!(overall_grade(&checks), InstanceHealthGrade::Healthy);
    }

    #[test]
    fn health_grade_prioritizes_failures_over_warnings() {
        let checks = [
            InstanceHealthCheck::warning(
                InstanceHealthCheckKey::DiskSpace,
                InstanceHealthDetailCode::SpaceLow,
                None,
            ),
            InstanceHealthCheck::fail(
                InstanceHealthCheckKey::Python,
                InstanceHealthDetailCode::RuntimeMissing,
                None,
            ),
        ];
        assert_eq!(overall_grade(&checks), InstanceHealthGrade::Critical);
    }

    #[test]
    fn disk_space_grade_uses_warning_and_critical_thresholds() {
        assert_eq!(
            check_disk_space_bytes(DISK_CRITICAL_BYTES - 1).status,
            InstanceHealthCheckStatus::Fail
        );
        assert_eq!(
            check_disk_space_bytes(DISK_CRITICAL_BYTES).status,
            InstanceHealthCheckStatus::Warning
        );
        assert_eq!(
            check_disk_space_bytes(DISK_WARNING_BYTES).status,
            InstanceHealthCheckStatus::Pass
        );
    }

    #[test]
    fn dependency_check_fails_when_requirements_are_missing() {
        assert_eq!(
            missing_requirement_count(br#"["astrbot","aiohttp"]"#),
            Some(2)
        );
        let check = dependency_check_result(0, 2);

        assert_eq!(check.status, InstanceHealthCheckStatus::Fail);
        assert_eq!(
            check.detail_code,
            InstanceHealthDetailCode::DependenciesIssues
        );
        assert_eq!(check.value.as_deref(), Some("2"));
    }

    #[test]
    fn dependency_check_passes_when_no_issues_are_found() {
        assert_eq!(missing_requirement_count(b"[]"), Some(0));
        assert_eq!(
            dependency_check_result(0, 0).status,
            InstanceHealthCheckStatus::Pass
        );
        assert_eq!(missing_requirement_count(b"not json"), None);
    }

    #[test]
    fn local_probe_does_not_resolve_or_connect_to_remote_hosts() {
        assert_eq!(
            local_probe_ip("127.0.0.1"),
            Some(IpAddr::V4(Ipv4Addr::LOCALHOST))
        );
        assert_eq!(
            local_probe_ip("0.0.0.0"),
            Some(IpAddr::V4(Ipv4Addr::LOCALHOST))
        );
        assert_eq!(local_probe_ip("example.com"), None);
        assert_eq!(local_probe_ip("192.0.2.1"), None);
    }
}
