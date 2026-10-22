# Threat model and end-to-end encryption design

Status: **proposal for review.** Nothing in this document is implemented. The direction
(end-to-end encrypted sync) is decided in [ADR 0002](adr/0002-end-to-end-encrypted-sync.md);
the details below are the starting design for that work. Items marked **Open** need a decision
from the maintainer before code is written.

## What Tessera protects

| Asset                                                      | Where it lives                                             | Notes                                                                           |
| ---------------------------------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Note content (markdown)                                    | Vault folder on each device; ciphertext on the sync server | The user's data. Plaintext on disk by design (Obsidian-compatible files).       |
| Attachments, drawings, ink, PDFs and annotations           | Vault folder; ciphertext on the server                     | Often larger and more personal than text.                                       |
| Structure: file and folder names, links, tags, frontmatter | Vault folder; inside encrypted documents on the server     | Names leak as much as content ("divorce lawyer.md"), so they are encrypted too. |
| Edit history (CRDT updates)                                | Local sync state; ciphertext on the server                 | Holds text the user deleted. See "Deleted text lives on in history".            |
| Keys                                                       | OS keychain on each device; wrapped copies on the server   | Device keys never leave the device.                                             |
| Recovery key                                               | With the user (printed or in a password manager)           | The only way back in if every device is lost.                                   |
| Search index                                               | App data folder on each device                             | Plaintext tokens of every note. Never synced.                                   |
| Metadata                                                   | The sync server                                            | Unavoidable; listed in "What the server sees".                                  |

## Trust boundaries

```
 ┌──────────────── device (trusted while unlocked) ─────────────────┐
 │                                                                   │
 │  webview UI  ──IPC (untrusted input)──▶  tessera-core (Rust)      │
 │  (React; renders                         vault · documents ·      │
 │   notes and themes)                      sync · crypto · index    │
 │                                            │          │           │
 │                              plaintext .md files   OS keychain    │
 └────────────────────────────────────────────┼──────────────────────┘
                                              │ TLS + E2EE: only ciphertext
                                              ▼   and the metadata listed below
                              ┌──────────────────────────────┐
                              │ sync server (untrusted for   │
                              │ confidentiality; relay and   │
                              │ backup; self-hostable)       │
                              └──────────────┬───────────────┘
                                             ▼
                              other devices · collaborators
```

1. **Webview to core.** The UI renders untrusted content (notes, imported themes, later PDFs).
   It is treated as compromised-able: it holds no keys, every IPC argument is validated in Rust,
   the CSP blocks inline and remote code, and Tauri capabilities grant only named commands.
2. **Core to filesystem.** The vault is plain files, readable by anything running as the user.
   Tessera does not try to protect the vault from other software on an unlocked machine; full-disk
   encryption (FileVault, BitLocker, LUKS) is the user's protection at rest, and the docs say so.
3. **Device to server.** The server is trusted to relay and store, and _not_ trusted with
   confidentiality or integrity of content. It is also not fully trusted for availability (it can
   drop or withhold data) or for the membership lists it serves (it could lie; see mitigations).
4. **User to user (sharing).** A collaborator is trusted with the vaults shared with them and
   nothing else, and only while they are a member.
5. **Build and release pipeline.** Dependencies, CI and the update channel can ship code that runs
   with full access to everything above.

## Adversaries

|     | Adversary                                                                                                       | Can                                                                                      | Goal                                                                 |
| --- | --------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- | -------------------------------------------------------------------- |
| A1  | Sync server operator, or an attacker who owns the server (including a third party hosting a self-hosted server) | Read and change everything stored and relayed; serve different data to different devices | Read notes; map who works with whom                                  |
| A2  | Network attacker                                                                                                | Observe, delay, drop or modify traffic                                                   | Same as A1                                                           |
| A3  | Former collaborator or removed device                                                                           | Kept every key they had                                                                  | Read content written after removal                                   |
| A4  | Current collaborator                                                                                            | Legitimate access to a shared vault                                                      | Read vaults not shared with them; impersonate another member's edits |
| A5  | Thief of a locked device                                                                                        | Physical access, no unlock                                                               | Read notes or keys                                                   |
| A6  | Malicious content: a crafted note, theme, PDF or link                                                           | Reaches the webview renderer                                                             | Run script, reach IPC, exfiltrate                                    |
| A7  | Supply chain: a compromised dependency, action or update                                                        | Code execution at build or run time                                                      | Everything                                                           |

