//! Declares the app's own commands so Tauri generates an `allow-<name>` permission for each.
//! A command missing here cannot be granted, and so cannot be called.

fn main() {
    let attributes = tauri_build::Attributes::new()
        .app_manifest(tauri_build::AppManifest::new().commands(&["app_info"]));
    if let Err(error) = tauri_build::try_build(attributes) {
        #[allow(
            clippy::print_stderr,
            reason = "a build script reports failure on stderr"
        )]
        {
            eprintln!("tauri build script failed: {error:#}");
        }
        std::process::exit(1);
    }
}
