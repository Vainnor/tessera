#!/bin/bash
# PreToolUse hook: Block git merge/rebase/pull of any branch but main.
# Prevents accidental contamination from other feature branches.
#
# Blocking contract: Claude Code blocks a tool call ONLY on `exit 2`, and reads
# stderr when it does — stdout is discarded. Exit 1 is a non-blocking error and
# the tool runs anyway. `exec >&2` keeps every message on the right stream.
#
# Tests: scripts/lib/branch-ancestry-gate.test.sh
exec >&2

HOOK_LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib/tool-input.sh"
if [[ ! -f "$HOOK_LIB" ]]; then
    echo "BLOCKED: cannot locate .claude/hooks/lib/tool-input.sh — refusing to pass the command unchecked."
    exit 2
fi
# shellcheck source=lib/tool-input.sh
. "$HOOK_LIB"

INPUT=$(cat)

# Fails closed: a parse fault used to print '' and exit 0, which matched none of
# the patterns below and allowed the merge through.
COMMAND="$(printf '%s' "$INPUT" | require_tool_input_field command)" || exit 2

# The line is parsed, not grepped (git_policy.py): every merge, rebase or pull it runs
# is checked, wherever it sits in the line and whatever options come before the
# source, and a heredoc or quoted argument naming one is data. `merge-base` is a
# read, not a merge. One naming no source integrates the current branch's upstream,
# so git resolves it here and only main passes. Any exit other than 0
# (unparseable line, python3 missing) blocks.
policy "$(dirname "${BASH_SOURCE[0]}")/lib/git_policy.py" non-main-integration
STATUS=$?
TARGET="$POLICY_OUT"
if [[ $STATUS -ne 0 ]]; then
    echo "BLOCKED: ${POLICY_WHY:-the command could not be parsed}, so a merge, rebase or pull in it cannot be checked."
    exit 2
fi
if [[ -z "$TARGET" ]]; then
    exit 0
fi

# Block everything else — this is a feature-branch merge attempt.
echo ""
echo "BLOCKED: Merging/rebasing '$TARGET' is not allowed; only main is."
echo ""
echo "This prevents cross-branch contamination. Feature branches must ONLY"
echo "incorporate changes from main. If you need changes from another branch,"
echo "land that branch on main first."
echo ""
echo "Allowed (name the source; a bare pull, merge or rebase takes the upstream, which may not be main):"
echo "  git rebase origin/main  (only before the branch is first pushed)"
echo "  git merge origin/main   (once the branch is pushed)"
echo "  git pull origin main"
echo ""
echo "If you believe this is a false positive, ask the user for confirmation."
exit 2
