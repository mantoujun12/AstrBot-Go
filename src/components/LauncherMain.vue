<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute } from "vue-router";
import InstanceWebuiView from "../views/InstanceWebuiView.vue";
import { useLauncherStore } from "../stores/launcher";
import { HOME_TAB, useTabsStore } from "../stores/tabs";

defineProps<{
  mobile: boolean;
}>();

const emit = defineEmits<{
  (event: "open-menu"): void;
}>();

const route = useRoute();
const { t } = useI18n();
const { instanceIds, activeTab } = useTabsStore();
const { instances } = useLauncherStore();
const homePane = ref<HTMLElement | null>(null);

const isHome = computed(() => activeTab.value === HOME_TAB);

const instanceTabs = computed(() =>
  instanceIds.value.map((id) => ({
    id,
    exists: instances.value.some((instance) => instance.id === id),
  })),
);

watch(
  () => route.fullPath,
  () => {
    void nextTick(() => {
      homePane.value?.scrollTo({ top: 0 });
    });
  },
);
</script>

<template>
  <v-main class="launcher-main">
    <div
      class="launcher-main__canvas"
      :class="{ 'launcher-main__canvas--full': !isHome }"
    >
      <v-btn
        v-if="mobile && isHome"
        class="launcher-mobile-menu"
        icon
        :aria-label="t('shell.main.openMenu')"
        color="surface"
        elevation="2"
        @click="emit('open-menu')"
      >
        <v-icon icon="mdi-menu" />
      </v-btn>

      <div
        v-show="isHome"
        id="tabpanel-home"
        ref="homePane"
        class="launcher-home-pane"
        role="tabpanel"
        :aria-label="t('shell.main.home')"
      >
        <router-view />
      </div>

      <div
        v-for="tab in instanceTabs"
        :key="tab.id"
        v-show="activeTab === tab.id"
        :id="`tabpanel-${tab.id}`"
        class="launcher-instance-pane"
        role="tabpanel"
      >
        <InstanceWebuiView v-if="tab.exists" :instance-id="tab.id" />
      </div>
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
  overflow: hidden;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.1);
  border-radius: 12px;
  background: rgb(var(--v-theme-background));
}

/* Instance tabs fill the whole window: no outer gap, border, or rounding. */
.launcher-main__canvas--full {
  height: 100%;
  margin: 0;
  border: 0;
  border-radius: 0;
}

.launcher-home-pane {
  height: 100%;
  min-height: 0;
  padding: 16px;
  overflow-x: hidden;
  overflow-y: auto;
}

.launcher-instance-pane {
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

.launcher-mobile-menu {
  position: absolute;
  z-index: 2;
  top: 16px;
  left: 16px;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.08);
}

@media (max-width: 959px) {
  .launcher-home-pane {
    padding: 72px 16px 16px;
  }
}
</style>
