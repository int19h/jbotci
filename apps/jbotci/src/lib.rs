// The LSP moves immutable generated syntax snapshots across the analysis worker
// boundary. Proving their recursive generated model is `Send + Sync` requires
// the same kind of deep auto-trait traversal as generating that model itself.
#![recursion_limit = "2048"]

mod benchmark;
mod cli;
mod commands;
mod lsp;
mod output;
mod tool;

pub use cli::main_entry;
use cli::run_cli_command_with_tool_context;
use commands::*;
use output::*;
pub use tool::*;

#[doc(hidden)]
pub mod test_harness {
    pub use super::commands::VlackuRenderOptions;
    pub use super::{
        Cli, CliCollisionScope, CliColorPolicy, CliGlideMark, CliStatus, CliStressMark,
        CliSumtiPlaces, CliTracePhase, CliUsePrecomputed, Command, CuktaCliFormat, CuktaInput,
        GentufaFormat, GentufaImageOutputType, GentufaInput, GimfihiCliFormat,
        GimfihiCliNormalizer, GimfihiCliScorer, GimfihiInput, GimfihiSourceWordKind, JvozbaInput,
        SetupInput, TersmuFormat, TersmuInput, TextInput, ToolAlineNormalizer, ToolAlineSaliences,
        ToolCollisionScope, ToolCuktaFormat, ToolCuktaMode, ToolCuktaRequest, ToolExecutionContext,
        ToolGimfihiFormat, ToolGimfihiRequest, ToolGimfihiScorer, ToolGimfihiSource, ToolStatus,
        ToolVlackuMode, ToolVlackuRequest, VlackuInput, VlaseiFormat, VlaseiInput, VlataiFormat,
        VlataiInput, run_tool_cukta_with_context, run_tool_gimfihi, run_tool_vlacku,
        run_tool_vlacku_with_context,
    };
    pub use bityzba::{ensures, invariant, new, requires};
    pub use clap::{CommandFactory, Parser};
    pub use jbotci_diagnostics::{TraceLevel, TraceOptions, TracePhase};
    pub use jbotci_gimfihi::{GimfihiOutput, GimfihiScorer, GimfihiSourceInput};
    pub use jbotci_jvozba::JvozbaInput as JvozbaSourceInput;
    pub use jbotci_output::{DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH, GlyphStyle};
    pub use jbotci_phonetic::{AlineParameters, AlineSaliences};
    pub use jbotci_search::vlacku::{
        VlackuAuthor, VlackuCard, VlackuOutcome, VlackuRequest, VlackuSearchOutput,
    };
    pub use std::fs;
    pub use std::num::NonZeroUsize;
    pub use std::path::PathBuf;

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn run_cli<WOut: std::io::Write, WErr: std::io::Write>(
        cli: Cli,
        stdout: &mut WOut,
        stderr: &mut WErr,
        color_enabled: bool,
    ) -> anyhow::Result<CliStatus> {
        super::cli::run_cli(cli, stdout, stderr, color_enabled)
    }

