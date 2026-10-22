// Mirrors of the Rust structs returned by commands in src-tauri/src/commands.rs and dto.rs.
// Change both in the same commit.

export interface AppInfo {
  name: string;
  version: string;
  coreVersion: string;
}

/** What every command rejects with. `message` is safe to show and never holds a path. */
export interface CommandError {
  code: string;
  message: string;
}

export interface NoteEntry {
  path: string;
  sizeBytes: number;
  /** Opaque; pass it back unchanged. */
  revision: string;
}

export type SkipReason = "symlink" | "unsupported-name" | "too-deep" | "unreadable";

export interface SkippedEntry {
  path: string;
  reason: SkipReason;
}

export interface NoteListing {
  notes: NoteEntry[];
  folders: string[];
  skipped: SkippedEntry[];
  truncated: boolean;
}

export interface OpenedVault {
  /** The folder's name only; the full path stays in Rust. */
  name: string;
  listing: NoteListing;
}

export interface Note {
  path: string;
  markdown: string;
  revision: string;
}
