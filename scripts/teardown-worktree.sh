#!/bin/bash
set -e

# Tear down a Tessera issue worktree.
# Removes the worktree directory, and deletes the local branch when origin already
# holds every commit on it, or when a merged PR's head is the branch's commit
# (asked of gh only where timeout(1) exists).
# Refuses, before anything is removed, while anything in the worktree is
# uncommitted, or when a detached HEAD holds commits no origin branch has.
# The remote branch and any PR are never touched.
#
# Usage (from the main checkout):
#   bash scripts/teardown-worktree.sh fix-12-vault-rename-race          # .claude/worktrees/<NAME>
#   bash scripts/teardown-worktree.sh /abs/path/to/worktree             # any registered worktree, by path
#   bash scripts/teardown-worktree.sh fix/12/vault-rename-race          # by the branch it has checked out
# Inside a worktree session use ExitWorktree instead: its hook
# (.claude/hooks/worktree-teardown.sh) runs this script and sets
# WORKTREE_TEARDOWN_FROM_HOOK, the only way the session's own lock is overridden.
#
# Tests: scripts/lib/teardown-worktree.test.sh

# ---------------------------------------------------------------------------
# Colors
# ---------------------------------------------------------------------------
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/portable.sh
. "$SCRIPT_DIR/lib/portable.sh"

blocked() { echo -e "${RED}BLOCKED:${NC} $1"; exit 1; }

# ---------------------------------------------------------------------------
# 1. Determine the worktree: a .claude/worktrees name, a path, or a branch
# ---------------------------------------------------------------------------
if ! PROJECT_ROOT="$(main_repo_root)"; then
    echo -e "${RED}Error:${NC} Not inside a git repository — cannot locate the worktree."
    exit 1
fi
PROJECT_ROOT="$(cd "$PROJECT_ROOT" && pwd -P)"
ARG="${1:-}"
[ -n "$ARG" ] || ARG="$(git rev-parse --show-toplevel)"

# git worktree list --porcelain as "path<US>branch-or-empty<US>locked-reason-or-empty",
# US the 0x1f unit separator: not whitespace, so read keeps an empty field.
LISTING="$(git -C "$PROJECT_ROOT" worktree list --porcelain | awk '
    function flush() { if (path != "") print path "\037" branch "\037" locked; path = ""; branch = ""; locked = "" }
    /^worktree / { flush(); path = substr($0, 10) }
    /^branch refs\/heads\// { branch = substr($0, 19) }
    /^locked/ { locked = substr($0, 8); if (locked == "") locked = "(no reason)" }
    END { flush() }')"
PRIMARY="$(printf '%s\n' "$LISTING" | head -1 | cut -d$'\037' -f1)"

# find_by FIELD VALUE: sets WORKTREE_DIR, BRANCH and LOCK_LINE from the listing entry whose
# resolved path (FIELD=path) or branch (FIELD=branch) is VALUE.
find_by() {
    local path branch locked
    WORKTREE_DIR=""
    while IFS=$'\037' read -r path branch locked; do
        [ -n "$path" ] || continue
        if { [ "$1" = path ] && [ "$(cd "$path" 2>/dev/null && pwd -P)" = "$2" ]; } \
            || { [ "$1" = branch ] && [ -n "$branch" ] && [ "$branch" = "$2" ]; } \
            || { [ "$1" = registered ] && [ "$path" = "$2" ]; }; then
            WORKTREE_DIR="$path" BRANCH="$branch" LOCK_LINE="$locked"
        fi
    done <<EOF
$LISTING
EOF
    [ -n "$WORKTREE_DIR" ]
}

NAME=""
BRANCH=""
LOCK_LINE=""
# A .claude/worktrees name is interpolated into a path. `[[ =~ ]]`, not `echo | grep`:
# grep matches per line and would let a payload through on its second line. `-` (or `+`
# in older worktrees) is how a worktree name encodes a branch name's `/`.
if [[ "$ARG" =~ ^[A-Za-z0-9_+-]+$ ]] && [[ -d "$PROJECT_ROOT/.claude/worktrees/$ARG" || ! -e "$ARG" ]] \
    && ! git -C "$PROJECT_ROOT" show-ref --verify --quiet "refs/heads/$ARG"; then
    NAME="$ARG"
    CANDIDATE="$PROJECT_ROOT/.claude/worktrees/$NAME"
    if [ -d "$CANDIDATE" ]; then
        find_by path "$(cd "$CANDIDATE" && pwd -P)" \
            || blocked "${CANDIDATE} is not a registered worktree. Inspect it and remove it by hand."
    elif find_by registered "$CANDIDATE"; then
        :
    else
        WORKTREE_DIR=""
    fi
