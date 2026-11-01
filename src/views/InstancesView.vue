<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { DataTableHeader } from "vuetify";
import { useDisplay } from "vuetify";
import { useRouter } from "vue-router";
import ConfirmDialog from "../components/common/ConfirmDialog.vue";
import EmptyState from "../components/common/EmptyState.vue";
import LockCheckDialog from "../components/common/LockCheckDialog.vue";
import PageHeader from "../components/common/PageHeader.vue";
import StatusChip from "../components/common/StatusChip.vue";
import TaskProgressDialog from "../components/common/TaskProgressDialog.vue";
import InstanceFormDialog from "../components/instances/InstanceFormDialog.vue";
import RepairDialog from "../components/instances/RepairDialog.vue";
import { useOperation } from "../composables/useOperation";
import { openExternalUrl } from "../services/app";
import { createBackup } from "../services/backups";
import { parseBackendError, type LockFailure } from "../services/errors";
import {
  checkLock,
  clearInstanceData,
  clearInstanceVenv,
  clearPycache,
  createInstance,
  deleteInstance,
  openInstanceCoreFolder,
  repairInstance,
  restartInstance,
  startInstance,
  stopInstance,
  updateInstance,
  type LockCheckInput,
} from "../services/instances";
import { useLauncherStore } from "../stores/launcher";
import { useProgressStore } from "../stores/progress";
import { useTabsStore } from "../stores/tabs";
import { showErrorToast, showToast } from "../stores/toast";
import type {
  InstanceStatus,
  RepairPreserveScope,
  UpdateInstanceRequest,
} from "../types/launcher";

const { mdAndDown } = useDisplay();
const router = useRouter();
const { snapshot, instances, versions, loading, error, refresh } =
  useLauncherStore();
const { isBusy, run } = useOperation();
const {
  deploys,
  activeDeployId,
  beginDeploy,
  dismissDeploy,
} = useProgressStore();
const { isHomeActive, openInstanceTab } = useTabsStore();

const headers: DataTableHeader[] = [
  { title: "名称", key: "name", sortable: false, minWidth: 160 },
  { title: "状态", key: "state", sortable: false, width: 160 },
  { title: "版本", key: "version", sortable: false, width: 120 },
  { title: "端口", key: "port", sortable: false, width: 90 },
  {
    title: "操作",
    key: "actions",
    sortable: false,
    align: "end",
    minWidth: 300,
  },
];

const portsInUse = computed(() =>
  instances.value.map((instance) => instance.configured_port),
);

const formOpen = ref(false);
const formInstance = ref<InstanceStatus | null>(null);

const formBusy = computed(() => {
  const target = formInstance.value;
  return target
    ? isBusy(`instance:${target.id}:edit`)
    : isBusy("instance:create");
});

const repairOpen = ref(false);
const repairTarget = ref<InstanceStatus | null>(null);

type ConfirmKind =
  | "delete"
  | "backup"
  | "clear-data"
  | "clear-venv"
  | "clear-pycache"
  | "upgrade";

interface ConfirmState {
  kind: ConfirmKind;
  instance: InstanceStatus;
  request?: UpdateInstanceRequest;
  version?: string;
}

const confirmState = ref<ConfirmState | null>(null);

const confirmOpen = computed({
  get: () => confirmState.value !== null,
  set: (value: boolean) => {
    if (!value) {
      confirmState.value = null;
    }
  },
});

const confirmTitle = computed(() => {
  switch (confirmState.value?.kind) {
    case "delete":
      return "删除实例";
    case "backup":
      return "创建备份";
    case "clear-data":
      return "清除数据";
    case "clear-venv":
      return "清除虚拟环境";
    case "clear-pycache":
      return "清除 Python 缓存";
    case "upgrade":
      return "变更版本";
    default:
      return "";
  }
});

