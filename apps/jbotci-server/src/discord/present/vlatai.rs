//! Vlatai presentation: validity, class, phonemes and formation facts of one
//! word; parser-derived segmentation when the input is several words.

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_morphology::{
    GlideMark, PhonemeRenderOptions, Phonemes, PlainWordClassification, StressMark,
    ValsiAnalysisStatus, ValsiClassification, ValsiClassificationKind, ValsiFuhivlaStage,
    ValsiLujvoPartKind, ValsiLujvoRafsiKind, WordKind,
};
use jbotci_output::{
    BracketRenderOptions, GlyphStyle, LojbanScript, pretty_morphology_brackets_with_limit,
};
use jbotci_web_core::VlataiReport;

use super::RenderedResult;
use super::diagnostics::{render_diagnostics_in_scope, summary_in_scope};
use super::markdown::{
    code_block_in_scope, escape_in_scope, inline_code_in_scope, join_lines_in_scope,
    subtext_in_scope,
};
use super::vlasei::word_kind_label;
use crate::discord::request::{DiscordTool, VlataiOptions, VlataiRequest};

#[requires(true)]
#[ensures(ret.mark_stress == if options.mark_stress { StressMark::Acute } else { StressMark::None })]
pub(crate) fn phoneme_options(options: VlataiOptions) -> PhonemeRenderOptions {
    PhonemeRenderOptions {
        mark_stress: if options.mark_stress {
            StressMark::Acute
        } else {
            StressMark::None
        },
        mark_glides: if options.mark_glides {
            GlideMark::Breve
        } else {
            GlideMark::None
        },
    }
}

#[requires(true)]
#[ensures(text_scope.is_exceeded() || (ret.tool() == DiscordTool::Vlatai))]
pub(crate) fn render_in_scope(
    report: &VlataiReport,
    request: &VlataiRequest,
    text_scope: &jbotci_output::budget::TextScope,
) -> RenderedResult {
    let options = request.options;
    let phonemes = phoneme_options(options);
    let input = report.analysis.input.as_str();
    let status_word = match report.analysis.result.status {
        ValsiAnalysisStatus::Valid => "valid",
        ValsiAnalysisStatus::Invalid => "invalid",
        ValsiAnalysisStatus::NotSingleWord => "not a single word",
    };
    let mut status =
        text_scope.buffer_with(&text_scope.format(format_args!("vlatai · {status_word}")));
    if let Some(counts) = summary_in_scope(&report.diagnostics, text_scope) {
        status.push_str(&text_scope.format(format_args!(" · {counts}")));
    }
    let mut rendered = RenderedResult::new_in_scope(DiscordTool::Vlatai, status, text_scope);
    let mut lines = Vec::new();
    match report.analysis.result.status {
        ValsiAnalysisStatus::Valid => {
            let classification = report
                .analysis
                .result
                .classification
                .as_ref()
                .expect("valid vlatai result carries classification");
            lines.push(text_scope.format(format_args!(
                "**{}** — valid",
                escape_in_scope(input, text_scope)
            )));
            lines.extend(classification_lines_in_scope(
                classification,
                &report.possible_rafsi,
                phonemes,
                options.show_details,
                "",
                text_scope,
            ));
        }
        ValsiAnalysisStatus::Invalid => {
            lines.push(text_scope.format(format_args!(
                "**{}** — not a valid Lojban word",
                escape_in_scope(input, text_scope)
            )));
        }
        ValsiAnalysisStatus::NotSingleWord => {
            let words = &report.analysis.result.words;
            lines.push(text_scope.format(format_args!(
                "**{}** — {} word{}, not one",
                escape_in_scope(input, text_scope),
                words.len(),
                if words.len() == 1 { "" } else { "s" }
            )));
            if !words.is_empty() {
                let brackets = pretty_morphology_brackets_with_limit(
                    words,
                    input,
                    BracketRenderOptions {
                        color: false,
                        phonemes,
                        script: LojbanScript::Latin,
                        glyphs: GlyphStyle::Unicode,
                        decompose_lujvo: options.show_details,
                        insert_hair_space: false,
                        show_elided: false,
                    },
                    text_scope.limit(),
                );
                let brackets = super::render_output_in_scope(brackets, text_scope);
                lines.push(code_block_in_scope(&brackets, text_scope));
                lines.push(subtext_in_scope(
                    "Use vlasei for the full segmentation.",
                    text_scope,
                ));
            }
        }
    }
    rendered.body.push(join_lines_in_scope(lines, text_scope));
    rendered.set_diagnostics(render_diagnostics_in_scope(
        input,
        &report.diagnostics,
        text_scope,
    ));
    rendered
}

