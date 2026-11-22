<script setup lang="ts">
import { useI18n } from "vue-i18n";
import StatusChip from "../../common/StatusChip.vue";
import InstanceStatusCell from "../InstanceStatusCell.vue";
import InstanceRowActions from "./InstanceRowActions.vue";
import type { InstanceStatus } from "../../../types/launcher";
import type {
  ExternalInstanceRow,
  InstanceRow,
} from "../../../composables/useInstanceList";
import type {
  ClearKind,
  InstanceRowContext,
} from "./instanceRowContext";

defineProps<{
  rows: InstanceRow[];
  ctx: InstanceRowContext;
  loading: boolean;
}>();

const emit = defineEmits<{
  start: [instance: InstanceStatus];
  stop: [instance: InstanceStatus];
  restart: [instance: InstanceStatus];
  upgrade: [instance: InstanceStatus];
  openWebui: [instance: InstanceStatus];
  openLogs: [instance: InstanceStatus];
  openResources: [instance: InstanceStatus];
  openFolder: [instance: InstanceStatus];
  edit: [instance: InstanceStatus];
  manageData: [instance: InstanceStatus];
  backup: [instance: InstanceStatus];
  repair: [instance: InstanceStatus];
  healthCheck: [instance: InstanceStatus];
  openInBrowser: [instance: InstanceStatus];
  clear: [instance: InstanceStatus, kind: ClearKind];
  delete: [instance: InstanceStatus];
  openExternalWebui: [row: ExternalInstanceRow];
  openExternalInBrowser: [row: ExternalInstanceRow];
}>();

const { t } = useI18n();
</script>

<template>
  <div class="instance-cards">
    <v-progress-linear
      v-if="loading"
      class="instance-cards__loading"
      indeterminate
      color="primary"
      height="5"
      rounded
      :aria-label="t('instances.loading')"
    />
    <div v-for="row in rows" :key="row.id" class="page-card instance-card">
      <div class="instance-card__header">
        <v-tooltip location="top">
          <template #activator="{ props: tooltipProps }">
            <v-icon
              v-bind="tooltipProps"
              size="22"
              class="page-muted"
              :icon="ctx.sourceIconOf(ctx.rowSourceOf(row))"
              :aria-label="ctx.sourceTitleOf(ctx.rowSourceOf(row))"
            />
          </template>
          {{ ctx.sourceTitleOf(ctx.rowSourceOf(row)) }}
        </v-tooltip>
        <span class="instance-card__name" :title="row.name">{{ row.name }}</span>
        <StatusChip
          v-if="row.external"
          :state="row.state"
        />
        <InstanceStatusCell v-else :instance="row" />
      </div>

      <div v-if="row.external" class="instance-meta page-muted page-nums">
        <template
          v-for="(part, index) in ctx.externalMetaParts(row)"
          :key="part.icon"
        >
          <span
            v-if="index > 0"
            aria-hidden="true"
          >·</span>
          <v-icon size="12" :icon="part.icon" aria-hidden="true" />
          <span>{{ part.text }}</span>
        </template>
      </div>
      <div
        v-else
        class="instance-meta page-muted page-nums"
        :aria-label="
          t('instances.list.meta', {
            version: row.version,
            port: row.state === 'running' ? row.port : row.configured_port,
          })
        "
      >
        <v-icon size="12" icon="mdi-tag-outline" aria-hidden="true" />
        <span>{{ row.version }}</span>
        <span aria-hidden="true">·</span>
        <v-icon size="12" icon="mdi-ethernet" aria-hidden="true" />
        <span>
          {{ row.state === "running" ? row.port : row.configured_port }}
        </span>
      </div>

      <div class="instance-card__actions">
        <InstanceRowActions
          :row="row"
          :ctx="ctx"
          @start="(instance) => emit('start', instance)"
          @stop="(instance) => emit('stop', instance)"
          @restart="(instance) => emit('restart', instance)"
          @upgrade="(instance) => emit('upgrade', instance)"
          @open-webui="(instance) => emit('openWebui', instance)"
          @open-logs="(instance) => emit('openLogs', instance)"
          @open-resources="(instance) => emit('openResources', instance)"
          @open-folder="(instance) => emit('openFolder', instance)"
          @edit="(instance) => emit('edit', instance)"
          @manage-data="(instance) => emit('manageData', instance)"
          @backup="(instance) => emit('backup', instance)"
          @repair="(instance) => emit('repair', instance)"
          @health-check="(instance) => emit('healthCheck', instance)"
          @open-in-browser="(instance) => emit('openInBrowser', instance)"
          @clear="(instance, kind) => emit('clear', instance, kind)"
          @delete="(instance) => emit('delete', instance)"
          @open-external-webui="(externalRow) => emit('openExternalWebui', externalRow)"
          @open-external-in-browser="(externalRow) => emit('openExternalInBrowser', externalRow)"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
.instance-cards {
  position: relative;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(min(300px, 100%), 1fr));
  gap: 16px;
}

/*
 * Pinned in the toolbar's bottom margin: an absolutely positioned grid child
 * so a refresh does not push the cards down when the bar appears.
 */
.instance-cards__loading {
  position: absolute;
  top: -12px;
  left: 0;
  right: 0;
}

.instance-card {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
}

.instance-card__header {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.instance-card__name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.instance-card__actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: auto;
}

.instance-meta {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
</style>