Out of scope: an attacker with the unlocked device or malware running as the user (they can read
the plaintext vault directly), and a user who hands over their recovery key.

## Design goals

- **G1.** The server learns nothing about note content, file names or structure.
- **G2.** A removed device or member cannot read anything written after their removal.
- **G3.** Any edit can be attributed to the device that made it, and the server cannot forge one.
- **G4.** The server cannot add a device or member to an account or vault on its own.
- **G5.** No custom cryptography: standard primitives and constructions from reviewed libraries,
  composed in patterns that existing products already use.
- **G6.** One implementation, in `tessera-core`, used by every client through Tauri now and UniFFI
  later. Swift and Kotlin code never implements cryptography.

## Primitives (proposed)

All from reviewed, widely used implementations; the crate choice is part of the first crypto
change and goes through `.claude/agents/crypto-reviewer.md`.

| Purpose                                                       | Primitive                                                                              | Why                                                                                                                            |
| ------------------------------------------------------------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| Content encryption                                            | XChaCha20-Poly1305                                                                     | AEAD with 192-bit nonces that are safe to pick at random, so many devices can encrypt under one key with no nonce coordination |
| Key derivation                                                | HKDF-SHA-256                                                                           | Standard; per-document and per-purpose keys from one secret                                                                    |
| Wrapping a key for a recipient                                | HPKE (RFC 9180), base mode, DHKEM(X25519, HKDF-SHA256), HKDF-SHA256, ChaCha20-Poly1305 | Standard public-key encryption with context binding; no hand-built `X25519 + KDF + AEAD`                                       |
| Signatures                                                    | Ed25519, strict verification                                                           | Device identity, signed device and membership logs, update attribution                                                         |
| Passphrase stretching (only if passphrase recovery is chosen) | Argon2id                                                                               | Memory-hard; standard parameters from RFC 9106                                                                                 |
| Randomness                                                    | OS CSPRNG (`getrandom`)                                                                | No user-space generators                                                                                                       |

## Key hierarchy

```
Device key (per device: Ed25519 + X25519, private half in the OS keychain, never exported)
   │  HPKE-wraps
   ▼
User key, generation u (per account: Ed25519 + X25519, shared by the user's devices)
   │  HPKE-wraps                                  ▲ also wrapped by the recovery key
   ▼                                              │
Vault key, generation g (per vault: 256-bit symmetric, shared by every member)
   │  HKDF(info = "tessera/doc/v1" ‖ doc id)        each generation also encrypts g − 1,
   ▼                                              so members can read old history
Document key ──▶ XChaCha20-Poly1305 over each batch of CRDT updates and each snapshot
   │
   └─ attachment keys: random per blob, wrapped by the document key
```

- **Why a user key between devices and vaults.** A shared vault's key is wrapped once per
  _member_, not per device. Adding a device then means wrapping one user key, not re-wrapping
  every vault (the Keybase "per-user key" pattern).
- **Generations.** Rotation never re-encrypts history. New writes use the new generation, and
  the new vault key encrypts the previous one, so a current member can walk back through every
  generation while a removed member never sees a new one. This is the pattern 1Password vaults
  and Keybase teams use.
- **Associated data.** Every ciphertext binds the vault id, document id, key generation, format
  version and the author device's id as AEAD associated data, so a blob cannot be replayed into
  another document or passed off as another generation.

## What the server stores

The server keeps, per vault, an append-only log of encrypted update batches plus encrypted
snapshots and attachment blobs. Document ids, paths and names exist only inside the ciphertext
(the vault manifest is itself an encrypted document), so the server sees vault-level activity,
not which note changed.

## Device enrollment and pairing

