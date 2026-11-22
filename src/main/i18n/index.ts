import { createI18n } from "vue-i18n";
import type { LanguagePreference, SupportedLocale } from "../types/launcher";
import { cacheLanguage, readCachedLanguage } from "../utils/languageCache";
import { isSupportedLocale, resolveLanguagePreference } from "./locale";
import enUS from "./locales/en-US";
import jaJP from "./locales/ja-JP";
import ruRU from "./locales/ru-RU";
import zhCN from "./locales/zh-CN";

export const initialLocale: SupportedLocale = resolveLanguagePreference(
  readCachedLanguage(),
);

export const i18n = createI18n({
  legacy: false,
  globalInjection: false,
  locale: initialLocale,
  fallbackLocale: "zh-CN",
  messages: {
    "zh-CN": zhCN,
    "en-US": enUS,
    "ru-RU": ruRU,
    "ja-JP": jaJP,
  },
  missingWarn: import.meta.env.DEV,
  fallbackWarn: import.meta.env.DEV,
});

export function getCurrentLocale(): SupportedLocale {
  const current = i18n.global.locale.value;
  return isSupportedLocale(current) ? current : "zh-CN";
}

export function setLocale(locale: SupportedLocale): void {
  if (i18n.global.locale.value !== locale) {
    i18n.global.locale.value = locale;
  }
  document.documentElement.lang = locale;
}

export function applyLanguagePreference(
  preference: LanguagePreference,
): SupportedLocale {
  const locale = resolveLanguagePreference(preference);
  setLocale(locale);
  cacheLanguage(preference);
  return locale;
}
