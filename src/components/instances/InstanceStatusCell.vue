<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import StatusChip from "../common/StatusChip.vue";
import type { InstanceStatus } from "../../types/launcher";
import { formatRunDuration } from "../../utils/format";

const DURATION_REFRESH_INTERVAL_MS = 30_000;

const props = defineProps<{
  instance: InstanceStatus;
}>();

const { t } = useI18n();

const now = ref(Date.now());
let durationTimer: number | null = null;

onMounted(() => {
  durationTimer = window.setInterval(() => {
    now.value = Date.now();
  }, DURATION_REFRESH_INTERVAL_MS);
});

onBeforeUnmount(() => {
  if (durationTimer !== null) {
    window.clearInterval(durationTimer);
    durationTimer = null;
  }
});

const duration = computed<string | null>(() => {
  const active =
    props.instance.state === "running" || props.instance.state === "stopping";
  return active && props.instance.started_at_ms !== null
    ? formatRunDuration(props.instance.started_at_ms, now.value)
    : null;
});

const hasIssueInfo = computed(
  () =>
    props.instance.last_exit_reason !== "" ||
    props.instance.restart_attempts > 0,
);
</script>

<template>
  <div class="instance-status">
    <StatusChip
      :state="instance.state"
      :animated="instance.state === 'starting' || instance.state === 'stopping'"
    />
    <span v-if="duration" class="instance-status__duration page-nums">
      <v-icon size="12" icon="mdi-clock-outline" aria-hidden="true" />
      {{ duration }}
    </span>
    <v-tooltip v-if="hasIssueInfo" location="top" max-width="360">
      <template #activator="{ props: issueProps }">
        <v-icon
          v-bind="issueProps"
          icon="mdi-alert-circle-outline"
          size="14"
          color="warning"
          tabindex="0"
          :aria-label="t('instances.list.aria.lastExit')"
        />
      </template>
      <div v-if="instance.last_exit_reason !== ''" class="instance-status__issue-line">
        {{ t("instances.list.lastExit", { reason: instance.last_exit_reason }) }}
      </div>
      <div v-if="instance.restart_attempts > 0" class="instance-status__issue-line">
        {{
          t("instances.list.restartAttempts", {
            attempt: instance.restart_attempts,
            max: instance.auto_restart_max_retries,
          })
        }}
      </div>
    </v-tooltip>
    <v-tooltip
      v-if="instance.pid_tracker_not_available"
      :text="t('instances.list.pidWarning')"
      location="top"
    >
      <template #activator="{ props: pidProps }">
        <v-icon
          v-bind="pidProps"
          icon="mdi-alert-outline"
          size="16"
          color="warning"
          tabindex="0"
          :aria-label="t('instances.list.aria.pidUnreliable')"
        />
      </template>
    </v-tooltip>
  </div>
</template>

<style scoped>
.instance-status {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.instance-status__duration {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 0.78rem;
  color: rgba(var(--v-theme-on-surface), 0.68);
}

.instance-status__issue-line + .instance-status__issue-line {
  margin-top: 2px;
}
</style>
