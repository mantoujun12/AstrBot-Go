use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use chrono::{
    DateTime, Datelike as _, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone as _, Weekday,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter as _, Manager as _};

use crate::commands::AppState;
use crate::error::{AppError, Result};
use crate::utils::fs_ops::replace_file_atomically;
use crate::utils::paths::get_data_dir;
use crate::utils::sync::lock_mutex_recover;

const JOBS_FILE: &str = "scheduled_jobs.json";
const UPDATE_EVENT: &str = "scheduled-jobs-updated";
const TICK: Duration = Duration::from_secs(20);
const MAX_NAME_CHARS: usize = 60;
const MAX_INTERVAL_MINUTES: u32 = 10_080;
const MAX_ERROR_CHARS: usize = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduledAction {
    StartInstance,
    StopInstance,
    Backup,
    ClearCache,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleMode {
    Once,
    Interval,
    Daily,
    Weekly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledJob {
    pub id: String,
    pub name: String,
    pub action: ScheduledAction,
    pub instance_id: String,
    pub mode: ScheduleMode,
    /// Local `YYYY-MM-DDTHH:MM`, only used by `once`.
    #[serde(default)]
    pub run_at: Option<String>,
    /// Only used by `interval`.
    #[serde(default)]
    pub interval_minutes: Option<u32>,
    /// Local `HH:MM`, used by `daily` and `weekly`.
    #[serde(default)]
    pub time: Option<String>,
    /// `0` = Sunday … `6` = Saturday, only used by `weekly`.
    #[serde(default)]
    pub weekday: Option<u8>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub next_run_at: Option<String>,
    #[serde(default)]
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub last_success: Option<bool>,
    #[serde(default)]
    pub last_error: Option<String>,
}

struct JobStore {
    jobs: Vec<ScheduledJob>,
}

impl JobStore {
    fn load() -> Self {
        Self {
            jobs: load_jobs(&get_data_dir()),
        }
    }

    fn persist(&self) -> bool {
        write_jobs(&get_data_dir(), &self.jobs)
    }
}

static JOB_STORE: Lazy<Mutex<JobStore>> = Lazy::new(|| Mutex::new(JobStore::load()));

fn store() -> std::sync::MutexGuard<'static, JobStore> {
    lock_mutex_recover(&JOB_STORE, "SCHEDULED_JOB_STORE")
}

fn jobs_file(data_dir: &Path) -> PathBuf {
    data_dir.join(JOBS_FILE)
}

fn load_jobs(data_dir: &Path) -> Vec<ScheduledJob> {
    let content = match std::fs::read_to_string(jobs_file(data_dir)) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };
    match serde_json::from_str::<Vec<ScheduledJob>>(&content) {
        Ok(jobs) => jobs,
        Err(error) => {
            log::warn!("Failed to parse scheduled jobs: {}", error);
            Vec::new()
        }
    }
}

fn write_jobs(data_dir: &Path, jobs: &[ScheduledJob]) -> bool {
    let path = jobs_file(data_dir);
    let content = match serde_json::to_string_pretty(jobs) {
        Ok(content) => content,
        Err(error) => {
            log::warn!("Failed to serialize scheduled jobs: {}", error);
            return false;
        }
    };
    if let Err(error) = std::fs::create_dir_all(data_dir) {
        log::warn!("Failed to create data directory: {}", error);
        return false;
    }
    let temp_path = data_dir.join(format!(".{}.{}.tmp", JOBS_FILE, uuid::Uuid::new_v4()));
    if let Err(error) = std::fs::write(&temp_path, content) {
        log::warn!("Failed to write scheduled jobs: {}", error);
        let _ = std::fs::remove_file(temp_path);
        return false;
    }
    if let Err(error) = replace_file_atomically(&temp_path, &path) {
        log::warn!("Failed to persist scheduled jobs: {}", error);
        let _ = std::fs::remove_file(temp_path);
        return false;
    }
    true
}

