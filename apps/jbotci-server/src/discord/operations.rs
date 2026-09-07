//! Typed application operations behind the Discord presenters.
//!
//! Each `/jbotci` request maps to one operation over the shared crates
//! (morphology, syntax, dictionary/CLL search, construction, diagram
//! rendering) and produces a typed [`ToolOutcome`]. Nothing here renders text
//! for Discord and nothing captures CLI output: presenters consume the
//! outcomes, and the CLI/MCP keep their own renderers over the same crates.
//!
//! CPU-bound work runs under the compute lane of the work governor; meaning
//! searches run on the server's single embedding worker and are awaited
//! outside the lane. Result sets are fetched up to the Discord presentation
//! cap plus one so a capped set is reported as capped rather than complete.

use std::fmt;
use std::num::NonZeroUsize;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_cll::{
    CllExample, CllParagraphRole, CllSearchChunkKind, CllSearchMatch, CllSection, CllSite,
    CuktaSearchMode, CuktaTargetFilter, cll_lookup_example, cll_lookup_section, cll_numbered_title,
    cll_resolve_example_reference, cll_resolve_section_reference, cukta_search, embedded_cll_site,
};
use jbotci_dialect::parse_dialect_definition;
use jbotci_gimfihi::{
    GimfihiCandidate, GimfihiError, GimfihiOutput, GimfihiPreset, GimfihiRequest, GimfihiScorer,
    GimfihiSourceInput, ResolvedSource, parse_source_spec, resolve_sources,
};
use jbotci_jvozba::{
    JvozbaBuildLimits, JvozbaBuildResult, JvozbaError, JvozbaInput, JvozbaMode,
    build_best_jvozba_detailed, build_best_jvozba_detailed_within, decompose_lujvo_like,
};
use jbotci_morphology::{
    LujvoPart, MorphologyOptions, PhonemeRenderOptions, Phonemes, normalize_lojban_input_text,
    segment_words_with_modifiers,
};
use jbotci_search::vlacku::{
    VlackuCard, VlackuOutcome as SearchOutcome, VlackuRequest as SearchRequest,
    VlackuSearchOptions, WordTypeFilter, dictionary_entry_card,
    dictionary_entry_passes_vlacku_filters, normalize_word_type_filter, parse_word_type_filter,
    run_vlacku_requests, word_like_lookup_text,
};
use jbotci_source::SourceId;
use jbotci_web_core::{
    GentufaScript, GentufaWebOptions, GentufaWebRequest, GentufaWebResult, GentufaWebViewMode,
    VlaseiAnalysis, VlataiReport, analyze_vlasei, analyze_vlatai,
};
use tokio::time::Instant;

use super::diagram::{DiagramError, DiagramImage, DiagramLimits, render_diagram};
use super::request::{
    CuktaMode, CuktaRequest, CuktaResultKind, DiscordRequest, GentufaRequest,
    GimfihiRequest as DiscordGimfihiRequest, JvozbaRequest, JvozbaTarget, MAX_PAGE, PAGE_SIZE,
    PageNumber, SourceField, SourceText, VLACKU_MAX_PAGE, VlackuMode, VlackuRequest, VlaseiRequest,
    VlataiRequest,
};
use super::work::{WorkError, WorkGovernor, WorkKeepalive};
use crate::{SearchQuery, SemanticSearchError, SemanticSearchErrorData, ToolServices};

/// Results fetched for a page-able search: the Discord cap plus one, so an
/// overflowing underlying result set is detected and reported as capped.
pub(crate) const VLACKU_FETCH_COUNT: usize = PAGE_SIZE * VLACKU_MAX_PAGE as usize + 1;
/// [`VLACKU_FETCH_COUNT`] as the worker's positive count.
const VLACKU_FETCH_LIMIT: NonZeroUsize = NonZeroUsize::new(VLACKU_FETCH_COUNT).unwrap();
pub(crate) const CUKTA_FETCH_COUNT: usize = PAGE_SIZE * MAX_PAGE as usize + 1;
/// [`CUKTA_FETCH_COUNT`] as the worker's positive count.
const CUKTA_FETCH_LIMIT: NonZeroUsize = NonZeroUsize::new(CUKTA_FETCH_COUNT).unwrap();
pub(crate) const GIMFIHI_FETCH_COUNT: usize = PAGE_SIZE * MAX_PAGE as usize + 1;

/// Source label attached to Discord diagnostics.
pub(crate) const SOURCE_LABEL: &str = "<discord>";

/// Separators between explicitly entered gimfihi source records.
pub(crate) const GIMFIHI_RECORD_SEPARATORS: [char; 3] = [',', ';', '\n'];

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

/// One page of a result list fetched up to the Discord cap plus one.
#[invariant(items.len() <= PAGE_SIZE && *page_count >= 1 && (page.get() as usize) <= *page_count)]
#[invariant(*shown_total <= PAGE_SIZE * (MAX_PAGE as usize))]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PagedResults<T> {
    pub(crate) items: Vec<T>,
    pub(crate) page: PageNumber,
    pub(crate) page_count: usize,
    /// Number of results inside the Discord cap (what the pages cover).
    pub(crate) shown_total: usize,
    /// The underlying result set has more than the cap; the app link is the
    /// continuation.
    pub(crate) capped: bool,
}

impl<T> PagedResults<T> {
    /// Slice `all` (fetched with cap+1) into `page` of `max_pages` pages.
    #[requires(max_pages >= 1 && max_pages <= MAX_PAGE)]
    #[ensures(ret.as_ref().is_ok_and(|paged| paged.page == page) || ret.is_err())]
    pub(crate) fn paginate(
        mut all: Vec<T>,
        page: PageNumber,
        max_pages: u8,
    ) -> Result<Self, PageUnavailable> {
        let cap = PAGE_SIZE * usize::from(max_pages);
        let capped = all.len() > cap;
        all.truncate(cap);
        let shown_total = all.len();
        let page_count = shown_total.div_ceil(PAGE_SIZE).max(1);
        let requested = usize::from(page.get());
        if requested > page_count {
            return Err(new!(PageUnavailable {
                requested: page.get(),
                available: page_count,
            }));
        }
        let start = page.first_index().min(shown_total);
        let end = (start + PAGE_SIZE).min(shown_total);
        let items = all.drain(start..end).collect::<Vec<_>>();
        Ok(new!(PagedResults {
            items,
            page,
            page_count,
            shown_total,
            capped,
        }))
    }

    #[requires(true)]
    #[ensures(ret == (self.shown_total == 0))]
    pub(crate) fn is_empty(&self) -> bool {
        self.shown_total == 0
    }
}

/// The requested page lies beyond the current result set.
#[invariant(*requested as usize > *available)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PageUnavailable {
    pub(crate) requested: u8,
    pub(crate) available: usize,
}

impl fmt::Display for PageUnavailable {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "page {} is not available; this result has {} page{}",
            self.requested,
            self.available,
            if self.available == 1 { "" } else { "s" }
        )
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// A request that cannot be run as stated. On the slash path this is
/// published as an editable result; on the modal path it is a private error
/// that leaves the last result in place.
#[invariant(::Dialect { .. } => true)]
#[invariant(::EmptyField { .. } => true)]
#[invariant(::CuktaQueryRequired { .. } => true)]
#[invariant(::JvozbaParts { .. } => true)]
#[invariant(::GimfihiSources { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RequestValidationError {
    Dialect {
        message: String,
    },
    EmptyField {
        field: SourceField,
    },
    /// A book search or reference lookup with nothing to look for.
    CuktaQueryRequired {
        mode: CuktaMode,
    },
    JvozbaParts {
        message: String,
    },
    GimfihiSources {
        errors: Vec<String>,
    },
}

