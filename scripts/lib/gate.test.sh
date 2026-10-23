#!/bin/bash
# ==============================================================================
# Tests for the tests-marker stamping in scripts/gate.sh.
#
#   bash scripts/lib/gate.test.sh
#
# Runs the real gate in a throwaway repo with every tool it calls stubbed on PATH
# (pnpm, cargo, cargo-deny, cargo-audit, gitleaks; the hook tests are a stub script),
# so the steps are instant and each can be made to fail. Checks that only a green run
# on a clean, unmoved HEAD writes .claude/markers/tests/<sha> with its manifest line.
# ==============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/gate-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR
export GIT_AUTHOR_NAME=T GIT_AUTHOR_EMAIL=t@e.com GIT_COMMITTER_NAME=T GIT_COMMITTER_EMAIL=t@e.com
export GIT_CONFIG_NOSYSTEM=1 HOME="$WORK"

PASS=0
FAIL=0
ok()  { echo "  PASS: $1"; PASS=$((PASS + 1)); }
bad() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

# Stubs: each exits 0 unless its name and first argument are listed in $WORK/fail (`cargo clippy`),
# and `cargo test` can move HEAD or dirty the tree mid-run when $WORK/during says so.
STUBS="$WORK/stubs"
mkdir -p "$STUBS"
for tool in pnpm cargo cargo-deny cargo-audit gitleaks; do
    cat >"$STUBS/$tool" <<STUB
#!/bin/bash
grep -qxF "$tool \${1:-}" "$WORK/fail" 2>/dev/null && exit 1
if [[ "$tool \${1:-}" == "cargo test" && -f "$WORK/during" ]]; then
    case "\$(cat "$WORK/during")" in
        commit) git -c core.hooksPath=/dev/null commit -q --allow-empty -m moved ;;
        dirty) echo x > dirtied.txt ;;
    esac
fi
exit 0
STUB
    chmod +x "$STUBS/$tool"
done

REPO="$WORK/repo"
mkdir -p "$REPO/scripts/lib"
cp "$ROOT/scripts/gate.sh" "$REPO/scripts/gate.sh"
cp "$ROOT/scripts/lib/marker_manifest.py" "$ROOT/scripts/lib/file_lock.py" "$REPO/scripts/lib/"
printf '#!/bin/bash\ngrep -qxF "hooks x" "%s/fail" 2>/dev/null && exit 1\nexit 0\n' "$WORK" >"$REPO/scripts/test-hooks.sh"
printf '.claude/markers/\n__pycache__/\n' >"$REPO/.gitignore"
git init -q -b main "$REPO"
git -C "$REPO" add -A && git -C "$REPO" -c core.hooksPath=/dev/null commit -q -m init

# gate: runs the gate in the fixture; sets OUT and RC.
gate() {
    OUT="$(cd "$REPO" && PATH="$STUBS:$PATH" bash scripts/gate.sh 2>&1)"
    RC=$?
}
marker_for() { python3 -I "$ROOT/scripts/lib/marker_manifest.py" vouched "$REPO" tests "$1"; }
reset() { rm -rf "$REPO/.claude/markers" "$WORK/fail" "$WORK/during" "$REPO/dirtied.txt"; }

echo "A green run on a clean tree stamps HEAD:"
reset
SHA="$(git -C "$REPO" rev-parse HEAD)"
gate
if [[ $RC -eq 0 ]]; then ok "exits 0"; else bad "exit $RC (${OUT: -300})"; fi
if marker_for "$SHA"; then ok "tests marker for HEAD, vouched by its manifest line"; else bad "no vouched marker (${OUT: -300})"; fi
if [[ "$OUT" == *"tests marker written for ${SHA:0:8}"* ]]; then ok "says so"; else bad "message (${OUT: -200})"; fi
if [[ "$OUT" == *"PASS     hook tests"* ]]; then ok "the hook tests are a gate step"; else bad "no hook tests step"; fi

echo "Anything short of that stamps nothing:"
for case in "cargo clippy" "pnpm lint" "gitleaks git" "hooks x"; do
    reset
    printf '%s\n' "$case" >"$WORK/fail"
    gate
    if [[ $RC -ne 0 ]] && ! marker_for "$SHA" && [[ "$OUT" == *"no tests marker written (a step did not pass)"* ]]; then
        ok "a failing step ($case): exit $RC, no marker"
    else
        bad "failing $case (rc=$RC: ${OUT: -200})"
    fi
done

reset
echo x >"$REPO/untracked.txt"
gate
if [[ $RC -eq 0 ]] && ! marker_for "$SHA" && [[ "$OUT" == *"not clean at the start"* ]]; then ok "a dirty tree: green, no marker"; else bad "dirty tree (rc=$RC: ${OUT: -200})"; fi
rm -f "$REPO/untracked.txt"

reset
echo dirty >"$WORK/during"
gate
if ! marker_for "$SHA" && [[ "$OUT" == *"changed during the run"* ]]; then ok "a tree dirtied mid-run: no marker"; else bad "dirtied mid-run (rc=$RC: ${OUT: -200})"; fi
rm -f "$REPO/dirtied.txt"

reset
echo commit >"$WORK/during"
gate
MOVED="$(git -C "$REPO" rev-parse HEAD)"
if ! marker_for "$SHA" && ! marker_for "$MOVED" && [[ "$OUT" == *"HEAD moved during the run"* ]]; then ok "HEAD moved mid-run: no marker for either commit"; else bad "moved HEAD (rc=$RC: ${OUT: -200})"; fi

reset
# Without the stub, and without any PATH directory that holds a real gitleaks.
rm "$STUBS/gitleaks"
BARE="$STUBS"
IFS=: read -ra DIRS <<<"$PATH"
for dir in "${DIRS[@]}"; do [[ -x "$dir/gitleaks" ]] || BARE="$BARE:$dir"; done
OUT="$(cd "$REPO" && PATH="$BARE" bash scripts/gate.sh 2>&1)"
RC=$?
if [[ $RC -ne 0 ]] && [[ "$OUT" == *"MISSING  secret scan"* ]] && ! marker_for "$MOVED"; then ok "a missing tool is MISSING and stamps nothing"; else bad "missing tool (rc=$RC: ${OUT: -200})"; fi

echo ""
echo "gate: $PASS passed, $FAIL failed"
(( FAIL == 0 ))
