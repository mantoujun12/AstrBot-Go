<template>
  <section class="overview-placeholder" aria-label="概览">
    <p>主界面内容占位</p>
  </section>
</template>

<style scoped>
.overview-placeholder {
  display: grid;
  min-height: 100%;
  place-items: center;
  padding: 24px;
}

.overview-placeholder p {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.48);
  font-size: 0.92rem;
  letter-spacing: 0.02em;
}

@media (max-width: 959px) {
  .overview-placeholder {
    min-height: 100%;
  }
}
</style>
