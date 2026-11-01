import { invoke } from "@tauri-apps/api/core";
import type { ComponentId } from "../types/launcher";

export function installComponent(componentId: ComponentId): Promise<string> {
  return invoke<string>("install_component", { componentId });
}

export function reinstallComponent(componentId: ComponentId): Promise<string> {
  return invoke<string>("reinstall_component", { componentId });
}

export function uninstallComponent(componentId: ComponentId): Promise<string> {
  return invoke<string>("uninstall_component", { componentId });
}

export function clearUvCache(): Promise<void> {
  return invoke("clear_uv_cache");
}
