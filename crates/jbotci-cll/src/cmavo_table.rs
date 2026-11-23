//! Dictionary links for the words in a cmavo table.
//!
//! The importer asks the shipped dictionary for each `cmavo` cell and each
//! `gismu` cell. A match becomes one dictionary link. A miss that is one
//! dictionary token keeps a Lojban span. Any other miss stays one text node.
//! Every host reads that result from the loaded book. An adjacent valsi
//! part that does not link is one Lojban span. The span keeps the part text.
//! A marked lujvo that misses the dictionary keeps a marked span. An
//! opted-out `[word]` cell keeps the brackets outside that span.

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, invariant, new, requires, try_new};

use super::*;

/// One cmavo or gismu cell that holds a single dictionary word.
///
/// Only a `cmavo` cell or a `gismu` cell uses this view. A description cell
/// and every other column keep their plain inlines. `target` is the dictionary
/// headword when the word links. It is absent when the word stays plain. An
/// empty target is not a word cell.
#[invariant(::Bracket { word, target } => {
    is_dictionary_token(word) && target.as_ref().is_none_or(|target| !target.is_empty())
})]
#[invariant(::Token { text, target } => {
    is_dictionary_token(text) && target.as_ref().is_none_or(|target| !target.is_empty())
})]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CllCmavoWordCell<'a> {
    /// The cell text is `[word]`. The brackets stay outside a link.
    Bracket {
        word: &'a str,
        target: Option<&'a str>,
    },
    /// The cell text is one dictionary token.
    Token {
        text: &'a str,
        target: Option<&'a str>,
    },
}

/// The inlines a single-word cmavo cell or gismu cell can hold.
///
/// The list is empty, one text node, one dictionary link, one Lojban span,
/// or brackets around one link or one marked span. Each link and each span
/// holds one text node. The check reads the list and does not allocate.
#[requires(true)]
#[ensures(true)]
pub(crate) fn word_cell_shape(inlines: &[CllInline]) -> bool {
    match inlines {
        [] | [CllInline::Text(_)] => true,
        [
            CllInline::Link {
                kind: CllLinkKind::Dictionary,
                inlines,
                ..
            },
        ]
        | [
            CllInline::LanguageSpan {
                kind: CllLanguageSpanKind::JboPhrase | CllLanguageSpanKind::MarkedValsi,
                inlines,
                ..
            },
        ] => matches!(inlines.as_slice(), [CllInline::Text(_)]),
        [
            CllInline::Text(open),
            CllInline::Link {
                kind: CllLinkKind::Dictionary,
                inlines,
                ..
            },
            CllInline::Text(close),
        ]
        | [
            CllInline::Text(open),
            CllInline::LanguageSpan {
                kind: CllLanguageSpanKind::MarkedValsi,
                inlines,
                ..
            },
            CllInline::Text(close),
        ] => open == "[" && close == "]" && matches!(inlines.as_slice(), [CllInline::Text(_)]),
        _ => false,
    }
}

