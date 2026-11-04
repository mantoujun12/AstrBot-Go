import { computed, ref } from "vue";
import { parseBackendError, toUserMessage } from "../services/errors";
import {
  listenTaskRemoved,
  listenTaskUpdated,
} from "../services/events";
import {
  cancelTask as cancelTaskRequest,
  clearFinishedTasks as clearFinishedTasksRequest,
  listTasks,
  pauseTask as pauseTaskRequest,
  removeTask as removeTaskRequest,
  resumeTask as resumeTaskRequest,
} from "../services/tasks";
import type { TaskRecord, TaskStatus } from "../types/launcher";
import { refresh } from "./launcher";
import { showToast } from "./toast";

const tasks = ref<Map<string, TaskRecord>>(new Map());
const loaded = ref(false);
const error = ref<string | null>(null);
let initPromise: Promise<void> | null = null;
let unlistenUpdated: (() => void) | null = null;
let unlistenRemoved: (() => void) | null = null;

const terminalStatuses: ReadonlySet<TaskStatus> = new Set([
  "cancelled",
  "succeeded",
  "failed",
  "interrupted",
]);

function isActive(status: TaskStatus): boolean {
  return !terminalStatuses.has(status);
}

function upsertTask(task: TaskRecord): void {
  const previous = tasks.value.get(task.id);
  const next = new Map(tasks.value);
  next.set(task.id, task);
  tasks.value = next;

  if (previous && isActive(previous.status) && !isActive(task.status)) {
    void refresh();
  }
}

function removeTaskFromStore(taskId: string): void {
  if (!tasks.value.has(taskId)) {
    return;
  }
  const next = new Map(tasks.value);
  next.delete(taskId);
  tasks.value = next;
}

async function handleTaskError(action: () => Promise<void>): Promise<void> {
  try {
    await action();
  } catch (cause) {
    showToast(parseBackendError(cause).message, "error");
  }
}

export function initTaskStore(): Promise<void> {
  if (initPromise) {
    return initPromise;
  }

  initPromise = (async () => {
    try {
      unlistenUpdated = await listenTaskUpdated(upsertTask);
      unlistenRemoved = await listenTaskRemoved(removeTaskFromStore);
      const records = await listTasks();
      tasks.value = new Map(records.map((task) => [task.id, task]));
      error.value = null;
      loaded.value = true;
    } catch (cause) {
      unlistenUpdated?.();
      unlistenRemoved?.();
      unlistenUpdated = null;
      unlistenRemoved = null;
      error.value = toUserMessage(cause);
      loaded.value = true;
    }
  })();

  return initPromise;
}

export function useTaskStore() {
  const sortedTasks = computed(() =>
    [...tasks.value.values()].sort((left, right) =>
      right.created_at.localeCompare(left.created_at),
    ),
  );
  const activeCount = computed(
    () => sortedTasks.value.filter((task) => isActive(task.status)).length,
  );

  return {
    loaded,
    error,
    tasks: sortedTasks,
    activeCount,
    init: initTaskStore,
    cancelTask: (taskId: string) => handleTaskError(() => cancelTaskRequest(taskId)),
    pauseTask: (taskId: string) => handleTaskError(() => pauseTaskRequest(taskId)),
    resumeTask: (taskId: string) => handleTaskError(() => resumeTaskRequest(taskId)),
    removeTask: (taskId: string) => handleTaskError(() => removeTaskRequest(taskId)),
    clearFinishedTasks: () => handleTaskError(clearFinishedTasksRequest),
  };
}
