import { invoke } from "@tauri-apps/api/core";
import { emitTo } from "@tauri-apps/api/event";
import type { TrayStyle } from "../types/launcher";

export type TrayPanelAction =
  | { kind: "settings" }
  | { kind: "instance"; instanceId: string };

export function saveTrayStyle(trayStyle: TrayStyle): Promise<void> {
  return invoke("save_tray_style", { trayStyle });
}

export function setTrayPanelHeight(height: number): Promise<void> {
  return invoke("set_tray_panel_height", { height });
}

export function hideTrayPanel(): Promise<void> {
  return invoke("hide_tray_panel");
}

/**
 * Reports that the tray panel page finished loading, so the backend may show
 * it (replaying a queued toggle) with fully loaded styles.
 */
export function notifyTrayPanelReady(): Promise<void> {
  return invoke("notify_tray_panel_ready");
}

export function quitApp(): Promise<void> {
  return invoke("quit_app");
}

/**
 * Asks the main window to reveal the settings page or an instance WebUI tab.
 * The main window is created at startup, so its listener is already attached.
 */
export function sendTrayPanelAction(action: TrayPanelAction): Promise<void> {
  return emitTo("main", "tray-panel-action", action);
}
