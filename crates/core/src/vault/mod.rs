//! The vault: a folder of markdown files laid out the way Obsidian lays them out.
//!
//! The files on disk are the user's data. The core reads and writes them; it never moves the
//! source of truth into a database. Other programs (Obsidian, a text editor, a sync client) may
//! change the same files at any time, so every write checks that the note has not changed since
//! it was read.
//!
//! Three rules keep a vault operation inside the vault:
//!
//! - Every path from outside the core is a [`VaultPath`], so it is relative and has no `..`.
//! - No symbolic link inside the vault is ever followed. The root itself may be a link, because
//!   the user chose it.
//! - Hidden entries (`.obsidian`, `.git`, `.trash`, editor temp files) are not notes: listing skips
//!   them and note operations refuse them.

mod durable;
mod path;

use std::fs::{self, File};
use std::io::{ErrorKind, Read, Write};
use std::path::PathBuf;
use std::time::UNIX_EPOCH;

pub use path::{MAX_COMPONENT_BYTES, MAX_PATH_BYTES, VaultPath};

use crate::Error;

/// The largest note the core reads or writes. Far above any hand-written note, low enough that a
/// stray multi-gigabyte `.md` file cannot exhaust memory.
pub const MAX_NOTE_BYTES: u64 = 16 * 1024 * 1024;

/// The most entries (notes, folders and skipped entries) a listing returns before it stops and
/// sets [`NoteListing::truncated`].
pub const MAX_LISTED_ENTRIES: usize = 200_000;

/// The deepest folder nesting a listing descends into.
pub const MAX_DEPTH: usize = 64;

/// A note's state on disk when it was read or written: its size and modification time.
///
/// Pass it back to [`Vault::write_note`] so the write fails with [`Error::Conflict`] instead of
/// discarding a change another program made in the meantime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revision {
    /// File size in bytes.
    pub size_bytes: u64,
    /// Modification time in nanoseconds since the Unix epoch, when the filesystem reports one.
    pub modified_ns: Option<u64>,
}

impl Revision {
    fn of(metadata: &fs::Metadata) -> Self {
        let modified_ns = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .and_then(|elapsed| u64::try_from(elapsed.as_nanos()).ok());
        Self {
            size_bytes: metadata.len(),
            modified_ns,
        }
    }
}

/// A note in a listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteEntry {
    /// Location inside the vault.
    pub path: VaultPath,
    /// Size and modification time when the vault was listed.
    pub revision: Revision,
}

/// Why a listing left an entry out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    /// A symbolic link. The core does not follow links inside a vault.
    Symlink,
    /// A name that is not UTF-8 or that some platform cannot store (see [`VaultPath::parse`]).
    UnsupportedName,
    /// A folder nested deeper than [`MAX_DEPTH`].
    TooDeep,
    /// A folder the OS would not let the core read.
    Unreadable,
}

/// An entry the listing left out, so the UI can tell the user instead of hiding it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedEntry {
    /// The entry's path relative to the vault root, with unreadable bytes replaced. Relative, so
    /// it never reveals where the vault lives.
    pub path: String,
    /// Why it was left out.
    pub reason: SkipReason,
}

/// What [`Vault::list_notes`] found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NoteListing {
    /// Markdown notes, sorted by path.
    pub notes: Vec<NoteEntry>,
    /// Folders that are not hidden, sorted by path, including empty ones.
    pub folders: Vec<VaultPath>,
    /// Entries left out, sorted by path.
    pub skipped: Vec<SkippedEntry>,
    /// The listing stopped at [`MAX_LISTED_ENTRIES`].
    pub truncated: bool,
}

/// A note read from disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// Location inside the vault.
    pub path: VaultPath,
    /// The full markdown source, frontmatter included, exactly as stored.
    pub markdown: String,
    /// The state the note was read in; pass it to [`Vault::write_note`].
    pub revision: Revision,
}

/// An open vault.
#[derive(Debug)]
pub struct Vault {
    root: PathBuf,
}

impl Vault {
    /// Opens the vault rooted at `root`, an absolute path to an existing folder.
    ///
    /// # Errors
    ///
    /// [`Error::VaultUnavailable`] when `root` is not an existing directory.
    pub fn open(root: &str) -> Result<Self, Error> {
        let root = fs::canonicalize(root).map_err(|_| Error::VaultUnavailable)?;
        if !root.is_dir() {
            return Err(Error::VaultUnavailable);
        }
        Ok(Self { root })
    }

