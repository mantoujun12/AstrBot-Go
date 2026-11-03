use std::net::{IpAddr, Ipv4Addr};
use std::time::{Duration, Instant};

use reqwest::redirect::Policy;
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
const PROBE_MAX_REDIRECTS: usize = 5;
const REDIRECT_LIMIT: usize = 10;
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

/// Resolve a `Location` header against the URL that produced it.
fn redirect_target(current: &reqwest::Url, location: &str) -> Option<reqwest::Url> {
    current.join(location.trim()).ok()
}

/// Parse a probe URL, rejecting non-HTTP schemes, missing hosts, and local host names.
fn parse_probe_url(url: &str) -> Result<reqwest::Url> {
    let parsed = reqwest::Url::parse(url)
        .map_err(|e| AppError::network_with_url(url, format!("探测目标地址无效：{}", e)))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(AppError::network_with_url(
            url,
            "仅支持 HTTP 或 HTTPS 探测目标",
        ));
    }
    let Some(host) = parsed.host_str() else {
        return Err(AppError::network_with_url(url, "探测目标缺少主机名"));
    };
    if host.is_empty() || probe_host_is_denied(host) {
        return Err(AppError::network_with_url(url, "探测目标地址不被允许"));
    }
    Ok(parsed)
}

/// Reject host names and IP literals that point at local or internal networks.
fn probe_host_is_denied(host: &str) -> bool {
    let host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('.');
    if let Ok(ip) = host.parse::<IpAddr>() {
        return probe_ip_is_denied(ip);
    }

    let host = host.to_ascii_lowercase();
    host.is_empty()
        || host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host.ends_with(".internal")
}

/// Reject IPv4 ranges that must not be reachable from a connectivity probe.
fn ipv4_is_reserved(v4: Ipv4Addr) -> bool {
    let octets = v4.octets();
    let carrier_grade_nat = octets[0] == 100 && (64..=127).contains(&octets[1]);
    let reserved = (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
        || (octets[0] == 198 && matches!(octets[1], 18 | 19))
        || octets[0] >= 240;
    let documentation = matches!(
        (octets[0], octets[1], octets[2]),
        (192, 0, 2) | (198, 51, 100) | (203, 0, 113)
    );
    carrier_grade_nat || reserved || documentation
}

/// Reject loopback, private, link-local, multicast, unspecified, and broadcast addresses.
fn probe_ip_is_denied(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_multicast()
                || v4.is_broadcast()
                || v4.is_unspecified()
                || ipv4_is_reserved(v4)
        }
        IpAddr::V6(v6) => {
            let segments = v6.segments();
            let multicast = (segments[0] & 0xff00) == 0xff00;
            let unique_local = (segments[0] & 0xfe00) == 0xfc00;
            let link_local = (segments[0] & 0xffc0) == 0xfe80;
            if v6.is_loopback() || v6.is_unspecified() || multicast || unique_local || link_local {
                return true;
            }
            // IPv4-mapped addresses such as ::ffff:127.0.0.1 must follow IPv4 rules.
            if segments[..5] == [0, 0, 0, 0, 0] && segments[5] == 0xffff {
                let mapped = Ipv4Addr::new(
                    (segments[6] >> 8) as u8,
                    (segments[6] & 0xff) as u8,
                    (segments[7] >> 8) as u8,
                    (segments[7] & 0xff) as u8,
                );
                return probe_ip_is_denied(IpAddr::V4(mapped));
            }
            false
        }
    }
}

/// Local DNS redirection is opt-in and only relaxes the address check for
/// hostnames; literal local IPs and local host names stay blocked.
fn resolved_probe_ip_is_denied(ip: IpAddr, allow_local_dns: bool) -> bool {
    !allow_local_dns && probe_ip_is_denied(ip)
}

/// Validate a probe destination, including every address its host resolves to.
async fn validate_probe_url(url: &str, allow_local_dns: bool) -> Result<reqwest::Url> {
    let parsed = parse_probe_url(url)?;
    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::network_with_url(url, "探测目标缺少主机名"))?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if host.parse::<IpAddr>().is_ok() {
        return Ok(parsed);
    }

    let port = parsed.port_or_known_default().unwrap_or(80);
    let addrs = tokio::net::lookup_host((host, port))
        .await
        .map_err(|e| AppError::network_with_url(url, format!("解析探测目标失败：{}", e)))?;

    let mut resolved = false;
    for addr in addrs {
        resolved = true;
        if resolved_probe_ip_is_denied(addr.ip(), allow_local_dns) {
            return Err(AppError::network_with_url(url, "探测目标地址不被允许"));
        }
    }
    if !resolved {
        return Err(AppError::network_with_url(url, "无法解析探测目标地址"));
    }
    Ok(parsed)
}

