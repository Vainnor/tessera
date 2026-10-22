---
paths:
  - "crates/**"
  - "apps/desktop/**"
  - "apps/web/astro.config.mjs"
  - "apps/web/public/**"
---

# Performance Rules (not loaded at startup; read in full before adding work that runs without a user action, or a dependency to a bundle)

The budgets and how each is measured are `docs/performance.md`. A change that crosses a budget is
a bug; a change that moves a metric more than 10% says so in the PR with before and after numbers.

- **No work without a trigger.** No timers, polling or `setInterval` while idle. File watching
  uses OS notifications, debounced. Every background task has an end.
- **Heavy work in Rust, off the UI thread**: parsing, indexing, sync, crypto. The webview renders.
- **Incremental, never full rescans**: indexes and backlinks update per changed file.
- **Lazy-load** the editor, PDF and drawing code as separate chunks.
- `pnpm check:size` runs in CI; a new runtime dependency reports its gzipped cost in the PR.
- Measure before optimizing. Numbers you did not measure are labeled estimates.
