<script setup lang="ts">
defineProps<{
  icon: string;
  title: string;
  description: string;
}>();
</script>

<template>
  <article class="extension-card">
    <div class="extension-card__cover" aria-hidden="true" />
    <div class="extension-card__info">
      <v-avatar class="extension-card__icon" size="32" :rounded="10">
        <v-icon :icon="icon" size="18" aria-hidden="true" />
      </v-avatar>
      <div class="extension-card__text">
        <p class="extension-card__title">{{ title }}</p>
        <p class="extension-card__description">{{ description }}</p>
      </div>
    </div>
  </article>
</template>

<style scoped>
.extension-card {
  display: flex;
  flex-direction: column;
  height: 280px;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.12);
  border-radius: var(--astrbot-radius-card);
  overflow: hidden;
  cursor: pointer;
  transition:
    border-color 0.15s ease,
    box-shadow 0.15s ease,
    transform 0.15s ease;
}

.extension-card:hover {
  border-color: rgba(var(--v-theme-primary), 0.45);
  box-shadow: 0 6px 18px rgba(0, 0, 0, 0.14);
}

.extension-card:focus-visible {
  outline: 2px solid rgb(var(--v-theme-primary));
  outline-offset: -2px;
}

.extension-card__cover {
  flex: 4;
  min-height: 0;
  /* shortcut: placeholder gradient, replace with real cover art */
  background: linear-gradient(
    135deg,
    rgba(var(--v-theme-primary), 0.85),
    rgba(var(--v-theme-secondary), 0.65)
  );
}

.extension-card__info {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 0;
  padding: 0 12px;
  background: rgb(var(--v-theme-surface));
}

.extension-card__icon {
  flex: none;
}

.extension-card__text {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.extension-card__title {
  margin: 0;
  color: rgb(var(--v-theme-on-surface));
  font-size: 0.875rem;
  font-weight: 600;
  line-height: 1.3;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.extension-card__description {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.55);
  font-size: 0.75rem;
  line-height: 1.3;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
