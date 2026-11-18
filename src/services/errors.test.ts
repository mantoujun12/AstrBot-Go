import { afterAll, beforeAll, describe, expect, it } from "vitest";

import { i18n } from "../i18n";
import { isDownloadCancelled, parseBackendError, toUserMessage } from "./errors";

const initialLocale = i18n.global.locale.value;

beforeAll(() => {
  i18n.global.locale.value = "zh-CN";
});

afterAll(() => {
  i18n.global.locale.value = initialLocale;
});

describe("parseBackendError", () => {
  it("translates known backend codes", () => {
    expect(parseBackendError({ code: 1001, payload: {} })).toEqual({
      message: "实例不存在",
      lock: null,
    });
    expect(parseBackendError({ code: 1006, payload: {} }).message).toBe(
      "下载已取消",
    );
  });

  it("appends payload details for detail-aware codes", () => {
    expect(
      parseBackendError({ code: 9999, payload: { detail: "boom" } }).message,
    ).toBe("操作失败：boom");
    expect(parseBackendError({ code: 9999, payload: {} }).message).toBe(
      "操作失败",
    );
  });

  it("uses the fallback message for unknown codes", () => {
    expect(
      parseBackendError({ code: 1234, payload: { detail: "oops" } }).message,
    ).toBe("操作失败：oops");
  });

  it("parses lock failures for code 3006", () => {
    const parsed = parseBackendError({
      code: 3006,
      payload: {
        reason: "locked",
        can_continue: "true",
        locking_processes: '["python.exe","cmd.exe"]',
      },
    });
    expect(parsed.message).toBe("目标路径被占用，请关闭相关进程后重试");
    expect(parsed.lock).toEqual({
      detail: "目标路径被占用，请关闭相关进程后重试",
      canContinue: true,
      processes: ["python.exe", "cmd.exe"],
    });
  });

  it("reports max-session lock failures distinctly", () => {
    const parsed = parseBackendError({
      code: 3006,
      payload: {
        reason: "check_failed",
        check_failure_kind: "max_sessions_reached",
      },
    });
    expect(parsed.lock?.detail).toBe(
      "系统 Restart Manager 会话数已达上限，无法检测占用状态",
    );
    expect(parsed.lock?.canContinue).toBe(false);
    expect(parsed.lock?.processes).toEqual([]);
  });

  it("ignores malformed lock payloads", () => {
    const parsed = parseBackendError({
      code: 3006,
      payload: { locking_processes: "[broken json" },
    });
    expect(parsed.lock?.processes).toEqual([]);
  });

  it("passes through Error and string messages", () => {
    expect(parseBackendError(new Error("boom"))).toEqual({
      message: "boom",
      lock: null,
    });
    expect(parseBackendError("direct text")).toEqual({
      message: "direct text",
      lock: null,
    });
  });

  it("falls back to the generic message for unusable errors", () => {
    expect(parseBackendError(null)).toEqual({ message: "操作失败", lock: null });
    expect(parseBackendError(42)).toEqual({ message: "操作失败", lock: null });
    expect(parseBackendError("   ")).toEqual({
      message: "操作失败",
      lock: null,
    });
    expect(parseBackendError({ code: "1001" })).toEqual({
      message: "操作失败",
      lock: null,
    });
  });

  it("keeps only string payload entries", () => {
    expect(
      parseBackendError({
        code: 1001,
        payload: { instance: 5, detail: "kept" },
        extra: "ignored",
      }).message,
    ).toBe("实例不存在");
  });
});

describe("toUserMessage", () => {
  it("returns the parsed message", () => {
    expect(toUserMessage({ code: 1002, payload: {} })).toBe(
      "实例正在运行，请先停止",
    );
  });
});

describe("isDownloadCancelled", () => {
  it("detects the download-cancelled code only", () => {
    expect(isDownloadCancelled({ code: 1006, payload: {} })).toBe(true);
    expect(isDownloadCancelled({ code: 1001, payload: {} })).toBe(false);
    expect(isDownloadCancelled(new Error("nope"))).toBe(false);
    expect(isDownloadCancelled("nope")).toBe(false);
  });
});
