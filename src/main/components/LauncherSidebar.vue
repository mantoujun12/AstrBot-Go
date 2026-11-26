<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import {
  Archive,
  ChevronRight,
  ClipboardCheck,
  FileText,
  House,
  MessageCircle,
  PanelLeft,
  Puzzle,
  Server,
  Settings,
  Tags,
} from "@lucide/vue";
import SidebarNavItem from "./SidebarNavItem.vue";
import { openChatWindow } from "../services/chat";
import { showErrorToast } from "../stores/toast";

const props = defineProps<{
  permanent: boolean;
  rail: boolean;
  homeActive: boolean;
}>();

const drawerOpen = defineModel<boolean>({ required: true });
const { t } = useI18n();

const emit = defineEmits<{
  (event: "navigate"): void;
  (event: "toggle"): void;
}>();

const toggleLabel = computed(() =>
  props.rail ? t("shell.sidebar.expand") : t("shell.sidebar.collapse"),
);

async function handleOpenChat(): Promise<void> {
  try {
    await openChatWindow();
  } catch (cause) {
    showErrorToast(cause);
  }
}
</script>

<template>
  <v-navigation-drawer
    v-model="drawerOpen"
    class="launcher-sidebar"
    :elevation="0"
    :permanent="props.permanent"
    :rail="props.rail"
    :rail-width="56"
    :temporary="!props.permanent"
    :width="236"
  >
    <div class="launcher-sidebar__content">
      <v-list nav density="comfortable" class="launcher-sidebar__nav">
        <SidebarNavItem
          :icon="MessageCircle"
          :label="t('shell.nav.chat')"
          beta
          :append-icon="ChevronRight"
          :tooltip-disabled="!props.rail"
          :active="false"
          @select="handleOpenChat"
        />
        <SidebarNavItem
          :icon="House"
          :label="t('shell.nav.overview')"
          to="/"
          :tooltip-disabled="!props.rail"
          :active="props.homeActive ? undefined : false"
          @select="emit('navigate')"
        />
        <SidebarNavItem
          :icon="Server"
          :label="t('shell.nav.instances')"
          to="/instances"
          :tooltip-disabled="!props.rail"
          :active="props.homeActive ? undefined : false"
          @select="emit('navigate')"
        />
        <SidebarNavItem
          :icon="Tags"
          :label="t('shell.nav.versions')"
          to="/versions"
          :tooltip-disabled="!props.rail"
          :active="props.homeActive ? undefined : false"
          @select="emit('navigate')"
        />
        <SidebarNavItem
          :icon="Archive"
          :label="t('shell.nav.backups')"
          to="/backups"
          :tooltip-disabled="!props.rail"
          :active="props.homeActive ? undefined : false"
          @select="emit('navigate')"
        />
        <SidebarNavItem
          :icon="ClipboardCheck"
          :label="t('shell.nav.tasks')"
          to="/tasks"
          :tooltip-disabled="!props.rail"
          :active="props.homeActive ? undefined : false"
          @select="emit('navigate')"
        />
        <SidebarNavItem
          :icon="FileText"
          :label="t('shell.nav.logs')"
          to="/logs"
          :tooltip-disabled="!props.rail"
          :active="props.homeActive ? undefined : false"
          @select="emit('navigate')"
        />
        <SidebarNavItem
          :icon="Puzzle"
          :label="t('shell.nav.extensions')"
          beta
          to="/extensions"
          :tooltip-disabled="!props.rail"
          :active="props.homeActive ? undefined : false"
          @select="emit('navigate')"
        />
      </v-list>

      <div class="launcher-sidebar__footer">
        <v-list
          nav
          density="comfortable"
          class="launcher-sidebar__nav launcher-sidebar__footer-nav"
        >
          <SidebarNavItem
            :icon="Settings"
            :label="t('shell.nav.settings')"
            to="/settings"
            :tooltip-disabled="!props.rail"
            :active="props.homeActive ? undefined : false"
            @select="emit('navigate')"
          />
        </v-list>

        <v-btn
          v-if="props.permanent"
          class="launcher-sidebar__toggle"
          :aria-label="toggleLabel"
          variant="text"
          @click="emit('toggle')"
        >
          <PanelLeft :size="20" />
        </v-btn>
      </div>
    </div>
  </v-navigation-drawer>
</template>

