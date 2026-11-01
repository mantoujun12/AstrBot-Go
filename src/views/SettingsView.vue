<script setup lang="ts">
import { computed, nextTick, onMounted, reactive, ref, watch } from "vue";
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
import {
  getAppVersion,
  openExternalUrl,
  pickDirectory,
  setDataDir,
} from "../services/app";
import { clearUvCache } from "../services/components";
import {
  saveAutostartMinimizeToTray,
  saveCheckInstanceUpdate,
  saveCloseToTray,
  saveGithubProxy,
  saveIgnoreExternalPath,
  saveLockCheckExtensionWhitelist,
  saveMainlandAcceleration,
  saveNodejsMirror,
  saveNpmRegistry,
  savePersistInstanceState,
  saveProxy,
  savePypiMirror,
  saveThemePreference,
  saveUseShortInstanceId,
  saveUseUvForDeps,
} from "../services/config";
import { toUserMessage } from "../services/errors";
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
import type { AppConfig, ThemePreference } from "../types/launcher";
import { cacheTheme } from "../utils/themeCache";

const { config, dataDir, loading, error, refresh } = useLauncherStore();

type SettingsSection = "general" | "appearance" | "network" | "data" | "about";

interface SettingsNavItem {
  id: SettingsSection;
  label: string;
  icon: string;
}

const sections: SettingsNavItem[] = [
  { id: "general", label: "通用", icon: "mdi-tune-variant" },
  { id: "appearance", label: "外观", icon: "mdi-palette-outline" },
  { id: "network", label: "网络", icon: "mdi-lan-connect" },
  { id: "data", label: "数据与维护", icon: "mdi-tools" },
  { id: "about", label: "关于", icon: "mdi-information-outline" },
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
    showToast(`无法读取开机自启状态：${toUserMessage(cause)}`, "error");
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
  title: string;
  subtitle?: string;
  save: (value: boolean) => Promise<void>;
}