/// Lines describing a classification; `prefix` labels nested parts.
#[requires(true)]
#[ensures(text_scope.is_exceeded() || (!ret.is_empty()))]
fn classification_lines_in_scope(
    classification: &ValsiClassification,
    possible_rafsi: &[String],
    phonemes: PhonemeRenderOptions,
    show_details: bool,
    prefix: &str,
    text_scope: &jbotci_output::budget::TextScope,
) -> Vec<String> {
    let mut lines = Vec::new();
    match classification.kind() {
        ValsiClassificationKind::PlainWord => {
            let word = classification
                .word()
                .expect("plain-word classification carries word");
            lines.extend(plain_word_lines_in_scope(
                word,
                possible_rafsi,
                phonemes,
                show_details,
                prefix,
                text_scope,
            ));
        }
        ValsiClassificationKind::QuotedWord => {
            lines.push(text_scope.format(format_args!("{prefix}quoted word")));
            lines.extend(plain_word_lines_in_scope(
                classification.marker().expect("marker"),
                &[],
                phonemes,
                show_details,
                "marker: ",
                text_scope,
            ));
            lines.extend(plain_word_lines_in_scope(
                classification.quoted_word().expect("quoted"),
                &[],
                phonemes,
                show_details,
                "quoted: ",
                text_scope,
            ));
        }
        ValsiClassificationKind::DelimitedNonLojbanQuote => {
            lines.push(text_scope.format(format_args!("{prefix}delimited non-Lojban quote")));
            lines.extend(plain_word_lines_in_scope(
                classification.marker().expect("marker"),
                &[],
                phonemes,
                show_details,
                "marker: ",
                text_scope,
            ));
            lines.push(text_scope.format(format_args!(
                "delimiter: {}",
                inline_code_in_scope(classification.delimiter().expect("delimiter"), text_scope)
            )));
        }
        ValsiClassificationKind::QuotedWords => {
            lines.push(text_scope.format(format_args!(
                "{prefix}quoted words ({})",
                classification.quoted_words().len()
            )));
            lines.extend(plain_word_lines_in_scope(
                classification.marker().expect("marker"),
                &[],
                phonemes,
                show_details,
                "marker: ",
                text_scope,
            ));
        }
        ValsiClassificationKind::ReplacementQuote => {
            lines.push(text_scope.format(format_args!(
                "{prefix}replacement quote ({} raw words)",
                classification.replacement_words().len()
            )));
        }
        ValsiClassificationKind::DelimitedWordQuote => {
            lines.push(text_scope.format(format_args!(
                "{prefix}delimited word quote, marker {}",
                inline_code_in_scope(
                    classification.marker_text().expect("marker text"),
                    text_scope
                )
            )));
        }
        ValsiClassificationKind::LerfuWord => {
            lines.push(text_scope.format(format_args!("{prefix}lerfu word")));
            lines.extend(classification_lines_in_scope(
                classification.base().expect("base"),
                &[],
                phonemes,
                show_details,
                "base: ",
                text_scope,
            ));
            lines.extend(plain_word_lines_in_scope(
                classification.suffix().expect("suffix"),
                &[],
                phonemes,
                show_details,
                "suffix: ",
                text_scope,
            ));
        }
        ValsiClassificationKind::ZeiCompound => {
            lines.push(text_scope.format(format_args!("{prefix}zei compound")));
            lines.extend(classification_lines_in_scope(
                classification.left().expect("left"),
                &[],
                phonemes,
                show_details,
                "left: ",
                text_scope,
            ));
            lines.extend(plain_word_lines_in_scope(
                classification.link().expect("link"),
                &[],
                phonemes,
                show_details,
                "link: ",
                text_scope,
            ));
            lines.extend(plain_word_lines_in_scope(
                classification.right().expect("right"),
                &[],
                phonemes,
                show_details,
                "right: ",
                text_scope,
            ));
        }
    }
    lines
}

