import { PRESETS, applyTheme, type ThemeChoice } from "@vainnor/tessera-theme";
import { Button, Wordmark } from "@vainnor/tessera-ui";
import { useEffect, useId, useState } from "react";

import { appInfo, openVault } from "./lib/ipc.ts";
import { errorMessage } from "./lib/messages.ts";
import { saveChoice, type ChoiceStorage } from "./lib/theme-choice.ts";
import type { AppInfo, OpenedVault } from "./lib/types.ts";
import { VaultView } from "./VaultView.tsx";

interface AppProps {
  initialChoice: ThemeChoice;
  storage: ChoiceStorage | undefined;
}

export function App({ initialChoice, storage }: AppProps) {
  const [choice, setChoice] = useState<ThemeChoice>(initialChoice);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [vault, setVault] = useState<OpenedVault | null>(null);
  const [openError, setOpenError] = useState<string | null>(null);
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

  function pickVault() {
    setOpenError(null);
    openVault()
      .then((opened) => {
        // `null` means the user cancelled the picker; keep whatever was open.
        if (opened) setVault(opened);
      })
      .catch((error: unknown) => {
        setOpenError(errorMessage(error));
      });
  }

  function chooseTheme(next: ThemeChoice) {
    setChoice(next);
    applyTheme(next);
    saveChoice(storage, next);
  }

  return (
    <div className="shell">
      <header className="shell__bar">
        <Wordmark name="tessera" />
        <div className="shell__actions">
          {vault ? <Button onClick={pickVault}>Open another vault</Button> : null}
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
        </div>
      </header>
      {vault ? (
        <main className="shell__vault">
          {openError ? (
            <p className="shell__error" role="status">
              {openError}
            </p>
          ) : null}
          <VaultView vault={vault} onListingChange={setVault} />
        </main>
      ) : (
        <main className="shell__main">
          <h1>No vault open</h1>
          <p className="t-muted">
            A vault is a folder of markdown notes, such as an Obsidian vault. Tessera can list and
            read notes; editing and sync are not built yet.
          </p>
          <p>
            <Button variant="primary" onClick={pickVault}>
              Open a vault folder
            </Button>
          </p>
          <p className="shell__error" role="status">
            {openError}
          </p>
        </main>
      )}
      <footer className="shell__footer t-muted">
        {info ? `${info.name} ${info.version} · core ${info.coreVersion}` : null}
      </footer>
    </div>
  );
}
