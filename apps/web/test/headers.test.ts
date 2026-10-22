import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const headers = readFileSync(new URL("../public/_headers", import.meta.url), "utf8");
const layout = readFileSync(new URL("../src/layouts/Base.astro", import.meta.url), "utf8");

function header(name: string): string {
  const line = headers.split("\n").find((l) => l.trim().startsWith(`${name}:`));
  if (!line) throw new Error(`missing header ${name}`);
  return line.slice(line.indexOf(":") + 1).trim();
}

/** The CSP string concatenated in Base.astro's frontmatter. */
function layoutCsp(): string {
  const match = /const csp =\s*([\s\S]*?);\n/.exec(layout);
  if (!match?.[1]) throw new Error("no csp constant in Base.astro");
  return [...match[1].matchAll(/"([^"]*)"/g)].map((m) => m[1]).join("");
}

describe("security headers", () => {
  it("sets a CSP that allows only same-origin scripts and styles and nothing else", () => {
    const csp = header("Content-Security-Policy");
    expect(csp).toContain("default-src 'none'");
    expect(csp).toContain("script-src 'self'");
    expect(csp).toContain("style-src 'self'");
    expect(csp).toContain("connect-src 'none'");
    expect(csp).toContain("frame-ancestors 'none'");
    expect(csp).not.toMatch(/unsafe-inline|unsafe-eval|https?:|\*/);
  });

  it("keeps the meta CSP in Base.astro identical apart from header-only directives", () => {
    const fromHeader = header("Content-Security-Policy")
      .replace("; frame-ancestors 'none'", "")
      .replace("; upgrade-insecure-requests", "");
    expect(layoutCsp()).toBe(fromHeader);
  });

  it.each([
    ["Strict-Transport-Security", "max-age=63072000; includeSubDomains"],
    ["X-Content-Type-Options", "nosniff"],
    ["X-Frame-Options", "DENY"],
    ["Referrer-Policy", "no-referrer"],
    ["Cross-Origin-Opener-Policy", "same-origin"],
    ["Cross-Origin-Resource-Policy", "same-origin"],
  ])("sets %s", (name, value) => {
    expect(header(name)).toBe(value);
  });

  it("turns off powerful browser features", () => {
    for (const feature of ["camera", "microphone", "geolocation", "browsing-topics"]) {
      expect(header("Permissions-Policy")).toContain(`${feature}=()`);
    }
  });
});
