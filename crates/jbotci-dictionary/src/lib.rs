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
use jbotci_morphology::{
    GLIDE_PARTNER_MARK, LookupStep, LookupStepData, MorphologyOptions, MorphologyWarningKind,
    ShortRafsiShape, awaits_next_unit, canonicalize_text, ends_with_vowel, fold_lojban_diacritic,
    is_fuhivla_rafsi_form, normalize_lojban_input_text, possible_short_rafsi_forms,
    segment_words_with_modifiers_with_options_and_source_id,
    segment_words_with_modifiers_with_options_and_source_id_attempt, strip_lojban_diacritic,
    transcribe_lookup_run,
};
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
    #[ensures(ret.is_some() == (index.0 < self.entries.len()))]
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
    #[ensures(ret.is_none_or(|index| self.entries.get(index.0).is_some_and(|candidate| std::ptr::eq(candidate, entry))))]
    pub fn entry_index_for_entry(&self, entry: &DictionaryEntry<'a>) -> Option<EntryIndex> {
        // Use the address to find a candidate index, then confirm that it is this entry.
        let bytes = std::ptr::from_ref(entry)
            .addr()
            .checked_sub(self.entries.as_ptr().addr())?;
        let index = bytes / std::mem::size_of::<DictionaryEntry<'_>>();
        self.entries
            .get(index)
            .filter(|candidate| std::ptr::eq(*candidate, entry))
            .map(|_| EntryIndex(index))
    }

    /// Return the highest-scored entry for a normalized lookup query.
    ///
    /// Entries with the same score stay in source order.
    #[requires(true)]
    #[ensures(true)]
    #[bityzba::expensive_ensures({
        let mut ranked = self.lookup_words(query);
        match ret {
            None => ranked.next().is_none(),
            Some(entry) => {
                ranked.next().is_some_and(|best| std::ptr::eq(best, entry))
                    && ranked.all(|other| other.score.get() <= entry.score.get())
            }
        }
    })]
    pub fn lookup_word(&self, query: &LookupKey) -> Option<&DictionaryEntry<'a>> {
        self.lookup_word_index(query)
            .map(|index| self.entry_at(index))
    }

    /// Return the index of the highest-scored entry for a word.
    #[requires(true)]
    #[ensures(ret.is_none_or(|index| index.0 < self.entries.len()))]
    pub fn lookup_word_index(&self, query: &LookupKey) -> Option<EntryIndex> {
        let targets = self
            .word_index_entry(query.as_str())
            .map(|entry| entry.targets)
            .unwrap_or(&[]);
        targets.iter().copied().min_by(|left, right| {
            self.entry_at(*right)
                .score
                .get()
                .total_cmp(&self.entry_at(*left).score.get())
                .then(left.cmp(right))
        })
    }

    /// Return every entry for a normalized lookup query.
    ///
    /// Entries that share the key are ordered by score, from high to low.
    /// Entries with the same score stay in source order.
    #[requires(true)]
    #[ensures(true)]
    pub fn lookup_words<'lookup>(
        &'lookup self,
        query: &LookupKey,
    ) -> impl Iterator<Item = &'lookup DictionaryEntry<'a>> + 'lookup {
        let mut indices = self
            .word_index_entry(query.as_str())
            .map(|entry| entry.targets)
            .unwrap_or(&[])
            .to_vec();
        rank_entry_indices_by_score(self, &mut indices);
        indices.into_iter().map(|index| self.entry_at(index))
    }

    /// Return the definition entries for one lookup key.
    ///
    /// There is no synthetic or decomposition fallback. Entries that share the
    /// key are ordered by score, from high to low. Entries with the same score
    /// stay in source order.
    #[requires(true)]
    #[ensures(entry_indices_ranked_by_score(self, &ret))]
    #[bityzba::expensive_ensures({
        let mut stored = self
            .word_index_entry(query.as_str())
            .map_or(&[][..], |entry| entry.targets)
            .iter()
            .copied()
            .filter(|index| index.0 < self.entries.len() && self.entries[index.0].has_definition())
            .collect::<Vec<_>>();
        let mut ranked = ret.clone();
        stored.sort();
        ranked.sort();
        stored == ranked
    })]
    pub fn exact_definition_indices(&self, query: &LookupKey) -> Vec<EntryIndex> {
        let mut targets = self
            .word_index_entry(query.as_str())
            .map_or(&[][..], |entry| entry.targets)
            .iter()
            .copied()
            .filter(|index| self.entry_at(*index).has_definition())
            .collect::<Vec<_>>();
        rank_entry_indices_by_score(self, &mut targets);
        targets
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

    /// Return the stored targets for one component sequence, in source order.
    #[requires(true)]
    #[ensures(true)]
    fn cmavo_sequence_targets<S: AsRef<str>>(&self, components: &[S]) -> &'a [EntryIndex] {
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

    /// Return the entries for one cmavo sequence.
    ///
    /// The lookup does not join spellings. Entries that share the key are
    /// ordered by score, from high to low. Entries with the same score stay in
    /// source order. The stored index keeps source order.
    #[requires(true)]
    #[ensures(entry_indices_ranked_by_score(self, &ret))]
    #[bityzba::expensive_ensures({
        let mut stored = self.cmavo_sequence_targets(components).to_vec();
        let mut ranked = ret.clone();
        stored.sort();
        ranked.sort();
        stored == ranked
    })]
    pub fn lookup_cmavo_sequence<S: AsRef<str>>(&self, components: &[S]) -> Vec<EntryIndex> {
        let mut targets = self.cmavo_sequence_targets(components).to_vec();
        rank_entry_indices_by_score(self, &mut targets);
        targets
    }

    /// Return whether a cmavo sequence has any stored entry.
    #[requires(true)]
    #[ensures(true)]
    #[bityzba::expensive_ensures(ret == !self.cmavo_sequence_targets(components).is_empty())]
    pub fn has_cmavo_sequence<S: AsRef<str>>(&self, components: &[S]) -> bool {
        !self.cmavo_sequence_targets(components).is_empty()
    }

    /// Return the first non-empty gloss keyword for each lookup word.
    ///
    /// Each word uses the ranked order from [`Self::lookup_words`]. The first
    /// non-empty gloss in that order is the result. Dictionary-wide validity
    /// is established at generation and explicit validation boundaries rather
    /// than by individual lookup operations.
    #[requires(true)]
    #[ensures(ret.len() == words.len())]
    #[bityzba::expensive_ensures(ret.iter().zip(words).all(|(gloss, word)| {
        *gloss
            == self
                .lookup_words(word)
                .flat_map(|entry| entry.gloss_keywords)
                .map(|keyword| keyword.word)
                .find(|item| !item.is_empty())
    }))]
    pub fn first_gloss_keywords_for_words(&self, words: &[&LookupKey]) -> Vec<Option<&'a str>> {
        words
            .iter()
            .map(|word| {
                self.lookup_words(word)
                    .flat_map(|entry| entry.gloss_keywords)
                    .map(|keyword| keyword.word)
                    .find(|gloss| !gloss.is_empty())
            })
            .collect()
    }

    /// Return dictionary entries whose normalized word starts with a reading
    /// of `prefix`.
    ///
    /// Each reading selects one run of index keys. With two readings, the
    /// runs are joined: an overlap is read once, so no entry repeats. Index
    /// keys stay in lexicographic order. The iterator reads one key at a
    /// time. Entries that share one key are ordered by score, from high to
    /// low. Entries with the same score stay in source order.
    #[requires(true)]
    #[ensures(true)]
    pub fn entries_by_word_prefix<'lookup>(
        &'lookup self,
        prefix: &LookupPrefix,
    ) -> impl Iterator<Item = &'lookup DictionaryEntry<'a>> + 'lookup {
        let mut runs = prefix
            .readings()
            .map(|reading| {
                let key = reading.as_str();
                let start = self.word_index.partition_point(|entry| entry.key < key);
                let end = self
                    .word_index
                    .partition_point(|entry| entry.key < key || entry.key.starts_with(key));
                start..end
            })
            .collect::<Vec<_>>();
        runs.sort_by_key(|run| run.start);
        // Join the runs that overlap, so each index position is read once.
        let mut positions = Vec::<std::ops::Range<usize>>::with_capacity(runs.len());
        for run in runs {
            match positions.last_mut() {
                Some(last) if run.start <= last.end => last.end = last.end.max(run.end),
                _ => positions.push(run),
            }
        }
        let mut positions = positions.into_iter().flatten();
        let mut pending = Vec::new();
        let mut cursor = 0;
        std::iter::from_fn(move || {
            loop {
                if cursor < pending.len() {
                    let index = pending[cursor];
                    cursor += 1;
                    return Some(self.entry_at(index));
                }
                let position = positions.next()?;
                pending = self.word_index[position].targets.to_vec();
                rank_entry_indices_by_score(self, &mut pending);
                cursor = 0;
            }
        })
    }

    /// Return all dictionary entries associated with a rafsi query.
    ///
    /// Entries that share the key are ordered by score, from high to low.
    /// Entries with the same score stay in source order. Two matches for one
    /// entry stay in input order.
    #[requires(true)]
    #[ensures(true)]
    pub fn lookup_rafsi<'lookup>(
        &'lookup self,
        query: &LookupKey,
    ) -> impl Iterator<Item = RafsiMatch<'lookup, 'a>> + 'lookup {
        let mut targets = self
            .rafsi_index_entry(query.as_str())
            .map(|entry| entry.targets)
            .unwrap_or(&[])
            .to_vec();
        rank_rafsi_targets_by_entry_score(self, &mut targets);
        targets.into_iter().map(|target| RafsiMatch {
            entry_index: target.entry_index,
            entry: self.entry_at(target.entry_index),
            source: target.source,
        })
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
    /// handed to a new gismu no matter which word holds it. The standing of the
    /// claim is the listed rafsi's own ([`Rafsi::standing`]), which need not
    /// match the claimant's word type: an official cmavo can hold an
    /// experimental rafsi.
    ///
    /// Only [`RafsiSource::Listed`] matches count. Derived fu'ivla stems
    /// carry no assignment. Universal forms of well-formed gismu cannot
    /// spell a short rafsi: a short rafsi is
    /// three letters, or four with an apostrophe third (CVV, `sa'i`), whereas
    /// the universal forms of a CVCCV or CCVCV gismu are its apostrophe-free
    /// four- and five-letter prefixes. Nothing forces a gismu-typed entry to
    /// be spelled that way, though — [`Dictionary::validate`] does not check
    /// word spelling against word type — so the filter is a real guard rather
    /// than a formality.
    #[requires(!rafsi.is_empty())]
    #[bityzba::expensive_ensures(
        ret.is_free()
            != self
                .lookup_rafsi(&LookupKey::from_stored(rafsi))
                .any(|matched| matched.source.listed_standing().is_some()),
        "any listed claimant takes the form, whatever its word type"
    )]
    #[bityzba::expensive_ensures(
        (ret.claim_kind() == Some(RafsiClaimKind::Official))
            == self
                .lookup_rafsi(&LookupKey::from_stored(rafsi))
                .any(|matched| matched.source.listed_standing() == Some(RafsiClaimKind::Official)),
        "an official claim outranks any experimental one"
    )]
    #[bityzba::expensive_ensures(
        ret.claimant_words().iter().map(String::as_str).eq(
            self.lookup_rafsi(&LookupKey::from_stored(rafsi))
                .filter(|matched| {
                    matched.source.listed_standing().is_some()
                        && matched.source.listed_standing() == ret.claim_kind()
                })
                .map(|matched| matched.entry.word)
        ),
        "the reported words are the claimants of the winning standing, highest score first, and equal scores keep source order"
    )]
    fn rafsi_availability(&self, rafsi: &str) -> RafsiAvailability {
        let mut official = Vec::new();
        let mut experimental = Vec::new();
        for matched in self.lookup_rafsi(&LookupKey::from_stored(rafsi)) {
            let word = matched.entry.word.to_owned();
            match matched.source.listed_standing() {
                Some(RafsiClaimKind::Official) => official.push(word),
                Some(RafsiClaimKind::Experimental) => experimental.push(word),
                None => {}
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
    pub sources: &'a [DictionaryLujvoSource<'a>],
}

/// A ranked dictionary entry behind a rafsi.
#[invariant(true, "Dictionary::validate checks the source against the entry table")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DictionaryLujvoSource<'a> {
    pub entry_index: EntryIndex,
    pub word: &'a str,
}

