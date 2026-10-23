#!/bin/bash
# ==============================================================================
# Tests for scripts/review-scan.sh.
#
#   bash scripts/lib/review-scan.test.sh
#
# Builds a throwaway repo with an origin/main, plants one instance of each pitfall on a
# branch and checks the scan names it; then checks that test code, a reasoned #[allow],
# a pinned Action and a clean branch report nothing. Pure shell; macOS bash 3.2.
# The pitfalls are written into files under $WORK, never into this repository.
# ==============================================================================
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCAN="$ROOT/scripts/review-scan.sh"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/review-scan-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR
export GIT_AUTHOR_NAME=T GIT_AUTHOR_EMAIL=t@e.com GIT_COMMITTER_NAME=T GIT_COMMITTER_EMAIL=t@e.com
export GIT_CONFIG_NOSYSTEM=1 HOME="$WORK"

PASS=0
FAIL=0
ok()  { echo "  PASS: $1"; PASS=$((PASS + 1)); }
bad() { echo "  FAIL: $1"; FAIL=$((FAIL + 1)); }

# The scan lives in the repo it scans: copy it, and the attribution library it sources, into each fixture.
fixture() {
    local repo="$WORK/$1"
    git init -q -b main "$repo"
    mkdir -p "$repo/scripts" "$repo/.claude/hooks/lib"
    cp "$SCAN" "$repo/scripts/review-scan.sh"
    cp "$ROOT/.claude/hooks/lib/attribution.sh" "$repo/.claude/hooks/lib/attribution.sh"
    printf 'fn main() {}\n' >"$repo/base.rs"
    git -C "$repo" add -A && git -C "$repo" -c core.hooksPath=/dev/null commit -q -m base
    git -C "$repo" update-ref refs/remotes/origin/main HEAD
    git -C "$repo" checkout -q -b fix/1/planted-pitfalls
    printf '%s' "$repo"
}

# plant REPO PATH CONTENT: write CONTENT (printf %b) at PATH and commit it.
plant() {
    mkdir -p "$(dirname "$1/$2")"
    printf '%b' "$3" >"$1/$2"
    git -C "$1" add -- "$2" && git -C "$1" -c core.hooksPath=/dev/null commit -q -m "add $2"
}

echo "Each planted pitfall is reported:"
REPO="$(fixture planted)"
plant "$REPO" crates/core/src/a.rs 'pub fn a() { let x = Some(1).unwrap(); dbg!(x); println!("{x}"); }\nfn b() { let _ = c(); }\n#[allow(dead_code)]\nfn d() { unsafe { } }\n'
plant "$REPO" apps/desktop/src/view.tsx 'export const V = () => <div dangerouslySetInnerHTML={{__html: n}} style={{color: "red"}} />;\nconsole.log(n);\n// @ts-ignore\nfetch("https://example.com/x");\n'
plant "$REPO" apps/desktop/src/view.test.ts 'it.only("x", () => {});\n'
plant "$REPO" apps/desktop/src-tauri/capabilities/main.json '{"permissions": ["core:default"]}\n'
plant "$REPO" apps/desktop/src-tauri/tauri.conf.json '{"csp": "script-src '"'"'self'"'"' '"'"'unsafe-inline'"'"'"}\n'
plant "$REPO" .github/workflows/ci.yml 'steps:\n  - uses: actions/checkout@v4\n'
plant "$REPO" crates/core/src/b.rs '<<<<<<< HEAD\nfn x() {}\n=======\n>>>>>>> other\n'
plant "$REPO" .env 'TOKEN=x\n'
printf 'x\n' >"$REPO/later.txt"
git -C "$REPO" add later.txt
git -C "$REPO" -c core.hooksPath=/dev/null commit -q -m "chore: x" -m "Co-Authored-By: Claude <noreply@anthropic.com>"
out="$(cd "$REPO" && bash scripts/review-scan.sh 2>&1)"
rc=$?
if [[ $rc -eq 0 ]]; then ok "exits 0 with hits"; else bad "exit $rc (${out:0:200})"; fi
for want in "unwrap/expect in non-test Rust" "panic!, todo!, unimplemented! or dbg!" "println!/eprintln!" \
    "an error swallowed" "#[allow] without reason" "unsafe in Rust" "raw HTML into the DOM" "console logging" \
    "type or lint suppression" ".only( leaves" "inline style" "network call from the UI" "remote URL in the UI" \
    "a broad or dangerous Tauri permission" "CSP relaxed" "GitHub Action not pinned" "merge conflict marker" \
    "a secret-shaped file" "AI attribution in a commit message"; do
    if [[ "$out" == *"^ $want"* ]]; then ok "reports: $want"; else bad "missing: $want"; fi
done
if [[ "$out" == *"crates/core/src/a.rs:1:"* ]]; then ok "names file:line"; else bad "no file:line (${out:0:200})"; fi
if [[ "$out" =~ scan\ covered\ ([0-9]+)\ file ]] && [[ "${BASH_REMATCH[1]}" -ge 8 ]]; then ok "control line counts the files"; else bad "control line (${out: -80})"; fi

echo "What the rules allow is not reported:"
REPO="$(fixture allowed)"
plant "$REPO" crates/core/src/c.rs '#[allow(clippy::x, reason = "the index is checked above")]\npub fn c() {}\n\n#[cfg(test)]\nmod tests { fn t() { Some(1).unwrap(); println!("t"); } }\n'
plant "$REPO" crates/core/src/sync/tests.rs 'fn t() { Some(1).expect("x"); }\n'
plant "$REPO" crates/core/tests/api.rs 'fn t() { Some(1).unwrap(); }\n'
plant "$REPO" .github/workflows/ci.yml 'steps:\n  - uses: actions/checkout@08eba0b27e820071cde6df949e0beb9ba4906955 # v4.3.0\n  - uses: ./.github/actions/local\n'
plant "$REPO" .env.example 'TOKEN=\n'
plant "$REPO" .claude/rules/x.md 'never use .unwrap() or dangerouslySetInnerHTML\n'
out="$(cd "$REPO" && bash scripts/review-scan.sh 2>&1)"
rc=$?
if [[ $rc -eq 0 && "$out" != *"^ "* ]]; then ok "no hits on test code, reasoned allow, pinned Action, .env.example or guidance"; else bad "unexpected hits (rc=$rc: ${out:0:400})"; fi
if [[ "$out" == *"scan covered 5 file(s)"* ]]; then ok "guidance is not scanned"; else bad "covered count (${out: -80})"; fi

echo "An unscannable state fails:"
out="$(cd "$REPO" && bash scripts/review-scan.sh origin/does-not-exist 2>&1)"
rc=$?
if [[ $rc -eq 1 && "$out" == *"SCAN FAILED"* ]]; then ok "an unknown base exits 1"; else bad "unknown base (rc=$rc: ${out:0:200})"; fi

echo ""
echo "review-scan: $PASS passed, $FAIL failed"
(( FAIL == 0 ))
