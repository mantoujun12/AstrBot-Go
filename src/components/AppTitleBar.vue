<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import {
  closeWindow,
  isWindowMaximized,
  minimizeWindow,
  onWindowResize,
  showSnapOverlay,
  toggleMaximizeWindow,
} from "../services/windowControls";

const isMaximized = ref(false);
let snapOverlayTimer: ReturnType<typeof setTimeout> | undefined;
let unlistenResize: (() => void) | undefined;
let isMounted = true;

async function updateMaximizedState() {
  try {
    isMaximized.value = await isWindowMaximized();
  } catch (error: unknown) {
    console.error("读取窗口状态失败", error);
  }
}

async function runWindowAction(action: () => Promise<void>) {
  try {
    await action();
  } catch (error: unknown) {
    console.error("窗口操作失败", error);
  }
}

function minimize() {
  cancelSnapOverlay();
  void runWindowAction(minimizeWindow);
}

function toggleMaximize() {
  cancelSnapOverlay();
  void runWindowAction(async () => {
    await toggleMaximizeWindow();
    await updateMaximizedState();
  });
}

function close() {
  cancelSnapOverlay();
  void runWindowAction(closeWindow);
}

function cancelSnapOverlay() {
  if (snapOverlayTimer !== undefined) {
    clearTimeout(snapOverlayTimer);
    snapOverlayTimer = undefined;
  }
}

function scheduleSnapOverlay() {
  cancelSnapOverlay();
  snapOverlayTimer = setTimeout(() => {
    snapOverlayTimer = undefined;
    void runWindowAction(showSnapOverlay);
  }, 620);
}

onMounted(async () => {
  await updateMaximizedState();

  try {
    const unlisten = await onWindowResize(() => {
      void updateMaximizedState();
    });

    if (isMounted) {
      unlistenResize = unlisten;
    } else {
      unlisten();
    }
  } catch (error: unknown) {
    console.error("监听窗口状态失败", error);
  }
});

onBeforeUnmount(() => {
  isMounted = false;
  cancelSnapOverlay();
  unlistenResize?.();
});
</script>

<template>
  <header
    class="app-titlebar"
    data-tauri-decorum-tb
    aria-label="应用标题栏"
  >
    <div
      class="app-titlebar__drag"
      data-tauri-drag-region
      aria-hidden="true"
    ></div>
    <div class="app-titlebar__controls">
      <button
        id="decorum-tb-minimize"
        class="decorum-tb-btn app-titlebar__control"
        type="button"
        aria-label="最小化"
        @click="minimize"
      >
        <span class="app-titlebar__glyph" aria-hidden="true">&#xE921;</span>
      </button>
      <button
        id="decorum-tb-maximize"
        class="decorum-tb-btn app-titlebar__control"
        type="button"
        :aria-label="isMaximized ? '还原' : '最大化'"
        @click="toggleMaximize"
        @mouseenter="scheduleSnapOverlay"
        @mouseleave="cancelSnapOverlay"
      >
        <span class="app-titlebar__glyph" aria-hidden="true">
          {{ isMaximized ? "\uE923" : "\uE922" }}
        </span>
      </button>
      <button
        id="decorum-tb-close"
        class="decorum-tb-btn app-titlebar__control app-titlebar__control--close"
        type="button"
        aria-label="关闭"
        @click="close"
      >
        <span class="app-titlebar__glyph" aria-hidden="true">&#xE8BB;</span>
      </button>
    </div>
  </header>
</template>

<style scoped>
.app-titlebar {
  position: fixed;
  inset: 0 0 auto;
  z-index: 3000;
  display: flex;
  align-items: stretch;
  width: 100%;
  height: var(--launcher-titlebar-height);
  color: rgba(var(--v-theme-on-surface), 0.78);
  background: rgb(var(--v-theme-surface));
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.08);
  user-select: none;
}

.app-titlebar__drag {
  flex: 1 1 auto;
  min-width: 0;
}

.app-titlebar__controls {
  display: flex;
  flex: 0 0 auto;
  align-self: stretch;
}

.app-titlebar__control {
  display: flex;
  width: 46px;
  height: 100%;
  flex: 0 0 46px;
  align-items: center;
  justify-content: center;
  padding: 0;
  color: rgba(var(--v-theme-on-surface), 0.78);
  background: transparent;
  border: 0;
  border-radius: 0;
  outline: none;
  cursor: default;
  transition:
    background-color 100ms ease,
    color 100ms ease;
}

.app-titlebar__glyph {
  font-family: "Segoe Fluent Icons", "Segoe MDL2 Assets", sans-serif;
  font-size: 10px;
  font-weight: 400;
  line-height: 1;
  -webkit-font-smoothing: antialiased;
  text-rendering: optimizeLegibility;
}

.app-titlebar__control:hover {
  color: rgb(var(--v-theme-on-surface));
  background: rgba(var(--v-theme-on-surface), 0.08);
}

.app-titlebar__control:active {
  background: rgba(var(--v-theme-on-surface), 0.12);
}

.app-titlebar__control--close:hover,
.app-titlebar__control--close:active {
  color: #fff;
  background: #c42b1c;
}

.app-titlebar__control:focus-visible {
  outline: 2px solid rgb(var(--v-theme-primary));
  outline-offset: -3px;
}
</style>
