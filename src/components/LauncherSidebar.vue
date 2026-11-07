<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import SidebarNavItem from "./SidebarNavItem.vue";

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
</script>

<template>
  <v-navigation-drawer
    v-model="drawerOpen"
    class="launcher-sidebar"
    color="surface"
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
          icon="mdi-view-dashboard-outline"
          :label="t('shell.nav.overview')"
          to="/"
          :tooltip-disabled="!props.rail"
          :active="props.homeActive ? undefined : false"
          @select="emit('navigate')"
        />
        <SidebarNavItem
          icon="mdi-server-outline"
          :label="t('shell.nav.instances')"
          to="/instances"
          :tooltip-disabled="!props.rail"
          :active="props.homeActive ? undefined : false"
          @select="emit('navigate')"
        />
        <SidebarNavItem
          icon="mdi-tag-multiple-outline"
          :label="t('shell.nav.versions')"
          to="/versions"
          :tooltip-disabled="!props.rail"
          :active="props.homeActive ? undefined : false"
          @select="emit('navigate')"
        />
        <SidebarNavItem
          icon="mdi-archive-outline"
          :label="t('shell.nav.backups')"
          to="/backups"
          :tooltip-disabled="!props.rail"
          :active="props.homeActive ? undefined : false"
          @select="emit('navigate')"
        />
        <SidebarNavItem
          icon="mdi-clipboard-check-outline"
          :label="t('shell.nav.tasks')"
          to="/tasks"
          :tooltip-disabled="!props.rail"
          :active="props.homeActive ? undefined : false"
          @select="emit('navigate')"
        />
        <SidebarNavItem
          icon="mdi-text-box-outline"
          :label="t('shell.nav.logs')"
          to="/logs"
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
            icon="mdi-cog-outline"
            :label="t('shell.nav.settings')"
            to="/settings"
            :tooltip-disabled="!props.rail"
            :active="props.homeActive ? undefined : false"
            @select="emit('navigate')"
          />
        </v-list>

        <v-btn
          class="launcher-sidebar__toggle"
          :icon="props.rail ? 'mdi-chevron-right' : 'mdi-chevron-left'"
          :aria-label="toggleLabel"
          variant="text"
          @click="emit('toggle')"
        />
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
  background: rgb(var(--v-theme-surface)) !important;
  box-shadow: none !important;
  transition:
    width 180ms cubic-bezier(0.2, 0, 0, 1),
    transform 180ms cubic-bezier(0.2, 0, 0, 1);
}

.launcher-sidebar__content {
  display: flex;
  flex-direction: column;
  min-height: 100%;
  padding: 12px 10px;
}

.launcher-sidebar__nav {
  flex: 0 0 auto;
  padding: 0;
}

.launcher-sidebar__nav :deep(.v-list-item) {
  min-height: 36px;
  margin-bottom: 2px;
  padding-inline: 10px;
  border-radius: 8px;
  color: rgba(var(--v-theme-on-surface), 0.7);
  font-size: 0.9rem;
  font-weight: 500;
  transition:
    background-color 160ms ease,
    color 160ms ease;
}

.launcher-sidebar__nav :deep(.v-list-item:hover) {
  color: rgb(var(--v-theme-primary));
  background: rgba(var(--v-theme-primary), 0.07);
}

.launcher-sidebar__nav :deep(.v-list-item--active) {
  color: rgb(var(--v-theme-primary));
  background: rgba(var(--v-theme-primary), 0.12);
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

.launcher-sidebar__nav :deep(.v-icon) {
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
  padding-top: 10px;
}

.launcher-sidebar__footer-nav {
  flex: 1 1 auto;
  min-width: 0;
  padding: 0;
}

.launcher-sidebar__toggle {
  flex: 0 0 auto;
  min-height: 44px;
  border-radius: 12px;
  color: rgba(var(--v-theme-on-surface), 0.58);
}

.launcher-sidebar__toggle:hover {
  color: rgb(var(--v-theme-primary));
  background: rgba(var(--v-theme-primary), 0.07);
}

.launcher-sidebar.v-navigation-drawer--rail
  .launcher-sidebar__content {
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

.launcher-sidebar.v-navigation-drawer--rail
  .launcher-sidebar__footer {
  flex-direction: column;
  justify-content: center;
  gap: 4px;
}
</style>
