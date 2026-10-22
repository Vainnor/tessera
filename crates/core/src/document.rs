//! The document model. Stub: the types the editor, sync and search will share.

use std::fmt;

use crate::Error;
use crate::vault::VaultPath;

/// A note's stable identity, independent of its path, so renames and moves sync as renames.
///
/// 128 bits, written as 32 lowercase hex characters. Ids are random; generating them belongs to
/// the sync layer and is not built yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NoteId(u128);

impl NoteId {
    /// Parses the 32-character lowercase hex form.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidNoteId`] for any other input.
    pub fn parse(hex: &str) -> Result<Self, Error> {
        let well_formed = hex.len() == 32
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !well_formed {
            return Err(Error::InvalidNoteId);
        }
        u128::from_str_radix(hex, 16)
            .map(Self)
            .map_err(|_| Error::InvalidNoteId)
    }
}

impl fmt::Display for NoteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

/// A note as the core hands it to a UI: where it lives and its markdown text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Stable identity.
    pub id: NoteId,
    /// Location inside the vault.
    pub path: VaultPath,
    /// The full markdown source, frontmatter included, exactly as stored on disk.
    pub markdown: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_id_round_trips_through_hex() {
        let hex = "0123456789abcdef0123456789abcdef";
        assert_eq!(NoteId::parse(hex).unwrap().to_string(), hex);
    }

    #[test]
    fn note_id_keeps_leading_zeros() {
        let hex = "00000000000000000000000000000001";
        assert_eq!(NoteId::parse(hex).unwrap().to_string(), hex);
    }

    #[test]
    fn note_id_rejects_uppercase() {
        assert_eq!(
            NoteId::parse("0123456789ABCDEF0123456789ABCDEF"),
            Err(Error::InvalidNoteId)
        );
    }

    #[test]
    fn note_id_rejects_wrong_length() {
        assert_eq!(NoteId::parse("abc"), Err(Error::InvalidNoteId));
    }

    #[test]
    fn note_id_rejects_sign_prefix() {
        assert_eq!(
            NoteId::parse("+123456789abcdef0123456789abcdef"),
            Err(Error::InvalidNoteId)
        );
    }
}
