#!/bin/bash
# PreToolUse hook: Block PR creation unless the full suite passed on the commit the PR opens on:
# its tests marker from scripts/gate.sh, or every CI job green on it (scripts/lib/ci-checks.sh). The commit is
# the current branch, or the branch `--head` names, as origin holds it.
#
# Blocking contract: Claude Code blocks a tool call ONLY on `exit 2`, reading
# stderr. check_marker_for_head returns 1 on a missing marker, so the gate maps
# that to 2. `exec >&2` puts its BLOCKED message on the stream Claude reads.
exec >&2

HOOK_LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/tool-input.sh"
if [[ ! -f "$HOOK_LIB" ]]; then
    echo "BLOCKED: cannot locate .claude/hooks/lib/tool-input.sh — refusing to pass the PR unchecked."
    exit 2
fi
# shellcheck source=lib/tool-input.sh
. "$HOOK_LIB"

INPUT=$(cat)

# Fails closed: a parse fault used to print '' and exit 0, allowing PR creation.
COMMAND="$(printf '%s' "$INPUT" | require_tool_input_field command)" || exit 2

# Only gate PR creation (gh pr create, its aliases, or a gh api call that opens a PR). git_policy.py
# parses the line, so a heredoc or quoted argument mentioning the command does not trip the gate.
# It prints nothing for no PR, `=BRANCH` for the head (`=` alone: the current branch) or `?REASON`,
# reading the repository in the current directory. Any other exit (unparseable, no python3) blocks.
pr_head_of_command() {
    policy "$HOOK_DIR/lib/git_policy.py" pr-head && PR_HEAD="$POLICY_OUT" && return 0
    echo "BLOCKED: ${POLICY_WHY:-the command could not be parsed}, so a PR it opens cannot be checked."
    exit 2
}
HOOK_DIR="$(dirname "${BASH_SOURCE[0]}")"
HOOK_DIR="$(cd "$HOOK_DIR" && pwd)"
pr_head_of_command
[[ -n "$PR_HEAD" ]] || exit 0

# The session cwd is shared across worktrees, so `cd DIR && gh pr create` is checked in DIR. command_match.py
# follows only that exact shape; git_policy.py pr-head refuses any other cd, -C or GIT_DIR on the line.
CD_DIR="$(printf '%s' "$COMMAND" | python3 -I "$HOOK_DIR/lib/command_match.py" --cd-target gh pr create 2>/dev/null)"
if [[ -n "$CD_DIR" ]]; then
    cd "$CD_DIR" 2>/dev/null || { echo "BLOCKED: cannot cd to $CD_DIR to check its tests marker."; exit 2; }
    pr_head_of_command
fi
case "$PR_HEAD" in
    '?'*) echo "BLOCKED: ${PR_HEAD#\?}."; exit 2 ;;
    '='*) ;;
    *) echo "BLOCKED: unexpected PR head '$PR_HEAD' from git_policy.py."; exit 2 ;;
esac

# Libraries come from beside this hook, which settings.json always runs from the primary
# checkout, never from the branch under test: a branch must not ship its own lenient checker.
MAIN_ROOT="$(cd "$HOOK_DIR/../.." && pwd)"

# shellcheck source=../../scripts/check-marker.sh
if ! source "$MAIN_ROOT/scripts/check-marker.sh" 2>/dev/null || ! declare -F check_marker_for_head >/dev/null \
    || ! declare -F pr_head_sha >/dev/null; then
    echo "BLOCKED: cannot load check_marker_for_head from scripts/check-marker.sh — refusing to open a PR without checking the test marker."
    exit 2
fi
pr_head_sha "${PR_HEAD#=}" || exit 2

CLAUDE_ONLY_LIB="$MAIN_ROOT/scripts/lib/claude-only-change.sh"
if [[ -f "$CLAUDE_ONLY_LIB" ]]; then
    # shellcheck source=../../scripts/lib/claude-only-change.sh
    . "$CLAUDE_ONLY_LIB"
    if claude_only_change "$PR_HEAD_SHA"; then
        echo "test-run-gate: skipped — the PR changes only agent guidance."
        exit 0
    fi
fi

# The local marker, or else every CI job green on the same commit (scripts/lib/ci-checks.sh).
MARKER_WHY="$(check_marker_for_head "tests" "Tests" "pnpm gate" "$PR_HEAD_SHA")" && exit 0
CI_LIB="$MAIN_ROOT/scripts/lib/ci-checks.sh"
CI_WHY="cannot load scripts/lib/ci-checks.sh."
# shellcheck source=../../scripts/lib/ci-checks.sh
if [[ -f "$CI_LIB" ]] && . "$CI_LIB" && declare -F ci_green_for_sha >/dev/null; then
    CI_WHY="$(ci_green_for_sha "$PR_HEAD_SHA")" && exit 0
fi
echo "$MARKER_WHY"
echo "Nor does CI vouch for it: $CI_WHY"
exit 2
