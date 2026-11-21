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

    /// No dashboard login credentials are stored for the instance.
    pub(crate) fn dashboard_credentials_missing(self) -> String {
        match self {
            Self::ZhCn => "未配置 AstrBot 面板登录凭据，请在实例设置中填写用户名和密码。".to_string(),
            Self::EnUs => {
                "No AstrBot dashboard credentials are configured. Add the username and password in the instance settings."
                    .to_string()
            }
            Self::RuRu => {
                "Учётные данные панели AstrBot не заданы. Укажите имя пользователя и пароль в настройках экземпляра."
                    .to_string()
            }
            Self::JaJp => {
                "AstrBot ダッシュボードのログイン情報が未設定です。インスタンス設定でユーザー名とパスワードを入力してください。"
                    .to_string()
            }
        }
    }

    /// The dashboard rejected the stored username/password.
    pub(crate) fn dashboard_login_failed(self) -> String {
        match self {
            Self::ZhCn => "面板登录失败，请检查实例设置中的用户名和密码。".to_string(),
            Self::EnUs => {
                "Dashboard login failed. Check the username and password in the instance settings."
                    .to_string()
            }
            Self::RuRu => {
                "Не удалось войти в панель. Проверьте имя пользователя и пароль в настройках экземпляра."
                    .to_string()
            }
            Self::JaJp => {
                "ダッシュボードへのログインに失敗しました。インスタンス設定のユーザー名とパスワードを確認してください。"
                    .to_string()
            }
        }
    }

    /// Neither the desktop exchange nor stored credentials can produce a
    /// WebUI session; the embedded login form is the fallback.
    pub(crate) fn dashboard_webui_manual_login(self) -> String {
        match self {
            Self::ZhCn => {
                "免密登录不可用：请启动实例后在 WebUI 中手动登录，或在实例设置中填写面板用户名和密码。"
                    .to_string()
            }
            Self::EnUs => {
                "Passwordless sign-in is unavailable. Sign in manually in the WebUI, or configure the dashboard username and password in the instance settings."
                    .to_string()
            }
            Self::RuRu => {
                "Автоматический вход недоступен. Войдите в WebUI вручную или укажите имя пользователя и пароль панели в настройках экземпляра."
                    .to_string()
            }
            Self::JaJp => {
                "パスワードなしログインは利用できません。WebUI で手動ログインするか、インスタンス設定にダッシュボードのユーザー名とパスワードを入力してください。"
                    .to_string()
            }
        }
    }

    /// The account requires a TOTP code, which the launcher cannot supply.
    pub(crate) fn dashboard_mfa_unsupported(self) -> String {
        match self {
            Self::ZhCn => "该实例启用了两步验证（TOTP），暂不支持自动登录。".to_string(),
            Self::EnUs => {
                "This instance requires two-factor authentication (TOTP), which automatic login does not support yet."
                    .to_string()
            }
            Self::RuRu => {
                "На экземпляре включена двухфакторная аутентификация (TOTP); автоматический вход пока не поддерживается."
                    .to_string()
            }
            Self::JaJp => {
                "このインスタンスでは二段階認証（TOTP）が有効なため、自動ログインはまだ対応していません。"
                    .to_string()
            }
        }
    }

    /// The instance port could not be reached or the stream dropped.
    pub(crate) fn dashboard_unreachable(self) -> String {
        match self {
            Self::ZhCn => "与实例的连接中断，请确认实例已启动且端口可访问。".to_string(),
            Self::EnUs => "Lost connection to the instance. Make sure it is running and the port is accessible.".to_string(),
            Self::RuRu => "Соединение с экземпляром потеряно. Убедитесь, что он запущен и порт доступен.".to_string(),
            Self::JaJp => "インスタンスとの接続が切断されました。起動しているか、ポートがアクセス可能か確認してください。".to_string(),
        }
    }

    /// HTTP 404: the instance is too old for the dashboard chat API.
    pub(crate) fn dashboard_unsupported_version(self) -> String {
        match self {
            Self::ZhCn => "实例不支持该面板接口，请升级 AstrBot 后重试。".to_string(),
            Self::EnUs => {
                "The instance does not support this dashboard API. Update AstrBot and try again."
                    .to_string()
            }
            Self::RuRu => {
                "Экземпляр не поддерживает этот API панели. Обновите AstrBot и повторите попытку."
                    .to_string()
            }
            Self::JaJp => {
                "インスタンスはこのダッシュボード API に対応していません。AstrBot を更新してから再試行してください。"
                    .to_string()
            }
        }
    }

    /// HTTP 403: the account may not perform this request.
    pub(crate) fn dashboard_forbidden(self) -> String {
        match self {
            Self::ZhCn => "面板拒绝了该请求，请确认当前账号有权限访问。".to_string(),
            Self::EnUs => {
                "The dashboard rejected the request. Make sure the account is allowed to do this."
                    .to_string()
            }
            Self::RuRu => {
                "Панель отклонила запрос. Убедитесь, что у учётной записи есть нужные права."
                    .to_string()
            }
            Self::JaJp => {
                "ダッシュボードがリクエストを拒否しました。アカウントに権限があるか確認してください。"
                    .to_string()
            }
        }
    }

    /// Generic failure while processing a chat message.
    pub(crate) fn dashboard_stream_error(self) -> String {
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
