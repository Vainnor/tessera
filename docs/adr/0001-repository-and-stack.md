# 0001: Repository layout, stack and dependencies

- Status: Accepted
- Date: 2026-10-22

## Context

Tessera needs a desktop app now, a website now, a sync server soon, and a native iPad app and
mobile clients later, all behaving the same on the same data. It must stay small, private and
cheap to run. Most decisions below were made by the maintainer before the repository existed;
this record writes them down with their reasons.

## Decision

- **One repository**: a Cargo workspace (`crates/*`, `apps/desktop/src-tauri`) and a pnpm
  workspace (`apps/*`, `packages/*`).
- **All logic in Rust, in `tessera-core`.** The vault, document model, sync and encryption live in
  one synchronous library with an FFI-friendly public API (owned types, objects behind `&self`, one
  flat error enum). The desktop app calls it through Tauri commands; the iPad app will call it
  through UniFFI-generated Swift bindings. No client reimplements core logic.
- **Desktop: Tauri 2 + React + TypeScript + Vite.** Tauri uses the OS webview, so the app is a
  few megabytes and idles at near-zero CPU, unlike a bundled Chromium. React because the editor
  ecosystem (CodeMirror, ProseMirror bindings) is there.
- **Website: Astro, static.** It renders to plain HTML and CSS at build time and ships no
  framework JavaScript.
- **Server: Rust, axum on tokio.** It shares types with the core and runs in a few megabytes.
- **Shared design tokens** in `packages/theme`, consumed by both the desktop app and the site.
- **License: AGPL-3.0-or-later**, so anyone who runs a modified sync server for others must share
  their changes.

## Dependencies

Each one is here because nothing smaller does the job. The policy for adding more is
`.claude/rules/dependencies.md`.

### Rust

| Crate                           | Used by         | Why                                                                           |
| ------------------------------- | --------------- | ----------------------------------------------------------------------------- |
| `thiserror`                     | core, server    | Typed error enums without hand-written `Display` impls                        |
| `anyhow`                        | server binary   | Context on setup errors in binaries only; libraries use `thiserror`           |
| `serde`, `serde_json`           | server, desktop | Config and IPC serialization; the de facto standard                           |
| `toml`                          | server          | Config file format; `parse` + `serde` features only                           |
| `tokio`                         | server          | Async runtime for axum; only `macros`, `net`, `rt-multi-thread`, `signal`     |
| `axum`                          | server          | HTTP routing from the tokio team, thin over hyper; `default-features = false` |
| `tracing`, `tracing-subscriber` | server          | Structured logs, JSON output, `RUST_LOG` filters                              |
| `tauri`, `tauri-build`          | desktop         | The shell. `default-features = false`; no plugins                             |
| `tower`, `http-body-util` (dev) | server tests    | Drive the router in tests without a socket                                    |

### JavaScript

| Package                                                                             | Kind            | Why                                                                                                                                                              |
| ----------------------------------------------------------------------------------- | --------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `react`, `react-dom`                                                                | desktop runtime | UI                                                                                                                                                               |
| `@tauri-apps/api`                                                                   | desktop runtime | `invoke` for IPC; imported only by `src/lib/ipc.ts`                                                                                                              |
| `astro`                                                                             | website build   | Static site generator; ships no JS to visitors                                                                                                                   |
| `vite`, `@vitejs/plugin-react`                                                      | desktop build   | Bundler Tauri expects                                                                                                                                            |
| `@tauri-apps/cli`                                                                   | desktop build   | `tauri dev`, `tauri build`, `tauri icon`                                                                                                                         |
| `typescript` (6.0)                                                                  | dev             | Type checking. Held at 6.0 because `@astrojs/check` and `typescript-eslint` do not support the 7.0 native compiler yet                                           |
| `@astrojs/check`                                                                    | dev             | Type checks `.astro` files                                                                                                                                       |
| `vitest`                                                                            | dev             | Tests, Vite-native                                                                                                                                               |
| `ajv`                                                                               | dev             | Proves the published theme JSON Schema agrees with the runtime validator                                                                                         |
| `eslint`, `@eslint/js`, `typescript-eslint`, `eslint-plugin-react-hooks`, `globals` | dev             | Lint, including the rules that protect the CSP                                                                                                                   |
| `prettier`, `prettier-plugin-astro`                                                 | dev             | Formatting                                                                                                                                                       |
| `@types/node`, `@types/react`, `@types/react-dom`                                   | dev             | Types                                                                                                                                                            |
| `@lhci/cli` (not a dependency)                                                      | CI tool         | Lighthouse budget. Run pinned through `pnpm dlx`: its puppeteer tree has unpatched advisories, so it stays out of the lockfile and `pnpm audit` stays meaningful |

`eslint-plugin-jsx-a11y` was considered and left out: it has not had a release since 2024 and does
not declare ESLint 10 support. Accessibility is enforced by Lighthouse on the website for now;
component-level checks for the desktop app are a follow-up.

## Consequences

- One language for every rule that must agree across platforms, at the cost of an FFI layer for
  the iPad app (UniFFI) and an IPC layer for desktop (Tauri commands).
- Two package managers in one repo. `pnpm gate` and CI run both.
- AGPL keeps the project open but rules out some downstream commercial embedding without a
  separate license.
- Revisit Tauri if the OS webviews diverge enough to cost more than a bundled engine would.
