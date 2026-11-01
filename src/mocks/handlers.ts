import { emit } from "@tauri-apps/api/event";
import type {
  DownloadProgress,
  GitHubRelease,
  LogEntry,
  ThemePreference,
  UpdateInstanceRequest,
} from "../types/launcher";
import { launcherReleaseNotes } from "./fixtures";
import { MockStore } from "./store";

export type MockCommandHandler = (
  args: Record<string, unknown>,
) => unknown | Promise<unknown>;

interface ProgressStep {
  progress: number;
  step: string;
  message: string;
}

const DOWNLOAD_TOTAL_BYTES = 18_735_104;

const downloadSteps: ProgressStep[] = [
  { progress: 0, step: "prepare", message: "准备下载" },
  { progress: 24, step: "download", message: "下载中" },
  { progress: 52, step: "download", message: "下载中" },
  { progress: 81, step: "download", message: "下载中" },
  { progress: 99, step: "extract", message: "校验文件" },
  { progress: 100, step: "done", message: "下载完成" },
];

export const store = new MockStore();

let windowMaximized = false;
let autostartEnabled = false;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function readString(args: Record<string, unknown>, key: string): string {
  const value = args[key];
  if (typeof value !== "string") {
    throw new Error(`mock 参数 ${key} 必须是字符串`);
  }
  return value;
}

function readNumber(args: Record<string, unknown>, key: string): number {
  const value = args[key];
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error(`mock 参数 ${key} 必须是数字`);
  }
  return value;
}

function readBoolean(args: Record<string, unknown>, key: string): boolean {
  const value = args[key];
  if (typeof value !== "boolean") {
    throw new Error(`mock 参数 ${key} 必须是布尔值`);
  }
  return value;
}

function readThemePreference(args: Record<string, unknown>): ThemePreference {
  const value = readString(args, "themePreference");
  if (value === "light" || value === "dark" || value === "system") {
    return value;
  }
  throw new Error(`未知主题：${value}`);
}

function readUpdateRequest(args: Record<string, unknown>): UpdateInstanceRequest {
  const value = args.request;
  if (!isRecord(value)) {
    throw new Error("mock 参数 request 必须是对象");
  }

  const request: UpdateInstanceRequest = {};

  if (value.name !== undefined) {
    if (typeof value.name !== "string") {
      throw new Error("mock 参数 request.name 必须是字符串");
    }
    request.name = value.name;
  }
  if (value.version !== undefined) {
    if (typeof value.version !== "string") {
      throw new Error("mock 参数 request.version 必须是字符串");
    }
    request.version = value.version;
  }
  if (value.host !== undefined) {
    if (typeof value.host !== "string") {
      throw new Error("mock 参数 request.host 必须是字符串");
    }
    request.host = value.host;
  }
  if (value.port !== undefined) {
    if (typeof value.port !== "number" || !Number.isInteger(value.port)) {
      throw new Error("mock 参数 request.port 必须是整数");
    }
    request.port = value.port;
  }
  if (value.checkUpdateEnabled !== undefined) {
    if (typeof value.checkUpdateEnabled !== "boolean") {
      throw new Error("mock 参数 request.checkUpdateEnabled 必须是布尔值");
    }
    request.checkUpdateEnabled = value.checkUpdateEnabled;
  }

  return request;
}

