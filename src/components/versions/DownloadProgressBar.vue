<script setup lang="ts">
import type { DownloadProgress } from "../../types/launcher";

const props = defineProps<{
  progress: DownloadProgress | null;
}>();
</script>

<template>
  <div v-if="props.progress" class="download-progress" role="status">
    <v-progress-linear
      :model-value="props.progress.progress ?? 0"
      :indeterminate="props.progress.progress === null"
      color="primary"
      height="5"
      rounded
    />
    <div class="download-progress__meta">
      <span>{{ props.progress.message || props.progress.step }}</span>
      <span v-if="props.progress.progress !== null">
        {{ Math.round(props.progress.progress) }}%
      </span>
    </div>
  </div>
</template>

<style scoped>
.download-progress {
  max-width: 460px;
  margin-top: 8px;
}

.download-progress__meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 6px;
  color: rgba(var(--v-theme-on-surface), 0.56);
  font-size: 0.75rem;
  line-height: 1.4;
}
</style>
