import { i18n } from "../i18n";
import type { InstanceState } from "../types/launcher";

const DATETIME_OPTIONS: Intl.DateTimeFormatOptions = {
  year: "numeric",
  month: "2-digit",
  day: "2-digit",
  hour: "2-digit",
  minute: "2-digit",
  hour12: false,
};

const TIME_OPTIONS: Intl.DateTimeFormatOptions = {
  hour: "2-digit",
  minute: "2-digit",
  second: "2-digit",
  hour12: false,
};

interface LocaleFormatters {
  locale: string;
  datetime: Intl.DateTimeFormat;
  time: Intl.DateTimeFormat;
}

let localeFormatters: LocaleFormatters | null = null;
const numberFormatters = new Map<string, Intl.NumberFormat>();

function currentLocale(): string {
  return i18n.global.locale.value;
}

function getFormatters(): LocaleFormatters {
  const locale = currentLocale();
  if (localeFormatters === null || localeFormatters.locale !== locale) {
    localeFormatters = {
      locale,
      datetime: new Intl.DateTimeFormat(locale, DATETIME_OPTIONS),
      time: new Intl.DateTimeFormat(locale, TIME_OPTIONS),
    };
  }
  return localeFormatters;
}

function formatNumber(value: number, digits: number): string {
  const locale = currentLocale();
  const cacheKey = `${locale}:${digits}`;
  let formatter = numberFormatters.get(cacheKey);
  if (!formatter) {
    formatter = new Intl.NumberFormat(locale, {
      minimumFractionDigits: digits,
      maximumFractionDigits: digits,
    });
    numberFormatters.set(cacheKey, formatter);
  }
  return formatter.format(value);
}

function toDate(value: string): Date | null {
  if (value.trim() === "") {
    return null;
  }
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
}

export function formatDateTime(value: string): string {
  const date = toDate(value);
  return date ? getFormatters().datetime.format(date) : "—";
}

export function formatLogTime(value: string): string {
  const date = toDate(value);
  return date ? getFormatters().time.format(date) : "—";
}

export function formatFileSize(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "—";
  }

  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unitIndex = 0;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  const digits = unitIndex === 0 ? 0 : 1;
  return `${formatNumber(value, digits)} ${units[unitIndex]}`;
}

export function instanceStateLabel(state: InstanceState): string {
  return i18n.global.t("common.instanceState." + state);
}

export function formatRunDuration(startedAtMs: number, now: number): string {
  const elapsedMinutes = Math.floor((now - startedAtMs) / 60_000);
  if (elapsedMinutes < 1) {
    return i18n.global.t("common.duration.lessThanMinute");
  }
  if (elapsedMinutes < 60) {
    return i18n.global.t("common.duration.minutes", { minutes: elapsedMinutes });
  }

  const hours = Math.floor(elapsedMinutes / 60);
  const minutes = elapsedMinutes % 60;
  if (minutes === 0) {
    return i18n.global.t("common.duration.hours", { hours });
  }
  return i18n.global.t("common.duration.hoursMinutes", { hours, minutes });
}

export function instanceStateColor(
  state: InstanceState,
): "success" | "info" | "warning" | "secondary" {
  switch (state) {
    case "running":
      return "success";
    case "starting":
      return "info";
    case "stopping":
      return "warning";
    case "stopped":
      return "secondary";
  }
}

const ANSI_PATTERN = /\u001B\[[0-?]*[ -/]*[@-~]/g;

export function stripAnsi(text: string): string {
  return text.replace(ANSI_PATTERN, "");
}

// In-source tests, stripped from the production bundle by the
// `import.meta.vitest` define in vite.config.ts.
if (import.meta.vitest) {
  const { afterAll, beforeAll, describe, expect, it } = import.meta.vitest;
  const initialLocale = i18n.global.locale.value;

  beforeAll(() => {
    i18n.global.locale.value = "zh-CN";
  });

  afterAll(() => {
    i18n.global.locale.value = initialLocale;
  });

  describe("formatFileSize", () => {
    it("returns a dash for zero, negative, and non-finite sizes", () => {
      expect(formatFileSize(0)).toBe("—");
      expect(formatFileSize(-10)).toBe("—");
      expect(formatFileSize(Number.NaN)).toBe("—");
      expect(formatFileSize(Number.POSITIVE_INFINITY)).toBe("—");
    });

    it("scales bytes up to gigabytes", () => {
      expect(formatFileSize(512)).toBe("512 B");
      expect(formatFileSize(2048)).toBe("2.0 KB");
      expect(formatFileSize(1.5 * 1024 ** 2)).toBe("1.5 MB");
      expect(formatFileSize(5 * 1024 ** 3)).toBe("5.0 GB");
    });
  });

  describe("formatLogTime", () => {
    it("formats a valid timestamp with hours, minutes, and seconds", () => {
      expect(formatLogTime("2026-11-18T10:30:45Z")).toMatch(/^\d{2}:\d{2}:\d{2}$/);
    });

    it("returns a dash for empty or invalid timestamps", () => {
      expect(formatLogTime("")).toBe("—");
      expect(formatLogTime("   ")).toBe("—");
      expect(formatLogTime("not-a-date")).toBe("—");
    });
  });

  describe("formatRunDuration", () => {
    it("describes sub-minute runs", () => {
      expect(formatRunDuration(0, 30_000)).toBe("< 1 分");
    });

    it("describes minute and hour runs", () => {
      expect(formatRunDuration(0, 120_000)).toBe("2 分");
      expect(formatRunDuration(0, 120 * 60_000)).toBe("2 小时");
      expect(formatRunDuration(0, 125 * 60_000)).toBe("2 小时 5 分");
    });
  });

  describe("instanceStateLabel", () => {
    it("translates each state", () => {
      expect(instanceStateLabel("running")).toBe("运行中");
      expect(instanceStateLabel("stopped")).toBe("已停止");
    });
  });

  describe("instanceStateColor", () => {
    it("maps every state to a semantic color", () => {
      expect(instanceStateColor("running")).toBe("success");
      expect(instanceStateColor("starting")).toBe("info");
      expect(instanceStateColor("stopping")).toBe("warning");
      expect(instanceStateColor("stopped")).toBe("secondary");
    });
  });

  describe("stripAnsi", () => {
    it("removes ANSI escape sequences", () => {
      expect(stripAnsi("\u001B[31mred\u001B[0m text")).toBe("red text");
      expect(stripAnsi("plain")).toBe("plain");
    });
  });
}
