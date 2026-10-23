# Tessera website

The static site for `tessera.vainnor.com`, built with Astro. It ships one script
(`public/theme.js`, under 2 KB: applies the saved theme and reveals the theme picker) and no
third-party requests of any kind.

```sh
pnpm dev:web                                  # from the repo root: builds tokens, then serves on 127.0.0.1:4321
pnpm --filter @vainnor/tessera-web build      # astro check + astro build + scripts/check-dist.ts
pnpm --filter @vainnor/tessera-web lighthouse # Lighthouse CI budget against dist/ (needs Chrome; runs a pinned @lhci/cli via pnpm dlx)
```

## Guards

- `scripts/check-dist.ts` fails the build on an inline script or style, a `style=` attribute, an
  inline event handler, any resource from another origin, or text glued to an inline element.
- `lighthouserc.json` requires performance, accessibility and best practices of at least 0.95,
  under 4 KB of script, under 50 KB per page and zero third-party requests. SEO is held to 0.9:
  Lighthouse fetches `robots.txt` from inside the page, and `connect-src 'none'` blocks that
  fetch. Crawlers read `robots.txt` directly and are unaffected, so the stricter CSP wins.
  Reports stay on disk (`.lighthouseci/`); nothing is uploaded.
- `test/headers.test.ts` keeps `public/_headers` and the meta CSP in `src/layouts/Base.astro`
  in step.

## Hosting

The site runs as a container on the maintainer's homelab, behind its TLS-terminating reverse
proxy. `Dockerfile` builds the site and serves `dist/` with Caddy on :8080 as an unprivileged
user; `Caddyfile` sets the same headers as `public/_headers` and writes no access log.
`test/headers.test.ts` keeps the two in step. `public/_headers` still works as-is on Cloudflare
Pages and Netlify.

```sh
docker build -f apps/web/Dockerfile -t tessera-web .   # from the repo root
docker run --rm --read-only --tmpfs /tmp --cap-drop=ALL -p 127.0.0.1:8080:8080 tessera-web
```

When CI passes on main, `.github/workflows/images.yml` pushes
`ghcr.io/vainnor/tessera-web:<version>-<sha7>` and moves the `prod` tag to it; the homelab
deploys `prod`.

The privacy page says the host's access-log retention is still to be confirmed; update it when
hosting is chosen.
