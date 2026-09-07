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

use super::diagram::DiagramImage;
use super::operations::{PagedResults, RequestValidationError, ToolOutcome};
use super::request::{DiscordRequest, DiscordTool};

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
#[invariant(true)]
#[derive(Debug, Clone)]
pub(crate) struct RenderedResult {
    pub(crate) status: ResultStatus,
    /// Result body chunks in order, each Discord Markdown.
    pub(crate) body: Vec<String>,
    /// Rendered diagnostics, kept ahead of body overflow.
    pub(crate) diagnostics: Option<String>,
    /// A short notice kept even under overflow (capped result set, missing
    /// image explanation, continuation hint).
    pub(crate) notice: Option<String>,
    /// Complete plain text of the result for the overflow attachment.
    pub(crate) full_text: Option<String>,
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
            image: None,
        }
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
    let tool = request.tool();
    let mut rendered = RenderedResult::new(tool, format!("{tool} · not run"));
    rendered.body.push(format!(
        "**Not run:** {}",
        markdown::escape(&error.to_string())
    ));
    rendered.notice = Some(markdown::subtext(
        "Use the ⚙️ button to correct the request.",
    ));
    rendered
}

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

/// Notice for a result set the Discord presentation caps.
#[requires(true)]
#[ensures(true)]
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
    if let Some(link) = app_link {
        notice.push_str(&format!(" Continue in the app: {link}"));
    }
    Some(markdown::subtext(&notice))
}
