/**
 * Deterministic seed data for the development mock backend.
 *
 * Keep values realistic but clearly fake. Do not put real user paths,
 * tokens, or account information in this file.
 */

import type {
  AppConfig,
  AppSnapshot,
  GitHubRelease,
} from "../types/launcher";

export const MOCK_DATA_DIR = "C:/Users/Mock/AstrBotLauncher";

export const initialConfig: AppConfig = {
  mainland_acceleration: true,
  github_proxy: "",
  proxy_url: "",
  proxy_port: "",
  proxy_username: "",
  proxy_password: "",
  pypi_mirror: "https://pypi.tuna.tsinghua.edu.cn/simple",
  nodejs_mirror: "",
  npm_registry: "",
  use_uv_for_deps: true,
  close_to_tray: true,
  autostart_minimize_to_tray: false,
  check_instance_update: true,
  persist_instance_state: false,
  ignore_external_path: false,
  use_short_instance_id: false,
  lock_check_extension_whitelist: false,
  theme_preference: "system",
};

export const initialSnapshot: AppSnapshot = {
  instances: [
    {
      id: "8f3c2a1d9b7e4f60",
      name: "主实例",
      state: "running",
      port: 6185,
      version: "v4.0.0",
      dashboard_enabled: true,
      pid_tracker_not_available: false,
      configured_host: "127.0.0.1",
      configured_port: 6185,
      check_update_enabled: true,
    },
    {
      id: "2b7e9d4c6a1f8035",
      name: "测试实例",
      state: "stopped",
      port: 6186,
      version: "v4.0.0",
      dashboard_enabled: true,
      pid_tracker_not_available: false,
      configured_host: "127.0.0.1",
      configured_port: 6186,
      check_update_enabled: true,
    },
    {
      id: "c41d8e2f5a9b7310",
      name: "插件调试",
      state: "stopped",
      port: 6190,
      version: "v3.5.22",
      dashboard_enabled: false,
      pid_tracker_not_available: true,
      configured_host: "0.0.0.0",
      configured_port: 6190,
      check_update_enabled: false,
    },
  ],
  versions: [
    {
      version: "v4.0.0",
      zip_path: `${MOCK_DATA_DIR}/data/versions/v4.0.0.zip`,
    },
    {
      version: "v3.5.22",
      zip_path: `${MOCK_DATA_DIR}/data/versions/v3.5.22.zip`,
    },
  ],
  backups: [
    {
      filename: "backup-main-20261028-211432.zip",
      path: `${MOCK_DATA_DIR}/data/backups/backup-main-20261028-211432.zip`,
      metadata: {
        created_at: "2026-10-28T21:14:32+08:00",
        instance_name: "主实例",
        instance_id: "8f3c2a1d9b7e4f60",
        version: "v4.0.0",
        includes_venv: false,
        includes_data: true,
        arch_target: "windows-x86_64",
        auto_generated: false,
      },
      corrupted: false,
      parse_error: null,
    },
    {
      filename: "backup-test-auto-20261030-090201.zip",
      path: `${MOCK_DATA_DIR}/data/backups/backup-test-auto-20261030-090201.zip`,
      metadata: {
        created_at: "2026-10-30T09:02:01+08:00",
        instance_name: "测试实例",
        instance_id: "2b7e9d4c6a1f8035",
        version: "v4.0.0",
        includes_venv: true,
        includes_data: true,
        arch_target: "windows-x86_64",
        auto_generated: true,
      },
      corrupted: false,
      parse_error: null,
    },
    {
      filename: "backup-broken-20261031-120000.zip",
      path: `${MOCK_DATA_DIR}/data/backups/backup-broken-20261031-120000.zip`,
      metadata: {
        created_at: "",
        instance_name: "",
        instance_id: "",
        version: "",
        includes_venv: false,
        includes_data: false,
        arch_target: "",
        auto_generated: false,
      },
      corrupted: true,
      parse_error: "无法读取 backup.toml",
    },
  ],
  components: {
    components: [
      {
        id: "python",
        installed: true,
        display_name: "Python",
        description: "Python 3.10 / 3.12 运行时",
      },
      {
        id: "nodejs",
        installed: false,
        display_name: "Node.js (LTS)",
        description: "Node.js 运行时",
      },
      {
        id: "uv",
        installed: true,
        display_name: "uv",
        description: "uv / uvx 包管理工具",
      },
    ],
  },
  config: initialConfig,
  data_dir: `${MOCK_DATA_DIR}/data`,
};

export const initialReleases: GitHubRelease[] = [
  {
    tag_name: "v4.1.0-beta.1",
    name: "AstrBot v4.1.0 Beta 1",
    published_at: "2026-10-30T08:00:00Z",
    prerelease: true,
    assets: [
      {
        name: "AstrBot-v4.1.0-beta.1.zip",
        browser_download_url:
          "https://github.com/AstrBotDevs/AstrBot/releases/download/v4.1.0-beta.1/AstrBot-v4.1.0-beta.1.zip",
        size: 19_503_104,
      },
      {
        name: "Source code (zip)",
        browser_download_url:
          "https://github.com/AstrBotDevs/AstrBot/archive/refs/tags/v4.1.0-beta.1.zip",
        size: 13_107_200,
      },
    ],
    html_url:
      "https://github.com/AstrBotDevs/AstrBot/releases/tag/v4.1.0-beta.1",
    body: "## 预览版本\n\n- 新增插件市场筛选\n- 修复 WebUI 在窄窗口下的布局问题",
  },
  {
    tag_name: "v4.0.0",
    name: "AstrBot v4.0.0",
    published_at: "2026-10-20T12:00:00Z",
    prerelease: false,
    assets: [
      {
        name: "AstrBot-v4.0.0.zip",
        browser_download_url:
          "https://github.com/AstrBotDevs/AstrBot/releases/download/v4.0.0/AstrBot-v4.0.0.zip",
        size: 18_735_104,
      },
      {
        name: "Source code (zip)",
        browser_download_url:
          "https://github.com/AstrBotDevs/AstrBot/archive/refs/tags/v4.0.0.zip",
        size: 12_582_912,
      },
    ],
    html_url: "https://github.com/AstrBotDevs/AstrBot/releases/tag/v4.0.0",
    body: "## 更新内容\n\n- 全新的实例管理接口\n- 支持多账号配置\n- 优化消息处理性能",
  },
  {
    tag_name: "v3.5.22",
    name: "AstrBot v3.5.22",
    published_at: "2026-09-18T03:30:00Z",
    prerelease: false,
    assets: [
      {
        name: "AstrBot-v3.5.22.zip",
        browser_download_url:
          "https://github.com/AstrBotDevs/AstrBot/releases/download/v3.5.22/AstrBot-v3.5.22.zip",
        size: 17_825_792,
      },
      {
        name: "Source code (zip)",
        browser_download_url:
          "https://github.com/AstrBotDevs/AstrBot/archive/refs/tags/v3.5.22.zip",
        size: 11_927_552,
      },
    ],
    html_url: "https://github.com/AstrBotDevs/AstrBot/releases/tag/v3.5.22",
    body: "## 修复\n\n- 修复部分情况下插件加载失败的问题",
  },
];

export const launcherReleaseNotes =
  "## 启动器更新\n\n- 新增实例概览页\n- 优化备份列表加载速度\n- 修复窗口还原状态异常";
