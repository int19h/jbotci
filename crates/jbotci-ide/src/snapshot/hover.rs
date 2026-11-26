#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_dictionary::normalize_lookup_query;
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
    #[invariant(true)]
    #[invariant(::Like(_, _) => true)]
    #[invariant(::Word(_, _) => true)]
    enum HoverJob<'word> {
        Like(&'word WordLike, HoverSemantics),
        Word(&'word Word, HoverSemantics),
    }
    let mut jobs = vec![HoverJob::Like(word_like, semantics)];
    while let Some(job) = jobs.pop() {
        match job {
            HoverJob::Word(word, semantics) => {
                if let Some(found) = hover_word_at(word, offset, semantics) {
                    return Some(found);
                }
            }
            HoverJob::Like(word_like, semantics) => match word_like.as_data() {
                data!(WordLike::PlainWord(word)) => {
                    if let Some(found) = hover_word_at(word, offset, semantics) {
                        return Some(found);
                    }
                }
                data!(WordLike::QuotedWord { zo, word }) => {
                    let semantics = inherited_semantics(semantics, HoverSemantics::Dictionary);
                    jobs.push(HoverJob::Word(word, semantics));
                    jobs.push(HoverJob::Word(zo, semantics));
                }
                data!(WordLike::SelmahoQuotedWord { mahoi, word }) => {
                    let semantics = inherited_semantics(semantics, HoverSemantics::Dictionary);
                    jobs.push(HoverJob::Word(word, semantics));
                    jobs.push(HoverJob::Word(mahoi, semantics));
                }
                data!(WordLike::DelimitedNonLojbanQuote {
                    zoi,
                    opening_delimiter,
                    closing_delimiter,
                    ..
                }) => {
                    jobs.push(HoverJob::Word(
                        closing_delimiter,
                        HoverSemantics::MorphologyOnly,
                    ));
                    jobs.push(HoverJob::Word(
                        opening_delimiter,
                        HoverSemantics::MorphologyOnly,
                    ));
                    jobs.push(HoverJob::Word(
                        zoi,
                        inherited_semantics(semantics, HoverSemantics::Dictionary),
                    ));
                }
                data!(WordLike::QuotedWords {
                    lohu,
                    quoted_words,
                    lehu,
                }) => {
                    jobs.push(HoverJob::Word(
                        lehu,
                        inherited_semantics(semantics, HoverSemantics::Dictionary),
                    ));
                    for word in quoted_words.iter().rev() {
                        jobs.push(HoverJob::Word(word, HoverSemantics::MorphologyOnly));
                    }
                    jobs.push(HoverJob::Word(
                        lohu,
                        inherited_semantics(semantics, HoverSemantics::Dictionary),
                    ));
                }
                data!(WordLike::DelimitedWordQuote { marker, .. }) => {
                    if let Some(found) = hover_word_at(
                        marker,
                        offset,
                        inherited_semantics(semantics, HoverSemantics::Dictionary),
                    ) {
                        return Some(found);
                    }
                }
                data!(WordLike::ReplacementQuote { runs, lehai }) => {
                    jobs.push(HoverJob::Word(
                        lehai,
                        inherited_semantics(semantics, HoverSemantics::Dictionary),
                    ));
                    for run in runs.iter().rev() {
                        for word in run.words.iter().rev() {
                            jobs.push(HoverJob::Like(&word.word, HoverSemantics::MorphologyOnly));
                        }
                        jobs.push(HoverJob::Word(
                            &run.marker,
                            inherited_semantics(semantics, HoverSemantics::Dictionary),
                        ));
                    }
                }
                data!(WordLike::LerfuWord { base, bu }) => {
                    jobs.push(HoverJob::Word(bu, HoverSemantics::MorphologyOnly));
                    jobs.push(HoverJob::Like(base, HoverSemantics::MorphologyOnly));
                }
                data!(WordLike::ZeiCompound { left, zei, right }) => {
                    jobs.push(HoverJob::Word(right, HoverSemantics::MorphologyOnly));
                    jobs.push(HoverJob::Word(zei, HoverSemantics::MorphologyOnly));
                    jobs.push(HoverJob::Like(left, HoverSemantics::MorphologyOnly));
                }
            },
        }
    }
    None
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
        .into_iter()
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
    let mut tasks = vec![new!(HeadTask::Enter(word_like))];
    let mut open = Vec::new();
    let mut built = Vec::new();
    while let Some(task) = tasks.pop() {
        match task.into_data() {
            HeadTaskData::Text(text) => deliver_head(&mut tasks, &mut open, &mut built, text),
            HeadTaskData::Lerfu { bu, base } => {
                let left = base.expect("lerfu base headword");
                deliver_head(&mut tasks, &mut open, &mut built, format!("{left} {bu}"));
            }
            HeadTaskData::Zei { right, left } => {
                let left = left.expect("zei left headword");
                deliver_head(
                    &mut tasks,
                    &mut open,
                    &mut built,
                    format!("{left} zei {right}"),
                );
            }
            HeadTaskData::Join { got, .. } => {
                deliver_head(&mut tasks, &mut open, &mut built, got.join(" "));
            }
            HeadTaskData::Enter(word_like) => {
                if let Some(lookup_text) = word_like_lookup_text(word_like) {
                    deliver_head(&mut tasks, &mut open, &mut built, lookup_text);
                    continue;
                }
                match word_like.as_data() {
                    data!(WordLike::PlainWord(word)) => {
                        deliver_head(&mut tasks, &mut open, &mut built, word.canonical_phonemes())
                    }
                    data!(WordLike::QuotedWord { zo, word }) => {
                        deliver_head(
                            &mut tasks,
                            &mut open,
                            &mut built,
                            format!("{} {}", zo.canonical_phonemes(), word.canonical_phonemes()),
                        );
                    }
                    data!(WordLike::SelmahoQuotedWord { mahoi, word }) => {
                        deliver_head(
                            &mut tasks,
                            &mut open,
                            &mut built,
                            format!(
                                "{} {}",
                                mahoi.canonical_phonemes(),
                                word.canonical_phonemes()
                            ),
                        );
                    }
                    data!(WordLike::DelimitedNonLojbanQuote {
                        zoi,
                        opening_delimiter,
                        quoted_text,
                        closing_delimiter,
                    }) => deliver_head(
                        &mut tasks,
                        &mut open,
                        &mut built,
                        format!(
                            "{} {} {} {}",
                            zoi.canonical_phonemes(),
                            opening_delimiter.canonical_phonemes(),
                            quoted_text.text,
                            closing_delimiter.canonical_phonemes(),
                        ),
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
                        deliver_head(&mut tasks, &mut open, &mut built, headword);
                    }
                    data!(WordLike::DelimitedWordQuote {
                        marker,
                        quoted_text,
                    }) => deliver_head(
                        &mut tasks,
                        &mut open,
                        &mut built,
                        format!("{} {}", marker.canonical_phonemes(), quoted_text.text),
                    ),
                    data!(WordLike::ReplacementQuote { runs, lehai }) => {
                        let count = runs
                            .iter()
                            .map(|run| run.words.len().saturating_add(1))
                            .sum::<usize>()
                            .saturating_add(1);
                        push_head_join(&mut tasks, &mut open, count);
                        tasks.push(new!(HeadTask::Text(lehai.canonical_phonemes())));
                        for run in runs.iter().rev() {
                            for word in run.words.iter().rev() {
                                tasks.push(new!(HeadTask::Enter(&word.word)));
                            }
                            tasks.push(new!(HeadTask::Text(run.marker.canonical_phonemes())));
                        }
                    }
                    data!(WordLike::LerfuWord { base, bu }) => {
                        push_head_lerfu(&mut tasks, &mut open, bu.canonical_phonemes());
                        tasks.push(new!(HeadTask::Enter(base)));
                    }
                    data!(WordLike::ZeiCompound { left, right, .. }) => {
                        push_head_zei(&mut tasks, &mut open, right.canonical_phonemes());
                        tasks.push(new!(HeadTask::Enter(left)));
                    }
                }
            }
        }
    }
    built.pop().expect("headword")
}