    #[requires(diagnostic_terminal_width > 0)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn run_cli_with_color_policy_and_width<WOut: std::io::Write, WErr: std::io::Write>(
        cli: Cli,
        stdout: &mut WOut,
        stderr: &mut WErr,
        color_policy: CliColorPolicy,
        diagnostic_terminal_width: usize,
    ) -> anyhow::Result<CliStatus> {
        super::cli::run_cli_with_color_policy_and_width(
            cli,
            stdout,
            stderr,
            color_policy,
            diagnostic_terminal_width,
        )
    }

    #[requires(limit > 0)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn trace_options(
        trace: &Option<Option<String>>,
        phase: TracePhase,
        limit: usize,
    ) -> anyhow::Result<TraceOptions> {
        super::trace_options(trace, phase, limit)
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn render_vlacku_output(
        output: &VlackuSearchOutput,
        color: bool,
        glyphs: GlyphStyle,
    ) -> String {
        super::render_vlacku_output(output, color, glyphs)
    }

    #[requires(output_terminal_width.is_none_or(|width| width > 0))]
    #[ensures(!ret.is_empty())]
    pub fn render_vlacku_output_with_width(
        output: &VlackuSearchOutput,
        color: bool,
        glyphs: GlyphStyle,
        output_terminal_width: Option<usize>,
    ) -> String {
        super::render_vlacku_output_with_width(output, color, glyphs, output_terminal_width)
    }

    #[requires(options.output_terminal_width.is_none_or(|width| width > 0))]
    #[ensures(!ret.is_empty())]
    pub fn render_vlacku_output_with_options(
        output: &VlackuSearchOutput,
        options: VlackuRenderOptions,
    ) -> String {
        super::render_vlacku_output_with_options(output, options)
    }
}

use benchmark::BenchmarkMeasurement;
use bityzba::{data, invariant, new, requires};
use std::fs;
use std::io::{IsTerminal, Read, Write};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

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
    generated_model_blocks_layout_with_references, recovered_generated_model_blocks_layout,
    render_gentufa_blocks_png, render_gentufa_blocks_svg,
};
use jbotci_gimfihi::{
    CollisionKind, CollisionScope, GIMFIHI_DEFAULT_COUNT, GIMFIHI_MAX_COUNT, GIMFIHI_MAX_WEIGHT,
    GIMFIHI_MIN_WEIGHT, GimfihiCandidate, GimfihiOutput, GimfihiRequest, GimfihiScorer,
    GimfihiSourceInput, GismuCollision, RafsiAvailability, compose_gismu, default_shapes,
    parse_preset, parse_shape, parse_source_spec,
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
    segment_words_with_modifiers_recovered_with_options_and_source_id_attempt,
};
use jbotci_output::{
    BracketRenderOptions, DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH, DefinitionPlaceMap,
    DiagnosticDetailMode, DiagnosticRenderOptions, GlideMark, GlyphStyle, JsonRenderOptions,
    LojbanScript, PhonemeRenderOptions, StressMark, TraceRenderOptions, TreeRenderOptions,
    compact_generated_model_json_string_with_options, compact_morphology_json_string_with_options,
    compact_morphology_json_value, compact_recovered_morphology_json_string_with_options,
    compact_recovered_syntax_json_string_with_options, format_definition_line_with_indexed_places,
    format_notes_line_with_indexed_places, generated_reference_display, ipa_morphology_text,
    json_string_with_options, pretty_generated_model_brackets_with_options,
    pretty_generated_model_tree_with_options, pretty_morphology_brackets_with_options,
    pretty_morphology_tree_with_options, pretty_recovered_morphology_brackets_with_options,
    pretty_recovered_morphology_raw, pretty_recovered_morphology_tree_with_options,
    pretty_recovered_syntax_brackets_with_options, pretty_recovered_syntax_raw,
    pretty_recovered_syntax_tree_with_options, render_diagnostics, render_json_value_with_options,
    render_trace_report,
};
use jbotci_phonetic::{AlineFeature, AlineNormalizer, AlineParameters, AlineSaliences};
use jbotci_search::vlacku::{
    DEFAULT_VLACKU_RESULT_COUNT, VlackuCard, VlackuCompositionKind, VlackuCompositionPiece,
    VlackuOutcome, VlackuRequest, VlackuRequestData, VlackuSearchOptions, VlackuSearchOutput,
    WordTypeFilter, dictionary_cards_for_word_likes, dictionary_entry_card,
    dictionary_entry_passes_vlacku_filters, dictionary_matches_for_word_likes, format_vote_display,
    normalize_word_type_filter, parse_word_type_filter, run_vlacku_requests,
};
use jbotci_semantics::{
    SemanticBuildOptions, build_generated_semantic_graph_with_dictionary_and_options, render_lean3,
    render_tree, render_tree_proj,
};
use jbotci_source::SourceId;
use jbotci_syntax::{
    ParseOptions, SYNTAX_TRACE_FILTERS, SyntaxRecoveryParseData,
    parse_syntax_tree_with_recovery_with_source_and_options_attempt,
};
use unicode_width::UnicodeWidthStr;

