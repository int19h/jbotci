//! Cukta presentation: CLL search cards, section and example excerpts and the
//! book's contents.
//!
//! Book content goes through the shared CLL renderer in its Discord dialect,
//! so which links survive a route-free rendering, how a rule-status note is
//! labelled and how example lines are typed are the same here as in the CLI,
//! the MCP tool and the web reader. The overflow attachment carries the same
//! content in the renderer's GitHub dialect, the workspace's plain-text form
//! of the book.

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_cll::{
    CllLinkRenderMode, CllParagraphRole, CllRenderFormat, CllSearchChunkKind,
    cll_section_chapter_title, render_example, render_section, search_chunk_kind_label,
    status_note_markdown,
};

use super::markdown::{
    escape, inline_code, join_lines, percent, split_paragraphs, subtext, truncate_units,
};
use super::{RenderedResult, capped_notice, page_status};
use crate::discord::operations::{CuktaOutcome, CuktaSearchCard};
use crate::discord::request::{CuktaMode, CuktaRequest, CuktaResultKind, DiscordTool};

/// Longest search-hit preview kept in the message.
const MAX_PREVIEW_UNITS: usize = 320;

#[requires(true)]
#[ensures(ret.tool() == DiscordTool::Cukta)]
pub(crate) fn render(
    outcome: &CuktaOutcome,
    request: &CuktaRequest,
    app_link: Option<&str>,
) -> RenderedResult {
    let mode_label = match request.options.mode {
        CuktaMode::Meaning => "meaning search",
        CuktaMode::Word => "word search",
        CuktaMode::Section => "section",
        CuktaMode::Example => "example",
        CuktaMode::Contents => "contents",
    };
    match outcome {
        CuktaOutcome::Unavailable { reason } => {
            let mut rendered = RenderedResult::new(
                DiscordTool::Cukta,
                format!("cukta · {mode_label} · unavailable"),
            );
            rendered.body.push(format!(
                "**Meaning search is unavailable on this server:** {}",
                escape(reason)
            ));
            rendered.notice = Some(subtext(
                "Word search, sections, examples and the contents still work; change the mode with the ⚙️ button.",
            ));
            rendered
        }
        CuktaOutcome::NotFound { what, reference } => {
            let mut rendered = RenderedResult::new(
                DiscordTool::Cukta,
                format!("cukta · {mode_label} · not found"),
            );
            rendered.body.push(format!(
                "**No {what} {}.** References look like `5.2` for sections and `6.8` for examples.",
                inline_code(reference)
            ));
            rendered
        }
        CuktaOutcome::Search { results, message } => {
            let mut status = format!("cukta · {mode_label} · {}", page_status(results));
            if !request.options.kinds.is_empty() {
                let kinds = CuktaResultKind::ALL
                    .iter()
                    .filter(|kind| request.options.kinds.contains(**kind))
                    .map(|kind| kind.value())
                    .collect::<Vec<_>>();
                status.push_str(&format!(" · {}", kinds.join("/")));
            }
            let mut rendered = RenderedResult::new(DiscordTool::Cukta, status);
            let mut lines = Vec::new();
            if let Some(message) = message {
                lines.push(escape(message));
            }
            if results.is_empty() {
                lines.push("**No matches found.**".to_owned());
            }
            let mut full = Vec::new();
            for card in &results.items {
                let (markdown, plain) = search_card(card);
                lines.push(markdown);
                full.push(plain);
            }
            rendered.body.push(join_lines(lines));
            if !full.is_empty() {
                rendered.full_text = Some(full.join("\n\n"));
            }
            rendered.notice = capped_notice(results, app_link);
            rendered
        }
        CuktaOutcome::Section { site, section } => {
            let mut rendered =
                RenderedResult::new(DiscordTool::Cukta, "cukta · section".to_owned());
            let mut chunks = split_paragraphs(&render_section(
                site,
                section,
                CllRenderFormat::DiscordMarkdown,
                CllLinkRenderMode::Plain,
            ));
            if let Some(chapter) = cll_section_chapter_title(site, &section.section_id)
                && let Some(heading) = chunks.first_mut()
            {
                heading.push('\n');
                heading.push_str(&subtext(&chapter));
            }
            rendered.body = chunks;
            rendered.full_text = Some(render_section(
                site,
                section,
                CllRenderFormat::Markdown,
                CllLinkRenderMode::Plain,
            ));
            rendered
        }
        CuktaOutcome::Example { site, example } => {
            let mut rendered =
                RenderedResult::new(DiscordTool::Cukta, "cukta · example".to_owned());
            rendered.body = split_paragraphs(&render_example(
                site,
                example,
                CllRenderFormat::DiscordMarkdown,
                CllLinkRenderMode::Plain,
            ));
            rendered.full_text = Some(render_example(
                site,
                example,
                CllRenderFormat::Markdown,
                CllLinkRenderMode::Plain,
            ));
            rendered
        }
        CuktaOutcome::Contents { edition, chapters } => {
            let mut rendered =
                RenderedResult::new(DiscordTool::Cukta, "cukta · contents".to_owned());
            let mut lines = vec![format!("**{}**", escape(edition))];
            for chapter in chapters {
                let sections = format!(
                    "{} section{}",
                    chapter.section_count,
                    if chapter.section_count == 1 { "" } else { "s" }
                );
                lines.push(match chapter.number {
                    Some(number) => format!("{number}. {} — {sections}", escape(&chapter.title)),
                    None => format!("- {} — {sections}", escape(&chapter.title)),
                });
            }
            lines.push(subtext(
                "Open a section with mode Section and a reference such as 5.2.",
            ));
            rendered.body.push(join_lines(lines));
            rendered
        }
    }
}

