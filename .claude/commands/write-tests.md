---
description: Audit the branch's tests against its diff, then write the missing ones (core unit tests, server router tests, vitest, crypto rejection cases) and prove each can fail.
---

You write the tests the branch is missing. Analyze the diff, find what is tested and what is not,
then write tests that give real confidence the code works. `.claude/rules/testing.md` is the rule;
this command is the procedure. For crypto or sync code, read `.claude/rules/crypto.md` → "Testing"
in full first.

## What a test is for

A test exists to catch a bug before it ships. One that can't is worthless. Every test you write
answers yes to all five:

1. Would it fail if I introduced a bug in the implementation?
2. Does it test behavior (input → output or side effect), not implementation details?
3. Is it testing our code, not Tauri, axum, serde, Automerge, React or vitest?
4. Could I delete the implementation and watch it fail?
5. Is no other test already covering this exact case?

Never write:

- an `is_err()` or "it rendered" test where the behavior has a specific value (`testing.md`:
  assert the exact value, `matches!(e, Error::InvalidPath { .. })`)
- a tautology (the expected value computed by the code under test, or by copying its formula)
- an over-mocked test (everything faked, nothing real tested)
- a fixture derived from the constant under test (a note of `MAX_NOTE_BYTES + 1` bytes passes for
  every value of the constant; use absolute sizes either side of it)
- a test that touches the network, the real home folder or the clock (`testing.md`: inject paths
  and time; fixtures are synthetic)

Always cover:

- edge cases: empty input, zero, both sides of every limit, the largest accepted value
- every validation rule, one accepting and one rejecting case (`testing.md` → "What gets tested")
- every error variant a caller can see
- untrusted input in the order `security.md` → "Untrusted input" gives: over the size limit before
  parsing, malformed, valid-but-refused (`..`, an absolute path, a reserved Windows name)
- the wiring: the server route through the real router, the core function a Tauri command calls,
  not only a private helper beneath it

## Phase 1: Diff inventory

1. `git diff origin/main...HEAD --name-only`.
2. Sort the files and pick the test each gets:

   | Category                       | Where it lives                                      | Test it with                                                                                              |
   | ------------------------------ | --------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
   | Core logic                     | `crates/core/src/**`                                | a `#[cfg(test)] mod tests` beside the code, or a `tests.rs` submodule as `sync/history/` has              |
   | Core public API across modules | `crates/core/src/lib.rs` re-exports                 | an integration test in `crates/core/tests/`                                                               |
   | Vault and paths                | `crates/core/src/vault/`                            | unit tests on a temp directory the test creates (never the home folder)                                   |
   | Crypto and sync                | `crates/core/src/crypto*`, `crates/core/src/sync*`  | published vectors, round trips and one rejection test per tampering case (`crypto.md` → "Testing")        |
   | Server routes and config       | `crates/server/src/**`                              | `tower::ServiceExt::oneshot` through the real router; config parsing for every accepted and refused shape |
   | Tauri shell                    | `apps/desktop/src-tauri/src/**`                     | commands stay thin: test the core function they call; the shell itself is checked by launching the app    |
   | Desktop UI logic               | `apps/desktop/src/lib/**`, components with behavior | vitest beside the code (`*.test.ts(x)`)                                                                   |
   | Theme                          | `packages/theme/**`                                 | the validator, schema agreement, contrast, CSS output and `applyTheme`                                    |
   | UI components                  | `packages/ui/**`                                    | render with `react-dom/server`                                                                            |
   | Website                        | `apps/web/**`                                       | headers, claims (`test/features.test.ts`), `scripts/check-dist.ts`; Lighthouse in CI                      |
   | "X never happens"              | anywhere                                            | a source-scan guard test, proven by planting the forbidden call in a throwaway file                       |

3. For each category, find **what** changed by reading the code, not the file names.

## Phase 2: Audit what exists

