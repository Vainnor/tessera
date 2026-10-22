use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;

/// A synthetic vault in the OS temp folder, removed when dropped.
struct TempVault {
    dir: PathBuf,
}

impl TempVault {
    fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let dir = std::env::temp_dir().join(format!(
            "tessera-vault-test-{}-{}-{nanos}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    fn file(&self, relative: &str, contents: &str) -> &Self {
        let path = self.dir.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
        self
    }

    fn folder(&self, relative: &str) -> &Self {
        fs::create_dir_all(self.dir.join(relative)).unwrap();
        self
    }

    fn open(&self) -> Vault {
        Vault::open(self.dir.to_str().unwrap()).unwrap()
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.dir.join(relative)).unwrap()
    }

    fn names_in(&self, relative: &str) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.dir.join(relative))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for TempVault {
    fn drop(&mut self) {
        let _cleanup = fs::remove_dir_all(&self.dir);
    }
}

fn path(raw: &str) -> VaultPath {
    VaultPath::parse(raw).unwrap()
}

fn note_paths(listing: &NoteListing) -> Vec<&str> {
    listing.notes.iter().map(|n| n.path.as_str()).collect()
}

#[cfg(unix)]
fn symlink(target: &std::path::Path, link: &std::path::Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}

// --- open ---

#[test]
fn open_rejects_a_missing_folder() {
    let missing = std::env::temp_dir().join("tessera-core-test-missing-vault");
    assert_eq!(
        Vault::open(missing.to_str().unwrap()).unwrap_err(),
        Error::VaultUnavailable
    );
}

#[test]
fn open_rejects_a_file() {
    let vault = TempVault::new();
    vault.file("note.md", "x");
    let file = vault.dir.join("note.md");
    assert_eq!(
        Vault::open(file.to_str().unwrap()).unwrap_err(),
        Error::VaultUnavailable
    );
}

// --- list_notes ---

#[test]
fn lists_nested_notes_and_folders_sorted() {
    let vault = TempVault::new();
    vault
        .file("b.md", "b")
        .file("Projects/Tessera/plan.md", "p")
        .file("a.md", "a")
        .folder("Empty");
    let listing = vault.open().list_notes().unwrap();
    assert_eq!(
        note_paths(&listing),
        ["Projects/Tessera/plan.md", "a.md", "b.md"]
    );
    let folders: Vec<&str> = listing.folders.iter().map(VaultPath::as_str).collect();
    assert_eq!(folders, ["Empty", "Projects", "Projects/Tessera"]);
    assert!(listing.skipped.is_empty());
    assert!(!listing.truncated);
}

#[test]
fn listing_reports_size_and_modification_time() {
    let vault = TempVault::new();
    vault.file("a.md", "hello");
    let listing = vault.open().list_notes().unwrap();
    let revision = listing.notes.first().unwrap().revision;
    assert_eq!(revision.size_bytes, 5);
    assert!(revision.modified_ns.is_some());
}

#[test]
fn listing_ignores_files_that_are_not_markdown() {
    let vault = TempVault::new();
    vault
        .file("a.md", "a")
        .file("image.png", "png")
        .file("notes.txt", "t");
    assert_eq!(note_paths(&vault.open().list_notes().unwrap()), ["a.md"]);
}

#[test]
fn listing_skips_hidden_entries_silently() {
    let vault = TempVault::new();
    vault
        .file("a.md", "a")
        .file(".obsidian/workspace.md", "w")
        .file(".trash/old.md", "o")
        .file("notes/.draft.md", "d");
    let listing = vault.open().list_notes().unwrap();
    assert_eq!(note_paths(&listing), ["a.md"]);
    let folders: Vec<&str> = listing.folders.iter().map(VaultPath::as_str).collect();
    assert_eq!(folders, ["notes"]);
    assert!(listing.skipped.is_empty());
}

#[test]
fn listing_of_an_empty_vault_is_empty() {
    let vault = TempVault::new();
    assert_eq!(vault.open().list_notes().unwrap(), NoteListing::default());
}