/// Segment kind in a precomputed lujvo decomposition.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(::Rafsi => true)]
#[invariant(::FuivlaRafsi => true)]
#[invariant(::Hyphen => true)]
pub enum DictionaryLujvoSegmentKind {
    Rafsi,
    FuivlaRafsi,
    Hyphen,
}

impl DictionaryLujvoSegmentKind {
    #[requires(true)]
    #[ensures(ret == (self != Self::Hyphen))]
    pub fn is_rafsi(self) -> bool {
        self != Self::Hyphen
    }
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

    #[requires(true)]
    #[ensures(ret == matches!(self, Self::Fuivla | Self::ObsoleteFuivla))]
    pub const fn is_fuivla_like(self) -> bool {
        matches!(self, Self::Fuivla | Self::ObsoleteFuivla)
    }

    /// Return the highest standing a rafsi assignment on a word of this type
    /// can have.
    ///
    /// Each listed rafsi carries its own standing ([`Rafsi::standing`]); this
    /// is only its ceiling, which the importer and [`Dictionary::validate`]
    /// enforce. An official word may hold experimental rafsi as well as
    /// official ones, but an experimental, obsolete or untyped word may hold
    /// only experimental ones.
    ///
    /// The match is deliberately exhaustive rather than defaulting: a new word
    /// type must be classified consciously, because an unclassified type could
    /// let an experimental word's rafsi pass as official.
    ///
    /// The ceiling is [`Experimental`] exactly when the type itself is
    /// experimental or obsolete, and [`Official`] otherwise, which the
    /// postcondition cross-checks against the Lensisku type names. [`Nalvla`]
    /// is the one exception: it is not named for a register, but an untyped
    /// entry cannot bind the standard register either.
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
        "only experimental, obsolete, and untyped word types are capped at experimental standing"
    )]
    pub fn max_rafsi_standing(self) -> RafsiClaimKind {
        match self {
            // Standard brivla and cmavo can hold official assignments.
            Self::Gismu
            | Self::Lujvo
            | Self::ZeiLujvo
            | Self::Cmavo
            | Self::CmavoCompound
            | Self::Fuivla
            | Self::Cmevla
            // Letterals and phrases are not rafsi-bearing in practice, but
            // they belong to the standard register like other standard words.
            | Self::BuLetteral
            | Self::Phrase => RafsiClaimKind::Official,
            // Experimental words are proposals, so their rafsi can only be
            // provisional and yield to any official claim on the same form.
            Self::ExperimentalGismu | Self::ExperimentalCmavo => RafsiClaimKind::Experimental,
            // Obsolete words were withdrawn from the standard register, so
            // their rafsi are likewise provisional rather than binding: the
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

/// Listed rafsi with the standing its assignment has.
///
/// Lensisku records each rafsi as official or experimental separately from the
/// word's own type: since 2026-10 the official cmavo `ma` carries the
/// experimental rafsi `maz`. So the standing belongs to the rafsi, not to the
/// word. The form is non-empty, which [`Dictionary::validate`] checks; the
/// type stays a plain struct so generated static entries remain `const`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[invariant(true)]
pub struct Rafsi<'a> {
    pub form: &'a str,
    pub standing: RafsiClaimKind,
}

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
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
///
/// A listed match carries the standing of that listed rafsi, so claim checks
/// read it directly instead of re-deriving it from the word.
#[invariant(true)]
#[invariant(::Listed { .. } => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RafsiSource {
    Listed { standing: RafsiClaimKind },
    UniversalShort,
    UniversalLong,
    FuivlaStem,
}

impl RafsiSource {
    /// The standing of a listed rafsi, or `None` for a derived form.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self, Self::Listed { .. }))]
    pub const fn listed_standing(self) -> Option<RafsiClaimKind> {
        match self {
            Self::Listed { standing } => Some(standing),
            Self::UniversalShort | Self::UniversalLong | Self::FuivlaStem => None,
        }
    }
}

/// Result of a rafsi lookup.
#[derive(Debug, Clone, Copy, PartialEq)]
#[invariant(true)]
pub struct RafsiMatch<'entry, 'dict> {
    pub entry_index: EntryIndex,
    pub entry: &'entry DictionaryEntry<'dict>,
    pub source: RafsiSource,
}

/// Standing of a rafsi assignment, and of the claims on a short rafsi.
///
/// An official assignment outranks experimental ones: a rafsi that some word
/// holds officially is reported as officially taken even when other words
/// hold it experimentally. The standing belongs to each listed rafsi
/// ([`Rafsi::standing`]); [`WordType::max_rafsi_standing`] only caps it.
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
    let mut pattern_index = Vec::with_capacity(entries.len());
    let mut cmavo_map: BTreeMap<Vec<String>, Vec<EntryIndex>> = BTreeMap::new();
    // Every fu'ivla F has the full form F + `'y`, which names F alone. A short
    // form equal to a full form would name a second word, so it is not
    // indexed (`eskua` would shorten to `esku`).
    let fuivla_full_forms = entries
        .iter()
        .filter(|entry| entry.word_type.is_fuivla_like())
        .map(|entry| normalize_lookup_query(entry.word))
        .collect::<std::collections::BTreeSet<_>>();

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
                .entry(normalize_lookup_query(rafsi.form))
                .or_default()
                .push(RafsiIndexTarget {
                    entry_index,
                    source: RafsiSource::Listed {
                        standing: rafsi.standing,
                    },
                });
            pattern_rafsi_keys.push(normalize_lookup_query(rafsi.form));
        }

        if entry.word_type.is_gismu_like() {
            for (rafsi, source) in universal_gismu_rafsi_forms(entry.word) {
                pattern_rafsi_keys.push(normalize_lookup_query(&rafsi));
                rafsi_map.entry(rafsi).or_default().push(RafsiIndexTarget {
                    entry_index,
                    source,
                });
            }
        }

        if entry.word_type.is_fuivla_like()
            && let Some(stem) = fuivla_stem_rafsi(entry.word)
            && !fuivla_full_forms.contains(&normalize_lookup_query(&stem))
        {
            // A derived stem is in the exact rafsi index only. Wildcard rafsi
            // search keeps to listed and universal forms, because nearly every
            // fu'ivla has a stem, and those stems would bury the rafsi.
            let key = normalize_lookup_query(&stem);
            rafsi_map.entry(key).or_default().push(RafsiIndexTarget {
                entry_index,
                source: RafsiSource::FuivlaStem,
            });
        }

        pattern_index.push(OwnedPatternIndexEntry {
            entry_index,
            word_key: normalize_lookup_query(entry.word),
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

#[requires(indices.iter().all(|index| index.0 < dictionary.entries.len()))]
#[ensures(entry_indices_ranked_by_score(dictionary, indices))]
fn rank_entry_indices_by_score(dictionary: &Dictionary<'_>, indices: &mut [EntryIndex]) {
    indices.sort_by(|left, right| {
        dictionary
            .entry_at(*right)
            .score
            .get()
            .total_cmp(&dictionary.entry_at(*left).score.get())
            .then(left.cmp(right))
    });
}

#[requires(true)]
#[ensures(true)]
fn entry_indices_ranked_by_score(dictionary: &Dictionary<'_>, indices: &[EntryIndex]) -> bool {
    indices
        .iter()
        .all(|index| index.0 < dictionary.entries.len())
        && indices.windows(2).all(|pair| {
            let order = dictionary.entries[pair[0].0]
                .score
                .get()
                .total_cmp(&dictionary.entries[pair[1].0].score.get());
            order.is_gt() || (order.is_eq() && pair[0] <= pair[1])
        })
}

#[requires(targets.iter().all(|target| target.entry_index.0 < dictionary.entries.len()))]
#[ensures(rafsi_targets_ranked_by_entry_score(dictionary, targets))]
fn rank_rafsi_targets_by_entry_score(
    dictionary: &Dictionary<'_>,
    targets: &mut [RafsiIndexTarget],
) {
    // Equal score and the same entry compare equal. `sort_by` is stable, so
    // the listed form and the universal form of one entry keep input order.
    targets.sort_by(|left, right| {
        dictionary
            .entry_at(right.entry_index)
            .score
            .get()
            .total_cmp(&dictionary.entry_at(left.entry_index).score.get())
            .then(left.entry_index.cmp(&right.entry_index))
    });
}

#[requires(true)]
#[ensures(true)]
fn rafsi_targets_ranked_by_entry_score(
    dictionary: &Dictionary<'_>,
    targets: &[RafsiIndexTarget],
) -> bool {
    targets
        .iter()
        .all(|target| target.entry_index.0 < dictionary.entries.len())
        && targets.windows(2).all(|pair| {
            let order = dictionary.entries[pair[0].entry_index.0]
                .score
                .get()
                .total_cmp(&dictionary.entries[pair[1].entry_index.0].score.get());
            order.is_gt() || (order.is_eq() && pair[0].entry_index <= pair[1].entry_index)
        })
}

/// Return the lookup key of `raw`, as the dictionary index stores it.
///
/// This is the key of `normalize_lookup_pattern` for `raw` alone, with one
/// difference: an index key cannot fail, so a character that a lookup rejects
/// is left out. See `lookup_key_char` for the character rules.
///
/// Most stored text, such as a headword or a completion label, is already
/// plain Lojban in lowercase ASCII. For that text the converter reads each
/// letter as itself, so the key is the words with commas removed, joined by
/// one space, and the function takes that short path. The expensive
/// postcondition checks that the short path gives the converter's key.
#[requires(true)]
#[ensures(!ret.starts_with(' ') && !ret.ends_with(' '))]
#[bityzba::expensive_ensures(!ret.contains("  ") && ret.chars().all(|value| value == ' ' || is_lookup_char(value)))]
#[bityzba::expensive_ensures(
    ret == key_text_of(
        &build_lookup_key::<()>(&[LookupPatternPart::Literal(raw)], LookupKeyMode::Index)
            .expect("an index key leaves out what a lookup rejects")
    ),
    "the short path gives the converter's key"
)]
pub fn normalize_lookup_query(raw: &str) -> String {
    if let Some(key) = plain_lojban_key(raw) {
        return key;
    }
    let keyed = build_lookup_key::<()>(&[LookupPatternPart::Literal(raw)], LookupKeyMode::Index)
        .expect("an index key leaves out what a lookup rejects");
    key_text_of(&keyed)
}

