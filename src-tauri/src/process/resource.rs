//! Resource usage monitoring: CPU, memory, and disk sampling with history.
//!
//! Samples are collected by the monitor loop for each live instance and kept
//! in a bounded ring buffer so the frontend can draw trend charts without
//! unbounded memory growth.

use std::collections::{HashSet, VecDeque};
use std::path::Path;
use std::time::{Instant, SystemTime};

use once_cell::sync::OnceCell;

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
    /// CPU usage of the instance process tree, in percent of the whole
    /// machine (100 means every logical core is busy).
    pub cpu_percent: Option<f32>,
    /// Private working set of the instance process tree in bytes, the value
    /// task managers report as a process's memory. `None` when unavailable.
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

/// Sample CPU and memory for the process tree rooted at `pid`.
///
/// CPU time and memory are summed over the root process and its descendants so
/// helper processes spawned by the instance are included. CPU is reported
/// relative to the whole machine (100% = all logical cores busy) and memory is
/// the private working set, matching what task managers show. The first CPU
/// probe has no previous baseline and therefore returns `None`.
pub(crate) fn sample_process_resources(
    pid: u32,
    meter: &mut CpuMeter,
) -> (Option<f32>, Option<u64>) {
    let (cpu_total_ns, memory_bytes) = read_process_tree_usage(pid);
    let cpu_percent = meter
        .observe(cpu_total_ns, Instant::now())
        .map(|single_core_percent| scale_cpu_percent(single_core_percent, logical_core_count()));
    (cpu_percent, memory_bytes)
}

/// Turn a single-core-relative percentage into a whole-machine percentage.
fn scale_cpu_percent(single_core_percent: f32, cores: f32) -> f32 {
    if cores > 1.0 {
        single_core_percent / cores
    } else {
        single_core_percent
    }
}

fn logical_core_count() -> f32 {
    static CORES: OnceCell<f32> = OnceCell::new();
    *CORES.get_or_init(|| {
        std::thread::available_parallelism()
            .map(|cores| cores.get() as f32)
            .unwrap_or(1.0)
    })
}

#[cfg(target_os = "windows")]
fn read_process_tree_usage(root_pid: u32) -> (Option<u64>, Option<u64>) {
    let mut cpu_total = 0u64;
    let mut memory_total = 0u64;
    let mut cpu_seen = false;
    let mut memory_seen = false;

    for pid in process_tree_pids(root_pid) {
        let (cpu_ns, memory_bytes) = read_single_process_usage(pid);
        if let Some(value) = cpu_ns {
            cpu_total = cpu_total.saturating_add(value);
            cpu_seen = true;
        }
        if let Some(value) = memory_bytes {
            memory_total = memory_total.saturating_add(value);
            memory_seen = true;
        }
    }

    (
        cpu_seen.then_some(cpu_total),
        memory_seen.then_some(memory_total),
    )
}

/// Collect the root process and every descendant PID.
#[cfg(target_os = "windows")]
fn process_tree_pids(root_pid: u32) -> Vec<u32> {
    use std::collections::HashMap;

    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return vec![root_pid];
    };

    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };

    if unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok() {
        loop {
            children
                .entry(entry.th32ParentProcessID)
                .or_default()
                .push(entry.th32ProcessID);
            if unsafe { Process32NextW(snapshot, &mut entry) }.is_err() {
                break;
            }
        }
    }
    let _ = unsafe { CloseHandle(snapshot) };

    let mut pids = vec![root_pid];
    let mut index = 0;
    // Cap the walk so a corrupted parent chain cannot loop forever.
    while index < pids.len() && pids.len() < 4096 {
        let current = pids[index];
        if let Some(kids) = children.get(&current) {
            pids.extend(kids.iter().copied());
        }
        index += 1;
    }
    pids
}

#[cfg(target_os = "windows")]
fn read_single_process_usage(pid: u32) -> (Option<u64>, Option<u64>) {
    use windows::Win32::Foundation::{CloseHandle, FILETIME};
    use windows::Win32::System::Threading::GetProcessTimes;

    let Some(handle) = open_process_for_metrics(pid) else {
        return (None, None);
    };

    let mut creation = FILETIME::default();
    let mut exit_time = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    let cpu_ns = if unsafe {
        GetProcessTimes(
            handle,
            &mut creation,
            &mut exit_time,
            &mut kernel,
            &mut user,
        )
    }
    .is_ok()
    {
        filetime_to_u64(&kernel)
            .saturating_add(filetime_to_u64(&user))
            .checked_mul(100)
    } else {
        None
    };

    let memory_bytes = query_private_working_set(handle);
    let _ = unsafe { CloseHandle(handle) };
    (cpu_ns, memory_bytes)
}

#[cfg(target_os = "windows")]
fn open_process_for_metrics(pid: u32) -> Option<windows::Win32::Foundation::HANDLE> {
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
    };

    let limited = unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ,
            false,
            pid,
        )
    };
    if let Ok(handle) = limited {
        return Some(handle);
    }
    unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok() }
}