const confirmMessage = computed(() => {
  const state = confirmState.value;
  if (!state) {
    return "";
  }
  const name = state.instance.name;
  switch (state.kind) {
    case "delete":
      return `确定删除实例“${name}”？实例数据将被移除，此操作不可恢复。`;
    case "backup":
      return `将为实例“${name}”创建备份。`;
    case "clear-data":
      return `将删除实例“${name}”的数据目录，此操作不可恢复。`;
    case "clear-venv":
      return `将删除实例“${name}”的虚拟环境，下次启动会重新安装依赖。`;
    case "clear-pycache":
      return `将删除实例“${name}”的 Python 字节码缓存。`;
    case "upgrade":
      return `将把实例“${name}”切换到 ${state.version ?? ""}，升级会重新部署并自动备份数据。`;
  }
});

const confirmLabel = computed(() => {
  switch (confirmState.value?.kind) {
    case "delete":
      return "删除";
    case "backup":
      return "创建";
    case "clear-data":
    case "clear-venv":
    case "clear-pycache":
      return "清除";
    case "upgrade":
      return "继续";
    default:
      return "确认";
  }
});

const confirmColor = computed(() => {
  switch (confirmState.value?.kind) {
    case "delete":
    case "clear-data":
    case "clear-venv":
    case "clear-pycache":
      return "error";
    case "upgrade":
      return "warning";
    default:
      return "primary";
  }
});

type LockableAction =
  | { kind: "delete"; instance: InstanceStatus }
  | { kind: "backup"; instance: InstanceStatus }
  | {
      kind: "upgrade";
      instance: InstanceStatus;
      request: UpdateInstanceRequest;
    };

const lockOpen = ref(false);
const lockFailure = ref<LockFailure | null>(null);
const lockAction = ref<LockableAction | null>(null);
const lockLoading = ref(false);

class HandledOperation extends Error {
  constructor() {
    super("handled");
    this.name = "HandledOperation";
  }
}

function startKey(instanceId: string): string {
  return `instance:${instanceId}:start`;
}

function stopKey(instanceId: string): string {
  return `instance:${instanceId}:stop`;
}

function restartKey(instanceId: string): string {
  return `instance:${instanceId}:restart`;
}

function upgradeKey(instanceId: string): string {
  return `instance:${instanceId}:upgrade`;
}

function backupKey(instanceId: string): string {
  return `instance:${instanceId}:backup`;
}

function deleteKey(instanceId: string): string {
  return `instance:${instanceId}:delete`;
}

function clearDataKey(instanceId: string): string {
  return `instance:${instanceId}:clear-data`;
}

function clearVenvKey(instanceId: string): string {
  return `instance:${instanceId}:clear-venv`;
}

function clearPycacheKey(instanceId: string): string {
  return `instance:${instanceId}:clear-pycache`;
}

function repairKey(instanceId: string): string {
  return `instance:${instanceId}:repair`;
}

function isInstanceBusy(instanceId: string): boolean {
  return [
    startKey(instanceId),
    stopKey(instanceId),
    restartKey(instanceId),
    upgradeKey(instanceId),
    backupKey(instanceId),
    deleteKey(instanceId),
    clearDataKey(instanceId),
    clearVenvKey(instanceId),
    clearPycacheKey(instanceId),
    repairKey(instanceId),
    `instance:${instanceId}:edit`,
  ].some((key) => isBusy(key));
}

function actionKey(action: LockableAction): string {
  switch (action.kind) {
    case "delete":
      return deleteKey(action.instance.id);
    case "backup":
      return backupKey(action.instance.id);
    case "upgrade":
      return upgradeKey(action.instance.id);
  }
}

function lockInput(action: LockableAction): LockCheckInput {
  switch (action.kind) {
    case "delete":
      return { target: "instance_data", instanceId: action.instance.id };
    case "backup":
      return { target: "backup_create", instanceId: action.instance.id };
    case "upgrade":
      return { target: "instance_upgrade", instanceId: action.instance.id };
  }
}

function actionSuccessMessage(action: LockableAction): string {
  switch (action.kind) {
    case "delete":
      return "实例已删除";
    case "backup":
      return "备份已创建";
    case "upgrade":
      return "实例已更新";
  }
}

function activeDeployProgress() {
  const id = activeDeployId.value;
  return id ? deploys.value[id] ?? null : null;
}

const activeDeploy = computed(activeDeployProgress);

const deployDialogOpen = computed(
  () => isHomeActive.value && activeDeployId.value !== null,
);

