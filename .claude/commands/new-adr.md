---
description: Draft a new architecture decision record in docs/adr from the template, with options and consequences, for the user to accept.
argument-hint: <short title of the decision>
---

Draft an ADR for: $ARGUMENTS

1. Read `docs/adr/README.md`, `docs/adr/template.md` and every existing ADR. If an accepted ADR
   already decides this, say so and stop, or draft a superseding record if the user wants one.
2. Take the next number. Create `docs/adr/NNNN-kebab-title.md` from the template with
   `Status: Proposed` and today's date.
3. Context: the forcing requirement, with links to the code or docs involved.
4. Options: at least two real ones, each with costs and benefits from primary sources (docs,
   specs, advisories), not memory. For a dependency, run `dependency-researcher`; for anything
   cryptographic, follow `crypto.md` and run `crypto-reviewer` on the draft.
5. Decision: your recommendation, stated as a rule, clearly marked as the recommendation.
6. Consequences, including what would make us revisit it.
7. Add the row to the table in `docs/adr/README.md`, and update any doc `docs.md` says this
   decision touches.

Present the draft and ask the user (with `AskUserQuestion`) to accept, change or reject it.
Only an accepted ADR changes status to `Accepted`. Writing follows `writing.md`.