const DEFAULT_MAX_ERRORS: NonZeroUsize = NonZeroUsize::new(20).unwrap();
const VLACKU_DETAIL_INDENT: &str = "    ";

#[derive(Debug, Clone, Parser)]
#[command(name = "jbotci")]
#[command(about = "Command-line Lojban toolkit")]
#[invariant(true)]
pub struct Cli {
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
    pub color: concolor_clap::ColorChoice,
    #[arg(long = "benchmark", global = true, value_name = "N")]
    pub benchmark: Option<NonZeroUsize>,
    #[command(subcommand)]
    pub command: Command,
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
#[invariant(::Lsp { .. } => true)]
#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    #[command(name = "vlasei", visible_alias = "lex")]
    Vlasei(VlaseiInput),
    #[command(name = "vlatai")]
    Vlatai(VlataiInput),
    #[command(name = "gentufa", visible_alias = "parse")]
    Gentufa(GentufaInput),
    #[command(name = "mulgau", visible_alias = "completions")]
    Mulgau(TextInput),
    #[command(
        name = "tersmu",
        about = "Build and render a typed semantic graph",
        long_about = "Build and render a typed semantic graph. The default tree+proj format is a structural scope tree plus only commitments projected out of their structural site. Bare tree is the same spine without the projected section; JSON is the canonical interchange graph.\n\nInterpretation contract: indentation and `>` mean structural descent. The tree spine is authoritative where commitment follows structural position; entries under `projected:` take widest commitment scope. `mode=` is exact graph vocabulary. `denotes` states referential identity; `binder-dependence=underspecified` names possible binders, not proven dependence. Generated-bound events co-vary through structural `binds=exists`; referential events use denotation commitments, and `binds=exists` is not a projected claim. Event suffixes always name time, actuality, aspect, recurrence, space, spatial aspect, spatial recurrence, and details; `unspecified` is explicit absence of information, never a negative claim."
    )]
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
    /// Run the Language Server Protocol server over standard input/output.
    #[command(name = "lsp")]
    Lsp {
        /// Accepted for editor compatibility; the LSP transport is always stdio.
        #[arg(long)]
        stdio: bool,
    },
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliStatus {
    Success,
    Failure,
    ValidMissing,
    InvalidInput,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CliColorPolicy {
    pub stdout: bool,
    pub stderr: bool,
}

impl CliColorPolicy {
    #[requires(true)]
    #[ensures(!ret.stdout)]
    #[ensures(!ret.stderr)]
    pub fn never() -> Self {
        Self {
            stdout: false,
            stderr: false,
        }
    }

    #[requires(true)]
    #[ensures(ret.stdout == enabled)]
    #[ensures(ret.stderr == enabled)]
    pub fn same(enabled: bool) -> Self {
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
pub struct CliProgressPolicy {
    pub embedding_setup: bool,
}

impl CliProgressPolicy {
    #[requires(true)]
    #[ensures(!ret.embedding_setup)]
    pub fn disabled() -> Self {
        Self {
            embedding_setup: false,
        }
    }

    #[requires(true)]
    #[ensures(ret.embedding_setup == enabled)]
    pub fn embedding_setup(enabled: bool) -> Self {
        Self {
            embedding_setup: enabled,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum GentufaFormat {
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
pub enum TersmuFormat {
    /// Canonical `lojban-semantics-json-1` flat id-graph.
    #[value(alias = "djeisone")]
    Json,
    /// Derived indented view of utterance and formula nesting.
    Tree,
    /// Structural tree plus only commitments displaced from their tree site.
    #[value(name = "tree+proj")]
    TreeProj,
    /// EXPERIMENTAL: model-facing "lean3" notation (Phase-B candidate; working
    /// name, subject to change — not the default, tree+proj remains default).
    /// Provenance (source spans) renders off; the provenance opt-in is
    /// library-only for now (`jbotci_semantics::render_lean3` with
    /// `Lean3Config { provenance: true }`), not exposed as a CLI/MCP flag.
    Lean3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum GentufaImageOutputType {
    Svg,
    Png,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum VlaseiFormat {
    Brackets,
    Tree,
    Ipa,
    Raw,
    #[value(alias = "djeisone")]
    Json,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum VlataiFormat {
    Text,
    #[value(alias = "djeisone")]
    Json,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum GimfihiCliFormat {
    Table,
    #[value(alias = "djeisone")]
    Json,
}

#[invariant(::Classic => true)]
#[invariant(::Phonetic => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum GimfihiCliScorer {
    Classic,
    Phonetic,
}

impl From<GimfihiCliScorer> for GimfihiScorer {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: GimfihiCliScorer) -> Self {
        match value {
            GimfihiCliScorer::Classic => Self::Classic,
            GimfihiCliScorer::Phonetic => Self::Phonetic,
        }
    }
}

#[invariant(::SourceSide => true)]
#[invariant(::CandidateSide => true)]
#[invariant(::Symmetric => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum GimfihiCliNormalizer {
    SourceSide,
    CandidateSide,
    Symmetric,
}

impl From<GimfihiCliNormalizer> for AlineNormalizer {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: GimfihiCliNormalizer) -> Self {
        match value {
            GimfihiCliNormalizer::SourceSide => Self::SourceSide,
            GimfihiCliNormalizer::CandidateSide => Self::CandidateSide,
            GimfihiCliNormalizer::Symmetric => Self::Symmetric,
        }
    }
}

#[invariant(value.is_finite() && *value >= 0.0)]
#[derive(Debug, Clone, PartialEq)]
pub struct GimfihiSalienceOverride {
    pub feature: AlineFeature,
    pub value: f64,
}

impl FromStr for GimfihiSalienceOverride {
    type Err = String;

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|override_value| override_value.value.is_finite()) || ret.is_err())]
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let Some((feature_name, raw_value)) = value.split_once('=') else {
            return Err("salience must use FEATURE=VALUE".to_owned());
        };
        let feature_name = feature_name.trim().to_ascii_lowercase();
        let Some(feature) = AlineFeature::all()
            .iter()
            .copied()
            .find(|feature| feature.as_str() == feature_name)
        else {
            return Err(format!("unknown salience feature `{feature_name}`"));
        };
        let parsed = raw_value
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("invalid salience `{feature_name}` value `{raw_value}`"))?;
        if !parsed.is_finite() {
            return Err(format!("salience `{feature_name}` must be finite"));
        }
        if parsed < 0.0 {
            return Err(format!("salience `{feature_name}` must be nonnegative"));
        }
        Ok(new!(GimfihiSalienceOverride {
            feature,
            value: parsed,
        }))
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliCollisionScope {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliStressMark {
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
pub enum CliGlideMark {
    None,
    Breve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliTracePhase {
    Morphology,
    Syntax,
    All,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliParsedTraceSpec {
    pub level: TraceLevel,
    pub filter: Option<TraceFilter>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CliTraceConfig {
    pub phase: TracePhase,
    pub limit: usize,
}

#[invariant(!self.command_name.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliTraceValidation {
    pub command_name: &'static str,
    pub trace_phase: Option<TracePhase>,
    pub trace_limit_present: bool,
    pub trace_list: bool,
    pub supports_morphology: bool,
    pub supports_syntax: bool,
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
pub struct VlaseiInput {
    #[arg(long = "file", alias = "sfaile")]
    pub file: Option<PathBuf>,
    #[arg(long = "ascii")]
    pub ascii: bool,
    #[arg(long = "detailed-errors")]
    pub detailed_errors: bool,
    #[arg(long = "max-errors", default_value_t = DEFAULT_MAX_ERRORS)]
    pub max_errors: NonZeroUsize,
    #[arg(long = "trace-phase", value_enum)]
    pub trace_phase: Option<CliTracePhase>,
    #[arg(long = "trace-limit")]
    pub trace_limit: Option<usize>,
    #[arg(long = "trace-list")]
    pub trace_list: bool,
    #[arg(
        long = "turtai",
        visible_alias = "format",
        default_value_t = VlaseiFormat::Brackets,
        value_enum
    )]
    pub format: VlaseiFormat,
    #[arg(
        long = "trace",
        alias = "plivei",
        value_name = "SPEC",
        num_args = 0..=1,
        default_missing_value = "1"
    )]
    pub trace: Option<Option<String>>,
    #[arg(long = "dialect")]
    pub dialect: Option<String>,
    #[arg(long = "indent")]
    pub indent: Option<usize>,
    #[arg(long = "mark-stress", value_enum)]
    pub mark_stress: Option<CliStressMark>,
    #[arg(long = "mark-glides", value_enum)]
    pub mark_glides: Option<CliGlideMark>,
    #[arg(long = "show-spans")]
    pub show_spans: bool,
    #[arg(long = "decompose-lujvo")]
    pub decompose_lujvo: bool,
    #[arg()]
    pub text: Vec<String>,
}

impl VlaseiInput {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn read_text(&self) -> Result<String> {
        self.read_text_with_stdin(None)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn read_text_with_stdin(&self, stdin_text: Option<&str>) -> Result<String> {
        read_text_input(self.file.as_ref(), &self.text, stdin_text)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn dialect_definition(&self) -> Result<DialectDefinition> {
        dialect_definition(self.dialect.as_deref())
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Args)]
pub struct VlataiInput {
    #[arg(long = "ascii")]
    pub ascii: bool,
    #[arg(long = "detailed-errors")]
    pub detailed_errors: bool,
    #[arg(
        long = "turtai",
        visible_alias = "format",
        default_value_t = VlataiFormat::Text,
        value_enum
    )]
    pub format: VlataiFormat,
    #[arg(long = "indent")]
    pub indent: Option<usize>,
    #[arg(long = "dialect")]
    pub dialect: Option<String>,
    #[arg(long = "mark-stress", value_enum)]
    pub mark_stress: Option<CliStressMark>,
    #[arg(long = "mark-glides", value_enum)]
    pub mark_glides: Option<CliGlideMark>,
    #[arg(required = true)]
    pub words: Vec<String>,
}

impl VlataiInput {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn dialect_definition(&self) -> Result<DialectDefinition> {
        dialect_definition(self.dialect.as_deref())
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Args)]
pub struct TextInput {
    #[arg(long = "file", alias = "sfaile")]
    pub file: Option<PathBuf>,
    #[arg(
        long = "trace",
        alias = "plivei",
        value_name = "SPEC",
        num_args = 0..=1,
        default_missing_value = "1"
    )]
    pub trace: Option<Option<String>>,
    #[arg(long = "dialect")]
    pub dialect: Option<String>,
    #[arg(long = "indent")]
    pub indent: Option<usize>,
    #[arg()]
    pub text: Vec<String>,
}

impl TextInput {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn read_text(&self) -> Result<String> {
        self.read_text_with_stdin(None)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn read_text_with_stdin(&self, stdin_text: Option<&str>) -> Result<String> {
        read_text_input(self.file.as_ref(), &self.text, stdin_text)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn dialect_definition(&self) -> Result<DialectDefinition> {
        dialect_definition(self.dialect.as_deref())
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Args)]
pub struct TersmuInput {
    #[arg(long = "file", alias = "sfaile")]
    pub file: Option<PathBuf>,
    #[arg(
        long = "format",
        default_value_t = TersmuFormat::TreeProj,
        value_enum,
        help = "Output tree+proj (default), bare tree, or canonical JSON; `+proj` names the projected-commitments feature added to the tree base format"
    )]
    pub format: TersmuFormat,
    #[arg(long = "max-errors", default_value_t = DEFAULT_MAX_ERRORS)]
    pub max_errors: NonZeroUsize,
    #[arg(
        long = "trace",
        alias = "plivei",
        value_name = "SPEC",
        num_args = 0..=1,
        default_missing_value = "1"
    )]
    pub trace: Option<Option<String>>,
    #[arg(long = "dialect")]
    pub dialect: Option<String>,
    #[arg(long = "show-defs")]
    pub show_defs: bool,
    #[arg(long = "story-time")]
    pub story_time: bool,
    #[arg(long = "indent")]
    pub indent: Option<usize>,
    #[arg()]
    pub text: Vec<String>,
}

impl TersmuInput {
    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn read_text_with_stdin(&self, stdin_text: Option<&str>) -> Result<String> {
        read_text_input(self.file.as_ref(), &self.text, stdin_text)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn dialect_definition(&self) -> Result<DialectDefinition> {
        dialect_definition(self.dialect.as_deref())
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Args)]
pub struct GentufaInput {
    #[arg(long = "file", alias = "sfaile")]
    pub file: Option<PathBuf>,
    #[arg(long = "ascii")]
    pub ascii: bool,
    #[arg(long = "detailed-errors")]
    pub detailed_errors: bool,
    #[arg(long = "error-context", default_value_t = 1)]
    pub error_context: usize,
    #[arg(long = "max-errors", default_value_t = DEFAULT_MAX_ERRORS)]
    pub max_errors: NonZeroUsize,
    #[arg(long = "trace-phase", value_enum)]
    pub trace_phase: Option<CliTracePhase>,
    #[arg(long = "trace-limit")]
    pub trace_limit: Option<usize>,
    #[arg(long = "trace-list")]
    pub trace_list: bool,
    #[arg(
        long = "turtai",
        visible_alias = "format",
        default_value_t = GentufaFormat::Brackets,
        value_enum
    )]
    pub format: GentufaFormat,
    #[arg(
        long = "trace",
        alias = "plivei",
        value_name = "SPEC",
        num_args = 0..=1,
        default_missing_value = "1"
    )]
    pub trace: Option<Option<String>>,
    #[arg(long = "dialect")]
    pub dialect: Option<String>,
    #[arg(long = "show-defs")]
    pub show_defs: bool,
    #[arg(long = "indent")]
    pub indent: Option<usize>,
    #[arg(long = "mark-stress", value_enum)]
    pub mark_stress: Option<CliStressMark>,
    #[arg(long = "mark-glides", value_enum)]
    pub mark_glides: Option<CliGlideMark>,
    #[arg(long = "show-spans")]
    pub show_spans: bool,
    #[arg(long = "show-refs")]
    pub show_refs: bool,
    #[arg(long = "show-elided")]
    pub show_elided: bool,
    #[arg(long = "decompose-lujvo")]
    pub decompose_lujvo: bool,
    #[arg(long = "output-type", value_enum)]
    pub output_type: Option<GentufaImageOutputType>,
    #[arg(short = 'o', long = "output-file")]
    pub output_file: Option<PathBuf>,
    #[arg()]
    pub text: Vec<String>,
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
    pub fn read_text(&self) -> Result<String> {
        self.read_text_with_stdin(None)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn read_text_with_stdin(&self, stdin_text: Option<&str>) -> Result<String> {
        read_text_input(self.file.as_ref(), &self.text, stdin_text)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
    pub fn dialect_definition(&self) -> Result<DialectDefinition> {
        dialect_definition(self.dialect.as_deref())
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Args)]
pub struct CuktaInput {
    #[arg(short = 'n', long = "count")]
    pub count: Option<usize>,
    #[arg(long = "toc")]
    pub toc: bool,
    #[arg(long = "section", value_name = "REF")]
    pub section: Option<String>,
    #[arg(long = "example", value_name = "REF")]
    pub example: Option<String>,
    #[arg(long = "valsi", value_name = "WORD")]
    pub valsi: Option<String>,
    #[arg(long = "target", value_name = "section|paragraph|example", action = ArgAction::Append)]
    pub targets: Vec<String>,
    #[arg(long = "sections")]
    pub target_sections: bool,
    #[arg(long = "paragraphs")]
    pub target_paragraphs: bool,
    #[arg(long = "examples")]
    pub target_examples: bool,
    #[arg(
        long = "turtai",
        visible_alias = "format",
        default_value_t = CuktaCliFormat::Markdown,
        value_enum
    )]
    pub format: CuktaCliFormat,
    #[arg()]
    pub query: Vec<String>,
}

