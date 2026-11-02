import type { LanguagePreference, SupportedLocale } from "../types/launcher";

export const SUPPORTED_LOCALES = [
  "zh-CN",
  "en-US",
  "ru-RU",
  "ja-JP",
] as const;

export function isSupportedLocale(value: unknown): value is SupportedLocale {
  return (
    typeof value === "string" &&
    (SUPPORTED_LOCALES as readonly string[]).includes(value)
  );
}

export function isLanguagePreference(
  value: unknown,
): value is LanguagePreference {
  return value === "system" || isSupportedLocale(value);
}

export function normalizeLocaleTag(tag: string): SupportedLocale | null {
  const normalized = tag.trim().toLowerCase();
  if (normalized.startsWith("zh")) {
    return "zh-CN";
  }
  if (normalized.startsWith("en")) {
    return "en-US";
  }
  if (normalized.startsWith("ru")) {
    return "ru-RU";
  }
  if (normalized.startsWith("ja")) {
    return "ja-JP";
  }
  return null;
}

export function resolveSystemLocale(): SupportedLocale {
  const candidates = [...(navigator.languages ?? []), navigator.language];
  for (const tag of candidates) {
    if (typeof tag !== "string") {
      continue;
    }
    const match = normalizeLocaleTag(tag);
    if (match !== null) {
      return match;
    }
  }
  return "zh-CN";
}

export function resolveLanguagePreference(
  preference: LanguagePreference,
): SupportedLocale {
  return preference === "system" ? resolveSystemLocale() : preference;
}