impl fmt::Display for RequestValidationError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dialect { message } => write!(formatter, "invalid dialect formula: {message}"),
            Self::CuktaQueryRequired { mode } => write!(
                formatter,
                "{} needs something to look for; add it in the form",
                mode.label()
            ),
            Self::EmptyField { field } => {
                write!(formatter, "the {} field must not be blank", field.label())
            }
            Self::JvozbaParts { message } => {
                write!(
                    formatter,
                    "the source words are not valid Lojban words: {message}"
                )
            }
            Self::GimfihiSources { errors } => {
                write!(
                    formatter,
                    "the source records are not usable: {}",
                    errors.join("; ")
                )
            }
        }
    }
}

impl std::error::Error for RequestValidationError {}

/// What can be judged about a request before doing any of its work: a field
/// that is actually there, and a dialect that parses. Everything needing
/// morphology, phonology or the dictionary is left to the tool's own run,
/// which happens inside the compute lane; checking it here would do that work
/// twice and do it on the runtime thread, where nothing bounds it.
#[requires(true)]
#[ensures(true)]
pub(crate) fn preflight(request: &DiscordRequest) -> Result<(), RequestValidationError> {
    match request {
        DiscordRequest::Gentufa(GentufaRequest { text, dialect, .. })
        | DiscordRequest::Vlasei(VlaseiRequest { text, dialect, .. })
        | DiscordRequest::Vlatai(VlataiRequest { text, dialect, .. }) => {
            require_visible(text, SourceField::Text)?;
            validate_dialect(dialect.as_ref())
        }
        DiscordRequest::Vlacku(request) => require_visible(&request.query, SourceField::Query),
        DiscordRequest::Cukta(request) => match &request.query {
            Some(query) if request.options.mode.requires_query() => {
                require_visible(query, SourceField::Query)
            }
            // A search or a reference lookup with nothing to look for is an
            // incomplete task: the reader finishes it in the form.
            None if request.options.mode.requires_query() => {
                Err(RequestValidationError::CuktaQueryRequired {
                    mode: request.options.mode,
                })
            }
            _ => Ok(()),
        },
        DiscordRequest::Jvozba(request) => require_visible(&request.parts, SourceField::Parts),
        // Reading gimfihi's source records needs the same phonology the run
        // itself needs, so the run does it: `run_gimfihi` reports exactly the
        // same error from inside its worker.
        DiscordRequest::Gimfihi(_) => Ok(()),
    }
}

#[requires(true)]
#[ensures(ret.is_ok() == !text.as_str().trim().is_empty())]
fn require_visible(text: &SourceText, field: SourceField) -> Result<(), RequestValidationError> {
    if text.as_str().trim().is_empty() {
        Err(RequestValidationError::EmptyField { field })
    } else {
        Ok(())
    }
}

/// The dialect formula to hand the shared analyzers: `None` for standard
/// Lojban (absent or blank field).
#[requires(true)]
#[ensures(ret.is_none_or(|formula| !formula.trim().is_empty()))]
fn dialect_formula(dialect: Option<&SourceText>) -> Option<&str> {
    dialect
        .map(SourceText::as_str)
        .filter(|formula| !formula.trim().is_empty())
}

#[requires(true)]
#[ensures(true)]
fn validate_dialect(dialect: Option<&SourceText>) -> Result<(), RequestValidationError> {
    match dialect_formula(dialect) {
        Some(formula) => parse_dialect_definition(formula.trim())
            .map(|_| ())
            .map_err(|error| RequestValidationError::Dialect {
                message: error.to_string(),
            }),
        None => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// Outcomes
// ---------------------------------------------------------------------------

#[invariant(true)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GentufaOutcome {
    pub(crate) result: GentufaWebResult,
    /// Present only when the diagram was requested and the parse produced a
    /// layout; a requested diagram that could not be rendered is an error, not
    /// a silently missing image.
    pub(crate) diagram: Option<DiagramImage>,
}

#[invariant(::Results { .. } => true)]
#[invariant(::Unavailable { .. } => true)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum VlackuOutcome {
    Results {
        /// Shared dictionary search cards in rank order; a card with
        /// `known == false` is a synthesized analysis of an unknown word, not
        /// a dictionary definition, and the presenter says so.
        results: PagedResults<VlackuCard>,
        /// Search-layer diagnostics (pattern errors and the like).
        diagnostics: Vec<String>,
        /// Whether the query was valid but matched nothing.
        valid_missing: bool,
    },
    /// Meaning search needs the embedding index and this server cannot
    /// provide it; the reason is shown, no other search is substituted.
    Unavailable { reason: String },
}

/// One CLL search hit, independent of web routes.
#[invariant(*rank >= 1 && !label.is_empty())]
#[invariant(role.is_none() || *kind == CllSearchChunkKind::Paragraph)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CuktaSearchCard {
    pub(crate) rank: usize,
    pub(crate) similarity: Option<f32>,
    pub(crate) kind: CllSearchChunkKind,
    pub(crate) role: Option<CllParagraphRole>,
    pub(crate) label: String,
    pub(crate) section_label: String,
    pub(crate) section_id: String,
    pub(crate) text: String,
}

#[invariant(!title.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuktaChapterEntry {
    pub(crate) number: Option<u16>,
    pub(crate) title: String,
    pub(crate) section_count: usize,
}

#[invariant(::Search { .. } => true)]
#[invariant(::Section { .. } => true)]
#[invariant(::Example { .. } => true)]
#[invariant(::Contents { .. } => true)]
#[invariant(::NotFound { .. } => true)]
#[invariant(::Unavailable { .. } => true)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CuktaOutcome {
    Search {
        results: PagedResults<CuktaSearchCard>,
        message: Option<String>,
    },
    /// A section of the embedded book; the site is carried alongside so the
    /// presenter can resolve the examples the section's blocks reference.
    Section {
        site: &'static CllSite,
        section: &'static CllSection,
    },
    Example {
        site: &'static CllSite,
        example: &'static CllExample,
    },
    Contents {
        edition: String,
        chapters: Vec<CuktaChapterEntry>,
    },
    NotFound {
        what: &'static str,
        reference: String,
    },
    Unavailable {
        reason: String,
    },
}

/// One rafsi (or hyphen) of a built word with the source word it came from
/// when the shared decomposition knows it.
#[invariant(!text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JvozbaConstituent {
    pub(crate) text: String,
    pub(crate) is_hyphen: bool,
    pub(crate) source: Option<String>,
}

#[invariant(!word.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JvozbaBuilt {
    pub(crate) word: String,
    pub(crate) constituents: Vec<JvozbaConstituent>,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JvozbaOutcome {
    pub(crate) inputs: Vec<JvozbaInput>,
    pub(crate) target: JvozbaTarget,
    pub(crate) result: Result<JvozbaBuilt, JvozbaError>,
}

#[invariant(::Setup { .. } => true)]
#[invariant(::Candidates { .. } => true)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum GimfihiOutcome {
    /// No source words were supplied: an editable setup response (PM
    /// decision), never invented candidates.
    Setup {
        preset: Option<GimfihiPreset>,
        languages: Vec<String>,
    },
    Candidates {
        sources: Vec<ResolvedSource>,
        winner: Option<String>,
        candidate_count: usize,
        filtered_count: usize,
        results: PagedResults<GimfihiCandidate>,
    },
}

