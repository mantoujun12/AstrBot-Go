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
  builtin: boolean;
}

/** One entry of a skill directory listing, path relative to the skill root. */
export interface SkillDirEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

/**
 * Addresses a skill through the file viewer: a local skill
 * (`plugin_name === null`) or a plugin / builtin provided one.
 */
export interface SkillFileRequest {
  skill_name: string;
  plugin_name: string | null;
  builtin: boolean;
  path: string;
}

export interface SkillZipInstall {
  zip_path: string;
  name_hint: string | null;
}