/// The word cell stored in `inlines`, when the importer built one.
#[requires(true)]
#[ensures(true)]
pub(crate) fn cmavo_word_cell(inlines: &[CllInline]) -> Option<CllCmavoWordCell<'_>> {
    match inlines {
        [
            CllInline::Text(open),
            CllInline::Link {
                target,
                inlines: inner,
                kind: CllLinkKind::Dictionary,
            },
            CllInline::Text(close),
        ] if open == "[" && close == "]" => {
            let [CllInline::Text(word)] = inner.as_slice() else {
                return None;
            };
            (!target.is_empty() && is_dictionary_token(word)).then(|| {
                new!(CllCmavoWordCell::Bracket {
                    word,
                    target: Some(target.as_str()),
                })
            })
        }
        [
            CllInline::Text(open),
            CllInline::LanguageSpan {
                kind: CllLanguageSpanKind::MarkedValsi,
                inlines: inner,
                ..
            },
            CllInline::Text(close),
        ] if open == "[" && close == "]" => {
            let [CllInline::Text(word)] = inner.as_slice() else {
                return None;
            };
            is_dictionary_token(word)
                .then(|| new!(CllCmavoWordCell::Bracket { word, target: None }))
        }
        [CllInline::Text(text)] => bracketed_dictionary_word(text)
            .map(|word| new!(CllCmavoWordCell::Bracket { word, target: None })),
        [
            CllInline::Link {
                target,
                inlines: inner,
                kind: CllLinkKind::Dictionary,
            },
        ] => {
            let [CllInline::Text(text)] = inner.as_slice() else {
                return None;
            };
            (!target.is_empty() && is_dictionary_token(text)).then(|| {
                new!(CllCmavoWordCell::Token {
                    text,
                    target: Some(target.as_str()),
                })
            })
        }
        [
            CllInline::LanguageSpan {
                kind: CllLanguageSpanKind::JboPhrase,
                inlines: inner,
                ..
            },
        ] => {
            let [CllInline::Text(text)] = inner.as_slice() else {
                return None;
            };
            is_dictionary_token(text).then(|| new!(CllCmavoWordCell::Token { text, target: None }))
        }
        [
            CllInline::LanguageSpan {
                kind: CllLanguageSpanKind::MarkedValsi,
                inlines: inner,
                ..
            },
        ] => {
            let [CllInline::Text(text)] = inner.as_slice() else {
                return None;
            };
            if let Some(word) = bracketed_dictionary_word(text) {
                return Some(new!(CllCmavoWordCell::Bracket { word, target: None }));
            }
            is_dictionary_token(text).then(|| new!(CllCmavoWordCell::Token { text, target: None }))
        }
        _ => None,
    }
}

/// One adjacent valsi part.
///
/// `text` is the spelling a reader sees. The spelling is not empty. It does
/// not start or end with whitespace. It has no line ending. `target` is the
/// dictionary headword when the part links. The spelling of a link is one
/// dictionary token. An empty target is not a part.
#[invariant(!text.is_empty())]
#[invariant(!text.chars().next().is_some_and(char::is_whitespace))]
#[invariant(!text.chars().next_back().is_some_and(char::is_whitespace))]
#[invariant(!text.contains(['\n', '\r']))]
#[invariant(target.as_ref().is_none_or(|link| {
    !link.is_empty() && is_dictionary_token(text)
}))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CllCmavoWordPart<'a> {
    pub text: &'a str,
    pub target: Option<&'a str>,
}

/// The separate valsi parts stored in `inlines`.
///
/// A part is one dictionary link or one unlinked Lojban span. The part type
/// holds the spelling rules. Zero parts is an empty list. One part is that
/// one inline. A bracket cell and any other inline return none. The cell
/// type, not this length, records that rule 12 applies.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|parts| parts.len() == inlines.len()))]
pub(crate) fn cmavo_word_parts(inlines: &[CllInline]) -> Option<Vec<CllCmavoWordPart<'_>>> {
    let mut parts = Vec::with_capacity(inlines.len());
    for inline in inlines {
        match inline {
            CllInline::Link {
                target,
                inlines,
                kind: CllLinkKind::Dictionary,
            } if !target.is_empty() => {
                let [CllInline::Text(text)] = inlines.as_slice() else {
                    return None;
                };
                parts.push(
                    try_new!(CllCmavoWordPart {
                        text: text.as_str(),
                        target: Some(target.as_str()),
                    })
                    .ok()?,
                );
            }
            CllInline::LanguageSpan {
                kind: CllLanguageSpanKind::JboPhrase | CllLanguageSpanKind::MarkedValsi,
                inlines,
                ..
            } => {
                let [CllInline::Text(text)] = inlines.as_slice() else {
                    return None;
                };
                parts.push(
                    try_new!(CllCmavoWordPart {
                        text: text.as_str(),
                        target: None,
                    })
                    .ok()?,
                );
            }
            _ => return None,
        }
    }
    Some(parts)
}

impl<'a> CllCmavoWordCell<'a> {
    /// The bracketed word and its dictionary target, when this cell is `[word]`.
    #[requires(true)]
    #[ensures(ret.is_none_or(|(word, _)| is_dictionary_token(word)))]
    pub fn bracket(self) -> Option<(&'a str, Option<&'a str>)> {
        match self.as_data() {
            data!(CllCmavoWordCell::Bracket { word, target }) => Some((word, *target)),
            data!(CllCmavoWordCell::Token { .. }) => None,
        }
    }