    /// Lists the vault's markdown notes and folders.
    ///
    /// Hidden entries are left out silently. Symbolic links, unsupported names, folders deeper than
    /// [`MAX_DEPTH`] and unreadable folders are left out and reported in
    /// [`NoteListing::skipped`]. Files that are not markdown are not notes and are not listed.
    ///
    /// # Errors
    ///
    /// [`Error::VaultUnavailable`] when the vault root itself cannot be read.
    pub fn list_notes(&self) -> Result<NoteListing, Error> {
        let mut listing = NoteListing::default();
        let mut pending = vec![(self.root.clone(), String::new(), 0_usize)];
        let mut entries = 0_usize;

        while let Some((dir, relative, depth)) = pending.pop() {
            let read = match fs::read_dir(&dir) {
                Ok(read) => read,
                Err(_) if relative.is_empty() => return Err(Error::VaultUnavailable),
                Err(_) => {
                    listing.skipped.push(SkippedEntry {
                        path: relative,
                        reason: SkipReason::Unreadable,
                    });
                    continue;
                }
            };
            for entry in read {
                if entries >= MAX_LISTED_ENTRIES {
                    listing.truncated = true;
                    break;
                }
                let Ok(entry) = entry else {
                    listing.skipped.push(SkippedEntry {
                        path: relative.clone(),
                        reason: SkipReason::Unreadable,
                    });
                    continue;
                };
                let name = entry.file_name();
                let name_lossy = name.to_string_lossy();
                if name_lossy.starts_with('.') {
                    continue;
                }
                entries += 1;
                let child = if relative.is_empty() {
                    name_lossy.into_owned()
                } else {
                    format!("{relative}/{name_lossy}")
                };
                let skip = |reason| SkippedEntry {
                    path: child.clone(),
                    reason,
                };
                // `DirEntry::file_type` does not follow links, so a link is seen as a link.
                let Ok(file_type) = entry.file_type() else {
                    listing.skipped.push(skip(SkipReason::Unreadable));
                    continue;
                };
                if file_type.is_symlink() {
                    listing.skipped.push(skip(SkipReason::Symlink));
                    continue;
                }
                // A non-UTF-8 name was made lossy above, so it must not reach `VaultPath`.
                let Some(Ok(path)) = name.to_str().map(|_| VaultPath::parse(&child)) else {
                    listing.skipped.push(skip(SkipReason::UnsupportedName));
                    continue;
                };
                if file_type.is_dir() {
                    if depth + 1 >= MAX_DEPTH {
                        listing.skipped.push(skip(SkipReason::TooDeep));
                        continue;
                    }
                    pending.push((entry.path(), child, depth + 1));
                    listing.folders.push(path);
                } else if file_type.is_file() && path.is_markdown() {
                    match entry.metadata() {
                        Ok(metadata) => listing.notes.push(NoteEntry {
                            path,
                            revision: Revision::of(&metadata),
                        }),
                        Err(_) => listing.skipped.push(skip(SkipReason::Unreadable)),
                    }
                }
            }
            if listing.truncated {
                break;
            }
        }

        listing.notes.sort_by(|a, b| a.path.cmp(&b.path));
        listing.folders.sort();
        listing.skipped.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(listing)
    }

    /// Reads the note at `path`.
    ///
    /// # Errors
    ///
    /// [`Error::NotANote`] for a hidden or non-markdown path, [`Error::SymlinkRefused`] when the
    /// path goes through a link, [`Error::NoteNotFound`], [`Error::NoteTooLarge`],
    /// [`Error::NotUtf8`], or [`Error::Io`].
    pub fn read_note(&self, path: &VaultPath) -> Result<Note, Error> {
        let full = self.resolve_note(path)?;
        // Windows refuses to open a folder as a file ("permission denied"), so check the type
        // first to report a folder named like a note the same way on every platform.
        match fs::symlink_metadata(&full) {
            Ok(metadata) if !metadata.is_file() => return Err(Error::NoteNotFound),
            Ok(_) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => return Err(Error::NoteNotFound),
            Err(e) => return Err(Error::io("check the note", &e)),
        }
        let file = File::open(&full).map_err(|e| match e.kind() {
            ErrorKind::NotFound => Error::NoteNotFound,
            _ => Error::io("open the note", &e),
        })?;
        let metadata = file
            .metadata()
            .map_err(|e| Error::io("read the note", &e))?;
        if !metadata.is_file() {
            return Err(Error::NoteNotFound);
        }
        if metadata.len() > MAX_NOTE_BYTES {
            return Err(Error::NoteTooLarge {
                limit_bytes: MAX_NOTE_BYTES,
            });
        }
        // Read at most one byte past the limit, in case the file grew after the size check.
        let mut bytes = Vec::new();
        file.take(MAX_NOTE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| Error::io("read the note", &e))?;
        if bytes.len() as u64 > MAX_NOTE_BYTES {
            return Err(Error::NoteTooLarge {
                limit_bytes: MAX_NOTE_BYTES,
            });
        }
        let markdown = String::from_utf8(bytes).map_err(|_| Error::NotUtf8)?;
        Ok(Note {
            path: path.clone(),
            markdown,
            revision: Revision::of(&metadata),
        })
    }

