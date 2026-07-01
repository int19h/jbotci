//! Embedded Lensisku dictionary snapshots.

use bityzba::{invariant, requires};
use jbotci_dictionary::Dictionary;

/// Metadata for a vendored Lensisku dictionary snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub struct DictionarySnapshotMetadata {
    pub language_tag: &'static str,
    pub language_realname: &'static str,
    pub format: &'static str,
    pub filename: &'static str,
    pub metadata_url: &'static str,
    pub download_url: &'static str,
    pub lensisku_created_at: &'static str,
    pub sha256: &'static str,
    pub entry_count: usize,
}

include!(concat!(env!("OUT_DIR"), "/dictionary_en.rs"));

/// Return the embedded English Lensisku dictionary.
#[requires(true)]
#[ensures(true)]
pub fn english() -> &'static Dictionary<'static> {
    &ENGLISH
}

/// Return metadata for the embedded English Lensisku dictionary snapshot.
#[requires(true)]
#[ensures(ret.entry_count == ENGLISH.entries().len())]
pub fn english_metadata() -> &'static DictionarySnapshotMetadata {
    &ENGLISH_METADATA
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use bityzba::requires;
    use jbotci_dictionary::{
        DictionaryLujvoEntry, DictionaryLujvoSegmentKind, DictionarySoundEntry, RafsiSource,
    };
    use jbotci_phonetic::is_valid_ipa_segment_id;

    use super::*;

    const DICTIONARY_SOUND_INDEX_SKIPS: &str =
        include_str!("../tests/dictionary_sound_index_skips.tsv");

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedded_dictionary_validates() {
        ENGLISH.validate().expect("embedded dictionary is valid");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedded_metadata_matches_dictionary() {
        assert_eq!(english_metadata().entry_count, english().entries().len());
        assert_eq!(
            english_metadata().lensisku_created_at,
            "2026-05-23T00:00:42.298977Z"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn known_entry_is_present() {
        let entry = english().lookup_word("a").expect("entry for a");
        assert_eq!(entry.word, "a");
        assert_eq!(entry.definition_id.get(), 1339);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn blank_selmaho_fields_are_absent() {
        let entry = english().lookup_word("brode").expect("entry for brode");
        assert_eq!(entry.selmaho, None);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedded_sound_index_is_sorted_and_tokenized() {
        let mut previous_index = None;
        for sound_entry in english().sound_index() {
            assert!(
                previous_index.is_none_or(|previous| sound_entry.entry_index.get() > previous),
                "sound index entry order regressed at {:?}",
                sound_entry.entry_index
            );
            assert!(!sound_entry.ipa.trim().is_empty());
            assert!(!sound_entry.token_sequence.segments.is_empty());
            assert!(sound_entry.token_sequence.self_similarity.is_finite());
            assert!(sound_entry.token_sequence.self_similarity > 0.0);
            assert!(
                sound_entry
                    .token_sequence
                    .segments
                    .iter()
                    .all(|segment| is_valid_ipa_segment_id(*segment))
            );
            previous_index = Some(sound_entry.entry_index.get());
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedded_sound_index_contains_known_standard_ipa() {
        let klama = sound_entry_for_word("klama").expect("sound entry for klama");
        assert_eq!(klama.ipa, "ˈkla.ma");
        assert_eq!(klama.token_sequence.segment_count(), 5);

        let coi = sound_entry_for_word("coi").expect("sound entry for coi");
        assert_eq!(coi.ipa, "ʃoj");
        assert_eq!(coi.token_sequence.segment_count(), 3);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedded_sound_index_skips_match_expected_list() {
        let indexed_entries = english()
            .sound_index()
            .iter()
            .map(|entry| entry.entry_index.get())
            .collect::<BTreeSet<_>>();
        let actual = english()
            .entries()
            .iter()
            .enumerate()
            .filter(|(index, _)| !indexed_entries.contains(index))
            .map(|(_, entry)| format!("{}\t{}", entry.word, entry.word_type.as_str()))
            .collect::<Vec<_>>();
        let expected = expected_sound_index_skips();

        assert_eq!(
            actual, expected,
            "sound-index skip list changed; review standard IPA preprocessing failures"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedded_lujvo_index_is_sorted_and_structural() {
        let mut previous_index = None;
        for lujvo_entry in english().lujvo_index() {
            assert!(
                previous_index.is_none_or(|previous| lujvo_entry.entry_index.get() > previous),
                "lujvo index entry order regressed at {:?}",
                lujvo_entry.entry_index
            );
            let entry = &english().entries()[lujvo_entry.entry_index.get()];
            assert!(entry.word_type.is_lujvo_like());
            assert!(!lujvo_entry.segments.is_empty());
            assert!(
                lujvo_entry
                    .segments
                    .iter()
                    .filter(|segment| segment.kind == DictionaryLujvoSegmentKind::Rafsi)
                    .count()
                    >= 2
            );
            assert!(
                lujvo_entry
                    .segments
                    .iter()
                    .all(|segment| !segment.surface.is_empty())
            );
            assert!(lujvo_entry.segments.iter().all(|segment| {
                segment.kind == DictionaryLujvoSegmentKind::Rafsi || segment.source_word.is_none()
            }));
            previous_index = Some(lujvo_entry.entry_index.get());
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedded_lujvo_index_contains_known_decomposition() {
        let jbobau = lujvo_entry_for_word("jbobau").expect("lujvo entry for jbobau");
        let segments = jbobau
            .segments
            .iter()
            .map(|segment| (segment.kind, segment.surface, segment.source_word))
            .collect::<Vec<_>>();

        assert_eq!(
            segments,
            [
                (DictionaryLujvoSegmentKind::Rafsi, "jbó", Some("lojbo")),
                (DictionaryLujvoSegmentKind::Rafsi, "baŭ", Some("bangu")),
            ]
        );
        assert_eq!(jbobau.source_words, ["lojbo", "bangu"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn embedded_lujvo_index_splits_r_hyphen_after_initial_cvv_rafsi() {
        let ciartai = lujvo_entry_for_word("ci'artai").expect("lujvo entry for ci'artai");
        let segments = ciartai
            .segments
            .iter()
            .map(|segment| (segment.kind, segment.surface, segment.source_word))
            .collect::<Vec<_>>();

        assert_eq!(
            segments,
            [
                (DictionaryLujvoSegmentKind::Rafsi, "ci'á", Some("ciska")),
                (DictionaryLujvoSegmentKind::Hyphen, "r", None),
                (DictionaryLujvoSegmentKind::Rafsi, "taĭ", Some("tarmi")),
            ]
        );
        assert_eq!(ciartai.source_words, ["ciska", "tarmi"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn normalized_word_lookup_preserves_collisions() {
        let words = english()
            .lookup_words("internet")
            .map(|entry| entry.word)
            .collect::<Vec<_>>();
        assert!(words.contains(&"INternet"));
        assert!(words.contains(&"internet"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rafsi_lookup_includes_listed_and_universal_sources() {
        let listed = english()
            .lookup_rafsi("bau")
            .map(|matched| (matched.entry.word, matched.source))
            .collect::<Vec<_>>();
        assert!(listed.contains(&("bangu", RafsiSource::Listed)));

        let universal = english()
            .lookup_rafsi("banl")
            .map(|matched| (matched.entry.word, matched.source))
            .collect::<Vec<_>>();
        assert!(universal.contains(&("banli", RafsiSource::UniversalShort)));
    }

    #[requires(!word.is_empty())]
    #[ensures(true)]
    fn sound_entry_for_word(word: &str) -> Option<&'static DictionarySoundEntry<'static>> {
        let index = english()
            .entries()
            .iter()
            .position(|entry| entry.word == word)?;
        english()
            .sound_index()
            .iter()
            .find(|entry| entry.entry_index.get() == index)
    }

    #[requires(!word.is_empty())]
    #[ensures(true)]
    fn lujvo_entry_for_word(word: &str) -> Option<&'static DictionaryLujvoEntry<'static>> {
        let index = english()
            .entries()
            .iter()
            .position(|entry| entry.word == word)?;
        english()
            .lujvo_index()
            .iter()
            .find(|entry| entry.entry_index.get() == index)
    }

    #[requires(true)]
    #[ensures(true)]
    fn expected_sound_index_skips() -> Vec<String> {
        DICTIONARY_SOUND_INDEX_SKIPS
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_owned)
            .collect()
    }
}