    /// The unlinked dictionary token, when this cell is one Lojban word with no link.
    #[requires(true)]
    #[ensures(ret.is_none_or(is_dictionary_token))]
    pub fn unlinked_token(self) -> Option<&'a str> {
        match self.as_data() {
            data!(CllCmavoWordCell::Token { text, target: None }) => Some(text),
            data!(CllCmavoWordCell::Token { .. }) | data!(CllCmavoWordCell::Bracket { .. }) => None,
        }
    }
}

/// The dictionary spelling for `candidate`.
///
/// `spellings` are the headwords that share the lookup key of `candidate`.
/// The exact spelling wins. Otherwise a spelling that differs only by pause
/// dots at the two ends wins. Among those, the spelling that equals the bare
/// key wins. Otherwise the first such spelling wins. Case, `h`, an inner dot
/// and an inner comma stay significant.
#[requires(true)]
#[ensures(!candidate.is_empty() || ret.is_none())]
#[ensures(ret.is_none_or(|spelling| {
    spelling == candidate
        || {
            let key = candidate.trim_matches('.');
            !key.is_empty() && spelling.trim_matches('.') == key
        }
}))]
pub(crate) fn dictionary_link_target<'word>(
    candidate: &str,
    spellings: impl IntoIterator<Item = &'word str>,
) -> Option<&'word str> {
    if candidate.is_empty() {
        return None;
    }
    let key = candidate.trim_matches('.');
    let mut outer_dot = None;
    for spelling in spellings {
        if spelling == candidate {
            return Some(spelling);
        }
        if key.is_empty() || spelling.trim_matches('.') != key {
            continue;
        }
        match outer_dot {
            Some(current) if current == key => {}
            Some(_) if spelling == key => outer_dot = Some(spelling),
            Some(_) => {}
            None => outer_dot = Some(spelling),
        }
    }
    outer_dot
}

/// The shipped-dictionary spelling for `candidate`.
///
/// The lookup is the dictionary's own word list for this candidate. This
/// function then keeps only an exact spelling, or a spelling that differs by
/// outer pause dots.
#[requires(true)]
#[ensures(ret.is_none_or(|spelling| {
    spelling == candidate
        || {
            let key = candidate.trim_matches('.');
            !key.is_empty() && spelling.trim_matches('.') == key
        }
}))]
pub(crate) fn shipped_dictionary_link_target(candidate: &str) -> Option<&'static str> {
    let dictionary = jbotci_dictionary_data::english();
    dictionary_link_target(
        candidate,
        dictionary.lookup_words(candidate).map(|entry| entry.word),
    )
}

/// One token of letters, apostrophes and pause dots. The token must be non-empty.
///
/// A true result has that shape. The shape is the word a bracket cell or a
/// one-word cell stores.
#[requires(true)]
#[ensures(!ret || {
    let mut characters = text.chars();
    matches!(
        characters.next(),
        Some('A'..='Z' | 'a'..='z' | '\'' | '.')
    ) && characters.all(|character| matches!(character, 'A'..='Z' | 'a'..='z' | '\'' | '.'))
})]
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
/// `target_for` returns the dictionary headword for one candidate. A bracketed
/// word that links keeps the brackets as text outside the link. A bracketed
/// word that does not link is one text node. A token that does not link is one
/// Lojban span, so its script does not depend on the dictionary. An empty
/// target is one text node for both shapes.
#[requires(text == text.trim())]
#[expensive_ensures(cmavo_cell_joined_text(&ret) == text)]
#[ensures(ret.iter().filter(|inline| matches!(inline, CllInline::Link { .. })).count() <= 1)]
#[ensures(ret.iter().all(|inline| match inline {
    CllInline::Text(_) => true,
    CllInline::Link {
        kind: CllLinkKind::Dictionary,
        inlines,
        ..
    } => matches!(inlines.as_slice(), [CllInline::Text(_)]),
    CllInline::LanguageSpan {
        kind: CllLanguageSpanKind::JboPhrase,
        inlines,
        ..
    } => matches!(inlines.as_slice(), [CllInline::Text(_)]),
    _ => false,
}))]
pub(crate) fn cmavo_or_gismu_cell_inlines_from<'word>(
    text: &str,
    mut target_for: impl FnMut(&str) -> Option<&'word str>,
) -> Vec<CllInline> {
    if text.is_empty() {
        return Vec::new();
    }
    if let Some(word) = bracketed_dictionary_word(text) {
        return match target_for(word) {
            Some(target) if !target.is_empty() => vec![
                CllInline::Text("[".to_owned()),
                dictionary_word_link(target, word),
                CllInline::Text("]".to_owned()),
            ],
            _ => vec![CllInline::Text(text.to_owned())],
        };
    }
    if is_dictionary_token(text) {
        return match target_for(text) {
            Some(target) if !target.is_empty() => vec![dictionary_word_link(target, text)],
            Some(_) => vec![CllInline::Text(text.to_owned())],
            None => vec![lojban_word_span(text)],
        };
    }
    vec![CllInline::Text(text.to_owned())]
}

