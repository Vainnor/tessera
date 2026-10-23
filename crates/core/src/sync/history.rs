//! One note's edit history, kept as an Automerge document (`docs/adr/0003-crdt-library.md`).
//!
//! Changes from other devices are untrusted input even after decryption, and Automerge 0.12
//! trusts what a change says about itself: which element an operation targets, what it replaces,
//! and where its operation numbers start. A change that lies about any of these can apply cleanly
//! and leave a history that no longer saves, loads or accepts edits. So before a change touches a
//! history this module checks it in full:
//!
//! - its size, that it is one uncompressed change chunk (Automerge decompresses without a limit),
//!   that it parses with panics caught, and that it re-encodes to exactly its own bytes;
//! - its author, no time, message or extra bytes, at most [`MAX_CHANGE_DEPS`] dependencies, and
//!   that it follows its author's previous change;
//! - that its operation numbers start right after the highest one in its dependencies;
//! - that every operation inserts one character into the note's text after the text's start or
//!   an element the change can see, or deletes one such element, with the predecessor an honest
//!   edit would carry. An element "the change can see" is one in its causal past or inserted
//!   earlier in the same change.
//!
//! A change whose dependencies have not arrived waits in a capped pending set owned by this
//! module, never in Automerge's internal queue, and is checked by itself when they do, so a
//! refusal names the change that caused it.
//!
//! Caught panics still reach the process's panic hook, so every app installs
//! [`crate::install_panic_hook`] first, which prints no message.
//!
//! Not covered yet, and required before sync ships (ADR 0003):
//!
//! - A change's columns are run-length encoded, so 135 bytes can declare any number of
//!   operations. [`MAX_CHANGE_OPS`] is checked after parsing, so a change at the limit can still
//!   use about 80 MB while it is checked. Checking the count before parsing would mean decoding
//!   Automerge's columns in Tessera.
//! - Memory: about 455 bytes per operation were measured at 1.9 million operations, so a history
//!   at [`MAX_HISTORY_OPS`] holds roughly 230 MB in Automerge, plus about 20 MB of element index
//!   and up to 10 MB of vector clocks. The caps apply per history, not across all open notes.
//! - Each call to [`NoteHistory::apply_change`] makes one Automerge `apply_changes` call (for the
//!   change and every waiting change it releases), and each call costs time that grows with the
//!   history (about 2.8 ms at 20,000 changes). A sync layer that hands over changes one by one
//!   still pays that per change; [`MAX_HISTORY_CHANGES`] bounds it.
//! - A change that still makes Automerge panic marks the history failed; after it is loaded
//!   again, the sync layer must not deliver that change again. The sync layer is not built.
//! - An author who sends different changes with the same sequence number to different devices
//!   splits them: each refuses the other's version. Detecting this belongs to the signed sync log.
//! - Each device applies the history caps to what it holds, so two devices near a cap can each
//!   accept their own change and refuse the other's, and then show different text. Judging a
//!   change by its causal past would give every device the same answer. With no history reset
//!   either, this module must not be wired into the app yet.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Mutex, MutexGuard};

use automerge::legacy::{ElementId, Key, ObjectId, OpId, OpType};
use automerge::transaction::{CommitOptions, Transactable};
use automerge::{
    ActorId, AutoCommit, Change, ChangeHash, ExpandedChange, LoadOptions, ObjId, ObjType,
    OnPartialLoad, ROOT, ReadDoc, ScalarValue, TextEncoding, Value, VerificationMode,
};

use super::DeviceId;
use crate::Error;
use crate::vault::MAX_NOTE_BYTES;

/// The largest change accepted from another device: a whole note plus room for its metadata.
pub const MAX_CHANGE_BYTES: u64 = MAX_NOTE_BYTES + 1024 * 1024;
/// The most operations one change may hold. This device splits larger edits into several changes.
pub const MAX_CHANGE_OPS: u64 = 100_000;
/// The most changes one change may depend on. An honest change depends on one change per device
/// that edited concurrently.
pub const MAX_CHANGE_DEPS: u64 = 64;
/// The most operations one history may hold, counting every character ever inserted or deleted.
/// A note that reaches it cannot sync further edits until its history is reset (ADR 0003), which
/// is not built.
pub const MAX_HISTORY_OPS: u64 = 500_000;
/// The most changes one history may hold. Automerge applies each change in time that grows with
/// the history, so this bounds the work a member can cause by sending many small changes.
pub const MAX_HISTORY_CHANGES: u64 = 20_000;
/// The most devices whose changes one history may hold, which bounds the per-change index.
pub const MAX_HISTORY_AUTHORS: u64 = 64;
// Every insert adds at most four bytes of text and one operation, so the operation cap keeps
// the text under the note limit, which `text_bytes` relies on.
const _: () = assert!(MAX_HISTORY_OPS * 4 <= MAX_NOTE_BYTES);

/// The most bytes of changes one history may hold, and the largest saved history accepted on load.
pub const MAX_SAVED_BYTES: u64 = 256 * 1024 * 1024;
/// The most changes that may wait for their dependencies in one history.
pub const MAX_PENDING_CHANGES: u64 = 10_000;
/// The most bytes of waiting changes in one history, counting 32 per dependency they list.
pub const MAX_PENDING_BYTES: u64 = 64 * 1024 * 1024;
/// The most waiting changes from one author, so one device cannot fill the pending set.
pub const MAX_PENDING_PER_AUTHOR: u64 = 1_000;
/// The most refused changes one history remembers. Past it, a resent change is checked again,
/// and refused again.
const MAX_REMEMBERED_REFUSALS: usize = 100_000;

