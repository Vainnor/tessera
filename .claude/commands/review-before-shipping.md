---
description: Review the committed HEAD before pushing or opening a PR — full gate, security and privacy checklist, crypto check, docs sync, scope check.
---

Review the commit you are about to ship. Report findings; fix only what this branch introduced.

## 1. Pin the commit

`git rev-parse HEAD` is the reviewed SHA. The worktree must be clean (`git status --porcelain`,
including `??` lines). If you change anything below, commit and start again from this step.

## 2. Scope

`git diff origin/main...HEAD --stat` (before the first push, diff against the root commit or the
base the user names). Name the requirement each changed file serves. Anything no requirement asks
for comes back out, or goes to the user.

## 3. Gate

`pnpm gate`, output to a scratch file. Every step must say PASS; read every `test result:` line
and vitest's `Tests` line, and name any failing test or MISSING tool.

## 4. Checklists

- `security.md` → "Change checklist", item by item, with the file:line that satisfies each one
  or "not applicable" with why.
- `privacy.md`: does any line send, store or log something new? Then `privacy.astro` changes too.
- `docs.md`: does each row the diff triggers have its doc updated in this diff?
- Website claims: does any feature status in `apps/web/src/data/site.ts` overstate what ships?
- Rust: no new `unwrap`/`expect`/`unsafe` outside the `rust.md` exceptions
  (`git diff origin/main...HEAD -- '*.rs' | grep -nE '^\+.*(unwrap\(|expect\(|unsafe )'`).

## 5. Fresh eyes

- A change touching `apps/desktop/src-tauri/`, the CSP, vault paths, imports, the server or a
  workflow: dispatch the `security-reviewer` agent with the SHA and wait for it.
- A change touching `crypto`, `sync`, keys, recovery or server-visible data: dispatch
  `crypto-reviewer` too.
- A new or upgraded dependency: dispatch `dependency-researcher`.

## 6. Report

Gate results by step (and what did not run, such as other-OS CI jobs, the GUI and the production
Tauri build), each finding as problem / why / effect / risk, and whether HEAD is ready to push.
Pushing still needs the user's go-ahead.
