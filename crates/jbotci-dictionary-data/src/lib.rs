//! Embedded Lensisku dictionary snapshots.

use bityzba::requires;
use jbotci_dictionary::Dictionary;

include!(concat!(env!("OUT_DIR"), "/dictionary_en.rs"));

/// Return the embedded English Lensisku dictionary.
#[requires(true)]
#[ensures(true)]
pub fn english() -> &'static Dictionary<'static> {
    &ENGLISH
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
