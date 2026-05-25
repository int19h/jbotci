use bityzba::{invariant, new, requires};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use jbotci_diagnostics::{
    DEFAULT_TRACE_LIMIT, Diagnostic, TraceFilter, TraceLevel, TraceOptions, TracePhase, TraceReport,
};
use jbotci_dialect::{DialectDefinition, parse_dialect_definition};
use jbotci_morphology::{
    MORPHOLOGY_TRACE_FILTERS, MorphologyOptions,
    segment_words_with_modifiers_with_options_and_source_id_attempt,
};
use jbotci_output::{
    BracketRenderOptions, DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH, DiagnosticDetailMode,
    DiagnosticRenderOptions, GlideMark, JsonRenderOptions, PhonemeRenderOptions, StressMark,
    TraceRenderOptions, TreeRenderOptions, compact_morphology_json_string_with_options,
    compact_syntax_json_string_with_options, pretty_brackets_with_options,
    pretty_morphology_brackets_with_options, pretty_morphology_tree_with_options,
    pretty_tree_with_options, render_diagnostics, render_trace_report,
};
use jbotci_source::SourceId;
use jbotci_syntax::{
    ParseOptions, SYNTAX_TRACE_FILTERS, parse_syntax_tree_with_source_and_options_attempt,
};
#[cfg(feature = "grammar-debug")]
use jbotci_syntax::{syntax_grammar_ebnf, syntax_grammar_svg};
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
    #[arg(long = "detailed-errors", global = true)]
    detailed_errors: bool,
    #[arg(long = "trace-phase", global = true, value_enum)]
    trace_phase: Option<CliTracePhase>,
    #[arg(long = "trace-limit", global = true)]
    trace_limit: Option<usize>,
    #[arg(long = "trace-list", global = true)]
    trace_list: bool,
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
#[invariant(::Gerna(..) => true)]
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
    #[cfg(feature = "grammar-debug")]
    #[command(name = "gerna", visible_alias = "grammar")]
    Gerna(GernaInput),
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

#[cfg(feature = "grammar-debug")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum GernaFormat {
    Ebnf,
    Svg,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliTracePhase {
    Morphology,
    Syntax,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct CliParsedTraceSpec {
    level: TraceLevel,
    filter: Option<TraceFilter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct CliTraceConfig {
    phase: TracePhase,
    limit: usize,
}

#[invariant(!self.command_name.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CliTraceValidation {
    command_name: &'static str,
    trace_phase: Option<TracePhase>,
    trace_limit_present: bool,
    trace_list: bool,
    supports_morphology: bool,
    supports_syntax: bool,
}

impl From<CliTracePhase> for TracePhase {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: CliTracePhase) -> Self {
        match value {
            CliTracePhase::Morphology => Self::Morphology,
            CliTracePhase::Syntax => Self::Syntax,
            CliTracePhase::All => Self::All,
        }
    }
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
    #[arg(
        long = "trace",
        alias = "plivei",
        value_name = "SPEC",
        num_args = 0..=1,
        default_missing_value = "1"
    )]
    trace: Option<Option<String>>,
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
    #[arg(
        long = "trace",
        alias = "plivei",
        value_name = "SPEC",
        num_args = 0..=1,
        default_missing_value = "1"
    )]
    trace: Option<Option<String>>,
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
    #[arg(
        long = "trace",
        alias = "plivei",
        value_name = "SPEC",
        num_args = 0..=1,
        default_missing_value = "1"
    )]
    trace: Option<Option<String>>,
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

#[cfg(feature = "grammar-debug")]
#[derive(Debug, Args)]
#[invariant(true)]
struct GernaInput {
    #[arg(
        long = "turtai",
        visible_alias = "format",
        default_value_t = GernaFormat::Ebnf,
        value_enum
    )]
    format: GernaFormat,
    #[arg(short = 'o', long = "output-file")]
    output_file: Option<PathBuf>,
    #[arg(long = "dialect")]
    dialect: Option<String>,
}

#[cfg(feature = "grammar-debug")]
impl GernaInput {
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
    let diagnostic_terminal_width = stderr_terminal_width();
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    run_cli_with_color_policy_and_width(
        cli,
        &mut stdout,
        &mut stderr,
        color_policy,
        diagnostic_terminal_width,
    )
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
    run_cli_with_color_policy_and_width(
        cli,
        stdout,
        stderr,
        color_policy,
        DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
    )
}

