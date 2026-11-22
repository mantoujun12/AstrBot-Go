/**
 * Frontend contract mirroring the Rust IPC types in
 * `src-tauri/src/instance/skills.rs`.
 */

export interface InstanceSkill {
  name: string;
  description: string;
  active: boolean;
}

/** Read-only skill shipped inside a plugin's own `skills/` directory. */
export interface InstancePluginSkill {
  name: string;
  description: string;
  plugin_name: string;
}

export interface SkillZipInstall {
  zip_path: string;
  name_hint: string | null;
}