1. For each changed source file, find its tests and read them in full.
2. For the tests that exist, judge them:
   - Do they assert the exact value or variant, or a weaker proxy?
   - Do they cover the rejecting side of every rule, not only the accepting one?
   - **Do they test the wiring?** A helper tested directly while the public function, route or
     command that calls it goes untested is the gap that ships: the fix lands in the helper, the
     caller never calls it, every test stays green.
   - Does a guard test exist that was proven by breaking the guard (`testing.md` → "How tests are
     written")?
3. Write a gap report: files with no tests, files with shallow tests, missing rejecting cases,
   missing error variants, missing wiring tests, and UI with behavior but no test.

If you only need the audit, dispatch `test-reviewer` instead and stop here.

## Phase 3: Write the tests

Copy the nearest existing test of the same kind; never invent a new harness.

### Core unit tests

In the module's own `#[cfg(test)] mod tests`, below the code (`rust.md` → "Gate":
`items_after_test_module`).

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_traversal() {
        assert!(matches!(
            VaultPath::parse("notes/../../etc/passwd"),
            Err(Error::InvalidPath { .. })
        ));
    }

    #[test]
    fn accepts_a_nested_relative_path() {
        let parsed = VaultPath::parse("a/b.md").map(|p| p.as_str().to_owned());
        assert_eq!(parsed.ok(), Some("a/b.md".to_owned()));
    }
}
```

Name the test for the behavior, not the function. Use absolute inputs and expected values worked
out by hand, with the arithmetic in a comment where there is any.

### Server routes

Drive the real router with `tower::ServiceExt::oneshot`; assert the status **and** the body, and
for a refused request that nothing changed.

### Crypto and sync

Published vectors for every primitive wrapped (RFC 8032, RFC 9180), a round trip, then one
rejection test per tampering case: a flipped ciphertext bit, wrong associated data, the wrong
generation, the wrong key, truncated input, a replay into another document or lineage. Never a
test helper that returns plaintext labeled as ciphertext (`crypto.md` → "Non-negotiable").

### Desktop UI and packages

vitest beside the code. Mock `apps/desktop/src/lib/ipc.ts` at its module boundary and assert the
call made and what the user sees, not component internals. `packages/ui` renders with
`react-dom/server`.

## Phase 4: Run and prove

1. Run each new test alone: `cargo test -p <crate> <name>` or
   `pnpm --filter <package> exec vitest run <path>`. Each passes. Read `test result:` (not
   `0 passed` from a filter typo) or vitest's `Tests` line (not "No test files found").
2. A failing test: read the implementation and decide whether the test or the code is wrong. Fix
   the test if the code is right; fix the code if the test found a bug, and say so.
3. **Break the guard.** For each new test, make the smallest plausible bug in the code it guards
   (flip a comparison, drop a size check, remove a match arm), run the test, watch it go red, then
   restore. `scripts/mutate.sh FILE OLD NEW -- COMMAND` with a narrow command (the one test) prints
   `RED` or `GREEN` and restores the file byte for byte. Mutating by hand, commit a checkpoint
   first and confirm the mutation applied (`git diff --quiet` fails). Never mutate while a build or
   suite runs in the same tree. A test that stays green under a plausible mutation is decoration:
   strengthen it or delete it, and mention each proof in the PR (`testing.md`).
4. A change with a screen: launch it (`pnpm dev:desktop`, `pnpm dev:web`) and exercise what
   changed.
5. `cargo fmt --all` and `pnpm exec prettier --write` on the files you touched, then `pnpm gate`
   once the tests are committed (in the background, output to a file). Never run it again on a SHA
   that already has a tests marker or green CI.

The work is not done until the gate is green and, for a UI change, the screen has been launched.

## Rules

- Never mock the thing you are testing.
- Never touch the network, the real home folder or the clock.
- Never change an expected value to make a test pass; find out whether the test or the code is
  wrong.
- Repetition in tests is fine; a shared helper beyond what the module already has is not.

## Priority

Write tests in this order, highest impact first:

1. Untrusted input and its limits (paths, imports, IPC arguments, sync payloads, server requests)
2. Crypto rejection cases
3. The `tessera-core` public API, every validation rule both ways and every error variant
4. Server routes through the router
5. UI behavior

## Output

- The number of tests written, by layer
- The gaps addressed, against the Phase 2 report
- Each mutation and whether it went red
- Any bug found while writing them, and whether you fixed it
- The targeted runs' summary lines, the `pnpm gate` result (or the marker or CI it relied on), and
  the screen launched or "no UI change"
