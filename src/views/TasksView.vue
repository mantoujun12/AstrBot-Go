<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import EmptyState from "../components/common/EmptyState.vue";
import PageHeader from "../components/common/PageHeader.vue";
import { useLauncherStore } from "../stores/launcher";
import { useTaskStore } from "../stores/tasks";
import type { TaskKind, TaskRecord, TaskStatus } from "../types/launcher";
import { formatDateTime } from "../utils/format";

type TaskFilter = "all" | TaskKind;

const { t } = useI18n();
const { instances } = useLauncherStore();
const {
  loaded,
  error,
  tasks,
  activeCount,
  cancelTask,
  pauseTask,
  resumeTask,
  removeTask,
  clearFinishedTasks,
} = useTaskStore();

const filter = ref<TaskFilter>("all");

const filters: TaskFilter[] = ["all", "download", "deploy", "backup", "repair"];
const filteredTasks = computed(() =>
  filter.value === "all"
    ? tasks.value
    : tasks.value.filter((task) => task.kind === filter.value),
);

function kindIcon(kind: TaskKind): string {
  return (
    {
      download: "mdi-download-outline",
      deploy: "mdi-rocket-launch-outline",
      backup: "mdi-archive-outline",
      repair: "mdi-wrench-outline",
    }[kind] ?? "mdi-clipboard-text-outline"
  );
}

function statusColor(status: TaskStatus): string {
  return {
    queued: "secondary",
    running: "primary",
    paused: "warning",
    cancelling: "warning",
    cancelled: "secondary",
    succeeded: "success",
    failed: "error",
    interrupted: "error",
  }[status];
}

function isActive(task: TaskRecord): boolean {
  return ["queued", "running", "paused", "cancelling"].includes(task.status);
}

function taskTitle(task: TaskRecord): string {
  return t(task.title_key, task.title_args);
}

function taskMessage(task: TaskRecord): string {
  if (!task.message_key && !task.message) {
    return "";
  }
  const translated = t(task.message_key);
  const text = (translated === task.message_key ? task.message : translated).trim();
  const statusText = t(`tasks.status.${task.status}`).trim();
  if (!isActive(task) && text === statusText) {
    return "";
  }
  return text;
}

function progressLabel(task: TaskRecord): string | null {
  if (task.progress === null || !Number.isFinite(task.progress)) {
    return null;
  }
  const clamped = Math.min(100, Math.max(0, Math.round(task.progress)));
  return `${clamped}%`;
}

function taskDuration(task: TaskRecord): string | null {
  if (!task.started_at || !task.finished_at) {
    return null;
  }
  const start = Date.parse(task.started_at);
  const end = Date.parse(task.finished_at);
  if (Number.isNaN(start) || Number.isNaN(end) || end < start) {
    return null;
  }
  const totalSeconds = Math.max(1, Math.round((end - start) / 1000));
  if (totalSeconds < 60) {
    return `${totalSeconds}s`;
  }
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return seconds > 0 ? `${minutes}m ${seconds}s` : `${minutes}m`;
}

function targetLabel(task: TaskRecord): string {
  if (!task.target_id) {
    return "—";
  }
  if (task.kind === "download") {
    return task.target_id;
  }
  return instances.value.find((instance) => instance.id === task.target_id)?.name
    ?? task.target_id;
}

function errorText(task: TaskRecord): string {
  if (!task.error) {
    return "";
  }
  const key = `tasks.errors.${task.error.kind}`;
  const translated = t(key);
  return translated === key ? t("tasks.errors.other") : translated;
}
</script>

