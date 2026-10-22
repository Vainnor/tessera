//! End-to-end encryption. **STUB: no cryptography is implemented here.**
//!
//! The design is `docs/threat-model.md` and `docs/adr/0002-end-to-end-encrypted-sync.md`; the
//! rules are `.claude/rules/crypto.md`. Until an implementation built on reviewed primitives
//! lands, every function in this module fails closed: it returns
//! [`Error::NotImplemented`](crate::Error::NotImplemented) and never returns plaintext dressed
//! as ciphertext.

use crate::Error;

/// Which generation of a vault key sealed a blob. A new generation starts on every rotation,
/// such as removing a device or a member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyGeneration(pub u32);

/// Ciphertext as it is stored on and relayed by the sync server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealedBlob {
    /// The vault key generation the blob was sealed under.
    pub generation: KeyGeneration,
    /// The authenticated ciphertext. Its layout is not defined yet.
    pub bytes: Vec<u8>,
}

/// Seals `plaintext` for the vault.
///
/// # Errors
///
/// Always [`Error::NotImplemented`]: encryption is not built.
pub fn seal(plaintext: &[u8]) -> Result<SealedBlob, Error> {
    let _ = plaintext;
    Err(Error::not_implemented("crypto::seal"))
}

/// Opens a blob sealed by [`seal`].
///
/// # Errors
///
/// Always [`Error::NotImplemented`]: decryption is not built.
pub fn open(blob: &SealedBlob) -> Result<Vec<u8>, Error> {
    let _ = blob;
    Err(Error::not_implemented("crypto::open"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_fails_closed() {
        assert!(matches!(
            seal(b"secret note"),
            Err(Error::NotImplemented { .. })
        ));
    }

    #[test]
    fn open_fails_closed() {
        let blob = SealedBlob {
            generation: KeyGeneration(1),
            bytes: b"secret note".to_vec(),
        };
        assert!(matches!(open(&blob), Err(Error::NotImplemented { .. })));
    }
}
