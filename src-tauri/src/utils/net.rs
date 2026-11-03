use std::time::{Duration, Instant};

use reqwest::{Client, NoProxy, Proxy};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::error::{AppError, Result};
use crate::utils::log_redact;
use crate::utils::proxy::{ProxySettings, ProxySource};

pub(crate) const USER_AGENT: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/AstrBotDevs/astrbot-launcher)"
);

const PROBE_TIMEOUT: Duration = Duration::from_secs(12);
const PROBE_SPEED_BUDGET: Duration = Duration::from_secs(6);
const PROBE_MAX_BYTES: u64 = 4 * 1024 * 1024;
const PROBE_MIN_SPEED_BYTES: u64 = 32 * 1024;
const PROBE_MIN_SPEED_DURATION: Duration = Duration::from_millis(100);
const PYPI_INDEX_MAX_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct ProbeResult {
    pub latency_ms: u64,
    pub speed_bps: Option<f64>,
    pub bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeStage {
    Both,
    Latency,
    Speed,
}

fn probe_request(client: &Client, url: &str) -> reqwest::RequestBuilder {
    client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept-Encoding", "identity")
        .timeout(PROBE_TIMEOUT)
}

/// Measure time-to-first-body-byte and, optionally, sustained transfer speed.
pub(crate) async fn probe_endpoint(
    client: &Client,
    url: &str,
    stage: ProbeStage,
) -> Result<ProbeResult> {
    let started = Instant::now();
    let mut resp = probe_request(client, url)
        .send()
        .await
        .map_err(|e| AppError::network_with_url(url, e.to_string()))?;
    ensure_success_status(&resp, |detail| AppError::network_with_url(url, detail))?;

    let first = resp
        .chunk()
        .await
        .map_err(|e| AppError::network_with_url(url, e.to_string()))?;
    let latency_ms = elapsed_ms(started.elapsed());

    let Some(first) = first else {
        return Ok(ProbeResult {
            latency_ms,
            speed_bps: None,
            bytes: 0,
        });
    };

    let mut bytes = first.len() as u64;
    if stage == ProbeStage::Latency {
        return Ok(ProbeResult {
            latency_ms,
            speed_bps: None,
            bytes,
        });
    }

    let speed_started = Instant::now();
    let deadline = speed_started + PROBE_SPEED_BUDGET;
    while bytes < PROBE_MAX_BYTES {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, resp.chunk()).await {
            Ok(Ok(Some(chunk))) => bytes += chunk.len() as u64,
            Ok(Ok(None)) => break,
            Ok(Err(e)) => {
                return Err(AppError::network_with_url(url, e.to_string()));
            }
            Err(_) => break,
        }
    }

    Ok(ProbeResult {
        latency_ms,
        speed_bps: compute_speed_bps(bytes, speed_started.elapsed()),
        bytes,
    })
}

/// Fetch the beginning of a response body, returning time-to-first-byte.
pub(crate) async fn fetch_probe_body(
    client: &Client,
    url: &str,
    limit: usize,
) -> Result<(u64, Vec<u8>)> {
    let started = Instant::now();
    let mut resp = probe_request(client, url)
        .send()
        .await
        .map_err(|e| AppError::network_with_url(url, e.to_string()))?;
    ensure_success_status(&resp, |detail| AppError::network_with_url(url, detail))?;

    let first = resp
        .chunk()
        .await
        .map_err(|e| AppError::network_with_url(url, e.to_string()))?;
    let latency_ms = elapsed_ms(started.elapsed());
    let mut body = Vec::new();

    if let Some(first) = first {
        body.extend_from_slice(&first[..first.len().min(limit)]);
    }

    while body.len() < limit {
        let remaining = PROBE_TIMEOUT.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, resp.chunk()).await {
            Ok(Ok(Some(chunk))) => {
                let take = (limit - body.len()).min(chunk.len());
                body.extend_from_slice(&chunk[..take]);
            }
            Ok(Ok(None)) => break,
            Ok(Err(e)) => {
                return Err(AppError::network_with_url(url, e.to_string()));
            }
            Err(_) => break,
        }
    }

    Ok((latency_ms, body))
}

