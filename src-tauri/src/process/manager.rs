//! Process manager — `Mutex<ProcessState>` approach.
//!
//! Sync methods for queries (direct lock → read → unlock).
//! Async methods for lifecycle operations that involve IO.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{broadcast, watch};

use super::control::graceful_shutdown;
use super::monitor;
use super::{
    auto_restart_delay_secs, InstanceProcess, InstanceRestartEvent, InstanceRuntimeInfo,
    InstanceState, ResourceAlertEvent, ResourceAlertThreshold, ResourceSample, RuntimeEvent,
    AUTO_RESTART_STABLE_RUN_SECS,
};
use tauri::Emitter as _;

use crate::backup::find_pending_auto_backup;
use crate::config::load_manifest;
use crate::error::{AppError, ErrorKind, Result};
use crate::instance::lifecycle;
use crate::utils::sync::lock_mutex_recover;

/// A single slot in the process state machine.
///
/// Each instance occupies at most one slot. The variant encodes the lifecycle
/// phase; CRUD guard tracking lives alongside in [`InstanceEntry`].
pub(super) enum Slot {
    /// Launch IO in progress.
    Starting,
    /// Process running.
    Live(InstanceProcess),
    /// Shutdown IO in progress. Keeps the same process info so the exit
    /// handler can force-kill processes that are still shutting down.
    Stopping(InstanceProcess),
}

/// Per-instance entry combining lifecycle slot and CRUD guard state.
///
/// An entry with `slot: None, guarded: true` represents a stopped instance
/// undergoing a CRUD operation.  The entry is removed when the guard drops.
pub(super) struct InstanceEntry {
    pub(super) slot: Option<Slot>,
    guarded: bool,
}

/// Tracks the auto-restart budget for one instance.
#[derive(Default)]
pub(super) struct RestartTracker {
    /// Consecutive unexpected exits since the last stable run.
    pub(super) attempts: u32,
    /// Set by a manual stop so a sleeping restart task aborts itself.
    pub(super) cancelled: bool,
    /// Diagnostic reason recorded for the latest unexpected exit.
    pub(super) last_exit_reason: String,
}

pub(super) struct ProcessState {
    pub(super) slots: HashMap<String, InstanceEntry>,
    pub(super) shutting_down: bool,
    pub(super) restarts: HashMap<String, RestartTracker>,
}

fn configured_resource_thresholds(instance_id: &str) -> ResourceAlertThreshold {
    let defaults = ResourceAlertThreshold::default();
    let Ok(manifest) = load_manifest() else {
        return defaults;
    };
    let Some(instance) = manifest.instances.get(instance_id) else {
        return defaults;
    };

    ResourceAlertThreshold {
        enabled: instance.resource_alert_enabled,
        cpu_percent: instance
            .resource_alert_cpu_percent
            .unwrap_or(defaults.cpu_percent),
        memory_bytes: instance
            .resource_alert_memory_bytes
            .unwrap_or(defaults.memory_bytes),
        disk_bytes: instance
            .resource_alert_disk_bytes
            .unwrap_or(defaults.disk_bytes),
    }
}

impl ProcessState {
    /// Reject if the application is shutting down.
    fn check_active(&self, id: &str) -> Result<()> {
        if self.shutting_down {
            return Err(AppError::process(format!(
                "Cannot operate on instance {id}: application is shutting down"
            )));
        }
        Ok(())
    }

    /// Reject if `id` is guarded or already occupies a lifecycle slot.
    fn ensure_vacant(&self, id: &str) -> Result<()> {
        match self.slots.get(id) {
            None => Ok(()),
            Some(entry) if entry.guarded => Err(AppError::process(format!(
                "Instance {id}: another operation is in progress"
            ))),
            Some(_) => Err(AppError::instance_running()),
        }
    }

    /// Transition a `Live` slot to `Stopping`, returning the pid and exe path.
    ///
    /// Returns an appropriate error for every non-`Live` state.
    fn prepare_stop(&mut self, id: &str) -> Result<(u32, PathBuf)> {
        let entry = match self.slots.get_mut(id) {
            None => return Err(AppError::instance_not_running()),
            Some(e) => e,
        };

        if entry.guarded {
            return Err(AppError::process(format!(
                "Instance {id}: another operation is in progress"
            )));
        }

        match entry.slot.take() {
            Some(Slot::Live(p)) => {
                let pid = p.pid;
                let exe = p.executable_path.clone();
                entry.slot = Some(Slot::Stopping(p));
                Ok((pid, exe))
            }
            Some(slot @ Slot::Stopping(_)) => {
                entry.slot = Some(slot);
                Err(AppError::process(format!(
                    "Instance {id} is already stopping"
                )))
            }
            Some(slot @ Slot::Starting) => {
                entry.slot = Some(slot);
                Err(AppError::process(format!(
                    "Instance {id} is still starting"
                )))
            }
            None => Err(AppError::instance_not_running()),
        }
    }