elif [ -d "$ARG" ]; then
    find_by path "$(cd "$ARG" && pwd -P)" \
        || blocked "${ARG} is not a registered worktree. Inspect it and remove it by hand; this script removes only what git worktree list names."
else
    find_by branch "$ARG" || blocked "no worktree has branch ${ARG} checked out, and there is no .claude/worktrees/${ARG}."
fi

if [ -n "$WORKTREE_DIR" ] && [ "$(cd "$WORKTREE_DIR" 2>/dev/null && pwd -P)" = "$(cd "$PRIMARY" && pwd -P)" ]; then
    blocked "${WORKTREE_DIR} is the primary checkout, which is never removed."
fi

# Only the loop's own worktrees: one elsewhere may hold ignored files (a local vault, an .env)
# that `git worktree remove` would delete with it.
if [ -n "$WORKTREE_DIR" ]; then
    RESOLVED="$WORKTREE_DIR"
    [ -d "$RESOLVED" ] && RESOLVED="$(cd "$RESOLVED" && pwd -P)"
    case "$RESOLVED/" in
        "$(cd "$PROJECT_ROOT" && pwd -P)/.claude/worktrees/"?*) ;;
        *) blocked "${WORKTREE_DIR} is not under .claude/worktrees/. Inspect it and remove it by hand." ;;
    esac
fi
case "$BRANCH" in
    main) blocked "the worktree has main checked out; main is never removed here." ;;
esac

# The ExitWorktree hook runs this with its output hidden from the session, so the
# main checkout keeps a record of every run.
mkdir -p "$PROJECT_ROOT/.claude"
exec > >(tee -a "$PROJECT_ROOT/.claude/worktree-teardown.log") 2>&1
echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] teardown ${WORKTREE_DIR:-.claude/worktrees/$NAME}"
echo -e "${CYAN}Tearing down worktree:${NC} ${WORKTREE_DIR:-.claude/worktrees/$NAME} (${BRANCH:-detached})"
echo ""

# ---------------------------------------------------------------------------
# 2. Uncommitted work, unpushed detached commits, and the worktree lock
# ---------------------------------------------------------------------------
IS_WORKTREE=false
[ -n "$WORKTREE_DIR" ] && [ -d "$WORKTREE_DIR" ] && IS_WORKTREE=true

INSIDE=false
if [ "$IS_WORKTREE" = true ]; then
    case "$(pwd -P)/" in "$(cd "$WORKTREE_DIR" && pwd -P)/"*) INSIDE=true ;; esac
fi

if [ "$IS_WORKTREE" = true ]; then
    # Ignored files (.env, node_modules, target/) don't show and are expected to go.
    DIRTY="$(git -C "$WORKTREE_DIR" status --porcelain 2>/dev/null)" || blocked "git status failed in ${WORKTREE_DIR}."
    if [ -n "$DIRTY" ]; then
        echo -e "${RED}BLOCKED: Worktree has uncommitted or untracked changes:${NC}"
        echo ""
        git -C "$WORKTREE_DIR" status --short
        echo ""
        echo -e "${YELLOW}Commit and push them, or keep the worktree. Discarding work is the user's decision, never this script's.${NC}"
        exit 1
    fi
    echo -e "${GREEN}Working directory is clean.${NC}"

    # Captured now: once the directory is gone nothing records which commit it had.
    BRANCH_SHA="$(git -C "$WORKTREE_DIR" rev-parse HEAD)"
    # A detached HEAD's commits live in no branch, so removing the worktree would strand
    # any that origin lacks. A branch keeps its commits whatever happens below.
    if [ -z "$BRANCH" ] && [ -z "$(git -C "$WORKTREE_DIR" branch -r --contains "$BRANCH_SHA" 2>/dev/null)" ]; then
        echo -e "${RED}BLOCKED: the worktree is detached at ${BRANCH_SHA:0:8}, which no origin branch contains; removing it would strand:${NC}"
        git -C "$WORKTREE_DIR" log --oneline "$BRANCH_SHA" --not --remotes=origin | head -20
        echo -e "${YELLOW}Put it on a branch (git -C ${WORKTREE_DIR} switch -c <branch>) or push it first.${NC}"
        exit 1
    fi

    if [ "$INSIDE" = true ] && [ "${WORKTREE_TEARDOWN_FROM_HOOK:-}" != 1 ]; then
        # A Bash call from inside would delete the session's own working directory.
        blocked "run this from the main checkout, never from inside the worktree it removes:
  cd ${PROJECT_ROOT} && bash scripts/teardown-worktree.sh ${NAME:-$WORKTREE_DIR}"
    fi

    # A Claude session locks the worktree it is in ("claude session <branch> (pid P
    # start S)"), and a locked worktree only goes with `remove -f -f`. Decide before
    # anything is dropped. Overridden: a lock whose process is gone, and our own
    # process's lock when called from inside the worktree (the ExitWorktree hook).
    # Blocked: a lock with no pid, a live process elsewhere, and our own process's
    # lock called from outside, which is a subagent still working in that worktree.
    # Where /proc exists the start time S proves the pid is the same process; without
    # it (macOS) a live pid counts as the lock's owner, which can only block a
    # teardown, never wrongly allow one.
    LOCK_OVERRIDE=""
    if [ -n "$LOCK_LINE" ]; then
        LOCK_PID="$(printf '%s' "$LOCK_LINE" | sed -nE 's/.*\(pid ([0-9]+)( start [0-9]+)?\).*/\1/p')"
        LOCK_START="$(printf '%s' "$LOCK_LINE" | sed -nE 's/.*\(pid ([0-9]+) start ([0-9]+)\).*/\2/p')"
        [ -n "$LOCK_PID" ] || blocked "worktree is locked with no owning process: ${LOCK_LINE}
