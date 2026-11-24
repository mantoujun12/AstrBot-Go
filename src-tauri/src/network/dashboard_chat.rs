//! AstrBot dashboard chat, authenticated by a dashboard login.
//!
//! All HTTP calls happen here in Rust: the webview is not exposed to CORS
//! and never sees the dashboard password or the JWTs used by the chat
//! proxy. The password is sent once to `POST /api/auth/login`; the returned
//! token is cached in `DashboardAuthState` and must never appear in logs or
//! error payloads.
//!
//! `get_webui_session` is the one exception: its JWT exists to be handed to
//! the embedded WebUI tab, which injects it into the hosted AstrBot SPA so
//! the user signs in without typing panel credentials. The desktop session
//! secret and the panel password still never reach the webview.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::ipc::Channel;
use tauri::State;

use crate::commands::AppState;
use crate::config::{load_config, load_manifest};
use crate::error::{AppError, ErrorKind, Result};
use crate::i18n::Locale;
use crate::network::desktop_session::{self, DesktopSessionState};
use crate::network_config;
use crate::process::InstanceRuntimeInfo;
use crate::utils::net::USER_AGENT;
use crate::utils::sync::lock_mutex_recover;
use crate::utils::validation::validate_instance_id;

/// Lists and logins must not hang on a stalled response; chat streaming is
/// separate and deliberately has no total timeout.
const LIST_TIMEOUT: Duration = Duration::from_secs(30);
/// shortcut: media crosses IPC base64-encoded, so cap it at 32 MiB; stream
/// larger files through a local protocol if chat ever needs them.
const MAX_MEDIA_BYTES: usize = 32 << 20;

const TOKEN_LOCK_NAME: &str = "DashboardAuthState.tokens";

/// Methods the proxy may use on chat routes.
const CHAT_METHODS: [&str; 4] = ["GET", "POST", "PATCH", "DELETE"];
/// Chat-supporting dashboard routes that the proxy may only read.
const READ_ONLY_ROUTES: [&str; 6] = [
    "/api/v1/providers",
    "/api/v1/configs",
    "/api/v1/personas",
    "/api/v1/commands",
    "/api/v1/config-profiles",
    "/api/v1/config-routes",
];

/// Cached dashboard JWTs keyed by instance id. A missing entry means "log in
/// on the next request"; a 401 drops it so the next attempt re-authenticates.
#[derive(Default)]
pub struct DashboardAuthState {
    tokens: Mutex<HashMap<String, String>>,
}

impl DashboardAuthState {
    fn cached(&self, instance_id: &str) -> Option<String> {
        lock_mutex_recover(&self.tokens, TOKEN_LOCK_NAME)
            .get(instance_id)
            .cloned()
    }

    fn store(&self, instance_id: &str, token: &str) {
        lock_mutex_recover(&self.tokens, TOKEN_LOCK_NAME)
            .insert(instance_id.to_string(), token.to_string());
    }

    fn clear(&self, instance_id: &str) {
        lock_mutex_recover(&self.tokens, TOKEN_LOCK_NAME).remove(instance_id);
    }
}

/// Stream event forwarded to the webview while a chat message is processed.
/// Field names and the `type` tag are the pinned frontend contract.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatStreamEvent {
    Session {
        session_id: String,
    },
    /// `chain` is set for reasoning/tool-call output: `reasoning`,
    /// `tool_call` or `tool_call_result`; `None` is normal reply text.
    Delta {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        chain: Option<String>,
    },
    /// Token/timing stats for the reply; the ChatUI's usage ring reads these.
    AgentStats {
        data: Value,
    },
    /// Final reply snapshot; the ChatUI fills in any delta the stream dropped.
    Complete {
        data: Value,
    },
    Done,
    Error {
        message: String,
    },
}

/// Lenient mirror of AstrBot's `{"status": ..., "message": ..., "data": ...}`
/// response envelope; unknown fields are ignored.
#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    #[serde(default)]
    status: String,
    message: Option<String>,
    data: Option<T>,
}

#[derive(Debug, Deserialize)]
struct RawSession {
    #[serde(default)]
    session_id: String,
}

/// Session payload of the desktop-session response; unknown fields are
/// ignored.
#[derive(Debug, Deserialize)]
struct RawDesktopSession {
    #[serde(default)]
    token: String,
    #[serde(default)]
    username: Option<String>,
}

/// One SSE frame from the AstrBot chat stream; unknown fields are ignored.
#[derive(Debug, Deserialize)]
struct SseEvent {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    data: serde_json::Value,
    #[serde(default)]
    chain_type: Option<String>,
}

/// Cap on one buffered event. A stalled or malicious endpoint that streams
/// without ever sending a separator must not grow the buffer without bound.
const MAX_EVENT_BYTES: usize = 1 << 20;

/// Incremental `text/event-stream` parser. Works on bytes so a UTF-8 sequence
/// split across chunks is not corrupted.
#[derive(Default)]
struct SseParser {
    buffer: Vec<u8>,
    /// Set once the buffer exceeded `MAX_EVENT_BYTES`; the buffer is dropped
    /// and the caller aborts the stream.
    overflowed: bool,
}

/// Index and length of the first event separator, accepting CRLF frames.
fn find_separator(buffer: &[u8]) -> Option<(usize, usize)> {
    (0..buffer.len()).find_map(|index| {
        if buffer[index..].starts_with(b"\r\n\r\n") {
            Some((index, 4))
        } else if buffer[index..].starts_with(b"\n\n") {
            Some((index, 2))
        } else {
            None
        }
    })
}

impl SseParser {
    /// Feed one chunk and return the `data:` payloads of all complete events.
    fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(chunk);
        let mut payloads = Vec::new();

        while let Some((index, separator_len)) = find_separator(&self.buffer) {
            let event: Vec<u8> = self.buffer.drain(..index + separator_len).collect();
            let mut data: Vec<u8> = Vec::new();
            for line in event[..index].split(|byte| *byte == b'\n') {
                // Tolerate a lone CRLF line ending inside the frame.
                let line = line.strip_suffix(b"\r").unwrap_or(line);
                // Comment lines such as ": heartbeat" keep the stream alive.
                if line.first() == Some(&b':') {
                    continue;
                }
                if let Some(rest) = line.strip_prefix(b"data:") {
                    if !data.is_empty() {
                        data.push(b'\n');
                    }
                    data.extend_from_slice(rest.strip_prefix(b" ").unwrap_or(rest));
                }
            }
            if !data.is_empty() {
                payloads.push(String::from_utf8_lossy(&data).into_owned());
            }
        }

        if self.buffer.len() > MAX_EVENT_BYTES {
            self.buffer.clear();
            self.overflowed = true;
        }

        payloads
    }
}

/// Map one SSE `data:` payload onto the pinned stream event. Unknown event
/// kinds are not forwarded; chain output (reasoning/tool calls) is forwarded
/// as deltas tagged with its chain type so the ChatUI can render it.
fn map_sse_data(raw: &str) -> Option<ChatStreamEvent> {
    let event: SseEvent = serde_json::from_str(raw).ok()?;
    match event.kind.as_str() {
        "session_id" => event
            .session_id
            .map(|session_id| ChatStreamEvent::Session { session_id }),
        "plain" => {
            let text = event.data.as_str()?;
            Some(ChatStreamEvent::Delta {
                text: text.to_string(),
                chain: event.chain_type,
            })
        }
        // Forwarded verbatim: the ChatUI already knows these payload shapes.
        "agent_stats" => Some(ChatStreamEvent::AgentStats { data: event.data }),
        "complete" => Some(ChatStreamEvent::Complete { data: event.data }),
        "error" => Some(ChatStreamEvent::Error {
            message: extract_error_message(&event.data).unwrap_or_default(),
        }),
        "end" => Some(ChatStreamEvent::Done),
        _ => None,
    }
}

