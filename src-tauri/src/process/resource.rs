//! Resource usage monitoring: CPU, memory, and disk sampling with history.
//!
//! Samples are collected by the monitor loop for each live instance and kept
//! in a bounded ring buffer so the frontend can draw trend charts without
//! unbounded memory growth.

use std::collections::VecDeque;
use std::path::Path;
use std::time::{Instant, SystemTime};

use serde::{Deserialize, Serialize};

/// Maximum number of samples retained per instance.
///
/// With a 5-second monitor interval this covers one hour of history.
pub const MAX_HISTORY_SAMPLES: usize = 720;

/// How many monitor ticks between disk usage samples.
///
/// Disk sampling walks the instance directory tree and can be slow for large
/// data directories, so we run it less frequently than CPU/memory probes.
pub(crate) const DISK_SAMPLE_INTERVAL_TICKS: u32 = 12; // ~60s at 5s interval

/// Return whether the given monitor tick should refresh disk usage.
pub(crate) fn is_disk_sample_due(tick: u32) -> bool {
    tick == 1 || tick.is_multiple_of(DISK_SAMPLE_INTERVAL_TICKS)
}

/// A single resource usage snapshot for an instance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceSample {
    /// Unix timestamp in seconds (wall-clock).
    pub timestamp: u64,
    /// CPU usage percentage of the process. Values above 100 are possible
    /// when a process uses more than one logical core.
    pub cpu_percent: Option<f32>,
    /// Resident memory in bytes. `None` when unavailable.
    pub memory_bytes: Option<u64>,
    /// Instance directory disk usage in bytes. `None` when unavailable.
    pub disk_bytes: Option<u64>,
}

/// Tracks cumulative process CPU time so consecutive probes can produce a
/// usage percentage instead of a lifetime average.
#[derive(Debug, Clone, Default)]
pub(crate) struct CpuMeter {
    last_total_ns: Option<u64>,
    last_probe_at: Option<Instant>,
}

impl CpuMeter {
    fn observe(&mut self, total_cpu_ns: Option<u64>, now: Instant) -> Option<f32> {
        let usage = match (total_cpu_ns, self.last_total_ns, self.last_probe_at) {
            (Some(current), Some(previous), Some(previous_at)) => {
                let elapsed = now.saturating_duration_since(previous_at).as_nanos();
                let consumed = current.saturating_sub(previous);
                if elapsed == 0 {
                    None
                } else {
                    Some((consumed as f64 / elapsed as f64 * 100.0) as f32)
                }
            }
            _ => None,
        };

        self.last_total_ns = total_cpu_ns;
        self.last_probe_at = total_cpu_ns.map(|_| now);
        usage
    }
}

/// Bounded history of resource samples.
#[derive(Debug, Clone, Default)]
pub struct ResourceHistory {
    samples: VecDeque<ResourceSample>,
    /// Tick counter used to throttle disk usage sampling (it costs I/O).
    pub(crate) disk_sample_tick: u32,
    /// Last measured disk usage, carried into samples between throttled probes.
    pub(crate) last_disk_bytes: Option<u64>,
    /// Cumulative process CPU meter shared across monitor ticks.
    pub(crate) cpu_meter: CpuMeter,
}

impl ResourceHistory {
    pub fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(MAX_HISTORY_SAMPLES),
            disk_sample_tick: 0,
            last_disk_bytes: None,
            cpu_meter: CpuMeter::default(),
        }
    }

    /// Push a new sample, dropping the oldest if the buffer is full.
    pub fn push(&mut self, sample: ResourceSample) {
        if let Some(disk_bytes) = sample.disk_bytes {
            self.last_disk_bytes = Some(disk_bytes);
        }
        if self.samples.len() >= MAX_HISTORY_SAMPLES {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    /// Return all samples ordered from oldest to newest.
    pub fn samples(&self) -> &VecDeque<ResourceSample> {
        &self.samples
    }

    /// Return the most recent sample, if any.
    pub fn latest(&self) -> Option<&ResourceSample> {
        self.samples.back()
    }
}

/// Convert the current wall-clock time to Unix seconds.
pub fn unix_secs_now() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

/// Recursively compute the total size of a directory in bytes.
///
/// Returns `None` if the path does not exist or cannot be traversed.
/// Silently skips entries that cannot be read and never follows symlinks.
pub fn compute_disk_usage(path: &Path) -> Option<u64> {
    if !path.is_dir() {
        return None;
    }

    let mut total = 0u64;
    for entry in walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if let Ok(metadata) = entry.metadata() {
            total = total.saturating_add(metadata.len());
        }
    }
    Some(total)
}

