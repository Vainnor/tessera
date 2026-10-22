// Prints every preset's contrast against WCAG AA and exits 1 if any pair fails. Run in CI.
import { measureContrast } from "../src/contrast.ts";
import { PRESETS } from "../src/presets.ts";

let failures = 0;
for (const theme of PRESETS) {
  const results = measureContrast(theme);
  const failed = results.filter((r) => !r.passes);
  failures += failed.length;
  const lowest = Math.min(...results.filter((r) => r.minimum === 4.5).map((r) => r.ratio));
  console.log(
    `${failed.length === 0 ? "PASS" : "FAIL"}  ${theme.id.padEnd(14)} lowest text pair ${lowest.toFixed(2)}:1`,
  );
  for (const r of failed) {
    console.log(
      `      ${r.foreground} on ${r.background}: ${r.ratio.toFixed(2)}:1, needs ${r.minimum}:1`,
    );
  }
}
console.log(`${PRESETS.length} presets, ${failures} failing pairs`);
process.exit(failures === 0 ? 0 : 1);
