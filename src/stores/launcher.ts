import { computed, ref } from "vue";
import type { AppSnapshot } from "../types/launcher";
import { getAppSnapshot } from "../services/app";
import { toUserMessage } from "../services/errors";
import { listenAppSnapshot } from "../services/events";

const snapshot = ref<AppSnapshot | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);

const instances = computed(() => snapshot.value?.instances ?? []);
const versions = computed(() => snapshot.value?.versions ?? []);
const backups = computed(() => snapshot.value?.backups ?? []);
const components = computed(() => snapshot.value?.components.components ?? []);
const config = computed(() => snapshot.value?.config ?? null);
const dataDir = computed(() => snapshot.value?.data_dir ?? "");

let initPromise: Promise<void> | null = null;
let refreshChain: Promise<void> | null = null;
let refreshQueued = false;
let stopListening: (() => void) | null = null;
let eventTimer: ReturnType<typeof setTimeout> | null = null;

async function loadOnce(): Promise<void> {
  loading.value = true;
  try {
    snapshot.value = await getAppSnapshot();
    error.value = null;
  } catch (cause) {
    error.value = toUserMessage(cause);
  } finally {
    loading.value = false;
  }
}

/**
 * Refreshes the snapshot with a serialized request chain. Requests issued
 * while a refresh is running are coalesced into one follow-up run, so a later
 * result always reflects at least the latest invalidation.
 */
export function refresh(): Promise<void> {
  if (refreshChain) {
    refreshQueued = true;
    return refreshChain;
  }

  refreshChain = (async () => {
    do {
      refreshQueued = false;
      await loadOnce();
    } while (refreshQueued);
  })().finally(() => {
    refreshChain = null;
  });

  return refreshChain;
}

function scheduleRefresh(): void {
  if (eventTimer !== null) {
    clearTimeout(eventTimer);
  }
  eventTimer = setTimeout(() => {
    eventTimer = null;
    void refresh();
  }, 120);
}

export function initLauncherStore(): Promise<void> {
  if (initPromise) {
    return initPromise;
  }

  initPromise = (async () => {
    try {
      stopListening = await listenAppSnapshot(() => {
        scheduleRefresh();
      });
    } catch (cause) {
      error.value = toUserMessage(cause);
    }
    await refresh();
  })();

  return initPromise;
}

export function disposeLauncherStore(): void {
  stopListening?.();
  stopListening = null;
  if (eventTimer !== null) {
    clearTimeout(eventTimer);
    eventTimer = null;
  }
}

/**
 * Runs a mutating command and refreshes the snapshot afterwards. The real
 * backend only pushes `app-snapshot` on process lifecycle changes, so create,
 * update, delete, config, version and backup commands need this refresh.
 */
export async function runMutation<T>(task: () => Promise<T>): Promise<T> {
  try {
    const result = await task();
    await refresh();
    return result;
  } catch (cause) {
    void refresh();
    throw cause;
  }
}

export function useLauncherStore() {
  return {
    snapshot,
    instances,
    versions,
    backups,
    components,
    config,
    dataDir,
    loading,
    error,
    refresh,
    runMutation,
    init: initLauncherStore,
  };
}
