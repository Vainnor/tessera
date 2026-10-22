// The only module that talks to the Rust side. One typed function per command; every command
// must also be granted by name in src-tauri/capabilities/main.json.
import { invoke } from "@tauri-apps/api/core";

import type { AppInfo, Note, NoteListing, OpenedVault } from "./types.ts";

export function appInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("app_info");
}

/** Shows the OS folder picker. Resolves to `null` when the user cancels. */
export function openVault(): Promise<OpenedVault | null> {
  return invoke<OpenedVault | null>("open_vault");
}

export function listNotes(): Promise<NoteListing> {
  return invoke<NoteListing>("list_notes");
}

export function readNote(path: string): Promise<Note> {
  return invoke<Note>("read_note", { path });
}
