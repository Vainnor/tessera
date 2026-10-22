//! The desktop shell. It opens the window and forwards IPC to `tessera-core`; it decides
//! nothing itself (`.claude/rules/tauri.md`).

#![forbid(unsafe_code)]

mod commands;
mod dto;

/// Builds and runs the app until the last window closes.
pub fn run() {
    let result = tauri::Builder::default()
        // Registered for the Rust-side folder picker only. The capability grants no `dialog:`
        // permission, so the plugin's own JavaScript commands are refused.
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::open_vault,
            commands::list_notes,
            commands::read_note,
        ])
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
