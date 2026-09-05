#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_morphology::{LujvoPart, Word, WordKind, WordLike, WordLikeData, canonicalize_text};
use jbotci_output::{render_vlacku_cards_markdown, render_vlacku_headword_markdown};
use jbotci_search::compounds::{cmavo_sequence_containing, compound_cards};
use jbotci_search::vlacku::{
    VlackuCard, VlackuCompositionKind, VlackuCompositionPiece, VlackuRequest, VlackuSearchOptions,
    dictionary_entry_card, run_vlacku_requests, word_like_lookup_text,
};
use jbotci_source::SourceSpan;

use super::DocumentSnapshot;

/// Markdown hover documentation and its full half-open source span.
#[invariant(!markdown.is_empty())]
#[invariant(span.byte_start < span.byte_end && span.char_start < span.char_end)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverContent {
    pub markdown: String,
    pub span: SourceSpan,
}

#[invariant(word.span().byte_start < word.span().byte_end)]
#[invariant(word.span().char_start < word.span().char_end)]
#[derive(Debug, Clone, Copy)]
struct HoverWord<'snapshot> {
    word: &'snapshot Word,
    semantics: HoverSemantics,
}

#[invariant(!cards.is_empty())]
#[invariant(span.byte_start < span.byte_end && span.char_start < span.char_end)]
#[derive(Debug, Clone, PartialEq)]
struct CmavoSequenceDocumentation {
    cards: Vec<VlackuCard>,
    span: SourceSpan,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HoverSemantics {
    Dictionary,
    MorphologyOnly,
}

impl DocumentSnapshot {
    /// Build transport-neutral Markdown documentation for the word at `char_offset`.
    ///
    /// The returned source span remains in source coordinates. Callers choose their
    /// position encoding through [`crate::LineIndex`], as they do for diagnostics.
    /// For a cmavo in a dictionary-attested sequence, the longest containing
    /// sequence replaces the constituent card and supplies the hover span.
    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|hover| hover.span.char_start <= char_offset && char_offset < hover.span.char_end))]
    pub fn hover(&self, char_offset: usize) -> Option<HoverContent> {
        let word_at = self.word_at(char_offset)?;

        if matches!(word_at.word.as_data(), data!(WordLike::ZeiCompound { .. })) {
            return Some(new!(HoverContent {
                markdown: zei_compound_hover_markdown(word_at.word),
                span: word_at.span.clone(),
            }));
        }

        let target =
            hover_word_in_word_like(word_at.word, char_offset, HoverSemantics::Dictionary)?;
        if target.semantics == HoverSemantics::Dictionary
            && word_at
                .word
                .bare_word()
                .is_some_and(|word| std::ptr::eq(word, target.word))
            && let Some(sequence) = self.cmavo_sequence_documentation(word_at.index)
        {
            let sequence = sequence.into_data();
            return Some(new!(HoverContent {
                markdown: render_vlacku_cards_markdown(&sequence.cards),
                span: sequence.span,
            }));
        }

        let markdown = match target.semantics {
            HoverSemantics::Dictionary => dictionary_hover_markdown(target.word),
            HoverSemantics::MorphologyOnly => word_classification_markdown(target.word),
        };

        Some(new!(HoverContent {
            markdown,
            span: target.word.span().clone(),
        }))
    }

    #[requires(index < self.words.words.len())]
    #[ensures(ret.as_ref().is_none_or(|sequence| !sequence.cards.is_empty()))]
    #[ensures(ret.as_ref().is_none_or(|sequence| sequence.span.char_start <= self.word_spans[index].char_start && self.word_spans[index].char_end <= sequence.span.char_end))]
    fn cmavo_sequence_documentation(&self, index: usize) -> Option<CmavoSequenceDocumentation> {
        let dictionary = jbotci_dictionary_data::english();
        let compound = cmavo_sequence_containing(dictionary, &self.words.words, &self.text, index)?;
        let cards = compound_cards(dictionary, &compound)
            .into_iter()
            .map(cmavo_sequence_card_for_hover)
            .collect();
        Some(new!(CmavoSequenceDocumentation {
            cards,
            span: compound.into_data().span
        }))
    }

    #[requires(index < self.words.words.len())]
    #[ensures(ret.as_ref().is_none_or(|span| span.char_start <= self.word_spans[index].char_start && self.word_spans[index].char_end <= span.char_end))]
    pub(super) fn attested_cmavo_sequence_span(&self, index: usize) -> Option<SourceSpan> {
        self.cmavo_sequence_documentation(index)
            .map(|documentation| documentation.span.clone())
    }
}

