use bityzba::{data, ensures, invariant, requires};
use chumsky::error::Rich;
use chumsky::prelude::*;
use chumsky::span::SimpleSpan;
use jbotci_source::{SourceId, SourceSpan};

use crate::{
    Cmavo, MorphologyError, MorphologyOptions, Phonemes, Verbatim, Word, WordKind, WordLike,
    WordLikeData, canonical_text_eq, canonical_text_is_all, canonicalize_text, erasure_selmaho,
};

type MorphExtra<'src> = extra::Err<Rich<'src, char>>;

#[requires(true)]
#[ensures(true)]
pub(crate) fn segment_words_with_modifiers(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_raw(input, options, source_id)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn segment_words_with_modifiers_raw(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordLike>, MorphologyError> {
    parser(input, options.clone(), source_id)
        .parse(input)
        .into_result()
        .map_err(|errors| morphology_error(input, errors))
}

#[requires(true)]
#[ensures(true)]
fn parser<'src>(
    input: &'src str,
    options: MorphologyOptions,
    source_id: Option<SourceId>,
) -> impl Parser<'src, &'src str, Vec<WordLike>, MorphExtra<'src>> {
    custom::<_, &'src str, Vec<WordLike>, MorphExtra<'src>>(move |inp| {
        let before = inp.cursor();
        let start_span: SimpleSpan = inp.span_since(&before);
        match Segmenter::new(input, &options, source_id.clone()).segment_raw() {
            Ok(words) => {
                for _ in input.chars() {
                    inp.skip();
                }
                Ok(words)
            }
            Err(error) => Err(Rich::custom(start_span, error.to_string())),
        }
    })
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
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn segment_raw(mut self) -> Result<Vec<WordLike>, MorphologyError> {
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
    fn next_segment(&mut self) -> Result<Vec<WordLike>, MorphologyError> {
        self.skip_separators();
        if self.peek_char().is_some_and(|value| value.is_ascii_digit()) {
            let candidate_end = self.candidate_end(self.index);
            if self.is_digit_sequence_candidate(self.index, candidate_end) {
                return self.digit_sequence();
            }
        }
        let start = self.index;
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
            return Ok(vec![word]);
        }
        if self.index == start {
            return Err(self.invalid_at(start, "", "internal morphology parser made no progress"));
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
            return self.handle_bu(acc, token);
        }
        if is_simple_cmavo_text(&token, "si") {
            self.handle_si(acc);
            return Ok(());
        }
        if is_simple_cmavo_text(&token, "fa'o") {
            return Ok(());
        }
        if is_simple_cmavo_text(&token, "sa") {
            return self.handle_sa(acc);
        }
        if is_simple_cmavo_text(&token, "su") {
            self.handle_su(acc);
            return Ok(());
        }
        if is_simple_cmavo_text(&token, "zei") {
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
        let end = self.trim_trailing_commas(start, candidate_end);
        if start == candidate_end || start == end {
            return Err(self.invalid_at(start, "", "expected Lojban word"));
        }
        let raw = self.slice(start, end);
        if let Some((invalid_index, invalid_char)) = self.first_invalid_word_char(start, end) {
            return Err(self.invalid_at(
                invalid_index,
                &invalid_char.to_string(),
                "unsupported character in Lojban word",
            ));
        }
        let normalized = crate::segment::normalize_word_with_options(raw, self.options);
        if normalized.is_empty() {
            return Err(self.invalid_at(start, raw, "no valid morphology characters"));
        }

        if let Some((kind, phonemes)) =
            crate::segment::classify_word_with_options(raw, &normalized, self.options)
            && !self.has_blocking_cmavo_prefix(start, end)
            && matches!(kind, WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla)
        {
            self.index = end;
            return self.word_with_modifiers(start, end, kind, phonemes);
        }

        if crate::segment::is_cmevla_with_options(&normalized, self.options) {
            self.index = end;
            return self.word_with_modifiers(
                start,
                end,
                WordKind::Cmevla,
                crate::segment::canonicalize_word_phonemes(&normalized),
            );
        }

        if let Some(phonemes) = crate::segment::parse_cmavo_form(&normalized) {
            self.index = end;
            return self.word_with_modifiers(start, end, WordKind::Cmavo, phonemes);
        }

        if let Some(cmavo) = self.cmavo_prefix(start, end) {
            self.index = cmavo.end;
            return self.word_with_modifiers(start, cmavo.end, WordKind::Cmavo, cmavo.phonemes);
        }

        if let Some((kind, phonemes)) =
            crate::segment::classify_word_with_options(raw, &normalized, self.options)
        {
            self.index = end;
            return self.word_with_modifiers(start, end, kind, phonemes);
        }

        Err(self.unsupported_at(
            start,
            raw,
            "the Rust Chumsky morphology port does not yet cover this word shape",
        ))
    }

    #[requires(true)]
    #[ensures(true)]
    fn zo_quote(
        &mut self,
        zo_word_with_modifiers: WordLike,
    ) -> Result<Vec<WordLike>, MorphologyError> {
        let after_marker = self.index;
        self.skip_y_words();
        let quoted = match self.next_plain_non_y_word() {
            Ok(quoted) => quoted,
            Err(_) => {
                self.index = after_marker;
                return Ok(vec![zo_word_with_modifiers]);
            }
        };
        let zo = into_bare_word(zo_word_with_modifiers)
            .ok_or_else(|| self.invalid_at(self.index, "zo", "zo must be a single word"))?;
        let word = into_bare_word(quoted)
            .ok_or_else(|| self.invalid_at(self.index, "", "zo requires a word to quote"))?;
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
        let opening_word_with_modifiers = match self.next_plain_word() {
            Ok(opening_word_with_modifiers) => opening_word_with_modifiers,
            Err(_) => {
                self.index = after_marker;
                return Ok(vec![zoi_word_with_modifiers]);
            }
        };
        if bare_word_ref(&zoi_word_with_modifiers).is_none() {
            return Err(self.invalid_at(self.index, "zoi", "ZOI must be a single word"));
        }
        let opening_delimiter = into_bare_word(opening_word_with_modifiers).ok_or_else(|| {
            self.invalid_at(self.index, "", "ZOI delimiter must be a single word")
        })?;
        if is_y_word_text(opening_delimiter.phonemes().as_str()) {
            self.index = after_marker;
            return Ok(vec![zoi_word_with_modifiers]);
        }
        if self.index == self.chars.len() {
            self.index = after_marker;
            return Ok(vec![zoi_word_with_modifiers]);
        }
        self.consume_zoi_open_dots();
        let quoted_start = self.index;
        let Some((quoted_end, closing_delimiter, close_start)) =
            self.find_zoi_close(&opening_delimiter)?
        else {
            return Err(MorphologyError::UnterminatedZoiQuote {
                char_offset: quoted_start,
                delimiter: opening_delimiter.phonemes().into_string(),
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
        let after_marker = self.index;
        self.skip_separators();
        let start = self.index;
        let end = self.candidate_end(start);
        if start == end {
            self.index = after_marker;
            return Ok(vec![marker_word_with_modifiers]);
        }
        self.index = end;
        let marker = into_bare_word(marker_word_with_modifiers).ok_or_else(|| {
            self.invalid_at(start, "", "single-word quote marker must be a single word")
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
        let lohu = into_bare_word(lohu_word_with_modifiers)
            .ok_or_else(|| self.invalid_at(self.index, "lo'u", "LOhU must be a single word"))?;
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
                let lehu = into_bare_word(word).ok_or_else(|| {
                    self.invalid_at(self.index, "le'u", "LEhU must be a single word")
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
            return Err(self.invalid_at(self.index, "bu", "bu requires a preceding word"));
        };
        let bu = into_bare_word(bu_word_with_modifiers)
            .ok_or_else(|| self.invalid_at(self.index, "bu", "bu must be a single word"))?;
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
            if is_simple_cmavo_text(&replacement, "zei") {
                acc.push(replacement);
                return Ok(());
            }
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
        let next = self.next_plain_word().ok();
        let prev_index = previous_word_skipping_y_index(acc);
        match (prev_index, next) {
            (Some(prev_index), Some(next)) => {
                let Some(zei) = into_bare_word(zei_word_with_modifiers) else {
                    return Err(self.invalid_at(self.index, "zei", "ZEI must be a single word"));
                };
                let Some(right) = into_bare_word(next) else {
                    return Err(self.invalid_at(self.index, "", "ZEI requires a following word"));
                };
                while acc.len() > prev_index + 1 {
                    acc.pop();
                }
                let prev = acc
                    .pop()
                    .expect("previous word index was checked as present");
                acc.push(base_word_like(WordLike::zei_lujvo(prev, zei, right)));
            }
            (None, Some(next)) => {
                acc.push(zei_word_with_modifiers);
                acc.push(next);
            }
            (_, None) => acc.push(zei_word_with_modifiers),
        }
        Ok(())
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_err() || ret.as_ref().is_ok_and(|value| value.as_ref().is_none_or(|(end, _, start)| *end <= *start)))]
    fn find_zoi_close(
        &mut self,
        opening_delimiter: &Word,
    ) -> Result<Option<(usize, Word, usize)>, MorphologyError> {
        let opening_delimiter_canonical = canonicalize_text(opening_delimiter.phonemes().as_str());
        let mut cursor = self.index;
        while cursor < self.chars.len() {
            let pause_start = cursor;
            let mut saw_separator = false;
            while cursor < self.chars.len() && self.is_word_separator_at(cursor) {
                saw_separator = true;
                cursor += 1;
            }
            if saw_separator && cursor < self.chars.len() {
                let saved = self.index;
                self.index = cursor;
                let maybe_word = self.next_plain_word();
                let after_word = self.index;
                self.index = saved;
                if let Ok(word_with_modifiers) = maybe_word
                    && let Some(closing_word) = extract_word(&word_with_modifiers)
                    && canonical_text_eq(
                        closing_word.phonemes().as_str(),
                        &opening_delimiter_canonical,
                    )
                {
                    return Ok(Some((
                        trim_trailing_separator_indices(&self.chars, self.index, pause_start),
                        closing_word,
                        cursor,
                    )));
                }
                cursor = after_word.max(cursor + 1);
            } else {
                cursor += 1;
            }
        }
        Ok(None)
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
            match self.next_plain_word() {
                Ok(word) if is_y_word(&word) => {}
                _ => {
                    self.index = saved;
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
            match self.next_plain_word() {
                Ok(word) if is_y_word(&word) => {
                    let after_y = self.index;
                    self.skip_separators();
                    let followed_by_bu = self
                        .next_plain_word()
                        .ok()
                        .is_some_and(|next| is_simple_cmavo_text(&next, "bu"));
                    self.index = if keep_y_before_bu && followed_by_bu {
                        saved
                    } else {
                        after_y
                    };
                }
                _ => self.index = saved,
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
    #[ensures(ret.as_ref().is_none_or(|prefix| prefix.end > start && prefix.end <= end && !prefix.phonemes.is_empty()))]
    fn cmavo_prefix(&self, start: usize, end: usize) -> Option<CmavoPrefix> {
        let whole_candidate =
            crate::segment::normalize_word_with_options(self.slice(start, end), self.options);
        if crate::segment::is_cmevla_with_options(&whole_candidate, self.options)
            || crate::segment::starts_with_cvcy_lujvo(&whole_candidate)
        {
            return None;
        }
        ((start + 1)..=end).find_map(|prefix_end| {
            let phonemes =
                crate::segment::parse_cmavo_form(&crate::segment::normalize_word_with_options(
                    self.slice(start, prefix_end),
                    self.options,
                ))?;
            if self.cmavo_boundary_ok(start, prefix_end, end) {
                Some(CmavoPrefix {
                    end: prefix_end,
                    phonemes,
                })
            } else {
                None
            }
        })
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(ret.is_none_or(|(index, _)| index >= start && index < end))]
    fn first_invalid_word_char(&self, start: usize, end: usize) -> Option<(usize, char)> {
        self.chars[start..end]
            .iter()
            .enumerate()
            .find_map(|(offset, source_char)| {
                (!crate::segment::is_normalizable_word_char(source_char.value, self.options))
                    .then_some((start + offset, source_char.value))
            })
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(true)]
    fn has_blocking_cmavo_prefix(&self, start: usize, end: usize) -> bool {
        let whole_candidate =
            crate::segment::normalize_word_with_options(self.slice(start, end), self.options);
        if crate::segment::is_cmevla_with_options(&whole_candidate, self.options)
            || crate::segment::starts_with_cvcy_lujvo(&whole_candidate)
        {
            return false;
        }
        ((start + 1)..=end).any(|prefix_end| {
            crate::segment::parse_cmavo_form(&crate::segment::normalize_word_with_options(
                self.slice(start, prefix_end),
                self.options,
            ))
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
        if prefix_end == candidate_end {
            return true;
        }
        let prefix = crate::segment::normalize_word_with_options(
            self.slice(prefix_start, prefix_end),
            self.options,
        );
        let remainder = crate::segment::normalize_word_with_options(
            self.slice(prefix_end, candidate_end),
            self.options,
        );
        if boundary_repeats_diphthong_semivowel(&prefix, &remainder) {
            return false;
        }
        !starts_with_nucleus(&text_chars(&remainder), 0)
            && self.candidate_starts_with_supported_word(prefix_end, candidate_end)
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[ensures(true)]
    fn candidate_starts_with_supported_word(&self, start: usize, end: usize) -> bool {
        if self.first_invalid_word_char(start, end).is_some() {
            return false;
        }
        let raw = self.slice(start, end);
        let normalized = crate::segment::normalize_word_with_options(raw, self.options);
        crate::segment::classify_word_with_options(raw, &normalized, self.options).is_some()
            || ((start + 1)..=end).any(|prefix_end| {
                crate::segment::parse_cmavo_form(&crate::segment::normalize_word_with_options(
                    self.slice(start, prefix_end),
                    self.options,
                ))
                .is_some()
                    && self.cmavo_boundary_ok(start, prefix_end, end)
            })
    }

    #[requires(start <= end && end <= self.chars.len())]
    #[requires(!phonemes.is_empty())]
    #[ensures(true)]
    fn word_with_modifiers(
        &self,
        start: usize,
        end: usize,
        kind: WordKind,
        phonemes: String,
    ) -> Result<WordLike, MorphologyError> {
        let span = self.source_span(start, end)?;
        let phonemes = Phonemes::from_canonical(phonemes)
            .map_err(|error| self.invalid_at(start, self.slice(start, end), &error))?;
        let word = if kind == WordKind::Lujvo {
            let parts = crate::segment::parse_lujvo_parts(phonemes.as_str()).ok_or_else(|| {
                self.invalid_at(start, self.slice(start, end), "invalid lujvo decomposition")
            })?;
            Word::lujvo(parts, span)
        } else {
            Word::from_kind(kind, phonemes, span)
        };
        Ok(base_word_like(WordLike::bare(word)))
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
                    self.invalid_at(start, &value.to_string(), "unrecognized digit")
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
                    self.invalid_at(start + 1, &digit.to_string(), "unrecognized digit")
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
    fn consume_zoi_open_dots(&mut self) {
        if self.peek_char() != Some('.') {
            return;
        }
        while self.peek_char() == Some('.') {
            self.index += 1;
        }
        while self.peek_char().is_some_and(|value| value.is_whitespace()) {
            self.index += 1;
        }
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

    #[requires(!reason.is_empty(), "morphology invalid errors must have a reason")]
    #[ensures(true)]
    fn invalid_at(&self, index: usize, word: &str, reason: &str) -> MorphologyError {
        MorphologyError::Invalid {
            char_offset: index,
            word: word.to_owned(),
            reason: reason.to_owned(),
        }
    }

    #[requires(!reason.is_empty(), "morphology unsupported errors must have a reason")]
    #[ensures(true)]
    fn unsupported_at(&self, index: usize, word: &str, reason: &str) -> MorphologyError {
        MorphologyError::Unsupported {
            char_offset: index,
            word: word.to_owned(),
            reason: reason.to_owned(),
        }
    }
}

#[ensures(matches!(ret, MorphologyError::Invalid { ref reason, .. } if !reason.is_empty()) || !matches!(ret, MorphologyError::Invalid { .. }))]
#[requires(true)]
fn morphology_error(input: &str, errors: Vec<Rich<'_, char>>) -> MorphologyError {
    let Some(error) = errors.into_iter().next() else {
        return MorphologyError::Invalid {
            char_offset: 0,
            word: String::new(),
            reason: "unknown Chumsky morphology error".to_owned(),
        };
    };
    let span = error.span();
    MorphologyError::Invalid {
        char_offset: char_offset(input, span.start),
        word: input
            .get(span.start..span.end)
            .unwrap_or_default()
            .to_owned(),
        reason: error.to_string(),
    }
}

#[requires(byte_offset <= input.len())]
#[requires(input.is_char_boundary(byte_offset))]
#[ensures(ret <= input.chars().count())]
fn char_offset(input: &str, byte_offset: usize) -> usize {
    input[..byte_offset].chars().count()
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct CmavoPrefix {
    end: usize,
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
        data!(WordLike::Bare(word)) => Some(word),
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
    match bare_word_ref(word) {
        Some(word) => match word.kind() {
            WordKind::Cmavo => word.selmaho().map(SAMatchTag::Selmaho),
            WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla => Some(SAMatchTag::Brivla),
            WordKind::Cmevla if options.cmevla_as_relation_words => Some(SAMatchTag::Brivla),
            WordKind::Cmevla => Some(SAMatchTag::Cmevla),
        },
        None => erasure_selmaho(word).map(SAMatchTag::Selmaho),
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
    if !is_vowel(value) {
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
    fn parses_zo_quote_as_one_wordlike() {
        let words = segment_words_with_modifiers("zo si", &MorphologyOptions::default(), None)
            .expect("valid morphology");

        assert_eq!(words.len(), 1);
        let data!(WordLike::ZoQuote { zo, word }) = words[0].as_data() else {
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
        let data!(WordLike::ZoiQuote {
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
        assert_eq!(quoted_text.span.byte_start, 6);
        assert_eq!(quoted_text.span.byte_end, 12);
        assert_eq!(closing_delimiter.phonemes().as_str(), "gy");
        assert_eq!(closing_delimiter.span().byte_start, 13);
        assert_eq!(closing_delimiter.span().byte_end, 15);
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
            data!(WordLike::Bare(word)) => Some(word),
            _ => None,
        }
    }
}
