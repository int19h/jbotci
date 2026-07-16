use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use xarsnu::{OpenRouterClient, RunAccounting, RunConfig};

#[requires(true)]
#[ensures(true)]
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xarsnu: {error}");
            ExitCode::FAILURE
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run() -> Result<(), Box<dyn Error>> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: xarsnu <config.toml>")?;
    let source = fs::read_to_string(&path)?;
    let config = RunConfig::from_toml(&source)?;
    let _client = OpenRouterClient::from_env()?;
    let accounting = RunAccounting::new(config.caps.max_cost_usd)?;
    println!(
        "loaded scenario `{}` with {} participants and ${:.4} budget",
        config.scenario,
        config.participants.len(),
        config.caps.max_cost_usd
    );
    debug_assert_eq!(accounting.usage().cost_usd, 0.0);
    Ok(())
}
