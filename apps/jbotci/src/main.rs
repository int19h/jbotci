use bityzba::{invariant, new, requires};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use jbotci_diagnostics::Diagnostic;
use jbotci_dialect::{DialectDefinition, parse_dialect_definition};
use jbotci_morphology::{
    MorphologyOptions, segment_words_with_modifiers_with_options_and_source_id,
};
use jbotci_output::{
    BracketRenderOptions, DiagnosticRenderOptions, GlideMark, JsonRenderOptions,
    PhonemeRenderOptions, StressMark, TreeRenderOptions,
    compact_morphology_json_string_with_options, compact_syntax_json_string_with_options,
    pretty_brackets_with_options, pretty_morphology_brackets_with_options,
    pretty_morphology_tree_with_options, pretty_tree_with_options, render_diagnostics,
};
use jbotci_source::SourceId;
use jbotci_syntax::{ParseOptions, parse_syntax_tree_with_source_and_options};
use owo_colors::OwoColorize;

const SYNTAX_WORKER_STACK_SIZE: usize = 128 * 1024 * 1024;

#[derive(Debug, Parser)]
#[command(name = "jbotci")]
#[command(about = "Command-line Lojban toolkit")]
#[invariant(true)]
struct Cli {
    #[arg(
        long = "color",
        global = true,
        value_name = "WHEN",
        value_enum,
        num_args = 0..=1,
        default_value_t = concolor_clap::ColorChoice::Auto,
        default_missing_value = "always",
        require_equals = true,
    )]
    color: concolor_clap::ColorChoice,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
#[invariant(true)]
#[invariant(::Vlasei(..) => true)]
#[invariant(::Gentufa(..) => true)]
#[invariant(::Mulgau(..) => true)]
#[invariant(::Tersmu(..) => true)]
#[invariant(::Vlacku(..) => true)]
#[invariant(::Jvozba(..) => true)]
#[invariant(::Cukta(..) => true)]
#[invariant(::Zbasu(..) => true)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum CliStatus {
    Success,
    Failure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct CliColorPolicy {
    stdout: bool,
    stderr: bool,
}

impl CliColorPolicy {
    #[requires(true)]
    #[ensures(!ret.stdout)]
    #[ensures(!ret.stderr)]
    fn never() -> Self {
        Self {
            stdout: false,
            stderr: false,
        }
    }

