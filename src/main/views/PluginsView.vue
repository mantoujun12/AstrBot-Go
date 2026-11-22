<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ConfirmDialog from "../components/common/ConfirmDialog.vue";
import EmptyState from "../components/common/EmptyState.vue";
import PageHeader from "../components/common/PageHeader.vue";
import { openExternalUrl } from "../services/app";
import { toUserMessage } from "../services/errors";
import {
  fetchPluginMarket,
  installInstancePlugin,
  listInstancePlugins,
  setInstancePluginEnabled,
  uninstallInstancePlugin,
} from "../services/plugins";
import { useLauncherStore } from "../stores/launcher";
import { showErrorToast, showToast } from "../stores/toast";
import type { InstanceStatus } from "../types/launcher";
import type {
  InstancePlugin,
  PluginMarketItem,
} from "../types/plugins";

const { t } = useI18n();
const { instances } = useLauncherStore();

type TabKey = "market" | "installed";

const MARKET_PAGE_SIZE = 30;

const tab = ref<TabKey>("market");
const instanceId = ref<string | null>(null);

const marketLoading = ref(false);
const marketError = ref<string | null>(null);
const marketItems = ref<PluginMarketItem[]>([]);
const marketTotal = ref(0);
const marketPage = ref(1);
const search = ref("");
const category = ref<string | null>(null);
const categories = ref<{ name: string; icon: string; count: number }[]>([]);

const installedLoading = ref(false);
const installedError = ref<string | null>(null);
const installed = ref<InstancePlugin[]>([]);

const installingId = ref<string | null>(null);
const pendingInstall = ref<PluginMarketItem | null>(null);
const installing = ref(false);

const pendingUninstall = ref<InstancePlugin | null>(null);
const uninstalling = ref(false);
const uninstallDeleteConfig = ref(false);
const uninstallDeleteData = ref(false);

const togglingDir = ref<string | null>(null);

const selectedInstance = computed<InstanceStatus | null>(() => {
  if (!instanceId.value) {
    return null;
  }
  return instances.value.find((item) => item.id === instanceId.value) ?? null;
});

/** Installs write instance files, so a running instance rejects them. */
const canManage = computed(() => selectedInstance.value?.state === "stopped");

const canLoadMore = computed(
  () => marketItems.value.length < marketTotal.value && !marketLoading.value,
);

const installedById = computed(() => {
  const map = new Map<string, InstancePlugin>();
  for (const plugin of installed.value) {
    if (plugin.plugin_id) {
      map.set(plugin.plugin_id, plugin);
    }
  }
  return map;
});

const categoryItems = computed(() => [
  { title: t("plugins.allCategories"), value: "" },
  ...categories.value.map((entry) => ({
    title: `${entry.name} (${formatCount(entry.count)})`,
    value: entry.name,
  })),
]);

function marketKey(item: PluginMarketItem): string {
  return item.plugin_id;
}

async function loadMarket(reset: boolean): Promise<void> {
  if (reset) {
    marketPage.value = 1;
    marketItems.value = [];
  }
  marketLoading.value = true;
  marketError.value = null;
  try {
    const page = await fetchPluginMarket(
      marketPage.value,
      MARKET_PAGE_SIZE,
      search.value.trim() || undefined,
      category.value ?? undefined,
    );
    categories.value = page.categories.map((entry) => ({
      name: entry.name,
      icon: entry.icon,
      count: page.category_counts[entry.name] ?? 0,
    }));
    marketTotal.value = page.total;
    if (reset) {
      marketItems.value = page.items;
    } else {
      marketItems.value.push(...page.items);
    }
  } catch (error) {
    marketError.value = toUserMessage(error);
  } finally {
    marketLoading.value = false;
  }
}

async function loadMore(): Promise<void> {
  marketPage.value += 1;
  await loadMarket(false);
}

async function loadInstalled(): Promise<void> {
  if (!instanceId.value) {
    installed.value = [];
    return;
  }
  installedLoading.value = true;
  installedError.value = null;
  try {
    installed.value = await listInstancePlugins(instanceId.value);
  } catch (error) {
    installed.value = [];
    installedError.value = toUserMessage(error);
  } finally {
    installedLoading.value = false;
  }
}

function confirmInstall(item: PluginMarketItem): void {
  if (!canManage.value) {
    return;
  }
  pendingInstall.value = item;
}

async function performInstall(): Promise<void> {
  const item = pendingInstall.value;
  if (!item || !instanceId.value) {
    return;
  }
  installing.value = true;
  try {
    await installInstancePlugin(instanceId.value, {
      plugin_id: item.plugin_id,
      download_url: item.download_url ?? "",
    });
    showToast(t("plugins.toasts.installed", { name: pluginTitle(item) }));
    pendingInstall.value = null;
    await loadInstalled();
  } catch (error) {
    showErrorToast(error);
  } finally {
    installing.value = false;
  }
}

