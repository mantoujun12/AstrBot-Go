<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import type { DataTableHeader } from "vuetify";
import { useDisplay } from "vuetify";
import { useRouter } from "vue-router";
import { useI18n } from "vue-i18n";
import BackupNoteField from "../components/backups/BackupNoteField.vue";
import ConfirmDialog from "../components/common/ConfirmDialog.vue";
import EmptyState from "../components/common/EmptyState.vue";
import LockCheckDialog from "../components/common/LockCheckDialog.vue";
import PageHeader from "../components/common/PageHeader.vue";
import TaskProgressDialog from "../components/common/TaskProgressDialog.vue";
import InstanceFormDialog from "../components/instances/InstanceFormDialog.vue";
import InstanceStatusCell from "../components/instances/InstanceStatusCell.vue";
import StatusChip from "../components/common/StatusChip.vue";
import HealthCheckDialog from "../components/instances/HealthCheckDialog.vue";
import RepairDialog from "../components/instances/RepairDialog.vue";
import ResourceMonitorDialog from "../components/instances/ResourceMonitorDialog.vue";
import { useOperation } from "../composables/useOperation";
import { translateDynamicKey } from "../i18n/dynamic";
import { openExternalUrl } from "../services/app";
import { createBackup } from "../services/backups";
import {
  parseBackendError,
  toUserMessage,
  type LockFailure,
} from "../services/errors";
import {
  checkLock,
  clearInstanceData,
  clearInstanceVenv,
  clearPycache,
  checkInstanceHealth,
  createInstance,
  deleteInstance,
  discoverInstances,
  openInstanceCoreFolder,
  repairInstance,
  restartInstance,
  startInstance,
  stopInstance,
  updateInstance,
  type LockCheckInput,
} from "../services/instances";
import { checkInstanceUpdates } from "../services/updates";
import { installVersion } from "../services/versions";
import { useLauncherStore } from "../stores/launcher";
import { useProgressStore } from "../stores/progress";
import { useTabsStore } from "../stores/tabs";
import { showErrorToast, showToast } from "../stores/toast";
import type {
  DiscoveredInstance,
  DiscoveredInstancesSnapshot,
  GitHubRelease,
  InstanceStatus,
  InstanceHealthReport,
  InstanceUpdateCheckResult,
  InstanceUpdateInfo,
  RepairPreserveScope,
  UpdateInstanceRequest,
} from "../types/launcher";
import {
  BACKUP_NOTE_MAX_LENGTH,
  countBackupNoteChars,
} from "../utils/backupNote";

const { mdAndDown } = useDisplay();
const router = useRouter();
const { snapshot, instances, versions, loading, error, refresh } =
  useLauncherStore();
const { isBusy, run } = useOperation();
const {
  deploys,
  downloads,
  activeDeployId,
  beginDeploy,
  dismissDeploy,
} = useProgressStore();
const { isHomeActive, openInstanceTab } = useTabsStore();
const { t } = useI18n();

const updateCheck = ref<InstanceUpdateCheckResult | null>(null);
const updatesLoading = ref(false);
const updatesError = ref<string | null>(null);
const upgradeDownloadId = ref<string | null>(null);
const batchUpgradeRunning = ref(false);
const upgradeAllOpen = ref(false);
const upgradeAllTargets = ref<InstanceStatus[]>([]);
const upgradeAllRelease = ref<GitHubRelease | null>(null);
let updatesRequest = 0;

const headers = computed<DataTableHeader[]>(() => [
  {
    title: t("instances.list.columns.name"),
    key: "name",
    sortable: false,
    minWidth: 200,
  },
  {
    title: t("instances.list.columns.state"),
    key: "state",
    sortable: false,
    width: 220,
  },
  {
    title: t("instances.list.columns.actions"),
    key: "actions",
    sortable: false,
    align: "end",
    minWidth: 240,
  },
]);

const portsInUse = computed(() =>
  instances.value.map((instance) => instance.configured_port),
);

