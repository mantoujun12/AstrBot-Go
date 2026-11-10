<script setup lang="ts">
import { computed, nextTick, ref, type ComponentPublicInstance } from "vue";
import { useI18n } from "vue-i18n";
import logoUrl from "../assets/favicon.svg";
import { useLauncherStore } from "../stores/launcher";
import { HOME_TAB, useTabsStore } from "../stores/tabs";

const {
  instanceIds,
  activeTab,
  isHomeActive,
  activateInstanceTab,
  closeInstanceTab,
  activateHome,
  reorderInstanceTabs,
} = useTabsStore();
const { instances } = useLauncherStore();
const { t } = useI18n();

// macOS 顶部红绿灯区占用标题栏左侧空间，无法与侧栏轨道精确对齐，仅对 Windows/Linux 采用 8px 内边距。
const isMac = navigator.userAgent.includes("Mac");

const tabs = computed(() =>
  instanceIds.value.map((id) => {
    const instance = instances.value.find((item) => item.id === id) ?? null;
    return {
      id,
      name: instance?.name ?? id,
      state: instance?.state ?? "stopped",
    };
  }),
);

const tabButtons = new Map<string, HTMLButtonElement>();

function setTabButton(
  id: string,
  element: Element | ComponentPublicInstance | null,
): void {
  if (element instanceof HTMLButtonElement) {
    tabButtons.set(id, element);
  } else {
    tabButtons.delete(id);
  }
}

const orderedTabIds = computed(() => [
  HOME_TAB,
  ...tabs.value.map((tab) => tab.id),
]);

function activateTab(id: string): void {
  if (id === HOME_TAB) {
    activateHome();
    return;
  }
  activateInstanceTab(id);
}

function focusAndActivate(id: string): void {
  activateTab(id);
  void nextTick(() => {
    tabButtons.get(id)?.focus();
  });
}

function onTabKeydown(event: KeyboardEvent, id: string): void {
  const ids = orderedTabIds.value;
  const index = ids.indexOf(id);
  if (index === -1) {
    return;
  }

  if (event.key === "ArrowRight") {
    event.preventDefault();
    focusAndActivate(ids[(index + 1) % ids.length]);
    return;
  }
  if (event.key === "ArrowLeft") {
    event.preventDefault();
    focusAndActivate(ids[(index - 1 + ids.length) % ids.length]);
    return;
  }
  if (event.key === "Home") {
    event.preventDefault();
    focusAndActivate(ids[0]);
    return;
  }
  if (event.key === "End") {
    event.preventDefault();
    focusAndActivate(ids[ids.length - 1]);
    return;
  }
  if ((event.key === "Delete" || event.key === "Backspace") && id !== HOME_TAB) {
    event.preventDefault();
    closeTab(id);
  }
}

function closeTab(id: string): void {
  const wasActive = activeTab.value === id;
  closeInstanceTab(id);
  if (wasActive) {
    void nextTick(() => {
      tabButtons.get(activeTab.value)?.focus();
    });
  }
}

function onAuxClick(event: MouseEvent, id: string): void {
  if (event.button === 1) {
    event.preventDefault();
    closeTab(id);
  }
}

const draggedTabId = ref<string | null>(null);

function onDragStart(event: DragEvent, id: string): void {
  draggedTabId.value = id;
  if (event.dataTransfer) {
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", id);
  }
}

function onDragOver(event: DragEvent, id: string): void {
  const dragging = draggedTabId.value;
  if (!dragging || dragging === id || !event.dataTransfer) {
    return;
  }

  const current = instanceIds.value;
  const from = current.indexOf(dragging);
  const to = current.indexOf(id);
  if (from === -1 || to === -1) {
    return;
  }

  const target = event.currentTarget;
  if (!(target instanceof HTMLElement)) {
    return;
  }
  const rect = target.getBoundingClientRect();
  const before = event.clientX < rect.left + rect.width / 2;

  const nextOrder = current.slice();
  nextOrder.splice(from, 1);
  const insertIndex = nextOrder.indexOf(id) + (before ? 0 : 1);
  nextOrder.splice(insertIndex, 0, dragging);
  reorderInstanceTabs(nextOrder);

  event.dataTransfer.dropEffect = "move";
  event.preventDefault();
}

function onDragEnd(): void {
  draggedTabId.value = null;
}
</script>

