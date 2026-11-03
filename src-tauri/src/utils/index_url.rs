pub(crate) fn normalize_default_index(pypi_mirror: &str) -> String {
    if pypi_mirror.trim().is_empty() {
        return "https://pypi.org/simple".to_string();
    }

    let mirror = pypi_mirror.trim().trim_end_matches('/');
    if mirror.ends_with("/simple") {
        mirror.to_string()
    } else {
        format!("{}/simple", mirror)
    }
}

/// Join a base URL and a path without duplicating or dropping the slash.
pub(crate) fn join_url(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

/// Wrap a URL with a proxy prefix.
/// If proxy is empty, returns the original URL unchanged.
pub(crate) fn wrap_with_proxy(proxy: &str, url: &str) -> String {
    if proxy.is_empty() {
        url.to_string()
    } else {
        let base = proxy.trim_end_matches('/');
        format!("{}/{}", base, url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_url_handles_both_slash_directions() {
        assert_eq!(
            join_url("https://example.com/simple/", "/pip/"),
            "https://example.com/simple/pip/"
        );
        assert_eq!(
            join_url("https://example.com/simple", "pip/"),
            "https://example.com/simple/pip/"
        );
        assert_eq!(
            join_url("https://example.com", "index.json"),
            "https://example.com/index.json"
        );
    }

    #[test]
    fn normalize_default_index_appends_simple_once() {
        assert_eq!(
            normalize_default_index("https://mirror.example.com"),
            "https://mirror.example.com/simple"
        );
        assert_eq!(
            normalize_default_index("https://mirror.example.com/simple/"),
            "https://mirror.example.com/simple"
        );
    }
}
