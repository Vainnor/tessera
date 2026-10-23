---
description: Review this session for the classes of problems and inefficiencies it hit, and fold the lessons into this repository's Claude files (CLAUDE.md, .claude/rules, agents, commands). Never touches user-level ~/.claude files.
---

Review this session and improve this repository's guidance so the next session avoids the same
classes of problem.

## 1. Find the classes, not the incidents

List what cost time, caused rework or nearly shipped a defect: wrong assumptions, slips, retries,
detours, misread tools or specs, checks that passed without testing anything, work redone after
review. For each, name the **class** it belongs to and why it happened: the mechanism, not the
issue, file, type name or command involved.

- Bad: "`set_markdown` must split edits over `MAX_CHANGE_OPS`."
- Good: "A cap enforced on input also has to hold for what our own code produces; test the local
  path against the cap, not only the remote one."

Drop anything that happened once for a reason that will not recur, and anything existing guidance
already covered but was not followed; the fix there is following it, not repeating it.

## 2. Prefer judgment over rules

For each class, pick the lightest guidance that would have changed the outcome:

1. **Nothing**: a capable engineer gets it right next time without being told.
2. **Sharpen an existing principle**: widen or clarify a rule already there so it covers the
   class.
3. **A new principle**: one that states the reasoning, so it applies to cases nobody has met yet.

Write principles that explain why, so the reader can judge new cases. No checklists of names,
paths or incidents: they go stale and only catch the exact repeat.

## 3. Propose, then apply

Show every proposed change to the user with AskUserQuestion before editing: the file, the exact
text, and the class it addresses. Apply only what is approved.

Write each change in the fewest words that carry it. Search `CLAUDE.md` and `.claude/` for the
topic first and extend or replace what is there; never add a second copy (`CLAUDE.md` →
"Pointers, not copies"). A fact about the codebase (layout, data flow, the encryption design)
belongs in `docs/`; how to work belongs in `CLAUDE.md` and `.claude/`.

## Scope: repository files only

- **Edit only this repository's `CLAUDE.md` and files under `.claude/`** (rules, agents,
  commands). Never create, change or delete anything under `~/.claude/`, including the user-level
  `CLAUDE.md`, memory and user commands. If a lesson belongs there, say so and leave it to the
  user.
- **During a goal, log each approved change, never edit**: append it to the session learnings
  file (`/ticket-loop` → "0. START"); the end of the goal applies the file.
- **Outside a goal, never write to a feature worktree's copy** (it dies with the worktree,
  `ticket-lifecycle.md` → "Worktrees") **or to the primary checkout**, which stays on `main`. Do
  not create branches or commit on your own: list the approved edits and ask the user for an issue
  and a branch. On a yes, apply them through `/start` on a `chore/<n>/<desc>` worktree, as
  `/ticket-loop` → "6. END" does.
