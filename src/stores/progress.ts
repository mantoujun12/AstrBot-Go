import { ref } from "vue";
import type { DeployProgress, DownloadProgress } from "../types/launcher";
import {
  listenDeployProgress,
  listenDownloadProgress,
} from "../services/events";

const deploys = ref<Record<string, DeployProgress>>({});
const downloads = ref<Record<string, DownloadProgress>>({});
const activeDeployId = ref<string | null>(null);

const clearTimers = new Map<string, number>();
let initPromise: Promise<void> | null = null;

function cancelClear(key: string): void {
  const existing = clearTimers.get(key);
  if (existing !== undefined) {
    clearTimeout(existing);
    clearTimers.delete(key);
  }
}

function scheduleClear(key: string, kind: "deploy" | "download"): void {
  cancelClear(key);

  const timer = window.setTimeout(() => {
    clearTimers.delete(key);
    if (kind === "deploy") {
      const next = { ...deploys.value };
      delete next[key];
      deploys.value = next;
    } else {
      const next = { ...downloads.value };
      delete next[key];
      downloads.value = next;
    }
  }, 2000);

  clearTimers.set(key, timer);
}

function handleDeploy(progress: DeployProgress): void {
  deploys.value = { ...deploys.value, [progress.instance_id]: progress };
  if (progress.step === "done" || progress.step === "error") {
    scheduleClear(progress.instance_id, "deploy");
  } else {
    cancelClear(progress.instance_id);
  }
}

function handleDownload(progress: DownloadProgress): void {
  downloads.value = { ...downloads.value, [progress.id]: progress };
  if (
    progress.step === "done" ||
    progress.step === "error" ||
    progress.step === "cancelled"
  ) {
    scheduleClear(progress.id, "download");
  } else {
    // A new download may reuse an id whose previous entry is still waiting to
    // be cleared; keep the timer from deleting the fresh progress.
    cancelClear(progress.id);
  }
}

export function initProgressStore(): Promise<void> {
  if (initPromise) {
    return initPromise;
  }

  initPromise = (async () => {
    await listenDeployProgress(handleDeploy);
    await listenDownloadProgress(handleDownload);
  })().catch(() => {
    initPromise = null;
  });

  return initPromise;
}

export function beginDeploy(instanceId: string): void {
  cancelClear(instanceId);
  const next = { ...deploys.value };
  delete next[instanceId];
  deploys.value = next;
  activeDeployId.value = instanceId;
}

export function dismissDeploy(): void {
  activeDeployId.value = null;
}

export function useProgressStore() {
  return {
    deploys,
    downloads,
    activeDeployId,
    beginDeploy,
    dismissDeploy,
    init: initProgressStore,
  };
}
