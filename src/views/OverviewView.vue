<script setup lang="ts">
import type { ChartConfiguration } from "chart.js";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useRouter } from "vue-router";
import { useDisplay, useTheme } from "vuetify";
import CanvasChart from "../components/common/CanvasChart.vue";
import EmptyState from "../components/common/EmptyState.vue";
import { getCurrentLocale } from "../i18n";
import { useLauncherStore } from "../stores/launcher";
import type {
  BackupInfo,
  InstanceState,
  InstanceStatus,
} from "../types/launcher";
import {
  formatDateTime,
  instanceStateColor,
  instanceStateLabel,
} from "../utils/format";

type BackupRange = 6 | 12;

interface StatCard {
  key: string;
  label: string;
  value: string;
  icon: string;
}

interface MonthBucket {
  key: string;
  label: string;
  count: number;
}

interface CompositionItem {
  key: string;
  label: string;
  count: number;
  color: string;
}

const CHART_FONT_FAMILY = '"Outfit Variable", "Outfit", "Noto Sans", sans-serif';
const COMPONENT_TOTAL = 3;
const STATE_ORDER: readonly InstanceState[] = [
  "running",
  "starting",
  "stopping",
  "stopped",
];

const router = useRouter();
const theme = useTheme();
const { t } = useI18n();
const { smAndDown } = useDisplay();
const {
  snapshot,
  instances,
  versions,
  backups,
  components,
  loading,
  error,
  refresh,
} = useLauncherStore();

const backupRange = ref<BackupRange>(6);
const backupRangeOptions = computed<
  ReadonlyArray<{ value: BackupRange; label: string }>
>(() => [
  { value: 6, label: t("overview.backups.rangeMonths", { count: 6 }) },
  { value: 12, label: t("overview.backups.rangeMonths", { count: 12 }) },
]);

const isDark = computed(() => theme.global.current.value.dark);

const themePalette = computed(() => {
  const colors = theme.global.current.value.colors as Record<string, string>;
  return {
    primary: colors.primary ?? "#3c96ca",
    secondary: colors.secondary ?? "#2f86bd",
    success: colors.success ?? "#00a850",
    info: colors.info ?? "#03c9d7",
    warning: colors.warning ?? "#ffc107",
    error: colors.error ?? "#f44336",
    border: colors.border ?? "#d0d0d0",
    text: colors["on-surface"] ?? "#1b1c1d",
  };
});

const donutHeight = computed(() => (smAndDown.value ? 240 : 300));
const barHeight = computed(() => (smAndDown.value ? 210 : 260));

const stateCounts = computed<Record<InstanceState, number>>(() => {
  const counts: Record<InstanceState, number> = {
    running: 0,
    starting: 0,
    stopping: 0,
    stopped: 0,
  };
  for (const instance of instances.value) {
    counts[instance.state] += 1;
  }
  return counts;
});

const installedComponentCount = computed(
  () => components.value.filter((component) => component.installed).length,
);

const statCards = computed<StatCard[]>(() => [
  {
    key: "instances",
    label: t("overview.stats.instances"),
    value: String(instances.value.length),
    icon: "mdi-robot-outline",
  },
  {
    key: "running",
    label: t("overview.stats.running"),
    value: String(stateCounts.value.running),
    icon: "mdi-play-circle-outline",
  },
  {
    key: "versions",
    label: t("overview.stats.versions"),
    value: String(versions.value.length),
    icon: "mdi-package-variant-closed",
  },
  {
    key: "backups",
    label: t("overview.stats.backups"),
    value: String(backups.value.length),
    icon: "mdi-archive-outline",
  },
  {
    key: "components",
    label: t("overview.stats.components"),
    value: `${installedComponentCount.value} / ${COMPONENT_TOTAL}`,
    icon: "mdi-puzzle-outline",
  },
]);

