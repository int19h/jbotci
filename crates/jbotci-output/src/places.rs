use bityzba::{invariant, new, requires};

use crate::GlyphStyle;

#[invariant(!text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedPlaceSpan {
    pub text: String,
    pub place: Option<usize>,
}

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

/// Render one definition line using its entry's prebuilt place map.
#[requires(true)]
#[ensures(input.is_empty() -> ret.is_empty())]
pub fn format_definition_line_with_indexed_places(
    input: &str,
    place_map: &DefinitionPlaceMap,
    glyphs: GlyphStyle,
) -> String {
    indexed_place_spans_for_definition_line(input, place_map, glyphs)
        .into_iter()
        .map(|span| span.into_data().text)
        .collect()
}

/// Render one notes line without deriving any new place assignments from it.
#[requires(true)]
#[ensures(input.is_empty() -> ret.is_empty())]
pub fn format_notes_line_with_indexed_places(
    input: &str,
    place_map: &DefinitionPlaceMap,
    glyphs: GlyphStyle,
) -> String {
    indexed_place_spans_for_notes_line(input, place_map, glyphs)
        .into_iter()
        .map(|span| span.into_data().text)
        .collect()
}

/// Split a definition line into plain and indexed-place spans.
#[requires(true)]
#[ensures(input.is_empty() -> ret.is_empty())]
pub fn indexed_place_spans_for_definition_line(
    input: &str,
    place_map: &DefinitionPlaceMap,
    glyphs: GlyphStyle,
) -> Vec<IndexedPlaceSpan> {
    indexed_place_spans_for_line(input, place_map, glyphs, IndexedPlaceLineKind::Definition)
}

