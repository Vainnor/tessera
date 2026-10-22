import { findPreset } from "../src/presets.ts";

/** A valid user theme, as a fresh mutable object each call. */
export function validUserTheme(): Record<string, unknown> {
  const base = structuredClone(findPreset("dark")) as unknown as Record<string, unknown>;
  return { ...base, id: "my-theme", name: "My theme" };
}

/** Themes the validator and the JSON Schema must both reject, with why. */
export function invalidThemes(): Array<[string, unknown]> {
  const withColors = (colors: Record<string, unknown>) => ({
    ...validUserTheme(),
    colors: { ...(validUserTheme().colors as object), ...colors },
  });
  return [
    ["not an object", ["a", "b"]],
    ["unknown top-level key", { ...validUserTheme(), css: "body{}" }],
    ["unknown color role", withColors({ link: "#ffffff" })],
    ["named color", withColors({ text: "red" })],
    ["CSS injection in a color", withColors({ text: "#ffffff; background: url(https://x)" })],
    ["8-digit hex with alpha", withColors({ text: "#ffffff80" })],
    ["missing color", { ...validUserTheme(), colors: { text: "#ffffff" } }],
    ["schemaVersion 2", { ...validUserTheme(), schemaVersion: 2 }],
    ["uppercase id", { ...validUserTheme(), id: "Mine" }],
    ["empty name", { ...validUserTheme(), name: "" }],
    ["control character in name", { ...validUserTheme(), name: "a\u0007b" }],
    ["unknown mode", { ...validUserTheme(), mode: "sepia" }],
    ["radius not an integer", { ...validUserTheme(), radius: 2.5 }],
    ["radius too large", { ...validUserTheme(), radius: 40 }],
    ["type scale too small", { ...validUserTheme(), typography: { body: "sans", heading: "sans", scale: 0.5, headingCase: "none" } }],
    ["unknown font", { ...validUserTheme(), typography: { body: "Comic Sans", heading: "sans", scale: 1, headingCase: "none" } }],
    ["font url smuggled in typography", { ...validUserTheme(), typography: { body: "sans", heading: "sans", scale: 1, headingCase: "none", url: "https://x" } }],
    ["motion as string", { ...validUserTheme(), motion: "false" }],
  ];
}
