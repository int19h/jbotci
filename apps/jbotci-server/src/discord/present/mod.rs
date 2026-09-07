//! Discord presentation of typed outcomes.
//!
//! Presenters turn a [`ToolOutcome`] into a [`RenderedResult`]: Discord
//! Markdown chunks, rendered diagnostics, an optional diagram and the status
//! text that closes the input block. They never look at CLI output or format
//! enums, and they never truncate silently: the assembler applies the message
//! budget and turns overflow into an explicit excerpt plus attachment.

pub(crate) mod cukta;
pub(crate) mod diagnostics;
pub(crate) mod gentufa;
pub(crate) mod gimfihi;
pub(crate) mod jvozba;
pub(crate) mod markdown;
pub(crate) mod vlacku;
pub(crate) mod vlasei;
pub(crate) mod vlatai;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};

use self::diagnostics::RenderedDiagnostics;
use super::diagram::DiagramImage;
use super::operations::{PagedResults, RequestValidationError, ToolOutcome};
use super::request::{DiscordRequest, DiscordTool, utf16_len};

/// The status line that closes the input block: the tool name first, then
/// the applied view/page and the diagnostics count, on one line.
#[invariant(text.starts_with(tool.name()) && !text.contains('\n') && !text.contains('\r'))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResultStatus {
    pub(crate) tool: DiscordTool,
    pub(crate) text: String,
}

/// What one presenter produced for one request. Every field combination is
/// valid: the status line carries its own invariant and the rest is content.
///
/// A presenter that shows less than it has says so through
/// [`RenderedResult::show_excerpt_of`], which is what makes the assembler
/// attach the complete text. Nothing else in the message may promise a file:
/// only the assembler knows whether one exists.
#[invariant(true)]
#[derive(Debug, Clone)]
pub(crate) struct RenderedResult {
    pub(crate) status: ResultStatus,
    /// Result body chunks in order, each Discord Markdown.
    pub(crate) body: Vec<String>,
    /// Rendered diagnostics: what the message shows, and every diagnostic in
    /// full for the attached result.
    pub(crate) diagnostics: Option<RenderedDiagnostics>,
    /// A short notice kept even under overflow (capped result set, missing
    /// image explanation, continuation hint).
    pub(crate) notice: Option<String>,
    /// Complete plain text of the result for the attachment.
    pub(crate) full_text: Option<String>,
    /// Whether the body chunks show less than `full_text` holds. The
    /// assembler attaches the complete result whenever this is set, however
    /// short the message turns out to be.
    pub(crate) shows_excerpt: bool,
    pub(crate) image: Option<DiagramImage>,
}

impl RenderedResult {
    #[requires(status.starts_with(tool.name()))]
    #[ensures(ret.tool() == tool)]
    pub(crate) fn new(tool: DiscordTool, status: String) -> Self {
        RenderedResult {
            status: new!(ResultStatus {
                tool,
                text: status.replace(['\n', '\r'], " "),
            }),
            body: Vec::new(),
            diagnostics: None,
            notice: None,
            full_text: None,
            shows_excerpt: false,
            image: None,
        }
    }

    /// Record the complete plain text of a result the message shows whole.
    #[requires(true)]
    #[ensures(self.full_text.is_some())]
    pub(crate) fn set_full_text(&mut self, full_text: String) {
        self.full_text = Some(full_text);
    }

    /// Record that the body shows only part of `full_text`. The assembler
    /// attaches the whole of it and says so; presenters never promise a file
    /// themselves, because a promise they cannot keep is worse than a cut.
    #[requires(true)]
    #[ensures(self.shows_excerpt && self.full_text.is_some())]
    pub(crate) fn show_excerpt_of(&mut self, full_text: String) {
        self.full_text = Some(full_text);
        self.shows_excerpt = true;
    }

