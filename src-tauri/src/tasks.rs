use std::collections::{BTreeMap, HashMap};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter as _};

use crate::error::{AppError, ErrorKind, Result};
use crate::utils::paths::get_data_dir;
use crate::utils::sync::lock_mutex_recover;

const MAX_PERSISTED_TASKS: usize = 100;
const TASK_UPDATED_EVENT: &str = "task-updated";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Download,
    Deploy,
    Backup,
    Repair,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Queued,
    Running,
    Paused,
    Cancelling,
    Cancelled,
    Succeeded,
    Failed,
    Interrupted,
}

impl TaskStatus {
    fn is_active(self) -> bool {
        matches!(
            self,
            Self::Queued | Self::Running | Self::Paused | Self::Cancelling
        )
    }

    fn is_terminal(self) -> bool {
        !self.is_active()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TaskError {
    pub code: u32,
    pub kind: ErrorKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TaskRecord {
    pub id: String,
    pub kind: TaskKind,
    pub title_key: String,
    pub title_args: BTreeMap<String, String>,
    pub target_id: Option<String>,
    pub status: TaskStatus,
    pub progress: Option<u8>,
    pub step: String,
    pub message_key: String,
    pub message: String,
    pub cancellable: bool,
    pub pausable: bool,
    pub error: Option<TaskError>,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TaskSpec {
    pub kind: TaskKind,
    pub title_key: &'static str,
    pub title_args: BTreeMap<String, String>,
    pub target_id: Option<String>,
    pub cancellable: bool,
    pub pausable: bool,
    pub dedupe_target: bool,
    pub control_ref: Option<String>,
    pub initial_message_key: String,
    pub initial_message: String,
}

impl TaskSpec {
    pub fn new(kind: TaskKind, title_key: &'static str) -> Self {
        Self {
            kind,
            title_key,
            title_args: BTreeMap::new(),
            target_id: None,
            cancellable: false,
            pausable: false,
            dedupe_target: false,
            control_ref: None,
            initial_message_key: "tasks.status.queued".to_string(),
            initial_message: "排队中".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskContext {
    pub app_handle: AppHandle,
    pub id: String,
    token: CancellationToken,
}

impl TaskContext {
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    pub fn check_cancelled(&self) -> Result<()> {
        if self.is_cancelled() {
            return Err(AppError::other("任务已取消"));
        }
        Ok(())
    }

    pub fn progress(
        &self,
        progress: Option<u8>,
        step: &str,
        message_key: &str,
        message: &str,
    ) -> Result<()> {
        update_progress(
            &self.id,
            progress,
            step,
            message_key,
            message,
            TaskStatus::Running,
            true,
        )
    }
}

#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone)]
struct TaskRuntime {
    app_handle: AppHandle,
    token: CancellationToken,
    control_ref: Option<String>,
    last_persisted_at: Instant,
}

#[derive(Debug)]
struct TaskManager {
    records: Vec<TaskRecord>,
    runtimes: HashMap<String, TaskRuntime>,
}

impl TaskManager {
    fn load() -> Self {
        let data_dir = get_data_dir();
        let LoadedRecords {
            mut records,
            legacy_to_remove,
        } = read_persisted_records(&data_dir);
        let now = chrono::Utc::now().to_rfc3339();
        for record in &mut records {
            if record.status.is_active() {
                record.status = TaskStatus::Interrupted;
                record.finished_at = Some(now.clone());
                record.updated_at = now.clone();
                record.message_key = "tasks.status.interrupted".to_string();
                record.message = "应用退出时任务被中断".to_string();
            }
        }
        let manager = Self {
            records,
            runtimes: HashMap::new(),
        };
        // Only drop the legacy file after the migrated records are safely stored
        // in the data root.
        if manager.persist_in(&data_dir) {
            if let Some(legacy_path) = legacy_to_remove.as_deref() {
                remove_legacy_tasks_file(legacy_path);
            }
        }
        manager
    }

    fn register(
        &mut self,
        app_handle: &AppHandle,
        spec: TaskSpec,
    ) -> Result<(TaskContext, TaskRecord)> {
        if spec.dedupe_target {
            if let Some(target_id) = &spec.target_id {
                let duplicate = self.records.iter().any(|record| {
                    record.kind == spec.kind
                        && record.target_id.as_deref() == Some(target_id.as_str())
                        && record.status.is_active()
                });
                if duplicate {
                    return Err(AppError::other("该任务已在进行中"));
                }
            }
        }

        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();
        let token = CancellationToken::new();
        let record = TaskRecord {
            id: id.clone(),
            kind: spec.kind,
            title_key: spec.title_key.to_string(),
            title_args: spec.title_args,
            target_id: spec.target_id,
            status: TaskStatus::Queued,
            progress: None,
            step: "queued".to_string(),
            message_key: spec.initial_message_key.to_string(),
            message: spec.initial_message.to_string(),
            cancellable: spec.cancellable,
            pausable: spec.pausable,
            error: None,
            created_at: now.clone(),
            updated_at: now,
            started_at: None,
            finished_at: None,
        };
        let context = TaskContext {
            app_handle: app_handle.clone(),
            id: id.clone(),
            token: token.clone(),
        };
        self.runtimes.insert(
            id,
            TaskRuntime {
                app_handle: app_handle.clone(),
                token,
                control_ref: spec.control_ref,
                last_persisted_at: Instant::now(),
            },
        );
        self.records.push(record.clone());
        self.persist();
        emit_record(app_handle, &record);
        Ok((context, record))
    }

    fn finish(
        &mut self,
        app_handle: &AppHandle,
        id: &str,
        result: std::result::Result<(), ErrorKind>,
    ) {
        let Some(index) = self.records.iter().position(|item| item.id == id) else {
            return;
        };
        let runtime = self.runtimes.remove(id);
        let app_handle = runtime
            .as_ref()
            .map(|runtime| runtime.app_handle.clone())
            .unwrap_or_else(|| app_handle.clone());
        let token_cancelled = runtime
            .as_ref()
            .is_some_and(|runtime| runtime.token.is_cancelled());
        let status = outcome_status(&result, token_cancelled);

        let now = chrono::Utc::now().to_rfc3339();
        let record = &mut self.records[index];
        if !record.status.is_terminal() {
            record.status = status;
            record.finished_at = Some(now.clone());
            if record.started_at.is_none() {
                record.started_at = Some(now.clone());
            }
            record.progress = if record.status == TaskStatus::Succeeded {
                Some(100)
            } else {
                record.progress
            };
            if status == TaskStatus::Cancelled {
                record.message_key = "tasks.status.cancelled".to_string();
                record.message = "任务已取消".to_string();
            } else if let Err(kind) = result {
                record.error = Some(TaskError::from_kind(kind));
                record.message_key = "tasks.status.failed".to_string();
                record.message = "任务失败".to_string();
            } else {
                record.message_key = "tasks.status.succeeded".to_string();
                record.message = "任务已完成".to_string();
            }
            record.updated_at = now;
        }
        let record = record.clone();
        self.persist();
        emit_record(&app_handle, &record);
    }

    fn persist(&self) {
        self.persist_in(&get_data_dir());
    }

    /// Persist into an explicit data directory snapshot so the write and any
    /// related legacy cleanup always target the same location.
    fn persist_in(&self, data_dir: &Path) -> bool {
        let mut active: Vec<_> = self
            .records
            .iter()
            .filter(|record| record.status.is_active())
            .cloned()
            .collect();
        let mut terminal: Vec<_> = self
            .records
            .iter()
            .filter(|record| record.status.is_terminal())
            .cloned()
            .collect();
        terminal.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        terminal.truncate(MAX_PERSISTED_TASKS);
        terminal.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        active.append(&mut terminal);
        write_persisted_records(data_dir, &active)
    }
}

/// A run that finished its work succeeds even when a cancel request arrived too late.
fn outcome_status(
    result: &std::result::Result<(), ErrorKind>,
    token_cancelled: bool,
) -> TaskStatus {
    match result {
        Ok(()) => TaskStatus::Succeeded,
        Err(ErrorKind::DownloadCancelled) => TaskStatus::Cancelled,
        Err(_) if token_cancelled => TaskStatus::Cancelled,
        Err(_) => TaskStatus::Failed,
    }
}

impl TaskError {
    fn from_kind(kind: ErrorKind) -> Self {
        Self {
            code: kind.code(),
            kind,
        }
    }
}

static TASK_MANAGER: Lazy<Mutex<TaskManager>> = Lazy::new(|| Mutex::new(TaskManager::load()));

fn manager() -> MutexGuard<TaskManager> {
    lock_mutex_recover(&TASK_MANAGER, "TASK_MANAGER")
}

type MutexGuard<T> = std::sync::MutexGuard<'static, T>;

/// Task records loaded from disk, plus the legacy file they were migrated from.
/// The legacy file is only removed after these records have been persisted to
/// the data root.
struct LoadedRecords {
    records: Vec<TaskRecord>,
    legacy_to_remove: Option<PathBuf>,
}

/// Path to the task records file in the data root.
fn tasks_file(data_dir: &Path) -> PathBuf {
    data_dir.join("tasks.json")
}

/// Legacy location used before task records moved to the data root.
fn legacy_tasks_file(data_dir: &Path) -> PathBuf {
    data_dir.join("tasks").join("tasks.json")
}

/// Read task records from `path`, returning `None` when the file is missing or
/// cannot be read or parsed.
fn read_records_file(path: &Path) -> Option<Vec<TaskRecord>> {
    let content = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<Vec<TaskRecord>>(&content) {
        Ok(records) => Some(records),
        Err(error) => {
            log::warn!("Failed to parse persisted task records: {}", error);
            None
        }
    }
}

fn read_persisted_records(data_dir: &Path) -> LoadedRecords {
    if let Some(records) = read_records_file(&tasks_file(data_dir)) {
        return LoadedRecords {
            records,
            legacy_to_remove: None,
        };
    }
    // The root file is missing or unusable: fall back to a valid legacy file so
    // upgrading users do not lose their task history.
    let legacy_path = legacy_tasks_file(data_dir);
    match read_records_file(&legacy_path) {
        Some(records) => LoadedRecords {
            records,
            legacy_to_remove: Some(legacy_path),
        },
        None => LoadedRecords {
            records: Vec::new(),
            legacy_to_remove: None,
        },
    }
}

/// Persist task records into `data_dir`, returning whether the write succeeded.
fn write_persisted_records(data_dir: &Path, records: &[TaskRecord]) -> bool {
    let path = tasks_file(data_dir);
    let content = match serde_json::to_string_pretty(records) {
        Ok(content) => content,
        Err(error) => {
            log::warn!("Failed to serialize task records: {}", error);
            return false;
        }
    };
    if let Err(error) = std::fs::create_dir_all(data_dir) {
        log::warn!("Failed to create data directory: {}", error);
        return false;
    }
    let temp_path = data_dir.join(format!(".tasks.json.{}.tmp", uuid::Uuid::new_v4()));
    if let Err(error) = std::fs::write(&temp_path, content) {
        log::warn!("Failed to write task records: {}", error);
        let _ = std::fs::remove_file(temp_path);
        return false;
    }
    if let Err(error) = crate::utils::fs_ops::replace_file_atomically(&temp_path, &path) {
        log::warn!("Failed to persist task records: {}", error);
        let _ = std::fs::remove_file(temp_path);
        return false;
    }
    true
}

/// Drop a migrated legacy task file so a stale copy can no longer shadow the
/// records in the data root.
fn remove_legacy_tasks_file(legacy_path: &Path) {
    if let Err(error) = std::fs::remove_file(legacy_path) {
        if error.kind() != std::io::ErrorKind::NotFound {
            log::warn!("Failed to remove legacy task records: {}", error);
        }
        return;
    }
    if let Some(parent) = legacy_path.parent() {
        let _ = std::fs::remove_dir(parent);
    }
}

fn emit_record(app_handle: &AppHandle, record: &TaskRecord) {
    let _ = app_handle.emit(TASK_UPDATED_EVENT, record);
}

pub fn initialize() {
    Lazy::force(&TASK_MANAGER);
}

pub fn list_tasks() -> Vec<TaskRecord> {
    manager().records.clone()
}

pub fn remove_task(app_handle: &AppHandle, id: &str) -> Result<()> {
    let mut store = manager();
    let Some(index) = store.records.iter().position(|item| item.id == id) else {
        return Err(AppError::other("任务不存在"));
    };
    if store.records[index].status.is_active() {
        return Err(AppError::other("任务仍在运行"));
    }
    let removed = store.records.remove(index);
    store.persist();
    drop(store);
    let _ = app_handle.emit("task-removed", &removed.id);
    Ok(())
}

pub fn clear_finished_tasks(app_handle: &AppHandle) {
    let mut store = manager();
    let removed: Vec<String> = store
        .records
        .iter()
        .filter(|record| record.status.is_terminal())
        .map(|record| record.id.clone())
        .collect();
    store.records.retain(|record| record.status.is_active());
    store.persist();
    drop(store);
    for id in removed {
        let _ = app_handle.emit("task-removed", &id);
    }
}
fn task_outcome<T>(outcome: &Result<T>) -> std::result::Result<(), ErrorKind> {
    outcome.as_ref().map(|_| ()).map_err(AppError::kind)
}

/// Run an async operation while exposing it in the unified task center.
///
/// The command keeps awaiting the operation so existing UI flows retain their
/// completion and error semantics. The task record is finalized exactly once.
pub async fn execute<F, Fut, T>(app_handle: AppHandle, spec: TaskSpec, work: F) -> Result<T>
where
    F: FnOnce(TaskContext) -> Fut + Send,
    Fut: Future<Output = Result<T>> + Send,
    T: Send,
{
    let (context, record) = manager().register(&app_handle, spec)?;
    let task_id = record.id;
    let outcome = work(context).await;
    manager().finish(&app_handle, &task_id, task_outcome(&outcome));
    outcome
}

/// Run blocking work on the blocking pool while exposing it in the task center.
pub async fn execute_blocking<F, T>(app_handle: AppHandle, spec: TaskSpec, work: F) -> Result<T>
where
    F: FnOnce(TaskContext) -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    let (context, record) = manager().register(&app_handle, spec)?;
    let task_id = record.id;
    let outcome = match tokio::task::spawn_blocking(move || work(context)).await {
        Ok(outcome) => outcome,
        Err(error) => Err(AppError::process(format!(
            "Task worker panicked: {}",
            error
        ))),
    };
    manager().finish(&app_handle, &task_id, task_outcome(&outcome));
    outcome
}

pub fn request_cancel(id: &str) -> Result<()> {
    let mut store = manager();
    let runtime = store
        .runtimes
        .get(id)
        .cloned()
        .ok_or_else(|| AppError::other("没有正在运行的任务"))?;
    let Some(index) = store.records.iter().position(|item| item.id == id) else {
        return Err(AppError::other("任务不存在"));
    };
    if !store.records[index].cancellable {
        return Err(AppError::other("该任务不支持取消"));
    }
    runtime.token.cancel();
    if let Some(control_ref) = &runtime.control_ref {
        crate::download_control::cancel(control_ref);
    }
    let app_handle = runtime.app_handle;
    store.records[index].status = TaskStatus::Cancelling;
    store.records[index].updated_at = chrono::Utc::now().to_rfc3339();
    let record = store.records[index].clone();
    store.persist();
    drop(store);
    emit_record(&app_handle, &record);
    Ok(())
}

pub fn request_pause(id: &str) -> Result<()> {
    let store = manager();
    let runtime = store
        .runtimes
        .get(id)
        .cloned()
        .ok_or_else(|| AppError::other("没有正在运行的任务"))?;
    let Some(index) = store.records.iter().position(|item| item.id == id) else {
        return Err(AppError::other("任务不存在"));
    };
    if !store.records[index].pausable {
        return Err(AppError::other("该任务不支持暂停"));
    }
    if store.records[index].status != TaskStatus::Running {
        return Err(AppError::other("只有运行中的任务可以暂停"));
    }
    if let Some(control_ref) = &runtime.control_ref {
        crate::download_control::pause(control_ref)?;
    }
    let app_handle = runtime.app_handle;
    drop(store);
    update_status(
        &app_handle,
        id,
        TaskStatus::Paused,
        "paused",
        "tasks.status.paused",
        "已暂停",
    )
}

pub fn request_resume(id: &str) -> Result<()> {
    let store = manager();
    let runtime = store
        .runtimes
        .get(id)
        .cloned()
        .ok_or_else(|| AppError::other("没有正在运行的任务"))?;
    let Some(index) = store.records.iter().position(|item| item.id == id) else {
        return Err(AppError::other("任务不存在"));
    };
    if store.records[index].status != TaskStatus::Paused {
        return Err(AppError::other("只有已暂停的任务可以继续"));
    }
    if let Some(control_ref) = &runtime.control_ref {
        crate::download_control::resume(control_ref)?;
    }
    let app_handle = runtime.app_handle;
    drop(store);
    update_status(
        &app_handle,
        id,
        TaskStatus::Running,
        "running",
        "tasks.status.running",
        "运行中",
    )
}

fn update_status(
    app_handle: &AppHandle,
    id: &str,
    status: TaskStatus,
    step: &str,
    message_key: &str,
    message: &str,
) -> Result<()> {
    let mut store = manager();
    let Some(index) = store.records.iter().position(|item| item.id == id) else {
        return Err(AppError::other("任务不存在"));
    };
    let app_handle = store
        .runtimes
        .get(id)
        .map(|runtime| runtime.app_handle.clone())
        .unwrap_or_else(|| app_handle.clone());
    let record = &mut store.records[index];
    if !record.status.is_active() {
        return Ok(());
    }
    record.status = status;
    record.step = step.to_string();
    record.message_key = message_key.to_string();
    record.message = message.to_string();
    record.updated_at = chrono::Utc::now().to_rfc3339();
    let record = record.clone();
    store.persist();
    drop(store);
    emit_record(&app_handle, &record);
    Ok(())
}

fn update_progress(
    id: &str,
    progress: Option<u8>,
    step: &str,
    message_key: &str,
    message: &str,
    fallback_status: TaskStatus,
    check_cancellation: bool,
) -> Result<()> {
    let mut store = manager();
    let runtime = store.runtimes.get(id).cloned();
    let Some(index) = store.records.iter().position(|item| item.id == id) else {
        return Err(AppError::other("任务不存在"));
    };
    let app_handle = runtime
        .as_ref()
        .map(|runtime| runtime.app_handle.clone())
        .ok_or_else(|| AppError::other("没有正在运行的任务"))?;
    if check_cancellation
        && runtime
            .as_ref()
            .is_some_and(|runtime| runtime.token.is_cancelled())
    {
        return Err(AppError::other("任务已取消"));
    }
    let record = &mut store.records[index];
    if !record.status.is_active() {
        return Ok(());
    }
    record.progress = progress;
    record.step = step.to_string();
    record.message_key = message_key.to_string();
    record.message = message.to_string();
    if record.status != TaskStatus::Cancelling && record.status != fallback_status {
        record.status = fallback_status;
    }
    if record.started_at.is_none() {
        record.started_at = Some(chrono::Utc::now().to_rfc3339());
    }
    record.updated_at = chrono::Utc::now().to_rfc3339();
    let record = record.clone();
    let should_persist = runtime
        .as_ref()
        .is_some_and(|runtime| runtime.last_persisted_at.elapsed() >= Duration::from_secs(2));
    if should_persist {
        store.persist();
        if let Some(runtime) = store.runtimes.get_mut(id) {
            runtime.last_persisted_at = Instant::now();
        }
    }
    drop(store);
    emit_record(&app_handle, &record);
    Ok(())
}

pub fn handle_download_progress(
    task_id: Option<&str>,
    progress: Option<u8>,
    step: &str,
    message_key: &str,
    message: &str,
) -> Result<()> {
    let Some(task_id) = task_id else {
        return Ok(());
    };
    let status = match step {
        "paused" => TaskStatus::Paused,
        "cancelled" => TaskStatus::Cancelling,
        _ => TaskStatus::Running,
    };
    update_progress(task_id, progress, step, message_key, message, status, false)
}

pub fn handle_instance_progress(
    _app_handle: &AppHandle,
    target_id: &str,
    step: &str,
    message_key: &str,
    message: &str,
    progress: u8,
) -> Result<()> {
    let Some(task_id) = active_task_id_for_target(&[TaskKind::Repair, TaskKind::Deploy], target_id)
    else {
        return Ok(());
    };

    update_progress(
        &task_id,
        Some(progress),
        step,
        message_key,
        message,
        TaskStatus::Running,
        true,
    )
}

fn active_task_id_for_target(kinds: &[TaskKind], target_id: &str) -> Option<String> {
    let store = manager();
    store
        .records
        .iter()
        .rev()
        .find(|record| {
            kinds.contains(&record.kind)
                && record.target_id.as_deref() == Some(target_id)
                && record.status.is_active()
        })
        .map(|record| record.id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record() -> TaskRecord {
        TaskRecord {
            id: "task-1".to_string(),
            kind: TaskKind::Download,
            title_key: "tasks.titles.downloadVersion".to_string(),
            title_args: BTreeMap::from([("version".to_string(), "v4.0.0".to_string())]),
            target_id: Some("v4.0.0".to_string()),
            status: TaskStatus::Paused,
            progress: Some(42),
            step: "paused".to_string(),
            message_key: "progress.download.paused".to_string(),
            message: "已暂停".to_string(),
            cancellable: true,
            pausable: true,
            error: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:01Z".to_string(),
            started_at: Some("2026-01-01T00:00:00Z".to_string()),
            finished_at: None,
        }
    }

    #[test]
    fn cancellation_token_is_cooperative() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn terminal_status_classification_is_stable() {
        assert!(TaskStatus::Queued.is_active());
        assert!(TaskStatus::Running.is_active());
        assert!(TaskStatus::Paused.is_active());
        assert!(TaskStatus::Cancelling.is_active());
        assert!(TaskStatus::Cancelled.is_terminal());
        assert!(TaskStatus::Succeeded.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Interrupted.is_terminal());
    }

    #[test]
    fn task_record_round_trips_with_frontend_field_names(
    ) -> std::result::Result<(), serde_json::Error> {
        let value = serde_json::to_value(sample_record())?;

        assert_eq!(value["kind"], "download");
        assert_eq!(value["status"], "paused");
        assert_eq!(value["title_args"]["version"], "v4.0.0");

        let decoded: TaskRecord = serde_json::from_value(value)?;
        assert_eq!(decoded.kind, TaskKind::Download);
        assert_eq!(decoded.status, TaskStatus::Paused);
        assert_eq!(decoded.progress, Some(42));
        Ok(())
    }

    #[test]
    fn task_error_kind_matches_the_frontend_catalog() -> std::result::Result<(), serde_json::Error>
    {
        let value = serde_json::to_value(TaskError::from_kind(ErrorKind::ProcessLocking))?;

        assert_eq!(value["code"], 3006);
        assert_eq!(value["kind"], "process_locking");
        Ok(())
    }

    #[test]
    fn task_outcome_preserves_the_original_error_kind() {
        let outcome: Result<()> = Err(AppError::backup("failed"));

        assert_eq!(task_outcome(&outcome), Err(ErrorKind::Backup));
    }

    #[test]
    fn completed_work_wins_over_a_late_cancel_request() {
        let done: std::result::Result<(), ErrorKind> = Ok(());
        let aborted: std::result::Result<(), ErrorKind> = Err(ErrorKind::Other);
        let cancelled_download: std::result::Result<(), ErrorKind> =
            Err(ErrorKind::DownloadCancelled);

        assert_eq!(outcome_status(&done, true), TaskStatus::Succeeded);
        assert_eq!(outcome_status(&aborted, true), TaskStatus::Cancelled);
        assert_eq!(outcome_status(&aborted, false), TaskStatus::Failed);
        assert_eq!(
            outcome_status(&cancelled_download, false),
            TaskStatus::Cancelled
        );
    }

    struct TestDataDir(PathBuf);

    impl TestDataDir {
        fn new() -> Result<Self> {
            let path = std::env::temp_dir().join(format!("tasks-test-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&path)?;
            Ok(Self(path))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDataDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn write_records(path: &Path, records: &[TaskRecord]) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(records)?)?;
        Ok(())
    }

    fn sample_with_id(id: &str) -> TaskRecord {
        let mut record = sample_record();
        record.id = id.to_string();
        record
    }

    #[test]
    fn root_records_take_precedence_over_legacy_records() -> Result<()> {
        let data_dir = TestDataDir::new()?;
        write_records(&tasks_file(data_dir.path()), &[sample_with_id("root")])?;
        write_records(
            &legacy_tasks_file(data_dir.path()),
            &[sample_with_id("legacy")],
        )?;

        let loaded = read_persisted_records(data_dir.path());

        assert_eq!(loaded.records.len(), 1);
        assert_eq!(loaded.records[0].id, "root");
        assert!(loaded.legacy_to_remove.is_none());
        Ok(())
    }

    #[test]
    fn malformed_root_records_fall_back_to_valid_legacy_records() -> Result<()> {
        let data_dir = TestDataDir::new()?;
        std::fs::write(tasks_file(data_dir.path()), b"{ not json")?;
        let legacy_path = legacy_tasks_file(data_dir.path());
        write_records(&legacy_path, &[sample_with_id("legacy")])?;

        let loaded = read_persisted_records(data_dir.path());

        assert_eq!(loaded.records.len(), 1);
        assert_eq!(loaded.records[0].id, "legacy");
        assert_eq!(
            loaded.legacy_to_remove.as_deref(),
            Some(legacy_path.as_path())
        );
        Ok(())
    }

    #[test]
    fn unreadable_legacy_records_are_kept_and_not_marked_for_removal() -> Result<()> {
        let data_dir = TestDataDir::new()?;
        let legacy_path = legacy_tasks_file(data_dir.path());
        if let Some(parent) = legacy_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&legacy_path, b"{ not json")?;

        let loaded = read_persisted_records(data_dir.path());

        assert!(loaded.records.is_empty());
        assert!(loaded.legacy_to_remove.is_none());
        assert!(legacy_path.is_file());
        Ok(())
    }

    #[test]
    fn migrated_legacy_records_are_removed_after_a_successful_write() -> Result<()> {
        let data_dir = TestDataDir::new()?;
        write_records(
            &legacy_tasks_file(data_dir.path()),
            &[sample_with_id("legacy")],
        )?;

        let LoadedRecords {
            records,
            legacy_to_remove,
        } = read_persisted_records(data_dir.path());
        assert!(write_persisted_records(data_dir.path(), &records));
        let Some(legacy_path) = legacy_to_remove else {
            return Err(AppError::other("legacy cleanup was not requested"));
        };
        remove_legacy_tasks_file(&legacy_path);

        assert!(!legacy_path.exists());
        let Some(legacy_parent) = legacy_path.parent() else {
            return Err(AppError::other("legacy task file has no parent"));
        };
        assert!(!legacy_parent.exists());
        assert_eq!(
            read_records_file(&tasks_file(data_dir.path())).map(|items| items.len()),
            Some(1)
        );
        Ok(())
    }
}
