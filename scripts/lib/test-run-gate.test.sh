#!/bin/bash
# ==============================================================================
# Tests for .claude/hooks/test-run-gate.sh: which commands it gates, and the
# SHA-keyed marker it reads.
#
#   bash scripts/lib/test-run-gate.test.sh
#
# The matching cases run the gate from a scratch directory outside any git repo,
# where the marker check cannot pass: a command the gate lets through exits 0
# before reaching it, and one it gates exits 2. The marker cases run it inside a
# scratch repo. A stub gh is first on PATH throughout, so the CI fallback never reaches
# GitHub: it prints the check-runs in $GH_RUNS, and fails when that is unset.
# Self-reports pass/fail, exits non-zero on any failure.
# ==============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GATE="$ROOT/.claude/hooks/test-run-gate.sh"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/test-run-gate-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

mkdir -p "$WORK/bin"
printf '#!/bin/bash\necho "gh $*" >> "%s/gh.log"\n[[ -n "${GH_RUNS:-}" ]] || exit 1\nprintf "%%b" "$GH_RUNS"\n' "$WORK" > "$WORK/bin/gh"
chmod +x "$WORK/bin/gh"
export PATH="$WORK/bin:$PATH"
unset GH_RUNS

PASS=0
FAIL=0
ok()  { echo "  PASS: $1"; PASS=$((PASS + 1)); }
bad() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

payload() {
    python3 -c 'import json,sys; print(json.dumps({"tool_name": "Bash", "tool_input": {"command": sys.argv[1]}}))' "$1"
}

# expect WANT(0 passed through | 2 gated) NAME COMMAND [PATH]
expect() {
    local got
    payload "$3" 2>/dev/null | (cd "$WORK" && GIT_CEILING_DIRECTORIES="$WORK" PATH="${4:-$PATH}" "$BASH_BIN" "$GATE" >/dev/null 2>&1)
    got=$?
    if [[ "$got" == "$1" ]]; then ok "$2"; else bad "$2 (want $1, got $got)"; fi
}

BASH_BIN="$(command -v bash)"
NL=$'\n'
PR="gh pr ""create --base main --title x"

echo "Gated:"
expect 2 "bare"                        "$PR"
expect 2 "chained after cd"            "cd /repo && $PR"
expect 2 "inside bash -c"              "bash -c \"$PR\""
expect 2 "heredoc fed to bash"         "bash <<'EOF'${NL}$PR${NL}EOF"
expect 2 "unparseable line"            "echo \"$PR"
expect 2 "the pr new alias"            "gh pr new --base main --title x"
expect 2 "-R before the subcommand"    "gh pr -R Vainnor/tessera create --title x"
expect 2 "-R before the group"          "gh -R Vainnor/tessera pr create --title x"

echo "A quoted string naming the command reads as a run (a grep would too):"
expect 2 "quoted in a grep"            "grep -n \"$PR\" .claude/hooks/*.sh"

echo "Passed through:"
expect 0 "other command"               "ls -la"
expect 0 "commit message"              "git commit -m \"docs: $PR\""
expect 0 "heredoc written to a file"   "cat > body.md <<'EOF'${NL}$PR${NL}EOF"
expect 0 "heredoc fed to python"       "python3 - <<'PY'${NL}print('$PR')${NL}PY"

echo "Fails closed without python3:"
expect 2 "python3 missing"             "ls -la" "/nonexistent:/usr/bin/nonexistent"

echo "Marker:"
REPO="$WORK/repo"
mkdir -p "$REPO"
git -C "$REPO" init -q
git -C "$REPO" config user.email t@t
git -C "$REPO" config user.name t
printf 'x\n' > "$REPO/app.rs"
git -C "$REPO" add app.rs
git -C "$REPO" commit -qm init
SHA="$(git -C "$REPO" rev-parse HEAD)"

# gate_in_repo: run the gate on a PR command from inside $REPO; sets RC, OUT.
gate_in_repo() {
    OUT="$(payload "$PR" | (cd "$REPO" && GIT_CEILING_DIRECTORIES="$WORK" "$BASH_BIN" "$GATE" 2>&1))"
    RC=$?
}

