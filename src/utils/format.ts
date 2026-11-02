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
