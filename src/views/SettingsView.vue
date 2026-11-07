<script setup lang="ts">
import { computed, nextTick, onMounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useTheme } from "vuetify";
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import ConfirmDialog from "../components/common/ConfirmDialog.vue";
import EmptyState from "../components/common/EmptyState.vue";
import PageHeader from "../components/common/PageHeader.vue";
import SettingsItem from "../components/settings/SettingsItem.vue";
import LauncherUpdateSection from "../components/settings/LauncherUpdateSection.vue";
import NetworkSourceList from "../components/settings/NetworkSourceList.vue";
import {
  NETWORK_SOURCES,
  type NetworkSourceKey,
} from "../constants/networkSources";
import {
  getAppVersion,
  openExternalUrl,
  pickDirectory,
  setDataDir,
} from "../services/app";
import { clearUvCache } from "../services/components";
import {
  saveAllowLocalDnsProbe,
  saveAutostartMinimizeToTray,
  saveCheckInstanceUpdate,
  saveCloseToTray,
  saveGithubProxy,
  saveIgnoreExternalPath,
  saveLanguagePreference,
  saveLockCheckExtensionWhitelist,
  saveNodejsMirror,
  saveNpmRegistry,
  savePersistInstanceState,
  saveProxy,
  savePypiMirror,
  savePythonMirror,
  saveReduceMotionPreference,
  saveThemePreference,
  saveUseShortInstanceId,
  saveUseUvForDeps,
  saveUvMirror,
} from "../services/config";
import { toUserMessage } from "../services/errors";
import {
  testHttpProxy,
  type NetworkProbeResult,
} from "../services/network";
import { rebuildInstanceManifest } from "../services/instances";
import { useLauncherStore } from "../stores/launcher";
import { showErrorToast, showToast } from "../stores/toast";
import {
  applyThemeColors,
  clearStoredThemeColors,
  DEFAULT_THEME_COLORS,
  isHexColor,
  readStoredThemeColors,
  restoreThemeColors,
  storeThemeColors,
} from "../theme/customColors";
import { useLanguage } from "../composables/useLanguage";
import { setReduceMotionPreference } from "../composables/useReduceMotion";
import { isLanguagePreference } from "../i18n/locale";
import type {
  AppConfig,
  LanguagePreference,
  ReduceMotionPreference,
  ThemePreference,
} from "../types/launcher";
import { readCachedLanguage } from "../utils/languageCache";
import { formatLatency, formatSpeed } from "../utils/network";
import { cacheTheme } from "../utils/themeCache";

const { config, dataDir, loading, error, refresh } = useLauncherStore();

type SettingsSection = "general" | "appearance" | "network" | "data" | "about";

interface SettingsNavItem {
  id: SettingsSection;
  labelKey: string;
  icon: string;
}

const sections: SettingsNavItem[] = [
  {
    id: "general",
    labelKey: "settings.nav.general",
    icon: "mdi-tune-variant",
  },
  {
    id: "appearance",
    labelKey: "settings.nav.appearance",
    icon: "mdi-palette-outline",
  },
  {
    id: "network",
    labelKey: "settings.nav.network",
    icon: "mdi-lan-connect",
  },
  {
    id: "data",
    labelKey: "settings.nav.data",
    icon: "mdi-tools",
  },
  {
    id: "about",
    labelKey: "settings.nav.about",
    icon: "mdi-information-outline",
  },
];

const activeSection = ref<SettingsSection>("general");
const bodyRef = ref<HTMLElement | null>(null);

function selectSection(section: SettingsSection): void {
  activeSection.value = section;
  void nextTick(() => {
    bodyRef.value?.scrollTo({ top: 0 });
  });
}

/* Theme */

const themePreference = ref<ThemePreference>("system");
const themeSaving = ref(false);

async function changeTheme(value: unknown): Promise<void> {
  if (themeSaving.value) {
    return;
  }
  if (value !== "system" && value !== "light" && value !== "dark") {
    return;
  }
  if (value === themePreference.value) {
    return;
  }

  const previous = themePreference.value;
  themePreference.value = value;
  themeSaving.value = true;
  try {
    await saveThemePreference(value);
    cacheTheme(value);
    await refresh();
  } catch (cause) {
    themePreference.value = previous;
    showErrorToast(cause);
  } finally {
    themeSaving.value = false;
  }
}

/* Reduce motion */

const reduceMotionPreference = ref<ReduceMotionPreference>("system");
const reduceMotionSaving = ref(false);

async function changeReduceMotion(value: unknown): Promise<void> {
  if (reduceMotionSaving.value) {
    return;
  }
  if (value !== "system" && value !== "on" && value !== "off") {
    return;
  }
  if (value === reduceMotionPreference.value) {
    return;
  }

  const previous = reduceMotionPreference.value;
  reduceMotionPreference.value = value;
  setReduceMotionPreference(value);
  reduceMotionSaving.value = true;
  try {
    await saveReduceMotionPreference(value);
    await refresh();
  } catch (cause) {
    reduceMotionPreference.value = previous;
    setReduceMotionPreference(previous);
    showErrorToast(cause);
  } finally {
    reduceMotionSaving.value = false;
  }
}

/* Language */

