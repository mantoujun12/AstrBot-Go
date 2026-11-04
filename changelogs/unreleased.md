## English

### Added

- Added per-instance CPU, memory, and disk history charts with configurable threshold alerts.
- Auto-restart instances that exit unexpectedly, with exponential backoff, a configurable retry limit, and crash notifications.
- Added an instance update center with stable release checks, update badges, single-instance upgrades, and batch upgrades, while respecting the global and per-instance update toggles.
- Launcher updates can now be checked, downloaded, and installed from Settings without manually downloading a release.
- Added a unified task center for downloads, deployments, backups, and repairs, with persistent records, live progress, and cancellable downloads with pause and resume support.

### Fixed

- Fixed manifests written by earlier builds being reset, which made existing instances and installed versions disappear after upgrading.

## 中文

### 新增

- 新增实例 CPU、内存和磁盘历史趋势图，并支持配置阈值告警。
- 异常退出的实例会按次数与时间间隔退避自动重启，支持配置最大重试次数与崩溃通知。
- 新增实例更新中心，支持检查最新稳定版、显示更新徽标、单实例升级和全部升级，并遵循全局与单实例更新开关。
- 现在可以在设置中检查、下载并安装启动器更新，无需手动下载发布安装包。
- 新增统一任务中心，集中展示下载、部署、备份和修复任务，并支持持久化记录、实时进度，下载任务可取消、暂停与继续。

### 修复

- 修复旧版本写入的清单被重置的问题，升级后已有实例与已安装版本不再丢失。
