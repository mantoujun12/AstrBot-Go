# Changelog

本目录记录面向用户的变更，并为自动发行工作流提供发布说明。

版本号遵循 [Semantic Versioning](https://semver.org/)，变更日志格式参考 [Keep a Changelog](https://keepachangelog.com/)。

日志需使用中英双语，优先展示英语；中文和英语需分隔开来，不要混合在一起。

## 待发布

每个面向用户的 PR 尽量在 `unreleased.md` 的“新增”“变更”“弃用”“移除”“修复”“安全”分节中追加一条。纯内部重构、CI 和文档改动可以省略。

没有的分节（空分节）就不要添加。

`unreleased.md` 只是工作草稿，不会直接作为 Release 说明。

## 发版

准备发版时创建一个版本更新 PR：

1. 将 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 的版本号改为发布版本。
2. 在 `src-tauri/` 下运行 `cargo check`，Cargo 会自动更新 `src-tauri/Cargo.lock` 中 `astrbot-launcher-vue` 的版本；提交前确认锁文件只包含预期的版本变化。
3. 将 `unreleased.md` 改名为 `v<版本号>.md`，例如 `v0.2.0.md`。版本文件的内容会直接作为 GitHub Release 说明。
4. 新建空的 `unreleased.md`，供下一轮开发使用。
5. 合入 `main`。工作流会校验版本号、版本文件和是否已有 GitHub Release，然后构建 Windows 安装包并创建 GitHub Release。若 tag 已存在但没有 Release，工作流会跳过并提示人工处理。

发版后继续开发，改动仍写入新的 `unreleased.md`，不需要立即改版本。