fn extract_error_message(data: &serde_json::Value) -> Option<String> {
    data.get("message")
        .and_then(serde_json::Value::as_str)
        .map(String::from)
        .or_else(|| data.as_str().map(String::from))
}

/// Host forms that cannot be dialed directly; mirror of the normalization in
/// `src/main/views/InstanceWebuiView.vue`.
pub(crate) fn normalize_dashboard_host(host: &str) -> String {
    let value = host.trim();
    if value.is_empty()
        || value == "0.0.0.0"
        || value == "::"
        || value == "[::]"
        || value.eq_ignore_ascii_case("localhost")
    {
        return "127.0.0.1".to_string();
    }
    if value.contains(':') && !(value.starts_with('[') && value.ends_with(']')) {
        return format!("[{value}]");
    }
    value.to_string()
}

enum StatusFailure {
    Unauthorized,
    Forbidden,
    NotFound,
    Other(u16),
}

fn classify_status(status: reqwest::StatusCode) -> Option<StatusFailure> {
    if status.is_success() {
        return None;
    }
    match status.as_u16() {
        401 => Some(StatusFailure::Unauthorized),
        403 => Some(StatusFailure::Forbidden),
        404 => Some(StatusFailure::NotFound),
        other => Some(StatusFailure::Other(other)),
    }
}

fn status_error_message(failure: StatusFailure, locale: Locale) -> String {
    match failure {
        StatusFailure::Unauthorized => locale.dashboard_login_failed(),
        StatusFailure::Forbidden => locale.dashboard_forbidden(),
        StatusFailure::NotFound => locale.dashboard_unsupported_version(),
        StatusFailure::Other(code) => format!("HTTP {code}: {}", locale.dashboard_stream_error()),
    }
}

fn current_locale() -> Locale {
    load_config()
        .map(|config| Locale::resolve(config.language_preference))
        .unwrap_or_default()
}

/// Dashboard login endpoints and stored credentials for one instance.
struct InstanceEndpoint {
    host: String,
    port: u16,
    username: String,
    password: String,
}

/// Host, dial port, and stored dashboard credentials. The live port is
/// preferred when the instance is running, mirroring the WebUI view.
fn instance_endpoint(state: &AppState, instance_id: &str) -> Result<InstanceEndpoint> {
    let manifest = load_manifest()?;
    let instance = manifest
        .instances
        .get(instance_id)
        .ok_or_else(|| AppError::instance_not_found(instance_id))?;
    let host = normalize_dashboard_host(&instance.host);
    let port = match state.process_manager.get_runtime_info().get(instance_id) {
        Some(InstanceRuntimeInfo::Live { port, .. }) => *port,
        _ => instance.port,
    };
    Ok(InstanceEndpoint {
        host,
        port,
        username: instance.dashboard_username.clone(),
        password: instance.dashboard_password.clone(),
    })
}

/// Whether the host is the local machine; only then is cleartext HTTP safe.
fn is_loopback_host(host: &str) -> bool {
    let host = host.trim().trim_start_matches('[').trim_end_matches(']');
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

/// Base URL for an instance. Loopback instances keep `http://`; any remote
/// host gets TLS so the JWT never travels in cleartext.
fn dashboard_base_url(host: &str, port: u16) -> String {
    let host = normalize_dashboard_host(host);
    let scheme = if is_loopback_host(&host) {
        "http"
    } else {
        "https"
    };
    format!("{scheme}://{host}:{port}")
}

/// Client without a total timeout: chat streams stay idle for minutes while
/// the agent thinks, so only connection setup is limited. Loopback targets
/// bypass any configured proxy so it cannot capture the login/JWT.
fn build_instance_client(host: &str) -> Result<reqwest::Client> {
    if !is_loopback_host(host) {
        let config = load_config()?;
        return network_config::build_download_client_from_config(config.as_ref());
    }

    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .no_proxy()
        .build()
        .map_err(|error| {
            AppError::network(format!(
                "创建网络客户端失败：{}",
                crate::utils::log_redact::redact_text(&error.to_string())
            ))
        })
}

fn transport_error(error: reqwest::Error, locale: Locale) -> AppError {
    if error.is_connect() {
        return AppError::network(locale.dashboard_unreachable());
    }
    log::warn!(
        "AstrBot dashboard request failed: {}",
        crate::utils::log_redact::redact_text(&error.to_string())
    );
    AppError::network(locale.dashboard_stream_error())
}

/// Token from a login response body, or the actionable reason it is missing.
/// A rejected login still carries the server message in the same envelope, so
/// the HTTP status is not needed here.
fn parse_login_response(body: &str, locale: Locale) -> Result<String> {
    let value: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let data = value.get("data");

    if data
        .and_then(|data| data.get("totp_required"))
        .and_then(Value::as_bool)
        == Some(true)
    {
        return Err(AppError::network(locale.dashboard_mfa_unsupported()));
    }

    if let Some(token) = data
        .and_then(|data| data.get("token"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        return Ok(token.to_string());
    }

    match value
        .get("message")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|message| !message.is_empty())
    {
        Some(message) => Err(AppError::network(message.to_string())),
        None => Err(AppError::network(locale.dashboard_login_failed())),
    }
}

/// Exchange the stored credentials for a dashboard JWT.
async fn login(endpoint: &InstanceEndpoint, locale: Locale) -> Result<String> {
    let username = endpoint.username.trim();
    if username.is_empty() || endpoint.password.is_empty() {
        return Err(AppError::network(locale.dashboard_credentials_missing()));
    }
    let client = build_instance_client(&endpoint.host)?;
    let resp = client
        .post(format!(
            "{}/api/auth/login",
            dashboard_base_url(&endpoint.host, endpoint.port)
        ))
        .header("User-Agent", USER_AGENT)
        .timeout(LIST_TIMEOUT)
        .json(&serde_json::json!({
            "username": username,
            "password": endpoint.password,
        }))
        .send()
        .await
        .map_err(|error| transport_error(error, locale))?;

    let status = resp.status();
    // The body is never logged: a successful login carries the JWT.
    let body = resp
        .text()
        .await
        .map_err(|_| AppError::network(locale.dashboard_login_failed()))?;
    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::network(locale.dashboard_unsupported_version()));
    }
    parse_login_response(&body, locale)
}

async fn token_for(
    auth: &DashboardAuthState,
    instance_id: &str,
    endpoint: &InstanceEndpoint,
    locale: Locale,
) -> Result<String> {
    if let Some(token) = auth.cached(instance_id) {
        return Ok(token);
    }
    let token = login(endpoint, locale).await?;
    auth.store(instance_id, &token);
    Ok(token)
}

/// Send one authenticated request. A 401 means the cached JWT expired, so the
/// cache is dropped, the credentials are exchanged again, and the request is
/// retried exactly once; a second 401 is reported instead of looping.
async fn authorized_send<F>(
    auth: &DashboardAuthState,
    instance_id: &str,
    endpoint: &InstanceEndpoint,
    locale: Locale,
    build: F,
) -> Result<reqwest::Response>
where
    F: Fn(&str) -> reqwest::RequestBuilder,
{
    let token = token_for(auth, instance_id, endpoint, locale).await?;
    let first = build(&token)
        .send()
        .await
        .map_err(|error| transport_error(error, locale))?;
    if first.status() != reqwest::StatusCode::UNAUTHORIZED {
        return Ok(first);
    }

    auth.clear(instance_id);
    let token = login(endpoint, locale).await?;
    auth.store(instance_id, &token);
    let retried = build(&token)
        .send()
        .await
        .map_err(|error| transport_error(error, locale))?;
    if retried.status() == reqwest::StatusCode::UNAUTHORIZED {
        auth.clear(instance_id);
        return Err(AppError::network(locale.dashboard_login_failed()));
    }
    Ok(retried)
}