/// The inlines for one cmavo or gismu cell in the shipped dictionary.
#[requires(text == text.trim())]
#[expensive_ensures(cmavo_cell_joined_text(&ret) == text)]
pub(crate) fn cmavo_or_gismu_cell_inlines(text: &str) -> Vec<CllInline> {
    cmavo_or_gismu_cell_inlines_from(text, |candidate| shipped_dictionary_link_target(candidate))
}

/// The inlines for a cell whose mark is one bare lujvo.
///
/// A dictionary hit uses the ordinary link. A miss stores a marked span, so
/// the word stays searchable. A bracketed miss keeps the brackets outside
/// that span. An empty target stays one text node.
#[requires(text == text.trim())]
#[expensive_ensures(cmavo_cell_joined_text(&ret) == text)]
#[ensures(ret.iter().filter(|inline| matches!(inline, CllInline::Link { .. })).count() <= 1)]
#[ensures(word_cell_shape(&ret))]
pub(crate) fn lujvo_cell_inlines(text: &str) -> Vec<CllInline> {
    if text.is_empty() {
        return Vec::new();
    }
    if let Some(word) = bracketed_dictionary_word(text) {
        return match shipped_dictionary_link_target(word) {
            Some(target) if !target.is_empty() => vec![
                CllInline::Text("[".to_owned()),
                dictionary_word_link(target, word),
                CllInline::Text("]".to_owned()),
            ],
            Some(_) => vec![CllInline::Text(text.to_owned())],
            None => vec![
                CllInline::Text("[".to_owned()),
                marked_valsi_span(word),
                CllInline::Text("]".to_owned()),
            ],
        };
    }
    if is_dictionary_token(text) {
        return match shipped_dictionary_link_target(text) {
            Some(target) if !target.is_empty() => vec![dictionary_word_link(target, text)],
            Some(_) => vec![CllInline::Text(text.to_owned())],
            None => vec![marked_valsi_span(text)],
        };
    }
    cmavo_or_gismu_cell_inlines(text)
}

#[requires(!target.is_empty())]
#[requires(is_dictionary_token(shown))]
#[ensures(matches!(ret, CllInline::Link { kind: CllLinkKind::Dictionary, .. }))]
fn dictionary_word_link(target: &str, shown: &str) -> CllInline {
    CllInline::Link {
        target: target.to_owned(),
        inlines: vec![CllInline::Text(shown.to_owned())],
        kind: CllLinkKind::Dictionary,
    }
}

#[requires(!text.is_empty())]
#[ensures(matches!(
    &ret,
    CllInline::LanguageSpan {
        kind: CllLanguageSpanKind::JboPhrase,
        language: Some(language),
        inlines,
    } if language == "jbo" && matches!(inlines.as_slice(), [CllInline::Text(shown)] if shown == text)
))]
fn lojban_phrase_span(text: &str) -> CllInline {
    CllInline::LanguageSpan {
        kind: CllLanguageSpanKind::JboPhrase,
        language: Some("jbo".to_owned()),
        inlines: vec![CllInline::Text(text.to_owned())],
    }
}

#[requires(is_dictionary_token(text))]
#[ensures(matches!(
    ret,
    CllInline::LanguageSpan {
        kind: CllLanguageSpanKind::JboPhrase,
        ..
    }
))]
fn lojban_word_span(text: &str) -> CllInline {
    lojban_phrase_span(text)
}

