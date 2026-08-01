//! Definition place maps: the jbovlaste `$x_{1}$`-style variable blocks in
//! dictionary definition and notes text, resolved to one-based dictionary
//! places.
//!
//! This module is the rendering-independent core shared by every surface that
//! presents definition text: it parses the variable blocks of one definition
//! into a [`DefinitionPlaceMap`] and segments definition/notes lines into
//! typed [`DefinitionPlaceSegment`] runs (prose text, resolved place, or an
//! unmapped variable). Glyph styling and final text rendering live in
//! `jbotci-output`, which is implemented on top of these segments.
//!
//! The two line kinds follow different alias rules, matching jbovlaste
//! conventions: definition lines only resolve variables that lead a
//! definition block (a later block led by the same ID can still establish its
//! own displayed place), while notes lines resolve every ID in every
//! definition block, with the first occurrence winning. Unmapped variables
//! are preserved verbatim in definition lines but surface as
//! [`DefinitionPlaceSegment::UnmappedVariable`] in notes lines so callers can
//! decide how to present them.

use bityzba::{data, invariant, new, requires};

#[invariant(!letter.is_empty())]
#[invariant(*index > 0, "dictionary place indices are one-based")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PlaceId {
    letter: String,
    index: usize,
}

#[invariant(*place > 0, "mapped dictionary places are one-based")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct DefinitionPlaceMapping {
    id: PlaceId,
    place: usize,
}

#[invariant(!ids.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct DefinitionPlaceBlock {
    ids: Vec<PlaceId>,
}

/// Place assignments and aliases established by all variable blocks in one definition.
#[invariant(
    definition_places
        .iter()
        .all(|mapping| mapping.id.letter != "x" || mapping.place == mapping.id.index),
    "x_N definition blocks must always retain place N"
)]
#[invariant(
    aliases
        .iter()
        .all(|mapping| mapping.id.letter != "x" || mapping.place == mapping.id.index),
    "x_N aliases must always retain place N"
)]
#[expensive_invariant(definition_places.iter().enumerate().all(|(index, mapping)| {
    definition_places[..index]
        .iter()
        .all(|earlier| earlier.id != mapping.id)
}), "definition block place IDs must be unique")]
#[expensive_invariant(
    aliases.iter().enumerate().all(|(index, mapping)| {
        aliases[..index]
            .iter()
            .all(|earlier| earlier.id != mapping.id)
    }),
    "definition place aliases must be unique"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionPlaceMap {
    // Leading IDs determine how definition blocks display. Secondary aliases are deliberately
    // excluded so a later block led by that ID can still establish its own displayed place.
    definition_places: Vec<DefinitionPlaceMapping>,
    // Notes use every ID in every definition block, with the first occurrence winning.
    aliases: Vec<DefinitionPlaceMapping>,
}

impl DefinitionPlaceMap {
    /// Build the single place map shared by every definition and notes line in an entry.
    #[requires(true)]
    #[ensures(
        ret.definition_places
            .iter()
            .all(|mapping| mapping.id.letter != "x" || mapping.place == mapping.id.index)
    )]
    #[ensures(
        ret.aliases
            .iter()
            .all(|mapping| mapping.id.letter != "x" || mapping.place == mapping.id.index)
    )]
    pub fn from_definition(definition: &str) -> Self {
        let normalized = normalize_place_block_separators(definition);
        let blocks = collect_definition_place_blocks(&normalized);
        build_definition_place_map(&blocks)
    }

    #[requires(true)]
    #[ensures(id.letter == "x" -> ret == Some(id.index))]
    fn place_for(&self, id: &PlaceId) -> Option<usize> {
        if id.letter == "x" {
            return Some(id.index);
        }
        self.aliases
            .iter()
            .find(|mapping| mapping.id == *id)
            .map(|mapping| mapping.place)
    }

    #[requires(true)]
    #[ensures(id.letter == "x" -> ret == Some(id.index))]
    fn definition_place_for(&self, id: &PlaceId) -> Option<usize> {
        if id.letter == "x" {
            return Some(id.index);
        }
        self.definition_places
            .iter()
            .find(|mapping| mapping.id == *id)
            .map(|mapping| mapping.place)
    }
}

