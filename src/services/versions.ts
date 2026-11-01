import { invoke } from "@tauri-apps/api/core";
import type { GitHubRelease } from "../types/launcher";

export function fetchReleases(forceRefresh = false): Promise<GitHubRelease[]> {
  return invoke<GitHubRelease[]>("fetch_releases", { forceRefresh });
}

export function installVersion(release: GitHubRelease): Promise<void> {
  return invoke("install_version", { release });
}

export function pauseVersionDownload(version: string): Promise<void> {
  return invoke("pause_download", { id: version });
}

export function resumeVersionDownload(version: string): Promise<void> {
  return invoke("resume_download", { id: version });
}

export function cancelVersionDownload(version: string): Promise<void> {
  return invoke("cancel_download", { id: version });
}

export function uninstallVersion(version: string): Promise<void> {
  return invoke("uninstall_version", { version });
}

export function fetchLauncherReleaseNotes(
  version: string,
): Promise<string | null> {
  return invoke<string | null>("fetch_launcher_release_notes", { version });
}
