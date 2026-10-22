// The only module that talks to the Rust side. One typed function per command; every command
// must also be granted by name in src-tauri/capabilities/main.json.
import { invoke } from "@tauri-apps/api/core";

import type { AppInfo, MovedNote, Note, NoteListing, OpenedVault } from "./types.ts";

export function appInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("app_info");
}

/** Shows the OS folder picker. Resolves to `null` when the user cancels. */
export function openVault(): Promise<OpenedVault | null> {
  return invoke<OpenedVault | null>("open_vault");
}

/** Reopens the vault from the last launch. Resolves to `null` when there is none to reopen. */
export function restoreVault(): Promise<OpenedVault | null> {
  return invoke<OpenedVault | null>("restore_vault");
}

export function listNotes(): Promise<NoteListing> {
  return invoke<NoteListing>("list_notes");
}

export function readNote(path: string): Promise<Note> {
  return invoke<Note>("read_note", { path });
}

/** Renames or moves a note. Never replaces another note; fails if it changed on disk. */
export function moveNote(from: string, to: string, revision: string): Promise<MovedNote> {
  return invoke<MovedNote>("move_note", { from, to, revision });
}

/** Moves a note into the vault's `.trash` folder. Resolves to where it went. */
export function trashNote(path: string, revision: string): Promise<string> {
  return invoke<string>("trash_note", { path, revision });
}
