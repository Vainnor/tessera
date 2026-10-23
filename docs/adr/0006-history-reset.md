# 0006: Resetting a note's history

- Status: Accepted
- Date: 2026-10-22

## Context

An Automerge document keeps every operation ever made, so deleted text stays in a note's history
on every device and, encrypted, on the server (threat model, "Deleted text lives on in history").
`NoteHistory` also caps a history at 500,000 operations, 20,000 changes and 64 devices
(`crates/core/src/sync/history.rs`). A note that reaches a cap stops syncing, and two devices near
a cap can refuse each other's changes and show different text. Until a history can be reset,
`NoteHistory` cannot be used by the app.

ADR 0003 decided that compaction is a reset, a new document seeded from the note's current text,
and required one property: the reset's lineage is bound into the AEAD associated data and the
author's signature, so a blob from an older lineage fails closed. It left open who may reset, how
"every device has caught up" is checked without trusting the server, concurrent resets, the vault
manifest's own reset, and what the UI says about deleted text (open question 7). ADR 0002 is fully
accepted once this is answered.

## Options

### A. Coordinated reset only

A reset happens only after every current device has reported, in an encrypted and signed
message, that it has applied the history up to the reset's base.

- Gives: no edit is lost while every device is reachable. Edits a device makes after its last
  report and before it sees the reset still come across by the text carry-over below: A narrows
  that window to minutes, and closing it would need a freeze step this record does not add.
- Costs: one device that never comes back (a lost phone not yet removed) blocks every reset in the
  vault, and a note at a cap stays frozen until that device is removed.

### B. Reset at any time, with carry-over

Any current device may reset whenever it likes; every device carries its own edits outside the
reset's base across as text, or as a conflict copy where they overlap.

- Gives: nothing ever blocks a reset.
- Costs: every edit concurrent with a reset merges as text instead of through the CRDT, and
  frequent resets would make conflict copies common.

### C. Coordinated by default, forced when needed

As A, but a reset may go ahead without every report when a history is near a cap or the user asks
to erase deleted text now. Devices that missed it carry their edits across as in B.

## Decision

Option C. This is a new protocol made of approved primitives (Ed25519, XChaCha20-Poly1305, Padmé,
the OS CSPRNG); `crypto.md` requires an ADR for it, and it goes to the outside review ADR 0002
requires before any release that syncs real data.

**Lineages.** Every note's history belongs to a lineage, named by 128 bits from the OS CSPRNG.
Every update and snapshot binds its lineage id in the AEAD associated data and under its author's
signature, so a blob from another lineage fails to verify and is never decoded. A note's first
seed is a reset with no parent. Of two parentless seeds of one note, every device keeps the one
with the lower lineage id, and the other seed's text becomes a conflict copy, since there is no
base to carry edits from.

**The reset record** is a signed entry in the vault's log, encrypted and padded like any
ciphertext (ADR 0005). Its associated data binds the document id and the parent lineage
id. Inside the ciphertext it holds the note's document id, its kind (coordinated, cap or
request), the parent lineage
id, the new lineage id, the parent lineage's heads at the base, and the resetter's genesis
changes for the new lineage: the empty-note change `NoteHistory::join` accepts, then the changes
that insert the text. Every device builds the new lineage from those exact bytes, so their later
changes merge; seeding from the text independently would give different operation ids that never
merge. The author signs the label `"tessera/reset/v1"`, the associated data, the nonce and the
ciphertext, each length-prefixed, and the record takes the author's next per-vault sequence
number under the same signature, so the server cannot replay or reorder it, or withhold it once a
later entry from the same author is seen, without a gap or regression that raises the rollback
warning. It can hold back an author's newest entry until that author uploads again, or for good
by keeping devices apart (threat model, A1 equivocation row).

**Adopting a reset.** A device adopts a reset only if all of these hold, and refuses it otherwise,
naming its signer:

- the signer is a device current in the device log of a member current in the membership log, both
  at the record's position;
- the record uses the newest key generation the device knows (the author reissues one made under
  an older generation);
- its parent is the device's current lineage or that lineage's ancestor;
- its kind holds: a coordinated reset is covered by the latest report from every current device,
  and a cap reset by the device's own copy of the base being past 90% of a cap;
- the text the genesis changes produce equals the parent lineage's text at the base heads. A
  device that lacks a base head fetches the old lineage first (old blobs stay on the server until
  every current device has reported the new lineage) and adopts nothing until it can check; if it
  cannot reach the base, it keeps its text and shows a warning.

**Catch-up reports.** At the end of every sync in which a device applied changes, it uploads a
report, even if it has no edits of its own; it never sends one on a timer. A report is a vault-level record (its associated data has no document id) and holds the
per-note heads the device has applied, the lineage each set belongs to, and its sequence number,
signed under `"tessera/report/v1"` like a reset. It counts only toward resets of the lineage it
names. A device starts a coordinated reset when the latest report from every current device covers
the base. The server stores reports like any blob and decides nothing. Withholding reports only
blocks coordinated resets, so the most a member gains by it is pushing others onto the forced
path.

**Forced resets** go ahead without every report in two cases:

- **At a cap.** A device proposes one when its copy of a history passes 90% of any cap, and
  another device accepts a cap-forced reset only if its own copy of the base is also past 90%. A
  reset is due only if the new lineage needs at most half of each cap; a note too large for that
  stops syncing and says so, rather than resetting in a loop.
