import dark from "../presets/dark.json" with { type: "json" };
import highContrast from "../presets/high-contrast.json" with { type: "json" };
import light from "../presets/light.json" with { type: "json" };
import paper from "../presets/paper.json" with { type: "json" };
import vainnor from "../presets/vainnor.json" with { type: "json" };
import type { Theme } from "./types.ts";
import { validateTheme } from "./validate.ts";

function preset(json: unknown): Theme {
  const result = validateTheme(json, { allowBuiltInId: true });
  if (!result.ok) throw new Error(`invalid preset: ${result.errors.join("; ")}`);
  return result.theme;
}

/** The built-in themes, in the order a picker shows them. */
export const PRESETS: readonly Theme[] = [vainnor, light, dark, paper, highContrast].map(preset);

/** The theme used when the user has not chosen one, by the OS color scheme. */
export const DEFAULT_THEME = { dark: "vainnor", light: "light" } as const;

export function findPreset(id: string): Theme | undefined {
  return PRESETS.find((theme) => theme.id === id);
}