/// One unlinked marked word.
///
/// The span keeps `text`. Word search reads this kind. The HTML class is
/// the same class as a Lojban phrase.
#[requires(!text.is_empty())]
#[ensures(matches!(
    &ret,
    CllInline::LanguageSpan {
        kind: CllLanguageSpanKind::MarkedValsi,
        language: Some(language),
        inlines,
    } if language == "jbo" && matches!(inlines.as_slice(), [CllInline::Text(shown)] if shown == text)
))]
pub(crate) fn marked_valsi_span(text: &str) -> CllInline {
    CllInline::LanguageSpan {
        kind: CllLanguageSpanKind::MarkedValsi,
        language: Some("jbo".to_owned()),
        inlines: vec![CllInline::Text(text.to_owned())],
    }
}

/// The inlines for a cmavo or gismu cell that an opt-out mark leaves unlinked.
///
/// A dictionary token becomes one marked span, so the word stays searchable.
/// A bracketed word keeps `[` and `]` outside that span. Any other text stays
/// one text node. `-----` is that text node. The cell does not gain a link.
#[requires(text == text.trim())]
#[expensive_ensures(cmavo_cell_joined_text(&ret) == text)]
#[ensures(word_cell_shape(&ret))]
#[ensures(match ret.as_slice() {
    [] => text.is_empty(),
    [CllInline::Text(shown)] => {
        shown == text
            && bracketed_dictionary_word(text).is_none()
            && !is_dictionary_token(text)
    }
    [CllInline::LanguageSpan {
        kind: CllLanguageSpanKind::MarkedValsi,
        inlines,
        ..
    }] => {
        is_dictionary_token(text)
            && bracketed_dictionary_word(text).is_none()
            && matches!(inlines.as_slice(), [CllInline::Text(shown)] if shown == text)
    }
    [CllInline::Text(open), CllInline::LanguageSpan {
        kind: CllLanguageSpanKind::MarkedValsi,
        inlines,
        ..
    }, CllInline::Text(close)] => {
        open == "["
            && close == "]"
            && matches!(
                inlines.as_slice(),
                [CllInline::Text(shown)] if bracketed_dictionary_word(text) == Some(shown.as_str())
            )
    }
    _ => false,
})]
pub(crate) fn marked_unlinked_cell_inlines(text: &str) -> Vec<CllInline> {
    if text.is_empty() {
        return Vec::new();
    }
    if let Some(word) = bracketed_dictionary_word(text) {
        return vec![
            CllInline::Text("[".to_owned()),
            marked_valsi_span(word),
            CllInline::Text("]".to_owned()),
        ];
    }
    if is_dictionary_token(text) {
        return vec![marked_valsi_span(text)];
    }
    vec![CllInline::Text(text.to_owned())]
}

/// One part of a cell made of adjacent valsi elements.
///
/// `link` is false when the cell or this part opts out. A dictionary hit
/// becomes one link. Every other part becomes one Lojban span. The span
/// keeps `text` when `text` is not a dictionary token.
#[requires(!text.is_empty())]
#[ensures(match &ret {
    CllInline::Link {
        kind: CllLinkKind::Dictionary,
        target,
        inlines,
    } => {
        !target.is_empty()
            && matches!(inlines.as_slice(), [CllInline::Text(shown)] if shown == text && is_dictionary_token(shown))
    }
    CllInline::LanguageSpan {
        kind: CllLanguageSpanKind::JboPhrase,
        inlines,
        ..
    } => matches!(inlines.as_slice(), [CllInline::Text(shown)] if shown == text && !shown.is_empty()),
    _ => false,
})]
pub(crate) fn cmavo_valsi_part_inline(text: &str, link: bool) -> CllInline {
    if link
        && is_dictionary_token(text)
        && let Some(target) = shipped_dictionary_link_target(text)
        && !target.is_empty()
    {
        return dictionary_word_link(target, text);
    }
    lojban_phrase_span(text)
}

