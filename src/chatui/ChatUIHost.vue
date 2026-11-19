// Ported from AstrBot dashboard (MIT, CodedThemes); see src/chatui/LICENSE.
// Island glue: mounts the ported Chat (chatbox-mode) full-size, keeps the
// island's theme/locale in sync with the host app, and renders the toolbar
// context (model selector + session title) that the dashboard's header would
// normally display.

<script setup lang="ts">
import { computed, watch } from "vue";
import { useDisplay, useTheme } from "vuetify";
import { i18n, setLocale as setAppLocale } from "../i18n";
import { isSupportedLocale, normalizeLocaleTag } from "../i18n/locale";
import { cacheLanguage } from "../utils/languageCache";
import { setChatInstance } from "./api/adapter";
import Chat from "./components/chat/Chat.vue";
import { CHATUI_FEATURES } from "./features";
import { initChatI18n, registerLocaleSwitch, useModuleI18n } from "./i18n/composables";
import type { IslandLocale } from "./i18n/composables";
import {
  customizerState,
  registerThemeChange,
  setIsDark,
} from "./stores/customizer";
import { useMobileDrawerStore } from "./stores/mobileDrawer";
import { headerContext } from "./stores/headerContext";

defineOptions({ name: "ChatUIHost", inheritAttrs: false });

const props = defineProps<{ instanceId: string | null }>();

const theme = useTheme();
const { smAndDown: isMobile } = useDisplay();
const mobileDrawer = useMobileDrawerStore();
const { tm } = useModuleI18n("features/chat");

const sidebarWidth = computed(() =>
  isMobile.value ? 0 : customizerState.chatSidebarCollapsed ? 56 : 245,
);

// Every instance change resets the ported conversation state the same way a
// fresh dashboard page load would: the Chat subtree is remounted.
watch(
  () => props.instanceId,
  (instanceId) => {
    setChatInstance(instanceId ?? "");
  },
  { immediate: true },
);

// The island's `isDark` follows the host Vuetify theme.
watch(
  () => theme.global.current.value.dark,
  (dark) => setIsDark(dark),
  { immediate: true },
);

// Theme changes from the island's own settings dialog stay window-local.
registerThemeChange((mode) => {
  const resolved =
    mode === "system"
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light"
      : mode;
  void theme.change(resolved);
});

// Locale: push the host locale in, and forward island switches back out.
function toIslandLocale(locale: string): IslandLocale {
  const normalized = normalizeLocaleTag(locale);
  return (normalized ?? "zh-CN") as IslandLocale;
}

watch(
  () => i18n.global.locale.value,
  (locale) => {
    void initChatI18n(toIslandLocale(locale));
  },
  { immediate: true },
);

registerLocaleSwitch((locale) => {
  if (isSupportedLocale(locale)) {
    setAppLocale(locale);
    cacheLanguage(locale);
  }
});

</script>

<template>
  <div class="chatui-host" :class="{ 'is-dark': theme.global.current.value.dark }">
    <header
      class="chatui-toolbar"
      data-tauri-drag-region
      :style="{ '--astrbot-chat-sidebar-width': `${sidebarWidth}px` }"
    >
      <button
        v-if="isMobile"
        type="button"
        class="chatui-menu-btn"
        :aria-label="tm('conversation.title')"
        @click="mobileDrawer.TOGGLE()"
      >
        <v-icon icon="mdi-menu" size="20" />
      </button>
      <component
        :is="headerContext.component"
        v-if="CHATUI_FEATURES.toolbar && headerContext.component"
      />
    </header>
    <Chat :key="instanceId ?? 'none'" :chatbox-mode="true" />
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

.chatui-toolbar {
  position: absolute;
  top: 0;
  right: 0;
  left: 0;
  z-index: 1006;
  height: var(--astrbot-toolbar-height);
  display: flex;
  align-items: center;
  box-sizing: border-box;
  background: var(--astrbot-chrome-bg);
  /* The fixed chat sidebar overlays this band on the left; content starts
     right of it, and the drag region still spans the whole strip. */
  padding-inline: 16px;
  padding-inline-start: calc(var(--astrbot-chat-sidebar-width, 0px) + 16px);
}

/* Their shell drops the toolbar band's top offset; the island toolbar sits
   exactly there, so re-reserve it with a specificity their rule cannot win. */
.chatui-host .chat-main.chat-main {
  margin-top: var(--astrbot-toolbar-height) !important;
}

/* Narrow windows hide the temporary sidebar; this is its only open affordance. */
.chatui-menu-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  margin-inline-end: 8px;
  border-radius: 8px;
  color: rgb(var(--v-theme-on-surface));
  cursor: pointer;
}

.chatui-menu-btn:hover {
  background: rgba(var(--v-theme-on-surface), 0.08);
}

.chatui-toolbar .chat-toolbar-context {
  flex: 1 1 auto;
}
</style>
