//! `tessera-server` binary: parse arguments, load config, start logging, serve.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, bail};
use tessera_server::config::Config;

const USAGE: &str = "usage: tessera-server [--config <path>]\n\n\
    Reads config from --config, else $TESSERA_SERVER_CONFIG, else built-in defaults.\n\
    RUST_LOG overrides the configured log filter.";

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            // Logging may not be up yet, so the fatal error goes straight to stderr.
            #[allow(
                clippy::print_stderr,
                reason = "last-resort report of a startup failure"
            )]
            {
                eprintln!("tessera-server: {error:#}");
            }
            ExitCode::FAILURE
        }
    }
}

async fn run() -> anyhow::Result<()> {
    let config_path = match parse_args(std::env::args().skip(1))? {
        Command::Run { config } => config,
        Command::Help => {
            #[allow(clippy::print_stdout, reason = "--help prints to stdout")]
            {
                println!("tessera-server {}\n\n{USAGE}", env!("CARGO_PKG_VERSION"));
            }
            return Ok(());
        }
    };
    let config_path =
        config_path.or_else(|| std::env::var_os("TESSERA_SERVER_CONFIG").map(PathBuf::from));
    let config = match &config_path {
        Some(path) => Config::load(path).with_context(|| format!("loading {}", path.display()))?,
        None => Config::default(),
    };
    tessera_server::logging::init(&config.log).context("starting logging")?;
    tessera_server::serve(config).await
}

enum Command {
    Help,
    Run { config: Option<PathBuf> },
}

fn parse_args(mut args: impl Iterator<Item = String>) -> anyhow::Result<Command> {
    let mut config = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => {
                let path = args.next().context("--config needs a path")?;
                config = Some(PathBuf::from(path));
            }
            "--help" | "-h" | "--version" | "-V" => return Ok(Command::Help),
            other => bail!("unknown argument {other:?}\n\n{USAGE}"),
        }
    }
    Ok(Command::Run { config })
}
