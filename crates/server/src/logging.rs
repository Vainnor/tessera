//! Structured logging to stderr.
//!
//! Privacy rule (`.claude/rules/privacy.md`): the server never logs client IP addresses,
//! request bodies, or any id that identifies a user at `info` or above.

use tracing_subscriber::EnvFilter;

use crate::config::{LogConfig, LogFormat};

/// Installs the global subscriber. `RUST_LOG`, when set, replaces the configured filter.
///
/// # Errors
///
/// When the filter does not parse or a subscriber is already installed.
pub fn init(config: &LogConfig) -> anyhow::Result<()> {
    let filter = match std::env::var("RUST_LOG") {
        Ok(env) if !env.is_empty() => EnvFilter::try_new(env)?,
        _ => EnvFilter::try_new(&config.filter)?,
    };
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr);
    match config.format {
        LogFormat::Json => builder.json().flatten_event(true).try_init(),
        LogFormat::Pretty => builder.try_init(),
    }
    .map_err(|error| anyhow::anyhow!(error))
}