/// AstrBot reports request failures inside a 200 envelope.
fn envelope_data<T: serde::de::DeserializeOwned>(
    envelope: ApiEnvelope<T>,
    locale: Locale,
) -> Result<T> {
    if envelope.status == "error" {
        return Err(AppError::network(
            envelope
                .message
                .filter(|message| !message.trim().is_empty())
                .unwrap_or_else(|| locale.dashboard_stream_error()),
        ));
    }
    envelope
        .data
        .ok_or_else(|| AppError::network(locale.dashboard_stream_error()))
}

fn error_envelope_message(text: &str, locale: Locale) -> String {
    let envelope: ApiEnvelope<Value> = serde_json::from_str(text).unwrap_or(ApiEnvelope {
        status: String::new(),
        message: None,
        data: None,
    });
    if envelope.status == "error" {
        envelope
            .message
            .filter(|message| !message.trim().is_empty())
            .unwrap_or_else(|| locale.dashboard_stream_error())
    } else {
        locale.dashboard_stream_error()
    }
}

/// How the embedded WebUI tab obtains its session: the desktop exchange for
/// launcher-managed instances, stored credentials as the fallback, and
/// manual sign-in in the SPA when neither is possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionSource {
    Desktop,
    Credentials,
    Manual,
}

/// Whether stored dashboard credentials can be exchanged for a session.
/// Mirrors the emptiness check inside `login`.
fn credentials_available(username: &str, password: &str) -> bool {
    !username.trim().is_empty() && !password.is_empty()
}

/// The source to try for one instance before any HTTP call is made.
fn resolve_session_source(has_secret: bool, endpoint: &InstanceEndpoint) -> SessionSource {
    if has_secret {
        SessionSource::Desktop
    } else if credentials_available(&endpoint.username, &endpoint.password) {
        SessionSource::Credentials
    } else {
        SessionSource::Manual
    }
}

/// The expected-state error that keeps the SPA's manual sign-in form as the
/// fallback. The `reason` marker lets the WebUI view stay silent instead of
/// toasting a state the iframe already renders on its own.
fn manual_login_required(locale: Locale) -> AppError {
    AppError::new(
        ErrorKind::Network,
        HashMap::from([
            ("detail".to_string(), locale.dashboard_webui_manual_login()),
            ("reason".to_string(), "webui_manual_login".to_string()),
        ]),
    )
}

/// Token and username from a desktop-session response body, or the reason
/// they are missing. The body is never logged: it carries the JWT.
fn parse_desktop_session_response(body: &str, locale: Locale) -> Result<(String, String)> {
    let envelope: ApiEnvelope<RawDesktopSession> = serde_json::from_str(body)
        .map_err(|_| AppError::network(locale.dashboard_stream_error()))?;
    let session = envelope_data(envelope, locale)?;
    if session.token.trim().is_empty()
        || session.username.as_deref().unwrap_or("").trim().is_empty()
    {
        return Err(AppError::network(locale.dashboard_stream_error()));
    }
    Ok((session.token, session.username.unwrap_or_default()))
}

/// Loopback-only local exchange; a few seconds is generous.
const DESKTOP_SESSION_TIMEOUT: Duration = Duration::from_secs(5);

/// Exchange the launch secret for a dashboard JWT via the upstream
/// desktop-session endpoint (v4.27.3+). Any failure — unsupported version,
/// instance restarted outside the launcher, transient error — is the
/// caller's cue to fall back to stored credentials.
async fn desktop_session_exchange(
    endpoint: &InstanceEndpoint,
    secret: &str,
    locale: Locale,
) -> Result<(String, String)> {
    let client = build_instance_client(&endpoint.host)?;
    let resp = client
        .post(format!(
            "{}{}",
            dashboard_base_url(&endpoint.host, endpoint.port),
            desktop_session::DESKTOP_SESSION_ENDPOINT
        ))
        .header("User-Agent", USER_AGENT)
        .header(desktop_session::DESKTOP_SESSION_HEADER, secret)
        .timeout(DESKTOP_SESSION_TIMEOUT)
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|error| transport_error(error, locale))?;
    // The body is never logged: it carries the JWT.
    let body = resp
        .text()
        .await
        .map_err(|_| AppError::network(locale.dashboard_stream_error()))?;
    parse_desktop_session_response(&body, locale)
}

/// A dashboard session issued for the embedded WebUI tab, so the user never
/// types panel credentials into the iframe.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebuiSession {
    pub token: String,
    pub username: String,
    /// `"desktop"` (secret exchange) or `"credentials"` (stored login).
    pub source: String,
}

const SOURCE_DESKTOP: &str = "desktop";
const SOURCE_CREDENTIALS: &str = "credentials";

/// Session for the embedded WebUI tab of a hosted instance. Prefers the
/// desktop-managed exchange; older AstrBot falls back to the stored panel
/// credentials, and the SPA keeps its own sign-in form when neither works.
/// The token and secret never appear in logs or error payloads.
#[tauri::command]
pub async fn get_webui_session(
    state: State<'_, AppState>,
    sessions: State<'_, DesktopSessionState>,
    instance_id: String,
) -> Result<WebuiSession> {
    let locale = current_locale();
    let endpoint = instance_endpoint(&state, &instance_id)?;
    // The iframe only loads for a live instance; a stopped instance has no
    // endpoint to exchange with.
    if !matches!(
        state.process_manager.get_runtime_info().get(&instance_id),
        Some(InstanceRuntimeInfo::Live { .. })
    ) {
        return Err(AppError::instance_not_running());
    }

    let secret = sessions.stored(&instance_id);
    if resolve_session_source(secret.is_some(), &endpoint) == SessionSource::Manual {
        return Err(manual_login_required(locale));
    }

    // Preferred: the desktop-managed exchange. A failure is not surfaced —
    // the credential fallback below is the actionable path.
    if let Some(secret) = secret.as_deref() {
        if let Ok((token, username)) = desktop_session_exchange(&endpoint, secret, locale).await {
            return Ok(WebuiSession {
                token,
                username,
                source: SOURCE_DESKTOP.to_string(),
            });
        }
    }

    // Fallback for AstrBot < v4.27.3: the stored panel credentials.
    if !credentials_available(&endpoint.username, &endpoint.password) {
        return Err(manual_login_required(locale));
    }
    let username = endpoint.username.trim().to_string();
    let token = login(&endpoint, locale).await?;
    Ok(WebuiSession {
        token,
        username,
        source: SOURCE_CREDENTIALS.to_string(),
    })
}

/// Applies the per-request model selection; AstrBot resolves provider and
/// model together, exactly like its own WebUI selector.
fn apply_model_selection(body: &mut Value, provider: Option<&str>, model: Option<&str>) {
    if let Some(provider) = provider {
        body["selected_provider"] = serde_json::json!(provider);
    }
    if let Some(model) = model {
        body["selected_model"] = serde_json::json!(model);
    }
}

/// Message payload for `POST /api/v1/chat`; the server derives the username
/// from the JWT, so the body must carry the session and model selection only.
fn chat_payload(
    session_id: &str,
    message: &str,
    provider: Option<&str>,
    model: Option<&str>,
) -> Value {
    let mut body = serde_json::json!({
        "session_id": session_id,
        "message": message,
        "enable_streaming": true,
    });
    apply_model_selection(&mut body, provider, model);
    body
}

/// Payload for the regenerate endpoint; the server rewinds the session to the
/// target message and streams the replacement reply, so only the model
/// selection travels.
fn regenerate_payload(provider: Option<&str>, model: Option<&str>) -> Value {
    let mut body = serde_json::json!({ "enable_streaming": true });
    apply_model_selection(&mut body, provider, model);
    body
}

/// True for dot segments in both literal and percent-encoded form.
///
/// URL parsers normalize `%2e` to `.` before resolving the path, so
/// `%2e%2e` is equivalent to `..` and would escape an allowlisted prefix.
fn is_dot_segment(segment: &str) -> bool {
    let lower = segment.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "." | ".." | "%2e" | "%2e%2e" | ".%2e" | "%2e."
    )
}

/// Percent-encode one path segment (RFC 3986 unreserved characters stay
/// plain) so a session or message id cannot escape the chat route prefix.
fn encode_path_segment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

