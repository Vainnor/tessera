## What and why

<!-- What changes, and the reason. Link the issue: Closes #N -->

## How it was verified

<!-- Commands run (pnpm gate), what did not run (other OSes, the GUI, Lighthouse), screenshots for UI. -->

## Checklist

- [ ] `pnpm gate` passes on the committed HEAD
- [ ] Docs updated per `.claude/rules/docs.md`
- [ ] New dependencies are justified here and in `docs/architecture.md` § Dependencies

### Security and privacy (`.claude/rules/security.md`)

- [ ] No new capability, plugin, Tauri command or CSP source (or justified here and in `docs/threat-model.md`)
- [ ] New commands validate their input in Rust and are granted by name
- [ ] Untrusted input (imports, files on disk, webview arguments, network) is size-limited and validated before use
- [ ] No telemetry, analytics, network call or third-party request added
- [ ] No secrets, personal paths or note content in code, logs, fixtures or this PR
- [ ] Crypto changes follow `.claude/rules/crypto.md` and an accepted ADR (or: no crypto touched)
