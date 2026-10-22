---
paths:
  - "apps/desktop/src/**"
  - "apps/desktop/index.html"
  - "apps/desktop/vite.config.ts"
  - "packages/**/*.ts"
  - "packages/**/*.tsx"
  - "eslint.config.js"
---

# Frontend Rules (not loaded at startup; read in full before writing TypeScript or React)

- React + TypeScript (strict, `noUncheckedIndexedAccess`) + Vite. No UI framework, no CSS-in-JS,
  no state library until a screen proves it needs one.
- **The UI decides nothing.** It renders what a command returns and sends intent back. Validation,
  ordering, storage, sync and crypto are `tessera-core`.
- **All IPC goes through `apps/desktop/src/lib/ipc.ts`**: one typed function per command, and the
  only file that imports `@tauri-apps/*` (ESLint enforces it). Types in `src/lib/types.ts` mirror
  the Rust structs by hand; change both in the same commit.
- **No inline styles, no `<style>` injection, no `style` prop**: the production CSP has no
  `'unsafe-inline'`. Style with CSS files and tokens (`theming.md`). Runtime theme variables go
  through `style.setProperty`, which the CSP allows (`applyTheme` in `packages/theme`).
- No `dangerouslySetInnerHTML`, `eval`, `new Function`, dynamic `import()` of a URL, or remote URLs
  in `src`, `href` or CSS. Note content will be rendered through a sanitizer, never as raw HTML.
- Browser storage (`localStorage`) only for UI preferences, wrapped in `try`/`catch`, and the stored
  value is validated on read (`src/lib/theme-choice.ts`). Never for note content or credentials.
- `react-hooks/exhaustive-deps` is an error. A deliberate omission carries its reason on the line.
- TypeScript is held at 6.0 (ADR 0001). Imports between source files use the `.ts`/`.tsx`
  extension so Node can run scripts directly.
- Dependencies are dev-only unless there is a runtime reason; the initial bundle has a budget
  (`docs/performance.md`).
