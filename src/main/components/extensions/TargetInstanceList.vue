<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { InstanceStatus } from "../../types/launcher";

const props = defineProps<{
  instances: InstanceStatus[];
  modelValue: string | null;
}>();

const emit = defineEmits<{
  (event: "update:modelValue", value: string): void;
}>();

const { t } = useI18n();

function isStopped(instance: InstanceStatus): boolean {
  return instance.state === "stopped";
}

function select(instance: InstanceStatus): void {
  if (isStopped(instance)) {
    emit("update:modelValue", instance.id);
  }
}
</script>

<template>
  <v-list class="target-instance-list" density="compact">
    <v-list-item
      v-for="instance in props.instances"
      :key="instance.id"
      :active="props.modelValue === instance.id"
      :disabled="!isStopped(instance)"
      class="target-instance-list__item"
      @click="select(instance)"
    >
      <div class="target-instance-list__row">
        <span class="page-anywhere">{{ instance.name }}</span>
        <span
          v-if="!isStopped(instance)"
          class="target-instance-list__reason"
        >
          {{ t("extensions.running") }}
        </span>
        <slot name="append" :instance="instance" />
      </div>
    </v-list-item>
  </v-list>
</template>

<style scoped>
.target-instance-list {
  border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
  border-radius: var(--astrbot-radius-control);
  padding: 0;
}

.target-instance-list__row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.target-instance-list__reason {
  flex: none;
  color: rgba(var(--v-theme-on-surface), 0.45);
  font-size: 0.75rem;
}
</style>