const toggleDefinitions: ToggleDefinition[] = [
  {
    key: "close_to_tray",
    title: "关闭时最小化到托盘",
    save: saveCloseToTray,
  },
  {
    key: "autostart_minimize_to_tray",
    title: "开机启动后最小化到托盘",
    save: saveAutostartMinimizeToTray,
  },
  {
    key: "check_instance_update",
    title: "检查实例更新",
    save: saveCheckInstanceUpdate,
  },
  {
    key: "persist_instance_state",
    title: "记住实例运行状态",
    save: savePersistInstanceState,
  },
  {
    key: "ignore_external_path",
    title: "允许外部路径",
    save: saveIgnoreExternalPath,
  },
  {
    key: "use_short_instance_id",
    title: "使用短实例 ID",
    save: saveUseShortInstanceId,
  },
  {
    key: "lock_check_extension_whitelist",
    title: "文件锁扩展名白名单",
    save: saveLockCheckExtensionWhitelist,
  },
  {
    key: "use_uv_for_deps",
    title: "使用 uv 安装依赖",
    subtitle: "需要已安装 uv",
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

const mainlandAcceleration = ref(false);
const mainlandSaving = ref(false);

const githubProxy = ref("");
const githubDirty = ref(false);
const githubSaving = ref(false);

const proxyForm = reactive({
  url: "",
  port: "",
  username: "",
  password: "",
});
const proxyDirty = ref(false);
const proxySaving = ref(false);

const pypiMirror = ref("");
const pypiDirty = ref(false);
const pypiSaving = ref(false);

const nodejsMirror = ref("");
const nodejsDirty = ref(false);
const nodejsSaving = ref(false);

const npmRegistry = ref("");
const npmDirty = ref(false);
const npmSaving = ref(false);

const proxyDisabled = computed(() => mainlandAcceleration.value);

const githubChanged = computed(() => {
  const value = config.value;
  return value !== null && githubProxy.value !== value.github_proxy;
});

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

const pypiChanged = computed(() => {
  const value = config.value;
  return value !== null && pypiMirror.value !== value.pypi_mirror;
});

const nodejsChanged = computed(() => {
  const value = config.value;
  return value !== null && nodejsMirror.value !== value.nodejs_mirror;
});

const npmChanged = computed(() => {
  const value = config.value;
  return value !== null && npmRegistry.value !== value.npm_registry;
});

function markProxyDirty(): void {
  proxyDirty.value = true;
}

async function changeMainlandAcceleration(value: unknown): Promise<void> {
  if (typeof value !== "boolean" || mainlandSaving.value) {
    return;
  }

  const previous =
    config.value?.mainland_acceleration ?? mainlandAcceleration.value;
  mainlandAcceleration.value = value;
  mainlandSaving.value = true;
  try {
    await saveMainlandAcceleration(value);
    await refresh();
  } catch (cause) {
    mainlandAcceleration.value = previous;
    showErrorToast(cause);
  } finally {
    mainlandSaving.value = false;
  }
}

async function saveGithubProxyForm(): Promise<void> {
  if (githubSaving.value || !githubChanged.value) {
    return;
  }

  githubSaving.value = true;
  try {
    await saveGithubProxy(githubProxy.value);
    githubDirty.value = false;
    await refresh();
    showToast("已保存");
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    githubSaving.value = false;
  }
}

async function saveProxyForm(): Promise<void> {
  if (proxySaving.value || proxyDisabled.value || !proxyChanged.value) {
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
    showToast("已保存");
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    proxySaving.value = false;
  }
}

async function savePypiMirrorForm(): Promise<void> {
  if (pypiSaving.value || !pypiChanged.value) {
    return;
  }

  pypiSaving.value = true;
  try {
    await savePypiMirror(pypiMirror.value);
    pypiDirty.value = false;
    await refresh();
    showToast("已保存");
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    pypiSaving.value = false;
  }
}

async function saveNodejsMirrorForm(): Promise<void> {
  if (nodejsSaving.value || !nodejsChanged.value) {
    return;
  }

  nodejsSaving.value = true;
  try {
    await saveNodejsMirror(nodejsMirror.value);
    nodejsDirty.value = false;
    await refresh();
    showToast("已保存");
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    nodejsSaving.value = false;
  }
}

async function saveNpmRegistryForm(): Promise<void> {
  if (npmSaving.value || !npmChanged.value) {
    return;
  }

  npmSaving.value = true;
  try {
    await saveNpmRegistry(npmRegistry.value);
    npmDirty.value = false;
    await refresh();
    showToast("已保存");
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    npmSaving.value = false;
  }
}

function syncNetworkForms(value: AppConfig | null): void {
  if (!value) {
    return;
  }
  if (!githubDirty.value) {
    githubProxy.value = value.github_proxy;
  }
  if (!proxyDirty.value) {
    proxyForm.url = value.proxy_url;
    proxyForm.port = value.proxy_port;
    proxyForm.username = value.proxy_username;
    proxyForm.password = value.proxy_password;
  }
  if (!pypiDirty.value) {
    pypiMirror.value = value.pypi_mirror;
  }
  if (!nodejsDirty.value) {
    nodejsMirror.value = value.nodejs_mirror;
  }
  if (!npmDirty.value) {
    npmRegistry.value = value.npm_registry;
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
    showToast(`已重建 ${result.instances} 个实例，${result.versions} 个版本`);
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
    showToast("uv 缓存已清理");
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    clearingUvCache.value = false;
  }
}

/* About */

const appVersion = ref("—");
const launcherRepoUrl = "https://github.com/AstrBotDevs/astrbot-launcher";
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
    showToast(`无法打开链接：${toUserMessage(cause)}`, "error");
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

    if (!mainlandSaving.value) {
      mainlandAcceleration.value = value.mainland_acceleration;
    }
    if (!themeSaving.value) {
      themePreference.value = value.theme_preference;
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
  <section class="settings-page" aria-label="设置">
    <PageHeader title="设置" />

    <div v-if="!config" class="settings-state">
      <v-progress-circular
        v-if="loading"
        indeterminate
        color="primary"
        aria-label="加载设置"
      />
      <EmptyState
        v-else
        icon="mdi-alert-circle-outline"
        :text="error ?? '设置加载失败'"
      >
        <v-btn variant="tonal" color="primary" size="small" @click="refresh">
          重试
        </v-btn>
      </EmptyState>
    </div>

    <div v-else class="settings-layout">
      <nav class="settings-nav" aria-label="设置分区">
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
          <span>{{ item.label }}</span>
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
            通用
          </h2>
          <div class="settings-list-card">
            <SettingsItem
              title="开机自启"
              :subtitle="autostartUnavailable ? '当前平台不可用' : ''"
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
              :title="item.title"
              :subtitle="item.subtitle ?? ''"
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
            外观
          </h2>
          <div class="settings-list-card">
            <SettingsItem title="主题" wide>
              <v-radio-group
                :model-value="themePreference"
                :disabled="themeSaving"
                inline
                hide-details
                aria-label="主题"
                class="settings-theme-group"
                @update:model-value="changeTheme"
              >
                <v-radio
                  label="跟随系统"
                  value="system"
                  color="primary"
                  density="compact"
                />
                <v-radio
                  label="浅色"
                  value="light"
                  color="primary"
                  density="compact"
                />
                <v-radio
                  label="深色"
                  value="dark"
                  color="primary"
                  density="compact"
                />
              </v-radio-group>
            </SettingsItem>

            <SettingsItem title="主题色" stacked>
              <div class="theme-color-controls">
                <v-color-input
                  :model-value="customPrimary"
                  color-pip
                  mode="hex"
                  label="主色"
                  variant="outlined"
                  density="compact"
                  hide-details
                  class="theme-color-input"
                  aria-label="主色"
                  @update:model-value="setCustomPrimary"
                />
                <v-color-input
                  :model-value="customSecondary"
                  color-pip
                  mode="hex"
                  label="次色"
                  variant="outlined"
                  density="compact"
                  hide-details
                  class="theme-color-input"
                  aria-label="次色"
                  @update:model-value="setCustomSecondary"
                />
                <v-btn
                  variant="text"
                  prepend-icon="mdi-restore"
                  :disabled="!customColorsActive"
                  @click="resetCustomColors"
                >
                  重置
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
            网络
          </h2>

          <div class="settings-list-card">
            <SettingsItem
              title="大陆加速"
              label-for="setting-mainland-acceleration"
            >
              <v-switch
                id="setting-mainland-acceleration"
                :model-value="mainlandAcceleration"
                :loading="mainlandSaving"
                :disabled="mainlandSaving"
                color="primary"
                density="compact"
                hide-details
                @update:model-value="changeMainlandAcceleration"
              />
            </SettingsItem>
          </div>

          <div class="settings-list-card">
            <SettingsItem
              title="GitHub 代理"
              stacked
              label-for="setting-github-proxy"
            >
              <div class="settings-field-row">
                <v-text-field
                  id="setting-github-proxy"
                  v-model="githubProxy"
                  aria-label="GitHub 代理"
                  variant="outlined"
                  density="compact"
                  hide-details
                  clearable
                  :disabled="githubSaving"
                  class="settings-inline-field"
                  @update:model-value="githubDirty = true"
                />
                <v-btn
                  color="primary"
                  variant="tonal"
                  aria-label="保存 GitHub 代理"
                  :disabled="!githubChanged || githubSaving"
                  :loading="githubSaving"
                  @click="saveGithubProxyForm"
                >
                  保存
                </v-btn>
              </div>
            </SettingsItem>
          </div>

          <div class="settings-list-card">
            <SettingsItem
              title="HTTP 代理"
              :subtitle="proxyDisabled ? '大陆加速开启时不可用' : ''"
              stacked
            >
              <div class="settings-proxy-form">
                <v-text-field
                  v-model="proxyForm.url"
                  label="地址"
                  variant="outlined"
                  density="compact"
                  hide-details
                  :disabled="proxyDisabled || proxySaving"
                  class="settings-proxy-form__url"
                  @update:model-value="markProxyDirty"
                />
                <v-text-field
                  v-model="proxyForm.port"
                  label="端口"
                  variant="outlined"
                  density="compact"
                  hide-details
                  :disabled="proxyDisabled || proxySaving"
                  class="settings-proxy-form__port"
                  @update:model-value="markProxyDirty"
                />
                <v-text-field
                  v-model="proxyForm.username"
                  label="用户名"
                  variant="outlined"
                  density="compact"
                  hide-details
                  :disabled="proxyDisabled || proxySaving"
                  class="settings-proxy-form__credential"
                  @update:model-value="markProxyDirty"
                />
                <v-text-field
                  v-model="proxyForm.password"
                  label="密码"
                  type="password"
                  variant="outlined"
                  density="compact"
                  hide-details
                  :disabled="proxyDisabled || proxySaving"
                  class="settings-proxy-form__credential"
                  @update:model-value="markProxyDirty"
                />
                <v-btn
                  color="primary"
                  variant="tonal"
                  class="settings-proxy-form__save"
                  aria-label="保存 HTTP 代理"
                  :disabled="proxyDisabled || !proxyChanged || proxySaving"
                  :loading="proxySaving"
                  @click="saveProxyForm"
                >
                  保存
                </v-btn>
              </div>
            </SettingsItem>
          </div>

          <div class="settings-list-card">
            <SettingsItem
              title="PyPI 镜像"
              stacked
              label-for="setting-pypi-mirror"
            >
              <div class="settings-field-row">
                <v-text-field
                  id="setting-pypi-mirror"
                  v-model="pypiMirror"
                  aria-label="PyPI 镜像"
                  variant="outlined"
                  density="compact"
                  hide-details
                  clearable
                  :disabled="pypiSaving"
                  class="settings-inline-field"
                  @update:model-value="pypiDirty = true"
                />
                <v-btn
                  color="primary"
                  variant="tonal"
                  aria-label="保存 PyPI 镜像"
                  :disabled="!pypiChanged || pypiSaving"
                  :loading="pypiSaving"
                  @click="savePypiMirrorForm"
                >
                  保存
                </v-btn>
              </div>
            </SettingsItem>

            <SettingsItem
              title="Node.js 镜像"
              stacked
              label-for="setting-nodejs-mirror"
            >
              <div class="settings-field-row">
                <v-text-field
                  id="setting-nodejs-mirror"
                  v-model="nodejsMirror"
                  aria-label="Node.js 镜像"
                  variant="outlined"
                  density="compact"
                  hide-details
                  clearable
                  :disabled="nodejsSaving"
                  class="settings-inline-field"
                  @update:model-value="nodejsDirty = true"
                />
                <v-btn
                  color="primary"
                  variant="tonal"
                  aria-label="保存 Node.js 镜像"
                  :disabled="!nodejsChanged || nodejsSaving"
                  :loading="nodejsSaving"
                  @click="saveNodejsMirrorForm"
                >
                  保存
                </v-btn>
              </div>
            </SettingsItem>

            <SettingsItem
              title="npm registry"
              stacked
              label-for="setting-npm-registry"
            >
              <div class="settings-field-row">
                <v-text-field
                  id="setting-npm-registry"
                  v-model="npmRegistry"
                  aria-label="npm registry"
                  variant="outlined"
                  density="compact"
                  hide-details
                  clearable
                  :disabled="npmSaving"
                  class="settings-inline-field"
                  @update:model-value="npmDirty = true"
                />
                <v-btn
                  color="primary"
                  variant="tonal"
                  aria-label="保存 npm registry"
                  :disabled="!npmChanged || npmSaving"
                  :loading="npmSaving"
                  @click="saveNpmRegistryForm"
                >
                  保存
                </v-btn>
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
            数据与维护
          </h2>

          <div class="settings-list-card">
            <SettingsItem title="数据目录" stacked>
              <div class="settings-path-row">
                <code class="settings-path">{{ dataDir || "—" }}</code>
                <v-btn
                  color="primary"
                  variant="tonal"
                  aria-label="更改数据目录"
                  :loading="pickingDirectory"
                  :disabled="changeDirSaving"
                  @click="chooseDataDirectory"
                >
                  更改目录
                </v-btn>
              </div>
            </SettingsItem>

            <SettingsItem title="重建实例清单">
              <v-btn
                color="primary"
                variant="tonal"
                aria-label="重建实例清单"
                :disabled="rebuilding"
                @click="rebuildDialogOpen = true"
              >
                重建
              </v-btn>
            </SettingsItem>

            <SettingsItem title="uv 缓存">
              <v-btn
                color="primary"
                variant="tonal"
                aria-label="清理 uv 缓存"
                :loading="clearingUvCache"
                :disabled="clearingUvCache"
                @click="handleClearUvCache"
              >
                清理
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
            <div>数据目录已更改，重启启动器后生效。</div>
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
            关于
          </h2>
          <div class="settings-list-card">
            <SettingsItem title="应用名称">
              <span class="settings-value">AstrBot Launcher</span>
            </SettingsItem>
            <SettingsItem title="版本">
              <span class="settings-value">{{ appVersion }}</span>
            </SettingsItem>
            <SettingsItem title="GitHub">
              <v-btn
                variant="text"
                color="primary"
                size="small"
                append-icon="mdi-open-in-new"
                aria-label="打开 astrbot-launcher 仓库"
                @click="openLink(launcherRepoUrl)"
              >
                astrbot-launcher
              </v-btn>
            </SettingsItem>
            <SettingsItem title="AstrBot">
              <v-btn
                variant="text"
                color="primary"
                size="small"
                append-icon="mdi-open-in-new"
                aria-label="打开 AstrBot 仓库"
                @click="openLink(astrbotRepoUrl)"
              >
                AstrBot
              </v-btn>
            </SettingsItem>
          </div>
        </section>
      </div>
    </div>

    <ConfirmDialog
      v-model="changeDirDialogOpen"
      title="更改数据目录"
      :message="`新目录：${pendingDir}\n重启启动器后生效。`"
      confirm-label="更改"
      color="warning"
      :loading="changeDirSaving"
      @confirm="confirmDataDirectoryChange"
    />

    <ConfirmDialog
      v-model="rebuildDialogOpen"
      title="重建实例清单"
      message="将根据磁盘内容重建实例清单。"
      confirm-label="重建"
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

.settings-proxy-form__save {
  flex: 0 0 auto;
  margin-left: auto;
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