/// `POST /api/v1/chat/sessions/{id}/messages/{id}/regenerate`, the upstream
/// regenerate SSE endpoint.
fn regenerate_url(host: &str, port: u16, session_id: &str, message_id: &str) -> String {
    format!(
        "{}/api/v1/chat/sessions/{}/messages/{}/regenerate",
        dashboard_base_url(host, port),
        encode_path_segment(session_id),
        encode_path_segment(message_id),
    )
}

/// What `chat_send_message` should do with the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatSendMode {
    /// Send a new message, creating a session when none was given.
    Send,
    /// Re-run the turn of one history message inside an existing session.
    Regenerate,
}

impl ChatSendMode {
    /// `None` and blank mean the default `send` mode; unknown values are
    /// rejected so the pinned contract stays explicit.
    fn parse(mode: Option<&str>) -> Option<Self> {
        match mode.map(str::trim) {
            None | Some("") | Some("send") => Some(Self::Send),
            Some("regenerate") => Some(Self::Regenerate),
            Some(_) => None,
        }
    }
}

/// The WebUI always chats inside an existing session, so a new conversation is
/// created first and its id becomes the session id of the streamed exchange.
async fn create_session(
    auth: &DashboardAuthState,
    instance_id: &str,
    endpoint: &InstanceEndpoint,
    client: &reqwest::Client,
    locale: Locale,
) -> Result<String> {
    let url = format!(
        "{}/api/v1/chat/sessions/new?platform_id=webchat",
        dashboard_base_url(&endpoint.host, endpoint.port)
    );
    let resp = authorized_send(auth, instance_id, endpoint, locale, |token| {
        client
            .get(&url)
            .bearer_auth(token)
            .header("User-Agent", USER_AGENT)
            .timeout(LIST_TIMEOUT)
    })
    .await?;

    if let Some(failure) = classify_status(resp.status()) {
        return Err(AppError::network(status_error_message(failure, locale)));
    }
    let envelope: ApiEnvelope<RawSession> = resp
        .json()
        .await
        .map_err(|_| AppError::network(locale.dashboard_stream_error()))?;
    let session_id = envelope_data(envelope, locale)?
        .session_id
        .trim()
        .to_string();
    if session_id.is_empty() {
        return Err(AppError::network(locale.dashboard_stream_error()));
    }
    Ok(session_id)
}

/// Verify one proxy target against the allowlist. Chat routes may write;
/// everything else the chat UI needs is read-only, so the webview cannot drive
/// unrelated dashboard features (plugins, backups, system updates) through the
/// proxy.
fn validate_proxy_target(method: &str, path: &str, locale: Locale) -> Result<()> {
    let without_query = path.split('?').next().unwrap_or("");
    // Fragments are stripped by reqwest before forwarding; strip them here too
    // so that `%2e%2e#fragment` is not seen as a single non-dot segment.
    let path_only = without_query.split('#').next().unwrap_or("");
    let rejected = || {
        log::warn!("Rejected dashboard proxy request: {method} {path_only}");
        AppError::network(locale.dashboard_forbidden())
    };

    if !path.starts_with('/') || path.starts_with("//") {
        return Err(rejected());
    }
    // Backslashes are treated as path separators by some HTTP clients and
    // URL parsers; reject them before the slash-split segment walk so that
    // `..\..\plugins` cannot escape the allowlisted prefix.
    if path_only.contains('\\') {
        return Err(rejected());
    }
    // Encoded slashes (%2f) can be decoded by upstream servers before route
    // resolution, injecting extra path separators that bypass segment checks.
    if path_only.to_ascii_lowercase().contains("%2f") {
        return Err(rejected());
    }
    // Dot segments — literal or percent-encoded — are normalized away by URL
    // parsers and would escape the allowlisted prefix.
    if path_only.split('/').any(is_dot_segment) {
        return Err(rejected());
    }

    if path_only.starts_with("/api/v1/chat/") {
        return if CHAT_METHODS.contains(&method) {
            Ok(())
        } else {
            Err(rejected())
        };
    }

    let read_only = READ_ONLY_ROUTES
        .iter()
        .any(|route| path_only == *route || path_only.starts_with(&format!("{route}/")));
    if read_only && method == "GET" {
        return Ok(());
    }
    Err(rejected())
}

/// Media reads are GET-only and limited to the two dashboard shapes: by
/// attachment id (`/api/v1/files/{id}/content`) and by stored filename
/// (`/api/v1/files/content?filename=...`).
fn validate_media_path(path: &str, locale: Locale) -> Result<()> {
    let (before_query, query) = path.split_once('?').unwrap_or((path, ""));
    // Fragments are stripped by reqwest before forwarding; strip them here too
    // so that `%2e%2e#fragment` is not seen as a single non-dot segment.
    let path_only = before_query.split('#').next().unwrap_or("");
    let rejected = || {
        log::warn!("Rejected chat media request: {path_only}");
        AppError::network(locale.dashboard_forbidden())
    };

    if !path.starts_with('/') || path.starts_with("//") {
        return Err(rejected());
    }
    // Backslashes are treated as path separators by some HTTP clients and
    // URL parsers; reject them before the slash-split segment walk so that
    // backslash-encoded traversals cannot escape the allowlisted prefixes.
    if path_only.contains('\\') {
        return Err(rejected());
    }
    // Encoded slashes (%2f) can be decoded by upstream servers before route
    // resolution, injecting extra path separators that bypass segment checks.
    if path_only.to_ascii_lowercase().contains("%2f") {
        return Err(rejected());
    }
    if path_only.split('/').any(is_dot_segment) {
        return Err(rejected());
    }

    if path_only == "/api/v1/files/content" {
        let has_filename = query.split('&').any(|pair| {
            pair.strip_prefix("filename=")
                .is_some_and(|value| !value.is_empty())
        });
        return if has_filename {
            Ok(())
        } else {
            Err(rejected())
        };
    }

    let segments: Vec<&str> = path_only.split('/').collect();
    if segments.len() == 6
        && segments[1..4] == ["api", "v1", "files"]
        && !segments[4].is_empty()
        && segments[5] == "content"
    {
        return Ok(());
    }
    Err(rejected())
}

/// Base64 for the IPC payload. Kept local so media does not pull in a crate
/// for one encode path; media is capped well below anything streaming needs.
fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        encoded.push(ALPHABET[(first >> 2) as usize] as char);
        encoded.push(ALPHABET[(((first & 0b11) << 4) | (second >> 4)) as usize] as char);
        if chunk.len() > 1 {
            encoded.push(ALPHABET[(((second & 0b1111) << 2) | (third >> 6)) as usize] as char);
        } else {
            encoded.push('=');
        }
        if chunk.len() > 2 {
            encoded.push(ALPHABET[(third & 0b111111) as usize] as char);
        } else {
            encoded.push('=');
        }
    }
    encoded
}

/// Authenticated proxy for the dashboard routes the chat UI reads and writes.
/// The raw dashboard envelope is returned so the frontend can consume it
/// as-is; transport, status, and post-retry auth failures become errors.
#[tauri::command]
pub async fn dashboard_api_request(
    state: State<'_, AppState>,
    auth: State<'_, DashboardAuthState>,
    instance_id: String,
    method: String,
    path: String,
    body: Option<Value>,
) -> Result<Value> {
    validate_instance_id(&instance_id)?;
    let method = method.trim().to_ascii_uppercase();
    let locale = current_locale();
    validate_proxy_target(&method, &path, locale)?;

    let endpoint = instance_endpoint(&state, &instance_id)?;
    let client = build_instance_client(&endpoint.host)?;
    let url = format!(
        "{}{path}",
        dashboard_base_url(&endpoint.host, endpoint.port)
    );
    let request_method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|_| AppError::network(locale.dashboard_forbidden()))?;

    let resp = authorized_send(&auth, &instance_id, &endpoint, locale, |token| {
        let request = client
            .request(request_method.clone(), &url)
            .bearer_auth(token)
            .header("User-Agent", USER_AGENT)
            .timeout(LIST_TIMEOUT);
        match &body {
            Some(body) if method != "GET" => request.json(body),
            _ => request,
        }
    })
    .await?;

    if let Some(failure) = classify_status(resp.status()) {
        return Err(AppError::network(status_error_message(failure, locale)));
    }
    if resp.status() == reqwest::StatusCode::NO_CONTENT {
        return Ok(Value::Null);
    }
    resp.json::<Value>()
        .await
        .map_err(|_| AppError::network(locale.dashboard_stream_error()))
}

