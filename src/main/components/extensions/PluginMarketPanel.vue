<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ConfirmDialog from "../common/ConfirmDialog.vue";
import EmptyState from "../common/EmptyState.vue";
import TargetInstanceList from "./TargetInstanceList.vue";
import faviconUrl from "../../assets/favicon.svg";
import { openExternalUrl } from "../../services/app";
import { toUserMessage } from "../../services/errors";
import {
  fetchPluginMarket,
  installInstancePlugin,
  listInstancePlugins,
} from "../../services/plugins";
import { useLauncherStore } from "../../stores/launcher";
import { showErrorToast, showToast } from "../../stores/toast";
import { isSamePlugin } from "../../utils/plugins";
import type { InstanceStatus } from "../../types/launcher";
import type { PluginMarketItem } from "../../types/plugins";

const MARKET_PAGE_SIZE = 30;

const { t } = useI18n();
const { instances } = useLauncherStore();

const search = ref("");
const category = ref<string>("");
const categories = ref<{ name: string; icon: string; count: number }[]>([]);

const loading = ref(false);
const error = ref<string | null>(null);
const items = ref<PluginMarketItem[]>([]);
const total = ref(0);
const page = ref(1);

const pendingInstall = ref<PluginMarketItem | null>(null);
const installTarget = ref<string | null>(null);
const installing = ref(false);
const preparingTargets = ref(false);
/** Stopped instances that already have the pending plugin, by instance id. */
const targetInstalled = ref<Map<string, boolean>>(new Map());

const stoppedInstances = computed<InstanceStatus[]>(() =>
  instances.value.filter((instance) => instance.state === "stopped"),
);

const totalPages = computed(() =>
  Math.max(1, Math.ceil(total.value / MARKET_PAGE_SIZE)),
);

const categoryItems = computed(() => [
  { title: t("plugins.allCategories"), value: "" },
  ...categories.value.map((entry) => ({
    title: `${entry.name} (${formatCount(entry.count)})`,
    value: entry.name,
  })),
]);

function pluginTitle(item: PluginMarketItem): string {
  return item.display_name?.trim() || item.name;
}

function formatCount(value: number): string {
  return value.toLocaleString();
}

let loadGeneration = 0;

/** Fetch the current page, replacing the grid instead of appending. */
async function load(): Promise<void> {
  // Paging and search debounce can overlap; only the newest request may win.
  const generation = ++loadGeneration;
  loading.value = true;
  error.value = null;
  try {
    const result = await fetchPluginMarket(
      page.value,
      MARKET_PAGE_SIZE,
      search.value.trim() || undefined,
      category.value || undefined,
    );
    if (generation !== loadGeneration) {
      return;
    }
    categories.value = result.categories.map((entry) => ({
      name: entry.name,
      icon: entry.icon,
      count: result.category_counts[entry.name] ?? 0,
    }));
    total.value = result.total;
    items.value = result.items;
  } catch (requestError) {
    if (generation === loadGeneration) {
      error.value = toUserMessage(requestError);
    }
  } finally {
    if (generation === loadGeneration) {
      loading.value = false;
    }
  }
}

function goToPage(target: number): void {
  page.value = Math.min(Math.max(target, 1), totalPages.value);
  void load();
}

async function openRepo(url: string | null): Promise<void> {
  if (!url) {
    return;
  }
  try {
    await openExternalUrl(url);
  } catch (cause) {
    showErrorToast(cause);
  }
}

/** Fall back to the app favicon when a market logo fails to load. */
function onLogoError(event: Event): void {
  const target = event.target as HTMLImageElement;
  if (!target.src.endsWith(faviconUrl)) {
    target.src = faviconUrl;
  }
}

async function confirmInstall(item: PluginMarketItem): Promise<void> {
  if (!item.download_url || installing.value) {
    return;
  }
  pendingInstall.value = item;
  installTarget.value = null;
  targetInstalled.value = new Map();

  // Mark the stopped instances that already have this plugin.
  preparingTargets.value = true;
  const checks = stoppedInstances.value.map(async (instance) => {
    try {
      const installed = await listInstancePlugins(instance.id);
      targetInstalled.value.set(
        instance.id,
        installed.some((plugin) => isSamePlugin(plugin, item)),
      );
    } catch {
      targetInstalled.value.set(instance.id, false);
    }
  });
  await Promise.all(checks);
  preparingTargets.value = false;
}

async function performInstall(): Promise<void> {
  const item = pendingInstall.value;
  const instanceId = installTarget.value;
  if (!item || !instanceId) {
    return;
  }
  installing.value = true;
  try {
    await installInstancePlugin(instanceId, {
      download_url: item.download_url ?? "",
    });
    showToast(t("plugins.toasts.installed", { name: pluginTitle(item) }));
    pendingInstall.value = null;
  } catch (requestError) {
    showErrorToast(requestError);
  } finally {
    installing.value = false;
  }
}

