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
  `is_err()`). A message that names a path is asserted with the full resolved path
  (`$(cd "$X" && pwd -P)/...`), not its last component, which a wrong directory also ends with.
- Every user-visible message the diff adds or rewords gets an assertion, summary lines included,
  and so does each value of an interpolated reason, not only the new one.
- No network, no real home folder, no clock: inject paths and time. Fixtures are synthetic.
- A test of path resolution builds its own symlink (`ln -s`) in the fixture. macOS's `$TMPDIR`
  sits behind `/var` → `/private/var` and Linux's does not, so a test leaning on it proves the
  behavior on one host only. Re-run its mutant with `TMPDIR="$(cd "$TMPDIR" && pwd -P)"`.
- Code that branches on a host tool (`timeout(1)`): the tests of the capable branch run only
  where the tool exists and print a `SKIP` line naming what did not run; one case always runs the
  other branch with a `PATH` built without the tool.
- A stub proves a timeout only for programs that die on the stub's signal. Go ignores `SIGALRM`,
  so perl's `alarm` never stops `gh`; a timeout test's stub ignores the signal the real program
  ignores (`trap '' ALRM`).
- **A guard test is proven once by breaking the guard and watching the test fail**, before you
  trust it. Mention it in the PR. Run the mutation before recording a guard as untestable: an
  existing assertion may already kill it.
- A test that needs `unsafe`, a new harness or a dependency is the user's decision. An ordering
  only memory can observe ("checked before it allocates") is structural; say so in the PR.

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
