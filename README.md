# Tessera

An open-source, local-first notes app. Your notes are plain markdown files in a folder you
choose, compatible with Obsidian vaults. Sync between devices will be end-to-end encrypted, so
the server cannot read your notes.

**Status: early development.** The foundations are in place (the Rust core, a desktop app shell,
the theme system and the website). The desktop app can open a folder of markdown notes and show them, but you cannot edit notes in it yet. Nothing below marked
_planned_ exists.

|                                                                               | Status                            |
| ----------------------------------------------------------------------------- | --------------------------------- |
| Open source under AGPL-3.0-or-later                                           | Available                         |
| Themes: Vainnor, Light, Dark, Paper, High contrast, plus your own JSON themes | In progress (built, not released) |
| Markdown vaults compatible with Obsidian: open a folder, list and read notes  | In progress (built, not released) |
| Editor, backlinks, local full-text search                                     | Planned                           |
| End-to-end encrypted sync, self-hostable server                               | Planned                           |
| Native iPad app with Apple Pencil handwriting, drawing and PDF annotation     | Planned                           |

Website: <https://tessera.vainnor.com> (planned). Privacy principles: [`PRIVACY.md`](PRIVACY.md).

## Development

You need Rust (installed automatically from `rust-toolchain.toml` by `rustup`), Node.js 24+, pnpm
(`corepack enable`), and [Tauri's system dependencies](https://v2.tauri.app/start/prerequisites/)
for your OS. `cargo-deny`, `cargo-audit` and `gitleaks` are needed for the full gate.

```sh
git config core.hooksPath .githooks   # commit-message check and local secret scan
pnpm install
pnpm build                            # theme CSS, desktop frontend, website

pnpm dev:desktop                      # desktop app with hot reload
pnpm dev:web                          # website at http://localhost:4321
cargo run -p tessera-server           # sync server skeleton: http://127.0.0.1:8787/healthz

pnpm gate                             # everything CI checks, locally

docker build -f apps/web/Dockerfile .      # website image (Caddy on :8080)
docker build -f crates/server/Dockerfile . # sync server image (:8787)
```

The server reads `--config <file>` or `$TESSERA_SERVER_CONFIG`; see
[`crates/server/tessera-server.example.toml`](crates/server/tessera-server.example.toml).

## Repository layout

```
crates/core      tessera-core: the shared Rust library every client uses
crates/server    tessera-server: the sync server (skeleton)
apps/desktop     Tauri 2 desktop app (React + Vite UI, Rust shell)
apps/web         Astro website
packages/theme   design tokens, theme presets and the theme file format
packages/ui      shared styles and components
docs/            architecture, threat model, decision records, budgets, design
```

Start with [`docs/architecture.md`](docs/architecture.md). The encryption design, which is a
proposal under review, is [`docs/threat-model.md`](docs/threat-model.md).

## Contributing and security

See [`CONTRIBUTING.md`](CONTRIBUTING.md). Work is tracked as issues on the
[Tessera Kanban](https://github.com/users/Vainnor/projects/1) board; how to file one is in
[`.github/ISSUE_GUIDELINES.md`](.github/ISSUE_GUIDELINES.md). Each change lands on `main` by a pull
request from an issue branch. Report vulnerabilities privately as described in
[`SECURITY.md`](SECURITY.md), never in a public issue.

## License

[AGPL-3.0-or-later](LICENSE). If you run a modified version of the sync server for other people,
you must offer them its source.
