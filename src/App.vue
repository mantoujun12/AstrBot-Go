<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { useDisplay, useTheme } from "vuetify";
import AppSnackbar from "./components/common/AppSnackbar.vue";
import DefaultCredentialsDialog from "./components/common/DefaultCredentialsDialog.vue";
import AppTitleBar from "./components/AppTitleBar.vue";
import LauncherMain from "./components/LauncherMain.vue";
import LauncherSidebar from "./components/LauncherSidebar.vue";
import { useTabShortcuts } from "./composables/useTabShortcuts";
import { listenDefaultCredentials } from "./services/events";
import { useLauncherStore } from "./stores/launcher";
import { useLogStore } from "./stores/logs";
import { useProgressStore } from "./stores/progress";
import { HOME_TAB, useTabsStore } from "./stores/tabs";
import type { DefaultCredentials } from "./types/launcher";
import { cacheTheme } from "./utils/themeCache";

const { mdAndUp } = useDisplay();
const theme = useTheme();

const { config, instances, init: initLauncher } = useLauncherStore();
const { activeTab, isHomeActive, activateHome, pruneTabs } = useTabsStore();
const { init: initLogs } = useLogStore();
const { init: initProgress } = useProgressStore();

useTabShortcuts();

const isCollapsed = ref(false);
const mobileDrawerOpen = ref(false);
const credentials = ref<DefaultCredentials | null>(null);
const credentialsOpen = ref(false);
let unlistenCredentials: UnlistenFn | null = null;

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

function handleNavigate() {
  activateHome();
  closeMobileDrawer();
}

watch(
  () => instances.value.map((instance) => instance.id),
  (ids) => {
    pruneTabs(ids);
  },
  { immediate: true },
);

watch(activeTab, (tab) => {
  if (tab !== HOME_TAB && !mdAndUp.value) {
    mobileDrawerOpen.value = false;
  }
});

watch(
  () => config.value?.theme_preference,
  (preference) => {
    if (!preference) {
      return;
    }
    void theme.change(preference);
    cacheTheme(preference);
  },
  { immediate: true },
);

onMounted(() => {
  void initLogs();
  void initProgress();
  void initLauncher();
  void listenDefaultCredentials((payload) => {
    credentials.value = payload;
    credentialsOpen.value = true;
  })
    .then((unlisten) => {
      unlistenCredentials = unlisten;
    })
    .catch(() => {
      // Missing the credential notification must not block the app;
      // the instance logs still contain the same information.
    });
});

onBeforeUnmount(() => {
  unlistenCredentials?.();
  unlistenCredentials = null;
});
</script>

<template>
  <v-app class="launcher-app">
    <div class="launcher-shell">
      <AppTitleBar />
      <!-- Instance tabs use the full window; the sidebar is only part of the
           home shell so it must not reserve layout space in those tabs. -->
      <LauncherSidebar
        v-if="isHomeActive"
        v-model="drawerOpen"
        :permanent="mdAndUp"
        :rail="sidebarIsCollapsed"
        :home-active="isHomeActive"
        @navigate="handleNavigate"
        @toggle="toggleSidebar"
      />
      <LauncherMain :mobile="!mdAndUp" @open-menu="mobileDrawerOpen = true" />
      <AppSnackbar />
      <DefaultCredentialsDialog
        v-model="credentialsOpen"
        :credentials="credentials"
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
