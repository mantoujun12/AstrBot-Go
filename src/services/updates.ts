import { invoke } from "@tauri-apps/api/core";
import type { InstanceUpdateCheckResult } from "../types/launcher";

export function checkInstanceUpdates(
  forceRefresh = false,
): Promise<InstanceUpdateCheckResult> {
  return invoke<InstanceUpdateCheckResult>("check_instance_updates", {
    forceRefresh,
  });
}
