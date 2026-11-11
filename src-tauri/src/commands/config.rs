use super::AppState;
use crate::component;
use crate::config::{
    load_config, with_config_mut, LanguagePreference, ReduceMotionPreference, ThemePreference,
    TrayStyle,
};
use crate::error::{AppError, Result};
use crate::github;
use crate::network_config::{self, NetworkSourceKind};
use crate::platform;
use crate::utils::index_url::normalize_default_index;
use crate::utils::net::{
    build_http_client_with_proxy, check_url, probe_endpoint, probe_pypi, ProbeResult, ProbeStage,
};
use crate::utils::proxy::{
    build_single_url_proxy_settings, ProxyFields, ProxySource, DEFAULT_NO_PROXY_VALUE,
};
use tauri::{AppHandle, State};

// === Config ===

#[tauri::command]
pub fn is_macos() -> bool {
    platform::is_macos()
}

#[tauri::command]
pub fn is_linux() -> bool {
    platform::is_linux()
}

#[tauri::command]
pub async fn save_github_proxy(github_proxy: String, state: State<'_, AppState>) -> Result<()> {
    let github_proxy = github_proxy.trim().to_string();

    // An empty value means "use github.com directly". Testing the default
    // source here would trap users behind an unreachable network or a broken
    // mirror they are trying to remove, so only custom proxies are tested.
    if !github_proxy.is_empty() {
        let url = github::build_api_url(&github_proxy);
        let client = state.client();
        check_url(&client, &url).await?;
    }

    with_config_mut(move |config| {
        config.github_proxy = github_proxy;
        Ok(())
    })
}

#[tauri::command]
pub async fn save_proxy(
    proxy_url: String,
    proxy_port: String,
    proxy_username: String,
    proxy_password: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let proxy_fields = ProxyFields::new(proxy_url, proxy_port, proxy_username, proxy_password);

    // Use a raw client (no system-proxy fallback) purely for connectivity tests.
    let configured_proxy = build_single_url_proxy_settings(
        ProxySource::AppConfig,
        &proxy_fields,
        Some(DEFAULT_NO_PROXY_VALUE.to_string()),
    )?;
    let test_proxy = configured_proxy.clone();
    let test_client = build_http_client_with_proxy(test_proxy)?;

    if !proxy_fields.url.is_empty()
        && check_url(&test_client, "https://cloudflare.com/cdn-cgi/trace")
            .await
            .is_err()
        && check_url(&test_client, "https://baidu.com").await.is_err()
    {
        return Err(AppError::network(
            "代理配置错误，无法连接 cloudflare.com 或 baidu.com",
        ));
    }

    let ProxyFields {
        url,
        port,
        username,
        password,
    } = proxy_fields;

    let next_client = with_config_mut(move |config| {
        config.proxy_url = url;
        config.proxy_port = port;
        config.proxy_username = username;
        config.proxy_password = password;
        network_config::build_http_client_from_config(config)
    })?;

    state.replace_client(next_client);

    Ok(())
}

#[tauri::command]
pub async fn save_pypi_mirror(pypi_mirror: String, state: State<'_, AppState>) -> Result<()> {
    let trimmed = pypi_mirror.trim();

    // An empty value means "use pypi.org directly" (see
    // `normalize_default_index`); testing the default source would block users
    // from removing a mirror they can no longer reach.
    let normalized_for_save = if trimmed.is_empty() {
        String::new()
    } else {
        let normalized_default_index = normalize_default_index(trimmed);
        let check_url_value = format!("{}/", normalized_default_index);
        let client = state.client();
        check_url(&client, &check_url_value).await?;
        normalized_default_index
    };

    with_config_mut(move |config| {
        config.pypi_mirror = normalized_for_save;
        Ok(())
    })
}

define_save_config_command!(save_close_to_tray, close_to_tray: bool, close_to_tray);
define_save_config_command!(
    save_autostart_minimize_to_tray,
    autostart_minimize_to_tray: bool,
    autostart_minimize_to_tray
);
define_save_config_command!(save_nodejs_mirror, nodejs_mirror: String, nodejs_mirror);
define_save_config_command!(save_npm_registry, npm_registry: String, npm_registry);

#[tauri::command]
pub async fn save_python_mirror(python_mirror: String) -> Result<()> {
    let python_mirror = python_mirror.trim().to_string();
    with_config_mut(move |config| {
        config.python_mirror = python_mirror;
        Ok(())
    })
}

#[tauri::command]
pub async fn save_uv_mirror(uv_mirror: String) -> Result<()> {
    let uv_mirror = uv_mirror.trim().to_string();
    with_config_mut(move |config| {
        config.uv_mirror = uv_mirror;
        Ok(())
    })
}

#[tauri::command]
pub async fn save_allow_local_dns_probe(allow_local_dns_probe: bool) -> Result<()> {
    with_config_mut(move |config| {
        config.allow_local_dns_probe = allow_local_dns_probe;
        Ok(())
    })
}

