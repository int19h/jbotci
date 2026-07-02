//! Shared semantic builder facade types.

use std::fmt;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_dictionary::{Dictionary, WordType, normalize_lookup_query};

use crate::model::SemanticObjectId;
use crate::references::ReferenceAnalysisError;

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticsError {
    pub kind: SemanticsErrorKind,
    pub message: String,
}

impl From<ReferenceAnalysisError> for SemanticsError {
    #[requires(true)]
    #[ensures(ret.kind == SemanticsErrorKind::ReferenceAnalysis)]
    fn from(error: ReferenceAnalysisError) -> Self {
        Self {
            kind: SemanticsErrorKind::ReferenceAnalysis,
            message: error.to_string(),
        }
    }
}

impl SemanticsError {
    #[requires(true)]
    #[ensures(ret.kind == SemanticsErrorKind::MissingSyntaxNode)]
    pub(crate) fn missing_syntax_node() -> Self {
        Self {
            kind: SemanticsErrorKind::MissingSyntaxNode,
            message: "semantic builder could not find a syntax node recorded by reference analysis"
                .to_owned(),
        }
    }

    #[requires(true)]
    #[ensures(ret.kind == SemanticsErrorKind::DuplicateObject)]
    pub(crate) fn duplicate_object(id: SemanticObjectId) -> Self {
        Self {
            kind: SemanticsErrorKind::DuplicateObject,
            message: format!("semantic builder attempted to insert duplicate object ID {id}"),
        }
    }

    #[requires(!message.is_empty())]
    #[ensures(ret.kind == SemanticsErrorKind::InvalidGraph)]
    pub(crate) fn invalid_graph(message: String) -> Self {
        Self {
            kind: SemanticsErrorKind::InvalidGraph,
            message: format!("semantic graph invariant failed: {message}"),
        }
    }
}

impl fmt::Display for SemanticsError {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SemanticsError {}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticsErrorKind {
    ReferenceAnalysis,
    MissingSyntaxNode,
    DuplicateObject,
    InvalidGraph,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
pub struct SemanticBuildOptions<'a> {
    pub source_text: Option<&'a str>,
    pub story_time: bool,
}

impl Default for SemanticBuildOptions<'_> {
    #[requires(true)]
    #[ensures(ret.source_text.is_none())]
    #[ensures(!ret.story_time)]
    fn default() -> Self {
        Self {
            source_text: None,
            story_time: false,
        }
    }
}

#[requires(!relation.is_empty())]
#[ensures(true)]
pub fn dictionary_relation_place_count(
    dictionary: &Dictionary<'_>,
    relation: &str,
) -> Option<usize> {
    let normalized = normalize_lookup_query(relation);
    let entry = dictionary.lookup_word(&normalized)?;
    if !word_type_is_brivla_like(entry.word_type) {
        return None;
    }
    let keyword_count = (!entry.place_keywords.is_empty()).then_some(entry.place_keywords.len());
    let definition_count = dictionary_definition_place_count(entry.definition);
    keyword_count.max(definition_count)
}

#[requires(true)]
#[ensures(true)]
fn dictionary_definition_place_count(definition: &str) -> Option<usize> {
    // Jbovlaste definitions encode place structure in prose rather than typed
    // metadata. Pin the conventions seen in the export: direct `$x_1$` markers,
    // braced `$x_{1}$` markers, angle markers such as `<1>`/`⟨1⟩`, and lujvo
    // component variables such as `$x_1$` plus `$z_1$`.
    let mut max_place = 0usize;
    let mut chars = definition.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '$' {
            if chars.next() != Some('x') || chars.next() != Some('_') {
                continue;
            }
            let braced = chars.peek() == Some(&'{');
            if braced {
                chars.next();
            }
            let mut digits = String::new();
            while let Some(next) = chars.peek().copied() {
                if next.is_ascii_digit() {
                    digits.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            if braced && chars.next() != Some('}') {
                continue;
            }
            if chars.next() != Some('$') {
                continue;
            }
            if let Ok(place) = digits.parse::<usize>() {
                max_place = max_place.max(place);
            }
            continue;
        }
        if character != '<' && character != '⟨' {
            continue;
        }
        let mut digits = String::new();
        while let Some(next) = chars.peek().copied() {
            if next.is_ascii_digit() {
                digits.push(next);
                chars.next();
            } else {
                break;
            }
        }
        let Some(closing) = chars.next() else {
            continue;
        };
        if (character == '<' && closing != '>') || (character == '⟨' && closing != '⟩') {
            continue;
        }
        if let Ok(place) = digits.parse::<usize>() {
            max_place = max_place.max(place);
        }
    }
    dictionary_lujvo_definition_place_count(definition)
        .into_iter()
        .chain((max_place > 0).then_some(max_place))
        .max()
}

#[requires(true)]
#[ensures(true)]
fn dictionary_lujvo_definition_place_count(definition: &str) -> Option<usize> {
    // Lujvo definitions use x-places for the resulting predicate and non-x
    // variables for component places. Jbovlaste's visible place count is the
    // highest x-place plus each distinct non-x component variable.
    let place_ids = collect_definition_place_ids(definition);
    if place_ids.is_empty() {
        return None;
    }
    let max_x_place = place_ids
        .iter()
        .filter(|place_id| place_id.letter == "x")
        .map(|place_id| place_id.index)
        .max()
        .unwrap_or(0);
    let non_x_count = place_ids
        .iter()
        .filter(|place_id| place_id.letter != "x")
        .count();
    Some(max_x_place + non_x_count)
}

#[requires(true)]
#[ensures(true)]
fn collect_definition_place_ids(definition: &str) -> Vec<DefinitionPlaceId> {
    let mut place_ids = Vec::new();
    let mut remaining = definition;
    while let Some(open) = remaining.find('$') {
        let after_open = &remaining[open + 1..];
        let Some(close) = after_open.find('$') else {
            break;
        };
        let block = &after_open[..close];
        if let Some(place_id) = first_definition_place_id(block)
            && !place_ids.contains(&place_id)
        {
            place_ids.push(place_id);
        }
        remaining = &after_open[close + 1..];
    }
    place_ids
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|place_id| !place_id.letter.is_empty() && place_id.index > 0))]
fn first_definition_place_id(block: &str) -> Option<DefinitionPlaceId> {
    let mut remaining = block;
    while !remaining.is_empty() {
        if let Some((letter, rest)) = try_definition_place_var(remaining) {
            let (digits, rest_digits) = span_ascii_digits(rest);
            if !digits.is_empty() {
                if let Some(stripped) = rest_digits.strip_prefix('}') {
                    if let Some(place_id) = definition_place_id(letter, digits) {
                        return Some(place_id);
                    }
                    remaining = stripped;
                    continue;
                }
                if let Some(place_id) = definition_place_id(letter, digits) {
                    return Some(place_id);
                }
            }
        }
        let mut chars = remaining.chars();
        let _ = chars.next();
        remaining = chars.as_str();
    }
    None
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(letter, _)| !letter.is_empty()))]
fn try_definition_place_var(input: &str) -> Option<(&str, &str)> {
    let (letters, rest) = span_ascii_lowercase_letters(input);
    if letters.len() >= 2
        && letters.chars().all(is_definition_var_letter)
        && let Some(after_prefix) = rest.strip_prefix("_{")
    {
        return Some((letters, after_prefix));
    }
    if letters.len() >= 2
        && letters.chars().all(is_definition_var_letter)
        && let Some(after_prefix) = rest.strip_prefix('_')
    {
        return Some((letters, after_prefix));
    }
    let mut chars = input.chars();
    let character = chars.next()?;
    if !is_definition_var_letter(character) {
        return None;
    }
    let rest = chars.as_str();
    rest.strip_prefix("_{")
        .or_else(|| rest.strip_prefix('_'))
        .map(|after_prefix| (&input[..character.len_utf8()], after_prefix))
}

