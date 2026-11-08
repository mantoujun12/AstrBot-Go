use crate::config::LanguagePreference;
use crate::process::InstanceState;

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

    pub(crate) fn tray_settings(self) -> &'static str {
        match self {
            Self::ZhCn => "打开设置",
            Self::EnUs => "Open Settings",
            Self::RuRu => "Открыть настройки",
            Self::JaJp => "設定を開く",
        }
    }

    pub(crate) fn tray_open_webui(self) -> &'static str {
        match self {
            Self::ZhCn => "打开 WebUI",
            Self::EnUs => "Open WebUI",
            Self::RuRu => "Открыть WebUI",
            Self::JaJp => "WebUI を開く",
        }
    }

    pub(crate) fn tray_start(self) -> &'static str {
        match self {
            Self::ZhCn => "启动",
            Self::EnUs => "Start",
            Self::RuRu => "Запустить",
            Self::JaJp => "起動",
        }
    }

    pub(crate) fn tray_restart(self) -> &'static str {
        match self {
            Self::ZhCn => "重启",
            Self::EnUs => "Restart",
            Self::RuRu => "Перезапустить",
            Self::JaJp => "再起動",
        }
    }

    pub(crate) fn tray_no_instances(self) -> &'static str {
        match self {
            Self::ZhCn => "暂无实例",
            Self::EnUs => "No instances",
            Self::RuRu => "Нет экземпляров",
            Self::JaJp => "インスタンスなし",
        }
    }

    pub(crate) fn tray_instances(self, total: usize) -> String {
        match self {
            Self::ZhCn => format!("实例（{total}）"),
            Self::EnUs => format!("Instances ({total})"),
            Self::RuRu => format!("Экземпляры ({total})"),
            Self::JaJp => format!("インスタンス（{total}）"),
        }
    }

    pub(crate) fn tray_status(self, running: usize, total: usize) -> String {
        match self {
            Self::ZhCn => format!("{running} 个运行中 · 共 {total} 个实例"),
            Self::EnUs => format!("{running} running · {total} total"),
            Self::RuRu => format!("Работает: {running} · всего: {total}"),
            Self::JaJp => format!("{running} 個実行中 · 全 {total} 個"),
        }
    }

    pub(crate) fn instance_state(self, state: InstanceState) -> &'static str {
        match state {
            InstanceState::Running => match self {
                Self::ZhCn => "运行中",
                Self::EnUs => "Running",
                Self::RuRu => "Работает",
                Self::JaJp => "実行中",
            },
            InstanceState::Starting => match self {
                Self::ZhCn => "启动中",
                Self::EnUs => "Starting",
                Self::RuRu => "Запуск",
                Self::JaJp => "起動中",
            },
            InstanceState::Stopping => match self {
                Self::ZhCn => "停止中",
                Self::EnUs => "Stopping",
                Self::RuRu => "Остановка",
                Self::JaJp => "停止中",
            },
            InstanceState::Stopped => match self {
                Self::ZhCn => "已停止",
                Self::EnUs => "Stopped",
                Self::RuRu => "Остановлен",
                Self::JaJp => "停止済み",
            },
        }
    }

    pub(crate) fn tray_duration_less_minute(self) -> &'static str {
        match self {
            Self::ZhCn => "< 1 分",
            Self::EnUs => "< 1 min",
            Self::RuRu => "< 1 мин",
            Self::JaJp => "1 分未満",
        }
    }

    pub(crate) fn tray_duration_minutes(self, minutes: i64) -> String {
        match self {
            Self::ZhCn => format!("{minutes} 分"),
            Self::EnUs => format!("{minutes} min"),
            Self::RuRu => format!("{minutes} мин"),
            Self::JaJp => format!("{minutes} 分"),
        }
    }

    pub(crate) fn tray_duration_hours(self, hours: i64) -> String {
        match self {
            Self::ZhCn => format!("{hours} 小时"),
            Self::EnUs => format!("{hours} h"),
            Self::RuRu => format!("{hours} ч"),
            Self::JaJp => format!("{hours} 時間"),
        }
    }

    pub(crate) fn tray_duration_hours_minutes(self, hours: i64, minutes: i64) -> String {
        match self {
            Self::ZhCn => format!("{hours} 小时 {minutes} 分"),
            Self::EnUs => format!("{hours} h {minutes} min"),
            Self::RuRu => format!("{hours} ч {minutes} мин"),
            Self::JaJp => format!("{hours} 時間 {minutes} 分"),
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
