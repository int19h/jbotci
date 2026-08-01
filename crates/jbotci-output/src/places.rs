//! Glyph rendering of definition and notes lines with indexed places.
//!
//! The rendering-independent core — jbovlaste variable-block parsing, place
//! assignment, and line segmentation — lives in
//! [`jbotci_dictionary::places`]; this module maps the typed
//! [`DefinitionPlaceSegment`] runs onto glyph-styled text spans. The public
//! signatures and rendered bytes are unchanged from when the core lived here.

use bityzba::{data, invariant, new, requires};
use jbotci_dictionary::places::{
    DefinitionPlaceSegment, DefinitionPlaceSegmentData,
    definition_place_segments_for_definition_line, definition_place_segments_for_notes_line,
};

pub use jbotci_dictionary::places::DefinitionPlaceMap;

use crate::GlyphStyle;

#[invariant(!text.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedPlaceSpan {
    pub text: String,
    pub place: Option<usize>,
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
    indexed_place_spans_for_segments(
        &definition_place_segments_for_definition_line(input, place_map),
        glyphs,
    )
}

/// Split a notes line, leaving unmapped variables in plain spans with no place metadata.
#[requires(true)]
#[ensures(input.is_empty() -> ret.is_empty())]
pub fn indexed_place_spans_for_notes_line(
    input: &str,
    place_map: &DefinitionPlaceMap,
    glyphs: GlyphStyle,
) -> Vec<IndexedPlaceSpan> {
    indexed_place_spans_for_segments(
        &definition_place_segments_for_notes_line(input, place_map),
        glyphs,
    )
}

#[requires(true)]
#[ensures(segments.is_empty() -> ret.is_empty())]
fn indexed_place_spans_for_segments(
    segments: &[DefinitionPlaceSegment],
    glyphs: GlyphStyle,
) -> Vec<IndexedPlaceSpan> {
    let mut output = Vec::new();
    for segment in segments {
        match segment.as_data() {
            data!(DefinitionPlaceSegment::Text(text)) => {
                push_indexed_place_span(&mut output, text.clone(), None);
            }
            data!(DefinitionPlaceSegment::Place(place)) => {
                push_indexed_place_span(
                    &mut output,
                    format!("{}{place}{}", glyphs.slot_open(), glyphs.slot_close()),
                    Some(*place),
                );
            }
            data!(DefinitionPlaceSegment::UnmappedVariable { letter, index }) => {
                push_indexed_place_span(
                    &mut output,
                    format_unmapped_place_id(letter, *index, glyphs),
                    None,
                );
            }
        }
    }
    output
}

#[requires(!letter.is_empty())]
#[requires(index > 0)]
#[ensures(ret.starts_with(letter))]
fn format_unmapped_place_id(letter: &str, index: usize, glyphs: GlyphStyle) -> String {
    match glyphs {
        GlyphStyle::Unicode => format!("{letter}{}", subscript_number(index)),
        GlyphStyle::Ascii => format!("{letter}_{index}"),
    }
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

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn resolves_x_subscript_markers_in_plain_prose() {
        let definition = "$x_1$ sees x₂ and x₁₂.";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            format_definition_line_with_indexed_places(definition, &place_map, GlyphStyle::Unicode),
            "⟨1⟩ sees ⟨2⟩ and ⟨12⟩."
        );
    }
}