const { t } = useI18n();
const { apply: applyLanguage } = useLanguage();
const languagePreference = ref<LanguagePreference>(readCachedLanguage());
const languageSaving = ref(false);
const languageOptions = computed(() => [
  { value: "system", title: t("settings.appearance.followSystem") },
  { value: "zh-CN", title: "简体中文" },
  { value: "en-US", title: "English" },
  { value: "ru-RU", title: "Русский" },
  { value: "ja-JP", title: "日本語" },
]);

async function changeLanguage(value: unknown): Promise<void> {
  if (languageSaving.value || !isLanguagePreference(value)) {
    return;
  }
  if (value === languagePreference.value) {
    return;
  }

  const previous = languagePreference.value;
  languagePreference.value = value;
  languageSaving.value = true;
  try {
    await saveLanguagePreference(value);
    applyLanguage(value);
    await refresh();
  } catch (cause) {
    languagePreference.value = previous;
    applyLanguage(previous);
    showErrorToast(cause);
  } finally {
    languageSaving.value = false;
  }
}

/* Custom theme colors */

const theme = useTheme();
const storedCustomColors = readStoredThemeColors();
const customPrimary = ref(
  storedCustomColors?.primary ?? DEFAULT_THEME_COLORS.primary,
);
const customSecondary = ref(
  storedCustomColors?.secondary ?? DEFAULT_THEME_COLORS.secondary,
);
const customColorsActive = ref(storedCustomColors !== null);

function persistCustomColors(): void {
  const primary = customPrimary.value.trim();
  const secondary = customSecondary.value.trim();
  if (!isHexColor(primary) || !isHexColor(secondary)) {
    return;
  }

  applyThemeColors(theme, { primary, secondary });
  storeThemeColors({ primary, secondary });
  customColorsActive.value = true;
}

function setCustomPrimary(value: unknown): void {
  if (typeof value !== "string") {
    return;
  }
  customPrimary.value = value;
  persistCustomColors();
}

function setCustomSecondary(value: unknown): void {
  if (typeof value !== "string") {
    return;
  }
  customSecondary.value = value;
  persistCustomColors();
}

function resetCustomColors(): void {
  restoreThemeColors(theme);
  clearStoredThemeColors();
  customPrimary.value = DEFAULT_THEME_COLORS.primary;
  customSecondary.value = DEFAULT_THEME_COLORS.secondary;
  customColorsActive.value = false;
}

/* Autostart */

const autostartEnabled = ref(false);
const autostartLoading = ref(true);
const autostartSaving = ref(false);
const autostartUnavailable = ref(false);

async function loadAutostartState(): Promise<void> {
  autostartLoading.value = true;
  try {
    autostartEnabled.value = await isAutostartEnabled();
    autostartUnavailable.value = false;
  } catch (cause) {
    autostartUnavailable.value = true;
    showToast(
      t("settings.general.autostartReadFailed", {
        detail: toUserMessage(cause),
      }),
      "error",
    );
  } finally {
    autostartLoading.value = false;
  }
}

async function changeAutostart(value: unknown): Promise<void> {
  if (
    typeof value !== "boolean" ||
    autostartSaving.value ||
    autostartUnavailable.value
  ) {
    return;
  }

  const previous = autostartEnabled.value;
  autostartEnabled.value = value;
  autostartSaving.value = true;
  try {
    if (value) {
      await enableAutostart();
    } else {
      await disableAutostart();
    }
  } catch (cause) {
    autostartEnabled.value = previous;
    showErrorToast(cause);
  } finally {
    autostartSaving.value = false;
  }
}

/* Config toggles */

type ToggleKey =
  | "close_to_tray"
  | "autostart_minimize_to_tray"
  | "check_instance_update"
  | "persist_instance_state"
  | "ignore_external_path"
  | "use_short_instance_id"
  | "lock_check_extension_whitelist"
  | "use_uv_for_deps";

interface ToggleDefinition {
  key: ToggleKey;
  titleKey: string;
  subtitleKey?: string;
  save: (value: boolean) => Promise<void>;
}

const toggleDefinitions: ToggleDefinition[] = [
  {
    key: "close_to_tray",
    titleKey: "settings.general.toggles.closeToTray",
    subtitleKey: "settings.general.toggles.closeToTrayHint",
    save: saveCloseToTray,
  },
  {
    key: "autostart_minimize_to_tray",
    titleKey: "settings.general.toggles.autostartMinimizeToTray",
    subtitleKey: "settings.general.toggles.autostartMinimizeToTrayHint",
    save: saveAutostartMinimizeToTray,
  },
  {
    key: "check_instance_update",
    titleKey: "settings.general.toggles.checkInstanceUpdate",
    subtitleKey: "settings.general.toggles.checkInstanceUpdateHint",
    save: saveCheckInstanceUpdate,
  },
  {
    key: "persist_instance_state",
    titleKey: "settings.general.toggles.persistInstanceState",
    subtitleKey: "settings.general.toggles.persistInstanceStateHint",
    save: savePersistInstanceState,
  },
  {
    key: "ignore_external_path",
    titleKey: "settings.general.toggles.ignoreExternalPath",
    subtitleKey: "settings.general.toggles.ignoreExternalPathHint",
    save: saveIgnoreExternalPath,
  },
  {
    key: "use_short_instance_id",
    titleKey: "settings.general.toggles.useShortInstanceId",
    subtitleKey: "settings.general.toggles.useShortInstanceIdHint",
    save: saveUseShortInstanceId,
  },
  {
    key: "lock_check_extension_whitelist",
    titleKey: "settings.general.toggles.lockCheckExtensionWhitelist",
    subtitleKey: "settings.general.toggles.lockCheckExtensionWhitelistHint",
    save: saveLockCheckExtensionWhitelist,
  },
  {
    key: "use_uv_for_deps",
    titleKey: "settings.general.toggles.useUvForDeps",
    subtitleKey: "settings.general.toggles.useUvForDepsHint",
    save: saveUseUvForDeps,
  },
];

