import { describe, expect, it } from "vitest";

import { applyTheme } from "../src/apply.ts";
import { buildStylesheet, themeVariables } from "../src/css.ts";
import { DEFAULT_THEME, PRESETS, findPreset } from "../src/presets.ts";

const css = buildStylesheet(PRESETS, DEFAULT_THEME);

describe("buildStylesheet", () => {
  it("has a block for every preset", () => {
    for (const theme of PRESETS) expect(css).toContain(`[data-theme="${theme.id}"] {`);
  });

  it("defaults to Vainnor, and to Light when the OS prefers light", () => {
    const root = css.slice(css.indexOf(":root {"), css.indexOf("}", css.indexOf(":root {")));
    expect(root).toContain("--t-color-background: #2e3440;");
    expect(css).toMatch(
      /@media \(prefers-color-scheme: light\) \{\s+:root:not\(\[data-theme\]\) \{\s+--t-color-background: #ffffff;/,
    );
  });

  it("zeroes motion when the OS asks for reduced motion", () => {
    expect(css).toMatch(/@media \(prefers-reduced-motion: reduce\) \{[^@]*--t-motion-pref: 0;/);
  });

  it("emits no url(), @import or expression()", () => {
    expect(css).not.toMatch(/url\(|@import|expression\(/i);
  });
});

describe("themeVariables", () => {
  it("turns motion off for a no-motion theme", () => {
    expect(themeVariables(findPreset("vainnor")!)["--t-motion"]).toBe("0");
    expect(themeVariables(findPreset("light")!)["--t-motion"]).toBe("1");
  });

  it("uses no shadows on a flat theme", () => {
    expect(themeVariables(findPreset("vainnor")!)["--t-shadow-2"]).toBe("none");
  });
});

/** The slice of HTMLElement that applyTheme touches. */
function fakeRoot() {
  const props = new Map<string, string>();
  const root = {
    dataset: {} as Record<string, string>,
    style: {
      setProperty: (name: string, value: string) => void props.set(name, value),
      removeProperty: (name: string) => void props.delete(name),
    },
  };
  return { root: root as unknown as HTMLElement, props, dataset: root.dataset };
}

describe("applyTheme", () => {
  it("selects a preset through data-theme only", () => {
    const { root, props, dataset } = fakeRoot();
    applyTheme("paper", root);
    expect(dataset.theme).toBe("paper");
    expect(props.size).toBe(0);
  });

  it("removes data-theme for auto and for an unknown id", () => {
    const { root, dataset } = fakeRoot();
    applyTheme("paper", root);
    applyTheme("auto", root);
    expect(dataset.theme).toBeUndefined();
    applyTheme("no-such-theme", root);
    expect(dataset.theme).toBeUndefined();
  });

  it("writes a user theme's variables and clears them on switching back", () => {
    const { root, props, dataset } = fakeRoot();
    const custom = { ...findPreset("dark")!, id: "mine", name: "Mine" };
    applyTheme(custom, root);
    expect(dataset.theme).toBe("custom");
    expect(props.get("--t-color-background")).toBe(custom.colors.background);
    expect(props.get("color-scheme")).toBe("dark");
    applyTheme("light", root);
    expect(props.size).toBe(0);
  });
});
