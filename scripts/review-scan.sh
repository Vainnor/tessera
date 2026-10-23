#!/bin/bash
# Deterministic pitfall scan for /code-review and /review-before-shipping.
#
# A script so it can run from a worktree session, where the harness refuses git fed to bash
# through a heredoc or $(...), and so the file list is word-split by bash rather than by the
# user's shell.
#
# Each hit is a lead, not proof: confirm it against the diff. The checks are the mistakes the
# rules name that compile, pass the tests and still break something:
#   * Rust outside tests: unwrap/expect, panic!/todo!/unimplemented!/dbg!, println!/eprintln!,
#     `let _ =`, `.ok();`, an #[allow] with no reason, `unsafe` (rust.md, privacy.md)
#   * TypeScript: raw HTML into the DOM, console logging, type or lint suppressions, `.only(`,
#     inline styles, network calls and remote URLs (security.md, privacy.md, frontend.md)
#   * Config: `core:default` or a shell, fs, http, opener or updater plugin in a capability,
#     'unsafe-inline' or 'unsafe-eval' in a CSP, a GitHub Action not pinned by commit SHA
#   * Everywhere: merge conflict markers, an added .env or key file, AI attribution in the
#     branch's commit messages (git-and-ci.md)
# Rust test code is skipped: a `tests.rs` file, anything under tests/, and every line from a
# file's first `#[cfg(test)]` on. Only source and config files are read, so this script, its
# test (scripts/lib/review-scan.test.sh) and the guidance under .claude/ never report themselves.
#
# Files are the union of committed-vs-BASE, staged, unstaged and untracked, so a hit may be
# pre-existing in a file the branch touched. Prints `file:line: text` then `^ message` per
# check. Exit 0 with hits printed; exit 1 when the scan itself could not run (a failed git call,
# an unknown base), so a partial scan never passes. The last line, `scan covered N file(s)`, is
# the positive control: 0 on a branch with changes means the file list was empty, not clean.
#
# Usage: bash scripts/review-scan.sh [base-ref]    (from the checkout under review; default origin/main)
# Tests: scripts/lib/review-scan.test.sh
set -uo pipefail

fail() { echo "SCAN FAILED: $*" >&2; exit 1; }

cd "$(git rev-parse --show-toplevel 2>/dev/null)" || fail "not inside a git checkout"
base="${1:-origin/main}"
mb="$(git merge-base "$base" HEAD 2>/dev/null)" || fail "no merge-base between $base and HEAD"

tmp="$(mktemp -d)" || fail "mktemp"
trap 'rm -rf "$tmp"' EXIT

{
    git diff --name-only --diff-filter=ACMR "$mb" HEAD || exit 1
    git diff --name-only --diff-filter=ACMR --cached || exit 1
    git diff --name-only --diff-filter=ACMR || exit 1
    git ls-files --others --exclude-standard || exit 1
} >"$tmp/all" || fail "git could not list the changed files"
sort -u "$tmp/all" | while IFS= read -r path; do [[ -f "$path" ]] && printf '%s\n' "$path"; done >"$tmp/files"

# Only source and config are read (see the header).
grep -E '\.(rs|ts|tsx|astro|json|yml|yaml|toml)$|(^|/)\.env|\.(key|pem|p12)$' "$tmp/files" \
    | grep -vE '^\.claude/|^scripts/lib/|(^|/)pnpm-lock\.yaml$|(^|/)Cargo\.lock$' >"$tmp/scanned" || true
covered="$(wc -l <"$tmp/scanned" | tr -d ' ')"

