/**
 * Frontend contract for the AstrBot instance-data commands.
 *
 * Response struct fields stay snake_case because serde serializes the backend
 * structs without a rename rule; the copy request follows the repo-wide
 * camelCase request convention. Command arguments are camelCase at the invoke
 * boundary, matching Tauri's argument conversion.
 */

export interface InstancePersona {
  persona_id: string;
  system_prompt: string;
  begin_dialogs: string[];
  /** `null` = all tools, `[]` = none. */
  tools: string[] | null;
  skills: string[] | null;
  custom_error_message: string | null;
  folder_id: string | null;
  sort_order: number;
}

export interface InstanceCronJob {
  job_id: string;
  name: string;
  description: string | null;
  job_type: "basic" | "active_agent";
  cron_expression: string | null;
  timezone: string | null;
  payload: Record<string, unknown>;
  enabled: boolean;
  persistent: boolean;
  run_once: boolean;
  status: string | null;
  last_run_at: string | null;
  next_run_time: string | null;
  last_error: string | null;
}

export type InstanceCronJobInput = Omit<
  InstanceCronJob,
  "status" | "last_run_at" | "next_run_time" | "last_error"
>;

export interface InstancePersonaFolder {
  folder_id: string;
  name: string;
  parent_id: string | null;
  description: string | null;
  sort_order: number;
}

export interface InstanceKnowledgeBase {
  kb_id: string;
  name: string;
  description: string | null;
  emoji: string | null;
  doc_count: number;
  chunk_count: number;
  size_bytes: number;
}

export interface InstanceConfigFile {
  /** "cmd_config.json" or "config/abconf_xxx.yaml". */
  name: string;
  size: number;
  modified_ms: number | null;
}

export interface CopyInstanceDataRequest {
  sourceInstanceId: string;
  targetInstanceIds: string[];
  personaIds: string[];
  cronJobIds: string[];
  configFiles: string[];
  overwrite: boolean;
}

/** Preselected ids per section when opening the copy dialog. */
export interface CopyPreselect {
  personas: string[];
  cronJobs: string[];
  configFiles: string[];
}
