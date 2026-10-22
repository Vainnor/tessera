import { describe, expect, it } from "vitest";

import { errorMessage, skippedSummary } from "./messages.ts";

describe("errorMessage", () => {
  it("shows a command error's message", () => {
    expect(errorMessage({ code: "note-not-found", message: "note not found" })).toBe(
      "note not found",
    );
  });

  it("falls back for anything else", () => {
    expect(errorMessage(new Error(""))).toBe("Something went wrong.");
    expect(errorMessage("boom")).toBe("Something went wrong.");
    expect(errorMessage(null)).toBe("Something went wrong.");
    expect(errorMessage({ message: 42 })).toBe("Something went wrong.");
  });
});

describe("skippedSummary", () => {
  it("says nothing when nothing was skipped", () => {
    expect(skippedSummary([])).toBeNull();
  });

  it("counts each reason with the right plural", () => {
    expect(
      skippedSummary([
        { path: "a", reason: "symlink" },
        { path: "b", reason: "symlink" },
        { path: "c?.md", reason: "unsupported-name" },
      ]),
    ).toBe("3 items not shown: 2 symbolic links, 1 name not every system can store.");
  });

  it("uses the singular for one item", () => {
    expect(skippedSummary([{ path: "x", reason: "unreadable" }])).toBe(
      "1 item not shown: 1 unreadable folder.",
    );
  });
});
