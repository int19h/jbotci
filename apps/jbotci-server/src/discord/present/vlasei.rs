//! Vlasei presentation: morphology-derived segmentation in the words,
//! brackets, tree and IPA views, with recovered regions kept visible.

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_morphology::{LujvoPart, PhonemeRenderOptions, WordKind, WordLike, WordLikeData};
use jbotci_output::{
    BracketRenderOptions, GlyphStyle, LojbanScript, TreeRenderOptions, ipa_morphology_text,
    pretty_morphology_brackets_with_options, pretty_morphology_tree_with_options,
    pretty_recovered_morphology_brackets_with_options,
    pretty_recovered_morphology_tree_with_options,
};
use jbotci_web_core::VlaseiAnalysis;

use super::RenderedResult;
use super::diagnostics::{render_diagnostics, summary};
use super::markdown::{code_block, escape, join_lines, strike, subtext};
use crate::discord::request::{DiscordTool, VlaseiRequest, VlaseiView};

/// Words listed in the message before the rest goes to the attachment.
const MAX_LISTED_WORDS: usize = 80;

#[requires(true)]
#[ensures(ret.tool() == DiscordTool::Vlasei)]
pub(crate) fn render(analysis: &VlaseiAnalysis, request: &VlaseiRequest) -> RenderedResult {
    let source = request.text.as_str();
    let options = request.options;
    let view_label = match options.view {
        VlaseiView::Words => "words",
        VlaseiView::Brackets => "brackets",
        VlaseiView::Tree => "tree",
        VlaseiView::Ipa => "IPA",
    };
    let mut status = format!("vlasei · {view_label}");
    if options.decompose_lujvo {
        status.push_str(" · lujvo decomposed");
    }
    if !analysis.is_complete() {
        status.push_str(" · recovered");
    }
    if let Some(counts) = summary(&analysis.diagnostics) {
        status.push_str(&format!(" · {counts}"));
    }
    let mut rendered = RenderedResult::new(DiscordTool::Vlasei, status);
    let words = &analysis.morphology.words;
    let phonemes = PhonemeRenderOptions::default();
    let (body, full_text, listed_in_part) = match options.view {
        VlaseiView::Words => words_view(source, analysis, options.decompose_lujvo),
        VlaseiView::Brackets => {
            let text = if analysis.is_complete() {
                pretty_morphology_brackets_with_options(
                    words,
                    source,
                    bracket_options(phonemes, options.decompose_lujvo),
                )
            } else {
                pretty_recovered_morphology_brackets_with_options(
                    &analysis.morphology,
                    source,
                    bracket_options(phonemes, options.decompose_lujvo),
                )
            }
            .unwrap_or_else(|error| error.to_string());
            (code_block(&text), text, false)
        }
        VlaseiView::Tree => {
            let text = if analysis.is_complete() {
                pretty_morphology_tree_with_options(
                    words,
                    source,
                    tree_options(phonemes, options.decompose_lujvo),
                )
            } else {
                pretty_recovered_morphology_tree_with_options(
                    &analysis.morphology,
                    source,
                    tree_options(phonemes, options.decompose_lujvo),
                )
            }
            .unwrap_or_else(|error| error.to_string());
            (code_block(&text), text, false)
        }
        VlaseiView::Ipa => {
            let text = ipa_morphology_text(words, source).unwrap_or_else(|error| error.to_string());
            let mut body = code_block(&text);
            if !analysis.is_complete() {
                body.push('\n');
                body.push_str(&subtext(
                    "IPA covers the words the parser recovered; skipped input has no pronunciation.",
                ));
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
    rendered.set_diagnostics(render_diagnostics(source, &analysis.diagnostics));
    if !analysis.is_complete() {
        rendered.notice = Some(subtext(
            "Recovered segmentation: struck-through input was skipped after a morphology error.",
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
        indent: 2,
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
#[ensures(!ret.0.is_empty())]
fn words_view(
    source: &str,
    analysis: &VlaseiAnalysis,
    decompose_lujvo: bool,
) -> (String, String, bool) {
    let mut entries: Vec<(usize, String, String)> = Vec::new();
    for word in &analysis.morphology.words {
        let (start, text) = word
            .byte_range()
            .and_then(|range| {
                source
                    .get(range.clone())
                    .map(|text| (range.start, text.to_owned()))
            })
            .unwrap_or((usize::MAX, word_fallback_text(word)));
        let description = describe_word(word, decompose_lujvo);
        entries.push((
            start,
            format!("{} — {}", escape(&text), escape(&description)),
            format!("{text} — {description}"),
        ));
    }
    for region in &analysis.morphology.error_regions {
        let text = source
            .get(region.byte_start..region.byte_end)
            .unwrap_or("")
            .trim_end()
            .to_owned();
        if text.is_empty() {
            continue;
        }
        entries.push((
            region.byte_start,
            format!("{} — skipped", strike(&text)),
            format!("{text} — skipped"),
        ));
    }
    entries.sort_by_key(|(start, _, _)| *start);
    if entries.is_empty() {
        return ("*(no words)*".to_owned(), "(no words)".to_owned(), false);
    }
    let full_text = entries
        .iter()
        .map(|(_, _, plain)| plain.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let mut lines = entries
        .iter()
        .take(MAX_LISTED_WORDS)
        .map(|(_, markdown, _)| markdown.clone())
        .collect::<Vec<_>>();
    if entries.len() > MAX_LISTED_WORDS {
        lines.push(subtext(&format!(
            "showing the first {MAX_LISTED_WORDS} of {} words",
            entries.len()
        )));
    }
    (
        join_lines(lines),
        full_text,
        entries.len() > MAX_LISTED_WORDS,
    )
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn word_fallback_text(word: &WordLike) -> String {
    word.bare_word()
        .map(|word| word.phonemes().as_str().to_owned())
        .unwrap_or_else(|| "?".to_owned())
}

/// Class and, where the morphology knows it, selma'o or lujvo parts.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn describe_word(word: &WordLike, decompose_lujvo: bool) -> String {
    match word.as_data() {
        data!(WordLike::PlainWord(plain)) => {
            let mut description = word_kind_label(plain.kind()).to_owned();
            if let Some(selmaho) = plain.selmaho() {
                description.push_str(&format!(" ({selmaho})"));
            }
            if decompose_lujvo && let Some(parts) = plain.lujvo_parts() {
                let parts = parts
                    .iter()
                    .map(|part| match part {
                        LujvoPart::Rafsi(phonemes) => phonemes.as_str().to_owned(),
                        LujvoPart::Hyphen(phonemes) => format!("-{}-", phonemes.as_str()),
                    })
                    .collect::<Vec<_>>();
                description.push_str(&format!(": {}", parts.join(" + ")));
            }
            description
        }
        data!(WordLike::QuotedWord { .. }) => "zo quote of one word".to_owned(),
        data!(WordLike::SelmahoQuotedWord { .. }) => "ma'oi quote of one word".to_owned(),
        data!(WordLike::DelimitedNonLojbanQuote { .. }) => "zoi quote (non-Lojban text)".to_owned(),
        data!(WordLike::QuotedWords { .. }) => "lo'u … le'u quote (Lojban words)".to_owned(),
        data!(WordLike::DelimitedWordQuote { .. }) => "single-word quote".to_owned(),
        data!(WordLike::LerfuWord { .. }) => "lerfu word (bu)".to_owned(),
        data!(WordLike::ZeiCompound { .. }) => "zei compound".to_owned(),
    }
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