const toggleValues = reactive<Record<ToggleKey, boolean>>({
  close_to_tray: false,
  autostart_minimize_to_tray: false,
  check_instance_update: false,
  persist_instance_state: false,
  ignore_external_path: false,
  use_short_instance_id: false,
  lock_check_extension_whitelist: false,
  use_uv_for_deps: false,
});

const toggleSaving = reactive<Record<ToggleKey, boolean>>({
  close_to_tray: false,
  autostart_minimize_to_tray: false,
  check_instance_update: false,
  persist_instance_state: false,
  ignore_external_path: false,
  use_short_instance_id: false,
  lock_check_extension_whitelist: false,
  use_uv_for_deps: false,
});

async function changeToggle(key: ToggleKey, value: unknown): Promise<void> {
  if (typeof value !== "boolean" || toggleSaving[key]) {
    return;
  }

  const definition = toggleDefinitions.find((item) => item.key === key);
  if (!definition) {
    return;
  }

  const previous = config.value?.[key] ?? toggleValues[key];
  toggleValues[key] = value;
  toggleSaving[key] = true;
  try {
    await definition.save(value);
    await refresh();
  } catch (cause) {
    toggleValues[key] = previous;
    showErrorToast(cause);
  } finally {
    toggleSaving[key] = false;
  }
}

/* Network */

const proxyForm = reactive({
  url: "",
  port: "",
  username: "",
  password: "",
});
const proxyDirty = ref(false);
const proxySaving = ref(false);
const proxyTesting = ref(false);
const proxyTestResult = ref<NetworkProbeResult | null>(null);
const proxyTestError = ref("");
const allowLocalDnsProbe = ref(false);
const allowLocalDnsProbeSaving = ref(false);

const networkDrafts = reactive<Record<NetworkSourceKey, string>>({
  github: "",
  pypi: "",
  nodejs: "",
  npm: "",
  python: "",
  uv: "",
});
const networkDirty = reactive<Record<NetworkSourceKey, boolean>>({
  github: false,
  pypi: false,
  nodejs: false,
  npm: false,
  python: false,
  uv: false,
});
const networkSaving = reactive<Record<NetworkSourceKey, boolean>>({
  github: false,
  pypi: false,
  nodejs: false,
  npm: false,
  python: false,
  uv: false,
});

const networkFields: Record<NetworkSourceKey, keyof AppConfig> = {
  github: "github_proxy",
  pypi: "pypi_mirror",
  nodejs: "nodejs_mirror",
  npm: "npm_registry",
  python: "python_mirror",
  uv: "uv_mirror",
};

const networkSaveHandlers: Record<
  NetworkSourceKey,
  (value: string) => Promise<void>
> = {
  github: saveGithubProxy,
  pypi: savePypiMirror,
  nodejs: saveNodejsMirror,
  npm: saveNpmRegistry,
  python: savePythonMirror,
  uv: saveUvMirror,
};

const proxyChanged = computed(() => {
  const value = config.value;
  if (!value) {
    return false;
  }
  return (
    proxyForm.url !== value.proxy_url ||
    proxyForm.port !== value.proxy_port ||
    proxyForm.username !== value.proxy_username ||
    proxyForm.password !== value.proxy_password
  );
});

function markProxyDirty(): void {
  proxyDirty.value = true;
  proxyTestResult.value = null;
  proxyTestError.value = "";
}

function setNetworkDraft(key: NetworkSourceKey, value: string): void {
  networkDrafts[key] = value;
  const current = config.value;
  networkDirty[key] =
    current !== null && value !== current[networkFields[key]];
}

async function saveNetworkSource(key: NetworkSourceKey): Promise<void> {
  if (networkSaving[key] || !networkDirty[key]) {
    return;
  }

  networkSaving[key] = true;
  const snapshot = networkDrafts[key];
  try {
    await networkSaveHandlers[key](snapshot);
    await refresh();
    const current = config.value;
    const saved = current?.[networkFields[key]];
    // Only adopt the saved value if the draft was not edited while saving.
    if (networkDrafts[key] === snapshot && typeof saved === "string") {
      networkDrafts[key] = saved;
    }
    networkDirty[key] =
      current !== null && networkDrafts[key] !== current[networkFields[key]];
    showToast(t("settings.saved"));
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    networkSaving[key] = false;
  }
}

async function saveProxyForm(): Promise<void> {
  if (proxySaving.value || !proxyChanged.value) {
    return;
  }

  proxySaving.value = true;
  try {
    await saveProxy({
      proxyUrl: proxyForm.url,
      proxyPort: proxyForm.port,
      proxyUsername: proxyForm.username,
      proxyPassword: proxyForm.password,
    });
    proxyDirty.value = false;
    await refresh();
    showToast(t("settings.saved"));
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    proxySaving.value = false;
  }
}