#[invariant(::Gentufa(_) => true)]
#[invariant(::Vlasei(_) => true)]
#[invariant(::Vlatai(_) => true)]
#[invariant(::Vlacku(_) => true)]
#[invariant(::Cukta(_) => true)]
#[invariant(::Jvozba(_) => true)]
#[invariant(::Gimfihi(_) => true)]
#[derive(Debug, Clone)]
pub(crate) enum ToolOutcome {
    Gentufa(GentufaOutcome),
    Vlasei(VlaseiAnalysis),
    Vlatai(VlataiReport),
    Vlacku(VlackuOutcome),
    Cukta(CuktaOutcome),
    Jvozba(JvozbaOutcome),
    Gimfihi(GimfihiOutcome),
}

#[invariant(::Invalid(_) => true)]
#[invariant(::Page(_) => true)]
#[invariant(::Diagram(_) => true)]
#[invariant(::Work(_) => true)]
#[invariant(::Internal { .. } => true)]
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum OperationError {
    Invalid(RequestValidationError),
    Page(PageUnavailable),
    Diagram(DiagramError),
    Work(WorkError),
    Internal { message: String },
}

impl fmt::Display for OperationError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(error) => write!(formatter, "{error}"),
            Self::Page(error) => write!(formatter, "{error}"),
            Self::Diagram(error) => write!(formatter, "{error}"),
            Self::Work(error) => write!(formatter, "{error}"),
            Self::Internal { message } => write!(formatter, "internal error: {message}"),
        }
    }
}

impl std::error::Error for OperationError {}

impl From<WorkError> for OperationError {
    #[requires(true)]
    #[ensures(true)]
    fn from(error: WorkError) -> Self {
        Self::Work(error)
    }
}

impl From<PageUnavailable> for OperationError {
    #[requires(true)]
    #[ensures(true)]
    fn from(error: PageUnavailable) -> Self {
        Self::Page(error)
    }
}

/// What an operation needs besides the request.
#[invariant(true)]
#[derive(Clone)]
pub(crate) struct OperationContext<'a> {
    pub(crate) tools: &'a ToolServices,
    pub(crate) governor: &'a WorkGovernor,
    pub(crate) deadline: Instant,
    /// What the running work stands for. It is carried into every compute
    /// job, so a caller that stops waiting does not release a delivery whose
    /// work is still going.
    pub(crate) keepalive: Option<WorkKeepalive>,
    /// Discord's per-interaction attachment size limit, when supplied.
    pub(crate) attachment_size_limit: Option<u64>,
    pub(crate) diagram_limits: DiagramLimits,
}

/// Run `request` to a typed outcome.
#[requires(true)]
#[ensures(true)]
pub(crate) async fn run_request(
    request: DiscordRequest,
    context: OperationContext<'_>,
) -> Result<ToolOutcome, OperationError> {
    preflight(&request).map_err(OperationError::Invalid)?;
    match request {
        DiscordRequest::Gentufa(request) => {
            let limits = context.diagram_limits;
            let attachment_size_limit = context.attachment_size_limit;
            context
                .governor
                .run_compute_keeping(context.deadline, context.keepalive.clone(), move || {
                    run_gentufa(&request, limits, attachment_size_limit)
                })
                .await?
                .map(ToolOutcome::Gentufa)
        }
        DiscordRequest::Vlasei(request) => context
            .governor
            .run_compute_keeping(context.deadline, context.keepalive.clone(), move || {
                run_vlasei(&request)
            })
            .await?
            .map(ToolOutcome::Vlasei),
        DiscordRequest::Vlatai(request) => context
            .governor
            .run_compute_keeping(context.deadline, context.keepalive.clone(), move || {
                run_vlatai(&request)
            })
            .await?
            .map(ToolOutcome::Vlatai),
        DiscordRequest::Vlacku(request) => {
            run_vlacku(request, context).await.map(ToolOutcome::Vlacku)
        }
        DiscordRequest::Cukta(request) => run_cukta(request, context).await.map(ToolOutcome::Cukta),
        DiscordRequest::Jvozba(request) => context
            .governor
            .run_compute_keeping(context.deadline, context.keepalive.clone(), move || {
                run_jvozba(&request)
            })
            .await?
            .map(ToolOutcome::Jvozba),
        DiscordRequest::Gimfihi(request) => context
            .governor
            .run_compute_keeping(context.deadline, context.keepalive.clone(), move || {
                run_gimfihi(&request)
            })
            .await?
            .map(ToolOutcome::Gimfihi),
    }
}

// ---------------------------------------------------------------------------
// Gentufa
// ---------------------------------------------------------------------------

#[requires(true)]
#[ensures(true)]
fn run_gentufa(
    request: &GentufaRequest,
    limits: DiagramLimits,
    attachment_size_limit: Option<u64>,
) -> Result<GentufaOutcome, OperationError> {
    let options = request.options;
    let web_request = GentufaWebRequest {
        text: request.text.as_str().to_owned(),
        options: GentufaWebOptions {
            dialect: dialect_formula(request.dialect.as_ref()).map(str::to_owned),
            view_mode: GentufaWebViewMode::Blocks,
            script: GentufaScript::Latin,
            show_elided: options.show_elided,
            show_glosses: options.show_glosses,
            show_compounds: options.show_compounds,
            show_definitions: false,
            error_context_depth: 1,
            phonemes: PhonemeRenderOptions::default(),
        },
    };
    let result = jbotci_web_core::parse_gentufa_for_web(&web_request);
    let diagram = match (&result, options.include_diagram) {
        (GentufaWebResult::Success(success), true) => Some(
            render_diagram(
                &success.blocks_layout,
                options.show_glosses,
                limits,
                attachment_size_limit,
            )
            .map_err(OperationError::Diagram)?,
        ),
        _ => None,
    };
    Ok(GentufaOutcome { result, diagram })
}

// ---------------------------------------------------------------------------
// Vlasei / vlatai
// ---------------------------------------------------------------------------

#[requires(true)]
#[ensures(true)]
fn run_vlasei(request: &VlaseiRequest) -> Result<VlaseiAnalysis, OperationError> {
    analyze_vlasei(
        request.text.as_str(),
        dialect_formula(request.dialect.as_ref()).map(str::trim),
        Some(SourceId(SOURCE_LABEL.to_owned())),
    )
    .map_err(|error| {
        OperationError::Invalid(RequestValidationError::Dialect {
            message: error.to_string(),
        })
    })
}

#[requires(true)]
#[ensures(true)]
fn run_vlatai(request: &VlataiRequest) -> Result<VlataiReport, OperationError> {
    let dialect = match dialect_formula(request.dialect.as_ref()) {
        Some(formula) => parse_dialect_definition(formula.trim()).map_err(|error| {
            OperationError::Invalid(RequestValidationError::Dialect {
                message: error.to_string(),
            })
        })?,
        None => Default::default(),
    };
    let options = MorphologyOptions::default()
        .try_with_dialect_definition(&dialect)
        .map_err(|error| {
            OperationError::Invalid(RequestValidationError::Dialect {
                message: error.to_string(),
            })
        })?;
    analyze_vlatai(
        request.text.as_str(),
        &options,
        Some(SourceId(SOURCE_LABEL.to_owned())),
    )
    .map_err(|error| OperationError::Internal {
        message: error.to_string(),
    })
}

// ---------------------------------------------------------------------------
// Vlacku
// ---------------------------------------------------------------------------

#[requires(true)]
#[ensures(true)]
fn vlacku_search_options(request: &VlackuRequest) -> VlackuSearchOptions {
    let word_types = request
        .options
        .word_types
        .iter()
        .filter_map(|word_type| {
            parse_word_type_filter(&normalize_word_type_filter(word_type.filter_value()))
        })
        .collect::<Vec<WordTypeFilter>>();
    VlackuSearchOptions::default().with_data(data! {
        count: VLACKU_FETCH_COUNT,
        word_types: word_types,
        decompose_lujvo: request.options.decompose_lujvo,
    })
}

