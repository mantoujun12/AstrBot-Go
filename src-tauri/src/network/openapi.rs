//! AstrBot OpenAPI (WebUI HTTP API, AstrBot >= v4.18.0) chat commands.
//!
//! All HTTP calls happen here in Rust: the webview never sees the API key and
//! is not exposed to CORS. The key is sent as `Authorization: Bearer abk_...`
//! and must never appear in logs or error payloads.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::State;

use crate::commands::AppState;
use crate::config::{load_config, load_manifest};
use crate::error::{AppError, Result};
use crate::i18n::Locale;
use crate::network_config;
use crate::process::InstanceRuntimeInfo;
use crate::utils::net::USER_AGENT;
use crate::utils::validation::validate_instance_id;

/// Fixed WebChat identity used as the OpenAPI `username` and session owner.
const CHAT_USERNAME: &str = "astrbot-go";

/// Lists must not hang on a stalled response; chat streaming is separate and
/// deliberately has no total timeout.
const LIST_TIMEOUT: Duration = Duration::from_secs(30);

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
    Done,
    Error {
        message: String,
    },
}

/// Provider entry normalized from AstrBot's provider configuration objects.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AstrBotProvider {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub models: Vec<String>,
}

/// Conversation entry normalized from `GET /api/v1/chat/sessions`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AstrBotSession {
    pub id: String,
    pub title: String,
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
struct ProvidersData {
    #[serde(default)]
    providers: Vec<RawProvider>,
}

#[derive(Debug, Deserialize)]
struct RawProvider {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    enable: Option<bool>,
    #[serde(default)]
    model: Option<String>,
}

