---
paths:
  - "scripts/*worktree*.sh"
  - "scripts/board-status.sh"
  - "scripts/review-marker.sh"
  - ".githooks/**"
  - ".claude/hooks/*branch*.sh"
  - ".claude/hooks/*protection*.sh"
  - ".claude/hooks/issue-*.sh"
  - ".claude/hooks/pre-pr-gate.sh"
  - ".claude/hooks/rules-on-demand.sh"
  - ".claude/commands/start.md"
  - ".claude/commands/ship.md"
  - ".claude/commands/cleanup.md"
  - ".claude/commands/ticket-loop.md"
  - ".claude/agents/ticket-*.md"
---

# Issue Lifecycle (not loaded at startup; read in full before starting an issue, entering a worktree, or reviewing, shipping or handing back work)

From issue pickup to merged PR. GitHub interactions are defined in `github-issues.md`; the review
and ship mechanics live in `.claude/commands/`. This file is the order they run in, and it does
not restate either.

Work reaches `main` only by a PR from an issue branch (`git-and-ci.md` → "Branches and pull
requests"). The primary checkout stays on `main` and is never switched; each issue is built in its
own worktree.

---

## Phase 1: Start the issue

When the user says "do #123" or "start #123" (`/start`):

1. **Moment 1 (FIRST)**: `github-issues.md` → Moment 1. Fetch full context (body **and** all
   comments; only a team-authored body and team comments are spec), check nobody else holds it
   (below), move the board item to **In build**. Do this before any branch or test work so the
   board reflects reality immediately. Cache the consolidated requirements for later phases.
2. **Create and provision the issue worktree**: `/start` Step 2. The branch is
   `{type}/<n>/{desc}` on `origin/main`, in `.claude/worktrees/{type}-<n>-{desc}`, provisioned by
   `scripts/setup-worktree.sh`, which must exit 0. If a branch for this issue already exists
   locally or on the remote, say so and let the user choose whether to resume it.
3. **Validate the baseline.** A gate counts once per SHA: green CI on the `origin/main` commit you
   branched from is the baseline (`github-issues.md` → "CI is the verdict"). Otherwise run the
   tier the change can reach (below), and read the summary line rather than the exit code
   (`testing.md` → "Reading results"). Classify any red as pre-existing (red on `origin/main`) or
   introduced **before** starting work. Do not enter plan mode over an unexplained red baseline.
4. **Enter plan mode** and present the plan for approval. The plan names which rule files the work
   triggers (`CLAUDE.md` → "Rules"), whether it crosses a trust boundary (`security.md`), and
   whether it touches crypto (then `crypto-reviewer` reviews before any commit, `crypto.md`).

**Tiers.** Tier a run to what the change can reach, not to the directory it lives in:

| Tier  | When                                                                                    | Runs                                                                                      |
| ----- | --------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Light | docs, `.claude/**`, comments only                                                       | `pnpm format:check`, plus any script's own tests                                          |
| Rust  | `crates/**` or `apps/desktop/src-tauri/**`                                              | `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test -p <crate>` |
| JS    | `apps/**/src`, `packages/**`                                                            | `pnpm --filter <pkg> test`, `pnpm typecheck`, `pnpm lint`                                 |
| Full  | `tessera-core`'s public API, a Tauri command or capability, the CSP, crypto, a lockfile | `pnpm gate`                                                                               |

### Check nobody else holds the card

The board column does not prove a card is free. Parallel sessions claim within minutes, sometimes
before any branch is pushed. Before claiming, and again right before pushing:

- `git fetch origin` and `git branch -r --list "*<n>*"`: a pushed `…/<n>/…` branch with no PR is
  someone mid-cycle. Not holders: a ticket-reviewer's `chore/<n>/review-*` branch, and, on a card
  returned to `To Do`, the issue's own open PR and its head branch, which are the work to recover.
  The branch and worktree of parked work are also the work to recover, once the user confirms it
  was parked.
- `git branch --list "*/<n>/*"` and `git worktree list` in the primary checkout: a local branch or
  worktree means someone owns it, even with no remote branch and no card move.
- `gh pr list --repo Vainnor/tessera --state all --search "<n> in:body"`: a combined branch can
  carry another issue's number.
- Re-read **that card's** own status and its latest **team** comment right before every move. A
  listing even a few minutes old can overwrite someone else's move. If the card is already ahead
  of where you left it, someone else moved it; never move it backwards.

If someone else holds it, read their branch and adopt or extend it, or pick another card. Never
force-push or delete another session's branch without asking.

## Phase 2: Plan approved

1. **Moment 2**: `github-issues.md` → Moment 2. One comment, 600 characters, no attribution of
   any kind. The full plan goes in the PR, not the issue.
2. Exit plan mode and begin Phase 2.5.

## Phase 2.5: Write tests first

Before any implementation code:

1. Dispatch the `tdd-planner` agent with the approved plan and the consolidated issue context.
2. It produces skeletons: Rust `#[ignore = "todo: <requirement>"]` tests with `todo!()` bodies
   beside the code (`testing.md`), and vitest `it.todo` entries for `src/lib/**` and components
   with behavior. A crypto change's skeletons include its rejection cases (`crypto.md` →
   "Testing"). A UI change also gets its GUI check written into the plan; it runs last.
