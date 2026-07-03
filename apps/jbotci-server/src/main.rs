use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, anyhow, bail};
#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_embeddings::native::setup_embeddings_with_progress;
use jbotci_embeddings::{SetupOptions, SetupProgress, UsePrecomputed};

#[tokio::main]
#[requires(true)]
#[ensures(true)]
async fn main() -> ExitCode {
    match run_args(std::env::args_os().skip(1).collect()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("jbotci-server: {error}");
            ExitCode::FAILURE
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
async fn run_args(args: Vec<OsString>) -> Result<()> {
    match args.split_first() {
        None => jbotci_server::run_server(jbotci_server::config_from_env()).await,
        Some((command, rest)) if command == OsStr::new("setup") => run_setup_args(rest),
        Some((command, _)) if command == OsStr::new("--help") || command == OsStr::new("-h") => {
            print_help();
            Ok(())
        }
        Some((command, _)) => bail!(
            "unknown command `{}`; run `jbotci-server --help` for usage",
            command.to_string_lossy()
        ),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_setup_args(args: &[OsString]) -> Result<()> {
    let mut embedding = false;
    let mut force = false;
    let mut use_precomputed = UsePrecomputed::Auto;
    let mut skip_validation = false;
    let mut model = jbotci_server::SERVER_DEFAULT_EMBEDDING_MODEL_KEY.to_owned();
    let mut index_dir = None;
    let mut model_dir = None;

    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        let Some(text) = arg.to_str() else {
            bail!(
                "setup option is not valid UTF-8: `{}`",
                arg.to_string_lossy()
            );
        };
        match text {
            "--embedding" => embedding = true,
            "--force" => force = true,
            "--skip-validation" => skip_validation = true,
            "--help" | "-h" => {
                print_setup_help();
                return Ok(());
            }
            "--model" => {
                index += 1;
                model = next_utf8_value(args, index, "--model")?;
            }
            "--index-dir" => {
                index += 1;
                index_dir = Some(next_path_value(args, index, "--index-dir")?);
            }
            "--model-dir" => {
                index += 1;
                model_dir = Some(next_path_value(args, index, "--model-dir")?);
            }
            "--use-precomputed" => {
                index += 1;
                let value = next_utf8_value(args, index, "--use-precomputed")?;
                use_precomputed = parse_use_precomputed(&value)?;
            }
            _ if text.starts_with("--model=") => {
                model = option_suffix(text, "--model=")?.to_owned();
            }
            _ if text.starts_with("--index-dir=") => {
                index_dir = Some(PathBuf::from(option_suffix(text, "--index-dir=")?));
            }
            _ if text.starts_with("--model-dir=") => {
                model_dir = Some(PathBuf::from(option_suffix(text, "--model-dir=")?));
            }
            _ if text.starts_with("--use-precomputed=") => {
                use_precomputed =
                    parse_use_precomputed(option_suffix(text, "--use-precomputed=")?)?;
            }
            _ => bail!("unknown setup option `{text}`"),
        }
        index += 1;
    }

    if !embedding {
        bail!("Choose at least one setup task, e.g. `jbotci-server setup --embedding`.");
    }
    let mut last_progress_key = String::new();
    let mut progress = |progress: SetupProgress| {
        let progress_key = format!(
            "{:?}:{}:{:?}",
            progress.phase, progress.kind, progress.percent
        );
        if progress_key == last_progress_key {
            return;
        }
        last_progress_key = progress_key;
        if let Some(percent) = progress.percent {
            eprintln!(
                "embedding setup: {}: {} ({percent}%)",
                progress.label, progress.detail
            );
        } else {
            eprintln!("embedding setup: {}: {}", progress.label, progress.detail);
        }
    };
    let report = setup_embeddings_with_progress(
        &SetupOptions {
            model_key: model,
            force,
            use_precomputed,
            skip_validation,
            index_dir,
            model_dir,
            ..SetupOptions::default()
        },
        &mut progress,
    )
    .map_err(|error| anyhow!(error.to_string()))?;
    println!(
        "Embedding setup complete.\nmodel: {}\nindex: {}\npack: {}\nsource: {}\ndictionary rows: {}\nCLL rows: {}",
        report
            .model_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "not checked".to_owned()),
        report.index_root.display(),
        report.pack_id,
        report.index_source.as_str(),
        report.dictionary_rows,
        report.cll_rows
    );
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| !value.is_empty()) || ret.is_err())]
fn next_utf8_value(args: &[OsString], index: usize, option: &str) -> Result<String> {
    let value = args
        .get(index)
        .ok_or_else(|| anyhow!("{option} requires a value"))?
        .to_str()
        .ok_or_else(|| anyhow!("{option} value is not valid UTF-8"))?;
    if value.is_empty() {
        bail!("{option} requires a non-empty value");
    }
    Ok(value.to_owned())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|path| !path.as_os_str().is_empty()) || ret.is_err())]
fn next_path_value(args: &[OsString], index: usize, option: &str) -> Result<PathBuf> {
    let value = args
        .get(index)
        .ok_or_else(|| anyhow!("{option} requires a path"))?;
    if value.is_empty() {
        bail!("{option} requires a non-empty path");
    }
    Ok(PathBuf::from(value))
}

#[requires(text.starts_with(prefix))]
#[ensures(ret.as_ref().is_ok_and(|value| !value.is_empty()) || ret.is_err())]
fn option_suffix<'a>(text: &'a str, prefix: &str) -> Result<&'a str> {
    let value = &text[prefix.len()..];
    if value.is_empty() {
        bail!(
            "{} requires a non-empty value",
            prefix.trim_end_matches('=')
        );
    }
    Ok(value)
}

#[requires(!value.is_empty())]
#[ensures(true)]
fn parse_use_precomputed(value: &str) -> Result<UsePrecomputed> {
    match value {
        "auto" => Ok(UsePrecomputed::Auto),
        "always" => Ok(UsePrecomputed::Always),
        "never" => Ok(UsePrecomputed::Never),
        _ => bail!("unknown --use-precomputed value `{value}`; expected auto, always, or never"),
    }
}

#[requires(true)]
#[ensures(true)]
fn print_help() {
    println!(
        "Usage: jbotci-server [COMMAND]\n\nCommands:\n  setup    Prepare deployment assets such as native embeddings\n\nOptions:\n  -h, --help    Print help"
    );
}

#[requires(true)]
#[ensures(true)]
fn print_setup_help() {
    println!(
        "Usage: jbotci-server setup [OPTIONS]\n\nOptions:\n      --embedding\n      --force\n      --use-precomputed <auto|always|never>  [default: auto]\n      --skip-validation\n      --model <MODEL>                        [default: f2llm-v2-80m-q4-k-m-320]\n      --index-dir <INDEX_DIR>\n      --model-dir <MODEL_DIR>\n  -h, --help                                Print help"
    );
}
