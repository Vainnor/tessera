// The only module that talks to the Rust side. One typed function per command; every command
// must also be granted by name in src-tauri/capabilities/main.json.
import { invoke } from "@tauri-apps/api/core";

import type { AppInfo } from "./types.ts";

export function appInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("app_info");
}
