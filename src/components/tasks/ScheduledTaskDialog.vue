<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type {
  InstanceStatus,
  ScheduledAction,
  ScheduledJob,
  ScheduleMode,
} from "../../types/launcher";

const props = withDefaults(
  defineProps<{
    modelValue: boolean;
    instances: InstanceStatus[];
    job: ScheduledJob | null;
    loading?: boolean;
  }>(),
  { loading: false },
);

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
  (event: "submit", job: ScheduledJob): void;
}>();

const { t } = useI18n();

const name = ref("");
const action = ref<ScheduledAction>("start_instance");
const instanceId = ref<string | null>(null);
const mode = ref<ScheduleMode>("daily");
const runAt = ref("");
const intervalValue = ref("60");
const intervalUnit = ref<"minutes" | "hours">("minutes");
const time = ref("03:00");
const weekday = ref(1);

const actionOptions = computed(() => [
  { title: t("tasks.scheduled.jobActions.startInstance"), value: "start_instance" },
  { title: t("tasks.scheduled.jobActions.stopInstance"), value: "stop_instance" },
  { title: t("tasks.scheduled.jobActions.backup"), value: "backup" },
  { title: t("tasks.scheduled.jobActions.clearCache"), value: "clear_cache" },
]);

const modeOptions = computed(() => [
  { title: t("tasks.scheduled.modes.once"), value: "once" },
  { title: t("tasks.scheduled.modes.interval"), value: "interval" },
  { title: t("tasks.scheduled.modes.daily"), value: "daily" },
  { title: t("tasks.scheduled.modes.weekly"), value: "weekly" },
]);

const unitOptions = computed(() => [
  { title: t("tasks.scheduled.units.minutes"), value: "minutes" },
  { title: t("tasks.scheduled.units.hours"), value: "hours" },
]);

const weekdayOptions = computed(() =>
  ["sun", "mon", "tue", "wed", "thu", "fri", "sat"].map((key, value) => ({
    title: t(`tasks.scheduled.weekdays.${key}`),
    value,
  })),
);

const intervalMinutes = computed(() => {
  const parsed = Number(intervalValue.value);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return null;
  }
  return intervalUnit.value === "hours" ? Math.round(parsed * 60) : Math.round(parsed);
});

const onceTimeValid = computed(
  () => runAt.value !== "" && Date.parse(runAt.value) > Date.now(),
);
const intervalValid = computed(
  () =>
    intervalMinutes.value !== null &&
    intervalMinutes.value >= 1 &&
    intervalMinutes.value <= 10_080,
);
const timeValid = computed(() => /^\d{2}:\d{2}$/.test(time.value));

const valid = computed(() => {
  if (instanceId.value === null) {
    return false;
  }
  if (mode.value === "once") {
    return onceTimeValid.value;
  }
  if (mode.value === "interval") {
    return intervalValid.value;
  }
  if (mode.value === "weekly" && (weekday.value < 0 || weekday.value > 6)) {
    return false;
  }
  return timeValid.value;
});

function resetFromJob(): void {
  const job = props.job;
  name.value = job?.name ?? "";
  action.value = job?.action ?? "start_instance";
  instanceId.value = job?.instance_id ?? props.instances[0]?.id ?? null;
  mode.value = job?.mode ?? "daily";
  runAt.value = job?.run_at ?? "";
  time.value = job?.time ?? "03:00";
  weekday.value = job?.weekday ?? 1;
  const minutes = job?.interval_minutes ?? 60;
  if (minutes % 60 === 0 && minutes >= 60) {
    intervalUnit.value = "hours";
    intervalValue.value = String(minutes / 60);
  } else {
    intervalUnit.value = "minutes";
    intervalValue.value = String(minutes);
  }
}

watch(
  () => props.modelValue,
  (open) => {
    if (open) {
      resetFromJob();
    }
  },
  { immediate: true },
);

// Launcher refreshes replace the instance list; only fill a still-empty choice.
watch(
  () => props.instances.map((instance) => instance.id).join(","),
  () => {
    if (instanceId.value === null && props.instances.length > 0) {
      instanceId.value = props.instances[0].id;
    }
  },
);

function updateOpen(value: boolean): void {
  if (!value && props.loading) {
    return;
  }
  emit("update:modelValue", value);
}