/// Sample CPU and memory for a process identified by PID.
///
/// The first CPU probe has no previous baseline and therefore returns `None`.
/// Subsequent probes report the CPU used between monitor ticks.
pub(crate) fn sample_process_resources(
    pid: u32,
    meter: &mut CpuMeter,
) -> (Option<f32>, Option<u64>) {
    let cpu_total_ns = read_process_cpu_time_ns(pid);
    let cpu_percent = meter.observe(cpu_total_ns, Instant::now());
    let memory_bytes = read_process_memory_bytes(pid);
    (cpu_percent, memory_bytes)
}

#[cfg(target_os = "windows")]
fn read_process_cpu_time_ns(pid: u32) -> Option<u64> {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut creation = FILETIME::default();
    let mut exit_time = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();

    let success = unsafe {
        GetProcessTimes(
            handle,
            &mut creation,
            &mut exit_time,
            &mut kernel,
            &mut user,
        )
    }
    .is_ok();
    let _ = unsafe { windows::Win32::Foundation::CloseHandle(handle) };

    if !success {
        return None;
    }

    filetime_to_u64(&kernel)
        .saturating_add(filetime_to_u64(&user))
        .checked_mul(100)
}

#[cfg(target_os = "windows")]
fn read_process_memory_bytes(pid: u32) -> Option<u64> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
    };

    let handle =
        unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) }.ok()?;
    let mut counters = PROCESS_MEMORY_COUNTERS::default();
    let success = unsafe {
        K32GetProcessMemoryInfo(
            handle,
            &mut counters,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
    };
    let _ = unsafe { CloseHandle(handle) };

    (success.0 != 0).then_some(counters.WorkingSetSize as u64)
}

#[cfg(target_os = "windows")]
fn filetime_to_u64(filetime: &windows::Win32::Foundation::FILETIME) -> u64 {
    (filetime.dwLowDateTime as u64) | ((filetime.dwHighDateTime as u64) << 32)
}

#[cfg(target_os = "linux")]
fn read_process_cpu_time_ns(pid: u32) -> Option<u64> {
    use std::fs;

    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let close_paren = stat.rfind(')')?;
    let fields: Vec<&str> = stat[close_paren + 2..].split_whitespace().collect();
    let user_ticks = fields.get(11)?.parse::<u64>().ok()?;
    let system_ticks = fields.get(12)?.parse::<u64>().ok()?;
    let clock_ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if clock_ticks <= 0 {
        return None;
    }

    user_ticks
        .saturating_add(system_ticks)
        .checked_mul(1_000_000_000)?
        .checked_div(clock_ticks as u64)
}

