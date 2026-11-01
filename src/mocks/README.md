# 前端 Mock 后端

基于 Tauri 官方 `@tauri-apps/api/mocks` 的 `mockIPC` 与 `mockWindows`，仅用于开发阶段。生产构建不会加载本目录。

## 启用规则

| 场景 | 行为 |
| --- | --- |
| `pnpm dev`（普通浏览器） | 自动启用 |
| `pnpm tauri dev` | 使用真实 Rust 后端 |
| `VITE_USE_MOCK=true pnpm tauri dev` | 强制启用 Mock |
| `pnpm build` | 不包含 Mock 代码 |

## 文件职责

- `fixtures.ts`：初始快照、版本列表与发布说明。
- `store.ts`：内存状态，命令处理后会修改这份数据。
- `handlers.ts`：Tauri 命令与事件的处理表。
- `index.ts`：安装 `mockIPC`、`mockWindows` 并模拟 IPC 延迟。

## 扩展命令

1. 在 `src/types/launcher.ts` 中补齐命令的参数与返回值类型。
2. 在 `handlers.ts` 的 `mockCommands` 中注册同名命令，参数名使用 camelCase。
3. 需要驱动界面刷新时，在处理函数里调用 `emitSnapshot()`。

命令未注册时会抛出 `Mock 未实现命令：xxx`，方便定位遗漏。

## 事件

`mockIPC` 已开启 `shouldMockEvents`，可以直接使用 `@tauri-apps/api/event` 的 `emit` 与 `listen`。当前模拟的事件有：

- `app-snapshot`：实例状态、配置、备份变化后推送。
- `log-entry`：启动、停止、安装版本、创建备份时推送模拟日志。
- `download-progress`：安装版本或组件时推送下载进度。
- `deploy-progress`：修复实例时推送部署进度。

插件 IPC 也在 `mockPluginCommands` 中模拟，包括窗口控制、`plugin:opener`、`plugin:dialog`、`plugin:app|version` 与 `plugin:autostart`。

## 模拟慢请求与失败

- 所有业务命令默认延迟 120ms，可在 `index.ts` 的 `MOCK_LATENCY_MS` 调整。
- 在任意处理函数中直接 `throw new Error("错误信息")`，即可测试前端的失败与错误提示。
