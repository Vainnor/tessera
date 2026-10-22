//! Crash-safe file operations.
//!
//! - [`replace`] writes a temporary file beside the target, flushes it, then renames it over the
//!   target. A rename within one folder is atomic on every platform Tessera supports, so a reader
//!   or a crash sees either the old file or the new one.
//! - [`move_no_replace`] gives a file a new name without ever replacing what is already there,
//!   which `std::fs::rename` does not promise on any platform.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::Revision;
use crate::Error;

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

/// Replaces `target` with `contents` and returns the new file's revision.
pub(super) fn replace(target: &Path, contents: &[u8]) -> Result<Revision, Error> {
    let Some(dir) = target.parent() else {
        return Err(Error::Io {
            operation: "write the note".to_owned(),
            kind: "invalid target".to_owned(),
        });
    };
    let temp = temp_beside(target)?;
    let result = write_new(&temp, contents)
        .and_then(|()| fs::rename(&temp, target).map_err(|e| Error::io("replace the note", &e)));
    if let Err(error) = result {
        // Report the write failure. A leftover temp file is hidden from listings and harmless,
        // so a failed cleanup must not replace the error that matters.
        let _cleanup = fs::remove_file(&temp);
        return Err(error);
    }
    os::sync_dir(dir)?;
    let metadata = fs::symlink_metadata(target).map_err(|e| Error::io("check the note", &e))?;
    Ok(Revision::of(&metadata))
}

/// Moves `source` to `target`, failing with [`Error::NoteExists`] if another file is at `target`.
///
/// Each step is a hard link, which fails atomically when the name is taken, followed by removing
/// the old name; a crash in between leaves the note under two names, never under none. Whether
/// two names are the same is left to the filesystem and never decided by comparing strings,
/// because filesystems fold letter case differently (NTFS keeps the Kelvin sign apart from `K`).
///
/// When the target is reported taken, the name may belong to the note itself: a rename that only
/// changes letter case on a case-insensitive filesystem. The note then steps aside to a hidden
/// temporary name and the link is tried again. If the target is still taken, another file holds
/// it, and the note goes back to its old name.
///
/// Filesystems without hard links (FAT, exFAT, some network shares) fall back to a check followed
/// by a rename, which a file created in the instant between the two could lose to.
pub(super) fn move_no_replace(source: &Path, target: &Path) -> Result<(), Error> {
    match link_then_unlink(source, target) {
        Ok(()) => {}
        Err(Link::Taken) => move_via_temp(source, target)?,
        Err(Link::Unsupported) => {
            if fs::symlink_metadata(target).is_ok() {
                return Err(Error::NoteExists);
            }
            fs::rename(source, target).map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => Error::NoteNotFound,
                _ => Error::io("move the note", &e),
            })?;
        }
        Err(Link::Failed(error)) => return Err(error),
    }
    if let Some(dir) = target.parent() {
        os::sync_dir(dir)?;
    }
    match source.parent() {
        Some(dir) if Some(dir) != target.parent() => os::sync_dir(dir),
        _ => Ok(()),
    }
}

/// Why a hard-link step did not happen.
enum Link {
    /// Something already has the target name.
    Taken,
    /// The filesystem has no hard links.
    Unsupported,
    Failed(Error),
}

fn link_then_unlink(from: &Path, to: &Path) -> Result<(), Link> {
    match fs::hard_link(from, to) {
        Ok(()) => {
            fs::remove_file(from).map_err(|e| Link::Failed(Error::io("remove the old name", &e)))
        }
        Err(e) => Err(match e.kind() {
            std::io::ErrorKind::AlreadyExists => Link::Taken,
            std::io::ErrorKind::NotFound => Link::Failed(Error::NoteNotFound),
            // exFAT and FAT report one or the other.
            std::io::ErrorKind::Unsupported | std::io::ErrorKind::PermissionDenied => {
                Link::Unsupported
            }
            _ => Link::Failed(Error::io("move the note", &e)),
        }),
    }
}

/// The second attempt of [`move_no_replace`]: step aside to a hidden name, then link again.
fn move_via_temp(source: &Path, target: &Path) -> Result<(), Error> {
    let temp = temp_beside(source)?;
    link_then_unlink(source, &temp).map_err(|step| match step {
        Link::Failed(error) => error,
        Link::Taken | Link::Unsupported => Error::Io {
            operation: "move the note".to_owned(),
            kind: "a temporary name was unavailable".to_owned(),
        },
    })?;
    let Err(step) = link_then_unlink(&temp, target) else {
        return Ok(());
    };
    // Another file holds the target. Put the note back under its old name before reporting.
    link_then_unlink(&temp, source).map_err(|_| Error::Io {
        operation: "put the note back".to_owned(),
        kind: "it was left under a hidden temporary name in its folder".to_owned(),
    })?;
    Err(match step {
        Link::Taken => Error::NoteExists,
        Link::Unsupported => Error::Io {
            operation: "move the note".to_owned(),
            kind: "the filesystem refused".to_owned(),
        },
        Link::Failed(error) => error,
    })
}

/// A hidden name beside `path`, unique per process and call, so a listing never shows it.
fn temp_beside(path: &Path) -> Result<PathBuf, Error> {
    let (Some(dir), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(Error::Io {
            operation: "write the note".to_owned(),
            kind: "invalid target".to_owned(),
        });
    };
    Ok(dir.join(format!(
        ".{}.tessera-{}-{}.tmp",
        name.to_string_lossy(),
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    )))
}

fn write_new(path: &Path, contents: &[u8]) -> Result<(), Error> {
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| Error::io("create a temporary file", &e))?;
    file.write_all(contents)
        .and_then(|()| file.sync_all())
        .map_err(|e| Error::io("write a temporary file", &e))
}

#[cfg(unix)]
mod os {
    use std::fs::File;
    use std::path::Path;

    use crate::Error;

    /// Flushes the folder entry so the rename itself survives a power loss.
    pub(super) fn sync_dir(dir: &Path) -> Result<(), Error> {
        File::open(dir)
            .and_then(|handle| handle.sync_all())
            .map_err(|e| Error::io("flush the folder", &e))
    }
}

#[cfg(not(unix))]
mod os {
    use std::path::Path;

    use crate::Error;

    /// Windows cannot open a folder as a file to flush it; NTFS journals the rename itself.
    #[allow(clippy::unnecessary_wraps, reason = "matches the Unix signature")]
    pub(super) fn sync_dir(_dir: &Path) -> Result<(), Error> {
        Ok(())
    }
}
