---
description: Ship the current issue — targeted tests, commit, review the commit, push, open the PR against main, Moment 3, and move the card to Post build.
argument-hint: "[issue number]"
---

You are wrapping up the current work and shipping it. Follow these steps in order and skip none.

The order is deliberate: **commit first, then review the committed commit, then push and open the
PR.** The review marker is keyed to a commit SHA (Step 4), so what you review is exactly what
ships. Reviewing a dirty worktree and committing afterwards ships a commit nobody reviewed.

## Step 1: Run the tests the change reaches

1. Choose the runs by what the change can **reach**, not the directory it sits in. A change to
   `tessera-core`'s public API reaches the desktop shell and the server; a `.claude/**` or docs
   change reaches no code and needs no test run beyond Prettier.
2. Run them targeted: `cargo test -p <crate>` per crate the change reaches,
   `pnpm --filter <pkg> exec vitest run` per package, `pnpm typecheck` after a change to
   `apps/desktop/src/lib/types.ts` or a shared package. Do not run `pnpm gate` here: the tree is
   not committed, so it stamps nothing, and `/review-before-shipping` runs it once on the committed
   HEAD. **Read the summary lines, not the exit code** (`testing.md` → "Reading results").
3. A change with a screen (the desktop UI, the website, `packages/ui`): launch it from the
   worktree (`pnpm dev:desktop` or `pnpm dev:web`) and record what you did and saw. A change with
   no screen says so in the PR.
4. Fix and re-run until green. Do not go on until green.

## Step 2: Look for missing tests

1. `git diff origin/main...HEAD --stat`. Always `origin/main` and three dots.
2. For each changed source file, check `testing.md` → "What gets tested": every public
   `tessera-core` function and validation rule has an accepting and a rejecting case, every error
   variant a caller can see is tested, server routes go through the real router, `src/lib/**` and
   components with behavior have vitest tests, crypto has what `crypto.md` → "Testing" lists.
3. Write what is missing (`/write-tests` helps) and re-run the affected tests.

## Step 3: Commit

Committing to the feature branch is not shipping: the push and PR wait for sign-off, the local
commit does not. Review needs a committed SHA for its marker (Step 4), so commit first.

1. The branch name matches `{type}/<n>/{desc}` (at most 50 characters, `ticket-lifecycle.md` →
   "Branch naming"), never `worktree-*`, which `/start` should already have renamed. **Never
   commit on `main`: you work in the issue's worktree.** On `main`, stop and ask.
2. Stage changed files by **explicit path** (never `git add -A`, `git add .` or `git add --all`),
   then commit: `type(scope): summary` (`git-and-ci.md` → "Commits"), a body in prose
   (`writing.md`), `Closes #<n>`, and no trailer or attribution of any kind, whatever a tool
   suggests. `.githooks/commit-msg` refuses one.
3. **Diff integrity after the commit**:

   ```bash
   git diff origin/main...HEAD --name-only
   git status --short
   ```

   The first lists **every** file you meant to ship and nothing else. The second shows none of
   them as still modified. A pre-commit refusal (gitleaks) leaves nothing committed, and a
   formatter run after staging leaves its fixes unstaged: stage and commit the rest before going
   on.

4. **Attribution over every commit the PR carries.** Two plain commands, the file in your
   scratchpad:

   ```bash
   git log --format='%an <%ae>%n%cn <%ce>%n%B' origin/main..HEAD > <scratchpad>/commits.txt
   bash -c '. <primary-checkout>/.claude/hooks/lib/attribution.sh; attribution_hits <scratchpad>/commits.txt; echo "rc=$?"'
   ```

   `rc=1` is clean; `rc=0` printed the offending lines (reword those commits); `rc=2` means the
   check did not run. Every author and committer line is the user.

## Step 4: Review the committed HEAD

Run `/review-before-shipping` through every phase. It reviews the committed HEAD and, on passing,
writes the **SHA-keyed** markers `.claude/markers/review-shipping/<HEAD sha>` (checked by
`pre-pr-gate.sh`) and `.claude/markers/code-review/<HEAD sha>`.

