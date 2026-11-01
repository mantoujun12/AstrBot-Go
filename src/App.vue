<script setup lang="ts">
import { computed, ref } from "vue";
import { useDisplay } from "vuetify";

const { mdAndUp } = useDisplay();

const isCollapsed = ref(false);
const mobileDrawerOpen = ref(false);

const drawerOpen = computed({
  get: () => mdAndUp.value || mobileDrawerOpen.value,
  set: (value: boolean) => {
    if (!mdAndUp.value) {
      mobileDrawerOpen.value = value;
    }
  },
});

const sidebarIsCollapsed = computed(
  () => mdAndUp.value && isCollapsed.value,
);

const toggleLabel = computed(() =>
  sidebarIsCollapsed.value ? "展开侧边栏" : "收起侧边栏",
);

function toggleSidebar() {
  isCollapsed.value = !isCollapsed.value;
}

function closeMobileDrawer() {
  if (!mdAndUp.value) {
    mobileDrawerOpen.value = false;
  }
}
</script>

<template>
  <v-app class="launcher-app">
    <v-navigation-drawer
      v-model="drawerOpen"
      class="launcher-sidebar"
      color="surface"
      :elevation="0"
      :permanent="mdAndUp"
      :rail="sidebarIsCollapsed"
      :rail-width="56"
      :temporary="!mdAndUp"
      :width="236"
    >
      <div class="launcher-sidebar__content">
        <v-list nav density="comfortable" class="launcher-sidebar__nav">
          <v-tooltip
            :disabled="!sidebarIsCollapsed"
            color="surface"
            location="right"
            :open-delay="0"
            text="概览"
          >
            <template #activator="{ props: tooltipProps }">
              <v-list-item
                v-bind="tooltipProps"
                exact
                aria-label="概览"
                rounded="lg"
                to="/"
                @click="closeMobileDrawer"
              >
                <template #prepend>
                  <v-icon icon="mdi-view-dashboard-outline" size="18" />
                </template>
                <v-list-item-title>概览</v-list-item-title>
              </v-list-item>
            </template>
          </v-tooltip>
        </v-list>

        <div class="launcher-sidebar__footer">
          <v-list
            nav
            density="comfortable"
            class="launcher-sidebar__nav launcher-sidebar__footer-nav"
          >
            <v-tooltip
              :disabled="!sidebarIsCollapsed"
              color="surface"
              location="right"
              :open-delay="0"
              text="设置"
            >
              <template #activator="{ props: tooltipProps }">
                <v-list-item
                  v-bind="tooltipProps"
                  exact
                  aria-label="设置"
                  rounded="lg"
                  to="/settings"
                  @click="closeMobileDrawer"
                >
                  <template #prepend>
                    <v-icon icon="mdi-cog-outline" size="18" />
                  </template>
                  <v-list-item-title>设置</v-list-item-title>
                </v-list-item>
              </template>
            </v-tooltip>
          </v-list>

          <v-btn
            class="launcher-sidebar__toggle"
            :icon="
              sidebarIsCollapsed ? 'mdi-chevron-right' : 'mdi-chevron-left'
            "
            :aria-label="toggleLabel"
            variant="text"
            @click="toggleSidebar"
          />
        </div>
      </div>
    </v-navigation-drawer>

    <v-main class="launcher-main">
      <div class="launcher-main__canvas">
        <v-btn
          v-if="!mdAndUp"
          class="launcher-mobile-menu"
          icon
          aria-label="打开菜单"
          color="surface"
          elevation="2"
          @click="mobileDrawerOpen = true"
        >
          <v-icon icon="mdi-menu" />
        </v-btn>

        <router-view />
      </div>
    </v-main>
  </v-app>
</template>

<style scoped>
.launcher-app {
  background: rgb(var(--v-theme-background));
}

.launcher-sidebar {
  border-inline-end: 1px solid
    rgba(var(--v-theme-on-surface), 0.08);
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
  border-top: 1px solid rgba(var(--v-theme-on-surface), 0.08);
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

.launcher-main {
  min-height: 100vh;
  background:
    radial-gradient(
      circle at 92% 8%,
      rgba(var(--v-theme-primary), 0.055),
      transparent 28rem
    ),
    rgb(var(--v-theme-background));
}

.launcher-main__canvas {
  position: relative;
  min-height: 100vh;
  padding: 24px;
}

.launcher-mobile-menu {
  position: absolute;
  z-index: 1;
  top: 16px;
  left: 16px;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.08);
}

@media (max-width: 959px) {
  .launcher-main__canvas {
    min-height: 100dvh;
    padding: 72px 16px 16px;
  }
}

@media (prefers-reduced-motion: reduce) {
  .launcher-sidebar {
    transition: none;
  }
}
</style>
