//! IPC commands. Each one is a few lines that call `tessera-core`; the webview is untrusted, so
//! any argument a command takes is validated in Rust before use. Disk work runs on a blocking
//! thread so the window never stalls.

use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tessera_core::vault::{Vault, VaultPath};

use crate::dto::{CommandError, ListingDto, NoteDto, OpenedVaultDto};

/// The open vault, shared by every command. `None` until the user picks a folder.
#[derive(Default)]
pub(crate) struct AppState {
    vault: Mutex<Option<Arc<Vault>>>,
}

impl AppState {
    fn current(&self) -> Result<Arc<Vault>, CommandError> {
        let guard = self
            .vault
            .lock()
            .map_err(|_| CommandError::new("internal", "the app state is unavailable"))?;
        guard
            .clone()
            .ok_or_else(|| CommandError::new("no-vault", "no vault is open"))
    }

    fn replace(&self, vault: Arc<Vault>) -> Result<(), CommandError> {
        let mut guard = self
            .vault
            .lock()
            .map_err(|_| CommandError::new("internal", "the app state is unavailable"))?;
        *guard = Some(vault);
        Ok(())
    }
}

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

/// Shows the native folder picker and opens the chosen folder as the vault. The path is chosen
/// by the user in an OS dialog, never supplied by the webview. Returns `None` when cancelled.
#[tauri::command]
pub(crate) async fn open_vault(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<OpenedVaultDto>, CommandError> {
    let opened = blocking(move || {
        let Some(picked) = app
            .dialog()
            .file()
            .set_title("Open a vault folder")
            .blocking_pick_folder()
        else {
            return Ok(None);
        };
        let root = picked
            .into_path()
            .map_err(|_| CommandError::new("vault-unavailable", "that folder cannot be opened"))?;
        let name = root
            .file_name()
            .map_or_else(|| "Vault".to_owned(), |n| n.to_string_lossy().into_owned());
        let root = root.to_str().ok_or_else(|| {
            CommandError::new("vault-unavailable", "the folder's path is not UTF-8")
        })?;
        let vault = Vault::open(root)?;
        let listing = vault.list_notes()?;
        Ok(Some((Arc::new(vault), name, listing)))
    })
    .await?;
    let Some((vault, name, listing)) = opened else {
        return Ok(None);
    };
    state.replace(vault)?;
    Ok(Some(OpenedVaultDto {
        name,
        listing: listing.into(),
    }))
}

/// Lists the open vault again, after the user asks to refresh.
#[tauri::command]
pub(crate) async fn list_notes(state: State<'_, AppState>) -> Result<ListingDto, CommandError> {
    let vault = state.current()?;
    let listing = blocking(move || vault.list_notes().map_err(CommandError::from)).await?;
    Ok(listing.into())
}

/// Reads one note. `path` comes from the webview, so it is parsed as a `VaultPath` first.
#[tauri::command]
pub(crate) async fn read_note(
    path: String,
    state: State<'_, AppState>,
) -> Result<NoteDto, CommandError> {
    let path = VaultPath::parse(&path)?;
    let vault = state.current()?;
    let note = blocking(move || vault.read_note(&path).map_err(CommandError::from)).await?;
    Ok(note.into())
}

async fn blocking<T, F>(work: F) -> Result<T, CommandError>
where
    F: FnOnce() -> Result<T, CommandError> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|_| CommandError::new("internal", "the operation stopped unexpectedly"))?
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

    #[test]
    fn commands_need_an_open_vault() {
        let state = AppState::default();
        assert_eq!(
            state.current().unwrap_err(),
            CommandError::new("no-vault", "no vault is open")
        );
    }
}