/// The root key that holds the note's text, the only key a history may have.
const TEXT_KEY: &str = "text";
/// Positions count Unicode code points, set explicitly so a feature enabled elsewhere in the
/// build cannot change how positions are counted.
const ENCODING: TextEncoding = TextEncoding::UnicodeCodePoint;
/// The first bytes of every Automerge chunk, and the chunk type of an uncompressed change.
const CHUNK_MAGIC: [u8; 4] = [0x85, 0x6f, 0x4a, 0x83];
const CHUNK_TYPE_OFFSET: usize = 8;
const CHUNK_TYPE_CHANGE: u8 = 0x01;
/// The first byte of the saved form, so it can change without guessing.
const SAVED_VERSION: u8 = 1;
const HASH_BYTES: usize = 32;
const HASH_BYTES_U64: u64 = 32;

/// The edit history of one note: its text plus every change that produced it, so edits made on
/// different devices merge. The markdown file on disk stays the source of truth; this is the
/// sync state beside it.
///
/// Changes this device makes are committed without a time or message, so history reveals
/// neither. Encrypting and moving changes belongs to the sync layer, which is not built.
pub struct NoteHistory {
    device: DeviceId,
    state: Mutex<State>,
}

/// Only the device and the size of the history: the document holds note text, which must never
/// reach a log or a panic message.
impl fmt::Debug for NoteHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let changes = self.state.lock().map(|state| state.known.len()).ok();
        f.debug_struct("NoteHistory")
            .field("device", &self.device)
            .field("changes", &changes)
            .finish_non_exhaustive()
    }
}

/// What happened to a change from another device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    /// Whether the change is now part of the history. `false` means it waits for changes it
    /// depends on.
    pub applied: bool,
    /// Changes that were waiting and were refused once they could be checked, or because a change
    /// they depend on was refused (including one refused on arrival by an earlier call), each with
    /// its author, so the sync layer can blame the right device.
    pub refused_later: Vec<RefusedChange>,
}

/// A waiting change refused once it could be checked. Not every refusal shows the author did
/// wrong: a change refused because a limit of this history was reached, or because it depends on
/// another refused change, may come from an honest device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusedChange {
    /// The device whose change it was.
    pub author: DeviceId,
    /// Why it was refused, as in [`Error::SyncDataRefused`].
    pub reason: String,
}

/// A change from another device that passed every check that does not need its dependencies.
struct Checked {
    change: Change,
    author: DeviceId,
    /// Bytes of text the change inserts; deletions are not subtracted.
    inserted_bytes: u64,
    /// Dependencies not in the history when it arrived, while it waits.
    missing: Vec<ChangeHash>,
}

/// An operation id with its actor as an index into [`State::actors`].
type Element = (u64, usize);

struct State {
    doc: AutoCommit,
    text: ObjId,
    /// The text object's id, as operations name it.
    text_op: OpId,
    /// Heads when local changes were last taken; this device's changes after them are unsent.
    taken: Vec<ChangeHash>,
    /// Every change in the history, with the highest operation number in it and the newest
    /// sequence number of each actor in its causal past (indexed like [`State::actors`]).
    known: HashMap<ChangeHash, Past>,
    /// Every actor in the history, and for each, its changes in sequence order as ranges of
    /// operation numbers.
    actors: HashMap<ActorId, usize>,
    ranges: Vec<Vec<(u64, u64)>>,
    /// Every character ever inserted into the text, deleted or not.
    elements: HashSet<Element>,
    /// Every (actor, sequence number) in the history or waiting, with its change, so a second
    /// change that reuses one is refused.
    seen: HashMap<(Vec<u8>, u64), ChangeHash>,
    /// Changes refused after they arrived, so a resend is refused again.
    refused: HashSet<ChangeHash>,
    /// Changes waiting for their dependencies, and which waiting changes each missing change
    /// would release.
    pending: HashMap<ChangeHash, Checked>,
    waiting_on: HashMap<ChangeHash, Vec<ChangeHash>>,
    pending_bytes: u64,
    pending_by_author: HashMap<DeviceId, u64>,
    /// Waiting changes refused because a change they depend on was refused on arrival, reported
    /// with the next [`Applied`].
    unreported: Vec<RefusedChange>,
    /// Bytes and operations of changes in the history.
    history_bytes: u64,
    history_ops: u64,
    /// An upper bound on the text's size in bytes: exact after a local edit or a load, and raised
    /// by every insertion from elsewhere.
    text_bytes: u64,
    /// Set when a panic or a failed in-place apply may have left the document inconsistent: every
    /// call then fails until the history is loaded again from its last save.
    failed: bool,
}

/// What [`State::known`] keeps about a change.
struct Past {
    max_op: u64,
    clock: Vec<u64>,
}

impl NoteHistory {
    /// Starts a history from a note's current text. Every change since is relative to it, so
    /// two devices must never seed the same note independently: their histories would not merge.
    ///
    /// # Errors
    ///
    /// [`Error::NoteTooLarge`] over [`MAX_NOTE_BYTES`], and [`Error::SyncDataRefused`] if the text
    /// needs more than [`MAX_HISTORY_OPS`] operations.
    pub fn seed(device: DeviceId, markdown: &str) -> Result<Self, Error> {
        check_note_size(markdown)?;
        let mut doc = AutoCommit::new_with_encoding(ENCODING).with_actor(actor(device));
        let text = doc
            .put_object(ROOT, TEXT_KEY, ObjType::Text)
            .map_err(|_| refused("the history could not be started"))?;
        commit(&mut doc);
        let mut state = State::new(doc, text, Vec::new())?;
        state.splice(0, 0, markdown)?;
        state.text_bytes = markdown.len() as u64;
        Ok(Self::with_state(device, state))
    }