/// One media file for the chat transcript. Base64 keeps the webview free of
/// dashboard URLs and JWTs; the island turns it into a blob URL.
#[derive(Debug, Serialize)]
pub struct ChatMediaPayload {
    mime: String,
    base64: String,
}

#[tauri::command]
pub async fn chat_fetch_media(
    state: State<'_, AppState>,
    auth: State<'_, DashboardAuthState>,
    instance_id: String,
    path: String,
) -> Result<ChatMediaPayload> {
    validate_instance_id(&instance_id)?;
    let locale = current_locale();
    validate_media_path(&path, locale)?;

    let endpoint = instance_endpoint(&state, &instance_id)?;
    let client = build_instance_client(&endpoint.host)?;
    let url = format!(
        "{}{path}",
        dashboard_base_url(&endpoint.host, endpoint.port)
    );
    let resp = authorized_send(&auth, &instance_id, &endpoint, locale, |token| {
        client
            .get(&url)
            .bearer_auth(token)
            .header("User-Agent", USER_AGENT)
            .timeout(LIST_TIMEOUT)
    })
    .await?;

    if let Some(failure) = classify_status(resp.status()) {
        return Err(AppError::network(status_error_message(failure, locale)));
    }
    if resp
        .content_length()
        .is_some_and(|length| length as usize > MAX_MEDIA_BYTES)
    {
        return Err(AppError::network(locale.dashboard_stream_error()));
    }
    let mime = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let bytes = resp
        .bytes()
        .await
        .map_err(|_| AppError::network(locale.dashboard_stream_error()))?;
    if bytes.len() > MAX_MEDIA_BYTES {
        return Err(AppError::network(locale.dashboard_stream_error()));
    }
    Ok(ChatMediaPayload {
        mime,
        base64: base64_encode(&bytes),
    })
}

/// Terminal event for a finished loop. A clean EOF (short replies such as a
/// command result) may close without the `end` frame, so completion is still
/// reported; a broken transport keeps reporting an error instead.
fn stream_end_event(done: bool, failed: bool, locale: Locale) -> Option<ChatStreamEvent> {
    if done {
        None
    } else if failed {
        Some(ChatStreamEvent::Error {
            message: locale.dashboard_unreachable(),
        })
    } else {
        Some(ChatStreamEvent::Done)
    }
}

