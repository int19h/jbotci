use std::collections::HashMap;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_morphology::{LujvoPart, Word, WordKind, WordLike, WordLikeData, canonicalize_text};
use jbotci_output::{render_vlacku_cards_markdown, render_vlacku_decomposition_markdown};
use jbotci_search::vlacku::{
    VlackuCard, VlackuCompositionKind, VlackuCompositionPiece, VlackuRequest, VlackuSearchOptions,
    normalize_word_type_filter, run_vlacku_requests,
};
use jbotci_source::SourceSpan;

use super::DocumentSnapshot;

/// Markdown hover documentation and its full half-open morphology word span.
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

#[invariant(!word.is_empty() && !cards.is_empty())]
#[derive(Debug, Clone, PartialEq)]
struct CmavoSequenceDocumentation {
    word: String,
    cards: Vec<VlackuCard>,
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
    /// For a cmavo, dictionary-attested sequences containing that constituent follow
    /// its lead card longest-first; equal-length sequences retain source order.
    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|hover| hover.span.char_start <= char_offset && char_offset < hover.span.char_end))]
    pub fn hover(&self, char_offset: usize) -> Option<HoverContent> {
        let word_at = self.word_at(char_offset)?;
        let target =
            hover_word_in_word_like(word_at.word, char_offset, HoverSemantics::Dictionary)?;
        let mut markdown = match target.semantics {
            HoverSemantics::Dictionary => dictionary_hover_markdown(target.word),
            HoverSemantics::MorphologyOnly => word_classification_markdown(target.word),
        };

        if target.semantics == HoverSemantics::Dictionary
            && word_at
                .word
                .bare_word()
                .is_some_and(|word| std::ptr::eq(word, target.word))
        {
            for sequence in self.cmavo_sequence_documentation(word_at.index) {
                markdown.push_str("\n\n---\n\n## Cmavo sequence ");
                markdown.push_str(&format!("`{}`", sequence.word));
                markdown.push_str("\n\n");
                markdown.push_str(&render_vlacku_cards_markdown(&sequence.cards));
            }
        }

        Some(new!(HoverContent {
            markdown,
            span: target.word.span().clone(),
        }))
    }

    #[requires(index < self.words.words.len())]
    #[ensures(ret.iter().all(|sequence| !sequence.cards.is_empty()))]
    fn cmavo_sequence_documentation(&self, index: usize) -> Vec<CmavoSequenceDocumentation> {
        if plain_cmavo_word(&self.words.words[index]).is_none() {
            return Vec::new();
        }
        let mut run_start = index;
        while run_start > 0 && plain_cmavo_word(&self.words.words[run_start - 1]).is_some() {
            run_start -= 1;
        }

        let mut run_end = index + 1;
        while run_end < self.words.words.len()
            && plain_cmavo_word(&self.words.words[run_end]).is_some()
        {
            run_end += 1;
        }

        let candidates =
            cmavo_sequence_candidates(&self.words.words[run_start..run_end], index - run_start);
        let requests = candidates
            .iter()
            .cloned()
            .map(VlackuRequest::valsi)
            .collect::<Vec<_>>();
        if requests.is_empty() {
            return Vec::new();
        }
        let output = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &requests,
            &hover_vlacku_options(false),
        );
        let mut candidate_indexes = HashMap::new();
        for (candidate_index, candidate) in candidates.iter().enumerate() {
            candidate_indexes
                .entry(candidate.as_str())
                .or_insert(candidate_index);
        }
        let mut card_groups = (0..candidates.len())
            .map(|_| Vec::new())
            .collect::<Vec<Vec<VlackuCard>>>();
        for card in output.cards {
            if !is_dictionary_cmavo_sequence_card(&card) {
                continue;
            }
            let canonical_word = canonicalize_text(&card.word);
            let Some(&candidate_index) = candidate_indexes.get(canonical_word.as_str()) else {
                continue;
            };
            let card = cmavo_sequence_card_for_hover(card);
            if !card_groups[candidate_index].contains(&card) {
                card_groups[candidate_index].push(card);
            }
        }

        candidates
            .into_iter()
            .zip(card_groups)
            .filter_map(|(word, cards)| {
                (!cards.is_empty()).then(|| new!(CmavoSequenceDocumentation { word, cards }))
            })
            .collect()
    }
}

