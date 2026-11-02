<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { useI18n } from "vue-i18n";
import ConfirmDialog from "../components/common/ConfirmDialog.vue";
import EmptyState from "../components/common/EmptyState.vue";
import PageHeader from "../components/common/PageHeader.vue";
import DownloadProgressBar from "../components/versions/DownloadProgressBar.vue";
import ReleaseNotesDialog from "../components/versions/ReleaseNotesDialog.vue";
import { useOperation } from "../composables/useOperation";
import { translateDynamicKey } from "../i18n/dynamic";
import { openFolder } from "../services/app";
import {
  clearUvCache,
  installComponent,
  reinstallComponent,
  uninstallComponent,
} from "../services/components";
import { isDownloadCancelled, toUserMessage } from "../services/errors";
import {
  cancelVersionDownload,
  fetchReleases,
  installVersion,
  pauseVersionDownload,
  resumeVersionDownload,
  uninstallVersion,
} from "../services/versions";
import { useLauncherStore } from "../stores/launcher";
import { useProgressStore } from "../stores/progress";
import { showErrorToast, showToast } from "../stores/toast";
import type {
  ComponentActionResult,
  ComponentId,
  ComponentStatus,
  DownloadProgress,
  GitHubRelease,
  InstanceStatus,
  InstalledVersion,
} from "../types/launcher";
import { formatDateTime, formatFileSize } from "../utils/format";
import { dirname } from "../utils/path";

const CLEAR_UV_CACHE_KEY = "component:uv:cache";

const {
  instances,
  versions,
  components,
  loading: snapshotLoading,
  error: snapshotError,
  refresh: refreshSnapshot,
} = useLauncherStore();
const { downloads } = useProgressStore();
const { isBusy, run } = useOperation();
const { t } = useI18n();

const releases = ref<GitHubRelease[]>([]);
const releasesLoading = ref(false);
const releasesError = ref<string | null>(null);

const notesOpen = ref(false);
const notesRelease = ref<GitHubRelease | null>(null);

type PendingAction =
  | { kind: "reinstall-component"; id: ComponentId; name: string }
  | { kind: "uninstall-component"; id: ComponentId; name: string }
  | { kind: "uninstall-version"; version: string };

const pendingAction = ref<PendingAction | null>(null);

interface ComponentRow {
  status: ComponentStatus;
  componentId: ComponentId | null;
  progress: DownloadProgress | null;
  busy: boolean;
}

interface InstalledRow {
  version: InstalledVersion;
  instances: InstanceStatus[];
  busy: boolean;
}

interface ReleaseRow {
  release: GitHubRelease;
  installed: boolean;
  busy: boolean;
  downloading: boolean;
  paused: boolean;
  progress: DownloadProgress | null;
  size: number;
}

function isComponentId(value: string): value is ComponentId {
  return value === "python" || value === "nodejs" || value === "uv";
}

function releaseSize(release: GitHubRelease): number {
  const zipAsset = release.assets.find((asset) =>
    asset.name.toLowerCase().endsWith(".zip"),
  );
  return zipAsset?.size ?? release.assets[0]?.size ?? 0;
}

const componentRows = computed<ComponentRow[]>(() =>
  components.value.map((status) => {
    const componentId = isComponentId(status.id) ? status.id : null;
    return {
      status,
      componentId,
      progress: downloads.value[status.id] ?? null,
      busy: componentId !== null && isBusy(`component:${componentId}`),
    };
  }),
);

const hasUvComponent = computed(() =>
  components.value.some((component) => component.id === "uv"),
);

const installedRows = computed<InstalledRow[]>(() =>
  versions.value.map((version) => ({
    version,
    instances: instances.value.filter(
      (instance) => instance.version === version.version,
    ),
    busy: isBusy(`version:${version.version}`),
  })),
);

const installedVersionSet = computed(
  () => new Set(versions.value.map((version) => version.version)),
);

// Downloads started from this page. Kept separately from the shared busy
// registry so uninstall operations on the same version do not show pause/stop.
const activeDownloads = reactive(new Set<string>());

const releaseRows = computed<ReleaseRow[]>(() =>
  releases.value.map((release) => {
    const progress = downloads.value[release.tag_name] ?? null;
    return {
      release,
      installed: installedVersionSet.value.has(release.tag_name),
      busy: isBusy(`version:${release.tag_name}`),
      downloading: activeDownloads.has(release.tag_name),
      paused: progress?.step === "paused",
      progress,
      size: releaseSize(release),
    };
  }),
);

