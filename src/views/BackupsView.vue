<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { DataTableHeader } from "vuetify";
import { useDisplay } from "vuetify";
import CreateBackupDialog from "../components/backups/CreateBackupDialog.vue";
import ConfirmDialog from "../components/common/ConfirmDialog.vue";
import EmptyState from "../components/common/EmptyState.vue";
import LockCheckDialog from "../components/common/LockCheckDialog.vue";
import PageHeader from "../components/common/PageHeader.vue";
import { useOperation } from "../composables/useOperation";
import { openFolder } from "../services/app";
import { createBackup, deleteBackup, restoreBackup } from "../services/backups";
import { parseBackendError, type LockFailure } from "../services/errors";
import { checkLock } from "../services/instances";
import { useLauncherStore } from "../stores/launcher";
import { showToast } from "../stores/toast";
import type { BackupInfo } from "../types/launcher";
import { formatDateTime } from "../utils/format";
import { dirname } from "../utils/path";

const { t } = useI18n();
const { mdAndDown } = useDisplay();
const { snapshot, instances, backups, loading, error, refresh } =
  useLauncherStore();
const { isBusy, run } = useOperation();

const headers = computed<DataTableHeader[]>(() => [
  {
    title: t("backups.headers.instanceName"),
    key: "instance_name",
    sortable: false,
    minWidth: 180,
  },
  {
    title: t("backups.headers.version"),
    key: "version",
    sortable: false,
    width: 120,
  },
  {
    title: t("backups.headers.createdAt"),
    key: "created_at",
    sortable: false,
    width: 170,
  },
  {
    title: t("backups.headers.note"),
    key: "note",
    sortable: false,
    minWidth: 160,
  },
  {
    title: t("backups.headers.actions"),
    key: "actions",
    sortable: false,
    align: "end",
    width: 240,
  },
]);

const search = ref<string | null>("");
const activeInstanceId = ref("");

const instanceOptions = computed(() => {
  const names = new Map<string, string>();
  for (const backup of backups.value) {
    const id = backup.metadata.instance_id;
    if (id === "" || names.has(id)) {
      continue;
    }
    names.set(
      id,
      backup.corrupted
        ? t("backups.corruptedName")
        : backup.metadata.instance_name || id,
    );
  }
  return Array.from(names, ([value, title]) => ({ title, value }));
});

const instanceFilterItems = computed(() => [
  { title: t("backups.allInstances"), value: "" },
  ...instanceOptions.value,
]);

const filteredBackups = computed(() => {
  const keyword = (search.value ?? "").trim().toLowerCase();
  return backups.value.filter((backup) => {
    if (
      activeInstanceId.value !== "" &&
      backup.metadata.instance_id !== activeInstanceId.value
    ) {
      return false;
    }
    return keyword === "" ||
      backup.filename.toLowerCase().includes(keyword) ||
      backup.metadata.note.toLowerCase().includes(keyword);
  });
});

watch(instanceOptions, (options) => {
  if (
    activeInstanceId.value !== "" &&
    !options.some((option) => option.value === activeInstanceId.value)
  ) {
    activeInstanceId.value = "";
  }
});

function handleRefresh(): void {
  void refresh();
}

function findBackup(path: string): BackupInfo | undefined {
  return backups.value.find((candidate) => candidate.path === path);
}

function restoreKey(path: string): string {
  return `backup:restore:${path}`;
}

function canRestore(path: string): boolean {
  const backup = findBackup(path);
  if (!backup || backup.corrupted) {
    return false;
  }
  const instance = instances.value.find(
    (candidate) => candidate.id === backup.metadata.instance_id,
  );
  return instance !== undefined && instance.state === "stopped";
}

const createOpen = ref(false);
const creatingInstanceId = ref<string | null>(null);

const createBusy = computed(() =>
  creatingInstanceId.value === null
    ? false
    : isBusy(`backup:create:${creatingInstanceId.value}`),
);

function openCreateDialog(): void {
  createOpen.value = true;
}

async function handleCreate(instanceId: string, note: string): Promise<void> {
  if (createBusy.value) {
    return;
  }
  creatingInstanceId.value = instanceId;
  try {
    const created = await run(
      `backup:create:${instanceId}`,
      () => createBackup(instanceId, note),
      { success: t("backups.created") },
    );
    if (created) {
      createOpen.value = false;
    }
  } finally {
    creatingInstanceId.value = null;
  }
}

const restoreTarget = ref<BackupInfo | null>(null);
const restoreConfirmOpen = ref(false);
const lockOpen = ref(false);
const lockFailure = ref<LockFailure | null>(null);
const lockTarget = ref<BackupInfo | null>(null);

const restoreBusy = computed(() =>
  restoreTarget.value === null
    ? false
    : isBusy(restoreKey(restoreTarget.value.path)),
);

