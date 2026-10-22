//! Sync abstractions. Stub: no CRDT engine or transport exists yet.
//!
//! Sync will merge edits from several devices with a CRDT and move only ciphertext through the
//! server (`docs/adr/0002-end-to-end-encrypted-sync.md`). These types fix the shape the UIs
//! program against while the engine is chosen.

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
}
