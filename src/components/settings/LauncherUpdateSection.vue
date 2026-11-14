<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import SettingsItem from "./SettingsItem.vue";
import {
  saveAutoCheckLauncherUpdate,
  saveAutoUpdateLauncher,
} from "../../services/config";
import {
  checkLauncherUpdate,
  downloadLauncherUpdate,
  installLauncherUpdate,
  listenLauncherUpdateProgress,
  type LauncherUpdateInfo,
  type LauncherUpdateProgress,
} from "../../services/updater";
import { toUserMessage } from "../../services/errors";
import { runMutation, useLauncherStore } from "../../stores/launcher";
import { showErrorToast } from "../../stores/toast";
import { formatFileSize } from "../../utils/format";
import type { UnlistenFn } from "@tauri-apps/api/event";

type UpdateStatus =
  | "idle"
  | "checking"
  | "unsupported"
  | "unavailable"
  | "available"
  | "downloading"
  | "downloaded"
  | "installing";

const { t } = useI18n();
const { config } = useLauncherStore();
const status = ref<UpdateStatus>("idle");
const updateInfo = ref<LauncherUpdateInfo | null>(null);
const progress = ref<LauncherUpdateProgress | null>(null);
const progressReady = ref(false);
const errorMessage = ref("");

const autoCheck = ref(true);
const autoUpdate = ref(false);
const autoCheckSaving = ref(false);
const autoUpdateSaving = ref(false);

let unlisten: UnlistenFn | null = null;
let disposed = false;

const checking = computed(() => status.value === "checking");
const supported = computed(() => updateInfo.value?.supported !== false);
const downloading = computed(() => status.value === "downloading");
const installing = computed(() => status.value === "installing");
const available = computed(() => status.value === "available");
const downloaded = computed(() => status.value === "downloaded");
const busy = computed(
  () => checking.value || downloading.value || installing.value,
);
const autoUpdateDisabled = computed(
  () => !autoCheck.value || autoCheckSaving.value || autoUpdateSaving.value,
);

const statusMessage = computed(() => {
  if (errorMessage.value) {
    return errorMessage.value;
  }
  if (checking.value) {
    return t("settings.about.updater.checking");
  }
  if (status.value === "unsupported") {
    return t("settings.about.updater.unsupported");
  }
  if (downloading.value) {
    return t("settings.about.updater.downloading");
  }
  if (installing.value) {
    return t("settings.about.updater.installing");
  }
  if (downloaded.value) {
    return t("settings.about.updater.downloaded");
  }
  if (available.value && updateInfo.value) {
    return t("settings.about.updater.available", {
      version: updateInfo.value.latest_version,
    });
  }
  if (status.value === "unavailable") {
    return t("settings.about.updater.upToDate");
  }
  return t("settings.about.updater.idle");
});

const percent = computed(() => {
  const value = progress.value?.progress;
  return value === null || value === undefined ? null : Math.round(value);
});

const sizeText = computed(() => {
  const value = progress.value;
  if (!value || value.downloaded <= 0) {
    return "";
  }
  return value.total
    ? `${formatFileSize(value.downloaded)} / ${formatFileSize(value.total)}`
    : formatFileSize(value.downloaded);
});

watch(
  config,
  (value) => {
    if (!value) {
      return;
    }
    if (!autoCheckSaving.value) {
      autoCheck.value = value.auto_check_launcher_update;
    }
    if (!autoUpdateSaving.value) {
      autoUpdate.value = value.auto_update_launcher;
    }
  },
  { immediate: true },
);

async function changeAutoCheck(value: unknown): Promise<void> {
  if (typeof value !== "boolean" || autoCheckSaving.value) {
    return;
  }

  const previous = autoCheck.value;
  autoCheck.value = value;
  autoCheckSaving.value = true;
  try {
    await runMutation(() => saveAutoCheckLauncherUpdate(value));
  } catch (error) {
    autoCheck.value = previous;
    showErrorToast(error);
  } finally {
    autoCheckSaving.value = false;
  }
}

async function changeAutoUpdate(value: unknown): Promise<void> {
  if (typeof value !== "boolean" || autoUpdateSaving.value) {
    return;
  }

  const previous = autoUpdate.value;
  autoUpdate.value = value;
  autoUpdateSaving.value = true;
  try {
    await runMutation(() => saveAutoUpdateLauncher(value));
  } catch (error) {
    autoUpdate.value = previous;
    showErrorToast(error);
  } finally {
    autoUpdateSaving.value = false;
  }
}

async function check(): Promise<void> {
  if (busy.value) {
    return;
  }

  status.value = "checking";
  errorMessage.value = "";
  progress.value = null;
  try {
    updateInfo.value = await checkLauncherUpdate();
    status.value = !updateInfo.value.supported
      ? "unsupported"
      : updateInfo.value.available
        ? "available"
        : "unavailable";
  } catch (error) {
    status.value = "idle";
    errorMessage.value = toUserMessage(error);
  }
}

async function download(): Promise<void> {
  if (busy.value || !supported.value) {
    return;
  }
  if (!progressReady.value) {
    errorMessage.value = t("errors.updateFailed");
    return;
  }

  status.value = "downloading";
  errorMessage.value = "";
  progress.value = {
    downloaded: 0,
    total: null,
    progress: null,
    speed: null,
  };

  try {
    updateInfo.value = await downloadLauncherUpdate();
    status.value = !updateInfo.value.supported
      ? "unsupported"
      : updateInfo.value.available
        ? "downloaded"
        : "unavailable";
    if (!updateInfo.value.available) {
      progress.value = null;
    }
  } catch (error) {
    status.value = updateInfo.value?.available ? "available" : "idle";
    errorMessage.value = toUserMessage(error);
  }
}

