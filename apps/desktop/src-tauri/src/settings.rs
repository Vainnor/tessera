//! The app's own settings file, `settings.json` in the OS app-config folder (on macOS,
//! `~/Library/Application Support/com.vainnor.tessera`). It stays on the device and holds only
//! what is listed in [`Settings`]. The file is read like any untrusted input: size first, then
//! parse with unknown fields refused.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "settings.json";
const MAX_BYTES: u64 = 64 * 1024;

/// Everything Tessera remembers between launches.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Settings {
    /// The absolute path of the vault to reopen at launch. Never sent to the webview.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) last_vault: Option<String>,
}

/// Why the settings could not be read or written. Holds no path.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SettingsError {
    TooLarge,
    Malformed,
    Io(ErrorKind),
}

/// Reads the settings in `dir`. A missing file is the default settings, not an error.
pub(crate) fn load(dir: &Path) -> Result<Settings, SettingsError> {
    let file = dir.join(FILE_NAME);
    let metadata = match fs::metadata(&file) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Settings::default()),
        Err(e) => return Err(SettingsError::Io(e.kind())),
    };
    if metadata.len() > MAX_BYTES {
        return Err(SettingsError::TooLarge);
    }
    let bytes = fs::read(&file).map_err(|e| SettingsError::Io(e.kind()))?;
    serde_json::from_slice(&bytes).map_err(|_| SettingsError::Malformed)
}

/// Writes the settings to `dir` through a temporary file, so a crash never leaves half a file.
pub(crate) fn save(dir: &Path, settings: &Settings) -> Result<(), SettingsError> {
    fs::create_dir_all(dir).map_err(|e| SettingsError::Io(e.kind()))?;
    let json = serde_json::to_vec_pretty(settings).map_err(|_| SettingsError::Malformed)?;
    let temp = dir.join(format!(".{FILE_NAME}.tmp"));
    fs::write(&temp, json)
        .and_then(|()| fs::rename(&temp, dir.join(FILE_NAME)))
        .map_err(|e| SettingsError::Io(e.kind()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let dir = std::env::temp_dir().join(format!(
                "tessera-settings-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _cleanup = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn missing_file_means_default_settings() {
        let dir = TempDir::new();
        assert_eq!(load(&dir.0), Ok(Settings::default()));
    }

    #[test]
    fn round_trips_the_last_vault() {
        let dir = TempDir::new();
        let settings = Settings {
            last_vault: Some("/synthetic/vault".to_owned()),
        };
        save(&dir.0, &settings).unwrap();
        assert_eq!(load(&dir.0), Ok(settings));
        let json = fs::read_to_string(dir.0.join(FILE_NAME)).unwrap();
        assert!(json.contains("\"lastVault\""));
    }

    #[test]
    fn creates_the_settings_folder() {
        let dir = TempDir::new();
        let nested = dir.0.join("not-yet");
        save(&nested, &Settings::default()).unwrap();
        assert_eq!(load(&nested), Ok(Settings::default()));
    }

    #[test]
    fn refuses_unknown_fields() {
        let dir = TempDir::new();
        fs::write(
            dir.0.join(FILE_NAME),
            r#"{"lastVault":"/v","telemetry":true}"#,
        )
        .unwrap();
        assert_eq!(load(&dir.0), Err(SettingsError::Malformed));
    }

    #[test]
    fn refuses_a_file_over_the_size_limit_before_parsing() {
        let dir = TempDir::new();
        fs::File::create(dir.0.join(FILE_NAME))
            .unwrap()
            .set_len(MAX_BYTES + 1)
            .unwrap();
        assert_eq!(load(&dir.0), Err(SettingsError::TooLarge));
    }
}