    /// Revert a `Stopping` slot back to `Live`. Returns `true` if reverted.
    fn revert_stop(&mut self, id: &str) -> bool {
        if let Some(entry) = self.slots.get_mut(id) {
            if let Some(Slot::Stopping(p)) = entry.slot.take() {
                entry.slot = Some(Slot::Live(p));
                return true;
            }
        }
        false
    }

    /// Drain all slots for application shutdown and collect killable processes.
    ///
    /// Sets `shutting_down = true`, drains every entry, and returns
    /// `(id, InstanceProcess)` for `Live`/`Stopping` entries.
    fn drain_for_shutdown(&mut self) -> Vec<(String, InstanceProcess)> {
        self.shutting_down = true;

        self.slots
            .drain()
            .filter_map(|(id, entry)| match entry.slot {
                Some(Slot::Live(p)) | Some(Slot::Stopping(p)) => Some((id, p)),
                Some(Slot::Starting) => {
                    log::info!(
                        "Cleared Starting slot for instance {} during shutdown \
                         (async task will handle cleanup)",
                        id
                    );
                    None
                }
                None => None,
            })
            .collect()
    }
}

/// Manages running instance processes via a shared mutex-guarded state.
#[derive(Clone)]
pub struct ProcessManager {
    state: Arc<Mutex<ProcessState>>,
    runtime_events: broadcast::Sender<RuntimeEvent>,
    resource_alerts: broadcast::Sender<ResourceAlertEvent>,
    shutdown_signal: watch::Sender<bool>,
    app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        let (runtime_events, _) = broadcast::channel(128);
        let (resource_alerts, _) = broadcast::channel(64);
        let (shutdown_signal, _) = watch::channel(false);
        let state = Arc::new(Mutex::new(ProcessState {
            slots: HashMap::new(),
            shutting_down: false,
            restarts: HashMap::new(),
        }));

