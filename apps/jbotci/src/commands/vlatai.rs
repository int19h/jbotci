use super::super::*;

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
        }
    }
    Ok(out)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
pub(crate) fn render_vlatai_json(
    analyses: &[ValsiAnalysis],
    phoneme_options: PhonemeRenderOptions,
    indent: usize,
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
    }
    Ok(value)
}

#[requires(true)]
#[ensures(matches!(ret, "valid" | "invalid" | "not-single-word"))]
fn vlatai_status(analysis: &ValsiAnalysis) -> &'static str {
    match analysis.result.status {
        ValsiAnalysisStatus::Valid => "valid",
        ValsiAnalysisStatus::Invalid => "invalid",
        ValsiAnalysisStatus::NotSingleWord => "not-single-word",
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok() || ret.is_err())]
fn vlatai_diagnostics(
    analysis: &ValsiAnalysis,
    source_id: Option<SourceId>,
) -> Result<Vec<Diagnostic>> {
    let mut diagnostics =
        morphology_warning_diagnostics(&analysis.warnings, source_id.clone(), &analysis.input);
    match analysis.result.status {
        ValsiAnalysisStatus::Invalid => {
            let error = analysis
                .result
                .error
                .as_ref()
                .expect("invalid vlatai result carries error");
            diagnostics.push(error.to_diagnostic(source_id, &analysis.input));
        }
        ValsiAnalysisStatus::NotSingleWord => {
            diagnostics.push(vlatai_not_single_word_diagnostic(
                source_id,
                &analysis.input,
                analysis.result.words.len(),
            )?);
        }
        ValsiAnalysisStatus::Valid => {}
    }
    Ok(diagnostics)
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error) || ret.is_err())]
fn vlatai_not_single_word_diagnostic(
    source_id: Option<SourceId>,
    source: &str,
    word_count: usize,
) -> Result<Diagnostic> {
    let char_end = source.chars().count();
    let span = source_span_from_char_offsets(source_id, source, 0, char_end)
        .map_err(|error| anyhow!(error))?;
    let (message, label) = if word_count == 0 {
        ("input did not parse as one word", "parsed zero words")
    } else {
        (
            "input parsed as multiple words",
            "parsed more than one word",
        )
    };
    Ok(Diagnostic::new(
        DiagnosticSeverity::Error,
        DiagnosticPhase::Morphology,
        "vlatai.not-single-word".to_owned(),
        message.to_owned(),
        vec![DiagnosticLabel::new(span, label.to_owned(), true)],
        vec![format!("parsed word count: {word_count}")],
        None,
    ))
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

#[requires(true)]
#[ensures(true)]
fn render_vlatai_classification_text_with_prefix(
    out: &mut String,
    classification: &ValsiClassification,
    phoneme_options: PhonemeRenderOptions,
    prefix: &str,
) {
    match classification.kind() {
        ValsiClassificationKind::PlainWord => {
            render_plain_word_classification_text(
                out,
                classification
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
                classification.marker().expect("quoted word marker"),
                phoneme_options,
                "marker ",
            );
            render_plain_word_classification_text(
                out,
                classification.quoted_word().expect("quoted word payload"),
                phoneme_options,
                "quoted ",
            );
        }
        ValsiClassificationKind::DelimitedNonLojbanQuote => {
            out.push_str(&format!("{prefix}category: delimited-non-lojban-quote\n"));
            render_plain_word_classification_text(
                out,
                classification.marker().expect("quote marker"),
                phoneme_options,
                "marker ",
            );
            let delimiter = classification
                .delimiter()
                .expect("delimited quote carries delimiter");
            out.push_str(&format!("{prefix}delimiter: {delimiter}\n"));
        }
        ValsiClassificationKind::QuotedWords => {
            out.push_str(&format!("{prefix}category: quoted-words\n"));
            render_plain_word_classification_text(
                out,
                classification.marker().expect("quoted words marker"),
                phoneme_options,
                "marker ",
            );
            out.push_str(&format!(
                "{prefix}quoted word count: {}\n",
                classification.quoted_words().len()
            ));
        }
        ValsiClassificationKind::DelimitedWordQuote => {
            out.push_str(&format!("{prefix}category: delimited-word-quote\n"));
            out.push_str(&format!(
                "{prefix}marker: {}\n",
                classification
                    .marker_text()
                    .expect("delimited word quote marker")
            ));
        }
        ValsiClassificationKind::LerfuWord => {
            out.push_str(&format!("{prefix}category: lerfu-word\n"));
            render_vlatai_classification_text_with_prefix(
                out,
                classification.base().expect("lerfu base"),
                phoneme_options,
                "base ",
            );
            render_plain_word_classification_text(
                out,
                classification.suffix().expect("lerfu suffix"),
                phoneme_options,
                "suffix ",
            );
        }
        ValsiClassificationKind::ZeiCompound => {
            out.push_str(&format!("{prefix}category: zei-compound\n"));
            render_vlatai_classification_text_with_prefix(
                out,
                classification.left().expect("zei left"),
                phoneme_options,
                "left ",
            );
            render_plain_word_classification_text(
                out,
                classification.link().expect("zei link"),
                phoneme_options,
                "link ",
            );
            render_plain_word_classification_text(
                out,
                classification.right().expect("zei right"),
                phoneme_options,
                "right ",
            );
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
    value
}

#[requires(true)]
#[ensures(!ret.is_empty() || phonemes.is_empty())]
fn render_vlatai_phonemes(phonemes: &str, options: PhonemeRenderOptions) -> String {
    Phonemes::from_canonical(phonemes.to_owned())
        .map(|value| value.render(options))
        .unwrap_or_else(|_| phonemes.to_owned())
}