#[cfg(unix)]
#[test]
fn listing_reports_names_some_platforms_cannot_store() {
    let vault = TempVault::new();
    vault.file("ok.md", "o").file("what?.md", "w");
    let listing = vault.open().list_notes().unwrap();
    assert_eq!(note_paths(&listing), ["ok.md"]);
    assert_eq!(
        listing.skipped,
        [SkippedEntry {
            path: "what?.md".to_owned(),
            reason: SkipReason::UnsupportedName,
        }]
    );
}

#[cfg(unix)]
#[test]
fn listing_does_not_follow_symlinks() {
    let vault = TempVault::new();
    let outside = TempVault::new();
    outside.file("secret.md", "outside the vault");
    vault.file("a.md", "a");
    symlink(&outside.dir, &vault.dir.join("linked-folder"));
    symlink(
        &outside.dir.join("secret.md"),
        &vault.dir.join("linked-note.md"),
    );
    let listing = vault.open().list_notes().unwrap();
    assert_eq!(note_paths(&listing), ["a.md"]);
    let skipped: Vec<(&str, SkipReason)> = listing
        .skipped
        .iter()
        .map(|s| (s.path.as_str(), s.reason))
        .collect();
    assert_eq!(
        skipped,
        [
            ("linked-folder", SkipReason::Symlink),
            ("linked-note.md", SkipReason::Symlink),
        ]
    );
}

#[cfg(unix)]
#[test]
fn opening_through_a_symlinked_root_is_allowed() {
    let vault = TempVault::new();
    let links = TempVault::new();
    vault.file("a.md", "a");
    let root_link = links.dir.join("vault");
    symlink(&vault.dir, &root_link);
    let opened = Vault::open(root_link.to_str().unwrap()).unwrap();
    assert_eq!(note_paths(&opened.list_notes().unwrap()), ["a.md"]);
}

#[test]
fn listing_stops_descending_at_the_depth_limit() {
    let vault = TempVault::new();
    let deep: Vec<String> = (0..MAX_DEPTH).map(|i| format!("d{i}")).collect();
    let deep = deep.join("/");
    vault.file(&format!("{deep}/note.md"), "n");
    let listing = vault.open().list_notes().unwrap();
    assert!(listing.notes.is_empty());
    assert_eq!(listing.folders.len(), MAX_DEPTH - 1);
    let skipped = listing.skipped.first().unwrap();
    assert_eq!(skipped.reason, SkipReason::TooDeep);
}

// --- read_note ---

#[test]
fn reads_a_note_exactly_as_stored() {
    let vault = TempVault::new();
    let source = "---\ntags: [a]\n---\n# Title\r\n\nBody with ünïcode\n";
    vault.file("notes/a.md", source);
    let note = vault.open().read_note(&path("notes/a.md")).unwrap();
    assert_eq!(note.markdown, source);
    assert_eq!(note.path, path("notes/a.md"));
    assert_eq!(note.revision.size_bytes, source.len() as u64);
}

#[test]
fn read_reports_a_missing_note() {
    let vault = TempVault::new();
    assert_eq!(
        vault.open().read_note(&path("missing.md")).unwrap_err(),
        Error::NoteNotFound
    );
}

#[test]
fn read_treats_a_folder_named_like_a_note_as_missing() {
    let vault = TempVault::new();
    vault.folder("folder.md");
    assert_eq!(
        vault.open().read_note(&path("folder.md")).unwrap_err(),
        Error::NoteNotFound
    );
}

#[test]
fn read_refuses_hidden_paths() {
    let vault = TempVault::new();
    vault.file(".obsidian/a.md", "a");
    assert!(matches!(
        vault.open().read_note(&path(".obsidian/a.md")),
        Err(Error::NotANote { .. })
    ));
}

#[test]
fn read_refuses_files_that_are_not_markdown() {
    let vault = TempVault::new();
    vault.file("a.txt", "a");
    assert!(matches!(
        vault.open().read_note(&path("a.txt")),
        Err(Error::NotANote { .. })
    ));
}

#[test]
fn read_refuses_a_note_over_the_size_limit() {
    let vault = TempVault::new();
    vault.file("big.md", "");
    // Sparse, so the test does not write 16 MiB.
    fs::File::options()
        .write(true)
        .open(vault.dir.join("big.md"))
        .unwrap()
        .set_len(MAX_NOTE_BYTES + 1)
        .unwrap();
    assert_eq!(
        vault.open().read_note(&path("big.md")).unwrap_err(),
        Error::NoteTooLarge {
            limit_bytes: MAX_NOTE_BYTES
        }
    );
}