const normalizedHost = (host: string): string => {
  const value = host.trim();
  if (
    value === "" ||
    value === "0.0.0.0" ||
    value === "::" ||
    value === "[::]" ||
    value.toLowerCase() === "localhost"
  ) {
    return "127.0.0.1";
  }
  if (value.includes(":") && !(value.startsWith("[") && value.endsWith("]"))) {
    return `[${value}]`;
  }
  return value;
};

function instanceWebuiUrl(instance: InstanceStatus): string {
  const host = normalizedHost(instance.configured_host);
  const port =
    instance.state === "running" ? instance.port : instance.configured_port;
  return `http://${host}:${port}/`;
}

function dismissDeployIfActive(instanceId: string): void {
  if (activeDeployId.value === instanceId) {
    dismissDeploy();
  }
}

function finishDeployIfNeeded(instanceId: string, ok: boolean): void {
  if (!ok || !deploys.value[instanceId]) {
    dismissDeployIfActive(instanceId);
  }
}

async function executeLockableAction(
  action: LockableAction,
): Promise<void> {
  switch (action.kind) {
    case "delete":
      await deleteInstance(action.instance.id);
      break;
    case "backup":
      await createBackup(action.instance.id);
      break;
    case "upgrade":
      beginDeploy(action.instance.id);
      await updateInstance(action.instance.id, action.request);
      break;
  }
}

function openLockDialog(action: LockableAction, failure: LockFailure): void {
  lockAction.value = action;
  lockFailure.value = failure;
  confirmState.value = null;
  lockOpen.value = true;
}

function handleOperationError(
  action: LockableAction,
): (cause: unknown) => boolean {
  return (cause: unknown): boolean => {
    if (cause instanceof HandledOperation) {
      return true;
    }
    const parsed = parseBackendError(cause);
    if (parsed.lock) {
      openLockDialog(action, parsed.lock);
      return true;
    }
    return false;
  };
}

async function preflightLock(action: LockableAction): Promise<boolean> {
  try {
    await checkLock(lockInput(action));
    return true;
  } catch (cause) {
    const parsed = parseBackendError(cause);
    if (parsed.lock) {
      openLockDialog(action, parsed.lock);
      return false;
    }
    showToast(parsed.message, "error");
    return false;
  }
}

async function runLockable(action: LockableAction): Promise<boolean> {
  if (isInstanceBusy(action.instance.id)) {
    showToast("操作进行中，请稍候", "info");
    return false;
  }

  const ok = await run(
    actionKey(action),
    async () => {
      if (!(await preflightLock(action))) {
        throw new HandledOperation();
      }
      await executeLockableAction(action);
    },
    {
      success: actionSuccessMessage(action),
      onError: handleOperationError(action),
    },
  );

  if (action.kind === "upgrade") {
    finishDeployIfNeeded(action.instance.id, ok);
  }
  return ok;
}

function handleLockDialogUpdate(value: boolean): void {
  if (!value) {
    lockOpen.value = false;
    lockAction.value = null;
    lockFailure.value = null;
  }
}

async function continueLockAction(): Promise<void> {
  const action = lockAction.value;
  if (!action || lockLoading.value) {
    return;
  }

  lockAction.value = null;
  lockFailure.value = null;
  lockLoading.value = true;

  const ok = await run(actionKey(action), () => executeLockableAction(action), {
    success: actionSuccessMessage(action),
    onError: handleOperationError(action),
  });

  lockLoading.value = false;
  if (action.kind === "upgrade") {
    finishDeployIfNeeded(action.instance.id, ok);
  }

  if (!ok && lockFailure.value) {
    return;
  }

  lockOpen.value = false;
  lockAction.value = null;
  lockFailure.value = null;
}

function openCreate(): void {
  formInstance.value = null;
  formOpen.value = true;
}

function openEdit(instance: InstanceStatus): void {
  if (instance.state !== "stopped") {
    return;
  }
  formInstance.value = instance;
  formOpen.value = true;
}

function handleGoVersions(): void {
  formOpen.value = false;
  void router.push("/versions");
}

