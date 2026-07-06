//! Borrowed dictionary model and lookup support.

#[cfg(feature = "import")]
pub mod import;

use std::collections::BTreeMap;

use bityzba::{expensive_invariant, invariant, requires};
use jbotci_phonetic::IpaTokenSequenceView;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Complete borrowed dictionary plus lookup indexes.
#[derive(Debug, Clone, Copy)]
#[invariant(
    true,
    "dictionary-wide validity is checked by validate and the expensive impl invariant"
)]
pub struct Dictionary<'a> {
    entries: &'a [DictionaryEntry<'a>],
    word_index: &'a [WordIndexEntry<'a>],
    rafsi_index: &'a [RafsiIndexEntry<'a>],
    selmaho_index: &'a [SelmahoIndexEntry<'a>],
    pattern_index: &'a [DictionaryPatternEntry<'a>],
    sound_index: &'a [DictionarySoundEntry<'a>],
    lujvo_index: &'a [DictionaryLujvoEntry<'a>],
}

impl<'a> Dictionary<'a> {
    /// Construct a dictionary from generated static slices.
    ///
    /// This is intentionally low level: callers must validate the same slices
    /// before generating them, and tests should call [`Dictionary::validate`]
    /// on the resulting value.
    #[requires(true)]
    #[ensures(true)]
    pub const fn from_static_slices(
        entries: &'a [DictionaryEntry<'a>],
        word_index: &'a [WordIndexEntry<'a>],
        rafsi_index: &'a [RafsiIndexEntry<'a>],
        selmaho_index: &'a [SelmahoIndexEntry<'a>],
        pattern_index: &'a [DictionaryPatternEntry<'a>],
        sound_index: &'a [DictionarySoundEntry<'a>],
        lujvo_index: &'a [DictionaryLujvoEntry<'a>],
    ) -> Self {
        Self {
            entries,
            word_index,
            rafsi_index,
            selmaho_index,
            pattern_index,
            sound_index,
            lujvo_index,
        }
    }

    /// Validate that all generated indexes match the entry table.
    #[requires(true)]
    #[ensures(true)]
    pub fn validate(&self) -> Result<(), DictionaryValidationError> {
        for (index, entry) in self.entries.iter().enumerate() {
            validate_entry(index, entry)?;
        }

        let expected = build_owned_indexes(self.entries);
        if !word_index_matches(self.word_index, &expected.word_index) {
            return Err(DictionaryValidationError::WordIndexMismatch);
        }
        if !rafsi_index_matches(self.rafsi_index, &expected.rafsi_index) {
            return Err(DictionaryValidationError::RafsiIndexMismatch);
        }
        if !selmaho_index_matches(self.selmaho_index, &expected.selmaho_index) {
            return Err(DictionaryValidationError::SelmahoIndexMismatch);
        }
        if !pattern_index_matches(self.pattern_index, &expected.pattern_index) {
            return Err(DictionaryValidationError::PatternIndexMismatch);
        }
        validate_sound_index(self.entries, self.sound_index)?;
        validate_lujvo_index(self.entries, self.lujvo_index)?;
        Ok(())
    }

    #[requires(index.0 < self.entries.len())]
    #[ensures(ret.word == self.entries[index.0].word)]
    fn entry_at(&self, index: EntryIndex) -> &DictionaryEntry<'a> {
        &self.entries[index.0]
    }
}

