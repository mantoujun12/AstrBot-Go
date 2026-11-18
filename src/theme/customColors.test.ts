import { describe, expect, it } from "vitest";

import type { ThemeInstance } from "vuetify";

import {
  DEFAULT_THEME_COLORS,
  applyThemeColors,
  clearStoredThemeColors,
  isHexColor,
  readStoredThemeColors,
  restoreThemeColors,
  storeThemeColors,
} from "./customColors";

const PRIMARY_KEY = "launcher.theme.primary";
const SECONDARY_KEY = "launcher.theme.secondary";

function fakeTheme(): ThemeInstance {
  return {
    themes: {
      value: {
        light: {
          dark: false,
          colors: {
            primary: "#111111",
            secondary: "#222222",
            darkprimary: "#333333",
          },
        },
        dark: {
          dark: true,
          colors: { primary: "#444444", secondary: "#555555" },
        },
      },
    },
  } as unknown as ThemeInstance;
}

describe("isHexColor", () => {
  it("accepts 3- and 6-digit hex colors", () => {
    expect(isHexColor("#abc")).toBe(true);
    expect(isHexColor("#A1B2C3")).toBe(true);
    expect(isHexColor(" #abc ")).toBe(true);
  });

  it("rejects malformed colors", () => {
    expect(isHexColor("abc")).toBe(false);
    expect(isHexColor("#12345")).toBe(false);
    expect(isHexColor("#GGG")).toBe(false);
    expect(isHexColor("")).toBe(false);
  });
});

describe("stored theme colors", () => {
  it("round-trips valid colors", () => {
    clearStoredThemeColors();
    storeThemeColors({ primary: "#112233", secondary: "#445566" });
    expect(readStoredThemeColors()).toEqual({
      primary: "#112233",
      secondary: "#445566",
    });

    clearStoredThemeColors();
    expect(readStoredThemeColors()).toBe(null);
    expect(localStorage.getItem(PRIMARY_KEY)).toBe(null);
    expect(localStorage.getItem(SECONDARY_KEY)).toBe(null);
  });

  it("ignores invalid stored values", () => {
    localStorage.setItem(PRIMARY_KEY, "#12");
    localStorage.setItem(SECONDARY_KEY, "#445566");
    expect(readStoredThemeColors()).toBe(null);
    localStorage.removeItem(PRIMARY_KEY);
    localStorage.removeItem(SECONDARY_KEY);
  });
});

describe("applyThemeColors", () => {
  it("overrides colors in every theme definition", () => {
    const theme = fakeTheme();
    applyThemeColors(theme, { primary: "#aabbcc", secondary: "#ddeeff" });

    const light = theme.themes.value.light.colors;
    expect(light.primary).toBe("#aabbcc");
    expect(light.secondary).toBe("#ddeeff");
    expect(light.darkprimary).toBe("#aabbcc");

    const dark = theme.themes.value.dark.colors;
    expect(dark.primary).toBe("#aabbcc");
    expect(dark.secondary).toBe("#ddeeff");
  });
});

describe("restoreThemeColors", () => {
  it("restores the built-in defaults", () => {
    const theme = fakeTheme();
    restoreThemeColors(theme);

    const light = theme.themes.value.light.colors;
    expect(light.primary).toBe(DEFAULT_THEME_COLORS.primary);
    expect(light.secondary).toBe(DEFAULT_THEME_COLORS.secondary);
    expect(light.darkprimary).toBe("#1565c0");

    const dark = theme.themes.value.dark.colors;
    expect(dark.primary).toBe("#5ba4d4");
    expect(dark.secondary).toBe("#4a95c4");
  });
});
