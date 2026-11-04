## English

### Added

- Added per-source network configuration for GitHub, PyPI, Node.js, npm, the Python runtime, and uv, each with a default source, preset mirrors, and a custom source.
- Added latency and download speed tests for network sources and the HTTP proxy. Click tests sources serially, Ctrl+click tests the selected source, and Shift+click tests sources in parallel.
- Added an opt-in setting that allows connectivity probes to follow DNS redirects to local or private addresses. Literal local addresses and local host names remain blocked.

### Changed

- Replaced the global mainland acceleration switch with per-source network configuration. Existing mainland acceleration settings are migrated to equivalent presets.

### Fixed

- Improved tooltip contrast in light and dark themes and separated the network speed test shortcut hint with a blank line.
- Restricted connectivity probes to public HTTP(S) destinations and revalidated every redirect target.
- Prevented legacy migration from overwriting newer per-source network settings.
- Made Node.js, npm, and Python speed tests download real archives instead of metadata.
- Kept newly edited source selections and proxy test results accurate while a save or test is pending.
- Removed the white flash when the main window opens at startup by waiting for the first frontend paint, matching the window background to the active theme, and deferring tray or second-instance activation until the UI is ready.

### Removed

- Removed the mainland acceleration switch.

## 中文

### 新增

- 新增 GitHub、PyPI、Node.js、npm、Python 运行时和 uv 的逐项网络源配置，每项提供默认源、预设镜像源和自定义源。
- 新增网络源与 HTTP 代理的延迟和下载速度测试。点击串行测试，Ctrl+点击测试当前选中源，Shift+点击并行测试。
- 新增“允许本地 DNS 重定向探测”选项，供连通性测试跟随解析到本地或私有地址的域名。直连本地地址和本地主机名仍会被拒绝。

### 变更

- 使用逐项网络源配置替代全局大陆加速开关；已有大陆加速配置会自动迁移为对应预设。

### 修复

- 提升浅色与深色主题下提示框的对比度，并将网络测速提示的快捷键说明分行显示。
- 将连通性探测限制为公网 HTTP(S) 目标，并对每次重定向重新校验目标地址。
- 避免旧版大陆加速迁移覆盖较新的逐项网络源设置。
- 将 Node.js、npm 和 Python 测速改为下载真实归档，不再测量元数据。
- 保存或测试进行期间，保留用户新编辑的源选择和代理测试结果。
- 修复启动时主窗口短暂白屏的问题：首屏渲染完成后再显示窗口，让窗口底色跟随当前主题，并让托盘或重复启动触发的显示同样等待界面就绪。

### 移除

- 移除大陆加速开关。