async function runProxyTest(): Promise<void> {
  if (proxyTesting.value) {
    return;
  }

  proxyTesting.value = true;
  proxyTestError.value = "";
  try {
    proxyTestResult.value = await testHttpProxy({
      proxyUrl: proxyForm.url,
      proxyPort: proxyForm.port,
      proxyUsername: proxyForm.username,
      proxyPassword: proxyForm.password,
    });
  } catch (cause) {
    proxyTestResult.value = null;
    proxyTestError.value = toUserMessage(cause);
  } finally {
    proxyTesting.value = false;
  }
}

async function changeAllowLocalDnsProbe(value: unknown): Promise<void> {
  if (typeof value !== "boolean" || allowLocalDnsProbeSaving.value) {
    return;
  }

  const previous = allowLocalDnsProbe.value;
  allowLocalDnsProbe.value = value;
  allowLocalDnsProbeSaving.value = true;
  try {
    await saveAllowLocalDnsProbe(value);
    await refresh();
  } catch (cause) {
    allowLocalDnsProbe.value = previous;
    showErrorToast(cause);
  } finally {
    allowLocalDnsProbeSaving.value = false;
  }
}

function syncNetworkForms(value: AppConfig | null): void {
  if (!value) {
    return;
  }
  if (!allowLocalDnsProbeSaving.value) {
    allowLocalDnsProbe.value = value.allow_local_dns_probe;
  }
  if (!proxyDirty.value) {
    proxyForm.url = value.proxy_url;
    proxyForm.port = value.proxy_port;
    proxyForm.username = value.proxy_username;
    proxyForm.password = value.proxy_password;
  }
  for (const definition of NETWORK_SOURCES) {
    if (!networkDirty[definition.key]) {
      networkDrafts[definition.key] = value[definition.configField];
    }
  }
}

/* Data and maintenance */

const pickingDirectory = ref(false);
const pendingDir = ref("");
const changeDirDialogOpen = ref(false);
const changeDirSaving = ref(false);
const restartPendingDir = ref("");

const rebuildDialogOpen = ref(false);
const rebuilding = ref(false);
const clearingUvCache = ref(false);

async function chooseDataDirectory(): Promise<void> {
  if (pickingDirectory.value || changeDirSaving.value) {
    return;
  }

  pickingDirectory.value = true;
  try {
    const selected = await pickDirectory();
    if (!selected) {
      return;
    }
    pendingDir.value = selected;
    changeDirDialogOpen.value = true;
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    pickingDirectory.value = false;
  }
}

async function confirmDataDirectoryChange(): Promise<void> {
  if (!pendingDir.value || changeDirSaving.value) {
    return;
  }

  changeDirSaving.value = true;
  const target = pendingDir.value;
  try {
    const result = await setDataDir(target);
    restartPendingDir.value =
      result.new_dir.trim() === "" ? target : result.new_dir;
    changeDirDialogOpen.value = false;
    pendingDir.value = "";
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    changeDirSaving.value = false;
  }
}

async function confirmRebuildManifest(): Promise<void> {
  if (rebuilding.value) {
    return;
  }

  rebuilding.value = true;
  try {
    const result = await rebuildInstanceManifest();
    await refresh();
    rebuildDialogOpen.value = false;
    showToast(
      t("settings.data.rebuildComplete", {
        instances: result.instances,
        versions: result.versions,
      }),
    );
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    rebuilding.value = false;
  }
}

async function handleClearUvCache(): Promise<void> {
  if (clearingUvCache.value) {
    return;
  }

  clearingUvCache.value = true;
  try {
    await clearUvCache();
    showToast(t("settings.data.uvCacheCleared"));
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    clearingUvCache.value = false;
  }
}

/* About */

const appVersion = ref("—");
const launcherRepoUrl = "https://github.com/AstrBotDevs/astrbot-launcher";
const currentRepoUrl =
  "https://github.com/mantoujun12/astrbot-launcher-vue";
const astrbotRepoUrl = "https://github.com/AstrBotDevs/AstrBot";

async function loadAppVersion(): Promise<void> {
  try {
    const version = await getAppVersion();
    appVersion.value = version.trim() === "" ? "—" : version;
  } catch {
    appVersion.value = "—";
  }
}

async function openLink(url: string): Promise<void> {
  try {
    await openExternalUrl(url);
  } catch (cause) {
    showToast(
      t("settings.about.openLinkFailed", { detail: toUserMessage(cause) }),
      "error",
    );
  }
}

watch(
  config,
  (value) => {
    if (!value) {
      return;
    }

    for (const definition of toggleDefinitions) {
      if (!toggleSaving[definition.key]) {
        toggleValues[definition.key] = value[definition.key];
      }
    }

    if (!themeSaving.value) {
      themePreference.value = value.theme_preference;
    }
    if (!reduceMotionSaving.value) {
      reduceMotionPreference.value = value.reduce_motion_preference;
    }
    if (!languageSaving.value) {
      languagePreference.value = value.language_preference;
    }

    syncNetworkForms(value);
  },
  { immediate: true },
);

onMounted(() => {
  void loadAppVersion();
  void loadAutostartState();
});
</script>