#[expensive_invariant(self.validate().is_ok(), "dictionary lookup indexes must match entries")]
impl<'a> Dictionary<'a> {
    /// Return all entries in source order.
    #[requires(true)]
    #[ensures(ret.len() == self.entries.len())]
    pub fn entries(&self) -> &'a [DictionaryEntry<'a>] {
        self.entries
    }

    /// Return the generated sound-search index in dictionary entry order.
    #[requires(true)]
    #[ensures(true)]
    pub fn sound_index(&self) -> &'a [DictionarySoundEntry<'a>] {
        self.sound_index
    }

    /// Return the generated dictionary-entry lujvo decomposition index.
    #[requires(true)]
    #[ensures(true)]
    pub fn lujvo_index(&self) -> &'a [DictionaryLujvoEntry<'a>] {
        self.lujvo_index
    }

    /// Return generated pattern-search keys in dictionary entry order.
    #[requires(true)]
    #[ensures(ret.len() == self.entries.len())]
    pub fn pattern_index(&self) -> &'a [DictionaryPatternEntry<'a>] {
        self.pattern_index
    }

    /// Return a dictionary entry by generated entry index.
    #[requires(true)]
    #[ensures(ret.is_none_or(|entry| self.entry_index_for_entry(entry) == Some(index)))]
    pub fn entry_for_index(&self, index: EntryIndex) -> Option<&'a DictionaryEntry<'a>> {
        self.entries.get(index.0)
    }

    /// Return generated lujvo decomposition data for an entry index.
    #[requires(true)]
    #[ensures(ret.as_ref().is_some_and(|entry| entry.entry_index == index) || ret.is_none())]
    pub fn lujvo_decomposition_for_entry_index(
        &self,
        index: EntryIndex,
    ) -> Option<&'a DictionaryLujvoEntry<'a>> {
        self.lujvo_index
            .binary_search_by(|entry| entry.entry_index.cmp(&index))
            .ok()
            .map(|position| &self.lujvo_index[position])
    }

    /// Return an entry's index when the reference belongs to this dictionary.
    #[requires(true)]
    #[ensures(ret.is_none_or(|index| index.0 < self.entries.len()))]
    pub fn entry_index_for_entry(&self, entry: &DictionaryEntry<'a>) -> Option<EntryIndex> {
        self.entries
            .iter()
            .position(|candidate| std::ptr::eq(candidate, entry))
            .map(EntryIndex)
    }

    /// Return the first entry matching a normalized lookup query.
    #[requires(true)]
    #[ensures(true)]
    pub fn lookup_word(&self, query: &str) -> Option<&DictionaryEntry<'a>> {
        self.lookup_words(query).next()
    }

    /// Return all entries matching a normalized lookup query.
    #[requires(true)]
    #[ensures(true)]
    pub fn lookup_words<'lookup>(
        &'lookup self,
        query: &str,
    ) -> impl Iterator<Item = &'lookup DictionaryEntry<'a>> + 'lookup {
        let normalized = normalize_lookup_query(query);
        let targets = self
            .word_index_entry(&normalized)
            .map_or(&[][..], |entry| entry.targets);
        targets.iter().map(|index| self.entry_at(*index))
    }

    /// Return all dictionary entries associated with a rafsi query.
    #[requires(true)]
    #[ensures(true)]
    pub fn lookup_rafsi<'lookup>(
        &'lookup self,
        query: &str,
    ) -> impl Iterator<Item = RafsiMatch<'lookup, 'a>> + 'lookup {
        let normalized = normalize_lookup_query(query);
        let targets = self
            .rafsi_index_entry(&normalized)
            .map_or(&[][..], |entry| entry.targets);
        targets.iter().map(|target| RafsiMatch {
            entry: self.entry_at(target.entry_index),
            source: target.source,
        })
    }

    /// Return all entries whose raw selma'o string matches exactly.
    #[requires(true)]
    #[ensures(true)]
    pub fn entries_by_selmaho<'lookup>(
        &'lookup self,
        selmaho: &str,
    ) -> impl Iterator<Item = &'lookup DictionaryEntry<'a>> + 'lookup {
        let targets = self
            .selmaho_index_entry(selmaho)
            .map_or(&[][..], |entry| entry.targets);
        targets.iter().map(|index| self.entry_at(*index))
    }

    #[requires(true)]
    #[ensures(true)]
    fn word_index_entry(&self, key: &str) -> Option<&WordIndexEntry<'a>> {
        self.word_index
            .binary_search_by(|entry| entry.key.cmp(key))
            .ok()
            .map(|index| &self.word_index[index])
    }

    #[requires(true)]
    #[ensures(true)]
    fn rafsi_index_entry(&self, key: &str) -> Option<&RafsiIndexEntry<'a>> {
        self.rafsi_index
            .binary_search_by(|entry| entry.key.cmp(key))
            .ok()
            .map(|index| &self.rafsi_index[index])
    }

    #[requires(true)]
    #[ensures(true)]
    fn selmaho_index_entry(&self, key: &str) -> Option<&SelmahoIndexEntry<'a>> {
        self.selmaho_index
            .binary_search_by(|entry| entry.key.cmp(key))
            .ok()
            .map(|index| &self.selmaho_index[index])
    }
}

