import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// Tauri loads the dev server from devUrl in src-tauri/tauri.conf.json; keep the two in step.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { host: "127.0.0.1", port: 5173, strictPort: true },
  build: {
    // The oldest webviews Tessera supports: Safari 16 (macOS 11+) and WebView2/WebKitGTK.
    target: ["es2022", "safari16", "chrome120"],
    sourcemap: false,
    // The polyfill is unnecessary for these targets and would add code to the entry chunk.
    modulePreload: { polyfill: false },
  },
});
