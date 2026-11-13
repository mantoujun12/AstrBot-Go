import { invoke, isTauri } from "@tauri-apps/api/core";

export async function syncWindowTheme(isDark: boolean): Promise<void> {
  if (!isTauri()) return;
  try {
    await invoke("set_window_theme", { dark: isDark });
  } catch {
    // Native theme sync is cosmetic; the webview theme still applies.
    console.debug("Native window theme sync failed; skipped.");
  }
}