# check MESSAGE PATH-REGEX LINE-REGEX [EXCLUDE-REGEX]: every line matching LINE-REGEX (and not
# EXCLUDE-REGEX) in every scanned file matching PATH-REGEX. Rust test code is cut first (see the
# header); cutting from the top keeps grep's line numbers right.
check() {
    local message="$1" paths="$2" pattern="$3" exclude="${4:-}" path found hits=""
    while IFS= read -r path; do
        [[ "$path" =~ $paths ]] || continue
        if [[ "$path" == *.rs ]]; then
            [[ "$path" == */tests.rs || "$path" == tests/* || "$path" == */tests/* ]] && continue
            found="$(awk '/#\[cfg\(test\)\]/ { exit } { print }' "$path" | grep -nE -- "$pattern")"
        else
            found="$(grep -nE -- "$pattern" "$path")"
        fi
        if [[ -n "$found" && -n "$exclude" ]]; then
            found="$(printf '%s\n' "$found" | grep -vE -- "$exclude")"
        fi
        [[ -n "$found" ]] && hits+="$(printf '%s\n' "$found" | sed "s|^|$path:|")"$'\n'
    done <"$tmp/scanned"
    if [[ -n "$hits" ]]; then
        printf '%s' "$hits"
        echo "^ $message"
        echo
    fi
}

RUST='\.rs$'
TS='\.(ts|tsx|astro)$'
UI='^(apps/desktop/src|apps/web/src|packages)/.*\.(ts|tsx|astro)$'

check "unwrap/expect in non-test Rust: propagate the error, or allow with a reason (rust.md)" \
    "$RUST" '\.(unwrap|expect)\('
check "panic!, todo!, unimplemented! or dbg! in non-test Rust (rust.md)" \
    "$RUST" '(panic|todo|unimplemented|dbg)!\('
check "println!/eprintln! in non-test Rust: log through tracing, and never note content or paths (privacy.md)" \
    "$RUST" '(e?print(ln)?)!\('
check "an error swallowed with let _ = or .ok(); (rust.md → Errors)" \
    "$RUST" '(let _ = |\.ok\(\);)'
check "#[allow] without reason = \"...\" (rust.md)" \
    "$RUST" '#\[allow\(' 'reason[[:space:]]*='
check "unsafe in Rust: every crate forbids it; it needs an ADR (rust.md)" \
    "$RUST" '(^|[^a-z_])unsafe[[:space:]]*(\{|fn|impl)'
check "raw HTML into the DOM: the webview renders untrusted notes (security.md)" \
    "$TS" '(dangerouslySetInnerHTML|\.innerHTML[[:space:]]*=|\.outerHTML[[:space:]]*=|insertAdjacentHTML|document\.write)'
check "console logging in shipped code (privacy.md: logs never hold user data)" \
    "$UI" 'console\.(log|debug|info)\('
check "type or lint suppression: fix the cause" \
    "$TS" '(@ts-ignore|@ts-nocheck|@ts-expect-error|eslint-disable)'
check ".only( leaves the rest of a suite unrun" \
    "$TS" '(describe|it|test)\.only\('
check "inline style: use the theme's tokens and classes (frontend.md, theming.md)" \
    "$UI" 'style=\{\{'
check "network call from the UI: no network by default, no third-party requests (privacy.md)" \
    "$UI" '(fetch\(|XMLHttpRequest|new WebSocket|navigator\.sendBeacon)'
check "remote URL in the UI or a package: everything ships in the bundle (privacy.md)" \
    "^(apps/desktop/src|packages)/" 'https?://[a-zA-Z0-9]'
check "a broad or dangerous Tauri permission: grant commands by name (security.md, tauri.md)" \
    '^apps/desktop/src-tauri/capabilities/.*\.json$' '"(core:default|shell:|fs:|http:|opener:|updater:)'
check "CSP relaxed with unsafe-inline or unsafe-eval (security.md)" \
    '(tauri\.conf\.json|\.astro|headers|nginx|Caddyfile|\.toml)$' "unsafe-(inline|eval)"
check "GitHub Action not pinned by commit SHA (security.md → Secrets and supply chain)" \
    '^\.github/workflows/.*\.ya?ml$' 'uses:[[:space:]]*[^.[:space:]][^@[:space:]]*@([^0-9a-f[:space:]]|[0-9a-f]{0,39}([^0-9a-f]|$))'
check "merge conflict marker" \
    '.' '^(<<<<<<<|>>>>>>>)( |$)|^=======$'

secrets="$(grep -E '(^|/)\.env($|\.)|\.(key|pem|p12)$' "$tmp/scanned" | grep -vE '\.env\.example$' || true)"
if [[ -n "$secrets" ]]; then
    printf '%s\n' "$secrets"
    echo "^ a secret-shaped file is in the change: never commit keys or .env files (security.md)"
    echo
fi

# AI attribution in the branch's commit messages (git-and-ci.md; .githooks/commit-msg enforces it too).
LIB="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/../.claude/hooks/lib/attribution.sh"
if [[ -f "$LIB" ]]; then
    git log --format='%an <%ae>%n%cn <%ce>%n%B' "$mb..HEAD" >"$tmp/commits" || fail "git log $mb..HEAD"
    # shellcheck source=../.claude/hooks/lib/attribution.sh
    . "$LIB"
    attribution_hits "$tmp/commits" >"$tmp/attribution"
    case $? in
        0) sed 's/^[^:]*:/commit message:/' "$tmp/attribution"; echo "^ AI attribution in a commit message: reword it (git-and-ci.md)"; echo ;;
        1) ;;
        *) fail "the attribution check could not read the commit messages" ;;
    esac
fi

echo "scan covered ${covered} file(s)"
