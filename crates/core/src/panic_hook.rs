//! A panic hook that reports where a panic happened and nothing else.
//!
//! The default hook prints the panic's message, and messages from code that handles notes (an
//! Automerge assertion, an index out of range) can quote note text. `NoteHistory` catches panics
//! from Automerge, but the hook runs before the catch, so every app installs this one first.

use std::io::Write;
use std::panic;

/// Replaces the process's panic hook with one that prints only the file and line, shortened to
/// its last two path components so no build-machine home folder appears.
pub fn install_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let report = report(info.location().map(|at| (at.file(), at.line())));
        // `eprintln!` panics if stderr is closed, and a panic inside the hook aborts the process,
        // turning a caught panic into a crash. Nothing is left to tell about a failed write.
        let _ignored = writeln!(std::io::stderr().lock(), "{report}");
    }));
}

fn report(location: Option<(&str, u32)>) -> String {
    match location {
        Some((file, line)) => format!("tessera: internal error at {}:{line}", short(file)),
        None => "tessera: internal error".to_owned(),
    }
}

/// The last two components of `file`, split on either separator.
fn short(file: &str) -> &str {
    let mut cuts = file.rmatch_indices(['/', '\\']).map(|(index, _)| index);
    match (cuts.next(), cuts.next()) {
        (Some(_), Some(before)) => file.get(before + 1..).unwrap_or(file),
        _ => file,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_keeps_only_the_last_two_path_components() {
        assert_eq!(
            report(Some((
                "/Users/someone/.cargo/registry/src/index/automerge-0.12.0/src/op_set2/change/batch.rs",
                1121
            ))),
            "tessera: internal error at change/batch.rs:1121"
        );
        assert_eq!(
            report(Some((r"C:\Users\someone\src\lib.rs", 7))),
            "tessera: internal error at src/lib.rs:7".replace('/', r"\")
        );
        assert_eq!(
            report(Some(("lib.rs", 3))),
            "tessera: internal error at lib.rs:3"
        );
    }

    #[test]
    fn report_without_a_location_says_only_that_it_failed() {
        assert_eq!(report(None), "tessera: internal error");
    }
}
