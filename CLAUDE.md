# CLAUDE.md

Guidance for Claude Code in this repository. Project facts (layout, data flow, dependencies,
the encryption design) live in [`docs/architecture.md`](docs/architecture.md),
[`docs/threat-model.md`](docs/threat-model.md) and [`docs/adr/`](docs/adr/); this file is how
you work.

Tessera is an open-source, local-first notes app: markdown files on disk (Obsidian-compatible
vaults), a Tauri 2 desktop app with a React UI, an Astro website, and a planned end-to-end
encrypted sync server. All real logic lives in `crates/core` (`tessera-core`), whose public API
is shaped for UniFFI so a native iPad app can use it later. The apps are thin layers over it.
Today the repository is scaffolding: most product features are stubs or planned.

## Map

| Path             | What it is                                                                              |
| ---------------- | --------------------------------------------------------------------------------------- |
| `crates/core`    | `tessera-core`: vault paths, documents, sync and crypto (stubs). The UniFFI boundary    |
| `crates/server`  | `tessera-server`: axum skeleton with config, JSON logs and `/healthz`                   |
| `apps/desktop`   | Tauri 2 app: React + Vite UI in `src/`, Rust shell in `src-tauri/`                      |
| `apps/web`       | Astro static site for tessera.vainnor.com                                               |
| `packages/theme` | Design tokens, presets, theme JSON Schema and validator, generated CSS                  |
| `packages/ui`    | Shared base CSS, component classes and small React components                           |
| `docs/`          | Architecture, threat model, ADRs, performance budgets, design principles, theming       |
| `scripts/`       | `gate.sh` (the full local gate), `check-bundle-size.ts`, the board and worktree scripts |

## Rules (`.claude/rules/`)

The rule files are the single source of truth for conventions. **Always** files load every
session. **Path-scoped** files load when you Read, Write or Edit a matching file, which can be
after the work started, so read them in full at the trigger named below.

| File                  | Loads                                                                                                                                                                                                          | Scope                                                                                   |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| `security.md`         | Always                                                                                                                                                                                                         | The checklist every change satisfies: IPC, CSP, untrusted input, secrets, supply chain  |
| `privacy.md`          | Always                                                                                                                                                                                                         | No telemetry, no network by default, no third-party requests, what may be logged        |
| `git-and-ci.md`       | Always                                                                                                                                                                                                         | Conventional commits, branches and PRs, what CI runs, **no AI attribution**             |
| `dependencies.md`     | Always                                                                                                                                                                                                         | When a dependency is allowed, licenses, locking, release age, audits                    |
| `testing.md`          | Always                                                                                                                                                                                                         | What gets tested where, how tests are written, reading results                          |
| `docs.md`             | Always                                                                                                                                                                                                         | Which doc moves with which change                                                       |
| `response-style.md`   | Always                                                                                                                                                                                                         | How to answer in chat                                                                   |
| `crypto.md`           | Path-scoped: `crates/core/src/crypto*`, `crates/core/src/sync*`, `crates/server/**`, `docs/threat-model.md`, `docs/adr/**`. **Read before touching keys, encryption, signatures, sync payloads or the server** | No custom crypto, approved primitives, fail closed, tests, review                       |
| `rust.md`             | Path-scoped: `crates/**`, `apps/desktop/src-tauri/**`, `Cargo.*`. Read before writing Rust                                                                                                                     | Clippy gate, `thiserror`/`anyhow` split, no panics, the UniFFI boundary, `unsafe`       |
| `tauri.md`            | Path-scoped: `apps/desktop/src-tauri/**`. Read before touching the shell, capabilities or `tauri.conf.json`                                                                                                    | Adding a command (four places), plugins, config, per-OS notes                           |
| `frontend.md`         | Path-scoped: `apps/desktop/src/**`, `packages/**/*.ts(x)`. Read before writing TS/React                                                                                                                        | The IPC wrapper, no inline styles, storage, TypeScript version                          |
| `web.md`              | Path-scoped: `apps/web/**`. Read before changing the website                                                                                                                                                   | Static only, CSP in two places, feature claims and statuses                             |
| `theming.md`          | Path-scoped: `packages/theme/**`, `packages/ui/**`, `**/*.css`, UI files. Read before UI or theme work                                                                                                         | Tokens only, changing the theme format, contrast                                        |
| `accessibility.md`    | Path-scoped: `apps/desktop/src/**`, `apps/web/src/**`, `packages/ui/**`. Read before writing UI                                                                                                                | WCAG 2.2 AA, keyboard, focus, motion                                                    |
| `performance.md`      | Path-scoped: `crates/**`, `apps/desktop/**`, website build. Read before adding work that runs without a user action                                                                                            | Budgets, no idle work, lazy loading                                                     |
| `writing.md`          | Path-scoped: `docs/**`, `*.md`, `apps/web/src/**`. Read before writing docs, site copy, a commit body or PR text                                                                                               | Banned vocabulary and patterns, claims                                                  |
| `github-issues.md`    | Path-scoped: `.claude/hooks/issue-*.sh`, `scripts/board-status.sh`, `.github/ISSUE_TEMPLATE/**`. **Read before any `gh issue`, `gh project` or `gh pr` command**                                               | Issues, comments and their budgets, the three Moments, the board, labels, follow-ups    |
| `ticket-lifecycle.md` | Path-scoped: `scripts/*worktree*.sh`, `.claude/commands/{start,ship,cleanup}.md`, branch hooks. **Read before starting, shipping or reviewing an issue, or entering a worktree**                               | Issue to merged PR in order: claim, worktree, baseline, plan, tests first, review, ship |