#[requires(true)]
#[ensures(text_scope.is_exceeded() || (!ret.is_empty()))]
fn plain_word_lines_in_scope(
    word: &PlainWordClassification,
    possible_rafsi: &[String],
    phonemes: PhonemeRenderOptions,
    show_details: bool,
    prefix: &str,
    text_scope: &jbotci_output::budget::TextScope,
) -> Vec<String> {
    let mut head = text_scope.buffer_with(
        &text_scope.format(format_args!("{prefix}{}", word_kind_label(word.category))),
    );
    if let Some(selmaho) = &word.selmaho {
        head.push_str(&text_scope.format(format_args!(
            " · selma'o {}",
            escape_in_scope(selmaho, text_scope)
        )));
    }
    if let Some(stage) = word.stage {
        head.push_str(&text_scope.format(format_args!(
            " · {}",
            match stage {
                ValsiFuhivlaStage::Stage3 => "stage 3",
                ValsiFuhivlaStage::Stage4 => "stage 4",
                ValsiFuhivlaStage::Unknown => "stage unknown",
            }
        )));
    }
    let mut rendered_phonemes = text_scope.buffer();
    if Phonemes::render_canonical_to(&word.phonemes, phonemes, &mut rendered_phonemes).is_err()
        && !text_scope.is_exceeded()
    {
        rendered_phonemes = text_scope.buffer_with(&word.phonemes);
    }
    let rendered_phonemes = rendered_phonemes.into_piece();
    head.push_str(&text_scope.format(format_args!(
        " · phonemes {}",
        inline_code_in_scope(&rendered_phonemes, text_scope)
    )));
    let mut lines = vec![head.into_piece()];
    if show_details {
        if word.category == WordKind::Gismu && !possible_rafsi.is_empty() {
            lines.push(subtext_in_scope(
                &text_scope.format(format_args!(
                    "possible short rafsi (by form only, not dictionary assignment): {}",
                    text_scope.join(possible_rafsi, " ")
                )),
                text_scope,
            ));
        }
        if word.category == WordKind::Lujvo {
            if let Some(split) = &word.split {
                lines.push(subtext_in_scope(
                    &text_scope.format(format_args!("split: {split}")),
                    text_scope,
                ));
            }
            if !word.parts.is_empty() {
                let parts = word
                    .parts
                    .iter()
                    .map(|part| match part.kind {
                        ValsiLujvoPartKind::Hyphen => {
                            text_scope.format(format_args!("-{}-", part.text))
                        }
                        ValsiLujvoPartKind::Rafsi => text_scope.format(format_args!(
                            "{} ({})",
                            part.text,
                            part.rafsi_kind.map(rafsi_kind_label).unwrap_or("rafsi")
                        )),
                    })
                    .collect::<Vec<_>>();
                lines.push(subtext_in_scope(
                    &text_scope.format(format_args!("parts: {}", text_scope.join(&parts, " + "))),
                    text_scope,
                ));
            }
        }
    }
    lines
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn rafsi_kind_label(kind: ValsiLujvoRafsiKind) -> &'static str {
    match kind {
        ValsiLujvoRafsiKind::Cvc => "CVC rafsi",
        ValsiLujvoRafsiKind::Ccv => "CCV rafsi",
        ValsiLujvoRafsiKind::Cvv => "CVV rafsi",
        ValsiLujvoRafsiKind::Long => "long rafsi",
        ValsiLujvoRafsiKind::Gismu => "gismu",
        ValsiLujvoRafsiKind::Fuhivla => "fu'ivla",
        ValsiLujvoRafsiKind::Cultural => "cultural rafsi",
        ValsiLujvoRafsiKind::Extended => "extended rafsi",
        ValsiLujvoRafsiKind::Unknown => "unknown rafsi",
    }
}

