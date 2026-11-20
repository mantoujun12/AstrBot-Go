// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// Island i18n: port of the dashboard's composables, self-contained (the
// locale chunks live next to this file). Locale switching is driven by the
// host app, so setLocale is only used by the island's own settings dialog.

import { computed, ref } from "vue";

export type IslandLocale = "zh-CN" | "en-US" | "ru-RU" | "ja-JP";
export type IslandLanguagePreference = "system" | IslandLocale;

const currentLocale = ref<IslandLocale>("zh-CN");
const currentPreference = ref<IslandLanguagePreference>("zh-CN");
const translations = ref<Record<string, any>>({});

const localeLoaders: Record<IslandLocale, () => Promise<Record<string, any>>> = {
  "zh-CN": () => import("./locales/zh-CN").then((m) => m.default),
  "en-US": () => import("./locales/en-US").then((m) => m.default),
  "ru-RU": () => import("./locales/ru-RU").then((m) => m.default),
  "ja-JP": () => import("./locales/ja-JP").then((m) => m.default),
};

async function loadTranslations(locale: IslandLocale) {
  try {
    translations.value = await (localeLoaders[locale] || localeLoaders["zh-CN"])();
  } catch (error) {
    console.error(`Failed to load chatui translations for ${locale}:`, error);
    translations.value = await localeLoaders["zh-CN"]();
  }
}

/** Load the island translations for a locale chosen by the host app. */
export async function initChatI18n(
  locale: IslandLocale,
  preference: IslandLanguagePreference = locale,
) {
  currentPreference.value = preference;
  currentLocale.value = locale;
  document.documentElement.lang = locale;
  await loadTranslations(locale);
}

export function useI18n() {
  const t = (key: string, params?: Record<string, string | number>): string => {
    const keys = key.split(".");
    let value: any = translations.value;
    for (const k of keys) {
      if (value && typeof value === "object" && k in value) {
        value = value[k];
      } else {
        return `[MISSING: ${key}]`;
      }
    }
    if (typeof value !== "string") {
      return `[INVALID: ${key}]`;
    }
    let result: string = value;
    if (params) {
      result = result.replace(/\{(\w+)\}/g, (match, paramKey) =>
        params[paramKey]?.toString() || match,
      );
    }
    return result;
  };

  // The island follows the host locale; switching from inside the island
  // forwards to the host callback registered by ChatUIHost.
  const setLocale = async (preference: IslandLanguagePreference) => {
    currentPreference.value = preference;
    // "system" has no message chunk: the host resolves and pushes the locale.
    if (preference !== "system") {
      currentLocale.value = preference;
      document.documentElement.lang = preference;
      await loadTranslations(preference);
    }
    registeredLocaleSwitch?.(preference);
  };

  return {
    t,
    locale: computed(() => currentLocale.value),
    setLocale,
    availableLocales: ["zh-CN", "en-US", "ru-RU", "ja-JP"] as IslandLocale[],
  };
}

let registeredLocaleSwitch:
  | ((preference: IslandLanguagePreference) => void)
  | null = null;

/** ChatUIHost registers the callback that applies locale changes app-wide. */
export function registerLocaleSwitch(
  handler: (preference: IslandLanguagePreference) => void,
) {
  registeredLocaleSwitch = handler;
}

export function useModuleI18n(moduleName: string) {
  const { t } = useI18n();
  const tm = (key: string, params?: Record<string, string | number>): string =>
    t(`${moduleName.replace(/\//g, ".")}.${key}`, params);
  return { tm };
}

export function useLanguageSwitcher() {
  const { locale, setLocale, availableLocales } = useI18n();
  const { tm } = useModuleI18n("features/chat");
  const languageOptions = computed(() => [
    { value: "system", label: tm("settings.system"), flag: "🖥️" },
    { value: "zh-CN", label: "简体中文", flag: "🇨🇳" },
    { value: "en-US", label: "English", flag: "🇺🇸" },
    { value: "ru-RU", label: "Русский", flag: "🇷🇺" },
    { value: "ja-JP", label: "日本語", flag: "🇯🇵" },
  ]);
  return {
    locale,
    languagePreference: computed(() => currentPreference.value),
    languageOptions,
    currentLanguage: computed(() =>
      languageOptions.value.find(
        (lang) => lang.value === currentPreference.value,
      ),
    ),
    switchLanguage: setLocale,
    availableLocales,
  };
}