fn error_text(error: AppError, locale: Locale) -> String {
    let detail = error.detail().trim();
    if detail.is_empty() {
        locale.dashboard_stream_error()
    } else {
        detail.to_string()
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn chat_send_message(
    state: State<'_, AppState>,
    auth: State<'_, DashboardAuthState>,
    instance_id: String,
    session_id: Option<String>,
    message: String,
    // AstrBot resolves a per-request model via provider + model, exactly like
    // its own WebUI selector; sending only the model can silently no-op.
    provider: Option<String>,
    model: Option<String>,
    // `send` (default) posts a new message; `regenerate` re-runs the turn of
    // `target_message_id` inside `session_id` without creating a session.
    mode: Option<String>,
    target_message_id: Option<String>,
    on_event: Channel<ChatStreamEvent>,
) -> Result<()> {
    validate_instance_id(&instance_id)?;
    let locale = current_locale();
    let mode = ChatSendMode::parse(mode.as_deref())
        .ok_or_else(|| AppError::network(locale.dashboard_forbidden()))?;
    // Blank strings from the webview are treated as "not provided".
    let session_id = session_id.filter(|value| !value.trim().is_empty());
    let provider = provider.filter(|value| !value.trim().is_empty());
    let model = model.filter(|value| !value.trim().is_empty());
    let target_message_id = target_message_id.filter(|value| !value.trim().is_empty());
    let endpoint = instance_endpoint(&state, &instance_id)?;
    let client = build_instance_client(&endpoint.host)?;

    // Regenerate always targets an existing session and history message.
    if mode == ChatSendMode::Regenerate && (session_id.is_none() || target_message_id.is_none()) {
        let _ = on_event.send(ChatStreamEvent::Error {
            message: locale.dashboard_stream_error(),
        });
        return Ok(());
    }

    let session_id = match session_id {
        Some(session_id) => session_id,
        None => match create_session(&auth, &instance_id, &endpoint, &client, locale).await {
            Ok(session_id) => session_id,
            Err(error) => {
                let _ = on_event.send(ChatStreamEvent::Error {
                    message: error_text(error, locale),
                });
                return Ok(());
            }
        },
    };
    // The session is known before streaming starts; the server echoes the same
    // id as its first SSE frame, which is not forwarded again.
    if on_event
        .send(ChatStreamEvent::Session {
            session_id: session_id.clone(),
        })
        .is_err()
    {
        return Ok(());
    }

    let (url, body) = match mode {
        ChatSendMode::Send => (
            format!(
                "{}/api/v1/chat",
                dashboard_base_url(&endpoint.host, endpoint.port)
            ),
            chat_payload(&session_id, &message, provider.as_deref(), model.as_deref()),
        ),
        ChatSendMode::Regenerate => (
            regenerate_url(
                &endpoint.host,
                endpoint.port,
                &session_id,
                target_message_id.as_deref().unwrap_or_default(),
            ),
            regenerate_payload(provider.as_deref(), model.as_deref()),
        ),
    };
    let send_result = authorized_send(&auth, &instance_id, &endpoint, locale, |token| {
        client
            .post(&url)
            .bearer_auth(token)
            .header("User-Agent", USER_AGENT)
            .json(&body)
    })
    .await;

    let resp = match send_result {
        Ok(resp) => resp,
        Err(error) => {
            let _ = on_event.send(ChatStreamEvent::Error {
                message: error_text(error, locale),
            });
            return Ok(());
        }
    };

    if let Some(failure) = classify_status(resp.status()) {
        let _ = on_event.send(ChatStreamEvent::Error {
            message: status_error_message(failure, locale),
        });
        return Ok(());
    }

    // Error envelopes come back as JSON with a success status instead of a
    // stream; anything else is treated as the event stream itself.
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if content_type.contains("json") {
        let text = resp.text().await.unwrap_or_default();
        let _ = on_event.send(ChatStreamEvent::Error {
            message: error_envelope_message(&text, locale),
        });
        return Ok(());
    }

    let mut parser = SseParser::default();
    let mut finished = false;
    let mut stream_failed = false;
    let mut stream = resp;
    loop {
        let chunk = match stream.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(error) => {
                log::warn!(
                    "AstrBot dashboard chat stream aborted: {}",
                    crate::utils::log_redact::redact_text(&error.to_string())
                );
                stream_failed = true;
                break;
            }
        };

        for data in parser.push(&chunk) {
            let Some(event) = map_sse_data(&data) else {
                continue;
            };
            match event {
                // The session id was already announced before the stream.
                ChatStreamEvent::Session { .. } => {}
                ChatStreamEvent::Delta { text, chain } => {
                    if on_event
                        .send(ChatStreamEvent::Delta { text, chain })
                        .is_err()
                    {
                        return Ok(());
                    }
                }
                ChatStreamEvent::AgentStats { data } => {
                    if on_event.send(ChatStreamEvent::AgentStats { data }).is_err() {
                        return Ok(());
                    }
                }
                ChatStreamEvent::Complete { data } => {
                    if on_event.send(ChatStreamEvent::Complete { data }).is_err() {
                        return Ok(());
                    }
                }
                ChatStreamEvent::Error { message } => {
                    let message = if message.is_empty() {
                        locale.dashboard_stream_error()
                    } else {
                        message
                    };
                    if on_event.send(ChatStreamEvent::Error { message }).is_err() {
                        return Ok(());
                    }
                }
                ChatStreamEvent::Done => {
                    if !finished {
                        let _ = on_event.send(ChatStreamEvent::Done);
                        finished = true;
                    }
                }
            }
        }

        if parser.overflowed {
            let _ = on_event.send(ChatStreamEvent::Error {
                message: locale.dashboard_stream_error(),
            });
            return Ok(());
        }

        if finished {
            break;
        }
    }

    if let Some(event) = stream_end_event(finished, stream_failed, locale) {
        let _ = on_event.send(event);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locale_messages(locale: Locale) -> Vec<String> {
        vec![
            locale.dashboard_credentials_missing(),
            locale.dashboard_login_failed(),
            locale.dashboard_mfa_unsupported(),
            locale.dashboard_unreachable(),
            locale.dashboard_unsupported_version(),
            locale.dashboard_forbidden(),
            locale.dashboard_stream_error(),
            locale.dashboard_webui_manual_login(),
        ]
    }

    #[test]
    fn dashboard_locale_messages_exist_in_every_language() {
        for locale in [Locale::ZhCn, Locale::EnUs, Locale::RuRu, Locale::JaJp] {
            let messages = locale_messages(locale);
            for message in &messages {
                assert!(!message.trim().is_empty(), "locale {locale:?}");
            }
            let unique: std::collections::HashSet<_> = messages.iter().collect();
            assert_eq!(unique.len(), messages.len(), "locale {locale:?}");
        }
    }

    #[test]
    fn chat_stream_events_serialize_the_pinned_shape() {
        assert_eq!(
            serde_json::to_string(&ChatStreamEvent::Session {
                session_id: "abc".to_string()
            })
            .unwrap_or_default(),
            r#"{"type":"session","session_id":"abc"}"#
        );
        assert_eq!(
            serde_json::to_string(&ChatStreamEvent::Delta {
                text: "你好".to_string(),
                chain: None
            })
            .unwrap_or_default(),
            r#"{"type":"delta","text":"你好"}"#
        );
        assert_eq!(
            serde_json::to_string(&ChatStreamEvent::Delta {
                text: "思考".to_string(),
                chain: Some("reasoning".to_string())
            })
            .unwrap_or_default(),
            r#"{"type":"delta","text":"思考","chain":"reasoning"}"#
        );
        assert_eq!(
            serde_json::to_string(&ChatStreamEvent::Done).unwrap_or_default(),
            r#"{"type":"done"}"#
        );
        assert_eq!(
            serde_json::to_string(&ChatStreamEvent::Error {
                message: "boom".to_string()
            })
            .unwrap_or_default(),
            r#"{"type":"error","message":"boom"}"#
        );
        assert_eq!(
            serde_json::to_string(&ChatStreamEvent::AgentStats {
                data: serde_json::json!({"total_tokens": 12})
            })
            .unwrap_or_default(),
            r#"{"type":"agent_stats","data":{"total_tokens":12}}"#
        );
        assert_eq!(
            serde_json::to_string(&ChatStreamEvent::Complete {
                data: serde_json::json!("final")
            })
            .unwrap_or_default(),
            r#"{"type":"complete","data":"final"}"#
        );
    }

    #[test]
    fn map_sse_data_forwards_stats_and_complete_verbatim() {
        assert_eq!(
            map_sse_data(r#"{"type":"agent_stats","data":{"total_tokens":12}}"#),
            Some(ChatStreamEvent::AgentStats {
                data: serde_json::json!({"total_tokens": 12})
            })
        );
        assert_eq!(
            map_sse_data(r#"{"type":"complete","data":"final text"}"#),
            Some(ChatStreamEvent::Complete {
                data: serde_json::json!("final text")
            })
        );
        // Unknown event kinds stay filtered out, so the contract stays pinned.
        assert_eq!(map_sse_data(r#"{"type":"run_snapshot","data":{}}"#), None);
    }

    #[test]
    fn sse_parser_handles_canned_stream_and_chunk_boundaries() {
        let canned = concat!(
            r#"data: {"type":"session_id","data":null,"session_id":"s-1"}"#,
            "\n\n",
            ": heartbeat\n\n",
            r#"data: {"type":"plain","data":"你","streaming":true}"#,
            "\n\n",
            r#"data: {"type":"plain","data":"思考","chain_type":"reasoning"}"#,
            "\n\n",
            r#"data: {"type":"message_saved","data":{"id":1}}"#,
            "\n\n",
            r#"data: {"type":"plain","data":"好","streaming":true,"chain_type":null}"#,
            "\n\n",
            r#"data: {"type":"error","data":{"code":"agent_error","message":"boom"}}"#,
            "\n\n",
            r#"data: {"type":"end"}"#,
            "\n\n",
        );

        // Split the stream at a byte inside the first delta's UTF-8 sequence
        // to prove the byte-buffered parser survives arbitrary chunking.
        let split = canned.find("你").unwrap_or_default() + 1;
        let (head, tail) = canned.as_bytes().split_at(split);
        let mut parser = SseParser::default();
        let mut raw_payloads = parser.push(head);
        raw_payloads.extend(parser.push(tail));

        let events: Vec<ChatStreamEvent> = raw_payloads
            .iter()
            .filter_map(|data| map_sse_data(data))
            .collect();

        assert_eq!(
            events,
            vec![
                ChatStreamEvent::Session {
                    session_id: "s-1".to_string()
                },
                ChatStreamEvent::Delta {
                    text: "你".to_string(),
                    chain: None
                },
                ChatStreamEvent::Delta {
                    text: "思考".to_string(),
                    chain: Some("reasoning".to_string())
                },
                ChatStreamEvent::Delta {
                    text: "好".to_string(),
                    chain: None
                },
                ChatStreamEvent::Error {
                    message: "boom".to_string()
                },
                ChatStreamEvent::Done,
            ]
        );
    }

    #[test]
    fn sse_parser_accepts_crlf_frames_at_any_chunk_boundary() {
        let lf = concat!(
            r#"data: {"type":"session_id","data":null,"session_id":"s-1"}"#,
            "\n\n",
            ": heartbeat\n\n",
            r#"data: {"type":"plain","data":"你","streaming":true}"#,
            "\n\n",
            r#"data: {"type":"plain","data":"思考","chain_type":"reasoning"}"#,
            "\n\n",
            r#"data: {"type":"end"}"#,
            "\n\n",
        );
        let crlf = lf.replace("\n\n", "\r\n\r\n");

        let expected = vec![
            ChatStreamEvent::Session {
                session_id: "s-1".to_string(),
            },
            ChatStreamEvent::Delta {
                text: "你".to_string(),
                chain: None,
            },
            ChatStreamEvent::Delta {
                text: "思考".to_string(),
                chain: Some("reasoning".to_string()),
            },
            ChatStreamEvent::Done,
        ];

        let parse = |stream: &str, split: usize| -> Vec<ChatStreamEvent> {
            let mut parser = SseParser::default();
            let (head, tail) = stream.as_bytes().split_at(split);
            let mut payloads = parser.push(head);
            payloads.extend(parser.push(tail));
            assert!(!parser.overflowed);
            payloads
                .iter()
                .filter_map(|data| map_sse_data(data))
                .collect()
        };

        assert_eq!(parse(lf, lf.len()), expected);
        // Every byte boundary, including the middle of a CRLF separator.
        for split in 0..=crlf.len() {
            assert_eq!(parse(&crlf, split), expected, "split at {split}");
        }
    }

    #[test]
    fn chat_send_mode_parses_the_pinned_values() {
        assert_eq!(ChatSendMode::parse(None), Some(ChatSendMode::Send));
        assert_eq!(ChatSendMode::parse(Some("")), Some(ChatSendMode::Send));
        assert_eq!(ChatSendMode::parse(Some("send")), Some(ChatSendMode::Send));
        assert_eq!(
            ChatSendMode::parse(Some(" send ")),
            Some(ChatSendMode::Send)
        );
        assert_eq!(
            ChatSendMode::parse(Some("regenerate")),
            Some(ChatSendMode::Regenerate)
        );
        assert_eq!(
            ChatSendMode::parse(Some(" regenerate ")),
            Some(ChatSendMode::Regenerate)
        );
        for rejected in ["regen", "SEND", "send-message"] {
            assert_eq!(ChatSendMode::parse(Some(rejected)), None, "{rejected}");
        }
    }

    fn test_endpoint(username: &str, password: &str) -> InstanceEndpoint {
        InstanceEndpoint {
            host: "127.0.0.1".to_string(),
            port: 6185,
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    #[test]
    fn resolve_session_source_prefers_secret_then_credentials() {
        let with_secret = test_endpoint("user", "pass");
        assert_eq!(
            resolve_session_source(true, &with_secret),
            SessionSource::Desktop
        );
        // Without a launch secret, stored credentials are the only exchange.
        assert_eq!(
            resolve_session_source(false, &with_secret),
            SessionSource::Credentials
        );
        // No secret and no credentials: the SPA's own form is the fallback.
        let bare = test_endpoint("", "");
        assert_eq!(resolve_session_source(false, &bare), SessionSource::Manual);
    }

    #[test]
    fn credentials_available_matches_the_login_emptiness_check() {
        assert!(credentials_available("user", "pass"));
        assert!(!credentials_available("", "pass"));
        // Whitespace-only usernames are treated as missing, like `login`.
        assert!(!credentials_available("  ", "pass"));
        assert!(!credentials_available("user", ""));
    }

    #[test]
    #[allow(clippy::expect_used)]
    fn parse_desktop_session_response_extracts_token_and_username() {
        let body = r#"{"status":"ok","data":{"token":"jwt-1","username":"astrbot"}}"#;
        let parsed = parse_desktop_session_response(body, Locale::EnUs)
            .expect("a well-formed session envelope parses");
        assert_eq!(parsed, ("jwt-1".to_string(), "astrbot".to_string()));
    }

    #[test]
    fn parse_desktop_session_response_rejects_unusable_payloads() {
        // Missing token, missing username, error envelope, and a non-JSON
        // body must all fail so the caller falls back to credentials.
        for body in [
            r#"{"status":"ok","data":{"username":"astrbot"}}"#,
            r#"{"status":"ok","data":{"token":"jwt-1"}}"#,
            r#"{"status":"error","message":"Not found","data":null}"#,
            "<html>502</html>",
        ] {
            assert!(
                parse_desktop_session_response(body, Locale::EnUs).is_err(),
                "body: {body}"
            );
        }
    }

    #[test]
    fn manual_login_required_marks_the_expected_state() {
        let error = manual_login_required(Locale::EnUs);
        assert_eq!(error.kind(), ErrorKind::Network);
        assert_eq!(error.detail(), Locale::EnUs.dashboard_webui_manual_login());
        // The marker lets the WebUI view stay silent on an expected state.
        assert!(
            error
                .detail()
                .contains("Passwordless sign-in is unavailable"),
            "unexpected detail: {}",
            error.detail()
        );
    }

    #[test]
    fn regenerate_payload_carries_only_the_model_selection() {
        let payload = regenerate_payload(Some("provider-1"), Some("gpt-4o"));
        assert_eq!(payload["enable_streaming"], true);
        assert_eq!(payload["selected_provider"], "provider-1");
        assert_eq!(payload["selected_model"], "gpt-4o");
        for key in ["session_id", "message"] {
            assert!(payload.get(key).is_none(), "unexpected {key}");
        }

        let minimal = regenerate_payload(None, None);
        assert!(minimal.get("selected_provider").is_none());
        assert!(minimal.get("selected_model").is_none());
    }

    #[test]
    fn regenerate_url_encodes_route_segments() {
        assert_eq!(
            regenerate_url("127.0.0.1", 6185, "webchat!admin!abc", "42"),
            "http://127.0.0.1:6185/api/v1/chat/sessions/webchat%21admin%21abc/messages/42/regenerate"
        );
        // Separators in either segment cannot escape the chat route prefix.
        assert_eq!(
            regenerate_url("127.0.0.1", 6185, "a/b?c", "1"),
            "http://127.0.0.1:6185/api/v1/chat/sessions/a%2Fb%3Fc/messages/1/regenerate"
        );
        assert_eq!(
            regenerate_url("127.0.0.1", 6185, "s-1", "../x"),
            "http://127.0.0.1:6185/api/v1/chat/sessions/s-1/messages/..%2Fx/regenerate"
        );
    }

    #[test]
    fn validate_media_path_accepts_the_two_dashboard_shapes() {
        let locale = Locale::EnUs;
        for path in [
            "/api/v1/files/abc-123/content",
            "/api/v1/files/content?filename=x.png",
            "/api/v1/files/content?filename=a%2Fb.png&download=1",
        ] {
            assert!(validate_media_path(path, locale).is_ok(), "{path}");
        }
    }

    #[test]
    fn validate_media_path_rejects_everything_else() {
        let locale = Locale::EnUs;
        for path in [
            "/api/v1/files",
            "/api/v1/files/abc",
            "/api/v1/files/abc/other",
            "/api/v1/files//content",
            "/api/v1/files/../content",
            "/api/v1/files/a/b/content",
            "/api/v1/files/content",
            "/api/v1/files/content?filename=",
            "/api/v1/chat/sessions",
            "api/v1/files/abc/content",
            "//api/v1/files/abc/content",
            // Percent-encoded dot segments must be blocked like their literal equivalents.
            "/api/v1/files/%2e%2e/content",
            "/api/v1/files/%2E%2E/content",
            "/api/v1/files/.%2e/content",
            "/api/v1/files/%2e./content",
            // Backslash separators must be rejected before the segment walk.
            "/api/v1/files/..\\content",
            "/api/v1/files\\content",
            // A six-segment backslash path would pass the shape check without the guard.
            "/api/v1/files/..\\..\\plugins/content",
            // Fragments must be stripped before segment validation.
            "/api/v1/files/%2e%2e#x/content",
            // Encoded slashes must be rejected since upstream can decode them.
            "/api/v1/files/%2f../content",
        ] {
            assert!(validate_media_path(path, locale).is_err(), "{path}");
        }
    }

    #[test]
    fn base64_encode_matches_rfc4648_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64_encode(&[0x00, 0xFF, 0x10]), "AP8Q");
    }

    #[test]
    fn sse_parser_flags_an_oversized_event_buffer() {
        let mut parser = SseParser::default();
        let chunk = vec![b'x'; MAX_EVENT_BYTES];
        assert!(parser.push(&chunk).is_empty());
        assert!(!parser.overflowed);

        assert!(parser.push(b"x").is_empty());
        assert!(parser.overflowed);
        assert!(parser.buffer.is_empty());
    }

    #[test]
    fn host_normalization_matches_the_webui_view() {
        for (input, expected) in [
            ("", "127.0.0.1"),
            ("0.0.0.0", "127.0.0.1"),
            ("::", "127.0.0.1"),
            ("[::]", "127.0.0.1"),
            ("localhost", "127.0.0.1"),
            ("Localhost", "127.0.0.1"),
            ("  127.0.0.1  ", "127.0.0.1"),
            ("192.168.1.5", "192.168.1.5"),
            ("fe80::1", "[fe80::1]"),
            ("[fe80::1]", "[fe80::1]"),
            ("example.com", "example.com"),
        ] {
            assert_eq!(normalize_dashboard_host(input), expected, "input: {input}");
        }
    }

    #[test]
    fn dashboard_base_url_uses_http_only_for_loopback() {
        for (host, expected) in [
            ("127.0.0.1", "http://127.0.0.1:6185"),
            ("127.0.0.2", "http://127.0.0.2:6185"),
            ("localhost", "http://127.0.0.1:6185"),
            ("0.0.0.0", "http://127.0.0.1:6185"),
            ("::1", "http://[::1]:6185"),
            ("[::1]", "http://[::1]:6185"),
            // Remote IPv6 stays bracketed and must use TLS.
            ("fe80::1", "https://[fe80::1]:6185"),
            ("[fe80::1]", "https://[fe80::1]:6185"),
            ("192.168.1.5", "https://192.168.1.5:6185"),
            ("example.com", "https://example.com:6185"),
        ] {
            assert_eq!(dashboard_base_url(host, 6185), expected, "host: {host}");
        }
    }

    #[test]
    fn login_response_parsing_returns_the_token_or_the_reason() -> Result<()> {
        let locale = Locale::ZhCn;
        assert_eq!(
            parse_login_response(
                r#"{"status":"ok","message":null,"data":{"token":"jwt","username":"admin"}}"#,
                locale
            )?,
            "jwt"
        );

        let rejected = parse_login_response(
            r#"{"status":"error","message":"用户名或密码错误","data":null}"#,
            locale,
        )
        .err()
        .map(|error| error.detail().to_string());
        assert_eq!(rejected.as_deref(), Some("用户名或密码错误"));

        let mfa = parse_login_response(
            r#"{"status":"error","message":"需要 TOTP 验证","data":{"totp_required":true}}"#,
            locale,
        )
        .err()
        .map(|error| error.detail().to_string());
        assert_eq!(mfa, Some(locale.dashboard_mfa_unsupported()));

        let malformed = parse_login_response("<html>no</html>", locale)
            .err()
            .map(|error| error.detail().to_string());
        assert_eq!(malformed, Some(locale.dashboard_login_failed()));
        Ok(())
    }

    #[test]
    fn new_session_response_parsing_accepts_the_dashboard_shape() -> Result<()> {
        let locale = Locale::ZhCn;
        let envelope: ApiEnvelope<RawSession> = serde_json::from_str(
            r#"{"status":"ok","message":null,"data":{"session_id":"webchat!admin!abc","platform_id":"webchat"}}"#,
        )?;
        assert_eq!(
            envelope_data(envelope, locale)?.session_id,
            "webchat!admin!abc"
        );

        let failed: ApiEnvelope<RawSession> =
            serde_json::from_str(r#"{"status":"error","message":"create failed","data":null}"#)?;
        assert_eq!(
            envelope_data(failed, locale)
                .err()
                .map(|error| error.detail().to_string())
                .as_deref(),
            Some("create failed")
        );
        Ok(())
    }

    #[test]
    fn chat_payload_carries_the_session_and_selection_but_no_username() {
        let payload = chat_payload(
            "webchat!admin!abc",
            "hi",
            Some("provider-1"),
            Some("gpt-4o"),
        );
        assert_eq!(payload["session_id"], "webchat!admin!abc");
        assert_eq!(payload["message"], "hi");
        assert_eq!(payload["enable_streaming"], true);
        assert_eq!(payload["selected_provider"], "provider-1");
        assert_eq!(payload["selected_model"], "gpt-4o");
        assert!(payload.get("username").is_none());

        let minimal = chat_payload("s-1", "hi", None, None);
        assert!(minimal.get("selected_provider").is_none());
        assert!(minimal.get("selected_model").is_none());
    }

    #[test]
    fn proxy_allowlist_accepts_chat_writes_and_read_only_routes() {
        let locale = Locale::ZhCn;
        for method in CHAT_METHODS {
            assert!(
                validate_proxy_target(
                    method,
                    "/api/v1/chat/sessions/new?platform_id=webchat",
                    locale
                )
                .is_ok(),
                "method {method}"
            );
            assert!(
                validate_proxy_target(method, "/api/v1/chat/sessions/s-1/messages/m-1", locale)
                    .is_ok(),
                "method {method}"
            );
        }
        for path in [
            "/api/v1/providers",
            "/api/v1/providers?capability=chat",
            "/api/v1/configs",
            "/api/v1/personas/by-id?persona_id=x",
            "/api/v1/commands",
            "/api/v1/config-profiles",
            "/api/v1/config-profiles/default",
            "/api/v1/config-routes",
            "/api/v1/config-routes?umo=webchat%21admin",
        ] {
            assert!(
                validate_proxy_target("GET", path, locale).is_ok(),
                "path {path}"
            );
        }

        // The added read-only routes stay read-only.
        for path in [
            "/api/v1/commands",
            "/api/v1/config-profiles/default",
            "/api/v1/config-routes",
        ] {
            assert!(
                validate_proxy_target("PATCH", path, locale).is_err(),
                "path {path}"
            );
            assert!(
                validate_proxy_target("DELETE", path, locale).is_err(),
                "path {path}"
            );
        }
    }

    #[test]
    fn proxy_allowlist_rejects_unrelated_or_escaping_targets() {
        let locale = Locale::ZhCn;
        for (method, path) in [
            ("PUT", "/api/v1/chat/sessions/s-1"),
            ("GET", "/api/v1/plugins"),
            ("POST", "/api/v1/providers"),
            ("DELETE", "/api/v1/personas"),
            ("GET", "api/v1/chat/sessions"),
            ("GET", "//evil.example/api/v1/chat/sessions"),
            ("GET", "/api/v1/chat/../plugins"),
            ("GET", "/api/v1/chat/sessions/../../plugins"),
            ("GET", ""),
            // Percent-encoded dot segments must be treated the same as literal ones.
            ("GET", "/api/v1/chat/%2e%2e/plugins"),
            ("GET", "/api/v1/chat/%2E%2E/plugins"),
            ("GET", "/api/v1/chat/.%2e/plugins"),
            ("GET", "/api/v1/chat/%2e./plugins"),
            ("GET", "/api/v1/chat/%2e/plugins"),
            // Backslash traversal must be rejected before the segment walk.
            ("GET", "/api/v1/chat/..\\..\\plugins"),
            ("GET", "/api/v1/chat\\..\\..\\plugins"),
            // Fragments must be stripped before segment validation.
            ("GET", "/api/v1/chat/%2e%2e#fragment"),
            // Encoded slashes must be rejected since upstream can decode them.
            ("GET", "/api/v1/chat/%2f../plugins"),
        ] {
            assert!(
                validate_proxy_target(method, path, locale).is_err(),
                "accepted {method} {path}"
            );
        }
    }

    #[test]
    fn http_failures_map_to_actionable_guidance() {
        let locale = Locale::ZhCn;
        let cases = [
            (401u16, locale.dashboard_login_failed()),
            (403, locale.dashboard_forbidden()),
            (404, locale.dashboard_unsupported_version()),
        ];
        for (status, expected) in cases {
            let failure = reqwest::StatusCode::from_u16(status)
                .ok()
                .and_then(classify_status);
            assert_eq!(
                failure.map(|f| status_error_message(f, locale)).as_deref(),
                Some(expected.as_str())
            );
        }

        let other = classify_status(reqwest::StatusCode::INTERNAL_SERVER_ERROR)
            .map(|f| status_error_message(f, locale))
            .unwrap_or_default();
        assert!(other.starts_with("HTTP 500"), "actual: {other}");
        assert!(classify_status(reqwest::StatusCode::OK).is_none());
    }

    #[test]
    fn a_clean_eof_completes_the_turn_unless_it_already_finished() {
        let locale = Locale::ZhCn;
        assert_eq!(
            stream_end_event(false, false, locale),
            Some(ChatStreamEvent::Done)
        );
        assert_eq!(stream_end_event(true, false, locale), None);
        assert_eq!(
            stream_end_event(false, true, locale),
            Some(ChatStreamEvent::Error {
                message: locale.dashboard_unreachable()
            })
        );
        assert_eq!(stream_end_event(true, true, locale), None);
    }
}
