<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import ConfirmDialog from "../common/ConfirmDialog.vue";
import EmptyState from "../common/EmptyState.vue";
import ScheduledTaskDialog from "./ScheduledTaskDialog.vue";
import { useLauncherStore } from "../../stores/launcher";
import { useScheduledStore } from "../../stores/scheduled";
import type { ScheduledAction, ScheduledJob } from "../../types/launcher";
import { formatDateTime } from "../../utils/format";
import { scheduleDescription } from "../../utils/schedule";

const ACTION_KEYS: Record<ScheduledAction, string> = {
  start_instance: "startInstance",
  stop_instance: "stopInstance",
  backup: "backup",
  clear_cache: "clearCache",
};

const { t } = useI18n();
const { instances } = useLauncherStore();
const {
  loaded,
  error,
  jobs,
  init,
  isBusy,
  save,
  remove,
  setEnabled,
  runNow,
} = useScheduledStore();

const dialogOpen = ref(false);
const editing = ref<ScheduledJob | null>(null);
const saving = ref(false);
const pendingDelete = ref<ScheduledJob | null>(null);
const deleting = ref(false);
const retrying = ref(false);

const deleteOpen = computed({
  get: () => pendingDelete.value !== null,
  set: (value: boolean) => {
    if (!value && !deleting.value) {
      pendingDelete.value = null;
    }
  },
});

onMounted(() => {
  void init();
});

function openCreate(): void {
  editing.value = null;
  dialogOpen.value = true;
}

function openEdit(job: ScheduledJob): void {
  editing.value = job;
  dialogOpen.value = true;
}

defineExpose({ openCreate });

function actionIcon(action: ScheduledAction): string {
  return (
    {
      start_instance: "mdi-play-circle-outline",
      stop_instance: "mdi-stop-circle-outline",
      backup: "mdi-archive-outline",
      clear_cache: "mdi-broom",
    }[action] ?? "mdi-calendar-clock"
  );
}

function actionLabel(action: ScheduledAction): string {
  return t(`tasks.scheduled.jobActions.${ACTION_KEYS[action]}`);
}

function jobTitle(job: ScheduledJob): string {
  return job.name.trim() === "" ? actionLabel(job.action) : job.name;
}

function instanceName(instanceId: string): string {
  return instances.value.find((instance) => instance.id === instanceId)?.name
    ?? instanceId;
}

function resultColor(job: ScheduledJob): string {
  if (job.last_success === null) {
    return "secondary";
  }
  return job.last_success ? "success" : "error";
}

function resultLabel(job: ScheduledJob): string {
  if (job.last_success === null) {
    return t("tasks.scheduled.result.never");
  }
  return job.last_success
    ? t("tasks.scheduled.result.succeeded")
    : t("tasks.scheduled.result.failed");
}

async function submit(job: ScheduledJob): Promise<void> {
  if (saving.value) {
    return;
  }
  saving.value = true;
  try {
    if (await save(job)) {
      dialogOpen.value = false;
    }
  } finally {
    saving.value = false;
  }
}

function toggle(job: ScheduledJob, value: boolean | null): void {
  if ((value === true) === job.enabled) {
    return;
  }
  void setEnabled(job.id, value === true);
}

function askDelete(job: ScheduledJob): void {
  pendingDelete.value = job;
}

async function confirmDelete(): Promise<void> {
  const job = pendingDelete.value;
  if (job === null || deleting.value) {
    return;
  }
  deleting.value = true;
  try {
    await remove(job.id);
    pendingDelete.value = null;
  } finally {
    deleting.value = false;
  }
}

async function retryInit(): Promise<void> {
  if (retrying.value) {
    return;
  }
  retrying.value = true;
  try {
    await init();
  } finally {
    retrying.value = false;
  }
}
</script>

