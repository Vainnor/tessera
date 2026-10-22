# Design principles

How Tessera's interface should behave, on every platform. The rules that enforce these in code
are `.claude/rules/theming.md` and `.claude/rules/accessibility.md`.

## Clarity

- One obvious primary action per screen. Secondary actions are visible but quieter.
- Plain words. Name things for what they do ("Open vault", not "Initialize workspace").
- Show the state of sync and encryption truthfully and calmly. Never imply data is encrypted or
  synced when it is not.
- No dark patterns: no nagging, no fake urgency, no hidden opt-outs.

## Density options

- People who write long-form want air; people who keep many small notes want density. Every theme
  sets a density (`compact`, `comfortable`, `spacious`) that scales all spacing, and the type
  scale is a separate setting.
- Layouts are built from spacing tokens so a density change never breaks them.

## Keyboard-first

- Every action is reachable from the keyboard, and every frequent action has a shortcut.
- A command palette (planned) reaches everything by name.
- Focus is always visible (`--t-color-focus`, 3:1 against every surface) and lands somewhere
  sensible after every action: a dialog returns focus to what opened it.
- `Escape` closes the innermost thing and nothing else.

## Accessibility

- WCAG 2.2 AA is the floor for every preset; contrast is checked in CI. High contrast reaches AAA.
- Respect the OS: `prefers-color-scheme` picks the default theme, `prefers-reduced-motion` turns
  every transition off, and text sizes are in `rem` so system font scaling works.
- Hit targets are at least 24×24 px, and 44×44 px for primary controls.
- Meaning never rides on color alone: status has text, icons have labels.
- Screen readers get real structure: landmarks, headings in order, labeled controls, live regions
  for sync status.

## Customizable

- Themes change tokens only (colors, fonts, radius, density, elevation, motion), never layout or
  behavior, so a user theme cannot break the app. The format is [`theming.md`](theming.md).
- The Vainnor terminal look is the default brand, not a requirement.

## Calm and fast

- Local data shows on first paint. No spinners for things on disk.
- Motion is short (120 to 320 ms), functional, and optional.
