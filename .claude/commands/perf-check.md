---
description: Read-only performance check of the branch against the budgets in docs/performance.md — idle work, polling, main-thread work, rescans, bundle size.
---

Assess the current branch against `.claude/rules/performance.md` and `docs/performance.md`.
Read-only: report, do not fix. Every number is measured or labeled an estimate.

1. `git diff origin/main...HEAD --stat`. For each changed file, say what triggers it: a user
   action, an OS event, startup, or nothing (idle). Anything that runs while idle is a finding.
2. Grep the diff for timers and loops: `interval`, `sleep`, `loop {`, `setInterval`,
   `setTimeout`, `requestAnimationFrame`, `notify::` watchers, `tokio::spawn`. Each needs a
   trigger and an end.
3. Core and shell: heavy work (parsing, indexing, crypto, sync) is off the UI thread and
   incremental; no full vault walk per change or per render.
4. Frontend: `pnpm build && pnpm check:size`, and compare with `origin/main`. A new chunk in the
   initial bundle that could be lazy is a finding.
5. Website: if `apps/web` changed, `pnpm --filter @vainnor/tessera-web lighthouse`.
6. If a release build is available, measure cold start, idle CPU and memory with the methods in
   `docs/performance.md` and compare with the budgets.

Report: each finding with its trigger and cost, the measured numbers against each budget, and
what you could not measure on this OS.
