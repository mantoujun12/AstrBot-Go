<script setup lang="ts">
import { useI18n } from "vue-i18n";

defineProps<{
  selectedCount: number;
  disabled: boolean;
}>();

const emit = defineEmits<{
  start: [];
  restart: [];
  stop: [];
}>();

const { t } = useI18n();
</script>

<template>
  <!-- Teleported to `body` like the title bar: `.launcher-shell`
       (isolation) and `.v-application__wrap` (backface-visibility) create
       stacking contexts that would trap a fixed bar inside the pane stack.
       z-index stays below Vuetify overlays (2400+), so a dialog scrim
       correctly covers the bar. -->
  <Teleport to="body">
    <Transition name="instance-batch-bar">
      <div
        v-if="selectedCount > 0"
        class="instance-batch-bar"
        role="toolbar"
        :aria-label="t('instances.list.batch.selected', { count: selectedCount })"
      >
        <span class="instance-batch-bar__count page-muted page-nums">
          {{ t("instances.list.batch.selected", { count: selectedCount }) }}
        </span>
        <div class="instance-batch-bar__actions">
          <v-btn
            color="primary"
            variant="tonal"
            size="small"
            prepend-icon="mdi-play"
            :disabled="disabled"
            @click="emit('start')"
          >
            {{ t("instances.list.batch.start") }}
          </v-btn>
          <v-btn
            color="warning"
            variant="tonal"
            size="small"
            prepend-icon="mdi-restart"
            :disabled="disabled"
            @click="emit('restart')"
          >
            {{ t("instances.list.batch.restart") }}
          </v-btn>
          <v-btn
            color="warning"
            variant="tonal"
            size="small"
            prepend-icon="mdi-stop"
            :disabled="disabled"
            @click="emit('stop')"
          >
            {{ t("instances.list.batch.stop") }}
          </v-btn>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.instance-batch-bar {
  position: fixed;
  left: 50%;
  bottom: 24px;
  transform: translateX(-50%);
  z-index: 2300;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  color: rgba(var(--v-theme-on-surface), 0.87);
  background: rgb(var(--v-theme-surface));
  border: 1px solid rgba(var(--v-theme-on-surface), 0.1);
  border-radius: var(--astrbot-radius-card);
  box-shadow: var(--astrbot-menu-shadow);
}

.instance-batch-bar__count {
  font-size: 0.8rem;
}

.instance-batch-bar__actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.instance-batch-bar-enter-active,
.instance-batch-bar-leave-active {
  transition:
    opacity var(--motion-duration-base) var(--motion-ease),
    transform var(--motion-duration-base) var(--motion-ease);
}

.instance-batch-bar-enter-from,
.instance-batch-bar-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(12px);
}
</style>
