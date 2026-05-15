use std::process::ExitCode;

use anyhow::{Result, anyhow};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "jbotci-server")]
#[command(about = "Server application for jbotci web and HTTP integrations")]
struct Cli {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    #[arg(long, default_value_t = 8080)]
    port: u16,
    #[arg(long, default_value = "/jbotci")]
    base_path: String,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("jbotci-server: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    Err(anyhow!(
        "server is scaffolded for {}:{}{} but is not implemented yet",
        cli.host,
        cli.port,
        cli.base_path
    ))
}
