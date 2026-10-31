//! Vlasei presentation: morphology-derived segmentation in the words,
//! brackets, tree and IPA views, with recovered regions kept visible.

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_morphology::{LujvoPart, PhonemeRenderOptions, WordKind, WordLike, WordLikeData};
use jbotci_output::{
    BracketRenderOptions, GlyphStyle, LojbanScript, TreeRenderOptions,
    pretty_morphology_brackets_with_limit, pretty_morphology_tree_with_limit,
    pretty_recovered_morphology_brackets_with_limit, pretty_recovered_morphology_tree_with_limit,
};
use jbotci_web_core::VlaseiAnalysis;

use super::RenderedResult;
use super::diagnostics::{render_diagnostics_in_scope, summary_in_scope};
use super::markdown::{
    code_block_in_scope, escape_in_scope, join_lines_in_scope, strike_in_scope, subtext_in_scope,
};
use crate::discord::request::{DiscordTool, VlaseiRequest, VlaseiView};

/// Words listed in the message before the rest goes to the attachment.
const MAX_LISTED_WORDS: usize = 80;

#[requires(true)]
#[ensures(text_scope.is_exceeded() || (ret.tool() == DiscordTool::Vlasei))]
pub(crate) fn render_in_scope(
    analysis: &VlaseiAnalysis,
    request: &VlaseiRequest,
    text_scope: &jbotci_output::budget::TextScope,
) -> RenderedResult {
    let source = request.text.as_str();
    let options = request.options;
    let view_label = match options.view {
        VlaseiView::Words => "words",
        VlaseiView::Brackets => "brackets",
        VlaseiView::Tree => "tree",
        VlaseiView::Ipa => "IPA",
    };
    let mut status =
        text_scope.buffer_with(&text_scope.format(format_args!("vlasei · {view_label}")));
    if options.decompose_lujvo {
        status.push_str(" · lujvo decomposed");
    }
    if !analysis.is_complete() {
        status.push_str(" · recovered");
    }
    if let Some(counts) = summary_in_scope(&analysis.diagnostics, text_scope) {
        status.push_str(&text_scope.format(format_args!(" · {counts}")));
    }
    let mut rendered = RenderedResult::new_in_scope(DiscordTool::Vlasei, status, text_scope);
    let words = &analysis.morphology.words;
    let phonemes = PhonemeRenderOptions::default();
    let (body, full_text, listed_in_part) = match options.view {
        VlaseiView::Words => {
            words_view_in_scope(source, analysis, options.decompose_lujvo, text_scope)
        }
        VlaseiView::Brackets => {
            let result = if analysis.is_complete() {
                pretty_morphology_brackets_with_limit(
                    words,
                    source,
                    bracket_options(phonemes, options.decompose_lujvo),
                    text_scope.limit(),
                )
            } else {
                pretty_recovered_morphology_brackets_with_limit(
                    &analysis.morphology,
                    source,
                    bracket_options(phonemes, options.decompose_lujvo),
                    text_scope.limit(),
                )
            };
            let text = super::render_output_in_scope(result, text_scope);
            (code_block_in_scope(&text, text_scope), text, false)
        }
        VlaseiView::Tree => {
            let result = if analysis.is_complete() {
                pretty_morphology_tree_with_limit(
                    words,
                    source,
                    tree_options(phonemes, options.decompose_lujvo),
                    text_scope.limit(),
                )
            } else {
                pretty_recovered_morphology_tree_with_limit(
                    &analysis.morphology,
                    source,
                    tree_options(phonemes, options.decompose_lujvo),
                    text_scope.limit(),
                )
            };
            let text = super::render_output_in_scope(result, text_scope);
            (code_block_in_scope(&text, text_scope), text, false)
        }
        VlaseiView::Ipa => {
            let text = super::render_output_in_scope(
                jbotci_output::ipa_morphology_text_with_limit(words, source, text_scope.limit()),
                text_scope,
            );
            let mut body = code_block_in_scope(&text, text_scope);
            if !analysis.is_complete() {
                body.push('\n');
                body.push_str(&subtext_in_scope(
                    "IPA covers the words the parser recovered; skipped input has no pronunciation.",
                 text_scope));
            }
            (body, text, false)
        }
    };
    rendered.body.push(body);
    if listed_in_part {
        rendered.show_excerpt_of(full_text);
    } else {
        rendered.set_full_text(full_text);
    }
    rendered.set_diagnostics(render_diagnostics_in_scope(
        source,
        &analysis.diagnostics,
        text_scope,
    ));
    if !analysis.is_complete() {
        rendered.notice = Some(subtext_in_scope(
            "Recovered segmentation: struck-through input was skipped after a morphology error.",
            text_scope,
        ));
    }
    rendered
}

