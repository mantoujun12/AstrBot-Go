<script setup lang="ts">
import { computed } from "vue";
import type { DownloadProgress } from "../../types/launcher";
import { formatFileSize } from "../../utils/format";

const props = defineProps<{
  progress: DownloadProgress | null;
}>();

const percent = computed(() => {
  const value = props.progress?.progress;
  return value === null || value === undefined ? null : Math.round(value);
});

const sizeText = computed(() => {
  const progress = props.progress;
  if (!progress || progress.downloaded <= 0) {
    return "";
  }
  const downloaded = formatFileSize(progress.downloaded);
  return progress.total
    ? `${downloaded} / ${formatFileSize(progress.total)}`
    : downloaded;
});

const speedText = computed(() => {
  const speed = props.progress?.speed ?? 0;
  return speed > 0 ? `${formatFileSize(speed)}/s` : "";
});
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
      <span class="download-progress__stats">
        <span v-if="sizeText">{{ sizeText }}</span>
        <span v-if="speedText" class="download-progress__speed">{{ speedText }}</span>
        <span v-if="percent !== null">{{ percent }}%</span>
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

.download-progress__stats {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  justify-content: flex-end;
  font-variant-numeric: tabular-nums;
}

.download-progress__speed {
  color: rgb(var(--v-theme-primary));
}
</style>
