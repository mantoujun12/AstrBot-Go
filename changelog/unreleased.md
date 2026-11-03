## English

### Added

- Added per-source network configuration for GitHub, PyPI, Node.js, npm, the Python runtime, and uv, each with a default source, preset mirrors, and a custom source.
- Added latency and download speed tests for network sources and the HTTP proxy. Click tests sources serially, Ctrl+click tests the selected source, and Shift+click tests sources in parallel.

### Changed

- Replaced the global mainland acceleration switch with per-source network configuration. Existing mainland acceleration settings are migrated to equivalent presets.

### Fixed

- Improved tooltip contrast in light and dark themes and separated the network speed test shortcut hint with a blank line.

### Removed

- Removed the mainland acceleration switch.

## 中文

### 新增

- 新增 GitHub、PyPI、Node.js、npm、Python 运行时和 uv 的逐项网络源配置，每项提供默认源、预设镜像源和自定义源。
- 新增网络源与 HTTP 代理的延迟和下载速度测试。点击串行测试，Ctrl+点击测试当前选中源，Shift+点击并行测试。

### 变更

- 使用逐项网络源配置替代全局大陆加速开关；已有大陆加速配置会自动迁移为对应预设。

### 修复

- 提升浅色与深色主题下提示框的对比度，并将网络测速提示的快捷键说明分行显示。

### 移除

- 移除大陆加速开关。
