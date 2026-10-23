//! Tessera's shared core.
//!
//! Every client (the Tauri desktop app now, a native iPad app through UniFFI later) links this
//! crate, so all logic that must behave the same everywhere lives here: the vault on disk, the
//! document model, sync and encryption. The UIs render what this crate returns.
//!
//! # API boundary
//!
//! The public items of this crate are the API that UniFFI will expose to Swift and Kotlin. To
//! keep that possible without a rewrite, every public item follows these rules:
//!
//! - Owned types only: `String`, `Vec<u8>`, integers, `bool`, `Option`, `Vec`, and records and
//!   enums built from them. No lifetimes, no generics, no `impl Trait` in signatures.
//! - Long-lived things (a vault, a sync engine) are objects: constructors return
//!   `Result<Self, Error>`, methods take `&self`, and any mutation uses interior mutability so
//!   the object can sit behind an `Arc`.
//! - Errors are one crate-wide [`Error`] enum whose variants carry owned, user-safe fields.
//! - The core is synchronous and owns no async runtime. Callers run blocking work on their own
//!   thread pool.
//!
//! The vault (listing, reading and writing notes on disk) and each note's sync history
//! ([`sync::NoteHistory`]) are implemented. Sync transport and encryption are stubs that fail
//! closed with [`Error::NotImplemented`].

#![forbid(unsafe_code)]

pub mod crypto;
pub mod document;
mod error;
pub mod sync;
pub mod vault;

pub use error::Error;

/// The version of this crate, for "about" screens and the sync handshake.
#[must_use]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_the_manifest() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}
