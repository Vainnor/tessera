# Privacy Rules (always loaded)

The website's privacy page (`apps/web/src/pages/privacy.astro`) is the public promise. Code that
would make it untrue does not ship until the page is changed and the user has agreed.

- **No telemetry, analytics, crash reporting or usage pings**, opt-in or not, in the app, the
  server or the site. No Sentry, PostHog, Plausible or "anonymous" counters.
- **No network by default.** The app works fully offline. Sync, when built, runs only once the
  user sets it up. Update checks will be manual.
- **No third-party requests.** No CDN fonts, scripts, images or icons, no remote theme gallery, no
  GitHub API calls from the visitor's browser. Everything ships in the bundle.
- **No cookies, accounts or fingerprinting on the site.** The only stored value is the theme
  choice, in the visitor's own `localStorage`.
- **Logs never hold user data.** No note content, file names, vault paths, IP addresses or
  account ids at `info` or above, in the app or the server. `debug` may log opaque ids.
- **Fixtures are synthetic.** No real notes, home directories or usernames in tests, docs or issues.
