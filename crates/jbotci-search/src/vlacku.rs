use bityzba::{data, invariant, requires};
use jbotci_dictionary::{Dictionary, DictionaryEntry, Keyword, WordType, normalize_lookup_query};
use jbotci_jvozba::{LujvoDecomposition, decompose_lujvo_like};
use jbotci_morphology::{
    Jvopau, Verbatim, Word, WordKind, WordLike, WordLikeData, canonicalize_text,
    segment_words_with_modifiers,
};

use crate::phonetic::{
    PhoneticError, aline_phonetic_similarity, compare_similarity_then_index, lojban_text_to_ipa,
    sound_query_to_token_sequence, tokenize_ipa_text,
};

pub const DEFAULT_VLACKU_RESULT_COUNT: usize = 20;
pub const OFFICIAL_WORD_VOTE_THRESHOLD: i32 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Valsi(_) => true)]
#[invariant(::Rafsi(_) => true)]
#[invariant(::Lujvo(_) => true)]
#[invariant(::Glob(_) => true)]
#[invariant(::Sound(_) => true)]
#[invariant(::Meaning(_) => true)]
pub enum VlackuRequest {
    Valsi(String),
    Rafsi(String),
    Lujvo(String),
    Glob(String),
    Sound(String),
    Meaning(String),
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub struct VlackuSearchOptions {
    pub count: usize,
    pub word_types: Vec<String>,
    pub min_votes: Option<i32>,
    pub min_similarity: Option<f32>,
    pub decompose_lujvo: bool,
}

impl Default for VlackuSearchOptions {
    #[requires(true)]
    #[ensures(ret.count == DEFAULT_VLACKU_RESULT_COUNT)]
    fn default() -> Self {
        Self {
            count: DEFAULT_VLACKU_RESULT_COUNT,
            word_types: Vec::new(),
            min_votes: None,
            min_similarity: None,
            decompose_lujvo: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[invariant(true)]
pub enum VlackuOutcome {
    Found,
    ValidMissing,
    Invalid,
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub struct VlackuSearchOutput {
    pub cards: Vec<VlackuCard>,
    pub outcome: VlackuOutcome,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub struct VlackuCard {
    pub word: String,
    pub word_type: String,
    pub selmaho: Option<String>,
    pub similarity: Option<f32>,
    pub votes: Option<i32>,
    pub rafsi: Vec<String>,
    pub glosses: Vec<String>,
    pub definition: String,
    pub notes: String,
    pub decomposition: Vec<VlackuCompositionPiece>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct VlackuCompositionPiece {
    pub kind: VlackuCompositionKind,
    pub surface: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
#[invariant(true)]
pub struct ParsedWordDictionaryMatch {
    pub lookup_text: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub char_start: usize,
    pub char_end: usize,
    pub cards: Vec<VlackuCard>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct ParsedWordLookupTarget {
    lookup_text: String,
    is_lujvo: bool,
    byte_start: usize,
    byte_end: usize,
    char_start: usize,
    char_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Rafsi => true)]
#[invariant(::Hyphen => true)]
pub enum VlackuCompositionKind {
    Rafsi,
    Hyphen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct WordClassification {
    pub word: String,
    pub word_type: String,
    pub selmaho: Option<String>,
}

#[requires(true)]
#[ensures(true)]
pub fn run_vlacku_requests(
    dictionary: &Dictionary<'_>,
    requests: &[VlackuRequest],
    options: &VlackuSearchOptions,
) -> VlackuSearchOutput {
    let mut cards = Vec::new();
    let mut diagnostics = Vec::new();
    let mut outcome = VlackuOutcome::Found;

    for request in requests {
        let result = run_single_request(dictionary, request, options);
        outcome = outcome.max(result.outcome);
        diagnostics.extend(result.diagnostics);
        cards.extend(result.cards);
    }

    VlackuSearchOutput {
        cards,
        outcome,
        diagnostics,
    }
}

#[requires(true)]
#[ensures(true)]
pub fn dictionary_cards_for_word_likes(
    dictionary: &Dictionary<'_>,
    words: &[WordLike],
) -> Vec<VlackuCard> {
    let mut cards = Vec::new();
    for parsed_match in dictionary_matches_for_word_likes(dictionary, words) {
        extend_unique_cards(&mut cards, parsed_match.cards);
    }
    cards
}

#[requires(true)]
#[ensures(true)]
pub fn dictionary_matches_for_word_likes(
    dictionary: &Dictionary<'_>,
    words: &[WordLike],
) -> Vec<ParsedWordDictionaryMatch> {
    let mut matches = Vec::new();
    for word_like in words {
        for target in dictionary_lookup_targets(word_like) {
            let cards = dictionary_cards_for_lookup_target(dictionary, &target);
            if cards.is_empty() {
                continue;
            }
            matches.push(ParsedWordDictionaryMatch {
                lookup_text: target.lookup_text,
                byte_start: target.byte_start,
                byte_end: target.byte_end,
                char_start: target.char_start,
                char_end: target.char_end,
                cards,
            });
        }
    }
    matches
}

#[requires(true)]
#[ensures(true)]
pub fn normalize_word_type_filter(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace(' ', "-")
}

#[requires(true)]
#[ensures(true)]
pub fn matches_word_type_filter(wanted: &str, normalized_type: &str) -> bool {
    wanted == normalized_type
        || (wanted == "cmavo" && is_cmavo_like(normalized_type))
        || (wanted == "letteral" && is_letteral_like(normalized_type))
        || (wanted == "cmevla" && is_cmevla_like(normalized_type))
        || (wanted == "gismu" && is_gismu_like(normalized_type))
        || (wanted == "fu'ivla" && is_fuhivla_like(normalized_type))
        || (wanted == "lujvo" && is_lujvo_like(normalized_type))
        || (wanted == "brivla" && is_brivla_like(normalized_type))
}

#[requires(true)]
#[ensures(true)]
pub fn grouped_word_type_filter_key(normalized_type: &str) -> String {
    if is_cmavo_like(normalized_type) {
        "cmavo".to_owned()
    } else if is_letteral_like(normalized_type) {
        "letteral".to_owned()
    } else if is_cmevla_like(normalized_type) {
        "cmevla".to_owned()
    } else if is_gismu_like(normalized_type) {
        "gismu".to_owned()
    } else if is_fuhivla_like(normalized_type) {
        "fu'ivla".to_owned()
    } else if is_lujvo_like(normalized_type) {
        "lujvo".to_owned()
    } else {
        normalized_type.to_owned()
    }
}

#[requires(true)]
#[ensures(true)]
pub fn is_cmavo_like(normalized_type: &str) -> bool {
    normalized_type == "cmavo"
        || normalized_type.starts_with("cmavo-")
        || normalized_type == "experimental-cmavo"
        || normalized_type == "obsolete-cmavo"
}

#[requires(true)]
#[ensures(true)]
pub fn is_letteral_like(normalized_type: &str) -> bool {
    normalized_type == "bu-letteral" || normalized_type == "letteral"
}

#[requires(true)]
#[ensures(true)]
pub fn is_cmevla_like(normalized_type: &str) -> bool {
    normalized_type == "cmevla" || normalized_type == "obsolete-cmevla"
}

#[requires(true)]
#[ensures(true)]
pub fn is_gismu_like(normalized_type: &str) -> bool {
    normalized_type == "gismu" || normalized_type == "experimental-gismu"
}

#[requires(true)]
#[ensures(true)]
pub fn is_fuhivla_like(normalized_type: &str) -> bool {
    normalized_type == "fu'ivla" || normalized_type == "obsolete-fu'ivla"
}

#[requires(true)]
#[ensures(true)]
pub fn is_lujvo_like(normalized_type: &str) -> bool {
    normalized_type == "lujvo"
        || normalized_type == "zei-lujvo"
        || normalized_type == "obsolete-zei-lujvo"
}

#[requires(true)]
#[ensures(true)]
pub fn is_brivla_like(normalized_type: &str) -> bool {
    is_gismu_like(normalized_type)
        || is_lujvo_like(normalized_type)
        || is_fuhivla_like(normalized_type)
}

#[requires(true)]
#[ensures(value > OFFICIAL_WORD_VOTE_THRESHOLD -> ret == "∞")]
#[ensures(value <= OFFICIAL_WORD_VOTE_THRESHOLD -> ret.starts_with('+') == (value > 0))]
pub fn format_votes(value: i32) -> String {
    if value > OFFICIAL_WORD_VOTE_THRESHOLD {
        "∞".to_owned()
    } else if value > 0 {
        format!("+{value}")
    } else {
        value.to_string()
    }
}

#[requires(true)]
#[ensures(true)]
fn run_single_request(
    dictionary: &Dictionary<'_>,
    request: &VlackuRequest,
    options: &VlackuSearchOptions,
) -> VlackuSearchOutput {
    match request {
        VlackuRequest::Valsi(query) => cards_for_valsi(dictionary, query, options),
        VlackuRequest::Rafsi(query) => cards_for_rafsi(dictionary, query, options),
        VlackuRequest::Lujvo(query) => cards_for_lujvo(dictionary, query, options),
        VlackuRequest::Glob(pattern) => cards_for_glob(dictionary, pattern, options),
        VlackuRequest::Sound(query) => cards_for_sound(dictionary, query, options),
        VlackuRequest::Meaning(_) => {
            invalid_output("Semantic vlacku search requires an embedding backend.".to_owned())
        }
    }
}

#[requires(!target.lookup_text.is_empty())]
#[ensures(true)]
fn dictionary_cards_for_lookup_target(
    dictionary: &Dictionary<'_>,
    target: &ParsedWordLookupTarget,
) -> Vec<VlackuCard> {
    let options = parsed_word_vlacku_options();
    let exact_entries = dictionary
        .lookup_words(&target.lookup_text)
        .collect::<Vec<_>>();
    let exact_definition_found = exact_entries
        .iter()
        .any(|entry| !entry.definition.trim().is_empty());
    let output = if target.is_lujvo && !exact_definition_found {
        cards_for_lujvo(dictionary, &target.lookup_text, &options)
    } else {
        cards_for_valsi(dictionary, &target.lookup_text, &options)
    };
    output.cards
}

#[requires(true)]
#[ensures(ret.count == usize::MAX)]
fn parsed_word_vlacku_options() -> VlackuSearchOptions {
    VlackuSearchOptions {
        count: usize::MAX,
        word_types: Vec::new(),
        min_votes: None,
        min_similarity: None,
        decompose_lujvo: false,
    }
}

#[requires(true)]
#[ensures(true)]
fn extend_unique_cards(target: &mut Vec<VlackuCard>, source: Vec<VlackuCard>) {
    for card in source {
        if !target.iter().any(|existing| existing == &card) {
            target.push(card);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn dictionary_lookup_targets(word_like: &WordLike) -> Vec<ParsedWordLookupTarget> {
    let mut targets = Vec::new();
    push_dictionary_lookup_targets(word_like, &mut targets);
    targets
}

#[requires(true)]
#[ensures(true)]
fn push_dictionary_lookup_targets(word_like: &WordLike, targets: &mut Vec<ParsedWordLookupTarget>) {
    match word_like.as_data() {
        data!(WordLike::Bare(word)) => {
            push_word_lookup_target(word, targets);
        }
        data!(WordLike::ZoQuote { zo, word }) => {
            push_word_lookup_target(zo, targets);
            push_word_lookup_target(word, targets);
        }
        data!(WordLike::ZoiQuote {
            zoi,
            opening_delimiter,
            closing_delimiter,
            ..
        }) => {
            push_word_lookup_target(zoi, targets);
            push_word_lookup_target(opening_delimiter, targets);
            push_word_lookup_target(closing_delimiter, targets);
        }
        data!(WordLike::LohuQuote {
            lohu,
            quoted_words,
            lehu,
        }) => {
            push_word_lookup_target(lohu, targets);
            for word in quoted_words {
                push_word_lookup_target(word, targets);
            }
            push_word_lookup_target(lehu, targets);
        }
        data!(WordLike::SingleWordQuote { marker, .. }) => {
            push_word_lookup_target(marker, targets);
        }
        data!(WordLike::Letter { .. }) | data!(WordLike::ZeiLujvo { .. }) => {
            if let Some(target) = word_like_lookup_target(word_like) {
                targets.push(target);
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn push_word_lookup_target(word: &Word, targets: &mut Vec<ParsedWordLookupTarget>) {
    targets.push(ParsedWordLookupTarget {
        lookup_text: word_lookup_text(word),
        is_lujvo: word.kind() == WordKind::Lujvo,
        byte_start: word.span().byte_start,
        byte_end: word.span().byte_end,
        char_start: word.span().char_start,
        char_end: word.span().char_end,
    });
}

#[requires(true)]
#[ensures(ret.as_ref().is_some_and(|target| !target.lookup_text.is_empty()) || ret.is_none())]
fn word_like_lookup_target(word_like: &WordLike) -> Option<ParsedWordLookupTarget> {
    let lookup_text = word_like_lookup_text(word_like)?;
    let spans = word_like.source_spans();
    let first = spans.first()?;
    let mut byte_start = first.byte_start;
    let mut byte_end = first.byte_end;
    let mut char_start = first.char_start;
    let mut char_end = first.char_end;
    for span in spans.iter().skip(1) {
        byte_start = byte_start.min(span.byte_start);
        byte_end = byte_end.max(span.byte_end);
        char_start = char_start.min(span.char_start);
        char_end = char_end.max(span.char_end);
    }
    Some(ParsedWordLookupTarget {
        lookup_text,
        is_lujvo: matches!(word_like.as_data(), data!(WordLike::ZeiLujvo { .. })),
        byte_start,
        byte_end,
        char_start,
        char_end,
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
fn word_like_lookup_text(word_like: &WordLike) -> Option<String> {
    match word_like.as_data() {
        data!(WordLike::Bare(word)) => Some(word_lookup_text(word)),
        data!(WordLike::Letter { base, .. }) => letteral_lookup_text(base),
        data!(WordLike::ZeiLujvo { left, right, .. }) => Some(format!(
            "{} zei {}",
            word_like_lookup_text(left)?,
            word_lookup_text(right)
        )),
        data!(WordLike::ZoQuote { .. })
        | data!(WordLike::ZoiQuote { .. })
        | data!(WordLike::LohuQuote { .. })
        | data!(WordLike::SingleWordQuote { .. }) => None,
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn word_lookup_text(word: &Word) -> String {
    canonicalize_text(word.phonemes().as_str())
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
fn letteral_lookup_text(base: &WordLike) -> Option<String> {
    let base_text = word_like_lookup_text(base)?;
    let normalized = normalize_lookup_query(&base_text);
    let mut chars = normalized.chars();
    let first = chars.next()?;
    if chars.next().is_none() {
        if is_lojban_consonant(first) {
            return Some(format!("{first}y"));
        }
        if is_lojban_vowel(first) {
            return Some(format!("{first}bu"));
        }
    }
    Some(format!("{normalized} bu"))
}

#[requires(true)]
#[ensures(true)]
fn cards_for_valsi(
    dictionary: &Dictionary<'_>,
    query: &str,
    options: &VlackuSearchOptions,
) -> VlackuSearchOutput {
    let entries = dictionary.lookup_words(query).collect::<Vec<_>>();
    if !entries.is_empty() {
        return found_or_missing(cards_with_optional_lujvo_sources(
            dictionary,
            query,
            entries
                .into_iter()
                .map(|entry| {
                    dictionary_entry_card(dictionary, entry, Some(1.0), options.decompose_lujvo)
                })
                .collect(),
            options,
        ));
    }

    if let Some(entries) = resolve_segmented_lookup(dictionary, query) {
        let cards = filter_and_limit(
            entries
                .into_iter()
                .map(|entry| {
                    dictionary_entry_card(dictionary, entry, Some(1.0), options.decompose_lujvo)
                })
                .collect(),
            options,
            false,
        );
        return found_or_missing(cards);
    }

    missing_exact_output(dictionary, query, options)
}

#[requires(true)]
#[ensures(true)]
fn cards_for_rafsi(
    dictionary: &Dictionary<'_>,
    query: &str,
    options: &VlackuSearchOptions,
) -> VlackuSearchOutput {
    let cards = filter_and_limit(
        dictionary
            .lookup_rafsi(query)
            .map(|matched| {
                dictionary_entry_card(
                    dictionary,
                    matched.entry,
                    Some(1.0),
                    options.decompose_lujvo,
                )
            })
            .collect(),
        options,
        false,
    );
    found_or_missing(cards)
}

#[requires(true)]
#[ensures(true)]
fn cards_for_lujvo(
    dictionary: &Dictionary<'_>,
    query: &str,
    options: &VlackuSearchOptions,
) -> VlackuSearchOutput {
    let normalized = normalize_lookup_query(query);
    let decomposition = decompose_lujvo_like(dictionary, query);
    let exact_entries = dictionary.lookup_words(query).collect::<Vec<_>>();
    let exact_found = !exact_entries.is_empty();
    let mut cards = if exact_entries.is_empty() {
        match classify_exact_word(query, &normalized) {
            Some(classification) => vec![unknown_card(classification, decomposition.as_ref())],
            None => {
                return invalid_output(format!("Invalid Lojban word: {query}"));
            }
        }
    } else {
        exact_entries
            .into_iter()
            .map(|entry| entry_card_with_decomposition(entry, Some(1.0), decomposition.as_ref()))
            .collect()
    };

    if let Some(decomposition) = &decomposition {
        cards.extend(decomposition.source_words.iter().filter_map(|source_word| {
            dictionary.lookup_word(source_word).map(|entry| {
                dictionary_entry_card(dictionary, entry, Some(1.0), options.decompose_lujvo)
            })
        }));
    }

    let cards = filter_and_limit(cards, options, false);
    let outcome = if exact_found {
        VlackuOutcome::Found
    } else if cards.is_empty() {
        VlackuOutcome::ValidMissing
    } else {
        VlackuOutcome::ValidMissing
    };
    VlackuSearchOutput {
        cards,
        outcome,
        diagnostics: Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn cards_for_glob(
    dictionary: &Dictionary<'_>,
    pattern: &str,
    options: &VlackuSearchOptions,
) -> VlackuSearchOutput {
    let compiled = match compile_glob_pattern(pattern) {
        Ok(compiled) => compiled,
        Err(message) => return invalid_output(message),
    };
    let cards = filter_and_limit(
        dictionary
            .entries()
            .iter()
            .filter(|entry| compiled.matches(&glob_target_key(entry.word)))
            .map(|entry| {
                dictionary_entry_card(dictionary, entry, Some(1.0), options.decompose_lujvo)
            })
            .collect(),
        options,
        false,
    );
    found_or_missing(cards)
}

#[requires(true)]
#[ensures(true)]
fn cards_for_sound(
    dictionary: &Dictionary<'_>,
    query: &str,
    options: &VlackuSearchOptions,
) -> VlackuSearchOutput {
    let query_sound = match sound_query_to_token_sequence(query) {
        Ok(sequence) => sequence,
        Err(error) => return invalid_output(error.to_string()),
    };

    let mut scored = dictionary
        .entries()
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            dictionary_word_sound_entry(entry).map(|entry_sound| {
                (
                    index,
                    aline_phonetic_similarity(&query_sound, &entry_sound) as f32,
                    entry,
                )
            })
        })
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| {
        compare_similarity_then_index((left.0, left.1.into()), (right.0, right.1.into()))
    });

    let min_similarity = options.min_similarity.unwrap_or(0.0) / 100.0;
    let cards = filter_and_limit(
        scored
            .into_iter()
            .filter(|(_, similarity, _)| *similarity >= min_similarity)
            .map(|(_, similarity, entry)| {
                dictionary_entry_card(dictionary, entry, Some(similarity), options.decompose_lujvo)
            })
            .collect(),
        options,
        true,
    );
    found_or_missing(cards)
}

#[requires(true)]
#[ensures(true)]
fn found_or_missing(cards: Vec<VlackuCard>) -> VlackuSearchOutput {
    let outcome = if cards.is_empty() {
        VlackuOutcome::ValidMissing
    } else {
        VlackuOutcome::Found
    };
    VlackuSearchOutput {
        cards,
        outcome,
        diagnostics: Vec::new(),
    }
}

#[requires(true)]
#[ensures(ret.outcome == VlackuOutcome::Invalid)]
fn invalid_output(message: String) -> VlackuSearchOutput {
    VlackuSearchOutput {
        cards: Vec::new(),
        outcome: VlackuOutcome::Invalid,
        diagnostics: vec![message],
    }
}

#[requires(true)]
#[ensures(true)]
fn missing_exact_output(
    dictionary: &Dictionary<'_>,
    query: &str,
    options: &VlackuSearchOptions,
) -> VlackuSearchOutput {
    let normalized = normalize_lookup_query(query);
    match classify_exact_word(query, &normalized) {
        Some(classification) => {
            let decomposition = options
                .decompose_lujvo
                .then(|| decompose_lujvo_like(dictionary, query))
                .flatten();
            let cards = cards_with_optional_lujvo_sources(
                dictionary,
                query,
                vec![unknown_card(classification, decomposition.as_ref())],
                options,
            );
            VlackuSearchOutput {
                cards,
                outcome: VlackuOutcome::ValidMissing,
                diagnostics: Vec::new(),
            }
        }
        None => invalid_output(format!("Invalid Lojban word: {query}")),
    }
}

#[requires(true)]
#[ensures(true)]
fn cards_with_optional_lujvo_sources(
    dictionary: &Dictionary<'_>,
    query: &str,
    mut cards: Vec<VlackuCard>,
    options: &VlackuSearchOptions,
) -> Vec<VlackuCard> {
    if options.decompose_lujvo
        && let Some(decomposition) = decompose_lujvo_like(dictionary, query)
    {
        cards.extend(decomposition.source_words.iter().filter_map(|source_word| {
            dictionary.lookup_word(source_word).map(|entry| {
                dictionary_entry_card(dictionary, entry, Some(1.0), options.decompose_lujvo)
            })
        }));
    }
    filter_and_limit(cards, options, false)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|entries| entries.len() > 1))]
fn resolve_segmented_lookup<'lookup, 'a>(
    dictionary: &'lookup Dictionary<'a>,
    raw_query: &str,
) -> Option<Vec<&'lookup DictionaryEntry<'a>>> {
    let words = segment_words_with_modifiers(raw_query).ok()?;
    if words.len() <= 1 {
        return None;
    }

    let mut entries = Vec::with_capacity(words.len());
    for word in &words {
        let surface = flattened_word_like_phonemes(word);
        entries.push(dictionary.lookup_word(&surface)?);
    }
    Some(entries)
}

#[requires(true)]
#[ensures(true)]
fn flattened_word_like_phonemes(word_like: &WordLike) -> String {
    let mut output = String::new();
    append_word_like_phonemes(word_like, &mut output);
    output
}

#[requires(true)]
#[ensures(true)]
fn append_word_like_phonemes(word_like: &WordLike, output: &mut String) {
    match word_like.as_data() {
        data!(WordLike::Bare(word)) => append_word_phonemes(word, output),
        data!(WordLike::ZoQuote { zo, word }) => {
            append_word_phonemes(zo, output);
            append_surface_chunk(
                output,
                &quoted_words_phonemes(std::iter::once(word.as_ref())),
            );
        }
        data!(WordLike::ZoiQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => {
            append_word_phonemes(zoi, output);
            append_word_phonemes(opening_delimiter, output);
            append_surface_chunk(output, &quoted_text_phonemes(quoted_text));
            append_word_phonemes(closing_delimiter, output);
        }
        data!(WordLike::LohuQuote {
            lohu,
            quoted_words,
            lehu,
        }) => {
            append_word_phonemes(lohu, output);
            append_surface_chunk(output, &quoted_words_phonemes(quoted_words.iter()));
            append_word_phonemes(lehu, output);
        }
        data!(WordLike::SingleWordQuote {
            marker,
            quoted_text,
        }) => {
            append_word_phonemes(marker, output);
            append_surface_chunk(output, &quoted_text_phonemes(quoted_text));
        }
        data!(WordLike::Letter { base, bu }) => {
            append_word_like_phonemes(base, output);
            append_word_phonemes(bu, output);
        }
        data!(WordLike::ZeiLujvo { left, zei, right }) => {
            append_word_like_phonemes(left, output);
            append_word_phonemes(zei, output);
            append_word_phonemes(right, output);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn append_word_phonemes(word: &Word, output: &mut String) {
    append_surface_chunk(output, word.phonemes().as_str());
}

#[requires(true)]
#[ensures(true)]
fn append_surface_chunk(output: &mut String, chunk: &str) {
    if !output.is_empty()
        && !ends_with_visible_pause_dot(output)
        && !starts_with_visible_pause_dot(chunk)
    {
        output.push('-');
    }
    output.push_str(chunk);
}

#[requires(true)]
#[ensures(ret.starts_with('«') && ret.ends_with('»'))]
fn quoted_words_phonemes<'a>(words: impl Iterator<Item = &'a Word>) -> String {
    format!(
        "«{}»",
        words
            .map(|word| word.phonemes().as_str().to_owned())
            .collect::<Vec<_>>()
            .join(" ")
    )
}

#[requires(true)]
#[ensures(ret.starts_with('«') && ret.ends_with('»'))]
fn quoted_text_phonemes(text: &Verbatim) -> String {
    format!("«{}»", drop_leading_zoi_separator(&text.text))
}

#[requires(true)]
#[ensures(true)]
fn drop_leading_zoi_separator(text: &str) -> &str {
    match text.chars().next() {
        Some(first) if first.is_whitespace() => &text[first.len_utf8()..],
        _ => text,
    }
}

#[requires(true)]
#[ensures(true)]
fn starts_with_visible_pause_dot(text: &str) -> bool {
    text.chars().next().is_some_and(is_visible_pause_dot)
}

#[requires(true)]
#[ensures(true)]
fn ends_with_visible_pause_dot(text: &str) -> bool {
    text.chars().next_back().is_some_and(is_visible_pause_dot)
}

#[requires(true)]
#[ensures(ret == (value == '.'))]
fn is_visible_pause_dot(value: char) -> bool {
    value == '.'
}

#[requires(true)]
#[ensures(true)]
fn classify_exact_word(raw_query: &str, normalized_query: &str) -> Option<WordClassification> {
    if normalized_query.is_empty() {
        return None;
    }
    let words = segment_words_with_modifiers(raw_query).ok()?;
    let [word_like] = words.as_slice() else {
        return None;
    };
    let word = word_like.bare_word()?;
    Some(WordClassification {
        word: normalized_query.to_owned(),
        word_type: word_kind_type_key(word.kind()).to_owned(),
        selmaho: word.selmaho().map(str::to_owned),
    })
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn word_kind_type_key(kind: WordKind) -> &'static str {
    match kind {
        WordKind::Cmavo => "cmavo",
        WordKind::Gismu => "gismu",
        WordKind::Lujvo => "lujvo",
        WordKind::Fuhivla => "fu'ivla",
        WordKind::Cmevla => "cmevla",
    }
}

#[requires(true)]
#[ensures(!ret.word.is_empty())]
pub fn dictionary_entry_card(
    dictionary: &Dictionary<'_>,
    entry: &DictionaryEntry<'_>,
    similarity: Option<f32>,
    decompose_lujvo: bool,
) -> VlackuCard {
    let decomposition = decompose_lujvo
        .then(|| decompose_lujvo_like(dictionary, entry.word))
        .flatten();
    entry_card_with_decomposition(entry, similarity, decomposition.as_ref())
}

#[requires(true)]
#[ensures(!ret.word.is_empty())]
fn entry_card_with_decomposition(
    entry: &DictionaryEntry<'_>,
    similarity: Option<f32>,
    decomposition: Option<&LujvoDecomposition<'_>>,
) -> VlackuCard {
    VlackuCard {
        word: entry.word.to_owned(),
        word_type: entry.word_type.as_str().to_owned(),
        selmaho: entry.selmaho.map(|selmaho| selmaho.0.to_owned()),
        similarity,
        votes: Some(entry.score.get().round() as i32),
        rafsi: entry.rafsi.iter().map(|rafsi| rafsi.0.to_owned()).collect(),
        glosses: entry.gloss_keywords.iter().map(format_keyword).collect(),
        definition: entry.definition.to_owned(),
        notes: entry.notes.to_owned(),
        decomposition: decomposition
            .map(composition_from_decomposition)
            .unwrap_or_default(),
    }
}

#[requires(true)]
#[ensures(!ret.word.is_empty())]
fn unknown_card(
    classification: WordClassification,
    decomposition: Option<&LujvoDecomposition<'_>>,
) -> VlackuCard {
    VlackuCard {
        word: classification.word,
        word_type: classification.word_type,
        selmaho: classification.selmaho,
        similarity: None,
        votes: None,
        rafsi: Vec::new(),
        glosses: Vec::new(),
        definition: String::new(),
        notes: String::new(),
        decomposition: decomposition
            .map(composition_from_decomposition)
            .unwrap_or_default(),
    }
}

#[requires(true)]
#[ensures(true)]
fn composition_from_decomposition(
    decomposition: &LujvoDecomposition<'_>,
) -> Vec<VlackuCompositionPiece> {
    decomposition
        .segments
        .iter()
        .map(|segment| match &segment.segment {
            Jvopau::Rafsi(phonemes) => VlackuCompositionPiece {
                kind: VlackuCompositionKind::Rafsi,
                surface: phonemes.as_str().to_owned(),
                source: segment.source.map(str::to_owned),
            },
            Jvopau::Hyphen(phonemes) => VlackuCompositionPiece {
                kind: VlackuCompositionKind::Hyphen,
                surface: phonemes.as_str().to_owned(),
                source: None,
            },
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn format_keyword(keyword: &Keyword<'_>) -> String {
    match keyword.meaning {
        Some(meaning) => format!("{} ({meaning})", keyword.word),
        None => keyword.word.to_owned(),
    }
}

#[requires(true)]
#[ensures(true)]
pub fn filter_vlacku_cards(
    cards: Vec<VlackuCard>,
    options: &VlackuSearchOptions,
    similarity_mode: bool,
) -> Vec<VlackuCard> {
    cards
        .into_iter()
        .filter(|card| passes_filters(card, options, similarity_mode))
        .take(options.count)
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn filter_and_limit(
    cards: Vec<VlackuCard>,
    options: &VlackuSearchOptions,
    similarity_mode: bool,
) -> Vec<VlackuCard> {
    filter_vlacku_cards(cards, options, similarity_mode)
}

#[requires(true)]
#[ensures(true)]
fn passes_filters(card: &VlackuCard, options: &VlackuSearchOptions, similarity_mode: bool) -> bool {
    let normalized_type = normalize_word_type_filter(&card.word_type);
    let word_type_ok = options.word_types.is_empty()
        || options
            .word_types
            .iter()
            .any(|wanted| matches_word_type_filter(wanted, &normalized_type));
    let votes_ok = options
        .min_votes
        .is_none_or(|min_votes| card.votes.unwrap_or(0) >= min_votes);
    let similarity_ok = !similarity_mode
        || options
            .min_similarity
            .is_none_or(|min_similarity| card.similarity.unwrap_or(0.0) * 100.0 >= min_similarity);
    word_type_ok && votes_ok && similarity_ok
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct GlobPattern {
    tokens: Vec<GlobToken>,
}

impl GlobPattern {
    #[requires(true)]
    #[ensures(true)]
    fn matches(&self, target: &str) -> bool {
        glob_matches_from(&self.tokens, 0, &target.chars().collect::<Vec<_>>(), 0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Literal(_) => true)]
#[invariant(::Consonant => true)]
#[invariant(::Vowel => true)]
#[invariant(::AnyOne => true)]
#[invariant(::AnyMany => true)]
enum GlobToken {
    Literal(char),
    Consonant,
    Vowel,
    AnyOne,
    AnyMany,
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|pattern| !pattern.tokens.is_empty()) || ret.is_err())]
fn compile_glob_pattern(pattern: &str) -> Result<GlobPattern, String> {
    let mut tokens = Vec::new();
    for raw in pattern.chars() {
        match raw {
            'C' => tokens.push(GlobToken::Consonant),
            'V' => tokens.push(GlobToken::Vowel),
            '?' => tokens.push(GlobToken::AnyOne),
            '*' => tokens.push(GlobToken::AnyMany),
            value if value.is_ascii_uppercase() => {
                return Err(format!(
                    "Invalid --glob pattern `{pattern}`: uppercase `{value}` is reserved."
                ));
            }
            value => {
                if let Some(normalized) = normalize_glob_literal(value) {
                    tokens.push(GlobToken::Literal(normalized));
                }
            }
        }
    }
    if tokens.is_empty() {
        Err("--glob requires a non-empty pattern.".to_owned())
    } else {
        Ok(GlobPattern { tokens })
    }
}

#[requires(true)]
#[ensures(true)]
fn normalize_glob_literal(value: char) -> Option<char> {
    let text = value.to_string();
    let normalized = canonicalize_text(&text);
    let mut chars = normalized.chars();
    let value = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    if value == 'h' {
        Some('\'')
    } else if value.is_ascii_lowercase() || value.is_ascii_digit() || value == '\'' {
        Some(value)
    } else {
        None
    }
}

#[requires(true)]
#[ensures(true)]
fn glob_target_key(raw_word: &str) -> String {
    normalize_lookup_query(raw_word)
        .chars()
        .map(|value| if value == 'h' { '\'' } else { value })
        .collect()
}

#[requires(token_index <= tokens.len())]
#[requires(target_index <= target.len())]
#[ensures(true)]
fn glob_matches_from(
    tokens: &[GlobToken],
    token_index: usize,
    target: &[char],
    target_index: usize,
) -> bool {
    if token_index == tokens.len() {
        return target_index == target.len();
    }
    match tokens[token_index] {
        GlobToken::AnyMany => (target_index..=target.len())
            .any(|next_index| glob_matches_from(tokens, token_index + 1, target, next_index)),
        token => {
            target_index < target.len()
                && glob_token_matches(token, target[target_index])
                && glob_matches_from(tokens, token_index + 1, target, target_index + 1)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn glob_token_matches(token: GlobToken, value: char) -> bool {
    match token {
        GlobToken::Literal(expected) => value == expected,
        GlobToken::Consonant => is_lojban_consonant(value),
        GlobToken::Vowel => is_lojban_vowel(value),
        GlobToken::AnyOne => true,
        GlobToken::AnyMany => true,
    }
}

#[requires(true)]
#[ensures(true)]
fn is_lojban_consonant(value: char) -> bool {
    matches!(
        value,
        'b' | 'c'
            | 'd'
            | 'f'
            | 'g'
            | 'j'
            | 'k'
            | 'l'
            | 'm'
            | 'n'
            | 'p'
            | 'r'
            | 's'
            | 't'
            | 'v'
            | 'x'
            | 'z'
    )
}

#[requires(true)]
#[ensures(true)]
fn is_lojban_vowel(value: char) -> bool {
    matches!(value, 'a' | 'e' | 'i' | 'o' | 'u' | 'y')
}

#[requires(true)]
#[ensures(true)]
fn dictionary_word_sound_entry(
    entry: &DictionaryEntry<'_>,
) -> Option<crate::phonetic::IpaTokenSequence> {
    let ipa = dictionary_word_ipa(entry).ok()?;
    tokenize_ipa_text(&ipa).ok()
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
fn dictionary_word_ipa(entry: &DictionaryEntry<'_>) -> Result<String, PhoneticError> {
    if entry.word.contains(' ') {
        lojban_text_to_ipa(entry.word)
    } else if dictionary_word_type_has_pronunciation(entry.word_type) {
        Ok(fast_dictionary_word_ipa(
            matches!(entry.word_type, WordType::Cmevla | WordType::ObsoleteCmevla),
            entry.word,
        ))
    } else {
        Err(PhoneticError::Message(format!(
            "Cannot infer morphology kind for dictionary word `{}`.",
            entry.word
        )))
    }
}

#[requires(true)]
#[ensures(true)]
fn dictionary_word_type_has_pronunciation(word_type: WordType) -> bool {
    !matches!(word_type, WordType::Phrase)
}

#[requires(true)]
#[ensures(!ret.is_empty() || word_text.is_empty())]
fn fast_dictionary_word_ipa(is_cmevla: bool, word_text: &str) -> String {
    let normalized = normalize_lookup_query(word_text);
    let leading_pause = normalized
        .chars()
        .next()
        .is_some_and(|first| is_cmevla || is_fast_vowel_phoneme(first));
    let mut rendered = String::new();
    if leading_pause {
        rendered.push('ʔ');
    }
    render_phoneme_chars(&mut rendered, None, &normalized.chars().collect::<Vec<_>>());
    rendered
}

#[requires(true)]
#[ensures(true)]
fn render_phoneme_chars(output: &mut String, previous: Option<char>, chars: &[char]) {
    let Some((&current, rest)) = chars.split_first() else {
        return;
    };
    if current == ',' {
        render_phoneme_chars(output, previous, rest);
        return;
    }
    push_fast_ipa_phoneme(output, previous, current, next_non_comma(rest));
    render_phoneme_chars(output, Some(current), rest);
}

#[requires(true)]
#[ensures(true)]
fn next_non_comma(chars: &[char]) -> Option<char> {
    chars.iter().copied().find(|value| *value != ',')
}

#[requires(true)]
#[ensures(true)]
fn push_fast_ipa_phoneme(
    output: &mut String,
    previous: Option<char>,
    phoneme: char,
    next: Option<char>,
) {
    match phoneme {
        'i' if is_glide_position(previous, phoneme, next) => output.push('j'),
        'u' if is_glide_position(previous, phoneme, next) => output.push('w'),
        'y' => output.push('ə'),
        '\'' => output.push('h'),
        'c' => output.push('ʃ'),
        'j' => output.push('ʒ'),
        other => output.push(other),
    }
}

#[requires(true)]
#[ensures(true)]
fn is_glide_position(previous: Option<char>, phoneme: char, next: Option<char>) -> bool {
    previous.is_some_and(|value| is_falling_diphthong_before(value, phoneme))
        || next.is_some_and(is_fast_vowel_phoneme)
}

#[requires(true)]
#[ensures(true)]
fn is_falling_diphthong_before(previous: char, phoneme: char) -> bool {
    matches!(
        (previous, phoneme),
        ('a', 'i') | ('a', 'u') | ('e', 'i') | ('o', 'i')
    )
}

#[requires(true)]
#[ensures(true)]
fn is_fast_vowel_phoneme(phoneme: char) -> bool {
    matches!(phoneme, 'a' | 'e' | 'i' | 'o' | 'u')
}

#[cfg(test)]
mod tests {
    use super::*;
    use bityzba::requires;
    use jbotci_morphology::segment_words_with_modifiers;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn grouped_word_type_filters_keep_letterals_and_phrases_out_of_brivla() {
        assert!(matches_word_type_filter("letteral", "bu-letteral"));
        assert!(!matches_word_type_filter("cmavo", "bu-letteral"));
        assert!(!matches_word_type_filter("brivla", "bu-letteral"));
        assert!(!matches_word_type_filter("brivla", "phrase"));
        assert!(matches_word_type_filter("brivla", "gismu"));
        assert!(matches_word_type_filter("brivla", "lujvo"));
        assert!(matches_word_type_filter("brivla", "fu'ivla"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parsed_word_dictionary_cards_deduplicate_in_source_order() {
        let words = segment_words_with_modifiers("mi klama mi").expect("morphology");
        let cards = dictionary_cards_for_word_likes(jbotci_dictionary_data::english(), &words);
        assert_eq!(
            cards
                .iter()
                .map(|card| card.word.as_str())
                .collect::<Vec<_>>(),
            vec!["mi", "klama"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dictionary_entry_cards_do_not_render_blank_selmaho() {
        let entry = jbotci_dictionary_data::english()
            .lookup_word("brode")
            .expect("entry for brode");
        let card =
            dictionary_entry_card(jbotci_dictionary_data::english(), entry, Some(1.0), false);

        assert_eq!(card.word, "brode");
        assert_eq!(card.selmaho, None);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parsed_word_dictionary_cards_keep_exact_lujvo_atomic() {
        let words = segment_words_with_modifiers("jbobau").expect("morphology");
        let cards = dictionary_cards_for_word_likes(jbotci_dictionary_data::english(), &words);
        assert_eq!(
            cards
                .iter()
                .map(|card| card.word.as_str())
                .collect::<Vec<_>>(),
            vec!["jbobau"]
        );
        assert!(!cards.iter().any(|card| card.word == "lojbo"));
        assert!(!cards.iter().any(|card| card.word == "bangu"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parsed_word_dictionary_cards_expand_missing_lujvo_like_lujvo_mode() {
        let words = segment_words_with_modifiers("brodau").expect("morphology");
        let cards = dictionary_cards_for_word_likes(jbotci_dictionary_data::english(), &words);
        assert_eq!(cards.first().map(|card| card.word.as_str()), Some("brodau"));
        assert!(
            cards
                .first()
                .is_some_and(|card| !card.decomposition.is_empty())
        );
        assert!(cards.iter().any(|card| card.word == "xebro"));
        assert!(cards.iter().any(|card| card.word == "darlu"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parsed_word_dictionary_cards_lookup_letterals() {
        let words = segment_words_with_modifiers("a bu c bu").expect("morphology");
        let matches = dictionary_matches_for_word_likes(jbotci_dictionary_data::english(), &words);
        assert_eq!(
            matches
                .iter()
                .map(|parsed_match| parsed_match.lookup_text.as_str())
                .collect::<Vec<_>>(),
            vec!["abu", "cy"]
        );
        assert_eq!(
            matches
                .iter()
                .flat_map(|parsed_match| parsed_match.cards.iter())
                .map(|card| card.word.as_str())
                .collect::<Vec<_>>(),
            vec!["abu", "cy"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn parsed_word_dictionary_cards_lookup_zei_lujvo() {
        let words = segment_words_with_modifiers("a bu zei sance").expect("morphology");
        let matches = dictionary_matches_for_word_likes(jbotci_dictionary_data::english(), &words);
        assert_eq!(
            matches
                .iter()
                .map(|parsed_match| parsed_match.lookup_text.as_str())
                .collect::<Vec<_>>(),
            vec!["abu zei sance"]
        );
        assert_eq!(
            matches
                .first()
                .and_then(|parsed_match| parsed_match.cards.first())
                .map(|card| card.word.as_str()),
            Some("abu zei sance")
        );
    }
}
