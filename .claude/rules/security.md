# Security Rules (always loaded)

The assets, trust boundaries and adversaries are `docs/threat-model.md`. This file is the
checklist every change satisfies. A change that cannot satisfy a line stops and asks; it never
loosens a control to get something working.

## Webview and IPC

- The webview is untrusted: it renders notes, themes and later PDFs. It holds no keys and decides
  nothing. A Tauri command validates its arguments as if they came from the network.
- Every command is declared in `apps/desktop/src-tauri/build.rs`, registered in
  `generate_handler!`, and granted by name in `capabilities/main.json` (`tauri.md`).
- No `core:default`, and no shell, fs, http, opener or updater plugin reachable from JS. A new
  plugin permission needs a reason in the capability's `description` and in the threat model.
- CSP: `default-src 'self'`, `script-src 'self'`, no `'unsafe-inline'`, no `'unsafe-eval'`, no
  remote origins. `devCsp` may relax styles for Vite HMR and nothing else.

## Untrusted input

Imported themes, vault files, sync payloads, config files and IPC arguments: **size limit first,
then parse, then validate, then build a fresh value.** Validation returns the validated type
(`VaultPath::parse`), so unvalidated data cannot reach the code that trusts it. Paths from outside
the core go through `VaultPath` before touching the filesystem.

## Cryptography

Any change touching keys, encryption, signatures, sync payloads or the server's view of data
follows `crypto.md` (path-scoped; read it in full first). Short form: no custom crypto, standard
primitives from reviewed crates, all of it in `tessera-core`, fail closed.

## Secrets and supply chain

- Never commit a key, token, `.env`, signing material or real note content. The pre-commit hook
  and CI run gitleaks; never bypass them (`--no-verify`). A check that a git hook is active tests
  `-x` on the file git will run: git skips a hook without the execute bit.
- Dependencies follow `dependencies.md`.
- GitHub Actions are pinned by commit SHA with the tag in a comment, and every workflow starts
  from `permissions: {}`.

## Change checklist

1. No new capability, command, plugin or CSP source without a reason and a threat-model edit
2. New commands validate input in Rust and are granted by name
3. Untrusted input is size-limited before parsing and validated before use
4. No secret, home-directory path or note content in logs, errors sent to the webview, fixtures,
   issues or commits
5. No network call or third-party request (`privacy.md`)
6. Crypto changes follow `crypto.md` and an accepted ADR
7. `cargo deny`, `cargo audit` and `pnpm audit` are green
