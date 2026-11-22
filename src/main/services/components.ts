import { invoke } from "@tauri-apps/api/core";
import type { ComponentActionResult, ComponentId } from "../types/launcher";

export function installComponent(
  componentId: ComponentId,
): Promise<ComponentActionResult> {
  return invoke<ComponentActionResult>("install_component", { componentId });
}

export function reinstallComponent(
  componentId: ComponentId,
): Promise<ComponentActionResult> {
  return invoke<ComponentActionResult>("reinstall_component", { componentId });
}

export function uninstallComponent(
  componentId: ComponentId,
): Promise<ComponentActionResult> {
  return invoke<ComponentActionResult>("uninstall_component", { componentId });
}

export function clearUvCache(): Promise<void> {
  return invoke("clear_uv_cache");
}