Unlock it deliberately first: git worktree unlock ${WORKTREE_DIR}"
        LOCK_OWNER=stale
        if [ -r "/proc/$$/stat" ]; then
            if [ -r "/proc/$LOCK_PID/stat" ] \
                && [ "$(sed 's/.*) //' "/proc/$LOCK_PID/stat" | awk '{ print $20 }')" = "$LOCK_START" ]; then
                LOCK_OWNER=other
            fi
        elif kill -0 "$LOCK_PID" 2>/dev/null; then
            LOCK_OWNER=other
        fi
        if [ "$LOCK_OWNER" = other ]; then
            p=$$
            while [ -n "$p" ] && [ "$p" -gt 1 ]; do
                if [ "$p" = "$LOCK_PID" ]; then LOCK_OWNER=self; break; fi
                p="$(ps -o ppid= -p "$p" 2>/dev/null | tr -d ' ')"
            done
        fi
        if [ "$LOCK_OWNER" = self ] && { [ "${WORKTREE_TEARDOWN_FROM_HOOK:-}" != 1 ] || [ "$INSIDE" != true ]; }; then
            blocked "this Claude session holds the worktree lock.
Leave the worktree with ExitWorktree (action: remove), which runs this teardown itself."
        fi
        [ "$LOCK_OWNER" != other ] || blocked "worktree is in use by another live Claude session (pid ${LOCK_PID}).
Exit that session, or run the teardown from inside it."
        LOCK_OVERRIDE="--force"
        echo -e "${YELLOW}Worktree lock held by ${LOCK_OWNER} session pid ${LOCK_PID} — overriding it.${NC}"
    fi
elif [ -n "$WORKTREE_DIR" ]; then
    echo -e "${YELLOW}${WORKTREE_DIR} is registered but gone — pruning its record.${NC}"
else
    echo -e "${YELLOW}No worktree at .claude/worktrees/${NAME} — nothing to remove.${NC}"
fi
echo ""

# ---------------------------------------------------------------------------
# 5. Remove the worktree directory
# ---------------------------------------------------------------------------
WORKTREE_REMOVED=skipped
cd "$PROJECT_ROOT"
if [ "$IS_WORKTREE" = true ]; then
    # Removal discards anything written since the clean check above, so look again
    # immediately before it.
    if [ -n "$(git -C "$WORKTREE_DIR" status --porcelain 2>/dev/null)" ]; then
        echo -e "${RED}BLOCKED: the worktree changed during teardown — directory left in place:${NC}"
        git -C "$WORKTREE_DIR" status --short
        exit 1
    fi
    echo -e "${YELLOW}Removing worktree directory...${NC}"
    # Without --force git refuses a worktree with modified or untracked files, a second
    # line of defense. Only a lock override adds the double force a locked worktree needs.
    # shellcheck disable=SC2086  # LOCK_OVERRIDE is empty or one flag
    git worktree remove ${LOCK_OVERRIDE:+--force $LOCK_OVERRIDE} "$WORKTREE_DIR" || true
    git worktree prune 2>/dev/null || true
    if [ -d "$WORKTREE_DIR" ]; then
        WORKTREE_REMOVED=failed
        echo -e "${RED}Could not remove ${WORKTREE_DIR}.${NC} git worktree remove refused; read its message above."
    else
        WORKTREE_REMOVED=yes
        echo -e "${GREEN}Worktree directory removed.${NC}"
    fi
    echo ""
elif [ -n "$WORKTREE_DIR" ]; then
    git worktree prune 2>/dev/null || true
fi

