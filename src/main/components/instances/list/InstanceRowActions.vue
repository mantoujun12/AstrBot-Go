<script setup lang="ts">
import { useI18n } from "vue-i18n";
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
  row: InstanceRow;
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
</script>

<template>
  <template v-if="row.external">
    <v-tooltip location="top">
      <template #activator="{ props: tooltipProps }">
        <v-btn
          v-bind="tooltipProps"
          icon="mdi-monitor-dashboard"
          variant="text"
          size="small"
          :aria-label="t('instances.actions.openWebui')"
          :disabled="row.webui_url === null"
          @click="emit('openExternalWebui', row)"
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
          :disabled="row.webui_url === null"
          @click="emit('openExternalInBrowser', row)"
        />
      </template>
      {{ t("instances.actions.openInBrowser") }}
    </v-tooltip>
  </template>
  <template v-else>
    <template v-if="row.state === 'stopped'">
      <v-tooltip location="top">
        <template #activator="{ props: tooltipProps }">
          <v-btn
            v-bind="tooltipProps"
            icon="mdi-play"
            color="success"
            variant="tonal"
            size="small"
            :aria-label="
              ctx.portConflictOf(row) !== null
                ? t('instances.actions.portConflict', {
                    port: ctx.portConflictOf(row),
                  })
                : t('instances.actions.start')
            "
            :disabled="
              ctx.isInstanceInteractionDisabled(row.id) ||
              ctx.portConflictOf(row) !== null
            "
            :loading="ctx.isBusy(ctx.startKey(row.id))"
            @click="emit('start', row)"
          />
        </template>
        {{
          ctx.portConflictOf(row) !== null
            ? t("instances.actions.portConflict", {
                port: ctx.portConflictOf(row),
              })
            : t("instances.actions.start")
        }}
      </v-tooltip>
    </template>
    <template v-else-if="row.state === 'running'">
      <v-tooltip location="top">
        <template #activator="{ props: tooltipProps }">
          <v-btn
            v-bind="tooltipProps"
            icon="mdi-stop"
            color="warning"
            variant="text"
            size="small"
            :aria-label="t('instances.actions.stop')"
            :disabled="ctx.isInstanceInteractionDisabled(row.id)"
            :loading="ctx.isBusy(ctx.stopKey(row.id))"
            @click="emit('stop', row)"
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
            :disabled="ctx.isInstanceInteractionDisabled(row.id)"
            :loading="ctx.isBusy(ctx.restartKey(row.id))"
            @click="emit('restart', row)"
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
            row.state === 'starting'
              ? t('common.instanceState.starting')
              : t('common.instanceState.stopping')
          "
        />
      </template>
      {{
        row.state === "starting"
          ? t("common.instanceState.starting")
          : t("common.instanceState.stopping")
      }}
    </v-tooltip>

    <v-tooltip
      v-if="ctx.updateInfoFor(row.id)?.state === 'available'"
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
            row.state !== 'stopped' ||
            ctx.isInstanceInteractionDisabled(row.id)
          "
          :loading="ctx.isBusy(ctx.upgradeKey(row.id))"
          :title="
            row.state !== 'stopped'
              ? t('instances.updates.stopFirst')
              : undefined
          "
          @click="emit('upgrade', row)"
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
          :disabled="!row.dashboard_enabled"
          @click="emit('openWebui', row)"
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
          @click="emit('openLogs', row)"
        />
      </template>
      {{ t("instances.actions.viewLogs") }}
    </v-tooltip>

    <v-tooltip v-if="row.state === 'running'" location="top">
      <template #activator="{ props: tooltipProps }">
        <v-btn
          v-bind="tooltipProps"
          icon="mdi-chart-timeline-variant"
          variant="text"
          size="small"
          :aria-label="t('resources.open')"
          @click="emit('openResources', row)"
        />
      </template>
      {{ t("resources.open") }}
    </v-tooltip>

    <v-menu location="bottom end" :disabled="ctx.batchUpgradeRunning">
      <template #activator="{ props }">
        <v-tooltip location="top">
          <template #activator="{ props: tooltipProps }">
            <v-btn
              v-bind="{ ...props, ...tooltipProps }"
              icon="mdi-dots-horizontal"
              variant="text"
              size="small"
              :aria-label="t('instances.list.aria.moreActions')"
            />
          </template>
          {{ t("instances.list.aria.moreActions") }}
        </v-tooltip>
      </template>
      <v-list density="compact" class="instance-menu">
        <v-list-item
          prepend-icon="mdi-folder-open-outline"
          :title="t('instances.actions.openCoreFolder')"
          @click="emit('openFolder', row)"
        />
        <v-list-item
          prepend-icon="mdi-pencil-outline"
          :title="t('common.actions.edit')"
          :disabled="
            row.state !== 'stopped' ||
            ctx.isInstanceInteractionDisabled(row.id)
          "
          @click="emit('edit', row)"
        />
        <v-list-item
          prepend-icon="mdi-database-cog-outline"
          :title="t('instances.actions.manageData')"
          @click="emit('manageData', row)"
        >
          <template #append>
            <v-chip
              size="x-small"
              color="purple"
              variant="tonal"
            >
              {{ t("instances.actions.beta") }}
            </v-chip>
          </template>
        </v-list-item>
        <v-list-item
          prepend-icon="mdi-archive-arrow-down-outline"
          :title="t('instances.actions.createBackup')"
          :disabled="
            row.state !== 'stopped' ||
            ctx.isInstanceInteractionDisabled(row.id)
          "
          @click="emit('backup', row)"
        />
        <v-list-item
          prepend-icon="mdi-lifebuoy"
          :title="t('instances.actions.repair')"
          :disabled="
            row.state !== 'stopped' ||
            ctx.isInstanceInteractionDisabled(row.id)
          "
          @click="emit('repair', row)"
        />
        <v-list-item
          prepend-icon="mdi-heart-pulse"
          :title="t('instances.actions.healthCheck')"
          :disabled="
            ctx.healthLoading || ctx.isInstanceInteractionDisabled(row.id)
          "
          @click="emit('healthCheck', row)"
        />
        <v-list-item
          prepend-icon="mdi-text-box-outline"
          :title="t('instances.actions.viewLogs')"
          @click="emit('openLogs', row)"
        />
        <v-list-item
          prepend-icon="mdi-chart-timeline-variant"
          :title="t('resources.title')"
          @click="emit('openResources', row)"
        />
        <v-list-item
          prepend-icon="mdi-open-in-new"
          :title="t('instances.actions.openInBrowser')"
          :disabled="!row.dashboard_enabled"
          @click="emit('openInBrowser', row)"
        />
        <v-divider />
        <v-list-item
          prepend-icon="mdi-database-remove-outline"
          :title="t('instances.actions.clearData')"
          :disabled="
            row.state !== 'stopped' ||
            ctx.isInstanceInteractionDisabled(row.id)
          "
          @click="emit('clear', row, 'clear-data')"
        />
        <v-list-item
          prepend-icon="mdi-delete-sweep-outline"
          :title="t('instances.actions.clearVenv')"
          :disabled="
            row.state !== 'stopped' ||
            ctx.isInstanceInteractionDisabled(row.id)
          "
          @click="emit('clear', row, 'clear-venv')"
        />
        <v-list-item
          prepend-icon="mdi-broom"
          :title="t('instances.actions.clearPycache')"
          :disabled="
            row.state !== 'stopped' ||
            ctx.isInstanceInteractionDisabled(row.id)
          "
          @click="emit('clear', row, 'clear-pycache')"
        />
        <v-divider />
        <v-list-item
          prepend-icon="mdi-delete-outline"
          :title="t('instances.actions.deleteInstance')"
          base-color="error"
          :disabled="
            row.state !== 'stopped' ||
            ctx.isInstanceInteractionDisabled(row.id)
          "
          @click="emit('delete', row)"
        />
      </v-list>
    </v-menu>
  </template>
</template>

<style scoped>
.instance-menu {
  min-width: 208px;
}
</style>
