//! Embedded Lensisku dictionary snapshots.

use bityzba::{invariant, requires};
use jbotci_dictionary::Dictionary;

/// Metadata for a vendored Lensisku dictionary snapshot.
///
/// `definition_count` counts the definitions Lensisku exported and
/// `entry_count` the entries this crate embeds. They differ because an export
/// may carry several definitions of one word, of which the snapshot keeps the
/// one Lensisku's own ranking prefers.
///
/// The type carries no generated invariant because its only value is embedded
/// as a `static`, which a validated wrapper cannot initialize. Its one real
/// constraint — `entry_count <= definition_count` — is enforced where the value
/// is produced (`build.rs`'s own metadata type) and re-checked on every read
/// through [`english_metadata`].
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DictionarySnapshotMetadata {
    pub language_tag: &'static str,
    pub language_realname: &'static str,
    /// Language of the defined words, `jbo` for every snapshot jbotci vendors.
    pub source_language_tag: &'static str,
    pub format: &'static str,
    /// Whether the export was restricted to positively scored definitions.
    pub positive_scores_only: bool,
    pub filename: &'static str,
    pub metadata_url: &'static str,
    pub download_url: &'static str,
    pub lensisku_created_at: &'static str,
    pub sha256: &'static str,
    pub definition_count: usize,
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
#[ensures(
    ret.entry_count <= ret.definition_count,
    "selection drops definitions, never invents them"
)]
pub fn english_metadata() -> &'static DictionarySnapshotMetadata {
    &ENGLISH_METADATA
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use bityzba::requires;
    use jbotci_dictionary::{
        DictionaryLujvoEntry, DictionaryLujvoSegment, DictionaryLujvoSegmentKind,
        DictionarySoundEntry, RafsiClaimKind, RafsiSource, WordType,
    };

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
            "2026-09-01T11:38:52Z"
        );
        assert_eq!(english_metadata().definition_count, 33053);
        assert!(english_metadata().definition_count > english_metadata().entry_count);
        assert!(!english_metadata().positive_scores_only);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn short_rafsi_candidates_resolve_against_the_embedded_dictionary() {
        let dictionary = english();
        // Every short rafsi `sakli` could claim is already assigned: `sal` to
        // `sakli` itself, and the rest to unrelated official gismu.
        assert_eq!(
            dictionary
                .short_rafsi_candidates("sakli")
                .iter()
                .map(|candidate| (
                    candidate.form.as_str(),
                    candidate.availability.claim_kind(),
                    candidate.availability.claimant_words().to_vec(),
                ))
                .collect::<Vec<_>>(),
            [
                ("kli", "klina"),
                ("sa'i", "sanli"),
                ("sai", "sanmi"),
                ("sak", "sakci"),
                ("sal", "sakli"),
                ("ska", "skari"),
            ]
            .map(|(form, holder)| (
                form,
                Some(RafsiClaimKind::Official),
                vec![holder.to_owned()]
            ))
        );
        assert!(
            dictionary
                .short_rafsi_candidates("nanpe")
                .iter()
                .any(|candidate| candidate.availability.is_free()),
            "the invented gismu nanpe still has free short rafsi"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn short_rafsi_candidates_count_cmavo_claims() {
        // Regression for issue #769: `kam` is the listed rafsi of the cmavo
        // `ka` (CLL 4.6), so it is not available to an invented `kacma`.
        let dictionary = english();
        assert_eq!(
            dictionary
                .short_rafsi_candidates("kacma")
                .iter()
                .map(|candidate| (
                    candidate.form.as_str(),
                    candidate.availability.claim_kind(),
                    candidate.availability.claimant_words().to_vec(),
                ))
                .collect::<Vec<_>>(),
            [
                ("cma", "cmalu"),
                ("ka'a", "katna"),
                ("kac", "kancu"),
                ("kam", "ka"),
            ]
            .map(|(form, holder)| (
                form,
                Some(RafsiClaimKind::Official),
                vec![holder.to_owned()]
            ))
        );
        assert_eq!(
            dictionary.rafsi_claimants("kam").collect::<Vec<_>>(),
            vec![("ka", WordType::Cmavo)]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn refreshed_snapshot_derived_indexes_match_audited_counts() {
        assert_eq!(english().sound_index().len(), 30_639);
        assert_eq!(english().lujvo_index().len(), 12_724);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn refreshed_snapshot_additions_cover_non_lujvo_word_types() {
        assert_non_lujvo_sound_addition("boxna bu", "phrase", "ˈbox.na bu");
        assert_non_lujvo_sound_addition("gerku bu", "bu-letteral", "ˈger.ku bu");
        assert_non_lujvo_sound_addition("bevda", "experimental gismu", "ˈbev.da");
        assert_non_lujvo_sound_addition("pi'u'i", "experimental cmavo", "pi.ˈhu.hi");
        assert_non_lujvo_sound_addition("plumbago", "fu'ivla", "plum.ˈba.go");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn refreshed_snapshot_additions_include_structural_lujvo_derivations() {
        let badjai = lujvo_entry_for_word("badja'i").expect("lujvo entry for badja'i");
        let [bad, jai] = badjai.segments else {
            panic!("badja'i should have two segments");
        };
        assert_lujvo_segment(bad, DictionaryLujvoSegmentKind::Rafsi, "bad", Some("bandu"));
        assert_lujvo_segment(
            jai,
            DictionaryLujvoSegmentKind::Rafsi,
            "já'i",
            Some("jadni"),
        );
        assert_eq!(badjai.source_words, ["bandu", "jadni"]);

        let bendicybia = lujvo_entry_for_word("bendicybi'a").expect("lujvo entry for bendicybi'a");
        let [ben, dic, y, bia] = bendicybia.segments else {
            panic!("bendicybi'a should have four segments");
        };
        assert_lujvo_segment(ben, DictionaryLujvoSegmentKind::Rafsi, "ben", Some("besna"));
        assert_lujvo_segment(dic, DictionaryLujvoSegmentKind::Rafsi, "dic", Some("dikca"));
        assert_lujvo_segment(y, DictionaryLujvoSegmentKind::Hyphen, "y", None);
        assert_lujvo_segment(
            bia,
            DictionaryLujvoSegmentKind::Rafsi,
            "bí'a",
            Some("bilma"),
        );
        assert_eq!(bendicybia.source_words, ["besna", "dikca", "bilma"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn refreshed_snapshot_rafsi_enrich_existing_lujvo_derivations() {
        assert_listed_rafsi("jgu", "jguna");
        assert_listed_rafsi("cfo", "jicfo");
        assert_listed_rafsi("jid", "jidge");

        let cemjgu = lujvo_entry_for_word("cemjgu").expect("lujvo entry for cemjgu");
        assert_eq!(cemjgu.source_words, ["cecmu", "jguna"]);
        assert_eq!(
            cemjgu.segments.last().map(|segment| segment.source_word),
            Some(Some("jguna"))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn extracted_rafsi_are_merged_into_the_embedded_dictionary() {
        // The vendored extraction (issue #768) backfills rafsi that the
        // snapshot only ever stated in prose; downstream they are ordinary
        // listed rafsi. Every word named here must still be extraction-only:
        // Lensisku has since recorded structured rafsi for several words the
        // extraction covered (issue #881), and those were dropped from the
        // table, so asserting them here would no longer test the merge.
        assert_extracted_rafsi("celdi", &["cle"]);
        assert_extracted_rafsi("supso", &["sus"]);

        // Losers of the owner-adjudicated conflicts keep no rafsi at all:
        // `dit` went to ditcu, `sus` to supso, and dzama's `zam` claim was
        // dropped in favour of the cmavo zai'e.
        for word in ["dinti", "smusu", "dzama"] {
            let entry = english()
                .lookup_word(word)
                .unwrap_or_else(|| panic!("entry for {word}"));
            assert!(
                entry.rafsi.is_empty(),
                "{word} lost its contested rafsi claim and must stay rafsi-free"
            );
        }
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
            assert!(!sound_entry.pronunciation_targets.targets.is_empty());
            assert!(
                sound_entry
                    .pronunciation_targets
                    .self_similarity
                    .is_finite()
            );
            assert!(sound_entry.pronunciation_targets.self_similarity > 0.0);
            assert_eq!(
                sound_entry.pronunciation_targets.target_count(),
                sound_entry.token_sequence.segment_count()
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
    fn embedded_sound_index_keeps_display_r_separate_from_scoring_target() {
        let prami = sound_entry_for_word("prami").expect("sound entry for prami");
        assert_eq!(
            jbotci_phonetic::ipa_segment_symbol(prami.token_sequence.segments[1]),
            Some("r")
        );
        assert_eq!(
            prami.pronunciation_targets.targets[1],
            jbotci_phonetic::lojban_r_pronunciation_target()
        );
        assert_eq!(
            prami.pronunciation_targets.targets[1].realization_count(),
            7
        );
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
        let dictionary = english();
        let entries = dictionary.entries();
        let lujvo_index = dictionary.lujvo_index();
        for lujvo_entry in lujvo_index {
            assert!(
                previous_index.is_none_or(|previous| lujvo_entry.entry_index.get() > previous),
                "lujvo index entry order regressed at {:?}",
                lujvo_entry.entry_index
            );
            let entry = &entries[lujvo_entry.entry_index.get()];
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

    #[requires(!word.is_empty() && !word_type.is_empty() && !ipa.is_empty())]
    #[ensures(true)]
    fn assert_non_lujvo_sound_addition(word: &str, word_type: &str, ipa: &str) {
        let entry = english()
            .lookup_word(word)
            .unwrap_or_else(|| panic!("entry for {word}"));
        assert_eq!(entry.word_type.as_str(), word_type);
        assert_eq!(
            sound_entry_for_word(word)
                .unwrap_or_else(|| panic!("sound entry for {word}"))
                .ipa,
            ipa
        );
        assert!(lujvo_entry_for_word(word).is_none());
    }

    #[requires(!surface.is_empty())]
    #[ensures(true)]
    fn assert_lujvo_segment(
        segment: &DictionaryLujvoSegment<'_>,
        kind: DictionaryLujvoSegmentKind,
        surface: &str,
        source_word: Option<&str>,
    ) {
        assert_eq!(segment.kind, kind);
        assert_eq!(segment.surface, surface);
        assert_eq!(segment.source_word, source_word);
    }

    /// Assert that `word` carries exactly `rafsi`, each resolving back to it.
    #[requires(!word.is_empty() && !rafsi.is_empty())]
    #[ensures(true)]
    fn assert_extracted_rafsi(word: &str, rafsi: &[&str]) {
        let entry = english()
            .lookup_word(word)
            .unwrap_or_else(|| panic!("entry for {word}"));
        assert_eq!(
            entry
                .rafsi
                .iter()
                .map(|value| value.0)
                .collect::<Vec<_>>()
                .as_slice(),
            rafsi,
            "extracted rafsi for {word} did not reach the embedded entry"
        );
        for form in rafsi {
            assert_listed_rafsi(form, word);
        }
    }

    #[requires(!rafsi.is_empty() && !word.is_empty())]
    #[ensures(true)]
    fn assert_listed_rafsi(rafsi: &str, word: &str) {
        assert!(
            english()
                .lookup_rafsi(rafsi)
                .any(|matched| matched.entry.word == word && matched.source == RafsiSource::Listed),
            "listed rafsi {rafsi} should resolve to {word}"
        );
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
