<script setup lang="ts">
import type { UnlistenFn } from "@tauri-apps/api/event";
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
} from "vue";
import { useI18n } from "vue-i18n";
import { useTheme } from "vuetify";
import { applyLanguagePreference } from "../../i18n";
import { getAppSnapshot } from "../../services/app";
import { toUserMessage } from "../../services/errors";
import { listenAppSnapshot, listenTrayPanelShown } from "../../services/events";
import {
  restartInstance,
  startInstance,
  stopInstance,
} from "../../services/instances";
import {
  hideTrayPanel,
  notifyTrayPanelReady,
  quitApp,
  sendTrayPanelAction,
  setTrayPanelHeight,
  type TrayPanelAction,
} from "../../services/tray";
import { showMainWindow } from "../../services/windowControls";
import type {
  AppSnapshot,
  InstanceState,
  InstanceStatus,
} from "../../types/launcher";
import { formatRunDuration } from "../../utils/format";
import { cacheTheme } from "../../utils/themeCache";

const LIST_MAX_HEIGHT = 360;
const PANEL_MIN_HEIGHT = 160;
const PANEL_MAX_HEIGHT = 600;
const PANEL_BORDER_HEIGHT = 2;
const FRAME_PADDING = 6;
const CLOCK_INTERVAL_MS = 30_000;

const STATE_LABEL_KEYS: Record<InstanceState, string> = {
  running: "common.instanceState.running",
  starting: "common.instanceState.starting",
  stopping: "common.instanceState.stopping",
  stopped: "common.instanceState.stopped",
};

type InstanceActionKind = "start" | "stop" | "restart";

interface TrayInstanceItem {
  id: string;
  name: string;
  state: InstanceState;
  stateLabel: string;
  version: string | null;
  port: number;
  duration: string | null;
}

const { t, locale } = useI18n();
const theme = useTheme();

const snapshot = ref<AppSnapshot | null>(null);
const loading = ref(true);
const loadFailed = ref(false);
const openingMainWindow = ref(false);
const quitting = ref(false);
const startedAtNow = ref(Date.now());
const pendingActions = ref<Record<string, InstanceActionKind>>({});
const actionErrors = ref<Record<string, string>>({});
const confirmStopId = ref<string | null>(null);
const headerRef = ref<HTMLElement | null>(null);
const listRef = ref<HTMLElement | null>(null);
const footerRef = ref<HTMLElement | null>(null);

const themeClass = computed(() => theme.themeClasses.value);

const items = computed<TrayInstanceItem[]>(() => {
  const current = snapshot.value;
  if (!current) {
    return [];
  }

  return current.instances.map((instance: InstanceStatus) => {
    // Some instance records already carry the leading `v`, so normalize it
    // instead of rendering a doubled prefix such as `vv4.29.4`.
    const version = instance.version.trim().replace(/^v/i, "");
    const duration =
      (instance.state === "running" || instance.state === "stopping") &&
      instance.started_at_ms !== null
        ? formatRunDuration(instance.started_at_ms, startedAtNow.value)
        : null;

    return {
      id: instance.id,
      name: instance.name,
      state: instance.state,
      stateLabel: t(STATE_LABEL_KEYS[instance.state]),
      version: version === "" ? null : version,
      port: instance.port,
      duration,
    };
  });
});

let loadSequence = 0;
let clockTimer: number | null = null;
let heightFrame: number | null = null;
let heightTimer: number | null = null;
let lastReportedHeight = -1;
let confirmStopTimer: number | null = null;
let disposed = false;
const actionErrorTimers = new Map<string, number>();
const unlisteners: UnlistenFn[] = [];

async function loadSnapshot(): Promise<void> {
  const sequence = ++loadSequence;
  if (snapshot.value === null) {
    loading.value = true;
  }

  try {
    const next = await getAppSnapshot();
    if (disposed || sequence !== loadSequence) {
      return;
    }
    applySnapshot(next);
  } catch {
    if (disposed || sequence !== loadSequence) {
      return;
    }
    loadFailed.value = true;
  } finally {
    if (!disposed && sequence === loadSequence) {
      loading.value = false;
    }
  }
}

