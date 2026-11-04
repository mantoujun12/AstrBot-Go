import { invoke } from "@tauri-apps/api/core";
import type {
  ResourceAlertThreshold,
  ResourceSample,
} from "../types/launcher";

export function getInstanceResourceHistory(
  instanceId: string,
): Promise<ResourceSample[]> {
  return invoke<ResourceSample[]>("get_instance_resource_history", {
    instanceId,
  });
}

export function getResourcesSnapshot(): Promise<
  Record<string, ResourceSample>
> {
  return invoke<Record<string, ResourceSample>>("get_resources_snapshot");
}

export function getResourceAlertThreshold(
  instanceId: string,
): Promise<ResourceAlertThreshold> {
  return invoke<ResourceAlertThreshold>("get_resource_alert_threshold", {
    instanceId,
  });
}

export function setResourceAlertThreshold(
  instanceId: string,
  thresholds: ResourceAlertThreshold,
): Promise<void> {
  return invoke("set_resource_alert_threshold", {
    instanceId,
    thresholds,
  });
}
