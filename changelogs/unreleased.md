## English

### Changed

- Separated the display of application name + version and repository link in "Settings → About". (Direct commit)

### Fixed

- Resolved all open code scanning alerts: annotated Windows-only and reserved system-proxy items to silence clippy dead-code warnings, removed a redundant clone in the backup module, and restricted GITHUB_TOKEN permissions in the coverage workflow.

## 中文

### 变更

- 将 “设置 → 关于” 中，显示应用名称+版本与仓库跳转，进行分隔。（直接提交）

### 修复

- 修复了全部 open 状态的代码扫描告警：为 Windows 专属与系统代理预留代码标注 dead_code、移除备份模块中的冗余 clone，并为 coverage 工作流限制 GITHUB_TOKEN 权限。