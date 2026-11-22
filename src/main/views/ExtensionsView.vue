<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import InstalledPluginsPanel from "../components/extensions/InstalledPluginsPanel.vue";
import PluginMarketPanel from "../components/extensions/PluginMarketPanel.vue";
import SkillsPanel from "../components/extensions/SkillsPanel.vue";

type ExtensionTab = "plugins" | "skills";
type PluginSubTab = "market" | "installed";

const { t } = useI18n();

const tabs = [
  { value: "plugins", icon: "mdi-puzzle-outline", label: "extensions.tabs.plugins" },
  { value: "skills", icon: "mdi-school-outline", label: "extensions.tabs.skills" },
] as const satisfies readonly { value: ExtensionTab; icon: string; label: string }[];

const activeTab = ref<ExtensionTab>("plugins");
const pluginSubTab = ref<PluginSubTab>("market");
const skillsPanel = ref<InstanceType<typeof SkillsPanel> | null>(null);
</script>

<template>
  <div class="page-shell extensions-view">
    <div class="extensions-view__toolbar">
      <div class="extensions-view__tabs-scroll">
        <nav
          class="extensions-view__tabs"
          role="tablist"
          :aria-label="t('shell.nav.extensions')"
        >
          <button
            v-for="tab in tabs"
            :key="tab.value"
            type="button"
            class="extensions-view__tab"
            :class="{ 'extensions-view__tab--active': activeTab === tab.value }"
            role="tab"
            :aria-selected="activeTab === tab.value"
            @click="activeTab = tab.value"
          >
            <v-icon :icon="tab.icon" size="16" aria-hidden="true" />
            <span>{{ t(tab.label) }}</span>
          </button>
        </nav>
      </div>

      <div class="page-actions">
        <v-btn
          v-if="activeTab === 'skills'"
          color="primary"
          variant="flat"
          prepend-icon="mdi-upload"
          @click="skillsPanel?.openUpload()"
        >
          {{ t("skills.upload") }}
        </v-btn>
      </div>
    </div>

    <template v-if="activeTab === 'plugins'">
      <div class="extensions-view__switch-row">
        <v-btn-toggle
          v-model="pluginSubTab"
          mandatory
          divided
          density="compact"
          variant="outlined"
          color="primary"
        >
          <v-btn size="small" value="market">
            {{ t("plugins.tabs.market") }}
          </v-btn>
          <v-btn size="small" value="installed">
            {{ t("plugins.tabs.installed") }}
          </v-btn>
        </v-btn-toggle>
      </div>
      <PluginMarketPanel v-if="pluginSubTab === 'market'" />
      <InstalledPluginsPanel v-else />
    </template>
    <SkillsPanel v-else ref="skillsPanel" />
  </div>
</template>

<style scoped>
.extensions-view__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 16px;
}

.extensions-view__tabs-scroll {
  min-width: 0;
  overflow-x: auto;
  scrollbar-width: none;
}

.extensions-view__tabs-scroll::-webkit-scrollbar {
  display: none;
}

.extensions-view__tabs {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  min-width: max-content;
}

.extensions-view__tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 34px;
  padding: 0 12px;
  border: 0;
  border-radius: var(--astrbot-radius-control);
  background: none;
  color: rgba(var(--v-theme-on-surface), 0.58);
  font-family: inherit;
  font-size: 0.8125rem;
  font-weight: 500;
  cursor: pointer;
}

.extensions-view__tab:hover {
  background: rgba(var(--v-theme-on-surface), 0.045);
  color: rgba(var(--v-theme-on-surface), 0.78);
}

.extensions-view__tab--active {
  background: rgba(var(--v-theme-primary), 0.12);
  color: rgb(var(--v-theme-primary));
}

.extensions-view__switch-row {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}
</style>
