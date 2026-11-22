// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// Island glue: mounts the ported Chat (chatbox-mode) full-size, keeps the
// island's theme/locale in sync with the host app, and renders the toolbar
// context (model selector + session title) that the dashboard's header would
// normally display.

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, provide, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useDisplay, useTheme } from "vuetify";import type { UnlistenFn } from "@tauri-apps/api/event";
import { applyLanguagePreference, i18n } from "../main/i18n";
import { normalizeLocaleTag } from "../main/i18n/locale";
import { toUserMessage } from "../main/services/errors";
import {
  saveLanguagePreference,
  saveThemePreference,
} from "../main/services/config";
import { listenPreferencesChanged } from "../main/services/events";
import type { LanguagePreference, ThemePreference } from "../main/types/launcher";
import { readCachedLanguage } from "../main/utils/languageCache";
import {
  closeWindow,
  isWindowMaximized,
  minimizeWindow,
  onWindowResize,
  setWindowTitle,
  showSnapOverlay,
  toggleMaximizeWindow,
} from "../main/services/windowControls";
import { setChatInstance } from "./api/adapter";
import Chat from "./components/chat/Chat.vue";
import ConfirmDialog from "./components/ConfirmDialog.vue";
import { initChatI18n, registerLocaleSwitch, useModuleI18n } from "./i18n/composables";
import type { IslandLocale } from "./i18n/composables";
import { chatInstanceSelectionKey } from "./instanceSelection";
import type { ChatReadiness } from "./instanceSelection";
import type { ConfirmDialogHandler, ConfirmDialogOptions } from "./utils/confirmDialog";
import {
  registerThemeChange,
  setIsDark,
  setThemePreference,
  customizerState,
} from "./stores/customizer";
import { useChatHeaderStore } from "./stores/chatHeader";
import { useMobileDrawerStore } from "./stores/mobileDrawer";
import { useLauncherStore } from "../main/stores/launcher";

defineOptions({ name: "ChatUIHost", inheritAttrs: false });

const props = defineProps<{ instanceId: string | null }>();

const emit = defineEmits<{
  selectInstance: [instanceId: string];
}>();

const theme = useTheme();
const { t } = useI18n();
const { smAndDown: isMobile } = useDisplay();
const mobileDrawer = useMobileDrawerStore();
const { tm } = useModuleI18n("features/chat");
const { config, instances } = useLauncherStore();
const chatHeader = useChatHeaderStore();

// Session title in the title-bar band: the island's Chat view pushes the
// current session name into the chatHeader store on load, switch and rename.
const sidebarWidth = computed(() =>
  isMobile.value ? 0 : customizerState.chatSidebarCollapsed ? 56 : 245,
);

watch(
  () => chatHeader.title,
  (title) => {
    // Empty/new-chat state falls back to the app name in the taskbar.
    void runWindowAction(() => setWindowTitle(title.trim() || "AstrBot Go"));
  },
  { immediate: true },
);

// Window controls mirror the launcher's AppTitleBar; macOS keeps its native
// traffic lights, so the custom cluster is hidden there.
const isMac = navigator.userAgent.includes("Mac");

const isMaximized = ref(false);
let snapOverlayTimer: ReturnType<typeof setTimeout> | undefined;
let unlistenResize: (() => void) | undefined;
let isMounted = true;

async function updateMaximizedState() {
  try {
    isMaximized.value = await isWindowMaximized();
  } catch (error: unknown) {
    console.error("读取窗口状态失败", toUserMessage(error));
  }
}

async function runWindowAction(action: () => Promise<void>) {
  try {
    await action();
  } catch (error: unknown) {
    console.error("窗口操作失败", toUserMessage(error));
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
  if (isMac) {
    return;
  }
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
    console.error("监听窗口状态失败", toUserMessage(error));
  }
});

