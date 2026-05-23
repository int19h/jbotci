//! Borrowed dictionary model and lookup support.

#[cfg(feature = "import")]
pub mod import;

use std::collections::BTreeMap;

use bityzba::{expensive_invariant, invariant, requires};
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
    ) -> Self {
        Self {
            entries,
            word_index,
            rafsi_index,
            selmaho_index,
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

/// Lensisku dictionary word type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
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

/// Rafsi target plus provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[invariant(true)]
pub struct RafsiIndexTarget {
    pub entry_index: EntryIndex,
    pub source: RafsiSource,
}

/// Source of a rafsi lookup match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[invariant(true)]
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

/// Validation error for generated dictionary tables.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::InvalidEntry => true)]
pub enum DictionaryValidationError {
    #[error("invalid dictionary entry at index {index}: {reason}")]
    InvalidEntry { index: usize, reason: &'static str },
    #[error("word index does not match dictionary entries")]
    WordIndexMismatch,
    #[error("rafsi index does not match dictionary entries")]
    RafsiIndexMismatch,
    #[error("selma'o index does not match dictionary entries")]
    SelmahoIndexMismatch,
}

/// Build owned indexes for a borrowed entry table.
#[requires(true)]
#[ensures(true)]
pub fn build_owned_indexes(entries: &[DictionaryEntry<'_>]) -> OwnedDictionaryIndexes {
    let mut word_map: BTreeMap<String, Vec<EntryIndex>> = BTreeMap::new();
    let mut rafsi_map: BTreeMap<String, Vec<RafsiIndexTarget>> = BTreeMap::new();
    let mut selmaho_map: BTreeMap<String, Vec<EntryIndex>> = BTreeMap::new();

    for (index, entry) in entries.iter().enumerate() {
        let entry_index = EntryIndex(index);
        word_map
            .entry(normalize_lookup_query(entry.word))
            .or_default()
            .push(entry_index);

        for rafsi in entry.rafsi {
            rafsi_map
                .entry(normalize_lookup_query(rafsi.0))
                .or_default()
                .push(RafsiIndexTarget {
                    entry_index,
                    source: RafsiSource::Listed,
                });
        }

        if entry.word_type.is_gismu_like() {
            for (rafsi, source) in universal_gismu_rafsi_forms(entry.word) {
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
        let dictionary =
            Dictionary::from_static_slices(entries, word_index, rafsi_index, selmaho_index);

        assert!(dictionary.validate().is_ok());
        assert_eq!(
            dictionary
                .lookup_words("internet")
                .map(|entry| entry.word)
                .collect::<Vec<_>>(),
            vec!["INternet", "internet"]
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
}
