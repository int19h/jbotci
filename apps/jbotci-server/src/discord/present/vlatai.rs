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
    BracketRenderOptions, GlyphStyle, LojbanScript, pretty_morphology_brackets_with_options,
};
use jbotci_web_core::VlataiReport;

use super::RenderedResult;
use super::diagnostics::{render_diagnostics, summary};
use super::markdown::{code_block, escape, inline_code, join_lines, subtext};
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
#[ensures(ret.tool() == DiscordTool::Vlatai)]
pub(crate) fn render(report: &VlataiReport, request: &VlataiRequest) -> RenderedResult {
    let options = request.options;
    let phonemes = phoneme_options(options);
    let input = report.analysis.input.as_str();
    let status_word = match report.analysis.result.status {
        ValsiAnalysisStatus::Valid => "valid",
        ValsiAnalysisStatus::Invalid => "invalid",
        ValsiAnalysisStatus::NotSingleWord => "not a single word",
    };
    let mut status = format!("vlatai · {status_word}");
    if let Some(counts) = summary(&report.diagnostics) {
        status.push_str(&format!(" · {counts}"));
    }
    let mut rendered = RenderedResult::new(DiscordTool::Vlatai, status);
    let mut lines = Vec::new();
    match report.analysis.result.status {
        ValsiAnalysisStatus::Valid => {
            let classification = report
                .analysis
                .result
                .classification
                .as_ref()
                .expect("valid vlatai result carries classification");
            lines.push(format!("**{}** — valid", escape(input)));
            lines.extend(classification_lines(
                classification,
                &report.possible_rafsi,
                phonemes,
                options.show_details,
                "",
            ));
        }
        ValsiAnalysisStatus::Invalid => {
            lines.push(format!("**{}** — not a valid Lojban word", escape(input)));
        }
        ValsiAnalysisStatus::NotSingleWord => {
            let words = &report.analysis.result.words;
            lines.push(format!(
                "**{}** — {} word{}, not one",
                escape(input),
                words.len(),
                if words.len() == 1 { "" } else { "s" }
            ));
            if !words.is_empty() {
                let brackets = pretty_morphology_brackets_with_options(
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
                )
                .unwrap_or_else(|error| error.to_string());
                lines.push(code_block(&brackets));
                lines.push(subtext("Use vlasei for the full segmentation."));
            }
        }
    }
    rendered.body.push(join_lines(lines));
    rendered.diagnostics = render_diagnostics(input, &report.diagnostics);
    rendered
}

/// Lines describing a classification; `prefix` labels nested parts.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn classification_lines(
    classification: &ValsiClassification,
    possible_rafsi: &[String],
    phonemes: PhonemeRenderOptions,
    show_details: bool,
    prefix: &str,
) -> Vec<String> {
    let mut lines = Vec::new();
    match classification.kind() {
        ValsiClassificationKind::PlainWord => {
            let word = classification
                .word()
                .expect("plain-word classification carries word");
            lines.extend(plain_word_lines(
                word,
                possible_rafsi,
                phonemes,
                show_details,
                prefix,
            ));
        }
        ValsiClassificationKind::QuotedWord => {
            lines.push(format!("{prefix}quoted word"));
            lines.extend(plain_word_lines(
                classification.marker().expect("marker"),
                &[],
                phonemes,
                show_details,
                "marker: ",
            ));
            lines.extend(plain_word_lines(
                classification.quoted_word().expect("quoted"),
                &[],
                phonemes,
                show_details,
                "quoted: ",
            ));
        }
        ValsiClassificationKind::DelimitedNonLojbanQuote => {
            lines.push(format!("{prefix}delimited non-Lojban quote"));
            lines.extend(plain_word_lines(
                classification.marker().expect("marker"),
                &[],
                phonemes,
                show_details,
                "marker: ",
            ));
            lines.push(format!(
                "delimiter: {}",
                inline_code(classification.delimiter().expect("delimiter"))
            ));
        }
        ValsiClassificationKind::QuotedWords => {
            lines.push(format!(
                "{prefix}quoted words ({})",
                classification.quoted_words().len()
            ));
            lines.extend(plain_word_lines(
                classification.marker().expect("marker"),
                &[],
                phonemes,
                show_details,
                "marker: ",
            ));
        }
        ValsiClassificationKind::DelimitedWordQuote => {
            lines.push(format!(
                "{prefix}delimited word quote, marker {}",
                inline_code(classification.marker_text().expect("marker text"))
            ));
        }
        ValsiClassificationKind::LerfuWord => {
            lines.push(format!("{prefix}lerfu word"));
            lines.extend(classification_lines(
                classification.base().expect("base"),
                &[],
                phonemes,
                show_details,
                "base: ",
            ));
            lines.extend(plain_word_lines(
                classification.suffix().expect("suffix"),
                &[],
                phonemes,
                show_details,
                "suffix: ",
            ));
        }
        ValsiClassificationKind::ZeiCompound => {
            lines.push(format!("{prefix}zei compound"));
            lines.extend(classification_lines(
                classification.left().expect("left"),
                &[],
                phonemes,
                show_details,
                "left: ",
            ));
            lines.extend(plain_word_lines(
                classification.link().expect("link"),
                &[],
                phonemes,
                show_details,
                "link: ",
            ));
            lines.extend(plain_word_lines(
                classification.right().expect("right"),
                &[],
                phonemes,
                show_details,
                "right: ",
            ));
        }
    }
    lines
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn plain_word_lines(
    word: &PlainWordClassification,
    possible_rafsi: &[String],
    phonemes: PhonemeRenderOptions,
    show_details: bool,
    prefix: &str,
) -> Vec<String> {
    let mut head = format!("{prefix}{}", word_kind_label(word.category));
    if let Some(selmaho) = &word.selmaho {
        head.push_str(&format!(" · selma'o {}", escape(selmaho)));
    }
    if let Some(stage) = word.stage {
        head.push_str(&format!(
            " · {}",
            match stage {
                ValsiFuhivlaStage::Stage3 => "stage 3",
                ValsiFuhivlaStage::Stage4 => "stage 4",
                ValsiFuhivlaStage::Unknown => "stage unknown",
            }
        ));
    }
    let rendered_phonemes = Phonemes::render_canonical(&word.phonemes, phonemes)
        .unwrap_or_else(|_| word.phonemes.clone());
    head.push_str(&format!(" · phonemes {}", inline_code(&rendered_phonemes)));
    let mut lines = vec![head];
    if show_details {
        if word.category == WordKind::Gismu && !possible_rafsi.is_empty() {
            lines.push(subtext(&format!(
                "possible short rafsi (by form only, not dictionary assignment): {}",
                possible_rafsi.join(" ")
            )));
        }
        if word.category == WordKind::Lujvo {
            if let Some(split) = &word.split {
                lines.push(subtext(&format!("split: {split}")));
            }
            if !word.parts.is_empty() {
                let parts = word
                    .parts
                    .iter()
                    .map(|part| match part.kind {
                        ValsiLujvoPartKind::Hyphen => format!("-{}-", part.text),
                        ValsiLujvoPartKind::Rafsi => format!(
                            "{} ({})",
                            part.text,
                            part.rafsi_kind.map(rafsi_kind_label).unwrap_or("rafsi")
                        ),
                    })
                    .collect::<Vec<_>>();
                lines.push(subtext(&format!("parts: {}", parts.join(" + "))));
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
