<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  closeWindow,
  isWindowMaximized,
  minimizeWindow,
  onWindowResize,
  showSnapOverlay,
  toggleMaximizeWindow,
} from "../services/windowControls";
import TitleBarTabs from "./TitleBarTabs.vue";

const { t } = useI18n();

// macOS keeps its native traffic lights, so the custom controls are hidden and
// the title bar reserves room for them.
const isMac = navigator.userAgent.includes("Mac");

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
  <!-- Teleported to `body`: both `.launcher-shell` (isolation) and Vuetify's
       `.v-application__wrap` (backface-visibility) create stacking contexts
       that would trap the titlebar below the overlay scrim (z 2400+). As a
       body-level sibling it genuinely sits above the overlay stack, so the
       window controls stay usable while dialogs are open. -->
  <Teleport to="body">
    <header
      class="app-titlebar"
      :class="{ 'app-titlebar--mac': isMac }"
      data-tauri-decorum-tb
      :aria-label="t('shell.titleBar.label')"
    >
    <TitleBarTabs />
    <div
      class="app-titlebar__drag"
      data-tauri-drag-region
      aria-hidden="true"
    ></div>
    <div v-if="!isMac" class="app-titlebar__controls">
      <button
        id="decorum-tb-minimize"
        class="decorum-tb-btn app-titlebar__control"
        type="button"
        :aria-label="t('shell.titleBar.minimize')"
        @click="minimize"
      >
        <span class="app-titlebar__glyph" aria-hidden="true">&#xE921;</span>
      </button>
      <button
        id="decorum-tb-maximize"
        class="decorum-tb-btn app-titlebar__control"
        type="button"
        :aria-label="
          isMaximized
            ? t('shell.titleBar.restore')
            : t('shell.titleBar.maximize')
        "
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
        :aria-label="t('shell.titleBar.close')"
        @click="close"
      >
        <span class="app-titlebar__glyph" aria-hidden="true">&#xE8BB;</span>
      </button>
    </div>
    </header>
  </Teleport>
</template>

<style scoped>
/* Stays above the Vuetify overlay stack (dialogs start at z-index 2400 and
   increment by 10 per nested overlay) so the window controls are never
   swallowed by a scrim. Requires AppTitleBar to be mounted outside the
   isolated `.launcher-shell` stacking context. */
.app-titlebar {
  position: fixed;
  inset: 0 0 auto;
  z-index: var(--launcher-titlebar-z-index, 3000);
  display: flex;
  align-items: stretch;
  width: 100%;
  height: var(--launcher-titlebar-height, 40px);
  color: rgba(var(--v-theme-on-surface), 0.78);
  /* The `.launcher-shell` base already paints the shared chrome tint; an
     extra layer here would double-stack the alpha and hide the Mica tint. */
  background: transparent;
  border-bottom: 0;
  user-select: none;
}

.app-titlebar--mac {
  padding-inline-start: 80px;
}

.app-titlebar__drag {
  flex: 1 1 48px;
  min-width: 48px;
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

/* 带遮罩的弹层激活时，标题栏用不透明 chrome 底替代透明 Mica 底：遮罩（z 2400+，
   body 层）画在标题栏之下但透过透明底会盖住条带，不透明底让标题栏在弹窗期间
   保持正常亮度（原生模态样式：内容压暗，标题栏 chrome 不变）。 */
:global(body:has(.v-overlay--active .v-overlay__scrim)) .app-titlebar {
  background: rgb(var(--launcher-chrome-bg));
}
</style>