#[cfg(target_os = "linux")]
fn read_process_memory_bytes(pid: u32) -> Option<u64> {
    use std::fs;

    let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in status.lines() {
        if let Some(raw) = line.strip_prefix("VmRSS:") {
            let kilobytes = raw.split_whitespace().next()?.parse::<u64>().ok()?;
            return kilobytes.checked_mul(1024);
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn read_process_cpu_time_ns(pid: u32) -> Option<u64> {
    use libproc::libproc::pid_rusage::{pidrusage, RUsageInfoV2};

    let usage = pidrusage::<RUsageInfoV2>(pid as i32).ok()?;
    Some(usage.ri_user_time.saturating_add(usage.ri_system_time))
}

#[cfg(target_os = "macos")]
fn read_process_memory_bytes(pid: u32) -> Option<u64> {
    use libproc::libproc::pid_rusage::{pidrusage, RUsageInfoV2};

    let usage = pidrusage::<RUsageInfoV2>(pid as i32).ok()?;
    Some(usage.ri_resident_size)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn read_process_cpu_time_ns(_pid: u32) -> Option<u64> {
    None
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn read_process_memory_bytes(_pid: u32) -> Option<u64> {
    None
}

/// Resource alert threshold configuration.
///
/// Each field is an optional upper bound. When a sample exceeds a configured
/// threshold, an alert event is emitted to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ResourceAlertThreshold {
    pub cpu_percent: Option<f32>,
    pub memory_bytes: Option<u64>,
    pub disk_bytes: Option<u64>,
}

impl ResourceAlertThreshold {
    pub fn is_empty(&self) -> bool {
        self.cpu_percent.is_none() && self.memory_bytes.is_none() && self.disk_bytes.is_none()
    }

    /// Return every metric that currently exceeds its configured threshold.
    pub fn breached_metrics(&self, sample: &ResourceSample) -> Vec<&'static str> {
        let mut metrics = Vec::new();

        if let (Some(threshold), Some(value)) = (self.cpu_percent, sample.cpu_percent) {
            if value > threshold {
                metrics.push("cpu");
            }
        }
        if let (Some(threshold), Some(value)) = (self.memory_bytes, sample.memory_bytes) {
            if value > threshold {
                metrics.push("memory");
            }
        }
        if let (Some(threshold), Some(value)) = (self.disk_bytes, sample.disk_bytes) {
            if value > threshold {
                metrics.push("disk");
            }
        }

        metrics
    }

    /// Read a metric's configured threshold as a finite number.
    pub fn threshold_for(&self, metric: &str) -> Option<f64> {
        match metric {
            "cpu" => self.cpu_percent.map(f64::from),
            "memory" => self.memory_bytes.map(|value| value as f64),
            "disk" => self.disk_bytes.map(|value| value as f64),
            _ => None,
        }
    }
}

/// Read a metric from a sample as a finite number.
pub fn sample_metric(sample: &ResourceSample, metric: &str) -> Option<f64> {
    match metric {
        "cpu" => sample.cpu_percent.map(f64::from),
        "memory" => sample.memory_bytes.map(|value| value as f64),
        "disk" => sample.disk_bytes.map(|value| value as f64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn sample(
        timestamp: u64,
        cpu_percent: Option<f32>,
        memory_bytes: Option<u64>,
        disk_bytes: Option<u64>,
    ) -> ResourceSample {
        ResourceSample {
            timestamp,
            cpu_percent,
            memory_bytes,
            disk_bytes,
        }
    }

    #[test]
    fn history_push_drops_oldest_sample_at_capacity() {
        let mut history = ResourceHistory::new();
        for index in 0..MAX_HISTORY_SAMPLES + 50 {
            history.push(sample(index as u64, None, None, None));
        }

        assert_eq!(history.samples().len(), MAX_HISTORY_SAMPLES);
        assert_eq!(
            history.samples().front().map(|item| item.timestamp),
            Some(50)
        );
        assert_eq!(
            history.samples().back().map(|item| item.timestamp),
            Some((MAX_HISTORY_SAMPLES + 49) as u64)
        );
    }

    #[test]
    fn history_carries_last_disk_usage() {
        let mut history = ResourceHistory::new();
        history.push(sample(1, None, None, Some(4096)));
        assert_eq!(history.last_disk_bytes, Some(4096));
        history.push(sample(2, None, None, Some(8192)));
        assert_eq!(history.last_disk_bytes, Some(8192));
    }

    #[test]
    fn disk_sampling_starts_immediately_then_throttles() {
        assert!(is_disk_sample_due(1));
        for tick in 2..DISK_SAMPLE_INTERVAL_TICKS {
            assert!(!is_disk_sample_due(tick));
        }
        assert!(is_disk_sample_due(DISK_SAMPLE_INTERVAL_TICKS));
    }

    #[test]
    fn cpu_meter_reports_delta_usage() {
        let mut meter = CpuMeter::default();
        let start = Instant::now();

        assert_eq!(meter.observe(Some(0), start), None);
        assert_eq!(
            meter.observe(Some(500_000_000), start + Duration::from_secs(1)),
            Some(50.0)
        );
        assert_eq!(
            meter.observe(Some(1_000_000_000), start + Duration::from_secs(2)),
            Some(50.0)
        );
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    #[test]
    fn current_process_probe_returns_memory_and_cpu_delta() {
        let mut meter = CpuMeter::default();
        let pid = std::process::id();

        let (first_cpu, memory_bytes) = sample_process_resources(pid, &mut meter);
        assert!(first_cpu.is_none());
        assert!(memory_bytes.is_some_and(|value| value > 0));

        std::thread::sleep(std::time::Duration::from_millis(20));
        let (second_cpu, _) = sample_process_resources(pid, &mut meter);
        assert!(second_cpu.is_some());
    }

    #[test]
    fn threshold_returns_all_breached_metrics() {
        let threshold = ResourceAlertThreshold {
            cpu_percent: Some(80.0),
            memory_bytes: Some(1024),
            disk_bytes: Some(4096),
        };
        let over = sample(0, Some(90.0), Some(2048), Some(8192));
        assert_eq!(
            threshold.breached_metrics(&over),
            vec!["cpu", "memory", "disk"]
        );

        let within = sample(0, Some(50.0), Some(512), Some(2048));
        assert!(threshold.breached_metrics(&within).is_empty());
        assert!(ResourceAlertThreshold::default()
            .breached_metrics(&over)
            .is_empty());
    }
}
