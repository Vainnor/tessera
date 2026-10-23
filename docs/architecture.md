# Architecture

Tessera is a local-first notes app. The user's notes are a folder of markdown files (an
Obsidian-compatible vault); every client reads and writes those files through one Rust library,
`tessera-core`. Sync, when it exists, moves end-to-end encrypted CRDT updates through a server
that cannot read them.

What exists today is the scaffold: the crates, apps and packages below, wired together and
tested, with the product logic stubbed.

## Principles

- **The files are the data.** Markdown on disk is the source of truth. No database owns the notes;
  anything else (sync state, the search index) can be rebuilt from the files.
- **Private by construction.** No telemetry, analytics, crash reporting or third-party requests,
  in the app or on the website. The sync server sees only ciphertext
  ([ADR 0002](adr/0002-end-to-end-encrypted-sync.md)).
- **One core, thin clients.** Logic lives in `tessera-core`; UIs render what it returns.
- **Cheap to run.** Budgets in [`performance.md`](performance.md) are set before features land.
- **Accessible and keyboard-first.** See [`design-principles.md`](design-principles.md).

## Map

```
tessera/
├── crates/
│   ├── core/            tessera-core: vault, documents, sync, crypto (stubs). The UniFFI boundary.
│   └── server/          tessera-server: axum sync server skeleton (config, logs, /healthz)
├── apps/
│   ├── desktop/         Tauri 2 app: React + Vite UI in src/, Rust shell in src-tauri/
│   └── web/             Astro static site for tessera.vainnor.com
├── packages/
│   ├── theme/           Design tokens, preset themes, theme schema and validator, generated CSS
│   └── ui/              Shared base CSS, component classes, and small React components
├── docs/                This folder: architecture, threat model, ADRs, budgets, design
└── scripts/             gate.sh (full local gate), check-bundle-size.ts
```

## How the pieces talk

```
            ┌─────────────── desktop app ───────────────┐
            │ React UI ──invoke("app_info")──▶ Tauri     │
            │ (apps/desktop/src)     IPC        commands │
            │        ▲                             │     │
            │        │ CSS variables               ▼     │
            │ @vainnor/tessera-theme        tessera-core │──▶ vault folder (.md files)
            └────────────────────────────────────┬───────┘
                                                 │ planned: encrypted CRDT updates over TLS
  future iPad app (Swift, PencilKit, PDFKit)     ▼
  └── UniFFI bindings ──▶ tessera-core      tessera-server ──▶ ciphertext storage
```

- **Desktop.** The webview runs the React UI. It reaches Rust only through `src/lib/ipc.ts`,
  which calls Tauri commands in `src-tauri/src/commands.rs`; each command is a few lines over
  `tessera-core`. The capability in `src-tauri/capabilities/main.json` grants commands by name and
  nothing else.
- **iPad, later.** A native Swift app links `tessera-core` through UniFFI-generated bindings and
  uses PencilKit and PDFKit for ink and PDFs. The core's public API already follows UniFFI's rules
  (see the crate docs in `crates/core/src/lib.rs`), so the bindings are generated rather than
  written. Ink and annotations are stored as files in the vault, so they sync like notes.
- **Website.** Built at compile time from the same theme tokens; ships one small script.
- **Server.** Will store per-vault logs of encrypted update batches and blobs, and relay them.

## The vault

`tessera-core`'s `Vault` (`crates/core/src/vault/`) is the only code that touches notes on disk.

- `list_notes` walks the folder and returns markdown notes and folders, sorted. It skips hidden
  entries (`.obsidian`, `.git`, `.trash`) silently and reports what it could not list: symbolic
  links, names some OS cannot store, folders deeper than 64 levels, unreadable folders. It stops
  at 200,000 entries and says so.
- `read_note` returns the markdown exactly as stored plus a `Revision` (size and modification
  time). Notes must be UTF-8 and at most 16 MiB.
- `create_note` never replaces a file. `write_note` takes the revision it is based on, refuses
  with `Conflict` if another program changed the note since, and replaces the file atomically.
- `move_note` renames or moves a note, creating folders as needed. It never replaces anything at
  the destination (hard link, then remove the old name) and handles case-only renames on
  case-insensitive filesystems.
- `trash_note` moves a note into the vault's `.trash`, keeping its folders, as Obsidian does;
  names already in the trash get a number. Nothing in the core deletes a note outright.
- Every operation that changes an existing note takes the revision it is based on and refuses
  with `Conflict` if the note changed on disk.
- Paths are `VaultPath`s; links inside the vault are never followed (see the threat model).

Not built yet: attachments and file watching.

The desktop app opens a vault with the native folder picker, reopens the last vault at launch
(its path is kept in `settings.json` in the app's config folder and never sent to the webview),
lists and reads notes, and renames, moves and trashes them (`apps/desktop/src-tauri/src/commands.rs`).
The note list and the open note refresh when the window regains focus, so changes made in other
apps show up without a file watcher or any idle work. It shows a note's markdown source; the editor
and the write command come next.

## Sync (planned)

A CRDT library merges concurrent edits. [ADR 0003](adr/0003-crdt-library.md) chooses Automerge
over `yrs` and Loro; it is not added yet. The candidates were judged on: correctness under
concurrent edits of markdown text, memory and CPU at 10,000 notes, update size, how cleanly
updates batch and encrypt as opaque blobs, compaction of history without the server's help, and
a Rust implementation usable from the core without a JS runtime. The encryption layer wraps
whatever the CRDT emits ([threat model](threat-model.md)).

## Container images

`.github/workflows/images.yml` builds two images and pushes them to `ghcr.io/vainnor`:

```
CI green on main ──> tessera-web:<version>-<sha7>, :main ──> retag :prod ──> homelab pulls :prod
                 └─> tessera-server:<version>-<sha7>, :main   (built, not deployed)
tag v<version>   ──> both images :<version>, :latest
pull request     ──> both images built, nothing pushed
```

`<version>` is `[workspace.package] version` in `Cargo.toml`. The website image is the static
build served by Caddy (`apps/web/Caddyfile`) on :8080, unprivileged, with the same headers as
`public/_headers`. The server image is the release binary on distroless with
`crates/server/container.toml`, which binds 0.0.0.0:8787 inside the container. TLS terminates at
the homelab's reverse proxy for both.

## Dependencies

The list, with the reason for each, is [ADR 0001](adr/0001-repository-and-stack.md#dependencies).
Adding one follows `.claude/rules/dependencies.md`.
