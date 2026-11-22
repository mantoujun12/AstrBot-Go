<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

const props = withDefaults(
  defineProps<{
    modelValue: boolean;
    title: string;
    message?: string;
    confirmLabel?: string;
    cancelLabel?: string;
    color?: string;
    loading?: boolean;
    confirmDisabled?: boolean;
    maxWidth?: number | string;
  }>(),
  {
    message: "",
    color: "primary",
    loading: false,
    confirmDisabled: false,
    maxWidth: 440,
  },
);

const { t } = useI18n();

const confirmText = computed(
  () => props.confirmLabel || t("common.actions.confirm"),
);
const cancelText = computed(
  () => props.cancelLabel || t("common.actions.cancel"),
);

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
  (event: "confirm"): void;
}>();

function close(): void {
  if (props.loading) {
    return;
  }
  emit("update:modelValue", false);
}
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    :max-width="maxWidth"
    :persistent="loading"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card>
      <v-card-title class="confirm-dialog__title">{{ title }}</v-card-title>
      <v-card-text v-if="message" class="confirm-dialog__message">
        {{ message }}
      </v-card-text>
      <v-card-text v-if="$slots.default" class="confirm-dialog__body">
        <slot />
      </v-card-text>
      <v-card-actions class="confirm-dialog__actions">
        <v-spacer />
        <v-btn variant="text" :disabled="loading" @click="close">
          {{ cancelText }}
        </v-btn>
        <v-btn
          :color="color"
          variant="flat"
          :loading="loading"
          :disabled="confirmDisabled"
          @click="emit('confirm')"
        >
          {{ confirmText }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.confirm-dialog__title {
  padding: 20px 20px 0;
  font-size: 1.05rem;
  font-weight: 600;
  white-space: normal;
}

.confirm-dialog__message {
  padding: 10px 20px 0;
  color: rgba(var(--v-theme-on-surface), 0.72);
  font-size: 0.875rem;
  line-height: 1.6;
  white-space: pre-wrap;
}

.confirm-dialog__body {
  padding: 12px 20px 0;
}

.confirm-dialog__actions {
  padding: 14px 16px 16px;
}
</style>