const allReleasesInstalled = computed(
  () =>
    releaseRows.value.length > 0 &&
    releaseRows.value.every((row) => row.installed),
);

const confirmLoading = computed(() => {
  const action = pendingAction.value;
  if (!action) {
    return false;
  }
  if (action.kind === "uninstall-version") {
    return isBusy(`version:${action.version}`);
  }
  return isBusy(`component:${action.id}`);
});

const confirmOpen = computed({
  get: () => pendingAction.value !== null,
  set: (value: boolean) => {
    if (!value) {
      pendingAction.value = null;
    }
  },
});

const confirmTitle = computed(() => {
  switch (pendingAction.value?.kind) {
    case "reinstall-component":
      return t("versions.confirm.titles.reinstallComponent");
    case "uninstall-component":
      return t("versions.confirm.titles.uninstallComponent");
    case "uninstall-version":
      return t("versions.confirm.titles.uninstallVersion");
    default:
      return "";
  }
});

const confirmMessage = computed(() => {
  const action = pendingAction.value;
  if (!action) {
    return "";
  }
  switch (action.kind) {
    case "reinstall-component":
      return t("versions.confirm.messages.reinstallComponent", {
        name: action.name,
      });
    case "uninstall-component":
      return t("versions.confirm.messages.uninstallComponent", {
        name: action.name,
      });
    case "uninstall-version":
      return t("versions.confirm.messages.uninstallVersion", {
        version: action.version,
      });
  }
});

const confirmLabel = computed(() => {
  switch (pendingAction.value?.kind) {
    case "reinstall-component":
      return t("versions.actions.reinstall");
    case "uninstall-component":
    case "uninstall-version":
      return t("common.actions.uninstall");
    default:
      return t("common.actions.confirm");
  }
});

const confirmColor = computed(() =>
  pendingAction.value?.kind === "reinstall-component" ? "primary" : "error",
);

let releasesRequest = 0;

async function loadReleases(forceRefresh = false): Promise<void> {
  const request = ++releasesRequest;
  releasesLoading.value = true;
  releasesError.value = null;

  try {
    const result = await fetchReleases(forceRefresh);
    if (request === releasesRequest) {
      releases.value = result;
    }
  } catch (cause) {
    if (request === releasesRequest) {
      releasesError.value = toUserMessage(cause);
    }
  } finally {
    if (request === releasesRequest) {
      releasesLoading.value = false;
    }
  }
}

onMounted(() => {
  void loadReleases();
});

function handleRefresh(): void {
  void loadReleases(true);
  void refreshSnapshot();
}

function retrySnapshot(): void {
  void refreshSnapshot();
}

function retryReleases(): void {
  void loadReleases(true);
}

function requestReinstallComponent(id: string, name: string): void {
  if (!isComponentId(id)) {
    return;
  }
  pendingAction.value = { kind: "reinstall-component", id, name };
}

function requestUninstallComponent(id: string, name: string): void {
  if (!isComponentId(id)) {
    return;
  }
  pendingAction.value = { kind: "uninstall-component", id, name };
}

function requestUninstallVersion(version: string): void {
  pendingAction.value = { kind: "uninstall-version", version };
}

async function runComponentCommand(
  componentId: ComponentId,
  task: () => Promise<ComponentActionResult>,
): Promise<boolean> {
  const results: ComponentActionResult[] = [];
  const ok = await run(`component:${componentId}`, async () => {
    results.push(await task());
  });
  const result = results.pop();

  if (!ok || result === undefined) {
    return false;
  }

  let key = result.key;
  if (result.versions.length === 0) {
    switch (key) {
      case "components.result.installed":
        key = "components.result.installedWithoutVersions";
        break;
      case "components.result.uninstalled":
        key = "components.result.uninstalledWithoutVersions";
        break;
      case "components.result.reinstalled":
        key = "components.result.reinstalledWithoutVersions";
        break;
      default:
        break;
    }
  }

  const fallback =
    result.message.trim() !== ""
      ? result.message
      : t("common.operationComplete");
  const text = translateDynamicKey(key, fallback, {
    name: t("components.name." + componentId),
    versions: result.versions.join(", "),
  });
  showToast(text, "success");
  return true;
}

function handleInstallComponent(id: string): void {
  if (!isComponentId(id)) {
    return;
  }
  const componentId = id;
  void runComponentCommand(componentId, () => installComponent(componentId));
}

