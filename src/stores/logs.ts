import { ref, shallowRef } from "vue";
import type { LogEntry } from "../types/launcher";
import { toUserMessage } from "../services/errors";
import { listenLogEntry } from "../services/events";

const MAX_ENTRIES = 2000;
const FLUSH_INTERVAL_MS = 80;

const entries = shallowRef<LogEntry[]>([]);
const error = ref<string | null>(null);

let pending: LogEntry[] = [];
let flushTimer: number | null = null;
let initPromise: Promise<void> | null = null;

function flush(): void {
  flushTimer = null;
  if (pending.length === 0) {
    return;
  }

  const next = entries.value.concat(pending);
  pending = [];
  entries.value =
    next.length > MAX_ENTRIES ? next.slice(next.length - MAX_ENTRIES) : next;
}

function enqueue(entry: LogEntry): void {
  pending.push(entry);
  if (flushTimer === null) {
    flushTimer = window.setTimeout(flush, FLUSH_INTERVAL_MS);
  }
}

export function initLogStore(): Promise<void> {
  if (initPromise) {
    return initPromise;
  }

  initPromise = (async () => {
    await listenLogEntry(enqueue);
    error.value = null;
  })().catch((cause) => {
    error.value = toUserMessage(cause);
    initPromise = null;
  });

  return initPromise;
}

export function clearLogs(): void {
  pending = [];
  entries.value = [];
}

export function useLogStore() {
  return {
    entries,
    error,
    clearLogs,
    init: initLogStore,
  };
}
