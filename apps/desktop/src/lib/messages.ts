import type { CommandError, SkipReason, SkippedEntry } from "./types.ts";

/** Whether `error` is a rejection from one of Tessera's own commands. */
export function isCommandError(error: unknown): error is CommandError {
  return (
    typeof error === "object" &&
    error !== null &&
    "code" in error &&
    typeof error.code === "string" &&
    "message" in error &&
    typeof error.message === "string"
  );
}

/** A rename target with `.md` added when the user left it off. Rust still validates it. */
export function withMarkdownExtension(path: string): string {
  const trimmed = path.trim();
  return /\.md$/i.test(trimmed) ? trimmed : `${trimmed}.md`;
}

/** The text to show for a rejected command, or a generic line for anything unexpected. */
export function errorMessage(error: unknown): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string" &&
    error.message.length > 0
  ) {
    return error.message;
  }
  return "Something went wrong.";
}

const SKIP_LABELS: Record<SkipReason, [singular: string, plural: string]> = {
  symlink: ["symbolic link", "symbolic links"],
  "unsupported-name": ["name not every system can store", "names not every system can store"],
  "too-deep": ["folder nested too deeply", "folders nested too deeply"],
  unreadable: ["unreadable folder", "unreadable folders"],
};

/** One sentence saying what a listing left out, or `null` when it left out nothing. */
export function skippedSummary(skipped: readonly SkippedEntry[]): string | null {
  if (skipped.length === 0) return null;
  const counts = new Map<SkipReason, number>();
  for (const entry of skipped) counts.set(entry.reason, (counts.get(entry.reason) ?? 0) + 1);
  const parts = [...counts].map(([reason, count]) => {
    const [one, many] = SKIP_LABELS[reason];
    return `${count} ${count === 1 ? one : many}`;
  });
  const items = skipped.length === 1 ? "1 item" : `${skipped.length} items`;
  return `${items} not shown: ${parts.join(", ")}.`;
}
