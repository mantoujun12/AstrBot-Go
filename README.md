<div align="center">

![AstrBot-Logo-Simplified](https://github.com/user-attachments/assets/36fb04e4-cc75-4454-bd8b-049d11aa86f9)

# AstrBot Launcher (Vue Version)

<!-- badge -->

<a href="https://github.com/mantoujun12/astrbot-launcher-vue/releases/latest"><img alt="GitHub Release" src="https://img.shields.io/github/v/release/mantoujun12/astrbot-launcher-vue?color=76bad9"></a>
<a href="https://tauri.app"><img alt="Tauri 2.12.4" src="https://img.shields.io/badge/Tauri-2.12.4-24C8D8?logo=tauri&logoColor=white"></a>
<a href="https://vuetifyjs.com"><img alt="Vuetify 4.3.1" src="https://img.shields.io/badge/Vuetify-4.3.1-1867C0?logo=vuetify"></a>
<a href="https://github.com/AstrBotDevs/astrbot-launcher"><img alt="Main - AstrBotDevs/astrbot-launcher" src="https://img.shields.io/badge/Main-AstrBotDevs%2Fastrbot--launcher-181717?logo=github"></a>
<a href="https://github.com/AstrBotDevs/AstrBot"><img alt="Upstream - AstrBotDevs/AstrBot" src="https://img.shields.io/badge/Upstream-AstrBotDevs%2FAstrBot-181717?logo=github"></a>

<!-- badge -->

Desktop launcher for AstrBot built with Tauri and Vuetify

Centralizes local instance management, version downloads, data backups, and Python runtime setup

English | [简体中文](./README_zh.md)

</div>

## Features

- Manage multiple AstrBot instances
- Download and switch AstrBot versions
- Back up and restore instance data
- Automatically configure Python / Node.js / uv runtime environments
- Browse instance status from a tray panel, with an optional native tray menu

## Platform Support

| Platform | Support | Architectures | Notes |
| --- | --- | --- | --- |
| Windows | 🟢 Supported | x64, ARM64 | Requires WebView2; Windows ARM64 runs via an x64-compatible Python runtime |
| Linux | 🟡 Partial | x64, ARM64 | Requires WebKitGTK 4.1 and other [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/#linux) |
| macOS | 🟡 Partial | Intel, Apple Silicon | Window adaptation only; building requires Xcode and its Command Line Tools |

Platform-specific capabilities are adapted through Rust conditional compilation and Tauri build configuration. The tray panel is available on Windows and macOS; Linux falls back to a fixed tray menu with only Show and Quit because appindicators do not receive click events. Cross-platform CI is not configured yet, so Linux and macOS packages should still be built and verified on target platforms or CI runners before release.

## Tech Stack

- Desktop framework: [Tauri 2](https://v2.tauri.app/)
- Frontend: [Vue 3](https://vuejs.org/), [Vuetify 4](https://vuetifyjs.com/), [TypeScript](https://www.typescriptlang.org/)
- Build and package manager: [Vite](https://vite.dev/), [pnpm](https://pnpm.io/)
- Native backend: [Rust](https://www.rust-lang.org/)

## Prerequisites

- Node.js and pnpm
- Rust toolchain
- System dependencies required by Tauri; see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

## Local Development

```
pnpm install
pnpm tauri dev
```

Run `pnpm dev` when debugging the frontend only. For a full production build:

```
pnpm build
pnpm tauri build
```

## Links

- Project and technical docs: [AstrBot](https://github.com/AstrBotDevs/AstrBot), [Tauri Docs](https://v2.tauri.app/), [Vue Docs](https://vuejs.org/guide/introduction.html), [Vuetify Docs](https://vuetifyjs.com/)
- Development and build: [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/), [Vite Guide](https://vite.dev/guide/), [Rust Installation Guide](https://www.rust-lang.org/tools/install), [pnpm Docs](https://pnpm.io/)
- Upstream projects: [AstrBot](https://github.com/AstrBotDevs/AstrBot), [astrbot-launcher](https://github.com/AstrBotDevs/astrbot-launcher)

## License and Attribution

This project is licensed under the [MIT License](./LICENSE).

The frontend is independently implemented by this project.

The backend in `src-tauri/` is modified from MIT-licensed code in [AstrBotDevs/astrbot-launcher](https://github.com/AstrBotDevs/astrbot-launcher), with original copyright held by Raven95676.

See [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md) for the full notices and license texts.
