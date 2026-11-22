<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ConfirmDialog from "../common/ConfirmDialog.vue";
import EmptyState from "../common/EmptyState.vue";
import faviconUrl from "../../assets/favicon.svg";
import { toUserMessage } from "../../services/errors";
import {
  fetchPluginMarket,
  installInstancePlugin,
  listInstancePlugins,
  setInstancePluginEnabled,
  uninstallInstancePlugin,
} from "../../services/plugins";
import { useLauncherStore } from "../../stores/launcher";
import { showErrorToast, showToast } from "../../stores/toast";
import type { InstanceStatus } from "../../types/launcher";
import type { InstancePlugin } from "../../types/plugins";

const { t } = useI18n();
const { instances } = useLauncherStore();

const instanceId = ref<string | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);
const installed = ref<InstancePlugin[]>([]);

const togglingDir = ref<string | null>(null);
const updatingDir = ref<string | null>(null);

const pendingUninstall = ref<InstancePlugin | null>(null);
const uninstalling = ref(false);
const uninstallDeleteConfig = ref(false);
const uninstallDeleteData = ref(false);

const selectedInstance = computed<InstanceStatus | null>(() => {
  if (!instanceId.value) {
    return null;
  }
  return instances.value.find((item) => item.id === instanceId.value) ?? null;
});

/** Installs write instance files, so a running instance rejects them. */
const canManage = computed(() => selectedInstance.value?.state === "stopped");

async function loadInstalled(): Promise<void> {
  if (!instanceId.value) {
    installed.value = [];
    return;
  }
  loading.value = true;
  error.value = null;
  try {
    installed.value = await listInstancePlugins(instanceId.value);
  } catch (requestError) {
    installed.value = [];
    error.value = toUserMessage(requestError);
  } finally {
    loading.value = false;
  }
}

async function togglePlugin(plugin: InstancePlugin, enabled: boolean): Promise<void> {
  if (!instanceId.value) {
    return;
  }
  togglingDir.value = plugin.dir_name;
  try {
    await setInstancePluginEnabled(instanceId.value, plugin.dir_name, enabled);
    plugin.enabled = enabled;
  } catch (requestError) {
    showErrorToast(requestError);
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
    showToast(t("plugins.toasts.uninstalled", { name: plugin.name }));
    pendingUninstall.value = null;
    await loadInstalled();
  } catch (requestError) {
    showErrorToast(requestError);
  } finally {
    uninstalling.value = false;
  }
}

async function checkUpdate(plugin: InstancePlugin): Promise<void> {
  if (!canManage.value || !instanceId.value || updatingDir.value) {
    return;
  }
  updatingDir.value = plugin.dir_name;
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
    await installInstancePlugin(instanceId.value, {
      plugin_id: match.plugin_id,
      download_url: match.download_url,
    });
    showToast(
      t("plugins.toasts.updated", {
        name: plugin.name,
        version: match.latest_version ?? "",
      }),
    );
    await loadInstalled();
  } catch (requestError) {
    showErrorToast(requestError);
  } finally {
    updatingDir.value = null;
  }
}

watch(instanceId, () => {
  void loadInstalled();
});
</script>

<template>
  <div class="installed-plugins">
    <div class="installed-plugins__controls">
      <v-select
        v-model="instanceId"
        :items="instances"
        item-title="name"
        item-value="id"
        :label="t('plugins.instance')"
        variant="outlined"
        density="compact"
        hide-details
        class="installed-plugins__instance"
      />
    </div>

    <v-alert
      v-if="selectedInstance && !canManage"
      type="warning"
      variant="tonal"
      density="comfortable"
      class="installed-plugins__banner"
    >
      {{ t("plugins.instanceRunning") }}
    </v-alert>

    <EmptyState
      v-if="!instanceId"
      :text="t('plugins.noInstance')"
      icon="mdi-cursor-default-click-outline"
    />
    <template v-else>
      <v-progress-linear
        v-if="loading"
        indeterminate
        color="primary"
        height="5"
        rounded
      />
      <v-alert v-else-if="error" type="error" variant="tonal" density="comfortable">
        {{ error }}
      </v-alert>
      <EmptyState
        v-else-if="installed.length === 0"
        :text="t('plugins.installedList.empty')"
        icon="mdi-puzzle-outline"
      />
      <div v-else class="installed-plugins__grid">
        <article
          v-for="plugin in installed"
          :key="plugin.dir_name"
          class="plugin-card page-card"
        >
          <div class="plugin-card__head">
            <img :src="faviconUrl" alt="" class="plugin-card__logo" />
            <div class="plugin-card__heading">
              <div class="plugin-card__title-row">
                <p class="plugin-card__title">{{ plugin.name }}</p>
                <v-chip v-if="plugin.version" size="x-small" variant="tonal" label>
                  v{{ plugin.version }}
                </v-chip>
              </div>
              <p class="plugin-card__author">
                {{ plugin.author || plugin.dir_name }}
              </p>
            </div>
            <v-switch
              :model-value="plugin.enabled"
              inset
              color="primary"
              hide-details
              density="compact"
              class="plugin-card__switch"
              :disabled="!canManage || togglingDir === plugin.dir_name"
              :aria-label="t('plugins.toggleAria', { name: plugin.name })"
              @update:model-value="(value: unknown) => togglePlugin(plugin, value === true)"
            />
          </div>
          <p class="plugin-card__desc">{{ plugin.desc || "—" }}</p>
          <div class="plugin-card__actions">
            <v-spacer />
            <v-btn
              variant="text"
              size="small"
              :loading="updatingDir === plugin.dir_name"
              :disabled="!canManage || updatingDir !== null || !plugin.plugin_id"
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

    <ConfirmDialog
      :model-value="pendingUninstall !== null"
      :title="t('plugins.uninstallConfirm.title')"
      :message="
        pendingUninstall
          ? t('plugins.uninstallConfirm.message', { name: pendingUninstall.name })
          : ''
      "
      :confirm-label="t('plugins.uninstall')"
      color="error"
      :loading="uninstalling"
      @update:model-value="(value: boolean) => !value && (pendingUninstall = null)"
      @confirm="performUninstall"
    >
      <div class="installed-plugins__uninstall-options">
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
.installed-plugins__controls {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}

.installed-plugins__instance {
  max-width: 280px;
  flex: 1 1 200px;
}

.installed-plugins__banner {
  margin-bottom: 12px;
}

.installed-plugins__grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 12px;
}

.installed-plugins__uninstall-options {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
</style>