    /// Creates a new note at `path`, creating missing folders. Never replaces an existing file.
    ///
    /// # Errors
    ///
    /// [`Error::NoteExists`] when anything already exists at `path`, plus the path and size errors
    /// of [`Vault::write_note`].
    pub fn create_note(&self, path: &VaultPath, markdown: &str) -> Result<Revision, Error> {
        let full = self.resolve_note(path)?;
        check_size(markdown)?;
        self.create_parent_folders(path)?;
        // `create_new` below is what guarantees no replacement. This check only makes a folder in
        // the way report `NoteExists` on Windows too, where creating over one is "permission denied".
        if fs::symlink_metadata(&full).is_ok() {
            return Err(Error::NoteExists);
        }
        let mut file = File::options()
            .write(true)
            .create_new(true)
            .open(&full)
            .map_err(|e| match e.kind() {
                ErrorKind::AlreadyExists => Error::NoteExists,
                _ => Error::io("create the note", &e),
            })?;
        file.write_all(markdown.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|e| Error::io("write the note", &e))?;
        let metadata = file
            .metadata()
            .map_err(|e| Error::io("write the note", &e))?;
        Ok(Revision::of(&metadata))
    }

    /// Replaces the note at `path` with `markdown`, atomically: readers see the old note or the
    /// new one, never a mix, and a crash leaves the old note in place.
    ///
    /// `expected` is the revision the caller last read or wrote. If the note on disk no longer
    /// matches it, another program changed the note and the write is refused.
    ///
    /// # Errors
    ///
    /// [`Error::Conflict`] when the note changed on disk, [`Error::NoteNotFound`] when it was
    /// deleted, [`Error::NotANote`], [`Error::SymlinkRefused`], [`Error::NoteTooLarge`], or
    /// [`Error::Io`].
    pub fn write_note(
        &self,
        path: &VaultPath,
        markdown: &str,
        expected: Revision,
    ) -> Result<Revision, Error> {
        let full = self.resolve_note(path)?;
        check_size(markdown)?;
        check_unchanged(&full, expected)?;
        durable::replace(&full, markdown.as_bytes())
    }

    /// Renames or moves the note at `from` to `to`, creating missing folders. Never replaces
    /// anything already at `to`, and refuses if the note changed on disk since `expected`.
    /// A rename that only changes letter case works on case-insensitive filesystems too.
    ///
    /// # Errors
    ///
    /// [`Error::NoteExists`] when something is already at `to`, [`Error::Conflict`],
    /// [`Error::NoteNotFound`], [`Error::NotANote`] or [`Error::SymlinkRefused`] for either
    /// path, or [`Error::Io`].
    pub fn move_note(
        &self,
        from: &VaultPath,
        to: &VaultPath,
        expected: Revision,
    ) -> Result<Revision, Error> {
        let source = self.resolve_note(from)?;
        let target = self.resolve_note(to)?;
        check_unchanged(&source, expected)?;
        if from == to {
            return Ok(expected);
        }
        self.create_parent_folders(to)?;
        durable::move_no_replace(&source, &target)?;
        let metadata =
            fs::symlink_metadata(&target).map_err(|e| Error::io("check the note", &e))?;
        Ok(Revision::of(&metadata))
    }

