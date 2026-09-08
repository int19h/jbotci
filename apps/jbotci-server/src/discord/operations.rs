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
//! outside the lane. A paged result is fetched one page at a time, and one
//! result past it, which is what says whether another page follows.

use std::fmt;
use std::num::NonZeroUsize;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_cll::{
    CllExample, CllParagraphRole, CllSearchChunkKind, CllSearchMatch, CllSection, CllSite,
    CuktaSearchMode, CuktaSearchWindow, CuktaTargetFilter, cll_lookup_example, cll_lookup_section,
    cll_numbered_title, cll_resolve_example_reference, cll_resolve_section_reference, cukta_search,
    embedded_cll_site,
};
use jbotci_dialect::parse_dialect_definition;
use jbotci_gimfihi::{
    GimfihiCandidate, GimfihiError, GimfihiOutput, GimfihiPreset, GimfihiRequest, GimfihiScorer,
    GimfihiSourceInput, ResolvedSource, parse_source_spec, resolve_sources,
};
use jbotci_jvozba::{
    JvozbaBuildLimits, JvozbaBuildResult, JvozbaError, JvozbaInput, JvozbaMode,
    build_best_jvozba_detailed_within, decompose_lujvo_like,
};
use jbotci_morphology::{
    LujvoPart, MorphologyOptions, PhonemeRenderOptions, Phonemes, normalize_lojban_input_text,
    segment_words_with_modifiers,
};
use jbotci_search::vlacku::{
    VlackuCard, VlackuOutcome as SearchOutcome, VlackuRequest as SearchRequest,
    VlackuSearchOptions, WordTypeFilter, dictionary_entry_card, normalize_word_type_filter,
    parse_word_type_filter, run_vlacku_requests, word_like_lookup_text,
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
    GimfihiRequest as DiscordGimfihiRequest, JvozbaRequest, JvozbaTarget, PAGE_SIZE, PageNumber,
    SourceField, SourceText, VlackuMode, VlackuRequest, VlaseiRequest, VlataiRequest,
};
use super::work::{WorkError, WorkGovernor, WorkKeepalive};
use crate::{SearchQuery, SemanticSearchError, SemanticSearchErrorData, ToolServices};

/// What one page asks a source for: its own results and one more, which is
/// what says whether another page follows. A source that can start at an
/// offset is asked for exactly this much wherever the page sits; a source
/// that can only rank from the beginning is asked for [`prefix_len`].
const WINDOW_LEN: usize = PAGE_SIZE + 1;
/// [`WINDOW_LEN`] as a positive count, for a worker that needs one.
const WINDOW_COUNT: NonZeroUsize = NonZeroUsize::new(WINDOW_LEN).unwrap();

/// Source label attached to Discord diagnostics.
pub(crate) const SOURCE_LABEL: &str = "<discord>";

/// Separators between explicitly entered gimfihi source records.
pub(crate) const GIMFIHI_RECORD_SEPARATORS: [char; 3] = [',', ';', '\n'];

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

/// One page of a result list, and what is honestly known about the rest.
///
/// A page is materialized on its own: the source is asked only for as much of
/// its ranking as this page needs, and rich cards are built only for what the
/// page shows. `total` is the complete number of results and is present only
/// when the source actually reached the end; a prefix that was cut short by
/// the request is never mistaken for a total.
#[invariant(items.len() <= PAGE_SIZE, "a page holds at most one page of results")]
#[invariant(
    total.is_none_or(|total| total >= items.len()),
    "a known total covers at least what this page shows"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PagedResults<T> {
    pub(crate) items: Vec<T>,
    pub(crate) page: PageNumber,
    /// The complete number of results when the source reached the end of them.
    pub(crate) total: Option<usize>,
    /// Another page follows this one.
    pub(crate) has_more: bool,
}

impl<T> PagedResults<T> {
    /// One page out of a complete list the caller already holds. The total is
    /// known exactly, because the list is all of it.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|paged| paged.page == page) || ret.is_err())]
    pub(crate) fn from_all(all: Vec<T>, page: PageNumber) -> Result<Self, PageUnavailable> {
        let total = all.len();
        let start = page.first_index();
        if start >= total && page.get() > 1 {
            return Err(new!(PageUnavailable {
                requested: page.get(),
                available: Some(total.div_ceil(PAGE_SIZE).max(1)),
            }));
        }
        let end = (start + PAGE_SIZE).min(total);
        let items = all.into_iter().skip(start).take(end - start).collect();
        Ok(new!(PagedResults {
            items,
            page,
            total: Some(total),
            has_more: end < total,
        }))
    }

