#!/bin/bash
# CI as an alternative to the local tests marker: did every job in .github/workflows/ci.yml pass on
# this exact commit?
#
#   . scripts/lib/ci-checks.sh; ci_green_for_sha <sha>
#
# Used by .claude/hooks/test-run-gate.sh when .claude/markers/tests/<sha> is missing. Reads the
# commit's check-runs from GitHub (a read; nothing is written) and passes only when every name in
# CI_REQUIRED_CHECKS has a check-run from the GitHub Actions app and the newest such run is
# completed with conclusion success. Only GitHub Actions' runs count: any app with checks:write can
# create a check-run under any name. CI runs on pull_request and on pushes to main, so this only
# helps for a commit CI already tested; a fresh branch still needs `pnpm gate`.
#
# Fails closed: no gh, a failed call, a missing job, or a job pending, skipped or failed is not green.
# Returns 0 when green; 1 otherwise with the reason on stdout.
# Tests: scripts/lib/ci-checks.test.sh (which also checks this list against ci.yml).

CI_REPO="Vainnor/tessera"
# One per job in ci.yml, matrix jobs once per value, as GitHub names their check-runs.
CI_REQUIRED_CHECKS="JS lint, test, build
Website Lighthouse budget
Rust format
Hook tests
Clippy and tests (ubuntu-24.04)
Clippy and tests (macos-latest)
Clippy and tests (windows-latest)
cargo-deny and cargo-audit
Secret scan (full history)"

# id, name, status, conclusion per GitHub Actions check-run.
CI_RUNS_JQ='.check_runs[] | select(.app.slug == "github-actions") | [.id, .name, .status, (.conclusion // "")] | @tsv'

ci_green_for_sha() {
    local sha="$1" runs name latest missing=""
    if [[ ! "$sha" =~ ^[0-9a-f]{40}$ ]]; then
        echo "CI: '$sha' is not a full commit sha."
        return 1
    fi
    command -v gh >/dev/null 2>&1 || { echo "CI: gh is not installed, so CI cannot be checked."; return 1; }
    if ! runs="$(gh api "repos/$CI_REPO/commits/$sha/check-runs?per_page=100" --jq "$CI_RUNS_JQ" 2>/dev/null)"; then
        echo "CI: could not read the check-runs for ${sha:0:8} from GitHub."
        return 1
    fi
    while IFS= read -r name; do
        [[ -n "$name" ]] || continue
        # The newest run of this job: a re-run after a failure supersedes it.
        latest="$(printf '%s\n' "$runs" | N="$name" awk -F'\t' '$2 == ENVIRON["N"] && $1 + 0 > best { best = $1 + 0; line = $3 "/" $4 } END { print line }')"
        if [[ "$latest" != "completed/success" ]]; then
            missing+="${missing:+, }$name (${latest:-no run})"
        fi
    done <<<"$CI_REQUIRED_CHECKS"
    if [[ -n "$missing" ]]; then
        echo "CI is not green on ${sha:0:8}: $missing."
        return 1
    fi
    return 0
}
