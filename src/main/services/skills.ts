import { invoke } from "@tauri-apps/api/core";
import type {
  InstancePluginSkill,
  InstanceSkill,
  SkillDirEntry,
  SkillFileRequest,
  SkillZipInstall,
} from "../types/skills";

export function listInstanceSkills(instanceId: string): Promise<InstanceSkill[]> {
  return invoke<InstanceSkill[]>("list_instance_skills", { instanceId });
}

export function listInstancePluginSkills(
  instanceId: string,
): Promise<InstancePluginSkill[]> {
  return invoke<InstancePluginSkill[]>("list_instance_plugin_skills", { instanceId });
}

export function listInstanceSkillFiles(
  instanceId: string,
  request: SkillFileRequest,
): Promise<SkillDirEntry[]> {
  return invoke<SkillDirEntry[]>("list_instance_skill_files", {
    instanceId,
    request,
  });
}

export function readInstanceSkillFile(
  instanceId: string,
  request: SkillFileRequest,
): Promise<string> {
  return invoke<string>("read_instance_skill_file", { instanceId, request });
}

export function writeInstanceSkillFile(
  instanceId: string,
  request: SkillFileRequest,
  content: string,
): Promise<void> {
  return invoke("write_instance_skill_file", { instanceId, request, content });
}

export function setInstanceSkillEnabled(
  instanceId: string,
  name: string,
  enabled: boolean,
): Promise<void> {
  return invoke("set_instance_skill_enabled", {
    instanceId,
    name,
    enabled,
  });
}

export function deleteInstanceSkill(
  instanceId: string,
  name: string,
): Promise<void> {
  return invoke("delete_instance_skill", { instanceId, name });
}

export function installInstanceSkillZip(
  instanceId: string,
  request: SkillZipInstall,
): Promise<string[]> {
  return invoke<string[]>("install_instance_skill_zip", {
    instanceId,
    request,
  });
}
