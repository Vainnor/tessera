# Theming

Tessera's look comes from design tokens: CSS custom properties that every component uses
instead of literal colors, sizes or durations. A theme sets the tokens; it cannot change layout
or behavior. The desktop app and the website load the same tokens from `packages/theme`.

## Presets

| Id              | Mode  | Character                                                                                                                          |
| --------------- | ----- | ---------------------------------------------------------------------------------------------------------------------------------- |
| `vainnor`       | dark  | The default. Retro terminal: monospace, lowercase headings, flat, lilac accent. The palette is provisional and shared with Muster. |
| `light`         | light | Neutral, system sans-serif, soft shadows                                                                                           |
| `dark`          | dark  | Neutral, system sans-serif, soft shadows                                                                                           |
| `paper`         | light | Warm sepia, serif, roomier spacing                                                                                                 |
| `high-contrast` | dark  | Black, white and yellow; every text pair above 7:1 (WCAG AAA)                                                                      |

With no choice saved, the OS color scheme picks: `vainnor` when dark, `light` when light. Every
preset meets WCAG AA for every pairing the components use; `pnpm check:contrast` prints the
ratios and CI fails on any miss.

## Writing your own theme

A theme is a JSON file described by
[`packages/theme/schema/theme.schema.json`](../packages/theme/schema/theme.schema.json). Start
from a preset in `packages/theme/presets/`, change `id` and `name`, and edit the values.

```json
{
  "$schema": "https://tessera.vainnor.com/theme.schema.json",
  "schemaVersion": 1,
  "id": "my-theme",
  "name": "My theme",
  "mode": "dark",
  "colors": {
    "background": "#1e1e2e",
    "surface": "#24273a",
    "surfaceRaised": "#2a2e42",
    "border": "#363a4f",
    "borderStrong": "#8087a2",
    "text": "#e6e9f5",
    "textMuted": "#b5bad0",
    "accent": "#c6a0f6",
    "onAccent": "#1e1e2e",
    "focus": "#c6a0f6",
    "danger": "#f5a3b5",
    "success": "#a6da95",
    "warning": "#eed49f",
    "selection": "#45496a"
  },
  "typography": { "body": "sans", "heading": "sans", "scale": 1, "headingCase": "none" },
  "radius": 6,
  "density": "comfortable",
  "elevation": "raised",
  "motion": true
}
```

The `$schema` URL is where the schema will be published; until the site is live, point it at the
file in the repository.

| Field                         | Values                                                                                               |
| ----------------------------- | ---------------------------------------------------------------------------------------------------- |
| `id`                          | 1 to 40 lowercase letters, digits and hyphens; not a preset id                                       |
| `name`                        | 1 to 40 characters                                                                                   |
| `mode`                        | `light` or `dark` (sets the browser's `color-scheme`)                                                |
| `colors.*`                    | `#rrggbb` only. Fourteen roles, all required; the schema describes each and its contrast requirement |
| `typography.body`, `.heading` | `sans`, `serif` or `mono`: system font stacks. Tessera loads no font files                           |
| `typography.scale`            | 0.875 to 1.25, multiplies every text size                                                            |
| `typography.headingCase`      | `none`, `lowercase` or `uppercase`                                                                   |
| `radius`                      | 0 to 16 (px)                                                                                         |
| `density`                     | `compact` (0.75×), `comfortable` (1×), `spacious` (1.25×) spacing                                    |
| `elevation`                   | `flat` (borders only) or `raised` (soft shadows)                                                     |
| `motion`                      | `false` turns all transitions off. The OS reduced-motion setting always wins                         |

### What an imported theme cannot do

The importer (`packages/theme/src/validate.ts`) refuses files over 16 KiB before parsing,
refuses unknown keys, and accepts only the value forms above. There is no way to express raw
CSS, a URL, `@import` or a font file, so a theme cannot load anything or restyle beyond the tokens.
A theme with low contrast is accepted with a warning listing the pairs that fail, because the
choice is the user's.

## Tokens

All tokens start with `--t-`. Components use only these; `.claude/rules/theming.md` forbids
literals.

| Group     | Tokens                                                                                                                                                                         |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Color     | `--t-color-{background, surface, surface-raised, border, border-strong, text, text-muted, accent, on-accent, focus, danger, success, warning, selection}`                      |
| Type      | `--t-font-{body, heading, code}`, `--t-text-{xs, sm, base, md, lg, xl, 2xl}`, `--t-leading-{tight, normal, relaxed}`, `--t-weight-{regular, medium, bold}`, `--t-heading-case` |
| Space     | `--t-space-1` … `--t-space-8` (4, 8, 12, 16, 24, 32, 48, 64 px × density)                                                                                                      |
| Radius    | `--t-radius-{sm, md, lg, full}`                                                                                                                                                |
| Elevation | `--t-shadow-1`, `--t-shadow-2` (`none` on flat themes)                                                                                                                         |
| Motion    | `--t-duration-{fast, base, slow}` (120, 200, 320 ms, or 0), `--t-ease-standard`                                                                                                |
| Focus     | `--t-focus-width`, `--t-focus-offset`                                                                                                                                          |

## Changing the format

A new field or token changes, in one commit: `src/types.ts`, `schema/theme.schema.json`,
`src/validate.ts`, `src/css.ts`, every preset, this document, and the tests. A breaking change
bumps `schemaVersion`.