function pluginTitle(item: PluginMarketItem): string {
  return item.display_name?.trim() || item.name;
}

function installedTitle(plugin: InstancePlugin): string {
  return plugin.name;
}

async function togglePlugin(plugin: InstancePlugin, enabled: boolean): Promise<void> {
  if (!instanceId.value) {
    return;
  }
  togglingDir.value = plugin.dir_name;
  try {
    await setInstancePluginEnabled(instanceId.value, plugin.dir_name, enabled);
    plugin.enabled = enabled;
  } catch (error) {
    showErrorToast(error);
  } finally {
    togglingDir.value = null;
  }
}

function confirmUninstall(plugin: InstancePlugin): void {
  if (!canManage.value) {
    return;
  }
  uninstallDeleteConfig.value = false;
  uninstallDeleteData.value = false;
  pendingUninstall.value = plugin;
}

async function performUninstall(): Promise<void> {
  const plugin = pendingUninstall.value;
  if (!plugin || !instanceId.value) {
    return;
  }
  uninstalling.value = true;
  try {
    await uninstallInstancePlugin(
      instanceId.value,
      plugin.dir_name,
      uninstallDeleteConfig.value,
      uninstallDeleteData.value,
    );
    showToast(t("plugins.toasts.uninstalled", { name: installedTitle(plugin) }));
    pendingUninstall.value = null;
    await loadInstalled();
  } catch (error) {
    showErrorToast(error);
  } finally {
    uninstalling.value = false;
  }
}

/** Update flow: find the market record for an installed plugin, then install. */
async function checkUpdate(plugin: InstancePlugin): Promise<void> {
  if (!canManage.value || installingId.value) {
    return;
  }
  installingId.value = plugin.dir_name;
  try {
    const page = await fetchPluginMarket(1, 20, plugin.name);
    const match = page.items.find((item) => item.plugin_id === plugin.plugin_id);
    if (!match?.download_url) {
      showToast(t("plugins.toasts.marketMissing"), "warning");
      return;
    }
    if (match.latest_version && match.latest_version === plugin.version) {
      showToast(t("plugins.toasts.upToDate", { version: match.latest_version }));
      return;
    }
    confirmInstall(match);
  } catch (error) {
    showErrorToast(error);
  } finally {
    installingId.value = null;
  }
}

function openRepo(url: string | null): void {
  if (url) {
    void openExternalUrl(url);
  }
}

/** Hide a broken market logo so the letter avatar shows instead. */
function onLogoError(event: Event): void {
  (event.target as Element).classList.add("plugin-card__logo--broken");
}

function formatCount(value: number): string {
  return value.toLocaleString();
}

let searchTimer: ReturnType<typeof setTimeout> | undefined;

watch(search, () => {
  clearTimeout(searchTimer);
  searchTimer = setTimeout(() => {
    void loadMarket(true);
  }, 300);
});

watch(category, () => {
  void loadMarket(true);
});

watch(
  () => selectedInstance.value?.id,
  () => {
    installed.value = [];
    installedError.value = null;
  },
);

watch(tab, (next) => {
  if (next === "installed") {
    void loadInstalled();
  }
});

void loadMarket(true);
</script>

