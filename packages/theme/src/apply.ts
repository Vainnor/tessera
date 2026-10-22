import { themeVariables } from "./css.ts";
import { findPreset } from "./presets.ts";
import type { Theme } from "./types.ts";

/** The `localStorage` key both apps use for the chosen theme id ("auto" when unset). */
export const THEME_STORAGE_KEY = "tessera.theme";

/** `"auto"` follows the OS color scheme; anything else is a preset id. */
export type ThemeChoice = "auto" | (string & {});

const CUSTOM_PROPERTIES = new Set<string>();

function clearCustom(root: HTMLElement): void {
  for (const name of CUSTOM_PROPERTIES) root.style.removeProperty(name);
  root.style.removeProperty("color-scheme");
  CUSTOM_PROPERTIES.clear();
}

/**
 * Applies a preset by id, `"auto"`, or a validated user theme.
 *
 * Presets switch the `data-theme` attribute and use the generated stylesheet. A user theme
 * writes its variables through the CSSOM (`style.setProperty`), which a strict CSP without
 * `'unsafe-inline'` allows.
 */
export function applyTheme(
  choice: ThemeChoice | Theme,
  root: HTMLElement = document.documentElement,
): void {
  clearCustom(root);
  if (typeof choice !== "string") {
    root.dataset.theme = "custom";
    for (const [name, value] of Object.entries(themeVariables(choice))) {
      root.style.setProperty(name, value);
      CUSTOM_PROPERTIES.add(name);
    }
    root.style.setProperty("color-scheme", choice.mode);
    return;
  }
  if (choice === "auto" || findPreset(choice) === undefined) {
    delete root.dataset.theme;
  } else {
    root.dataset.theme = choice;
  }
}
