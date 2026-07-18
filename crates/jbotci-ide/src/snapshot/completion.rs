use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_dictionary::{Dictionary, WordType, normalize_lookup_query};
use jbotci_morphology::{
    Cmavo, MorphologyContextKind, MorphologyError, Selmaho, WordKind, WordLike, WordLikeData,
    analyze_valsi, canonicalize_text, is_word_forming_character, segment_words_with_modifiers,
    segment_words_with_modifiers_recovered,
};
use jbotci_output::render_vlacku_cards_markdown;
use jbotci_search::vlacku::dictionary_entry_card;
use jbotci_source::SourceSpan;
use jbotci_syntax::{
    ParseOptions, SyntaxExpectation, SyntaxExpectationReason, SyntaxExpectationReasonData,
    SyntaxExpectedToken, SyntaxExpectedTokenData, SyntaxWordCategory,
    expected_continuations_with_time_limit,
};

use super::{DocumentSnapshot, SemanticTokenKind};

mod tree_context;

use tree_context::{OpenConstructCandidate, TreeCompletionContext};

/// Transport-neutral completion classification.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Brivla,
    ProSumti,
    LetterWord,
    Cmavo,
    Cmevla,
    Terminator,
}

/// Which cursor interpretation produced a completion candidate.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionInterpretation {
    Continue,
    Extend,
}

/// Grammar source that produced a candidate.
///
/// This remains part of the result even when multiple expectations expand to
/// the same spelling. For example, `i` is both a BY letter word and the I
/// statement separator; callers must be able to distinguish those meanings.
#[invariant(::Expected { token } => !token.summary_text().is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompletionProvenance {
    Expected { token: SyntaxExpectedToken },
    GrammarUnavailable,
    UnfilteredQuote,
}

// The wall-clock limit is the completion-specific safety boundary. Recovery's
// shared memo now makes a separate low error cap counterproductive: it would
// discard grammar context even when the engine can reach the cut in time.
// Parse-dominated documents may still exhaust this limit and deliberately
// degrade to morphology-valid candidates.
const COMPLETION_GRAMMAR_TIME_LIMIT: Duration = Duration::from_secs(1);

/// Cooperative cancellation shared with a completion worker.
#[invariant(true)]
#[derive(Debug, Clone, Default)]
pub struct CompletionCancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CompletionCancellationToken {
    /// Create a live completion cancellation token.
    #[requires(true)]
    #[ensures(!ret.is_cancelled())]
    pub fn new() -> Self {
        Self::default()
    }

    /// Request cancellation of every completion sharing this token.
    #[requires(true)]
    #[ensures(self.is_cancelled())]
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Return whether cancellation has been requested.
    #[requires(true)]
    #[ensures(true)]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

impl CompletionInterpretation {
    #[requires(true)]
    #[ensures(ret <= 1)]
    pub const fn sort_rank(self) -> u8 {
        match self {
            Self::Continue => 0,
            Self::Extend => 1,
        }
    }
}

/// Opaque dictionary/morphology key retained for lazy documentation resolution.
#[invariant(!word.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionDocumentationHandle {
    word: String,
}

impl CompletionDocumentationHandle {
    #[requires(!word.is_empty())]
    #[ensures(true)]
    pub fn new(word: String) -> Self {
        new!(CompletionDocumentationHandle { word })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn word(&self) -> &str {
        &self.word
    }
}

/// One completion candidate in document source coordinates.
#[invariant(!label.is_empty())]
#[invariant(replacement_span.byte_start <= replacement_span.byte_end)]
#[invariant(replacement_span.char_start <= replacement_span.char_end)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub interpretation: CompletionInterpretation,
    pub provenance: CompletionProvenance,
    pub reason: SyntaxExpectationReason,
    pub replacement_span: SourceSpan,
    pub short_gloss: Option<String>,
    pub documentation: CompletionDocumentationHandle,
    suffix_consistent: bool,
}

