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

`astrbot-go` is a desktop application built with Tauri 2, Vue 3, and pnpm. It provides a fast way to manage AstrBot instances, including version downloads, data backups, and automated Python runtime configuration.

Reference links:

- [AstrBot upstream repository](https://github.com/AstrBotDevs/AstrBot)
- [Vuetify documentation](https://vuetifyjs.com)

Repository documentation layout:

- `README.md` (English) and `README_zh.md` (Simplified Chinese): keep both files synchronized when features, platform support, tech stack, commands, links, or licensing details change.
- `CONTRIBUTING.md`: bilingual contribution guide in a single file; English comes first (`## English`), followed by Simplified Chinese (`## 中文`), separated cleanly rather than mixed line-by-line.
- `CODE_OF_CONDUCT.md` (English) and `CODE_OF_CONDUCT_zh.md` (Simplified Chinese): community code of conduct in separate language files.

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

### Dependency and Registry Notes

- The default registry on this machine (`~/.npmrc`) is the npmmirror mirror, which can lag behind npm by days for fresh releases. When `pnpm install` fails with `ERR_PNPM_MINIMUM_RELEASE_AGE_VIOLATION` ("version not present in registry manifest") or a 404 tarball, check the version on `registry.npmjs.org` first; if it is published there, install with `--registry=https://registry.npmjs.org/`.
- Never downgrade a dependency or hand-edit `pnpm-lock.yaml` to work around mirror lag. The version is usually fine; the mirror is behind.
- A failed pre-run install can corrupt `node_modules` (for example, prune `.bin`) even though the lockfile is fine. Restore it with an official-registry install before trusting any subsequent build result.

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

### Vuetify Control Conventions

Shared control styling lives in `src/styles/controls.css` and `src/scss/components/`. Do not patch the same problem locally in one component when it applies to every instance of a control.

- `v-switch` must set `inset`, `color="primary"`, and `hide-details`. The active track is painted by the `bg-primary` class added by `color="primary"`; omitting `color` leaves the neutral `surface-variant` track and makes the enabled state look disabled. Bind `:loading` and `:disabled` to the saving state. Use `density="compact"` in settings rows, filters, and toolbars; in forms and dialogs keep the density consistent with adjacent fields.
- Standard option groups (`v-radio`, `v-checkbox`) set `color="primary"` and `density="compact"` on each item. `v-radio-group` sets `hide-details`, plus `inline` when the options fit on one line.
- Settings, filter, and toolbar inputs (`v-text-field`, `v-select`, `v-textarea`, `v-color-input`) use `variant="outlined"`, `density="compact"`, and `hide-details`. Form fields in dialogs keep `variant="outlined"` and may show validation messages instead of `hide-details`. Disable the control while its value is saving.
- `v-btn` variants express hierarchy:
  - one primary action per view: `color="primary" variant="flat"`;
  - secondary actions: `color="primary" variant="tonal"`;
  - toolbar, row, and link-like actions: `variant="text"` (add `color="primary"` when emphasized);
  - neutral secondary actions: `variant="outlined"`;
  - destructive actions: `color="error"`; restart, install, and rollback actions: `color="warning"`.
  Use `size="small"` in tables, dialogs, and setting rows. Give icon-only buttons an `aria-label` and a tooltip.
- `v-btn-toggle` uses `mandatory`, `divided`, `density="compact"`, `variant="outlined"`, and `color="primary"`, with `size="small"` buttons inside.
- `v-chip` uses `size="x-small"` or `size="small"` with `variant="tonal"` and a semantic color; add `label` for status text. `v-alert` uses `variant="tonal"`, `density="comfortable"`, and the matching semantic `type`. `v-progress-linear` uses `color="primary"`, `height="5"`, and `rounded`.
- Use `SettingsItem` for settings rows and `PageHeader`'s actions slot for page-level actions. Let action groups wrap and stay reachable at narrow desktop widths. For any other Vuetify element, follow the closest existing usage in the repository instead of Vuetify defaults.
- Use the radius tokens (`--astrbot-radius-control`, `--astrbot-radius-card`, and the other tokens in `src/scss/_radius-tokens.scss`) instead of hardcoded radii.

### Vuetify Cascade and Stacking Conventions

Project styles in `src/scss/` and component `<style>` blocks are unlayered, so they always beat Vuetify's `@layer` rules. When overriding a Vuetify default, verify the selector wins twice: over Vuetify's layered rule and over any competing unlayered rule in this repository. Count specificity including `:not()` arguments; a lower-specificity "fix" rule placed next to a higher-specificity global rule never takes effect (this is how a `border-radius: 0` group fix silently failed against the global `.v-btn` radius rule).

- Vuetify pins many surface radii (card, menu content, dialog, snackbar, alert, button group) to its 4px root default. Map every surface to a radius token from `src/scss/_radius-tokens.scss` instead of accepting the default, and keep parent and child radii aligned: menu popups carry their radius on `.v-overlay__content` with `v-card`/`v-sheet`/`v-list` children inheriting it, and `v-select` also attaches the elevation class to the content element, so a mismatched child radius lets the elevation shadow's corners show behind the panel.
- `v-btn-toggle` / `v-btn-group` buttons must stay square and inherit only the container's outer corners. A per-button radius breaks the `divided`/`outlined` border joins and the container's rounded overflow clips the last button.
- Vuetify teleports overlays to `body` starting at z-index 2400, and `.v-application__wrap` (`backface-visibility: hidden`) as well as any `isolation: isolate` shell create stacking contexts. z-index only competes within its own stacking context, so verify the full ancestor chain before trusting a z-index value. Window chrome that must stay above the dialog scrim, such as the custom title bar with its window controls, must be teleported to `body` and use an opaque chrome background while a scrimmed overlay is active (`:has()` is available in WebView2).
- Layout animation is scoped: `v-main` has no transition by default (tab switching must not animate); only the sidebar expand/collapse enables a `v-main` padding transition via the `.layout-animating` class in `App.vue`.

## Tauri and Rust Conventions

- Implement filesystem, process, download, archive extraction, environment variable, and privileged system operations in Rust. Expose them to the frontend through controlled Tauri commands; do not grant the frontend arbitrary shell or path capabilities.
- Tauri commands must have stable parameter and return types and be registered in `invoke_handler`. Keep field naming, nullability, and error shapes consistent between the frontend and backend.
- Rust errors should include enough diagnostic context and expose a predictable error contract. The frontend must translate them into user-friendly messages rather than swallowing failures.
- Do not perform long-running blocking work inside async commands. Use a blocking thread pool, background tasks, or streamed events for downloads, extraction, and installation, and provide cancellable or at least observable progress.
- Pass external process arguments as arrays. Never concatenate user input into a shell command. Explicitly handle the working directory, environment variables, and exit codes.
- Follow the principle of least privilege when adding plugins or capabilities. Grant only the permissions required by the current feature, and justify them in the same change.
- Handle platform differences in paths, file permissions, and process behavior explicitly. Avoid hardcoding Windows path separators in shared logic.
- On Windows 11, apply DWM system backdrops (`DWMWA_SYSTEMBACKDROP_TYPE`) before setting the window corner preference: applying a backdrop resets the corner preference, so rounding must come last. Gate Windows 11-only visuals by the real build number from `RtlGetVersion` (22000 = Mica, 22621 = Mica Alt); `GetVersionEx` is shimmed and lies.
- Apply window effects and other DWM attributes explicitly and log every result. Tauri's config-driven `windowEffects` discards all errors silently, so a failed Mica/acrylic call is invisible without logs; a silent failure also falls back to plain transparency, which changes the whole window's look.

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
- Russian copy uses native typography such as `«»` and `—`; Japanese copy uses full-width punctuation such as `、。：「」`.
- Only adjust punctuation in comments, docstrings, and string literals. Punctuation that is part of code syntax must not be changed.
- Keep locale keys synchronized across `zh-CN`, `en-US`, `ru-RU`, and `ja-JP` when adding, removing, or modifying entries. Do not leave visible copy that exists in only some languages.
- When writing bilingual Markdown documents in a single file (such as `CONTRIBUTING.md` and `changelogs/*.md`), present English first and Simplified Chinese second in separate top-level language sections (`## English` and `## 中文`) rather than mixing both languages in the same paragraph or list item.
- Do not mechanically replace punctuation in URLs, regular expressions, version numbers, format placeholders, or code snippets.

## Verification Requirements

- For frontend changes, run at least `pnpm build` to confirm that `vue-tsc` and the Vite build pass. If the run fails on the pnpm supply-chain pre-check instead of on TypeScript or Vite, rerun with `--registry=https://registry.npmjs.org/` and report the real tool result; the policy error is an environment issue, not a code failure.
- For Rust or Tauri configuration changes, run at least `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo check`.
- For UI interaction, Tauri commands, permissions, filesystem operations, or process behavior, document the relevant manual verification. Run `pnpm tauri dev` for desktop integration testing when possible.
- Add unit tests for new critical pure logic, prioritizing path validation, version comparison, backup manifests, error conversion, and repeated execution.
- If the repository has no configured frontend lint or test script, do not invent commands or claim that they were run.
- Report only the commands that were actually run and their results during verification. Clearly disclose checks that were not run, platform behavior that could not be reproduced, and any remaining risks.

## Changelog and Release

References:

- [Semantic Versioning](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/)

- Follow the rules described in `changelogs/README.md`.
- During development, add user-facing changes to `changelogs/unreleased.md` in the same pull request, under `Added`, `Changed`, `Deprecated`, `Removed`, `Fixed`, and `Security` as applicable. Internal refactors, CI changes, and documentation-only changes may skip it.
- Keep each entry short and focused on the most important user-visible change. One pull request contributes exactly one logical entry (one English bullet and its matching Chinese bullet in the same section), not one entry per section. Do not split one pull request across multiple entries; when it spans several areas, choose the most user-visible change.
- Keep `changelogs/*.md` bilingual with `## English` first and `## 中文` second, and omit empty sections instead of leaving placeholder headings.
- `changelogs/unreleased.md` is a working draft. Leave its entries unannotated during development; pull request links are added only in the version update pull request. It is not used as release notes until it is renamed.
- Versions follow [Semantic Versioning](https://semver.org/); changelog entries follow [Keep a Changelog](https://keepachangelog.com/).
- After a release, continue development and keep adding entries to `changelogs/unreleased.md`; do not bump the version until you are ready to release.
- When ready to release, create a version update pull request that bumps `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` to the release version, updates `src-tauri/Cargo.lock` by running `cargo check` in `src-tauri/`, renames `changelogs/unreleased.md` to `changelogs/v<version>.md`, and recreates an empty `changelogs/unreleased.md` for the next cycle.
- In that release pull request, annotate every entry with its source pull request number as a full Markdown link: `([#12](https://github.com/mantoujun12/AstrBot-Go/pull/12))` in English sections and `（[#12](https://github.com/mantoujun12/AstrBot-Go/pull/12)）` in Chinese sections. Both language versions of an entry must link to the same pull request.
- Do not backfill pull request links into historical version files such as `changelogs/v0.1.1.md`.
- Before merging, rebase the version update pull request onto the latest `main` and move concurrent `unreleased.md` entries into the new draft so they stay in the next release cycle.
- Merge the version update pull request into `main`. `.github/workflows/auto-release.yml` builds the Windows installers and publishes the GitHub Release.
- The workflow only enforces that `package.json` and `src-tauri/tauri.conf.json` share the same version; `src-tauri/Cargo.toml` must be kept in sync manually.
- The workflow only publishes when the versions match, `changelogs/v<version>.md` exists with real entries, and no GitHub Release exists for the tag. An existing tag without a release also skips and requires manual handling.

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
