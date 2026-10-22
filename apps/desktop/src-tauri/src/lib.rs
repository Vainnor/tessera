//! The desktop shell. It opens the window and forwards IPC to `tessera-core`; it decides
//! nothing itself (`.claude/rules/tauri.md`).

#![forbid(unsafe_code)]

mod commands;

/// Builds and runs the app until the last window closes.
pub fn run() {
    let result = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::app_info])
        .run(tauri::generate_context!());
    if let Err(error) = result {
        // The window never opened, so stderr is the only place left to report it.
        #[allow(
            clippy::print_stderr,
            reason = "fatal startup error before any UI exists"
        )]
        {
            eprintln!("tessera: could not start: {error}");
        }
        std::process::exit(1);
    }
}