const lockBusy = computed(() =>
  lockTarget.value === null ? false : isBusy(restoreKey(lockTarget.value.path)),
);

const restoreMessage = computed(() => {
  const backup = restoreTarget.value;
  if (!backup) {
    return "";
  }
  const instance = instances.value.find(
    (candidate) => candidate.id === backup.metadata.instance_id,
  );
  const name = (instance?.name || backup.metadata.instance_name).trim();
  return name === ""
    ? t("backups.restoreMessageGeneric")
    : t("backups.restoreMessage", { name });
});

function requestRestore(path: string): void {
  if (isBusy(restoreKey(path))) {
    return;
  }
  const backup = findBackup(path);
  if (!backup || !canRestore(path)) {
    return;
  }
  restoreTarget.value = backup;
  restoreConfirmOpen.value = true;
}

async function runRestore(
  backup: BackupInfo,
  checkTargetLock: boolean,
): Promise<boolean> {
  return run(
    restoreKey(backup.path),
    async () => {
      if (checkTargetLock) {
        await checkLock({ target: "backup_restore", backupPath: backup.path });
      }
      await restoreBackup(backup.path);
    },
    {
      success: t("backups.restored"),
      onError: (cause) => {
        const parsed = parseBackendError(cause);
        if (!parsed.lock) {
          return false;
        }
        restoreConfirmOpen.value = false;
        restoreTarget.value = null;
        lockFailure.value = parsed.lock;
        lockTarget.value = backup;
        lockOpen.value = true;
        return true;
      },
    },
  );
}

async function handleRestoreConfirmed(): Promise<void> {
  const backup = restoreTarget.value;
  if (!backup) {
    return;
  }
  const restored = await runRestore(backup, true);
  if (restored) {
    restoreConfirmOpen.value = false;
    restoreTarget.value = null;
  }
}

async function handleLockContinue(): Promise<void> {
  const backup = lockTarget.value;
  const canContinue = lockFailure.value?.canContinue === true;
  lockOpen.value = false;
  lockFailure.value = null;
  lockTarget.value = null;
  if (!backup || !canContinue) {
    return;
  }
  await runRestore(backup, false);
}

const deleteTarget = ref<BackupInfo | null>(null);
const deleteConfirmOpen = ref(false);

const deleteBusy = computed(() =>
  deleteTarget.value === null
    ? false
    : isBusy(`backup:delete:${deleteTarget.value.path}`),
);

const deleteMessage = computed(() => {
  const backup = deleteTarget.value;
  return backup
    ? t("backups.deleteMessage", { filename: backup.filename })
    : "";
});

function requestDelete(path: string): void {
  const backup = findBackup(path);
  if (!backup) {
    return;
  }
  deleteTarget.value = backup;
  deleteConfirmOpen.value = true;
}

async function handleDeleteConfirmed(): Promise<void> {
  const backup = deleteTarget.value;
  if (!backup) {
    return;
  }
  const deleted = await run(
    `backup:delete:${backup.path}`,
    () => deleteBackup(backup.path),
    { success: t("backups.deleted") },
  );
  if (deleted) {
    deleteConfirmOpen.value = false;
    deleteTarget.value = null;
  }
}

async function handleOpenFolder(path: string): Promise<void> {
  try {
    await openFolder(dirname(path));
  } catch (cause) {
    showToast(parseBackendError(cause).message, "error");
  }
}
</script>