onBeforeUnmount(() => {
  isMounted = false;
  cancelSnapOverlay();
  unlistenResize?.();
  unlistenPreferences?.();
  unlistenPreferences = null;
  if (onSystemThemeChange) {
    systemTheme.removeEventListener("change", onSystemThemeChange);
    onSystemThemeChange = null;
  }
  window.removeEventListener("languagechange", handleLanguageChange);
});

// Every instance change resets the ported conversation state the same way a
// fresh dashboard page load would: the Chat subtree is remounted.
watch(
  () => props.instanceId,
  (instanceId) => {
    setChatInstance(instanceId ?? "");
  },
  { immediate: true },
);

// The island switches chat targets through this contract; ChatView owns the
// actual target and remounts the island per instance.
const readiness = computed<ChatReadiness>(() => {
  const instance = instances.value.find(
    (entry) => entry.id === props.instanceId,
  );
  return {
    running: instance?.state === "running",
    credentialsConfigured: instance?.dashboard_credentials_configured === true,
  };
});

provide(chatInstanceSelectionKey, {
  current: computed(() => props.instanceId),
  select: (instanceId) => {
    if (instanceId !== "") emit("selectInstance", instanceId);
  },
  readiness,
});

// Destructive chat actions await `inject("$confirm")` (utils/confirmDialog.ts).
// window.confirm is silently dropped in WebView2, so the host provides a
// Vuetify dialog that resolves true on confirm and false on cancel/close.
const confirmDialog = ref<{
  open: (options: ConfirmDialogOptions) => Promise<boolean>;
} | null>(null);
const confirm: ConfirmDialogHandler = (options) =>
  confirmDialog.value?.open(options) ?? Promise.resolve(false);
provide("$confirm", confirm);

// The island's `isDark` follows the host Vuetify theme.
watch(
  () => theme.global.current.value.dark,
  (dark) => setIsDark(dark),
  { immediate: true },
);

// --- Shared appearance preferences ---------------------------------------
// Theme and language are stored once in the backend and broadcast to every
// window, so the chat window and the launcher stay in sync in both directions.

const systemTheme = window.matchMedia("(prefers-color-scheme: dark)");
const languagePreference = ref<LanguagePreference>(readCachedLanguage());
let unlistenPreferences: UnlistenFn | null = null;
let onSystemThemeChange: (() => void) | null = null;

function applyThemePreference(preference: ThemePreference) {
  setThemePreference(preference);
  if (onSystemThemeChange) {
    systemTheme.removeEventListener("change", onSystemThemeChange);
    onSystemThemeChange = null;
  }
  if (preference === "system") {
    onSystemThemeChange = () => setIsDark(systemTheme.matches);
    systemTheme.addEventListener("change", onSystemThemeChange);
  }
  setIsDark(
    preference === "system" ? systemTheme.matches : preference === "dark",
  );
  void theme.change(preference);
}

function applyHostLanguage(preference: LanguagePreference) {
  languagePreference.value = preference;
  void initChatI18n(applyLanguagePreference(preference), preference);
}

// The host config is the source of truth; pushing it covers the first load and
// any launcher-side change that refreshed the snapshot.
watch(
  () => config.value,
  (value) => {
    if (!value) {
      return;
    }
    applyThemePreference(value.theme_preference);
    applyHostLanguage(value.language_preference);
  },
  { immediate: true },
);

onMounted(async () => {
  try {
    const unlisten = await listenPreferencesChanged((preferences) => {
      applyThemePreference(preferences.theme_preference);
      applyHostLanguage(preferences.language_preference);
    });
    if (isMounted) {
      unlistenPreferences = unlisten;
    } else {
      unlisten();
    }
  } catch (error: unknown) {
    console.error("监听外观设置失败", toUserMessage(error));
  }
});

function handleLanguageChange() {
  if (languagePreference.value === "system") {
    applyHostLanguage("system");
  }
}
window.addEventListener("languagechange", handleLanguageChange);

// Locale: push the host locale in, and forward island switches back out.
function toIslandLocale(locale: string): IslandLocale {
  const normalized = normalizeLocaleTag(locale);
  return (normalized ?? "zh-CN") as IslandLocale;
}

