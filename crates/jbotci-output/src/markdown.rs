use std::fmt::Write;

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_diagnostics::{DiagnosticTextLink, DiagnosticTextRole, DiagnosticTextSegment};
use jbotci_search::vlacku::{VlackuCard, VlackuCompositionKind, VlackuCompositionPiece};

use crate::unicode_math;
use crate::{DefinitionPlaceMap, GlyphStyle, indexed_place_spans_for_definition_line};

/// Canonical public jbotci base used for links in transport-neutral Markdown.
pub const DEFAULT_MARKDOWN_LINK_BASE: &str = "https://jbotci.app";

/// Render diagnostic rich-text runs as CommonMark using the canonical jbotci links.
///
/// Specific Lojban words use code spans. Selma'o and word categories use strong
/// emphasis, while grammar constructs and explanatory keywords use ordinary
/// emphasis. Punctuation and plain prose remain unstyled. Any typed link on a
/// segment wraps that styled run without discarding its role-specific markup.
#[requires(true)]
#[ensures(segments.is_empty() -> ret.is_empty())]
pub fn render_diagnostic_text_segments_markdown(segments: &[DiagnosticTextSegment]) -> String {
    render_diagnostic_text_segments_markdown_with_link_base(segments, DEFAULT_MARKDOWN_LINK_BASE)
}

/// Render diagnostic rich-text runs with links rooted at `link_base`.
#[requires(!link_base.trim_end_matches('/').is_empty())]
#[ensures(segments.is_empty() -> ret.is_empty())]
pub fn render_diagnostic_text_segments_markdown_with_link_base(
    segments: &[DiagnosticTextSegment],
    link_base: &str,
) -> String {
    let mut output = String::new();
    for segment in segments {
        let styled = render_diagnostic_segment_role(segment.role, &segment.text);
        if let Some(link) = &segment.link {
            let href = diagnostic_link_href(link_base, link);
            push_markdown_link(&mut output, &styled, &href);
        } else {
            output.push_str(&styled);
        }
    }
    output
}

/// Render one dictionary card as a compact Markdown documentation block.
#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn render_vlacku_card_markdown(card: &VlackuCard) -> String {
    render_vlacku_card_markdown_with_link_base(card, DEFAULT_MARKDOWN_LINK_BASE)
}

/// Render one dictionary card with inline dictionary links rooted at `link_base`.
#[requires(!link_base.trim_end_matches('/').is_empty())]
#[ensures(!ret.is_empty())]
pub fn render_vlacku_card_markdown_with_link_base(card: &VlackuCard, link_base: &str) -> String {
    let place_map = DefinitionPlaceMap::from_definition(&card.definition);
    let mut output = render_vlacku_headword_markdown(
        &card.word,
        &card.word_type,
        card.selmaho
            .as_deref()
            .filter(|selmaho| !selmaho.trim().is_empty()),
    );

    if !card.decomposition.is_empty() {
        output.push_str("\n\n**Decomposition:** ");
        output.push_str(&render_vlacku_decomposition_markdown(&card.decomposition));
    }
    if !card.definition.trim().is_empty() {
        output.push_str("\n\n");
        output.push_str(&render_vlacku_detail_markdown(
            &card.definition,
            &place_map,
            link_base,
        ));
    }
    if !card.glosses.is_empty() {
        output.push_str("\n\n**Glosses:** ");
        push_code_list(&mut output, &card.glosses);
    }
    if !card.rafsi.is_empty() {
        output.push_str("\n\n**Rafsi:** ");
        push_code_list(&mut output, &card.rafsi);
    }
    output
}

/// Render the compact headword line shared by dictionary and classification cards.
#[requires(!word.is_empty())]
#[requires(selmaho.is_none_or(|value| !value.trim().is_empty()))]
#[ensures(!ret.is_empty())]
pub fn render_vlacku_headword_markdown(
    word: &str,
    word_type: &str,
    selmaho: Option<&str>,
) -> String {
    let mut output = String::from("### ");
    output.push_str(&inline_code(word));
    output.push_str(" — *");
    output.push_str(&escape_markdown_inline(word_type));
    output.push('*');
    if let Some(selmaho) = selmaho {
        output.push_str(" · **");
        output.push_str(&escape_markdown_inline(selmaho));
        output.push_str("**");
    }
    output
}

