<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useRoute } from "vue-router";
import ExternalWebuiView from "../components/instances/ExternalWebuiView.vue";
import InstanceWebuiView from "../views/InstanceWebuiView.vue";
import { useLauncherStore } from "../stores/launcher";
import { HOME_TAB, externalTabOf, useTabsStore } from "../stores/tabs";

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
  instanceIds.value.map((id) => {
    const external = externalTabOf(id);
    return {
      id,
      exists: external !== null || instances.value.some((instance) => instance.id === id),
      external,
    };
  }),
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
    <div class="launcher-main__canvas">
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
        class="launcher-home-pane motion-fade-in"
        role="tabpanel"
        :aria-label="t('shell.main.home')"
      >
        <router-view v-slot="{ Component, route: currentRoute }">
          <Transition name="route-fade" mode="out-in">
            <component :is="Component" :key="currentRoute.path" />
          </Transition>
        </router-view>
      </div>

      <div
        v-for="tab in instanceTabs"
        :key="tab.id"
        v-show="activeTab === tab.id"
        :id="`tabpanel-${tab.id}`"
        class="launcher-instance-pane motion-fade-in"
        role="tabpanel"
      >
        <ExternalWebuiView
          v-if="tab.external"
          :name="tab.external.name"
          :url="tab.external.url"
        />
        <InstanceWebuiView
          v-else-if="tab.exists"
          :instance-id="tab.id"
        />
      </div>
    </div>
  </v-main>
</template>
<style scoped>
.launcher-main {
  --launcher-content-gap: 6px;
  height: calc(100% - var(--launcher-titlebar-height));
  min-height: 0;
  margin-top: var(--launcher-titlebar-height);
  overflow: hidden;
  background: transparent;
}

.launcher-main__canvas {
  position: relative;
  height: 100%;
  min-height: 0;
  overflow: hidden;
}

/* 首页内容为浮动圆角卡片：背景刻意不透明，原生毛玻璃只露在卡片外。 */
.launcher-home-pane {
  box-sizing: border-box;
  width: calc(100% - var(--launcher-content-gap));
  height: calc(100% - var(--launcher-content-gap));
  min-height: 0;
  padding: 16px;
  border: 1px solid rgba(var(--v-theme-on-surface), 0.1);
  border-radius: var(--astrbot-radius-navigation);
  background: rgb(var(--v-theme-surface));
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

/* 移动端对齐上游 ≤959.98px：无缝隙、无圆角、无边框，内容全幅。 */
@media (max-width: 959.98px) {
  .launcher-main {
    --launcher-content-gap: 0px;
  }

  .launcher-home-pane {
    padding: 72px 16px 16px;
    border: 0;
    border-radius: 0;
  }
}
</style>
