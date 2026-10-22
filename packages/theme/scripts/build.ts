// Writes dist/tessera.css from the presets. Run by `pnpm build`.
import { mkdirSync, writeFileSync } from "node:fs";

import { buildStylesheet } from "../src/css.ts";
import { DEFAULT_THEME, PRESETS } from "../src/presets.ts";

const out = new URL("../dist/tessera.css", import.meta.url);
mkdirSync(new URL(".", out), { recursive: true });
const css = buildStylesheet(PRESETS, DEFAULT_THEME);
writeFileSync(out, css);
console.log(`wrote dist/tessera.css (${PRESETS.length} presets, ${css.length} bytes)`);
