#!/bin/bash

# ==============================================================================
# Verify a worktree is actually usable
# ==============================================================================
# Standalone health check. Run it any time; run it before believing a test
# result you got inside a worktree.
#
# The failure this exists to catch is not "the worktree is broken" — it is "the
# worktree is subtly wrong and every gate you run inside it lies to you". A fresh
# worktree has no node_modules, so the JS half dies on a missing binary, and no
# apps/desktop/dist, so the desktop crate does not compile; a stale node_modules runs
# the tests against packages the lockfile no longer names. Each reads as a branch
# regression and is not one.
#
# Usage: bash scripts/verify-worktree.sh          (from inside the worktree)
# Exit:  0 all critical checks passed, 1 otherwise.
# Tests: scripts/lib/verify-worktree.test.sh
# ==============================================================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

PASS=0
FAIL=0
WARN=0

check_pass() { echo -e "  ${GREEN}PASS${NC} $1"; PASS=$((PASS + 1)); }
check_fail() { echo -e "  ${RED}FAIL${NC} $1"; FAIL=$((FAIL + 1)); }
check_warn() { echo -e "  ${YELLOW}WARN${NC} $1"; WARN=$((WARN + 1)); }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/portable.sh
. "$SCRIPT_DIR/lib/portable.sh"

# lock_body FILE: the packages document of a pnpm lockfile, or the whole file when it is one document.
lock_body() {
    if [[ "$(head -1 "$1")" == "---" ]]; then
        awk 'seen >= 2 { print } /^---$/ { seen++ }' "$1"
    else
        cat "$1"
    fi
}

WORKTREE_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
NAME="$(basename "$WORKTREE_DIR")"

echo ""
echo -e "${YELLOW}Worktree health check: ${CYAN}${NAME}${NC}"
echo "-----------------------------------------------------------"

# -- 1. Is this even a linked worktree? --
#
# In a linked worktree --absolute-git-dir points at .git/worktrees/<name> while
# --git-common-dir points at the main .git. In the main checkout they are equal.
GIT_DIR_PATH=$(git rev-parse --absolute-git-dir 2>/dev/null || echo "")
GIT_COMMON_PATH=$(cd "$(git rev-parse --git-common-dir 2>/dev/null || echo .)" 2>/dev/null && pwd)
IS_WORKTREE=0
if [[ -n "$GIT_DIR_PATH" ]] && [[ -n "$GIT_COMMON_PATH" ]] && [[ "$GIT_DIR_PATH" != "$GIT_COMMON_PATH" ]]; then
    IS_WORKTREE=1
    check_pass "Linked worktree"
else
    check_warn "This is the MAIN checkout, not a worktree — some checks are skipped"
fi

# -- 2. node_modules matches the lockfile --
#
# pnpm keeps the lockfile it last installed from at node_modules/.pnpm/lock.yaml. Since
# pnpm 11 pnpm-lock.yaml is two YAML documents (pnpm's own environment, then the
# packages) and the copy holds only the second, so that is what is compared. When
# it differs from pnpm-lock.yaml the packages under test are not the ones this branch
# declares, and a frontend repro proves nothing.
if [[ ! -d "$WORKTREE_DIR/node_modules" ]]; then
    check_fail "node_modules missing — run: pnpm install --frozen-lockfile (bash scripts/setup-worktree.sh does it)"
elif [[ -L "$WORKTREE_DIR/node_modules" ]]; then
    check_fail "node_modules is a symlink to another checkout — rm it and run: pnpm install --frozen-lockfile"
elif [[ ! -f "$WORKTREE_DIR/node_modules/.pnpm/lock.yaml" ]]; then
    check_fail "node_modules was not installed by pnpm (no .pnpm/lock.yaml) — run: pnpm install --frozen-lockfile"
elif ! lock_body "$WORKTREE_DIR/pnpm-lock.yaml" | cmp -s - "$WORKTREE_DIR/node_modules/.pnpm/lock.yaml"; then
    check_fail "node_modules was installed from a different pnpm-lock.yaml — run: pnpm install --frozen-lockfile"
else
    check_pass "node_modules matches pnpm-lock.yaml"
fi

# -- 3. This worktree's own Cargo target dir --
#
# A CARGO_TARGET_DIR (or CARGO_BUILD_TARGET_DIR) outside this checkout shares build output with another one, and
# cargo then judges a path package fresh by mtime and runs that checkout's test binary
# CARGO_BUILD_TARGET_DIR is the same setting (build.target-dir) by another name; CARGO_TARGET_DIR wins over it.
OWN_TARGET=1
for VAR in CARGO_TARGET_DIR CARGO_BUILD_TARGET_DIR; do
    VALUE="${!VAR:-}"
    if [[ -n "$VALUE" && "${VALUE%/}" != "$WORKTREE_DIR/target" ]]; then
        check_fail "${VAR}=${VALUE} is exported: cargo builds there, not in this checkout's target/, and a target dir shared with another checkout runs its stale test binaries. unset ${VAR}"
        OWN_TARGET=0
    fi
done
if [[ "$OWN_TARGET" -eq 0 ]]; then
    :
elif [[ -d "$WORKTREE_DIR/target" ]]; then
    check_pass "cargo builds into this checkout's own target/"
else
    check_pass "cargo builds into this checkout's own target/ (not built yet; bash scripts/setup-worktree.sh seeds it from the primary checkout's)"
fi

# -- 4. The desktop frontend is built --
#
# The desktop crate embeds apps/desktop/dist, so clippy and cargo test cannot compile it
# without one (.claude/rules/rust.md → "Gate").
if [[ -f "$WORKTREE_DIR/apps/desktop/dist/index.html" ]]; then
    check_pass "apps/desktop/dist is built"
