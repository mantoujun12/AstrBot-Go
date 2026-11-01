import { shallowRef } from "vue";
import type { LogEntry } from "../types/launcher";
import { listenLogEntry } from "../services/events";

const MAX_ENTRIES = 2000;
const FLUSH_INTERVAL_MS = 80;

const entries = shallowRef<LogEntry[]>([]);

let pending: LogEntry[] = [];
let flushTimer: number | null = null;
let initPromise: Promise<void> | null = null;
let stopListening: (() => void) | null = null;

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
    stopListening = await listenLogEntry(enqueue);
  })().catch(() => {
    initPromise = null;
  });

  return initPromise;
}

export function clearLogs(): void {
  pending = [];
  entries.value = [];
}

export function disposeLogStore(): void {
  stopListening?.();
  stopListening = null;
  if (flushTimer !== null) {
    clearTimeout(flushTimer);
    flushTimer = null;
  }
}

export function useLogStore() {
  return {
    entries,
    clearLogs,
    init: initLogStore,
  };
}