/// The key of text that the converter reads letter by letter as itself, or
/// `None` for any other text.
///
/// That text has only lowercase ASCII letters other than `h`, `q` and `w`,
/// digits, apostrophes, commas, periods and ASCII whitespace. The Latin
/// converter reads each such letter, digit and apostrophe as itself and adds
/// no implicit apostrophe, a comma is a mark inside a word, and a period or a
/// space is a word break. `h` is an apostrophe spelling, and `q` and `w` are
/// no Lojban letters, so they take the full path.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.starts_with(' ') && !key.ends_with(' ') && !key.contains("  ")))]
fn plain_lojban_key(raw: &str) -> Option<String> {
    let plain = raw.bytes().all(|byte| {
        matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'\'' | b',' | b'.')
            && !matches!(byte, b'h' | b'q' | b'w')
            || byte.is_ascii_whitespace()
    });
    if !plain {
        return None;
    }
    let mut key = String::with_capacity(raw.len());
    for word in raw.split(|value: char| value == '.' || value.is_ascii_whitespace()) {
        let mut letters = word.chars().filter(|value| *value != ',').peekable();
        if letters.peek().is_none() {
            continue;
        }
        if !key.is_empty() {
            key.push(' ');
        }
        key.extend(letters);
    }
    Some(key)
}

/// A key of the word index or the rafsi index.
///
/// Every `Dictionary` lookup takes this type, so the caller must say which
/// rule makes the key. `from_typed` is the rule for text that a user typed:
/// it fails on a character that gentufa rejects, such as `â` in `klâma` or
/// `#` in `klama#`, and the error names that character. `from_stored` is the
/// rule of the index itself, for a stored headword, a canonical rafsi or
/// morphology phonemes: it leaves such a character out, so the headword
/// `klâma` keeps the key `klama`. Typed text therefore cannot reach the
/// index through the index rule by accident.
#[invariant(!text.starts_with(' ') && !text.ends_with(' ') && !text.contains("  "), "one space between key words")]
#[invariant(text.chars().all(|value| value == ' ' || is_lookup_char(value)), "only key characters")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupKey {
    text: String,
}

/// The key of a typed prefix of a word, for a search by word prefix.
///
/// A prefix is not a whole query: it is the text before a cursor, so its
/// last unit can still wait for a partner. The prefix rule follows the
/// typed-query rule (`LookupKey::from_typed`) with two differences, both for
/// such a last unit:
///
/// - A Zbalermorna shorthand mark with no payload yet (a bare U+ED8B at the
///   end) adds nothing, because its letters come from the payload.
/// - A vowel that a following breve would turn into a glide has two
///   readings, and the prefix holds the key of each. Cyrillic `шои` reads
///   `co'i`, and `шои` + U+0306 reads `coĭ`, so the prefix searches for both
///   `co'i` and `coi`.
///
/// So a prefix has one or two keys, and the type cannot hold more or none.
/// A prefix with any other character that a lookup rejects is still an
/// error. This is a separate type, so an exact lookup cannot take a prefix
/// key, and a prefix search cannot take an exact key.
#[invariant(glide.as_ref().is_none_or(|glide| glide != typed), "two readings have two different keys")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupPrefix {
    /// The reading of the text as typed.
    typed: LookupKey,
    /// The reading with a following breve, when it is another key.
    glide: Option<LookupKey>,
}

impl LookupPrefix {
    #[requires(true)]
    #[ensures(true)]
    #[bityzba::expensive_ensures(
        match (&ret, text.trim_end().strip_suffix(awaits_next_unit)) {
            (Ok(prefix), Some(rest)) => LookupKey::from_typed(rest).is_ok_and(|key| key == prefix.typed) && prefix.glide.is_none(),
            (Ok(prefix), None) => LookupKey::from_typed(text).is_ok_and(|key| key == prefix.typed),
            (Err(_), _) => true,
        },
        "the typed reading is the typed key of the text before any waiting shorthand mark"
    )]
    pub fn from_typed(text: &str) -> Result<Self, LookupPatternError> {
        let text = text.trim_end();
        if let Some(rest) = text.strip_suffix(awaits_next_unit) {
            let typed = LookupKey::from_typed(rest)?;
            return Ok(new!(LookupPrefix { typed, glide: None }));
        }
        let typed = LookupKey::from_typed(text)?;
        let glide = LookupKey::from_typed(&format!("{text}{GLIDE_PARTNER_MARK}"))
            .ok()
            .filter(|glide| *glide != typed);
        Ok(new!(LookupPrefix { typed, glide }))
    }

    /// The one or two keys of this prefix, the typed reading first.
    #[requires(true)]
    #[ensures(true)]
    pub fn readings(&self) -> impl Iterator<Item = &LookupKey> {
        std::iter::once(&self.typed).chain(self.glide.as_ref())
    }

    /// Whether `key` starts with one of the readings.
    #[requires(true)]
    #[ensures(ret == self.readings().any(|reading| key.starts_with(reading.as_str())))]
    pub fn matches(&self, key: &str) -> bool {
        key.starts_with(self.typed.as_str())
            || self
                .glide
                .as_ref()
                .is_some_and(|glide| key.starts_with(glide.as_str()))
    }
}

impl LookupKey {
    /// The key of text that a user typed. Empty or blank text gives the
    /// empty key.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|key| !key.is_empty()) || text.trim().is_empty() || ret.is_err())]
    pub fn from_typed(text: &str) -> Result<Self, LookupPatternError> {
        let keyed =
            build_lookup_key::<()>(&[LookupPatternPart::Literal(text)], LookupKeyMode::Lookup)?;
        Ok(new!(LookupKey {
            text: key_text_of(&keyed),
        }))
    }

    /// The key of stored text: a headword, a canonical rafsi, or phonemes.
    #[requires(true)]
    #[ensures(true)]
    pub fn from_stored(text: &str) -> Self {
        new!(LookupKey {
            text: normalize_lookup_query(text),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    #[requires(true)]
    #[ensures(ret == self.text.is_empty())]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

/// The short rafsi form of a fu'ivla, from CLL 4.16.
///
/// Only a fu'ivla whose final nucleus is one vowel has a short form. The
/// morphology decides the nucleus: its reading of the word marks each glide,
/// so `barbekeiu` reads as `barbekéĭu`, with the single vowel `u` after the
/// glide `ĭ`, while `adzau` reads as `adzáŭ`, which ends in the diphthong
/// `aŭ` and has no short form. The final vowel goes, and an apostrophe right
/// before it goes too. A consonant-final stem takes `y` (`ierxi` gives
/// `ierx`), and a vowel-final stem takes `'y` (`actce'o` gives `actce`,
/// `barbekeiu` gives `barbekei`). The morphology also decides whether the
/// stem is a fu'ivla rafsi, and so whether the short form is safe. The word
/// must be one morphological word: an obsolete headword such as `aierne`
/// splits into several words, so no lujvo can start with its stem. Every
/// fu'ivla also has the full form, the word itself before `'y`, which needs
/// no index entry. A short form is refused when it adds the experimental CgV
/// warning: a compound built with it gets the warning, and the same compound
/// built with the full form does not (`bolviia` has no short form, but
/// `abniena` keeps `abnien`, because `abniena'y` has the warning too).
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|stem| !stem.is_empty()))]
#[bityzba::expensive_ensures(ret.as_ref().is_none_or(|stem| is_fuhivla_rafsi_form(stem)))]
#[bityzba::expensive_ensures(
    ret.as_ref().is_none_or(|stem| normalize_lojban_input_text(word)
        .is_some_and(|text| !short_form_adds_experimental_cgv(stem, text.trim_matches('.')))),
    "a short form adds no experimental CgV warning to its compounds"
)]
#[bityzba::expensive_ensures(ret.is_some() -> normalize_lojban_input_text(word).is_some_and(|text| single_word_phonemes(text.trim_matches('.')).is_some()))]
pub fn fuivla_stem_rafsi(word: &str) -> Option<String> {
    let normalized = normalize_lojban_input_text(word)?;
    let full = normalized.trim_matches('.');
    let phonemes = single_word_phonemes(full)?;
    let stem = short_form_of_phonemes(&phonemes)?;
    (is_fuhivla_rafsi_form(&stem) && !short_form_adds_experimental_cgv(&stem, full)).then_some(stem)
}

/// True when a compound built with the short form gets the experimental CgV
/// warning from the morphology, and the same compound built with the full
/// form does not.
///
/// The owner refuses such a short form (`bolviia` keeps `bolviia'y`, not
/// `bolvii'y`). A word that carries the warning itself keeps its short form,
/// because the short form adds no new warning (`abniena` keeps `abnien`).
/// The check builds `stem` + its hyphen + `kla` and `full` + `'ykla`, and
/// reads the morphology's own warnings. A glide cluster inside a rafsi is the
/// same in every compound, so one probe per form is enough.
#[requires(!stem.is_empty() && !full.is_empty())]
#[ensures(true)]
fn short_form_adds_experimental_cgv(stem: &str, full: &str) -> bool {
    let hyphen = if ends_with_vowel(stem) { "'y" } else { "y" };
    compound_has_experimental_cgv(&format!("{stem}{hyphen}kla"))
        && !compound_has_experimental_cgv(&format!("{full}'ykla"))
}

/// True when the morphology gives `text` the experimental CgV warning.
#[requires(!text.is_empty())]
#[ensures(true)]
fn compound_has_experimental_cgv(text: &str) -> bool {
    segment_words_with_modifiers_with_options_and_source_id_attempt(
        text,
        &MorphologyOptions::default(),
        None,
    )
    .into_data()
    .warnings
    .iter()
    .any(|warning| warning.kind == MorphologyWarningKind::ExperimentalCgv)
}

/// The phonemes that the morphology reads for `text`, when `text` is exactly
/// one plain word. The phonemes mark stress and glides.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|phonemes| !phonemes.is_empty()))]
fn single_word_phonemes(text: &str) -> Option<String> {
    let words = segment_words_with_modifiers_with_options_and_source_id(
        text,
        &MorphologyOptions::default(),
        None,
    )
    .ok()?;
    let [word] = words.as_slice() else {
        return None;
    };
    Some(word.bare_word()?.phonemes().as_str().to_owned())
}

