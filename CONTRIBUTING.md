# Contributing

This document is written in English and Simplified Chinese. English comes first, and the two language sections are kept separate.

[English](#english) | [中文](#中文)

## English

Thank you for contributing to `astrbot-go`. Please read this guide and our [Code of Conduct](./CODE_OF_CONDUCT.md) before opening an issue or pull request.

### Prerequisites

- Node.js and `pnpm` (use `pnpm` consistently; do not mix `npm` or `yarn`, and do not edit `pnpm-lock.yaml` manually)
- Rust toolchain (`cargo`, `rustfmt`, `clippy`)
- Platform dependencies required by [Tauri 2](https://v2.tauri.app/start/prerequisites/)

### Local Development

Install dependencies and start the desktop app in development mode:

```bash
pnpm install
pnpm tauri dev
```

Common commands:

```bash
pnpm dev             # Start the Vite frontend dev server only
pnpm build           # Run vue-tsc and build the frontend
pnpm preview         # Preview the built frontend
pnpm tauri dev       # Start the Tauri desktop app in development mode
pnpm tauri build     # Build the desktop bundle
```

Backend checks (run inside `src-tauri/`):

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo check
```

### Project Structure

- `src/`: Vue 3 frontend (`<script setup lang="ts">`, Vuetify 4, strict TypeScript). Wrap Tauri `invoke` calls in a service layer rather than placing process, download, or backup logic directly in `App.vue`.
- `src-tauri/src/`: Rust backend and Tauri command handlers. Keep filesystem, process, archive, and environment operations in Rust behind explicit Tauri commands.
- `src-tauri/capabilities/`: Tauri permission and capability definitions following least privilege.
- `changelogs/`: User-facing release notes and `unreleased.md` draft entries.

### Coding and Copy Conventions

- Keep TypeScript in strict mode and avoid `any`.
- Validate paths and guard against path traversal, Zip Slip, and unintended writes outside the instance root.
- Do not log tokens, passwords, full user paths, or sensitive configuration.
- Keep locale keys synchronized across `zh-CN`, `en-US`, `ru-RU`, and `ja-JP` whenever user-facing copy changes.
- Use full-width punctuation in Chinese copy (`，。：；？！（）“”`), ASCII punctuation in English copy and code (`, . : ; ( )`), native Russian typography (`«»`, `—`), and full-width punctuation in Japanese (`、。：「」`).

### Branch and Commit Conventions

Name branches after the change type:

- `feat/xxx`
- `fix/xxx`
- `refactor/xxx`
- `chore/xxx`
- `docs/xxx`

Write commit messages and pull request titles in English using Conventional Commits:

- `feat:` new feature
- `fix:` bug fix
- `refactor:` refactoring without external behavior changes
- `chore:` build, dependency, configuration, or maintenance work
- `docs:` documentation-only changes

Keep each commit and pull request focused on a single concern.

### Verification Before Submitting

- Frontend changes: run `pnpm build`.
- Rust or Tauri changes: run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo check` in `src-tauri/`.
- Desktop interaction, process, backup, or download changes: verify with `pnpm tauri dev` when possible and describe the manual test steps in the pull request.

### Changelog Rules

Follow [changelogs/README.md](./changelogs/README.md):

- Add user-facing changes to `changelogs/unreleased.md` in the same pull request. Internal refactors, CI changes, and documentation-only changes may skip it.
- Each pull request contributes at most one logical entry: one concise English bullet under `## English` and its matching Chinese bullet under `## 中文`, placed in the same section (`Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, or `Security`).
- Keep `changelogs/unreleased.md` unannotated during normal development; pull request links are added only in release pull requests.

### Pull Request Checklist

1. Create a scoped branch from `main`.
2. Complete the required frontend and Rust verification commands.
3. Synchronize all locale files if UI text changed.
4. Update `changelogs/unreleased.md` if the change is user-visible.
5. Explain the purpose, main changes, user-visible impact, verification performed, and any remaining risks or rollback notes in the pull request description. Include screenshots or recordings for UI changes.

---

## 中文

感谢为 `astrbot-go` 做出贡献。在提交 Issue 或 Pull Request 前，请先阅读本指南以及[行为准则](./CODE_OF_CONDUCT_zh.md)。

### 环境要求

- Node.js 与 `pnpm`（统一使用 `pnpm`，不要混用 `npm` 或 `yarn`，也不要手动编辑 `pnpm-lock.yaml`）
- Rust 工具链（`cargo`、`rustfmt`、`clippy`）
- [Tauri 2](https://v2.tauri.app/start/prerequisites/) 所需的平台系统依赖

### 本地开发

安装依赖并启动桌面开发模式：

```bash
pnpm install
pnpm tauri dev
```

常用命令：

```bash
pnpm dev             # 仅启动 Vite 前端开发服务器
pnpm build           # 运行 vue-tsc 类型检查并构建前端
pnpm preview         # 预览前端构建产物
pnpm tauri dev       # 启动 Tauri 桌面端开发应用
pnpm tauri build     # 构建桌面安装包
```

后端检查（在 `src-tauri/` 目录下运行）：

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo check
```

### 目录职责

- `src/`：Vue 3 前端代码（`<script setup lang="ts">`、Vuetify 4、TypeScript 严格模式）。Tauri `invoke` 调用应封装在服务层，不要把下载、进程管理、备份恢复等逻辑直接塞进 `App.vue`。
- `src-tauri/src/`：Rust 后端与 Tauri 命令注册。文件系统、进程、压缩包解压和运行环境配置统一在 Rust 层实现，并通过受控的 Tauri 命令暴露给前端。
- `src-tauri/capabilities/`：遵循最小权限原则的 Tauri 权限与能力配置。
- `changelogs/`：面向用户的更新日志与 `unreleased.md` 待发布草稿。

### 代码与文案规范

- 保持 TypeScript 严格模式，不引入 `any`。
- 校验并规范化实例路径、备份路径与解压目标路径，防止路径穿越、Zip Slip 以及越出实例根目录的写入。
- 禁止在日志或控制台中输出 Token、密码、完整用户路径和敏感配置。
- 新增、删除或修改界面文案时，必须同步更新 `zh-CN`、`en-US`、`ru-RU`、`ja-JP` 四个语言文件。
- 中文文案使用全角标点（`，。：；？！（）“”`），英文文案与代码使用 ASCII 标点（`, . : ; ( )`），俄文使用原生排版标点（`«»`、`—`），日文使用全角标点（`、。：「」`）。

### 分支与提交规范

分支名按改动类型命名：

- `feat/xxx`
- `fix/xxx`
- `refactor/xxx`
- `chore/xxx`
- `docs/xxx`

提交信息与 PR 标题统一使用英文 Conventional Commits：

- `feat:` 新功能
- `fix:` 问题修复
- `refactor:` 不改变外部行为的重构
- `chore:` 构建、依赖、配置或维护工作
- `docs:` 仅文档改动

每个提交和 PR 只聚焦单一职责，不要把无关格式化、依赖升级、文案调整和功能开发混在一起。

### 提交前验证

- 前端改动：至少运行 `pnpm build`。
- Rust 或 Tauri 配置改动：在 `src-tauri/` 下运行 `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings` 和 `cargo check`。
- 涉及桌面交互、进程行为、备份恢复或下载流程的改动：尽可能通过 `pnpm tauri dev` 完成桌面联调，并在 PR 中说明手动验证步骤。

### 变更日志规范

遵循 [changelogs/README.md](./changelogs/README.md)：

- 面向用户的改动需在同一个 PR 中写入 `changelogs/unreleased.md`；纯内部重构、CI 和仅文档改动可省略。
- 每个 PR 只写入一条逻辑记录：在 `## English` 与 `## 中文` 下的对应分节（`Added`/`新增`、`Changed`/`变更`、`Deprecated`/`弃用`、`Removed`/`移除`、`Fixed`/`修复`、`Security`/`安全`）各写一条简短记录，英文在前、中文在后，分节分开维护。
- 日常开发期间不在 `changelogs/unreleased.md` 中标注 PR 链接，PR 链接仅在发版 PR 中统一补齐。

### Pull Request 检查清单

1. 从 `main` 创建职责单一的分支。
2. 完成对应的前端与 Rust 构建和静态检查。
3. 若修改了界面文案，确认四语种词条已同步。
4. 若包含用户可感知的变化，确认已更新 `changelogs/unreleased.md`。
5. 在 PR 描述中写明改动目的、主要变更、用户可见影响、实际执行过的验证命令以及风险或回滚说明；涉及 UI 变更时附上截图或录屏。