// `app-snapshot` already carries the complete snapshot, so the panel can use it
// as-is instead of asking the backend for the same data again. Bumping the
// sequence keeps an in-flight `get_app_snapshot` from applying a stale result
// after a newer event has landed.
function applySnapshot(next: AppSnapshot): void {
  loadSequence += 1;
  if (disposed) {
    return;
  }
  snapshot.value = next;
  loadFailed.value = false;
  loading.value = false;
}

async function openMainWindowWith(
  action?: TrayPanelAction,
): Promise<void> {
  if (openingMainWindow.value) {
    return;
  }

  openingMainWindow.value = true;
  try {
    await showMainWindow();
    if (action) {
      try {
        await sendTrayPanelAction(action);
      } catch {
        // Navigation is best effort; the main window still opens.
      }
    }
    await hideTrayPanel();
  } catch {
    // Keep the panel open when the main window cannot be shown.
  } finally {
    openingMainWindow.value = false;
  }
}

function openMainWindow(): Promise<void> {
  return openMainWindowWith();
}

function openSettings(): Promise<void> {
  return openMainWindowWith({ kind: "settings" });
}

function openInstance(instanceId: string): Promise<void> {
  return openMainWindowWith({ kind: "instance", instanceId });
}

function openInstanceLabel(item: TrayInstanceItem): string {
  const open = t("trayPanel.actions.open", { name: item.name });
  // Each locale joins the two clauses with its own punctuation.
  let separator = ", ";
  if (locale.value === "zh-CN") {
    separator = "，";
  } else if (locale.value === "ja-JP") {
    separator = "、";
  }
  return `${open}${separator}${item.stateLabel}`;
}

function isPending(instanceId: string): boolean {
  return pendingActions.value[instanceId] !== undefined;
}

function isActionDisabled(
  instanceId: string,
  kind: InstanceActionKind,
): boolean {
  // The rows may still show the last snapshot, so acting on it would target a
  // state that can no longer be trusted.
  if (loadFailed.value) {
    return true;
  }
  const pending = pendingActions.value[instanceId];
  return pending !== undefined && pending !== kind;
}

function clearActionError(instanceId: string): void {
  const timer = actionErrorTimers.get(instanceId);
  if (timer !== undefined) {
    window.clearTimeout(timer);
    actionErrorTimers.delete(instanceId);
  }
  if (actionErrors.value[instanceId] === undefined) {
    return;
  }
  const next = { ...actionErrors.value };
  delete next[instanceId];
  actionErrors.value = next;
}

function setActionError(instanceId: string, message: string): void {
  clearActionError(instanceId);
  actionErrors.value = { ...actionErrors.value, [instanceId]: message };
  const timer = window.setTimeout(() => {
    actionErrorTimers.delete(instanceId);
    clearActionError(instanceId);
  }, 6000);
  actionErrorTimers.set(instanceId, timer);
}

function clearStopConfirm(): void {
  if (confirmStopTimer !== null) {
    window.clearTimeout(confirmStopTimer);
    confirmStopTimer = null;
  }
  confirmStopId.value = null;
}

function requestStopConfirmation(instanceId: string): void {
  clearStopConfirm();
  confirmStopId.value = instanceId;
  confirmStopTimer = window.setTimeout(() => {
    confirmStopTimer = null;
    confirmStopId.value = null;
  }, 3000);
}

async function runInstanceAction(
  instanceId: string,
  kind: InstanceActionKind,
  action: () => Promise<unknown>,
): Promise<void> {
  if (isPending(instanceId)) {
    return;
  }

  pendingActions.value = { ...pendingActions.value, [instanceId]: kind };
  clearActionError(instanceId);
  try {
    await action();
  } catch (cause) {
    setActionError(instanceId, toUserMessage(cause));
  } finally {
    if (pendingActions.value[instanceId] === kind) {
      const next = { ...pendingActions.value };
      delete next[instanceId];
      pendingActions.value = next;
    }
  }
}

