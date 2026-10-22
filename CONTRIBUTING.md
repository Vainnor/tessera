# Contributing to Tessera

Thanks for your interest. Tessera is in early development; the foundations exist and most
features do not yet. Read [`docs/architecture.md`](docs/architecture.md) first.

## Ground rules

- **Privacy is a feature.** No telemetry, analytics, crash reporting or third-party requests, in
  the app or the website. A pull request that adds any of these will be closed.
- **No custom cryptography.** Crypto changes follow [`docs/adr/`](docs/adr/) and
  [`.claude/rules/crypto.md`](.claude/rules/crypto.md), and need a design discussion first.
- **Security reports go through private reporting**, never a public issue
  ([`SECURITY.md`](SECURITY.md)).
- Be kind. The [Code of Conduct](CODE_OF_CONDUCT.md) applies everywhere.

## Setup

You need:

- Rust (the version in `rust-toolchain.toml` installs itself through `rustup`)
- Node.js 24 or newer and pnpm 12 (`corepack enable` gives you the pinned version)
- Tauri's system dependencies for your OS: <https://v2.tauri.app/start/prerequisites/>
- Optional: `cargo-deny`, `cargo-audit` and `gitleaks` to run the supply-chain and secret checks
  locally

```sh
git clone https://github.com/Vainnor/tessera.git
cd tessera
git config core.hooksPath .githooks   # commit-message check and local secret scan
pnpm install
pnpm build                            # theme tokens, desktop frontend, website
cargo build --workspace
```

The commands for running each app are in the [README](README.md#development).

## Before you open a pull request

Run the full gate. It is what CI runs:

```sh
pnpm gate
```

It runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo deny`,
`cargo audit`, ESLint, Prettier, the TypeScript checks, the JS tests, the theme contrast check,
the builds and the bundle-size budgets. A check is never disabled to make a pull request pass.

## Commits and pull requests

- Conventional commits: `type(scope): summary`, imperative, at most 72 characters. Types: `feat`,
  `fix`, `refactor`, `perf`, `test`, `docs`, `build`, `ci`, `chore`, `security`.
- One logical change per commit.
- Fill in the pull request template, including the security and privacy checklist.
- A new dependency needs a reason in the pull request and an entry in
  [`docs/architecture.md`](docs/architecture.md#dependencies).

## Licensing of contributions

Tessera is licensed under the GNU Affero General Public License v3.0 or later. By submitting a
contribution you agree that it is licensed under the same terms (inbound = outbound). There is
no CLA.
