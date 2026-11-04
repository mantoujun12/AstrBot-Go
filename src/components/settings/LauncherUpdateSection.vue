<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import SettingsItem from "./SettingsItem.vue";
import {
  checkLauncherUpdate,
  downloadLauncherUpdate,
  installLauncherUpdate,
  listenLauncherUpdateProgress,
  type LauncherUpdateInfo,
  type LauncherUpdateProgress,
} from "../../services/updater";
import { toUserMessage } from "../../services/errors";
import { formatFileSize } from "../../utils/format";
import type { UnlistenFn } from "@tauri-apps/api/event";

type UpdateStatus =
  | "idle"
  | "checking"
  | "unavailable"
  | "available"
  | "downloading"
  | "downloaded"
  | "installing";

const { t } = useI18n();
const status = ref<UpdateStatus>("idle");
const updateInfo = ref<LauncherUpdateInfo | null>(null);
const progress = ref<LauncherUpdateProgress | null>(null);
const progressReady = ref(false);
const errorMessage = ref("");
let unlisten: UnlistenFn | null = null;
let disposed = false;

const checking = computed(() => status.value === "checking");
const downloading = computed(() => status.value === "downloading");
const installing = computed(() => status.value === "installing");
const available = computed(() => status.value === "available");
const downloaded = computed(() => status.value === "downloaded");
const busy = computed(
  () => checking.value || downloading.value || installing.value,
);

const statusMessage = computed(() => {
  if (errorMessage.value) {
    return errorMessage.value;
  }
  if (checking.value) {
    return t("settings.about.updater.checking");
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
  return "";
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

async function check(): Promise<void> {
  if (busy.value) {
    return;
  }

  status.value = "checking";
  errorMessage.value = "";
  progress.value = null;
  try {
    updateInfo.value = await checkLauncherUpdate();
    status.value = updateInfo.value.available ? "available" : "unavailable";
  } catch (error) {
    status.value = "idle";
    errorMessage.value = toUserMessage(error);
  }
}

async function download(): Promise<void> {
  if (busy.value) {
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
    status.value = updateInfo.value.available ? "downloaded" : "unavailable";
    if (!updateInfo.value.available) {
      progress.value = null;
    }
  } catch (error) {
    status.value = updateInfo.value?.available ? "available" : "idle";
    errorMessage.value = toUserMessage(error);
  }
}

async function install(): Promise<void> {
  if (busy.value) {
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
  <SettingsItem
    :title="t('settings.about.updater.title')"
    :subtitle="statusMessage"
    stacked
  >
    <div class="launcher-update">
      <div class="launcher-update__actions">
        <v-btn
          color="primary"
          variant="tonal"
          size="small"
          :loading="checking"
          :disabled="busy"
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
    </div>
  </SettingsItem>
</template>

<style scoped>
.launcher-update__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: flex-start;
}

.launcher-update__progress {
  margin-top: 12px;
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
</style>
