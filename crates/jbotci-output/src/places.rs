use bityzba::{data, invariant, new, requires};

use crate::GlyphStyle;

#[invariant(!self.text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedPlaceSpan {
    pub text: String,
    pub place: Option<usize>,
}

#[invariant(!self.letter.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PlaceId {
    letter: String,
    index: usize,
}

#[requires(true)]
#[ensures(true)]
pub fn format_definition_or_notes_line_with_indexed_places(
    input: &str,
    glyphs: GlyphStyle,
) -> String {
    indexed_place_spans_for_definition_or_notes_line(input, glyphs)
        .into_iter()
        .map(|span| span.into_data().text)
        .collect()
}

#[requires(true)]
#[ensures(true)]
pub fn indexed_place_spans_for_definition_or_notes_line(
    input: &str,
    glyphs: GlyphStyle,
) -> Vec<IndexedPlaceSpan> {
    replace_place_markers_with_indexed_place_spans(&substitute_definition_vars("x", input), glyphs)
}

#[requires(!target_letter.is_empty())]
#[ensures(true)]
fn substitute_definition_vars(target_letter: &str, input: &str) -> String {
    let normalized = input.replace("$=$", "=");
    let place_ids = collect_place_ids(&normalized);
    let place_map = build_lujvo_place_map(&place_ids);
    replace_place_blocks(target_letter, &place_map, &normalized)
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
                push_indexed_place_span(&mut output, "x", None);
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
                push_indexed_place_span(&mut output, &text, Some(place_index));
            } else {
                push_indexed_place_span(&mut output, &format!("x{subscripts}"), None);
            }
            remaining = rest;
            continue;
        }
        let mut chars = remaining.chars();
        if let Some(character) = chars.next() {
            push_indexed_place_span(&mut output, &character.to_string(), None);
        }
        remaining = chars.as_str();
    }
    output
}

#[requires(true)]
#[ensures(output.len() >= old(output.len()))]
fn push_indexed_place_span(output: &mut Vec<IndexedPlaceSpan>, text: &str, place: Option<usize>) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = output.last_mut()
        && last.place == place
    {
        let merged_text = format!("{}{}", last.text, text);
        *last = last.clone().with_data(data! {
            text: merged_text,
        });
        return;
    }
    output.push(new!(IndexedPlaceSpan {
        text: text.to_owned(),
        place,
    }));
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
fn collect_place_ids(input: &str) -> Vec<PlaceId> {
    let mut place_ids = Vec::new();
    let mut remaining = input;
    while !remaining.is_empty() {
        let Some(after_open) = remaining.strip_prefix('$') else {
            remaining = advance_one_char(remaining);
            continue;
        };
        if let Some((place_id, after_block)) = extract_first_place_id(after_open) {
            push_unique_place_id(&mut place_ids, place_id);
            remaining = after_block;
        } else {
            remaining = drop_to_closing_dollar(after_open);
        }
    }
    place_ids
}

#[requires(true)]
#[ensures(true)]
fn push_unique_place_id(place_ids: &mut Vec<PlaceId>, place_id: PlaceId) {
    if !place_ids.contains(&place_id) {
        place_ids.push(place_id);
    }
}

