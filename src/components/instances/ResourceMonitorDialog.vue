<script setup lang="ts">
import type { ChartConfiguration } from "chart.js";
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useTheme } from "vuetify";
import { getCurrentLocale } from "../../i18n";
import {
  getInstanceResourceHistory,
  getResourceAlertThreshold,
  setResourceAlertThreshold,
} from "../../services/resources";
import { showErrorToast, showToast } from "../../stores/toast";
import { useLauncherStore } from "../../stores/launcher";
import type {
  InstanceStatus,
  ResourceAlertThreshold,
  ResourceSample,
} from "../../types/launcher";
import { formatFileSize } from "../../utils/format";
import CanvasChart from "../common/CanvasChart.vue";

const props = defineProps<{
  modelValue: boolean;
  instance: InstanceStatus | null;
}>();

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
}>();

const POLL_INTERVAL_MS = 5_000;
const BYTES_PER_MIB = 1024 * 1024;
const BYTES_PER_GIB = 1024 * 1024 * 1024;
/// A metric only counts as breached after this many consecutive samples.
const CONFIRM_SAMPLES = 2;

type ResourceMetric = "cpu" | "memory" | "disk";

const { t } = useI18n();
const theme = useTheme();
const { instances } = useLauncherStore();

const history = ref<ResourceSample[]>([]);
const appliedThresholds = ref<ResourceAlertThreshold>({
  enabled: true,
  cpu_percent: 90,
  memory_bytes: 4 * 1024 * 1024 * 1024,
  disk_bytes: 20 * 1024 * 1024 * 1024,
});
const alertEnabled = ref(true);
const cpuInput = ref("");
const memoryInput = ref("");
const diskInput = ref("");
const loading = ref(false);
const saving = ref(false);
const thresholdError = ref("");

let pollTimer: ReturnType<typeof setInterval> | null = null;
let requestSequence = 0;
let refreshing = false;
let activeInstanceId = "";

const currentInstance = computed(
  () =>
    instances.value.find((item) => item.id === props.instance?.id) ??
    props.instance,
);

const latest = computed(
  () => history.value[history.value.length - 1] ?? null,
);

const isRunning = computed(
  () => currentInstance.value?.state === "running",
);

const palette = computed(() => {
  const colors = theme.global.current.value.colors as Record<string, string>;
  return {
    primary: colors.primary ?? "#3c96ca",
    info: colors.info ?? "#03c9d7",
    warning: colors.warning ?? "#ffc107",
    error: colors.error ?? "#f44336",
    text: colors["on-surface"] ?? "#1b1c1d",
    muted: `rgba(${colors["on-surface"] ?? "27, 28, 29"}, 0.58)`,
    grid: `rgba(${colors["on-surface"] ?? "27, 28, 29"}, 0.1)`,
  };
});

function timeLabel(timestamp: number): string {
  return new Intl.DateTimeFormat(getCurrentLocale(), {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  }).format(new Date(timestamp * 1000));
}

const labels = computed(() => history.value.map((item) => timeLabel(item.timestamp)));
const cpuValues = computed(() =>
  history.value.map((item) => item.cpu_percent),
);
const memoryValues = computed(() =>
  history.value.map((item) => item.memory_bytes),
);
const diskValues = computed(() =>
  history.value.map((item) => item.disk_bytes),
);

