use bityzba::{invariant, requires};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, anyhow};
use clap::{Args, Parser, Subcommand, ValueEnum};
use jbotci_morphology::segment_words_with_modifiers;
use jbotci_output::{BracketRenderOptions, pretty_brackets_with_options};
use jbotci_syntax::parse_syntax_tree;
use owo_colors::{OwoColorize, Stream};

#[derive(Debug, Parser)]
#[command(name = "jbotci")]
#[command(about = "Command-line Lojban toolkit")]
#[invariant(true)]
struct Cli {
    #[arg(long = "color", global = true)]
    color: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
#[invariant(true)]
enum Command {
    #[command(name = "vlasei", visible_alias = "lex")]
    Vlasei(TextInput),
    #[command(name = "gentufa", visible_alias = "parse")]
    Gentufa(GentufaInput),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[invariant(true)]
enum GentufaFormat {
    Compact,
    Raw,
}

#[derive(Debug, Args)]
#[invariant(true)]
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
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn read_text(&self) -> Result<String> {
        read_text_input(self.file.as_ref(), &self.text)
    }
}

#[derive(Debug, Args)]
#[invariant(true)]
struct GentufaInput {
    #[arg(long = "file", alias = "sfaile")]
    file: Option<PathBuf>,
    #[arg(
        long = "turtau",
        visible_alias = "format",
        default_value_t = GentufaFormat::Compact,
        value_enum
    )]
    format: GentufaFormat,
    #[arg(long = "trace", alias = "plivei")]
    trace: Option<String>,
    #[arg(long = "no-postproc", alias = "na-velruhe")]
    no_postproc: bool,
    #[arg(long = "camxes")]
    camxes: bool,
    #[arg(long = "skicu", visible_alias = "defs")]
    definitions: bool,
    #[arg()]
    text: Vec<String>,
}

impl GentufaInput {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn read_text(&self) -> Result<String> {
        read_text_input(self.file.as_ref(), &self.text)
    }
}

#[derive(Debug, Args)]
#[invariant(true)]
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
#[invariant(true)]
struct JvozbaInput {
    #[arg(long = "cmevla")]
    cmevla: bool,
    #[arg()]
    parts: Vec<String>,
}