/// Measure a PyPI source using a real wheel when the index page exposes one.
pub(crate) async fn probe_pypi(
    client: &Client,
    index_url: &str,
    stage: ProbeStage,
) -> Result<ProbeResult> {
    if stage == ProbeStage::Latency {
        return probe_endpoint(client, index_url, ProbeStage::Latency).await;
    }

    let (index_latency, body) = fetch_probe_body(client, index_url, PYPI_INDEX_MAX_BYTES).await?;

    let wheel_url = extract_wheel_url(&String::from_utf8_lossy(&body))
        .and_then(|href| reqwest::Url::parse(index_url).ok()?.join(&href).ok());

    let mut result = match wheel_url {
        Some(url) => match probe_endpoint(client, url.as_str(), ProbeStage::Speed).await {
            Ok(result) => result,
            Err(_) => probe_endpoint(client, index_url, ProbeStage::Speed).await?,
        },
        None => probe_endpoint(client, index_url, ProbeStage::Speed).await?,
    };

    result.latency_ms = if stage == ProbeStage::Both {
        index_latency
    } else {
        0
    };
    Ok(result)
}

pub(crate) fn extract_wheel_url(html: &str) -> Option<String> {
    let mut fallback = None;

    for marker in ["href=\"", "href='"] {
        let quote = marker.chars().last().unwrap_or('"');
        let mut rest = html;
        while let Some(start) = rest.find(marker) {
            let fragment = &rest[start + marker.len()..];
            let Some(end) = fragment.find(quote) else {
                break;
            };
            let href = fragment[..end].trim();
            if href.contains(".whl") {
                if href.contains("-py3-none-any.whl") {
                    return Some(href.to_string());
                }
                if fallback.is_none() {
                    fallback = Some(href.to_string());
                }
            }
            rest = &fragment[end + 1..];
        }
    }

    fallback
}

pub(crate) fn compute_speed_bps(bytes: u64, elapsed: Duration) -> Option<f64> {
    if bytes < PROBE_MIN_SPEED_BYTES || elapsed < PROBE_MIN_SPEED_DURATION {
        return None;
    }
    Some(bytes as f64 / elapsed.as_secs_f64())
}

fn elapsed_ms(elapsed: Duration) -> u64 {
    elapsed.as_millis().min(u128::from(u64::MAX)) as u64
}

fn build_client_builder(proxy: Option<ProxySettings>) -> Result<reqwest::ClientBuilder> {
    let mut builder = Client::builder();

    let Some(proxy) = proxy else {
        return Ok(builder.no_proxy());
    };

    let mut has_proxy = false;

    if let Some(url) = proxy.all_proxy.as_deref() {
        if let Some(proxy_value) = build_reqwest_proxy(Proxy::all(url), &proxy, "all")? {
            builder = builder.proxy(proxy_value);
            has_proxy = true;
        }
    }

    if let Some(url) = proxy.http_proxy.as_deref() {
        if let Some(proxy_value) = build_reqwest_proxy(Proxy::http(url), &proxy, "http")? {
            builder = builder.proxy(proxy_value);
            has_proxy = true;
        }
    }

    if let Some(url) = proxy.https_proxy.as_deref() {
        if let Some(proxy_value) = build_reqwest_proxy(Proxy::https(url), &proxy, "https")? {
            builder = builder.proxy(proxy_value);
            has_proxy = true;
        }
    }

    Ok(if has_proxy {
        builder
    } else {
        builder.no_proxy()
    })
}

pub(crate) fn build_http_client_with_proxy(proxy: Option<ProxySettings>) -> Result<Client> {
    build_client_builder(proxy)?
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::network(format!("创建网络客户端失败：{}", e)))
}

/// Client for large downloads: limits connection setup only.
///
/// A total request timeout would abort long transfers and would also fire
/// while a download is paused (the response body is not being polled).
pub(crate) fn build_download_client_with_proxy(proxy: Option<ProxySettings>) -> Result<Client> {
    build_client_builder(proxy)?
        .connect_timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| AppError::network(format!("创建网络客户端失败：{}", e)))
}