    /// Starts this device's copy of a history another device seeded, from that device's first
    /// change (the first of its [`NoteHistory::take_local_changes`]), which creates the empty
    /// note. Its later changes follow through [`NoteHistory::apply_change`].
    ///
    /// # Errors
    ///
    /// As [`NoteHistory::apply_change`], and [`Error::SyncDataRefused`] if the change is anything
    /// but the start of an empty note.
    pub fn join(device: DeviceId, seed: Vec<u8>, author: DeviceId) -> Result<Self, Error> {
        if author == device {
            return Err(refused("a change from this device that it never made"));
        }
        let (parsed, expanded) = parse_change(seed, author)?;
        let starts_a_note = parsed.deps().is_empty()
            && parsed.seq() == 1
            && parsed.start_op().get() == 1
            && matches!(expanded.operations.as_slice(), [op]
                if op.action == OpType::Make(ObjType::Text)
                    && op.obj == ObjectId::Root
                    && op.key == Key::Map(TEXT_KEY.into())
                    && !op.insert
                    && op.pred.is_empty());
        if !starts_a_note {
            return Err(refused("the change is not the start of a note"));
        }
        let mut doc = AutoCommit::new_with_encoding(ENCODING).with_actor(actor(device));
        guarded(|| doc.apply_changes([parsed]))?
            .map_err(|_| refused("the change does not apply"))?;
        let text = guarded(|| text_object(&doc))??;
        let heads = guarded(|| doc.get_heads())?;
        let state = State::new(doc, text, heads)?;
        state.check_shape()?;
        Ok(Self::with_state(device, state))
    }

    /// Loads a history saved by [`NoteHistory::save`] on this device. Never load one that came
    /// from elsewhere: loading decompresses without a limit.
    ///
    /// # Errors
    ///
    /// [`Error::SyncDataTooLarge`] over [`MAX_SAVED_BYTES`], [`Error::SyncDataRefused`] if it does
    /// not parse or does not hold exactly one note's text, and [`Error::NoteTooLarge`] if that
    /// text is over [`MAX_NOTE_BYTES`].
    pub fn load(device: DeviceId, saved: &[u8]) -> Result<Self, Error> {
        if saved.len() as u64 > MAX_SAVED_BYTES {
            return Err(Error::SyncDataTooLarge {
                limit_bytes: MAX_SAVED_BYTES,
            });
        }
        let malformed = || refused("the saved history is malformed");
        let (taken, document) = split_saved(saved).ok_or_else(malformed)?;
        let options = LoadOptions::new()
            .on_partial_load(OnPartialLoad::Error)
            .verification_mode(VerificationMode::Check)
            .text_encoding(ENCODING);
        let doc = guarded(|| {
            AutoCommit::load_with_options(document, options)
                .map(|doc| doc.with_actor(actor(device)))
        })?
        .map_err(|_| malformed())?;
        let text = guarded(|| text_object(&doc))??;
        let mut state = State::new(doc, text, taken)?;
        if state
            .taken
            .iter()
            .any(|hash| !state.known.contains_key(hash))
        {
            return Err(malformed());
        }
        state.check_shape()?;
        state.text_bytes = state.markdown()?.len() as u64;
        Ok(Self::with_state(device, state))
    }

    /// The history for this device's own storage: a version byte, the heads when local changes
    /// were last taken, and the document in Automerge's compressed form. Changes still waiting
    /// for their dependencies are not saved; the sync layer fetches them again.
    ///
    /// # Errors
    ///
    /// [`Error::SyncDataRefused`] if the history failed (see [`NoteHistory::set_markdown`]); a
    /// panic while saving marks it failed.
    pub fn save(&self) -> Result<Vec<u8>, Error> {
        let mut state = self.lock()?;
        let document = state.guarded_mut(|state| state.doc.save())?;
        let count = u16::try_from(state.taken.len())
            .map_err(|_| refused("the history has too many heads"))?;
        let mut saved = Vec::with_capacity(3 + state.taken.len() * HASH_BYTES + document.len());
        saved.push(SAVED_VERSION);
        saved.extend_from_slice(&count.to_be_bytes());
        for hash in &state.taken {
            saved.extend_from_slice(hash.as_ref());
        }
        saved.extend_from_slice(&document);
        Ok(saved)
    }

    /// The note's text as the history has it.
    ///
    /// # Errors
    ///
    /// [`Error::SyncDataRefused`] if the history failed or the text cannot be read.
    pub fn markdown(&self) -> Result<String, Error> {
        let state = self.lock()?;
        state.markdown()
    }

    /// Records the note's new text: the shortest splice between the old text and the new, so
    /// concurrent edits elsewhere in the note merge, split into changes of at most
    /// [`MAX_CHANGE_OPS`] operations.
    ///
    /// # Errors
    ///
    /// [`Error::NoteTooLarge`] over [`MAX_NOTE_BYTES`], and [`Error::SyncDataRefused`] if the
    /// edit would take the history past [`MAX_HISTORY_OPS`]; in both cases the history is
    /// unchanged. [`Error::SyncDataRefused`] also if the history failed, or if the edit failed
    /// partway: the history then refuses every call until it is loaded again from its last save.
    pub fn set_markdown(&self, markdown: &str) -> Result<(), Error> {
        check_note_size(markdown)?;
        let mut state = self.lock()?;
        let current = state.markdown()?;
        let Some((position, deleted, inserted)) = splice_range(&current, markdown) else {
            return Ok(());
        };
        state.splice(position, deleted, inserted)?;
        state.text_bytes = markdown.len() as u64;
        Ok(())
    }

    /// This device's changes that it has not handed out yet, each as one uncompressed change for
    /// the sync layer to encrypt and send. Save the history durably before sending them: a
    /// device that sends a change and then loses it would make a different change with the same
    /// sequence number, which every other device refuses.
    ///
    /// # Errors
    ///
    /// [`Error::SyncDataRefused`] if the history failed; a panic here marks it failed.
    pub fn take_local_changes(&self) -> Result<Vec<Vec<u8>>, Error> {
        let mut state = self.lock()?;
        let own = actor(self.device);
        let (changes, heads) = state.guarded_mut(|state| {
            let taken = state.taken.clone();
            (state.doc.get_changes(&taken), state.doc.get_heads())
        })?;
        let ours = changes
            .into_iter()
            .filter(|change| change.actor_id() == &own)
            .map(|change| change.raw_bytes().to_vec())
            .collect();
        state.taken = heads;
        Ok(ours)
    }

