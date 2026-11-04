/**
 * Frontend contract mirroring the Rust IPC types.
 *
 * Field names stay snake_case because serde serializes the backend structs
 * without a rename rule. Command arguments are camelCase at the invoke
 * boundary, matching Tauri's argument conversion.
 */

export type ThemePreference = "system" | "light" | "dark";

export type SupportedLocale = "zh-CN" | "en-US" | "ru-RU" | "ja-JP";

export type LanguagePreference = "system" | SupportedLocale;

export type InstanceState = "stopped" | "starting" | "running" | "stopping";

export type ComponentId = "python" | "nodejs" | "uv";

export type LogLevel = "debug" | "info" | "warn" | "error";

export interface DefaultCredentials {
  source: string;
  display_name: string;
  username: string;
  password: string;
}

export type LockCheckTarget =
  | "instance_data"
  | "backup_create"
  | "backup_restore"
  | "instance_upgrade";

export type RepairPreserveScope =
  | "data_directory"
  | "config_and_data_files"
  | "core_config_and_data_files"
  | "database_only";

export interface AppConfig {
  github_proxy: string;
  proxy_url: string;
  proxy_port: string;
  proxy_username: string;
  proxy_password: string;
  pypi_mirror: string;
  nodejs_mirror: string;
  npm_registry: string;
  python_mirror: string;
  uv_mirror: string;
  allow_local_dns_probe: boolean;
  use_uv_for_deps: boolean;
  close_to_tray: boolean;
  autostart_minimize_to_tray: boolean;
  check_instance_update: boolean;
  persist_instance_state: boolean;
  ignore_external_path: boolean;
  use_short_instance_id: boolean;
  lock_check_extension_whitelist: boolean;
  theme_preference: ThemePreference;
  language_preference: LanguagePreference;
}

export interface InstanceStatus {
  id: string;
  name: string;
  state: InstanceState;
  port: number;
  version: string;
  dashboard_enabled: boolean;
  pid_tracker_not_available: boolean;
  configured_host: string;
  configured_port: number;
  check_update_enabled: boolean;
  auto_restart_enabled: boolean;
  auto_restart_max_retries: number;
  auto_restart_notify: boolean;
  last_exit_reason: string;
  restart_attempts: number;
}

export interface InstalledVersion {
  version: string;
  zip_path: string;
}

export interface BackupMetadata {
  created_at: string;
  instance_name: string;
  instance_id: string;
  version: string;
  includes_venv: boolean;
  includes_data: boolean;
  arch_target: string;
  auto_generated: boolean;
  note: string;
}

export interface BackupInfo {
  filename: string;
  path: string;
  metadata: BackupMetadata;
  corrupted: boolean;
  parse_error: string | null;
}

export interface ComponentStatus {
  id: string;
  installed: boolean;
  display_name: string;
  description: string;
}

export interface ComponentsSnapshot {
  components: ComponentStatus[];
}

export interface AppSnapshot {
  instances: InstanceStatus[];
  versions: InstalledVersion[];
  backups: BackupInfo[];
  components: ComponentsSnapshot;
  config: AppConfig;
  data_dir: string;
}

export interface GitHubAsset {
  name: string;
  browser_download_url: string;
  size: number;
}

export interface GitHubRelease {
  tag_name: string;
  name: string;
  published_at: string;
  prerelease: boolean;
  assets: GitHubAsset[];
  html_url: string;
  body: string | null;
}

export type InstanceUpdateState =
  | "available"
  | "up_to_date"
  | "disabled"
  | "unknown";

export interface InstanceUpdateInfo {
  instance_id: string;
  current_version: string;
  latest_version: string | null;
  state: InstanceUpdateState;
  check_enabled: boolean;
}

export interface InstanceUpdateCheckResult {
  update_check_enabled: boolean;
  latest_release: GitHubRelease | null;
  instances: InstanceUpdateInfo[];
}
export interface DownloadProgress {
  id: string;
  downloaded: number;
  total: number | null;
  progress: number | null;
  /** Smoothed transfer rate in bytes per second, `null` when unknown. */
  speed: number | null;
  step: string;
  message: string;
  message_key?: string;
}

export interface DeployProgress {
  instance_id: string;
  step: string;
  message: string;
  message_key?: string;
  progress: number;
}

export interface ComponentActionResult {
  key: string;
  versions: string[];
  message: string;
}

export interface LogEntry {
  source: string;
  level: LogLevel;
  message: string;
  timestamp: string;
}

export interface UpdateInstanceRequest {
  name?: string;
  version?: string;
  host?: string;
  port?: number;
  checkUpdateEnabled?: boolean;
  autoRestartEnabled?: boolean;
  autoRestartMaxRetries?: number;
  autoRestartNotify?: boolean;
}

export interface InstanceRestartEvent {
  instance_id: string;
  instance_name: string;
  attempt: number;
  max_retries: number;
  will_retry: boolean;
  delay_secs: number;
  reason: string;
}

export interface DataDirChangeResult {
  old_dir: string;
  new_dir: string;
}

export interface RebuildInstanceManifestResult {
  instances: number;
  versions: number;
}
