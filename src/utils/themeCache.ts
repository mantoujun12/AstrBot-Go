import type { ThemePreference } from "../types/launcher";

const THEME_CACHE_KEY = "launcher.theme";

function isThemePreference(value: string | null): value is ThemePreference {
  return value === "system" || value === "light" || value === "dark";
}

export function readCachedTheme(): ThemePreference {
  try {
    const cached = localStorage.getItem(THEME_CACHE_KEY);
    return isThemePreference(cached) ? cached : "system";
  } catch {
    return "system";
  }
}

export function cacheTheme(theme: ThemePreference): void {
  try {
    localStorage.setItem(THEME_CACHE_KEY, theme);
  } catch {
    // Storage may be unavailable in restricted webviews; the backend remains
    // the source of truth on the next snapshot.
  }
}
