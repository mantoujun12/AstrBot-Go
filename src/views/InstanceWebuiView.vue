<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import EmptyState from "../components/common/EmptyState.vue";
import StatusChip from "../components/common/StatusChip.vue";
import TaskProgressDialog from "../components/common/TaskProgressDialog.vue";
import { useOperation } from "../composables/useOperation";
import { openExternalUrl } from "../services/app";
import { openInstanceCoreFolder, startInstance } from "../services/instances";
import { useLauncherStore } from "../stores/launcher";
import { useProgressStore } from "../stores/progress";
import { useTabsStore } from "../stores/tabs";
import { showErrorToast } from "../stores/toast";
import type { InstanceStatus } from "../types/launcher";

type FrameState = "idle" | "loading" | "ready" | "error";

const props = defineProps<{
  instanceId: string;
}>();

const { instances } = useLauncherStore();
const { isBusy, run } = useOperation();
const { activeTab } = useTabsStore();
const { deploys, activeDeployId, beginDeploy, dismissDeploy } =
  useProgressStore();

const instance = computed<InstanceStatus | null>(
  () => instances.value.find((item) => item.id === props.instanceId) ?? null,
);

const startKey = computed(() => `instance:${props.instanceId}:start`);
const startBusy = computed(() => isBusy(startKey.value));

function normalizeHost(host: string): string {
  const value = host.trim();
  if (
    value === "" ||
    value === "0.0.0.0" ||
    value === "::" ||
    value === "[::]" ||
    value.toLowerCase() === "localhost"
  ) {
    return "127.0.0.1";
  }
  if (value.includes(":") && !(value.startsWith("[") && value.endsWith("]"))) {
    return `[${value}]`;
  }
  return value;
}

function resolveWebuiUrl(target: InstanceStatus): string {
  const host = normalizeHost(target.configured_host);
  const port = target.state === "running" ? target.port : target.configured_port;
  return `http://${host}:${port}/`;
}

const webuiUrl = computed(() =>
  instance.value ? resolveWebuiUrl(instance.value) : "",
);

const showIframe = computed(
  () =>
    instance.value !== null &&
    instance.value.state === "running" &&
    instance.value.dashboard_enabled,
);

const frameState = ref<FrameState>("idle");
const frameKey = ref(0);
let loadTimer: ReturnType<typeof setTimeout> | null = null;

function clearLoadTimer(): void {
  if (loadTimer !== null) {
    clearTimeout(loadTimer);
    loadTimer = null;
  }
}

function startFrameLoad(): void {
  clearLoadTimer();
  frameState.value = "loading";
  loadTimer = setTimeout(() => {
    loadTimer = null;
    if (frameState.value === "loading") {
      frameState.value = "error";
    }
  }, 12000);
}

function handleFrameLoad(): void {
  clearLoadTimer();
  frameState.value = "ready";
}

function reloadFrame(): void {
  frameKey.value += 1;
  startFrameLoad();
}

const activeDeploy = computed(() =>
  activeDeployId.value === props.instanceId
    ? deploys.value[props.instanceId] ?? null
    : null,
);

const deployDialogOpen = computed(
  () =>
    activeTab.value === props.instanceId &&
    activeDeployId.value === props.instanceId,
);

const deployActive = computed(
  () => activeDeployId.value === props.instanceId,
);

function dismissDeployIfActive(instanceId: string): void {
  if (activeDeployId.value === instanceId) {
    dismissDeploy();
  }
}

async function startFromEmpty(): Promise<void> {
  const target = instance.value;
  if (!target || isBusy(startKey.value) || deployActive.value) {
    return;
  }

  beginDeploy(target.id);
  const ok = await run(startKey.value, () => startInstance(target.id), {
    success: "实例已启动",
  });

  if (!ok || !deploys.value[target.id]) {
    dismissDeployIfActive(target.id);
  }
}

async function openCoreFolder(): Promise<void> {
  try {
    await openInstanceCoreFolder(props.instanceId);
  } catch (cause) {
    showErrorToast(cause);
  }
}

async function openInBrowser(): Promise<void> {
  if (!webuiUrl.value) {
    return;
  }
  try {
    await openExternalUrl(webuiUrl.value);
  } catch (cause) {
    showErrorToast(cause);
  }
}

watch(
  () => (showIframe.value ? webuiUrl.value : ""),
  (url) => {
    if (url !== "") {
      startFrameLoad();
    } else {
      clearLoadTimer();
      frameState.value = "idle";
    }
  },
  { immediate: true },
);

watch(activeDeploy, (progress) => {
  if (progress && (progress.step === "done" || progress.step === "error")) {
    dismissDeploy();
  }
});

onBeforeUnmount(() => {
  clearLoadTimer();
});
</script>