1. **First device.** Generates its device key, a user key (generation 1) and, for each vault, a
   vault key (generation 1). It registers the account with the server: a random account id, the
   device's public keys, the user key's public key, a recovery-key verifier (below), and the first
   entry of the account's **device log**: an append-only list of devices, each entry signed by an
   already-trusted device (the first entry is self-signed).
2. **Adding a device.** An existing device E shows a QR code holding a one-time 128-bit pairing
   secret and the server address. When there is no camera (desktop to desktop), it shows a short
   code instead and the two devices run a PAKE (CPace, or SPAKE2 as Magic Wormhole uses) over the
   server relay. The pairing secret protects a channel the server cannot read.
3. Over that channel the new device N sends its public keys and a device name. E shows the name
   and a short fingerprint of N's key, and N shows the same fingerprint. The user confirms they
   match and approves on E.
4. E appends a signed entry for N to the device log and HPKE-wraps the current user key to N. N
   can now unwrap every vault key it is entitled to.

The server relays pairing messages and stores the log. It cannot add a device, because it cannot
produce a signature from a trusted device (G4).

## Sharing with other people

Sharing is per vault: a shared vault is its own vault with its own keys. Sharing a single note
is out of scope for the first version (**Open**).

1. The owner looks up the invitee's account. The server returns the invitee's user public key and
   device log.
2. **Identity check.** The server could substitute its own key here (a man in the middle). The
   owner's and invitee's apps each show a **safety number**: a fingerprint of both user keys. They
   compare it out of band (in person, by phone, or over a messenger they already trust). An invite
   link sent out of band can carry the invitee's fingerprint so the check is automatic.
   **Open:** whether to require the check, or allow trust-on-first-use with a visible warning
   when a key changes.
3. The owner HPKE-wraps the current vault key (which also unlocks older generations) to the
   invitee's user key, and appends a signed entry to the vault's **membership log**. Every member's
   client verifies the membership log before trusting a key wrap or a write.

## Rotation and revocation

| Event                                   | What rotates                                                                                                            | Effect                                                          |
| --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| A device is removed or lost             | User key → new generation, wrapped to the remaining devices. Every vault the user belongs to → new vault key generation | The removed device cannot read anything written afterwards (G2) |
| A member is removed from a shared vault | That vault's key → new generation, wrapped to the remaining members                                                     | Same, for that vault                                            |
| Suspected compromise                    | Device key of the affected device (re-enroll), then as above                                                            |                                                                 |
| Routine                                 | **Open:** whether scheduled rotation adds anything, given content keys are already per-document                         |                                                                 |

A removal is signed into the device log or membership log _before_ the new generation is created,
and clients refuse to encrypt new content under a generation older than the newest one they have
seen. Tessera cannot take back what a removed party already downloaded; the UI says so when
removing someone. Re-encrypting history under new keys is possible but expensive, and is not
planned (**Open**).

## Account recovery

The server cannot recover an account. It never holds a key that decrypts anything. This is the
cost of G1, and the app must say so plainly when sync is set up.

- **Any surviving device is a recovery path**: it can enroll a new device. Local notes are also
  never at risk from losing sync: they are plain files on every device.
- **Recovery key.** At setup the app generates a 256-bit recovery key and shows it once, as a
  printable recovery kit (**Open:** 24 words from a standard word list, or grouped base32). The
  user key is wrapped by a key derived from it, and that wrapped copy is stored on the server. A
  second derived key (HKDF with a different label) acts as a login credential for recovery, so the
  server can authenticate a recovering user without learning the recovery key.
- **If every device and the recovery key are lost, the synced data cannot be recovered by anyone.**
- **Rejected for the default:** recovery from a user-chosen passphrase. The server would hold a
  blob it can attack offline, and most passphrases do not survive that. **Open:** offer it as an
  opt-in with Argon2id and a minimum strength, or not at all.
- **Later:** social recovery (Shamir shares held by trusted contacts).

## What the server inevitably sees

Even with everything above, a sync server learns:

- account ids, how many devices each has, their public keys and device names (**Open:** encrypt
  device names to the user key);
