//! Vlacku presentation: compact dictionary cards that keep provenance and
//! the attested-versus-synthesized distinction.

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_output::{
    DefinitionPlaceMap, GlyphStyle, format_definition_line_with_indexed_places,
    indexed_place_spans_for_notes_line,
};
use jbotci_search::vlacku::{VlackuCard, VlackuCompositionKind, format_vote_display};

use super::markdown::{escape, inline_code, join_lines, percent, subtext, truncate_units};
use super::{RenderedResult, page_status};
use crate::discord::operations::VlackuOutcome;
use crate::discord::request::{DiscordTool, VlackuMode, VlackuRequest};

/// Longest definition or notes text kept per card in the message.
const MAX_CARD_FIELD_UNITS: usize = 420;

#[requires(true)]
#[ensures(ret.tool() == DiscordTool::Vlacku)]
pub(crate) fn render(
    outcome: &VlackuOutcome,
    request: &VlackuRequest,
    app_link: Option<&str>,
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
            let mut rendered = RenderedResult::new(
                DiscordTool::Vlacku,
                format!("vlacku · {mode_label} · unavailable"),
            );
            rendered.body.push(format!(
                "**Meaning search is unavailable on this server:** {}",
                escape(reason)
            ));
            rendered.notice = Some(subtext(
                "Word, rafsi, lujvo and sound search still work; change the mode with the ⚙️ button.",
            ));
            rendered
        }
        VlackuOutcome::Results {
            results,
            diagnostics,
            valid_missing,
        } => {
            let mut status = format!("vlacku · {mode_label} · {}", page_status(results));
            if !options.word_types.is_empty() {
                let filters = options
                    .word_types
                    .iter()
                    .map(|word_type| word_type.filter_value())
                    .collect::<Vec<_>>();
                status.push_str(&format!(" · {}", filters.join("/")));
            }
            let mut rendered = RenderedResult::new(DiscordTool::Vlacku, status);
            let mut lines = Vec::new();
            for diagnostic in diagnostics {
                lines.push(format!("**Search problem:** {}", escape(diagnostic)));
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
                let card =
                    card_markdown(rank, card, options.decompose_lujvo, options.show_etymology)
                        .into_data();
                shows_excerpt |= card.is_excerpt;
                lines.push(card.markdown);
                full.push(card.plain);
            }
            rendered.body.push(join_lines(lines));
            if !full.is_empty() {
                let full = full.join("\n\n");
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
#[ensures(!ret.markdown.is_empty())]
fn card_markdown(
    rank: usize,
    card: &VlackuCard,
    decompose_lujvo: bool,
    show_etymology: bool,
) -> RenderedCard {
    let mut head = format!("{rank}. **{}**", escape(&card.word));
    let mut plain_head = format!("{rank}. {}", card.word);
    let class = match &card.selmaho {
        Some(selmaho) if !selmaho.trim().is_empty() => format!("{} {}", card.word_type, selmaho),
        _ => card.word_type.clone(),
    };
    head.push_str(&format!(" · {}", escape(&class)));
    plain_head.push_str(&format!(" · {class}"));
    if !card.rafsi.is_empty() {
        let rafsi = card
            .rafsi
            .iter()
            .map(|rafsi| inline_code(rafsi))
            .collect::<Vec<_>>()
            .join(" ");
        head.push_str(&format!(" · rafsi {rafsi}"));
        plain_head.push_str(&format!(" · rafsi {}", card.rafsi.join(" ")));
    }
    if let Some(similarity) = card.similarity {
        head.push_str(&format!(" · {}", percent(similarity)));
        plain_head.push_str(&format!(" · {}", percent(similarity)));
    }
    let mut lines = vec![head];
    let mut plain = vec![plain_head];
    let mut is_excerpt = false;
    if !card.known {
        let line = "not in the dictionary; the class above is the morphology parser's analysis of the word form, not a definition";
        lines.push(subtext(line));
        plain.push(line.to_owned());
    }
    if !card.glosses.is_empty() {
        let glosses = card.glosses.join("; ");
        lines.push(escape(&glosses));
        plain.push(glosses);
    }
    let place_map = DefinitionPlaceMap::from_definition(&card.definition);
    if !card.definition.trim().is_empty() {
        let definition = card
            .definition
            .lines()
            .map(|line| {
                format_definition_line_with_indexed_places(line, &place_map, GlyphStyle::Unicode)
            })
            .collect::<Vec<_>>()
            .join("\n");
        let (shown, cut) = truncate_units(&definition, MAX_CARD_FIELD_UNITS);
        is_excerpt |= cut;
        lines.push(escape(&shown));
        plain.push(definition);
    }
    if !card.notes.trim().is_empty() {
        let notes = card
            .notes
            .lines()
            .map(|line| {
                indexed_place_spans_for_notes_line(line, &place_map, GlyphStyle::Unicode)
                    .into_iter()
                    .map(|span| span.into_data().text)
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        let (shown, cut) = truncate_units(&notes, MAX_CARD_FIELD_UNITS);
        is_excerpt |= cut;
        lines.push(subtext(&format!("notes: {}", shown.replace('\n', " "))));
        plain.push(format!("notes: {notes}"));
    }
    if decompose_lujvo && !card.decomposition.is_empty() {
        let pieces = card
            .decomposition
            .iter()
            .map(|piece| match (piece.kind, &piece.source) {
                (VlackuCompositionKind::Hyphen, _) => format!("-{}-", piece.surface),
                (VlackuCompositionKind::Rafsi, Some(source)) if source != &piece.surface => {
                    format!("{} ({source})", piece.surface)
                }
                (VlackuCompositionKind::Rafsi, _) => piece.surface.clone(),
            })
            .collect::<Vec<_>>()
            .join(" + ");
        lines.push(subtext(&format!("decomposition: {pieces}")));
        plain.push(format!("decomposition: {pieces}"));
    }
    if show_etymology
        && let Some(etymology) = card
            .etymology
            .as_deref()
            .filter(|text| !text.trim().is_empty())
    {
        let (shown, cut) = truncate_units(etymology, MAX_CARD_FIELD_UNITS);
        is_excerpt |= cut;
        lines.push(subtext(&format!("etymology: {}", shown.replace('\n', " "))));
        plain.push(format!("etymology: {etymology}"));
    }
    let mut provenance = Vec::new();
    if let Some(author) = &card.author {
        provenance.push(format!("by {}", author.username));
    }
    if let Some(votes) = card.votes {
        provenance.push(format!(
            "votes {}",
            format_vote_display(votes, card.is_official)
        ));
    } else if card.is_official {
        provenance.push("official".to_owned());
    }
    if !provenance.is_empty() {
        let line = provenance.join(" · ");
        lines.push(subtext(&line));
        plain.push(line);
    }
    new!(RenderedCard {
        markdown: join_lines(lines),
        plain: plain.join("\n"),
        is_excerpt,
    })
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
            None,
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
            None,
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
            None,
        );
        assert!(rendered.status.text.ends_with("unavailable"));
        assert!(rendered.body[0].contains("no embedding index"));
    }
}