/// Split a notes line, leaving unmapped variables in plain spans with no place metadata.
#[requires(true)]
#[ensures(input.is_empty() -> ret.is_empty())]
pub fn indexed_place_spans_for_notes_line(
    input: &str,
    place_map: &DefinitionPlaceMap,
    glyphs: GlyphStyle,
) -> Vec<IndexedPlaceSpan> {
    indexed_place_spans_for_line(input, place_map, glyphs, IndexedPlaceLineKind::Notes)
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IndexedPlaceLineKind {
    Definition,
    Notes,
}

#[requires(true)]
#[ensures(input.is_empty() -> ret.is_empty())]
fn indexed_place_spans_for_line(
    input: &str,
    place_map: &DefinitionPlaceMap,
    glyphs: GlyphStyle,
    line_kind: IndexedPlaceLineKind,
) -> Vec<IndexedPlaceSpan> {
    let normalized = normalize_place_block_separators(input);
    let mut output = Vec::new();
    let mut remaining = normalized.as_str();
    while !remaining.is_empty() {
        let Some(open_index) = remaining.find('$') else {
            append_plain_text_spans(&mut output, remaining, glyphs);
            break;
        };
        append_plain_text_spans(&mut output, &remaining[..open_index], glyphs);
        let after_open = &remaining[open_index + 1..];
        let Some(close_index) = after_open.find('$') else {
            append_plain_text_spans(&mut output, &remaining[open_index..], glyphs);
            break;
        };
        let block_text = &after_open[..close_index];
        if let Some((place_id, _)) = find_place_var(block_text) {
            let place = match line_kind {
                IndexedPlaceLineKind::Definition => place_map.definition_place_for(&place_id),
                IndexedPlaceLineKind::Notes => place_map.place_for(&place_id),
            };
            if let Some(place) = place {
                push_indexed_place_span(
                    &mut output,
                    format!("{}{place}{}", glyphs.slot_open(), glyphs.slot_close()),
                    Some(place),
                );
            } else if line_kind == IndexedPlaceLineKind::Notes {
                push_indexed_place_span(
                    &mut output,
                    format_unmapped_place_id(&place_id, glyphs),
                    None,
                );
            } else {
                push_indexed_place_span(&mut output, format!("${block_text}$"), None);
            }
        } else {
            push_indexed_place_span(&mut output, format!("${block_text}$"), None);
        }
        remaining = &after_open[close_index + 1..];
    }
    output
}

#[requires(true)]
#[ensures(output.len() >= old(output.len()))]
fn append_plain_text_spans(output: &mut Vec<IndexedPlaceSpan>, input: &str, glyphs: GlyphStyle) {
    for span in replace_place_markers_with_indexed_place_spans(input, glyphs) {
        let span = span.into_data();
        push_indexed_place_span(output, span.text, span.place);
    }
}

#[requires(true)]
#[ensures(ret.starts_with(&place_id.letter))]
fn format_unmapped_place_id(place_id: &PlaceId, glyphs: GlyphStyle) -> String {
    match glyphs {
        GlyphStyle::Unicode => format!("{}{}", place_id.letter, subscript_number(place_id.index)),
        GlyphStyle::Ascii => format!("{}_{}", place_id.letter, place_id.index),
    }
}

#[requires(true)]
#[ensures(true)]
fn replace_place_markers_with_indexed_place_spans(
    input: &str,
    glyphs: GlyphStyle,
) -> Vec<IndexedPlaceSpan> {
    let mut output = Vec::new();
    let mut remaining = input;
    while !remaining.is_empty() {
        if let Some(after_x) = remaining.strip_prefix('x') {
            let (subscripts, rest) = span_subscript_digits(after_x);
            if subscripts.is_empty() {
                push_indexed_place_span(&mut output, "x".to_owned(), None);
                remaining = after_x;
                continue;
            }
            if let Some(place_index) = decode_subscript_digits(subscripts) {
                let text = format!(
                    "{}{}{}",
                    glyphs.slot_open(),
                    place_index,
                    glyphs.slot_close()
                );
                push_indexed_place_span(&mut output, text, Some(place_index));
            } else {
                push_indexed_place_span(&mut output, format!("x{subscripts}"), None);
            }
            remaining = rest;
            continue;
        }
        let mut chars = remaining.chars();
        if let Some(character) = chars.next() {
            push_indexed_place_span(&mut output, character.to_string(), None);
        }
        remaining = chars.as_str();
    }
    output
}

#[requires(true)]
#[ensures(output.len() >= old(output.len()))]
fn push_indexed_place_span(output: &mut Vec<IndexedPlaceSpan>, text: String, place: Option<usize>) {
    if text.is_empty() {
        return;
    }
    if output.last().is_some_and(|last| last.place == place) {
        let last = output
            .pop()
            .expect("a matching last span was observed immediately before pop");
        let mut merged_text = last.into_data().text;
        merged_text.push_str(&text);
        output.push(new!(IndexedPlaceSpan {
            text: merged_text,
            place,
        }));
        return;
    }
    output.push(new!(IndexedPlaceSpan { text, place }));
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

#[requires(true)]
#[ensures(value == 0 || !ret.is_empty())]
fn subscript_number(value: usize) -> String {
    value.to_string().chars().map(subscript_digit).collect()
}

#[requires(character.is_ascii_digit())]
#[ensures(true)]
fn subscript_digit(character: char) -> char {
    match character {
        '0' => '₀',
        '1' => '₁',
        '2' => '₂',
        '3' => '₃',
        '4' => '₄',
        '5' => '₅',
        '6' => '₆',
        '7' => '₇',
        '8' => '₈',
        '9' => '₉',
        _ => unreachable!("requires ASCII digit"),
    }
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
            format_definition_line_with_indexed_places(definition, &place_map, GlyphStyle::Unicode),
            concat!(
                "⟨1⟩ makes an image of ⟨2⟩ (object/concept) with result ⟨3⟩ ",
                "(picture) in medium ⟨4⟩."
            )
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fills_baldakyxahi_blocks_around_reserved_pin() {
        let definition = "$xa_1$ is a great sword for use against $xa_2$ by $x_3$.";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            format_definition_line_with_indexed_places(definition, &place_map, GlyphStyle::Unicode),
            "⟨1⟩ is a great sword for use against ⟨2⟩ by ⟨3⟩."
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn maps_bircidni_blocks_and_all_aliases() {
        let definition = "$c_1=b_1$ is an elbow of body $x_2=c_3=b_2$.";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            format_definition_line_with_indexed_places(definition, &place_map, GlyphStyle::Unicode),
            "⟨1⟩ is an elbow of body ⟨2⟩."
        );
        assert_eq!(
            format_notes_line_with_indexed_places("$b_2$", &place_map, GlyphStyle::Unicode),
            "⟨2⟩"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn notes_leave_unmapped_variables_unindexed() {
        let definition = "$x_1$ is a porch attached to building $x_2$.";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            indexed_place_spans_for_notes_line(
                "$x_2$ = $bartu_2$.",
                &place_map,
                GlyphStyle::Unicode,
            ),
            vec![
                new!(IndexedPlaceSpan {
                    text: "⟨2⟩".to_owned(),
                    place: Some(2),
                }),
                new!(IndexedPlaceSpan {
                    text: " = bartu₂.".to_owned(),
                    place: None,
                }),
            ]
        );
        assert_eq!(
            format_notes_line_with_indexed_places(
                "deleting $b_3$ must retain its source place",
                &place_map,
                GlyphStyle::Unicode,
            ),
            "deleting b₃ must retain its source place"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shares_one_place_map_across_definition_lines() {
        let definition = "$a_1$ first.\n$c_1$ third.\n$x_2=b_1$ pinned second.";
        let place_map = DefinitionPlaceMap::from_definition(definition);
        let rendered = definition
            .lines()
            .map(|line| {
                format_definition_line_with_indexed_places(line, &place_map, GlyphStyle::Unicode)
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(rendered, "⟨1⟩ first.\n⟨3⟩ third.\n⟨2⟩ pinned second.");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn formats_mapped_and_unmapped_places_as_ascii() {
        let definition = "$x_{12}$ and $p_1$";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            format_definition_line_with_indexed_places(definition, &place_map, GlyphStyle::Ascii),
            "<12> and <1>"
        );
        assert_eq!(
            format_notes_line_with_indexed_places(
                "$p_1$ / $x_7$ / $bartu_2$",
                &place_map,
                GlyphStyle::Ascii,
            ),
            "<1> / <7> / bartu_2"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn repeated_first_id_does_not_consume_another_place() {
        let definition = "$d_1=c_1$ is today; $d_1=c_1$ is the day of $c_2$, standard $d_3$.";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            format_definition_line_with_indexed_places(definition, &place_map, GlyphStyle::Unicode),
            "⟨1⟩ is today; ⟨1⟩ is the day of ⟨2⟩, standard ⟨3⟩."
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

        assert_eq!(
            format_definition_line_with_indexed_places(definition, &place_map, GlyphStyle::Unicode),
            concat!(
                "⟨1⟩ is a wind from direction ⟨2⟩ with speed ⟨3⟩, ",
                "shoving ⟨4⟩ at locus ⟨5⟩."
            )
        );
        assert_eq!(
            format_notes_line_with_indexed_places("$c_2$", &place_map, GlyphStyle::Unicode),
            "⟨1⟩"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn preserves_malformed_blocks_and_normalizes_dollar_equals_typos() {
        let definition = "$bad$ $x_0$ $x_1$=$p_2$";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            format_definition_line_with_indexed_places(definition, &place_map, GlyphStyle::Unicode),
            "$bad$ $x_0$ ⟨1⟩"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exposes_indexed_place_spans_for_repeated_places() {
        let definition = "$x_1$ sees $x_2$; $x_1$ again.";
        let place_map = DefinitionPlaceMap::from_definition(definition);
        let spans =
            indexed_place_spans_for_definition_line(definition, &place_map, GlyphStyle::Unicode);

        assert_eq!(
            spans,
            vec![
                new!(IndexedPlaceSpan {
                    text: "⟨1⟩".to_owned(),
                    place: Some(1),
                }),
                new!(IndexedPlaceSpan {
                    text: " sees ".to_owned(),
                    place: None,
                }),
                new!(IndexedPlaceSpan {
                    text: "⟨2⟩".to_owned(),
                    place: Some(2),
                }),
                new!(IndexedPlaceSpan {
                    text: "; ".to_owned(),
                    place: None,
                }),
                new!(IndexedPlaceSpan {
                    text: "⟨1⟩".to_owned(),
                    place: Some(1),
                }),
                new!(IndexedPlaceSpan {
                    text: " again.".to_owned(),
                    place: None,
                }),
            ]
        );
    }
}
