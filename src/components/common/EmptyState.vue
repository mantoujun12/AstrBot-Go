<script setup lang="ts">
withDefaults(
  defineProps<{
    text: string;
    icon?: string;
  }>(),
  { icon: "mdi-inbox-outline" },
);
</script>

<template>
  <div class="empty-state">
    <v-icon :icon="icon" size="36" aria-hidden="true" />
    <p>{{ text }}</p>
    <slot />
  </div>
</template>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  min-height: 160px;
  padding: 24px;
  color: rgba(var(--v-theme-on-surface), 0.48);
  text-align: center;
}

.empty-state p {
  margin: 0;
  font-size: 0.875rem;
  line-height: 1.5;
}
</style>
