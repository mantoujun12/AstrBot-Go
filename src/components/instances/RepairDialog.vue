<script setup lang="ts">
import { ref, watch } from "vue";
import type { RepairPreserveScope } from "../../types/launcher";

const props = withDefaults(
  defineProps<{
    modelValue: boolean;
    loading?: boolean;
  }>(),
  { loading: false },
);

const emit = defineEmits<{
  (event: "update:modelValue", value: boolean): void;
  (event: "submit", scope: RepairPreserveScope): void;
}>();

interface ScopeOption {
  title: string;
  value: RepairPreserveScope;
}

const scopeOptions: ScopeOption[] = [
  { title: "保留数据目录", value: "data_directory" },
  { title: "保留配置与数据库", value: "config_and_data_files" },
  { title: "保留核心配置与数据库", value: "core_config_and_data_files" },
  { title: "仅保留数据库", value: "database_only" },
];

const scope = ref<RepairPreserveScope>("data_directory");

function close(): void {
  if (props.loading) {
    return;
  }
  emit("update:modelValue", false);
}

watch(
  () => props.modelValue,
  (open) => {
    if (open) {
      scope.value = "data_directory";
    }
  },
);
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    max-width="460"
    :persistent="loading"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card>
      <v-card-title class="repair-dialog__title">修复实例</v-card-title>
      <v-card-text class="repair-dialog__body">
        <p class="repair-dialog__hint">选择保留范围，实例将重新部署。</p>
        <v-radio-group v-model="scope" hide-details>
          <v-radio
            v-for="option in scopeOptions"
            :key="option.value"
            :label="option.title"
            :value="option.value"
          />
        </v-radio-group>
      </v-card-text>
      <v-card-actions class="repair-dialog__actions">
        <v-spacer />
        <v-btn variant="text" :disabled="loading" @click="close">取消</v-btn>
        <v-btn
          color="warning"
          variant="flat"
          :loading="loading"
          @click="emit('submit', scope)"
        >
          修复
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.repair-dialog__title {
  padding: 20px 20px 0;
  font-size: 1.05rem;
  font-weight: 650;
}

.repair-dialog__body {
  padding: 12px 20px 4px;
}

.repair-dialog__hint {
  margin: 0 0 8px;
  color: rgba(var(--v-theme-on-surface), 0.64);
  font-size: 0.85rem;
  line-height: 1.5;
}

.repair-dialog__actions {
  padding: 14px 16px 16px;
}
</style>
