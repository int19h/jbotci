use bityzba::{data, invariant, new, requires};
use jbotci_dictionary::{
    Dictionary, DictionaryEntry, DictionaryLujvoEntry, DictionaryLujvoSegmentKind, Keyword,
    WordType, normalize_lookup_query, universal_gismu_rafsi_forms,
};
use jbotci_jvozba::{LujvoDecomposition, decompose_lujvo_like};
use jbotci_morphology::{
    LujvoPart, Verbatim, Word, WordKind, WordLike, WordLikeData, canonicalize_text,
    normalize_lojban_input_text, segment_words_with_modifiers,
};
use jbotci_phonetic::{
    AlineSimilarityScratch, aline_phonetic_similarity_with_scratch, compare_similarity_then_index,
    sound_query_to_token_sequence,
};
use regex::Regex;

pub const DEFAULT_VLACKU_RESULT_COUNT: usize = 20;
pub const OFFICIAL_AUTHOR_USERNAME: &str = "officialdata";
pub const INVALID_LOJBAN_WORD_MESSAGE_PREFIX: &str = "Invalid Lojban word: ";

#[invariant(::Valsi(query) => !query.is_empty())]
#[invariant(::Rafsi(query) => !query.is_empty())]
#[invariant(::Lujvo(query) => !query.is_empty())]
#[invariant(::Sound(query) => !query.is_empty())]
#[invariant(::Meaning(query) => !query.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VlackuRequest {
    Valsi(String),
    Rafsi(String),
    Lujvo(String),
    Sound(String),
    Meaning(String),
}

impl VlackuRequest {
    #[requires(!query.is_empty())]
    #[ensures(true)]
    pub fn valsi(query: String) -> Self {
        new!(VlackuRequest::Valsi(query))
    }

    #[requires(!query.is_empty())]
    #[ensures(true)]
    pub fn rafsi(query: String) -> Self {
        new!(VlackuRequest::Rafsi(query))
    }

    #[requires(!query.is_empty())]
    #[ensures(true)]
    pub fn lujvo(query: String) -> Self {
        new!(VlackuRequest::Lujvo(query))
    }

    #[requires(!query.is_empty())]
    #[ensures(true)]
    pub fn sound(query: String) -> Self {
        new!(VlackuRequest::Sound(query))
    }

    #[requires(!query.is_empty())]
    #[ensures(true)]
    pub fn meaning(query: String) -> Self {
        new!(VlackuRequest::Meaning(query))
    }
}

#[invariant(*count > 0)]
#[invariant(min_similarity.as_ref().is_none_or(|value| value.is_finite()))]
#[derive(Debug, Clone, PartialEq)]
pub struct VlackuSearchOptions {
    pub count: usize,
    pub word_types: Vec<WordTypeFilter>,
    pub min_votes: Option<i32>,
    pub min_similarity: Option<f32>,
    pub decompose_lujvo: bool,
}

