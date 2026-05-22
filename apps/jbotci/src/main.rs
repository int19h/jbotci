use bityzba::{invariant, requires};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, anyhow};
use clap::{Args, Parser, Subcommand, ValueEnum};
use jbotci_dialect::{DialectDefinition, parse_dialect_definition};
use jbotci_morphology::{MorphologyOptions, segment_words_with_modifiers_with_options};
use jbotci_output::{
    BracketRenderOptions, JsonRenderOptions, TreeRenderOptions, compact_json_string_with_options,
    pretty_brackets_with_options, pretty_tree_with_options,
};
use jbotci_syntax::{
    ParseOptions, SyntaxParse, parse_syntax_tree_with_source_and_options, syntax_warning_displays,
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
    Vlasei(VlaseiInput),
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
    Brackets,
    #[value(alias = "vipcihe", help = "alias: vipcihe")]
    Tree,
    Raw,
    #[value(alias = "djeisone")]
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[invariant(true)]
enum VlaseiFormat {
    Plain,
    #[value(alias = "djeisone")]
    Json,
}

#[derive(Debug, Args)]
#[invariant(true)]
struct VlaseiInput {
    #[arg(long = "file", alias = "sfaile")]
    file: Option<PathBuf>,
    #[arg(
        long = "turtai",
        visible_alias = "format",
        default_value_t = VlaseiFormat::Plain,
        value_enum
    )]
    format: VlaseiFormat,
    #[arg(long = "trace", alias = "plivei")]
    trace: Option<String>,
    #[arg(long = "dialect")]
    dialect: Option<String>,
    #[arg(long = "no-postproc", alias = "na-velruhe")]
    no_postproc: bool,
    #[arg(long = "camxes")]
    camxes: bool,
    #[arg(long = "indent")]
    indent: Option<usize>,
    #[arg()]
    text: Vec<String>,
}

impl VlaseiInput {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn read_text(&self) -> Result<String> {
        read_text_input(self.file.as_ref(), &self.text)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn dialect_definition(&self) -> Result<DialectDefinition> {
        dialect_definition(self.dialect.as_deref())
    }
}

#[derive(Debug, Args)]
#[invariant(true)]
struct TextInput {
    #[arg(long = "file", alias = "sfaile")]
    file: Option<PathBuf>,
    #[arg(long = "trace", alias = "plivei")]
    trace: Option<String>,
    #[arg(long = "dialect")]
    dialect: Option<String>,
    #[arg(long = "no-postproc", alias = "na-velruhe")]
    no_postproc: bool,
    #[arg(long = "camxes")]
    camxes: bool,
    #[arg(long = "indent")]
    indent: Option<usize>,
    #[arg()]
    text: Vec<String>,
}

impl TextInput {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn read_text(&self) -> Result<String> {
        read_text_input(self.file.as_ref(), &self.text)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn dialect_definition(&self) -> Result<DialectDefinition> {
        dialect_definition(self.dialect.as_deref())
    }
}

#[derive(Debug, Args)]
#[invariant(true)]
struct GentufaInput {
    #[arg(long = "file", alias = "sfaile")]
    file: Option<PathBuf>,
    #[arg(
        long = "turtai",
        visible_alias = "format",
        default_value_t = GentufaFormat::Brackets,
        value_enum
    )]
    format: GentufaFormat,
    #[arg(long = "trace", alias = "plivei")]
    trace: Option<String>,
    #[arg(long = "dialect")]
    dialect: Option<String>,
    #[arg(long = "no-postproc", alias = "na-velruhe")]
    no_postproc: bool,
    #[arg(long = "camxes")]
    camxes: bool,
    #[arg(long = "skicu", visible_alias = "defs")]
    definitions: bool,
    #[arg(long = "indent")]
    indent: Option<usize>,
    #[arg()]
    text: Vec<String>,
}