/// Private working set, the number Windows Task Manager shows as a process's
/// memory. Falls back to the plain working set on builds that reject the
/// larger counter struct.
#[cfg(target_os = "windows")]
fn query_private_working_set(handle: windows::Win32::Foundation::HANDLE) -> Option<u64> {
    use windows::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX,
        PROCESS_MEMORY_COUNTERS_EX2,
    };

    let mut counters = PROCESS_MEMORY_COUNTERS_EX2 {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32,
        ..Default::default()
    };
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32;
    let success = unsafe {
        K32GetProcessMemoryInfo(
            handle,
            (&mut counters as *mut PROCESS_MEMORY_COUNTERS_EX2).cast::<PROCESS_MEMORY_COUNTERS>(),
            size,
        )
    }
    .as_bool();
    if success {
        return Some(counters.PrivateWorkingSetSize as u64);
    }

    let mut legacy = PROCESS_MEMORY_COUNTERS_EX {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
        ..Default::default()
    };
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
    let success = unsafe {
        K32GetProcessMemoryInfo(
            handle,
            (&mut legacy as *mut PROCESS_MEMORY_COUNTERS_EX).cast::<PROCESS_MEMORY_COUNTERS>(),
            size,
        )
    }
    .as_bool();
    success.then_some(legacy.WorkingSetSize as u64)
}

#[cfg(target_os = "windows")]
fn filetime_to_u64(filetime: &windows::Win32::Foundation::FILETIME) -> u64 {
    (filetime.dwLowDateTime as u64) | ((filetime.dwHighDateTime as u64) << 32)
}

#[cfg(target_os = "linux")]
fn read_process_tree_usage(root_pid: u32) -> (Option<u64>, Option<u64>) {
    use std::collections::HashMap;

    let mut parents: HashMap<u32, u32> = HashMap::new();
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<u32>().ok())
            else {
                continue;
            };
            let Some(parent) = linux_parent_pid(pid) else {
                continue;
            };
            parents.insert(pid, parent);
        }
    }

    let mut pids = vec![root_pid];
    let mut index = 0;
    // Cap the walk so a corrupted parent chain cannot loop forever.
    while index < pids.len() && pids.len() < 4096 {
        let current = pids[index];
        for (pid, parent) in &parents {
            if *parent == current {
                pids.push(*pid);
            }
        }
        index += 1;
    }

    let mut cpu_total = 0u64;
    let mut memory_total = 0u64;
    let mut cpu_seen = false;
    let mut memory_seen = false;
    for pid in pids {
        if let Some(value) = linux_process_cpu_ns(pid) {
            cpu_total = cpu_total.saturating_add(value);
            cpu_seen = true;
        }
        if let Some(value) = linux_process_memory_bytes(pid) {
            memory_total = memory_total.saturating_add(value);
            memory_seen = true;
        }
    }

    (
        cpu_seen.then_some(cpu_total),
        memory_seen.then_some(memory_total),
    )
}

