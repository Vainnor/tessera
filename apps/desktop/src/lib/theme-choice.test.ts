import { THEME_STORAGE_KEY } from "@vainnor/tessera-theme";
import { describe, expect, it } from "vitest";

import { loadChoice, saveChoice, type ChoiceStorage } from "./theme-choice.ts";

function memoryStorage(initial: Record<string, string> = {}): ChoiceStorage {
  const data = new Map(Object.entries(initial));
  return {
    getItem: (key) => data.get(key) ?? null,
    setItem: (key, value) => void data.set(key, value),
  };
}

const throwing: ChoiceStorage = {
  getItem: () => {
    throw new Error("SecurityError");
  },
  setItem: () => {
    throw new Error("QuotaExceededError");
  },
};

describe("loadChoice", () => {
  it("returns a saved preset id", () => {
    expect(loadChoice(memoryStorage({ [THEME_STORAGE_KEY]: "paper" }))).toBe("paper");
  });

  it("falls back to auto for nothing saved, an unknown id, no storage or throwing storage", () => {
    expect(loadChoice(memoryStorage())).toBe("auto");
    expect(loadChoice(memoryStorage({ [THEME_STORAGE_KEY]: "<script>" }))).toBe("auto");
    expect(loadChoice(undefined)).toBe("auto");
    expect(loadChoice(throwing)).toBe("auto");
  });
});

describe("saveChoice", () => {
  it("round-trips through loadChoice", () => {
    const storage = memoryStorage();
    expect(saveChoice(storage, "high-contrast")).toBe(true);
    expect(loadChoice(storage)).toBe("high-contrast");
  });

  it("reports failure instead of throwing", () => {
    expect(saveChoice(throwing, "dark")).toBe(false);
    expect(saveChoice(undefined, "dark")).toBe(false);
  });
});
