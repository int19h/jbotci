mod benchmark;
mod cli;
mod output;
mod tool;

pub use cli::main_entry;
use cli::run_cli_command_with_tool_context;
#[cfg(test)]
use cli::{run_cli, run_cli_with_color_policy_and_width};
use output::*;
pub use tool::*;

use benchmark::BenchmarkMeasurement;
use bityzba::{data, invariant, new, requires};
use std::fs;
use std::io::{IsTerminal, Read, Write};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow, bail};
use clap::{
    Arg, ArgAction, ArgMatches, Args, Command as ClapCommand, FromArgMatches, Parser, Subcommand,
    ValueEnum, value_parser,
};
use clx::progress::{ProgressJobBuilder, ProgressStatus};
use jbotci_cll::{
    CllError, CllRenderFormat, CuktaRequest, CuktaSearchMode, CuktaTargetFilter,
    DEFAULT_CUKTA_CLI_RESULT_COUNT, embedded_cll_site, render_cukta_request, render_search_output,
};
use jbotci_diagnostics::{
    DEFAULT_TRACE_LIMIT, Diagnostic, DiagnosticLabel, DiagnosticPhase, DiagnosticSeverity,
    TraceFilter, TraceLevel, TraceOptions, TracePhase, TraceReport, source_span_from_char_offsets,
};
use jbotci_dialect::{DialectDefinition, DialectSettings, parse_dialect_selection_formula};
use jbotci_embeddings::native::{
    load_backend_for_search, setup_embeddings_with_progress, suppress_llama_logs_for_cli,
};
use jbotci_embeddings::{
    DEFAULT_MODEL_KEY, SetupOptions, SetupProgress, UsePrecomputed, default_index_root,
    semantic_cukta_output, semantic_vlacku_hits,
};
use jbotci_gentufa::{
    EmbeddedGentufaFonts, GentufaBlockAnnotation, GentufaBlockOptions, GentufaPngOptions,
    GentufaScript, GentufaSvgOptions, WebSourceRange,
    generated_model_blocks_layout_with_references, render_gentufa_blocks_png,
    render_gentufa_blocks_svg,
};
use jbotci_gimfihi::{
    CollisionKind, CollisionScope, GIMFIHI_DEFAULT_COUNT, GIMFIHI_MAX_COUNT, GIMFIHI_MAX_WEIGHT,
    GIMFIHI_MIN_WEIGHT, GimfihiCandidate, GimfihiOutput, GimfihiRequest, GimfihiSourceInput,
    GismuCollision, RafsiAvailability, compose_gismu, default_shapes, parse_preset, parse_shape,
    parse_source_spec,
};
use jbotci_jvozba::{
    JvozbaBuildResult, JvozbaInput as JvozbaSourceInput, JvozbaMode, JvozbaSegmentKind,
    build_best_jvozba_detailed,
};
use jbotci_morphology::{
    MORPHOLOGY_TRACE_FILTERS, MorphologyOptions, MorphologyWarning, Phonemes,
    PlainWordClassification, ValsiAnalysis, ValsiAnalysisStatus, ValsiClassification,
    ValsiClassificationKind, ValsiFuhivlaStage, ValsiLujvoPart, ValsiLujvoPartKind,
    ValsiLujvoRafsiKind, WordKind, WordLike, analyze_valsi_with_options_and_source_id,
    segment_words_with_modifiers_with_options_and_source_id_attempt,
};
use jbotci_output::{
    BracketRenderOptions, DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH, DiagnosticDetailMode,
    DiagnosticRenderOptions, GlideMark, GlyphStyle, JsonRenderOptions, LojbanScript,
    PhonemeRenderOptions, StressMark, TraceRenderOptions, TreeRenderOptions,
    compact_generated_model_json_string_with_options, compact_morphology_json_string_with_options,
    compact_morphology_json_value, format_definition_or_notes_line_with_indexed_places,
    generated_reference_display, ipa_morphology_text, json_string_with_options,
    pretty_generated_model_brackets_with_options, pretty_generated_model_tree_with_options,
    pretty_morphology_brackets_with_options, pretty_morphology_tree_with_options,
    render_diagnostics, render_json_value_with_options, render_trace_report,
};
use jbotci_search::vlacku::{
    DEFAULT_VLACKU_RESULT_COUNT, VlackuCard, VlackuCompositionKind, VlackuCompositionPiece,
    VlackuOutcome, VlackuRequest, VlackuRequestData, VlackuSearchOptions, VlackuSearchOutput,
    WordTypeFilter, dictionary_cards_for_word_likes, dictionary_entry_card,
    dictionary_entry_passes_vlacku_filters, dictionary_matches_for_word_likes, format_vote_display,
    normalize_word_type_filter, parse_word_type_filter, run_vlacku_requests,
};
use jbotci_semantics::{
    SemanticBuildOptions, build_generated_semantic_graph_with_dictionary_and_options,
};
use jbotci_source::SourceId;
use jbotci_syntax::{
    ParseOptions, SYNTAX_TRACE_FILTERS, parse_syntax_tree_generated_model_with_source_and_options,
    parse_syntax_tree_generated_model_with_source_and_options_attempt,
};
#[cfg(feature = "grammar-debug")]
use jbotci_syntax::{syntax_grammar_ebnf, syntax_grammar_svg};
use unicode_width::UnicodeWidthStr;

#[cfg(test)]
use jbotci_search::vlacku::VlackuAuthor;

const VLACKU_DETAIL_INDENT: &str = "    ";

#[derive(Debug, Clone, Parser)]
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
    #[arg(long = "benchmark", global = true, value_name = "N")]
    benchmark: Option<NonZeroUsize>,
    #[command(subcommand)]
    command: Command,
}

