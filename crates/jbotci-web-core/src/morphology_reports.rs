//! Typed morphology reports shared by every delivery surface.
//!
//! `vlasei` (segmentation with recovery) and `vlatai` (single-word analysis)
//! have no CLI-independent result type elsewhere in the workspace. This module
//! owns the orchestration around the morphology crate: dialect resolution,
//! recovered segmentation, the diagnostics list in the shared phase order, and
//! for `vlatai` the status diagnostics and the phonotactically possible short
//! rafsi of a gismu. The CLI renders from these reports and Discord presents
//! them, so neither can drift from the other.

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_diagnostics::{
    Diagnostic, DiagnosticLabel, DiagnosticPhase, DiagnosticSeverity, DiagnosticSpanError,
    source_span_from_char_offsets,
};
use jbotci_morphology::{
    MorphologyOptions, PlainWordClassification, RecoveredMorphologySegmentation, ValsiAnalysis,
    ValsiAnalysisStatus, WordKind, analyze_valsi_with_options_and_source_id,
    fold_lojban_diacritics, possible_short_rafsi_forms,
    segment_words_with_modifiers_recovered_with_options_and_source_id_attempt,
};
use jbotci_source::SourceId;

use crate::{GentufaWebError, dialect_definition};

/// Recovery-capable segmentation of one text with its diagnostics.
///
/// `diagnostics` lists the morphology warnings followed by the errors, in
/// source order within each group, so a surface can show them without
/// reassembling them from the segmentation.
#[invariant(diagnostics.len() >= morphology.errors.len() + morphology.warnings.len())]
#[derive(Debug, Clone)]
pub struct VlaseiAnalysis {
    pub morphology_options: MorphologyOptions,
    pub morphology: RecoveredMorphologySegmentation,
    pub diagnostics: Vec<Diagnostic>,
}

impl VlaseiAnalysis {
    /// Whether segmentation covered the whole input without recovery.
    #[requires(true)]
    #[ensures(ret == self.morphology.errors.is_empty())]
    pub fn is_complete(&self) -> bool {
        self.morphology.errors.is_empty()
    }
}

/// Segment `source` under `dialect` (a formula, or `None` for standard
/// Lojban), keeping every recovered word and skipped region.
#[requires(true)]
#[ensures(ret.is_err() || ret.as_ref().is_ok_and(|analysis| analysis.diagnostics.iter().all(|diagnostic| diagnostic.phase == DiagnosticPhase::Morphology)))]
pub fn analyze_vlasei(
    source: &str,
    dialect: Option<&str>,
    source_id: Option<SourceId>,
) -> Result<VlaseiAnalysis, GentufaWebError> {
    let dialect = dialect_definition(dialect)?;
    let morphology_options = MorphologyOptions::default()
        .try_with_dialect_definition(&dialect)
        .map_err(|error| GentufaWebError::Dialect(error.to_string()))?;
    let morphology = segment_words_with_modifiers_recovered_with_options_and_source_id_attempt(
        source,
        &morphology_options,
        source_id.clone(),
    )
    .into_data()
    .result;
    let mut diagnostics = morphology
        .warnings
        .iter()
        .map(|warning| {
            warning
                .to_diagnostic(source_id.clone(), source)
                .expect("morphology warning offsets belong to the parser source")
        })
        .collect::<Vec<_>>();
    diagnostics.extend(morphology.errors.iter().map(|error| {
        error
            .to_diagnostic(source_id.clone(), source)
            .expect("morphology error offsets belong to the parser source")
    }));
    Ok(new!(VlaseiAnalysis {
        morphology_options,
        morphology,
        diagnostics,
    }))
}

/// One word's `vlatai` report.
#[invariant(analysis.result.is_valid() || diagnostics.iter().any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error), "an invalid or multi-word input always carries an error diagnostic")]
#[invariant(possible_rafsi.is_empty() || analysis.result.classification.as_ref().and_then(|classification| classification.word()).is_some_and(|word| word.category == WordKind::Gismu), "possible rafsi exist only for a gismu")]
#[derive(Debug, Clone)]
pub struct VlataiReport {
    pub analysis: ValsiAnalysis,
    /// Morphology warnings, then the error for invalid input or the
    /// not-single-word error for input that segments into several words.
    pub diagnostics: Vec<Diagnostic>,
    /// The short rafsi a valid gismu could claim by CLL 4.6 phonotactics,
    /// sorted. This is formation possibility, not dictionary assignment or
    /// availability; the dictionary answers those.
    pub possible_rafsi: Vec<String>,
}