/// One typed segment of a definition or notes line.
///
/// Segments never carry rendered glyphs: `Place` is the one-based dictionary
/// place the `$...$` variable block or `x<N>` subscript marker resolved to,
/// and `UnmappedVariable` is a notes-line variable that the entry's
/// definition blocks never mapped, preserved so each consumer can render it
/// in its own style. Adjacent `Place` segments are deliberately not merged:
/// `$x_1$$x_1$` is two markers in the source and must stay distinguishable
/// from a single one. Adjacent `Text` runs are merged.
#[invariant(::Text(text) => !text.is_empty())]
#[invariant(::Place(place) => *place > 0, "dictionary place indices are one-based")]
#[invariant(::UnmappedVariable { letter, index } => !letter.is_empty() && *index > 0)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionPlaceSegment {
    /// Verbatim prose (with `$=$` block-separator typos normalized to `=`).
    Text(String),
    /// A variable block or subscript marker resolved to a one-based place.
    Place(usize),
    /// A notes-line variable the definition's blocks never mapped.
    UnmappedVariable { letter: String, index: usize },
}

/// Split one definition line into typed segments using its entry's prebuilt place map.
///
/// Unmapped variables in definition lines stay verbatim `$...$` text: a
/// definition block that does not lead with a mapped ID establishes nothing.
#[requires(true)]
#[ensures(input.is_empty() -> ret.is_empty())]
pub fn definition_place_segments_for_definition_line(
    input: &str,
    place_map: &DefinitionPlaceMap,
) -> Vec<DefinitionPlaceSegment> {
    definition_place_segments_for_line(input, place_map, DefinitionPlaceLineKind::Definition)
}

