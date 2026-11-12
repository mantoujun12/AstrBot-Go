import { invoke } from "@tauri-apps/api/core";
import type {
  LockCheckTarget,
  InstanceHealthReport,
  DiscoveredInstancesSnapshot,
  RebuildInstanceManifestResult,
  RepairPreserveScope,
  UpdateInstanceRequest,
} from "../types/launcher";

export interface LockCheckInput {
  target: LockCheckTarget;
  instanceId?: string;
  backupPath?: string;
}

export function createInstance(
  name: string,
  version: string,
  port: number,
): Promise<void> {
  return invoke("create_instance", { name, version, port });
}

export function updateInstance(
  instanceId: string,
  request: UpdateInstanceRequest,
): Promise<void> {
  return invoke("update_instance", { instanceId, request });
}

export function deleteInstance(instanceId: string): Promise<void> {
  return invoke("delete_instance", { instanceId });
}

export function startInstance(instanceId: string): Promise<number> {
  return invoke<number>("start_instance", { instanceId });
}

export function stopInstance(instanceId: string): Promise<void> {
  return invoke("stop_instance", { instanceId });
}

export function restartInstance(instanceId: string): Promise<number> {
  return invoke<number>("restart_instance", { instanceId });
}

export function getInstancePort(instanceId: string): Promise<number> {
  return invoke<number>("get_instance_port", { instanceId });
}

export function checkInstanceHealth(
  instanceId: string,
): Promise<InstanceHealthReport> {
  return invoke<InstanceHealthReport>("check_instance_health", { instanceId });
}

export function openInstanceCoreFolder(instanceId: string): Promise<void> {
  return invoke("open_instance_core_folder", { instanceId });
}

export function checkLock(input: LockCheckInput): Promise<void> {
  const args: Record<string, string> = { target: input.target };
  if (input.instanceId !== undefined) {
    args.instanceId = input.instanceId;
  }
  if (input.backupPath !== undefined) {
    args.backupPath = input.backupPath;
  }
  return invoke("check_lock", args);
}

export function clearInstanceData(instanceId: string): Promise<void> {
  return invoke("clear_instance_data", { instanceId });
}

export function clearInstanceVenv(instanceId: string): Promise<void> {
  return invoke("clear_instance_venv", { instanceId });
}

export function clearPycache(instanceId: string): Promise<void> {
  return invoke("clear_pycache", { instanceId });
}

export function repairInstance(
  instanceId: string,
  preserveScope: RepairPreserveScope,
): Promise<void> {
  return invoke("repair_instance", { instanceId, preserveScope });
}

export function rebuildInstanceManifest(): Promise<RebuildInstanceManifestResult> {
  return invoke<RebuildInstanceManifestResult>("rebuild_instance_manifest");
}

export function discoverInstances(): Promise<DiscoveredInstancesSnapshot> {
  return invoke<DiscoveredInstancesSnapshot>("list_discovered_instances");
}

export function saveInstanceSourceEnabled(
  source: string,
  enabled: boolean,
): Promise<void> {
  return invoke("save_instance_source_enabled", { source, enabled });
}
