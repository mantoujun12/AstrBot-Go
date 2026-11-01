<script setup lang="ts">
import type { InstanceState } from "../../types/launcher";
import { instanceStateColor, instanceStateLabel } from "../../utils/format";

withDefaults(
  defineProps<{
    state: InstanceState;
    size?: "x-small" | "small";
  }>(),
  { size: "small" },
);
</script>

<template>
  <v-chip
    :color="instanceStateColor(state)"
    :size="size"
    variant="tonal"
    label
    class="status-chip"
  >
    <span class="status-chip__dot" aria-hidden="true" />
    {{ instanceStateLabel(state) }}
  </v-chip>
</template>

<style scoped>
.status-chip__dot {
  display: inline-block;
  width: 6px;
  height: 6px;
  margin-inline-end: 6px;
  border-radius: 50%;
  background: currentColor;
}
</style>