async function handleCreate(payload: {
  name: string;
  version: string;
  port: number;
}): Promise<void> {
  const ok = await run(
    "instance:create",
    () => createInstance(payload.name, payload.version, payload.port),
    { success: "实例已创建" },
  );
  if (ok) {
    formOpen.value = false;
  }
}

async function handleUpdate(
  instanceId: string,
  request: UpdateInstanceRequest,
): Promise<void> {
  const target = instances.value.find((item) => item.id === instanceId);
  if (!target) {
    showToast("实例不存在", "error");
    return;
  }

  const versionChanged =
    request.version !== undefined && request.version !== target.version;

  if (versionChanged) {
    formOpen.value = false;
    confirmState.value = {
      kind: "upgrade",
      instance: target,
      request,
      version: request.version,
    };
    return;
  }

  const ok = await run(
    `instance:${instanceId}:edit`,
    () => updateInstance(instanceId, request),
    { success: "实例已更新" },
  );
  if (ok) {
    formOpen.value = false;
  }
}

async function handleConfirm(): Promise<void> {
  const current = confirmState.value;
  if (!current) {
    return;
  }

  confirmState.value = null;

  switch (current.kind) {
    case "delete":
      await runLockable({ kind: "delete", instance: current.instance });
      break;
    case "backup":
      await runLockable({ kind: "backup", instance: current.instance });
      break;
    case "upgrade":
      if (current.request) {
        await runLockable({
          kind: "upgrade",
          instance: current.instance,
          request: current.request,
        });
      }
      break;
    case "clear-data":
      await run(
        clearDataKey(current.instance.id),
        () => clearInstanceData(current.instance.id),
        { success: "数据已清除" },
      );
      break;
    case "clear-venv":
      await run(
        clearVenvKey(current.instance.id),
        () => clearInstanceVenv(current.instance.id),
        { success: "虚拟环境已清除" },
      );
      break;
    case "clear-pycache":
      await run(
        clearPycacheKey(current.instance.id),
        () => clearPycache(current.instance.id),
        { success: "Python 缓存已清除" },
      );
      break;
  }
}

function askDelete(instance: InstanceStatus): void {
  confirmState.value = { kind: "delete", instance };
}

function askBackup(instance: InstanceStatus): void {
  confirmState.value = { kind: "backup", instance };
}

function askClear(
  instance: InstanceStatus,
  kind: "clear-data" | "clear-venv" | "clear-pycache",
): void {
  confirmState.value = { kind, instance };
}

function openRepair(instance: InstanceStatus): void {
  if (instance.state !== "stopped") {
    return;
  }
  repairTarget.value = instance;
  repairOpen.value = true;
}

async function handleRepair(scope: RepairPreserveScope): Promise<void> {
  const target = repairTarget.value;
  if (!target || isInstanceBusy(target.id)) {
    return;
  }

  repairOpen.value = false;
  beginDeploy(target.id);

  const ok = await run(
    repairKey(target.id),
    () => repairInstance(target.id, scope),
    { success: "修复完成" },
  );

  finishDeployIfNeeded(target.id, ok);
}

async function handleStart(instance: InstanceStatus): Promise<void> {
  if (isInstanceBusy(instance.id)) {
    return;
  }
  beginDeploy(instance.id);
  const ok = await run(
    startKey(instance.id),
    () => startInstance(instance.id),
    { success: "实例已启动" },
  );
  finishDeployIfNeeded(instance.id, ok);
}

async function handleStop(instance: InstanceStatus): Promise<void> {
  await run(stopKey(instance.id), () => stopInstance(instance.id), {
    success: "实例已停止",
  });
}

async function handleRestart(instance: InstanceStatus): Promise<void> {
  if (isInstanceBusy(instance.id)) {
    return;
  }
  beginDeploy(instance.id);
  const ok = await run(
    restartKey(instance.id),
    () => restartInstance(instance.id),
    { success: "实例已重启" },
  );
  finishDeployIfNeeded(instance.id, ok);
}

async function handleOpenCoreFolder(instance: InstanceStatus): Promise<void> {
  try {
    await openInstanceCoreFolder(instance.id);
  } catch (cause) {
    showErrorToast(cause);
  }
}