/// The short form of a word from its phonemes, before the rafsi check.
///
/// The last phoneme must be a vowel that is the whole final nucleus: not a
/// glide (`ŭ` in `adzáŭ` ends a diphthong) and not after another vowel. A
/// glide before it starts the syllable and stays in the stem
/// (`barbekéĭu` gives `barbekei`).
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|stem| !stem.is_empty()))]
fn short_form_of_phonemes(phonemes: &str) -> Option<String> {
    // Keep the glide marks and fold only the stress marks.
    let mut chars = phonemes
        .chars()
        .filter(|value| *value != ',')
        .filter_map(strip_lojban_diacritic)
        .collect::<Vec<_>>();
    let last = chars.pop()?;
    if !matches!(last, 'a' | 'e' | 'i' | 'o' | 'u') {
        return None;
    }
    match chars.last().copied() {
        Some('\'') => {
            chars.pop();
        }
        Some('a' | 'e' | 'i' | 'o' | 'u' | 'y') | None => return None,
        Some(_) => {}
    }
    let stem = canonicalize_text(&chars.into_iter().collect::<String>());
    (!stem.is_empty()).then_some(stem)
}

/// One piece of a lookup pattern, before keying.
///
/// A literal is source text. A wildcard stands for one pattern token.
#[invariant(true)]
#[invariant(::Literal(_) => true)]
#[invariant(::Wildcard(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupPatternPart<'a, T> {
    Literal(&'a str),
    Wildcard(T),
}

/// One piece of a lookup pattern after keying.
///
/// `Text` is non-empty key text. A wildcard is the same token the caller
/// supplied.
#[invariant(::Text(text) => !text.is_empty())]
#[invariant(::Wildcard(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupKeyPart<T> {
    Text(String),
    Wildcard(T),
}

impl<T> LookupKeyPart<T> {
    /// The key text of this part, when the part is text.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(LookupKeyPart::Text(_))))]
    pub fn text(&self) -> Option<&str> {
        match self.as_data() {
            data!(LookupKeyPart::Text(text)) => Some(text.as_str()),
            data!(LookupKeyPart::Wildcard(_)) => None,
        }
    }

    /// The wildcard token of this part, when the part is a wildcard.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(LookupKeyPart::Wildcard(_))))]
    pub fn wildcard(&self) -> Option<&T> {
        match self.as_data() {
            data!(LookupKeyPart::Wildcard(token)) => Some(token),
            data!(LookupKeyPart::Text(_)) => None,
        }
    }
}

/// A lookup pattern that a lookup rejects.
///
/// `InvalidCharacter` names a character that gentufa also rejects, such as
/// a letter with a non-stress diacritic. `NoLookupCharacter` names a literal
/// run that has only characters a key leaves out, such as `,` in `*,*`.
#[invariant(::InvalidCharacter { character } => !character.is_whitespace())]
#[invariant(::NoLookupCharacter { text } => !text.is_empty() && text.trim() == text)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LookupPatternError {
    InvalidCharacter { character: char },
    NoLookupCharacter { text: String },
}

impl LookupPatternError {
    /// The rejected character, when the error names one.
    #[requires(true)]
    #[ensures(ret.is_none_or(|character| !character.is_whitespace()))]
    pub fn invalid_character(&self) -> Option<char> {
        match self.as_data() {
            data!(LookupPatternError::InvalidCharacter { character }) => Some(*character),
            data!(LookupPatternError::NoLookupCharacter { .. }) => None,
        }
    }
}

impl std::fmt::Display for LookupPatternError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.as_data() {
            data!(LookupPatternError::InvalidCharacter { character }) => {
                write!(formatter, "unsupported character `{character}`")
            }
            data!(LookupPatternError::NoLookupCharacter { text }) => {
                write!(
                    formatter,
                    "`{text}` has no character that a lookup can search for"
                )
            }
        }
    }
}

/// Key one whole lookup pattern.
///
/// `parts` is literal text and wildcard tokens in pattern order. The
/// function transcribes each run of literals between wildcards as one stream.
/// It carries the converter state across adjacent literals. A wildcard resets
/// that state. Each transcription step then follows `lookup_key_char`:
/// a letter stays, a word break becomes one space, a comma or a middle dot
/// disappears, and any other character is an error that names it. A wildcard
/// counts as a part that stays. A space stays when a surviving part is on
/// both sides. A literal run that keeps no letter is an error. A pattern with
/// no wildcard has the same key as that text alone.
#[requires(true)]
#[ensures(true)]
#[bityzba::expensive_ensures({
    if parts
        .iter()
        .any(|part| matches!(part, LookupPatternPart::Wildcard(_)))
    {
        true
    } else {
        let mut joined = String::new();
        for part in parts {
            if let LookupPatternPart::Literal(text) = part {
                joined.push_str(text);
            }
        }
        let whole = build_lookup_key::<T>(
            &[LookupPatternPart::Literal(joined.as_str())],
            LookupKeyMode::Lookup,
        );
        // The lookup transcript and the strict converter of the parser are
        // two walks. When the strict conversion succeeds, both give one key.
        let agrees_with_parser =
            match (jbotci_morphology::normalize_lojban_input_text(&joined), ret.as_ref()) {
                (None, _) => true,
                (Some(strict), Ok(actual)) => key_text_of(actual) == normalize_lookup_query(&strict),
                // A run with no letter, such as `, ,`, is an error here and
                // an empty key there.
                (Some(strict), Err(error)) => {
                    error.invalid_character().is_none() && normalize_lookup_query(&strict).is_empty()
                }
            };
        agrees_with_parser
            && match (ret.as_ref(), whole.as_ref()) {
                (Ok(actual), Ok(expected)) => key_text_of(actual) == key_text_of(expected),
                (Err(actual), Err(expected)) => actual == expected,
                _ => false,
            }
    }
})]
pub fn normalize_lookup_pattern<T: Clone>(
    parts: &[LookupPatternPart<'_, T>],
) -> Result<Vec<LookupKeyPart<T>>, LookupPatternError> {
    build_lookup_key(parts, LookupKeyMode::Lookup)
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

/// Whether a key may fail.
///
/// A lookup reports a rejected character. An index key leaves it out.
#[invariant(true)]
#[invariant(::Lookup => true)]
#[invariant(::Index => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LookupKeyMode {
    Lookup,
    Index,
}

/// What a lookup key does with one transcription step.
#[invariant(::Letter(value) => is_lookup_char(*value))]
#[invariant(::WordBreak => true)]
#[invariant(::Removed => true)]
#[invariant(::Invalid(value) => !value.is_whitespace())]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LookupKeyChar {
    Letter(char),
    WordBreak,
    Removed,
    Invalid(char),
}

/// The key rule for one transcription step.
///
/// The converter of the parser decides the class of each character (see
/// `jbotci_morphology::LookupStep`). A letter keeps its base letter: its
/// stress mark and its glide mark fold away. A comma is a mark inside a
/// word, so it disappears. A word break is a space (`broda-zei-brode` keys
/// as `broda zei brode`). A character that the converter cannot read is
/// invalid, as in gentufa: a letter with another diacritic, a letter that no
/// converter reads such as `q` or `w`, a combining breve, and other symbols.
/// The one exception is the middle dot U+00B7 and the hyphenation point
/// U+2027, which the owner ruled to be marks inside a word, like a comma.
#[requires(true)]
#[ensures(matches!(step.into_data(), data!(LookupStep::WordBreak(_))) == matches!(ret.as_data(), data!(LookupKeyChar::WordBreak)))]
fn lookup_key_char(step: LookupStep) -> LookupKeyChar {
    match step.into_data() {
        data!(LookupStep::Letter(',')) => new!(LookupKeyChar::Removed),
        data!(LookupStep::Letter(value)) => {
            let folded = fold_lojban_diacritic(value).unwrap_or(value);
            if is_lookup_char(folded) {
                new!(LookupKeyChar::Letter(folded))
            } else {
                new!(LookupKeyChar::Invalid(value))
            }
        }
        data!(LookupStep::WordBreak(_)) => new!(LookupKeyChar::WordBreak),
        data!(LookupStep::Unread('\u{00b7}' | '\u{2027}')) => new!(LookupKeyChar::Removed),
        data!(LookupStep::Unread(value)) => new!(LookupKeyChar::Invalid(value)),
    }
}

#[requires(true)]
#[ensures(true)]
fn key_text_of<T>(parts: &[LookupKeyPart<T>]) -> String {
    let mut text = String::new();
    for part in parts {
        if let data!(LookupKeyPart::Text(value)) = part.as_data() {
            text.push_str(value);
        }
    }
    text
}

/// Key one literal run and add it to `current`.
///
/// A word break sets `pending_space`, and the space is written only before
/// the next letter, or before a wildcard (see `build_lookup_key`). `started`
/// is true when a letter or a wildcard came before the run. In the lookup
/// mode, the first character that the key rejects is an error, and a run
/// with text that adds nothing to the key is an error too. A run adds to the
/// key when it keeps a letter, or when it leaves a space between two
/// searchable parts: the pending space of a word break, when a wildcard
/// follows (`before_wildcard`). So `*!*` and `* , *` key as `* *`, while
/// `*,*`, `!*` and a trailing `*!` add nothing.
#[requires(!current.ends_with(' '))]
#[ensures(!current.ends_with(' '))]
#[ensures(*pending_space -> (started || !current.is_empty()))]
fn key_literal_run(
    run: &str,
    mode: LookupKeyMode,
    before_wildcard: bool,
    started: bool,
    current: &mut String,
    pending_space: &mut bool,
) -> Result<(), LookupPatternError> {
    let mut started = started;
    let mut kept = false;
    let mut invalid = None;
    transcribe_lookup_run(run, |step| match lookup_key_char(step).into_data() {
        data!(LookupKeyChar::WordBreak) => {
            if started {
                *pending_space = true;
            }
        }
        data!(LookupKeyChar::Removed) => {}
        data!(LookupKeyChar::Invalid(value)) => {
            invalid.get_or_insert(value);
        }
        data!(LookupKeyChar::Letter(letter)) => {
            if *pending_space {
                current.push(' ');
                *pending_space = false;
            }
            current.push(letter);
            started = true;
            kept = true;
        }
    });
    if mode == LookupKeyMode::Lookup {
        if let Some(character) = invalid {
            return Err(new!(LookupPatternError::InvalidCharacter { character }));
        }
        let adds_space = before_wildcard && *pending_space;
        let trimmed = run.trim();
        if !kept && !adds_space && !trimmed.is_empty() {
            return Err(new!(LookupPatternError::NoLookupCharacter {
                text: trimmed.to_owned(),
            }));
        }
    }
    Ok(())
}