impl From<RawProvider> for AstrBotProvider {
    fn from(raw: RawProvider) -> Self {
        Self {
            name: raw
                .name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| raw.id.clone()),
            enabled: raw.enable.unwrap_or(false),
            models: raw
                .model
                .filter(|model| !model.trim().is_empty())
                .map(|model| vec![model])
                .unwrap_or_default(),
            id: raw.id,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SessionsData {
    #[serde(default)]
    sessions: Vec<RawSession>,
}

#[derive(Debug, Deserialize)]
struct RawSession {
    #[serde(default)]
    session_id: String,
    #[serde(default)]
    display_name: Option<String>,
}

impl From<RawSession> for AstrBotSession {
    fn from(raw: RawSession) -> Self {
        Self {
            title: raw
                .display_name
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| raw.session_id.clone()),
            id: raw.session_id,
        }
    }
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
/// `src/views/InstanceWebuiView.vue`.
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

fn status_error_message(failure: StatusFailure, locale: Locale, scope: &str) -> String {
    match failure {
        StatusFailure::Unauthorized => locale.openapi_invalid_key(),
        StatusFailure::Forbidden => locale.openapi_insufficient_scope(scope),
        StatusFailure::NotFound => locale.openapi_unsupported_version(),
        StatusFailure::Other(code) => format!("HTTP {code}: {}", locale.openapi_stream_error()),
    }
}

fn current_locale() -> Locale {
    load_config()
        .map(|config| Locale::resolve(config.language_preference))
        .unwrap_or_default()
}

/// Host, dial port, and stored OpenAPI key for an instance. The live port is
/// preferred when the instance is running, mirroring the WebUI view.
fn instance_endpoint(state: &AppState, instance_id: &str) -> Result<(String, u16, String)> {
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
    Ok((host, port, instance.openapi_key.clone()))
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
/// host gets TLS so the bearer key never travels in cleartext.
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
/// bypass any configured proxy so it cannot capture the bearer key.
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

fn require_key(key: &str, locale: Locale) -> Result<String> {
    let key = key.trim().to_string();
    if key.is_empty() {
        return Err(AppError::network(locale.openapi_key_missing()));
    }
    Ok(key)
}

/// GET an OpenAPI endpoint and return its `data` payload, mapping auth and
/// reachability failures to user-actionable messages.
async fn get_openapi_data<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    key: &str,
    locale: Locale,
    scope: &str,
) -> Result<T> {
    let resp = client
        .get(url)
        .bearer_auth(key)
        .header("User-Agent", USER_AGENT)
        .timeout(LIST_TIMEOUT)
        .send()
        .await
        .map_err(|error| {
            // The key lives in a header, which reqwest never includes in the
            // error text; redact anyway to keep this invariant obvious.
            if error.is_connect() {
                AppError::network(locale.openapi_unreachable())
            } else {
                log::warn!(
                    "AstrBot OpenAPI request failed: {}",
                    crate::utils::log_redact::redact_text(&error.to_string())
                );
                AppError::network(locale.openapi_stream_error())
            }
        })?;

    if let Some(failure) = classify_status(resp.status()) {
        return Err(AppError::network(status_error_message(
            failure, locale, scope,
        )));
    }

    let envelope: ApiEnvelope<T> = resp
        .json()
        .await
        .map_err(|_| AppError::network(locale.openapi_stream_error()))?;
    if envelope.status == "error" {
        return Err(AppError::network(
            envelope
                .message
                .filter(|message| !message.trim().is_empty())
                .unwrap_or_else(|| locale.openapi_stream_error()),
        ));
    }

    envelope
        .data
        .ok_or_else(|| AppError::network(locale.openapi_stream_error()))
}

#[tauri::command]
pub async fn chat_list_providers(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<Vec<AstrBotProvider>> {
    validate_instance_id(&instance_id)?;
    let locale = current_locale();
    let (host, port, raw_key) = instance_endpoint(&state, &instance_id)?;
    let key = require_key(&raw_key, locale)?;
    let client = build_instance_client(&host)?;
    let url = format!(
        "{}/api/v1/providers?capability=chat",
        dashboard_base_url(&host, port)
    );

    let data: ProvidersData = get_openapi_data(&client, &url, &key, locale, "provider").await?;
    Ok(data
        .providers
        .into_iter()
        .map(AstrBotProvider::from)
        .collect())
}

#[tauri::command]
pub async fn chat_list_sessions(
    state: State<'_, AppState>,
    instance_id: String,
) -> Result<Vec<AstrBotSession>> {
    validate_instance_id(&instance_id)?;
    let locale = current_locale();
    let (host, port, raw_key) = instance_endpoint(&state, &instance_id)?;
    let key = require_key(&raw_key, locale)?;
    let client = build_instance_client(&host)?;
    let url = format!(
        "{}/api/v1/chat/sessions?username={CHAT_USERNAME}",
        dashboard_base_url(&host, port)
    );

    let data: SessionsData = get_openapi_data(&client, &url, &key, locale, "chat").await?;
    Ok(data
        .sessions
        .into_iter()
        .map(AstrBotSession::from)
        .collect())
}

#[tauri::command]
pub async fn chat_send_message(
    state: State<'_, AppState>,
    instance_id: String,
    session_id: Option<String>,
    message: String,
    model: Option<String>,
    on_event: Channel<ChatStreamEvent>,
) -> Result<()> {
    validate_instance_id(&instance_id)?;
    // Blank strings from the webview are treated as "not provided".
    let session_id = session_id.filter(|value| !value.trim().is_empty());
    let model = model.filter(|value| !value.trim().is_empty());
    let locale = current_locale();
    let (host, port, raw_key) = instance_endpoint(&state, &instance_id)?;
    let key = require_key(&raw_key, locale)?;
    let client = build_instance_client(&host)?;

    let mut body = serde_json::json!({
        "username": CHAT_USERNAME,
        "message": message,
        "enable_streaming": true,
    });
    if let Some(session_id) = &session_id {
        body["session_id"] = serde_json::json!(session_id);
    }
    if let Some(model) = &model {
        body["selected_model"] = serde_json::json!(model);
    }

    // A caller-provided session id is already known; the server echoes it as
    // the first SSE event, which is skipped via the emitted flag.
    let mut session_emitted = session_id.is_some();
    if let Some(session_id) = &session_id {
        if on_event
            .send(ChatStreamEvent::Session {
                session_id: session_id.clone(),
            })
            .is_err()
        {
            return Ok(());
        }
    }

    let send_result = client
        .post(format!("{}/api/v1/chat", dashboard_base_url(&host, port)))
        .bearer_auth(key)
        .header("User-Agent", USER_AGENT)
        .json(&body)
        .send()
        .await;
    let resp = match send_result {
        Ok(resp) => resp,
        Err(error) => {
            if !error.is_connect() {
                log::warn!(
                    "AstrBot OpenAPI chat request failed: {}",
                    crate::utils::log_redact::redact_text(&error.to_string())
                );
            }
            let _ = on_event.send(ChatStreamEvent::Error {
                message: locale.openapi_unreachable(),
            });
            return Ok(());
        }
    };

    if let Some(failure) = classify_status(resp.status()) {
        let _ = on_event.send(ChatStreamEvent::Error {
            message: status_error_message(failure, locale, "chat"),
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
        let envelope: ApiEnvelope<serde_json::Value> =
            serde_json::from_str(&text).unwrap_or(ApiEnvelope {
                status: String::new(),
                message: None,
                data: None,
            });
        let message = if envelope.status == "error" {
            envelope
                .message
                .filter(|message| !message.trim().is_empty())
                .unwrap_or_else(|| locale.openapi_stream_error())
        } else {
            locale.openapi_stream_error()
        };
        let _ = on_event.send(ChatStreamEvent::Error { message });
        return Ok(());
    }

    let mut parser = SseParser::default();
    let mut finished = false;
    let mut stream = resp;
    loop {
        let chunk = match stream.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(error) => {
                log::warn!(
                    "AstrBot OpenAPI chat stream aborted: {}",
                    crate::utils::log_redact::redact_text(&error.to_string())
                );
                break;
            }
        };

        for data in parser.push(&chunk) {
            let Some(event) = map_sse_data(&data) else {
                continue;
            };
            match event {
                ChatStreamEvent::Session { session_id } => {
                    if session_emitted {
                        continue;
                    }
                    session_emitted = true;
                    if on_event
                        .send(ChatStreamEvent::Session { session_id })
                        .is_err()
                    {
                        return Ok(());
                    }
                }
                ChatStreamEvent::Delta { text, chain } => {
                    if on_event
                        .send(ChatStreamEvent::Delta { text, chain })
                        .is_err()
                    {
                        return Ok(());
                    }
                }
                ChatStreamEvent::Error { message } => {
                    let message = if message.is_empty() {
                        locale.openapi_stream_error()
                    } else {
                        message
                    };
                    if on_event.send(ChatStreamEvent::Error { message }).is_err() {
                        return Ok(());
                    }
                }
                ChatStreamEvent::Done => {
                    let _ = on_event.send(ChatStreamEvent::Done);
                    finished = true;
                }
            }
        }

        if parser.overflowed {
            let _ = on_event.send(ChatStreamEvent::Error {
                message: locale.openapi_stream_error(),
            });
            return Ok(());
        }

        if finished {
            break;
        }
    }

    if !finished {
        let _ = on_event.send(ChatStreamEvent::Error {
            message: locale.openapi_unreachable(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locale_messages(locale: Locale) -> Vec<String> {
        vec![
            locale.openapi_key_missing(),
            locale.openapi_invalid_key(),
            locale.openapi_insufficient_scope("chat"),
            locale.openapi_unreachable(),
            locale.openapi_unsupported_version(),
            locale.openapi_stream_error(),
        ]
    }

    #[test]
    fn openapi_locale_messages_exist_in_every_language() {
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
    fn http_failures_map_to_actionable_guidance() {
        let locale = Locale::ZhCn;
        let cases = [
            (401u16, locale.openapi_invalid_key()),
            (403, locale.openapi_insufficient_scope("chat")),
            (404, locale.openapi_unsupported_version()),
        ];
        for (status, expected) in cases {
            let failure = reqwest::StatusCode::from_u16(status)
                .ok()
                .and_then(classify_status);
            assert_eq!(
                failure
                    .map(|f| status_error_message(f, locale, "chat"))
                    .as_deref(),
                Some(expected.as_str())
            );
        }

        let other = classify_status(reqwest::StatusCode::INTERNAL_SERVER_ERROR)
            .map(|f| status_error_message(f, locale, "chat"))
            .unwrap_or_default();
        assert!(other.starts_with("HTTP 500"), "actual: {other}");
        assert!(classify_status(reqwest::StatusCode::OK).is_none());
    }

    #[test]
    fn provider_and_session_responses_parse_leniently() -> Result<()> {
        let data: ProvidersData = serde_json::from_str(
            r#"{"providers":[
                {"id":"p1","name":"GPT","enable":true,"model":"gpt-4o","key":"secret","unknown":1},
                {"id":"p2"},
                {"id":"p3","enable":false}
            ],"model_metadata":{}}"#,
        )?;

        let providers: Vec<AstrBotProvider> = data
            .providers
            .into_iter()
            .map(AstrBotProvider::from)
            .collect();
        assert_eq!(
            providers,
            vec![
                AstrBotProvider {
                    id: "p1".to_string(),
                    name: "GPT".to_string(),
                    enabled: true,
                    models: vec!["gpt-4o".to_string()],
                },
                AstrBotProvider {
                    id: "p2".to_string(),
                    name: "p2".to_string(),
                    enabled: false,
                    models: vec![],
                },
                AstrBotProvider {
                    id: "p3".to_string(),
                    name: "p3".to_string(),
                    enabled: false,
                    models: vec![],
                },
            ]
        );

        let data: SessionsData = serde_json::from_str(
            r#"{"sessions":[
                {"session_id":"s-1","display_name":"聊天 1","creator":"astrbot-go"},
                {"session_id":"s-2","display_name":""}
            ],"page":1,"page_size":20,"total":2}"#,
        )?;

        let sessions: Vec<AstrBotSession> = data
            .sessions
            .into_iter()
            .map(AstrBotSession::from)
            .collect();
        assert_eq!(
            sessions,
            vec![
                AstrBotSession {
                    id: "s-1".to_string(),
                    title: "聊天 1".to_string(),
                },
                AstrBotSession {
                    id: "s-2".to_string(),
                    title: "s-2".to_string(),
                },
            ]
        );
        Ok(())
    }
}
