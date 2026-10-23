---
description: Remove a finished issue worktree and its local branch — refuses while anything in it is uncommitted or unpushed.
argument-hint: "[worktree name] (default: the worktree you are in)"
---

You are tearing down a worktree locally: the worktree directory and the local branch. `/ship` does
the same as its last step; `/cleanup` is for a worktree that never shipped, was abandoned or was
left behind by an earlier session.

This is **destructive and cannot be undone**. The checks below exist so it never destroys work
that is not already safe on `origin`. Follow every step in order.

`scripts/teardown-worktree.sh` does the work. For a worktree the current session entered,
`ExitWorktree` triggers it through its PreToolUse hook (`.claude/hooks/worktree-teardown.sh`); for
any other worktree, run it from the primary checkout.

## Step 1: Pick the worktree

Worktrees live under `.claude/worktrees/<NAME>`. `/start` names them after the branch with `/`
encoded as `-` (`fix-12-vault-rename-case`). Take an explicit name from the user's message;
otherwise use the worktree the session is in.

```bash
pwd
git worktree list
```

- **A name was given**: use it.
- **No name**: the basename of the current `.claude/worktrees/<NAME>` directory. If the cwd is not
  under `.claude/worktrees/`, **stop** and ask which worktree to tear down.

The name matches `^[A-Za-z0-9_-]+$`, the pattern `scripts/setup-worktree.sh` and
`scripts/teardown-worktree.sh` enforce.

**Refuse** the primary checkout (the first entry in `git worktree list`, whose `.git` is a
directory; it stays on `main` and is never removed), a worktree on `main`, and one another session
is plainly using (you didn't create it and it isn't yours to remove): ask.

A worktree made by hand with `git worktree add` outside `.claude/worktrees/` gets the same Step 2
and Step 3 checks, then `git worktree remove <WT>` and `git branch -d <BR>` from the primary
checkout, never `--force`.

## Step 2: Check nothing unsaved is about to be destroyed

From the primary checkout, with `<WT>` the absolute worktree path:

```bash
git -C <WT> fetch --prune origin                 # fresh remote refs; a failure goes to Step 3
git -C <WT> status --porcelain                   # a) uncommitted or untracked changes
git -C <WT> rev-parse --abbrev-ref HEAD          # b) the branch ...
git -C <WT> rev-parse HEAD                       #    ... its commit ...
git ls-remote origin refs/heads/<branch>         #    ... and origin's (empty: never pushed)
git -C <WT> log origin/main..HEAD --oneline      # c) commits not on origin/main
```

Inside a worktree session the harness refuses both `git -C` into another checkout and git nested
in `$(...)`, so run the same commands plainly, without `-C`, from inside the target.

- **(a) is clean and either (b) HEAD equals origin's SHA or (c) prints nothing**: every commit is
  on `origin`. Go to Step 4; asking for `/cleanup` is the go-ahead for a safe teardown.
- **(a) is dirty**, the fetch failed, or commits exist that are on neither `origin/<branch>` nor
  `origin/main`: go to Step 3.

Compare against `origin/...` and `git ls-remote`, never the local `main`. Ignored files
(`node_modules`, `target/`, `dist/`) don't show in (a) and are expected to go.

A `ticket-reviewer` `chore/<n>/review-<hash>` branch is never pushed under its own name, so an
empty `ls-remote` is expected. Judge it against the PR's branch
(`gh pr view <pr> --repo Vainnor/tessera --json headRefName`): with (a) clean,
`git -C <WT> log origin/<pr-branch>..HEAD --not origin/main --no-merges --oneline` printing
nothing means only the review's own `main` merge is local, and nothing is lost. The teardown script
keeps that local branch, since origin has none of its name; once this check passed, delete it with
`git branch -D <branch>` from the primary checkout.

## Step 3: Confirm before losing anything (only when Step 2 found unsaved work)

Stop and tell the user exactly what would be lost:

- "Worktree `<NAME>` has uncommitted changes: `<list>`."
- "Branch `<branch>` has N commit(s) not on origin: `<oneline list>`."
- If `origin/<branch>` is gone after the fetch, check
  `gh pr list --repo Vainnor/tessera --head <branch> --state merged` first: a merged PR whose
  branch GitHub deleted is saved work, so say "PR #N merged and its branch was deleted; the work is
  on `main`" rather than calling it unpushed.

Offer the alternatives (commit and push first, or `ExitWorktree` with `action: "keep"`), and go on
only on an explicit yes. The teardown script refuses a dirty worktree regardless, so discarding
uncommitted changes means the user discarding them first; and it keeps a local branch whose
commits are not on origin, which then needs a deliberate `git branch -D`. Never reach for
`--force`, `rm -rf` or a stash to get past a check.

## Step 4: Tear down

**The session is inside the target worktree**: call `ExitWorktree` with `action: "remove"`. It
refuses when the branch has commits not on the default branch; when Step 2 showed they are on
`origin` (or Step 3 got a yes), call it again with `discard_changes: true`. The hook runs the
teardown script before the removal and blocks it if the script fails, so a blocked call removed
nothing: read the message, fix, retry.

**Any other case** (a name given from the primary checkout, or a worktree from an earlier session,
where `ExitWorktree` does nothing): from the primary checkout,

```bash
bash scripts/teardown-worktree.sh <NAME>
```

Both paths remove the directory and delete the local branch when origin holds its commits.

## Step 5: Check and report

From the primary checkout:

```bash
ls -d .claude/worktrees/<NAME> 2>/dev/null && echo "DIR STILL PRESENT" || echo "dir removed"
git branch --list "<branch>"                     # empty, unless kept for unpushed commits
git worktree list                                # target not listed
git worktree prune --dry-run                     # nothing stale left behind
```

Anything left over: run `bash scripts/teardown-worktree.sh <NAME>` again (idempotent) and check. A
worktree locked by a live session in another terminal is refused before anything is removed; exit
that session first.

Report what was removed and that the session is back in the primary checkout on `main`. Say that
the **remote branch and any open PR are untouched**: `/cleanup` is local only.

## Notes

- After `/ship` there is normally nothing left to clean. `/cleanup <NAME>` is still safe then;
  every step is idempotent.
- This command touches no issue, board card or marker.
