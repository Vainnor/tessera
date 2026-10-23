#!/bin/bash
# ==============================================================================
# Tests for .claude/hooks/session-context.sh and its SessionStart registration.
#
#   bash scripts/lib/session-context.test.sh
# ==============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HOOK="$ROOT/.claude/hooks/session-context.sh"

PASS=0
FAIL=0
ok()  { echo "  PASS: $1"; PASS=$((PASS + 1)); }
bad() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

# Every run is a session in a worktree of a scratch primary checkout on main with the gates; the primary-checkout
# cases below change it.
WORK="$(cd "$(mktemp -d "${TMPDIR:-/tmp}/session-context-test.XXXXXX")" && pwd -P)"
trap 'rm -rf "$WORK"' EXIT
export GIT_AUTHOR_NAME=T GIT_AUTHOR_EMAIL=t@e.com GIT_COMMITTER_NAME=T GIT_COMMITTER_EMAIL=t@e.com GIT_CONFIG_NOSYSTEM=1
P="$WORK/primary"
git init -q -b main "$P" && mkdir -p "$P/.claude/hooks" && : > "$P/.claude/hooks/master-protection-gate.sh"
git -C "$P" add -A && git -C "$P" -c core.hooksPath=/dev/null commit -qm init
git -C "$P" worktree add -q -b feat/1/some-work "$WORK/wt" 2>/dev/null
export CLAUDE_PROJECT_DIR="$WORK/wt"

out="$(bash "$HOOK" </dev/null)"
status=$?
if (( status == 0 )); then ok "exit 0"; else bad "exit 0 (got $status)"; fi
if printf '%s' "$out" | grep -q '^BUILD: .*pnpm gate'; then ok "names the full gate"; else bad "names the full gate ($out)"; fi
if ! printf '%s' "$out" | grep -qi 'sccache\|TaskCreate'; then ok "carries nothing OIS-specific"; else bad "OIS leftovers ($out)"; fi
registered="$(python3 -c 'import json,sys
hooks = json.load(open(sys.argv[1])).get("hooks", {}).get("SessionStart", [])
print(any("session-context.sh" in h.get("command", "") for entry in hooks for h in entry.get("hooks", [])))' "$ROOT/.claude/settings.json")"
if [[ "$registered" == "True" ]]; then ok "registered under SessionStart"; else bad "registered under SessionStart"; fi

(unset CLAUDE_ENV_FILE; bash "$HOOK" </dev/null >/dev/null)
if [[ $? -eq 0 ]]; then ok "no \$CLAUDE_ENV_FILE: exit 0"; else bad "no \$CLAUDE_ENV_FILE failed"; fi
if ! grep -q 'CLAUDE_ENV_FILE' "$HOOK"; then ok "exports no build environment"; else bad "the hook writes \$CLAUDE_ENV_FILE"; fi

echo "The primary checkout every hook runs from:"
# context DIR -> sets CTX (stdout, the agent's context), ERR (stderr) and RC for a session in DIR
context() {
    CTX="$(CLAUDE_PROJECT_DIR="$1" bash "$HOOK" </dev/null 2>"$WORK/err")"; RC=$?; ERR="$(cat "$WORK/err")"
}
context "$WORK/wt"
if [[ $RC -eq 0 && "$CTX" != *WARNING* && -z "$ERR" ]]; then ok "positive control: primary on main with the gates, no warning"; else bad "warned on a healthy primary (rc=$RC: $CTX $ERR)"; fi
git -C "$P" -c core.hooksPath=/dev/null checkout -q -b feat/2/elsewhere
context "$WORK/wt"
if [[ "$CTX" == *"WARNING: the primary checkout $P is on feat/2/elsewhere, not main"* ]]; then ok "a primary off main is a warning in the agent's context"; else bad "no warning for a primary off main ($CTX)"; fi
if [[ "$ERR" == *"is on feat/2/elsewhere, not main"* ]]; then ok "...and on stderr"; else bad "...stderr ($ERR)"; fi
if [[ $RC -eq 0 ]]; then ok "...and the hook still exits 0"; else bad "...exit $RC"; fi
git -C "$P" -c core.hooksPath=/dev/null checkout -q main
rm "$P/.claude/hooks/master-protection-gate.sh"
context "$WORK/wt"
if [[ "$CTX" == *"WARNING: the primary checkout $P lacks .claude/hooks/master-protection-gate.sh"* ]]; then ok "a primary without master-protection-gate.sh is a warning"; else bad "no warning for a missing gate ($CTX)"; fi
context "$P"
if [[ "$CTX" == *"lacks .claude/hooks/master-protection-gate.sh"* ]]; then ok "...also for a session in the primary itself"; else bad "primary session ($CTX)"; fi

echo ""
echo "session-context: $PASS passed, $FAIL failed"
(( FAIL == 0 ))