/// A join frame owns the parts it has received.
/// `got` stays at most `need`. A lerfu frame and a zei frame each wait for one base.
/// Any finished string is a legal headword piece. The missing base is the waiting state.
#[invariant(true)]
#[invariant(::Enter(_) => true)]
#[invariant(::Lerfu { .. } => true)]
#[invariant(::Zei { .. } => true)]
#[invariant(::Join { need, got } => got.len() <= *need)]
#[invariant(::Text(_) => true)]
enum HeadTask<'a> {
    Enter(&'a WordLike),
    Lerfu { bu: String, base: Option<String> },
    Zei { right: String, left: Option<String> },
    Join { need: usize, got: Vec<String> },
    Text(String),
}

#[requires(true)]
#[ensures(true)]
fn push_head_join<'a>(tasks: &mut Vec<HeadTask<'a>>, open: &mut Vec<usize>, need: usize) {
    tasks.push(new!(HeadTask::Join {
        need,
        got: Vec::new(),
    }));
    if need > 0 {
        open.push(tasks.len() - 1);
    }
}

#[requires(true)]
#[ensures(true)]
fn push_head_lerfu<'a>(tasks: &mut Vec<HeadTask<'a>>, open: &mut Vec<usize>, bu: String) {
    tasks.push(new!(HeadTask::Lerfu { bu, base: None }));
    open.push(tasks.len() - 1);
}

