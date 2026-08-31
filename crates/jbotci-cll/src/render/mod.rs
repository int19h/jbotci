use super::*;

mod html;
mod markdown;

pub(crate) use html::render_block_html;
pub(crate) use markdown::render_block_markdown;

/// Appends a rule-status note as a labelled block quote.
///
/// A status note is the edition's own annotation about the standing of the rule
/// being taught rather than part of the exposition, so it is set off from the
/// prose around it. Section rendering and search-result rendering share this so
/// a note looks the same however a reader reached it, and the label is literal
/// text because Markdown carries no styling.
#[requires(true)]
#[ensures(output.contains(CLL_STATUS_NOTE_LABEL))]
pub(crate) fn push_status_note_markdown(output: &mut String, body: &str) {
    output.push_str("> **");
    output.push_str(CLL_STATUS_NOTE_LABEL);
    output.push_str(".** ");
    for (index, line) in body.lines().enumerate() {
        if index > 0 {
            output.push_str("\n> ");
        }
        output.push_str(line);
    }
}

/// Renders a rule-status note as a labelled aside.
///
/// `id_attribute` is empty or an already-escaped ` id="…"`, and `classes` comes
/// from this crate rather than from content. The label is emitted as text and
/// not only as a class, because the CLI and the MCP tool both hand this HTML to
/// a reader with no stylesheet attached.
#[requires(!classes.is_empty())]
#[ensures(ret.contains(CLL_STATUS_NOTE_LABEL))]
pub(crate) fn render_status_note_html(
    id_attribute: &str,
    classes: &'static str,
    body: &str,
) -> String {
    format!(
        "<aside{id_attribute} class=\"{classes}\"><span class=\"cll-status-note-label\">{}</span> {body}</aside>",
        escape_html(CLL_STATUS_NOTE_LABEL),
    )
}

/// The classes a status note carries when a section is read.
pub(crate) const CLL_STATUS_NOTE_BLOCK_CLASSES: &str = "cll-para cll-status-note";

/// The classes a status note carries when it is a search hit's preview.
pub(crate) const CLL_STATUS_NOTE_PREVIEW_CLASSES: &str = "cll-search-preview cll-status-note";

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CllPlainLinkDisposition {
    KeepContent,
    Drop,
}

impl CllLinkKind {
    /// Defines Plain behavior exhaustively for every semantic link kind.
    #[requires(true)]
    #[ensures(ret == CllPlainLinkDisposition::Drop -> self == CllLinkKind::Parse)]
    pub(crate) fn plain_disposition(self) -> CllPlainLinkDisposition {
        match self {
            // Section references carry a meaningful section title or number.
            CllLinkKind::Section => CllPlainLinkDisposition::KeepContent,
            // Example references carry a meaningful example label.
            CllLinkKind::Example => CllPlainLinkDisposition::KeepContent,
            // Dictionary links carry the referenced Lojban word.
            CllLinkKind::Dictionary => CllPlainLinkDisposition::KeepContent,
            // Rafsi links carry the referenced rafsi text.
            CllLinkKind::Rafsi => CllPlainLinkDisposition::KeepContent,
            // "Parse" is only an action; without its route it is noise.
            CllLinkKind::Parse => CllPlainLinkDisposition::Drop,
            // Asset links carry a descriptive label even without the path.
            CllLinkKind::Asset => CllPlainLinkDisposition::KeepContent,
            // External links carry descriptive text even without the URI.
            CllLinkKind::External => CllPlainLinkDisposition::KeepContent,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_ebnf_href(site: &CllSite, href: &str) -> String {
    if let Some(target) = href.strip_prefix("../vlacku/") {
        return cll_link_href(site, CllLinkKind::Dictionary, target);
    }
    href.to_owned()
}
