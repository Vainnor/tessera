#!/bin/bash
# ==============================================================================
# claude_only_change [REF] — is REF's committed diff (default HEAD) guidance only?
#
# For the review and test-marker gates: a branch that changes nothing but agent
# guidance (rules, plans, skills, agents, commands, the root CLAUDE.md and AGENTS.md)
# carries no application code, so those gates have nothing to check on it.
#
# Returns 0 (exempt) only when ALL hold:
#   * origin/main resolves to a commit
#   * `git diff BASE...REF` succeeds and lists at least one path
#   * every path is CLAUDE.md or under .claude/{rules,plans,skills,agents,commands}/,
#     and none defines the review itself or a security control: the code-review,
#     review-before-shipping and ship commands, a reviewer agent, or the security,
#     crypto, privacy, dependencies and git-and-ci rules, whose weakening is a security
#     change. Hooks, settings, the lens map, scripts and anything else under .claude/
#     stay gated, as do docs/ (the threat model and ADRs) and the website.
# Anything else returns 1, so a git error or a missing base ref leaves the gates
# running. Runs in the current directory's repository.
#
# Bash 3.2 and BSD/GNU safe: no arrays, no GNU-only flags.
# Tests: scripts/lib/claude-only-change.test.sh
# ==============================================================================

claude_only_change() {
    local base="origin/main" ref="${1:-HEAD}" files path

    git rev-parse --verify --quiet "${base}^{commit}" >/dev/null 2>&1 || return 1

    # --no-renames lists a move's source too, so moving code into .claude/ is not exempt.
    files="$(git -c core.quotepath=true diff --name-only --no-renames "${base}...${ref}" 2>/dev/null)" || return 1
    [ -n "$files" ] || return 1

    while IFS= read -r path; do
        case "$path" in
            .claude/commands/code-review.md|.claude/commands/review-before-shipping.md) return 1 ;;
            .claude/commands/ship.md|.claude/agents/*review*) return 1 ;;
            .claude/rules/security.md|.claude/rules/crypto.md|.claude/rules/privacy.md) return 1 ;;
            .claude/rules/dependencies.md|.claude/rules/git-and-ci.md) return 1 ;;
            CLAUDE.md) ;;
            .claude/rules/*|.claude/plans/*|.claude/skills/*|.claude/agents/*|.claude/commands/*) ;;
            *) return 1 ;;
        esac
    done <<EOF
$files
EOF

    return 0
}
