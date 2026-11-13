# Changelog

## English

### Added

- Added a pluggable instance source framework: an "Instances" section in Settings with an enable/disable switch per source, the instance list now merges externally discovered instances (source badge, read-only, open-WebUI entry when available), and Docker-deployed, `uv tool`-deployed (`astrbot` CLI) and directory-scanned instances are discovered read-only with their WebUI addresses.

### Changed

- The project was renamed to AstrBot Go: the app identifier is now `com.github.mantoujun12.astrbot-go` and the launcher-internal config directory moved accordingly; move the contents of the old `com.github.mantoujun12.astrbot-launcher-vue` config directory to the new one to keep the data-directory override and related settings. The updater now downloads from the renamed repository's releases (https://github.com/mantoujun12/AstrBot-Go), and historical changelog links were updated to point at the renamed repository.
- Redesigned the external instance rows: a source icon with tooltip replaces the badge, each source shows its own meta line (Docker host and port, uv tool version and port, deployment path for directory scans), and the WebUI button opens the dashboard in an embedded launcher tab; opening in an external browser moved to a secondary action.
- The default data directory is now `~/.astrbot_go`; on first launch of this version, an existing `~/.astrbot_launcher` directory is renamed automatically (if that fails, for example while locked by another process, the old location is used for that launch and the migration is retried next time). Application config and manifest are now stored as `config.json` and `manifest.json` in the data directory instead of the `data.redb` database; existing `data.redb` records are migrated automatically on first load and the database file is kept as a backup.

## 中文

### 新增

- 新增可扩展的实例来源框架：设置中新增“实例”分区，可按来源开关发现；实例列表合并展示外部来源实例（带来源徽标、只读、可用时提供打开 WebUI 入口）；支持只读发现 Docker 部署、uv tool 部署（astrbot 命令行）与目录扫描发现的实例，可列出其 WebUI 地址。

### 调整

- 项目更名为 AstrBot Go：应用标识改为 `com.github.mantoujun12.astrbot-go`，启动器内部配置目录随之迁移；如需保留数据目录覆盖等设置，请将旧配置目录 `com.github.mantoujun12.astrbot-launcher-vue` 中的内容移动到新目录。更新器现在从更名后的仓库 Release 下载（https://github.com/mantoujun12/AstrBot-Go），历史更新日志中的仓库链接也已同步指向新地址。
- 重构外部实例行展示：来源徽标改为名称左侧图标，悬浮显示部署方式；按来源补充专属信息行（Docker 显示主机地址与端口、uv tool 显示版本与端口、目录扫描显示部署路径）；WebUI 按钮改为在启动器内嵌标签页打开，改为在浏览器中打开为次要操作。
- 默认数据目录改为 `~/.astrbot_go`；本版本首次启动时，若存在旧目录 `~/.astrbot_launcher` 会自动重命名迁移（若失败，例如目录被其他进程占用，则本次启动沿用旧目录，下次启动重试）。应用配置与清单改为存储在数据目录下的 `config.json` 与 `manifest.json`，不再使用 `data.redb` 数据库；首次加载时会自动迁移已有的 `data.redb` 记录，数据库文件保留作为备份。
