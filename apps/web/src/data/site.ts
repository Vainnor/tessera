export const REPO_URL = "https://github.com/Vainnor/tessera";
export const RELEASES_URL = `${REPO_URL}/releases`;
export const DOCS_URL = `${REPO_URL}/tree/main/docs`;

/** "available": in a release. "in-progress": exists in the source, not released. "planned": not built. */
export type Status = "available" | "in-progress" | "planned";

export interface Feature {
  title: string;
  description: string;
  status: Status;
}

/**
 * The home page feature list. Every status must be true of the code today
 * (.claude/rules/web.md → "Claims"); test/features.test.ts holds the list to that.
 */
export const FEATURES: readonly Feature[] = [
  {
    title: "Local-first",
    description: "Your notes live on your devices and work without a server or an account.",
    status: "planned",
  },
  {
    title: "Plain markdown files",
    description:
      "A vault is a folder of .md files with wikilinks and frontmatter, laid out like Obsidian's.",
    status: "planned",
  },
  {
    title: "Real-time sync",
    description:
      "Edits merge across your devices as you type, through a server you can host yourself.",
    status: "planned",
  },
  {
    title: "End-to-end encrypted",
    description:
      "The sync server only ever stores ciphertext. The design is published for review before any code.",
    status: "planned",
  },
  {
    title: "Works offline",
    description: "Everything works with no connection; changes sync when you reconnect.",
    status: "planned",
  },
  {
    title: "Drawing and handwriting",
    description:
      "Sketches, diagrams, handwriting-to-text and PDF annotation, with a native iPad app later.",
    status: "planned",
  },
  {
    title: "Themes",
    description:
      "Five built-in themes, including high contrast, and your own themes as small JSON files. This site already uses them.",
    status: "in-progress",
  },
  {
    title: "Low resource use",
    description:
      "Budgets for memory, startup time and download size are set before features are built.",
    status: "planned",
  },
  {
    title: "Open source",
    description: "Licensed under the GNU AGPL v3 or later. No telemetry, analytics or tracking.",
    status: "available",
  },
];

export const STATUS_LABEL: Record<Status, string> = {
  available: "available",
  "in-progress": "in progress",
  planned: "planned",
};
