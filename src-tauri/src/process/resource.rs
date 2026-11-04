//! Resource usage monitoring: CPU, memory, and disk sampling with history.
//!
//! Samples are collected by the monitor loop for each live instance and kept
//! in a bounded ring buffer so the frontend can draw trend charts without
//! unbounded memory growth.

use std::collections::{HashMap, HashSet, VecDeque};
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
    /// Private memory of the instance process tree in bytes: the private
    /// working set on Windows (the value task managers show) and private
    /// resident pages on Linux. `None` when unavailable.
    pub memory_bytes: Option<u64>,
    /// Instance directory disk usage in bytes. `None` when unavailable.
    pub disk_bytes: Option<u64>,
}

/// Identifies one process across probes. The start time keeps a recycled PID
/// from being mistaken for the process that used it before.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ProcessKey {
    pid: u32,
    started_at: u64,
}

/// Cumulative CPU time of one process in nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessCpuTime {
    key: ProcessKey,
    cpu_ns: u64,
}

/// Tracks cumulative CPU time per process so consecutive probes can produce a
/// usage percentage instead of a lifetime average. Deltas are taken per
/// process, so a child exiting between probes does not make the tree total
/// drop and hide the CPU the remaining processes used.
#[derive(Debug, Clone, Default)]
pub(crate) struct CpuMeter {
    last_cpu_ns: HashMap<ProcessKey, u64>,
    last_probe_at: Option<Instant>,
}

