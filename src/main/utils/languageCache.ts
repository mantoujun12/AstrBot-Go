import { isLanguagePreference } from "../i18n/locale";
import type { LanguagePreference } from "../types/launcher";

const LANGUAGE_CACHE_KEY = "launcher.language";

export function readCachedLanguage(): LanguagePreference {
  try {
    const value = localStorage.getItem(LANGUAGE_CACHE_KEY);
    return isLanguagePreference(value) ? value : "system";
  } catch {
    return "system";
  }
}

export function cacheLanguage(preference: LanguagePreference): void {
  try {
    localStorage.setItem(LANGUAGE_CACHE_KEY, preference);
  } catch {
    // A missing cache only costs one boot-time flash; the backend config
    // remains the source of truth.
  }
}
