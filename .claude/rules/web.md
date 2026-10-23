---
paths:
  - "apps/web/**"
---

# Website Rules (not loaded at startup; read in full before changing the website)

- Static Astro output only. No islands, no client framework, no view transitions, no prefetch, no
  analytics. The only script is `public/theme.js`; anything more needs a reason and must fit the
  2 KB budget.
- CSP: same-origin scripts and styles only, `connect-src 'none'`. `inlineStylesheets: "never"`
  and `compressHTML: false` in `astro.config.mjs` are load-bearing; `scripts/check-dist.ts` fails
  the build if inline code, a third-party resource or glued inline text appears.
- The CSP and headers live in three places that must match: `public/_headers`, the meta tag in
  `src/layouts/Base.astro` and the container's `Caddyfile`. `test/headers.test.ts` enforces it.
- No third-party fonts, images, embeds or badges. Link to GitHub; never load from it.
- Styling uses tokens and the `@vainnor/tessera-ui` classes (`theming.md`).

## Claims

- **Only claim what exists.** Every feature on the site carries a status from `src/data/site.ts`:
  `available` (in a release), `in-progress` (in the source, not released) or `planned`. A status
  moves to `available` in the same commit as the release that makes it true;
  `test/features.test.ts` makes that a deliberate edit.
- Every factual claim (a size, "no cookies", "encrypted") is true of the code at the commit that
  carries it.
- Privacy and terms pages are drafts needing legal review and say so. Changing what the app or site
  collects changes `privacy.astro` in the same commit (`privacy.md`).
- Copy follows `writing.md`.
