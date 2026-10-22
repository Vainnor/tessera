---
paths:
  - "crates/**"
  - "apps/desktop/src-tauri/**"
  - "Cargo.toml"
  - "Cargo.lock"
---

# Rust Rules (not loaded at startup; read in full before writing or reviewing Rust)

## Gate

- `cargo clippy --workspace --all-targets --locked -- -D warnings`. `--all-targets` matters: test
  modules only compile under it. Lints live in the root `Cargo.toml` `[workspace.lints]`; every
  crate has `[lints] workspace = true`.
- `cargo fmt --all`; `rustfmt.toml` is the style.
- Implementation goes above `#[cfg(test)] mod tests`, or clippy's `items_after_test_module` fails.
- Over seven parameters fails `too_many_arguments`: group them in a struct, never `#[allow]`.
- The desktop crate needs `apps/desktop/dist`: run `pnpm build` first (`pnpm gate` does).

## Errors

- **Libraries (`tessera-core`) use `thiserror`**: one `Error` enum, variants a caller can match on,
  owned user-safe fields, never an absolute path (`crates/core/src/error.rs`).
- **Binaries (`tessera-server`, the desktop shell) use `anyhow`** for setup and glue, with
  `.context("what was being done")`.
- A Tauri command returns `Result<T, E>` where `E` is serializable with a stable code and a
  user-safe message. Never send `io::Error` text to the webview; it can hold a path.
- No `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!` or panicking indexing in
  non-test code; the workspace lints deny them. A truly unreachable case is an invariant the code
  guarantees, written with `#[allow(..., reason = "...")]` on the line.
- Never swallow: no `let _ = fallible()`, no `.ok()` discarding an error, no `unwrap_or_default()`
  on I/O. Log and surface, or propagate.

## The UniFFI boundary (`tessera-core`)

Everything `pub` in `tessera-core` will be exposed to Swift and Kotlin through UniFFI. Keep it
expressible:

- Owned types only in public signatures: `String`, `Vec<u8>`, integers, `bool`, `Option`, `Vec`,
  and records and enums built from them. No lifetimes, generics, `impl Trait`, `&str` returns or
  borrowed iterators in public signatures (internal helpers are free).
- Long-lived things are objects: constructor returns `Result<Self, Error>`, methods take `&self`,
  mutation uses interior mutability (`Mutex`, `RwLock`) so the object can live in an `Arc`.
- Callbacks into the client are traits that are `Send + Sync` (`sync::SyncEngine`).
- The core owns no async runtime and spawns no threads it cannot stop. Callers run blocking work on
  their own threads.
- `tessera-core` has no Tauri, UI, HTTP or async-runtime dependency. If a feature seems to need
  one, the boundary is wrong.

## Style

- `#![forbid(unsafe_code)]` in every crate. Unsafe needs an ADR, its own module, a safe wrapper and
  a `// SAFETY:` comment per block.
- Types over strings: `VaultPath`, `NoteId`, `KeyGeneration`. Enums for closed sets.
- Per-OS code goes in its own module behind `#[cfg(target_os = "...")]`, never scattered inside a
  function.
- Comments explain a non-obvious reason for the code as it is now. No history, no issue stories.
