//! Assembling the published message from a rendered result and its request.
//!
//! The message carries, in this order: the Section with the input block and
//! the ⚙️ accessory, the result body, rendered diagnostics, notices, the
//! diagram gallery and any attachments. The application text budget is
//! applied here. When the result does not fit, the overflow notice is budgeted
//! first, then the presenter's notice, then diagnostics, then the body, and
//! the complete result (body and diagnostics) is attached; the notice states
//! whether that attachment is complete, cut at the server's attachment limit,
//! or impossible. The state-bearing source is never cut: a source that cannot
//! be carried exactly is a rejected candidate, not a partial message.

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use vec1::Vec1;

use super::codec::{
    INPUT_ATTACHMENT_FILENAME, INPUT_COMPONENT_ID, PageControl, RequestHeader, encode_input_block,
    input_block_fits_inline,
};
use super::components::{
    ActionRow, AttachmentName, AttachmentRequest, AttachmentRequestData, BoundsError, Button,
    CustomId, FileComponent, InvalidAttachmentName, MESSAGE_TEXT_BUDGET_UNITS, MediaGallery,
    MediaItem, MessageComponent, MessagePayload, PayloadError, Section, TextDisplay,
};
use super::present::markdown::{escape, subtext, truncate_units};
use super::present::{Pagination, RenderedResult};
use super::request::{PublishedRequest, Revision, SourceField, SourceStore, utf16_len};

/// Attachment carrying the result when the message shows an excerpt.
pub(crate) const RESULT_ATTACHMENT_FILENAME: &str = "jbotci-result.txt";
/// Attachment carrying the Gentufa diagram.
pub(crate) const DIAGRAM_ATTACHMENT_FILENAME: &str = "gentufa.png";
/// Units of the primary source shown inline when the source is attached.
const INPUT_PREVIEW_UNITS: usize = 200;
use super::transport::TEXT_CONTENT_TYPE;
const PNG_CONTENT_TYPE: &str = "image/png";
const FENCE_CLOSE: &str = "\n```";
/// Most units a presenter's notice keeps in the message under overflow, so an
/// informational line can never crowd out the body or the diagnostics.
/// Presenter notices are far shorter by construction; this only bounds them.
const OVERFLOW_NOTICE_RESERVE_UNITS: usize = 300;
/// Last line of a result attachment that had to be cut.
const ATTACHMENT_CUT_NOTE: &str =
    "\n[cut here: the complete result exceeds this server's attachment size limit]";

/// A message ready to publish, with the transport decisions it embodies.
#[invariant(header.store == *store)]
#[derive(Debug, Clone)]
pub(crate) struct AssembledMessage {
    pub(crate) payload: MessagePayload,
    /// Where the source fields live (inline block or attachment).
    pub(crate) store: SourceStore,
    /// The header encoded in the gear button.
    pub(crate) header: RequestHeader,
    /// Whether the message shows an excerpt of the result.
    pub(crate) overflowed: bool,
}

/// Why assembly failed. Bounds and payload cases are violations the layout is
/// built to avoid; the source case is a genuine limit of the interaction.
#[invariant(::Bounds(_) => true)]
#[invariant(::Payload(_) => true)]
#[invariant(::AttachmentName(_) => true)]
#[invariant(::SourceExceedsAttachmentLimit { bytes, limit } => bytes > limit)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AssembleError {
    Bounds(BoundsError),
    Payload(PayloadError),
    AttachmentName(InvalidAttachmentName),
    /// The encoded source does not fit the interaction's attachment limit;
    /// publishing a cut source would leave a message that cannot reopen.
    SourceExceedsAttachmentLimit {
        bytes: usize,
        limit: usize,
    },
}

impl std::fmt::Display for AssembleError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.as_data() {
            data!(AssembleError::Bounds(error)) => {
                write!(formatter, "message component out of bounds: {error}")
            }
            data!(AssembleError::Payload(error)) => {
                write!(formatter, "message payload invalid: {error}")
            }
            data!(AssembleError::AttachmentName(error)) => {
                write!(formatter, "attachment name invalid: {error}")
            }
            data!(AssembleError::SourceExceedsAttachmentLimit { bytes, limit }) => write!(
                formatter,
                "the source is {bytes} bytes but this server accepts attachments of at most {limit} bytes; shorten the input"
            ),
        }
    }
}

impl std::error::Error for AssembleError {}

impl From<BoundsError> for AssembleError {
    #[requires(true)]
    #[ensures(true)]
    fn from(error: BoundsError) -> Self {
        new!(AssembleError::Bounds(error))
    }
}

impl From<PayloadError> for AssembleError {
    #[requires(true)]
    #[ensures(true)]
    fn from(error: PayloadError) -> Self {
        new!(AssembleError::Payload(error))
    }
}