<template>
  <div class="page-shell plugins-page">
    <PageHeader :title="t('plugins.title')" />

    <div class="plugins-page__controls">
      <v-select
        v-model="instanceId"
        :items="instances"
        item-title="name"
        item-value="id"
        :label="t('plugins.instance')"
        variant="outlined"
        density="compact"
        hide-details
        class="plugins-page__instance"
      />
      <v-btn-toggle
        v-model="tab"
        mandatory
        divided
        density="compact"
        variant="outlined"
        color="primary"
        class="plugins-page__tabs"
      >
        <v-btn size="small" value="market">
          {{ t("plugins.tabs.market") }}
        </v-btn>
        <v-btn size="small" value="installed">
          {{ t("plugins.tabs.installed") }}
        </v-btn>
      </v-btn-toggle>
    </div>

    <v-alert
      v-if="selectedInstance && !canManage"
      type="warning"
      variant="tonal"
      density="comfortable"
      class="plugins-page__banner"
    >
      {{ t("plugins.instanceRunning") }}
    </v-alert>

    <template v-if="tab === 'market'">
      <div class="plugins-page__filters">
        <v-text-field
          v-model="search"
          :label="t('plugins.search')"
          prepend-inner-icon="mdi-magnify"
          variant="outlined"
          density="compact"
          hide-details
          clearable
          class="plugins-page__search"
        />
        <v-select
          v-model="category"
          :items="categoryItems"
          :label="t('plugins.category')"
          variant="outlined"
          density="compact"
          hide-details
          class="plugins-page__category"
        />
      </div>

      <v-progress-linear
        v-if="marketLoading"
        indeterminate
        color="primary"
        height="5"
        rounded
        class="plugins-page__progress"
      />
      <v-alert
        v-else-if="marketError"
        type="error"
        variant="tonal"
        density="comfortable"
      >
        {{ marketError }}
      </v-alert>
      <EmptyState
        v-else-if="marketItems.length === 0"
        :text="t('plugins.market.empty')"
        icon="mdi-puzzle-outline"
      />
      <template v-else>
        <div class="plugins-page__grid">
          <article
            v-for="item in marketItems"
            :key="marketKey(item)"
            class="plugin-card page-card"
          >
            <div class="plugin-card__head">
              <img
                v-if="item.logo_url"
                :src="item.logo_url"
                alt=""
                class="plugin-card__logo"
                loading="lazy"
                @error="onLogoError"
              />
              <v-avatar v-else size="32" class="plugin-card__fallback">
                <span>{{ pluginTitle(item).charAt(0).toUpperCase() }}</span>
              </v-avatar>
              <div class="plugin-card__heading">
                <div class="plugin-card__title-row">
                  <p class="plugin-card__title">{{ pluginTitle(item) }}</p>
                  <v-chip v-if="item.official" size="x-small" variant="tonal" color="primary" label>
                    {{ t("plugins.official") }}
                  </v-chip>
                </div>
                <p class="plugin-card__author">{{ item.author }}</p>
              </div>
            </div>
            <p class="plugin-card__desc">{{ item.description }}</p>
            <div class="plugin-card__meta">
              <span class="page-nums">
                {{ t("plugins.downloads", { count: formatCount(item.downloads) }) }}
              </span>
              <span class="page-nums">
                {{ t("plugins.stars", { count: formatCount(item.github_stars) }) }}
              </span>
              <v-chip
                v-if="item.latest_version"
                size="x-small"
                variant="tonal"
                label
              >
                v{{ item.latest_version }}
              </v-chip>
              <v-chip
                v-if="item.astrbot_version"
                size="x-small"
                variant="tonal"
                color="warning"
                label
                :title="t('plugins.requiresVersion')"
              >
                {{ item.astrbot_version }}
              </v-chip>
            </div>
            <div class="plugin-card__actions">
              <v-chip
                v-if="installedById.has(item.plugin_id) && instanceId"
                size="x-small"
                variant="tonal"
                color="success"
                label
              >
                {{ t("plugins.installed") }}
              </v-chip>
              <v-spacer />
              <v-btn
                v-if="item.repository_url"
                variant="text"
                size="small"
                :aria-label="t('plugins.openRepo')"
                @click="openRepo(item.repository_url)"
              >
                {{ t("plugins.repo") }}
              </v-btn>
              <v-btn
                color="primary"
                variant="tonal"
                size="small"
                :disabled="!canManage || !item.download_url || installing"
                @click="confirmInstall(item)"
              >
                {{ t("plugins.install") }}
              </v-btn>
            </div>
          </article>
        </div>
        <div v-if="canLoadMore || marketLoading" class="plugins-page__more">
          <v-btn
            variant="outlined"
            :loading="marketLoading"
            :disabled="!canLoadMore"
            @click="loadMore()"
          >
            {{ t("plugins.loadMore") }}
          </v-btn>
        </div>
      </template>
    </template>

    <template v-else>
      <EmptyState
        v-if="!instanceId"
        :text="t('plugins.noInstance')"
        icon="mdi-cursor-default-click-outline"
      />
      <template v-else>
        <v-progress-linear
          v-if="installedLoading"
          indeterminate
          color="primary"
          height="5"
          rounded
        />
        <v-alert
          v-else-if="installedError"
          type="error"
          variant="tonal"
          density="comfortable"
        >
          {{ installedError }}
        </v-alert>
        <EmptyState
          v-else-if="installed.length === 0"
          :text="t('plugins.installedList.empty')"
          icon="mdi-puzzle-outline"
        />
        <div v-else class="plugins-page__grid">
          <article
            v-for="plugin in installed"
            :key="plugin.dir_name"
            class="plugin-card page-card"
          >
            <div class="plugin-card__head">
              <v-avatar size="32" class="plugin-card__fallback">
                <span>{{ installedTitle(plugin).charAt(0).toUpperCase() }}</span>
              </v-avatar>
              <div class="plugin-card__heading">
                <div class="plugin-card__title-row">
                  <p class="plugin-card__title">{{ installedTitle(plugin) }}</p>
                  <v-chip
                    v-if="plugin.version"
                    size="x-small"
                    variant="tonal"
                    label
                  >
                    v{{ plugin.version }}
                  </v-chip>
                </div>
                <p class="plugin-card__author">{{ plugin.author || plugin.dir_name }}</p>
              </div>
              <v-switch
                :model-value="plugin.enabled"
                inset
                color="primary"
                hide-details
                density="compact"
                class="plugin-card__switch"
                :disabled="!canManage || togglingDir === plugin.dir_name"
                :aria-label="t('plugins.toggleAria', { name: installedTitle(plugin) })"
                @update:model-value="(value: unknown) => togglePlugin(plugin, value === true)"
              />
            </div>
            <p class="plugin-card__desc">{{ plugin.desc || "—" }}</p>
            <div class="plugin-card__actions">
              <v-spacer />
              <v-btn
                variant="text"
                size="small"
                :loading="installingId === plugin.dir_name"
                :disabled="!canManage || installing || !plugin.plugin_id"
                @click="checkUpdate(plugin)"
              >
                {{ t("plugins.update") }}
              </v-btn>
              <v-btn
                variant="text"
                size="small"
                color="error"
                :disabled="!canManage"
                @click="confirmUninstall(plugin)"
              >
                {{ t("plugins.uninstall") }}
              </v-btn>
            </div>
          </article>
        </div>
      </template>
    </template>

    <ConfirmDialog
      :model-value="pendingInstall !== null"
      :title="t('plugins.installConfirm.title')"
      :message="
        pendingInstall
          ? t('plugins.installConfirm.message', {
              name: pluginTitle(pendingInstall),
              instance: selectedInstance?.name ?? '',
            })
          : ''
      "
      :confirm-label="t('plugins.install')"
      :loading="installing"
      @update:model-value="(value: boolean) => !value && (pendingInstall = null)"
      @confirm="performInstall"
    />

    <ConfirmDialog
      :model-value="pendingUninstall !== null"
      :title="t('plugins.uninstallConfirm.title')"
      :message="
        pendingUninstall
          ? t('plugins.uninstallConfirm.message', {
              name: installedTitle(pendingUninstall),
            })
          : ''
      "
      :confirm-label="t('plugins.uninstall')"
      color="error"
      :loading="uninstalling"
      @update:model-value="(value: boolean) => !value && (pendingUninstall = null)"
      @confirm="performUninstall"
    >
      <div class="plugins-page__uninstall-options">
        <v-checkbox
          v-model="uninstallDeleteConfig"
          density="compact"
          color="primary"
          hide-details
          :label="t('plugins.uninstallConfirm.deleteConfig')"
        />
        <v-checkbox
          v-model="uninstallDeleteData"
          density="compact"
          color="primary"
          hide-details
          :label="t('plugins.uninstallConfirm.deleteData')"
        />
      </div>
    </ConfirmDialog>
  </div>