        Self {
            state,
            runtime_events,
            resource_alerts,
            shutdown_signal,
            app_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Associates the manager with the Tauri app handle so it can emit
    /// lifecycle events and spawn auto-restart tasks.
    pub fn bind_app_handle(&self, app_handle: tauri::AppHandle) {
        *lock_mutex_recover(&self.app_handle, "AppHandleSlot") = Some(app_handle);
    }

    /// Spawn the background monitor task that periodically polls all instances.
    ///
    /// Must be called after the Tauri async runtime is available (e.g. in `setup`).
    pub fn start_monitor(&self, app_handle: tauri::AppHandle) {
        self.bind_app_handle(app_handle);
        let monitor_state = Arc::clone(&self.state);
        let monitor_events = self.runtime_events.clone();
        let monitor_alerts = self.resource_alerts.clone();
        let pm = self.clone();
        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(super::MONITOR_INTERVAL);
            loop {
                interval.tick().await;
                let dead =
                    monitor::poll_instances(&monitor_state, &monitor_events, &monitor_alerts).await;
                pm.reset_stable_restart_budget();
                for (id, reason) in dead {
                    pm.handle_unexpected_exit(&id, reason);
                }
            }
        });
    }

    pub fn subscribe_runtime_events(&self) -> broadcast::Receiver<RuntimeEvent> {
        self.runtime_events.subscribe()
    }

    pub fn subscribe_resource_alerts(&self) -> broadcast::Receiver<ResourceAlertEvent> {
        self.resource_alerts.subscribe()
    }

    // -- Sync methods (direct lock → read/write → unlock) ---------------------

    /// Returns the port for a **live** instance, or `None` if the instance is
    /// not in the `Live` state.
    ///
    /// Ports are not available during `Starting` or other transient states
    /// because the actual port is only known after launch completes.
    pub fn get_port(&self, id: &str) -> Option<u16> {
        let state = lock_mutex_recover(&self.state, "ProcessState");
        match state.slots.get(id) {
            Some(InstanceEntry {
                slot: Some(Slot::Live(p)),
                ..
            }) => Some(p.port),
            _ => None,
        }
    }

    pub fn get_runtime_info(&self) -> HashMap<String, InstanceRuntimeInfo> {
        let state = lock_mutex_recover(&self.state, "ProcessState");
        state
            .slots
            .iter()
            .filter_map(|(id, entry)| {
                let info = match &entry.slot {
                    Some(Slot::Starting) => InstanceRuntimeInfo::Starting,
                    Some(Slot::Live(p)) => InstanceRuntimeInfo::Live {
                        port: p.port,
                        dashboard_enabled: p.dashboard_enabled,
                    },
                    Some(Slot::Stopping(p)) => InstanceRuntimeInfo::Stopping {
                        port: p.port,
                        dashboard_enabled: p.dashboard_enabled,
                    },
                    None => return None,
                };
                Some((id.clone(), info))
            })
            .collect()
    }

    /// Returns IDs of instances that are either live or starting.
    ///
    /// Used to persist in-progress instance IDs across application restarts.
    pub fn get_active_ids(&self) -> Vec<String> {
        let state = lock_mutex_recover(&self.state, "ProcessState");
        state
            .slots
            .iter()
            .filter(|(_, entry)| matches!(&entry.slot, Some(Slot::Live(_)) | Some(Slot::Starting)))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Return the full resource history for a live instance.
    pub fn get_resource_history(&self, id: &str) -> Vec<ResourceSample> {
        let state = lock_mutex_recover(&self.state, "ProcessState");
        match state.slots.get(id) {
            Some(InstanceEntry {
                slot: Some(Slot::Live(process)),
                ..
            }) => process.resource_history.samples().iter().cloned().collect(),
            _ => Vec::new(),
        }
    }

    /// Return the latest resource sample for every live instance.
    pub fn get_all_latest_resources(&self) -> HashMap<String, ResourceSample> {
        let state = lock_mutex_recover(&self.state, "ProcessState");
        state
            .slots
            .iter()
            .filter_map(|(id, entry)| match &entry.slot {
                Some(Slot::Live(process)) => process
                    .resource_history
                    .latest()
                    .map(|sample| (id.clone(), sample.clone())),
                _ => None,
            })
            .collect()
    }

    /// Returns recorded exit reason and consecutive crash attempts for every
    /// instance that currently holds a restart tracker.
    pub fn get_restart_info(&self) -> HashMap<String, (String, u32)> {
        let state = lock_mutex_recover(&self.state, "ProcessState");
        state
            .restarts
            .iter()
            .map(|(id, tracker)| {
                (
                    id.clone(),
                    (tracker.last_exit_reason.clone(), tracker.attempts),
                )
            })
            .collect()
    }

    /// Apply alert thresholds to a live instance.
    pub fn set_resource_thresholds(
        &self,
        id: &str,
        thresholds: ResourceAlertThreshold,
    ) -> Result<()> {
        let mut state = lock_mutex_recover(&self.state, "ProcessState");
        match state.slots.get_mut(id) {
            Some(InstanceEntry {
                slot: Some(Slot::Live(process)),
                ..
            }) => {
                process.resource_thresholds = thresholds;
                process.alert_tracker.clear();
                Ok(())
            }
            _ => Err(AppError::instance_not_running()),
        }
    }

    /// Return the active threshold configuration for a live instance.
    pub fn get_resource_thresholds(&self, id: &str) -> Option<ResourceAlertThreshold> {
        let state = lock_mutex_recover(&self.state, "ProcessState");
        match state.slots.get(id) {
            Some(InstanceEntry {
                slot: Some(Slot::Live(process)),
                ..
            }) => Some(process.resource_thresholds.clone()),
            _ => None,
        }
    }

    /// Resets the crash budget for instances that have been live beyond the
    /// stable-run threshold. Called once per monitor tick.
    pub(super) fn reset_stable_restart_budget(&self) {
        let mut state = lock_mutex_recover(&self.state, "ProcessState");
        let stable_ids: Vec<String> = state
            .slots
            .iter()
            .filter_map(|(id, entry)| match &entry.slot {
                Some(Slot::Live(p)) => p
                    .started_at
                    .filter(|t| t.elapsed() >= Duration::from_secs(AUTO_RESTART_STABLE_RUN_SECS))
                    .map(|_| id.clone()),
                _ => None,
            })
            .collect();
        for id in stable_ids {
            if let Some(tracker) = state.restarts.get_mut(&id) {
                if tracker.attempts > 0 {
                    log::info!(
                        "Instance {} ran stably for over {}s, resetting auto-restart attempts",
                        id,
                        AUTO_RESTART_STABLE_RUN_SECS
                    );
                    tracker.attempts = 0;
                }
            }
        }
    }

    /// Reacts to an unexpected exit detected by the monitor.
    ///
    /// Manual stops never reach this path: they transition through
    /// `Stopping` -> `Stopped` directly. When auto-restart is enabled for the
    /// instance, schedules a backoff-delayed restart and emits an
    /// `instance-restart` event to the frontend.
    pub(super) fn handle_unexpected_exit(&self, id: &str, reason: String) {
        let manifest = match load_manifest() {
            Ok(m) => m,
            Err(e) => {
                log::warn!("Cannot read instance config for auto-restart: {}", e);
                return;
            }
        };
        let Some(instance) = manifest.instances.get(id) else {
            return;
        };
        if !instance.auto_restart_enabled {
            let mut state = lock_mutex_recover(&self.state, "ProcessState");
            state.restarts.remove(id);
            drop(state);
            return;
        }
        let max_retries = instance.auto_restart_max_retries.max(1);
        let notify = instance.auto_restart_notify;
        let instance_name = instance.name.clone();

        let attempt = {
            let mut state = lock_mutex_recover(&self.state, "ProcessState");
            if state.shutting_down {
                return;
            }
            let tracker = state.restarts.entry(id.to_string()).or_default();
            tracker.cancelled = false;
            tracker.last_exit_reason = reason.clone();
            tracker.attempts += 1;
            let attempt = tracker.attempts;
            drop(state);
            attempt
        };

        let delay_secs = auto_restart_delay_secs(attempt);
        let will_retry = attempt <= max_retries;

        log::warn!(
            "Instance {} exited unexpectedly (attempt {}/{}, reason: {})",
            id,
            attempt,
            max_retries,
            reason
        );

        let app_slot = lock_mutex_recover(&self.app_handle, "AppHandleSlot");
        if let Some(app) = &*app_slot {
            let payload = InstanceRestartEvent {
                instance_id: id.to_string(),
                instance_name,
                attempt,
                max_retries,
                will_retry,
                delay_secs,
                reason,
            };
            if notify {
                let _ = app.emit("instance-restart", &payload);
            }
        }
        drop(app_slot);

        if !will_retry {
            log::warn!(
                "Instance {} exceeded the maximum auto-restart retries ({}), not restarting again",
                id,
                max_retries
            );
            return;
        }

        let pm = self.clone();
        let id_owned = id.to_string();
        let app_slot = lock_mutex_recover(&self.app_handle, "AppHandleSlot");
        let app = app_slot.clone();
        drop(app_slot);
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(Duration::from_secs(delay_secs)).await;
            let abort = {
                let state = lock_mutex_recover(&pm.state, "ProcessState");
                state.shutting_down
                    || state.slots.contains_key(&id_owned)
                    || state
                        .restarts
                        .get(&id_owned)
                        .map(|t| t.cancelled)
                        .unwrap_or(false)
            };
            if abort {
                log::info!("Skipping scheduled auto-restart for instance {}", id_owned);
                return;
            }
            // The switch may have been turned off during the backoff delay.
            let still_enabled = load_manifest()
                .ok()
                .and_then(|m| m.instances.get(&id_owned).map(|i| i.auto_restart_enabled))
                .unwrap_or(false);
            if !still_enabled {
                log::info!(
                    "Auto-restart for instance {} was disabled, skipping",
                    id_owned
                );
                return;
            }
            let Some(app) = app else {
                return;
            };
            if let Err(e) = pm.start_instance(&id_owned, app).await {
                log::warn!(
                    "Scheduled auto-restart failed to launch instance {}: {}",
                    id_owned,
                    e
                );
                pm.handle_unexpected_exit(&id_owned, format!("launch failed: {e}"));
            }
        });
    }

    /// Acquire a guard that prevents lifecycle operations on the instance.
    /// The guard is released when dropped.
    pub fn acquire_guard(&self, id: &str) -> Result<InstanceGuard> {
        let mut state = lock_mutex_recover(&self.state, "ProcessState");
        state.check_active(id)?;
        state.ensure_vacant(id)?;
        state.slots.insert(
            id.to_string(),
            InstanceEntry {
                slot: None,
                guarded: true,
            },
        );
        drop(state);
        Ok(InstanceGuard {
            instance_id: id.to_string(),
            state: Arc::clone(&self.state),
        })
    }

    // -- Async methods (involve IO) -------------------------------------------

    pub async fn start_instance(&self, id: &str, app_handle: tauri::AppHandle) -> Result<u16> {
        // Check for pending auto backup before attempting to start
        let pending_auto_backup = find_pending_auto_backup(id)?;
        if let Some(pending_auto_backup) = pending_auto_backup {
            return Err(AppError::backup(format!(
                "检测到未处理的自动备份 {}（{}）。这通常表示上次升降级的数据还原失败。请先在“备份管理”中手动恢复该备份并确认数据，再启动实例。",
                pending_auto_backup.filename, pending_auto_backup.path
            )));
        }

        // Phase 1: lock → check → set Starting → unlock
        {
            let mut state = lock_mutex_recover(&self.state, "ProcessState");
            state.check_active(id)?;
            state.ensure_vacant(id)?;
            state.slots.insert(
                id.to_string(),
                InstanceEntry {
                    slot: Some(Slot::Starting),
                    guarded: false,
                },
            );
        }
        self.emit(id, InstanceState::Starting);

        // Phase 2+3: launch IO and finalize slot
        self.launch_and_finalize(id, &app_handle).await
    }

    pub async fn stop_instance(&self, id: &str) -> Result<()> {
        // Cancel any pending auto-restart first: a user-initiated stop must
        // not be followed by a delayed restart task.
        {
            let mut state = lock_mutex_recover(&self.state, "ProcessState");
            state.check_active(id)?;
            if let Some(tracker) = state.restarts.get_mut(id) {
                tracker.cancelled = true;
                tracker.attempts = 0;
            }
        }
        // Phase 2: lock → transition to Stopping → unlock
        let (pid, exe) = {
            let mut state = lock_mutex_recover(&self.state, "ProcessState");
            state.check_active(id)?;
            state.prepare_stop(id)?
        };
        self.emit(id, InstanceState::Stopping);

        // Phase 2: shutdown IO (no lock held)
        if let Err(e) = lifecycle::shutdown_instance(pid, exe).await {
            self.revert_stopping_to_live(id);
            return Err(e);
        }

        // Phase 3: finalize — remove slot and emit Stopped.
        self.finalize_stop(id);
        Ok(())
    }

    pub async fn restart_instance(&self, id: &str, app_handle: tauri::AppHandle) -> Result<u16> {
        // Check for pending auto backup before attempting to restart
        let pending_auto_backup = find_pending_auto_backup(id)?;
        if let Some(pending_auto_backup) = pending_auto_backup {
            return Err(AppError::backup(format!(
                "检测到未处理的自动备份 {}（{}）。这通常表示上次升降级的数据还原失败。请先在“备份管理”中手动恢复该备份并确认数据，再启动实例。",
                pending_auto_backup.filename, pending_auto_backup.path
            )));
        }

        // Phase 1: lock → check → prepare stop if running, or insert Starting directly
        let stop_info = {
            let mut state = lock_mutex_recover(&self.state, "ProcessState");
            state.check_active(id)?;
            match state.prepare_stop(id) {
                Ok((pid, exe)) => Some((pid, exe)),
                Err(e) if e.kind() == ErrorKind::InstanceNotRunning => {
                    // Not running — go straight to Starting.
                    state.slots.insert(
                        id.to_string(),
                        InstanceEntry {
                            slot: Some(Slot::Starting),
                            guarded: false,
                        },
                    );
                    None
                }
                Err(e) => return Err(e),
            }
        };

        // Phase 2: stop if was running, then atomically transition Stopping → Starting
        if let Some((pid, exe)) = stop_info {
            self.emit(id, InstanceState::Stopping);
            if let Err(e) = lifecycle::shutdown_instance(pid, exe).await {
                log::error!("Instance {} shutdown failed during restart: {}", id, e);
                self.revert_stopping_to_live(id);
                return Err(e);
            }
            // Atomically transition Stopping → Starting (no unprotected gap).
            {
                let mut state = lock_mutex_recover(&self.state, "ProcessState");
                if state.shutting_down {
                    state.slots.remove(id);
                    drop(state);
                    self.emit(id, InstanceState::Stopped);
                    return Err(AppError::process(format!(
                        "Cannot restart instance {id}: application is shutting down"
                    )));
                }
                if let Some(entry) = state.slots.get_mut(id) {
                    entry.slot = Some(Slot::Starting);
                }
            }
        }

        // Skip emitting Stopped to avoid UI flicker — go directly to Starting.
        self.emit(id, InstanceState::Starting);

        // Phase 3: launch IO and finalize slot
        self.launch_and_finalize(id, &app_handle).await
    }

    /// Run launch IO and finalize the slot.
    ///
    /// Precondition: the caller has already inserted `Slot::Starting` for `id`.
    async fn launch_and_finalize(&self, id: &str, app_handle: &tauri::AppHandle) -> Result<u16> {
        let result =
            lifecycle::launch_instance(id, app_handle, self.shutdown_signal.subscribe()).await;

        let shutting_down = {
            let mut state = lock_mutex_recover(&self.state, "ProcessState");
            if state.shutting_down {
                state.slots.remove(id);
                true
            } else {
                false
            }
        };
        if shutting_down {
            if let Ok(launch) = result {
                log::info!(
                    "Killing late-started instance {} (pid: {}) due to shutdown",
                    id,
                    launch.pid
                );
                if let Err(e) =
                    lifecycle::shutdown_instance(launch.pid, launch.executable_path).await
                {
                    log::warn!("Failed to kill late-started instance {}: {}", id, e);
                }
            }
            return Err(AppError::process(format!(
                "Cannot start instance {id}: application is shutting down"
            )));
        }

        let thresholds = configured_resource_thresholds(id);
        let mut state = lock_mutex_recover(&self.state, "ProcessState");
        match result {
            Ok(launch) => {
                let port = launch.port;
                if let Some(entry) = state.slots.get_mut(id) {
                    let mut process = InstanceProcess::new(
                        launch.pid,
                        launch.executable_path,
                        launch.port,
                        launch.dashboard_enabled,
                        #[cfg(target_os = "windows")]
                        launch.job_object,
                    );
                    process.resource_thresholds = thresholds;
                    entry.slot = Some(Slot::Live(process));
                }
                drop(state);
                self.emit(id, InstanceState::Running);
                Ok(port)
            }
            Err(e) => {
                state.slots.remove(id);
                drop(state);
                self.emit(id, InstanceState::Stopped);
                Err(e)
            }
        }
    }

    /// Blocking shutdown for the exit handler.
    ///
    /// Called from the Tauri `RunEvent::Exit` handler (not inside async context).
    /// Drains ALL slots and force-kills every instance that has a known PID.
    pub fn stop_all_blocking(&self) {
        let entries = {
            let mut state = lock_mutex_recover(&self.state, "ProcessState");
            state.drain_for_shutdown()
        };
        let _ = self.shutdown_signal.send(true);

        if entries.is_empty() {
            return;
        }

        for (id, p) in &entries {
            log::info!(
                "Stopping instance {} (pid: {}, port: {})",
                id,
                p.pid,
                p.port
            );
        }

        let target_refs: Vec<(u32, &std::path::Path)> = entries
            .iter()
            .map(|(_, p)| (p.pid, p.executable_path.as_path()))
            .collect();

        graceful_shutdown(&target_refs);
    }

    /// Revert a `Stopping` slot back to `Live` after a failed shutdown,
    /// so the user can retry. Emits `Running` if the slot was restored.
    fn revert_stopping_to_live(&self, id: &str) {
        let mut state = lock_mutex_recover(&self.state, "ProcessState");
        if state.revert_stop(id) {
            drop(state);
            self.emit(id, InstanceState::Running);
        }
    }

    /// Finalize a stop: remove the slot and emit Stopped.
    fn finalize_stop(&self, id: &str) {
        let mut state = lock_mutex_recover(&self.state, "ProcessState");
        state.slots.remove(id);
        // Manual stops are not crashes: forget the crash budget.
        if let Some(tracker) = state.restarts.get_mut(id) {
            tracker.attempts = 0;
        }
        drop(state);
        self.emit(id, InstanceState::Stopped);
    }

    fn emit(&self, instance_id: &str, state: InstanceState) {
        let _ = self.runtime_events.send(RuntimeEvent {
            instance_id: instance_id.to_string(),
            state,
        });
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard that prevents lifecycle operations on an instance.
/// Released automatically when dropped.
pub struct InstanceGuard {
    instance_id: String,
    state: Arc<Mutex<ProcessState>>,
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        let mut state = lock_mutex_recover(&self.state, "ProcessState");
        // Remove the guard-only entry. If shutdown drained it, this is a no-op.
        if matches!(
            state.slots.get(&self.instance_id),
            Some(InstanceEntry {
                slot: None,
                guarded: true
            })
        ) {
            state.slots.remove(&self.instance_id);
        }
    }
}