    #[requires(true)]
    #[ensures(ret.stdout == enabled)]
    #[ensures(ret.stderr == enabled)]
    fn same(enabled: bool) -> Self {
        Self {
            stdout: enabled,
            stderr: enabled,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn with_choice(self, choice: concolor_clap::ColorChoice) -> Self {
        match choice {
            concolor_clap::ColorChoice::Auto => self,
            concolor_clap::ColorChoice::Always => Self::same(true),
            concolor_clap::ColorChoice::Never => Self::never(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum GentufaFormat {
    Brackets,
    #[value(alias = "vipcihe", help = "alias: vipcihe")]
    Tree,
    Raw,
    #[value(alias = "djeisone")]
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum VlaseiFormat {
    Brackets,
    Tree,
    Raw,
    #[value(alias = "djeisone")]
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliStressMark {
    None,
    Acute,
    Caps,
}

impl From<CliStressMark> for StressMark {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: CliStressMark) -> Self {
        match value {
            CliStressMark::None => Self::None,
            CliStressMark::Acute => Self::Acute,
            CliStressMark::Caps => Self::Caps,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliGlideMark {
    None,
    Breve,
}

impl From<CliGlideMark> for GlideMark {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: CliGlideMark) -> Self {
        match value {
            CliGlideMark::None => Self::None,
            CliGlideMark::Breve => Self::Breve,
        }
    }
}

#[derive(Debug, Args)]
#[invariant(true)]
struct VlaseiInput {
    #[arg(long = "file", alias = "sfaile")]
    file: Option<PathBuf>,
    #[arg(
        long = "turtai",
        visible_alias = "format",
        default_value_t = VlaseiFormat::Brackets,
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
    #[arg(long = "mark-stress", value_enum)]
    mark_stress: Option<CliStressMark>,
    #[arg(long = "mark-glides", value_enum)]
    mark_glides: Option<CliGlideMark>,
    #[arg(long = "show-spans")]
    show_spans: bool,
    #[arg(long = "decompose-lujvo")]
    decompose_lujvo: bool,
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
    #[arg(long = "mark-stress", value_enum)]
    mark_stress: Option<CliStressMark>,
    #[arg(long = "mark-glides", value_enum)]
    mark_glides: Option<CliGlideMark>,
    #[arg(long = "show-spans")]
    show_spans: bool,
    #[arg(long = "decompose-lujvo")]
    decompose_lujvo: bool,
    #[arg()]
    text: Vec<String>,
}

#[invariant(stdout.is_empty() || stdout.ends_with('\n'))]
#[invariant(stderr.is_empty() || stderr.ends_with('\n'))]
struct GentufaRendered {
    status: CliStatus,
    stdout: String,
    stderr: String,
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
        Ok(CliStatus::Success) => ExitCode::SUCCESS,
        Ok(CliStatus::Failure) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("jbotci: {error}");
            ExitCode::FAILURE
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn run() -> Result<CliStatus> {
    let cli = Cli::parse();
    let color_policy = CliColorPolicy {
        stdout: stream_supports_ansi_color(concolor::Stream::Stdout),
        stderr: stream_supports_ansi_color(concolor::Stream::Stderr),
    };
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    run_cli_with_color_policy(cli, &mut stdout, &mut stderr, color_policy)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_cli<WOut: Write, WErr: Write>(
    cli: Cli,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_enabled: bool,
) -> Result<CliStatus> {
    run_cli_with_color_policy(cli, stdout, stderr, CliColorPolicy::same(color_enabled))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_cli_with_color_policy<WOut: Write, WErr: Write>(
    cli: Cli,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
) -> Result<CliStatus> {
    let color_policy = color_policy.with_choice(cli.color);
    match cli.command {
        Command::Vlasei(input) => {
            validate_vlasei_options(&input)?;
            let source_label = input_source_label(input.file.as_ref(), input.text.is_empty());
            let text = input.read_text()?;
            let dialect = input.dialect_definition()?;
            let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
            let words = match segment_words_with_modifiers_with_options_and_source_id(
                &text,
                &morphology_options,
                Some(SourceId(source_label.clone())),
            ) {
                Ok(words) => words,
                Err(error) => {
                    let diagnostic =
                        error.to_diagnostic(Some(SourceId(source_label.clone())), &text);
                    write_source_diagnostics(
                        stderr,
                        &source_label,
                        &text,
                        std::slice::from_ref(&diagnostic),
                        color_policy.stderr,
                    )?;
                    return Ok(CliStatus::Failure);
                }
            };
            let phoneme_options = phoneme_render_options(input.mark_stress, input.mark_glides);
            match input.format {
                VlaseiFormat::Json => {
                    let rendered = compact_morphology_json_string_with_options(
                        &words,
                        JsonRenderOptions {
                            indent: input.indent.unwrap_or(2),
                            phonemes: phoneme_options,
                        },
                    )?;
                    writeln!(stdout, "{}", colorize_json(&rendered, color_policy.stdout))?;
                }
                VlaseiFormat::Brackets => {
                    let rendered = pretty_morphology_brackets_with_options(
                        &words,
                        &text,
                        BracketRenderOptions {
                            color: color_policy.stdout,
                            phonemes: phoneme_options,
                            decompose_lujvo: input.decompose_lujvo,
                        },
                    )?;
                    writeln!(stdout, "{rendered}")?;
                }
                VlaseiFormat::Tree => {
                    let rendered = pretty_morphology_tree_with_options(
                        &words,
                        &text,
                        TreeRenderOptions {
                            color: color_policy.stdout,
                            indent: input.indent.unwrap_or(2),
                            phonemes: phoneme_options,
                            show_spans: input.show_spans,
                            decompose_lujvo: input.decompose_lujvo,
                        },
                    )?;
                    writeln!(stdout, "{rendered}")?;
                }
                VlaseiFormat::Raw => write_debug_output(stdout, &words, input.indent)?,
            }
            Ok(CliStatus::Success)
        }
        Command::Gentufa(input) => run_gentufa(input, stdout, stderr, color_policy),
        Command::Mulgau(input) => {
            let _ = input.read_text()?;
            command_not_implemented("mulgau")?;
            Ok(CliStatus::Success)
        }
        Command::Tersmu(input) => {
            let _ = input.read_text()?;
            command_not_implemented("tersmu")?;
            Ok(CliStatus::Success)
        }
        Command::Vlacku(_input) => {
            command_not_implemented("vlacku")?;
            Ok(CliStatus::Success)
        }
        Command::Jvozba(_input) => {
            command_not_implemented("jvozba")?;
            Ok(CliStatus::Success)
        }
        Command::Cukta(_input) => {
            command_not_implemented("cukta")?;
            Ok(CliStatus::Success)
        }
        Command::Zbasu(input) => {
            let _ = input.read_text()?;
            command_not_implemented("zbasu")?;
            Ok(CliStatus::Success)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_gentufa<WOut: Write, WErr: Write>(
    input: GentufaInput,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
) -> Result<CliStatus> {
    let rendered = render_gentufa_on_large_stack(input, color_policy)?;
    stderr.write_all(rendered.stderr.as_bytes())?;
    stdout.write_all(rendered.stdout.as_bytes())?;
    Ok(rendered.status)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn render_gentufa_on_large_stack(
    input: GentufaInput,
    color_policy: CliColorPolicy,
) -> Result<GentufaRendered> {
    let worker = std::thread::Builder::new()
        .name("jbotci-gentufa".to_owned())
        .stack_size(SYNTAX_WORKER_STACK_SIZE)
        .spawn(move || render_gentufa(input, color_policy))
        .context("failed to spawn gentufa syntax worker")?;
    match worker.join() {
        Ok(result) => result,
        Err(_) => bail!("gentufa syntax worker panicked"),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn render_gentufa(input: GentufaInput, color_policy: CliColorPolicy) -> Result<GentufaRendered> {
    validate_gentufa_options(&input)?;
    let source_label = input_source_label(input.file.as_ref(), input.text.is_empty());
    let text = input.read_text()?;
    let dialect = input.dialect_definition()?;
    let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let words = match segment_words_with_modifiers_with_options_and_source_id(
        &text,
        &morphology_options,
        Some(SourceId(source_label.clone())),
    ) {
        Ok(words) => words,
        Err(error) => {
            let diagnostic = error.to_diagnostic(Some(SourceId(source_label.clone())), &text);
            let stderr = render_source_diagnostics(
                &source_label,
                &text,
                std::slice::from_ref(&diagnostic),
                color_policy.stderr,
            )?;
            return Ok(new!(GentufaRendered {
                status: CliStatus::Failure,
                stdout: String::new(),
                stderr,
            }));
        }
    };
    let parse_options = ParseOptions::default().with_dialect_definition(&dialect);
    let parsed = match parse_syntax_tree_with_source_and_options(&words, &text, &parse_options) {
        Ok(parsed) => parsed,
        Err(error) => {
            let diagnostic = error.to_diagnostic(Some(SourceId(source_label.clone())), &text);
            let stderr = render_source_diagnostics(
                &source_label,
                &text,
                std::slice::from_ref(&diagnostic),
                color_policy.stderr,
            )?;
            return Ok(new!(GentufaRendered {
                status: CliStatus::Failure,
                stdout: String::new(),
                stderr,
            }));
        }
    };
    let diagnostics = parsed
        .warnings
        .iter()
        .map(|warning| warning.to_diagnostic(Some(SourceId(source_label.clone())), &text))
        .collect::<Vec<_>>();
    let stderr =
        render_source_diagnostics(&source_label, &text, &diagnostics, color_policy.stderr)?;
    let phoneme_options = phoneme_render_options(input.mark_stress, input.mark_glides);
    let mut stdout = String::new();
    match input.format {
        GentufaFormat::Brackets => {
            let rendered = pretty_brackets_with_options(
                &parsed.parse_tree,
                &text,
                BracketRenderOptions {
                    color: color_policy.stdout,
                    phonemes: phoneme_options,
                    decompose_lujvo: input.decompose_lujvo,
                },
            )?;
            stdout.push_str(&rendered);
            stdout.push('\n');
        }
        GentufaFormat::Raw => {
            stdout.push_str(&debug_output_string(&parsed.parse_tree, input.indent));
        }
        GentufaFormat::Tree => {
            let rendered = pretty_tree_with_options(
                &parsed.parse_tree,
                &text,
                TreeRenderOptions {
                    color: color_policy.stdout,
                    indent: input.indent.unwrap_or(2),
                    phonemes: phoneme_options,
                    show_spans: input.show_spans,
                    decompose_lujvo: input.decompose_lujvo,
                },
            )?;
            stdout.push_str(&rendered);
            stdout.push('\n');
        }
        GentufaFormat::Json => {
            let rendered = compact_syntax_json_string_with_options(
                &parsed.parse_tree,
                JsonRenderOptions {
                    indent: input.indent.unwrap_or(2),
                    phonemes: phoneme_options,
                },
            )?;
            stdout.push_str(&colorize_json(&rendered, color_policy.stdout));
            stdout.push('\n');
        }
    }
    Ok(new!(GentufaRendered {
        status: CliStatus::Success,
        stdout,
        stderr,
    }))
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

#[requires(!source_label.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn write_source_diagnostics<W: Write>(
    stderr: &mut W,
    source_label: &str,
    source: &str,
    diagnostics: &[Diagnostic],
    color_enabled: bool,
) -> Result<()> {
    let rendered = render_source_diagnostics(source_label, source, diagnostics, color_enabled)?;
    stderr.write_all(rendered.as_bytes())?;
    Ok(())
}

#[requires(!source_label.is_empty())]
#[ensures(diagnostics.is_empty() -> ret.as_ref().is_ok_and(String::is_empty))]
#[ensures(!diagnostics.is_empty() -> ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
fn render_source_diagnostics(
    source_label: &str,
    source: &str,
    diagnostics: &[Diagnostic],
    color_enabled: bool,
) -> Result<String> {
    render_diagnostics(
        source_label,
        source,
        diagnostics,
        DiagnosticRenderOptions {
            color: color_enabled,
        },
    )
    .map_err(|error| anyhow!(error))
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
#[ensures(true)]
fn phoneme_render_options(
    mark_stress: Option<CliStressMark>,
    mark_glides: Option<CliGlideMark>,
) -> PhonemeRenderOptions {
    let default = PhonemeRenderOptions::default();
    PhonemeRenderOptions {
        mark_stress: mark_stress
            .map(StressMark::from)
            .unwrap_or(default.mark_stress),
        mark_glides: mark_glides
            .map(GlideMark::from)
            .unwrap_or(default.mark_glides),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_vlasei_options(input: &VlaseiInput) -> Result<()> {
    match input.format {
        VlaseiFormat::Raw => {
            validate_raw_indent(input.indent)?;
            validate_no_phoneme_projection(input.mark_stress, input.mark_glides)?;
            validate_not_present(
                input.show_spans,
                "`--show-spans` is only supported with `--turtai tree`",
            )?;
            validate_not_present(
                input.decompose_lujvo,
                "`--decompose-lujvo` is only supported with `--turtai tree` or `--turtai brackets`",
            )?;
        }
        VlaseiFormat::Json => {
            validate_not_present(
                input.show_spans,
                "`--show-spans` is only supported with `--turtai tree`",
            )?;
            validate_not_present(
                input.decompose_lujvo,
                "`--decompose-lujvo` is only supported with `--turtai tree` or `--turtai brackets`",
            )?;
        }
        VlaseiFormat::Tree => {}
        VlaseiFormat::Brackets => {
            validate_no_indent(
                input.indent,
                "`--indent` is only supported with raw, JSON, and tree output",
            )?;
            validate_not_present(
                input.show_spans,
                "`--show-spans` is only supported with `--turtai tree`",
            )?;
        }
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
    if input.format == GentufaFormat::Raw {
        validate_raw_indent(input.indent)?;
        validate_no_phoneme_projection(input.mark_stress, input.mark_glides)?;
        validate_not_present(
            input.show_spans,
            "`--show-spans` is only supported with `--turtai tree`",
        )?;
        validate_not_present(
            input.decompose_lujvo,
            "`--decompose-lujvo` is only supported with `--turtai tree` or `--turtai brackets`",
        )?;
    } else {
        match input.format {
            GentufaFormat::Json => {
                validate_not_present(
                    input.show_spans,
                    "`--show-spans` is only supported with `--turtai tree`",
                )?;
                validate_not_present(
                    input.decompose_lujvo,
                    "`--decompose-lujvo` is only supported with `--turtai tree` or `--turtai brackets`",
                )?;
            }
            GentufaFormat::Tree => {}
            GentufaFormat::Brackets => {
                validate_no_indent(
                    input.indent,
                    "`--indent` is only supported with raw, JSON, and tree output",
                )?;
                validate_not_present(
                    input.show_spans,
                    "`--show-spans` is only supported with `--turtai tree`",
                )?;
            }
            GentufaFormat::Raw => {}
        }
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_no_indent(indent: Option<usize>, message: &str) -> Result<()> {
    if indent.is_some() {
        return Err(anyhow!(message.to_owned()));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_not_present(value: bool, message: &str) -> Result<()> {
    if value {
        return Err(anyhow!(message.to_owned()));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_no_phoneme_projection(
    mark_stress: Option<CliStressMark>,
    mark_glides: Option<CliGlideMark>,
) -> Result<()> {
    if mark_stress.is_some() || mark_glides.is_some() {
        return Err(anyhow!(
            "`--mark-stress` and `--mark-glides` are not supported with raw output"
        ));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_raw_indent(indent: Option<usize>) -> Result<()> {
    if let Some(indent) = indent
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
fn write_debug_output<W: Write, T: std::fmt::Debug>(
    stdout: &mut W,
    value: &T,
    indent: Option<usize>,
) -> Result<()> {
    if indent == Some(0) {
        writeln!(stdout, "{value:?}")?;
    } else {
        writeln!(stdout, "{value:#?}")?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.ends_with('\n'))]
fn debug_output_string<T: std::fmt::Debug>(value: &T, indent: Option<usize>) -> String {
    if indent == Some(0) {
        format!("{value:?}\n")
    } else {
        format!("{value:#?}\n")
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
fn stream_supports_ansi_color(stream: concolor::Stream) -> bool {
    concolor::get(stream).ansi_color()
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
            '{' | '}' | '[' | ']' | '(' | ')' | '@' | ':' | ',' => {
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

        let Command::Gentufa(dialect_input) = Cli::try_parse_from([
            "jbotci",
            "gentufa",
            "--dialect",
            "(zantufa-connectives)",
            "coi",
        ])
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
                .contains(&DialectFeature::ZantufaConnectives)
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
        assert_eq!(default_input.format, VlaseiFormat::Brackets);

        let Command::Vlasei(json_input) =
            Cli::try_parse_from(["jbotci", "vlasei", "--turtai", "json", "coi"])
                .expect("vlasei json")
                .command
        else {
            panic!("expected vlasei command")
        };
        assert_eq!(json_input.format, VlaseiFormat::Json);

        let Command::Vlasei(raw_input) =
            Cli::try_parse_from(["jbotci", "vlasei", "--format", "raw", "coi"])
                .expect("vlasei raw")
                .command
        else {
            panic!("expected vlasei command")
        };
        assert_eq!(raw_input.format, VlaseiFormat::Raw);

        let Command::Vlasei(alias_input) =
            Cli::try_parse_from(["jbotci", "vlasei", "--format", "djeisone", "coi"])
                .expect("vlasei format alias")
                .command
        else {
            panic!("expected vlasei command")
        };
        assert_eq!(alias_input.format, VlaseiFormat::Json);

        let Command::Vlasei(brackets_input) =
            Cli::try_parse_from(["jbotci", "vlasei", "--format", "brackets", "coi"])
                .expect("vlasei brackets")
                .command
        else {
            panic!("expected vlasei command")
        };
        assert_eq!(brackets_input.format, VlaseiFormat::Brackets);

        let Command::Vlasei(tree_input) =
            Cli::try_parse_from(["jbotci", "vlasei", "--format", "tree", "coi"])
                .expect("vlasei tree")
                .command
        else {
            panic!("expected vlasei command")
        };
        assert_eq!(tree_input.format, VlaseiFormat::Tree);

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
    fn parses_color_policy_values() {
        let default_cli = Cli::try_parse_from(["jbotci", "gentufa", "coi"]).expect("default color");
        assert_eq!(default_cli.color, concolor_clap::ColorChoice::Auto);

        let bare_cli =
            Cli::try_parse_from(["jbotci", "gentufa", "--color", "coi"]).expect("bare color");
        assert_eq!(bare_cli.color, concolor_clap::ColorChoice::Always);

        let always_cli = Cli::try_parse_from(["jbotci", "gentufa", "--color=always", "coi"])
            .expect("always color");
        assert_eq!(always_cli.color, concolor_clap::ColorChoice::Always);

        let never_cli = Cli::try_parse_from(["jbotci", "gentufa", "--color=never", "coi"])
            .expect("never color");
        assert_eq!(never_cli.color, concolor_clap::ColorChoice::Never);
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
        assert!(!help.contains("plain"));
        assert!(help.contains("brackets"));
        assert!(help.contains("tree"));
        assert!(help.contains("raw"));
        assert!(help.contains("json"));
        assert!(!help.contains("--turtau"));
        assert!(!help.contains("--termoha"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_default_output_matches_bracket_renderer() {
        run_on_large_stack(|| {
            let cli =
                Cli::try_parse_from(["jbotci", "gentufa", "mi", "klama"]).expect("gentufa default");
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
                BracketRenderOptions {
                    color: false,
                    ..BracketRenderOptions::default()
                },
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

        assert_eq!(value[0]["Bare"]["Cmavo"]["phonemes"], "coĭ");
        assert_eq!(value[0]["Bare"]["Cmavo"]["span"], serde_json::json!([0, 3]));
        assert!(
            String::from_utf8(output)
                .expect("utf8")
                .contains("\"Bare\"")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_raw_output_is_debug_morphology() {
        let cli = Cli::try_parse_from(["jbotci", "vlasei", "--format", "raw", "coi"])
            .expect("vlasei raw");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("vlasei raw run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");

        assert!(output.starts_with("[\n"));
        assert!(output.contains("Bare("));
        assert!(output.contains("Cmavo"));
        assert!(output.contains("Phonemes"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_raw_indent_zero_uses_compact_debug() {
        let cli = Cli::try_parse_from([
            "jbotci", "vlasei", "--format", "raw", "--indent", "0", "coi",
        ])
        .expect("vlasei raw indent zero");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("vlasei raw run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");

        assert!(!output.trim_end().contains('\n'));
        assert!(output.starts_with("[Bare("));
        assert!(output.contains("Bare("));
        assert!(output.contains("Cmavo"));
        assert!(output.contains("Phonemes"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_raw_rejects_nonzero_indent() {
        let cli = Cli::try_parse_from([
            "jbotci", "vlasei", "--format", "raw", "--indent", "2", "coi",
        ])
        .expect("vlasei raw indent parses");
        let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("raw nonzero indent rejected");
        assert!(error.to_string().contains("only supports `0`"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_projection_flags_affect_non_raw_output() {
        let cli = Cli::try_parse_from([
            "jbotci",
            "vlasei",
            "--format",
            "tree",
            "--mark-stress",
            "none",
            "--mark-glides",
            "none",
            "coi",
            "klama",
        ])
        .expect("vlasei projection flags parse");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("vlasei tree run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");
        assert!(output.contains("Cmavo \"coi\""));
        assert!(output.contains("Gismu \"klama\""));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_morphology_errors_go_to_stderr() {
        let cli = Cli::try_parse_from(["jbotci", "vlasei", "aa"]).expect("vlasei parses");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("vlasei run");

        assert_eq!(status, CliStatus::Failure);
        assert!(output.is_empty());
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("[morphology.invalid] Error"));
        assert!(stderr.contains("aa"));
        assert!(!stderr.contains("jbotci:"));
        assert!(!stderr.contains("\x1b["));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn raw_rejects_projection_flags() {
        let cli = Cli::try_parse_from([
            "jbotci",
            "gentufa",
            "--format",
            "raw",
            "--mark-stress",
            "none",
            "mi",
            "klama",
        ])
        .expect("gentufa raw projection flag parses");
        let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("raw projection flags rejected");
        assert!(error.to_string().contains("not supported with raw output"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tree_show_spans_and_lujvo_decomposition() {
        let cli = Cli::try_parse_from([
            "jbotci",
            "vlasei",
            "--format",
            "tree",
            "--show-spans",
            "--decompose-lujvo",
            "mivyselbai",
        ])
        .expect("vlasei tree span flags parse");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("vlasei tree run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");
        assert!(output.contains("Lujvo @[0‥10)"));
        assert!(output.contains("miv·y·sél·baĭ"));
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
            assert!(output.contains("\n  leading_terms: [\n    Cmavo \"mi\""));
            assert!(output.contains("leading_terms: ["));
            assert!(output.contains("Gismu \"kláma\""));
            assert!(!output.contains("Text {"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_tree_preserves_source_order_for_connected_relation() {
        run_on_large_stack(|| {
            let cli = Cli::try_parse_from([
                "jbotci", "gentufa", "--format", "tree", "gleki", "je", "klama",
            ])
            .expect("gentufa tree");
            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, false).expect("gentufa tree run");
            assert!(error.is_empty());
            let output = String::from_utf8(output).expect("utf8");

            let leading = output.find("leading_relation").expect("leading relation");
            let connective = output.find("connective").expect("connective");
            let trailing = output.find("trailing_relation").expect("trailing relation");
            assert!(leading < connective);
            assert!(connective < trailing);
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_tree_preserves_source_order_for_binary_math() {
        run_on_large_stack(|| {
            let cli = Cli::try_parse_from([
                "jbotci", "gentufa", "--format", "tree", "li", "pa", "su'i", "re",
            ])
            .expect("gentufa tree");
            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, false).expect("gentufa tree run");
            assert!(error.is_empty());
            let output = String::from_utf8(output).expect("utf8");

            let left = output.find("left_expression").expect("left expression");
            let operator = output.find("operator").expect("operator");
            let right = output.find("right_expression").expect("right expression");
            assert!(left < operator);
            assert!(operator < right);
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
                r#"Predicate{leading_terms:[Cmavo "mi"],Gismu "kláma"}"#
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
                assert!(stderr.contains("Warning: experimental syntax"));
                assert!(stderr.contains("syntax.warning.experimental-fihoi-adverbial"));
                assert!(stderr.contains("FIhOI bridi/subsentence adverbial term"));
                assert!(stderr.contains("fi'oi"));
            })
            .expect("spawn warning test")
            .join()
            .expect("warning test thread");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_syntax_errors_go_to_stderr() {
        run_on_large_stack(|| {
            let cli =
                Cli::try_parse_from(["jbotci", "gentufa", "gleki", "ku", "klama", "zei", "klama"])
                    .expect("gentufa parses");
            let mut output = Vec::new();
            let mut error = Vec::new();
            let status = run_cli(cli, &mut output, &mut error, false).expect("gentufa run");

            assert_eq!(status, CliStatus::Failure);
            assert!(output.is_empty());
            let stderr = String::from_utf8(error).expect("stderr utf8");
            assert!(stderr.contains("[syntax.parse] Error"));
            assert!(stderr.contains("syntax parse failed"));
            assert!(stderr.contains("expected cmavo"));
            assert!(stderr.contains("ku"));
            assert!(!stderr.contains("jbotci:"));
            assert!(!stderr.contains("\x1b["));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn warning_context_includes_verbatim_quote_text() {
        run_on_large_stack(|| {
            let cli = Cli::try_parse_from(["jbotci", "gentufa", "zo'oi", "gleki"])
                .expect("zo'oi warning parse");
            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, false).expect("gentufa warning run");

            let stderr = String::from_utf8(error).expect("stderr utf8");
            assert!(stderr.contains("ZOhOI single-word foreign quote"));
            assert!(stderr.contains("zo'oi gleki"));
            assert!(stderr.contains("syntax.warning.experimental-zoh-oi-quote"));
            assert!(!stderr.contains("<5 chars>"));
        });
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
            assert!(output.starts_with("TextSyntax"));
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
    fn gentufa_color_flag_forces_ansi_bracket_output() {
        run_on_large_stack(|| {
            let cli = Cli::try_parse_from(["jbotci", "gentufa", "--color", "mi", "klama"])
                .expect("gentufa color");
            assert_eq!(cli.color, concolor_clap::ColorChoice::Always);
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
            assert!(output.contains("\x1b[94mCmavo\x1b[39m"));
            assert!(output.contains("\x1b[33m\"mi\"\x1b[39m"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_runs_reported_color_case_on_normal_cli_stack() {
        let cli = Cli::try_parse_from([
            "jbotci", "gentufa", "--color", "gleki", "je", "klama", "zei", "klama",
        ])
        .expect("gentufa color");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("gentufa color run");
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");
        assert!(output.contains("\x1b["));
        assert!(output.contains("gléki"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn color_never_disables_ansi_output() {
        let cli = Cli::try_parse_from(["jbotci", "gentufa", "--color=never", "mi", "klama"])
            .expect("gentufa color never");
        assert_eq!(cli.color, concolor_clap::ColorChoice::Never);

        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, true).expect("gentufa color never run");

        let output = String::from_utf8(output).expect("utf8");
        assert!(!output.contains("\x1b["));
        assert!(error.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn json_colorizer_distinguishes_keys_from_string_values() {
        let output = colorize_json(r#"{"key":"value","Predicate":{}}"#, true);
        assert!(output.contains("\x1b[32m\"key\"\x1b[39m"));
        assert!(output.contains("\x1b[33m\"value\"\x1b[39m"));
        assert!(output.contains("\x1b[94m\"Predicate\"\x1b[39m"));
        assert!(output.contains("\x1b[90m{\x1b[39m"));
        assert!(output.contains("\x1b[90m}\x1b[39m"));
        assert!(!output.contains("\x1b[36m"));
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
