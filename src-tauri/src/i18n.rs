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

    /// No OpenAPI key is stored for the instance.
    pub(crate) fn openapi_key_missing(self) -> String {
        match self {
            Self::ZhCn => "未配置 OpenAPI API Key，请在实例设置的 API Key 中填写。".to_string(),
            Self::EnUs => {
                "No OpenAPI API key is configured. Add one in the instance's API Key setting."
                    .to_string()
            }
            Self::RuRu => {
                "API-ключ OpenAPI не задан. Добавьте его в поле «API Key» в настройках экземпляра."
                    .to_string()
            }
            Self::JaJp => {
                "OpenAPI API キーが未設定です。インスタンス設定の「API キー」に入力してください。"
                    .to_string()
            }
        }
    }

    /// HTTP 401: the key was rejected by AstrBot.
    pub(crate) fn openapi_invalid_key(self) -> String {
        match self {
            Self::ZhCn => "API Key 无效或已过期，请在 AstrBot WebUI 设置 → OpenAPI 中重新创建。".to_string(),
            Self::EnUs => "The API key is invalid or expired. Recreate it in AstrBot WebUI Settings > OpenAPI.".to_string(),
            Self::RuRu => "API-ключ недействителен или истёк. Создайте его заново в веб-интерфейсе AstrBot: Настройки > OpenAPI.".to_string(),
            Self::JaJp => "API キーが無効または期限切れです。AstrBot WebUI の設定 → OpenAPI で再作成してください。".to_string(),
        }
    }

    /// HTTP 403: the key lacks the required scope.
    pub(crate) fn openapi_insufficient_scope(self, scope: &str) -> String {
        match self {
            Self::ZhCn => format!("API Key 缺少 {scope} 权限，请在 AstrBot WebUI 设置 → OpenAPI 勾选后重新创建。"),
            Self::EnUs => format!("The API key lacks the {scope} scope. Recreate it with that scope in AstrBot WebUI Settings > OpenAPI."),
            Self::RuRu => format!("У API-ключа нет области {scope}. Создайте его заново с этой областью в веб-интерфейсе AstrBot: Настройки > OpenAPI."),
            Self::JaJp => format!("API キーに {scope} スコープがありません。AstrBot WebUI の設定 → OpenAPI でスコープを付与して再作成してください。"),
        }
    }

    /// The instance port could not be reached or the stream dropped.
    pub(crate) fn openapi_unreachable(self) -> String {
        match self {
            Self::ZhCn => "与实例的连接中断，请确认实例已启动且端口可访问。".to_string(),
            Self::EnUs => "Lost connection to the instance. Make sure it is running and the port is accessible.".to_string(),
            Self::RuRu => "Соединение с экземпляром потеряно. Убедитесь, что он запущен и порт доступен.".to_string(),
            Self::JaJp => "インスタンスとの接続が切断されました。起動しているか、ポートがアクセス可能か確認してください。".to_string(),
        }
    }

    /// HTTP 404: the AstrBot version predates the OpenAPI.
    pub(crate) fn openapi_unsupported_version(self) -> String {
        match self {
            Self::ZhCn => "实例不支持 OpenAPI（需要 AstrBot ≥ v4.18.0）。".to_string(),
            Self::EnUs => {
                "The instance does not support OpenAPI (requires AstrBot >= v4.18.0).".to_string()
            }
            Self::RuRu => {
                "Экземпляр не поддерживает OpenAPI (требуется AstrBot >= v4.18.0).".to_string()
            }
            Self::JaJp => {
                "インスタンスは OpenAPI をサポートしていません（AstrBot >= v4.18.0 が必要）。"
                    .to_string()
            }
        }
    }

    /// Generic failure while processing a chat message.
    pub(crate) fn openapi_stream_error(self) -> String {
        match self {
            Self::ZhCn => "AstrBot 处理消息失败。".to_string(),
            Self::EnUs => "AstrBot failed to process the message.".to_string(),
            Self::RuRu => "AstrBot не смог обработать сообщение.".to_string(),
            Self::JaJp => "AstrBot がメッセージを処理できませんでした。".to_string(),
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
