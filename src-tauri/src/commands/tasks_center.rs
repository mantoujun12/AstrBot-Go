use crate::error::Result;
use crate::tasks::{self, TaskRecord};
use tauri::AppHandle;

// === Unified task center ===

#[tauri::command]
pub async fn list_tasks() -> Result<Vec<TaskRecord>> {
    Ok(tasks::list_tasks())
}

#[tauri::command]
pub async fn cancel_task(task_id: String) -> Result<()> {
    tasks::request_cancel(&task_id)
}

#[tauri::command]
pub async fn pause_task(task_id: String) -> Result<()> {
    tasks::request_pause(&task_id)
}

#[tauri::command]
pub async fn resume_task(task_id: String) -> Result<()> {
    tasks::request_resume(&task_id)
}

#[tauri::command]
pub async fn remove_task(app_handle: AppHandle, task_id: String) -> Result<()> {
    tasks::remove_task(&app_handle, &task_id)
}

#[tauri::command]
pub async fn clear_finished_tasks(app_handle: AppHandle) -> Result<()> {
    tasks::clear_finished_tasks(&app_handle);
    Ok(())
}
