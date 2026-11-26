//! Entry point for the `argus` CLI + TUI binary.

mod args;
mod commands;

use args::Cli;
use clap::Parser;
use std::process::ExitCode;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    let _ = color_eyre::install();
    let cli = Cli::parse();

    init_logging(cli.verbose, cli.quiet);

    let cancel = CancellationToken::new();
    let cancel_signal = cancel.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            cancel_signal.cancel();
        }
    });

    let verbose = cli.verbose;
    match commands::execute(cli, cancel).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            if verbose {
                eprintln!("Erro: {}\nDetalhes técnicos: {err:?}", err.friendly_message());
            } else {
                eprintln!("Erro: {}", err.friendly_message());
            }
            ExitCode::FAILURE
        }
    }
}

fn init_logging(verbose: bool, quiet: bool) {
    let default_level = if verbose {
        "debug"
    } else if quiet {
        "error"
    } else {
        "warn"
    };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_level));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}
