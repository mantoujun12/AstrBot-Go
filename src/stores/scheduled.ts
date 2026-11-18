import { ref } from "vue";
import { parseBackendError, toUserMessage } from "../services/errors";
import { listenScheduledJobsUpdated } from "../services/events";
import {
  deleteScheduledJob as deleteScheduledJobRequest,
  listScheduledJobs,
  runScheduledJobNow as runScheduledJobNowRequest,
  saveScheduledJob as saveScheduledJobRequest,
  setScheduledJobEnabled as setScheduledJobEnabledRequest,
} from "../services/scheduled";
import type { ScheduledJob } from "../types/launcher";
import { showToast } from "./toast";

const jobs = ref<ScheduledJob[]>([]);
const loaded = ref(false);
const error = ref<string | null>(null);
const busyIds = ref<Set<string>>(new Set());
let initPromise: Promise<void> | null = null;
let unlisten: (() => void) | null = null;

function sortJobs(list: ScheduledJob[]): ScheduledJob[] {
  return [...list].sort((left, right) => {
    if (left.enabled !== right.enabled) {
      return left.enabled ? -1 : 1;
    }
    if (left.next_run_at === null || right.next_run_at === null) {
      if (left.next_run_at === right.next_run_at) {
        return left.name.localeCompare(right.name);
      }
      return left.next_run_at === null ? 1 : -1;
    }
    const byNextRun = left.next_run_at.localeCompare(right.next_run_at);
    return byNextRun !== 0 ? byNextRun : left.name.localeCompare(right.name);
  });
}

function replaceJobs(list: ScheduledJob[]): void {
  jobs.value = sortJobs(list);
}

function setBusy(jobId: string, busy: boolean): void {
  const next = new Set(busyIds.value);
  if (busy) {
    next.add(jobId);
  } else {
    next.delete(jobId);
  }
  busyIds.value = next;
}

function isBusy(jobId: string): boolean {
  return busyIds.value.has(jobId);
}

async function initScheduledStore(): Promise<void> {
  if (initPromise) {
    return initPromise;
  }

  initPromise = (async () => {
    try {
      unlisten = await listenScheduledJobsUpdated(replaceJobs);
      replaceJobs(await listScheduledJobs());
      error.value = null;
      loaded.value = true;
    } catch (cause) {
      unlisten?.();
      unlisten = null;
      error.value = toUserMessage(cause);
      loaded.value = true;
      initPromise = null;
    }
  })();

  return initPromise;
}

async function save(job: ScheduledJob): Promise<boolean> {
  if (isBusy(job.id)) {
    return false;
  }
  setBusy(job.id, true);
  try {
    replaceJobs(await saveScheduledJobRequest(job));
    return true;
  } catch (cause) {
    showToast(parseBackendError(cause).message, "error");
    return false;
  } finally {
    setBusy(job.id, false);
  }
}

async function remove(jobId: string): Promise<void> {
  if (isBusy(jobId)) {
    return;
  }
  setBusy(jobId, true);
  try {
    replaceJobs(await deleteScheduledJobRequest(jobId));
  } catch (cause) {
    showToast(parseBackendError(cause).message, "error");
  } finally {
    setBusy(jobId, false);
  }
}

async function setEnabled(jobId: string, enabled: boolean): Promise<void> {
  if (isBusy(jobId)) {
    return;
  }
  setBusy(jobId, true);
  try {
    replaceJobs(await setScheduledJobEnabledRequest(jobId, enabled));
  } catch (cause) {
    showToast(parseBackendError(cause).message, "error");
  } finally {
    setBusy(jobId, false);
  }
}

async function runNow(jobId: string): Promise<void> {
  if (isBusy(jobId)) {
    return;
  }
  setBusy(jobId, true);
  try {
    await runScheduledJobNowRequest(jobId);
  } catch (cause) {
    showToast(parseBackendError(cause).message, "error");
  } finally {
    setBusy(jobId, false);
  }
}

export function useScheduledStore() {
  return {
    loaded,
    error,
    jobs,
    busyIds,
    init: initScheduledStore,
    isBusy,
    save,
    remove,
    setEnabled,
    runNow,
  };
}