<template>
  <section class="settings-page" :aria-label="t('settings.title')">
    <PageHeader :title="t('settings.title')" />

    <div v-if="!config" class="settings-state">
      <v-progress-circular
        v-if="loading"
        indeterminate
        color="primary"
        :aria-label="t('settings.loading')"
      />
      <EmptyState
        v-else
        icon="mdi-alert-circle-outline"
        :text="error ?? t('settings.loadFailed')"
      >
        <v-btn variant="tonal" color="primary" size="small" @click="refresh">
          {{ t("common.actions.retry") }}
        </v-btn>
      </EmptyState>
    </div>

    <div v-else class="settings-layout">
      <nav class="settings-nav" :aria-label="t('settings.nav.ariaLabel')">
        <button
          v-for="item in sections"
          :key="item.id"
          type="button"
          class="settings-nav__item"
          :class="{ 'settings-nav__item--active': activeSection === item.id }"
          :aria-current="activeSection === item.id ? 'true' : undefined"
          @click="selectSection(item.id)"
        >
          <v-icon :icon="item.icon" size="16" aria-hidden="true" />
          <span>{{ t(item.labelKey) }}</span>
        </button>
      </nav>

      <div ref="bodyRef" class="settings-body">
        <section
          v-show="activeSection === 'general'"
          id="settings-general"
          class="settings-section"
          aria-labelledby="settings-general-title"
        >
          <h2 id="settings-general-title" class="settings-section__title">
            {{ t("settings.nav.general") }}
          </h2>
          <div class="settings-list-card">
            <SettingsItem
              :title="t('settings.general.autostart')"
              :subtitle="
                autostartUnavailable
                  ? t('settings.general.autostartUnavailable')
                  : t('settings.general.autostartHint')
              "
              label-for="setting-autostart"
            >
              <v-switch
                id="setting-autostart"
                :model-value="autostartEnabled"
                :loading="autostartLoading || autostartSaving"
                :disabled="
                  autostartLoading || autostartSaving || autostartUnavailable
                "
                color="primary"
                density="compact"
                hide-details
                @update:model-value="changeAutostart"
              />
            </SettingsItem>

            <SettingsItem
              v-for="item in toggleDefinitions"
              :key="item.key"
              :title="t(item.titleKey)"
              :subtitle="item.subtitleKey ? t(item.subtitleKey) : ''"
              :label-for="`setting-${item.key}`"
            >
              <v-switch
                :id="`setting-${item.key}`"
                :model-value="toggleValues[item.key]"
                :loading="toggleSaving[item.key]"
                :disabled="toggleSaving[item.key]"
                color="primary"
                density="compact"
                hide-details
                @update:model-value="(value) => changeToggle(item.key, value)"
              />
            </SettingsItem>
          </div>
        </section>

        <section
          v-show="activeSection === 'appearance'"
          id="settings-appearance"
          class="settings-section"
          aria-labelledby="settings-appearance-title"
        >
          <h2 id="settings-appearance-title" class="settings-section__title">
            {{ t("settings.nav.appearance") }}
          </h2>
          <div class="settings-list-card">
            <SettingsItem :title="t('settings.appearance.theme')" wide>
              <v-radio-group
                :model-value="themePreference"
                :disabled="themeSaving"
                inline
                hide-details
                :aria-label="t('settings.appearance.theme')"
                class="settings-theme-group"
                @update:model-value="changeTheme"
              >
                <v-radio
                  :label="t('settings.appearance.followSystem')"
                  value="system"
                  color="primary"
                  density="compact"
                />
                <v-radio
                  :label="t('settings.appearance.light')"
                  value="light"
                  color="primary"
                  density="compact"
                />
                <v-radio
                  :label="t('settings.appearance.dark')"
                  value="dark"
                  color="primary"
                  density="compact"
                />
              </v-radio-group>
            </SettingsItem>

            <SettingsItem
              :title="t('settings.appearance.reduceMotion')"
              :subtitle="t('settings.appearance.reduceMotionHint')"
              wide
            >
              <v-radio-group
                :model-value="reduceMotionPreference"
                :disabled="reduceMotionSaving"
                inline
                hide-details
                :aria-label="t('settings.appearance.reduceMotion')"
                class="settings-theme-group"
                @update:model-value="changeReduceMotion"
              >
                <v-radio
                  :label="t('settings.appearance.followSystem')"
                  value="system"
                  color="primary"
                  density="compact"
                />
                <v-radio
                  :label="t('settings.appearance.reduceMotionOn')"
                  value="on"
                  color="primary"
                  density="compact"
                />
                <v-radio
                  :label="t('settings.appearance.reduceMotionOff')"
                  value="off"
                  color="primary"
                  density="compact"
                />
              </v-radio-group>
            </SettingsItem>

            <SettingsItem :title="t('settings.appearance.language')" wide>
              <v-select
                :model-value="languagePreference"
                :items="languageOptions"
                item-title="title"
                item-value="value"
                :disabled="languageSaving"
                :loading="languageSaving"
                variant="outlined"
                density="compact"
                hide-details
                :aria-label="t('settings.appearance.language')"
                @update:model-value="changeLanguage"
              />
            </SettingsItem>

            <SettingsItem :title="t('settings.appearance.themeColor')" stacked>
              <div class="theme-color-controls">
                <v-color-input
                  :model-value="customPrimary"
                  color-pip
                  mode="hex"
                  :label="t('settings.appearance.primary')"
                  variant="outlined"
                  density="compact"
                  hide-details
                  class="theme-color-input"
                  :aria-label="t('settings.appearance.primary')"
                  @update:model-value="setCustomPrimary"
                />
                <v-color-input
                  :model-value="customSecondary"
                  color-pip
                  mode="hex"
                  :label="t('settings.appearance.secondary')"
                  variant="outlined"
                  density="compact"
                  hide-details
                  class="theme-color-input"
                  :aria-label="t('settings.appearance.secondary')"
                  @update:model-value="setCustomSecondary"
                />
                <v-btn
                  variant="text"
                  prepend-icon="mdi-restore"
                  :disabled="!customColorsActive"
                  @click="resetCustomColors"
                >
                  {{ t("settings.appearance.reset") }}
                </v-btn>
              </div>
            </SettingsItem>
          </div>
        </section>

        <section
          v-show="activeSection === 'network'"
          id="settings-network"
          class="settings-section"
          aria-labelledby="settings-network-title"
        >
          <h2 id="settings-network-title" class="settings-section__title">
            {{ t("settings.nav.network") }}
          </h2>

          <div class="settings-list-card">
            <SettingsItem
              :title="t('settings.network.allowLocalDnsProbe')"
              :subtitle="t('settings.network.allowLocalDnsProbeHint')"
              label-for="setting-allow-local-dns-probe"
            >
              <v-switch
                id="setting-allow-local-dns-probe"
                :model-value="allowLocalDnsProbe"
                :loading="allowLocalDnsProbeSaving"
                :disabled="allowLocalDnsProbeSaving"
                color="primary"
                density="compact"
                hide-details
                @update:model-value="changeAllowLocalDnsProbe"
              />
            </SettingsItem>
          </div>

          <div
            v-for="definition in NETWORK_SOURCES"
            :key="definition.key"
            class="settings-list-card"
          >
            <NetworkSourceList
              :definition="definition"
              :model-value="networkDrafts[definition.key]"
              :dirty="networkDirty[definition.key]"
              :saving="networkSaving[definition.key]"
              @update:model-value="
                (value: string) => setNetworkDraft(definition.key, value)
              "
              @save="saveNetworkSource(definition.key)"
            />
          </div>

          <div class="settings-list-card">
            <SettingsItem :title="t('settings.network.httpProxy')" stacked>
              <div class="settings-proxy-form">
                <v-text-field
                  v-model="proxyForm.url"
                  :label="t('settings.network.fields.url')"
                  variant="outlined"
                  density="compact"
                  hide-details
                  :disabled="proxySaving"
                  class="settings-proxy-form__url"
                  @update:model-value="markProxyDirty"
                />
                <v-text-field
                  v-model="proxyForm.port"
                  :label="t('settings.network.fields.port')"
                  variant="outlined"
                  density="compact"
                  hide-details
                  :disabled="proxySaving"
                  class="settings-proxy-form__port"
                  @update:model-value="markProxyDirty"
                />
                <v-text-field
                  v-model="proxyForm.username"
                  :label="t('settings.network.fields.username')"
                  variant="outlined"
                  density="compact"
                  hide-details
                  :disabled="proxySaving"
                  class="settings-proxy-form__credential"
                  @update:model-value="markProxyDirty"
                />
                <v-text-field
                  v-model="proxyForm.password"
                  :label="t('settings.network.fields.password')"
                  type="password"
                  variant="outlined"
                  density="compact"
                  hide-details
                  :disabled="proxySaving"
                  class="settings-proxy-form__credential"
                  @update:model-value="markProxyDirty"
                />
                <div class="settings-proxy-form__actions">
                  <v-tooltip
                    location="top"
                    :text="t('settings.network.sources.httpTestHint')"
                  >
                    <template #activator="{ props: tooltipProps }">
                      <v-btn
                        v-bind="tooltipProps"
                        variant="tonal"
                        color="primary"
                        :loading="proxyTesting"
                        :disabled="proxyTesting"
                        :aria-label="t('settings.network.sources.testHttp')"
                        @click="runProxyTest"
                      >
                        {{
                          proxyTesting
                            ? t("settings.network.sources.testing", {
                                done: 0,
                                total: 1,
                              })
                            : t("settings.network.sources.testHttp")
                        }}
                      </v-btn>
                    </template>
                  </v-tooltip>
                  <v-btn
                    color="primary"
                    variant="tonal"
                    :aria-label="t('settings.network.saveHttpProxy')"
                    :disabled="!proxyChanged || proxySaving"
                    :loading="proxySaving"
                    @click="saveProxyForm"
                  >
                    {{ t("common.actions.save") }}
                  </v-btn>
                </div>
              </div>
              <div
                v-if="proxyTestResult || proxyTestError"
                class="settings-proxy-result"
              >
                <template v-if="proxyTestResult">
                  <v-chip
                    size="small"
                    color="success"
                    variant="tonal"
                    :aria-label="`${t('settings.network.sources.latency')} ${formatLatency(proxyTestResult.latency_ms)}`"
                  >
                    {{ formatLatency(proxyTestResult.latency_ms) }}
                  </v-chip>
                  <v-chip
                    v-if="proxyTestResult.speed_bps !== null"
                    size="small"
                    color="primary"
                    variant="tonal"
                    :aria-label="`${t('settings.network.sources.speed')} ${formatSpeed(proxyTestResult.speed_bps)}`"
                  >
                    {{ formatSpeed(proxyTestResult.speed_bps) }}
                  </v-chip>
                </template>
                <span v-else class="settings-proxy-result__error" role="alert">
                  {{ proxyTestError }}
                </span>
              </div>
            </SettingsItem>
          </div>

        </section>

        <section
          v-show="activeSection === 'data'"
          id="settings-data"
          class="settings-section"
          aria-labelledby="settings-data-title"
        >
          <h2 id="settings-data-title" class="settings-section__title">
            {{ t("settings.nav.data") }}
          </h2>

          <div class="settings-list-card">
            <SettingsItem
              :title="t('settings.data.dataDirectory')"
              :subtitle="t('settings.data.dataDirectoryHint')"
              stacked
            >
              <div class="settings-path-row">
                <code class="settings-path">{{ dataDir || "—" }}</code>
                <v-btn
                  color="primary"
                  variant="tonal"
                  :aria-label="t('settings.data.changeDirectory')"
                  :loading="pickingDirectory"
                  :disabled="changeDirSaving"
                  @click="chooseDataDirectory"
                >
                  {{ t("settings.data.change") }}
                </v-btn>
              </div>
            </SettingsItem>

            <SettingsItem
              :title="t('settings.data.rebuildManifest')"
              :subtitle="t('settings.data.rebuildManifestHint')"
            >
              <v-btn
                color="primary"
                variant="tonal"
                :aria-label="t('settings.data.rebuildManifest')"
                :disabled="rebuilding"
                @click="rebuildDialogOpen = true"
              >
                {{ t("settings.data.rebuild") }}
              </v-btn>
            </SettingsItem>

            <SettingsItem
              :title="t('settings.data.uvCache')"
              :subtitle="t('settings.data.uvCacheHint')"
            >
              <v-btn
                color="primary"
                variant="tonal"
                :aria-label="t('settings.data.clearUvCacheAria')"
                :loading="clearingUvCache"
                :disabled="clearingUvCache"
                @click="handleClearUvCache"
              >
                {{ t("settings.data.clear") }}
              </v-btn>
            </SettingsItem>
          </div>

          <v-alert
            v-if="restartPendingDir"
            class="settings-notice"
            type="warning"
            variant="tonal"
            density="comfortable"
            role="status"
          >
            <div>{{ t("settings.data.restartNotice") }}</div>
            <code class="settings-path">{{ restartPendingDir }}</code>
          </v-alert>
        </section>

        <section
          v-show="activeSection === 'about'"
          id="settings-about"
          class="settings-section"
          aria-labelledby="settings-about-title"
        >
          <h2 id="settings-about-title" class="settings-section__title">
            {{ t("settings.nav.about") }}
          </h2>
          <div class="settings-list-card">
            <LauncherUpdateSection />
          </div>
          <div class="settings-list-card">
            <SettingsItem :title="t('settings.about.appName')">
              <span class="settings-value">AstrBot Launcher</span>
            </SettingsItem>
            <SettingsItem :title="t('settings.about.version')">
              <span class="settings-value">{{ appVersion }}</span>
            </SettingsItem>
            <SettingsItem
              title="astrbot-launcher"
              :subtitle="t('settings.about.launcherRepoDescription')"
              wide
            >
              <v-btn
                variant="text"
                color="primary"
                size="small"
                append-icon="mdi-open-in-new"
                :aria-label="t('settings.about.openLauncherRepo')"
                @click="openLink(launcherRepoUrl)"
              >
                {{ t("settings.about.openRepository") }}
              </v-btn>
            </SettingsItem>
            <SettingsItem
              title="astrbot-launcher-vue"
              :subtitle="t('settings.about.currentRepoDescription')"
              wide
            >
              <v-btn
                variant="text"
                color="primary"
                size="small"
                append-icon="mdi-open-in-new"
                :aria-label="t('settings.about.openCurrentRepo')"
                @click="openLink(currentRepoUrl)"
              >
                {{ t("settings.about.openRepository") }}
              </v-btn>
            </SettingsItem>
            <SettingsItem
              title="AstrBot"
              :subtitle="t('settings.about.astrbotRepoDescription')"
              wide
            >
              <v-btn
                variant="text"
                color="primary"
                size="small"
                append-icon="mdi-open-in-new"
                :aria-label="t('settings.about.openAstrbotRepo')"
                @click="openLink(astrbotRepoUrl)"
              >
                {{ t("settings.about.openRepository") }}
              </v-btn>
            </SettingsItem>
          </div>
        </section>
      </div>
    </div>

    <ConfirmDialog
      v-model="changeDirDialogOpen"
      :title="t('settings.data.changeDirectory')"
      :message="
        t('settings.data.changeDirectoryMessage', { path: pendingDir })
      "
      :confirm-label="t('settings.data.change')"
      color="warning"
      :loading="changeDirSaving"
      @confirm="confirmDataDirectoryChange"
    />

    <ConfirmDialog
      v-model="rebuildDialogOpen"
      :title="t('settings.data.rebuildManifest')"
      :message="t('settings.data.rebuildManifestMessage')"
      :confirm-label="t('settings.data.rebuild')"
      :loading="rebuilding"
      @confirm="confirmRebuildManifest"
    />
  </section>
