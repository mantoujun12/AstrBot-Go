import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppSnapshot,
  DefaultCredentials,
  DeployProgress,
  DownloadProgress,
  InstanceRestartEvent,
  LogEntry,
  LogLevel,
  ResourceAlertEvent,
  TaskRecord,
} from "../types/launcher";
import { toTaskRecord } from "./tasks";

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

function toResourceAlert(value: unknown): ResourceAlertEvent | null {
  if (!isRecord(value)) {
    return null;
  }
  const metric = value.metric;
  if (
    typeof value.instance_id !== "string" ||
    (metric !== "cpu" && metric !== "memory" && metric !== "disk") ||
    typeof value.value !== "number" ||
    typeof value.threshold !== "number"
  ) {
    return null;
  }
  return {
    instance_id: value.instance_id,
    metric,
    value: value.value,
    threshold: value.threshold,
  };
}

function toInstanceRestartEvent(value: unknown): InstanceRestartEvent | null {
  if (
    !isRecord(value) ||
    typeof value.instance_id !== "string" ||
    typeof value.instance_name !== "string"
  ) {
    return null;
  }
  const attempt = Number(value.attempt);
  const maxRetries = Number(value.max_retries);
  const delaySecs = Number(value.delay_secs);
  if (
    !Number.isFinite(attempt) ||
    !Number.isFinite(maxRetries) ||
    !Number.isFinite(delaySecs)
  ) {
    return null;
  }
  return {
    instance_id: value.instance_id,
    instance_name: value.instance_name,
    attempt,
    max_retries: maxRetries,
    will_retry: value.will_retry === true,
    delay_secs: delaySecs,
    reason: typeof value.reason === "string" ? value.reason : "",
  };
}

export function listenInstanceRestart(
  handler: (event: InstanceRestartEvent) => void,
): Promise<UnlistenFn> {
  return listen<unknown>("instance-restart", (event) => {
    const payload = toInstanceRestartEvent(event.payload);
    if (payload) {
      handler(payload);
    }
  });
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

export function listenTaskUpdated(
  handler: (task: TaskRecord) => void,
): Promise<UnlistenFn> {
  return listen<unknown>("task-updated", (event) => {
    const task = toTaskRecord(event.payload);
    if (task) {
      handler(task);
    }
  });
}

export function listenTaskRemoved(
  handler: (taskId: string) => void,
): Promise<UnlistenFn> {
  return listen<unknown>("task-removed", (event) => {
    if (typeof event.payload === "string") {
      handler(event.payload);
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

export function listenResourceAlert(
  handler: (alert: ResourceAlertEvent) => void,
): Promise<UnlistenFn> {
  return listen<unknown>("resource-alert", (event) => {
    const alert = toResourceAlert(event.payload);
    if (alert) {
      handler(alert);
    }
  });
}
