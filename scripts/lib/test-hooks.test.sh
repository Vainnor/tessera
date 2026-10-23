#!/bin/bash
# ==============================================================================
# Tests for scripts/test-hooks.sh, the runner CI's "Hook tests" job calls: it
# exits non-zero when any test file fails, and prints that file's failing cases.
#
#   bash scripts/lib/test-hooks.test.sh
#
# Runs a copy of the runner in a scratch tree whose scripts/lib holds planted
# test files, so no real test runs here.
# ==============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/test-hooks-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

PASS=0
FAIL=0
ok()  { echo "  PASS: $1"; PASS=$((PASS + 1)); }
bad() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

mkdir -p "$WORK/scripts/lib"
cp "$ROOT/scripts/test-hooks.sh" "$WORK/scripts/test-hooks.sh"
plant() { printf '%s\n' "$2" >"$WORK/scripts/lib/$1.test.sh"; }
# run_with NAME BODY: the passing file plus one planted file; sets RC and OUT.
run_with() {
    rm -f "$WORK"/scripts/lib/*.test.sh
    plant good 'echo "good: 3 passed, 0 failed"'
    plant "$1" "$2"
    OUT="$(TMPDIR="$WORK" bash "$WORK/scripts/test-hooks.sh" 2>&1)"
    RC=$?
}

echo "A run fails when any file does not pass:"
run_with other 'echo "other: 2 passed, 0 failed"'
if [[ $RC -eq 0 && "$OUT" == *"2 files, 5 passed, 0 failed, 0 file(s) not passing"* ]]; then ok "positive control: every file passing exits 0"; else bad "all passing: rc=$RC $OUT"; fi
run_with bad 'echo "  FAIL: the case that broke"; echo "bad: 1 passed, 1 failed"; exit 1'
if [[ $RC -eq 1 && "$OUT" == *"1 file(s) not passing"* ]]; then ok "a file with a failed case exits 1"; else bad "failed case: rc=$RC $OUT"; fi
run_with bad 'echo "bad: 1 passed, 1 failed"; exit 0'
if [[ $RC -eq 1 ]]; then ok "a failure count exits 1 even when the file exits 0"; else bad "count with exit 0: rc=$RC"; fi
run_with bad 'echo "bad: 4 passed, 0 failed"; exit 3'
if [[ $RC -eq 1 ]]; then ok "a non-zero exit fails even with no failed case"; else bad "exit 3: rc=$RC"; fi
run_with bad 'exit 0'
if [[ $RC -eq 1 && "$OUT" == *"no summary line"* ]]; then ok "a file that dies before its summary exits 1"; else bad "no summary: rc=$RC $OUT"; fi
run_with bad 'echo "bad: 0 passed, 0 failed"'
if [[ $RC -eq 1 ]]; then ok "a file that passes nothing exits 1"; else bad "zero passed: rc=$RC"; fi

echo "A failing file's cases are printed, since CI keeps no log file:"
run_with bad 'echo "  FAIL: first broken case"; echo "  PASS: a FAIL-free case"; echo "  FAIL: second broken case"; echo "bad: 1 passed, 2 failed"; exit 1'
if [[ "$OUT" == *"FAIL: first broken case"* && "$OUT" == *"FAIL: second broken case"* ]]; then ok "every failed case is printed"; else bad "failed cases not all printed: $OUT"; fi
if [[ "$OUT" != *"a FAIL-free case"* ]]; then ok "a passing case that names FAIL is not printed"; else bad "passing case printed: $OUT"; fi
run_with bad 'for i in 1 2 3 4 5 6 7 8 9 10 11 12; do echo "output line $i"; done; echo "boom: unbound variable" >&2; exit 1'
if [[ "$OUT" == *"boom: unbound variable"* && "$OUT" == *"output line 4"$'\n'* && "$OUT" != *"output line 3"$'\n'* ]]; then ok "a file with no FAIL line shows its last ten lines"; else bad "crash output not the last ten lines: $OUT"; fi

echo ""
echo "test-hooks: $PASS passed, $FAIL failed"
(( FAIL == 0 ))
