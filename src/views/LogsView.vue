<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute } from "vue-router";
import LogConsole from "../components/logs/LogConsole.vue";
import { useLauncherStore } from "../stores/launcher";
import { useLogStore } from "../stores/logs";
import { showToast } from "../stores/toast";
import type { LogEntry, LogLevel } from "../types/launcher";
import { formatLogTime, stripAnsi } from "../utils/format";

interface LevelOption {
  value: LogLevel;
  label: string;
  color: string;
}

interface SourceOption {
  title: string;
  value: string;
}

const SOURCE_ALL = "all";
const SOURCE_SYSTEM = "system";
const MAX_RENDERED_ENTRIES = 800;
const SEARCH_DEBOUNCE_MS = 200;
const MIN_FONT_SIZE = 10;
const MAX_FONT_SIZE = 20;
const DEFAULT_FONT_SIZE = 12;
const FONT_SIZE_STORAGE_KEY = "astrbot-launcher.logs.font-size";

const { t } = useI18n();

const LEVEL_OPTIONS = computed<LevelOption[]>(() => [
  { value: "debug", label: t("logs.levels.debug"), color: "secondary" },
  { value: "info", label: t("logs.levels.info"), color: "info" },
  { value: "warn", label: t("logs.levels.warn"), color: "warning" },
  { value: "error", label: t("logs.levels.error"), color: "error" },
]);

const LEVEL_TEXT: Record<LogLevel, string> = {
  debug: "DEBUG",
  info: "INFO",
  warn: "WARN",
  error: "ERROR",
};

const route = useRoute();
const { entries, clearLogs } = useLogStore();
const { instances } = useLauncherStore();

const sourceOptions = computed<SourceOption[]>(() => [
  { title: t("logs.sources.all"), value: SOURCE_ALL },
  { title: t("logs.sources.system"), value: SOURCE_SYSTEM },
  ...instances.value.map((instance) => ({
    title: instance.name || instance.id,
    value: instance.id,
  })),
]);

const sourceLabels = computed<Record<string, string>>(() => {
  const labels: Record<string, string> = {
    [SOURCE_SYSTEM]: t("logs.sources.system"),
  };
  for (const instance of instances.value) {
    labels[instance.id] = instance.name;
  }
  return labels;
});

function readRouteSource(): string {
  const raw = route.query.source;
  const value = Array.isArray(raw) ? raw[0] : raw;
  return typeof value === "string" ? value : "";
}

const selectedSource = ref<string>(SOURCE_ALL);
let initialSourceResolved = false;

function resolveInitialSource(): void {
  if (initialSourceResolved) {
    return;
  }

  const requested = readRouteSource();
  if (requested === "") {
    selectedSource.value = SOURCE_ALL;
    initialSourceResolved = true;
    return;
  }

  if (instances.value.some((instance) => instance.id === requested)) {
    selectedSource.value = requested;
    initialSourceResolved = true;
    return;
  }

  if (instances.value.length > 0) {
    selectedSource.value = SOURCE_ALL;
    initialSourceResolved = true;
  }
}

resolveInitialSource();

watch(
  () => instances.value,
  () => {
    resolveInitialSource();

    if (
      selectedSource.value !== SOURCE_ALL &&
      selectedSource.value !== SOURCE_SYSTEM &&
      !instances.value.some(
        (instance) => instance.id === selectedSource.value,
      )
    ) {
      selectedSource.value = SOURCE_ALL;
    }
  },
);

const selectedLevels = ref<LogLevel[]>(
  LEVEL_OPTIONS.value.map((option) => option.value),
);

const searchInput = ref("");
const searchKeyword = ref("");
let searchTimer: number | null = null;

watch(searchInput, (value) => {
  if (searchTimer !== null) {
    window.clearTimeout(searchTimer);
  }

  const nextKeyword = (value ?? "").trim();
  searchTimer = window.setTimeout(() => {
    searchTimer = null;
    searchKeyword.value = nextKeyword;
  }, SEARCH_DEBOUNCE_MS);
});