#[requires(true)]
#[ensures(true)]
fn extract_first_place_id(input: &str) -> Option<(PlaceId, &str)> {
    let (place_id, rest) = find_place_var(input)?;
    Some((place_id, drop_to_closing_dollar(rest)))
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
#[ensures(true)]
fn drop_to_closing_dollar(input: &str) -> &str {
    match input.find('$') {
        Some(index) => &input[index + 1..],
        None => "",
    }
}

#[requires(true)]
#[ensures(true)]
fn advance_one_char(input: &str) -> &str {
    let mut chars = input.chars();
    let _ = chars.next();
    chars.as_str()
}

#[requires(true)]
#[ensures(true)]
fn build_lujvo_place_map(place_ids: &[PlaceId]) -> Vec<(PlaceId, usize)> {
    let max_x_place = place_ids
        .iter()
        .filter(|place_id| place_id.letter == "x")
        .map(|place_id| place_id.index)
        .max()
        .unwrap_or(0);
    let mut mapping = Vec::new();
    for place_id in place_ids {
        if place_id.letter == "x" {
            mapping.push((place_id.clone(), place_id.index));
        }
    }
    let mut next_index = max_x_place + 1;
    for place_id in place_ids {
        if place_id.letter != "x" {
            mapping.push((place_id.clone(), next_index));
            next_index += 1;
        }
    }
    mapping
}

#[requires(!target_letter.is_empty())]
#[ensures(true)]
fn replace_place_blocks(
    target_letter: &str,
    place_map: &[(PlaceId, usize)],
    input: &str,
) -> String {
    let mut output = String::new();
    let mut remaining = input;
    while !remaining.is_empty() {
        let Some(after_open) = remaining.strip_prefix('$') else {
            let mut chars = remaining.chars();
            if let Some(character) = chars.next() {
                output.push(character);
            }
            remaining = chars.as_str();
            continue;
        };
        if let Some((place_id, after_block)) = extract_first_place_id(after_open) {
            if let Some((_place_id, mapped_index)) = place_map
                .iter()
                .find(|(candidate, _)| *candidate == place_id)
            {
                output.push_str(target_letter);
                output.push_str(&subscript_number(*mapped_index));
            } else {
                output.push('$');
            }
            remaining = after_block;
        } else if let Some(close_index) = after_open.find('$') {
            output.push('$');
            output.push_str(&after_open[..close_index]);
            output.push('$');
            remaining = &after_open[close_index + 1..];
        } else {
            output.push('$');
            output.push_str(after_open);
            break;
        }
    }
    output
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
    fn formats_simple_x_places_as_unicode_indices() {
        assert_eq!(
            format_definition_or_notes_line_with_indexed_places(
                "$x_1$ is a loanword meaning $x_2$ in language $x_3$.",
                GlyphStyle::Unicode,
            ),
            "⟨1⟩ is a loanword meaning ⟨2⟩ in language ⟨3⟩."
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn formats_lujvo_non_x_places_after_canonical_x_places() {
        assert_eq!(
            format_definition_or_notes_line_with_indexed_places(
                "$x_1=p_2$ foo $x_2=p_1$ bar $p_3$ baz $bi_3=ba_2$",
                GlyphStyle::Unicode,
            ),
            "⟨1⟩ foo ⟨2⟩ bar ⟨3⟩ baz ⟨4⟩"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn formats_ascii_indices_when_requested() {
        assert_eq!(
            format_definition_or_notes_line_with_indexed_places(
                "$x_{12}$ and $x_2$",
                GlyphStyle::Ascii,
            ),
            "<12> and <2>"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn preserves_malformed_blocks_and_normalizes_dollar_equals_typos() {
        assert_eq!(
            format_definition_or_notes_line_with_indexed_places(
                "$bad$ $x_1$=$p_2$",
                GlyphStyle::Unicode,
            ),
            "$bad$ ⟨1⟩"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exposes_indexed_place_spans_for_repeated_places() {
        let spans = indexed_place_spans_for_definition_or_notes_line(
            "$x_1$ sees $x_2$; $x_1$ again.",
            GlyphStyle::Unicode,
        );

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

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exposes_mapped_lujvo_and_malformed_place_spans() {
        let spans = indexed_place_spans_for_definition_or_notes_line(
            "$bad$ $x_1=p_2$ foo $bi_3=ba_2$",
            GlyphStyle::Ascii,
        );

        assert_eq!(
            spans,
            vec![
                new!(IndexedPlaceSpan {
                    text: "$bad$ ".to_owned(),
                    place: None,
                }),
                new!(IndexedPlaceSpan {
                    text: "<1>".to_owned(),
                    place: Some(1),
                }),
                new!(IndexedPlaceSpan {
                    text: " foo ".to_owned(),
                    place: None,
                }),
                new!(IndexedPlaceSpan {
                    text: "<2>".to_owned(),
                    place: Some(2),
                }),
            ]
        );
    }
}
