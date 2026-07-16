use std::collections::{BTreeMap, BTreeSet};

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
    SyntaxExpectedToken, SyntaxExpectedTokenData, SyntaxWordCategory, expected_continuations,
};

use super::{DocumentSnapshot, SemanticTokenKind};

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
#[invariant(::Expected { .. } => true)]
#[invariant(::UnfilteredQuote => true)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompletionProvenance {
    Expected { token: SyntaxExpectedToken },
    UnfilteredQuote,
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

// This is a short-lived mutable accumulator. Its referenced snapshot and
// source span are already validated, and candidate insertion preserves the
// map's key/value relationship locally.
#[invariant(true)]
struct CompletionBuilder<'snapshot, 'dictionary, 'entries> {
    snapshot: &'snapshot DocumentSnapshot,
    dictionary: &'dictionary Dictionary<'entries>,
    document_cmevla: &'snapshot BTreeSet<String>,
    interpretation: CompletionInterpretation,
    replacement_span: SourceSpan,
    normalized_prefix: String,
    items: &'snapshot mut BTreeMap<(u8, String, CompletionProvenance), CompletionItem>,
}

impl DocumentSnapshot {
    /// Return grammar- and morphology-filtered completions at `char_offset`.
    ///
    /// The cursor is clamped through the snapshot's line index. Non-empty seeds
    /// are interpreted both as an incomplete word and, when they segment
    /// cleanly, as complete words before a new insertion point.
    #[requires(true)]
    #[ensures(ret.windows(2).all(|items| completion_sort_key(&items[0]) <= completion_sort_key(&items[1])))]
    pub fn completions(&self, char_offset: usize) -> Vec<CompletionItem> {
        let cursor = self.line_index.offsets_for_char(char_offset);
        let seed_span = self.completion_seed_span(cursor.byte, cursor.char);
        let seed = &self.text[seed_span.byte_start..seed_span.byte_end];
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
                seed_span.clone(),
                seed,
                &preceding_words,
                preceding_awaits_zo_target,
                &mut items,
            );
        }

        let seed_is_complete = seed.is_empty() || segment_words_with_modifiers(seed).is_ok();
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
            let insertion_span =
                SourceSpan::new(None, cursor.byte, cursor.byte, cursor.char, cursor.char)
                    .expect("a cursor position is an ordered empty source span");
            self.add_completion_interpretation(
                dictionary,
                &document_cmevla,
                CompletionInterpretation::Continue,
                insertion_span,
                "",
                continuation_words,
                continuation_awaits_zo_target,
                &mut items,
            );
        }

        let mut result = items.into_values().collect::<Vec<_>>();
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

    #[allow(clippy::too_many_arguments)]
    #[requires(replacement_span.byte_end <= self.text.len())]
    #[requires(replacement_span.char_end <= self.line_index.char_len())]
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
        items: &mut BTreeMap<(u8, String, CompletionProvenance), CompletionItem>,
    ) {
        let context = match self.cursor_completion_context(&replacement_span) {
            CursorCompletionContext::Grammar if preceding_awaits_zo_target => {
                CursorCompletionContext::UnfilteredQuotedWord
            }
            context => context,
        };
        let mut builder = CompletionBuilder {
            snapshot: self,
            dictionary,
            document_cmevla,
            interpretation,
            replacement_span,
            normalized_prefix: normalize_lookup_query(prefix),
            items,
        };
        match context {
            CursorCompletionContext::SuppressedNonLojbanQuote => {}
            CursorCompletionContext::UnfilteredQuotedWord => {
                builder.add_unfiltered_candidates(new!(SyntaxExpectationReason::ContinueCurrent {
                    construct: "quoted word".to_owned(),
                }))
            }
            CursorCompletionContext::UnfilteredQuotedWords => {
                builder.add_unfiltered_candidates(new!(SyntaxExpectationReason::ContinueCurrent {
                    construct: "LOhU quote".to_owned(),
                }))
            }
            CursorCompletionContext::Grammar => {
                let expectations =
                    expected_continuations(preceding_words, &ParseOptions::default());
                builder.add_expected_candidates(&expectations);
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

impl CompletionBuilder<'_, '_, '_> {
    #[requires(true)]
    #[ensures(true)]
    fn add_expected_candidates(&mut self, expectations: &[SyntaxExpectation]) {
        let mut tokens = BTreeMap::<SyntaxExpectedToken, SyntaxExpectationReason>::new();
        for expectation in expectations {
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
            self.add_expected_token(&token, &reason);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_expected_token(
        &mut self,
        token: &SyntaxExpectedToken,
        reason: &SyntaxExpectationReason,
    ) {
        let provenance = CompletionProvenance::Expected {
            token: token.clone(),
        };
        match token.as_data() {
            data!(SyntaxExpectedToken::Cmavo(cmavo)) => {
                self.add_cmavo(*cmavo, CompletionKind::Cmavo, reason, &provenance);
            }
            data!(SyntaxExpectedToken::Selmaho(selmaho)) => {
                self.add_selmaho(*selmaho, reason, &provenance);
            }
            data!(SyntaxExpectedToken::WordCategory(category)) => match category {
                SyntaxWordCategory::Brivla | SyntaxWordCategory::SelbriWord => {
                    self.add_dictionary_brivla(reason, &provenance);
                }
                SyntaxWordCategory::ProSumti => {
                    self.add_selmaho(Selmaho::Koha, reason, &provenance)
                }
                SyntaxWordCategory::LetterWord => {
                    self.add_selmaho(Selmaho::By, reason, &provenance)
                }
                SyntaxWordCategory::Cmevla => self.add_document_cmevla(reason, &provenance),
                SyntaxWordCategory::Quote => {
                    for cmavo in Cmavo::ALL
                        .iter()
                        .copied()
                        .filter(|cmavo| cmavo.is_quote_opener())
                    {
                        self.add_cmavo(cmavo, CompletionKind::Cmavo, reason, &provenance);
                    }
                }
            },
            data!(SyntaxExpectedToken::EndOfInput) | data!(SyntaxExpectedToken::Named(_)) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_unfiltered_candidates(&mut self, reason: SyntaxExpectationReason) {
        let provenance = CompletionProvenance::UnfilteredQuote;
        for cmavo in Cmavo::ALL {
            self.add_cmavo(*cmavo, CompletionKind::Cmavo, &reason, &provenance);
        }
        self.add_dictionary_brivla(&reason, &provenance);
        self.add_document_cmevla(&reason, &provenance);
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_selmaho(
        &mut self,
        selmaho: Selmaho,
        reason: &SyntaxExpectationReason,
        provenance: &CompletionProvenance,
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
            self.add_cmavo(cmavo, kind, reason, provenance);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_cmavo(
        &mut self,
        cmavo: Cmavo,
        expected_kind: CompletionKind,
        reason: &SyntaxExpectationReason,
        provenance: &CompletionProvenance,
    ) {
        let kind = if is_elidable_terminator(cmavo) {
            CompletionKind::Terminator
        } else {
            expected_kind
        };
        self.add_candidate(cmavo.canonical_text(), kind, reason, provenance);
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_dictionary_brivla(
        &mut self,
        reason: &SyntaxExpectationReason,
        provenance: &CompletionProvenance,
    ) {
        let dictionary = self.dictionary;
        for entry in dictionary
            .entries_by_word_prefix(&self.normalized_prefix)
            .filter(|entry| is_completion_brivla_type(entry.word_type))
        {
            self.add_candidate(entry.word, CompletionKind::Brivla, reason, provenance);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_document_cmevla(
        &mut self,
        reason: &SyntaxExpectationReason,
        provenance: &CompletionProvenance,
    ) {
        for word in self.document_cmevla {
            self.add_candidate(word, CompletionKind::Cmevla, reason, provenance);
        }
    }

    #[requires(!label.is_empty())]
    #[ensures(true)]
    fn add_candidate(
        &mut self,
        label: &str,
        kind: CompletionKind,
        reason: &SyntaxExpectationReason,
        provenance: &CompletionProvenance,
    ) {
        let normalized_label = normalize_lookup_query(label);
        if normalized_label.is_empty()
            || !normalized_label.starts_with(&self.normalized_prefix)
            || !self
                .snapshot
                .completion_candidate_surfaces(label, &self.replacement_span)
        {
            return;
        }

        let key = (
            self.interpretation.sort_rank(),
            normalized_label,
            provenance.clone(),
        );
        if self
            .items
            .get(&key)
            .is_some_and(|existing| existing.reason <= *reason)
        {
            return;
        }
        let short_gloss = self
            .dictionary
            .lookup_words(label)
            .flat_map(|entry| entry.gloss_keywords)
            .map(|keyword| keyword.word)
            .find(|gloss| !gloss.is_empty())
            .map(str::to_owned);
        self.items.insert(
            key,
            new!(CompletionItem {
                label: label.to_owned(),
                kind,
                interpretation: self.interpretation,
                provenance: provenance.clone(),
                reason: reason.clone(),
                replacement_span: self.replacement_span.clone(),
                short_gloss,
                documentation: CompletionDocumentationHandle::new(label.to_owned()),
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
fn completion_sort_key(item: &CompletionItem) -> (u8, u8, &str) {
    (
        item.interpretation.sort_rank(),
        item.reason_sort_rank(),
        item.label.as_str(),
    )
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

    #[requires(true)]
    #[ensures(true)]
    fn has_label(items: &[CompletionItem], label: &str) -> bool {
        items.iter().any(|item| item.label == label)
    }

    #[requires(true)]
    #[ensures(true)]
    fn is_statement_separator_i(item: &CompletionItem) -> bool {
        if item.label != "i" {
            return false;
        }
        let CompletionProvenance::Expected { token } = &item.provenance else {
            return false;
        };
        matches!(
            token.as_data(),
            data!(SyntaxExpectedToken::Cmavo(Cmavo::I))
                | data!(SyntaxExpectedToken::Selmaho(Selmaho::I))
        )
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
}
