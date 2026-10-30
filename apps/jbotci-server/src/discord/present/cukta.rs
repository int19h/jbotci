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
    cll_section_chapter_title, render_example_with_limit, render_section_with_limit,
    search_chunk_kind_label,
};

use super::markdown::{
    escape_in_scope, inline_code_in_scope, join_lines_in_scope, percent_in_scope,
    split_paragraphs_in_scope, subtext_in_scope, truncate_units_in_scope,
};
use super::{Pagination, RenderedResult, page_status};
use crate::discord::operations::{CuktaOutcome, CuktaSearchCard};
use crate::discord::request::{CuktaMode, CuktaRequest, CuktaResultKind, DiscordTool};

/// Longest search-hit preview kept in the message.
const MAX_PREVIEW_UNITS: usize = 320;

#[requires(true)]
#[ensures(text_scope.is_exceeded() || (ret.tool() == DiscordTool::Cukta))]
pub(crate) fn render_in_scope(
    outcome: &CuktaOutcome,
    request: &CuktaRequest,
    text_scope: &jbotci_output::budget::TextScope,
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
            let mut rendered = RenderedResult::new_in_scope(
                DiscordTool::Cukta,
                text_scope.format(format_args!("cukta · {mode_label} · unavailable")),
                text_scope,
            );
            rendered.body.push(text_scope.format(format_args!(
                "**Meaning search is unavailable on this server:** {}",
                escape_in_scope(reason, text_scope)
            )));
            rendered.notice = Some(subtext_in_scope(
                "Word search, sections, examples and the contents still work; change the mode with the ⚙️ button.",
                text_scope,
            ));
            rendered
        }
        CuktaOutcome::NotFound { what, reference } => {
            let mut rendered = RenderedResult::new_in_scope(
                DiscordTool::Cukta,
                text_scope.format(format_args!("cukta · {mode_label} · not found")),
                text_scope,
            );
            rendered.body.push(text_scope.format(format_args!(
                "**No {what} {}.** References look like `5.2` for sections and `6.8` for examples.",
                inline_code_in_scope(reference, text_scope)
            )));
            rendered
        }
        CuktaOutcome::Search { results, message } => {
            let mut status = text_scope.buffer_with(&text_scope.format(format_args!(
                "cukta · {mode_label} · {}",
                page_status(results)
            )));
            if !request.options.kinds.is_empty() {
                let kinds = CuktaResultKind::ALL
                    .iter()
                    .filter(|kind| request.options.kinds.contains(**kind))
                    .map(|kind| kind.value())
                    .collect::<Vec<_>>();
                status.push_str(
                    &text_scope.format(format_args!(" · {}", text_scope.join(&kinds, "/"))),
                );
            }
            let mut rendered = RenderedResult::new_in_scope(DiscordTool::Cukta, status, text_scope);
            rendered.pagination = Some(Pagination::of(results));
            let mut lines = Vec::new();
            if let Some(message) = message {
                lines.push(escape_in_scope(message, text_scope));
            }
            if results.is_empty() {
                lines.push("**No matches found.**".to_owned());
            }
            let mut full = Vec::new();
            let mut shows_excerpt = false;
            for card in &results.items {
                let (markdown, plain, cut) = search_card_in_scope(card, text_scope);
                shows_excerpt |= cut;
                lines.push(markdown);
                full.push(plain);
            }
            rendered.body.push(join_lines_in_scope(lines, text_scope));
            if !full.is_empty() {
                let full = text_scope.join(&full, "\n\n");
                if shows_excerpt {
                    rendered.show_excerpt_of(full);
                } else {
                    rendered.set_full_text(full);
                }
            }
            rendered
        }
        CuktaOutcome::Section { site, section } => {
            let mut rendered = RenderedResult::new_in_scope(
                DiscordTool::Cukta,
                "cukta · section".to_owned(),
                text_scope,
            );
            let mut chunks = split_paragraphs_in_scope(
                &super::render_cll_in_scope(
                    render_section_with_limit(
                        site,
                        section,
                        CllRenderFormat::DiscordMarkdown,
                        CllLinkRenderMode::Plain,
                        text_scope.limit(),
                    ),
                    text_scope,
                ),
                text_scope,
            );
            if let Some(chapter) = cll_section_chapter_title(site, &section.section_id)
                && let Some(heading) = chunks.first_mut()
            {
                heading.push('\n');
                heading.push_str(&subtext_in_scope(&chapter, text_scope));
            }
            rendered.body = chunks;
            rendered.set_full_text(super::render_cll_in_scope(
                render_section_with_limit(
                    site,
                    section,
                    CllRenderFormat::Markdown,
                    CllLinkRenderMode::Plain,
                    text_scope.limit(),
                ),
                text_scope,
            ));
            rendered
        }
        CuktaOutcome::Example { site, example } => {
            let mut rendered = RenderedResult::new_in_scope(
                DiscordTool::Cukta,
                "cukta · example".to_owned(),
                text_scope,
            );
            rendered.body = split_paragraphs_in_scope(
                &super::render_cll_in_scope(
                    render_example_with_limit(
                        site,
                        example,
                        CllRenderFormat::DiscordMarkdown,
                        CllLinkRenderMode::Plain,
                        text_scope.limit(),
                    ),
                    text_scope,
                ),
                text_scope,
            );
            rendered.set_full_text(super::render_cll_in_scope(
                render_example_with_limit(
                    site,
                    example,
                    CllRenderFormat::Markdown,
                    CllLinkRenderMode::Plain,
                    text_scope.limit(),
                ),
                text_scope,
            ));
            rendered
        }
        CuktaOutcome::Contents { edition, chapters } => {
            let mut rendered = RenderedResult::new_in_scope(
                DiscordTool::Cukta,
                "cukta · contents".to_owned(),
                text_scope,
            );
            let mut lines = vec![
                text_scope.format(format_args!("**{}**", escape_in_scope(edition, text_scope))),
            ];
            for chapter in chapters {
                let sections = text_scope.format(format_args!(
                    "{} section{}",
                    chapter.section_count,
                    if chapter.section_count == 1 { "" } else { "s" }
                ));
                lines.push(match chapter.number {
                    Some(number) => text_scope.format(format_args!(
                        "{number}. {} — {sections}",
                        escape_in_scope(&chapter.title, text_scope)
                    )),
                    None => text_scope.format(format_args!(
                        "- {} — {sections}",
                        escape_in_scope(&chapter.title, text_scope)
                    )),
                });
            }
            lines.push(subtext_in_scope(
                "Open a section with mode Section and a reference such as 5.2.",
                text_scope,
            ));
            rendered.body.push(join_lines_in_scope(lines, text_scope));
            rendered
        }
    }
}