#[cfg(test)]
#[requires(true)]
#[ensures(true)]
pub(crate) fn render(report: &VlataiReport, request: &VlataiRequest) -> RenderedResult {
    let text_scope = jbotci_output::budget::TextScope::new(
        jbotci_output::budget::ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    );
    let result = render_in_scope(report, request, &text_scope);
    text_scope
        .finish(String::new())
        .expect("unrestricted Discord rendering");
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discord::request::SourceText;
    use jbotci_morphology::MorphologyOptions;
    use jbotci_web_core::analyze_vlatai;

    #[requires(true)]
    #[ensures(true)]
    fn request(text: &str, options: VlataiOptions) -> VlataiRequest {
        VlataiRequest {
            text: SourceText::new(text).expect("text"),
            dialect: None,
            options,
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn gismu_shows_class_phonemes_and_possible_rafsi_without_claiming_assignment() {
        let default_request = request("klama", VlataiOptions::default());
        let report = analyze_vlatai("klama", &MorphologyOptions::default(), None).expect("report");
        let rendered = render(&report, &default_request);
        assert_eq!(rendered.status.text, "vlatai · valid");
        let body = rendered.body.join("\n");
        assert!(body.contains("gismu"), "{body}");
        assert!(
            body.contains("`klá.ma`") || body.contains("`kláma`") || body.contains("`kla"),
            "{body}"
        );
        assert!(
            body.contains("possible short rafsi \\(by form only"),
            "{body}"
        );
        assert!(body.contains("kla"), "{body}");
        // Details off hides the formation facts but keeps the class.
        let quiet = render(
            &report,
            &request(
                "klama",
                VlataiOptions {
                    show_details: false,
                    ..VlataiOptions::default()
                },
            ),
        );
        assert!(!quiet.body.join("\n").contains("possible short rafsi"));
        // Stress marks follow the option.
        let unmarked = render(
            &report,
            &request(
                "klama",
                VlataiOptions {
                    mark_stress: false,
                    ..VlataiOptions::default()
                },
            ),
        );
        assert!(
            !unmarked.body.join("\n").contains('á'),
            "{}",
            unmarked.body.join("\n")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn invalid_and_multiword_inputs_are_distinct() {
        let options = MorphologyOptions::default();
        let invalid = render(
            &analyze_vlatai("'klama", &options, None).expect("report"),
            &request("'klama", VlataiOptions::default()),
        );
        assert_eq!(invalid.status.text.split(" · ").nth(1), Some("invalid"));
        assert!(invalid.diagnostics.is_some());
        let multi = render(
            &analyze_vlatai("mi klama", &options, None).expect("report"),
            &request("mi klama", VlataiOptions::default()),
        );
        assert!(
            multi.status.text.contains("not a single word"),
            "{}",
            multi.status.text
        );
        let body = multi.body.join("\n");
        assert!(body.contains("2 words, not one"), "{body}");
        assert!(body.contains("```"), "segmentation shown: {body}");
        let lujvo = render(
            &analyze_vlatai("klabajra", &options, None).expect("report"),
            &request("klabajra", VlataiOptions::default()),
        );
        let body = lujvo.body.join("\n");
        assert!(body.contains("lujvo"), "{body}");
        assert!(body.contains("parts\\:"), "{body}");
    }
}