/// Split one notes line into typed segments without deriving any new place
/// assignments from it. Variables the definition never mapped surface as
/// [`DefinitionPlaceSegment::UnmappedVariable`].
#[requires(true)]
#[ensures(input.is_empty() -> ret.is_empty())]
pub fn definition_place_segments_for_notes_line(
    input: &str,
    place_map: &DefinitionPlaceMap,
) -> Vec<DefinitionPlaceSegment> {
    definition_place_segments_for_line(input, place_map, DefinitionPlaceLineKind::Notes)
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DefinitionPlaceLineKind {
    Definition,
    Notes,
}

#[requires(true)]
#[ensures(input.is_empty() -> ret.is_empty())]
fn definition_place_segments_for_line(
    input: &str,
    place_map: &DefinitionPlaceMap,
    line_kind: DefinitionPlaceLineKind,
) -> Vec<DefinitionPlaceSegment> {
    let normalized = normalize_place_block_separators(input);
    let mut output = Vec::new();
    let mut remaining = normalized.as_str();
    while !remaining.is_empty() {
        let Some(open_index) = remaining.find('$') else {
            append_text_segments(&mut output, remaining);
            break;
        };
        append_text_segments(&mut output, &remaining[..open_index]);
        let after_open = &remaining[open_index + 1..];
        let Some(close_index) = after_open.find('$') else {
            append_text_segments(&mut output, &remaining[open_index..]);
            break;
        };
        let block_text = &after_open[..close_index];
        if let Some((place_id, _)) = find_place_var(block_text) {
            let place = match line_kind {
                DefinitionPlaceLineKind::Definition => place_map.definition_place_for(&place_id),
                DefinitionPlaceLineKind::Notes => place_map.place_for(&place_id),
            };
            if let Some(place) = place {
                push_segment(&mut output, new!(DefinitionPlaceSegment::Place(place)));
            } else if line_kind == DefinitionPlaceLineKind::Notes {
                let data = place_id.into_data();
                push_segment(
                    &mut output,
                    new!(DefinitionPlaceSegment::UnmappedVariable {
                        letter: data.letter,
                        index: data.index,
                    }),
                );
            } else {
                push_text_segment(&mut output, format!("${block_text}$"));
            }
        } else {
            push_text_segment(&mut output, format!("${block_text}$"));
        }
        remaining = &after_open[close_index + 1..];
    }
    output
}

/// Split plain prose at `x<N>` subscript markers, which are an alternative
/// jbovlaste spelling of `$x_{N}$` and resolve to the same one-based place.
#[requires(true)]
#[ensures(input.is_empty() -> output.len() == old(output.len()))]
fn append_text_segments(output: &mut Vec<DefinitionPlaceSegment>, input: &str) {
    let mut text = String::new();
    let mut remaining = input;
    while !remaining.is_empty() {
        if let Some(after_x) = remaining.strip_prefix('x') {
            let (subscripts, rest) = span_subscript_digits(after_x);
            if subscripts.is_empty() {
                text.push('x');
                remaining = after_x;
                continue;
            }
            if let Some(place_index) = decode_subscript_digits(subscripts) {
                push_text_segment(output, std::mem::take(&mut text));
                push_segment(output, new!(DefinitionPlaceSegment::Place(place_index)));
            } else {
                text.push('x');
                text.push_str(subscripts);
            }
            remaining = rest;
            continue;
        }
        let mut chars = remaining.chars();
        if let Some(character) = chars.next() {
            text.push(character);
        }
        remaining = chars.as_str();
    }
    push_text_segment(output, text);
}

/// Append one text run, merging with a trailing [`DefinitionPlaceSegment::Text`].
#[requires(true)]
#[ensures(output.len() >= old(output.len()))]
fn push_text_segment(output: &mut Vec<DefinitionPlaceSegment>, text: String) {
    if text.is_empty() {
        return;
    }
    if matches!(
        output.last().map(DefinitionPlaceSegment::as_data),
        Some(data!(DefinitionPlaceSegment::Text(_)))
    ) {
        let last = output
            .pop()
            .expect("a trailing text segment was observed immediately before pop");
        let data!(DefinitionPlaceSegment::Text(mut merged_text)) = last.into_data() else {
            unreachable!("the trailing segment was observed to be text");
        };
        merged_text.push_str(&text);
        output.push(new!(DefinitionPlaceSegment::Text(merged_text)));
        return;
    }
    push_segment(output, new!(DefinitionPlaceSegment::Text(text)));
}

#[requires(true)]
#[ensures(output.len() == old(output.len()) + 1)]
fn push_segment(output: &mut Vec<DefinitionPlaceSegment>, segment: DefinitionPlaceSegment) {
    output.push(segment);
}

#[requires(true)]
#[ensures(true)]
fn span_subscript_digits(input: &str) -> (&str, &str) {
    let end = input
        .char_indices()
        .find_map(|(index, character)| (!is_subscript_digit(character)).then_some(index))
        .unwrap_or(input.len());
    input.split_at(end)
}

#[requires(true)]
#[ensures(true)]
fn decode_subscript_digits(input: &str) -> Option<usize> {
    if input.is_empty() {
        return None;
    }
    let mut value = 0usize;
    for character in input.chars() {
        let digit = subscript_digit_value(character)?;
        value = value.checked_mul(10)?.checked_add(digit)?;
    }
    Some(value)
}

#[requires(true)]
#[ensures(true)]
fn subscript_digit_value(character: char) -> Option<usize> {
    match character {
        '₀' => Some(0),
        '₁' => Some(1),
        '₂' => Some(2),
        '₃' => Some(3),
        '₄' => Some(4),
        '₅' => Some(5),
        '₆' => Some(6),
        '₇' => Some(7),
        '₈' => Some(8),
        '₉' => Some(9),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn is_subscript_digit(character: char) -> bool {
    matches!(
        character,
        '₀' | '₁' | '₂' | '₃' | '₄' | '₅' | '₆' | '₇' | '₈' | '₉'
    )
}

#[requires(true)]
#[ensures(true)]
fn normalize_place_block_separators(input: &str) -> String {
    input.replace("$=$", "=")
}

#[requires(true)]
#[ensures(input.is_empty() -> ret.is_empty())]
fn collect_definition_place_blocks(input: &str) -> Vec<DefinitionPlaceBlock> {
    let mut blocks = Vec::new();
    let mut remaining = input;
    while !remaining.is_empty() {
        let Some(open_index) = remaining.find('$') else {
            break;
        };
        let after_open = &remaining[open_index + 1..];
        let Some(close_index) = after_open.find('$') else {
            break;
        };
        let ids = collect_place_ids_in_block(&after_open[..close_index]);
        if !ids.is_empty() {
            blocks.push(new!(DefinitionPlaceBlock { ids }));
        }
        remaining = &after_open[close_index + 1..];
    }
    blocks
}

#[requires(true)]
#[ensures(input.is_empty() -> ret.is_empty())]
fn collect_place_ids_in_block(input: &str) -> Vec<PlaceId> {
    let mut ids = Vec::new();
    let mut remaining = input;
    while let Some((id, rest)) = find_place_var(remaining) {
        ids.push(id);
        remaining = rest;
    }
    ids
}

#[requires(true)]
#[ensures(true)]
fn build_definition_place_map(blocks: &[DefinitionPlaceBlock]) -> DefinitionPlaceMap {
    let mut used_places = Vec::new();
    for block in blocks {
        let first_id = block
            .ids
            .first()
            .expect("definition place blocks are non-empty by invariant");
        if first_id.letter == "x" && !used_places.contains(&first_id.index) {
            used_places.push(first_id.index);
        }
    }

    let mut definition_places: Vec<DefinitionPlaceMapping> = Vec::new();
    let mut aliases: Vec<DefinitionPlaceMapping> = Vec::new();
    let mut next_place = 1usize;
    for block in blocks {
        let first_id = block
            .ids
            .first()
            .expect("definition place blocks are non-empty by invariant");
        let existing_place = definition_places
            .iter()
            .find(|mapping| mapping.id == *first_id)
            .map(|mapping| mapping.place);
        let block_place = if let Some(existing_place) = existing_place {
            existing_place
        } else if first_id.letter == "x" {
            first_id.index
        } else {
            while used_places.contains(&next_place) {
                next_place = next_place
                    .checked_add(1)
                    .expect("the number of definition place blocks cannot exhaust usize");
            }
            let place = next_place;
            used_places.push(place);
            next_place = next_place
                .checked_add(1)
                .expect("the number of definition place blocks cannot exhaust usize");
            place
        };

        if existing_place.is_none() {
            definition_places.push(new!(DefinitionPlaceMapping {
                id: first_id.clone(),
                place: block_place,
            }));
        }

        for id in &block.ids {
            if aliases
                .iter()
                .any(|mapping: &DefinitionPlaceMapping| mapping.id == *id)
            {
                continue;
            }
            aliases.push(new!(DefinitionPlaceMapping {
                id: id.clone(),
                place: if id.letter == "x" {
                    id.index
                } else {
                    block_place
                },
            }));
        }
    }
    new!(DefinitionPlaceMap {
        definition_places,
        aliases,
    })
}

#[requires(true)]
#[ensures(true)]
fn find_place_var(input: &str) -> Option<(PlaceId, &str)> {
    if input.is_empty() || input.starts_with('$') {
        return None;
    }
    if let Some((letter, rest)) = try_multi_letter_var_brace(input) {
        let (digits, rest_digits) = span_ascii_digits(rest);
        if !digits.is_empty() {
            if let Some(after_close) = rest_digits.strip_prefix('}') {
                return Some((place_id(letter, digits)?, after_close));
            }
        }
    }
    if let Some((letter, rest)) = try_multi_letter_var(input) {
        let (digits, rest_digits) = span_ascii_digits(rest);
        if !digits.is_empty() {
            return Some((place_id(letter, digits)?, rest_digits));
        }
    }
    let mut chars = input.chars();
    let character = chars.next()?;
    let rest = chars.as_str();
    if is_var_letter(character) {
        if let Some(after_prefix) = rest.strip_prefix("_{") {
            let (digits, rest_digits) = span_ascii_digits(after_prefix);
            if !digits.is_empty() {
                if let Some(after_close) = rest_digits.strip_prefix('}') {
                    return Some((place_id(&character.to_string(), digits)?, after_close));
                }
            }
        }
        if let Some(after_prefix) = rest.strip_prefix('_') {
            let (digits, rest_digits) = span_ascii_digits(after_prefix);
            if !digits.is_empty() {
                return Some((place_id(&character.to_string(), digits)?, rest_digits));
            }
        }
    }
    find_place_var(rest)
}

#[requires(!letter.is_empty())]
#[requires(!digits.is_empty())]
#[ensures(ret.as_ref().is_none_or(|place_id| place_id.letter == letter))]
fn place_id(letter: &str, digits: &str) -> Option<PlaceId> {
    let index = digits.parse::<usize>().ok()?;
    if index == 0 {
        return None;
    }
    Some(new!(PlaceId {
        letter: letter.to_owned(),
        index,
    }))
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
fn try_multi_letter_var_brace(input: &str) -> Option<(&str, &str)> {
    let (letters, rest) = span_ascii_lowercase_letters(input);
    (letters.len() >= 2 && letters.chars().all(is_var_letter))
        .then(|| {
            rest.strip_prefix("_{")
                .map(|after_prefix| (letters, after_prefix))
        })
        .flatten()
}

#[requires(true)]
#[ensures(true)]
fn try_multi_letter_var(input: &str) -> Option<(&str, &str)> {
    let (letters, rest) = span_ascii_lowercase_letters(input);
    (letters.len() >= 2 && letters.chars().all(is_var_letter))
        .then(|| {
            rest.strip_prefix('_')
                .map(|after_prefix| (letters, after_prefix))
        })
        .flatten()
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
fn is_var_letter(character: char) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn preserves_terxra_appearance_order() {
        let definition = concat!(
            "$p_3$ makes an image of $p_2$ (object/concept) with result $p_1$ ",
            "(picture) in medium $p_4$."
        );
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            definition_place_segments_for_definition_line(definition, &place_map),
            vec![
                new!(DefinitionPlaceSegment::Place(1)),
                new!(DefinitionPlaceSegment::Text(
                    " makes an image of ".to_owned()
                )),
                new!(DefinitionPlaceSegment::Place(2)),
                new!(DefinitionPlaceSegment::Text(
                    " (object/concept) with result ".to_owned()
                )),
                new!(DefinitionPlaceSegment::Place(3)),
                new!(DefinitionPlaceSegment::Text(" (picture) in medium ".to_owned())),
                new!(DefinitionPlaceSegment::Place(4)),
                new!(DefinitionPlaceSegment::Text(".".to_owned())),
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fills_baldakyxahi_blocks_around_reserved_pin() {
        let definition = "$xa_1$ is a great sword for use against $xa_2$ by $x_3$.";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            definition_place_segments_for_definition_line(definition, &place_map),
            vec![
                new!(DefinitionPlaceSegment::Place(1)),
                new!(DefinitionPlaceSegment::Text(
                    " is a great sword for use against ".to_owned()
                )),
                new!(DefinitionPlaceSegment::Place(2)),
                new!(DefinitionPlaceSegment::Text(" by ".to_owned())),
                new!(DefinitionPlaceSegment::Place(3)),
                new!(DefinitionPlaceSegment::Text(".".to_owned())),
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn maps_bircidni_blocks_and_all_aliases() {
        let definition = "$c_1=b_1$ is an elbow of body $x_2=c_3=b_2$.";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            definition_place_segments_for_definition_line(definition, &place_map),
            vec![
                new!(DefinitionPlaceSegment::Place(1)),
                new!(DefinitionPlaceSegment::Text(" is an elbow of body ".to_owned())),
                new!(DefinitionPlaceSegment::Place(2)),
                new!(DefinitionPlaceSegment::Text(".".to_owned())),
            ]
        );
        assert_eq!(
            definition_place_segments_for_notes_line("$b_2$", &place_map),
            vec![new!(DefinitionPlaceSegment::Place(2))]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn resolves_bavlamdei_style_chained_aliases_to_the_leading_place() {
        let definition = "$d_1=b_1=l_1$ is today of $b_2$ at location $l_3$.";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            definition_place_segments_for_definition_line(definition, &place_map),
            vec![
                new!(DefinitionPlaceSegment::Place(1)),
                new!(DefinitionPlaceSegment::Text(" is today of ".to_owned())),
                new!(DefinitionPlaceSegment::Place(2)),
                new!(DefinitionPlaceSegment::Text(" at location ".to_owned())),
                new!(DefinitionPlaceSegment::Place(3)),
                new!(DefinitionPlaceSegment::Text(".".to_owned())),
            ]
        );
        assert_eq!(
            definition_place_segments_for_notes_line("$l_1$", &place_map),
            vec![new!(DefinitionPlaceSegment::Place(1))]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn resolves_plain_braced_x_marker_and_preserves_surrounding_prose() {
        let definition = "$x_1$ sees $x_{2}$.";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            definition_place_segments_for_definition_line(definition, &place_map),
            vec![
                new!(DefinitionPlaceSegment::Place(1)),
                new!(DefinitionPlaceSegment::Text(" sees ".to_owned())),
                new!(DefinitionPlaceSegment::Place(2)),
                new!(DefinitionPlaceSegment::Text(".".to_owned())),
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn notes_leave_unmapped_variables_unindexed() {
        let definition = "$x_1$ is a porch attached to building $x_2$.";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            definition_place_segments_for_notes_line("$x_2$ = $bartu_2$.", &place_map),
            vec![
                new!(DefinitionPlaceSegment::Place(2)),
                new!(DefinitionPlaceSegment::Text(" = ".to_owned())),
                new!(DefinitionPlaceSegment::UnmappedVariable {
                    letter: "bartu".to_owned(),
                    index: 2,
                }),
                new!(DefinitionPlaceSegment::Text(".".to_owned())),
            ]
        );
        assert_eq!(
            definition_place_segments_for_notes_line(
                "deleting $b_3$ must retain its source place",
                &place_map,
            ),
            vec![
                new!(DefinitionPlaceSegment::Text("deleting ".to_owned())),
                new!(DefinitionPlaceSegment::UnmappedVariable {
                    letter: "b".to_owned(),
                    index: 3,
                }),
                new!(DefinitionPlaceSegment::Text(
                    " must retain its source place".to_owned()
                )),
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shares_one_place_map_across_definition_lines() {
        let definition = "$a_1$ first.\n$c_1$ third.\n$x_2=b_1$ pinned second.";
        let place_map = DefinitionPlaceMap::from_definition(definition);
        let places = definition
            .lines()
            .map(|line| {
                definition_place_segments_for_definition_line(line, &place_map)
                    .into_iter()
                    .filter_map(|segment| match segment.into_data() {
                        data!(DefinitionPlaceSegment::Place(place)) => Some(place),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        assert_eq!(places, vec![vec![1], vec![3], vec![2]]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn repeated_first_id_does_not_consume_another_place() {
        let definition = "$d_1=c_1$ is today; $d_1=c_1$ is the day of $c_2$, standard $d_3$.";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            definition_place_segments_for_definition_line(definition, &place_map),
            vec![
                new!(DefinitionPlaceSegment::Place(1)),
                new!(DefinitionPlaceSegment::Text(" is today; ".to_owned())),
                new!(DefinitionPlaceSegment::Place(1)),
                new!(DefinitionPlaceSegment::Text(" is the day of ".to_owned())),
                new!(DefinitionPlaceSegment::Place(2)),
                new!(DefinitionPlaceSegment::Text(", standard ".to_owned())),
                new!(DefinitionPlaceSegment::Place(3)),
                new!(DefinitionPlaceSegment::Text(".".to_owned())),
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn definition_blocks_and_note_aliases_use_their_distinct_places() {
        let definition = concat!(
            "$b_1=c_2$ is a wind from direction $b_2$ with speed $b_3$, ",
            "shoving $c_2$ at locus $c_3$."
        );
        let place_map = DefinitionPlaceMap::from_definition(definition);

        let definition_places = definition_place_segments_for_definition_line(definition, &place_map)
            .into_iter()
            .filter_map(|segment| match segment.into_data() {
                data!(DefinitionPlaceSegment::Place(place)) => Some(place),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(definition_places, vec![1, 2, 3, 4, 5]);
        assert_eq!(
            definition_place_segments_for_notes_line("$c_2$", &place_map),
            vec![new!(DefinitionPlaceSegment::Place(1))]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn preserves_malformed_blocks_and_normalizes_dollar_equals_typos() {
        let definition = "$bad$ $x_0$ $x_1$=$p_2$";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            definition_place_segments_for_definition_line(definition, &place_map),
            vec![
                new!(DefinitionPlaceSegment::Text("$bad$ $x_0$ ".to_owned())),
                new!(DefinitionPlaceSegment::Place(1)),
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn resolves_x_subscript_markers_in_plain_prose() {
        let definition = "$x_1$ sees x₂ and x₁₂.";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            definition_place_segments_for_definition_line(definition, &place_map),
            vec![
                new!(DefinitionPlaceSegment::Place(1)),
                new!(DefinitionPlaceSegment::Text(" sees ".to_owned())),
                new!(DefinitionPlaceSegment::Place(2)),
                new!(DefinitionPlaceSegment::Text(" and ".to_owned())),
                new!(DefinitionPlaceSegment::Place(12)),
                new!(DefinitionPlaceSegment::Text(".".to_owned())),
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn keeps_adjacent_identical_place_markers_unmerged() {
        let definition = "$x_1$$x_1$ again.";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            definition_place_segments_for_definition_line(definition, &place_map),
            vec![
                new!(DefinitionPlaceSegment::Place(1)),
                new!(DefinitionPlaceSegment::Place(1)),
                new!(DefinitionPlaceSegment::Text(" again.".to_owned())),
            ]
        );
    }
}
