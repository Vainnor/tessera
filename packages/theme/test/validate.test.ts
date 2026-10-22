import { Ajv2020 } from "ajv/dist/2020.js";
import { describe, expect, it } from "vitest";

import schema from "../schema/theme.schema.json" with { type: "json" };
import { MAX_THEME_BYTES, parseTheme, validateTheme } from "../src/validate.ts";
import { invalidThemes, validUserTheme } from "./fixtures.ts";

const schemaValidate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);

describe("validateTheme", () => {
  it("accepts a valid user theme and returns a new object", () => {
    const input = validUserTheme();
    const result = validateTheme(input);
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.theme).not.toBe(input);
      expect(result.theme.id).toBe("my-theme");
      expect(result.contrastWarnings).toEqual([]);
    }
  });

  it("normalizes colors to lowercase", () => {
    const input = validUserTheme();
    (input.colors as Record<string, string>).text = "#ABCDEF";
    const result = validateTheme(input);
    expect(result.ok && result.theme.colors.text).toBe("#abcdef");
  });

  it("drops $schema from the result", () => {
    const result = validateTheme({ ...validUserTheme(), $schema: "./theme.schema.json" });
    expect(result.ok && "$schema" in result.theme).toBe(false);
  });

  it("refuses a user theme that reuses a built-in id", () => {
    const result = validateTheme({ ...validUserTheme(), id: "vainnor" });
    expect(result).toEqual({ ok: false, errors: ['id: "vainnor" belongs to a built-in theme'] });
  });

  it("accepts low contrast but reports it as warnings", () => {
    const input = validUserTheme();
    (input.colors as Record<string, string>).textMuted = "#30343c";
    const result = validateTheme(input);
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.contrastWarnings.map((w) => `${w.foreground}/${w.background}`)).toContain(
        "textMuted/background",
      );
    }
  });

  it.each(invalidThemes())("rejects %s", (_why, theme) => {
    expect(validateTheme(theme).ok).toBe(false);
  });

  it.each(invalidThemes())("the JSON Schema also rejects %s", (_why, theme) => {
    expect(schemaValidate(theme)).toBe(false);
  });

  it("the JSON Schema accepts the valid fixture", () => {
    expect(schemaValidate(validUserTheme()), JSON.stringify(schemaValidate.errors)).toBe(true);
  });
});

describe("parseTheme", () => {
  it("refuses oversized input before parsing it", () => {
    const huge = "x".repeat(MAX_THEME_BYTES + 1);
    expect(parseTheme(huge)).toEqual({
      ok: false,
      errors: [`theme files are limited to ${MAX_THEME_BYTES} bytes`],
    });
  });

  it("counts bytes, not characters", () => {
    const multibyte = `"${"é".repeat(MAX_THEME_BYTES / 2)}"`;
    expect(parseTheme(multibyte)).toMatchObject({ ok: false, errors: [expect.stringContaining("limited")] });
  });

  it("reports invalid JSON", () => {
    expect(parseTheme("{ not json")).toEqual({ ok: false, errors: ["not valid JSON"] });
  });

  it("does not let a __proto__ key through", () => {
    const source = JSON.stringify(validUserTheme()).replace(/^\{/, '{"__proto__":{"polluted":true},');
    expect(parseTheme(source).ok).toBe(false);
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  });

  it("round-trips a valid file", () => {
    expect(parseTheme(JSON.stringify(validUserTheme())).ok).toBe(true);
  });
});