#[requires(diagnostic_terminal_width > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_cli_with_color_policy_and_width<WOut: Write, WErr: Write>(
    cli: Cli,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_terminal_width: usize,
) -> Result<CliStatus> {
    let color_policy = color_policy.with_choice(cli.color);
    let diagnostic_detail = if cli.detailed_errors {
        DiagnosticDetailMode::Detailed
    } else {
        DiagnosticDetailMode::Summary
    };
    let trace_limit = cli.trace_limit.unwrap_or(DEFAULT_TRACE_LIMIT);
    let trace_limit_present = cli.trace_limit.is_some();
    if trace_limit == 0 {
        bail!("--trace-limit must be greater than 0");
    }
    let requested_trace_phase = cli.trace_phase.map(TracePhase::from);
    match cli.command {
        Command::Vlasei(mut input) => {
            normalize_trace_text_input(&mut input.trace, &input.file, &mut input.text);
            validate_vlasei_options(&input)?;
            validate_trace_controls(
                &input.trace,
                new!(CliTraceValidation {
                    command_name: "vlasei",
                    trace_phase: requested_trace_phase,
                    trace_limit_present,
                    trace_list: cli.trace_list,
                    supports_morphology: true,
                    supports_syntax: false,
                }),
            )?;
            if cli.trace_list {
                write_trace_filter_list(
                    stdout,
                    requested_trace_phase.unwrap_or(TracePhase::Morphology),
                    true,
                    false,
                )?;
                return Ok(CliStatus::Success);
            }
            let morphology_trace_options = trace_options(
                &input.trace,
                requested_trace_phase.unwrap_or(TracePhase::Morphology),
                trace_limit,
            )?;
            let source_label = input_source_label(input.file.as_ref(), input.text.is_empty());
            let text = input.read_text()?;
            let dialect = input.dialect_definition()?;
            let morphology_options = MorphologyOptions::default()
                .with_dialect_definition(&dialect)
                .with_trace_options(morphology_trace_options);
            let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
                &text,
                &morphology_options,
                Some(SourceId(source_label.clone())),
            );
            let trace_stderr = render_cli_trace(
                attempt.trace.as_ref(),
                color_policy.stderr,
                diagnostic_terminal_width,
            );
            let words = match attempt.result {
                Ok(words) => words,
                Err(error) => {
                    stderr.write_all(trace_stderr.as_bytes())?;
                    let diagnostic =
                        error.to_diagnostic(Some(SourceId(source_label.clone())), &text);
                    write_source_diagnostics(
                        stderr,
                        &source_label,
                        &text,
                        std::slice::from_ref(&diagnostic),
                        color_policy.stderr,
                        diagnostic_detail,
                        diagnostic_terminal_width,
                    )?;
                    return Ok(CliStatus::Failure);
                }
            };
            stderr.write_all(trace_stderr.as_bytes())?;
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
        Command::Gentufa(mut input) => {
            normalize_trace_text_input(&mut input.trace, &input.file, &mut input.text);
            validate_trace_controls(
                &input.trace,
                new!(CliTraceValidation {
                    command_name: "gentufa",
                    trace_phase: requested_trace_phase,
                    trace_limit_present,
                    trace_list: cli.trace_list,
                    supports_morphology: true,
                    supports_syntax: true,
                }),
            )?;
            if cli.trace_list {
                write_trace_filter_list(
                    stdout,
                    requested_trace_phase.unwrap_or(TracePhase::Syntax),
                    true,
                    true,
                )?;
                return Ok(CliStatus::Success);
            }
            run_gentufa(
                input,
                stdout,
                stderr,
                color_policy,
                diagnostic_detail,
                diagnostic_terminal_width,
                CliTraceConfig {
                    phase: requested_trace_phase.unwrap_or(TracePhase::Syntax),
                    limit: trace_limit,
                },
            )
        }
        Command::Mulgau(input) => {
            validate_trace_controls_for_unsupported_command(
                "mulgau",
                &input.trace,
                requested_trace_phase,
                trace_limit_present,
                cli.trace_list,
            )?;
            let _ = input.read_text()?;
            command_not_implemented("mulgau")?;
            Ok(CliStatus::Success)
        }
        Command::Tersmu(input) => {
            validate_trace_controls_for_unsupported_command(
                "tersmu",
                &input.trace,
                requested_trace_phase,
                trace_limit_present,
                cli.trace_list,
            )?;
            let _ = input.read_text()?;
            command_not_implemented("tersmu")?;
            Ok(CliStatus::Success)
        }
        Command::Vlacku(_input) => {
            validate_trace_controls_for_unsupported_command(
                "vlacku",
                &None,
                requested_trace_phase,
                trace_limit_present,
                cli.trace_list,
            )?;
            command_not_implemented("vlacku")?;
            Ok(CliStatus::Success)
        }
        Command::Jvozba(_input) => {
            validate_trace_controls_for_unsupported_command(
                "jvozba",
                &None,
                requested_trace_phase,
                trace_limit_present,
                cli.trace_list,
            )?;
            command_not_implemented("jvozba")?;
            Ok(CliStatus::Success)
        }
        Command::Cukta(_input) => {
            validate_trace_controls_for_unsupported_command(
                "cukta",
                &None,
                requested_trace_phase,
                trace_limit_present,
                cli.trace_list,
            )?;
            command_not_implemented("cukta")?;
            Ok(CliStatus::Success)
        }
        Command::Zbasu(input) => {
            validate_trace_controls_for_unsupported_command(
                "zbasu",
                &input.trace,
                requested_trace_phase,
                trace_limit_present,
                cli.trace_list,
            )?;
            let _ = input.read_text()?;
            command_not_implemented("zbasu")?;
            Ok(CliStatus::Success)
        }
        #[cfg(feature = "grammar-debug")]
        Command::Gerna(input) => {
            validate_trace_controls_for_unsupported_command(
                "gerna",
                &None,
                requested_trace_phase,
                trace_limit_present,
                cli.trace_list,
            )?;
            run_gerna(input, stdout)
        }
    }
}

#[requires(diagnostic_terminal_width > 0)]
#[requires(trace.limit > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_gentufa<WOut: Write, WErr: Write>(
    input: GentufaInput,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_detail: DiagnosticDetailMode,
    diagnostic_terminal_width: usize,
    trace: CliTraceConfig,
) -> Result<CliStatus> {
    let rendered = render_gentufa_on_large_stack(
        input,
        color_policy,
        diagnostic_detail,
        diagnostic_terminal_width,
        trace,
    )?;
    stderr.write_all(rendered.stderr.as_bytes())?;
    stdout.write_all(rendered.stdout.as_bytes())?;
    Ok(rendered.status)
}