/// One search hit as Discord Markdown, as plain text, and whether the shown
/// preview leaves anything out.
#[requires(true)]
#[ensures(text_scope.is_exceeded() || (!ret.0.is_empty()))]
fn search_card_in_scope(
    card: &CuktaSearchCard,
    text_scope: &jbotci_output::budget::TextScope,
) -> (String, String, bool) {
    let kind = search_chunk_kind_label(card.kind);
    let mut head = text_scope.buffer_with(&text_scope.format(format_args!(
        "{}. **{}**",
        card.rank,
        escape_in_scope(&card.label, text_scope)
    )));
    if card.kind != CllSearchChunkKind::Section {
        head.push_str(&text_scope.format(format_args!(
            " · {kind} in {}",
            escape_in_scope(&card.section_label, text_scope)
        )));
    }
    if let Some(similarity) = card.similarity {
        head.push_str(&text_scope.format(format_args!(
            " · {}",
            percent_in_scope(similarity, text_scope)
        )));
    }
    let compact = text_scope.join(card.text.split_whitespace(), " ");
    let (preview, cut) = truncate_units_in_scope(&compact, MAX_PREVIEW_UNITS, text_scope);
    let body = if card
        .role
        .as_ref()
        .is_some_and(CllParagraphRole::is_status_note)
    {
        jbotci_cll::status_note_markdown_in_scope(
            &escape_in_scope(&preview, text_scope),
            text_scope,
        )
    } else {
        escape_in_scope(&preview, text_scope)
    };
    let plain = text_scope.format(format_args!(
        "{}. {} · {kind} in {}\n{}",
        card.rank, card.label, card.section_label, card.text
    ));
    (
        join_lines_in_scope(vec![head.into_piece(), body], text_scope),
        plain,
        cut,
    )
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
pub(crate) fn render(outcome: &CuktaOutcome, request: &CuktaRequest) -> RenderedResult {
    let text_scope = jbotci_output::budget::TextScope::new(
        jbotci_output::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    let result = render_in_scope(outcome, request, &text_scope);
    text_scope
        .finish(String::new())
        .expect("unrestricted Discord rendering");
    result
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
fn search_card(card: &CuktaSearchCard) -> (String, String, bool) {
    let text_scope = jbotci_output::budget::TextScope::new(
        jbotci_output::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    let result = search_card_in_scope(card, &text_scope);
    text_scope
        .finish(String::new())
        .expect("unrestricted Discord rendering");
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord::operations::{CuktaChapterEntry, PagedResults, cukta_card_for_tests};
    use crate::discord::request::{CuktaOptions, CuktaResultKindSet, PageNumber, SourceText};
    use jbotci_cll::{
        CuktaSearchMode, CuktaSearchWindow, CuktaTargetFilter, cll_lookup_section,
        cll_resolve_section_reference, cukta_search, embedded_cll_site,
    };

    #[requires(true)]
    #[ensures(true)]
    fn request(mode: CuktaMode, query: Option<&str>) -> CuktaRequest {
        CuktaRequest {
            query: query.map(|text| SourceText::new(text).expect("text")),
            options: CuktaOptions {
                mode,
                kinds: CuktaResultKindSet::empty(),
                page: PageNumber::first(),
            },
        }
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
            CuktaSearchWindow::first(6),
            CuktaTargetFilter::default(),
        );
        let cards = output
            .matches
            .iter()
            .map(cukta_card_for_tests)
            .collect::<Vec<_>>();
        assert!(!cards.is_empty());
        let results = PagedResults::from_all(cards, PageNumber::first()).expect("page");
        let rendered = render(
            &CuktaOutcome::Search {
                results,
                message: None,
            },
            &request(CuktaMode::Word, Some("tanru")),
        );
        assert!(
            rendered
                .status
                .text
                .starts_with("cukta · word search · 1-5 of "),
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
        );
        assert!(
            missing.body[0].contains("No example `99.99`"),
            "{}",
            missing.body[0]
        );
    }
}
