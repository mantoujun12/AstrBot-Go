import { invoke } from "@tauri-apps/api/core";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function minimizeWindow(): Promise<void> {
  return getCurrentWindow().minimize();
}

export function toggleMaximizeWindow(): Promise<void> {
  return getCurrentWindow().toggleMaximize();
}

export function closeWindow(): Promise<void> {
  return getCurrentWindow().close();
}

export function isWindowMaximized(): Promise<boolean> {
  return getCurrentWindow().isMaximized();
}

export function onWindowResize(handler: () => void): Promise<UnlistenFn> {
  return getCurrentWindow().onResized(() => handler());
}

export function showSnapOverlay(): Promise<void> {
  return invoke("plugin:decorum|show_snap_overlay");
}
