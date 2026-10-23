#!/bin/bash
# Runs every shell test of the Claude Code hooks and the workflow scripts (scripts/lib/*.test.sh).
#
#   bash scripts/test-hooks.sh                 # all of them
#   bash scripts/test-hooks.sh pre-pr-gate     # scripts/lib/pre-pr-gate.test.sh only
#
# Each test file ends with `<name>: N passed, M failed` and exits non-zero on a failure. A file
# passes only when it exits 0 AND that line reports at least one pass and no failure
# (.claude/rules/testing.md → "Reading results"): a test that died before its summary passes
# nothing. Under a file that fails go its FAIL lines, or its last ten lines when it has none,
# since CI keeps no log file. The last line is the total. Tests: scripts/lib/test-hooks.test.sh.
set -uo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1
command -v python3 >/dev/null 2>&1 || { echo "test-hooks: python3 is required" >&2; exit 1; }

if [[ $# -gt 0 ]]; then
    files=()
    for name in "$@"; do
        # A test name, never a path: `../x` would run a script outside scripts/lib.
        [[ "$name" =~ ^[A-Za-z0-9_-]+(\.test\.sh)?$ ]] \
            || { echo "test-hooks: not a test name: $name" >&2; exit 1; }
        files+=("scripts/lib/${name%.test.sh}.test.sh")
    done
else
    files=(scripts/lib/*.test.sh)
fi

logs="$(mktemp -d "${TMPDIR:-/tmp}/tessera-hook-tests.XXXXXX")"
total_pass=0
total_fail=0
bad_files=()
for file in "${files[@]}"; do
    [[ -f "$file" ]] || { echo "test-hooks: no such test: $file" >&2; bad_files+=("$file (missing)"); continue; }
    name="$(basename "$file" .test.sh)"
    log="$logs/$name.log"
    bash "$file" >"$log" 2>&1
    rc=$?
    summary="$(grep -E '^[^ ]+: [0-9]+ passed, [0-9]+ failed$' "$log" | tail -1)"
    passed=0 failed=0
    if [[ "$summary" =~ ([0-9]+)\ passed,\ ([0-9]+)\ failed$ ]]; then
        passed="${BASH_REMATCH[1]}" failed="${BASH_REMATCH[2]}"
    fi
    total_pass=$((total_pass + passed))
    total_fail=$((total_fail + failed))
    if [[ $rc -ne 0 || -z "$summary" || "$failed" -ne 0 || "$passed" -eq 0 ]]; then
        printf '%-28s FAIL (exit %s, %s; log: %s)\n' "$name" "$rc" "${summary:-no summary line}" "$log"
        # The failed cases, or the end of the output when none is marked: a CI runner keeps no log.
        if grep -q 'FAIL' "$log"; then grep 'FAIL' "$log"; else tail -n 10 "$log"; fi | sed 's/^/    /'
        bad_files+=("$name")
    else
        printf '%-28s %s passed\n' "$name" "$passed"
    fi
done

echo "hook tests: ${#files[@]} files, $total_pass passed, $total_fail failed, ${#bad_files[@]} file(s) not passing"
[[ ${#bad_files[@]} -eq 0 ]]
