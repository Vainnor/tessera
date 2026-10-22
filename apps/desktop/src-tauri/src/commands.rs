//! IPC commands. Each one is a few lines that call `tessera-core`; the webview is untrusted, so
//! any argument a command takes is validated in Rust before use.

use serde::Serialize;

/// What the about line in the window shows.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppInfo {
    name: &'static str,
    version: &'static str,
    core_version: String,
}

#[tauri::command]
pub(crate) fn app_info() -> AppInfo {
    AppInfo {
        name: "Tessera",
        version: env!("CARGO_PKG_VERSION"),
        core_version: tessera_core::version(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_reports_both_versions() {
        let info = app_info();
        assert_eq!(info.name, "Tessera");
        assert_eq!(info.core_version, tessera_core::version());
    }

    #[test]
    fn app_info_serializes_camel_case_for_the_ui() {
        let json = serde_json::to_value(app_info()).unwrap();
        assert_eq!(json["name"], "Tessera");
        assert_eq!(json["coreVersion"], tessera_core::version());
    }
}
