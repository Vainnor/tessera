//! Tessera's sync server.
//!
//! The server is a relay and a backup. Clients will send it encrypted CRDT updates and blobs;
//! it stores and forwards them and never holds a key that can read them
//! (`docs/adr/0002-end-to-end-encrypted-sync.md`). Sync is not built yet: this crate serves a
//! health endpoint and fixes the config, logging and shutdown shape.

#![forbid(unsafe_code)]

pub mod config;
pub mod logging;
mod routes;

use anyhow::Context;
use tokio::net::TcpListener;

pub use routes::router;

/// Binds the configured address and serves until SIGINT or SIGTERM.
///
/// # Errors
///
/// When the address cannot be bound or the server fails while running.
pub async fn serve(config: config::Config) -> anyhow::Result<()> {
    let listener = TcpListener::bind(config.bind)
        .await
        .with_context(|| format!("binding {}", config.bind))?;
    tracing::info!(addr = %config.bind, version = env!("CARGO_PKG_VERSION"), "listening");
    axum::serve(listener, router())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serving")?;
    tracing::info!("stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "could not listen for Ctrl-C");
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(error) => {
                tracing::error!(%error, "could not listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
    tracing::info!("shutting down");
}
