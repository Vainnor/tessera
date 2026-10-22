import { describe, expect, it } from "vitest";

import { FEATURES } from "../src/data/site.ts";

describe("home page feature list", () => {
  it("lists every feature the product vision names", () => {
    expect(FEATURES.map((f) => f.title)).toEqual([
      "Local-first",
      "Plain markdown files",
      "Real-time sync",
      "End-to-end encrypted",
      "Works offline",
      "Drawing and handwriting",
      "Themes",
      "Low resource use",
      "Open source",
    ]);
  });

  // Changing this list is a claim about shipped software. Update it in the same commit as the
  // release that makes it true (.claude/rules/web.md → "Claims").
  it("claims as available only what has shipped", () => {
    expect(FEATURES.filter((f) => f.status === "available").map((f) => f.title)).toEqual([
      "Open source",
    ]);
  });
});
