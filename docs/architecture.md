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
- Paths are `VaultPath`s; links inside the vault are never followed (see the threat model).

Not built yet: rename, move and delete (to the vault's `.trash` or the OS trash), attachments,
file watching, and the desktop commands that expose the vault to the UI.

## Sync (planned)

A CRDT library merges concurrent edits (candidates: Yjs via `yrs`, Loro, Automerge). The
choice is open and will be judged on: correctness under concurrent edits of markdown text,
memory and CPU at 10,000 notes, update size, how cleanly updates batch and encrypt as opaque
blobs, compaction of history without the server's help, and a Rust implementation usable from
the core without a JS runtime. The encryption layer wraps whatever the CRDT emits
([threat model](threat-model.md)).

## Dependencies

The list, with the reason for each, is [ADR 0001](adr/0001-repository-and-stack.md#dependencies).
Adding one follows `.claude/rules/dependencies.md`.
