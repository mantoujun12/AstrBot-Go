import { invoke } from "@tauri-apps/api/core";
import type { TaskKind, TaskRecord, TaskStatus } from "../types/launcher";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isTaskKind(value: unknown): value is TaskKind {
  return (
    value === "download" ||
    value === "deploy" ||
    value === "backup" ||
    value === "repair"
  );
}

function isTaskStatus(value: unknown): value is TaskStatus {
  return (
    value === "queued" ||
    value === "running" ||
    value === "paused" ||
    value === "cancelling" ||
    value === "cancelled" ||
    value === "succeeded" ||
    value === "failed" ||
    value === "interrupted"
  );
}

export function toTaskRecord(value: unknown): TaskRecord | null {
  if (!isRecord(value)) {
    return null;
  }
  if (
    typeof value.id !== "string" ||
    !isTaskKind(value.kind) ||
    typeof value.title_key !== "string" ||
    !isRecord(value.title_args) ||
    !isTaskStatus(value.status) ||
    typeof value.step !== "string" ||
    typeof value.message_key !== "string" ||
    typeof value.message !== "string"
  ) {
    return null;
  }
  const titleArgs: Record<string, string> = {};
  for (const [key, arg] of Object.entries(value.title_args)) {
    if (typeof arg === "string") {
      titleArgs[key] = arg;
    }
  }
  const errorRecord = isRecord(value.error) ? value.error : null;
  const error = errorRecord
    ? {
        code: typeof errorRecord.code === "number" ? errorRecord.code : 0,
        kind: typeof errorRecord.kind === "string" ? errorRecord.kind : "other",
      }
    : null;
  return {
    id: value.id,
    kind: value.kind,
    title_key: value.title_key,
    title_args: titleArgs,
    target_id: typeof value.target_id === "string" ? value.target_id : null,
    status: value.status,
    progress: typeof value.progress === "number" ? value.progress : null,
    step: value.step,
    message_key: value.message_key,
    message: value.message,
    cancellable: value.cancellable === true,
    pausable: value.pausable === true,
    error,
    created_at: typeof value.created_at === "string" ? value.created_at : "",
    updated_at: typeof value.updated_at === "string" ? value.updated_at : "",
    started_at: typeof value.started_at === "string" ? value.started_at : null,
    finished_at:
      typeof value.finished_at === "string" ? value.finished_at : null,
  };
}

export async function listTasks(): Promise<TaskRecord[]> {
  const records = await invoke<unknown>("list_tasks");
  if (!Array.isArray(records)) {
    throw new Error("Invalid task list response");
  }
  return records
    .map(toTaskRecord)
    .filter((task): task is TaskRecord => task !== null);
}

export function cancelTask(taskId: string): Promise<void> {
  return invoke("cancel_task", { taskId });
}

export function pauseTask(taskId: string): Promise<void> {
  return invoke("pause_task", { taskId });
}

export function resumeTask(taskId: string): Promise<void> {
  return invoke("resume_task", { taskId });
}

export function removeTask(taskId: string): Promise<void> {
  return invoke("remove_task", { taskId });
}

export function clearFinishedTasks(): Promise<void> {
  return invoke("clear_finished_tasks");
}
