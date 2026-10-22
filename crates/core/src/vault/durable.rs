//! Crash-safe file replacement: write a temporary file beside the target, flush it to disk, then
//! rename it over the target. A rename within one folder is atomic on every platform Tessera
//! supports, so a reader or a crash sees either the old file or the new one.

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