    /// Moves the note at `path` into the vault's `.trash` folder, keeping its folder structure so
    /// it can be put back, as Obsidian does. A name already in the trash gets a number:
    /// `.trash/notes/plan (2).md`. Refuses if the note changed on disk since `expected`.
    ///
    /// Returns where the note now is.
    ///
    /// # Errors
    ///
    /// [`Error::Conflict`], [`Error::NoteNotFound`], [`Error::NotANote`],
    /// [`Error::SymlinkRefused`] (including a `.trash` that is a link), or [`Error::Io`].
    pub fn trash_note(&self, path: &VaultPath, expected: Revision) -> Result<VaultPath, Error> {
        let source = self.resolve_note(path)?;
        check_unchanged(&source, expected)?;
        for attempt in 1..=MAX_TRASH_NAMES {
            let destination = trash_destination(path, attempt)?;
            let target = self.resolve_inside(&destination)?;
            self.create_parent_folders(&destination)?;
            match durable::move_no_replace(&source, &target) {
                Ok(()) => return Ok(destination),
                Err(Error::NoteExists) => {}
                Err(error) => return Err(error),
            }
        }
        Err(Error::Io {
            operation: "move the note to the trash".to_owned(),
            kind: "too many notes with that name are already in the trash".to_owned(),
        })
    }

    /// Checks that `path` can name a note and that nothing on the way to it is a symbolic link,
    /// then returns its location on disk. Components that do not exist yet are fine.
    fn resolve_note(&self, path: &VaultPath) -> Result<PathBuf, Error> {
        if path.is_hidden() {
            return Err(Error::NotANote {
                reason: "hidden path".to_owned(),
            });
        }
        if !path.is_markdown() {
            return Err(Error::NotANote {
                reason: "not a .md file".to_owned(),
            });
        }
        self.resolve_inside(path)
    }

    /// Returns where `path` is on disk, refusing a path that goes through a symbolic link.
    /// Components that do not exist yet are fine.
    fn resolve_inside(&self, path: &VaultPath) -> Result<PathBuf, Error> {
        let components: Vec<&str> = path.components().collect();
        let mut full = self.root.clone();
        for (index, component) in components.iter().enumerate() {
            full.push(component);
            match fs::symlink_metadata(&full) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(Error::SymlinkRefused);
                }
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::NotFound => {
                    // The rest of the path does not exist, so it cannot contain a link either.
                    full.extend(components.iter().skip(index + 1));
                    return Ok(full);
                }
                Err(e) => return Err(Error::io("check the path", &e)),
            }
        }
        Ok(full)
    }

    fn create_parent_folders(&self, path: &VaultPath) -> Result<(), Error> {
        let mut dir = self.root.clone();
        let components: Vec<&str> = path.components().collect();
        let Some((_, folders)) = components.split_last() else {
            return Ok(());
        };
        for folder in folders {
            dir.push(folder);
            match fs::create_dir(&dir) {
                Ok(()) => {}
                Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                    // `resolve_note` ran first, but a link could have appeared since.
                    let metadata =
                        fs::symlink_metadata(&dir).map_err(|e| Error::io("create a folder", &e))?;
                    if metadata.file_type().is_symlink() {
                        return Err(Error::SymlinkRefused);
                    }
                    if !metadata.is_dir() {
                        return Err(Error::Io {
                            operation: "create a folder".to_owned(),
                            kind: "a file is in the way".to_owned(),
                        });
                    }
                }
                Err(e) => return Err(Error::io("create a folder", &e)),
            }
        }
        Ok(())
    }
}

/// The most numbered names `trash_note` tries for one note before giving up.
const MAX_TRASH_NAMES: u32 = 1000;

/// Fails unless a regular file is at `full` in the state `expected` describes.
fn check_unchanged(full: &std::path::Path, expected: Revision) -> Result<(), Error> {
    let current = fs::symlink_metadata(full).map_err(|e| match e.kind() {
        ErrorKind::NotFound => Error::NoteNotFound,
        _ => Error::io("check the note", &e),
    })?;
    if !current.is_file() {
        return Err(Error::NoteNotFound);
    }
    if Revision::of(&current) != expected {
        return Err(Error::Conflict);
    }
    Ok(())
}

/// `.trash/<path>` for the first attempt, then `.trash/<folders>/<stem> (n).md`.
fn trash_destination(path: &VaultPath, attempt: u32) -> Result<VaultPath, Error> {
    let raw = path.as_str();
    if attempt == 1 {
        return VaultPath::parse(&format!(".trash/{raw}"));
    }
    // A note path always ends in `.md` (any case), checked by `resolve_note`.
    let (stem, extension) = raw.rsplit_once('.').unwrap_or((raw, "md"));
    VaultPath::parse(&format!(".trash/{stem} ({attempt}).{extension}"))
}

fn check_size(markdown: &str) -> Result<(), Error> {
    if markdown.len() as u64 > MAX_NOTE_BYTES {
        return Err(Error::NoteTooLarge {
            limit_bytes: MAX_NOTE_BYTES,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