impl From<InvalidAttachmentName> for AssembleError {
    #[requires(true)]
    #[ensures(true)]
    fn from(error: InvalidAttachmentName) -> Self {
        new!(AssembleError::AttachmentName(error))
    }
}

/// The result attachment offered under overflow.
#[invariant(::Complete(bytes) => !bytes.is_empty())]
#[invariant(::Partial(bytes) => !bytes.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
enum ResultAttachment {
    /// The whole result fits the attachment limit.
    Complete(Vec<u8>),
    /// A prefix of the result, ending in a cut note.
    Partial(Vec<u8>),
    /// Not even a meaningful prefix fits.
    Unattachable,
}

/// Build the message for `rendered`, published as `published`.
/// `attachment_size_limit` is the interaction's per-file limit in bytes.
#[requires(rendered.tool() == published.request.tool())]
#[requires(attachment_size_limit > 0)]
#[ensures(ret.as_ref().is_ok_and(|message| message.payload.text_units() <= MESSAGE_TEXT_BUDGET_UNITS) || ret.is_err())]
pub(crate) fn assemble(
    rendered: &RenderedResult,
    published: &PublishedRequest,
    attachment_size_limit: usize,
) -> Result<AssembledMessage, AssembleError> {
    let tool = published.request.tool();
    let source_fields = published.request.source_fields();
    let fields = source_fields
        .iter()
        .map(|(field, value)| (*field, value.map(|value| value.as_str())))
        .collect::<Vec<_>>();
    let block = encode_input_block(tool, &fields, &rendered.status.text);
    let store = if input_block_fits_inline(&block) {
        SourceStore::Inline
    } else {
        if block.len() > attachment_size_limit {
            return Err(new!(AssembleError::SourceExceedsAttachmentLimit {
                bytes: block.len(),
                limit: attachment_size_limit,
            }));
        }
        SourceStore::Attachment
    };
    let section_text = match store {
        SourceStore::Inline => block.clone(),
        SourceStore::Attachment => attached_source_summary(&fields, &rendered.status.text),
    };
    let header = RequestHeader::describe(published, store);
    let gear = Button::gear(CustomId::new(&header.encode())?);
    let section_display = TextDisplay::new(Some(INPUT_COMPONENT_ID), section_text.clone())?;
    let mut components = vec![MessageComponent::Section(new!(Section {
        texts: Vec1::new(section_display),
        accessory: gear,
    }))];
    let mut attachments = Vec::new();
    let mut notices = Vec::new();

    // The diagram is bounded by the operations layer; a mismatch is stated,
    // never uploaded.
    let mut image = rendered.image.as_ref();
    if let Some(candidate) = image
        && candidate.bytes.len() > attachment_size_limit
    {
        notices.push(subtext(
            "The diagram was omitted: it exceeds this server's attachment size limit.",
        ));
        image = None;
    }

    let budget = MESSAGE_TEXT_BUDGET_UNITS.saturating_sub(utf16_len(&section_text));
    let body_full = rendered.body.join("\n\n");
    let diagnostics_full = rendered
        .diagnostics
        .as_ref()
        .map(|diagnostics| diagnostics.shown.as_str())
        .unwrap_or_default();
    let presenter_notice_units = rendered
        .notice
        .as_deref()
        .map(|notice| utf16_len(notice) + 1)
        .unwrap_or(0);
    let omitted_units = notices
        .iter()
        .map(|notice| utf16_len(notice) + 1)
        .sum::<usize>();
    // A presenter that already showed less than it has says so, and then the
    // complete result is attached however short the message turns out to be.
    let fits = !rendered.shows_excerpt
        && presenter_notice_units
            + omitted_units
            + utf16_len(diagnostics_full)
            + utf16_len(&body_full)
            <= budget;

    let (body_shown, diagnostics_shown, overflowed) = if fits {
        if let Some(notice) = &rendered.notice {
            notices.push(notice.clone());
        }
        (body_full, diagnostics_full.to_owned(), false)
    } else {
        // Overflow: the attachment decides the notice wording, so build it
        // first, then budget notice, presenter notice, diagnostics, body.
        let attachment = result_attachment(rendered, attachment_size_limit);
        let overflow_note = overflow_notice(&attachment);
        let mut remaining = budget.saturating_sub(omitted_units);
        remaining = remaining.saturating_sub(utf16_len(&overflow_note) + 1);
        notices.push(overflow_note);
        if let Some(notice) = &rendered.notice {
            let shown = cut_markdown(
                notice,
                remaining
                    .saturating_sub(1)
                    .min(OVERFLOW_NOTICE_RESERVE_UNITS),
            );
            remaining = remaining.saturating_sub(utf16_len(&shown) + 1);
            if !shown.is_empty() {
                notices.push(shown);
            }
        }
        // Diagnostics keep at least half of what is left so they cannot be
        // squeezed out by a long body, and the body is packed first so a
        // short body is never squeezed out by long diagnostics.
        let diagnostics_units = utf16_len(diagnostics_full);
        let diagnostics_reserve = if diagnostics_units == 0 {
            0
        } else {
            diagnostics_units.min(remaining / 2) + 1
        };
        let body_shown = pack_chunks(
            &rendered.body,
            remaining.saturating_sub(diagnostics_reserve),
        );
        if !body_shown.is_empty() {
            remaining = remaining.saturating_sub(utf16_len(&body_shown) + 1);
        }
        let diagnostics_shown = cut_markdown(diagnostics_full, remaining.saturating_sub(1));
        match attachment.into_data() {
            data!(ResultAttachment::Complete(bytes)) | data!(ResultAttachment::Partial(bytes)) => {
                let name = AttachmentName::new(RESULT_ATTACHMENT_FILENAME)?;
                components.push(MessageComponent::File(FileComponent {
                    attachment: name.clone(),
                }));
                attachments.push(new!(AttachmentRequest {
                    name,
                    content_type: TEXT_CONTENT_TYPE,
                    bytes,
                }));
            }
            data!(ResultAttachment::Unattachable) => {}
        }
        (body_shown, diagnostics_shown, true)
    };

    // Order: section, body, diagnostics, notices, gallery, files. The file
    // component pushed above is moved behind the text so the order holds.
    let trailing_files = components.split_off(1);
    if !body_shown.is_empty() {
        components.push(MessageComponent::TextDisplay(TextDisplay::new(
            None, body_shown,
        )?));
    }
    if !diagnostics_shown.is_empty() {
        components.push(MessageComponent::TextDisplay(TextDisplay::new(
            None,
            diagnostics_shown,
        )?));
    }
    if !notices.is_empty() {
        components.push(MessageComponent::TextDisplay(TextDisplay::new(
            None,
            notices.join("\n"),
        )?));
    }
    if let Some(image) = image {
        let name = AttachmentName::new(DIAGRAM_ATTACHMENT_FILENAME)?;
        components.push(MessageComponent::MediaGallery(new!(MediaGallery {
            items: Vec1::new(new!(MediaItem {
                attachment: name.clone(),
                description: Some(format!(
                    "Gentufa parse diagram, {}×{} pixels",
                    image.width, image.height
                )),
            })),
        })));
        attachments.push(new!(AttachmentRequest {
            name,
            content_type: PNG_CONTENT_TYPE,
            bytes: image.bytes.clone(),
        }));
    }
    components.extend(trailing_files);
    if store == SourceStore::Attachment {
        let name = AttachmentName::new(INPUT_ATTACHMENT_FILENAME)?;
        components.push(MessageComponent::File(FileComponent {
            attachment: name.clone(),
        }));
        attachments.push(new!(AttachmentRequest {
            name,
            content_type: TEXT_CONTENT_TYPE,
            bytes: block.into_bytes(),
        }));
    }
    // The pager goes last, under everything it turns.
    if let Some(pagination) = rendered.pagination
        && (pagination.has_previous || pagination.has_next)
    {
        components.push(MessageComponent::ActionRow(page_row(
            pagination,
            published.revision,
        )?));
    }
    let payload = MessagePayload::new(
        Vec1::try_from_vec(components).expect("the section is always present"),
        attachments,
    )?;
    Ok(new!(AssembledMessage {
        payload,
        store,
        header,
        overflowed,
    }))
}