const updatesById = computed(() => {
  const byId = new Map<string, InstanceUpdateInfo>();
  for (const update of updateCheck.value?.instances ?? []) {
    byId.set(update.instance_id, update);
  }
  return byId;
});

/* External instances */

type ExternalInstanceRow = DiscoveredInstance & { external: true };

const externalInstances = computed<ExternalInstanceRow[]>(() => {
  if (discovered.value === null) {
    return [];
  }
  return discovered.value.instances
    .filter((instance) => instance.source !== "launcher")
    .map((instance) => ({ ...instance, external: true }));
});

type InstanceRow =
  | (InstanceStatus & { external?: false })
  | ExternalInstanceRow;

const instanceRows = computed<InstanceRow[]>(() => [
  ...instances.value,
  ...externalInstances.value,
]);

function sourceTitleOf(sourceId: string): string {
  const source = discovered.value?.sources.find(
    (candidate) => candidate.id === sourceId,
  );
  return source === undefined ? sourceId : t(source.title_key);
}

const discovered = ref<DiscoveredInstancesSnapshot | null>(null);

async function loadDiscovery(): Promise<void> {
  try {
    discovered.value = await discoverInstances();
  } catch (cause) {
    showErrorToast(cause);
  }
}

async function openExternalWebui(instance: DiscoveredInstance): Promise<void> {
  if (instance.webui_url === null) {
    return;
  }
  try {
    await openExternalUrl(instance.webui_url);
  } catch (cause) {
    showErrorToast(cause);
  }
}

onMounted(() => {
  void loadDiscovery();
});

function updateInfoFor(instanceId: string): InstanceUpdateInfo | null {
  return updatesById.value.get(instanceId) ?? null;
}

const latestRelease = computed(
  () => updateCheck.value?.latest_release ?? null,
);

const updateAvailableInstances = computed(() =>
  instances.value.filter(
    (instance) => updateInfoFor(instance.id)?.state === "available",
  ),
);

const batchUpgradeTargets = computed(() =>
  updateAvailableInstances.value.filter(
    (instance) => instance.state === "stopped" && !isInstanceBusy(instance.id),
  ),
);

const skippedUpgradeCount = computed(
  () => updateAvailableInstances.value.length - batchUpgradeTargets.value.length,
);

async function loadUpdateCheck(forceRefresh = false): Promise<void> {
  const request = ++updatesRequest;
  updatesLoading.value = true;
  updatesError.value = null;

  try {
    const result = await checkInstanceUpdates(forceRefresh);
    if (request === updatesRequest) {
      updateCheck.value = result;
    }
  } catch (cause) {
    if (request === updatesRequest) {
      updateCheck.value = null;
      updatesError.value = toUserMessage(cause);
    }
  } finally {
    if (request === updatesRequest) {
      updatesLoading.value = false;
    }
  }
}

const updateFingerprint = computed(() => {
  const enabled = snapshot.value?.config.check_instance_update ? "1" : "0";
  const instanceState = instances.value
    .map(
      (instance) =>
        `${instance.id}:${instance.version}:${instance.check_update_enabled ? "1" : "0"}`,
    )
    .join("|");
  return `${enabled}|${instanceState}`;
});

watch(
  updateFingerprint,
  () => {
    if (snapshot.value !== null) {
      void loadUpdateCheck();
    }
  },
  { immediate: true },
);

function handleRefresh(): void {
  void refresh();
  void loadDiscovery();
  void loadUpdateCheck(true);
}

function retryUpdateCheck(): void {
  void loadUpdateCheck(true);
}

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
const resourceOpen = ref(false);
const resourceTarget = ref<InstanceStatus | null>(null);
const healthOpen = ref(false);
const healthTarget = ref<InstanceStatus | null>(null);
const healthReport = ref<InstanceHealthReport | null>(null);
const healthLoading = ref(false);
const healthError = ref<string | null>(null);
let healthRequest = 0;

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
  release?: GitHubRelease;
}

const confirmState = ref<ConfirmState | null>(null);
const confirmNote = ref("");

