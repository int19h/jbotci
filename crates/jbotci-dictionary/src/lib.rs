//! Borrowed dictionary model and lookup support.

#[cfg(feature = "import")]
pub mod import;

pub mod places;

mod compounds;
pub use compounds::{
    CmavoSequenceIndexEntry, OwnedCmavoSequenceIndexEntry, cmavo_sequence_key,
    is_compound_separator,
};

use std::collections::BTreeMap;

use bityzba::{data, expensive_invariant, invariant, new, requires};
use jbotci_morphology::{ShortRafsiShape, fold_lojban_diacritic, possible_short_rafsi_forms};
use jbotci_phonetic::{IpaTokenSequenceView, PronunciationTargetSequenceView};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use vec1::Vec1;

/// Complete borrowed dictionary plus lookup indexes.
#[derive(Debug, Clone, Copy)]
#[invariant(
    true,
    "dictionary-wide validity is established by generation-time and explicit validation"
)]
pub struct Dictionary<'a> {
    entries: &'a [DictionaryEntry<'a>],
    word_index: &'a [WordIndexEntry<'a>],
    rafsi_index: &'a [RafsiIndexEntry<'a>],
    selmaho_index: &'a [SelmahoIndexEntry<'a>],
    pattern_index: &'a [DictionaryPatternEntry<'a>],
    sound_index: &'a [DictionarySoundEntry<'a>],
    lujvo_index: &'a [DictionaryLujvoEntry<'a>],
    cmavo_sequence_index: &'a [CmavoSequenceIndexEntry<'a>],
    max_cmavo_sequence_len: usize,
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
        cmavo_sequence_index: &'a [CmavoSequenceIndexEntry<'a>],
        max_cmavo_sequence_len: usize,
    ) -> Self {
        Self {
            entries,
            word_index,
            rafsi_index,
            selmaho_index,
            pattern_index,
            sound_index,
            lujvo_index,
            cmavo_sequence_index,
            max_cmavo_sequence_len,
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
        if self.max_cmavo_sequence_len != expected.max_cmavo_sequence_len
            || self.cmavo_sequence_index.len() != expected.cmavo_sequence_index.len()
            || !self
                .cmavo_sequence_index
                .iter()
                .zip(&expected.cmavo_sequence_index)
                .all(|(actual, expected)| {
                    actual
                        .components()
                        .iter()
                        .copied()
                        .eq(expected.components.iter().map(String::as_str))
                        && actual.targets() == expected.targets
                })
        {
            return Err(DictionaryValidationError::CmavoSequenceIndexMismatch);
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

// Deliberately no impl invariant: bityzba copies impl invariants onto every method taking `self`.
// `validate()` rebuilds the corpus-wide lookup indexes in O(n), so checking it per method makes
// full-corpus suites that perform O(n) dictionary calls take O(n²) time (#635). Coverage instead
// lives in generation-time validation in jbotci-dictionary-data's build.rs, its
// `embedded_dictionary_validates()` unit test, and explicit `validate()` calls in this crate's
// tests. Any future runtime `Dictionary` constructor must carry
// `#[expensive_ensures(ret.validate().is_ok())]` at the construction boundary. The existing
// `from_static_slices` constructor is const and used in a static initializer, so it cannot carry
// runtime contracts.
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

    /// Exact attestation in source order, with no synthetic or decomposition fallback.
    #[requires(true)]
    #[ensures(true)]
    pub fn exact_definition_indices(&self, query: &str) -> impl Iterator<Item = EntryIndex> + '_ {
        let normalized = normalize_lookup_query(query);
        let targets = self
            .word_index_entry(&normalized)
            .map_or(&[][..], |entry| entry.targets);
        targets
            .iter()
            .copied()
            .filter(|index| self.entry_at(*index).has_definition())
    }

    /// Structured keys, ordered lexicographically by canonical morphology components.
    #[requires(true)]
    #[ensures(true)]
    pub fn cmavo_sequence_index(&self) -> &'a [CmavoSequenceIndexEntry<'a>] {
        self.cmavo_sequence_index
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn max_cmavo_sequence_len(&self) -> usize {
        self.max_cmavo_sequence_len
    }

    /// Find real entries for a canonical component sequence without joining spellings.
    #[requires(true)]
    #[ensures(true)]
    pub fn lookup_cmavo_sequence<S: AsRef<str>>(&self, components: &[S]) -> &'a [EntryIndex] {
        self.cmavo_sequence_index
            .binary_search_by(|entry| {
                entry
                    .components()
                    .iter()
                    .copied()
                    .cmp(components.iter().map(AsRef::as_ref))
            })
            .ok()
            .map_or(&[], |index| self.cmavo_sequence_index[index].targets())
    }

    /// Return the first non-empty gloss keyword for each lookup word.
    ///
    /// This is the batch counterpart to repeatedly calling [`Self::lookup_words`].
    /// Dictionary-wide validity is established at generation and explicit
    /// validation boundaries rather than by individual lookup operations.
    #[requires(true)]
    #[ensures(ret.len() == words.len())]
    pub fn first_gloss_keywords_for_words(&self, words: &[&str]) -> Vec<Option<&'a str>> {
        words
            .iter()
            .map(|word| {
                let normalized = normalize_lookup_query(word);
                let targets = self
                    .word_index
                    .binary_search_by(|entry| entry.key.cmp(normalized.as_str()))
                    .ok()
                    .map_or(&[][..], |position| self.word_index[position].targets);
                targets
                    .iter()
                    .map(|index| self.entry_at(*index))
                    .flat_map(|entry| entry.gloss_keywords)
                    .map(|keyword| keyword.word)
                    .find(|gloss| !gloss.is_empty())
            })
            .collect()
    }

    /// Return dictionary entries whose normalized word starts with `prefix`.
    ///
    /// The returned iterator is backed by the contiguous matching range in the
    /// sorted word index. It stays lazy over both index keys and collision
    /// targets, so callers do not have to scan or materialize the dictionary.
    #[requires(true)]
    #[ensures(true)]
    pub fn entries_by_word_prefix<'lookup>(
        &'lookup self,
        prefix: &str,
    ) -> impl Iterator<Item = &'lookup DictionaryEntry<'a>> + 'lookup {
        let normalized = normalize_lookup_query(prefix);
        let start = self
            .word_index
            .partition_point(|entry| entry.key < normalized.as_str());
        let end = self.word_index.partition_point(|entry| {
            entry.key < normalized.as_str() || entry.key.starts_with(&normalized)
        });
        self.word_index[start..end]
            .iter()
            .flat_map(|entry| entry.targets.iter().map(|index| self.entry_at(*index)))
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

    /// Return the word and type of every dictionary entry claiming `rafsi`.
    ///
    /// Borrowed counterpart of [`Dictionary::short_rafsi_candidates`]: it hands
    /// back the entry text itself rather than owned copies. Unlike short-rafsi
    /// availability this keeps every [`RafsiSource`], so a four- or five-letter
    /// query also reports the gismu whose universal forms spell it.
    #[requires(true)]
    #[ensures(true)]
    pub fn rafsi_claimants<'lookup>(
        &'lookup self,
        rafsi: &str,
    ) -> impl Iterator<Item = (&'a str, WordType)> + 'lookup {
        self.lookup_rafsi(rafsi)
            .map(|matched| (matched.entry.word, matched.entry.word_type))
    }

    /// Return every short rafsi `gismu` could claim, with its availability.
    ///
    /// The derivation itself is pure phonotactics
    /// ([`possible_short_rafsi_forms`]); this method adds what only the
    /// dictionary knows, namely which words already hold each form.
    #[requires(true)]
    #[ensures(
        ret.windows(2).all(|pair| pair[0].form < pair[1].form),
        "candidates inherit the sorted, duplicate-free derivation order"
    )]
    pub fn short_rafsi_candidates(&self, gismu: &str) -> Vec<RafsiCandidate> {
        possible_short_rafsi_forms(gismu)
            .into_iter()
            .map(|form| {
                let availability = self.rafsi_availability(&form.form);
                let form = form.into_data();
                new!(RafsiCandidate {
                    form: form.form,
                    shape: form.shape,
                    availability: availability,
                })
            })
            .collect()
    }

    /// Classify who, if anyone, has already claimed `rafsi`.
    ///
    /// Every entry that lists the form is a claimant, whatever its word type:
    /// rafsi are assigned to cmavo as well as gismu (CLL 4.6; `kam` belongs to
    /// the cmavo `ka`), and a form that some word already spells cannot be
    /// handed to a new gismu no matter which word holds it. The claimant's word
    /// type only decides the standing of the claim
    /// ([`WordType::rafsi_claim_kind`]).
    ///
    /// Only [`RafsiSource::Listed`] matches count. The other sources are the
    /// universal forms synthesized for every gismu-like entry, and for
    /// well-formed gismu those cannot spell a short rafsi: a short rafsi is
    /// three letters, or four with an apostrophe third (CVV, `sa'i`), whereas
    /// the universal forms of a CVCCV or CCVCV gismu are its apostrophe-free
    /// four- and five-letter prefixes. Nothing forces a gismu-typed entry to
    /// be spelled that way, though — [`Dictionary::validate`] does not check
    /// word spelling against word type — so the filter is a real guard rather
    /// than a formality.
    #[requires(!rafsi.is_empty())]
    #[ensures(
        ret.is_free()
            != self
                .lookup_rafsi(rafsi)
                .any(|matched| matched.source == RafsiSource::Listed),
        "any listed claimant takes the form, whatever its word type"
    )]
    #[ensures(
        (ret.claim_kind() == Some(RafsiClaimKind::Official))
            == self.lookup_rafsi(rafsi).any(|matched| {
                matched.source == RafsiSource::Listed
                    && matched.entry.word_type.rafsi_claim_kind() == RafsiClaimKind::Official
            }),
        "an official claim outranks any experimental one"
    )]
    #[ensures(
        ret.claimant_words().iter().map(String::as_str).eq(
            self.lookup_rafsi(rafsi)
                .filter(|matched| {
                    matched.source == RafsiSource::Listed
                        && Some(matched.entry.word_type.rafsi_claim_kind()) == ret.claim_kind()
                })
                .map(|matched| matched.entry.word)
        ),
        "the reported words are the claimants of the winning standing, in index order"
    )]
    fn rafsi_availability(&self, rafsi: &str) -> RafsiAvailability {
        let mut official = Vec::new();
        let mut experimental = Vec::new();
        for matched in self
            .lookup_rafsi(rafsi)
            .filter(|matched| matched.source == RafsiSource::Listed)
        {
            let word = matched.entry.word.to_owned();
            match matched.entry.word_type.rafsi_claim_kind() {
                RafsiClaimKind::Official => official.push(word),
                RafsiClaimKind::Experimental => experimental.push(word),
            }
        }
        if let Ok(words) = Vec1::try_from_vec(official) {
            return new!(RafsiAvailability::Taken {
                kind: RafsiClaimKind::Official,
                words: words,
            });
        }
        match Vec1::try_from_vec(experimental) {
            Ok(words) => new!(RafsiAvailability::Taken {
                kind: RafsiClaimKind::Experimental,
                words: words,
            }),
            Err(_) => new!(RafsiAvailability::Free),
        }
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
    /// Concrete tokens for the canonical display IPA, retained for API
    /// compatibility and diagnostics rather than scoring.
    pub token_sequence: IpaTokenSequenceView<'a>,
    /// Lojban pronunciation targets used by sound scoring.
    pub pronunciation_targets: PronunciationTargetSequenceView<'a>,
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
    /// Lensisku's `typeid = 0`, its "not a word" catch-all.
    ///
    /// In practice it marks an entry whose submitter never chose a type rather
    /// than a considered judgement that the text is unpronounceable, so the
    /// snapshot keeps such entries and treats their claims as provisional.
    #[serde(rename = "nalvla")]
    Nalvla,
}

