import { Button } from "@vainnor/tessera-ui";
import { useRef, useState } from "react";

import { listNotes, readNote } from "./lib/ipc.ts";
import { errorMessage, skippedSummary } from "./lib/messages.ts";
import type { Note, OpenedVault } from "./lib/types.ts";

interface VaultViewProps {
  vault: OpenedVault;
  onListingChange: (vault: OpenedVault) => void;
}

/** The open vault: the note list and the selected note's source. Read-only for now. */
export function VaultView({ vault, onListingChange }: VaultViewProps) {
  const [note, setNote] = useState<Note | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  // Only the most recent read may update the view, however fast the user clicks.
  const latestRead = useRef(0);
  const { listing } = vault;
  const skipped = skippedSummary(listing.skipped);

  function open(path: string) {
    const request = ++latestRead.current;
    setStatus(null);
    readNote(path)
      .then((result) => {
        if (request === latestRead.current) setNote(result);
      })
      .catch((error: unknown) => {
        if (request === latestRead.current) setStatus(errorMessage(error));
      });
  }

  function refresh() {
    setStatus(null);
    listNotes()
      .then((next) => {
        onListingChange({ ...vault, listing: next });
      })
      .catch((error: unknown) => {
        setStatus(errorMessage(error));
      });
  }

  return (
    <div className="vault">
      <nav className="vault__list" aria-label="Notes">
        <div className="vault__heading">
          <h2>{vault.name}</h2>
          <Button onClick={refresh}>Refresh</Button>
        </div>
        {listing.notes.length === 0 ? (
          <p className="t-muted">This folder has no markdown notes.</p>
        ) : (
          <ul>
            {listing.notes.map((entry) => (
              <li key={entry.path}>
                <button
                  type="button"
                  className="vault__note"
                  aria-current={note?.path === entry.path ? "page" : undefined}
                  onClick={() => {
                    open(entry.path);
                  }}
                >
                  {entry.path}
                </button>
              </li>
            ))}
          </ul>
        )}
        {skipped ? <p className="t-muted vault__notice">{skipped}</p> : null}
        {listing.truncated ? (
          <p className="t-muted vault__notice">
            This vault is very large; only the first 200,000 items are listed.
          </p>
        ) : null}
      </nav>
      <section className="vault__note-view" aria-label="Note">
        <p className="vault__status" role="status">
          {status}
        </p>
        {note ? (
          <>
            <h1 className="vault__title">{note.path}</h1>
            <p className="t-muted">Showing the markdown source. Editing is not built yet.</p>
            <pre className="vault__source">{note.markdown}</pre>
          </>
        ) : (
          <p className="t-muted">Choose a note to read it.</p>
        )}
      </section>
    </div>
  );
}
