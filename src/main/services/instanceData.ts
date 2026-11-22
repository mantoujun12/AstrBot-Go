import { invoke } from "@tauri-apps/api/core";
import type {
  CopyInstanceDataRequest,
  InstanceConfigFile,
  InstanceCronJob,
  InstanceCronJobInput,
  InstanceKnowledgeBase,
  InstancePersona,
  InstancePersonaFolder,
} from "../types/instanceData";

export function listInstancePersonas(
  instanceId: string,
): Promise<InstancePersona[]> {
  return invoke<InstancePersona[]>("list_instance_personas", { instanceId });
}

export function saveInstancePersonas(
  instanceId: string,
  upserts: InstancePersona[],
  deletes: string[],
): Promise<void> {
  return invoke("save_instance_personas", { instanceId, upserts, deletes });
}

export function listInstanceCronJobs(
  instanceId: string,
): Promise<InstanceCronJob[]> {
  return invoke<InstanceCronJob[]>("list_instance_cron_jobs", { instanceId });
}

export function saveInstanceCronJobs(
  instanceId: string,
  upserts: InstanceCronJobInput[],
  deletes: string[],
): Promise<void> {
  return invoke("save_instance_cron_jobs", { instanceId, upserts, deletes });
}

export function listInstancePersonaFolders(
  instanceId: string,
): Promise<InstancePersonaFolder[]> {
  return invoke<InstancePersonaFolder[]>("list_instance_persona_folders", {
    instanceId,
  });
}

export function listInstanceKnowledgeBases(
  instanceId: string,
): Promise<InstanceKnowledgeBase[]> {
  return invoke<InstanceKnowledgeBase[]>("list_instance_knowledge_bases", {
    instanceId,
  });
}

export function deleteInstanceKnowledgeBase(
  instanceId: string,
  kbId: string,
): Promise<void> {
  return invoke("delete_instance_knowledge_base", { instanceId, kbId });
}

export function listInstanceConfigFiles(
  instanceId: string,
): Promise<InstanceConfigFile[]> {
  return invoke<InstanceConfigFile[]>("list_instance_config_files", {
    instanceId,
  });
}

export function readInstanceConfigFile(
  instanceId: string,
  name: string,
): Promise<string> {
  return invoke<string>("read_instance_config_file", { instanceId, name });
}

export function writeInstanceConfigFile(
  instanceId: string,
  name: string,
  content: string,
): Promise<void> {
  return invoke("write_instance_config_file", {
    instanceId,
    name,
    content,
  });
}

export function copyInstanceData(
  request: CopyInstanceDataRequest,
): Promise<void> {
  return invoke("copy_instance_data", { request });
}

/**
 * The backend reports a missing data_v4.db only through its error detail,
 * so detect it textually to show the v4-only hint instead.
 */
export function isDataV4Missing(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    JSON.stringify(error).includes("data_v4")
  );
}

/**
 * A read failed because the config file does not exist yet (AstrBot creates
 * some files on first run) rather than because of a real failure.
 */
export function isConfigFileMissing(error: unknown): boolean {
  if (typeof error !== "object" || error === null) {
    return false;
  }
  const payload = (error as { payload?: unknown }).payload;
  return typeof payload === "object" && payload !== null && "missing_file" in payload;
}
