<script setup lang="ts">
withDefaults(
  defineProps<{
    title: string;
    subtitle?: string;
    labelFor?: string;
    stacked?: boolean;
    wide?: boolean;
  }>(),
  {
    subtitle: "",
    stacked: false,
    wide: false,
  },
);
</script>

<template>
  <div
    class="settings-item"
    :class="{
      'settings-item--stacked': stacked,
      'settings-item--wide': wide,
    }"
  >
    <label v-if="labelFor" :for="labelFor" class="settings-item__label">
      <span class="settings-item__title">{{ title }}</span>
      <span v-if="subtitle" class="settings-item__subtitle">{{ subtitle }}</span>
    </label>
    <div v-else class="settings-item__label">
      <div class="settings-item__title">{{ title }}</div>
      <div v-if="subtitle" class="settings-item__subtitle">{{ subtitle }}</div>
    </div>
    <div class="settings-item__control">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.settings-item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(180px, 34%);
  gap: 18px;
  align-items: center;
  min-height: 60px;
  padding: 12px 16px;
  border-bottom: 1px solid rgba(var(--v-theme-on-surface), 0.08);
}

.settings-item:last-child {
  border-bottom: 0;
}

.settings-item--stacked {
  display: block;
  min-height: auto;
  padding-bottom: 16px;
}

.settings-item--wide {
  grid-template-columns: minmax(0, 1fr) minmax(300px, 48%);
}

.settings-item__label {
  min-width: 0;
}

label.settings-item__label {
  cursor: pointer;
}

.settings-item__title {
  display: block;
  color: rgb(var(--v-theme-on-surface));
  font-size: 0.875rem;
  font-weight: 600;
  line-height: 1.4;
}

.settings-item__subtitle {
  display: block;
  margin-top: 3px;
  color: rgba(var(--v-theme-on-surface), 0.62);
  font-size: 0.78rem;
  line-height: 1.45;
}

.settings-item__control {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  width: 100%;
  max-width: 270px;
  min-width: 0;
  justify-self: end;
}

.settings-item--stacked .settings-item__label {
  margin-bottom: 10px;
}

.settings-item--stacked .settings-item__control {
  display: block;
  max-width: none;
  justify-self: stretch;
}

@media (max-width: 720px) {
  .settings-item,
  .settings-item--wide {
    grid-template-columns: 1fr;
    gap: 10px;
    align-items: stretch;
  }

  .settings-item__control {
    max-width: none;
    justify-self: stretch;
  }
}
</style>
