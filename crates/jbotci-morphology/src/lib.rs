//! Lojban morphology model.

mod cmavo;
mod diacritics;
mod dialect;
mod grammar;
mod lujvo;
mod segment;
mod surface;
mod syntax_eq;
pub mod tree;

use std::{fmt, sync::Arc};

use bityzba::{data, invariant, new, requires, try_new};
use jbotci_diagnostics::{
    Diagnostic, DiagnosticLabel, DiagnosticNoteMode, DiagnosticPhase, DiagnosticSeverity,
    DiagnosticStyledNote, DiagnosticTextRole, DiagnosticTextSegment, TraceOptions, TracePhase,
    TraceReport, source_span_from_char_offsets,
};
use jbotci_dialect::{DialectDefinition, DialectFeature};
use jbotci_source::{SourceId, SourceLocationError, SourceSpan};
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use vec1::Vec1;

pub use cmavo::{Cmavo, Selmaho};
pub use diacritics::{
    fold_lojban_diacritic, fold_lojban_diacritics, folded_lojban_diacritics_eq,
    push_folded_lojban_diacritics_to, push_stripped_diacritics_to,
    push_stripped_lojban_diacritics_to, strip_diacritics, strip_diacritics_eq,
    strip_lojban_diacritic, strip_lojban_diacritics, stripped_lojban_diacritics_eq,
};
pub use dialect::{
    CompiledDialectDefinition, CompiledDialectEntry, CompiledDialectWord, DialectCompilationError,
};
pub use lujvo::{
    ConsonantPairClass, LujvoBuildMode, LujvoBuildPart, LujvoBuildPartData, LujvoCandidate,
    bond_rafsis, choose_best_lujvo_candidate, choose_best_lujvo_candidate_from_parts,
    consonant_pair_class, ends_with_consonant, ends_with_vowel, ensure_cmevla_word,
    is_bonding_hyphen, is_cmevla, is_consonant, is_valid_lujvo_candidate_word, is_vowel,
    permissible_consonant_pair, syllables_pattern,
};
pub use surface::{
    LeadingPauseContext, LeadingPauseVowelMode, word_needs_leading_pause,
    word_needs_leading_pause_in_context,
};
pub use syntax_eq::{word_like_syntax_eq, word_syntax_eq};
pub use tree::{
    AtomRef, LujvoPart, NodeRef, TreeNode, Verbatim, VerbatimData, Word, WordData, WordLike,
    WordLikeData,
};

pub const MORPHOLOGY_TRACE_FILTERS: &[&str] = &[
    "morphology",
    "segment",
    "digit sequence",
    "LOhU quote",
    "ZOI quote",
    "single-word quote",
    "ZO quote",
    "FAhO",
    "BU attachment",
    "SI erasure",
    "SA erasure",
    "SU erasure",
    "ZEI lujvo",
    "word",
    "CMAVO",
    "CMAVO prefix",
    "GISMU",
    "LUJVO",
    "FUHIVLA",
    "CMEVLA",
];

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MorphologyOptions {
    pub accept_latin: bool,
    pub accept_cyrillic: bool,
    pub accept_zbalermorna: bool,
    #[serde(default)]
    pub compiled_dialect: CompiledDialectDefinition,
    pub cmevla_as_relation_words: bool,
    pub uppercase_marks_stress: bool,
    #[serde(default)]
    pub trace: TraceOptions,
}

impl Default for MorphologyOptions {
    #[requires(true)]
    #[ensures(true)]
    fn default() -> Self {
        MorphologyOptions {
            accept_latin: true,
            accept_cyrillic: true,
            accept_zbalermorna: true,
            compiled_dialect: CompiledDialectDefinition::default(),
            cmevla_as_relation_words: false,
            uppercase_marks_stress: true,
            trace: TraceOptions::disabled(),
        }
    }
}

impl MorphologyOptions {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|options| options.compiled_dialect.entries.len() == definition.cmavo_entries.len()) || ret.is_err())]
    #[ensures(ret.as_ref().is_ok_and(|options| !definition.features.contains(&DialectFeature::Cbm) || options.cmevla_as_relation_words) || ret.is_err())]
    #[ensures(ret.as_ref().is_ok_and(|options| !definition.features.contains(&DialectFeature::CaseInsensitive) || !options.uppercase_marks_stress) || ret.is_err())]
    pub fn try_with_dialect_definition(
        self,
        definition: &DialectDefinition,
    ) -> Result<Self, DialectCompilationError> {
        let cmevla_as_relation_words = self.cmevla_as_relation_words;
        let uppercase_marks_stress = self.uppercase_marks_stress;
        Ok(MorphologyOptions {
            compiled_dialect: CompiledDialectDefinition::compile(definition)?,
            cmevla_as_relation_words: cmevla_as_relation_words
                || definition.features.contains(&DialectFeature::Cbm),
            uppercase_marks_stress: uppercase_marks_stress
                && !definition
                    .features
                    .contains(&DialectFeature::CaseInsensitive),
            ..self
        })
    }

    #[requires(true)]
    #[ensures(definition.features.contains(&DialectFeature::Cbm) -> ret.cmevla_as_relation_words)]
    #[ensures(definition.features.contains(&DialectFeature::CaseInsensitive) -> !ret.uppercase_marks_stress)]
    pub fn with_dialect_definition(self, definition: &DialectDefinition) -> Self {
        self.try_with_dialect_definition(definition)
            .expect("dialect definition must compile")
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn with_trace_options(self, trace: TraceOptions) -> Self {
        MorphologyOptions { trace, ..self }
    }
}

#[invariant(warnings.iter().all(|warning| warning.char_start < warning.char_end))]
#[derive(Debug, Clone)]
pub struct MorphologySegmentAttempt {
    pub result: Result<Vec<WordLike>, MorphologyError>,
    pub warnings: Vec<MorphologyWarning>,
    pub trace: Option<TraceReport>,
}

#[invariant(recovered_morphology_errors_match_regions(&errors, &error_regions))]
#[invariant(warnings.iter().all(|warning| warning.char_start < warning.char_end))]
#[bityzba::expensive_invariant(recovered_morphology_words_disjoint_from_error_regions(&words, &error_regions))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredMorphologySegmentation {
    pub words: Vec<WordLike>,
    pub errors: Vec<MorphologyError>,
    pub error_regions: Vec<SourceSpan>,
    pub warnings: Vec<MorphologyWarning>,
}

#[invariant(trace.as_ref().is_none_or(|trace| trace.phase == TracePhase::Morphology))]
#[derive(Debug, Clone)]
pub struct RecoveredMorphologySegmentAttempt {
    pub result: RecoveredMorphologySegmentation,
    pub trace: Option<TraceReport>,
}

#[requires(true)]
#[ensures(ret -> errors.len() == error_regions.len())]
fn recovered_morphology_errors_match_regions(
    errors: &[MorphologyError],
    error_regions: &[SourceSpan],
) -> bool {
    if errors.len() != error_regions.len() {
        return false;
    }
    if !error_regions
        .windows(2)
        .all(|regions| regions[0].char_end <= regions[1].char_start)
    {
        return false;
    }
    if !errors.windows(2).all(|errors| {
        let Some(left) = morphology_error_recovery_start(&errors[0]) else {
            return true;
        };
        let Some(right) = morphology_error_recovery_start(&errors[1]) else {
            return true;
        };
        left <= right
    }) {
        return false;
    }
    errors.iter().zip(error_regions).all(|(error, region)| {
        morphology_error_recovery_start(error)
            .is_none_or(|start| region.char_start <= start && start <= region.char_end)
    })
}

#[requires(true)]
#[ensures(true)]
fn recovered_morphology_words_disjoint_from_error_regions(
    words: &[WordLike],
    error_regions: &[SourceSpan],
) -> bool {
    words.iter().all(|word| {
        word.source_spans().into_iter().all(|span| {
            error_regions.iter().all(|region| {
                span.char_end <= region.char_start || region.char_end <= span.char_start
            })
        })
    })
}

#[requires(true)]
#[ensures(ret.is_none() == matches!(error, MorphologyError::SourceSpan(_)))]
fn morphology_error_recovery_start(error: &MorphologyError) -> Option<usize> {
    match error {
        MorphologyError::Invalid { char_start, .. } => Some(*char_start),
        MorphologyError::UnterminatedZoiQuote { char_offset, .. } => Some(*char_offset),
        MorphologyError::SourceSpan(_) => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum WordKind {
    #[serde(rename = "cmavo")]
    Cmavo,
    #[serde(rename = "gismu")]
    Gismu,
    #[serde(rename = "lujvo")]
    Lujvo,
    #[serde(rename = "fu'ivla")]
    Fuhivla,
    #[serde(rename = "cmevla")]
    Cmevla,
}

#[invariant(result.is_valid() -> warnings.iter().all(|warning| warning.char_start < warning.char_end))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValsiAnalysis {
    pub input: String,
    pub warnings: Vec<MorphologyWarning>,
    pub result: ValsiAnalysisResult,
}

#[invariant(matches!(status, ValsiAnalysisStatus::Valid) == word.is_some())]
#[invariant(matches!(status, ValsiAnalysisStatus::Valid) == classification.is_some())]
#[invariant(matches!(status, ValsiAnalysisStatus::Invalid) == error.is_some())]
#[invariant(matches!(status, ValsiAnalysisStatus::NotSingleWord) == (word.is_none() && classification.is_none() && error.is_none()))]
#[invariant(matches!(status, ValsiAnalysisStatus::NotSingleWord) || words.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValsiAnalysisResult {
    pub status: ValsiAnalysisStatus,
    pub word: Option<WordLike>,
    pub classification: Option<ValsiClassification>,
    pub error: Option<MorphologyError>,
    pub words: Vec<WordLike>,
}

impl ValsiAnalysisResult {
    #[requires(true)]
    #[ensures(ret == matches!(self.status, ValsiAnalysisStatus::Valid))]
    pub fn is_valid(&self) -> bool {
        matches!(self.status, ValsiAnalysisStatus::Valid)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValsiAnalysisStatus {
    Valid,
    Invalid,
    NotSingleWord,
}

#[invariant(::PlainWord { word } => !word.phonemes.is_empty())]
#[invariant(::QuotedWord { marker, quoted_word } =>
    marker.category == WordKind::Cmavo && !quoted_word.phonemes.is_empty())]
#[invariant(::DelimitedNonLojbanQuote { marker, delimiter } =>
    marker.category == WordKind::Cmavo && !delimiter.is_empty())]
#[invariant(::QuotedWords { marker, .. } => marker.category == WordKind::Cmavo)]
#[invariant(::DelimitedWordQuote { marker_text } => !marker_text.is_empty())]
#[invariant(::LerfuWord { suffix, .. } => suffix.category == WordKind::Cmavo)]
#[invariant(::ZeiCompound { link, right, .. } =>
    link.category == WordKind::Cmavo && !right.phonemes.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValsiClassification {
    PlainWord {
        word: PlainWordClassification,
    },
    QuotedWord {
        marker: PlainWordClassification,
        quoted_word: PlainWordClassification,
    },
    DelimitedNonLojbanQuote {
        marker: PlainWordClassification,
        delimiter: String,
    },
    QuotedWords {
        marker: PlainWordClassification,
        quoted_words: Vec<PlainWordClassification>,
    },
    DelimitedWordQuote {
        marker_text: String,
    },
    LerfuWord {
        base: Box<ValsiClassification>,
        suffix: PlainWordClassification,
    },
    ZeiCompound {
        left: Box<ValsiClassification>,
        link: PlainWordClassification,
        right: PlainWordClassification,
    },
}

impl ValsiClassification {
    #[requires(true)]
    #[ensures(true)]
    pub fn kind(&self) -> ValsiClassificationKind {
        match self.as_data() {
            data!(ValsiClassification::PlainWord { .. }) => ValsiClassificationKind::PlainWord,
            data!(ValsiClassification::QuotedWord { .. }) => ValsiClassificationKind::QuotedWord,
            data!(ValsiClassification::DelimitedNonLojbanQuote { .. }) => {
                ValsiClassificationKind::DelimitedNonLojbanQuote
            }
            data!(ValsiClassification::QuotedWords { .. }) => ValsiClassificationKind::QuotedWords,
            data!(ValsiClassification::DelimitedWordQuote { .. }) => {
                ValsiClassificationKind::DelimitedWordQuote
            }
            data!(ValsiClassification::LerfuWord { .. }) => ValsiClassificationKind::LerfuWord,
            data!(ValsiClassification::ZeiCompound { .. }) => ValsiClassificationKind::ZeiCompound,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(ValsiClassification::PlainWord { .. })))]
    pub fn word(&self) -> Option<&PlainWordClassification> {
        match self.as_data() {
            data!(ValsiClassification::PlainWord { word }) => Some(word),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(ValsiClassification::QuotedWord { .. }) | data!(ValsiClassification::DelimitedNonLojbanQuote { .. }) | data!(ValsiClassification::QuotedWords { .. })))]
    pub fn marker(&self) -> Option<&PlainWordClassification> {
        match self.as_data() {
            data!(ValsiClassification::QuotedWord { marker, .. })
            | data!(ValsiClassification::DelimitedNonLojbanQuote { marker, .. })
            | data!(ValsiClassification::QuotedWords { marker, .. }) => Some(marker),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(ValsiClassification::QuotedWord { .. })))]
    pub fn quoted_word(&self) -> Option<&PlainWordClassification> {
        match self.as_data() {
            data!(ValsiClassification::QuotedWord { quoted_word, .. }) => Some(quoted_word),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(matches!(self.as_data(), data!(ValsiClassification::QuotedWords { quoted_words, .. }) if ret.len() == quoted_words.len()) || (!matches!(self.as_data(), data!(ValsiClassification::QuotedWords { .. })) && ret.is_empty()))]
    pub fn quoted_words(&self) -> &[PlainWordClassification] {
        match self.as_data() {
            data!(ValsiClassification::QuotedWords { quoted_words, .. }) => quoted_words,
            _ => &[],
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(ValsiClassification::DelimitedWordQuote { .. })))]
    pub fn marker_text(&self) -> Option<&str> {
        match self.as_data() {
            data!(ValsiClassification::DelimitedWordQuote { marker_text }) => Some(marker_text),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(ValsiClassification::DelimitedNonLojbanQuote { .. })))]
    pub fn delimiter(&self) -> Option<&str> {
        match self.as_data() {
            data!(ValsiClassification::DelimitedNonLojbanQuote { delimiter, .. }) => {
                Some(delimiter)
            }
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(ValsiClassification::LerfuWord { .. })))]
    pub fn base(&self) -> Option<&ValsiClassification> {
        match self.as_data() {
            data!(ValsiClassification::LerfuWord { base, .. }) => Some(base),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(ValsiClassification::ZeiCompound { .. })))]
    pub fn left(&self) -> Option<&ValsiClassification> {
        match self.as_data() {
            data!(ValsiClassification::ZeiCompound { left, .. }) => Some(left),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(ValsiClassification::ZeiCompound { .. })))]
    pub fn link(&self) -> Option<&PlainWordClassification> {
        match self.as_data() {
            data!(ValsiClassification::ZeiCompound { link, .. }) => Some(link),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(ValsiClassification::ZeiCompound { .. })))]
    pub fn right(&self) -> Option<&PlainWordClassification> {
        match self.as_data() {
            data!(ValsiClassification::ZeiCompound { right, .. }) => Some(right),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(ValsiClassification::LerfuWord { .. })))]
    pub fn suffix(&self) -> Option<&PlainWordClassification> {
        match self.as_data() {
            data!(ValsiClassification::LerfuWord { suffix, .. }) => Some(suffix),
            _ => None,
        }
    }
}

impl Serialize for ValsiClassification {
    #[requires(true)]
    #[ensures(true)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ValsiClassification", 12)?;
        let kind = self.kind();
        let word = self.word();
        let marker = self.marker();
        let quoted_word = self.quoted_word();
        let quoted_words = self.quoted_words();
        let marker_text = self.marker_text();
        let delimiter = self.delimiter();
        let base = self.base();
        let left = self.left();
        let link = self.link();
        let right = self.right();
        let suffix = self.suffix();

        state.serialize_field("kind", &kind)?;
        state.serialize_field("word", &word)?;
        state.serialize_field("marker", &marker)?;
        state.serialize_field("quoted-word", &quoted_word)?;
        state.serialize_field("quoted-words", &quoted_words)?;
        state.serialize_field("marker-text", &marker_text)?;
        state.serialize_field("delimiter", &delimiter)?;
        state.serialize_field("base", &base)?;
        state.serialize_field("left", &left)?;
        state.serialize_field("link", &link)?;
        state.serialize_field("right", &right)?;
        state.serialize_field("suffix", &suffix)?;
        state.end()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValsiClassificationKind {
    PlainWord,
    QuotedWord,
    DelimitedNonLojbanQuote,
    QuotedWords,
    DelimitedWordQuote,
    LerfuWord,
    ZeiCompound,
}

#[invariant(!phonemes.is_empty())]
#[invariant(*category == WordKind::Cmavo || selmaho.is_none())]
#[invariant(*category == WordKind::Lujvo || (split.is_none() && parts.is_empty()))]
#[invariant(*category == WordKind::Fuhivla || stage.is_none())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PlainWordClassification {
    pub category: WordKind,
    pub phonemes: String,
    pub selmaho: Option<String>,
    pub split: Option<String>,
    pub parts: Vec<ValsiLujvoPart>,
    pub stage: Option<ValsiFuhivlaStage>,
}