#[test]
fn read_refuses_text_that_is_not_utf8() {
    let vault = TempVault::new();
    fs::write(vault.dir.join("latin1.md"), b"caf\xe9").unwrap();
    assert_eq!(
        vault.open().read_note(&path("latin1.md")).unwrap_err(),
        Error::NotUtf8
    );
}

#[cfg(unix)]
#[test]
fn read_refuses_a_symlinked_note() {
    let vault = TempVault::new();
    let outside = TempVault::new();
    outside.file("secret.md", "outside the vault");
    symlink(&outside.dir.join("secret.md"), &vault.dir.join("a.md"));
    assert_eq!(
        vault.open().read_note(&path("a.md")).unwrap_err(),
        Error::SymlinkRefused
    );
}

#[cfg(unix)]
#[test]
fn read_refuses_a_path_through_a_symlinked_folder() {
    let vault = TempVault::new();
    let outside = TempVault::new();
    outside.file("secret.md", "outside the vault");
    symlink(&outside.dir, &vault.dir.join("linked"));
    assert_eq!(
        vault
            .open()
            .read_note(&path("linked/secret.md"))
            .unwrap_err(),
        Error::SymlinkRefused
    );
}

// --- create_note ---

#[test]
fn creates_a_note_and_its_folders() {
    let vault = TempVault::new();
    let opened = vault.open();
    let revision = opened.create_note(&path("new/deep/a.md"), "# A\n").unwrap();
    assert_eq!(vault.read("new/deep/a.md"), "# A\n");
    assert_eq!(
        opened.read_note(&path("new/deep/a.md")).unwrap().revision,
        revision
    );
}

#[test]
fn create_never_replaces_an_existing_note() {
    let vault = TempVault::new();
    vault.file("a.md", "original");
    assert_eq!(
        vault.open().create_note(&path("a.md"), "new").unwrap_err(),
        Error::NoteExists
    );
    assert_eq!(vault.read("a.md"), "original");
}

#[test]
fn create_reports_a_folder_named_like_the_note_as_existing() {
    let vault = TempVault::new();
    vault.folder("a.md");
    assert_eq!(
        vault.open().create_note(&path("a.md"), "new").unwrap_err(),
        Error::NoteExists
    );
    assert!(vault.dir.join("a.md").is_dir());
}

#[test]
fn create_refuses_hidden_paths() {
    let vault = TempVault::new();
    assert!(matches!(
        vault.open().create_note(&path(".git/hooks/a.md"), "x"),
        Err(Error::NotANote { .. })
    ));
    assert!(!vault.dir.join(".git").exists());
}

#[test]
fn create_refuses_a_note_over_the_size_limit() {
    let vault = TempVault::new();
    let big = "a".repeat(usize::try_from(MAX_NOTE_BYTES).unwrap() + 1);
    assert_eq!(
        vault.open().create_note(&path("big.md"), &big).unwrap_err(),
        Error::NoteTooLarge {
            limit_bytes: MAX_NOTE_BYTES
        }
    );
    assert!(!vault.dir.join("big.md").exists());
}

#[test]
fn create_fails_when_a_file_is_in_the_way_of_a_folder() {
    let vault = TempVault::new();
    vault.file("notes", "a file, not a folder");
    assert!(matches!(
        vault.open().create_note(&path("notes/a.md"), "x"),
        Err(Error::Io { .. })
    ));
}

#[cfg(unix)]
#[test]
fn create_refuses_to_write_through_a_symlinked_folder() {
    let vault = TempVault::new();
    let outside = TempVault::new();
    symlink(&outside.dir, &vault.dir.join("linked"));
    assert_eq!(
        vault
            .open()
            .create_note(&path("linked/a.md"), "x")
            .unwrap_err(),
        Error::SymlinkRefused
    );
    assert!(outside.names_in("").is_empty());
}

// --- write_note ---

