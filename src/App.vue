<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { useRouter } from "vue-router";
import { useDisplay, useLocale as useVuetifyLocale, useTheme } from "vuetify";
import AppSnackbar from "./components/common/AppSnackbar.vue";
import DefaultCredentialsDialog from "./components/common/DefaultCredentialsDialog.vue";
import LauncherUpdatePrompt from "./components/common/LauncherUpdatePrompt.vue";
import LegacyMigrationDialog from "./components/common/LegacyMigrationDialog.vue";
import AppTitleBar from "./components/AppTitleBar.vue";
import LauncherMain from "./components/LauncherMain.vue";
import LauncherSidebar from "./components/LauncherSidebar.vue";
import { setReduceMotionPreference } from "./composables/useReduceMotion";
import { useTabShortcuts } from "./composables/useTabShortcuts";
import { useFadingScrollbars } from "./composables/useFadingScrollbars";
import { applyLanguagePreference, i18n } from "./i18n";
import { isSupportedLocale } from "./i18n/locale";
import {
  listenDefaultCredentials,
  listenInstanceRestart,
  listenResourceAlert,
  listenTrayPanelAction,
} from "./services/events";
import { syncWindowTheme } from "./services/nativeWindow";
import { detectLegacyDataDir } from "./services/app";
import type { TrayPanelAction } from "./services/tray";
import { useLauncherStore } from "./stores/launcher";
import { showToast } from "./stores/toast";
import { useLogStore } from "./stores/logs";
import { useProgressStore } from "./stores/progress";
import { useTaskStore } from "./stores/tasks";
import { HOME_TAB, useTabsStore } from "./stores/tabs";
import type {
  DefaultCredentials,
  LegacyDataDirPrompt,
  ResourceAlertEvent,
} from "./types/launcher";
import { formatFileSize } from "./utils/format";
import { cacheTheme } from "./utils/themeCache";

const router = useRouter();
const { mdAndUp } = useDisplay();
const theme = useTheme();
const { current: vuetifyLocale } = useVuetifyLocale();
const { t } = useI18n();

const { config, instances, init: initLauncher } = useLauncherStore();
const { activeTab, isHomeActive, activateHome, openInstanceTab, pruneTabs } =
  useTabsStore();
const { init: initLogs } = useLogStore();
const { init: initProgress } = useProgressStore();
const { init: initTasks } = useTaskStore();

useTabShortcuts();
useFadingScrollbars();

const isCollapsed = ref(false);
const mobileDrawerOpen = ref(false);
const credentials = ref<DefaultCredentials | null>(null);
const credentialsOpen = ref(false);
const legacyPrompt = ref<LegacyDataDirPrompt | null>(null);
const legacyDialogOpen = ref(false);
let unlistenCredentials: UnlistenFn | null = null;
let unlistenResourceAlert: UnlistenFn | null = null;
let unlistenRestart: UnlistenFn | null = null;
let unlistenTrayAction: UnlistenFn | null = null;

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

// 仅侧边栏展开/收起的瞬间挂上 .layout-animating，让内容区（v-main 内边距）
// 跟随 180ms 过渡与侧栏宽度动画同步；标签页切换等其余布局变化保持无动画。
const railAnimating = ref(false);
let railAnimationTimer: ReturnType<typeof setTimeout> | undefined;