#[requires(card.author.is_some())]
#[ensures(ret.word_type == "cmavo sequence")]
fn cmavo_sequence_card_for_hover(card: VlackuCard) -> VlackuCard {
    card.with_data(data! {
        word_type: "cmavo sequence".to_owned(),
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|target| target.word.span().char_start <= offset && offset < target.word.span().char_end))]
fn hover_word_in_word_like<'word>(
    word_like: &'word WordLike,
    offset: usize,
    semantics: HoverSemantics,
) -> Option<HoverWord<'word>> {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => hover_word_at(word, offset, semantics),
        data!(WordLike::QuotedWord { zo, word }) => hover_word_at(
            zo,
            offset,
            inherited_semantics(semantics, HoverSemantics::Dictionary),
        )
        .or_else(|| {
            hover_word_at(
                word,
                offset,
                inherited_semantics(semantics, HoverSemantics::Dictionary),
            )
        }),
        data!(WordLike::SelmahoQuotedWord { mahoi, word }) => hover_word_at(
            mahoi,
            offset,
            inherited_semantics(semantics, HoverSemantics::Dictionary),
        )
        .or_else(|| {
            hover_word_at(
                word,
                offset,
                inherited_semantics(semantics, HoverSemantics::Dictionary),
            )
        }),
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            closing_delimiter,
            ..
        }) => hover_word_at(
            zoi,
            offset,
            inherited_semantics(semantics, HoverSemantics::Dictionary),
        )
        .or_else(|| hover_word_at(opening_delimiter, offset, HoverSemantics::MorphologyOnly))
        .or_else(|| hover_word_at(closing_delimiter, offset, HoverSemantics::MorphologyOnly)),
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            lehu,
        }) => hover_word_at(
            lohu,
            offset,
            inherited_semantics(semantics, HoverSemantics::Dictionary),
        )
        .or_else(|| {
            quoted_words
                .iter()
                .find_map(|word| hover_word_at(word, offset, HoverSemantics::MorphologyOnly))
        })
        .or_else(|| {
            hover_word_at(
                lehu,
                offset,
                inherited_semantics(semantics, HoverSemantics::Dictionary),
            )
        }),
        data!(WordLike::DelimitedWordQuote { marker, .. }) => hover_word_at(
            marker,
            offset,
            inherited_semantics(semantics, HoverSemantics::Dictionary),
        ),
        data!(WordLike::LerfuWord { base, bu }) => {
            hover_word_in_word_like(base, offset, HoverSemantics::MorphologyOnly)
                .or_else(|| hover_word_at(bu, offset, HoverSemantics::MorphologyOnly))
        }
        data!(WordLike::ZeiCompound { left, zei, right }) => {
            hover_word_in_word_like(left, offset, HoverSemantics::MorphologyOnly)
                .or_else(|| hover_word_at(zei, offset, HoverSemantics::MorphologyOnly))
                .or_else(|| hover_word_at(right, offset, HoverSemantics::MorphologyOnly))
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_some() == (word.span().char_start <= offset && offset < word.span().char_end))]
fn hover_word_at(word: &Word, offset: usize, semantics: HoverSemantics) -> Option<HoverWord<'_>> {
    (word.span().char_start <= offset && offset < word.span().char_end)
        .then(|| new!(HoverWord { word, semantics }))
}

#[requires(true)]
#[ensures(matches!(inherited, HoverSemantics::MorphologyOnly) -> ret == HoverSemantics::MorphologyOnly)]
fn inherited_semantics(inherited: HoverSemantics, requested: HoverSemantics) -> HoverSemantics {
    match inherited {
        HoverSemantics::Dictionary => requested,
        HoverSemantics::MorphologyOnly => HoverSemantics::MorphologyOnly,
    }
}