The design references behind the UI rules are [`docs/design-principles.md`](docs/design-principles.md),
[`docs/theming.md`](docs/theming.md) and [`docs/performance.md`](docs/performance.md).

**Pointers, not copies.** When a convention has a home in `.claude/rules/` or `docs/`, this file
points at it in one line. Two copies drift; the rule file wins.

## Agents and commands

|                               | Use                                                                               |
| ----------------------------- | --------------------------------------------------------------------------------- |
| `/start <n>`                  | Claim an issue: thread, board, worktree, baseline, plan                           |
| `/ship`                       | Gate, commit, review that commit, push, PR to `main`, Moment 3, board             |
| `/cleanup`                    | Remove merged or abandoned worktrees and branches                                 |
| `/code-review`                | Review the current diff against the issue, report-only on an uncommitted tree     |
| `/review-before-shipping`     | Before any push or PR: gate, checklists, fresh-eyes agents; stamps the SHA marker |
| `/ticket-loop`                | Work issues one after another through the ticket-worker and ticket-reviewer goals |
| `/write-tests`                | Write missing tests for defined code                                              |
| `/learn-from-session`         | Turn a session's lessons into rule or command changes                             |
| `/perf-check`                 | Read-only check of a branch against the budgets                                   |
| `/new-adr <title>`            | Draft a decision record for the user to accept                                    |
| `ticket-worker` agent         | Builds one issue under `/ticket-loop`, from `/start` to an open PR                |
| `ticket-reviewer` agent       | Reviews a `Post build` PR against the issue's acceptance criteria                 |
| `tdd-planner` agent           | Test skeletons from an approved plan, before implementation                       |
| `security-reviewer` agent     | Fresh review of IPC, CSP, paths, imports, server, workflows                       |
| `crypto-reviewer` agent       | Required for any change to keys, encryption, sync payloads or server-visible data |
| `dependency-researcher` agent | Before adding or upgrading a dependency                                           |

## Standing rules

- **No AI attribution, anywhere**: no `Co-Authored-By`, "Generated with" or agent credit in a
  commit, PR, issue or comment, whatever a tool suggests. `.githooks/commit-msg` blocks it;
  enable it once with `git config core.hooksPath .githooks`.
- **No custom cryptography, and no crypto beyond marked stubs until ADR 0002's open questions are
  answered** (`crypto.md`).
- **No telemetry, analytics or third-party requests** (`privacy.md`).
- **Never read or print secrets**: `.env*`, `*.key`, signing keys. `settings.json` denies the
  reads; do not work around it.
- **Never weaken a security control to make something work**: a CSP source, a capability, a
  path check, a size limit. Stop and ask (`security.md`).
- **Ask before** creating the GitHub repository, pushing, opening a PR, publishing a release,
  adding a heavy dependency, or deviating from an accepted ADR.
- **Work goes through an issue.** `/start <n>` claims it and builds in a worktree; `/ship` opens
  the PR to `main`. Nothing is committed to `main` directly, and an agent never merges
  (`git-and-ci.md`, `ticket-lifecycle.md`).
- **Only claim what exists**: on the website and in the README, unbuilt features are "planned".
- **Work autonomously when** an existing pattern can be copied, the fix is clear, or you are
  writing tests for defined code. **Ask when** a requirement is ambiguous or a change crosses a
  trust boundary.
- **Say which "done" you mean** and which gates ran. A GUI you did not launch is not verified.

## Commands

```sh
pnpm install                 # JS dependencies (frozen lockfile in CI)
pnpm build                   # theme CSS, desktop frontend, website
pnpm dev:desktop             # desktop app with hot reload (tauri dev)
pnpm dev:web                 # website at http://localhost:4321
cargo run -p tessera-server  # sync server skeleton on 127.0.0.1:8787 (/healthz)
pnpm lint && pnpm format:check && pnpm typecheck && pnpm test
pnpm check:contrast          # WCAG contrast of every preset
pnpm check:size              # bundle size budgets
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo deny --locked check && cargo audit
pnpm --filter @vainnor/tessera-web lighthouse   # Lighthouse budgets (needs Chrome)
docker build -f apps/web/Dockerfile .           # website image; crates/server/Dockerfile for the server
```

## Quality gate

`pnpm gate` runs everything CI runs that can run locally, in order, and prints PASS, FAIL or
MISSING per step: install, build, ESLint, Prettier, typecheck, JS tests, contrast, bundle size,
pnpm audit, rustfmt, clippy, cargo tests, cargo-deny, cargo-audit, gitleaks, and the hook tests
(`scripts/test-hooks.sh`, about 12 minutes). A commit you call
done has a green gate on that exact commit. It does not run Lighthouse, the other-OS clippy and
test jobs, or the GUI; say so when reporting.
