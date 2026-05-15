use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, anyhow};
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "jbotci")]
#[command(about = "Command-line Lojban toolkit")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(name = "vlasei", visible_alias = "lex")]
    Vlasei(TextInput),
    #[command(name = "gentufa", visible_alias = "parse")]
    Gentufa(TextInput),
    #[command(name = "mulgau", visible_alias = "completions")]
    Mulgau(TextInput),
    #[command(name = "tersmu")]
    Tersmu(TextInput),
    #[command(name = "vlacku", visible_alias = "dict")]
    Vlacku(SearchInput),
    #[command(name = "jvozba")]
    Jvozba(JvozbaInput),
    #[command(name = "cukta", visible_alias = "book")]
    Cukta(SearchInput),
    #[command(name = "zbasu")]
    Zbasu(TextInput),
}

#[derive(Debug, Args)]
struct TextInput {
    #[arg(long = "file", alias = "sfaile")]
    file: Option<PathBuf>,
    #[arg()]
    text: Vec<String>,
}

#[derive(Debug, Args)]
struct SearchInput {
    #[arg(short = 'n', long = "count")]
    count: Option<usize>,
    #[arg()]
    query: Vec<String>,
}

#[derive(Debug, Args)]
struct JvozbaInput {
    #[arg(long = "cmevla")]
    cmevla: bool,
    #[arg()]
    parts: Vec<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("jbotci: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Vlasei(input) => command_not_implemented("vlasei", &format!("{input:?}")),
        Command::Gentufa(input) => command_not_implemented("gentufa", &format!("{input:?}")),
        Command::Mulgau(input) => command_not_implemented("mulgau", &format!("{input:?}")),
        Command::Tersmu(input) => command_not_implemented("tersmu", &format!("{input:?}")),
        Command::Vlacku(input) => command_not_implemented("vlacku", &format!("{input:?}")),
        Command::Jvozba(input) => command_not_implemented("jvozba", &format!("{input:?}")),
        Command::Cukta(input) => command_not_implemented("cukta", &format!("{input:?}")),
        Command::Zbasu(input) => command_not_implemented("zbasu", &format!("{input:?}")),
    }
}

fn command_not_implemented(command: &str, _debug_args: &str) -> Result<()> {
    Err(anyhow!(
        "`{command}` is scaffolded but its implementation has not been ported yet"
    ))
}