**If the review applies a fix**, the fix is a new commit, HEAD moves, and the markers are for the
old SHA. Commit the fix, repeat the Step 3 integrity check and run `/review-before-shipping` again
so it stamps the new HEAD. Never copy a marker to a new SHA, and never write one here.

## Step 5: Check the reviewed commit, then push

**Ask the user before pushing** (`CLAUDE.md` → "Standing rules"), unless `/ticket-loop`'s goal
already lifted it.

1. **Ship the reviewed commit as it is.** HEAD is the SHA `/review-before-shipping` reviewed and
   `git status --porcelain` prints nothing. Anything changed since means running the review
   again; never commit it here.
2. **Confirm the markers, the attestation and the gate for HEAD**:
   `.claude/markers/review-shipping/<sha>` and `.claude/markers/code-review/<sha>` exist, and
   `bash scripts/lib/review-attestation.sh check` prints `attestation complete`. The gate counts
   once per SHA, so either proof satisfies it: `.claude/markers/tests/<sha>` in this checkout, or
   green CI check-runs on that exact SHA
   (`gh api repos/Vainnor/tessera/commits/<sha>/check-runs --jq '.check_runs[] | "\(.conclusion) \(.name)"'`,
   every job `success`). With neither, run `pnpm gate`. This holds on rework too, where the PR
   exists and `pre-pr-gate.sh` does not fire.
3. `git push -u origin {branch-name}`.
4. Check the push against the remote itself, not the local tracking ref:

   ```bash
   git ls-remote origin refs/heads/{branch-name}
   git rev-parse HEAD
   ```

   The two SHAs match. A push has failed silently before; nothing below happens until they do.
   Chain later steps on the push with `&&`, never `;`.

## Step 6: Open the PR

**Ask the user before opening a PR** (`CLAUDE.md` → "Standing rules"), unless the goal lifted it.
`pre-pr-gate.sh` refuses `gh pr create` without the `review-shipping` marker and a complete lens
attestation for the PR's head commit, and `test-run-gate.sh` without the tests marker or green
CI. Fill in `.github/pull_request_template.md`, security checklist included, for someone who knows
Rust and React but not this corner of the code, with the before/after diagram `git-and-ci.md`
asks for when the change moves data, and put the tests lens's guard inventory in it:

```bash
gh pr create --repo Vainnor/tessera --base main --title "type(scope): summary" --body-file <scratchpad>/pr.md
```

The title follows the commit format: it becomes the squash commit. The body starts:

```markdown
#<n> [short summary] (<board column>)

Closes #<n>
```

then the template's sections, with a test plan that lists what actually ran on this branch (the
targeted runs and their summary lines, `pnpm gate` on <sha>, the screens launched) and every gate
that did **not** run (Lighthouse, the other-OS jobs, the GUI) and why.

No attribution line of any kind; check the file with
`! grep -qiE 'Drafted by|Generated with|Co-Authored' <scratchpad>/pr.md` before creating it.
**Read the PR number from `gh pr create`'s output**; never predict it. If the branch is stacked on
another open PR, say so on the body's first line.

**Rework on an open PR:** skip `gh pr create`; the Step 5 push already updated it. Read its number
with `gh pr view {branch} --repo Vainnor/tessera --json number --jq .number`. Once a branch is
pushed, bring in `origin/main` by merging, never rebasing.

Read the PR head's check-runs:
`gh api repos/Vainnor/tessera/commits/<sha>/check-runs --jq '.check_runs[] | "\(.conclusion) \(.name)"'`.
An empty list right after a push means CI has not started, not green. Report them as they stand,
pending included; don't poll in a loop (the API budget is shared with the board). Under the ticket
loop, `ticket-worker.md` → "Phase D: After /ship" waits for them.