impl CompletionItem {
    #[requires(true)]
    #[ensures(ret <= 2)]
    pub fn reason_sort_rank(&self) -> u8 {
        reason_sort_rank(&self.reason)
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CursorCompletionContext {
    Grammar,
    UnfilteredQuotedWord,
    UnfilteredQuotedWords,
    SuppressedNonLojbanQuote,
}

// This short-lived context captures the immutable facts shared by one
// cursor interpretation. Candidate accumulation remains an explicit mutable
// parameter, so the validated wrapper never needs interior mutation.
#[invariant(replacement_span.byte_end <= snapshot.text.len())]
#[invariant(replacement_span.char_end <= snapshot.line_index.char_len())]
struct CompletionContext<'snapshot, 'dictionary, 'entries, 'cancellation> {
    snapshot: &'snapshot DocumentSnapshot,
    dictionary: &'dictionary Dictionary<'entries>,
    document_cmevla: &'snapshot BTreeSet<String>,
    cancellation: &'cancellation CompletionCancellationToken,
    interpretation: CompletionInterpretation,
    replacement_span: SourceSpan,
    normalized_prefix: String,
    suffix_consistent_labels: BTreeSet<String>,
}

impl DocumentSnapshot {
    /// Return grammar- and morphology-filtered completions at `char_offset`.
    ///
    /// The cursor is clamped through the snapshot's line index. Non-empty seeds
    /// are interpreted both as an incomplete word and, when the cursor is at
    /// the word end and the seed segments cleanly, as a complete word before a
    /// new insertion point.
    #[requires(true)]
    #[ensures(ret.windows(2).all(|items| completion_sort_key(&items[0]) <= completion_sort_key(&items[1])))]
    pub fn completions(&self, char_offset: usize) -> Vec<CompletionItem> {
        self.completions_cancellable(char_offset, &CompletionCancellationToken::new())
    }

    /// Return completions while cooperatively observing `cancellation`.
    #[requires(true)]
    #[ensures(ret.windows(2).all(|items| completion_sort_key(&items[0]) <= completion_sort_key(&items[1])))]
    pub fn completions_cancellable(
        &self,
        char_offset: usize,
        cancellation: &CompletionCancellationToken,
    ) -> Vec<CompletionItem> {
        self.completions_with_grammar_time_limit_and_cancellation(
            char_offset,
            COMPLETION_GRAMMAR_TIME_LIMIT,
            cancellation,
        )
    }

    #[requires(!grammar_time_limit.is_zero())]
    #[ensures(ret.windows(2).all(|items| completion_sort_key(&items[0]) <= completion_sort_key(&items[1])))]
    fn completions_with_grammar_time_limit(
        &self,
        char_offset: usize,
        grammar_time_limit: Duration,
    ) -> Vec<CompletionItem> {
        self.completions_with_grammar_time_limit_and_cancellation(
            char_offset,
            grammar_time_limit,
            &CompletionCancellationToken::new(),
        )
    }

    #[requires(!grammar_time_limit.is_zero())]
    #[ensures(ret.windows(2).all(|items| completion_sort_key(&items[0]) <= completion_sort_key(&items[1])))]
    fn completions_with_grammar_time_limit_and_cancellation(
        &self,
        char_offset: usize,
        grammar_time_limit: Duration,
        cancellation: &CompletionCancellationToken,
    ) -> Vec<CompletionItem> {
        if cancellation.is_cancelled() {
            return Vec::new();
        }
        let cursor = self.line_index.offsets_for_char(char_offset);
        let seed_span = self.completion_seed_span(cursor.byte, cursor.char);
        let seed = &self.text[seed_span.byte_start..seed_span.byte_end];
        let replacement_span = self.completion_replacement_span(&seed_span);
        let preceding_source = &self.text[..seed_span.byte_start];
        let preceding_segmentation =
            segment_words_with_modifiers_recovered(preceding_source).into_data();
        let preceding_awaits_zo_target = segmentation_awaits_zo_target(
            &preceding_segmentation.errors,
            &preceding_segmentation.error_regions,
            seed_span.char_start,
        );
        let preceding_words = preceding_segmentation.words;
        let dictionary = jbotci_dictionary_data::english();
        let document_cmevla = self.document_cmevla();
        let mut items = BTreeMap::new();

        if !seed.is_empty() {
            self.add_completion_interpretation(
                dictionary,
                &document_cmevla,
                CompletionInterpretation::Extend,
                replacement_span.clone(),
                seed,
                &preceding_words,
                preceding_awaits_zo_target,
                grammar_time_limit,
                cancellation,
                &mut items,
            );
        }

        if cancellation.is_cancelled() {
            return Vec::new();
        }

        let cursor_ends_word = self.text[cursor.byte..]
            .chars()
            .next()
            .is_none_or(|value| !is_word_forming_character(value));
        let seed_is_complete =
            seed.is_empty() || (cursor_ends_word && segment_words_with_modifiers(seed).is_ok());
        if seed_is_complete {
            // Segmenting `seed` alone establishes the Continue interpretation,
            // but those word spans start at zero. Re-segment the complete
            // source prefix so syntax recovery receives globally ordered spans.
            let completed_words;
            let continuation_awaits_zo_target;
            let continuation_words = if seed.is_empty() {
                continuation_awaits_zo_target = preceding_awaits_zo_target;
                &preceding_words
            } else {
                let completed_segmentation =
                    segment_words_with_modifiers_recovered(&self.text[..cursor.byte]).into_data();
                continuation_awaits_zo_target = segmentation_awaits_zo_target(
                    &completed_segmentation.errors,
                    &completed_segmentation.error_regions,
                    cursor.char,
                );
                completed_words = completed_segmentation.words;
                &completed_words
            };
            let continuation_span =
                if seed.is_empty() && replacement_span.byte_start < replacement_span.byte_end {
                    // At the left edge of an existing word, completion replaces
                    // that word. Treating the cut as a zero-width insertion would
                    // fuse every candidate to the right-hand word and force a
                    // separate morphology parse for every candidate merely to
                    // reject most of them.
                    replacement_span.clone()
                } else {
                    SourceSpan::new(None, cursor.byte, cursor.byte, cursor.char, cursor.char)
                        .expect("a cursor position is an ordered empty source span")
                };
            self.add_completion_interpretation(
                dictionary,
                &document_cmevla,
                CompletionInterpretation::Continue,
                continuation_span,
                "",
                continuation_words,
                continuation_awaits_zo_target,
                grammar_time_limit,
                cancellation,
                &mut items,
            );
        }

        if cancellation.is_cancelled() {
            return Vec::new();
        }

        let short_glosses = {
            let labels = items
                .values()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>();
            dictionary.first_gloss_keywords_for_words(&labels)
        };
        if cancellation.is_cancelled() {
            return Vec::new();
        }
        let mut result = items
            .into_values()
            .zip(short_glosses)
            .map(|(item, short_gloss)| {
                item.with_data(data! {
                    short_gloss: short_gloss.map(str::to_owned),
                })
            })
            .collect::<Vec<_>>();
        result.sort_by(|left, right| completion_sort_key(left).cmp(&completion_sort_key(right)));
        result
    }

    #[requires(cursor_byte <= self.text.len())]
    #[requires(cursor_char <= self.line_index.char_len())]
    #[ensures(ret.byte_end == cursor_byte && ret.char_end == cursor_char)]
    fn completion_seed_span(&self, cursor_byte: usize, cursor_char: usize) -> SourceSpan {
        let prefix = &self.text[..cursor_byte];
        let mut byte_start = cursor_byte;
        for (byte_offset, value) in prefix.char_indices().rev() {
            if !is_word_forming_character(value) {
                break;
            }
            byte_start = byte_offset;
        }
        let seed_char_len = self.text[byte_start..cursor_byte].chars().count();
        SourceSpan::new(
            None,
            byte_start,
            cursor_byte,
            cursor_char - seed_char_len,
            cursor_char,
        )
        .expect("the trailing seed is an ordered source slice")
    }

    #[requires(seed_span.byte_end <= self.text.len())]
    #[requires(seed_span.char_end <= self.line_index.char_len())]
    #[ensures(ret.byte_start == seed_span.byte_start)]
    #[ensures(ret.char_start == seed_span.char_start)]
    #[ensures(ret.byte_end >= seed_span.byte_end)]
    #[ensures(ret.char_end >= seed_span.char_end)]
    fn completion_replacement_span(&self, seed_span: &SourceSpan) -> SourceSpan {
        let mut byte_end = seed_span.byte_end;
        let mut char_end = seed_span.char_end;
        for value in self.text[seed_span.byte_end..].chars() {
            if !is_word_forming_character(value) {
                break;
            }
            byte_end += value.len_utf8();
            char_end += 1;
        }
        SourceSpan::new(
            None,
            seed_span.byte_start,
            byte_end,
            seed_span.char_start,
            char_end,
        )
        .expect("the current word is an ordered source slice")
    }

    #[allow(clippy::too_many_arguments)]
    #[requires(replacement_span.byte_end <= self.text.len())]
    #[requires(replacement_span.char_end <= self.line_index.char_len())]
    #[requires(!grammar_time_limit.is_zero())]
    #[ensures(true)]
    fn add_completion_interpretation<'dictionary, 'entries>(
        &self,
        dictionary: &'dictionary Dictionary<'entries>,
        document_cmevla: &BTreeSet<String>,
        interpretation: CompletionInterpretation,
        replacement_span: SourceSpan,
        prefix: &str,
        preceding_words: &[WordLike],
        preceding_awaits_zo_target: bool,
        grammar_time_limit: Duration,
        cancellation: &CompletionCancellationToken,
        items: &mut BTreeMap<(u8, String, CompletionProvenance), CompletionItem>,
    ) {
        if cancellation.is_cancelled() {
            return;
        }
        let tree_context = TreeCompletionContext::from_parse(
            &self.parse,
            replacement_span.byte_start,
            self.text.len(),
        );
        let statement_words = statement_local_words(preceding_words, tree_context.restart_byte);
        let cursor_context = match self.cursor_completion_context(&replacement_span) {
            CursorCompletionContext::Grammar if preceding_awaits_zo_target => {
                CursorCompletionContext::UnfilteredQuotedWord
            }
            context => context,
        };
        let suffix_consistent_labels = tree_context
            .suffix_consistent_cmavo
            .iter()
            .copied()
            .chain(
                tree_context
                    .open_constructs
                    .iter()
                    .map(|candidate| candidate.cmavo),
            )
            .map(|cmavo| normalize_lookup_query(cmavo.canonical_text()))
            .collect();
        let completion_context = new!(CompletionContext {
            snapshot: self,
            dictionary,
            document_cmevla,
            cancellation,
            interpretation,
            replacement_span,
            normalized_prefix: normalize_lookup_query(prefix),
            suffix_consistent_labels,
        });
        match cursor_context {
            CursorCompletionContext::SuppressedNonLojbanQuote => {}
            CursorCompletionContext::UnfilteredQuotedWord => completion_context
                .add_unfiltered_candidates(
                    new!(SyntaxExpectationReason::ContinueCurrent {
                        construct: "quoted word".to_owned(),
                    }),
                    new!(CompletionProvenance::UnfilteredQuote),
                    items,
                ),
            CursorCompletionContext::UnfilteredQuotedWords => completion_context
                .add_unfiltered_candidates(
                    new!(SyntaxExpectationReason::ContinueCurrent {
                        construct: "LOhU quote".to_owned(),
                    }),
                    new!(CompletionProvenance::UnfilteredQuote),
                    items,
                ),
            CursorCompletionContext::Grammar => {
                if cancellation.is_cancelled() {
                    return;
                }
                let expectations = expected_continuations_with_time_limit(
                    statement_words,
                    &ParseOptions::default(),
                    grammar_time_limit,
                );
                if cancellation.is_cancelled() {
                    return;
                }
                if completion_context.add_grammar_candidates(&expectations, items) {
                    completion_context
                        .add_open_construct_candidates(&tree_context.open_constructs, items);
                }
            }
        }
    }

    #[requires(replacement_span.byte_end <= self.text.len())]
    #[requires(replacement_span.char_end <= self.line_index.char_len())]
    #[ensures(true)]
    fn cursor_completion_context(&self, replacement_span: &SourceSpan) -> CursorCompletionContext {
        for word_like in &self.words.words {
            match word_like.as_data() {
                data!(WordLike::DelimitedNonLojbanQuote { quoted_text, .. })
                    if span_or_cursor_is_within(replacement_span, &quoted_text.span) =>
                {
                    return CursorCompletionContext::SuppressedNonLojbanQuote;
                }
                data!(WordLike::QuotedWord { zo, word })
                    if zo.is_cmavo(Cmavo::Zo)
                        && ((replacement_span.char_start < replacement_span.char_end
                            && word.span().char_start <= replacement_span.char_start
                            && replacement_span.char_end <= word.span().char_end)
                            || (replacement_span.char_start == replacement_span.char_end
                                && zo.span().char_end <= replacement_span.char_start
                                && replacement_span.char_start <= word.span().char_start)) =>
                {
                    return CursorCompletionContext::UnfilteredQuotedWord;
                }
                data!(WordLike::QuotedWords { lohu, lehu, .. })
                    if lohu.span().char_end <= replacement_span.char_start
                        && replacement_span.char_end <= lehu.span().char_start =>
                {
                    return CursorCompletionContext::UnfilteredQuotedWords;
                }
                _ => {}
            }
        }
        CursorCompletionContext::Grammar
    }

    #[requires(true)]
    #[ensures(ret.iter().all(|word| !word.is_empty()))]
    fn document_cmevla(&self) -> BTreeSet<String> {
        self.semantic_tokens
            .iter()
            .filter(|token| token.kind == SemanticTokenKind::Cmevla)
            .map(|token| canonicalize_text(&self.text[token.span.byte_start..token.span.byte_end]))
            .filter(|word| !word.is_empty())
            .collect()
    }

    #[requires(!label.is_empty())]
    #[requires(replacement_span.byte_end <= self.text.len())]
    #[requires(replacement_span.char_end <= self.line_index.char_len())]
    #[ensures(true)]
    fn completion_candidate_surfaces(&self, label: &str, replacement_span: &SourceSpan) -> bool {
        let mut byte_start = replacement_span.byte_start;
        for (byte_offset, value) in self.text[..replacement_span.byte_start]
            .char_indices()
            .rev()
        {
            if !is_word_forming_character(value) {
                break;
            }
            byte_start = byte_offset;
        }
        let mut byte_end = replacement_span.byte_end;
        for value in self.text[replacement_span.byte_end..].chars() {
            if !is_word_forming_character(value) {
                break;
            }
            byte_end += value.len_utf8();
        }

        // Every candidate source already guarantees that `label` is a word in
        // isolation. With separators on both sides there is no morphology
        // boundary to re-check, which is the common empty-prefix path.
        if byte_start == replacement_span.byte_start && byte_end == replacement_span.byte_end {
            return true;
        }

        let left = &self.text[byte_start..replacement_span.byte_start];
        let right = &self.text[replacement_span.byte_end..byte_end];
        let mut updated = String::with_capacity(left.len() + label.len() + right.len());
        updated.push_str(left);
        updated.push_str(label);
        updated.push_str(right);
        let candidate_byte_start = left.len();
        let candidate_byte_end = candidate_byte_start + label.len();
        let candidate_char_start = left.chars().count();
        let candidate_char_end = candidate_char_start + label.chars().count();
        let segmentation = segment_words_with_modifiers_recovered(&updated);
        let mut spans = Vec::new();
        segmentation.words.iter().any(|word| {
            spans.clear();
            word.source_spans_into(&mut spans);
            spans.iter().any(|span| {
                span.byte_start == candidate_byte_start
                    && span.byte_end == candidate_byte_end
                    && span.char_start == candidate_char_start
                    && span.char_end == candidate_char_end
            })
        })
    }
}

impl CompletionContext<'_, '_, '_, '_> {
    #[requires(true)]
    #[ensures(!ret || !expectations.is_empty())]
    fn add_grammar_candidates(
        &self,
        expectations: &[SyntaxExpectation],
        items: &mut BTreeMap<(u8, String, CompletionProvenance), CompletionItem>,
    ) -> bool {
        if self.cancellation.is_cancelled() {
            return false;
        }
        if expectations.is_empty() {
            self.add_unfiltered_candidates(
                new!(SyntaxExpectationReason::ContinueCurrent {
                    construct: "word".to_owned(),
                }),
                new!(CompletionProvenance::GrammarUnavailable),
                items,
            );
            return false;
        }
        let mut tokens = BTreeMap::<SyntaxExpectedToken, SyntaxExpectationReason>::new();
        for expectation in expectations {
            if self.cancellation.is_cancelled() {
                return false;
            }
            for token in &expectation.tokens {
                match tokens.entry(token.clone()) {
                    std::collections::btree_map::Entry::Vacant(entry) => {
                        entry.insert(expectation.reason.clone());
                    }
                    std::collections::btree_map::Entry::Occupied(mut entry)
                        if expectation.reason < *entry.get() =>
                    {
                        entry.insert(expectation.reason.clone());
                    }
                    _ => {}
                }
            }
        }
        for (token, reason) in tokens {
            if self.cancellation.is_cancelled() {
                return false;
            }
            self.add_expected_token(&token, &reason, items);
        }
        true
    }

    #[requires(candidates.iter().all(|candidate| !candidate.construct.is_empty()))]
    #[ensures(true)]
    fn add_open_construct_candidates(
        &self,
        candidates: &[OpenConstructCandidate],
        items: &mut BTreeMap<(u8, String, CompletionProvenance), CompletionItem>,
    ) {
        for candidate in candidates {
            if self.cancellation.is_cancelled() {
                break;
            }
            self.add_expected_token(
                &new!(SyntaxExpectedToken::Cmavo(candidate.cmavo)),
                &new!(SyntaxExpectationReason::ContinueCurrent {
                    construct: candidate.construct.clone(),
                }),
                items,
            );
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_expected_token(
        &self,
        token: &SyntaxExpectedToken,
        reason: &SyntaxExpectationReason,
        items: &mut BTreeMap<(u8, String, CompletionProvenance), CompletionItem>,
    ) {
        if self.cancellation.is_cancelled() {
            return;
        }
        let provenance = new!(CompletionProvenance::Expected {
            token: token.clone(),
        });
        match token.as_data() {
            data!(SyntaxExpectedToken::Cmavo(cmavo)) => {
                self.add_cmavo(*cmavo, CompletionKind::Cmavo, reason, &provenance, items);
            }
            data!(SyntaxExpectedToken::Selmaho(selmaho)) => {
                self.add_selmaho(*selmaho, reason, &provenance, items);
            }
            data!(SyntaxExpectedToken::WordCategory(category)) => match category {
                SyntaxWordCategory::Brivla | SyntaxWordCategory::SelbriWord => {
                    self.add_dictionary_brivla(reason, &provenance, items);
                }
                SyntaxWordCategory::ProSumti => {
                    self.add_selmaho(Selmaho::Koha, reason, &provenance, items)
                }
                SyntaxWordCategory::LetterWord => {
                    self.add_selmaho(Selmaho::By, reason, &provenance, items)
                }
                SyntaxWordCategory::Cmevla => self.add_document_cmevla(reason, &provenance, items),
                SyntaxWordCategory::Quote => {
                    for cmavo in Cmavo::ALL
                        .iter()
                        .copied()
                        .filter(|cmavo| cmavo.is_quote_opener())
                    {
                        if self.cancellation.is_cancelled() {
                            break;
                        }
                        self.add_cmavo(cmavo, CompletionKind::Cmavo, reason, &provenance, items);
                    }
                }
            },
            data!(SyntaxExpectedToken::EndOfInput) | data!(SyntaxExpectedToken::Named(_)) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_unfiltered_candidates(
        &self,
        reason: SyntaxExpectationReason,
        provenance: CompletionProvenance,
        items: &mut BTreeMap<(u8, String, CompletionProvenance), CompletionItem>,
    ) {
        for cmavo in Cmavo::ALL {
            if self.cancellation.is_cancelled() {
                return;
            }
            self.add_cmavo(*cmavo, CompletionKind::Cmavo, &reason, &provenance, items);
        }
        self.add_dictionary_brivla(&reason, &provenance, items);
        self.add_document_cmevla(&reason, &provenance, items);
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_selmaho(
        &self,
        selmaho: Selmaho,
        reason: &SyntaxExpectationReason,
        provenance: &CompletionProvenance,
        items: &mut BTreeMap<(u8, String, CompletionProvenance), CompletionItem>,
    ) {
        let kind = match selmaho {
            Selmaho::Koha => CompletionKind::ProSumti,
            Selmaho::By => CompletionKind::LetterWord,
            _ => CompletionKind::Cmavo,
        };
        for cmavo in Cmavo::ALL
            .iter()
            .copied()
            .filter(|cmavo| selmaho.contains(*cmavo))
        {
            if self.cancellation.is_cancelled() {
                break;
            }
            self.add_cmavo(cmavo, kind, reason, provenance, items);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_cmavo(
        &self,
        cmavo: Cmavo,
        expected_kind: CompletionKind,
        reason: &SyntaxExpectationReason,
        provenance: &CompletionProvenance,
        items: &mut BTreeMap<(u8, String, CompletionProvenance), CompletionItem>,
    ) {
        let kind = if is_elidable_terminator(cmavo) {
            CompletionKind::Terminator
        } else {
            expected_kind
        };
        self.add_candidate(cmavo.canonical_text(), kind, reason, provenance, items);
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_dictionary_brivla(
        &self,
        reason: &SyntaxExpectationReason,
        provenance: &CompletionProvenance,
        items: &mut BTreeMap<(u8, String, CompletionProvenance), CompletionItem>,
    ) {
        let dictionary = self.dictionary;
        for entry in dictionary
            .entries_by_word_prefix(&self.normalized_prefix)
            .filter(|entry| is_completion_brivla_type(entry.word_type))
        {
            if self.cancellation.is_cancelled() {
                break;
            }
            self.add_candidate(
                entry.word,
                CompletionKind::Brivla,
                reason,
                provenance,
                items,
            );
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_document_cmevla(
        &self,
        reason: &SyntaxExpectationReason,
        provenance: &CompletionProvenance,
        items: &mut BTreeMap<(u8, String, CompletionProvenance), CompletionItem>,
    ) {
        for word in self.document_cmevla {
            if self.cancellation.is_cancelled() {
                break;
            }
            self.add_candidate(word, CompletionKind::Cmevla, reason, provenance, items);
        }
    }

    #[requires(!label.is_empty())]
    #[ensures(true)]
    fn add_candidate(
        &self,
        label: &str,
        kind: CompletionKind,
        reason: &SyntaxExpectationReason,
        provenance: &CompletionProvenance,
        items: &mut BTreeMap<(u8, String, CompletionProvenance), CompletionItem>,
    ) {
        if self.cancellation.is_cancelled() {
            return;
        }
        let normalized_label = normalize_lookup_query(label);
        if normalized_label.is_empty()
            || !normalized_label.starts_with(&self.normalized_prefix)
            || !self
                .snapshot
                .completion_candidate_surfaces(label, &self.replacement_span)
        {
            return;
        }

        let suffix_consistent = self.suffix_consistent_labels.contains(&normalized_label);
        let key = (
            self.interpretation.sort_rank(),
            normalized_label,
            provenance.clone(),
        );
        if let Some(existing) = items.get(&key) {
            if existing.suffix_consistent && !suffix_consistent {
                return;
            }
            if existing.suffix_consistent == suffix_consistent && existing.reason <= *reason {
                return;
            }
        }
        items.insert(
            key,
            new!(CompletionItem {
                label: label.to_owned(),
                kind,
                interpretation: self.interpretation,
                provenance: provenance.clone(),
                reason: reason.clone(),
                replacement_span: self.replacement_span.clone(),
                short_gloss: None,
                documentation: CompletionDocumentationHandle::new(label.to_owned()),
                suffix_consistent,
            }),
        );
    }
}

/// Resolve a completion documentation key into shared-renderer Markdown.
#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn completion_documentation_markdown(handle: &CompletionDocumentationHandle) -> String {
    let dictionary = jbotci_dictionary_data::english();
    let cards = dictionary
        .lookup_words(handle.word())
        .map(|entry| dictionary_entry_card(dictionary, entry, None, true))
        .collect::<Vec<_>>();
    if !cards.is_empty() {
        return render_vlacku_cards_markdown(&cards);
    }

    let analysis = analyze_valsi(handle.word());
    let word_kind = analysis
        .result
        .word
        .as_ref()
        .and_then(WordLike::bare_word)
        .map(|word| word.kind());
    match word_kind {
        Some(WordKind::Cmavo) => "**Word type:** cmavo".to_owned(),
        Some(WordKind::Gismu) => "**Word type:** gismu".to_owned(),
        Some(WordKind::Lujvo) => "**Word type:** lujvo".to_owned(),
        Some(WordKind::Fuhivla) => "**Word type:** fu'ivla".to_owned(),
        Some(WordKind::Cmevla) => "**Word type:** name word (cmevla)".to_owned(),
        None => format!("**Lojban word:** `{}`", handle.word()),
    }
}

#[requires(true)]
#[ensures(ret <= 2)]
fn reason_sort_rank(reason: &SyntaxExpectationReason) -> u8 {
    match reason.as_data() {
        data!(SyntaxExpectationReason::ContinueCurrent { .. }) => 0,
        data!(SyntaxExpectationReason::StartNested { .. }) => 1,
        data!(SyntaxExpectationReason::EndThenStart { .. }) => 2,
    }
}

#[requires(true)]
#[ensures(true)]
fn completion_sort_key(item: &CompletionItem) -> (u8, u8, u8, &str) {
    (
        item.interpretation.sort_rank(),
        u8::from(!item.suffix_consistent),
        item.reason_sort_rank(),
        item.label.as_str(),
    )
}

#[requires(true)]
#[ensures(ret.len() <= words.len())]
fn statement_local_words(words: &[WordLike], restart_byte: usize) -> &[WordLike] {
    let start = words.partition_point(|word| {
        word.byte_range()
            .is_none_or(|range| range.end <= restart_byte)
    });
    &words[start..]
}

#[requires(span.byte_start <= span.byte_end && container.byte_start <= container.byte_end)]
#[requires(span.char_start <= span.char_end && container.char_start <= container.char_end)]
#[ensures(true)]
fn span_or_cursor_is_within(span: &SourceSpan, container: &SourceSpan) -> bool {
    if span.char_start == span.char_end {
        container.char_start <= span.char_start && span.char_start < container.char_end
    } else {
        container.char_start <= span.char_start && span.char_end <= container.char_end
    }
}

#[requires(errors.len() == error_regions.len())]
#[ensures(true)]
fn segmentation_awaits_zo_target(
    errors: &[MorphologyError],
    error_regions: &[SourceSpan],
    prefix_char_end: usize,
) -> bool {
    errors.iter().zip(error_regions).any(|(error, region)| {
        region.char_end == prefix_char_end
            && matches!(
                error,
                MorphologyError::Invalid {
                    context: Some(context),
                    ..
                } if context.kind == MorphologyContextKind::QuotedWord
            )
    })
}

#[requires(true)]
#[ensures(true)]
fn is_completion_brivla_type(word_type: WordType) -> bool {
    matches!(
        word_type,
        WordType::Gismu
            | WordType::ExperimentalGismu
            | WordType::Lujvo
            | WordType::Fuivla
            | WordType::ObsoleteFuivla
    )
}

#[requires(true)]
#[ensures(true)]
fn is_elidable_terminator(cmavo: Cmavo) -> bool {
    jbotci_syntax::generated_model::GENERATED_MODEL_ELIDABLE_TERMINATORS
        .iter()
        .any(|terminator| terminator.cmavo == cmavo)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    #[invariant(true)]
    struct BoundaryCompletionCase {
        name: &'static str,
        marked_source: &'static str,
        expected_replacement: Option<&'static str>,
        expected_label: &'static str,
    }

    #[requires(source.matches('|').count() == 1)]
    #[ensures(true)]
    fn completions_at_marker(source: &str) -> Vec<CompletionItem> {
        let marker_byte = source
            .find('|')
            .expect("precondition requires a cursor marker");
        let cursor = source[..marker_byte].chars().count();
        let text = source.replacen('|', "", 1);
        DocumentSnapshot::new(text, 1).completions(cursor)
    }

    #[requires(source.matches('|').count() == 1)]
    #[requires(!grammar_time_limit.is_zero())]
    #[ensures(true)]
    fn completions_at_marker_with_grammar_time_limit(
        source: &str,
        grammar_time_limit: Duration,
    ) -> Vec<CompletionItem> {
        let marker_byte = source
            .find('|')
            .expect("precondition requires a cursor marker");
        let cursor = source[..marker_byte].chars().count();
        let text = source.replacen('|', "", 1);
        DocumentSnapshot::new(text, 1)
            .completions_with_grammar_time_limit(cursor, grammar_time_limit)
    }

    #[requires(true)]
    #[ensures(true)]
    fn has_label(items: &[CompletionItem], label: &str) -> bool {
        items.iter().any(|item| item.label == label)
    }

    #[requires(true)]
    #[ensures(true)]
    fn uses_degraded_grammar_fallback(item: &CompletionItem) -> bool {
        matches!(
            item.provenance.as_data(),
            data!(CompletionProvenance::GrammarUnavailable)
        )
    }

    #[requires(source.matches('|').count() == 1)]
    #[ensures(ret.cut_byte == source.find('|').expect("precondition requires a marker"))]
    fn tree_context_at_marker(source: &str) -> TreeCompletionContext {
        let marker_byte = source
            .find('|')
            .expect("precondition requires a cursor marker");
        let text = source.replacen('|', "", 1);
        let snapshot = DocumentSnapshot::new(text, 1);
        TreeCompletionContext::from_parse(&snapshot.parse, marker_byte, snapshot.text.len())
    }

    #[requires(true)]
    #[ensures(true)]
    fn is_statement_separator_i(item: &CompletionItem) -> bool {
        if item.label != "i" {
            return false;
        }
        let data!(CompletionProvenance::Expected { token }) = item.provenance.as_data() else {
            return false;
        };
        matches!(
            token.as_data(),
            data!(SyntaxExpectedToken::Cmavo(Cmavo::I))
                | data!(SyntaxExpectedToken::Selmaho(Selmaho::I))
        )
    }

    #[requires(!grammar_time_limit.is_zero())]
    #[requires(literal_time_limit.is_none_or(|limit| !limit.is_zero()))]
    #[ensures(true)]
    fn assert_boundary_completion_cases(
        grammar_time_limit: Duration,
        literal_time_limit: Option<Duration>,
    ) {
        let cases = [
            BoundaryCompletionCase {
                name: "word start after a space",
                marked_source: "mi klama le |zarci",
                expected_replacement: Some("zarci"),
                expected_label: "barda",
            },
            BoundaryCompletionCase {
                name: "word start after a pause period",
                marked_source: "mi tavla .|alis.",
                expected_replacement: Some("alis"),
                expected_label: "do",
            },
            BoundaryCompletionCase {
                name: "first word of a statement",
                marked_source: "mi klama .i |sruma",
                expected_replacement: Some("sruma"),
                expected_label: "sruma",
            },
            BoundaryCompletionCase {
                name: "document offset zero",
                marked_source: "|sruma",
                expected_replacement: Some("sruma"),
                expected_label: "sruma",
            },
            BoundaryCompletionCase {
                name: "document end",
                marked_source: "mi klama |",
                expected_replacement: None,
                expected_label: "i",
            },
            BoundaryCompletionCase {
                name: "owner zvati position",
                marked_source: "ne'i zo'e le mamta ku |zvati",
                expected_replacement: Some("zvati"),
                expected_label: "zvati",
            },
            BoundaryCompletionCase {
                name: "owner sruma position",
                marked_source: ".i la prux. ba'o |sruma lo du'u le ckule cipra ku frili ra",
                expected_replacement: Some("sruma"),
                expected_label: "sruma",
            },
        ];

        for case in cases {
            let marker_byte = case
                .marked_source
                .find('|')
                .expect("every boundary case contains a cursor marker");
            let cursor = case.marked_source[..marker_byte].chars().count();
            let text = case.marked_source.replacen('|', "", 1);
            let snapshot = DocumentSnapshot::new(text.clone(), 1);
            let started = Instant::now();
            let items = snapshot.completions_with_grammar_time_limit(cursor, grammar_time_limit);
            let elapsed = started.elapsed();

            assert!(
                has_label(&items, case.expected_label),
                "{} must offer {:?}: {items:#?}",
                case.name,
                case.expected_label,
            );
            let (expected_start, expected_end) = match case.expected_replacement {
                Some(replacement) => {
                    let byte_start = text
                        .find(replacement)
                        .expect("the expected replacement occurs in the boundary case");
                    (
                        text[..byte_start].chars().count(),
                        text[..byte_start + replacement.len()].chars().count(),
                    )
                }
                None => (cursor, cursor),
            };
            assert!(
                items.iter().all(|item| {
                    item.replacement_span.char_start == expected_start
                        && item.replacement_span.char_end == expected_end
                }),
                "{} must replace the current word (or insert at document end): expected {expected_start}..{expected_end}, first item {:#?}",
                case.name,
                items.first(),
            );
            if let Some(limit) = literal_time_limit {
                assert!(
                    elapsed < limit,
                    "{} completion took {elapsed:?}, limit {limit:?}",
                    case.name,
                );
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn incomplete_description_is_grammar_filtered_to_sumti_tail() {
        let items = completions_at_marker("mi klama le |");

        assert!(
            items.iter().any(|item| item.kind == CompletionKind::Brivla),
            "the description requires a brivla-capable sumti tail",
        );
        assert!(has_label(&items, "barda"));
        assert!(
            !items.iter().any(is_statement_separator_i),
            "statement I must not leak through the incomplete description: {:#?}",
            items
                .iter()
                .filter(|item| item.label == "i")
                .collect::<Vec<_>>(),
        );
        assert!(
            items
                .iter()
                .any(|item| { item.label == "i" && item.kind == CompletionKind::LetterWord }),
            "the legitimate BY interpretation of i remains available",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn complete_text_offers_sentence_continuation() {
        let items = completions_at_marker("mi klama |");
        let i = items
            .iter()
            .find(|item| is_statement_separator_i(item))
            .expect("a complete text can continue with .i");

        assert_eq!(i.interpretation, CompletionInterpretation::Continue);
        assert_eq!(i.replacement_span.char_start, i.replacement_span.char_end);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn complete_seed_offers_continue_before_extend() {
        let items = completions_at_marker("ba|");

        assert!(
            items.iter().any(|item| {
                item.label == "barda" && item.interpretation == CompletionInterpretation::Extend
            }),
            "barda candidates: {:#?}",
            items
                .iter()
                .filter(|item| item.label == "barda")
                .collect::<Vec<_>>()
        );
        assert!(
            items
                .iter()
                .any(|item| item.interpretation == CompletionInterpretation::Continue),
            "ba is also a complete cmavo, so next-word candidates are required",
        );
        assert_eq!(
            items.first().map(|item| item.interpretation),
            Some(CompletionInterpretation::Continue),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mid_word_seed_only_extends_the_current_word() {
        let items = completions_at_marker("mi klama l|o nu");

        assert!(has_label(&items, "lo"));
        assert!(
            items.iter().all(|item| {
                item.interpretation == CompletionInterpretation::Extend
                    && item.replacement_span.char_end == "mi klama lo".chars().count()
            }),
            "mid-word completion must replace the current word without a second insertion interpretation",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn earlier_recovered_error_does_not_disable_contextual_completion() {
        let items = completions_at_marker("mi ku .i do klama le |");

        assert!(has_label(&items, "barda"));
        assert!(
            !items.iter().any(is_statement_separator_i),
            "unexpected .i candidates: {:#?}",
            items
                .iter()
                .filter(|item| item.label == "i")
                .collect::<Vec<_>>(),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quoted_statement_restart_comes_from_the_nested_snapshot_tree() {
        let marked = "mi cusku lu .i ni'o mi cusku zo ni'o zo .i d|o li'u";
        let source = marked.replacen('|', "", 1);
        let actual_i = source
            .find(".i ni'o")
            .expect("fixture contains the real nested statement/paragraph separator");
        let expected_restart = DocumentSnapshot::new(source, 1)
            .word_at(actual_i + 1)
            .expect("the real separator has a morphology span")
            .span
            .byte_start;

        let context = tree_context_at_marker(marked);

        assert_eq!(context.restart_byte, expected_restart);
        assert!(
            context.restart_byte
                > marked
                    .find("lu")
                    .expect("fixture contains the nested text opener"),
            "the nested text must own its own statement restart",
        );
        assert!(
            context.restart_byte
                < marked
                    .find("zo ni'o")
                    .expect("fixture contains the quoted NIhO spelling"),
            "quoted NIhO and .i tokens must not become statement restarts",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn open_nested_construct_closers_rank_above_local_prefix_candidates() {
        let items = completions_at_marker("mi cusku lu mi djica lo nu do| ");
        let continuation = items
            .iter()
            .filter(|item| item.interpretation == CompletionInterpretation::Continue)
            .collect::<Vec<_>>();

        for closer in ["kei", "ku", "li'u"] {
            assert!(
                continuation
                    .iter()
                    .any(|item| item.label == closer && item.suffix_consistent),
                "tree covering-chain closer {closer} must be present and suffix-ranked: {continuation:#?}",
            );
        }
        let closer_index = continuation
            .iter()
            .position(|item| item.label == "kei")
            .expect("KEI closer is present");
        let local_index = continuation
            .iter()
            .position(|item| item.label == "barda")
            .expect("the local prefix parse offers a selbri word");
        assert!(closer_index < local_index);
        assert!(!continuation[local_index].suffix_consistent);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn suffix_consistency_ranks_without_filtering_transitional_edits() {
        // Replacing LO with LA leaves the following NU invalid because LA
        // requires a cmevla. The edit is nevertheless prefix-valid while the
        // user is transitioning the suffix, so it must remain available.
        let items = completions_at_marker("mi djica l|o nu do klama");
        let lo_index = items
            .iter()
            .position(|item| item.label == "lo")
            .expect("the committed-tree LO reading remains available");
        let la_index = items
            .iter()
            .position(|item| item.label == "la")
            .expect("prefix-valid LA must not be hard-filtered by the NU suffix");

        assert!(items[lo_index].suffix_consistent);
        assert!(!items[la_index].suffix_consistent);
        assert!(lo_index < la_index);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn magic_quote_contexts_suppress_or_unfilter_as_modeled_by_morphology() {
        let after_zo = completions_at_marker("zo |");
        assert!(
            has_label(&after_zo, "ku"),
            "the word quoted by zo is not grammar-filtered",
        );

        let inside_lohu = completions_at_marker("lo'u | klama le'u");
        assert!(
            has_label(&inside_lohu, "ku"),
            "words inside lo'u…le'u are not grammar-filtered",
        );

        let inside_zoi = completions_at_marker("zoi gy non|Lojban payload gy");
        assert!(
            inside_zoi.is_empty(),
            "non-Lojban delimited quote payloads have no completion",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cmevla_candidates_are_harvested_from_the_current_document() {
        let items = completions_at_marker("la .xunreblab. cu klama .i la |");
        let name = items
            .iter()
            .find(|item| item.label == "xunreblab")
            .expect("the earlier document cmevla is reusable after la");

        assert_eq!(name.kind, CompletionKind::Cmevla);
        assert_eq!(
            completion_documentation_markdown(&name.documentation),
            "**Word type:** name word (cmevla)",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_filter_excludes_candidates_that_would_fuse() {
        let separated = completions_at_marker("mi |")
            .into_iter()
            .filter(|item| item.interpretation == CompletionInterpretation::Continue)
            .map(|item| item.label.clone())
            .collect::<BTreeSet<_>>();
        let attached = completions_at_marker("mi|")
            .into_iter()
            .filter(|item| item.interpretation == CompletionInterpretation::Continue)
            .map(|item| item.label.clone())
            .collect::<BTreeSet<_>>();
        let excluded = separated.difference(&attached).collect::<Vec<_>>();

        assert!(
            !excluded.is_empty(),
            "at least one grammar-valid candidate must be rejected when it fuses with mi: {excluded:?}",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn completion_on_two_hundred_word_document_stays_interactive() {
        let text = "mi klama .i ".repeat(67);
        let snapshot = DocumentSnapshot::new(text.clone(), 1);
        let started = Instant::now();
        let items = snapshot.completions(text.chars().count());

        assert!(!items.is_empty());
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "completion unexpectedly took {:?}",
            started.elapsed(),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn empty_expectations_degrade_to_all_morphology_valid_candidate_sources() {
        let text = "la .xunreblab. cu klama .i  mukti lo nu".to_owned();
        let phrase_start = text
            .find("mukti lo nu")
            .expect("fixture contains the completion phrase");
        let cursor = phrase_start - 1;
        let snapshot = DocumentSnapshot::new(text, 1);
        let dictionary = jbotci_dictionary_data::english();
        let document_cmevla = snapshot.document_cmevla();
        let cancellation = CompletionCancellationToken::new();
        let replacement_span = SourceSpan::new(None, cursor, cursor, cursor, cursor)
            .expect("the cursor is an ordered empty span");
        let context = new!(CompletionContext {
            snapshot: &snapshot,
            dictionary,
            document_cmevla: &document_cmevla,
            cancellation: &cancellation,
            interpretation: CompletionInterpretation::Continue,
            replacement_span,
            normalized_prefix: String::new(),
            suffix_consistent_labels: BTreeSet::new(),
        });
        let mut items = BTreeMap::new();
        context.add_grammar_candidates(&[], &mut items);

        for label in ["ku", "barda", "xunreblab"] {
            assert!(
                items.values().any(|item| {
                    item.label == label
                        && matches!(
                            item.provenance.as_data(),
                            data!(CompletionProvenance::GrammarUnavailable)
                        )
                }),
                "degraded completion must include {label}",
            );
        }

        let mid_word_cursor = phrase_start + "mukti l".len();
        let seed_span = snapshot.completion_seed_span(mid_word_cursor, mid_word_cursor);
        let replacement_span = snapshot.completion_replacement_span(&seed_span);
        let context = new!(CompletionContext {
            snapshot: &snapshot,
            dictionary,
            document_cmevla: &document_cmevla,
            cancellation: &cancellation,
            interpretation: CompletionInterpretation::Extend,
            replacement_span,
            normalized_prefix: normalize_lookup_query("l"),
            suffix_consistent_labels: BTreeSet::new(),
        });
        let mut items = BTreeMap::new();
        context.add_grammar_candidates(&[], &mut items);
        let lo = items
            .values()
            .find(|item| item.label == "lo")
            .expect("degraded mid-word completion includes cmavo matching prefix l");
        assert_eq!(lo.replacement_span.char_end, mid_word_cursor + 1);
    }

    #[cfg(not(feature = "expensive_contracts"))]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn statement_local_error_heavy_completion_meets_literal_boundary() {
        let marked = format!("{}mukti l|o nu", "mi ku .i ".repeat(14));
        let marker_byte = marked.find('|').expect("fixture contains a cursor marker");
        let cursor = marked[..marker_byte].chars().count();
        let snapshot = DocumentSnapshot::new(marked.replacen('|', "", 1), 1);
        let assert_literal_boundary = std::env::var_os("CI").is_none();
        let started = Instant::now();
        let items = if assert_literal_boundary {
            snapshot.completions(cursor)
        } else {
            snapshot.completions_with_grammar_time_limit(cursor, Duration::from_secs(30))
        };
        let elapsed = started.elapsed();

        assert!(has_label(&items, "lo"));
        assert!(
            !items.iter().any(uses_degraded_grammar_fallback),
            "statement-local completion must remain grammar-filtered",
        );
        if assert_literal_boundary {
            assert!(
                elapsed < Duration::from_millis(150),
                "statement-local completion took {elapsed:?}",
            );
        }
    }

    #[cfg(feature = "expensive_contracts")]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn statement_local_error_heavy_completion_is_grammar_filtered_with_instrumentation() {
        let marked = format!("{}mukti l|o nu", "mi ku .i ".repeat(14));
        let marker_byte = marked.find('|').expect("fixture contains a cursor marker");
        let cursor = marked[..marker_byte].chars().count();
        let snapshot = DocumentSnapshot::new(marked.replacen('|', "", 1), 1);
        let items = snapshot.completions_with_grammar_time_limit(cursor, Duration::from_secs(30));

        assert!(has_label(&items, "lo"));
        assert!(
            !items.iter().any(uses_degraded_grammar_fallback),
            "instrumented statement-local completion must remain grammar-filtered",
        );
    }

    // Deep contract instrumentation changes the work measured by this literal
    // production wall-clock boundary. Its functional twin below exercises the
    // same document and provenance assertion with an instrumentation budget.
    #[cfg(not(feature = "expensive_contracts"))]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn memoized_mid_size_recovery_remains_grammar_filtered() {
        let marked = format!("mi ku .i {}mukti l|o nu", "do klama .i ".repeat(250),);
        // Shared CI hardware makes the literal one-second boundary nondeterministic, so CI runs the ample-budget semantic twin.
        let assert_literal_boundary = std::env::var_os("CI").is_none();
        let started = Instant::now();
        let items = if assert_literal_boundary {
            completions_at_marker(&marked)
        } else {
            completions_at_marker_with_grammar_time_limit(&marked, Duration::from_secs(30))
        };
        let elapsed = started.elapsed();

        assert!(
            items.iter().any(|item| {
                item.label == "lo"
                    && matches!(
                        item.provenance.as_data(),
                        data!(CompletionProvenance::Expected { .. })
                    )
            }),
            "memoized recovery should reach the cut within the selected grammar budget: {items:#?}",
        );
        if assert_literal_boundary {
            assert!(
                elapsed < Duration::from_secs(2),
                "mid-size completion unexpectedly took {elapsed:?}",
            );
        }
    }

    #[cfg(feature = "expensive_contracts")]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn memoized_mid_size_recovery_remains_grammar_filtered_with_instrumentation() {
        let marked = format!("mi ku .i {}mukti l|o nu", "do klama .i ".repeat(250),);
        let items = completions_at_marker_with_grammar_time_limit(&marked, Duration::from_secs(30));

        assert!(
            items.iter().any(|item| {
                item.label == "lo"
                    && matches!(
                        item.provenance.as_data(),
                        data!(CompletionProvenance::Expected { .. })
                    )
            }),
            "instrumented memoized recovery should reach the cut with an ample grammar budget: {items:#?}",
        );
    }

    // Deep contract instrumentation and shared CI hardware change literal
    // wall-clock behavior. CI and the expensive-contract build run the
    // functional twin below with an ample grammar budget; local production
    // builds additionally enforce the interactive latency boundary per case.
    #[cfg(not(feature = "expensive_contracts"))]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn completion_boundary_positions_remain_interactive() {
        let assert_literal_boundary = std::env::var_os("CI").is_none();
        let grammar_time_limit = if assert_literal_boundary {
            COMPLETION_GRAMMAR_TIME_LIMIT
        } else {
            Duration::from_secs(30)
        };
        assert_boundary_completion_cases(
            grammar_time_limit,
            assert_literal_boundary.then_some(Duration::from_secs(2)),
        );
    }

    #[cfg(feature = "expensive_contracts")]
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn completion_boundary_positions_remain_correct_with_instrumentation() {
        assert_boundary_completion_cases(Duration::from_secs(30), None);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cancelled_completion_stops_before_boundary_candidate_expansion() {
        let text = ".i la prux. ba'o sruma lo du'u le ckule cipra ku frili ra";
        let cursor = text.find("sruma").expect("fixture contains sruma");
        let snapshot = DocumentSnapshot::new(text.to_owned(), 1);
        let cancellation = CompletionCancellationToken::new();
        cancellation.cancel();

        let started = Instant::now();
        let items = snapshot.completions_cancellable(cursor, &cancellation);

        assert!(items.is_empty());
        assert!(
            started.elapsed() < Duration::from_millis(50),
            "pre-cancelled completion must not enter candidate expansion",
        );
    }
}
