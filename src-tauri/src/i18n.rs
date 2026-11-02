use crate::config::LanguagePreference;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Locale {
    #[default]
    ZhCn,
    EnUs,
    RuRu,
    JaJp,
}

impl Locale {
    pub(crate) fn from_tag(tag: &str) -> Option<Self> {
        let normalized = tag.trim().to_ascii_lowercase();
        if normalized.starts_with("zh") {
            Some(Self::ZhCn)
        } else if normalized.starts_with("en") {
            Some(Self::EnUs)
        } else if normalized.starts_with("ru") {
            Some(Self::RuRu)
        } else if normalized.starts_with("ja") {
            Some(Self::JaJp)
        } else {
            None
        }
    }

    pub(crate) fn detect() -> Self {
        sys_locale::get_locale()
            .as_deref()
            .and_then(Self::from_tag)
            .unwrap_or_default()
    }

    pub(crate) fn resolve(preference: LanguagePreference) -> Self {
        match preference {
            LanguagePreference::System => Self::detect(),
            LanguagePreference::ZhCn => Self::ZhCn,
            LanguagePreference::EnUs => Self::EnUs,
            LanguagePreference::RuRu => Self::RuRu,
            LanguagePreference::JaJp => Self::JaJp,
        }
    }

    pub(crate) fn tray_show(self) -> &'static str {
        match self {
            Self::ZhCn => "显示窗口",
            Self::EnUs => "Show Window",
            Self::RuRu => "Показать окно",
            Self::JaJp => "ウィンドウを表示",
        }
    }

    pub(crate) fn tray_quit(self) -> &'static str {
        match self {
            Self::ZhCn => "退出",
            Self::EnUs => "Quit",
            Self::RuRu => "Выход",
            Self::JaJp => "終了",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_tags_match_language_prefixes() {
        assert_eq!(Locale::from_tag("zh-CN"), Some(Locale::ZhCn));
        assert_eq!(Locale::from_tag("zh-Hant"), Some(Locale::ZhCn));
        assert_eq!(Locale::from_tag("en-GB"), Some(Locale::EnUs));
        assert_eq!(Locale::from_tag("ru-RU"), Some(Locale::RuRu));
        assert_eq!(Locale::from_tag("ja-JP"), Some(Locale::JaJp));
        assert_eq!(Locale::from_tag("de-DE"), None);
    }
}