function handleClearUvCache(): void {
  void run(CLEAR_UV_CACHE_KEY, clearUvCache, {
    success: t("versions.toasts.uvCacheCleared"),
  });
}

async function handleConfirm(): Promise<void> {
  const action = pendingAction.value;
  if (!action) {
    return;
  }

  let ok = false;
  switch (action.kind) {
    case "reinstall-component":
      ok = await runComponentCommand(action.id, () =>
        reinstallComponent(action.id),
      );
      break;
    case "uninstall-component":
      ok = await runComponentCommand(action.id, () =>
        uninstallComponent(action.id),
      );
      break;
    case "uninstall-version":
      ok = await run(
        `version:${action.version}`,
        () => uninstallVersion(action.version),
        {
          success: t("versions.toasts.versionUninstalled", {
            version: action.version,
          }),
        },
      );
      break;
  }

  if (ok) {
    pendingAction.value = null;
  }
}

async function handleOpenVersionFolder(
  version: InstalledVersion,
): Promise<void> {
  try {
    await openFolder(dirname(version.zip_path));
  } catch (cause) {
    showErrorToast(cause);
  }
}

function handleInstallRelease(release: GitHubRelease): void {
  const tag = release.tag_name;
  if (installedVersionSet.value.has(tag) || activeDownloads.has(tag)) {
    return;
  }

  activeDownloads.add(tag);
  void run(`version:${tag}`, () => installVersion(release), {
    success: t("versions.toasts.versionInstalled", { version: tag }),
    onError: (error) => {
      if (!isDownloadCancelled(error)) {
        return false;
      }
      showToast(t("versions.toasts.downloadStopped", { version: tag }), "info");
      return true;
    },
  }).finally(() => {
    activeDownloads.delete(tag);
  });
}

async function handleTogglePause(row: ReleaseRow): Promise<void> {
  const tag = row.release.tag_name;
  try {
    await (row.paused
      ? resumeVersionDownload(tag)
      : pauseVersionDownload(tag));
  } catch (cause) {
    showErrorToast(cause);
  }
}

async function handleStopDownload(row: ReleaseRow): Promise<void> {
  try {
    await cancelVersionDownload(row.release.tag_name);
  } catch (cause) {
    showErrorToast(cause);
  }
}

function openReleaseNotes(release: GitHubRelease): void {
  notesRelease.value = release;
  notesOpen.value = true;
}
</script>

