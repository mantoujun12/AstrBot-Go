<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { LogEntry, LogLevel } from "../../types/launcher";
import { formatLogTime, stripAnsi } from "../../utils/format";

interface MessageSegment {
  text: string;
  highlight: boolean;
}

interface LogRow {
  id: number;
  time: string;
  level: LogLevel;
  levelText: string;
  source: string;
  segments: MessageSegment[];
}

const props = defineProps<{
  entries: LogEntry[];
  sourceLabels: Record<string, string>;
  keyword: string;
  autoScroll: boolean;
  fontSize: number;
  filterKey: string;
}>();

const { t } = useI18n();

const LEVEL_TEXT: Record<LogLevel, string> = {
  debug: "DEBUG",
  info: "INFO",
  warn: "WARN",
  error: "ERROR",
};

const BOTTOM_THRESHOLD_PX = 24;

const scrollEl = ref<HTMLElement | null>(null);
const pinnedToBottom = ref(true);

const entryIds = new WeakMap<LogEntry, number>();
let nextEntryId = 1;

function getEntryId(entry: LogEntry): number {
  const existing = entryIds.get(entry);
  if (existing !== undefined) {
    return existing;
  }
  const id = nextEntryId;
  nextEntryId += 1;
  entryIds.set(entry, id);
  return id;
}

function resolveSource(source: string): string {
  const label = props.sourceLabels[source];
  if (typeof label === "string" && label !== "") {
    return label;
  }
  return source === "" ? "—" : source;
}

function splitMessage(message: string, keyword: string): MessageSegment[] {
  if (keyword === "") {
    return [{ text: message, highlight: false }];
  }

  const lowerMessage = message.toLowerCase();
  const lowerKeyword = keyword.toLowerCase();
  const segments: MessageSegment[] = [];
  let cursor = 0;
  let matchIndex = lowerMessage.indexOf(lowerKeyword);

  while (matchIndex !== -1) {
    if (matchIndex > cursor) {
      segments.push({
        text: message.slice(cursor, matchIndex),
        highlight: false,
      });
    }
    segments.push({
      text: message.slice(matchIndex, matchIndex + keyword.length),
      highlight: true,
    });
    cursor = matchIndex + keyword.length;
    matchIndex = lowerMessage.indexOf(lowerKeyword, cursor);
  }

  if (cursor < message.length) {
    segments.push({ text: message.slice(cursor), highlight: false });
  }

  return segments;
}

const rows = computed<LogRow[]>(() => {
  const keyword = props.keyword.trim();
  return props.entries.map((entry) => ({
    id: getEntryId(entry),
    time: formatLogTime(entry.timestamp),
    level: entry.level,
    levelText: LEVEL_TEXT[entry.level],
    source: resolveSource(entry.source),
    segments: splitMessage(stripAnsi(entry.message), keyword),
  }));
});

function isNearBottom(element: HTMLElement): boolean {
  return (
    element.scrollHeight - element.scrollTop - element.clientHeight <=
    BOTTOM_THRESHOLD_PX
  );
}

function updatePinnedState(): void {
  const element = scrollEl.value;
  if (!element || element.clientHeight === 0) {
    return;
  }
  pinnedToBottom.value = isNearBottom(element);
}

function scrollToBottom(): void {
  const element = scrollEl.value;
  if (!element || element.clientHeight === 0) {
    return;
  }
  element.scrollTop = element.scrollHeight;
  pinnedToBottom.value = true;
}

function handleScroll(): void {
  updatePinnedState();
}

function handleJumpToLatest(): void {
  scrollToBottom();
}

function handleResize(): void {
  if (props.autoScroll && pinnedToBottom.value) {
    scrollToBottom();
  } else {
    updatePinnedState();
  }
}

let resizeObserver: ResizeObserver | null = null;

watch(
  () => props.filterKey,
  () => {
    if (props.autoScroll) {
      pinnedToBottom.value = true;
      scrollToBottom();
    } else {
      updatePinnedState();
    }
  },
  { flush: "post" },
);

watch(
  () => props.entries,
  () => {
    if (props.autoScroll && pinnedToBottom.value) {
      scrollToBottom();
    } else {
      updatePinnedState();
    }
  },
  { flush: "post" },
);

watch(
  () => props.autoScroll,
  (enabled) => {
    if (enabled) {
      pinnedToBottom.value = true;
      scrollToBottom();
    } else {
      updatePinnedState();
    }
  },
);

watch(
  () => props.fontSize,
  () => {
    if (props.autoScroll && pinnedToBottom.value) {
      scrollToBottom();
    }
  },
  { flush: "post" },
);