    /// Applies one change made by `author`. The caller has already authenticated the change and
    /// learned its author from the signature; this checks that the change agrees.
    ///
    /// A change whose dependencies have not arrived waits, and applies by itself, with every
    /// check, once they have. A change the history already holds or is already waiting is
    /// accepted again and changes nothing.
    ///
    /// # Errors
    ///
    /// [`Error::SyncDataTooLarge`] over [`MAX_CHANGE_BYTES`], or if the pending set would grow
    /// past [`MAX_PENDING_BYTES`], or the history past [`MAX_SAVED_BYTES`].
    /// [`Error::NoteTooLarge`] if the text would grow past [`MAX_NOTE_BYTES`].
    /// [`Error::SyncDataRefused`] if any check in the module documentation fails, if too many
    /// changes are waiting ([`MAX_PENDING_CHANGES`], [`MAX_PENDING_PER_AUTHOR`]), if it claims to
    /// come from this device, takes the history past [`MAX_HISTORY_OPS`], or was refused before.
    /// The history is unchanged after any of these. [`Error::NoteTooLarge`] is a backstop: the
    /// operation cap keeps the text under the limit. If Automerge still refuses or panics on the
    /// change and the waiting changes it released, which go to it together, the history is marked
    /// failed and this returns that failure without saying which change caused it.
    pub fn apply_change(&self, change: Vec<u8>, author: DeviceId) -> Result<Applied, Error> {
        let (parsed, expanded) = parse_change(change, author)?;
        let mut state = self.lock()?;
        let hash = parsed.hash();
        if state.refused.contains(&hash) {
            return Err(refused("the change was refused before"));
        }
        match state.seen.get(&key_of(&parsed)) {
            Some(seen) if *seen == hash => {
                return Ok(Applied {
                    applied: state.known.contains_key(&hash),
                    refused_later: Vec::new(),
                });
            }
            Some(_) => return Err(refused("the change reuses a sequence number")),
            None => {}
        }
        if author == self.device {
            return Err(refused("a change from this device that it never made"));
        }
        let inserted_bytes = if parsed.deps().iter().any(|dep| state.refused.contains(dep)) {
            Err(refused("it depends on a refused change"))
        } else {
            state.check_operations(&parsed, &expanded)
        };
        let inserted_bytes = match inserted_bytes {
            Ok(bytes) => bytes,
            Err(error) => {
                state.refuse_on_arrival(&parsed, author, &error);
                return Err(error);
            }
        };
        let checked = Checked {
            change: parsed,
            author,
            inserted_bytes,
            missing: Vec::new(),
        };
        if !state.ready(&checked.change) {
            state.hold(checked)?;
            return Ok(Applied {
                applied: false,
                refused_later: std::mem::take(&mut state.unreported),
            });
        }
        let mut batch = Vec::new();
        if let Err(error) = state.admit(&checked, Some(&expanded), &mut batch) {
            state.refuse_on_arrival(&checked.change, author, &error);
            return Err(error);
        }
        let mut refused_later = std::mem::take(&mut state.unreported);
        refused_later.extend(state.release(hash, &mut batch));
        state.apply_batch(batch)?;
        Ok(Applied {
            applied: true,
            refused_later,
        })
    }

    fn with_state(device: DeviceId, state: State) -> Self {
        Self {
            device,
            state: Mutex::new(state),
        }
    }

    fn lock(&self) -> Result<MutexGuard<'_, State>, Error> {
        // Automerge calls on a history run inside `guarded`, so a poisoned lock means a panic
        // somewhere else while it was held; treat the history as failed rather than trust it.
        let state = self.state.lock().map_err(|_| failed())?;
        if state.failed {
            return Err(failed());
        }
        Ok(state)
    }
}

impl State {
    /// Indexes every change of `doc`. Fails if the changes do not form per-author chains with
    /// operation numbers that follow their dependencies, as every change this module admits does.
    fn new(mut doc: AutoCommit, text: ObjId, taken: Vec<ChangeHash>) -> Result<Self, Error> {
        let ObjId::Id(counter, text_actor, _) = &text else {
            return Err(refused("the history does not hold one note's text"));
        };
        let text_op = OpId(*counter, text_actor.clone());
        let changes = guarded(|| doc.get_changes(&[]))?;
        let mut state = Self {
            doc,
            text,
            text_op,
            taken,
            known: HashMap::new(),
            actors: HashMap::new(),
            ranges: Vec::new(),
            elements: HashSet::new(),
            seen: HashMap::new(),
            refused: HashSet::new(),
            pending: HashMap::new(),
            waiting_on: HashMap::new(),
            pending_bytes: 0,
            pending_by_author: HashMap::new(),
            unreported: Vec::new(),
            history_bytes: 0,
            history_ops: 0,
            text_bytes: 0,
            failed: false,
        };
        let malformed = || refused("the saved history is malformed");
        for change in causal_order(changes).ok_or_else(malformed)? {
            if !state.follows_its_author(&change)
                || state.start_op(&change) != Some(change.start_op().get())
            {
                return Err(malformed());
            }
            let expanded = guarded(|| change.decode())?;
            state.record(&change, &expanded)?;
        }
        Ok(state)
    }

    fn markdown(&self) -> Result<String, Error> {
        guarded(|| self.doc.text(&self.text))?.map_err(|_| refused("the history could not be read"))
    }

