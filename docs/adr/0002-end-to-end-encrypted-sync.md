# 0002: End-to-end encrypted sync

- Status: Accepted (direction). The design in `docs/threat-model.md` is proposed, not accepted.
- Date: 2026-10-22

## Context

Tessera syncs notes across a user's devices and, later, with a few other people, through a
server that is either self-hosted or run by Vainnor. Notes are among the most private data
people have. A sync server is a single place where every note of every user can be read, logged,
subpoenaed or leaked. The maintainer has decided that the server must never be able to read them.

## Decision

1. Sync is **end-to-end encrypted**. The server stores and relays ciphertext plus the minimum
   metadata needed to route it (listed in `docs/threat-model.md` → "What the server inevitably
   sees"). It never holds a key that decrypts content, names or structure.
2. **No custom cryptography.** Tessera composes standard primitives (XChaCha20-Poly1305,
   HKDF-SHA-256, HPKE per RFC 9180, Ed25519, Argon2id only if needed) from reviewed libraries,
   in a key-generation scheme that existing products have run for years (Keybase teams,
   1Password vaults). Every crypto change follows `.claude/rules/crypto.md`.
3. All cryptography lives in `tessera-core`. No client (TypeScript, Swift, Kotlin) implements any.
4. The server cannot recover accounts. Recovery is a surviving device or the user's recovery key.

## Options considered

| Option                                                                   | What it is                                                                                                             | For                                                                                                                                                                                                  | Against                                                                                                                                                                                                                                                                                                                                           |
| ------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **A. libsodium** (via Rust bindings)                                     | The C library behind many E2EE apps                                                                                    | Mature, audited, misuse-resistant APIs (`crypto_box_seal`, `secretstream`, XChaCha20-Poly1305)                                                                                                       | C code behind FFI and `unsafe` in the core; cross-compiling for iOS and Android adds build work; still leaves group key management to us                                                                                                                                                                                                          |
| **B. RustCrypto + dalek primitives, HPKE for wrapping, key generations** | The same primitives as libsodium in pure Rust, composed as per-vault keys with generations, wrapped per user with HPKE | No C toolchain; builds for every target UniFFI needs; HPKE is a standard construction for exactly "encrypt this key for that public key"; the generation pattern gives new devices access to history | We own the composition, so it needs external review before release; audit status differs crate by crate and must be checked                                                                                                                                                                                                                       |
| **C. age** (the `age` crate)                                             | A file encryption format with X25519 and passphrase recipients                                                         | Simple, well specified, good Rust implementation                                                                                                                                                     | A file format, not a sync protocol: no signatures, membership or rotation. Useful for exported backups and recovery kits                                                                                                                                                                                                                          |
| **D. MLS** (RFC 9420, via OpenMLS)                                       | The IETF standard for group key agreement, with forward secrecy and post-compromise security                           | Standardized; efficient add and remove for large groups; strong security under device compromise                                                                                                     | Built for messaging, where old messages may become unreadable. Notes must stay readable forever, so history keys would have to be re-shared outside MLS, which gives up much of its benefit. It needs a strictly ordered delivery service and keeps heavy per-group state on every device. Much more code than a few-person sharing feature needs |
| **E. Signal protocol / Olm and Megolm** (vodozemac)                      | Messaging protocols                                                                                                    | Proven at scale                                                                                                                                                                                      | Same mismatch as MLS: session ratchets for conversations, not long-lived shared documents                                                                                                                                                                                                                                                         |

## Recommendation

**Option B**, with **C (age)** for exported backups and the printable recovery kit, if the
recovery format work shows it fits. MLS is deferred, not rejected. The vault key is separate from
how it reaches members, so the distribution layer could later move to MLS for large or
high-churn shared vaults without changing how content is encrypted. Revisit if sharing grows
beyond a few people per vault, or if post-compromise security becomes a requirement.

## Consequences

- The server is simple and holds nothing worth stealing except metadata.
- Losing every device and the recovery key loses the synced copy for good. The app must say so
  at setup, and local notes are unaffected.
- Server-side search, web access to notes and server-side previews are impossible by design.
- The composition in option B must be reviewed by someone outside the project before any release
  that syncs real data. Budget for that.
- The CRDT library choice is constrained: its updates must be batchable, encryptable as opaque
  blobs, and compactable without the server's help.

## Open questions

The list in `docs/threat-model.md` → "Open questions for the maintainer" belongs to this
decision; ADR 0002 is fully accepted when those are answered.
