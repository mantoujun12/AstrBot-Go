<script setup lang="ts">
defineProps<{
  mobile: boolean;
}>();

const emit = defineEmits<{
  (event: "open-menu"): void;
}>();
</script>

<template>
  <v-main class="launcher-main">
    <div class="launcher-main__canvas">
      <v-btn
        v-if="mobile"
        class="launcher-mobile-menu"
        icon
        aria-label="打开菜单"
        color="surface"
        elevation="2"
        @click="emit('open-menu')"
      >
        <v-icon icon="mdi-menu" />
      </v-btn>

      <router-view />
    </div>
  </v-main>
</template>

<style scoped>
.launcher-main {
  height: calc(100vh - var(--launcher-titlebar-height));
  height: calc(100dvh - var(--launcher-titlebar-height));
  min-height: 0;
  margin-top: var(--launcher-titlebar-height);
  overflow-x: hidden;
  overflow-y: auto;
  background: rgb(var(--v-theme-background));
}

.launcher-main__canvas {
  position: relative;
  box-sizing: border-box;
  height: 100%;
  min-height: 0;
  padding: 24px;
}

.launcher-mobile-menu {
  position: absolute;
  z-index: 1;
  top: 16px;
  left: 16px;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.08);
}

@media (max-width: 959px) {
  .launcher-main__canvas {
    padding: 72px 16px 16px;
  }
}
</style>
