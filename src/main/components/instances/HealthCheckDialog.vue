<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type {
  InstanceHealthCheck,
  InstanceHealthReport,
} from "../../types/launcher";
import { formatFileSize } from "../../utils/format";

const props = defineProps<{
  modelValue: boolean;
  instanceName: string;
  report: InstanceHealthReport | null;
  loading: boolean;
  error: string | null;
}>();

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
  (event: "retry"): void;
}>();

const { t } = useI18n();

const gradeColor = computed(() => {
  switch (props.report?.overall) {
    case "critical":
      return "error";
    case "warning":
      return "warning";
    case "healthy":
      return "success";
    default:
      return "secondary";
  }
});

const gradeIcon = computed(() => {
  switch (props.report?.overall) {
    case "critical":
      return "mdi-alert-octagon-outline";
    case "warning":
      return "mdi-alert-outline";
    case "healthy":
      return "mdi-check-circle-outline";
    default:
      return "mdi-heart-pulse";
  }
});

function checkColor(check: InstanceHealthCheck): string {
  switch (check.status) {
    case "pass":
      return "success";
    case "warning":
      return "warning";
    case "fail":
      return "error";
    case "skipped":
      return "secondary";
  }
}

function displayValue(check: InstanceHealthCheck): string {
  if (check.value === null) {
    return "";
  }
  if (check.key === "disk_space") {
    const bytes = Number(check.value);
    return bytes === 0 ? "0 B" : formatFileSize(bytes);
  }
  return check.value;
}

function checkDetail(check: InstanceHealthCheck): string {
  const key = `instances.health.details.${check.detail_code}`;
  const value = displayValue(check);
  return value ? t(key, { value }) : t(key);
}
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    max-width="560"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card>
      <v-card-title class="health-check-dialog__title">
        {{ t("instances.health.title", { instance: instanceName }) }}
      </v-card-title>
      <v-card-text class="health-check-dialog__body">
        <v-progress-linear
          v-if="loading"
          color="primary"
          indeterminate
          class="health-check-dialog__progress"
        />
        <v-alert
          v-if="error"
          type="error"
          variant="tonal"
          density="compact"
          class="health-check-dialog__alert"
        >
          {{ error }}
        </v-alert>
        <template v-if="report">
          <v-alert
            :color="gradeColor"
            :icon="gradeIcon"
            variant="tonal"
            density="compact"
            class="health-check-dialog__alert"
          >
            {{ t(`instances.health.grades.${report.overall}`) }}
          </v-alert>
          <div class="health-check-dialog__checks">
            <div
              v-for="check in report.checks"
              :key="check.key"
              class="health-check-dialog__check"
            >
              <v-icon :color="checkColor(check)" size="20">
                {{
                  check.status === "pass"
                    ? "mdi-check-circle-outline"
                    : check.status === "skipped"
                      ? "mdi-minus-circle-outline"
                      : "mdi-alert-circle-outline"
                }}
              </v-icon>
              <div class="health-check-dialog__check-copy">
                <div class="health-check-dialog__check-title">
                  {{ t(`instances.health.checks.${check.key}`) }}
                </div>
                <div class="health-check-dialog__check-detail">
                  {{ checkDetail(check) }}
                </div>
              </div>
              <v-chip
                size="small"
                variant="tonal"
                :color="checkColor(check)"
              >
                {{ t(`instances.health.status.${check.status}`) }}
              </v-chip>
            </div>
          </div>
        </template>
      </v-card-text>
      <v-card-actions class="health-check-dialog__actions">
        <v-btn
          variant="text"
          :disabled="loading"
          @click="emit('retry')"
        >
          {{ t("common.actions.refresh") }}
        </v-btn>
        <v-spacer />
        <v-btn
          variant="text"
          @click="emit('update:modelValue', false)"
        >
          {{ t("common.actions.close") }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.health-check-dialog__title {
  padding: 20px 20px 0;
  font-size: 1.05rem;
  font-weight: 600;
}

.health-check-dialog__body {
  padding: 16px 20px 8px;
}

.health-check-dialog__progress {
  margin-bottom: 12px;
}

.health-check-dialog__alert {
  margin-bottom: 12px;
}

.health-check-dialog__checks {
  display: grid;
  gap: 2px;
}

.health-check-dialog__check {
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 56px;
  border-bottom: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
}

.health-check-dialog__check:last-child {
  border-bottom: 0;
}

.health-check-dialog__check-copy {
  flex: 1;
  min-width: 0;
}

.health-check-dialog__check-title {
  font-size: 0.9rem;
  font-weight: 500;
}

.health-check-dialog__check-detail {
  color: rgba(var(--v-theme-on-surface), 0.64);
  font-size: 0.8rem;
  line-height: 1.4;
  overflow-wrap: anywhere;
}

.health-check-dialog__actions {
  padding: 12px 16px 16px;
}
</style>
