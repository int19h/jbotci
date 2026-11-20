//! Dictionary links for the words in a cmavo table.
//!
//! The importer decides each `cmavo` cell and each `gismu` cell against the
//! dictionary that this build ships. A match becomes one dictionary link. A
//! miss stays plain text. Every host reads those links from the loaded book.

use std::sync::OnceLock;

#[allow(unused_imports)]
use bityzba::{ensures, expensive_ensures, expensive_invariant, new, requires};
use rustc_hash::{FxHashMap, FxHashSet};

use super::*;

/// Exact spellings from one dictionary, plus a second map for outer pause dots.
///
/// The exact map wins. The second map drops only the pause dots at the two ends
/// of a spelling. It keeps the spelling that equals the stripped key when one
/// exists. Otherwise it keeps the first spelling.
#[expensive_invariant(
    exact.iter().all(|spelling| !spelling.is_empty()),
    "each exact entry is a dictionary spelling"
)]
#[expensive_invariant(
    stripped.iter().all(|(key, spelling)| {
        !key.is_empty() && spelling.trim_matches('.') == *key
    }),
    "each stripped key drops only the outer pause dots of its spelling"
)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DictionarySpellingIndex<'word> {
    exact: FxHashSet<&'word str>,
    stripped: FxHashMap<&'word str, &'word str>,
}

impl<'word> DictionarySpellingIndex<'word> {
    /// Build an index from dictionary headwords. Empty headwords are skipped.
    ///
    /// The maps store the headwords that the caller already owns. This function
    /// does not copy those spellings.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_entry_words(words: impl IntoIterator<Item = &'word str>) -> Self {
        let mut exact = FxHashSet::default();
        let mut stripped = FxHashMap::default();
        for word in words {
            if word.is_empty() {
                continue;
            }
            exact.insert(word);
            remember_outer_dot_spelling(&mut stripped, word);
        }
        new!(Self { exact, stripped })
    }

    /// The dictionary spelling for `candidate`, if one matches.
    ///
    /// A match is the same spelling, or the same spelling once outer pause dots
    /// are ignored. Case, `h`, inner dots and inner commas stay significant.
    /// The returned spelling is the headword stored in this index.
    #[requires(true)]
    #[ensures(match ret {
        Some(spelling) => {
            spelling == candidate
                || {
                    let key = candidate.trim_matches('.');
                    !key.is_empty() && spelling.trim_matches('.') == key
                }
        }
        None => {
            !self.exact.contains(candidate)
                && {
                    let key = candidate.trim_matches('.');
                    key.is_empty() || !self.stripped.contains_key(key)
                }
        }
    })]
    pub(crate) fn link_target(&self, candidate: &str) -> Option<&'word str> {
        if let Some(spelling) = self.exact.get(candidate).copied() {
            return Some(spelling);
        }
        let key = candidate.trim_matches('.');
        if key.is_empty() {
            return None;
        }
        self.stripped.get(key).copied()
    }
}

impl DictionarySpellingIndex<'static> {
    /// The dictionary that jbotci ships with this build.
    #[requires(true)]
    #[ensures(!ret.exact.is_empty())]
    pub(crate) fn from_shipped_dictionary() -> Self {
        Self::from_entry_words(
            jbotci_dictionary_data::english()
                .entries()
                .iter()
                .map(|entry| entry.word),
        )
    }
}

/// The one shared index for the shipped dictionary.
///
/// Site load and the import probes borrow this index. They do not build a
/// second copy of the headword maps.
#[requires(true)]
#[ensures(!ret.exact.is_empty())]
pub(crate) fn shipped_dictionary_index() -> &'static DictionarySpellingIndex<'static> {
    static INDEX: OnceLock<DictionarySpellingIndex<'static>> = OnceLock::new();
    INDEX.get_or_init(DictionarySpellingIndex::from_shipped_dictionary)
}

