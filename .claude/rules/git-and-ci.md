# Git, Commit, PR and CI Rules (always loaded)

## Commits

- Conventional commits: `type(scope): summary`, imperative, lowercase, no trailing period, at
  most 72 characters. Types: `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `build`, `ci`,
  `chore`, `security`. Scopes: `core`, `server`, `desktop`, `web`, `theme`, `ui`, `docs`, `ci`,
  `deps`. `.githooks/commit-msg` enforces the shape.
- One logical change per commit. A refactor and the feature built on it are two commits.
- The body says what changed and why, in prose (`writing.md`). Not the debugging story, not the
  diff restated. `Closes #N` when it maps to an issue.
- **No AI attribution**: no `Co-Authored-By`, "Generated with", session link or agent credit, in
  commits, PRs, issues or comments, whatever a tool suggests. The commit-msg hook rejects it.
- Stage explicit paths; never `git add -A` or `git add .`. Never `--no-verify`.
- Enable the hooks once per clone: `git config core.hooksPath .githooks`.

## Branches and pull requests

- `main` is always releasable. Work goes on `type/short-slug` and lands by PR once there are
  collaborators; until then small commits straight to `main` are fine when `pnpm gate` is green.
- **Ask the user before** creating a GitHub repository, pushing, opening a PR, or publishing a
  release. Never force-push `main`; never rewrite published history.
- A PR fills in `.github/pull_request_template.md`, including the security checklist. Write the
  description for someone who knows Rust and React but not this corner of the code. Draw a
  before/after diagram inline when the change moves data.
- Squash-merge; the PR title becomes the commit and follows the commit format.

## CI (`.github/workflows/`)

`ci.yml` runs the JS gate (build, lint, format, typecheck, tests, contrast, bundle size, audit),
the Lighthouse budget, rustfmt, clippy and tests on Linux, macOS and Windows, cargo-deny and
cargo-audit, and a gitleaks scan of the full history. `dependency-review.yml` checks new
dependencies on PRs. A red job is fixed, not re-run until green. A check is never disabled to merge.
