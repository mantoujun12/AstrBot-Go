use crate::error::Result;
use crate::scheduler::{self, ScheduledJob};
use tauri::AppHandle;

// === Scheduled tasks ===

#[tauri::command]
pub async fn list_scheduled_jobs() -> Result<Vec<ScheduledJob>> {
    Ok(scheduler::list_jobs())
}

#[tauri::command]
pub async fn save_scheduled_job(
    app_handle: AppHandle,
    job: ScheduledJob,
) -> Result<Vec<ScheduledJob>> {
    scheduler::save_job(&app_handle, job)
}

#[tauri::command]
pub async fn delete_scheduled_job(
    app_handle: AppHandle,
    job_id: String,
) -> Result<Vec<ScheduledJob>> {
    scheduler::delete_job(&app_handle, &job_id)
}

#[tauri::command]
pub async fn set_scheduled_job_enabled(
    app_handle: AppHandle,
    job_id: String,
    enabled: bool,
) -> Result<Vec<ScheduledJob>> {
    scheduler::set_enabled(&app_handle, &job_id, enabled)
}

#[tauri::command]
pub async fn run_scheduled_job_now(app_handle: AppHandle, job_id: String) -> Result<()> {
    scheduler::run_now(&app_handle, &job_id).await
}
