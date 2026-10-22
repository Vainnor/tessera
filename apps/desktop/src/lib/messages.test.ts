import { describe, expect, it } from "vitest";

import { errorMessage, isCommandError, skippedSummary, withMarkdownExtension } from "./messages.ts";

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

describe("isCommandError", () => {
  it("recognizes a command rejection", () => {
    expect(isCommandError({ code: "conflict", message: "changed" })).toBe(true);
  });

  it("rejects anything without both fields", () => {
    expect(isCommandError(new Error("x"))).toBe(false);
    expect(isCommandError({ code: 1, message: "x" })).toBe(false);
    expect(isCommandError("conflict")).toBe(false);
  });
});

describe("withMarkdownExtension", () => {
  it("adds .md when it is missing", () => {
    expect(withMarkdownExtension("Projects/plan")).toBe("Projects/plan.md");
  });

  it("keeps an existing extension in any case and trims spaces", () => {
    expect(withMarkdownExtension(" notes/A.MD ")).toBe("notes/A.MD");
  });
});
