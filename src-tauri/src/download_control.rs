//! Cooperative pause / resume / cancel controls for in-flight downloads.
//!
//! Each download registers a control under its id (the version tag for version
//! downloads). UI commands flip the state; the download loop checks it between
//! chunks and waits while paused.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use once_cell::sync::Lazy;
use tokio::sync::Notify;

use crate::error::{AppError, Result};
use crate::utils::sync::lock_mutex_recover;

const STATE_RUNNING: u8 = 0;
const STATE_PAUSED: u8 = 1;
const STATE_CANCELLED: u8 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlState {
    Running,
    Paused,
    Cancelled,
}

/// Shared control handle for one in-flight download.
#[derive(Debug)]
pub struct DownloadControl {
    state: AtomicU8,
    notify: Notify,
}

impl DownloadControl {
    fn new() -> Self {
        Self {
            state: AtomicU8::new(STATE_RUNNING),
            notify: Notify::new(),
        }
    }

    pub fn state(&self) -> ControlState {
        match self.state.load(Ordering::SeqCst) {
            STATE_PAUSED => ControlState::Paused,
            STATE_CANCELLED => ControlState::Cancelled,
            _ => ControlState::Running,
        }
    }

    fn pause(&self) {
        let _ = self.state.compare_exchange(
            STATE_RUNNING,
            STATE_PAUSED,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        self.notify.notify_waiters();
    }

    fn resume(&self) {
        let _ = self.state.compare_exchange(
            STATE_PAUSED,
            STATE_RUNNING,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        self.notify.notify_waiters();
    }

    fn cancel(&self) {
        self.state.store(STATE_CANCELLED, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    /// Wait until another thread signals a state change.
    ///
    /// The timeout keeps callers responsive even when a notification races
    /// with the state check that happens right before waiting.
    pub async fn wait_for_change(&self) {
        let _ = tokio::time::timeout(Duration::from_millis(500), self.notify.notified()).await;
    }
}

static DOWNLOAD_CONTROLS: Lazy<Mutex<HashMap<String, Arc<DownloadControl>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn controls() -> std::sync::MutexGuard<'static, HashMap<String, Arc<DownloadControl>>> {
    lock_mutex_recover(&DOWNLOAD_CONTROLS, "DOWNLOAD_CONTROLS")
}

/// Unregisters its control when the download finishes, fails, or is cancelled.
pub struct DownloadGuard {
    id: String,
    control: Arc<DownloadControl>,
}

impl DownloadGuard {
    pub fn control(&self) -> &DownloadControl {
        &self.control
    }
}

impl Drop for DownloadGuard {
    fn drop(&mut self) {
        let mut map = controls();
        let is_current = map
            .get(&self.id)
            .is_some_and(|control| Arc::ptr_eq(control, &self.control));
        if is_current {
            map.remove(&self.id);
        }
    }
}

/// Register a new download under `id`.
///
/// Fails when another download with the same id is already running so the UI
/// cannot start a duplicate.
pub fn acquire(id: &str) -> Result<DownloadGuard> {
    let mut map = controls();
    if map.contains_key(id) {
        return Err(AppError::other("该下载已在进行中"));
    }
    let control = Arc::new(DownloadControl::new());
    map.insert(id.to_string(), Arc::clone(&control));
    drop(map);
    Ok(DownloadGuard {
        id: id.to_string(),
        control,
    })
}

fn find(id: &str) -> Result<Arc<DownloadControl>> {
    controls()
        .get(id)
        .cloned()
        .ok_or_else(|| AppError::other("没有正在进行的下载"))
}

pub fn pause(id: &str) -> Result<()> {
    find(id)?.pause();
    Ok(())
}

pub fn resume(id: &str) -> Result<()> {
    find(id)?.resume();
    Ok(())
}

/// Cancel a download.
///
/// Does nothing when no download is active because the requested end state
/// (not downloading) has already been reached.
pub fn cancel(id: &str) {
    let control = controls().get(id).cloned();
    if let Some(control) = control {
        control.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_resume_and_cancel_transitions() {
        let control = DownloadControl::new();

        assert_eq!(control.state(), ControlState::Running);
        control.pause();
        assert_eq!(control.state(), ControlState::Paused);
        control.resume();
        assert_eq!(control.state(), ControlState::Running);
        control.cancel();
        assert_eq!(control.state(), ControlState::Cancelled);

        // Cancellation is terminal; pause/resume cannot revive the download.
        control.pause();
        control.resume();
        assert_eq!(control.state(), ControlState::Cancelled);
    }

    #[test]
    fn duplicate_downloads_are_rejected() {
        let id = "test:duplicate-download";
        let first = acquire(id);
        assert!(first.is_ok());
        assert!(acquire(id).is_err());

        drop(first);

        let second = acquire(id);
        assert!(second.is_ok());
        drop(second);
    }
}
