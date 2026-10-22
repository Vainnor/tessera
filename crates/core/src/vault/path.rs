//! Validated paths inside a vault.

use std::path::Path;

use crate::Error;

/// The longest path, in bytes, that a vault may contain. Shorter than every OS limit so a vault
/// syncs to any platform.
pub const MAX_PATH_BYTES: usize = 1024;

/// The longest single path component, in bytes (the common filesystem limit).
pub const MAX_COMPONENT_BYTES: usize = 255;

/// Names Windows refuses as a file or folder name, with or without an extension.
const WINDOWS_RESERVED: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// A validated path relative to the vault root, always written with `/` separators.
///
/// Every path that arrives from outside the core (a UI, a sync peer, an import) is parsed into
/// a `VaultPath` before it reaches the filesystem, so it cannot escape the vault or produce a
/// name that some platform cannot store.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VaultPath(String);

impl VaultPath {
    /// Validates `raw` as a path inside a vault.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidPath`] when the path is empty, absolute, too long, uses `\`, contains
    /// an empty, `.` or `..` component, a control character, a character Windows forbids, a
    /// trailing dot or space, or a Windows reserved name.
    pub fn parse(raw: &str) -> Result<Self, Error> {
        let invalid = |reason: &str| Error::InvalidPath {
            reason: reason.to_owned(),
        };
        if raw.is_empty() {
            return Err(invalid("empty"));
        }
        if raw.len() > MAX_PATH_BYTES {
            return Err(invalid("too long"));
        }
        if raw.starts_with('/') || raw.contains('\\') || has_drive_prefix(raw) {
            return Err(invalid("must be relative and use '/'"));
        }
        for component in raw.split('/') {
            validate_component(component).map_err(invalid)?;
        }
        Ok(Self(raw.to_owned()))
    }

    /// The path as stored, with `/` separators.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The path's components, root first.
    pub(crate) fn components(&self) -> impl Iterator<Item = &str> {
        self.0.split('/')
    }

    /// Whether any component starts with `.`, the convention for app and tool folders such as
    /// `.obsidian`, `.git` and `.trash`. Note operations leave these alone.
    #[must_use]
    pub fn is_hidden(&self) -> bool {
        self.components().any(|c| c.starts_with('.'))
    }

    /// Whether the path names a markdown note (`.md`, any case).
    #[must_use]
    pub fn is_markdown(&self) -> bool {
        Path::new(&self.0)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
    }
}

fn has_drive_prefix(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    bytes.len() >= 2
        && bytes.first().is_some_and(u8::is_ascii_alphabetic)
        && bytes.get(1) == Some(&b':')
}

fn validate_component(component: &str) -> Result<(), &'static str> {
    if component.is_empty() {
        return Err("empty component");
    }
    if component == "." || component == ".." {
        return Err("'.' and '..' are not allowed");
    }
    if component.len() > MAX_COMPONENT_BYTES {
        return Err("component too long");
    }
    if component
        .chars()
        .any(|c| c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
    {
        return Err("contains a character some platforms cannot store");
    }
    if component.ends_with('.') || component.ends_with(' ') {
        return Err("ends with a dot or space");
    }
    let stem = component
        .split('.')
        .next()
        .unwrap_or(component)
        .to_ascii_lowercase();
    if WINDOWS_RESERVED.contains(&stem.as_str()) {
        return Err("reserved name on Windows");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reason(raw: &str) -> String {
        match VaultPath::parse(raw) {
            Err(Error::InvalidPath { reason }) => reason,
            other => panic!("expected InvalidPath for {raw:?}, got {other:?}"),
        }
    }

    #[test]
    fn accepts_nested_markdown_path() {
        let path = VaultPath::parse("Projects/Tessera/Plan 2026.md").unwrap();
        assert_eq!(path.as_str(), "Projects/Tessera/Plan 2026.md");
        assert!(path.is_markdown());
    }

    #[test]
    fn accepts_attachment_and_reports_it_is_not_markdown() {
        let path = VaultPath::parse("attachments/sketch.png").unwrap();
        assert!(!path.is_markdown());
    }

    #[test]
    fn markdown_extension_is_case_insensitive() {
        assert!(VaultPath::parse("README.MD").unwrap().is_markdown());
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(reason(""), "empty");
    }

    #[test]
    fn rejects_absolute_unix_path() {
        assert_eq!(reason("/etc/passwd"), "must be relative and use '/'");
    }

    #[test]
    fn rejects_windows_drive_path() {
        assert_eq!(reason("C:/Users/note.md"), "must be relative and use '/'");
    }

    #[test]
    fn rejects_backslash_separator() {
        assert_eq!(reason("notes\\a.md"), "must be relative and use '/'");
    }

    #[test]
    fn rejects_parent_traversal() {
        assert_eq!(
            reason("notes/../../secret.md"),
            "'.' and '..' are not allowed"
        );
    }

    #[test]
    fn rejects_current_dir_component() {
        assert_eq!(reason("./a.md"), "'.' and '..' are not allowed");
    }

    #[test]
    fn rejects_empty_component() {
        assert_eq!(reason("notes//a.md"), "empty component");
        assert_eq!(reason("notes/"), "empty component");
    }

    #[test]
    fn rejects_control_character() {
        assert_eq!(
            reason("a\u{0}.md"),
            "contains a character some platforms cannot store"
        );
    }

    #[test]
    fn rejects_windows_forbidden_character() {
        assert_eq!(
            reason("what?.md"),
            "contains a character some platforms cannot store"
        );
    }

    #[test]
    fn rejects_trailing_dot_or_space() {
        assert_eq!(reason("notes./a.md"), "ends with a dot or space");
        assert_eq!(reason("a.md "), "ends with a dot or space");
    }

    #[test]
    fn rejects_windows_reserved_name_with_extension() {
        assert_eq!(reason("Con.md"), "reserved name on Windows");
        assert_eq!(reason("logs/lpt1.txt"), "reserved name on Windows");
    }

    #[test]
    fn allows_reserved_word_inside_a_longer_name() {
        assert!(VaultPath::parse("console.md").is_ok());
    }

    #[test]
    fn rejects_path_over_the_byte_limit() {
        let long = format!("{}.md", "a/".repeat(MAX_PATH_BYTES / 2));
        assert_eq!(reason(&long), "too long");
    }

    #[test]
    fn rejects_component_over_the_byte_limit() {
        let long = format!("{}.md", "a".repeat(MAX_COMPONENT_BYTES));
        assert_eq!(reason(&long), "component too long");
    }

    #[test]
    fn hidden_when_any_component_starts_with_a_dot() {
        assert!(VaultPath::parse(".obsidian/app.json").unwrap().is_hidden());
        assert!(VaultPath::parse("notes/.draft.md").unwrap().is_hidden());
        assert!(!VaultPath::parse("notes/draft.md").unwrap().is_hidden());
    }
}