watch(
  () => i18n.global.locale.value,
  (locale) => {
    void initChatI18n(toIslandLocale(locale), languagePreference.value);
  },
  { immediate: true },
);

// Island changes are applied locally, then persisted; the backend broadcasts
// the saved value to the launcher.
registerThemeChange((mode) => {
  applyThemePreference(mode);
  void saveThemePreference(mode).catch((error: unknown) => {
    console.error("保存主题设置失败", toUserMessage(error));
  });
});

registerLocaleSwitch((preference) => {
  applyHostLanguage(preference);
  void saveLanguagePreference(preference).catch((error: unknown) => {
    console.error("保存语言设置失败", toUserMessage(error));
  });
});

</script>

<template>
  <div class="chatui-host" :class="{ 'is-dark': theme.global.current.value.dark }">
    <!-- Native title-bar drag surface. No decorum script is registered for this
         window, so this is the only drag region in the page. -->
    <div class="chatui-titlebar-drag" data-tauri-drag-region aria-hidden="true" />
    <div
      v-if="chatHeader.title"
      class="chatui-titlebar-title"
      :class="{ 'chatui-titlebar-title--mac': isMac }"
      :style="{ left: `max(${sidebarWidth}px, 48px)` }"
    >
      <span class="chatui-titlebar-title-text" :title="chatHeader.title">{{
        chatHeader.title
      }}</span>
    </div>
    <button
      v-if="isMobile"
      type="button"
      class="chatui-menu-btn"
      :aria-label="tm('conversation.title')"
      @click="mobileDrawer.TOGGLE()"
    >
      <v-icon icon="mdi-menu" size="20" />
    </button>
    <Chat :key="instanceId ?? 'none'" :chatbox-mode="true" />
    <ConfirmDialog ref="confirmDialog" />
    <!-- Teleported to `body`: `.chat-shell` (isolation) and Vuetify's
         `.v-application__wrap` (backface-visibility) create stacking contexts
         that would trap the cluster below the settings dialog's scrim
         (z-index 2400+). As a body-level sibling it stays clickable while the
         dialog is open. No drag attribute: the buttons must stay buttons. -->
    <Teleport to="body">
      <div
        v-if="!isMac"
        class="chatui-window-controls"
      >
        <button
          id="decorum-tb-minimize"
          class="chatui-window-controls__btn"
          type="button"
          :aria-label="t('shell.titleBar.minimize')"
          @click="minimize"
        >
          <span class="chatui-window-controls__glyph" aria-hidden="true"
            >&#xE921;</span
          >
        </button>
        <button
          id="decorum-tb-maximize"
          class="chatui-window-controls__btn"
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
          <span class="chatui-window-controls__glyph" aria-hidden="true">{{
            isMaximized ? "\uE923" : "\uE922"
          }}</span>
        </button>
        <button
          id="decorum-tb-close"
          class="chatui-window-controls__btn chatui-window-controls__btn--close"
          type="button"
          :aria-label="t('shell.titleBar.close')"
          @click="close"
        >
          <span class="chatui-window-controls__glyph" aria-hidden="true"
            >&#xE8BB;</span
          >
        </button>
      </div>
    </Teleport>
  </div>
</template>

<style>
:root {
  --astrbot-toolbar-height: 50px;
}

/* The island's brand/welcome titles use the dashboard's `.Outfit` class. */
.chatui-host .Outfit {
  font-family:
    "Outfit Variable",
    "Outfit",
    "Noto Sans",
    var(--astrbot-font-cjk-sans),
    sans-serif !important;
}

/* Upstream's dashboard assumes bare <button>s render chromeless, but the
   Vuetify 4 reset (unlike their full scss stack) no longer strips native
   button backgrounds and borders — every plain island button showed OS
   chrome. :where() keeps this at zero specificity so the island's own
   component styles always win; Vuetify component roots carry v-* classes
   and stay untouched. */
