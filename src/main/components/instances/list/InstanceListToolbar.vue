<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type {
  InstanceSortKey,
  InstanceStateFilter,
  InstanceSourceFilter,
  InstanceViewMode,
} from "../../../composables/useInstanceList";

const search = defineModel<string>("search", { required: true });
const stateFilter = defineModel<InstanceStateFilter>("stateFilter", {
  required: true,
});
const sourceFilter = defineModel<InstanceSourceFilter>("sourceFilter", {
  required: true,
});
const sortKey = defineModel<InstanceSortKey>("sortKey", { required: true });
const viewMode = defineModel<InstanceViewMode>("viewMode", { required: true });

defineProps<{
  refreshLoading: boolean;
  refreshDisabled: boolean;
  showUpgradeAll: boolean;
  upgradeAllLoading: boolean;
  upgradeAllDisabled: boolean;
  createDisabled: boolean;
}>();

const emit = defineEmits<{
  refresh: [];
  upgradeAll: [];
  create: [];
}>();

const { t } = useI18n();

const stateItems = computed(() => [
  { title: t("instances.list.filter.stateAll"), value: "all" },
  { title: t("instances.list.filter.stateRunning"), value: "running" },
  { title: t("instances.list.filter.stateStopped"), value: "stopped" },
]);

const sourceItems = computed(() => [
  { title: t("instances.list.filter.sourceAll"), value: "all" },
  { title: t("instances.list.filter.sourceLauncher"), value: "launcher" },
  { title: t("instances.list.filter.sourceExternal"), value: "external" },
]);

const sortItems = computed(() => [
  { title: t("instances.list.sort.nameAsc"), value: "nameAsc" },
  { title: t("instances.list.sort.nameDesc"), value: "nameDesc" },
  { title: t("instances.list.sort.state"), value: "state" },
  { title: t("instances.list.sort.versionDesc"), value: "versionDesc" },
  { title: t("instances.list.sort.portAsc"), value: "portAsc" },
]);
</script>

<template>
  <div class="instance-list-toolbar">
    <v-text-field
      v-model="search"
      class="instance-list-toolbar__search"
      clearable
      prepend-inner-icon="mdi-magnify"
      :placeholder="t('instances.list.search')"
      :aria-label="t('instances.list.search')"
      variant="outlined"
      density="compact"
      hide-details
    />
    <v-select
      v-model="stateFilter"
      class="instance-list-toolbar__filter"
      :items="stateItems"
      :label="t('instances.list.filter.state')"
      variant="outlined"
      density="compact"
      hide-details
    />
    <v-select
      v-model="sourceFilter"
      class="instance-list-toolbar__filter"
      :items="sourceItems"
      :label="t('instances.list.filter.source')"
      variant="outlined"
      density="compact"
      hide-details
    />
    <v-select
      v-model="sortKey"
      class="instance-list-toolbar__sort"
      :items="sortItems"
      prepend-inner-icon="mdi-sort-variant"
      variant="outlined"
      density="compact"
      hide-details
    />
    <v-btn-toggle
      v-model="viewMode"
      class="instance-list-toolbar__view"
      mandatory
      divided
      density="compact"
      variant="outlined"
      color="primary"
    >
      <v-tooltip location="bottom">
        <template #activator="{ props: tooltipProps }">
          <v-btn
            v-bind="tooltipProps"
            value="table"
            icon="mdi-table"
            size="small"
            :aria-label="t('instances.list.view.table')"
          />
        </template>
        {{ t("instances.list.view.table") }}
      </v-tooltip>
      <v-tooltip location="bottom">
        <template #activator="{ props: tooltipProps }">
          <v-btn
            v-bind="tooltipProps"
            value="cards"
            icon="mdi-view-grid-outline"
            size="small"
            :aria-label="t('instances.list.view.cards')"
          />
        </template>
        {{ t("instances.list.view.cards") }}
      </v-tooltip>
    </v-btn-toggle>
    <div class="instance-list-toolbar__actions">
      <v-btn
        variant="text"
        prepend-icon="mdi-refresh"
        :loading="refreshLoading"
        :disabled="refreshDisabled"
        @click="emit('refresh')"
      >
        {{ t("common.actions.refresh") }}
      </v-btn>
      <v-btn
        v-if="showUpgradeAll"
        color="primary"
        variant="text"
        prepend-icon="mdi-update"
        :loading="upgradeAllLoading"
        :disabled="upgradeAllDisabled"
        @click="emit('upgradeAll')"
      >
        {{ t("instances.actions.upgradeAll") }}
      </v-btn>
      <v-btn
        color="primary"
        variant="flat"
        prepend-icon="mdi-plus"
        :disabled="createDisabled"
        @click="emit('create')"
      >
        {{ t("instances.actions.create") }}
      </v-btn>
    </div>
  </div>
</template>

<style scoped>
/*
 * Sticky within `.launcher-home-pane`: Chromium pins a sticky element at
 * `inset + padding-top` of the scrolling container, so `top: -16px` keeps the
 * bar flush with the pane's visible top edge once the 16px padding scrolls
 * past (the pane's top padding grows to 72px under 840px, where the bar then
 * sticks below the floating menu button instead).
 */
.instance-list-toolbar {
  position: sticky;
  top: -16px;
  z-index: 1;
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-inline: -16px;
  margin-bottom: 16px;
  padding: 8px 16px;
  background: rgb(var(--v-theme-surface));
  border: 1px solid rgba(var(--v-theme-on-surface), 0.1);
  border-radius: var(--astrbot-radius-card);
}

.instance-list-toolbar__search {
  width: 240px;
  max-width: 100%;
}

.instance-list-toolbar__filter {
  width: 160px;
  max-width: 100%;
}

.instance-list-toolbar__sort {
  width: 170px;
  max-width: 100%;
}

.instance-list-toolbar__actions {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
  margin-left: auto;
}
</style>
