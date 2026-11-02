<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
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

const { t } = useI18n();

const scopeOptions = computed<ScopeOption[]>(() => [
  { title: t("instances.repair.scopes.dataDirectory"), value: "data_directory" },
  {
    title: t("instances.repair.scopes.configAndDataFiles"),
    value: "config_and_data_files",
  },
  {
    title: t("instances.repair.scopes.coreConfigAndDataFiles"),
    value: "core_config_and_data_files",
  },
  { title: t("instances.repair.scopes.databaseOnly"), value: "database_only" },
]);

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
      <v-card-title class="repair-dialog__title">
        {{ t("instances.repair.title") }}
      </v-card-title>
      <v-card-text class="repair-dialog__body">
        <p class="repair-dialog__hint">{{ t("instances.repair.hint") }}</p>
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
        <v-btn variant="text" :disabled="loading" @click="close">
          {{ t("common.actions.cancel") }}
        </v-btn>
        <v-btn
          color="warning"
          variant="flat"
          :loading="loading"
          @click="emit('submit', scope)"
        >
          {{ t("instances.repair.submit") }}
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
