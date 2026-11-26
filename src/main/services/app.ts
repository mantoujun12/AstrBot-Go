import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { i18n } from "../i18n";
import type { AppSnapshot, DataDirChangeResult } from "../types/launcher";
import { isHttpUrl } from "../utils/network";

export function getAppSnapshot(): Promise<AppSnapshot> {
  return invoke<AppSnapshot>("get_app_snapshot");
}

export function openFolder(path: string): Promise<void> {
  return invoke("open_folder", { path });
}

export async function openExternalUrl(url: string): Promise<void> {
  if (!isHttpUrl(url)) {
    throw new Error(i18n.global.t("errors.unsupportedExternalUrl"));
  }
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
    title: i18n.global.t("dialogs.pickDirectoryTitle"),
  });

  if (typeof selected === "string" && selected.trim() !== "") {
    return selected;
  }
  return null;
}

/** Pick one skill ZIP; the file name becomes the root-mode skill name hint. */
export async function pickSkillZip(): Promise<{
  path: string;
  nameHint: string;
} | null> {
  const selected = await open({
    multiple: false,
    filters: [{ name: "ZIP", extensions: ["zip"] }],
  });
  if (typeof selected !== "string" || selected.trim() === "") {
    return null;
  }
  const fileName = selected.split(/[\\/]/).pop() ?? "";
  const nameHint = fileName.replace(/\.zip$/i, "");
  return { path: selected, nameHint };
}
