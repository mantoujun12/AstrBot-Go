//! Per-instance secrets for the desktop-managed WebUI session exchange.
//!
//! Upstream AstrBot (v4.27.3+, PR #9585) exposes
//! `POST /api/v1/auth/desktop-session` when launched with
//! `ASTRBOT_DESKTOP_MANAGED=1` and a per-instance
//! `ASTRBOT_DESKTOP_SESSION_SECRET`: the launcher exchanges that secret for a
//! dashboard JWT. Secrets stay in Rust memory only; they never cross into
//! the webview and are never logged.

use std::collections::HashMap;
use std::sync::Mutex;

use tauri::{AppHandle, Manager as _};

use crate::error::{AppError, Result};
use crate::utils::sync::lock_mutex_recover;

/// Set on every hosted instance launch; enables the desktop-session endpoint.
pub(crate) const DESKTOP_MANAGED_ENV: &str = "ASTRBOT_DESKTOP_MANAGED";
/// Per-instance secret (>= 32 chars) authenticating the launcher to the
/// desktop-session endpoint.
pub(crate) const DESKTOP_SESSION_SECRET_ENV: &str = "ASTRBOT_DESKTOP_SESSION_SECRET";
/// Header carrying the secret on the desktop-session exchange.
pub(crate) const DESKTOP_SESSION_HEADER: &str = "X-AstrBot-Desktop-Session";
/// Upstream exchange endpoint; only loopback clients are accepted.
pub(crate) const DESKTOP_SESSION_ENDPOINT: &str = "/api/v1/auth/desktop-session";

const SESSION_SECRET_LOCK_NAME: &str = "DesktopSessionState.secrets";

/// Per-instance session secrets, keyed by instance id. The same secret is
/// reused for the whole launcher lifetime so an auto-restarted instance keeps
/// accepting exchanges without a fresh launch.
#[derive(Default)]
pub struct DesktopSessionState {
    secrets: Mutex<HashMap<String, String>>,
}

impl DesktopSessionState {
    /// The stored secret, or `None` when this instance was not launched by
    /// the launcher in this run.
    pub(crate) fn stored(&self, instance_id: &str) -> Option<String> {
        lock_mutex_recover(&self.secrets, SESSION_SECRET_LOCK_NAME)
            .get(instance_id)
            .cloned()
    }

    /// The stored secret for one instance, generating and caching a fresh
    /// 64-hex-char secret (256 random bits) when none exists.
    fn secret_for(&self, instance_id: &str) -> Result<String> {
        let stored = lock_mutex_recover(&self.secrets, SESSION_SECRET_LOCK_NAME)
            .get(instance_id)
            .cloned();
        if let Some(secret) = stored {
            return Ok(secret);
        }
        let secret = generate_secret()?;
        lock_mutex_recover(&self.secrets, SESSION_SECRET_LOCK_NAME)
            .insert(instance_id.to_string(), secret.clone());
        Ok(secret)
    }

    /// Drops the secret of a deleted instance so the map does not grow for
    /// the manager lifetime.
    pub(crate) fn remove(&self, instance_id: &str) {
        lock_mutex_recover(&self.secrets, SESSION_SECRET_LOCK_NAME).remove(instance_id);
    }

    /// `secret_for` through the managed state, for launch-time injection.
    pub fn secret_for_launch(app_handle: &AppHandle, instance_id: &str) -> Result<String> {
        app_handle.state::<Self>().secret_for(instance_id)
    }
}

/// 256 random bits as 64 lowercase hex chars.
fn generate_secret() -> Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|error| AppError::other(format!("Failed to generate session secret: {error}")))?;
    let mut secret = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(secret, "{byte:02x}");
    }
    Ok(secret)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn generated_secret_is_64_lowercase_hex_chars() {
        let secret = generate_secret().expect("os entropy should be available");
        assert_eq!(secret.len(), 64);
        assert!(secret
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()));
    }

    #[test]
    fn secret_is_reused_for_the_same_instance() {
        let state = DesktopSessionState::default();
        let first = state.secret_for("inst-a").expect("secret generation");
        let second = state.secret_for("inst-a").expect("secret generation");
        assert_eq!(first, second);
        let other = state.secret_for("inst-b").expect("secret generation");
        assert_ne!(first, other);
    }

    #[test]
    fn remove_drops_only_the_target_instance() {
        let state = DesktopSessionState::default();
        let first = state.secret_for("inst-a").expect("secret generation");
        state.secret_for("inst-b").expect("secret generation");
        state.remove("inst-a");
        assert_eq!(state.stored("inst-a"), None);
        assert!(state.stored("inst-b").is_some());
        // A relaunched instance gets a fresh secret, not the removed one.
        assert_ne!(
            state.secret_for("inst-a").expect("secret generation"),
            first
        );
    }
}