impl WordType {
    /// Return whether this type is a gismu class for rafsi purposes.
    #[requires(true)]
    #[ensures(true)]
    pub const fn is_gismu_like(self) -> bool {
        matches!(self, Self::Gismu | Self::ExperimentalGismu)
    }

    /// Return the standing of a rafsi claim made by a word of this type.
    ///
    /// The match is deliberately exhaustive rather than defaulting: a new word
    /// type must be classified consciously, because an unclassified claimant
    /// would silently hand an already-spelled rafsi to a new gismu.
    ///
    /// The rule is nearly uniform across the taxonomy — a claim is
    /// [`Experimental`] exactly when the type itself is experimental or
    /// obsolete, and [`Official`] otherwise — which the postcondition
    /// cross-checks against the Lensisku type names. [`Nalvla`] is the one
    /// exception: it is not named for a register, but an untyped entry cannot
    /// bind the standard register either.
    ///
    /// [`Experimental`]: RafsiClaimKind::Experimental
    /// [`Official`]: RafsiClaimKind::Official
    /// [`Nalvla`]: WordType::Nalvla
    #[requires(true)]
    #[ensures(
        (ret == RafsiClaimKind::Experimental)
            == (self == Self::Nalvla
                || self.as_str().starts_with("experimental ")
                || self.as_str().starts_with("obsolete ")),
        "only experimental, obsolete, and untyped word types make merely experimental claims"
    )]
    pub fn rafsi_claim_kind(self) -> RafsiClaimKind {
        match self {
            // Standard brivla and cmavo: the dictionary records their rafsi as
            // the official assignment for the form.
            Self::Gismu
            | Self::Lujvo
            | Self::ZeiLujvo
            | Self::Cmavo
            | Self::CmavoCompound
            | Self::Fuivla
            | Self::Cmevla
            // Letterals and phrases are not rafsi-bearing in practice, but a
            // listed rafsi on one is still a standard-register assignment.
            | Self::BuLetteral
            | Self::Phrase => RafsiClaimKind::Official,
            // Experimental words are proposals, so their rafsi claims are
            // provisional and yield to any official claim on the same form.
            Self::ExperimentalGismu | Self::ExperimentalCmavo => RafsiClaimKind::Experimental,
            // Obsolete words were withdrawn from the standard register, so
            // their claims are likewise provisional rather than binding — the
            // form stays flagged, but an official claimant outranks them.
            Self::ObsoleteZeiLujvo
            | Self::ObsoleteCmavo
            | Self::ObsoleteFuivla
            | Self::ObsoleteCmevla => RafsiClaimKind::Experimental,
            // An entry Lensisku never classified has no standing to hold a
            // form against a word that was classified.
            Self::Nalvla => RafsiClaimKind::Experimental,
        }
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
            Self::Nalvla => "nalvla",
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

/// Standing of the words that already claim a short rafsi.
///
/// Official words outrank experimental and obsolete ones
/// ([`WordType::rafsi_claim_kind`]): a rafsi held by an official word is
/// reported as officially taken even when experimental words claim it too.
#[invariant(::Official => true)]
#[invariant(::Experimental => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RafsiClaimKind {
    Official,
    Experimental,
}