/// Resolve and validate a redirect target before it is requested.
fn parse_probe_redirect(base: &reqwest::Url, location: &str) -> Result<reqwest::Url> {
    let target = redirect_target(base, location)
        .ok_or_else(|| AppError::network_with_url(base.as_str(), "重定向目标无效"))?;
    parse_probe_url(target.as_str())
}

/// Follow redirects manually so every hop passes the destination policy.
///
/// Relies on the client built by `build_http_client_with_proxy`, which disables
/// reqwest's automatic redirects (reqwest has no per-request redirect policy).
async fn probe_get(
    client: &Client,
    url: reqwest::Url,
    allow_local_dns: bool,
) -> Result<reqwest::Response> {
    let mut current = url;
    let mut redirects = 0;

    loop {
        let resp = probe_request(client, current.as_str())
            .send()
            .await
            .map_err(|e| AppError::network_with_url(current.as_str(), e.to_string()))?;

        if !resp.status().is_redirection() {
            return Ok(resp);
        }
        let Some(location) = resp
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
        else {
            return Ok(resp);
        };
        if redirects >= PROBE_MAX_REDIRECTS {
            return Err(AppError::network_with_url(
                current.as_str(),
                "探测目标重定向次数过多",
            ));
        }

        let target = parse_probe_redirect(&current, location)?;
        current = validate_probe_url(target.as_str(), allow_local_dns).await?;
        redirects += 1;
    }
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
    allow_local_dns: bool,
) -> Result<ProbeResult> {
    let parsed = validate_probe_url(url, allow_local_dns).await?;
    let started = Instant::now();
    let mut resp = probe_get(client, parsed, allow_local_dns).await?;
    ensure_success_status(&resp, |detail| AppError::network_with_url(url, detail))?;

    // Start sampling before the first chunk so its bytes count toward the speed.
    let speed_started = Instant::now();
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
    allow_local_dns: bool,
) -> Result<(u64, Vec<u8>)> {
    let parsed = validate_probe_url(url, allow_local_dns).await?;
    let started = Instant::now();
    let mut resp = probe_get(client, parsed, allow_local_dns).await?;
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
    allow_local_dns: bool,
) -> Result<ProbeResult> {
    if stage == ProbeStage::Latency {
        return probe_endpoint(client, index_url, ProbeStage::Latency, allow_local_dns).await;
    }

    let (index_latency, body) =
        fetch_probe_body(client, index_url, PYPI_INDEX_MAX_BYTES, allow_local_dns).await?;

    let wheel_url = extract_wheel_url(&String::from_utf8_lossy(&body))
        .and_then(|href| reqwest::Url::parse(index_url).ok()?.join(&href).ok());

    let mut result = match wheel_url {
        Some(url) => match probe_endpoint(client, url.as_str(), ProbeStage::Speed, allow_local_dns)
            .await
        {
            Ok(result) => result,
            Err(_) => probe_endpoint(client, index_url, ProbeStage::Speed, allow_local_dns).await?,
        },
        None => probe_endpoint(client, index_url, ProbeStage::Speed, allow_local_dns).await?,
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
        // Redirects are followed manually (see probe_get/send_get) so probe hops
        // can be validated; reqwest cannot disable redirects per request.
        .redirect(Policy::none())
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
    let mut current = url.to_string();
    let mut redirects = 0;

    loop {
        let request = if accept_json {
            client
                .get(&current)
                .header("User-Agent", USER_AGENT)
                .header("Accept", "application/json")
        } else {
            client.get(&current).header("User-Agent", USER_AGENT)
        };
        let resp = request.send().await?;

        // The API client disables automatic redirects; keep following them here so
        // callers see the same final response as before.
        if !resp.status().is_redirection() || redirects >= REDIRECT_LIMIT {
            return Ok(resp);
        }
        let base = reqwest::Url::parse(&current).ok();
        let next = resp
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|location| base.and_then(|base| redirect_target(&base, location)));
        let Some(next) = next else {
            return Ok(resp);
        };
        current = next.to_string();
        redirects += 1;
    }
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

/// Fetch JSON after validating the destination and every redirect hop.
pub(crate) async fn fetch_json_checked<T: DeserializeOwned>(
    client: &Client,
    url: &str,
    allow_local_dns: bool,
) -> Result<T> {
    let parsed = validate_probe_url(url, allow_local_dns).await?;
    let resp = probe_get(client, parsed, allow_local_dns).await?;
    ensure_success_status(&resp, |detail| AppError::network_with_url(url, detail))?;

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

    #[test]
    fn probe_url_requires_http_scheme_and_host() {
        assert!(parse_probe_url("https://pypi.org/simple/").is_ok());
        assert!(parse_probe_url("http://mirror.example.com:8080/simple").is_ok());
        assert!(parse_probe_url("ftp://pypi.org/simple/").is_err());
        assert!(parse_probe_url("file:///etc/passwd").is_err());
        assert!(parse_probe_url("http://").is_err());
        assert!(parse_probe_url("not a url").is_err());
        assert!(parse_probe_url("http://localhost:8080/").is_err());
    }

    #[test]
    fn probe_host_policy_denies_local_destinations() {
        for host in [
            "localhost",
            "LOCALHOST.",
            "api.localhost",
            "printer.local",
            "metadata.internal",
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "224.0.0.1",
            "0.0.0.0",
            "255.255.255.255",
            "[::1]",
            "::",
            "fc00::1",
            "fd12:3456::1",
            "fe80::1",
            "ff02::1",
            "100.64.0.1",
            "192.0.2.1",
            "198.18.0.1",
            "240.0.0.1",
            "::ffff:127.0.0.1",
            "::ffff:192.168.0.1",
        ] {
            assert!(probe_host_is_denied(host), "{host} must be denied");
        }

        for host in [
            "pypi.org",
            "sub.local.example.com",
            "8.8.8.8",
            "2606:4700::1111",
            "[2606:4700::1111]",
        ] {
            assert!(!probe_host_is_denied(host), "{host} must be allowed");
        }
    }

    #[test]
    fn local_dns_option_only_relaxes_resolved_addresses() {
        let loopback = "127.0.0.1".parse::<IpAddr>();
        assert!(loopback.is_ok());
        let Ok(loopback) = loopback else {
            return;
        };
        assert!(resolved_probe_ip_is_denied(loopback, false));
        assert!(!resolved_probe_ip_is_denied(loopback, true));

        let public = "8.8.8.8".parse::<IpAddr>();
        assert!(public.is_ok());
        let Ok(public) = public else {
            return;
        };
        assert!(!resolved_probe_ip_is_denied(public, false));
    }

    #[test]
    fn local_dns_option_keeps_local_host_names_blocked() {
        assert!(parse_probe_url("http://localhost:8080/").is_err());
        assert!(parse_probe_url("http://127.0.0.1:8080/").is_err());
    }

    #[tokio::test]
    async fn local_dns_option_does_not_allow_literal_local_targets() {
        assert!(validate_probe_url("http://localhost:8080/", true)
            .await
            .is_err());
        assert!(validate_probe_url("http://127.0.0.1:8080/", true)
            .await
            .is_err());
        assert!(validate_probe_url("http://8.8.8.8/", true).await.is_ok());
    }

    #[test]
    fn probe_redirect_targets_are_revalidated() {
        let base_result = reqwest::Url::parse("https://pypi.org/simple/foo/");
        assert!(base_result.is_ok());
        let Ok(base) = base_result else {
            return;
        };

        let relative = parse_probe_redirect(&base, "../bar/index.html");
        assert!(relative.is_ok());
        let Ok(relative) = relative else {
            return;
        };
        assert_eq!(relative.as_str(), "https://pypi.org/simple/bar/index.html");

        let scheme_relative = parse_probe_redirect(&base, "//files.example.com/x.whl");
        assert!(scheme_relative.is_ok());
        let Ok(scheme_relative) = scheme_relative else {
            return;
        };
        assert_eq!(scheme_relative.as_str(), "https://files.example.com/x.whl");
        assert!(parse_probe_redirect(&base, "file:///etc/passwd").is_err());
        assert!(parse_probe_redirect(&base, "http://localhost/x").is_err());
        assert!(parse_probe_redirect(&base, "http://127.0.0.1/x").is_err());
    }
}