3. Run the skeletons targeted. They should compile and report as ignored or todo, not erroring.
4. Implement, making them pass one at a time.
5. Zero `todo` skeletons remain when the phase closes, and every acceptance criterion maps to at
   least one test.

## Phase 3: Implement

1. Study the existing pattern before writing the new one. Match it exactly.
2. Write the test alongside the source file, not in a later phase.
3. **A new Tauri command touches four places** (`tauri.md`); a new dependency goes through
   `dependency-researcher` and is asked about first (`dependencies.md`); a doc that disagrees with
   the code is fixed in the same change (`docs.md`).
4. **Format before the final run**: `cargo fmt --all` and `pnpm format` once implementation is
   done, _then_ run the suites. Reformatting after a green run invalidates it.
5. Keep the task list current. New work found mid-task goes on it immediately.

## Phase 4: Pre-completion review

1. **Run `/code-review` yourself** before telling the user the work is ready. On an uncommitted
   tree it is report-only and stamps nothing. Fix what it surfaces. The SHA-keyed markers `/ship`
   checks come from `/review-before-shipping`, run against the committed HEAD. A marker is a process
   check, not a security control: an agent that writes its own attestation rows can stamp one
   without a real review. What stops an unreviewed push or PR is the user's `ask` prompt on
   `git push` and `gh pr create`, so never move those to `allow`.
2. **Do NOT run `/review-before-shipping` or `/ship`.** Those are the user's to trigger, unless a
   session goal (`/ticket-loop`) has told you to.

   **Take a review agent's first report and act on it.** Every follow-up you send keeps that agent
   alive, and the cost is invisible to you and visible to the user. Ask once, for something
   specific you cannot proceed without. Never re-dispatch a fresh review because you fixed the
   findings; re-review at ship time against the new SHA.

   **But a dispatched agent that has not reported yet is not a report you may act without.** Once
   you have asked for a second opinion, its absence is not a green light.

   **Nothing terminal happens while an agent you dispatched is still running.** Terminal means
   anything that closes work off in someone else's eyes: a verdict, a board move, an issue
   comment, a commit, a push, a `Closes #` keyword, or telling the user it is done. Do the
   non-blocking work (read the diff, run the gates, draft the report) and hold the last step. If
   an agent is stuck, say so and let the user decide.

3. **Verify the diff.** `git diff --stat` for unstaged work and `git diff origin/main...HEAD
--stat` for the branch. Get unrelated dirty files out of the commit, but never blindly
   `git checkout --` them: another session may own that change. Stage only this issue's files, by
   explicit path.
4. **Contamination audit.** `git log origin/main..HEAD --oneline`: every commit belongs to this
   issue. `git diff origin/main...HEAD --stat`: every changed file sits inside its scope. If
   another issue's work is present, **stop and flag it.** Three dots for the file audit, two for
   the commit audit: `git diff origin/main..HEAD` renders everything that landed on `main` since
   you forked as a deletion on your side, which reads as false contamination.

   **Audit against `origin/main`, never the local `main` ref.** A stale local `main` makes
   `main...HEAD` list everything landed upstream since it last moved. It looks exactly like
   contamination, and the natural reaction (assume the branch is poisoned, reset it) destroys good
   work. Confirm first: `git rev-list --count main..origin/main` above zero means your local ref
   is simply behind.

