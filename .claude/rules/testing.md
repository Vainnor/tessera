# Testing Rules (always loaded)

## What gets tested

- **`tessera-core`**: unit tests beside the code for every public function and every validation
  rule (one accepting and one rejecting case per rule), and every error variant a caller can see.
  Integration tests in `crates/core/tests/` drive the public API across modules once they exist.
- **The server**: routes through the real router (`tower::ServiceExt::oneshot`), config parsing
  for every accepted and rejected shape.
- **The desktop shell**: commands stay thin; their logic is core and tested there. The UI has
  vitest tests for `src/lib/**` and for components with behavior.
- **Packages**: `packages/theme` tests the validator, the schema agreement, contrast, CSS output
  and `applyTheme`; `packages/ui` renders components with `react-dom/server`.
- **The website**: headers, claims (`test/features.test.ts`) and the build checks in
  `scripts/check-dist.ts`; Lighthouse in CI.
- **Crypto** has its own requirements: `crypto.md` → "Testing".

## How tests are written

- The name states the behavior: `rejects_parent_traversal`, not `test_path_3`.
- Assert the exact value, not a weaker proxy (`matches!(e, Error::InvalidPath { .. })` over
  `is_err()`).
- No network, no real home folder, no clock: inject paths and time. Fixtures are synthetic.
- **A guard test is proven once by breaking the guard and watching the test fail**, before you
  trust it. Mention it in the PR.

## Reading results

- Read the summary line of every runner (`test result:` per Rust binary, vitest's `Tests` line),
  not the exit code. A sharply lower count means a binary failed to build or a filter excluded
  too much.
- Say which tiers ran. Other-OS CI jobs, the GUI and Lighthouse do not run in a quick local loop;
  say so rather than implying they passed. `pnpm gate` runs everything that can run locally.

## Reproducing a Linux CI job locally

A Docker Desktop container is not the `ubuntu-24.04` runner, and a script can pass in one and fail
on the other. Before calling a Linux CI change reproduced, match the runner: a non-root user, the
runner's tools installed (zip, rsync, jq), no git identity (a test that commits sets its own), and
`TMPDIR` on an ext4 loop mount, since the container's overlayfs does not hand out inode numbers
the way the runner's ext4 does. ext4 gives a freed inode number to the next new file, so a test
that compares a file's inode before and after a rewrite holds the old file open across the call
(`exec 9<file`), as `scripts/lib/marker-audit.test.sh` does.
