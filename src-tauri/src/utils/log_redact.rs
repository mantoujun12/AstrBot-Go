//! Helpers for keeping local paths, URL credentials, and query parameters out
//! of diagnostic logs.

use std::path::Path;

const UNKNOWN_PATH: &str = "<path>";
const INVALID_URL: &str = "<invalid-url>";

/// Return only the final path component, which is enough to identify a file
/// without exposing the user's directory layout.
pub(crate) fn path_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| UNKNOWN_PATH.to_string())
}

/// Return a URL with credentials, query parameters, and fragments removed.
///
/// Downloads and proxy settings may carry signed query strings or passwords, so
/// diagnostics keep only the scheme, host, port, and path.
pub(crate) fn url_for_log(raw: &str) -> String {
    let Ok(mut url) = reqwest::Url::parse(raw) else {
        return INVALID_URL.to_string();
    };

    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

/// Redact every URL embedded in an error message.
///
/// Library errors such as `reqwest` failures may echo the request URL. This
/// keeps the surrounding diagnostic text while stripping credentials and
/// query parameters from those URLs.
pub(crate) fn redact_text(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut remaining = text;

    while let Some(separator) = remaining.find("://") {
        let scheme_start = remaining[..separator]
            .char_indices()
            .rev()
            .find(|(_, ch)| !(ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')))
            .map(|(index, ch)| index + ch.len_utf8())
            .unwrap_or(0);

        output.push_str(&remaining[..scheme_start]);

        let candidate = &remaining[scheme_start..];
        let end = candidate
            .char_indices()
            .find(|(_, ch)| {
                ch.is_whitespace()
                    || matches!(ch, '"' | '\'' | '<' | '>' | ')' | ']' | '}' | ',' | ';')
            })
            .map(|(index, _)| index)
            .unwrap_or(candidate.len());

        if end == 0 {
            let mut chars = candidate.chars();
            if let Some(ch) = chars.next() {
                output.push(ch);
                remaining = chars.as_str();
                continue;
            }
            break;
        }

        output.push_str(&url_for_log(&candidate[..end]));
        remaining = &candidate[end..];
    }

    output.push_str(remaining);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_name_keeps_only_the_final_component() {
        assert_eq!(path_name(Path::new("/home/alice/secret.zip")), "secret.zip");
        assert_eq!(path_name(Path::new("relative/dir/file.txt")), "file.txt");
        assert_eq!(path_name(Path::new("")), UNKNOWN_PATH);
    }

    #[test]
    fn urls_drop_credentials_query_and_fragment() {
        assert_eq!(
            url_for_log("https://alice:secret@example.com/api/file.zip?token=abc#frag"),
            "https://example.com/api/file.zip"
        );
        assert_eq!(url_for_log("not a url"), INVALID_URL);
    }

    #[test]
    fn text_redaction_replaces_embedded_urls() {
        assert_eq!(
            redact_text(
                "request failed for https://alice:secret@example.com/a?token=x#frag (timeout)"
            ),
            "request failed for https://example.com/a (timeout)"
        );
        assert_eq!(redact_text("no urls here"), "no urls here");
    }
}
