<script setup lang="ts">
defineProps<{
  title: string;
}>();
</script>

<template>
  <header class="page-header">
    <h1 class="page-header__title">{{ title }}</h1>
    <div v-if="$slots.actions" class="page-header__actions">
      <slot name="actions" />
    </div>
  </header>
</template>

<style scoped>
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.page-header__title {
  margin: 0;
  color: rgb(var(--v-theme-on-surface));
  font-size: 1.5rem;
  line-height: 1.2;
  font-weight: 600;
  letter-spacing: 0;
}

.page-header__actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
</style>
