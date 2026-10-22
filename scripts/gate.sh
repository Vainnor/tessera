#!/usr/bin/env bash
# The full local gate: what CI runs, in one command (`pnpm gate`). Each step's output goes to a
# log file; the summary at the end is the result. A step whose tool is missing reports MISSING,
# which is not a pass.
set -uo pipefail

cd "$(dirname "$0")/.."
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
step "cargo tests"         cargo-nextest cargo nextest run --workspace --locked
step "cargo doc tests"     cargo cargo test --workspace --doc --locked
step "cargo deny"          cargo-deny cargo deny --locked check
step "cargo audit"         cargo-audit cargo audit
step "secret scan"         gitleaks gitleaks git --redact --no-banner

echo
printf '%s\n' "${results[@]}"
echo "logs: $logs"
exit "$failed"