/// The characters a reader sees in one cmavo cell, with no added spaces.
#[requires(true)]
#[ensures(true)]
fn cmavo_cell_joined_text(inlines: &[CllInline]) -> String {
    let mut joined = String::new();
    for inline in inlines {
        match inline {
            CllInline::Text(text) => joined.push_str(text),
            CllInline::Link { inlines, .. } | CllInline::LanguageSpan { inlines, .. } => {
                for child in inlines {
                    if let CllInline::Text(text) = child {
                        joined.push_str(text);
                    }
                }
            }
            _ => {}
        }
    }
    joined
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn link_target_matches_exact_text_or_outer_pause_dots_only() {
        let both = [".ui", "ui", "..ui", "klama", "ui'"];
        assert_eq!(dictionary_link_target(".ui", both), Some(".ui"));
        assert_eq!(dictionary_link_target("ui", both), Some("ui"));
        assert_eq!(dictionary_link_target("..ui", both), Some("..ui"));
        assert_eq!(dictionary_link_target(".ui.", both), Some("ui"));
        assert_eq!(dictionary_link_target("klama", both), Some("klama"));
        assert_eq!(dictionary_link_target("Klama", both), None);
        assert_eq!(dictionary_link_target("uih", both), None);
        assert_eq!(dictionary_link_target("u.i", both), None);
        assert_eq!(dictionary_link_target("", both), None);
        assert_eq!(dictionary_link_target("...", both), None);

        assert_eq!(dictionary_link_target("ui", [".ui"]), Some(".ui"));
        assert_eq!(dictionary_link_target(".ui", [".ui"]), Some(".ui"));
        assert_eq!(dictionary_link_target(".ui", ["ui"]), Some("ui"));
        assert_eq!(dictionary_link_target("ui", [".ui", "..ui"]), Some(".ui"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shipped_dictionary_keeps_case_and_h_distinct() {
        assert_eq!(shipped_dictionary_link_target("coi"), Some("coi"));
        assert_eq!(shipped_dictionary_link_target("klama"), Some("klama"));
        assert_eq!(shipped_dictionary_link_target("jundi"), Some("jundi"));
        assert_eq!(shipped_dictionary_link_target("do'e"), Some("do'e"));
        assert_eq!(shipped_dictionary_link_target(".ui"), Some("ui"));
        assert_eq!(shipped_dictionary_link_target("Klama"), None);
        assert_eq!(shipped_dictionary_link_target("uih"), None);
        assert_eq!(shipped_dictionary_link_target("dohe"), None);
        assert_eq!(shipped_dictionary_link_target("ui'"), None);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cell_inlines_link_a_known_compound_and_keep_an_unknown_one_as_text() {
        let words = ["roinai", "ui"];
        let cell = |text| {
            cmavo_or_gismu_cell_inlines_from(text, |candidate| {
                dictionary_link_target(candidate, words)
            })
        };
        assert!(matches!(
            cell("roinai").as_slice(),
            [CllInline::Link {
                target,
                inlines,
                kind: CllLinkKind::Dictionary,
            }] if target == "roinai"
                && matches!(inlines.as_slice(), [CllInline::Text(text)] if text == "roinai")
        ));
        assert!(matches!(
            cell("pino'o").as_slice(),
            [CllInline::LanguageSpan {
                kind: CllLanguageSpanKind::JboPhrase,
                inlines,
                ..
            }] if matches!(inlines.as_slice(), [CllInline::Text(text)] if text == "pino'o")
        ));
        assert!(matches!(
            cell(".ui").as_slice(),
            [CllInline::Link {
                target,
                inlines,
                kind: CllLinkKind::Dictionary,
            }] if target == "ui"
                && matches!(inlines.as_slice(), [CllInline::Text(text)] if text == ".ui")
        ));
        assert!(matches!(
            cell("Klama").as_slice(),
            [CllInline::LanguageSpan {
                kind: CllLanguageSpanKind::JboPhrase,
                inlines,
                ..
            }] if matches!(inlines.as_slice(), [CllInline::Text(text)] if text == "Klama")
        ));
        assert_eq!(cell("[jundi]"), vec![CllInline::Text("[jundi]".to_owned())]);
        assert!(cell("").is_empty());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn empty_dictionary_target_is_rejected() {
        let bracket = std::panic::catch_unwind(|| {
            new!(CllCmavoWordCell::Bracket {
                word: "coi",
                target: Some(""),
            })
        });
        assert!(
            panic_is_type_invariant(bracket),
            "an empty bracket target must fail the type invariant"
        );
        let token = std::panic::catch_unwind(|| {
            new!(CllCmavoWordCell::Token {
                text: "coi",
                target: Some(""),
            })
        });
        assert!(
            panic_is_type_invariant(token),
            "an empty token target must fail the type invariant"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn empty_link_target_is_not_a_word_cell() {
        let single = vec![CllInline::Link {
            target: String::new(),
            inlines: vec![CllInline::Text("coi".to_owned())],
            kind: CllLinkKind::Dictionary,
        }];
        assert!(cmavo_word_cell(&single).is_none());

        let bracket = vec![
            CllInline::Text("[".to_owned()),
            CllInline::Link {
                target: String::new(),
                inlines: vec![CllInline::Text("coi".to_owned())],
                kind: CllLinkKind::Dictionary,
            },
            CllInline::Text("]".to_owned()),
        ];
        assert!(cmavo_word_cell(&bracket).is_none());

        let dot = vec![CllInline::Link {
            target: String::new(),
            inlines: vec![CllInline::Text(".".to_owned())],
            kind: CllLinkKind::Dictionary,
        }];
        assert!(cmavo_word_cell(&dot).is_none());

        let dots = vec![CllInline::Link {
            target: String::new(),
            inlines: vec![CllInline::Text("...".to_owned())],
            kind: CllLinkKind::Dictionary,
        }];
        assert!(cmavo_word_cell(&dots).is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_part_rejects_outer_whitespace_and_a_line_ending() {
        let rejected = ["", "    ", " a", "a ", "a  \nb", "a\nb", "\n==="];
        for text in rejected {
            let unlinked =
                std::panic::catch_unwind(|| new!(CllCmavoWordPart { text, target: None }));
            assert!(
                panic_is_type_invariant(unlinked),
                "part {text:?} must fail the type invariant"
            );
        }
        let empty_target = std::panic::catch_unwind(|| {
            new!(CllCmavoWordPart {
                text: "coi",
                target: Some(""),
            })
        });
        assert!(panic_is_type_invariant(empty_target));
        let not_a_token = std::panic::catch_unwind(|| {
            new!(CllCmavoWordPart {
                text: "mi,o",
                target: Some("mi,o"),
            })
        });
        assert!(panic_is_type_invariant(not_a_token));
        let linked = new!(CllCmavoWordPart {
            text: "roi",
            target: Some("roi"),
        });
        assert_eq!(linked.text, "roi");
        assert_eq!(linked.target, Some("roi"));
        let plain = new!(CllCmavoWordPart {
            text: "mi,o",
            target: None,
        });
        assert_eq!(plain.text, "mi,o");
        let spaced = vec![
            CllInline::LanguageSpan {
                kind: CllLanguageSpanKind::JboPhrase,
                language: None,
                inlines: vec![CllInline::Text(" a".to_owned())],
            },
            CllInline::LanguageSpan {
                kind: CllLanguageSpanKind::JboPhrase,
                language: None,
                inlines: vec![CllInline::Text("nai".to_owned())],
            },
        ];
        assert!(cmavo_word_parts(&spaced).is_none());
        let broken = vec![
            CllInline::LanguageSpan {
                kind: CllLanguageSpanKind::JboPhrase,
                language: None,
                inlines: vec![CllInline::Text("a  \nb".to_owned())],
            },
            CllInline::LanguageSpan {
                kind: CllLanguageSpanKind::JboPhrase,
                language: None,
                inlines: vec![CllInline::Text("z".to_owned())],
            },
        ];
        assert!(cmavo_word_parts(&broken).is_none());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn empty_lookup_target_is_plain_text() {
        let cell = |text| cmavo_or_gismu_cell_inlines_from(text, |_| Some(""));
        assert_eq!(cell("coi"), vec![CllInline::Text("coi".to_owned())]);
        assert_eq!(cell("[jundi]"), vec![CllInline::Text("[jundi]".to_owned())]);
        assert_eq!(cell("-----"), vec![CllInline::Text("-----".to_owned())]);
    }

    #[requires(true)]
    #[ensures(true)]
    fn panic_is_type_invariant<T>(result: Result<T, Box<dyn std::any::Any + Send>>) -> bool {
        let Err(payload) = result else {
            return false;
        };
        payload
            .downcast_ref::<String>()
            .is_some_and(|message| message.contains("type invariant"))
            || payload
                .downcast_ref::<&str>()
                .is_some_and(|message| message.contains("type invariant"))
    }
}