async function install(): Promise<void> {
  if (busy.value || !supported.value) {
    return;
  }

  status.value = "installing";
  errorMessage.value = "";
  try {
    await installLauncherUpdate();
  } catch (error) {
    status.value = "downloaded";
    errorMessage.value = toUserMessage(error);
  }
}

onMounted(async () => {
  try {
    const stopListening = await listenLauncherUpdateProgress((event) => {
      if (status.value === "downloading") {
        progress.value = event;
      }
    });
    if (disposed) {
      stopListening();
    } else {
      unlisten = stopListening;
      progressReady.value = true;
    }
  } catch (error) {
    progressReady.value = false;
    errorMessage.value = toUserMessage(error);
  }
});

onUnmounted(() => {
  disposed = true;
  unlisten?.();
  unlisten = null;
});
</script>

<template>
  <div class="launcher-update">
    <div class="launcher-update__head">
      <div class="launcher-update__info">
        <div class="launcher-update__title">
          <v-icon icon="mdi-update" size="18" />
          <span>{{ t("settings.about.updater.title") }}</span>
        </div>
        <p
          class="launcher-update__status"
          :class="{
            'launcher-update__status--error': errorMessage,
            'launcher-update__status--available': available,
          }"
          role="status"
        >
          {{ statusMessage }}
        </p>
      </div>

      <div class="launcher-update__actions">
        <v-btn
          color="primary"
          variant="tonal"
          size="small"
          prepend-icon="mdi-refresh"
          :loading="checking"
          :disabled="busy || status === 'unsupported'"
          @click="check"
        >
          {{ t("settings.about.updater.check") }}
        </v-btn>
        <v-btn
          v-if="available"
          color="primary"
          variant="flat"
          size="small"
          prepend-icon="mdi-tray-arrow-down"
          :disabled="busy || !progressReady"
          @click="download"
        >
          {{ t("settings.about.updater.download") }}
        </v-btn>
        <v-btn
          v-if="downloading"
          color="primary"
          variant="flat"
          size="small"
          :loading="true"
          disabled
        >
          {{ t("settings.about.updater.download") }}
        </v-btn>
        <v-btn
          v-if="downloaded || installing"
          color="warning"
          variant="flat"
          size="small"
          prepend-icon="mdi-restart-alert"
          :loading="installing"
          :disabled="busy"
          @click="install"
        >
          {{ t("settings.about.updater.installAndRestart") }}
        </v-btn>
      </div>
    </div>

    <div
      v-if="downloading && progress"
      class="launcher-update__progress"
      role="status"
    >
      <v-progress-linear
        :model-value="progress.progress ?? 0"
        :indeterminate="progress.progress === null"
        color="primary"
        height="5"
        rounded
      />
      <div class="launcher-update__progress-meta">
        <span>{{ sizeText }}</span>
        <span v-if="percent !== null">{{ percent }}%</span>
      </div>
    </div>

    <div class="launcher-update__options">
      <SettingsItem
        :title="t('settings.about.updater.autoCheck')"
        :subtitle="t('settings.about.updater.autoCheckHint')"
        label-for="setting-auto-check-launcher-update"
      >
        <v-switch
          id="setting-auto-check-launcher-update"
          inset
          color="primary"
          density="compact"
          hide-details
          :model-value="autoCheck"
          :loading="autoCheckSaving"
          :disabled="autoCheckSaving || status === 'unsupported'"
          @update:model-value="changeAutoCheck"
        />
      </SettingsItem>
      <SettingsItem
        :title="t('settings.about.updater.autoUpdate')"
        :subtitle="
          autoCheck
            ? t('settings.about.updater.autoUpdateHint')
            : t('settings.about.updater.autoUpdateRequiresCheck')
        "
        label-for="setting-auto-update-launcher"
      >
        <v-switch
          id="setting-auto-update-launcher"
          inset
          color="primary"
          density="compact"
          hide-details
          :model-value="autoUpdate"
          :loading="autoUpdateSaving"
          :disabled="autoUpdateDisabled || status === 'unsupported'"
          @update:model-value="changeAutoUpdate"
        />
      </SettingsItem>
    </div>
  </div>
</template>

<style scoped>
.launcher-update__head {
  display: flex;
  flex-wrap: wrap;
  gap: 12px 20px;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
}

.launcher-update__info {
  flex: 1 1 260px;
  min-width: 0;
}

.launcher-update__title {
  display: flex;
  gap: 8px;
  align-items: center;
  color: rgb(var(--v-theme-on-surface));
  font-size: 0.875rem;
  font-weight: 600;
  line-height: 1.4;
}

.launcher-update__title :deep(.v-icon) {
  color: rgba(var(--v-theme-on-surface), 0.62);
}

.launcher-update__status {
  margin: 4px 0 0;
  min-height: 1.15rem;
  color: rgba(var(--v-theme-on-surface), 0.62);
  font-size: 0.78rem;
  line-height: 1.45;
}

.launcher-update__status--available {
  color: rgb(var(--v-theme-primary));
  font-weight: 500;
}

.launcher-update__status--error {
  color: rgb(var(--v-theme-error));
}

.launcher-update__actions {
  display: flex;
  flex: 0 1 auto;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  justify-content: flex-end;
}

.launcher-update__progress {
  padding: 0 16px 14px;
}

.launcher-update__progress-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 6px;
  color: rgba(var(--v-theme-on-surface), 0.56);
  font-size: 0.75rem;
  font-variant-numeric: tabular-nums;
}

.launcher-update__options {
  border-top: 1px solid rgba(var(--v-theme-on-surface), 0.08);
}

@media (max-width: 720px) {
  .launcher-update__actions {
    justify-content: flex-start;
    width: 100%;
  }
}
</style>