function handleStart(instanceId: string): void {
  void runInstanceAction(instanceId, "start", () => startInstance(instanceId));
}

function handleRestart(instanceId: string): void {
  void runInstanceAction(instanceId, "restart", () =>
    restartInstance(instanceId),
  );
}

function handleStop(instanceId: string): void {
  if (isPending(instanceId)) {
    return;
  }
  if (confirmStopId.value !== instanceId) {
    requestStopConfirmation(instanceId);
    return;
  }

  clearStopConfirm();
  void runInstanceAction(instanceId, "stop", () => stopInstance(instanceId));
}

async function quitLauncher(): Promise<void> {
  if (quitting.value) {
    return;
  }

  quitting.value = true;
  try {
    await quitApp();
  } catch {
    quitting.value = false;
  }
}

function handleKeydown(event: KeyboardEvent): void {
  if (event.key !== "Escape" || event.defaultPrevented) {
    return;
  }
  event.preventDefault();
  if (confirmStopId.value !== null) {
    clearStopConfirm();
    return;
  }
  void hideTrayPanel().catch(() => undefined);
}

async function reportHeight(): Promise<void> {
  if (heightFrame !== null) {
    cancelAnimationFrame(heightFrame);
    heightFrame = null;
  }
  if (heightTimer !== null) {
    window.clearTimeout(heightTimer);
    heightTimer = null;
  }
  const header = headerRef.value;
  const list = listRef.value;
  const footer = footerRef.value;
  if (!header || !list || !footer) {
    return;
  }

  const desired =
    header.offsetHeight +
    Math.min(list.scrollHeight, LIST_MAX_HEIGHT) +
    footer.offsetHeight +
    PANEL_BORDER_HEIGHT +
    FRAME_PADDING * 2;
  const height = Math.round(
    Math.min(Math.max(desired, PANEL_MIN_HEIGHT), PANEL_MAX_HEIGHT),
  );

  if (height === lastReportedHeight) {
    return;
  }
  lastReportedHeight = height;
  try {
    await setTrayPanelHeight(height);
  } catch {
    // Height is re-reported on the next content change; never fatal here.
  }
}

function scheduleHeightReport(): void {
  if (heightFrame !== null) {
    cancelAnimationFrame(heightFrame);
  }
  // rAF does not run while the panel window is still hidden during prewarm,
  // so a timer fallback makes sure the measured height reaches Rust before the
  // first show. Otherwise the first open would resize a visible window and
  // leave a stale frame behind.
  heightFrame = requestAnimationFrame(reportHeight);
  if (heightTimer !== null) {
    window.clearTimeout(heightTimer);
  }
  heightTimer = window.setTimeout(reportHeight, 120);
}

async function attachListeners(): Promise<void> {
  try {
    const unlisten = await listenAppSnapshot((next) => {
      // The service drops payloads that fail validation, so anything reaching
      // this handler is a complete snapshot.
      applySnapshot(next);
    });
    if (disposed) {
      unlisten();
      return;
    }
    unlisteners.push(unlisten);
  } catch {
    // The panel still refreshes whenever it is shown.
  }

  try {
    const unlisten = await listenTrayPanelShown(() => {
      startedAtNow.value = Date.now();
      void loadSnapshot();
      void nextTick(scheduleHeightReport);
    });
    if (disposed) {
      unlisten();
      return;
    }
    unlisteners.push(unlisten);
  } catch {
    // The panel still loads once on mount.
  }
}

watch(
  () => [
    items.value.length,
    loading.value,
    loadFailed.value,
    snapshot.value === null,
  ],
  () => {
    void nextTick(scheduleHeightReport);
  },
);

// The panel lives in its own webview and can stay hidden while the user changes
// the language in the main window, so follow the snapshot instead of relying on
// the cached locale captured at startup.
watch(
  () => snapshot.value?.config.language_preference,
  (preference) => {
    if (preference) {
      applyLanguagePreference(preference);
    }
  },
  { immediate: true },
);