gate_in_repo
if [[ $RC -eq 2 && "$OUT" == *"pnpm gate"* ]]; then ok "no marker blocks, naming pnpm gate"; else bad "no marker: rc=$RC out=$OUT"; fi
if [[ "$OUT" == *"Nor does CI vouch for it: CI: could not read the check-runs"* ]]; then ok "...and says CI could not be read"; else bad "CI reason: $OUT"; fi

echo "CI green on the exact commit, instead of a marker:"
# ci_runs [NAME=STATUS/CONCLUSION...]: every required job completed/success, with the overrides applied.
ci_runs() {
    local id=1 name line
    . "$ROOT/scripts/lib/ci-checks.sh"
    GH_RUNS=""
    while IFS= read -r name; do
        line="completed\tsuccess"
        for o in "$@"; do [[ "${o%%=*}" == "$name" ]] && line="$(printf '%s' "${o#*=}" | tr '/' '\t')"; done
        [[ "$line" == "-" ]] || GH_RUNS+="$id\t$name\t$line\n"
        id=$((id + 1))
    done <<<"$CI_REQUIRED_CHECKS"
    export GH_RUNS
}
: > "$WORK/gh.log"
ci_runs; gate_in_repo
if [[ $RC -eq 0 ]]; then ok "every CI job green on HEAD passes without a marker"; else bad "green CI: rc=$RC out=$OUT"; fi
if grep -q "^gh api repos/Vainnor/tessera/commits/$SHA/check-runs" "$WORK/gh.log"; then ok "...read for the exact PR head sha"; else bad "gh call: $(cat "$WORK/gh.log")"; fi
ci_runs "Rust format=completed/failure"; gate_in_repo
if [[ $RC -eq 2 && "$OUT" == *"Rust format (completed/failure)"* ]]; then ok "one failed job blocks, naming it"; else bad "failed job: rc=$RC out=$OUT"; fi
ci_runs "Clippy and tests (windows-latest)=in_progress/"; gate_in_repo
if [[ $RC -eq 2 && "$OUT" == *"windows-latest) (in_progress/)"* ]]; then ok "a job still running blocks"; else bad "pending job: rc=$RC out=$OUT"; fi
ci_runs "Website Lighthouse budget=completed/skipped"; gate_in_repo
if [[ $RC -eq 2 ]]; then ok "a skipped job blocks"; else bad "skipped job: rc=$RC"; fi
ci_runs "Secret scan (full history)=-"; gate_in_repo
if [[ $RC -eq 2 && "$OUT" == *"Secret scan (full history) (no run)"* ]]; then ok "a missing job blocks"; else bad "missing job: rc=$RC out=$OUT"; fi
ci_runs "Rust format=completed/failure"; GH_RUNS+="999\tRust format\tcompleted\tsuccess\n"; gate_in_repo
if [[ $RC -eq 0 ]]; then ok "a green re-run supersedes the failed run"; else bad "re-run: rc=$RC out=$OUT"; fi
ci_runs; GH_RUNS+="999\tRust format\tcompleted\tfailure\n"; gate_in_repo
if [[ $RC -eq 2 ]]; then ok "a failed re-run supersedes the green run"; else bad "failed re-run: rc=$RC"; fi
unset GH_RUNS

stamp_marker() { python3 -I "$ROOT/scripts/lib/marker_manifest.py" stamp "$@"; }
MK="$REPO/.claude/markers"
mkdir -p "$MK/tests"
date +%s > "$MK/tests/$SHA"
gate_in_repo
if [[ $RC -eq 2 && "$OUT" == *"no matching line in .claude/markers/manifest"* ]]; then ok "a hand-written marker is refused"; else bad "hand-written marker: rc=$RC out=$OUT"; fi
printf 'tests %s %s\n' "$SHA" "$(printf 'x' | sha256sum | cut -d' ' -f1)" > "$MK/manifest"
gate_in_repo
if [[ $RC -eq 2 ]]; then ok "a manifest line whose hash is not the marker's is ignored"; else bad "wrong-hash line passed: rc=$RC"; fi
printf 'tests %s %s\n' "$SHA" "$(sha256sum < "$MK/tests/$SHA" | cut -d' ' -f1)" > "$WORK/forged-manifest"
ln -sf "$WORK/forged-manifest" "$MK/manifest"
gate_in_repo
if [[ $RC -eq 2 ]]; then ok "a symlinked manifest is not a manifest"; else bad "symlinked manifest passed: rc=$RC"; fi
rm -f "$MK/manifest" "$MK/tests/$SHA"
printf 'review-shipping %s %s\n' "$SHA" "$(sha256sum < "$WORK/forged-manifest" | cut -d' ' -f1)" > "$MK/manifest"
cp "$WORK/forged-manifest" "$MK/tests/$SHA"
gate_in_repo
if [[ $RC -eq 2 ]]; then ok "a line for another kind does not vouch for a tests marker"; else bad "other-kind line passed: rc=$RC"; fi
rm -f "$MK/manifest" "$MK/tests/$SHA"
stamp_marker "$REPO" tests "$SHA"
ln -s "$MK/tests/$SHA" "$MK/tests/link" && mv "$MK/tests/$SHA" "$WORK/real" && ln -s "$WORK/real" "$MK/tests/$SHA"
gate_in_repo
if [[ $RC -eq 2 ]]; then ok "a symlinked marker is not a marker"; else bad "symlinked marker passed: rc=$RC"; fi
rm -f "$MK/tests/$SHA" "$MK/tests/link" "$MK/manifest"
stamp_marker "$REPO" tests "$SHA"
gate_in_repo
if [[ $RC -eq 0 ]]; then ok "a marker for HEAD with its manifest line passes"; else bad "marker for HEAD: rc=$RC out=$OUT"; fi

git -C "$REPO" commit -q --allow-empty -m moved
gate_in_repo
if [[ $RC -eq 2 ]]; then ok "a marker for an earlier commit blocks"; else bad "stale-SHA marker: rc=$RC out=$OUT"; fi

date +%s > "$REPO/.claude/.tests-passed"
gate_in_repo
if [[ $RC -eq 2 ]]; then ok "the legacy .tests-passed timestamp is not evidence"; else bad "legacy marker passed: rc=$RC out=$OUT"; fi

# The marker must describe the commit the PR opens on: a `cd DIR &&` checkout, or a --head branch.
stamp_marker "$REPO" tests "$(git -C "$REPO" rev-parse HEAD)"

# An unresolved PR head is refused, never checked against whatever HEAD has a marker for.
(cd "$REPO" && . "$ROOT/scripts/check-marker.sh" && check_marker_for_head tests Tests "pnpm gate" "$(git rev-parse HEAD)" >/dev/null)
if [[ $? -eq 0 ]]; then ok "positive control: check_marker_for_head passes HEAD's marker"; else bad "check_marker_for_head HEAD"; fi
(cd "$REPO" && . "$ROOT/scripts/check-marker.sh" && check_marker_for_head tests Tests "pnpm gate" "" >/dev/null)
if [[ $? -eq 1 ]]; then ok "an empty PR head does not fall back to HEAD's marker"; else bad "empty PR head fell back to HEAD's marker"; fi
UNTESTED="$WORK/untested"
# A clone has no user.* config, and a CI runner has no global identity: name the author here.
git clone -q "$REPO" "$UNTESTED" && git -C "$UNTESTED" -c user.email=t@t -c user.name=t commit -q --allow-empty -m untested
if [[ "$(git -C "$UNTESTED" rev-parse HEAD^ 2>/dev/null)" == "$(git -C "$REPO" rev-parse HEAD)" ]]; then ok "the untested checkout holds one commit on top of the tested one"; else bad "the untested checkout's commit was not made"; fi
git -C "$REPO" branch -q feat/2/untested && git -C "$REPO" checkout -q feat/2/untested \
    && git -C "$REPO" commit -q --allow-empty -m untested && git -C "$REPO" checkout -q -
gate_cmd() { # gate_cmd CWD COMMAND; sets RC
    payload "$2" | (cd "$1" && GIT_CEILING_DIRECTORIES="$WORK" "$BASH_BIN" "$GATE" >/dev/null 2>&1)
    RC=$?
}
gate_cmd "$REPO" "$PR"
if [[ $RC -eq 0 ]]; then ok "positive control: the tested checkout passes"; else bad "tested checkout: rc=$RC"; fi
gate_cmd "$REPO" "cd $UNTESTED && $PR"
if [[ $RC -eq 2 ]]; then ok "cd into an untested checkout is checked there"; else bad "cd into untested checkout: rc=$RC"; fi
gate_cmd "$UNTESTED" "cd $REPO && $PR"
if [[ $RC -eq 0 ]]; then ok "cd into the tested checkout is checked there"; else bad "cd into tested checkout: rc=$RC"; fi
gate_cmd "$REPO" "$PR --head feat/2/untested"
if [[ $RC -eq 2 ]]; then ok "--head an untested branch blocks"; else bad "--head untested: rc=$RC"; fi

# expect_from CWD WANT NAME COMMAND
expect_from() {
    gate_cmd "$1" "$4"
    if [[ $RC -eq $2 ]]; then ok "$3"; else bad "$3 (want $2, got $RC)"; fi
}
echo "Other repositories and API PRs, from a tested checkout:"
git -C "$REPO" remote add origin git@github.com:Vainnor/tessera.git
expect_from "$REPO" 0 "positive control: --repo this checkout's origin"  "$PR --repo Vainnor/tessera"
expect_from "$REPO" 2 "--repo another repository"                         "$PR --repo someone/fork"
expect_from "$REPO" 2 "--head on a fork, naming the tested branch"        "$PR --head someone:$(git -C "$REPO" branch --show-current)"
expect_from "$REPO" 0 "--head owner:branch on this checkout's origin"     "$PR --head Vainnor:$(git -C "$REPO" branch --show-current)"
expect_from "$REPO" 2 "gh api POST to pulls"                              "gh api repos/Vainnor/tessera/pulls -f head=x -f base=main"
expect_from "$REPO" 2 "gh api -iX POST to pulls"                          "gh api -iX POST repos/Vainnor/tessera/pulls --input body.json"
expect_from "$REPO" 2 "GraphQL createPullRequest"                         "gh api graphql -f query='mutation { createPullRequest(input: {}) { clientMutationId } }'"

echo "A line that moves directory before the PR, from a tested cwd:"
expect_from "$REPO" 2 "cd DIR; gh pr create"                              "cd $UNTESTED; $PR"
expect_from "$REPO" 2 "cd DIR && git push && gh pr create"                "cd $UNTESTED && git push -u origin x && $PR"
expect_from "$REPO" 2 "pushd DIR && gh pr create"                         "pushd $UNTESTED && $PR"

echo "Guidance-only exemption, judged on the PR head:"
git -C "$REPO" update-ref refs/remotes/origin/main "$(git -C "$REPO" rev-list --max-parents=0 HEAD)"
git -C "$REPO" checkout -q -b chore/3/guidance origin/main && mkdir -p "$REPO/.claude/rules" \
    && printf 'x\n' > "$REPO/.claude/rules/x.md" && git -C "$REPO" add .claude/rules/x.md && git -C "$REPO" commit -qm rules
expect_from "$REPO" 0 "a rules-only branch skips the gate"                "$PR"
expect_from "$REPO" 2 "a rules-only HEAD does not exempt --head another branch" "$PR --head feat/2/untested"
git -C "$REPO" checkout -q -

# A copy of the hook tree without scripts/check-marker.sh must refuse, not pass.
COPY="$WORK/copy"
mkdir -p "$COPY/.claude/hooks" "$COPY/scripts/lib"
cp -R "$ROOT/.claude/hooks/lib" "$COPY/.claude/hooks/lib"
cp "$GATE" "$COPY/.claude/hooks/test-run-gate.sh"
cp "$ROOT/scripts/lib/claude-only-change.sh" "$COPY/scripts/lib/claude-only-change.sh"
OUT="$(payload "$PR" | (cd "$REPO" && "$BASH_BIN" "$COPY/.claude/hooks/test-run-gate.sh" 2>&1))"
RC=$?
if [[ $RC -eq 2 && "$OUT" == *"cannot load check_marker_for_head"* ]]; then ok "a missing check-marker.sh is gated"; else bad "missing check-marker.sh: rc=$RC out=$OUT"; fi

echo "A refused line says why:"
out="$(python3 -c 'import json,sys; print(json.dumps({"tool_name": "Bash", "tool_input": {"command": sys.argv[1]}}))' "gh \$'pr' create" \
    | (cd "$WORK" && bash "$GATE" 2>&1))"
rc=$?
if [[ $rc -eq 2 && "$out" == *"ANSI-C"* ]]; then ok "the parser's reason is in the message"; else bad "reason missing (rc=$rc: ${out:0:200})"; fi

echo ""
echo "test-run-gate: $PASS passed, $FAIL failed"
(( FAIL == 0 ))
