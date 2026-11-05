## English

### Fixed

- Fixed the problem that the prompt bar of the resource monitoring pop-up window was squeezed because the window was too small.
- Fixed the macOS build failing because of a `statvfs` integer width mismatch.
- Fixed the app crashing on startup on macOS because the traffic-light positioner touched a window without a superview.
- Fixed the macOS window missing its native traffic lights and showing Windows-style window controls.

## 中文

### 修复

- 修复资源监控弹窗的提示条因为窗口过小被挤压的问题。
- 修复因 `statvfs` 整数宽度不匹配导致 macOS 构建失败的问题。
- 修复 macOS 上因交通灯按钮定位访问到没有父视图的窗口导致的启动崩溃。
- 修复 macOS 窗口缺少原生交通灯按钮、并显示 Windows 风格窗口控制按钮的问题。