/// Single dictionary entry in the imported Lensisku data.
#[derive(Debug, Clone, Copy, PartialEq)]
#[invariant(
    true,
    "dictionary entry field consistency is checked by Dictionary::validate"
)]
pub struct DictionaryEntry<'a> {
    pub word: &'a str,
    pub word_type: WordType,
    pub definition: &'a str,
    pub definition_id: DefinitionId,
    pub notes: &'a str,
    pub score: Score,
    pub gloss_keywords: &'a [Keyword<'a>],
    pub place_keywords: &'a [Keyword<'a>],
    pub rafsi: &'a [Rafsi<'a>],
    pub selmaho: Option<RawSelmaho<'a>>,
    pub etymology: Option<&'a str>,
    pub jargon: Option<&'a str>,
    pub user: DictionaryUser<'a>,
}

/// Precomputed sound-search data for a pronounceable dictionary entry.
#[derive(Debug, Clone, Copy, PartialEq)]
#[invariant(true)]
pub struct DictionarySoundEntry<'a> {
    pub entry_index: EntryIndex,
    pub ipa: &'a str,
    pub token_sequence: IpaTokenSequenceView<'a>,
}

/// Precomputed lujvo decomposition data for a dictionary entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(
    true,
    "dictionary lujvo decomposition consistency is checked by Dictionary::validate"
)]
pub struct DictionaryLujvoEntry<'a> {
    pub entry_index: EntryIndex,
    pub segments: &'a [DictionaryLujvoSegment<'a>],
    pub source_words: &'a [&'a str],
}

/// Single precomputed lujvo decomposition segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(
    true,
    "dictionary lujvo decomposition segment consistency is checked by Dictionary::validate"
)]
pub struct DictionaryLujvoSegment<'a> {
    pub kind: DictionaryLujvoSegmentKind,
    pub surface: &'a str,
    pub source_word: Option<&'a str>,
}

/// Segment kind in a precomputed lujvo decomposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Rafsi => true)]
#[invariant(::Hyphen => true)]
pub enum DictionaryLujvoSegmentKind {
    Rafsi,
    Hyphen,
}

/// Lensisku dictionary word type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum WordType {
    #[serde(rename = "gismu")]
    Gismu,
    #[serde(rename = "experimental gismu")]
    ExperimentalGismu,
    #[serde(rename = "lujvo")]
    Lujvo,
    #[serde(rename = "zei-lujvo")]
    ZeiLujvo,
    #[serde(rename = "obsolete zei-lujvo")]
    ObsoleteZeiLujvo,
    #[serde(rename = "cmavo")]
    Cmavo,
    #[serde(rename = "experimental cmavo")]
    ExperimentalCmavo,
    #[serde(rename = "obsolete cmavo")]
    ObsoleteCmavo,
    #[serde(rename = "cmavo-compound")]
    CmavoCompound,
    #[serde(rename = "fu'ivla")]
    Fuivla,
    #[serde(rename = "obsolete fu'ivla")]
    ObsoleteFuivla,
    #[serde(rename = "cmevla")]
    Cmevla,
    #[serde(rename = "obsolete cmevla")]
    ObsoleteCmevla,
    #[serde(rename = "bu-letteral")]
    BuLetteral,
    #[serde(rename = "phrase")]
    Phrase,
}

impl WordType {
    /// Return whether this type is a gismu class for rafsi purposes.
    #[requires(true)]
    #[ensures(true)]
    pub const fn is_gismu_like(self) -> bool {
        matches!(self, Self::Gismu | Self::ExperimentalGismu)
    }

    /// Return whether this type is a lujvo-like class.
    #[requires(true)]
    #[ensures(matches!(self, Self::Lujvo | Self::ZeiLujvo | Self::ObsoleteZeiLujvo) == ret)]
    pub const fn is_lujvo_like(self) -> bool {
        matches!(self, Self::Lujvo | Self::ZeiLujvo | Self::ObsoleteZeiLujvo)
    }

    /// Return the Lensisku string representation.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Gismu => "gismu",
            Self::ExperimentalGismu => "experimental gismu",
            Self::Lujvo => "lujvo",
            Self::ZeiLujvo => "zei-lujvo",
            Self::ObsoleteZeiLujvo => "obsolete zei-lujvo",
            Self::Cmavo => "cmavo",
            Self::ExperimentalCmavo => "experimental cmavo",
            Self::ObsoleteCmavo => "obsolete cmavo",
            Self::CmavoCompound => "cmavo-compound",
            Self::Fuivla => "fu'ivla",
            Self::ObsoleteFuivla => "obsolete fu'ivla",
            Self::Cmevla => "cmevla",
            Self::ObsoleteCmevla => "obsolete cmevla",
            Self::BuLetteral => "bu-letteral",
            Self::Phrase => "phrase",
        }
    }
}

