//! Crash-safe file operations.
//!
//! - [`replace`] writes a temporary file beside the target, flushes it, then renames it over the
//!   target. A rename within one folder is atomic on every platform Tessera supports, so a reader
//!   or a crash sees either the old file or the new one.
//! - [`move_no_replace`] gives a file a new name without ever replacing what is already there,
//!   which `std::fs::rename` does not promise on any platform.

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use super::Revision;
use crate::Error;

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

/// Replaces `target` with `contents` and returns the new file's revision.
pub(super) fn replace(target: &Path, contents: &[u8]) -> Result<Revision, Error> {
    let (Some(dir), Some(name)) = (target.parent(), target.file_name()) else {
        return Err(Error::Io {
            operation: "write the note".to_owned(),
            kind: "invalid target".to_owned(),
        });
    };
    // Hidden, so a listing never shows it, and unique per process and call.
    let temp = dir.join(format!(
        ".{}.tessera-{}-{}.tmp",
        name.to_string_lossy(),
        std::process::id(),
        NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
    ));
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

/// Moves `source` to `target`, failing with [`Error::NoteExists`] if anything is at `target`.
///
/// The file is hard-linked to the new name first, which fails atomically if the name is taken,
/// and the old name is removed after. A crash in between leaves the note under both names, never
/// under none. Filesystems without hard links (FAT, exFAT, some network shares) fall back to a
/// check followed by a rename, which a file created in the instant between the two could lose to.
pub(super) fn move_no_replace(source: &Path, target: &Path) -> Result<(), Error> {
    let map_rename = |e: std::io::Error| match e.kind() {
        std::io::ErrorKind::NotFound => Error::NoteNotFound,
        _ => Error::io("move the note", &e),
    };
    if is_case_only_rename(source, target) {
        // On a case-insensitive filesystem the target already "exists": it is the source. Only a
        // different entry with exactly the target's name counts.
        if exact_name_exists(target)? {
            return Err(Error::NoteExists);
        }
        fs::rename(source, target).map_err(map_rename)?;
    } else {
        match fs::hard_link(source, target) {
            Ok(()) => fs::remove_file(source).map_err(|e| Error::io("remove the old name", &e))?,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(Error::NoteExists);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(Error::NoteNotFound),
            Err(_) => {
                if fs::symlink_metadata(target).is_ok() {
                    return Err(Error::NoteExists);
                }
                fs::rename(source, target).map_err(map_rename)?;
            }
        }
    }
    if let Some(dir) = target.parent() {
        os::sync_dir(dir)?;
    }
    match source.parent() {
        Some(dir) if Some(dir) != target.parent() => os::sync_dir(dir),
        _ => Ok(()),
    }
}

fn is_case_only_rename(source: &Path, target: &Path) -> bool {
    source != target
        && source.parent() == target.parent()
        && source
            .file_name()
            .zip(target.file_name())
            .is_some_and(|(a, b)| {
                a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
            })
}

/// Whether the folder holds an entry named exactly like `path`'s last component, compared
/// byte for byte rather than the way the filesystem compares names.
fn exact_name_exists(path: &Path) -> Result<bool, Error> {
    let (Some(dir), Some(name)) = (path.parent(), path.file_name()) else {
        return Ok(false);
    };
    let entries = fs::read_dir(dir).map_err(|e| Error::io("check the folder", &e))?;
    for entry in entries {
        let entry = entry.map_err(|e| Error::io("check the folder", &e))?;
        if entry.file_name() == name {
            return Ok(true);
        }
    }
    Ok(false)
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
