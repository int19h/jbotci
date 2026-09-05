//! Morphology-derived dictionary keys. Presentation overlap policy belongs to search.

use bityzba::{invariant, requires};
use jbotci_morphology::{WordKind, segment_words_with_modifiers};

use crate::EntryIndex;

/// A generated sequence row, validated at construction even in static initializers.
/// Dictionary validation additionally checks canonical morphology and exact targets.
#[invariant(true, "private fields are validated by the const constructor")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CmavoSequenceIndexEntry<'a> {
    components: &'a [&'a str],
    targets: &'a [EntryIndex],
}

impl<'a> CmavoSequenceIndexEntry<'a> {
    #[requires(components.len() >= 2 && !targets.is_empty())]
    #[ensures(true)]
    pub const fn from_static_parts(components: &'a [&'a str], targets: &'a [EntryIndex]) -> Self {
        let mut index = 0;
        while index < components.len() {
            assert!(!components[index].is_empty());
            index += 1;
        }
        index = 1;
        while index < targets.len() {
            assert!(targets[index - 1].0 < targets[index].0);
            index += 1;
        }
        Self {
            components,
            targets,
        }
    }

    #[requires(true)]
    #[ensures(ret.len() >= 2)]
    pub const fn components(&self) -> &'a [&'a str] {
        self.components
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub const fn targets(&self) -> &'a [EntryIndex] {
        self.targets
    }
}

/// Owned generation row; spelling variants share one key and retain source-order targets.
#[invariant(components.len() >= 2 && !targets.is_empty())]
#[expensive_invariant(components.iter().all(|component| !component.is_empty()))]
#[expensive_invariant(targets.windows(2).all(|pair| pair[0] < pair[1]))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedCmavoSequenceIndexEntry {
    pub components: Vec<String>,
    pub targets: Vec<EntryIndex>,
}

/// The only source characters allowed between recognized compound members.
/// Word boundaries themselves always come from morphology.
#[requires(true)]
#[ensures(ret == (character.is_whitespace() || character == '.'))]
pub fn is_compound_separator(character: char) -> bool {
    character.is_whitespace() || character == '.'
}

/// Parse an entire headword as at least two plain cmavo, rejecting any uncovered text.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|components| components.len() >= 2))]
pub fn cmavo_sequence_key(headword: &str) -> Option<Vec<String>> {
    let words = segment_words_with_modifiers(headword).ok()?;
    if words.len() < 2 {
        return None;
    }
    let mut end = 0;
    let mut components = Vec::with_capacity(words.len());
    for word_like in &words {
        let word = word_like.bare_word()?;
        if word.kind() != WordKind::Cmavo {
            return None;
        }
        let span = word.span();
        if !headword
            .get(end..span.byte_start)?
            .chars()
            .all(is_compound_separator)
        {
            return None;
        }
        end = span.byte_end;
        components.push(word.canonical_phonemes());
    }
    headword
        .get(end..)?
        .chars()
        .all(is_compound_separator)
        .then_some(components)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn keys_follow_morphology_and_require_full_source_coverage() {
        for spelling in ["punaijecanai", "pu nai je ca nai", "punai je canai"] {
            assert_eq!(
                cmavo_sequence_key(spelling).unwrap(),
                ["pu", "nai", "je", "ca", "nai"]
            );
        }
        assert_eq!(cmavo_sequence_key("na.a").unwrap(), ["na", "a"]);
        for spelling in [
            "ma;u",
            "madagasikara",
            "fa'onai",
            "o'ebu",
            "la dontu'u",
            "pu zei ba",
            "zo pu",
            "pu ! ba",
        ] {
            assert_eq!(cmavo_sequence_key(spelling), None, "{spelling}");
        }
    }
}