#[requires(true)]
#[ensures(true)]
fn push_head_zei<'a>(tasks: &mut Vec<HeadTask<'a>>, open: &mut Vec<usize>, right: String) {
    tasks.push(new!(HeadTask::Zei { right, left: None }));
    open.push(tasks.len() - 1);
}

#[requires(true)]
#[ensures(true)]
fn deliver_head<'a>(
    tasks: &mut Vec<HeadTask<'a>>,
    open: &mut Vec<usize>,
    built: &mut Vec<String>,
    text: String,
) {
    let Some(&index) = open.last() else {
        built.push(text);
        return;
    };
    let dummy = new!(HeadTask::Text(String::new()));
    let task = std::mem::replace(&mut tasks[index], dummy);
    let (task, full) = match task.into_data() {
        HeadTaskData::Lerfu { bu, .. } => (
            new!(HeadTask::Lerfu {
                bu,
                base: Some(text),
            }),
            true,
        ),
        HeadTaskData::Zei { right, .. } => (
            new!(HeadTask::Zei {
                right,
                left: Some(text),
            }),
            true,
        ),
        HeadTaskData::Join { need, mut got } => {
            got.push(text);
            let full = got.len() == need;
            (new!(HeadTask::Join { need, got }), full)
        }
        other => (HeadTask::from_data(other), false),
    };
    tasks[index] = task;
    if full {
        open.pop();
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
    let mut current = word_like;
    let mut rights = Vec::new();
    loop {
        match current.as_data() {
            data!(WordLike::ZeiCompound { left, right, .. }) => {
                rights.push(right);
                current = left;
            }
            _ => break,
        }
    }
    cards.extend(dictionary_cards_for_word_like_component(current));
    for right in rights.iter().rev() {
        cards.extend(dictionary_cards_for_word(right));
    }
}

#[requires(true)]
#[ensures(ret.iter().all(|card| card.author.is_some()))]
fn dictionary_cards_for_word_like_component(word_like: &WordLike) -> Vec<VlackuCard> {
    let Some(lookup_text) = word_like_lookup_text(word_like) else {
        return Vec::new();
    };
    let output = run_vlacku_requests(
        jbotci_dictionary_data::english(),
        &[VlackuRequest::valsi(lookup_text.clone())],
        &hover_vlacku_options(false),
    )
    .expect("a caller with no deadline holds a token that stays clear");
    output
        .cards
        .into_iter()
        .filter(|card| card.author.is_some() && same_lookup_key(&card.word, &lookup_text))
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
    )
    .expect("a caller with no deadline holds a token that stays clear");
    let decomposition = morphology_lujvo_decomposition(word, &output.cards);
    let own_cards = output
        .cards
        .iter()
        .filter(|card| card.author.is_some() && same_lookup_key(&card.word, &lookup_text))
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
                .find(|card| same_lookup_key(&card.word, &lookup_text))
                .cloned()
                .map(|card| with_decomposition(card, decomposition))
                .expect("a morphology-valid lujvo search has a classification card"),
        ];
        cards.extend(
            output
                .cards
                .iter()
                .filter(|card| card.author.is_some() && !same_lookup_key(&card.word, &lookup_text))
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
    let annotated = cards
        .iter()
        .find(|card| same_lookup_key(&card.word, &lookup_text) && !card.decomposition.is_empty());
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
                .filter(|piece| piece.kind == kind && same_lookup_key(&piece.surface, &surface))
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
    )
    .expect("a caller with no deadline holds a token that stays clear");
    output
        .cards
        .into_iter()
        .filter(|card| card.author.is_some())
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn same_lookup_key(left: &str, right: &str) -> bool {
    normalize_lookup_query(left) == normalize_lookup_query(right)
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
    use bityzba::try_new;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_join_frame_with_too_many_parts_is_rejected() {
        assert!(
            try_new!(HeadTask::Join {
                need: 0,
                got: vec!["bu".to_owned()],
            })
            .is_err()
        );
    }

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

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lookup_key_matches_h_and_modifier_apostrophe_cards() {
        assert!(same_lookup_key("nuhempcr", "nu'empcr"));
        assert!(same_lookup_key("gaurkihu", "gaurki'u"));
        assert!(same_lookup_key("gaurki\u{02bc}u", "gaurki'u"));
        assert!(same_lookup_key("gaurki\u{2019}u", "gaurki'u"));
        assert!(!same_lookup_key("jo\u{2018}a", "jo'a"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zei_hover_keeps_the_h_spelling_component_card() {
        for source in [
            "nu'empcr zei klama",
            "nu\u{02bc}empcr zei klama",
            "nuhempcr zei klama",
        ] {
            let markdown = DocumentSnapshot::new(source.to_owned(), 1)
                .hover(0)
                .expect("zei hover")
                .markdown
                .clone();
            assert!(
                markdown.contains("New Hampshire"),
                "{source} dropped the nuhempcr card: {markdown}"
            );
            assert!(markdown.contains("nu'empcr zei klama"), "{source}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lujvo_hover_keeps_the_best_card_for_apostrophe_spellings() {
        for source in ["gaurki'u", "gaurkihu", "gaurki\u{02bc}u", "gaurki\u{2019}u"] {
            let markdown = DocumentSnapshot::new(source.to_owned(), 1)
                .hover(0)
                .expect("lujvo hover")
                .markdown
                .clone();
            let best = markdown.find("gaurki'u").expect("best lujvo card");
            if let Some(worse) = markdown.find("gaurkihu") {
                assert!(best < worse, "{source}");
            }
        }
    }
}

#[cfg(test)]
mod deep_links {
    use std::thread;

    use bityzba::requires;
    use jbotci_morphology::{WordLike, segment_words_with_modifiers};

    use super::{
        HoverSemantics, append_zei_component_cards, hover_word_in_word_like, word_like_headword,
        word_like_lookup_text,
    };

    #[requires(links > 0)]
    #[ensures(true)]
    fn parse_chain(kind: &str, links: usize) -> WordLike {
        let mut text = if kind == "bu" {
            String::from("a")
        } else {
            String::from("broda")
        };
        let piece = if kind == "bu" { " bu" } else { " zei broda" };
        for _ in 0..links {
            text.push_str(piece);
        }
        let kind = kind.to_owned();
        thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                let words = segment_words_with_modifiers(&text)
                    .unwrap_or_else(|error| panic!("{kind} parse {error}"));
                assert_eq!(words.len(), 1);
                words.into_iter().next().expect("one word")
            })
            .expect("parse thread")
            .join()
            .expect("parse join")
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn hover_walks_fit_in_160kib() {
        let bu = parse_chain("bu", 2666);
        let zei = parse_chain("zei", 799);
        let bu_lookup = word_like_lookup_text(&bu);
        let zei_lookup = word_like_lookup_text(&zei);
        let bu_head = thread::Builder::new()
            .stack_size(160 * 1024)
            .spawn({
                let word = bu.clone();
                move || word_like_headword(&word)
            })
            .expect("head thread")
            .join()
            .expect("head join");
        let zei_head = thread::Builder::new()
            .stack_size(160 * 1024)
            .spawn({
                let word = zei.clone();
                move || word_like_headword(&word)
            })
            .expect("head thread")
            .join()
            .expect("head join");
        assert_eq!(bu_head, bu_lookup.expect("bu lookup"));
        assert_eq!(zei_head, zei_lookup.expect("zei lookup"));
        let bu_hit = thread::Builder::new()
            .stack_size(160 * 1024)
            .spawn({
                let word = bu.clone();
                move || hover_word_in_word_like(&word, 0, HoverSemantics::MorphologyOnly).is_some()
            })
            .expect("hover thread")
            .join()
            .expect("hover join");
        let zei_hit = thread::Builder::new()
            .stack_size(160 * 1024)
            .spawn({
                let word = zei.clone();
                move || hover_word_in_word_like(&word, 0, HoverSemantics::MorphologyOnly).is_some()
            })
            .expect("hover thread")
            .join()
            .expect("hover join");
        assert!(bu_hit);
        assert!(zei_hit);
        let zei_cards = thread::Builder::new()
            .stack_size(160 * 1024)
            .spawn(move || {
                let mut cards = Vec::new();
                append_zei_component_cards(&zei, &mut cards);
                cards.len()
            })
            .expect("card thread")
            .join()
            .expect("card join");
        let bu_cards = thread::Builder::new()
            .stack_size(160 * 1024)
            .spawn(move || {
                let mut cards = Vec::new();
                append_zei_component_cards(&bu, &mut cards);
                cards.len()
            })
            .expect("card thread")
            .join()
            .expect("card join");
        assert!(zei_cards >= 800, "zei cards {zei_cards}");
        let _ = bu_cards;
    }
}
