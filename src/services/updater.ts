import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface LauncherUpdateInfo {
  available: boolean;
  current_version: string;
  latest_version: string;
  notes: string | null;
  published_at: string | null;
}

export interface LauncherUpdateProgress {
  downloaded: number;
  total: number | null;
  progress: number | null;
  speed: number | null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function isLauncherUpdateInfo(
  value: unknown,
): value is LauncherUpdateInfo {
  if (!isRecord(value)) {
    return false;
  }

  return (
    typeof value.available === "boolean" &&
    typeof value.current_version === "string" &&
    typeof value.latest_version === "string" &&
    (value.notes === null || typeof value.notes === "string") &&
    (value.published_at === null || typeof value.published_at === "string")
  );
}

function isUpdateProgress(value: unknown): value is LauncherUpdateProgress {
  if (!isRecord(value)) {
    return false;
  }

  return (
    typeof value.downloaded === "number" &&
    Number.isFinite(value.downloaded) &&
    (value.total === null ||
      (typeof value.total === "number" && Number.isFinite(value.total))) &&
    (value.progress === null ||
      (typeof value.progress === "number" && Number.isFinite(value.progress))) &&
    (value.speed === null ||
      (typeof value.speed === "number" && Number.isFinite(value.speed)))
  );
}

async function invokeUpdateInfo(command: string): Promise<LauncherUpdateInfo> {
  const value = await invoke<unknown>(command);
  if (!isLauncherUpdateInfo(value)) {
    throw new Error("Received invalid launcher update information");
  }
  return value;
}

export function checkLauncherUpdate(): Promise<LauncherUpdateInfo> {
  return invokeUpdateInfo("check_launcher_update");
}

export function downloadLauncherUpdate(): Promise<LauncherUpdateInfo> {
  return invokeUpdateInfo("download_launcher_update");
}

export function installLauncherUpdate(): Promise<void> {
  return invoke("install_launcher_update");
}

export function restartLauncher(): Promise<void> {
  return invoke("restart_launcher");
}

export function listenLauncherUpdateProgress(
  handler: (progress: LauncherUpdateProgress) => void,
): Promise<UnlistenFn> {
  return listen<unknown>("launcher-update-progress", (event) => {
    if (isUpdateProgress(event.payload)) {
      handler(event.payload);
    }
  });
}
