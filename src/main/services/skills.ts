import { invoke } from "@tauri-apps/api/core";
import type { InstancePluginSkill, InstanceSkill, SkillZipInstall } from "../types/skills";

export function listInstanceSkills(instanceId: string): Promise<InstanceSkill[]> {
  return invoke<InstanceSkill[]>("list_instance_skills", { instanceId });
}

export function listInstancePluginSkills(
  instanceId: string,
): Promise<InstancePluginSkill[]> {
  return invoke<InstancePluginSkill[]>("list_instance_plugin_skills", { instanceId });
}

export function readInstancePluginSkill(
  instanceId: string,
  pluginName: string,
  name: string,
): Promise<string> {
  return invoke<string>("read_instance_plugin_skill", {
    instanceId,
    pluginName,
    name,
  });
}

export function readInstanceSkill(
  instanceId: string,
  name: string,
): Promise<string> {
  return invoke<string>("read_instance_skill", { instanceId, name });
}

export function writeInstanceSkill(
  instanceId: string,
  name: string,
  content: string,
): Promise<void> {
  return invoke("write_instance_skill", { instanceId, name, content });
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
