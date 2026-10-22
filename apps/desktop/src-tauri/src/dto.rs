//! The shapes the UI receives, converted from `tessera-core` types. They mirror
//! `apps/desktop/src/lib/types.ts`; change both together.

use serde::Serialize;
use tessera_core::Error;
use tessera_core::vault::{Note, NoteListing, Revision, SkipReason};

/// An error as the UI sees it: a stable code to branch on and a message safe to show. Core
/// errors never hold a path, so neither does this.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct CommandError {
    pub(crate) code: &'static str,
    message: String,
}

impl CommandError {
    pub(crate) fn new(code: &'static str, message: &str) -> Self {
        Self {
            code,
            message: message.to_owned(),
        }
    }
}

impl From<Error> for CommandError {
    fn from(error: Error) -> Self {
        let code = match &error {
            Error::InvalidPath { .. } => "invalid-path",
            Error::InvalidNoteId => "invalid-note-id",
            Error::VaultUnavailable => "vault-unavailable",
            Error::NotANote { .. } => "not-a-note",
            Error::SymlinkRefused => "symlink-refused",
            Error::NoteNotFound => "note-not-found",
            Error::NoteExists => "note-exists",
            Error::NoteTooLarge { .. } => "note-too-large",
            Error::NotUtf8 => "not-utf8",
            Error::Conflict => "conflict",
            Error::Io { .. } => "io",
            Error::NotImplemented { .. } => "not-implemented",
        };
        Self {
            code,
            message: error.to_string(),
        }
    }
}

/// A revision as an opaque token. A nanosecond timestamp does not fit in a JavaScript number,
/// so it crosses IPC as a string the UI passes back unchanged.
pub(crate) fn revision_token(revision: Revision) -> String {
    match revision.modified_ns {
        Some(ns) => format!("{}:{ns}", revision.size_bytes),
        None => format!("{}:-", revision.size_bytes),
    }
}

/// Parses a token made by [`revision_token`]. The webview hands it back, so it is checked like any
/// other untrusted argument.
pub(crate) fn parse_revision_token(token: &str) -> Result<Revision, CommandError> {
    let invalid = || CommandError::new("invalid-revision", "the note's revision is not valid");
    if token.len() > 64 {
        return Err(invalid());
    }
    let (size, modified) = token.split_once(':').ok_or_else(invalid)?;
    let size_bytes = size.parse::<u64>().map_err(|_| invalid())?;
    let modified_ns = match modified {
        "-" => None,
        digits => Some(digits.parse::<u64>().map_err(|_| invalid())?),
    };
    Ok(Revision {
        size_bytes,
        modified_ns,
    })
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NoteEntryDto {
    path: String,
    size_bytes: u64,
    revision: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkippedDto {
    path: String,
    reason: &'static str,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListingDto {
    notes: Vec<NoteEntryDto>,
    folders: Vec<String>,
    skipped: Vec<SkippedDto>,
    truncated: bool,
}

impl From<NoteListing> for ListingDto {
    fn from(listing: NoteListing) -> Self {
        Self {
            notes: listing
                .notes
                .into_iter()
                .map(|note| NoteEntryDto {
                    path: note.path.as_str().to_owned(),
                    size_bytes: note.revision.size_bytes,
                    revision: revision_token(note.revision),
                })
                .collect(),
            folders: listing
                .folders
                .into_iter()
                .map(|folder| folder.as_str().to_owned())
                .collect(),
            skipped: listing
                .skipped
                .into_iter()
                .map(|skipped| SkippedDto {
                    path: skipped.path,
                    reason: match skipped.reason {
                        SkipReason::Symlink => "symlink",
                        SkipReason::UnsupportedName => "unsupported-name",
                        SkipReason::TooDeep => "too-deep",
                        SkipReason::Unreadable => "unreadable",
                    },
                })
                .collect(),
            truncated: listing.truncated,
        }
    }
}

/// A vault the user just opened. Only the folder's name crosses to the UI, never its full path.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenedVaultDto {
    pub(crate) name: String,
    pub(crate) listing: ListingDto,
    /// Whether the vault was saved to reopen at the next launch.
    pub(crate) remembered: bool,
}

/// A note after a rename or move: where it is now and its revision there.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MovedNoteDto {
    pub(crate) path: String,
    pub(crate) revision: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NoteDto {
    path: String,
    markdown: String,
    revision: String,
}

impl From<Note> for NoteDto {
    fn from(note: Note) -> Self {
        Self {
            path: note.path.as_str().to_owned(),
            markdown: note.markdown,
            revision: revision_token(note.revision),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_token_survives_timestamps_past_javascripts_safe_integers() {
        let revision = Revision {
            size_bytes: 12,
            modified_ns: Some(1_792_000_000_123_456_789),
        };
        assert_eq!(revision_token(revision), "12:1792000000123456789");
    }

    #[test]
    fn revision_token_marks_a_missing_timestamp() {
        let revision = Revision {
            size_bytes: 3,
            modified_ns: None,
        };
        assert_eq!(revision_token(revision), "3:-");
    }

    #[test]
    fn revision_tokens_round_trip() {
        for revision in [
            Revision {
                size_bytes: 12,
                modified_ns: Some(1_792_000_000_123_456_789),
            },
            Revision {
                size_bytes: 0,
                modified_ns: None,
            },
        ] {
            assert_eq!(
                parse_revision_token(&revision_token(revision)),
                Ok(revision)
            );
        }
    }

    #[test]
    fn revision_tokens_from_the_webview_are_validated() {
        for bad in [
            "",
            "12",
            "12:",
            ":5",
            "-1:5",
            "12:abc",
            "12:5:6",
            &"9".repeat(65),
        ] {
            assert_eq!(
                parse_revision_token(bad).unwrap_err().code,
                "invalid-revision",
                "{bad:?}"
            );
        }
    }

    #[test]
    fn core_errors_map_to_stable_codes_without_paths() {
        let error = CommandError::from(Error::Io {
            operation: "read the note".to_owned(),
            kind: "permission denied".to_owned(),
        });
        assert_eq!(
            error,
            CommandError::new("io", "could not read the note: permission denied")
        );
        assert_eq!(
            CommandError::from(Error::SymlinkRefused).code,
            "symlink-refused"
        );
    }

    #[test]
    fn listing_serializes_camel_case_for_the_ui() {
        let listing = ListingDto {
            notes: vec![NoteEntryDto {
                path: "a.md".to_owned(),
                size_bytes: 1,
                revision: "1:2".to_owned(),
            }],
            folders: vec!["notes".to_owned()],
            skipped: vec![SkippedDto {
                path: "link".to_owned(),
                reason: "symlink",
            }],
            truncated: false,
        };
        let json = serde_json::to_value(listing).unwrap();
        assert_eq!(json["notes"][0]["sizeBytes"], 1);
        assert_eq!(json["skipped"][0]["reason"], "symlink");
        assert_eq!(json["truncated"], false);
    }
}