/// Key a lookup pattern: literal runs between wildcards, in order.
///
/// Each run of adjacent literals is one text for the converter, so a pair
/// that spans two literals, such as a base letter and its combining mark,
/// converts as one unit. A wildcard ends the run, and it counts as a part
/// that stays. A space stays when a surviving part is on both sides.
#[requires(true)]
#[ensures(mode == LookupKeyMode::Index -> ret.is_ok())]
fn build_lookup_key<T: Clone>(
    parts: &[LookupPatternPart<'_, T>],
    mode: LookupKeyMode,
) -> Result<Vec<LookupKeyPart<T>>, LookupPatternError> {
    let mut keyed = Vec::new();
    let mut current = String::new();
    let mut pending_space = false;
    let mut run = String::new();
    for part in parts {
        match part {
            LookupPatternPart::Literal(text) => run.push_str(text),
            LookupPatternPart::Wildcard(token) => {
                let started = !keyed.is_empty() || !current.is_empty();
                key_literal_run(&run, mode, true, started, &mut current, &mut pending_space)?;
                run.clear();
                if pending_space {
                    current.push(' ');
                    pending_space = false;
                }
                if !current.is_empty() {
                    keyed.push(new!(LookupKeyPart::Text(std::mem::take(&mut current))));
                }
                keyed.push(new!(LookupKeyPart::Wildcard(token.clone())));
            }
        }
    }
    let started = !keyed.is_empty() || !current.is_empty();
    key_literal_run(&run, mode, false, started, &mut current, &mut pending_space)?;
    if !current.is_empty() {
        keyed.push(new!(LookupKeyPart::Text(current)));
    }
    Ok(keyed)
}

#[requires(true)]
#[ensures(true)]
fn is_lookup_char(value: char) -> bool {
    value.is_ascii_lowercase() || value.is_ascii_digit() || value == '\''
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
    if entry.rafsi.iter().any(|rafsi| rafsi.form.is_empty()) {
        return Err(DictionaryValidationError::InvalidEntry {
            index,
            reason: "rafsi is empty",
        });
    }
    if entry.word_type.max_rafsi_standing() == RafsiClaimKind::Experimental
        && entry
            .rafsi
            .iter()
            .any(|rafsi| rafsi.standing == RafsiClaimKind::Official)
    {
        return Err(DictionaryValidationError::InvalidEntry {
            index,
            reason: "an experimental-standing word lists an official rafsi",
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
        if lujvo_entry.segments.is_empty() {
            return Err(DictionaryValidationError::InvalidLujvoIndexEntry {
                index,
                reason: "decomposition has no segments",
            });
        }
        let rafsi_count = lujvo_entry
            .segments
            .iter()
            .filter(|segment| segment.kind.is_rafsi())
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
        if lujvo_entry
            .segments
            .iter()
            .any(|segment| !segment.kind.is_rafsi() && !segment.sources.is_empty())
        {
            return Err(DictionaryValidationError::InvalidLujvoIndexEntry {
                index,
                reason: "hyphen segment has a source word",
            });
        }
        if lujvo_entry.segments.iter().any(|segment| {
            segment.kind == DictionaryLujvoSegmentKind::Rafsi && segment.sources.len() > 1
        }) {
            return Err(DictionaryValidationError::InvalidLujvoIndexEntry {
                index,
                reason: "ordinary rafsi segment has more than one source",
            });
        }
        if lujvo_entry
            .segments
            .iter()
            .flat_map(|segment| segment.sources)
            .any(|source| {
                entries
                    .get(source.entry_index.0)
                    .is_none_or(|entry| entry.word != source.word)
            })
        {
            return Err(DictionaryValidationError::InvalidLujvoIndexEntry {
                index,
                reason: "decomposition source does not match its dictionary entry",
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
        assert_eq!(normalize_lookup_query("da\u{02d2}oi"), "da'oi");
        assert_eq!(normalize_lookup_query("da\u{a78b}oi"), "da'oi");
        assert_eq!(normalize_lookup_query("dahoi"), "da'oi");
        assert_eq!(normalize_lookup_query("lo  brodá"), "lo broda");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn the_plain_lojban_short_path_gives_the_converter_key() {
        // The short path of `normalize_lookup_query` must give the key of the
        // full converter path for every text that it takes.
        let full = |text: &str| {
            key_text_of(
                &build_lookup_key::<()>(&[LookupPatternPart::Literal(text)], LookupKeyMode::Index)
                    .expect("an index key"),
            )
        };
        for text in [
            "klama",
            "co'i",
            "dat,ni",
            ".bad.",
            "mi  klama",
            " lo nu ",
            "ma,u",
            ", , ,",
            "pa re ci 123",
            "la.djan.",
            "\tmi\nklama ",
            "",
        ] {
            assert_eq!(plain_lojban_key(text), Some(full(text)), "{text:?}");
            assert_eq!(normalize_lookup_query(text), full(text), "{text:?}");
        }
        // Text with `h`, `q`, `w`, a capital or any other character takes the
        // full path.
        for text in ["coha", "klaqma", "wa", "Klama", "bárda", "клама", "ma;u"] {
            assert_eq!(plain_lojban_key(text), None, "{text:?}");
            assert_eq!(normalize_lookup_query(text), full(text), "{text:?}");
        }
        // Every headword of a small dictionary keys alike on both paths.
        for word in ["broda", "brode", "lo'e", "nu'e", "zo'u"] {
            assert_eq!(normalize_lookup_query(word), full(word), "{word:?}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_prefix_key_allows_a_last_unit_that_waits_for_a_partner() {
        let prefix = |text: &str| {
            LookupPrefix::from_typed(text).map(|prefix| {
                prefix
                    .readings()
                    .map(|reading| reading.as_str().to_owned())
                    .collect::<Vec<_>>()
            })
        };
        // A bare shorthand mark waits for its payload. An exact lookup
        // rejects it, and a prefix keys the text before it.
        assert_eq!(prefix("\u{ed8b}"), Ok(vec![String::new()]));
        assert_eq!(prefix("co\u{ed8b}"), Ok(vec!["co".to_owned()]));
        assert_eq!(
            LookupKey::from_typed("\u{ed8b}").map_err(|error| error.invalid_character()),
            Err(Some('\u{ed8b}'))
        );
        // A Cyrillic vowel before a breve has two readings: a vowel with an
        // implicit apostrophe, and a glide.
        for text in ["шои", "ШОИ"] {
            assert_eq!(
                prefix(text),
                Ok(vec!["co'i".to_owned(), "coi".to_owned()]),
                "{text}"
            );
        }
        assert_eq!(prefix("шои\u{0306}"), Ok(vec!["coi".to_owned()]));
        // `шу` reads `cu` with or without the breve, and a Latin vowel keys
        // alike with or without it, so each has one reading.
        assert_eq!(prefix("шу"), Ok(vec!["cu".to_owned()]));
        assert_eq!(prefix("coi"), Ok(vec!["coi".to_owned()]));
        // A base letter before its combining mark, or before a stress mark,
        // is a whole unit.
        assert_eq!(prefix("kla"), Ok(vec!["kla".to_owned()]));
        // Any other rejected character is still an error.
        for (text, character) in [("klâ", 'â'), ("kla#", '#'), ("q", 'q')] {
            assert_eq!(
                LookupPrefix::from_typed(text).map_err(|error| error.invalid_character()),
                Err(Some(character)),
                "{text}"
            );
        }
        // A prefix search returns the union of both readings, with no repeat,
        // and nothing that starts with `co` but continues neither reading.
        let entries = &[
            test_entry("co'i", WordType::Cmavo, &[], None),
            test_entry("coi", WordType::Cmavo, &[], None),
            test_entry("coico'u", WordType::Cmavo, &[], None),
            test_entry("cortu", WordType::Gismu, &[], None),
        ];
        let indexes = build_owned_indexes(entries);
        let dictionary = Dictionary::from_static_slices(
            entries,
            leak_word_index(&indexes.word_index),
            leak_rafsi_index(&indexes.rafsi_index),
            leak_pattern_index(&indexes.pattern_index),
            &[],
            &[],
            &[],
            0,
        );
        let found = |text: &str| {
            dictionary
                .entries_by_word_prefix(&LookupPrefix::from_typed(text).expect("prefix"))
                .map(|entry| entry.word)
                .collect::<Vec<_>>()
        };
        assert_eq!(found("шои"), ["co'i", "coi", "coico'u"]);
        assert_eq!(found("co"), ["co'i", "coi", "coico'u", "cortu"]);
        assert_eq!(found("coi"), ["coi", "coico'u"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_typed_key_names_what_a_stored_key_leaves_out() {
        // The owner rule of #1130: a typed lookup with a non-stress diacritic
        // is an invalid character that names it, while the stored headword
        // keeps its base letter.
        for (text, character, stored) in [
            ("klâma", 'â', "klama"),
            ("çmene", 'ç', "cmene"),
            ("klama#", '#', "klama"),
            ("klaqma", 'q', "klama"),
        ] {
            assert_eq!(
                LookupKey::from_typed(text).map_err(|error| error.invalid_character()),
                Err(Some(character)),
                "{text}"
            );
            assert_eq!(LookupKey::from_stored(text).as_str(), stored, "{text}");
        }
        for (text, key) in [
            ("klama", "klama"),
            ("Klámà", "klama"),
            ("клама", "klama"),
            ("co'i", "co'i"),
            ("ko·ri", "kori"),
            ("ko\u{2027}ri", "kori"),
            ("broda-zei-brode", "broda zei brode"),
            ("", ""),
        ] {
            assert_eq!(
                LookupKey::from_typed(text).as_ref().map(LookupKey::as_str),
                Ok(key),
                "{text}"
            );
            assert_eq!(LookupKey::from_stored(text).as_str(), key, "{text}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pattern_lookup_key_uses_apostrophe_spelling() {
        assert_eq!(normalize_lookup_query("daʼoi"), "da'oi");
        assert_eq!(normalize_lookup_query("da\u{2019}oi"), "da'oi");
        assert_eq!(normalize_lookup_query("dahoi"), "da'oi");
        assert_eq!(normalize_lookup_query("da\u{02d2}oi"), "da'oi");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lookup_pattern_uses_the_exact_key_rules() {
        #[requires(true)]
        #[ensures(true)]
        fn whole(text: &str) -> Result<Vec<LookupKeyPart<char>>, LookupPatternError> {
            normalize_lookup_pattern(&[LookupPatternPart::Literal(text)])
        }

        #[requires(true)]
        #[ensures(true)]
        fn text_of(text: &str) -> String {
            key_text_of(&whole(text).expect("the sample keeps a lookup character"))
        }

        #[requires(true)]
        #[ensures(true)]
        fn invalid(text: &str) -> Option<char> {
            whole(text)
                .expect_err("the sample is rejected")
                .invalid_character()
        }

        assert_eq!(text_of(" .Án,iis. "), normalize_lookup_query(" .Án,iis. "));
        assert_eq!(text_of(" .Án,iis. "), "aniis");
        assert_eq!(text_of("lo  brodá"), "lo broda");
        assert_eq!(text_of("клама"), "klama");
        // No converter reads `q`, so a lookup names it, as gentufa does.
        assert_eq!(
            whole("qá").map_err(|error| error.invalid_character()),
            Err(Some('q'))
        );
        assert_eq!(text_of("\u{ed8b}\u{eda2}"), "i'");
        // A word break is a space. A comma, U+00B7 and U+2027 disappear.
        assert_eq!(text_of("broda-zei-brode"), "broda zei brode");
        assert_eq!(text_of("ma;u"), "ma u");
        assert_eq!(text_of("xa!"), "xa");
        assert_eq!(text_of("mi\u{00b7}do"), "mido");
        assert_eq!(text_of("mi\u{2027}do"), "mido");
        assert_eq!(text_of("mi,do"), "mido");
        // Gentufa rejects these characters, so a lookup does too.
        assert_eq!(invalid("клама#da"), Some('#'));
        assert_eq!(invalid("coһi#"), Some('#'));
        assert_eq!(invalid("klâma"), Some('â'));
        assert_eq!(invalid("klama\u{0302}"), Some('\u{0302}'));
        assert_eq!(invalid("kläm"), Some('ä'));
        assert_eq!(invalid("кламя"), Some('я'));
        assert_eq!(invalid("mi\u{2022}do"), Some('\u{2022}'));
        assert_eq!(invalid("mi\u{2219}do"), Some('\u{2219}'));
        assert_eq!(invalid("\u{0306}"), Some('\u{0306}'));
        assert_eq!(invalid(",\u{0306}"), Some('\u{0306}'));
        assert_eq!(invalid("🙂"), Some('🙂'));
        // A run that keeps no letter names its text.
        assert_eq!(
            whole(", ,").expect_err("no letter"),
            new!(LookupPatternError::NoLookupCharacter {
                text: ", ,".to_owned()
            })
        );
        // The index key leaves a rejected character out. `â` is `a` and a
        // combining circumflex, so only the mark goes.
        assert_eq!(normalize_lookup_query("klâma"), "klama");
        assert_eq!(normalize_lookup_query("kla\u{0302}ma"), "klama");
        assert_eq!(normalize_lookup_query("ma;u"), "ma u");
        assert!(whole(" ").expect("blank").is_empty());

        let split = normalize_lookup_pattern(&[
            LookupPatternPart::Literal("ca"),
            LookupPatternPart::Literal(","),
        ]);
        assert_eq!(split, whole("ca,"));
        assert_eq!(text_of("ca,"), "ca");

        let comma_before_star = normalize_lookup_pattern(&[
            LookupPatternPart::Literal("ca ,"),
            LookupPatternPart::Wildcard('*'),
        ])
        .expect("space stays before the star");
        assert_eq!(key_text_of(&comma_before_star), "ca ");
        assert_eq!(
            comma_before_star,
            vec![
                new!(LookupKeyPart::Text("ca ".to_owned())),
                new!(LookupKeyPart::Wildcard('*'))
            ]
        );

        let bang_before_star = normalize_lookup_pattern(&[
            LookupPatternPart::Literal("ca !"),
            LookupPatternPart::Wildcard('*'),
        ])
        .expect("a bang does not hide the space");
        assert_eq!(key_text_of(&bang_before_star), "ca ");

        let comma_between = normalize_lookup_pattern(&[
            LookupPatternPart::Literal("ca ,"),
            LookupPatternPart::Wildcard('?'),
            LookupPatternPart::Literal("ma"),
        ])
        .expect("comma does not hide the space");
        assert_eq!(key_text_of(&comma_between), "ca ma");

        let stars = normalize_lookup_pattern(&[
            LookupPatternPart::Wildcard('*'),
            LookupPatternPart::Literal(" "),
            LookupPatternPart::Wildcard('*'),
        ])
        .expect("space between stars");
        assert_eq!(key_text_of(&stars), " ");
        assert_eq!(stars.len(), 3);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_decomposition_never_makes_a_word_break_in_a_key() {
        // U+037E decomposes to `;`, and U+226E and U+226F to `<` or `>` and a
        // mark. A word break is decided from the typed character, so none of
        // them is a space in the index key, in an exact lookup or in a glob.
        for value in ['\u{037e}', '\u{226e}', '\u{226f}', '\u{0387}'] {
            let text = format!("abu{value}sance");
            assert!(!normalize_lookup_query(&text).contains(' '), "{value:?}");
            assert_eq!(
                normalize_lookup_pattern::<char>(&[LookupPatternPart::Literal(&text)])
                    .expect_err("gentufa rejects the character")
                    .invalid_character(),
                Some(value),
                "{value:?}"
            );
            assert!(
                normalize_lookup_pattern(&[
                    LookupPatternPart::Literal(&text),
                    LookupPatternPart::Wildcard('*'),
                ])
                .is_err(),
                "{value:?}"
            );
        }
        // Canonically equivalent headwords get one index key, also where a
        // lookup rejects them.
        assert_eq!(
            normalize_lookup_query("аи\u{0327}\u{0306}"),
            normalize_lookup_query("аи\u{0306}\u{0327}")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_word_break_between_wildcards_is_a_space() {
        #[requires(true)]
        #[ensures(true)]
        fn between(text: &str) -> Result<Vec<LookupKeyPart<char>>, LookupPatternError> {
            normalize_lookup_pattern(&[
                LookupPatternPart::Wildcard('*'),
                LookupPatternPart::Literal(text),
                LookupPatternPart::Wildcard('*'),
            ])
        }

        let space = between(" ").expect("a space between stars");
        let mut breaks = 0;
        let mut buffer = [0; 4];
        for code in 0..=u32::from(char::MAX) {
            let Some(value) = char::from_u32(code) else {
                continue;
            };
            // The converter is the authority on word breaks, as for the key.
            let mut steps = Vec::new();
            transcribe_lookup_run(value.encode_utf8(&mut buffer), |step| steps.push(step));
            if !matches!(steps.as_slice(), [step] if matches!(step.as_data(), data!(LookupStep::WordBreak(_))))
            {
                continue;
            }
            assert_eq!(between(&value.to_string()), Ok(space.clone()), "{value:?}");
            breaks += 1;
        }
        assert!(breaks > 30, "{breaks}");
        // A removed mark next to a break leaves the space.
        for text in [" , ", ",!", "!,", "\u{00b7}-", "\u{2027} ", ". \u{00b7}"] {
            assert_eq!(between(text), Ok(space.clone()), "{text:?}");
        }
        // A run that adds nothing to the key is still an error.
        for text in [",", "\u{00b7}", "\u{2027}", ",,"] {
            assert_eq!(
                between(text),
                Err(new!(LookupPatternError::NoLookupCharacter {
                    text: text.to_owned()
                })),
                "{text:?}"
            );
        }
        for parts in [
            [
                LookupPatternPart::Literal("!"),
                LookupPatternPart::Wildcard('*'),
            ],
            [
                LookupPatternPart::Wildcard('*'),
                LookupPatternPart::Literal("!"),
            ],
        ] {
            assert!(normalize_lookup_pattern(&parts).is_err(), "{parts:?}");
        }
        // A letter run keeps its break as before.
        assert_eq!(
            key_text_of(
                &normalize_lookup_pattern(&[
                    LookupPatternPart::Literal("ca"),
                    LookupPatternPart::Wildcard('*'),
                    LookupPatternPart::Literal("!"),
                    LookupPatternPart::Wildcard('*'),
                ])
                .expect("ca *!*")
            ),
            "ca "
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn adjacent_literals_share_one_lookup_stream() {
        #[requires(true)]
        #[ensures(true)]
        fn whole(text: &str) -> Result<Vec<LookupKeyPart<char>>, LookupPatternError> {
            normalize_lookup_pattern(&[LookupPatternPart::Literal(text)])
        }

        let cyrillic = normalize_lookup_pattern(&[
            LookupPatternPart::Literal("шо"),
            LookupPatternPart::Literal("и"),
        ]);
        assert_eq!(cyrillic, whole("шои"));
        assert_eq!(
            key_text_of(&cyrillic.expect("split Cyrillic keeps a letter")),
            "co'i"
        );

        let with_empty = normalize_lookup_pattern(&[
            LookupPatternPart::Literal("шо"),
            LookupPatternPart::Literal(""),
            LookupPatternPart::Literal("и"),
        ]);
        assert_eq!(with_empty, whole("шои"));

        let three = normalize_lookup_pattern(&[
            LookupPatternPart::Literal("ш"),
            LookupPatternPart::Literal("о"),
            LookupPatternPart::Literal("и"),
        ]);
        assert_eq!(three, whole("шои"));

        let shorthand = normalize_lookup_pattern(&[
            LookupPatternPart::Literal("\u{ed8b}"),
            LookupPatternPart::Literal("\u{eda2}"),
        ]);
        assert_eq!(shorthand, whole("\u{ed8b}\u{eda2}"));
        assert_eq!(
            key_text_of(&shorthand.expect("split shorthand keeps a letter")),
            "i'"
        );

        let shorthand_with_empty = normalize_lookup_pattern(&[
            LookupPatternPart::Literal("\u{ed8b}"),
            LookupPatternPart::Literal(""),
            LookupPatternPart::Literal("\u{eda2}"),
        ]);
        assert_eq!(shorthand_with_empty, whole("\u{ed8b}\u{eda2}"));

        // A shorthand mark with no payload is a character that gentufa
        // rejects, so the key rejects it.
        let bare_mark = normalize_lookup_pattern(&[
            LookupPatternPart::Literal("\u{ed8b}"),
            LookupPatternPart::Literal("a"),
        ]);
        assert_eq!(bare_mark, whole("\u{ed8b}a"));
        assert_eq!(
            bare_mark.expect_err("a bare mark").invalid_character(),
            Some('\u{ed8b}')
        );

        let across_star = normalize_lookup_pattern(&[
            LookupPatternPart::Literal("шо"),
            LookupPatternPart::Wildcard('*'),
            LookupPatternPart::Literal("и"),
        ])
        .expect("a wildcard starts a new stream");
        assert_eq!(
            across_star,
            vec![
                new!(LookupKeyPart::Text("co".to_owned())),
                new!(LookupKeyPart::Wildcard('*')),
                new!(LookupKeyPart::Text("i".to_owned())),
            ]
        );

        assert_eq!(
            normalize_lookup_pattern(&[
                LookupPatternPart::Literal("\u{ed8b}"),
                LookupPatternPart::Wildcard('*'),
            ])
            .expect_err("a bare shorthand mark keeps no letter")
            .invalid_character(),
            Some('\u{ed8b}')
        );

        let mark_before_star = normalize_lookup_pattern(&[
            LookupPatternPart::Literal("a"),
            LookupPatternPart::Literal("\u{ed8b}"),
            LookupPatternPart::Wildcard('*'),
            LookupPatternPart::Literal("\u{eda2}"),
        ]);
        // A wildcard ends the run, so the mark has no payload.
        assert_eq!(
            mark_before_star
                .expect_err("a bare mark before a wildcard")
                .invalid_character(),
            Some('\u{ed8b}')
        );
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
    fn cyrillic_fuivla_supplies_a_normalized_stem_key() {
        let word = "иерхи";
        let entries = [test_entry(word, WordType::Fuivla, &[], None)];
        let indexes = build_owned_indexes(&entries);
        let stem = fuivla_stem_rafsi(word).expect("Cyrillic spelling has a stem");
        assert_eq!(stem, "i'erx");
        let key = normalize_lookup_query(&stem);
        assert_eq!(indexes.rafsi_index[0].key, key);
        // A derived stem is not a wildcard rafsi key.
        assert!(indexes.pattern_index[0].rafsi_keys.is_empty());
        assert_eq!(
            indexes.rafsi_index[0].targets[0].source,
            RafsiSource::FuivlaStem
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn derived_fuivla_stems_pass_morphology_and_do_not_claim_rafsi() {
        let entries = &[
            test_entry("ierxi", WordType::Fuivla, &[], None),
            test_entry("tci'ile", WordType::ObsoleteFuivla, &[], None),
            test_entry("spageti", WordType::Fuivla, &[], None),
        ];
        let indexes = build_owned_indexes(entries);
        let dictionary = Dictionary::from_static_slices(
            entries,
            leak_word_index(&indexes.word_index),
            leak_rafsi_index(&indexes.rafsi_index),
            leak_pattern_index(&indexes.pattern_index),
            &[],
            &[],
            &[],
            0,
        );
        dictionary.validate().expect("consistent indexes");
        for (stem, word) in [("ierx", "ierxi"), ("tci'il", "tci'ile")] {
            let matched = dictionary
                .lookup_rafsi(&LookupKey::from_stored(stem))
                .next()
                .expect(stem);
            assert_eq!(matched.source, RafsiSource::FuivlaStem);
            assert_eq!(matched.entry.word, word);
            assert!(dictionary.rafsi_availability(stem).is_free());
        }
        assert!(fuivla_stem_rafsi("\u{0301},").is_none());
        // These obsolete headwords split into several words, so no lujvo can
        // start with their stems.
        for word in [
            "aierne",
            "matriocka",
            "patriarko",
            "reskuamata",
            "restuatara",
        ] {
            assert!(fuivla_stem_rafsi(word).is_none(), "{word}");
        }
        assert_eq!(fuivla_stem_rafsi("ierxi").as_deref(), Some("ierx"));
        // CLL 4.16: an apostrophe before the final vowel goes with it, and a
        // diphthong-final fu'ivla has no short form.
        assert_eq!(fuivla_stem_rafsi("actce'o").as_deref(), Some("actce"));
        assert_eq!(fuivla_stem_rafsi("enri").as_deref(), Some("enr"));
        assert_eq!(fuivla_stem_rafsi("adzau"), None);
        // The morphology's reading decides the final nucleus. A glide before
        // the final vowel is not part of it.
        for (word, stem) in [
            ("barbekeiu", "barbekei"),
            ("aldeia", "aldei"),
            ("alminiu", "almini"),
            ("amnio", "amni"),
            ("anmonia", "anmoni"),
        ] {
            assert_eq!(fuivla_stem_rafsi(word).as_deref(), Some(stem), "{word}");
        }
        assert!(
            dictionary
                .lookup_rafsi(&LookupKey::from_stored("spaget"))
                .next()
                .is_none()
        );
        assert!(dictionary.rafsi_availability("spaget").is_free());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_short_form_is_refused_only_when_it_adds_the_cgv_warning() {
        // Owner rule 3, as refined: `bolvii'y` has the experimental CgV
        // warning and `bolviia'y` does not, so `bolviia` has no short form.
        assert!(short_form_adds_experimental_cgv("bolvii", "bolviia"));
        assert_eq!(fuivla_stem_rafsi("bolviia"), None);
        // `abniena` has the warning itself, so `abniena'y` has it too. The
        // short form adds nothing, and `abniena` keeps `abnien`.
        assert!(compound_has_experimental_cgv("abnienykla"));
        assert!(compound_has_experimental_cgv("abniena'ykla"));
        assert_eq!(fuivla_stem_rafsi("abniena").as_deref(), Some("abnien"));
        // `anmonia` has the warning, and its short form loses it.
        assert!(compound_has_experimental_cgv("anmonia'ykla"));
        assert!(!compound_has_experimental_cgv("anmoni'ykla"));
        assert_eq!(fuivla_stem_rafsi("anmonia").as_deref(), Some("anmoni"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn validation_rejects_an_ordinary_rafsi_segment_with_several_sources() {
        let entries = [
            test_entry("ierxi", WordType::Fuivla, &[], None),
            test_entry("ierxa", WordType::Fuivla, &[], None),
            test_entry("klama", WordType::Gismu, &[], None),
            test_entry("ierxykla", WordType::Lujvo, &[], None),
        ];
        let fuivla_sources = [
            DictionaryLujvoSource {
                entry_index: EntryIndex(0),
                word: "ierxi",
            },
            DictionaryLujvoSource {
                entry_index: EntryIndex(1),
                word: "ierxa",
            },
        ];
        let klama_source = [DictionaryLujvoSource {
            entry_index: EntryIndex(2),
            word: "klama",
        }];
        let source_words = ["ierxi", "ierxa", "klama"];
        let shared = [
            DictionaryLujvoSegment {
                kind: DictionaryLujvoSegmentKind::FuivlaRafsi,
                surface: "ierx",
                sources: &fuivla_sources,
            },
            DictionaryLujvoSegment {
                kind: DictionaryLujvoSegmentKind::Rafsi,
                surface: "kla",
                sources: &klama_source,
            },
        ];
        let accepted = [DictionaryLujvoEntry {
            entry_index: EntryIndex(3),
            segments: &shared,
            source_words: &source_words,
        }];
        assert!(validate_lujvo_index(&entries, &accepted).is_ok());

        let ordinary = [
            DictionaryLujvoSegment {
                kind: DictionaryLujvoSegmentKind::Rafsi,
                surface: "ierx",
                sources: &fuivla_sources,
            },
            DictionaryLujvoSegment {
                kind: DictionaryLujvoSegmentKind::Rafsi,
                surface: "kla",
                sources: &klama_source,
            },
        ];
        let rejected = [DictionaryLujvoEntry {
            entry_index: EntryIndex(3),
            segments: &ordinary,
            source_words: &source_words,
        }];
        assert_eq!(
            validate_lujvo_index(&entries, &rejected),
            Err(DictionaryValidationError::InvalidLujvoIndexEntry {
                index: 0,
                reason: "ordinary rafsi segment has more than one source",
            })
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn short_rafsi_candidates_report_claims_from_every_word_type() {
        static SAL: [Rafsi<'static>; 1] = [official("sal")];
        static SAK: [Rafsi<'static>; 1] = [experimental("sak")];
        static SKA: [Rafsi<'static>; 1] = [official("ska")];
        static SKA_EXPERIMENTAL: [Rafsi<'static>; 1] = [experimental("ska")];
        static KLI: [Rafsi<'static>; 1] = [official("kli")];
        static SAI: [Rafsi<'static>; 1] = [official("sai")];
        static SAHI: [Rafsi<'static>; 1] = [experimental("sa'i")];
        static KAM: [Rafsi<'static>; 1] = [official("kam")];
        // Synthetic assignments, but each mirrors a real dictionary shape: the
        // cmavo `ka` really does hold `kam` (CLL 4.6), and fu'ivla, obsolete
        // and experimental entries all appear in the rafsi index.
        let entries = &[
            test_entry("salci", WordType::Gismu, &SAL, None),
            test_entry("salpo", WordType::Gismu, &SAL, None),
            test_entry("sakta", WordType::ExperimentalGismu, &SAK, None),
            test_entry("skami", WordType::Gismu, &SKA, None),
            test_entry(
                "skeci",
                WordType::ExperimentalGismu,
                &SKA_EXPERIMENTAL,
                None,
            ),
            test_entry("kliniko", WordType::Fuivla, &KLI, None),
            test_entry("sa'e", WordType::Cmavo, &SAI, None),
            test_entry("xua'ai", WordType::ObsoleteCmavo, &SAHI, None),
            test_entry("ka", WordType::Cmavo, &KAM, None),
        ];
        let indexes = build_owned_indexes(entries);
        let dictionary = Dictionary::from_static_slices(
            entries,
            leak_word_index(&indexes.word_index),
            leak_rafsi_index(&indexes.rafsi_index),
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
            dictionary
                .lookup_rafsi(&LookupKey::from_stored("ska"))
                .map(|matched| (matched.entry.word, matched.entry.word_type))
                .collect::<Vec<_>>(),
            vec![
                ("skami", WordType::Gismu),
                ("skeci", WordType::ExperimentalGismu)
            ]
        );
        assert_eq!(
            dictionary
                .lookup_rafsi(&LookupKey::from_stored("kli"))
                .map(|matched| (matched.entry.word, matched.entry.word_type))
                .collect::<Vec<_>>(),
            vec![("kliniko", WordType::Fuivla)]
        );
        // Universal forms share the rafsi index but cannot spell a short
        // rafsi, so availability never has to filter them out on content.
        assert_eq!(
            dictionary
                .lookup_rafsi(&LookupKey::from_stored("salc"))
                .map(|matched| (matched.entry.word, matched.entry.word_type))
                .collect::<Vec<_>>(),
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
    fn max_rafsi_standing_covers_every_word_type() {
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
            .map(WordType::max_rafsi_standing),
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
        let dictionary = Dictionary::from_static_slices(
            entries,
            word_index,
            rafsi_index,
            leak_pattern_index(&indexes.pattern_index),
            &[],
            &[],
            &[],
            0,
        );

        assert!(dictionary.validate().is_ok());
        assert_eq!(
            dictionary
                .lookup_words(&LookupKey::from_stored("internet"))
                .map(|entry| entry.word)
                .collect::<Vec<_>>(),
            vec!["INternet", "internet"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shared_lookup_key_ranks_the_higher_score_first() {
        static LOW_GLOSS: [Keyword<'static>; 1] = [Keyword {
            word: "justification",
            meaning: None,
        }];
        static HIGH_GLOSS: [Keyword<'static>; 1] = [Keyword {
            word: "reason",
            meaning: None,
        }];
        let mut worse = test_entry("gaurkihu", WordType::Lujvo, &[], None);
        worse.score = Score(-2.0);
        worse.gloss_keywords = &LOW_GLOSS;
        let mut better = test_entry("gaurki'u", WordType::Lujvo, &[], None);
        better.score = Score(1.0);
        better.gloss_keywords = &HIGH_GLOSS;
        let entries = &[worse, better];
        let indexes = build_owned_indexes(entries);
        let dictionary = Dictionary::from_static_slices(
            entries,
            leak_word_index(&indexes.word_index),
            leak_rafsi_index(&indexes.rafsi_index),
            leak_pattern_index(&indexes.pattern_index),
            &[],
            &[],
            &[],
            0,
        );

        assert!(dictionary.validate().is_ok());
        assert_eq!(
            dictionary
                .lookup_words(&LookupKey::from_stored("gaurkihu"))
                .map(|entry| entry.word)
                .collect::<Vec<_>>(),
            vec!["gaurki'u", "gaurkihu"]
        );
        assert_eq!(
            dictionary
                .lookup_words(&LookupKey::from_stored("gaurki\u{02bc}u"))
                .map(|entry| (entry.word, entry.score.get()))
                .collect::<Vec<_>>(),
            vec![("gaurki'u", 1.0), ("gaurkihu", -2.0)]
        );
        assert_eq!(
            dictionary
                .lookup_word(&LookupKey::from_stored("gaurki'u"))
                .unwrap()
                .word,
            "gaurki'u"
        );
        assert_eq!(
            dictionary
                .entries_by_word_prefix(&LookupPrefix::from_typed("gaurki").expect("prefix"))
                .map(|entry| entry.word)
                .collect::<Vec<_>>(),
            vec!["gaurki'u", "gaurkihu"]
        );
        assert_eq!(
            dictionary.first_gloss_keywords_for_words(&[&LookupKey::from_stored("gaurkihu")]),
            vec![Some("reason")]
        );
        assert_eq!(
            dictionary
                .exact_definition_indices(&LookupKey::from_stored("gaurki'u"))
                .iter()
                .map(|index| dictionary.entry_at(*index).word)
                .collect::<Vec<_>>(),
            vec!["gaurki'u", "gaurkihu"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shared_key_tie_keeps_source_order_after_the_higher_score() {
        let mut low = test_entry("gaurkihu", WordType::Lujvo, &[], None);
        low.score = Score(-2.0);
        low.definition_id = DefinitionId(11);
        let mut high_first = test_entry("gaurki'u", WordType::Lujvo, &[], None);
        high_first.score = Score(1.0);
        high_first.definition_id = DefinitionId(12);
        let mut high_second = test_entry("gaurki\u{02bc}u", WordType::Lujvo, &[], None);
        high_second.score = Score(1.0);
        high_second.definition_id = DefinitionId(13);
        let entries = &[low, high_first, high_second];
        let indexes = build_owned_indexes(entries);
        assert_eq!(indexes.word_index.len(), 1);
        assert_eq!(
            indexes.word_index[0].targets,
            [EntryIndex(0), EntryIndex(1), EntryIndex(2)]
        );
        let dictionary = Dictionary::from_static_slices(
            entries,
            leak_word_index(&indexes.word_index),
            leak_rafsi_index(&indexes.rafsi_index),
            leak_pattern_index(&indexes.pattern_index),
            &[],
            &[],
            &[],
            0,
        );

        assert!(dictionary.validate().is_ok());
        assert_eq!(
            dictionary.exact_definition_indices(&LookupKey::from_stored("gaurkihu")),
            [EntryIndex(1), EntryIndex(2), EntryIndex(0)]
        );
        assert_eq!(
            dictionary
                .lookup_words(&LookupKey::from_stored("gaurki'u"))
                .map(|entry| entry.word)
                .collect::<Vec<_>>(),
            vec!["gaurki'u", "gaurki\u{02bc}u", "gaurkihu"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn lookup_cmavo_sequence_ranks_a_shared_key_and_keeps_stored_targets() {
        let mut worse = test_entry("nahe bo", WordType::CmavoCompound, &[], None);
        worse.score = Score(-2.0);
        worse.definition_id = DefinitionId(21);
        let mut better = test_entry("na'e bo", WordType::CmavoCompound, &[], None);
        better.score = Score(3.0);
        better.definition_id = DefinitionId(22);
        let entries = &[worse, better];
        let indexes = build_owned_indexes(entries);
        assert_eq!(indexes.cmavo_sequence_index.len(), 1);
        assert_eq!(
            indexes.cmavo_sequence_index[0].targets,
            [EntryIndex(0), EntryIndex(1)]
        );
        let components: Vec<&str> = indexes.cmavo_sequence_index[0]
            .components
            .iter()
            .map(String::as_str)
            .collect();
        let component_rows = [components.as_slice()];
        let rows = [CmavoSequenceIndexEntry::from_static_parts(
            component_rows[0],
            &indexes.cmavo_sequence_index[0].targets,
        )];
        let dictionary = Dictionary::from_static_slices(
            entries,
            leak_word_index(&indexes.word_index),
            leak_rafsi_index(&indexes.rafsi_index),
            leak_pattern_index(&indexes.pattern_index),
            &[],
            &[],
            &rows,
            indexes.max_cmavo_sequence_len,
        );

        assert_eq!(dictionary.validate(), Ok(()));
        assert_eq!(
            dictionary.cmavo_sequence_index()[0].targets(),
            rows[0].targets()
        );
        assert_eq!(
            dictionary.lookup_cmavo_sequence(&components),
            [EntryIndex(1), EntryIndex(0)]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shared_rafsi_key_ranks_the_higher_score_first() {
        static TIED_LOW: [Rafsi<'static>; 1] = [experimental("gahu")];
        static TIED_HIGH_FIRST: [Rafsi<'static>; 1] = [experimental("ga'u")];
        static TIED_HIGH_SECOND: [Rafsi<'static>; 1] = [experimental("ga\u{02bc}u")];
        let mut worse = test_entry("gasny", WordType::Lujvo, &TIED_LOW, None);
        worse.score = Score(-2.0);
        let mut better_first = test_entry("ganzy", WordType::Lujvo, &TIED_HIGH_FIRST, None);
        better_first.score = Score(1.0);
        let mut better_second = test_entry("gancy", WordType::Lujvo, &TIED_HIGH_SECOND, None);
        better_second.score = Score(1.0);
        let tied = &[better_first, worse, better_second];
        let tied_indexes = build_owned_indexes(tied);
        let tied_dictionary = Dictionary::from_static_slices(
            tied,
            leak_word_index(&tied_indexes.word_index),
            leak_rafsi_index(&tied_indexes.rafsi_index),
            leak_pattern_index(&tied_indexes.pattern_index),
            &[],
            &[],
            &[],
            0,
        );
        assert!(tied_dictionary.validate().is_ok());
        assert_eq!(
            tied_dictionary
                .lookup_rafsi(&LookupKey::from_stored("gahu"))
                .map(|matched| matched.entry.word)
                .collect::<Vec<_>>(),
            vec!["ganzy", "gancy", "gasny"]
        );
        assert_eq!(
            tied_dictionary.rafsi_availability("ga'u").claimant_words(),
            vec!["ganzy".to_owned(), "gancy".to_owned(), "gasny".to_owned()]
        );

        static OFFICIAL_RAFSI: [Rafsi<'static>; 1] = [official("gau")];
        static EXPERIMENTAL_RAFSI: [Rafsi<'static>; 1] = [experimental("gau")];
        let mut official_entry = test_entry("offcl", WordType::Lujvo, &OFFICIAL_RAFSI, None);
        official_entry.score = Score(-2.0);
        let mut experimental_entry =
            test_entry("exprm", WordType::Fuivla, &EXPERIMENTAL_RAFSI, None);
        experimental_entry.score = Score(5.0);
        let mixed = &[official_entry, experimental_entry];
        let mixed_indexes = build_owned_indexes(mixed);
        let mixed_dictionary = Dictionary::from_static_slices(
            mixed,
            leak_word_index(&mixed_indexes.word_index),
            leak_rafsi_index(&mixed_indexes.rafsi_index),
            leak_pattern_index(&mixed_indexes.pattern_index),
            &[],
            &[],
            &[],
            0,
        );
        assert!(mixed_dictionary.validate().is_ok());
        assert_eq!(
            mixed_dictionary
                .lookup_rafsi(&LookupKey::from_stored("gau"))
                .map(|matched| (matched.entry.word, matched.entry.score.get()))
                .collect::<Vec<_>>(),
            vec![("exprm", 5.0), ("offcl", -2.0)]
        );
        let availability = mixed_dictionary.rafsi_availability("gau");
        assert_eq!(availability.claim_kind(), Some(RafsiClaimKind::Official));
        assert_eq!(availability.claimant_words(), vec!["offcl".to_owned()]);
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
            leak_pattern_index(&indexes.pattern_index),
            &[],
            &[],
            &[],
            0,
        );

        assert!(dictionary.validate().is_ok());
        assert_eq!(
            dictionary.first_gloss_keywords_for_words(&[
                &LookupKey::from_stored(".bárda."),
                &LookupKey::from_stored("missing"),
            ]),
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
            leak_pattern_index(&indexes.pattern_index),
            &[],
            &[],
            &[],
            0,
        );

        assert!(dictionary.validate().is_ok());
        assert_eq!(
            dictionary
                .entries_by_word_prefix(&LookupPrefix::from_typed("BÁ").expect("prefix"))
                .map(|entry| entry.word)
                .collect::<Vec<_>>(),
            vec!["BÁJRA", "bajra", "barda"],
        );
        assert!(
            dictionary
                .entries_by_word_prefix(&LookupPrefix::from_typed("bas").expect("prefix"))
                .next()
                .is_none(),
            "the partitioned range must stop before adjacent non-matches",
        );
        assert_eq!(
            dictionary
                .entries_by_word_prefix(&LookupPrefix::from_typed("").expect("prefix"))
                .count(),
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
        let dictionary = Dictionary::from_static_slices(
            entries,
            word_index,
            rafsi_index,
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
            0,
        );
        assert_eq!(
            incomplete.exact_definition_indices(&LookupKey::from_stored("pu ba")),
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
            dictionary.exact_definition_indices(&LookupKey::from_stored("pu ba")),
            [EntryIndex(1)]
        );
        assert!(
            dictionary
                .exact_definition_indices(&LookupKey::from_stored("klama zei tavla"))
                .is_empty()
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

    #[requires(!form.is_empty())]
    #[ensures(matches!(ret.standing, RafsiClaimKind::Official))]
    const fn official(form: &'static str) -> Rafsi<'static> {
        Rafsi {
            form,
            standing: RafsiClaimKind::Official,
        }
    }

    #[requires(!form.is_empty())]
    #[ensures(matches!(ret.standing, RafsiClaimKind::Experimental))]
    const fn experimental(form: &'static str) -> Rafsi<'static> {
        Rafsi {
            form,
            standing: RafsiClaimKind::Experimental,
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn validate_rejects_an_official_rafsi_above_the_word_type_ceiling() {
        // Synthetic: an obsolete cmavo can hold only experimental rafsi, so a
        // dictionary that gives it an official one is invalid, whoever built it.
        static OFFICIAL: [Rafsi<'static>; 1] = [official("xua")];
        let entries = &[test_entry(
            "xua'ai",
            WordType::ObsoleteCmavo,
            &OFFICIAL,
            None,
        )];
        let indexes = build_owned_indexes(entries);
        let dictionary = Dictionary::from_static_slices(
            entries,
            leak_word_index(&indexes.word_index),
            leak_rafsi_index(&indexes.rafsi_index),
            leak_pattern_index(&indexes.pattern_index),
            &[],
            &[],
            &[],
            0,
        );
        assert!(matches!(
            dictionary.validate(),
            Err(DictionaryValidationError::InvalidEntry { index: 0, reason })
                if reason == "an experimental-standing word lists an official rafsi"
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn claim_standing_comes_from_the_rafsi_not_the_word_type() {
        // Synthetic: an official cmavo holding an experimental rafsi, the
        // shape Lensisku's `maz` proposal has on a non-selected definition of
        // `ma`. The claim must stay experimental, so an official claimant of
        // the same form would still outrank it.
        static MAZ: [Rafsi<'static>; 1] = [experimental("maz")];
        let entries = &[test_entry("ma", WordType::Cmavo, &MAZ, None)];
        let indexes = build_owned_indexes(entries);
        let dictionary = Dictionary::from_static_slices(
            entries,
            leak_word_index(&indexes.word_index),
            leak_rafsi_index(&indexes.rafsi_index),
            leak_pattern_index(&indexes.pattern_index),
            &[],
            &[],
            &[],
            0,
        );
        let availability = dictionary.rafsi_availability("maz");
        assert_eq!(
            availability.claim_kind(),
            Some(RafsiClaimKind::Experimental)
        );
        assert_eq!(availability.claimant_words(), ["ma".to_owned()]);
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