#[test]
fn write_replaces_the_note_and_returns_the_new_revision() {
    let vault = TempVault::new();
    vault.file("a.md", "old");
    let opened = vault.open();
    let read = opened.read_note(&path("a.md")).unwrap();
    let written = opened
        .write_note(&path("a.md"), "new contents", read.revision)
        .unwrap();
    assert_eq!(vault.read("a.md"), "new contents");
    assert_eq!(written.size_bytes, 12);
    assert_eq!(opened.read_note(&path("a.md")).unwrap().revision, written);
}

#[test]
fn successive_writes_chain_revisions() {
    let vault = TempVault::new();
    let opened = vault.open();
    let mut revision = opened.create_note(&path("a.md"), "0").unwrap();
    for text in ["1", "22", "333"] {
        revision = opened.write_note(&path("a.md"), text, revision).unwrap();
    }
    assert_eq!(vault.read("a.md"), "333");
}

#[test]
fn write_refuses_when_the_note_changed_on_disk() {
    let vault = TempVault::new();
    vault.file("a.md", "old");
    let opened = vault.open();
    let read = opened.read_note(&path("a.md")).unwrap();
    vault.file("a.md", "edited in another app");
    assert_eq!(
        opened
            .write_note(&path("a.md"), "mine", read.revision)
            .unwrap_err(),
        Error::Conflict
    );
    assert_eq!(vault.read("a.md"), "edited in another app");
}

#[test]
fn write_reports_a_note_deleted_since_it_was_read() {
    let vault = TempVault::new();
    vault.file("a.md", "old");
    let opened = vault.open();
    let read = opened.read_note(&path("a.md")).unwrap();
    fs::remove_file(vault.dir.join("a.md")).unwrap();
    assert_eq!(
        opened
            .write_note(&path("a.md"), "mine", read.revision)
            .unwrap_err(),
        Error::NoteNotFound
    );
    assert!(!vault.dir.join("a.md").exists());
}

#[test]
fn write_leaves_no_temporary_files_behind() {
    let vault = TempVault::new();
    let opened = vault.open();
    let revision = opened.create_note(&path("notes/a.md"), "0").unwrap();
    opened
        .write_note(&path("notes/a.md"), "1", revision)
        .unwrap();
    assert_eq!(vault.names_in("notes"), ["a.md"]);
}

#[test]
fn write_refuses_a_note_over_the_size_limit() {
    let vault = TempVault::new();
    vault.file("a.md", "old");
    let opened = vault.open();
    let read = opened.read_note(&path("a.md")).unwrap();
    let big = "a".repeat(usize::try_from(MAX_NOTE_BYTES).unwrap() + 1);
    assert_eq!(
        opened
            .write_note(&path("a.md"), &big, read.revision)
            .unwrap_err(),
        Error::NoteTooLarge {
            limit_bytes: MAX_NOTE_BYTES
        }
    );
    assert_eq!(vault.read("a.md"), "old");
}

#[cfg(unix)]
#[test]
fn write_refuses_a_symlinked_note() {
    let vault = TempVault::new();
    let outside = TempVault::new();
    outside.file("secret.md", "outside the vault");
    symlink(&outside.dir.join("secret.md"), &vault.dir.join("a.md"));
    let revision = Revision {
        size_bytes: 0,
        modified_ns: None,
    };
    assert_eq!(
        vault
            .open()
            .write_note(&path("a.md"), "overwrite", revision)
            .unwrap_err(),
        Error::SymlinkRefused
    );
    assert_eq!(outside.read("secret.md"), "outside the vault");
}

// --- move_note ---

fn revision_of(vault: &Vault, raw: &str) -> Revision {
    vault.read_note(&path(raw)).unwrap().revision
}

#[test]
fn moves_a_note_and_keeps_its_contents_and_revision() {
    let vault = TempVault::new();
    vault.file("a.md", "contents");
    let opened = vault.open();
    let before = revision_of(&opened, "a.md");
    let after = opened
        .move_note(&path("a.md"), &path("b.md"), before)
        .unwrap();
    assert_eq!(vault.read("b.md"), "contents");
    assert!(!vault.dir.join("a.md").exists());
    assert_eq!(after, revision_of(&opened, "b.md"));
    assert_eq!(after.size_bytes, before.size_bytes);
}