fn build_reqwest_proxy(
    proxy_result: std::result::Result<Proxy, reqwest::Error>,
    settings: &ProxySettings,
    label: &str,
) -> Result<Option<Proxy>> {
    let mut reqwest_proxy = match proxy_result {
        Ok(proxy) => proxy,
        Err(error) if settings.source != ProxySource::AppConfig => {
            let source = match settings.source {
                ProxySource::AppConfig => "configured",
                ProxySource::Environment => "environment",
                ProxySource::System => "system",
            };
            log::warn!(
                "Invalid {} {} proxy address, ignored: {}",
                source,
                label,
                log_redact::redact_text(&error.to_string())
            );
            return Ok(None);
        }
        Err(error) => {
            return Err(AppError::config(format!(
                "代理地址无效：{}",
                log_redact::redact_text(&error.to_string())
            )));
        }
    };

    if let Some(no_proxy) = settings.no_proxy.as_deref() {
        reqwest_proxy = reqwest_proxy.no_proxy(NoProxy::from_string(no_proxy));
    }

    Ok(Some(reqwest_proxy))
}

pub(crate) async fn send_get(
    client: &Client,
    url: &str,
    accept_json: bool,
) -> std::result::Result<reqwest::Response, reqwest::Error> {
    let request = if accept_json {
        client
            .get(url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/json")
    } else {
        client.get(url).header("User-Agent", USER_AGENT)
    };
    request.send().await
}

pub(crate) fn ensure_success_status(
    resp: &reqwest::Response,
    make_error: impl FnOnce(String) -> AppError,
) -> Result<()> {
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(make_error(resp.status().to_string()))
    }
}

/// Fetch JSON from `url` and deserialize into `T`.
pub(crate) async fn fetch_json<T: DeserializeOwned>(client: &Client, url: &str) -> Result<T> {
    let resp = send_get(client, url, true)
        .await
        .map_err(|e| AppError::network(e.to_string()))?;
    ensure_success_status(&resp, AppError::network)?;

    resp.json::<T>()
        .await
        .map_err(|e| AppError::network(format!("Failed to parse response: {}", e)))
}

/// Check whether `url` is reachable (HTTP GET returns a success status).
pub(crate) async fn check_url(client: &Client, url: &str) -> Result<()> {
    let resp = send_get(client, url, false)
        .await
        .map_err(|e| AppError::network_with_url(url, e.to_string()))?;
    ensure_success_status(&resp, |detail| AppError::network_with_url(url, detail))?;

    Ok(())
}

#[cfg(test)]
mod probe_tests {
    use super::*;

    #[test]
    fn extract_wheel_url_prefers_pure_python_wheel() {
        let html = r#"<a href="../../packages/foo-1.0-cp312-cp312-win_amd64.whl#sha256=abc">a</a>
            <a href='../../packages/foo-1.0-py3-none-any.whl#sha256=def'>b</a>"#;
        assert_eq!(
            extract_wheel_url(html),
            Some("../../packages/foo-1.0-py3-none-any.whl#sha256=def".to_string())
        );
    }

    #[test]
    fn extract_wheel_url_picks_first_wheel_as_fallback() {
        let html = r"<a href='/packages/a-1.0-cp312.whl'>a</a>";
        assert_eq!(
            extract_wheel_url(html),
            Some("/packages/a-1.0-cp312.whl".to_string())
        );
        assert_eq!(extract_wheel_url("<a href='/simple/pip/'>none</a>"), None);
    }

    #[test]
    fn compute_speed_rejects_small_or_fast_samples() {
        assert_eq!(compute_speed_bps(1024, Duration::from_secs(1)), None);
        assert_eq!(
            compute_speed_bps(64 * 1024, Duration::from_millis(50)),
            None
        );
        let speed = compute_speed_bps(64 * 1024, Duration::from_millis(500)).unwrap_or_default();
        assert!((speed - 131_072.0).abs() < 1.0);
    }
}