fn emit_jobs(app_handle: &AppHandle, jobs: &[ScheduledJob]) {
    let _ = app_handle.emit(UPDATE_EVENT, jobs.to_vec());
}

fn parse_local_datetime(value: &str) -> Option<DateTime<Local>> {
    let value = value.trim();
    let naive = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S"))
        .ok()?;
    Local.from_local_datetime(&naive).earliest()
}

fn parse_local_time(value: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(value.trim(), "%H:%M").ok()
}

fn weekday_from_number(value: u8) -> Option<Weekday> {
    match value {
        0 => Some(Weekday::Sun),
        1 => Some(Weekday::Mon),
        2 => Some(Weekday::Tue),
        3 => Some(Weekday::Wed),
        4 => Some(Weekday::Thu),
        5 => Some(Weekday::Fri),
        6 => Some(Weekday::Sat),
        _ => None,
    }
}

fn local_datetime(date: NaiveDate, time: NaiveTime) -> Option<DateTime<Local>> {
    Local.from_local_datetime(&date.and_time(time)).earliest()
}

fn next_daily(time: NaiveTime, now: DateTime<Local>) -> Option<DateTime<Local>> {
    let today = now.date_naive();
    let candidate = local_datetime(today, time)?;
    if candidate > now {
        return Some(candidate);
    }
    local_datetime(today.checked_add_days(chrono::Days::new(1))?, time)
}

fn next_weekly(weekday: Weekday, time: NaiveTime, now: DateTime<Local>) -> Option<DateTime<Local>> {
    let today = now.date_naive();
    let delta = (i64::from(weekday.num_days_from_sunday())
        - i64::from(today.weekday().num_days_from_sunday()))
    .rem_euclid(7);
    let date = today.checked_add_days(chrono::Days::new(delta as u64))?;
    let candidate = local_datetime(date, time)?;
    if candidate > now {
        return Some(candidate);
    }
    local_datetime(date.checked_add_days(chrono::Days::new(7))?, time)
}

/// Next local time this job should run, or `None` when it never runs again.
// shortcut: no cron expressions or monthly schedules, upgrade when a cron parser crate is added.
fn next_run(job: &ScheduledJob, now: DateTime<Local>) -> Option<DateTime<Local>> {
    match job.mode {
        ScheduleMode::Once => {
            let run_at = parse_local_datetime(job.run_at.as_deref()?)?;
            (run_at > now).then_some(run_at)
        }
        ScheduleMode::Interval => {
            let minutes = i64::from(job.interval_minutes?.clamp(1, MAX_INTERVAL_MINUTES));
            now.checked_add_signed(chrono::Duration::minutes(minutes))
        }
        ScheduleMode::Daily => next_daily(parse_local_time(job.time.as_deref()?)?, now),
        ScheduleMode::Weekly => next_weekly(
            weekday_from_number(job.weekday?)?,
            parse_local_time(job.time.as_deref()?)?,
            now,
        ),
    }
}