    /// Attach `diagnostics`, noting when they are shown in part.
    #[requires(true)]
    #[ensures(self.diagnostics.is_some() == old(diagnostics.is_some()))]
    pub(crate) fn set_diagnostics(&mut self, diagnostics: Option<RenderedDiagnostics>) {
        if let Some(diagnostics) = &diagnostics
            && diagnostics.is_excerpt
        {
            self.shows_excerpt = true;
        }
        self.diagnostics = diagnostics;
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn tool(&self) -> DiscordTool {
        self.status.tool
    }
}

/// Status suffix describing the page of a paged result.
#[requires(true)]
#[ensures(ret.starts_with("page "))]
pub(crate) fn page_status<T>(results: &PagedResults<T>) -> String {
    format!("page {}/{}", results.page.get(), results.page_count)
}

/// Present `outcome` for `request`. `app_link` is the app URL for the published
/// state when the tool has a web page; presenters mention it as the
/// continuation of a capped result set.
#[requires(true)]
#[ensures(ret.tool() == request.tool())]
pub(crate) fn render(
    outcome: &ToolOutcome,
    request: &DiscordRequest,
    app_link: Option<&str>,
) -> RenderedResult {
    match (outcome, request) {
        (ToolOutcome::Gentufa(outcome), DiscordRequest::Gentufa(request)) => {
            gentufa::render(outcome, request)
        }
        (ToolOutcome::Vlasei(analysis), DiscordRequest::Vlasei(request)) => {
            vlasei::render(analysis, request)
        }
        (ToolOutcome::Vlatai(report), DiscordRequest::Vlatai(request)) => {
            vlatai::render(report, request)
        }
        (ToolOutcome::Vlacku(outcome), DiscordRequest::Vlacku(request)) => {
            vlacku::render(outcome, request, app_link)
        }
        (ToolOutcome::Cukta(outcome), DiscordRequest::Cukta(request)) => {
            cukta::render(outcome, request, app_link)
        }
        (ToolOutcome::Jvozba(outcome), DiscordRequest::Jvozba(request)) => {
            jvozba::render(outcome, request)
        }
        (ToolOutcome::Gimfihi(outcome), DiscordRequest::Gimfihi(request)) => {
            gimfihi::render(outcome, request, app_link)
        }
        (outcome, request) => {
            // The operation layer produces the outcome for the request's own
            // tool; a mismatch is a programming error, reported rather than
            // rendered as someone else's result.
            let mut rendered = RenderedResult::new(
                request.tool(),
                format!("{} · internal error", request.tool()),
            );
            rendered.body.push(markdown::escape(&format!(
                "internal error: {} outcome for a {} request",
                outcome_tool(outcome),
                request.tool()
            )));
            rendered
        }
    }
}

/// Present a request that failed validation, for the slash path where there
/// is no previous result to keep: the problem plus the gear to fix it.
#[requires(true)]
#[ensures(ret.tool() == request.tool())]
pub(crate) fn render_validation_error(
    error: &RequestValidationError,
    request: &DiscordRequest,
) -> RenderedResult {
    render_failure(&error.to_string(), request)
}

/// Present a first result that could not be produced at all: what happened,
/// plus the gear that reopens the request. A failure is still a result, so it
/// is a message like any other rather than a private note beside a message
/// that never says anything.
#[requires(true)]
#[ensures(ret.tool() == request.tool())]
pub(crate) fn render_failure(reason: &str, request: &DiscordRequest) -> RenderedResult {
    let tool = request.tool();
    let mut rendered = RenderedResult::new(tool, format!("{tool} · not run"));
    // The reason comes from an error type, not from a result, so it is short
    // by construction; it is cut before escaping all the same, because one
    // unexpectedly long reason must not be what makes a message unsendable.
    let reason = reason
        .chars()
        .take(FAILURE_REASON_CHARS)
        .collect::<String>();
    rendered
        .body
        .push(format!("**Not run:** {}", markdown::escape(&reason)));
    rendered.notice = Some(markdown::subtext(
        "Use the ⚙️ button to correct the request.",
    ));
    rendered
}

/// Most characters of a failure reason a message repeats.
const FAILURE_REASON_CHARS: usize = 500;

#[requires(true)]
#[ensures(true)]
fn outcome_tool(outcome: &ToolOutcome) -> DiscordTool {
    match outcome {
        ToolOutcome::Gentufa(_) => DiscordTool::Gentufa,
        ToolOutcome::Vlasei(_) => DiscordTool::Vlasei,
        ToolOutcome::Vlatai(_) => DiscordTool::Vlatai,
        ToolOutcome::Vlacku(_) => DiscordTool::Vlacku,
        ToolOutcome::Cukta(_) => DiscordTool::Cukta,
        ToolOutcome::Jvozba(_) => DiscordTool::Jvozba,
        ToolOutcome::Gimfihi(_) => DiscordTool::Gimfihi,
    }
}

/// Notice for a result set the Discord presentation caps. The app link itself
/// lives in the ⚙️ form (it can be thousands of characters); the notice only
/// says that the continuation exists, so its length is bounded.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|notice| notice.starts_with("-# ") && utf16_len(notice) < 160))]
pub(crate) fn capped_notice<T>(
    results: &PagedResults<T>,
    app_link: Option<&str>,
) -> Option<String> {
    if !results.capped {
        return None;
    }
    let mut notice = format!(
        "Discord shows the first {} results; more exist.",
        results.shown_total
    );
    if app_link.is_some() {
        notice.push_str(" Open in app (⚙️) continues the full list.");
    }
    Some(markdown::subtext(&notice))
}