#[requires(true)]
#[ensures(true)]
async fn run_vlacku(
    request: VlackuRequest,
    context: OperationContext<'_>,
) -> Result<VlackuOutcome, OperationError> {
    let query = request.query.as_str().trim().to_owned();
    if request.options.mode == VlackuMode::Meaning {
        let dictionary = jbotci_dictionary_data::english();
        let Some(search_query) = SearchQuery::new(&query) else {
            return Err(OperationError::Invalid(
                RequestValidationError::EmptyField {
                    field: SourceField::Query,
                },
            ));
        };
        // Entry filters travel with the job and apply while ranking, so the
        // worker never materializes more than the fetch limit.
        let hits = match context
            .tools
            .semantic_vlacku_hits(
                search_query,
                VLACKU_FETCH_LIMIT,
                vlacku_search_options(&request),
                Some(context.deadline),
                context.keepalive.clone(),
            )
            .await
        {
            Ok(hits) => hits,
            Err(error) => {
                return semantic_failure(error).map(|reason| VlackuOutcome::Unavailable { reason });
            }
        };
        return context
            .governor
            .run_compute_keeping(context.deadline, context.keepalive.clone(), move || {
                let options = vlacku_search_options(&request);
                let cards = hits
                    .into_iter()
                    .filter_map(|hit| {
                        dictionary
                            .entries()
                            .get(hit.entry_index)
                            .map(|entry| (hit.score, entry))
                    })
                    .filter(|(score, entry)| {
                        dictionary_entry_passes_vlacku_filters(entry, &options, Some(*score), true)
                    })
                    .take(VLACKU_FETCH_COUNT)
                    .map(|(score, entry)| {
                        dictionary_entry_card(
                            dictionary,
                            entry,
                            Some(score),
                            options.decompose_lujvo,
                        )
                    })
                    .collect::<Vec<_>>();
                let results = PagedResults::paginate(cards, request.options.page, VLACKU_MAX_PAGE)?;
                Ok(VlackuOutcome::Results {
                    valid_missing: results.is_empty(),
                    results,
                    diagnostics: Vec::new(),
                })
            })
            .await?;
    }
    context
        .governor
        .run_compute_keeping(context.deadline, context.keepalive.clone(), move || {
            let search_request = match request.options.mode {
                VlackuMode::Word => SearchRequest::valsi(query),
                VlackuMode::Rafsi => SearchRequest::rafsi(query),
                VlackuMode::Lujvo => SearchRequest::lujvo(query),
                VlackuMode::Sound => SearchRequest::sound(query),
                VlackuMode::Meaning => unreachable!("meaning search is handled above"),
            };
            let options = vlacku_search_options(&request);
            let output = run_vlacku_requests(
                jbotci_dictionary_data::english(),
                &[search_request],
                &options,
            );
            let valid_missing = output.outcome == SearchOutcome::ValidMissing
                || (output.outcome == SearchOutcome::Found && output.cards.is_empty());
            let results =
                PagedResults::paginate(output.cards, request.options.page, VLACKU_MAX_PAGE)?;
            Ok(VlackuOutcome::Results {
                results,
                diagnostics: output.diagnostics,
                valid_missing,
            })
        })
        .await?
}

// ---------------------------------------------------------------------------
// Cukta
// ---------------------------------------------------------------------------

/// Map a typed meaning-search failure: a missing index is a presented
/// outcome (its reason), while admission, deadline and search failures are
/// operation errors.
#[requires(true)]
#[ensures(true)]
fn semantic_failure(error: SemanticSearchError) -> Result<String, OperationError> {
    match error.into_data() {
        data!(SemanticSearchError::Unavailable { reason }) => Ok(reason),
        data!(SemanticSearchError::Lane(error)) => Err(OperationError::Work(error)),
        data!(SemanticSearchError::Failed { message }) => Err(OperationError::Internal { message }),
    }
}

#[requires(true)]
#[ensures(true)]
fn cukta_targets(request: &CuktaRequest) -> CuktaTargetFilter {
    let kinds = request.options.kinds;
    if kinds.is_empty() {
        CuktaTargetFilter::default()
    } else {
        CuktaTargetFilter {
            sections: kinds.contains(CuktaResultKind::Section),
            paragraphs: kinds.contains(CuktaResultKind::Paragraph),
            examples: kinds.contains(CuktaResultKind::Example),
        }
    }
}

#[requires(true)]
#[ensures(ret.rank == item.rank)]
fn cukta_card(item: &CllSearchMatch) -> CuktaSearchCard {
    new!(CuktaSearchCard {
        rank: item.rank,
        similarity: item.similarity,
        kind: item.chunk.kind,
        role: item.chunk.role.clone(),
        label: item.chunk.label.clone(),
        section_label: cll_numbered_title(
            item.chunk.section_number.as_deref(),
            &item.chunk.section_title,
        ),
        section_id: item.chunk.section_id.clone(),
        text: item.chunk.text.clone(),
    })
}

/// Test access to the search-card projection.
#[cfg(test)]
#[requires(true)]
#[ensures(ret.rank == item.rank)]
pub(crate) fn cukta_card_for_tests(item: &CllSearchMatch) -> CuktaSearchCard {
    cukta_card(item)
}

#[requires(true)]
#[ensures(true)]
async fn run_cukta(
    request: CuktaRequest,
    context: OperationContext<'_>,
) -> Result<CuktaOutcome, OperationError> {
    let query = request
        .query
        .as_ref()
        .map(|query| query.as_str().trim().to_owned())
        .unwrap_or_default();
    match request.options.mode {
        CuktaMode::Meaning => {
            let targets = cukta_targets(&request);
            let Some(search_query) = SearchQuery::new(&query) else {
                return Err(OperationError::Invalid(
                    RequestValidationError::EmptyField {
                        field: SourceField::Query,
                    },
                ));
            };
            let output = match context
                .tools
                .semantic_cukta_search(
                    search_query,
                    CUKTA_FETCH_LIMIT,
                    targets,
                    Some(context.deadline),
                    context.keepalive.clone(),
                )
                .await
            {
                Ok(output) => output,
                Err(error) => {
                    return semantic_failure(error)
                        .map(|reason| CuktaOutcome::Unavailable { reason });
                }
            };
            let cards = output.matches.iter().map(cukta_card).collect::<Vec<_>>();
            let results = PagedResults::paginate(cards, request.options.page, MAX_PAGE)?;
            Ok(CuktaOutcome::Search {
                results,
                message: output.message,
            })
        }
        CuktaMode::Word => {
            let page = request.options.page;
            let targets = cukta_targets(&request);
            context
                .governor
                .run_compute_keeping(context.deadline, context.keepalive.clone(), move || {
                    let site = embedded_cll_site().map_err(|error| OperationError::Internal {
                        message: error.to_string(),
                    })?;
                    let output = cukta_search(
                        site,
                        CuktaSearchMode::Word,
                        &query,
                        CUKTA_FETCH_COUNT,
                        targets,
                    );
                    let cards = output.matches.iter().map(cukta_card).collect::<Vec<_>>();
                    let results = PagedResults::paginate(cards, page, MAX_PAGE)?;
                    Ok(CuktaOutcome::Search {
                        results,
                        message: output.message,
                    })
                })
                .await?
        }
        CuktaMode::Section => {
            context
                .governor
                .run_compute_keeping(context.deadline, context.keepalive.clone(), move || {
                    let site = embedded_cll_site().map_err(|error| OperationError::Internal {
                        message: error.to_string(),
                    })?;
                    let Some(section_id) = cll_resolve_section_reference(site, &query) else {
                        return Ok(CuktaOutcome::NotFound {
                            what: "section",
                            reference: query,
                        });
                    };
                    let Some(section) = cll_lookup_section(site, &section_id) else {
                        return Ok(CuktaOutcome::NotFound {
                            what: "section",
                            reference: query,
                        });
                    };
                    Ok(CuktaOutcome::Section { site, section })
                })
                .await?
        }
        CuktaMode::Example => {
            context
                .governor
                .run_compute_keeping(context.deadline, context.keepalive.clone(), move || {
                    let site = embedded_cll_site().map_err(|error| OperationError::Internal {
                        message: error.to_string(),
                    })?;
                    let example = cll_resolve_example_reference(site, &query)
                        .and_then(|example_id| cll_lookup_example(site, &example_id));
                    Ok(match example {
                        Some(example) => CuktaOutcome::Example { site, example },
                        None => CuktaOutcome::NotFound {
                            what: "example",
                            reference: query,
                        },
                    })
                })
                .await?
        }
        CuktaMode::Contents => {
            context
                .governor
                .run_compute_keeping(context.deadline, context.keepalive.clone(), move || {
                    let site = embedded_cll_site().map_err(|error| OperationError::Internal {
                        message: error.to_string(),
                    })?;
                    let chapters = site
                        .chapters
                        .iter()
                        .map(|chapter| {
                            new!(CuktaChapterEntry {
                                number: chapter
                                    .division
                                    .chapter_number()
                                    .map(|number| number.get()),
                                title: chapter.chapter_title.clone(),
                                section_count: chapter.root_section_ids.len(),
                            })
                        })
                        .collect();
                    Ok(CuktaOutcome::Contents {
                        edition: format!(
                            "{} {}",
                            site.metadata.edition.title, site.metadata.edition.version
                        ),
                        chapters,
                    })
                })
                .await?
        }
    }
}

