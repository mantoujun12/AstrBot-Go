## English

### Changed

- Separated the display of application name + version and repository link in "Settings → About". (Direct commit)

### Removed

- Removed pre-release compatibility and migration paths: the legacy launcher data-directory migration wizard, `data.redb` and TOML config/manifest import, the old `mainland_acceleration` preset conversion, and legacy task-history and Python directory migrations.

### Fixed

- Resolved all open code scanning alerts: annotated Windows-only and reserved system-proxy items to silence clippy dead-code warnings, removed a redundant clone in the backup module, and restricted GITHUB_TOKEN permissions in the coverage workflow.
- Fixed narrow-window layout and window chrome: instance table rows align their names and actions correctly, the home content no longer reserves unused space for the mobile menu at intermediate widths, the temporary sidebar hides its non-functional collapse button, and the sidebar and title bar now share a smooth, opaque background instead of showing the page through Mica.
- Guarded against duplicate submissions in the frontend: a backup being restored can no longer be deleted, download pause/resume and stop buttons ignore rapid double clicks, and quitting the launcher from the tray now uses the same click-again confirmation as stopping an instance.
- Surfaced frontend event-subscription and polling failures: the resource monitor now shows an error state with retry instead of frozen charts, and the log and task pages expose subscription/list failures with a retry action.

## 中文

### 变更

- 将 “设置 → 关于” 中，显示应用名称+版本与仓库跳转，进行分隔。（直接提交）

### 移除

- 移除面向预发布旧版本的兼容与迁移逻辑：旧版启动器数据目录迁移向导、`data.redb` 与 TOML 配置/清单导入、旧 `mainland_acceleration` 预设转换，以及旧任务记录与 Python 目录迁移。

### 修复

- 修复了全部 open 状态的代码扫描告警：为 Windows 专属与系统代理预留代码标注 dead_code、移除备份模块中的冗余 clone，并为 coverage 工作流限制 GITHUB_TOKEN 权限。
- 修复窄窗口下的布局与窗口效果：实例表格行的名称与操作区正确对齐，中等宽度下主内容不再为移动端菜单预留空白，临时侧边栏不再显示无效的伸缩按钮，侧边栏与标题栏统一为平滑过渡的不透明底色，不再透出页面内容。
- 修复前端重复提交问题：恢复中的备份不再允许删除，版本下载的暂停/继续与停止按钮忽略快速连点，托盘退出启动器改为与停止实例一致的二次点击确认。
- 前端事件订阅与轮询失败不再被静默吞掉：资源监控改为展示错误态并提供重试，日志页与任务页在订阅或列表失败时展示可重试的错误态。
