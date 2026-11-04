//! Runtime monitoring: liveness probes, resource sampling, and state reconciliation.
//!
//! All evaluation functions are standalone — they take snapshot data and return results.
//! The monitor task (spawned by ProcessManager) calls `poll_instances` on a timer.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use super::control::is_expected_process_alive;
use super::manager::{InstanceEntry, ProcessState, Slot};
use super::{
    compute_disk_usage, sample_metric, sample_process_resources, unix_secs_now, CpuMeter,
    InstanceState, ResourceAlertEvent, ResourceSample, RuntimeEvent,
};
use crate::utils::paths::get_instance_dir;
use crate::utils::sync::lock_mutex_recover;

#[cfg(target_os = "windows")]
use super::control::is_process_alive;
#[cfg(target_os = "windows")]
use super::win_api::get_pid_on_port;
#[cfg(target_os = "windows")]
use super::ALIVE_EXIT_THRESHOLD;

/// Snapshot of a single live instance for evaluation.
#[derive(Clone)]
struct LiveSnapshot {
    id: String,
    port: u16,
    pid: u32,
    executable_path: PathBuf,
    dashboard_enabled: bool,
    alive_failure_count: u32,
    next_alive_check_at: Option<Instant>,
    /// Flag indicating whether a disk sample should be taken on this tick
    /// (carried over from the previous state to maintain throttling cadence).
    disk_sample_tick: u32,
    /// Last measured disk usage, reused between throttled disk probes.
    last_disk_bytes: Option<u64>,
    /// Cumulative CPU usage meter from the previous monitor tick.
    cpu_meter: CpuMeter,
}

/// Outcome of evaluating a single instance during monitoring.
enum MonitorOutcome {
    /// The process is dead — its slot should be removed.
    Dead { id: String, reason: String },
    /// The process is alive — update its fields.
    Alive {
        id: String,
        /// Always 0 / `None` on non-Windows (no retry mechanism).
        alive_failure_count: u32,
        next_alive_check_at: Option<Instant>,
        new_pid: Option<u32>,
    },
}

/// Resource sampling result for a live instance.
struct ResourceOutcome {
    id: String,
    sample: ResourceSample,
    /// Current tick counter for disk sampling throttling.
    disk_sample_tick: u32,
    /// Updated CPU meter to apply back to the live process.
    cpu_meter: CpuMeter,
}

/// Entry point called by the monitor task on each tick.
///
/// Snap-then-apply: locks briefly to snapshot, evaluates with no
/// lock held, then locks again to apply results.
///
/// Emits state changes and resource alert events outside the state lock, and
/// returns the IDs (with exit reasons) of instances whose processes died
/// during this poll, so the manager can apply auto-restart policy.
pub(super) async fn poll_instances(
    state: &Mutex<ProcessState>,
    runtime_events: &tokio::sync::broadcast::Sender<RuntimeEvent>,
    resource_alerts: &tokio::sync::broadcast::Sender<ResourceAlertEvent>,
) -> Vec<(String, String)> {
    // Phase 1: lock → snapshot Live slots → unlock
    let entries: Vec<LiveSnapshot> = {
        let state = lock_mutex_recover(state, "ProcessState");
        if state.shutting_down {
            return Vec::new();
        }
        state
            .slots
            .iter()
            .filter_map(|(id, entry)| {
                if let Some(Slot::Live(p)) = &entry.slot {
                    Some(LiveSnapshot {
                        id: id.clone(),
                        port: p.port,
                        pid: p.pid,
                        executable_path: p.executable_path.clone(),
                        dashboard_enabled: p.dashboard_enabled,
                        alive_failure_count: p.alive_failure_count,
                        next_alive_check_at: p.next_alive_check_at,
                        disk_sample_tick: p.resource_history.disk_sample_tick,
                        last_disk_bytes: p.resource_history.last_disk_bytes,
                        cpu_meter: p.resource_history.cpu_meter.clone(),
                    })
                } else {
                    None
                }
            })
            .collect()
    };

    if entries.is_empty() {
        return Vec::new();
    }

    // Phase 2: evaluate liveness
    let outcomes = evaluate_instances(&entries);

    // Phase 2b: sample resources in a blocking task. Disk traversal can be
    // expensive and must not stall the async monitor loop.
    let alive_ids: std::collections::HashSet<String> = outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            MonitorOutcome::Alive { id, .. } => Some(id.clone()),
            MonitorOutcome::Dead { .. } => None,
        })
        .collect();
    let resource_outcomes = if alive_ids.is_empty() {
        Vec::new()
    } else {
        let sample_entries = entries.clone();
        match tokio::task::spawn_blocking(move || sample_resources(&sample_entries, &alive_ids))
            .await
        {
            Ok(results) => results,
            Err(error) => {
                log::warn!("Resource sampling task failed: {}", error);
                Vec::new()
            }
        }
    };

    // Phase 3: lock → apply outcomes → unlock → collect events
    let (events, dead, alert_events) = {
        let mut state = lock_mutex_recover(state, "ProcessState");
        let (runtime, dead) = apply_outcomes(&mut state, &outcomes);
        let alerts = apply_resource_samples(&mut state, &resource_outcomes);
        drop(state);
        (runtime, dead, alerts)
    };

    // Phase 4: emit events (outside lock)
    for (id, new_state) in events {
        let _ = runtime_events.send(RuntimeEvent {
            instance_id: id,
            state: new_state,
        });
    }

    for alert in alert_events {
        let _ = resource_alerts.send(alert);
    }

    dead
}