</template>

<style scoped>
.settings-page {
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
  width: 100%;
  max-width: 940px;
  height: 100%;
  min-height: 0;
  margin: 0 auto;
}

.settings-state {
  display: grid;
  flex: 1;
  min-height: 0;
  place-items: center;
}

.settings-layout {
  display: grid;
  flex: 1;
  grid-template-columns: 126px minmax(0, 1fr);
  grid-template-rows: minmax(0, 1fr);
  gap: 34px;
  min-height: 0;
}

.settings-nav {
  display: flex;
  flex-direction: column;
  gap: 5px;
  padding-top: 2px;
  overflow-y: auto;
  scrollbar-width: thin;
}

.settings-nav__item {
  position: relative;
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 8px;
  width: 100%;
  min-height: 34px;
  padding: 6px 9px 6px 12px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: rgba(var(--v-theme-on-surface), 0.64);
  cursor: pointer;
  font: inherit;
  font-size: 0.84rem;
  font-weight: 650;
  line-height: 1.25;
  text-align: left;
}

.settings-nav__item:hover {
  background: rgba(var(--v-theme-on-surface), 0.045);
  color: rgb(var(--v-theme-on-surface));
}

.settings-nav__item--active {
  background: rgba(var(--v-theme-on-surface), 0.07);
  color: rgb(var(--v-theme-on-surface));
  font-weight: 750;
}

