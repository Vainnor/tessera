# 0003: CRDT library for note sync

- Status: Accepted
- Date: 2026-10-22

## Context

Sync merges edits that two devices made to the same note while apart. ADR 0002 settles that the
server sees only ciphertext and the metadata listed in the [threat model](../threat-model.md).
That puts three constraints on whatever merges the edits. Its updates must be opaque blobs we
can encrypt and sign. They must apply in any order: the server keeps an append-only log but
cannot order updates causally, since it cannot read them (derived from ADR 0002). And, per ADR
0002's consequences, the history of deleted text must be removable without the server's help
(threat model, "Deleted text lives on in history"). The
library runs inside `tessera-core`, so it must be synchronous, build for iOS and Android with no
JS runtime, and stay behind the UniFFI boundary (`.claude/rules/rust.md`). Updates that arrive
after decryption come from devices or collaborators we trust only so far (adversary A4), so the
decoder is an untrusted-input parser and must fail with an error, not a panic.

[`docs/architecture.md`](../architecture.md) named three candidates. This record compares them
on the criteria listed there and in ADR 0002. Text merging without a CRDT (three-way diff, as a
plain file sync would do) is out: ADR 0002 and the threat model assume CRDT-based sync.

The markdown files stay the source of truth, and the CRDT is sync state beside them. Switching
libraries later costs one full resync of every note, not a data migration. Seeding a document
from a file is itself a reset (see Decision): two devices that seed the same note on their own
get two unrelated histories, and merging them duplicates the text.

## Measurements

A throwaway crate ran the same scenarios through all three libraries: Rust 1.98.1, release
build, Apple M3 Pro, a counting allocator for heap, synthetic ASCII markdown. It is not in the
repository. Single runs, so treat the timings as rough.

| Scenario                                                         | yrs 0.28.0                        | Loro 1.16.2                            | Automerge 0.12.0           |
| ---------------------------------------------------------------- | --------------------------------- | -------------------------------------- | -------------------------- |
| Three devices, overlapping insert and delete                     | Converges                         | Converges                              | Converges                  |
| Updates delivered out of order                                   | Converges                         | Converges (reports the update pending) | Converges                  |
| Two words typed concurrently at one spot, forward                | `alphaBRAVO`                      | `alphaBRAVO`                           | `BRAVOalpha`               |
| The same, typed backward (cursor stays put)                      | `helloWORLD`                      | `helloWORLD`                           | `WhOeRlLlDo` (interleaved) |
| Average update per keystroke                                     | 15 B                              | 85 B                                   | 96 B                       |
| Snapshot of a 4 KB note after 2,000 keystrokes                   | 4,103 B                           | 1,041 B                                | 492 B                      |
| Load one 2 KB note with 50 edits                                 | 3 µs                              | 13 µs                                  | 815 µs                     |
| Heap per loaded note                                             | 4.2 KiB                           | 34.3 KiB                               | 25.1 KiB                   |
| Deleted text recoverable from a normal snapshot                  | No (dropped by default GC)        | Yes                                    | Yes                        |
| Deleted text recoverable after compaction                        | No                                | No (shallow snapshot)                  | No compaction exists       |
| A device offline across a compaction merges its edits afterwards | Yes                               | No: refused as missing dependencies    | Not applicable             |
| 20,000 random inputs of at most 64 bytes into the decoder        | A 56-byte input asked for > 2 GiB | No panic, no slow input                | No panic, no slow input    |

The random-input row flatters Loro and Automerge: both check a magic number and a checksum
first, so random bytes rarely reach the real parser. yrs has no such header.

Per-keystroke sizes matter less than they look. An encrypted, signed envelope adds tens of bytes
to any update, so sync batches keystrokes in every design, and a batch of 2,000 characters was
about 2 KB in all three.

## Options

### A. yrs (Yjs in Rust)

Gives the best engineering fit: the smallest updates, the least memory, deleted text dropped by
default, and edits from a device that was offline across a compaction still merge. Interop with
the Yjs ecosystem.

