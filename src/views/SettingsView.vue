<template>
  <section class="settings-placeholder" aria-label="设置">
    <p>设置页面占位</p>
  </section>
</template>

<style scoped>
.settings-placeholder {
  display: grid;
  min-height: 100%;
  place-items: center;
  padding: 24px;
}

.settings-placeholder p {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.48);
  font-size: 0.92rem;
  letter-spacing: 0.02em;
}

@media (max-width: 959px) {
  .settings-placeholder {
    min-height: 100%;
  }
}
</style>
