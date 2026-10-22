import { Button } from "@vainnor/tessera-ui";
import { useEffect, useId, useRef, useState, type FormEvent, type KeyboardEvent } from "react";

import { listNotes, moveNote, readNote, trashNote } from "./lib/ipc.ts";
import {
  errorMessage,
  isCommandError,
  skippedSummary,
  withMarkdownExtension,
} from "./lib/messages.ts";
import type { Note, OpenedVault } from "./lib/types.ts";

interface VaultViewProps {
  vault: OpenedVault;
  onListingChange: (vault: OpenedVault) => void;
}

/** The open vault: the note list and the selected note's source, with rename and trash. */
export function VaultView({ vault, onListingChange }: VaultViewProps) {
  const [note, setNote] = useState<Note | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [renaming, setRenaming] = useState(false);
  const [draft, setDraft] = useState("");
  const renameInputId = useId();
  const renameButton = useRef<HTMLButtonElement>(null);
  // Only the most recent read may update the view, however fast the user clicks.
  const latestRead = useRef(0);
  const { listing } = vault;
  const skipped = skippedSummary(listing.skipped);

  function reloadNote(path: string): Promise<void> {
    const request = ++latestRead.current;
    return readNote(path).then(
      (result) => {
        if (request === latestRead.current) setNote(result);
      },
      (error: unknown) => {
        if (request !== latestRead.current) return;
        if (isCommandError(error) && error.code === "note-not-found") {
          setNote(null);
          setStatus("That note was moved or deleted outside Tessera.");
          return;
        }
        throw error;
      },
    );
  }

  // The focus handler outlives renders, so it reads the latest props and state from a ref that
  // is updated after every render.
  const latest = useRef({ vault, note, onListingChange, reloadNote });
  useEffect(() => {
    latest.current = { vault, note, onListingChange, reloadNote };
  });

  // Another app may have changed the vault while Tessera was in the background. Refreshing when
  // the window regains focus catches that without any work while idle.
  useEffect(() => {
    let refreshing = false;
    function onFocus() {
      if (refreshing) return;
      refreshing = true;
      const {
        vault: shown,
        note: open,
        onListingChange: update,
        reloadNote: reload,
      } = latest.current;
      const reread = open ? reload(open.path) : Promise.resolve();
      Promise.all([listNotes().then((next) => update({ ...shown, listing: next })), reread])
        .catch((error: unknown) => {
          setStatus(errorMessage(error));
        })
        .finally(() => {
          refreshing = false;
        });
    }
    window.addEventListener("focus", onFocus);
    return () => {
      window.removeEventListener("focus", onFocus);
    };
  }, []);

  function open(path: string) {
    setStatus(null);
    setRenaming(false);
    reloadNote(path).catch((error: unknown) => {
      setStatus(errorMessage(error));
    });
  }

  function refresh(): Promise<void> {
    return listNotes().then((next) => {
      onListingChange({ ...vault, listing: next });
    });
  }

  function startRename() {
    if (!note) return;
    setDraft(note.path);
    setRenaming(true);
  }

  function stopRename() {
    setRenaming(false);
    // Return focus to the control that opened the form (.claude/rules/accessibility.md).
    requestAnimationFrame(() => renameButton.current?.focus());
  }

  function submitRename(event: FormEvent) {
    event.preventDefault();
    if (!note) return;
    const target = withMarkdownExtension(draft);
    setStatus(null);
    moveNote(note.path, target, note.revision)
      .then((moved) => {
        setNote({ ...note, path: moved.path, revision: moved.revision });
        stopRename();
        setStatus(`Renamed to ${moved.path}.`);
        return refresh();
      })
      .catch((error: unknown) => {
        setStatus(errorMessage(error));
      });
  }

  function cancelOnEscape(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      stopRename();
    }
  }

  function trash() {
    if (!note) return;
    setStatus(null);
    trashNote(note.path, note.revision)
      .then((trashedTo) => {
        latestRead.current++;
        setNote(null);
        setStatus(`Moved to ${trashedTo}. Move it back out of .trash to restore it.`);
        return refresh();
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
          <Button
            onClick={() => {
              setStatus(null);
              refresh().catch((error: unknown) => {
                setStatus(errorMessage(error));
              });
            }}
          >
            Refresh
          </Button>
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
            <div className="vault__note-bar">
              <h1 className="vault__title">{note.path}</h1>
              {renaming ? null : (
                <div className="vault__note-actions">
                  <Button ref={renameButton} onClick={startRename}>
                    Rename
                  </Button>
                  <Button onClick={trash}>Move to trash</Button>
                </div>
              )}
            </div>
            {renaming ? (
              <form className="vault__rename" onSubmit={submitRename}>
                <label htmlFor={renameInputId}>New name or path</label>
                <input
                  id={renameInputId}
                  className="t-input"
                  value={draft}
                  // The user just asked for this field, so focus moves into it.
                  autoFocus
                  spellCheck={false}
                  onChange={(event) => {
                    setDraft(event.target.value);
                  }}
                  onKeyDown={cancelOnEscape}
                />
                <Button variant="primary" type="submit">
                  Rename
                </Button>
                <Button onClick={stopRename}>Cancel</Button>
              </form>
            ) : null}
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
