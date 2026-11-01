import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppSnapshot,
  DefaultCredentials,
  DeployProgress,
  DownloadProgress,
  LogEntry,
  LogLevel,
} from "../types/launcher";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function normalizeLevel(value: unknown): LogLevel {
  if (value === "debug") {
    return "debug";
  }
  if (value === "info") {
    return "info";
  }
  if (value === "warn" || value === "warning") {
    return "warn";
  }
  if (value === "error") {
    return "error";
  }
  // The backend logs through the `log` crate, which may also emit `trace`.
  if (value === "trace") {
    return "debug";
  }
  return "info";
}

function isAppSnapshot(value: unknown): value is AppSnapshot {
  if (!isRecord(value)) {
    return false;
  }
  const components = value.components;
  return (
    Array.isArray(value.instances) &&
    Array.isArray(value.versions) &&
    Array.isArray(value.backups) &&
    isRecord(components) &&
    Array.isArray(components.components) &&
    isRecord(value.config) &&
    typeof value.data_dir === "string"
  );
}

function toLogEntry(value: unknown): LogEntry | null {
  if (!isRecord(value)) {
    return null;
  }
  if (typeof value.source !== "string" || typeof value.message !== "string") {
    return null;
  }
  return {
    source: value.source,
    level: normalizeLevel(value.level),
    message: value.message,
    timestamp: typeof value.timestamp === "string" ? value.timestamp : "",
  };
}

function toDownloadProgress(value: unknown): DownloadProgress | null {
  if (!isRecord(value) || typeof value.id !== "string") {
    return null;
  }
  const downloaded = typeof value.downloaded === "number" ? value.downloaded : 0;
  const total = typeof value.total === "number" ? value.total : null;
  const progress = typeof value.progress === "number" ? value.progress : null;
  const speed = typeof value.speed === "number" ? value.speed : null;
  return {
    id: value.id,
    downloaded,
    total,
    progress,
    speed,
    step: typeof value.step === "string" ? value.step : "",
    message: typeof value.message === "string" ? value.message : "",
  };
}

function toDeployProgress(value: unknown): DeployProgress | null {
  if (
    !isRecord(value) ||
    typeof value.instance_id !== "string" ||
    typeof value.step !== "string"
  ) {
    return null;
  }
  return {
    instance_id: value.instance_id,
    step: value.step,
    message: typeof value.message === "string" ? value.message : "",
    progress: typeof value.progress === "number" ? value.progress : 0,
  };
}

function toDefaultCredentials(value: unknown): DefaultCredentials | null {
  if (
    !isRecord(value) ||
    typeof value.source !== "string" ||
    typeof value.display_name !== "string" ||
    typeof value.username !== "string" ||
    typeof value.password !== "string"
  ) {
    return null;
  }
  return {
    source: value.source,
    display_name: value.display_name,
    username: value.username,
    password: value.password,
  };
}

export function listenAppSnapshot(
  handler: (snapshot: AppSnapshot) => void,
): Promise<UnlistenFn> {
  return listen<unknown>("app-snapshot", (event) => {
    if (isAppSnapshot(event.payload)) {
      handler(event.payload);
    }
  });
}

export function listenLogEntry(
  handler: (entry: LogEntry) => void,
): Promise<UnlistenFn> {
  return listen<unknown>("log-entry", (event) => {
    const entry = toLogEntry(event.payload);
    if (entry) {
      handler(entry);
    }
  });
}

export function listenDownloadProgress(
  handler: (progress: DownloadProgress) => void,
): Promise<UnlistenFn> {
  return listen<unknown>("download-progress", (event) => {
    const progress = toDownloadProgress(event.payload);
    if (progress) {
      handler(progress);
    }
  });
}

export function listenDeployProgress(
  handler: (progress: DeployProgress) => void,
): Promise<UnlistenFn> {
  return listen<unknown>("deploy-progress", (event) => {
    const progress = toDeployProgress(event.payload);
    if (progress) {
      handler(progress);
    }
  });
}

export function listenDefaultCredentials(
  handler: (credentials: DefaultCredentials) => void,
): Promise<UnlistenFn> {
  return listen<unknown>("default-credentials-detected", (event) => {
    const credentials = toDefaultCredentials(event.payload);
    if (credentials) {
      handler(credentials);
    }
  });
}