#[requires(true)]
#[ensures(true)]
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("jbotci: {error}");
            ExitCode::FAILURE
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn run() -> Result<()> {
    let cli = Cli::parse();
    let color_enabled = cli.color || stdout_supports_color();
    let mut stdout = std::io::stdout();
    run_cli(cli, &mut stdout, color_enabled)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_cli<W: Write>(cli: Cli, stdout: &mut W, color_enabled: bool) -> Result<()> {
    let color_enabled = cli.color || color_enabled;
    match cli.command {
        Command::Vlasei(input) => {
            let text = input.read_text()?;
            let words = segment_words_with_modifiers(&text)?;
            for word in words {
                writeln!(stdout, "{word}")?;
            }
            Ok(())
        }
        Command::Gentufa(input) => run_gentufa(input, stdout, color_enabled),
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

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_gentufa<W: Write>(input: GentufaInput, stdout: &mut W, color_enabled: bool) -> Result<()> {
    validate_gentufa_options(&input)?;
    let text = input.read_text()?;
    let words = segment_words_with_modifiers(&text)?;
    let parsed = parse_syntax_tree(&words)?;
    match input.format {
        GentufaFormat::Compact => {
            let rendered = pretty_brackets_with_options(
                &parsed.parse_tree,
                &text,
                BracketRenderOptions {
                    color: color_enabled,
                },
            )?;
            writeln!(stdout, "{rendered}")?;
        }
        GentufaFormat::Raw => {
            writeln!(stdout, "{parsed:#?}")?;
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_gentufa_options(input: &GentufaInput) -> Result<()> {
    if input.definitions {
        match input.format {
            GentufaFormat::Compact => Err(anyhow!(
                "`--skicu`/`--defs` is accepted for compact output, but dictionary definition rendering has not been ported yet"
            )),
            GentufaFormat::Raw => Err(anyhow!(
                "`--skicu`/`--defs` is only meaningful with `--turtau compact`; dictionary definition rendering has not been ported yet"
            )),
        }
    } else {
        Ok(())
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn read_text_input(file: Option<&PathBuf>, text: &[String]) -> Result<String> {
    match (file, text.is_empty()) {
        (Some(path), _) => fs::read_to_string(path)
            .map_err(|source| anyhow!("failed to read `{}`: {source}", path.display())),
        (None, false) => Ok(text.join(" ")),
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

#[requires(true)]
#[ensures(true)]
fn stdout_supports_color() -> bool {
    "x".if_supports_color(Stream::Stdout, |text| text.red())
        .to_string()
        != "x"
}

#[requires(true)]
#[ensures(true)]
fn command_not_implemented(command: &str) -> Result<()> {
    Err(anyhow!(
        "`{command}` is scaffolded but its implementation has not been ported yet"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    #[test]
    #[requires(true)]
    #[ensures(true)]
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
    #[requires(true)]
    #[ensures(true)]
    fn parses_gentufa_formats_and_flags() {
        let Command::Gentufa(default_input) = Cli::try_parse_from(["jbotci", "gentufa", "coi"])
            .expect("default gentufa")
            .command
        else {
            panic!("expected gentufa command")
        };
        assert_eq!(default_input.format, GentufaFormat::Compact);

        let Command::Gentufa(compact_input) =
            Cli::try_parse_from(["jbotci", "gentufa", "--turtau", "compact", "coi"])
                .expect("turtau compact")
                .command
        else {
            panic!("expected gentufa command")
        };
        assert_eq!(compact_input.format, GentufaFormat::Compact);

        let Command::Gentufa(alias_input) =
            Cli::try_parse_from(["jbotci", "gentufa", "--format", "compact", "coi"])
                .expect("format alias")
                .command
        else {
            panic!("expected gentufa command")
        };
        assert_eq!(alias_input.format, GentufaFormat::Compact);

        let Command::Gentufa(raw_input) =
            Cli::try_parse_from(["jbotci", "gentufa", "--turtau", "raw", "--skicu", "coi"])
                .expect("raw with skicu parses")
                .command
        else {
            panic!("expected gentufa command")
        };
        assert_eq!(raw_input.format, GentufaFormat::Raw);
        assert!(raw_input.definitions);

        let Command::Gentufa(defs_input) =
            Cli::try_parse_from(["jbotci", "gentufa", "--defs", "coi"])
                .expect("defs alias")
                .command
        else {
            panic!("expected gentufa command")
        };
        assert!(defs_input.definitions);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_unknown_gentufa_format_and_word_kind_flag() {
        assert_eq!(
            Cli::try_parse_from(["jbotci", "gentufa", "--turtau", "xml", "coi"])
                .expect_err("unknown format")
                .kind(),
            ErrorKind::InvalidValue
        );
        assert_eq!(
            Cli::try_parse_from(["jbotci", "gentufa", "--wordKind", "coi"])
                .expect_err("wordKind is not supported")
                .kind(),
            ErrorKind::UnknownArgument
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_help_lists_formats_and_compact_flags() {
        let error = Cli::try_parse_from(["jbotci", "gentufa", "--help"]).expect_err("help");
        assert_eq!(error.kind(), ErrorKind::DisplayHelp);
        let help = error.to_string();
        assert!(help.contains("--turtau"));
        assert!(help.contains("--format"));
        assert!(help.contains("compact"));
        assert!(help.contains("raw"));
        assert!(help.contains("--skicu"));
        assert!(help.contains("--defs"));
        assert!(!help.contains("--wordKind"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_compact_output_matches_bracket_renderer() {
        let cli =
            Cli::try_parse_from(["jbotci", "gentufa", "mi", "klama"]).expect("gentufa compact");
        let mut output = Vec::new();
        run_cli(cli, &mut output, false).expect("gentufa run");

        let text = "mi klama";
        let words = segment_words_with_modifiers(text).expect("morphology");
        let parsed = parse_syntax_tree(&words).expect("syntax");
        let expected = pretty_brackets_with_options(
            &parsed.parse_tree,
            text,
            BracketRenderOptions { color: false },
        )
        .expect("brackets");
        assert_eq!(
            String::from_utf8(output).expect("utf8"),
            format!("{expected}\n")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_raw_output_is_debug_syntax_parse() {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "--turtau", "raw", "mi", "klama"])
            .expect("gentufa raw");
        let mut output = Vec::new();
        run_cli(cli, &mut output, false).expect("gentufa run");
        let output = String::from_utf8(output).expect("utf8");
        assert!(output.contains("SyntaxParse"));
        assert!(output.contains("parse_tree"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_definitions_report_not_implemented() {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "--defs", "mi", "klama"])
            .expect("gentufa defs");
        let error = run_cli(cli, &mut Vec::new(), false).expect_err("defs not implemented");
        assert!(error.to_string().contains("definition rendering"));

        let cli = Cli::try_parse_from([
            "jbotci", "gentufa", "--turtau", "raw", "--skicu", "mi", "klama",
        ])
        .expect("gentufa raw defs");
        let error = run_cli(cli, &mut Vec::new(), false).expect_err("raw defs not implemented");
        assert!(error.to_string().contains("only meaningful"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_color_flag_forces_ansi_compact_output() {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "--color", "mi", "klama"])
            .expect("gentufa color");
        assert!(cli.color);
        let mut output = Vec::new();
        run_cli(cli, &mut output, false).expect("gentufa color run");
        let output = String::from_utf8(output).expect("utf8");
        assert!(output.contains("\x1b["));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
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