// ---------------------------------------------------------------------------
// Jvozba
// ---------------------------------------------------------------------------

/// Read the ordered pieces of a compound from one field. A span between
/// hyphens is a rafsi given literally, as in `blanu -blo- zdani`; everything
/// outside those spans is words, and the morphology parser is what decides
/// where a word begins and ends. The order the reader wrote is the order the
/// builder receives, whichever kind comes first.
#[requires(true)]
#[ensures(true)]
pub(crate) fn jvozba_inputs(
    request: &JvozbaRequest,
) -> Result<Vec<JvozbaInput>, RequestValidationError> {
    let mut inputs = Vec::new();
    let mut rest = request.parts.as_str();
    while let Some(open) = rest.find(FIXED_RAFSI_DELIMITER) {
        push_jvozba_words(&rest[..open], &mut inputs)?;
        let after = &rest[open + FIXED_RAFSI_DELIMITER.len_utf8()..];
        let Some(close) = after.find(FIXED_RAFSI_DELIMITER) else {
            return Err(RequestValidationError::JvozbaParts {
                message: format!(
                    "a fixed rafsi opened with `{FIXED_RAFSI_DELIMITER}` was never closed; \
                     write it as `-blo-`"
                ),
            });
        };
        inputs.push(JvozbaInput::FixedRafsi(fixed_rafsi_text(&after[..close])?));
        rest = &after[close + FIXED_RAFSI_DELIMITER.len_utf8()..];
    }
    push_jvozba_words(rest, &mut inputs)?;
    Ok(inputs)
}

/// What marks a rafsi given literally rather than looked up.
const FIXED_RAFSI_DELIMITER: char = '-';

/// Add the words of one span, in order. An empty or blank span contributes
/// nothing, which is what lets a compound begin or end with a fixed rafsi.
#[requires(true)]
#[ensures(ret.is_err() || inputs.len() >= old(inputs.len()))]
fn push_jvozba_words(
    span: &str,
    inputs: &mut Vec<JvozbaInput>,
) -> Result<(), RequestValidationError> {
    if span.trim().is_empty() {
        return Ok(());
    }
    let words = segment_words_with_modifiers(span).map_err(|error| {
        RequestValidationError::JvozbaParts {
            message: error.to_string(),
        }
    })?;
    for word in &words {
        let Some(text) = word_like_lookup_text(word) else {
            return Err(RequestValidationError::JvozbaParts {
                message: "quoted material cannot be a source word".to_owned(),
            });
        };
        inputs.push(JvozbaInput::Word(text));
    }
    Ok(())
}

/// The canonical text of one literal rafsi. It is not looked up, so nothing
/// else will catch a stray character: it must be Lojban letters and nothing
/// more, in the spelling the rest of the workspace uses.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
fn fixed_rafsi_text(span: &str) -> Result<String, RequestValidationError> {
    if span.is_empty() {
        return Err(RequestValidationError::JvozbaParts {
            message: "a fixed rafsi between hyphens is empty; write it as `-blo-`".to_owned(),
        });
    }
    if span.chars().any(char::is_whitespace) {
        return Err(RequestValidationError::JvozbaParts {
            message: format!(
                "`{span}` is not one rafsi: a fixed rafsi holds no spaces, and each one \
                 needs its own hyphens"
            ),
        });
    }
    let normalized =
        normalize_lojban_input_text(span).ok_or_else(|| RequestValidationError::JvozbaParts {
            message: format!("`{span}` is not Lojban text, so it cannot be a rafsi"),
        })?;
    Phonemes::from_canonical(normalized.clone()).map_err(|_| {
        RequestValidationError::JvozbaParts {
            message: format!("`{span}` is not Lojban text, so it cannot be a rafsi"),
        }
    })?;
    Ok(normalized)
}

/// How much work one Discord compound build may do. The search compares every
/// spelling before it can name a best one and nothing stops it once it starts,
/// so the whole request is refused when the plan is too large.
///
/// Measured in release on the development machine, with `klama` repeated so
/// each non-final piece offers two candidates: 48 placements took 5.5ms, 320
/// took 65ms, 768 took 178ms, 4096 took 1.24s, 9216 took 3.1s, 20480 took
/// 7.4s, and 458752 took 213s; the cost per placement rose from 0.12ms to
/// 0.46ms across those samples. 8192 placements is a couple of seconds of one
/// compute worker in the shapes measured, and it admits eight plain words or
/// five pieces offering four candidates each. The piece and spelling bounds
/// stand apart from it because a short choice list can still carry long
/// pieces, and because recursion depth follows the pieces; no attested
/// compound approaches either figure.
#[requires(true)]
#[ensures(ret.max_placements == 8192)]
fn jvozba_limits() -> JvozbaBuildLimits {
    new!(JvozbaBuildLimits {
        max_pieces: 24,
        max_placements: 8192,
        max_spelling_letters: 256,
    })
}

#[requires(true)]
#[ensures(true)]
fn run_jvozba(request: &JvozbaRequest) -> Result<JvozbaOutcome, OperationError> {
    let inputs = jvozba_inputs(request).map_err(OperationError::Invalid)?;
    let mode = match request.options.target {
        JvozbaTarget::Lujvo => JvozbaMode::Lujvo,
        JvozbaTarget::Cmevla => JvozbaMode::Cmevla,
    };
    let dictionary = jbotci_dictionary_data::english();
    let result = build_best_jvozba_detailed_within(mode, dictionary, &inputs, jvozba_limits()).map(
        |built| {
            let constituents = constituents_for(dictionary, &built);
            new!(JvozbaBuilt {
                word: built.word.clone(),
                constituents,
            })
        },
    );
    Ok(JvozbaOutcome {
        inputs,
        target: request.options.target,
        result,
    })
}

