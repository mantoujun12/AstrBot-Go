use std::path::PathBuf;

/// Convert an archive entry path to a relative PathBuf, rejecting empty, traversal,
/// or control-character-containing paths.
pub(crate) fn parse_entry_rel_path(raw: &str) -> Option<PathBuf> {
    if raw.chars().any(|c| c.is_control()) {
        return None;
    }

    let normalized = raw.replace('\\', "/");
    let first = normalized.split('/').next()?;
    let bytes = normalized.as_bytes();
    let has_windows_drive_prefix =
        bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic();
    if first.is_empty() || has_windows_drive_prefix {
        return None;
    }

    let mut relative = PathBuf::new();

    for part in normalized.split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            _ => relative.push(part),
        }
    }

    if relative.as_os_str().is_empty() {
        return None;
    }

    Some(relative)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_and_absolute_paths() {
        assert!(parse_entry_rel_path("../escape").is_none());
        assert!(parse_entry_rel_path("data/../../escape").is_none());
        assert!(parse_entry_rel_path("data/../escape").is_none());
        assert!(parse_entry_rel_path("/absolute/path").is_none());
        assert!(parse_entry_rel_path("C:/Windows/System32").is_none());
        assert!(parse_entry_rel_path("data\\..\\escape").is_none());
        assert!(parse_entry_rel_path("data/\u{0}bad").is_none());
    }

    #[test]
    fn accepts_normal_relative_paths() {
        assert_eq!(
            parse_entry_rel_path("data/config.json"),
            Some(PathBuf::from("data").join("config.json"))
        );
        assert_eq!(
            parse_entry_rel_path("./data//config.json"),
            Some(PathBuf::from("data").join("config.json"))
        );
        assert_eq!(
            parse_entry_rel_path("data/配置.json"),
            Some(PathBuf::from("data").join("配置.json"))
        );
    }

    #[test]
    fn rejects_empty_unc_and_control_character_paths() {
        assert!(parse_entry_rel_path("").is_none());
        assert!(parse_entry_rel_path("//server/share/file.txt").is_none());
        assert!(parse_entry_rel_path("C:relative").is_none());
        assert!(parse_entry_rel_path("data/\u{7f}bad").is_none());
        assert!(parse_entry_rel_path("data/foo/..").is_none());
        assert!(parse_entry_rel_path("./").is_none());
    }
}