const confirmOpen = computed({
  get: () => confirmState.value !== null,
  set: (value: boolean) => {
    if (!value) {
      confirmState.value = null;
      confirmNote.value = "";
    }
  },
});

const confirmNoteTooLong = computed(
  () => countBackupNoteChars(confirmNote.value) > BACKUP_NOTE_MAX_LENGTH,
);

const confirmTitle = computed(() => {
  switch (confirmState.value?.kind) {
    case "delete":
      return t("instances.confirm.titles.delete");
    case "backup":
      return t("instances.confirm.titles.backup");
    case "clear-data":
      return t("instances.confirm.titles.clearData");
    case "clear-venv":
      return t("instances.confirm.titles.clearVenv");
    case "clear-pycache":
      return t("instances.confirm.titles.clearPycache");
    case "upgrade":
      return t("instances.confirm.titles.upgrade");
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
      return t("instances.confirm.messages.delete", { name });
    case "backup":
      return t("instances.confirm.messages.backup", { name });
    case "clear-data":
      return t("instances.confirm.messages.clearData", { name });
    case "clear-venv":
      return t("instances.confirm.messages.clearVenv", { name });
    case "clear-pycache":
      return t("instances.confirm.messages.clearPycache", { name });
    case "upgrade":
      return t("instances.confirm.messages.upgrade", {
        name,
        version: state.version ?? "",
      });
  }
});