#[invariant(true)]
#[invariant(::Vlasei(..) => true)]
#[invariant(::Vlatai(..) => true)]
#[invariant(::Gentufa(..) => true)]
#[invariant(::Mulgau(..) => true)]
#[invariant(::Tersmu(..) => true)]
#[invariant(::Vlacku(..) => true)]
#[invariant(::Jvozba(..) => true)]
#[invariant(::Gimfihi(..) => true)]
#[invariant(::Cukta(..) => true)]
#[invariant(::Zbasu(..) => true)]
#[invariant(::Setup(..) => true)]
#[invariant(::Gerna(..) => true)]
#[derive(Debug, Clone, Subcommand)]
enum Command {
    #[command(name = "vlasei", visible_alias = "lex")]
    Vlasei(VlaseiInput),
    #[command(name = "vlatai")]
    Vlatai(VlataiInput),
    #[command(name = "gentufa", visible_alias = "parse")]
    Gentufa(GentufaInput),
    #[command(name = "mulgau", visible_alias = "completions")]
    Mulgau(TextInput),
    #[command(name = "tersmu")]
    Tersmu(TersmuInput),
    #[command(name = "vlacku", visible_alias = "dict")]
    Vlacku(VlackuInput),
    #[command(name = "jvozba")]
    Jvozba(JvozbaInput),
    #[command(name = "gimfihi", alias = "gimfi'i")]
    Gimfihi(GimfihiInput),
    #[command(name = "cukta", visible_alias = "book")]
    Cukta(CuktaInput),
    #[command(name = "zbasu")]
    Zbasu(TextInput),
    #[command(name = "setup")]
    Setup(SetupInput),
    #[cfg(feature = "grammar-debug")]
    #[command(name = "gerna", visible_alias = "grammar")]
    Gerna(GernaInput),
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliStatus {
    Success,
    Failure,
    ValidMissing,
    InvalidInput,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CliProgressPolicy {
    embedding_setup: bool,
}

impl CliProgressPolicy {
    #[requires(true)]
    #[ensures(!ret.embedding_setup)]
    fn disabled() -> Self {
        Self {
            embedding_setup: false,
        }
    }

    #[requires(true)]
    #[ensures(ret.embedding_setup == enabled)]
    fn embedding_setup(enabled: bool) -> Self {
        Self {
            embedding_setup: enabled,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum GentufaFormat {
    Brackets,
    Blocks,
    #[value(alias = "vipcihe", help = "alias: vipcihe")]
    Tree,
    Raw,
    #[value(alias = "djeisone")]
    Json,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum TersmuFormat {
    #[value(alias = "djeisone")]
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum GentufaImageOutputType {
    Svg,
    Png,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum VlaseiFormat {
    Brackets,
    Tree,
    Ipa,
    Raw,
    #[value(alias = "djeisone")]
    Json,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum VlataiFormat {
    Text,
    #[value(alias = "djeisone")]
    Json,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum GimfihiCliFormat {
    Table,
    #[value(alias = "djeisone")]
    Json,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliCollisionScope {
    All,
    Official,
    None,
}

impl From<CliCollisionScope> for CollisionScope {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: CliCollisionScope) -> Self {
        match value {
            CliCollisionScope::All => Self::All,
            CliCollisionScope::Official => Self::Official,
            CliCollisionScope::None => Self::None,
        }
    }
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

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct CliParsedTraceSpec {
    level: TraceLevel,
    filter: Option<TraceFilter>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[invariant(true)]
#[derive(Debug, Clone, Args)]
struct VlaseiInput {
    #[arg(long = "file", alias = "sfaile")]
    file: Option<PathBuf>,
    #[arg(long = "ascii")]
    ascii: bool,
    #[arg(long = "detailed-errors")]
    detailed_errors: bool,
    #[arg(long = "trace-phase", value_enum)]
    trace_phase: Option<CliTracePhase>,
    #[arg(long = "trace-limit")]
    trace_limit: Option<usize>,
    #[arg(long = "trace-list")]
    trace_list: bool,
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
        self.read_text_with_stdin(None)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn read_text_with_stdin(&self, stdin_text: Option<&str>) -> Result<String> {
        read_text_input(self.file.as_ref(), &self.text, stdin_text)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn dialect_definition(&self) -> Result<DialectDefinition> {
        dialect_definition(self.dialect.as_deref())
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Args)]
struct VlataiInput {
    #[arg(long = "ascii")]
    ascii: bool,
    #[arg(long = "detailed-errors")]
    detailed_errors: bool,
    #[arg(
        long = "turtai",
        visible_alias = "format",
        default_value_t = VlataiFormat::Text,
        value_enum
    )]
    format: VlataiFormat,
    #[arg(long = "indent")]
    indent: Option<usize>,
    #[arg(long = "dialect")]
    dialect: Option<String>,
    #[arg(long = "mark-stress", value_enum)]
    mark_stress: Option<CliStressMark>,
    #[arg(long = "mark-glides", value_enum)]
    mark_glides: Option<CliGlideMark>,
    #[arg(required = true)]
    words: Vec<String>,
}

impl VlataiInput {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn dialect_definition(&self) -> Result<DialectDefinition> {
        dialect_definition(self.dialect.as_deref())
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Args)]
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
    #[arg(long = "indent")]
    indent: Option<usize>,
    #[arg()]
    text: Vec<String>,
}

impl TextInput {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn read_text(&self) -> Result<String> {
        self.read_text_with_stdin(None)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn read_text_with_stdin(&self, stdin_text: Option<&str>) -> Result<String> {
        read_text_input(self.file.as_ref(), &self.text, stdin_text)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn dialect_definition(&self) -> Result<DialectDefinition> {
        dialect_definition(self.dialect.as_deref())
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Args)]
struct TersmuInput {
    #[arg(long = "file", alias = "sfaile")]
    file: Option<PathBuf>,
    #[arg(
        long = "format",
        default_value_t = TersmuFormat::Json,
        value_enum
    )]
    format: TersmuFormat,
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
    #[arg(long = "story-time")]
    story_time: bool,
    #[arg(long = "indent")]
    indent: Option<usize>,
    #[arg()]
    text: Vec<String>,
}

impl TersmuInput {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn read_text_with_stdin(&self, stdin_text: Option<&str>) -> Result<String> {
        read_text_input(self.file.as_ref(), &self.text, stdin_text)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn dialect_definition(&self) -> Result<DialectDefinition> {
        dialect_definition(self.dialect.as_deref())
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Args)]
struct GentufaInput {
    #[arg(long = "file", alias = "sfaile")]
    file: Option<PathBuf>,
    #[arg(long = "ascii")]
    ascii: bool,
    #[arg(long = "detailed-errors")]
    detailed_errors: bool,
    #[arg(long = "error-context", default_value_t = 1)]
    error_context: usize,
    #[arg(long = "trace-phase", value_enum)]
    trace_phase: Option<CliTracePhase>,
    #[arg(long = "trace-limit")]
    trace_limit: Option<usize>,
    #[arg(long = "trace-list")]
    trace_list: bool,
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
    #[arg(long = "show-defs")]
    show_defs: bool,
    #[arg(long = "indent")]
    indent: Option<usize>,
    #[arg(long = "mark-stress", value_enum)]
    mark_stress: Option<CliStressMark>,
    #[arg(long = "mark-glides", value_enum)]
    mark_glides: Option<CliGlideMark>,
    #[arg(long = "show-spans")]
    show_spans: bool,
    #[arg(long = "show-refs")]
    show_refs: bool,
    #[arg(long = "show-elided")]
    show_elided: bool,
    #[arg(long = "decompose-lujvo")]
    decompose_lujvo: bool,
    #[arg(long = "output-type", value_enum)]
    output_type: Option<GentufaImageOutputType>,
    #[arg(short = 'o', long = "output-file")]
    output_file: Option<PathBuf>,
    #[arg()]
    text: Vec<String>,
}

#[invariant(stderr.is_empty() || stderr.ends_with('\n'))]
struct GentufaRendered {
    status: CliStatus,
    stdout: Vec<u8>,
    stderr: String,
}

#[invariant(stderr.is_empty() || stderr.ends_with('\n'))]
struct TersmuRendered {
    status: CliStatus,
    stdout: Vec<u8>,
    stderr: String,
}

impl GentufaInput {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn read_text(&self) -> Result<String> {
        self.read_text_with_stdin(None)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn read_text_with_stdin(&self, stdin_text: Option<&str>) -> Result<String> {
        read_text_input(self.file.as_ref(), &self.text, stdin_text)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    fn dialect_definition(&self) -> Result<DialectDefinition> {
        dialect_definition(self.dialect.as_deref())
    }
}

#[cfg(feature = "grammar-debug")]
#[invariant(true)]
#[derive(Debug, Clone, Args)]
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

#[invariant(true)]
#[derive(Debug, Clone, Args)]
struct CuktaInput {
    #[arg(short = 'n', long = "count")]
    count: Option<usize>,
    #[arg(long = "toc")]
    toc: bool,
    #[arg(long = "section", value_name = "REF")]
    section: Option<String>,
    #[arg(long = "example", value_name = "REF")]
    example: Option<String>,
    #[arg(long = "valsi", value_name = "WORD")]
    valsi: Option<String>,
    #[arg(long = "target", value_name = "section|paragraph|example", action = ArgAction::Append)]
    targets: Vec<String>,
    #[arg(long = "sections")]
    target_sections: bool,
    #[arg(long = "paragraphs")]
    target_paragraphs: bool,
    #[arg(long = "examples")]
    target_examples: bool,
    #[arg(
        long = "turtai",
        visible_alias = "format",
        default_value_t = CuktaCliFormat::Markdown,
        value_enum
    )]
    format: CuktaCliFormat,
    #[arg()]
    query: Vec<String>,
}

#[invariant(true)]
#[derive(Debug, Clone, Args)]
struct GimfihiInput {
    /// A source word as `LANG[:WEIGHT]:WORD` (repeat per source). WORD is Lojban
    /// letters, or a phonemic IPA transcription in `[ ... ]` brackets (e.g.
    /// `eng:210:[kæt]`).
    #[arg(
        long = "source",
        value_name = "LANG[:WEIGHT]:WORD",
        value_parser = parse_source_spec,
        action = ArgAction::Append
    )]
    sources: Vec<GimfihiSourceInput>,
    #[arg(long = "preset", value_name = "PRESET")]
    preset: Option<String>,
    #[arg(long = "shape", value_name = "SHAPE", action = ArgAction::Append)]
    shapes: Vec<String>,
    #[arg(
        long = "check-collisions",
        value_enum,
        default_value_t = CliCollisionScope::All
    )]
    check_collisions: CliCollisionScope,
    #[arg(long = "all-letters")]
    all_letters: bool,
    #[arg(long = "show-collisions")]
    show_collisions: bool,
    #[arg(long = "require-free-short-rafsi")]
    require_free_short_rafsi: bool,
    #[arg(short = 'n', long = "count", value_name = "N")]
    count: Option<usize>,
    #[arg(long = "highlight", value_name = "GISMU")]
    highlight: Option<String>,
    #[arg(long = "format", value_enum, default_value_t = GimfihiCliFormat::Table)]
    format: GimfihiCliFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CuktaCliFormat {
    Markdown,
    Html,
    #[value(alias = "docbook")]
    Raw,
}

impl From<CuktaCliFormat> for CllRenderFormat {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: CuktaCliFormat) -> Self {
        match value {
            CuktaCliFormat::Markdown => Self::Markdown,
            CuktaCliFormat::Html => Self::Html,
            CuktaCliFormat::Raw => Self::Raw,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CliSumtiPlaces {
    Raw,
    Index,
}

impl CliSumtiPlaces {
    #[requires(true)]
    #[ensures(true)]
    fn parse(value: &str) -> Option<Self> {
        match value {
            "raw" => Some(Self::Raw),
            "index" => Some(Self::Index),
            _ => None,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct VlackuInput {
    count: Option<usize>,
    ascii: bool,
    word_types: Vec<String>,
    min_votes: Option<i32>,
    min_similarity: Option<f32>,
    sumti_places: CliSumtiPlaces,
    decompose_lujvo: bool,
    show_etymology: bool,
    requests: Vec<VlackuRequest>,
    query: Vec<String>,
}

#[invariant(true)]
#[derive(Debug, Clone, Args)]
struct SetupInput {
    #[arg(long = "embedding")]
    embedding: bool,
    #[arg(long = "force")]
    force: bool,
    #[arg(
        long = "use-precomputed",
        value_enum,
        default_value_t = CliUsePrecomputed::Auto
    )]
    use_precomputed: CliUsePrecomputed,
    #[arg(long = "skip-validation")]
    skip_validation: bool,
    #[arg(long = "model", default_value = DEFAULT_MODEL_KEY)]
    model: String,
    #[arg(long = "index-dir")]
    index_dir: Option<PathBuf>,
    #[arg(long = "model-dir")]
    model_dir: Option<PathBuf>,
}

#[invariant(::Auto => true)]
#[invariant(::Always => true)]
#[invariant(::Never => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum CliUsePrecomputed {
    Auto,
    Always,
    Never,
}

impl From<CliUsePrecomputed> for UsePrecomputed {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: CliUsePrecomputed) -> Self {
        match value {
            CliUsePrecomputed::Auto => Self::Auto,
            CliUsePrecomputed::Always => Self::Always,
            CliUsePrecomputed::Never => Self::Never,
        }
    }
}

impl Args for VlackuInput {
    #[requires(true)]
    #[ensures(true)]
    fn augment_args(command: ClapCommand) -> ClapCommand {
        augment_vlacku_args(command)
    }

    #[requires(true)]
    #[ensures(true)]
    fn augment_args_for_update(command: ClapCommand) -> ClapCommand {
        augment_vlacku_args(command)
    }
}

impl FromArgMatches for VlackuInput {
    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn from_arg_matches(matches: &ArgMatches) -> std::result::Result<Self, clap::Error> {
        Ok(parse_vlacku_matches(matches))
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn update_from_arg_matches(
        &mut self,
        matches: &ArgMatches,
    ) -> std::result::Result<(), clap::Error> {
        *self = parse_vlacku_matches(matches);
        Ok(())
    }
}

#[requires(true)]
#[ensures(true)]
fn augment_vlacku_args(command: ClapCommand) -> ClapCommand {
    command
        .arg(
            Arg::new("count")
                .short('n')
                .long("count")
                .value_name("N")
                .value_parser(value_parser!(usize)),
        )
        .arg(Arg::new("ascii").long("ascii").action(ArgAction::SetTrue))
        .arg(
            Arg::new("word_type")
                .long("word-type")
                .value_name("T,...")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("min_votes")
                .long("min-votes")
                .value_name("N")
                .value_parser(value_parser!(i32)),
        )
        .arg(
            Arg::new("min_similarity")
                .long("min-similarity")
                .value_name("PCT")
                .value_parser(value_parser!(f32)),
        )
        .arg(
            Arg::new("sumti_places")
                .long("sumti-places")
                .value_name("STYLE")
                .value_parser(["raw", "index"]),
        )
        .arg(
            Arg::new("valsi")
                .long("valsi")
                .value_name("WORD")
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("rafsi")
                .long("rafsi")
                .value_name("RAFSI")
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("lujvo")
                .long("lujvo")
                .value_name("WORD")
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("sound")
                .long("sound")
                .value_name("TEXT|[IPA]")
                .value_parser(clap::builder::NonEmptyStringValueParser::new())
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("decompose_lujvo")
                .long("decompose-lujvo")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("show_etymology")
                .long("show-etymology")
                .action(ArgAction::SetTrue),
        )
        .arg(Arg::new("query").action(ArgAction::Append).num_args(0..))
}

#[requires(true)]
#[ensures(true)]
fn parse_vlacku_matches(matches: &ArgMatches) -> VlackuInput {
    let mut ordered_requests = Vec::new();
    collect_ordered_vlacku_requests(
        matches,
        "valsi",
        VlackuRequest::valsi,
        &mut ordered_requests,
    );
    collect_ordered_vlacku_requests(
        matches,
        "rafsi",
        VlackuRequest::rafsi,
        &mut ordered_requests,
    );
    collect_ordered_vlacku_requests(
        matches,
        "lujvo",
        VlackuRequest::lujvo,
        &mut ordered_requests,
    );
    collect_ordered_vlacku_requests(
        matches,
        "sound",
        VlackuRequest::sound,
        &mut ordered_requests,
    );
    ordered_requests.sort_by_key(|(index, _)| *index);

    VlackuInput {
        count: matches.get_one::<usize>("count").copied(),
        ascii: matches.get_flag("ascii"),
        word_types: matches
            .get_many::<String>("word_type")
            .map(|values| values.cloned().collect())
            .unwrap_or_default(),
        min_votes: matches.get_one::<i32>("min_votes").copied(),
        min_similarity: matches.get_one::<f32>("min_similarity").copied(),
        sumti_places: matches
            .get_one::<String>("sumti_places")
            .and_then(|value| CliSumtiPlaces::parse(value))
            .unwrap_or(CliSumtiPlaces::Index),
        decompose_lujvo: matches.get_flag("decompose_lujvo"),
        show_etymology: matches.get_flag("show_etymology"),
        requests: ordered_requests
            .into_iter()
            .map(|(_, request)| request)
            .collect(),
        query: matches
            .get_many::<String>("query")
            .map(|values| values.cloned().collect())
            .unwrap_or_default(),
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_ordered_vlacku_requests<F>(
    matches: &ArgMatches,
    id: &'static str,
    make_request: F,
    output: &mut Vec<(usize, VlackuRequest)>,
) where
    F: Fn(String) -> VlackuRequest,
{
    let values = matches
        .get_many::<String>(id)
        .map(|values| values.cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let indices = matches
        .indices_of(id)
        .map(|indices| indices.collect::<Vec<_>>())
        .unwrap_or_default();
    for (index, value) in indices.into_iter().zip(values) {
        output.push((index, make_request(value)));
    }
}

#[invariant(true)]
#[derive(Debug, Clone)]
struct JvozbaInput {
    cmevla: bool,
    sources: Vec<JvozbaSourceInput>,
}

impl Args for JvozbaInput {
    #[requires(true)]
    #[ensures(true)]
    fn augment_args(command: ClapCommand) -> ClapCommand {
        augment_jvozba_args(command)
    }

    #[requires(true)]
    #[ensures(true)]
    fn augment_args_for_update(command: ClapCommand) -> ClapCommand {
        augment_jvozba_args(command)
    }
}

impl FromArgMatches for JvozbaInput {
    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn from_arg_matches(matches: &ArgMatches) -> std::result::Result<Self, clap::Error> {
        Ok(parse_jvozba_matches(matches))
    }

    #[requires(true)]
    #[ensures(ret.is_ok())]
    fn update_from_arg_matches(
        &mut self,
        matches: &ArgMatches,
    ) -> std::result::Result<(), clap::Error> {
        *self = parse_jvozba_matches(matches);
        Ok(())
    }
}

#[requires(true)]
#[ensures(true)]
fn augment_jvozba_args(command: ClapCommand) -> ClapCommand {
    command
        .arg(Arg::new("cmevla").long("cmevla").action(ArgAction::SetTrue))
        .arg(
            Arg::new("rafsi")
                .long("rafsi")
                .value_name("RAFSI")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("parts")
                .value_name("WORD")
                .action(ArgAction::Append)
                .num_args(0..),
        )
}

#[requires(true)]
#[ensures(true)]
fn parse_jvozba_matches(matches: &ArgMatches) -> JvozbaInput {
    let mut ordered_sources = Vec::new();
    collect_ordered_jvozba_sources(
        matches,
        "parts",
        JvozbaSourceInput::Word,
        &mut ordered_sources,
    );
    collect_ordered_jvozba_sources(
        matches,
        "rafsi",
        JvozbaSourceInput::FixedRafsi,
        &mut ordered_sources,
    );
    ordered_sources.sort_by_key(|(index, _)| *index);
    JvozbaInput {
        cmevla: matches.get_flag("cmevla"),
        sources: ordered_sources
            .into_iter()
            .map(|(_, source)| source)
            .collect(),
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_ordered_jvozba_sources<F>(
    matches: &ArgMatches,
    id: &'static str,
    make_source: F,
    output: &mut Vec<(usize, JvozbaSourceInput)>,
) where
    F: Fn(String) -> JvozbaSourceInput,
{
    let values = matches
        .get_many::<String>(id)
        .map(|values| values.cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let indices = matches
        .indices_of(id)
        .map(|indices| indices.collect::<Vec<_>>())
        .unwrap_or_default();
    for (index, value) in indices.into_iter().zip(values) {
        output.push((index, make_source(value)));
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_setup<WOut: Write>(
    input: SetupInput,
    stdout: &mut WOut,
    progress_policy: CliProgressPolicy,
) -> Result<CliStatus> {
    if !input.embedding {
        bail!("Choose at least one setup task, e.g. `jbotci setup --embedding`.");
    }
    let mut reporter = CliSetupProgressReporter::new(progress_policy.embedding_setup);
    let mut progress = |progress: SetupProgress| {
        reporter.update(&progress);
    };
    let report = match setup_embeddings_with_progress(
        &SetupOptions {
            model_key: input.model,
            force: input.force,
            use_precomputed: input.use_precomputed.into(),
            skip_validation: input.skip_validation,
            index_dir: input.index_dir,
            model_dir: input.model_dir,
            ..SetupOptions::default()
        },
        &mut progress,
    ) {
        Ok(report) => {
            reporter.finish();
            report
        }
        Err(error) => {
            reporter.fail();
            return Err(anyhow!(error.to_string()));
        }
    };
    writeln!(
        stdout,
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
    )?;
    Ok(CliStatus::Success)
}

#[invariant(true)]
#[derive(Debug)]
struct CliSetupProgressReporter {
    job: Option<std::sync::Arc<clx::progress::ProgressJob>>,
    determinate: bool,
}

impl CliSetupProgressReporter {
    #[requires(true)]
    #[ensures(enabled -> ret.job.is_some() || clx::progress::is_disabled())]
    fn new(enabled: bool) -> Self {
        if !enabled || clx::progress::is_disabled() {
            return Self {
                job: None,
                determinate: false,
            };
        }
        let job = ProgressJobBuilder::new()
            .body("{{ spinner() }} {{ message }} {{ detail | flex }}")
            .prop("message", "Embedding setup")
            .prop("detail", "")
            .start();
        Self {
            job: Some(job),
            determinate: false,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn update(&mut self, progress: &SetupProgress) {
        let Some(job) = &self.job else {
            return;
        };
        let detail = cli_setup_progress_detail(progress);
        if let (Some(loaded), Some(total)) = (progress.loaded, progress.total) {
            if !self.determinate {
                job.set_body("{{ spinner() }} {{ message }} {{ detail | flex }} {{ progress_bar(flex=true) }}");
                self.determinate = true;
            }
            job.progress_total(usize::try_from(total).unwrap_or(usize::MAX));
            job.progress_current(usize::try_from(loaded).unwrap_or(usize::MAX));
        } else if self.determinate {
            job.set_body("{{ spinner() }} {{ message }} {{ detail | flex }}");
            self.determinate = false;
        }
        job.message(&progress.label);
        job.prop("detail", &detail);
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(&mut self) {
        if let Some(job) = &self.job {
            job.set_status(ProgressStatus::Done);
            clx::progress::stop_clear();
        }
        self.job = None;
    }

    #[requires(true)]
    #[ensures(true)]
    fn fail(&mut self) {
        if let Some(job) = &self.job {
            job.set_status(ProgressStatus::Failed);
            clx::progress::stop_clear();
        }
        self.job = None;
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn cli_setup_progress_detail(progress: &SetupProgress) -> String {
    if let (Some(loaded), Some(total)) = (progress.loaded, progress.total) {
        return match progress.kind.as_str() {
            "download" | "validate" => format!("{} / {}", human_bytes(loaded), human_bytes(total)),
            _ => format!("{loaded}/{total} rows"),
        };
    }
    if progress.detail.is_empty() {
        return progress.label.clone();
    }
    progress.detail.clone()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_vlacku<WOut: Write, WErr: Write>(
    input: VlackuInput,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color: bool,
    glyphs: GlyphStyle,
    output_terminal_width: Option<usize>,
    tool_context: Option<&mut ToolExecutionContext<'_>>,
) -> Result<CliStatus> {
    validate_vlacku_input(&input)?;
    let options = vlacku_search_options(&input)?;
    let output = if input.requests.is_empty() {
        match run_semantic_vlacku(&input, &options, tool_context) {
            Ok(output) => output,
            Err(error) => {
                writeln!(stderr, "vlacku: {error}")?;
                return Ok(CliStatus::InvalidInput);
            }
        }
    } else {
        run_vlacku_requests(jbotci_dictionary_data::english(), &input.requests, &options)
    };
    for diagnostic in &output.diagnostics {
        writeln!(stderr, "vlacku: {diagnostic}")?;
    }
    if !output.cards.is_empty() || output.outcome != VlackuOutcome::Invalid {
        write!(
            stdout,
            "{}",
            render_vlacku_output_with_options(
                &output,
                new!(VlackuRenderOptions {
                    color,
                    glyphs,
                    output_terminal_width,
                    sumti_places: input.sumti_places,
                    show_etymology: input.show_etymology,
                }),
            )
        )?;
    }
    Ok(cli_status_from_vlacku_outcome(output.outcome))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_semantic_vlacku(
    input: &VlackuInput,
    options: &VlackuSearchOptions,
    tool_context: Option<&mut ToolExecutionContext<'_>>,
) -> Result<VlackuSearchOutput> {
    let query = joined_query_text(&input.query).trim().to_owned();
    if query.is_empty() {
        bail!("vlacku query text must be non-empty.");
    }
    let dictionary = jbotci_dictionary_data::english();
    let hits = if let Some(context) = tool_context {
        if let Some(service) = context.embedding_search()? {
            service
                .semantic_vlacku_hits(&query, dictionary.entries().len())
                .map_err(|error| anyhow!(error.to_string()))?
        } else {
            semantic_vlacku_hits_with_new_backend(&query, dictionary.entries().len())?
        }
    } else {
        semantic_vlacku_hits_with_new_backend(&query, dictionary.entries().len())?
    };
    let cards = hits
        .into_iter()
        .filter_map(|hit| {
            dictionary
                .entries()
                .get(hit.entry_index)
                .map(|entry| (hit.score, entry))
        })
        .filter(|(score, entry)| {
            dictionary_entry_passes_vlacku_filters(entry, options, Some(*score), true)
        })
        .take(options.count)
        .map(|(score, entry)| {
            dictionary_entry_card(dictionary, entry, Some(score), options.decompose_lujvo)
        })
        .collect::<Vec<_>>();
    Ok(VlackuSearchOutput {
        cards,
        outcome: VlackuOutcome::Found,
        diagnostics: Vec::new(),
    })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn semantic_vlacku_hits_with_new_backend(
    query: &str,
    count: usize,
) -> Result<Vec<jbotci_embeddings::DictionarySemanticHit>> {
    let index_root = default_index_root().map_err(|error| anyhow!(error.to_string()))?;
    let mut backend = load_backend_for_search(DEFAULT_MODEL_KEY, None)
        .map_err(|error| anyhow!(error.to_string()))?;
    semantic_vlacku_hits(&mut backend, query, count, &index_root, DEFAULT_MODEL_KEY)
        .map_err(|error| anyhow!(error.to_string()))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_cukta<WOut: Write, WErr: Write>(
    input: CuktaInput,
    stdout: &mut WOut,
    stderr: &mut WErr,
    tool_context: Option<&mut ToolExecutionContext<'_>>,
) -> Result<CliStatus> {
    validate_cukta_input(&input)?;
    let request = cukta_request_from_input(&input)?;
    let site = embedded_cll_site().map_err(|error| anyhow!(error.to_string()))?;
    let mut tool_context = tool_context;
    let rendered = match &request {
        CuktaRequest::Search {
            mode: CuktaSearchMode::Meaning,
            query,
            count,
            targets,
        } => {
            let output = match run_semantic_cukta(
                tool_context.as_deref_mut(),
                site,
                query,
                *count,
                *targets,
            ) {
                Ok(output) => output,
                Err(error) => {
                    writeln!(stderr, "{error}")?;
                    return Ok(CliStatus::InvalidInput);
                }
            };
            render_search_output(&output, input.format.into())
        }
        _ => match render_cukta_request(site, &request, input.format.into()) {
            Ok(rendered) => rendered,
            Err(CllError::SemanticSearchDisabled) => {
                writeln!(stderr, "{}", CllError::SemanticSearchDisabled)?;
                return Ok(CliStatus::InvalidInput);
            }
            Err(error) => return Err(anyhow!(error.to_string())),
        },
    };
    write!(stdout, "{rendered}")?;
    if !rendered.ends_with('\n') {
        writeln!(stdout)?;
    }
    Ok(CliStatus::Success)
}

#[requires(!query.trim().is_empty())]
#[requires(count > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_semantic_cukta(
    tool_context: Option<&mut ToolExecutionContext<'_>>,
    site: &jbotci_cll::CllSite,
    query: &str,
    count: usize,
    targets: CuktaTargetFilter,
) -> Result<jbotci_cll::CuktaSearchOutput> {
    let chunks = jbotci_cll::cll_search_all_chunks(site);
    if let Some(context) = tool_context
        && let Some(service) = context.embedding_search()?
    {
        return service
            .semantic_cukta_output(chunks, query, count, targets)
            .map_err(|error| anyhow!(error.to_string()));
    }
    let index_root = default_index_root().map_err(|error| anyhow!(error.to_string()))?;
    let mut backend = load_backend_for_search(DEFAULT_MODEL_KEY, None)
        .map_err(|error| anyhow!(error.to_string()))?;
    semantic_cukta_output(
        &mut backend,
        chunks,
        query,
        count,
        targets,
        &index_root,
        DEFAULT_MODEL_KEY,
    )
    .map_err(|error| anyhow!(error.to_string()))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_cukta_input(input: &CuktaInput) -> Result<()> {
    if input.count == Some(0) {
        bail!("`--count` must be greater than 0");
    }
    let request_mode_count = usize::from(input.toc)
        + usize::from(input.section.is_some())
        + usize::from(input.example.is_some())
        + usize::from(input.valsi.is_some())
        + usize::from(!input.query.is_empty());
    if request_mode_count > 1 {
        bail!(
            "Choose only one cukta mode: --toc, --section, --example, --valsi, or a positional query."
        );
    }
    if !input.targets.is_empty()
        || input.target_sections
        || input.target_paragraphs
        || input.target_examples
    {
        let _ = cukta_target_filter_from_input(input)?;
        if input.toc || input.section.is_some() || input.example.is_some() {
            bail!("Cukta target filters are only valid with search modes.");
        }
    }
    if request_mode_count == 0 {
        return Ok(());
    }
    if let Some(valsi) = &input.valsi
        && valsi.trim().is_empty()
    {
        bail!("`--valsi` requires a non-empty query.");
    }
    if let Some(section) = &input.section
        && section.trim().is_empty()
    {
        bail!("`--section` requires a non-empty reference.");
    }
    if let Some(example) = &input.example
        && example.trim().is_empty()
    {
        bail!("`--example` requires a non-empty reference.");
    }
    if !input.query.is_empty() && joined_query_text(&input.query).trim().is_empty() {
        bail!("cukta query text must be non-empty.");
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn cukta_request_from_input(input: &CuktaInput) -> Result<CuktaRequest> {
    if input.toc {
        return Ok(CuktaRequest::Toc);
    }
    if let Some(reference) = &input.section {
        return Ok(CuktaRequest::Section {
            reference: reference.trim().to_owned(),
        });
    }
    if let Some(reference) = &input.example {
        return Ok(CuktaRequest::Example {
            reference: reference.trim().to_owned(),
        });
    }
    if let Some(query) = &input.valsi {
        return Ok(CuktaRequest::Search {
            mode: CuktaSearchMode::Word,
            query: query.trim().to_owned(),
            count: input.count.unwrap_or(DEFAULT_CUKTA_CLI_RESULT_COUNT),
            targets: cukta_target_filter_from_input(input)?,
        });
    }
    if !input.query.is_empty() {
        return Ok(CuktaRequest::Search {
            mode: CuktaSearchMode::Meaning,
            query: joined_query_text(&input.query).trim().to_owned(),
            count: input.count.unwrap_or(DEFAULT_CUKTA_CLI_RESULT_COUNT),
            targets: cukta_target_filter_from_input(input)?,
        });
    }
    Ok(CuktaRequest::Section {
        reference: jbotci_cll::DEFAULT_CUKTA_SECTION_ID.to_owned(),
    })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn cukta_target_filter_from_input(input: &CuktaInput) -> Result<CuktaTargetFilter> {
    let mut explicit = input.target_sections || input.target_paragraphs || input.target_examples;
    let mut sections = input.target_sections;
    let mut paragraphs = input.target_paragraphs;
    let mut examples = input.target_examples;
    for raw_target in &input.targets {
        for target in raw_target.split(',') {
            match target.trim().to_ascii_lowercase().as_str() {
                "" => {}
                "section" | "sections" => {
                    explicit = true;
                    sections = true;
                }
                "paragraph" | "paragraphs" => {
                    explicit = true;
                    paragraphs = true;
                }
                "example" | "examples" => {
                    explicit = true;
                    examples = true;
                }
                other => {
                    bail!(
                        "Unknown cukta search target `{other}`. Use section, paragraph, or example."
                    );
                }
            }
        }
    }
    if !explicit {
        return Ok(CuktaTargetFilter::default());
    }
    if !(sections || paragraphs || examples) {
        bail!("Select at least one cukta search target.");
    }
    Ok(CuktaTargetFilter {
        sections,
        paragraphs,
        examples,
    })
}

#[requires(true)]
#[ensures(true)]
fn cukta_target_flags_present(input: &CuktaInput) -> bool {
    !input.targets.is_empty()
        || input.target_sections
        || input.target_paragraphs
        || input.target_examples
}

#[requires(true)]
#[ensures(true)]
fn joined_query_text(query: &[String]) -> String {
    query.join(" ")
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_jvozba<WOut: Write>(
    input: JvozbaInput,
    stdout: &mut WOut,
    color: bool,
) -> Result<CliStatus> {
    let mode = if input.cmevla {
        JvozbaMode::Cmevla
    } else {
        JvozbaMode::Lujvo
    };
    let result =
        build_best_jvozba_detailed(mode, jbotci_dictionary_data::english(), &input.sources)
            .map_err(|message| anyhow!(message))?;
    writeln!(stdout, "{}", render_jvozba_result(&result, color))?;
    Ok(CliStatus::Success)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn render_jvozba_result(result: &JvozbaBuildResult, color: bool) -> String {
    if !color || result.segments.is_empty() {
        return result.word.clone();
    }
    let mut rafsi_index = 0;
    let mut output = String::new();
    for segment in &result.segments {
        match segment.kind {
            JvozbaSegmentKind::Rafsi => {
                let segment_text = if rafsi_index % 2 == 0 {
                    green(&segment.text, true)
                } else {
                    magenta(&segment.text, true)
                };
                output.push_str(&segment_text);
                rafsi_index += 1;
            }
            JvozbaSegmentKind::Hyphen => output.push_str(&dark(&segment.text, true)),
        }
    }
    output
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_gimfihi<WOut: Write>(input: GimfihiInput, stdout: &mut WOut) -> Result<CliStatus> {
    let request = gimfihi_request_from_input(&input)?;
    let output = compose_gismu(jbotci_dictionary_data::english(), &request)
        .map_err(|error| anyhow!(error.to_string()))?;
    match input.format {
        GimfihiCliFormat::Table => writeln!(stdout, "{}", render_gimfihi_table(&output))?,
        GimfihiCliFormat::Json => {
            writeln!(
                stdout,
                "{}",
                serde_json::to_string_pretty(&output)
                    .context("failed to serialize gimfihi output")?
            )?;
        }
    }
    Ok(CliStatus::Success)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn gimfihi_request_from_input(input: &GimfihiInput) -> Result<GimfihiRequest> {
    let count = input.count.unwrap_or(GIMFIHI_DEFAULT_COUNT);
    if count == 0 {
        bail!("`--count` must be greater than 0");
    }
    if count > GIMFIHI_MAX_COUNT {
        bail!("`--count` must be at most {GIMFIHI_MAX_COUNT}");
    }
    let preset = input
        .preset
        .as_deref()
        .map(parse_preset)
        .transpose()
        .map_err(|error| anyhow!(error.to_string()))?;
    let sources = input.sources.clone();
    let shapes = if input.shapes.is_empty() {
        default_shapes()
    } else {
        input
            .shapes
            .iter()
            .map(|shape| parse_shape(shape).map_err(|error| anyhow!(error.to_string())))
            .collect::<Result<Vec<_>>>()?
    };
    Ok(GimfihiRequest {
        preset,
        sources,
        shapes,
        all_letters: input.all_letters,
        check_collisions: input.check_collisions.into(),
        show_collisions: input.show_collisions,
        require_free_short_rafsi: input.require_free_short_rafsi,
        count,
        highlight: input.highlight.clone(),
    })
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn render_gimfihi_table(output: &GimfihiOutput) -> String {
    if output.candidates.is_empty() {
        return "No gismu candidates matched the selected filters.".to_owned();
    }
    let mut lines = Vec::new();
    lines.push(format!(
        "winner: {}",
        output.winner.as_deref().unwrap_or("none")
    ));
    lines.push(format!(
        "candidates: {} shown of {} passing ({} valid)",
        output.candidates.len(),
        output.filtered_count,
        output.candidate_count
    ));
    lines.push("mark  gismu  score     rafsi".to_owned());
    for candidate in &output.candidates {
        lines.push(render_gimfihi_candidate_row(candidate));
    }
    lines.join("\n")
}

#[requires(!candidate.word.is_empty())]
#[ensures(!ret.is_empty())]
fn render_gimfihi_candidate_row(candidate: &GimfihiCandidate) -> String {
    let marker = if candidate.highlighted { "*" } else { " " };
    let collision = candidate
        .collision
        .as_ref()
        .map(|collision| format!("{} ", format_gimfihi_collision(collision)))
        .unwrap_or_default();
    format!(
        "{marker}     {:<5}  {:<8} {collision}{}",
        candidate.word,
        format_gimfihi_score(candidate.score),
        format_gimfihi_rafsi(candidate)
    )
}

/// Render a candidate's gismu-level collision with an existing word.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_gimfihi_collision(collision: &GismuCollision) -> String {
    match collision.kind {
        CollisionKind::Identical => {
            format!("[= existing {}]", collision.existing_word_type.as_str())
        }
        CollisionKind::FinalVowel => format!("[~ {}: final vowel]", collision.existing_word),
        CollisionKind::SimilarConsonant => {
            format!("[~ {}: similar consonant]", collision.existing_word)
        }
    }
}

#[requires(score.is_finite())]
#[ensures(!ret.is_empty())]
fn format_gimfihi_score(score: f64) -> String {
    trim_float(&format!("{score:.6}"))
}

#[requires(!candidate.word.is_empty())]
#[ensures(true)]
fn format_gimfihi_rafsi(candidate: &GimfihiCandidate) -> String {
    if candidate.rafsi().is_empty() {
        return String::new();
    }
    candidate
        .rafsi()
        .iter()
        .map(|rafsi| {
            let status = match rafsi.availability {
                RafsiAvailability::Free => "free".to_owned(),
                RafsiAvailability::OfficialTaken => format!(
                    "official-taken{}",
                    format_taken_rafsi_sources(&rafsi.taken_by)
                ),
                RafsiAvailability::ExperimentalTaken => format!(
                    "experimental-taken{}",
                    format_taken_rafsi_sources(&rafsi.taken_by)
                ),
            };
            format!("{}:{status}", rafsi.form)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[requires(true)]
#[ensures(true)]
fn format_taken_rafsi_sources(sources: &[String]) -> String {
    if sources.is_empty() {
        String::new()
    } else {
        format!("({})", sources.join("/"))
    }
}

#[requires(!value.is_empty())]
#[ensures(!ret.is_empty())]
fn trim_float(value: &str) -> String {
    let trimmed = value.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_vlacku_input(input: &VlackuInput) -> Result<()> {
    if input.count == Some(0) {
        bail!("`--count` must be greater than 0");
    }
    if let Some(min_similarity) = input.min_similarity {
        if !(0.0..=100.0).contains(&min_similarity) {
            bail!("`--min-similarity` must be between 0 and 100");
        }
    }
    if input.requests.is_empty() {
        if input.query.is_empty() {
            bail!("No query provided for vlacku. Use --valsi, --rafsi, --lujvo, or --sound.");
        }
        let _ = joined_query_text(&input.query);
    }
    if !input.query.is_empty() && !input.requests.is_empty() {
        bail!(
            "Do not pass positional query text when using --valsi, --rafsi, --lujvo, or --sound."
        );
    }
    let sound_count = input
        .requests
        .iter()
        .filter(|request| matches!(request.as_data(), data!(VlackuRequest::Sound(_))))
        .count();
    if sound_count > 1 {
        bail!("`--sound` may be specified only once");
    }
    if sound_count == 1 && input.requests.len() > 1 {
        bail!("`--sound` cannot be combined with --valsi, --rafsi, or --lujvo");
    }
    if input.min_similarity.is_some() && sound_count != 1 && !input.requests.is_empty() {
        bail!("`--min-similarity` is only valid with `--sound` or semantic search");
    }
    for request in &input.requests {
        validate_vlacku_request_value(request)?;
    }
    let _ = parse_vlacku_word_types(&input.word_types)?;
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_vlacku_request_value(request: &VlackuRequest) -> Result<()> {
    let (flag, value) = match request.as_data() {
        data!(VlackuRequest::Valsi(value)) => ("--valsi", value),
        data!(VlackuRequest::Rafsi(value)) => ("--rafsi", value),
        data!(VlackuRequest::Lujvo(value)) => ("--lujvo", value),
        data!(VlackuRequest::Sound(value)) => ("--sound", value),
        data!(VlackuRequest::Meaning(value)) => ("semantic query", value),
    };
    if value.trim().is_empty() {
        bail!("{flag} requires a non-empty value");
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn vlacku_search_options(input: &VlackuInput) -> Result<VlackuSearchOptions> {
    let word_types = parse_vlacku_word_types(&input.word_types)?;
    Ok(new!(VlackuSearchOptions {
        count: input.count.unwrap_or(DEFAULT_VLACKU_RESULT_COUNT),
        word_types,
        min_votes: input.min_votes,
        min_similarity: input.min_similarity,
        decompose_lujvo: input.decompose_lujvo,
    }))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn parse_vlacku_word_types(raw_values: &[String]) -> Result<Vec<WordTypeFilter>> {
    let mut values = Vec::new();
    for raw_value in raw_values {
        for piece in raw_value.split(',') {
            let normalized = normalize_word_type_filter(piece);
            if normalized.is_empty() {
                continue;
            }
            let Some(filter) = parse_word_type_filter(&normalized) else {
                bail!(
                    "Unknown `--word-type` value: {normalized}. Use gismu, lujvo, cmavo, cmevla, fu'ivla, or brivla."
                );
            };
            if !is_valid_vlacku_word_type_filter(filter) {
                bail!(
                    "Unknown `--word-type` value: {normalized}. Use gismu, lujvo, cmavo, cmevla, fu'ivla, or brivla."
                );
            }
            if !values.contains(&filter) {
                values.push(filter);
            }
        }
    }
    if !raw_values.is_empty() && values.is_empty() {
        bail!("`--word-type` requires at least one non-empty type");
    }
    Ok(values)
}

#[requires(true)]
#[ensures(true)]
fn is_valid_vlacku_word_type_filter(value: WordTypeFilter) -> bool {
    matches!(
        value,
        WordTypeFilter::Gismu
            | WordTypeFilter::Lujvo
            | WordTypeFilter::Cmavo
            | WordTypeFilter::Cmevla
            | WordTypeFilter::Fuivla
            | WordTypeFilter::Brivla
    )
}

#[requires(true)]
#[ensures(true)]
fn cli_status_from_vlacku_outcome(outcome: VlackuOutcome) -> CliStatus {
    match outcome {
        VlackuOutcome::Found => CliStatus::Success,
        VlackuOutcome::ValidMissing => CliStatus::ValidMissing,
        VlackuOutcome::Invalid => CliStatus::InvalidInput,
    }
}

#[invariant(self.output_terminal_width.is_none_or(|width| width > 0))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct VlackuRenderOptions {
    color: bool,
    glyphs: GlyphStyle,
    output_terminal_width: Option<usize>,
    sumti_places: CliSumtiPlaces,
    show_etymology: bool,
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn render_vlacku_output(output: &VlackuSearchOutput, color: bool, glyphs: GlyphStyle) -> String {
    render_vlacku_output_with_options(
        output,
        new!(VlackuRenderOptions {
            color,
            glyphs,
            output_terminal_width: None,
            sumti_places: CliSumtiPlaces::Index,
            show_etymology: false,
        }),
    )
}

#[requires(output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(!ret.is_empty())]
fn render_vlacku_output_with_width(
    output: &VlackuSearchOutput,
    color: bool,
    glyphs: GlyphStyle,
    output_terminal_width: Option<usize>,
) -> String {
    render_vlacku_output_with_options(
        output,
        new!(VlackuRenderOptions {
            color,
            glyphs,
            output_terminal_width,
            sumti_places: CliSumtiPlaces::Index,
            show_etymology: false,
        }),
    )
}

#[requires(options.output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(!ret.is_empty())]
fn render_vlacku_output_with_options(
    output: &VlackuSearchOutput,
    options: VlackuRenderOptions,
) -> String {
    if output.cards.is_empty() {
        return "No matches found.\n".to_owned();
    }
    let mut rendered = String::new();
    for (index, card) in output.cards.iter().enumerate() {
        rendered.push_str(&render_vlacku_card(index + 1, card, &options));
        rendered.push('\n');
    }
    rendered
}

#[requires(index > 0)]
#[requires(options.output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(!ret.is_empty())]
fn render_vlacku_card(index: usize, card: &VlackuCard, options: &VlackuRenderOptions) -> String {
    let mut lines = Vec::new();
    let mut header = String::new();
    header.push_str(&dark(&format!("{index}."), options.color));
    header.push(' ');
    header.push_str(&yellow_underlined(&card.word, options.color));
    if let Some(author) = &card.author {
        header.push_str(&dark(" | ", options.color));
        header.push_str(&dark("by: ", options.color));
        header.push_str(&author.username);
    }
    header.push_str(&dark(" | ", options.color));
    header.push_str(&blue(&vlacku_header_type(card), options.color));
    if let Some(similarity) = card.similarity {
        header.push_str(&dark(" | ", options.color));
        header.push_str(&dark("similarity: ", options.color));
        header.push_str(&magenta(
            &format_similarity_percent(similarity),
            options.color,
        ));
    }
    if let Some(votes) = card.votes {
        header.push_str(&dark(" | ", options.color));
        header.push_str(&dark("votes: ", options.color));
        header.push_str(&green(
            &format_vlacku_votes(votes, card.is_official, options.glyphs),
            options.color,
        ));
    }
    lines.push(header);

    if !card.rafsi.is_empty() {
        lines.push(format!(
            "  {}{}",
            dark("rafsi: ", options.color),
            card.rafsi
                .iter()
                .map(|rafsi| red(rafsi, options.color))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    if !card.decomposition.is_empty() {
        lines.push(format!(
            "  {}{}",
            dark("decomposition: ", options.color),
            render_vlacku_decomposition(&card.decomposition, options.color, options.glyphs)
        ));
    }
    if !card.glosses.is_empty() {
        lines.push(format!("  {}", dark("glosses:", options.color)));
        push_vlacku_detail_lines(&mut lines, &card.glosses.join("; "), options);
    }
    if !card.definition.trim().is_empty() {
        lines.push(format!("  {}", dark("definitions:", options.color)));
        for line in card.definition.lines() {
            push_vlacku_detail_lines(&mut lines, line, options);
        }
    }
    if !card.notes.trim().is_empty() {
        lines.push(format!("  {}", dark("notes:", options.color)));
        for line in card.notes.lines() {
            push_vlacku_detail_lines(&mut lines, line, options);
        }
    }
    if options.show_etymology {
        if let Some(etymology) = card
            .etymology
            .as_deref()
            .filter(|etymology| !etymology.trim().is_empty())
        {
            lines.push(format!("  {}", dark("etymology:", options.color)));
            for line in etymology.lines() {
                push_vlacku_detail_lines(&mut lines, line, options);
            }
        }
    }
    lines.join("\n") + "\n"
}

#[requires(options.output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(true)]
fn push_vlacku_detail_lines(lines: &mut Vec<String>, text: &str, options: &VlackuRenderOptions) {
    let rendered_text = vlacku_detail_text_for_sumti_places(text, options);
    for line in wrap_vlacku_detail_line(&rendered_text, options.output_terminal_width) {
        lines.push(format!(
            "{VLACKU_DETAIL_INDENT}{}",
            render_vlacku_rich_text(&line, options)
        ));
    }
}

#[requires(true)]
#[ensures(true)]
fn vlacku_detail_text_for_sumti_places(text: &str, options: &VlackuRenderOptions) -> String {
    match options.sumti_places {
        CliSumtiPlaces::Raw => text.to_owned(),
        CliSumtiPlaces::Index => {
            format_definition_or_notes_line_with_indexed_places(text, options.glyphs)
        }
    }
}

#[requires(output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(!ret.is_empty())]
fn wrap_vlacku_detail_line(text: &str, output_terminal_width: Option<usize>) -> Vec<String> {
    let Some(output_terminal_width) = output_terminal_width else {
        return vec![text.to_owned()];
    };
    let wrap_width = output_terminal_width
        .saturating_sub(UnicodeWidthStr::width(VLACKU_DETAIL_INDENT))
        .max(1);
    if UnicodeWidthStr::width(text) <= wrap_width {
        return vec![text.to_owned()];
    }
    let atoms = vlacku_wrap_atoms(text);
    if atoms.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;
    for atom in atoms {
        let atom_width = UnicodeWidthStr::width(atom.as_str());
        if current.is_empty() {
            current_width = atom_width;
            current = atom;
        } else if current_width + 1 + atom_width <= wrap_width {
            current.push(' ');
            current.push_str(&atom);
            current_width += 1 + atom_width;
        } else {
            lines.push(current);
            current_width = atom_width;
            current = atom;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[requires(true)]
#[ensures(input.trim().is_empty() -> ret.is_empty())]
fn vlacku_wrap_atoms(input: &str) -> Vec<String> {
    let mut atoms = Vec::new();
    let mut remaining = input.trim();
    while !remaining.is_empty() {
        if let Some(after_open) = remaining.strip_prefix('$') {
            if let Some(close_index) = after_open.find('$') {
                let mut atom_end = close_index + 2;
                let trailing_text = &remaining[atom_end..];
                let trailing_end = trailing_text
                    .find(char::is_whitespace)
                    .unwrap_or(trailing_text.len());
                atom_end += trailing_end;
                atoms.push(remaining[..atom_end].to_owned());
                remaining = remaining[atom_end..].trim_start();
                continue;
            }
        }
        let atom_end = remaining
            .find(char::is_whitespace)
            .unwrap_or(remaining.len());
        atoms.push(remaining[..atom_end].to_owned());
        remaining = remaining[atom_end..].trim_start();
    }
    atoms
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vlacku_header_type(card: &VlackuCard) -> String {
    let normalized = normalize_word_type_filter(&card.word_type);
    if normalized.starts_with("cmavo") {
        if let Some(selmaho) = &card.selmaho {
            if !selmaho.trim().is_empty() {
                return format!("cmavo: {selmaho}");
            }
        }
    }
    card.word_type.clone()
}

#[requires(true)]
#[ensures(ret.ends_with('%'))]
fn format_similarity_percent(value: f32) -> String {
    format!("{}%", (value * 100.0).round() as i32)
}

#[requires(true)]
#[ensures(glyphs == GlyphStyle::Ascii && is_official -> ret == "official")]
fn format_vlacku_votes(value: i32, is_official: bool, glyphs: GlyphStyle) -> String {
    if glyphs == GlyphStyle::Ascii && is_official {
        "official".to_owned()
    } else {
        format_vote_display(value, is_official)
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_decomposition(
    pieces: &[VlackuCompositionPiece],
    color: bool,
    glyphs: GlyphStyle,
) -> String {
    let separator = dark(lujvo_separator(glyphs), color);
    pieces
        .iter()
        .map(|piece| render_vlacku_decomposition_piece(piece, color, glyphs))
        .collect::<Vec<_>>()
        .join(&separator)
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_decomposition_piece(
    piece: &VlackuCompositionPiece,
    color: bool,
    glyphs: GlyphStyle,
) -> String {
    let phoneme_options = phoneme_render_options(None, None, glyphs);
    let surface = Phonemes::from_canonical(piece.surface.clone())
        .map(|phonemes| phonemes.render(phoneme_options))
        .unwrap_or_else(|_| piece.surface.clone());
    match piece.kind {
        VlackuCompositionKind::Rafsi => red(&surface, color),
        VlackuCompositionKind::Hyphen => dark(&surface, color),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn lujvo_separator(glyphs: GlyphStyle) -> &'static str {
    match glyphs {
        GlyphStyle::Unicode => "·",
        GlyphStyle::Ascii => "~",
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_rich_text(input: &str, options: &VlackuRenderOptions) -> String {
    let mut output = String::new();
    let mut remaining = input;
    while !remaining.is_empty() {
        let Some(open_index) = remaining.find('$') else {
            output.push_str(&render_vlacku_word_links(remaining, options));
            break;
        };
        let before = &remaining[..open_index];
        let after_open = &remaining[open_index + 1..];
        let Some(close_index) = after_open.find('$') else {
            output.push_str(&render_vlacku_word_links(remaining, options));
            break;
        };
        output.push_str(&render_vlacku_word_links(before, options));
        let math_body = &after_open[..close_index];
        output.push_str(&render_vlacku_raw_place_span(math_body, options.color));
        remaining = &after_open[close_index + 1..];
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_raw_place_span(input: &str, color: bool) -> String {
    let mut output = String::new();
    output.push_str(&dark("$", color));
    let mut remaining = input;
    while !remaining.is_empty() {
        let Some(equals_index) = remaining.find('=') else {
            output.push_str(&cyan(remaining, color));
            break;
        };
        output.push_str(&cyan(&remaining[..equals_index], color));
        output.push_str(&dark("=", color));
        remaining = &remaining[equals_index + 1..];
    }
    output.push_str(&dark("$", color));
    output
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_word_links(input: &str, options: &VlackuRenderOptions) -> String {
    let mut output = String::new();
    let mut remaining = input;
    while !remaining.is_empty() {
        let Some(open_index) = remaining.find('{') else {
            output.push_str(&render_vlacku_plain_or_indexed_places(remaining, options));
            break;
        };
        let before = &remaining[..open_index];
        let after_open = &remaining[open_index + 1..];
        let Some(close_index) = after_open.find('}') else {
            output.push_str(&render_vlacku_plain_or_indexed_places(remaining, options));
            break;
        };
        output.push_str(&render_vlacku_plain_or_indexed_places(before, options));
        let inside = &after_open[..close_index];
        let link_value = inside.trim();
        if is_vlacku_word_link(link_value) {
            output.push_str(&dark("{", options.color));
            output.push_str(&yellow(link_value, options.color));
            output.push_str(&dark("}", options.color));
        } else {
            output.push_str(&light(&format!("{{{inside}}}"), options.color));
        }
        remaining = &after_open[close_index + 1..];
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_plain_or_indexed_places(input: &str, options: &VlackuRenderOptions) -> String {
    if options.sumti_places == CliSumtiPlaces::Raw {
        return light(input, options.color);
    }

    let mut output = String::new();
    let mut remaining = input;
    while !remaining.is_empty() {
        let Some(open_index) = remaining.find(options.glyphs.slot_open()) else {
            output.push_str(&light(remaining, options.color));
            break;
        };
        output.push_str(&light(&remaining[..open_index], options.color));
        let after_open = &remaining[open_index + options.glyphs.slot_open().len()..];
        let Some(close_index) = after_open.find(options.glyphs.slot_close()) else {
            output.push_str(&light(&remaining[open_index..], options.color));
            break;
        };
        let place_index = &after_open[..close_index];
        if !place_index.is_empty()
            && place_index
                .chars()
                .all(|character| character.is_ascii_digit())
        {
            output.push_str(&dark(options.glyphs.slot_open(), options.color));
            output.push_str(&cyan(place_index, options.color));
            output.push_str(&dark(options.glyphs.slot_close(), options.color));
            remaining = &after_open[close_index + options.glyphs.slot_close().len()..];
        } else {
            output.push_str(&light(options.glyphs.slot_open(), options.color));
            remaining = after_open;
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn is_vlacku_word_link(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_whitespace)
}

#[allow(clippy::too_many_arguments)]
#[requires(diagnostic_terminal_width > 0)]
#[requires(trace.limit > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_gentufa<WOut: Write, WErr: Write>(
    input: GentufaInput,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_detail: DiagnosticDetailMode,
    glyphs: GlyphStyle,
    diagnostic_terminal_width: usize,
    trace: CliTraceConfig,
    stdin_text: Option<&str>,
) -> Result<CliStatus> {
    let output_file = input.output_file.clone();
    let rendered = render_gentufa(
        input,
        color_policy,
        diagnostic_detail,
        glyphs,
        diagnostic_terminal_width,
        trace,
        stdin_text,
    )?;
    stderr.write_all(rendered.stderr.as_bytes())?;
    if rendered.status == CliStatus::Success
        && let Some(path) = output_file.as_ref()
    {
        fs::write(path, &rendered.stdout)
            .with_context(|| format!("failed to write gentufa output to `{}`", path.display()))?;
    } else {
        stdout.write_all(&rendered.stdout)?;
    }
    Ok(rendered.status)
}

#[requires(diagnostic_terminal_width > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_tersmu<WOut: Write, WErr: Write>(
    input: TersmuInput,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_detail: DiagnosticDetailMode,
    glyphs: GlyphStyle,
    diagnostic_terminal_width: usize,
    stdin_text: Option<&str>,
) -> Result<CliStatus> {
    let rendered = render_tersmu(
        input,
        color_policy,
        diagnostic_detail,
        glyphs,
        diagnostic_terminal_width,
        stdin_text,
    )?;
    stderr.write_all(rendered.stderr.as_bytes())?;
    stdout.write_all(&rendered.stdout)?;
    Ok(rendered.status)
}

#[cfg(feature = "grammar-debug")]
#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_gerna<WOut: Write>(input: GernaInput, stdout: &mut WOut) -> Result<CliStatus> {
    let output_file = input.output_file.clone();
    let rendered = render_gerna(input)?;
    write_gerna_output(stdout, output_file.as_ref(), &rendered)?;
    Ok(CliStatus::Success)
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
    glyphs: GlyphStyle,
    diagnostic_terminal_width: usize,
    trace: CliTraceConfig,
    stdin_text: Option<&str>,
) -> Result<GentufaRendered> {
    normalize_trace_text_input(&mut input.trace, &input.file, &mut input.text);
    validate_gentufa_options(&input, glyphs)?;
    let morphology_trace_options = trace_options(&input.trace, trace.phase, trace.limit)?;
    let syntax_trace_options = trace_options(&input.trace, trace.phase, trace.limit)?;
    let source_label = input_source_label(input.file.as_ref(), input.text.is_empty());
    let text = input.read_text_with_stdin(stdin_text)?;
    let dialect = input.dialect_definition()?;
    let morphology_options = MorphologyOptions::default()
        .with_dialect_definition(&dialect)
        .with_trace_options(morphology_trace_options);
    let morphology_attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
        &text,
        &morphology_options,
        Some(SourceId(source_label.clone())),
    );
    let morphology_attempt = morphology_attempt.into_data();
    let morphology_trace_stderr = render_cli_trace(
        morphology_attempt.trace.as_ref(),
        color_policy.stderr,
        diagnostic_terminal_width,
    );
    let morphology_diagnostics = morphology_warning_diagnostics(
        &morphology_attempt.warnings,
        Some(SourceId(source_label.clone())),
        &text,
    );
    let words = match morphology_attempt.result {
        Ok(words) => words,
        Err(error) => {
            let mut diagnostics = morphology_diagnostics;
            diagnostics.push(error.to_diagnostic(Some(SourceId(source_label.clone())), &text));
            let mut stderr = morphology_trace_stderr;
            stderr.push_str(&render_source_diagnostics(
                &source_label,
                &text,
                &diagnostics,
                color_policy.stderr,
                diagnostic_detail,
                glyphs,
                diagnostic_terminal_width,
            )?);
            return Ok(new!(GentufaRendered {
                status: CliStatus::Failure,
                stdout: Vec::new(),
                stderr,
            }));
        }
    };
    let parse_options = ParseOptions::default()
        .with_dialect_definition(&dialect)
        .with_trace_options(syntax_trace_options)
        .with_error_context_depth(input.error_context);
    let generated_model = match parse_syntax_tree_generated_model_with_source_and_options(
        &words,
        &text,
        &parse_options,
    ) {
        Ok(parsed) => parsed,
        Err(error) => {
            let mut diagnostics = morphology_diagnostics;
            diagnostics.push(error.to_diagnostic(Some(SourceId(source_label.clone())), &text));
            let mut stderr = morphology_trace_stderr;
            stderr.push_str(&render_source_diagnostics(
                &source_label,
                &text,
                &diagnostics,
                color_policy.stderr,
                diagnostic_detail,
                glyphs,
                diagnostic_terminal_width,
            )?);
            return Ok(new!(GentufaRendered {
                status: CliStatus::Failure,
                stdout: Vec::new(),
                stderr,
            }));
        }
    };
    let diagnostics = morphology_diagnostics;
    let mut stderr = morphology_trace_stderr;
    stderr.push_str(&render_source_diagnostics(
        &source_label,
        &text,
        &diagnostics,
        color_policy.stderr,
        diagnostic_detail,
        glyphs,
        diagnostic_terminal_width,
    )?);
    let phoneme_options = phoneme_render_options(input.mark_stress, input.mark_glides, glyphs);
    let mut stdout = String::new();
    if input.show_defs {
        let cards =
            dictionary_cards_for_word_likes(jbotci_dictionary_data::english(), words.as_slice());
        if !cards.is_empty() {
            stdout.push_str(&render_vlacku_output_with_options(
                &VlackuSearchOutput {
                    cards,
                    outcome: VlackuOutcome::Found,
                    diagnostics: Vec::new(),
                },
                new!(VlackuRenderOptions {
                    color: color_policy.stdout,
                    glyphs,
                    output_terminal_width: None,
                    sumti_places: CliSumtiPlaces::Index,
                    show_etymology: false,
                }),
            ));
        }
    }
    match input.format {
        GentufaFormat::Blocks => {
            let output_type = resolve_gentufa_blocks_output_type(&input)?;
            let stdout = render_gentufa_generated_blocks_output(
                &generated_model,
                &text,
                words.as_slice(),
                phoneme_options,
                output_type,
            )?;
            return Ok(new!(GentufaRendered {
                status: CliStatus::Success,
                stdout,
                stderr,
            }));
        }
        GentufaFormat::Brackets => {
            let rendered = pretty_generated_model_brackets_with_options(
                &generated_model,
                &text,
                BracketRenderOptions {
                    color: color_policy.stdout,
                    phonemes: phoneme_options,
                    script: LojbanScript::Latin,
                    glyphs,
                    decompose_lujvo: input.decompose_lujvo,
                    insert_hair_space: false,
                    show_elided: false,
                },
            )?;
            stdout.push_str(&rendered);
            stdout.push('\n');
        }
        GentufaFormat::Raw => {
            stdout.push_str(&debug_output_string(&generated_model, input.indent));
        }
        GentufaFormat::Tree => {
            let tree_options = TreeRenderOptions {
                color: color_policy.stdout,
                indent: input.indent.unwrap_or(2),
                phonemes: phoneme_options,
                glyphs,
                show_spans: input.show_spans,
                show_refs: input.show_refs,
                decompose_lujvo: input.decompose_lujvo,
                show_elided: false,
            };
            let rendered =
                pretty_generated_model_tree_with_options(&generated_model, &text, tree_options)?;
            stdout.push_str(&rendered);
            stdout.push('\n');
        }
        GentufaFormat::Json => {
            let rendered = compact_generated_model_json_string_with_options(
                &generated_model,
                JsonRenderOptions {
                    indent: input.indent.unwrap_or(2),
                    phonemes: phoneme_options,
                    show_elided: false,
                    color: color_policy.stdout,
                },
            )?;
            stdout.push_str(&rendered);
            stdout.push('\n');
        }
    }
    let stdout = stdout.into_bytes();
    Ok(new!(GentufaRendered {
        status: CliStatus::Success,
        stdout,
        stderr,
    }))
}

#[requires(diagnostic_terminal_width > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn render_tersmu(
    input: TersmuInput,
    color_policy: CliColorPolicy,
    diagnostic_detail: DiagnosticDetailMode,
    glyphs: GlyphStyle,
    diagnostic_terminal_width: usize,
    stdin_text: Option<&str>,
) -> Result<TersmuRendered> {
    let morphology_trace_options =
        trace_options(&input.trace, TracePhase::Syntax, DEFAULT_TRACE_LIMIT)?;
    let syntax_trace_options =
        trace_options(&input.trace, TracePhase::Syntax, DEFAULT_TRACE_LIMIT)?;
    let source_label = input_source_label(input.file.as_ref(), input.text.is_empty());
    let text = input.read_text_with_stdin(stdin_text)?;
    let dialect = input.dialect_definition()?;
    let morphology_options = MorphologyOptions::default()
        .with_dialect_definition(&dialect)
        .with_trace_options(morphology_trace_options);
    let morphology_attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
        &text,
        &morphology_options,
        Some(SourceId(source_label.clone())),
    );
    let morphology_attempt = morphology_attempt.into_data();
    let morphology_trace_stderr = render_cli_trace(
        morphology_attempt.trace.as_ref(),
        color_policy.stderr,
        diagnostic_terminal_width,
    );
    let morphology_diagnostics = morphology_warning_diagnostics(
        &morphology_attempt.warnings,
        Some(SourceId(source_label.clone())),
        &text,
    );
    let words = match morphology_attempt.result {
        Ok(words) => words,
        Err(error) => {
            let mut diagnostics = morphology_diagnostics;
            diagnostics.push(error.to_diagnostic(Some(SourceId(source_label.clone())), &text));
            let mut stderr = morphology_trace_stderr;
            stderr.push_str(&render_source_diagnostics(
                &source_label,
                &text,
                &diagnostics,
                color_policy.stderr,
                diagnostic_detail,
                glyphs,
                diagnostic_terminal_width,
            )?);
            return Ok(new!(TersmuRendered {
                status: CliStatus::Failure,
                stdout: Vec::new(),
                stderr,
            }));
        }
    };
    let parse_options = ParseOptions::default()
        .with_dialect_definition(&dialect)
        .with_trace_options(syntax_trace_options);
    let parsed = parse_syntax_tree_generated_model_with_source_and_options_attempt(
        &words,
        &text,
        &parse_options,
    );
    let trace_stderr = render_cli_trace(
        parsed.trace.as_ref(),
        color_policy.stderr,
        diagnostic_terminal_width,
    );
    let parsed = match parsed.result {
        Ok(parsed) => parsed,
        Err(error) => {
            let mut diagnostics = morphology_diagnostics;
            diagnostics.push(error.to_diagnostic(Some(SourceId(source_label.clone())), &text));
            let mut stderr = morphology_trace_stderr;
            stderr.push_str(&trace_stderr);
            stderr.push_str(&render_source_diagnostics(
                &source_label,
                &text,
                &diagnostics,
                color_policy.stderr,
                diagnostic_detail,
                glyphs,
                diagnostic_terminal_width,
            )?);
            return Ok(new!(TersmuRendered {
                status: CliStatus::Failure,
                stdout: Vec::new(),
                stderr,
            }));
        }
    };
    let mut diagnostics = morphology_diagnostics;
    diagnostics.extend(
        parsed
            .warnings
            .iter()
            .map(|warning| warning.to_diagnostic(Some(SourceId(source_label.clone())), &text)),
    );
    let mut stderr = morphology_trace_stderr;
    stderr.push_str(&trace_stderr);
    stderr.push_str(&render_source_diagnostics(
        &source_label,
        &text,
        &diagnostics,
        color_policy.stderr,
        diagnostic_detail,
        glyphs,
        diagnostic_terminal_width,
    )?);
    let graph = match build_generated_semantic_graph_with_dictionary_and_options(
        &parsed.parse_tree,
        SemanticBuildOptions {
            source_text: Some(&text),
            story_time: input.story_time,
        },
        jbotci_dictionary_data::english(),
    ) {
        Ok(graph) => graph,
        Err(error) => {
            stderr.push_str(&format!("semantic error: {error}\n"));
            return Ok(new!(TersmuRendered {
                status: CliStatus::Failure,
                stdout: Vec::new(),
                stderr,
            }));
        }
    };
    let mut rendered = match input.format {
        TersmuFormat::Json => json_string_with_options(
            &graph,
            JsonRenderOptions {
                indent: input.indent.unwrap_or(0),
                color: color_policy.stdout,
                ..JsonRenderOptions::default()
            },
        )?,
    };
    rendered.push('\n');
    Ok(new!(TersmuRendered {
        status: CliStatus::Success,
        stdout: rendered.into_bytes(),
        stderr,
    }))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|output| !output.is_empty()) || ret.is_err())]
fn render_gentufa_generated_blocks_output(
    syntax: &jbotci_syntax::generated_model::TextSyntax,
    source: &str,
    words: &[WordLike],
    phoneme_options: PhonemeRenderOptions,
    output_type: GentufaImageOutputType,
) -> Result<Vec<u8>> {
    let block_options = GentufaBlockOptions {
        script: GentufaScript::Latin,
        show_elided: false,
        phonemes: phoneme_options,
    };
    let annotations = gentufa_block_annotations(words);
    let reference_display = generated_reference_display(
        syntax,
        source,
        TreeRenderOptions {
            color: false,
            indent: 2,
            phonemes: phoneme_options,
            glyphs: GlyphStyle::Unicode,
            show_spans: false,
            show_refs: true,
            decompose_lujvo: false,
            show_elided: false,
        },
    )?;
    let layout = generated_model_blocks_layout_with_references(
        syntax,
        source,
        Some(&reference_display.analysis.syntax_index),
        Some(&reference_display.references),
        &annotations,
        &block_options,
    );
    let svg_options = GentufaSvgOptions {
        show_glosses: false,
        script: GentufaScript::Latin,
        title: "jbotci gentufa generated syntax".to_owned(),
    };
    let fonts = EmbeddedGentufaFonts::get();
    match output_type {
        GentufaImageOutputType::Svg => {
            Ok(render_gentufa_blocks_svg(&layout, &svg_options, fonts)?.into_bytes())
        }
        GentufaImageOutputType::Png => Ok(render_gentufa_blocks_png(
            &layout,
            &GentufaPngOptions::default().with_data(data! { svg: svg_options }),
            fonts,
        )?),
    }
}

#[requires(true)]
#[ensures(true)]
fn gentufa_block_annotations(words: &[WordLike]) -> Vec<GentufaBlockAnnotation<()>> {
    dictionary_matches_for_word_likes(jbotci_dictionary_data::english(), words)
        .into_iter()
        .map(|parsed_match| {
            let parsed_match = parsed_match.into_data();
            let first = parsed_match.cards.first();
            GentufaBlockAnnotation {
                range: new!(WebSourceRange {
                    byte_start: parsed_match.byte_start,
                    byte_end: parsed_match.byte_end,
                    char_start: parsed_match.char_start,
                    char_end: parsed_match.char_end,
                }),
                text: Some(parsed_match.lookup_text),
                glosses: first.map(|card| card.glosses.clone()).unwrap_or_default(),
                definition: first
                    .map(|card| card.definition.trim().to_owned())
                    .filter(|definition| !definition.is_empty()),
                tooltip: None,
            }
        })
        .collect()
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

#[allow(clippy::too_many_arguments)]
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
    glyphs: GlyphStyle,
    diagnostic_terminal_width: usize,
) -> Result<()> {
    let rendered = render_source_diagnostics(
        source_label,
        source,
        diagnostics,
        color_enabled,
        diagnostic_detail,
        glyphs,
        diagnostic_terminal_width,
    )?;
    stderr.write_all(rendered.as_bytes())?;
    Ok(())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vlatai_source_label(index: usize) -> String {
    format!("<arg:{}>", index + 1)
}

#[requires(true)]
#[ensures(true)]
fn render_vlatai_text(
    analyses: &[ValsiAnalysis],
    phoneme_options: PhonemeRenderOptions,
    color_enabled: bool,
    diagnostic_detail: DiagnosticDetailMode,
    glyphs: GlyphStyle,
    diagnostic_terminal_width: usize,
) -> Result<String> {
    let mut out = String::new();
    for (index, analysis) in analyses.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let source_label = vlatai_source_label(index);
        out.push_str(&format!("valsi: {}\n", analysis.input));
        out.push_str(&format!("status: {}\n", vlatai_status(analysis)));
        let diagnostics = vlatai_diagnostics(analysis, Some(SourceId(source_label.clone())))?;
        out.push_str(&render_source_diagnostics(
            &source_label,
            &analysis.input,
            &diagnostics,
            color_enabled,
            diagnostic_detail,
            glyphs,
            diagnostic_terminal_width,
        )?);
        match analysis.result.status {
            ValsiAnalysisStatus::Valid => {
                let classification = analysis
                    .result
                    .classification
                    .as_ref()
                    .expect("valid vlatai result carries classification");
                render_vlatai_classification_text(&mut out, classification, phoneme_options);
            }
            ValsiAnalysisStatus::NotSingleWord => {
                let rendered = pretty_morphology_brackets_with_options(
                    &analysis.result.words,
                    &analysis.input,
                    BracketRenderOptions {
                        color: color_enabled,
                        phonemes: phoneme_options,
                        script: LojbanScript::Latin,
                        glyphs,
                        decompose_lujvo: true,
                        insert_hair_space: false,
                        show_elided: false,
                    },
                )?;
                out.push_str(&format!("words: {rendered}\n"));
            }
            ValsiAnalysisStatus::Invalid => {}
        }
    }
    Ok(out)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
fn render_vlatai_json(
    analyses: &[ValsiAnalysis],
    phoneme_options: PhonemeRenderOptions,
    indent: usize,
    color: bool,
) -> Result<String> {
    let reports = analyses
        .iter()
        .enumerate()
        .map(|(index, analysis)| vlatai_json_value(index, analysis, phoneme_options))
        .collect::<Result<Vec<_>>>()?;
    let value = serde_json::Value::Array(reports);
    Ok(render_json_value_with_options(
        &value,
        JsonRenderOptions {
            indent,
            color,
            ..JsonRenderOptions::default()
        },
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok() || ret.is_err())]
fn vlatai_json_value(
    index: usize,
    analysis: &ValsiAnalysis,
    phoneme_options: PhonemeRenderOptions,
) -> Result<serde_json::Value> {
    let diagnostics = vlatai_diagnostics(analysis, Some(SourceId(vlatai_source_label(index))))?;
    let mut value = serde_json::json!({
        "input": analysis.input,
        "status": vlatai_status(analysis),
        "diagnostics": diagnostics,
    });
    match analysis.result.status {
        ValsiAnalysisStatus::Valid => {
            let word = analysis
                .result
                .word
                .as_ref()
                .expect("valid vlatai result carries word");
            let classification = analysis
                .result
                .classification
                .as_ref()
                .expect("valid vlatai result carries classification");
            value["classification"] = vlatai_classification_json(classification, phoneme_options);
            value["word"] = compact_morphology_json_value(std::slice::from_ref(word))?;
        }
        ValsiAnalysisStatus::Invalid => {}
        ValsiAnalysisStatus::NotSingleWord => {
            value["words"] = compact_morphology_json_value(&analysis.result.words)?;
        }
    }
    Ok(value)
}

#[requires(true)]
#[ensures(matches!(ret, "valid" | "invalid" | "not-single-word"))]
fn vlatai_status(analysis: &ValsiAnalysis) -> &'static str {
    match analysis.result.status {
        ValsiAnalysisStatus::Valid => "valid",
        ValsiAnalysisStatus::Invalid => "invalid",
        ValsiAnalysisStatus::NotSingleWord => "not-single-word",
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok() || ret.is_err())]
fn vlatai_diagnostics(
    analysis: &ValsiAnalysis,
    source_id: Option<SourceId>,
) -> Result<Vec<Diagnostic>> {
    let mut diagnostics =
        morphology_warning_diagnostics(&analysis.warnings, source_id.clone(), &analysis.input);
    match analysis.result.status {
        ValsiAnalysisStatus::Invalid => {
            let error = analysis
                .result
                .error
                .as_ref()
                .expect("invalid vlatai result carries error");
            diagnostics.push(error.to_diagnostic(source_id, &analysis.input));
        }
        ValsiAnalysisStatus::NotSingleWord => {
            diagnostics.push(vlatai_not_single_word_diagnostic(
                source_id,
                &analysis.input,
                analysis.result.words.len(),
            )?);
        }
        ValsiAnalysisStatus::Valid => {}
    }
    Ok(diagnostics)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error) || ret.is_err())]
fn vlatai_not_single_word_diagnostic(
    source_id: Option<SourceId>,
    source: &str,
    word_count: usize,
) -> Result<Diagnostic> {
    let char_end = source.chars().count();
    let span = source_span_from_char_offsets(source_id, source, 0, char_end)
        .map_err(|error| anyhow!(error))?;
    let (message, label) = if word_count == 0 {
        ("input did not parse as one word", "parsed zero words")
    } else {
        (
            "input parsed as multiple words",
            "parsed more than one word",
        )
    };
    Ok(Diagnostic::new(
        DiagnosticSeverity::Error,
        DiagnosticPhase::Morphology,
        "vlatai.not-single-word".to_owned(),
        message.to_owned(),
        vec![DiagnosticLabel::new(span, label.to_owned(), true)],
        vec![format!("parsed word count: {word_count}")],
        None,
    ))
}

#[requires(true)]
#[ensures(true)]
fn render_vlatai_classification_text(
    out: &mut String,
    classification: &ValsiClassification,
    phoneme_options: PhonemeRenderOptions,
) {
    render_vlatai_classification_text_with_prefix(out, classification, phoneme_options, "");
}

#[requires(true)]
#[ensures(true)]
fn render_vlatai_classification_text_with_prefix(
    out: &mut String,
    classification: &ValsiClassification,
    phoneme_options: PhonemeRenderOptions,
    prefix: &str,
) {
    match classification.kind() {
        ValsiClassificationKind::PlainWord => {
            render_plain_word_classification_text(
                out,
                classification
                    .word()
                    .expect("plain-word classification carries word"),
                phoneme_options,
                prefix,
            );
        }
        ValsiClassificationKind::QuotedWord => {
            out.push_str(&format!("{prefix}category: quoted-word\n"));
            render_plain_word_classification_text(
                out,
                classification.marker().expect("quoted word marker"),
                phoneme_options,
                "marker ",
            );
            render_plain_word_classification_text(
                out,
                classification.quoted_word().expect("quoted word payload"),
                phoneme_options,
                "quoted ",
            );
        }
        ValsiClassificationKind::DelimitedNonLojbanQuote => {
            out.push_str(&format!("{prefix}category: delimited-non-lojban-quote\n"));
            render_plain_word_classification_text(
                out,
                classification.marker().expect("quote marker"),
                phoneme_options,
                "marker ",
            );
            let delimiter = classification
                .delimiter()
                .expect("delimited quote carries delimiter");
            out.push_str(&format!("{prefix}delimiter: {delimiter}\n"));
        }
        ValsiClassificationKind::QuotedWords => {
            out.push_str(&format!("{prefix}category: quoted-words\n"));
            render_plain_word_classification_text(
                out,
                classification.marker().expect("quoted words marker"),
                phoneme_options,
                "marker ",
            );
            out.push_str(&format!(
                "{prefix}quoted word count: {}\n",
                classification.quoted_words().len()
            ));
        }
        ValsiClassificationKind::DelimitedWordQuote => {
            out.push_str(&format!("{prefix}category: delimited-word-quote\n"));
            out.push_str(&format!(
                "{prefix}marker: {}\n",
                classification
                    .marker_text()
                    .expect("delimited word quote marker")
            ));
        }
        ValsiClassificationKind::LerfuWord => {
            out.push_str(&format!("{prefix}category: lerfu-word\n"));
            render_vlatai_classification_text_with_prefix(
                out,
                classification.base().expect("lerfu base"),
                phoneme_options,
                "base ",
            );
            render_plain_word_classification_text(
                out,
                classification.suffix().expect("lerfu suffix"),
                phoneme_options,
                "suffix ",
            );
        }
        ValsiClassificationKind::ZeiCompound => {
            out.push_str(&format!("{prefix}category: zei-compound\n"));
            render_vlatai_classification_text_with_prefix(
                out,
                classification.left().expect("zei left"),
                phoneme_options,
                "left ",
            );
            render_plain_word_classification_text(
                out,
                classification.link().expect("zei link"),
                phoneme_options,
                "link ",
            );
            render_plain_word_classification_text(
                out,
                classification.right().expect("zei right"),
                phoneme_options,
                "right ",
            );
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_plain_word_classification_text(
    out: &mut String,
    classification: &PlainWordClassification,
    phoneme_options: PhonemeRenderOptions,
    prefix: &str,
) {
    match classification.category {
        WordKind::Cmavo => {
            out.push_str(&format!("{prefix}category: cmavo\n"));
            out.push_str(&format!(
                "{prefix}phonemes: {}\n",
                render_vlatai_phonemes(&classification.phonemes, phoneme_options)
            ));
            if let Some(selmaho) = &classification.selmaho {
                out.push_str(&format!("{prefix}selma'o: {selmaho}\n"));
            }
        }
        WordKind::Gismu => {
            out.push_str(&format!("{prefix}category: gismu\n"));
            out.push_str(&format!(
                "{prefix}phonemes: {}\n",
                render_vlatai_phonemes(&classification.phonemes, phoneme_options)
            ));
        }
        WordKind::Lujvo => {
            out.push_str(&format!("{prefix}category: lujvo\n"));
            out.push_str(&format!(
                "{prefix}phonemes: {}\n",
                render_vlatai_phonemes(&classification.phonemes, phoneme_options)
            ));
            let split = classification
                .split
                .as_ref()
                .expect("lujvo classification carries split");
            out.push_str(&format!("{prefix}split: {split}\n"));
            out.push_str(&format!("{prefix}parts:\n"));
            for part in &classification.parts {
                out.push_str(&format!("{prefix}  - {}\n", vlatai_lujvo_part_text(part)));
            }
        }
        WordKind::Fuhivla => {
            out.push_str(&format!("{prefix}category: fu'ivla\n"));
            out.push_str(&format!(
                "{prefix}phonemes: {}\n",
                render_vlatai_phonemes(&classification.phonemes, phoneme_options)
            ));
            let stage = classification
                .stage
                .expect("fu'ivla classification carries stage");
            out.push_str(&format!("{prefix}stage: {}\n", vlatai_fuhivla_stage(stage)));
        }
        WordKind::Cmevla => {
            out.push_str(&format!("{prefix}category: cmevla\n"));
            out.push_str(&format!(
                "{prefix}phonemes: {}\n",
                render_vlatai_phonemes(&classification.phonemes, phoneme_options)
            ));
        }
    }
}

#[requires(!part.text.is_empty())]
#[ensures(!ret.is_empty())]
fn vlatai_lujvo_part_text(part: &ValsiLujvoPart) -> String {
    match part.kind {
        ValsiLujvoPartKind::Hyphen => format!("hyphen: {}", part.text),
        ValsiLujvoPartKind::Rafsi => format!(
            "rafsi: {} ({})",
            part.text,
            part.rafsi_kind
                .map(vlatai_rafsi_kind)
                .unwrap_or("unknown-rafsi")
        ),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vlatai_rafsi_kind(kind: ValsiLujvoRafsiKind) -> &'static str {
    match kind {
        ValsiLujvoRafsiKind::Cvc => "cvc-rafsi",
        ValsiLujvoRafsiKind::Ccv => "ccv-rafsi",
        ValsiLujvoRafsiKind::Cvv => "cvv-rafsi",
        ValsiLujvoRafsiKind::Long => "long-rafsi",
        ValsiLujvoRafsiKind::Gismu => "gismu",
        ValsiLujvoRafsiKind::Fuhivla => "fu'ivla",
        ValsiLujvoRafsiKind::Cultural => "cultural-rafsi",
        ValsiLujvoRafsiKind::Extended => "extended-rafsi",
        ValsiLujvoRafsiKind::Unknown => "unknown-rafsi",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vlatai_fuhivla_stage(stage: ValsiFuhivlaStage) -> &'static str {
    match stage {
        ValsiFuhivlaStage::Stage3 => "stage-3",
        ValsiFuhivlaStage::Stage4 => "stage-4",
        ValsiFuhivlaStage::Unknown => "unknown",
    }
}

#[requires(true)]
#[ensures(true)]
fn vlatai_classification_json(
    classification: &ValsiClassification,
    phoneme_options: PhonemeRenderOptions,
) -> serde_json::Value {
    match classification.kind() {
        ValsiClassificationKind::PlainWord => plain_word_classification_json(
            classification
                .word()
                .expect("plain-word classification carries word"),
            phoneme_options,
        ),
        _ => serde_json::to_value(classification).expect("vlatai classification serializes"),
    }
}

#[requires(true)]
#[ensures(true)]
fn plain_word_classification_json(
    classification: &PlainWordClassification,
    phoneme_options: PhonemeRenderOptions,
) -> serde_json::Value {
    let mut value =
        serde_json::to_value(classification).expect("plain word classification serializes");
    value["phonemes"] = serde_json::Value::String(render_vlatai_phonemes(
        &classification.phonemes,
        phoneme_options,
    ));
    value
}

#[requires(true)]
#[ensures(!ret.is_empty() || phonemes.is_empty())]
fn render_vlatai_phonemes(phonemes: &str, options: PhonemeRenderOptions) -> String {
    Phonemes::from_canonical(phonemes.to_owned())
        .map(|value| value.render(options))
        .unwrap_or_else(|_| phonemes.to_owned())
}

#[requires(!source_label.is_empty())]
#[requires(diagnostic_terminal_width > 0)]
#[ensures(diagnostics.is_empty() -> (!ret.is_err() && ret.as_ref().is_ok_and(String::is_empty)))]
#[ensures(!diagnostics.is_empty() -> (!ret.is_err() && ret.as_ref().is_ok_and(|text| !text.is_empty())))]
fn render_source_diagnostics(
    source_label: &str,
    source: &str,
    diagnostics: &[Diagnostic],
    color_enabled: bool,
    diagnostic_detail: DiagnosticDetailMode,
    glyphs: GlyphStyle,
    diagnostic_terminal_width: usize,
) -> Result<String> {
    render_diagnostics(
        source_label,
        source,
        diagnostics,
        new!(DiagnosticRenderOptions {
            color: color_enabled,
            detail: diagnostic_detail,
            glyphs,
            terminal_width: diagnostic_terminal_width,
        }),
    )
    .map_err(|error| anyhow!(error))
}

#[requires(true)]
#[ensures(ret.len() == warnings.len())]
fn morphology_warning_diagnostics(
    warnings: &[MorphologyWarning],
    source_id: Option<SourceId>,
    source: &str,
) -> Vec<Diagnostic> {
    warnings
        .iter()
        .map(|warning| warning.to_diagnostic(source_id.clone(), source))
        .collect()
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
            new!(TraceRenderOptions {
                color: color_enabled,
                terminal_width,
            }),
        )
    })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn dialect_definition(source: Option<&str>) -> Result<DialectDefinition> {
    source.map_or_else(
        || Ok(DialectDefinition::default()),
        |source| {
            parse_dialect_selection_formula(&DialectSettings::default(), source)
                .map_err(|error| anyhow!(error))
        },
    )
}

#[requires(true)]
#[ensures(true)]
fn cli_glyph_style(ascii: bool) -> GlyphStyle {
    if ascii {
        GlyphStyle::Ascii
    } else {
        GlyphStyle::Unicode
    }
}

#[requires(true)]
#[ensures(true)]
fn cli_diagnostic_detail(detailed_errors: bool) -> DiagnosticDetailMode {
    if detailed_errors {
        DiagnosticDetailMode::Detailed
    } else {
        DiagnosticDetailMode::Summary
    }
}

#[requires(true)]
#[ensures(true)]
fn phoneme_render_options(
    mark_stress: Option<CliStressMark>,
    mark_glides: Option<CliGlideMark>,
    glyphs: GlyphStyle,
) -> PhonemeRenderOptions {
    let default = match glyphs {
        GlyphStyle::Unicode => PhonemeRenderOptions::default(),
        GlyphStyle::Ascii => PhonemeRenderOptions {
            mark_stress: StressMark::None,
            mark_glides: GlideMark::None,
        },
    };
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
fn validate_vlasei_options(input: &VlaseiInput, glyphs: GlyphStyle) -> Result<()> {
    if input.format == VlaseiFormat::Ipa && glyphs == GlyphStyle::Ascii {
        return Err(anyhow!("`--ascii` is not compatible with `--turtai ipa`"));
    }
    validate_ascii_phoneme_projection(input.mark_stress, input.mark_glides, glyphs)?;
    // `--show-refs` is intentionally absent from vlasei: morphology has no
    // place-structure references, so the flag is only defined for `gentufa`.
    match input.format {
        VlaseiFormat::Raw => {
            validate_raw_indent(input.indent)?;
            if glyphs == GlyphStyle::Unicode {
                validate_no_phoneme_projection(input.mark_stress, input.mark_glides, "raw")?;
            }
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
        VlaseiFormat::Ipa => {
            validate_no_indent(
                input.indent,
                "`--indent` is only supported with raw, JSON, and tree output",
            )?;
            validate_no_phoneme_projection(input.mark_stress, input.mark_glides, "IPA")?;
            validate_not_present(
                input.show_spans,
                "`--show-spans` is only supported with `--turtai tree`",
            )?;
            validate_not_present(
                input.decompose_lujvo,
                "`--decompose-lujvo` is only supported with `--turtai tree` or `--turtai brackets`",
            )?;
        }
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
fn validate_gentufa_options(input: &GentufaInput, glyphs: GlyphStyle) -> Result<()> {
    validate_ascii_phoneme_projection(input.mark_stress, input.mark_glides, glyphs)?;
    if input.format != GentufaFormat::Blocks {
        validate_not_present(
            input.output_type.is_some(),
            "`--output-type` is only supported with `--turtai blocks`",
        )?;
    }
    if input.format == GentufaFormat::Raw {
        validate_raw_indent(input.indent)?;
        if glyphs == GlyphStyle::Unicode {
            validate_no_phoneme_projection(input.mark_stress, input.mark_glides, "raw")?;
        }
        validate_not_present(
            input.show_spans,
            "`--show-spans` is only supported with `--turtai tree`",
        )?;
        validate_not_present(
            input.show_refs,
            "`--show-refs` is only supported with `--turtai tree`",
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
                    input.show_refs,
                    "`--show-refs` is only supported with `--turtai tree`",
                )?;
                validate_not_present(
                    input.decompose_lujvo,
                    "`--decompose-lujvo` is only supported with `--turtai tree` or `--turtai brackets`",
                )?;
            }
            GentufaFormat::Blocks => {
                validate_no_indent(
                    input.indent,
                    "`--indent` is only supported with raw, JSON, and tree output",
                )?;
                validate_not_present(
                    input.show_defs,
                    "`--show-defs` is not supported with `--turtai blocks`",
                )?;
                validate_not_present(
                    input.show_spans,
                    "`--show-spans` is only supported with `--turtai tree`",
                )?;
                validate_not_present(
                    input.show_refs,
                    "`--show-refs` is only supported with `--turtai tree`",
                )?;
                validate_not_present(
                    input.decompose_lujvo,
                    "`--decompose-lujvo` is only supported with `--turtai tree` or `--turtai brackets`",
                )?;
                let _ = resolve_gentufa_blocks_output_type(input)?;
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
                validate_not_present(
                    input.show_refs,
                    "`--show-refs` is only supported with `--turtai tree`",
                )?;
            }
            GentufaFormat::Raw => {}
        }
    }
    Ok(())
}

#[requires(input.format == GentufaFormat::Blocks)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn resolve_gentufa_blocks_output_type(input: &GentufaInput) -> Result<GentufaImageOutputType> {
    if let Some(output_type) = input.output_type {
        return Ok(output_type);
    }
    let Some(path) = input.output_file.as_ref() else {
        return Ok(GentufaImageOutputType::Svg);
    };
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.trim().to_ascii_lowercase());
    match extension.as_deref() {
        Some("svg") => Ok(GentufaImageOutputType::Svg),
        Some("png") => Ok(GentufaImageOutputType::Png),
        Some(extension) if !extension.is_empty() => Err(anyhow!(
            "cannot infer gentufa blocks output type from extension `.{extension}`; use `--output-type svg` or `--output-type png`"
        )),
        _ => Err(anyhow!(
            "cannot infer gentufa blocks output type without a .svg or .png extension; use `--output-type svg` or `--output-type png`"
        )),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_ascii_phoneme_projection(
    mark_stress: Option<CliStressMark>,
    mark_glides: Option<CliGlideMark>,
    glyphs: GlyphStyle,
) -> Result<()> {
    if glyphs == GlyphStyle::Unicode {
        return Ok(());
    }
    if matches!(
        mark_stress,
        Some(CliStressMark::Acute | CliStressMark::Caps)
    ) {
        return Err(anyhow!(
            "`--ascii` is not compatible with `--mark-stress acute` or `--mark-stress caps`"
        ));
    }
    if matches!(mark_glides, Some(CliGlideMark::Breve)) {
        return Err(anyhow!(
            "`--ascii` is not compatible with `--mark-glides breve`"
        ));
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

#[requires(!output_format.is_empty())]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_no_phoneme_projection(
    mark_stress: Option<CliStressMark>,
    mark_glides: Option<CliGlideMark>,
    output_format: &str,
) -> Result<()> {
    if mark_stress.is_some() || mark_glides.is_some() {
        return Err(anyhow!(
            "`--mark-stress` and `--mark-glides` are not supported with {output_format} output"
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
fn read_text_input(
    file: Option<&PathBuf>,
    text: &[String],
    stdin_text: Option<&str>,
) -> Result<String> {
    match (file, text.is_empty()) {
        (Some(path), _) => fs::read_to_string(path)
            .map_err(|source| anyhow!("failed to read `{}`: {source}", path.display())),
        (None, false) => Ok(text.join(" ")),
        (None, true) => {
            if let Some(input) = stdin_text {
                return Ok(input.to_owned());
            }
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
fn command_not_implemented(command: &str) -> Result<()> {
    Err(anyhow!(
        "`{command}` is scaffolded but its implementation has not been ported yet"
    ))
}

#[cfg(test)]
#[path = "../tests/support/cli.rs"]
mod tests;