/// Whether a short rafsi is still available, and to whom it belongs otherwise.
///
/// The claimant list lives inside `Taken` so that a free rafsi cannot carry
/// claimants and a taken one cannot lack them.
#[invariant(::Free => true)]
#[invariant(::Taken { words, .. } => words.iter().all(|word| !word.is_empty()))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RafsiAvailability {
    Free,
    Taken {
        kind: RafsiClaimKind,
        words: Vec1<String>,
    },
}

impl RafsiAvailability {
    /// Whether no dictionary word has claimed this rafsi yet.
    #[requires(true)]
    #[ensures(ret == matches!(self.as_data(), data!(RafsiAvailability::Free)))]
    pub fn is_free(&self) -> bool {
        matches!(self.as_data(), data!(RafsiAvailability::Free))
    }

    /// Return the standing of the claim, or `None` while the rafsi is free.
    #[requires(true)]
    #[ensures(ret.is_none() == self.is_free())]
    pub fn claim_kind(&self) -> Option<RafsiClaimKind> {
        match self.as_data() {
            data!(RafsiAvailability::Free) => None,
            data!(RafsiAvailability::Taken { kind, .. }) => Some(*kind),
        }
    }

    /// Return the words that claim this rafsi, empty when it is free.
    #[requires(true)]
    #[ensures(self.is_free() == ret.is_empty())]
    pub fn claimant_words(&self) -> &[String] {
        match self.as_data() {
            data!(RafsiAvailability::Free) => &[],
            data!(RafsiAvailability::Taken { words, .. }) => words,
        }
    }
}