.settings-nav__item--active::before {
  position: absolute;
  top: 7px;
  bottom: 7px;
  left: 0;
  width: 2px;
  border-radius: 999px;
  background: rgba(var(--v-theme-on-surface), 0.52);
  content: "";
}

.settings-nav__item:focus-visible {
  outline: 2px solid rgb(var(--v-theme-primary));
  outline-offset: 2px;
}

.settings-body {
  min-width: 0;
  height: 100%;
  padding-right: 4px;
  padding-bottom: 8px;
  overflow-y: auto;
  overscroll-behavior: contain;
  scrollbar-gutter: stable;
}

.settings-section {
  min-width: 0;
}

.settings-section__title {
  margin: 0 0 12px;
  color: rgb(var(--v-theme-on-surface));
  font-size: 1.05rem;
  font-weight: 700;
  letter-spacing: 0;
  line-height: 1.3;
}

.settings-list-card {
  overflow: hidden;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.1);
  border-radius: 16px;
  background: rgb(var(--v-theme-surface));
}

.settings-list-card + .settings-list-card {
  margin-top: 14px;
}

.settings-field-row {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}

.settings-inline-field {
  flex: 1 1 260px;
  min-width: 0;
}

.settings-field-row :deep(.v-btn) {
  flex: 0 0 auto;
}