/// The Section text when the source travels as an attachment: a short
/// preview of the primary field, a pointer to the file, and the status line.
#[requires(!fields.is_empty())]
#[requires(!status.is_empty() && !status.contains('\n'))]
#[ensures(ret.ends_with(status))]
fn attached_source_summary(fields: &[(SourceField, Option<&str>)], status: &str) -> String {
    let primary = fields
        .first()
        .and_then(|(_, value)| *value)
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let (preview, _) = truncate_units(&primary, INPUT_PREVIEW_UNITS);
    let mut lines = Vec::new();
    if !preview.is_empty() {
        lines.push(escape(&preview));
    }
    lines.push(subtext(&format!(
        "full source attached as {INPUT_ATTACHMENT_FILENAME}"
    )));
    lines.push(format!("-# {status}"));
    lines.join("\n")
}

/// The complete result as attachment text: the presenter's own full text (or
/// the shown body when it showed everything), followed by every diagnostic in
/// full, not the shortened form the message carries.
#[requires(true)]
#[ensures(true)]
fn complete_result_text(rendered: &RenderedResult) -> String {
    let mut text = rendered
        .full_text
        .clone()
        .unwrap_or_else(|| rendered.body.join("\n\n"));
    if let Some(diagnostics) = &rendered.diagnostics {
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        text.push_str("Diagnostics\n\n");
        text.push_str(&diagnostics.complete);
    }
    text
}

