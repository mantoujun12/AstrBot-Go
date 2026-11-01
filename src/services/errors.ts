/**
 * Translates the backend `{ code, payload }` error envelope into messages the
 * interface can show. Backend error codes are defined in `src-tauri/src/error.rs`.
 */

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

function parseLockFailure(payload: Record<string, string>): LockFailure {
  return {
    detail: payload.detail ?? "目标路径被占用，请关闭相关进程后重试",
    canContinue: payload.can_continue === "true",
    processes: parseProcessList(payload.locking_processes),
  };
}

function messageForCode(code: number, payload: Record<string, string>): string {
  switch (code) {
    case 1001:
      return "实例不存在";
    case 1002:
      return "实例正在运行，请先停止";
    case 1003:
      return "实例未运行";
    case 1004:
      return "版本不存在或未安装";
    case 1005:
      return payload.instance
        ? `版本正被实例“${payload.instance}”使用`
        : "版本正被实例使用";
    case 1006:
      return "下载已取消";
    case 2001:
      return payload.detail ?? "配置保存失败";
    case 2002:
      return payload.detail ?? "文件操作失败";
    case 2003:
      return payload.detail ?? "网络请求失败";
    case 3001:
      return payload.detail ?? "Python 运行环境错误";
    case 3002:
      return "Python 运行环境未安装";
    case 3003:
      return payload.detail ?? "进程操作失败";
    case 3004:
      return payload.port ? `端口 ${payload.port} 已被占用` : "端口已被占用";
    case 3005:
      return "实例启动超时";
    case 3006:
      return payload.detail ?? "目标路径被占用";
    case 3007:
      return payload.detail ? `主机地址无效：${payload.detail}` : "主机地址无效";
    case 4001:
      return payload.detail ?? "备份操作失败";
    case 4002:
      return "GitHub 请求失败";
    case 9999:
      return payload.detail ?? "操作失败";
    default:
      return payload.detail ?? "操作失败";
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

  return { message: "操作失败", lock: null };
}

export function toUserMessage(error: unknown): string {
  return parseBackendError(error).message;
}

export function isLockError(error: unknown): boolean {
  return parseBackendError(error).lock !== null;
}

export function isDownloadCancelled(error: unknown): boolean {
  return toBackendError(error)?.code === 1006;
}
