use bityzba::{invariant, requires};
use jbotci_dictionary::{Dictionary, DictionaryEntry, Keyword, WordType, normalize_lookup_query};
use jbotci_jvozba::{LujvoDecomposition, decompose_lujvo_like};
use jbotci_morphology::{Jvopau, WordKind, canonicalize_text, segment_words_with_modifiers};

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
pub enum VlackuRequest {
    Valsi(String),
    Rafsi(String),
    Lujvo(String),
    Glob(String),
    Sound(String),
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
pub fn normalize_word_type_filter(raw: &str) -> String {
    raw.trim().to_ascii_lowercase().replace(' ', "-")
}

#[requires(true)]
#[ensures(true)]
pub fn matches_word_type_filter(wanted: &str, normalized_type: &str) -> bool {
    wanted == normalized_type
        || (wanted == "cmavo" && is_cmavo_like(normalized_type))
        || (wanted == "brivla" && is_brivla_like(normalized_type))
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
pub fn is_brivla_like(normalized_type: &str) -> bool {
    !is_cmavo_like(normalized_type)
        && normalized_type != "cmevla"
        && normalized_type != "obsolete-cmevla"
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
    }
}

#[requires(true)]
#[ensures(true)]
fn cards_for_valsi(
    dictionary: &Dictionary<'_>,
    query: &str,
    options: &VlackuSearchOptions,
) -> VlackuSearchOutput {
    let entries = dictionary.lookup_words(query).collect::<Vec<_>>();
    if entries.is_empty() {
        missing_exact_output(dictionary, query, options.decompose_lujvo)
    } else {
        let cards = filter_and_limit(
            entries
                .into_iter()
                .map(|entry| entry_card(dictionary, entry, Some(1.0), options.decompose_lujvo))
                .collect(),
            options,
            false,
        );
        found_or_missing(cards)
    }
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
                entry_card(
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
            dictionary
                .lookup_word(source_word)
                .map(|entry| entry_card(dictionary, entry, Some(1.0), options.decompose_lujvo))
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
            .map(|entry| entry_card(dictionary, entry, Some(1.0), options.decompose_lujvo))
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
                entry_card(dictionary, entry, Some(similarity), options.decompose_lujvo)
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
    decompose_lujvo: bool,
) -> VlackuSearchOutput {
    let normalized = normalize_lookup_query(query);
    match classify_exact_word(query, &normalized) {
        Some(classification) => {
            let decomposition = decompose_lujvo
                .then(|| decompose_lujvo_like(dictionary, query))
                .flatten();
            VlackuSearchOutput {
                cards: vec![unknown_card(classification, decomposition.as_ref())],
                outcome: VlackuOutcome::ValidMissing,
                diagnostics: Vec::new(),
            }
        }
        None => invalid_output(format!("Invalid Lojban word: {query}")),
    }
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
fn entry_card(
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
fn filter_and_limit(
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