function toggleSidebar() {
  isCollapsed.value = !isCollapsed.value;
  if (railAnimationTimer !== undefined) {
    clearTimeout(railAnimationTimer);
  }
  railAnimating.value = true;
  railAnimationTimer = setTimeout(() => {
    railAnimating.value = false;
    railAnimationTimer = undefined;
  }, 220);
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

function handleTrayPanelAction(action: TrayPanelAction): void {
  if (action.kind === "settings") {
    activateHome();
    closeMobileDrawer();
    void router.push("/settings");
    return;
  }
  openInstanceTab(action.instanceId);
}

function resourceAlertMessage(alert: ResourceAlertEvent): string {
  const instance = instances.value.find(
    (entry) => entry.id === alert.instance_id,
  );
  const name = instance?.name ?? alert.instance_id;
  const value =
    alert.metric === "cpu"
      ? `${alert.value.toFixed(1)}%`
      : formatFileSize(alert.value);
  const metric = i18n.global.t(`resources.metrics.${alert.metric}`);
  return i18n.global.t("resources.alerts.breached", {
    name,
    metric,
    value,
  });
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

// 直接监听解析后的生效主题：偏好为 system 时 OS 切换深浅也要同步原生窗口染色。
watch(
  () => theme.global.current.value.dark,
  (dark) => {
    void syncWindowTheme(dark);
  },
  { immediate: true },
);

watch(
  () => i18n.global.locale.value,
  (locale) => {
    if (isSupportedLocale(locale)) {
      vuetifyLocale.value = locale;
    }
  },
  { immediate: true },
);

watch(
  () => config.value?.language_preference,
  (preference) => {
    if (!preference) {
      return;
    }
    applyLanguagePreference(preference);
  },
  { immediate: true },
);

watch(
  () => config.value?.reduce_motion_preference,
  (preference) => {
    if (!preference) {
      return;
    }
    setReduceMotionPreference(preference);
  },
  { immediate: true },
);

onMounted(() => {
  void initLogs();
  void initProgress();
  void initTasks();
  void initLauncher();
  void detectLegacyDataDir()
    .then((prompt) => {
      if (prompt) {
        legacyPrompt.value = prompt;
        legacyDialogOpen.value = true;
      }
    })
    .catch(() => {
      // Detection must not block startup; the wizard can be shown on the
      // next launch.
    });
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
  void listenResourceAlert((alert) => {
    showToast(resourceAlertMessage(alert), "warning");
  })
    .then((unlisten) => {
      unlistenResourceAlert = unlisten;
    })
    .catch(() => {
      // Resource charts remain available when event subscription fails.
    });
  void listenTrayPanelAction(handleTrayPanelAction)
    .then((unlisten) => {
      unlistenTrayAction = unlisten;
    })
    .catch(() => {
      // The tray panel still opens the main window without deep navigation.
    });
  void listenInstanceRestart((payload) => {
    if (payload.will_retry) {
      showToast(
        t("instances.restart.scheduled", {
          name: payload.instance_name,
          seconds: payload.delay_secs,
          attempt: payload.attempt,
          max: payload.max_retries,
        }),
        "info",
      );
    } else {
      showToast(
        t("instances.restart.exhausted", { name: payload.instance_name }),
        "error",
      );
    }
  })
    .then((unlisten) => {
      unlistenRestart = unlisten;
    })
    .catch(() => {
      // Restart notifications are best-effort diagnostics only.
    });
});

onBeforeUnmount(() => {
  unlistenCredentials?.();
  unlistenCredentials = null;
  unlistenResourceAlert?.();
  unlistenResourceAlert = null;
  unlistenRestart?.();
  unlistenRestart = null;
  unlistenTrayAction?.();
  unlistenTrayAction = null;
  if (railAnimationTimer !== undefined) {
    clearTimeout(railAnimationTimer);
  }
});
</script>

<template>
  <v-app class="launcher-app">
    <!-- The title bar teleports itself to `body`: `.launcher-shell`'s
         `isolation` and Vuetify `.v-application__wrap`'s stacking context
         would otherwise trap it below the dialog scrim (z-index 2400). -->
    <AppTitleBar />
    <div
      class="launcher-shell"
      :class="{ 'layout-animating': railAnimating }"
    >
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
      <LegacyMigrationDialog
        v-model="legacyDialogOpen"
        :prompt="legacyPrompt"
      />
      <LauncherUpdatePrompt />
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
  --launcher-titlebar-z-index: 3000;
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
  background: rgba(var(--launcher-chrome-bg), var(--launcher-chrome-alpha, 1));
}

/* 窗口底色 token：默认跟随主题 surface，平台效果层可按平台覆盖 alpha。 */
:global(:root) {
  --launcher-chrome-bg: var(--v-theme-surface);
}
</style>
