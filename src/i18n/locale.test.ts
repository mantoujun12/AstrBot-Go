import { afterEach, describe, expect, it } from "vitest";

import type { SupportedLocale } from "../types/launcher";
import {
  isLanguagePreference,
  isSupportedLocale,
  normalizeLocaleTag,
  resolveLanguagePreference,
  resolveSystemLocale,
} from "./locale";

function stubNavigatorLanguages(tags: readonly string[]): void {
  Object.defineProperty(navigator, "languages", {
    value: [...tags],
    configurable: true,
  });
  Object.defineProperty(navigator, "language", {
    value: tags[0] ?? "en-US",
    configurable: true,
  });
}

describe("isSupportedLocale", () => {
  it("accepts the supported locale tags", () => {
    for (const tag of ["zh-CN", "en-US", "ru-RU", "ja-JP"] as const) {
      expect(isSupportedLocale(tag)).toBe(true);
    }
  });

  it("rejects unknown tags and non-strings", () => {
    expect(isSupportedLocale("fr-FR")).toBe(false);
    expect(isSupportedLocale("zh_cn")).toBe(false);
    expect(isSupportedLocale(42)).toBe(false);
    expect(isSupportedLocale(null)).toBe(false);
  });
});

describe("isLanguagePreference", () => {
  it("accepts system and supported locales", () => {
    expect(isLanguagePreference("system")).toBe(true);
    expect(isLanguagePreference("ja-JP")).toBe(true);
  });

  it("rejects other values", () => {
    expect(isLanguagePreference("auto")).toBe(false);
    expect(isLanguagePreference("")).toBe(false);
  });
});

describe("normalizeLocaleTag", () => {
  it("maps language prefixes to the supported locales", () => {
    expect(normalizeLocaleTag("zh")).toBe("zh-CN");
    expect(normalizeLocaleTag("en-US")).toBe("en-US");
    expect(normalizeLocaleTag("RU-ru")).toBe("ru-RU");
    expect(normalizeLocaleTag("ja_JP")).toBe("ja-JP");
  });

  it("trims surrounding whitespace", () => {
    expect(normalizeLocaleTag("  en  ")).toBe("en-US");
  });

  it("returns null for unsupported languages", () => {
    expect(normalizeLocaleTag("fr-FR")).toBe(null);
    expect(normalizeLocaleTag("")).toBe(null);
  });
});

describe("resolveSystemLocale", () => {
  afterEach(() => {
    stubNavigatorLanguages(["zh-CN"]);
  });

  it("prefers the first match among navigator languages", () => {
    stubNavigatorLanguages(["fr-FR", "zh-TW"]);
    expect(resolveSystemLocale()).toBe("zh-CN");

    stubNavigatorLanguages(["fr-FR", "en-GB"]);
    expect(resolveSystemLocale()).toBe("en-US");
  });

  it("falls back to zh-CN when nothing matches", () => {
    stubNavigatorLanguages(["fr-FR", "de-DE"]);
    expect(resolveSystemLocale()).toBe("zh-CN");
  });
});

describe("resolveLanguagePreference", () => {
  it("returns an explicit locale unchanged", () => {
    expect(resolveLanguagePreference("ru-RU" as SupportedLocale)).toBe("ru-RU");
  });

  it("resolves the system preference", () => {
    stubNavigatorLanguages(["fr-FR", "ja-JP"]);
    expect(resolveLanguagePreference("system")).toBe("ja-JP");
    stubNavigatorLanguages(["zh-CN"]);
  });
});
