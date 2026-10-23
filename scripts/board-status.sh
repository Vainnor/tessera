#!/usr/bin/env bash
# Move a Vainnor/tessera issue's card to a Status column on the "Tessera Kanban" board
# (user project 1 under Vainnor).
#
#   bash scripts/board-status.sh <issue-number> "<Status column>" [--dry-run]
#   e.g. bash scripts/board-status.sh 47 "In build"
#
# Columns: Triaging · To Do · In build · Post build · Blocked · Done
# (.claude/rules/github-issues.md → "Board reference"). The workflow moves a card only to
# Triaging (a filed issue), In build (/start) and Post build (/ship); the rest are the maintainer's.
#
# Every id (project, Status field, option, item) is read at run time, so a renamed column or a
# recreated field fails loudly instead of moving the card somewhere else. The item is read from the
# ISSUE's own projectItems, never by listing the board: `gh project item-list` truncates at --limit
# without saying so, and it pulls every card's body, which trips the Projects secondary rate limit.
# --dry-run resolves every id and writes nothing.
#
# The one GraphQL mutation the workflow makes happens here, inside the script; the hooks refuse a
# direct `gh api graphql` mutation (.claude/hooks/master-protection-gate.sh).
set -euo pipefail

OWNER="Vainnor"
REPO="tessera"
NUMBER=1
COLUMNS="Triaging, To Do, In build, Post build, Blocked, Done"

usage() { echo "usage: bash scripts/board-status.sh <issue-number> \"<Status column>\" [--dry-run]" >&2; exit 2; }
issue="${1:-}"
column="${2:-}"
dry_run="${3:-}"
[[ "$issue" =~ ^[0-9]+$ && -n "$column" ]] || usage
[[ -z "$dry_run" || "$dry_run" == "--dry-run" ]] || usage
case ", $COLUMNS, " in
    *", $column, "*) ;;
    *) echo "no Status column named \"$column\"; columns: $COLUMNS" >&2; exit 1 ;;
esac
command -v gh >/dev/null 2>&1 || { echo "board-status: needs gh" >&2; exit 2; }

# One query: the issue's project items, each with its project's owner, number, id and Status options.
# The issue number goes in as a GraphQL variable, never into the query text.
query='query($owner: String!, $repo: String!, $issue: Int!) {
  repository(owner: $owner, name: $repo) {
    issue(number: $issue) {
      projectItems(first: 100) {
        nodes {
          id
          project {
            id
            number
            owner { ... on User { login } ... on Organization { login } }
            field(name: "Status") { ... on ProjectV2SingleSelectField { id options { id name } } }
          }
        }
      }
    }
  }
}'
found="$(gh api graphql -f query="$query" -F owner="$OWNER" -F repo="$REPO" -F issue="$issue" \
    --jq '.data.repository.issue.projectItems.nodes[]
          | select(.project.number == '"$NUMBER"' and .project.owner.login == "'"$OWNER"'")
          | [.id, .project.id, (.project.field.id // ""),
             ((.project.field.options // [])[] | select(.name == "'"${column//\"/}"'") | .id)] | @tsv')" \
    || { echo "board-status: could not read issue #$issue's project items from GitHub" >&2; exit 1; }

if [[ -z "$found" ]]; then
    echo "issue #$issue is not on project $NUMBER (or no such issue). Add it, then re-run this as a separate call:" >&2
    echo "  gh project item-add $NUMBER --owner $OWNER --url https://github.com/$OWNER/$REPO/issues/$issue" >&2
    exit 1
fi
IFS=$'\t' read -r item project field option <<<"$found"
[[ -n "$field" ]] || { echo "project $NUMBER has no Status field" >&2; exit 1; }
[[ -n "$option" ]] || { echo "project $NUMBER's Status field has no \"$column\" option; columns: $COLUMNS" >&2; exit 1; }

if [[ "$dry_run" == "--dry-run" ]]; then
    echo "would move #$issue (item $item) to \"$column\" (project $project, field $field, option $option)"
    exit 0
fi
mutation='mutation($project: ID!, $item: ID!, $field: ID!, $option: String!) {
  updateProjectV2ItemFieldValue(input: {projectId: $project, itemId: $item, fieldId: $field,
                                        value: {singleSelectOptionId: $option}}) { projectV2Item { id } }
}'
gh api graphql -f query="$mutation" -f project="$project" -f item="$item" -f field="$field" -f option="$option" >/dev/null
echo "moved #$issue → $column"