    /// Runs `f`, marking the history failed if it panics.
    fn guarded_mut<T>(&mut self, f: impl FnOnce(&mut Self) -> T) -> Result<T, Error> {
        let outcome = guarded(|| f(self));
        if outcome.is_err() {
            self.failed = true;
        }
        outcome
    }

    /// Adds a change that is now in the document to the indexes.
    fn record(&mut self, change: &Change, expanded: &ExpandedChange) -> Result<(), Error> {
        let mut clock = self
            .clock_of(change.deps())
            .ok_or_else(|| refused("the change does not apply"))?;
        let author = self.actor_index(change.actor_id());
        if clock.len() <= author {
            clock.resize(author + 1, 0);
        }
        if let Some(own) = clock.get_mut(author) {
            *own = change.seq();
        }
        let start = change.start_op().get();
        let max_op = start + change.len() as u64 - 1;
        if let Some(ranges) = self.ranges.get_mut(author) {
            ranges.push((start, max_op));
        }
        for (op, counter) in expanded.operations.iter().zip(start..) {
            if op.insert {
                self.elements.insert((counter, author));
            }
        }
        self.known.insert(change.hash(), Past { max_op, clock });
        self.seen.insert(key_of(change), change.hash());
        self.history_bytes += change.raw_bytes().len() as u64;
        self.history_ops += change.len() as u64;
        Ok(())
    }

    fn actor_index(&mut self, actor: &ActorId) -> usize {
        if let Some(index) = self.actors.get(actor) {
            return *index;
        }
        let index = self.ranges.len();
        self.actors.insert(actor.clone(), index);
        self.ranges.push(Vec::new());
        index
    }

    /// The newest sequence number of each actor in the causal past of `deps`, or `None` if one
    /// is not in the history.
    fn clock_of(&self, deps: &[ChangeHash]) -> Option<Vec<u64>> {
        let mut clock = Vec::new();
        for dep in deps {
            let past = self.known.get(dep)?;
            if clock.len() < past.clock.len() {
                clock.resize(past.clock.len(), 0);
            }
            for (mine, theirs) in clock.iter_mut().zip(&past.clock) {
                *mine = (*mine).max(*theirs);
            }
        }
        Some(clock)
    }

    /// The operation number a change with these dependencies must start at, or `None` if one is
    /// not in the history.
    fn start_op(&self, change: &Change) -> Option<u64> {
        let mut highest = 0;
        for dep in change.deps() {
            highest = highest.max(self.known.get(dep)?.max_op);
        }
        Some(highest + 1)
    }

    /// Whether `change` is its author's first, or depends on its author's previous one, which is
    /// in the history.
    fn follows_its_author(&self, change: &Change) -> bool {
        let count = self
            .actors
            .get(change.actor_id())
            .and_then(|index| self.ranges.get(*index))
            .map_or(0, Vec::len);
        if change.seq() != count as u64 + 1 {
            return false;
        }
        change.seq() == 1
            || self
                .seen
                .get(&(change.actor_id().to_bytes().to_vec(), change.seq() - 1))
                .is_some_and(|previous| change.deps().contains(previous))
    }

