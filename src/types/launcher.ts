/**
 * Frontend contract mirroring the Rust IPC types.
 *
 * Field names stay snake_case because serde serializes the backend structs
 * without a rename rule. Command arguments are camelCase at the invoke
 * boundary, matching Tauri's argument conversion.
 */

export type ThemePreference = "system" | "light" | "dark";

export type ReduceMotionPreference = "system" | "on" | "off";

export type SupportedLocale = "zh-CN" | "en-US" | "ru-RU" | "ja-JP";

export type LanguagePreference = "system" | SupportedLocale;

export type TrayStyle = "native_menu" | "panel";

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
  tray_style: TrayStyle;
  autostart_minimize_to_tray: boolean;
  check_instance_update: boolean;
  persist_instance_state: boolean;
  ignore_external_path: boolean;
  use_short_instance_id: boolean;
  lock_check_extension_whitelist: boolean;
  theme_preference: ThemePreference;
  language_preference: LanguagePreference;
  reduce_motion_preference: ReduceMotionPreference;
  ignored_launcher_version: string;
  instance_source_enabled: Record<string, boolean>;
  instance_scan_paths: string[];
}

export interface InstanceCapabilities {
  read_only: boolean;
}

export interface DiscoveredInstance {
  source: string;
  id: string;
  name: string;
  webui_url: string | null;
  state: InstanceState;
  capabilities: InstanceCapabilities;
}

export interface InstanceSourceInfo {
  id: string;
  title_key: string;
}

export interface DiscoveredInstancesSnapshot {
  sources: InstanceSourceInfo[];
  instances: DiscoveredInstance[];
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
  restarts_completed: number;
  started_at_ms: number | null;
}

export type InstanceHealthGrade = "healthy" | "warning" | "critical";

export type InstanceHealthCheckKey =
  | "python"
  | "venv"
  | "dependencies"
  | "port"
  | "disk_space"
  | "configuration"
  | "webui";

export type InstanceHealthCheckStatus =
  | "pass"
  | "warning"
  | "fail"
  | "skipped";

export type InstanceHealthDetailCode =
  | "runtime_ready"
  | "runtime_missing"
  | "runtime_failed"
  | "venv_ready"
  | "venv_missing"
  | "venv_failed"
  | "dependencies_ready"
  | "dependencies_issues"
  | "dependencies_check_failed"
  | "requirements_missing"
  | "pip_unavailable"
  | "venv_required"
  | "dependencies_timeout"
  | "port_available"
  | "port_occupied"
  | "port_listening"
  | "port_unreachable"
  | "port_invalid"
  | "port_check_failed"
  | "space_sufficient"
  | "space_low"
  | "space_critical"
  | "space_unavailable"
  | "config_valid"
  | "config_invalid"
  | "config_missing"
  | "webui_reachable"
  | "webui_unreachable"
  | "webui_not_running"
  | "webui_disabled"
  | "webui_host_unsupported";

export interface InstanceHealthCheck {
  key: InstanceHealthCheckKey;
  status: InstanceHealthCheckStatus;
  detail_code: InstanceHealthDetailCode;
  value: string | null;
}

export interface InstanceHealthReport {
  overall: InstanceHealthGrade;
  checks: InstanceHealthCheck[];
}

export interface ResourceSample {
  timestamp: number;
  cpu_percent: number | null;
  memory_bytes: number | null;
  disk_bytes: number | null;
}

export interface ResourceAlertThreshold {
  enabled: boolean;
  cpu_percent: number;
  memory_bytes: number;
  disk_bytes: number;
}

export interface ResourceAlertEvent {
  instance_id: string;
  metric: "cpu" | "memory" | "disk";
  value: number;
  threshold: number;
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

export type TaskKind = "download" | "deploy" | "backup" | "repair";

export type TaskStatus =
  | "queued"
  | "running"
  | "paused"
  | "cancelling"
  | "cancelled"
  | "succeeded"
  | "failed"
  | "interrupted";

export interface TaskError {
  code: number;
  kind: string;
}

export interface TaskRecord {
  id: string;
  kind: TaskKind;
  title_key: string;
  title_args: Record<string, string>;
  target_id: string | null;
  status: TaskStatus;
  progress: number | null;
  step: string;
  message_key: string;
  message: string;
  cancellable: boolean;
  pausable: boolean;
  error: TaskError | null;
  created_at: string;
  updated_at: string;
  started_at: string | null;
  finished_at: string | null;
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