onBeforeUnmount(() => {
  if (searchTimer !== null) {
    window.clearTimeout(searchTimer);
    searchTimer = null;
  }
});

const filteredEntries = computed<LogEntry[]>(() => {
  const levels = new Set(selectedLevels.value);
  const source = selectedSource.value;
  const keyword = searchKeyword.value.toLowerCase();

  return entries.value.filter((entry) => {
    if (!levels.has(entry.level)) {
      return false;
    }
    if (source !== SOURCE_ALL && entry.source !== source) {
      return false;
    }
    if (
      keyword !== "" &&
      !stripAnsi(entry.message).toLowerCase().includes(keyword)
    ) {
      return false;
    }
    return true;
  });
});

const renderedEntries = computed<LogEntry[]>(() => {
  const filtered = filteredEntries.value;
  return filtered.length > MAX_RENDERED_ENTRIES
    ? filtered.slice(filtered.length - MAX_RENDERED_ENTRIES)
    : filtered;
});

const filterKey = computed(() =>
  [
    selectedSource.value,
    selectedLevels.value.slice().sort().join(","),
    searchKeyword.value,
  ].join("|"),
);

function readStoredFontSize(): number {
  try {
    const raw = window.localStorage.getItem(FONT_SIZE_STORAGE_KEY);
    if (raw === null) {
      return DEFAULT_FONT_SIZE;
    }
    const parsed = Number(raw);
    if (!Number.isFinite(parsed)) {
      return DEFAULT_FONT_SIZE;
    }
    return Math.min(MAX_FONT_SIZE, Math.max(MIN_FONT_SIZE, Math.trunc(parsed)));
  } catch {
    return DEFAULT_FONT_SIZE;
  }
}

const autoScroll = ref(true);
const fontSize = ref(readStoredFontSize());
const copying = ref(false);

watch(fontSize, (value) => {
  try {
    window.localStorage.setItem(FONT_SIZE_STORAGE_KEY, String(value));
  } catch {
    // A blocked localStorage must not break the log page.
  }
});

function decreaseFontSize(): void {
  fontSize.value = Math.max(MIN_FONT_SIZE, fontSize.value - 1);
}

function increaseFontSize(): void {
  fontSize.value = Math.min(MAX_FONT_SIZE, fontSize.value + 1);
}

function resolveSourceLabel(source: string): string {
  const label = sourceLabels.value[source];
  if (typeof label === "string" && label !== "") {
    return label;
  }
  return source === "" ? "—" : source;
}

async function copyLogs(): Promise<void> {
  if (copying.value || filteredEntries.value.length === 0) {
    return;
  }

  copying.value = true;
  try {
    const text = filteredEntries.value
      .map((entry) =>
        [
          formatLogTime(entry.timestamp),
          LEVEL_TEXT[entry.level],
          resolveSourceLabel(entry.source),
          stripAnsi(entry.message),
        ].join(" "),
      )
      .join("\n");

    const clipboard = navigator.clipboard;
    if (!clipboard || typeof clipboard.writeText !== "function") {
      throw new Error("clipboard unavailable");
    }
    await clipboard.writeText(text);
    showToast(t("logs.copied"));
  } catch {
    showToast(t("logs.copyFailed"), "error");
  } finally {
    copying.value = false;
  }
}

function handleClear(): void {
  clearLogs();
}
</script>

