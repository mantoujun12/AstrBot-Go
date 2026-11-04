<div align="center">

# AstrBot Launcher (Vue Version)

<!-- badge -->

<a href="https://github.com/mantoujun12/astrbot-launcher-vue/releases/latest"><img alt="GitHub Release" src="https://img.shields.io/github/v/release/mantoujun12/astrbot-launcher-vue?color=76bad9"></a>
<a href="https://tauri.app"><img alt="Tauri 2.12.4" src="https://img.shields.io/badge/Tauri-2.12.4-24C8D8?logo=tauri&logoColor=white"></a>
<a href="https://vuetifyjs.com"><img alt="Vuetify 4.3.1" src="https://img.shields.io/badge/Vuetify-4.3.1-1867C0?logo=vuetify"></a>
<a href="https://github.com/AstrBotDevs/astrbot-launcher"><img alt="Static Badge" src="https://img.shields.io/badge/Main-AstrBotDevs%2Fastrbot--launcher-181717?logo=github"></a>
<a href="https://github.com/AstrBotDevs/AstrBot"><img alt="Upstream - AstrBotDevs/AstrBot" src="https://img.shields.io/badge/Upstream-AstrBotDevs%2FAstrBot-181717?logo=github"></a>

<!-- badge -->

基于 Tauri 2 与 Vuetify 4 构建的 AstrBot 桌面管理器

用于集中管理本地实例，并提供版本下载、数据备份和 Python 等运行环境配置。

</div>

## 功能

- 管理多个 AstrBot 实例
- 下载并切换 AstrBot 版本
- 备份与恢复实例数据
- 自动配置 Python / Node.js / uv 运行环境

## 平台支持

| 平台 | 支持情况 | 支持架构 | 说明 |
| --- | --- | --- | --- |
| Windows | 🟢支持 | x64、ARM64 | 需要 WebView2；Windows ARM64 通过 x64 Python 运行环境兼容 |
| Linux | ⚠️未测试 | x64、ARM64 | 需要 WebKitGTK 4.1 等 [Tauri 系统依赖](https://v2.tauri.app/start/prerequisites/#linux) |
| macOS | ⚠️未测试 | Intel、Apple Silicon | 构建时需要 Xcode 及其命令行工具 |

平台相关能力通过 Rust 条件编译和 Tauri 构建配置分别适配。仓库目前尚未配置跨平台 CI，Linux 和 macOS 安装包发布前仍应在对应平台或 CI runner 上完成构建与运行验证。

## 技术栈

- 桌面框架：[Tauri 2](https://v2.tauri.app/)
- 前端：[Vue 3](https://vuejs.org/)、[Vuetify 4](https://vuetifyjs.com/)、[TypeScript](https://www.typescriptlang.org/)
- 构建与包管理：[Vite](https://vite.dev/)、[pnpm](https://pnpm.io/)
- 原生后端：[Rust](https://www.rust-lang.org/)

## 开发环境

- Node.js 与 pnpm
- Rust 工具链
- Tauri 所需的系统依赖，参见 [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

## 本地开发

```
pnpm install
pnpm tauri dev
```

仅调试前端时可运行 `pnpm dev`。完整构建使用：

```
pnpm build
pnpm tauri build
```

## 相关链接

- 项目与技术文档：[AstrBot](https://github.com/AstrBotDevs/AstrBot)、[Tauri 文档](https://v2.tauri.app/)、[Vue 文档](https://vuejs.org/guide/introduction.html)、[Vuetify 文档](https://vuetifyjs.com/)
- 开发与构建：[Tauri 环境要求](https://v2.tauri.app/start/prerequisites/)、[Vite 文档](https://vite.dev/guide/)、[Rust 安装指南](https://www.rust-lang.org/tools/install)、[pnpm 文档](https://pnpm.io/)
- 上游项目：[AstrBot](https://github.com/AstrBotDevs/AstrBot)、[astrbot-launcher](https://github.com/AstrBotDevs/astrbot-launcher)

## 许可证与来源

本项目采用 [MIT License](./LICENSE)。

前端由本项目独立实现。

后端 `src-tauri/` 基于 [AstrBotDevs/astrbot-launcher](https://github.com/AstrBotDevs/astrbot-launcher) 的 MIT 许可代码修改，原始版权归 Raven95676 所有。相关声明和许可证全文见 [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)。
