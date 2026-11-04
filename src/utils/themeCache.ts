import type { ThemePreference } from "../types/launcher";

const THEME_CACHE_KEY = "launcher.theme";

function isThemePreference(value: string | null): value is ThemePreference {
  return value === "system" || value === "light" || value === "dark";
}

function readInjectedTheme(): ThemePreference | null {
  const injected = window.__ASTRBOT_THEME_PREFERENCE__;
  return typeof injected === "string" && isThemePreference(injected)
    ? injected
    : null;
}

export function readCachedTheme(): ThemePreference {
  const injected = readInjectedTheme();
  if (injected) {
    return injected;
  }

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