Costs, from the [dependency research](https://github.com/y-crdt/y-crdt/issues):

- Memory safety is in question. An open RustSec report, [advisory-db
  #3118](https://github.com/rustsec/advisory-db/issues/3118), shows a use-after-free reachable
  from safe code, and [y-crdt #676](https://github.com/y-crdt/y-crdt/issues/676) is a heap
  use-after-free that only happens with GC on, the feature that drops deleted text. The crate
  has about 70 `unsafe` blocks, including hand-written `Send` and `Sync` impls.
- The decoder is open to allocation bombs and panics on crafted bytes (#675, #679, #720; the
  table above reproduced one).
- Out-of-order delivery, which encrypted sync relies on, has open bugs (#670, #582, #677).
- `cargo deny` fails on [RUSTSEC-2026-0215](https://rustsec.org/advisories/RUSTSEC-2026-0215)
  (`smallstr` unmaintained, no fixed version).
- One maintainer publishes every release and wrote 91 of the last 100 commits. Every minor
  version breaks the API, and there is no written promise about the update format.

### B. Loro

Gives a fast, compact engine that kept concurrent words whole in both typing tests, shallow
snapshots that really drop old history, and a stated stable encoding since 1.0.

Costs:

- `cargo deny` fails on four unmaintained crates (`im`, `sized-chunks`, `bitmaps`,
  `atomic-polyfill`), and `cargo audit` reports two of them unsound
  ([RUSTSEC-2023-0126](https://rustsec.org/advisories/RUSTSEC-2023-0126),
  [RUSTSEC-2026-0255](https://rustsec.org/advisories/RUSTSEC-2026-0255)). The fix, replacing
  `im` with `imbl` ([loro #1122](https://github.com/loro-dev/loro/pull/1122)), is not merged.
- The sub-crates do not follow semver: `loro = "=1.16.2"` alone resolves `loro-internal` 1.17.1
  and fails to compile. Three more crates must be pinned exactly.
- `import` panics on some inconsistent or unluckily ordered input, and the panic poisons the
  document so every later call panics too ([#793](https://github.com/loro-dev/loro/issues/793),
  [#1068](https://github.com/loro-dev/loro/issues/1068),
  [#1215](https://github.com/loro-dev/loro/issues/1215)). A shallow-snapshot import can also stay
  pending forever without an error ([#1040](https://github.com/loro-dev/loro/issues/1040)).
- Compaction needs every device caught up first; an edit made against trimmed history is refused
  (measured above).
- About 110 new crates, including `psm`, which assembles a `.s` file at build time.
- One main maintainer; funding is GitHub Sponsors only.

### C. Automerge

Gives the cleanest supply chain of the three: no RustSec or GHSA advisory against it or its
dependencies, and `cargo deny` passes. The decoder survived the random-input run. It queues a
change whose dependencies are missing and applies it when they arrive, so any order works. A
funded team of two or three works on it, with Ink & Switch paying for full-time staff and named
sponsors and grants ([automerge.org](https://automerge.org/)). Automerge 3 kept the Automerge 2
file format.

Costs:

- No way to drop history in place. Issues asking for it were closed without a change, and a
  maintainer's answer is that devices "coordinate a total document reset"
  ([#348](https://github.com/automerge/automerge/issues/348),
  [#799](https://github.com/automerge/automerge/issues/799)). To remove deleted text, a note
  starts a new document from its current text once every device has caught up. An edit made
  against the old document is carried over by the device that made it, from its own local state,
  as a new signed change in the new document.
- Text typed backward at the same spot by two devices at once interleaves letter by letter
  (measured above). Forward typing, the common case, stays whole.
- Loading is slow: 815 µs per note, so a fresh device syncing 10,000 notes spends about 8 s
  loading documents. Documents are loaded only while syncing a note and dropped after.
- Every minor release has broken the API, and 0.11 started refusing some documents that older
  buggy versions wrote. Each upgrade needs a load test against saved fixtures.
- It depends on `hexane`, an alpha crate with one `unsafe` block.
- About 40 new crates in `tessera-core` and 2.6 MB of binary.
- Its built-in sync protocol exchanges change hashes and Bloom filters between peers that both
  hold the plaintext. It cannot run through a server that sees only ciphertext. Showing that
  server the hashes or heads would reveal which blobs belong to the same note and how edits
  relate, and would let it test guesses of content. We exchange change blobs instead.
- Each change carries a millisecond timestamp and an optional message, which every member who
  can decrypt the history would see.

## Decision

Use Automerge, under these rules.

The library and the API:

- `automerge = { version = "=0.12.0", default-features = false }`, with the text encoding set
  explicitly so feature unification elsewhere cannot change how positions are counted.
- Automerge types stay inside `tessera-core`. Nothing public names them, so the library can be
  replaced behind the same API. No `automerge-swift` in the iPad app: a second Rust library and a
  second copy of Automerge would sit beside the core.
- Changes are committed with a time of 0 and no message. Tessera keeps any timestamps it needs in
  its own envelope.

What the server sees:

- Sync moves encrypted, signed change blobs in a Tessera envelope that is versioned from its first
  byte, independently of Automerge's format. Automerge's sync protocol, change hashes, heads and
  actor ids never reach the server outside the ciphertext.
- Sequence numbers are per device per vault. Document ids, reset lineage and per-note counters
  stay inside the ciphertext.
- Snapshot sizes, the number of snapshots a reset or first seed uploads, and the cadence of
  batches from one device reveal note sizes, note counts and editing sessions. The sync design
  adds these to the threat model's "What the server inevitably sees", or pads and batches them
  (open question 4).

Integrity and attribution:

- Automerge's checksums, change hashes and heads are not integrity or authentication. The AEAD
  and the author device's signature are verified before any decrypted bytes reach Automerge.
  Where the signature sits, and that it covers the same context as the associated data, is part
  of the crypto design.
- Each device has one fixed Automerge actor id, bound to its signing key. A change whose actor
  id does not belong to the device that signed it, or that reuses an actor and sequence already
  seen with different bytes, is refused. Attribution comes from the signature, never from the
  actor id.

Untrusted input after decryption:

- Size limit on the decrypted bytes, then decode with limits on decompressed size and operation
  count (Automerge chunks may be DEFLATE-compressed), then check the document's shape (one text
  object at the expected key) and the text against the 16 MiB note limit, then write the file.
- Release builds change from `panic = "abort"` to `panic = "unwind"` when this dependency lands,
  so a decoder panic can be caught. Decoding and applying happen on a copy, or the in-memory
  document is dropped and reloaded from its saved state after any failure. A panic hook that
  never prints the payload is installed. A change that fails is quarantined, not retried on every
  start. This does not cover aborts, stack overflow or running out of memory, which is why the
  limits come first.
- The decoder gets a fuzz target, including compressed chunks, before sync ships. Every version
  bump adds a load test against documents saved by the previous version.

Deleted text:

- The threat model's compaction step (still open) has to become a document reset, because an
  Automerge snapshot keeps the full history. A reset starts a new document from the note's
  current text.
- This record requires one property of the reset: its lineage is bound into the AEAD associated
  data and the author's signature, so a blob from an older lineage fails closed and is never
  decoded or diffed into the new document. Only a device's own unsynced edits are carried across.
  Seeding from a file follows the same rule.
- Who may reset, how "every device has caught up" is checked without trusting the server,
  concurrent resets, the vault manifest's own reset, and what the UI says about deleted text are
  open question 7, decided with the crypto design.

The reasons, in order: yrs has an open memory-safety report against the exact feature we need,
and Loro fails two of our gates and panics on input it should refuse. Automerge's costs are a
coordinated reset, slow loading and one interleaving case. Each one is design work or a speed
limit, and none of them puts a device's memory or a note at risk.

## Consequences

- `tessera-core` gains its first heavy dependency. Adding it is a separate approval
  (`.claude/rules/dependencies.md`), recorded in ADR 0001's table when it lands.
- Compaction becomes a protocol step. Devices report what they have applied in encrypted, signed
  messages, and clients, not the server, decide that every current device in the signed device
  log has caught up. A device that never comes back blocks compaction until it is removed from
  the vault. Those reports are new server-visible metadata.
- A reset removes deleted text from the devices that take part. It does not remove it from a
  server operator (A1) who kept old ciphertext along with a key, or from a removed member who
  already downloaded it.
- `panic = "unwind"` makes release binaries somewhat larger; the change reports the size
  difference against the budgets in `docs/performance.md`.
- The crypto tests gain two rejection cases: an old-lineage blob replayed into a reset document,
  and an actor id that does not match the signing device.
- A fresh device's first sync is slower than with yrs or Loro. If 10,000-note first syncs miss
  the budget in `docs/performance.md`, load notes lazily or keep a cached snapshot per note.
- Concurrent backward typing at one spot can interleave. It needs two devices editing the same
  sentence at the same time, which a personal notes app rarely sees; shared vaults see it more.

Revisit when any of these happens:

- yrs closes advisory-db #3118 and y-crdt #676 and replaces `smallstr`.
- Loro merges the `im` replacement and fixes the import panics.
- Automerge adds a supported way to drop history in place, or its interleaving case shows up in
  real use.
- Automerge gains an advisory, or its maintenance stops.
