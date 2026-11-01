import type { ThemeInstance } from "vuetify";
import { PurpleTheme } from "./LightTheme";
import { PurpleThemeDark } from "./DarkTheme";

export interface CustomThemeColors {
  primary: string;
  secondary: string;
}

interface ThemeDefaultColors extends CustomThemeColors {
  darkprimary?: string;
  darksecondary?: string;
}

const PRIMARY_STORAGE_KEY = "launcher.theme.primary";
const SECONDARY_STORAGE_KEY = "launcher.theme.secondary";

const HEX_COLOR_PATTERN = /^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/;

function readColor(value: unknown, fallback: string): string {
  return typeof value === "string" ? value : fallback;
}

const LIGHT_DEFAULTS: ThemeDefaultColors = {
  primary: readColor(PurpleTheme.colors?.primary, "#3c96ca"),
  secondary: readColor(PurpleTheme.colors?.secondary, "#2f86bd"),
  darkprimary: readColor(PurpleTheme.colors?.darkprimary, "#1565c0"),
  darksecondary: readColor(PurpleTheme.colors?.darksecondary, "#236b99"),
};

const DARK_DEFAULTS: ThemeDefaultColors = {
  primary: readColor(PurpleThemeDark.colors?.primary, "#5ba4d4"),
  secondary: readColor(PurpleThemeDark.colors?.secondary, "#4a95c4"),
  darkprimary: readColor(PurpleThemeDark.colors?.darkprimary, "#3a8ab8"),
  darksecondary: readColor(PurpleThemeDark.colors?.darksecondary, "#3a8ab8"),
};

const THEME_DEFAULTS: Record<string, ThemeDefaultColors> = {
  light: LIGHT_DEFAULTS,
  dark: DARK_DEFAULTS,
};

export const DEFAULT_THEME_COLORS: CustomThemeColors = {
  primary: LIGHT_DEFAULTS.primary,
  secondary: LIGHT_DEFAULTS.secondary,
};

export function isHexColor(value: string): boolean {
  return HEX_COLOR_PATTERN.test(value.trim());
}

function readStorage(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeStorage(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // The colors still apply for the current session when storage is blocked.
  }
}

function removeStorage(key: string): void {
  try {
    localStorage.removeItem(key);
  } catch {
    // Ignore blocked storage.
  }
}

export function readStoredThemeColors(): CustomThemeColors | null {
  const primary = readStorage(PRIMARY_STORAGE_KEY);
  const secondary = readStorage(SECONDARY_STORAGE_KEY);
  if (
    primary === null ||
    secondary === null ||
    !isHexColor(primary) ||
    !isHexColor(secondary)
  ) {
    return null;
  }
  return { primary, secondary };
}

export function storeThemeColors(colors: CustomThemeColors): void {
  writeStorage(PRIMARY_STORAGE_KEY, colors.primary);
  writeStorage(SECONDARY_STORAGE_KEY, colors.secondary);
}

export function clearStoredThemeColors(): void {
  removeStorage(PRIMARY_STORAGE_KEY);
  removeStorage(SECONDARY_STORAGE_KEY);
}

export function applyThemeColors(
  theme: ThemeInstance,
  colors: CustomThemeColors,
): void {
  for (const definition of Object.values(theme.themes.value)) {
    const palette = definition.colors;
    palette.primary = colors.primary;
    palette.secondary = colors.secondary;
    if (typeof palette.darkprimary === "string") {
      palette.darkprimary = colors.primary;
    }
    if (typeof palette.darksecondary === "string") {
      palette.darksecondary = colors.secondary;
    }
  }
}

export function restoreThemeColors(theme: ThemeInstance): void {
  for (const [name, definition] of Object.entries(theme.themes.value)) {
    const palette = definition.colors;
    const defaults = THEME_DEFAULTS[name] ?? LIGHT_DEFAULTS;
    palette.primary = defaults.primary;
    palette.secondary = defaults.secondary;
    if (typeof palette.darkprimary === "string") {
      palette.darkprimary = defaults.darkprimary ?? defaults.primary;
    }
    if (typeof palette.darksecondary === "string") {
      palette.darksecondary = defaults.darksecondary ?? defaults.secondary;
    }
  }
}