- **On request.** The user asks to erase deleted text now and confirms a prompt that names the
  members, and the user's own devices, that have not reported. In a shared vault, the reset names
  its signer to every member, and each device warns, naming the signer, when one signer issues
  more than one on-request reset of a note within a few lineages (counted per lineage, not by
  clock).

**Carry-over.** A device that adopts a reset takes the changes it holds that are outside the
base's causal past and that it authored. It diffs the text at the base against the text at the
base plus those changes, and records the diff as Automerge changes whose dependencies are the
genesis heads. The genesis text equals the base text, so the base-relative offsets map exactly,
and Automerge merges the carried changes with any edits others have already made in the new
lineage. Changes outside the base that other devices authored are dropped when their author is
still current, because that author carries them itself; their old-lineage blobs stay on the server
until the author reports the new lineage. When that author has been removed, one device, the
resetter if it is current and otherwise the current device with the lowest id, writes a single
conflict copy holding them. Each edit is carried once, so none is duplicated. Carried edits can
still merge in ways a person would not choose (two edits on adjacent markdown lines), as with any
text merge.

**Conflict copies** are named from the note's name and a sanitized, length-limited device label
(in a shared vault, the member's name, since device names are encrypted to their owner's user
key), and the path goes through `VaultPath`. If the name is taken, a counter is added; a conflict
copy never replaces a file.

**Concurrent resets** of one note form lineages with the same parent. Every device keeps the one
whose base contains the other's base; if neither contains the other, it keeps the lower lineage
id. An author can choose a low id, which is safe only because every reset's text is checked
against its base. Devices carry their edits relative to the base of the reset they keep. A device that already
adopted the losing reset also carries its own changes made in that lineage, diffing its genesis
text against its current text onto the winner's genesis heads, and keeps the losing lineage until
that is done.

**The vault manifest** resets by the same rules, but a note's current lineage is decided by its
chain of reset records, never by the manifest: lineage ids in the manifest are hints, and a
device follows the chain to the descendant the concurrent-reset rule keeps. A manifest reset carries entries across per
document id (added, renamed, removed), not by text.

**Old ciphertext.** Old history, on a device and on the server, is deleted only once every current
device has reported the same new lineage and no sibling reset appears in the log up to those
reports. Then a device asks the server to delete the old lineage's blobs. A deletion request is
signed by the device key under `"tessera/delete/v1"` over the server id, vault id, the listed blob
ids and the author's sequence number. Requests are batched vault-wide: one goes with the next
upload after at least four resets have settled, or after the next sign-in following one, so a
single reset is not one request (the server still knows which blobs were uploaded together). The
server verifies the signature through `tessera-core` against a device current in the account's
device log, checks that the account is a current member of the vault in the membership log it
already verifies, and deletes only blobs in that vault. Deletion is soft for 30 days: the request
is stored in the vault's log, each device checks every listed blob against the superseded
lineages it knows, and if a current blob is listed, it warns, naming the signer, and uploads it
again from its own copy. The server may not comply, and an operator may keep backups.

**What the UI says:** "Erasing deleted text removes it from this note's history on your devices
as they sync; each device deletes its saved old history once every device has switched. Copies made before
now, by someone you shared the vault with, by a backup of this device, or kept by a server
operator, may still have it."

## Consequences

- `NoteHistory` will gain a lineage id in its saved form, a way to start a lineage from a reset
  record's genesis changes, text at given heads for the base check, and the carry-over diff. It
  will warn that a reset is due near a cap instead of only refusing at it. None of this is built.
- This record amends two accepted records without rewriting them. ADR 0004's server operations
  gain the verification of deletion requests (`crypto.md`'s list follows), and a stolen session
  token's reach is unchanged, since deletion needs a device signature. ADR 0003's consequence that
  "a device that never comes back blocks compaction until it is removed" now holds only for
  coordinated resets.
- The threat model changes with this record: the associated data gains the lineage id; the signed
  message kinds gain the reset, report and deletion labels; "Deleted text lives on in history",
  open question 7 and the A4 row point here; an A4 row covers forged, abused or racing resets and deletion requests; "What the server stores"
  gains reset records, reports and soft deletion; the "Signatures" primitive gains resets, reports,
  deletion requests and sign-in; "What the server inevitably sees" gains reports
  (which show each device's sync times), reset records and their snapshot sizes, and batched
  deletion requests.
- The crypto tests gain rejection cases for: an old-lineage blob replayed into a new lineage; a
  reset signed by a removed device or under an older generation; a reset whose text does not match
  its base; a replayed old reset or old-lineage report; a forged parent lineage under valid
  associated data; a reset whose kind does not hold; a deletion request from a non-member, or one
  listing a current-lineage blob (restored and warned about); a device switching from a losing
  sibling it already edited in; an all-zero lineage id losing to a reset whose base contains its own; carrying
  uploaded post-base edits (nothing lost); another device's post-base changes (dropped while their author is
  current, one conflict copy after removal, nothing duplicated); a device name holding `/` or `..` in a conflict copy; a cap-forced reset on
  a note whose text is near the cap (refused); and two concurrent resets every device resolves the
  same way.
- Conflict copies are a new kind of file in a vault; they appear only when a reset overlapped an
  edit.
- Revisit if forced resets are frequent in practice (make local edits fewer, larger changes), or
  if a future Automerge version can drop history itself.
