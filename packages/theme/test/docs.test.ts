import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

import { parseTheme } from "../src/validate.ts";

const doc = readFileSync(new URL("../../../docs/theming.md", import.meta.url), "utf8");

describe("docs/theming.md", () => {
  it("has an example theme that imports cleanly, with no contrast warnings", () => {
    const example = /```json\n([\s\S]*?)\n```/.exec(doc)?.[1];
    expect(example).toBeDefined();
    const result = parseTheme(example!);
    expect(result).toMatchObject({ ok: true, contrastWarnings: [] });
  });
});