#[requires(true)]
#[ensures(!ret.color)]
fn bracket_options(phonemes: PhonemeRenderOptions, decompose_lujvo: bool) -> BracketRenderOptions {
    BracketRenderOptions {
        color: false,
        phonemes,
        script: LojbanScript::Latin,
        glyphs: GlyphStyle::Unicode,
        decompose_lujvo,
        insert_hair_space: false,
        show_elided: false,
    }
}

#[requires(true)]
#[ensures(!ret.color && !ret.show_refs)]
fn tree_options(phonemes: PhonemeRenderOptions, decompose_lujvo: bool) -> TreeRenderOptions {
    TreeRenderOptions {
        color: false,
        indent: jbotci_output::Indentation::default(),
        phonemes,
        glyphs: GlyphStyle::Unicode,
        show_spans: false,
        show_refs: false,
        decompose_lujvo,
        show_elided: false,
    }
}

/// Human-readable word class.
#[requires(true)]
#[ensures(!ret.is_empty())]
pub(crate) fn word_kind_label(kind: WordKind) -> &'static str {
    match kind {
        WordKind::Cmavo => "cmavo",
        WordKind::Gismu => "gismu",
        WordKind::Lujvo => "lujvo",
        WordKind::Fuhivla => "fu'ivla",
        WordKind::Cmevla => "cmevla",
    }
}

/// One line per morphology word (never per whitespace token), with skipped
/// regions interleaved in source order. Returns the Markdown body, the plain
/// text of every word, and whether the body lists fewer than all of them.
#[requires(true)]
#[ensures(text_scope.is_exceeded() || (!ret.0.is_empty()))]
fn words_view_in_scope(
    source: &str,
    analysis: &VlaseiAnalysis,
    decompose_lujvo: bool,
    text_scope: &jbotci_output::budget::TextScope,
) -> (String, String, bool) {
    let mut entries: Vec<(usize, String, String)> = Vec::new();
    for word in &analysis.morphology.words {
        if text_scope.is_exceeded() {
            break;
        }
        let (start, text) = word
            .byte_range()
            .and_then(|range| {
                source
                    .get(range.clone())
                    .map(|text| (range.start, text_scope.copy(text)))
            })
            .unwrap_or_else(|| (usize::MAX, word_fallback_text_in_scope(word, text_scope)));
        let description = describe_word_in_scope(word, decompose_lujvo, text_scope);
        entries.push((
            start,
            text_scope.format(format_args!(
                "{} — {}",
                escape_in_scope(&text, text_scope),
                escape_in_scope(&description, text_scope)
            )),
            text_scope.format(format_args!("{text} — {description}")),
        ));
    }
    for region in &analysis.morphology.error_regions {
        let text = text_scope.copy(
            source
                .get(region.byte_start..region.byte_end)
                .unwrap_or("")
                .trim_end(),
        );
        if text.is_empty() {
            continue;
        }
        entries.push((
            region.byte_start,
            text_scope.format(format_args!(
                "{} — skipped",
                strike_in_scope(&text, text_scope)
            )),
            text_scope.format(format_args!("{text} — skipped")),
        ));
    }
    entries.sort_by_key(|(start, _, _)| *start);
    if entries.is_empty() {
        return ("*(no words)*".to_owned(), "(no words)".to_owned(), false);
    }
    let full_text = text_scope.join(entries.iter().map(|(_, _, plain)| plain.as_str()), "\n");
    let mut lines = entries
        .iter()
        .take(MAX_LISTED_WORDS)
        .map(|(_, markdown, _)| markdown.clone())
        .collect::<Vec<_>>();
    if entries.len() > MAX_LISTED_WORDS {
        lines.push(subtext_in_scope(
            &text_scope.format(format_args!(
                "showing the first {MAX_LISTED_WORDS} of {} words",
                entries.len()
            )),
            text_scope,
        ));
    }
    (
        join_lines_in_scope(lines, text_scope),
        full_text,
        entries.len() > MAX_LISTED_WORDS,
    )
}

#[requires(true)]
#[ensures(text_scope.is_exceeded() || (!ret.is_empty()))]
fn word_fallback_text_in_scope(
    word: &WordLike,
    text_scope: &jbotci_output::budget::TextScope,
) -> String {
    word.bare_word()
        .map(|word| text_scope.copy(word.phonemes().as_str()))
        .unwrap_or_else(|| "?".to_owned())
}