// The panel window is prewarmed and reused, so it must follow theme changes
// pushed through the snapshot instead of the theme captured at startup.
// `system` is resolved by Vuetify itself.
watch(
  () => snapshot.value?.config.theme_preference,
  (preference) => {
    if (preference) {
      void theme.change(preference);
      cacheTheme(preference);
    }
  },
  { immediate: true },
);

onMounted(() => {
  window.addEventListener("keydown", handleKeydown);
  void attachListeners();
  void loadSnapshot();
  startedAtNow.value = Date.now();
  clockTimer = window.setInterval(() => {
    startedAtNow.value = Date.now();
  }, CLOCK_INTERVAL_MS);
  void nextTick(scheduleHeightReport);

async function announceReadiness(): Promise<void> {
  try {
    if (document.fonts) {
      await document.fonts.ready;
    }
  } catch {
    // The fonts API is unavailable; the measured height below is still valid.
  }
  if (disposed) {
    return;
  }
  scheduleHeightReport();
  // The measured height must reach the backend before readiness, otherwise a
  // queued replay would open the panel at a stale height and visibly resize.
  await reportHeight();
  void notifyTrayPanelReady().catch(() => undefined);
}

void announceReadiness();
});

onBeforeUnmount(() => {
  disposed = true;
  loadSequence += 1;
  window.removeEventListener("keydown", handleKeydown);
  for (const unlisten of unlisteners) {
    unlisten();
  }
  unlisteners.length = 0;
  if (clockTimer !== null) {
    window.clearInterval(clockTimer);
  }
  if (heightFrame !== null) {
    cancelAnimationFrame(heightFrame);
  }
  if (heightTimer !== null) {
    window.clearTimeout(heightTimer);
  }
  if (confirmStopTimer !== null) {
    window.clearTimeout(confirmStopTimer);
  }
  for (const timer of actionErrorTimers.values()) {
    window.clearTimeout(timer);
  }
  actionErrorTimers.clear();
});
</script>