#[requires(matches!(word_like.as_data(), data!(WordLike::ZeiCompound { .. })))]
#[ensures(!ret.is_empty())]
fn zei_compound_hover_markdown(word_like: &WordLike) -> String {
    let lookup_text = word_like_headword(word_like);
    let dictionary = jbotci_dictionary_data::english();
    let own_cards = dictionary
        .exact_definition_indices(&lookup_text)
        .map(|index| {
            dictionary_entry_card(
                dictionary,
                dictionary.entry_for_index(index).expect("exact entry"),
                None,
                false,
            )
        })
        .map(zei_compound_card_for_hover)
        .collect::<Vec<_>>();
    if !own_cards.is_empty() {
        return render_vlacku_cards_markdown(&own_cards);
    }

    let mut markdown = render_vlacku_headword_markdown(&lookup_text, "ZEI compound", None);
    let mut component_cards = Vec::new();
    append_zei_component_cards(word_like, &mut component_cards);
    if !component_cards.is_empty() {
        markdown.push_str("\n\n---\n\n");
        markdown.push_str(&render_vlacku_cards_markdown(&component_cards));
    }
    markdown
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn word_like_headword(word_like: &WordLike) -> String {
    if let Some(lookup_text) = word_like_lookup_text(word_like) {
        return lookup_text;
    }

    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => word.canonical_phonemes(),
        data!(WordLike::QuotedWord { zo, word }) => {
            format!("{} {}", zo.canonical_phonemes(), word.canonical_phonemes())
        }
        data!(WordLike::SelmahoQuotedWord { mahoi, word }) => format!(
            "{} {}",
            mahoi.canonical_phonemes(),
            word.canonical_phonemes()
        ),
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => format!(
            "{} {} {} {}",
            zoi.canonical_phonemes(),
            opening_delimiter.canonical_phonemes(),
            quoted_text.text,
            closing_delimiter.canonical_phonemes(),
        ),
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            lehu,
        }) => {
            let mut headword = lohu.canonical_phonemes();
            for word in quoted_words {
                headword.push(' ');
                headword.push_str(&word.canonical_phonemes());
            }
            headword.push(' ');
            headword.push_str(&lehu.canonical_phonemes());
            headword
        }
        data!(WordLike::DelimitedWordQuote {
            marker,
            quoted_text,
        }) => format!("{} {}", marker.canonical_phonemes(), quoted_text.text),
        data!(WordLike::LerfuWord { base, bu }) => {
            format!("{} {}", word_like_headword(base), bu.canonical_phonemes())
        }
        data!(WordLike::ZeiCompound { left, right, .. }) => format!(
            "{} zei {}",
            word_like_headword(left),
            right.canonical_phonemes()
        ),
    }
}

#[requires(card.author.is_some())]
#[ensures(ret.word_type == "ZEI compound")]
fn zei_compound_card_for_hover(card: VlackuCard) -> VlackuCard {
    card.with_data(data! {
        word_type: "ZEI compound".to_owned(),
    })
}

#[requires(true)]
#[ensures(cards.len() >= old(cards.len()))]
fn append_zei_component_cards(word_like: &WordLike, cards: &mut Vec<VlackuCard>) {
    match word_like.as_data() {
        data!(WordLike::ZeiCompound { left, right, .. }) => {
            append_zei_component_cards(left, cards);
            cards.extend(dictionary_cards_for_word(right));
        }
        _ => cards.extend(dictionary_cards_for_word_like_component(word_like)),
    }
}