const confirmLabel = computed(() => {
  switch (confirmState.value?.kind) {
    case "delete":
      return t("common.actions.delete");
    case "backup":
      return t("common.actions.create");
    case "clear-data":
    case "clear-venv":
    case "clear-pycache":
      return t("instances.confirm.labels.clear");
    case "upgrade":
      return t("instances.confirm.labels.continue");
    default:
      return t("common.actions.confirm");
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
  | { kind: "backup"; instance: InstanceStatus; note: string }
  | {
      kind: "upgrade";
      instance: InstanceStatus;
      request: UpdateInstanceRequest;
      release?: GitHubRelease;
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

function isVersionInstalled(version: string): boolean {
  return versions.value.some((installed) => installed.version === version);
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

function isInstanceInteractionDisabled(instanceId: string): boolean {
  return batchUpgradeRunning.value || isInstanceBusy(instanceId);
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
      return t("instances.toasts.deleted");
    case "backup":
      return t("instances.toasts.backupCreated");
    case "upgrade":
      return t("instances.toasts.upgraded", {
        version: action.request.version ?? "",
      });
  }
}

function activeDeployProgress() {
  const id = activeDeployId.value;
  return id ? deploys.value[id] ?? null : null;
}

const activeDeploy = computed(activeDeployProgress);

const activeUpgradeDownload = computed(() => {
  const id = upgradeDownloadId.value;
  return id ? downloads.value[id] ?? null : null;
});

const activeTaskMessage = computed(() => {
  const deploy = activeDeploy.value;
  if (deploy) {
    return translateDynamicKey(deploy.message_key, deploy.message);
  }
  const download = activeUpgradeDownload.value;
  return download
    ? translateDynamicKey(
        download.message_key,
        download.message || download.step,
      )
    : "";
});

const activeTaskProgress = computed(
  () => activeDeploy.value?.progress ?? activeUpgradeDownload.value?.progress ?? null,
);

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

function instanceReadyMessage(instance: InstanceStatus): string {
  return t("instances.toasts.started", { name: instance.name });
}

function dismissDeployIfActive(instanceId: string): void {
  if (activeDeployId.value === instanceId) {
    dismissDeploy();
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
      await createBackup(action.instance.id, action.note);
      break;
    case "upgrade":
      if (action.release && !isVersionInstalled(action.release.tag_name)) {
        beginDeploy(action.instance.id);
        upgradeDownloadId.value = action.release.tag_name;
        try {
          await installVersion(action.release);
        } finally {
          upgradeDownloadId.value = null;
        }
      }
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

async function runLockable(
  action: LockableAction,
  showSuccess = true,
): Promise<boolean> {
  if (isInstanceBusy(action.instance.id)) {
    showToast(t("instances.toasts.busy"), "info");
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
      success: showSuccess ? actionSuccessMessage(action) : undefined,
      onError: handleOperationError(action),
    },
  );

  if (action.kind === "upgrade") {
    dismissDeployIfActive(action.instance.id);
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
    dismissDeployIfActive(action.instance.id);
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
    { success: t("instances.toasts.created") },
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
    showToast(t("instances.toasts.missing"), "error");
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
    { success: t("instances.toasts.updated") },
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

  const note = confirmNote.value.trim();
  confirmNote.value = "";

  switch (current.kind) {
    case "delete":
      await runLockable({ kind: "delete", instance: current.instance });
      break;
    case "backup":
      await runLockable({
        kind: "backup",
        instance: current.instance,
        note,
      });
      break;
    case "upgrade":
      if (current.request) {
        await runLockable({
          kind: "upgrade",
          instance: current.instance,
          request: current.request,
          release: current.release,
        });
      }
      break;
    case "clear-data":
      await run(
        clearDataKey(current.instance.id),
        () => clearInstanceData(current.instance.id),
        { success: t("instances.toasts.dataCleared") },
      );
      break;
    case "clear-venv":
      await run(
        clearVenvKey(current.instance.id),
        () => clearInstanceVenv(current.instance.id),
        { success: t("instances.toasts.venvCleared") },
      );
      break;
    case "clear-pycache":
      await run(
        clearPycacheKey(current.instance.id),
        () => clearPycache(current.instance.id),
        { success: t("instances.toasts.pycacheCleared") },
      );
      break;
  }
}

function askDelete(instance: InstanceStatus): void {
  confirmState.value = { kind: "delete", instance };
}

function askBackup(instance: InstanceStatus): void {
  confirmNote.value = "";
  confirmState.value = { kind: "backup", instance };
}

function askClear(
  instance: InstanceStatus,
  kind: "clear-data" | "clear-venv" | "clear-pycache",
): void {
  confirmState.value = { kind, instance };
}

function askUpgrade(instance: InstanceStatus): void {
  const release = latestRelease.value;
  if (
    !release ||
    batchUpgradeRunning.value ||
    instance.state !== "stopped" ||
    isInstanceBusy(instance.id) ||
    updateInfoFor(instance.id)?.state !== "available"
  ) {
    return;
  }

  confirmState.value = {
    kind: "upgrade",
    instance,
    request: { version: release.tag_name },
    version: release.tag_name,
    release,
  };
}

function askUpgradeAll(): void {
  const release = latestRelease.value;
  if (
    !release ||
    batchUpgradeRunning.value ||
    batchUpgradeTargets.value.length === 0
  ) {
    return;
  }

  upgradeAllTargets.value = [...batchUpgradeTargets.value];
  upgradeAllRelease.value = release;
  upgradeAllOpen.value = true;
}

const upgradeAllMessage = computed(() => {
  const release = upgradeAllRelease.value;
  if (!release) {
    return "";
  }

  const message = t("instances.updates.confirm.message", {
    count: upgradeAllTargets.value.length,
    version: release.tag_name,
  });
  const skipped = skippedUpgradeCount.value;
  return skipped > 0
    ? `${message}
${t("instances.updates.confirm.skipped", { count: skipped })}`
    : message;
});

async function handleUpgradeAll(): Promise<void> {
  const release = upgradeAllRelease.value;
  const targets = [...upgradeAllTargets.value];
  upgradeAllOpen.value = false;

  if (!release || targets.length === 0) {
    return;
  }

  batchUpgradeRunning.value = true;
  let completed = 0;

  try {
    for (const target of targets) {
      const current = instances.value.find((instance) => instance.id === target.id);
      if (
        !current ||
        current.state !== "stopped" ||
        updateInfoFor(current.id)?.state !== "available"
      ) {
        continue;
      }

      const ok = await runLockable(
        {
          kind: "upgrade",
          instance: current,
          request: { version: release.tag_name },
          release,
        },
        false,
      );
      if (!ok) {
        showToast(t("instances.updates.batchPaused"), "info");
        break;
      }
      completed += 1;
    }

    if (completed === targets.length) {
      showToast(
        t("instances.updates.batchComplete", { count: completed }),
        "success",
      );
    }
  } finally {
    batchUpgradeRunning.value = false;
    upgradeDownloadId.value = null;
    upgradeAllRelease.value = null;
    upgradeAllTargets.value = [];
    void loadUpdateCheck();
  }
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

  await run(
    repairKey(target.id),
    () => repairInstance(target.id, scope),
    { success: t("instances.toasts.repairComplete") },
  );

  dismissDeployIfActive(target.id);
}

async function handleStart(instance: InstanceStatus): Promise<void> {
  if (isInstanceBusy(instance.id)) {
    return;
  }

  await run(startKey(instance.id), () => startInstance(instance.id), {
    success: instanceReadyMessage(instance),
  });
}

async function handleStop(instance: InstanceStatus): Promise<void> {
  await run(stopKey(instance.id), () => stopInstance(instance.id), {
    success: t("instances.toasts.stopped"),
  });
}

async function handleRestart(instance: InstanceStatus): Promise<void> {
  if (isInstanceBusy(instance.id)) {
    return;
  }

  await run(restartKey(instance.id), () => restartInstance(instance.id), {
    success: instanceReadyMessage(instance),
  });
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

async function loadHealthCheck(): Promise<void> {
  const target = healthTarget.value;
  if (!target) {
    return;
  }

  const request = ++healthRequest;
  healthLoading.value = true;
  healthReport.value = null;
  healthError.value = null;
  try {
    const report = await checkInstanceHealth(target.id);
    if (request === healthRequest) {
      healthReport.value = report;
    }
  } catch (cause) {
    if (request === healthRequest) {
      healthError.value = toUserMessage(cause);
    }
  } finally {
    if (request === healthRequest) {
      healthLoading.value = false;
    }
  }
}

function openHealthCheck(instance: InstanceStatus): void {
  if (healthLoading.value || isInstanceInteractionDisabled(instance.id)) {
    return;
  }
  healthTarget.value = instance;
  healthOpen.value = true;
  void loadHealthCheck();
}

function openResourceMonitor(instance: InstanceStatus): void {
  resourceTarget.value = instance;
  resourceOpen.value = true;
}

function openWebui(instance: InstanceStatus): void {
  openInstanceTab(instance.id);
}

</script>

<template>
  <div class="page-shell">
    <PageHeader :title="t('instances.title')">
      <template #actions>
        <v-btn
          variant="text"
          prepend-icon="mdi-refresh"
          :loading="loading || updatesLoading"
          :disabled="batchUpgradeRunning"
          @click="handleRefresh"
        >
          {{ t("common.actions.refresh") }}
        </v-btn>
        <v-btn
          v-if="batchUpgradeRunning || batchUpgradeTargets.length > 0"
          color="primary"
          variant="text"
          prepend-icon="mdi-update"
          :loading="batchUpgradeRunning"
          :disabled="batchUpgradeRunning || updatesLoading"
          @click="askUpgradeAll"
        >
          {{ t("instances.actions.upgradeAll") }}
        </v-btn>
        <v-btn
          color="primary"
          variant="flat"
          prepend-icon="mdi-plus"
          :disabled="snapshot === null || batchUpgradeRunning"
          @click="openCreate"
        >
          {{ t("instances.actions.create") }}
        </v-btn>
      </template>
    </PageHeader>

    <v-alert
      v-if="error"
      type="error"
      variant="tonal"
      density="comfortable"
      class="instances-alert page-alert"
    >
      <div class="instances-alert__body">
        <span>{{ error }}</span>
        <v-btn
          size="small"
          variant="text"
          :loading="loading"
          @click="refresh()"
        >
          {{ t("common.actions.retry") }}
        </v-btn>
      </div>
    </v-alert>

    <v-alert
      v-if="snapshot !== null && updatesError"
      type="warning"
      variant="tonal"
      density="comfortable"
      class="instances-alert page-alert"
    >
      <div class="instances-alert__body">
        <span>{{ updatesError }}</span>
        <v-btn
          size="small"
          variant="text"
          :loading="updatesLoading"
          @click="retryUpdateCheck"
        >
          {{ t("common.actions.retry") }}
        </v-btn>
      </div>
    </v-alert>

    <div v-if="snapshot === null && !error" class="instances-state page-center">
      <v-progress-circular
        indeterminate
        color="primary"
        size="32"
        :aria-label="t('instances.loading')"
      />
    </div>

    <div
      v-else-if="snapshot !== null && instanceRows.length === 0"
      class="page-table-card"
    >
      <EmptyState :text="t('instances.list.empty')" icon="mdi-server-plus-outline">
        <v-btn
          color="primary"
          variant="flat"
          size="small"
          prepend-icon="mdi-plus"
          @click="openCreate"
        >
          {{ t("instances.actions.create") }}
        </v-btn>
      </EmptyState>
    </div>

    <div v-else-if="snapshot !== null" class="page-table-card">
      <v-data-table
        :headers="headers"
        :items="instanceRows"
        item-value="id"
        :mobile="mdAndDown"
        :loading="loading"
        hide-default-footer
        :items-per-page="-1"
        disable-sort
        hover
        :no-data-text="t('instances.list.empty')"
      >
        <template #item.name="{ item }">
          <div class="page-cell">
            <div class="page-row">
              <span>{{ item.name }}</span>
              <v-chip
                v-if="item.external"
                size="x-small"
                variant="tonal"
                label
              >
                {{ sourceTitleOf(item.source) }}
              </v-chip>
            </div>
            <span
              v-if="!item.external"
              class="instance-meta page-muted page-nums"
              :aria-label="
                t('instances.list.meta', {
                  version: item.version,
                  port:
                    item.state === 'running' ? item.port : item.configured_port,
                })
              "
            >
              <v-icon size="12" icon="mdi-tag-outline" aria-hidden="true" />
              <span>{{ item.version }}</span>
              <span aria-hidden="true">·</span>
              <v-icon size="12" icon="mdi-ethernet" aria-hidden="true" />
              <span>
                {{ item.state === "running" ? item.port : item.configured_port }}
              </span>
            </span>
          </div>
        </template>

        <template #item.state="{ item }">
          <StatusChip
            v-if="item.external"
            :state="item.state"
          />
          <InstanceStatusCell v-else :instance="item" />
        </template>

        <template #item.actions="{ item }">
          <div v-if="item.external" class="instance-actions page-row">
            <v-tooltip location="top">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-monitor-dashboard"
                  variant="text"
                  size="small"
                  :aria-label="t('instances.actions.openWebui')"
                  :disabled="item.webui_url === null"
                  @click="openExternalWebui(item)"
                />
              </template>
              {{ t("instances.actions.openWebui") }}
            </v-tooltip>
          </div>
          <div v-else class="instance-actions page-row">
            <template v-if="item.state === 'stopped'">
              <v-tooltip location="top">
                <template #activator="{ props: tooltipProps }">
                  <v-btn
                    v-bind="tooltipProps"
                    icon="mdi-play"
                    color="success"
                    variant="tonal"
                    size="small"
                    :aria-label="t('instances.actions.start')"
                    :disabled="isInstanceInteractionDisabled(item.id)"
                    :loading="isBusy(startKey(item.id))"
                    @click="handleStart(item)"
                  />
                </template>
                {{ t("instances.actions.start") }}
              </v-tooltip>
            </template>
            <template v-else-if="item.state === 'running'">
              <v-tooltip location="top">
                <template #activator="{ props: tooltipProps }">
                  <v-btn
                    v-bind="tooltipProps"
                    icon="mdi-stop"
                    color="warning"
                    variant="text"
                    size="small"
                    :aria-label="t('instances.actions.stop')"
                    :disabled="isInstanceInteractionDisabled(item.id)"
                    :loading="isBusy(stopKey(item.id))"
                    @click="handleStop(item)"
                  />
                </template>
                {{ t("instances.actions.stop") }}
              </v-tooltip>
              <v-tooltip location="top">
                <template #activator="{ props: tooltipProps }">
                  <v-btn
                    v-bind="tooltipProps"
                    icon="mdi-restart"
                    variant="text"
                    size="small"
                    :aria-label="t('instances.actions.restart')"
                    :disabled="isInstanceInteractionDisabled(item.id)"
                    :loading="isBusy(restartKey(item.id))"
                    @click="handleRestart(item)"
                  />
                </template>
                {{ t("instances.actions.restart") }}
              </v-tooltip>
            </template>
            <v-tooltip v-else location="top">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-loading"
                  variant="tonal"
                  size="small"
                  disabled
                  loading
                  :aria-label="
                    item.state === 'starting'
                      ? t('common.instanceState.starting')
                      : t('common.instanceState.stopping')
                  "
                />
              </template>
              {{
                item.state === "starting"
                  ? t("common.instanceState.starting")
                  : t("common.instanceState.stopping")
              }}
            </v-tooltip>

            <v-tooltip
              v-if="updateInfoFor(item.id)?.state === 'available'"
              location="top"
            >
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-arrow-up-bold-circle"
                  color="primary"
                  variant="text"
                  size="small"
                  :aria-label="t('instances.actions.upgrade')"
                  :disabled="
                    item.state !== 'stopped' ||
                    isInstanceInteractionDisabled(item.id)
                  "
                  :loading="isBusy(upgradeKey(item.id))"
                  :title="
                    item.state !== 'stopped'
                      ? t('instances.updates.stopFirst')
                      : undefined
                  "
                  @click="askUpgrade(item)"
                />
              </template>
              {{ t("instances.actions.upgrade") }}
            </v-tooltip>

            <v-tooltip location="top">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-monitor-dashboard"
                  variant="text"
                  size="small"
                  :aria-label="t('instances.actions.openWebui')"
                  :disabled="!item.dashboard_enabled"
                  @click="openWebui(item)"
                />
              </template>
              {{ t("instances.actions.openWebui") }}
            </v-tooltip>

            <v-tooltip location="top">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-text-box-outline"
                  variant="text"
                  size="small"
                  :aria-label="t('instances.actions.viewLogs')"
                  @click="openLogs(item)"
                />
              </template>
              {{ t("instances.actions.viewLogs") }}
            </v-tooltip>

            <v-tooltip v-if="item.state === 'running'" location="top">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-chart-timeline-variant"
                  variant="text"
                  size="small"
                  :aria-label="t('resources.open')"
                  @click="openResourceMonitor(item)"
                />
              </template>
              {{ t("resources.open") }}
            </v-tooltip>

            <v-menu location="bottom end" :disabled="batchUpgradeRunning">
              <template #activator="{ props }">
                <v-btn
                  v-bind="props"
                  icon="mdi-dots-horizontal"
                  variant="text"
                  size="small"
                  :aria-label="t('instances.list.aria.moreActions')"
                />
              </template>
              <v-list density="compact" class="instance-menu">
                <v-list-item
                  prepend-icon="mdi-folder-open-outline"
                  :title="t('instances.actions.openCoreFolder')"
                  @click="handleOpenCoreFolder(item)"
                />
                <v-list-item
                  prepend-icon="mdi-pencil-outline"
                  :title="t('common.actions.edit')"
                  :disabled="
                    item.state !== 'stopped' ||
                    isInstanceInteractionDisabled(item.id)
                  "
                  @click="openEdit(item)"
                />
                <v-list-item
                  prepend-icon="mdi-archive-arrow-down-outline"
                  :title="t('instances.actions.createBackup')"
                  :disabled="
                    item.state !== 'stopped' ||
                    isInstanceInteractionDisabled(item.id)
                  "
                  @click="askBackup(item)"
                />
                <v-list-item
                  prepend-icon="mdi-lifebuoy"
                  :title="t('instances.actions.repair')"
                  :disabled="
                    item.state !== 'stopped' ||
                    isInstanceInteractionDisabled(item.id)
                  "
                  @click="openRepair(item)"
                />
                <v-list-item
                  prepend-icon="mdi-heart-pulse"
                  :title="t('instances.actions.healthCheck')"
                  :disabled="
                    healthLoading || isInstanceInteractionDisabled(item.id)
                  "
                  @click="openHealthCheck(item)"
                />
                <v-list-item
                  prepend-icon="mdi-text-box-outline"
                  :title="t('instances.actions.viewLogs')"
                  @click="openLogs(item)"
                />
                <v-list-item
                  prepend-icon="mdi-chart-timeline-variant"
                  :title="t('resources.title')"
                  @click="openResourceMonitor(item)"
                />
                <v-list-item
                  prepend-icon="mdi-open-in-new"
                  :title="t('instances.actions.openInBrowser')"
                  :disabled="!item.dashboard_enabled"
                  @click="handleOpenInBrowser(item)"
                />
                <v-divider />
                <v-list-item
                  prepend-icon="mdi-database-remove-outline"
                  :title="t('instances.actions.clearData')"
                  :disabled="
                    item.state !== 'stopped' ||
                    isInstanceInteractionDisabled(item.id)
                  "
                  @click="askClear(item, 'clear-data')"
                />
                <v-list-item
                  prepend-icon="mdi-delete-sweep-outline"
                  :title="t('instances.actions.clearVenv')"
                  :disabled="
                    item.state !== 'stopped' ||
                    isInstanceInteractionDisabled(item.id)
                  "
                  @click="askClear(item, 'clear-venv')"
                />
                <v-list-item
                  prepend-icon="mdi-broom"
                  :title="t('instances.actions.clearPycache')"
                  :disabled="
                    item.state !== 'stopped' ||
                    isInstanceInteractionDisabled(item.id)
                  "
                  @click="askClear(item, 'clear-pycache')"
                />
                <v-divider />
                <v-list-item
                  prepend-icon="mdi-delete-outline"
                  :title="t('instances.actions.deleteInstance')"
                  base-color="error"
                  :disabled="
                    item.state !== 'stopped' ||
                    isInstanceInteractionDisabled(item.id)
                  "
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

    <ResourceMonitorDialog
      v-model="resourceOpen"
      :instance="resourceTarget"
    />

    <HealthCheckDialog
      v-model="healthOpen"
      :instance-name="healthTarget?.name ?? ''"
      :report="healthReport"
      :loading="healthLoading"
      :error="healthError"
      @retry="loadHealthCheck"
    />

    <ConfirmDialog
      v-model="confirmOpen"
      :title="confirmTitle"
      :message="confirmMessage"
      :confirm-label="confirmLabel"
      :cancel-label="t('common.actions.cancel')"
      :color="confirmColor"
      :confirm-disabled="
        confirmState?.kind === 'backup' && confirmNoteTooLong
      "
      @confirm="handleConfirm"
    >
      <BackupNoteField
        v-if="confirmState?.kind === 'backup'"
        v-model="confirmNote"
      />
    </ConfirmDialog>

    <ConfirmDialog
      v-model="upgradeAllOpen"
      :title="t('instances.updates.confirm.title')"
      :message="upgradeAllMessage"
      :confirm-label="t('instances.actions.upgradeAll')"
      :cancel-label="t('common.actions.cancel')"
      color="warning"
      :loading="batchUpgradeRunning"
      @confirm="handleUpgradeAll"
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
      :title="t('instances.deploy.title')"
      :message="activeTaskMessage"
      :progress="activeTaskProgress"
    />
  </div>
</template>

<style scoped>
.instances-alert__body {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.instances-state {
  min-height: 180px;
}

.instance-actions {
  justify-content: flex-end;
  gap: 4px;
}

.instance-meta {
  display: inline-flex;
  align-items: center;
  gap: 4px;
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