<template>
  <div
    class="tray-panel"
    :class="themeClass"
    role="dialog"
    :aria-label="t('trayPanel.title')"
  >
    <header ref="headerRef" class="tray-panel__header">
      <span class="tray-panel__title">{{ t("trayPanel.title") }}</span>
      <v-btn
        class="tray-panel__icon-button"
        icon="mdi-cog-outline"
        variant="text"
        density="comfortable"
        size="small"
        :aria-label="t('trayPanel.openSettings')"
        @click="openSettings"
      />
    </header>

    <div ref="listRef" class="tray-panel__list">
      <p
        v-if="loadFailed && snapshot !== null"
        class="tray-panel__message tray-panel__message--error"
        role="alert"
      >
        {{ t("trayPanel.loadFailed") }}
      </p>
      <div
        v-if="loading && snapshot === null"
        class="tray-panel__message"
        role="status"
      >
        <v-progress-circular
          indeterminate
          size="20"
          width="2"
          color="primary"
          :aria-label="t('trayPanel.loading')"
        />
      </div>
      <p
        v-else-if="loadFailed && snapshot === null"
        class="tray-panel__message"
        role="alert"
      >
        {{ t("trayPanel.loadFailed") }}
      </p>
      <p
        v-else-if="items.length === 0 && !loadFailed"
        class="tray-panel__message"
      >
        {{ t("trayPanel.empty") }}
      </p>
      <template v-else>
        <div v-for="item in items" :key="item.id" class="tray-panel__item">
          <button
            type="button"
            class="tray-panel__open"
            :aria-label="openInstanceLabel(item)"
            @click="openInstance(item.id)"
          >
            <span
              class="tray-panel__status"
              :class="`tray-panel__status--${item.state}`"
            >
              <span class="tray-panel__dot" aria-hidden="true" />
            </span>
            <span class="tray-panel__content">
              <span class="tray-panel__name">{{ item.name }}</span>
              <span
                v-if="actionErrors[item.id]"
                class="tray-panel__meta tray-panel__meta--error"
                aria-live="polite"
              >
                {{ actionErrors[item.id] }}
              </span>
              <span v-else class="tray-panel__meta">
                <span v-if="item.version" class="tray-panel__version">
                  v{{ item.version }}
                </span>
                <v-chip
                  class="tray-panel__chip"
                  color="info"
                  variant="tonal"
                  size="x-small"
                  prepend-icon="mdi-ethernet"
                >
                  {{ item.port }}
                </v-chip>
                <v-chip
                  v-if="item.duration"
                  class="tray-panel__chip"
                  color="success"
                  variant="tonal"
                  size="x-small"
                  prepend-icon="mdi-timer-outline"
                >
                  {{ item.duration }}
                </v-chip>
              </span>
            </span>
          </button>

          <div class="tray-panel__actions">
            <v-btn
              v-if="item.state === 'stopped'"
              class="tray-panel__action"
              icon="mdi-play"
              variant="text"
              size="small"
              color="success"
              :loading="pendingActions[item.id] === 'start'"
              :disabled="isActionDisabled(item.id, 'start')"
              :aria-label="t('trayPanel.actions.start', { name: item.name })"
              @click="handleStart(item.id)"
            />
            <template v-else-if="item.state === 'running'">
              <v-btn
                class="tray-panel__action"
                :class="{
                  'tray-panel__action--confirm': confirmStopId === item.id,
                }"
                :icon="
                  confirmStopId === item.id ? 'mdi-check-bold' : 'mdi-stop'
                "
                variant="text"
                size="small"
                color="error"
                :loading="pendingActions[item.id] === 'stop'"
                :disabled="isActionDisabled(item.id, 'stop')"
                :aria-label="
                  confirmStopId === item.id
                    ? t('trayPanel.actions.confirmStop', { name: item.name })
                    : t('trayPanel.actions.stop', { name: item.name })
                "
                @click="handleStop(item.id)"
              />
              <v-btn
                class="tray-panel__action"
                icon="mdi-restart"
                variant="text"
                size="small"
                color="primary"
                :loading="pendingActions[item.id] === 'restart'"
                :disabled="isActionDisabled(item.id, 'restart')"
                :aria-label="
                  t('trayPanel.actions.restart', { name: item.name })
                "
                @click="handleRestart(item.id)"
              />
            </template>
          </div>
        </div>
      </template>
    </div>

    <footer ref="footerRef" class="tray-panel__footer">
      <v-btn
        class="tray-panel__footer-button tray-panel__footer-button--primary"
        variant="text"
        size="small"
        color="primary"
        prepend-icon="mdi-open-in-new"
        :disabled="openingMainWindow"
        @click="openMainWindow"
      >
        {{ t("trayPanel.openMainWindow") }}
      </v-btn>
      <v-btn
        class="tray-panel__footer-button"
        variant="text"
        size="small"
        color="error"
        prepend-icon="mdi-power"
        :loading="quitting"
        :disabled="quitting"
        @click="quitLauncher"
      >
        {{ t("trayPanel.quit") }}
      </v-btn>
    </footer>
  </div>
</template>

<style scoped>
.tray-panel {
  box-sizing: border-box;
  width: 360px;
  max-height: 600px;
  margin: 0;
  overflow: hidden;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
  border-radius: var(--astrbot-radius-menu);
  background: rgb(var(--v-theme-surface));
  color: rgb(var(--v-theme-on-surface));
  /* Keep the shadow inside the 6px transparent frame so it is not clipped. */
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.18);
  font-family: "Outfit Variable", "Outfit", sans-serif;
  user-select: none;
}

.tray-panel__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  min-height: 44px;
  padding: 5px 8px 5px 14px;
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.08);
}

