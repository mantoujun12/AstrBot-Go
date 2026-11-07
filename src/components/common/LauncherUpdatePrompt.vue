<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import type { UnlistenFn } from "@tauri-apps/api/event";
import MarkdownContent from "./MarkdownContent.vue";
import { saveIgnoredLauncherVersion } from "../../services/config";
import { toUserMessage } from "../../services/errors";
import {
  checkLauncherUpdate,
  downloadLauncherUpdate,
  installLauncherUpdate,
  listenLauncherUpdateProgress,
  type LauncherUpdateInfo,
  type LauncherUpdateProgress,
} from "../../services/updater";
import { runMutation, useLauncherStore } from "../../stores/launcher";
import { formatFileSize } from "../../utils/format";

type Stage = "idle" | "downloading" | "installing";

const { t } = useI18n();
const { config, init } = useLauncherStore();

const open = ref(false);
const info = ref<LauncherUpdateInfo | null>(null);
const stage = ref<Stage>("idle");
const progress = ref<LauncherUpdateProgress | null>(null);
const progressReady = ref(false);
const errorMessage = ref("");

let unlisten: UnlistenFn | null = null;
let disposed = false;

const busy = computed(
  () => stage.value === "downloading" || stage.value === "installing",
);

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
  let result: LauncherUpdateInfo;
  try {
    result = await checkLauncherUpdate();
  } catch {
    // Startup check is best-effort; never surface network errors here.
    return;
  }

  if (!result.available) {
    return;
  }

  const ignored = config.value?.ignored_launcher_version;
  if (ignored && ignored === result.latest_version) {
    return;
  }

  info.value = result;
  open.value = true;
}

async function update(): Promise<void> {
  if (busy.value) {
    return;
  }

  stage.value = "downloading";
  errorMessage.value = "";
  progress.value = {
    downloaded: 0,
    total: null,
    progress: null,
    speed: null,
  };

  try {
    const result = await downloadLauncherUpdate();
    if (!result.available) {
      // The update became unavailable between the check and this download
      // (e.g. a newer release was published). Close the prompt instead of
      // leaving it open in a stale state.
      open.value = false;
      stage.value = "idle";
      progress.value = null;
      return;
    }

    stage.value = "installing";
    await installLauncherUpdate();
    // The updater exits the app after launching the Windows installer.
  } catch (error) {
    stage.value = "idle";
    errorMessage.value = toUserMessage(error);
  }
}

function remindLater(): void {
  open.value = false;
}

async function ignore(): Promise<void> {
  const version = info.value?.latest_version;
  open.value = false;
  if (!version) {
    return;
  }

  try {
    await runMutation(() => saveIgnoredLauncherVersion(version));
  } catch {
    // A failed save only means the dialog may reappear on the next launch.
  }
}

onMounted(async () => {
  void listenLauncherUpdateProgress((event) => {
    // Also capture the trailing 100% event, which can be delivered just
    // after the stage flips to installing and would otherwise be dropped.
    if (stage.value === "downloading" || stage.value === "installing") {
      progress.value = event;
    }
  })
    .then((stop) => {
      if (disposed) {
        stop();
      } else {
        unlisten = stop;
        progressReady.value = true;
      }
    })
    .catch(() => {
      // Progress display is optional; the download still works without it.
    });

  try {
    // Ensure the config snapshot (with the ignored version) is loaded first.
    await init();
  } catch {
    // Config load failures must not block the update check.
  }
  void check();
});

onBeforeUnmount(() => {
  disposed = true;
  unlisten?.();
  unlisten = null;
});
</script>

<template>
  <v-dialog
    :model-value="open"
    max-width="560"
    scrollable
    :persistent="busy"
    @update:model-value="open = $event"
  >
    <v-card>
      <v-card-title class="launcher-update-prompt__title">
        {{ t("launcherUpdate.title", { version: info?.latest_version }) }}
      </v-card-title>
      <v-card-text class="launcher-update-prompt__body">
        <MarkdownContent
          v-if="info?.notes"
          :content="info.notes"
        />
        <p v-else class="launcher-update-prompt__empty">
          {{ t("launcherUpdate.noNotes") }}
        </p>

        <div
          v-if="stage === 'downloading' && progress"
          class="launcher-update-prompt__progress"
          role="status"
        >
          <v-progress-linear
            :model-value="progress.progress ?? 0"
            :indeterminate="progress.progress === null"
            color="primary"
            height="5"
            rounded
          />
          <div class="launcher-update-prompt__progress-meta">
            <span>{{ sizeText }}</span>
            <span v-if="percent !== null">{{ percent }}%</span>
          </div>
        </div>

        <p v-if="errorMessage" class="launcher-update-prompt__error" role="alert">
          {{ errorMessage }}
        </p>
      </v-card-text>
      <v-card-actions class="launcher-update-prompt__actions">
        <v-btn variant="text" :disabled="busy" @click="remindLater">
          {{ t("launcherUpdate.remindLater") }}
        </v-btn>
        <v-btn variant="text" :disabled="busy" @click="ignore">
          {{ t("launcherUpdate.ignore") }}
        </v-btn>
        <v-spacer />
        <v-btn
          color="primary"
          variant="flat"
          prepend-icon="mdi-tray-arrow-down"
          :loading="busy"
          :disabled="!progressReady"
          @click="update"
        >
          {{ t("launcherUpdate.update") }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.launcher-update-prompt__title {
  padding: 20px 20px 0;
  font-size: 1.05rem;
  font-weight: 650;
  white-space: normal;
}

.launcher-update-prompt__body {
  max-height: 60vh;
  padding: 12px 20px;
  overflow-y: auto;
}

.launcher-update-prompt__empty {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.56);
  font-size: 0.875rem;
}

.launcher-update-prompt__progress {
  margin-top: 12px;
}

.launcher-update-prompt__progress-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 6px;
  color: rgba(var(--v-theme-on-surface), 0.56);
  font-size: 0.75rem;
  font-variant-numeric: tabular-nums;
}

.launcher-update-prompt__error {
  margin: 12px 0 0;
  color: rgb(var(--v-theme-error));
  font-size: 0.875rem;
}

.launcher-update-prompt__actions {
  padding: 12px 16px 16px;
}
</style>
