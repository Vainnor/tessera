#!/bin/bash
set -uo pipefail

# ==============================================================================
# Provision a Tessera worktree
# ==============================================================================
# The counterpart to scripts/teardown-worktree.sh. A fresh worktree has no
# node_modules and no apps/desktop/dist, so `pnpm lint` dies on a missing binary and
# clippy fails to compile the desktop crate, which embeds dist/. Both read as branch
# regressions rather than as a half-built checkout (.claude/rules/ticket-lifecycle.md
# → "Worktrees").
#
# Idempotent. Re-run it on an existing worktree to repair drift.
#
# Usage:
#   bash scripts/setup-worktree.sh feat-12-vault-rename-race     # .claude/worktrees/<NAME>
#   bash scripts/setup-worktree.sh /abs/path/to/worktree         # any worktree, by path
#   bash scripts/setup-worktree.sh                               # the checkout you are in
#
# Tests: scripts/lib/setup-worktree.test.sh
# ==============================================================================

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

fatal() { echo -e "${RED}FATAL:${NC} $1" >&2; exit 1; }
step()  { echo -e "\n${CYAN}==>${NC} $1"; }
ok()    { echo -e "  ${GREEN}ok${NC} $1"; }
warn()  { echo -e "  ${YELLOW}warn${NC} $1"; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/portable.sh
. "$SCRIPT_DIR/lib/portable.sh"

TARGET="${1:-.}"
# A bare name is EnterWorktree's: .claude/worktrees/<NAME> in the main checkout, `-` (or `+` in older
# worktrees) standing for the branch's `/`. `[[ =~ ]]`, not `echo | grep`, which matches per line.
if [[ "$TARGET" =~ ^[A-Za-z0-9_+-]+$ ]] && MAIN_ROOT="$(main_repo_root)" && [[ -d "$MAIN_ROOT/.claude/worktrees/$TARGET" ]]; then
    TARGET="$MAIN_ROOT/.claude/worktrees/$TARGET"
fi
[[ -d "$TARGET" ]] || fatal "No worktree at ${TARGET}
Create it first — prefer the EnterWorktree tool over raw git; see .claude/commands/start.md."
WORKTREE_DIR="$(git -C "$TARGET" rev-parse --show-toplevel 2>/dev/null)" || fatal "${TARGET} is not inside a git checkout."
cd "$WORKTREE_DIR" || fatal "Cannot enter ${WORKTREE_DIR}"
PROJECT_ROOT="$(main_repo_root)" || fatal "Not inside a git repository."

# Linked, not the main checkout: the main checkout is never provisioned from itself.
if [[ "$(git rev-parse --absolute-git-dir)" == "$(git rev-parse --path-format=absolute --git-common-dir)" ]]; then
    fatal "${WORKTREE_DIR} is the main checkout, not a linked worktree."
fi

# -- Mirror everything to a log, so a failed run is never silent --
# .claude/ is gitignored apart from the shared workflow files.
mkdir -p "$WORKTREE_DIR/.claude"
LOG="$WORKTREE_DIR/.claude/worktree-setup.log"
exec > >(tee -a "$LOG") 2>&1

echo -e "${YELLOW}Provisioning worktree:${NC} ${CYAN}${WORKTREE_DIR}${NC}"
echo "  branch: $(git symbolic-ref --quiet --short HEAD 2>/dev/null || echo '(detached)')"
echo "  main:   ${PROJECT_ROOT}"

# -- node_modules --
#
# Installed here, never linked: a symlinked node_modules runs this branch's tests
# against the packages another checkout's lockfile names. --prefer-offline reuses
# the pnpm store, so a worktree install takes seconds when nothing changed.
step "node_modules"
command -v pnpm >/dev/null 2>&1 || fatal "pnpm not found (corepack enable, or brew install pnpm)."
if [[ -L node_modules ]]; then
    warn "node_modules was a symlink — replacing it with this worktree's own install"
    rm node_modules
fi
pnpm install --frozen-lockfile --prefer-offline || fatal "pnpm install --frozen-lockfile failed — output above"
ok "pnpm install --frozen-lockfile done"

# -- Cargo target dir --
#
# Each worktree builds into its own target/. A single target dir shared across
# worktrees is unsafe: cargo hashes path packages workspace-relative and judges their
# freshness by mtime, so one worktree can run another's test binary and pass on code it
# does not have. To skip the cold build, a missing target/ is seeded with a
# copy-on-write clone of the primary checkout's (APFS clonefile on macOS, reflink on
# Linux): seconds, and no extra disk until either side writes. Never into an existing
# target/, never into the primary itself.
step "cargo target dir"
if [[ "$WORKTREE_DIR" == "$PROJECT_ROOT" ]]; then
    ok "the primary checkout keeps its own target/ (not seeded)"
elif [[ -e "$WORKTREE_DIR/target" ]]; then
    ok "target/ already present (not seeded)"
elif [[ ! -d "$PROJECT_ROOT/target" ]]; then
    warn "the primary checkout has no target/ to seed from: the first build here is cold"
else
    case "$(uname -s)" in
        Darwin) CLONE=(cp -c -R) ;;
        *) CLONE=(cp -R --reflink=always) ;;
    esac
    SEED_START="$(date +%s)"
    if "${CLONE[@]}" "$PROJECT_ROOT/target" "$WORKTREE_DIR/target"; then
        # Load-bearing, and it must run right after the clone: every tracked file is
        # touched so each workspace source is newer than any seeded artifact. Without
        # it cargo treats a crate whose source differs from the primary's as fresh and
        # runs the primary's stale build of it. Dependencies, keyed on their version,
        # stay reused. -c because ls-files lists index entries that may be deleted in
        # the worktree, which a plain touch recreates empty; -h touches a symlink itself.
        if git -C "$WORKTREE_DIR" ls-files -z | xargs -0 touch -c -h; then
            ok "seeded target/ from ${PROJECT_ROOT}/target in $(( $(date +%s) - SEED_START ))s (${CLONE[*]}); sources touched so workspace crates rebuild"
        else
            rm -rf "$WORKTREE_DIR/target"
            warn "could not touch the tracked files after seeding, so the seed was removed: the first build here is cold"
        fi
    else
        rm -rf "$WORKTREE_DIR/target"
        warn "could not clone ${PROJECT_ROOT}/target (${CLONE[*]} failed: no copy-on-write on this filesystem?); removed the partial copy, the first build here is cold"
    fi
fi
# -- Frontend build --
#
# The desktop crate embeds apps/desktop/dist, so clippy and cargo test fail to compile
# it until the JS build has run once (.claude/rules/rust.md → "Gate").
step "pnpm build"
pnpm build >/dev/null || fatal "pnpm build failed — run it in ${WORKTREE_DIR} to see why"
ok "pnpm build done (apps/desktop/dist present)"

# -- Health check --
step "health check"
if bash "$SCRIPT_DIR/verify-worktree.sh"; then
    echo -e "\n${GREEN}Worktree ${CYAN}${WORKTREE_DIR}${GREEN} is ready.${NC}"
    exit 0
fi

echo -e "\n${RED}Provisioning finished but the health check failed.${NC}"
echo -e "${YELLOW}Read the log:${NC} ${LOG}"
exit 1
