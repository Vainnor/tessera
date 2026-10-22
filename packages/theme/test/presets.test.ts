import { Ajv2020 } from "ajv/dist/2020.js";
import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

import schema from "../schema/theme.schema.json" with { type: "json" };
import { contrastFailures, contrastRatio } from "../src/contrast.ts";
import { DEFAULT_THEME, PRESETS, findPreset } from "../src/presets.ts";
import { BUILT_IN_THEME_IDS } from "../src/validate.ts";

const schemaValidate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
const presetDir = new URL("../presets/", import.meta.url);

describe("contrastRatio", () => {
  it("matches the WCAG reference values", () => {
    expect(contrastRatio("#000000", "#ffffff")).toBeCloseTo(21, 5);
    expect(contrastRatio("#ffffff", "#ffffff")).toBeCloseTo(1, 5);
    expect(contrastRatio("#767676", "#ffffff")).toBeCloseTo(4.54, 2);
  });

  it("is symmetric", () => {
    expect(contrastRatio("#2e3440", "#c8b4f0")).toBe(contrastRatio("#c8b4f0", "#2e3440"));
  });
});

describe("presets", () => {
  it("loads every file in presets/", () => {
    const files = readdirSync(presetDir).filter((f) => f.endsWith(".json")).sort();
    expect(PRESETS.map((t) => `${t.id}.json`).sort()).toEqual(files);
  });

  it("reserves exactly the preset ids", () => {
    expect([...BUILT_IN_THEME_IDS].sort()).toEqual(PRESETS.map((t) => t.id).sort());
  });

  it("includes light, dark and the Vainnor default", () => {
    expect(DEFAULT_THEME).toEqual({ dark: "vainnor", light: "light" });
    expect(findPreset("vainnor")?.mode).toBe("dark");
    expect(findPreset("light")?.mode).toBe("light");
    expect(findPreset("dark")?.mode).toBe("dark");
  });

  it.each(PRESETS.map((t) => [t.id, t] as const))("%s meets WCAG AA", (_id, theme) => {
    expect(contrastFailures(theme)).toEqual([]);
  });

  it("high-contrast meets AAA (7:1) for every text pair", () => {
    const theme = findPreset("high-contrast");
    expect(theme).toBeDefined();
    const text = ["text", "textMuted", "accent", "danger", "success", "warning"] as const;
    for (const role of text) {
      expect(contrastRatio(theme!.colors[role], theme!.colors.background)).toBeGreaterThanOrEqual(7);
    }
  });

  it.each(readdirSync(presetDir).filter((f) => f.endsWith(".json")))(
    "%s validates against the JSON Schema",
    (file) => {
      const json: unknown = JSON.parse(readFileSync(new URL(file, presetDir), "utf8"));
      expect(schemaValidate(json), JSON.stringify(schemaValidate.errors)).toBe(true);
    },
  );
});
