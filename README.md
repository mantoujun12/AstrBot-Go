# AstrBot Launcher (Vue Version)

基于 Tauri 2 与 Vue 3 构建的 AstrBot 桌面管理器，用于集中管理本地实例，并提供版本下载、数据备份和 Python 运行环境配置。

## 功能

- 管理多个 AstrBot 实例
- 下载并切换 AstrBot 版本
- 备份与恢复实例数据
- 自动配置 Python 运行环境

## 技术栈

Tauri 2 · Vue 3 · TypeScript · Vuetify 4 · Rust

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

- [AstrBot](https://github.com/AstrBotDevs/AstrBot)
- [Vuetify 文档](https://vuetifyjs.com)