<template>
  <div class="page-shell backups-page">
    <PageHeader :title="t('backups.title')">
      <template #actions>
        <v-btn
          variant="text"
          prepend-icon="mdi-refresh"
          :loading="loading"
          @click="handleRefresh"
        >
          {{ t("common.actions.refresh") }}
        </v-btn>
        <v-btn
          color="primary"
          variant="flat"
          prepend-icon="mdi-plus"
          :disabled="snapshot === null"
          @click="openCreateDialog"
        >
          {{ t("backups.createBackup") }}
        </v-btn>
      </template>
    </PageHeader>

    <v-alert
      v-if="error"
      type="error"
      variant="tonal"
      density="comfortable"
      class="backups-alert page-alert"
    >
      <div class="backups-alert__body page-row--between">
        <span>{{ error }}</span>
        <v-btn
          size="small"
          variant="text"
          :loading="loading"
          @click="handleRefresh"
        >
          {{ t("common.actions.retry") }}
        </v-btn>
      </div>
    </v-alert>

    <div v-if="snapshot === null && !error" class="backups-state page-center">
      <v-progress-circular
        indeterminate
        color="primary"
        size="32"
        :aria-label="t('backups.loading')"
      />
    </div>

    <template v-else-if="snapshot !== null && backups.length === 0">
      <div class="page-table-card">
        <EmptyState :text="t('backups.empty')" icon="mdi-archive-outline">
          <v-btn
            color="primary"
            variant="flat"
            size="small"
            prepend-icon="mdi-plus"
            @click="openCreateDialog"
          >
            {{ t("backups.createBackup") }}
          </v-btn>
        </EmptyState>
      </div>
    </template>

    <template v-else-if="snapshot !== null">
      <div class="backups-filters page-row">
        <v-select
          v-model="activeInstanceId"
          :items="instanceFilterItems"
          item-title="title"
          item-value="value"
          :label="t('backups.instanceFilter')"
          variant="outlined"
          density="compact"
          hide-details
          class="backups-filters__select"
        />
        <v-text-field
          v-model="search"
          :label="t('backups.searchPlaceholder')"
          variant="outlined"
          density="compact"
          hide-details
          clearable
          prepend-inner-icon="mdi-magnify"
          class="backups-filters__search"
        />
      </div>

      <div class="page-table-card">
        <v-data-table
          :headers="headers"
          :items="filteredBackups"
          item-value="path"
          :mobile="mdAndDown"
          :loading="loading"
          hide-default-footer
          :items-per-page="-1"
          disable-sort
          hover
          :loading-text="t('backups.loading')"
          :no-data-text="t('backups.noMatches')"
        >
          <template #item.instance_name="{ item }">
            <div class="backup-name page-row">
              <span class="backup-name__text page-anywhere">
                {{
                  item.corrupted
                    ? t("backups.corruptedName")
                    : item.metadata.instance_name || "—"
                }}
              </span>
              <v-chip
                v-if="item.metadata.auto_generated"
                size="x-small"
                variant="tonal"
                color="info"
              >
                {{ t("backups.autoGenerated") }}
              </v-chip>
              <v-chip
                v-if="item.corrupted"
                size="x-small"
                variant="tonal"
                color="error"
              >
                {{ t("backups.corrupted") }}
              </v-chip>
            </div>
          </template>

          <template #item.version="{ item }">
            {{ item.metadata.version || "—" }}
          </template>

          <template #item.created_at="{ item }">
            {{ formatDateTime(item.metadata.created_at) }}
          </template>

          <template #item.note="{ item }">
            <span class="backup-note page-anywhere">
              {{ item.corrupted ? item.parse_error : item.metadata.note || "—" }}
            </span>
          </template>

          <template #item.actions="{ item }">
            <div class="backup-actions page-row">
              <v-btn
                size="small"
                variant="text"
                prepend-icon="mdi-restore"
                :loading="isBusy(restoreKey(item.path))"
                :disabled="!canRestore(item.path) || isBusy(restoreKey(item.path))"
                @click="requestRestore(item.path)"
              >
                {{ t("backups.restore") }}
              </v-btn>
              <v-btn
                size="small"
                variant="text"
                color="error"
                prepend-icon="mdi-delete-outline"
                @click="requestDelete(item.path)"
              >
                {{ t("common.actions.delete") }}
              </v-btn>
              <v-btn
                size="small"
                variant="text"
                icon="mdi-folder-open-outline"
                :aria-label="t('backups.openFolder')"
                :title="t('backups.openFolder')"
                @click="handleOpenFolder(item.path)"
              />
            </div>
          </template>
        </v-data-table>
      </div>
    </template>

    <CreateBackupDialog
      v-model="createOpen"
      :instances="instances"
      :loading="createBusy"
      @submit="handleCreate"
    />

    <ConfirmDialog
      v-model="restoreConfirmOpen"
      :title="t('backups.restoreTitle')"
      :message="restoreMessage"
      :confirm-label="t('backups.restore')"
      color="warning"
      :loading="restoreBusy"
      @confirm="handleRestoreConfirmed"
    />

    <ConfirmDialog
      v-model="deleteConfirmOpen"
      :title="t('backups.deleteTitle')"
      :message="deleteMessage"
      :confirm-label="t('common.actions.delete')"
      color="error"
      :loading="deleteBusy"
      @confirm="handleDeleteConfirmed"
    />

    <LockCheckDialog
      v-model="lockOpen"
      :failure="lockFailure"
      :loading="lockBusy"
      @continue="handleLockContinue"
    />
  </div>
</template>

<style scoped>
.backups-alert__body {
  gap: 12px;
}

.backups-state {
  min-height: 220px;
}

.backups-filters {
  gap: 12px;
  margin-bottom: 12px;
}

.backups-filters__select {
  width: 220px;
}

.backups-filters__search {
  width: 280px;
}

.backup-name {
  gap: 6px;
  min-width: 0;
}

.backup-note {
  color: rgba(var(--v-theme-on-surface), 0.72);
  font-size: 0.82rem;
}

.backup-actions {
  justify-content: flex-end;
  gap: 2px;
}

@media (max-width: 599px) {
  .backups-filters__select,
  .backups-filters__search {
    width: 100%;
  }

  .backup-actions {
    justify-content: flex-start;
  }
}
</style>
