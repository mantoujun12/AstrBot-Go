<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { InstanceStatus } from "../../types/launcher";
import {
  BACKUP_NOTE_MAX_LENGTH,
  countBackupNoteChars,
} from "../../utils/backupNote";
import BackupNoteField from "./BackupNoteField.vue";

const props = withDefaults(
  defineProps<{
    modelValue: boolean;
    instances: InstanceStatus[];
    loading?: boolean;
  }>(),
  { loading: false },
);

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
  (event: "submit", instanceId: string, note: string): void;
}>();

const { t } = useI18n();
const selectedId = ref<string | null>(null);
const note = ref("");

const noteTooLong = computed(
  () => countBackupNoteChars(note.value) > BACKUP_NOTE_MAX_LENGTH,
);

const stoppedInstances = computed(() =>
  props.instances.filter((instance) => instance.state === "stopped"),
);

const emptyReason = computed(() =>
  props.instances.length === 0
    ? t("backups.createDialog.noInstances")
    : t("backups.createDialog.noStoppedInstances"),
);

watch(
  () => props.modelValue,
  (open) => {
    if (open) {
      selectedId.value = null;
      note.value = "";
    }
  },
);

watch(stoppedInstances, (stopped) => {
  if (
    selectedId.value !== null &&
    !stopped.some((instance) => instance.id === selectedId.value)
  ) {
    selectedId.value = null;
  }
});

function updateOpen(value: boolean): void {
  if (!value && props.loading) {
    return;
  }
  emit("update:modelValue", value);
}

function close(): void {
  updateOpen(false);
}

function submit(): void {
  if (props.loading || noteTooLong.value) {
    return;
  }
  const instance = stoppedInstances.value.find(
    (candidate) => candidate.id === selectedId.value,
  );
  if (!instance) {
    return;
  }
  emit("submit", instance.id, note.value.trim());
}
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    max-width="420"
    :persistent="loading"
    @update:model-value="updateOpen"
  >
    <v-card>
      <v-card-title class="create-backup__title">
        {{ t("backups.createDialog.title") }}
      </v-card-title>
      <v-card-text class="create-backup__body">
        <v-alert
          v-if="stoppedInstances.length === 0"
          type="warning"
          variant="tonal"
          density="compact"
        >
          {{ emptyReason }}
        </v-alert>
        <v-select
          v-else
          v-model="selectedId"
          :items="stoppedInstances"
          item-title="name"
          item-value="id"
          :label="t('backups.createDialog.instanceLabel')"
          variant="outlined"
          density="compact"
          hide-details
          :disabled="loading"
        />
        <BackupNoteField
          v-if="stoppedInstances.length > 0"
          v-model="note"
          :disabled="loading"
          class="create-backup__note"
        />
      </v-card-text>
      <v-card-actions class="create-backup__actions">
        <v-spacer />
        <v-btn variant="text" :disabled="loading" @click="close">
          {{ t("common.actions.cancel") }}
        </v-btn>
        <v-btn
          color="primary"
          variant="flat"
          :loading="loading"
          :disabled="
            stoppedInstances.length === 0 ||
            selectedId === null ||
            noteTooLong
          "
          @click="submit"
        >
          {{ t("common.actions.create") }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.create-backup__title {
  padding: 20px 20px 0;
  font-size: 1.05rem;
  font-weight: 600;
}

.create-backup__body {
  padding: 14px 20px 0;
}

.create-backup__note {
  margin-top: 12px;
}

.create-backup__actions {
  padding: 16px;
}
</style>