#[requires(limit > 0)]
#[ensures(match ret.as_data() { data!(ResultAttachment::Complete(bytes)) | data!(ResultAttachment::Partial(bytes)) => bytes.len() <= limit, data!(ResultAttachment::Unattachable) => true })]
fn result_attachment(rendered: &RenderedResult, limit: usize) -> ResultAttachment {
    let text = complete_result_text(rendered);
    if text.is_empty() {
        return new!(ResultAttachment::Unattachable);
    }
    if text.len() <= limit {
        return new!(ResultAttachment::Complete(text.into_bytes()));
    }
    let Some(mut end) = limit.checked_sub(ATTACHMENT_CUT_NOTE.len()) else {
        return new!(ResultAttachment::Unattachable);
    };
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    if end == 0 {
        return new!(ResultAttachment::Unattachable);
    }
    let mut bytes = text[..end].as_bytes().to_vec();
    bytes.extend_from_slice(ATTACHMENT_CUT_NOTE.as_bytes());
    new!(ResultAttachment::Partial(bytes))
}

#[requires(true)]
#[ensures(ret.starts_with("-# "))]
fn overflow_notice(attachment: &ResultAttachment) -> String {
    let text = match attachment.as_data() {
        data!(ResultAttachment::Complete(_)) => format!(
            "The message shows an excerpt; the complete result is attached as {RESULT_ATTACHMENT_FILENAME}."
        ),
        data!(ResultAttachment::Partial(_)) => format!(
            "The message shows an excerpt; the attached {RESULT_ATTACHMENT_FILENAME} is itself cut at this server's attachment size limit."
        ),
        data!(ResultAttachment::Unattachable) => "The message shows an excerpt; the complete result exceeds this server's attachment size limit and could not be attached.".to_owned(),
    };
    subtext(&text)
}

/// As many leading chunks as fit in `available` units; when not even the
/// first fits, a fence-safe cut of it.
#[requires(true)]
#[ensures(utf16_len(&ret) <= available)]
fn pack_chunks(chunks: &[String], available: usize) -> String {
    let mut kept = String::new();
    for chunk in chunks {
        let separator = if kept.is_empty() { 0 } else { 2 };
        if utf16_len(&kept) + separator + utf16_len(chunk) > available {
            if kept.is_empty() {
                return cut_markdown(chunk, available);
            }
            break;
        }
        if !kept.is_empty() {
            kept.push_str("\n\n");
        }
        kept.push_str(chunk);
    }
    kept
}

/// Cut Markdown to at most `max_units` UTF-16 units: never inside a backtick
/// run or an escape, and never leaving a code fence open. When even a closed
/// fence cannot fit, the result is empty rather than over budget.
#[requires(true)]
#[ensures(utf16_len(&ret) <= max_units)]
#[ensures(ret.matches("```").count() % 2 == 0)]
fn cut_markdown(markdown: &str, max_units: usize) -> String {
    if utf16_len(markdown) <= max_units {
        return if markdown.matches("```").count() % 2 == 0 {
            markdown.to_owned()
        } else {
            // Unbalanced input is not produced by the presenters; closing it
            // keeps the postcondition rather than trusting the caller.
            cut_markdown(markdown, max_units.saturating_sub(1))
        };
    }
    let fence_units = utf16_len(FENCE_CLOSE);
    let mut budget = max_units;
    loop {
        if budget == 0 {
            return String::new();
        }
        let (cut, _) = truncate_units(markdown, budget);
        let core = cut
            .strip_suffix('…')
            .unwrap_or(&cut)
            .trim_end_matches('`')
            .trim_end_matches('\\')
            .trim_end();
        let mut cut = format!("{core}…");
        if utf16_len(&cut) > budget {
            // Only possible when nothing but the ellipsis fits.
            cut = String::new();
        }
        if cut.matches("```").count() % 2 == 1 {
            if utf16_len(&cut) + fence_units <= max_units {
                cut.push_str(FENCE_CLOSE);
                return cut;
            }
            budget = budget.saturating_sub(fence_units);
            continue;
        }
        return cut;
    }
}

