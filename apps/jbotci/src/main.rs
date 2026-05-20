use bityzba::{invariant, requires};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, anyhow};
use clap::{Args, Parser, Subcommand, ValueEnum};
use jbotci_morphology::segment_words_with_modifiers;
use jbotci_output::{BracketRenderOptions, compact_json_string, pretty_brackets_with_options};
use jbotci_syntax::{
    ExperimentalConstruct, ParseOptions, SyntaxParse, SyntaxWarning,
    parse_syntax_tree_with_source_and_options,
};
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
    #[value(alias = "djeisone")]
    Json,
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
    let mut stderr = std::io::stderr();
    run_cli(cli, &mut stdout, &mut stderr, color_enabled)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_cli<WOut: Write, WErr: Write>(
    cli: Cli,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_enabled: bool,
) -> Result<()> {
    let color_enabled = cli.color || color_enabled;
    match cli.command {
        Command::Vlasei(input) => {
            let text = input.read_text()?;
            let words = segment_words_with_modifiers(&text)?;
            if matches!(input.format.as_deref(), Some("json" | "djeisone")) {
                let rendered = compact_json_string(&words)?;
                writeln!(stdout, "{}", colorize_json(&rendered, color_enabled))?;
            } else {
                for word in words {
                    writeln!(stdout, "{word}")?;
                }
            }
            Ok(())
        }
        Command::Gentufa(input) => run_gentufa(input, stdout, stderr, color_enabled),
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
fn run_gentufa<WOut: Write, WErr: Write>(
    input: GentufaInput,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_enabled: bool,
) -> Result<()> {
    validate_gentufa_options(&input)?;
    let warning_source = input_source_label(input.file.as_ref(), input.text.is_empty());
    let text = input.read_text()?;
    let words = segment_words_with_modifiers(&text)?;
    let parsed =
        parse_syntax_tree_with_source_and_options(&words, &text, &ParseOptions::default())?;
    render_syntax_warnings(&parsed, &text, &warning_source, stderr)?;
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
            writeln!(stdout, "{:#?}", parsed.parse_tree)?;
        }
        GentufaFormat::Json => {
            let rendered = compact_json_string(&parsed.parse_tree)?;
            writeln!(stdout, "{}", colorize_json(&rendered, color_enabled))?;
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn input_source_label(file: Option<&PathBuf>, stdin: bool) -> String {
    match file {
        Some(path) => path.display().to_string(),
        None if stdin => "<stdin>".to_owned(),
        None => "<input>".to_owned(),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn render_syntax_warnings<W: Write>(
    parsed: &SyntaxParse,
    source: &str,
    source_label: &str,
    stderr: &mut W,
) -> Result<()> {
    for warning in &parsed.warnings {
        let (line, column) = warning_line_column(source, warning);
        writeln!(
            stderr,
            "{source_label}:{line}:{column}: warning: experimental syntax: {}",
            syntax_warning_message(warning)
        )?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn syntax_warning_message(warning: &SyntaxWarning) -> String {
    let message = warning.message();
    if warning.kind == ExperimentalConstruct::ExperimentalCmavo
        && let Some(word) = warning.anchor.visible_word()
    {
        return format!("{message}: {}", word.canonical_phonemes());
    }
    message.to_owned()
}

#[requires(true)]
#[ensures(ret.0 > 0 && ret.1 > 0)]
fn warning_line_column(source: &str, warning: &SyntaxWarning) -> (usize, usize) {
    let char_offset = warning
        .anchor
        .visible_word()
        .map_or(0, |word| word.span.char_start);
    char_offset_to_line_column(source, char_offset)
}

#[requires(true)]
#[ensures(ret.0 > 0 && ret.1 > 0)]
fn char_offset_to_line_column(source: &str, char_offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;
    for (index, ch) in source.chars().enumerate() {
        if index == char_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_gentufa_options(input: &GentufaInput) -> Result<()> {
    if input.definitions {
        match input.format {
            GentufaFormat::Compact => Err(anyhow!(
                "`--skicu`/`--defs` is accepted for compact output, but dictionary definition rendering has not been ported yet"
            )),
            GentufaFormat::Raw | GentufaFormat::Json => Err(anyhow!(
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
#[ensures(!enabled -> ret == text)]
fn colorize_json(text: &str, enabled: bool) -> String {
    if !enabled {
        return text.to_owned();
    }
    let chars = text.chars().collect::<Vec<_>>();
    let mut output = String::new();
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            '{' | '}' | '[' | ']' => {
                output.push_str(&chars[index].to_string().cyan().to_string());
                index += 1;
            }
            ':' | ',' => {
                output.push_str(&chars[index].to_string().bright_black().to_string());
                index += 1;
            }
            '"' => {
                let start = index;
                index += 1;
                let mut escaped = false;
                while index < chars.len() {
                    let ch = chars[index];
                    index += 1;
                    if escaped {
                        escaped = false;
                    } else if ch == '\\' {
                        escaped = true;
                    } else if ch == '"' {
                        break;
                    }
                }
                let token = chars[start..index].iter().collect::<String>();
                if json_string_is_key(&chars, index) {
                    if json_string_token_is_constructor_key(&token) {
                        output.push_str(&token.bright_blue().to_string());
                    } else {
                        output.push_str(&token.green().to_string());
                    }
                } else {
                    output.push_str(&token.yellow().to_string());
                }
            }
            ch if ch.is_ascii_digit() || ch == '-' => {
                let start = index;
                index += 1;
                while index < chars.len()
                    && matches!(chars[index], '0'..='9' | '.' | 'e' | 'E' | '+' | '-')
                {
                    index += 1;
                }
                output.push_str(
                    &chars[start..index]
                        .iter()
                        .collect::<String>()
                        .magenta()
                        .to_string(),
                );
            }
            ch if ch.is_ascii_alphabetic() => {
                let start = index;
                index += 1;
                while index < chars.len() && chars[index].is_ascii_alphabetic() {
                    index += 1;
                }
                output.push_str(
                    &chars[start..index]
                        .iter()
                        .collect::<String>()
                        .magenta()
                        .to_string(),
                );
            }
            ch => {
                output.push(ch);
                index += 1;
            }
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn json_string_is_key(chars: &[char], mut index: usize) -> bool {
    while index < chars.len() && chars[index].is_whitespace() {
        index += 1;
    }
    index < chars.len() && chars[index] == ':'
}

#[requires(token.starts_with('"'))]
#[ensures(true)]
fn json_string_token_is_constructor_key(token: &str) -> bool {
    token
        .chars()
        .nth(1)
        .is_some_and(|ch| ch.is_ascii_uppercase())
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
    use jbotci_syntax::parse_syntax_tree;

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
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa run");
        assert!(error.is_empty());

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
    fn vlasei_json_outputs_compact_morphology() {
        let cli = Cli::try_parse_from(["jbotci", "vlasei", "--format", "json", "coi"])
            .expect("vlasei json");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("vlasei json run");
        assert!(error.is_empty());
        let value: serde_json::Value =
            serde_json::from_slice(&output).expect("valid uncolored JSON");

        assert_eq!(value[0]["Bare"]["kind"], "cmavo");
        assert_eq!(value[0]["Bare"]["span"], serde_json::json!([0, 3]));
        assert!(
            String::from_utf8(output)
                .expect("utf8")
                .contains("\"Bare\"")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_json_outputs_typed_syntax_tree() {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "--format", "djeisone", "mi", "klama"])
            .expect("gentufa json");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa json run");
        assert!(error.is_empty());
        let text = String::from_utf8(output).expect("utf8");
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");

        assert!(value.get("leading_nai").is_none());
        assert!(value["paragraphs"].as_array().is_some());
        assert!(text.contains("\"Predicate\""));
        assert!(!text.contains("\"constructor\""));
        assert!(!text.contains("\"kind\": \"node\""));
        assert!(!text.contains("\"leadingNai\""));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_warnings_go_to_stderr() {
        std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let cli = Cli::try_parse_from([
                    "jbotci", "gentufa", "--format", "djeisone", "mi", "klama", "fi'oi", "broda",
                ])
                .expect("gentufa warning parse");
                let mut output = Vec::new();
                let mut error = Vec::new();
                run_cli(cli, &mut output, &mut error, false).expect("gentufa warning run");

                let stdout = String::from_utf8(output).expect("stdout utf8");
                let stderr = String::from_utf8(error).expect("stderr utf8");
                assert!(stdout.starts_with('{'));
                assert!(!stdout.contains("warning:"));
                assert!(stderr.contains("warning: experimental syntax"));
                assert!(stderr.contains("FIhOI bridi/subsentence adverbial term"));
            })
            .expect("spawn warning test")
            .join()
            .expect("warning test thread");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_raw_output_is_debug_syntax_parse() {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "--turtau", "raw", "mi", "klama"])
            .expect("gentufa raw");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");
        assert!(output.contains("TextSyntax"));
        assert!(output.contains("PredicateSyntax"));
        assert!(!output.contains("SyntaxValue"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_definitions_report_not_implemented() {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "--defs", "mi", "klama"])
            .expect("gentufa defs");
        let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("defs not implemented");
        assert!(error.to_string().contains("definition rendering"));

        let cli = Cli::try_parse_from([
            "jbotci", "gentufa", "--turtau", "raw", "--skicu", "mi", "klama",
        ])
        .expect("gentufa raw defs");
        let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("raw defs not implemented");
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
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa color run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");
        assert!(output.contains("\x1b["));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn json_colorizer_distinguishes_keys_from_string_values() {
        let output = colorize_json(r#"{"key":"value","Predicate":{}}"#, true);
        assert!(output.contains("\x1b[32m\"key\"\x1b[39m"));
        assert!(output.contains("\x1b[33m\"value\"\x1b[39m"));
        assert!(output.contains("\x1b[94m\"Predicate\"\x1b[39m"));
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