<template>
  <div
    class="titlebar-tabs"
    :class="{ 'titlebar-tabs--mac': isMac }"
    role="tablist"
    :aria-label="t('shell.tabs.label')"
  >
    <button
      type="button"
      class="titlebar-tabs__home"
      role="tab"
      :aria-label="t('shell.tabs.home')"
      aria-controls="tabpanel-home"
      :aria-selected="isHomeActive"
      :tabindex="isHomeActive ? 0 : -1"
      :ref="(element) => setTabButton(HOME_TAB, element)"
      @click="activateHome"
      @keydown="onTabKeydown($event, HOME_TAB)"
    >
      <img :src="logoUrl" alt="" class="titlebar-tabs__logo" />
    </button>

    <TransitionGroup
      tag="div"
      name="titlebar-tab"
      class="titlebar-tabs__scroll"
      role="presentation"
    >
      <div
        v-for="tab in tabs"
        :key="tab.id"
        class="titlebar-tabs__tab"
        :class="{
          'titlebar-tabs__tab--active': activeTab === tab.id,
          'titlebar-tabs__tab--dragging': draggedTabId === tab.id,
        }"
        :draggable="true"
        @dragstart="onDragStart($event, tab.id)"
        @dragover="onDragOver($event, tab.id)"
        @dragend="onDragEnd"
        @drop.prevent
      >
        <button
          type="button"
          class="titlebar-tabs__main"
          role="tab"
          :aria-selected="activeTab === tab.id"
          :aria-controls="`tabpanel-${tab.id}`"
          :tabindex="activeTab === tab.id ? 0 : -1"
          :ref="(element) => setTabButton(tab.id, element)"
          @click="activateInstanceTab(tab.id)"
          @keydown="onTabKeydown($event, tab.id)"
          @auxclick="onAuxClick($event, tab.id)"
        >
          <span
            class="titlebar-tabs__dot"
            :class="`titlebar-tabs__dot--${tab.state}`"
            aria-hidden="true"
          />
          <span class="titlebar-tabs__label">{{ tab.name }}</span>
        </button>
        <button
          type="button"
          class="titlebar-tabs__close"
          :aria-label="t('shell.tabs.close', { name: tab.name })"
          @click.stop="closeTab(tab.id)"
        >
          <v-icon icon="mdi-close" size="14" />
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.titlebar-tabs {
  display: flex;
  flex: 0 1 auto;
  align-items: center;
  min-width: 0;
  height: 100%;
  padding-inline-start: 8px;
}

/* macOS 红绿灯预留区使轨道对齐无法实现，保持原 4px 内边距。 */
.titlebar-tabs--mac {
  padding-inline-start: 4px;
}

.titlebar-tabs__home {
  display: flex;
  flex: 0 0 40px;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 30px;
  margin-inline-end: 2px;
  padding: 0;
  border: 0;
  border-radius: 8px;
  background: transparent;
  cursor: default;
  transition:
    background-color 120ms ease,
    color 120ms ease;
}

.titlebar-tabs__home:hover,
.titlebar-tabs__home[aria-selected="true"] {
  background: rgba(var(--v-theme-primary), 0.12);
}

.titlebar-tabs__home:focus-visible,
.titlebar-tabs__main:focus-visible,
.titlebar-tabs__close:focus-visible {
  outline: 2px solid rgb(var(--v-theme-primary));
  outline-offset: -2px;
}

.titlebar-tabs__logo {
  width: 20px;
  height: 20px;
  display: block;
}

.titlebar-tabs__scroll {
  display: flex;
  flex: 0 1 auto;
  align-items: center;
  min-width: 0;
  overflow-x: auto;
  overflow-y: hidden;
  scrollbar-width: none;
}

.titlebar-tabs__scroll::-webkit-scrollbar {
  display: none;
}

.titlebar-tabs__tab {
  display: flex;
  flex: 1 1 180px;
  align-items: center;
  min-width: 96px;
  max-width: 200px;
  height: 30px;
  margin-inline-end: 2px;
  overflow: hidden;
  border-radius: 8px;
  color: rgba(var(--v-theme-on-surface), 0.68);
  transition:
    flex-grow var(--motion-duration-base) var(--motion-ease),
    flex-basis var(--motion-duration-base) var(--motion-ease),
    min-width var(--motion-duration-base) var(--motion-ease),
    opacity var(--motion-duration-base) var(--motion-ease),
    background-color 120ms ease,
    color 120ms ease;
}

.titlebar-tab-enter-from,
.titlebar-tab-leave-to {
  flex: 0 1 0;
  min-width: 0;
  opacity: 0;
}

.titlebar-tabs__tab:hover {
  background: rgba(var(--v-theme-on-surface), 0.06);
}

.titlebar-tabs__tab--dragging {
  opacity: 0.4;
}

.titlebar-tabs__tab--active {
  color: rgb(var(--v-theme-primary));
  background: rgba(var(--v-theme-primary), 0.12);
}

.titlebar-tabs__main {
  display: flex;
  flex: 1 1 auto;
  align-items: center;
  gap: 7px;
  min-width: 0;
  height: 100%;
  padding: 0 2px 0 10px;
  border: 0;
  border-radius: 8px 0 0 8px;
  background: transparent;
  color: inherit;
  font: inherit;
  font-size: 0.78rem;
  font-weight: 500;
  text-align: left;
  cursor: default;
}

.titlebar-tabs__label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.titlebar-tabs__dot {
  flex: 0 0 auto;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: rgba(var(--v-theme-on-surface), 0.4);
}

.titlebar-tabs__dot--running {
  background: rgb(var(--v-theme-success));
}

.titlebar-tabs__dot--starting {
  background: rgb(var(--v-theme-info));
}

.titlebar-tabs__dot--stopping {
  background: rgb(var(--v-theme-warning));
}

.titlebar-tabs__close {
  display: flex;
  flex: 0 0 22px;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  margin-inline-end: 4px;
  padding: 0;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: inherit;
  opacity: 0.6;
  cursor: default;
}

.titlebar-tabs__close:hover {
  background: rgba(var(--v-theme-on-surface), 0.1);
  opacity: 1;
}
</style>
