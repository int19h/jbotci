use super::super::*;
use bityzba::new;
#[cfg(test)]
use bityzba::try_new;
use jbotci_output::Indentation;

#[requires(diagnostic_terminal_width > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn run_vlatai<WOut: Write>(
    input: VlataiInput,
    stdout: &mut WOut,
    color_policy: CliColorPolicy,
    diagnostic_terminal_width: usize,
    cancellation: &jbotci_interrupt::CancellationToken,
) -> Result<CliStatus> {
    let glyphs = cli_glyph_style(input.ascii);
    let diagnostic_detail = cli_diagnostic_detail(input.detailed_errors);
    let dialect = input.dialect_definition()?;
    let morphology_options = MorphologyOptions::default()
        .try_with_dialect_definition(&dialect)?
        .with_cancellation(cancellation);
    let phoneme_options = phoneme_render_options(input.mark_stress, input.mark_glides, glyphs);
    let analyses = input
        .words
        .iter()
        .enumerate()
        .map(|(index, word)| {
            let source_label = vlatai_source_label(index);
            analyze_valsi_with_options_and_source_id(
                word,
                &morphology_options,
                Some(SourceId(source_label)),
            )
        })
        .collect::<Vec<_>>();
    let status = if analyses.iter().all(|analysis| analysis.result.is_valid()) {
        CliStatus::Success
    } else {
        CliStatus::Failure
    };
    match input.format {
        VlataiFormat::Text => {
            let rendered = render_vlatai_text(
                &analyses,
                phoneme_options,
                color_policy.stdout,
                diagnostic_detail,
                glyphs,
                diagnostic_terminal_width,
            )?;
            stdout.write_all(rendered.as_bytes())?;
        }
        VlataiFormat::Json => {
            let rendered = render_vlatai_json(
                &analyses,
                phoneme_options,
                input.indent.unwrap_or_default(),
                color_policy.stdout,
            )?;
            writeln!(stdout, "{rendered}")?;
        }
    }
    Ok(status)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(crate) fn vlatai_source_label(index: usize) -> String {
    format!("<arg:{}>", index + 1)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn render_vlatai_text(
    analyses: &[ValsiAnalysis],
    phoneme_options: PhonemeRenderOptions,
    color_enabled: bool,
    diagnostic_detail: DiagnosticDetailMode,
    glyphs: GlyphStyle,
    diagnostic_terminal_width: usize,
) -> Result<String> {
    let mut out = String::new();
    for (index, analysis) in analyses.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let source_label = vlatai_source_label(index);
        out.push_str(&format!("valsi: {}\n", analysis.input));
        out.push_str(&format!("status: {}\n", vlatai_status(analysis)));
        let diagnostics = vlatai_diagnostics(analysis, Some(SourceId(source_label.clone())))?;
        out.push_str(&render_source_diagnostics(
            &source_label,
            &analysis.input,
            &diagnostics,
            color_enabled,
            diagnostic_detail,
            glyphs,
            diagnostic_terminal_width,
        )?);
        match analysis.result.status {
            ValsiAnalysisStatus::Valid => {
                let classification = analysis
                    .result
                    .classification
                    .as_ref()
                    .expect("valid vlatai result carries classification");
                render_vlatai_classification_text(&mut out, classification, phoneme_options);
            }
            ValsiAnalysisStatus::NotSingleWord => {
                let rendered = pretty_morphology_brackets_with_options(
                    &analysis.result.words,
                    &analysis.input,
                    BracketRenderOptions {
                        color: color_enabled,
                        phonemes: phoneme_options,
                        script: LojbanScript::Latin,
                        glyphs,
                        decompose_lujvo: true,
                        insert_hair_space: false,
                        show_elided: false,
                    },
                )?;
                out.push_str(&format!("words: {rendered}\n"));
            }
            ValsiAnalysisStatus::Invalid => {}
            // A cancelled word is not an invalid word. The tool boundary
            // returns `Interrupted` before this renderer runs.
            ValsiAnalysisStatus::Interrupted => {
                return Err(jbotci_interrupt::Interrupted.into());
            }
        }
    }
    Ok(out)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(crate) fn render_vlatai_json(
    analyses: &[ValsiAnalysis],
    phoneme_options: PhonemeRenderOptions,
    indent: Indentation,
    color: bool,
) -> Result<String> {
    let reports = analyses
        .iter()
        .enumerate()
        .map(|(index, analysis)| vlatai_json_value(index, analysis, phoneme_options))
        .collect::<Result<Vec<_>>>()?;
    let value = serde_json::Value::Array(reports);
    Ok(render_json_value_with_options(
        &value,
        JsonRenderOptions {
            indent,
            color,
            ..JsonRenderOptions::default()
        },
    ))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok() || ret.is_err())]
fn vlatai_json_value(
    index: usize,
    analysis: &ValsiAnalysis,
    phoneme_options: PhonemeRenderOptions,
) -> Result<serde_json::Value> {
    let diagnostics = vlatai_diagnostics(analysis, Some(SourceId(vlatai_source_label(index))))?;
    let mut value = serde_json::json!({
        "input": analysis.input,
        "status": vlatai_status(analysis),
        "diagnostics": diagnostics,
    });
    match analysis.result.status {
        ValsiAnalysisStatus::Valid => {
            let word = analysis
                .result
                .word
                .as_ref()
                .expect("valid vlatai result carries word");
            let classification = analysis
                .result
                .classification
                .as_ref()
                .expect("valid vlatai result carries classification");
            value["classification"] = vlatai_classification_json(classification, phoneme_options);
            value["word"] = compact_morphology_json_value(std::slice::from_ref(word))?;
        }
        ValsiAnalysisStatus::Invalid => {}
        ValsiAnalysisStatus::NotSingleWord => {
            value["words"] = compact_morphology_json_value(&analysis.result.words)?;
        }
        ValsiAnalysisStatus::Interrupted => {
            return Err(jbotci_interrupt::Interrupted.into());
        }
    }
    Ok(value)
}

#[requires(true)]
#[ensures(matches!(ret, "valid" | "invalid" | "not-single-word" | "interrupted"))]
fn vlatai_status(analysis: &ValsiAnalysis) -> &'static str {
    match analysis.result.status {
        ValsiAnalysisStatus::Valid => "valid",
        ValsiAnalysisStatus::Invalid => "invalid",
        ValsiAnalysisStatus::NotSingleWord => "not-single-word",
        ValsiAnalysisStatus::Interrupted => "interrupted",
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok() || ret.is_err())]
fn vlatai_diagnostics(
    analysis: &ValsiAnalysis,
    source_id: Option<SourceId>,
) -> Result<Vec<Diagnostic>> {
    jbotci_web_core::vlatai_diagnostics(analysis, source_id).map_err(|error| anyhow!(error))
}

