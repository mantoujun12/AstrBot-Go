import { invoke } from "@tauri-apps/api/core";
import type {
  ScheduledAction,
  ScheduledJob,
  ScheduleMode,
} from "../types/launcher";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isScheduledAction(value: unknown): value is ScheduledAction {
  return (
    value === "start_instance" ||
    value === "stop_instance" ||
    value === "backup" ||
    value === "clear_cache"
  );
}

function isScheduleMode(value: unknown): value is ScheduleMode {
  return (
    value === "once" ||
    value === "interval" ||
    value === "daily" ||
    value === "weekly"
  );
}

function isOptionalString(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function isOptionalBoolean(value: unknown): value is boolean | null {
  return value === null || typeof value === "boolean";
}

function isOptionalNumber(value: unknown): value is number | null {
  return value === null || (typeof value === "number" && Number.isFinite(value));
}

export function toScheduledJob(value: unknown): ScheduledJob | null {
  if (!isRecord(value)) {
    return null;
  }
  if (
    typeof value.id !== "string" ||
    typeof value.name !== "string" ||
    !isScheduledAction(value.action) ||
    typeof value.instance_id !== "string" ||
    !isScheduleMode(value.mode) ||
    !isOptionalString(value.run_at) ||
    !isOptionalNumber(value.interval_minutes) ||
    !isOptionalString(value.time) ||
    !isOptionalNumber(value.weekday) ||
    typeof value.enabled !== "boolean" ||
    !isOptionalString(value.next_run_at) ||
    !isOptionalString(value.last_run_at) ||
    !isOptionalBoolean(value.last_success) ||
    !isOptionalString(value.last_error)
  ) {
    return null;
  }
  const weekday = value.weekday;
  if (
    weekday !== null &&
    (!Number.isInteger(weekday) || weekday < 0 || weekday > 6)
  ) {
    return null;
  }
  return {
    id: value.id,
    name: value.name,
    action: value.action,
    instance_id: value.instance_id,
    mode: value.mode,
    run_at: value.run_at,
    interval_minutes: value.interval_minutes,
    time: value.time,
    weekday,
    enabled: value.enabled,
    next_run_at: value.next_run_at,
    last_run_at: value.last_run_at,
    last_success: value.last_success,
    last_error: value.last_error,
  };
}

function toScheduledJobList(value: unknown): ScheduledJob[] {
  if (!Array.isArray(value)) {
    throw new Error("Invalid scheduled job list response");
  }
  return value
    .map(toScheduledJob)
    .filter((job): job is ScheduledJob => job !== null);
}

export async function listScheduledJobs(): Promise<ScheduledJob[]> {
  return toScheduledJobList(await invoke<unknown>("list_scheduled_jobs"));
}

export async function saveScheduledJob(
  job: ScheduledJob,
): Promise<ScheduledJob[]> {
  return toScheduledJobList(await invoke<unknown>("save_scheduled_job", { job }));
}

export async function deleteScheduledJob(
  jobId: string,
): Promise<ScheduledJob[]> {
  return toScheduledJobList(
    await invoke<unknown>("delete_scheduled_job", { jobId }),
  );
}

export async function setScheduledJobEnabled(
  jobId: string,
  enabled: boolean,
): Promise<ScheduledJob[]> {
  return toScheduledJobList(
    await invoke<unknown>("set_scheduled_job_enabled", { jobId, enabled }),
  );
}

export function runScheduledJobNow(jobId: string): Promise<void> {
  return invoke("run_scheduled_job_now", { jobId });
}