/// Render dictionary cards as independently separated Markdown blocks.
#[requires(true)]
#[ensures(cards.is_empty() -> ret.is_empty())]
pub fn render_vlacku_cards_markdown(cards: &[VlackuCard]) -> String {
    render_vlacku_cards_markdown_with_link_base(cards, DEFAULT_MARKDOWN_LINK_BASE)
}

/// Render dictionary cards with inline links rooted at `link_base`.
#[requires(!link_base.trim_end_matches('/').is_empty())]
#[ensures(cards.is_empty() -> ret.is_empty())]
pub fn render_vlacku_cards_markdown_with_link_base(
    cards: &[VlackuCard],
    link_base: &str,
) -> String {
    let mut output = String::new();
    for (index, card) in cards.iter().enumerate() {
        if index > 0 {
            output.push_str("\n\n---\n\n");
        }
        output.push_str(&render_vlacku_card_markdown_with_link_base(card, link_base));
    }
    output
}

/// Render a lujvo decomposition as `rafsi·rafsi → source + source`.
#[requires(true)]
#[ensures(pieces.is_empty() -> ret.is_empty())]
pub fn render_vlacku_decomposition_markdown(pieces: &[VlackuCompositionPiece]) -> String {
    let mut output = String::new();
    for (index, piece) in pieces.iter().enumerate() {
        if index > 0 {
            output.push('·');
        }
        output.push_str(&inline_code(&piece.surface));
    }

    let mut source_count = 0usize;
    for piece in pieces {
        if piece.kind != VlackuCompositionKind::Rafsi {
            continue;
        }
        let Some(source) = piece.source.as_deref() else {
            continue;
        };
        if source_count == 0 {
            output.push_str(" → ");
        } else {
            output.push_str(" + ");
        }
        output.push_str(&inline_code(source));
        source_count += 1;
    }
    output
}

#[requires(!text.is_empty())]
#[ensures(!ret.is_empty())]
fn render_diagnostic_segment_role(role: DiagnosticTextRole, text: &str) -> String {
    match role {
        DiagnosticTextRole::SpecificWord => inline_code(text),
        DiagnosticTextRole::Selmaho | DiagnosticTextRole::WordCategory => {
            format!("**{}**", escape_markdown_inline(text))
        }
        DiagnosticTextRole::Construct | DiagnosticTextRole::Keyword => {
            format!("*{}*", escape_markdown_inline(text))
        }
        DiagnosticTextRole::Punctuation | DiagnosticTextRole::Plain => escape_markdown_inline(text),
    }
}

#[requires(true)]
#[ensures(input.is_empty() -> ret.is_empty())]
fn render_vlacku_detail_markdown(
    input: &str,
    place_map: &DefinitionPlaceMap,
    link_base: &str,
) -> String {
    let mut output = String::new();
    for (line_index, line) in input.lines().enumerate() {
        if line_index > 0 {
            output.push_str("\n\n");
        }
        let spans = indexed_place_spans_for_definition_line(line, place_map, GlyphStyle::Unicode);
        for span in spans {
            let span = span.into_data();
            if let Some(place) = span.place {
                output.push_str(&mathematical_place_variable(place));
            } else {
                push_vlacku_text_with_links(&mut output, &span.text, link_base);
            }
        }
    }
    output
}

#[requires(place > 0)]
#[ensures(ret.starts_with('𝑥'))]
fn mathematical_place_variable(place: usize) -> String {
    format!("𝑥{}", GlyphStyle::Unicode.numeric_suffix(place))
}

#[requires(true)]
#[ensures(true)]
fn push_vlacku_text_with_links(output: &mut String, input: &str, link_base: &str) {
    let mut remaining = input;
    while !remaining.is_empty() {
        let Some(open_index) = remaining.find('$') else {
            push_vlacku_text_links_only(output, remaining, link_base);
            break;
        };
        let after_open = &remaining[open_index + 1..];
        let Some(close_index) = after_open.find('$') else {
            push_vlacku_text_links_only(output, remaining, link_base);
            break;
        };
        push_vlacku_text_links_only(output, &remaining[..open_index], link_base);
        let source = &after_open[..close_index];
        if let Some(rendered) = unicode_math::render(source) {
            output.push_str(&rendered);
        } else if source.is_empty() {
            output.push_str(&inline_code("$$"));
        } else {
            output.push_str(&inline_code(source));
        }
        remaining = &after_open[close_index + 1..];
    }
}

