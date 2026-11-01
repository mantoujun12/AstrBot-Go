<script setup lang="ts">
import { computed, ref } from "vue";
import { useDisplay } from "vuetify";
import AppTitleBar from "./components/AppTitleBar.vue";
import LauncherMain from "./components/LauncherMain.vue";
import LauncherSidebar from "./components/LauncherSidebar.vue";

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
    <div class="launcher-shell">
      <AppTitleBar />
      <LauncherSidebar
        v-model="drawerOpen"
        :permanent="mdAndUp"
        :rail="sidebarIsCollapsed"
        @navigate="closeMobileDrawer"
        @toggle="toggleSidebar"
      />
      <LauncherMain
        :mobile="!mdAndUp"
        @open-menu="mobileDrawerOpen = true"
      />
    </div>
  </v-app>
</template>

<style scoped>
:global(html),
:global(body),
:global(#app) {
  width: 100%;
  height: 100%;
  margin: 0;
  overflow: hidden;
  background: transparent !important;
}

.launcher-app {
  --launcher-titlebar-height: 40px;
  width: 100%;
  height: 100vh;
  height: 100dvh;
  min-height: 0;
  background: transparent !important;
}

.launcher-app :deep(.v-application__wrap) {
  height: 100vh;
  height: 100dvh;
  min-height: 0;
  overflow: hidden;
  box-sizing: border-box;
  background: transparent;
}

.launcher-shell {
  position: relative;
  isolation: isolate;
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  box-sizing: border-box;
  background: rgb(var(--v-theme-surface));
}
</style>