<style scoped>
.launcher-sidebar {
  position: absolute !important;
  top: var(--launcher-titlebar-height) !important;
  left: 0 !important;
  height: calc(100% - var(--launcher-titlebar-height)) !important;
  border-inline-end: 0;
  /* Keep Vuetify's opaque surface background out: in the permanent and rail
     modes the drawer sits inside `.launcher-shell`, which already paints the
     shared chrome tint, and a second layer would double-stack the alpha and
     hide the Mica tint. The temporary drawer floats above the content outside
     that shell, so it must paint its own background — see below. */
  background: transparent !important;
  box-shadow: none !important;
  transition:
    width 180ms cubic-bezier(0.2, 0, 0, 1),
    transform 180ms cubic-bezier(0.2, 0, 0, 1);
}

/* Temporary (< md, i.e. narrower than 840px): overlays the page instead of
   sharing the shell tint, so a transparent background would show the page
   content through the nav. Reuse the same chrome tint, fully opaque. */
.launcher-sidebar.v-navigation-drawer--temporary {
  background: rgb(var(--launcher-chrome-bg)) !important;
}

.launcher-sidebar__content {
  display: flex;
  flex-direction: column;
  min-height: 100%;
}

/* Matches upstream VerticalSidebar: list padding 2px 16px 12px, nav item radius via the navigation token. */
.launcher-sidebar__nav {
  flex: 0 0 auto;
  padding: 2px 16px 12px;
  background: transparent;
}

.launcher-sidebar__nav :deep(.v-list-item) {
  min-height: 36px;
  margin-bottom: 2px;
  padding-inline: 10px;
  border-radius: var(--astrbot-radius-navigation);
  color: rgba(var(--v-theme-on-surface), 0.76);
  font-size: 0.9rem;
  font-weight: 500;
  transition:
    background-color 160ms ease,
    color 160ms ease;
}

.launcher-sidebar__nav :deep(.v-list-item:hover) {
  color: rgb(var(--v-theme-on-surface));
  background: rgba(var(--v-theme-on-surface), 0.05);
}

.launcher-sidebar__nav :deep(.v-list-item--active) {
  color: rgb(var(--v-theme-primary));
  background: rgba(var(--v-theme-primary), 0.08);
  font-weight: 600;
}

.launcher-sidebar__nav :deep(.v-list-item__overlay) {
  opacity: 0 !important;
}

.launcher-sidebar__nav :deep(.v-list-item__prepend) {
  min-width: 0;
  margin-inline-end: 10px;
}

.launcher-sidebar__nav :deep(.v-list-item__spacer) {
  width: 0;
}

.launcher-sidebar__nav :deep(.v-list-item__content) {
  min-width: 0;
}

.launcher-sidebar__nav :deep(.sidebar-lucide-icon) {
  flex: 0 0 auto;
  color: currentcolor;
  stroke-width: 2;
  opacity: 0.9;
}

.launcher-sidebar__nav :deep(.v-list-item-title) {
  font-size: 14px;
  font-weight: 500;
  line-height: 20px;
}

.launcher-sidebar__footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  margin-top: auto;
  padding: 10px 16px 12px;
}

.launcher-sidebar__footer-nav {
  flex: 1 1 auto;
  min-width: 0;
  padding: 0;
}

.launcher-sidebar__toggle {
  flex: 0 0 auto;
  min-height: 44px;
  border-radius: var(--astrbot-radius-control);
  color: rgba(var(--v-theme-on-surface), 0.58);
}

.launcher-sidebar__toggle:hover {
  color: rgb(var(--v-theme-primary));
  background: rgba(var(--v-theme-primary), 0.07);
}

/* In rail mode nav items shrink to 40px and the list padding narrows. */
.launcher-sidebar.v-navigation-drawer--rail
  .launcher-sidebar__nav {
  padding-inline: 8px;
}

.launcher-sidebar.v-navigation-drawer--rail
  .launcher-sidebar__nav :deep(.v-list-item) {
  grid-template-columns: 1fr;
  width: 40px;
  min-width: 40px;
  max-width: 40px;
  min-height: 40px;
  margin-inline: auto;
  padding-inline: 0;
}

.launcher-sidebar.v-navigation-drawer--rail
  .launcher-sidebar__nav :deep(.v-list-item__prepend) {
  width: 100%;
  margin-inline-end: 0;
  justify-content: center;
}

.launcher-sidebar.v-navigation-drawer--rail
  .launcher-sidebar__nav :deep(.v-list-item__content) {
  display: none;
}

/* Rail items are a single centered icon column; the append hint (the chat
   entry's chevron) would share that column and squeeze the icon. */
.launcher-sidebar.v-navigation-drawer--rail
  .launcher-sidebar__nav :deep(.v-list-item__append) {
  display: none;
}

.launcher-sidebar.v-navigation-drawer--rail
  .launcher-sidebar__footer {
  flex-direction: column;
  justify-content: center;
  gap: 4px;
  padding-inline: 8px;
}
</style>