#[requires(true)]
#[ensures(true)]
fn push_vlacku_text_links_only(output: &mut String, input: &str, link_base: &str) {
    let mut remaining = input;
    while !remaining.is_empty() {
        let Some(open_index) = remaining.find('{') else {
            output.push_str(&escape_markdown_inline(remaining));
            break;
        };
        let after_open = &remaining[open_index + 1..];
        let Some(close_index) = after_open.find('}') else {
            output.push_str(&escape_markdown_inline(remaining));
            break;
        };
        output.push_str(&escape_markdown_inline(&remaining[..open_index]));
        let inside = &after_open[..close_index];
        let word = inside.trim();
        if !word.is_empty() && !word.chars().any(char::is_whitespace) {
            let href = format!(
                "{}/vlacku/{}",
                link_base.trim_end_matches('/'),
                percent_encode_path_component(word),
            );
            push_markdown_link(output, &inline_code(word), &href);
        } else {
            output.push_str(&escape_markdown_inline(
                &remaining[open_index..open_index + close_index + 2],
            ));
        }
        remaining = &after_open[close_index + 1..];
    }
}

#[requires(true)]
#[ensures(true)]
fn push_code_list(output: &mut String, values: &[String]) {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        output.push_str(&inline_code(value));
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn diagnostic_link_href(link_base: &str, link: &DiagnosticTextLink) -> String {
    let base = link_base.trim_end_matches('/');
    if let Some(word) = link.vlacku_word() {
        return format!("{base}/vlacku/{}", percent_encode_path_component(word));
    }
    if let Some((section_id, anchor)) = link.cll_section() {
        let mut href = format!(
            "{base}/cukta/section/{}",
            percent_encode_path_component(section_id),
        );
        if let Some(anchor) = anchor {
            href.push('#');
            href.push_str(&percent_encode_fragment(anchor.trim_start_matches('#')));
        }
        return href;
    }
    if let Some(rule_name) = link.ebnf_rule() {
        return format!(
            "{base}/cukta/section/section-EBNF#{}",
            ebnf_rule_anchor_id(rule_name),
        );
    }
    unreachable!("diagnostic text link variants are exhaustive")
}

#[requires(!rule_name.is_empty())]
#[ensures(ret.starts_with("ebnf-rule-"))]
fn ebnf_rule_anchor_id(rule_name: &str) -> String {
    let mut output = String::from("ebnf-rule-");
    let mut needs_separator = false;
    for character in rule_name.chars() {
        if character.is_ascii_alphanumeric() {
            if needs_separator && !output.ends_with('-') {
                output.push('-');
            }
            output.push(character.to_ascii_lowercase());
            needs_separator = false;
        } else {
            needs_separator = true;
        }
    }
    output
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn inline_code(text: &str) -> String {
    let longest_run = longest_backtick_run(text);
    let fence = "`".repeat(longest_run + 1);
    let needs_padding = text.starts_with(['`', ' ']) || text.ends_with(['`', ' ']);
    if needs_padding {
        format!("{fence} {text} {fence}")
    } else {
        format!("{fence}{text}{fence}")
    }
}

#[requires(true)]
#[ensures(true)]
fn longest_backtick_run(text: &str) -> usize {
    let mut longest = 0usize;
    let mut current = 0usize;
    for character in text.chars() {
        if character == '`' {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    longest
}

#[requires(true)]
#[ensures(text.is_empty() -> ret.is_empty())]
fn escape_markdown_inline(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>') {
            output.push('\\');
        }
        output.push(character);
    }
    output
}

#[requires(!label.is_empty())]
#[requires(!href.is_empty())]
#[ensures(true)]
fn push_markdown_link(output: &mut String, label: &str, href: &str) {
    output.push('[');
    output.push_str(label);
    output.push_str("](");
    output.push_str(href);
    output.push(')');
}

#[requires(true)]
#[ensures(true)]
fn percent_encode_path_component(input: &str) -> String {
    percent_encode(input, is_path_component_byte)
}

#[requires(true)]
#[ensures(true)]
fn percent_encode_fragment(input: &str) -> String {
    percent_encode(input, is_fragment_byte)
}

#[requires(true)]
#[ensures(true)]
fn percent_encode(input: &str, permitted: fn(u8) -> bool) -> String {
    let mut output = String::with_capacity(input.len());
    for byte in input.bytes() {
        if permitted(byte) {
            output.push(char::from(byte));
        } else {
            write!(&mut output, "%{byte:02X}").expect("writing to String cannot fail");
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn is_path_component_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'\'')
}

#[requires(true)]
#[ensures(true)]
fn is_fragment_byte(byte: u8) -> bool {
    is_path_component_byte(byte) || matches!(byte, b':' | b'@' | b'/' | b'?')
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};
    use jbotci_diagnostics::DiagnosticTextRole;
    use jbotci_search::vlacku::{VlackuRequest, VlackuSearchOptions, run_vlacku_requests};

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diagnostic_segments_render_every_role_and_preserve_links() {
        let segments = [
            DiagnosticTextSegment::new(DiagnosticTextRole::SpecificWord, "klama".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Plain, " / ".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Selmaho, "KOhA".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Plain, " / ".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::WordCategory, "BRIVLA".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Plain, " / ".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Keyword, "expected".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Plain, " / ".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Construct, "sumti".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ":".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Plain, " plain".to_owned()),
        ];

        assert_eq!(
            render_diagnostic_text_segments_markdown(&segments),
            "[`klama`](https://jbotci.app/vlacku/klama) / [**KOhA**](https://jbotci.app/cukta/section/section-index#KOhA) / [**BRIVLA**](https://jbotci.app/cukta/section/section-morphology-brivla) / *expected* / [*sumti*](https://jbotci.app/cukta/section/section-EBNF#ebnf-rule-sumti): plain",
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_card_markdown_indexes_places_and_renders_dictionary_fields() {
        let output = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::valsi("klama".to_owned())],
            &VlackuSearchOptions::default(),
        );
        let card = output.cards.first().expect("klama has a dictionary card");
        let markdown = render_vlacku_card_markdown(card);

        assert_eq!(
            markdown,
            concat!(
                "### `klama` — *gismu*\n\n",
                "𝑥₁ comes/goes to destination 𝑥₂ from origin 𝑥₃ via route 𝑥₄ ",
                "using means/vehicle 𝑥₅.\n\n",
                "**Glosses:** `come`\n\n",
                "**Rafsi:** `kla`",
            ),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_card_markdown_uses_the_whole_definition_place_map() {
        let output = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::valsi("baldakyxa'i".to_owned())],
            &VlackuSearchOptions::default(),
        );
        let card = output
            .cards
            .first()
            .expect("baldakyxa'i has a dictionary card");
        let markdown = render_vlacku_card_markdown(card);

        assert!(
            markdown.contains("𝑥₁ is a great sword for use against 𝑥₂ by 𝑥₃."),
            "{markdown}"
        );
        assert!(!markdown.contains("𝑥₄"), "{markdown}");
        assert!(!markdown.contains("𝑥₅"), "{markdown}");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_card_markdown_renders_avgadro_unicode_math() {
        let output = run_vlacku_requests(
            jbotci_dictionary_data::english(),
            &[VlackuRequest::valsi("avgadro".to_owned())],
            &VlackuSearchOptions::default(),
        );
        let card = output.cards.first().expect("avgadro has a dictionary card");
        let markdown = render_vlacku_card_markdown(card);

        assert_eq!(
            markdown,
            concat!(
                "### `avgadro` — *fu'ivla*\n\n",
                "𝑥₁ is Avogadro constant `N_{A}` \\[approximately equal to: ",
                "6.02214129(27)×10²³ mol⁻¹\\], expressed in units 𝑥₂ in ",
                "paradigm/system/metaphysics/universe 𝑥₃ (default: this, our actual, ",
                "physical universe).\n\n",
                "**Glosses:** ",
                "`6.02214129(27)×10^23 mol^(−1) (approximately Avogadro constant N_A)`, ",
                "`Avogadro constant (N_A; approximately 6.02214129(27)×10^23 mol^(−1))`, ",
                "`Avogadro's number (Avogadro constant N_A; approximately ",
                "6.02214129(27)×10^23 mol^(−1))`, ",
                "`N_A (Avogadro constant; approximately 6.02214129(27)×10^23 mol^(−1))`",
            )
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlacku_card_markdown_preserves_unmappable_latex_in_code() {
        let definition = r"before $\sqrt{x}$ and $N_{A}$ after";
        let place_map = DefinitionPlaceMap::from_definition(definition);

        assert_eq!(
            render_vlacku_detail_markdown(definition, &place_map, DEFAULT_MARKDOWN_LINK_BASE),
            r"before `\sqrt{x}` and `N_{A}` after",
        );
    }
}
