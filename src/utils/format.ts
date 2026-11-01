import type { InstanceState } from "../types/launcher";

const datetimeFormatter = new Intl.DateTimeFormat("zh-CN", {
  year: "numeric",
  month: "2-digit",
  day: "2-digit",
  hour: "2-digit",
  minute: "2-digit",
  hour12: false,
});

const timeFormatter = new Intl.DateTimeFormat("zh-CN", {
  hour: "2-digit",
  minute: "2-digit",
  second: "2-digit",
  hour12: false,
});

function toDate(value: string): Date | null {
  if (value.trim() === "") {
    return null;
  }
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
}

export function formatDateTime(value: string): string {
  const date = toDate(value);
  return date ? datetimeFormatter.format(date) : "—";
}

export function formatLogTime(value: string): string {
  const date = toDate(value);
  return date ? timeFormatter.format(date) : "—";
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
  return `${value.toFixed(digits)} ${units[unitIndex]}`;
}

export function instanceStateLabel(state: InstanceState): string {
  switch (state) {
    case "running":
      return "运行中";
    case "starting":
      return "启动中";
    case "stopping":
      return "停止中";
    case "stopped":
      return "已停止";
  }
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
