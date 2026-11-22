/**
 * Frontend contract mirroring the Rust IPC types in
 * `src-tauri/src/instance/skills.rs`.
 */

export interface InstanceSkill {
  name: string;
  description: string;
  active: boolean;
}

export interface SkillZipInstall {
  zip_path: string;
  name_hint: string | null;
}