impl Default for VlackuSearchOptions {
    #[requires(true)]
    #[ensures(ret.count == DEFAULT_VLACKU_RESULT_COUNT)]
    fn default() -> Self {
        new!(VlackuSearchOptions {
            count: DEFAULT_VLACKU_RESULT_COUNT,
            word_types: Vec::new(),
            min_votes: None,
            min_similarity: None,
            decompose_lujvo: false,
        })
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WordTypeFilter {
    Cmavo,
    ExperimentalCmavo,
    ObsoleteCmavo,
    CmavoCompound,
    Letteral,
    BuLetteral,
    Cmevla,
    ObsoleteCmevla,
    Gismu,
    ExperimentalGismu,
    Fuivla,
    ObsoleteFuivla,
    Lujvo,
    ZeiLujvo,
    ObsoleteZeiLujvo,
    Brivla,
    Phrase,
}

impl WordTypeFilter {
    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|filter| !filter.as_str().is_empty()))]
    pub fn parse(raw: &str) -> Option<Self> {
        match normalize_word_type_filter(raw).as_str() {
            "cmavo" => Some(Self::Cmavo),
            "experimental-cmavo" => Some(Self::ExperimentalCmavo),
            "obsolete-cmavo" => Some(Self::ObsoleteCmavo),
            "cmavo-compound" => Some(Self::CmavoCompound),
            "letteral" => Some(Self::Letteral),
            "bu-letteral" => Some(Self::BuLetteral),
            "cmevla" => Some(Self::Cmevla),
            "obsolete-cmevla" => Some(Self::ObsoleteCmevla),
            "gismu" => Some(Self::Gismu),
            "experimental-gismu" => Some(Self::ExperimentalGismu),
            "fu'ivla" => Some(Self::Fuivla),
            "obsolete-fu'ivla" => Some(Self::ObsoleteFuivla),
            "lujvo" => Some(Self::Lujvo),
            "zei-lujvo" => Some(Self::ZeiLujvo),
            "obsolete-zei-lujvo" => Some(Self::ObsoleteZeiLujvo),
            "brivla" => Some(Self::Brivla),
            "phrase" => Some(Self::Phrase),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cmavo => "cmavo",
            Self::ExperimentalCmavo => "experimental-cmavo",
            Self::ObsoleteCmavo => "obsolete-cmavo",
            Self::CmavoCompound => "cmavo-compound",
            Self::Letteral => "letteral",
            Self::BuLetteral => "bu-letteral",
            Self::Cmevla => "cmevla",
            Self::ObsoleteCmevla => "obsolete-cmevla",
            Self::Gismu => "gismu",
            Self::ExperimentalGismu => "experimental-gismu",
            Self::Fuivla => "fu'ivla",
            Self::ObsoleteFuivla => "obsolete-fu'ivla",
            Self::Lujvo => "lujvo",
            Self::ZeiLujvo => "zei-lujvo",
            Self::ObsoleteZeiLujvo => "obsolete-zei-lujvo",
            Self::Brivla => "brivla",
            Self::Phrase => "phrase",
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub const fn from_word_type(word_type: WordType) -> Self {
        match word_type {
            WordType::Gismu => Self::Gismu,
            WordType::ExperimentalGismu => Self::ExperimentalGismu,
            WordType::Lujvo => Self::Lujvo,
            WordType::ZeiLujvo => Self::ZeiLujvo,
            WordType::ObsoleteZeiLujvo => Self::ObsoleteZeiLujvo,
            WordType::Cmavo => Self::Cmavo,
            WordType::ExperimentalCmavo => Self::ExperimentalCmavo,
            WordType::ObsoleteCmavo => Self::ObsoleteCmavo,
            WordType::CmavoCompound => Self::CmavoCompound,
            WordType::Fuivla => Self::Fuivla,
            WordType::ObsoleteFuivla => Self::ObsoleteFuivla,
            WordType::Cmevla => Self::Cmevla,
            WordType::ObsoleteCmevla => Self::ObsoleteCmevla,
            WordType::BuLetteral => Self::BuLetteral,
            WordType::Phrase => Self::Phrase,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub const fn group_for_word_type(word_type: WordType) -> Self {
        match word_type {
            WordType::Cmavo
            | WordType::ExperimentalCmavo
            | WordType::ObsoleteCmavo
            | WordType::CmavoCompound => Self::Cmavo,
            WordType::BuLetteral => Self::Letteral,
            WordType::Cmevla | WordType::ObsoleteCmevla => Self::Cmevla,
            WordType::Gismu | WordType::ExperimentalGismu => Self::Gismu,
            WordType::Fuivla | WordType::ObsoleteFuivla => Self::Fuivla,
            WordType::Lujvo | WordType::ZeiLujvo | WordType::ObsoleteZeiLujvo => Self::Lujvo,
            WordType::Phrase => Self::Phrase,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn matches_word_type(self, word_type: WordType) -> bool {
        self.matches_filter(Self::from_word_type(word_type))
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn matches_filter(self, actual: Self) -> bool {
        self == actual
            || match self {
                Self::Cmavo => actual.is_cmavo_like(),
                Self::Letteral => actual.is_letteral_like(),
                Self::Cmevla => actual.is_cmevla_like(),
                Self::Gismu => actual.is_gismu_like(),
                Self::Fuivla => actual.is_fuivla_like(),
                Self::Lujvo => actual.is_lujvo_like(),
                Self::Brivla => actual.is_brivla_like(),
                Self::ExperimentalCmavo
                | Self::ObsoleteCmavo
                | Self::CmavoCompound
                | Self::BuLetteral
                | Self::ObsoleteCmevla
                | Self::ExperimentalGismu
                | Self::ObsoleteFuivla
                | Self::ZeiLujvo
                | Self::ObsoleteZeiLujvo
                | Self::Phrase => false,
            }
    }

    #[requires(true)]
    #[ensures(!ret.as_str().is_empty())]
    pub const fn group(self) -> Self {
        match self {
            Self::Cmavo | Self::ExperimentalCmavo | Self::ObsoleteCmavo | Self::CmavoCompound => {
                Self::Cmavo
            }
            Self::Letteral | Self::BuLetteral => Self::Letteral,
            Self::Cmevla | Self::ObsoleteCmevla => Self::Cmevla,
            Self::Gismu | Self::ExperimentalGismu => Self::Gismu,
            Self::Fuivla | Self::ObsoleteFuivla => Self::Fuivla,
            Self::Lujvo | Self::ZeiLujvo | Self::ObsoleteZeiLujvo => Self::Lujvo,
            Self::Brivla => Self::Brivla,
            Self::Phrase => Self::Phrase,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub const fn is_lujvo_like(self) -> bool {
        matches!(self, Self::Lujvo | Self::ZeiLujvo | Self::ObsoleteZeiLujvo)
    }

    #[requires(true)]
    #[ensures(true)]
    pub const fn is_brivla_like(self) -> bool {
        self.is_gismu_like() || self.is_lujvo_like() || self.is_fuivla_like()
    }

    #[requires(true)]
    #[ensures(true)]
    const fn is_cmavo_like(self) -> bool {
        matches!(
            self,
            Self::Cmavo | Self::ExperimentalCmavo | Self::ObsoleteCmavo | Self::CmavoCompound
        )
    }

    #[requires(true)]
    #[ensures(true)]
    const fn is_letteral_like(self) -> bool {
        matches!(self, Self::Letteral | Self::BuLetteral)
    }

    #[requires(true)]
    #[ensures(true)]
    const fn is_cmevla_like(self) -> bool {
        matches!(self, Self::Cmevla | Self::ObsoleteCmevla)
    }

    #[requires(true)]
    #[ensures(true)]
    const fn is_gismu_like(self) -> bool {
        matches!(self, Self::Gismu | Self::ExperimentalGismu)
    }

    #[requires(true)]
    #[ensures(true)]
    const fn is_fuivla_like(self) -> bool {
        matches!(self, Self::Fuivla | Self::ObsoleteFuivla)
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

#[invariant(!word.is_empty())]
#[derive(Debug, Clone, PartialEq)]
pub struct VlackuCard {
    pub word: String,
    pub word_type: String,
    pub selmaho: Option<String>,
    pub author: Option<VlackuAuthor>,
    pub is_official: bool,
    pub similarity: Option<f32>,
    pub votes: Option<i32>,
    pub rafsi: Vec<String>,
    pub glosses: Vec<String>,
    pub definition: String,
    pub notes: String,
    pub etymology: Option<String>,
    pub decomposition: Vec<VlackuCompositionPiece>,
}

#[invariant(!self.username.is_empty())]
#[invariant(self.realname.as_ref().is_none_or(|realname| !realname.trim().is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VlackuAuthor {
    pub username: String,
    pub realname: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct VlackuCompositionPiece {
    pub kind: VlackuCompositionKind,
    pub surface: String,
    pub source: Option<String>,
}

#[invariant(!lookup_text.is_empty())]
#[invariant(byte_start <= byte_end)]
#[invariant(char_start <= char_end)]
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedWordDictionaryMatch {
    pub lookup_text: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub char_start: usize,
    pub char_end: usize,
    pub cards: Vec<VlackuCard>,
}

#[invariant(!lookup_text.is_empty())]
#[invariant(byte_start <= byte_end)]
#[invariant(char_start <= char_end)]
#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub word_type: WordKindTypeKey,
    pub selmaho: Option<String>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordKindTypeKey {
    Cmavo,
    Gismu,
    Lujvo,
    Fuivla,
    Cmevla,
}

impl WordKindTypeKey {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cmavo => "cmavo",
            Self::Gismu => "gismu",
            Self::Lujvo => "lujvo",
            Self::Fuivla => "fu'ivla",
            Self::Cmevla => "cmevla",
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub const fn is_lujvo(self) -> bool {
        matches!(self, Self::Lujvo)
    }

    #[requires(true)]
    #[ensures(true)]
    pub const fn is_brivla(self) -> bool {
        matches!(self, Self::Gismu | Self::Lujvo | Self::Fuivla)
    }
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
        extend_unique_cards(&mut cards, parsed_match.into_data().cards);
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
            let target = target.into_data();
            matches.push(new!(ParsedWordDictionaryMatch {
                lookup_text: target.lookup_text,
                byte_start: target.byte_start,
                byte_end: target.byte_end,
                char_start: target.char_start,
                char_end: target.char_end,
                cards,
            }));
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
pub fn parse_word_type_filter(raw: &str) -> Option<WordTypeFilter> {
    WordTypeFilter::parse(raw)
}

#[requires(true)]
#[ensures(true)]
pub fn matches_word_type_filter(wanted: WordTypeFilter, actual: WordTypeFilter) -> bool {
    wanted.matches_filter(actual)
}

#[requires(true)]
#[ensures(true)]
pub fn grouped_word_type_filter_key(normalized_type: &str) -> String {
    WordTypeFilter::parse(normalized_type)
        .map(|filter| filter.group().as_str().to_owned())
        .unwrap_or_else(|| normalized_type.to_owned())
}

#[requires(true)]
#[ensures(ret == (query.trim().starts_with('/') || query.trim().chars().any(is_glob_wildcard)))]
pub fn vlacku_exact_query_is_pattern(query: &str) -> bool {
    let query = query.trim();
    query.starts_with('/') || query.chars().any(is_glob_wildcard)
}

#[requires(true)]
#[ensures(true)]
pub fn is_cmavo_like(normalized_type: &str) -> bool {
    WordTypeFilter::parse(normalized_type).is_some_and(WordTypeFilter::is_cmavo_like)
}

#[requires(true)]
#[ensures(true)]
pub fn is_letteral_like(normalized_type: &str) -> bool {
    WordTypeFilter::parse(normalized_type).is_some_and(WordTypeFilter::is_letteral_like)
}

#[requires(true)]
#[ensures(true)]
pub fn is_cmevla_like(normalized_type: &str) -> bool {
    WordTypeFilter::parse(normalized_type).is_some_and(WordTypeFilter::is_cmevla_like)
}

#[requires(true)]
#[ensures(true)]
pub fn is_gismu_like(normalized_type: &str) -> bool {
    WordTypeFilter::parse(normalized_type).is_some_and(WordTypeFilter::is_gismu_like)
}

#[requires(true)]
#[ensures(true)]
pub fn is_fuhivla_like(normalized_type: &str) -> bool {
    WordTypeFilter::parse(normalized_type).is_some_and(WordTypeFilter::is_fuivla_like)
}

#[requires(true)]
#[ensures(true)]
pub fn is_lujvo_like(normalized_type: &str) -> bool {
    WordTypeFilter::parse(normalized_type).is_some_and(WordTypeFilter::is_lujvo_like)
}

#[requires(true)]
#[ensures(true)]
pub fn is_brivla_like(normalized_type: &str) -> bool {
    WordTypeFilter::parse(normalized_type).is_some_and(WordTypeFilter::is_brivla_like)
}

#[requires(true)]
#[ensures(ret.starts_with('+') == (value > 0))]
pub fn format_votes(value: i32) -> String {
    if value > 0 {
        format!("+{value}")
    } else {
        value.to_string()
    }
}

#[requires(true)]
#[ensures(is_official -> ret == "∞")]
pub fn format_vote_display(value: i32, is_official: bool) -> String {
    if is_official {
        "∞".to_owned()
    } else {
        format_votes(value)
    }
}

#[requires(true)]
#[ensures(true)]
pub fn dictionary_entry_passes_vlacku_filters(
    entry: &DictionaryEntry<'_>,
    options: &VlackuSearchOptions,
    similarity: Option<f32>,
    similarity_mode: bool,
) -> bool {
    let entry_filters_ok = dictionary_entry_passes_vlacku_entry_filters(entry, options);
    let similarity_ok = !similarity_mode
        || options
            .min_similarity
            .is_none_or(|min_similarity| similarity.unwrap_or(0.0) * 100.0 >= min_similarity);
    entry_filters_ok && similarity_ok
}

#[requires(true)]
#[ensures(true)]
pub fn dictionary_entry_passes_vlacku_entry_filters(
    entry: &DictionaryEntry<'_>,
    options: &VlackuSearchOptions,
) -> bool {
    let word_type_ok = options.word_types.is_empty()
        || options
            .word_types
            .iter()
            .any(|wanted| wanted.matches_word_type(entry.word_type));
    let votes_ok = options
        .min_votes
        .is_none_or(|min_votes| entry_vote_count(entry) >= min_votes);
    word_type_ok && votes_ok
}

#[requires(true)]
#[ensures(true)]
fn entry_vote_count(entry: &DictionaryEntry<'_>) -> i32 {
    entry.score.get().round() as i32
}

#[requires(true)]
#[ensures(true)]
fn run_single_request(
    dictionary: &Dictionary<'_>,
    request: &VlackuRequest,
    options: &VlackuSearchOptions,
) -> VlackuSearchOutput {
    match request.as_data() {
        data!(VlackuRequest::Valsi(query)) => cards_for_valsi(dictionary, query, options),
        data!(VlackuRequest::Rafsi(query)) => cards_for_rafsi(dictionary, query, options),
        data!(VlackuRequest::Lujvo(query)) => cards_for_lujvo(dictionary, query, options),
        data!(VlackuRequest::Sound(query)) => cards_for_sound(dictionary, query, options),
        data!(VlackuRequest::Meaning(_)) => {
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
    new!(VlackuSearchOptions {
        count: usize::MAX,
        word_types: Vec::new(),
        min_votes: None,
        min_similarity: None,
        decompose_lujvo: false,
    })
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
        data!(WordLike::PlainWord(word)) => {
            push_word_lookup_target(word, targets);
        }
        data!(WordLike::QuotedWord { zo, word }) => {
            push_word_lookup_target(zo, targets);
            push_word_lookup_target(word, targets);
        }
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            closing_delimiter,
            ..
        }) => {
            push_word_lookup_target(zoi, targets);
            push_word_lookup_target(opening_delimiter, targets);
            push_word_lookup_target(closing_delimiter, targets);
        }
        data!(WordLike::QuotedWords {
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
        data!(WordLike::DelimitedWordQuote { marker, .. }) => {
            push_word_lookup_target(marker, targets);
        }
        data!(WordLike::LerfuWord { .. }) | data!(WordLike::ZeiCompound { .. }) => {
            if let Some(target) = word_like_lookup_target(word_like) {
                targets.push(target);
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn push_word_lookup_target(word: &Word, targets: &mut Vec<ParsedWordLookupTarget>) {
    targets.push(new!(ParsedWordLookupTarget {
        lookup_text: word_lookup_text(word),
        is_lujvo: word.kind() == WordKind::Lujvo,
        byte_start: word.span().byte_start,
        byte_end: word.span().byte_end,
        char_start: word.span().char_start,
        char_end: word.span().char_end,
    }));
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
    Some(new!(ParsedWordLookupTarget {
        lookup_text,
        is_lujvo: matches!(word_like.as_data(), data!(WordLike::ZeiCompound { .. })),
        byte_start,
        byte_end,
        char_start,
        char_end,
    }))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
fn word_like_lookup_text(word_like: &WordLike) -> Option<String> {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => Some(word_lookup_text(word)),
        data!(WordLike::LerfuWord { base, .. }) => letteral_lookup_text(base),
        data!(WordLike::ZeiCompound { left, right, .. }) => Some(format!(
            "{} zei {}",
            word_like_lookup_text(left)?,
            word_lookup_text(right)
        )),
        data!(WordLike::QuotedWord { .. })
        | data!(WordLike::DelimitedNonLojbanQuote { .. })
        | data!(WordLike::QuotedWords { .. })
        | data!(WordLike::DelimitedWordQuote { .. }) => None,
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
    if vlacku_exact_query_is_pattern(query) {
        return cards_for_valsi_pattern(dictionary, query.trim(), options);
    }

    let Some(query) = normalize_exact_lojban_query(query) else {
        return invalid_lojban_word_output(query);
    };
    let query = query.as_str();
    let entries = dictionary.lookup_words(query).collect::<Vec<_>>();
    if !entries.is_empty() {
        return found_or_missing(cards_with_optional_lujvo_sources(
            dictionary,
            query,
            entries
                .into_iter()
                .filter(|entry| {
                    dictionary_entry_passes_vlacku_filters(entry, options, Some(1.0), false)
                })
                .map(|entry| {
                    dictionary_entry_card(dictionary, entry, Some(1.0), options.decompose_lujvo)
                })
                .collect(),
            options,
        ));
    }

    if let Some(entries) = resolve_segmented_lookup(dictionary, query) {
        let cards = filter_vlacku_cards(
            entries
                .into_iter()
                .filter(|entry| {
                    dictionary_entry_passes_vlacku_filters(entry, options, Some(1.0), false)
                })
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
    if vlacku_exact_query_is_pattern(query) {
        return cards_for_rafsi_pattern(dictionary, query.trim(), options);
    }

    let Some(query) = normalize_exact_lojban_query(query) else {
        return invalid_lojban_word_output(query);
    };
    let query = query.as_str();
    let cards = filter_vlacku_cards(
        dictionary
            .lookup_rafsi(query)
            .filter(|matched| {
                dictionary_entry_passes_vlacku_filters(matched.entry, options, Some(1.0), false)
            })
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
    let Some(query) = normalize_exact_lojban_query(query) else {
        return invalid_lojban_word_output(query);
    };
    let query = query.as_str();
    let normalized = normalize_lookup_query(query);
    let exact_entries = dictionary.lookup_words(query).collect::<Vec<_>>();
    let exact_found = !exact_entries.is_empty();
    let mut runtime_decomposition = None;
    let mut cards = if exact_entries.is_empty() {
        let decomposition = decompose_lujvo_like(dictionary, query);
        let cards = match classify_exact_word(query, &normalized) {
            Some(classification) => vec![unknown_card(classification, decomposition.as_ref())],
            None => {
                return invalid_lojban_word_output(query);
            }
        };
        runtime_decomposition = decomposition;
        cards
    } else {
        let decomposition = exact_entries
            .iter()
            .copied()
            .find_map(|entry| dictionary_lujvo_decomposition_for_entry(dictionary, entry));
        let mut cards = exact_entries
            .into_iter()
            .filter(|entry| {
                dictionary_entry_passes_vlacku_filters(entry, options, Some(1.0), false)
            })
            .map(|entry| {
                entry_card_with_dictionary_decomposition(
                    entry,
                    Some(1.0),
                    dictionary_lujvo_decomposition_for_entry(dictionary, entry),
                )
            })
            .collect();
        if let Some(decomposition) = decomposition {
            extend_unique_cards(
                &mut cards,
                cards_for_dictionary_lujvo_decomposition(dictionary, decomposition, options),
            );
        }
        cards
    };

    if let Some(decomposition) = runtime_decomposition.as_ref() {
        extend_unique_cards(
            &mut cards,
            cards_for_lujvo_decomposition(dictionary, decomposition, options),
        );
    }

    let cards = filter_vlacku_cards(cards, options, false);
    let outcome = if exact_found {
        VlackuOutcome::Found
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
fn cards_for_valsi_pattern(
    dictionary: &Dictionary<'_>,
    pattern: &str,
    options: &VlackuSearchOptions,
) -> VlackuSearchOutput {
    let compiled = match compile_exact_pattern(pattern) {
        Ok(compiled) => compiled,
        Err(message) => return invalid_output(message),
    };
    let cards = cards_for_matching_entries(
        dictionary,
        dictionary
            .entries()
            .iter()
            .filter(|entry| compiled.matches(&exact_pattern_target_key(entry.word))),
        options,
    );
    found_or_missing(cards)
}

#[requires(true)]
#[ensures(true)]
fn cards_for_rafsi_pattern(
    dictionary: &Dictionary<'_>,
    pattern: &str,
    options: &VlackuSearchOptions,
) -> VlackuSearchOutput {
    let compiled = match compile_exact_pattern(pattern) {
        Ok(compiled) => compiled,
        Err(message) => return invalid_output(message),
    };
    let cards = cards_for_matching_entries(
        dictionary,
        dictionary
            .entries()
            .iter()
            .filter(|entry| entry_has_matching_rafsi_pattern(entry, &compiled)),
        options,
    );
    found_or_missing(cards)
}

#[requires(true)]
#[ensures(ret.len() <= options.count)]
fn cards_for_matching_entries<'dictionary>(
    dictionary: &Dictionary<'dictionary>,
    entries: impl IntoIterator<Item = &'dictionary DictionaryEntry<'dictionary>>,
    options: &VlackuSearchOptions,
) -> Vec<VlackuCard> {
    cards_for_matching_entries_with_builder(entries, options, |entry| {
        dictionary_entry_card(dictionary, entry, Some(1.0), options.decompose_lujvo)
    })
}

#[requires(true)]
#[ensures(ret.len() <= options.count)]
fn cards_for_matching_entries_with_builder<'dictionary>(
    entries: impl IntoIterator<Item = &'dictionary DictionaryEntry<'dictionary>>,
    options: &VlackuSearchOptions,
    mut build_card: impl FnMut(&'dictionary DictionaryEntry<'dictionary>) -> VlackuCard,
) -> Vec<VlackuCard> {
    entries
        .into_iter()
        .filter(|entry| dictionary_entry_passes_vlacku_filters(entry, options, Some(1.0), false))
        .take(options.count)
        .map(|entry| build_card(entry))
        .collect()
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

    let mut scratch = AlineSimilarityScratch::default();
    let query_view = query_sound.view();
    let entries = dictionary.entries();
    let sound_index = dictionary.sound_index();
    let mut scored = sound_index
        .iter()
        .filter_map(|sound_entry| {
            let entry = entries.get(sound_entry.entry_index.get())?;
            if !dictionary_entry_passes_vlacku_entry_filters(entry, options) {
                return None;
            }
            let similarity = aline_phonetic_similarity_with_scratch(
                query_view,
                sound_entry.token_sequence,
                &mut scratch,
            ) as f32;
            dictionary_entry_passes_vlacku_filters(entry, options, Some(similarity), true)
                .then_some((sound_entry.entry_index.get(), similarity, entry))
        })
        .collect::<Vec<_>>();
    retain_best_sound_scores(&mut scored, options.count);

    let cards = scored
        .into_iter()
        .take(options.count)
        .map(|(_, similarity, entry)| {
            dictionary_entry_card(dictionary, entry, Some(similarity), options.decompose_lujvo)
        })
        .collect();
    found_or_missing(cards)
}

#[requires(true)]
#[ensures(scored.len() <= count)]
fn retain_best_sound_scores<'dictionary>(
    scored: &mut Vec<(usize, f32, &'dictionary DictionaryEntry<'dictionary>)>,
    count: usize,
) {
    if scored.len() > count {
        scored.select_nth_unstable_by(count, compare_sound_score);
        scored.truncate(count);
    }
    scored.sort_by(compare_sound_score);
}

#[requires(true)]
#[ensures(matches!(ret, std::cmp::Ordering::Less | std::cmp::Ordering::Equal | std::cmp::Ordering::Greater))]
fn compare_sound_score<T>(left: &(usize, f32, T), right: &(usize, f32, T)) -> std::cmp::Ordering {
    compare_similarity_then_index((left.0, left.1.into()), (right.0, right.1.into()))
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
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty() || query.trim().is_empty()))]
fn normalize_exact_lojban_query(query: &str) -> Option<String> {
    let normalized = normalize_lookup_query(
        &normalize_lojban_input_text(query).unwrap_or_else(|| query.to_owned()),
    );
    if normalized.is_empty() && !query.trim().is_empty() {
        None
    } else {
        Some(normalized)
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
#[ensures(ret.outcome == VlackuOutcome::Invalid)]
fn invalid_lojban_word_output(query: &str) -> VlackuSearchOutput {
    invalid_output(format!("{INVALID_LOJBAN_WORD_MESSAGE_PREFIX}{query}"))
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
            let decomposition = (options.decompose_lujvo && classification.word_type.is_lujvo())
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
        None => invalid_lojban_word_output(query),
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
    let should_decompose_sources = options.decompose_lujvo
        && cards.iter().any(|card| {
            WordTypeFilter::parse(&card.word_type).is_some_and(WordTypeFilter::is_lujvo_like)
        });
    if should_decompose_sources {
        if let Some(decomposition) = dictionary_lujvo_decomposition_for_query(dictionary, query) {
            extend_unique_cards(
                &mut cards,
                cards_for_dictionary_lujvo_decomposition(dictionary, decomposition, options),
            );
        } else if let Some(decomposition) = decompose_lujvo_like(dictionary, query) {
            extend_unique_cards(
                &mut cards,
                cards_for_lujvo_decomposition(dictionary, &decomposition, options),
            );
        }
    }
    filter_vlacku_cards(cards, options, false)
}

#[requires(true)]
#[ensures(ret.is_none_or(|decomposition| !decomposition.segments.is_empty()))]
fn dictionary_lujvo_decomposition_for_query<'dictionary>(
    dictionary: &'dictionary Dictionary<'dictionary>,
    query: &str,
) -> Option<&'dictionary DictionaryLujvoEntry<'dictionary>> {
    dictionary
        .lookup_words(query)
        .find_map(|entry| dictionary_lujvo_decomposition_for_entry(dictionary, entry))
}

#[requires(true)]
#[ensures(true)]
fn cards_for_lujvo_decomposition(
    dictionary: &Dictionary<'_>,
    decomposition: &LujvoDecomposition<'_>,
    options: &VlackuSearchOptions,
) -> Vec<VlackuCard> {
    let mut cards = Vec::new();
    for source_word in &decomposition.source_words {
        if let Some(entry) = dictionary.lookup_word(source_word) {
            if !dictionary_entry_passes_vlacku_filters(entry, options, Some(1.0), false) {
                continue;
            }
            extend_unique_cards(
                &mut cards,
                vec![dictionary_entry_card(
                    dictionary,
                    entry,
                    Some(1.0),
                    options.decompose_lujvo,
                )],
            );
        }
    }
    if let Some(surface) = final_rafsi_surface(decomposition) {
        extend_unique_cards(
            &mut cards,
            exact_word_cards_for_lujvo_final_segment(dictionary, surface, options),
        );
    }
    cards
}

#[requires(true)]
#[ensures(true)]
fn cards_for_dictionary_lujvo_decomposition(
    dictionary: &Dictionary<'_>,
    decomposition: &DictionaryLujvoEntry<'_>,
    options: &VlackuSearchOptions,
) -> Vec<VlackuCard> {
    let mut cards = Vec::new();
    for source_word in decomposition.source_words {
        if let Some(entry) = dictionary.lookup_word(source_word) {
            if !dictionary_entry_passes_vlacku_filters(entry, options, Some(1.0), false) {
                continue;
            }
            extend_unique_cards(
                &mut cards,
                vec![dictionary_entry_card(
                    dictionary,
                    entry,
                    Some(1.0),
                    options.decompose_lujvo,
                )],
            );
        }
    }
    if let Some(surface) = final_dictionary_rafsi_surface(decomposition) {
        extend_unique_cards(
            &mut cards,
            exact_word_cards_for_lujvo_final_segment(dictionary, surface, options),
        );
    }
    cards
}

#[requires(true)]
#[ensures(ret.is_none_or(|surface| !surface.is_empty()))]
fn final_rafsi_surface<'a>(decomposition: &'a LujvoDecomposition<'_>) -> Option<&'a str> {
    decomposition
        .segments
        .iter()
        .rev()
        .find_map(|segment| match &segment.segment {
            LujvoPart::Rafsi(phonemes) => Some(phonemes.as_str()),
            LujvoPart::Hyphen(_) => None,
        })
}

#[requires(true)]
#[ensures(ret.is_none_or(|surface| !surface.is_empty()))]
fn final_dictionary_rafsi_surface<'a>(
    decomposition: &'a DictionaryLujvoEntry<'_>,
) -> Option<&'a str> {
    decomposition
        .segments
        .iter()
        .rev()
        .find_map(|segment| match segment.kind {
            DictionaryLujvoSegmentKind::Rafsi => Some(segment.surface),
            DictionaryLujvoSegmentKind::Hyphen => None,
        })
}

#[requires(!surface.is_empty())]
#[ensures(true)]
fn exact_word_cards_for_lujvo_final_segment(
    dictionary: &Dictionary<'_>,
    surface: &str,
    options: &VlackuSearchOptions,
) -> Vec<VlackuCard> {
    let normalized = normalize_lookup_query(surface);
    let Some(classification) = classify_exact_word(surface, &normalized) else {
        return Vec::new();
    };
    if !classification.word_type.is_brivla() {
        return Vec::new();
    }

    let entries = dictionary.lookup_words(&normalized).collect::<Vec<_>>();
    if entries.is_empty() {
        vec![unknown_card(classification, None)]
    } else {
        entries
            .into_iter()
            .filter(|entry| {
                dictionary_entry_passes_vlacku_filters(entry, options, Some(1.0), false)
            })
            .map(|entry| {
                dictionary_entry_card(dictionary, entry, Some(1.0), options.decompose_lujvo)
            })
            .collect()
    }
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
        data!(WordLike::PlainWord(word)) => append_word_phonemes(word, output),
        data!(WordLike::QuotedWord { zo, word }) => {
            append_word_phonemes(zo, output);
            append_surface_chunk(
                output,
                &quoted_words_phonemes(std::iter::once(word.as_ref())),
            );
        }
        data!(WordLike::DelimitedNonLojbanQuote {
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
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            lehu,
        }) => {
            append_word_phonemes(lohu, output);
            append_surface_chunk(output, &quoted_words_phonemes(quoted_words.iter()));
            append_word_phonemes(lehu, output);
        }
        data!(WordLike::DelimitedWordQuote {
            marker,
            quoted_text,
        }) => {
            append_word_phonemes(marker, output);
            append_surface_chunk(output, &quoted_text_phonemes(quoted_text));
        }
        data!(WordLike::LerfuWord { base, bu }) => {
            append_word_like_phonemes(base, output);
            append_word_phonemes(bu, output);
        }
        data!(WordLike::ZeiCompound { left, zei, right }) => {
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
        word_type: word_kind_type_key(word.kind()),
        selmaho: word.selmaho().map(str::to_owned),
    })
}

#[requires(true)]
#[ensures(!ret.as_str().is_empty())]
fn word_kind_type_key(kind: WordKind) -> WordKindTypeKey {
    match kind {
        WordKind::Cmavo => WordKindTypeKey::Cmavo,
        WordKind::Gismu => WordKindTypeKey::Gismu,
        WordKind::Lujvo => WordKindTypeKey::Lujvo,
        WordKind::Fuhivla => WordKindTypeKey::Fuivla,
        WordKind::Cmevla => WordKindTypeKey::Cmevla,
    }
}

#[requires(true)]
#[ensures(true)]
pub fn dictionary_entry_card(
    dictionary: &Dictionary<'_>,
    entry: &DictionaryEntry<'_>,
    similarity: Option<f32>,
    decompose_lujvo: bool,
) -> VlackuCard {
    let decomposition = decompose_lujvo
        .then(|| dictionary_lujvo_decomposition_for_entry(dictionary, entry))
        .flatten();
    entry_card_with_dictionary_decomposition(entry, similarity, decomposition)
}

#[requires(true)]
#[ensures(ret.is_none_or(|decomposition| !decomposition.segments.is_empty()))]
fn dictionary_lujvo_decomposition_for_entry<'dictionary>(
    dictionary: &'dictionary Dictionary<'dictionary>,
    entry: &'dictionary DictionaryEntry<'dictionary>,
) -> Option<&'dictionary DictionaryLujvoEntry<'dictionary>> {
    if !WordTypeFilter::Lujvo.matches_word_type(entry.word_type) {
        return None;
    }
    let index = dictionary.entry_index_for_entry(entry)?;
    dictionary.lujvo_decomposition_for_entry_index(index)
}

#[requires(true)]
#[ensures(true)]
fn entry_card_with_dictionary_decomposition(
    entry: &DictionaryEntry<'_>,
    similarity: Option<f32>,
    decomposition: Option<&DictionaryLujvoEntry<'_>>,
) -> VlackuCard {
    new!(VlackuCard {
        word: entry.word.to_owned(),
        word_type: entry.word_type.as_str().to_owned(),
        selmaho: entry.selmaho.map(|selmaho| selmaho.0.to_owned()),
        author: Some(new!(VlackuAuthor {
            username: entry.user.username.to_owned(),
            realname: entry
                .user
                .realname
                .filter(|realname| !realname.trim().is_empty())
                .map(str::to_owned),
        })),
        is_official: entry.user.username == OFFICIAL_AUTHOR_USERNAME,
        similarity,
        votes: Some(entry_vote_count(entry)),
        rafsi: entry.rafsi.iter().map(|rafsi| rafsi.0.to_owned()).collect(),
        glosses: entry.gloss_keywords.iter().map(format_keyword).collect(),
        definition: entry.definition.to_owned(),
        notes: entry.notes.to_owned(),
        etymology: entry
            .etymology
            .filter(|etymology| !etymology.trim().is_empty())
            .map(str::to_owned),
        decomposition: decomposition
            .map(composition_from_dictionary_decomposition)
            .unwrap_or_default(),
    })
}

#[requires(true)]
#[ensures(true)]
fn unknown_card(
    classification: WordClassification,
    decomposition: Option<&LujvoDecomposition<'_>>,
) -> VlackuCard {
    new!(VlackuCard {
        word: classification.word,
        word_type: classification.word_type.as_str().to_owned(),
        selmaho: classification.selmaho,
        author: None,
        is_official: false,
        similarity: None,
        votes: None,
        rafsi: Vec::new(),
        glosses: Vec::new(),
        definition: String::new(),
        notes: String::new(),
        etymology: None,
        decomposition: decomposition
            .map(composition_from_decomposition)
            .unwrap_or_default(),
    })
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
            LujvoPart::Rafsi(phonemes) => VlackuCompositionPiece {
                kind: VlackuCompositionKind::Rafsi,
                surface: phonemes.as_str().to_owned(),
                source: segment.source.map(str::to_owned),
            },
            LujvoPart::Hyphen(phonemes) => VlackuCompositionPiece {
                kind: VlackuCompositionKind::Hyphen,
                surface: phonemes.as_str().to_owned(),
                source: None,
            },
        })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn composition_from_dictionary_decomposition(
    decomposition: &DictionaryLujvoEntry<'_>,
) -> Vec<VlackuCompositionPiece> {
    decomposition
        .segments
        .iter()
        .map(|segment| match segment.kind {
            DictionaryLujvoSegmentKind::Rafsi => VlackuCompositionPiece {
                kind: VlackuCompositionKind::Rafsi,
                surface: segment.surface.to_owned(),
                source: segment.source_word.map(str::to_owned),
            },
            DictionaryLujvoSegmentKind::Hyphen => VlackuCompositionPiece {
                kind: VlackuCompositionKind::Hyphen,
                surface: segment.surface.to_owned(),
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
fn passes_filters(card: &VlackuCard, options: &VlackuSearchOptions, similarity_mode: bool) -> bool {
    let card_word_type = WordTypeFilter::parse(&card.word_type);
    let word_type_ok = options.word_types.is_empty()
        || card_word_type.is_some_and(|actual| {
            options
                .word_types
                .iter()
                .any(|wanted| wanted.matches_filter(actual))
        });
    let votes_ok = options
        .min_votes
        .is_none_or(|min_votes| card.votes.unwrap_or(0) >= min_votes);
    let similarity_ok = !similarity_mode
        || options
            .min_similarity
            .is_none_or(|min_similarity| card.similarity.unwrap_or(0.0) * 100.0 >= min_similarity);
    word_type_ok && votes_ok && similarity_ok
}

#[derive(Debug, Clone)]
#[invariant(true)]
#[invariant(::Glob(_) => true)]
#[invariant(::Regex(_) => true)]
enum ExactPattern {
    Glob(GlobPattern),
    Regex(Regex),
}

impl ExactPattern {
    #[requires(true)]
    #[ensures(true)]
    fn matches(&self, target: &str) -> bool {
        match self {
            Self::Glob(pattern) => pattern.matches(target),
            Self::Regex(pattern) => pattern.is_match(target),
        }
    }
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
        glob_matches(&self.tokens, &target.chars().collect::<Vec<_>>())
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
#[ensures(ret.is_ok() || ret.as_ref().err().is_some_and(|message| !message.is_empty()))]
fn compile_exact_pattern(pattern: &str) -> Result<ExactPattern, String> {
    if pattern.starts_with('/') {
        compile_regex_pattern(pattern).map(ExactPattern::Regex)
    } else {
        compile_glob_pattern(pattern).map(ExactPattern::Glob)
    }
}

#[requires(pattern.starts_with('/'))]
#[ensures(ret.is_ok() || ret.as_ref().err().is_some_and(|message| !message.is_empty()))]
fn compile_regex_pattern(pattern: &str) -> Result<Regex, String> {
    let Some(body) = regex_pattern_body(pattern) else {
        return Err(format!(
            "Invalid regex pattern `{pattern}`: regex searches must be written as /.../."
        ));
    };
    if body.is_empty() {
        return Err(format!(
            "Invalid regex pattern `{pattern}`: regex body must not be empty."
        ));
    }

    // Regex is matched as an unanchored substring (like grep), so the caller's
    // own `^`/`$` anchors are meaningful: `/kla/` matches anywhere, `/^kla/`
    // anchors to the start, `/^kla$/` requires the whole word. (Glob patterns,
    // by contrast, always describe the whole word.) Case-insensitive by default;
    // the caller can override with an inline `(?-i)` flag.
    let pattern_source = format!("(?i){body}");
    Regex::new(&pattern_source)
        .map_err(|error| format!("Invalid regex pattern `{pattern}`: {error}"))
}

#[requires(pattern.starts_with('/'))]
#[ensures(ret.is_none() || pattern.ends_with('/'))]
fn regex_pattern_body(pattern: &str) -> Option<&str> {
    if pattern.len() < 2 || !pattern.ends_with('/') {
        return None;
    }
    Some(&pattern[1..pattern.len() - 1])
}

#[requires(true)]
#[ensures(true)]
#[ensures(ret.as_ref().is_ok_and(|pattern| !pattern.tokens.is_empty()) || ret.is_err())]
fn compile_glob_pattern(pattern: &str) -> Result<GlobPattern, String> {
    let mut tokens = Vec::new();
    for raw in pattern.chars() {
        match raw {
            '$' => tokens.push(GlobToken::Consonant),
            '@' => tokens.push(GlobToken::Vowel),
            '?' => tokens.push(GlobToken::AnyOne),
            '*' => {
                if tokens.last() != Some(&GlobToken::AnyMany) {
                    tokens.push(GlobToken::AnyMany);
                }
            }
            value => {
                if let Some(normalized) = normalize_glob_literal(value) {
                    tokens.push(GlobToken::Literal(normalized));
                } else {
                    return Err(format!(
                        "Invalid glob pattern `{pattern}`: unsupported character `{value}`."
                    ));
                }
            }
        }
    }
    if tokens.is_empty() {
        Err("Glob pattern must contain at least one searchable character.".to_owned())
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
fn exact_pattern_target_key(raw_word: &str) -> String {
    normalize_lookup_query(raw_word)
        .chars()
        .map(|value| if value == 'h' { '\'' } else { value })
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn is_glob_wildcard(value: char) -> bool {
    matches!(value, '@' | '$' | '?' | '*')
}

#[requires(true)]
#[ensures(true)]
fn entry_has_matching_rafsi_pattern(entry: &DictionaryEntry<'_>, pattern: &ExactPattern) -> bool {
    entry.rafsi.iter().any(|rafsi| {
        let key = exact_pattern_target_key(rafsi.0);
        pattern.matches(&key)
    }) || entry.word_type.is_gismu_like()
        && universal_gismu_rafsi_forms(entry.word)
            .into_iter()
            .any(|(rafsi, _source)| {
                let key = exact_pattern_target_key(&rafsi);
                pattern.matches(&key)
            })
}

#[requires(true)]
#[ensures(true)]
fn glob_matches(tokens: &[GlobToken], target: &[char]) -> bool {
    let mut previous = vec![false; target.len() + 1];
    previous[0] = true;

    for token in tokens {
        let mut current = vec![false; target.len() + 1];
        match token {
            GlobToken::AnyMany => {
                current[0] = previous[0];
                for target_index in 1..=target.len() {
                    current[target_index] = previous[target_index] || current[target_index - 1];
                }
            }
            token => {
                for target_index in 1..=target.len() {
                    current[target_index] = previous[target_index - 1]
                        && glob_token_matches(*token, target[target_index - 1]);
                }
            }
        }
        previous = current;
    }
    previous[target.len()]
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

#[cfg(test)]
mod tests {
    use super::*;
    use bityzba::requires;
    use jbotci_morphology::segment_words_with_modifiers;

    #[requires(true)]
    #[ensures(true)]
    fn filter(value: &str) -> WordTypeFilter {
        WordTypeFilter::parse(value).expect("known word type filter")
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn grouped_word_type_filters_keep_letterals_and_phrases_out_of_brivla() {
        assert!(matches_word_type_filter(
            filter("letteral"),
            filter("bu-letteral")
        ));
        assert!(!matches_word_type_filter(
            filter("cmavo"),
            filter("bu-letteral")
        ));
        assert!(!matches_word_type_filter(
            filter("brivla"),
            filter("bu-letteral")
        ));
        assert!(!matches_word_type_filter(
            filter("brivla"),
            filter("phrase")
        ));
        assert!(matches_word_type_filter(filter("brivla"), filter("gismu")));
        assert!(matches_word_type_filter(filter("brivla"), filter("lujvo")));
        assert!(matches_word_type_filter(
            filter("brivla"),
            filter("fu'ivla")
        ));
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
    fn dictionary_entry_cards_include_author_and_official_status() {
        let entry = jbotci_dictionary_data::english()
            .lookup_word("klama")
            .expect("entry for klama");
        let card =
            dictionary_entry_card(jbotci_dictionary_data::english(), entry, Some(1.0), false);

        let author = card.author.as_ref().expect("author");
        assert_eq!(author.username, "officialdata");
        assert_eq!(author.realname.as_deref(), Some("Official Data"));
        assert!(card.is_official);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dictionary_entry_cards_include_etymology() {
        let entry = jbotci_dictionary_data::english()
            .lookup_word("abniena")
            .expect("entry for abniena");
        let card =
            dictionary_entry_card(jbotci_dictionary_data::english(), entry, Some(1.0), false);

        let etymology = card.etymology.as_deref().expect("etymology");
        assert!(etymology.contains("ava, people"), "{etymology}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn dictionary_entry_cards_do_not_decompose_non_lujvo_entries() {
        let dictionary = jbotci_dictionary_data::english();
        let entry = dictionary
            .entries()
            .iter()
            .find(|entry| entry.word.starts_with("cu'u'u'u"))
            .expect("long cmavo entry");
        assert!(is_cmavo_like(&normalize_word_type_filter(
            entry.word_type.as_str()
        )));

        let card = dictionary_entry_card(dictionary, entry, Some(1.0), true);

        assert!(card.decomposition.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn official_status_uses_author_not_vote_threshold() {
        let entry = jbotci_dictionary_data::english()
            .lookup_word("birka")
            .expect("entry for birka");
        let card =
            dictionary_entry_card(jbotci_dictionary_data::english(), entry, Some(1.0), false);

        assert_eq!(card.votes, Some(10000));
        assert!(card.is_official);
        assert_eq!(
            format_vote_display(card.votes.unwrap(), card.is_official),
            "∞"
        );
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
    fn exact_valsi_search_normalizes_non_latin_glide_words() {
        for query in ["шой", "\u{ed86}\u{eda8}"] {
            let result = run_vlacku_requests(
                jbotci_dictionary_data::english(),
                &[VlackuRequest::valsi(query.to_owned())],
                &VlackuSearchOptions::default(),
            );

            assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
            assert_eq!(result.outcome, VlackuOutcome::Found, "{query}");
            assert_eq!(
                result.cards.first().map(|card| card.word.as_str()),
                Some("coi"),
                "{query}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_valsi_glob_matches_words() {
        let result = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::valsi("klam@".to_owned())],
            &VlackuSearchOptions::default(),
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.outcome, VlackuOutcome::Found);
        assert_eq!(
            result.cards.first().map(|card| card.word.as_str()),
            Some("klama")
        );

        let broad = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::valsi("$$@$@".to_owned())],
            &VlackuSearchOptions::default().with_data(data! { count: usize::MAX }),
        );
        assert!(broad.cards.iter().any(|card| card.word == "klama"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_glob_collapses_repeated_stars_and_matches_with_dp() {
        let repeated_stars = "*".repeat(512);
        let pattern =
            compile_glob_pattern(&format!("{repeated_stars}x")).expect("valid glob pattern");

        assert_eq!(
            pattern.tokens,
            vec![GlobToken::AnyMany, GlobToken::Literal('x')]
        );
        assert!(pattern.matches(&format!("{}x", "a".repeat(512))));
        assert!(!pattern.matches(&"a".repeat(512)));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_valsi_glob_reports_invalid_characters() {
        let result = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::valsi("kl!m*".to_owned())],
            &VlackuSearchOptions::default(),
        );

        assert_eq!(result.outcome, VlackuOutcome::Invalid);
        assert!(result.cards.is_empty());
        assert!(
            result
                .diagnostics
                .first()
                .is_some_and(|message| message.contains("Invalid glob pattern"))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_pattern_cards_stop_after_requested_display_count() {
        let dictionary = jbotci_dictionary_data::english();
        let compiled = compile_exact_pattern("*").expect("match-all glob compiles");
        let mut visited = 0;
        let cards = cards_for_matching_entries(
            dictionary,
            dictionary
                .entries()
                .iter()
                .inspect(|_| visited += 1)
                .filter(|entry| compiled.matches(&exact_pattern_target_key(entry.word))),
            &VlackuSearchOptions::default().with_data(data! { count: 1 }),
        );

        assert_eq!(cards.len(), 1);
        assert_eq!(visited, 1);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_pattern_filters_entries_before_building_cards() {
        let dictionary = jbotci_dictionary_data::english();
        let compiled = compile_exact_pattern("*").expect("match-all glob compiles");
        let options = VlackuSearchOptions::default().with_data(data! {
            count: 2,
            word_types: vec![WordTypeFilter::Cmavo],
            decompose_lujvo: true,
        });
        let mut visited = 0;
        let mut built = 0;

        let cards = cards_for_matching_entries_with_builder(
            dictionary
                .entries()
                .iter()
                .inspect(|_| visited += 1)
                .filter(|entry| compiled.matches(&exact_pattern_target_key(entry.word))),
            &options,
            |entry| {
                built += 1;
                dictionary_entry_card(dictionary, entry, Some(1.0), options.decompose_lujvo)
            },
        );

        assert_eq!(cards.len(), 2);
        assert_eq!(built, cards.len());
        assert!(visited < dictionary.entries().len());
        assert!(cards.iter().all(|card| {
            WordTypeFilter::parse(&card.word_type)
                .is_some_and(|actual| WordTypeFilter::Cmavo.matches_filter(actual))
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_pattern_decompose_lujvo_skips_non_lujvo_cards() {
        let result = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::valsi("/.{10,}/".to_owned())],
            &VlackuSearchOptions::default().with_data(data! {
                count: 40,
                word_types: vec![WordTypeFilter::Cmavo],
                decompose_lujvo: true,
            }),
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.outcome, VlackuOutcome::Found);
        assert!(!result.cards.is_empty());
        assert!(result.cards.iter().all(|card| {
            WordTypeFilter::parse(&card.word_type)
                .is_some_and(|actual| WordTypeFilter::Cmavo.matches_filter(actual))
        }));
        assert!(
            result
                .cards
                .iter()
                .all(|card| card.decomposition.is_empty())
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn entry_filters_match_card_filters_for_dictionary_entries() {
        let dictionary = jbotci_dictionary_data::english();
        let entry = dictionary.lookup_word("klama").expect("entry for klama");
        let card = dictionary_entry_card(dictionary, entry, Some(0.42), false);

        for options in [
            VlackuSearchOptions::default().with_data(data! {
                word_types: vec![WordTypeFilter::Brivla],
            }),
            VlackuSearchOptions::default().with_data(data! {
                word_types: vec![WordTypeFilter::Cmavo],
            }),
            VlackuSearchOptions::default().with_data(data! {
                min_votes: Some(1),
            }),
            VlackuSearchOptions::default().with_data(data! {
                min_similarity: Some(50.0),
            }),
        ] {
            assert_eq!(
                dictionary_entry_passes_vlacku_filters(entry, &options, Some(0.42), true),
                passes_filters(&card, &options, true)
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sound_search_uses_precomputed_dictionary_index() {
        let result = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::sound("klama".to_owned())],
            &VlackuSearchOptions::default().with_data(data! { count: 1 }),
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.outcome, VlackuOutcome::Found);
        assert_eq!(
            result.cards.first().map(|card| card.word.as_str()),
            Some("klama")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sound_score_retention_keeps_best_scores_with_dictionary_order_ties() {
        let dictionary = jbotci_dictionary_data::english();
        let entry = dictionary.lookup_word("klama").expect("entry for klama");
        let mut scored = vec![
            (4, 0.60, entry),
            (2, 0.90, entry),
            (1, 0.90, entry),
            (3, 0.10, entry),
        ];

        retain_best_sound_scores(&mut scored, 2);

        let retained = scored
            .iter()
            .map(|(index, similarity, _)| (*index, *similarity))
            .collect::<Vec<_>>();
        assert_eq!(retained, vec![(1, 0.90), (2, 0.90)]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_rafsi_glob_matches_listed_and_universal_rafsi() {
        let listed = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::rafsi("kl@".to_owned())],
            &VlackuSearchOptions::default(),
        );

        assert!(listed.diagnostics.is_empty(), "{:?}", listed.diagnostics);
        assert_eq!(listed.outcome, VlackuOutcome::Found);
        assert!(listed.cards.iter().any(|card| card.word == "klama"));

        let universal = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::rafsi("klam@".to_owned())],
            &VlackuSearchOptions::default(),
        );

        assert!(
            universal.diagnostics.is_empty(),
            "{:?}",
            universal.diagnostics
        );
        assert_eq!(universal.outcome, VlackuOutcome::Found);
        assert!(universal.cards.iter().any(|card| card.word == "klama"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_valsi_regex_is_substring_and_case_insensitive_by_default() {
        // Fully anchored and upper-case: still resolves to exactly "klama"
        // (anchors honored, case-insensitive by default).
        let exact = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::valsi("/^KLAMA$/".to_owned())],
            &VlackuSearchOptions::default(),
        );
        assert!(exact.diagnostics.is_empty(), "{:?}", exact.diagnostics);
        assert_eq!(exact.outcome, VlackuOutcome::Found);
        assert_eq!(
            exact.cards.first().map(|card| card.word.as_str()),
            Some("klama")
        );

        // Unanchored substring: `/lam/` matches words that merely contain "lam"
        // (e.g. "klama"), which the old full-match behavior would have rejected.
        let substring = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::valsi("/lam/".to_owned())],
            &VlackuSearchOptions::default(),
        );
        assert!(
            substring.diagnostics.is_empty(),
            "{:?}",
            substring.diagnostics
        );
        assert_eq!(substring.outcome, VlackuOutcome::Found);
        assert!(
            substring
                .cards
                .iter()
                .all(|card| card.word.to_lowercase().contains("lam")),
            "{:?}",
            substring.cards.iter().map(|c| &c.word).collect::<Vec<_>>()
        );

        // The caller's own anchors are honored: `/^kla/` restricts the match to
        // words that START with "kla".
        let prefix = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::valsi("/^kla/".to_owned())],
            &VlackuSearchOptions::default(),
        );
        assert_eq!(prefix.outcome, VlackuOutcome::Found);
        assert!(
            prefix
                .cards
                .iter()
                .all(|card| card.word.to_lowercase().starts_with("kla")),
            "{:?}",
            prefix.cards.iter().map(|c| &c.word).collect::<Vec<_>>()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_valsi_regex_allows_user_to_disable_case_insensitive_mode() {
        let result = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::valsi("/(?-i)KLAMA/".to_owned())],
            &VlackuSearchOptions::default(),
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(result.outcome, VlackuOutcome::ValidMissing);
        assert!(result.cards.is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_valsi_regex_reports_invalid_patterns() {
        let result = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::valsi("/[/".to_owned())],
            &VlackuSearchOptions::default(),
        );

        assert_eq!(result.outcome, VlackuOutcome::Invalid);
        assert!(result.cards.is_empty());
        assert!(
            result
                .diagnostics
                .first()
                .is_some_and(|message| message.contains("Invalid regex pattern"))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn punctuation_only_exact_queries_are_invalid() {
        for request in [
            VlackuRequest::valsi("!!!".to_owned()),
            VlackuRequest::rafsi("!!!".to_owned()),
            VlackuRequest::lujvo("!!!".to_owned()),
        ] {
            let result = run_vlacku_requests(
                jbotci_dictionary_data::english(),
                &[request],
                &VlackuSearchOptions::default(),
            );

            assert_eq!(result.outcome, VlackuOutcome::Invalid);
            assert!(result.cards.is_empty(), "{:?}", result.cards);
            let expected = format!("{INVALID_LOJBAN_WORD_MESSAGE_PREFIX}!!!");
            assert_eq!(
                result.diagnostics.first().map(String::as_str),
                Some(expected.as_str())
            );
        }
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
    fn exact_lujvo_search_includes_dictionary_backed_fuhivla_component_card() {
        let result = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::valsi("jenjigu'ydi'e".to_owned())],
            &VlackuSearchOptions::default().with_data(data! {
                decompose_lujvo: true,
            }),
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.cards.first().map(|card| card.word.as_str()),
            Some("jenjigu'ydi'e")
        );
        assert!(result.cards.iter().any(|card| card.word == "jenjigu"));
        assert!(result.cards.iter().any(|card| card.word == "dirce"));
        let headword = result.cards.first().expect("headword card");
        assert!(headword.decomposition.iter().any(|piece| {
            piece.surface == "jenjigu" && piece.source.as_deref() == Some("jenjigu")
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_lujvo_search_uses_dictionary_backed_r_hyphen_decomposition() {
        let result = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::valsi("ci'artai".to_owned())],
            &VlackuSearchOptions::default().with_data(data! {
                decompose_lujvo: true,
            }),
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.cards.first().map(|card| card.word.as_str()),
            Some("ci'artai")
        );
        assert!(result.cards.iter().any(|card| card.word == "ciska"));
        assert!(result.cards.iter().any(|card| card.word == "tarmi"));
        let headword = result.cards.first().expect("headword card");
        let decomposition = headword
            .decomposition
            .iter()
            .map(|piece| (piece.kind, piece.surface.as_str(), piece.source.as_deref()))
            .collect::<Vec<_>>();
        assert_eq!(
            decomposition,
            [
                (VlackuCompositionKind::Rafsi, "ci'á", Some("ciska")),
                (VlackuCompositionKind::Hyphen, "r", None),
                (VlackuCompositionKind::Rafsi, "taĭ", Some("tarmi")),
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_garden_path_fuhivla_search_does_not_add_lujvo_source_cards() {
        let result = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::valsi("pudlu'avalsi'ipo'ato".to_owned())],
            &VlackuSearchOptions::default().with_data(data! {
                decompose_lujvo: true,
            }),
        );

        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        assert_eq!(
            result.cards.first().map(|card| card.word.as_str()),
            Some("pudlu'avalsi'ipo'ato")
        );
        assert!(
            result
                .cards
                .first()
                .is_some_and(|card| card.decomposition.is_empty())
        );
        assert!(!result.cards.iter().any(|card| card.word == "purdi"));
        assert!(!result.cards.iter().any(|card| card.word == "valsi"));
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