const cpuConfig = computed<ChartConfiguration<"line">>(() => ({
  type: "line",
  data: {
    labels: labels.value,
    datasets: [
      {
        label: t("resources.metrics.cpu"),
        data: cpuValues.value,
        borderColor: palette.value.primary,
        backgroundColor: "rgba(60, 150, 202, 0.14)",
        pointRadius: 0,
        pointHoverRadius: 3,
        borderWidth: 2,
        tension: 0.32,
        fill: true,
        spanGaps: true,
      },
    ],
  },
  options: {
    responsive: true,
    maintainAspectRatio: false,
    animation: false,
    interaction: { intersect: false, mode: "index" },
    plugins: {
      legend: { display: false },
      tooltip: {
        callbacks: {
          label: (context) => {
            const value = context.parsed.y;
            return `${t("resources.metrics.cpu")}: ${
              typeof value === "number" ? value.toFixed(1) : "—"
            }%`;
          },
        },
      },
    },
    scales: {
      x: {
        grid: { display: false },
        ticks: { maxTicksLimit: 6, color: palette.value.muted },
      },
      y: {
        beginAtZero: true,
        suggestedMax: 100,
        ticks: {
          color: palette.value.muted,
          callback: (value) => `${value}%`,
        },
        grid: { color: palette.value.grid },
      },
    },
  },
}));

const storageConfig = computed<ChartConfiguration<"line">>(() => ({
  type: "line",
  data: {
    labels: labels.value,
    datasets: [
      {
        label: t("resources.metrics.memory"),
        data: memoryValues.value,
        borderColor: palette.value.info,
        backgroundColor: "rgba(3, 201, 215, 0.08)",
        pointRadius: 0,
        pointHoverRadius: 3,
        borderWidth: 2,
        tension: 0.28,
        fill: false,
        spanGaps: true,
      },
      {
        label: t("resources.metrics.disk"),
        data: diskValues.value,
        borderColor: palette.value.warning,
        backgroundColor: "rgba(255, 193, 7, 0.08)",
        pointRadius: 0,
        pointHoverRadius: 3,
        borderWidth: 2,
        tension: 0.2,
        fill: false,
        spanGaps: true,
      },
    ],
  },
  options: {
    responsive: true,
    maintainAspectRatio: false,
    animation: false,
    interaction: { intersect: false, mode: "index" },
    plugins: {
      legend: {
        position: "bottom",
        labels: {
          color: palette.value.text,
          usePointStyle: true,
          boxWidth: 6,
        },
      },
      tooltip: {
        callbacks: {
          label: (context) => {
            const value = context.parsed.y;
            return `${context.dataset.label}: ${
              typeof value === "number" ? formatFileSize(value) : "—"
            }`;
          },
        },
      },
    },
    scales: {
      x: {
        grid: { display: false },
        ticks: { maxTicksLimit: 6, color: palette.value.muted },
      },
      y: {
        beginAtZero: true,
        ticks: {
          color: palette.value.muted,
          callback: (value) => formatFileSize(Number(value)),
        },
        grid: { color: palette.value.grid },
      },
    },
  },
}));

const cpuDisplay = computed(() => {
  const value = latest.value?.cpu_percent;
  return typeof value === "number" ? `${value.toFixed(1)}%` : "—";
});
const memoryDisplay = computed(() => formatFileSize(latest.value?.memory_bytes ?? 0));
const diskDisplay = computed(() => formatFileSize(latest.value?.disk_bytes ?? 0));

/** A metric is only reported once it stayed above the threshold. */
function sustainedBreach(
  read: (sample: ResourceSample) => number | null,
  threshold: number,
): boolean {
  const samples = history.value.slice(-CONFIRM_SAMPLES);
  if (samples.length < CONFIRM_SAMPLES) {
    return false;
  }
  return samples.every((item) => {
    const value = read(item);
    return value !== null && value > threshold;
  });
}

const breachedMetrics = computed(() => {
  const thresholds = appliedThresholds.value;
  const metrics: ResourceMetric[] = [];
  if (!thresholds.enabled) {
    return metrics;
  }
  if (sustainedBreach((item) => item.cpu_percent, thresholds.cpu_percent)) {
    metrics.push("cpu");
  }
  if (sustainedBreach((item) => item.memory_bytes, thresholds.memory_bytes)) {
    metrics.push("memory");
  }
  if (sustainedBreach((item) => item.disk_bytes, thresholds.disk_bytes)) {
    metrics.push("disk");
  }
  return metrics;
});