impl CpuMeter {
    /// `processes` is empty when no CPU time could be read for the tree.
    fn observe(&mut self, processes: &[ProcessCpuTime], now: Instant) -> Option<f32> {
        let usage = match self.last_probe_at {
            Some(previous_at) if !processes.is_empty() => {
                let elapsed = now.saturating_duration_since(previous_at).as_nanos();
                // Processes not seen at the previous probe started since
                // then, so their whole CPU time belongs to this interval.
                let consumed = processes
                    .iter()
                    .map(|process| match self.last_cpu_ns.get(&process.key) {
                        Some(previous) => process.cpu_ns.saturating_sub(*previous),
                        None => process.cpu_ns,
                    })
                    .fold(0u64, u64::saturating_add);
                if elapsed == 0 {
                    None
                } else {
                    Some((consumed as f64 / elapsed as f64 * 100.0) as f32)
                }
            }
            _ => None,
        };

        self.last_cpu_ns = processes
            .iter()
            .map(|process| (process.key, process.cpu_ns))
            .collect();
        self.last_probe_at = (!processes.is_empty()).then_some(now);
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
/// private memory (see [`ResourceSample::memory_bytes`]). The first CPU
/// probe has no previous baseline and therefore returns `None`.
pub(crate) fn sample_process_resources(
    pid: u32,
    meter: &mut CpuMeter,
) -> (Option<f32>, Option<u64>) {
    let (cpu_times, memory_bytes) = read_process_tree_usage(pid);
    let cpu_percent = meter
        .observe(&cpu_times, Instant::now())
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
fn read_process_tree_usage(root_pid: u32) -> (Vec<ProcessCpuTime>, Option<u64>) {
    let mut cpu_times = Vec::new();
    let mut memory_total = 0u64;
    let mut memory_seen = false;

    for pid in process_tree_pids(root_pid) {
        let (cpu_time, memory_bytes) = read_single_process_usage(pid);
        cpu_times.extend(cpu_time);
        if let Some(value) = memory_bytes {
            memory_total = memory_total.saturating_add(value);
            memory_seen = true;
        }
    }

    (cpu_times, memory_seen.then_some(memory_total))
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
fn read_single_process_usage(pid: u32) -> (Option<ProcessCpuTime>, Option<u64>) {
    use windows::Win32::Foundation::{CloseHandle, FILETIME};
    use windows::Win32::System::Threading::GetProcessTimes;

    let Some(handle) = open_process_for_metrics(pid) else {
        return (None, None);
    };

    let mut creation = FILETIME::default();
    let mut exit_time = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();
    let cpu_time = if unsafe {
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
            .map(|cpu_ns| ProcessCpuTime {
                key: ProcessKey {
                    pid,
                    started_at: filetime_to_u64(&creation),
                },
                cpu_ns,
            })
    } else {
        None
    };

    let memory_bytes = query_private_working_set(handle);
    let _ = unsafe { CloseHandle(handle) };
    (cpu_time, memory_bytes)
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
    if let Ok(handle) =
        unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid) }
    {
        return Some(handle);
    }
    // Processes that refuse memory reads still expose CPU times; the memory
    // query then fails on its own without dropping the CPU sample.
    unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok() }
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
fn read_process_tree_usage(root_pid: u32) -> (Vec<ProcessCpuTime>, Option<u64>) {
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

    let mut cpu_times = Vec::new();
    let mut memory_total = 0u64;
    let mut memory_seen = false;
    for pid in pids {
        cpu_times.extend(linux_process_cpu_time(pid));
        if let Some(value) = linux_process_memory_bytes(pid) {
            memory_total = memory_total.saturating_add(value);
            memory_seen = true;
        }
    }

    (cpu_times, memory_seen.then_some(memory_total))
}

#[cfg(target_os = "linux")]
fn linux_parent_pid(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    parse_stat_fields(&stat)?.ppid
}

#[cfg(target_os = "linux")]
fn linux_process_cpu_time(pid: u32) -> Option<ProcessCpuTime> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let fields = parse_stat_fields(&stat)?;
    let clock_ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    if clock_ticks <= 0 {
        return None;
    }

    let cpu_ns = fields
        .cpu_ticks?
        .checked_mul(1_000_000_000)?
        .checked_div(clock_ticks as u64)?;
    Some(ProcessCpuTime {
        key: ProcessKey {
            pid,
            started_at: fields.start_ticks?,
        },
        cpu_ns,
    })
}

/// Private memory of one process. Summing `VmRSS` over a tree counts pages
/// shared between parent and children once per process, so the private
/// counters from `smaps_rollup` are used instead; `VmRSS` is only the
/// fallback for kernels without that file.
#[cfg(target_os = "linux")]
fn linux_process_memory_bytes(pid: u32) -> Option<u64> {
    if let Some(bytes) = std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup"))
        .ok()
        .and_then(|rollup| parse_smaps_private_bytes(&rollup))
    {
        return Some(bytes);
    }

    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in status.lines() {
        if let Some(raw) = line.strip_prefix("VmRSS:") {
            let kilobytes = raw.split_whitespace().next()?.parse::<u64>().ok()?;
            return kilobytes.checked_mul(1024);
        }
    }
    None
}

/// Fields read from `/proc/<pid>/stat`.
#[cfg(any(target_os = "linux", test))]
#[derive(Debug, PartialEq, Eq)]
struct StatFields {
    ppid: Option<u32>,
    /// `utime + stime` in clock ticks.
    cpu_ticks: Option<u64>,
    /// Process start time in clock ticks since boot.
    start_ticks: Option<u64>,
}

/// Parse `/proc/<pid>/stat`. The command name may contain spaces and
/// parentheses, so fields are counted from the last `)`: the first field
/// after it is the state (field 3), followed by the parent PID (field 4).
#[cfg(any(target_os = "linux", test))]
fn parse_stat_fields(stat: &str) -> Option<StatFields> {
    let close_paren = stat.rfind(')')?;
    let fields: Vec<&str> = stat.get(close_paren + 1..)?.split_whitespace().collect();
    let field = |index: usize| fields.get(index).and_then(|raw| raw.parse::<u64>().ok());

    let cpu_ticks = match (field(11), field(12)) {
        (Some(user), Some(system)) => Some(user.saturating_add(system)),
        _ => None,
    };
    Some(StatFields {
        ppid: fields.get(1).and_then(|raw| raw.parse().ok()),
        cpu_ticks,
        start_ticks: field(19),
    })
}

/// Sum the `Private_Clean` and `Private_Dirty` lines of `smaps_rollup`.
#[cfg(any(target_os = "linux", test))]
fn parse_smaps_private_bytes(rollup: &str) -> Option<u64> {
    let mut total_kilobytes: Option<u64> = None;
    for line in rollup.lines() {
        let Some(raw) = line
            .strip_prefix("Private_Clean:")
            .or_else(|| line.strip_prefix("Private_Dirty:"))
        else {
            continue;
        };
        let kilobytes = raw.split_whitespace().next()?.parse::<u64>().ok()?;
        total_kilobytes = Some(total_kilobytes.unwrap_or(0).saturating_add(kilobytes));
    }
    total_kilobytes?.checked_mul(1024)
}

#[cfg(target_os = "macos")]
fn read_process_tree_usage(pid: u32) -> (Vec<ProcessCpuTime>, Option<u64>) {
    use libproc::libproc::pid_rusage::{pidrusage, RUsageInfoV2};

    let Ok(usage) = pidrusage::<RUsageInfoV2>(pid as i32) else {
        return (Vec::new(), None);
    };
    let cpu_time = ProcessCpuTime {
        key: ProcessKey {
            pid,
            started_at: usage.ri_proc_start_abstime,
        },
        cpu_ns: usage.ri_user_time.saturating_add(usage.ri_system_time),
    };
    (vec![cpu_time], Some(usage.ri_resident_size))
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn read_process_tree_usage(_pid: u32) -> (Vec<ProcessCpuTime>, Option<u64>) {
    (Vec::new(), None)
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

    fn cpu(pid: u32, started_at: u64, cpu_ns: u64) -> ProcessCpuTime {
        ProcessCpuTime {
            key: ProcessKey { pid, started_at },
            cpu_ns,
        }
    }

    #[test]
    fn cpu_meter_reports_delta_usage() {
        let mut meter = CpuMeter::default();
        let start = Instant::now();

        assert_eq!(meter.observe(&[cpu(1, 0, 0)], start), None);
        assert_eq!(
            meter.observe(&[cpu(1, 0, 500_000_000)], start + Duration::from_secs(1)),
            Some(50.0)
        );
        assert_eq!(
            meter.observe(&[cpu(1, 0, 1_000_000_000)], start + Duration::from_secs(2)),
            Some(50.0)
        );
    }

    #[test]
    fn cpu_meter_ignores_exited_children() {
        let mut meter = CpuMeter::default();
        let start = Instant::now();

        meter.observe(
            &[cpu(1, 0, 1_000_000_000), cpu(2, 5, 30_000_000_000)],
            start,
        );
        assert_eq!(
            meter.observe(&[cpu(1, 0, 1_500_000_000)], start + Duration::from_secs(1)),
            Some(50.0)
        );
    }

    #[test]
    fn cpu_meter_counts_new_and_recycled_processes_from_zero() {
        let mut meter = CpuMeter::default();
        let start = Instant::now();

        meter.observe(&[cpu(1, 0, 0), cpu(2, 5, 9_000_000_000)], start);
        // PID 2 was recycled by a new process (different start time) and a
        // new child PID 3 appeared; both count their whole CPU time.
        assert_eq!(
            meter.observe(
                &[cpu(1, 0, 0), cpu(2, 7, 250_000_000), cpu(3, 8, 250_000_000)],
                start + Duration::from_secs(1)
            ),
            Some(50.0)
        );
    }

    #[test]
    fn cpu_meter_resets_when_cpu_is_unavailable() {
        let mut meter = CpuMeter::default();
        let start = Instant::now();

        meter.observe(&[cpu(1, 0, 0)], start);
        assert_eq!(meter.observe(&[], start + Duration::from_secs(1)), None);
        assert_eq!(
            meter.observe(&[cpu(1, 0, 500_000_000)], start + Duration::from_secs(2)),
            None
        );
    }

    #[test]
    fn stat_parser_reads_parent_cpu_and_start_time() {
        let stat = "4242 (python (worker) x) S 100 4242 4242 0 -1 4194560 2000 0 0 0 \
                    150 50 0 0 20 0 3 0 987654 123456789 4096 18446744073709551615";
        assert_eq!(
            parse_stat_fields(stat),
            Some(StatFields {
                ppid: Some(100),
                cpu_ticks: Some(200),
                start_ticks: Some(987654),
            })
        );
        assert_eq!(parse_stat_fields("garbage"), None);
    }

    #[test]
    fn smaps_parser_sums_private_pages_only() {
        let rollup = "55d0c0000000-7ffd00000000 ---p 00000000 00:00 0  [rollup]\n\
                      Rss:              300000 kB\n\
                      Pss:              200000 kB\n\
                      Shared_Clean:     100000 kB\n\
                      Private_Clean:     50000 kB\n\
                      Private_Dirty:    150000 kB\n";
        assert_eq!(parse_smaps_private_bytes(rollup), Some(200_000 * 1024));
        assert_eq!(parse_smaps_private_bytes("Rss: 1 kB\n"), None);
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