/// Diagnostic exit reason recorded when a live instance is declared dead.
fn exit_reason(entry: &LiveSnapshot) -> String {
    #[cfg(target_os = "windows")]
    {
        format!(
            "liveness probe failed {} consecutive times; process no longer running (pid {})",
            entry.alive_failure_count, entry.pid
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        format!("process exited (pid {})", entry.pid)
    }
}

/// Evaluate all instances via liveness probes.
fn evaluate_instances(entries: &[LiveSnapshot]) -> Vec<MonitorOutcome> {
    let now = Instant::now();
    let mut outcomes = Vec::new();

    for entry in entries {
        let Some(outcome) = evaluate_liveness(entry, now) else {
            outcomes.push(MonitorOutcome::Dead {
                id: entry.id.clone(),
                reason: exit_reason(entry),
            });
            continue;
        };

        outcomes.push(outcome);
    }

    outcomes
}

/// `apply_outcomes` result: (state-change events, dead IDs with reasons).
type ApplyResult = (Vec<(String, InstanceState)>, Vec<(String, String)>);

/// Apply monitor outcomes to the live process state. Returns (events to
/// emit, dead instance IDs with their recorded exit reasons).
fn apply_outcomes(state: &mut ProcessState, outcomes: &[MonitorOutcome]) -> ApplyResult {
    let mut events = Vec::new();
    let mut dead = Vec::new();

    for outcome in outcomes {
        match outcome {
            MonitorOutcome::Dead { id, reason } => {
                // Only remove slots still in Live state — another lifecycle
                // method may have transitioned the slot in the meantime.
                if matches!(
                    state.slots.get(id),
                    Some(InstanceEntry {
                        slot: Some(Slot::Live(_)),
                        ..
                    })
                ) {
                    state.slots.remove(id);
                    log::info!("Removed dead process tracking entry for instance {}", id);
                    events.push((id.clone(), InstanceState::Stopped));
                    dead.push((id.clone(), reason.clone()));
                }
            }
            MonitorOutcome::Alive {
                id,
                alive_failure_count,
                next_alive_check_at,
                new_pid,
            } => {
                if let Some(entry) = state.slots.get_mut(id) {
                    if let Some(Slot::Live(p)) = &mut entry.slot {
                        p.alive_failure_count = *alive_failure_count;
                        p.next_alive_check_at = *next_alive_check_at;
                        if let Some(new_pid) = new_pid {
                            log::info!(
                                "Instance {} PID updated: {} -> {} (port {})",
                                id,
                                p.pid,
                                new_pid,
                                p.port
                            );
                            p.pid = *new_pid;
                        }
                    }
                }
            }
        }
    }

    (events, dead)
}

// -- Resource sampling -------------------------------------------------------

/// Sample CPU/memory/disk for instances that remain alive after liveness evaluation.
fn sample_resources(
    entries: &[LiveSnapshot],
    alive_ids: &std::collections::HashSet<String>,
) -> Vec<ResourceOutcome> {
    let mut results = Vec::new();

    for entry in entries {
        if !alive_ids.contains(&entry.id) {
            continue;
        }

        let timestamp = unix_secs_now();
        let mut cpu_meter = entry.cpu_meter.clone();
        let (cpu_percent, memory_bytes) = sample_process_resources(entry.pid, &mut cpu_meter);

        // Disk sampling is throttled, but the last measured value is carried
        // into every sample so charts show a continuous disk line.
        let next_tick = entry.disk_sample_tick.wrapping_add(1);
        let should_sample_disk = super::resource::is_disk_sample_due(next_tick);

        let disk_bytes = if should_sample_disk {
            compute_disk_usage(&get_instance_dir(&entry.id)).or(entry.last_disk_bytes)
        } else {
            entry.last_disk_bytes
        };

        let sample = ResourceSample {
            timestamp,
            cpu_percent,
            memory_bytes,
            disk_bytes,
        };

        results.push(ResourceOutcome {
            id: entry.id.clone(),
            sample,
            disk_sample_tick: next_tick,
            cpu_meter,
        });
    }

    results
}

/// Apply resource samples to live slots and detect alert transitions.
/// Returns alert events that should be emitted.
fn apply_resource_samples(
    state: &mut ProcessState,
    outcomes: &[ResourceOutcome],
) -> Vec<ResourceAlertEvent> {
    let mut alerts = Vec::new();

    for outcome in outcomes {
        let Some(entry) = state.slots.get_mut(&outcome.id) else {
            continue;
        };
        let Some(Slot::Live(p)) = &mut entry.slot else {
            continue;
        };

        p.resource_history.disk_sample_tick = outcome.disk_sample_tick;
        p.resource_history.cpu_meter = outcome.cpu_meter.clone();
        p.resource_history.push(outcome.sample.clone());

        let threshold = p.resource_thresholds.clone();
        if !threshold.enabled {
            p.alert_tracker.clear();
            continue;
        }

        let breached = threshold.breached_metrics(&outcome.sample);

        for metric in ["cpu", "memory", "disk"] {
            if !p.alert_tracker.observe(metric, breached.contains(&metric)) {
                continue;
            }
            let Some(value) = sample_metric(&outcome.sample, metric) else {
                continue;
            };
            let Some(threshold_value) = threshold.threshold_for(metric) else {
                continue;
            };
            alerts.push(ResourceAlertEvent {
                instance_id: outcome.id.clone(),
                metric: metric.to_string(),
                value,
                threshold: threshold_value,
            });
        }
    }

    alerts
}

// -- Liveness evaluation ------------------------------------------------------

/// Evaluate liveness for a single instance. Returns `None` if the process is
/// dead (slot should be removed), `Some(MonitorOutcome::Alive { .. })` if alive.
fn evaluate_liveness(entry: &LiveSnapshot, now: Instant) -> Option<MonitorOutcome> {
    // Backoff: not yet time to probe — preserve previous counters.
    // On non-Windows this is always a no-op (next_alive_check_at is always None).
    if entry.dashboard_enabled {
        if let Some(next_at) = entry.next_alive_check_at {
            if now < next_at {
                return Some(MonitorOutcome::Alive {
                    id: entry.id.clone(),
                    alive_failure_count: entry.alive_failure_count,
                    next_alive_check_at: entry.next_alive_check_at,
                    new_pid: None,
                });
            }
        }
    }

    let (alive_failure_count, next_alive_check_at, new_pid) = probe_liveness(entry, now)?;

    // If dashboard disabled and we are in retry mode, treat as dead.
    if !entry.dashboard_enabled && next_alive_check_at.is_some() {
        return None;
    }

    Some(MonitorOutcome::Alive {
        id: entry.id.clone(),
        alive_failure_count,
        next_alive_check_at,
        new_pid,
    })
}

// -- Platform-specific liveness probing ---------------------------------------

/// Returns `None` for terminal death, or `Some((alive_failure_count,
/// next_alive_check_at, new_pid))` when the process is alive or retriable.
#[cfg(target_os = "windows")]
fn probe_liveness(
    entry: &LiveSnapshot,
    now: Instant,
) -> Option<(u32, Option<Instant>, Option<u32>)> {
    if is_expected_process_alive(entry.pid, &entry.executable_path) {
        return Some((0, None, None));
    }

    // PID check failed — try port-based PID discovery.
    if let Some(new_pid) = get_pid_on_port(entry.port) {
        if new_pid == entry.pid {
            if entry.alive_failure_count == 0 {
                log::warn!(
                    "Instance {} liveness probe failed for PID {}, but port {} still resolves to the same PID",
                    entry.id,
                    entry.pid,
                    entry.port
                );
            } else {
                log::debug!(
                    "Instance {} still resolves port {} to PID {} while liveness probe remains failed",
                    entry.id,
                    entry.port,
                    entry.pid
                );
            }
            // Fall through to failure handling below.
        } else if is_expected_process_alive(new_pid, &entry.executable_path) {
            return Some((0, None, Some(new_pid)));
        } else if is_process_alive(new_pid) {
            log::warn!(
                "Instance {} rejected PID update {} -> {}: executable path mismatch",
                entry.id,
                entry.pid,
                new_pid
            );
        } else {
            log::debug!(
                "Instance {} observed transient PID {} on port {}, but process was not alive during validation",
                entry.id,
                new_pid,
                entry.port
            );
        }
    }

    // Liveness probe failed — apply retry/threshold logic.
    let new_count = entry.alive_failure_count + 1;

    if new_count >= ALIVE_EXIT_THRESHOLD {
        log::warn!(
            "Instance {} liveness probe failed {} times, treating process as exited",
            entry.id,
            new_count
        );
        None
    } else {
        let interval = super::MONITOR_INTERVAL;
        log::debug!(
            "Instance {} liveness probe failed (count: {}), retry in {:?}",
            entry.id,
            new_count,
            interval
        );
        Some((new_count, Some(now + interval), None))
    }
}

#[cfg(not(target_os = "windows"))]
fn probe_liveness(
    entry: &LiveSnapshot,
    _now: Instant,
) -> Option<(u32, Option<Instant>, Option<u32>)> {
    if is_expected_process_alive(entry.pid, &entry.executable_path) {
        Some((0, None, None))
    } else {
        None
    }
}