#[cfg(target_os = "linux")]
fn linux_parent_pid(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let close_paren = stat.rfind(')')?;
    stat[close_paren + 2..]
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

#[cfg(target_os = "linux")]
fn linux_process_cpu_ns(pid: u32) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
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
fn linux_process_memory_bytes(pid: u32) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in status.lines() {
        if let Some(raw) = line.strip_prefix("VmRSS:") {
            let kilobytes = raw.split_whitespace().next()?.parse::<u64>().ok()?;
            return kilobytes.checked_mul(1024);
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn read_process_tree_usage(pid: u32) -> (Option<u64>, Option<u64>) {
    use libproc::libproc::pid_rusage::{pidrusage, RUsageInfoV2};

    let Ok(usage) = pidrusage::<RUsageInfoV2>(pid as i32) else {
        return (None, None);
    };
    let cpu_ns = Some(usage.ri_user_time.saturating_add(usage.ri_system_time));
    (cpu_ns, Some(usage.ri_resident_size))
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn read_process_tree_usage(_pid: u32) -> (Option<u64>, Option<u64>) {
    (None, None)
}

/// Consecutive samples a metric must breach before an alert fires, and must
/// stay back within the threshold before it clears. Filters one-off spikes and
/// stops alerts from flapping around the threshold.
pub(crate) const ALERT_CONFIRM_SAMPLES: u8 = 2;

/// Tracks breached metrics across samples so alert events only fire on real
/// transitions instead of every monitor tick.
#[derive(Debug, Default, Clone)]
pub(crate) struct AlertTracker {
    breach_streak: std::collections::HashMap<String, u8>,
    clear_streak: std::collections::HashMap<String, u8>,
    active: HashSet<String>,
}

impl AlertTracker {
    pub(crate) fn clear(&mut self) {
        self.breach_streak.clear();
        self.clear_streak.clear();
        self.active.clear();
    }

    /// Record one sample for `metric`; returns true only on the sample where
    /// the alert becomes active.
    pub(crate) fn observe(&mut self, metric: &str, breached: bool) -> bool {
        if !breached {
            self.breach_streak.remove(metric);
            let streak = self.clear_streak.entry(metric.to_string()).or_insert(0);
            *streak = streak.saturating_add(1);
            if *streak >= ALERT_CONFIRM_SAMPLES {
                self.active.remove(metric);
            }
            return false;
        }

        self.clear_streak.remove(metric);
        let streak = self.breach_streak.entry(metric.to_string()).or_insert(0);
        *streak = streak.saturating_add(1);
        *streak >= ALERT_CONFIRM_SAMPLES && self.active.insert(metric.to_string())
    }

    #[cfg(test)]
    pub(crate) fn is_active(&self, metric: &str) -> bool {
        self.active.contains(metric)
    }
}

/// Resource alert configuration: a per-instance switch plus one upper bound
/// per metric. When enabled and a sample exceeds a bound, an alert is
/// emitted after it stays above for two consecutive samples.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceAlertThreshold {
    pub enabled: bool,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub disk_bytes: u64,
}

impl Default for ResourceAlertThreshold {
    fn default() -> Self {
        Self {
            enabled: true,
            cpu_percent: 90.0,
            memory_bytes: 4 * 1024 * 1024 * 1024,
            disk_bytes: 20 * 1024 * 1024 * 1024,
        }
    }
}

impl ResourceAlertThreshold {
    /// Return every metric that currently exceeds its configured threshold.
    pub fn breached_metrics(&self, sample: &ResourceSample) -> Vec<&'static str> {
        if !self.enabled {
            return Vec::new();
        }

        let mut metrics = Vec::new();

        if let Some(value) = sample.cpu_percent {
            if value > self.cpu_percent {
                metrics.push("cpu");
            }
        }
        if let Some(value) = sample.memory_bytes {
            if value > self.memory_bytes {
                metrics.push("memory");
            }
        }
        if let Some(value) = sample.disk_bytes {
            if value > self.disk_bytes {
                metrics.push("disk");
            }
        }

        metrics
    }

    /// Read a metric's configured threshold as a finite number.
    pub fn threshold_for(&self, metric: &str) -> Option<f64> {
        match metric {
            "cpu" => Some(f64::from(self.cpu_percent)),
            "memory" => Some(self.memory_bytes as f64),
            "disk" => Some(self.disk_bytes as f64),
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
    fn alerts_require_two_consecutive_breaches() {
        let mut tracker = AlertTracker::default();
        assert!(!tracker.observe("cpu", true));
        assert!(!tracker.is_active("cpu"));
        assert!(tracker.observe("cpu", true));
        assert!(tracker.is_active("cpu"));
        assert!(!tracker.observe("cpu", true));
    }

    #[test]
    fn alerts_clear_after_two_normal_samples() {
        let mut tracker = AlertTracker::default();
        tracker.observe("cpu", true);
        tracker.observe("cpu", true);
        assert!(!tracker.observe("cpu", false));
        assert!(tracker.is_active("cpu"));
        assert!(!tracker.observe("cpu", false));
        assert!(!tracker.is_active("cpu"));
    }

    #[test]
    fn cpu_percent_is_scaled_to_the_whole_machine() {
        assert_eq!(scale_cpu_percent(200.0, 8.0), 25.0);
        assert_eq!(scale_cpu_percent(80.0, 1.0), 80.0);
    }

    #[test]
    fn threshold_returns_all_breached_metrics() {
        let threshold = ResourceAlertThreshold {
            enabled: true,
            cpu_percent: 80.0,
            memory_bytes: 1024,
            disk_bytes: 4096,
        };
        let over = sample(0, Some(90.0), Some(2048), Some(8192));
        assert_eq!(
            threshold.breached_metrics(&over),
            vec!["cpu", "memory", "disk"]
        );

        let within = sample(0, Some(50.0), Some(512), Some(2048));
        assert!(threshold.breached_metrics(&within).is_empty());
    }

    #[test]
    fn disabled_alerts_never_breach_and_defaults_are_sane() {
        let disabled = ResourceAlertThreshold {
            enabled: false,
            ..ResourceAlertThreshold::default()
        };
        let over = sample(0, Some(99.0), Some(u64::MAX), Some(u64::MAX));
        assert!(disabled.breached_metrics(&over).is_empty());

        let defaults = ResourceAlertThreshold::default();
        assert!(defaults.enabled);
        assert_eq!(defaults.cpu_percent, 90.0);
        assert_eq!(defaults.memory_bytes, 4 * 1024 * 1024 * 1024);
        assert_eq!(defaults.disk_bytes, 20 * 1024 * 1024 * 1024);
    }
}