5. **Check whether the branch would revert `main`.** Contamination is other work arriving in your
   diff; this is the mirror: your diff undoing work that landed while you were building. The
   exposure is any file the whole repo shares: `.claude/hooks/`, `.claude/rules/`, `scripts/`,
   `.githooks/`, `CLAUDE.md`, `.github/workflows/`, `deny.toml`, `Cargo.lock`, `pnpm-lock.yaml`,
   `docs/threat-model.md`.

   ```bash
   git diff --name-only HEAD...origin/main     # what main moved since you forked
   git diff --name-only origin/main...HEAD     # what this branch touched
   ```

   For any file in both lists, read `git diff origin/main HEAD -- <file>` and confirm every line is
   a change you meant. Bring in `origin/main` (rebase before the first push, merge after it) and
   take main's version of what you did not change. A control loosened to unblock your branch is
   the standing case: main may have tightened it since, and merging quietly puts the loophole back.

## Phase 5: Report to the user

Report what changed against what the issue asked for, what to check (controls, commands, edge
cases), and which gates ran and which did not: the other-OS CI jobs, Lighthouse and the GUI do not
run in a local loop (`testing.md`). Name the paths the change reaches: the desktop shell, the
server, and the `tessera-core` API that the iPad app will use later.

Where a person must look at the change, run it from the worktree (`pnpm dev:desktop`,
`pnpm dev:web`, `cargo run -p tessera-server`) against a synthetic vault, confirm the changed
element is there, and record what you saw. A website or server URL is checked before you hand it
over: `curl -s -o /dev/null -w '%{http_code}\n' <url>` must print 200. A GUI you did not launch is
not verified; say so.

Move the board item to **Post build** (`scripts/board-status.sh <n> "Post build"`): built and
self-reviewed. It stays there through `/ship`.

**Then wait for sign-off.** `/review-before-shipping` and `/ship` are the user's to trigger. Never
run either off your own bat: not when the tests go green, not when `/code-review` comes back
clean, not because the work feels finished. Say which "done" you mean ("ready for review",
"pushed, PR open", "merged") and what you ran.

## Phase 6: Ship (after sign-off)

Run `/ship`. It owns the order: the gate (`pnpm gate`, once per SHA), commit,
`/review-before-shipping`, push, PR against `main`, then **Moment 3** (`github-issues.md` →
Moment 3): verification comment (1,200 characters, real controls and commands, no attribution),
board item to **Post build**, follow-ups only where all four scope tests pass. Pushing and opening
the PR still need the user's go-ahead (`CLAUDE.md` → "Standing rules"); `/ship` asks.

Things `/ship` does not own:

- **Merging.** The user squash-merges the PR in GitHub after review. An agent never merges a PR
  and never pushes `main`; `master-protection-gate.sh` blocks both.
- **The board after `Post build`.** `Done` is the user's, or GitHub's when the merged PR closes
  the issue.
- **Closing.** `Closes #N` in the PR body closes the issue when the PR merges into `main`, the
  default branch. Check the link before reporting:
  `gh pr view <pr> --repo Vainnor/tessera --json closingIssuesReferences`.

If the work comes back (a failed check, a review finding, a walk showing a defect, a card moved
back to `To Do` with a return reason), treat it as a fresh Phase 3, not a patch: read why, fix, and
re-run Phase 4 before handing back again. Resume with `/start <n>`, which rebuilds a worktree from
the pushed branch. A reviewer may return a card on a stale read; settle it with timestamps (the
comment's `created_at` against the PR's `createdAt`), restore the column, and correct any claim the
stale comment made.

---

## Branch naming

