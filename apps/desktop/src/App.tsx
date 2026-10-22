import { PRESETS, applyTheme, type ThemeChoice } from "@vainnor/tessera-theme";
import { Wordmark } from "@vainnor/tessera-ui";
import { useEffect, useId, useState } from "react";

import { appInfo } from "./lib/ipc.ts";
import { saveChoice, type ChoiceStorage } from "./lib/theme-choice.ts";
import type { AppInfo } from "./lib/types.ts";

interface AppProps {
  initialChoice: ThemeChoice;
  storage: ChoiceStorage | undefined;
}

export function App({ initialChoice, storage }: AppProps) {
  const [choice, setChoice] = useState<ThemeChoice>(initialChoice);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const themeSelectId = useId();

  useEffect(() => {
    let live = true;
    appInfo()
      .then((result) => {
        if (live) setInfo(result);
      })
      // Outside the Tauri window (plain `vite dev`) there is no IPC; the footer stays empty.
      .catch(() => undefined);
    return () => {
      live = false;
    };
  }, []);

  function chooseTheme(next: ThemeChoice) {
    setChoice(next);
    applyTheme(next);
    saveChoice(storage, next);
  }

  return (
    <div className="shell">
      <header className="shell__bar">
        <Wordmark name="tessera" />
        <div className="shell__theme">
          <label htmlFor={themeSelectId}>Theme</label>
          <select
            id={themeSelectId}
            className="t-select"
            value={choice}
            onChange={(event) => chooseTheme(event.target.value)}
          >
            <option value="auto">Follow system</option>
            {PRESETS.map((theme) => (
              <option key={theme.id} value={theme.id}>
                {theme.name}
              </option>
            ))}
          </select>
        </div>
      </header>
      <main className="shell__main">
        <h1>No vault open</h1>
        <p className="t-muted">
          Tessera is in early development. Opening a vault, editing and sync are not built yet.
        </p>
      </main>
      <footer className="shell__footer t-muted">
        {info ? `${info.name} ${info.version} · core ${info.coreVersion}` : null}
      </footer>
    </div>
  );
}
