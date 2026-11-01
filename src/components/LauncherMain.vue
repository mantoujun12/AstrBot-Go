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
  height: calc(100% - var(--launcher-titlebar-height));
  min-height: 0;
  margin-top: var(--launcher-titlebar-height);
  overflow: hidden;
  background: transparent;
}

.launcher-main__canvas {
  position: relative;
  box-sizing: border-box;
  height: calc(100% - 16px);
  min-height: 0;
  margin: 8px;
  padding: 16px;
  overflow-x: hidden;
  overflow-y: auto;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.1);
  border-radius: 12px;
  background: rgb(var(--v-theme-background));
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
