## English

### Changed

- Moved large file IO and blocking network probes (version archive extraction, instance directory cleanup, lock checks, and port probing) off the async runtime onto the blocking thread pool.
- Separated the display of application name + version and repository link in "Settings → About". (Direct commit)

### Removed

- Removed pre-release compatibility and migration paths: the legacy launcher data-directory migration wizard, `data.redb` and TOML config/manifest import, the old `mainland_acceleration` preset conversion, and legacy task-history and Python directory migrations.

### Fixed

- Resolved all open code scanning alerts: annotated Windows-only and reserved system-proxy items to silence clippy dead-code warnings, removed a redundant clone in the backup module, and restricted GITHUB_TOKEN permissions in the coverage workflow.
- Fixed narrow-window layout and window chrome: instance table rows align their names and actions correctly, the home content no longer reserves unused space for the mobile menu at intermediate widths, the temporary sidebar hides its non-functional collapse button, and the sidebar and title bar now share a smooth, opaque background instead of showing the page through Mica.

## 中文

### 变更

- 将大文件 IO 与阻塞网络探测（版本压缩包解压、实例目录清理、锁文件检查、端口探测）移出异步运行时，改为在阻塞线程池中执行。
- 将 “设置 → 关于” 中，显示应用名称+版本与仓库跳转，进行分隔。（直接提交）

### 移除

- 移除面向预发布旧版本的兼容与迁移逻辑：旧版启动器数据目录迁移向导、`data.redb` 与 TOML 配置/清单导入、旧 `mainland_acceleration` 预设转换，以及旧任务记录与 Python 目录迁移。

### 修复

- 修复了全部 open 状态的代码扫描告警：为 Windows 专属与系统代理预留代码标注 dead_code、移除备份模块中的冗余 clone，并为 coverage 工作流限制 GITHUB_TOKEN 权限。
- 修复窄窗口下的布局与窗口效果：实例表格行的名称与操作区正确对齐，中等宽度下主内容不再为移动端菜单预留空白，临时侧边栏不再显示无效的伸缩按钮，侧边栏与标题栏统一为平滑过渡的不透明底色，不再透出页面内容。