<template>
  <div class="page-shell">
    <PageHeader :title="t('versions.title')">
      <template #actions>
        <v-btn
          variant="tonal"
          color="primary"
          prepend-icon="mdi-refresh"
          :loading="releasesLoading"
          @click="handleRefresh"
        >
          {{ t("common.actions.refresh") }}
        </v-btn>
      </template>
    </PageHeader>

    <v-alert
      v-if="snapshotError"
      class="version-alert"
      type="error"
      variant="tonal"
      density="comfortable"
    >
      {{ snapshotError }}
      <template #append>
        <v-btn
          size="small"
          variant="text"
          :loading="snapshotLoading"
          @click="retrySnapshot"
        >
          {{ t("common.actions.retry") }}
        </v-btn>
      </template>
    </v-alert>

    <section class="page-section">
      <div class="page-section-head">
        <h2 class="page-section-title version-section-title">
          {{ t("versions.sections.components") }}
        </h2>
      </div>
      <div class="page-card version-card">
        <div
          v-if="snapshotLoading && componentRows.length === 0"
          class="version-loading"
        >
          <v-progress-linear indeterminate color="primary" height="5" rounded />
          <div class="page-muted">{{ t("versions.loading") }}</div>
        </div>
        <EmptyState
          v-else-if="componentRows.length === 0"
          :text="t('versions.components.empty')"
          icon="mdi-puzzle-outline"
        />
        <div v-else class="version-rows">
          <div
            v-for="row in componentRows"
            :key="row.status.id"
            class="version-row"
          >
            <div class="version-row__main">
              <div class="version-row__title">
                <span>{{ row.status.display_name }}</span>
                <v-chip
                  size="x-small"
                  label
                  :color="row.status.installed ? 'success' : 'secondary'"
                  variant="tonal"
                >
                  {{
                    row.status.installed
                      ? t("versions.status.installed")
                      : t("versions.status.notInstalled")
                  }}
                </v-chip>
              </div>
              <div class="version-row__sub">
                {{
                  translateDynamicKey(
                    "components.description." + row.status.id,
                    row.status.description,
                  )
                }}
              </div>
              <DownloadProgressBar :progress="row.progress" />
            </div>
            <div class="version-row__actions">
              <template v-if="row.componentId">
                <v-btn
                  v-if="!row.status.installed"
                  size="small"
                  variant="tonal"
                  color="primary"
                  :loading="row.busy"
                  @click="handleInstallComponent(row.status.id)"
                >
                  {{ t("versions.actions.download") }}
                </v-btn>
                <template v-else>
                  <v-btn
                    size="small"
                    variant="tonal"
                    color="primary"
                    :disabled="row.busy"
                    @click="
                      requestReinstallComponent(
                        row.status.id,
                        row.status.display_name,
                      )
                    "
                  >
                    {{ t("versions.actions.reinstall") }}
                  </v-btn>
                  <v-btn
                    size="small"
                    variant="tonal"
                    color="error"
                    :disabled="row.busy"
                    @click="
                      requestUninstallComponent(
                        row.status.id,
                        row.status.display_name,
                      )
                    "
                  >
                    {{ t("common.actions.uninstall") }}
                  </v-btn>
                </template>
              </template>
            </div>
          </div>
        </div>
        <div v-if="hasUvComponent" class="version-card__footer">
          <v-btn
            size="small"
            variant="text"
            prepend-icon="mdi-broom"
            :loading="isBusy(CLEAR_UV_CACHE_KEY)"
            @click="handleClearUvCache"
          >
            {{ t("versions.actions.clearUvCache") }}
          </v-btn>
        </div>
      </div>
    </section>

    <section class="page-section">
      <div class="page-section-head">
        <h2 class="page-section-title version-section-title">
          {{ t("versions.sections.installed") }}
        </h2>
      </div>
      <div class="page-card version-card">
        <div
          v-if="snapshotLoading && installedRows.length === 0"
          class="version-loading"
        >
          <v-progress-linear indeterminate color="primary" height="5" rounded />
          <div class="page-muted">{{ t("versions.loading") }}</div>
        </div>
        <EmptyState
          v-else-if="installedRows.length === 0"
          :text="t('versions.installed.empty')"
          icon="mdi-archive-outline"
        />
        <div v-else class="version-rows">
          <div
            v-for="row in installedRows"
            :key="row.version.version"
            class="version-row"
          >
            <div class="version-row__main">
              <div class="version-row__title">
                <span>{{ row.version.version }}</span>
              </div>
              <div class="version-row__instances">
                <template v-if="row.instances.length > 0">
                  <v-chip
                    v-for="instance in row.instances"
                    :key="instance.id"
                    size="x-small"
                    variant="tonal"
                    label
                  >
                    {{ instance.name }}
                  </v-chip>
                </template>
                <span v-else class="page-muted">
                  {{ t("versions.installed.unused") }}
                </span>
              </div>
            </div>
            <div class="version-row__actions">
              <v-btn
                size="small"
                variant="text"
                prepend-icon="mdi-folder-open-outline"
                @click="handleOpenVersionFolder(row.version)"
              >
                {{ t("versions.actions.openFolder") }}
              </v-btn>
              <v-btn
                size="small"
                variant="tonal"
                color="error"
                :disabled="row.busy"
                @click="requestUninstallVersion(row.version.version)"
              >
                {{ t("common.actions.uninstall") }}
              </v-btn>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section class="page-section">
      <div class="page-section-head">
        <h2 class="page-section-title version-section-title">
          {{ t("versions.sections.available") }}
        </h2>
      </div>
      <div class="page-card version-card">
        <v-alert
          v-if="releasesError"
          class="version-alert"
          type="error"
          variant="tonal"
          density="comfortable"
        >
          {{ releasesError }}
          <template #append>
            <v-btn
              size="small"
              variant="text"
              :loading="releasesLoading"
              @click="retryReleases"
            >
              {{ t("common.actions.retry") }}
            </v-btn>
          </template>
        </v-alert>

        <div
          v-if="releasesLoading && releaseRows.length === 0"
          class="version-loading"
        >
          <v-progress-linear indeterminate color="primary" height="5" rounded />
          <div class="page-muted">{{ t("versions.releases.loading") }}</div>
        </div>
        <EmptyState
          v-else-if="!releasesError && releaseRows.length === 0"
          :text="t('versions.releases.empty')"
          icon="mdi-cloud-off-outline"
        />
        <EmptyState
          v-else-if="allReleasesInstalled"
          :text="t('versions.releases.allInstalled')"
          icon="mdi-check-circle-outline"
        />
        <div v-else-if="releaseRows.length > 0" class="version-rows">
          <div
            v-for="row in releaseRows"
            :key="row.release.tag_name"
            class="version-row"
          >
            <div class="version-row__main">
              <div class="version-row__title">
                <span>{{ row.release.tag_name }}</span>
                <v-chip
                  v-if="row.release.prerelease"
                  size="x-small"
                  color="warning"
                  variant="tonal"
                  label
                >
                  {{ t("versions.status.prerelease") }}
                </v-chip>
                <v-chip
                  v-if="row.installed"
                  size="x-small"
                  color="success"
                  variant="tonal"
                  label
                >
                  {{ t("versions.status.installed") }}
                </v-chip>
              </div>
              <div class="version-row__sub">
                {{ row.release.name || row.release.tag_name }}
              </div>
              <div class="version-row__meta page-muted">
                <span>{{ formatDateTime(row.release.published_at) }}</span>
                <span>{{ formatFileSize(row.size) }}</span>
              </div>
              <DownloadProgressBar :progress="row.progress" />
            </div>
            <div class="version-row__actions">
              <v-btn
                size="small"
                variant="text"
                @click="openReleaseNotes(row.release)"
              >
                {{ t("versions.actions.details") }}
              </v-btn>
              <template v-if="row.downloading">
                <v-btn
                  size="small"
                  variant="tonal"
                  color="primary"
                  :prepend-icon="row.paused ? 'mdi-play' : 'mdi-pause'"
                  @click="handleTogglePause(row)"
                >
                  {{
                    row.paused
                      ? t("versions.actions.resume")
                      : t("versions.actions.pause")
                  }}
                </v-btn>
                <v-btn
                  size="small"
                  variant="tonal"
                  color="error"
                  prepend-icon="mdi-stop"
                  @click="handleStopDownload(row)"
                >
                  {{ t("versions.actions.stop") }}
                </v-btn>
              </template>
              <v-btn
                v-else
                size="small"
                variant="tonal"
                color="primary"
                :disabled="row.installed || row.busy"
                :loading="row.busy"
                @click="handleInstallRelease(row.release)"
              >
                {{ t("common.actions.install") }}
              </v-btn>
            </div>
          </div>
        </div>
      </div>
    </section>

    <ConfirmDialog
      v-model="confirmOpen"
      :title="confirmTitle"
      :message="confirmMessage"
      :confirm-label="confirmLabel"
      :color="confirmColor"
      :loading="confirmLoading"
      @confirm="handleConfirm"
    />

    <ReleaseNotesDialog v-model="notesOpen" :release="notesRelease" />
  </div>