else
    check_fail "apps/desktop/dist missing — run: pnpm build (bash scripts/setup-worktree.sh does it)"
fi

# -- 5. Branch name matches the enforced convention --
BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")
if [[ "$BRANCH" == "main" ]]; then
    if [[ "$IS_WORKTREE" -eq 1 ]]; then
        check_warn "On ${BRANCH} — a worktree should be on a feature branch"
    else
        check_pass "Main checkout on ${BRANCH}"
    fi
elif [[ "$BRANCH" =~ ^(feat|fix|refactor|perf|test|docs|build|ci|chore|security)/[a-z0-9]+/[a-z0-9]+(-[a-z0-9]+){1,3}$ ]]; then
    if [[ ${#BRANCH} -gt 50 ]]; then
        check_fail "Branch '${BRANCH}' is ${#BRANCH} chars (max 50)"
    else
        check_pass "Branch name: ${BRANCH}"
    fi
elif [[ "$BRANCH" == worktree-* ]]; then
    check_warn "Branch '${BRANCH}' is the EnterWorktree name — rename it before pushing, as /start does"
else
    check_fail "Branch '${BRANCH}' does not match {type}/{issue-or-hash}/{description} (a 2-4 word description)"
    echo -e "         ${YELLOW}The branch-naming hook will refuse to push it. Rename:${NC} git branch -m ${BRANCH} <valid>"
fi

# -- 6. Branch ancestry — must descend from main, not another feature branch --
if [[ "$BRANCH" != "main" ]]; then
    MERGE_BASE=$(git merge-base HEAD origin/main 2>/dev/null || echo "")
    if [[ -z "$MERGE_BASE" ]]; then
        check_warn "Cannot determine a merge base with origin/main"
    else
        BEHIND_TIP=$(git rev-list --count "${MERGE_BASE}..origin/main" 2>/dev/null || echo "?")
        check_pass "Forks from origin/main (${BEHIND_TIP} commits behind its tip)"

        # Compare against origin/main, never the local ref: a stale local main
        # makes every commit landed upstream look like contamination.
        # Merging origin/main into the branch is routine. A merge only brings in another branch's work when one
        # of its non-first parents carries commits on neither.
        FOREIGN=""
        while read -r merge; do
            [[ -n "$merge" ]] || continue
            for parent in $(git rev-list --parents -n 1 "$merge" | cut -d' ' -f3-); do
                if [[ -n "$(git rev-list -n 1 "$parent" --not origin/main 2>/dev/null)" ]]; then
                    FOREIGN+="$(git log -1 --oneline "$merge")"$'\n'
                    break
                fi
            done
        done < <(git rev-list HEAD --not origin/main --merges 2>/dev/null)
        FOREIGN="$(printf '%s' "$FOREIGN" | head -5)"
        if [[ -n "$FOREIGN" ]]; then
            check_fail "Branch carries merge commits from branches other than main:"
            echo "$FOREIGN" | while read -r line; do echo -e "         ${RED}${line}${NC}"; done
            echo -e "         ${YELLOW}Possible cross-ticket contamination.${NC}"
        else
            check_pass "No merge commits from other branches"
        fi
    fi
fi

# -- 7. The repo's git hooks are installed --
#
# .githooks/ (pre-commit, commit-msg) runs only once core.hooksPath points at it, and
# nothing else notices when it does not: commits then skip gitleaks and the attribution
# check. Git resolves a relative value against each checkout, so `.githooks` means this
# worktree's copy; an absolute value must name the primary checkout's .githooks, compared
# physically (macOS reports /var as /private/var). The fix needs the user: the gates refuse
# an agent's core.hooksPath write, since a rewritten value could switch the hooks off.
HOOKS_PATH="$(git config core.hooksPath 2>/dev/null)"
HOOKS_FIX="run ! git config core.hooksPath .githooks (the gates refuse this write from an agent)"
HOOKS_DIR=""
if [[ "$HOOKS_PATH" == ".githooks" ]]; then
    HOOKS_DIR="$WORKTREE_DIR/.githooks"
elif [[ "$HOOKS_PATH" == /* ]] && PRIMARY_ROOT="$(main_repo_root)"; then
    WANT="$(cd "$PRIMARY_ROOT/.githooks" 2>/dev/null && pwd -P)"
    GOT="$(cd "$HOOKS_PATH" 2>/dev/null && pwd -P)"
    if [[ "$GOT" == "$WANT" ]]; then
        HOOKS_DIR="$WANT" # empty, so still a failure, when neither directory exists
    fi
fi
if [[ -z "$HOOKS_DIR" ]]; then
    check_fail "git hooks not installed (core.hooksPath is '${HOOKS_PATH}', not .githooks or the primary checkout's .githooks): ${HOOKS_FIX}"
elif [[ ! -x "$HOOKS_DIR/commit-msg" ]]; then
    check_fail "git hooks not installed (${HOOKS_DIR} has no executable commit-msg): ${HOOKS_FIX}"
else
    check_pass "git hooks installed (core.hooksPath=${HOOKS_PATH})"
fi

# -- Summary --
echo "-----------------------------------------------------------"
echo -e "  ${GREEN}${PASS} passed${NC}  ${RED}${FAIL} failed${NC}  ${YELLOW}${WARN} warnings${NC}"
echo ""

if [[ "$FAIL" -gt 0 ]]; then
    echo -e "  ${RED}HEALTH CHECK FAILED${NC} — fix the above before trusting any gate run here."
    if [[ -f "$WORKTREE_DIR/.claude/worktree-setup.log" ]]; then
        echo -e "  ${YELLOW}Setup output was captured:${NC} .claude/worktree-setup.log"
    fi
    exit 1
fi

echo -e "  ${GREEN}HEALTH CHECK PASSED${NC}"
exit 0
