//! Dictionary links for the words in a cmavo table.
//!
//! The importer records a candidate. It does not decide which spellings exist.
//! Site load checks each candidate against the dictionary that this build ships.
//! A match becomes one dictionary link. A miss becomes plain text. Both
//! renderers then see the same links.

use std::collections::BTreeMap;

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, expensive_invariant, invariant, new, requires};

use crate::visitor::{CllBlockVisitor, walk_inline, walk_inline_mut};

use super::*;

/// Exact spellings from the shipped dictionary, plus a second map for outer pause dots.
///
/// The exact map wins. The second map drops only the pause dots at the two ends
/// of a spelling. It keeps the spelling that has no outer dots when several
/// entries share that core.
#[expensive_invariant(
    exact.iter().all(|(key, spelling)| key == spelling && !key.is_empty()),
    "each exact key is that dictionary spelling"
)]
#[expensive_invariant(
    stripped.iter().all(|(key, spelling)| {
        !key.is_empty() && spelling.trim_matches('.') == key.as_str()
    }),
    "each stripped key drops only the outer pause dots of its spelling"
)]
#[derive(Debug)]
pub(crate) struct DictionarySpellingIndex {
    exact: BTreeMap<String, String>,
    stripped: BTreeMap<String, String>,
}

impl DictionarySpellingIndex {
    /// Build an index from dictionary headwords. Empty headwords are skipped.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_entry_words<'word>(words: impl IntoIterator<Item = &'word str>) -> Self {
        let mut exact = BTreeMap::new();
        let mut stripped = BTreeMap::new();
        for word in words {
            if word.is_empty() {
                continue;
            }
            exact.insert(word.to_owned(), word.to_owned());
            remember_outer_dot_spelling(&mut stripped, word);
        }
        new!(Self { exact, stripped })
    }

    /// The dictionary that jbotci ships with this build.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn from_shipped_dictionary() -> Self {
        Self::from_entry_words(
            jbotci_dictionary_data::english()
                .entries()
                .iter()
                .map(|entry| entry.word),
        )
    }

    /// The dictionary spelling for `candidate`, if one matches.
    ///
    /// A match is the same spelling, or the same spelling once outer pause dots
    /// are ignored. Case, `h`, inner dots and inner commas stay significant.
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
            !self.exact.contains_key(candidate)
                && {
                    let key = candidate.trim_matches('.');
                    key.is_empty() || !self.stripped.contains_key(key)
                }
        }
    })]
    pub(crate) fn link_target(&self, candidate: &str) -> Option<&str> {
        if let Some(spelling) = self.exact.get(candidate) {
            return Some(spelling.as_str());
        }
        let key = candidate.trim_matches('.');
        if key.is_empty() {
            return None;
        }
        self.stripped.get(key).map(String::as_str)
    }
}

/// Keep the spelling that equals the stripped key. Otherwise keep the first one.
#[requires(!word.is_empty())]
#[ensures(true)]
fn remember_outer_dot_spelling(stripped: &mut BTreeMap<String, String>, word: &str) {
    let key = word.trim_matches('.');
    if key.is_empty() {
        return;
    }
    match stripped.get(key) {
        Some(current) if current.as_str() == key => {}
        Some(_) if word == key => {
            stripped.insert(key.to_owned(), word.to_owned());
        }
        Some(_) => {}
        None => {
            stripped.insert(key.to_owned(), word.to_owned());
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
#[ensures(ret.is_some_and(|word| {
    text.starts_with('[') && text.ends_with(']') && is_dictionary_token(word)
}) || ret.is_none())]
pub(crate) fn bracketed_dictionary_word(text: &str) -> Option<&str> {
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    is_dictionary_token(inner).then_some(inner)
}

/// The inlines for one cmavo or gismu cell. This does not consult the dictionary.
#[requires(true)]
#[ensures(ret.is_empty() == text.is_empty())]
#[ensures(!ret.iter().any(|inline| matches!(inline, CllInline::Link { .. })))]
pub(crate) fn cmavo_or_gismu_cell_inlines(text: &str) -> Vec<CllInline> {
    if text.is_empty() {
        return Vec::new();
    }
    if let Some(word) = bracketed_dictionary_word(text) {
        return vec![
            CllInline::Text("[".to_owned()),
            CllInline::DictionaryCandidate {
                spelling: word.to_owned(),
            },
            CllInline::Text("]".to_owned()),
        ];
    }
    if is_dictionary_token(text) {
        return vec![CllInline::DictionaryCandidate {
            spelling: text.to_owned(),
        }];
    }
    vec![CllInline::Text(text.to_owned())]
}

/// Replace every candidate in `blocks` with a dictionary link or with plain text.
#[requires(true)]
#[ensures(true)]
pub(crate) fn resolve_dictionary_candidates(
    blocks: &mut [CllBlock],
    index: &DictionarySpellingIndex,
) {
    let mut visitor = DictionaryCandidateResolver { index };
    visitor.visit_blocks_mut(blocks);
}