impl GentufaInput {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn read_text(&self) -> Result<String> {
        read_text_input(self.file.as_ref(), &self.text)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn dialect_definition(&self) -> Result<DialectDefinition> {
        dialect_definition(self.dialect.as_deref())
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
            let dialect = input.dialect_definition()?;
            let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
            let words = segment_words_with_modifiers_with_options(&text, &morphology_options)?;
            match input.format {
                VlaseiFormat::Plain => {
                    for word in words {
                        writeln!(stdout, "{word}")?;
                    }
                }
                VlaseiFormat::Json => {
                    let rendered = compact_json_string_with_options(
                        &words,
                        JsonRenderOptions {
                            indent: input.indent.unwrap_or(2),
                        },
                    )?;
                    writeln!(stdout, "{}", colorize_json(&rendered, color_enabled))?;
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
    let dialect = input.dialect_definition()?;
    let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let words = segment_words_with_modifiers_with_options(&text, &morphology_options)?;
    let parse_options = ParseOptions::default().with_dialect_definition(&dialect);
    let parsed = parse_syntax_tree_with_source_and_options(&words, &text, &parse_options)?;
    render_syntax_warnings(&parsed, &text, &warning_source, stderr)?;
    match input.format {
        GentufaFormat::Brackets => {
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
            if input.indent == Some(0) {
                writeln!(stdout, "{:?}", parsed.parse_tree)?;
            } else {
                writeln!(stdout, "{:#?}", parsed.parse_tree)?;
            }
        }
        GentufaFormat::Tree => {
            let rendered = pretty_tree_with_options(
                &parsed.parse_tree,
                &text,
                TreeRenderOptions {
                    color: color_enabled,
                    indent: input.indent.unwrap_or(2),
                },
            )?;
            writeln!(stdout, "{rendered}")?;
        }
        GentufaFormat::Json => {
            let rendered = compact_json_string_with_options(
                &parsed.parse_tree,
                JsonRenderOptions {
                    indent: input.indent.unwrap_or(2),
                },
            )?;
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
fn dialect_definition(source: Option<&str>) -> Result<DialectDefinition> {
    source.map_or_else(
        || Ok(DialectDefinition::default()),
        |source| parse_dialect_definition(source).map_err(|error| anyhow!(error)),
    )
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn render_syntax_warnings<W: Write>(
    parsed: &SyntaxParse,
    source: &str,
    source_label: &str,
    stderr: &mut W,
) -> Result<()> {
    let mut syntax_words = Vec::new();
    parsed
        .parse_tree
        .visit_words(&mut |word| syntax_words.push(word.clone()));
    for warning in syntax_warning_displays(source_label, source, &syntax_words, &parsed.warnings) {
        writeln!(
            stderr,
            "{}:{}:{}: warning: experimental syntax: {}",
            warning.source_label, warning.line, warning.column, warning.message
        )?;
        writeln!(stderr, "  {}", warning.context)?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_gentufa_options(input: &GentufaInput) -> Result<()> {
    if input.definitions {
        return match input.format {
            GentufaFormat::Brackets => Err(anyhow!(
                "`--skicu`/`--defs` is accepted for brackets output, but dictionary definition rendering has not been ported yet"
            )),
            GentufaFormat::Raw | GentufaFormat::Tree | GentufaFormat::Json => Err(anyhow!(
                "`--skicu`/`--defs` is only meaningful with `--turtai brackets`; dictionary definition rendering has not been ported yet"
            )),
        };
    }
    if input.format == GentufaFormat::Raw
        && let Some(indent) = input.indent
        && indent != 0
    {
        return Err(anyhow!(
            "`--indent` for raw output only supports `0`, because Rust Debug formatting only supports pretty or compact output"
        ));
    }
    Ok(())
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
    use jbotci_dialect::DialectFeature;
    use jbotci_morphology::segment_words_with_modifiers;
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
        assert_eq!(default_input.format, GentufaFormat::Brackets);

        let Command::Gentufa(brackets_input) =
            Cli::try_parse_from(["jbotci", "gentufa", "--turtai", "brackets", "coi"])
                .expect("turtai brackets")
                .command
        else {
            panic!("expected gentufa command")
        };
        assert_eq!(brackets_input.format, GentufaFormat::Brackets);

        let Command::Gentufa(alias_input) =
            Cli::try_parse_from(["jbotci", "gentufa", "--format", "brackets", "coi"])
                .expect("format alias")
                .command
        else {
            panic!("expected gentufa command")
        };
        assert_eq!(alias_input.format, GentufaFormat::Brackets);

        let Command::Gentufa(raw_input) =
            Cli::try_parse_from(["jbotci", "gentufa", "--turtai", "raw", "--skicu", "coi"])
                .expect("raw with skicu parses")
                .command
        else {
            panic!("expected gentufa command")
        };
        assert_eq!(raw_input.format, GentufaFormat::Raw);
        assert!(raw_input.definitions);

        let Command::Gentufa(tree_input) =
            Cli::try_parse_from(["jbotci", "gentufa", "--turtai", "tree", "coi"])
                .expect("tree parses")
                .command
        else {
            panic!("expected gentufa command")
        };
        assert_eq!(tree_input.format, GentufaFormat::Tree);

        let Command::Gentufa(vipcihe_input) =
            Cli::try_parse_from(["jbotci", "gentufa", "--turtai", "vipcihe", "coi"])
                .expect("vipcihe parses")
                .command
        else {
            panic!("expected gentufa command")
        };
        assert_eq!(vipcihe_input.format, GentufaFormat::Tree);

        let Command::Gentufa(defs_input) =
            Cli::try_parse_from(["jbotci", "gentufa", "--defs", "coi"])
                .expect("defs alias")
                .command
        else {
            panic!("expected gentufa command")
        };
        assert!(defs_input.definitions);

        let Command::Gentufa(dialect_input) =
            Cli::try_parse_from(["jbotci", "gentufa", "--dialect", "(zantufa-cmavo)", "coi"])
                .expect("dialect flag parses")
                .command
        else {
            panic!("expected gentufa command")
        };
        assert!(
            dialect_input
                .dialect_definition()
                .expect("dialect definition")
                .features
                .contains(&DialectFeature::ZantufaCmavo)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_vlasei_formats_and_rejects_unknown_values() {
        let Command::Vlasei(default_input) = Cli::try_parse_from(["jbotci", "vlasei", "coi"])
            .expect("default vlasei")
            .command
        else {
            panic!("expected vlasei command")
        };
        assert_eq!(default_input.format, VlaseiFormat::Plain);

        let Command::Vlasei(json_input) =
            Cli::try_parse_from(["jbotci", "vlasei", "--turtai", "json", "coi"])
                .expect("vlasei json")
                .command
        else {
            panic!("expected vlasei command")
        };
        assert_eq!(json_input.format, VlaseiFormat::Json);

        let Command::Vlasei(alias_input) =
            Cli::try_parse_from(["jbotci", "vlasei", "--format", "djeisone", "coi"])
                .expect("vlasei format alias")
                .command
        else {
            panic!("expected vlasei command")
        };
        assert_eq!(alias_input.format, VlaseiFormat::Json);

        assert_eq!(
            Cli::try_parse_from(["jbotci", "vlasei", "--turtai", "xml", "coi"])
                .expect_err("unknown vlasei format")
                .kind(),
            ErrorKind::InvalidValue
        );
        assert_eq!(
            Cli::try_parse_from(["jbotci", "vlasei", "--termoha", "json", "coi"])
                .expect_err("old vlasei format option")
                .kind(),
            ErrorKind::UnknownArgument
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_unknown_gentufa_format_and_word_kind_flag() {
        assert_eq!(
            Cli::try_parse_from(["jbotci", "gentufa", "--turtai", "xml", "coi"])
                .expect_err("unknown format")
                .kind(),
            ErrorKind::InvalidValue
        );
        assert_eq!(
            Cli::try_parse_from(["jbotci", "gentufa", "--turtau", "raw", "coi"])
                .expect_err("old gentufa format option")
                .kind(),
            ErrorKind::UnknownArgument
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
    fn gentufa_help_lists_formats_and_brackets_flags() {
        let error = Cli::try_parse_from(["jbotci", "gentufa", "--help"]).expect_err("help");
        assert_eq!(error.kind(), ErrorKind::DisplayHelp);
        let help = error.to_string();
        assert!(help.contains("--turtai"));
        assert!(help.contains("--format"));
        assert!(help.contains("brackets"));
        assert!(help.contains("tree"));
        assert!(help.contains("vipcihe"));
        assert!(!help.contains("compact"));
        assert!(help.contains("raw"));
        assert!(help.contains("--skicu"));
        assert!(help.contains("--defs"));
        assert!(help.contains("--indent"));
        assert!(!help.contains("--wordKind"));
        assert!(!help.contains("--turtau"));
        assert!(!help.contains("--termoha"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_help_lists_restricted_formats() {
        let error = Cli::try_parse_from(["jbotci", "vlasei", "--help"]).expect_err("help");
        assert_eq!(error.kind(), ErrorKind::DisplayHelp);
        let help = error.to_string();
        assert!(help.contains("--turtai"));
        assert!(help.contains("--format"));
        assert!(help.contains("plain"));
        assert!(help.contains("json"));
        assert!(!help.contains("--turtau"));
        assert!(!help.contains("--termoha"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_compact_output_matches_bracket_renderer() {
        run_on_large_stack(|| {
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
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_json_outputs_compact_morphology() {
        let cli = Cli::try_parse_from(["jbotci", "vlasei", "--turtai", "json", "coi"])
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
        run_on_large_stack(|| {
            let cli =
                Cli::try_parse_from(["jbotci", "gentufa", "--format", "djeisone", "mi", "klama"])
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
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_tree_outputs_collapsed_syntax_tree() {
        run_on_large_stack(|| {
            let cli = Cli::try_parse_from(["jbotci", "gentufa", "--format", "tree", "mi", "klama"])
                .expect("gentufa tree");
            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, false).expect("gentufa tree run");
            assert!(error.is_empty());
            let output = String::from_utf8(output).expect("utf8");

            assert!(output.starts_with("Predicate {\n"));
            assert!(output.contains("\n  leading_terms: [\n    \"mi\","));
            assert!(output.contains("leading_terms: ["));
            assert!(output.contains("\"mi\""));
            assert!(output.contains("\"kláma\""));
            assert!(!output.contains("Text {"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_indent_zero_makes_tree_single_line() {
        run_on_large_stack(|| {
            let cli = Cli::try_parse_from([
                "jbotci", "gentufa", "--format", "tree", "--indent", "0", "mi", "klama",
            ])
            .expect("gentufa tree indent zero");
            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, false).expect("gentufa tree run");
            assert!(error.is_empty());
            let output = String::from_utf8(output).expect("utf8");
            assert_eq!(
                output.trim_end(),
                r#"Predicate{leading_terms:["mi"],"kláma"}"#
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_indent_zero_makes_json_single_line() {
        run_on_large_stack(|| {
            let cli = Cli::try_parse_from([
                "jbotci", "gentufa", "--format", "json", "--indent", "0", "mi", "klama",
            ])
            .expect("gentufa json indent zero");
            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, false).expect("gentufa json run");
            assert!(error.is_empty());
            let output = String::from_utf8(output).expect("utf8");
            assert!(!output.trim_end().contains('\n'));
            let _: serde_json::Value = serde_json::from_str(&output).expect("valid JSON");
        });
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
                assert!(stderr.contains("@ "));
                assert!(stderr.contains("👉"));
            })
            .expect("spawn warning test")
            .join()
            .expect("warning test thread");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_raw_output_is_debug_syntax_parse() {
        run_on_large_stack(|| {
            let cli = Cli::try_parse_from(["jbotci", "gentufa", "--turtai", "raw", "mi", "klama"])
                .expect("gentufa raw");
            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, false).expect("gentufa run");
            assert!(error.is_empty());
            let output = String::from_utf8(output).expect("utf8");
            assert!(output.contains("TextSyntax"));
            assert!(output.contains("PredicateSyntax"));
            assert!(!output.contains("SyntaxValue"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_raw_indent_zero_uses_compact_debug() {
        run_on_large_stack(|| {
            let cli = Cli::try_parse_from([
                "jbotci", "gentufa", "--turtai", "raw", "--indent", "0", "mi", "klama",
            ])
            .expect("gentufa raw indent zero");
            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, false).expect("gentufa raw run");
            assert!(error.is_empty());
            let output = String::from_utf8(output).expect("utf8");
            assert!(!output.trim_end().contains('\n'));
            assert!(output.starts_with("TextSyntax { "));
            assert!(output.contains("PredicateSyntax"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_raw_rejects_nonzero_indent() {
        let cli = Cli::try_parse_from([
            "jbotci", "gentufa", "--turtai", "raw", "--indent", "2", "mi", "klama",
        ])
        .expect("gentufa raw indent parses");
        let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("raw nonzero indent rejected");
        assert!(error.to_string().contains("only supports `0`"));
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
            "jbotci", "gentufa", "--turtai", "raw", "--skicu", "mi", "klama",
        ])
        .expect("gentufa raw defs");
        let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("raw defs not implemented");
        assert!(error.to_string().contains("only meaningful"));

        let cli = Cli::try_parse_from([
            "jbotci", "gentufa", "--turtai", "tree", "--skicu", "mi", "klama",
        ])
        .expect("gentufa tree defs");
        let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("tree defs not implemented");
        assert!(error.to_string().contains("only meaningful"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_color_flag_forces_ansi_compact_output() {
        run_on_large_stack(|| {
            let cli = Cli::try_parse_from(["jbotci", "gentufa", "--color", "mi", "klama"])
                .expect("gentufa color");
            assert!(cli.color);
            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, false).expect("gentufa color run");
            assert!(error.is_empty());
            let output = String::from_utf8(output).expect("utf8");
            assert!(output.contains("\x1b["));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_color_flag_forces_ansi_tree_output() {
        run_on_large_stack(|| {
            let cli = Cli::try_parse_from([
                "jbotci", "gentufa", "--color", "--format", "vipcihe", "mi", "klama",
            ])
            .expect("gentufa tree color");
            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, false).expect("gentufa tree color run");
            assert!(error.is_empty());
            let output = String::from_utf8(output).expect("utf8");
            assert!(output.contains("\x1b[94mPredicate\x1b[39m"));
            assert!(output.contains("\x1b[33m\"mi\"\x1b[39m"));
        });
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
            trace: None,
            dialect: None,
            no_postproc: false,
            camxes: false,
            indent: None,
            text: vec!["coi".into(), "rodo".into()],
        };
        assert_eq!(input.read_text().expect("text"), "coi rodo");
    }

    #[requires(true)]
    #[ensures(true)]
    fn run_on_large_stack(test: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(test)
            .expect("spawn large-stack test")
            .join()
            .expect("large-stack test thread");
    }
}