function monthKey(date: Date): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}`;
}

function parseBackupDate(backup: BackupInfo): Date | null {
  const raw = backup.metadata.created_at.trim();
  if (raw === "") {
    return null;
  }
  const date = new Date(raw);
  return Number.isNaN(date.getTime()) ? null : date;
}

const backupMonths = computed<MonthBucket[]>(() => {
  const now = new Date();
  const monthFormatter = new Intl.DateTimeFormat(getCurrentLocale(), {
    month: "short",
  });
  const buckets: MonthBucket[] = [];
  const positions = new Map<string, number>();

  for (let offset = backupRange.value - 1; offset >= 0; offset -= 1) {
    const date = new Date(now.getFullYear(), now.getMonth() - offset, 1);
    const key = monthKey(date);
    positions.set(key, buckets.length);
    buckets.push({
      key,
      label: monthFormatter.format(date),
      count: 0,
    });
  }

  for (const backup of backups.value) {
    const date = parseBackupDate(backup);
    if (!date) {
      continue;
    }
    const position = positions.get(monthKey(date));
    if (position === undefined) {
      continue;
    }
    const bucket = buckets[position];
    if (bucket) {
      bucket.count += 1;
    }
  }

  return buckets;
});

const backupMonthLabels = computed(() =>
  backupMonths.value.map((bucket) => bucket.label),
);

const backupTotalInRange = computed(() =>
  backupMonths.value.reduce((total, bucket) => total + bucket.count, 0),
);

const backupCounts = computed(() =>
  backupMonths.value.map((bucket) => bucket.count),
);

const latestBackup = computed<BackupInfo | null>(() => {
  let latest: BackupInfo | null = null;
  let latestTime = Number.NEGATIVE_INFINITY;

  for (const backup of backups.value) {
    const date = parseBackupDate(backup);
    if (!date) {
      continue;
    }
    const time = date.getTime();
    if (time > latestTime) {
      latest = backup;
      latestTime = time;
    }
  }

  return latest;
});

const latestBackupName = computed(() => latestBackup.value?.filename ?? "");

const latestBackupDate = computed(() => {
  const backup = latestBackup.value;
  return backup ? formatDateTime(backup.metadata.created_at) : "";
});

const backupComposition = computed<CompositionItem[]>(() => {
  let manual = 0;
  let auto = 0;
  let corrupted = 0;

  for (const backup of backups.value) {
    if (backup.corrupted) {
      corrupted += 1;
    } else if (backup.metadata.auto_generated) {
      auto += 1;
    } else {
      manual += 1;
    }
  }

  return [
    {
      key: "manual",
      label: t("overview.backups.composition.manual"),
      count: manual,
      color: themePalette.value.primary,
    },
    {
      key: "auto",
      label: t("overview.backups.composition.auto"),
      count: auto,
      color: themePalette.value.success,
    },
    {
      key: "corrupted",
      label: t("overview.backups.composition.corrupted"),
      count: corrupted,
      color: themePalette.value.error,
    },
  ];
});

const stateLabels = computed(() =>
  STATE_ORDER.map((state) => instanceStateLabel(state)),
);

const donutSeries = computed(() =>
  STATE_ORDER.map((state) => stateCounts.value[state]),
);

const donutAriaLabel = computed(() => {
  const counts = stateCounts.value;
  return t("overview.instances.donutAria", {
    running: instanceStateLabel("running"),
    runningCount: counts.running,
    starting: instanceStateLabel("starting"),
    startingCount: counts.starting,
    stopping: instanceStateLabel("stopping"),
    stoppingCount: counts.stopping,
    stopped: instanceStateLabel("stopped"),
    stoppedCount: counts.stopped,
  });
});

const backupChartAriaLabel = computed(() =>
  t("overview.backups.chartAria", {
    range: backupRange.value,
    total: backupTotalInRange.value,
  }),
);

function withAlpha(color: string, alpha: number): string {
  if (!color.startsWith("#")) {
    return color;
  }

  let hex = color.slice(1);
  if (hex.length === 3) {
    hex = hex
      .split("")
      .map((char) => char + char)
      .join("");
  } else if (hex.length === 8) {
    hex = hex.slice(0, 6);
  }

  if (hex.length !== 6) {
    return color;
  }

  const red = Number.parseInt(hex.slice(0, 2), 16);
  const green = Number.parseInt(hex.slice(2, 4), 16);
  const blue = Number.parseInt(hex.slice(4, 6), 16);
  if (Number.isNaN(red) || Number.isNaN(green) || Number.isNaN(blue)) {
    return color;
  }

  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}

function chartTooltipStyle() {
  const palette = themePalette.value;

  return {
    backgroundColor: isDark.value ? "#242424" : "#ffffff",
    titleColor: palette.text,
    bodyColor: palette.text,
    borderColor: withAlpha(palette.border, isDark.value ? 0.55 : 0.5),
    borderWidth: 1,
    padding: 10,
    cornerRadius: 8,
    displayColors: true,
    boxWidth: 7,
    boxHeight: 7,
    boxPadding: 4,
    titleFont: { family: CHART_FONT_FAMILY, size: 12, weight: 600 },
    bodyFont: { family: CHART_FONT_FAMILY, size: 12 },
  };
}

const donutConfig = computed<ChartConfiguration<"doughnut", number[], string>>(
  () => {
    const palette = themePalette.value;
    const mutedText = withAlpha(palette.text, 0.62);

    return {
      type: "doughnut",
      data: {
        labels: stateLabels.value,
        datasets: [
          {
            data: donutSeries.value,
            backgroundColor: [
              palette.success,
              palette.info,
              palette.warning,
              palette.secondary,
            ],
            borderWidth: 0,
            hoverBorderWidth: 0,
            spacing: 0,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        cutout: "72%",
        animation: { duration: 350 },
        font: { family: CHART_FONT_FAMILY, size: 12 },
        color: mutedText,
        plugins: {
          legend: {
            display: true,
            position: "bottom",
            labels: {
              color: mutedText,
              usePointStyle: true,
              pointStyle: "circle",
              boxWidth: 7,
              boxHeight: 7,
              padding: 12,
              font: { family: CHART_FONT_FAMILY, size: 12 },
            },
          },
          tooltip: chartTooltipStyle(),
        },
      },
    };
  },
);

const backupConfig = computed<ChartConfiguration<"bar", number[], string>>(
  () => {
    const palette = themePalette.value;
    const mutedText = withAlpha(palette.text, 0.62);

    return {
      type: "bar",
      data: {
        labels: backupMonthLabels.value,
        datasets: [
          {
            label: t("overview.backups.series"),
            data: backupCounts.value,
            backgroundColor: palette.primary,
            hoverBackgroundColor: palette.primary,
            borderRadius: 6,
            borderSkipped: "start",
            categoryPercentage: 0.46,
            barPercentage: 1,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        animation: { duration: 350 },
        font: { family: CHART_FONT_FAMILY, size: 12 },
        color: mutedText,
        layout: { padding: { top: 8 } },
        scales: {
          x: {
            grid: { display: false },
            border: { display: false },
            ticks: {
              color: mutedText,
              maxRotation: 0,
              font: { family: CHART_FONT_FAMILY, size: 12 },
            },
          },
          y: {
            beginAtZero: true,
            grid: {
              color: withAlpha(
                palette.border,
                isDark.value ? 0.4 : 0.26,
              ),
              drawTicks: false,
            },
            border: { display: false },
            ticks: {
              color: mutedText,
              precision: 0,
              padding: 8,
              font: { family: CHART_FONT_FAMILY, size: 12 },
            },
          },
        },
        plugins: {
          legend: { display: false },
          tooltip: {
            ...chartTooltipStyle(),
            callbacks: {
              label: (context) => {
                const value = context.parsed.y;
                return t("overview.backups.tooltip", {
                  label:
                    context.dataset.label ?? t("overview.backups.series"),
                  value,
                });
              },
            },
          },
        },
      },
    };
  },
);

function stateDotColor(state: InstanceState): string {
  return `rgb(var(--v-theme-${instanceStateColor(state)}))`;
}

function instanceRowLabel(instance: InstanceStatus): string {
  return t("overview.instances.rowLabel", {
    name: instance.name,
    state: instanceStateLabel(instance.state),
    version: instance.version || t("overview.instances.unknownVersion"),
    port: instance.port,
  });
}

function goToInstances(): void {
  void router.push("/instances");
}

function retry(): void {
  void refresh();
}
</script>

<template>
  <div class="page-shell overview-page" :class="{ 'is-dark': isDark }">
    <v-alert
      v-if="error"
      type="error"
      variant="tonal"
      class="overview-alert page-alert"
      role="alert"
    >
      <div class="alert-content page-row--between">
        <span>{{ error }}</span>
        <v-btn
          color="error"
          size="small"
          variant="text"
          :loading="loading"
          :disabled="loading"
          @click="retry"
        >
          {{ t("common.actions.retry") }}
        </v-btn>
      </div>
    </v-alert>

    <div
      v-if="snapshot === null"
      class="overview-loading"
      role="status"
      :aria-label="t('overview.loading')"
    >
      <div class="stat-grid" aria-hidden="true">
        <div
          v-for="index in 5"
          :key="`stat-skeleton-${index}`"
          class="skeleton skeleton-stat"
        />
      </div>
      <div class="skeleton-panels" aria-hidden="true">
        <div class="skeleton skeleton-panel" />
        <div class="skeleton skeleton-panel" />
      </div>
    </div>

    <template v-else>
      <div class="stat-grid">
        <section
          v-for="card in statCards"
          :key="card.key"
          class="stat-card"
          :aria-label="
            t('overview.stats.ariaLabel', {
              label: card.label,
              value: card.value,
            })
          "
        >
          <v-icon
            :icon="card.icon"
            size="88"
            class="stat-icon"
            aria-hidden="true"
          />
          <div class="stat-label">{{ card.label }}</div>
          <div class="stat-value page-nums">{{ card.value }}</div>
        </section>
      </div>

      <section class="page-section" aria-labelledby="overview-instances-title">
        <div class="page-section-head overview-section-head">
          <h2 id="overview-instances-title" class="section-title">
            {{ t("overview.sections.instances") }}
          </h2>
        </div>

        <div class="page-grid-2 instance-grid">
          <div class="panel-card chart-card">
            <EmptyState
              v-if="instances.length === 0"
              :text="t('overview.empty.instances')"
              icon="mdi-robot-off-outline"
            />
            <div
              v-else
              class="chart-wrap"
              role="img"
              :aria-label="donutAriaLabel"
            >
              <CanvasChart :config="donutConfig" :height="donutHeight" />
            </div>
          </div>

          <section
            class="panel-card list-card"
            :aria-label="t('overview.instances.listLabel')"
          >
            <EmptyState
              v-if="instances.length === 0"
              :text="t('overview.empty.instances')"
              icon="mdi-robot-off-outline"
            />
            <TransitionGroup
              v-else
              name="motion-list"
              tag="div"
              class="instance-list"
            >
              <button
                v-for="instance in instances"
                :key="instance.id"
                type="button"
                class="instance-row"
                :aria-label="instanceRowLabel(instance)"
                @click="goToInstances"
              >
                <span
                  class="instance-dot"
                  :style="{ backgroundColor: stateDotColor(instance.state) }"
                  aria-hidden="true"
                />
                <span class="instance-main">
                  <span class="instance-name">{{ instance.name }}</span>
                  <span class="instance-meta">
                    {{ instanceStateLabel(instance.state) }} ·
                    {{ instance.version || "—" }} ·
                    {{
                      t("overview.instances.port", { port: instance.port })
                    }}
                  </span>
                </span>
                <v-icon
                  icon="mdi-chevron-right"
                  size="18"
                  class="instance-chevron"
                  aria-hidden="true"
                />
              </button>
            </TransitionGroup>
          </section>
        </div>
      </section>

      <section class="page-section" aria-labelledby="overview-backups-title">
        <div class="page-section-head overview-section-head">
          <h2 id="overview-backups-title" class="section-title">
            {{ t("overview.sections.backups") }}
          </h2>
          <div
            class="range-switch"
            role="group"
            :aria-label="t('overview.backups.rangeLabel')"
          >
            <button
              v-for="option in backupRangeOptions"
              :key="option.value"
              type="button"
              class="range-chip"
              :class="{ active: backupRange === option.value }"
              :aria-pressed="backupRange === option.value"
              @click="backupRange = option.value"
            >
              {{ option.label }}
            </button>
          </div>
        </div>

        <div class="page-grid-2">
          <div class="panel-card chart-card">
            <EmptyState
              v-if="backups.length === 0"
              :text="t('overview.empty.backups')"
              icon="mdi-archive-off-outline"
            />
            <div
              v-else
              class="chart-wrap"
              role="img"
              :aria-label="backupChartAriaLabel"
            >
              <CanvasChart :config="backupConfig" :height="barHeight" />
            </div>
          </div>

          <div class="overview-side">
            <section
              class="panel-card info-card"
              aria-labelledby="overview-composition-title"
            >
              <h3 id="overview-composition-title" class="card-title">
                {{ t("overview.backups.composition.title") }}
              </h3>
              <div class="composition-list">
                <div
                  v-for="item in backupComposition"
                  :key="item.key"
                  class="composition-row"
                >
                  <span class="composition-label">
                    <span
                      class="composition-dot"
                      :style="{ backgroundColor: item.color }"
                      aria-hidden="true"
                    />
                    {{ item.label }}
                  </span>
                  <strong class="page-nums">{{ item.count }}</strong>
                </div>
              </div>

              <div class="recent-backup">
                <div class="card-subtitle">
                  {{ t("overview.backups.recentTitle") }}
                </div>
                <template v-if="latestBackupName">
                  <div class="recent-name" :title="latestBackupName">
                    {{ latestBackupName }}
                  </div>
                  <div class="recent-date">{{ latestBackupDate }}</div>
                </template>
                <div v-else class="page-muted recent-empty">
                  {{ t("overview.empty.backups") }}
                </div>
              </div>
            </section>

            <section
              class="panel-card info-card"
              aria-labelledby="overview-components-title"
            >
              <h3 id="overview-components-title" class="card-title">
                {{ t("overview.components.title") }}
              </h3>
              <EmptyState
                v-if="components.length === 0"
                :text="t('overview.empty.components')"
                icon="mdi-puzzle-outline"
              />
              <div v-else class="component-list">
                <div
                  v-for="component in components"
                  :key="component.id"
                  class="component-row"
                >
                  <span class="component-name" :title="component.display_name">
                    {{ component.display_name }}
                  </span>
                  <v-chip
                    :color="component.installed ? 'success' : 'secondary'"
                    size="x-small"
                    variant="tonal"
                    label
                  >
                    {{
                      component.installed
                        ? t("overview.components.installed")
                        : t("overview.components.notInstalled")
                    }}
                  </v-chip>
                </div>
              </div>
            </section>
          </div>
        </div>
      </section>
    </template>
  </div>
</template>

<style scoped>
.overview-page {
  /* 白色卡片在浅灰染色层上形成层次差，边框兜住低对比环境。 */
  --overview-card: #ffffff;
  --overview-muted: rgba(var(--v-theme-on-surface), 0.58);
  --overview-border: rgba(var(--v-theme-on-surface), 0.1);
  color: rgb(var(--v-theme-on-surface));
}

.overview-page.is-dark {
  --overview-card: rgba(var(--v-theme-on-surface), 0.06);
  --overview-border: rgba(var(--v-theme-on-surface), 0.14);
}

.overview-alert {
  border-radius: var(--astrbot-radius-control);
}

.alert-content {
  gap: 12px;
  width: 100%;
}

.stat-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 16px;
  margin-bottom: 20px;
}

.stat-card {
  position: relative;
  overflow: hidden;
  padding: 20px 20px 18px;
  border: 1px solid var(--overview-border);
  border-radius: var(--astrbot-radius-card);
  background: var(--overview-card);
}

.stat-icon {
  position: absolute;
  right: -18px;
  bottom: -20px;
  color: rgb(var(--v-theme-primary));
  opacity: 0.09;
  pointer-events: none;
}

.stat-label {
  position: relative;
  z-index: 1;
  color: var(--overview-muted);
  font-size: 13px;
  font-weight: 500;
}

.stat-value {
  position: relative;
  z-index: 1;
  margin-top: 8px;
  font-size: clamp(24px, 2vw, 34px);
  font-weight: 700;
  line-height: 1.1;
  letter-spacing: -0.03em;
  white-space: nowrap;
}

.overview-section-head {
  align-items: center;
}

.section-title {
  margin: 0;
  font-size: 19px;
  font-weight: 600;
  letter-spacing: -0.02em;
  line-height: 1.3;
}

.range-switch {
  display: inline-flex;
  gap: 8px;
  padding: 5px;
  border: 1px solid var(--overview-border);
  border-radius: var(--astrbot-radius-pill);
  background: rgb(var(--v-theme-surface));
}

.range-chip {
  padding: 7px 14px;
  border: 0;
  border-radius: var(--astrbot-radius-pill);
  background: transparent;
  color: var(--overview-muted);
  font-family: inherit;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition:
    background-color 0.18s ease,
    color 0.18s ease;
}

.range-chip:hover {
  background: rgba(var(--v-theme-on-surface), 0.05);
}

.range-chip.active {
  background: rgba(var(--v-theme-primary), 0.1);
  color: rgb(var(--v-theme-primary));
}

.overview-page.is-dark .range-chip.active {
  background: rgba(var(--v-theme-primary), 0.18);
}

.range-chip:focus-visible {
  outline: 2px solid rgb(var(--v-theme-primary));
  outline-offset: 2px;
}

.panel-card {
  min-width: 0;
  padding: 22px;
  border: 1px solid var(--overview-border);
  border-radius: var(--astrbot-radius-card);
  background: var(--overview-card);
}

.chart-card {
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.chart-wrap {
  width: 100%;
  min-width: 0;
}

.instance-grid {
  align-items: start;
}

.list-card {
  display: flex;
  flex-direction: column;
}

.instance-list {
  display: grid;
  gap: 2px;
}

.instance-row {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 12px 10px;
  border: 0;
  border-radius: var(--astrbot-radius-control);
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: start;
  cursor: pointer;
  transition: background-color 0.15s ease;
}

.instance-row:hover {
  background: rgba(var(--v-theme-on-surface), 0.05);
}

.overview-page.is-dark .instance-row:hover {
  background: rgba(var(--v-theme-on-surface), 0.08);
}

.instance-row:focus-visible {
  outline: 2px solid rgb(var(--v-theme-primary));
  outline-offset: 1px;
}

.instance-dot {
  flex: none;
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.instance-main {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.instance-name {
  overflow: hidden;
  font-size: 14px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.instance-meta {
  overflow: hidden;
  color: var(--overview-muted);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.instance-chevron {
  flex: none;
  color: var(--overview-muted);
}

.overview-side {
  display: grid;
  align-content: start;
  gap: 20px;
  min-width: 0;
}

.info-card {
  display: flex;
  flex-direction: column;
}

.card-title {
  margin: 0 0 12px;
  font-size: 15px;
  font-weight: 600;
  letter-spacing: -0.01em;
}

.composition-list {
  display: grid;
}

.composition-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 0;
  border-bottom: 1px solid var(--overview-border);
}

.composition-row strong {
  font-size: 14px;
}

.composition-label {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  color: var(--overview-muted);
  font-size: 13px;
}

.composition-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}

.recent-backup {
  margin-top: 16px;
  padding-top: 14px;
  border-top: 1px solid var(--overview-border);
}

.card-subtitle {
  color: var(--overview-muted);
  font-size: 12px;
  font-weight: 500;
}

.recent-name {
  margin-top: 8px;
  overflow: hidden;
  font-size: 13px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recent-date {
  margin-top: 4px;
  color: var(--overview-muted);
  font-size: 12px;
}

.recent-empty {
  margin-top: 8px;
}

.component-list {
  display: grid;
}

.component-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 8px 0;
  border-bottom: 1px solid var(--overview-border);
}

.component-row:last-child {
  border-bottom: 0;
}

.component-name {
  overflow: hidden;
  font-size: 14px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.overview-loading {
  display: grid;
  gap: 20px;
}

.skeleton {
  border-radius: var(--astrbot-radius-card);
  background: var(--overview-card);
  animation: overview-pulse 1.5s ease-in-out infinite;
}

.skeleton-stat {
  height: 118px;
}

.skeleton-panels {
  display: grid;
  grid-template-columns: 1.6fr 0.9fr;
  gap: 20px;
}

.skeleton-panel {
  height: 344px;
}

@keyframes overview-pulse {
  0%,
  100% {
    opacity: 1;
  }

  50% {
    opacity: 0.5;
  }
}

@media (max-width: 1080px) {
  .skeleton-panels {
    grid-template-columns: 1fr;
  }

  .skeleton-panel {
    height: 280px;
  }
}

@media (max-width: 640px) {
  .stat-card {
    padding: 18px 18px 16px;
  }

  .panel-card {
    padding: 18px;
  }

  .range-chip {
    padding: 6px 12px;
  }

  .skeleton-panel {
    height: 260px;
  }
}
</style>