.settings-proxy-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}

.settings-proxy-form__url {
  flex: 2 1 200px;
  min-width: 0;
}

.settings-proxy-form__port {
  flex: 0 1 110px;
  min-width: 0;
}

.settings-proxy-form__credential {
  flex: 1 1 140px;
  min-width: 0;
}

.settings-proxy-form__actions {
  display: flex;
  flex: 0 0 auto;
  flex-wrap: wrap;
  gap: 8px;
  margin-left: auto;
}

.settings-proxy-result {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  margin-top: 10px;
}

.settings-proxy-result__error {
  color: rgb(var(--v-theme-error));
  font-size: 0.8rem;
}

.settings-path-row {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  align-items: center;
  justify-content: space-between;
}

.settings-path {
  min-width: 0;
  color: rgba(var(--v-theme-on-surface), 0.76);
  font-family: "Consolas", "Cascadia Mono", monospace;
  font-size: 0.8rem;
  line-height: 1.5;
  overflow-wrap: anywhere;
}

.settings-value {
  color: rgba(var(--v-theme-on-surface), 0.76);
  font-size: 0.84rem;
}

.settings-theme-group {
  width: 100%;
}

.settings-theme-group :deep(.v-selection-control-group) {
  justify-content: flex-end;
}

.theme-color-controls {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.theme-color-input {
  flex: 1 1 180px;
  max-width: 240px;
}

.settings-notice {
  margin-top: 14px;
}

.settings-notice .settings-path {
  display: block;
  margin-top: 6px;
  color: inherit;
}

@media (max-width: 720px) {
  .settings-layout {
    grid-template-columns: 1fr;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 16px;
  }

  .settings-nav {
    flex-direction: row;
    gap: 6px;
    padding: 0 0 4px;
    overflow-x: auto;
    overflow-y: hidden;
  }

  .settings-nav__item {
    width: auto;
    min-height: 32px;
    padding: 5px 11px;
    border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
    border-radius: 999px;
    background: rgb(var(--v-theme-surface));
    white-space: nowrap;
  }

  .settings-nav__item--active::before {
    top: 8px;
    bottom: 8px;
    left: 4px;
  }

  .settings-nav__item--active {
    padding-left: 14px;
  }

  .settings-body {
    padding-right: 0;
    scrollbar-gutter: auto;
  }

  .settings-proxy-form__save {
    margin-left: 0;
  }

  .settings-theme-group :deep(.v-selection-control-group) {
    justify-content: flex-start;
  }
}
</style>