#[requires(diagnostic_terminal_width > 0)]
#[requires(trace.limit > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn render_gentufa_on_large_stack(
    input: GentufaInput,
    color_policy: CliColorPolicy,
    diagnostic_detail: DiagnosticDetailMode,
    diagnostic_terminal_width: usize,
    trace: CliTraceConfig,
) -> Result<GentufaRendered> {
    let worker = std::thread::Builder::new()
        .name("jbotci-gentufa".to_owned())
        .stack_size(SYNTAX_WORKER_STACK_SIZE)
        .spawn(move || {
            render_gentufa(
                input,
                color_policy,
                diagnostic_detail,
                diagnostic_terminal_width,
                trace,
            )
        })
        .context("failed to spawn gentufa syntax worker")?;
    match worker.join() {
        Ok(result) => result,
        Err(_) => bail!("gentufa syntax worker panicked"),
    }
}

#[cfg(feature = "grammar-debug")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_gerna<WOut: Write>(input: GernaInput, stdout: &mut WOut) -> Result<CliStatus> {
    let output_file = input.output_file.clone();
    let rendered = render_gerna_on_large_stack(input)?;
    write_gerna_output(stdout, output_file.as_ref(), &rendered)?;
    Ok(CliStatus::Success)
}

#[cfg(feature = "grammar-debug")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn render_gerna_on_large_stack(input: GernaInput) -> Result<String> {
    let worker = std::thread::Builder::new()
        .name("jbotci-gerna".to_owned())
        .stack_size(SYNTAX_WORKER_STACK_SIZE)
        .spawn(move || render_gerna(input))
        .context("failed to spawn gerna syntax worker")?;
    match worker.join() {
        Ok(result) => result,
        Err(_) => bail!("gerna syntax worker panicked"),
    }
}

#[cfg(feature = "grammar-debug")]
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|output| !output.is_empty()) || ret.is_err())]
fn render_gerna(input: GernaInput) -> Result<String> {
    let dialect = input.dialect_definition()?;
    let options = ParseOptions::default().with_dialect_definition(&dialect);
    Ok(match input.format {
        GernaFormat::Ebnf => syntax_grammar_ebnf(&options),
        GernaFormat::Svg => syntax_grammar_svg(&options),
    })
}

#[cfg(feature = "grammar-debug")]
#[requires(!rendered.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn write_gerna_output<WOut: Write>(
    stdout: &mut WOut,
    output_file: Option<&PathBuf>,
    rendered: &str,
) -> Result<()> {
    let mut output = rendered.to_owned();
    if !output.ends_with('\n') {
        output.push('\n');
    }
    if let Some(path) = output_file {
        fs::write(path, output)
            .with_context(|| format!("failed to write grammar output to `{}`", path.display()))?;
    } else {
        stdout.write_all(output.as_bytes())?;
    }
    Ok(())
}

