<script setup lang="ts">
import { computed } from "vue";
import type { DataTableHeader } from "vuetify";
import { useI18n } from "vue-i18n";
import StatusChip from "../../common/StatusChip.vue";
import InstanceStatusCell from "../InstanceStatusCell.vue";
import InstanceRowActions from "./InstanceRowActions.vue";
import type { InstanceStatus } from "../../../types/launcher";
import type {
  ExternalInstanceRow,
  InstanceColumnName,
  InstanceRow,
} from "../../../composables/useInstanceList";
import { externalRunningPort } from "../../../composables/useInstanceList";
import type {
  ClearKind,
  InstanceRowContext,
} from "./instanceRowContext";

const props = defineProps<{
  rows: InstanceRow[];
  loading: boolean;
  ctx: InstanceRowContext;
  hiddenColumns: InstanceColumnName[];
}>();

const selected = defineModel<string[]>("selected", { required: true });

/** External rows cannot be started or stopped, so they never join batch selection. */
function isSelectable(item: InstanceRow): boolean {
  return !item.external;
}

/* Meta parts under the name duplicate the optional columns, so each part is shown only while its column is hidden. */
const showVersionMeta = computed(() => props.hiddenColumns.includes("version"));
const showPortMeta = computed(() => props.hiddenColumns.includes("port"));

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

const headers = computed<DataTableHeader[]>(() => {
  const visible: DataTableHeader[] = [
    {
      title: t("instances.list.columns.name"),
      key: "name",
      sortable: false,
      minWidth: 200,
    },
    {
      title: t("instances.list.columns.state"),
      key: "state",
      sortable: false,
      width: 170,
    },
  ];
  if (!props.hiddenColumns.includes("version")) {
    visible.push({
      title: t("instances.list.columns.version"),
      key: "version",
      sortable: false,
      minWidth: 130,
    });
  }
  if (!props.hiddenColumns.includes("port")) {
    visible.push({
      title: t("instances.list.columns.port"),
      key: "port",
      sortable: false,
      minWidth: 110,
    });
  }
  visible.push({
    title: t("instances.list.columns.actions"),
    key: "actions",
    sortable: false,
    align: "end",
    minWidth: 240,
  });
  return visible;
});
</script>

<template>
  <div class="page-table-card">
    <v-data-table
      v-model="selected"
      :headers="headers"
      :items="rows"
      :item-selectable="isSelectable"
      item-value="id"
      show-select
      :loading="loading"
      hide-default-footer
      :items-per-page="-1"
      disable-sort
      hover
      :no-data-text="t('instances.list.empty')"
    >
      <template #item.name="{ item }">
        <div class="page-cell instance-name-cell">
          <v-tooltip location="top">
            <template #activator="{ props: tooltipProps }">
              <v-icon
                v-bind="tooltipProps"
                size="22"
                class="page-muted"
                :icon="ctx.sourceIconOf(ctx.rowSourceOf(item))"
                :aria-label="ctx.sourceTitleOf(ctx.rowSourceOf(item))"
              />
            </template>
            {{ ctx.sourceTitleOf(ctx.rowSourceOf(item)) }}
          </v-tooltip>
          <div class="page-cell">
            <span>{{ item.name }}</span>
            <span
              v-if="item.external"
              class="instance-meta page-muted page-nums"
            >
              <template
                v-for="(part, index) in ctx.externalMetaParts(item)"
                :key="part.icon"
              >
                <span
                  v-if="index > 0"
                  aria-hidden="true"
                >·</span>
                <v-icon size="12" :icon="part.icon" aria-hidden="true" />
                <span>{{ part.text }}</span>
              </template>
            </span>
            <span
              v-else-if="showVersionMeta || showPortMeta"
              class="instance-meta page-muted page-nums"
            >
              <template v-if="showVersionMeta">
                <v-icon
                  size="12"
                  icon="mdi-tag-outline"
                  aria-hidden="true"
                />
                <span>{{ item.version }}</span>
                <span
                  v-if="showPortMeta"
                  aria-hidden="true"
                >·</span>
              </template>
              <template v-if="showPortMeta">
                <v-icon size="12" icon="mdi-ethernet" aria-hidden="true" />
                <span>
                  {{
                    item.state === "running"
                      ? item.port
                      : item.configured_port
                  }}
                </span>
              </template>
            </span>
          </div>
        </div>
      </template>

      <template #item.state="{ item }">
        <StatusChip
          v-if="item.external"
          :state="item.state"
        />
        <InstanceStatusCell v-else :instance="item" />
      </template>

      <template #item.version="{ item }">
        <span v-if="item.external" class="page-cell page-muted">—</span>
        <span v-else class="page-cell page-nums">{{ item.version }}</span>
      </template>

      <template #item.port="{ item }">
        <span v-if="item.external" class="page-cell page-nums">{{
          externalRunningPort(item) ?? "—"
        }}</span>
        <span v-else class="page-cell page-nums">{{
          item.state === "running" ? item.port : item.configured_port
        }}</span>
      </template>

      <template #item.actions="{ item }">
        <div class="instance-actions page-row">
          <InstanceRowActions
            :row="item"
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
      </template>
    </v-data-table>
  </div>
</template>

<style scoped>
.instance-actions {
  justify-content: flex-end;
  gap: 4px;
}

.instance-meta {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

/* Icon column on the left, name + meta stacked on the right. */
.instance-name-cell {
  flex-direction: row;
  align-items: center;
  gap: 10px;
}
</style>