#[tauri::command]
pub async fn test_network_source(
    kind: NetworkSourceKind,
    value: String,
    stage: ProbeStage,
    state: State<'_, AppState>,
) -> Result<ProbeResult> {
    let config = load_config()?;
    let allow_local_dns = config.allow_local_dns_probe;
    let client = state.client();
    let url =
        network_config::source_probe_url(kind, &value, config.as_ref(), &client, allow_local_dns)
            .await?;

    if kind == NetworkSourceKind::Pypi {
        probe_pypi(&client, &url, stage, allow_local_dns).await
    } else {
        probe_endpoint(&client, &url, stage, allow_local_dns).await
    }
}

#[tauri::command]
pub async fn test_http_proxy(
    proxy_url: String,
    proxy_port: String,
    proxy_username: String,
    proxy_password: String,
) -> Result<ProbeResult> {
    let proxy_fields = ProxyFields::new(proxy_url, proxy_port, proxy_username, proxy_password);
    let proxy = build_single_url_proxy_settings(
        ProxySource::AppConfig,
        &proxy_fields,
        Some(DEFAULT_NO_PROXY_VALUE.to_string()),
    )?;
    let client = build_http_client_with_proxy(proxy)?;
    const SPEED_URL: &str = "https://speed.cloudflare.com/__down?bytes=1048576";

    match probe_endpoint(&client, SPEED_URL, ProbeStage::Both, false).await {
        Ok(result) => Ok(result),
        Err(_) => {
            let mut latency =
                probe_endpoint(&client, "https://www.baidu.com", ProbeStage::Latency, false)
                    .await?;
            latency.speed_bps = None;
            Ok(latency)
        }
    }
}

#[tauri::command]
pub async fn save_use_uv_for_deps(use_uv_for_deps: bool) -> Result<()> {
    if use_uv_for_deps && !component::is_uv_installed() {
        return Err(AppError::other("uv 组件未安装，无法启用 uv 安装依赖"));
    }

    with_config_mut(move |config| {
        config.use_uv_for_deps = use_uv_for_deps;
        Ok(())
    })
}

#[tauri::command]
pub fn compare_versions(a: String, b: String) -> i32 {
    match (
        semver::Version::parse(a.trim_start_matches('v')),
        semver::Version::parse(b.trim_start_matches('v')),
    ) {
        (Ok(va), Ok(vb)) => va.cmp(&vb) as i32,
        _ => 0,
    }
}

define_save_config_command!(
    save_check_instance_update,
    check_instance_update: bool,
    check_instance_update
);
define_save_config_command!(
    save_persist_instance_state,
    persist_instance_state: bool,
    persist_instance_state
);
define_save_config_command!(
    save_use_short_instance_id,
    use_short_instance_id: bool,
    use_short_instance_id
);
define_save_config_command!(
    save_ignore_external_path,
    ignore_external_path: bool,
    ignore_external_path
);
define_save_config_command!(
    save_lock_check_extension_whitelist,
    lock_check_extension_whitelist: bool,
    lock_check_extension_whitelist
);
define_save_config_command!(
    save_ignored_launcher_version,
    ignored_launcher_version: String,
    ignored_launcher_version
);
define_save_config_command!(
    save_theme_preference,
    theme_preference: ThemePreference,
    theme_preference
);
define_save_config_command!(
    save_reduce_motion_preference,
    reduce_motion_preference: ReduceMotionPreference,
    reduce_motion_preference
);

#[tauri::command]
pub fn save_language_preference(
    language_preference: LanguagePreference,
    app_handle: AppHandle,
) -> Result<()> {
    with_config_mut(move |config| {
        config.language_preference = language_preference;
        Ok(())
    })?;

    crate::tray::refresh_tray_menu(&app_handle);
    Ok(())
}

#[tauri::command]
pub fn save_tray_style(tray_style: TrayStyle, app_handle: AppHandle) -> Result<()> {
    let previous = load_config()?.tray_style;

    with_config_mut(move |config| {
        config.tray_style = tray_style;
        Ok(())
    })?;

    let Err(error) = crate::tray::apply_tray_style(&app_handle, tray_style) else {
        return Ok(());
    };

    // The style is already stored, and applying it removes the old tray icon
    // before building the new one, so a failure here would leave the session
    // without a tray icon while the frontend rolls its optimistic update back.
    // Restore both the stored value and the previous icon.
    if let Err(rollback_error) = with_config_mut(move |config| {
        config.tray_style = previous;
        Ok(())
    }) {
        log::error!(
            "Failed to roll back the tray style after a failed switch: {}",
            rollback_error
        );
    }
    if let Err(restore_error) = crate::tray::apply_tray_style(&app_handle, previous) {
        log::error!(
            "Failed to restore the previous tray icon after a failed switch: {}",
            restore_error
        );
    }

    Err(error)
}