    /// One page out of the beginning of a ranking. `fetched` is what the
    /// source returned when asked for [`prefix_len`] items; returning fewer
    /// than that is how a ranked source says it has reached the end.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|paged| paged.page == page) || ret.is_err())]
    pub(crate) fn from_prefix(fetched: Vec<T>, page: PageNumber) -> Result<Self, PageUnavailable> {
        let fetched_len = fetched.len();
        let exhausted = fetched_len < prefix_len(page);
        let start = page.first_index();
        if start >= fetched_len && page.get() > 1 {
            return Err(new!(PageUnavailable {
                requested: page.get(),
                // Falling short of the prefix is the source saying it reached
                // the end, so this count is the whole of it.
                available: Some(fetched_len.div_ceil(PAGE_SIZE).max(1)),
            }));
        }
        let end = (start + PAGE_SIZE).min(fetched_len);
        let has_more = fetched_len > start + PAGE_SIZE;
        let items = fetched.into_iter().skip(start).take(end - start).collect();
        Ok(new!(PagedResults {
            items,
            page,
            // Only an exhausted source knows the total; a prefix that stopped
            // because it was asked to stop knows nothing about the rest.
            total: exhausted.then_some(fetched_len),
            has_more,
        }))
    }

    /// One page out of a window the source was asked for directly: `window`
    /// is what it returned when asked for [`WINDOW_LEN`] results starting at
    /// this page's first one. Returning fewer than that is how a windowed
    /// source says the results end here, which is the only thing it says
    /// about the total; short of that, how many there are is unknown and the
    /// page says so rather than inventing a figure.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|paged| paged.page == page) || ret.is_err())]
    pub(crate) fn from_window(window: Vec<T>, page: PageNumber) -> Result<Self, PageUnavailable> {
        let has_more = window.len() > PAGE_SIZE;
        if window.is_empty() && page.get() > 1 {
            return Err(new!(PageUnavailable {
                requested: page.get(),
                available: None,
            }));
        }
        let items = window.into_iter().take(PAGE_SIZE).collect::<Vec<_>>();
        Ok(new!(PagedResults {
            total: (!has_more).then(|| page.first_index() + items.len()),
            items,
            page,
            has_more,
        }))
    }

    /// One page out of a window whose source also counted the whole result
    /// set while it worked, so the total is known without holding the results
    /// it counted.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|paged| paged.total == Some(total)) || ret.is_err())]
    pub(crate) fn from_counted_window(
        window: Vec<T>,
        page: PageNumber,
        total: usize,
    ) -> Result<Self, PageUnavailable> {
        let start = page.first_index();
        if start >= total && page.get() > 1 {
            return Err(new!(PageUnavailable {
                requested: page.get(),
                available: Some(total.div_ceil(PAGE_SIZE).max(1)),
            }));
        }
        let items = window.into_iter().take(PAGE_SIZE).collect::<Vec<_>>();
        Ok(new!(PagedResults {
            has_more: start + items.len() < total,
            total: Some(total),
            items,
            page,
        }))
    }

    #[requires(true)]
    #[ensures(ret == self.items.is_empty())]
    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Turn each item of this page into what the reader sees, keeping the
    /// page's own facts. Only the page is mapped, which is what keeps rich
    /// values page-sized however deep the page is.
    #[requires(true)]
    #[ensures(ret.page == old(self.page) && ret.total == old(self.total) && ret.has_more == old(self.has_more))]
    pub(crate) fn map<U, F: FnMut(T) -> U>(self, transform: F) -> PagedResults<U> {
        let data = self.into_data();
        new!(PagedResults {
            items: data.items.into_iter().map(transform).collect(),
            page: data.page,
            total: data.total,
            has_more: data.has_more,
        })
    }

