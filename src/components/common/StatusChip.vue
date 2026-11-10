<script setup lang="ts">
import type { InstanceState } from "../../types/launcher";
import { instanceStateColor, instanceStateLabel } from "../../utils/format";

withDefaults(
  defineProps<{
    state: InstanceState;
    size?: "x-small" | "small";
    animated?: boolean;
  }>(),
  { size: "small", animated: false },
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
    <span
      class="status-chip__dot"
      :class="{ 'status-chip__dot--animated': animated }"
      aria-hidden="true"
    />
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

@keyframes status-pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.25;
  }
}

.status-chip__dot--animated {
  animation: status-pulse 1.4s ease-in-out infinite;
}

@media (prefers-reduced-motion: reduce) {
  .status-chip__dot--animated {
    animation: none;
  }
}

:global(html[data-reduce-motion]) .status-chip__dot--animated {
  animation: none;
}
</style>