</template>

<style scoped>
.version-section-title {
  margin: 0;
}

.version-card {
  overflow: hidden;
}

.version-card__footer {
  display: flex;
  justify-content: flex-end;
  padding: 8px 14px;
  border-top: 1px solid rgba(var(--v-theme-on-surface), 0.08);
}

.version-alert {
  margin: 14px;
}

.version-rows {
  display: flex;
  flex-direction: column;
}

.version-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 18px;
  flex-wrap: wrap;
}

.version-row + .version-row {
  border-top: 1px solid rgba(var(--v-theme-on-surface), 0.08);
}

.version-row__main {
  flex: 1 1 320px;
  min-width: 0;
}

.version-row__title {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  color: rgb(var(--v-theme-on-surface));
  font-size: 0.95rem;
  font-weight: 600;
  line-height: 1.4;
  overflow-wrap: anywhere;
}

.version-row__sub {
  margin-top: 2px;
  color: rgba(var(--v-theme-on-surface), 0.6);
  font-size: 0.8rem;
  line-height: 1.5;
  overflow-wrap: anywhere;
}

.version-row__meta {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 4px;
  flex-wrap: wrap;
}

.version-row__instances {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 6px;
  flex-wrap: wrap;
}

.version-row__actions {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: auto;
  flex-wrap: wrap;
}

.version-loading {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 24px 20px;
}

@media (max-width: 720px) {
  .version-row {
    align-items: flex-start;
  }

  .version-row__actions {
    width: 100%;
    margin-left: 0;
    justify-content: flex-start;
  }
}
</style>
