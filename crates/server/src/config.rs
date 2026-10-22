//! Server configuration, read from a TOML file.

use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;

use serde::Deserialize;

/// The largest config file the server reads. Real configs are a few hundred bytes.
const MAX_CONFIG_BYTES: u64 = 64 * 1024;

/// Server configuration. Unknown keys are an error, so a typo is caught at startup instead of
/// silently falling back to a default.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Config {
    /// Address to listen on. Defaults to loopback; put a TLS-terminating proxy in front to
    /// expose it.
    pub bind: SocketAddr,
    /// Logging.
    pub log: LogConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: SocketAddr::from((Ipv4Addr::LOCALHOST, 8787)),
            log: LogConfig::default(),
        }
    }
}

/// Logging configuration.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LogConfig {
    /// Output format.
    pub format: LogFormat,
    /// A `tracing` filter such as `info` or `tessera_server=debug`. `RUST_LOG` overrides it.
    pub filter: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::Json,
            filter: "info".to_owned(),
        }
    }
}

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// One JSON object per line, for log collectors.
    Json,
    /// Human-readable lines, for development.
    Pretty,
}

/// Why a config could not be loaded.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The file could not be read.
    #[error("could not read the config file")]
    Read(#[source] std::io::Error),
    /// The file is larger than the server will read.
    #[error("the config file is larger than {MAX_CONFIG_BYTES} bytes")]
    TooLarge,
    /// The file is not valid config.
    #[error("invalid config")]
    Parse(#[source] toml::de::Error),
}

impl Config {
    /// Reads and parses a config file.
    ///
    /// # Errors
    ///
    /// [`ConfigError`] when the file cannot be read, is too large or does not parse.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let size = std::fs::metadata(path).map_err(ConfigError::Read)?.len();
        if size > MAX_CONFIG_BYTES {
            return Err(ConfigError::TooLarge);
        }
        let text = std::fs::read_to_string(path).map_err(ConfigError::Read)?;
        Self::parse(&text)
    }

    /// Parses config from TOML text.
    ///
    /// # Errors
    ///
    /// [`ConfigError::Parse`] when the text is not valid config.
    pub fn parse(text: &str) -> Result<Self, ConfigError> {
        toml::from_str(text).map_err(ConfigError::Parse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_gives_loopback_defaults() {
        let config = Config::parse("").unwrap();
        assert_eq!(config, Config::default());
        assert_eq!(config.bind.to_string(), "127.0.0.1:8787");
        assert_eq!(config.log.format, LogFormat::Json);
    }

    #[test]
    fn reads_every_field() {
        let config = Config::parse(
            "bind = \"0.0.0.0:9000\"\n[log]\nformat = \"pretty\"\nfilter = \"debug\"\n",
        )
        .unwrap();
        assert_eq!(config.bind.to_string(), "0.0.0.0:9000");
        assert_eq!(
            config.log,
            LogConfig {
                format: LogFormat::Pretty,
                filter: "debug".to_owned()
            }
        );
    }

    #[test]
    fn rejects_unknown_key() {
        assert!(matches!(
            Config::parse("bnid = \"0.0.0.0:9000\""),
            Err(ConfigError::Parse(_))
        ));
    }

    #[test]
    fn rejects_bind_without_port() {
        assert!(matches!(
            Config::parse("bind = \"0.0.0.0\""),
            Err(ConfigError::Parse(_))
        ));
    }

    #[test]
    fn rejects_unknown_log_format() {
        assert!(matches!(
            Config::parse("[log]\nformat = \"xml\""),
            Err(ConfigError::Parse(_))
        ));
    }

    #[test]
    fn load_reports_a_missing_file() {
        let missing = std::env::temp_dir().join("tessera-server-test-missing.toml");
        assert!(matches!(Config::load(&missing), Err(ConfigError::Read(_))));
    }
}
