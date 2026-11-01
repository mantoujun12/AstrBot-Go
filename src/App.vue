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
  </v-app>
</template>

<style scoped>
.launcher-app {
  --launcher-titlebar-height: 40px;
  background: rgb(var(--v-theme-background));
}

.launcher-app :deep(.v-application__wrap) {
  height: 100vh;
  height: 100dvh;
  min-height: 0;
  overflow: hidden;
}
</style>
