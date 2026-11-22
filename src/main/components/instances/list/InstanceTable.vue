<script setup lang="ts">
import { computed } from "vue";
import type { DataTableHeader } from "vuetify";
import { useI18n } from "vue-i18n";
import StatusChip from "../../common/StatusChip.vue";
import InstanceStatusCell from "../InstanceStatusCell.vue";
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
  loading: boolean;
  ctx: InstanceRowContext;
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

const headers = computed<DataTableHeader[]>(() => [
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
    width: 220,
  },
  {
    title: t("instances.list.columns.actions"),
    key: "actions",
    sortable: false,
    align: "end",
    minWidth: 240,
  },
]);
</script>

<template>
  <div class="page-table-card">
    <v-data-table
      :headers="headers"
      :items="rows"
      item-value="id"
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
              v-else
              class="instance-meta page-muted page-nums"
              :aria-label="
                t('instances.list.meta', {
                  version: item.version,
                  port:
                    item.state === 'running' ? item.port : item.configured_port,
                })
              "
            >
              <v-icon size="12" icon="mdi-tag-outline" aria-hidden="true" />
                <span>{{ item.version }}</span>
                <span aria-hidden="true">·</span>
                <v-icon size="12" icon="mdi-ethernet" aria-hidden="true" />
                <span>
                  {{
                    item.state === "running"
                      ? item.port
                      : item.configured_port
                  }}
                </span>
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

      <template #item.actions="{ item }">
        <div v-if="item.external" class="instance-actions page-row">
          <v-tooltip location="top">
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                icon="mdi-monitor-dashboard"
                variant="text"
                size="small"
                :aria-label="t('instances.actions.openWebui')"
                :disabled="item.webui_url === null"
                @click="emit('openExternalWebui', item)"
              />
            </template>
            {{ t("instances.actions.openWebui") }}
          </v-tooltip>
          <v-tooltip location="top">
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                icon="mdi-open-in-new"
                variant="text"
                size="small"
                :aria-label="t('instances.actions.openInBrowser')"
                :disabled="item.webui_url === null"
                @click="emit('openExternalInBrowser', item)"
              />
            </template>
            {{ t("instances.actions.openInBrowser") }}
          </v-tooltip>
        </div>
        <div v-else class="instance-actions page-row">
          <template v-if="item.state === 'stopped'">
            <v-tooltip location="top">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-play"
                  color="success"
                  variant="tonal"
                  size="small"
                  :aria-label="
                    ctx.portConflictOf(item) !== null
                      ? t('instances.actions.portConflict', {
                          port: ctx.portConflictOf(item),
                        })
                      : t('instances.actions.start')
                  "
                  :disabled="
                    ctx.isInstanceInteractionDisabled(item.id) ||
                    ctx.portConflictOf(item) !== null
                  "
                  :loading="ctx.isBusy(ctx.startKey(item.id))"
                  @click="emit('start', item)"
                />
              </template>
              {{
                ctx.portConflictOf(item) !== null
                  ? t("instances.actions.portConflict", {
                      port: ctx.portConflictOf(item),
                    })
                  : t("instances.actions.start")
              }}
            </v-tooltip>
          </template>
          <template v-else-if="item.state === 'running'">
            <v-tooltip location="top">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-stop"
                  color="warning"
                  variant="text"
                  size="small"
                  :aria-label="t('instances.actions.stop')"
                  :disabled="ctx.isInstanceInteractionDisabled(item.id)"
                  :loading="ctx.isBusy(ctx.stopKey(item.id))"
                  @click="emit('stop', item)"
                />
              </template>
              {{ t("instances.actions.stop") }}
            </v-tooltip>
            <v-tooltip location="top">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-restart"
                  variant="text"
                  size="small"
                  :aria-label="t('instances.actions.restart')"
                  :disabled="ctx.isInstanceInteractionDisabled(item.id)"
                  :loading="ctx.isBusy(ctx.restartKey(item.id))"
                  @click="emit('restart', item)"
                />
              </template>
              {{ t("instances.actions.restart") }}
            </v-tooltip>
          </template>
          <v-tooltip v-else location="top">
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                icon="mdi-loading"
                variant="tonal"
                size="small"
                disabled
                loading
                :aria-label="
                  item.state === 'starting'
                    ? t('common.instanceState.starting')
                    : t('common.instanceState.stopping')
                "
              />
            </template>
            {{
              item.state === "starting"
                ? t("common.instanceState.starting")
                : t("common.instanceState.stopping")
            }}
          </v-tooltip>

          <v-tooltip
            v-if="ctx.updateInfoFor(item.id)?.state === 'available'"
            location="top"
          >
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                icon="mdi-arrow-up-bold-circle"
                color="primary"
                variant="text"
                size="small"
                :aria-label="t('instances.actions.upgrade')"
                :disabled="
                  item.state !== 'stopped' ||
                  ctx.isInstanceInteractionDisabled(item.id)
                "
                :loading="ctx.isBusy(ctx.upgradeKey(item.id))"
                :title="
                  item.state !== 'stopped'
                    ? t('instances.updates.stopFirst')
                    : undefined
                "
                @click="emit('upgrade', item)"
              />
            </template>
            {{ t("instances.actions.upgrade") }}
          </v-tooltip>

          <v-tooltip location="top">
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                icon="mdi-monitor-dashboard"
                variant="text"
                size="small"
                :aria-label="t('instances.actions.openWebui')"
                :disabled="!item.dashboard_enabled"
                @click="emit('openWebui', item)"
              />
            </template>
            {{ t("instances.actions.openWebui") }}
          </v-tooltip>

          <v-tooltip location="top">
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                icon="mdi-text-box-outline"
                variant="text"
                size="small"
                :aria-label="t('instances.actions.viewLogs')"
                @click="emit('openLogs', item)"
              />
            </template>
            {{ t("instances.actions.viewLogs") }}
          </v-tooltip>

          <v-tooltip v-if="item.state === 'running'" location="top">
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                icon="mdi-chart-timeline-variant"
                variant="text"
                size="small"
                :aria-label="t('resources.open')"
                @click="emit('openResources', item)"
              />
            </template>
            {{ t("resources.open") }}
          </v-tooltip>

          <v-menu location="bottom end" :disabled="ctx.batchUpgradeRunning">
            <template #activator="{ props }">
              <v-btn
                v-bind="props"
                icon="mdi-dots-horizontal"
                variant="text"
                size="small"
                :aria-label="t('instances.list.aria.moreActions')"
              />
            </template>
            <v-list density="compact" class="instance-menu">
              <v-list-item
                prepend-icon="mdi-folder-open-outline"
                :title="t('instances.actions.openCoreFolder')"
                @click="emit('openFolder', item)"
              />
              <v-list-item
                prepend-icon="mdi-pencil-outline"
                :title="t('common.actions.edit')"
                :disabled="
                  item.state !== 'stopped' ||
                  ctx.isInstanceInteractionDisabled(item.id)
                "
                @click="emit('edit', item)"
              />
              <v-list-item
                prepend-icon="mdi-database-cog-outline"
                :title="t('instances.actions.manageData')"
                @click="emit('manageData', item)"
              >
                <template #append>
                  <v-chip
                    size="x-small"
                    color="purple"
                    variant="tonal"
                  >
                    Beta
                  </v-chip>
                </template>
              </v-list-item>
              <v-list-item
                prepend-icon="mdi-archive-arrow-down-outline"
                :title="t('instances.actions.createBackup')"
                :disabled="
                  item.state !== 'stopped' ||
                  ctx.isInstanceInteractionDisabled(item.id)
                "
                @click="emit('backup', item)"
              />
              <v-list-item
                prepend-icon="mdi-lifebuoy"
                :title="t('instances.actions.repair')"
                :disabled="
                  item.state !== 'stopped' ||
                  ctx.isInstanceInteractionDisabled(item.id)
                "
                @click="emit('repair', item)"
              />
              <v-list-item
                prepend-icon="mdi-heart-pulse"
                :title="t('instances.actions.healthCheck')"
                :disabled="
                  ctx.healthLoading ||
                  ctx.isInstanceInteractionDisabled(item.id)
                "
                @click="emit('healthCheck', item)"
              />
              <v-list-item
                prepend-icon="mdi-text-box-outline"
                :title="t('instances.actions.viewLogs')"
                @click="emit('openLogs', item)"
              />
              <v-list-item
                prepend-icon="mdi-chart-timeline-variant"
                :title="t('resources.title')"
                @click="emit('openResources', item)"
              />
              <v-list-item
                prepend-icon="mdi-open-in-new"
                :title="t('instances.actions.openInBrowser')"
                :disabled="!item.dashboard_enabled"
                @click="emit('openInBrowser', item)"
              />
              <v-divider />
              <v-list-item
                prepend-icon="mdi-database-remove-outline"
                :title="t('instances.actions.clearData')"
                :disabled="
                  item.state !== 'stopped' ||
                  ctx.isInstanceInteractionDisabled(item.id)
                "
                @click="emit('clear', item, 'clear-data')"
              />
              <v-list-item
                prepend-icon="mdi-delete-sweep-outline"
                :title="t('instances.actions.clearVenv')"
                :disabled="
                  item.state !== 'stopped' ||
                  ctx.isInstanceInteractionDisabled(item.id)
                "
                @click="emit('clear', item, 'clear-venv')"
              />
              <v-list-item
                prepend-icon="mdi-broom"
                :title="t('instances.actions.clearPycache')"
                :disabled="
                  item.state !== 'stopped' ||
                  ctx.isInstanceInteractionDisabled(item.id)
                "
                @click="emit('clear', item, 'clear-pycache')"
              />
              <v-divider />
              <v-list-item
                prepend-icon="mdi-delete-outline"
                :title="t('instances.actions.deleteInstance')"
                base-color="error"
                :disabled="
                  item.state !== 'stopped' ||
                  ctx.isInstanceInteractionDisabled(item.id)
                "
                @click="emit('delete', item)"
              />
            </v-list>
          </v-menu>
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

.instance-menu {
  min-width: 208px;
}
</style>