#[requires(diagnostic_terminal_width > 0)]
#[requires(trace.limit > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn render_gentufa(
    mut input: GentufaInput,
    color_policy: CliColorPolicy,
    diagnostic_detail: DiagnosticDetailMode,
    diagnostic_terminal_width: usize,
    trace: CliTraceConfig,
) -> Result<GentufaRendered> {
    normalize_trace_text_input(&mut input.trace, &input.file, &mut input.text);
    validate_gentufa_options(&input)?;
    let morphology_trace_options = trace_options(&input.trace, trace.phase, trace.limit)?;
    let syntax_trace_options = trace_options(&input.trace, trace.phase, trace.limit)?;
    let source_label = input_source_label(input.file.as_ref(), input.text.is_empty());
    let text = input.read_text()?;
    let dialect = input.dialect_definition()?;
    let morphology_options = MorphologyOptions::default()
        .with_dialect_definition(&dialect)
        .with_trace_options(morphology_trace_options);
    let morphology_attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
        &text,
        &morphology_options,
        Some(SourceId(source_label.clone())),
    );
    let morphology_trace_stderr = render_cli_trace(
        morphology_attempt.trace.as_ref(),
        color_policy.stderr,
        diagnostic_terminal_width,
    );
    let words = match morphology_attempt.result {
        Ok(words) => words,
        Err(error) => {
            let diagnostic = error.to_diagnostic(Some(SourceId(source_label.clone())), &text);
            let mut stderr = morphology_trace_stderr;
            stderr.push_str(&render_source_diagnostics(
                &source_label,
                &text,
                std::slice::from_ref(&diagnostic),
                color_policy.stderr,
                diagnostic_detail,
                diagnostic_terminal_width,
            )?);
            return Ok(new!(GentufaRendered {
                status: CliStatus::Failure,
                stdout: String::new(),
                stderr,
            }));
        }
    };
    let parse_options = ParseOptions::default()
        .with_dialect_definition(&dialect)
        .with_trace_options(syntax_trace_options);
    let parsed = parse_syntax_tree_with_source_and_options_attempt(&words, &text, &parse_options);
    let trace_stderr = render_cli_trace(
        parsed.trace.as_ref(),
        color_policy.stderr,
        diagnostic_terminal_width,
    );
    let parsed = match parsed.result {
        Ok(parsed) => parsed,
        Err(error) => {
            let diagnostic = error.to_diagnostic(Some(SourceId(source_label.clone())), &text);
            let mut stderr = morphology_trace_stderr;
            stderr.push_str(&trace_stderr);
            stderr.push_str(&render_source_diagnostics(
                &source_label,
                &text,
                std::slice::from_ref(&diagnostic),
                color_policy.stderr,
                diagnostic_detail,
                diagnostic_terminal_width,
            )?);
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
    let mut stderr = morphology_trace_stderr;
    stderr.push_str(&trace_stderr);
    stderr.push_str(&render_source_diagnostics(
        &source_label,
        &text,
        &diagnostics,
        color_policy.stderr,
        diagnostic_detail,
        diagnostic_terminal_width,
    )?);
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
#[requires(diagnostic_terminal_width > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn write_source_diagnostics<W: Write>(
    stderr: &mut W,
    source_label: &str,
    source: &str,
    diagnostics: &[Diagnostic],
    color_enabled: bool,
    diagnostic_detail: DiagnosticDetailMode,
    diagnostic_terminal_width: usize,
) -> Result<()> {
    let rendered = render_source_diagnostics(
        source_label,
        source,
        diagnostics,
        color_enabled,
        diagnostic_detail,
        diagnostic_terminal_width,
    )?;
    stderr.write_all(rendered.as_bytes())?;
    Ok(())
}

#[requires(!source_label.is_empty())]
#[requires(diagnostic_terminal_width > 0)]
#[ensures(diagnostics.is_empty() -> ret.as_ref().is_ok_and(String::is_empty))]
#[ensures(!diagnostics.is_empty() -> ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
fn render_source_diagnostics(
    source_label: &str,
    source: &str,
    diagnostics: &[Diagnostic],
    color_enabled: bool,
    diagnostic_detail: DiagnosticDetailMode,
    diagnostic_terminal_width: usize,
) -> Result<String> {
    render_diagnostics(
        source_label,
        source,
        diagnostics,
        DiagnosticRenderOptions {
            color: color_enabled,
            detail: diagnostic_detail,
            terminal_width: diagnostic_terminal_width,
        },
    )
    .map_err(|error| anyhow!(error))
}

#[requires(limit > 0)]
#[ensures(ret.as_ref().is_ok_and(|options| trace.is_none() == !options.enabled) || ret.is_err())]
fn trace_options(
    trace: &Option<Option<String>>,
    phase: TracePhase,
    limit: usize,
) -> Result<TraceOptions> {
    let Some(spec) = trace else {
        return Ok(TraceOptions::disabled());
    };
    let spec = spec.as_deref().unwrap_or("1");
    let spec = parse_trace_spec(spec)?;
    Ok(TraceOptions::enabled(spec.level, spec.filter, phase, limit))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|parsed| parsed.filter.as_ref().is_none_or(|filter| !filter.name.is_empty())) || ret.is_err())]
fn parse_trace_spec(spec: &str) -> Result<CliParsedTraceSpec> {
    if spec.is_empty() {
        bail!("invalid trace specification: empty value");
    }
    if spec.chars().all(|character| character.is_ascii_digit()) {
        let value = spec
            .parse::<u8>()
            .with_context(|| format!("invalid trace level `{spec}`"))?;
        let level = TraceLevel::from_number(value).map_err(|error| anyhow!(error))?;
        return Ok(CliParsedTraceSpec {
            level,
            filter: None,
        });
    }
    if let Some((filter, level)) = spec.split_once(':') {
        if filter.is_empty() || level.is_empty() {
            bail!("invalid trace specification `{spec}`; use N, rule, or rule:N");
        }
        let value = level
            .parse::<u8>()
            .with_context(|| format!("invalid trace level `{level}`"))?;
        let level = TraceLevel::from_number(value).map_err(|error| anyhow!(error))?;
        return Ok(CliParsedTraceSpec {
            level,
            filter: Some(TraceFilter::new(filter.to_owned())),
        });
    }
    Ok(CliParsedTraceSpec {
        level: TraceLevel::All,
        filter: Some(TraceFilter::new(spec.to_owned())),
    })
}

#[requires(true)]
#[ensures(trace.as_ref().is_none_or(|value| value.as_ref().is_none_or(|text| !text.is_empty())))]
fn normalize_trace_text_input(
    trace: &mut Option<Option<String>>,
    file: &Option<PathBuf>,
    text: &mut Vec<String>,
) {
    let Some(Some(spec)) = trace.as_ref() else {
        return;
    };
    if file.is_some() || !text.is_empty() || trace_spec_can_stand_alone(spec) {
        return;
    }
    let text_arg = spec.clone();
    *trace = Some(None);
    text.push(text_arg);
}

#[requires(true)]
#[ensures(spec.is_empty() -> !ret)]
fn trace_spec_can_stand_alone(spec: &str) -> bool {
    if spec.is_empty() {
        return false;
    }
    if spec
        .parse::<u8>()
        .is_ok_and(|value| TraceLevel::from_number(value).is_ok())
    {
        return true;
    }
    if let Some((filter, level)) = spec.split_once(':') {
        return !filter.is_empty()
            && level
                .parse::<u8>()
                .is_ok_and(|value| TraceLevel::from_number(value).is_ok())
            && is_known_trace_filter(filter);
    }
    is_known_trace_filter(spec)
}

