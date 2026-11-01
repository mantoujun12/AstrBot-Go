<script setup lang="ts">
import type { LockFailure } from "../../services/errors";

const props = withDefaults(
  defineProps<{
    modelValue: boolean;
    failure: LockFailure | null;
    loading?: boolean;
  }>(),
  { loading: false },
);

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
  (event: "continue"): void;
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
    max-width="520"
    :persistent="loading"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card>
      <v-card-title class="lock-dialog__title">文件被占用</v-card-title>
      <v-card-text class="lock-dialog__body">
        <p class="lock-dialog__detail">
          {{ failure?.detail ?? "目标路径被占用，请关闭相关进程后重试" }}
        </p>
        <ul v-if="failure && failure.processes.length > 0" class="lock-dialog__list">
          <li v-for="process in failure.processes" :key="process">
            {{ process }}
          </li>
        </ul>
      </v-card-text>
      <v-card-actions class="lock-dialog__actions">
        <v-spacer />
        <v-btn variant="text" :disabled="loading" @click="close">取消</v-btn>
        <v-btn
          v-if="failure?.canContinue"
          color="warning"
          variant="flat"
          :loading="loading"
          @click="emit('continue')"
        >
          仍然继续
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.lock-dialog__title {
  padding: 20px 20px 0;
  font-size: 1.05rem;
  font-weight: 650;
}

.lock-dialog__body {
  padding: 10px 20px 0;
}

.lock-dialog__detail {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.72);
  font-size: 0.875rem;
  line-height: 1.6;
}

.lock-dialog__list {
  margin: 12px 0 0;
  padding: 10px 14px;
  border-radius: 8px;
  background: rgba(var(--v-theme-on-surface), 0.05);
  color: rgba(var(--v-theme-on-surface), 0.7);
  font-family: "Consolas", "Cascadia Mono", monospace;
  font-size: 0.78rem;
  line-height: 1.6;
  list-style: disc inside;
  max-height: 180px;
  overflow-y: auto;
}

.lock-dialog__actions {
  padding: 14px 16px 16px;
}
</style>