<template>
  <div class="scheduled-tasks">
    <v-card variant="flat" class="page-table-card scheduled-tasks__card">
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
        class="scheduled-tasks__error"
      >
        <div class="scheduled-tasks__error-row">
          <span>{{ error }}</span>
          <v-btn
            color="primary"
            variant="tonal"
            size="small"
            :loading="retrying"
            :disabled="retrying"
            @click="retryInit"
          >
            {{ t("common.actions.retry") }}
          </v-btn>
        </div>
      </v-alert>
      <TransitionGroup
        v-else-if="jobs.length > 0"
        name="motion-list"
        tag="div"
        class="scheduled-tasks__list"
      >
        <div v-for="job in jobs" :key="job.id" class="scheduled-item">
          <v-avatar
            :color="job.enabled ? 'primary' : 'secondary'"
            variant="tonal"
            size="38"
            class="scheduled-item__avatar"
          >
            <v-icon :icon="actionIcon(job.action)" size="20" />
          </v-avatar>

          <div class="scheduled-item__body">
            <div class="scheduled-item__header page-row">
              <span class="scheduled-item__title page-anywhere">
                {{ jobTitle(job) }}
              </span>
              <v-chip size="x-small" variant="tonal" color="primary">
                {{ scheduleDescription(job) }}
              </v-chip>
              <v-chip
                :color="resultColor(job)"
                size="x-small"
                variant="tonal"
              >
                {{ resultLabel(job) }}
              </v-chip>
            </div>

            <div class="scheduled-item__meta page-row">
              <span>{{ instanceName(job.instance_id) }}</span>
              <span class="scheduled-item__dot" aria-hidden="true">·</span>
              <span>
                {{ t("tasks.scheduled.fields.nextRun") }}
                {{ formatDateTime(job.next_run_at ?? "") }}
              </span>
              <template v-if="job.last_run_at">
                <span class="scheduled-item__dot" aria-hidden="true">·</span>
                <span>
                  {{ t("tasks.scheduled.fields.lastRun") }}
                  {{ formatDateTime(job.last_run_at) }}
                </span>
              </template>
            </div>

            <div v-if="job.last_error" class="scheduled-item__error page-anywhere">
              {{ job.last_error }}
            </div>
          </div>

          <div class="scheduled-item__actions">
            <v-switch
              :model-value="job.enabled"
              inset
              color="primary"
              hide-details
              density="compact"
              class="scheduled-item__switch"
              :aria-label="jobTitle(job)"
              :loading="isBusy(job.id)"
              :disabled="isBusy(job.id)"
              @update:model-value="toggle(job, $event)"
            />
            <v-tooltip :text="t('tasks.scheduled.actions.runNow')">
              <template #activator="{ props: tip }">
                <v-btn
                  v-bind="tip"
                  icon="mdi-play"
                  size="small"
                  variant="text"
                  :aria-label="t('tasks.scheduled.actions.runNow')"
                  :loading="isBusy(job.id)"
                  @click="runNow(job.id)"
                />
              </template>
            </v-tooltip>
            <v-tooltip :text="t('tasks.scheduled.actions.edit')">
              <template #activator="{ props: tip }">
                <v-btn
                  v-bind="tip"
                  icon="mdi-pencil-outline"
                  size="small"
                  variant="text"
                  :aria-label="t('tasks.scheduled.actions.edit')"
                  @click="openEdit(job)"
                />
              </template>
            </v-tooltip>
            <v-tooltip :text="t('tasks.scheduled.actions.remove')">
              <template #activator="{ props: tip }">
                <v-btn
                  v-bind="tip"
                  icon="mdi-delete-outline"
                  size="small"
                  variant="text"
                  color="error"
                  :aria-label="t('tasks.scheduled.actions.remove')"
                  @click="askDelete(job)"
                />
              </template>
            </v-tooltip>
          </div>
        </div>
      </TransitionGroup>
      <EmptyState
        v-else
        :text="t('tasks.scheduled.empty')"
        icon="mdi-calendar-clock"
      />
    </v-card>

    <p class="scheduled-tasks__hint">{{ t("tasks.scheduled.hint") }}</p>

    <ScheduledTaskDialog
      v-model="dialogOpen"
      :instances="instances"
      :job="editing"
      :loading="saving"
      @submit="submit"
    />

    <ConfirmDialog
      v-model="deleteOpen"
      :title="t('tasks.scheduled.confirm.title')"
      :message="
        t('tasks.scheduled.confirm.message', {
          name: pendingDelete ? jobTitle(pendingDelete) : '',
        })
      "
      color="error"
      :loading="deleting"
      @confirm="confirmDelete"
    />
  </div>
</template>

<style scoped>
.scheduled-tasks__card {
  overflow: hidden;
}

.scheduled-tasks__list {
  display: flex;
  flex-direction: column;
}

.scheduled-tasks__error {
  margin: 12px;
}

.scheduled-tasks__error-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 8px 12px;
}

.scheduled-tasks__hint {
  margin: 8px 2px 0;
  color: rgba(var(--v-theme-on-surface), 0.52);
  font-size: 0.75rem;
  line-height: 1.4;
}

.scheduled-item {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 18px;
}

.scheduled-item + .scheduled-item {
  border-top: 1px solid rgba(var(--v-theme-on-surface), 0.08);
}

.scheduled-item__avatar {
  flex-shrink: 0;
}

.scheduled-item__body {
  flex: 1 1 auto;
  min-width: 0;
}

.scheduled-item__header {
  gap: 8px;
}

.scheduled-item__title {
  color: rgb(var(--v-theme-on-surface));
  font-size: 0.95rem;
  font-weight: 600;
  line-height: 1.4;
}

.scheduled-item__meta {
  gap: 6px;
  margin-top: 6px;
  color: rgba(var(--v-theme-on-surface), 0.52);
  font-size: 0.75rem;
  line-height: 1.4;
}

.scheduled-item__dot {
  opacity: 0.6;
}

.scheduled-item__error {
  margin-top: 4px;
  color: rgb(var(--v-theme-error));
  font-size: 0.75rem;
  line-height: 1.4;
}

.scheduled-item__actions {
  display: flex;
  flex-shrink: 0;
  align-items: center;
  gap: 2px;
  margin-left: auto;
}

.scheduled-item__switch {
  margin-right: 4px;
}
</style>