<template>
  <div class="page-shell tasks-view">
    <PageHeader :title="t('tasks.title')">
      <template #actions>
        <v-btn
          variant="tonal"
          prepend-icon="mdi-broom"
          :disabled="!loaded || activeCount === tasks.length"
          @click="clearFinishedTasks"
        >
          {{ t("tasks.actions.clearFinished") }}
        </v-btn>
      </template>
    </PageHeader>

    <v-btn-toggle
      v-model="filter"
      mandatory
      density="compact"
      color="primary"
      divided
      variant="outlined"
      class="tasks-view__filters"
    >
      <v-btn v-for="item in filters" :key="item" :value="item" size="small">
        {{ t(`tasks.filters.${item}`) }}
      </v-btn>
    </v-btn-toggle>

    <v-card variant="flat" class="page-table-card tasks-view__card">
      <v-progress-linear
        v-if="!loaded"
        indeterminate
        color="primary"
        height="3"
      />
      <v-alert
        v-else-if="error"
        type="error"
        variant="tonal"
        class="tasks-view__error"
      >
        {{ error }}
      </v-alert>
      <TransitionGroup
        v-else-if="filteredTasks.length > 0"
        name="motion-list"
        tag="div"
        class="tasks-view__list"
      >
        <div
          v-for="task in filteredTasks"
          :key="task.id"
          class="task-item"
        >
          <v-avatar
            :color="statusColor(task.status)"
            variant="tonal"
            size="38"
            class="task-item__avatar"
          >
            <v-icon :icon="kindIcon(task.kind)" size="20" />
          </v-avatar>

          <div class="task-item__body">
            <div class="task-item__header page-row">
              <span class="task-item__title page-anywhere">{{ taskTitle(task) }}</span>
              <v-chip
                :color="statusColor(task.status)"
                size="x-small"
                variant="tonal"
                label
              >
                {{ t(`tasks.status.${task.status}`) }}
              </v-chip>
            </div>

            <div v-if="taskMessage(task)" class="task-item__message page-anywhere">
              {{ taskMessage(task) }}
            </div>

            <div v-if="isActive(task)" class="task-item__progress-row">
              <v-progress-linear
                :model-value="task.progress ?? 0"
                :indeterminate="task.progress === null"
                :color="statusColor(task.status)"
                height="6"
                rounded
                class="task-item__progress"
              />
              <span
                v-if="progressLabel(task)"
                class="task-item__progress-value page-nums"
              >
                {{ progressLabel(task) }}
              </span>
            </div>

            <div v-if="task.error" class="task-item__error page-anywhere">
              {{ errorText(task) }}
            </div>

            <div class="task-item__meta page-row">
              <span>
                {{
                  t("tasks.fields.targetValue", {
                    target: targetLabel(task),
                  })
                }}
              </span>
              <template v-if="taskDuration(task)">
                <span class="task-item__dot" aria-hidden="true">·</span>
                <span>{{ taskDuration(task) }}</span>
              </template>
              <span class="task-item__dot" aria-hidden="true">·</span>
              <span>{{ formatDateTime(task.updated_at) }}</span>
            </div>
          </div>

          <div class="task-item__actions">
            <v-btn
              v-if="task.pausable && task.status === 'running'"
              icon="mdi-pause"
              size="small"
              variant="text"
              :aria-label="t('tasks.actions.pause')"
              @click="pauseTask(task.id)"
            />
            <v-btn
              v-if="task.pausable && task.status === 'paused'"
              icon="mdi-play"
              size="small"
              variant="text"
              :aria-label="t('tasks.actions.resume')"
              @click="resumeTask(task.id)"
            />
            <v-btn
              v-if="task.cancellable && isActive(task)"
              icon="mdi-close"
              size="small"
              variant="text"
              :aria-label="t('tasks.actions.cancel')"
              :loading="task.status === 'cancelling'"
              @click="cancelTask(task.id)"
            />
            <v-btn
              v-if="!isActive(task)"
              icon="mdi-delete-outline"
              size="small"
              variant="text"
              :aria-label="t('tasks.actions.remove')"
              @click="removeTask(task.id)"
            />
          </div>
        </div>
      </TransitionGroup>
      <EmptyState
        v-else
        :text="t('tasks.empty')"
        icon="mdi-clipboard-check-outline"
      />
    </v-card>
  </div>
</template>

<style scoped>
.tasks-view__filters {
  max-width: 100%;
  margin-bottom: 12px;
  border-radius: 999px;
  overflow-x: auto;
  overflow-y: hidden;
}

.tasks-view__card {
  overflow: hidden;
}

.tasks-view__list {
  display: flex;
  flex-direction: column;
}

.tasks-view__error {
  margin: 12px;
}

.task-item {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 18px;
}

.task-item + .task-item {
  border-top: 1px solid rgba(var(--v-theme-on-surface), 0.08);
}

.task-item__avatar {
  flex-shrink: 0;
}

.task-item__body {
  flex: 1 1 auto;
  min-width: 0;
}

.task-item__header {
  gap: 8px;
}

.task-item__title {
  color: rgb(var(--v-theme-on-surface));
  font-size: 0.95rem;
  font-weight: 600;
  line-height: 1.4;
}

.task-item__message {
  margin-top: 3px;
  color: rgba(var(--v-theme-on-surface), 0.68);
  font-size: 0.82rem;
  line-height: 1.45;
}

.task-item__progress-row {
  display: flex;
  align-items: center;
  gap: 10px;
  max-width: 440px;
  margin-top: 8px;
}

.task-item__progress {
  flex: 1 1 auto;
}

.task-item__progress-value {
  flex-shrink: 0;
  color: rgba(var(--v-theme-on-surface), 0.68);
  font-size: 0.75rem;
}

.task-item__meta {
  gap: 6px;
  margin-top: 6px;
  color: rgba(var(--v-theme-on-surface), 0.52);
  font-size: 0.75rem;
  line-height: 1.4;
}

.task-item__dot {
  opacity: 0.6;
}

.task-item__error {
  margin-top: 4px;
  color: rgb(var(--v-theme-error));
  font-size: 0.75rem;
  line-height: 1.4;
}

.task-item__actions {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  gap: 2px;
  margin-left: auto;
}
</style>
