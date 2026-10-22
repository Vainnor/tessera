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
    /// The feature exists in the API but has not been built yet.
    #[error("not implemented yet: {what}")]
    NotImplemented {
        /// The operation that was called.
        what: String,
    },
}

impl Error {
    pub(crate) fn not_implemented(what: &str) -> Self {
        Self::NotImplemented {
            what: what.to_owned(),
        }
    }
}
