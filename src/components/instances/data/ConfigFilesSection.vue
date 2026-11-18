<script setup lang="ts">
import { useI18n } from "vue-i18n";
import EmptyState from "../../common/EmptyState.vue";
import type { InstanceConfigFile } from "../../../types/instanceData";
import { formatDateTime, formatFileSize } from "../../../utils/format";

defineProps<{
  files: InstanceConfigFile[];
  selectedName: string | null;
}>();

const emit = defineEmits<{
  (event: "select", name: string): void;
  (event: "edit", name: string): void;
  (event: "copy", name: string): void;
}>();

const { t } = useI18n();

function modifiedLabel(file: InstanceConfigFile): string {
  return file.modified_ms === null
    ? "—"
    : formatDateTime(new Date(file.modified_ms).toISOString());
}
</script>

<template>
  <div class="config-files">
    <EmptyState
      v-if="files.length === 0"
      :text="t('instanceData.configFiles.empty')"
      icon="mdi-file-cog-outline"
    />
    <div v-else class="page-table-card">
      <div
        v-for="file in files"
        :key="file.name"
        class="config-file"
        :class="{ 'config-file--active': file.name === selectedName }"
        role="button"
        tabindex="0"
        @click="emit('select', file.name)"
        @keydown.enter.prevent="emit('select', file.name)"
      >
        <div class="page-cell config-file__body">
          <span class="config-file__name page-mono page-anywhere">
            {{ file.name }}
          </span>
          <span class="page-muted page-nums">
            {{ formatFileSize(file.size) }}
            <span aria-hidden="true">·</span>
            {{ modifiedLabel(file) }}
          </span>
        </div>
        <div class="page-row config-file__actions">
          <v-tooltip location="top">
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                icon="mdi-file-edit-outline"
                variant="text"
                size="small"
                :aria-label="t('instanceData.editConfigFile')"
                @click.stop="emit('edit', file.name)"
              />
            </template>
            {{ t("instanceData.editConfigFile") }}
          </v-tooltip>
          <v-tooltip location="top">
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                icon="mdi-content-duplicate"
                variant="text"
                size="small"
                :aria-label="t('instanceData.copy.title')"
                @click.stop="emit('copy', file.name)"
              />
            </template>
            {{ t("instanceData.copy.title") }}
          </v-tooltip>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.config-file {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 18px;
  cursor: pointer;
}

.config-file + .config-file {
  border-top: 1px solid rgba(var(--v-theme-on-surface), 0.08);
}

.config-file--active {
  background: rgba(var(--v-theme-primary), 0.08);
}

.config-file__body {
  min-width: 0;
}

.config-file__name {
  font-size: 0.9rem;
  font-weight: 500;
}

.config-file__actions {
  flex-shrink: 0;
  gap: 2px;
}
</style>