:where(.chatui-host) button:not([class*="v-"]) {
  appearance: none;
  -webkit-appearance: none;
  background: transparent;
  border: none;
  color: inherit;
}

/* Shared chrome background for the toolbar and the sidebar so they read as
   one surface, mirroring the dashboard's FullLayout: light #f2f2f7, dark the
   theme background. The Chat island inherits it for its own sidebar. */
.chatui-host {
  position: relative;
  height: 100%;
  min-height: 0;
  --astrbot-chrome-bg: #f2f2f7;
  background: var(--astrbot-chrome-bg);
}

.chatui-host.is-dark {
  --astrbot-chrome-bg: rgb(var(--v-theme-background));
}

/* Their shell drops the toolbar band's top offset; the island toolbar sits
   exactly there, so re-reserve it with a specificity their rule cannot win. */
.chatui-host .chat-main.chat-main {
  margin-top: var(--astrbot-toolbar-height) !important;
}

/* Native drag band over the empty top strip. Sits below the sidebar drawer and
   the dialog scrim, so it never competes with their interactive regions. */
.chatui-titlebar-drag {
  position: absolute;
  inset: 0 0 auto;
  height: var(--astrbot-toolbar-height);
  z-index: 0;
  background: transparent;
}

/* Session title in the native title-bar band: centered over the chat pane,
   clear of the sidebar and the window controls, truncating when narrow.
   pointer-events: none keeps the drag band beneath it usable. */
.chatui-titlebar-title {
  position: absolute;
  top: 0;
  /* The window control cluster is 3 x 46px; macOS keeps native controls.
     `left` follows the sidebar toggle and shares the launcher's motion. */
  right: 138px;
  transition: left 180ms cubic-bezier(0.2, 0, 0, 1);
  height: var(--astrbot-toolbar-height);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  pointer-events: none;
}

.chatui-titlebar-title--mac {
  right: 12px;
}

.chatui-titlebar-title-text {
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: rgba(var(--v-theme-on-surface), 0.62);
  font-size: 13px;
  font-weight: 500;
}

/* Narrow windows hide the temporary sidebar; this floating button is its only
   open affordance. No full-width chrome band sits under it. */
.chatui-menu-btn {
  position: fixed;
  top: 8px;
  left: 8px;
  z-index: 1001;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: var(--astrbot-radius-control, 8px);
  background: var(--astrbot-chrome-bg);
  color: rgb(var(--v-theme-on-surface));
  cursor: pointer;
}

.chatui-menu-btn:hover {
  background: rgba(var(--v-theme-on-surface), 0.08);
}

/* Window controls, mirroring AppTitleBar's. The cluster is teleported to
   `body` and z-index 3000 beats the Vuetify overlay stack (dialogs start at
   2400), so the buttons stay clickable while the settings dialog's scrim is
   up. Transparent background: under an active scrim the glyphs blend into the
   dimmed surface instead of showing as an opaque strip. */
.chatui-window-controls {
  position: fixed;
  top: 0;
  right: 0;
  z-index: 3000;
  display: flex;
  height: var(--astrbot-toolbar-height);
  background: transparent;
}

.chatui-window-controls__btn {
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

.chatui-window-controls__glyph {
  font-family: "Segoe Fluent Icons", "Segoe MDL2 Assets", sans-serif;
  font-size: 10px;
  font-weight: 400;
  line-height: 1;
  -webkit-font-smoothing: antialiased;
  text-rendering: optimizeLegibility;
}

.chatui-window-controls__btn:hover {
  color: rgb(var(--v-theme-on-surface));
  background: rgba(var(--v-theme-on-surface), 0.08);
}

.chatui-window-controls__btn:active {
  background: rgba(var(--v-theme-on-surface), 0.12);
}

.chatui-window-controls__btn--close:hover,
.chatui-window-controls__btn--close:active {
  color: #fff;
  background: #c42b1c;
}

.chatui-window-controls__btn:focus-visible {
  outline: 2px solid rgb(var(--v-theme-primary));
  outline-offset: -3px;
}
</style>