/// Constituents of a built word with their source words from the shared
/// decomposition when it recognizes the word; otherwise the build's own
/// segments without sources.
#[requires(!built.word.is_empty())]
#[ensures(!ret.is_empty() || built.segments.is_empty())]
fn constituents_for(
    dictionary: &jbotci_dictionary::Dictionary<'_>,
    built: &JvozbaBuildResult,
) -> Vec<JvozbaConstituent> {
    if let Some(decomposition) = decompose_lujvo_like(dictionary, &built.word) {
        let decomposition = decomposition.into_data();
        return decomposition
            .segments
            .into_iter()
            .map(|segment| {
                let (text, is_hyphen) = match &segment.segment {
                    LujvoPart::Rafsi(phonemes) => (phonemes.as_str().to_owned(), false),
                    LujvoPart::Hyphen(phonemes) => (phonemes.as_str().to_owned(), true),
                };
                new!(JvozbaConstituent {
                    text,
                    is_hyphen,
                    source: segment.source.map(str::to_owned),
                })
            })
            .collect();
    }
    built
        .segments
        .iter()
        .map(|segment| {
            new!(JvozbaConstituent {
                text: segment.text.clone(),
                is_hyphen: segment.kind == jbotci_jvozba::JvozbaSegmentKind::Hyphen,
                source: None,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Gimfihi
// ---------------------------------------------------------------------------

#[invariant(::Setup { .. } => true)]
#[invariant(::Compose(_) => true)]
enum GimfihiPlan {
    Setup { preset: Option<GimfihiPreset> },
    Compose(GimfihiRequest),
}

/// Decide between the editable setup response and candidate generation, and
/// validate the source records with the shared parser and resolver.
#[requires(true)]
#[ensures(true)]
fn gimfihi_plan(request: &DiscordGimfihiRequest) -> Result<GimfihiPlan, RequestValidationError> {
    let options = request.options;
    let records = request
        .sources
        .as_ref()
        .map(|sources| {
            sources
                .as_str()
                .split(GIMFIHI_RECORD_SEPARATORS)
                .map(str::trim)
                .filter(|record| !record.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if records.is_empty() {
        return Ok(GimfihiPlan::Setup {
            preset: options.preset,
        });
    }
    let mut sources: Vec<GimfihiSourceInput> = Vec::with_capacity(records.len());
    let mut errors = Vec::new();
    for record in records {
        match parse_source_spec(record) {
            Ok(source) => sources.push(source),
            Err(error) => errors.push(error.to_string()),
        }
    }
    if !errors.is_empty() {
        return Err(RequestValidationError::GimfihiSources { errors });
    }
    // The shared resolver owns preset/weight/language consistency and IPA
    // validation; its verdict is the validation verdict.
    if let Err(error) = resolve_sources(options.preset, &sources) {
        return Err(RequestValidationError::GimfihiSources {
            errors: vec![error.to_string()],
        });
    }
    Ok(GimfihiPlan::Compose(GimfihiRequest {
        scorer: GimfihiScorer::Classic,
        phonetic_parameters: Default::default(),
        preset: options.preset,
        sources,
        shapes: options.shapes.shapes(),
        all_letters: options.all_letters,
        check_collisions: options.collisions,
        show_collisions: options.show_collisions,
        require_free_short_rafsi: options.require_free_short_rafsi,
        count: GIMFIHI_FETCH_COUNT,
        highlight: None,
    }))
}

#[requires(true)]
#[ensures(true)]
fn run_gimfihi(request: &DiscordGimfihiRequest) -> Result<GimfihiOutcome, OperationError> {
    match gimfihi_plan(request).map_err(OperationError::Invalid)? {
        GimfihiPlan::Setup { preset } => Ok(GimfihiOutcome::Setup {
            preset,
            languages: preset
                .map(|preset| {
                    preset
                        .entries()
                        .iter()
                        .map(|entry| entry.language.to_owned())
                        .collect()
                })
                .unwrap_or_default(),
        }),
        GimfihiPlan::Compose(gimfihi_request) => {
            let output: GimfihiOutput =
                jbotci_gimfihi::compose_gismu(jbotci_dictionary_data::english(), &gimfihi_request)
                    .map_err(|error: GimfihiError| {
                        OperationError::Invalid(RequestValidationError::GimfihiSources {
                            errors: vec![error.to_string()],
                        })
                    })?;
            let results =
                PagedResults::paginate(output.candidates, request.options.page, MAX_PAGE)?;
            Ok(GimfihiOutcome::Candidates {
                sources: output.resolved_sources,
                winner: output.winner,
                candidate_count: output.candidate_count,
                filtered_count: output.filtered_count,
                results,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord::request::{
        CuktaOptions, CuktaResultKindSet, GentufaOptions, GimfihiOptions, JvozbaOptions,
        VlackuOptions, VlaseiOptions, VlataiOptions,
    };
    use crate::discord::work::WorkLimits;
    use jbotci_cll::format_section_display_title;
    use std::time::Duration;

    #[requires(true)]
    #[ensures(true)]
    fn text(value: &str) -> SourceText {
        SourceText::new(value).expect("test text fits")
    }

    #[requires(true)]
    #[ensures(true)]
    async fn run(request: DiscordRequest) -> Result<ToolOutcome, OperationError> {
        let tools = ToolServices::new();
        let governor = WorkGovernor::new(WorkLimits::default());
        run_request(
            request,
            OperationContext {
                tools: &tools,
                governor: &governor,
                deadline: Instant::now() + Duration::from_secs(60),
                keepalive: None,
                attachment_size_limit: None,
                diagram_limits: DiagramLimits::default(),
            },
        )
        .await
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pagination_detects_the_capped_set_and_rejects_missing_pages() {
        let all = (1..=CUKTA_FETCH_COUNT).collect::<Vec<_>>();
        let first =
            PagedResults::paginate(all.clone(), PageNumber::first(), MAX_PAGE).expect("page 1");
        assert_eq!(first.items, [1, 2, 3, 4, 5]);
        assert_eq!(first.page_count, MAX_PAGE as usize);
        assert_eq!(first.shown_total, PAGE_SIZE * MAX_PAGE as usize);
        assert!(first.capped, "cap + 1 results means the set is capped");
        let last = PagedResults::paginate(
            all.clone(),
            PageNumber::new(MAX_PAGE).expect("page"),
            MAX_PAGE,
        )
        .expect("last page");
        assert_eq!(last.items, [121, 122, 123, 124, 125]);
        let exact = PagedResults::paginate((1..=125).collect(), PageNumber::first(), MAX_PAGE)
            .expect("page");
        assert!(!exact.capped);
        let short = PagedResults::paginate(vec![1, 2, 3, 4, 5, 6], PageNumber::first(), MAX_PAGE)
            .expect("page");
        assert_eq!(short.page_count, 2);
        assert_eq!(
            PagedResults::paginate(vec![1, 2, 3], PageNumber::new(2).expect("page"), MAX_PAGE),
            Err(new!(PageUnavailable {
                requested: 2,
                available: 1
            }))
        );
        let empty =
            PagedResults::<u8>::paginate(Vec::new(), PageNumber::first(), MAX_PAGE).expect("page");
        assert!(empty.is_empty() && empty.page_count == 1);
        // The vlacku selector has room for 23 pages only.
        let vlacku = PagedResults::paginate(
            (1..=VLACKU_FETCH_COUNT).collect(),
            PageNumber::first(),
            VLACKU_MAX_PAGE,
        )
        .expect("page");
        assert_eq!(vlacku.page_count, VLACKU_MAX_PAGE as usize);
        assert!(vlacku.capped);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn preflight_reports_dialect_blank_and_source_record_problems() {
        let bad_dialect = DiscordRequest::Gentufa(GentufaRequest {
            text: text("mi klama"),
            dialect: Some(text("not-a-dialect((")),
            options: GentufaOptions::default(),
        });
        assert!(matches!(
            preflight(&bad_dialect),
            Err(RequestValidationError::Dialect { .. })
        ));
        let cleared_dialect = DiscordRequest::Gentufa(GentufaRequest {
            text: text("mi klama"),
            dialect: Some(text("  ")),
            options: GentufaOptions::default(),
        });
        assert_eq!(preflight(&cleared_dialect), Ok(()));
        let blank = DiscordRequest::Vlasei(VlaseiRequest {
            text: text(" \n"),
            dialect: None,
            options: VlaseiOptions::default(),
        });
        assert_eq!(
            preflight(&blank),
            Err(RequestValidationError::EmptyField {
                field: SourceField::Text
            })
        );
        let empty_parts = DiscordRequest::Jvozba(JvozbaRequest {
            parts: text("  "),
            options: JvozbaOptions::default(),
        });
        assert_eq!(
            preflight(&empty_parts),
            Err(RequestValidationError::EmptyField {
                field: SourceField::Parts
            })
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ordered_parts_keep_words_and_literal_rafsi_in_the_order_written() {
        let parse = |text: &str| {
            jvozba_inputs(&JvozbaRequest {
                parts: SourceText::new(text).expect("parts fit"),
                options: JvozbaOptions::default(),
            })
        };
        let word = |text: &str| JvozbaInput::Word(text.to_owned());
        let rafsi = |text: &str| JvozbaInput::FixedRafsi(text.to_owned());

        assert_eq!(
            parse("blanu -blo- zdani").expect("a mix"),
            vec![word("blanu"), rafsi("blo"), word("zdani")]
        );
        // A literal rafsi may open or close the compound, and two may follow
        // each other; order is whatever was written.
        assert_eq!(
            parse("-blo- zdani").expect("leading"),
            vec![rafsi("blo"), word("zdani")]
        );
        assert_eq!(
            parse("zdani -blo-").expect("trailing"),
            vec![word("zdani"), rafsi("blo")]
        );
        assert_eq!(
            parse("-blo--kla- zdani").expect("adjacent"),
            vec![rafsi("blo"), rafsi("kla"), word("zdani")]
        );
        // Apostrophes and the hyphen letter are ordinary rafsi text.
        assert_eq!(
            parse("mi -u'u- klama").expect("apostrophe"),
            vec![word("mi"), rafsi("u'u"), word("klama")]
        );
        // Words are still segmented by morphology, not by spaces: written
        // without a space, this is the cmavo `lo` followed by `jbobangu`,
        // which is what the parser must see.
        assert_eq!(
            parse("lojbobangu").expect("morphology decides"),
            vec![word("lo"), word("jbobangu")]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn malformed_ordered_parts_say_what_is_wrong() {
        let parse = |text: &str| {
            jvozba_inputs(&JvozbaRequest {
                parts: SourceText::new(text).expect("parts fit"),
                options: JvozbaOptions::default(),
            })
        };
        let message = |text: &str| match parse(text) {
            Err(RequestValidationError::JvozbaParts { message }) => message,
            other => panic!("expected a parts error for {text:?}, got {other:?}"),
        };

        assert!(
            message("blanu -blo").contains("never closed"),
            "{}",
            message("blanu -blo")
        );
        assert!(
            message("blanu -- zdani").contains("empty"),
            "{}",
            message("blanu -- zdani")
        );
        assert!(
            message("blanu -blo kla- zdani").contains("no spaces"),
            "{}",
            message("blanu -blo kla- zdani")
        );
        assert!(
            message("blanu -qwx- zdani").contains("not Lojban text"),
            "{}",
            message("blanu -qwx- zdani")
        );
        // A look-alike dash is not a delimiter; it reaches morphology as
        // ordinary text and is refused there rather than silently accepted.
        assert!(parse("blanu –blo– zdani").is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn word_and_source_problems_are_reported_from_the_governed_run() {
        // These need morphology and phonology, so they are found where that
        // work is admitted rather than on the runtime thread beforehand; the
        // caller still receives them as invalid input.
        let bad_parts = DiscordRequest::Jvozba(JvozbaRequest {
            parts: text("klama 'bajra"),
            options: JvozbaOptions::default(),
        });
        assert!(matches!(
            run(bad_parts).await,
            Err(OperationError::Invalid(
                RequestValidationError::JvozbaParts { .. }
            ))
        ));
        let bad_sources = DiscordRequest::Gimfihi(DiscordGimfihiRequest {
            sources: Some(text("eng:go, ???")),
            options: GimfihiOptions::default(),
        });
        assert!(matches!(
            run(bad_sources).await,
            Err(OperationError::Invalid(
                RequestValidationError::GimfihiSources { .. }
            ))
        ));
        // Weights are required without a preset; the shared resolver says so.
        let no_weights = DiscordRequest::Gimfihi(DiscordGimfihiRequest {
            sources: Some(text("eng:go; spa:ir")),
            options: GimfihiOptions::default(),
        });
        assert!(matches!(
            run(no_weights).await,
            Err(OperationError::Invalid(
                RequestValidationError::GimfihiSources { .. }
            ))
        ));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn gentufa_defaults_produce_a_parse_without_a_diagram() {
        let outcome = run(DiscordRequest::Gentufa(GentufaRequest {
            text: text("mi klama lo zarci"),
            dialect: None,
            options: GentufaOptions::default(),
        }))
        .await
        .expect("gentufa");
        let ToolOutcome::Gentufa(gentufa) = outcome else {
            panic!("gentufa outcome");
        };
        assert!(matches!(gentufa.result, GentufaWebResult::Success(_)));
        assert!(gentufa.diagram.is_none());
        let with_diagram = run(DiscordRequest::Gentufa(GentufaRequest {
            text: text("mi klama lo zarci"),
            dialect: None,
            options: GentufaOptions {
                include_diagram: true,
                ..GentufaOptions::default()
            },
        }))
        .await
        .expect("gentufa with diagram");
        let ToolOutcome::Gentufa(gentufa) = with_diagram else {
            panic!("gentufa outcome");
        };
        assert!(gentufa.diagram.is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn morphology_tools_return_typed_reports() {
        let ToolOutcome::Vlasei(vlasei) = run(DiscordRequest::Vlasei(VlaseiRequest {
            text: text("coiro'ido"),
            dialect: None,
            options: VlaseiOptions::default(),
        }))
        .await
        .expect("vlasei") else {
            panic!("vlasei outcome");
        };
        assert_eq!(
            vlasei.morphology.words.len(),
            3,
            "morphology, not spaces, finds the words"
        );
        let ToolOutcome::Vlatai(vlatai) = run(DiscordRequest::Vlatai(VlataiRequest {
            text: text("klama"),
            dialect: None,
            options: VlataiOptions::default(),
        }))
        .await
        .expect("vlatai") else {
            panic!("vlatai outcome");
        };
        assert!(vlatai.analysis.result.is_valid());
        assert!(!vlatai.possible_rafsi.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn vlacku_word_search_pages_and_flags_missing_pages() {
        let ToolOutcome::Vlacku(VlackuOutcome::Results {
            results,
            valid_missing,
            ..
        }) = run(DiscordRequest::Vlacku(new!(VlackuRequest {
            query: text("kla*"),
            options: VlackuOptions::default(),
        })))
        .await
        .expect("vlacku")
        else {
            panic!("vlacku results");
        };
        assert!(!valid_missing);
        assert!(!results.items.is_empty());
        assert!(results.items.iter().all(|card| card.known));
        let missing = run(DiscordRequest::Vlacku(new!(VlackuRequest {
            query: text("klabajra"),
            options: VlackuOptions::default(),
        })))
        .await
        .expect("vlacku");
        let ToolOutcome::Vlacku(VlackuOutcome::Results {
            results,
            valid_missing,
            ..
        }) = missing
        else {
            panic!("vlacku results");
        };
        // A valid lujvo absent from the dictionary yields a synthesized
        // analysis card flagged as such, followed (lujvo decomposition is the
        // default) by the attested cards of its components.
        assert!(
            valid_missing,
            "klabajra is a valid lujvo absent from the dictionary"
        );
        let (first, components) = results.items.split_first().expect("synthesized card");
        assert!(first.word == "klabajra" && !first.known, "{first:?}");
        assert!(!components.is_empty() && components.iter().all(|card| card.known));
        let too_far = run(DiscordRequest::Vlacku(new!(VlackuRequest {
            query: text("klama"),
            options: VlackuOptions {
                page: PageNumber::new(9).expect("page"),
                ..VlackuOptions::default()
            },
        })))
        .await;
        assert!(matches!(too_far, Err(OperationError::Page(_))));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn cukta_modes_produce_their_own_outcomes() {
        let contents = run(DiscordRequest::Cukta(CuktaRequest {
            query: None,
            options: CuktaOptions {
                mode: CuktaMode::Contents,
                kinds: CuktaResultKindSet::empty(),
                page: PageNumber::first(),
            },
        }))
        .await
        .expect("contents");
        let ToolOutcome::Cukta(CuktaOutcome::Contents { chapters, .. }) = contents else {
            panic!("contents outcome");
        };
        assert!(chapters.len() > 10);
        let section = run(DiscordRequest::Cukta(CuktaRequest {
            query: Some(text("5.2")),
            options: CuktaOptions {
                mode: CuktaMode::Section,
                kinds: CuktaResultKindSet::empty(),
                page: PageNumber::first(),
            },
        }))
        .await
        .expect("section");
        let ToolOutcome::Cukta(CuktaOutcome::Section { section, .. }) = section else {
            panic!("section outcome");
        };
        let heading = format_section_display_title(section);
        assert!(heading.starts_with("5.2"), "{heading}");
        assert!(!section.blocks.is_empty());
        let example = run(DiscordRequest::Cukta(CuktaRequest {
            query: Some(text("6.8")),
            options: CuktaOptions {
                mode: CuktaMode::Example,
                kinds: CuktaResultKindSet::empty(),
                page: PageNumber::first(),
            },
        }))
        .await
        .expect("example");
        assert!(matches!(
            example,
            ToolOutcome::Cukta(CuktaOutcome::Example { .. })
        ));
        let missing = run(DiscordRequest::Cukta(CuktaRequest {
            query: Some(text("999.99")),
            options: CuktaOptions {
                mode: CuktaMode::Section,
                kinds: CuktaResultKindSet::empty(),
                page: PageNumber::first(),
            },
        }))
        .await
        .expect("missing section");
        assert!(matches!(
            missing,
            ToolOutcome::Cukta(CuktaOutcome::NotFound {
                what: "section",
                ..
            })
        ));
        let word = run(DiscordRequest::Cukta(CuktaRequest {
            query: Some(text("tanru")),
            options: CuktaOptions {
                mode: CuktaMode::Word,
                kinds: CuktaResultKindSet::empty().with(CuktaResultKind::Section),
                page: PageNumber::first(),
            },
        }))
        .await
        .expect("word search");
        let ToolOutcome::Cukta(CuktaOutcome::Search { results, .. }) = word else {
            panic!("search outcome");
        };
        assert!(!results.items.is_empty());
        assert!(
            results
                .items
                .iter()
                .all(|card| card.kind == CllSearchChunkKind::Section)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn jvozba_keeps_fixed_rafsi_typed_and_explains_constituents() {
        let ToolOutcome::Jvozba(outcome) = run(DiscordRequest::Jvozba(JvozbaRequest {
            parts: text("klama bajra"),
            options: JvozbaOptions::default(),
        }))
        .await
        .expect("jvozba") else {
            panic!("jvozba outcome");
        };
        assert_eq!(
            outcome.inputs,
            [
                JvozbaInput::Word("klama".to_owned()),
                JvozbaInput::Word("bajra".to_owned())
            ]
        );
        let built = outcome.result.expect("lujvo built");
        assert!(!built.word.is_empty());
        let rafsi = built
            .constituents
            .iter()
            .filter(|constituent| !constituent.is_hyphen)
            .collect::<Vec<_>>();
        assert_eq!(rafsi.len(), 2);
        assert_eq!(rafsi[0].source.as_deref(), Some("klama"));
        assert_eq!(rafsi[1].source.as_deref(), Some("bajra"));

        // A literal rafsi keeps the place it was written in, before the word
        // that follows it.
        let ToolOutcome::Jvozba(mixed) = run(DiscordRequest::Jvozba(JvozbaRequest {
            parts: text("blanu -blo- zdani"),
            options: JvozbaOptions::default(),
        }))
        .await
        .expect("jvozba") else {
            panic!("jvozba outcome");
        };
        assert_eq!(
            mixed.inputs,
            vec![
                JvozbaInput::Word("blanu".to_owned()),
                JvozbaInput::FixedRafsi("blo".to_owned()),
                JvozbaInput::Word("zdani".to_owned()),
            ]
        );
        let ToolOutcome::Jvozba(single) = run(DiscordRequest::Jvozba(JvozbaRequest {
            parts: text("klama"),
            options: JvozbaOptions::default(),
        }))
        .await
        .expect("jvozba") else {
            panic!("jvozba outcome");
        };
        assert_eq!(single.result, Err(JvozbaError::RequiresAtLeastTwoInputs));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn gimfihi_without_source_words_is_a_setup_response() {
        let bare = run(DiscordRequest::Gimfihi(DiscordGimfihiRequest {
            sources: None,
            options: GimfihiOptions::default(),
        }))
        .await
        .expect("gimfihi");
        assert!(matches!(
            bare,
            ToolOutcome::Gimfihi(GimfihiOutcome::Setup { preset: None, .. })
        ));
        let preset_only = run(DiscordRequest::Gimfihi(DiscordGimfihiRequest {
            sources: Some(text("")),
            options: GimfihiOptions {
                preset: Some(GimfihiPreset::Ilmen6),
                ..GimfihiOptions::default()
            },
        }))
        .await
        .expect("gimfihi");
        let ToolOutcome::Gimfihi(GimfihiOutcome::Setup { preset, languages }) = preset_only else {
            panic!("setup outcome");
        };
        assert_eq!(preset, Some(GimfihiPreset::Ilmen6));
        // The language list is the shared preset table, in its own order.
        let expected = GimfihiPreset::Ilmen6
            .entries()
            .iter()
            .map(|entry| entry.language.to_owned())
            .collect::<Vec<_>>();
        assert_eq!(languages, expected);
        assert_eq!(languages.len(), 6);
        let composed = run(DiscordRequest::Gimfihi(DiscordGimfihiRequest {
            sources: Some(text("eng:5:go, spa:3:[ir]")),
            options: GimfihiOptions::default(),
        }))
        .await
        .expect("gimfihi");
        let ToolOutcome::Gimfihi(GimfihiOutcome::Candidates {
            results, sources, ..
        }) = composed
        else {
            panic!("candidates outcome");
        };
        assert_eq!(sources.len(), 2);
        assert!(!results.items.is_empty());
    }
}