</template>

<style scoped>
.plugins-page__controls {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}

.plugins-page__instance {
  max-width: 280px;
  flex: 1 1 200px;
}

.plugins-page__banner {
  margin-bottom: 12px;
}

.plugins-page__filters {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}

.plugins-page__search {
  flex: 1 1 220px;
}

.plugins-page__category {
  flex: 1 1 200px;
  max-width: 280px;
}

.plugins-page__progress {
  margin-bottom: 12px;
}

.plugins-page__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 12px;
}

.plugins-page__more {
  display: flex;
  justify-content: center;
  margin-top: 16px;
}

.plugins-page__uninstall-options {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.plugin-card {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 14px;
}

.plugin-card__head {
  display: flex;
  align-items: flex-start;
  gap: 10px;
}

.plugin-card__logo {
  width: 32px;
  height: 32px;
  border-radius: var(--astrbot-radius-control);
  object-fit: cover;
  flex: none;
}

.plugin-card__logo--broken {
  display: none;
}

.plugin-card__fallback {
  flex: none;
}

.plugin-card__heading {
  min-width: 0;
  flex: 1;
}

.plugin-card__title-row {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.plugin-card__title {
  margin: 0;
  color: rgb(var(--v-theme-on-surface));
  font-size: 0.875rem;
  font-weight: 600;
  line-height: 1.3;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.plugin-card__author {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.55);
  font-size: 0.75rem;
  line-height: 1.3;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.plugin-card__switch {
  flex: none;
}

.plugin-card__desc {
  margin: 0;
  color: rgba(var(--v-theme-on-surface), 0.7);
  font-size: 0.8125rem;
  line-height: 1.45;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
  min-height: 2.4em;
}

.plugin-card__meta {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
  color: rgba(var(--v-theme-on-surface), 0.55);
  font-size: 0.75rem;
}

.plugin-card__actions {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: auto;
}
</style>
