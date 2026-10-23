#!/usr/bin/env bash
# The full local gate: what CI runs, in one command (`pnpm gate`). Each step's output goes to a
# log file; the summary at the end is the result. A step whose tool is missing reports MISSING,
# which is not a pass.
#
# A run where every step passed, on a committed HEAD whose tree was clean at the start and is
# unchanged at the end, writes the tests marker .claude/markers/tests/<HEAD sha> and its manifest
# line (scripts/lib/marker_manifest.py). .claude/hooks/test-run-gate.sh checks that marker before
# `gh pr create`. The gate's own process writes it, so a backgrounded or redirected run counts.
set -uo pipefail

cd "$(dirname "$0")/.."

# The porcelain status, or UNKNOWN when git cannot say: an unreadable tree never passes for clean.
tree_state() {
    git status --porcelain --untracked-files=normal 2>/dev/null || echo UNKNOWN
}
start_sha="$(git rev-parse --verify --quiet HEAD 2>/dev/null)"
start_tree="$(tree_state)"
logs="$(mktemp -d "${TMPDIR:-/tmp}/tessera-gate.XXXXXX")"
results=()
failed=0

step() {
    local name="$1" tool="$2"
    shift 2
    if ! command -v "$tool" >/dev/null 2>&1; then
        results+=("MISSING  $name ($tool not installed)")
        failed=1
        return
    fi
    local log="$logs/${name// /-}.log"
    printf '%-28s' "$name"
    if "$@" >"$log" 2>&1; then
        echo "pass"
        results+=("PASS     $name")
    else
        echo "FAIL (log: $log)"
        results+=("FAIL     $name  ->  $log")
        failed=1
    fi
}

# The desktop crate embeds apps/desktop/dist, so the JS build runs before the Rust steps.
step "pnpm install"        pnpm  pnpm install --frozen-lockfile
step "pnpm build"          pnpm  pnpm build
step "eslint"              pnpm  pnpm lint
step "prettier"            pnpm  pnpm format:check
step "typescript"          pnpm  pnpm typecheck
step "js tests"            pnpm  pnpm test
step "theme contrast"      pnpm  pnpm check:contrast
step "bundle size"         pnpm  pnpm check:size
step "pnpm audit"          pnpm  pnpm audit --audit-level=high
step "cargo fmt"           cargo cargo fmt --all -- --check
step "cargo clippy"        cargo cargo clippy --workspace --all-targets --locked -- -D warnings
step "cargo tests"         cargo cargo test --workspace --locked
step "cargo deny"          cargo-deny cargo deny --locked check
step "cargo audit"         cargo-audit cargo audit
step "secret scan"         gitleaks gitleaks git --redact --no-banner
step "hook tests"          python3 bash scripts/test-hooks.sh

echo
printf '%s\n' "${results[@]}"
echo "logs: $logs"

reason=""
if [[ "$failed" -ne 0 ]]; then
    reason="a step did not pass"
elif [[ -z "$start_sha" ]]; then
    reason="HEAD could not be read"
elif [[ -n "$start_tree" ]]; then
    reason="the working tree was not clean at the start"
elif [[ "$(git rev-parse --verify --quiet HEAD 2>/dev/null)" != "$start_sha" ]]; then
    reason="HEAD moved during the run"
elif [[ -n "$(tree_state)" ]]; then
    reason="the working tree changed during the run"
fi
if [[ -n "$reason" ]]; then
    echo "gate: no tests marker written ($reason)."
elif python3 -I scripts/lib/marker_manifest.py stamp "$(pwd -P)" tests "$start_sha"; then
    echo "gate: tests marker written for ${start_sha:0:8}."
else
    echo "gate: could not write the tests marker." >&2
fi
exit "$failed"
