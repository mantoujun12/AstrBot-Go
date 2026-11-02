<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import {
  BACKUP_NOTE_MAX_LENGTH,
  countBackupNoteChars,
} from "../../utils/backupNote";

const props = withDefaults(
  defineProps<{
    modelValue: string;
    disabled?: boolean;
  }>(),
  { disabled: false },
);

const emit = defineEmits<{
  (event: "update:modelValue", value: string): void;
}>();

const { t } = useI18n();

const tooLong = computed(
  () => countBackupNoteChars(props.modelValue) > BACKUP_NOTE_MAX_LENGTH,
);

function update(value: string): void {
  emit("update:modelValue", value);
}
</script>

<template>
  <v-text-field
    :model-value="modelValue"
    :label="t('backups.createDialog.noteLabel')"
    :placeholder="t('backups.createDialog.notePlaceholder')"
    :counter="BACKUP_NOTE_MAX_LENGTH"
    :counter-value="countBackupNoteChars"
    :error="tooLong"
    :error-messages="
      tooLong
        ? t('backups.createDialog.noteTooLong', {
            max: BACKUP_NOTE_MAX_LENGTH,
          })
        : []
    "
    variant="outlined"
    density="compact"
    hide-details="auto"
    :disabled="disabled"
    @update:model-value="update"
  />
</template>
