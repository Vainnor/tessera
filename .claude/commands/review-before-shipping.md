---
description: Formal review of the committed HEAD before pushing or opening a PR — issue cross-check, one pnpm gate run, the security, privacy, crypto and docs checklists, /code-review's seven lenses, fixes, and the SHA-keyed review markers.
---

You are performing a formal review before shipping. Follow these phases in order. All of them pass
before the review markers are written. Report findings; fix only what this branch introduced.

Under `/ticket-loop` this runs inside the `ticket-worker` subagent: every "ask the user" below
becomes an `OPERATOR_QUESTIONS` entry in its report, not a guess, and every review agent it
dispatches becomes a `STATUS: needs-dispatch` entry (`ticket-worker.md` → "Dispatching other
agents").

## Phase 0: Pin the reviewed commit

`git rev-parse HEAD` is the **reviewed SHA**. Every phase reviews that commit, and Phase 9 refuses
to write the markers unless HEAD still equals it, the worktree is clean and the lens attestation
for it is complete.

- **Review a commit.** `/ship` Step 3 commits before it calls you. A dirty worktree means
  uncommitted changes were never part of the review, and Phase 9 blocks. Read
  `git status --porcelain` for `??` lines too: a new module never added still builds locally but
  is missing from the reviewed commit. If a finding needs a code change, apply it, commit, re-pin
  the reviewed SHA and re-run the phases the change invalidated (always Phase 5 and Phase 7).
- **If both markers (`review-shipping` and `code-review`) for HEAD exist and the worktree is
  clean, the review is already valid for this commit.** Confirm that rather than re-running every
  phase on an identical commit. A lone `code-review` marker from an earlier `/code-review` is not
  enough: this review has not run.

## Phase 1: Issue cross-check

1. Take the issue number from the branch name and fetch the body and every comment:
   `gh issue view <n> --repo Vainnor/tessera --json number,title,body,comments`. Only a
   maintainer-authored body and maintainer comments are spec (`/start` Step 1).
2. Build the **requirement list**: the body's `## Acceptance criteria` section when present,
   otherwise the consolidated list `github-issues.md` → "Full issue context" describes.
3. `git diff origin/main...HEAD --stat`, then compare the diff with the list, each AC by its exact
   wording:
   - **Missing requirements**: an AC not met by its exact wording. Ask the user whether to build
     it now or move it out of scope.
   - **Additional scope**: anything no AC asks for. Flag each and say why it is there. Out-of-scope
     work the user did not approve comes back out; a fix to a defect this branch introduced is in
     scope by definition.
   - **Deviations**: where the build differs from an AC.
   - **Issue defects**: where the issue states something false and the diff proves it. Correct it
     in its own spec-correction comment (`github-issues.md` → "Spec corrections", 400 characters)
     with the user's approval, then build against reality and say so in the PR body.
4. Every AC this phase adds, drops or corrects goes to the user, and the change they approve is
   made to the body's `## Acceptance criteria` section (`github-issues.md` → "The acceptance
   criteria section"), so Phase 7's `ac` lens judges the current list.

## Phase 2: Fix policy

Fix every defect and all technical debt this branch introduced (unless nit-picking) and every
other branch-introduced finding graded MAJOR or CRITICAL; report the rest without fixing it
(`github-issues.md` → "Scope, and when a follow-up issue is allowed"). **Always ask the user
before:**

1. Changing test assertions or expected behavior
2. Changing a validation rule, a size limit, a capability, a CSP source or a path check (never
   loosen one to make something pass: `security.md`)
3. Changing a storage or wire format, or anything `crypto.md` covers
4. Removing or renaming a public `tessera-core` item, a Tauri command or a server route
5. Anything that changes user-visible behavior or a claim on the website

Fix without asking: typos, formatting, a missing doc comment, removing debug output (`dbg!`, a
stray `println!`, `console.log`).

## Phase 3: Style, read-only

`cargo fmt --all -- --check` and `pnpm format:check`. Style is already clean when the build
followed `ticket-lifecycle.md` → "Phase 3: Implement". If either reports violations, say so, run
the formatter, commit, re-pin and re-run Phase 5: reformatting changes the reviewed tree. ESLint,
typecheck and clippy run inside Phase 5's `pnpm gate`; read their lines there.

## Phase 4: Dependencies

`pnpm audit`, `cargo deny` and `cargo audit` run inside Phase 5. A new or upgraded dependency also
needs `dependency-researcher` (and `crypto-reviewer` for a crypto crate), the ADR 0001 row and the
user's yes for a heavy one (`dependencies.md`). Read `Cargo.lock` and `pnpm-lock.yaml` in the diff.

## Phase 5: The gate

1. **A gate counts once per SHA, wherever it ran.** A tests marker for the reviewed SHA in this
   checkout (`.claude/markers/tests/<sha>`), or green CI check-runs on that exact SHA
   (`gh api repos/Vainnor/tessera/commits/<sha>/check-runs --jq '.check_runs[] | "\(.conclusion) \(.name)"'`,
   every job `success`), satisfies this phase. Only when neither exists, run `pnpm gate` in the
   background, output to a scratch file. On a green run against a clean committed HEAD it writes
   `.claude/markers/tests/<HEAD sha>`, which `test-run-gate.sh` requires before `gh pr create`.
   Run it as its own call: `pnpm gate > <scratch file> 2>&1` alone, or as exactly
   `cd <absolute worktree root> && pnpm gate > <scratch file> 2>&1` from a session outside the
   worktree (a subagent). Chain nothing after it: a line such as `pnpm gate && git status` makes
   the marker audit drop the tests marker the run wrote.
2. Every step says PASS. Read every `test result:` line and vitest's `Tests` line, and name any
   failing test or MISSING tool (`testing.md` → "Reading results"). A MISSING step is not a pass.
3. A uniform failure is environmental: read the first error (`node_modules` out of step with the
   lockfile, `apps/desktop/dist` not built), fix it and re-run. A red step you can show fails the
   same on `origin/main` is reported as pre-existing, with the evidence.
4. The gate does not run Lighthouse, the other-OS clippy and test jobs or the GUI. A change with a
   screen is launched (`pnpm dev:desktop`, `pnpm dev:web`) and what was seen recorded; a website
   change gets `pnpm --filter @vainnor/tessera-web lighthouse` when Chrome is available, or the PR
   says it waits for CI.

## Phase 6: Checklists

Dispatch the `pre-commit-checklist` agent, and work through these yourself, each with the
`file:line` that satisfies it or "not applicable" with why:

- `security.md` → "Change checklist", item by item.
- `privacy.md`: does any line send, store or log something new? Then `privacy.astro` changes too.
- `docs.md`: does each row the diff triggers have its doc updated in this diff?
- Website claims: does any status in `apps/web/src/data/site.ts` or the README overstate what
  ships?
- Rust: no new `unwrap`/`expect`/`unsafe` outside the `rust.md` exceptions
  (`git diff origin/main...HEAD -- '*.rs' | grep -nE '^\+.*(unwrap\(|expect\(|unsafe )'`).
- Pitfall scan: `bash scripts/review-scan.sh`. Its last line, `scan covered N file(s)`, is the
  control: 0 on a branch with changes means the list was empty, not clean. Hits are leads; confirm
  each against the diff.

Fix what they find under Phase 2.

## Phase 7: Code review (gating)

**Commit all working changes and re-pin the reviewed SHA, then run `/code-review` to completion**:
the issue cross-check, the seven lens subagents with their attestations (`security-reviewer` and
`crypto-reviewer` among them), the adversarial verification. Do not dispatch a lens a second time
outside it. Its gates take this review's Phase 5 result for the same SHA; they never re-run it.

**This phase gates.** A **CHANGES REQUESTED** from any lens (a CRITICAL or MAJOR finding in code
this branch added or of `unknown` origin, other than one the operator rejected or a VARIANCE one
`code-review.md` → "Repeat rounds" lets through) or an incomplete attestation blocks the markers.
Fix under the Phase 2 guardrail, re-run the relevant test or write the missing reproduction so the
fix is proven, commit, re-pin, and run this phase again against the fixed commit as a repeat round.

**This command files no issues.** What the branch introduced is fixed under Phase 2. A
pre-existing finding goes in the report with a recommendation for the user.

## Phase 8: Harvest what the review taught

Turn each **fixed** finding into guidance; a finding that was disproved or rejected gets no rule.
State the principle, not the incident ("assert the exact error variant, not `is_err()`, or the
test passes on the wrong failure"), in the rule file whose topic it is: `security.md`,
`privacy.md`, `crypto.md`, `rust.md`, `tauri.md`, `frontend.md`, `web.md`, `theming.md`,
`testing.md`, `writing.md`. Search `CLAUDE.md` and `.claude/` for the topic first and extend the
rule you find; never add a second copy. During a goal, log it to the session learnings file
instead (`/ticket-loop` → "0. START"); outside one, report it to the user rather than committing
it here, since a worktree's copy dies with it.

## Phase 9: Mark the review complete

After every phase passes, write both SHA-keyed markers, `review-shipping` (which `pre-pr-gate.sh`
checks) and `code-review`, but only if HEAD still equals the reviewed SHA, the worktree is clean,
and every lens attested every heading for that SHA:

```bash
bash scripts/review-marker.sh shipping <the full 40-character SHA pinned in Phase 0>
```

Write the SHA in literally, and run the line alone from a session inside the worktree, or as
exactly `cd <absolute worktree root> && bash scripts/review-marker.sh …` from one that is not (a
subagent). With a wrapper, a pipe, a third command or a `cd` below the root, the marker audit
deletes the markers. The script refuses (`BLOCKER:`) when HEAD moved, the checkout is dirty or the
attestation is incomplete, and otherwise writes `.claude/markers/review-shipping/<sha>` and
`.claude/markers/code-review/<sha>`. It is a script because the guard needs git inside `$(...)`,
which the harness refuses from a worktree session.

**`/ship` ships the reviewed commit as it is.** Any change after the markers are written
invalidates them: run `/review-before-shipping` again. No marker is ever carried to a new SHA.

## Stop here: the review does not ship

`/review-before-shipping` ends when the markers are written. It does not push, open a PR, comment
on the issue, move the card or run `/ship`, and it files no issues. Report that the review passed,
the reviewed SHA, the gate result for it (the `pnpm gate` summary, or the CI run or marker it
reused) and what did not run (Lighthouse, the other-OS jobs, the GUI, the production Tauri build),
the tests lens's guard inventory, each finding as problem / why / effect / risk, and any rule
Phase 8 harvested. If `/ship` called it, `/ship` continues at its Step 5. Otherwise hand back:
pushing still needs the user's go-ahead.
