/**
 * Translates the backend `{ code, payload }` error envelope into messages the
 * interface can show. Backend error codes are defined in `src-tauri/src/error.rs`.
 */

import { i18n } from "../i18n";

export interface BackendError {
  code: number;
  payload: Record<string, string>;
}

export interface LockFailure {
  detail: string;
  canContinue: boolean;
  processes: string[];
}

export interface ParsedBackendError {
  message: string;
  lock: LockFailure | null;
}

function translate(key: string, params?: Record<string, string>): string {
  return params ? i18n.global.t(key, params) : i18n.global.t(key);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function readPayload(value: unknown): Record<string, string> {
  if (!isRecord(value)) {
    return {};
  }

  const payload: Record<string, string> = {};
  for (const [key, entry] of Object.entries(value)) {
    if (typeof entry === "string") {
      payload[key] = entry;
    }
  }
  return payload;
}

function toBackendError(error: unknown): BackendError | null {
  if (!isRecord(error)) {
    return null;
  }

  const code = error.code;
  if (typeof code !== "number" || !Number.isInteger(code)) {
    return null;
  }

  return {
    code,
    payload: readPayload(error.payload),
  };
}

function parseProcessList(raw: string | undefined): string[] {
  if (!raw) {
    return [];
  }

  try {
    const value: unknown = JSON.parse(raw);
    if (!Array.isArray(value)) {
      return [];
    }
    return value.filter((item): item is string => typeof item === "string");
  } catch {
    return [];
  }
}

function lockDetail(payload: Record<string, string>): string {
  if (payload.reason === "locked") {
    return translate("errors.lock.detailLocked");
  }
  if (payload.reason === "check_failed") {
    return payload.check_failure_kind === "max_sessions_reached"
      ? translate("errors.lock.detailMaxSessions")
      : translate("errors.lock.detailCheckFailed");
  }
  return payload.detail ?? translate("errors.lock.detailLocked");
}

function parseLockFailure(payload: Record<string, string>): LockFailure {
  return {
    detail: lockDetail(payload),
    canContinue: payload.can_continue === "true",
    processes: parseProcessList(payload.locking_processes),
  };
}

function messageForCode(code: number, payload: Record<string, string>): string {
  switch (code) {
    case 1001:
      return translate("errors.instanceNotFound");
    case 1002:
      return translate("errors.instanceRunning");
    case 1003:
      return translate("errors.instanceNotRunning");
    case 1004:
      return translate("errors.versionNotFound");
    case 1005:
      return payload.instance
        ? translate("errors.versionInUseWithInstance", { instance: payload.instance })
        : translate("errors.versionInUse");
    case 1006:
      return translate("errors.downloadCancelled");
    case 2001:
      return payload.detail
        ? translate("errors.configSaveFailedWithDetail", { detail: payload.detail })
        : translate("errors.configSaveFailed");
    case 2002:
      return payload.detail
        ? translate("errors.fileOperationFailedWithDetail", { detail: payload.detail })
        : translate("errors.fileOperationFailed");
    case 2003:
      return payload.detail
        ? translate("errors.networkRequestFailedWithDetail", { detail: payload.detail })
        : translate("errors.networkRequestFailed");
    case 3001:
      return payload.detail
        ? translate("errors.pythonRuntimeErrorWithDetail", { detail: payload.detail })
        : translate("errors.pythonRuntimeError");
    case 3002:
      return translate("errors.pythonNotInstalled");
    case 3003:
      return payload.detail
        ? translate("errors.processOperationFailedWithDetail", { detail: payload.detail })
        : translate("errors.processOperationFailed");
    case 3004:
      return payload.port
        ? translate("errors.portOccupiedWithPort", { port: payload.port })
        : translate("errors.portOccupied");
    case 3005:
      return translate("errors.startupTimeout");
    case 3006:
      return lockDetail(payload);
    case 3007:
      return payload.detail
        ? translate("errors.invalidHostWithDetail", { detail: payload.detail })
        : translate("errors.invalidHost");
    case 4001:
      return payload.detail
        ? translate("errors.backupFailedWithDetail", { detail: payload.detail })
        : translate("errors.backupFailed");
    case 4002:
      return translate("errors.githubFailed");
    case 5001:
      return translate("errors.updateFailed");
    case 9999:
      return payload.detail
        ? translate("errors.operationFailedWithDetail", { detail: payload.detail })
        : translate("errors.operationFailed");
    default:
      return payload.detail
        ? translate("errors.operationFailedWithDetail", { detail: payload.detail })
        : translate("errors.operationFailed");
  }
}

export function parseBackendError(error: unknown): ParsedBackendError {
  const backend = toBackendError(error);
  if (backend) {
    return {
      message: messageForCode(backend.code, backend.payload),
      lock: backend.code === 3006 ? parseLockFailure(backend.payload) : null,
    };
  }

  if (error instanceof Error && error.message.trim() !== "") {
    return { message: error.message, lock: null };
  }

  if (typeof error === "string" && error.trim() !== "") {
    return { message: error, lock: null };
  }

  return { message: translate("errors.operationFailed"), lock: null };
}

export function toUserMessage(error: unknown): string {
  return parseBackendError(error).message;
}

export function isDownloadCancelled(error: unknown): boolean {
  return toBackendError(error)?.code === 1006;
}

/**
 * `true` when a `get_webui_session` failure means "the WebUI keeps its own
 * sign-in form" and must stay silent: the instance is not running (1003), or
 * neither the desktop exchange nor stored credentials could produce a
 * session, which the backend marks with `reason: "webui_manual_login"`.
 */
export function isWebuiManualLoginRequired(error: unknown): boolean {
  const backend = toBackendError(error);
  if (!backend) {
    return false;
  }
  return (
    backend.code === 1003 || backend.payload.reason === "webui_manual_login"
  );
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
}