fn trimmed(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// Validate user input and normalize the fields the schedule depends on.
fn normalize(mut job: ScheduledJob) -> Result<ScheduledJob> {
    job.name = job.name.trim().chars().take(MAX_NAME_CHARS).collect();
    job.time = trimmed(&job.time);
    job.run_at = trimmed(&job.run_at);

    let manifest = crate::config::load_manifest()?;
    if !manifest.instances.contains_key(&job.instance_id) {
        return Err(AppError::instance_not_found(&job.instance_id));
    }

    let now = Local::now();
    match job.mode {
        ScheduleMode::Once => {
            let run_at = job
                .run_at
                .as_deref()
                .ok_or_else(|| AppError::other("请选择执行时间"))?;
            let parsed =
                parse_local_datetime(run_at).ok_or_else(|| AppError::other("执行时间格式无效"))?;
            if parsed <= now {
                return Err(AppError::other("执行时间必须晚于当前时间"));
            }
        }
        ScheduleMode::Interval => {
            let minutes = job
                .interval_minutes
                .ok_or_else(|| AppError::other("请填写执行间隔"))?;
            if !(1..=MAX_INTERVAL_MINUTES).contains(&minutes) {
                return Err(AppError::other("执行间隔超出允许范围"));
            }
        }
        ScheduleMode::Daily => {
            parse_local_time(job.time.as_deref().unwrap_or_default())
                .ok_or_else(|| AppError::other("执行时间格式无效"))?;
        }
        ScheduleMode::Weekly => {
            if !job.weekday.is_some_and(|value| value <= 6) {
                return Err(AppError::other("星期取值无效"));
            }
            parse_local_time(job.time.as_deref().unwrap_or_default())
                .ok_or_else(|| AppError::other("执行时间格式无效"))?;
        }
    }

    job.next_run_at = if job.enabled {
        next_run(&job, now).map(|value| value.to_rfc3339())
    } else {
        None
    };
    Ok(job)
}

/// Load the store and repair schedules that expired while the app was closed.
/// Missed runs are skipped instead of being replayed.
pub fn initialize() {
    Lazy::force(&JOB_STORE);
    let now = Local::now();
    let mut guard = store();
    let mut changed = false;
    for job in &mut guard.jobs {
        if !job.enabled {
            continue;
        }
        let next = next_run(job, now).map(|value| value.to_rfc3339());
        if job.next_run_at != next {
            job.next_run_at = next;
            changed = true;
        }
        if job.next_run_at.is_none() {
            job.enabled = false;
            changed = true;
        }
    }
    if changed {
        guard.persist();
    }
}

pub fn list_jobs() -> Vec<ScheduledJob> {
    store().jobs.clone()
}

pub fn save_job(app_handle: &AppHandle, job: ScheduledJob) -> Result<Vec<ScheduledJob>> {
    let mut job = normalize(job)?;
    let mut guard = store();
    if job.id.is_empty() {
        job.id = uuid::Uuid::new_v4().to_string();
        job.last_run_at = None;
        job.last_success = None;
        job.last_error = None;
        guard.jobs.push(job);
    } else {
        let index = guard
            .jobs
            .iter()
            .position(|item| item.id == job.id)
            .ok_or_else(|| AppError::other("定时任务不存在"))?;
        // Run statistics belong to the backend; an edit must not rewrite them.
        job.last_run_at = guard.jobs[index].last_run_at.clone();
        job.last_success = guard.jobs[index].last_success;
        job.last_error = guard.jobs[index].last_error.clone();
        guard.jobs[index] = job;
    }
    guard.persist();
    let jobs = guard.jobs.clone();
    drop(guard);
    emit_jobs(app_handle, &jobs);
    Ok(jobs)
}

pub fn delete_job(app_handle: &AppHandle, id: &str) -> Result<Vec<ScheduledJob>> {
    let mut guard = store();
    let index = guard
        .jobs
        .iter()
        .position(|job| job.id == id)
        .ok_or_else(|| AppError::other("定时任务不存在"))?;
    guard.jobs.remove(index);
    guard.persist();
    let jobs = guard.jobs.clone();
    drop(guard);
    emit_jobs(app_handle, &jobs);
    Ok(jobs)
}

pub fn set_enabled(app_handle: &AppHandle, id: &str, enabled: bool) -> Result<Vec<ScheduledJob>> {
    let mut guard = store();
    let index = guard
        .jobs
        .iter()
        .position(|job| job.id == id)
        .ok_or_else(|| AppError::other("定时任务不存在"))?;
    let mut job = guard.jobs[index].clone();
    job.enabled = enabled;
    let next = if enabled {
        next_run(&job, Local::now())
    } else {
        None
    };
    if enabled && next.is_none() {
        return Err(AppError::other("该任务已没有可执行的下次时间"));
    }
    job.next_run_at = next.map(|value| value.to_rfc3339());
    guard.jobs[index] = job;
    guard.persist();
    let jobs = guard.jobs.clone();
    drop(guard);
    emit_jobs(app_handle, &jobs);
    Ok(jobs)
}

async fn execute(app_handle: &AppHandle, job: &ScheduledJob) -> Result<()> {
    let state = app_handle.state::<AppState>();
    match job.action {
        ScheduledAction::StartInstance => {
            crate::commands::start_instance(app_handle.clone(), job.instance_id.clone(), state)
                .await
                .map(|_| ())
        }
        ScheduledAction::StopInstance => {
            crate::commands::stop_instance(job.instance_id.clone(), state).await
        }
        ScheduledAction::Backup => {
            crate::commands::create_backup(app_handle.clone(), job.instance_id.clone(), None, state)
                .await
                .map(|_| ())
        }
        ScheduledAction::ClearCache => {
            crate::commands::clear_pycache(job.instance_id.clone(), state).await
        }
    }
}

fn record_outcome(app_handle: &AppHandle, id: &str, error: Option<&AppError>) {
    let now = Local::now();
    let mut guard = store();
    let Some(index) = guard.jobs.iter().position(|job| job.id == id) else {
        return;
    };
    let job = &mut guard.jobs[index];
    job.last_run_at = Some(now.to_rfc3339());
    job.last_success = Some(error.is_none());
    job.last_error = error.map(|cause| {
        cause
            .to_string()
            .chars()
            .take(MAX_ERROR_CHARS)
            .collect::<String>()
    });
    if job.mode == ScheduleMode::Once {
        // A one-time job never fires again, even when the run failed.
        job.enabled = false;
        job.next_run_at = None;
    } else if job.enabled {
        let next = next_run(job, now).map(|value| value.to_rfc3339());
        job.next_run_at = next;
    }
    guard.persist();
    let jobs = guard.jobs.clone();
    drop(guard);
    emit_jobs(app_handle, &jobs);
}

pub async fn run_now(app_handle: &AppHandle, id: &str) -> Result<()> {
    let job = store()
        .jobs
        .iter()
        .find(|job| job.id == id)
        .cloned()
        .ok_or_else(|| AppError::other("定时任务不存在"))?;
    let outcome = execute(app_handle, &job).await;
    record_outcome(app_handle, id, outcome.as_ref().err());
    outcome
}

fn is_due(job: &ScheduledJob, now: DateTime<Local>) -> bool {
    job.next_run_at
        .as_deref()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .is_some_and(|next| next.with_timezone(&Local) <= now)
}

async fn run_due_jobs(app_handle: &AppHandle) {
    let due: Vec<ScheduledJob> = {
        let guard = store();
        let now = Local::now();
        guard
            .jobs
            .iter()
            .filter(|job| job.enabled && is_due(job, now))
            .cloned()
            .collect()
    };
    for job in due {
        let outcome = execute(app_handle, &job).await;
        record_outcome(app_handle, &job.id, outcome.as_ref().err());
    }
}

/// Background scheduler loop; jobs run one at a time and only while the app runs.
pub fn start(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            run_due_jobs(&app_handle).await;
            tokio::time::sleep(TICK).await;
        }
    });
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use chrono::Timelike as _;

    fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .earliest()
            .expect("valid local datetime")
    }

    fn job(mode: ScheduleMode) -> ScheduledJob {
        ScheduledJob {
            id: String::new(),
            name: "job".to_string(),
            action: ScheduledAction::Backup,
            instance_id: "instance".to_string(),
            mode,
            run_at: None,
            interval_minutes: None,
            time: None,
            weekday: None,
            enabled: true,
            next_run_at: None,
            last_run_at: None,
            last_success: None,
            last_error: None,
        }
    }

    #[test]
    fn job_round_trips_with_frontend_field_names() -> std::result::Result<(), serde_json::Error> {
        let mut once = job(ScheduleMode::Once);
        once.action = ScheduledAction::StartInstance;
        once.run_at = Some("2026-05-10T09:30".to_string());
        once.next_run_at = Some("2026-05-10T09:30:00+08:00".to_string());

        let value = serde_json::to_value(&once)?;
        assert_eq!(value["action"], "start_instance");
        assert_eq!(value["mode"], "once");
        assert_eq!(value["run_at"], "2026-05-10T09:30");
        assert!(value.get("next_run_at").is_some());

        let decoded: ScheduledJob = serde_json::from_value(value)?;
        assert_eq!(decoded.action, ScheduledAction::StartInstance);
        Ok(())
    }

    #[test]
    fn once_rejects_past_and_returns_future_time() {
        let now = at(2026, 5, 10, 12, 0);

        let mut past = job(ScheduleMode::Once);
        past.run_at = Some("2026-05-10T09:30".to_string());
        assert_eq!(next_run(&past, now), None);

        let mut future = job(ScheduleMode::Once);
        future.run_at = Some("2026-05-10T18:30".to_string());
        let next = next_run(&future, now).expect("future once run");
        assert_eq!((next.hour(), next.minute()), (18, 30));
        assert_eq!(next.date_naive(), now.date_naive());
    }

    #[test]
    fn interval_adds_the_configured_minutes() {
        let now = at(2026, 5, 10, 12, 0);
        let mut interval = job(ScheduleMode::Interval);
        interval.interval_minutes = Some(90);

        let next = next_run(&interval, now).expect("interval run");
        assert_eq!(next, now + chrono::Duration::minutes(90));
    }

    #[test]
    fn daily_returns_today_or_tomorrow_depending_on_the_time() {
        let now = at(2026, 5, 10, 8, 0);

        let mut before = job(ScheduleMode::Daily);
        before.time = Some("09:30".to_string());
        let next = next_run(&before, now).expect("daily run");
        assert_eq!(next.date_naive(), now.date_naive());
        assert_eq!((next.hour(), next.minute()), (9, 30));

        let mut after = job(ScheduleMode::Daily);
        after.time = Some("07:30".to_string());
        let next = next_run(&after, now).expect("daily run");
        assert_eq!(
            next.date_naive(),
            now.date_naive().succ_opt().expect("tomorrow")
        );
        assert_eq!((next.hour(), next.minute()), (7, 30));
    }

    #[test]
    fn weekly_picks_today_next_week_or_another_weekday() {
        let now = at(2026, 5, 10, 12, 0);
        let today = now.weekday();

        let mut later_today = job(ScheduleMode::Weekly);
        later_today.weekday = Some(now.weekday().num_days_from_sunday() as u8);
        later_today.time = Some("23:59".to_string());
        let next = next_run(&later_today, now).expect("weekly run");
        assert_eq!(next.date_naive(), now.date_naive());
        assert_eq!(next.weekday(), today);

        let mut passed_today = job(ScheduleMode::Weekly);
        passed_today.weekday = Some(now.weekday().num_days_from_sunday() as u8);
        passed_today.time = Some("00:00".to_string());
        let next = next_run(&passed_today, now).expect("weekly run");
        assert_eq!(
            next.date_naive(),
            now.date_naive()
                .checked_add_days(chrono::Days::new(7))
                .expect("next week")
        );

        let tomorrow = now.date_naive().succ_opt().expect("tomorrow");
        let mut tomorrow_job = job(ScheduleMode::Weekly);
        tomorrow_job.weekday = Some(tomorrow.weekday().num_days_from_sunday() as u8);
        tomorrow_job.time = Some("00:00".to_string());
        let next = next_run(&tomorrow_job, now).expect("weekly run");
        assert_eq!(next.date_naive(), tomorrow);
    }

    #[test]
    fn invalid_or_missing_schedule_fields_have_no_next_run() {
        let now = at(2026, 5, 10, 12, 0);

        let mut daily = job(ScheduleMode::Daily);
        daily.time = Some("not-a-time".to_string());
        assert_eq!(next_run(&daily, now), None);

        let mut weekly = job(ScheduleMode::Weekly);
        weekly.time = Some("08:00".to_string());
        weekly.weekday = Some(9);
        assert_eq!(next_run(&weekly, now), None);

        let interval = job(ScheduleMode::Interval);
        assert_eq!(next_run(&interval, now), None);
    }
}
