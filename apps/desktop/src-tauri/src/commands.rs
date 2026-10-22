//! IPC commands. Each one is a few lines that call `tessera-core`; the webview is untrusted, so
//! any argument a command takes is validated in Rust before use. Disk work runs on a blocking
//! thread so the window never stalls.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tessera_core::vault::{NoteListing, Vault, VaultPath};

use crate::dto::{
    CommandError, ListingDto, MovedNoteDto, NoteDto, OpenedVaultDto, parse_revision_token,
    revision_token,
};
use crate::settings::{self, Settings};

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
/// by the user in an OS dialog, never supplied by the webview, and is remembered for the next
/// launch. Returns `None` when cancelled.
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
        let opened = open_at(&root)?;
        let remembered = remember(&app, Some(&root));
        Ok(Some((opened, remembered)))
    })
    .await?;
    let Some((opened, remembered)) = opened else {
        return Ok(None);
    };
    Ok(Some(opened.install(&state, remembered)?))
}

/// Reopens the vault from the last launch, if there was one and it still exists. A vault that
/// has moved or been deleted is forgotten, and the app starts with no vault open.
#[tauri::command]
pub(crate) async fn restore_vault(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<OpenedVaultDto>, CommandError> {
    let opened = blocking(move || {
        let dir = config_dir(&app)?;
        let saved = settings::load(&dir).map_err(|_| {
            CommandError::new("settings-unreadable", "Tessera could not read its settings")
        })?;
        let Some(root) = saved.last_vault else {
            return Ok(None);
        };
        // The picker always yields an absolute path; anything else did not come from Tessera.
        if !Path::new(&root).is_absolute() {
            remember(&app, None);
            return Ok(None);
        }
        match open_at(Path::new(&root)) {
            Ok(opened) => Ok(Some(opened)),
            Err(error) if error.code == "vault-unavailable" => {
                remember(&app, None);
                Ok(None)
            }
            Err(error) => Err(error),
        }
    })
    .await?;
    match opened {
        Some(opened) => Ok(Some(opened.install(&state, true)?)),
        None => Ok(None),
    }
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

/// Renames or moves a note. Both paths and the revision come from the webview and are parsed
/// in Rust first; the core refuses to replace anything or to move a note that changed on disk.
#[tauri::command]
pub(crate) async fn move_note(
    from: String,
    to: String,
    revision: String,
    state: State<'_, AppState>,
) -> Result<MovedNoteDto, CommandError> {
    let from = VaultPath::parse(&from)?;
    let to = VaultPath::parse(&to)?;
    let expected = parse_revision_token(&revision)?;
    let vault = state.current()?;
    let moved = blocking(move || {
        vault
            .move_note(&from, &to, expected)
            .map(|revision| MovedNoteDto {
                path: to.as_str().to_owned(),
                revision: revision_token(revision),
            })
            .map_err(CommandError::from)
    })
    .await?;
    Ok(moved)
}

/// Moves a note to the vault's `.trash` folder and returns where it went.
#[tauri::command]
pub(crate) async fn trash_note(
    path: String,
    revision: String,
    state: State<'_, AppState>,
) -> Result<String, CommandError> {
    let path = VaultPath::parse(&path)?;
    let expected = parse_revision_token(&revision)?;
    let vault = state.current()?;
    blocking(move || {
        vault
            .trash_note(&path, expected)
            .map(|trashed| trashed.as_str().to_owned())
            .map_err(CommandError::from)
    })
    .await
}

/// A vault opened from disk, not yet the app's current vault.
struct Opened {
    vault: Arc<Vault>,
    name: String,
    listing: NoteListing,
}

impl Opened {
    fn install(self, state: &AppState, remembered: bool) -> Result<OpenedVaultDto, CommandError> {
        state.replace(self.vault)?;
        Ok(OpenedVaultDto {
            name: self.name,
            listing: self.listing.into(),
            remembered,
        })
    }
}

fn open_at(root: &Path) -> Result<Opened, CommandError> {
    let name = root
        .file_name()
        .map_or_else(|| "Vault".to_owned(), |n| n.to_string_lossy().into_owned());
    let root = root
        .to_str()
        .ok_or_else(|| CommandError::new("vault-unavailable", "the folder's path is not UTF-8"))?;
    let vault = Vault::open(root)?;
    let listing = vault.list_notes()?;
    Ok(Opened {
        vault: Arc::new(vault),
        name,
        listing,
    })
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, CommandError> {
    app.path()
        .app_config_dir()
        .map_err(|_| CommandError::new("settings-unreadable", "the settings folder is unavailable"))
}

/// Saves `root` as the vault to reopen at launch (`None` forgets it). Returns whether it worked;
/// failing to remember never stops the vault from opening, and the UI says so instead.
fn remember(app: &AppHandle, root: Option<&Path>) -> bool {
    let Ok(dir) = config_dir(app) else {
        return false;
    };
    // An unreadable settings file is replaced: it holds nothing but this one value.
    let mut saved = settings::load(&dir).unwrap_or_else(|_| Settings::default());
    saved.last_vault = root.and_then(Path::to_str).map(str::to_owned);
    settings::save(&dir, &saved).is_ok()
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
    use tessera_core::Error;

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
    fn open_at_reports_a_missing_folder_as_unavailable() {
        let missing = std::env::temp_dir().join("tessera-desktop-test-missing-vault");
        assert_eq!(
            open_at(&missing).err().map(|e| e.code),
            Some(CommandError::from(Error::VaultUnavailable).code)
        );
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
