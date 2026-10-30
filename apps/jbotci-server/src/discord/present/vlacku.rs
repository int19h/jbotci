//! Vlacku presentation: compact dictionary cards that keep provenance and
//! the attested-versus-synthesized distinction.

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_output::{
    DefinitionPlaceMap, GlyphStyle, format_definition_line_with_indexed_places,
    indexed_place_spans_for_notes_line,
};
use jbotci_search::vlacku::{VlackuCard, VlackuCompositionKind, format_vote_display};

use super::markdown::{
    escape_in_scope, inline_code_in_scope, join_lines_in_scope, percent_in_scope, subtext_in_scope,
    truncate_units_in_scope,
};
use super::{Pagination, RenderedResult, page_status};
use crate::discord::operations::VlackuOutcome;
use crate::discord::request::{DiscordTool, VlackuMode, VlackuRequest};

/// Longest definition or notes text kept per card in the message.
const MAX_CARD_FIELD_UNITS: usize = 420;

#[requires(true)]
#[ensures(text_scope.is_exceeded() || (ret.tool() == DiscordTool::Vlacku))]
pub(crate) fn render_in_scope(
    outcome: &VlackuOutcome,
    request: &VlackuRequest,
    text_scope: &jbotci_output::budget::TextScope,
) -> RenderedResult {
    let options = request.options;
    let mode_label = match options.mode {
        VlackuMode::Word => "word",
        VlackuMode::Rafsi => "rafsi",
        VlackuMode::Lujvo => "lujvo",
        VlackuMode::Sound => "sound",
        VlackuMode::Meaning => "meaning",
    };
    match outcome {
        VlackuOutcome::Unavailable { reason } => {
            let mut rendered = RenderedResult::new_in_scope(
                DiscordTool::Vlacku,
                text_scope.format(format_args!("vlacku · {mode_label} · unavailable")),
                text_scope,
            );
            rendered.body.push(text_scope.format(format_args!(
                "**Meaning search is unavailable on this server:** {}",
                escape_in_scope(reason, text_scope)
            )));
            rendered.notice = Some(subtext_in_scope(
                "Word, rafsi, lujvo and sound search still work; change the mode with the ⚙️ button.",
                text_scope,
            ));
            rendered
        }
        VlackuOutcome::Results {
            results,
            diagnostics,
            valid_missing,
        } => {
            let mut status = text_scope.buffer_with(&text_scope.format(format_args!(
                "vlacku · {mode_label} · {}",
                page_status(results)
            )));
            if !options.word_types.is_empty() {
                let filters = options
                    .word_types
                    .iter()
                    .map(|word_type| word_type.filter_value())
                    .collect::<Vec<_>>();
                status.push_str(
                    &text_scope.format(format_args!(" · {}", text_scope.join(&filters, "/"))),
                );
            }
            let mut rendered =
                RenderedResult::new_in_scope(DiscordTool::Vlacku, status, text_scope);
            rendered.pagination = Some(Pagination::of(results));
            let mut lines = Vec::new();
            for diagnostic in diagnostics {
                lines.push(text_scope.format(format_args!(
                    "**Search problem:** {}",
                    escape_in_scope(diagnostic, text_scope)
                )));
            }
            if results.is_empty() && diagnostics.is_empty() {
                lines.push("**No matches found.**".to_owned());
            } else if *valid_missing {
                lines.push("**No dictionary entry for this word.**".to_owned());
            }
            let mut full = Vec::new();
            let mut shows_excerpt = false;
            for (offset, card) in results.items.iter().enumerate() {
                let rank = results.page.first_index() + offset + 1;
                let Some(card) = card_markdown(
                    rank,
                    card,
                    options.decompose_lujvo,
                    options.show_etymology,
                    text_scope,
                ) else {
                    break;
                };
                let card = card.into_data();
                shows_excerpt |= card.is_excerpt;
                lines.push(card.markdown);
                full.push(card.plain);
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
    }
}

/// One card as the message shows it and as it reads in full.
#[invariant(!markdown.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct RenderedCard {
    markdown: String,
    plain: String,
    /// Whether the shown card leaves anything out of the plain text.
    is_excerpt: bool,
}

/// One card as Markdown and as plain text.
#[requires(rank >= 1)]
#[ensures(ret.is_none() || !ret.as_ref().unwrap().markdown.is_empty())]
fn card_markdown(
    rank: usize,
    card: &VlackuCard,
    decompose_lujvo: bool,
    show_etymology: bool,
    text_scope: &jbotci_output::budget::TextScope,
) -> Option<RenderedCard> {
    let mut head = text_scope.buffer_with(&text_scope.format(format_args!(
        "{rank}. **{}**",
        escape_in_scope(&card.word, text_scope)
    )));
    let mut plain_head =
        text_scope.buffer_with(&text_scope.format(format_args!("{rank}. {}", card.word)));
    let class = match &card.selmaho {
        Some(selmaho) if !selmaho.trim().is_empty() => {
            text_scope.format(format_args!("{} {}", card.word_type, selmaho))
        }
        _ => text_scope.copy(&card.word_type),
    };
    head.push_str(&text_scope.format(format_args!(" · {}", escape_in_scope(&class, text_scope))));
    plain_head.push_str(&text_scope.format(format_args!(" · {class}")));
    if !card.rafsi.is_empty() {
        let rafsi = text_scope.join(
            card.rafsi
                .iter()
                .map(|rafsi| inline_code_in_scope(rafsi, text_scope)),
            " ",
        );
        head.push_str(&text_scope.format(format_args!(" · rafsi {rafsi}")));
        plain_head.push_str(&text_scope.format(format_args!(
            " · rafsi {}",
            text_scope.join(&card.rafsi, " ")
        )));
    }
    if let Some(similarity) = card.similarity {
        head.push_str(&text_scope.format(format_args!(
            " · {}",
            percent_in_scope(similarity, text_scope)
        )));
        plain_head.push_str(&text_scope.format(format_args!(
            " · {}",
            percent_in_scope(similarity, text_scope)
        )));
    }
    let mut lines = vec![head.into_piece()];
    let mut plain = vec![plain_head.into_piece()];
    let mut is_excerpt = false;
    if !card.known {
        let line = "not in the dictionary; the class above is the morphology parser's analysis of the word form, not a definition";
        lines.push(subtext_in_scope(line, text_scope));
        plain.push(line.to_owned());
    }
    if !card.glosses.is_empty() {
        let glosses = text_scope.join(&card.glosses, "; ");
        lines.push(escape_in_scope(&glosses, text_scope));
        plain.push(glosses);
    }
    let place_map = DefinitionPlaceMap::from_definition(&card.definition);
    if !card.definition.trim().is_empty() {
        let definition = text_scope.join(
            card.definition.lines().map(|line| {
                jbotci_output::format_definition_line_with_indexed_places_in_scope(
                    line,
                    &place_map,
                    GlyphStyle::Unicode,
                    text_scope,
                )
            }),
            "\n",
        );
        let (shown, cut) = truncate_units_in_scope(&definition, MAX_CARD_FIELD_UNITS, text_scope);
        is_excerpt |= cut;
        lines.push(escape_in_scope(&shown, text_scope));
        plain.push(definition);
    }
    if !card.notes.trim().is_empty() {
        let notes = text_scope.join(
            card.notes.lines().map(|line| {
                jbotci_output::format_notes_line_with_indexed_places_in_scope(
                    line,
                    &place_map,
                    GlyphStyle::Unicode,
                    text_scope,
                )
            }),
            "\n",
        );
        let (shown, cut) = truncate_units_in_scope(&notes, MAX_CARD_FIELD_UNITS, text_scope);
        is_excerpt |= cut;
        lines.push(subtext_in_scope(
            &text_scope.format(format_args!("notes: {}", shown.replace('\n', " "))),
            text_scope,
        ));
        plain.push(text_scope.format(format_args!("notes: {notes}")));
    }
    if decompose_lujvo && !card.decomposition.is_empty() {
        let pieces = text_scope.join(
            card.decomposition
                .iter()
                .map(|piece| match (piece.kind, &piece.source) {
                    (VlackuCompositionKind::Hyphen, _) => {
                        text_scope.format(format_args!("-{}-", piece.surface))
                    }
                    (VlackuCompositionKind::Rafsi, Some(source)) if source != &piece.surface => {
                        text_scope.format(format_args!("{} ({source})", piece.surface))
                    }
                    (VlackuCompositionKind::Rafsi, _) => text_scope.copy(&piece.surface),
                }),
            " + ",
        );
        lines.push(subtext_in_scope(
            &text_scope.format(format_args!("decomposition: {pieces}")),
            text_scope,
        ));
        plain.push(text_scope.format(format_args!("decomposition: {pieces}")));
    }
    if show_etymology
        && let Some(etymology) = card
            .etymology
            .as_deref()
            .filter(|text| !text.trim().is_empty())
    {
        let (shown, cut) = truncate_units_in_scope(etymology, MAX_CARD_FIELD_UNITS, text_scope);
        is_excerpt |= cut;
        lines.push(subtext_in_scope(
            &text_scope.format(format_args!("etymology: {}", shown.replace('\n', " "))),
            text_scope,
        ));
        plain.push(text_scope.format(format_args!("etymology: {etymology}")));
    }
    let mut provenance = Vec::new();
    if let Some(author) = &card.author {
        provenance.push(text_scope.format(format_args!("by {}", author.username)));
    }
    if let Some(votes) = card.votes {
        provenance.push(text_scope.format(format_args!(
            "votes {}",
            format_vote_display(votes, card.is_official)
        )));
    } else if card.is_official {
        provenance.push("official".to_owned());
    }
    if !provenance.is_empty() {
        let line = text_scope.join(&provenance, " · ");
        lines.push(subtext_in_scope(&line, text_scope));
        plain.push(line);
    }
    let markdown = join_lines_in_scope(lines, text_scope);
    let plain = text_scope.join(&plain, "\n");
    if text_scope.is_exceeded() {
        return None;
    }
    Some(new!(RenderedCard {
        markdown,
        plain,
        is_excerpt
    }))
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
pub(crate) fn render(outcome: &VlackuOutcome, request: &VlackuRequest) -> RenderedResult {
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
mod tests {
    use super::*;
    use crate::discord::operations::PagedResults;
    use crate::discord::request::{PageNumber, SourceText, VlackuOptions};
    use jbotci_search::vlacku::{
        VlackuRequest as SearchRequest, VlackuSearchOptions, run_vlacku_requests,
    };

    #[requires(!query.is_empty())]
    #[ensures(true)]
    fn cards(query: &str) -> Vec<VlackuCard> {
        run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[SearchRequest::valsi(query.to_owned())],
            &VlackuSearchOptions::default().with_data(data! { count: 10, decompose_lujvo: true }),
        )
        .cards
    }

    #[requires(true)]
    #[ensures(true)]
    fn request(query: &str) -> VlackuRequest {
        VlackuRequest {
            query: SourceText::new(query).expect("text"),
            options: VlackuOptions::default(),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn attested_cards_show_class_rafsi_definition_and_provenance() {
        let cards = cards("klama");
        let results = PagedResults::from_all(cards, PageNumber::first()).expect("page");
        let rendered = render(
            &VlackuOutcome::Results {
                results,
                diagnostics: Vec::new(),
                valid_missing: false,
            },
            &request("klama"),
        );
        let body = rendered.body.join("\n");
        assert!(
            body.contains("1. **klama** · gismu · rafsi `kla`"),
            "{body}"
        );
        assert!(
            !body.contains("$x_1$"),
            "LaTeX place markers are rendered: {body}"
        );
        assert!(body.contains("⟨1⟩ comes/goes to destination ⟨2⟩"), "{body}");
        assert!(
            body.contains("-# by ") || body.contains("official"),
            "provenance: {body}"
        );
        assert!(!body.contains("not in the dictionary"), "{body}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn synthesized_cards_are_labelled_not_defined() {
        let cards = cards("klabajra");
        assert!(
            cards
                .first()
                .is_some_and(|card| card.word == "klabajra" && !card.known),
            "a valid lujvo absent from the dictionary yields a synthesized card first: {cards:?}"
        );
        let results = PagedResults::from_all(cards, PageNumber::first()).expect("page");
        let rendered = render(
            &VlackuOutcome::Results {
                results,
                diagnostics: Vec::new(),
                valid_missing: true,
            },
            &request("klabajra"),
        );
        let body = rendered.body.join("\n");
        assert!(body.contains("No dictionary entry"), "{body}");
        assert!(
            body.contains("1. **klabajra** · lujvo\n-# not in the dictionary; the class above"),
            "{body}"
        );
        assert!(
            body.contains("2. **klama** · gismu"),
            "component cards follow: {body}"
        );
        assert!(!body.contains("**Search problem"), "{body}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unavailable_meaning_search_is_reported_not_substituted() {
        let mut request = request("goer");
        request.options = VlackuOptions {
            mode: VlackuMode::Meaning,
            ..VlackuOptions::default()
        };
        let rendered = render(
            &VlackuOutcome::Unavailable {
                reason: "no embedding index".to_owned(),
            },
            &request,
        );
        assert!(rendered.status.text.ends_with("unavailable"));
        assert!(rendered.body[0].contains("no embedding index"));
    }
}