async function handleOpenInBrowser(instance: InstanceStatus): Promise<void> {
  try {
    await openExternalUrl(instanceWebuiUrl(instance));
  } catch (cause) {
    showErrorToast(cause);
  }
}

function openLogs(instance: InstanceStatus): void {
  void router.push({ path: "/logs", query: { source: instance.id } });
}

function openWebui(instance: InstanceStatus): void {
  openInstanceTab(instance.id);
}

watch(activeDeploy, (progress) => {
  if (progress && (progress.step === "done" || progress.step === "error")) {
    dismissDeploy();
  }
});
</script>

<template>
  <div class="page-shell">
    <PageHeader title="实例">
      <template #actions>
        <v-btn
          variant="text"
          prepend-icon="mdi-refresh"
          :loading="loading"
          @click="refresh()"
        >
          刷新
        </v-btn>
        <v-btn
          color="primary"
          variant="flat"
          prepend-icon="mdi-plus"
          :disabled="snapshot === null"
          @click="openCreate"
        >
          创建实例
        </v-btn>
      </template>
    </PageHeader>

    <v-alert
      v-if="error"
      type="error"
      variant="tonal"
      density="comfortable"
      class="instances-alert"
    >
      <div class="instances-alert__body">
        <span>{{ error }}</span>
        <v-btn
          size="small"
          variant="text"
          :loading="loading"
          @click="refresh()"
        >
          重试
        </v-btn>
      </div>
    </v-alert>

    <div v-if="snapshot === null && !error" class="instances-state">
      <v-progress-circular
        indeterminate
        color="primary"
        size="32"
        aria-label="加载中"
      />
    </div>

    <div
      v-else-if="snapshot !== null && instances.length === 0"
      class="page-table-card"
    >
      <EmptyState text="暂无实例" icon="mdi-server-plus-outline">
        <v-btn
          color="primary"
          variant="flat"
          size="small"
          prepend-icon="mdi-plus"
          @click="openCreate"
        >
          创建实例
        </v-btn>
      </EmptyState>
    </div>

    <div v-else-if="snapshot !== null" class="page-table-card">
      <v-data-table
        :headers="headers"
        :items="instances"
        item-value="id"
        :mobile="mdAndDown"
        :loading="loading"
        hide-default-footer
        :items-per-page="-1"
        disable-sort
        hover
        no-data-text="暂无实例"
      >
        <template #item.state="{ item }">
          <div class="instance-status">
            <StatusChip :state="item.state" />
            <v-tooltip
              v-if="item.pid_tracker_not_available"
              text="WebUI 未启用，无法跟踪进程状态"
              location="top"
            >
              <template #activator="{ props }">
                <v-icon
                  v-bind="props"
                  icon="mdi-alert-outline"
                  size="16"
                  color="warning"
                  tabindex="0"
                  aria-label="进程状态可能不准确"
                />
              </template>
            </v-tooltip>
          </div>
        </template>

        <template #item.port="{ item }">
          {{ item.state === "running" ? item.port : item.configured_port }}
        </template>

        <template #item.actions="{ item }">
          <div class="instance-actions">
            <template v-if="item.state === 'stopped'">
              <v-btn
                size="small"
                color="success"
                variant="tonal"
                :disabled="isInstanceBusy(item.id)"
                :loading="isBusy(startKey(item.id))"
                @click="handleStart(item)"
              >
                启动
              </v-btn>
            </template>
            <template v-else-if="item.state === 'running'">
              <v-btn
                size="small"
                color="warning"
                variant="text"
                :disabled="isInstanceBusy(item.id)"
                :loading="isBusy(stopKey(item.id))"
                @click="handleStop(item)"
              >
                停止
              </v-btn>
              <v-btn
                size="small"
                variant="text"
                :disabled="isInstanceBusy(item.id)"
                :loading="isBusy(restartKey(item.id))"
                @click="handleRestart(item)"
              >
                重启
              </v-btn>
            </template>
            <v-btn v-else size="small" variant="tonal" disabled loading>
              {{ item.state === "starting" ? "启动中" : "停止中" }}
            </v-btn>

            <v-btn
              size="small"
              variant="text"
              :disabled="!item.dashboard_enabled"
              @click="openWebui(item)"
            >
              打开
            </v-btn>

            <v-menu location="bottom end">
              <template #activator="{ props }">
                <v-btn
                  v-bind="props"
                  icon="mdi-dots-horizontal"
                  variant="text"
                  size="small"
                  aria-label="更多操作"
                />
              </template>
              <v-list density="compact" class="instance-menu">
                <v-list-item
                  prepend-icon="mdi-folder-open-outline"
                  title="打开核心目录"
                  @click="handleOpenCoreFolder(item)"
                />
                <v-list-item
                  prepend-icon="mdi-pencil-outline"
                  title="编辑"
                  :disabled="item.state !== 'stopped' || isInstanceBusy(item.id)"
                  @click="openEdit(item)"
                />
                <v-list-item
                  prepend-icon="mdi-archive-arrow-down-outline"
                  title="创建备份"
                  :disabled="item.state !== 'stopped' || isInstanceBusy(item.id)"
                  @click="askBackup(item)"
                />
                <v-list-item
                  prepend-icon="mdi-lifebuoy"
                  title="修复实例"
                  :disabled="item.state !== 'stopped' || isInstanceBusy(item.id)"
                  @click="openRepair(item)"
                />
                <v-list-item
                  prepend-icon="mdi-text-box-outline"
                  title="查看日志"
                  @click="openLogs(item)"
                />
                <v-list-item
                  prepend-icon="mdi-open-in-new"
                  title="在浏览器中打开"
                  :disabled="!item.dashboard_enabled"
                  @click="handleOpenInBrowser(item)"
                />
                <v-divider />
                <v-list-item
                  prepend-icon="mdi-database-remove-outline"
                  title="清除数据"
                  :disabled="item.state !== 'stopped' || isInstanceBusy(item.id)"
                  @click="askClear(item, 'clear-data')"
                />
                <v-list-item
                  prepend-icon="mdi-delete-sweep-outline"
                  title="清除虚拟环境"
                  :disabled="item.state !== 'stopped' || isInstanceBusy(item.id)"
                  @click="askClear(item, 'clear-venv')"
                />
                <v-list-item
                  prepend-icon="mdi-broom"
                  title="清除 Python 缓存"
                  :disabled="item.state !== 'stopped' || isInstanceBusy(item.id)"
                  @click="askClear(item, 'clear-pycache')"
                />
                <v-divider />
                <v-list-item
                  prepend-icon="mdi-delete-outline"
                  title="删除实例"
                  base-color="error"
                  :disabled="item.state !== 'stopped' || isInstanceBusy(item.id)"
                  @click="askDelete(item)"
                />
              </v-list>
            </v-menu>
          </div>
        </template>
      </v-data-table>
    </div>

    <InstanceFormDialog
      v-model="formOpen"
      :instance="formInstance"
      :versions="versions"
      :used-ports="portsInUse"
      :loading="formBusy"
      @create="handleCreate"
      @update="handleUpdate"
      @go-versions="handleGoVersions"
    />

    <RepairDialog v-model="repairOpen" @submit="handleRepair" />

    <ConfirmDialog
      v-model="confirmOpen"
      :title="confirmTitle"
      :message="confirmMessage"
      :confirm-label="confirmLabel"
      :color="confirmColor"
      @confirm="handleConfirm"
    />

    <LockCheckDialog
      :model-value="lockOpen"
      :failure="lockFailure"
      :loading="lockLoading"
      @update:model-value="handleLockDialogUpdate"
      @continue="continueLockAction"
    />

    <TaskProgressDialog
      :model-value="deployDialogOpen"
      title="实例部署"
      :message="activeDeploy?.message ?? ''"
      :progress="activeDeploy?.progress ?? null"
    />
  </div>
</template>

<style scoped>
.instances-alert {
  margin-bottom: 16px;
}

.instances-alert__body {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.instances-state {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 180px;
}

.instance-status {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.instance-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
  flex-wrap: wrap;
}

.instance-menu {
  min-width: 208px;
}

@media (max-width: 959px) {
  .instance-actions {
    justify-content: flex-start;
  }
}
</style>