<template>
  <section class="page-shell logs-page" :aria-label="t('logs.title')">
    <div class="logs-toolbar">
      <v-select
        v-model="selectedSource"
        class="logs-source"
        :items="sourceOptions"
        item-title="title"
        item-value="value"
        density="compact"
        variant="outlined"
        hide-details
        rounded="lg"
        :label="t('logs.sourceLabel')"
        :aria-label="t('logs.sourceFilter')"
      />

      <v-chip-group
        v-model="selectedLevels"
        class="logs-levels"
        multiple
        filter
        role="group"
        :aria-label="t('logs.levelFilter')"
      >
        <v-chip
          v-for="level in LEVEL_OPTIONS"
          :key="level.value"
          :value="level.value"
          :color="level.color"
          size="small"
          role="checkbox"
          :aria-checked="selectedLevels.includes(level.value)"
        >
          {{ level.label }}
        </v-chip>
      </v-chip-group>

      <v-text-field
        v-model="searchInput"
        class="logs-search"
        density="compact"
        variant="outlined"
        hide-details
        rounded="lg"
        clearable
        prepend-inner-icon="mdi-magnify"
        :label="t('logs.search')"
        :aria-label="t('logs.searchAria')"
      />

      <div class="logs-actions page-row">
        <v-switch
          v-model="autoScroll"
          class="logs-autoscroll"
          color="primary"
          density="compact"
          hide-details
          inset
          :label="t('logs.autoScroll')"
          :aria-label="t('logs.autoScroll')"
        />

        <div
          class="logs-font-size"
          role="group"
          :aria-label="t('logs.fontSize')"
        >
          <v-btn
            icon
            size="x-small"
            variant="text"
            :disabled="fontSize <= MIN_FONT_SIZE"
            :aria-label="t('logs.decreaseFontSize')"
            @click="decreaseFontSize"
          >
            <v-icon icon="mdi-minus" size="16" aria-hidden="true" />
          </v-btn>
          <output
            class="logs-font-size__value page-nums"
            :aria-label="t('logs.currentFontSize')"
          >
            {{ fontSize }}
          </output>
          <v-btn
            icon
            size="x-small"
            variant="text"
            :disabled="fontSize >= MAX_FONT_SIZE"
            :aria-label="t('logs.increaseFontSize')"
            @click="increaseFontSize"
          >
            <v-icon icon="mdi-plus" size="16" aria-hidden="true" />
          </v-btn>
        </div>

        <v-btn
          size="small"
          variant="text"
          prepend-icon="mdi-content-copy"
          :loading="copying"
          :disabled="filteredEntries.length === 0"
          :aria-label="t('logs.copyLogs')"
          @click="copyLogs"
        >
          {{ t("common.actions.copy") }}
        </v-btn>

        <v-btn
          size="small"
          variant="text"
          prepend-icon="mdi-delete-outline"
          :disabled="entries.length === 0"
          :aria-label="t('logs.clearLogs')"
          @click="handleClear"
        >
          {{ t("logs.clear") }}
        </v-btn>
      </div>
    </div>

    <LogConsole
      :entries="renderedEntries"
      :source-labels="sourceLabels"
      :keyword="searchKeyword"
      :auto-scroll="autoScroll"
      :font-size="fontSize"
      :filter-key="filterKey"
    />
  </section>
</template>

<style scoped>
.logs-page {
  display: flex;
  flex-direction: column;
  gap: 12px;
  height: 100%;
  min-height: 0;
}

.logs-toolbar {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px 10px;
  padding: 8px 10px;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.1);
  border-radius: 12px;
  background: rgb(var(--v-theme-surface));
}

.logs-source {
  flex: 0 1 174px;
  min-width: 150px;
}

.logs-levels {
  flex: 0 0 auto;
}

.logs-search {
  flex: 1 1 220px;
  min-width: 180px;
  max-width: 520px;
}

.logs-actions {
  flex: 0 0 auto;
  gap: 2px 4px;
  margin-left: auto;
}

.logs-autoscroll {
  flex: 0 0 auto;
}

.logs-autoscroll :deep(.v-selection-control) {
  min-height: 32px;
}

.logs-autoscroll :deep(.v-label) {
  font-size: 0.8rem;
  opacity: 1;
}

.logs-font-size {
  display: flex;
  align-items: center;
  height: 32px;
  padding: 0 2px;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
  border-radius: 8px;
}

.logs-font-size__value {
  min-width: 20px;
  color: rgba(var(--v-theme-on-surface), 0.62);
  font-size: 0.72rem;
  text-align: center;
}

@media (max-width: 760px) {
  .logs-search {
    max-width: none;
  }
}
</style>
