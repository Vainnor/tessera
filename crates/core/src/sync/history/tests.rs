use automerge::legacy::{ElementId, Key, OpId};
use automerge::marks::{ExpandMark, Mark};
use automerge::transaction::{CommitOptions, Transactable};
use automerge::{ActorId, AutoCommit, ROOT, ReadDoc};
use sha2::{Digest, Sha256};

use super::*;

fn device(n: u8) -> DeviceId {
    DeviceId::parse(&format!("{n:032x}")).unwrap()
}

fn refusal(result: Result<impl std::fmt::Debug, Error>) -> String {
    match result {
        Err(Error::SyncDataRefused { reason }) => reason,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// Device 1 seeds a note and device 2 joins it.
fn shared(markdown: &str) -> (NoteHistory, NoteHistory) {
    let a = NoteHistory::seed(device(1), markdown).unwrap();
    let b = joined(device(2), &a.take_local_changes().unwrap(), device(1));
    (a, b)
}

/// `device`'s copy of a history built from `changes`, all made by `author`.
fn joined(device: DeviceId, changes: &[Vec<u8>], author: DeviceId) -> NoteHistory {
    let history = NoteHistory::join(device, changes[0].clone(), author).unwrap();
    for change in &changes[1..] {
        history.apply_change(change.clone(), author).unwrap();
    }
    history
}

/// A raw Automerge document holding `history`'s state, acting as `as_device`, for building
/// changes a well-behaved device would never send.
fn forger(history: &NoteHistory, as_device: u8) -> (AutoCommit, automerge::ObjId) {
    let doc = AutoCommit::load(&document(history))
        .unwrap()
        .with_actor(ActorId::from(device(as_device).to_bytes().as_slice()));
    let text = doc.get(ROOT, TEXT_KEY).unwrap().unwrap().1;
    (doc, text)
}

/// The Automerge document inside the saved form.
fn document(history: &NoteHistory) -> Vec<u8> {
    let saved = history.save().unwrap();
    let count = usize::from(u16::from_be_bytes([saved[1], saved[2]]));
    saved[3 + count * HASH_BYTES..].to_vec()
}

fn last_change(doc: &mut AutoCommit) -> Vec<u8> {
    let heads = doc.get_heads();
    doc.get_change_by_hash(&heads[0])
        .unwrap()
        .raw_bytes()
        .to_vec()
}

#[test]
fn seed_holds_the_markdown() {
    let history = NoteHistory::seed(device(1), "# Plan\n\nTiles 🧩\n").unwrap();
    assert_eq!(history.markdown().unwrap(), "# Plan\n\nTiles 🧩\n");
}

#[test]
fn seed_refuses_a_note_over_the_limit() {
    let big = "a".repeat(usize::try_from(MAX_NOTE_BYTES).unwrap() + 1);
    assert_eq!(
        NoteHistory::seed(device(1), &big).map(|_| ()),
        Err(Error::NoteTooLarge {
            limit_bytes: MAX_NOTE_BYTES
        })
    );
}

#[test]
fn set_markdown_records_the_new_text() {
    let history = NoteHistory::seed(device(1), "one two three").unwrap();
    history.set_markdown("one 2 three").unwrap();
    assert_eq!(history.markdown().unwrap(), "one 2 three");
}

#[test]
fn set_markdown_with_the_same_text_records_nothing() {
    let history = NoteHistory::seed(device(1), "same").unwrap();
    let _ = history.take_local_changes().unwrap();
    history.set_markdown("same").unwrap();
    assert!(history.take_local_changes().unwrap().is_empty());
}

#[test]
fn set_markdown_refuses_a_note_over_the_limit_and_keeps_the_text() {
    let history = NoteHistory::seed(device(1), "small").unwrap();
    let big = "a".repeat(usize::try_from(MAX_NOTE_BYTES).unwrap() + 1);
    assert_eq!(
        history.set_markdown(&big),
        Err(Error::NoteTooLarge {
            limit_bytes: MAX_NOTE_BYTES
        })
    );
    assert_eq!(history.markdown().unwrap(), "small");
}

#[test]
fn changes_carry_no_time_or_message() {
    let history = NoteHistory::seed(device(1), "a").unwrap();
    history.set_markdown("ab").unwrap();
    for bytes in history.take_local_changes().unwrap() {
        let change = Change::from_bytes(bytes).unwrap();
        assert_eq!((change.timestamp(), change.message()), (0, None));
    }
}

#[test]
fn splice_range_keeps_the_common_start_and_end() {
    assert_eq!(
        splice_range("one two three", "one 2 three"),
        Some((4, 3, "2"))
    );
    assert_eq!(splice_range("abc", "abcd"), Some((3, 0, "d")));
    assert_eq!(splice_range("abc", "bc"), Some((0, 1, "")));
    assert_eq!(splice_range("aaa", "aaaa"), Some((3, 0, "a")));
    assert_eq!(splice_range("same", "same"), None);
}

#[test]
fn splice_range_counts_code_points() {
    // The emoji is four bytes and one code point; positions must count it once.
    assert_eq!(splice_range("🧩 a", "🧩 b"), Some((2, 1, "b")));
    let history = NoteHistory::seed(device(1), "🧩 a é").unwrap();
    history.set_markdown("🧩 b é").unwrap();
    assert_eq!(history.markdown().unwrap(), "🧩 b é");
}

#[test]
fn save_and_load_keep_the_text_and_history() {
    let (a, b) = shared("base\n");
    a.set_markdown("base\nmore\n").unwrap();
    let restored = NoteHistory::load(device(1), &a.save().unwrap()).unwrap();
    assert_eq!(restored.markdown().unwrap(), "base\nmore\n");
    // The restored copy still merges with a device that joined before the save.
    for change in a.take_local_changes().unwrap() {
        b.apply_change(change, device(1)).unwrap();
    }
    b.set_markdown("start\nbase\nmore\n").unwrap();
    for change in b.take_local_changes().unwrap() {
        restored.apply_change(change, device(2)).unwrap();
    }
    assert_eq!(restored.markdown().unwrap(), "start\nbase\nmore\n");
}

#[test]
fn load_refuses_a_history_over_the_limit() {
    let big = vec![0; usize::try_from(MAX_SAVED_BYTES).unwrap() + 1];
    assert_eq!(
        NoteHistory::load(device(1), &big).map(|_| ()),
        Err(Error::SyncDataTooLarge {
            limit_bytes: MAX_SAVED_BYTES
        })
    );
}

#[test]
fn load_refuses_garbage() {
    assert_eq!(
        refusal(NoteHistory::load(device(1), b"not a history")),
        "the saved history is malformed"
    );
}

#[test]
fn load_refuses_a_document_that_is_not_one_note() {
    let mut doc = AutoCommit::new();
    doc.put(ROOT, "title", "not a note").unwrap();
    let saved = [&[SAVED_VERSION, 0, 0][..], &doc.save()].concat();
    assert_eq!(
        refusal(NoteHistory::load(device(1), &saved)),
        "the history does not hold one note's text"
    );
}

#[test]
fn concurrent_edits_on_two_devices_merge() {
    let (a, b) = shared("Alpha beta gamma\n");
    a.set_markdown("# Alpha beta gamma\n").unwrap();
    b.set_markdown("Alpha beta gamma delta\n").unwrap();
    for change in a.take_local_changes().unwrap() {
        b.apply_change(change, device(1)).unwrap();
    }
    for change in b.take_local_changes().unwrap() {
        a.apply_change(change, device(2)).unwrap();
    }
    assert_eq!(a.markdown().unwrap(), "# Alpha beta gamma delta\n");
    assert_eq!(b.markdown().unwrap(), a.markdown().unwrap());
}

#[test]
fn changes_apply_in_any_order() {
    let (a, b) = shared("one\n");
    a.set_markdown("one\ntwo\n").unwrap();
    a.set_markdown("one\ntwo\nthree\n").unwrap();
    let mut changes = a.take_local_changes().unwrap();
    changes.reverse();
    for change in changes {
        b.apply_change(change, device(1)).unwrap();
    }
    assert_eq!(b.markdown().unwrap(), "one\ntwo\nthree\n");
}

#[test]
fn a_change_held_twice_is_accepted_once() {
    let (a, b) = shared("x");
    a.set_markdown("xy").unwrap();
    let change = a.take_local_changes().unwrap().remove(0);
    b.apply_change(change.clone(), device(1)).unwrap();
    b.apply_change(change, device(1)).unwrap();
    assert_eq!(b.markdown().unwrap(), "xy");
}

#[test]
fn local_changes_leave_out_changes_from_other_devices() {
    let (a, b) = shared("x");
    a.set_markdown("xa").unwrap();
    for change in a.take_local_changes().unwrap() {
        b.apply_change(change, device(1)).unwrap();
    }
    b.set_markdown("xab").unwrap();
    let own = b.take_local_changes().unwrap();
    assert_eq!(own.len(), 1);
    let author = Change::from_bytes(own[0].clone()).unwrap();
    assert_eq!(author.actor_id(), &actor(device(2)));
    assert!(b.take_local_changes().unwrap().is_empty());
}

#[test]
fn join_refuses_a_change_that_depends_on_others() {
    let a = NoteHistory::seed(device(1), "x").unwrap();
    let _ = a.take_local_changes().unwrap();
    a.set_markdown("xy").unwrap();
    let second = a.take_local_changes().unwrap().remove(0);
    assert_eq!(
        refusal(NoteHistory::join(device(2), second, device(1))),
        "the change is not the start of a note"
    );
}

#[test]
fn join_refuses_a_change_that_is_not_a_note() {
    let mut doc = AutoCommit::new().with_actor(actor(device(1)));
    doc.put(ROOT, "title", "x").unwrap();
    doc.commit_with(CommitOptions::default().with_time(0));
    let change = last_change(&mut doc);
    assert_eq!(
        refusal(NoteHistory::join(device(2), change, device(1))),
        "the change is not the start of a note"
    );
}

#[test]
fn join_refuses_a_change_claiming_to_be_from_this_device() {
    let a = NoteHistory::seed(device(1), "x").unwrap();
    let seed = a.take_local_changes().unwrap().remove(0);
    assert_eq!(
        refusal(NoteHistory::join(device(1), seed, device(1))),
        "a change from this device that it never made"
    );
}

#[test]
fn refuses_a_change_over_the_size_limit() {
    let (a, _) = shared("x");
    let big = vec![0; usize::try_from(MAX_CHANGE_BYTES).unwrap() + 1];
    assert_eq!(
        a.apply_change(big, device(2)),
        Err(Error::SyncDataTooLarge {
            limit_bytes: MAX_CHANGE_BYTES
        })
    );
}

#[test]
fn refuses_anything_but_one_uncompressed_change() {
    let (a, b) = shared("x");
    // A saved document is a different chunk type.
    assert_eq!(
        refusal(a.apply_change(b.save().unwrap(), device(2))),
        "not a single uncompressed change"
    );
    // Two changes in one blob: Automerge's parser refuses the bytes after the first.
    b.set_markdown("xy").unwrap();
    b.set_markdown("xyz").unwrap();
    let two = b.take_local_changes().unwrap().concat();
    assert_eq!(
        refusal(a.apply_change(two, device(2))),
        "the change is malformed"
    );
    // A compressed change chunk (type 2) is refused before Automerge decompresses it.
    let mut compressed = b.save().unwrap();
    if let Some(kind) = compressed.get_mut(CHUNK_TYPE_OFFSET) {
        *kind = 0x02;
    }
    assert_eq!(
        refusal(a.apply_change(compressed, device(2))),
        "not a single uncompressed change"
    );
}

#[test]
fn refuses_a_malformed_change() {
    let (a, b) = shared("x");
    b.set_markdown("xy").unwrap();
    let mut change = b.take_local_changes().unwrap().remove(0);
    let last = change.len() - 1;
    change[last] ^= 0xff;
    assert_eq!(
        refusal(a.apply_change(change, device(2))),
        "the change is malformed"
    );
    assert_eq!(a.markdown().unwrap(), "x");
}

#[test]
fn refuses_a_change_from_another_author() {
    let (a, b) = shared("x");
    b.set_markdown("xy").unwrap();
    let change = b.take_local_changes().unwrap().remove(0);
    assert_eq!(
        refusal(a.apply_change(change, device(3))),
        "the change was not made by its author"
    );
}

#[test]
fn refuses_a_change_with_a_time_or_message() {
    let (a, b) = shared("x");
    let (mut doc, text) = forger(&b, 2);
    doc.splice_text(&text, 1, 0, "y").unwrap();
    doc.commit_with(CommitOptions::default().with_time(1_700_000_000));
    assert_eq!(
        refusal(a.apply_change(last_change(&mut doc), device(2))),
        "the change carries a time or message"
    );
    let (mut doc, text) = forger(&b, 2);
    doc.splice_text(&text, 1, 0, "y").unwrap();
    doc.commit_with(CommitOptions::default().with_time(0).with_message("hi"));
    assert_eq!(
        refusal(a.apply_change(last_change(&mut doc), device(2))),
        "the change carries a time or message"
    );
}

#[test]
fn refuses_a_change_that_reuses_a_sequence_number() {
    let a = NoteHistory::seed(device(1), "x").unwrap();
    let seed = a.take_local_changes().unwrap();
    // Two copies of device 2 make different first edits: same actor, same sequence number.
    let b = joined(device(2), &seed, device(1));
    let twin = joined(device(2), &seed, device(1));
    b.set_markdown("xb").unwrap();
    twin.set_markdown("xt").unwrap();
    a.apply_change(b.take_local_changes().unwrap().remove(0), device(2))
        .unwrap();
    assert_eq!(
        refusal(a.apply_change(twin.take_local_changes().unwrap().remove(0), device(2))),
        "the change reuses a sequence number"
    );
    assert_eq!(a.markdown().unwrap(), "xb");
}

#[test]
fn refuses_a_change_claiming_to_be_from_this_device() {
    let (a, _) = shared("x");
    let (mut doc, text) = forger(&a, 1);
    doc.splice_text(&text, 1, 0, "y").unwrap();
    doc.commit_with(CommitOptions::default().with_time(0));
    assert_eq!(
        refusal(a.apply_change(last_change(&mut doc), device(1))),
        "a change from this device that it never made"
    );
}

#[test]
fn refuses_a_change_that_adds_anything_but_text() {
    let (a, b) = shared("x");
    let (mut doc, _) = forger(&b, 2);
    doc.put(ROOT, "title", "extra").unwrap();
    doc.commit_with(CommitOptions::default().with_time(0));
    assert_eq!(
        refusal(a.apply_change(last_change(&mut doc), device(2))),
        "the change does more than edit the note's text"
    );
    let (mut doc, text) = forger(&b, 2);
    doc.mark(
        &text,
        Mark::new("bold".to_owned(), true, 0, 1),
        ExpandMark::None,
    )
    .unwrap();
    doc.commit_with(CommitOptions::default().with_time(0));
    assert_eq!(
        refusal(a.apply_change(last_change(&mut doc), device(2))),
        "the change does more than edit the note's text"
    );
    // The refused changes left nothing behind: a normal edit still applies.
    assert_eq!(a.markdown().unwrap(), "x");
    b.set_markdown("xy").unwrap();
    a.apply_change(b.take_local_changes().unwrap().remove(0), device(2))
        .unwrap();
    assert_eq!(a.markdown().unwrap(), "xy");
}

#[test]
fn guarded_turns_a_panic_into_a_refusal() {
    let result: Result<(), Error> = guarded(|| std::panic::resume_unwind(Box::new("boom")));
    assert_eq!(refusal(result), "the decoder failed");
}

#[test]
fn load_refuses_an_unknown_version() {
    let history = NoteHistory::seed(device(1), "x").unwrap();
    let mut saved = history.save().unwrap();
    saved[0] = SAVED_VERSION + 1;
    assert_eq!(
        refusal(NoteHistory::load(device(1), &saved)),
        "the saved history is malformed"
    );
}

#[test]
fn an_edit_saved_before_it_was_taken_is_still_handed_out_after_loading() {
    let (a, b) = shared("x");
    a.set_markdown("xy").unwrap();
    let restored = NoteHistory::load(device(1), &a.save().unwrap()).unwrap();
    let changes = restored.take_local_changes().unwrap();
    assert_eq!(changes.len(), 1);
    b.apply_change(changes[0].clone(), device(1)).unwrap();
    assert_eq!(b.markdown().unwrap(), "xy");
}

#[test]
fn debug_output_never_holds_the_note() {
    let history = NoteHistory::seed(device(1), "SECRET-NOTE-CONTENT").unwrap();
    let printed = format!("{history:?}");
    assert!(!printed.contains("SECRET"), "{printed}");
    assert!(!printed.contains("83, 69, 67"), "{printed}");
}

#[test]
fn large_edits_are_split_into_changes_another_device_accepts() {
    let (a, b) = shared("");
    let big = "a".repeat(usize::try_from(MAX_CHANGE_OPS).unwrap() * 2 + 10);
    a.set_markdown(&big).unwrap();
    let changes = a.take_local_changes().unwrap();
    assert_eq!(changes.len(), 3);
    for change in changes {
        b.apply_change(change, device(1)).unwrap();
    }
    assert_eq!(b.markdown().unwrap(), big);
    a.set_markdown("").unwrap();
    for change in a.take_local_changes().unwrap() {
        b.apply_change(change, device(1)).unwrap();
    }
    assert_eq!(b.markdown().unwrap(), "");
}

#[test]
fn refuses_a_change_with_too_many_operations() {
    let (a, b) = shared("x");
    let (mut doc, text) = forger(&b, 2);
    let ops = usize::try_from(MAX_CHANGE_OPS).unwrap() + 1;
    doc.splice_text(&text, 1, 0, &"y".repeat(ops)).unwrap();
    doc.commit_with(CommitOptions::default().with_time(0));
    assert_eq!(
        refusal(a.apply_change(last_change(&mut doc), device(2))),
        "the change holds too many operations"
    );
}

#[test]
fn refuses_a_change_with_extra_bytes() {
    let (a, b) = shared("x");
    let (mut doc, text) = forger(&b, 2);
    doc.splice_text(&text, 1, 0, "y").unwrap();
    doc.commit_with(CommitOptions::default().with_time(0));
    let honest = Change::from_bytes(last_change(&mut doc)).unwrap();
    let mut expanded = honest.decode();
    expanded.extra_bytes = b"hidden".to_vec();
    let smuggled = Change::from(expanded).raw_bytes().to_vec();
    assert_eq!(
        refusal(a.apply_change(smuggled, device(2))),
        "the change carries extra bytes"
    );
}

/// `change` re-encoded as sequence number `seq` of its author, keeping its dependencies.
fn renumbered(change: &[u8], seq: u64) -> Vec<u8> {
    let mut expanded = Change::from_bytes(change.to_vec()).unwrap().decode();
    expanded.seq = seq;
    Change::from(expanded).raw_bytes().to_vec()
}

#[test]
fn a_change_waiting_for_its_dependencies_is_checked_alone_when_they_arrive() {
    let a = NoteHistory::seed(device(1), "x").unwrap();
    let seed = a.take_local_changes().unwrap();
    let b = joined(device(2), &seed, device(1));
    let c = joined(device(3), &seed, device(1));
    // Device 2 makes an honest edit H; device 3 receives it and builds P on top, numbered as its
    // second change though it never sent a first.
    b.set_markdown("xh").unwrap();
    let honest = b.take_local_changes().unwrap().remove(0);
    c.apply_change(honest.clone(), device(2)).unwrap();
    c.set_markdown("xhp").unwrap();
    let bad = renumbered(&c.take_local_changes().unwrap()[0], 2);
    // Device 1 gets P first: it waits.
    let waiting = a.apply_change(bad.clone(), device(3)).unwrap();
    assert!(!waiting.applied);
    // H arrives: H applies, and P is refused on its own, blamed on device 3.
    let outcome = a.apply_change(honest, device(2)).unwrap();
    assert!(outcome.applied);
    assert_eq!(
        outcome.refused_later,
        vec![RefusedChange {
            author: device(3),
            reason: "sync data refused: the change does not follow its author's previous one"
                .to_owned(),
        }]
    );
    assert_eq!(a.markdown().unwrap(), "xh");
    // Sent again, P is refused at once rather than reported as waiting.
    assert_eq!(
        refusal(a.apply_change(bad, device(3))),
        "the change was refused before"
    );
}

#[test]
fn refuses_a_change_that_skips_its_authors_previous_one() {
    let (a, b) = shared("x");
    b.set_markdown("xy").unwrap();
    let skipped = renumbered(&b.take_local_changes().unwrap()[0], 2);
    assert_eq!(
        refusal(a.apply_change(skipped, device(2))),
        "the change does not follow its author's previous one"
    );
    assert_eq!(a.markdown().unwrap(), "x");
}

#[test]
fn a_waiting_change_is_refused_with_the_change_it_depends_on() {
    let (a, b) = shared("x");
    let (mut doc, text) = forger(&b, 2);
    doc.put(ROOT, "title", "extra").unwrap();
    doc.commit_with(CommitOptions::default().with_time(0));
    let bad = last_change(&mut doc);
    doc.splice_text(&text, 1, 0, "y").unwrap();
    doc.commit_with(CommitOptions::default().with_time(0));
    let built_on_it = last_change(&mut doc);
    assert!(!a.apply_change(built_on_it, device(2)).unwrap().applied);
    assert_eq!(
        refusal(a.apply_change(bad, device(2))),
        "the change does more than edit the note's text"
    );
    // The waiting change is reported with the next change that arrives.
    b.set_markdown("xz").unwrap();
    let next = a
        .apply_change(b.take_local_changes().unwrap().remove(0), device(2))
        .unwrap();
    assert_eq!(
        next.refused_later,
        vec![RefusedChange {
            author: device(2),
            reason: "sync data refused: it depends on a refused change".to_owned(),
        }]
    );
    assert_eq!(a.markdown().unwrap(), "xz");
}

#[test]
fn refuses_a_change_that_edits_the_root_and_leaves_it_unchanged() {
    let (a, b) = shared("x");
    let (mut doc, _) = forger(&b, 2);
    doc.put(ROOT, "padding", "a".repeat(1024)).unwrap();
    doc.delete(ROOT, "padding").unwrap();
    doc.commit_with(CommitOptions::default().with_time(0));
    assert_eq!(
        refusal(a.apply_change(last_change(&mut doc), device(2))),
        "the change does more than edit the note's text"
    );
}

/// `change` with an extra column Automerge does not know, and a checksum that matches: Automerge
/// parses and applies it, then writes it back without the column when saving.
fn with_unknown_column(change: &[u8]) -> Vec<u8> {
    fn read(bytes: &[u8], at: &mut usize) -> u64 {
        let (mut value, mut shift) = (0, 0);
        loop {
            let byte = bytes[*at];
            *at += 1;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return value;
            }
            shift += 7;
        }
    }
    fn write(out: &mut Vec<u8>, mut value: u64) {
        loop {
            let byte = u8::try_from(value & 0x7f).unwrap();
            value >>= 7;
            if value == 0 {
                out.push(byte);
                return;
            }
            out.push(byte | 0x80);
        }
    }
    fn skip(bytes: &[u8], at: &mut usize) {
        let len = read(bytes, at);
        *at += usize::try_from(len).unwrap();
    }
    let mut at = CHUNK_TYPE_OFFSET + 1;
    let len = usize::try_from(read(change, &mut at)).unwrap();
    let body = &change[at..at + len];
    let mut j = 0;
    let deps = read(body, &mut j);
    j += usize::try_from(deps).unwrap() * HASH_BYTES;
    skip(body, &mut j); // actor
    read(body, &mut j); // seq
    read(body, &mut j); // start op
    read(body, &mut j); // time
    skip(body, &mut j); // message
    for _ in 0..read(body, &mut j) {
        skip(body, &mut j); // other actors
    }
    let columns_start = j;
    let mut columns = Vec::new();
    for _ in 0..read(body, &mut j) {
        columns.push((read(body, &mut j), read(body, &mut j)));
    }
    let data_len = usize::try_from(columns.iter().map(|c| c.1).sum::<u64>()).unwrap();
    // Column id 200, an integer column, holding one run-length-encoded value.
    columns.push(((200 << 4) | 1, 2));
    let mut new_body = body[..columns_start].to_vec();
    write(&mut new_body, columns.len() as u64);
    for (spec, len) in &columns {
        write(&mut new_body, *spec);
        write(&mut new_body, *len);
    }
    new_body.extend_from_slice(&body[j..j + data_len]);
    new_body.extend_from_slice(&[0x7f, 0x05]);
    new_body.extend_from_slice(&body[j + data_len..]);
    let mut header = vec![CHUNK_TYPE_CHANGE];
    write(&mut header, new_body.len() as u64);
    let checksum = Sha256::new()
        .chain_update(&header)
        .chain_update(&new_body)
        .finalize();
    [&CHUNK_MAGIC[..], &checksum[..4], &header, &new_body].concat()
}

#[test]
fn refuses_a_change_that_is_not_in_canonical_form() {
    let (a, b) = shared("note");
    b.set_markdown("note!").unwrap();
    let honest = b.take_local_changes().unwrap().remove(0);
    let odd = with_unknown_column(&honest);
    // Automerge itself accepts it, which is what made the saved history fail to load.
    assert!(Change::from_bytes(odd.clone()).is_ok());
    assert_eq!(
        refusal(a.apply_change(odd, device(2))),
        "the change is not in canonical form"
    );
    a.apply_change(honest, device(2)).unwrap();
    let restored = NoteHistory::load(device(1), &a.save().unwrap()).unwrap();
    assert_eq!(restored.markdown().unwrap(), "note!");
}

#[test]
fn one_author_cannot_fill_the_pending_set() {
    let a = NoteHistory::seed(device(1), "x").unwrap();
    let seed = a.take_local_changes().unwrap();
    let b = joined(device(2), &seed, device(1));
    let c = joined(device(3), &seed, device(1));
    let cap = usize::try_from(MAX_PENDING_PER_AUTHOR).unwrap();
    let mut text = String::from("x");
    for _ in 0..=cap + 1 {
        text.push('y');
        b.set_markdown(&text).unwrap();
    }
    // Withhold device 2's first change, so every later one waits.
    let changes = b.take_local_changes().unwrap();
    for change in &changes[1..=cap] {
        assert!(!a.apply_change(change.clone(), device(2)).unwrap().applied);
    }
    assert_eq!(
        refusal(a.apply_change(changes[cap + 1].clone(), device(2))),
        "too many changes are waiting"
    );
    // Another device's out-of-order changes still wait.
    c.set_markdown("xc").unwrap();
    c.set_markdown("xcc").unwrap();
    let second = c.take_local_changes().unwrap().remove(1);
    assert!(!a.apply_change(second, device(3)).unwrap().applied);
}

#[test]
fn an_edit_past_the_history_operation_limit_is_refused_and_changes_nothing() {
    let (a, _) = shared("");
    let half = "a".repeat(usize::try_from(MAX_HISTORY_OPS / 2).unwrap() + 1);
    a.set_markdown(&half).unwrap();
    assert_eq!(
        refusal(a.set_markdown("")),
        "the history holds too many operations"
    );
    assert_eq!(a.markdown().unwrap(), half);
    a.set_markdown(&format!("{half}!")).unwrap();
}

#[test]
fn refuses_a_character_inserted_anywhere_but_the_note_text() {
    let (a, b) = shared("x");
    b.set_markdown("xy").unwrap();
    let mut expanded = Change::from_bytes(b.take_local_changes().unwrap().remove(0))
        .unwrap()
        .decode();
    for op in &mut expanded.operations {
        op.obj = automerge::legacy::ObjectId::Root;
    }
    let elsewhere = Change::from(expanded).raw_bytes().to_vec();
    assert_eq!(
        refusal(a.apply_change(elsewhere, device(2))),
        "the change does more than edit the note's text"
    );
    assert_eq!(a.markdown().unwrap(), "x");
}

/// `change` decoded, altered by `alter`, and encoded again with a matching hash.
fn tampered(change: &[u8], alter: impl FnOnce(&mut ExpandedChange)) -> Vec<u8> {
    let mut expanded = Change::from_bytes(change.to_vec()).unwrap().decode();
    alter(&mut expanded);
    Change::from(expanded).raw_bytes().to_vec()
}

fn op_id(counter: u64, device_number: u8) -> OpId {
    OpId::new(counter, &actor(device(device_number)))
}

/// Device 1 seeded "abc": operation 1 made the text, 2 to 4 inserted a, b and c.
fn abc() -> (NoteHistory, NoteHistory) {
    shared("abc")
}

/// After a refusal the history still holds its text, saves, loads, and takes an honest edit.
fn still_sound(a: &NoteHistory, text: &str) {
    assert_eq!(a.markdown().unwrap(), text);
    let restored = NoteHistory::load(device(1), &a.save().unwrap()).unwrap();
    assert_eq!(restored.markdown().unwrap(), text);
    a.set_markdown(&format!("{text}!")).unwrap();
    assert_eq!(a.markdown().unwrap(), format!("{text}!"));
}

#[test]
fn refuses_a_change_whose_operation_numbers_do_not_follow_its_dependencies() {
    let (a, b) = abc();
    b.set_markdown("abcd").unwrap();
    let honest = b.take_local_changes().unwrap().remove(0);
    for start in [1, 2, (1 << 32) - 3] {
        let moved = tampered(&honest, |c| {
            c.start_op = std::num::NonZeroU64::new(start).unwrap();
        });
        assert_eq!(
            refusal(a.apply_change(moved, device(2))),
            "the change's operation numbers do not follow its dependencies"
        );
    }
    still_sound(&a, "abc");
}

#[test]
fn join_refuses_a_seed_whose_operation_numbers_do_not_start_at_one() {
    let a = NoteHistory::seed(device(1), "").unwrap();
    let seed = a.take_local_changes().unwrap().remove(0);
    let moved = tampered(&seed, |c| {
        c.start_op = std::num::NonZeroU64::new(2).unwrap();
    });
    assert_eq!(
        refusal(NoteHistory::join(device(2), moved, device(1))),
        "the change is not the start of a note"
    );
}

#[test]
fn refuses_inserts_that_point_at_anything_but_a_visible_character() {
    let (a, b) = abc();
    b.set_markdown("abcd").unwrap();
    let honest = b.take_local_changes().unwrap().remove(0);
    let start = Change::from_bytes(honest.clone()).unwrap().start_op().get();
    let after = |target: OpId| {
        tampered(&honest, move |c| {
            c.operations[0].key = Key::Seq(ElementId::Id(target));
        })
    };
    // After the text object, a character that does not exist, and itself.
    for odd in [
        after(op_id(1, 1)),
        after(op_id(999, 1)),
        after(op_id(start, 2)),
    ] {
        assert_eq!(
            refusal(a.apply_change(odd, device(2))),
            "the change does more than edit the note's text"
        );
    }
    // An insert that also names a predecessor would replace that character.
    let replacing = tampered(&honest, |c| c.operations[0].pred = vec![op_id(2, 1)].into());
    assert_eq!(
        refusal(a.apply_change(replacing, device(2))),
        "the change does more than edit the note's text"
    );
    still_sound(&a, "abc");
}

#[test]
fn refuses_deletes_that_do_not_name_exactly_one_visible_character() {
    let (a, b) = abc();
    b.set_markdown("ac").unwrap();
    let honest = b.take_local_changes().unwrap().remove(0);
    let delete = |key: Key, pred: Vec<OpId>| {
        tampered(&honest, move |c| {
            c.operations[0].key = key;
            c.operations[0].pred = pred.into();
        })
    };
    let b_char = op_id(3, 1);
    for odd in [
        delete(Key::Seq(ElementId::Head), vec![b_char.clone()]),
        delete(Key::Seq(ElementId::Id(b_char.clone())), vec![op_id(2, 1)]),
        delete(Key::Seq(ElementId::Id(b_char.clone())), vec![op_id(1, 1)]),
        delete(Key::Seq(ElementId::Id(op_id(999, 1))), vec![op_id(999, 1)]),
        delete(Key::Seq(ElementId::Id(op_id(1, 1))), vec![op_id(1, 1)]),
    ] {
        assert_eq!(
            refusal(a.apply_change(odd, device(2))),
            "the change does more than edit the note's text"
        );
    }
    still_sound(&a, "abc");
    a.apply_change(honest, device(2)).unwrap();
}

#[test]
fn refuses_a_change_that_references_a_character_outside_its_causal_past() {
    let a = NoteHistory::seed(device(1), "x").unwrap();
    let seed = a.take_local_changes().unwrap();
    let b = joined(device(2), &seed, device(1));
    let c = joined(device(3), &seed, device(1));
    // Device 3 inserts "c"; device 2 never saw it, but names it as the character it types after.
    c.set_markdown("xc").unwrap();
    let from_c = c.take_local_changes().unwrap().remove(0);
    let c_char = Change::from_bytes(from_c.clone()).unwrap().start_op().get();
    a.apply_change(from_c, device(3)).unwrap();
    b.set_markdown("xb").unwrap();
    let from_b = tampered(&b.take_local_changes().unwrap()[0], |change| {
        change.operations[0].key = Key::Seq(ElementId::Id(op_id(c_char, 3)));
    });
    assert_eq!(
        refusal(a.apply_change(from_b, device(2))),
        "the change does more than edit the note's text"
    );
    still_sound(&a, "xc");
}

#[test]
fn refuses_a_change_with_too_many_dependencies() {
    let (a, b) = shared("x");
    b.set_markdown("xy").unwrap();
    let many = tampered(&b.take_local_changes().unwrap()[0], |c| {
        let limit = usize::try_from(MAX_CHANGE_DEPS).unwrap();
        c.deps
            .extend((0..limit).map(|n| ChangeHash([u8::try_from(n).unwrap(); 32])));
    });
    assert_eq!(
        refusal(a.apply_change(many, device(2))),
        "the change has too many dependencies"
    );
}

#[test]
fn a_refused_change_leaves_nothing_waiting_behind() {
    let (a, b) = shared("x");
    let (mut doc, text) = forger(&b, 2);
    doc.put(ROOT, "title", "extra").unwrap();
    doc.commit_with(CommitOptions::default().with_time(0));
    let bad = last_change(&mut doc);
    doc.splice_text(&text, 1, 0, "y").unwrap();
    doc.commit_with(CommitOptions::default().with_time(0));
    let mut built_on_it = Change::from_bytes(last_change(&mut doc)).unwrap().decode();
    built_on_it.deps.push(ChangeHash([7; 32]));
    let built_on_it = Change::from(built_on_it).raw_bytes().to_vec();
    assert!(!a.apply_change(built_on_it, device(2)).unwrap().applied);
    refusal(a.apply_change(bad, device(2)));
    let state = a.state.lock().unwrap();
    assert!(state.pending.is_empty());
    assert!(
        state.waiting_on.is_empty(),
        "{} stale",
        state.waiting_on.len()
    );
    assert_eq!(state.pending_bytes, 0);
}

/// A saved history holding `a`'s document plus `extra`, applied by raw Automerge, which does not
/// check what Tessera checks.
fn saved_with(a: &NoteHistory, extra: Vec<u8>) -> Vec<u8> {
    let mut doc = AutoCommit::load(&document(a)).unwrap();
    doc.apply_changes([Change::from_bytes(extra).unwrap()])
        .unwrap();
    [&[SAVED_VERSION, 0, 0][..], &doc.save()].concat()
}

#[test]
fn load_refuses_a_change_whose_operation_numbers_do_not_follow_its_dependencies() {
    let (a, b) = abc();
    b.set_markdown("abcd").unwrap();
    let moved = tampered(&b.take_local_changes().unwrap()[0], |c| {
        c.start_op = std::num::NonZeroU64::new(100).unwrap();
    });
    assert_eq!(
        refusal(NoteHistory::load(device(1), &saved_with(&a, moved))),
        "the saved history is malformed"
    );
}

#[test]
fn load_refuses_a_change_that_skips_its_authors_previous_one() {
    let a = NoteHistory::seed(device(1), "abc").unwrap();
    let seed = a.take_local_changes().unwrap();
    let b = joined(device(2), &seed, device(1));
    let c = joined(device(3), &seed, device(1));
    b.set_markdown("abcd").unwrap();
    let first = b.take_local_changes().unwrap().remove(0);
    // Device 3's change uses higher operation numbers than device 2's first.
    c.set_markdown("abczzzz").unwrap();
    let from_c = c.take_local_changes().unwrap().remove(0);
    // A second copy of device 2 that never made its first change builds on device 3's, and
    // numbers it as device 2's second: its operation numbers are right, its chain is not.
    let twin = joined(device(2), &seed, device(1));
    twin.apply_change(from_c.clone(), device(3)).unwrap();
    twin.set_markdown("abczzzz!").unwrap();
    let skipping = renumbered(&twin.take_local_changes().unwrap()[0], 2);
    let mut doc = AutoCommit::load(&document(&a)).unwrap();
    for change in [first, from_c, skipping] {
        doc.apply_changes([Change::from_bytes(change).unwrap()])
            .unwrap();
    }
    let saved = [&[SAVED_VERSION, 0, 0][..], &doc.save()].concat();
    assert_eq!(
        refusal(NoteHistory::load(device(1), &saved)),
        "the saved history is malformed"
    );
}

#[test]
fn refuses_a_change_that_deletes_one_character_twice() {
    let (a, b) = abc();
    b.set_markdown("ac").unwrap();
    let twice = tampered(&b.take_local_changes().unwrap()[0], |c| {
        let delete = c.operations[0].clone();
        c.operations.push(delete);
    });
    assert_eq!(
        refusal(a.apply_change(twice, device(2))),
        "the change does more than edit the note's text"
    );
    still_sound(&a, "abc");
}

#[test]
fn refuses_changes_past_the_history_change_limit() {
    let (a, b) = shared("x");
    // Building 20,000 real changes takes minutes in a debug build; stand-ins in the index count
    // the same.
    {
        let mut state = a.state.lock().unwrap();
        let limit = usize::try_from(MAX_HISTORY_CHANGES).unwrap();
        let mut n = 0u64;
        while state.known.len() < limit {
            n += 1;
            let mut hash = [0; 32];
            hash[..8].copy_from_slice(&n.to_be_bytes());
            state.known.insert(
                ChangeHash(hash),
                Past {
                    max_op: 0,
                    clock: Vec::new(),
                },
            );
        }
    }
    assert_eq!(
        refusal(a.set_markdown("xa")),
        "the history holds too many changes"
    );
    b.set_markdown("xb").unwrap();
    assert_eq!(
        refusal(a.apply_change(b.take_local_changes().unwrap().remove(0), device(2))),
        "the history holds too many changes"
    );
}

#[test]
fn refuses_changes_from_more_devices_than_the_limit() {
    let a = NoteHistory::seed(device(1), "x").unwrap();
    let seed = a.take_local_changes().unwrap();
    let limit = u8::try_from(MAX_HISTORY_AUTHORS).unwrap();
    for n in 2..=limit {
        let other = joined(device(n), &seed, device(1));
        other.set_markdown(&format!("x{n}")).unwrap();
        a.apply_change(other.take_local_changes().unwrap().remove(0), device(n))
            .unwrap();
    }
    // A device that joined but never edited cannot record its first edit either.
    let late = NoteHistory::load(device(limit + 2), &a.save().unwrap()).unwrap();
    assert_eq!(
        refusal(late.set_markdown("late")),
        "the history holds changes from too many devices"
    );
    let one_more = joined(device(limit + 1), &seed, device(1));
    one_more.set_markdown("x!").unwrap();
    assert_eq!(
        refusal(a.apply_change(
            one_more.take_local_changes().unwrap().remove(0),
            device(limit + 1)
        )),
        "the history holds changes from too many devices"
    );
}