function submit(): void {
  if (props.loading || !valid.value || instanceId.value === null) {
    return;
  }
  emit("submit", {
    id: props.job?.id ?? "",
    name: name.value.trim(),
    action: action.value,
    instance_id: instanceId.value,
    mode: mode.value,
    run_at: mode.value === "once" ? runAt.value : null,
    interval_minutes: mode.value === "interval" ? intervalMinutes.value : null,
    time: mode.value === "daily" || mode.value === "weekly" ? time.value : null,
    weekday: mode.value === "weekly" ? weekday.value : null,
    enabled: props.job?.enabled ?? true,
    next_run_at: props.job?.next_run_at ?? null,
    last_run_at: props.job?.last_run_at ?? null,
    last_success: props.job?.last_success ?? null,
    last_error: props.job?.last_error ?? null,
  });
}
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    max-width="480"
    :persistent="loading"
    @update:model-value="updateOpen"
  >
    <v-card>
      <v-card-title class="scheduled-dialog__title">
        {{
          job
            ? t("tasks.scheduled.dialog.editTitle")
            : t("tasks.scheduled.dialog.createTitle")
        }}
      </v-card-title>
      <v-card-text class="scheduled-dialog__body">
        <v-alert
          v-if="instances.length === 0"
          type="warning"
          variant="tonal"
          density="compact"
        >
          {{ t("tasks.scheduled.dialog.noInstances") }}
        </v-alert>
        <template v-else>
          <v-select
            v-model="instanceId"
            :items="instances"
            item-title="name"
            item-value="id"
            :label="t('tasks.scheduled.fields.instance')"
            variant="outlined"
            density="compact"
            hide-details
            :disabled="loading"
          />
          <v-select
            v-model="action"
            :items="actionOptions"
            :label="t('tasks.scheduled.fields.action')"
            variant="outlined"
            density="compact"
            hide-details
            :disabled="loading"
          />
          <v-text-field
            v-model="name"
            :label="t('tasks.scheduled.fields.name')"
            variant="outlined"
            density="compact"
            hide-details
            maxlength="60"
            :disabled="loading"
          />
          <v-select
            v-model="mode"
            :items="modeOptions"
            :label="t('tasks.scheduled.fields.schedule')"
            variant="outlined"
            density="compact"
            hide-details
            :disabled="loading"
          />

          <v-text-field
            v-if="mode === 'once'"
            v-model="runAt"
            type="datetime-local"
            :label="t('tasks.scheduled.dialog.dateTime')"
            variant="outlined"
            density="compact"
            :disabled="loading"
            :error-messages="
              runAt !== '' && !onceTimeValid
                ? [t('tasks.scheduled.dialog.timeInPast')]
                : []
            "
          />

          <div v-else-if="mode === 'interval'" class="scheduled-dialog__row">
            <v-text-field
              v-model="intervalValue"
              type="number"
              min="1"
              :label="t('tasks.scheduled.dialog.intervalCount')"
              variant="outlined"
              density="compact"
              hide-details
              :disabled="loading"
            />
            <v-select
              v-model="intervalUnit"
              :items="unitOptions"
              :label="t('tasks.scheduled.dialog.intervalUnit')"
              variant="outlined"
              density="compact"
              hide-details
              :disabled="loading"
            />
          </div>

          <v-text-field
            v-else
            v-model="time"
            type="time"
            :label="t('tasks.scheduled.dialog.time')"
            variant="outlined"
            density="compact"
            hide-details
            :disabled="loading"
          />

          <v-select
            v-if="mode === 'weekly'"
            v-model="weekday"
            :items="weekdayOptions"
            :label="t('tasks.scheduled.dialog.weekday')"
            variant="outlined"
            density="compact"
            hide-details
            :disabled="loading"
          />
        </template>
      </v-card-text>
      <v-card-actions class="scheduled-dialog__actions">
        <v-spacer />
        <v-btn variant="text" :disabled="loading" @click="updateOpen(false)">
          {{ t("common.actions.cancel") }}
        </v-btn>
        <v-btn
          color="primary"
          variant="flat"
          :loading="loading"
          :disabled="instances.length === 0 || !valid"
          @click="submit"
        >
          {{ t("common.actions.save") }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.scheduled-dialog__title {
  padding: 20px 20px 0;
  font-size: 1.05rem;
  font-weight: 600;
}

.scheduled-dialog__body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px 20px 0;
}

.scheduled-dialog__row {
  display: grid;
  grid-template-columns: 1.4fr 1fr;
  gap: 12px;
}

.scheduled-dialog__actions {
  padding: 16px;
}
</style>
