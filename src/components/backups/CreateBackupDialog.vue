<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { InstanceStatus } from "../../types/launcher";

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
  (event: "submit", instanceId: string): void;
}>();

const selectedId = ref<string | null>(null);

const stoppedInstances = computed(() =>
  props.instances.filter((instance) => instance.state === "stopped"),
);

const emptyReason = computed(() =>
  props.instances.length === 0 ? "暂无实例" : "没有已停止的实例",
);

watch(
  () => props.modelValue,
  (open) => {
    if (open) {
      selectedId.value = null;
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
  if (props.loading) {
    return;
  }
  const instance = stoppedInstances.value.find(
    (candidate) => candidate.id === selectedId.value,
  );
  if (!instance) {
    return;
  }
  emit("submit", instance.id);
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
      <v-card-title class="create-backup__title">创建备份</v-card-title>
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
          label="实例"
          variant="outlined"
          density="compact"
          hide-details
          :disabled="loading"
        />
      </v-card-text>
      <v-card-actions class="create-backup__actions">
        <v-spacer />
        <v-btn variant="text" :disabled="loading" @click="close">取消</v-btn>
        <v-btn
          color="primary"
          variant="flat"
          :loading="loading"
          :disabled="stoppedInstances.length === 0 || selectedId === null"
          @click="submit"
        >
          创建
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.create-backup__title {
  padding: 20px 20px 0;
  font-size: 1.05rem;
  font-weight: 650;
}

.create-backup__body {
  padding: 14px 20px 0;
}

.create-backup__actions {
  padding: 16px;
}
</style>
