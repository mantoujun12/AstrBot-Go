import { invoke } from "@tauri-apps/api/core";
import type { GitHubRelease } from "../types/launcher";

export function fetchReleases(forceRefresh = false): Promise<GitHubRelease[]> {
  return invoke<GitHubRelease[]>("fetch_releases", { forceRefresh });
}

export function installVersion(release: GitHubRelease): Promise<void> {
  return invoke("install_version", { release });
}

export function uninstallVersion(version: string): Promise<void> {
  return invoke("uninstall_version", { version });
}

export function fetchLauncherReleaseNotes(
  version: string,
): Promise<string | null> {
  return invoke<string | null>("fetch_launcher_release_notes", { version });
}
