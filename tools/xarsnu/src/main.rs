use std::error::Error;
use std::path::PathBuf;
use std::process::ExitCode;

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use xarsnu::{OpenRouterClient, report_file, run as run_live};

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
    let first = arguments
        .next()
        .ok_or("usage: xarsnu <config.toml> | xarsnu report <transcript.jsonl>")?;
    if first == "report" {
        let path = arguments
            .next()
            .map(PathBuf::from)
            .ok_or("usage: xarsnu report <transcript.jsonl>")?;
        if arguments.next().is_some() {
            return Err("usage: xarsnu report <transcript.jsonl>".into());
        }
        print!("{}", report_file(&path)?);
        return Ok(());
    }
    if arguments.next().is_some() {
        return Err("usage: xarsnu <config.toml>".into());
    }
    let path = PathBuf::from(first);
    let summary = match run_live(&path, OpenRouterClient::from_env) {
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