.tray-panel__title {
  min-width: 0;
  overflow: hidden;
  font-size: 0.84rem;
  font-weight: 600;
  letter-spacing: 0.01em;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tray-panel__icon-button {
  flex: 0 0 auto;
}

.tray-panel__list {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-height: 360px;
  padding: 6px;
  overflow-y: auto;
  overscroll-behavior: contain;
}

.tray-panel__list::-webkit-scrollbar {
  width: 6px;
}

.tray-panel__list::-webkit-scrollbar-thumb {
  border-radius: var(--astrbot-radius-pill);
  background: rgba(var(--v-theme-on-surface), 0.2);
}

.tray-panel__list::-webkit-scrollbar-track {
  background: transparent;
}

.tray-panel__message {
  display: grid;
  place-items: center;
  min-height: 72px;
  margin: 0;
  padding: 12px;
  color: rgba(var(--v-theme-on-surface), 0.6);
  font-size: 0.8rem;
  text-align: center;
}

.tray-panel__message--error {
  display: block;
  min-height: 0;
  padding: 6px 8px;
  color: rgb(var(--v-theme-error));
  text-align: left;
}

.tray-panel__item {
  display: flex;
  align-items: center;
  gap: 4px;
  width: 100%;
  min-width: 0;
  border-radius: var(--astrbot-radius-control);
}

.tray-panel__item:hover {
  background: rgba(var(--v-theme-on-surface), 0.06);
}

.tray-panel__open {
  display: flex;
  flex: 1 1 auto;
  align-items: center;
  gap: 8px;
  min-width: 0;
  padding: 7px 4px 7px 10px;
  border: 0;
  border-radius: var(--astrbot-radius-control);
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.tray-panel__open:focus-visible {
  outline: 2px solid rgb(var(--v-theme-primary));
  outline-offset: -2px;
}

.tray-panel__status {
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: center;
  width: 8px;
  height: 20px;
}

.tray-panel__dot {
  flex: 0 0 auto;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: rgba(var(--v-theme-on-surface), 0.32);
}

.tray-panel__actions {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  padding-right: 2px;
}

.tray-panel__action {
  width: 30px;
  min-width: 30px;
  height: 30px;
}

.tray-panel__action--confirm {
  background: rgba(var(--v-theme-error), 0.14);
}

.tray-panel__status--running .tray-panel__dot {
  background: rgb(var(--v-theme-success));
  box-shadow: 0 0 0 3px rgba(var(--v-theme-success), 0.18);
}

.tray-panel__status--starting .tray-panel__dot {
  background: rgb(var(--v-theme-warning));
}

.tray-panel__status--stopping .tray-panel__dot {
  background: rgb(var(--v-theme-info));
}

.tray-panel__status--stopped .tray-panel__dot {
  background: rgba(var(--v-theme-on-surface), 0.3);
}

.tray-panel__content {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.tray-panel__name {
  overflow: hidden;
  font-size: 0.82rem;
  font-weight: 600;
  line-height: 1.25;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tray-panel__meta {
  display: flex;
  align-items: center;
  gap: 5px;
  min-width: 0;
  min-height: 20px;
  overflow: hidden;
  font-size: 0.72rem;
  font-variant-numeric: tabular-nums;
  line-height: 1.25;
}

.tray-panel__version {
  flex: 0 1 auto;
  min-width: 0;
  overflow: hidden;
  color: rgba(var(--v-theme-on-surface), 0.6);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tray-panel__chip {
  flex: 0 0 auto;
  height: 20px;
  font-size: 0.68rem;
}

.tray-panel__meta--error {
  display: block;
  overflow: hidden;
  color: rgb(var(--v-theme-error));
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tray-panel__footer {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 9px;
  border-top: 1px solid rgba(var(--v-theme-on-surface), 0.08);
}

.tray-panel__footer-button {
  min-height: 40px;
  height: 40px;
}

.tray-panel__footer .tray-panel__footer-button--primary {
  flex: 1 1 auto;
  justify-content: flex-start;
}

.tray-panel__footer .tray-panel__footer-button--primary :deep(.v-btn__content) {
  justify-content: flex-start;
}
</style>
