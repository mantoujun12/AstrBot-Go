<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: boolean;
    title: string;
    message?: string;
    progress?: number | null;
  }>(),
  {
    message: "",
    progress: null,
  },
);
</script>

<template>
  <v-dialog :model-value="modelValue" persistent max-width="420">
    <v-card>
      <v-card-title class="task-progress__title">{{ title }}</v-card-title>
      <v-card-text class="task-progress__body">
        <div class="task-progress__message">
          {{ message || "处理中…" }}
        </div>
        <v-progress-linear
          :model-value="progress ?? 0"
          :indeterminate="progress === null"
          color="primary"
          height="6"
          rounded
        />
        <div v-if="progress !== null" class="task-progress__percent">
          {{ Math.round(progress) }}%
        </div>
      </v-card-text>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.task-progress__title {
  padding: 20px 20px 0;
  font-size: 1.05rem;
  font-weight: 650;
  white-space: normal;
}

.task-progress__body {
  padding: 12px 20px 20px;
}

.task-progress__message {
  margin-bottom: 12px;
  color: rgba(var(--v-theme-on-surface), 0.72);
  font-size: 0.875rem;
  line-height: 1.5;
}

.task-progress__percent {
  margin-top: 8px;
  color: rgba(var(--v-theme-on-surface), 0.56);
  font-size: 0.75rem;
  text-align: right;
}
</style>