<template>
  <div v-if="instance" class="instance-webui">
    <div class="instance-webui__toolbar">
      <StatusChip :state="instance.state" size="x-small" />
      <span class="instance-webui__version">{{ instance.version }}</span>
      <span class="instance-webui__address" :title="webuiUrl">
        {{ webuiUrl }}
      </span>
      <v-spacer />
      <v-btn
        icon="mdi-refresh"
        variant="text"
        size="small"
        aria-label="刷新 WebUI"
        :disabled="!showIframe"
        @click="reloadFrame"
      />
      <v-btn
        icon="mdi-open-in-new"
        variant="text"
        size="small"
        aria-label="在浏览器中打开"
        :disabled="!instance.dashboard_enabled"
        @click="openInBrowser"
      />
      <v-btn
        icon="mdi-folder-open-outline"
        variant="text"
        size="small"
        aria-label="打开核心目录"
        @click="openCoreFolder"
      />
    </div>

    <div class="instance-webui__content">
      <div v-if="showIframe" class="instance-webui__frame-wrap">
        <iframe
          :key="frameKey"
          :src="webuiUrl"
          title="AstrBot WebUI"
          allow="clipboard-write; microphone; autoplay"
          class="instance-webui__frame"
          @load="handleFrameLoad"
        />
        <div v-if="frameState !== 'ready'" class="instance-webui__overlay">
          <template v-if="frameState === 'loading'">
            <v-progress-circular
              indeterminate
              color="primary"
              size="36"
              aria-label="加载中"
            />
          </template>
          <template v-else-if="frameState === 'error'">
            <v-icon
              icon="mdi-alert-circle-outline"
              size="36"
              color="warning"
            />
            <p class="instance-webui__overlay-text">WebUI 加载超时</p>
            <div class="instance-webui__overlay-actions">
              <v-btn
                variant="tonal"
                color="primary"
                prepend-icon="mdi-refresh"
                @click="reloadFrame"
              >
                重试
              </v-btn>
              <v-btn
                variant="text"
                prepend-icon="mdi-open-in-new"
                @click="openInBrowser"
              >
                在浏览器中打开
              </v-btn>
            </div>
          </template>
        </div>
      </div>

      <div v-else class="instance-webui__state">
        <EmptyState
          v-if="!instance.dashboard_enabled"
          text="WebUI 未启用"
          icon="mdi-web-off"
        >
          <v-btn
            variant="tonal"
            prepend-icon="mdi-folder-open-outline"
            @click="openCoreFolder"
          >
            打开核心目录
          </v-btn>
        </EmptyState>

        <EmptyState
          v-else-if="instance.state === 'stopped'"
          text="实例未运行"
          icon="mdi-power"
        >
          <v-btn
            color="primary"
            variant="flat"
            prepend-icon="mdi-play"
            :disabled="deployActive"
            :loading="startBusy"
            @click="startFromEmpty"
          >
            启动
          </v-btn>
        </EmptyState>

        <EmptyState
          v-else
          :text="instance.state === 'starting' ? '启动中…' : '停止中…'"
          icon="mdi-progress-clock"
        >
          <v-progress-circular
            indeterminate
            color="primary"
            size="24"
            width="2"
          />
        </EmptyState>
      </div>
    </div>

    <TaskProgressDialog
      :model-value="deployDialogOpen"
      title="实例部署"
      :message="activeDeploy?.message ?? ''"
      :progress="activeDeploy?.progress ?? null"
    />
  </div>
</template>

<style scoped>
.instance-webui {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.instance-webui__toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  box-sizing: border-box;
  flex: 0 0 48px;
  height: 48px;
  padding: 0 12px;
  background: rgb(var(--v-theme-surface));
}

.instance-webui__version {
  flex: 0 0 auto;
  color: rgba(var(--v-theme-on-surface), 0.72);
  font-size: 0.8rem;
  font-weight: 600;
  white-space: nowrap;
}

.instance-webui__address {
  min-width: 0;
  overflow: hidden;
  color: rgba(var(--v-theme-on-surface), 0.56);
  font-family: "Consolas", "Cascadia Mono", monospace;
  font-size: 0.78rem;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.instance-webui__content {
  position: relative;
  display: flex;
  flex: 1 1 auto;
  min-height: 0;
}

.instance-webui__frame-wrap {
  position: relative;
  flex: 1 1 auto;
  min-height: 0;
}

.instance-webui__frame {
  display: block;
  width: 100%;
  height: 100%;
  border: none;
  background: #ffffff;
}

.instance-webui__overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 24px;
  background: rgb(var(--v-theme-surface));
}

.instance-webui__overlay-text {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.72);
  font-size: 0.9rem;
}

.instance-webui__overlay-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.instance-webui__state {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1 1 auto;
  min-height: 0;
}

.instance-webui__state :deep(.empty-state) {
  width: 100%;
}
</style>
