---
paths:
  - "apps/desktop/src/**"
  - "apps/web/src/**"
  - "packages/ui/**"
---

# Accessibility Rules (not loaded at startup; read in full before writing UI)

WCAG 2.2 AA is the floor. `docs/design-principles.md` → "Accessibility" is the why.

- Semantic HTML first: `button` for actions, `a` for navigation, real `label`s for inputs,
  landmarks (`header`, `nav`, `main`, `footer`), one `h1`, headings in order.
- Every interactive element is reachable and operable by keyboard, in a logical order, with the
  shared visible focus style. Never `outline: none` without a replacement.
- `Escape` closes the innermost overlay; focus returns to what opened it. Dialogs trap focus.
- Contrast comes from tokens that CI checks; do not pair colors the contrast table does not cover.
- Hit targets at least 24×24 px; primary controls 44×44 px (`.t-button` is 2.75rem).
- Meaning never rides on color alone. Icon-only controls have an accessible name.
- Motion uses the duration tokens, which go to zero under reduced motion or `motion: false`.
- Text sizes in `rem` so OS and browser font scaling work. No text in images.
- Status changes (sync state, save errors) are announced through a polite live region.
- Verify: keyboard-only pass on every changed screen, and Lighthouse accessibility ≥ 0.95 for the
  site. Component-level automated checks (axe) for the desktop app are a planned addition.