/// Lensisku definition id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
#[invariant(true)]
pub struct DefinitionId(pub u64);

impl DefinitionId {
    /// Return the raw numeric id.
    #[requires(true)]
    #[ensures(true)]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Lensisku score.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
#[invariant(true)]
pub struct Score(pub f64);

impl Score {
    /// Return the raw score.
    #[requires(true)]
    #[ensures(true)]
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Gloss or place keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub struct Keyword<'a> {
    pub word: &'a str,
    pub meaning: Option<&'a str>,
}

/// Listed rafsi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[invariant(true)]
pub struct Rafsi<'a>(pub &'a str);

/// Raw Lensisku selma'o string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[invariant(true)]
pub struct RawSelmaho<'a>(pub &'a str);

/// Lensisku contributor metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub struct DictionaryUser<'a> {
    pub username: &'a str,
    pub realname: Option<&'a str>,
}

/// Entry index into a dictionary entry slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[invariant(true)]
pub struct EntryIndex(pub usize);

impl EntryIndex {
    /// Return the raw index.
    #[requires(true)]
    #[ensures(true)]
    pub const fn get(self) -> usize {
        self.0
    }
}

/// Word lookup index entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub struct WordIndexEntry<'a> {
    pub key: &'a str,
    pub targets: &'a [EntryIndex],
}

/// Rafsi lookup index entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub struct RafsiIndexEntry<'a> {
    pub key: &'a str,
    pub targets: &'a [RafsiIndexTarget],
}

/// Selma'o lookup index entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub struct SelmahoIndexEntry<'a> {
    pub key: &'a str,
    pub targets: &'a [EntryIndex],
}

/// Precomputed pattern-search keys for one dictionary entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(
    true,
    "borrowed pattern index entries are static generated data validated against dictionary entries and normalized key generation"
)]
pub struct DictionaryPatternEntry<'a> {
    pub entry_index: EntryIndex,
    pub word_key: &'a str,
    pub rafsi_keys: &'a [&'a str],
}

/// Rafsi target plus provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[invariant(true)]
pub struct RafsiIndexTarget {
    pub entry_index: EntryIndex,
    pub source: RafsiSource,
}

/// Source of a rafsi lookup match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RafsiSource {
    Listed,
    UniversalShort,
    UniversalLong,
}

/// Result of a rafsi lookup.
#[derive(Debug, Clone, Copy, PartialEq)]
#[invariant(true)]
pub struct RafsiMatch<'entry, 'dict> {
    pub entry: &'entry DictionaryEntry<'dict>,
    pub source: RafsiSource,
}

/// Owned indexes used by importers and validation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct OwnedDictionaryIndexes {
    pub word_index: Vec<OwnedWordIndexEntry>,
    pub rafsi_index: Vec<OwnedRafsiIndexEntry>,
    pub selmaho_index: Vec<OwnedSelmahoIndexEntry>,
    pub pattern_index: Vec<OwnedPatternIndexEntry>,
}

/// Owned word index entry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct OwnedWordIndexEntry {
    pub key: String,
    pub targets: Vec<EntryIndex>,
}

/// Owned rafsi index entry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct OwnedRafsiIndexEntry {
    pub key: String,
    pub targets: Vec<RafsiIndexTarget>,
}

/// Owned selma'o index entry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct OwnedSelmahoIndexEntry {
    pub key: String,
    pub targets: Vec<EntryIndex>,
}

/// Owned pattern index entry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(
    true,
    "owned pattern index entries are produced by build_owned_indexes"
)]
pub struct OwnedPatternIndexEntry {
    pub entry_index: EntryIndex,
    pub word_key: String,
    pub rafsi_keys: Vec<String>,
}