#[invariant(!text.is_empty())]
#[invariant(matches!(kind, ValsiLujvoPartKind::Hyphen) -> rafsi_kind.is_none())]
#[invariant(matches!(kind, ValsiLujvoPartKind::Rafsi) -> rafsi_kind.is_some())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ValsiLujvoPart {
    pub kind: ValsiLujvoPartKind,
    pub text: String,
    pub rafsi_kind: Option<ValsiLujvoRafsiKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValsiLujvoPartKind {
    Rafsi,
    Hyphen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValsiLujvoRafsiKind {
    Cvc,
    Ccv,
    Cvv,
    Long,
    Gismu,
    Fuhivla,
    Cultural,
    Extended,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValsiFuhivlaStage {
    Stage3,
    Stage4,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StressMark {
    None,
    Acute,
    Caps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GlideMark {
    None,
    Breve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct PhonemeRenderOptions {
    pub mark_stress: StressMark,
    pub mark_glides: GlideMark,
}

impl Default for PhonemeRenderOptions {
    #[requires(true)]
    #[ensures(ret.mark_stress == StressMark::Acute)]
    #[ensures(ret.mark_glides == GlideMark::Breve)]
    fn default() -> Self {
        Self {
            mark_stress: StressMark::Acute,
            mark_glides: GlideMark::Breve,
        }
    }
}

#[invariant(!text.is_empty(), "phoneme text must not be empty")]
#[invariant(text.chars().all(is_valid_phoneme), "phonemes must use canonical Lojban phoneme characters")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Phonemes {
    text: String,
}

impl Phonemes {
    #[requires(!text.is_empty())]
    #[ensures(true)]
    pub fn from_canonical(text: String) -> Result<Self, String> {
        try_new!(Phonemes { text: text }).map_err(|error| error.to_string())
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|rendered| !rendered.is_empty()) || ret.is_err())]
    pub fn render_canonical(text: &str, options: PhonemeRenderOptions) -> Result<String, String> {
        if text.is_empty() {
            return Err("phoneme text must not be empty".to_owned());
        }
        if !text.chars().all(is_valid_phoneme) {
            return Err("phonemes must use canonical Lojban phoneme characters".to_owned());
        }
        Ok(text
            .chars()
            .map(|ch| render_phoneme_char(ch, options))
            .collect())
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn into_string(self) -> String {
        self.into_data().text
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn render(&self, options: PhonemeRenderOptions) -> String {
        self.text
            .chars()
            .map(|ch| render_phoneme_char(ch, options))
            .collect()
    }
}

#[invariant(!phonemes.as_str().is_empty(), "word key phonemes must not be empty")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WordKey {
    pub kind: WordKind,
    pub phonemes: Phonemes,
}

#[requires(true)]
#[ensures(true)]
fn render_phoneme_char(ch: char, options: PhonemeRenderOptions) -> char {
    match ch {
        'á' | 'é' | 'í' | 'ó' | 'ú' | 'ý' => render_stressed_vowel(ch, options.mark_stress),
        'ĭ' | 'ŭ' => render_glide(ch, options.mark_glides),
        other => other,
    }
}

#[requires(matches!(ch, 'á' | 'é' | 'í' | 'ó' | 'ú' | 'ý'))]
#[ensures(true)]
fn render_stressed_vowel(ch: char, mark: StressMark) -> char {
    match mark {
        StressMark::Acute => ch,
        StressMark::None => unstressed_vowel(ch),
        StressMark::Caps => unstressed_vowel(ch).to_ascii_uppercase(),
    }
}

#[requires(matches!(ch, 'ĭ' | 'ŭ'))]
#[ensures(true)]
fn render_glide(ch: char, mark: GlideMark) -> char {
    match (ch, mark) {
        ('ĭ', GlideMark::Breve) => 'ĭ',
        ('ŭ', GlideMark::Breve) => 'ŭ',
        ('ĭ', GlideMark::None) => 'i',
        ('ŭ', GlideMark::None) => 'u',
        _ => ch,
    }
}

#[requires(matches!(ch, 'á' | 'é' | 'í' | 'ó' | 'ú' | 'ý'))]
#[ensures(matches!(ret, 'a' | 'e' | 'i' | 'o' | 'u' | 'y'))]
fn unstressed_vowel(ch: char) -> char {
    match ch {
        'á' => 'a',
        'é' => 'e',
        'í' => 'i',
        'ó' => 'o',
        'ú' => 'u',
        'ý' => 'y',
        _ => ch,
    }
}

impl fmt::Display for WordKind {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Cmavo => "cmavo",
            Self::Gismu => "gismu",
            Self::Lujvo => "lujvo",
            Self::Fuhivla => "fu'ivla",
            Self::Cmevla => "cmevla",
        };
        f.write_str(text)
    }
}

impl fmt::Display for Word {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind(), self.phonemes().as_str())
    }
}

impl Word {
    #[requires(!phonemes.as_str().is_empty())]
    #[ensures(ret.kind() == kind)]
    pub fn from_kind(kind: WordKind, phonemes: Phonemes, span: SourceSpan) -> Self {
        match kind {
            WordKind::Cmavo => new!(Word::Cmavo {
                phonemes: phonemes,
                span: Arc::new(span),
            }),
            WordKind::Gismu => new!(Word::Gismu {
                phonemes: phonemes,
                span: Arc::new(span),
            }),
            WordKind::Lujvo => new!(Word::Lujvo {
                parts: Vec1::new(LujvoPart::rafsi(phonemes)),
                span: Arc::new(span),
            }),
            WordKind::Fuhivla => new!(Word::Fuhivla {
                phonemes: phonemes,
                span: Arc::new(span),
            }),
            WordKind::Cmevla => new!(Word::Cmevla {
                phonemes: phonemes,
                span: Arc::new(span),
            }),
        }
    }

