---
paths:
  - "crates/core/src/crypto*"
  - "crates/core/src/crypto/**"
  - "crates/core/src/sync*"
  - "crates/core/src/sync/**"
  - "crates/server/**"
  - "docs/threat-model.md"
  - "docs/adr/**"
---

# Cryptography Rules (not loaded at startup; read in full before touching keys, encryption, signatures, sync payloads or the server)

The design is `docs/threat-model.md`; the decision is `docs/adr/0002-end-to-end-encrypted-sync.md`.

## Non-negotiable

- **No custom cryptography.** No new primitive, mode, padding scheme, KDF construction or
  protocol. Compose only what the threat model names: XChaCha20-Poly1305, HKDF-SHA-256, HPKE
  (RFC 9180), Ed25519 with strict verification, SHA-256 only for hashing 256-bit random tokens and the BIP-39 checksum of the recovery key (never for passphrases, MACs or key derivation), Padmé length padding inside the AEAD in the layout the threat model gives (ADR 0005), the OS CSPRNG. Argon2id returns only through an ADR that needs it. Anything else is an ADR first.
- **Reviewed crates only.** A crypto crate is added through `dependency-researcher` _and_
  `crypto-reviewer`, with its audit status checked from the source (not memory) and written in
  ADR 0001's dependency table.
- **All crypto in `tessera-core`.** Never in TypeScript, Swift or Kotlin. The server performs exactly three cryptographic operations, all for access control (ADR 0004) and all by calling `tessera-core`: strict Ed25519 verification of sign-in, log and deletion-request signatures (ADR 0006), SHA-256 of invite and session tokens, and OS-CSPRNG draws for tokens, challenges and ids. It holds no private key. Anything else on the server, including a server identity key, is an ADR first. No key material crosses
  IPC or FFI except opaque handles.
- **Fail closed.** An error, an unknown version, a failed signature or an unexpected generation
  stops the operation. Never fall back to plaintext, an older key or "skip verification".
- **Stubs are marked and fail.** Until the real code lands, functions in `crypto.rs` return
  `NotImplemented`. Never return plaintext labeled as ciphertext, even in a test helper.

## Implementation rules

- Randomness from `getrandom`/`OsRng` only. Nonces for XChaCha20-Poly1305 are random 24 bytes.
- Bind context as AEAD associated data: format version, vault id, document id, lineage id (ADR 0006), key generation,
  author device id. A ciphertext that verifies in the wrong context is a bug.
- Domain-separate every derived key with a distinct HKDF `info` label (`"tessera/doc/v1"`).
- Version every wire and storage format from the first byte, so it can change without guessing.
- Secrets live in types that zeroize on drop and do not implement `Debug`, `Display` or `Serialize`.
  Session tokens, invites, pairing codes and the recovery key follow the same rules: never logged,
  never returned over IPC.
- Compare secrets and MACs in constant time; use the library's verify functions, never `==`. Looking up the stored SHA-256 hash of a 256-bit random token by index is allowed; never compare raw tokens.
- Never log, print, panic with or put in an error message any key, nonce-plus-key, plaintext, or
  ciphertext large enough to be content.

## Testing

- Published test vectors for every primitive the code wraps (RFC 9180, RFC 8032 test vectors).
- Round-trip tests, plus a rejection test for each tampering case: flipped ciphertext bit, wrong
  associated data, wrong generation, wrong key, truncated input, replay into another document or another lineage.
- A guard test is proven by breaking the guard and watching it fail (`testing.md`).

## Review

Every crypto change is reviewed by the `crypto-reviewer` agent before it is committed, and by a
person outside the project before any release that syncs real data.
