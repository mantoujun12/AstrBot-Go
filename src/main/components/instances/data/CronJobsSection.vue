<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import type { DataTableHeader } from "vuetify";
import type { InstanceCronJob } from "../../../types/instanceData";
import { formatDateTime } from "../../../utils/format";

/** Converts a stored ISO timestamp into the local `datetime-local` value. */
function toLocalInput(value: unknown): string {
  if (typeof value !== "string") {
    return "";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return "";
  }
  const pad = (part: number): string => String(part).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

const props = defineProps<{
  jobs: InstanceCronJob[];
  readonly: boolean;
}>();

const emit = defineEmits<{
  (event: "upsert", job: InstanceCronJob): void;
  (event: "remove", jobIds: string[]): void;
  (event: "copy", jobId: string): void;
}>();

const { t } = useI18n();

const selected = ref<string[]>([]);

const headers = computed<DataTableHeader[]>(() => [
  {
    title: t("instanceData.cronJobs.columns.name"),
    key: "name",
    sortable: false,
  },
  {
    title: t("instanceData.cronJobs.columns.jobType"),
    key: "job_type",
    sortable: false,
    width: 130,
  },
  {
    title: t("instanceData.cronJobs.columns.cronExpression"),
    key: "cron_expression",
    sortable: false,
  },
  {
    title: t("instanceData.cronJobs.columns.enabled"),
    key: "enabled",
    sortable: false,
    width: 90,
  },
  {
    title: t("instanceData.cronJobs.columns.nextRun"),
    key: "next_run_time",
    sortable: false,
  },
  {
    title: t("instanceData.actionsColumn"),
    key: "actions",
    sortable: false,
    align: "end",
    width: 140,
  },
]);

const LOCAL_TIMEZONE = Intl.DateTimeFormat().resolvedOptions().timeZone;

function jobTypeLabel(jobType: InstanceCronJob["job_type"]): string {
  return jobType === "basic"
    ? t("instanceData.cronJobs.jobTypes.basic")
    : t("instanceData.cronJobs.jobTypes.activeAgent");
}

function toggleEnabled(job: InstanceCronJob): void {
  emit("upsert", { ...job, enabled: !job.enabled });
}

const formOpen = ref(false);
const editing = ref<InstanceCronJob | null>(null);
const name = ref("");
const description = ref("");
const jobType = ref<InstanceCronJob["job_type"]>("basic");
const cronExpression = ref("");
const timezone = ref("");
const payloadText = ref("");
const runAtText = ref("");
const promptText = ref("");
const enabled = ref(true);
const persistent = ref(true);
const runOnce = ref(false);

function openAdd(): void {
  editing.value = null;
  name.value = "";
  description.value = "";
  jobType.value = "basic";
  cronExpression.value = "";
  timezone.value = "";
  payloadText.value = "{}";
  runAtText.value = "";
  promptText.value = "";
  enabled.value = true;
  persistent.value = true;
  runOnce.value = false;
  formOpen.value = true;
}

function openEdit(job: InstanceCronJob): void {
  editing.value = job;
  name.value = job.name;
  description.value = job.description ?? "";
  jobType.value = job.job_type;
  cronExpression.value = job.cron_expression ?? "";
  timezone.value = job.timezone ?? "";
  payloadText.value = JSON.stringify(job.payload, null, 2);
  runAtText.value = job.run_once ? toLocalInput(job.payload.run_at) : "";
  promptText.value =
    typeof job.payload.prompt === "string" ? job.payload.prompt : "";
  enabled.value = job.enabled;
  persistent.value = job.persistent;
  runOnce.value = job.run_once;
  formOpen.value = true;
}

const payloadError = computed(() => {
  try {
    const parsed: unknown = JSON.parse(payloadText.value);
    if (
      typeof parsed !== "object" ||
      parsed === null ||
      Array.isArray(parsed)
    ) {
      return t("instanceData.cronJobs.payloadInvalid");
    }
    return "";
  } catch {
    return t("instanceData.cronJobs.payloadInvalid");
  }
});

const nameError = computed(() =>
  name.value.trim() === "" ? t("instanceData.cronJobs.nameRequired") : "",
);

const cronError = computed(() =>
  !runOnce.value && cronExpression.value.trim() === ""
    ? t("instanceData.cronJobs.cronRequired")
    : "",
);

const formValid = computed(
  () =>
    nameError.value === "" &&
    cronError.value === "" &&
    payloadError.value === "",
);

function submitForm(): void {
  if (!formValid.value) {
    return;
  }
  let payload: Record<string, unknown> = {};
  try {
    payload = JSON.parse(payloadText.value) as Record<string, unknown>;
  } catch {
    return;
  }
  // Structured fields merge on top of the raw JSON payload; other keys stay.
  if (runOnce.value && runAtText.value !== "") {
    const runAt = new Date(runAtText.value);
    if (!Number.isNaN(runAt.getTime())) {
      payload.run_at = runAt.toISOString();
    }
  }
  if (jobType.value === "active_agent") {
    payload.prompt = promptText.value;
  }
  emit("upsert", {
    job_id: editing.value?.job_id ?? crypto.randomUUID(),
    name: name.value.trim(),
    description: description.value.trim() || null,
    job_type: jobType.value,
    cron_expression: runOnce.value ? null : cronExpression.value.trim(),
    timezone: timezone.value.trim() || null,
    payload,
    enabled: enabled.value,
    persistent: persistent.value,
    run_once: runOnce.value,
    status: editing.value?.status ?? null,
    last_run_at: editing.value?.last_run_at ?? null,
    next_run_time: editing.value?.next_run_time ?? null,
    last_error: editing.value?.last_error ?? null,
  });
  formOpen.value = false;
}

function removeSelected(): void {
  emit("remove", selected.value);
  selected.value = [];
}

function removeOne(jobId: string): void {
  emit("remove", [jobId]);
  selected.value = selected.value.filter((id) => id !== jobId);
}
</script>

<template>
  <div class="cron-jobs">
    <div class="cron-jobs__toolbar page-row">
      <v-spacer />
      <v-btn
        size="small"
        variant="text"
        color="error"
        prepend-icon="mdi-delete-outline"
        :disabled="selected.length === 0 || readonly"
        @click="removeSelected"
      >
        {{ t("common.actions.delete") }}
      </v-btn>
      <v-btn
        size="small"
        color="primary"
        variant="tonal"
        prepend-icon="mdi-plus"
        :disabled="readonly"
        @click="openAdd"
      >
        {{ t("instanceData.cronJobs.addTitle") }}
      </v-btn>
    </div>

    <div class="page-table-card">
      <v-data-table
        v-model:selected="selected"
        :headers="headers"
        :items="jobs"
        item-value="job_id"
        show-select
        hide-default-footer
        :items-per-page="-1"
        disable-sort
        hover
        :no-data-text="t('instanceData.cronJobs.empty')"
      >
        <template #item.job_type="{ item }">
          {{ jobTypeLabel(item.job_type) }}
        </template>

        <template #item.cron_expression="{ item }">
          <span class="page-mono page-anywhere">
            {{ item.cron_expression || "—" }}
          </span>
        </template>

        <template #item.enabled="{ item }">
          <v-switch
            :model-value="item.enabled"
            inset
            color="primary"
            hide-details
            :disabled="readonly"
            :aria-label="t('instanceData.cronJobs.fields.enabled')"
            @update:model-value="toggleEnabled(item)"
          />
        </template>

        <template #item.next_run_time="{ item }">
          <span class="page-nums">{{ formatDateTime(item.next_run_time ?? "") }}</span>
        </template>

        <template #item.actions="{ item }">
          <div class="page-row cron-jobs__row-actions">
            <v-tooltip location="top">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-pencil-outline"
                  variant="text"
                  size="small"
                  :aria-label="t('common.actions.edit')"
                  :disabled="readonly"
                  @click="openEdit(item)"
                />
              </template>
              {{ t("common.actions.edit") }}
            </v-tooltip>
            <v-tooltip location="top">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-content-duplicate"
                  variant="text"
                  size="small"
                  :aria-label="t('instanceData.copy.title')"
                  @click="emit('copy', item.job_id)"
                />
              </template>
              {{ t("instanceData.copy.title") }}
            </v-tooltip>
            <v-tooltip location="top">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-delete-outline"
                  variant="text"
                  size="small"
                  :aria-label="t('common.actions.delete')"
                  :disabled="readonly"
                  @click="removeOne(item.job_id)"
                />
              </template>
              {{ t("common.actions.delete") }}
            </v-tooltip>
          </div>
        </template>
      </v-data-table>
    </div>

    <v-dialog v-model="formOpen" max-width="560">
      <v-card class="cron-form">
        <v-card-title class="cron-form__title">
          {{
            editing
              ? t("instanceData.cronJobs.editTitle")
              : t("instanceData.cronJobs.addTitle")
          }}
        </v-card-title>
        <v-card-text class="cron-form__body">
          <v-text-field
            v-model="name"
            variant="outlined"
            density="compact"
            :label="t('instanceData.cronJobs.fields.name')"
            :error-messages="nameError === '' ? [] : [nameError]"
          />
          <v-text-field
            v-model="description"
            variant="outlined"
            density="compact"
            hide-details
            :label="t('instanceData.cronJobs.fields.description')"
          />
          <v-radio-group
            v-model="jobType"
            inline
            hide-details
            density="compact"
          >
            <v-radio
              :label="t('instanceData.cronJobs.jobTypes.basic')"
              value="basic"
              color="primary"
              density="compact"
            />
            <v-radio
              :label="t('instanceData.cronJobs.jobTypes.activeAgent')"
              value="active_agent"
              color="primary"
              density="compact"
            />
          </v-radio-group>
          <v-text-field
            v-model="cronExpression"
            variant="outlined"
            density="compact"
            class="page-mono"
            :label="t('instanceData.cronJobs.fields.cronExpression')"
            :disabled="runOnce"
            :error-messages="cronError === '' ? [] : [cronError]"
          />
          <v-text-field
            v-model="timezone"
            variant="outlined"
            density="compact"
            hide-details
            :label="t('instanceData.cronJobs.fields.timezone')"
            :placeholder="LOCAL_TIMEZONE"
          />
          <v-textarea
            v-model="payloadText"
            variant="outlined"
            density="compact"
            rows="5"
            class="page-mono"
            :label="t('instanceData.cronJobs.fields.payload')"
            :error-messages="payloadError === '' ? [] : [payloadError]"
          />
          <v-text-field
            v-if="runOnce"
            v-model="runAtText"
            type="datetime-local"
            variant="outlined"
            density="compact"
            hide-details
            :label="t('instanceData.cronJobs.fields.runAt')"
          />
          <v-textarea
            v-if="jobType === 'active_agent'"
            v-model="promptText"
            variant="outlined"
            density="compact"
            rows="3"
            :label="t('instanceData.cronJobs.fields.prompt')"
          />
          <div class="cron-form__switches">
            <v-switch
              v-model="enabled"
              inset
              color="primary"
              density="compact"
              hide-details
              :label="t('instanceData.cronJobs.fields.enabled')"
            />
            <v-switch
              v-model="persistent"
              inset
              color="primary"
              density="compact"
              hide-details
              :label="t('instanceData.cronJobs.fields.persistent')"
            />
            <v-switch
              v-model="runOnce"
              inset
              color="primary"
              density="compact"
              hide-details
              :label="t('instanceData.cronJobs.fields.runOnce')"
            />
          </div>
        </v-card-text>
        <v-card-actions class="cron-form__actions">
          <v-spacer />
          <v-btn variant="text" @click="formOpen = false">
            {{ t("common.actions.cancel") }}
          </v-btn>
          <v-btn
            color="primary"
            variant="flat"
            :disabled="!formValid"
            @click="submitForm"
          >
            {{ t("common.actions.save") }}
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<style scoped>
.cron-jobs__toolbar {
  gap: 8px;
  margin-bottom: 12px;
}

.cron-jobs__row-actions {
  justify-content: flex-end;
  gap: 2px;
}

.cron-form__title {
  padding: 20px 20px 0;
  font-size: 1.05rem;
  font-weight: 600;
}

.cron-form__body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px 20px 0;
}

.cron-form__switches {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 24px;
}

.cron-form__actions {
  padding: 16px;
}
</style>
