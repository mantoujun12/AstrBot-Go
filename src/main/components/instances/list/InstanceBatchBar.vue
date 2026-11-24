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
  <v-sheet class="instance-batch-bar" elevation="10">
    <span class="page-muted">
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
  </v-sheet>
</template>

<style scoped>
.instance-batch-bar {
  position: fixed;
  left: 50%;
  bottom: 24px;
  transform: translateX(-50%);
  z-index: 4;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  border-radius: var(--astrbot-radius-card);
}

.instance-batch-bar__actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
