use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use xarsnu::{OpenRouterClient, dialog_file, report_file, run as run_live};

const USAGE: &str = "usage: xarsnu <config.toml> | xarsnu report [--dialog] <transcript.jsonl>";

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
    let mut arguments = std::env::args_os().skip(1);
    let first = arguments.next().ok_or(USAGE)?;
    if first == "report" {
        let first_report_argument = arguments.next().ok_or(USAGE)?;
        let dialog_only = first_report_argument == "--dialog";
        let path = if dialog_only {
            arguments.next().map(PathBuf::from).ok_or(USAGE)?
        } else {
            PathBuf::from(first_report_argument)
        };
        if arguments.next().is_some() {
            return Err(USAGE.into());
        }
        if dialog_only {
            print!("{}", dialog_file(&path)?);
        } else {
            print!("{}", report_file(&path)?);
        }
        return Ok(());
    }
    if arguments.next().is_some() {
        return Err("usage: xarsnu <config.toml>".into());
    }
    let path = PathBuf::from(first);
    let summary = match run_live(&path, OpenRouterClient::from_env_with_timeout) {
        Ok(summary) => summary,
        Err(error) => {
            if let Some(transcript_path) = error.transcript_path()
                && transcript_path.exists()
            {
                println!("transcript: {}", transcript_path.display());
                println!("outcome: runtime failure");
            }
            return Err(error.into());
        }
    };
    println!("transcript: {}", summary.transcript_path.display());
    println!("outcome: {}", summary.outcome_line());
    Ok(())
}
