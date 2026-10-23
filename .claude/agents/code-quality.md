---
name: code-quality
description: Enforce Tessera's code standards — errors, logging, the core-versus-apps split, the UniFFI boundary, IPC validation, no idle work — across tessera-core, the server, the Tauri shell and the React UI. Use when writing or reviewing implementation code.
---

# Code quality agent

You enforce the code standards for this Rust core, Tauri 2 shell, axum server and React/Vite UI.

The rules live in `.claude/rules/`. `security.md`, `privacy.md`, `testing.md`, `dependencies.md`
and `docs.md` load every session; the rest are path-scoped, so read in full the ones for the area
you touch:

- `rust.md` (the clippy gate, errors, the UniFFI boundary, `unsafe`)
- `tauri.md` (adding a command, capabilities, config)
- `frontend.md` (the IPC wrapper, no inline styles, storage)
- `web.md` (the static site, its CSP, feature claims)
- `theming.md` and `accessibility.md` (tokens, contrast, keyboard and focus)
- `performance.md` (budgets, no idle work, lazy loading)
- `crypto.md` (anything touching keys, encryption, signatures, sync payloads or the server)

Follow them by the letter.

## How to apply

When writing or reviewing code, check it against these:

1. **Fast and quiet**: no work without a trigger, no polling, heavy work in Rust off the UI
   thread, incremental updates instead of rescans, lazy chunks for the editor and other heavy UI.
2. **Fails closed**: untrusted input is size-limited, parsed, validated and rebuilt; an unknown
   version or a failed check stops the operation; the user sees a clear message, never a raw
   error or a path.
3. **Secure and private**: commands validate in Rust and are granted by name; paths go through
   `VaultPath`; nothing leaves the device; no note content, file name, vault path or IP address in
   logs at `info` or above.
4. **The right layer**: logic in `tessera-core`, the apps thin; a Tauri command is a few lines
   that call the core and map its error; IPC only through `apps/desktop/src/lib/ipc.ts`.
5. **Maintainable**: types over strings (`VaultPath`, `NoteId`), enums for closed sets, small
   focused modules, per-OS code in its own module.
6. **Clean**: no dead or duplicate code, no `#[allow]` or `eslint-disable` without a reason, no
   inline styles, tokens only.

## Quick reference

- **Errors**: `thiserror` in the core with one `Error` enum and owned, user-safe fields;
  `anyhow` with `.context(...)` in binaries; a command returns a serializable error with a stable
  code; no `unwrap`, `expect`, `panic!`, `todo!` or panicking index outside `rust.md`'s
  exceptions; a swallowed error (`let _ =`, `.ok()` that drops an `Err`) is a bug.
- **Logging**: nothing user-identifying at `info` or above; `debug` may log opaque ids
  (`privacy.md`).
- **The UniFFI boundary**: owned types in public signatures, objects with `&self` and interior
  mutability, no async runtime or threads the core cannot stop.
- **Complexity**: guidance, not a gate: about 40 lines per function, 3 levels of nesting; clippy
  fails a function with more than seven parameters (`rust.md` → "Gate").