#[requires(true)]
#[ensures(ret -> !name.is_empty())]
fn is_known_trace_filter(name: &str) -> bool {
    SYNTAX_TRACE_FILTERS.contains(&name) || MORPHOLOGY_TRACE_FILTERS.contains(&name)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_trace_controls(
    trace: &Option<Option<String>>,
    options: CliTraceValidation,
) -> Result<()> {
    let trace_enabled = trace.is_some();
    if options.trace_list && trace_enabled {
        bail!("`--trace-list` cannot be combined with `--trace`");
    }
    if options.trace_limit_present && !trace_enabled {
        bail!("`--trace-limit` requires `--trace`");
    }
    if options.trace_phase.is_some() && !trace_enabled && !options.trace_list {
        bail!("`--trace-phase` requires `--trace` or `--trace-list`");
    }
    if options.trace_list && !options.supports_morphology && !options.supports_syntax {
        bail!(
            "`--trace-list` is not supported with `{}`",
            options.command_name
        );
    }
    if trace_enabled && !options.supports_morphology && !options.supports_syntax {
        bail!("`--trace` is not supported with `{}`", options.command_name);
    }
    if let Some(phase) = options.trace_phase
        && !trace_phase_supported(phase, options.supports_morphology, options.supports_syntax)
    {
        bail!(
            "`--trace-phase {}` is not supported with `{}`",
            trace_phase_argument(phase),
            options.command_name
        );
    }
    Ok(())
}

#[requires(!command_name.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_trace_controls_for_unsupported_command(
    command_name: &'static str,
    trace: &Option<Option<String>>,
    trace_phase: Option<TracePhase>,
    trace_limit_present: bool,
    trace_list: bool,
) -> Result<()> {
    validate_trace_controls(
        trace,
        new!(CliTraceValidation {
            command_name,
            trace_phase,
            trace_limit_present,
            trace_list,
            supports_morphology: false,
            supports_syntax: false,
        }),
    )
}

