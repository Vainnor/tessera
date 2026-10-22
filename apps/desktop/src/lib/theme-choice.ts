import { THEME_STORAGE_KEY, findPreset, type ThemeChoice } from "@vainnor/tessera-theme";

/** The parts of `Storage` this module uses, so tests can pass a fake. */
export type ChoiceStorage = Pick<Storage, "getItem" | "setItem">;

/**
 * The saved theme choice, or `"auto"`. Storage can be missing or throw (private mode, blocked
 * storage), and the stored value is untrusted, so anything but a known id reads as `"auto"`.
 */
export function loadChoice(storage: ChoiceStorage | undefined): ThemeChoice {
  try {
    const saved = storage?.getItem(THEME_STORAGE_KEY);
    if (saved && findPreset(saved)) return saved;
  } catch {
    // Unreadable storage behaves like no saved choice.
  }
  return "auto";
}

/** Saves the choice. Returns false when storage refused it; the theme still applies. */
export function saveChoice(storage: ChoiceStorage | undefined, choice: ThemeChoice): boolean {
  try {
    storage?.setItem(THEME_STORAGE_KEY, choice);
    return storage !== undefined;
  } catch {
    return false;
  }
}