#[invariant(true)]
#[derive(Debug, Clone, Args)]
pub struct GimfihiInput {
    /// A source word as `LANG[:WEIGHT]:WORD` (repeat per source). WORD is Lojban
    /// letters, or a phonemic IPA transcription in `[ ... ]` brackets (e.g.
    /// `eng:210:[kæt]`).
    #[arg(
        long = "source",
        value_name = "LANG[:WEIGHT]:WORD",
        value_parser = parse_source_spec,
        action = ArgAction::Append
    )]
    pub sources: Vec<GimfihiSourceInput>,
    /// Candidate scorer. `classic` preserves CLL §4.14 behavior; `phonetic`
    /// uses docs/gismu-phonetic-medoid.md.
    #[arg(long = "scorer", value_enum, default_value_t = GimfihiCliScorer::Classic)]
    pub scorer: GimfihiCliScorer,
    /// ALINE substitution ceiling from docs/gismu-phonetic-medoid.md.
    #[arg(long = "c-sub", default_value_t = AlineParameters::default().c_sub)]
    pub c_sub: f64,
    /// ALINE 1↔2 expansion ceiling from docs/gismu-phonetic-medoid.md.
    #[arg(long = "c-exp", default_value_t = AlineParameters::default().c_exp)]
    pub c_exp: f64,
    /// ALINE unmatched-segment penalty from docs/gismu-phonetic-medoid.md.
    #[arg(long = "c-skip", default_value_t = AlineParameters::default().c_skip)]
    pub c_skip: f64,
    /// ALINE vowel evidence discount from docs/gismu-phonetic-medoid.md.
    #[arg(long = "c-vwl", default_value_t = AlineParameters::default().c_vwl)]
    pub c_vwl: f64,
    /// Source prefix/suffix skip rate from docs/gismu-phonetic-medoid.md.
    #[arg(long = "c-flank", default_value_t = AlineParameters::default().c_flank)]
    pub c_flank: f64,
    /// ALINE normalizer from docs/gismu-phonetic-medoid.md.
    #[arg(
        long = "normalizer",
        value_enum,
        default_value_t = GimfihiCliNormalizer::SourceSide
    )]
    pub normalizer: GimfihiCliNormalizer,
    /// Override a feature salience as FEATURE=VALUE (repeatable). Defaults:
    /// syllabic=5, place=40, manner=50, voice=10, nasal=10,
    /// retroflex=10, lateral=10, aspirated=5, high=5, back=5, round=5,
    /// long=1; see docs/gismu-phonetic-medoid.md.
    #[arg(
        long = "salience",
        value_name = "FEATURE=VALUE",
        action = ArgAction::Append
    )]
    pub saliences: Vec<GimfihiSalienceOverride>,
    #[arg(long = "preset", value_name = "PRESET")]
    pub preset: Option<String>,
    #[arg(long = "shape", value_name = "SHAPE", action = ArgAction::Append)]
    pub shapes: Vec<String>,
    #[arg(
        long = "check-collisions",
        value_enum,
        default_value_t = CliCollisionScope::All
    )]
    pub check_collisions: CliCollisionScope,
    #[arg(long = "all-letters")]
    pub all_letters: bool,
    #[arg(long = "show-collisions")]
    pub show_collisions: bool,
    #[arg(long = "require-free-short-rafsi")]
    pub require_free_short_rafsi: bool,
    #[arg(short = 'n', long = "count", value_name = "N")]
    pub count: Option<usize>,
    #[arg(long = "highlight", value_name = "GISMU")]
    pub highlight: Option<String>,
    #[arg(long = "format", value_enum, default_value_t = GimfihiCliFormat::Table)]
    pub format: GimfihiCliFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CuktaCliFormat {
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
pub enum CliSumtiPlaces {
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
pub struct VlackuInput {
    pub count: Option<usize>,
    pub ascii: bool,
    pub word_types: Vec<String>,
    pub min_votes: Option<i32>,
    pub min_similarity: Option<f32>,
    pub sumti_places: CliSumtiPlaces,
    pub decompose_lujvo: bool,
    pub show_etymology: bool,
    pub requests: Vec<VlackuRequest>,
    pub query: Vec<String>,
}

#[invariant(true)]
#[derive(Debug, Clone, Args)]
pub struct SetupInput {
    #[arg(long = "embedding")]
    pub embedding: bool,
    #[arg(long = "force")]
    pub force: bool,
    #[arg(
        long = "use-precomputed",
        value_enum,
        default_value_t = CliUsePrecomputed::Auto
    )]
    pub use_precomputed: CliUsePrecomputed,
    #[arg(long = "skip-validation")]
    pub skip_validation: bool,
    #[arg(long = "model", default_value = DEFAULT_MODEL_KEY)]
    pub model: String,
    #[arg(long = "index-dir")]
    pub index_dir: Option<PathBuf>,
    #[arg(long = "model-dir")]
    pub model_dir: Option<PathBuf>,
}

