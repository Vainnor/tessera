// Fails the build if the output breaks the site's rules: no inline scripts or styles (the CSP
// would block them), no resources from another origin, no cookies-or-tracking style scripts.
// Run after `astro build`.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const dist = new URL("../dist/", import.meta.url).pathname;
const problems: string[] = [];

function walk(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });
}

const files = walk(dist);
const html = files.filter((f) => f.endsWith(".html"));
if (html.length === 0) problems.push("no HTML files in dist/");

for (const file of html) {
  const page = readFileSync(file, "utf8");
  const at = relative(dist, file);
  for (const tag of page.match(/<script\b[^>]*>/gi) ?? []) {
    if (!/\bsrc="\/[^/]/.test(tag)) problems.push(`${at}: inline or external script ${tag}`);
  }
  if (/<style\b/i.test(page)) problems.push(`${at}: inline <style>`);
  if (/\sstyle="/i.test(page)) problems.push(`${at}: style attribute`);
  if (/\son[a-z]+="/i.test(page)) problems.push(`${at}: inline event handler`);
  // Text glued to an inline element reads as one word to a screen reader. The wordmark's
  // "^v" + cursor is the one intended case.
  for (const glued of page.match(/[A-Za-z0-9.,:;]<(?:a|strong|em|span|time|code)\b[^>]*>/g) ?? []) {
    if (!glued.includes("t-wordmark__cursor")) problems.push(`${at}: no space before ${glued}`);
  }
  // Resources (not links) must be same-origin.
  for (const match of page.matchAll(
    /<(?:script|link|img|source|iframe|video|audio)\b[^>]*\b(?:src|href)="(https?:)?\/\/[^"]+"/gi,
  )) {
    if (!/rel="canonical"/.test(match[0])) problems.push(`${at}: third-party resource ${match[0]}`);
  }
}

for (const file of files.filter((f) => f.endsWith(".css") || f.endsWith(".js"))) {
  const text = readFileSync(file, "utf8");
  if (/url\(\s*["']?(https?:)?\/\//i.test(text) || /@import\s+["']?(https?:)?\/\//i.test(text)) {
    problems.push(`${relative(dist, file)}: loads a third-party resource`);
  }
}

if (problems.length > 0) {
  console.error(`check-dist: ${problems.length} problem(s)\n  ${problems.join("\n  ")}`);
  process.exit(1);
}
console.log(`check-dist: ${html.length} pages, no inline code, no third-party resources`);