/// Keep the spelling that equals the stripped key. Otherwise keep the first one.
#[requires(!word.is_empty())]
#[ensures({
    let key = word.trim_matches('.');
    key.is_empty()
        || stripped
            .get(key)
            .is_some_and(|spelling| spelling.trim_matches('.') == key)
})]
#[ensures({
    let key = word.trim_matches('.');
    key.is_empty() || word != key || stripped.get(key).copied() == Some(word)
})]
#[expensive_ensures({
    let key = word.trim_matches('.');
    let previous = old(stripped.get(word.trim_matches('.')).copied());
    if key.is_empty() {
        previous.is_none() && stripped.get(key).is_none()
    } else {
        match previous {
            Some(current) if current == key => stripped.get(key).copied() == Some(current),
            Some(_) if word == key => stripped.get(key).copied() == Some(word),
            Some(current) => stripped.get(key).copied() == Some(current),
            None => stripped.get(key).copied() == Some(word),
        }
    }
})]
fn remember_outer_dot_spelling<'word>(
    stripped: &mut FxHashMap<&'word str, &'word str>,
    word: &'word str,
) {
    let key = word.trim_matches('.');
    if key.is_empty() {
        return;
    }
    match stripped.get(key).copied() {
        Some(current) if current == key => {}
        Some(_) if word == key => {
            stripped.insert(key, word);
        }
        Some(_) => {}
        None => {
            stripped.insert(key, word);
        }
    }
}

/// One token of letters, apostrophes and pause dots. The token must be non-empty.
#[requires(true)]
#[ensures(
    ret == (!text.is_empty()
        && text
            .chars()
            .all(|character| character.is_ascii_alphabetic() || character == '\'' || character == '.'))
)]
pub(crate) fn is_dictionary_token(text: &str) -> bool {
    !text.is_empty()
        && text.chars().all(|character| {
            character.is_ascii_alphabetic() || character == '\'' || character == '.'
        })
}

/// The word inside one pair of brackets, when that word is one dictionary token.
#[requires(true)]
#[ensures({
    let paired = text
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'));
    match ret {
        Some(word) => paired == Some(word) && is_dictionary_token(word),
        None => paired.is_none_or(|inner| !is_dictionary_token(inner)),
    }
})]
pub(crate) fn bracketed_dictionary_word(text: &str) -> Option<&str> {
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    is_dictionary_token(inner).then_some(inner)
}