/// Replace every candidate in the site. Chapter preludes, sections and examples are all included.
#[requires(true)]
#[ensures(true)]
#[bityzba::expensive_ensures(!site_contains_dictionary_candidate(&ret))]
pub(crate) fn resolve_cmavo_table_dictionary_links(site: CllSite) -> CllSite {
    let index = DictionarySpellingIndex::from_shipped_dictionary();
    let mut site_data = site.into_data();
    site_data.chapters = site_data
        .chapters
        .into_iter()
        .map(|chapter| {
            let mut chapter_data = chapter.into_data();
            resolve_dictionary_candidates(&mut chapter_data.prelude_blocks, &index);
            CllChapter::from_data(chapter_data)
        })
        .collect();
    site_data.examples_by_id = site_data
        .examples_by_id
        .into_iter()
        .map(|(id, example)| {
            let mut example_data = example.into_data();
            resolve_dictionary_candidates(&mut example_data.blocks, &index);
            (id, CllExample::from_data(example_data))
        })
        .collect();
    site_data.sections_by_id = site_data
        .sections_by_id
        .into_iter()
        .map(|(id, section)| {
            let mut section_data = section.into_data();
            resolve_dictionary_candidates(&mut section_data.blocks, &index);
            (id, CllSection::from_data(section_data))
        })
        .collect();
    CllSite::from_data(site_data)
}

#[invariant(true)]
#[derive(Debug)]
struct DictionaryCandidateResolver<'index> {
    index: &'index DictionarySpellingIndex,
}

#[contract_trait]
impl CllBlockVisitor for DictionaryCandidateResolver<'_> {
    #[requires(true)]
    #[ensures(true)]
    fn visit_inline_mut(&mut self, inline: &mut CllInline) {
        if let CllInline::DictionaryCandidate { spelling } = inline {
            let spelling = spelling.clone();
            *inline = match self.index.link_target(&spelling) {
                Some(target) => CllInline::Link {
                    target: target.to_owned(),
                    inlines: vec![CllInline::Text(spelling)],
                    kind: CllLinkKind::Dictionary,
                },
                None => CllInline::Text(spelling),
            };
            return;
        }
        walk_inline_mut(self, inline);
    }
}

// The expensive postcondition is the caller. An ordinary build drops that check.
#[allow(dead_code)]
#[invariant(true)]
#[derive(Debug)]
struct DictionaryCandidateFinder {
    found: bool,
}

#[contract_trait]
impl CllBlockVisitor for DictionaryCandidateFinder {
    #[requires(true)]
    #[ensures(true)]
    fn visit_inline(&mut self, inline: &CllInline) {
        if self.found {
            return;
        }
        if matches!(inline, CllInline::DictionaryCandidate { .. }) {
            self.found = true;
            return;
        }
        walk_inline(self, inline);
    }
}

#[allow(dead_code)]
#[requires(true)]
#[ensures(true)]
fn blocks_contain_dictionary_candidate(blocks: &[CllBlock]) -> bool {
    let mut finder = DictionaryCandidateFinder { found: false };
    finder.visit_blocks(blocks);
    finder.found
}

#[allow(dead_code)]
#[requires(true)]
#[ensures(true)]
pub(crate) fn site_contains_dictionary_candidate(site: &CllSite) -> bool {
    site.chapters
        .iter()
        .any(|chapter| blocks_contain_dictionary_candidate(&chapter.prelude_blocks))
        || site
            .sections_by_id
            .values()
            .any(|section| blocks_contain_dictionary_candidate(&section.blocks))
        || site
            .examples_by_id
            .values()
            .any(|example| blocks_contain_dictionary_candidate(&example.blocks))
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
    fn resolver_links_a_known_compound_and_keeps_an_unknown_one_as_text() {
        let index = DictionarySpellingIndex::from_entry_words(["roinai", "ui"]);
        let mut blocks = vec![CllBlock::CmavoList {
            id: None,
            titles: Vec::new(),
            headers: Vec::new(),
            rows: vec![vec![
                cmavo_or_gismu_cell_inlines("roinai"),
                cmavo_or_gismu_cell_inlines("pino'o"),
                cmavo_or_gismu_cell_inlines(".ui"),
                cmavo_or_gismu_cell_inlines("Klama"),
                cmavo_or_gismu_cell_inlines("[jundi]"),
            ]],
        }];

        resolve_dictionary_candidates(&mut blocks, &index);

        let CllBlock::CmavoList { rows, .. } = &blocks[0] else {
            panic!("the block is a cmavo list");
        };
        let row = &rows[0];
        assert!(matches!(
            row[0].as_slice(),
            [CllInline::Link {
                target,
                inlines,
                kind: CllLinkKind::Dictionary,
            }] if target == "roinai"
                && matches!(inlines.as_slice(), [CllInline::Text(text)] if text == "roinai")
        ));
        assert_eq!(row[1], vec![CllInline::Text("pino'o".to_owned())]);
        assert!(matches!(
            row[2].as_slice(),
            [CllInline::Link {
                target,
                inlines,
                kind: CllLinkKind::Dictionary,
            }] if target == "ui"
                && matches!(inlines.as_slice(), [CllInline::Text(text)] if text == ".ui")
        ));
        assert_eq!(row[3], vec![CllInline::Text("Klama".to_owned())]);
        assert_eq!(
            row[4],
            vec![
                CllInline::Text("[".to_owned()),
                CllInline::Text("jundi".to_owned()),
                CllInline::Text("]".to_owned()),
            ]
        );
    }
}
