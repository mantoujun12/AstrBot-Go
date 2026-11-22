# Changelog

## English

This directory records user-facing changes and provides release notes for the automated release workflow.

Versions follow [Semantic Versioning](https://semver.org/), and the changelog format is based on [Keep a Changelog](https://keepachangelog.com/).

### Unreleased

Each user-facing PR should produce one logical entry. In `unreleased.md`, choose the single most appropriate section—Added, Changed, Deprecated, Removed, Fixed, or Security—and write one English entry and its corresponding Chinese entry. Do not split a change into multiple entries or sections just because it covers multiple aspects. Purely internal refactoring, CI, and documentation changes may be omitted.

Section headings use each language's own words: under `## English`, keep the Keep a Changelog originals (`Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, `Security`); under `## 中文`, use `新增`, `变更`, `弃用`, `移除`, `修复`, `安全`.

Keep entries brief and describe only user-visible changes. Do not add empty sections.

`unreleased.md` is a working draft and is not used directly as release notes. Leave entries untagged during development; add PR links only in the release PR.

### Releasing

When preparing a release, create a version-bump PR:

1. Set the release version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
2. Run `cargo check` in `src-tauri/`. Cargo will update the `astrbot-go` version in `src-tauri/Cargo.lock`; before committing, confirm that the lockfile contains only the expected version change.
3. Rename `unreleased.md` to `v<version>.md`, for example `v0.2.0.md`. The version file will form the first part of the GitHub Release notes. The workflow then appends GitHub's automatically generated changelog, including contributors and the Full Changelog link, separated by a divider.
4. Add the full Markdown link to the source PR to every entry in the version file. Use parentheses for English entries, for example `([#8](https://github.com/mantoujun12/AstrBot-Go/pull/8))`, and full-width parentheses for Chinese entries, for example `（[#8](https://github.com/mantoujun12/AstrBot-Go/pull/8)）`. The English and Chinese entries for the same logical change must link to the same PR.
5. Create a new, empty `unreleased.md` for the next development cycle.
6. Before merging, rebase onto the latest `main` and make sure any concurrently added entries are moved back to the new `unreleased.md`; `v<version>.md` should contain only changes included in this release.
7. Merge into `main`. The workflow validates the version, version file, and whether a GitHub Release already exists, then builds the Windows installer and creates the GitHub Release. If the tag exists but no Release does, the workflow skips the release and prompts for manual handling.

Before releasing, ensure every entry is brief and includes its source PR number. Use `git log --follow -- changelogs/unreleased.md` to trace the source PR for an entry.

Do not add PR links to historical version files (for example, `v0.1.1.md`).

After releasing, continue development by adding changes to the new `unreleased.md`; there is no need to bump the version immediately.

---

## 中文

本目录记录面向用户的变更，并为自动发行工作流提供发布说明。

版本号遵循 [Semantic Versioning](https://semver.org/)，变更日志格式参考 [Keep a Changelog](https://keepachangelog.com/)。

### 待发布

每个面向用户的 PR 只产生一条逻辑条目：在 `unreleased.md` 的“新增”“变更”“弃用”“移除”“修复”“安全”中选择最合适的一个分节，写入一条英文条目和对应的中文条目。不要因为改动涉及多个方面就拆成多条，也不要分散到多个分节。纯内部重构、CI 和文档改动可以省略。

分节标题两种语言各用各的：`## English` 分节下使用 Keep a Changelog 原文的英文标题（`Added`、`Changed`、`Deprecated`、`Removed`、`Fixed`、`Security`）；`## 中文` 分节下使用中文标题（`新增`、`变更`、`弃用`、`移除`、`修复`、`安全`），不要沿用英文标题。

条目应简短，只描述用户可感知的变化。没有的分节（空分节）就不要添加。

`unreleased.md` 只是工作草稿，不会直接作为 Release 说明。开发期间条目保持不标注，PR 链接只在发版 PR 中补齐。

### 发版

准备发版时创建一个版本更新 PR：

1. 将 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 的版本号改为发布版本。
2. 在 `src-tauri/` 下运行 `cargo check`，Cargo 会自动更新 `src-tauri/Cargo.lock` 中 `astrbot-go` 的版本；提交前确认锁文件只包含预期的版本变化。
3. 将 `unreleased.md` 改名为 `v<版本号>.md`，例如 `v0.2.0.md`。版本文件的内容会作为 GitHub Release 说明的第一部分，工作流随后附上 GitHub 自动生成的变更列表（包括贡献者和 Full Changelog 链接），两者以分隔线隔开。
4. 为版本文件中的每条记录补上来源 PR 的完整 Markdown 链接：英文条目使用半角括号，例如 `([#8](https://github.com/mantoujun12/AstrBot-Go/pull/8))`；中文条目使用全角括号，例如 `（[#8](https://github.com/mantoujun12/AstrBot-Go/pull/8)）`。同一逻辑条目的英文和中文必须链接到同一个 PR。
5. 新建空的 `unreleased.md`，供下一轮开发使用。
6. 合并前先变基到最新 `main`，确认 `unreleased.md` 中并发新增的条目已移回新的 `unreleased.md`；`v<版本号>.md` 只保留本次发布的内容。
7. 合入 `main`。工作流会校验版本号、版本文件和是否已有 GitHub Release，然后构建 Windows 安装包并创建 GitHub Release。若 tag 已存在但没有 Release，工作流会跳过并提示人工处理。

发版前确认每个条目都简短且已标注来源 PR 编号；可通过 `git log --follow -- changelogs/unreleased.md` 追溯条目的来源 PR。

历史版本文件（例如 `v0.1.1.md`）不补写 PR 链接。

发版后继续开发，改动仍写入新的 `unreleased.md`，不需要立即改版本。
