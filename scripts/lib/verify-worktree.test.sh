#!/bin/bash
# ==============================================================================
# Tests for scripts/verify-worktree.sh: what it says about node_modules against the
# lockfile, the worktree's own target dir, the built desktop frontend, the branch name,
# the branch's merges and the installed git hooks.
#
#   bash scripts/lib/verify-worktree.test.sh
#
# Runs the script in a linked worktree of a scratch repository. Checks only the lines
# under test. Self-reports pass/fail, exits non-zero on any failure.
# ==============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCRIPT="$ROOT/scripts/verify-worktree.sh"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/verify-worktree-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR CARGO_TARGET_DIR CARGO_BUILD_TARGET_DIR
export GIT_AUTHOR_NAME=T GIT_AUTHOR_EMAIL=t@e.com GIT_COMMITTER_NAME=T GIT_COMMITTER_EMAIL=t@e.com
export GIT_CONFIG_NOSYSTEM=1 HOME="$WORK"

PASS=0
FAIL=0
ok()  { echo "  PASS: $1"; PASS=$((PASS + 1)); }
bad() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

MAIN="$WORK/main"
g() { git -C "$MAIN" -c core.hooksPath=/dev/null "$@"; }
mkdir -p "$MAIN/scripts/lib"
git init -q -b main "$MAIN"
printf 'node_modules/\napps/desktop/dist/\n' > "$MAIN/.gitignore"
# pnpm 11+ writes two YAML documents; node_modules/.pnpm/lock.yaml holds only the second.
printf -- '---\npnpmfileChecksum: x\n---\nlockfileVersion: 9.0\n' > "$MAIN/pnpm-lock.yaml"
g add -A && g commit -qm init
g update-ref refs/remotes/origin/main HEAD
g config core.hooksPath .githooks
WT="$WORK/wt/feat/1/thing"
g worktree add -q -b feat/1/the-thing "$WT"

# healthy: a pnpm-installed node_modules matching the lockfile, and a built desktop frontend.
healthy() {
    rm -rf "$WT/node_modules"; mkdir -p "$WT/node_modules/.pnpm" "$WT/apps/desktop/dist"
    printf 'lockfileVersion: 9.0\n' > "$WT/node_modules/.pnpm/lock.yaml"
    : > "$WT/apps/desktop/dist/index.html"
}
# verify: run the script in the worktree; sets OUT (colors stripped) and RC.
verify() {
    local esc
    esc="$(printf '\033')"
    OUT="$(cd "$WT" && bash "$SCRIPT" 2>&1 | sed "s/${esc}\[[0-9;]*m//g"; exit "${PIPESTATUS[0]}")"
    RC=$?
}
# has NAME TEXT / lacks NAME TEXT: against $OUT
has()   { case "$OUT" in *"$2"*) ok "$1" ;; *) bad "$1 (no '$2' in: $OUT)" ;; esac; }
lacks() { case "$OUT" in *"$2"*) bad "$1 ('$2' in: $OUT)" ;; *) ok "$1" ;; esac; }

echo "== a healthy worktree =="
healthy
verify
has "a linked worktree" "PASS Linked worktree"
has "node_modules matches the lockfile" "PASS node_modules matches pnpm-lock.yaml"
has "cargo builds into the worktree's own target dir" "PASS cargo builds into this checkout's own target/"
has "the desktop frontend is built" "PASS apps/desktop/dist is built"
has "the git hooks are installed" "PASS git hooks installed (core.hooksPath=.githooks)"
has "a conventional branch name" "PASS Branch name: feat/1/the-thing"
has "forks from origin/main" "PASS Forks from origin/main (0 commits behind its tip)"
has "no foreign merges" "PASS No merge commits from other branches"
[ "$RC" -eq 0 ] && ok "...and the health check exits 0" || bad "healthy exit $RC: $OUT"

echo "== node_modules against the lockfile =="
healthy; rm -rf "$WT/node_modules"; verify
has "no node_modules fails" "FAIL node_modules missing"
healthy; printf 'lockfileVersion: 9.0\n# older\n' > "$WT/node_modules/.pnpm/lock.yaml"; verify
has "node_modules installed from another lockfile fails" "FAIL node_modules was installed from a different pnpm-lock.yaml"
lacks "...and does not pass" "PASS node_modules matches"
healthy; rm "$WT/node_modules/.pnpm/lock.yaml"; verify
has "node_modules pnpm did not install fails" "FAIL node_modules was not installed by pnpm"
healthy; rm -rf "$WT/node_modules"; ln -s "$MAIN" "$WT/node_modules"; verify
has "a symlinked node_modules fails" "FAIL node_modules is a symlink"
rm "$WT/node_modules"; healthy

