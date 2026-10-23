# Issue guidelines

Work on Tessera is tracked as issues on this repository and on the
[Tessera Kanban](https://github.com/users/Vainnor/projects/1) board. An issue is a small
specification that someone who was not in the conversation can pick up. The full rules the
maintainers and their tools follow are in
[`.claude/rules/github-issues.md`](../.claude/rules/github-issues.md).

## Before you file

- **Security problems are never public issues.** Report them privately, as
  [`SECURITY.md`](../SECURITY.md) describes.
- **Use made-up examples only.** No note content, real file paths, usernames or log lines from
  your own vault. Issues are public and permanent.
- **Search first**, including closed issues: `is:issue <the file, command or feature>`. A closed
  duplicate means someone already decided.

## Writing one

Pick the form that fits: bug, feature or chore. A good issue has:

- **A title that states the problem and its consequence**: "Vault listing skips nested folders,
  so their notes never appear in the sidebar", not "Fix vault".
- **What happens**, with the steps or output that show it.
- **What should happen**, and the trade-off if there is one.
- **Acceptance criteria**: checkable outcomes, one per line. These are what the work is reviewed
  against.

## What happens next

New issues start in **Triaging**. A maintainer accepts, labels and prioritizes them, and moves
them to **To Do**. Work moves through **In build** and **Post build** (a pull request is open) to
**Done**. Labels:

- `type:` bug, feature, chore, documentation or security;
- `area:` core, crypto, server, desktop, web, theme, ui or ci;
- `priority:` critical to trivial, set by a maintainer.

Keep comments to what changes the work: a sharper reproduction, a correction, a decision.
Progress updates and "+1" comments make the thread harder to read; use a reaction instead.