#[requires(true)]
#[ensures(ret.iter().all(|card| card.author.is_some()))]
fn dictionary_cards_for_word_like_component(word_like: &WordLike) -> Vec<VlackuCard> {
    let Some(lookup_text) = word_like_lookup_text(word_like) else {
        return Vec::new();
    };
    let canonical_lookup = canonicalize_text(&lookup_text);
    let output = run_vlacku_requests(
        jbotci_dictionary_data::english(),
        &[VlackuRequest::valsi(lookup_text)],
        &hover_vlacku_options(false),
    );
    output
        .cards
        .into_iter()
        .filter(|card| card.author.is_some() && canonicalize_text(&card.word) == canonical_lookup)
        .collect()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn dictionary_hover_markdown(word: &Word) -> String {
    match word.kind() {
        WordKind::Cmevla => word_classification_markdown(word),
        WordKind::Lujvo => lujvo_hover_markdown(word),
        WordKind::Cmavo | WordKind::Gismu | WordKind::Fuhivla => {
            let cards = dictionary_cards_for_word(word);
            if cards.is_empty() {
                word_classification_markdown(word)
            } else {
                render_vlacku_cards_markdown(&cards)
            }
        }
    }
}

#[requires(word.kind() == WordKind::Lujvo)]
#[ensures(!ret.is_empty())]
fn lujvo_hover_markdown(word: &Word) -> String {
    let lookup_text = word.canonical_phonemes();
    let output = run_vlacku_requests(
        jbotci_dictionary_data::english(),
        &[VlackuRequest::lujvo(lookup_text.clone())],
        &hover_vlacku_options(true),
    );
    let decomposition = morphology_lujvo_decomposition(word, &output.cards);
    let own_cards = output
        .cards
        .iter()
        .filter(|card| card.author.is_some() && canonicalize_text(&card.word) == lookup_text)
        .cloned()
        .map(|card| with_decomposition(card, decomposition.clone()))
        .collect::<Vec<_>>();
    let cards = if !own_cards.is_empty() {
        own_cards
    } else {
        let mut cards = vec![
            output
                .cards
                .iter()
                .find(|card| canonicalize_text(&card.word) == lookup_text)
                .cloned()
                .map(|card| with_decomposition(card, decomposition))
                .expect("a morphology-valid lujvo search has a classification card"),
        ];
        cards.extend(
            output
                .cards
                .iter()
                .filter(|card| {
                    card.author.is_some() && canonicalize_text(&card.word) != lookup_text
                })
                .cloned()
                .map(without_decomposition),
        );
        cards
    };
    render_vlacku_cards_markdown(&cards)
}

#[requires(true)]
#[ensures(ret.decomposition.len() == old(decomposition.len()))]
fn with_decomposition(card: VlackuCard, decomposition: Vec<VlackuCompositionPiece>) -> VlackuCard {
    card.with_data(data! {
        decomposition: decomposition,
    })
}

#[requires(true)]
#[ensures(ret.decomposition.is_empty())]
fn without_decomposition(card: VlackuCard) -> VlackuCard {
    card.with_data(data! {
        decomposition: Vec::new(),
    })
}

#[requires(word.kind() == WordKind::Lujvo)]
#[ensures(!ret.is_empty())]
fn morphology_lujvo_decomposition(
    word: &Word,
    cards: &[VlackuCard],
) -> Vec<VlackuCompositionPiece> {
    let lookup_text = word.canonical_phonemes();
    let annotated = cards.iter().find(|card| {
        canonicalize_text(&card.word) == lookup_text && !card.decomposition.is_empty()
    });
    word.lujvo_parts()
        .expect("lujvo words carry parsed parts")
        .iter()
        .enumerate()
        .map(|(index, part)| {
            let kind = match part {
                LujvoPart::Rafsi(_) => VlackuCompositionKind::Rafsi,
                LujvoPart::Hyphen(_) => VlackuCompositionKind::Hyphen,
            };
            let surface = canonicalize_text(part.phonemes().as_str());
            let source = annotated
                .and_then(|card| card.decomposition.get(index))
                .filter(|piece| piece.kind == kind && canonicalize_text(&piece.surface) == surface)
                .and_then(|piece| piece.source.clone());
            VlackuCompositionPiece {
                kind,
                surface,
                source,
            }
        })
        .collect()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn word_classification_markdown(word: &Word) -> String {
    let word_type = match word.kind() {
        WordKind::Cmavo => "cmavo",
        WordKind::Gismu => "gismu",
        WordKind::Lujvo => "lujvo",
        WordKind::Fuhivla => "fu'ivla",
        WordKind::Cmevla => "name word (cmevla)",
    };
    render_vlacku_headword_markdown(&word.canonical_phonemes(), word_type, word.selmaho())
}

#[requires(true)]
#[ensures(ret.iter().all(|card| card.author.is_some()))]
fn dictionary_cards_for_word(word: &Word) -> Vec<VlackuCard> {
    let output = run_vlacku_requests(
        jbotci_dictionary_data::english(),
        &[VlackuRequest::valsi(word.canonical_phonemes())],
        &hover_vlacku_options(false),
    );
    output
        .cards
        .into_iter()
        .filter(|card| card.author.is_some())
        .collect()
}

#[requires(true)]
#[ensures(ret.count == usize::MAX)]
#[ensures(ret.decompose_lujvo == decompose_lujvo)]
fn hover_vlacku_options(decompose_lujvo: bool) -> VlackuSearchOptions {
    VlackuSearchOptions::default().with_data(data! {
        count: usize::MAX,
        decompose_lujvo: decompose_lujvo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cursor_prefers_longest_containing_sequence_and_can_cross_blocks_partition() {
        let snapshot = DocumentSnapshot::new("bi no no vo".to_owned(), 1);
        let selected = snapshot.cmavo_sequence_documentation(1).unwrap();
        assert_eq!(selected.span.byte_start, 0);
        assert_eq!(selected.span.byte_end, 11);
        let snapshot = DocumentSnapshot::new("ba pu ba".to_owned(), 1);
        let selected = snapshot.cmavo_sequence_documentation(2).unwrap();
        assert_eq!(selected.span.byte_start, 3);
        assert_eq!(selected.span.byte_end, 8);
    }
}
