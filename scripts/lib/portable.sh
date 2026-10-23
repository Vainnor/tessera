#!/bin/bash
# Shared shell helpers. Sourced, never executed; must stay side-effect-free on source.
# Bash 3.2 and BSD/GNU safe.

# main_repo_root
#
# Absolute path of the MAIN checkout, correct even from inside a worktree.
# `--show-toplevel` would return the worktree, and `$PWD` returns wherever the
# caller happens to stand.
#
# Returns 1 and prints nothing outside a git repository, so callers fail loudly
# instead of building paths off an empty root.
main_repo_root() {
    local common
    common="$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null)" || return 1
    [[ -n "$common" ]] || return 1
    dirname "$common"
}

# with_timeout SECONDS COMMAND...: COMMAND, killed after SECONDS. timeout(1) is not on
# macOS by default; perl's alarm survives the exec and does the same.
with_timeout() {
    local seconds="$1"
    shift
    if command -v timeout >/dev/null 2>&1; then
        timeout "$seconds" "$@"
    else
        perl -e 'alarm shift; exec @ARGV or exit 127' "$seconds" "$@"
    fi
}