/// Analyze one word under `options`; `source_id` labels its diagnostics.
#[requires(true)]
#[ensures(true)]
pub fn analyze_vlatai(
    word: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<VlataiReport, DiagnosticSpanError> {
    let analysis = analyze_valsi_with_options_and_source_id(word, options, source_id.clone());
    let diagnostics = vlatai_diagnostics(&analysis, source_id)?;
    let possible_rafsi = analysis
        .result
        .classification
        .as_ref()
        .and_then(|classification| classification.word())
        .filter(|word| word.category == WordKind::Gismu)
        .map(possible_rafsi_for_gismu)
        .unwrap_or_default();
    Ok(new!(VlataiReport {
        analysis,
        diagnostics,
        possible_rafsi,
    }))
}

/// The diagnostics a `vlatai` analysis reports: morphology warnings first,
/// then the input's error when it is invalid or did not segment as one word.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|diagnostics| diagnostics.len() >= analysis.warnings.len()) || ret.is_err())]
pub fn vlatai_diagnostics(
    analysis: &ValsiAnalysis,
    source_id: Option<SourceId>,
) -> Result<Vec<Diagnostic>, DiagnosticSpanError> {
    let mut diagnostics = analysis
        .warnings
        .iter()
        .map(|warning| warning.to_diagnostic(source_id.clone(), &analysis.input))
        .collect::<Result<Vec<_>, _>>()?;
    match analysis.result.status {
        ValsiAnalysisStatus::Invalid => {
            let error = analysis
                .result
                .error
                .as_ref()
                .expect("invalid vlatai result carries error");
            diagnostics.push(error.to_diagnostic(source_id, &analysis.input)?);
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

/// The error `vlatai` reports when its input is not exactly one word.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error) || ret.is_err())]
pub fn vlatai_not_single_word_diagnostic(
    source_id: Option<SourceId>,
    source: &str,
    word_count: usize,
) -> Result<Diagnostic, DiagnosticSpanError> {
    let char_end = source.chars().count();
    let span = source_span_from_char_offsets(source_id, source, 0, char_end)?;
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

/// The short rafsi a gismu classification could claim, sorted by spelling.
///
/// Availability is deliberately absent: this never consults the dictionary,
/// so it reports what CLL phonotactics permit and leaves who already holds a
/// rafsi to `vlacku`. Canonical phoneme text marks stress with acute accents,
/// so it is folded back to plain gismu letters before derivation.
#[requires(classification.category == WordKind::Gismu)]
#[ensures(ret.windows(2).all(|pair| pair[0] < pair[1]))]
pub fn possible_rafsi_for_gismu(classification: &PlainWordClassification) -> Vec<String> {
    possible_short_rafsi_forms(&fold_lojban_diacritics(&classification.phonemes))
        .into_iter()
        .map(|form| form.into_data().form)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlasei_reports_recovered_regions_and_diagnostics() {
        let complete = analyze_vlasei("mi klama", None, None).expect("analysis");
        assert!(complete.is_complete());
        assert_eq!(complete.morphology.words.len(), 2);
        assert!(complete.diagnostics.is_empty());

        // An apostrophe that cannot start a word forces recovery.
        let recovered = analyze_vlasei("mi 'klama do", None, None).expect("analysis");
        assert!(!recovered.is_complete());
        assert_eq!(
            recovered.morphology.error_regions.len(),
            recovered.morphology.errors.len()
        );
        assert!(
            recovered
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        );

        assert!(matches!(
            analyze_vlasei("mi klama", Some("not-a-dialect(("), None),
            Err(GentufaWebError::Dialect(_))
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn vlatai_distinguishes_valid_invalid_and_multiword_input() {
        let options = MorphologyOptions::default();
        let gismu = analyze_vlatai("klama", &options, None).expect("report");
        assert_eq!(gismu.analysis.result.status, ValsiAnalysisStatus::Valid);
        // CLL 4.6's CCVCV table (letters 1-5 = k l a m a): CVC 134 `kam` and
        // 234 `lam`; CVV 13'5 `ka'a` and 23'5 `la'a`, while the apostrophe-free
        // 135/235 forms are excluded because `aa` is not one of ai/ei/oi/au;
        // CCV 123 `kla` since `kl` is a permissible initial pair. Formation
        // possibility only: which of these klama actually holds is a
        // dictionary question.
        assert_eq!(gismu.possible_rafsi, ["ka'a", "kam", "kla", "la'a", "lam"]);
        assert!(gismu.diagnostics.is_empty());

        let cmavo = analyze_vlatai("coi", &options, None).expect("report");
        assert_eq!(cmavo.analysis.result.status, ValsiAnalysisStatus::Valid);
        assert!(cmavo.possible_rafsi.is_empty());

        let multi = analyze_vlatai("mi klama", &options, None).expect("report");
        assert_eq!(
            multi.analysis.result.status,
            ValsiAnalysisStatus::NotSingleWord
        );
        assert_eq!(multi.analysis.result.words.len(), 2);
        assert_eq!(multi.diagnostics.len(), 1);
        assert_eq!(multi.diagnostics[0].code, "vlatai.not-single-word");

        let invalid = analyze_vlatai("'klama", &options, None).expect("report");
        assert_eq!(invalid.analysis.result.status, ValsiAnalysisStatus::Invalid);
        assert!(
            invalid
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        );
    }
}
