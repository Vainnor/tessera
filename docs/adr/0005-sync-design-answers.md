# 0005: Answers to the sync design's open questions

- Status: Accepted
- Date: 2026-10-22

## Context

ADR 0002 accepted end-to-end encrypted sync and left its details to the open questions at the end
of `docs/threat-model.md`, saying it is fully accepted once those are answered. ADR 0004 added a
tenth question about server access. Encryption and the server cannot be built until these are
settled, because most of them fix a storage format, a wire format or a key the server stores.
The maintainer answered them on 2026-10-22; this record keeps the answers and why. Question 7,
history compaction, is not answered here and goes to its own ADR.

## Options

Each answer below names the option chosen and the one set aside. Where a choice trades privacy
against cost, the deployment it was judged for is one operator running a server for themselves
and a few people they invite (ADR 0004).

## Decision

1. **Group keys: key generations, no MLS.** Shared vaults keep ADR 0002's design: a removal makes
   a new vault key generation, wrapped to each remaining member. MLS (RFC 9420) would bring a
   large dependency and protocol for forward secrecy that small groups gain little from. Revisit
   if shared vaults routinely have tens of members.
2. **Identity checks when sharing: trust on first use, with warnings.** What is pinned is a
   member's device-log root (the self-signed first entry), not their user key, which changes on
   every device removal. A share link carries the invitee's fingerprint, so following it checks
   the root with no extra step. Without a link, the first root seen is pinned. A new user-key
   generation signed by a device current in that member's signed device log is not a key change;
   the warning, which names the member, fires only for a key that cannot be reached from the
   pinned root. Until it is acknowledged, the client wraps no vault key to that key and applies no
   membership entry or update signed under it. Safety numbers stay available
   for anyone who wants to compare them. Requiring a comparison every time was set aside as
   friction most people would click through.
3. **Recovery kit: 24 words. Passphrase recovery: never.** The 256-bit recovery key is shown as
   24 words from the BIP-39 English word list: 264 bits, the key plus 8 bits from the first byte of
   its SHA-256. A word outside the list is always caught, and a wrong valid word 255 times in 256;
   a key that passes the checksum but is wrong then fails to unwrap, and nothing changes. The
   words encode the key directly; BIP-39's passphrase-to-seed derivation is not used. Input is
   normalized (trimmed, lowercased, NFKD) and must be exactly 24 list words, unique 4-letter
   prefixes accepted; the checksum is recomputed and compared in constant time, and the parsed key
   is a zeroizing type. Tests use the BIP-39 reference vectors' entropy and mnemonic columns, pin
   the word list by its SHA-256, and refuse 23 or 25 words, a word outside the list and a bad
   checksum. No passphrase-wrapped copy of any key is stored on the server, so the server holds
   nothing protected by a secret weaker than 256 random bits, and losing every device and the kit
   loses the synced data. ADR 0002 suggested age for the kit; age is not used for it, and remains
   a candidate for exported backups.
4. **Ciphertext sizes are padded with Padmé.** Every ciphertext the server stores (update
   batches, snapshots, attachments) is padded before encryption to the next Padmé size (Nikitin
   et al., "Reducing Metadata Leakage from Encrypted Files and Communication with PURBs", 2019).
   Padmé reveals O(log log n) bits of a length and costs at most about 12% extra space. It hides
   little below a few KiB, so small update batches still show their approximate size, and batch
   count and timing stay visible. The layout inside the AEAD (version byte, 4-byte length,
   content, zero padding), the fail-closed checks on decryption and the tests are in the threat
   model's "Padding" section. No padding was the alternative; it lets the server estimate the
   size of every edit and file.
5. **Device names are encrypted** to the user key. The server sees opaque device ids only. During
   pairing the existing device encrypts the new device's name inside the new device's signed
   device-log entry, so the server can neither read nor swap it (threat model, "Device
   enrollment and pairing").
6. **Sharing is per vault only** in the first version. To share one note, move it into a shared
   vault. Per-note keys and membership were set aside: they roughly double the key management to
   get right.
7. Not answered here (history compaction; its own ADR).
8. **Raw HTML in notes renders as a safe subset.** The renderer passes raw HTML through a
   sanitizer that parses into a tree and re-serializes from a fixed allowlist (never a regex),
   in `tessera-core` or a reviewed library, before the HTML reaches the DOM; the webview decides
   nothing. The allowlist holds formatting elements (such as `details`, `summary`, `sub`,
   `sup`, `kbd`, `mark`, `br`) and no attributes, so no script, style, event handler, link target
   or remote resource survives. Anything outside the allowlist shows as text. This keeps more
   Obsidian notes readable than showing all HTML as text, at the cost of a sanitizer allowlist that
   is security-relevant and needs its own tests. The exact list is set when the renderer is built.
9. Answered by ADR 0003 (Automerge).
10. **Server access details** (ADR 0004):
    - The server id is 128 random bits from the OS CSPRNG, generated at first start and kept with
      the server's data, so moving the server to a new address does not break pinned devices.
      The server needs no signing key.
    - The default storage quota is 10 GiB per account; the operator can change it in the config.
    - Sessions end after 8 hours without use and 7 days after sign-in, whichever comes first.
      These were chosen for fewer device-key signatures and keychain prompts on mobile; a
      stolen session token stays useful for up to 7 days. Removing a device ends its sessions at once regardless.
    - Invites carry no name or label. The server never links a person to an account id; an
      operator who wants to track who an invite was for does so outside the server.
    - Rate-limit values stay an implementation detail, set and tested with the server.

Two smaller open items in the threat model are settled the same way:

- **No scheduled key rotation.** Keys rotate on removal or suspected compromise only. Rotation
  only cuts off someone who has lost access, which removal already does; with random 192-bit
  nonces no key wears out from use; and a timer would add server-visible events.
- **Equivocation is caught by device cross-checks.** Each device includes the log heads it has
  seen in its signed, encrypted updates, so a fork is detected as soon as forked devices see each
  other's updates, but not if the server keeps them apart for good. A transparency log is set aside until there is a hosted service with
  operators the users do not know.

## Consequences

- The threat model's primitives gain Padmé and the BIP-39 word list, and lose Argon2id, which only
  passphrase recovery needed. `crypto.md` follows: Padmé and the BIP-39 checksum are added, and
  Argon2id is removed until an ADR needs it.
- Padding costs up to about 12% of storage and bandwidth on every server.
- A raw HTML sanitizer allowlist becomes part of the security surface (threat A6), tested with one accept case per
  element; attribute stripping for every attribute, including `open`, `on*`, `style`, `href` and
  `src`; `script`, `style`, `iframe`, `svg`, `math`, `template` and `noscript` coming out as text;
  mutation-XSS and namespace cases (`<svg><style>`, `<math><mtext>`), comments, CDATA, unclosed,
  misnested and entity-encoded tags; output stable over two passes; and one test proven by
  breaking the allowlist. Markdown's own links and images are governed separately, by the CSP and
  link handling.
- The 7-day session ceiling widens the window for a stolen session token, which can store and fetch
  ciphertext, use up the account's quota and open pairing rendezvous, but cannot add a device,
  forge a signed update or read notes (ADR 0004).
- With question 7 answered by ADR 0006, ADR 0002's open questions are all answered.
- Revisit item 1 if shared vaults grow large, item 2 if users report clicking through key-change
  warnings, and the equivocation answer if a hosted service exists.
