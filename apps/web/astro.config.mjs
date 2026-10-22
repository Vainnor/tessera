// @ts-check
import { defineConfig } from "astro/config";

export default defineConfig({
  site: "https://tessera.vainnor.com",
  output: "static",
  trailingSlash: "ignore",
  // Astro's default compression drops the whitespace between text and an inline element on the
  // next source line, so "the <a>GNU</a>" renders as "theGNU" and screen readers run words
  // together. scripts/check-dist.ts fails the build if that comes back.
  compressHTML: false,
  build: {
    format: "directory",
    // The CSP allows only same-origin stylesheets, so CSS must never be inlined.
    inlineStylesheets: "never",
  },
  // No prefetch script, no dev toolbar: the site ships no JavaScript beyond public/theme.js.
  prefetch: false,
  devToolbar: { enabled: false },
  server: { host: "127.0.0.1", port: 4321 },
});
