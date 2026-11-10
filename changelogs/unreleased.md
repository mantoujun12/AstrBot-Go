## English

### Added

- Support dragging instance tabs on the title bar or moving the active tab with Ctrl+Shift+PageUp/PageDown; tabs now slide to their new position while reordering, and the home logo aligns with the collapsed sidebar rail.
- Redesigned the instance list: the state column now shows runtime duration (refreshed every 30s), a warning icon reveals the last crash exit reason and auto-restart attempts, merged version/port into the second line of the instance name, and replaced text action buttons with icon buttons plus a quick logs shortcut.

### Changed

- **Important:** The launcher-internal config directory was renamed from `com.github.raven95676.astrbot-launcher` to `com.github.mantoujun12.astrbot-launcher-vue` to match the application identifier. A custom data directory override (if previously set in the old directory) is no longer read, and the launcher falls back to the default `~/.astrbot_launcher`; move the old `com.github.raven95676.astrbot-launcher\.data-dir.json` to the new directory to restore it.

### Fixed

- Auto-restart behavior now reports real data in the instance list: the last crash exit reason stays visible when auto-restart is disabled, the restart line shows the number of auto-restarts actually launched instead of the raw unexpected-exit count, and the retry limit displays at least 1. Stopped instances regained a resource-monitor entry in the row menu.

## 中文

### 新增

- 标题栏实例标签支持拖拽排序（拖动时标签平滑滑入新位置），或按 Ctrl+Shift+PageUp/PageDown 移动当前标签；主页logo与折叠侧栏轨道对齐。
- 实例列表改版：状态列显示运行时长（每 30 秒刷新），异常退出的警告图标悬浮可查看上次退出原因与自动重启次数；版本与端口并入实例名称次行；文字按钮改为图标按钮，并新增日志快捷入口。

### 变更

- **重要：** 启动器内部配置目录名从 `com.github.raven95676.astrbot-launcher` 更名为 `com.github.mantoujun12.astrbot-launcher-vue`，与应用标识保持一致。此前在旧目录中设置的自定义数据目录覆盖项将不再被读取，启动器会回退到默认的 `~/.astrbot_launcher`；如需恢复，请将旧目录 `com.github.raven95676.astrbot-launcher\.data-dir.json` 移至新目录。

### 修复

- 实例列表修复自动重启相关展示：自动重启关闭时仍保留并显示上次崩溃退出原因；重启行现在显示实际已执行的自动重启次数，而非原始意外退出计数；重试上限展示统一不小于 1；停止的实例在行内菜单重新获得资源监控入口。