#[requires(true)]
#[ensures(true)]
fn render_vlatai_classification_text(
    out: &mut String,
    classification: &ValsiClassification,
    phoneme_options: PhonemeRenderOptions,
) {
    render_vlatai_classification_text_with_prefix(out, classification, phoneme_options, "");
}

/// A suffix frame names a cmavo. A zei frame names a cmavo link and a word with phonemes.
#[invariant(::Suffix(suffix) => suffix.category == WordKind::Cmavo)]
#[invariant(::Zei { link, right } => link.category == WordKind::Cmavo && !right.phonemes.is_empty())]
enum ClassAfter<'a> {
    Suffix(&'a PlainWordClassification),
    Zei {
        link: &'a PlainWordClassification,
        right: &'a PlainWordClassification,
    },
}

#[requires(true)]
#[ensures(true)]
fn render_vlatai_classification_text_with_prefix(
    out: &mut String,
    classification: &ValsiClassification,
    phoneme_options: PhonemeRenderOptions,
    prefix: &str,
) {
    let mut current = classification;
    let mut prefix_now = prefix.to_owned();
    let mut afters = Vec::new();
    loop {
        if current.kind() == ValsiClassificationKind::LerfuWord {
            out.push_str(&format!("{prefix_now}category: lerfu-word\n"));
            let base = current.base().expect("lerfu base");
            let suffix = current.suffix().expect("lerfu suffix");
            afters.push(new!(ClassAfter::Suffix(suffix)));
            current = base;
            prefix_now = "base ".to_owned();
            continue;
        }
        if current.kind() == ValsiClassificationKind::ZeiCompound {
            out.push_str(&format!("{prefix_now}category: zei-compound\n"));
            let left = current.left().expect("zei left");
            let link = current.link().expect("zei link");
            let right = current.right().expect("zei right");
            afters.push(new!(ClassAfter::Zei { link, right }));
            current = left;
            prefix_now = "left ".to_owned();
            continue;
        }
        let prefix = prefix_now.as_str();
        match current.kind() {
            ValsiClassificationKind::PlainWord => {
                render_plain_word_classification_text(
                    out,
                    current
                        .word()
                        .expect("plain-word classification carries word"),
                    phoneme_options,
                    prefix,
                );
            }
            ValsiClassificationKind::QuotedWord => {
                out.push_str(&format!("{prefix}category: quoted-word\n"));
                render_plain_word_classification_text(
                    out,
                    current.marker().expect("quoted word marker"),
                    phoneme_options,
                    "marker ",
                );
                render_plain_word_classification_text(
                    out,
                    current.quoted_word().expect("quoted word payload"),
                    phoneme_options,
                    "quoted ",
                );
            }
            ValsiClassificationKind::DelimitedNonLojbanQuote => {
                out.push_str(&format!("{prefix}category: delimited-non-lojban-quote\n"));
                render_plain_word_classification_text(
                    out,
                    current.marker().expect("quote marker"),
                    phoneme_options,
                    "marker ",
                );
                let delimiter = current
                    .delimiter()
                    .expect("delimited quote carries delimiter");
                out.push_str(&format!("{prefix}delimiter: {delimiter}\n"));
            }
            ValsiClassificationKind::QuotedWords => {
                out.push_str(&format!("{prefix}category: quoted-words\n"));
                render_plain_word_classification_text(
                    out,
                    current.marker().expect("quoted words marker"),
                    phoneme_options,
                    "marker ",
                );
                out.push_str(&format!(
                    "{prefix}quoted word count: {}\n",
                    current.quoted_words().len()
                ));
            }
            ValsiClassificationKind::ReplacementQuote => {
                out.push_str(&format!("{prefix}category: replacement-quote\n"));
                out.push_str(&format!(
                    "{prefix}raw word count: {}\n",
                    current.replacement_words().len()
                ));
            }
            ValsiClassificationKind::DelimitedWordQuote => {
                out.push_str(&format!("{prefix}category: delimited-word-quote\n"));
                out.push_str(&format!(
                    "{prefix}marker: {}\n",
                    current.marker_text().expect("delimited word quote marker")
                ));
            }
            ValsiClassificationKind::LerfuWord | ValsiClassificationKind::ZeiCompound => {
                unreachable!("lerfu and zei stay on the classification spine");
            }
        }
        break;
    }
    while let Some(after) = afters.pop() {
        match after.into_data() {
            ClassAfterData::Suffix(suffix) => {
                render_plain_word_classification_text(out, suffix, phoneme_options, "suffix ");
            }
            ClassAfterData::Zei { link, right } => {
                render_plain_word_classification_text(out, link, phoneme_options, "link ");
                render_plain_word_classification_text(out, right, phoneme_options, "right ");
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn render_plain_word_classification_text(
    out: &mut String,
    classification: &PlainWordClassification,
    phoneme_options: PhonemeRenderOptions,
    prefix: &str,
) {
    match classification.category {
        WordKind::Cmavo => {
            out.push_str(&format!("{prefix}category: cmavo\n"));
            out.push_str(&format!(
                "{prefix}phonemes: {}\n",
                render_vlatai_phonemes(&classification.phonemes, phoneme_options)
            ));
            if let Some(selmaho) = &classification.selmaho {
                out.push_str(&format!("{prefix}selma'o: {selmaho}\n"));
            }
        }
        WordKind::Gismu => {
            out.push_str(&format!("{prefix}category: gismu\n"));
            out.push_str(&format!(
                "{prefix}phonemes: {}\n",
                render_vlatai_phonemes(&classification.phonemes, phoneme_options)
            ));
            out.push_str(&format!(
                "{prefix}possible rafsi: {}\n",
                vlatai_possible_rafsi(classification).join(" ")
            ));
        }
        WordKind::Lujvo => {
            out.push_str(&format!("{prefix}category: lujvo\n"));
            out.push_str(&format!(
                "{prefix}phonemes: {}\n",
                render_vlatai_phonemes(&classification.phonemes, phoneme_options)
            ));
            let split = classification
                .split
                .as_ref()
                .expect("lujvo classification carries split");
            out.push_str(&format!("{prefix}split: {split}\n"));
            out.push_str(&format!("{prefix}parts:\n"));
            for part in &classification.parts {
                out.push_str(&format!("{prefix}  - {}\n", vlatai_lujvo_part_text(part)));
            }
        }
        WordKind::Fuhivla => {
            out.push_str(&format!("{prefix}category: fu'ivla\n"));
            out.push_str(&format!(
                "{prefix}phonemes: {}\n",
                render_vlatai_phonemes(&classification.phonemes, phoneme_options)
            ));
            let stage = classification
                .stage
                .expect("fu'ivla classification carries stage");
            out.push_str(&format!("{prefix}stage: {}\n", vlatai_fuhivla_stage(stage)));
        }
        WordKind::Cmevla => {
            out.push_str(&format!("{prefix}category: cmevla\n"));
            out.push_str(&format!(
                "{prefix}phonemes: {}\n",
                render_vlatai_phonemes(&classification.phonemes, phoneme_options)
            ));
        }
    }
}

#[requires(!part.text.is_empty())]
#[ensures(!ret.is_empty())]
fn vlatai_lujvo_part_text(part: &ValsiLujvoPart) -> String {
    match part.kind {
        ValsiLujvoPartKind::Hyphen => format!("hyphen: {}", part.text),
        ValsiLujvoPartKind::Rafsi => format!(
            "rafsi: {} ({})",
            part.text,
            part.rafsi_kind
                .map(vlatai_rafsi_kind)
                .unwrap_or("unknown-rafsi")
        ),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vlatai_rafsi_kind(kind: ValsiLujvoRafsiKind) -> &'static str {
    match kind {
        ValsiLujvoRafsiKind::Cvc => "cvc-rafsi",
        ValsiLujvoRafsiKind::Ccv => "ccv-rafsi",
        ValsiLujvoRafsiKind::Cvv => "cvv-rafsi",
        ValsiLujvoRafsiKind::Long => "long-rafsi",
        ValsiLujvoRafsiKind::Gismu => "gismu",
        ValsiLujvoRafsiKind::Fuhivla => "fu'ivla",
        ValsiLujvoRafsiKind::Cultural => "cultural-rafsi",
        ValsiLujvoRafsiKind::Extended => "extended-rafsi",
        ValsiLujvoRafsiKind::Unknown => "unknown-rafsi",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vlatai_fuhivla_stage(stage: ValsiFuhivlaStage) -> &'static str {
    match stage {
        ValsiFuhivlaStage::Stage3 => "stage-3",
        ValsiFuhivlaStage::Stage4 => "stage-4",
        ValsiFuhivlaStage::Unknown => "unknown",
    }
}

#[requires(true)]
#[ensures(true)]
fn vlatai_classification_json(
    classification: &ValsiClassification,
    phoneme_options: PhonemeRenderOptions,
) -> serde_json::Value {
    match classification.kind() {
        ValsiClassificationKind::PlainWord => plain_word_classification_json(
            classification
                .word()
                .expect("plain-word classification carries word"),
            phoneme_options,
        ),
        _ => serde_json::to_value(classification).expect("vlatai classification serializes"),
    }
}

#[requires(true)]
#[ensures(true)]
fn plain_word_classification_json(
    classification: &PlainWordClassification,
    phoneme_options: PhonemeRenderOptions,
) -> serde_json::Value {
    let mut value =
        serde_json::to_value(classification).expect("plain word classification serializes");
    value["phonemes"] = serde_json::Value::String(render_vlatai_phonemes(
        &classification.phonemes,
        phoneme_options,
    ));
    // Derivable from the word itself, so it is injected here instead of taxing
    // every `PlainWordClassification` produced by vlasei and the IDE.
    if classification.category == WordKind::Gismu {
        value["possible-rafsi"] = serde_json::Value::Array(
            vlatai_possible_rafsi(classification)
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        );
    }
    value
}

/// The short rafsi a gismu classification could claim (shared with the other
/// surfaces; formation possibility, not dictionary availability).
#[requires(classification.category == WordKind::Gismu)]
#[ensures(ret.windows(2).all(|pair| pair[0] < pair[1]))]
fn vlatai_possible_rafsi(classification: &PlainWordClassification) -> Vec<String> {
    jbotci_web_core::possible_rafsi_for_gismu(classification)
}

#[requires(true)]
#[ensures(!ret.is_empty() || phonemes.is_empty())]
fn render_vlatai_phonemes(phonemes: &str, options: PhonemeRenderOptions) -> String {
    Phonemes::from_canonical(phonemes.to_owned())
        .map(|value| value.render(options))
        .unwrap_or_else(|_| phonemes.to_owned())
}

#[cfg(test)]
mod class_after_invariants {
    use super::*;

    #[requires(true)]
    #[ensures(true)]
    fn gismu() -> PlainWordClassification {
        new!(PlainWordClassification {
            category: WordKind::Gismu,
            phonemes: "broda".to_owned(),
            selmaho: None,
            split: None,
            parts: Vec::new(),
            stage: None,
        })
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_suffix_frame_rejects_a_gismu() {
        let word = gismu();
        assert!(try_new!(ClassAfter::Suffix(&word)).is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_zei_frame_rejects_a_gismu_link() {
        let link = gismu();
        let right = gismu();
        assert!(
            try_new!(ClassAfter::Zei {
                link: &link,
                right: &right
            })
            .is_err()
        );
    }
}

#[cfg(test)]
mod deep_links {
    use std::thread;

    use bityzba::requires;
    use jbotci_morphology::{PhonemeRenderOptions, ValsiClassification, analyze_valsi};

    use super::render_vlatai_classification_text;

    #[requires(links > 0)]
    #[ensures(true)]
    fn classification(kind: &str, links: usize) -> ValsiClassification {
        let mut text = if kind == "bu" {
            String::from("a")
        } else {
            String::from("broda")
        };
        let piece = if kind == "bu" { " bu" } else { " zei broda" };
        for _ in 0..links {
            text.push_str(piece);
        }
        let kind = kind.to_owned();
        thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(move || {
                analyze_valsi(&text)
                    .result
                    .classification
                    .clone()
                    .unwrap_or_else(|| panic!("{kind} has no classification"))
            })
            .expect("parse thread")
            .join()
            .expect("parse join")
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn classification_text_fits_in_160kib() {
        let bu = classification("bu", 2666);
        let zei = classification("zei", 799);
        let options = PhonemeRenderOptions::default();
        let bu_text = thread::Builder::new()
            .stack_size(160 * 1024)
            .spawn(move || {
                let mut out = String::new();
                render_vlatai_classification_text(&mut out, &bu, options);
                out
            })
            .expect("render thread")
            .join()
            .expect("render join");
        let zei_text = thread::Builder::new()
            .stack_size(160 * 1024)
            .spawn(move || {
                let mut out = String::new();
                render_vlatai_classification_text(&mut out, &zei, options);
                out
            })
            .expect("render thread")
            .join()
            .expect("render join");
        assert_eq!(bu_text.matches("category: lerfu-word\n").count(), 2666);
        assert_eq!(zei_text.matches("category: zei-compound\n").count(), 799);
    }
}
