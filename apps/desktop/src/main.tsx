import "@vainnor/tessera-theme/tessera.css";
import "@vainnor/tessera-ui/base.css";
import "@vainnor/tessera-ui/components.css";
import "./app.css";

import { applyTheme } from "@vainnor/tessera-theme";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App.tsx";
import { loadChoice } from "./lib/theme-choice.ts";

function storage(): Storage | undefined {
  try {
    return window.localStorage;
  } catch {
    return undefined;
  }
}

const initialChoice = loadChoice(storage());
applyTheme(initialChoice);

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <App initialChoice={initialChoice} storage={storage()} />
    </StrictMode>,
  );
}