function readRelease(args: Record<string, unknown>): GitHubRelease {
  const value = args.release;
  if (!isRecord(value) || typeof value.tag_name !== "string") {
    throw new Error("mock 参数 release 不是有效的版本信息");
  }

  const assets = Array.isArray(value.assets)
    ? value.assets.filter(isRecord).map((asset) => ({
        name: typeof asset.name === "string" ? asset.name : "",
        browser_download_url:
          typeof asset.browser_download_url === "string"
            ? asset.browser_download_url
            : "",
        size: typeof asset.size === "number" ? asset.size : 0,
      }))
    : [];

  return {
    tag_name: value.tag_name,
    name: typeof value.name === "string" ? value.name : value.tag_name,
    published_at:
      typeof value.published_at === "string" ? value.published_at : "",
    prerelease: value.prerelease === true,
    assets,
    html_url: typeof value.html_url === "string" ? value.html_url : "",
    body: typeof value.body === "string" ? value.body : null,
  };
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

async function emitSnapshot(): Promise<void> {
  await emit("app-snapshot", store.snapshot);
}

async function emitLog(
  source: string,
  level: LogEntry["level"],
  message: string,
): Promise<void> {
  await emit("log-entry", {
    source,
    level,
    message,
    timestamp: new Date().toISOString(),
  });
}

const componentSteps: ProgressStep[] = [
  { progress: 0, step: "prepare", message: "准备下载" },
  { progress: 45, step: "download", message: "下载中" },
  { progress: 100, step: "done", message: "安装完成" },
];

async function simulateComponentDownload(id: string): Promise<void> {
  for (const item of componentSteps) {
    await emit("download-progress", {
      id,
      downloaded: 0,
      total: null,
      progress: item.progress,
      step: item.step,
      message: item.message,
    });

    if (item.progress < 100) {
      await delay(160);
    }
  }
}

function requireStoppedInstance(id: string) {
  const instance = store.requireInstance(id);
  if (instance.state !== "stopped") {
    throw new Error("请先停止实例后再执行此操作");
  }
  return instance;
}

async function simulateDownload(tag: string): Promise<void> {
  for (const item of downloadSteps) {
    const payload: DownloadProgress = {
      id: tag,
      downloaded: Math.round(
        (DOWNLOAD_TOTAL_BYTES * item.progress) / 100,
      ),
      total: DOWNLOAD_TOTAL_BYTES,
      progress: item.progress,
      step: item.step,
      message: item.message,
    };

    await emit("download-progress", payload);

    if (item.progress < 100) {
      await delay(140);
    }
  }
}

function compareVersions(a: string, b: string): number {
  const parse = (raw: string): number[] | null => {
    const core = raw
      .trim()
      .replace(/^v/, "")
      .split("-")[0]
      .split("+")[0];
    const parts = core.split(".").map((part) => Number.parseInt(part, 10));
    return parts.some((part) => Number.isNaN(part)) ? null : parts;
  };

  const left = parse(a);
  const right = parse(b);
  if (!left || !right) {
    return 0;
  }

  const length = Math.max(left.length, right.length);
  for (let index = 0; index < length; index += 1) {
    const leftPart = left[index] ?? 0;
    const rightPart = right[index] ?? 0;
    if (leftPart !== rightPart) {
      return leftPart < rightPart ? -1 : 1;
    }
  }

  return 0;
}

async function startInstance(id: string): Promise<number> {
  const instance = store.requireInstance(id);
  if (instance.state === "running") {
    return instance.port;
  }

  await emitLog(instance.id, "info", `正在启动实例 ${instance.name}`);
  store.setInstanceState(id, "starting");
  await emitSnapshot();
  await delay(400);
  store.setInstanceState(id, "running");
  await emitLog(instance.id, "info", "AstrBot started.");
  await emitLog("system", "info", `实例 ${instance.name} 已启动`);
  await emitSnapshot();
  return instance.configured_port;
}

async function stopInstance(id: string): Promise<void> {
  const instance = store.requireInstance(id);
  if (instance.state === "stopped") {
    return;
  }

  await emitLog(instance.id, "info", `正在停止实例 ${instance.name}`);
  store.setInstanceState(id, "stopping");
  await emitSnapshot();
  await delay(280);
  store.setInstanceState(id, "stopped");
  await emitLog(instance.id, "info", "实例已停止");
  await emitLog("system", "info", `实例 ${instance.name} 已停止`);
  await emitSnapshot();
}

export const mockCommands: Record<string, MockCommandHandler> = {
  // Snapshot
  get_app_snapshot: () => store.snapshot,
  rebuild_app_snapshot: () => store.snapshot,

  // Config
  save_github_proxy: (args) => {
    store.updateConfig({ github_proxy: readString(args, "githubProxy") });
  },
  save_proxy: (args) => {
    store.updateConfig({
      proxy_url: readString(args, "proxyUrl"),
      proxy_port: readString(args, "proxyPort"),
      proxy_username: readString(args, "proxyUsername"),
      proxy_password: readString(args, "proxyPassword"),
    });
  },
  save_pypi_mirror: (args) => {
    store.updateConfig({ pypi_mirror: readString(args, "pypiMirror") });
  },
  save_nodejs_mirror: (args) => {
    store.updateConfig({ nodejs_mirror: readString(args, "nodejsMirror") });
  },
  save_npm_registry: (args) => {
    store.updateConfig({ npm_registry: readString(args, "npmRegistry") });
  },
  save_use_uv_for_deps: (args) => {
    store.updateConfig({
      use_uv_for_deps: readBoolean(args, "useUvForDeps"),
    });
  },
  save_close_to_tray: (args) => {
    store.updateConfig({ close_to_tray: readBoolean(args, "closeToTray") });
  },
  save_autostart_minimize_to_tray: (args) => {
    store.updateConfig({
      autostart_minimize_to_tray: readBoolean(
        args,
        "autostartMinimizeToTray",
      ),
    });
  },
  save_check_instance_update: (args) => {
    store.updateConfig({
      check_instance_update: readBoolean(args, "checkInstanceUpdate"),
    });
  },
  save_persist_instance_state: (args) => {
    store.updateConfig({
      persist_instance_state: readBoolean(args, "persistInstanceState"),
    });
  },
  save_ignore_external_path: (args) => {
    store.updateConfig({
      ignore_external_path: readBoolean(args, "ignoreExternalPath"),
    });
  },
  save_use_short_instance_id: (args) => {
    store.updateConfig({
      use_short_instance_id: readBoolean(args, "useShortInstanceId"),
    });
  },
  save_lock_check_extension_whitelist: (args) => {
    store.updateConfig({
      lock_check_extension_whitelist: readBoolean(
        args,
        "lockCheckExtensionWhitelist",
      ),
    });
  },
  save_theme_preference: (args) => {
    store.updateConfig({
      theme_preference: readThemePreference(args),
    });
  },
  save_mainland_acceleration: (args) => {
    store.updateConfig({
      mainland_acceleration: readBoolean(args, "mainlandAcceleration"),
    });
  },
  compare_versions: (args) =>
    compareVersions(readString(args, "a"), readString(args, "b")),
  is_macos: () => navigator.userAgent.includes("Mac"),

  // Data directory
  set_data_dir: async (args) => {
    const oldDir = store.dataDir;
    const newDir = readString(args, "newDir").trim();

    if (!newDir) {
      throw new Error("新数据目录不能为空");
    }
    if (newDir === oldDir) {
      throw new Error("新数据目录与当前数据目录相同");
    }

    store.setDataDir(newDir);
    await emitSnapshot();

    return { old_dir: oldDir, new_dir: newDir };
  },
  open_folder: (args) => {
    readString(args, "path");
  },

  // Components
  install_component: async (args) => {
    const componentId = readString(args, "componentId");
    await simulateComponentDownload(componentId);
    store.setComponentInstalled(componentId, true);
    await emitSnapshot();
    return "组件安装完成";
  },
  reinstall_component: async (args) => {
    const componentId = readString(args, "componentId");
    await simulateComponentDownload(componentId);
    store.setComponentInstalled(componentId, true);
    await emitSnapshot();
    return "组件重装完成";
  },
  uninstall_component: async (args) => {
    store.setComponentInstalled(readString(args, "componentId"), false);
    await emitSnapshot();
    return "组件已卸载";
  },

  // GitHub
  fetch_releases: () => store.releases,
  fetch_launcher_release_notes: () => launcherReleaseNotes,

  // Version management
  install_version: async (args) => {
    const release = readRelease(args);
    await emitLog("system", "info", `开始下载版本 ${release.tag_name}`);
    await simulateDownload(release.tag_name);
    store.addVersion(release.tag_name);
    await emitLog("system", "info", `版本 ${release.tag_name} 安装完成`);
    await emitSnapshot();
  },
  uninstall_version: async (args) => {
    const version = readString(args, "version");
    if (store.isVersionInUse(version)) {
      throw new Error(`版本正在被实例使用：${version}`);
    }
    store.removeVersion(version);
    await emitSnapshot();
  },

  // Troubleshooting
  check_lock: () => undefined,
  clear_instance_data: async (args) => {
    requireStoppedInstance(readString(args, "instanceId"));
  },
  clear_instance_venv: async (args) => {
    requireStoppedInstance(readString(args, "instanceId"));
  },
  clear_pycache: async (args) => {
    requireStoppedInstance(readString(args, "instanceId"));
  },
  clear_uv_cache: () => undefined,
  repair_instance: async (args) => {
    const instance = requireStoppedInstance(readString(args, "instanceId"));
    await emit("deploy-progress", {
      instance_id: instance.id,
      step: "repair",
      message: "正在修复实例",
      progress: 50,
    });
    await delay(300);
    await emit("deploy-progress", {
      instance_id: instance.id,
      step: "done",
      message: "修复完成",
      progress: 100,
    });
  },
  rebuild_instance_manifest: () => {
    if (store.hasRunningInstances()) {
      throw new Error("请先停止所有运行中的实例");
    }
    const snapshot = store.snapshot;
    return {
      instances: snapshot.instances.length,
      versions: snapshot.versions.length,
    };
  },

  // Instance management
  open_instance_core_folder: (args) => {
    store.requireInstance(readString(args, "instanceId"));
  },
  create_instance: async (args) => {
    const name = readString(args, "name").trim();
    const version = readString(args, "version");
    const port = readNumber(args, "port");

    if (!name) {
      throw new Error("实例名称不能为空");
    }
    if (!store.hasVersion(version)) {
      throw new Error(`版本未安装：${version}`);
    }
    if (store.isPortInUse(port)) {
      throw new Error(`端口已被占用：${port}`);
    }

    store.addInstance(name, version, port);
    await emitSnapshot();
  },
  delete_instance: async (args) => {
    const id = readString(args, "instanceId");
    requireStoppedInstance(id);
    store.removeInstance(id);
    await emitSnapshot();
  },
  update_instance: async (args) => {
    const id = readString(args, "instanceId");
    const instance = requireStoppedInstance(id);
    const request = readUpdateRequest(args);

    if (request.version !== undefined && !store.hasVersion(request.version)) {
      throw new Error(`版本未安装：${request.version}`);
    }
    if (
      request.port !== undefined &&
      request.port !== instance.configured_port &&
      store.isPortInUse(request.port)
    ) {
      throw new Error(`端口已被占用：${request.port}`);
    }

    store.updateInstance(id, request);
    await emitSnapshot();
  },
  start_instance: (args) => startInstance(readString(args, "instanceId")),
  stop_instance: async (args) => {
    await stopInstance(readString(args, "instanceId"));
  },
  restart_instance: async (args) => {
    const id = readString(args, "instanceId");
    await stopInstance(id);
    return startInstance(id);
  },
  get_instance_port: (args) => {
    const instance = store.requireInstance(readString(args, "instanceId"));
    if (instance.state !== "running") {
      throw new Error("实例未运行");
    }
    return instance.port;
  },

  // Backup
  create_backup: async (args) => {
    const instance = requireStoppedInstance(readString(args, "instanceId"));
    const backup = store.createBackup(instance.id);
    await emitLog("system", "info", `已创建备份：${backup.filename}`);
    await emitSnapshot();
    return backup.path;
  },
  restore_backup: async (args) => {
    const backupPath = readString(args, "backupPath");
    const backup = store.findBackup(backupPath);
    if (!backup) {
      throw new Error(`备份不存在：${backupPath}`);
    }
    if (backup.corrupted) {
      throw new Error("备份文件已损坏");
    }

    requireStoppedInstance(backup.metadata.instance_id);
    await emitSnapshot();
  },
  delete_backup: async (args) => {
    const backupPath = readString(args, "backupPath");
    if (!store.findBackup(backupPath)) {
      throw new Error(`备份不存在：${backupPath}`);
    }
    store.removeBackup(backupPath);
    await emitSnapshot();
  },
};

export const mockPluginCommands: Record<string, MockCommandHandler> = {
  "plugin:window|minimize": () => undefined,
  "plugin:window|toggle_maximize": () => {
    windowMaximized = !windowMaximized;
  },
  "plugin:window|is_maximized": () => windowMaximized,
  "plugin:window|close": () => undefined,
  "plugin:window|start_dragging": () => undefined,
  "plugin:decorum|show_snap_overlay": () => undefined,
  "plugin:opener|open_url": () => undefined,
  "plugin:opener|open_path": () => undefined,
  "plugin:opener|reveal_item_in_dir": () => undefined,
  "plugin:dialog|open": () => "C:/Users/Mock/AstrBotLauncher/data-alt",
  "plugin:app|version": () => "0.1.0",
  "plugin:autostart|is_enabled": () => autostartEnabled,
  "plugin:autostart|enable": () => {
    autostartEnabled = true;
  },
  "plugin:autostart|disable": () => {
    autostartEnabled = false;
  },
};
