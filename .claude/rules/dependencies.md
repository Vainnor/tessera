# Dependency Rules (always loaded)

Prefer boring, well-maintained dependencies, and fewer of them.

## Before adding one

1. Can the standard library or an existing dependency do it in under ~50 lines? Then do that.
2. Run the `dependency-researcher` agent: maintenance, advisories, license, transitive weight,
   network or telemetry behavior, platform support. Crypto crates also go through
   `crypto-reviewer` (`crypto.md`).
3. **Ask the user before adding a heavy dependency** (a framework, a runtime, anything with a large
   transitive tree or native code) and before any new runtime dependency in the desktop bundle.
4. Record it with a one-line reason in `docs/adr/0001-repository-and-stack.md` → "Dependencies"
   in the same commit, and say why in the PR.

## Rules

- **Licenses**: permissive or MPL-2.0 only (`deny.toml` `[licenses] allow`; the dependency review
  workflow uses the same list). Anything else is an ADR.
- **Features off by default**: `default-features = false`, then only what is used.
- **Pinned and locked**: `Cargo.lock` and `pnpm-lock.yaml` are committed; CI installs with
  `--locked` / `--frozen-lockfile`. Never `cargo update` or `pnpm update` wholesale as a drive-by.
- **Release age**: pnpm refuses versions newer than 7 days (`minimumReleaseAge` in
  `pnpm-workspace.yaml`); Dependabot waits 7 days too. Cargo has no such gate: read the
  `Cargo.lock` diff of every update.
- **No install scripts**: pnpm runs no dependency lifecycle scripts unless listed in
  `onlyBuiltDependencies` with a reason.
- **Audits are gates**: `cargo deny check`, `cargo audit` and `pnpm audit --audit-level=high` stay
  green. An advisory ignore in `deny.toml` carries the id, the reason and a revisit date. A dev
  tool with unfixable advisories leaves the lockfile and runs pinned through `pnpm dlx` (as
  `@lhci/cli` does).
