// Checks built asset sizes against the budgets in docs/performance.md. Run after `pnpm build`.
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { gzipSync } from "node:zlib";

interface Budget {
  label: string;
  dir: string;
  ext: string;
  /** Gzipped bytes for all matching files together. */
  maxGzip: number;
}

const BUDGETS: Budget[] = [
  {
    label: "desktop JS (initial)",
    dir: "apps/desktop/dist/assets",
    ext: ".js",
    maxGzip: 100 * 1024,
  },
  { label: "desktop CSS", dir: "apps/desktop/dist/assets", ext: ".css", maxGzip: 10 * 1024 },
  { label: "website CSS", dir: "apps/web/dist/_astro", ext: ".css", maxGzip: 10 * 1024 },
  { label: "website JS", dir: "apps/web/dist", ext: ".js", maxGzip: 2 * 1024 },
];

const root = new URL("..", import.meta.url).pathname;
let failed = false;

for (const budget of BUDGETS) {
  const dir = join(root, budget.dir);
  let files: string[];
  try {
    files = readdirSync(dir).filter((f) => f.endsWith(budget.ext));
  } catch {
    console.log(`MISSING  ${budget.label}: ${budget.dir} does not exist; run pnpm build`);
    failed = true;
    continue;
  }
  const gzip = files.reduce((sum, f) => sum + gzipSync(readFileSync(join(dir, f))).byteLength, 0);
  const ok = gzip <= budget.maxGzip;
  failed ||= !ok;
  const kb = (n: number) => `${(n / 1024).toFixed(1)} KB`;
  console.log(
    `${ok ? "PASS" : "FAIL"}     ${budget.label}: ${kb(gzip)} gzip of ${kb(budget.maxGzip)}`,
  );
}

process.exit(failed ? 1 : 0);