/// The inlines for one cmavo or gismu cell.
///
/// `text` is the cell text with whitespace already collapsed and trimmed.
/// A bracketed word that links keeps the brackets as text outside the link.
/// A bracketed word that does not link is one text inline.
#[requires(text == text.trim())]
#[ensures(ret.is_empty() == text.is_empty())]
#[ensures({
    if text.is_empty() {
        ret.is_empty()
    } else if let Some(word) = bracketed_dictionary_word(text) {
        match dictionary.link_target(word) {
            Some(target) => matches!(
                ret.as_slice(),
                [
                    CllInline::Text(open),
                    CllInline::Link {
                        target: link_target,
                        inlines,
                        kind: CllLinkKind::Dictionary,
                    },
                    CllInline::Text(close),
                ] if open == "["
                    && close == "]"
                    && link_target == target
                    && matches!(inlines.as_slice(), [CllInline::Text(shown)] if shown == word)
            ),
            None => matches!(ret.as_slice(), [CllInline::Text(shown)] if shown == text),
        }
    } else if is_dictionary_token(text) {
        match dictionary.link_target(text) {
            Some(target) => matches!(
                ret.as_slice(),
                [CllInline::Link {
                    target: link_target,
                    inlines,
                    kind: CllLinkKind::Dictionary,
                }] if link_target == target
                    && matches!(inlines.as_slice(), [CllInline::Text(shown)] if shown == text)
            ),
            None => matches!(ret.as_slice(), [CllInline::Text(shown)] if shown == text),
        }
    } else {
        matches!(ret.as_slice(), [CllInline::Text(shown)] if shown == text)
    }
})]
pub(crate) fn cmavo_or_gismu_cell_inlines(
    text: &str,
    dictionary: &DictionarySpellingIndex<'_>,
) -> Vec<CllInline> {
    if text.is_empty() {
        return Vec::new();
    }
    let inlines = if let Some(word) = bracketed_dictionary_word(text) {
        match dictionary.link_target(word) {
            Some(target) => vec![
                CllInline::Text("[".to_owned()),
                CllInline::Link {
                    target: target.to_owned(),
                    inlines: vec![CllInline::Text(word.to_owned())],
                    kind: CllLinkKind::Dictionary,
                },
                CllInline::Text("]".to_owned()),
            ],
            None => vec![
                CllInline::Text("[".to_owned()),
                CllInline::Text(word.to_owned()),
                CllInline::Text("]".to_owned()),
            ],
        }
    } else if is_dictionary_token(text) {
        match dictionary.link_target(text) {
            Some(target) => vec![CllInline::Link {
                target: target.to_owned(),
                inlines: vec![CllInline::Text(text.to_owned())],
                kind: CllLinkKind::Dictionary,
            }],
            None => vec![CllInline::Text(text.to_owned())],
        }
    } else {
        vec![CllInline::Text(text.to_owned())]
    };
    crate::import::merge_adjacent_text_inlines(inlines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn spelling_index_matches_exact_text_or_outer_pause_dots_only() {
        let both = DictionarySpellingIndex::from_entry_words([".ui", "ui", "..ui", "klama", "ui'"]);
        assert_eq!(both.link_target(".ui"), Some(".ui"));
        assert_eq!(both.link_target("ui"), Some("ui"));
        assert_eq!(both.link_target("..ui"), Some("..ui"));
        assert_eq!(both.link_target(".ui."), Some("ui"));
        assert_eq!(both.link_target("klama"), Some("klama"));
        assert_eq!(both.link_target("Klama"), None);
        assert_eq!(both.link_target("uih"), None);
        assert_eq!(both.link_target("u.i"), None);
        assert_eq!(both.link_target(""), None);
        assert_eq!(both.link_target("..."), None);

        let dotted = DictionarySpellingIndex::from_entry_words([".ui"]);
        assert_eq!(dotted.link_target("ui"), Some(".ui"));
        assert_eq!(dotted.link_target(".ui"), Some(".ui"));

        let bare = DictionarySpellingIndex::from_entry_words(["ui"]);
        assert_eq!(bare.link_target(".ui"), Some("ui"));

        let first_dotted = DictionarySpellingIndex::from_entry_words([".ui", "..ui"]);
        assert_eq!(first_dotted.link_target("ui"), Some(".ui"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shipped_dictionary_keeps_case_and_h_distinct() {
        let index = DictionarySpellingIndex::from_shipped_dictionary();
        assert_eq!(index.link_target("coi"), Some("coi"));
        assert_eq!(index.link_target("klama"), Some("klama"));
        assert_eq!(index.link_target("jundi"), Some("jundi"));
        assert_eq!(index.link_target("do'e"), Some("do'e"));
        assert_eq!(index.link_target("Klama"), None);
        assert_eq!(index.link_target("uih"), None);
        assert_eq!(index.link_target("dohe"), None);
        assert_eq!(index.link_target("ui'"), None);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shipped_dictionary_index_is_one_shared_value() {
        let first = shipped_dictionary_index();
        let second = shipped_dictionary_index();
        assert!(std::ptr::eq(first, second));
        assert_eq!(first.link_target("coi"), Some("coi"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cell_inlines_link_a_known_compound_and_keep_an_unknown_one_as_text() {
        let index = DictionarySpellingIndex::from_entry_words(["roinai", "ui"]);
        assert!(matches!(
            cmavo_or_gismu_cell_inlines("roinai", &index).as_slice(),
            [CllInline::Link {
                target,
                inlines,
                kind: CllLinkKind::Dictionary,
            }] if target == "roinai"
                && matches!(inlines.as_slice(), [CllInline::Text(text)] if text == "roinai")
        ));
        assert_eq!(
            cmavo_or_gismu_cell_inlines("pino'o", &index),
            vec![CllInline::Text("pino'o".to_owned())]
        );
        assert!(matches!(
            cmavo_or_gismu_cell_inlines(".ui", &index).as_slice(),
            [CllInline::Link {
                target,
                inlines,
                kind: CllLinkKind::Dictionary,
            }] if target == "ui"
                && matches!(inlines.as_slice(), [CllInline::Text(text)] if text == ".ui")
        ));
        assert_eq!(
            cmavo_or_gismu_cell_inlines("Klama", &index),
            vec![CllInline::Text("Klama".to_owned())]
        );
        assert_eq!(
            cmavo_or_gismu_cell_inlines("[jundi]", &index),
            vec![CllInline::Text("[jundi]".to_owned())]
        );
        assert!(cmavo_or_gismu_cell_inlines("", &index).is_empty());
    }
}