echo "== the desktop frontend =="
rm -rf "$WT/apps/desktop/dist"; verify
has "no apps/desktop/dist fails" "FAIL apps/desktop/dist missing — run: pnpm build"
[ "$RC" -ne 0 ] && ok "...and the health check exits non-zero" || bad "no dist exit $RC"
healthy

echo "== the worktree's own target dir =="
CARGO_TARGET_DIR="$(cd "$MAIN" && pwd -P)/target" verify
has "a CARGO_TARGET_DIR shared with the main checkout fails" "FAIL CARGO_TARGET_DIR=$(cd "$MAIN" && pwd -P)/target is exported"
[ "$RC" -ne 0 ] && ok "...and the health check exits non-zero" || bad "shared CARGO_TARGET_DIR exit $RC"
CARGO_TARGET_DIR="$(cd "$WT" && pwd -P)/target/" verify
has "a CARGO_TARGET_DIR naming the worktree's own target/ passes" "PASS cargo builds into this checkout's own target/"
CARGO_BUILD_TARGET_DIR="$WORK/shared-target" verify
has "a CARGO_BUILD_TARGET_DIR elsewhere fails too" "FAIL CARGO_BUILD_TARGET_DIR=$WORK/shared-target is exported"
lacks "...and does not pass the target check" "PASS cargo builds into this checkout's own target/"
[ "$RC" -ne 0 ] && ok "...and the health check exits non-zero" || bad "CARGO_BUILD_TARGET_DIR exit $RC"
CARGO_BUILD_TARGET_DIR="$(cd "$WT" && pwd -P)/target" verify
has "a CARGO_BUILD_TARGET_DIR naming the worktree's own target/ passes" "PASS cargo builds into this checkout's own target/"
verify
lacks "no CARGO_TARGET_DIR is no warning" "CARGO_TARGET_DIR"

echo "== the git hooks =="
g config --unset core.hooksPath; verify
has "no core.hooksPath fails and names the fix" "FAIL git hooks not installed (core.hooksPath is '', not .githooks): run git config core.hooksPath .githooks once per clone"
[ "$RC" -ne 0 ] && ok "...and the health check exits non-zero" || bad "no hooks exit $RC"
g config core.hooksPath /dev/null; verify
has "another core.hooksPath fails" "FAIL git hooks not installed (core.hooksPath is '/dev/null'"
g config core.hooksPath .githooks; verify
has "core.hooksPath=.githooks passes" "PASS git hooks installed"

echo "== branch name and ancestry =="
git -C "$WT" branch -q -m worktree-feat+1+thing; verify
has "a worktree-* branch is a warning before /start renames it" "WARN Branch 'worktree-feat+1+thing' is the EnterWorktree name"
git -C "$WT" branch -q -m bad_name; verify
has "another bad name fails" "FAIL Branch 'bad_name' does not match {type}/{issue-or-hash}/{description}"
git -C "$WT" branch -q -m chore/1617e87/a-name-long-enough-to-run-past-fiftyx; verify
has "a description over 4 words fails" "FAIL Branch 'chore/1617e87/a-name-long-enough-to-run-past-fiftyx' does not match"
git -C "$WT" branch -q -m feat/1/thing2; verify
has "a one-word description fails" "FAIL Branch 'feat/1/thing2' does not match"
git -C "$WT" branch -q -m chore/1617e87/aviodeck-workflow-sync; verify
has "a 3-word description passes" "PASS Branch name: chore/1617e87/aviodeck-workflow-sync"
git -C "$WT" branch -q -m chore/1617e87/abcdefghijklmnopqrstuvw-abcdefghijklm; verify
has "a 2-word, 51-character name fails on length" "FAIL Branch 'chore/1617e87/abcdefghijklmnopqrstuvw-abcdefghijklm' is 51 chars (max 50)"
git -C "$WT" branch -q -m feat/1/the-thing
g checkout -q -b fix/2/other && printf 'x\n' > "$MAIN/other.txt" && g add other.txt && g commit -qm other && g checkout -q main
git -C "$WT" -c core.hooksPath=/dev/null merge -q --no-ff --no-edit fix/2/other
verify
has "a merge of another feature branch fails" "FAIL Branch carries merge commits from branches other than main"
git -C "$WT" reset -q --hard origin/main
g checkout -q -b up main && printf 'y\n' > "$MAIN/up.txt" && g add up.txt && g commit -qm up && g update-ref refs/remotes/origin/main HEAD && g checkout -q main
git -C "$WT" -c core.hooksPath=/dev/null merge -q --no-ff --no-edit origin/main
verify
has "a merge of origin/main passes" "PASS No merge commits from other branches"

echo
echo "verify-worktree: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