/// Enumerate every word-stream-contiguous cmavo sequence containing
/// `hovered_index`, ordered by decreasing constituent count and then source order.
#[requires(hovered_index < words.len() && words.iter().all(|word| plain_cmavo_word(word).is_some()))]
#[ensures(ret.iter().all(|candidate| !candidate.is_empty()))]
fn cmavo_sequence_candidates(words: &[WordLike], hovered_index: usize) -> Vec<String> {
    let mut candidates = Vec::new();
    for length in (2..=words.len()).rev() {
        let first_start = (hovered_index + 1).saturating_sub(length);
        let last_start = hovered_index.min(words.len() - length);
        for start in first_start..=last_start {
            let mut candidate = String::new();
            for word_like in &words[start..start + length] {
                candidate.push_str(
                    &plain_cmavo_word(word_like)
                        .expect("cmavo sequence candidates come from a plain-cmavo run")
                        .canonical_phonemes(),
                );
            }
            candidates.push(candidate);
        }
    }
    candidates
}

#[requires(true)]
#[ensures(true)]
fn is_dictionary_cmavo_sequence_card(card: &VlackuCard) -> bool {
    card.author.is_some() && normalize_word_type_filter(&card.word_type) == "cmavo-compound"
}

#[requires(is_dictionary_cmavo_sequence_card(&card))]
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
    let mut markdown = String::from("**Word type:** lujvo");
    if !decomposition.is_empty() {
        markdown.push_str("\n\n**Decomposition:** ");
        markdown.push_str(&render_vlacku_decomposition_markdown(&decomposition));
    }

    let own_cards = output
        .cards
        .iter()
        .filter(|card| card.author.is_some() && canonicalize_text(&card.word) == lookup_text)
        .cloned()
        .map(without_decomposition)
        .collect::<Vec<_>>();
    let cards = if own_cards.is_empty() {
        markdown.push_str("\n\n**Component definitions**");
        output
            .cards
            .iter()
            .filter(|card| card.author.is_some() && canonicalize_text(&card.word) != lookup_text)
            .cloned()
            .map(without_decomposition)
            .collect::<Vec<_>>()
    } else {
        own_cards
    };
    if !cards.is_empty() {
        markdown.push_str("\n\n---\n\n");
        markdown.push_str(&render_vlacku_cards_markdown(&cards));
    }
    markdown
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
    match word.kind() {
        WordKind::Cmavo => match word.selmaho() {
            Some(selmaho) => format!("**Word type:** cmavo\n\n**Selma'o:** `{selmaho}`"),
            None => "**Word type:** cmavo".to_owned(),
        },
        WordKind::Gismu => "**Word type:** gismu".to_owned(),
        WordKind::Lujvo => "**Word type:** lujvo".to_owned(),
        WordKind::Fuhivla => "**Word type:** fu'ivla".to_owned(),
        WordKind::Cmevla => "**Word type:** name word (cmevla)".to_owned(),
    }
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(ret.is_some() == word_like.bare_word().is_some_and(|word| word.kind() == WordKind::Cmavo))]
fn plain_cmavo_word(word_like: &WordLike) -> Option<&Word> {
    word_like
        .bare_word()
        .filter(|word| word.kind() == WordKind::Cmavo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sequence_candidates_include_every_containing_subsequence_in_documented_order() {
        let snapshot = DocumentSnapshot::new("bi no no vo".to_owned(), 1);

        assert_eq!(
            cmavo_sequence_candidates(&snapshot.words.words, 1),
            ["binonovo", "binono", "nonovo", "bino", "nono"],
        );
    }
}
