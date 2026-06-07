use bityzba::{data, ensures, invariant, new, requires};
use jbotci_diagnostics::{TraceEventKind, TraceLevel, TracePhase, TraceRecorder};
use jbotci_source::{SourceId, SourceSpan};

use crate::{
    Cmavo, ExpectedWordDetailKind, MorphologyContext, MorphologyContextKind, MorphologyError,
    MorphologyErrorDetail, MorphologyErrorDetailData, MorphologyErrorKind, MorphologyOptions,
    MorphologySegmentAttempt, MorphologyWarning, MorphologyWarningKind, Phonemes, Verbatim, Word,
    WordKind, WordLike, WordLikeData, ZoiDelimiterDetailKind, canonical_text_eq,
    canonical_text_is_all, canonicalize_text, erasure_selmaho,
};

#[requires(true)]
#[ensures(true)]
pub(crate) fn segment_words_with_modifiers(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_attempt(input, options, source_id)
        .into_data()
        .result
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn segment_words_with_modifiers_attempt(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> MorphologySegmentAttempt {
    segment_words_with_modifiers_raw_attempt(input, options, source_id)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn segment_words_with_modifiers_raw(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_raw_attempt(input, options, source_id)
        .into_data()
        .result
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn segment_words_with_modifiers_raw_attempt(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> MorphologySegmentAttempt {
    let segmenter = Segmenter::new(input, options, source_id);
    segmenter.segment_raw_attempt()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn segment_words_for_display(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_for_display_attempt(input, options, source_id)
        .into_data()
        .result
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn segment_words_for_display_attempt(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> MorphologySegmentAttempt {
    let segmenter = Segmenter::new(input, options, source_id);
    segmenter.segment_display_attempt()
}

#[derive(Debug, Clone, Copy)]
#[invariant(true)]
struct SourceChar {
    byte_offset: usize,
    value: char,
}

#[derive(Debug)]
#[invariant(true)]
struct Segmenter<'a> {
    input: &'a str,
    options: &'a MorphologyOptions,
    source_id: Option<SourceId>,
    chars: Vec<SourceChar>,
    index: usize,
    warnings: Vec<MorphologyWarning>,
    trace: TraceRecorder,
}

impl<'a> Segmenter<'a> {
    #[ensures(ret.index == 0)]
    #[ensures(ret.chars.len() == input.chars().count())]
    #[requires(true)]
    fn new(input: &'a str, options: &'a MorphologyOptions, source_id: Option<SourceId>) -> Self {
        Self {
            input,
            options,
            source_id,
            chars: input
                .char_indices()
                .map(|(byte_offset, value)| SourceChar { byte_offset, value })
                .collect(),
            index: 0,
            warnings: Vec::new(),
            trace: TraceRecorder::new(options.trace.clone(), TracePhase::Morphology),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn segment_raw_attempt(mut self) -> MorphologySegmentAttempt {
        self.trace_step(TraceLevel::Top, "morphology", 0, 0, || None);
        let result = self.segment_raw();
        let trace = self.trace.finish();
        new!(MorphologySegmentAttempt {
            result,
            warnings: self.warnings,
            trace,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn segment_display_attempt(mut self) -> MorphologySegmentAttempt {
        self.trace_step(TraceLevel::Top, "morphology display", 0, 0, || None);
        let result = self.segment_display();
        let trace = self.trace.finish();
        new!(MorphologySegmentAttempt {
            result,
            warnings: self.warnings,
            trace,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn segment_raw(&mut self) -> Result<Vec<WordLike>, MorphologyError> {
        let mut acc = Vec::new();
        while self.skip_magic_noise(true)? {
            if self.index == self.chars.len() {
                break;
            }
            let segment = self.next_segment()?;
            self.process_segment(&mut acc, segment)?;
        }
        Ok(acc)
    }

    #[requires(true)]
    #[ensures(true)]
    fn segment_display(&mut self) -> Result<Vec<WordLike>, MorphologyError> {
        let mut acc = Vec::new();
        loop {
            self.skip_separators();
            if self.index == self.chars.len() {
                break;
            }
            acc.extend(self.next_display_segment()?);
        }
        Ok(acc)
    }

    #[requires(start <= end)]
    #[ensures(true)]
    fn trace_step(
        &mut self,
        level: TraceLevel,
        label: &str,
        start: usize,
        end: usize,
        detail: impl FnOnce() -> Option<String>,
    ) {
        let byte_start = self.byte_offset(start);
        let byte_end = self.byte_offset(end);
        self.trace.record_with_detail(
            level,
            TraceEventKind::MorphologyStep,
            label,
            byte_start,
            byte_end,
            detail,
        );
    }

    #[requires(start <= end)]
    #[ensures(true)]
    fn trace_failure(
        &mut self,
        label: &str,
        start: usize,
        end: usize,
        detail: impl FnOnce() -> Option<String>,
    ) {
        let byte_start = self.byte_offset(start);
        let byte_end = self.byte_offset(end);
        self.trace.record_with_detail(
            TraceLevel::Top,
            TraceEventKind::MorphologyFailure,
            label,
            byte_start,
            byte_end,
            detail,
        );
    }

    #[requires(start <= end)]
    #[ensures(true)]
    fn trace_slice_detail(
        &self,
        level: TraceLevel,
        label: &str,
        start: usize,
        end: usize,
    ) -> Option<String> {
        if self.trace.should_record(level, label) {
            Some(self.slice(start, end).to_owned())
        } else {
            None
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_segment(&mut self) -> Result<Vec<WordLike>, MorphologyError> {
        self.skip_separators();
        let segment_start = self.index;
        self.trace_step(
            TraceLevel::Detailed,
            "segment",
            segment_start,
            segment_start,
            || None,
        );
        if self.peek_char().is_some_and(|value| value.is_ascii_digit()) {
            let candidate_end = self.candidate_end(self.index);
            if self.is_digit_sequence_candidate(self.index, candidate_end) {
                let detail = self.trace_slice_detail(
                    TraceLevel::Detailed,
                    "digit sequence",
                    self.index,
                    candidate_end,
                );
                self.trace_step(
                    TraceLevel::Detailed,
                    "digit sequence",
                    self.index,
                    candidate_end,
                    move || detail,
                );
                return self.digit_sequence();
            }
        }
        let start = self.index;
        let word = self.next_plain_word()?;
        if is_simple_cmavo_text(&word, "lo'u") {
            self.trace_step(
                TraceLevel::Detailed,
                "LOhU quote",
                start,
                self.index,
                || None,
            );
            return self.lohu_quote(word);
        }
        if is_simple_cmavo_text(&word, "zoi")
            || is_simple_cmavo_text(&word, "la'o")
            || is_simple_cmavo_text(&word, "mu'oi")
        {
            self.trace_step(TraceLevel::Detailed, "ZOI quote", start, self.index, || {
                None
            });
            return self.zoi_quote(word);
        }
        if is_simple_cmavo_text(&word, "zo'oi")
            || is_simple_cmavo_text(&word, "la'oi")
            || is_simple_cmavo_text(&word, "ra'oi")
            || is_simple_cmavo_text(&word, "me'oi")
            || is_simple_cmavo_text(&word, "go'oi")
        {
            self.trace_step(
                TraceLevel::Detailed,
                "single-word quote",
                start,
                self.index,
                || None,
            );
            return self.single_word_quote(word);
        }
        if is_simple_cmavo_text(&word, "zo") || is_simple_cmavo_text(&word, "ma'oi") {
            self.trace_step(TraceLevel::Detailed, "ZO quote", start, self.index, || None);
            return self.zo_quote(word);
        }
        if is_simple_cmavo_text(&word, "fa'o") {
            self.trace_step(TraceLevel::Detailed, "FAhO", start, self.index, || None);
            self.index = self.chars.len();
            return Ok(vec![word]);
        }
        if self.index == start {
            return Err(self.invalid_span(
                MorphologyErrorKind::UnrecognizedWord,
                start,
                start,
                None,
            ));
        }
        Ok(vec![word])
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_display_segment(&mut self) -> Result<Vec<WordLike>, MorphologyError> {
        self.skip_separators();
        let segment_start = self.index;
        self.trace_step(
            TraceLevel::Detailed,
            "display segment",
            segment_start,
            segment_start,
            || None,
        );
        if self.peek_char().is_some_and(|value| value.is_ascii_digit()) {
            let candidate_end = self.candidate_end(self.index);
            if self.is_digit_sequence_candidate(self.index, candidate_end) {
                let detail = self.trace_slice_detail(
                    TraceLevel::Detailed,
                    "digit sequence",
                    self.index,
                    candidate_end,
                );
                self.trace_step(
                    TraceLevel::Detailed,
                    "digit sequence",
                    self.index,
                    candidate_end,
                    move || detail,
                );
                return self.digit_sequence();
            }
        }
        let start = self.index;
        let word = self.next_plain_word()?;
        if is_simple_cmavo_text(&word, "lo'u") {
            self.trace_step(
                TraceLevel::Detailed,
                "LOhU quote",
                start,
                self.index,
                || None,
            );
            return self.lohu_quote(word);
        }
        if is_simple_cmavo_text(&word, "zoi")
            || is_simple_cmavo_text(&word, "la'o")
            || is_simple_cmavo_text(&word, "mu'oi")
        {
            self.trace_step(TraceLevel::Detailed, "ZOI quote", start, self.index, || {
                None
            });
            return self.zoi_quote(word);
        }
        if is_simple_cmavo_text(&word, "zo'oi")
            || is_simple_cmavo_text(&word, "la'oi")
            || is_simple_cmavo_text(&word, "ra'oi")
            || is_simple_cmavo_text(&word, "me'oi")
            || is_simple_cmavo_text(&word, "go'oi")
        {
            self.trace_step(
                TraceLevel::Detailed,
                "single-word quote",
                start,
                self.index,
                || None,
            );
            return self.single_word_quote(word);
        }
        if is_simple_cmavo_text(&word, "zo") || is_simple_cmavo_text(&word, "ma'oi") {
            self.trace_step(TraceLevel::Detailed, "ZO quote", start, self.index, || None);
            return self.zo_quote(word);
        }
        if self.index == start {
            return Err(self.invalid_span(
                MorphologyErrorKind::UnrecognizedWord,
                start,
                start,
                None,
            ));
        }
        Ok(vec![word])
    }

    #[requires(true)]
    #[ensures(true)]
    fn process_segment(
        &mut self,
        acc: &mut Vec<WordLike>,
        segment: Vec<WordLike>,
    ) -> Result<(), MorphologyError> {
        if segment.len() != 1 {
            for word in segment {
                acc.push(word);
            }
            return Ok(());
        }
        let token = segment.into_iter().next().expect("length checked");
        self.process_token(acc, token)
    }

    #[requires(true)]
    #[ensures(true)]
    fn process_token(
        &mut self,
        acc: &mut Vec<WordLike>,
        token: WordLike,
    ) -> Result<(), MorphologyError> {
        if is_simple_cmavo_text(&token, "bu") {
            self.trace_step(
                TraceLevel::Detailed,
                "BU attachment",
                self.index,
                self.index,
                || None,
            );
            return self.handle_bu(acc, token);
        }
        if is_simple_cmavo_text(&token, "si") {
            self.trace_step(
                TraceLevel::Detailed,
                "SI erasure",
                self.index,
                self.index,
                || None,
            );
            self.handle_si(acc);
            return Ok(());
        }
        if is_simple_cmavo_text(&token, "fa'o") {
            return Ok(());
        }
        if is_simple_cmavo_text(&token, "sa") {
            self.trace_step(
                TraceLevel::Detailed,
                "SA erasure",
                self.index,
                self.index,
                || None,
            );
            return self.handle_sa(acc);
        }
        if is_simple_cmavo_text(&token, "su") {
            self.trace_step(
                TraceLevel::Detailed,
                "SU erasure",
                self.index,
                self.index,
                || None,
            );
            self.handle_su(acc);
            return Ok(());
        }
        if is_simple_cmavo_text(&token, "zei") {
            self.trace_step(
                TraceLevel::Detailed,
                "ZEI lujvo",
                self.index,
                self.index,
                || None,
            );
            return self.handle_zei(acc, token);
        }
        acc.push(token);
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_plain_word(&mut self) -> Result<WordLike, MorphologyError> {
        self.skip_separators();
        let start = self.index;
        let candidate_end = self.candidate_end(start);
        if start == candidate_end {
            return Err(self.invalid_span(MorphologyErrorKind::ExpectedWord, start, start, None));
        }
        if let Some(candidate) = self.streaming_word_candidate(start, candidate_end) {
            let data!(StreamingWordCandidate {
                end,
                kind,
                phonemes
            }) = candidate.into_data();
            let raw = self.slice(start, end);
            self.index = end;
            self.trace_step(
                TraceLevel::Top,
                word_kind_trace_label(kind),
                start,
                end,
                || Some(raw.to_owned()),
            );
            return self.word_with_modifiers(start, end, kind, phonemes);
        }

        let error_end = self.trim_trailing_commas(start, candidate_end);
        if start == error_end {
            return Err(self.invalid_span(MorphologyErrorKind::ExpectedWord, start, start, None));
        }
        let raw = self.slice(start, error_end);
        if let Some((invalid_index, invalid_char)) =
            self.first_invalid_word_char(start, candidate_end)
        {
            self.trace_failure("word", invalid_index, invalid_index + 1, || {
                Some(format!("unsupported character `{invalid_char}`"))
            });
            return Err(self.invalid_span_with_detail(
                MorphologyErrorKind::InvalidCharacter,
                invalid_index,
                invalid_index + 1,
                None,
                crate::phonotactic_error_detail(MorphologyErrorKind::InvalidCharacter),
            ));
        }
        let normalized = crate::segment::normalize_word_with_options(raw, self.options);
        if normalized.is_empty() {
            self.trace_failure("word", start, error_end, || {
                Some("no valid morphology characters".to_owned())
            });
            return Err(self.invalid_span(
                MorphologyErrorKind::UnrecognizedWord,
                start,
                error_end,
                None,
            ));
        }
        let error = self.invalid_word_error(start, error_end);
        self.trace_failure("word", start, error_end, || Some(error_message(&error)));
        Err(error)
    }

    #[requires(start < candidate_end && candidate_end <= self.chars.len())]
    #[ensures(ret.as_ref().is_none_or(|candidate| candidate.end > start && candidate.end <= candidate_end && !candidate.phonemes.is_empty()))]
    fn streaming_word_candidate(
        &self,
        start: usize,
        candidate_end: usize,
    ) -> Option<StreamingWordCandidate> {
        self.streaming_brivla_candidate(start, candidate_end)
            .or_else(|| self.streaming_cmevla_candidate(start, candidate_end))
            .or_else(|| self.streaming_cmavo_candidate(start, candidate_end))
    }

    #[requires(start < candidate_end && candidate_end <= self.chars.len())]
    #[ensures(ret.as_ref().is_none_or(|candidate| candidate.end > start && candidate.end <= candidate_end && !candidate.phonemes.is_empty()))]
    fn streaming_brivla_candidate(
        &self,
        start: usize,
        candidate_end: usize,
    ) -> Option<StreamingWordCandidate> {
        ((start + 1)..=candidate_end).find_map(|end| {
            if !self.post_word_ok_for_brivla(start, end, candidate_end) {
                return None;
            }
            let normalized = self.checked_normalized_slice(start, end)?;
            let (kind, phonemes) =
                crate::segment::classify_word_with_options(&normalized, self.options)?;
            if !matches!(kind, WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla) {
                return None;
            }
            if self.has_blocking_cmavo_prefix(start, end) {
                return None;
            }
            Some(new!(StreamingWordCandidate {
                end: end,
                kind: kind,
                phonemes: phonemes,
            }))
        })
    }

    #[requires(start < end && end <= candidate_end && candidate_end <= self.chars.len())]
    #[ensures(true)]
    fn post_word_ok_for_brivla(&self, start: usize, end: usize, candidate_end: usize) -> bool {
        let Some(normalized) = self.checked_normalized_slice(start, end) else {
            return false;
        };
        if has_explicit_brivla_stress(&normalized) {
            explicit_brivla_stress_is_valid(&normalized)
                && self.brivla_boundary_ok(start, end, candidate_end)
        } else {
            self.pause_at(end)
        }
    }

    #[requires(start <= end && end <= candidate_end && candidate_end <= self.chars.len())]
    #[ensures(true)]
    fn brivla_boundary_ok(&self, start: usize, end: usize, candidate_end: usize) -> bool {
        if self.pause_at(end) {
            return true;
        }
        let Some(prefix) = self.checked_normalized_slice(start, end) else {
            return false;
        };
        let Some(remainder) = self.checked_normalized_slice(end, candidate_end) else {
            return false;
        };
        if boundary_repeats_diphthong_semivowel(&prefix, &remainder) {
            return false;
        }
        self.post_word_at(end)
    }

    #[requires(start < candidate_end && candidate_end <= self.chars.len())]
    #[ensures(ret.as_ref().is_none_or(|candidate| candidate.end > start && candidate.end <= candidate_end && !candidate.phonemes.is_empty()))]
    fn streaming_cmevla_candidate(
        &self,
        start: usize,
        candidate_end: usize,
    ) -> Option<StreamingWordCandidate> {
        ((start + 1)..=candidate_end).find_map(|end| {
            if !self.pause_at(end) {
                return None;
            }
            let normalized = self.checked_normalized_slice(start, end)?;
            if !crate::segment::is_cmevla_with_options(&normalized, self.options) {
                return None;
            }
            Some(new!(StreamingWordCandidate {
                end: end,
                kind: WordKind::Cmevla,
                phonemes: crate::segment::canonicalize_word_phonemes(&normalized),
            }))
        })
    }

    #[requires(start < candidate_end && candidate_end <= self.chars.len())]
    #[ensures(ret.as_ref().is_none_or(|candidate| candidate.end > start && candidate.end <= candidate_end && !candidate.phonemes.is_empty()))]
    fn streaming_cmavo_candidate(
        &self,
        start: usize,
        candidate_end: usize,
    ) -> Option<StreamingWordCandidate> {
        let full_candidate = self.checked_normalized_slice(start, candidate_end)?;
        if full_candidate
            .chars()
            .all(|value| matches!(value, 'y' | 'ý'))
            && let Some(phonemes) = crate::segment::parse_cmavo_form(&full_candidate)
        {
            return Some(new!(StreamingWordCandidate {
                end: candidate_end,
                kind: WordKind::Cmavo,
                phonemes: phonemes,
            }));
        }

        ((start + 1)..=candidate_end).find_map(|end| {
            let phonemes =
                crate::segment::parse_cmavo_form(&self.checked_normalized_slice(start, end)?)?;
            if !self.cmavo_boundary_ok(start, end, candidate_end) {
                return None;
            }
            Some(new!(StreamingWordCandidate {
                end: end,
                kind: WordKind::Cmavo,
                phonemes: phonemes,
            }))
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn zo_quote(
        &mut self,
        zo_word_with_modifiers: WordLike,
    ) -> Result<Vec<WordLike>, MorphologyError> {
        let after_marker = self.index;
        self.skip_y_words();
        let quote_context =
            word_like_context(&zo_word_with_modifiers, MorphologyContextKind::QuotedWord);
        let quoted = match self.next_plain_non_y_word() {
            Ok(quoted) => quoted,
            Err(error) if is_expected_word_error(&error) => {
                return Err(self.invalid_span_with_detail(
                    MorphologyErrorKind::ExpectedWord,
                    after_marker,
                    after_marker,
                    quote_context,
                    Some(new!(MorphologyErrorDetail::ExpectedWord {
                        expected: ExpectedWordDetailKind::QuoteTarget,
                    })),
                ));
            }
            Err(error) => return Err(error_with_fallback_context(error, quote_context)),
        };
        let zo = into_bare_word(zo_word_with_modifiers).ok_or_else(|| {
            self.invalid_span(
                MorphologyErrorKind::InvalidQuoteMarker,
                after_marker,
                after_marker,
                quote_context,
            )
        })?;
        let quoted_context = word_like_context(&quoted, MorphologyContextKind::QuotedWord);
        let word = into_bare_word(quoted).ok_or_else(|| {
            self.invalid_span_with_detail(
                MorphologyErrorKind::ExpectedWord,
                after_marker,
                self.index,
                quoted_context,
                Some(new!(MorphologyErrorDetail::ExpectedWord {
                    expected: ExpectedWordDetailKind::QuoteTarget,
                })),
            )
        })?;
        Ok(vec![base_word_like(WordLike::zo_quote(zo, word))])
    }

    #[requires(true)]
    #[ensures(true)]
    fn zoi_quote(
        &mut self,
        zoi_word_with_modifiers: WordLike,
    ) -> Result<Vec<WordLike>, MorphologyError> {
        let after_marker = self.index;
        self.skip_separators();
        let quote_context = word_like_context(
            &zoi_word_with_modifiers,
            MorphologyContextKind::DelimitedNonLojbanQuote,
        );
        let opening_word_with_modifiers = match self.next_plain_word() {
            Ok(opening_word_with_modifiers) => opening_word_with_modifiers,
            Err(error) if is_expected_word_error(&error) => {
                return Err(self.invalid_span_with_detail(
                    MorphologyErrorKind::InvalidZoiDelimiter,
                    after_marker,
                    after_marker,
                    quote_context,
                    Some(new!(MorphologyErrorDetail::InvalidZoiDelimiter {
                        reason: ZoiDelimiterDetailKind::Missing,
                    })),
                ));
            }
            Err(error) => return Err(error_with_fallback_context(error, quote_context)),
        };
        if bare_word_ref(&zoi_word_with_modifiers).is_none() {
            return Err(self.invalid_span(
                MorphologyErrorKind::InvalidQuoteMarker,
                after_marker,
                after_marker,
                quote_context,
            ));
        }
        let delimiter_context = word_like_context(
            &opening_word_with_modifiers,
            MorphologyContextKind::DelimitedNonLojbanQuote,
        );
        let opening_delimiter = into_bare_word(opening_word_with_modifiers).ok_or_else(|| {
            self.invalid_span_with_detail(
                MorphologyErrorKind::InvalidZoiDelimiter,
                after_marker,
                self.index,
                delimiter_context,
                Some(new!(MorphologyErrorDetail::InvalidZoiDelimiter {
                    reason: ZoiDelimiterDetailKind::NotSingleWord,
                })),
            )
        })?;
        if is_y_word_text(opening_delimiter.phonemes().as_str()) {
            return Err(self.invalid_span_with_detail(
                MorphologyErrorKind::InvalidZoiDelimiter,
                opening_delimiter.span().char_start,
                opening_delimiter.span().char_end,
                self.context(
                    MorphologyContextKind::DelimitedNonLojbanQuote,
                    after_marker,
                    self.index,
                ),
                Some(new!(MorphologyErrorDetail::InvalidZoiDelimiter {
                    reason: ZoiDelimiterDetailKind::YWord,
                })),
            ));
        }
        let consumed_open_separator = self.consume_zoi_open_separators();
        let quoted_start = self.index;
        let Some((quoted_end, closing_delimiter, close_start)) =
            self.find_zoi_close(&opening_delimiter, consumed_open_separator)?
        else {
            return Err(MorphologyError::UnterminatedZoiQuote {
                char_offset: quoted_start,
                delimiter: opening_delimiter.phonemes().into_string(),
                context: self.context(
                    MorphologyContextKind::DelimitedNonLojbanQuote,
                    after_marker,
                    self.index,
                ),
            });
        };
        self.index = close_start;
        let closing = self.next_plain_word()?;
        let zoi =
            into_bare_word(zoi_word_with_modifiers).expect("ZOI marker was checked as a bare word");
        let closing_delimiter = into_bare_word(closing).unwrap_or(closing_delimiter);
        Ok(vec![base_word_like(WordLike::zoi_quote(
            zoi,
            opening_delimiter,
            self.verbatim(quoted_start, quoted_end)?,
            closing_delimiter,
        ))])
    }

    #[requires(true)]
    #[ensures(true)]
    fn single_word_quote(
        &mut self,
        marker_word_with_modifiers: WordLike,
    ) -> Result<Vec<WordLike>, MorphologyError> {
        self.skip_separators();
        let start = self.index;
        let end = self.candidate_end(start);
        if start == end {
            return Err(self.invalid_span(
                MorphologyErrorKind::ExpectedWord,
                start,
                start,
                word_like_context(
                    &marker_word_with_modifiers,
                    MorphologyContextKind::DelimitedWordQuote,
                ),
            ));
        }
        self.index = end;
        let marker_context = word_like_context(
            &marker_word_with_modifiers,
            MorphologyContextKind::DelimitedWordQuote,
        );
        let marker = into_bare_word(marker_word_with_modifiers).ok_or_else(|| {
            self.invalid_span(
                MorphologyErrorKind::InvalidQuoteMarker,
                start,
                start,
                marker_context,
            )
        })?;
        Ok(vec![base_word_like(WordLike::single_word_quote(
            marker,
            self.verbatim(start, end)?,
        ))])
    }

    #[requires(true)]
    #[ensures(true)]
    fn lohu_quote(
        &mut self,
        lohu_word_with_modifiers: WordLike,
    ) -> Result<Vec<WordLike>, MorphologyError> {
        let lohu_context = word_like_context(
            &lohu_word_with_modifiers,
            MorphologyContextKind::QuotedWords,
        );
        let lohu = into_bare_word(lohu_word_with_modifiers).ok_or_else(|| {
            self.invalid_span(
                MorphologyErrorKind::InvalidQuoteMarker,
                self.index,
                self.index,
                lohu_context,
            )
        })?;
        let mut quoted_words = Vec::new();
        loop {
            self.skip_separators();
            if self.index == self.chars.len() {
                let mut words = vec![base_word_like(WordLike::bare(lohu))];
                words.extend(
                    quoted_words
                        .into_iter()
                        .map(|word| base_word_like(WordLike::bare(word))),
                );
                return Ok(words);
            }
            let word = self.next_plain_word()?;
            if is_simple_cmavo_text(&word, "le'u") {
                let lehu_context = word_like_context(&word, MorphologyContextKind::QuotedWords);
                let lehu = into_bare_word(word).ok_or_else(|| {
                    self.invalid_span(
                        MorphologyErrorKind::InvalidQuoteMarker,
                        self.index,
                        self.index,
                        lehu_context,
                    )
                })?;
                return Ok(vec![base_word_like(WordLike::lohu_quote(
                    lohu,
                    quoted_words,
                    lehu,
                ))]);
            }
            if let Some(inner) = into_bare_word(word) {
                quoted_words.push(inner);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn handle_bu(
        &self,
        acc: &mut Vec<WordLike>,
        bu_word_with_modifiers: WordLike,
    ) -> Result<(), MorphologyError> {
        let Some(prev) = acc.pop() else {
            let (start, end) =
                word_like_char_range(&bu_word_with_modifiers).unwrap_or((self.index, self.index));
            return Err(self.invalid_span_with_detail(
                MorphologyErrorKind::ExpectedWord,
                start,
                end,
                word_like_context(&bu_word_with_modifiers, MorphologyContextKind::Bu),
                Some(new!(MorphologyErrorDetail::ExpectedWord {
                    expected: ExpectedWordDetailKind::BuOperand,
                })),
            ));
        };
        let bu_context = word_like_context(&bu_word_with_modifiers, MorphologyContextKind::Bu);
        let bu = into_bare_word(bu_word_with_modifiers).ok_or_else(|| {
            self.invalid_span(
                MorphologyErrorKind::InvalidQuoteMarker,
                self.index,
                self.index,
                bu_context,
            )
        })?;
        acc.push(base_word_like(WordLike::letter(prev, bu)));
        Ok(())
    }

    #[requires(true)]
    #[ensures(true)]
    fn handle_si(&self, acc: &mut Vec<WordLike>) {
        drop(pop_previous_word_skipping_y(acc));
    }

    #[requires(true)]
    #[ensures(true)]
    fn handle_sa(&mut self, acc: &mut Vec<WordLike>) -> Result<(), MorphologyError> {
        let mut sa_count = 1;
        loop {
            self.skip_magic_noise(true)?;
            if self.index == self.chars.len() {
                return Ok(());
            }
            let replacement = match self.next_sa_base_segment() {
                Ok(replacement) => replacement,
                Err(error @ MorphologyError::UnterminatedZoiQuote { .. }) => return Err(error),
                Err(_) => {
                    acc.clear();
                    self.index = self.chars.len();
                    return Ok(());
                }
            };
            if replacement.len() != 1 {
                for word in replacement {
                    self.process_token(acc, word)?;
                }
                return Ok(());
            }
            let replacement = replacement.into_iter().next().expect("length checked");
            if is_simple_cmavo_text(&replacement, "sa") {
                sa_count += 1;
                continue;
            }
            let target_tag = sa_match_tag(self.options, &replacement);
            let acc_after_erase = target_tag
                .and_then(|tag| find_nth_matching_word_index(self.options, sa_count, tag, acc))
                .unwrap_or_default();
            acc.truncate(acc_after_erase);
            return self.process_token(acc, replacement);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_sa_base_segment(&mut self) -> Result<Vec<WordLike>, MorphologyError> {
        self.skip_separators();
        if self.peek_char().is_some_and(|value| value.is_ascii_digit()) {
            let candidate_end = self.candidate_end(self.index);
            if self.is_digit_sequence_candidate(self.index, candidate_end) {
                return self.digit_sequence();
            }
        }
        let word = self.next_plain_word()?;
        if is_simple_cmavo_text(&word, "lo'u") {
            return self.lohu_quote(word);
        }
        if is_simple_cmavo_text(&word, "zoi")
            || is_simple_cmavo_text(&word, "la'o")
            || is_simple_cmavo_text(&word, "mu'oi")
        {
            return self.zoi_quote(word);
        }
        if is_simple_cmavo_text(&word, "zo'oi")
            || is_simple_cmavo_text(&word, "la'oi")
            || is_simple_cmavo_text(&word, "ra'oi")
            || is_simple_cmavo_text(&word, "me'oi")
            || is_simple_cmavo_text(&word, "go'oi")
        {
            return self.single_word_quote(word);
        }
        if is_simple_cmavo_text(&word, "zo") || is_simple_cmavo_text(&word, "ma'oi") {
            return self.zo_quote(word);
        }
        if is_simple_cmavo_text(&word, "fa'o") {
            self.index = self.chars.len();
        }
        Ok(vec![word])
    }

    #[requires(true)]
    #[ensures(true)]
    fn handle_su(&self, acc: &mut Vec<WordLike>) {
        acc.truncate(su_boundary_index(acc));
    }

    #[requires(true)]
    #[ensures(true)]
    fn handle_zei(
        &mut self,
        acc: &mut Vec<WordLike>,
        zei_word_with_modifiers: WordLike,
    ) -> Result<(), MorphologyError> {
        self.skip_y_words();
        let next = self.next_plain_word();
        let prev_index = previous_word_skipping_y_index(acc);
        match (prev_index, next) {
            (Some(prev_index), Ok(next)) => {
                let zei_context =
                    word_like_context(&zei_word_with_modifiers, MorphologyContextKind::Zei);
                let Some(zei) = into_bare_word(zei_word_with_modifiers) else {
                    return Err(self.invalid_span(
                        MorphologyErrorKind::InvalidQuoteMarker,
                        self.index,
                        self.index,
                        zei_context,
                    ));
                };
                let right_context = word_like_context(&next, MorphologyContextKind::Zei);
                let Some(right) = into_bare_word(next) else {
                    return Err(self.invalid_span_with_detail(
                        MorphologyErrorKind::ExpectedWord,
                        self.index,
                        self.index,
                        right_context,
                        Some(new!(MorphologyErrorDetail::ExpectedWord {
                            expected: ExpectedWordDetailKind::ZeiOperand,
                        })),
                    ));
                };
                while acc.len() > prev_index + 1 {
                    acc.pop();
                }
                let prev = acc
                    .pop()
                    .expect("previous word index was checked as present");
                acc.push(base_word_like(WordLike::zei_lujvo(prev, zei, right)));
            }
            (Some(_), Err(error)) if !is_expected_word_error(&error) => {
                return Err(error_with_fallback_context(
                    error,
                    word_like_context(&zei_word_with_modifiers, MorphologyContextKind::Zei),
                ));
            }
            (None, Ok(_)) => {
                let (start, end) = word_like_char_range(&zei_word_with_modifiers)
                    .unwrap_or((self.index, self.index));
                return Err(self.invalid_span_with_detail(
                    MorphologyErrorKind::ExpectedWord,
                    start,
                    end,
                    word_like_context(&zei_word_with_modifiers, MorphologyContextKind::Zei),
                    Some(new!(MorphologyErrorDetail::ExpectedWord {
                        expected: ExpectedWordDetailKind::ZeiOperand,
                    })),
                ));
            }
            (_, Err(_)) => {
                let (start, end) = word_like_char_range(&zei_word_with_modifiers)
                    .unwrap_or((self.index, self.index));
                return Err(self.invalid_span_with_detail(
                    MorphologyErrorKind::ExpectedWord,
                    start,
                    end,
                    word_like_context(&zei_word_with_modifiers, MorphologyContextKind::Zei),
                    Some(new!(MorphologyErrorDetail::ExpectedWord {
                        expected: ExpectedWordDetailKind::ZeiOperand,
                    })),
                ));
            }
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|value| value.as_ref().is_none_or(|(end, _, start)| *end <= *start)))]
    fn find_zoi_close(
        &mut self,
        opening_delimiter: &Word,
        consumed_open_separator: bool,
    ) -> Result<Option<(usize, Word, usize)>, MorphologyError> {
        let opening_delimiter_canonical = canonicalize_text(opening_delimiter.phonemes().as_str());
        let mut cursor = self.index;
        if consumed_open_separator
            && let Some(closing_word) =
                self.zoi_closing_word_at(&opening_delimiter_canonical, cursor)
        {
            return Ok(Some((cursor, closing_word, cursor)));
        }
        while cursor < self.chars.len() {
            let pause_start = cursor;
            let mut saw_separator = false;
            while cursor < self.chars.len() && self.is_word_separator_at(cursor) {
                saw_separator = true;
                cursor += 1;
            }
            if saw_separator && cursor < self.chars.len() {
                if let Some(closing_word) =
                    self.zoi_closing_word_at(&opening_delimiter_canonical, cursor)
                {
                    return Ok(Some((
                        trim_trailing_separator_indices(&self.chars, self.index, pause_start),
                        closing_word,
                        cursor,
                    )));
                }
                cursor += 1;
            } else {
                cursor += 1;
            }
        }
        Ok(None)
    }

    #[requires(cursor <= self.chars.len())]
    #[ensures(true)]
    fn zoi_closing_word_at(
        &mut self,
        opening_delimiter_canonical: &str,
        cursor: usize,
    ) -> Option<Word> {
        let saved = self.index;
        self.index = cursor;
        let warning_count = self.warnings.len();
        let maybe_word = self.next_plain_word();
        self.warnings.truncate(warning_count);
        self.index = saved;
        if let Ok(word_with_modifiers) = maybe_word
            && let Some(closing_word) = extract_word(&word_with_modifiers)
            && canonical_text_eq(
                closing_word.phonemes().as_str(),
                opening_delimiter_canonical,
            )
        {
            return Some(closing_word);
        }
        None
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_plain_non_y_word(&mut self) -> Result<WordLike, MorphologyError> {
        loop {
            let word = self.next_plain_word()?;
            if !is_y_word(&word) {
                return Ok(word);
            }
        }
    }

    #[ensures(self.index <= self.chars.len())]
    #[requires(true)]
    fn skip_y_words(&mut self) {
        loop {
            self.skip_separators();
            let saved = self.index;
            let warning_count = self.warnings.len();
            match self.next_plain_word() {
                Ok(word) if is_y_word(&word) => {}
                _ => {
                    self.index = saved;
                    self.warnings.truncate(warning_count);
                    break;
                }
            }
        }
    }

    #[ensures(ret.as_ref().is_err() || self.index <= self.chars.len())]
    #[requires(true)]
    fn skip_magic_noise(&mut self, keep_y_before_bu: bool) -> Result<bool, MorphologyError> {
        loop {
            let before = self.index;
            self.skip_separators();
            let saved = self.index;
            let word_warning_count = self.warnings.len();
            match self.next_plain_word() {
                Ok(word) if is_y_word(&word) => {
                    let after_y = self.index;
                    self.skip_separators();
                    let bu_warning_count = self.warnings.len();
                    let followed_by_bu = self
                        .next_plain_word()
                        .ok()
                        .is_some_and(|next| is_simple_cmavo_text(&next, "bu"));
                    self.warnings.truncate(bu_warning_count);
                    self.index = if keep_y_before_bu && followed_by_bu {
                        saved
                    } else {
                        after_y
                    };
                }
                _ => {
                    self.index = saved;
                    self.warnings.truncate(word_warning_count);
                }
            }
            if self.index == before {
                return Ok(true);
            }
        }
    }

    #[ensures(self.index <= self.chars.len())]
    #[requires(true)]
    fn skip_separators(&mut self) {
        while self.index < self.chars.len() && self.is_magic_noise_at(self.index) {
            self.index += 1;
        }
    }

    #[requires(start <= self.chars.len())]
    #[ensures(ret >= start && ret <= self.chars.len())]
    fn candidate_end(&self, start: usize) -> usize {
        let mut end = start;
        while end < self.chars.len() && !self.is_word_separator_at(end) {
            end += 1;
        }
        end
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(ret >= start && ret <= end)]
    fn trim_trailing_commas(&self, start: usize, end: usize) -> usize {
        let mut trimmed_end = end;
        while start < trimmed_end
            && self
                .chars
                .get(trimmed_end - 1)
                .is_some_and(|source_char| source_char.value == ',')
        {
            trimmed_end -= 1;
        }
        trimmed_end
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(ret.is_none_or(|(index, _)| index >= start && index < end))]
    fn first_invalid_word_char(&self, start: usize, end: usize) -> Option<(usize, char)> {
        crate::segment::first_unnormalizable_word_char(self.slice(start, end), self.options)
            .map(|(offset, value)| (start + offset, value))
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(ret.as_ref().is_none_or(|text| !text.is_empty() || start == end))]
    fn checked_normalized_slice(&self, start: usize, end: usize) -> Option<String> {
        crate::segment::normalize_word_checked_with_options(self.slice(start, end), self.options)
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(true)]
    fn has_blocking_cmavo_prefix(&self, start: usize, end: usize) -> bool {
        let Some(whole_candidate) = self.checked_normalized_slice(start, end) else {
            return true;
        };
        if crate::segment::is_cmevla_with_options(&whole_candidate, self.options)
            || crate::segment::starts_with_cvcy_lujvo(&whole_candidate)
        {
            return false;
        }
        ((start + 1)..=end).any(|prefix_end| {
            self.checked_normalized_slice(start, prefix_end)
                .and_then(|prefix| crate::segment::parse_cmavo_form(&prefix))
                .is_some()
                && self.cmavo_boundary_ok(start, prefix_end, end)
        })
    }

    #[requires(prefix_start <= prefix_end && prefix_end <= candidate_end && candidate_end <= self.chars.len())]
    #[ensures(true)]
    fn cmavo_boundary_ok(
        &self,
        prefix_start: usize,
        prefix_end: usize,
        candidate_end: usize,
    ) -> bool {
        if self.pause_at(prefix_end) {
            return true;
        }
        let Some(prefix) = self.checked_normalized_slice(prefix_start, prefix_end) else {
            return false;
        };
        let Some(remainder) = self.checked_normalized_slice(prefix_end, candidate_end) else {
            return false;
        };
        if boundary_repeats_diphthong_semivowel(&prefix, &remainder) {
            return false;
        }
        !self.starts_with_pause_required_nucleus_at(prefix_end)
            && self.lojban_word_starts_at(prefix_end)
    }

    #[requires(index <= self.chars.len())]
    #[ensures(true)]
    fn post_word_at(&self, index: usize) -> bool {
        self.pause_at(index)
            || (!self.starts_with_pause_required_nucleus_at(index)
                && self.lojban_word_starts_at(index))
    }

    #[requires(index <= self.chars.len())]
    #[ensures(true)]
    fn pause_at(&self, index: usize) -> bool {
        let index = self.skip_commas_index(index);
        index == self.chars.len() || self.is_word_separator_at(index)
    }

    #[requires(index <= self.chars.len())]
    #[ensures(true)]
    fn starts_with_pause_required_nucleus_at(&self, index: usize) -> bool {
        let index = self.skip_commas_index(index);
        if index >= self.chars.len() || self.is_word_separator_at(index) {
            return false;
        }
        let end = self.candidate_end(index);
        self.checked_normalized_slice(index, end)
            .is_some_and(|normalized| starts_with_pause_required_nucleus(&text_chars(&normalized)))
    }

    #[requires(index <= self.chars.len())]
    #[ensures(true)]
    fn lojban_word_starts_at(&self, index: usize) -> bool {
        let index = self.skip_commas_index(index);
        if index >= self.chars.len() || self.is_word_separator_at(index) {
            return false;
        }
        self.streaming_word_candidate(index, self.candidate_end(index))
            .is_some()
    }

    #[requires(index <= self.chars.len())]
    #[ensures(ret >= index && ret <= self.chars.len())]
    fn skip_commas_index(&self, index: usize) -> usize {
        let mut cursor = index;
        while cursor < self.chars.len()
            && self
                .chars
                .get(cursor)
                .is_some_and(|source_char| source_char.value == ',')
        {
            cursor += 1;
        }
        cursor
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[requires(!phonemes.is_empty())]
    #[ensures(true)]
    fn word_with_modifiers(
        &mut self,
        start: usize,
        end: usize,
        kind: WordKind,
        phonemes: String,
    ) -> Result<WordLike, MorphologyError> {
        let normalized = self.normalized_source_chars(start, end);
        if let Some(range) = crate::segment::required_breve_not_glide_source_range(&normalized) {
            return Err(self.invalid_span_with_detail(
                MorphologyErrorKind::BreveNotGlide,
                range.start,
                range.end,
                self.context(word_context_kind(kind), start, end),
                crate::phonotactic_error_detail(MorphologyErrorKind::BreveNotGlide),
            ));
        }
        let span = self.source_span(start, end)?;
        let phonemes = Phonemes::from_canonical(phonemes).map_err(|_| {
            self.invalid_span(
                MorphologyErrorKind::UnrecognizedWord,
                start,
                end,
                self.context(word_context_kind(kind), start, end),
            )
        })?;
        let word = if kind == WordKind::Lujvo {
            let parts = crate::segment::parse_lujvo_parts(phonemes.as_str()).ok_or_else(|| {
                self.invalid_span_with_detail(
                    MorphologyErrorKind::InvalidLujvo,
                    start,
                    end,
                    self.context(MorphologyContextKind::Lujvo, start, end),
                    crate::segment::invalid_lujvo_error_detail(phonemes.as_str()),
                )
            })?;
            Word::lujvo(parts, span)
        } else {
            Word::from_kind(kind, phonemes, span)
        };
        self.warn_word_morphology(start, end, kind, &normalized);
        Ok(base_word_like(WordLike::bare(word)))
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(true)]
    fn normalized_source_chars(
        &self,
        start: usize,
        end: usize,
    ) -> Vec<crate::segment::NormalizedSourceChar> {
        crate::segment::normalize_source_chars(
            self.chars[start..end]
                .iter()
                .enumerate()
                .map(|(offset, source_char)| (start + offset, source_char.value)),
            self.options,
        )
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(true)]
    fn warn_word_morphology(
        &mut self,
        start: usize,
        end: usize,
        kind: WordKind,
        normalized: &[crate::segment::NormalizedSourceChar],
    ) {
        let mut warnings = Vec::new();
        if let Some(range) = crate::segment::cgv_source_range(normalized) {
            warnings.push((MorphologyWarningKind::ExperimentalCgv, range));
        }
        if let Some(range) = crate::segment::experimental_mz_source_range(normalized) {
            warnings.push((MorphologyWarningKind::ExperimentalMz, range));
        }
        if let Some(range) = crate::segment::latin_breve_not_glide_source_range(normalized) {
            warnings.push((MorphologyWarningKind::BreveNotGlide, range));
        }
        warnings.sort_by_key(|(_, range)| (range.start, range.end));
        for (warning_kind, range) in warnings {
            self.warnings.push(MorphologyWarning::new(
                warning_kind,
                range.start,
                range.end,
                self.slice(range.start, range.end).to_owned(),
                self.context(word_context_kind(kind), start, end),
            ));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn digit_sequence(&mut self) -> Result<Vec<WordLike>, MorphologyError> {
        let mut words = Vec::new();
        while self.index < self.chars.len() {
            let start = self.index;
            let value = self.chars[start].value;
            if value.is_ascii_digit() {
                self.index += 1;
                let phonemes = digit_to_cmavo(value).ok_or_else(|| {
                    self.invalid_span(
                        MorphologyErrorKind::UnrecognizedWord,
                        start,
                        start + 1,
                        self.context(MorphologyContextKind::Cmavo, start, start + 1),
                    )
                })?;
                words.push(self.word_with_modifiers(
                    start,
                    self.index,
                    WordKind::Cmavo,
                    phonemes.to_owned(),
                )?);
            } else if value == '.'
                && self
                    .chars
                    .get(start + 1)
                    .is_some_and(|next| next.value.is_ascii_digit())
            {
                self.index += 1;
                words.push(self.word_with_modifiers(
                    start,
                    self.index,
                    WordKind::Cmavo,
                    "pi".to_owned(),
                )?);
            } else if value == ','
                && self
                    .chars
                    .get(start + 1)
                    .is_some_and(|next| next.value.is_ascii_digit())
            {
                self.index += 2;
                let digit = self.chars[start + 1].value;
                let phonemes = digit_to_cmavo(digit).ok_or_else(|| {
                    self.invalid_span(
                        MorphologyErrorKind::UnrecognizedWord,
                        start + 1,
                        start + 2,
                        self.context(MorphologyContextKind::Cmavo, start, start + 2),
                    )
                })?;
                words.push(self.word_with_modifiers(
                    start,
                    self.index,
                    WordKind::Cmavo,
                    phonemes.to_owned(),
                )?);
            } else {
                break;
            }
        }
        Ok(words)
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(true)]
    fn is_digit_sequence_candidate(&self, start: usize, end: usize) -> bool {
        start < end
            && self.chars[start..end].iter().all(|source_char| {
                source_char.value.is_ascii_digit()
                    || source_char.value == '.'
                    || source_char.value == ','
            })
    }

    #[ensures(self.index <= self.chars.len())]
    #[requires(true)]
    fn consume_zoi_open_separators(&mut self) -> bool {
        let start = self.index;
        if self.peek_char().is_some_and(|value| value == '.') {
            while self.peek_char().is_some_and(|value| value == '.') {
                self.index += 1;
            }
            while self.peek_char().is_some_and(char::is_whitespace) {
                self.index += 1;
            }
        } else {
            while self.peek_char().is_some_and(char::is_whitespace) {
                self.index += 1;
            }
        }
        self.index != start
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|span| span.byte_start <= span.byte_end && span.char_start <= span.char_end))]
    fn source_span(&self, start: usize, end: usize) -> Result<SourceSpan, MorphologyError> {
        SourceSpan::new(
            self.source_id.clone(),
            self.byte_offset(start),
            self.byte_offset(end),
            start,
            end,
        )
        .map_err(MorphologyError::SourceSpan)
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|verbatim| verbatim.span.char_start == start && verbatim.span.char_end == end))]
    fn verbatim(&self, start: usize, end: usize) -> Result<Verbatim, MorphologyError> {
        Ok(Verbatim::new(
            self.source_span(start, end)?,
            self.slice(start, end).to_owned(),
        ))
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(true)]
    fn slice(&self, start: usize, end: usize) -> &'a str {
        &self.input[self.byte_offset(start)..self.byte_offset(end)]
    }

    #[requires(index <= self.chars.len())]
    #[ensures(ret <= self.input.len())]
    fn byte_offset(&self, index: usize) -> usize {
        self.chars
            .get(index)
            .map_or(self.input.len(), |source_char| source_char.byte_offset)
    }

    #[requires(true)]
    #[ensures(true)]
    fn peek_char(&self) -> Option<char> {
        self.chars
            .get(self.index)
            .map(|source_char| source_char.value)
    }

    #[requires(index <= self.chars.len())]
    #[ensures(true)]
    fn is_word_separator_at(&self, index: usize) -> bool {
        self.chars
            .get(index)
            .is_some_and(|source_char| crate::segment::is_separator(source_char.value))
    }

    #[requires(index <= self.chars.len())]
    #[ensures(true)]
    fn is_magic_noise_at(&self, index: usize) -> bool {
        self.chars.get(index).is_some_and(|source_char| {
            crate::segment::is_separator(source_char.value) || source_char.value == ','
        })
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(true)]
    fn invalid_word_error(&self, start: usize, end: usize) -> MorphologyError {
        let normalized = crate::segment::normalize_source_chars(
            self.chars[start..end]
                .iter()
                .enumerate()
                .map(|(offset, source_char)| (start + offset, source_char.value)),
            self.options,
        );
        if let Some(violation) = crate::segment::first_morphology_violation(&normalized) {
            let detail = if violation.kind == MorphologyErrorKind::Slinkuhi {
                Some(new!(MorphologyErrorDetail::Slinkuhi))
            } else {
                crate::phonotactic_error_detail(violation.kind)
            };
            return self.invalid_span_with_detail(
                violation.kind,
                violation.start,
                violation.end,
                self.context(context_kind_for_violation(violation.kind), start, end),
                detail,
            );
        }
        let stripped = normalized
            .iter()
            .filter_map(|source_char| (source_char.value != ',').then_some(source_char.value))
            .collect::<String>();
        if let Some(detail) = crate::segment::invalid_lujvo_error_detail(&stripped) {
            return self.invalid_span_with_detail(
                MorphologyErrorKind::InvalidLujvo,
                start,
                end,
                self.context(MorphologyContextKind::Lujvo, start, end),
                Some(detail),
            );
        }
        if let Some(detail) = crate::segment::fuhivla_y_error_detail(&stripped) {
            return self.invalid_span_with_detail(
                MorphologyErrorKind::UnrecognizedWord,
                start,
                end,
                self.context(MorphologyContextKind::Fuhivla, start, end),
                Some(detail),
            );
        }
        self.invalid_span(MorphologyErrorKind::UnrecognizedWord, start, end, None)
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(true)]
    fn invalid_span(
        &self,
        kind: MorphologyErrorKind,
        start: usize,
        end: usize,
        context: Option<MorphologyContext>,
    ) -> MorphologyError {
        self.invalid_span_with_detail(kind, start, end, context, None)
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(true)]
    fn invalid_span_with_detail(
        &self,
        kind: MorphologyErrorKind,
        start: usize,
        end: usize,
        context: Option<MorphologyContext>,
        detail: Option<MorphologyErrorDetail>,
    ) -> MorphologyError {
        MorphologyError::Invalid {
            kind,
            char_start: start,
            char_end: end,
            text: self.slice(start, end).to_owned(),
            context,
            detail,
        }
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(ret.as_ref().is_none_or(|context| context.char_start == start && context.char_end == end))]
    fn context(
        &self,
        kind: MorphologyContextKind,
        start: usize,
        end: usize,
    ) -> Option<MorphologyContext> {
        (start < end).then(|| MorphologyContext::new(kind, start, end))
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn word_kind_trace_label(kind: WordKind) -> &'static str {
    match kind {
        WordKind::Cmavo => "CMAVO",
        WordKind::Gismu => "GISMU",
        WordKind::Lujvo => "LUJVO",
        WordKind::Fuhivla => "FUHIVLA",
        WordKind::Cmevla => "CMEVLA",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn error_message(error: &MorphologyError) -> String {
    error.to_string()
}

#[requires(true)]
#[ensures(true)]
fn is_expected_word_error(error: &MorphologyError) -> bool {
    matches!(
        error,
        MorphologyError::Invalid {
            kind: MorphologyErrorKind::ExpectedWord,
            ..
        }
    )
}

#[requires(true)]
#[ensures(true)]
fn error_with_fallback_context(
    error: MorphologyError,
    fallback_context: Option<MorphologyContext>,
) -> MorphologyError {
    match error {
        MorphologyError::Invalid {
            kind,
            char_start,
            char_end,
            text,
            context: None,
            detail,
        } => MorphologyError::Invalid {
            kind,
            char_start,
            char_end,
            text,
            context: fallback_context,
            detail,
        },
        MorphologyError::UnterminatedZoiQuote {
            char_offset,
            delimiter,
            context: None,
        } => MorphologyError::UnterminatedZoiQuote {
            char_offset,
            delimiter,
            context: fallback_context,
        },
        error => error,
    }
}

#[requires(true)]
#[ensures(true)]
fn word_context_kind(kind: WordKind) -> MorphologyContextKind {
    match kind {
        WordKind::Cmavo => MorphologyContextKind::Cmavo,
        WordKind::Gismu => MorphologyContextKind::Gismu,
        WordKind::Lujvo => MorphologyContextKind::Lujvo,
        WordKind::Fuhivla => MorphologyContextKind::Fuhivla,
        WordKind::Cmevla => MorphologyContextKind::Cmevla,
    }
}

#[requires(true)]
#[ensures(true)]
fn context_kind_for_violation(kind: MorphologyErrorKind) -> MorphologyContextKind {
    match kind {
        MorphologyErrorKind::Slinkuhi => MorphologyContextKind::Fuhivla,
        MorphologyErrorKind::InvalidLujvo => MorphologyContextKind::Lujvo,
        MorphologyErrorKind::InvalidZoiDelimiter => MorphologyContextKind::DelimitedNonLojbanQuote,
        MorphologyErrorKind::InvalidQuoteMarker => MorphologyContextKind::QuotedWord,
        _ => MorphologyContextKind::Fuhivla,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|context| context.char_start < context.char_end))]
fn word_like_context(
    word_like: &WordLike,
    kind: MorphologyContextKind,
) -> Option<MorphologyContext> {
    let spans = word_like.source_spans();
    let first = spans.first()?;
    let last = spans.last()?;
    (first.char_start < last.char_end)
        .then(|| MorphologyContext::new(kind, first.char_start, last.char_end))
}

#[requires(true)]
#[ensures(ret.is_none_or(|(start, end)| start <= end))]
fn word_like_char_range(word_like: &WordLike) -> Option<(usize, usize)> {
    let spans = word_like.source_spans();
    let first = spans.first()?;
    let last = spans.last()?;
    Some((first.char_start, last.char_end))
}

#[invariant(self.end > 0, "streaming word candidates must consume input")]
#[invariant(!self.phonemes.is_empty(), "streaming word candidates must have phonemes")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct StreamingWordCandidate {
    end: usize,
    kind: WordKind,
    phonemes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Selmaho(_) => true)]
enum SAMatchTag<'a> {
    Selmaho(&'a str),
    Brivla,
    Cmevla,
}

#[requires(true)]
#[ensures(true)]
fn base_word_like(word_like: WordLike) -> WordLike {
    word_like
}

#[requires(true)]
#[ensures(true)]
fn extract_word(word: &WordLike) -> Option<Word> {
    bare_word_ref(word).cloned()
}

#[requires(true)]
#[ensures(true)]
fn bare_word_ref(word: &WordLike) -> Option<&Word> {
    word.bare_word()
}

#[requires(true)]
#[ensures(true)]
fn into_bare_word(word: WordLike) -> Option<Word> {
    match word.into_data() {
        data!(WordLike::PlainWord(word)) => Some(word),
        _ => None,
    }
}

#[requires(!text.is_empty())]
#[ensures(true)]
fn is_simple_cmavo_text(word: &WordLike, text: &str) -> bool {
    Cmavo::from_text(text).is_some_and(|cmavo| word.is_cmavo(cmavo))
}

#[requires(true)]
#[ensures(true)]
fn is_y_word(word: &WordLike) -> bool {
    bare_word_ref(word).is_some_and(|word| {
        word.kind() == WordKind::Cmavo && is_y_word_text(word.phonemes().as_str())
    })
}

#[requires(true)]
#[ensures(true)]
fn is_y_word_text(text: &str) -> bool {
    canonical_text_is_all(text, 'y')
}

#[requires(start <= end && end <= chars.len())]
#[ensures(ret >= start && ret <= end)]
fn trim_trailing_separator_indices(chars: &[SourceChar], start: usize, end: usize) -> usize {
    let mut trimmed_end = end;
    while start < trimmed_end
        && chars
            .get(trimmed_end - 1)
            .is_some_and(|source_char| crate::segment::is_separator(source_char.value))
    {
        trimmed_end -= 1;
    }
    trimmed_end
}

#[requires(true)]
#[ensures(true)]
fn pop_previous_word_skipping_y(acc: &mut Vec<WordLike>) -> Option<WordLike> {
    let mut last_y = None;
    while acc.last().is_some_and(is_y_word) {
        last_y = acc.pop();
    }
    acc.pop().or(last_y)
}

#[requires(true)]
#[ensures(true)]
fn previous_word_skipping_y_index(acc: &[WordLike]) -> Option<usize> {
    let mut last_y_index = None;
    for (index, token) in acc.iter().enumerate().rev() {
        if !is_y_word(token) {
            return Some(index);
        }
        last_y_index = Some(index);
    }
    last_y_index
}

#[requires(true)]
#[ensures(ret <= acc.len())]
fn su_boundary_index(acc: &[WordLike]) -> usize {
    for (index, token) in acc.iter().enumerate().rev() {
        let selmaho = erasure_selmaho(token);
        if matches!(selmaho, Some("NIhO" | "LU" | "TUhE" | "TO")) {
            return index;
        }
    }
    0
}

#[requires(true)]
#[ensures(true)]
fn sa_match_tag<'a>(options: &MorphologyOptions, word: &'a WordLike) -> Option<SAMatchTag<'a>> {
    match word.as_data() {
        data!(WordLike::PlainWord(word)) => match word.kind() {
            WordKind::Cmavo => word.selmaho().map(SAMatchTag::Selmaho),
            WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla => Some(SAMatchTag::Brivla),
            WordKind::Cmevla if options.cmevla_as_relation_words => Some(SAMatchTag::Brivla),
            WordKind::Cmevla => Some(SAMatchTag::Cmevla),
        },
        data!(WordLike::QuotedWord { .. }) => Some(SAMatchTag::Selmaho("ZO")),
        data!(WordLike::DelimitedNonLojbanQuote { zoi, .. }) => {
            zoi.selmaho().map(SAMatchTag::Selmaho)
        }
        data!(WordLike::QuotedWords { .. }) => Some(SAMatchTag::Selmaho("LOhU")),
        data!(WordLike::DelimitedWordQuote { marker, .. }) => {
            single_word_quote_marker_sa_tag(marker)
        }
        data!(WordLike::LerfuWord { .. }) => Some(SAMatchTag::Selmaho("BY")),
        data!(WordLike::ZeiCompound { .. }) => Some(SAMatchTag::Brivla),
    }
}

#[requires(true)]
#[ensures(true)]
fn single_word_quote_marker_sa_tag(marker: &Word) -> Option<SAMatchTag<'static>> {
    match marker.cmavo()? {
        Cmavo::Zohoi => Some(SAMatchTag::Selmaho("ZOhOI")),
        Cmavo::Lahoi => Some(SAMatchTag::Selmaho("LAhOI")),
        Cmavo::Rahoi => Some(SAMatchTag::Selmaho("RAhOI")),
        Cmavo::Mehoi => Some(SAMatchTag::Selmaho("MEhOI")),
        Cmavo::Gohoi => Some(SAMatchTag::Selmaho("GOhOI")),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn find_nth_matching_word_index(
    options: &MorphologyOptions,
    count: usize,
    target: SAMatchTag<'_>,
    acc: &[WordLike],
) -> Option<usize> {
    let mut remaining = count;
    for (index, token) in acc.iter().enumerate().rev() {
        if sa_match_tag(options, token) == Some(target) {
            remaining -= 1;
            if remaining == 0 {
                return Some(index);
            }
        }
    }
    None
}

#[requires(true)]
#[ensures(true)]
fn text_chars(text: &str) -> Vec<char> {
    text.chars().collect()
}

#[requires(true)]
#[ensures(true)]
fn boundary_repeats_diphthong_semivowel(prefix: &str, remainder: &str) -> bool {
    let prefix_chars = text_chars(prefix);
    let remainder_chars = text_chars(remainder);
    let Some(next_index) = next_non_comma_index(&remainder_chars, 0) else {
        return false;
    };
    let Some((last_index, last)) = previous_non_comma(&prefix_chars, prefix_chars.len()) else {
        return false;
    };
    let semivowel = match base_vowel(last) {
        Some('i') => 'ĭ',
        Some('u') => 'ŭ',
        _ => return false,
    };
    if !matches_diphthong_semivowel(remainder_chars[next_index], semivowel) {
        return false;
    }
    previous_non_comma(&prefix_chars, last_index).is_some_and(|(_, previous)| {
        matches!(
            (base_vowel(previous), semivowel),
            (Some('a'), 'ĭ') | (Some('e'), 'ĭ') | (Some('o'), 'ĭ') | (Some('a'), 'ŭ')
        )
    })
}

#[requires(true)]
#[ensures(true)]
fn has_explicit_brivla_stress(normalized_word: &str) -> bool {
    normalized_word
        .chars()
        .any(|value| matches!(value, 'á' | 'é' | 'í' | 'ó' | 'ú'))
}

#[requires(true)]
#[ensures(true)]
fn explicit_brivla_stress_is_valid(normalized_word: &str) -> bool {
    let chars = text_chars(normalized_word);
    let nuclei = stressable_nucleus_starts(&chars);
    let stressed = nuclei
        .iter()
        .copied()
        .filter(|index| {
            chars
                .get(*index)
                .is_some_and(|value| matches!(value, 'á' | 'é' | 'í' | 'ó' | 'ú'))
        })
        .collect::<Vec<_>>();
    nuclei
        .iter()
        .rev()
        .nth(1)
        .is_some_and(|penultimate| stressed.as_slice() == [*penultimate])
}

#[requires(true)]
#[ensures(true)]
fn stressable_nucleus_starts(chars: &[char]) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == ',' {
            index += 1;
            continue;
        }
        if let Some((_, end)) = parse_diphthong(chars, index) {
            starts.push(index);
            index = end;
        } else if let Some((_, end)) = parse_single_vowel(chars, index) {
            if !matches!(chars[index], 'y' | 'ý') {
                starts.push(index);
            }
            index = end;
        } else {
            index += 1;
        }
    }
    starts
}

#[requires(index <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|(found, _)| *found < old(index) && *found < chars.len()))]
fn previous_non_comma(chars: &[char], mut index: usize) -> Option<(usize, char)> {
    while index > 0 {
        index -= 1;
        if chars[index] != ',' {
            return Some((index, chars[index]));
        }
    }
    None
}

#[requires(start <= chars.len())]
#[ensures(true)]
fn starts_with_nucleus(chars: &[char], start: usize) -> bool {
    let mut start = start;
    while chars.get(start) == Some(&',') {
        start += 1;
    }
    if start >= chars.len() {
        return false;
    }
    parse_diphthong(chars, start).is_some() || parse_single_vowel(chars, start).is_some()
}

#[requires(true)]
#[ensures(true)]
fn starts_with_pause_required_nucleus(chars: &[char]) -> bool {
    let mut start = 0;
    while chars.get(start) == Some(&',') {
        start += 1;
    }
    chars
        .get(start)
        .is_some_and(|value| is_vowel(*value) || matches!(*value, 'y' | 'ý' | 'ĭ' | 'ŭ'))
}

#[requires(start <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|(_, end)| *end > start && *end <= chars.len()))]
fn parse_diphthong(chars: &[char], start: usize) -> Option<(String, usize)> {
    let first = *chars.get(start)?;
    let second = *chars.get(start + 1)?;
    let semivowel = match (base_vowel(first)?, second) {
        ('a', 'i' | 'í' | 'ĭ') | ('e', 'i' | 'í' | 'ĭ') | ('o', 'i' | 'í' | 'ĭ') => 'ĭ',
        ('a', 'u' | 'ú' | 'ŭ') => 'ŭ',
        _ => return None,
    };
    let end = start + 2;
    if next_non_comma_index(chars, end)
        .is_some_and(|next| matches_diphthong_semivowel(chars[next], semivowel))
    {
        return None;
    }
    if starts_with_nucleus(chars, end) {
        return None;
    }
    Some((format!("{}{}", normalize_vowel(first), semivowel), end))
}

#[requires(true)]
#[ensures(true)]
fn matches_diphthong_semivowel(value: char, semivowel: char) -> bool {
    match semivowel {
        'ĭ' => matches!(value, 'i' | 'í' | 'ĭ'),
        'ŭ' => matches!(value, 'u' | 'ú' | 'ŭ'),
        _ => false,
    }
}

#[requires(index <= chars.len())]
#[ensures(ret.is_none_or(|found| found >= index && found < chars.len()))]
fn next_non_comma_index(chars: &[char], mut index: usize) -> Option<usize> {
    while chars.get(index) == Some(&',') {
        index += 1;
    }
    (index < chars.len()).then_some(index)
}

#[requires(start <= chars.len())]
#[ensures(ret.as_ref().is_none_or(|(_, end)| *end == start + 1))]
fn parse_single_vowel(chars: &[char], start: usize) -> Option<(String, usize)> {
    let value = *chars.get(start)?;
    if value == 'y' || value == 'ý' {
        let end = start + 1;
        if starts_with_nucleus(chars, end) {
            return None;
        }
        return Some((value.to_string(), end));
    }
    if !is_vowel(value) && !matches!(value, 'ĭ' | 'ŭ') {
        return None;
    }
    let end = start + 1;
    if starts_with_nucleus(chars, end) {
        return None;
    }
    Some((normalize_vowel(value).to_string(), end))
}

#[requires(true)]
#[ensures(true)]
fn is_vowel(value: char) -> bool {
    base_vowel(value).is_some()
}

#[requires(true)]
#[ensures(true)]
fn base_vowel(value: char) -> Option<char> {
    match value {
        'a' | 'á' => Some('a'),
        'e' | 'é' => Some('e'),
        'i' | 'í' => Some('i'),
        'o' | 'ó' => Some('o'),
        'u' | 'ú' => Some('u'),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_vowel(value: char) -> char {
    match value {
        'á' => 'á',
        'é' => 'é',
        'í' => 'í',
        'ó' => 'ó',
        'ú' => 'ú',
        'ĭ' => 'i',
        'ŭ' => 'u',
        _ => base_vowel(value).unwrap_or(value),
    }
}

#[requires(true)]
#[ensures(true)]
fn digit_to_cmavo(value: char) -> Option<&'static str> {
    Some(match value {
        '0' => "no",
        '1' => "pa",
        '2' => "re",
        '3' => "ci",
        '4' => "vo",
        '5' => "mu",
        '6' => "xa",
        '7' => "ze",
        '8' => "bi",
        '9' => "so",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LujvoParseExpectation, PhonotacticDetailKind};
    use bityzba::requires;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn segments_ordinary_sentence() {
        let words =
            segment_words_with_modifiers("mi klama do", &MorphologyOptions::default(), None)
                .expect("valid morphology");

        assert_eq!(bare_phonemes(&words), ["mi", "kláma", "do"]);
        assert_eq!(bare_span(&words[1]).map(|span| span.byte_start), Some(3));
        assert_eq!(bare_span(&words[1]).map(|span| span.byte_end), Some(8));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn segments_adjacent_cmavo_and_brivla() {
        let words = segment_words_with_modifiers(
            "coimi miklama lonublanu coicai",
            &MorphologyOptions::default(),
            None,
        )
        .expect("valid morphology");

        assert_eq!(
            bare_phonemes(&words),
            [
                "coĭ", "mi", "mi", "kláma", "lo", "nu", "blánu", "coĭ", "caĭ"
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn explicit_stress_disambiguates_brivla_before_adjacent_cmavo() {
        let words = segment_words_with_modifiers("KLAmami", &MorphologyOptions::default(), None)
            .expect("valid morphology");

        assert_eq!(bare_phonemes(&words), ["kláma", "mi"]);
        assert_eq!(bare_span(&words[0]).map(|span| span.byte_end), Some(5));
        assert_eq!(bare_span(&words[1]).map(|span| span.byte_start), Some(5));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unstressed_brivla_prefix_does_not_split_before_adjacent_cmavo() {
        let words = segment_words_with_modifiers("klamami", &MorphologyOptions::default(), None)
            .expect("valid morphology");

        assert_eq!(bare_phonemes(&words), ["klamámi"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_forbidden_consonant_pairs_inside_fuhivla_shapes() {
        let cases = [
            ("basza", MorphologyErrorKind::VoicingMismatch, 2, 4),
            ("lapda", MorphologyErrorKind::VoicingMismatch, 2, 4),
            ("basca", MorphologyErrorKind::ForbiddenConsonantPair, 2, 4),
            ("najza", MorphologyErrorKind::ForbiddenConsonantPair, 2, 4),
        ];

        for (source, expected_kind, expected_start, expected_end) in cases {
            let error = segment_words_with_modifiers(source, &MorphologyOptions::default(), None)
                .expect_err("forbidden consonant pairs must reject the word");
            assert_invalid_error(
                &error,
                expected_kind,
                expected_start,
                expected_end,
                Some(MorphologyContextKind::Fuhivla),
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mz_relaxation_accepts_gismu_shape_with_warning() {
        let attempt =
            segment_words_with_modifiers_attempt("namzi", &MorphologyOptions::default(), None);
        let data = attempt.into_data();
        let words = data
            .result
            .expect("MZ relaxation should permit gismu shape");

        assert_eq!(bare_phonemes(&words), ["námzi"]);
        assert_eq!(data.warnings.len(), 1);
        assert_eq!(data.warnings[0].kind, MorphologyWarningKind::ExperimentalMz);
        assert_eq!(data.warnings[0].char_start, 2);
        assert_eq!(data.warnings[0].char_end, 4);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mz_relaxation_accepts_lujvo_boundary_with_warning() {
        let attempt =
            segment_words_with_modifiers_attempt("kamzifre", &MorphologyOptions::default(), None);
        let data = attempt.into_data();
        let words = data
            .result
            .expect("MZ relaxation should permit lujvo boundary");

        assert_eq!(bare_phonemes(&words), ["kamzífre"]);
        assert_eq!(data.warnings.len(), 1);
        assert_eq!(data.warnings[0].kind, MorphologyWarningKind::ExperimentalMz);
        assert_eq!(data.warnings[0].char_start, 2);
        assert_eq!(data.warnings[0].char_end, 4);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mz_relaxation_does_not_make_mz_an_initial_pair() {
        assert!(
            crate::segment::classify_word_with_options("mzai", &MorphologyOptions::default())
                .is_none()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cgv_relaxation_does_not_turn_invalid_lujvo_like_forms_into_fuhivla() {
        let error = segment_words_with_modifiers("language", &MorphologyOptions::default(), None)
            .expect_err("CgV relaxation must not bypass fu'ivla shape parsing");

        assert_invalid_error(&error, MorphologyErrorKind::UnrecognizedWord, 0, 8, None);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn invalid_lujvo_reports_parser_progress_for_xlaglymlu() {
        let error = segment_words_with_modifiers("xlaglymlu", &MorphologyOptions::default(), None)
            .expect_err("invalid lujvo-like word should fail");

        assert_invalid_error(
            &error,
            MorphologyErrorKind::InvalidLujvo,
            0,
            9,
            Some(MorphologyContextKind::Lujvo),
        );
        let data!(MorphologyErrorDetail::InvalidLujvo {
            parsed_prefix,
            expected,
        }) = invalid_error_detail(&error)
            .expect("invalid lujvo detail")
            .as_data()
        else {
            panic!("expected invalid lujvo detail, got {error:?}");
        };
        assert_eq!(parsed_prefix.as_deref(), Some("xlá"));
        assert_eq!(*expected, LujvoParseExpectation::FinalOrInitialRafsi);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn invalid_lujvo_neighbors_remain_valid() {
        let xlagymlu =
            segment_words_with_modifiers("xlagymlu", &MorphologyOptions::default(), None)
                .expect("valid lujvo with y-hyphen");
        let laglymlu =
            segment_words_with_modifiers("laglymlu", &MorphologyOptions::default(), None)
                .expect("valid lujvo without leading x");

        assert_eq!(
            bare_word(&xlagymlu[0]).expect("bare word").kind(),
            WordKind::Lujvo
        );
        assert_eq!(
            bare_word(&laglymlu[0]).expect("bare word").kind(),
            WordKind::Lujvo
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lujvo_can_end_with_fuhivla_core_after_y_hyphen() {
        let words = segment_words_with_modifiers(
            "pirytorveki jetcybolxada",
            &MorphologyOptions::default(),
            None,
        )
        .expect("lujvo may end with a fu'ivla core");

        assert_eq!(bare_phonemes(&words), ["pirytorvéki", "jetcybolxáda"]);
        assert_eq!(words.len(), 2);
        assert_eq!(
            bare_word(&words[0]).expect("bare word").kind(),
            WordKind::Lujvo
        );
        assert_eq!(
            bare_word(&words[1]).expect("bare word").kind(),
            WordKind::Lujvo
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn final_fuhivla_lujvo_core_is_decomposed_as_rafsi_part() {
        let words =
            segment_words_with_modifiers("jetcybolxada", &MorphologyOptions::default(), None)
                .expect("lujvo may end with a fu'ivla core");
        let parts = bare_word(&words[0])
            .expect("bare word")
            .lujvo_parts()
            .expect("lujvo parts");
        let part_texts = parts
            .iter()
            .map(|part| part.phonemes().as_str())
            .collect::<Vec<_>>();

        assert_eq!(part_texts, ["jetc", "y", "bolxáda"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vowel_initial_words_require_pause_at_word_boundary() {
        let error =
            segment_words_with_modifiers("mi leia klama", &MorphologyOptions::default(), None)
                .expect_err("vowel-initial word without pause should fail");
        assert_invalid_error(&error, MorphologyErrorKind::UnrecognizedWord, 3, 7, None);

        let words =
            segment_words_with_modifiers("mi le .ia klama", &MorphologyOptions::default(), None)
                .expect("pause before vowel-initial word should parse");
        assert_eq!(bare_phonemes(&words), ["mi", "le", "ĭa", "kláma"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn extended_final_lujvo_rafsi_cores_parse_as_lujvo() {
        let cases = [
            ("aktyiismu", &["akt", "y", "iismu"][..]),
            ("gimyiismu", &["gim", "y", "iismu"][..]),
            ("gismyiismu", &["gism", "y", "iismu"][..]),
            ("fuly'ismu", &["ful", "y'", "ismu"][..]),
            ("tcenelyiismu", &["tce", "nel", "y", "iismu"][..]),
        ];

        for (word, expected_parts) in cases {
            let words = segment_words_with_modifiers(word, &MorphologyOptions::default(), None)
                .unwrap_or_else(|error| panic!("{word} should parse as lujvo: {error}"));
            assert_lujvo_part_texts(word, &words, expected_parts);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn jvot3_extension_only_lujvo_forms_remain_invalid() {
        for word in [
            "kerly'u'u'ykerlo",
            "rly'u'u'ykerlo",
            "kerlyfa'u'ukerlo",
            "xlastmlu",
            "xlastymlu",
            "sincyrboua",
            "le'yia",
            "fa'u'yiismu",
        ] {
            assert!(
                segment_words_with_modifiers(word, &MorphologyOptions::default(), None).is_err(),
                "{word} should remain invalid"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn slinkuhi_reports_fuhivla_context() {
        let error = segment_words_with_modifiers("xlamkai", &MorphologyOptions::default(), None)
            .expect_err("slinku'i form should fail");

        assert_invalid_error(
            &error,
            MorphologyErrorKind::Slinkuhi,
            0,
            7,
            Some(MorphologyContextKind::Fuhivla),
        );
        let expected = new!(MorphologyErrorDetail::Slinkuhi);
        assert_eq!(invalid_error_detail(&error), Some(&expected));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fuhivla_y_rejection_reports_y_specific_detail() {
        let error = segment_words_with_modifiers("jgruyta", &MorphologyOptions::default(), None)
            .expect_err("fu'ivla candidate with y should fail");

        assert_invalid_error(&error, MorphologyErrorKind::UnrecognizedWord, 0, 7, None);
        assert_eq!(invalid_error_detail(&error), None);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cgv_relaxation_accepts_fuhivla_glide_onset_with_warning() {
        let attempt =
            segment_words_with_modifiers_attempt("atkuila", &MorphologyOptions::default(), None);
        let data = attempt.into_data();
        let words = data
            .result
            .expect("CgV relaxation should permit fu'ivla glide onset");

        assert_eq!(bare_phonemes(&words), ["atkŭíla"]);
        assert_eq!(
            bare_word(&words[0]).expect("bare word").kind(),
            WordKind::Fuhivla
        );
        assert_eq!(data.warnings.len(), 1);
        assert_eq!(
            data.warnings[0].kind,
            MorphologyWarningKind::ExperimentalCgv
        );
        assert_eq!(data.warnings[0].char_start, 2);
        assert_eq!(data.warnings[0].char_end, 5);
        assert_eq!(data.warnings[0].text, "kui");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cgv_relaxation_accepts_comma_crossing_fuhivla_glide_onset_with_warning() {
        let attempt =
            segment_words_with_modifiers_attempt("atku,ila", &MorphologyOptions::default(), None);
        let data = attempt.into_data();
        let words = data
            .result
            .expect("CgV relaxation should treat comma as syllable separator only");

        assert_eq!(bare_phonemes(&words), ["atkŭíla"]);
        assert_eq!(
            bare_word(&words[0]).expect("bare word").kind(),
            WordKind::Fuhivla
        );
        assert_eq!(data.warnings.len(), 1);
        assert_eq!(
            data.warnings[0].kind,
            MorphologyWarningKind::ExperimentalCgv
        );
        assert_eq!(data.warnings[0].char_start, 2);
        assert_eq!(data.warnings[0].char_end, 6);
        assert_eq!(data.warnings[0].text, "ku,i");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cgv_relaxation_still_accepts_existing_long_fuhivla_case() {
        let attempt = segment_words_with_modifiers_attempt(
            "cipnrxakuila",
            &MorphologyOptions::default(),
            None,
        );
        let data = attempt.into_data();
        let words = data
            .result
            .expect("existing CgV fu'ivla acceptance should remain valid");

        assert_eq!(bare_phonemes(&words), ["cipnrxakŭíla"]);
        assert_eq!(
            bare_word(&words[0]).expect("bare word").kind(),
            WordKind::Fuhivla
        );
        assert_eq!(data.warnings.len(), 1);
        assert_eq!(
            data.warnings[0].kind,
            MorphologyWarningKind::ExperimentalCgv
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fuhivla_with_initial_cluster_is_not_rejected_as_lujvo_like() {
        let words = segment_words_with_modifiers("ctremna", &MorphologyOptions::default(), None)
            .expect("valid fu'ivla morphology");

        assert_eq!(bare_phonemes(&words), ["ctrémna"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn trailing_comma_is_pause_not_word_text() {
        let words = segment_words_with_modifiers("klama,", &MorphologyOptions::default(), None)
            .expect("valid morphology");

        assert_eq!(bare_phonemes(&words), ["kláma"]);
        assert_eq!(bare_span(&words[0]).map(|span| span.byte_end), Some(5));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zo_quote_as_one_wordlike() {
        let words = segment_words_with_modifiers("zo si", &MorphologyOptions::default(), None)
            .expect("valid morphology");

        assert_eq!(words.len(), 1);
        let data!(WordLike::QuotedWord { zo, word }) = words[0].as_data() else {
            panic!("expected ZO quote");
        };
        assert_eq!(zo.phonemes().as_str(), "zo");
        assert_eq!(word.phonemes().as_str(), "si");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parses_zoi_quote_as_one_wordlike() {
        let words =
            segment_words_with_modifiers("zoi gy broda gy", &MorphologyOptions::default(), None)
                .expect("valid morphology");

        assert_eq!(words.len(), 1);
        let data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) = words[0].as_data()
        else {
            panic!("expected ZOI quote");
        };
        assert_eq!(zoi.phonemes().as_str(), "zoĭ");
        assert_eq!(opening_delimiter.phonemes().as_str(), "gy");
        assert_eq!(opening_delimiter.span().byte_start, 4);
        assert_eq!(opening_delimiter.span().byte_end, 6);
        assert_eq!(quoted_text.span.byte_start, 7);
        assert_eq!(quoted_text.span.byte_end, 12);
        assert_eq!(quoted_text.text, "broda");
        assert_eq!(closing_delimiter.phonemes().as_str(), "gy");
        assert_eq!(closing_delimiter.span().byte_start, 13);
        assert_eq!(closing_delimiter.span().byte_end, 15);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zoi_quote_opening_separator_variants_do_not_enter_payload() {
        for source in ["zoi gy Steve gy", "zoi gy.Steve.gy", "zoi gy. Steve gy"] {
            let words = segment_words_with_modifiers(source, &MorphologyOptions::default(), None)
                .expect("valid morphology");
            let data!(WordLike::DelimitedNonLojbanQuote { quoted_text, .. }) = words[0].as_data()
            else {
                panic!("expected ZOI quote for {source}");
            };
            assert_eq!(quoted_text.text, "Steve", "{source}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zoi_quote_whitespace_separator_does_not_consume_payload_dot() {
        let words =
            segment_words_with_modifiers("la'o gy .sig gy", &MorphologyOptions::default(), None)
                .expect("valid morphology");
        let data!(WordLike::DelimitedNonLojbanQuote { quoted_text, .. }) = words[0].as_data()
        else {
            panic!("expected ZOI quote");
        };
        assert_eq!(quoted_text.text, ".sig");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zoi_quote_opening_separator_can_precede_immediate_close() {
        for source in ["zoi ly ly", "zoi ly.ly"] {
            let words = segment_words_with_modifiers(source, &MorphologyOptions::default(), None)
                .expect("valid morphology");
            let data!(WordLike::DelimitedNonLojbanQuote { quoted_text, .. }) = words[0].as_data()
            else {
                panic!("expected ZOI quote for {source}");
            };
            assert_eq!(quoted_text.text, "", "{source}");
            assert_eq!(quoted_text.span.byte_start, quoted_text.span.byte_end);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reports_unclosed_zoi_quote() {
        let error =
            segment_words_with_modifiers("zoi gy broda", &MorphologyOptions::default(), None)
                .expect_err("unclosed ZOI should fail");

        assert!(error.to_string().contains("expected closing delimiter"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reports_unclosed_zoi_quote_after_opening_delimiter_at_eof() {
        for source in ["zoi gy", "la'o gy", "mu'oi gy"] {
            assert_unterminated_zoi_quote(source, "gy");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reports_unclosed_zoi_quote_after_opening_delimiter_with_payload() {
        for source in ["zoi gy foo", "la'o gy foo", "mu'oi gy foo"] {
            assert_unterminated_zoi_quote(source, "gy");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reports_expected_word_for_missing_zo_target() {
        let error = segment_words_with_modifiers("zo", &MorphologyOptions::default(), None)
            .expect_err("ZO requires a target");

        assert_invalid_error(
            &error,
            MorphologyErrorKind::ExpectedWord,
            2,
            2,
            Some(MorphologyContextKind::QuotedWord),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zo_quote_preserves_specific_quoted_word_error() {
        let error = segment_words_with_modifiers("zo biryrka", &MorphologyOptions::default(), None)
            .expect_err("invalid ZO target should surface its own morphology error");

        assert_invalid_error(
            &error,
            MorphologyErrorKind::InvalidLujvo,
            3,
            10,
            Some(MorphologyContextKind::Lujvo),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zo_quote_preserves_specific_quoted_word_violation() {
        let error = segment_words_with_modifiers("zo basza", &MorphologyOptions::default(), None)
            .expect_err("invalid ZO target should keep its specific morphology violation");

        assert_invalid_error(
            &error,
            MorphologyErrorKind::VoicingMismatch,
            5,
            7,
            Some(MorphologyContextKind::Fuhivla),
        );
        let expected = new!(MorphologyErrorDetail::Phonotactic {
            reason: PhonotacticDetailKind::VoicingMismatch,
        });
        assert_eq!(invalid_error_detail(&error), Some(&expected));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reports_expected_word_for_bu_without_operand() {
        let error = segment_words_with_modifiers("bu", &MorphologyOptions::default(), None)
            .expect_err("BU requires a preceding word");

        assert_invalid_error(
            &error,
            MorphologyErrorKind::ExpectedWord,
            0,
            2,
            Some(MorphologyContextKind::Bu),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reports_expected_word_for_zei_without_operand() {
        let error = segment_words_with_modifiers("zei", &MorphologyOptions::default(), None)
            .expect_err("ZEI requires operands");

        assert_invalid_error(
            &error,
            MorphologyErrorKind::ExpectedWord,
            0,
            3,
            Some(MorphologyContextKind::Zei),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reports_expected_word_for_zei_without_right_operand() {
        let error = segment_words_with_modifiers("broda zei", &MorphologyOptions::default(), None)
            .expect_err("ZEI requires a right operand");

        assert_invalid_error(
            &error,
            MorphologyErrorKind::ExpectedWord,
            6,
            9,
            Some(MorphologyContextKind::Zei),
        );
        let expected = new!(MorphologyErrorDetail::ExpectedWord {
            expected: ExpectedWordDetailKind::ZeiOperand,
        });
        assert_eq!(invalid_error_detail(&error), Some(&expected));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sa_treats_zei_compound_as_brivla_match() {
        let words = segment_words_with_modifiers(
            "lo brodi zei broda mi sa brode cu broda",
            &MorphologyOptions::default(),
            None,
        )
        .expect("SA should replace the previous ZEI compound as a brivla");

        assert_eq!(bare_phonemes(&words), ["lo", "bróde", "cu", "bróda"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sa_zei_does_not_match_inside_zei_compound() {
        let error = segment_words_with_modifiers(
            "lo mi zei do mi do sa zei di cu broda",
            &MorphologyOptions::default(),
            None,
        )
        .expect_err("SA ZEI should erase to the start and leave ZEI without a left operand");

        assert_invalid_error(
            &error,
            MorphologyErrorKind::ExpectedWord,
            22,
            25,
            Some(MorphologyContextKind::Zei),
        );
        let expected = new!(MorphologyErrorDetail::ExpectedWord {
            expected: ExpectedWordDetailKind::ZeiOperand,
        });
        assert_eq!(invalid_error_detail(&error), Some(&expected));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sa_treats_bu_word_as_by_match() {
        let words = segment_words_with_modifiers(
            "lo broda bu mi sa by di cu broda",
            &MorphologyOptions::default(),
            None,
        )
        .expect("SA BY should replace the previous BU-created lerfu word");

        assert_eq!(bare_phonemes(&words), ["lo", "by", "di", "cu", "bróda"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sa_bu_does_not_decompose_bu_word() {
        let cases = [
            ("lo broda bu mi sa bu di cu broda", 18, 20),
            (".abu sa bu", 8, 10),
        ];

        for (source, start, end) in cases {
            let error = segment_words_with_modifiers(source, &MorphologyOptions::default(), None)
                .expect_err("SA BU should not decompose a BU-created lerfu word");
            assert_invalid_error(
                &error,
                MorphologyErrorKind::ExpectedWord,
                start,
                end,
                Some(MorphologyContextKind::Bu),
            );
            let expected = new!(MorphologyErrorDetail::ExpectedWord {
                expected: ExpectedWordDetailKind::BuOperand,
            });
            assert_eq!(invalid_error_detail(&error), Some(&expected), "{source}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sa_treats_quote_wordlikes_by_marker() {
        let zo_words = segment_words_with_modifiers(
            "lo zo broda mi sa zo da cu broda",
            &MorphologyOptions::default(),
            None,
        )
        .expect("SA ZO should replace the previous ZO quote");
        assert_eq!(zo_words.len(), 4);
        assert_eq!(
            bare_word(&zo_words[0])
                .expect("leading word should remain")
                .phonemes()
                .as_str(),
            "lo"
        );
        let data!(WordLike::QuotedWord { word, .. }) = zo_words[1].as_data() else {
            panic!("expected replacement ZO quote");
        };
        assert_eq!(word.phonemes().as_str(), "da");

        let zoi_words = segment_words_with_modifiers(
            "lo zoi gy foo gy mi sa zoi gy bar gy cu broda",
            &MorphologyOptions::default(),
            None,
        )
        .expect("SA ZOI should replace the previous ZOI quote");
        assert_eq!(zoi_words.len(), 4);
        assert_eq!(
            bare_word(&zoi_words[0])
                .expect("leading word should remain")
                .phonemes()
                .as_str(),
            "lo"
        );
        let data!(WordLike::DelimitedNonLojbanQuote { quoted_text, .. }) = zoi_words[1].as_data()
        else {
            panic!("expected replacement ZOI quote");
        };
        assert_eq!(quoted_text.text, "bar");

        let lohu_words = segment_words_with_modifiers(
            "lo lo'u do cinki le'u mi sa lo'u do fenki le'u cu broda",
            &MorphologyOptions::default(),
            None,
        )
        .expect("SA LOhU should replace the previous LOhU quote");
        assert_eq!(lohu_words.len(), 4);
        assert_eq!(
            bare_word(&lohu_words[0])
                .expect("leading word should remain")
                .phonemes()
                .as_str(),
            "lo"
        );
        let data!(WordLike::QuotedWords { quoted_words, .. }) = lohu_words[1].as_data() else {
            panic!("expected replacement LOhU quote");
        };
        assert_eq!(
            quoted_words
                .iter()
                .map(|word| word.phonemes().into_string())
                .collect::<Vec<_>>(),
            vec!["do".to_string(), "fénki".to_string()]
        );

        let delimited_words = segment_words_with_modifiers(
            "lo zo'oi foo mi sa zo'oi bar cu broda",
            &MorphologyOptions::default(),
            None,
        )
        .expect("SA single-word quote marker should replace the previous quote");
        assert_eq!(delimited_words.len(), 4);
        assert_eq!(
            bare_word(&delimited_words[0])
                .expect("leading word should remain")
                .phonemes()
                .as_str(),
            "lo"
        );
        let data!(WordLike::DelimitedWordQuote { quoted_text, .. }) = delimited_words[1].as_data()
        else {
            panic!("expected replacement delimited word quote");
        };
        assert_eq!(quoted_text.text, "bar");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zei_preserves_specific_right_operand_error() {
        let error =
            segment_words_with_modifiers("broda zei biryrka", &MorphologyOptions::default(), None)
                .expect_err("invalid ZEI right operand should surface its own morphology error");

        assert_invalid_error(
            &error,
            MorphologyErrorKind::InvalidLujvo,
            10,
            17,
            Some(MorphologyContextKind::Lujvo),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reports_invalid_zoi_delimiter_for_missing_delimiter() {
        let error = segment_words_with_modifiers("zoi", &MorphologyOptions::default(), None)
            .expect_err("ZOI requires a delimiter");

        assert_invalid_error(
            &error,
            MorphologyErrorKind::InvalidZoiDelimiter,
            3,
            3,
            Some(MorphologyContextKind::DelimitedNonLojbanQuote),
        );
        let expected = new!(MorphologyErrorDetail::InvalidZoiDelimiter {
            reason: ZoiDelimiterDetailKind::Missing,
        });
        assert_eq!(invalid_error_detail(&error), Some(&expected));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zoi_quote_preserves_specific_opening_delimiter_error() {
        let error = segment_words_with_modifiers(
            "zoi biryrka foo biryrka",
            &MorphologyOptions::default(),
            None,
        )
        .expect_err("invalid ZOI delimiter should surface its own morphology error");

        assert_invalid_error(
            &error,
            MorphologyErrorKind::InvalidLujvo,
            4,
            11,
            Some(MorphologyContextKind::Lujvo),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reports_invalid_zoi_delimiter_for_y() {
        let error =
            segment_words_with_modifiers("zoi y broda y", &MorphologyOptions::default(), None)
                .expect_err("Y cannot be a ZOI delimiter");

        assert_invalid_error(
            &error,
            MorphologyErrorKind::InvalidZoiDelimiter,
            4,
            5,
            Some(MorphologyContextKind::DelimitedNonLojbanQuote),
        );
        let expected = new!(MorphologyErrorDetail::InvalidZoiDelimiter {
            reason: ZoiDelimiterDetailKind::YWord,
        });
        assert_eq!(invalid_error_detail(&error), Some(&expected));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn keeps_full_y_run_as_bu_operand() {
        let words = segment_words_with_modifiers(".yyyyy. bu", &MorphologyOptions::default(), None)
            .expect("valid morphology");

        let data!(WordLike::LerfuWord { base, bu }) = words[0].as_data() else {
            panic!("expected BU letter");
        };
        let data!(WordLike::PlainWord(base)) = base.as_data() else {
            panic!("expected bare Y base");
        };
        assert_eq!(base.phonemes().as_str(), "yyyyy");
        assert_eq!(base.span().byte_start, 1);
        assert_eq!(base.span().byte_end, 6);
        assert_eq!(bu.phonemes().as_str(), "bu");
    }

    #[requires(true)]
    #[ensures(true)]
    fn bare_phonemes(words: &[WordLike]) -> Vec<String> {
        words
            .iter()
            .map(|word| bare_word(word).expect("bare word").phonemes().into_string())
            .collect()
    }

    #[requires(true)]
    #[ensures(true)]
    fn bare_span(word: &WordLike) -> Option<&SourceSpan> {
        bare_word(word).map(Word::span)
    }

    #[requires(true)]
    #[ensures(true)]
    fn bare_word(word: &WordLike) -> Option<&Word> {
        match word.as_data() {
            data!(WordLike::PlainWord(word)) => Some(word),
            _ => None,
        }
    }

    #[requires(!source.is_empty())]
    #[requires(!expected_parts.is_empty())]
    #[ensures(true)]
    fn assert_lujvo_part_texts(source: &str, words: &[WordLike], expected_parts: &[&str]) {
        let [word_like] = words else {
            panic!("{source} should parse as one word");
        };
        let word = bare_word(word_like).unwrap_or_else(|| panic!("{source} should be a bare word"));
        assert_eq!(word.kind(), WordKind::Lujvo, "{source}");
        let actual_parts = word
            .lujvo_parts()
            .unwrap_or_else(|| panic!("{source} should expose lujvo parts"));
        assert_eq!(actual_parts.len(), expected_parts.len(), "{source}");
        for (actual, expected) in actual_parts.iter().zip(expected_parts) {
            assert!(
                crate::canonical_text_eq(actual.phonemes().as_str(), expected),
                "{source}: parsed part `{}` did not match `{expected}`",
                actual.phonemes().as_str()
            );
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_invalid_error(
        error: &MorphologyError,
        expected_kind: MorphologyErrorKind,
        expected_start: usize,
        expected_end: usize,
        expected_context: Option<MorphologyContextKind>,
    ) {
        let MorphologyError::Invalid {
            kind,
            char_start,
            char_end,
            context,
            ..
        } = error
        else {
            panic!("expected invalid morphology error, got {error:?}");
        };
        assert_eq!(*kind, expected_kind);
        assert_eq!(*char_start, expected_start);
        assert_eq!(*char_end, expected_end);
        assert_eq!(
            context.as_ref().map(|context| context.kind),
            expected_context
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_unterminated_zoi_quote(source: &str, expected_delimiter: &str) {
        let error = segment_words_with_modifiers(source, &MorphologyOptions::default(), None)
            .expect_err("source should contain an unterminated ZOI-family quote");
        let MorphologyError::UnterminatedZoiQuote { delimiter, .. } = error else {
            panic!("expected unterminated ZOI quote for {source}");
        };
        assert_eq!(delimiter, expected_delimiter, "{source}");
    }

    #[requires(true)]
    #[ensures(true)]
    fn invalid_error_detail(error: &MorphologyError) -> Option<&MorphologyErrorDetail> {
        let MorphologyError::Invalid { detail, .. } = error else {
            return None;
        };
        detail.as_ref()
    }
}