#[test]
fn move_creates_missing_folders() {
    let vault = TempVault::new();
    vault.file("a.md", "x");
    let opened = vault.open();
    let revision = revision_of(&opened, "a.md");
    opened
        .move_note(&path("a.md"), &path("Archive/2026/a.md"), revision)
        .unwrap();
    assert_eq!(vault.read("Archive/2026/a.md"), "x");
}

#[test]
fn move_never_replaces_an_existing_note() {
    let vault = TempVault::new();
    vault.file("a.md", "mine").file("b.md", "theirs");
    let opened = vault.open();
    let revision = revision_of(&opened, "a.md");
    assert_eq!(
        opened
            .move_note(&path("a.md"), &path("b.md"), revision)
            .unwrap_err(),
        Error::NoteExists
    );
    assert_eq!(vault.read("a.md"), "mine");
    assert_eq!(vault.read("b.md"), "theirs");
}

#[test]
fn move_changes_only_letter_case() {
    let vault = TempVault::new();
    vault.file("plan.md", "x");
    let opened = vault.open();
    let revision = revision_of(&opened, "plan.md");
    opened
        .move_note(&path("plan.md"), &path("Plan.md"), revision)
        .unwrap();
    assert_eq!(vault.names_in(""), ["Plan.md"]);
}

/// Only meaningful on a case-sensitive filesystem (Linux CI, or a case-sensitive APFS volume),
/// where both names can exist; elsewhere it returns early.
#[test]
fn case_only_rename_never_replaces_a_differently_cased_note() {
    let vault = TempVault::new();
    vault.file("plan.md", "lower").file("Plan.md", "upper");
    if vault.names_in("").len() < 2 {
        return;
    }
    let opened = vault.open();
    let revision = revision_of(&opened, "plan.md");
    assert_eq!(
        opened
            .move_note(&path("plan.md"), &path("Plan.md"), revision)
            .unwrap_err(),
        Error::NoteExists
    );
    assert_eq!(vault.read("Plan.md"), "upper");
    assert_eq!(vault.read("plan.md"), "lower");
}

/// The Kelvin sign (U+212A) lowercases to `k` in Unicode, but NTFS keeps it apart from `K`, so a
/// string comparison would call this a case-only rename and replace `K.md` there. Whatever the
/// filesystem decides, no note may be lost.
#[test]
fn rename_never_loses_a_note_to_unicode_case_folding() {
    let vault = TempVault::new();
    vault.file("\u{212A}.md", "kelvin").file("K.md", "upper");
    if vault.names_in("").len() < 2 {
        return;
    }
    let opened = vault.open();
    let revision = revision_of(&opened, "\u{212A}.md");
    let result = opened.move_note(&path("\u{212A}.md"), &path("k.md"), revision);
    assert!(
        matches!(result, Ok(_) | Err(Error::NoteExists)),
        "{result:?}"
    );
    assert_eq!(vault.read("K.md"), "upper");
    assert_eq!(vault.names_in("").len(), 2);
}

#[test]
fn move_to_the_same_path_changes_nothing() {
    let vault = TempVault::new();
    vault.file("a.md", "x");
    let opened = vault.open();
    let revision = revision_of(&opened, "a.md");
    assert_eq!(
        opened.move_note(&path("a.md"), &path("a.md"), revision),
        Ok(revision)
    );
    assert_eq!(vault.read("a.md"), "x");
}

#[test]
fn move_refuses_when_the_note_changed_on_disk() {
    let vault = TempVault::new();
    vault.file("a.md", "old");
    let opened = vault.open();
    let revision = revision_of(&opened, "a.md");
    vault.file("a.md", "edited elsewhere");
    assert_eq!(
        opened
            .move_note(&path("a.md"), &path("b.md"), revision)
            .unwrap_err(),
        Error::Conflict
    );
    assert!(!vault.dir.join("b.md").exists());
}

#[test]
fn move_reports_a_missing_note() {
    let vault = TempVault::new();
    let revision = Revision {
        size_bytes: 0,
        modified_ns: None,
    };
    assert_eq!(
        vault
            .open()
            .move_note(&path("a.md"), &path("b.md"), revision)
            .unwrap_err(),
        Error::NoteNotFound
    );
}

