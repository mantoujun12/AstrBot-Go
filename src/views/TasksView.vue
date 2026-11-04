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
  return {
    download: "mdi-download-outline",
    deploy: "mdi-rocket-launch-outline",
    backup: "mdi-archive-outline",
    repair: "mdi-wrench-outline",
  }[kind];
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
  const translated = t(task.message_key);
  return translated === task.message_key ? task.message : translated;
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
  <div class="tasks-view">
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

    <v-card variant="flat" class="tasks-view__card">
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
      <v-list
        v-else-if="filteredTasks.length > 0"
        lines="two"
        class="tasks-view__list"
      >
        <v-list-item
          v-for="task in filteredTasks"
          :key="task.id"
          class="task-item"
        >
          <template #prepend>
            <v-avatar color="surface-variant" size="38">
              <v-icon :icon="kindIcon(task.kind)" size="20" />
            </v-avatar>
          </template>

          <v-list-item-title class="task-item__title">
            <span>{{ taskTitle(task) }}</span>
            <v-chip
              :color="statusColor(task.status)"
              size="x-small"
              variant="tonal"
              label
            >
              {{ t(`tasks.status.${task.status}`) }}
            </v-chip>
          </v-list-item-title>

          <v-list-item-subtitle>
            <div class="task-item__message">{{ taskMessage(task) }}</div>
            <v-progress-linear
              :model-value="task.progress ?? 0"
              :indeterminate="isActive(task) && task.progress === null"
              :color="statusColor(task.status)"
              height="5"
              rounded
              class="task-item__progress"
            />
            <div class="task-item__meta">
              <span>
                {{
                  t("tasks.fields.targetValue", {
                    target: targetLabel(task),
                  })
                }}
              </span>
              <span>{{ formatDateTime(task.updated_at) }}</span>
            </div>
            <div v-if="task.error" class="task-item__error">
              {{ errorText(task) }}
            </div>
          </v-list-item-subtitle>

          <template #append>
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
          </template>
        </v-list-item>
      </v-list>
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
  border: 1px solid rgba(var(--v-theme-on-surface), 0.08);
  border-radius: 12px;
}

.tasks-view__error {
  margin: 12px;
}

.task-item {
  padding-block: 12px;
  align-items: flex-start;
}

.task-item__title {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.task-item__message {
  margin-bottom: 8px;
  color: rgba(var(--v-theme-on-surface), 0.72);
}

.task-item__progress {
  max-width: 420px;
  margin-bottom: 8px;
}

.task-item__meta {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  color: rgba(var(--v-theme-on-surface), 0.52);
  font-size: 0.75rem;
}

.task-item__error {
  margin-top: 4px;
  color: rgb(var(--v-theme-error));
  font-size: 0.75rem;
}

.task-item__actions {
  display: flex;
  align-items: center;
  gap: 2px;
}
</style>
