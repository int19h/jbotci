use std::fs;
use std::io::Read;
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
    #[arg(long = "format", alias = "termoha")]
    format: Option<String>,
    #[arg(long = "trace", alias = "plivei")]
    trace: Option<String>,
    #[arg(long = "no-postproc", alias = "na-velruhe")]
    no_postproc: bool,
    #[arg(long = "camxes")]
    camxes: bool,
    #[arg()]
    text: Vec<String>,
}

impl TextInput {
    fn read_text(&self) -> Result<String> {
        match (&self.file, self.text.is_empty()) {
            (Some(path), _) => fs::read_to_string(path)
                .map_err(|source| anyhow!("failed to read `{}`: {source}", path.display())),
            (None, false) => Ok(self.text.join(" ")),
            (None, true) => {
                let mut input = String::new();
                let mut stdin = std::io::stdin();
                stdin
                    .read_to_string(&mut input)
                    .map_err(|source| anyhow!("failed to read stdin: {source}"))?;
                Ok(input)
            }
        }
    }
}

#[derive(Debug, Args)]
struct SearchInput {
    #[arg(short = 'n', long = "count")]
    count: Option<usize>,
    #[arg(long = "index")]
    index: bool,
    #[arg(long = "valsi")]
    valsi: Option<String>,
    #[arg(long = "rafsi")]
    rafsi: Option<String>,
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
        Command::Vlasei(input) => {
            let _ = input.read_text()?;
            command_not_implemented("vlasei")
        }
        Command::Gentufa(input) => {
            let _ = input.read_text()?;
            command_not_implemented("gentufa")
        }
        Command::Mulgau(input) => {
            let _ = input.read_text()?;
            command_not_implemented("mulgau")
        }
        Command::Tersmu(input) => {
            let _ = input.read_text()?;
            command_not_implemented("tersmu")
        }
        Command::Vlacku(_input) => command_not_implemented("vlacku"),
        Command::Jvozba(_input) => command_not_implemented("jvozba"),
        Command::Cukta(_input) => command_not_implemented("cukta"),
        Command::Zbasu(input) => {
            let _ = input.read_text()?;
            command_not_implemented("zbasu")
        }
    }
}

fn command_not_implemented(command: &str) -> Result<()> {
    Err(anyhow!(
        "`{command}` is scaffolded but its implementation has not been ported yet"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_and_english_aliases() {
        assert!(matches!(
            Cli::try_parse_from(["jbotci", "vlasei", "coi"])
                .expect("canonical command")
                .command,
            Command::Vlasei(_)
        ));
        assert!(matches!(
            Cli::try_parse_from(["jbotci", "lex", "coi"])
                .expect("alias command")
                .command,
            Command::Vlasei(_)
        ));
        assert!(Cli::try_parse_from(["jbotci", "server"]).is_err());
        assert!(Cli::try_parse_from(["jbotci", "selfu"]).is_err());
    }

    #[test]
    fn joins_positional_text() {
        let input = TextInput {
            file: None,
            format: None,
            trace: None,
            no_postproc: false,
            camxes: false,
            text: vec!["coi".into(), "rodo".into()],
        };
        assert_eq!(input.read_text().expect("text"), "coi rodo");
    }
}