#[test]
fn move_refuses_a_hidden_or_non_markdown_target() {
    let vault = TempVault::new();
    vault.file("a.md", "x");
    let opened = vault.open();
    let revision = revision_of(&opened, "a.md");
    for target in [".git/hooks/a.md", "a.txt"] {
        assert!(matches!(
            opened.move_note(&path("a.md"), &path(target), revision),
            Err(Error::NotANote { .. })
        ));
    }
    assert_eq!(vault.read("a.md"), "x");
}

#[cfg(unix)]
#[test]
fn move_refuses_a_target_through_a_symlinked_folder() {
    let vault = TempVault::new();
    let outside = TempVault::new();
    vault.file("a.md", "x");
    symlink(&outside.dir, &vault.dir.join("linked"));
    let opened = vault.open();
    let revision = revision_of(&opened, "a.md");
    assert_eq!(
        opened
            .move_note(&path("a.md"), &path("linked/a.md"), revision)
            .unwrap_err(),
        Error::SymlinkRefused
    );
    assert!(outside.names_in("").is_empty());
}

// --- trash_note ---

#[test]
fn trash_moves_the_note_into_dot_trash_keeping_its_folders() {
    let vault = TempVault::new();
    vault.file("notes/a.md", "x");
    let opened = vault.open();
    let revision = revision_of(&opened, "notes/a.md");
    let trashed = opened.trash_note(&path("notes/a.md"), revision).unwrap();
    assert_eq!(trashed, path(".trash/notes/a.md"));
    assert_eq!(vault.read(".trash/notes/a.md"), "x");
    assert!(opened.list_notes().unwrap().notes.is_empty());
}

#[test]
fn trash_numbers_a_name_already_in_the_trash() {
    let vault = TempVault::new();
    vault
        .file(".trash/notes/a.md", "older")
        .file(".trash/notes/a (2).md", "old")
        .file("notes/a.md", "new");
    let opened = vault.open();
    let revision = revision_of(&opened, "notes/a.md");
    let trashed = opened.trash_note(&path("notes/a.md"), revision).unwrap();
    assert_eq!(trashed, path(".trash/notes/a (3).md"));
    assert_eq!(vault.read(".trash/notes/a.md"), "older");
    assert_eq!(vault.read(".trash/notes/a (3).md"), "new");
}

#[test]
fn trash_refuses_when_the_note_changed_on_disk() {
    let vault = TempVault::new();
    vault.file("a.md", "old");
    let opened = vault.open();
    let revision = revision_of(&opened, "a.md");
    vault.file("a.md", "edited elsewhere");
    assert_eq!(
        opened.trash_note(&path("a.md"), revision).unwrap_err(),
        Error::Conflict
    );
    assert_eq!(vault.read("a.md"), "edited elsewhere");
}

#[test]
fn trash_refuses_notes_already_in_the_trash() {
    let vault = TempVault::new();
    vault.file(".trash/a.md", "x");
    let revision = Revision {
        size_bytes: 1,
        modified_ns: None,
    };
    assert!(matches!(
        vault.open().trash_note(&path(".trash/a.md"), revision),
        Err(Error::NotANote { .. })
    ));
}

#[cfg(unix)]
#[test]
fn trash_refuses_a_symlinked_trash_folder() {
    let vault = TempVault::new();
    let outside = TempVault::new();
    vault.file("a.md", "x");
    symlink(&outside.dir, &vault.dir.join(".trash"));
    let opened = vault.open();
    let revision = revision_of(&opened, "a.md");
    assert_eq!(
        opened.trash_note(&path("a.md"), revision).unwrap_err(),
        Error::SymlinkRefused
    );
    assert_eq!(vault.read("a.md"), "x");
    assert!(outside.names_in("").is_empty());
}

// --- errors ---

#[test]
fn io_errors_keep_the_category_and_drop_the_text() {
    let error = std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "/Users/someone/private/vault/a.md",
    );
    let mapped = Error::io("read the note", &error);
    assert_eq!(
        mapped,
        Error::Io {
            operation: "read the note".to_owned(),
            kind: "permission denied".to_owned(),
        }
    );
    assert!(!mapped.to_string().contains("/Users"));
}
