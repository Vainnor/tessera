/// Every error the core returns.
///
/// Variants carry owned, user-safe strings and never an absolute filesystem path, so a message
/// can cross the FFI or IPC boundary and be shown without leaking where the user keeps files.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// A path inside the vault was rejected by [`crate::vault::VaultPath::parse`].
    #[error("invalid vault path: {reason}")]
    InvalidPath {
        /// Why the path was rejected.
        reason: String,
    },
    /// A note id was not 32 lowercase hex characters.
    #[error("invalid note id")]
    InvalidNoteId,
    /// The vault root does not exist or is not a directory.
    #[error("the vault folder could not be opened")]
    VaultUnavailable,
    /// The path is valid but cannot be used as a note: it is hidden or not a `.md` file.
    #[error("not a note: {reason}")]
    NotANote {
        /// Why the path cannot be a note.
        reason: String,
    },
    /// A component of the path inside the vault is a symbolic link. The core never follows one,
    /// so a link cannot point a read or write outside the vault.
    #[error("the path goes through a symbolic link, which Tessera does not follow")]
    SymlinkRefused,
    /// No note exists at the path.
    #[error("note not found")]
    NoteNotFound,
    /// A note already exists at the path.
    #[error("a note already exists there")]
    NoteExists,
    /// The note is larger than [`crate::vault::MAX_NOTE_BYTES`].
    #[error("the note is larger than {limit_bytes} bytes")]
    NoteTooLarge {
        /// The limit that was exceeded.
        limit_bytes: u64,
    },
    /// The note is not valid UTF-8 text.
    #[error("the note is not valid UTF-8 text")]
    NotUtf8,
    /// The note changed on disk since it was read, so writing would discard that change.
    #[error("the note changed on disk since it was opened")]
    Conflict,
    /// A filesystem operation failed for a reason not covered above.
    #[error("could not {operation}: {kind}")]
    Io {
        /// What the core was doing, such as "read the note".
        operation: String,
        /// The OS error category, such as "permission denied". Never the path.
        kind: String,
    },
    /// The feature exists in the API but has not been built yet.
    #[error("not implemented yet: {what}")]
    NotImplemented {
        /// The operation that was called.
        what: String,
    },
}

impl Error {
    /// Maps an I/O error to [`Error::Io`], keeping its category and dropping its text, which can
    /// hold a path.
    pub(crate) fn io(operation: &str, error: &std::io::Error) -> Self {
        Self::Io {
            operation: operation.to_owned(),
            kind: error.kind().to_string(),
        }
    }

    pub(crate) fn not_implemented(what: &str) -> Self {
        Self::NotImplemented {
            what: what.to_owned(),
        }
    }
}