const breachedMessage = computed(() =>
  t("resources.alerts.current", {
    metrics: breachedMetrics.value
      .map((metric) => t(`resources.metrics.${metric}`))
      .join(t("resources.alerts.separator")),
  }),
);

function isMetricBreached(metric: ResourceMetric): boolean {
  return breachedMetrics.value.includes(metric);
}

function stopPolling(): void {
  if (pollTimer !== null) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

async function refreshHistory(): Promise<void> {
  const id = props.instance?.id;
  if (!id || refreshing) {
    return;
  }
  refreshing = true;
  const sequence = ++requestSequence;
  try {
    const next = await getInstanceResourceHistory(id);
    if (sequence === requestSequence && id === activeInstanceId) {
      history.value = next;
    }
  } catch {
    if (sequence === requestSequence) {
      stopPolling();
    }
  } finally {
    refreshing = false;
  }
}

function startPolling(): void {
  stopPolling();
  if (!isRunning.value) {
    return;
  }
  pollTimer = setInterval(() => {
    void refreshHistory();
  }, POLL_INTERVAL_MS);
}

function parseOptionalNumber(raw: string | null | undefined): number | undefined {
  const value = (raw ?? "").trim();
  const parsed = Number(value);
  return value !== "" && Number.isFinite(parsed) && parsed > 0
    ? parsed
    : undefined;
}

function readFormValues(): {
  cpu: number;
  memory: number;
  disk: number;
} | null {
  const cpu = parseOptionalNumber(cpuInput.value);
  const memoryMiB = parseOptionalNumber(memoryInput.value);
  const diskGiB = parseOptionalNumber(diskInput.value);
  if (cpu === undefined || memoryMiB === undefined || diskGiB === undefined) {
    return null;
  }

  return {
    cpu,
    memory: Math.round(memoryMiB * BYTES_PER_MIB),
    disk: Math.round(diskGiB * BYTES_PER_GIB),
  };
}

function syncFormFromThresholds(value: ResourceAlertThreshold): void {
  alertEnabled.value = value.enabled;
  cpuInput.value = String(value.cpu_percent);
  memoryInput.value = String(
    Number((value.memory_bytes / BYTES_PER_MIB).toFixed(2)),
  );
  diskInput.value = String(
    Number((value.disk_bytes / BYTES_PER_GIB).toFixed(2)),
  );
}

async function initialize(): Promise<void> {
  const instance = props.instance;
  if (!instance) {
    return;
  }

  activeInstanceId = instance.id;
  history.value = [];
  thresholdError.value = "";
  loading.value = true;

  try {
    const [nextHistory, nextThresholds] = await Promise.all([
      getInstanceResourceHistory(instance.id),
      getResourceAlertThreshold(instance.id),
    ]);
    if (activeInstanceId !== instance.id) {
      return;
    }
    history.value = nextHistory;
    appliedThresholds.value = nextThresholds;
    syncFormFromThresholds(nextThresholds);
    startPolling();
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    // A poll that starts while this load is in flight must not strand the
    // dialog on its skeleton, so the spinner follows the dialog, not the request.
    if (activeInstanceId === instance.id) {
      loading.value = false;
    }
  }
}

async function saveThresholds(): Promise<void> {
  const instance = props.instance;
  if (!instance || saving.value) {
    return;
  }
  const values = readFormValues();
  if (!values && alertEnabled.value) {
    thresholdError.value = t("resources.thresholds.invalid");
    return;
  }

  thresholdError.value = "";
  saving.value = true;
  const current = appliedThresholds.value;
  try {
    await setResourceAlertThreshold(instance.id, {
      enabled: alertEnabled.value,
      cpu_percent: values?.cpu ?? current.cpu_percent,
      memory_bytes: values?.memory ?? current.memory_bytes,
      disk_bytes: values?.disk ?? current.disk_bytes,
    });
    const stored = await getResourceAlertThreshold(instance.id);
    appliedThresholds.value = stored;
    syncFormFromThresholds(stored);
    showToast(t("resources.thresholds.saved"));
  } catch (cause) {
    showErrorToast(cause);
  } finally {
    saving.value = false;
  }
}

function close(): void {
  stopPolling();
  emit("update:modelValue", false);
}

watch(
  () => [props.modelValue, props.instance?.id] as const,
  ([open, id], previous) => {
    if (!open) {
      stopPolling();
      activeInstanceId = "";
      return;
    }
    if (id && (previous?.[0] !== true || previous?.[1] !== id)) {
      void initialize();
    }
  },
);

watch(
  () => isRunning.value,
  (running) => {
    if (!props.modelValue) {
      return;
    }
    if (running) {
      void refreshHistory();
      startPolling();
    } else {
      stopPolling();
    }
  },
);

onBeforeUnmount(stopPolling);
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    max-width="1080"
    scrollable
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card class="resource-dialog">
      <v-card-title class="resource-dialog__title">
        <div class="resource-dialog__heading">
          <span>{{ t("resources.title") }}</span>
          <span class="resource-dialog__name">{{ instance?.name }}</span>
        </div>
        <v-btn
          icon="mdi-close"
          variant="text"
          size="small"
          :aria-label="t('common.actions.close')"
          @click="close"
        />
      </v-card-title>

      <v-card-text class="resource-dialog__body">
        <v-alert
          v-if="!isRunning"
          type="info"
          variant="tonal"
          density="compact"
          class="resource-dialog__alert"
        >
          {{ t("resources.stoppedHint") }}
        </v-alert>
        <v-alert
          v-else-if="breachedMetrics.length > 0"
          type="warning"
          variant="tonal"
          density="compact"
          class="resource-dialog__alert"
        >
          {{ breachedMessage }}
        </v-alert>

        <div class="resource-metrics">
          <div
            class="resource-metric"
            :class="{ 'resource-metric--alert': isMetricBreached('cpu') }"
          >
            <div class="resource-metric__label">
              {{ t("resources.metrics.cpu") }}
            </div>
            <div class="resource-metric__value">{{ cpuDisplay }}</div>
          </div>
          <div
            class="resource-metric"
            :class="{ 'resource-metric--alert': isMetricBreached('memory') }"
          >
            <div class="resource-metric__label">
              {{ t("resources.metrics.memory") }}
            </div>
            <div class="resource-metric__value">{{ memoryDisplay }}</div>
          </div>
          <div
            class="resource-metric"
            :class="{ 'resource-metric--alert': isMetricBreached('disk') }"
          >
            <div class="resource-metric__label">
              {{ t("resources.metrics.disk") }}
            </div>
            <div class="resource-metric__value">{{ diskDisplay }}</div>
          </div>
        </div>

        <div class="resource-charts">
          <section class="resource-panel">
            <h3 class="resource-panel__title">
              {{ t("resources.charts.cpu") }}
            </h3>
            <div v-if="loading" class="resource-panel__skeleton" />
            <CanvasChart v-else-if="history.length > 0" :config="cpuConfig" :height="220" />
            <div v-else class="resource-panel__empty">
              {{ t("resources.charts.empty") }}
            </div>
          </section>

          <section class="resource-panel">
            <h3 class="resource-panel__title">
              {{ t("resources.charts.storage") }}
            </h3>
            <div v-if="loading" class="resource-panel__skeleton" />
            <CanvasChart
              v-else-if="history.length > 0"
              :config="storageConfig"
              :height="220"
            />
            <div v-else class="resource-panel__empty">
              {{ t("resources.charts.empty") }}
            </div>
          </section>
        </div>

        <section class="resource-panel resource-thresholds">
          <h3 class="resource-panel__title">
            {{ t("resources.thresholds.title") }}
          </h3>
          <v-switch
            v-model="alertEnabled"
            color="primary"
            density="compact"
            hide-details
            class="resource-thresholds__toggle"
            :label="t('resources.thresholds.enabled')"
          />
          <div class="resource-thresholds__fields">
            <v-text-field
              v-model="cpuInput"
              :disabled="!alertEnabled"
              type="number"
              min="1"
              step="1"
              density="compact"
              variant="outlined"
              :label="t('resources.thresholds.cpu')"
              :suffix="t('resources.units.percent')"
              :error-messages="thresholdError"
            />
            <v-text-field
              v-model="memoryInput"
              type="number"
              min="1"
              step="1"
              density="compact"
              variant="outlined"
              :disabled="!alertEnabled"
              :label="t('resources.thresholds.memory')"
              :suffix="t('resources.units.mib')"
              :error-messages="thresholdError"
            />
            <v-text-field
              v-model="diskInput"
              type="number"
              min="1"
              step="1"
              density="compact"
              variant="outlined"
              :disabled="!alertEnabled"
              :label="t('resources.thresholds.disk')"
              :suffix="t('resources.units.gib')"
              :error-messages="thresholdError"
            />
          </div>
        </section>
      </v-card-text>

      <v-card-actions class="resource-dialog__actions">
        <v-spacer />
        <v-btn variant="text" @click="close">
          {{ t("common.actions.close") }}
        </v-btn>
        <v-btn
          color="primary"
          variant="flat"
          :loading="saving"
          @click="saveThresholds"
        >
          {{ t("common.actions.save") }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.resource-dialog {
  overflow: hidden;
}

.resource-dialog__title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 20px 20px 12px;
}

.resource-dialog__heading {
  display: flex;
  align-items: baseline;
  gap: 10px;
  min-width: 0;
  font-size: 1.05rem;
  font-weight: 600;
}

.resource-dialog__name {
  overflow: hidden;
  color: rgba(var(--v-theme-on-surface), 0.6);
  font-size: 0.86rem;
  font-weight: 500;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.resource-dialog__body {
  display: grid;
  grid-auto-rows: max-content;
  gap: 16px;
  padding: 0 20px 4px;
}

.resource-dialog__alert {
  margin: 0;
}

.resource-metrics {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
}

.resource-metric,
.resource-panel {
  border: 1px solid rgba(var(--v-theme-on-surface), 0.08);
  border-radius: 14px;
  background: rgba(var(--v-theme-surface-variant), 0.2);
}

.resource-metric {
  padding: 14px 16px;
}

.resource-metric--alert {
  border-color: rgba(var(--v-theme-warning), 0.5);
  background: rgba(var(--v-theme-warning), 0.08);
}

.resource-metric--alert .resource-metric__value {
  color: rgb(var(--v-theme-warning));
}

.resource-metric__label {
  color: rgba(var(--v-theme-on-surface), 0.6);
  font-size: 0.78rem;
  font-weight: 500;
}

.resource-metric__value {
  margin-top: 4px;
  font-size: 1.35rem;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.resource-charts {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.resource-panel {
  min-width: 0;
  padding: 16px;
}

.resource-panel__title {
  margin: 0 0 12px;
  font-size: 0.88rem;
  font-weight: 600;
}

.resource-panel__skeleton {
  height: 220px;
  border-radius: 10px;
  background: rgba(var(--v-theme-on-surface), 0.06);
  animation: resource-pulse 1.4s ease-in-out infinite;
}

.resource-panel__empty {
  display: grid;
  place-items: center;
  height: 220px;
  color: rgba(var(--v-theme-on-surface), 0.52);
  font-size: 0.84rem;
}

.resource-thresholds__fields {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
}

.resource-dialog__actions {
  padding: 14px 20px 16px;
}

@keyframes resource-pulse {
  0%,
  100% {
    opacity: 1;
  }

  50% {
    opacity: 0.5;
  }
}

@media (max-width: 840px) {
  .resource-charts,
  .resource-thresholds__fields {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 560px) {
  .resource-metrics {
    grid-template-columns: 1fr;
  }

  .resource-dialog__heading {
    align-items: flex-start;
    flex-direction: column;
    gap: 2px;
  }
}
</style>
