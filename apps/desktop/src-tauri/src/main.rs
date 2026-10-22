//! Desktop entry point.

// No console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tessera_desktop_lib::run();
}