`{type}/{issue-or-hash}/{2-4-word-description}`, at most 50 characters. Types are the commit types
(`git-and-ci.md`): `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `build`, `ci`, `chore`,
`security`. The regex is
`^(feat|fix|refactor|perf|test|docs|build|ci|chore|security)/[a-z0-9]+/[a-z0-9]+(-[a-z0-9]+){1,3}$`,
so a one-word or five-word description fails. `branch-naming-gate.sh` enforces it; keep any other
copy of the regex byte-identical, or two gates disagree.

- `feat/12/nested-vault-listing` (28 characters)
- `fix/31/theme-name-limit` (23 characters)
- `chore/1617e87/hook-tests-port` (29 characters: no issue, so a short hash)

**The middle segment is required.** An issue number where there is an issue, a short hash where
there is not, so every branch says on sight what it belongs to. A two-segment name
(`chore/update-deps`) is rejected; rename with `git branch -m`.

**Never push `main`.** `master-protection-gate.sh` blocks it. If you are on `main` with work to
commit, stop and ask for an issue rather than creating a branch by hand.

## Worktrees

**Create the worktree through `/start`.** `EnterWorktree` makes
`.claude/worktrees/{type}-<n>-{desc}` on a `worktree-*` branch cut from `origin/main`; `/start`
renames it to the issue branch at once. A subagent cannot use `EnterWorktree` and uses
`git worktree add` at the same path instead (`/start` Step 2).

**Then provision it: a fresh worktree is an empty checkout.** `scripts/setup-worktree.sh` does the
job and is idempotent, so re-running it repairs drift:

```bash
bash <primary-checkout>/scripts/setup-worktree.sh <worktree-dir-name>   # node_modules, desktop dist, hooks path
bash <primary-checkout>/scripts/teardown-worktree.sh <worktree-dir-name> # removes the dir and the local branch
```

**An unprovisioned worktree does not fail loudly; it fails in the shape of a regression.** No
`node_modules` fails the JS half; no `apps/desktop/dist` fails the desktop crate's build
(`rust.md` → "Gate"). Both read as something your diff did. Trust no gate result from a worktree
whose setup did not exit 0.

- **Each worktree builds into its own `target/`.** Never export a `CARGO_TARGET_DIR` another
  worktree builds into: cargo judges path crates fresh by mtime, so a worktree can run another's
  test binary and pass on code it does not have.
- **Creating and removing the issue worktree is yours**, through `/start`, `/ship` and `/cleanup`.
  The branch rule protects the primary checkout, which stays on `main`.
- **`ExitWorktree` deletes only the `worktree-*` branch name.** The issue branch is deleted by
  `scripts/teardown-worktree.sh`, and only when `origin` holds its commits.
- **The session cwd is shared and pins to whoever entered a worktree last.** With several
  worktrees open, a bare `cd`, a relative path or an unqualified `git` lands in whichever was
  entered most recently. **Use absolute paths.** Afterwards check `git status` in both the
  worktree and the primary checkout: a stray write to the primary checkout is silent.
- **Another session may touch your worktree.** A tracked file you did not modify, a stray `.bak`
  or a compile error in an unrelated file is the tell. Run `git status --porcelain` before and
  after every mutation or merge, and keep backups in your scratchpad, not in the worktree.
- **Inside a worktree session the harness confines git and edits to that worktree.** It refuses
  `git -C` into another checkout, git nested in `$(...)` or `<(...)`, and Edit or Write to a
  primary-checkout path. Run git as plain commands with values written in literally, and put
  multi-step git logic in a script under `scripts/`, invoked as `bash <path>`. The gates parse
  quoted arguments too: a probe or mutation whose text names a refused command goes in a file
  (built by Python string concatenation), not on the line, and ANSI-C `$'...'` quoting stays off
  any line that runs `git commit`.
- **A worktree runs its own copy of `.claude/rules/` and `.claude/commands/`, but the primary
  checkout's copy of the hooks** (`.claude/settings.json` resolves them through
  `--git-common-dir`) **and of `.claude/agents/`**: a branch that changes an agent definition is
  reviewed by the old one. A branch cut before a rule landed keeps following the old rule. When a
  rule change governs what gets posted, pushed or moved, bring `origin/main` into the open
  worktrees.
- **Rebase onto `origin/main` only before the first push; once a branch is pushed, merge
  `origin/main` into it.** A rebase of a pushed branch needs a force-push and rewrites what a
  reviewer already read.
- **Only ever rebase onto or merge `origin/main`.** `branch-ancestry-gate.sh` blocks merging or
  rebasing any other ref, unless the branch is stacked on it by the user's decision. If you need
  another branch's changes, they go to `main` first.

## Stage on purpose

- Stage explicit paths. Never `git add -A`, `git add .` or `git add --all`: they commit unmerged
  files with their conflict markers, and `settings.json` denies them.
- Before committing a merge or conflict resolution, both of these come back empty:
  `git diff --name-only --diff-filter=U` and `git grep -n '^<<<<<<< '`.

## Squash onto a fixed point

`git reset --soft origin/main` squashes onto whatever that ref points at **now**, and any
`git fetch` in any worktree moves it. The commit then pairs your old tree with the new base and
reverts everyone else's work.

- Squash with `git reset --soft <sha>`, where `<sha>` is the merge base you read first
  (`git merge-base HEAD origin/main`, as its own command) or `HEAD~N`.
- Before committing, `git diff --cached --stat` lists only your files. A long list means stop.
- Rebase onto the new `main` as a separate step afterwards.

## Never stash in a background command

A background job killed between `git stash` and `git stash pop` strands your work in the stash and
leaves a clean tree that looks like lost edits. The stash stack is shared by every worktree and
session. To set work aside, commit a WIP checkpoint or copy files to your scratchpad. Never
`git stash` mid-merge either: it drops `MERGE_HEAD`, and the commit comes out with one parent.

## Shared and stacked branches move under you

- **Before pushing to a branch someone else may hold**, check that it has not moved:
  `git merge-base --is-ancestor origin/<branch> HEAD`. Push with
  `--force-with-lease=<branch>:<full-sha>`, never `--force`.
- **A stacked base moves mid-ship.** Right before pushing, `git fetch` and `git rev-parse` the
  base, one ref per call; `git rev-parse a b` fails and quietly stops an `&&` chain. Re-rebase only
  if `git merge-tree --write-tree --name-only <base> <branch>` reports a conflict or the base
  reintroduces what review flagged. Prove the rebase did not change reviewed content with
  `git range-diff`.

## Pitfalls

- **`git checkout` destroys uncommitted work, and some of the tree is untracked.** Check
  `git status --short` for `??` entries before any checkout that could clobber them.
- **A rebase dirties unrelated files.** Run `git diff --stat` afterwards and `git checkout --` only
  what you have confirmed the rebase touched.
- **Integrating `main` brings main's newer checks against your additions** (a tightened lint in
  `[workspace.lints]`, a new `deny.toml` rule, a new test in `crates/core/tests/`). Those are
  branch-introduced failures to fix, not pre-existing ones.
- **Check a baseline failure against `main` from CI on `origin/main`'s SHA**, never by
  `git checkout main` inside the worktree.
- **After any commit, check it holds your changes**: `git diff origin/main...HEAD --name-only` and
  `git status --short`. Every intended file is in the diff, nothing intended is still modified,
  nothing you did not mean to ship is staged. A pre-commit refusal (gitleaks, the commit-msg hook)
  leaves nothing committed.
- **A push is not done until the remote says so.** After every push,
  `git ls-remote origin refs/heads/<branch>` must equal `git rev-parse HEAD`, before you comment,
  move a card or remove the worktree. Chain the hand-off on the push with `&&`, never `;`.
- **A rebase that moved `pnpm-lock.yaml` leaves `node_modules` stale.** `pnpm audit` and the
  typecheck read what is installed, not what is committed. Run
  `pnpm install --frozen-lockfile` before diagnosing an audit or type failure.
- **Tests failing for environment reasons after three debug attempts: stop and ask the user.**
- **Never remove tooling to bypass a check.** If clippy, rustfmt, gitleaks or a hook blocks a
  commit, fix the cause. Never `--no-verify` (`git-and-ci.md`), never uninstall the tool, delete
  its config, or add an `#[allow(...)]` without a stated `reason` to hide an error your change
  introduced (`rust.md`).
