mod benchmark;

use benchmark::BenchmarkMeasurement;
use bityzba::{invariant, new, requires};
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
use jbotci_cll::{
    CllError, CllRenderFormat, CuktaRequest, CuktaSearchMode, CuktaTargetFilter,
    DEFAULT_CUKTA_CLI_RESULT_COUNT, embedded_cll_site, render_cukta_request, render_search_output,
};
use jbotci_diagnostics::{
    DEFAULT_TRACE_LIMIT, Diagnostic, TraceFilter, TraceLevel, TraceOptions, TracePhase, TraceReport,
};
use jbotci_dialect::{DialectDefinition, parse_dialect_definition};
use jbotci_embeddings::native::{load_backend_for_search, setup_embeddings};
use jbotci_embeddings::{
    DEFAULT_MODEL_KEY, SetupOptions, default_index_root, semantic_cukta_output,
    semantic_vlacku_hits,
};
use jbotci_gentufa::{
    ElidedTerminator, EmbeddedGentufaFonts, GentufaBlockAnnotation, GentufaBlockOptions,
    GentufaPngOptions, GentufaScript, GentufaSvgOptions, WebSourceRange, blocks_layout,
    elided_terminators, render_gentufa_blocks_png, render_gentufa_blocks_svg, rendered_leaves,
};
use jbotci_jvozba::{
    JvozbaBuildResult, JvozbaInput as JvozbaSourceInput, JvozbaMode, JvozbaSegmentKind,
    build_best_jvozba_detailed,
};
use jbotci_morphology::{
    MORPHOLOGY_TRACE_FILTERS, MorphologyOptions, MorphologyWarning, Phonemes, WordLike,
    segment_words_with_modifiers_with_options_and_source_id_attempt,
};
use jbotci_output::{
    BracketRenderOptions, DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH, DiagnosticDetailMode,
    DiagnosticRenderOptions, GlideMark, GlyphStyle, JsonRenderOptions, LojbanScript,
    PhonemeRenderOptions, StressMark, TraceRenderOptions, TreeRenderOptions,
    compact_morphology_json_string_with_options, compact_syntax_json_string_with_options,
    format_definition_or_notes_line_with_indexed_places, ipa_morphology_text,
    pretty_brackets_with_options, pretty_morphology_brackets_with_options,
    pretty_morphology_tree_with_options, pretty_tree_with_options,
    reference_display_model_for_syntax_tree, render_diagnostics, render_trace_report,
};
use jbotci_search::vlacku::{
    DEFAULT_VLACKU_RESULT_COUNT, VlackuCard, VlackuCompositionKind, VlackuCompositionPiece,
    VlackuOutcome, VlackuRequest, VlackuSearchOptions, VlackuSearchOutput,
    dictionary_cards_for_word_likes, dictionary_entry_card, dictionary_matches_for_word_likes,
    filter_vlacku_cards, format_vote_display, normalize_word_type_filter, run_vlacku_requests,
};
use jbotci_semantics::references::ReferenceAnalysis;
use jbotci_source::SourceId;
use jbotci_syntax::{
    ParseOptions, SYNTAX_TRACE_FILTERS, parse_syntax_tree_with_source_and_options_attempt,
};
#[cfg(feature = "grammar-debug")]
use jbotci_syntax::{syntax_grammar_ebnf, syntax_grammar_svg};
use owo_colors::OwoColorize;
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

#[derive(Debug, Clone, Subcommand)]
#[invariant(true)]
#[invariant(::Vlasei(..) => true)]
#[invariant(::Gentufa(..) => true)]
#[invariant(::Mulgau(..) => true)]
#[invariant(::Tersmu(..) => true)]
#[invariant(::Vlacku(..) => true)]
#[invariant(::Jvozba(..) => true)]
#[invariant(::Cukta(..) => true)]
#[invariant(::Zbasu(..) => true)]
#[invariant(::Setup(..) => true)]
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
    Vlacku(VlackuInput),
    #[command(name = "jvozba")]
    Jvozba(JvozbaInput),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
enum CliStatus {
    Success,
    Failure,
    ValidMissing,
    InvalidInput,
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
    Blocks,
    #[value(alias = "vipcihe", help = "alias: vipcihe")]
    Tree,
    Raw,
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

#[derive(Debug, Clone, Args)]
#[invariant(true)]
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

#[derive(Debug, Clone, Args)]
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

#[derive(Debug, Clone, Args)]
#[invariant(true)]
struct GentufaInput {
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
#[derive(Debug, Clone, Args)]
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

#[derive(Debug, Clone, Args)]
#[invariant(true)]
struct CuktaInput {
    #[arg(short = 'n', long = "count")]
    count: Option<usize>,
    #[arg(long = "index")]
    index: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
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

#[derive(Debug, Clone)]
#[invariant(true)]
struct VlackuInput {
    count: Option<usize>,
    index: bool,
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

#[derive(Debug, Clone, Args)]
#[invariant(true)]
struct SetupInput {
    #[arg(long = "embedding")]
    embedding: bool,
    #[arg(long = "force")]
    force: bool,
    #[arg(long = "model", default_value = DEFAULT_MODEL_KEY)]
    model: String,
    #[arg(long = "index-dir")]
    index_dir: Option<PathBuf>,
    #[arg(long = "model-dir")]
    model_dir: Option<PathBuf>,
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
        .arg(Arg::new("index").long("index").action(ArgAction::SetTrue))
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
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("rafsi")
                .long("rafsi")
                .value_name("RAFSI")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("lujvo")
                .long("lujvo")
                .value_name("WORD")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("glob")
                .long("glob")
                .value_name("PATTERN")
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("sound")
                .long("sound")
                .value_name("TEXT|[IPA]")
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
        VlackuRequest::Valsi,
        &mut ordered_requests,
    );
    collect_ordered_vlacku_requests(
        matches,
        "rafsi",
        VlackuRequest::Rafsi,
        &mut ordered_requests,
    );
    collect_ordered_vlacku_requests(
        matches,
        "lujvo",
        VlackuRequest::Lujvo,
        &mut ordered_requests,
    );
    collect_ordered_vlacku_requests(matches, "glob", VlackuRequest::Glob, &mut ordered_requests);
    collect_ordered_vlacku_requests(
        matches,
        "sound",
        VlackuRequest::Sound,
        &mut ordered_requests,
    );
    ordered_requests.sort_by_key(|(index, _)| *index);