/// One short rafsi a gismu could claim, with its dictionary standing.
#[invariant(shape.matches_form(form), "the spelling realizes the declared shape")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RafsiCandidate {
    pub form: String,
    pub shape: ShortRafsiShape,
    pub availability: RafsiAvailability,
}

impl DictionaryEntry<'_> {
    /// Shared attestation predicate for exact entries and generated sequence targets.
    #[requires(true)]
    #[ensures(ret == !self.definition.is_empty())]
    pub fn has_definition(&self) -> bool {
        !self.definition.is_empty()
    }
}

/// Owned indexes used by importers and validation.
#[expensive_invariant(*max_cmavo_sequence_len == cmavo_sequence_index.iter().map(|entry| entry.components.len()).max().unwrap_or(0))]
#[expensive_invariant(cmavo_sequence_index.windows(2).all(|pair| pair[0].components < pair[1].components))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedDictionaryIndexes {
    pub word_index: Vec<OwnedWordIndexEntry>,
    pub rafsi_index: Vec<OwnedRafsiIndexEntry>,
    pub selmaho_index: Vec<OwnedSelmahoIndexEntry>,
    pub pattern_index: Vec<OwnedPatternIndexEntry>,
    pub cmavo_sequence_index: Vec<OwnedCmavoSequenceIndexEntry>,
    pub max_cmavo_sequence_len: usize,
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
    #[error("cmavo sequence index does not match dictionary entries")]
    CmavoSequenceIndexMismatch,
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
    let mut cmavo_map: BTreeMap<Vec<String>, Vec<EntryIndex>> = BTreeMap::new();

    for (index, entry) in entries.iter().enumerate() {
        let entry_index = EntryIndex(index);
        if entry.has_definition()
            && let Some(components) = cmavo_sequence_key(entry.word)
        {
            cmavo_map.entry(components).or_default().push(entry_index);
        }
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

    let max_cmavo_sequence_len = cmavo_map.keys().map(Vec::len).max().unwrap_or(0);
    new!(OwnedDictionaryIndexes {
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
        cmavo_sequence_index: cmavo_map
            .into_iter()
            .map(|(components, targets)| {
                new!(OwnedCmavoSequenceIndexEntry {
                    components,
                    targets
                })
            })
            .collect(),
        max_cmavo_sequence_len,
    })
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
        .filter_map(fold_lojban_diacritic)
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
        if sound_entry.pronunciation_targets.targets.is_empty() {
            return Err(DictionaryValidationError::InvalidSoundIndexEntry {
                index,
                reason: "pronunciation target sequence is empty",
            });
        }
        if !sound_entry
            .pronunciation_targets
            .self_similarity
            .is_finite()
            || sound_entry.pronunciation_targets.self_similarity <= 0.0
        {
            return Err(DictionaryValidationError::InvalidSoundIndexEntry {
                index,
                reason: "pronunciation target self-similarity is not positive and finite",
            });
        }
        if sound_entry.pronunciation_targets.target_count()
            != sound_entry.token_sequence.segment_count()
        {
            return Err(DictionaryValidationError::InvalidSoundIndexEntry {
                index,
                reason: "display tokens and pronunciation targets have different lengths",
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
    fn short_rafsi_candidates_report_claims_from_every_word_type() {
        static SAL: [Rafsi<'static>; 1] = [Rafsi("sal")];
        static SAK: [Rafsi<'static>; 1] = [Rafsi("sak")];
        static SKA: [Rafsi<'static>; 1] = [Rafsi("ska")];
        static KLI: [Rafsi<'static>; 1] = [Rafsi("kli")];
        static SAI: [Rafsi<'static>; 1] = [Rafsi("sai")];
        static SAhI: [Rafsi<'static>; 1] = [Rafsi("sa'i")];
        static KAM: [Rafsi<'static>; 1] = [Rafsi("kam")];
        // Synthetic assignments, but each mirrors a real dictionary shape: the
        // cmavo `ka` really does hold `kam` (CLL 4.6), and fu'ivla, obsolete
        // and experimental entries all appear in the rafsi index.
        let entries = &[
            test_entry("salci", WordType::Gismu, &SAL, None),
            test_entry("salpo", WordType::Gismu, &SAL, None),
            test_entry("sakta", WordType::ExperimentalGismu, &SAK, None),
            test_entry("skami", WordType::Gismu, &SKA, None),
            test_entry("skeci", WordType::ExperimentalGismu, &SKA, None),
            test_entry("kliniko", WordType::Fuivla, &KLI, None),
            test_entry("sa'e", WordType::Cmavo, &SAI, None),
            test_entry("xua'ai", WordType::ObsoleteCmavo, &SAhI, None),
            test_entry("ka", WordType::Cmavo, &KAM, None),
        ];
        let indexes = build_owned_indexes(entries);
        let dictionary = Dictionary::from_static_slices(
            entries,
            leak_word_index(&indexes.word_index),
            leak_rafsi_index(&indexes.rafsi_index),
            leak_selmaho_index(&indexes.selmaho_index),
            leak_pattern_index(&indexes.pattern_index),
            &[],
            &[],
            &[],
            0,
        );
        assert!(dictionary.validate().is_ok());

        let candidates = dictionary.short_rafsi_candidates("sakli");
        let described = candidates
            .iter()
            .map(|candidate| {
                (
                    candidate.form.as_str(),
                    candidate.shape,
                    candidate.availability.claim_kind(),
                    candidate
                        .availability
                        .claimant_words()
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            described,
            vec![
                // A fu'ivla holding `kli` spells the form just as a gismu
                // would, so it is an ordinary official claim.
                (
                    "kli",
                    ShortRafsiShape::Ccv,
                    Some(RafsiClaimKind::Official),
                    vec!["kliniko"],
                ),
                // An obsolete cmavo has left the standard register, so its
                // claim only ever counts as experimental.
                (
                    "sa'i",
                    ShortRafsiShape::Cvv,
                    Some(RafsiClaimKind::Experimental),
                    vec!["xua'ai"],
                ),
                // Cmavo carry rafsi too (CLL 4.6).
                (
                    "sai",
                    ShortRafsiShape::Cvv,
                    Some(RafsiClaimKind::Official),
                    vec!["sa'e"],
                ),
                (
                    "sak",
                    ShortRafsiShape::Cvc,
                    Some(RafsiClaimKind::Experimental),
                    vec!["sakta"],
                ),
                (
                    "sal",
                    ShortRafsiShape::Cvc,
                    Some(RafsiClaimKind::Official),
                    vec!["salci", "salpo"],
                ),
                // `skeci` also claims `ska`, but an official claim outranks it.
                (
                    "ska",
                    ShortRafsiShape::Ccv,
                    Some(RafsiClaimKind::Official),
                    vec!["skami"],
                ),
            ]
        );

        // The reported bug in miniature: only the cmavo `ka` holds any rafsi a
        // hypothetical `kacma` could claim, and it makes `kam` unavailable.
        let kacma = dictionary
            .short_rafsi_candidates("kacma")
            .into_iter()
            .map(|candidate| {
                let candidate = candidate.into_data();
                (candidate.form, candidate.availability)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            kacma,
            vec![
                ("cma".to_owned(), new!(RafsiAvailability::Free)),
                ("ka'a".to_owned(), new!(RafsiAvailability::Free)),
                ("kac".to_owned(), new!(RafsiAvailability::Free)),
                (
                    "kam".to_owned(),
                    new!(RafsiAvailability::Taken {
                        kind: RafsiClaimKind::Official,
                        words: vec1::vec1!["ka".to_owned()],
                    }),
                ),
            ]
        );

        assert_eq!(
            dictionary.rafsi_claimants("ska").collect::<Vec<_>>(),
            vec![
                ("skami", WordType::Gismu),
                ("skeci", WordType::ExperimentalGismu)
            ]
        );
        assert_eq!(
            dictionary.rafsi_claimants("kli").collect::<Vec<_>>(),
            vec![("kliniko", WordType::Fuivla)]
        );
        // Universal forms share the rafsi index but cannot spell a short
        // rafsi, so availability never has to filter them out on content.
        assert_eq!(
            dictionary.rafsi_claimants("salc").collect::<Vec<_>>(),
            vec![("salci", WordType::Gismu)]
        );
        assert!(
            possible_short_rafsi_forms("sakli")
                .iter()
                .all(|form| form.form != "salc")
        );
        assert!(dictionary.short_rafsi_candidates("coi").is_empty());

        // The candidate invariant validates the whole spelling, so neither
        // trailing text nor an illegal apostrophe-free CVV can be constructed.
        for (form, shape) in [
            ("sal!", ShortRafsiShape::Cvc),
            ("sae", ShortRafsiShape::Cvv),
            ("nra", ShortRafsiShape::Ccv),
        ] {
            assert!(
                bityzba::try_new!(RafsiCandidate {
                    form: form.to_owned(),
                    shape: shape,
                    availability: new!(RafsiAvailability::Free),
                })
                .is_err(),
                "{form} is not a {shape:?} rafsi"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rafsi_claim_kind_covers_every_word_type() {
        assert_eq!(
            [
                WordType::Gismu,
                WordType::ExperimentalGismu,
                WordType::Lujvo,
                WordType::ZeiLujvo,
                WordType::ObsoleteZeiLujvo,
                WordType::Cmavo,
                WordType::ExperimentalCmavo,
                WordType::ObsoleteCmavo,
                WordType::CmavoCompound,
                WordType::Fuivla,
                WordType::ObsoleteFuivla,
                WordType::Cmevla,
                WordType::ObsoleteCmevla,
                WordType::BuLetteral,
                WordType::Phrase,
                WordType::Nalvla,
            ]
            .map(WordType::rafsi_claim_kind),
            [
                RafsiClaimKind::Official,
                RafsiClaimKind::Experimental,
                RafsiClaimKind::Official,
                RafsiClaimKind::Official,
                RafsiClaimKind::Experimental,
                RafsiClaimKind::Official,
                RafsiClaimKind::Experimental,
                RafsiClaimKind::Experimental,
                RafsiClaimKind::Official,
                RafsiClaimKind::Official,
                RafsiClaimKind::Experimental,
                RafsiClaimKind::Official,
                RafsiClaimKind::Experimental,
                RafsiClaimKind::Official,
                RafsiClaimKind::Official,
                RafsiClaimKind::Experimental,
            ]
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
            &[],
            0,
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
    fn batch_gloss_lookup_preserves_query_order_and_collision_order() {
        static LARGE_GLOSS: [Keyword<'static>; 1] = [Keyword {
            word: "large",
            meaning: None,
        }];
        let mut second = test_entry("barda", WordType::Gismu, &[], None);
        second.gloss_keywords = &LARGE_GLOSS;
        let entries = &[test_entry("BÁRDA", WordType::Gismu, &[], None), second];
        let indexes = build_owned_indexes(entries);
        let dictionary = Dictionary::from_static_slices(
            entries,
            leak_word_index(&indexes.word_index),
            leak_rafsi_index(&indexes.rafsi_index),
            leak_selmaho_index(&indexes.selmaho_index),
            leak_pattern_index(&indexes.pattern_index),
            &[],
            &[],
            &[],
            0,
        );

        assert!(dictionary.validate().is_ok());
        assert_eq!(
            dictionary.first_gloss_keywords_for_words(&[".bárda.", "missing"]),
            vec![Some("large"), None],
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn prefix_lookup_uses_normalized_sorted_range_and_preserves_collisions() {
        let entries = &[
            test_entry("barda", WordType::Gismu, &[], None),
            test_entry("BÁJRA", WordType::Gismu, &[], None),
            test_entry("bajra", WordType::Gismu, &[], None),
            test_entry("cadzu", WordType::Gismu, &[], None),
        ];
        let indexes = build_owned_indexes(entries);
        let dictionary = Dictionary::from_static_slices(
            entries,
            leak_word_index(&indexes.word_index),
            leak_rafsi_index(&indexes.rafsi_index),
            leak_selmaho_index(&indexes.selmaho_index),
            leak_pattern_index(&indexes.pattern_index),
            &[],
            &[],
            &[],
            0,
        );

        assert!(dictionary.validate().is_ok());
        assert_eq!(
            dictionary
                .entries_by_word_prefix("BÁ")
                .map(|entry| entry.word)
                .collect::<Vec<_>>(),
            vec!["BÁJRA", "bajra", "barda"],
        );
        assert!(
            dictionary.entries_by_word_prefix("bas").next().is_none(),
            "the partitioned range must stop before adjacent non-matches",
        );
        assert_eq!(
            dictionary.entries_by_word_prefix("").count(),
            entries.len(),
            "an empty prefix covers the complete index without a separate scan",
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
            &[],
            0,
        );

        assert_eq!(
            dictionary.validate(),
            Err(DictionaryValidationError::InvalidEntry {
                index: 0,
                reason: "selma'o is empty",
            })
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sequence_targets_ignore_tags_preserve_collisions_and_require_definitions() {
        let mut missing = test_entry("pu ba", WordType::Phrase, &[], None);
        missing.definition = "";
        let entries = [
            test_entry("puba", WordType::CmavoCompound, &[], None),
            test_entry("pu ba", WordType::Nalvla, &[], None),
        ];
        let with_missing = [entries[0], entries[1], missing];
        let incomplete_indexes = build_owned_indexes(&with_missing);
        assert_eq!(
            incomplete_indexes.cmavo_sequence_index[0].targets,
            [EntryIndex(0), EntryIndex(1)]
        );
        let incomplete = Dictionary::from_static_slices(
            &with_missing,
            leak_word_index(&incomplete_indexes.word_index),
            &[],
            &[],
            &[],
            &[],
            &[],
            &[],
            0,
        );
        assert_eq!(
            incomplete
                .exact_definition_indices("pu ba")
                .collect::<Vec<_>>(),
            [EntryIndex(1)]
        );
        let indexes = build_owned_indexes(&entries);
        assert_eq!(indexes.cmavo_sequence_index.len(), 1);
        assert_eq!(indexes.max_cmavo_sequence_len, 2);
        assert_eq!(indexes.cmavo_sequence_index[0].components, ["pu", "ba"]);
        assert_eq!(
            indexes.cmavo_sequence_index[0].targets,
            [EntryIndex(0), EntryIndex(1)]
        );
        let rows = [CmavoSequenceIndexEntry::from_static_parts(
            &["pu", "ba"],
            &[EntryIndex(0), EntryIndex(1)],
        )];
        let dictionary = Dictionary::from_static_slices(
            &entries,
            leak_word_index(&indexes.word_index),
            leak_rafsi_index(&indexes.rafsi_index),
            leak_selmaho_index(&indexes.selmaho_index),
            leak_pattern_index(&indexes.pattern_index),
            &[],
            &[],
            &rows,
            2,
        );
        assert_eq!(dictionary.validate(), Ok(()));
        assert_eq!(
            dictionary.lookup_cmavo_sequence(&["pu", "ba"]),
            rows[0].targets()
        );
        assert_eq!(
            dictionary
                .exact_definition_indices("pu ba")
                .collect::<Vec<_>>(),
            [EntryIndex(1)]
        );
        assert!(
            dictionary
                .exact_definition_indices("klama zei tavla")
                .next()
                .is_none()
        );
        let bad_maximum = Dictionary {
            max_cmavo_sequence_len: 3,
            ..dictionary
        };
        assert_eq!(
            bad_maximum.validate(),
            Err(DictionaryValidationError::CmavoSequenceIndexMismatch)
        );
        let missing_rows = Dictionary {
            cmavo_sequence_index: &[],
            ..dictionary
        };
        assert_eq!(
            missing_rows.validate(),
            Err(DictionaryValidationError::CmavoSequenceIndexMismatch)
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