    #[requires(!parts.is_empty())]
    #[ensures(ret.kind() == WordKind::Lujvo)]
    pub fn lujvo(parts: Vec1<LujvoPart>, span: SourceSpan) -> Self {
        new!(Word::Lujvo {
            parts: parts,
            span: Arc::new(span),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn kind(&self) -> WordKind {
        match self.as_data() {
            data!(Word::Cmavo { .. }) => WordKind::Cmavo,
            data!(Word::Gismu { .. }) => WordKind::Gismu,
            data!(Word::Lujvo { .. }) => WordKind::Lujvo,
            data!(Word::Fuhivla { .. }) => WordKind::Fuhivla,
            data!(Word::Cmevla { .. }) => WordKind::Cmevla,
        }
    }

    #[requires(true)]
    #[ensures(!ret.as_str().is_empty())]
    pub fn phonemes(&self) -> Phonemes {
        match self.as_data() {
            data!(Word::Cmavo { phonemes, .. })
            | data!(Word::Gismu { phonemes, .. })
            | data!(Word::Fuhivla { phonemes, .. })
            | data!(Word::Cmevla { phonemes, .. }) => phonemes.clone(),
            data!(Word::Lujvo { parts, .. }) => Phonemes::from_canonical(
                parts
                    .iter()
                    .map(LujvoPart::phonemes)
                    .map(Phonemes::as_str)
                    .collect::<String>(),
            )
            .expect("lujvo parts are valid phoneme text"),
        }
    }

    #[requires(true)]
    #[ensures(ret.kind == self.kind())]
    pub fn key(&self) -> WordKey {
        new!(WordKey {
            kind: self.kind(),
            phonemes: self.phonemes(),
        })
    }

    #[requires(true)]
    #[ensures(ret == (self.key() == other.key()))]
    pub fn is_same_word(&self, other: &Word) -> bool {
        self.key() == other.key()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn phonemes_ref(&self) -> Option<&Phonemes> {
        match self.as_data() {
            data!(Word::Cmavo { phonemes, .. })
            | data!(Word::Gismu { phonemes, .. })
            | data!(Word::Fuhivla { phonemes, .. })
            | data!(Word::Cmevla { phonemes, .. }) => Some(phonemes),
            data!(Word::Lujvo { .. }) => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn lujvo_parts(&self) -> Option<&Vec1<LujvoPart>> {
        match self.as_data() {
            data!(Word::Lujvo { parts, .. }) => Some(parts),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.char_start <= ret.char_end)]
    pub fn span(&self) -> &SourceSpan {
        match self.as_data() {
            data!(Word::Cmavo { span, .. })
            | data!(Word::Gismu { span, .. })
            | data!(Word::Lujvo { span, .. })
            | data!(Word::Fuhivla { span, .. })
            | data!(Word::Cmevla { span, .. }) => span.as_ref(),
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn canonical_phonemes(&self) -> String {
        canonicalize_text(self.phonemes().as_str())
    }

    #[requires(true)]
    #[ensures(ret == (self.kind() == WordKind::Cmavo))]
    pub fn is_cmavo_word(&self) -> bool {
        self.kind() == WordKind::Cmavo
    }

    #[requires(true)]
    #[ensures(ret == matches!(self.kind(), WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla))]
    pub fn is_brivla(&self) -> bool {
        matches!(
            self.kind(),
            WordKind::Gismu | WordKind::Lujvo | WordKind::Fuhivla
        )
    }

    #[requires(true)]
    #[ensures(ret == (self.kind() == WordKind::Cmevla))]
    pub fn is_cmevla(&self) -> bool {
        self.kind() == WordKind::Cmevla
    }

    #[requires(true)]
    #[ensures(ret.is_some() -> self.kind() == WordKind::Cmavo)]
    pub fn cmavo(&self) -> Option<Cmavo> {
        if self.is_cmavo_word() {
            // Lujvo can never be cmavo; once the kind check passes, the
            // phoneme storage is borrowed directly instead of rebuilding text.
            Cmavo::from_text(
                self.phonemes_ref()
                    .expect("cmavo words have direct phoneme storage")
                    .as_str(),
            )
        } else {
            None
        }
    }

    #[requires(true)]
    #[ensures(ret == (self.cmavo() == Some(cmavo)))]
    pub fn is_cmavo(&self, cmavo: Cmavo) -> bool {
        self.cmavo() == Some(cmavo)
    }

    #[requires(!cmavo.is_empty())]
    #[ensures(ret == self.cmavo().is_some_and(|actual| cmavo.contains(&actual)))]
    pub fn is_one_of_cmavo(&self, cmavo: &[Cmavo]) -> bool {
        self.cmavo().is_some_and(|actual| cmavo.contains(&actual))
    }

    #[requires(true)]
    #[ensures(ret == self.cmavo().is_some_and(|cmavo| selmaho.contains(cmavo)))]
    pub fn is_selmaho(&self, selmaho: Selmaho) -> bool {
        self.cmavo().is_some_and(|cmavo| selmaho.contains(cmavo))
    }

    #[requires(!selmaho.is_empty())]
    #[ensures(ret == self.cmavo().is_some_and(|cmavo| selmaho.iter().any(|selmaho| selmaho.contains(cmavo))))]
    pub fn is_one_of_selmaho(&self, selmaho: &[Selmaho]) -> bool {
        self.cmavo()
            .is_some_and(|cmavo| selmaho.iter().any(|selmaho| selmaho.contains(cmavo)))
    }

    #[requires(!text.is_empty())]
    #[ensures(true)]
    pub fn is_cmavo_text(&self, text: &str) -> bool {
        self.is_cmavo_word()
            && self
                .phonemes_ref()
                .is_some_and(|phonemes| canonical_text_eq(phonemes.as_str(), text))
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn selmaho_kind(&self) -> Option<Selmaho> {
        self.cmavo().and_then(Cmavo::primary_selmaho)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn selmaho(&self) -> Option<&'static str> {
        self.selmaho_kind().map(Selmaho::name)
    }
}

impl LujvoPart {
    #[requires(!phonemes.as_str().is_empty())]
    #[ensures(true)]
    pub fn rafsi(phonemes: Phonemes) -> Self {
        LujvoPart::Rafsi(phonemes)
    }

    #[requires(!phonemes.as_str().is_empty())]
    #[ensures(true)]
    pub fn hyphen(phonemes: Phonemes) -> Self {
        LujvoPart::Hyphen(phonemes)
    }

    #[requires(true)]
    #[ensures(!ret.as_str().is_empty())]
    pub fn phonemes(&self) -> &Phonemes {
        match self {
            LujvoPart::Rafsi(phonemes) | LujvoPart::Hyphen(phonemes) => phonemes,
        }
    }
}

impl Verbatim {
    #[requires(span.char_len() == text.chars().count())]
    #[ensures(true)]
    pub fn new(span: SourceSpan, text: String) -> Self {
        new!(Verbatim {
            span: Arc::new(span),
            text: text,
        })
    }
}

impl WordLike {
    #[requires(true)]
    #[ensures(true)]
    pub fn bare(word: Word) -> Self {
        new!(WordLike::PlainWord(word))
    }

    #[requires(zo.is_cmavo(Cmavo::Zo))]
    #[ensures(true)]
    pub fn zo_quote(zo: Word, word: Word) -> Self {
        new!(WordLike::QuotedWord {
            zo: Box::new(zo),
            word: Box::new(word),
        })
    }

    #[requires(zoi.is_selmaho(Selmaho::Zoi))]
    #[requires(canonical_text_eq(
        opening_delimiter.phonemes().as_str(),
        closing_delimiter.phonemes().as_str(),
    ))]
    #[requires(opening_delimiter.span().byte_end <= quoted_text.span.byte_start)]
    #[requires(quoted_text.span.byte_end <= closing_delimiter.span().byte_start)]
    #[ensures(true)]
    pub fn zoi_quote(
        zoi: Word,
        opening_delimiter: Word,
        quoted_text: Verbatim,
        closing_delimiter: Word,
    ) -> Self {
        new!(WordLike::DelimitedNonLojbanQuote {
            zoi: Box::new(zoi),
            opening_delimiter: Box::new(opening_delimiter),
            quoted_text: Box::new(quoted_text),
            closing_delimiter: Box::new(closing_delimiter),
        })
    }

    #[requires(lohu.is_cmavo(Cmavo::Lohu))]
    #[requires(lehu.is_cmavo(Cmavo::Lehu))]
    #[ensures(true)]
    pub fn lohu_quote(lohu: Word, quoted_words: Vec<Word>, lehu: Word) -> Self {
        new!(WordLike::QuotedWords {
            lohu: Box::new(lohu),
            quoted_words: quoted_words,
            lehu: Box::new(lehu),
        })
    }

    #[requires(is_single_word_quote_marker(&marker))]
    #[ensures(true)]
    pub fn single_word_quote(marker: Word, quoted_text: Verbatim) -> Self {
        new!(WordLike::DelimitedWordQuote {
            marker: Box::new(marker),
            quoted_text: Box::new(quoted_text),
        })
    }

    #[requires(bu.is_cmavo(Cmavo::Bu))]
    #[ensures(true)]
    pub fn letter(base: WordLike, bu: Word) -> Self {
        new!(WordLike::LerfuWord {
            base: Box::new(base),
            bu: Box::new(bu),
        })
    }

    #[requires(zei.is_cmavo(Cmavo::Zei))]
    #[ensures(true)]
    pub fn zei_lujvo(left: WordLike, zei: Word, right: Word) -> Self {
        new!(WordLike::ZeiCompound {
            left: Box::new(left),
            zei: Box::new(zei),
            right: Box::new(right),
        })
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(WordLike::PlainWord(_))))]
    pub fn bare_word(&self) -> Option<&Word> {
        match self.as_data() {
            data!(WordLike::PlainWord(word)) => Some(word),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.as_data(), data!(WordLike::QuotedWord { .. }) | data!(WordLike::DelimitedNonLojbanQuote { .. }) | data!(WordLike::QuotedWords { .. }) | data!(WordLike::DelimitedWordQuote { .. })))]
    pub fn quote_marker_cmavo(&self) -> Option<Cmavo> {
        match self.as_data() {
            data!(WordLike::QuotedWord { zo, .. }) => zo.cmavo(),
            data!(WordLike::DelimitedNonLojbanQuote { zoi, .. }) => zoi.cmavo(),
            data!(WordLike::QuotedWords { lohu, .. }) => lohu.cmavo(),
            data!(WordLike::DelimitedWordQuote { marker, .. }) => marker.cmavo(),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret == (self.quote_marker_cmavo() == Some(cmavo)))]
    pub fn is_quote_marker_cmavo(&self, cmavo: Cmavo) -> bool {
        self.quote_marker_cmavo() == Some(cmavo)
    }

    #[requires(true)]
    #[ensures(ret.is_some() == self.bare_word().is_some_and(|word| word.cmavo().is_some()))]
    pub fn cmavo(&self) -> Option<Cmavo> {
        self.bare_word().and_then(Word::cmavo)
    }

    #[requires(true)]
    #[ensures(ret == (self.cmavo() == Some(cmavo)))]
    pub fn is_cmavo(&self, cmavo: Cmavo) -> bool {
        self.cmavo() == Some(cmavo)
    }

    #[requires(!cmavo.is_empty())]
    #[ensures(ret == self.cmavo().is_some_and(|actual| cmavo.contains(&actual)))]
    pub fn is_one_of_cmavo(&self, cmavo: &[Cmavo]) -> bool {
        self.cmavo().is_some_and(|actual| cmavo.contains(&actual))
    }

    #[requires(true)]
    #[ensures(ret == self.cmavo().is_some_and(|cmavo| selmaho.contains(cmavo)))]
    pub fn is_selmaho(&self, selmaho: Selmaho) -> bool {
        self.cmavo().is_some_and(|cmavo| selmaho.contains(cmavo))
    }

    #[requires(!selmaho.is_empty())]
    #[ensures(ret == self.cmavo().is_some_and(|cmavo| selmaho.iter().any(|selmaho| selmaho.contains(cmavo))))]
    pub fn is_one_of_selmaho(&self, selmaho: &[Selmaho]) -> bool {
        self.cmavo()
            .is_some_and(|cmavo| selmaho.iter().any(|selmaho| selmaho.contains(cmavo)))
    }

    #[requires(true)]
    #[ensures(ret == matches!(self.as_data(), data!(WordLike::PlainWord(word)) if word.is_brivla()))]
    pub fn is_brivla(&self) -> bool {
        matches!(self.as_data(), data!(WordLike::PlainWord(word)) if word.is_brivla())
    }

    #[requires(true)]
    #[ensures(ret == matches!(self.as_data(), data!(WordLike::PlainWord(word)) if word.is_cmevla()))]
    pub fn is_cmevla(&self) -> bool {
        matches!(self.as_data(), data!(WordLike::PlainWord(word)) if word.is_cmevla())
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|range| range.start <= range.end))]
    pub fn byte_range(&self) -> Option<std::ops::Range<usize>> {
        word_like_byte_range(self)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn source_spans(&self) -> Vec<&SourceSpan> {
        let mut spans = Vec::new();
        self.source_spans_into(&mut spans);
        spans
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn source_spans_into<'a>(&'a self, out: &mut Vec<&'a SourceSpan>) {
        match self.as_data() {
            data!(WordLike::PlainWord(word)) => out.push(word.span()),
            data!(WordLike::QuotedWord { zo, word }) => {
                out.push(zo.span());
                out.push(word.span());
            }
            data!(WordLike::DelimitedNonLojbanQuote {
                zoi,
                opening_delimiter,
                quoted_text,
                closing_delimiter,
            }) => {
                out.push(zoi.span());
                out.push(opening_delimiter.span());
                out.push(quoted_text.span.as_ref());
                out.push(closing_delimiter.span());
            }
            data!(WordLike::QuotedWords {
                lohu,
                quoted_words,
                lehu,
            }) => {
                out.push(lohu.span());
                for word in quoted_words {
                    out.push(word.span());
                }
                out.push(lehu.span());
            }
            data!(WordLike::DelimitedWordQuote {
                marker,
                quoted_text,
            }) => {
                out.push(marker.span());
                out.push(quoted_text.span.as_ref());
            }
            data!(WordLike::LerfuWord { base, bu }) => {
                base.source_spans_into(out);
                out.push(bu.span());
            }
            data!(WordLike::ZeiCompound { left, zei, right }) => {
                left.source_spans_into(out);
                out.push(zei.span());
                out.push(right.span());
            }
        }
    }
}

impl<'de> Deserialize<'de> for WordLike {
    #[requires(true)]
    #[ensures(true)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        word_like_from_json(serde_json::Value::deserialize(deserializer)?)
            .map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for WordLike {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_data() {
            data!(WordLike::PlainWord(word)) => write!(f, "{word}"),
            data!(WordLike::QuotedWord { zo, word }) => write!(f, "{zo}-<<{word}>>"),
            data!(WordLike::DelimitedNonLojbanQuote {
                zoi,
                opening_delimiter,
                quoted_text,
                closing_delimiter,
            }) => write!(
                f,
                "{zoi}-{opening_delimiter}-{:?}-{closing_delimiter}",
                quoted_text.text
            ),
            data!(WordLike::QuotedWords {
                lohu,
                quoted_words,
                lehu,
            }) => {
                write!(f, "{lohu}-<<")?;
                for (index, word) in quoted_words.iter().enumerate() {
                    if index > 0 {
                        f.write_str(" ")?;
                    }
                    write!(f, "{word}")?;
                }
                write!(f, ">>-{lehu}")
            }
            data!(WordLike::DelimitedWordQuote {
                marker,
                quoted_text,
            }) => write!(f, "{marker}-{text:?}", text = quoted_text.text),
            data!(WordLike::LerfuWord { base, bu }) => write!(f, "{base}-{bu}"),
            data!(WordLike::ZeiCompound { left, zei, right }) => {
                write!(f, "{left}-{zei}-{right}")
            }
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
pub fn map_word_like_spans<F>(word_like: WordLike, map_span: &F) -> Result<WordLike, String>
where
    F: Fn(SourceSpan) -> Result<SourceSpan, String>,
{
    Ok(match word_like.into_data() {
        data!(WordLike::PlainWord(word)) => WordLike::bare(map_word_spans(word, map_span)?),
        data!(WordLike::QuotedWord { zo, word }) => WordLike::zo_quote(
            map_word_spans(*zo, map_span)?,
            map_word_spans(*word, map_span)?,
        ),
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            quoted_text,
            closing_delimiter,
        }) => WordLike::zoi_quote(
            map_word_spans(*zoi, map_span)?,
            map_word_spans(*opening_delimiter, map_span)?,
            map_verbatim_span(*quoted_text, map_span)?,
            map_word_spans(*closing_delimiter, map_span)?,
        ),
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            lehu,
        }) => WordLike::lohu_quote(
            map_word_spans(*lohu, map_span)?,
            quoted_words
                .into_iter()
                .map(|word| map_word_spans(word, map_span))
                .collect::<Result<Vec<_>, _>>()?,
            map_word_spans(*lehu, map_span)?,
        ),
        data!(WordLike::DelimitedWordQuote {
            marker,
            quoted_text,
        }) => WordLike::single_word_quote(
            map_word_spans(*marker, map_span)?,
            map_verbatim_span(*quoted_text, map_span)?,
        ),
        data!(WordLike::LerfuWord { base, bu }) => WordLike::letter(
            map_word_like_spans(*base, map_span)?,
            map_word_spans(*bu, map_span)?,
        ),
        data!(WordLike::ZeiCompound { left, zei, right }) => WordLike::zei_lujvo(
            map_word_like_spans(*left, map_span)?,
            map_word_spans(*zei, map_span)?,
            map_word_spans(*right, map_span)?,
        ),
    })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
pub fn map_word_spans<F>(word: Word, map_span: &F) -> Result<Word, String>
where
    F: Fn(SourceSpan) -> Result<SourceSpan, String>,
{
    Ok(match word.into_data() {
        data!(Word::Cmavo { phonemes, span }) => new!(Word::Cmavo {
            phonemes: phonemes,
            span: Arc::new(map_span((*span).clone())?),
        }),
        data!(Word::Gismu { phonemes, span }) => new!(Word::Gismu {
            phonemes: phonemes,
            span: Arc::new(map_span((*span).clone())?),
        }),
        data!(Word::Lujvo { parts, span }) => new!(Word::Lujvo {
            parts: parts,
            span: Arc::new(map_span((*span).clone())?),
        }),
        data!(Word::Fuhivla { phonemes, span }) => new!(Word::Fuhivla {
            phonemes: phonemes,
            span: Arc::new(map_span((*span).clone())?),
        }),
        data!(Word::Cmevla { phonemes, span }) => new!(Word::Cmevla {
            phonemes: phonemes,
            span: Arc::new(map_span((*span).clone())?),
        }),
    })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
pub fn map_verbatim_span<F>(verbatim: Verbatim, map_span: &F) -> Result<Verbatim, String>
where
    F: Fn(SourceSpan) -> Result<SourceSpan, String>,
{
    let data = verbatim.into_data();
    Ok(Verbatim::new(map_span((*data.span).clone())?, data.text))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum MorphologyErrorKind {
    InvalidCharacter,
    ExpectedWord,
    UnrecognizedWord,
    InvalidApostrophe,
    GeminatedConsonant,
    VoicingMismatch,
    ForbiddenConsonantPair,
    ForbiddenConsonantTriple,
    VowelHiatus,
    YHiatus,
    BreveNotGlide,
    DigitApostrophe,
    DigitVowel,
    Slinkuhi,
    InvalidLujvo,
    InvalidQuoteMarker,
    InvalidZoiDelimiter,
}

impl MorphologyErrorKind {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidCharacter => "morphology.invalid-character",
            Self::ExpectedWord => "morphology.expected-word",
            Self::UnrecognizedWord => "morphology.unrecognized-word",
            Self::InvalidApostrophe => "morphology.invalid-apostrophe",
            Self::GeminatedConsonant => "morphology.geminated-consonant",
            Self::VoicingMismatch => "morphology.voicing-mismatch",
            Self::ForbiddenConsonantPair => "morphology.forbidden-consonant-pair",
            Self::ForbiddenConsonantTriple => "morphology.forbidden-consonant-triple",
            Self::VowelHiatus => "morphology.vowel-hiatus",
            Self::YHiatus => "morphology.y-hiatus",
            Self::BreveNotGlide => "morphology.breve-not-glide",
            Self::DigitApostrophe => "morphology.digit-apostrophe",
            Self::DigitVowel => "morphology.digit-vowel",
            Self::Slinkuhi => "morphology.slinkuhi",
            Self::InvalidLujvo => "morphology.invalid-lujvo",
            Self::InvalidQuoteMarker => "morphology.invalid-quote-marker",
            Self::InvalidZoiDelimiter => "morphology.invalid-zoi-delimiter",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn message(self) -> &'static str {
        match self {
            Self::InvalidCharacter => "invalid character in Lojban word",
            Self::ExpectedWord => "expected Lojban word",
            Self::UnrecognizedWord => "word is not a valid Lojban word",
            Self::InvalidApostrophe => "apostrophe is only allowed between vowels",
            Self::GeminatedConsonant => "geminated consonants are not allowed",
            Self::VoicingMismatch => "adjacent consonants must agree in voicing",
            Self::ForbiddenConsonantPair => "forbidden consonant pair",
            Self::ForbiddenConsonantTriple => "forbidden consonant triple",
            Self::VowelHiatus => "vowels in hiatus are not allowed",
            Self::YHiatus => "y cannot be followed by a non-y vowel nucleus",
            Self::BreveNotGlide => "breve-marked vowel is not in a glide position",
            Self::DigitApostrophe => "digit cannot be followed by apostrophe",
            Self::DigitVowel => "digit cannot be followed by a vowel",
            Self::Slinkuhi => "slinku'i form is not a valid word",
            Self::InvalidLujvo => "invalid lujvo decomposition",
            Self::InvalidQuoteMarker => "quote marker must be a single word",
            Self::InvalidZoiDelimiter => "ZOI delimiter must be a single non-y word",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum MorphologyWarningKind {
    ExperimentalCgv,
    ExperimentalMz,
    BreveNotGlide,
}

impl MorphologyWarningKind {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn code(self) -> &'static str {
        match self {
            Self::ExperimentalCgv => "morphology.warning.experimental-cgv",
            Self::ExperimentalMz => "morphology.warning.experimental-mz",
            Self::BreveNotGlide => "morphology.warning.breve-not-glide",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn message(self) -> &'static str {
        match self {
            Self::ExperimentalCgv => "experimental morphology: consonant-glide-vowel sequence",
            Self::ExperimentalMz => "experimental morphology: MZ consonant pair",
            Self::BreveNotGlide => "breve-marked vowel is not in a glide position",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn label(self) -> &'static str {
        match self {
            Self::ExperimentalCgv => {
                "consonant-glide-vowel sequence accepted as experimental morphology"
            }
            Self::ExperimentalMz => "MZ consonant pair accepted as experimental morphology",
            Self::BreveNotGlide => "breve-marked vowel parsed as a vowel",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn detail_reason(self) -> &'static str {
        match self {
            Self::ExperimentalCgv => {
                "accepted by the experimental consonant-glide-vowel relaxation"
            }
            Self::ExperimentalMz => "accepted by the experimental MZ consonant-pair relaxation",
            Self::BreveNotGlide => {
                "Latin breve marks are optional glide hints and do not determine morphology"
            }
        }
    }
}

#[invariant(self.char_start < self.char_end, "morphology warnings must cover a non-empty span")]
#[invariant(!self.text.is_empty(), "morphology warnings must preserve offending source text")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MorphologyWarning {
    pub kind: MorphologyWarningKind,
    pub char_start: usize,
    pub char_end: usize,
    pub text: String,
    pub context: Option<MorphologyContext>,
}

impl MorphologyWarning {
    #[requires(char_start < char_end)]
    #[requires(!text.is_empty())]
    #[ensures(ret.kind == kind)]
    #[ensures(ret.char_start == char_start)]
    #[ensures(ret.char_end == char_end)]
    pub fn new(
        kind: MorphologyWarningKind,
        char_start: usize,
        char_end: usize,
        text: String,
        context: Option<MorphologyContext>,
    ) -> Self {
        new!(MorphologyWarning {
            kind: kind,
            char_start: char_start,
            char_end: char_end,
            text: text,
            context: context,
        })
    }

    #[requires(true)]
    #[ensures(!ret.code.is_empty())]
    pub fn to_diagnostic(&self, source_id: Option<SourceId>, source: &str) -> Diagnostic {
        morphology_diagnostic(
            source_id,
            source,
            new!(MorphologyDiagnosticDetails {
                severity: DiagnosticSeverity::Warning,
                code: self.kind.code(),
                message: self.kind.message(),
            }),
            self.char_start,
            self.char_end,
            self.kind.label(),
            self.context.as_ref(),
        )
        .with_styled_notes(vec![morphology_detail_note(
            self.kind.message(),
            &self.text,
            self.kind.detail_reason(),
        )])
    }
}

impl fmt::Display for MorphologyErrorKind {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum MorphologyContextKind {
    Cmavo,
    Gismu,
    Lujvo,
    Fuhivla,
    Cmevla,
    QuotedWord,
    DelimitedNonLojbanQuote,
    QuotedWords,
    DelimitedWordQuote,
    Bu,
    Zei,
}

impl MorphologyContextKind {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn label(self) -> &'static str {
        match self {
            Self::Cmavo => "while parsing cmavo",
            Self::Gismu => "while parsing gismu",
            Self::Lujvo => "while parsing lujvo",
            Self::Fuhivla => "while parsing fu'ivla",
            Self::Cmevla => "while parsing cmevla",
            Self::QuotedWord => "while parsing ZO quote",
            Self::DelimitedNonLojbanQuote => "while parsing ZOI quote",
            Self::QuotedWords => "while parsing LOhU quote",
            Self::DelimitedWordQuote => "while parsing single-word quote",
            Self::Bu => "while applying BU",
            Self::Zei => "while applying ZEI",
        }
    }
}

#[invariant(self.char_start < self.char_end, "morphology context labels must cover a non-empty span")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MorphologyContext {
    pub kind: MorphologyContextKind,
    pub char_start: usize,
    pub char_end: usize,
}

impl MorphologyContext {
    #[requires(char_start < char_end)]
    #[ensures(ret.char_start == char_start)]
    #[ensures(ret.char_end == char_end)]
    pub fn new(kind: MorphologyContextKind, char_start: usize, char_end: usize) -> Self {
        new!(MorphologyContext {
            kind: kind,
            char_start: char_start,
            char_end: char_end,
        })
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn label(&self) -> &'static str {
        self.kind.label()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub enum LujvoParseExpectation {
    InitialOrStandaloneFinalRafsi,
    FinalOrInitialRafsi,
}

impl LujvoParseExpectation {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn description(self) -> &'static str {
        match self {
            Self::InitialOrStandaloneFinalRafsi => "an initial rafsi or a standalone final rafsi",
            Self::FinalOrInitialRafsi => "a final rafsi or another initial rafsi",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub enum ExpectedWordDetailKind {
    PlainWord,
    QuoteTarget,
    BuOperand,
    ZeiOperand,
    ZoiDelimiter,
}

impl ExpectedWordDetailKind {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn description(self) -> &'static str {
        match self {
            Self::PlainWord => "the parser reached a point where a Lojban word is required",
            Self::QuoteTarget => "ZO requires one following non-y word to quote",
            Self::BuOperand => "BU must attach to a preceding word",
            Self::ZeiOperand => "ZEI must have a word on both sides",
            Self::ZoiDelimiter => "ZOI requires an opening delimiter word after the quote marker",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub enum ZoiDelimiterDetailKind {
    Missing,
    YWord,
    NotSingleWord,
}

impl ZoiDelimiterDetailKind {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn description(self) -> &'static str {
        match self {
            Self::Missing => "ZOI requires an opening delimiter word after the quote marker",
            Self::YWord => "y is grammar noise, so it cannot delimit a ZOI quote",
            Self::NotSingleWord => "a ZOI delimiter must be exactly one bare word",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub enum PhonotacticDetailKind {
    InvalidCharacter,
    InvalidApostrophe,
    GeminatedConsonant,
    VoicingMismatch,
    ForbiddenConsonantPair,
    ForbiddenConsonantTriple,
    VowelHiatus,
    YHiatus,
    BreveNotGlide,
    DigitApostrophe,
    DigitVowel,
}

impl PhonotacticDetailKind {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn description(self) -> &'static str {
        match self {
            Self::InvalidCharacter => "this character is not part of Lojban morphology",
            Self::InvalidApostrophe => "apostrophe can only separate two vowel nuclei",
            Self::GeminatedConsonant => "the same consonant appears twice in a row",
            Self::VoicingMismatch => "this consonant pair mixes voiced and unvoiced consonants",
            Self::ForbiddenConsonantPair => "this consonant pair is not a permissible Lojban pair",
            Self::ForbiddenConsonantTriple => {
                "this consonant triple does not contain a permissible adjacent pair"
            }
            Self::VowelHiatus => "these adjacent vowel nuclei need a separating apostrophe",
            Self::YHiatus => "y cannot be immediately followed by another vowel nucleus",
            Self::BreveNotGlide => "a breve-marked vowel must be part of a glide",
            Self::DigitApostrophe => "digit lerfu cannot be followed directly by apostrophe",
            Self::DigitVowel => "digit lerfu cannot be followed directly by a vowel nucleus",
        }
    }
}

#[invariant(true)]
#[invariant(::InvalidLujvo => parsed_prefix.as_ref().is_none_or(|prefix| !prefix.is_empty()))]
#[invariant(::ExpectedWord => matches!(expected,
    ExpectedWordDetailKind::PlainWord
        | ExpectedWordDetailKind::QuoteTarget
        | ExpectedWordDetailKind::BuOperand
        | ExpectedWordDetailKind::ZeiOperand
        | ExpectedWordDetailKind::ZoiDelimiter))]
#[invariant(::InvalidZoiDelimiter => matches!(reason,
    ZoiDelimiterDetailKind::Missing
        | ZoiDelimiterDetailKind::YWord
        | ZoiDelimiterDetailKind::NotSingleWord))]
#[invariant(::Phonotactic => matches!(reason,
    PhonotacticDetailKind::InvalidCharacter
        | PhonotacticDetailKind::InvalidApostrophe
        | PhonotacticDetailKind::GeminatedConsonant
        | PhonotacticDetailKind::VoicingMismatch
        | PhonotacticDetailKind::ForbiddenConsonantPair
        | PhonotacticDetailKind::ForbiddenConsonantTriple
        | PhonotacticDetailKind::VowelHiatus
        | PhonotacticDetailKind::YHiatus
        | PhonotacticDetailKind::BreveNotGlide
        | PhonotacticDetailKind::DigitApostrophe
        | PhonotacticDetailKind::DigitVowel))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MorphologyErrorDetail {
    InvalidLujvo {
        parsed_prefix: Option<String>,
        expected: LujvoParseExpectation,
    },
    FuhivlaContainsY,
    Slinkuhi,
    ExpectedWord {
        expected: ExpectedWordDetailKind,
    },
    InvalidZoiDelimiter {
        reason: ZoiDelimiterDetailKind,
    },
    Phonotactic {
        reason: PhonotacticDetailKind,
    },
}

impl MorphologyErrorDetail {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn construct(&self) -> &'static str {
        match self.as_data() {
            data!(MorphologyErrorDetail::InvalidLujvo { .. }) => "invalid lujvo",
            data!(MorphologyErrorDetail::FuhivlaContainsY) => "fu'ivla",
            data!(MorphologyErrorDetail::Slinkuhi) => "slinku'i",
            data!(MorphologyErrorDetail::ExpectedWord { .. }) => "expected word",
            data!(MorphologyErrorDetail::InvalidZoiDelimiter { .. }) => "ZOI delimiter",
            data!(MorphologyErrorDetail::Phonotactic { .. }) => "phonotactics",
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn reason(&self) -> String {
        match self.as_data() {
            data!(MorphologyErrorDetail::InvalidLujvo {
                parsed_prefix,
                expected,
            }) => parsed_prefix.as_ref().map_or_else(
                || {
                    format!(
                        "the lujvo parser expected {} at the start",
                        expected.description()
                    )
                },
                |prefix| {
                    format!(
                        "after parsing `{prefix}`, the lujvo parser expected {} at the next source position",
                        expected.description()
                    )
                },
            ),
            data!(MorphologyErrorDetail::FuhivlaContainsY) => {
                "fu'ivla syllables cannot use y as a vowel nucleus".to_owned()
            }
            data!(MorphologyErrorDetail::Slinkuhi) => {
                "adding a leading consonant before a lujvo-shaped form would break word resolution"
                    .to_owned()
            }
            data!(MorphologyErrorDetail::ExpectedWord { expected }) => {
                expected.description().to_owned()
            }
            data!(MorphologyErrorDetail::InvalidZoiDelimiter { reason }) => {
                reason.description().to_owned()
            }
            data!(MorphologyErrorDetail::Phonotactic { reason }) => {
                reason.description().to_owned()
            }
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
#[invariant(::Invalid => true)]
#[invariant(::UnterminatedZoiQuote => true)]
#[invariant(::SourceSpan(_) => true)]
pub enum MorphologyError {
    #[error("{kind} at character {char_start}: `{text}`")]
    Invalid {
        kind: MorphologyErrorKind,
        char_start: usize,
        char_end: usize,
        text: String,
        context: Option<MorphologyContext>,
        detail: Option<MorphologyErrorDetail>,
    },
    #[error("unterminated ZOI quote, expected closing delimiter `{delimiter}`")]
    UnterminatedZoiQuote {
        char_offset: usize,
        delimiter: String,
        context: Option<MorphologyContext>,
    },
    #[error("invalid source span: {0}")]
    SourceSpan(#[from] SourceLocationError),
}

impl MorphologyError {
    #[requires(true)]
    #[ensures(!ret.code.is_empty())]
    pub fn to_diagnostic(&self, source_id: Option<SourceId>, source: &str) -> Diagnostic {
        match self {
            Self::Invalid {
                kind,
                char_start,
                char_end,
                text,
                context,
                detail,
            } => {
                let diagnostic = morphology_diagnostic(
                    source_id.clone(),
                    source,
                    new!(MorphologyDiagnosticDetails {
                        severity: DiagnosticSeverity::Error,
                        code: kind.code(),
                        message: kind.message(),
                    }),
                    *char_start,
                    *char_end,
                    kind.message(),
                    context.as_ref(),
                );
                diagnostic_with_optional_detail(diagnostic, text, detail.as_ref())
            }
            Self::UnterminatedZoiQuote {
                char_offset,
                delimiter,
                context,
            } => {
                let source_end = source.chars().count();
                morphology_diagnostic(
                    source_id.clone(),
                    source,
                    new!(MorphologyDiagnosticDetails {
                        severity: DiagnosticSeverity::Error,
                        code: "morphology.unterminated-zoi-quote",
                        message: "unterminated ZOI quote",
                    }),
                    *char_offset,
                    source_end,
                    &format!("expected closing delimiter `{delimiter}`"),
                    context.as_ref(),
                )
                .with_styled_notes(vec![morphology_detail_note(
                    "unterminated ZOI quote",
                    delimiter,
                    "expected closing delimiter",
                )])
            }
            Self::SourceSpan(error) => {
                let span = source_span_from_char_offsets(source_id, source, 0, 0)
                    .expect("the start of a source string is always a valid source span");
                Diagnostic::new(
                    DiagnosticSeverity::Error,
                    DiagnosticPhase::Morphology,
                    "morphology.source-span".to_owned(),
                    "invalid source span".to_owned(),
                    vec![DiagnosticLabel::new(span, error.to_string(), true)],
                    Vec::new(),
                    None,
                )
            }
        }
    }
}

#[requires(true)]
#[ensures(!ret.code.is_empty())]
fn diagnostic_with_optional_detail(
    diagnostic: Diagnostic,
    text: &str,
    detail: Option<&MorphologyErrorDetail>,
) -> Diagnostic {
    let Some(detail) = detail else {
        return diagnostic;
    };
    let reason = detail.reason();
    diagnostic.with_styled_notes(vec![morphology_detail_note(
        detail.construct(),
        text,
        &reason,
    )])
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn phonotactic_error_detail(kind: MorphologyErrorKind) -> Option<MorphologyErrorDetail> {
    let reason = match kind {
        MorphologyErrorKind::InvalidCharacter => PhonotacticDetailKind::InvalidCharacter,
        MorphologyErrorKind::InvalidApostrophe => PhonotacticDetailKind::InvalidApostrophe,
        MorphologyErrorKind::GeminatedConsonant => PhonotacticDetailKind::GeminatedConsonant,
        MorphologyErrorKind::VoicingMismatch => PhonotacticDetailKind::VoicingMismatch,
        MorphologyErrorKind::ForbiddenConsonantPair => {
            PhonotacticDetailKind::ForbiddenConsonantPair
        }
        MorphologyErrorKind::ForbiddenConsonantTriple => {
            PhonotacticDetailKind::ForbiddenConsonantTriple
        }
        MorphologyErrorKind::VowelHiatus => PhonotacticDetailKind::VowelHiatus,
        MorphologyErrorKind::YHiatus => PhonotacticDetailKind::YHiatus,
        MorphologyErrorKind::BreveNotGlide => PhonotacticDetailKind::BreveNotGlide,
        MorphologyErrorKind::DigitApostrophe => PhonotacticDetailKind::DigitApostrophe,
        MorphologyErrorKind::DigitVowel => PhonotacticDetailKind::DigitVowel,
        _ => return None,
    };
    Some(new!(MorphologyErrorDetail::Phonotactic { reason }))
}

#[requires(!message.is_empty())]
#[requires(!reason.is_empty())]
#[ensures(!ret.segments.is_empty())]
fn morphology_detail_note(message: &str, text: &str, reason: &str) -> DiagnosticStyledNote {
    let display_text = if text.is_empty() { "input" } else { text };
    DiagnosticStyledNote::new(
        DiagnosticNoteMode::Detailed,
        vec![
            DiagnosticTextSegment::new(DiagnosticTextRole::Keyword, "morphology detail".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ": ".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Construct, message.to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, " (".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::SpecificWord, display_text.to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ")\n".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Keyword, "reason".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Punctuation, ": ".to_owned()),
            DiagnosticTextSegment::new(DiagnosticTextRole::Plain, reason.to_owned()),
        ],
    )
}

#[invariant(!self.code.is_empty())]
#[invariant(!self.message.is_empty())]
struct MorphologyDiagnosticDetails {
    severity: DiagnosticSeverity,
    code: &'static str,
    message: &'static str,
}

#[requires(!label.is_empty())]
#[requires(char_start <= char_end)]
#[ensures(!ret.code.is_empty())]
fn morphology_diagnostic(
    source_id: Option<SourceId>,
    source: &str,
    details: MorphologyDiagnosticDetails,
    char_start: usize,
    char_end: usize,
    label: &str,
    context: Option<&MorphologyContext>,
) -> Diagnostic {
    let span = source_span_from_char_offsets(source_id.clone(), source, char_start, char_end)
        .expect("morphology errors store offsets derived from the same source text");
    let mut labels = vec![DiagnosticLabel::new(span, label.to_owned(), true)];
    if let Some(context_label) = context.and_then(|context| {
        source_span_from_char_offsets(
            source_id.clone(),
            source,
            context.char_start,
            context.char_end,
        )
        .ok()
        .map(|span| DiagnosticLabel::new(span, context.label().to_owned(), false))
    }) {
        labels.push(context_label);
    }
    Diagnostic::new(
        details.severity,
        DiagnosticPhase::Morphology,
        details.code.to_owned(),
        details.message.to_owned(),
        labels,
        Vec::new(),
        None,
    )
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers(input: &str) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_with_options_and_source_id(
        input,
        &MorphologyOptions::default(),
        None,
    )
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_recovered(input: &str) -> RecoveredMorphologySegmentation {
    segment_words_with_modifiers_recovered_with_options_and_source_id(
        input,
        &MorphologyOptions::default(),
        None,
    )
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_recovered_with_options(
    input: &str,
    options: &MorphologyOptions,
) -> RecoveredMorphologySegmentation {
    segment_words_with_modifiers_recovered_with_options_and_source_id(input, options, None)
}

#[requires(true)]
#[ensures(ret.input == input)]
#[ensures(matches!(ret.result.status, ValsiAnalysisStatus::Valid | ValsiAnalysisStatus::Invalid | ValsiAnalysisStatus::NotSingleWord))]
pub fn analyze_valsi_with_options_and_source_id(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> ValsiAnalysis {
    let attempt =
        segment_words_with_modifiers_with_options_and_source_id_attempt(input, options, source_id);
    let attempt = attempt.into_data();
    let result = match attempt.result {
        Ok(words) if words.len() == 1 => {
            let word = words
                .into_iter()
                .next()
                .expect("length checked above guarantees a word");
            let classification = valsi_classification(&word);
            new!(ValsiAnalysisResult {
                status: ValsiAnalysisStatus::Valid,
                word: Some(word),
                classification: Some(classification),
                error: None,
                words: Vec::new(),
            })
        }
        Ok(words) => new!(ValsiAnalysisResult {
            status: ValsiAnalysisStatus::NotSingleWord,
            word: None,
            classification: None,
            error: None,
            words: words,
        }),
        Err(error) => new!(ValsiAnalysisResult {
            status: ValsiAnalysisStatus::Invalid,
            word: None,
            classification: None,
            error: Some(error),
            words: Vec::new(),
        }),
    };
    new!(ValsiAnalysis {
        input: input.to_owned(),
        warnings: attempt.warnings,
        result: result,
    })
}

#[requires(true)]
#[ensures(true)]
pub fn analyze_valsi_with_options(input: &str, options: &MorphologyOptions) -> ValsiAnalysis {
    analyze_valsi_with_options_and_source_id(input, options, None)
}

#[requires(true)]
#[ensures(true)]
pub fn analyze_valsi(input: &str) -> ValsiAnalysis {
    analyze_valsi_with_options(input, &MorphologyOptions::default())
}

#[requires(true)]
#[ensures(true)]
fn valsi_classification(word_like: &WordLike) -> ValsiClassification {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => new!(ValsiClassification::PlainWord {
            word: plain_word_classification(word),
        }),
        data!(WordLike::QuotedWord { zo, word }) => new!(ValsiClassification::QuotedWord {
            marker: plain_word_classification(zo),
            quoted_word: plain_word_classification(word),
        }),
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            opening_delimiter,
            ..
        }) => new!(ValsiClassification::DelimitedNonLojbanQuote {
            marker: plain_word_classification(zoi),
            delimiter: opening_delimiter.phonemes().into_string(),
        }),
        data!(WordLike::QuotedWords {
            lohu,
            quoted_words,
            ..
        }) => new!(ValsiClassification::QuotedWords {
            marker: plain_word_classification(lohu),
            quoted_words: quoted_words.iter().map(plain_word_classification).collect(),
        }),
        data!(WordLike::DelimitedWordQuote { marker, .. }) => {
            new!(ValsiClassification::DelimitedWordQuote {
                marker_text: marker.phonemes().into_string(),
            })
        }
        data!(WordLike::LerfuWord { base, bu }) => new!(ValsiClassification::LerfuWord {
            base: Box::new(valsi_classification(base)),
            suffix: plain_word_classification(bu),
        }),
        data!(WordLike::ZeiCompound { left, zei, right }) => {
            new!(ValsiClassification::ZeiCompound {
                left: Box::new(valsi_classification(left)),
                link: plain_word_classification(zei),
                right: plain_word_classification(right),
            })
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn plain_word_classification(word: &Word) -> PlainWordClassification {
    let phonemes = word.phonemes().into_string();
    match word.kind() {
        WordKind::Cmavo => new!(PlainWordClassification {
            category: WordKind::Cmavo,
            phonemes: phonemes,
            selmaho: word.selmaho().map(str::to_owned),
            split: None,
            parts: Vec::new(),
            stage: None,
        }),
        WordKind::Gismu => new!(PlainWordClassification {
            category: WordKind::Gismu,
            phonemes: phonemes,
            selmaho: None,
            split: None,
            parts: Vec::new(),
            stage: None,
        }),
        WordKind::Lujvo => {
            let parts = word
                .lujvo_parts()
                .expect("lujvo words carry parsed lujvo parts")
                .iter()
                .map(valsi_lujvo_part)
                .collect::<Vec<_>>();
            let split = parts
                .iter()
                .map(|part| part.text.as_str())
                .collect::<Vec<_>>()
                .join("-");
            new!(PlainWordClassification {
                category: WordKind::Lujvo,
                phonemes: phonemes,
                selmaho: None,
                split: Some(split),
                parts: parts,
                stage: None,
            })
        }
        WordKind::Fuhivla => new!(PlainWordClassification {
            category: WordKind::Fuhivla,
            stage: Some(segment::classify_fuhivla_stage(&phonemes)),
            phonemes: phonemes,
            selmaho: None,
            split: None,
            parts: Vec::new(),
        }),
        WordKind::Cmevla => new!(PlainWordClassification {
            category: WordKind::Cmevla,
            phonemes: phonemes,
            selmaho: None,
            split: None,
            parts: Vec::new(),
            stage: None,
        }),
    }
}

#[requires(true)]
#[ensures(!ret.text.is_empty())]
fn valsi_lujvo_part(part: &LujvoPart) -> ValsiLujvoPart {
    match part {
        LujvoPart::Rafsi(phonemes) => {
            let text = phonemes.as_str().to_owned();
            new!(ValsiLujvoPart {
                kind: ValsiLujvoPartKind::Rafsi,
                rafsi_kind: Some(segment::classify_lujvo_rafsi(&text)),
                text: text,
            })
        }
        LujvoPart::Hyphen(phonemes) => new!(ValsiLujvoPart {
            kind: ValsiLujvoPartKind::Hyphen,
            rafsi_kind: None,
            text: phonemes.as_str().to_owned(),
        }),
    }
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_with_options_and_source_id(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_with_modifiers_with_options_and_source_id_attempt(input, options, source_id)
        .into_data()
        .result
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_with_options_and_source_id_attempt(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> MorphologySegmentAttempt {
    let attempt = grammar::segment_words_with_modifiers_attempt(input, options, source_id);
    let data = attempt.into_data();
    let result = data
        .result
        .map(|words| apply_compiled_dialect_entries(words, &options.compiled_dialect));
    new!(MorphologySegmentAttempt {
        result,
        warnings: data.warnings,
        trace: data.trace,
    })
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_recovered_with_options_and_source_id(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> RecoveredMorphologySegmentation {
    segment_words_with_modifiers_recovered_with_options_and_source_id_attempt(
        input, options, source_id,
    )
    .into_data()
    .result
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_with_modifiers_recovered_with_options_and_source_id_attempt(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> RecoveredMorphologySegmentAttempt {
    let attempt =
        grammar::segment_words_with_modifiers_recovered_attempt(input, options, source_id);
    let attempt = attempt.into_data();
    let result = attempt.result.into_data();
    let words = apply_compiled_dialect_entries(result.words, &options.compiled_dialect);
    let result =
        RecoveredMorphologySegmentation::from_data(data!(RecoveredMorphologySegmentation {
            words,
            errors: result.errors,
            error_regions: result.error_regions,
            warnings: result.warnings,
        }));
    new!(RecoveredMorphologySegmentAttempt {
        result,
        trace: attempt.trace,
    })
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_for_display(input: &str) -> Result<Vec<WordLike>, MorphologyError> {
    segment_words_for_display_with_options_and_source_id(input, &MorphologyOptions::default(), None)
}

#[requires(true)]
#[ensures(true)]
pub fn segment_words_for_display_with_options_and_source_id(
    input: &str,
    options: &MorphologyOptions,
    source_id: Option<SourceId>,
) -> Result<Vec<WordLike>, MorphologyError> {
    grammar::segment_words_for_display(input, options, source_id)
        .map(|words| apply_compiled_dialect_entries(words, &options.compiled_dialect))
}

#[requires(!phonemes.as_str().is_empty())]
#[ensures(ret.as_ref().is_ok_and(|syllables| !syllables.is_empty() && syllables.iter().all(|syllable| !syllable.is_empty())) || ret.as_ref().err().is_some_and(|message| !message.is_empty()))]
pub fn pronunciation_syllables(phonemes: &Phonemes) -> Result<Vec<String>, String> {
    segment::pronunciation_syllable_texts(phonemes.as_str())
        .ok_or_else(|| format!("could not syllabify `{}`", phonemes.as_str()))
}

#[requires(true)]
#[ensures(true)]
fn apply_compiled_dialect_entries(
    mut words: Vec<WordLike>,
    dialect: &CompiledDialectDefinition,
) -> Vec<WordLike> {
    for entry in &dialect.entries {
        words = apply_compiled_dialect_entry(words, entry);
    }
    words
}

#[requires(true)]
#[ensures(true)]
fn apply_compiled_dialect_entry(
    words: Vec<WordLike>,
    entry: &CompiledDialectEntry,
) -> Vec<WordLike> {
    words
        .into_iter()
        .flat_map(|word_like| apply_compiled_dialect_entry_to_word_like(word_like, entry))
        .collect()
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn apply_compiled_dialect_entry_to_word_like(
    word_like: WordLike,
    entry: &CompiledDialectEntry,
) -> Vec<WordLike> {
    let data!(WordLike::PlainWord(word)) = word_like.as_data() else {
        return vec![word_like];
    };
    let key = word.key();
    let Some(replacement) = entry.replacement_for(&key) else {
        return vec![word_like];
    };
    let span = word.span().clone();
    replacement
        .into_iter()
        .map(|word| word.to_word_like_with_span(&span))
        .collect()
}

#[requires(true)]
#[ensures(true)]
fn is_single_word_quote_marker(word: &Word) -> bool {
    word.is_one_of_cmavo(&[
        Cmavo::Zohoi,
        Cmavo::Lahoi,
        Cmavo::Rahoi,
        Cmavo::Mehoi,
        Cmavo::Gohoi,
        Cmavo::Zehoi,
        Cmavo::Tahai,
        Cmavo::Bohei,
    ])
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn word_like_from_json(value: serde_json::Value) -> Result<WordLike, String> {
    let mut object = json_object(value)?;
    if let Some(kind) = object.remove("kind") {
        let kind = json_string(kind)?;
        return match kind.as_str() {
            "bare" => Ok(WordLike::bare(word_field(&mut object, "word")?)),
            "zo-quote" => Ok(WordLike::zo_quote(
                word_field(&mut object, "zo")?,
                word_field(&mut object, "word")?,
            )),
            "zoi-quote" => Ok(WordLike::zoi_quote(
                word_field(&mut object, "zoi")?,
                word_field(&mut object, "opening_delimiter")?,
                verbatim_field(&mut object, "quoted_text")?,
                word_field(&mut object, "closing_delimiter")?,
            )),
            "lohu-quote" => Ok(WordLike::lohu_quote(
                word_field(&mut object, "lohu")?,
                words_field(&mut object, "quoted_words")?,
                word_field(&mut object, "lehu")?,
            )),
            "single-word-quote" => Ok(WordLike::single_word_quote(
                word_field(&mut object, "marker")?,
                verbatim_field(&mut object, "quoted_text")?,
            )),
            "letter" => Ok(WordLike::letter(
                word_like_field(&mut object, "base")?,
                word_field(&mut object, "bu")?,
            )),
            "zei-lujvo" => Ok(WordLike::zei_lujvo(
                word_like_field(&mut object, "left")?,
                word_field(&mut object, "zei")?,
                word_field(&mut object, "right")?,
            )),
            other => Err(format!("unknown word-like kind `{other}`")),
        };
    }
    let (constructor, payload) = single_constructor(object)?;
    let mut payload = json_object(payload)?;
    match constructor.as_str() {
        "Bare" | "PlainWord" => Ok(WordLike::bare(word_payload(payload)?)),
        "QuotedWord" => Ok(WordLike::zo_quote(
            word_field(&mut payload, "zo")?,
            word_field(&mut payload, "word")?,
        )),
        "DelimitedNonLojbanQuote" => Ok(WordLike::zoi_quote(
            word_field(&mut payload, "zoi")?,
            word_field(&mut payload, "opening_delimiter")?,
            verbatim_field(&mut payload, "quoted_text")?,
            word_field(&mut payload, "closing_delimiter")?,
        )),
        "QuotedWords" => Ok(WordLike::lohu_quote(
            word_field(&mut payload, "lohu")?,
            words_field(&mut payload, "quoted_words")?,
            word_field(&mut payload, "lehu")?,
        )),
        "DelimitedWordQuote" => Ok(WordLike::single_word_quote(
            word_field(&mut payload, "marker")?,
            verbatim_field(&mut payload, "quoted_text")?,
        )),
        "Letter" => Ok(WordLike::letter(
            word_like_field(&mut payload, "base")?,
            word_field(&mut payload, "bu")?,
        )),
        "ZeiCompound" => Ok(WordLike::zei_lujvo(
            word_like_field(&mut payload, "left")?,
            word_field(&mut payload, "zei")?,
            word_field(&mut payload, "right")?,
        )),
        other => Err(format!("unknown word-like constructor `{other}`")),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn word_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<Word, String> {
    serde_json::from_value(required_field(object, name)?)
        .map_err(|error| format!("invalid word field `{name}`: {error}"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn word_payload(mut object: serde_json::Map<String, serde_json::Value>) -> Result<Word, String> {
    if object.contains_key("word") {
        return word_field(&mut object, "word");
    }
    serde_json::from_value(serde_json::Value::Object(object))
        .map_err(|error| format!("invalid word payload: {error}"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn words_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<Vec<Word>, String> {
    let Some(value) = object.remove(name) else {
        return Ok(Vec::new());
    };
    serde_json::from_value(value)
        .map_err(|error| format!("invalid word list field `{name}`: {error}"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn source_span_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<SourceSpan, String> {
    serde_json::from_value(required_field(object, name)?)
        .map_err(|error| format!("invalid source span field `{name}`: {error}"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn verbatim_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<Verbatim, String> {
    serde_json::from_value(required_field(object, name)?)
        .map_err(|error| format!("invalid verbatim field `{name}`: {error}"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn word_like_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<WordLike, String> {
    word_like_from_json(required_field(object, name)?)
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn required_field(
    object: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<serde_json::Value, String> {
    object
        .remove(name)
        .ok_or_else(|| format!("missing field `{name}`"))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn json_object(
    value: serde_json::Value,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    match value {
        serde_json::Value::Object(object) => Ok(object),
        other => Err(format!("expected object, got {other}")),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn json_string(value: serde_json::Value) -> Result<String, String> {
    match value {
        serde_json::Value::String(text) => Ok(text),
        other => Err(format!("expected string, got {other}")),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|message| !message.is_empty()))]
fn single_constructor(
    object: serde_json::Map<String, serde_json::Value>,
) -> Result<(String, serde_json::Value), String> {
    if object.len() != 1 {
        return Err(format!(
            "expected single constructor key, got {}",
            object.len()
        ));
    }
    Ok(object.into_iter().next().expect("object has one item"))
}

#[requires(true)]
#[ensures(true)]
pub fn is_valid_phoneme(value: char) -> bool {
    matches!(
        value,
        'a' | 'á'
            | 'e'
            | 'é'
            | 'i'
            | 'í'
            | 'ĭ'
            | 'o'
            | 'ó'
            | 'u'
            | 'ú'
            | 'ŭ'
            | 'y'
            | 'ý'
            | '\''
            | ','
            | '0'..='9'
    ) || matches!(
        value,
        'b' | 'c'
            | 'd'
            | 'f'
            | 'g'
            | 'j'
            | 'k'
            | 'l'
            | 'm'
            | 'n'
            | 'p'
            | 'r'
            | 's'
            | 't'
            | 'v'
            | 'x'
            | 'z'
    )
}

#[requires(true)]
#[ensures(!ret.is_empty() || text.is_empty())]
pub fn canonicalize_text(text: &str) -> String {
    text.chars()
        .filter(|value| *value != ',')
        .filter_map(fold_lojban_diacritic)
        .flat_map(char::to_lowercase)
        .collect()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|parts| !parts.is_empty()))]
pub fn parse_lujvo_word_parts(word: &str) -> Option<Vec<LujvoPart>> {
    let normalized = canonicalize_text(word);
    let shape = normalized.replace(',', "");
    let (kind, phonemes) = segment::classify_word(&normalized)?;
    if kind != WordKind::Lujvo {
        return None;
    }
    segment::parse_lujvo_parts_with_canonical_phonemes(&shape, &phonemes).map(Vec1::into_vec)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|parts| !parts.is_empty()))]
pub fn parse_cmevla_lujvo_word_parts(word: &str) -> Option<Vec<LujvoPart>> {
    let normalized = canonicalize_text(word);
    let shape = normalized.replace(',', "");
    segment::parse_cmevla_lujvo_parts_with_canonical_phonemes(&shape, &normalized)
        .map(Vec1::into_vec)
}

#[requires(true)]
#[ensures(ret.iter().all(|parts| !parts.is_empty()))]
pub fn parse_cmevla_lujvo_word_part_candidates(word: &str) -> Vec<Vec<LujvoPart>> {
    let normalized = canonicalize_text(word);
    let shape = normalized.replace(',', "");
    segment::parse_cmevla_lujvo_part_candidates_with_canonical_phonemes(&shape, &normalized)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty() || input.is_empty()))]
pub fn normalize_lojban_input_text(input: &str) -> Option<String> {
    normalize_lojban_input_text_with_options(input, &MorphologyOptions::default())
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty() || input.is_empty()))]
pub fn normalize_lojban_input_text_with_options(
    input: &str,
    options: &MorphologyOptions,
) -> Option<String> {
    let mut output = String::new();
    let mut chunk = String::new();
    for value in input.chars() {
        if segment::is_separator(value) {
            append_normalized_lojban_input_chunk(&mut output, &chunk, options)?;
            chunk.clear();
            output.push(normalized_lojban_input_separator(value));
        } else {
            chunk.push(value);
        }
    }
    append_normalized_lojban_input_chunk(&mut output, &chunk, options)?;
    Some(output)
}

#[requires(true)]
#[ensures(true)]
fn append_normalized_lojban_input_chunk(
    output: &mut String,
    chunk: &str,
    options: &MorphologyOptions,
) -> Option<()> {
    if chunk.is_empty() {
        return Some(());
    }
    let normalized = segment::normalize_word_checked_with_options(chunk, options)?;
    if normalized.is_empty() {
        return None;
    }
    output.push_str(&canonicalize_text(&normalized));
    Some(())
}

#[requires(true)]
#[ensures(true)]
fn normalized_lojban_input_separator(value: char) -> char {
    if is_period_character(value) {
        '.'
    } else {
        value
    }
}

#[requires(true)]
#[ensures(ret == matches!(value, '.' | 'ӏ' | 'Ӏ' | '\u{ed89}'))]
pub fn is_period_character(value: char) -> bool {
    value == '.' || segment::is_cyrillic_period(value) || segment::is_zbalermorna_period(value)
}

#[requires(true)]
#[ensures(true)]
pub fn canonical_text_eq(left: &str, right: &str) -> bool {
    left.chars()
        .filter(|value| *value != ',')
        .filter_map(fold_lojban_diacritic)
        .flat_map(char::to_lowercase)
        .eq(right
            .chars()
            .filter(|value| *value != ',')
            .filter_map(fold_lojban_diacritic)
            .flat_map(char::to_lowercase))
}

#[requires(true)]
#[ensures(ret -> !text.is_empty())]
pub fn canonical_text_is_all(text: &str, expected: char) -> bool {
    let mut saw_char = false;
    for value in text
        .chars()
        .filter(|value| *value != ',')
        .filter_map(fold_lojban_diacritic)
        .flat_map(char::to_lowercase)
    {
        if value != expected {
            return false;
        }
        saw_char = true;
    }
    saw_char
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
pub fn normalize_cmavo_form(text: &str) -> Option<String> {
    normalize_normalized_cmavo_form(text)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
fn normalize_normalized_cmavo_form(text: &str) -> Option<String> {
    let normalized = segment::parse_cmavo_form(text)?;
    Some(
        normalized
            .chars()
            .map(|value| if value == 'ý' { 'y' } else { value })
            .collect(),
    )
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|phonemes| !phonemes.as_str().is_empty()))]
pub fn cmavo_phonemes(text: &str) -> Option<Phonemes> {
    let normalized = normalize_cmavo_form(text)?;
    Cmavo::from_text(&normalized)?;
    Phonemes::from_canonical(normalized).ok()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn erasure_selmaho(word_like: &WordLike) -> Option<&'static str> {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => word.selmaho(),
        data!(WordLike::QuotedWord { .. }) => Some("ZO"),
        data!(WordLike::DelimitedNonLojbanQuote { zoi, .. }) => zoi.selmaho(),
        data!(WordLike::QuotedWords { .. }) => Some("LOhU"),
        data!(WordLike::DelimitedWordQuote { marker, .. }) => marker.selmaho(),
        data!(WordLike::LerfuWord { .. }) => Some("BU"),
        data!(WordLike::ZeiCompound { .. }) => Some("ZEI"),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|range| range.start <= range.end))]
fn word_like_byte_range(word_like: &WordLike) -> Option<std::ops::Range<usize>> {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => Some(word.span().byte_start..word.span().byte_end),
        data!(WordLike::QuotedWord { zo, word }) => {
            Some(zo.span().byte_start..word.span().byte_end)
        }
        data!(WordLike::DelimitedNonLojbanQuote {
            zoi,
            closing_delimiter,
            ..
        }) => Some(zoi.span().byte_start..closing_delimiter.span().byte_end),
        data!(WordLike::QuotedWords { lohu, lehu, .. }) => {
            Some(lohu.span().byte_start..lehu.span().byte_end)
        }
        data!(WordLike::DelimitedWordQuote {
            marker,
            quoted_text
        }) => Some(marker.span().byte_start..quoted_text.span.byte_end),
        data!(WordLike::LerfuWord { base, bu }) => {
            word_like_byte_range(base).map(|range| range.start..bu.span().byte_end.max(range.end))
        }
        data!(WordLike::ZeiCompound { left, right, .. }) => word_like_byte_range(left)
            .map(|range| range.start..right.span().byte_end.max(range.end)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bityzba::requires;
    use jbotci_dialect::{
        CmavoDialectEntry, CmavoDialectEntryData, DialectDefinition, DialectFeature,
    };

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cgv_relaxation_is_enabled_by_default_with_warning() {
        let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
            "la siatl.",
            &MorphologyOptions::default(),
            None,
        );
        let data = attempt.into_data();
        let words = data.result.expect("CgV relaxation should permit cmevla");

        assert_eq!(base_phonemes(&words[1]).as_deref(), Some("sĭatl"));
        assert_eq!(data.warnings.len(), 1);
        assert_eq!(
            data.warnings[0].kind,
            MorphologyWarningKind::ExperimentalCgv
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn mz_relaxation_is_enabled_by_default_with_warning() {
        let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
            "la djeimz.",
            &MorphologyOptions::default(),
            None,
        );
        let data = attempt.into_data();
        let words = data.result.expect("MZ relaxation should permit cmevla");

        assert_eq!(base_phonemes(&words[1]).as_deref(), Some("djeĭmz"));
        assert_eq!(data.warnings.len(), 1);
        assert_eq!(data.warnings[0].kind, MorphologyWarningKind::ExperimentalMz);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn latin_breve_cmevla_surfaces_parse_as_single_words() {
        let cases = [
            ("la .djeĭmz.", "djeĭmz"),
            ("la .saĭmn.", "saĭmn"),
            ("la .eĭvn.", "eĭvn"),
            ("la .paŭlas.", "paŭlas"),
            ("la .nu,ĭórk.", "nu,ĭórk"),
        ];

        for (source, expected) in cases {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            assert_eq!(
                base_phonemes(&words[1]).as_deref(),
                Some(expected),
                "{source}"
            );
            assert_eq!(
                base_word(&words[1]).map(Word::kind),
                Some(WordKind::Cmevla),
                "{source}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn latin_breve_is_warning_not_error_outside_glide_position() {
        let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
            "mĭ pŭ",
            &MorphologyOptions::default(),
            None,
        );
        let data = attempt.into_data();
        let words = data.result.expect("Latin breve marks should be optional");

        assert_eq!(base_phoneme_texts(&words), vec!["mi", "pu"]);
        assert_eq!(data.warnings.len(), 2);
        assert_eq!(data.warnings[0].kind, MorphologyWarningKind::BreveNotGlide);
        assert_eq!(data.warnings[0].char_start, 1);
        assert_eq!(data.warnings[0].char_end, 2);
        assert_eq!(data.warnings[0].text, "ĭ");
        assert_eq!(data.warnings[1].kind, MorphologyWarningKind::BreveNotGlide);
        assert_eq!(data.warnings[1].text, "ŭ");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pronunciation_syllables_allow_y_nuclei_and_native_clusters() {
        let cases = [
            ("jetcybolxáda", vec!["je", "tcy", "bol", "xá", "da"]),
            ("bolxáda", vec!["bol", "xá", "da"]),
            ("dikyjvo", vec!["di", "ky", "jvo"]),
            ("díkyjvo", vec!["dí", "ky", "jvo"]),
            ("cidjrspageti", vec!["cid", "jr", "spa", "ge", "ti"]),
            ("krĭófla", vec!["krĭó", "fla"]),
            ("trĭárko", vec!["trĭár", "ko"]),
        ];

        for (source, expected) in cases {
            assert_eq!(
                pronunciation_syllables_for_test(source),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn valsi_analysis_classifies_lujvo_with_parts() {
        let analysis = analyze_valsi("jetcybolxada");

        assert!(matches!(analysis.result.status, ValsiAnalysisStatus::Valid));
        let classification = analysis
            .result
            .classification
            .as_ref()
            .expect("valid analysis has classification");
        let word = classification.word().expect("plain word classification");
        assert_eq!(word.category, WordKind::Lujvo);
        assert_eq!(word.phonemes, "jetcybolxáda");
        assert_eq!(word.split.as_deref(), Some("jetc-y-bolxáda"));
        assert_eq!(
            word.parts
                .iter()
                .map(|part| part.text.as_str())
                .collect::<Vec<_>>(),
            vec!["jetc", "y", "bolxáda"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn valsi_analysis_reports_invalid_and_not_single_word() {
        let invalid = analyze_valsi("aa");
        assert!(matches!(
            invalid.result.status,
            ValsiAnalysisStatus::Invalid
        ));
        assert!(invalid.result.error.is_some());

        let multiple = analyze_valsi("coibroda");
        assert!(matches!(
            multiple.result.status,
            ValsiAnalysisStatus::NotSingleWord
        ));
        assert_eq!(multiple.result.words.len(), 2);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn valsi_analysis_labels_fuhivla_stage() {
        let stage3 = analyze_valsi("cidjrspageti");
        let stage3_word = stage3
            .result
            .classification
            .as_ref()
            .and_then(ValsiClassification::word)
            .expect("valid plain word classification");
        assert_eq!(stage3_word.category, WordKind::Fuhivla);
        assert_eq!(stage3_word.stage, Some(ValsiFuhivlaStage::Stage3));

        let stage4 = analyze_valsi("spageti");
        let stage4_word = stage4
            .result
            .classification
            .as_ref()
            .and_then(ValsiClassification::word)
            .expect("valid plain word classification");
        assert_eq!(stage4_word.category, WordKind::Fuhivla);
        assert_eq!(stage4_word.stage, Some(ValsiFuhivlaStage::Stage4));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pronunciation_syllables_cover_long_dictionary_clusters() {
        let cases = [
            ("cipnrstrígi", vec!["cip", "nr", "strí", "gi"]),
            ("cabrspréso", vec!["ca", "br", "spré", "so"]),
            ("bolstropfédo", vec!["bol", "strop", "fé", "do"]),
            ("ciskrpeŭédji", vec!["cis", "kr", "pe", "ŭé", "dji"]),
            ("bangrsfe'énska", vec!["ban", "gr", "sfe", "'én", "ska"]),
        ];

        for (source, expected) in cases {
            assert_eq!(
                pronunciation_syllables_for_test(source),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_rejects_invalid_three_consonant_onsets() {
        segment_words_with_modifiers("actla").expect_err("ctl is not a valid syllable onset");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_does_not_insert_implicit_y() {
        segment_words_with_modifiers("refgau").expect_err("fg must not be repaired with y");
        segment_words_with_modifiers("refygau").expect("explicit y hyphen remains valid");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_accepts_fuhivla_rafsi_inside_lujvo() {
        let words = segment_words_with_modifiers("tci'ilykemcantutra")
            .expect("camxes-std accepts fuhivla rafsi before ordinary lujvo rafsi");

        assert_eq!(base_phoneme_texts(&words), vec!["tci'ilykemcantútra"]);
        assert_eq!(base_word(&words[0]).map(Word::kind), Some(WordKind::Lujvo));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_accepts_pathological_long_lujvo() {
        let cases = [
            "zgikemfi'inalka'esefsysajyke'ejvekemsefsyda'atoiflike'ejvejagborkemjilryjvesefsyborxenze'a",
            "jbojevysofkemsuzgugje'ake'eborkemfaipaltrusi'oke'ekemgubyseltru",
            "tci'ilykemcantutra",
        ];

        for source in cases {
            let words = segment_words_with_modifiers(source)
                .unwrap_or_else(|error| panic!("{source} should parse as lujvo: {error:?}"));
            let word = base_word(&words[0]).expect("base word");

            assert_eq!(words.len(), 1, "{source}");
            assert_eq!(word.kind(), WordKind::Lujvo, "{source}");
            assert!(
                word.lujvo_parts().is_some_and(|parts| parts.len() > 1),
                "{source}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn invalid_slinkuhi_examples_remain_invalid() {
        for source in ["xlaglymlu", "jbaugri"] {
            let Err(error) = segment_words_with_modifiers(source) else {
                panic!("{source} must remain invalid");
            };

            assert!(
                matches!(
                    error,
                    MorphologyError::Invalid {
                        kind: MorphologyErrorKind::Slinkuhi,
                        ..
                    }
                ),
                "{source}: {error:?}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn pronunciation_syllables_match_updated_jvot3_clusters() {
        let cases = [
            (
                "arnonkrtcerimola",
                vec!["ar", "non", "kr", "tce", "ri", "mo", "la"],
            ),
            ("bangrtcosena", vec!["ban", "gr", "tco", "se", "na"]),
            ("dansrdja'aza", vec!["dan", "sr", "dja", "'a", "za"]),
            ("nanbrtcuro", vec!["nan", "br", "tcu", "ro"]),
            ("mutcmle", vec!["mut", "cmle"]),
        ];

        for (source, expected) in cases {
            segment_words_with_modifiers(source).unwrap_or_else(|error| {
                panic!("{source} should parse after updated onset rules: {error:?}")
            });
            assert_eq!(
                pronunciation_syllables_for_test(source),
                expected,
                "{source}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn latin_breve_in_glide_position_does_not_warn() {
        let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
            "faŭ la .saĭmn.",
            &MorphologyOptions::default(),
            None,
        );
        let data = attempt.into_data();
        let words = data.result.expect("valid morphology");

        assert_eq!(base_phonemes(&words[0]).as_deref(), Some("faŭ"));
        assert_eq!(base_phonemes(&words[2]).as_deref(), Some("saĭmn"));
        assert!(
            data.warnings
                .iter()
                .all(|warning| warning.kind != MorphologyWarningKind::BreveNotGlide)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn latin_breve_final_diphthong_stays_with_stressed_brivla() {
        let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
            "ko múvgaŭ ti",
            &MorphologyOptions::default(),
            None,
        );
        let data = attempt.into_data();
        let words = data.result.expect("valid morphology");

        assert_eq!(base_phoneme_texts(&words), vec!["ko", "múvgaŭ", "ti"]);
        assert_eq!(base_word(&words[1]).map(Word::kind), Some(WordKind::Lujvo));
        assert!(
            data.warnings
                .iter()
                .all(|warning| warning.kind != MorphologyWarningKind::BreveNotGlide)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn latin_breve_final_diphthong_counts_as_one_stress_nucleus() {
        for source in ["citkakei", "citkákeĭ"] {
            let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
                source,
                &MorphologyOptions::default(),
                None,
            );
            let data = attempt.into_data();
            let words = data.result.expect("valid morphology");

            assert_eq!(base_phoneme_texts(&words), vec!["citkákeĭ"], "{source}");
            assert_eq!(
                base_word(&words[0]).map(Word::kind),
                Some(WordKind::Fuhivla),
                "{source}"
            );
            assert!(
                data.warnings
                    .iter()
                    .all(|warning| warning.kind != MorphologyWarningKind::BreveNotGlide),
                "{source}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_error_kind_codes_are_stable() {
        let cases = [
            (
                MorphologyErrorKind::InvalidCharacter,
                "morphology.invalid-character",
                "invalid character in Lojban word",
            ),
            (
                MorphologyErrorKind::ExpectedWord,
                "morphology.expected-word",
                "expected Lojban word",
            ),
            (
                MorphologyErrorKind::UnrecognizedWord,
                "morphology.unrecognized-word",
                "word is not a valid Lojban word",
            ),
            (
                MorphologyErrorKind::InvalidApostrophe,
                "morphology.invalid-apostrophe",
                "apostrophe is only allowed between vowels",
            ),
            (
                MorphologyErrorKind::GeminatedConsonant,
                "morphology.geminated-consonant",
                "geminated consonants are not allowed",
            ),
            (
                MorphologyErrorKind::VoicingMismatch,
                "morphology.voicing-mismatch",
                "adjacent consonants must agree in voicing",
            ),
            (
                MorphologyErrorKind::ForbiddenConsonantPair,
                "morphology.forbidden-consonant-pair",
                "forbidden consonant pair",
            ),
            (
                MorphologyErrorKind::ForbiddenConsonantTriple,
                "morphology.forbidden-consonant-triple",
                "forbidden consonant triple",
            ),
            (
                MorphologyErrorKind::VowelHiatus,
                "morphology.vowel-hiatus",
                "vowels in hiatus are not allowed",
            ),
            (
                MorphologyErrorKind::YHiatus,
                "morphology.y-hiatus",
                "y cannot be followed by a non-y vowel nucleus",
            ),
            (
                MorphologyErrorKind::BreveNotGlide,
                "morphology.breve-not-glide",
                "breve-marked vowel is not in a glide position",
            ),
            (
                MorphologyErrorKind::DigitApostrophe,
                "morphology.digit-apostrophe",
                "digit cannot be followed by apostrophe",
            ),
            (
                MorphologyErrorKind::DigitVowel,
                "morphology.digit-vowel",
                "digit cannot be followed by a vowel",
            ),
            (
                MorphologyErrorKind::Slinkuhi,
                "morphology.slinkuhi",
                "slinku'i form is not a valid word",
            ),
            (
                MorphologyErrorKind::InvalidLujvo,
                "morphology.invalid-lujvo",
                "invalid lujvo decomposition",
            ),
            (
                MorphologyErrorKind::InvalidQuoteMarker,
                "morphology.invalid-quote-marker",
                "quote marker must be a single word",
            ),
            (
                MorphologyErrorKind::InvalidZoiDelimiter,
                "morphology.invalid-zoi-delimiter",
                "ZOI delimiter must be a single non-y word",
            ),
        ];

        for (kind, code, message) in cases {
            assert_eq!(kind.code(), code);
            assert_eq!(kind.message(), message);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_warning_kind_codes_are_stable() {
        let cases = [
            (
                MorphologyWarningKind::ExperimentalCgv,
                "morphology.warning.experimental-cgv",
                "experimental morphology: consonant-glide-vowel sequence",
            ),
            (
                MorphologyWarningKind::ExperimentalMz,
                "morphology.warning.experimental-mz",
                "experimental morphology: MZ consonant pair",
            ),
            (
                MorphologyWarningKind::BreveNotGlide,
                "morphology.warning.breve-not-glide",
                "breve-marked vowel is not in a glide position",
            ),
        ];

        for (kind, code, message) in cases {
            assert_eq!(kind.code(), code);
            assert_eq!(kind.message(), message);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_diagnostic_uses_precise_vowel_hiatus_span() {
        let error = segment_words_with_modifiers("aa").expect_err("vowel hiatus must fail");
        let diagnostic = error.to_diagnostic(None, "aa");

        assert_eq!(diagnostic.code, "morphology.vowel-hiatus");
        let label = diagnostic.primary_label();
        assert_eq!(label.span.byte_start, 0);
        assert_eq!(label.span.byte_end, 2);
        assert_eq!(label.span.char_start, 0);
        assert_eq!(label.span.char_end, 2);
        assert_eq!(label.message, "vowels in hiatus are not allowed");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_diagnostic_maps_non_ascii_source_span() {
        let source = "éa";
        let error = segment_words_with_modifiers(source).expect_err("vowel hiatus must fail");
        let diagnostic = error.to_diagnostic(None, source);

        assert_eq!(diagnostic.code, "morphology.vowel-hiatus");
        let label = diagnostic.primary_label();
        assert_eq!(label.span.byte_start, 0);
        assert_eq!(label.span.byte_end, 3);
        assert_eq!(label.span.char_start, 0);
        assert_eq!(label.span.char_end, 2);
        assert!(diagnostic.styled_notes.iter().any(|note| {
            note.segments
                .iter()
                .any(|segment| segment.text.contains("adjacent vowel nuclei"))
        }));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_warning_diagnostic_maps_comma_crossing_cgv_span() {
        let source = "melxi,or.";
        let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
            source,
            &MorphologyOptions::default(),
            None,
        );
        let data = attempt.into_data();
        data.result.expect("CgV relaxation should parse");
        assert_eq!(data.warnings.len(), 1);
        let diagnostic = data.warnings[0].to_diagnostic(None, source);

        assert_eq!(diagnostic.code, "morphology.warning.experimental-cgv");
        let label = diagnostic.primary_label();
        assert_eq!(label.span.char_start, 3);
        assert_eq!(label.span.char_end, 7);
        assert_eq!(&source[label.span.byte_start..label.span.byte_end], "xi,o");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn morphology_warning_diagnostic_maps_comma_crossing_mz_span() {
        let source = "nam,zi";
        let attempt = segment_words_with_modifiers_with_options_and_source_id_attempt(
            source,
            &MorphologyOptions::default(),
            None,
        );
        let data = attempt.into_data();
        data.result.expect("MZ relaxation should parse");
        assert_eq!(data.warnings.len(), 1);
        let diagnostic = data.warnings[0].to_diagnostic(None, source);

        assert_eq!(diagnostic.code, "morphology.warning.experimental-mz");
        let label = diagnostic.primary_label();
        assert_eq!(label.span.char_start, 2);
        assert_eq!(label.span.char_end, 5);
        assert_eq!(&source[label.span.byte_start..label.span.byte_end], "m,z");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_morphology_resyncs_at_whitespace() {
        let source = "mi @@@ do";
        let strict_error = segment_words_with_modifiers(source).expect_err("strict API fails");
        let recovered = segment_words_with_modifiers_recovered(source);

        assert_eq!(base_phoneme_texts(&recovered.words), vec!["mi", "do"]);
        assert_eq!(recovered.errors, vec![strict_error]);
        assert_eq!(recovered.error_regions.len(), 1);
        assert_eq!(recovered.error_regions[0].char_start, 3);
        assert_eq!(recovered.error_regions[0].char_end, 7);
        assert_invalid_error(
            &recovered.errors[0],
            MorphologyErrorKind::InvalidCharacter,
            3,
            4,
            None,
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_morphology_reports_multiple_errors_in_order() {
        let recovered = segment_words_with_modifiers_recovered("mi @@@ do ### mi");

        assert_eq!(base_phoneme_texts(&recovered.words), vec!["mi", "do", "mi"]);
        assert_eq!(recovered.errors.len(), 2);
        assert_eq!(recovered.error_regions.len(), 2);
        assert_invalid_error(
            &recovered.errors[0],
            MorphologyErrorKind::InvalidCharacter,
            3,
            4,
            None,
        );
        assert_invalid_error(
            &recovered.errors[1],
            MorphologyErrorKind::InvalidCharacter,
            10,
            11,
            None,
        );
        assert_eq!(
            recovered
                .error_regions
                .iter()
                .map(|span| [span.char_start, span.char_end])
                .collect::<Vec<_>>(),
            vec![[3, 7], [10, 14]]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_morphology_stops_at_invalid_tail_without_whitespace() {
        let recovered = segment_words_with_modifiers_recovered("mi do @@@");

        assert_eq!(base_phoneme_texts(&recovered.words), vec!["mi", "do"]);
        assert_eq!(recovered.errors.len(), 1);
        assert_eq!(recovered.error_regions.len(), 1);
        assert_eq!(recovered.error_regions[0].char_start, 6);
        assert_eq!(recovered.error_regions[0].char_end, 9);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_morphology_resyncs_after_error_in_first_word() {
        let recovered = segment_words_with_modifiers_recovered("@@@ mi");

        assert_eq!(base_phoneme_texts(&recovered.words), vec!["mi"]);
        assert_eq!(recovered.errors.len(), 1);
        assert_eq!(recovered.error_regions.len(), 1);
        assert_eq!(recovered.error_regions[0].char_start, 0);
        assert_eq!(recovered.error_regions[0].char_end, 4);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_morphology_does_not_resync_unterminated_zoi() {
        let source = "zoi gy foo bar";
        let strict_error = segment_words_with_modifiers(source).expect_err("strict API fails");
        let recovered = segment_words_with_modifiers_recovered(source);

        assert!(recovered.words.is_empty());
        assert_eq!(recovered.errors, vec![strict_error]);
        assert_eq!(recovered.error_regions.len(), 1);
        assert_eq!(recovered.error_regions[0].char_start, 0);
        assert_eq!(recovered.error_regions[0].char_end, source.chars().count());
        assert!(matches!(
            recovered.errors[0],
            MorphologyError::UnterminatedZoiQuote { .. }
        ));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_morphology_preserves_warnings_from_valid_stretches() {
        let recovered = segment_words_with_modifiers_recovered("namzi @@@ kamzifre");

        assert_eq!(
            base_phoneme_texts(&recovered.words),
            vec!["námzi", "kamzífre"]
        );
        assert_eq!(recovered.errors.len(), 1);
        assert_eq!(recovered.warnings.len(), 2);
        assert!(
            recovered
                .warnings
                .iter()
                .all(|warning| warning.kind == MorphologyWarningKind::ExperimentalMz)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn segments_simple_cmavo_and_gismu() {
        let words = segment_words_with_modifiers("mi klama do").expect("valid morphology");
        assert_eq!(words.len(), 3);
        assert_eq!(base_word(&words[0]).map(Word::kind), Some(WordKind::Cmavo));
        assert_eq!(base_phonemes(&words[0]).as_deref(), Some("mi"));
        assert_eq!(base_word(&words[1]).map(Word::kind), Some(WordKind::Gismu));
        assert_eq!(base_phonemes(&words[1]).as_deref(), Some("kláma"));
        assert_eq!(base_word(&words[2]).map(Word::kind), Some(WordKind::Cmavo));
        assert_eq!(
            base_word(&words[2]).map(|word| word.span().char_start),
            Some(9)
        );
        assert_eq!(
            base_word(&words[2]).map(|word| word.span().char_end),
            Some(11)
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn segments_cyrillic_cmavo_and_gismu() {
        let words = segment_words_with_modifiers("ми клама до").expect("valid morphology");

        assert_eq!(base_phoneme_texts(&words), vec!["mi", "kláma", "do"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn segments_zbalermorna_cmavo_and_gismu() {
        let words = segment_words_with_modifiers(
            "\u{ed87}\u{eda2} \u{ed82}\u{ed84}\u{eda0}\u{ed87}\u{eda0} \u{ed91}\u{eda3}",
        )
        .expect("valid morphology");

        assert_eq!(base_phoneme_texts(&words), vec!["mi", "kláma", "do"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn normalizes_cyrillic_aliases_and_implicit_apostrophe() {
        let cases = [
            ("шой", "coĭ"),
            ("щой", "coĭ"),
            ("шо'и", "co'i"),
            ("шоһи", "co'i"),
            ("шои", "co'i"),
            ("мі", "mi"),
            ("лэ", "le"),
            ("лє", "le"),
            ("ӏфіныксӏ", "finyks"),
        ];

        for (source, expected) in cases {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            assert_eq!(
                base_phonemes(&words[0]).as_deref(),
                Some(expected),
                "{source}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn normalizes_zbalermorna_glyphs() {
        let cases = [
            ("\u{ed87}\u{edb2}", "mi"),
            ("\u{ed86}\u{eda6}", "caĭ"),
            ("\u{ed82}\u{eda7}", "keĭ"),
            ("\u{ed86}\u{eda8}", "coĭ"),
            ("\u{ed86}\u{eda9}", "caŭ"),
            ("\u{ed86}\u{edb3}\u{edaa}", "coĭ"),
            ("\u{ed86}\u{edb3}\u{ed8a}\u{edb2}", "co'i"),
            ("\u{ed8b}\u{eda0}\u{eda1}", "a'e"),
            ("\u{ed86}\u{ed99}\u{ed9b}\u{ed8c}\u{eda8}", "coĭ"),
            (
                "\u{ed89}\u{ed83}\u{edb2}\u{ed97}\u{eda5}\u{ed82}\u{ed85}\u{ed89}",
                "finyks",
            ),
            (
                "\u{ed89}\u{edb0}\u{ed84}\u{edb2}\u{ed98}\u{ed85}\u{ed89}",
                "alís",
            ),
        ];

        for (source, expected) in cases {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            assert_eq!(
                base_phonemes(&words[0]).as_deref(),
                Some(expected),
                "{source}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zbalermorna_rejects_unsupported_private_use_glyphs() {
        let error = segment_words_with_modifiers("\u{ed86}\u{edac}")
            .expect_err("unsupported zbalermorna glyph must not be silently dropped");

        assert!(
            matches!(
                error,
                MorphologyError::Invalid {
                    kind: MorphologyErrorKind::InvalidCharacter,
                    char_start: 1,
                    ..
                }
            ),
            "{error:?}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn normalizes_cyrillic_stress_marks() {
        let cases = [
            ("ӏалисӏ", "alis"),
            ("ӏалІсӏ", "alís"),
            ("ӏалі\u{0301}сӏ", "alís"),
            ("ӏалі\u{0300}сӏ", "alís"),
        ];

        for (source, expected) in cases {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            assert_eq!(
                base_phonemes(&words[0]).as_deref(),
                Some(expected),
                "{source}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cyrillic_glide_letters_are_rejected_outside_glide_positions() {
        let error = segment_words_with_modifiers("й").expect_err("glide must be rejected");

        assert!(
            matches!(
                error,
                MorphologyError::Invalid {
                    kind: MorphologyErrorKind::BreveNotGlide,
                    ..
                }
            ),
            "{error:?}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn display_segmentation_keeps_magic_words_visible() {
        let si_words = segment_words_for_display("mi si").expect("valid display morphology");
        assert_eq!(base_phoneme_texts(&si_words), vec!["mi", "si"]);

        let zei_words = segment_words_for_display("zei").expect("valid display morphology");
        assert_eq!(base_phoneme_texts(&zei_words), vec!["zeĭ"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn normalizes_lojban_input_text_for_exact_lookups() {
        assert_eq!(
            normalize_lojban_input_text("ми клама").as_deref(),
            Some("mi klama")
        );
        assert_eq!(normalize_lojban_input_text("шои").as_deref(), Some("co'i"));
        assert_eq!(normalize_lojban_input_text("шой").as_deref(), Some("coi"));
        assert_eq!(
            normalize_lojban_input_text("ӏфіныксӏ").as_deref(),
            Some(".finyks.")
        );
        assert_eq!(
            normalize_lojban_input_text(
                "\u{ed87}\u{eda2} \u{ed82}\u{ed84}\u{eda0}\u{ed87}\u{eda0}"
            )
            .as_deref(),
            Some("mi klama")
        );
        assert_eq!(
            normalize_lojban_input_text("\u{ed86}\u{eda8}").as_deref(),
            Some("coi")
        );
        assert_eq!(
            normalize_lojban_input_text("\u{ed86}\u{edb3}\u{ed8a}\u{edb2}").as_deref(),
            Some("co'i")
        );
        assert_eq!(
            normalize_lojban_input_text(
                "\u{ed89}\u{ed83}\u{edb2}\u{ed97}\u{eda5}\u{ed82}\u{ed85}\u{ed89}"
            )
            .as_deref(),
            Some(".finyks.")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zbalermorna_input_does_not_insert_implicit_apostrophes() {
        let error = segment_words_with_modifiers("\u{eda0}\u{eda0}")
            .expect_err("zbalermorna vowels should not insert apostrophes");

        assert!(
            matches!(
                error,
                MorphologyError::Invalid {
                    kind: MorphologyErrorKind::VowelHiatus,
                    ..
                }
            ),
            "{error:?}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn splits_adjacent_cmavo() {
        let words = segment_words_with_modifiers("mimi").expect("valid morphology");
        let phonemes: Vec<_> = words
            .iter()
            .map(|word| base_word(word).expect("base word").phonemes().into_string())
            .collect();
        assert_eq!(phonemes, vec!["mi".to_owned(), "mi".to_owned()]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn marks_cmavo_glides() {
        let words = segment_words_with_modifiers("coi .ui").expect("valid morphology");
        let phonemes: Vec<_> = words
            .iter()
            .map(|word| base_word(word).expect("base word").phonemes().into_string())
            .collect();
        assert_eq!(phonemes, vec!["coĭ".to_owned(), "ŭi".to_owned()]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn phonemes_render_stress_and_glides() {
        let phonemes = Phonemes::from_canonical("bródacoĭ".to_owned()).expect("valid phonemes");
        assert_eq!(phonemes.render(PhonemeRenderOptions::default()), "bródacoĭ");
        assert_eq!(
            phonemes.render(PhonemeRenderOptions {
                mark_stress: StressMark::None,
                mark_glides: GlideMark::None,
            }),
            "brodacoi"
        );
        assert_eq!(
            phonemes.render(PhonemeRenderOptions {
                mark_stress: StressMark::Caps,
                mark_glides: GlideMark::Breve,
            }),
            "brOdacoĭ"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_cbm_dialect_to_morphology_options() {
        let dialect = DialectDefinition {
            cmavo_entries: Vec::new(),
            features: std::collections::BTreeSet::from([DialectFeature::Cbm]),
        };
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);
        let words = segment_words_with_modifiers_with_options_and_source_id(
            "mi .alis. do sa broda",
            &options,
            None,
        )
        .expect("valid morphology");
        let phonemes: Vec<_> = words
            .iter()
            .map(|word| base_word(word).expect("base word").phonemes().into_string())
            .collect();
        assert_eq!(phonemes, vec!["mi".to_owned(), "bróda".to_owned()]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_case_insensitive_dialect_to_morphology_options() {
        let dialect = DialectDefinition {
            cmavo_entries: Vec::new(),
            features: std::collections::BTreeSet::from([DialectFeature::CaseInsensitive]),
        };
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);
        let words =
            segment_words_with_modifiers_with_options_and_source_id("NALSELTRO", &options, None)
                .expect("valid morphology");
        assert_eq!(base_phonemes(&words[0]).as_deref(), Some("nalséltro"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_combined_dialect_formula_to_morphology_options() {
        let dialect = DialectDefinition {
            cmavo_entries: Vec::new(),
            features: std::collections::BTreeSet::from([DialectFeature::CaseInsensitive]),
        };
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);
        let words =
            segment_words_with_modifiers_with_options_and_source_id("la ITALIAS.", &options, None)
                .expect("valid morphology");
        assert_eq!(base_phonemes(&words[1]).as_deref(), Some("italĭas"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_cmavo_dialect_swaps_in_order() {
        let dialect = DialectDefinition {
            cmavo_entries: vec![
                new!(CmavoDialectEntry::Swap {
                    left: "ce'u".to_owned(),
                    right: "ce".to_owned(),
                }),
                new!(CmavoDialectEntry::Swap {
                    left: "ce'u".to_owned(),
                    right: "ki".to_owned(),
                }),
            ],
            features: std::collections::BTreeSet::new(),
        };
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);

        let words = segment_words_with_modifiers_with_options_and_source_id("ce", &options, None)
            .expect("valid morphology");

        assert_eq!(base_phoneme_texts(&words), vec!["ki"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_cmavo_dialect_expansions() {
        let dialect = DialectDefinition {
            cmavo_entries: vec![new!(CmavoDialectEntry::Expansion {
                source: "la'u".to_owned(),
                replacement: vec!["la'e".to_owned(), "di'u".to_owned()],
            })],
            features: std::collections::BTreeSet::new(),
        };
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);

        let words = segment_words_with_modifiers_with_options_and_source_id("la'u", &options, None)
            .expect("valid morphology");

        assert_eq!(base_phoneme_texts(&words), vec!["la'e", "di'u"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_compiled_non_cmavo_dialect_entries() {
        let dialect =
            jbotci_dialect::parse_dialect_definition("((klama <-> cadzu))").expect("dialect");
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);

        let words =
            segment_words_with_modifiers_with_options_and_source_id("mi klama", &options, None)
                .expect("valid morphology");

        assert_eq!(base_phoneme_texts(&words), vec!["mi", "cádzu"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rejects_morphologically_invalid_compiled_dialect_words() {
        let dialect = jbotci_dialect::parse_dialect_definition("((aaa <-> eee))").expect("dialect");

        assert!(
            MorphologyOptions::default()
                .try_with_dialect_definition(&dialect)
                .is_err()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn applies_multiple_cmavo_dialect_entries() {
        let dialect = DialectDefinition {
            cmavo_entries: vec![
                new!(CmavoDialectEntry::Expansion {
                    source: "po".to_owned(),
                    replacement: vec!["lo".to_owned(), "su'u".to_owned()],
                }),
                new!(CmavoDialectEntry::Expansion {
                    source: "nei".to_owned(),
                    replacement: vec!["kei".to_owned()],
                }),
                new!(CmavoDialectEntry::Swap {
                    left: "ce'u".to_owned(),
                    right: "ce".to_owned(),
                }),
                new!(CmavoDialectEntry::Swap {
                    left: "ke'a".to_owned(),
                    right: "ki".to_owned(),
                }),
                new!(CmavoDialectEntry::Swap {
                    left: "tu'a".to_owned(),
                    right: "tau".to_owned(),
                }),
                new!(CmavoDialectEntry::Swap {
                    left: "su'o".to_owned(),
                    right: "su".to_owned(),
                }),
            ],
            features: std::collections::BTreeSet::new(),
        };
        let options = MorphologyOptions::default().with_dialect_definition(&dialect);

        let words = segment_words_with_modifiers_with_options_and_source_id(
            "po nei ce ki tau su'o",
            &options,
            None,
        )
        .expect("valid morphology");

        assert_eq!(
            base_phoneme_texts(&words),
            vec!["lo", "su'u", "keĭ", "ce'u", "ke'a", "tu'a", "su"]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn decomposes_v0_lujvo_examples() {
        let cases: &[(&str, &[&str])] = &[
            ("gerzda", &["rafsi:ger", "rafsi:zda"]),
            ("sutkla", &["rafsi:sut", "rafsi:kla"]),
            ("ge'urzdani", &["rafsi:ge'u", "hyphen:r", "rafsi:zdani"]),
            ("ba'irgau", &["rafsi:ba'i", "hyphen:r", "rafsi:gaŭ"]),
            ("so'irdja", &["rafsi:so'i", "hyphen:r", "rafsi:dja"]),
            ("ci'artai", &["rafsi:ci'a", "hyphen:r", "rafsi:taĭ"]),
            ("ro'inre'o", &["rafsi:ro'i", "hyphen:n", "rafsi:re'o"]),
            ("baurgri", &["rafsi:baŭ", "hyphen:r", "rafsi:gri"]),
            ("bajyzda", &["rafsi:baj", "hyphen:y", "rafsi:zda"]),
            ("kamykla", &["rafsi:kam", "hyphen:y", "rafsi:kla"]),
            ("papykla", &["rafsi:pap", "hyphen:y", "rafsi:kla"]),
            ("selpa'i", &["rafsi:sel", "rafsi:pa'i"]),
            ("tolsi'arai", &["rafsi:tol", "rafsi:si'a", "rafsi:raĭ"]),
            (
                "jboplijvogau",
                &["rafsi:jbo", "rafsi:pli", "rafsi:jvo", "rafsi:gaŭ"],
            ),
            ("baibra", &["rafsi:baĭ", "rafsi:bra"]),
            ("xlagau", &["rafsi:xla", "rafsi:gaŭ"]),
        ];

        for (source, expected) in cases {
            assert_eq!(lujvo_part_labels(source), *expected, "{source}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn cmavo_do_not_have_lujvo_parts() {
        for source in ["mi", "do", "lo"] {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let word = base_word(&words[0]).expect("base word");
            assert!(word.lujvo_parts().is_none(), "{source}");
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syntax_equivalence_ignores_spans_and_diacritics_on_words() {
        let mut left = segment_words_with_modifiers("coi").expect("valid morphology");
        let mut right = segment_words_with_modifiers("coi").expect("valid morphology");
        let word = match right[0].as_data() {
            data!(WordLike::PlainWord(word)) => word.clone(),
            _ => panic!("expected bare word"),
        };
        right[0] = WordLike::bare(Word::from_kind(
            word.kind(),
            Phonemes::from_canonical("coĭ".to_owned()).expect("valid phonemes"),
            SourceSpan::new(None, 99, 102, 99, 102).expect("valid span"),
        ));

        assert!(word_like_syntax_eq(&left.remove(0), &right.remove(0)));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn strip_diacritics_allows_combining_marks_only() {
        assert_eq!(strip_diacritics("\u{0301}\u{0300}\u{0306}"), "");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn diacritic_helpers_distinguish_glide_preservation_from_folding() {
        let source = "coĭ taŭ bródà\u{0301}";

        assert_eq!(strip_lojban_diacritics(source), "coĭ taŭ broda");
        assert_eq!(fold_lojban_diacritics(source), "coi tau broda");
        assert_eq!(strip_diacritics(source), "coi tau broda");
        assert!(stripped_lojban_diacritics_eq("coĭ", "coĭ"));
        assert!(!stripped_lojban_diacritics_eq("coĭ", "coi"));
        assert!(folded_lojban_diacritics_eq("coĭ", "coi"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syntax_equivalence_ignores_zoi_verbatim_spans_not_text() {
        let left = WordLike::zoi_quote(
            test_word(WordKind::Cmavo, "zoĭ", 0),
            test_word(WordKind::Cmavo, "gy", 4),
            Verbatim::new(
                SourceSpan::new(None, 7, 12, 7, 12).expect("valid span"),
                "broda".to_owned(),
            ),
            test_word(WordKind::Cmavo, "gy", 13),
        );
        let right = WordLike::zoi_quote(
            test_word(WordKind::Cmavo, "zoĭ", 20),
            test_word(WordKind::Cmavo, "gy", 24),
            Verbatim::new(
                SourceSpan::new(None, 27, 32, 27, 32).expect("valid span"),
                "broda".to_owned(),
            ),
            test_word(WordKind::Cmavo, "gy", 33),
        );
        let different_text = WordLike::zoi_quote(
            test_word(WordKind::Cmavo, "zoĭ", 20),
            test_word(WordKind::Cmavo, "gy", 24),
            Verbatim::new(
                SourceSpan::new(None, 27, 32, 27, 32).expect("valid span"),
                "brode".to_owned(),
            ),
            test_word(WordKind::Cmavo, "gy", 33),
        );

        assert!(word_like_syntax_eq(&left, &right));
        assert!(!word_like_syntax_eq(&left, &different_text));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syntax_equivalence_ignores_single_word_quote_verbatim_spans_not_text() {
        let left = WordLike::single_word_quote(
            test_word(WordKind::Cmavo, "zo'oi", 0),
            Verbatim::new(
                SourceSpan::new(None, 6, 11, 6, 11).expect("valid span"),
                "hello".to_owned(),
            ),
        );
        let right = WordLike::single_word_quote(
            test_word(WordKind::Cmavo, "zo'oi", 20),
            Verbatim::new(
                SourceSpan::new(None, 26, 31, 26, 31).expect("valid span"),
                "hello".to_owned(),
            ),
        );
        let different_text = WordLike::single_word_quote(
            test_word(WordKind::Cmavo, "zo'oi", 20),
            Verbatim::new(
                SourceSpan::new(None, 26, 31, 26, 31).expect("valid span"),
                "hullo".to_owned(),
            ),
        );

        assert!(word_like_syntax_eq(&left, &right));
        assert!(!word_like_syntax_eq(&left, &different_text));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn invalid_cmavo_dialect_entries_are_rejected() {
        let panic = std::panic::catch_unwind(|| {
            let _ = new!(CmavoDialectEntry::Expansion {
                source: "mi".to_owned(),
                replacement: Vec::new(),
            });
        });
        assert!(panic.is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn word_deserialization_rejects_invalid_words() {
        let error = serde_json::from_str::<Word>(
            r#"{
                "Cmavo": {
                    "phonemes": "",
                    "span": {
                        "source_id": null,
                        "byte_start": 0,
                        "byte_end": 0,
                        "char_start": 0,
                        "char_end": 0,
                        "start": null,
                        "end": null
                    }
                }
            }"#,
        )
        .expect_err("empty phoneme text must be rejected");

        assert!(error.to_string().contains("phoneme text must not be empty"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn word_like_deserializes_compact_constructor_json() {
        let word_like = serde_json::from_str::<WordLike>(
            r#"{
                "QuotedWord": {
                    "zo": {"Cmavo": {"phonemes": "zo", "span": {"source_id": null, "byte_start": 0, "byte_end": 2, "char_start": 0, "char_end": 2, "start": null, "end": null}}},
                    "word": {"Cmavo": {"phonemes": "coi", "span": {"source_id": null, "byte_start": 3, "byte_end": 6, "char_start": 3, "char_end": 6, "start": null, "end": null}}}
                }
            }"#,
        )
        .expect("compact constructor JSON should deserialize");

        let data!(WordLike::QuotedWord { zo, word }) = word_like.as_data() else {
            panic!("expected zo quote");
        };
        assert!(zo.is_cmavo(Cmavo::Zo));
        assert_eq!(word.phonemes().as_str(), "coi");
        assert_eq!(word.span().char_start, 3);
        assert_eq!(word.span().char_end, 6);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn word_like_constructor_rejects_wrong_zo_marker() {
        let panic = std::panic::catch_unwind(|| {
            let _ = WordLike::zo_quote(
                test_word(WordKind::Cmavo, "mi", 0),
                test_word(WordKind::Cmavo, "do", 3),
            );
        });
        assert!(panic.is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn word_like_constructor_rejects_wrong_bu_marker() {
        let panic = std::panic::catch_unwind(|| {
            let _ = WordLike::letter(
                WordLike::bare(test_word(WordKind::Cmavo, "a", 0)),
                test_word(WordKind::Cmavo, "cu", 2),
            );
        });
        assert!(panic.is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn word_like_constructor_rejects_unordered_zoi_quote_spans() {
        let panic = std::panic::catch_unwind(|| {
            let _ = WordLike::zoi_quote(
                test_word(WordKind::Cmavo, "zoi", 0),
                test_word(WordKind::Cmavo, "gy", 4),
                Verbatim::new(
                    SourceSpan::new(None, 10, 12, 10, 12).expect("valid test span"),
                    "xx".to_owned(),
                ),
                test_word(WordKind::Cmavo, "gy", 8),
            );
        });
        assert!(panic.is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn word_like_constructor_rejects_mismatched_zoi_quote_delimiters() {
        let panic = std::panic::catch_unwind(|| {
            let _ = WordLike::zoi_quote(
                test_word(WordKind::Cmavo, "zoi", 0),
                test_word(WordKind::Cmavo, "gy", 4),
                Verbatim::new(
                    SourceSpan::new(None, 7, 11, 7, 11).expect("valid test span"),
                    "test".to_owned(),
                ),
                test_word(WordKind::Cmavo, "ly", 12),
            );
        });
        assert!(panic.is_err());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn typed_cmavo_parsing_is_canonical_and_multi_class() {
        assert_eq!(Cmavo::from_text("NÁ'E"), Some(Cmavo::Nahe));
        assert_eq!(Cmavo::Nahe.canonical_text(), "na'e");
        assert!(Selmaho::Nahe.contains(Cmavo::Nahe));
        assert_eq!(Cmavo::Nahe.primary_selmaho(), Some(Selmaho::Nahe));

        assert!(Selmaho::Bai.contains(Cmavo::Lahei));
        assert!(Selmaho::Le.contains(Cmavo::Lahei));
        assert!(Selmaho::Ui.contains(Cmavo::Lahei));
        assert_eq!(Cmavo::Lahei.primary_selmaho(), Some(Selmaho::Bai));

        let word = test_word(WordKind::Cmavo, "na'e", 0);
        assert_eq!(word.selmaho_kind(), Some(Selmaho::Nahe));
        assert_eq!(word.selmaho(), Some("NAhE"));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_cmavo_round_trips_through_canonical_text() {
        assert_eq!(Cmavo::ALL.len(), Cmavo::Zy as usize + 1);
        for (expected_index, cmavo) in Cmavo::ALL.iter().copied().enumerate() {
            assert_eq!(cmavo as usize, expected_index);
            assert_eq!(
                Cmavo::from_text(cmavo.canonical_text()),
                Some(cmavo),
                "{cmavo:?} canonical text does not map back to the same variant"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn every_selmaho_round_trips_through_name() {
        for selmaho in Selmaho::ALL.iter().copied() {
            assert_eq!(
                Selmaho::from_name(selmaho.name()),
                Some(selmaho),
                "{selmaho:?} name does not map back to the same variant"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn selmaho_all_is_complete_and_ordered_for_primary_precedence() {
        assert_eq!(Selmaho::ALL.len(), Selmaho::Zoi as usize + 1);
        for (expected_index, selmaho) in Selmaho::ALL.iter().copied().enumerate() {
            assert_eq!(selmaho as usize, expected_index);
            assert_eq!(
                Selmaho::ALL
                    .iter()
                    .copied()
                    .filter(|entry| *entry == selmaho)
                    .count(),
                1,
                "{selmaho:?} appears more than once in Selmaho::ALL"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn zantufa_1_17_terminal_reference_for_gohoi_and_lohoi() {
        let gohoi = [
            (Cmavo::Gohoi, "go'oi"),
            (Cmavo::Zehoi, "ze'oi"),
            (Cmavo::Tahai, "ta'ai"),
            (Cmavo::Bohei, "bo'ei"),
        ];
        for (cmavo, text) in gohoi {
            assert_eq!(Cmavo::from_text(text), Some(cmavo));
            assert!(!Selmaho::Goha.contains(cmavo));
        }

        let lohoi = [
            (Cmavo::Lohoi, "lo'oi"),
            (Cmavo::Xuhu, "xu'u"),
            (Cmavo::Xauha, "xau'a"),
            (Cmavo::Mauha, "mau'a"),
        ];
        for (cmavo, text) in lohoi {
            assert_eq!(Cmavo::from_text(text), Some(cmavo));
            assert!(Selmaho::Lohoi.contains(cmavo));
        }

        assert!(Selmaho::Soi.contains(Cmavo::Xoi));
        assert!(!Selmaho::Sei.contains(Cmavo::Xoi));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn typed_cmavo_checks_are_only_for_bare_cmavo_words() {
        let cmavo = test_word(WordKind::Cmavo, "zo", 0);
        let quoted = test_word(WordKind::Cmavo, "coi", 3);
        let word_like = WordLike::zo_quote(cmavo, quoted);

        assert_eq!(word_like.bare_word(), None);
        assert_eq!(word_like.cmavo(), None);
        assert!(!word_like.is_cmavo(Cmavo::Zo));
        assert!(!word_like.is_selmaho(Selmaho::Zo));

        let bare = WordLike::bare(test_word(WordKind::Cmavo, "zo", 0));
        assert_eq!(bare.bare_word().and_then(Word::cmavo), Some(Cmavo::Zo));
        assert!(bare.is_cmavo(Cmavo::Zo));
        assert!(bare.is_selmaho(Selmaho::Zo));
        assert!(bare.is_one_of_selmaho(&[Selmaho::A, Selmaho::Zo]));

        let letter = WordLike::letter(
            WordLike::bare(test_word(WordKind::Cmavo, "zo", 0)),
            test_word(WordKind::Cmavo, "bu", 3),
        );
        assert_eq!(letter.cmavo(), None);
        assert!(!letter.is_cmavo(Cmavo::Zo));

        let zei_lujvo = WordLike::zei_lujvo(
            WordLike::bare(test_word(WordKind::Cmavo, "zo", 0)),
            test_word(WordKind::Cmavo, "zei", 3),
            test_word(WordKind::Cmavo, "coi", 7),
        );
        assert_eq!(zei_lujvo.cmavo(), None);
        assert!(!zei_lujvo.is_cmavo(Cmavo::Zo));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quote_marker_cmavo_checks_quote_markers_only() {
        let zo_quote = WordLike::zo_quote(
            test_word(WordKind::Cmavo, "zo", 0),
            test_word(WordKind::Cmavo, "coi", 3),
        );
        assert_eq!(zo_quote.quote_marker_cmavo(), Some(Cmavo::Zo));
        assert!(zo_quote.is_quote_marker_cmavo(Cmavo::Zo));
        assert!(!zo_quote.is_cmavo(Cmavo::Zo));

        let zoi_quote = WordLike::zoi_quote(
            test_word(WordKind::Cmavo, "zoi", 0),
            test_word(WordKind::Cmavo, "gy", 4),
            Verbatim::new(
                SourceSpan::new(None, 7, 11, 7, 11).expect("valid test span"),
                "test".to_owned(),
            ),
            test_word(WordKind::Cmavo, "gy", 12),
        );
        assert_eq!(zoi_quote.quote_marker_cmavo(), Some(Cmavo::Zoi));

        let lohu_quote = WordLike::lohu_quote(
            test_word(WordKind::Cmavo, "lo'u", 0),
            vec![test_word(WordKind::Cmavo, "coi", 5)],
            test_word(WordKind::Cmavo, "le'u", 9),
        );
        assert_eq!(lohu_quote.quote_marker_cmavo(), Some(Cmavo::Lohu));

        let single_word_quote = WordLike::single_word_quote(
            test_word(WordKind::Cmavo, "zo'oi", 0),
            Verbatim::new(
                SourceSpan::new(None, 6, 11, 6, 11).expect("valid test span"),
                "hello".to_owned(),
            ),
        );
        assert_eq!(single_word_quote.quote_marker_cmavo(), Some(Cmavo::Zohoi));

        let letter = WordLike::letter(
            WordLike::bare(test_word(WordKind::Cmavo, "a", 0)),
            test_word(WordKind::Cmavo, "bu", 2),
        );
        assert_eq!(letter.quote_marker_cmavo(), None);
    }

    #[requires(!phonemes.is_empty())]
    #[ensures(ret.kind() == kind)]
    fn test_word(kind: WordKind, phonemes: &str, byte_start: usize) -> Word {
        let byte_end = byte_start + phonemes.len();
        let char_end = byte_start + phonemes.chars().count();
        Word::from_kind(
            kind,
            Phonemes::from_canonical(phonemes.to_owned()).expect("valid test phonemes"),
            SourceSpan::new(None, byte_start, byte_end, byte_start, char_end)
                .expect("valid test span"),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn base_word(word: &WordLike) -> Option<&Word> {
        match word.as_data() {
            data!(WordLike::PlainWord(word)) => Some(word),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn base_phonemes(word: &WordLike) -> Option<String> {
        base_word(word).map(|word| word.phonemes().into_string())
    }

    #[requires(true)]
    #[ensures(ret.iter().all(|text| !text.is_empty()))]
    fn base_phoneme_texts(words: &[WordLike]) -> Vec<String> {
        words
            .iter()
            .map(|word| base_phonemes(word).expect("base word"))
            .collect()
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_invalid_error(
        error: &MorphologyError,
        expected_kind: MorphologyErrorKind,
        expected_start: usize,
        expected_end: usize,
        expected_context: Option<MorphologyContextKind>,
    ) {
        let MorphologyError::Invalid {
            kind,
            char_start,
            char_end,
            context,
            ..
        } = error
        else {
            panic!("expected invalid morphology error, got {error:?}");
        };
        assert_eq!(*kind, expected_kind);
        assert_eq!(*char_start, expected_start);
        assert_eq!(*char_end, expected_end);
        assert_eq!(
            context.as_ref().map(|context| context.kind),
            expected_context
        );
    }

    #[requires(!source.is_empty())]
    #[ensures(ret.iter().all(|label| !label.is_empty()))]
    fn lujvo_part_labels(source: &str) -> Vec<String> {
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        let word = base_word(&words[0]).expect("base word");
        word.lujvo_parts()
            .expect("lujvo parts")
            .iter()
            .map(jvopau_label)
            .collect()
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn jvopau_label(part: &LujvoPart) -> String {
        match part {
            LujvoPart::Rafsi(phonemes) => format!("rafsi:{}", render_unstressed(phonemes)),
            LujvoPart::Hyphen(phonemes) => format!("hyphen:{}", render_unstressed(phonemes)),
        }
    }

    #[requires(!source.is_empty())]
    #[ensures(ret.iter().all(|syllable| !syllable.is_empty()))]
    fn pronunciation_syllables_for_test(source: &str) -> Vec<String> {
        let phonemes = Phonemes::from_canonical(source.to_owned()).expect("valid phonemes");
        pronunciation_syllables(&phonemes).expect("syllabified phonemes")
    }

    #[requires(!phonemes.as_str().is_empty())]
    #[ensures(!ret.is_empty())]
    fn render_unstressed(phonemes: &Phonemes) -> String {
        phonemes.render(PhonemeRenderOptions {
            mark_stress: StressMark::None,
            mark_glides: GlideMark::Breve,
        })
    }
}
