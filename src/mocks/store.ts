import type {
  AppConfig,
  AppSnapshot,
  BackupInfo,
  GitHubRelease,
  InstanceState,
  InstanceStatus,
  InstalledVersion,
  UpdateInstanceRequest,
} from "../types/launcher";
import { initialReleases, initialSnapshot } from "./fixtures";

function clone<T>(value: T): T {
  return structuredClone(value);
}

function pad(value: number): string {
  return value.toString().padStart(2, "0");
}

function randomInstanceId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(8));
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join(
    "",
  );
}

function formatBackupStamp(date: Date): string {
  return [
    date.getFullYear(),
    pad(date.getMonth() + 1),
    pad(date.getDate()),
    "-",
    pad(date.getHours()),
    pad(date.getMinutes()),
    pad(date.getSeconds()),
  ].join("");
}

function detectArchTarget(): string {
  const userAgent = navigator.userAgent;
  if (userAgent.includes("Windows")) {
    return "windows-x86_64";
  }
  if (userAgent.includes("Mac")) {
    return "darwin-aarch64";
  }
  return "linux-x86_64";
}

/**
 * In-memory copy of the launcher backend state.
 *
 * Every command handler mutates this store instead of the fixture objects,
 * so repeated actions during a session stay consistent.
 */
export class MockStore {
  private snapshotValue: AppSnapshot = clone(initialSnapshot);
  private releasesValue: GitHubRelease[] = clone(initialReleases);

  get snapshot(): AppSnapshot {
    return clone(this.snapshotValue);
  }

  get releases(): GitHubRelease[] {
    return clone(this.releasesValue);
  }

  get dataDir(): string {
    return this.snapshotValue.data_dir;
  }

  updateConfig(patch: Partial<AppConfig>): void {
    this.snapshotValue.config = {
      ...this.snapshotValue.config,
      ...patch,
    };
  }

  hasVersion(version: string): boolean {
    return this.snapshotValue.versions.some(
      (installed) => installed.version === version,
    );
  }

  isVersionInUse(version: string): boolean {
    return this.snapshotValue.instances.some(
      (instance) => instance.version === version,
    );
  }

  isPortInUse(port: number): boolean {
    return this.snapshotValue.instances.some(
      (instance) => instance.configured_port === port,
    );
  }

  hasRunningInstances(): boolean {
    return this.snapshotValue.instances.some(
      (instance) => instance.state !== "stopped",
    );
  }

  requireInstance(id: string): InstanceStatus {
    const instance = this.snapshotValue.instances.find(
      (candidate) => candidate.id === id,
    );

    if (!instance) {
      throw new Error(`实例不存在：${id}`);
    }

    return instance;
  }

  setInstanceState(id: string, state: InstanceState): void {
    this.requireInstance(id).state = state;
  }

  addInstance(name: string, version: string, port: number): InstanceStatus {
    const instance: InstanceStatus = {
      id: randomInstanceId(),
      name,
      state: "stopped",
      port,
      version,
      dashboard_enabled: true,
      pid_tracker_not_available: false,
      configured_host: "127.0.0.1",
      configured_port: port,
      check_update_enabled: true,
    };

    this.snapshotValue.instances.push(instance);
    return instance;
  }

  updateInstance(id: string, request: UpdateInstanceRequest): void {
    const instance = this.requireInstance(id);

    if (request.name !== undefined) {
      instance.name = request.name;
    }
    if (request.version !== undefined) {
      instance.version = request.version;
    }
    if (request.host !== undefined) {
      instance.configured_host = request.host;
    }
    if (request.port !== undefined) {
      instance.configured_port = request.port;
      instance.port = request.port;
    }
    if (request.checkUpdateEnabled !== undefined) {
      instance.check_update_enabled = request.checkUpdateEnabled;
    }
  }

  removeInstance(id: string): void {
    this.snapshotValue.instances = this.snapshotValue.instances.filter(
      (instance) => instance.id !== id,
    );
  }

  addVersion(version: string): InstalledVersion {
    this.snapshotValue.versions = this.snapshotValue.versions.filter(
      (installed) => installed.version !== version,
    );

    const installed: InstalledVersion = {
      version,
      zip_path: `${this.snapshotValue.data_dir}/versions/${version}.zip`,
    };
    this.snapshotValue.versions.push(installed);
    return installed;
  }

  removeVersion(version: string): void {
    this.snapshotValue.versions = this.snapshotValue.versions.filter(
      (installed) => installed.version !== version,
    );
  }

  createBackup(instanceId: string): BackupInfo {
    const instance = this.requireInstance(instanceId);
    const now = new Date();
    const filename = `backup-${instance.id.slice(0, 8)}-${formatBackupStamp(now)}.zip`;

    const backup: BackupInfo = {
      filename,
      path: `${this.snapshotValue.data_dir}/backups/${filename}`,
      metadata: {
        created_at: now.toISOString(),
        instance_name: instance.name,
        instance_id: instance.id,
        version: instance.version,
        includes_venv: false,
        includes_data: true,
        arch_target: detectArchTarget(),
        auto_generated: false,
      },
      corrupted: false,
      parse_error: null,
    };

    this.snapshotValue.backups.unshift(backup);
    return backup;
  }

  findBackup(path: string): BackupInfo | undefined {
    return this.snapshotValue.backups.find((backup) => backup.path === path);
  }

  removeBackup(path: string): void {
    this.snapshotValue.backups = this.snapshotValue.backups.filter(
      (backup) => backup.path !== path,
    );
  }

  setComponentInstalled(id: string, installed: boolean): void {
    const component = this.snapshotValue.components.components.find(
      (candidate) => candidate.id === id,
    );

    if (!component) {
      throw new Error(`未知组件：${id}`);
    }

    component.installed = installed;
  }

  setDataDir(dir: string): void {
    this.snapshotValue.data_dir = dir;
  }
}