    /// Deletes `deleted` code points at `position` and inserts `inserted` there, committing at
    /// most [`MAX_CHANGE_OPS`] operations per change. Refused before the first change if the
    /// history would pass [`MAX_HISTORY_OPS`]; a failure after that marks the history failed.
    fn splice(&mut self, position: usize, deleted: usize, inserted: &str) -> Result<(), Error> {
        let ops = (deleted + inserted.chars().count()) as u64;
        if self.history_ops + ops > MAX_HISTORY_OPS {
            return Err(refused("the history holds too many operations"));
        }
        let step = usize::try_from(MAX_CHANGE_OPS).map_err(|_| refused("the edit is too large"))?;
        let changes = deleted.div_ceil(step) + inserted.chars().count().div_ceil(step);
        if self.known.len() + changes > usize::try_from(MAX_HISTORY_CHANGES).unwrap_or(usize::MAX) {
            return Err(refused("the history holds too many changes"));
        }
        let own = self.doc.get_actor().clone();
        if !self.actors.contains_key(&own) && self.actors.len() as u64 >= MAX_HISTORY_AUTHORS {
            return Err(refused("the history holds changes from too many devices"));
        }
        let result = self.splice_in_steps(position, deleted, inserted, step);
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    fn splice_in_steps(
        &mut self,
        position: usize,
        deleted: usize,
        inserted: &str,
        step: usize,
    ) -> Result<(), Error> {
        let mut left = deleted;
        while left > 0 {
            let count = left.min(step);
            self.edit(position, count, "")?;
            left -= count;
        }
        let mut at = position;
        let mut rest = inserted;
        while !rest.is_empty() {
            let cut = rest
                .char_indices()
                .nth(step)
                .map_or(rest.len(), |(index, _)| index);
            let (chunk, tail) = rest.split_at(cut);
            self.edit(at, 0, chunk)?;
            at += chunk.chars().count();
            rest = tail;
        }
        Ok(())
    }

    /// One committed change of this device's own edit.
    fn edit(&mut self, position: usize, deleted: usize, inserted: &str) -> Result<(), Error> {
        let deleted = isize::try_from(deleted).map_err(|_| refused("the edit is too large"))?;
        let text = self.text.clone();
        let (change, expanded) = guarded(|| {
            self.doc
                .splice_text(&text, position, deleted, inserted)
                .ok()?;
            let hash = commit(&mut self.doc)?;
            let change = self.doc.get_change_by_hash(&hash)?;
            let expanded = change.decode();
            Some((change, expanded))
        })?
        .ok_or_else(|| refused("the edit could not be recorded"))?;
        self.record(&change, &expanded)
    }

    /// The checks on a change's operations that need nothing but the change: each inserts one
    /// character into the note's text with no predecessor, or deletes one element of it naming
    /// that element as its only predecessor, and no element is deleted twice. Returns how many
    /// bytes of text the change inserts.
    fn check_operations(&self, change: &Change, expanded: &ExpandedChange) -> Result<u64, Error> {
        if change.deps().len() as u64 > MAX_CHANGE_DEPS {
            return Err(refused("the change has too many dependencies"));
        }
        let not_text = || refused("the change does more than edit the note's text");
        let mut inserted = 0;
        let mut deleted = HashSet::new();
        for op in &expanded.operations {
            if !matches!(&op.obj, ObjectId::Id(id) if *id == self.text_op) {
                return Err(not_text());
            }
            let Key::Seq(element) = &op.key else {
                return Err(not_text());
            };
            match &op.action {
                OpType::Put(ScalarValue::Str(s))
                    if op.insert && s.chars().count() == 1 && op.pred.is_empty() =>
                {
                    inserted += s.len() as u64;
                }
                OpType::Delete if !op.insert => {
                    let ElementId::Id(target) = element else {
                        return Err(not_text());
                    };
                    let only_target = op.pred.len() == 1 && op.pred.get(0) == Some(target);
                    if !only_target || !deleted.insert(target) {
                        return Err(not_text());
                    }
                }
                _ => return Err(not_text()),
            }
        }
        Ok(inserted)
    }

    /// The checks on a change's operations that need its dependencies: every element it inserts
    /// after or deletes is in its causal past, or was inserted earlier in the same change.
    fn check_references(&self, change: &Change, expanded: &ExpandedChange) -> Result<(), Error> {
        let not_text = || refused("the change does more than edit the note's text");
        let clock = self.clock_of(change.deps()).ok_or_else(not_text)?;
        let start = change.start_op().get();
        let own_inserts: HashSet<u64> = expanded
            .operations
            .iter()
            .zip(start..)
            .filter(|(op, _)| op.insert)
            .map(|(_, counter)| counter)
            .collect();
        for (op, counter) in expanded.operations.iter().zip(start..) {
            let target = match &op.key {
                Key::Seq(ElementId::Head) if op.insert => continue,
                Key::Seq(ElementId::Id(target)) => target,
                _ => return Err(not_text()),
            };
            let earlier_in_change = target.actor() == change.actor_id()
                && target.0 < counter
                && own_inserts.contains(&target.0);
            if !earlier_in_change && !self.visible(&clock, target) {
                return Err(not_text());
            }
        }
        Ok(())
    }

    /// Whether `element` is a character of the text inserted by a change within `clock`.
    fn visible(&self, clock: &[u64], element: &OpId) -> bool {
        let Some(&actor) = self.actors.get(element.actor()) else {
            return false;
        };
        if !self.elements.contains(&(element.0, actor)) {
            return false;
        }
        let Some(ranges) = self.ranges.get(actor) else {
            return false;
        };
        let index = ranges.partition_point(|(start, _)| *start <= element.0);
        let within = index
            .checked_sub(1)
            .and_then(|i| ranges.get(i))
            .is_some_and(|(_, max_op)| element.0 <= *max_op);
        within && (index as u64) <= clock.get(actor).copied().unwrap_or(0)
    }

    /// Whether every change `change` depends on is in the history.
    fn ready(&self, change: &Change) -> bool {
        change.deps().iter().all(|dep| self.known.contains_key(dep))
    }

    fn pending_cost(change: &Change) -> u64 {
        change.raw_bytes().len() as u64 + HASH_BYTES_U64 * change.deps().len() as u64
    }

    fn hold(&mut self, mut checked: Checked) -> Result<(), Error> {
        let bytes = Self::pending_cost(&checked.change);
        let from_author = self
            .pending_by_author
            .get(&checked.author)
            .copied()
            .unwrap_or(0);
        if self.pending.len() as u64 >= MAX_PENDING_CHANGES || from_author >= MAX_PENDING_PER_AUTHOR
        {
            return Err(refused("too many changes are waiting"));
        }
        if self.pending_bytes + bytes > MAX_PENDING_BYTES {
            return Err(Error::SyncDataTooLarge {
                limit_bytes: MAX_PENDING_BYTES,
            });
        }
        let hash = checked.change.hash();
        checked.missing = checked
            .change
            .deps()
            .iter()
            .filter(|dep| !self.known.contains_key(*dep))
            .copied()
            .collect();
        for dep in &checked.missing {
            self.waiting_on.entry(*dep).or_default().push(hash);
        }
        self.seen.insert(key_of(&checked.change), hash);
        self.pending_bytes += bytes;
        *self.pending_by_author.entry(checked.author).or_insert(0) += 1;
        self.pending.insert(hash, checked);
        Ok(())
    }

    /// Takes a change out of the pending set and every index that points at it.
    fn unhold(&mut self, hash: &ChangeHash) -> Option<Checked> {
        let checked = self.pending.remove(hash)?;
        self.pending_bytes -= Self::pending_cost(&checked.change);
        if let Some(count) = self.pending_by_author.get_mut(&checked.author) {
            *count -= 1;
        }
        for dep in &checked.missing {
            if let Some(waiting) = self.waiting_on.get_mut(dep) {
                waiting.retain(|other| other != hash);
                if waiting.is_empty() {
                    self.waiting_on.remove(dep);
                }
            }
        }
        Some(checked)
    }

    /// Runs every check that needs `checked`'s dependencies, then indexes it and adds it to
    /// `batch` for [`State::apply_batch`]. The indexes run ahead of the document until the batch
    /// is applied, so later changes in the batch are checked against this one.
    fn admit(
        &mut self,
        checked: &Checked,
        expanded: Option<&ExpandedChange>,
        batch: &mut Vec<Change>,
    ) -> Result<(), Error> {
        let change = &checked.change;
        if !self.follows_its_author(change) {
            return Err(refused(
                "the change does not follow its author's previous one",
            ));
        }
        if self.start_op(change) != Some(change.start_op().get()) {
            return Err(refused(
                "the change's operation numbers do not follow its dependencies",
            ));
        }
        if self.history_ops + change.len() as u64 > MAX_HISTORY_OPS {
            return Err(refused("the history holds too many operations"));
        }
        if self.known.len() as u64 >= MAX_HISTORY_CHANGES {
            return Err(refused("the history holds too many changes"));
        }
        if !self.actors.contains_key(change.actor_id())
            && self.actors.len() as u64 >= MAX_HISTORY_AUTHORS
        {
            return Err(refused("the history holds changes from too many devices"));
        }
        if self.history_bytes + change.raw_bytes().len() as u64 > MAX_SAVED_BYTES {
            return Err(Error::SyncDataTooLarge {
                limit_bytes: MAX_SAVED_BYTES,
            });
        }
        if self.text_bytes + checked.inserted_bytes > MAX_NOTE_BYTES {
            return Err(Error::NoteTooLarge {
                limit_bytes: MAX_NOTE_BYTES,
            });
        }
        let decoded;
        let expanded = if let Some(expanded) = expanded {
            expanded
        } else {
            decoded = guarded(|| change.decode())?;
            &decoded
        };
        self.check_references(change, expanded)?;
        self.record(change, expanded)?;
        self.text_bytes += checked.inserted_bytes;
        batch.push(change.clone());
        Ok(())
    }

    /// Gives Automerge every admitted change in one call: applying them one call at a time costs
    /// time that grows with the history for each. If Automerge still refuses or panics, the
    /// indexes no longer match the document, so the history is marked failed.
    fn apply_batch(&mut self, batch: Vec<Change>) -> Result<(), Error> {
        if batch.is_empty() {
            return Ok(());
        }
        let applied = self.guarded_mut(|state| state.doc.apply_changes(batch))?;
        if applied.is_err() {
            self.failed = true;
            return Err(failed());
        }
        Ok(())
    }

    /// Applies every waiting change that `arrived` makes ready, and the ones those make ready in
    /// turn. A refused change is reported with its author, and so is every waiting change that
    /// depends on it.
    fn release(&mut self, arrived: ChangeHash, batch: &mut Vec<Change>) -> Vec<RefusedChange> {
        let mut refused_later = Vec::new();
        let mut arrivals = vec![arrived];
        while let Some(hash) = arrivals.pop() {
            for waiting in self.waiting_on.remove(&hash).unwrap_or_default() {
                let ready = self
                    .pending
                    .get(&waiting)
                    .is_some_and(|checked| self.ready(&checked.change));
                if !ready {
                    continue;
                }
                let Some(checked) = self.unhold(&waiting) else {
                    continue;
                };
                match self.admit(&checked, None, batch) {
                    Ok(()) => arrivals.push(waiting),
                    Err(error) => self.refuse(
                        &checked.change,
                        checked.author,
                        &error.to_string(),
                        &mut refused_later,
                    ),
                }
            }
        }
        refused_later
    }

    /// Records a change refused by the call that delivered it. The caller already has the error,
    /// so only the waiting changes refused with it are kept for the next [`Applied`].
    fn refuse_on_arrival(&mut self, change: &Change, author: DeviceId, error: &Error) {
        let mut out = Vec::new();
        self.refuse(change, author, &error.to_string(), &mut out);
        self.unreported.extend(out.into_iter().skip(1));
    }

    /// Records `change` as refused, then every waiting change that depends on it.
    fn refuse(
        &mut self,
        change: &Change,
        author: DeviceId,
        reason: &str,
        out: &mut Vec<RefusedChange>,
    ) {
        let mut queue = vec![(change.hash(), key_of(change), author, reason.to_owned())];
        while let Some((hash, key, author, reason)) = queue.pop() {
            if self.refused.len() < MAX_REMEMBERED_REFUSALS {
                self.refused.insert(hash);
            }
            if self.seen.get(&key) == Some(&hash) {
                self.seen.remove(&key);
            }
            out.push(RefusedChange { author, reason });
            for dependent in self.waiting_on.remove(&hash).unwrap_or_default() {
                if let Some(waiting) = self.unhold(&dependent) {
                    queue.push((
                        dependent,
                        key_of(&waiting.change),
                        waiting.author,
                        refused("it depends on a refused change").to_string(),
                    ));
                }
            }
        }
    }

    /// The history holds exactly one note: a root with only the text key, plain characters in
    /// the text, no marks, and no more than [`MAX_NOTE_BYTES`] of it. Used where a whole
    /// document arrives at once; single changes are checked operation by operation instead.
    fn check_shape(&self) -> Result<(), Error> {
        let shape = || refused("the history does not hold one note's text");
        let (text, plain, unmarked) = guarded(|| {
            let text = text_object(&self.doc);
            let plain = self.doc.values(&self.text).all(|(value, _)| {
                matches!(value, Value::Scalar(s) if matches!(s.as_ref(), ScalarValue::Str(_)))
            });
            let unmarked = self
                .doc
                .marks(&self.text)
                .is_ok_and(|marks| marks.is_empty());
            (text, plain, unmarked)
        })?;
        if text? != self.text || !plain || !unmarked {
            return Err(shape());
        }
        check_note_size(&self.markdown()?)
    }
}

/// `changes` if every change follows the changes it depends on, as `get_changes` returns them,
/// or `None` if one does not. One pass, so loading stays linear in the number of changes.
fn causal_order(changes: Vec<Change>) -> Option<Vec<Change>> {
    let mut placed = HashSet::with_capacity(changes.len());
    for change in &changes {
        if !change.deps().iter().all(|dep| placed.contains(dep)) {
            return None;
        }
        placed.insert(change.hash());
    }
    Some(changes)
}

/// The checks every change from another device passes before it touches a history: size, a
/// single uncompressed chunk, a clean parse, the canonical encoding, the operation limit, the
/// author's id, and no time, message or extra bytes. Returns the change and its operations.
fn parse_change(change: Vec<u8>, author: DeviceId) -> Result<(Change, ExpandedChange), Error> {
    if change.len() as u64 > MAX_CHANGE_BYTES {
        return Err(Error::SyncDataTooLarge {
            limit_bytes: MAX_CHANGE_BYTES,
        });
    }
    let single_change = change.get(..CHUNK_MAGIC.len()) == Some(&CHUNK_MAGIC[..])
        && change.get(CHUNK_TYPE_OFFSET) == Some(&CHUNK_TYPE_CHANGE);
    if !single_change {
        return Err(refused("not a single uncompressed change"));
    }
    // The parser also refuses bytes after the first chunk, so one blob is one change.
    let parsed =
        guarded(|| Change::from_bytes(change))?.map_err(|_| refused("the change is malformed"))?;
    if parsed.len() as u64 > MAX_CHANGE_OPS {
        return Err(refused("the change holds too many operations"));
    }
    if parsed.is_empty() {
        return Err(refused("the change holds no operations"));
    }
    if parsed.actor_id() != &actor(author) {
        return Err(refused("the change was not made by its author"));
    }
    if parsed.timestamp() != 0 || parsed.message().is_some() {
        return Err(refused("the change carries a time or message"));
    }
    // Automerge keeps extra bytes, including its own author footer, in the history of every
    // device; Tessera never writes any.
    if !parsed.extra_bytes().is_empty() {
        return Err(refused("the change carries extra bytes"));
    }
    // Automerge accepts some encodings (an unknown column, for one) that it writes back
    // differently when saving, which changes the hash and makes the saved history fail to load.
    let expanded = guarded(|| parsed.decode())?;
    let canonical = guarded(|| Change::from(expanded.clone()).raw_bytes() == parsed.raw_bytes())?;
    if !canonical {
        return Err(refused("the change is not in canonical form"));
    }
    Ok((parsed, expanded))
}

/// The id of the root's text object, if the root holds that and nothing else.
fn text_object(doc: &AutoCommit) -> Result<ObjId, Error> {
    let shape = || refused("the history does not hold one note's text");
    if doc.keys(ROOT).ne([TEXT_KEY]) {
        return Err(shape());
    }
    let mut values = doc.get_all(ROOT, TEXT_KEY).map_err(|_| shape())?;
    match (values.pop(), values.is_empty()) {
        (Some((Value::Object(ObjType::Text), id)), true) => Ok(id),
        _ => Err(shape()),
    }
}

/// Splits the saved form into the taken heads and the Automerge document.
fn split_saved(saved: &[u8]) -> Option<(Vec<ChangeHash>, &[u8])> {
    let (&version, rest) = saved.split_first()?;
    if version != SAVED_VERSION {
        return None;
    }
    let (count, rest) = rest.split_first_chunk::<2>()?;
    let count = usize::from(u16::from_be_bytes(*count));
    let (heads, document) = rest.split_at_checked(count * HASH_BYTES)?;
    let taken = heads
        .as_chunks::<HASH_BYTES>()
        .0
        .iter()
        .map(|bytes| ChangeHash(*bytes))
        .collect();
    Some((taken, document))
}

/// The one splice that turns `old` into `new`, keeping their common start and end: the position
/// and number of code points to delete, and the text to insert. `None` if they are equal.
fn splice_range<'a>(old: &str, new: &'a str) -> Option<(usize, usize, &'a str)> {
    if old == new {
        return None;
    }
    let (prefix_chars, prefix_bytes) = old
        .chars()
        .zip(new.chars())
        .take_while(|(a, b)| a == b)
        .fold((0, 0), |(n, bytes), (c, _)| (n + 1, bytes + c.len_utf8()));
    let old_rest = old.get(prefix_bytes..)?;
    let new_rest = new.get(prefix_bytes..)?;
    let (suffix_chars, suffix_bytes) = old_rest
        .chars()
        .rev()
        .zip(new_rest.chars().rev())
        .take_while(|(a, b)| a == b)
        .fold((0, 0), |(n, bytes), (c, _)| (n + 1, bytes + c.len_utf8()));
    let deleted = old_rest.chars().count() - suffix_chars;
    let inserted = new_rest.get(..new_rest.len() - suffix_bytes)?;
    Some((prefix_chars, deleted, inserted))
}