#[requires(true)]
#[ensures(true)]
fn span_ascii_lowercase_letters(input: &str) -> (&str, &str) {
    let end = input
        .char_indices()
        .find_map(|(index, character)| (!character.is_ascii_lowercase()).then_some(index))
        .unwrap_or(input.len());
    input.split_at(end)
}

#[requires(true)]
#[ensures(true)]
fn span_ascii_digits(input: &str) -> (&str, &str) {
    let end = input
        .char_indices()
        .find_map(|(index, character)| (!character.is_ascii_digit()).then_some(index))
        .unwrap_or(input.len());
    input.split_at(end)
}

#[requires(true)]
#[ensures(true)]
fn is_definition_var_letter(character: char) -> bool {
    matches!(
        character,
        'a' | 'b'
            | 'c'
            | 'd'
            | 'e'
            | 'f'
            | 'g'
            | 'i'
            | 'j'
            | 'k'
            | 'l'
            | 'm'
            | 'n'
            | 'o'
            | 'p'
            | 'r'
            | 's'
            | 't'
            | 'u'
            | 'v'
            | 'x'
            | 'z'
    )
}

#[requires(!letter.is_empty())]
#[requires(!digits.is_empty())]
#[ensures(ret.as_ref().is_none_or(|place_id| place_id.letter == letter && place_id.index > 0))]
fn definition_place_id(letter: &str, digits: &str) -> Option<DefinitionPlaceId> {
    let index = digits.parse::<usize>().ok()?;
    (index > 0).then(|| {
        new!(DefinitionPlaceId {
            letter: letter.to_owned(),
            index,
        })
    })
}

#[requires(true)]
#[ensures(true)]
fn word_type_is_brivla_like(word_type: WordType) -> bool {
    matches!(
        word_type,
        WordType::Gismu
            | WordType::ExperimentalGismu
            | WordType::Lujvo
            | WordType::ZeiLujvo
            | WordType::ObsoleteZeiLujvo
            | WordType::Fuivla
            | WordType::ObsoleteFuivla
    )
}

#[invariant(!letter.is_empty())]
#[invariant(*index > 0)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct DefinitionPlaceId {
    letter: String,
    index: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn definition_place_count_pins_jbovlaste_marker_conventions() {
        assert_eq!(
            dictionary_definition_place_count("agent $x_1$ acts on $x_{2}$ at <3> and ⟨4⟩"),
            Some(4)
        );
        assert_eq!(
            dictionary_definition_place_count("lujvo result $x_1$ from $z_1$ and $r_1$"),
            Some(3)
        );
        assert_eq!(
            dictionary_definition_place_count("repeated component $x_1$ and $z_1$ and $z_1$"),
            Some(2)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn first_definition_place_id_continues_after_invalid_braced_marker() {
        assert_eq!(
            first_definition_place_id("x_{0} noise x_{3}"),
            Some(new!(DefinitionPlaceId {
                letter: "x".to_owned(),
                index: 3,
            }))
        );
        assert_eq!(
            dictionary_definition_place_count("bad first marker $x_{0} noise x_{3}$"),
            Some(3)
        );
    }
}