    VlackuInput {
        count: matches.get_one::<usize>("count").copied(),
        index: matches.get_flag("index"),
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

#[derive(Debug, Clone)]
#[invariant(true)]
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
#[ensures(true)]
fn main() -> ExitCode {
    match run() {
        Ok(CliStatus::Success) => ExitCode::SUCCESS,
        Ok(CliStatus::Failure) => ExitCode::FAILURE,
        Ok(CliStatus::ValidMissing) => ExitCode::from(10),
        Ok(CliStatus::InvalidInput) => ExitCode::from(11),
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
    let output_terminal_width = stdout_terminal_width();
    let diagnostic_terminal_width = stderr_terminal_width();
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    run_cli_with_color_policy_and_terminal_widths(
        cli,
        &mut stdout,
        &mut stderr,
        color_policy,
        diagnostic_terminal_width,
        output_terminal_width,
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
    run_cli_with_color_policy_and_terminal_widths(
        cli,
        stdout,
        stderr,
        color_policy,
        diagnostic_terminal_width,
        None,
    )
}

#[requires(diagnostic_terminal_width > 0)]
#[requires(output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_cli_with_color_policy_and_terminal_widths<WOut: Write, WErr: Write>(
    cli: Cli,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_terminal_width: usize,
    output_terminal_width: Option<usize>,
) -> Result<CliStatus> {
    let color_policy = color_policy.with_choice(cli.color);
    if let Some(iterations) = cli.benchmark {
        return run_cli_benchmark(
            cli.command,
            iterations,
            stdout,
            stderr,
            color_policy,
            diagnostic_terminal_width,
            output_terminal_width,
        );
    }
    run_cli_command(
        cli.command,
        stdout,
        stderr,
        color_policy,
        diagnostic_terminal_width,
        output_terminal_width,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
#[requires(diagnostic_terminal_width > 0)]
#[requires(output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_cli_benchmark<WOut: Write, WErr: Write>(
    command: Command,
    iterations: NonZeroUsize,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_terminal_width: usize,
    output_terminal_width: Option<usize>,
) -> Result<CliStatus> {
    validate_benchmark_command(&command)?;
    let stdin_text = benchmark_stdin_text(&command)?;
    let mut measurement = BenchmarkMeasurement::start(iterations);
    for _ in 0..iterations.get() {
        let iteration_start = std::time::Instant::now();
        let status = run_cli_command(
            command.clone(),
            stdout,
            stderr,
            color_policy,
            diagnostic_terminal_width,
            output_terminal_width,
            stdin_text.as_deref(),
        )?;
        measurement.record_iteration(iteration_start.elapsed(), status);
    }
    let report = measurement.finish();
    stderr.write_all(report.render().as_bytes())?;
    Ok(report.final_status())
}

#[allow(clippy::too_many_arguments)]
#[requires(diagnostic_terminal_width > 0)]
#[requires(output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_cli_command<WOut: Write, WErr: Write>(
    command: Command,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_terminal_width: usize,
    output_terminal_width: Option<usize>,
    stdin_text: Option<&str>,
) -> Result<CliStatus> {
    match command {
        Command::Vlasei(mut input) => {
            let glyphs = cli_glyph_style(input.ascii);
            let diagnostic_detail = cli_diagnostic_detail(input.detailed_errors);
            let trace_limit = input.trace_limit.unwrap_or(DEFAULT_TRACE_LIMIT);
            let trace_limit_present = input.trace_limit.is_some();
            if trace_limit == 0 {
                bail!("--trace-limit must be greater than 0");
            }
            let requested_trace_phase = input.trace_phase.map(TracePhase::from);
            normalize_trace_text_input(&mut input.trace, &input.file, &mut input.text);
            validate_vlasei_options(&input, glyphs)?;
            validate_trace_controls(
                &input.trace,
                new!(CliTraceValidation {
                    command_name: "vlasei",
                    trace_phase: requested_trace_phase,
                    trace_limit_present,
                    trace_list: input.trace_list,
                    supports_morphology: true,
                    supports_syntax: false,
                }),
            )?;
            if input.trace_list {
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
            let text = input.read_text_with_stdin(stdin_text)?;
            let dialect = input.dialect_definition()?;
            let morphology_options = MorphologyOptions::default()
                .with_dialect_definition(&dialect)
                .with_trace_options(morphology_trace_options);
            let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
                &text,
                &morphology_options,
                Some(SourceId(source_label.clone())),
            );
            let attempt = attempt.into_data();
            let trace_stderr = render_cli_trace(
                attempt.trace.as_ref(),
                color_policy.stderr,
                diagnostic_terminal_width,
            );
            let words = match attempt.result {
                Ok(words) => words,
                Err(error) => {
                    stderr.write_all(trace_stderr.as_bytes())?;
                    let mut diagnostics = morphology_warning_diagnostics(
                        &attempt.warnings,
                        Some(SourceId(source_label.clone())),
                        &text,
                    );
                    diagnostics
                        .push(error.to_diagnostic(Some(SourceId(source_label.clone())), &text));
                    write_source_diagnostics(
                        stderr,
                        &source_label,
                        &text,
                        &diagnostics,
                        color_policy.stderr,
                        diagnostic_detail,
                        glyphs,
                        diagnostic_terminal_width,
                    )?;
                    return Ok(CliStatus::Failure);
                }
            };
            stderr.write_all(trace_stderr.as_bytes())?;
            let diagnostics = morphology_warning_diagnostics(
                &attempt.warnings,
                Some(SourceId(source_label.clone())),
                &text,
            );
            write_source_diagnostics(
                stderr,
                &source_label,
                &text,
                &diagnostics,
                color_policy.stderr,
                diagnostic_detail,
                glyphs,
                diagnostic_terminal_width,
            )?;
            let phoneme_options =
                phoneme_render_options(input.mark_stress, input.mark_glides, glyphs);
            match input.format {
                VlaseiFormat::Json => {
                    let rendered = compact_morphology_json_string_with_options(
                        &words,
                        JsonRenderOptions {
                            indent: input.indent.unwrap_or(2),
                            phonemes: phoneme_options,
                            show_elided: false,
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
                            script: LojbanScript::Latin,
                            glyphs,
                            decompose_lujvo: input.decompose_lujvo,
                            insert_hair_space: false,
                            show_elided: false,
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
                            glyphs,
                            show_spans: input.show_spans,
                            show_refs: false,
                            decompose_lujvo: input.decompose_lujvo,
                            show_elided: false,
                        },
                    )?;
                    writeln!(stdout, "{rendered}")?;
                }
                VlaseiFormat::Ipa => {
                    let rendered = ipa_morphology_text(&words, &text)?;
                    writeln!(stdout, "{rendered}")?;
                }
                VlaseiFormat::Raw => write_debug_output(stdout, &words, input.indent)?,
            }
            Ok(CliStatus::Success)
        }
        Command::Gentufa(mut input) => {
            let glyphs = cli_glyph_style(input.ascii);
            let diagnostic_detail = cli_diagnostic_detail(input.detailed_errors);
            let trace_limit = input.trace_limit.unwrap_or(DEFAULT_TRACE_LIMIT);
            let trace_limit_present = input.trace_limit.is_some();
            if trace_limit == 0 {
                bail!("--trace-limit must be greater than 0");
            }
            let requested_trace_phase = input.trace_phase.map(TracePhase::from);
            normalize_trace_text_input(&mut input.trace, &input.file, &mut input.text);
            validate_trace_controls(
                &input.trace,
                new!(CliTraceValidation {
                    command_name: "gentufa",
                    trace_phase: requested_trace_phase,
                    trace_limit_present,
                    trace_list: input.trace_list,
                    supports_morphology: true,
                    supports_syntax: true,
                }),
            )?;
            if input.trace_list {
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
                glyphs,
                diagnostic_terminal_width,
                CliTraceConfig {
                    phase: requested_trace_phase.unwrap_or(TracePhase::Syntax),
                    limit: trace_limit,
                },
                stdin_text,
            )
        }
        Command::Mulgau(input) => {
            validate_trace_controls_for_unsupported_command(
                "mulgau",
                &input.trace,
                None,
                false,
                false,
            )?;
            let _ = input.read_text_with_stdin(stdin_text)?;
            command_not_implemented("mulgau")?;
            Ok(CliStatus::Success)
        }
        Command::Tersmu(input) => {
            validate_trace_controls_for_unsupported_command(
                "tersmu",
                &input.trace,
                None,
                false,
                false,
            )?;
            let _ = input.read_text_with_stdin(stdin_text)?;
            command_not_implemented("tersmu")?;
            Ok(CliStatus::Success)
        }
        Command::Vlacku(input) => {
            let glyphs = cli_glyph_style(input.ascii);
            run_vlacku(
                input,
                stdout,
                stderr,
                color_policy.stdout,
                glyphs,
                output_terminal_width,
            )
        }
        Command::Jvozba(input) => run_jvozba(input, stdout, color_policy.stdout),
        Command::Cukta(input) => run_cukta(input, stdout, stderr),
        Command::Zbasu(input) => {
            validate_trace_controls_for_unsupported_command(
                "zbasu",
                &input.trace,
                None,
                false,
                false,
            )?;
            let _ = input.read_text_with_stdin(stdin_text)?;
            command_not_implemented("zbasu")?;
            Ok(CliStatus::Success)
        }
        Command::Setup(input) => run_setup(input, stdout),
        #[cfg(feature = "grammar-debug")]
        Command::Gerna(input) => run_gerna(input, stdout),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_benchmark_command(command: &Command) -> Result<()> {
    if command_supports_benchmark(command) {
        Ok(())
    } else {
        bail!("`--benchmark` is only supported with vlasei, gentufa, vlacku, and cukta")
    }
}

#[requires(true)]
#[ensures(true)]
fn command_supports_benchmark(command: &Command) -> bool {
    matches!(
        command,
        Command::Vlasei(_) | Command::Gentufa(_) | Command::Vlacku(_) | Command::Cukta(_)
    )
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn benchmark_stdin_text(command: &Command) -> Result<Option<String>> {
    if benchmark_command_reads_stdin(command) {
        read_text_input(None, &[], None).map(Some)
    } else {
        Ok(None)
    }
}

#[requires(true)]
#[ensures(true)]
fn benchmark_command_reads_stdin(command: &Command) -> bool {
    match command {
        Command::Vlasei(input) => vlasei_input_reads_stdin(input),
        Command::Gentufa(input) => gentufa_input_reads_stdin(input),
        _ => false,
    }
}

#[requires(true)]
#[ensures(input.file.is_some() -> !ret)]
fn vlasei_input_reads_stdin(input: &VlaseiInput) -> bool {
    if input.trace_list {
        return false;
    }
    trace_text_input_reads_stdin(&input.file, &input.text, &input.trace)
}

#[requires(true)]
#[ensures(input.file.is_some() -> !ret)]
fn gentufa_input_reads_stdin(input: &GentufaInput) -> bool {
    if input.trace_list {
        return false;
    }
    trace_text_input_reads_stdin(&input.file, &input.text, &input.trace)
}

#[requires(true)]
#[ensures(file.is_some() -> !ret)]
fn trace_text_input_reads_stdin(
    file: &Option<PathBuf>,
    text: &[String],
    trace: &Option<Option<String>>,
) -> bool {
    let mut normalized_trace = trace.clone();
    let mut normalized_text = text.to_owned();
    normalize_trace_text_input(&mut normalized_trace, file, &mut normalized_text);
    file.is_none() && normalized_text.is_empty()
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_setup<WOut: Write>(input: SetupInput, stdout: &mut WOut) -> Result<CliStatus> {
    if !input.embedding {
        bail!("Choose at least one setup task, e.g. `jbotci setup --embedding`.");
    }
    let report = setup_embeddings(&SetupOptions {
        model_key: input.model,
        force: input.force,
        index_dir: input.index_dir,
        model_dir: input.model_dir,
    })
    .map_err(|error| anyhow!(error.to_string()))?;
    writeln!(
        stdout,
        "Embedding setup complete.\nmodel: {}\nindex: {}\npack: {}\ndictionary rows: {}\nCLL rows: {}",
        report.model_path.display(),
        report.index_root.display(),
        report.pack_id,
        report.dictionary_rows,
        report.cll_rows
    )?;
    Ok(CliStatus::Success)
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
) -> Result<CliStatus> {
    validate_vlacku_input(&input)?;
    let options = vlacku_search_options(&input)?;
    let output = if input.requests.is_empty() {
        match run_semantic_vlacku(&input, &options) {
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
) -> Result<VlackuSearchOutput> {
    let query = joined_query_text(&input.query).trim().to_owned();
    if query.is_empty() {
        bail!("vlacku query text must be non-empty.");
    }
    let index_root = default_index_root().map_err(|error| anyhow!(error.to_string()))?;
    let mut backend = load_backend_for_search(DEFAULT_MODEL_KEY, None)
        .map_err(|error| anyhow!(error.to_string()))?;
    let dictionary = jbotci_dictionary_data::english();
    let hits = semantic_vlacku_hits(
        &mut backend,
        &query,
        dictionary.entries().len(),
        &index_root,
        DEFAULT_MODEL_KEY,
    )
    .map_err(|error| anyhow!(error.to_string()))?;
    let cards = hits
        .into_iter()
        .filter_map(|hit| {
            dictionary.entries().get(hit.entry_index).map(|entry| {
                dictionary_entry_card(dictionary, entry, Some(hit.score), options.decompose_lujvo)
            })
        })
        .collect::<Vec<_>>();
    Ok(VlackuSearchOutput {
        cards: filter_vlacku_cards(cards, options, true),
        outcome: VlackuOutcome::Found,
        diagnostics: Vec::new(),
    })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_cukta<WOut: Write, WErr: Write>(
    input: CuktaInput,
    stdout: &mut WOut,
    stderr: &mut WErr,
) -> Result<CliStatus> {
    validate_cukta_input(&input)?;
    let request = cukta_request_from_input(&input)?;
    let site = embedded_cll_site().map_err(|error| anyhow!(error.to_string()))?;
    let rendered = match &request {
        CuktaRequest::Search {
            mode: CuktaSearchMode::Meaning,
            query,
            count,
            targets,
        } => {
            let index_root = default_index_root().map_err(|error| anyhow!(error.to_string()))?;
            let output =
                match load_backend_for_search(DEFAULT_MODEL_KEY, None).and_then(|mut backend| {
                    semantic_cukta_output(
                        &mut backend,
                        jbotci_cll::cll_search_all_chunks(site),
                        query,
                        *count,
                        *targets,
                        &index_root,
                        DEFAULT_MODEL_KEY,
                    )
                }) {
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
    if input.index {
        if request_mode_count > 0 || cukta_target_flags_present(input) {
            bail!("`cukta --index` does not accept fetch/search modes or target filters.");
        }
        bail!("`cukta --index` is reserved for future semantic embeddings.");
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
fn validate_vlacku_input(input: &VlackuInput) -> Result<()> {
    if input.index {
        bail!("`vlacku --index` is reserved for future semantic embeddings");
    }
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
            bail!(
                "No query provided for vlacku. Use --valsi, --rafsi, --lujvo, --glob, or --sound."
            );
        }
        let _ = joined_query_text(&input.query);
    }
    if !input.query.is_empty() && !input.requests.is_empty() {
        bail!(
            "Do not pass positional query text when using --valsi, --rafsi, --lujvo, --glob, or --sound."
        );
    }
    let sound_count = input
        .requests
        .iter()
        .filter(|request| matches!(request, VlackuRequest::Sound(_)))
        .count();
    if sound_count > 1 {
        bail!("`--sound` may be specified only once");
    }
    if sound_count == 1 && input.requests.len() > 1 {
        bail!("`--sound` cannot be combined with --valsi, --rafsi, --lujvo, or --glob");
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
    let (flag, value) = match request {
        VlackuRequest::Valsi(value) => ("--valsi", value),
        VlackuRequest::Rafsi(value) => ("--rafsi", value),
        VlackuRequest::Lujvo(value) => ("--lujvo", value),
        VlackuRequest::Glob(value) => ("--glob", value),
        VlackuRequest::Sound(value) => ("--sound", value),
        VlackuRequest::Meaning(value) => ("semantic query", value),
    };
    if value.trim().is_empty() {
        bail!("{flag} requires a non-empty value");
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn vlacku_search_options(input: &VlackuInput) -> Result<VlackuSearchOptions> {
    Ok(VlackuSearchOptions {
        count: input.count.unwrap_or(DEFAULT_VLACKU_RESULT_COUNT),
        word_types: parse_vlacku_word_types(&input.word_types)?,
        min_votes: input.min_votes,
        min_similarity: input.min_similarity,
        decompose_lujvo: input.decompose_lujvo,
    })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn parse_vlacku_word_types(raw_values: &[String]) -> Result<Vec<String>> {
    let mut values = Vec::new();
    for raw_value in raw_values {
        for piece in raw_value.split(',') {
            let normalized = normalize_word_type_filter(piece);
            if normalized.is_empty() {
                continue;
            }
            if !is_valid_vlacku_word_type_filter(&normalized) {
                bail!(
                    "Unknown `--word-type` value: {normalized}. Use gismu, lujvo, cmavo, cmevla, fu'ivla, or brivla."
                );
            }
            if !values.contains(&normalized) {
                values.push(normalized);
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
fn is_valid_vlacku_word_type_filter(value: &str) -> bool {
    matches!(
        value,
        "gismu" | "lujvo" | "cmavo" | "cmevla" | "fu'ivla" | "brivla"
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

#[requires(true)]
#[ensures(true)]
fn dark(text: &str, color: bool) -> String {
    if color {
        text.bright_black().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn yellow(text: &str, color: bool) -> String {
    if color {
        text.yellow().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn yellow_underlined(text: &str, color: bool) -> String {
    if color {
        text.yellow().underline().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn blue(text: &str, color: bool) -> String {
    if color {
        text.bright_blue().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn magenta(text: &str, color: bool) -> String {
    if color {
        text.magenta().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn green(text: &str, color: bool) -> String {
    if color {
        text.green().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn red(text: &str, color: bool) -> String {
    if color {
        text.red().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn cyan(text: &str, color: bool) -> String {
    if color {
        text.cyan().to_string()
    } else {
        text.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
fn light(text: &str, color: bool) -> String {
    if color {
        text.white().to_string()
    } else {
        text.to_owned()
    }
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
            return Ok(new!(GentufaRendered {
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
            let stdout = render_gentufa_blocks_output(
                &parsed.parse_tree,
                &text,
                words.as_slice(),
                phoneme_options,
                input.show_elided,
                output_type,
            )?;
            return Ok(new!(GentufaRendered {
                status: CliStatus::Success,
                stdout,
                stderr,
            }));
        }
        GentufaFormat::Brackets => {
            let rendered = pretty_brackets_with_options(
                &parsed.parse_tree,
                &text,
                BracketRenderOptions {
                    color: color_policy.stdout,
                    phonemes: phoneme_options,
                    script: LojbanScript::Latin,
                    glyphs,
                    decompose_lujvo: input.decompose_lujvo,
                    insert_hair_space: false,
                    show_elided: input.show_elided,
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
                    glyphs,
                    show_spans: input.show_spans,
                    show_refs: input.show_refs,
                    decompose_lujvo: input.decompose_lujvo,
                    show_elided: input.show_elided,
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
                    show_elided: input.show_elided,
                },
            )?;
            stdout.push_str(&colorize_json(&rendered, color_policy.stdout));
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

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|output| !output.is_empty()) || ret.is_err())]
fn render_gentufa_blocks_output(
    syntax: &jbotci_syntax::ast::TextSyntax,
    source: &str,
    words: &[WordLike],
    phoneme_options: PhonemeRenderOptions,
    show_elided: bool,
    output_type: GentufaImageOutputType,
) -> Result<Vec<u8>> {
    let analysis =
        ReferenceAnalysis::analyze(syntax).map_err(|error| anyhow!(error.to_string()))?;
    let reference_model = reference_display_model_for_syntax_tree(
        &analysis,
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
            show_elided,
        },
    );
    let block_options = GentufaBlockOptions {
        script: GentufaScript::Latin,
        show_elided,
        phonemes: phoneme_options,
    };
    let leaves = rendered_leaves(syntax, source, &block_options);
    let elided = elided_terminators(&analysis, syntax, &block_options);
    let mut annotations = gentufa_block_annotations(words);
    annotations.extend(gentufa_elided_block_annotations(&elided));
    let layout = blocks_layout(
        &analysis,
        &reference_model,
        source,
        &leaves,
        &elided,
        &annotations,
        &block_options,
    );
    let svg_options = GentufaSvgOptions {
        show_glosses: true,
        script: GentufaScript::Latin,
        title: "jbotci gentufa blocks".to_owned(),
    };
    let fonts = EmbeddedGentufaFonts::get();
    match output_type {
        GentufaImageOutputType::Svg => {
            Ok(render_gentufa_blocks_svg(&layout, &svg_options, fonts)?.into_bytes())
        }
        GentufaImageOutputType::Png => Ok(render_gentufa_blocks_png(
            &layout,
            &GentufaPngOptions {
                svg: svg_options,
                ..GentufaPngOptions::default()
            },
            fonts,
        )?),
    }
}

#[requires(true)]
#[ensures(true)]
fn gentufa_elided_block_annotations(
    terminators: &[ElidedTerminator],
) -> Vec<GentufaBlockAnnotation<()>> {
    terminators
        .iter()
        .filter_map(|terminator| {
            let output = run_vlacku_requests(
                jbotci_dictionary_data::english(),
                &[VlackuRequest::Valsi(terminator.dictionary_text.clone())],
                &VlackuSearchOptions {
                    count: 1,
                    word_types: Vec::new(),
                    min_votes: None,
                    min_similarity: None,
                    decompose_lujvo: false,
                },
            );
            let card = output.cards.into_iter().next()?;
            Some(GentufaBlockAnnotation {
                range: terminator.range,
                text: Some(terminator.text.clone()),
                glosses: card.glosses,
                definition: Some(card.definition.trim().to_owned())
                    .filter(|definition| !definition.is_empty()),
                tooltip: None,
            })
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn gentufa_block_annotations(words: &[WordLike]) -> Vec<GentufaBlockAnnotation<()>> {
    dictionary_matches_for_word_likes(jbotci_dictionary_data::english(), words)
        .into_iter()
        .map(|parsed_match| {
            let first = parsed_match.cards.first();
            GentufaBlockAnnotation {
                range: WebSourceRange {
                    byte_start: parsed_match.byte_start,
                    byte_end: parsed_match.byte_end,
                    char_start: parsed_match.char_start,
                    char_end: parsed_match.char_end,
                },
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
    glyphs: GlyphStyle,
    diagnostic_terminal_width: usize,
) -> Result<String> {
    render_diagnostics(
        source_label,
        source,
        diagnostics,
        DiagnosticRenderOptions {
            color: color_enabled,
            detail: diagnostic_detail,
            glyphs,
            terminal_width: diagnostic_terminal_width,
        },
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
fn stream_supports_ansi_color(stream: concolor::Stream) -> bool {
    concolor::get(stream).ansi_color()
}

#[requires(true)]
#[ensures(ret.is_none_or(|width| width > 0))]
fn stdout_terminal_width() -> Option<usize> {
    let stdout = std::io::stdout();
    if !stdout.is_terminal() {
        return None;
    }
    terminal_size::terminal_size_of(stdout)
        .map(|(terminal_size::Width(width), _height)| usize::from(width))
        .filter(|width| *width > 0)
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
    use jbotci_embeddings::{EMBEDDING_INDEX_DIR_ENV, EMBEDDING_MODEL_DIR_ENV};
    use jbotci_morphology::segment_words_with_modifiers;
    use jbotci_syntax::parse_syntax_tree;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};

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
    fn parses_benchmark_before_and_after_subcommand() {
        let before_cli = Cli::try_parse_from(["jbotci", "--benchmark", "3", "vlasei", "coi"])
            .expect("benchmark before subcommand");
        assert_eq!(before_cli.benchmark.map(NonZeroUsize::get), Some(3));
        assert!(matches!(before_cli.command, Command::Vlasei(_)));

        let after_cli = Cli::try_parse_from(["jbotci", "vlasei", "--benchmark", "4", "coi"])
            .expect("benchmark after subcommand");
        assert_eq!(after_cli.benchmark.map(NonZeroUsize::get), Some(4));
        assert!(matches!(after_cli.command, Command::Vlasei(_)));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_zero_benchmark_iterations() {
        let error = Cli::try_parse_from(["jbotci", "vlasei", "--benchmark", "0", "coi"])
            .expect_err("zero benchmark iteration count is rejected");
        assert_eq!(error.kind(), ErrorKind::ValueValidation);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn benchmark_repeats_stdout_and_reports_success_metrics() {
        let once = run_cli_capture(&["jbotci", "vlasei", "--format", "brackets", "coi"], false);
        assert_eq!(once.status, CliStatus::Success);
        assert!(once.stderr.is_empty());

        let benchmark = run_cli_capture(
            &[
                "jbotci",
                "vlasei",
                "--benchmark",
                "2",
                "--format",
                "brackets",
                "coi",
            ],
            false,
        );
        assert_eq!(benchmark.status, CliStatus::Success);
        assert_eq!(benchmark.stdout, format!("{}{}", once.stdout, once.stdout));
        assert_benchmark_report_contains(
            &benchmark.stderr,
            "iterations: 2",
            "statuses: success=2 failure=0 valid-missing=0 invalid-input=0",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn benchmark_continues_failure_statuses_and_appends_metrics_after_stderr() {
        let once = run_cli_capture(&["jbotci", "vlasei", "aa"], false);
        assert_eq!(once.status, CliStatus::Failure);
        assert!(!once.stderr.is_empty());

        let benchmark = run_cli_capture(&["jbotci", "vlasei", "--benchmark", "2", "aa"], false);
        assert_eq!(benchmark.status, CliStatus::Failure);
        let benchmark_start = benchmark
            .stderr
            .rfind("benchmark:\n")
            .expect("benchmark report");
        assert_eq!(
            &benchmark.stderr[..benchmark_start],
            format!("{}{}", once.stderr, once.stderr)
        );
        assert_benchmark_report_contains(
            &benchmark.stderr[benchmark_start..],
            "iterations: 2",
            "statuses: success=0 failure=2 valid-missing=0 invalid-input=0",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn benchmark_rejects_unsupported_commands() {
        let cli = Cli::try_parse_from(["jbotci", "jvozba", "--benchmark", "2", "lojbo", "bangu"])
            .expect("benchmark flag parses globally");
        let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("benchmark rejects unsupported command");
        assert!(
            error
                .to_string()
                .contains("only supported with vlasei, gentufa, vlacku, and cukta")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_vlacku_primary_name_and_dict_alias() {
        let Command::Vlacku(primary_input) =
            Cli::try_parse_from(["jbotci", "vlacku", "--valsi", "klama"])
                .expect("primary vlacku command")
                .command
        else {
            panic!("expected vlacku command");
        };
        assert_eq!(
            primary_input.requests,
            vec![VlackuRequest::Valsi("klama".to_owned())]
        );
        assert_eq!(primary_input.sumti_places, CliSumtiPlaces::Index);

        let Command::Vlacku(alias_input) =
            Cli::try_parse_from(["jbotci", "dict", "--sumti-places", "raw", "--rafsi", "kla"])
                .expect("dict alias command")
                .command
        else {
            panic!("expected vlacku command");
        };
        assert_eq!(
            alias_input.requests,
            vec![VlackuRequest::Rafsi("kla".to_owned())]
        );
        assert_eq!(alias_input.sumti_places, CliSumtiPlaces::Raw);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_jvozba_command() {
        let Command::Jvozba(input) =
            Cli::try_parse_from(["jbotci", "jvozba", "--cmevla", "lojbo", "--rafsi", "bau"])
                .expect("jvozba command")
                .command
        else {
            panic!("expected jvozba command");
        };
        assert!(input.cmevla);
        assert_eq!(
            input.sources,
            vec![
                JvozbaSourceInput::Word("lojbo".to_owned()),
                JvozbaSourceInput::FixedRafsi("bau".to_owned()),
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_jvozba_word_and_rafsi_order() {
        let Command::Jvozba(input) = Cli::try_parse_from([
            "jbotci", "jvozba", "--rafsi", "jbo", "bangu", "--rafsi", "bau",
        ])
        .expect("jvozba command")
        .command
        else {
            panic!("expected jvozba command");
        };
        assert_eq!(
            input.sources,
            vec![
                JvozbaSourceInput::FixedRafsi("jbo".to_owned()),
                JvozbaSourceInput::Word("bangu".to_owned()),
                JvozbaSourceInput::FixedRafsi("bau".to_owned()),
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_vlacku_mixed_repeated_request_order() {
        let Command::Vlacku(input) = Cli::try_parse_from([
            "jbotci",
            "vlacku",
            "--valsi",
            "a",
            "--rafsi",
            "bau",
            "--valsi",
            "klama",
            "--glob",
            "CVCCV",
            "--lujvo",
            "mivyselbai",
        ])
        .expect("mixed vlacku requests")
        .command
        else {
            panic!("expected vlacku command");
        };

        assert_eq!(
            input.requests,
            vec![
                VlackuRequest::Valsi("a".to_owned()),
                VlackuRequest::Rafsi("bau".to_owned()),
                VlackuRequest::Valsi("klama".to_owned()),
                VlackuRequest::Glob("CVCCV".to_owned()),
                VlackuRequest::Lujvo("mivyselbai".to_owned()),
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_removed_vlacku_min_match_switch() {
        let error =
            Cli::try_parse_from(["jbotci", "vlacku", "--min-match", "80", "--valsi", "klama"])
                .expect_err("min-match is no longer accepted");
        assert_eq!(error.kind(), ErrorKind::UnknownArgument);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_reserved_vlacku_index_and_accepts_semantic_query_path() {
        let index_cli = Cli::try_parse_from(["jbotci", "vlacku", "--index", "--valsi", "klama"])
            .expect("index flag parses");
        let index_error = run_cli(index_cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("index is not implemented");
        assert!(
            index_error
                .to_string()
                .contains("future semantic embeddings")
        );

        let run = run_cli_capture_with_embedding_dirs(
            &["jbotci", "vlacku", "going somewhere"],
            false,
            &unique_embedding_test_path("reserved-vlacku-model-missing"),
            &unique_embedding_test_path("reserved-vlacku-index-missing"),
        );
        assert_eq!(run.status, CliStatus::InvalidInput);
        assert!(run.stderr.contains("jbotci setup --embedding"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_vlacku_sound_exclusive_combinations() {
        let cli = Cli::try_parse_from(["jbotci", "vlacku", "--sound", "klama", "--valsi", "klama"])
            .expect("sound combination parses");
        let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("sound cannot combine with exact modes");
        assert!(error.to_string().contains("cannot be combined"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_vlacku_min_similarity_outside_sound_mode() {
        let cli = Cli::try_parse_from([
            "jbotci",
            "vlacku",
            "--min-similarity",
            "80",
            "--valsi",
            "klama",
        ])
        .expect("min-similarity with valsi parses");
        let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("min-similarity is sound-only");
        assert!(error.to_string().contains("only valid with `--sound`"));
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
            Cli::try_parse_from(["jbotci", "gentufa", "--turtai", "raw", "--show-defs", "coi"])
                .expect("raw with show-defs parses")
                .command
        else {
            panic!("expected gentufa command")
        };
        assert_eq!(raw_input.format, GentufaFormat::Raw);
        assert!(raw_input.show_defs);

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
            Cli::try_parse_from(["jbotci", "gentufa", "--show-defs", "coi"])
                .expect("show-defs flag")
                .command
        else {
            panic!("expected gentufa command")
        };
        assert!(defs_input.show_defs);

        let Command::Gentufa(dialect_input) = Cli::try_parse_from([
            "jbotci",
            "gentufa",
            "--dialect",
            "(+ZANTUFA-CONNECTIVES)",
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
            Cli::try_parse_from(["jbotci", "gerna", "--dialect", "(+ZANTUFA-QUOTES)"])
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

        let Command::Vlasei(ipa_input) =
            Cli::try_parse_from(["jbotci", "vlasei", "--format", "ipa", "coi"])
                .expect("vlasei IPA")
                .command
        else {
            panic!("expected vlasei command")
        };
        assert_eq!(ipa_input.format, VlaseiFormat::Ipa);

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
        let Command::Gentufa(default_input) = default_cli.command else {
            panic!("expected gentufa command");
        };
        assert!(!default_input.ascii);
        assert!(!default_input.detailed_errors);

        let bare_cli =
            Cli::try_parse_from(["jbotci", "gentufa", "--color", "coi"]).expect("bare color");
        assert_eq!(bare_cli.color, concolor_clap::ColorChoice::Always);

        let always_cli = Cli::try_parse_from(["jbotci", "gentufa", "--color=always", "coi"])
            .expect("always color");
        assert_eq!(always_cli.color, concolor_clap::ColorChoice::Always);

        let never_cli = Cli::try_parse_from(["jbotci", "gentufa", "--color=never", "coi"])
            .expect("never color");
        assert_eq!(never_cli.color, concolor_clap::ColorChoice::Never);

        let detailed_cli = Cli::try_parse_from(["jbotci", "gentufa", "--detailed-errors", "coi"])
            .expect("detailed errors");
        let Command::Gentufa(detailed_input) = detailed_cli.command else {
            panic!("expected gentufa command");
        };
        assert!(detailed_input.detailed_errors);

        let ascii_cli =
            Cli::try_parse_from(["jbotci", "gentufa", "--ascii", "coi"]).expect("ascii flag");
        let Command::Gentufa(ascii_input) = ascii_cli.command else {
            panic!("expected gentufa command");
        };
        assert!(ascii_input.ascii);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_trace_options_and_aliases() {
        let cli = Cli::try_parse_from([
            "jbotci",
            "gentufa",
            "--trace-phase",
            "all",
            "--trace-limit",
            "7",
            "--trace",
            "argument:3",
            "mi",
            "klama",
        ])
        .expect("trace options");
        let Command::Gentufa(input) = cli.command else {
            panic!("expected gentufa command")
        };
        assert_eq!(input.trace_phase, Some(CliTracePhase::All));
        assert_eq!(input.trace_limit, Some(7));
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
        assert!(stdout.contains("- sumti"));
        assert!(stdout.contains("- free modifier"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn trace_context_flags_require_trace_or_trace_list() {
        let cases = [
            (
                ["jbotci", "gentufa", "--trace-phase", "syntax", "coi"].as_slice(),
                "`--trace-phase` requires `--trace` or `--trace-list`",
            ),
            (
                ["jbotci", "gentufa", "--trace-limit", "3", "coi"].as_slice(),
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
            "vlasei",
            "--trace-phase",
            "syntax",
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
            "gentufa",
            "--trace-phase",
            "morphology",
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
            Cli::try_parse_from(["jbotci", "gentufa", "--format", "ipa", "coi"])
                .expect_err("IPA is only a vlasei format")
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
        assert!(help.contains("blocks"));
        assert!(help.contains("tree"));
        assert!(help.contains("vipcihe"));
        assert!(!help.contains("compact"));
        assert!(help.contains("raw"));
        assert!(help.contains("--show-defs"));
        assert!(!help.contains("--skicu"));
        assert!(!help.contains("--defs"));
        assert!(help.contains("--indent"));
        assert!(help.contains("--output-type"));
        assert!(help.contains("--output-file"));
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
        assert!(help.contains("ipa"));
        assert!(help.contains("json"));
        assert!(!help.contains("--turtau"));
        assert!(!help.contains("--termoha"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_default_output_matches_bracket_renderer() {
        run_on_normal_stack(|| {
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
    fn gentufa_blocks_stdout_defaults_to_svg() {
        run_on_normal_stack(|| {
            let output =
                run_success_bytes(&["jbotci", "gentufa", "--format", "blocks", "mi", "klama"]);
            let svg = String::from_utf8(output).expect("SVG is UTF-8");
            assert!(svg.starts_with("<svg"));
            assert!(svg.contains("<text"));
            assert!(svg.contains("@font-face"));
            assert!(!svg.ends_with('\n'));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_blocks_output_file_extension_infers_svg_and_png() {
        run_on_normal_stack(|| {
            let svg_path = unique_cli_output_path("gentufa-blocks-inferred-svg", "svg");
            let png_path = unique_cli_output_path("gentufa-blocks-inferred-png", "png");
            let _ = fs::remove_file(&svg_path);
            let _ = fs::remove_file(&png_path);

            let svg_arg = svg_path.to_string_lossy().into_owned();
            let png_arg = png_path.to_string_lossy().into_owned();
            let mut svg_stdout = Vec::new();
            let mut svg_stderr = Vec::new();
            let svg_cli = Cli::try_parse_from([
                "jbotci",
                "gentufa",
                "--format",
                "blocks",
                "--output-file",
                svg_arg.as_str(),
                "mi",
                "klama",
            ])
            .expect("SVG output-file args parse");
            let svg_status =
                run_cli(svg_cli, &mut svg_stdout, &mut svg_stderr, false).expect("SVG run");
            assert_eq!(svg_status, CliStatus::Success);
            assert!(svg_stdout.is_empty());
            assert!(
                svg_stderr.is_empty(),
                "{}",
                String::from_utf8_lossy(&svg_stderr)
            );
            let svg = fs::read_to_string(&svg_path).expect("SVG output file");
            assert!(svg.starts_with("<svg"));

            let mut png_stdout = Vec::new();
            let mut png_stderr = Vec::new();
            let png_cli = Cli::try_parse_from([
                "jbotci",
                "gentufa",
                "--format",
                "blocks",
                "--output-file",
                png_arg.as_str(),
                "mi",
                "klama",
            ])
            .expect("PNG output-file args parse");
            let png_status =
                run_cli(png_cli, &mut png_stdout, &mut png_stderr, false).expect("PNG run");
            assert_eq!(png_status, CliStatus::Success);
            assert!(png_stdout.is_empty());
            assert!(
                png_stderr.is_empty(),
                "{}",
                String::from_utf8_lossy(&png_stderr)
            );
            let png = fs::read(&png_path).expect("PNG output file");
            assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));

            let _ = fs::remove_file(svg_path);
            let _ = fs::remove_file(png_path);
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_blocks_explicit_output_type_wins_over_extension() {
        run_on_normal_stack(|| {
            let path = unique_cli_output_path("gentufa-blocks-explicit-png", "svg");
            let _ = fs::remove_file(&path);
            let path_arg = path.to_string_lossy().into_owned();
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let cli = Cli::try_parse_from([
                "jbotci",
                "gentufa",
                "--format",
                "blocks",
                "--output-type",
                "png",
                "--output-file",
                path_arg.as_str(),
                "mi",
                "klama",
            ])
            .expect("explicit PNG args parse");
            let status = run_cli(cli, &mut stdout, &mut stderr, false).expect("explicit PNG run");
            assert_eq!(status, CliStatus::Success);
            assert!(stdout.is_empty());
            assert!(stderr.is_empty(), "{}", String::from_utf8_lossy(&stderr));
            let png = fs::read(&path).expect("PNG output file");
            assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
            let _ = fs::remove_file(path);
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_blocks_png_stdout_is_binary_without_added_newline() {
        run_on_normal_stack(|| {
            let output = run_success_bytes(&[
                "jbotci",
                "gentufa",
                "--format",
                "blocks",
                "--output-type",
                "png",
                "mi",
                "klama",
            ]);
            assert!(output.starts_with(b"\x89PNG\r\n\x1a\n"));
            assert_ne!(output.last().copied(), Some(b'\n'));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_blocks_unknown_extension_requires_explicit_output_type() {
        let path = unique_cli_output_path("gentufa-blocks-unknown-extension", "dat");
        let path_arg = path.to_string_lossy().into_owned();
        let cli = Cli::try_parse_from([
            "jbotci",
            "gentufa",
            "--format",
            "blocks",
            "--output-file",
            path_arg.as_str(),
            "mi",
            "klama",
        ])
        .expect("unknown extension args parse");
        let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("unknown extension rejected");
        assert!(
            error
                .to_string()
                .contains("cannot infer gentufa blocks output type")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_blocks_rejects_text_only_options() {
        assert_gentufa_error_contains(
            &[
                "jbotci",
                "gentufa",
                "--format",
                "blocks",
                "--show-defs",
                "mi",
                "klama",
            ],
            "`--show-defs`",
        );
        assert_gentufa_error_contains(
            &[
                "jbotci", "gentufa", "--format", "blocks", "--indent", "2", "mi", "klama",
            ],
            "`--indent`",
        );
        assert_gentufa_error_contains(
            &[
                "jbotci",
                "gentufa",
                "--format",
                "blocks",
                "--show-spans",
                "mi",
                "klama",
            ],
            "`--show-spans`",
        );
        assert_gentufa_error_contains(
            &[
                "jbotci",
                "gentufa",
                "--format",
                "blocks",
                "--show-refs",
                "mi",
                "klama",
            ],
            "`--show-refs`",
        );
        assert_gentufa_error_contains(
            &[
                "jbotci",
                "gentufa",
                "--format",
                "blocks",
                "--decompose-lujvo",
                "mi",
                "klama",
            ],
            "`--decompose-lujvo`",
        );
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

        assert_eq!(value[0]["PlainWord"]["Cmavo"]["phonemes"], "coĭ");
        assert_eq!(
            value[0]["PlainWord"]["Cmavo"]["span"],
            serde_json::json!([0, 3])
        );
        assert!(
            String::from_utf8(output)
                .expect("utf8")
                .contains("\"PlainWord\"")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_ipa_outputs_pronunciation_surface() {
        let cli = Cli::try_parse_from([
            "jbotci", "vlasei", "--format", "ipa", "mi", "klama", "le", "zarci",
        ])
        .expect("vlasei IPA");
        let mut output = Vec::new();
        let mut error = Vec::new();
        run_cli(cli, &mut output, &mut error, false).expect("vlasei IPA run");

        assert!(error.is_empty());
        assert_eq!(
            String::from_utf8(output).expect("stdout utf8"),
            "mi ˈkla.ma le ˈzar.ʃi\n"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_cgv_warning_keeps_json_stdout_clean() {
        let cli = Cli::try_parse_from(["jbotci", "vlasei", "--format", "json", "siatl."])
            .expect("vlasei json");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("vlasei run");

        assert_eq!(status, CliStatus::Success);
        let stdout = String::from_utf8(output).expect("stdout utf8");
        let _json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("morphology.warning.experimental-cgv"));
        assert!(stderr.contains("experimental morphology"));
        assert!(!stdout.contains("morphology.warning.experimental-cgv"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_mz_warning_keeps_json_stdout_clean() {
        let cli = Cli::try_parse_from(["jbotci", "vlasei", "--format", "json", "namzi"])
            .expect("vlasei json");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("vlasei run");

        assert_eq!(status, CliStatus::Success);
        let stdout = String::from_utf8(output).expect("stdout utf8");
        let _json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("morphology.warning.experimental-mz"));
        assert!(stderr.contains("experimental morphology: MZ consonant pair"));
        assert!(!stdout.contains("morphology.warning.experimental-mz"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_detailed_error_reports_xlaglymlu_lujvo_progress() {
        let run = run_cli_capture(
            &["jbotci", "vlasei", "--detailed-errors", "xlaglymlu"],
            false,
        );

        assert_eq!(run.status, CliStatus::Failure);
        assert!(run.stdout.is_empty());
        assert!(run.stderr.contains("morphology.invalid-lujvo"));
        assert!(run.stderr.contains("after parsing"));
        assert!(run.stderr.contains("`xla`"));
        assert!(!run.stderr.contains("morphology.slinkuhi"));
        assert!(
            !run.stderr
                .contains("reason: word is not a valid Lojban word")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_detailed_error_reports_zoi_delimiter_reason() {
        let run = run_cli_capture(&["jbotci", "vlasei", "--detailed-errors", "zoi"], false);

        assert_eq!(run.status, CliStatus::Failure);
        assert!(run.stdout.is_empty());
        assert!(run.stderr.contains("morphology.invalid-zoi-delimiter"));
        assert!(run.stderr.contains("ZOI requires an"));
        let compact_stderr = run.stderr.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(compact_stderr.contains("opening delimiter word after the quote marker"));
        assert!(!compact_stderr.contains("reason: ZOI delimiter must be a single non-y word"));
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
        assert!(output.contains("PlainWord("));
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
        assert!(output.starts_with("[PlainWord("));
        assert!(output.contains("PlainWord("));
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
    fn ascii_rejects_incompatible_diacritic_flags() {
        let stress_cli = Cli::try_parse_from([
            "jbotci",
            "gentufa",
            "--ascii",
            "--format",
            "tree",
            "--mark-stress",
            "acute",
            "mi",
            "klama",
        ])
        .expect("ASCII stress conflict parses");
        let error = run_cli(stress_cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("ASCII stress conflict rejected");
        assert!(error.to_string().contains("`--ascii`"));
        assert!(error.to_string().contains("`--mark-stress acute`"));

        let glide_cli = Cli::try_parse_from([
            "jbotci",
            "vlasei",
            "--ascii",
            "--format",
            "tree",
            "--mark-glides",
            "breve",
            "coi",
        ])
        .expect("ASCII glide conflict parses");
        let error = run_cli(glide_cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("ASCII glide conflict rejected");
        assert!(error.to_string().contains("`--mark-glides breve`"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_ipa_rejects_ascii_output() {
        let cli = Cli::try_parse_from([
            "jbotci", "vlasei", "--ascii", "--format", "ipa", "mi", "klama",
        ])
        .expect("vlasei IPA ASCII parses");
        let error =
            run_cli(cli, &mut Vec::new(), &mut Vec::new(), false).expect_err("ASCII IPA rejected");

        assert!(error.to_string().contains("`--ascii`"));
        assert!(error.to_string().contains("`--turtai ipa`"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_ipa_rejects_phoneme_projection_flags() {
        let cli = Cli::try_parse_from([
            "jbotci",
            "vlasei",
            "--format",
            "ipa",
            "--mark-stress",
            "none",
            "mi",
            "klama",
        ])
        .expect("vlasei IPA projection flag parses");
        let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("IPA projection flags rejected");

        assert!(error.to_string().contains("`--mark-stress`"));
        assert!(error.to_string().contains("IPA output"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ascii_accepts_compatible_diacritic_flags() {
        let output = run_success_stdout(&[
            "jbotci",
            "gentufa",
            "--ascii",
            "--format",
            "tree",
            "--mark-stress",
            "none",
            "--mark-glides",
            "none",
            "mi",
            "klama",
        ]);

        assert!(output.contains("Gismu \"klama\""));
        assert!(!output.contains("kláma"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ascii_affects_human_and_json_outputs() {
        let gentufa_tree = run_success_stdout(&[
            "jbotci",
            "gentufa",
            "--ascii",
            "--format",
            "tree",
            "--show-spans",
            "--show-refs",
            "mi",
            "klama",
            "do",
        ]);
        assert!(gentufa_tree.contains("k<1>-> Cmavo @[0..2) \"mi\""));
        assert!(gentufa_tree.contains("Gismu @[3..8) \"klama\" ->k"));
        assert!(!gentufa_tree.contains('→'));
        assert!(!gentufa_tree.contains('‥'));
        assert!(!gentufa_tree.contains('á'));

        let gentufa_brackets = run_success_stdout(&[
            "jbotci",
            "gentufa",
            "--ascii",
            "--format",
            "brackets",
            "--decompose-lujvo",
            "mivyselbai",
        ]);
        assert!(gentufa_brackets.contains("miv~y~sel~bai"));

        let gentufa_json = run_success_stdout(&[
            "jbotci", "gentufa", "--ascii", "--format", "json", "coi", "klama",
        ]);
        assert!(gentufa_json.contains("\"phonemes\": \"coi\""));
        assert!(gentufa_json.contains("\"phonemes\": \"klama\""));

        let vlasei_tree = run_success_stdout(&[
            "jbotci",
            "vlasei",
            "--ascii",
            "--format",
            "tree",
            "--show-spans",
            "coi",
            "klama",
        ]);
        assert!(vlasei_tree.contains("Cmavo @[0..3) \"coi\""));
        assert!(vlasei_tree.contains("Gismu @[4..9) \"klama\""));

        let vlasei_brackets = run_success_stdout(&[
            "jbotci",
            "vlasei",
            "--ascii",
            "--format",
            "brackets",
            "--decompose-lujvo",
            "mivyselbai",
        ]);
        assert!(vlasei_brackets.contains("miv~y~sel~bai"));

        let vlasei_json = run_success_stdout(&[
            "jbotci", "vlasei", "--ascii", "--format", "json", "coi", "klama",
        ]);
        assert!(vlasei_json.contains("\"phonemes\": \"coi\""));
        assert!(vlasei_json.contains("\"phonemes\": \"klama\""));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn show_refs_is_tree_only() {
        let cli = Cli::try_parse_from([
            "jbotci",
            "gentufa",
            "--format",
            "brackets",
            "--show-refs",
            "mi",
            "klama",
        ])
        .expect("gentufa show refs flag parses");
        let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("show refs rejected for non-tree output");
        assert!(error.to_string().contains("`--show-refs`"));
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
        run_on_normal_stack(|| {
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
            assert!(text.contains("\"Bridi\""));
            assert!(!text.contains("\"constructor\""));
            assert!(!text.contains("\"kind\": \"node\""));
            assert!(!text.contains("\"leadingNai\""));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_morphology_warnings_go_to_stderr() {
        run_on_normal_stack(|| {
            let cli = Cli::try_parse_from([
                "jbotci", "gentufa", "--format", "json", "la", "siatl.", "cu", "klama",
            ])
            .expect("gentufa json");
            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, false).expect("gentufa run");

            let stdout = String::from_utf8(output).expect("stdout utf8");
            let _json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
            let stderr = String::from_utf8(error).expect("stderr utf8");
            assert!(stderr.contains("morphology.warning.experimental-cgv"));
            assert!(stderr.contains("experimental morphology"));
            assert!(!stdout.contains("morphology.warning.experimental-cgv"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_tree_outputs_collapsed_syntax_tree() {
        run_on_normal_stack(|| {
            let cli = Cli::try_parse_from(["jbotci", "gentufa", "--format", "tree", "mi", "klama"])
                .expect("gentufa tree");
            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, false).expect("gentufa tree run");
            assert!(error.is_empty());
            let output = String::from_utf8(output).expect("utf8");

            assert!(output.starts_with("Bridi {\n"));
            assert!(output.contains("\n  leading_terms: [\n    Cmavo \"mi\""));
            assert!(output.contains("leading_terms: ["));
            assert!(output.contains("Gismu \"kláma\""));
            assert!(!output.contains("Text {"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_tree_preserves_source_order_for_selbri_connection() {
        run_on_normal_stack(|| {
            let cli = Cli::try_parse_from([
                "jbotci", "gentufa", "--format", "tree", "gleki", "je", "klama",
            ])
            .expect("gentufa tree");
            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, false).expect("gentufa tree run");
            assert!(error.is_empty());
            let output = String::from_utf8(output).expect("utf8");

            let leading = output.find("leading_selbri").expect("leading selbri");
            let connective = output.find("connective").expect("connective");
            let trailing = output.find("trailing_selbri").expect("trailing selbri");
            assert!(leading < connective);
            assert!(connective < trailing);
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_tree_preserves_source_order_for_binary_math() {
        run_on_normal_stack(|| {
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
        run_on_normal_stack(|| {
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
                r#"Bridi{leading_terms:[Cmavo "mi"],Gismu "kláma"}"#
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_indent_zero_makes_json_single_line() {
        run_on_normal_stack(|| {
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
        run_on_normal_stack(|| {
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
            assert!(stderr.contains("FIhOI bridi/subbridi adverbial term"));
            assert!(stderr.contains("fi'oi"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_syntax_errors_go_to_stderr() {
        run_on_normal_stack(|| {
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
            assert!(stderr.contains("expected: free modifier, statement, or end of input"));
            assert!(!stderr.contains("expected one of:"));
            assert!(!stderr.contains("needs one of:"));
            assert!(!stderr.contains("{be}"));
            assert!(!stderr.contains("BRIVLA"));
            assert!(stderr.contains("ku"));
            assert!(!stderr.contains("jbotci:"));
            assert!(!stderr.contains("\x1b["));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_syntax_error_uses_explicit_diagnostic_width() {
        run_on_normal_stack(|| {
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
            assert!(stderr.contains("expected: free modifier, statement, or end of input"));
            assert!(!stderr.contains("expected one of:"));
            assert!(stderr.contains("\n            "));
            assert!(!stderr.contains("\x1b["));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_detailed_syntax_errors_show_expectation_breakdown() {
        run_on_normal_stack(|| {
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
            assert!(stderr.contains("selbri"));
            assert!(stderr.contains("{be}"));
            assert!(stderr.contains("BRIVLA"));
            let compact_stderr = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(compact_stderr.contains("[ends selbri, bridi, statement, or text]"));
            assert!(!stderr.contains("end of input (end of input)"));
            let free_modifier = compact_stderr
                .find("- metalinguistic comment")
                .expect("free modifier subtype group");
            let sumti = compact_stderr.find("- sumti").expect("sumti group");
            let selbri = compact_stderr
                .find("continues selbri]")
                .expect("selbri continuation group");
            let end = compact_stderr
                .find("ends selbri, bridi")
                .expect("end group");
            assert!(free_modifier < sumti);
            assert!(sumti < selbri);
            assert!(selbri < end);
            assert!(!stderr.contains("\x1b["));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_syntax_error_labels_unique_current_construct() {
        run_on_normal_stack(|| {
            let cli = Cli::try_parse_from(["jbotci", "gentufa", "--detailed-errors", "mi", "cu"])
                .expect("gentufa detailed parses");
            let mut output = Vec::new();
            let mut error = Vec::new();
            let status = run_cli(cli, &mut output, &mut error, false).expect("gentufa run");

            assert_eq!(status, CliStatus::Failure);
            assert!(output.is_empty());
            let stderr = String::from_utf8(error).expect("stderr utf8");
            assert!(stderr.contains("while parsing bridi"), "{stderr}");
            assert_eq!(stderr.matches("while parsing").count(), 1, "{stderr}");
            assert!(stderr.contains("mi cu"));
            assert!(stderr.contains("needs one of:"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_trace_writes_to_stderr_and_keeps_json_stdout_clean() {
        run_on_normal_stack(|| {
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
    fn gentufa_show_elided_renders_tree_and_json_terminators() {
        run_on_normal_stack(|| {
            let tree_cli = Cli::try_parse_from([
                "jbotci",
                "gentufa",
                "--show-elided",
                "--turtai",
                "tree",
                "--show-spans",
                "mi",
                "klama",
            ])
            .expect("gentufa tree parses");
            let mut tree_output = Vec::new();
            let mut tree_error = Vec::new();
            let tree_status =
                run_cli(tree_cli, &mut tree_output, &mut tree_error, false).expect("gentufa tree");

            assert_eq!(tree_status, CliStatus::Success);
            assert!(tree_error.is_empty());
            let tree_stdout = String::from_utf8(tree_output).expect("tree stdout utf8");
            assert!(tree_stdout.contains("vau: Cmavo @[8‥8) \"vau\""));

            let json_cli = Cli::try_parse_from([
                "jbotci",
                "gentufa",
                "--show-elided",
                "--turtai",
                "json",
                "mi",
                "klama",
            ])
            .expect("gentufa json parses");
            let mut json_output = Vec::new();
            let mut json_error = Vec::new();
            let json_status =
                run_cli(json_cli, &mut json_output, &mut json_error, false).expect("gentufa json");

            assert_eq!(json_status, CliStatus::Success);
            assert!(json_error.is_empty());
            let json_stdout = String::from_utf8(json_output).expect("json stdout utf8");
            assert!(json_stdout.contains("\"phonemes\": \"vau\""));
            assert!(json_stdout.contains("\"span\": [8, 8]"));
            assert!(json_stdout.contains("\"elided\": true"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn bare_trace_before_text_uses_default_trace_level() {
        run_on_normal_stack(|| {
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
        run_on_normal_stack(|| {
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
        run_on_normal_stack(|| {
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

        assert_eq!(status, CliStatus::Success);
        assert!(!output.is_empty());
        let stderr = String::from_utf8(error).expect("stderr utf8");
        assert!(stderr.contains("trace[morphology]"), "{stderr}");
        assert!(
            stderr.contains("morphology.warning.experimental-cgv"),
            "{stderr}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn warning_context_includes_verbatim_quote_text() {
        run_on_normal_stack(|| {
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
        run_on_normal_stack(|| {
            let cli = Cli::try_parse_from(["jbotci", "gentufa", "--turtai", "raw", "mi", "klama"])
                .expect("gentufa raw");
            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, false).expect("gentufa run");
            assert!(error.is_empty());
            let output = String::from_utf8(output).expect("utf8");
            assert!(output.contains("TextSyntax"));
            assert!(output.contains("BridiSyntax"));
            assert!(!output.contains("SyntaxValue"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_raw_indent_zero_uses_compact_debug() {
        run_on_normal_stack(|| {
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
            assert!(output.contains("BridiSyntax"));
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
        let zantufa_cli =
            Cli::try_parse_from(["jbotci", "gerna", "--dialect", "(+ZANTUFA-QUOTES)"])
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
    fn gentufa_show_defs_prepends_dictionary_cards() {
        let output = run_success_stdout(&[
            "jbotci",
            "gentufa",
            "--show-defs",
            "--color=never",
            "mi",
            "klama",
        ]);
        assert!(output.starts_with("1. mi | by: officialdata | cmavo: KOhA3"));
        assert!(output.contains("\n2. klama | by: officialdata | gismu"));
        assert!(output.contains("  definitions:"));
        assert!(output.contains("\n\n(mi kl"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_show_defs_works_for_non_bracket_formats() {
        for format in ["raw", "tree", "json"] {
            let output = run_success_stdout(&[
                "jbotci",
                "gentufa",
                "--show-defs",
                "--format",
                format,
                "--color=never",
                "mi",
                "klama",
            ]);
            assert!(
                output.starts_with("1. mi | by: officialdata | cmavo: KOhA3"),
                "{format}"
            );
            assert!(
                output.contains("\n2. klama | by: officialdata | gismu"),
                "{format}"
            );
            assert!(output.contains("\n\n"), "{format}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_old_definition_flags_are_removed() {
        assert_eq!(
            Cli::try_parse_from(["jbotci", "gentufa", "--defs", "mi", "klama"])
                .expect_err("defs flag removed")
                .kind(),
            ErrorKind::UnknownArgument
        );
        assert_eq!(
            Cli::try_parse_from(["jbotci", "gentufa", "--skicu", "mi", "klama"])
                .expect_err("skicu flag removed")
                .kind(),
            ErrorKind::UnknownArgument
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_color_flag_forces_ansi_bracket_output() {
        run_on_normal_stack(|| {
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
        run_on_normal_stack(|| {
            let cli = Cli::try_parse_from([
                "jbotci", "gentufa", "--color", "--format", "vipcihe", "mi", "klama",
            ])
            .expect("gentufa tree color");
            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, false).expect("gentufa tree color run");
            assert!(error.is_empty());
            let output = String::from_utf8(output).expect("utf8");
            assert!(output.contains("\x1b[94mBridi\x1b[39m"));
            assert!(output.contains("\x1b[94mCmavo\x1b[39m"));
            assert!(output.contains("\x1b[33m\"mi\"\x1b[39m"));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gentufa_runs_reported_color_case_on_normal_cli_stack() {
        run_on_normal_stack(|| {
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
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn color_never_disables_ansi_output() {
        run_on_normal_stack(|| {
            let cli = Cli::try_parse_from(["jbotci", "gentufa", "--color=never", "mi", "klama"])
                .expect("gentufa color never");
            assert_eq!(cli.color, concolor_clap::ColorChoice::Never);

            let mut output = Vec::new();
            let mut error = Vec::new();
            run_cli(cli, &mut output, &mut error, true).expect("gentufa color never run");

            let output = String::from_utf8(output).expect("utf8");
            assert!(!output.contains("\x1b["));
            assert!(error.is_empty());
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn json_colorizer_distinguishes_keys_from_string_values() {
        let output = colorize_json(r#"{"key":"value","Bridi":{}}"#, true);
        assert!(output.contains("\x1b[32m\"key\"\x1b[39m"));
        assert!(output.contains("\x1b[33m\"value\"\x1b[39m"));
        assert!(output.contains("\x1b[94m\"Bridi\"\x1b[39m"));
        assert!(output.contains("\x1b[90m{\x1b[39m"));
        assert!(output.contains("\x1b[90m}\x1b[39m"));
        assert!(!output.contains("\x1b[36m"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_exact_found_outputs_dictionary_card() {
        let run = run_cli_capture(&["jbotci", "vlacku", "--valsi", "klama"], false);

        assert_eq!(run.status, CliStatus::Success);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(
            run.stdout
                .contains("1. klama | by: officialdata | gismu | similarity: 100% | votes: ∞")
        );
        assert!(run.stdout.contains("  rafsi: "));
        assert!(run.stdout.contains("  glosses:"));
        assert!(run.stdout.contains("  definitions:"));

        for query in ["шой", "\u{ed86}\u{eda8}"] {
            let run = run_cli_capture(&["jbotci", "vlacku", "--valsi", query], false);

            assert_eq!(run.status, CliStatus::Success, "{query}");
            assert!(run.stderr.is_empty(), "{}", run.stderr);
            assert!(
                run.stdout
                    .contains("1. coi | by: officialdata | cmavo: COI | similarity: 100%"),
                "{query}: {}",
                run.stdout
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_section_fetch_outputs_default_section() {
        let run = run_cli_capture(
            &["jbotci", "cukta", "--section", "section-what-is-lojban"],
            false,
        );

        assert_eq!(run.status, CliStatus::Success);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(run.stdout.starts_with("# 1.1. What is Lojban?"));
        assert!(run.stdout.contains("Lojban (pronounced"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_book_alias_exact_word_search_uses_tagged_content() {
        let run = run_cli_capture(&["jbotci", "book", "--valsi", "lojban", "-n", "3"], false);

        assert_eq!(run.status, CliStatus::Success);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert_in_order(
            &run.stdout,
            &[
                "### 1. 4.3. brivla",
                "### 2. Paragraph in 4.3. brivla",
                "### 3.",
            ],
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_exact_word_search_accepts_non_latin_query() {
        for query in ["шой", "\u{ed86}\u{eda8}"] {
            let run = run_cli_capture(&["jbotci", "cukta", "--valsi", query, "-n", "3"], false);

            assert_eq!(run.status, CliStatus::Success, "{query}");
            assert!(run.stderr.is_empty(), "{}", run.stderr);
            assert!(
                run.stdout.contains("the cmavo coi means hello"),
                "{query}: {}",
                run.stdout
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_toc_outputs_table_of_contents() {
        let run = run_cli_capture(&["jbotci", "cukta", "--toc"], false);

        assert_eq!(run.status, CliStatus::Success);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(run.stdout.starts_with("# Table of Contents"));
        assert!(run.stdout.contains("1.1. What is Lojban?"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cukta_semantic_search_reports_missing_setup() {
        let run = run_cli_capture_with_embedding_dirs(
            &["jbotci", "cukta", "lojban"],
            false,
            &unique_embedding_test_path("cukta-model-missing"),
            &unique_embedding_test_path("cukta-index-missing"),
        );

        assert_eq!(run.status, CliStatus::InvalidInput);
        assert!(run.stdout.is_empty(), "{}", run.stdout);
        assert!(run.stderr.contains("jbotci setup --embedding"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_semantic_search_reports_missing_setup() {
        let run = run_cli_capture_with_embedding_dirs(
            &["jbotci", "vlacku", "language"],
            false,
            &unique_embedding_test_path("vlacku-model-missing"),
            &unique_embedding_test_path("vlacku-index-missing"),
        );

        assert_eq!(run.status, CliStatus::InvalidInput);
        assert!(run.stdout.is_empty(), "{}", run.stdout);
        assert!(run.stderr.contains("jbotci setup --embedding"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn setup_embedding_requires_a_setup_task() {
        let cli = Cli::try_parse_from(["jbotci", "setup"]).expect("setup parses");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let result = run_cli(cli, &mut output, &mut error, false);

        assert!(result.is_err());
        assert!(output.is_empty());
        assert!(error.is_empty());
        assert!(
            result
                .expect_err("setup without task fails")
                .to_string()
                .contains("jbotci setup --embedding")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn setup_embedding_rejects_unknown_model_without_download() {
        let cli =
            Cli::try_parse_from(["jbotci", "setup", "--embedding", "--model", "unknown-model"])
                .expect("setup parses");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let result = run_cli(cli, &mut output, &mut error, false);

        assert!(result.is_err());
        assert!(output.is_empty());
        assert!(error.is_empty());
        assert!(
            result
                .expect_err("unknown model fails")
                .to_string()
                .contains("unsupported embedding model")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jvozba_outputs_best_lujvo_word() {
        let run = run_cli_capture(&["jbotci", "jvozba", "lojbo", "bangu"], false);

        assert_eq!(run.status, CliStatus::Success);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert_eq!(run.stdout, "jbobau\n");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jvozba_accepts_fixed_rafsi_and_cmevla_mode() {
        let run = run_cli_capture(
            &["jbotci", "jvozba", "--cmevla", "lojbo", "--rafsi", "bau"],
            false,
        );

        assert_eq!(run.status, CliStatus::Success);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert_eq!(run.stdout, "jbobaus\n");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jvozba_rejects_option_like_positional_rafsi() {
        let error = Cli::try_parse_from(["jbotci", "jvozba", "lojbo", "-bau-"])
            .expect_err("fixed rafsi marker is not positional syntax");

        assert_eq!(error.kind(), ErrorKind::UnknownArgument);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jvozba_rejects_unsupported_flags_with_clap() {
        let error = Cli::try_parse_from(["jbotci", "jvozba", "--detailed-errors", "lojbo"])
            .expect_err("jvozba does not expose detailed errors");

        assert_eq!(error.kind(), ErrorKind::UnknownArgument);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jvozba_help_only_lists_supported_options() {
        let error = Cli::try_parse_from(["jbotci", "jvozba", "--help"]).expect_err("help");
        assert_eq!(error.kind(), ErrorKind::DisplayHelp);
        let help = error.to_string();

        assert!(help.contains("--cmevla"));
        assert!(help.contains("--rafsi"));
        assert!(help.contains("--color"));
        assert!(!help.contains("--detailed-errors"));
        assert!(!help.contains("--trace-phase"));
        assert!(!help.contains("--trace-list"));
        assert!(!help.contains("--ascii"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jvozba_colorizes_segments_when_requested() {
        let run = run_cli_capture(&["jbotci", "jvozba", "--color", "lojbo", "bangu"], false);

        assert_eq!(run.status, CliStatus::Success);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(run.stdout.contains("\x1b[32mjbo\x1b[39m"));
        assert!(run.stdout.contains("\x1b[35mbau\x1b[39m"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jvozba_colorizes_cmevla_suffix_like_hyphen() {
        let run = run_cli_capture(
            &[
                "jbotci", "jvozba", "--color", "--cmevla", "birti", "--rafsi", "zba",
            ],
            false,
        );

        assert_eq!(run.status, CliStatus::Success);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(
            run.stdout.contains("\x1b[32mbit\x1b[39m"),
            "{:?}",
            run.stdout
        );
        assert!(run.stdout.contains("\x1b[90my\x1b[39m"), "{:?}", run.stdout);
        assert!(
            run.stdout.contains("\x1b[35mzba\x1b[39m"),
            "{:?}",
            run.stdout
        );
        assert!(run.stdout.contains("\x1b[90ms\x1b[39m"), "{:?}", run.stdout);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jvozba_colorizes_cmevla_final_consonant_rafsi_as_rafsi() {
        let run = run_cli_capture(
            &["jbotci", "jvozba", "--color", "--cmevla", "cmene", "valsi"],
            false,
        );

        assert_eq!(run.status, CliStatus::Success);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(run.stdout.contains("\x1b[32mcme\x1b[39m"));
        assert!(run.stdout.contains("\x1b[35mval\x1b[39m"));
        assert!(!run.stdout.contains("\x1b[35mva\x1b[39m\x1b[90ml\x1b[39m"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jvozba_errors_match_v0_text() {
        let cli = Cli::try_parse_from(["jbotci", "jvozba", "lojbo"]).expect("jvozba args");
        let error =
            run_cli(cli, &mut Vec::new(), &mut Vec::new(), false).expect_err("jvozba error");

        assert_eq!(
            error.to_string(),
            "jvozba requires at least two rafsi-producing inputs."
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_exact_valid_missing_outputs_classification_card() {
        let run = run_cli_capture(&["jbotci", "vlacku", "--valsi", "brodax"], false);

        assert_eq!(run.status, CliStatus::ValidMissing);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(run.stdout.contains("1. brodax | cmevla"));
        assert!(!run.stdout.contains("  rafsi:"));
        assert!(!run.stdout.contains("  glosses:"));
        assert!(!run.stdout.contains("  definitions:"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_exact_invalid_word_reports_invalid_input_status() {
        let run = run_cli_capture(&["jbotci", "vlacku", "--valsi", "aa"], false);

        assert_eq!(run.status, CliStatus::InvalidInput);
        assert!(run.stdout.is_empty(), "{}", run.stdout);
        assert!(run.stderr.contains("Invalid Lojban word: aa"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_rafsi_lookup_returns_source_entry() {
        let run = run_cli_capture(&["jbotci", "vlacku", "--rafsi", "kla"], false);

        assert_eq!(run.status, CliStatus::Success);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(
            run.stdout
                .contains("1. klama | by: officialdata | gismu | similarity: 100% | votes: ∞")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_lujvo_outputs_headword_decomposition_then_sources() {
        let run = run_cli_capture(
            &["jbotci", "vlacku", "--ascii", "--lujvo", "mivyselbai"],
            false,
        );

        assert_eq!(run.status, CliStatus::ValidMissing);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(run.stdout.contains("1. mivyselbai | lujvo"));
        assert!(run.stdout.contains("  decomposition: miv~y~sel~bai"));
        assert_in_order(
            &run.stdout,
            &[
                "1. mivyselbai | lujvo",
                "2. jmive | by: officialdata | gismu",
                "3. se | by: officialdata | cmavo: SE",
                "4. bapli | by: officialdata | gismu",
            ],
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_decompose_lujvo_adds_decomposition_to_exact_lujvo_cards() {
        let run = run_cli_capture(
            &[
                "jbotci",
                "vlacku",
                "--ascii",
                "--decompose-lujvo",
                "--valsi",
                "mivyselbai",
            ],
            false,
        );

        assert_eq!(run.status, CliStatus::ValidMissing);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(run.stdout.contains("1. mivyselbai | lujvo"));
        assert!(run.stdout.contains("  decomposition: miv~y~sel~bai"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_glob_matches_and_rejects_reserved_uppercase() {
        let found = run_cli_capture(
            &["jbotci", "vlacku", "--glob", "klamV", "--count", "1"],
            false,
        );
        assert_eq!(found.status, CliStatus::Success);
        assert!(found.stdout.contains("1. klama | by: officialdata | gismu"));

        let invalid = run_cli_capture(&["jbotci", "vlacku", "--glob", "K"], false);
        assert_eq!(invalid.status, CliStatus::InvalidInput);
        assert!(invalid.stderr.contains("uppercase `K` is reserved"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_filters_can_turn_hits_into_no_hit_status() {
        let run = run_cli_capture(
            &[
                "jbotci",
                "vlacku",
                "--valsi",
                "klama",
                "--word-type",
                "cmavo",
            ],
            false,
        );

        assert_eq!(run.status, CliStatus::ValidMissing);
        assert_eq!(run.stdout, "No matches found.\n");
        assert!(run.stderr.is_empty(), "{}", run.stderr);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_official_author_low_score_renders_official_marker() {
        let run = run_cli_capture(&["jbotci", "vlacku", "--valsi", "birka"], false);

        assert_eq!(run.status, CliStatus::Success);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(
            run.stdout
                .contains("1. birka | by: officialdata | gismu | similarity: 100% | votes: ∞"),
            "{}",
            run.stdout
        );
        assert!(!run.stdout.contains("votes: +10000"), "{}", run.stdout);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_ascii_renders_official_author_marker_as_ascii() {
        let run = run_cli_capture(&["jbotci", "vlacku", "--ascii", "--valsi", "birka"], false);

        assert_eq!(run.status, CliStatus::Success);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(run.stdout.contains("votes: official"), "{}", run.stdout);
        assert!(!run.stdout.contains('∞'), "{}", run.stdout);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_hides_etymology_by_default() {
        let run = run_cli_capture(&["jbotci", "vlacku", "--valsi", "abniena"], false);

        assert_eq!(run.status, CliStatus::Success);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(!run.stdout.contains("etymology:"), "{}", run.stdout);
        assert!(run.stdout.contains("Guaraní in aspect"), "{}", run.stdout);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_show_etymology_renders_etymology_section() {
        let run = run_cli_capture(
            &["jbotci", "vlacku", "--show-etymology", "--valsi", "abniena"],
            false,
        );

        assert_eq!(run.status, CliStatus::Success);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(run.stdout.contains("  etymology:"), "{}", run.stdout);
        assert!(run.stdout.contains("ava, people"), "{}", run.stdout);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_sound_search_accepts_bracketed_ipa_and_orders_by_similarity() {
        let run = run_cli_capture(
            &[
                "jbotci",
                "vlacku",
                "--sound",
                "[ˈkla.ma]",
                "--count",
                "3",
                "--min-similarity",
                "90",
            ],
            false,
        );

        assert_eq!(run.status, CliStatus::Success);
        assert!(run.stderr.is_empty(), "{}", run.stderr);
        assert!(
            run.stdout
                .contains("1. klama | by: officialdata | gismu | similarity: 100%")
        );
        assert!(
            run.stdout
                .contains("2. klani | by: officialdata | gismu | similarity: 92%")
        );
        assert!(
            run.stdout
                .contains("3. klina | by: officialdata | gismu | similarity: 92%")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_colors_card_labels_dividers_and_rich_text() {
        let output = render_vlacku_output_with_options(
            &VlackuSearchOutput {
                cards: vec![VlackuCard {
                    word: "klama".to_owned(),
                    word_type: "gismu".to_owned(),
                    selmaho: None,
                    author: None,
                    is_official: false,
                    similarity: Some(1.0),
                    votes: Some(7),
                    rafsi: vec!["kla".to_owned()],
                    glosses: vec!["come".to_owned()],
                    definition: "references {cadzu} at $x_1$; malformed {bad link}.".to_owned(),
                    notes: "unmatched $ remains plain".to_owned(),
                    etymology: None,
                    decomposition: Vec::new(),
                }],
                outcome: VlackuOutcome::Found,
                diagnostics: Vec::new(),
            },
            new!(VlackuRenderOptions {
                color: true,
                glyphs: GlyphStyle::Unicode,
                output_terminal_width: None,
                sumti_places: CliSumtiPlaces::Index,
                show_etymology: false,
            }),
        );

        assert!(output.contains("\x1b[90m1.\x1b[39m"));
        assert!(output.contains("\x1b[4m\x1b[33mklama"), "{output}");
        assert!(output.contains("\x1b[90m | \x1b[39m"));
        assert!(output.contains("\x1b[90msimilarity: \x1b[39m\x1b[35m100%\x1b[39m"));
        assert!(output.contains("\x1b[90mvotes: \x1b[39m\x1b[32m+7\x1b[39m"));
        assert!(output.contains("\x1b[90mrafsi: \x1b[39m\x1b[31mkla\x1b[39m"));
        assert!(output.contains("\x1b[90m{\x1b[39m\x1b[33mcadzu\x1b[39m\x1b[90m}\x1b[39m"));
        assert!(!output.contains("\x1b[4mcadzu"), "{output}");
        assert!(
            output.contains("\x1b[90m⟨\x1b[39m\x1b[36m1\x1b[39m\x1b[90m⟩\x1b[39m"),
            "{output}"
        );
        assert!(output.contains("\x1b[37m{bad link}\x1b[39m"));
        assert!(output.contains("\x1b[37munmatched $ remains plain\x1b[39m"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_raw_sumti_places_keep_dollar_spans_and_color_equals() {
        let output = render_vlacku_output_with_options(
            &VlackuSearchOutput {
                cards: vec![VlackuCard {
                    word: "klama".to_owned(),
                    word_type: "gismu".to_owned(),
                    selmaho: None,
                    author: None,
                    is_official: false,
                    similarity: Some(1.0),
                    votes: Some(7),
                    rafsi: Vec::new(),
                    glosses: Vec::new(),
                    definition: "$x_2=b_1$ moves to $x_3$.".to_owned(),
                    notes: String::new(),
                    etymology: None,
                    decomposition: Vec::new(),
                }],
                outcome: VlackuOutcome::Found,
                diagnostics: Vec::new(),
            },
            new!(VlackuRenderOptions {
                color: true,
                glyphs: GlyphStyle::Unicode,
                output_terminal_width: None,
                sumti_places: CliSumtiPlaces::Raw,
                show_etymology: false,
            }),
        );

        assert!(output.contains(
            "\x1b[90m$\x1b[39m\x1b[36mx_2\x1b[39m\x1b[90m=\x1b[39m\x1b[36mb_1\x1b[39m\x1b[90m$\x1b[39m"
        ));
        assert!(output.contains("\x1b[90m$\x1b[39m\x1b[36mx_3\x1b[39m\x1b[90m$\x1b[39m"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_terminal_width_wraps_long_detail_lines_with_indent() {
        let output = render_vlacku_output_with_width(
            &VlackuSearchOutput {
                cards: vec![VlackuCard {
                    word: "cmevla".to_owned(),
                    word_type: "lujvo".to_owned(),
                    selmaho: None,
                    author: None,
                    is_official: false,
                    similarity: Some(1.0),
                    votes: Some(4),
                    rafsi: Vec::new(),
                    glosses: Vec::new(),
                    definition: "$x_1$ is a morphologically defined name word meaning $x_2$ in language $x_3$.".to_owned(),
                    notes: "In Lojban, such words are characterized by ending with a consonant.".to_owned(),
                    etymology: None,
                    decomposition: Vec::new(),
                }],
                outcome: VlackuOutcome::Found,
                diagnostics: Vec::new(),
            },
            false,
            GlyphStyle::Unicode,
            Some(48),
        );

        assert!(
            output.contains(
                "    ⟨1⟩ is a morphologically defined name word\n    meaning ⟨2⟩ in language ⟨3⟩."
            ),
            "{output}"
        );
        assert!(
            output.contains(
                "    In Lojban, such words are characterized by\n    ending with a consonant."
            ),
            "{output}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_official_author_renders_infinity() {
        let output = render_vlacku_output(
            &VlackuSearchOutput {
                cards: vec![VlackuCard {
                    word: "birka".to_owned(),
                    word_type: "gismu".to_owned(),
                    selmaho: None,
                    author: Some(new!(VlackuAuthor {
                        username: "officialdata".to_owned(),
                        realname: Some("Official Data".to_owned()),
                    })),
                    is_official: true,
                    similarity: Some(1.0),
                    votes: Some(10000),
                    rafsi: Vec::new(),
                    glosses: Vec::new(),
                    definition: String::new(),
                    notes: String::new(),
                    etymology: None,
                    decomposition: Vec::new(),
                }],
                outcome: VlackuOutcome::Found,
                diagnostics: Vec::new(),
            },
            false,
            GlyphStyle::Unicode,
        );

        assert!(output.contains("votes: ∞"));
        assert!(!output.contains("votes: +10000"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_ascii_renders_index_places_and_official_votes_as_ascii() {
        let output = render_vlacku_output_with_options(
            &VlackuSearchOutput {
                cards: vec![VlackuCard {
                    word: "fuhivla".to_owned(),
                    word_type: "fu'ivla".to_owned(),
                    selmaho: None,
                    author: Some(new!(VlackuAuthor {
                        username: "officialdata".to_owned(),
                        realname: Some("Official Data".to_owned()),
                    })),
                    is_official: true,
                    similarity: Some(1.0),
                    votes: Some(10000),
                    rafsi: Vec::new(),
                    glosses: Vec::new(),
                    definition: "$x_1$ is a loanword meaning $x_2$.".to_owned(),
                    notes: String::new(),
                    etymology: None,
                    decomposition: Vec::new(),
                }],
                outcome: VlackuOutcome::Found,
                diagnostics: Vec::new(),
            },
            new!(VlackuRenderOptions {
                color: false,
                glyphs: GlyphStyle::Ascii,
                output_terminal_width: None,
                sumti_places: CliSumtiPlaces::Index,
                show_etymology: false,
            }),
        );

        assert!(output.contains("votes: official"));
        assert!(output.contains("<1> is a loanword meaning <2>."));
        assert!(!output.contains('∞'));
        assert!(!output.contains('⟨'));
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

    #[derive(Debug)]
    #[invariant(true)]
    struct CapturedCliRun {
        status: CliStatus,
        stdout: String,
        stderr: String,
    }

    static EMBEDDING_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    #[requires(true)]
    #[ensures(true)]
    fn embedding_env_lock() -> &'static Mutex<()> {
        EMBEDDING_ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    #[requires(!suffix.is_empty())]
    #[ensures(!ret.as_os_str().is_empty())]
    fn unique_embedding_test_path(suffix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "jbotci-embedding-test-{}-{}",
            std::process::id(),
            suffix
        ))
    }

    #[requires(true)]
    #[ensures(true)]
    fn run_cli_capture_with_embedding_dirs(
        args: &[&str],
        color_enabled: bool,
        model_dir: &Path,
        index_dir: &Path,
    ) -> CapturedCliRun {
        let _guard = embedding_env_lock()
            .lock()
            .expect("embedding env lock is not poisoned");
        let old_model_dir = std::env::var_os(EMBEDDING_MODEL_DIR_ENV);
        let old_index_dir = std::env::var_os(EMBEDDING_INDEX_DIR_ENV);
        set_embedding_test_env(EMBEDDING_MODEL_DIR_ENV, Some(model_dir.as_os_str()));
        set_embedding_test_env(EMBEDDING_INDEX_DIR_ENV, Some(index_dir.as_os_str()));
        let run = run_cli_capture(args, color_enabled);
        set_embedding_test_env(EMBEDDING_MODEL_DIR_ENV, old_model_dir.as_deref());
        set_embedding_test_env(EMBEDDING_INDEX_DIR_ENV, old_index_dir.as_deref());
        run
    }

    #[requires(!name.is_empty())]
    #[ensures(true)]
    fn set_embedding_test_env(name: &str, value: Option<&std::ffi::OsStr>) {
        // The embedding env vars are process-global; tests that mutate them hold
        // EMBEDDING_ENV_LOCK so concurrent semantic-search tests cannot observe a
        // half-updated pair.
        unsafe {
            if let Some(value) = value {
                std::env::set_var(name, value);
            } else {
                std::env::remove_var(name);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn run_cli_capture(args: &[&str], color_enabled: bool) -> CapturedCliRun {
        let cli = Cli::try_parse_from(args).expect("CLI args parse");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status =
            run_cli(cli, &mut output, &mut error, color_enabled).expect("CLI run succeeds");

        CapturedCliRun {
            status,
            stdout: String::from_utf8(output).expect("stdout utf8"),
            stderr: String::from_utf8(error).expect("stderr utf8"),
        }
    }

    #[requires(!needles.is_empty())]
    #[ensures(true)]
    fn assert_in_order(haystack: &str, needles: &[&str]) {
        let mut start_index = 0;
        for needle in needles {
            let Some(relative_index) = haystack[start_index..].find(needle) else {
                panic!("missing `{needle}` after byte {start_index} in:\n{haystack}");
            };
            start_index += relative_index + needle.len();
        }
    }

    #[requires(!expected_statuses.is_empty())]
    #[requires(!expected_iterations.is_empty())]
    #[ensures(true)]
    fn assert_benchmark_report_contains(
        stderr: &str,
        expected_iterations: &str,
        expected_statuses: &str,
    ) {
        assert_in_order(
            stderr,
            &[
                "benchmark:\n",
                expected_iterations,
                expected_statuses,
                "wall: total=",
                "throughput=",
                "cpu: ",
                "memory: ",
                "page-faults: ",
                "context-switches: ",
                "block-io: ",
            ],
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn run_success_stdout(args: &[&str]) -> String {
        let cli = Cli::try_parse_from(args).expect("CLI args parse");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("CLI run succeeds");

        assert_eq!(status, CliStatus::Success);
        assert!(error.is_empty(), "{}", String::from_utf8_lossy(&error));
        String::from_utf8(output).expect("stdout utf8")
    }

    #[requires(true)]
    #[ensures(true)]
    fn run_success_bytes(args: &[&str]) -> Vec<u8> {
        let cli = Cli::try_parse_from(args).expect("CLI args parse");
        let mut output = Vec::new();
        let mut error = Vec::new();
        let status = run_cli(cli, &mut output, &mut error, false).expect("CLI run succeeds");

        assert_eq!(status, CliStatus::Success);
        assert!(error.is_empty(), "{}", String::from_utf8_lossy(&error));
        output
    }

    #[requires(!stem.is_empty())]
    #[requires(!extension.is_empty())]
    #[ensures(ret.extension().is_some())]
    fn unique_cli_output_path(stem: &str, extension: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "jbotci-{stem}-{}.{}",
            std::process::id(),
            extension
        ))
    }

    #[requires(!expected.is_empty())]
    #[ensures(true)]
    fn assert_gentufa_error_contains(args: &[&str], expected: &str) {
        let cli = Cli::try_parse_from(args).expect("CLI args parse");
        let error = run_cli(cli, &mut Vec::new(), &mut Vec::new(), false)
            .expect_err("CLI run rejects args");
        assert!(
            error.to_string().contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn run_on_normal_stack(test: impl FnOnce()) {
        test();
    }
}