**Under the ticket loop, post the worker's evidence comment on the PR straight after it opens,
before the `Post build` move** (`ticket-worker.md` → "Review it as ticket-reviewer will"),
including "Operator decisions and rejected findings": copy
`<git common dir>/review-decisions/<n>.md` into it, confirm the comment posted, then delete the
file. From then on the PR comment is the record.

## Step 7: Moment 3, the verification comment and the board move

**First, fetch the issue body's `## Acceptance criteria` section again and compare it with lens
`ac`'s attestation for HEAD** (`.claude/markers/attest/<HEAD>/ac.md`): same count, same text. Any
difference means an AC changed since the review: **stop** and run `/review-before-shipping` again.

Follow `.claude/rules/github-issues.md` → "Moment 3: Work Complete". Before posting, cut every line
of the draft that names test machinery (cargo, vitest, clippy, `pnpm gate`, mutation proofs), a
source or test file path, branch or worktree mechanics, a note to a code reviewer rather than to
whoever checks the behavior, or an incidental tidy-up: those belong in the PR. Post it with its
length and attribution checked as a gate, the body file in your scratchpad:

```bash
python3 -c 'import sys; n = len(open(sys.argv[1]).read().rstrip()); print(n); sys.exit(n > 1200)' <scratchpad>/moment3.md \
  && ! grep -qiE 'Drafted by|Generated with|Co-Authored' <scratchpad>/moment3.md \
  && gh issue comment <n> --repo Vainnor/tessera --body-file <scratchpad>/moment3.md
```

A skipped `gh` call is not a posted comment: read the issue back. Then move the card to
**`Post build`**:

```bash
bash scripts/board-status.sh <n> "Post build"
gh issue view <n> --repo Vainnor/tessera --json projectItems,assignees \
  --jq '{status: [.projectItems[] | select(.title == "Tessera Kanban") | .status.name], assignees: [.assignees[].login]}'
```

Not `Done` or `Blocked`: those are the user's. Read the status back before reporting the move, and
confirm the issue is assigned to the user. Merging the PR closes the issue, and the board's
built-in workflow moves a closed issue to `Done`.

## Step 8: Tear down the worktree

The branch is on `origin` and the PR is open, so the local worktree is disposable. Review feedback
later resumes with `/start <n>`, which rebuilds a worktree from the pushed branch.

1. **Confirm nothing exists only locally**, from inside the worktree. All of these hold, or stop
   and report instead of tearing down:

   ```bash
   git status --porcelain                          # prints nothing
   git rev-parse --abbrev-ref HEAD                 # the branch
   git rev-parse HEAD                              # equals the next line
   git ls-remote origin refs/heads/<branch>
   ```

   Separate plain commands, the values written literally. Record the worktree directory name
   (`basename "$PWD"`) and the branch name for sub-step 3.

2. **Run `ExitWorktree` with `action: "remove"` and `discard_changes: true`.** It refuses without
   `discard_changes` whenever the branch has commits not on the default branch, which a shipped
   branch always does; sub-step 1 proved those commits are on `origin`. Its PreToolUse hook
   (`.claude/hooks/worktree-teardown.sh`) runs `scripts/teardown-worktree.sh`, which removes the
   directory and deletes the local branch. If the hook blocks, read its message, fix the cause and
   retry; never remove the directory by hand.

   If `ExitWorktree` reports no active worktree session (the worktree came from an earlier
   session), run the teardown from the primary checkout instead:
   `bash scripts/teardown-worktree.sh <dir-name>`.

3. **Check from the primary checkout**: evidence, not the tool's message.

   ```bash
   ls -d .claude/worktrees/<dir-name> 2>/dev/null && echo "DIR STILL PRESENT" || echo "dir removed"
   git branch --list "<branch>"                     # empty
   git worktree list                                # not listed
   git rev-parse --abbrev-ref HEAD                  # main
   git pull --ff-only origin main
   ```

   Anything left over: run `bash scripts/teardown-worktree.sh <dir-name>` (idempotent) and check
   again. The remote branch and the PR are left alone on purpose.