/// The two page buttons for one published page. A direction that does not
/// exist is shown disabled rather than removed, so the row keeps its shape as
/// the reader moves; a page with neither direction gets no row at all.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|row| row.buttons.len() == 2) || ret.is_err())]
fn page_row(pagination: Pagination, revision: Revision) -> Result<ActionRow, AssembleError> {
    let previous = PageControl {
        from_revision: revision,
        target: pagination.page.previous().unwrap_or(pagination.page),
    };
    let next = PageControl {
        from_revision: revision,
        target: pagination.page.next().unwrap_or(pagination.page),
    };
    let previous_id = CustomId::new(&previous.encode())?;
    let next_id = CustomId::new(&next.encode())?;
    Ok(new!(ActionRow {
        buttons: vec![
            Button::page(previous_id, "Previous", !pagination.has_previous),
            Button::page(next_id, "Next", !pagination.has_next),
        ],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord::codec::INLINE_INPUT_BUDGET_UNITS;
    use crate::discord::components::FLAG_IS_COMPONENTS_V2;
    use crate::discord::diagram::DiagramImage;
    use crate::discord::operations::{PagedResults, VlackuOutcome};
    use crate::discord::present::diagnostics::{RenderedDiagnostics, render_diagnostics};
    use crate::discord::present::{vlacku, vlasei};
    use crate::discord::request::{
        BuildTag, DiscordRequest, DiscordTool, GentufaOptions, GentufaRequest, PageNumber,
        Revision, Snowflake, SourceText, VlackuOptions, VlackuRequest, VlaseiOptions,
        VlaseiRequest,
    };

    #[requires(true)]
    #[ensures(true)]
    fn published(text: &str) -> PublishedRequest {
        PublishedRequest {
            request: DiscordRequest::Gentufa(GentufaRequest {
                text: SourceText::new(text).expect("text"),
                dialect: None,
                options: GentufaOptions::default(),
            }),
            revision: Revision::INITIAL,
            initiator: Snowflake::parse("123456789012345678").expect("snowflake"),
            build_tag: BuildTag::current(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn diagnostics(shown: &str, complete: &str, is_excerpt: bool) -> RenderedDiagnostics {
        new!(RenderedDiagnostics {
            shown: shown.to_owned(),
            complete: complete.to_owned(),
            is_excerpt,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn published_vlasei(text: &str) -> PublishedRequest {
        PublishedRequest {
            request: DiscordRequest::Vlasei(VlaseiRequest {
                text: SourceText::new(text).expect("text"),
                dialect: None,
                options: VlaseiOptions::default(),
            }),
            revision: Revision::INITIAL,
            initiator: Snowflake::parse("123456789012345678").expect("snowflake"),
            build_tag: BuildTag::current(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn rendered(body: Vec<String>) -> RenderedResult {
        let mut rendered =
            RenderedResult::new(DiscordTool::Gentufa, "gentufa · brackets".to_owned());
        rendered.body = body;
        rendered
    }

    #[requires(true)]
    #[ensures(true)]
    fn text_displays(message: &AssembledMessage) -> Vec<&str> {
        message
            .payload
            .components
            .iter()
            .filter_map(|component| match component {
                MessageComponent::TextDisplay(text) => Some(text.content.as_str()),
                _ => None,
            })
            .collect()
    }

    #[requires(true)]
    #[ensures(true)]
    fn attachment_names(message: &AssembledMessage) -> Vec<String> {
        message
            .payload
            .attachments
            .iter()
            .map(|attachment| attachment.name().as_str().to_owned())
            .collect()
    }

    #[requires(true)]
    #[ensures(true)]
    fn upload_bytes(message: &AssembledMessage, name: &str) -> Vec<u8> {
        message
            .payload
            .attachments
            .iter()
            .find(|attachment| attachment.name().as_str() == name)
            .map(|attachment| {
                let data!(AttachmentRequest { bytes, .. }) = attachment.as_data();
                bytes.clone()
            })
            .expect("uploaded attachment")
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn short_result_is_one_section_and_one_body_without_attachments() {
        let message = assemble(
            &rendered(vec!["`( mi klama )`".to_owned()]),
            &published("mi klama"),
            25 << 20,
        )
        .expect("assembled");
        assert_eq!(message.store, SourceStore::Inline);
        assert!(!message.overflowed);
        assert_eq!(message.payload.components.len(), 2);
        assert!(message.payload.attachments.is_empty());
        let json = message.payload.to_json();
        assert_eq!(json["flags"].as_u64(), Some(FLAG_IS_COMPONENTS_V2));
        let section = &json["components"][0];
        assert_eq!(
            section["components"][0]["id"].as_u64(),
            Some(u64::from(INPUT_COMPONENT_ID))
        );
        let block = section["components"][0]["content"].as_str().expect("block");
        assert!(
            block.starts_with("mi klama\n-# gentufa · brackets"),
            "{block}"
        );
        assert!(
            section["accessory"]["custom_id"]
                .as_str()
                .is_some_and(|id| id.starts_with("j1.gi")),
            "{section}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn long_source_moves_to_an_exact_attachment_with_a_preview() {
        let source = "mi klama ".repeat(200);
        let message = assemble(
            &rendered(vec!["body".to_owned()]),
            &published(&source),
            25 << 20,
        )
        .expect("assembled");
        assert_eq!(message.store, SourceStore::Attachment);
        assert!(
            message.header.encode().starts_with("j1.ga"),
            "{}",
            message.header.encode()
        );
        assert_eq!(
            attachment_names(&message),
            vec![INPUT_ATTACHMENT_FILENAME.to_owned()]
        );
        let MessageComponent::Section(section) = &message.payload.components[0] else {
            panic!("section first");
        };
        let text = &section.texts[0].content;
        assert!(utf16_len(text) < INLINE_INPUT_BUDGET_UNITS, "{text}");
        assert!(text.ends_with("-# gentufa · brackets"), "{text}");
        assert!(
            text.contains("full source attached as jbotci-input.txt"),
            "{text}"
        );
        let block =
            String::from_utf8(upload_bytes(&message, INPUT_ATTACHMENT_FILENAME)).expect("utf-8");
        assert!(block.ends_with("-# gentufa · brackets"), "{block}");
        assert!(
            block.starts_with(&source.trim_end().replace(' ', " ")),
            "{block}"
        );
        assert!(matches!(
            message.payload.components.last(),
            MessageComponent::File(_)
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn source_over_the_attachment_limit_is_rejected_not_cut() {
        let source = "mi klama ".repeat(200);
        let error = assemble(
            &rendered(vec!["body".to_owned()]),
            &published(&source),
            1000,
        )
        .expect_err("rejected");
        assert!(
            matches!(
                error.as_data(),
                data!(AssembleError::SourceExceedsAttachmentLimit { bytes, limit }) if *bytes > 1000 && *limit == 1000
            ),
            "{error:?}"
        );
        assert!(error.to_string().contains("shorten the input"), "{error}");
        // The same source with a sufficient limit is carried exactly.
        assemble(
            &rendered(vec!["body".to_owned()]),
            &published(&source),
            4096,
        )
        .expect("fits the limit");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn oversized_body_becomes_an_excerpt_plus_complete_attachment_within_budget() {
        let chunks = (0..40)
            .map(|index| format!("```\nchunk {index}\n{}\n```", "x".repeat(150)))
            .collect::<Vec<_>>();
        let mut result = rendered(chunks);
        result.full_text = Some("FULL".repeat(1000));
        result.diagnostics = Some(diagnostics("**1 warning**", "warning w: one", false));
        result.notice = Some("-# note".to_owned());
        let message = assemble(&result, &published("mi klama"), 25 << 20).expect("assembled");
        assert!(message.overflowed);
        assert!(message.payload.text_units() <= MESSAGE_TEXT_BUDGET_UNITS);
        let texts = text_displays(&message);
        assert_eq!(texts.len(), 3, "{texts:?}");
        assert_eq!(
            texts[0].matches("```").count() % 2,
            0,
            "fences balanced: {}",
            texts[0]
        );
        assert!(texts[0].contains("chunk 0") && !texts[0].contains("chunk 39"));
        assert_eq!(texts[1], "**1 warning**");
        assert!(
            texts[2]
                .starts_with("-# The message shows an excerpt; the complete result is attached")
                && texts[2].ends_with("\n-# note"),
            "{}",
            texts[2]
        );
        assert_eq!(
            attachment_names(&message),
            vec![RESULT_ATTACHMENT_FILENAME.to_owned()]
        );
        let attached =
            String::from_utf8(upload_bytes(&message, RESULT_ATTACHMENT_FILENAME)).expect("utf-8");
        assert!(attached.starts_with("FULLFULL"), "{attached}");
        assert!(
            attached.ends_with("Diagnostics\n\nwarning w: one"),
            "complete diagnostics travel with the result: {}",
            &attached[attached.len() - 40..]
        );
        assert!(matches!(
            message.payload.components.last(),
            MessageComponent::File(_)
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn long_diagnostics_with_a_short_body_overflow_visibly() {
        let mut result = rendered(vec!["**Parse failed.**".to_owned()]);
        let long = (0..120)
            .map(|index| format!("**error {index}** at 1:{index}: unexpected word"))
            .collect::<Vec<_>>()
            .join("\n");
        result.diagnostics = Some(diagnostics(&long, &long, false));
        let message = assemble(&result, &published("mi klama"), 25 << 20).expect("assembled");
        assert!(message.overflowed);
        assert!(message.payload.text_units() <= MESSAGE_TEXT_BUDGET_UNITS);
        let texts = text_displays(&message);
        assert_eq!(texts[0], "**Parse failed.**", "the short body still shows");
        assert!(
            texts[1].starts_with("**error 0**") && texts[1].ends_with('…'),
            "{}",
            texts[1]
        );
        assert!(
            texts[2].contains("complete result is attached"),
            "{}",
            texts[2]
        );
        let attached =
            String::from_utf8(upload_bytes(&message, RESULT_ATTACHMENT_FILENAME)).expect("utf-8");
        assert!(
            attached.contains("**error 119**"),
            "diagnostics complete in the attachment"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn attachment_cut_is_labelled_and_impossible_attachment_is_stated() {
        let mut result = rendered(vec!["z".repeat(6000)]);
        result.full_text = Some("z".repeat(6000));
        let partial = assemble(&result, &published("mi klama"), 500).expect("assembled");
        let bytes = upload_bytes(&partial, RESULT_ATTACHMENT_FILENAME);
        assert!(bytes.len() <= 500);
        assert!(
            String::from_utf8(bytes)
                .expect("utf-8")
                .ends_with("attachment size limit]")
        );
        assert!(
            text_displays(&partial)
                .last()
                .is_some_and(|notice| notice.contains("is itself cut")),
            "{:?}",
            text_displays(&partial)
        );
        let impossible = assemble(&result, &published("mi klama"), 8).expect("assembled");
        assert!(attachment_names(&impossible).is_empty());
        assert!(
            text_displays(&impossible)
                .last()
                .is_some_and(|notice| notice.contains("could not be attached")),
            "{:?}",
            text_displays(&impossible)
        );
        assert!(impossible.payload.text_units() <= MESSAGE_TEXT_BUDGET_UNITS);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn first_chunk_alone_over_budget_is_cut_with_its_fence_closed() {
        let huge = format!("```\n{}\n```", "y".repeat(6000));
        let message =
            assemble(&rendered(vec![huge]), &published("mi klama"), 25 << 20).expect("assembled");
        assert!(message.overflowed);
        let body = text_displays(&message)[0];
        assert!(body.ends_with("…\n```"), "{}", &body[body.len() - 20..]);
        assert!(message.payload.text_units() <= MESSAGE_TEXT_BUDGET_UNITS);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn huge_presenter_notice_still_yields_a_message_within_budget() {
        let mut result = rendered(vec!["body".to_owned()]);
        result.notice = Some(format!("-# {}", "n".repeat(3995)));
        result.diagnostics = Some(diagnostics("**diag**", "diag in full", false));
        let message = assemble(&result, &published("mi klama"), 25 << 20).expect("assembled");
        assert!(message.payload.text_units() <= MESSAGE_TEXT_BUDGET_UNITS);
        assert!(message.overflowed);
        let texts = text_displays(&message);
        assert_eq!(texts[0], "body", "the short body still shows: {texts:?}");
        assert_eq!(
            texts[1], "**diag**",
            "short diagnostics still show: {texts:?}"
        );
        assert!(
            texts[2].starts_with("-# The message shows an excerpt"),
            "{}",
            texts[2]
        );
        assert!(
            texts[2].ends_with('…'),
            "the presenter notice is cut visibly: {}",
            texts[2]
        );
        assert!(utf16_len(texts[2]) < 500, "{}", utf16_len(texts[2]));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagram_is_a_gallery_over_an_uploaded_png_unless_over_the_limit() {
        let mut result = rendered(vec!["body".to_owned()]);
        result.image = Some(new!(DiagramImage {
            bytes: vec![0x89, b'P', b'N', b'G'],
            width: 10,
            height: 5,
        }));
        let message = assemble(&result, &published("mi klama"), 25 << 20).expect("assembled");
        assert!(matches!(
            message.payload.components[2],
            MessageComponent::MediaGallery(_)
        ));
        let json = message.payload.to_json();
        assert_eq!(
            json["attachments"][0]["filename"].as_str(),
            Some(DIAGRAM_ATTACHMENT_FILENAME)
        );
        assert_eq!(message.payload.uploads().len(), 1);
        let omitted = assemble(&result, &published("mi klama"), 3).expect("assembled");
        assert!(omitted.payload.attachments.is_empty());
        assert!(
            text_displays(&omitted)
                .last()
                .is_some_and(|notice| notice.contains("diagram was omitted"))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_word_list_longer_than_the_message_shows_is_attached_even_when_short() {
        // Eighty-one one-syllable words: the message lists what it lists, and
        // the rest still reaches the reader, though nothing here is near the
        // text budget.
        let source = ".a ".repeat(81);
        let request = VlaseiRequest {
            text: SourceText::new(&source).expect("text"),
            dialect: None,
            options: VlaseiOptions::default(),
        };
        let analysis = jbotci_web_core::analyze_vlasei(&source, None, None).expect("analysis");
        let result = vlasei::render(&analysis, &request);
        assert!(
            result.shows_excerpt,
            "the presenter showed part of the list"
        );
        let message = assemble(&result, &published_vlasei(&source), 25 << 20).expect("assembled");
        assert!(
            message.payload.text_units() < MESSAGE_TEXT_BUDGET_UNITS / 2,
            "the message is far inside the budget: {}",
            message.payload.text_units()
        );
        assert!(message.overflowed, "and still carries the complete list");
        let attached =
            String::from_utf8(upload_bytes(&message, RESULT_ATTACHMENT_FILENAME)).expect("utf-8");
        assert_eq!(
            attached
                .lines()
                .filter(|line| line.contains("cmavo"))
                .count(),
            81,
            "every word is attached"
        );
        let shown = text_displays(&message).join("\n");
        assert!(
            shown.contains("showing the first 80 of 81 words"),
            "{shown}"
        );
        assert!(
            text_displays(&message)
                .last()
                .is_some_and(|notice| notice.contains("complete result is attached")),
            "the promise is the assembler's, and it is kept"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_dictionary_card_cut_short_is_attached_in_full() {
        let definition = format!("x1 {} x2", "very long gloss ".repeat(40));
        let card = new!(jbotci_search::vlacku::VlackuCard {
            word: "brodavla".to_owned(),
            word_type: "gismu".to_owned(),
            known: true,
            selmaho: None,
            author: None,
            is_official: true,
            similarity: None,
            votes: None,
            rafsi: Vec::new(),
            glosses: Vec::new(),
            definition: definition.clone(),
            notes: String::new(),
            etymology: None,
            decomposition: Vec::new(),
        });
        let request = VlackuRequest {
            query: SourceText::new("brodavla").expect("text"),
            options: VlackuOptions::default(),
        };
        let results = PagedResults::from_all(vec![card], PageNumber::first()).expect("page");
        let result = vlacku::render(
            &VlackuOutcome::Results {
                results,
                diagnostics: Vec::new(),
                valid_missing: false,
            },
            &request,
            None,
        );
        assert!(result.shows_excerpt, "the definition was cut to fit a card");
        let published = PublishedRequest {
            request: DiscordRequest::Vlacku(request),
            revision: Revision::INITIAL,
            initiator: Snowflake::parse("123456789012345678").expect("snowflake"),
            build_tag: BuildTag::current(),
        };
        let message = assemble(&result, &published, 25 << 20).expect("assembled");
        assert!(message.payload.text_units() < MESSAGE_TEXT_BUDGET_UNITS / 2);
        let attached =
            String::from_utf8(upload_bytes(&message, RESULT_ATTACHMENT_FILENAME)).expect("utf-8");
        assert!(
            attached.contains(&definition),
            "the whole definition is attached"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagnostics_beyond_the_shown_few_travel_complete() {
        let source = "'a 'b 'c 'd 'e 'zukte";
        let analysis = jbotci_web_core::analyze_vlasei(source, None, None).expect("analysis");
        assert!(
            analysis.diagnostics.len() > 4,
            "{} diagnostics",
            analysis.diagnostics.len()
        );
        let mut result = rendered(vec!["**Recovered.**".to_owned()]);
        result.set_diagnostics(render_diagnostics(source, &analysis.diagnostics));
        assert!(
            result.shows_excerpt,
            "more diagnostics than the message shows"
        );
        let message = assemble(&result, &published(source), 25 << 20).expect("assembled");
        let shown = text_displays(&message).join("\n");
        assert!(shown.contains("more diagnostic"), "{shown}");
        let attached =
            String::from_utf8(upload_bytes(&message, RESULT_ATTACHMENT_FILENAME)).expect("utf-8");
        for diagnostic in &analysis.diagnostics {
            assert!(
                attached.contains(&diagnostic.message),
                "{}",
                diagnostic.message
            );
            assert!(attached.contains(&diagnostic.code), "{}", diagnostic.code);
            for note in &diagnostic.notes {
                assert!(attached.contains(note.as_str()), "a note is kept: {note}");
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cut_markdown_never_exceeds_tiny_budgets_and_keeps_fences_balanced() {
        let samples = [
            "```\nabc\n```",
            "text ``` more ``` and ``",
            "a\\*b\\*c\\",
            "``inline`` `x`",
            "…………………",
            "```\n```\n```\nopen",
        ];
        for sample in samples {
            for max_units in 0..=16 {
                let cut = cut_markdown(sample, max_units);
                assert!(
                    utf16_len(&cut) <= max_units,
                    "{sample:?} @ {max_units}: {cut:?}"
                );
                assert_eq!(
                    cut.matches("```").count() % 2,
                    0,
                    "{sample:?} @ {max_units}: {cut:?}"
                );
                assert!(cut == sample || !cut.ends_with('\\'), "{cut:?}");
            }
        }
        assert_eq!(cut_markdown("plain", 10), "plain");
        assert_eq!(cut_markdown("abcdef", 3), "ab…");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pack_chunks_keeps_whole_chunks_in_order() {
        let chunks = vec!["one".to_owned(), "two".to_owned(), "three".to_owned()];
        assert_eq!(pack_chunks(&chunks, 8), "one\n\ntwo");
        assert_eq!(pack_chunks(&chunks, 100), "one\n\ntwo\n\nthree");
        assert_eq!(pack_chunks(&chunks, 2), "o…");
        assert_eq!(pack_chunks(&chunks, 0), "");
    }
}
