//! Sync. The per-note edit history exists ([`NoteHistory`]); transport and encryption do not.
//!
//! Sync merges edits from several devices with Automerge and will move only ciphertext through
//! the server (`docs/adr/0002-end-to-end-encrypted-sync.md`, `docs/adr/0003-crdt-library.md`).

use std::fmt;

use crate::Error;
use crate::document::parse_hex128;

mod history;

pub use history::{
    Applied, MAX_CHANGE_BYTES, MAX_CHANGE_DEPS, MAX_CHANGE_OPS, MAX_HISTORY_AUTHORS,
    MAX_HISTORY_CHANGES, MAX_HISTORY_OPS, MAX_PENDING_BYTES, MAX_PENDING_CHANGES,
    MAX_PENDING_PER_AUTHOR, MAX_SAVED_BYTES, NoteHistory, RefusedChange,
};

/// A device's stable identity. Every change a device makes carries it, and a change is accepted
/// only from the device whose id it carries.
///
/// 128 bits, written as 32 lowercase hex characters. Generating ids belongs to device enrollment,
/// which is not built yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeviceId(u128);

impl DeviceId {
    /// Parses the 32-character lowercase hex form.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidDeviceId`] for any other input.
    pub fn parse(hex: &str) -> Result<Self, Error> {
        parse_hex128(hex).map(Self).ok_or(Error::InvalidDeviceId)
    }

    fn to_bytes(self) -> [u8; 16] {
        self.0.to_be_bytes()
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

/// What the UI shows about sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// Sync is not configured, or the device is offline. Everything still works locally.
    Offline,
    /// Connecting to the sync server.
    Connecting,
    /// Every local change has reached the server and every remote change has been applied.
    UpToDate,
    /// Changes are waiting to be sent or applied.
    Pending,
}

/// A sync engine as clients see it. Implementations hold their state behind interior
/// mutability so one instance can be shared across threads and, later, across the FFI.
pub trait SyncEngine: Send + Sync {
    /// The current status.
    fn status(&self) -> SyncStatus;
}

/// The engine every vault uses until sync exists: local-only, always [`SyncStatus::Offline`].
#[derive(Debug, Default)]
pub struct LocalOnly;

impl SyncEngine for LocalOnly {
    fn status(&self) -> SyncStatus {
        SyncStatus::Offline
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_only_engine_reports_offline() {
        assert_eq!(LocalOnly.status(), SyncStatus::Offline);
    }

    #[test]
    fn device_id_round_trips_through_hex() {
        let hex = "00112233445566778899aabbccddeeff";
        assert_eq!(DeviceId::parse(hex).unwrap().to_string(), hex);
    }

    #[test]
    fn device_id_rejects_uppercase() {
        assert_eq!(
            DeviceId::parse("00112233445566778899AABBCCDDEEFF"),
            Err(Error::InvalidDeviceId)
        );
    }

    #[test]
    fn device_id_rejects_wrong_length() {
        assert_eq!(DeviceId::parse("0011"), Err(Error::InvalidDeviceId));
    }
}
