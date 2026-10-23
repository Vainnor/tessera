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
# One line per fact about each job with a `bash scripts/test-hooks.sh` step, plus the workflow's
# top-level permissions. Two such jobs repeat the job lines, so the exact match below fails.
hook_job="$(python3 -I - "$ROOT/.github/workflows/ci.yml" <<'PY2'
import re, sys
head, jobs = open(sys.argv[1]).read().split("\njobs:\n", 1)
print("top permissions: " + ("{}" if re.search(r"^permissions: \{\}$", head, re.M) else "not {}"))
for block in re.split(r"\n(?=  [a-z][a-z0-9-]*:\n)", "\n" + jobs + "\n"):
    if not re.search(r"^      - run: bash scripts/test-hooks\.sh$", block, re.M):
        continue
    m = re.search(r"^    runs-on: (.+)$", block, re.M)
    print("runs-on: " + (m.group(1).strip() if m else "none"))
    m = re.search(r"^    permissions:\n((?:      .+\n)*)", block, re.M)
    print("permissions: " + (" ".join(m.group(1).split()) if m else "none"))
    for uses in re.findall(r"^ +(?:- )?uses: (.+)$", block, re.M):
        pinned = re.fullmatch(r"[\w.-]+/[\w.-]+@[0-9a-f]{40} # v[0-9][\w.]*", uses.strip())
        print(("pinned: " if pinned else "not pinned: ") + uses.split("@")[0])
    print("persist-credentials: " + ("false" if re.search(r"^ +persist-credentials: false$", block, re.M) else "not false"))
    # Either key would let the job pass while the tests fail, or skip it.
    for key in ("if", "continue-on-error"):
        if re.search(r"^ +(?:- )?" + key + r":", block, re.M):
            print("has " + key)
PY2
)"
want_ac1='runs-on: ubuntu-24.04'
want_ac2=$'top permissions: {}\npermissions: contents: read\npinned: actions/checkout\npersist-credentials: false'
got_ac1="$(printf '%s\n' "$hook_job" | grep -E '^(runs-on:|has) ')"
got_ac2="$(printf '%s\n' "$hook_job" | grep -Ev '^(runs-on:|has) ')"
if [[ "$got_ac1" == "$want_ac1" ]]; then ok "one job runs bash scripts/test-hooks.sh, on ubuntu-24.04, with no if or continue-on-error"; else bad "hook-tests job: $(printf '%s' "$hook_job" | tr '\n' ';')"; fi
if [[ "$got_ac2" == "$want_ac2" ]]; then ok "that job reads contents only, under permissions: {}, with a SHA-pinned checkout that keeps no credentials"; else bad "hook-tests job: $(printf '%s' "$hook_job" | tr '\n' ';')"; fi

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
