#!/bin/bash
# ==============================================================================
# Tests for scripts/lib/ci-checks.sh: that its required checks are exactly the jobs
# in .github/workflows/ci.yml, that its jq filter keeps only GitHub Actions' runs,
# and that a malformed sha never reaches gh.
#
#   bash scripts/lib/ci-checks.test.sh
#
# The gate's pass/block cases are in test-run-gate.test.sh. No network: gh is a stub.
# ==============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/ci-checks-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

PASS=0
FAIL=0
ok()  { echo "  PASS: $1"; PASS=$((PASS + 1)); }
bad() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

# shellcheck source=ci-checks.sh
. "$ROOT/scripts/lib/ci-checks.sh"

echo "The required checks are ci.yml's jobs:"
# Each job's name:, with a matrix job expanded once per os value, as GitHub names the check-runs.
python3 -I - "$ROOT/.github/workflows/ci.yml" >"$WORK/want" <<'PY'
import re, sys
text = open(sys.argv[1]).read()
jobs = text.split("\njobs:\n", 1)[1]
for block in re.split(r"\n(?=  [a-z][a-z0-9-]*:\n)", "\n" + jobs):
    m = re.search(r"^    name: (.+)$", block, re.M)
    if not m:
        continue
    name = m.group(1).strip()
    os_list = re.search(r"^        os: \[(.+)\]$", block, re.M)
    if "${{ matrix.os }}" in name and os_list:
        for os_ in os_list.group(1).split(","):
            print(name.replace("${{ matrix.os }}", os_.strip()))
    else:
        print(name)
PY
printf '%s\n' "$CI_REQUIRED_CHECKS" | sort >"$WORK/have"
sort -o "$WORK/want" "$WORK/want"
if [[ -s "$WORK/want" ]] && cmp -s "$WORK/want" "$WORK/have"; then ok "CI_REQUIRED_CHECKS matches ci.yml ($(wc -l <"$WORK/want" | tr -d ' ') jobs)"; else bad "drift: $(diff "$WORK/want" "$WORK/have" | tr '\n' ' ')"; fi

echo "The hook tests run in CI:"
# The runs-on of every job with a `bash scripts/test-hooks.sh` step: want exactly one, on Linux.
hook_jobs="$(python3 -I - "$ROOT/.github/workflows/ci.yml" <<'PY2'
import re, sys
jobs = open(sys.argv[1]).read().split("\njobs:\n", 1)[1]
for block in re.split(r"\n(?=  [a-z][a-z0-9-]*:\n)", "\n" + jobs):
    if re.search(r"^      - run: bash scripts/test-hooks\.sh$", block, re.M):
        m = re.search(r"^    runs-on: (.+)$", block, re.M)
        print(m.group(1).strip() if m else "(no runs-on)")
PY2
)"
if [[ "$hook_jobs" == "ubuntu-24.04" ]]; then ok "one job runs bash scripts/test-hooks.sh, on ubuntu-24.04"; else bad "jobs running the hook tests, by runs-on: '$hook_jobs'"; fi

echo "Only GitHub Actions' check-runs count:"
if command -v jq >/dev/null 2>&1; then
    printf '%s' '{"check_runs":[
      {"id":1,"name":"Rust format","status":"completed","conclusion":"success","app":{"slug":"github-actions"}},
      {"id":2,"name":"Rust format","status":"completed","conclusion":"success","app":{"slug":"someone-else"}},
      {"id":3,"name":"JS lint, test, build","status":"queued","conclusion":null,"app":{"slug":"github-actions"}}]}' >"$WORK/runs.json"
    out="$(jq -r "$CI_RUNS_JQ" "$WORK/runs.json")"
    if [[ "$out" == $'1\tRust format\tcompleted\tsuccess\n3\tJS lint, test, build\tqueued\t' ]]; then ok "another app's run is dropped, a null conclusion is empty"; else bad "jq output: $out"; fi
else
    bad "jq is not installed, so the filter could not be checked"
fi

echo "A malformed sha never reaches gh:"
mkdir -p "$WORK/bin"
printf '#!/bin/bash\necho called >> "%s/gh.log"\nexit 0\n' "$WORK" >"$WORK/bin/gh"
chmod +x "$WORK/bin/gh"
for sha in "" abc1234 "$(printf 'a%.0s' {1..39})x" "$(printf 'a%.0s' {1..40})/../../x" "$(printf 'A%.0s' {1..40})"; do
    out="$(PATH="$WORK/bin:$PATH" ci_green_for_sha "$sha")"; rc=$?
    if [[ $rc -eq 1 && "$out" == *"not a full commit sha"* && ! -e "$WORK/gh.log" ]]; then ok "refused: '${sha:0:20}'"; else bad "sha '$sha': rc=$rc $out"; fi
done
out="$(PATH="$WORK/bin:$PATH" ci_green_for_sha "$(printf 'a%.0s' {1..40})")"; rc=$?
if [[ $rc -eq 1 && -e "$WORK/gh.log" && "$out" == *"(no run)"* ]]; then ok "control: a full sha calls gh, and no runs is not green"; else bad "control: rc=$rc $out"; fi

echo ""
echo "ci-checks: $PASS passed, $FAIL failed"
(( FAIL == 0 ))
