import { afterAll, beforeAll, describe, expect, it } from "vitest";

import { i18n } from "../i18n";
import {
  formatFileSize,
  formatLogTime,
  formatRunDuration,
  instanceStateColor,
  instanceStateLabel,
  stripAnsi,
} from "./format";

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