/// Validation error for generated dictionary tables.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::InvalidEntry => true)]
#[invariant(::InvalidSoundIndexEntry => true)]
#[invariant(::InvalidLujvoIndexEntry => true)]
pub enum DictionaryValidationError {
    #[error("invalid dictionary entry at index {index}: {reason}")]
    InvalidEntry { index: usize, reason: &'static str },
    #[error("word index does not match dictionary entries")]
    WordIndexMismatch,
    #[error("rafsi index does not match dictionary entries")]
    RafsiIndexMismatch,
    #[error("selma'o index does not match dictionary entries")]
    SelmahoIndexMismatch,
    #[error("pattern index does not match dictionary entries")]
    PatternIndexMismatch,
    #[error("invalid dictionary sound index entry at index {index}: {reason}")]
    InvalidSoundIndexEntry { index: usize, reason: &'static str },
    #[error("invalid dictionary lujvo index entry at index {index}: {reason}")]
    InvalidLujvoIndexEntry { index: usize, reason: &'static str },
}

/// Build owned indexes for a borrowed entry table.
#[requires(true)]
#[ensures(true)]
pub fn build_owned_indexes(entries: &[DictionaryEntry<'_>]) -> OwnedDictionaryIndexes {
    let mut word_map: BTreeMap<String, Vec<EntryIndex>> = BTreeMap::new();
    let mut rafsi_map: BTreeMap<String, Vec<RafsiIndexTarget>> = BTreeMap::new();
    let mut selmaho_map: BTreeMap<String, Vec<EntryIndex>> = BTreeMap::new();
    let mut pattern_index = Vec::with_capacity(entries.len());

    for (index, entry) in entries.iter().enumerate() {
        let entry_index = EntryIndex(index);
        word_map
            .entry(normalize_lookup_query(entry.word))
            .or_default()
            .push(entry_index);

        let mut pattern_rafsi_keys = Vec::new();
        for rafsi in entry.rafsi {
            rafsi_map
                .entry(normalize_lookup_query(rafsi.0))
                .or_default()
                .push(RafsiIndexTarget {
                    entry_index,
                    source: RafsiSource::Listed,
                });
            pattern_rafsi_keys.push(normalize_pattern_lookup_key(rafsi.0));
        }

        if entry.word_type.is_gismu_like() {
            for (rafsi, source) in universal_gismu_rafsi_forms(entry.word) {
                pattern_rafsi_keys.push(normalize_pattern_lookup_key(&rafsi));
                rafsi_map.entry(rafsi).or_default().push(RafsiIndexTarget {
                    entry_index,
                    source,
                });
            }
        }

        if let Some(selmaho) = entry.selmaho {
            selmaho_map
                .entry(selmaho.0.to_owned())
                .or_default()
                .push(entry_index);
        }

        pattern_index.push(OwnedPatternIndexEntry {
            entry_index,
            word_key: normalize_pattern_lookup_key(entry.word),
            rafsi_keys: pattern_rafsi_keys,
        });
    }

    OwnedDictionaryIndexes {
        word_index: word_map
            .into_iter()
            .map(|(key, targets)| OwnedWordIndexEntry { key, targets })
            .collect(),
        rafsi_index: rafsi_map
            .into_iter()
            .map(|(key, targets)| OwnedRafsiIndexEntry { key, targets })
            .collect(),
        selmaho_index: selmaho_map
            .into_iter()
            .map(|(key, targets)| OwnedSelmahoIndexEntry { key, targets })
            .collect(),
        pattern_index,
    }
}

/// Return v0-compatible normalized lookup text.
#[requires(true)]
#[ensures(true)]
pub fn normalize_lookup_query(raw: &str) -> String {
    raw.split_whitespace()
        .map(normalize_lookup_token)
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Return normalized text for dictionary pattern matching.
#[requires(true)]
#[ensures(true)]
pub fn normalize_pattern_lookup_key(raw: &str) -> String {
    normalize_lookup_query(raw)
        .chars()
        .map(|value| if value == 'h' { '\'' } else { value })
        .collect()
}

/// Return v0-compatible universal rafsi forms for a gismu-like word.
#[requires(true)]
#[ensures(true)]
pub fn universal_gismu_rafsi_forms(word: &str) -> Vec<(String, RafsiSource)> {
    let normalized = normalize_lookup_query(word);
    if normalized.len() != 5 {
        return Vec::new();
    }
    let Some(final_char) = normalized.chars().last() else {
        return Vec::new();
    };
    if !matches!(final_char, 'a' | 'e' | 'i' | 'o' | 'u') {
        return Vec::new();
    }

    let short = normalized[..4].to_owned();
    let mut forms = Vec::new();
    if short != "brod" {
        forms.push((short, RafsiSource::UniversalShort));
    }
    forms.push((normalized, RafsiSource::UniversalLong));
    forms
}

#[requires(true)]
#[ensures(true)]
fn normalize_lookup_token(raw: &str) -> String {
    raw.chars()
        .filter_map(strip_diacritic)
        .flat_map(char::to_lowercase)
        .map(normalize_apostrophe)
        .filter(|value| is_lookup_char(*value))
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn normalize_apostrophe(value: char) -> char {
    match value {
        '’' | 'ʼ' => '\'',
        other => other,
    }
}

#[requires(true)]
#[ensures(true)]
fn is_lookup_char(value: char) -> bool {
    value.is_ascii_lowercase() || value.is_ascii_digit() || value == '\'' || value == 'h'
}

#[requires(true)]
#[ensures(true)]
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

#[requires(true)]
#[ensures(true)]
fn validate_entry(
    index: usize,
    entry: &DictionaryEntry<'_>,
) -> Result<(), DictionaryValidationError> {
    if entry.word.is_empty() {
        return Err(DictionaryValidationError::InvalidEntry {
            index,
            reason: "word is empty",
        });
    }
    if entry.definition.is_empty() {
        return Err(DictionaryValidationError::InvalidEntry {
            index,
            reason: "definition is empty",
        });
    }
    if entry.definition_id.0 == 0 {
        return Err(DictionaryValidationError::InvalidEntry {
            index,
            reason: "definition id is zero",
        });
    }
    if !entry.score.0.is_finite() {
        return Err(DictionaryValidationError::InvalidEntry {
            index,
            reason: "score is not finite",
        });
    }
    if entry.user.username.is_empty() {
        return Err(DictionaryValidationError::InvalidEntry {
            index,
            reason: "user username is empty",
        });
    }
    if entry.rafsi.iter().any(|rafsi| rafsi.0.is_empty()) {
        return Err(DictionaryValidationError::InvalidEntry {
            index,
            reason: "rafsi is empty",
        });
    }
    if entry
        .selmaho
        .is_some_and(|selmaho| selmaho.0.trim().is_empty())
    {
        return Err(DictionaryValidationError::InvalidEntry {
            index,
            reason: "selma'o is empty",
        });
    }
    if entry
        .gloss_keywords
        .iter()
        .chain(entry.place_keywords.iter())
        .any(|keyword| keyword.word.is_empty())
    {
        return Err(DictionaryValidationError::InvalidEntry {
            index,
            reason: "keyword word is empty",
        });
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn word_index_matches(actual: &[WordIndexEntry<'_>], expected: &[OwnedWordIndexEntry]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected.iter())
            .all(|(actual, expected)| {
                actual.key == expected.key && actual.targets == expected.targets
            })
}

#[requires(true)]
#[ensures(true)]
fn rafsi_index_matches(actual: &[RafsiIndexEntry<'_>], expected: &[OwnedRafsiIndexEntry]) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected.iter())
            .all(|(actual, expected)| {
                actual.key == expected.key && actual.targets == expected.targets
            })
}

#[requires(true)]
#[ensures(true)]
fn selmaho_index_matches(
    actual: &[SelmahoIndexEntry<'_>],
    expected: &[OwnedSelmahoIndexEntry],
) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected.iter())
            .all(|(actual, expected)| {
                actual.key == expected.key && actual.targets == expected.targets
            })
}

#[requires(true)]
#[ensures(true)]
fn pattern_index_matches(
    actual: &[DictionaryPatternEntry<'_>],
    expected: &[OwnedPatternIndexEntry],
) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected.iter())
            .all(|(actual, expected)| {
                actual.entry_index == expected.entry_index
                    && actual.word_key == expected.word_key
                    && actual.rafsi_keys == expected.rafsi_keys
            })
}

#[requires(true)]
#[ensures(true)]
fn validate_sound_index(
    entries: &[DictionaryEntry<'_>],
    sound_index: &[DictionarySoundEntry<'_>],
) -> Result<(), DictionaryValidationError> {
    let mut previous_index = None;
    for (index, sound_entry) in sound_index.iter().enumerate() {
        if sound_entry.entry_index.0 >= entries.len() {
            return Err(DictionaryValidationError::InvalidSoundIndexEntry {
                index,
                reason: "entry index is out of range",
            });
        }
        if previous_index.is_some_and(|previous| sound_entry.entry_index.0 <= previous) {
            return Err(DictionaryValidationError::InvalidSoundIndexEntry {
                index,
                reason: "entry indexes are not strictly ascending",
            });
        }
        if sound_entry.ipa.trim().is_empty() {
            return Err(DictionaryValidationError::InvalidSoundIndexEntry {
                index,
                reason: "IPA text is empty",
            });
        }
        if sound_entry.token_sequence.segments.is_empty() {
            return Err(DictionaryValidationError::InvalidSoundIndexEntry {
                index,
                reason: "token sequence is empty",
            });
        }
        if !sound_entry.token_sequence.self_similarity.is_finite()
            || sound_entry.token_sequence.self_similarity <= 0.0
        {
            return Err(DictionaryValidationError::InvalidSoundIndexEntry {
                index,
                reason: "token sequence self-similarity is not positive and finite",
            });
        }
        previous_index = Some(sound_entry.entry_index.0);
    }
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn validate_lujvo_index(
    entries: &[DictionaryEntry<'_>],
    lujvo_index: &[DictionaryLujvoEntry<'_>],
) -> Result<(), DictionaryValidationError> {
    let mut previous_index = None;
    for (index, lujvo_entry) in lujvo_index.iter().enumerate() {
        if lujvo_entry.entry_index.0 >= entries.len() {
            return Err(DictionaryValidationError::InvalidLujvoIndexEntry {
                index,
                reason: "entry index is out of range",
            });
        }
        if previous_index.is_some_and(|previous| lujvo_entry.entry_index.0 <= previous) {
            return Err(DictionaryValidationError::InvalidLujvoIndexEntry {
                index,
                reason: "entry indexes are not strictly ascending",
            });
        }
        if !entries[lujvo_entry.entry_index.0].word_type.is_lujvo_like() {
            return Err(DictionaryValidationError::InvalidLujvoIndexEntry {
                index,
                reason: "entry is not lujvo-like",
            });
        }
        if lujvo_entry.segments.is_empty() {
            return Err(DictionaryValidationError::InvalidLujvoIndexEntry {
                index,
                reason: "decomposition has no segments",
            });
        }
        let rafsi_count = lujvo_entry
            .segments
            .iter()
            .filter(|segment| segment.kind == DictionaryLujvoSegmentKind::Rafsi)
            .count();
        if rafsi_count < 2 {
            return Err(DictionaryValidationError::InvalidLujvoIndexEntry {
                index,
                reason: "decomposition has fewer than two rafsi segments",
            });
        }
        if lujvo_entry
            .segments
            .iter()
            .any(|segment| segment.surface.is_empty())
        {
            return Err(DictionaryValidationError::InvalidLujvoIndexEntry {
                index,
                reason: "decomposition segment surface is empty",
            });
        }
        if lujvo_entry.segments.iter().any(|segment| {
            segment.kind == DictionaryLujvoSegmentKind::Hyphen && segment.source_word.is_some()
        }) {
            return Err(DictionaryValidationError::InvalidLujvoIndexEntry {
                index,
                reason: "hyphen segment has a source word",
            });
        }
        if lujvo_entry
            .source_words
            .iter()
            .any(|source| source.is_empty())
        {
            return Err(DictionaryValidationError::InvalidLujvoIndexEntry {
                index,
                reason: "source word is empty",
            });
        }
        if lujvo_entry
            .source_words
            .iter()
            .enumerate()
            .any(|(source_index, source)| lujvo_entry.source_words[..source_index].contains(source))
        {
            return Err(DictionaryValidationError::InvalidLujvoIndexEntry {
                index,
                reason: "source words contain duplicates",
            });
        }
        previous_index = Some(lujvo_entry.entry_index.0);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn normalizes_lookup_query_like_v0() {
        assert_eq!(normalize_lookup_query(" .Án,iis. "), "aniis");
        assert_eq!(normalize_lookup_query("daʼoi"), "da'oi");
        assert_eq!(normalize_lookup_query("da’oi"), "da'oi");
        assert_eq!(normalize_lookup_query("lo  brodá"), "lo broda");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pattern_lookup_key_uses_apostrophe_spelling() {
        assert_eq!(normalize_pattern_lookup_key("daʼoi"), "da'oi");
        assert_eq!(normalize_pattern_lookup_key("dahoi"), "da'oi");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn builds_universal_gismu_rafsi_forms_like_v0() {
        assert_eq!(
            universal_gismu_rafsi_forms("banli"),
            vec![
                ("banl".to_owned(), RafsiSource::UniversalShort),
                ("banli".to_owned(), RafsiSource::UniversalLong)
            ]
        );
        assert_eq!(
            universal_gismu_rafsi_forms("broda"),
            vec![("broda".to_owned(), RafsiSource::UniversalLong)]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lookup_preserves_normalized_word_collisions() {
        let entries = &[
            test_entry("INternet", WordType::Cmevla, &[], None),
            test_entry("internet", WordType::Cmevla, &[], None),
        ];
        let indexes = build_owned_indexes(entries);
        let word_index = leak_word_index(&indexes.word_index);
        let rafsi_index = leak_rafsi_index(&indexes.rafsi_index);
        let selmaho_index = leak_selmaho_index(&indexes.selmaho_index);
        let dictionary = Dictionary::from_static_slices(
            entries,
            word_index,
            rafsi_index,
            selmaho_index,
            leak_pattern_index(&indexes.pattern_index),
            &[],
            &[],
        );

        assert!(dictionary.validate().is_ok());
        assert_eq!(
            dictionary
                .lookup_words("internet")
                .map(|entry| entry.word)
                .collect::<Vec<_>>(),
            vec!["INternet", "internet"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn validate_rejects_empty_selmaho() {
        let entries = &[test_entry(
            "brode",
            WordType::ExperimentalGismu,
            &[],
            Some(RawSelmaho("")),
        )];
        let indexes = build_owned_indexes(entries);
        let word_index = leak_word_index(&indexes.word_index);
        let rafsi_index = leak_rafsi_index(&indexes.rafsi_index);
        let selmaho_index = leak_selmaho_index(&indexes.selmaho_index);
        let dictionary = Dictionary::from_static_slices(
            entries,
            word_index,
            rafsi_index,
            selmaho_index,
            leak_pattern_index(&indexes.pattern_index),
            &[],
            &[],
        );

        assert_eq!(
            dictionary.validate(),
            Err(DictionaryValidationError::InvalidEntry {
                index: 0,
                reason: "selma'o is empty",
            })
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn test_entry(
        word: &'static str,
        word_type: WordType,
        rafsi: &'static [Rafsi<'static>],
        selmaho: Option<RawSelmaho<'static>>,
    ) -> DictionaryEntry<'static> {
        DictionaryEntry {
            word,
            word_type,
            definition: "test definition",
            definition_id: DefinitionId(1),
            notes: "",
            score: Score(1.0),
            gloss_keywords: &[],
            place_keywords: &[],
            rafsi,
            selmaho,
            etymology: None,
            jargon: None,
            user: DictionaryUser {
                username: "test",
                realname: None,
            },
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn leak_word_index(index: &[OwnedWordIndexEntry]) -> &'static [WordIndexEntry<'static>] {
        index
            .iter()
            .map(|entry| WordIndexEntry {
                key: Box::leak(entry.key.clone().into_boxed_str()),
                targets: Box::leak(entry.targets.clone().into_boxed_slice()),
            })
            .collect::<Vec<_>>()
            .leak()
    }

    #[requires(true)]
    #[ensures(true)]
    fn leak_rafsi_index(index: &[OwnedRafsiIndexEntry]) -> &'static [RafsiIndexEntry<'static>] {
        index
            .iter()
            .map(|entry| RafsiIndexEntry {
                key: Box::leak(entry.key.clone().into_boxed_str()),
                targets: Box::leak(entry.targets.clone().into_boxed_slice()),
            })
            .collect::<Vec<_>>()
            .leak()
    }

    #[requires(true)]
    #[ensures(true)]
    fn leak_selmaho_index(
        index: &[OwnedSelmahoIndexEntry],
    ) -> &'static [SelmahoIndexEntry<'static>] {
        index
            .iter()
            .map(|entry| SelmahoIndexEntry {
                key: Box::leak(entry.key.clone().into_boxed_str()),
                targets: Box::leak(entry.targets.clone().into_boxed_slice()),
            })
            .collect::<Vec<_>>()
            .leak()
    }

    #[requires(true)]
    #[ensures(true)]
    fn leak_pattern_index(
        index: &[OwnedPatternIndexEntry],
    ) -> &'static [DictionaryPatternEntry<'static>] {
        index
            .iter()
            .map(|entry| DictionaryPatternEntry {
                entry_index: entry.entry_index,
                word_key: Box::leak(entry.word_key.clone().into_boxed_str()),
                rafsi_keys: entry
                    .rafsi_keys
                    .iter()
                    .map(|key| &*Box::leak(key.clone().into_boxed_str()))
                    .collect::<Vec<_>>()
                    .leak(),
            })
            .collect::<Vec<_>>()
            .leak()
    }
}