onMounted(() => {
  const element = scrollEl.value;
  if (typeof ResizeObserver !== "undefined" && element) {
    resizeObserver = new ResizeObserver(handleResize);
    resizeObserver.observe(element);
  }

  if (props.autoScroll) {
    scrollToBottom();
  } else {
    updatePinnedState();
  }
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  resizeObserver = null;
});
</script>

<template>
  <div class="log-console">
    <div
      ref="scrollEl"
      class="log-console__scroll"
      role="log"
      aria-live="off"
      :aria-label="t('logs.title')"
      tabindex="0"
      :style="{ fontSize: `${fontSize}px` }"
      @scroll.passive="handleScroll"
    >
      <div v-if="rows.length === 0" class="log-console__empty">
        {{ t("logs.empty") }}
      </div>
      <div v-else class="log-console__rows">
        <div
          v-for="row in rows"
          :key="row.id"
          class="log-line"
          :class="`log-line--${row.level}`"
        >
          <span class="log-line__time">{{ row.time }}</span>
          <span class="log-line__level">{{ row.levelText }}</span>
          <span class="log-line__source" :title="row.source">{{ row.source }}</span>
          <span class="log-line__message">
            <template v-for="(segment, index) in row.segments" :key="index">
              <mark v-if="segment.highlight" class="log-line__mark">{{ segment.text }}</mark>
              <template v-else>{{ segment.text }}</template>
            </template>
          </span>
        </div>
      </div>
    </div>

    <v-btn
      v-if="!pinnedToBottom && rows.length > 0"
      class="log-console__jump"
      color="primary"
      variant="flat"
      size="small"
      rounded="pill"
      prepend-icon="mdi-arrow-down"
      :aria-label="t('logs.jumpToLatestAria')"
      @click="handleJumpToLatest"
    >
      {{ t("logs.jumpToLatest") }}
    </v-btn>
  </div>
</template>

<style scoped>
.log-console {
  position: relative;
  display: flex;
  flex: 1 1 auto;
  min-height: 240px;
  min-width: 0;
}

.log-console__scroll {
  flex: 1 1 auto;
  min-width: 0;
  min-height: 0;
  padding: 12px 14px;
  overflow-x: hidden;
  overflow-y: auto;
  overscroll-behavior: contain;
  border-radius: 12px;
  background: #17191c;
  color: #d7dade;
  font-family: Consolas, "Cascadia Mono", "Courier New", monospace;
  font-size: 12px;
  line-height: 1.6;
  scrollbar-color: rgba(255, 255, 255, 0.18) transparent;
  scrollbar-width: thin;
}

.log-console__scroll:focus-visible {
  outline: 2px solid rgb(var(--v-theme-primary));
  outline-offset: -2px;
}

.log-console__scroll::-webkit-scrollbar {
  width: 10px;
}

.log-console__scroll::-webkit-scrollbar-track {
  background: transparent;
}

.log-console__scroll::-webkit-scrollbar-thumb {
  border: 2px solid #17191c;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.18);
}

.log-console__rows {
  min-width: 0;
}

.log-console__empty {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100%;
  padding: 32px 12px;
  color: rgba(255, 255, 255, 0.38);
  font-size: 0.85rem;
}

.log-line {
  display: grid;
  grid-template-columns:
    max-content max-content minmax(0, 150px) minmax(0, 1fr);
  column-gap: 12px;
  align-items: start;
  padding: 1px 6px;
  border-radius: 4px;
}

.log-line:hover {
  background: rgba(255, 255, 255, 0.04);
}

.log-line__time {
  color: #7c848d;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.log-line__level {
  font-weight: 700;
  letter-spacing: 0.02em;
  white-space: nowrap;
}

.log-line--debug .log-line__level {
  color: #8b929a;
}

.log-line--info .log-line__level {
  color: #67b7dc;
}

.log-line--warn .log-line__level {
  color: #d4ad45;
}

.log-line--error .log-line__level {
  color: #df6262;
}

.log-line__source {
  min-width: 0;
  overflow: hidden;
  color: #9aa4ae;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.log-line__message {
  min-width: 0;
  white-space: pre-wrap;
  word-break: break-word;
  overflow-wrap: anywhere;
}

.log-line__mark {
  padding: 0 1px;
  border-radius: 2px;
  background: rgba(255, 213, 79, 0.32);
  color: #ffe082;
}

.log-console__jump {
  position: absolute;
  right: 20px;
  bottom: 16px;
  z-index: 1;
  box-shadow: 0 6px 18px rgba(0, 0, 0, 0.3);
}

@media (max-width: 720px) {
  .log-console__scroll {
    padding: 10px;
  }

  .log-line {
    grid-template-columns: max-content max-content minmax(0, 96px) minmax(0, 1fr);
    column-gap: 10px;
  }
}
</style>
