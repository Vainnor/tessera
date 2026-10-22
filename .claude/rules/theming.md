---
paths:
  - "packages/theme/**"
  - "packages/ui/**"
  - "**/*.css"
  - "apps/desktop/src/**/*.tsx"
  - "apps/web/src/**/*.astro"
---

# Theming and Design-Token Rules (not loaded at startup; read in full before UI, CSS or theme work)

The theme format and token list are `docs/theming.md`; the principles are
`docs/design-principles.md`.

## Tokens only

- Every color, font, size, spacing, radius, shadow and duration in a stylesheet is a `var(--t-*)`
  token. No hex, `rgb()`, named color, pixel font size or literal duration outside
  `packages/theme`. A missing token is added to the theme package and `docs/theming.md`, never
  inlined. Exceptions: `1px` borders, `0`, and layout values (`grid-template-*`, `min()` widths
  in `rem`).
- Spacing uses `--t-space-1..8`; type uses `--t-text-*`. Nothing in between.
- A theme changes token values, never layout or behavior.
- Components use semantic roles (`--t-color-text-muted`), never a raw palette step.

## Changing the theme system

- A token or theme field changes `src/types.ts`, the JSON Schema, `src/validate.ts`, `src/css.ts`,
  every preset, `docs/theming.md` and the tests in one commit.
- Every preset passes `pnpm check:contrast` (WCAG AA for every pairing in `src/contrast.ts`). A
  new color role adds its pairings there.
- `dist/tessera.css` is generated (`pnpm --filter @vainnor/tessera-theme build`); never edit it.
- The Vainnor palette is provisional and shared with `~/Programing/muster/themes/vainnor.json`;
  change both together.
