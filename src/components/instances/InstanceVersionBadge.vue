<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { InstanceUpdateInfo } from "../../types/launcher";

const props = withDefaults(
  defineProps<{
    version: string;
    update: InstanceUpdateInfo | null;
    loading?: boolean;
  }>(),
  { loading: false },
);

const { t } = useI18n();

const statusColor = computed(() => {
  switch (props.update?.state) {
    case "available":
      return "warning";
    case "up_to_date":
      return "success";
    case "disabled":
      return "secondary";
    default:
      return "default";
  }
});

const statusLabel = computed(() => {
  switch (props.update?.state) {
    case "available":
      return t("instances.updates.status.available", {
        version: props.update.latest_version ?? "",
      });
    case "up_to_date":
      return t("instances.updates.status.upToDate");
    case "disabled":
      return t("instances.updates.status.disabled");
    case "unknown":
      return t("instances.updates.status.unknown");
    default:
      return t("instances.updates.status.disabled");
  }
});

const statusTooltip = computed(() => {
  switch (props.update?.state) {
    case "disabled":
      return t("instances.updates.tooltips.disabled");
    case "unknown":
      return t("instances.updates.tooltips.unknown");
    default:
      return "";
  }
});
</script>

<template>
  <div class="instance-version">
    <v-chip color="primary" variant="tonal" size="small" label>
      {{ version }}
    </v-chip>
    <v-progress-circular
      v-if="loading && !update"
      indeterminate
      size="14"
      width="2"
      color="primary"
      :aria-label="t('instances.updates.loading')"
    />
    <v-tooltip v-else-if="update && statusTooltip" :text="statusTooltip">
      <template #activator="{ props: activatorProps }">
        <v-chip
          v-bind="activatorProps"
          :color="statusColor"
          variant="tonal"
          size="x-small"
          label
          tabindex="0"
        >
          {{ statusLabel }}
        </v-chip>
      </template>
    </v-tooltip>
    <v-chip
      v-else-if="update"
      :color="statusColor"
      variant="tonal"
      size="x-small"
      label
    >
      {{ statusLabel }}
    </v-chip>
  </div>
</template>

<style scoped>
.instance-version {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  min-width: 0;
}

.instance-version :deep(.v-chip) {
  font-variant-numeric: tabular-nums;
}
</style>
