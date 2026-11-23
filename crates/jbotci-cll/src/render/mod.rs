use super::*;

mod discord;
mod html;
mod markdown;

pub(crate) use discord::escape_discord_markdown_at;
pub use discord::{
    DISCORD_MARKDOWN_ESCAPED, discord_code_block, discord_code_block_in_scope, discord_inline_code,
    discord_inline_code_in_scope, escape_discord_markdown, escape_discord_markdown_in_scope,
    escape_discord_markdown_line, escape_discord_markdown_line_in_scope,
};
#[cfg(test)]
pub(crate) use html::render_block_html;
pub(crate) use html::render_block_html_in_scope;
#[cfg(test)]
pub(crate) use markdown::render_block_markdown;
pub(crate) use markdown::{
    dialect_text_in_scope, render_block_markdown_to, render_example_markdown_to,
};

/// Splits `text` on a CommonMark line ending.
///
/// A line ending is LF, CRLF, or a lone CR. The ending bytes stay with that
/// line. The line content has no CR and no LF.
#[requires(true)]
#[ensures(!ret.is_empty())]
#[ensures(ret.last().is_some_and(|(_, ending)| ending.is_empty()))]
#[ensures(ret.iter().all(|(content, ending)| {
    !content.bytes().any(|byte| byte == b'\n' || byte == b'\r')
        && (ending.is_empty() || *ending == "\n" || *ending == "\r" || *ending == "\r\n")
}))]
#[ensures(
    ret.iter()
        .fold(0usize, |len, (content, ending)| len + content.len() + ending.len())
        == text.len()
)]
fn commonmark_lines(text: &str) -> Vec<(&str, &str)> {
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\n' {
            lines.push((&text[start..index], "\n"));
            index += 1;
            start = index;
        } else if bytes[index] == b'\r' {
            if bytes.get(index + 1) == Some(&b'\n') {
                lines.push((&text[start..index], "\r\n"));
                index += 2;
            } else {
                lines.push((&text[start..index], "\r"));
                index += 1;
            }
            start = index;
        } else {
            index += 1;
        }
    }
    lines.push((&text[start..], ""));
    lines
}

/// True when `line` is not a CommonMark blank line.
///
/// A blank line is empty, or it holds only spaces and tabs. A no-break
/// space is text.
#[requires(true)]
// The predicate is the function body. A blank line has no separate property.
#[ensures(true)]
fn commonmark_line_has_text(line: &str) -> bool {
    line.bytes().any(|byte| byte != b' ' && byte != b'\t')
}

/// Writes `text` as lines of a Markdown block quote.
///
/// A line ends at LF, CRLF, or a lone CR. Those ending bytes stay on the
/// line. Each line starts with `> `, and a blank line does too. Pass
/// `prefix_first` as false when the caller already wrote `> **` on the
/// first line. `commonmark_lines` adds an empty piece after the last
/// ending. That piece is not a line.
#[requires(true)]
#[ensures(true)]
pub(crate) fn push_markdown_quote_lines(
    output: &mut jbotci_budget::TextBuffer,
    text: &str,
    prefix_first: bool,
) {
    for (index, (content, ending)) in commonmark_lines(text).into_iter().enumerate() {
        if content.is_empty() && ending.is_empty() {
            break;
        }
        if prefix_first || index > 0 {
            output.push_str("> ");
        }
        output.push_str(content);
        output.push_str(ending);
    }
}

/// Writes a rule-status note as a labelled block quote.
///
/// `body` is already rendered in the target Markdown dialect.
#[requires(true)]
#[ensures(true)]
pub(crate) fn push_status_note_markdown(output: &mut jbotci_budget::TextBuffer, body: &str) {
    output.push_str("> **");
    output.push_str(CLL_STATUS_NOTE_LABEL);
    output.push_str(".** ");
    push_markdown_quote_lines(output, body, false);
}

/// Appends a rule-status note as a labelled block quote.
///
/// A status note is the edition's own annotation about the standing of the rule
/// being taught rather than part of the exposition, so it is set off from the
/// prose around it. Section rendering and search-result rendering share this so
/// a note looks the same however a reader reached it, and the label is literal
/// text because Markdown carries no styling.

/// A rule-status note as a labelled block quote. `body` is already rendered
/// in the target Markdown dialect.
#[requires(true)]
#[ensures(ret.contains(CLL_STATUS_NOTE_LABEL) && ret.starts_with("> **"))]
pub fn status_note_markdown(body: &str) -> String {
    let scope = jbotci_budget::TextScope::new(
        jbotci_budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    status_note_markdown_in_scope(body, &scope)
}
#[requires(true)]
#[ensures(scope.is_exceeded() || (ret.contains(CLL_STATUS_NOTE_LABEL) && ret.starts_with("> **")))]
pub fn status_note_markdown_in_scope(body: &str, scope: &jbotci_budget::TextScope) -> String {
    let mut output = scope.buffer();
    push_status_note_markdown(&mut output, body);
    output.into_piece()
}

/// Renders a rule-status note as a labelled aside.
///
/// `id_attribute` is empty or an already-escaped ` id="…"`, and `classes` comes
/// from this crate rather than from content. The label is emitted as text and
/// not only as a class, because the CLI and the MCP tool both hand this HTML to
/// a reader with no stylesheet attached.
#[requires(!classes.is_empty())]
#[ensures(scope.is_exceeded() || ret.contains(CLL_STATUS_NOTE_LABEL))]
pub(crate) fn render_status_note_html(
    id_attribute: &str,
    classes: &'static str,
    body: &str,
    scope: &jbotci_budget::TextScope,
) -> String {
    scope.format(format_args!(
        "<aside{id_attribute} class=\"{classes}\"><span class=\"cll-status-note-label\">{}</span>{CLL_STATUS_NOTE_LABEL_SEPARATOR}{body}</aside>",
        escape_html(CLL_STATUS_NOTE_LABEL),
    ))
}

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