/// One search hit as Discord Markdown and as plain text.
#[requires(true)]
#[ensures(!ret.0.is_empty())]
fn search_card(card: &CuktaSearchCard) -> (String, String) {
    let kind = search_chunk_kind_label(card.kind);
    let mut head = format!("{}. **{}**", card.rank, escape(&card.label));
    if card.kind != CllSearchChunkKind::Section {
        head.push_str(&format!(" · {kind} in {}", escape(&card.section_label)));
    }
    if let Some(similarity) = card.similarity {
        head.push_str(&format!(" · {}", percent(similarity)));
    }
    let compact = card.text.split_whitespace().collect::<Vec<_>>().join(" ");
    let (preview, cut) = truncate_units(&compact, MAX_PREVIEW_UNITS);
    let body = if card
        .role
        .as_ref()
        .is_some_and(CllParagraphRole::is_status_note)
    {
        status_note_markdown(&escape(&preview))
    } else {
        escape(&preview)
    };
    let mut lines = vec![head, body];
    if cut {
        lines.push(subtext("full text in the attachment"));
    }
    let plain = format!(
        "{}. {} · {kind} in {}\n{}",
        card.rank, card.label, card.section_label, card.text
    );
    (join_lines(lines), plain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord::operations::{CuktaChapterEntry, PagedResults, cukta_card_for_tests};
    use crate::discord::request::{
        CuktaOptions, CuktaResultKindSet, MAX_PAGE, PageNumber, SourceText,
    };
    use jbotci_cll::{
        CuktaSearchMode, CuktaTargetFilter, cll_lookup_section, cll_resolve_section_reference,
        cukta_search, embedded_cll_site,
    };

    #[requires(true)]
    #[ensures(true)]
    fn request(mode: CuktaMode, query: Option<&str>) -> CuktaRequest {
        new!(CuktaRequest {
            query: query.map(|text| SourceText::new(text).expect("text")),
            options: CuktaOptions {
                mode,
                kinds: CuktaResultKindSet::empty(),
                page: PageNumber::first(),
            },
        })
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn section_excerpt_uses_the_shared_discord_dialect() {
        let site = embedded_cll_site().expect("site");
        let section_id = cll_resolve_section_reference(site, "4.6").expect("4.6 resolves");
        let section = cll_lookup_section(site, &section_id).expect("section");
        let rendered = render(
            &CuktaOutcome::Section { site, section },
            &request(CuktaMode::Section, Some("4.6")),
            None,
        );
        assert_eq!(rendered.status.text, "cukta · section");
        assert!(
            rendered.body[0].starts_with("## 4\\.6. rafsi"),
            "{}",
            rendered.body[0]
        );
        assert!(
            rendered.body[0].contains("\n-# "),
            "chapter subtext: {}",
            rendered.body[0]
        );
        let body = rendered.body.join("\n\n");
        assert!(body.contains("**Example 4.27**"), "{body}");
        assert!(body.contains("- **jbo**: mamtypatfu"), "{body}");
        assert!(body.contains("> **Rule status.**"), "{body}");
        assert!(
            body.contains("```\nCVC | 123 | -sak-"),
            "tables become code blocks: {body}"
        );
        assert!(
            body.contains("\\(see "),
            "prose is escaped for Discord: {body}"
        );
        assert!(!body.contains("](/"), "no web routes leak: {body}");
        assert!(!body.contains("| --- |"), "no pipe tables: {body}");
        // Chunk boundaries never split a fence.
        for chunk in &rendered.body {
            assert_eq!(chunk.matches("```").count() % 2, 0, "{chunk}");
        }
        let full = rendered.full_text.as_deref().expect("attachment text");
        assert!(full.starts_with("# 4.6"), "{full}");
        assert!(
            full.contains("| --- |"),
            "attachment keeps the GitHub dialect: {full}"
        );
        assert!(!full.contains("\\("), "{full}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn search_cards_show_reference_kind_and_preview() {
        let site = embedded_cll_site().expect("site");
        let output = cukta_search(
            site,
            CuktaSearchMode::Word,
            "tanru",
            6,
            CuktaTargetFilter::default(),
        );
        let cards = output
            .matches
            .iter()
            .map(cukta_card_for_tests)
            .collect::<Vec<_>>();
        assert!(!cards.is_empty());
        let results = PagedResults::paginate(cards, PageNumber::first(), MAX_PAGE).expect("page");
        let rendered = render(
            &CuktaOutcome::Search {
                results,
                message: None,
            },
            &request(CuktaMode::Word, Some("tanru")),
            Some("https://jbotci.app/cukta?q=tanru"),
        );
        assert!(
            rendered
                .status
                .text
                .starts_with("cukta · word search · page 1/"),
            "{}",
            rendered.status.text
        );
        let body = rendered.body.join("\n");
        assert!(body.starts_with("1. **"), "{body}");
        assert!(rendered.full_text.is_some());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn contents_and_not_found_are_distinct_outcomes() {
        let rendered = render(
            &CuktaOutcome::Contents {
                edition: "CLL test".to_owned(),
                chapters: vec![new!(CuktaChapterEntry {
                    number: Some(1),
                    title: "Lojban As We Mangle It in Lojbanistan".to_owned(),
                    section_count: 4,
                })],
            },
            &request(CuktaMode::Contents, None),
            None,
        );
        assert!(
            rendered.body[0].contains("1. Lojban As We Mangle It"),
            "{}",
            rendered.body[0]
        );
        let missing = render(
            &CuktaOutcome::NotFound {
                what: "example",
                reference: "99.99".to_owned(),
            },
            &request(CuktaMode::Example, Some("99.99")),
            None,
        );
        assert!(
            missing.body[0].contains("No example `99.99`"),
            "{}",
            missing.body[0]
        );
    }
}
