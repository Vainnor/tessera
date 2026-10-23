#!/bin/bash
# SessionStart: what the agent needs before its first piece of work.
# SessionStart stdout is added to the agent's context. Tests: scripts/lib/session-context.test.sh.

# Every hook runs from the primary checkout's .claude/hooks (settings.json resolves it through the git common
# dir), so a primary checkout off main, or on a commit without the gates, runs old gates or none.
COMMON="$(git -C "${CLAUDE_PROJECT_DIR:-.}" rev-parse --path-format=absolute --git-common-dir 2>/dev/null)"
if [[ -n "$COMMON" ]]; then
    PRIMARY="$(dirname "$COMMON")"
    PRIMARY_BRANCH="$(git -C "$PRIMARY" symbolic-ref --quiet --short HEAD 2>/dev/null)"
    PROBLEM=""
    if [[ "$PRIMARY_BRANCH" != "main" ]]; then
        PROBLEM="is on ${PRIMARY_BRANCH:-a detached HEAD}, not main"
    fi
    if [[ ! -f "$PRIMARY/.claude/hooks/master-protection-gate.sh" ]]; then
        PROBLEM="${PROBLEM:+$PROBLEM and }lacks .claude/hooks/master-protection-gate.sh"
    fi
    if [[ -n "$PROBLEM" ]]; then
        WARNING="WARNING: the primary checkout ${PRIMARY} ${PROBLEM}. Every Claude Code hook runs from there, so the gates are stale or missing (a missing hook blocks every tool call it guards). Tell the operator: the primary checkout must be on main with the current hooks (git -C ${PRIMARY} switch main && git -C ${PRIMARY} pull --ff-only origin main)."
        echo "$WARNING"
        echo "$WARNING" >&2
    fi
fi

echo "BUILD: Cargo builds into this checkout's own target/ (bash scripts/setup-worktree.sh seeds a worktree's from the primary checkout's). Run the full gate once per commit (pnpm gate); a targeted cargo test -p or pnpm --filter run is always fine."
exit 0