let searchTimer: ReturnType<typeof setTimeout> | undefined;

watch(search, () => {
  clearTimeout(searchTimer);
  searchTimer = setTimeout(() => {
    goToPage(1);
  }, 300);
});

watch(category, () => {
  goToPage(1);
});

onBeforeUnmount(() => {
  clearTimeout(searchTimer);
});

void load();
</script>

<template>
  <div class="plugin-market">
    <div class="plugin-market__filters">
      <v-text-field
        v-model="search"
        :label="t('plugins.search')"
        prepend-inner-icon="mdi-magnify"
        variant="outlined"
        density="compact"
        hide-details
        clearable
        class="plugin-market__search"
      />
      <v-select
        v-model="category"
        :items="categoryItems"
        :label="t('plugins.category')"
        variant="outlined"
        density="compact"
        hide-details
        class="plugin-market__category"
      />
    </div>

    <v-progress-linear
      v-if="loading"
      indeterminate
      color="primary"
      height="5"
      rounded
      class="plugin-market__progress"
    />
    <v-alert
      v-else-if="error"
      type="error"
      variant="tonal"
      density="comfortable"
    >
      {{ error }}
    </v-alert>
    <EmptyState
      v-else-if="items.length === 0"
      :text="t('plugins.market.empty')"
      icon="mdi-puzzle-outline"
    />
    <template v-else>
      <div class="plugin-market__grid">
        <article
          v-for="item in items"
          :key="item.plugin_id"
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
            <img
              v-else
              :src="faviconUrl"
              alt=""
              class="plugin-card__logo"
            />
            <div class="plugin-card__heading">
              <div class="plugin-card__title-row">
                <p class="plugin-card__title">{{ pluginTitle(item) }}</p>
                <v-chip
                  v-if="item.official"
                  size="x-small"
                  variant="tonal"
                  color="primary"
                  label
                >
                  {{ t("plugins.official") }}
                </v-chip>
              </div>
              <p class="plugin-card__author">{{ item.author }}</p>
            </div>
          </div>
          <p class="plugin-card__desc">{{ item.description }}</p>
          <div class="plugin-card__meta">
            <span
              class="page-nums plugin-card__stat"
              :title="t('plugins.downloads', { count: formatCount(item.downloads) })"
            >
              <v-icon icon="mdi-download-outline" size="14" aria-hidden="true" />
              {{ formatCount(item.downloads) }}
            </span>
            <span
              class="page-nums plugin-card__stat"
              :title="t('plugins.stars', { count: formatCount(item.github_stars) })"
            >
              <v-icon icon="mdi-star-outline" size="14" aria-hidden="true" />
              {{ formatCount(item.github_stars) }}
            </span>
            <v-chip v-if="item.latest_version" size="x-small" variant="tonal" label>
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
              :disabled="!item.download_url || installing"
              @click="confirmInstall(item)"
            >
              {{ t("plugins.install") }}
            </v-btn>
          </div>
        </article>
      </div>
      <div v-if="totalPages > 1" class="plugin-market__pagination">
        <v-pagination
          :model-value="page"
          :length="totalPages"
          :total-visible="7"
          size="small"
          @update:model-value="goToPage"
        />
      </div>
    </template>

    <ConfirmDialog
      :model-value="pendingInstall !== null"
      :title="t('plugins.installConfirm.title')"
      :message="
        pendingInstall
          ? t('plugins.installConfirm.pickInstance', {
              name: pluginTitle(pendingInstall),
            })
          : ''
      "
      :confirm-label="t('plugins.install')"
      :confirm-disabled="installTarget === null"
      :loading="installing || preparingTargets"
      max-width="520"
      @update:model-value="(value: boolean) => !value && (pendingInstall = null)"
      @confirm="performInstall"
    >
      <TargetInstanceList
        :instances="instances"
        v-model="installTarget"
      >
        <template #append="{ instance }">
          <v-chip
            v-if="targetInstalled.get(instance.id)"
            size="x-small"
            variant="tonal"
            color="success"
            label
          >
            {{ t("plugins.installed") }}
          </v-chip>
        </template>
      </TargetInstanceList>
    </ConfirmDialog>
  </div>
</template>

<style scoped>
.plugin-market__filters {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}

.plugin-market__search {
  flex: 1 1 220px;
}

.plugin-market__category {
  flex: 1 1 200px;
  max-width: 280px;
}

.plugin-market__progress {
  margin-bottom: 12px;
}

.plugin-market__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 12px;
}

.plugin-market__pagination {
  display: flex;
  justify-content: center;
  margin-top: 16px;
}
</style>