#[requires(true)]
#[ensures(matches!(phase, TracePhase::All) && (supports_morphology || supports_syntax) -> ret)]
fn trace_phase_supported(
    phase: TracePhase,
    supports_morphology: bool,
    supports_syntax: bool,
) -> bool {
    match phase {
        TracePhase::Morphology => supports_morphology,
        TracePhase::Syntax => supports_syntax,
        TracePhase::All => supports_morphology || supports_syntax,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn trace_phase_argument(phase: TracePhase) -> &'static str {
    match phase {
        TracePhase::Morphology => "morphology",
        TracePhase::Syntax => "syntax",
        TracePhase::All => "all",
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn write_trace_filter_list<W: Write>(
    stdout: &mut W,
    phase: TracePhase,
    supports_morphology: bool,
    supports_syntax: bool,
) -> Result<()> {
    match phase {
        TracePhase::Morphology if supports_morphology => {
            write_trace_filter_group(stdout, "morphology", MORPHOLOGY_TRACE_FILTERS)?
        }
        TracePhase::Syntax if supports_syntax => {
            write_trace_filter_group(stdout, "syntax", SYNTAX_TRACE_FILTERS)?
        }
        TracePhase::All => {
            if supports_morphology {
                write_trace_filter_group(stdout, "morphology", MORPHOLOGY_TRACE_FILTERS)?;
            }
            if supports_syntax {
                write_trace_filter_group(stdout, "syntax", SYNTAX_TRACE_FILTERS)?;
            }
        }
        TracePhase::Morphology | TracePhase::Syntax => {
            bail!("unsupported trace phase `{}`", trace_phase_argument(phase));
        }
    }
    Ok(())
}

#[requires(!title.is_empty())]
#[requires(names.iter().all(|name| !name.is_empty()))]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn write_trace_filter_group<W: Write>(stdout: &mut W, title: &str, names: &[&str]) -> Result<()> {
    writeln!(stdout, "{title}:")?;
    for name in names {
        writeln!(stdout, "- {name}")?;
    }
    Ok(())
}

#[requires(terminal_width > 0)]
#[ensures(ret.is_empty() || ret.ends_with('\n'))]
fn render_cli_trace(
    report: Option<&TraceReport>,
    color_enabled: bool,
    terminal_width: usize,
) -> String {
    report.map_or_else(String::new, |report| {
        render_trace_report(
            report,
            TraceRenderOptions {
                color: color_enabled,
                terminal_width,
            },
        )
    })
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
#[ensures(ret > 0)]
fn stderr_terminal_width() -> usize {
    terminal_size::terminal_size_of(std::io::stderr())
        .map(|(terminal_size::Width(width), _height)| usize::from(width))
        .filter(|width| *width > 0)
        .unwrap_or(DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH)
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
    #[cfg(not(feature = "grammar-debug"))]
    use clap::CommandFactory;
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

    #[cfg(not(feature = "grammar-debug"))]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn stable_cli_omits_gerna() {
        assert!(Cli::try_parse_from(["jbotci", "gerna"]).is_err());
        let help = Cli::command().render_long_help().to_string();
        assert!(!help.contains("gerna"));
        assert!(!help.contains("grammar"));
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

    #[cfg(feature = "grammar-debug")]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_gerna_formats_and_flags() {
        let Command::Gerna(default_input) = Cli::try_parse_from(["jbotci", "gerna"])
            .expect("default gerna")
            .command
        else {
            panic!("expected gerna command")
        };
        assert_eq!(default_input.format, GernaFormat::Ebnf);
        assert!(default_input.output_file.is_none());

        let Command::Gerna(svg_input) =
            Cli::try_parse_from(["jbotci", "gerna", "--format", "svg", "-o", "grammar.svg"])
                .expect("gerna svg")
                .command
        else {
            panic!("expected gerna command")
        };
        assert_eq!(svg_input.format, GernaFormat::Svg);
        assert_eq!(svg_input.output_file, Some(PathBuf::from("grammar.svg")));

        let Command::Gerna(dialect_input) =
            Cli::try_parse_from(["jbotci", "gerna", "--dialect", "(zantufa-quotes)"])
                .expect("gerna dialect")
                .command
        else {
            panic!("expected gerna command")
        };
        assert!(
            dialect_input
                .dialect_definition()
                .expect("dialect definition")
                .features
                .contains(&DialectFeature::ZantufaQuotes)
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
        assert!(!default_cli.detailed_errors);

        let bare_cli =
            Cli::try_parse_from(["jbotci", "gentufa", "--color", "coi"]).expect("bare color");
        assert_eq!(bare_cli.color, concolor_clap::ColorChoice::Always);

        let always_cli = Cli::try_parse_from(["jbotci", "gentufa", "--color=always", "coi"])
            .expect("always color");
        assert_eq!(always_cli.color, concolor_clap::ColorChoice::Always);

        let never_cli = Cli::try_parse_from(["jbotci", "gentufa", "--color=never", "coi"])
            .expect("never color");
        assert_eq!(never_cli.color, concolor_clap::ColorChoice::Never);

        let detailed_cli = Cli::try_parse_from(["jbotci", "--detailed-errors", "gentufa", "coi"])
            .expect("detailed errors");
        assert!(detailed_cli.detailed_errors);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_trace_options_and_aliases() {
        let cli = Cli::try_parse_from([
            "jbotci",
            "--trace-phase",
            "all",
            "--trace-limit",
            "7",
            "gentufa",
            "--trace",
            "argument:3",
            "mi",
            "klama",
        ])
        .expect("trace options");
        assert_eq!(cli.trace_phase, Some(CliTracePhase::All));
        assert_eq!(cli.trace_limit, Some(7));
        let Command::Gentufa(input) = cli.command else {
            panic!("expected gentufa command")
        };
        assert_eq!(input.trace, Some(Some("argument:3".to_owned())));
        assert_eq!(input.text, vec!["mi".to_owned(), "klama".to_owned()]);

        let alias_cli =
            Cli::try_parse_from(["jbotci", "vlasei", "--plivei", "2", "coi"]).expect("alias");
        let Command::Vlasei(input) = alias_cli.command else {
            panic!("expected vlasei command")
        };
        assert_eq!(input.trace, Some(Some("2".to_owned())));
        assert_eq!(input.text, vec!["coi".to_owned()]);

        let bare = trace_options(&Some(None), TracePhase::Syntax, 7).expect("bare trace");
        assert!(bare.enabled);
        assert_eq!(bare.level, TraceLevel::Top);
        assert_eq!(bare.phase, TracePhase::Syntax);
        assert_eq!(bare.limit, 7);
        assert!(trace_options(&Some(Some("5".to_owned())), TracePhase::Syntax, 7).is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn trace_list_prints_known_filters() {
        let cli =
            Cli::try_parse_from(["jbotci", "gentufa", "--trace-list"]).expect("trace list parses");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("trace list run");

        assert_eq!(status, CliStatus::Success);
        assert!(error.is_empty());
        let stdout = String::from_utf8(output).expect("stdout utf8");
        assert!(stdout.contains("syntax:"));
        assert!(stdout.contains("- argument"));
        assert!(stdout.contains("- free modifier"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn trace_context_flags_require_trace_or_trace_list() {
        let cases = [
            (
                ["jbotci", "--trace-phase", "syntax", "gentufa", "coi"].as_slice(),
                "`--trace-phase` requires `--trace` or `--trace-list`",
            ),
            (
                ["jbotci", "--trace-limit", "3", "gentufa", "coi"].as_slice(),
                "`--trace-limit` requires `--trace`",
            ),
            (
                [
                    "jbotci",
                    "gentufa",
                    "--trace-list",
                    "--trace",
                    "argument:3",
                    "coi",
                ]
                .as_slice(),
                "`--trace-list` cannot be combined with `--trace`",
            ),
        ];
        for (args, message) in cases {
            let cli = Cli::try_parse_from(args).expect("trace context parses");
            let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
                .expect_err("trace context rejected");
            assert!(error.to_string().contains(message), "{error}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn trace_phase_is_validated_for_command() {
        let cli = Cli::try_parse_from([
            "jbotci",
            "--trace-phase",
            "syntax",
            "vlasei",
            "--trace",
            "coi",
        ])
        .expect("vlasei trace parses");
        let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("syntax trace rejected for vlasei");
        assert!(
            error
                .to_string()
                .contains("`--trace-phase syntax` is not supported with `vlasei`"),
            "{error}"
        );

        let cli = Cli::try_parse_from([
            "jbotci",
            "--trace-phase",
            "morphology",
            "gentufa",
            "--trace-list",
        ])
        .expect("trace list phase parses");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("trace list run");
        assert_eq!(status, CliStatus::Success);
        assert!(error.is_empty());
        let stdout = String::from_utf8(output).expect("stdout utf8");
        assert!(stdout.contains("morphology:"));
        assert!(!stdout.contains("syntax:"));
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
        assert!(stderr.contains("morphology.vowel-hiatus"));
        assert!(stderr.contains("vowels in hiatus are not allowed"));
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
                assert!(stderr.contains("experimental syntax"), "{stderr}");
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
            assert!(stderr.contains("syntax.parse"), "{stderr}");
            assert!(stderr.contains("syntax parse failed"));
            assert!(stderr.contains("expected one of:"));
            assert!(stderr.contains("{be}"));
            assert!(stderr.contains("BRIVLA"));
            assert!(!stderr.contains("needs one of:"));
            assert!(stderr.contains("ku"));
            assert!(!stderr.contains("jbotci:"));
            assert!(!stderr.contains("\x1b["));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_syntax_error_uses_explicit_diagnostic_width() {
        run_on_large_stack(|| {
            let cli =
                Cli::try_parse_from(["jbotci", "gentufa", "gleki", "ku", "klama", "zei", "klama"])
                    .expect("gentufa parses");
            let mut output = Vec::new();
            let mut error = Vec::new();
            let status = run_cli_with_color_policy_and_width(
                cli,
                &mut output,
                &mut error,
                CliColorPolicy::same(false),
                40,
            )
            .expect("gentufa run");

            assert_eq!(status, CliStatus::Failure);
            assert!(output.is_empty());
            let stderr = String::from_utf8(error).expect("stderr utf8");
            assert!(stderr.contains("expected one of:"));
            assert!(stderr.contains("\n            "));
            assert!(!stderr.contains("\x1b["));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_detailed_syntax_errors_show_expectation_breakdown() {
        run_on_large_stack(|| {
            let cli = Cli::try_parse_from([
                "jbotci",
                "gentufa",
                "--detailed-errors",
                "gleki",
                "ku",
                "klama",
                "zei",
                "klama",
            ])
            .expect("gentufa detailed parses");
            let mut output = Vec::new();
            let mut error = Vec::new();
            let status = run_cli(cli, &mut output, &mut error, false).expect("gentufa run");

            assert_eq!(status, CliStatus::Failure);
            assert!(output.is_empty());
            let stderr = String::from_utf8(error).expect("stderr utf8");
            assert!(stderr.contains("needs one of:"));
            assert!(stderr.contains("relation"));
            assert!(stderr.contains("{be}"));
            assert!(stderr.contains("BRIVLA"));
            assert!(stderr.contains("[ends relation, statement or text]"));
            assert!(!stderr.contains("end of input (end of input)"));
            let compact_stderr = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
            let argument = compact_stderr.find("- argument").expect("argument group");
            let relation = compact_stderr
                .find("[continues relation]")
                .expect("relation continuation group");
            let end = compact_stderr.find("[ends relation").expect("end group");
            assert!(argument < relation);
            assert!(relation < end);
            assert!(!stderr.contains("\x1b["));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_syntax_error_labels_unique_current_construct() {
        run_on_large_stack(|| {
            let cli = Cli::try_parse_from(["jbotci", "gentufa", "--detailed-errors", "mi", "cu"])
                .expect("gentufa detailed parses");
            let mut output = Vec::new();
            let mut error = Vec::new();
            let status = run_cli(cli, &mut output, &mut error, false).expect("gentufa run");

            assert_eq!(status, CliStatus::Failure);
            assert!(output.is_empty());
            let stderr = String::from_utf8(error).expect("stderr utf8");
            assert!(stderr.contains("while parsing statement"), "{stderr}");
            assert!(stderr.contains("mi cu"));
            assert!(stderr.contains("needs one of:"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_trace_writes_to_stderr_and_keeps_json_stdout_clean() {
        run_on_large_stack(|| {
            let cli = Cli::try_parse_from([
                "jbotci", "gentufa", "--trace", "1", "--turtai", "json", "mi", "klama",
            ])
            .expect("gentufa trace parses");
            let mut output = Vec::new();
            let mut error = Vec::new();
            let status = run_cli(cli, &mut output, &mut error, false).expect("gentufa run");

            assert_eq!(status, CliStatus::Success);
            let stdout = String::from_utf8(output).expect("stdout utf8");
            let _: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
            assert!(!stdout.contains("trace["));
            let stderr = String::from_utf8(error).expect("stderr utf8");
            assert!(stderr.contains("trace[syntax]"), "{stderr}");
            assert!(!stderr.contains("\x1b["));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_trace_before_text_uses_default_trace_level() {
        run_on_large_stack(|| {
            let cli =
                Cli::try_parse_from(["jbotci", "gentufa", "--trace", "gleki ku klama zei klama"])
                    .expect("bare trace parses");
            let mut output = Vec::new();
            let mut error = Vec::new();
            let status = run_cli(cli, &mut output, &mut error, false).expect("gentufa run");

            assert_eq!(status, CliStatus::Failure);
            assert!(output.is_empty());
            let stderr = String::from_utf8(error).expect("stderr utf8");
            assert!(stderr.contains("trace[syntax]"), "{stderr}");
            assert!(stderr.contains("syntax.parse"), "{stderr}");
            assert!(!stderr.contains("syntax worker panicked"), "{stderr}");
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn trace_color_policy_controls_ansi() {
        run_on_large_stack(|| {
            let always_cli = Cli::try_parse_from([
                "jbotci",
                "gentufa",
                "--color=always",
                "--trace",
                "argument:3",
                "gleki",
                "ku",
                "klama",
                "zei",
                "klama",
            ])
            .expect("always color trace parses");
            let mut output = Vec::new();
            let mut error = Vec::new();
            let status = run_cli(always_cli, &mut output, &mut error, false)
                .expect("always color trace run");
            assert_eq!(status, CliStatus::Failure);
            assert!(output.is_empty());
            let stderr = String::from_utf8(error).expect("stderr utf8");
            assert!(stderr.contains("\x1b["), "{stderr}");

            let never_cli = Cli::try_parse_from([
                "jbotci",
                "gentufa",
                "--color=never",
                "--trace",
                "argument:3",
                "gleki",
                "ku",
                "klama",
                "zei",
                "klama",
            ])
            .expect("never color trace parses");
            let mut output = Vec::new();
            let mut error = Vec::new();
            let status =
                run_cli(never_cli, &mut output, &mut error, true).expect("never color trace run");
            assert_eq!(status, CliStatus::Failure);
            assert!(output.is_empty());
            let stderr = String::from_utf8(error).expect("stderr utf8");
            assert!(stderr.contains("trace[syntax]"), "{stderr}");
            assert!(!stderr.contains("\x1b["), "{stderr}");
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn detailed_syntax_error_color_controls_word_braces() {
        run_on_large_stack(|| {
            let cli = Cli::try_parse_from([
                "jbotci",
                "gentufa",
                "--color=always",
                "--detailed-errors",
                "gleki",
                "ku",
                "klama",
                "zei",
                "klama",
            ])
            .expect("gentufa color parses");
            let mut output = Vec::new();
            let mut error = Vec::new();
            let status = run_cli(cli, &mut output, &mut error, false).expect("gentufa run");

            assert_eq!(status, CliStatus::Failure);
            assert!(output.is_empty());
            let stderr = String::from_utf8(error).expect("stderr utf8");
            assert!(stderr.contains("\x1b["));
            assert!(stderr.contains("be"));
            assert!(!stderr.contains("{be}"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_detailed_morphology_errors_show_detail_note() {
        let cli = Cli::try_parse_from(["jbotci", "vlasei", "--detailed-errors", "aa"])
            .expect("vlasei detailed parses");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("vlasei run");

        assert_eq!(status, CliStatus::Failure);
        assert!(output.is_empty());
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("morphology detail:"));
        assert!(stderr.contains("vowels in hiatus are not allowed"));
        assert!(stderr.contains("while parsing fu'ivla"));
        assert!(stderr.contains("reason"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_trace_writes_morphology_stderr() {
        let cli = Cli::try_parse_from(["jbotci", "vlasei", "--trace", "1", "melxi,or."])
            .expect("vlasei trace parses");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("vlasei run");

        assert_eq!(status, CliStatus::Failure);
        assert!(output.is_empty());
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("trace[morphology]"), "{stderr}");
        assert!(!stderr.contains("unsupported word shape"), "{stderr}");
        assert!(stderr.contains("morphology.cgv-ban"), "{stderr}");
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

    #[cfg(feature = "grammar-debug")]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gerna_ebnf_outputs_named_grammar() {
        let cli = Cli::try_parse_from(["jbotci", "gerna", "--format", "ebnf"]).expect("gerna ebnf");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("gerna ebnf run");

        assert_eq!(status, CliStatus::Success);
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");
        assert!(output.contains("argument"));
        assert!(output.contains("BRIVLA"));
        assert!(output.contains("QUOTE"));
    }

    #[cfg(feature = "grammar-debug")]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gerna_svg_outputs_svg_document() {
        let cli = Cli::try_parse_from(["jbotci", "gerna", "--format", "svg"]).expect("gerna svg");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("gerna svg run");

        assert_eq!(status, CliStatus::Success);
        assert!(error.is_empty());
        let output = String::from_utf8(output).expect("utf8");
        assert!(output.contains("<svg"));
        assert!(output.contains("argument"));
    }

    #[cfg(feature = "grammar-debug")]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gerna_output_file_writes_without_stdout() {
        let path = std::env::temp_dir().join(format!(
            "jbotci-gerna-{}-{}.ebnf",
            std::process::id(),
            "output-file"
        ));
        let _ = fs::remove_file(&path);
        let cli = Cli::try_parse_from([
            "jbotci",
            "gerna",
            "--format",
            "ebnf",
            "--output-file",
            path.to_str().expect("temporary path is utf8"),
        ])
        .expect("gerna output file");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("gerna output run");

        assert_eq!(status, CliStatus::Success);
        assert!(output.is_empty());
        assert!(error.is_empty());
        let file_output = fs::read_to_string(&path).expect("grammar output file");
        let _ = fs::remove_file(&path);
        assert!(file_output.contains("argument"));
    }

    #[cfg(feature = "grammar-debug")]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gerna_dialect_changes_output() {
        let default_cli = Cli::try_parse_from(["jbotci", "gerna"]).expect("default gerna");
        let zantufa_cli = Cli::try_parse_from(["jbotci", "gerna", "--dialect", "(zantufa-quotes)"])
            .expect("zantufa gerna");
        let mut default_output = Vec::new();
        let mut zantufa_output = Vec::new();

        run_cli(default_cli, &mut default_output, &mut Vec::new(), false)
            .expect("default gerna run");
        run_cli(zantufa_cli, &mut zantufa_output, &mut Vec::new(), false)
            .expect("zantufa gerna run");

        let default_output = String::from_utf8(default_output).expect("default utf8");
        let zantufa_output = String::from_utf8(zantufa_output).expect("zantufa utf8");
        assert_ne!(default_output, zantufa_output);
        assert!(zantufa_output.contains("mu'oi"));
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