    /// The one-based positions this page covers, for saying what is shown.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn range(&self) -> Option<(usize, usize)> {
        let first = self.page.first_index() + 1;
        (!self.items.is_empty()).then(|| (first, first + self.items.len() - 1))
    }
}

/// How much of a ranking one page needs: everything up to the end of it, and
/// one more result, which is what tells the page whether another follows.
#[requires(true)]
#[ensures(ret > PAGE_SIZE)]
pub(crate) fn prefix_len(page: PageNumber) -> usize {
    page.first_index() + WINDOW_LEN
}

/// [`prefix_len`] as a positive count, for a worker that needs one.
#[requires(true)]
#[ensures(ret.get() == prefix_len(page) || ret.get() == usize::MAX)]
pub(crate) fn prefix_count(page: PageNumber) -> NonZeroUsize {
    WINDOW_COUNT.saturating_add(page.first_index())
}

/// The requested page lies beyond the current result set. How many pages
/// there are is known only when the source reached the end of its results;
/// a windowed source that simply came back empty knows that this page is not
/// there and nothing more, and says that instead.
#[invariant(available.is_none_or(|available| *requested as usize > available))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PageUnavailable {
    pub(crate) requested: u16,
    pub(crate) available: Option<usize>,
}

impl fmt::Display for PageUnavailable {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.available {
            Some(available) => write!(
                formatter,
                "page {} is not available; this result has {} page{}",
                self.requested,
                available,
                if available == 1 { "" } else { "s" }
            ),
            None => write!(
                formatter,
                "page {} is not available; this result does not reach that far",
                self.requested
            ),
        }
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

/// The reader's filters, and the stretch of the results this page wants.
/// A search that can start where the page starts is given the page's own
/// window; one that can only rank from the beginning is given the prefix that
/// ends at this page.
#[requires(count > 0)]
#[ensures(ret.skip == skip && ret.count == count)]
fn vlacku_search_options(
    request: &VlackuRequest,
    skip: usize,
    count: usize,
) -> VlackuSearchOptions {
    let word_types = request
        .options
        .word_types
        .iter()
        .filter_map(|word_type| {
            parse_word_type_filter(&normalize_word_type_filter(word_type.filter_value()))
        })
        .collect::<Vec<WordTypeFilter>>();
    VlackuSearchOptions::default().with_data(data! {
        skip: skip,
        count: count,
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
        // Meaning is a ranking over the whole dictionary, so there is no
        // starting at this page: the worker ranks up to the end of it. That
        // count is what bounds the work, and the entry filters travel with
        // the job so they apply while it ranks. What it hands back is an
        // entry index and a score per hit; the cards come later, for one page.
        let page = request.options.page;
        let hits = match context
            .tools
            .semantic_vlacku_hits(
                search_query,
                prefix_count(page),
                vlacku_search_options(&request, 0, prefix_len(page)),
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
                let options = vlacku_search_options(&request, 0, prefix_len(page));
                // The hits are the ranking itself: the worker applied the
                // reader's filters while it ranked, so what came back is what
                // passed them, and how many came back is what says whether the
                // ranking reached its end. Cutting the list again here would
                // make a short list look like the end of the results when it
                // was only a second opinion about them.
                let results = PagedResults::from_prefix(hits, page)?.map(|hit| {
                    let entry = dictionary
                        .entries()
                        .get(hit.entry_index)
                        .expect("a ranked hit names an entry that was just read");
                    dictionary_entry_card(
                        dictionary,
                        entry,
                        Some(hit.score),
                        options.decompose_lujvo,
                    )
                });
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
            // A dictionary search can start where the page starts, so it is
            // asked for this page's window and builds cards for that alone.
            let options =
                vlacku_search_options(&request, request.options.page.first_index(), WINDOW_LEN);
            let output = run_vlacku_requests(
                jbotci_dictionary_data::english(),
                &[search_request],
                &options,
            );
            let valid_missing = output.outcome == SearchOutcome::ValidMissing
                || (output.outcome == SearchOutcome::Found && output.cards.is_empty());
            let results = PagedResults::from_window(output.cards, request.options.page)?;
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
                    prefix_count(request.options.page),
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
            // Matches are compact; only this page becomes cards.
            let results = PagedResults::from_prefix(output.matches, request.options.page)?
                .map(|matched| cukta_card(&matched));
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
                    // The book's index can start where the page starts, so
                    // the section text is copied for this page's window and
                    // not for everything before it.
                    let output = cukta_search(
                        site,
                        CuktaSearchMode::Word,
                        &query,
                        CuktaSearchWindow::after(page.first_index(), WINDOW_LEN),
                        targets,
                    );
                    let results = PagedResults::from_window(output.matches, page)?
                        .map(|matched| cukta_card(&matched));
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
        // The scorer ranks every generated candidate as words and scores,
        // and builds the full detail of the ones in this window only, so a
        // deep page costs a page rather than everything before it.
        skip: options.page.first_index(),
        count: WINDOW_LEN,
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
            // The scorer counted every candidate that passed the filters
            // while it ranked them, so the total is known exactly and is not
            // the size of what was kept.
            let results = PagedResults::from_counted_window(
                output.candidates,
                request.options.page,
                output.filtered_count,
            )?;
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[requires(true)]
    #[ensures(true)]
    async fn a_deep_page_is_fetched_and_shown_rather_than_refused() {
        // The old selector stopped at 25 pages, and the searches fetched only
        // what those pages could show. A page well beyond that is now an
        // ordinary request.
        let deep = PageNumber::new(30).expect("page 30");
        let outcome = run(DiscordRequest::Vlacku(VlackuRequest {
            query: text("*a*"),
            options: VlackuOptions {
                page: deep,
                ..VlackuOptions::default()
            },
        }))
        .await
        .expect("a deep page");
        let ToolOutcome::Vlacku(VlackuOutcome::Results { results, .. }) = outcome else {
            panic!("dictionary results");
        };
        assert_eq!(results.page, deep);
        assert_eq!(
            results.range().map(|(first, _)| first),
            Some(146),
            "page 30 starts where page 30 starts"
        );
        assert!(!results.items.is_empty(), "the page has its results");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_page_says_what_it_shows_and_only_claims_a_total_it_knows() {
        // A complete list knows its total exactly.
        let all = (1..=12).collect::<Vec<_>>();
        let first = PagedResults::from_all(all.clone(), PageNumber::first()).expect("page 1");
        assert_eq!(first.items, [1, 2, 3, 4, 5]);
        assert_eq!(first.total, Some(12));
        assert_eq!(first.range(), Some((1, 5)));
        assert!(first.has_more);
        let last =
            PagedResults::from_all(all.clone(), PageNumber::new(3).expect("page")).expect("page 3");
        assert_eq!(last.items, [11, 12]);
        assert_eq!(last.range(), Some((11, 12)));
        assert!(!last.has_more, "the last page says so");

        // A ranking cut short by the request knows only that more may follow.
        let page = PageNumber::first();
        let cut = (1..=prefix_len(page)).collect::<Vec<_>>();
        let prefix = PagedResults::from_prefix(cut, page).expect("page 1");
        assert_eq!(prefix.items, [1, 2, 3, 4, 5]);
        assert_eq!(prefix.total, None, "a fetched prefix is never a total");
        assert!(prefix.has_more);

        // A ranking that returned less than it was asked for has reached the
        // end, and its length is the real total.
        let exhausted =
            PagedResults::from_prefix(vec![1, 2, 3], PageNumber::first()).expect("page");
        assert_eq!(exhausted.total, Some(3));
        assert!(!exhausted.has_more);

        // Deep pages are addressable, and a page past the end is refused
        // rather than shown empty.
        let deep = PageNumber::new(400).expect("page 400");
        assert_eq!(prefix_len(deep), 399 * PAGE_SIZE + PAGE_SIZE + 1);
        assert_eq!(
            PagedResults::from_prefix(vec![1, 2, 3], deep),
            Err(new!(PageUnavailable {
                requested: 400,
                available: Some(1)
            }))
        );

        // An empty result set is page one with nothing on it.
        let empty = PagedResults::<u8>::from_all(Vec::new(), PageNumber::first()).expect("page");
        assert!(empty.is_empty() && empty.total == Some(0) && empty.range().is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_windowed_page_says_where_it_is_without_inventing_a_total() {
        // A source asked for this page and one more: the extra result says
        // another page follows, and says nothing about how many there are.
        let page = PageNumber::new(24).expect("page 24");
        assert_eq!(page.first_index(), 115);
        let window = (116..=121).collect::<Vec<_>>();
        assert_eq!(window.len(), WINDOW_LEN);
        let paged = PagedResults::from_window(window, page).expect("page 24");
        assert_eq!(paged.items, [116, 117, 118, 119, 120]);
        assert_eq!(paged.range(), Some((116, 120)));
        assert_eq!(paged.total, None, "a full window is not a count of results");
        assert!(paged.has_more);

        // A short window is the end of the results, and that is the one time
        // a windowed source knows the total.
        let last = PagedResults::from_window(vec![116, 117], page).expect("page 24");
        assert_eq!(last.total, Some(117));
        assert!(!last.has_more);

        // An empty window past the end refuses the page and does not claim to
        // know how many pages there are.
        assert_eq!(
            PagedResults::<u8>::from_window(Vec::new(), page),
            Err(new!(PageUnavailable {
                requested: 24,
                available: None
            }))
        );
        assert_eq!(
            new!(PageUnavailable {
                requested: 24,
                available: None
            })
            .to_string(),
            "page 24 is not available; this result does not reach that far"
        );

        // A source that counted its whole result set while it worked says so,
        // and the count is not the size of what it kept.
        let counted =
            PagedResults::from_counted_window(vec![116, 117, 118, 119, 120, 121], page, 11_864)
                .expect("page 24");
        assert_eq!(counted.total, Some(11_864));
        assert!(counted.has_more);
        let counted_last =
            PagedResults::from_counted_window(vec![116, 117], page, 117).expect("page 24");
        assert!(!counted_last.has_more && counted_last.total == Some(117));
        assert_eq!(
            PagedResults::from_counted_window(vec![1], PageNumber::new(30).expect("page"), 117),
            Err(new!(PageUnavailable {
                requested: 30,
                available: Some(24)
            }))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_ranking_is_asked_for_the_prefix_that_reaches_the_page_being_shown() {
        // A worker that can only rank from the beginning is asked for
        // everything up to the end of the page, and one more.
        assert_eq!(prefix_count(PageNumber::first()).get(), WINDOW_LEN);
        let deep = PageNumber::new(24).expect("page 24");
        assert_eq!(prefix_count(deep).get(), 121);
        assert_eq!(prefix_count(deep).get(), prefix_len(deep));

        // A meaning search over a set of 115 hits, page by page: the worker
        // ranks to the count it is given and stops there, so the pages have
        // to walk to the end of the results rather than to the end of the
        // first fetch.
        let ranked =
            |page: PageNumber| -> Vec<usize> { (1..=115).take(prefix_count(page).get()).collect() };
        let mut page = PageNumber::first();
        let mut seen = Vec::new();
        loop {
            let results = PagedResults::from_prefix(ranked(page), page).expect("page");
            seen.extend(results.items.iter().copied());
            if !results.has_more {
                assert_eq!(results.total, Some(115), "the end of a ranking is a total");
                assert_eq!(results.range(), Some((111, 115)));
                assert_eq!(page.get(), 23);
                break;
            }
            assert_eq!(results.total, None, "a prefix is never a total");
            page = page.next().expect("another page");
            assert!(page.get() <= 23, "the walk must not run past the results");
        }
        assert_eq!(seen, (1..=115).collect::<Vec<_>>());
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
        }) = run(DiscordRequest::Vlacku(VlackuRequest {
            query: text("kla*"),
            options: VlackuOptions::default(),
        }))
        .await
        .expect("vlacku")
        else {
            panic!("vlacku results");
        };
        assert!(!valid_missing);
        assert!(!results.items.is_empty());
        assert!(results.items.iter().all(|card| card.known));
        let missing = run(DiscordRequest::Vlacku(VlackuRequest {
            query: text("klabajra"),
            options: VlackuOptions::default(),
        }))
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
        let too_far = run(DiscordRequest::Vlacku(VlackuRequest {
            query: text("klama"),
            options: VlackuOptions {
                page: PageNumber::new(9).expect("page"),
                ..VlackuOptions::default()
            },
        }))
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
