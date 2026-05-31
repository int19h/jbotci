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
    use bityzba::requires;
    use jbotci_dictionary::RafsiSource;

    use super::*;

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
}
