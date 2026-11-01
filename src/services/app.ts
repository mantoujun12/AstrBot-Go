import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { AppSnapshot, DataDirChangeResult } from "../types/launcher";

export function getAppSnapshot(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("get_app_snapshot");
}

export function rebuildAppSnapshot(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("rebuild_app_snapshot");
}

export function openFolder(path: string): Promise<void> {
  return invoke("open_folder", { path });
}

export function openExternalUrl(url: string): Promise<void> {
  return openUrl(url);
}

export function setDataDir(newDir: string): Promise<DataDirChangeResult> {
  return invoke<DataDirChangeResult>("set_data_dir", { newDir });
}

export function getAppVersion(): Promise<string> {
  return getVersion();
}

export async function pickDirectory(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "选择数据目录",
  });

  if (typeof selected === "string" && selected.trim() !== "") {
    return selected;
  }
  return null;
}
