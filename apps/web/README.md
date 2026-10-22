# Tessera website

The static site for `tessera.vainnor.com`, built with Astro. It ships one script
(`public/theme.js`, under 2 KB: applies the saved theme and reveals the theme picker) and no
third-party requests of any kind.

```sh
pnpm dev:web                                  # from the repo root: builds tokens, then serves on 127.0.0.1:4321
pnpm --filter @vainnor/tessera-web build      # astro check + astro build + scripts/check-dist.ts
pnpm --filter @vainnor/tessera-web lighthouse # Lighthouse CI budget against dist/ (needs Chrome)
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

Not decided yet. `public/_headers` works as-is on Cloudflare Pages and Netlify. For Caddy:

```caddy
tessera.vainnor.com {
	root * /srv/tessera-web
	file_server
	header {
		Content-Security-Policy "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; manifest-src 'self'; connect-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'; upgrade-insecure-requests"
		Strict-Transport-Security "max-age=63072000; includeSubDomains"
		X-Content-Type-Options "nosniff"
		X-Frame-Options "DENY"
		Referrer-Policy "no-referrer"
		Permissions-Policy "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=(), browsing-topics=()"
		Cross-Origin-Opener-Policy "same-origin"
		Cross-Origin-Resource-Policy "same-origin"
		Cross-Origin-Embedder-Policy "require-corp"
		-Server
	}
	@assets path /_astro/*
	header @assets Cache-Control "public, max-age=31536000, immutable"
	# Access logs: keep them short-lived and say so on /privacy/ before launch.
	log {
		output file /var/log/caddy/tessera-web.log {
			roll_keep_for 168h
		}
	}
}
```

The privacy page says the host's access-log retention is still to be confirmed; update it when
hosting is chosen.