#[invariant(::Auto => true)]
#[invariant(::Always => true)]
#[invariant(::Never => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CliUsePrecomputed {
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
    let query = matches
        .get_many::<String>("query")
        .map(|values| values.cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let positional_meaning_request = ordered_requests
        .is_empty()
        .then(|| joined_query_text(&query))
        .filter(|query| !query.is_empty())
        .map(VlackuRequest::meaning);
    let has_positional_meaning_request = positional_meaning_request.is_some();

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
        requests: positional_meaning_request
            .into_iter()
            .chain(ordered_requests.into_iter().map(|(_, request)| request))
            .collect(),
        query: if has_positional_meaning_request {
            Vec::new()
        } else {
            query
        },
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
pub struct JvozbaInput {
    pub cmevla: bool,
    pub sources: Vec<JvozbaSourceInput>,
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
#[ensures(true)]
fn joined_query_text(query: &[String]) -> String {
    query.join(" ")
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
        .map(|warning| {
            warning
                .to_diagnostic(source_id.clone(), source)
                .expect("morphology warning offsets belong to the parser source")
        })
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

#[requires(true)]
#[ensures(ret.is_empty() || ret.ends_with('\n'))]
fn render_cli_trace(report: Option<&TraceReport>, color_enabled: bool) -> String {
    report.map_or_else(String::new, |report| {
        render_trace_report(
            report,
            TraceRenderOptions {
                color: color_enabled,
            },
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
fn validate_tersmu_options(input: &TersmuInput) -> Result<()> {
    if input.format == TersmuFormat::Json {
        validate_not_present(
            input.show_defs,
            "`--show-defs` is not supported with `--format json`",
        )?;
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
