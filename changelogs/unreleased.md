## English

### Added

- Added page transitions and a reduce-motion setting (follow system, on, off).
- Added a tray panel for browsing instance status with quick start, stop, and restart actions, plus a setting to switch to a native tray menu that groups instances and their actions (panel is the default).
- Added a startup launcher update check that shows a dialog with release notes, letting you update now, remind later, or ignore this version.

### Changed

- The instance list no longer shows an update summary banner or per-instance status chips; versions render as plain text and the batch upgrade action only appears when there is something to upgrade.
- Refined i18n copy across all locales and added descriptions for configuration items in Settings.

### Fixed

- Fixed the problem that the prompt bar of the resource monitoring pop-up window was squeezed because the window was too small.
- Fixed the macOS build failing because of a `statvfs` integer width mismatch.
- Fixed the app crashing on startup on macOS because the traffic-light positioner touched a window without a superview.
- Fixed the macOS window missing its native traffic lights and showing Windows-style window controls.

## 中文

### 新增

- 新增页面切换过渡动画与减少动态设置（跟随系统、开启、关闭）。
- 新增托盘面板用于查看实例状态并快捷启动、停止和重启实例，并提供设置项切换到按实例分组的原生托盘菜单（默认使用面板）。
- 新增启动时检测启动器更新，发现新版本后弹出对话框显示发行版说明，可选择立即更新、稍后提醒或忽略本次版本。

### 变更

- 实例列表不再显示更新汇总横幅与实例级状态标签，版本号以纯文本呈现，批量升级入口仅在存在可升级实例时显示。
- 优化多语言文案，并补充“设置”中各配置项的说明文案。

### 修复

- 修复资源监控弹窗的提示条因为窗口过小被挤压的问题。
- 修复因 `statvfs` 整数宽度不匹配导致 macOS 构建失败的问题。
- 修复 macOS 上因交通灯按钮定位访问到没有父视图的窗口导致的启动崩溃。
- 修复 macOS 窗口缺少原生交通灯按钮、并显示 Windows 风格窗口控制按钮的问题。
