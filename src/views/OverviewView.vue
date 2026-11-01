<template>
  <section class="overview-placeholder" aria-label="概览">
    <p>主界面内容占位</p>
  </section>
</template>

<style scoped>
.overview-placeholder {
  display: grid;
  min-height: calc(100vh - 48px);
  place-items: center;
  padding: 32px;
  border: 1px dashed rgba(var(--v-theme-on-surface), 0.14);
  border-radius: 20px;
  background: rgba(var(--v-theme-surface), 0.72);
}

.overview-placeholder p {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.48);
  font-size: 0.92rem;
  letter-spacing: 0.02em;
}

@media (max-width: 959px) {
  .overview-placeholder {
    min-height: calc(100dvh - 88px);
  }
}
</style>
