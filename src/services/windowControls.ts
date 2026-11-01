import { invoke } from "@tauri-apps/api/core";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

// Resolved lazily so the development mock can install window metadata first.
function currentWindow() {
  return getCurrentWindow();
}

export function minimizeWindow(): Promise<void> {
  return currentWindow().minimize();
}

export function toggleMaximizeWindow(): Promise<void> {
  return currentWindow().toggleMaximize();
}

export function closeWindow(): Promise<void> {
  return currentWindow().close();
}

export function isWindowMaximized(): Promise<boolean> {
  return currentWindow().isMaximized();
}

export function onWindowResize(handler: () => void): Promise<UnlistenFn> {
  return currentWindow().onResized(() => handler());
}

export function showSnapOverlay(): Promise<void> {
  return invoke("plugin:decorum|show_snap_overlay");
}
