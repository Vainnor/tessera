// @ts-check
import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: [
      "**/dist/**",
      "**/.astro/**",
      "**/.lighthouseci/**",
      "**/node_modules/**",
      "target/**",
      "apps/desktop/src-tauri/**",
    ],
  },
  js.configs.recommended,
  tseslint.configs.strict,
  {
    files: ["**/*.{ts,tsx}"],
    plugins: { "react-hooks": reactHooks },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "react-hooks/exhaustive-deps": "error",
      "@typescript-eslint/consistent-type-imports": "error",
      // The CSP has no 'unsafe-inline', and untrusted HTML never reaches the DOM.
      "no-restricted-syntax": [
        "error",
        {
          selector: "JSXAttribute[name.name='dangerouslySetInnerHTML']",
          message: "No raw HTML: .claude/rules/frontend.md.",
        },
        {
          selector: "JSXAttribute[name.name='style']",
          message: "No inline styles: the CSP blocks them. Use a class and tokens.",
        },
      ],
      "no-eval": "error",
      "no-implied-eval": "error",
      "no-new-func": "error",
    },
  },
  {
    // Only the IPC wrapper talks to Rust (.claude/rules/frontend.md).
    files: ["apps/desktop/src/**/*.{ts,tsx}"],
    ignores: ["apps/desktop/src/lib/ipc.ts"],
    rules: {
      "no-restricted-imports": [
        "error",
        { patterns: [{ group: ["@tauri-apps/*"], message: "Call Rust through src/lib/ipc.ts." }] },
      ],
    },
  },
  {
    files: ["**/test/**", "**/*.test.{ts,tsx}"],
    rules: { "@typescript-eslint/no-non-null-assertion": "off" },
  },
  {
    files: ["apps/web/public/**/*.js"],
    languageOptions: { sourceType: "script", globals: globals.browser },
  },
  {
    files: ["**/scripts/**", "**/*.config.{js,mjs,ts}", "**/test/**"],
    languageOptions: { globals: globals.node },
  },
);