fn key_of(change: &Change) -> (Vec<u8>, u64) {
    (change.actor_id().to_bytes().to_vec(), change.seq())
}

fn actor(device: DeviceId) -> ActorId {
    ActorId::from(device.to_bytes().as_slice())
}

/// Commits pending operations with a time of 0 and no message.
fn commit(doc: &mut AutoCommit) -> Option<ChangeHash> {
    doc.commit_with(CommitOptions::default().with_time(0))
}

fn check_note_size(text: &str) -> Result<(), Error> {
    if text.len() as u64 > MAX_NOTE_BYTES {
        return Err(Error::NoteTooLarge {
            limit_bytes: MAX_NOTE_BYTES,
        });
    }
    Ok(())
}

fn refused(reason: &str) -> Error {
    Error::SyncDataRefused {
        reason: reason.to_owned(),
    }
}

fn failed() -> Error {
    refused("the history failed and must be loaded again")
}

/// Runs Automerge code over data that may hold other devices' changes, turning a panic into an
/// error. Release builds unwind on panic for this (`Cargo.toml`); an abort, a stack overflow or
/// running out of memory is not caught, which is why sizes are limited first.
fn guarded<T>(f: impl FnOnce() -> T) -> Result<T, Error> {
    panic::catch_unwind(AssertUnwindSafe(f)).map_err(|_| refused("the decoder failed"))
}

#[cfg(test)]
mod tests;
