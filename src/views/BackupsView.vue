<script setup lang="ts">
import { computed, ref, watch } from "vue";
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

const { mdAndDown } = useDisplay();
const { snapshot, instances, backups, loading, error, refresh } =
  useLauncherStore();
const { isBusy, run } = useOperation();

const headers: DataTableHeader[] = [
  { title: "实例名称", key: "instance_name", sortable: false, minWidth: 180 },
  { title: "版本", key: "version", sortable: false, width: 120 },
  { title: "创建时间", key: "created_at", sortable: false, width: 170 },
  { title: "备注", key: "parse_error", sortable: false, minWidth: 160 },
  {
    title: "操作",
    key: "actions",
    sortable: false,
    align: "end",
    width: 240,
  },
];

const search = ref<string | null>("");
const activeInstanceId = ref("");

const instanceOptions = computed(() => {
  const names = new Map<string, string>();
  for (const backup of backups.value) {
    const id = backup.metadata.instance_id;
    if (id === "" || names.has(id)) {
      continue;
    }
    names.set(id, backup.metadata.instance_name || id);
  }
  return Array.from(names, ([value, title]) => ({ title, value }));
});

const instanceFilterItems = computed(() => [
  { title: "全部实例", value: "" },
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
    return keyword === "" || backup.filename.toLowerCase().includes(keyword);
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

async function handleCreate(instanceId: string): Promise<void> {
  if (createBusy.value) {
    return;
  }
  creatingInstanceId.value = instanceId;
  try {
    const created = await run(
      `backup:create:${instanceId}`,
      () => createBackup(instanceId),
      { success: "备份已创建" },
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
    ? "恢复将覆盖目标实例的当前数据。"
    : `恢复将覆盖实例“${name}”的当前数据。`;
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
      success: "备份已恢复",
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
    ? `确定删除备份“${backup.filename}”吗？删除后无法恢复。`
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
    { success: "备份已删除" },
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
    <PageHeader title="备份">
      <template #actions>
        <v-btn
          variant="text"
          prepend-icon="mdi-refresh"
          :loading="loading"
          @click="handleRefresh"
        >
          刷新
        </v-btn>
        <v-btn
          color="primary"
          variant="flat"
          prepend-icon="mdi-plus"
          :disabled="snapshot === null"
          @click="openCreateDialog"
        >
          创建备份
        </v-btn>
      </template>
    </PageHeader>

    <v-alert
      v-if="error"
      type="error"
      variant="tonal"
      density="comfortable"
      class="backups-alert"
    >
      <div class="backups-alert__body">
        <span>{{ error }}</span>
        <v-btn
          size="small"
          variant="text"
          :loading="loading"
          @click="handleRefresh"
        >
          重试
        </v-btn>
      </div>
    </v-alert>

    <div v-if="snapshot === null && !error" class="backups-state">
      <v-progress-circular
        indeterminate
        color="primary"
        size="32"
        aria-label="加载中"
      />
    </div>

    <template v-else-if="snapshot !== null && backups.length === 0">
      <div class="page-table-card">
        <EmptyState text="暂无备份" icon="mdi-archive-outline">
          <v-btn
            color="primary"
            variant="flat"
            size="small"
            prepend-icon="mdi-plus"
            @click="openCreateDialog"
          >
            创建备份
          </v-btn>
        </EmptyState>
      </div>
    </template>

    <template v-else-if="snapshot !== null">
      <div class="backups-filters">
        <v-select
          v-model="activeInstanceId"
          :items="instanceFilterItems"
          item-title="title"
          item-value="value"
          label="实例"
          variant="outlined"
          density="compact"
          hide-details
          class="backups-filters__select"
        />
        <v-text-field
          v-model="search"
          label="搜索文件名"
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
          loading-text="加载中"
          no-data-text="没有符合条件的备份"
        >
          <template #item.instance_name="{ item }">
            <div class="backup-name">
              <span class="backup-name__text">
                {{ item.metadata.instance_name || "—" }}
              </span>
              <v-chip
                v-if="item.metadata.auto_generated"
                size="x-small"
                variant="tonal"
                color="info"
              >
                自动创建
              </v-chip>
              <v-chip
                v-if="item.corrupted"
                size="x-small"
                variant="tonal"
                color="error"
              >
                已损坏
              </v-chip>
            </div>
          </template>

          <template #item.version="{ item }">
            {{ item.metadata.version || "—" }}
          </template>

          <template #item.created_at="{ item }">
            {{ formatDateTime(item.metadata.created_at) }}
          </template>

          <template #item.parse_error="{ item }">
            <span class="backup-note">{{ item.parse_error || "—" }}</span>
          </template>

          <template #item.actions="{ item }">
            <div class="backup-actions">
              <v-btn
                size="small"
                variant="text"
                prepend-icon="mdi-restore"
                :loading="isBusy(restoreKey(item.path))"
                :disabled="!canRestore(item.path) || isBusy(restoreKey(item.path))"
                @click="requestRestore(item.path)"
              >
                恢复
              </v-btn>
              <v-btn
                size="small"
                variant="text"
                color="error"
                prepend-icon="mdi-delete-outline"
                @click="requestDelete(item.path)"
              >
                删除
              </v-btn>
              <v-btn
                size="small"
                variant="text"
                icon="mdi-folder-open-outline"
                aria-label="打开所在文件夹"
                title="打开所在文件夹"
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
      title="恢复备份"
      :message="restoreMessage"
      confirm-label="恢复"
      color="warning"
      :loading="restoreBusy"
      @confirm="handleRestoreConfirmed"
    />

    <ConfirmDialog
      v-model="deleteConfirmOpen"
      title="删除备份"
      :message="deleteMessage"
      confirm-label="删除"
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
.backups-alert {
  margin-bottom: 16px;
}

.backups-alert__body {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.backups-state {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 220px;
}

.backups-filters {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}

.backups-filters__select {
  width: 220px;
}

.backups-filters__search {
  width: 280px;
}

.backup-name {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  min-width: 0;
}

.backup-name__text {
  overflow-wrap: anywhere;
}

.backup-note {
  color: rgba(var(--v-theme-on-surface), 0.72);
  font-size: 0.82rem;
  overflow-wrap: anywhere;
}

.backup-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 2px;
  flex-wrap: wrap;
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
