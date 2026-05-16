use std::collections::VecDeque;

use chumsky::error::Rich;
use chumsky::prelude::*;
use chumsky::span::SimpleSpan;
use jbotci_source::{SourceId, SourceSpan};

use crate::{MorphologyError, MorphologyOptions, Word, WordKind, WordLike, WordWithModifiers};

type MorphExtra<'src> = extra::Err<Rich<'src, char>>;

pub(crate) fn segment_words_with_modifiers(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    Ok(apply_passes(segment_words_with_modifiers_raw(
        input, options, source_id,
    )?))
}

pub(crate) fn segment_words_with_modifiers_raw(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordWithModifiers>, MorphologyError> {
    parser(input, options.clone(), source_id)
        .parse(input)
        .into_result()
        .map_err(|errors| morphology_error(input, errors))
}

fn parser<'src>(
    input: &'src str,
    options: MorphologyOptions,
    source_id: Option<SourceId>,
) -> impl Parser<'src, &'src str, Vec<WordWithModifiers>, MorphExtra<'src>> {
    custom::<_, &'src str, Vec<WordWithModifiers>, MorphExtra<'src>>(move |inp| {
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
struct SourceChar {
    byte_offset: usize,
    value: char,
}

#[derive(Debug)]
struct Segmenter<'a> {
    input: &'a str,
    options: &'a MorphologyOptions,
    source_id: Option<SourceId>,
    chars: Vec<SourceChar>,
    index: usize,
}

impl<'a> Segmenter<'a> {
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

    fn segment_raw(mut self) -> Result<Vec<WordWithModifiers>, MorphologyError> {
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

    fn next_segment(&mut self) -> Result<Vec<WordWithModifiers>, MorphologyError> {
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

    fn process_segment(
        &mut self,
        acc: &mut Vec<WordWithModifiers>,
        segment: Vec<WordWithModifiers>,
    ) -> Result<(), MorphologyError> {
        if segment.len() != 1 {
            for word in segment {
                acc.push(word);
            }
            return Ok(());
        }
        let token = segment.into_iter().next().expect("length checked");
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

    fn next_plain_word(&mut self) -> Result<WordWithModifiers, MorphologyError> {
        self.skip_separators();
        let start = self.index;
        let candidate_end = self.candidate_end(start);
        let end = self.trim_trailing_commas(start, candidate_end);
        if start == candidate_end || start == end {
            return Err(self.invalid_at(start, "", "expected Lojban word"));
        }
        let raw = self.slice(start, end);
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

    fn zo_quote(
        &mut self,
        zo_word_with_modifiers: WordWithModifiers,
    ) -> Result<Vec<WordWithModifiers>, MorphologyError> {
        let after_marker = self.index;
        self.skip_y_words();
        let quoted = match self.next_plain_non_y_word() {
            Ok(quoted) => quoted,
            Err(_) => {
                self.index = after_marker;
                return Ok(vec![zo_word_with_modifiers]);
            }
        };
        let zo = extract_word(&zo_word_with_modifiers)
            .ok_or_else(|| self.invalid_at(self.index, "zo", "zo must be a single word"))?;
        let word = extract_word(&quoted)
            .ok_or_else(|| self.invalid_at(self.index, "", "zo requires a word to quote"))?;
        Ok(vec![base_word_like(WordLike::ZoQuote {
            zo: Box::new(zo),
            word: Box::new(word),
        })])
    }

    fn zoi_quote(
        &mut self,
        zoi_word_with_modifiers: WordWithModifiers,
    ) -> Result<Vec<WordWithModifiers>, MorphologyError> {
        let after_marker = self.index;
        self.skip_separators();
        let opening_word_with_modifiers = match self.next_plain_word() {
            Ok(opening_word_with_modifiers) => opening_word_with_modifiers,
            Err(_) => {
                self.index = after_marker;
                return Ok(vec![zoi_word_with_modifiers]);
            }
        };
        let zoi = extract_word(&zoi_word_with_modifiers)
            .ok_or_else(|| self.invalid_at(self.index, "zoi", "ZOI must be a single word"))?;
        let opening_delimiter = extract_word(&opening_word_with_modifiers).ok_or_else(|| {
            self.invalid_at(self.index, "", "ZOI delimiter must be a single word")
        })?;
        if is_y_word_text(&opening_delimiter.phonemes) {
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
            return Err(self.invalid_at(
                quoted_start,
                "",
                &format!(
                    "unterminated ZOI quote, expected closing delimiter `{}`",
                    opening_delimiter.phonemes
                ),
            ));
        };
        self.index = close_start;
        let closing = self.next_plain_word()?;
        let closing_delimiter = extract_word(&closing).unwrap_or(closing_delimiter);
        Ok(vec![base_word_like(WordLike::ZoiQuote {
            zoi: Box::new(zoi),
            opening_delimiter: Box::new(opening_delimiter),
            quoted_text: self.source_span(quoted_start, quoted_end)?,
            closing_delimiter: Box::new(closing_delimiter),
        })])
    }

    fn single_word_quote(
        &mut self,
        marker_word_with_modifiers: WordWithModifiers,
    ) -> Result<Vec<WordWithModifiers>, MorphologyError> {
        let after_marker = self.index;
        self.skip_separators();
        let start = self.index;
        let end = self.candidate_end(start);
        if start == end {
            self.index = after_marker;
            return Ok(vec![marker_word_with_modifiers]);
        }
        self.index = end;
        let marker = extract_word(&marker_word_with_modifiers).ok_or_else(|| {
            self.invalid_at(start, "", "single-word quote marker must be a single word")
        })?;
        Ok(vec![base_word_like(WordLike::SingleWordQuote {
            marker: Box::new(marker),
            quoted_text: self.source_span(start, end)?,
        })])
    }

    fn lohu_quote(
        &mut self,
        lohu_word_with_modifiers: WordWithModifiers,
    ) -> Result<Vec<WordWithModifiers>, MorphologyError> {
        let lohu = extract_word(&lohu_word_with_modifiers)
            .ok_or_else(|| self.invalid_at(self.index, "lo'u", "LOhU must be a single word"))?;
        let mut quoted_words = Vec::new();
        loop {
            self.skip_separators();
            if self.index == self.chars.len() {
                let mut words = vec![base_word_like(WordLike::Bare {
                    word: Box::new(lohu),
                })];
                words.extend(quoted_words.into_iter().map(|word| {
                    base_word_like(WordLike::Bare {
                        word: Box::new(word),
                    })
                }));
                return Ok(words);
            }
            let word = self.next_plain_word()?;
            if is_simple_cmavo_text(&word, "le'u") {
                let lehu = extract_word(&word).ok_or_else(|| {
                    self.invalid_at(self.index, "le'u", "LEhU must be a single word")
                })?;
                return Ok(vec![base_word_like(WordLike::LohuQuote {
                    lohu: Box::new(lohu),
                    quoted_words,
                    lehu: Box::new(lehu),
                })]);
            }
            if let Some(inner) = extract_word(&word) {
                quoted_words.push(inner);
            }
        }
    }

    fn handle_bu(
        &self,
        acc: &mut Vec<WordWithModifiers>,
        bu_word_with_modifiers: WordWithModifiers,
    ) -> Result<(), MorphologyError> {
        let Some(prev) = acc.pop() else {
            return Err(self.invalid_at(self.index, "bu", "bu requires a preceding word"));
        };
        let bu = extract_word(&bu_word_with_modifiers)
            .ok_or_else(|| self.invalid_at(self.index, "bu", "bu must be a single word"))?;
        acc.push(base_word_like(WordLike::Letter {
            base: Box::new(get_word_like(&prev)),
            bu: Box::new(bu),
        }));
        Ok(())
    }

    fn handle_si(&self, acc: &mut Vec<WordWithModifiers>) {
        let (_prev, rest) = skip_acc_y(acc);
        *acc = rest;
    }

    fn handle_sa(&mut self, acc: &mut Vec<WordWithModifiers>) -> Result<(), MorphologyError> {
        let mut sa_count = 1;
        loop {
            self.skip_magic_noise(true)?;
            if self.index == self.chars.len() {
                return Ok(());
            }
            let replacement = match self.next_sa_base_segment() {
                Ok(replacement) => replacement,
                Err(_) => {
                    acc.clear();
                    self.index = self.chars.len();
                    return Ok(());
                }
            };
            if replacement.len() != 1 {
                for word in replacement {
                    self.process_segment(acc, vec![word])?;
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
                .and_then(|tag| find_nth_matching_word(self.options, sa_count, tag, acc))
                .unwrap_or_default();
            *acc = acc_after_erase;
            if is_simple_cmavo_text(&replacement, "zei") {
                acc.push(replacement);
                return Ok(());
            }
            return self.process_segment(acc, vec![replacement]);
        }
    }

    fn next_sa_base_segment(&mut self) -> Result<Vec<WordWithModifiers>, MorphologyError> {
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

    fn handle_su(&self, acc: &mut Vec<WordWithModifiers>) {
        *acc = erase_back_to_su_boundary(acc);
    }

    fn handle_zei(
        &mut self,
        acc: &mut Vec<WordWithModifiers>,
        zei_word_with_modifiers: WordWithModifiers,
    ) -> Result<(), MorphologyError> {
        self.skip_y_words();
        let next = self.next_plain_word().ok();
        let (prev, rest) = skip_acc_y(acc);
        match (prev, next) {
            (Some(prev), Some(next)) => {
                let Some(zei) = extract_word(&zei_word_with_modifiers) else {
                    return Err(self.invalid_at(self.index, "zei", "ZEI must be a single word"));
                };
                let Some(right) = extract_word(&next) else {
                    return Err(self.invalid_at(self.index, "", "ZEI requires a following word"));
                };
                *acc = rest;
                acc.push(base_word_like(WordLike::ZeiLujvo {
                    left: Box::new(get_word_like(&prev)),
                    zei: Box::new(zei),
                    right: Box::new(right),
                }));
            }
            (None, Some(next)) => {
                acc.push(zei_word_with_modifiers);
                acc.push(next);
            }
            (_, None) => acc.push(zei_word_with_modifiers),
        }
        Ok(())
    }

    fn find_zoi_close(
        &mut self,
        opening_delimiter: &Word,
    ) -> Result<Option<(usize, Word, usize)>, MorphologyError> {
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
                    && canonicalize_text(&closing_word.phonemes)
                        == canonicalize_text(&opening_delimiter.phonemes)
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

    fn next_plain_non_y_word(&mut self) -> Result<WordWithModifiers, MorphologyError> {
        loop {
            let word = self.next_plain_word()?;
            if !is_y_word(&word) {
                return Ok(word);
            }
        }
    }

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

    fn skip_separators(&mut self) {
        while self.index < self.chars.len() && self.is_magic_noise_at(self.index) {
            self.index += 1;
        }
    }

    fn candidate_end(&self, start: usize) -> usize {
        let mut end = start;
        while end < self.chars.len() && !self.is_word_separator_at(end) {
            end += 1;
        }
        end
    }

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
            if self.cmavo_boundary_ok(prefix_end, end) {
                Some(CmavoPrefix {
                    end: prefix_end,
                    phonemes,
                })
            } else {
                None
            }
        })
    }

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
                && self.cmavo_boundary_ok(prefix_end, end)
        })
    }

    fn cmavo_boundary_ok(&self, prefix_end: usize, candidate_end: usize) -> bool {
        if prefix_end == candidate_end {
            return true;
        }
        let remainder = crate::segment::normalize_word_with_options(
            self.slice(prefix_end, candidate_end),
            self.options,
        );
        !starts_with_nucleus(&text_chars(&remainder), 0)
            && self.candidate_starts_with_supported_word(prefix_end, candidate_end)
    }

    fn candidate_starts_with_supported_word(&self, start: usize, end: usize) -> bool {
        let raw = self.slice(start, end);
        let normalized = crate::segment::normalize_word_with_options(raw, self.options);
        crate::segment::classify_word_with_options(raw, &normalized, self.options).is_some()
            || ((start + 1)..=end).any(|prefix_end| {
                crate::segment::parse_cmavo_form(&crate::segment::normalize_word_with_options(
                    self.slice(start, prefix_end),
                    self.options,
                ))
                .is_some()
            })
    }

    fn word_with_modifiers(
        &self,
        start: usize,
        end: usize,
        kind: WordKind,
        phonemes: String,
    ) -> Result<WordWithModifiers, MorphologyError> {
        Ok(base_word_like(WordLike::Bare {
            word: Box::new(Word {
                kind,
                phonemes,
                span: self.source_span(start, end)?,
                surface_override: None,
                dialect_transform: None,
            }),
        }))
    }

    fn digit_sequence(&mut self) -> Result<Vec<WordWithModifiers>, MorphologyError> {
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

    fn is_digit_sequence_candidate(&self, start: usize, end: usize) -> bool {
        start < end
            && self.chars[start..end].iter().all(|source_char| {
                source_char.value.is_ascii_digit()
                    || source_char.value == '.'
                    || source_char.value == ','
            })
    }

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

    fn slice(&self, start: usize, end: usize) -> &'a str {
        &self.input[self.byte_offset(start)..self.byte_offset(end)]
    }

    fn byte_offset(&self, index: usize) -> usize {
        self.chars
            .get(index)
            .map_or(self.input.len(), |source_char| source_char.byte_offset)
    }

    fn peek_char(&self) -> Option<char> {
        self.chars
            .get(self.index)
            .map(|source_char| source_char.value)
    }

    fn is_word_separator_at(&self, index: usize) -> bool {
        self.chars
            .get(index)
            .is_some_and(|source_char| crate::segment::is_separator(source_char.value))
    }

    fn is_magic_noise_at(&self, index: usize) -> bool {
        self.chars.get(index).is_some_and(|source_char| {
            crate::segment::is_separator(source_char.value) || source_char.value == ','
        })
    }

    fn invalid_at(&self, index: usize, word: &str, reason: &str) -> MorphologyError {
        MorphologyError::Invalid {
            char_offset: index,
            word: word.to_owned(),
            reason: reason.to_owned(),
        }
    }

    fn unsupported_at(&self, index: usize, word: &str, reason: &str) -> MorphologyError {
        MorphologyError::Unsupported {
            char_offset: index,
            word: word.to_owned(),
            reason: reason.to_owned(),
        }
    }
}

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

fn char_offset(input: &str, byte_offset: usize) -> usize {
    input[..byte_offset].chars().count()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CmavoPrefix {
    end: usize,
    phonemes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SAMatchTag<'a> {
    Selmaho(&'a str),
    Brivla,
    Cmevla,
}

fn base_word_like(word_like: WordLike) -> WordWithModifiers {
    WordWithModifiers::BaseWord {
        word_like: Box::new(word_like),
    }
}

fn extract_word(word: &WordWithModifiers) -> Option<Word> {
    match word {
        WordWithModifiers::BaseWord { word_like } => match word_like.as_ref() {
            WordLike::Bare { word } => Some((**word).clone()),
            _ => None,
        },
        WordWithModifiers::Emphasized { word_like, .. } => match word_like.as_ref() {
            WordLike::Bare { word } => Some((**word).clone()),
            _ => None,
        },
        WordWithModifiers::WithIndicator { base, .. } => extract_word(base),
        WordWithModifiers::StandaloneIndicator { indicator, .. } => Some((**indicator).clone()),
        WordWithModifiers::NotEof => None,
    }
}

fn get_word_like(word: &WordWithModifiers) -> WordLike {
    match word {
        WordWithModifiers::BaseWord { word_like } => (**word_like).clone(),
        WordWithModifiers::Emphasized { word_like, .. } => (**word_like).clone(),
        WordWithModifiers::WithIndicator { base, .. } => get_word_like(base),
        WordWithModifiers::StandaloneIndicator { indicator, .. } => WordLike::Bare {
            word: indicator.clone(),
        },
        WordWithModifiers::NotEof => WordLike::Bare {
            word: Box::new(Word {
                kind: WordKind::Cmavo,
                phonemes: String::new(),
                span: SourceSpan::new(None, 0, 0, 0, 0).expect("valid empty span"),
                surface_override: None,
                dialect_transform: None,
            }),
        },
    }
}

fn is_simple_cmavo_text(word: &WordWithModifiers, text: &str) -> bool {
    match word {
        WordWithModifiers::BaseWord { word_like } => match word_like.as_ref() {
            WordLike::Bare { word } => {
                word.kind == WordKind::Cmavo && canonicalize_text(&word.phonemes) == text
            }
            _ => false,
        },
        WordWithModifiers::Emphasized { word_like, .. } => match word_like.as_ref() {
            WordLike::Bare { word } => {
                word.kind == WordKind::Cmavo && canonicalize_text(&word.phonemes) == text
            }
            _ => false,
        },
        WordWithModifiers::WithIndicator { base, .. } => is_simple_cmavo_text(base, text),
        _ => false,
    }
}

fn is_y_word(word: &WordWithModifiers) -> bool {
    match word {
        WordWithModifiers::BaseWord { word_like } => match word_like.as_ref() {
            WordLike::Bare { word } => {
                word.kind == WordKind::Cmavo && is_y_word_text(&word.phonemes)
            }
            _ => false,
        },
        _ => false,
    }
}

fn is_y_word_text(text: &str) -> bool {
    let canonical = canonicalize_text(text);
    !canonical.is_empty() && canonical.chars().all(|value| value == 'y')
}

fn canonicalize_text(text: &str) -> String {
    text.chars()
        .filter(|value| *value != ',')
        .flat_map(strip_diacritic)
        .flat_map(char::to_lowercase)
        .collect()
}

fn strip_diacritic(value: char) -> Option<char> {
    Some(match value {
        'á' | 'à' | 'Á' | 'À' => 'a',
        'é' | 'è' | 'É' | 'È' => 'e',
        'í' | 'ì' | 'ĭ' | 'Ĭ' | 'Í' | 'Ì' => 'i',
        'ó' | 'ò' | 'Ó' | 'Ò' => 'o',
        'ú' | 'ù' | 'ŭ' | 'Ŭ' | 'Ú' | 'Ù' => 'u',
        'ý' | 'ỳ' | 'Ý' | 'Ỳ' => 'y',
        '\u{0301}' | '\u{0300}' | '\u{0306}' => return None,
        other => other,
    })
}

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

fn skip_acc_y(acc: &[WordWithModifiers]) -> (Option<WordWithModifiers>, Vec<WordWithModifiers>) {
    let mut last_y = None;
    for (index, token) in acc.iter().enumerate().rev() {
        if is_y_word(token) {
            last_y = Some(token.clone());
            continue;
        }
        return (Some(token.clone()), acc[..index].to_vec());
    }
    (last_y, Vec::new())
}

fn erase_back_to_su_boundary(acc: &[WordWithModifiers]) -> Vec<WordWithModifiers> {
    for (index, token) in acc.iter().enumerate().rev() {
        let selmaho = visible_selmaho(&get_word_like(token));
        if matches!(selmaho, Some("NIhO" | "LU" | "TUhE" | "TO")) {
            return acc[..index].to_vec();
        }
    }
    Vec::new()
}

fn sa_match_tag<'a>(
    options: &MorphologyOptions,
    word: &'a WordWithModifiers,
) -> Option<SAMatchTag<'a>> {
    match get_word_like(word) {
        WordLike::Bare { word } => match word.kind {
            WordKind::Cmavo => visible_selmaho(&WordLike::Bare { word }).map(SAMatchTag::Selmaho),
            WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla => Some(SAMatchTag::Brivla),
            WordKind::Cmevla if options.cmevla_as_relation_words => Some(SAMatchTag::Brivla),
            WordKind::Cmevla => Some(SAMatchTag::Cmevla),
        },
        other => visible_selmaho(&other).map(SAMatchTag::Selmaho),
    }
}

fn find_nth_matching_word(
    options: &MorphologyOptions,
    count: usize,
    target: SAMatchTag<'_>,
    acc: &[WordWithModifiers],
) -> Option<Vec<WordWithModifiers>> {
    let mut remaining = count;
    for (index, token) in acc.iter().enumerate().rev() {
        if sa_match_tag(options, token) == Some(target) {
            remaining -= 1;
            if remaining == 0 {
                return Some(acc[..index].to_vec());
            }
        }
    }
    None
}

fn visible_selmaho(word_like: &WordLike) -> Option<&'static str> {
    match word_like {
        WordLike::Bare { word } if word.kind == WordKind::Cmavo => selmaho(&word.phonemes),
        WordLike::ZoQuote { .. } => Some("ZO"),
        WordLike::ZoiQuote { zoi, .. } => selmaho(&zoi.phonemes),
        WordLike::LohuQuote { .. } => Some("LOhU"),
        WordLike::SingleWordQuote { marker, .. } => selmaho(&marker.phonemes),
        WordLike::Letter { .. } => Some("BU"),
        WordLike::ZeiLujvo { .. } => Some("ZEI"),
        _ => None,
    }
}

fn selmaho(cmavo: &str) -> Option<&'static str> {
    match canonicalize_text(cmavo).as_str() {
        "a" | "e" | "ji" | "o" | "u" => Some("A"),
        "ba" | "ca" | "pu" => Some("PU"),
        "ba'e" | "za'e" => Some("BAhE"),
        "be" => Some("BE"),
        "bei" => Some("BEI"),
        "be'o" => Some("BEhO"),
        "by" | "cy" | "dy" | "fy" | "gy" | "jy" | "ky" | "ly" | "my" | "ny" | "py" | "ry"
        | "sy" | "ty" | "vy" | "xy" | "y'y" | "zy" => Some("BY"),
        "cai" | "cu'i" | "pei" | "ru'e" | "sai" => Some("CAI"),
        "ce'e" => Some("CEhE"),
        "co" => Some("CO"),
        "coi" | "co'o" | "je'e" | "ju'i" | "mi'e" | "mu'o" | "ta'a" => Some("COI"),
        "cu" => Some("CU"),
        "da" | "de" | "di" | "do" | "fo'a" | "fo'e" | "fo'i" | "fo'o" | "fo'u" | "ko" | "ko'a"
        | "ko'e" | "ko'i" | "ko'o" | "ko'u" | "ma" | "mi" | "ra" | "ri" | "ru" | "ta" | "ti"
        | "tu" | "vo'a" | "vo'e" | "vo'i" | "vo'o" | "vo'u" | "zo'e" | "zu'i" => Some("KOhA"),
        "fa" | "fai" | "fe" | "fi" | "fi'a" | "fo" | "fu" => Some("FA"),
        "fa'o" => Some("FAhO"),
        "fu'a" => Some("FUhA"),
        "ge'a" | "pa'i" | "re'a" | "sa'i" | "sa'o" | "su'i" | "te'a" | "va'a" | "vu'u" | "cu'a"
        | "de'o" | "fe'a" | "fe'i" | "fu'u" | "ju'u" | "ne'o" | "pi'a" | "pi'i" | "ri'o" => {
            Some("VUhU")
        }
        "goi" | "ne" | "no'u" | "pe" | "po" | "po'e" | "po'u" => Some("GOI"),
        "i" => Some("I"),
        "ja" | "je" | "jo" | "ju" => Some("JA"),
        "jei" | "ka" | "li'i" | "mu'e" | "ni" | "nu" | "pu'u" | "si'o" | "su'u" | "za'i"
        | "zu'o" => Some("NU"),
        "jo'i" => Some("JOhI"),
        "joi" | "ce" | "ce'o" | "fa'u" | "jo'e" | "jo'u" | "ju'e" | "ku'a" | "pi'u" => Some("JOI"),
        "la" | "lai" | "la'i" => Some("LA"),
        "le" | "lei" | "le'e" | "le'i" | "lo" | "loi" | "lo'e" | "lo'i" => Some("LE"),
        "li" | "me'o" => Some("LI"),
        "la'e" | "lu'a" | "lu'e" | "lu'i" | "lu'o" | "tu'a" | "vu'i" => Some("LAhE"),
        "li'u" => Some("LIhU"),
        "lo'o" => Some("LOhO"),
        "ni'o" => Some("NIhO"),
        "lu" => Some("LU"),
        "tu'e" => Some("TUhE"),
        "to" => Some("TO"),
        "toi" => Some("TOI"),
        "zo" | "ma'oi" => Some("ZO"),
        "zoi" | "la'o" | "mu'oi" => Some("ZOI"),
        "lo'u" => Some("LOhU"),
        "le'u" => Some("LEhU"),
        "bu" => Some("BU"),
        "zei" => Some("ZEI"),
        "na" | "ja'a" => Some("NA"),
        "nai" => Some("NAI"),
        "na'e" | "je'a" | "no'e" | "to'e" => Some("NAhE"),
        "noi" | "poi" | "voi" => Some("NOI"),
        "pa" | "re" | "ci" | "vo" | "mu" | "xa" | "ze" | "bi" | "so" | "no" | "pi" => Some("PA"),
        "pe'e" => Some("PEhE"),
        "sa" => Some("SA"),
        "se" | "te" | "ve" | "xe" => Some("SE"),
        "si" => Some("SI"),
        "su" => Some("SU"),
        "va" | "vi" | "vu" => Some("VA"),
        "vau" => Some("VAU"),
        "vei" => Some("VEI"),
        "ve'o" => Some("VEhO"),
        "xi" => Some("XI"),
        "y" => Some("Y"),
        "za" | "zi" | "zu" => Some("ZI"),
        "zo'u" => Some("ZOhU"),
        _ => None,
    }
}

fn apply_passes(words: Vec<WordWithModifiers>) -> Vec<WordWithModifiers> {
    pass_ui(pass_bahe(words))
}

fn pass_bahe(words: Vec<WordWithModifiers>) -> Vec<WordWithModifiers> {
    let mut reversed: VecDeque<_> = words.into_iter().rev().collect();
    let mut out = Vec::new();
    while let Some(word) = reversed.pop_front() {
        if reversed.front().is_some_and(|next| {
            is_simple_cmavo_text(next, "ba'e") || is_simple_cmavo_text(next, "za'e")
        }) && let Some(bahe_token) = reversed.pop_front()
            && let Some(bahe) = get_modifier_word(&bahe_token)
        {
            reversed.push_front(WordWithModifiers::Emphasized {
                bahe: Box::new(bahe),
                word_like: Box::new(get_word_like(&word)),
            });
        } else {
            out.push(word);
        }
    }
    out.reverse();
    out
}

fn pass_ui(words: Vec<WordWithModifiers>) -> Vec<WordWithModifiers> {
    let mut out: Vec<WordWithModifiers> = Vec::new();
    let mut iter = words.into_iter().peekable();
    while let Some(word) = iter.next() {
        if is_indicator(&word) {
            let indicator = get_modifier_word(&word);
            let nai = if iter
                .peek()
                .is_some_and(|next| is_simple_cmavo_text(next, "nai"))
            {
                iter.next()
                    .and_then(|next| get_modifier_word(&next))
                    .map(Box::new)
            } else {
                None
            };
            if let Some(indicator) = indicator {
                if let Some(prev) = out.pop() {
                    out.push(WordWithModifiers::WithIndicator {
                        base: Box::new(prev),
                        indicator: Box::new(indicator),
                        nai,
                    });
                } else if nai.is_some() {
                    out.push(WordWithModifiers::StandaloneIndicator {
                        indicator: Box::new(indicator),
                        nai,
                    });
                } else {
                    out.push(word);
                }
            } else {
                out.push(word);
            }
        } else {
            out.push(word);
        }
    }
    out
}

fn get_modifier_word(word: &WordWithModifiers) -> Option<Word> {
    match word {
        WordWithModifiers::BaseWord { word_like } => match word_like.as_ref() {
            WordLike::Bare { word } => Some((**word).clone()),
            _ => None,
        },
        WordWithModifiers::StandaloneIndicator { indicator, .. } => Some((**indicator).clone()),
        WordWithModifiers::Emphasized { bahe, .. } => Some((**bahe).clone()),
        WordWithModifiers::WithIndicator { indicator, .. } => Some((**indicator).clone()),
        WordWithModifiers::NotEof => None,
    }
}

fn is_indicator(word: &WordWithModifiers) -> bool {
    match extract_word(word) {
        Some(word) if word.kind == WordKind::Cmavo => {
            INDICATORS.contains(&canonicalize_text(&word.phonemes).as_str())
                || is_y_word_text(&word.phonemes)
        }
        _ => false,
    }
}

const INDICATORS: &[&str] = &[
    "i'a", "ie", "a'e", "u'i", "i'o", "i'e", "a'a", "ia", "o'i", "o'e", "e'e", "oi", "uo", "e'i",
    "u'o", "au", "ua", "a'i", "i'u", "ii", "u'a", "ui", "a'o", "ai", "a'u", "iu", "ei", "o'o",
    "e'a", "uu", "o'a", "o'u", "u'u", "e'o", "io", "e'u", "ue", "i'i", "u'e", "ba'a", "ja'o",
    "ca'e", "su'a", "ti'e", "ka'u", "se'o", "za'a", "pe'i", "ru'a", "ju'a", "ta'o", "ra'u", "li'a",
    "ba'u", "mu'a", "do'a", "to'u", "va'i", "pa'e", "zu'u", "sa'e", "la'a", "ke'u", "sa'u", "da'i",
    "je'u", "sa'a", "kau", "ta'u", "na'i", "jo'a", "bi'u", "li'o", "li'oi", "pau", "mi'u", "ku'i",
    "ji'a", "si'a", "po'o", "pe'a", "ro'i", "ro'e", "ro'o", "ro'u", "ro'a", "re'e", "le'o", "ju'o",
    "fu'i", "dai", "ga'i", "zo'o", "be'u", "ri'e", "se'i", "se'a", "vu'e", "ki'a", "xu", "ge'e",
    "bu'o", "ai'i", "e'ei", "fu'au", "ju'oi", "ko'oi", "oi'a", "si'au", "ue'i", "xo'o", "fu'e",
    "fu'o", "cai", "pei", "cu'i", "sai", "ru'e", "y", "da'o",
];

fn text_chars(text: &str) -> Vec<char> {
    text.chars().collect()
}

fn starts_with_nucleus(chars: &[char], start: usize) -> bool {
    if start >= chars.len() {
        return false;
    }
    parse_diphthong(chars, start).is_some() || parse_single_vowel(chars, start).is_some()
}

fn parse_diphthong(chars: &[char], start: usize) -> Option<(String, usize)> {
    let first = *chars.get(start)?;
    let second = *chars.get(start + 1)?;
    let semivowel = match (base_vowel(first)?, second) {
        ('a', 'i' | 'í' | 'ĭ') | ('e', 'i' | 'í' | 'ĭ') | ('o', 'i' | 'í' | 'ĭ') => 'ĭ',
        ('a', 'u' | 'ú' | 'ŭ') => 'ŭ',
        _ => return None,
    };
    let end = start + 2;
    if starts_with_nucleus(chars, end) {
        return None;
    }
    Some((format!("{}{}", normalize_vowel(first), semivowel), end))
}

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

fn is_vowel(value: char) -> bool {
    base_vowel(value).is_some()
}

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

    #[test]
    fn segments_ordinary_sentence() {
        let words =
            segment_words_with_modifiers("mi klama do", &MorphologyOptions::default(), None)
                .expect("valid morphology");

        assert_eq!(bare_phonemes(&words), ["mi", "klama", "do"]);
        assert_eq!(bare_span(&words[1]).map(|span| span.byte_start), Some(3));
        assert_eq!(bare_span(&words[1]).map(|span| span.byte_end), Some(8));
    }

    #[test]
    fn parses_zo_quote_as_one_wordlike() {
        let words = segment_words_with_modifiers("zo si", &MorphologyOptions::default(), None)
            .expect("valid morphology");

        assert_eq!(words.len(), 1);
        let WordWithModifiers::BaseWord { word_like } = &words[0] else {
            panic!("expected base word");
        };
        let WordLike::ZoQuote { zo, word } = word_like.as_ref() else {
            panic!("expected ZO quote");
        };
        assert_eq!(zo.phonemes, "zo");
        assert_eq!(word.phonemes, "si");
    }

    #[test]
    fn parses_zoi_quote_as_one_wordlike() {
        let words =
            segment_words_with_modifiers("zoi gy broda gy", &MorphologyOptions::default(), None)
                .expect("valid morphology");

        assert_eq!(words.len(), 1);
        let WordWithModifiers::BaseWord { word_like } = &words[0] else {
            panic!("expected base word");
        };
        let WordLike::ZoiQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        } = word_like.as_ref()
        else {
            panic!("expected ZOI quote");
        };
        assert_eq!(zoi.phonemes, "zoĭ");
        assert_eq!(opening_delimiter.phonemes, "gy");
        assert_eq!(opening_delimiter.span.byte_start, 4);
        assert_eq!(opening_delimiter.span.byte_end, 6);
        assert_eq!(quoted_text.byte_start, 6);
        assert_eq!(quoted_text.byte_end, 12);
        assert_eq!(closing_delimiter.phonemes, "gy");
        assert_eq!(closing_delimiter.span.byte_start, 13);
        assert_eq!(closing_delimiter.span.byte_end, 15);
    }

    #[test]
    fn reports_unclosed_zoi_quote() {
        let error =
            segment_words_with_modifiers("zoi gy broda", &MorphologyOptions::default(), None)
                .expect_err("unclosed ZOI should fail");

        assert!(error.to_string().contains("expected closing delimiter"));
    }

    fn bare_phonemes(words: &[WordWithModifiers]) -> Vec<&str> {
        words
            .iter()
            .map(|word| bare_word(word).expect("bare word").phonemes.as_str())
            .collect()
    }

    fn bare_span(word: &WordWithModifiers) -> Option<&SourceSpan> {
        bare_word(word).map(|word| &word.span)
    }

    fn bare_word(word: &WordWithModifiers) -> Option<&Word> {
        match word {
            WordWithModifiers::BaseWord { word_like } => match word_like.as_ref() {
                WordLike::Bare { word } => Some(word),
                _ => None,
            },
            _ => None,
        }
    }
}
