# Performance budgets

Tessera should feel instant and cost nothing when idle. These budgets are set now, before the
features that could break them. A change that pushes a metric past its budget is a bug; a change
that moves one by more than 10% says so in the pull request, with numbers.

Reference machine: a mid-range laptop from the last four years (for example an M1 MacBook Air,
or a Windows laptop with a 4-core CPU and 8 GB RAM). Release builds only.

## Desktop app

| Metric                           | Budget                                         | Measured by                                                                                                                       |
| -------------------------------- | ---------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| Cold start to interactive window | < 500 ms empty; < 1 s with a 10,000-note vault | `hyperfine` on launch-to-first-paint once a `--measure-startup` flag exists; until then the webview inspector's performance panel |
| Idle CPU, window open, no input  | < 0.1% averaged over 60 s                      | macOS `top -pid`, Activity Monitor "Idle Wake Ups"; Linux `pidstat`; Windows Process Explorer                                     |
| Memory, idle, empty vault        | < 100 MB total, including webview processes    | Sum of RSS of the app and its webview children                                                                                    |
| Memory, 10,000-note vault open   | < 250 MB total                                 | Same                                                                                                                              |
| Initial JS bundle                | < 100 KB gzipped (currently 70 KB)             | `pnpm check:size` in CI                                                                                                           |
| Desktop CSS                      | < 10 KB gzipped                                | `pnpm check:size`                                                                                                                 |
| Installer                        | < 15 MB                                        | The artifacts the release workflow produces                                                                                       |
| Typing latency in the editor     | < 16 ms per keystroke at the 99th percentile   | To be defined with the editor                                                                                                     |

## Sync server

| Metric      | Budget                                             |
| ----------- | -------------------------------------------------- |
| Idle memory | < 20 MB RSS                                        |
| Idle CPU    | 0%: no timers or polling without connected clients |
| `/healthz`  | < 1 ms in process                                  |

## Website

| Metric                                                | Budget                                        | Measured by                                  |
| ----------------------------------------------------- | --------------------------------------------- | -------------------------------------------- |
| Lighthouse performance, accessibility, best practices | ≥ 0.95 on every page                          | Lighthouse CI (`apps/web/lighthouserc.json`) |
| Total transfer per page                               | < 50 KB                                       | Lighthouse CI                                |
| JavaScript                                            | < 2 KB gzipped (`theme.js`, currently 0.7 KB) | `pnpm check:size`, Lighthouse CI             |
| Third-party requests                                  | 0                                             | Lighthouse CI and `scripts/check-dist.ts`    |

## Rules that keep the budgets

- **No work without a trigger.** Nothing runs on a timer while idle. File watching uses OS
  notifications, debounced. Background work has an end.
- **Lazy by default.** The editor, PDF viewer and drawing tools load as separate chunks when first
  opened, not with the shell.
- **Index incrementally.** Search and backlinks update per changed file, never by rescanning the
  vault.
- **Heavy work in Rust, off the UI thread.** Parsing, indexing, sync and crypto run in the core on
  a worker thread; the webview renders.
- **Measure before optimizing**, and put the before and after numbers in the pull request.
