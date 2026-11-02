# AGENTS.md

## Collaboration

- Communicate in Simplified Chinese by default. Stay professional and upbeat, and use emojis and honorifics sparingly.
- Lead with the conclusion, then provide the key evidence and any necessary follow-up actions. Avoid making users reread the process.
- When the user is only asking a question, do not call tools. If accessing local files or running commands is genuinely necessary, ask for the user's consent first.
- Apply the same principle to Git operations. Commit, push, create or modify a PR, or rewrite history only after the user explicitly requests it. For irreversible operations, explain the impact and obtain confirmation first.
- Read the relevant code and configuration before making changes, and follow the existing structure. Do not rewrite unrelated modules for a narrowly scoped request.
- The workspace may already contain changes from the user or other collaborators. Preserve them and build on top of them. Never revert, overwrite, or delete them without authorization.
- If an ambiguity would materially affect the result, state the assumption or ask the user first. When a safe validation is possible within the current scope, prefer performing it yourself.

## Project Overview

`astrbot-launcher-vue` is a desktop application built with Tauri 2, Vue 3, and pnpm. It provides a fast way to manage AstrBot instances, including version downloads, data backups, and automated Python runtime configuration.

Reference links:

- [AstrBot upstream repository](https://github.com/AstrBotDevs/AstrBot)
- [Vuetify documentation](https://vuetifyjs.com)

## Tech Stack

- Tauri 2
- Vue 3
- TypeScript
- Vite
- pnpm
- Vuetify 4
- Chart.js (MIT)
- Rust

## Directory Responsibilities

- `src/`: Vue frontend code.
- `src-tauri/src/`: Tauri backend logic and command registration. `lib.rs` handles application assembly, while `main.rs` contains only the startup layer.
- `src-tauri/capabilities/`: Tauri capability and permission configuration.
- `src-tauri/tauri.conf.json`: Window, build, bundle, and security configuration.
- `public/`: Static assets that do not need to pass through the module graph.

When extending functionality, separate pages, components, services, and state logic by responsibility. Do not put downloads, process management, backup and restore logic, or complex `invoke` calls into `App.vue`.

## Development Commands

Use pnpm consistently. Do not mix npm or yarn, and do not edit `pnpm-lock.yaml` manually.

```
pnpm dev             # Start the Vite frontend development server
pnpm build           # Type-check and build the frontend
pnpm preview         # Preview the frontend build
pnpm tauri dev       # Start the Tauri desktop development app
pnpm tauri build     # Build the desktop application package
```

Run the following commands from `src-tauri/` as needed:

```
cargo check
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

`pnpm dev` cannot validate Tauri commands, permissions, or native capabilities. When desktop integration is involved, use `pnpm tauri dev` or the appropriate Rust checks.

## Frontend Conventions

- Use the Vue 3 Composition API and prefer `<script setup lang="ts">`.
- Keep TypeScript in strict mode and do not introduce `any`. Validate external data before converting it into explicit domain types.
- Wrap Tauri `invoke` calls in a service layer. Pages and components should depend only on clear business methods, parameter types, and return types.
- Async operations must handle loading, success, empty, and failure states. Prevent duplicate submissions, race conditions, and uncaught exceptions.
- Use Vuetify 4 components and its theme system for new interfaces. Reuse existing design tokens and interaction patterns, and do not casually introduce a second UI framework.
- Ensure keyboard access, visible focus states, readable contrast, and meaningful labels. Critical actions must remain reachable when the desktop window is resized.
- Do not output sensitive paths, tokens, user configuration, or complete command arguments to frontend logs or the console.
- **Do not add unnecessary literary copy or sentences to the WebUI.**
- **Minimize explanatory text, especially in UI titles.**

## Tauri and Rust Conventions

- Implement filesystem, process, download, archive extraction, environment variable, and privileged system operations in Rust. Expose them to the frontend through controlled Tauri commands; do not grant the frontend arbitrary shell or path capabilities.
- Tauri commands must have stable parameter and return types and be registered in `invoke_handler`. Keep field naming, nullability, and error shapes consistent between the frontend and backend.
- Rust errors should include enough diagnostic context and expose a predictable error contract. The frontend must translate them into user-friendly messages rather than swallowing failures.
- Do not perform long-running blocking work inside async commands. Use a blocking thread pool, background tasks, or streamed events for downloads, extraction, and installation, and provide cancellable or at least observable progress.
- Pass external process arguments as arrays. Never concatenate user input into a shell command. Explicitly handle the working directory, environment variables, and exit codes.
- Follow the principle of least privilege when adding plugins or capabilities. Grant only the permissions required by the current feature, and justify them in the same change.
- Handle platform differences in paths, file permissions, and process behavior explicitly. Avoid hardcoding Windows path separators in shared logic.

## Data and Security

- Deletion, overwrite, restore, rollback, and process termination are high-risk operations. The frontend must require explicit confirmation, and the backend must validate the target again rather than relying solely on UI safeguards.
- Canonicalize and validate instance paths, backup destinations, and extraction paths. Do not modify content outside the instance root, and specifically guard against path traversal and Zip Slip.
- Download version files into a staging directory. Validate their source, size, or hash before an atomic replacement, and preserve recoverable previous data if the operation fails.
- Backup and restore operations must retain version, timestamp, instance information, and a manifest. Prefer temporary files and atomic renames when writing critical configuration.
- Redact tokens, passwords, full user paths, and private configuration from logs. Diagnostics should identify problems without exposing sensitive content.
- When changing AstrBot data formats, version compatibility, or runtime environments, account for legacy instances, interrupted operations, and repeated execution.

## Copy and Internationalization

- Chinese copy must use full-width punctuation: `，。：；？！（）“”`. Do not use an ASCII comma `,` instead of `，` or an ASCII colon `:` instead of `：` at Chinese sentence boundaries.
- English copy, code, and `en-US` locale files must use ASCII punctuation: `, . : ; ( )`.
- Only adjust punctuation in comments, docstrings, and string literals. Punctuation that is part of code syntax must not be changed.
- Keep locale keys synchronized across Chinese and English when adding, removing, or modifying entries. Do not leave visible copy that exists in only one language.
- Do not mechanically replace punctuation in URLs, regular expressions, version numbers, format placeholders, or code snippets.

## Verification Requirements

- For frontend changes, run at least `pnpm build` to confirm that `vue-tsc` and the Vite build pass.
- For Rust or Tauri configuration changes, run at least `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo check`.
- For UI interaction, Tauri commands, permissions, filesystem operations, or process behavior, document the relevant manual verification. Run `pnpm tauri dev` for desktop integration testing when possible.
- Add unit tests for new critical pure logic, prioritizing path validation, version comparison, backup manifests, error conversion, and repeated execution.
- If the repository has no configured frontend lint or test script, do not invent commands or claim that they were run.
- Report only the commands that were actually run and their results during verification. Clearly disclose checks that were not run, platform behavior that could not be reproduced, and any remaining risks.

## Commit Conventions

Use Conventional Commits with English messages, and keep each commit focused on a single responsibility.

Allowed types:

- `feat:` New feature
- `fix:` Bug fix
- `refactor:` Refactoring that does not change external behavior
- `chore:` Build, dependency, configuration, or maintenance work
- `docs:` Documentation-only changes

Use branch names based on the change type: `feat/xxx`, `fix/xxx`, `refactor/xxx`, `chore/xxx`, or `docs/xxx`.

Each commit and PR must address one concern. Do not mix unrelated formatting, dependency upgrades, copy fixes, and feature changes. Unless the user explicitly requests it, do not run commit, amend, rebase, force push, or create a PR.

## Pull Requests

- Start the title with a Conventional Commit type: `feat:`, `fix:`, `refactor:`, `chore:`, or `docs:`.
- Explain the purpose, main changes, user-visible impact, and verification actually performed in the description.
- Include screenshots or recordings for UI changes. Document risks and rollback procedures for data, permission, download, backup, and process operations.
- Do not conceal incomplete work, untested platforms, or known issues. Keep the PR scoped and independently reviewable.