# ---------------------------------------------------------------------------
# 5b. Delete the local branch — only when every commit on it is on origin
# ---------------------------------------------------------------------------
# Safe only when origin holds the commits: the pushed branch at this SHA,
# origin/main already containing it, or (below) a merged PR whose head is this
# commit. The pushed branch is asked of origin now, not of refs/remotes/origin/*,
# which still matches after the remote branch is deleted; an unreachable origin
# prints nothing, so the branch is kept. No prompts and a time limit: the hook's
# output is hidden, so a credential prompt would hang the session invisibly. A
# stale origin/main only makes the check stricter.
#
# A squash merge satisfies neither origin check: GitHub deletes the remote branch
# and main gets a new commit. So, failing both, ask GitHub (not origin) for the
# head SHAs of the branch's merged PRs: GitHub keeps a merged PR's commits under
# refs/pull/<n>/head. Only an exact match to the branch's commit counts, so a
# branch with commits past the PR's head is kept. A gh that is missing, fails (its
# output is then ignored) or runs past its time limit keeps the branch; the limit
# is what stops a hidden hang. gh is asked only where timeout(1) exists: without it
# with_timeout falls back to a SIGALRM, which gh, a Go program, ignores.
BRANCH_RESULT=skipped
if [ "$WORKTREE_REMOVED" = yes ] && [ -n "$BRANCH" ] \
    && git -C "$PROJECT_ROOT" show-ref --verify --quiet "refs/heads/$BRANCH"; then
    REMOTE_SHA="$(GIT_TERMINAL_PROMPT=0 GIT_SSH_COMMAND='ssh -o BatchMode=yes' \
        with_timeout 20 git -C "$PROJECT_ROOT" ls-remote origin "refs/heads/$BRANCH" 2>/dev/null | awk '{ print $1 }' || true)"
    ON_ORIGIN=""
    if [ "$REMOTE_SHA" = "$BRANCH_SHA" ] \
        || git -C "$PROJECT_ROOT" merge-base --is-ancestor "$BRANCH_SHA" origin/main 2>/dev/null; then
        ON_ORIGIN="its commits are on origin"
    elif command -v timeout >/dev/null 2>&1 \
        && MERGED_HEADS="$(GH_PROMPT_DISABLED=1 with_timeout 10 gh pr list --repo Vainnor/tessera \
            --head "$BRANCH" --state merged --json headRefOid --jq '.[].headRefOid' 2>/dev/null)" \
        && printf '%s\n' "$MERGED_HEADS" | grep -qxF -- "$BRANCH_SHA"; then
        ON_ORIGIN="its merged PR's head is this commit"
    fi
    if [ -n "$ON_ORIGIN" ]; then
        # Checked delete: only if the branch still points at the commit origin holds.
        if git -C "$PROJECT_ROOT" update-ref -d "refs/heads/$BRANCH" "$BRANCH_SHA" 2>/dev/null; then
            BRANCH_RESULT=deleted
            echo -e "${GREEN}Local branch ${BRANCH} deleted (${ON_ORIGIN}).${NC}"
        else
            BRANCH_RESULT=moved
            echo -e "${YELLOW}Local branch ${BRANCH} kept: it moved during teardown.${NC}"
        fi
    else
        BRANCH_RESULT=kept
        echo -e "${YELLOW}Local branch ${BRANCH} kept: neither origin nor a merged PR is known to hold its commits.${NC}"
        echo -e "  Push it, or delete it deliberately: ${CYAN}git branch -D ${BRANCH}${NC}"
    fi
    echo ""
fi

# ---------------------------------------------------------------------------
# 6. Summary
# ---------------------------------------------------------------------------
echo -e "${GREEN}Teardown complete for ${CYAN}${WORKTREE_DIR:-.claude/worktrees/$NAME}${GREEN}:${NC}"
case "$WORKTREE_REMOVED" in
    yes)     echo -e "  - Worktree directory removed" ;;
    failed)  echo -e "  - ${RED}Worktree directory NOT removed${NC} — see above" ;;
    skipped) echo -e "  - No worktree directory to remove" ;;
esac
case "$BRANCH_RESULT" in
    deleted) echo -e "  - Local branch ${CYAN}${BRANCH}${NC} deleted; the remote branch and any PR are untouched" ;;
    kept)    echo -e "  - ${YELLOW}Local branch ${BRANCH} kept${NC} — its commits are not known to be on origin" ;;
    moved)   echo -e "  - ${YELLOW}Local branch ${BRANCH} kept${NC} — it moved" ;;
esac

# A directory left behind fails the run, so the ExitWorktree hook blocks while the
# worktree still exists and a retry works.
if [ "$WORKTREE_REMOVED" = failed ]; then
    echo -e "${RED}Teardown incomplete — see above.${NC}"
    exit 1
fi