/// Class and, where the morphology knows it, selma'o or lujvo parts.
#[requires(true)]
#[ensures(text_scope.is_exceeded() || (!ret.is_empty()))]
fn describe_word_in_scope(
    word: &WordLike,
    decompose_lujvo: bool,
    text_scope: &jbotci_output::budget::TextScope,
) -> String {
    match word.as_data() {
        data!(WordLike::PlainWord(plain)) => {
            let mut description = text_scope.buffer_with(word_kind_label(plain.kind()));
            if let Some(selmaho) = plain.selmaho() {
                description.push_str(&text_scope.format(format_args!(" ({selmaho})")));
            }
            if decompose_lujvo && let Some(parts) = plain.lujvo_parts() {
                let parts = parts
                    .iter()
                    .map(|part| match part {
                        LujvoPart::Rafsi(phonemes) => text_scope.copy(phonemes.as_str()),
                        LujvoPart::Hyphen(phonemes) => {
                            text_scope.format(format_args!("-{}-", phonemes.as_str()))
                        }
                    })
                    .collect::<Vec<_>>();
                description.push_str(
                    &text_scope.format(format_args!(": {}", text_scope.join(&parts, " + "))),
                );
            }
            description.into_piece()
        }
        data!(WordLike::QuotedWord { .. }) => "zo quote of one word".to_owned(),
        data!(WordLike::SelmahoQuotedWord { .. }) => "ma'oi quote of one word".to_owned(),
        data!(WordLike::DelimitedNonLojbanQuote { .. }) => "zoi quote (non-Lojban text)".to_owned(),
        data!(WordLike::QuotedWords { .. }) => "lo'u … le'u quote (Lojban words)".to_owned(),
        data!(WordLike::ReplacementQuote { .. }) => "replacement quote (raw words)".to_owned(),
        data!(WordLike::DelimitedWordQuote { .. }) => "single-word quote".to_owned(),
        data!(WordLike::LerfuWord { .. }) => "lerfu word (bu)".to_owned(),
        data!(WordLike::ZeiCompound { .. }) => "zei compound".to_owned(),
    }
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
pub(crate) fn render(analysis: &VlaseiAnalysis, request: &VlaseiRequest) -> RenderedResult {
    let text_scope = jbotci_output::budget::TextScope::new(
        jbotci_output::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    let result = render_in_scope(analysis, request, &text_scope);
    text_scope
        .finish(String::new())
        .expect("unrestricted Discord rendering");
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord::request::{SourceText, VlaseiOptions};
    use jbotci_web_core::analyze_vlasei;

    #[requires(true)]
    #[ensures(true)]
    fn request(text: &str, view: VlaseiView, decompose_lujvo: bool) -> VlaseiRequest {
        VlaseiRequest {
            text: SourceText::new(text).expect("text"),
            dialect: None,
            options: VlaseiOptions {
                view,
                decompose_lujvo,
            },
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn words_view_uses_morphology_boundaries_and_marks_skipped_input() {
        let request = request("coiro'ido 'x klama", VlaseiView::Words, false);
        let analysis = analyze_vlasei(request.text.as_str(), None, None).expect("analysis");
        let rendered = render(&analysis, &request);
        let body = rendered.body.join("\n");
        assert!(body.contains("coi — cmavo"), "{body}");
        assert!(
            body.contains("ro'i — cmavo") || body.contains("ro\\'i — cmavo"),
            "{body}"
        );
        assert!(body.contains("~~"), "skipped region struck through: {body}");
        assert!(
            rendered.status.text.contains("recovered"),
            "{}",
            rendered.status.text
        );
        assert!(rendered.diagnostics.is_some());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn other_views_render_the_shared_renderers_in_code_blocks() {
        for view in [VlaseiView::Brackets, VlaseiView::Tree, VlaseiView::Ipa] {
            let request = request("mi klama", view, false);
            let analysis = analyze_vlasei(request.text.as_str(), None, None).expect("analysis");
            let rendered = render(&analysis, &request);
            assert!(
                rendered.body[0].starts_with("```\n"),
                "{view:?}: {}",
                rendered.body[0]
            );
            assert!(
                rendered
                    .full_text
                    .as_deref()
                    .is_some_and(|text| !text.is_empty())
            );
        }
        let ipa = request("mi klama", VlaseiView::Ipa, false);
        let analysis = analyze_vlasei(ipa.text.as_str(), None, None).expect("analysis");
        let rendered = render(&analysis, &ipa);
        assert!(
            rendered
                .full_text
                .as_deref()
                .is_some_and(|text| text.contains("ˈkla.ma")),
            "{:?}",
            rendered.full_text
        );
        let decomposed = request("klabajra", VlaseiView::Words, true);
        let analysis = analyze_vlasei(decomposed.text.as_str(), None, None).expect("analysis");
        let rendered = render(&analysis, &decomposed);
        assert!(
            rendered.body[0].contains("lujvo\\: kla + bájra"),
            "{}",
            rendered.body[0]
        );
    }
}