- IP addresses and connection times (and so rough location and daily patterns), unless the user
  connects through a VPN or Tor;
- which vaults exist, how large each is, how often each changes and when;
- the sizes of update batches and attachments, which approximate the size of edits and files
  (**Open:** pad to size buckets, at a storage and bandwidth cost);
- who shares a vault with whom, and when membership changes;
- key generation counts, which reveal how often devices or members were removed.

A self-hosted server shows all of this to whoever runs it. The privacy policy must list it before
a hosted service exists.

## Search and indexing

Search runs on the device and only there.

- Each device builds its own full-text index from the plaintext vault (**Open:** SQLite FTS5 or
  tantivy) and keeps it in the app data folder. The index is never synced or sent anywhere; a new
  device builds its own.
- Handwriting recognition runs on the device (on iPad, Apple's on-device Vision framework) and
  feeds the same local index. No cloud OCR.
- Backlinks and the link graph are computed locally from the files.
- The index holds plaintext tokens of every note, so it is as sensitive as the vault. Deleting a
  note removes it from the index, and the user can clear or rebuild the index at any time.
- Server-side search and searchable encryption are rejected: every practical scheme leaks query
  and access patterns to the server, which defeats G1.

## Deleted text lives on in history

A CRDT keeps the history of every edit so it can merge, which means text a user deleted still
exists in the update log, on every device and (encrypted) on the server. Before sync ships, the
design needs a compaction step (snapshot the current state, drop the old updates once every
device has caught up) and a clear answer in the UI for "I deleted this, is it gone?" (**Open**).

## Other threats and mitigations

| Threat                                                                   | Mitigation                                                                                                                                                         | State                                   |
| ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------- |
| A6: a crafted note runs script in the webview                            | CSP without `'unsafe-inline'` or remote sources; markdown rendered through a sanitizer; raw HTML in notes not rendered by default (**Open:** allow a safe subset?) | CSP done; renderer not built            |
| A6: an imported theme injects CSS                                        | Themes hold only `#rrggbb` colors, bounded numbers and keywords; refused over 16 KiB before parsing                                                                | Done (`packages/theme/src/validate.ts`) |
| A6: a compromised webview calls powerful APIs                            | Capabilities deny by default; one command granted by name; no fs, shell or http plugins                                                                            | Done                                    |
| A1/A2: the server rolls back or withholds updates                        | Signed per-device sequence numbers; clients remember the newest state they have seen and warn on regression                                                        | Not built                               |
| A1: the server serves different logs to different devices (equivocation) | Devices cross-check log heads when they sync with each other; **Open:** a transparency log                                                                         | Not built                               |
| A4: a member forges another member's edit                                | Updates are signed by the author's device key, not only encrypted with the shared vault key                                                                        | Not built                               |
| A5: locked device stolen                                                 | Keys in the OS keychain (Secure Enclave or TPM-backed where available); vault protected by OS disk encryption                                                      | Not built                               |
| A7: compromised dependency                                               | cargo-deny, cargo-audit, pnpm audit, dependency review, 7-day release age for npm and Dependabot, lockfiles, SHA-pinned actions                                    | Done                                    |
| A7: malicious update                                                     | Signed updates (Tauri updater, minisign), manual check only, no auto-download                                                                                      | Not built                               |
| Secrets committed to the repo                                            | gitleaks pre-commit hook and CI scan of the full history                                                                                                           | Done                                    |

## Open questions for the maintainer

1. MLS for shared vaults later, or keep key generations for good (see ADR 0002)?
2. Require safety-number verification for sharing, or trust on first use with change warnings?
3. Recovery kit format: words or base32? Passphrase recovery: opt-in, or never?
4. Pad ciphertext sizes, and at what cost?
5. Encrypt device names on the server?
6. Sharing a single note, or only whole vaults?
7. CRDT history compaction and how the UI explains deleted text.
8. Raw HTML in notes: never render, or a sanitized subset?
9. Which CRDT library (Yjs/yrs, Loro, Automerge), judged partly by how cleanly its updates can be
   batched, encrypted and compacted.
