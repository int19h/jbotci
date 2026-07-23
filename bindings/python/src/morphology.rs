//! Strongly typed Python projection of morphology parsing and result models.

use std::borrow::Cow;
use std::num::NonZeroUsize;
use std::sync::Arc;

use bityzba::{contract_trait, data, ensures, expensive_ensures, invariant, new, requires};
use jbotci_diagnostics::source_span_from_char_offsets;
use jbotci_morphology::{
    Cmavo, CompiledDialectDefinition, CompiledDialectEntry, CompiledDialectEntryData,
    CompiledDialectWord, ConsonantPairClass, DialectCompilationError, DialectCompilationErrorData,
    ExpectedWordDetailKind, GlideMark, LeadingPauseContext, LeadingPauseVowelMode, LujvoBuildMode,
    LujvoBuildPart, LujvoBuildPartData, LujvoCandidate, LujvoParseExpectation, LujvoPart,
    MORPHOLOGY_TRACE_FILTERS, MorphologyContext, MorphologyContextKind,
    MorphologyError as RustMorphologyError, MorphologyErrorDetail, MorphologyErrorDetailData,
    MorphologyErrorKind, MorphologyOptions, MorphologySegmentAttempt, MorphologyWarning,
    MorphologyWarningKind, PERMISSIVE_IGNORABLE_RESERVED_CHARACTERS, PhonemeRenderOptions,
    Phonemes, PhonotacticDetailKind, PlainWordClassification, RafsiShape,
    RecoveredMorphologySegmentAttempt, RecoveredMorphologySegmentation, Selmaho, StressMark,
    StringEnumMetadata, ValsiAnalysis, ValsiAnalysisResult, ValsiAnalysisStatus,
    ValsiClassification, ValsiClassificationData, ValsiClassificationKind, ValsiFuhivlaStage,
    ValsiLujvoPart, ValsiLujvoPartKind, ValsiLujvoRafsiKind, Verbatim, Word, WordKey, WordKind,
    WordLike, WordLikeData, ZoiDelimiterDetailKind,
};
use jbotci_syntax::{Token, WithIndicators, WithIndicatorsData};
use jbotci_tree::TreePath;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule};

use crate::InvalidInputError;
use crate::diagnostics::PyTraceOptions;
use crate::diagnostics::{PyDiagnostic, PyTraceReport};
use crate::dialect::PyDialectDefinition;
use crate::source::{
    PySourceId, PySourceSpan, source_location_error_from_python, source_location_error_to_python,
};
use crate::support::{
    PythonStringEnum, extract_sequence, extract_string_enum, public_exception_with_value,
    register_private_object, register_string_enum, register_type, sequence_to_tuple,
    string_enum_member, string_repr,
};

const PUBLIC_MODULE: &str = "jbotci.morphology";

pub(crate) const NATIVE_EXPORTS: &[&str] = &[
    "_morphology_MORPHOLOGY_TRACE_FILTERS",
    "_morphology_PERMISSIVE_IGNORABLE_RESERVED_CHARACTERS",
    "_morphology_WordKind",
    "_morphology_ValsiAnalysisStatus",
    "_morphology_ValsiClassificationKind",
    "_morphology_ValsiLujvoPartKind",
    "_morphology_ValsiLujvoRafsiKind",
    "_morphology_ValsiFuhivlaStage",
    "_morphology_StressMark",
    "_morphology_GlideMark",
    "_morphology_MorphologyErrorKind",
    "_morphology_MorphologyWarningKind",
    "_morphology_MorphologyContextKind",
    "_morphology_LujvoParseExpectation",
    "_morphology_ExpectedWordDetailKind",
    "_morphology_ZoiDelimiterDetailKind",
    "_morphology_PhonotacticDetailKind",
    "_morphology_LujvoBuildMode",
    "_morphology_RafsiShape",
    "_morphology_ConsonantPairClass",
    "_morphology_LeadingPauseVowelMode",
    "_morphology_LeadingPauseContext",
    "_morphology_Cmavo",
    "_morphology_Selmaho",
    "_morphology_PhonemeRenderOptions",
    "_morphology_Phonemes",
    "_morphology_WordKey",
    "_morphology_MorphologyOptions",
    "_morphology_CompiledDialectDefinition",
    "_morphology_InvalidDialectWord",
    "_morphology_CompiledDialectSwap",
    "_morphology_CompiledDialectExpansion",
    "_morphology_CompiledDialectWord",
    "_morphology_LujvoRafsi",
    "_morphology_LujvoHyphen",
    "_morphology_Verbatim",
    "_morphology_CmavoWord",
    "_morphology_GismuWord",
    "_morphology_LujvoWord",
    "_morphology_FuhivlaWord",
    "_morphology_CmevlaWord",
    "_morphology_PlainWord",
    "_morphology_QuotedWord",
    "_morphology_SelmahoQuotedWord",
    "_morphology_DelimitedNonLojbanQuote",
    "_morphology_QuotedWords",
    "_morphology_DelimitedWordQuote",
    "_morphology_LerfuWord",
    "_morphology_ZeiCompound",
    "_morphology_MorphologyContext",
    "_morphology_MorphologyWarning",
    "_morphology_InvalidLujvoDetail",
    "_morphology_FuhivlaContainsYDetail",
    "_morphology_SlinkuhiDetail",
    "_morphology_ExpectedWordDetail",
    "_morphology_InvalidZoiDelimiterDetail",
    "_morphology_PhonotacticDetail",
    "_morphology_InvalidMorphology",
    "_morphology_UnterminatedZoiQuote",
    "_morphology_SourceSpanMorphologyError",
    "_morphology_MorphologySegmentAttempt",
    "_morphology_RecoveredMorphologySegmentation",
    "_morphology_RecoveredMorphologySegmentAttempt",
    "_morphology_segment_attempt",
    "_morphology_segment_recovered_attempt",
    "_morphology_segment_for_display_attempt",
    "_morphology_ValsiLujvoPart",
    "_morphology_PlainWordClassification",
    "_morphology_PlainWordValsiClassification",
    "_morphology_QuotedWordValsiClassification",
    "_morphology_DelimitedNonLojbanQuoteValsiClassification",
    "_morphology_QuotedWordsValsiClassification",
    "_morphology_DelimitedWordQuoteValsiClassification",
    "_morphology_LerfuWordValsiClassification",
    "_morphology_ZeiCompoundValsiClassification",
    "_morphology_ValsiAnalysisResult",
    "_morphology_ValsiAnalysis",
    "_morphology_analyze_valsi",
    "_morphology_normalize_input",
    "_morphology_canonicalize_text",
    "_morphology_canonical_text_eq",
    "_morphology_canonical_text_is_all",
    "_morphology_normalize_cmavo_form",
    "_morphology_cmavo_phonemes",
    "_morphology_pronunciation_syllables",
    "_morphology_strip_lojban_diacritic",
    "_morphology_fold_lojban_diacritic",
    "_morphology_strip_lojban_diacritics",
    "_morphology_fold_lojban_diacritics",
    "_morphology_stripped_lojban_diacritics_eq",
    "_morphology_folded_lojban_diacritics_eq",
    "_morphology_strip_diacritics",
    "_morphology_strip_diacritics_eq",
    "_morphology_is_valid_phoneme",
    "_morphology_is_word_forming_character",
    "_morphology_is_period_character",
    "_morphology_is_permissive_ignorable_character",
    "_morphology_parse_lujvo_parts",
    "_morphology_parse_cmevla_lujvo_parts",
    "_morphology_parse_cmevla_lujvo_part_candidates",
    "_morphology_bond_rafsis",
    "_morphology_is_valid_lujvo_candidate_word",
    "_morphology_ensure_cmevla_word",
    "_morphology_ends_with_consonant",
    "_morphology_ends_with_vowel",
    "_morphology_is_bonding_hyphen",
    "_morphology_syllables_pattern",
    "_morphology_rafsi_shape",
    "_morphology_rafsi_shape_score",
    "_morphology_is_vowel",
    "_morphology_is_consonant",
    "_morphology_is_cmevla",
    "_morphology_consonant_pair_class",
    "_morphology_permissible_consonant_pair",
    "_morphology_consonant_pair_is_permissible",
    "_morphology_consonant_pair_is_initial",
    "_morphology_word_needs_leading_pause",
    "_morphology_word_needs_leading_pause_in_context",
    "_morphology_word_syntax_eq",
    "_morphology_word_like_syntax_eq",
    "_morphology_cmavo_from_text",
    "_morphology_cmavo_text",
    "_morphology_cmavo_is_selmaho",
    "_morphology_cmavo_primary_selmaho",
    "_morphology_cmavo_is_quote_opener",
    "_morphology_cmavo_is_single_word_quote_opener",
    "_morphology_cmavo_is_delimited_non_lojban_quote_opener",
    "_morphology_selmaho_from_name",
    "_morphology_selmaho_name",
    "_morphology_selmaho_contains",
    "_morphology_LujvoRafsiBuildPart",
    "_morphology_LujvoBrivlaCoreBuildPart",
    "_morphology_LujvoCandidate",
    "_morphology_choose_best_lujvo_candidate",
    "_morphology_choose_best_lujvo_candidate_from_parts",
];

macro_rules! impl_python_string_enum {
    ($type:ty, $native_name:literal, $python_name:literal) => {
        #[contract_trait]
        impl PythonStringEnum for $type {
            fn native_export_name() -> &'static str {
                $native_name
            }

            fn python_type_name() -> &'static str {
                $python_name
            }

            fn python_module_name() -> &'static str {
                PUBLIC_MODULE
            }

            fn python_doc() -> &'static str {
                concat!("jbotci morphology enum ", $python_name, ".")
            }

            fn variants() -> &'static [Self] {
                <$type as StringEnumMetadata>::variants()
            }

            fn python_member_name(self) -> Cow<'static, str> {
                Cow::Borrowed(StringEnumMetadata::variant_name(self))
            }

            fn python_value(self) -> &'static str {
                StringEnumMetadata::canonical_name(self)
            }
        }
    };
}

impl_python_string_enum!(WordKind, "_morphology_WordKind", "WordKind");
impl_python_string_enum!(
    ValsiAnalysisStatus,
    "_morphology_ValsiAnalysisStatus",
    "ValsiAnalysisStatus"
);
impl_python_string_enum!(
    ValsiClassificationKind,
    "_morphology_ValsiClassificationKind",
    "ValsiClassificationKind"
);
impl_python_string_enum!(
    ValsiLujvoPartKind,
    "_morphology_ValsiLujvoPartKind",
    "ValsiLujvoPartKind"
);
impl_python_string_enum!(
    ValsiLujvoRafsiKind,
    "_morphology_ValsiLujvoRafsiKind",
    "ValsiLujvoRafsiKind"
);
impl_python_string_enum!(
    ValsiFuhivlaStage,
    "_morphology_ValsiFuhivlaStage",
    "ValsiFuhivlaStage"
);
impl_python_string_enum!(StressMark, "_morphology_StressMark", "StressMark");
impl_python_string_enum!(GlideMark, "_morphology_GlideMark", "GlideMark");
impl_python_string_enum!(
    MorphologyErrorKind,
    "_morphology_MorphologyErrorKind",
    "MorphologyErrorKind"
);
impl_python_string_enum!(
    MorphologyWarningKind,
    "_morphology_MorphologyWarningKind",
    "MorphologyWarningKind"
);
impl_python_string_enum!(
    MorphologyContextKind,
    "_morphology_MorphologyContextKind",
    "MorphologyContextKind"
);
impl_python_string_enum!(
    LujvoParseExpectation,
    "_morphology_LujvoParseExpectation",
    "LujvoParseExpectation"
);
impl_python_string_enum!(
    ExpectedWordDetailKind,
    "_morphology_ExpectedWordDetailKind",
    "ExpectedWordDetailKind"
);
impl_python_string_enum!(
    ZoiDelimiterDetailKind,
    "_morphology_ZoiDelimiterDetailKind",
    "ZoiDelimiterDetailKind"
);
impl_python_string_enum!(
    PhonotacticDetailKind,
    "_morphology_PhonotacticDetailKind",
    "PhonotacticDetailKind"
);
impl_python_string_enum!(
    LujvoBuildMode,
    "_morphology_LujvoBuildMode",
    "LujvoBuildMode"
);
impl_python_string_enum!(RafsiShape, "_morphology_RafsiShape", "RafsiShape");
impl_python_string_enum!(
    ConsonantPairClass,
    "_morphology_ConsonantPairClass",
    "ConsonantPairClass"
);
impl_python_string_enum!(
    LeadingPauseVowelMode,
    "_morphology_LeadingPauseVowelMode",
    "LeadingPauseVowelMode"
);
impl_python_string_enum!(
    LeadingPauseContext,
    "_morphology_LeadingPauseContext",
    "LeadingPauseContext"
);

#[contract_trait]
impl PythonStringEnum for Cmavo {
    fn native_export_name() -> &'static str {
        "_morphology_Cmavo"
    }
    fn python_type_name() -> &'static str {
        "Cmavo"
    }
    fn python_module_name() -> &'static str {
        PUBLIC_MODULE
    }
    fn python_doc() -> &'static str {
        "Complete cmavo inventory generated from the Rust cmavo table."
    }
    fn variants() -> &'static [Self] {
        Cmavo::ALL
    }
    fn python_member_name(self) -> Cow<'static, str> {
        Cow::Owned(self.variant_name().to_ascii_uppercase())
    }
    fn python_value(self) -> &'static str {
        self.canonical_text()
    }
}

#[contract_trait]
impl PythonStringEnum for Selmaho {
    fn native_export_name() -> &'static str {
        "_morphology_Selmaho"
    }
    fn python_type_name() -> &'static str {
        "Selmaho"
    }
    fn python_module_name() -> &'static str {
        PUBLIC_MODULE
    }
    fn python_doc() -> &'static str {
        "Complete selmaho inventory generated from Rust metadata."
    }
    fn variants() -> &'static [Self] {
        Selmaho::ALL
    }
    fn python_member_name(self) -> Cow<'static, str> {
        Cow::Owned(self.name().to_ascii_uppercase())
    }
    fn python_value(self) -> &'static str {
        self.name()
    }
}

#[requires(true)]
#[ensures(true)]
fn native_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    py.import("jbotci._native")
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn enum_from_python<E: PythonStringEnum>(
    py: Python<'_>,
    value: &Bound<'_, PyAny>,
) -> PyResult<E> {
    extract_string_enum(&native_module(py)?, value)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn enum_to_python<E: PythonStringEnum>(py: Python<'_>, value: E) -> PyResult<Py<PyAny>> {
    string_enum_member(&native_module(py)?, value).map(Bound::unbind)
}

/// Options controlling phoneme rendering for display.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "PhonemeRenderOptions",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PyPhonemeRenderOptions {
    value: PhonemeRenderOptions,
}

impl PyPhonemeRenderOptions {
    #[requires(true)]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from_rust(value: PhonemeRenderOptions) -> Self {
        Self { value }
    }
}

#[pymethods]
impl PyPhonemeRenderOptions {
    /// Construct phoneme rendering options with Rust defaults for omitted fields.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (*, mark_stress=None, mark_glides=None))]
    fn new(
        py: Python<'_>,
        mark_stress: Option<&Bound<'_, PyAny>>,
        mark_glides: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let defaults = PhonemeRenderOptions::default();
        Ok(Self::from_rust(PhonemeRenderOptions {
            mark_stress: mark_stress.map_or(Ok(defaults.mark_stress), |value| {
                enum_from_python(py, value)
            })?,
            mark_glides: mark_glides.map_or(Ok(defaults.mark_glides), |value| {
                enum_from_python(py, value)
            })?,
        }))
    }

    /// Return the configured stress-marking style.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn mark_stress(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.mark_stress)
    }

    /// Return the configured glide-marking style.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn mark_glides(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.mark_glides)
    }
}

/// Canonical non-empty Lojban phoneme sequence.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "Phonemes",
    frozen,
    eq,
    hash,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PyPhonemes {
    value: Arc<Phonemes>,
}

impl PyPhonemes {
    #[requires(!value.as_str().is_empty())]
    #[expensive_ensures(ret.value.as_str() == old(value.clone()).as_str())]
    fn from_rust(value: Phonemes) -> Self {
        PyPhonemes {
            value: Arc::new(value),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_str() == self.value.as_str())]
    fn clone_rust(&self) -> Phonemes {
        self.value.as_ref().clone()
    }
}

#[pymethods]
impl PyPhonemes {
    /// Construct a validated non-empty canonical phoneme sequence.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(text: String) -> PyResult<Self> {
        if text.is_empty() {
            return Err(InvalidInputError::new_err("phoneme text must not be empty"));
        }
        Phonemes::from_canonical(text)
            .map(Self::from_rust)
            .map_err(InvalidInputError::new_err)
    }

    /// Return the canonical phoneme text.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn text(&self) -> &str {
        self.value.as_str()
    }

    /// Render phonemes with explicit stress and glide marking options.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn render(&self, options: PyRef<'_, PyPhonemeRenderOptions>) -> String {
        self.value.render(options.value)
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> &str {
        self.value.as_str()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "jbotci.morphology.Phonemes({})",
            string_repr(py, self.value.as_str())?
        ))
    }
}

/// Syntax identity key combining word kind and canonical phonemes.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "WordKey",
    frozen,
    eq,
    hash,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PyWordKey {
    value: WordKey,
}

impl PyWordKey {
    #[requires(!value.phonemes.as_str().is_empty())]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from_rust(value: WordKey) -> Self {
        PyWordKey { value }
    }
}

#[pymethods]
impl PyWordKey {
    /// Construct a syntax identity key from kind and canonical phonemes.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(
        py: Python<'_>,
        kind: &Bound<'_, PyAny>,
        phonemes: PyRef<'_, PyPhonemes>,
    ) -> PyResult<Self> {
        let kind = enum_from_python(py, kind)?;
        Ok(Self::from_rust(new!(WordKey {
            kind,
            phonemes: phonemes.clone_rust(),
        })))
    }

    /// Return the word kind component.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.kind)
    }

    /// Return the canonical phoneme component.
    #[requires(true)]
    #[ensures(ret.value.as_str() == self.value.phonemes.as_str())]
    #[getter]
    fn phonemes(&self) -> PyPhonemes {
        PyPhonemes::from_rust(self.value.phonemes.clone())
    }
}

#[invariant(
    !word.is_empty(),
    "dialect compilation errors retain a non-empty word"
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct InvalidDialectWordStorage {
    word: String,
}

/// Dialect-compilation failure carrying the exact invalid word.
#[invariant(
    true,
    "PyO3 requires the declared class shape; validated storage preserves a non-empty word"
)]
#[pyclass(
    name = "InvalidDialectWord",
    frozen,
    eq,
    hash,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PyInvalidDialectWord {
    value: InvalidDialectWordStorage,
}

#[pymethods]
impl PyInvalidDialectWord {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("word",);

    /// Construct an invalid-dialect-word detail with its exact spelling.
    #[requires(true)]
    #[ensures(
        ret.as_ref()
            .is_ok_and(|detail| detail.value.word == old(word.clone()))
            || ret.is_err()
    )]
    #[new]
    fn new(word: String) -> PyResult<Self> {
        if word.is_empty() {
            return Err(InvalidInputError::new_err(
                "invalid dialect word must not be empty",
            ));
        }
        Ok(PyInvalidDialectWord {
            value: new!(InvalidDialectWordStorage { word }),
        })
    }

    /// Return the exact word rejected by morphology compilation.
    #[requires(true)]
    #[ensures(ret == self.value.word.as_str())]
    #[getter]
    fn word(&self) -> &str {
        &self.value.word
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        format!(
            "dialect word is not morphologically valid: {}",
            self.value.word
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "jbotci.morphology.InvalidDialectWord(word={})",
            string_repr(py, &self.value.word)?
        ))
    }
}

#[requires(true)]
#[ensures(true)]
fn dialect_compilation_error_to_python(py: Python<'_>, error: DialectCompilationError) -> PyErr {
    let data!(DialectCompilationError::InvalidWord { word }) = error.into_data();
    let value = new!(InvalidDialectWordStorage { word });
    match Py::new(py, PyInvalidDialectWord { value }) {
        Ok(value) => public_exception_with_value(
            py,
            PUBLIC_MODULE,
            "DialectCompilationError",
            value.into_any(),
        ),
        Err(error) => error,
    }
}

#[invariant(
    ::Owned { .. } => true,
    "Arc ownership and CompiledDialectDefinition validation fully constrain this variant"
)]
#[invariant(
    ::Options { .. } => true,
    "Arc ownership and MorphologyOptions validation fully constrain this variant"
)]
#[derive(Debug, Clone)]
enum CompiledDialectStorage {
    Owned {
        value: Arc<CompiledDialectDefinition>,
    },
    Options {
        value: Arc<MorphologyOptions>,
    },
}

impl CompiledDialectStorage {
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &CompiledDialectDefinition {
        match self {
            Self::Owned { value } => value.as_ref(),
            Self::Options { value } => &value.compiled_dialect,
        }
    }
}

impl PartialEq for CompiledDialectStorage {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for CompiledDialectStorage {}

/// Parser-ready compiled morphology dialect definition.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "CompiledDialectDefinition",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyCompiledDialectDefinition {
    value: CompiledDialectStorage,
}

impl PyCompiledDialectDefinition {
    #[requires(true)]
    #[expensive_ensures(ret.value.get() == &old(value.clone()))]
    fn from_rust(value: CompiledDialectDefinition) -> Self {
        Self {
            value: CompiledDialectStorage::Owned {
                value: Arc::new(value),
            },
        }
    }

    #[requires(true)]
    #[expensive_ensures(ret.value.get() == &old(value.clone()).compiled_dialect)]
    fn from_options(value: Arc<MorphologyOptions>) -> Self {
        Self {
            value: CompiledDialectStorage::Options { value },
        }
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    fn rust(&self) -> &CompiledDialectDefinition {
        self.value.get()
    }
}

#[pymethods]
impl PyCompiledDialectDefinition {
    /// Compile a declarative morphology dialect, or the baseline when omitted.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (definition=None))]
    fn new(py: Python<'_>, definition: Option<PyRef<'_, PyDialectDefinition>>) -> PyResult<Self> {
        let value = match definition {
            Some(definition) => CompiledDialectDefinition::compile(definition.rust())
                .map_err(|error| dialect_compilation_error_to_python(py, error))?,
            None => CompiledDialectDefinition::default(),
        };
        Ok(Self::from_rust(value))
    }

    /// Return the immutable compiled dialect entries.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn entries(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        let values = self
            .value
            .get()
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| compiled_entry_to_python(py, self.value.clone(), index, entry))
            .collect::<PyResult<Vec<_>>>()?;
        crate::support::sequence_to_tuple(py, values).map(Bound::unbind)
    }
}

#[invariant(*index < owner.get().entries.len())]
#[derive(Debug, Clone)]
struct CompiledEntryHandle {
    owner: CompiledDialectStorage,
    index: usize,
}

impl PartialEq for CompiledEntryHandle {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for CompiledEntryHandle {}

impl CompiledEntryHandle {
    #[requires(index < owner.get().entries.len())]
    #[ensures(ret.index == index)]
    fn new(owner: CompiledDialectStorage, index: usize) -> Self {
        new!(CompiledEntryHandle { owner, index })
    }

    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &CompiledDialectEntry {
        &self.owner.get().entries[self.index]
    }
}

/// Compiled dialect entry swapping two parsed words.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "CompiledDialectSwap",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyCompiledDialectSwap {
    handle: CompiledEntryHandle,
}

#[pymethods]
impl PyCompiledDialectSwap {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("left", "right");

    /// Return the first parsed word in the swap.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn left(&self) -> PyCompiledDialectWord {
        PyCompiledDialectWord::new(CompiledWordHandle::new(
            self.handle.clone(),
            CompiledWordSlot::SwapLeft,
        ))
    }

    /// Return the second parsed word in the swap.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn right(&self) -> PyCompiledDialectWord {
        PyCompiledDialectWord::new(CompiledWordHandle::new(
            self.handle.clone(),
            CompiledWordSlot::SwapRight,
        ))
    }
}

/// Compiled dialect entry expanding one parsed word into a sequence.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "CompiledDialectExpansion",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyCompiledDialectExpansion {
    handle: CompiledEntryHandle,
}

#[pymethods]
impl PyCompiledDialectExpansion {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("source", "replacement");

    /// Return the parsed source word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn source(&self) -> PyCompiledDialectWord {
        PyCompiledDialectWord::new(CompiledWordHandle::new(
            self.handle.clone(),
            CompiledWordSlot::ExpansionSource,
        ))
    }

    /// Return the immutable parsed replacement words.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn replacement(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        let data!(CompiledDialectEntry::Expansion { replacement, .. }) =
            self.handle.get().as_data()
        else {
            unreachable!("private projection fixes the compiled dialect entry variant")
        };
        let values = (0..replacement.len()).map(|index| {
            PyCompiledDialectWord::new(CompiledWordHandle::new(
                self.handle.clone(),
                CompiledWordSlot::ExpansionReplacement { index },
            ))
        });
        crate::support::sequence_to_tuple(py, values).map(Bound::unbind)
    }
}

#[invariant(::SwapLeft => true)]
#[invariant(::SwapRight => true)]
#[invariant(::ExpansionSource => true)]
#[invariant(
    ::ExpansionReplacement { .. } => true,
    "replacement bounds are contextual to the enclosing compiled entry handle"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompiledWordSlot {
    SwapLeft,
    SwapRight,
    ExpansionSource,
    ExpansionReplacement { index: usize },
}

#[invariant(compiled_word_handle_resolves(entry.get(), *slot))]
#[derive(Debug, Clone)]
struct CompiledWordHandle {
    entry: CompiledEntryHandle,
    slot: CompiledWordSlot,
}

impl PartialEq for CompiledWordHandle {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for CompiledWordHandle {}

impl CompiledWordHandle {
    #[requires(compiled_word_handle_resolves(entry.get(), slot))]
    #[ensures(compiled_word_handle_resolves(ret.entry.get(), ret.slot))]
    fn new(entry: CompiledEntryHandle, slot: CompiledWordSlot) -> Self {
        new!(CompiledWordHandle { entry, slot })
    }

    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &CompiledDialectWord {
        match (self.entry.get().as_data(), self.slot) {
            (data!(CompiledDialectEntry::Swap { left, .. }), CompiledWordSlot::SwapLeft) => left,
            (data!(CompiledDialectEntry::Swap { right, .. }), CompiledWordSlot::SwapRight) => right,
            (
                data!(CompiledDialectEntry::Expansion { source, .. }),
                CompiledWordSlot::ExpansionSource,
            ) => source,
            (
                data!(CompiledDialectEntry::Expansion { replacement, .. }),
                CompiledWordSlot::ExpansionReplacement { index },
            ) => &replacement[index],
            _ => unreachable!("compiled-word handle is valid by construction"),
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn compiled_word_handle_resolves(entry: &CompiledDialectEntry, slot: CompiledWordSlot) -> bool {
    match (entry.as_data(), slot) {
        (
            data!(CompiledDialectEntry::Swap { .. }),
            CompiledWordSlot::SwapLeft | CompiledWordSlot::SwapRight,
        )
        | (data!(CompiledDialectEntry::Expansion { .. }), CompiledWordSlot::ExpansionSource) => {
            true
        }
        (
            data!(CompiledDialectEntry::Expansion { replacement, .. }),
            CompiledWordSlot::ExpansionReplacement { index },
        ) => index < replacement.len(),
        _ => false,
    }
}

/// Parsed word and syntax key stored in a compiled dialect entry.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "CompiledDialectWord",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyCompiledDialectWord {
    handle: CompiledWordHandle,
}

impl PyCompiledDialectWord {
    #[requires(true)]
    #[ensures(true)]
    fn new(handle: CompiledWordHandle) -> Self {
        Self { handle }
    }
}

#[pymethods]
impl PyCompiledDialectWord {
    /// Return the parsed morphology word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn word(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_to_python(py, WordHandle::from_compiled(self.handle.clone()))
    }

    /// Return the word's syntax identity key.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn key(&self) -> PyWordKey {
        PyWordKey::from_rust(self.handle.get().key.clone())
    }
}

#[requires(index < owner.get().entries.len())]
#[ensures(true)]
fn compiled_entry_to_python(
    py: Python<'_>,
    owner: CompiledDialectStorage,
    index: usize,
    entry: &CompiledDialectEntry,
) -> PyResult<Py<PyAny>> {
    let handle = CompiledEntryHandle::new(owner, index);
    match entry.as_data() {
        data!(CompiledDialectEntry::Swap { .. }) => {
            Ok(Py::new(py, PyCompiledDialectSwap { handle })?.into_any())
        }
        data!(CompiledDialectEntry::Expansion { .. }) => {
            Ok(Py::new(py, PyCompiledDialectExpansion { handle })?.into_any())
        }
    }
}

/// Complete immutable configuration for morphology operations.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "MorphologyOptions",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyMorphologyOptions {
    value: Arc<MorphologyOptions>,
}

impl PyMorphologyOptions {
    #[requires(value.max_recovery_errors.get() > 0)]
    #[expensive_ensures(ret.value.as_ref() == &old(value.clone()))]
    fn from_rust(value: MorphologyOptions) -> Self {
        PyMorphologyOptions {
            value: Arc::new(value),
        }
    }

    #[requires(true)]
    #[ensures(ret == self.value.as_ref())]
    fn rust(&self) -> &MorphologyOptions {
        self.value.as_ref()
    }
}

#[pymethods]
impl PyMorphologyOptions {
    /// Construct morphology options and validate the non-zero recovery limit.
    #[allow(clippy::too_many_arguments)]
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (*, accept_latin=true, accept_cyrillic=true, accept_zbalermorna=true, dialect=None, cmevla_as_relation_words=false, permissive_lexer=false, uppercase_marks_stress=true, max_recovery_errors=20, trace=None))]
    fn new(
        py: Python<'_>,
        accept_latin: bool,
        accept_cyrillic: bool,
        accept_zbalermorna: bool,
        dialect: Option<PyRef<'_, PyDialectDefinition>>,
        cmevla_as_relation_words: bool,
        permissive_lexer: bool,
        uppercase_marks_stress: bool,
        max_recovery_errors: usize,
        trace: Option<PyRef<'_, PyTraceOptions>>,
    ) -> PyResult<Self> {
        let max_recovery_errors = NonZeroUsize::new(max_recovery_errors).ok_or_else(|| {
            InvalidInputError::new_err("max_recovery_errors must be greater than zero")
        })?;
        let mut value = MorphologyOptions {
            accept_latin,
            accept_cyrillic,
            accept_zbalermorna,
            compiled_dialect: CompiledDialectDefinition::default(),
            cmevla_as_relation_words,
            permissive_lexer,
            uppercase_marks_stress,
            max_recovery_errors,
            trace: trace
                .as_ref()
                .map_or_else(jbotci_diagnostics::TraceOptions::disabled, |trace| {
                    trace.rust().clone()
                }),
        };
        if let Some(dialect) = dialect {
            value = value
                .try_with_dialect_definition(dialect.rust())
                .map_err(|error| dialect_compilation_error_to_python(py, error))?;
        }
        Ok(Self::from_rust(value))
    }

    /// Return the complete Rust default morphology options.
    #[requires(true)]
    #[ensures(true)]
    #[staticmethod]
    fn default() -> Self {
        Self::from_rust(MorphologyOptions::default())
    }

    /// Return a copy using an already compiled morphology dialect.
    #[requires(true)]
    #[ensures(ret.rust().compiled_dialect.entries.len() == dialect.rust().entries.len())]
    fn with_compiled_dialect(&self, dialect: PyRef<'_, PyCompiledDialectDefinition>) -> Self {
        Self::from_rust(MorphologyOptions {
            compiled_dialect: dialect.rust().clone(),
            ..self.value.as_ref().clone()
        })
    }

    /// Return a copy compiled from the supplied declarative dialect definition.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|options| options.rust().compiled_dialect.entries.len() == dialect.rust().cmavo_entries.len()) || ret.is_err())]
    fn with_dialect(
        &self,
        py: Python<'_>,
        dialect: PyRef<'_, PyDialectDefinition>,
    ) -> PyResult<Self> {
        self.value
            .as_ref()
            .clone()
            .try_with_dialect_definition(dialect.rust())
            .map(Self::from_rust)
            .map_err(|error| dialect_compilation_error_to_python(py, error))
    }

    /// Return a copy using the supplied immutable trace options.
    #[requires(true)]
    #[ensures(&ret.rust().trace == trace.rust())]
    fn with_trace(&self, trace: PyRef<'_, PyTraceOptions>) -> Self {
        Self::from_rust(
            self.value
                .as_ref()
                .clone()
                .with_trace_options(trace.rust().clone()),
        )
    }

    /// Return a copy with a validated non-zero recovery error limit.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|options| options.value.max_recovery_errors.get() == max_recovery_errors) || ret.is_err())]
    fn with_max_recovery_errors(&self, max_recovery_errors: usize) -> PyResult<Self> {
        if max_recovery_errors == 0 {
            return Err(InvalidInputError::new_err(
                "max_recovery_errors must be greater than zero",
            ));
        }
        Ok(Self::from_rust(
            self.value
                .as_ref()
                .clone()
                .with_max_recovery_errors(max_recovery_errors),
        ))
    }

    /// Report whether Latin orthography is accepted.
    #[requires(true)]
    #[ensures(ret == self.value.accept_latin)]
    #[getter]
    fn accept_latin(&self) -> bool {
        self.value.accept_latin
    }

    /// Report whether Cyrillic orthography is accepted.
    #[requires(true)]
    #[ensures(ret == self.value.accept_cyrillic)]
    #[getter]
    fn accept_cyrillic(&self) -> bool {
        self.value.accept_cyrillic
    }

    /// Report whether zbalermorna orthography is accepted.
    #[requires(true)]
    #[ensures(ret == self.value.accept_zbalermorna)]
    #[getter]
    fn accept_zbalermorna(&self) -> bool {
        self.value.accept_zbalermorna
    }

    /// Return the parser-ready compiled morphology dialect.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn compiled_dialect(&self) -> PyCompiledDialectDefinition {
        PyCompiledDialectDefinition::from_options(Arc::clone(&self.value))
    }

    /// Report whether cmevla are accepted as relation words.
    #[requires(true)]
    #[ensures(ret == self.value.cmevla_as_relation_words)]
    #[getter]
    fn cmevla_as_relation_words(&self) -> bool {
        self.value.cmevla_as_relation_words
    }

    /// Report whether permissive lexer recovery is enabled.
    #[requires(true)]
    #[ensures(ret == self.value.permissive_lexer)]
    #[getter]
    fn permissive_lexer(&self) -> bool {
        self.value.permissive_lexer
    }

    /// Report whether uppercase letters mark stress.
    #[requires(true)]
    #[ensures(ret == self.value.uppercase_marks_stress)]
    #[getter]
    fn uppercase_marks_stress(&self) -> bool {
        self.value.uppercase_marks_stress
    }

    /// Return the non-zero maximum recovered-error count.
    #[requires(true)]
    #[ensures(ret == self.value.max_recovery_errors.get())]
    #[getter]
    fn max_recovery_errors(&self) -> usize {
        self.value.max_recovery_errors.get()
    }

    /// Return the immutable trace configuration.
    #[requires(true)]
    #[ensures(ret.rust() == &self.value.trace)]
    #[getter]
    fn trace(&self) -> PyTraceOptions {
        PyTraceOptions::from_rust(self.value.trace.clone())
    }
}

/// Identity-preserving owner for a syntax token projected into morphology wrappers.
///
/// `Token` already contains its immutable `Arc<WithIndicators<WordLike>>`. Keeping the token
/// itself here lets later syntax bindings recover the exact token rather than guessing identity
/// from a source span, which is not unique for dialect-expansion siblings.
#[invariant(true, "Token enforces its own validated source-bearing invariant")]
#[derive(Debug, Clone)]
pub(crate) struct TokenHandle {
    value: Token,
}

impl PartialEq for TokenHandle {
    #[requires(true)]
    #[ensures(ret == Token::ptr_eq(self.get(), other.get()))]
    fn eq(&self, other: &Self) -> bool {
        Token::ptr_eq(self.get(), other.get())
    }
}

impl Eq for TokenHandle {}

impl TokenHandle {
    /// Retain the exact Arc-backed token supplied by a syntax owner.
    #[requires(true)]
    #[ensures(Token::ptr_eq(ret.get(), &old(value.clone())))]
    pub(crate) fn from_rust(value: Token) -> Self {
        TokenHandle { value }
    }

    /// Borrow the exact token, including its stable Arc identity.
    #[requires(true)]
    #[ensures(Token::ptr_eq(ret, &self.value))]
    pub(crate) fn get(&self) -> &Token {
        &self.value
    }

    /// Clone the token's Arc, not its recursive indicator or morphology tree.
    #[requires(true)]
    #[ensures(Token::ptr_eq(&ret, self.get()))]
    pub(crate) fn clone_rust(&self) -> Token {
        self.value.clone()
    }

    /// Locate the token's complete indicator tree without cloning it.
    #[requires(true)]
    #[ensures(ret.exact_token().is_some_and(|token| token == self))]
    pub(crate) fn indicators(&self) -> WithIndicatorsHandle {
        WithIndicatorsHandle::from_token(self.clone())
    }

    /// Locate the token's core morphology word-like value without cloning it.
    #[requires(true)]
    #[ensures(ret.root_token().is_some_and(|token| token == self))]
    pub(crate) fn core_word(&self) -> WordLikeHandle {
        self.indicators().core_word()
    }
}

#[invariant(::Base => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WithIndicatorsStep {
    Base,
}

#[invariant(::Owned { .. } => true)]
#[invariant(::Token { .. } => true)]
#[invariant(::Projected { owner, path, lens } =>
    !lens.is_empty() && owner.with_indicators_at(path, lens).is_some())]
// `new!` constructs the generated data variant, which rustc's dead-code lint
// cannot attribute back to this wrapper declaration.
#[allow(dead_code)]
#[derive(Debug, Clone)]
enum WithIndicatorsRoot {
    Owned {
        value: Arc<WithIndicators<WordLike>>,
    },
    Token {
        token: TokenHandle,
    },
    Projected {
        owner: Arc<crate::syntax::SyntaxOwner>,
        path: TreePath,
        lens: Vec<usize>,
    },
}

impl WithIndicatorsRoot {
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &WithIndicators<WordLike> {
        match self.as_data() {
            data!(WithIndicatorsRoot::Owned { value }) => value.as_ref(),
            data!(WithIndicatorsRoot::Token { token }) => token.get().as_indicators(),
            data!(WithIndicatorsRoot::Projected { owner, path, lens }) => owner
                .with_indicators_at(path, lens)
                .expect("projected indicator owner is valid by construction"),
        }
    }

    #[requires(true)]
    #[ensures(ret.is_none() || matches!(self.as_data(), data!(WithIndicatorsRoot::Token { .. })))]
    fn root_token(&self) -> Option<&TokenHandle> {
        match self.as_data() {
            data!(WithIndicatorsRoot::Owned { .. }) => None,
            data!(WithIndicatorsRoot::Token { token }) => Some(token),
            data!(WithIndicatorsRoot::Projected { .. }) => None,
        }
    }
}

/// Strongly typed path to one `WithIndicators<WordLike>` value.
#[invariant(with_indicators_path_resolves(root.get(), steps))]
#[derive(Debug, Clone)]
pub(crate) struct WithIndicatorsHandle {
    root: WithIndicatorsRoot,
    steps: Vec<WithIndicatorsStep>,
}

impl PartialEq for WithIndicatorsHandle {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for WithIndicatorsHandle {}

impl WithIndicatorsHandle {
    /// Own a standalone indicator tree created at a Python construction boundary.
    #[requires(true)]
    #[ensures(ret.steps.is_empty())]
    #[expensive_ensures(ret.get() == &old(value.clone()))]
    pub(crate) fn from_owned(value: WithIndicators<WordLike>) -> Self {
        new!(WithIndicatorsHandle {
            root: new!(WithIndicatorsRoot::Owned {
                value: Arc::new(value),
            }),
            steps: Vec::new(),
        })
    }

    /// Retain an exact syntax token as the owner of its indicator tree.
    #[requires(true)]
    #[ensures(ret.steps.is_empty())]
    #[ensures(
        ret.exact_token()
            .is_some_and(|exact| exact == &old(token.clone()))
    )]
    pub(crate) fn from_token(token: TokenHandle) -> Self {
        new!(WithIndicatorsHandle {
            root: new!(WithIndicatorsRoot::Token { token }),
            steps: Vec::new(),
        })
    }

    /// Retain a generated syntax root and its schema-derived direct-field lens.
    #[requires(!lens.is_empty())]
    #[ensures(ret.is_some() == old(owner.with_indicators_at(&path, &lens).is_some()))]
    #[ensures(ret.as_ref().is_none_or(|handle| handle.steps.is_empty()))]
    pub(crate) fn from_projection(
        owner: Arc<crate::syntax::SyntaxOwner>,
        path: TreePath,
        lens: Vec<usize>,
    ) -> Option<Self> {
        owner.with_indicators_at(&path, &lens)?;
        Some(new!(WithIndicatorsHandle {
            root: new!(WithIndicatorsRoot::Projected { owner, path, lens }),
            steps: Vec::new(),
        }))
    }

    /// Locate the recursive base of an indicator layer when one exists.
    #[requires(true)]
    #[ensures(ret.is_some() == matches!(self.get().as_data(), data!(WithIndicators::WithIndicator { .. })))]
    pub(crate) fn base(&self) -> Option<Self> {
        if !matches!(
            self.get().as_data(),
            data!(WithIndicators::WithIndicator { .. })
        ) {
            return None;
        }
        let mut steps = self.steps.clone();
        steps.push(WithIndicatorsStep::Base);
        Some(new!(WithIndicatorsHandle {
            root: self.root.clone(),
            steps,
        }))
    }

    /// Borrow the located indicator value without materializing its recursive tree.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn get(&self) -> &WithIndicators<WordLike> {
        project_with_indicators(self.root.get(), &self.steps)
            .expect("indicator handle is valid by construction")
    }

    /// Compare exact owner-and-path identity without conflating equal source spans.
    #[requires(true)]
    #[ensures(ret -> self.steps == other.steps)]
    pub(crate) fn same_identity(&self, other: &Self) -> bool {
        if self.steps != other.steps {
            return false;
        }
        match (self.root.as_data(), other.root.as_data()) {
            (
                data!(WithIndicatorsRoot::Owned { value: left }),
                data!(WithIndicatorsRoot::Owned { value: right }),
            ) => Arc::ptr_eq(left, right),
            (
                data!(WithIndicatorsRoot::Token { token: left }),
                data!(WithIndicatorsRoot::Token { token: right }),
            ) => left == right,
            (
                data!(WithIndicatorsRoot::Projected {
                    owner: left_owner,
                    path: left_path,
                    lens: left_lens,
                }),
                data!(WithIndicatorsRoot::Projected {
                    owner: right_owner,
                    path: right_path,
                    lens: right_lens,
                }),
            ) => {
                Arc::ptr_eq(left_owner, right_owner)
                    && left_path == right_path
                    && left_lens == right_lens
            }
            _ => false,
        }
    }

    /// Return the exact token that owns this locator, when it is token-backed.
    #[requires(true)]
    #[ensures(ret.is_none() == self.root.root_token().is_none())]
    pub(crate) fn root_token(&self) -> Option<&TokenHandle> {
        self.root.root_token()
    }

    /// Return this value as an exact complete token indicator tree, when possible.
    ///
    /// A nested `base` locator still retains its root token for lifetime and identity, but it is
    /// not itself the full indicator value represented by that token.
    #[requires(true)]
    #[ensures(ret.is_some() -> self.steps.is_empty())]
    pub(crate) fn exact_token(&self) -> Option<&TokenHandle> {
        if self.steps.is_empty() {
            self.root.root_token()
        } else {
            None
        }
    }

    /// Locate the core morphology word-like value without cloning it.
    #[requires(true)]
    #[ensures(ret.get() == self.get().core_word())]
    pub(crate) fn core_word(&self) -> WordLikeHandle {
        WordLikeHandle::from_indicators(self.clone())
    }

    /// Materialize an owned indicator value only for an owned Rust construction boundary.
    #[requires(true)]
    #[ensures(ret == self.get().clone())]
    pub(crate) fn into_owned(self) -> WithIndicators<WordLike> {
        self.get().clone()
    }
}

#[requires(true)]
#[ensures(
    ret.is_some()
        == matches!(
            (value.as_data(), step),
            (data!(WithIndicators::WithIndicator { .. }), WithIndicatorsStep::Base)
        )
)]
fn with_indicators_child(
    value: &WithIndicators<WordLike>,
    step: WithIndicatorsStep,
) -> Option<&WithIndicators<WordLike>> {
    match (value.as_data(), step) {
        (data!(WithIndicators::WithIndicator { base, .. }), WithIndicatorsStep::Base) => {
            Some(base.as_ref())
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_some() == with_indicators_path_resolves(root, steps))]
fn project_with_indicators<'a>(
    root: &'a WithIndicators<WordLike>,
    steps: &[WithIndicatorsStep],
) -> Option<&'a WithIndicators<WordLike>> {
    let mut current = root;
    for step in steps {
        current = with_indicators_child(current, *step)?;
    }
    Some(current)
}

#[requires(true)]
#[ensures(steps.is_empty() -> ret)]
fn with_indicators_path_resolves(
    root: &WithIndicators<WordLike>,
    steps: &[WithIndicatorsStep],
) -> bool {
    let mut current = root;
    for step in steps {
        let Some(child) = with_indicators_child(current, *step) else {
            return false;
        };
        current = child;
    }
    true
}

#[invariant(::Owned { .. } => true)]
#[invariant(::Indicators { .. } => true)]
#[derive(Debug, Clone)]
enum WordLikeRoot {
    Owned { value: Arc<WordLike> },
    Indicators { handle: WithIndicatorsHandle },
}

impl WordLikeRoot {
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &WordLike {
        match self {
            WordLikeRoot::Owned { value } => value.as_ref(),
            WordLikeRoot::Indicators { handle } => handle.get().core_word(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn root_token(&self) -> Option<&TokenHandle> {
        match self {
            WordLikeRoot::Owned { .. } => None,
            WordLikeRoot::Indicators { handle } => handle.root_token(),
        }
    }
}

#[invariant(::LerfuBase => true)]
#[invariant(::ZeiLeft => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WordLikeStep {
    LerfuBase,
    ZeiLeft,
}

#[invariant(word_like_path_resolves(root.get(), steps))]
#[derive(Debug, Clone)]
pub(crate) struct WordLikeHandle {
    root: WordLikeRoot,
    steps: Vec<WordLikeStep>,
}

impl PartialEq for WordLikeHandle {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for WordLikeHandle {}

impl WordLikeHandle {
    #[requires(true)]
    #[ensures(ret.steps.is_empty())]
    #[expensive_ensures(ret.get() == &old(value.clone()))]
    pub(crate) fn root(value: WordLike) -> Self {
        new!(WordLikeHandle {
            root: WordLikeRoot::Owned {
                value: Arc::new(value),
            },
            steps: Vec::new(),
        })
    }

    #[requires(true)]
    #[ensures(ret.steps.is_empty())]
    #[expensive_ensures(ret.get() == old(value.clone()).as_ref())]
    pub(crate) fn from_arc(value: Arc<WordLike>) -> Self {
        new!(WordLikeHandle {
            root: WordLikeRoot::Owned { value },
            steps: Vec::new(),
        })
    }

    #[requires(true)]
    #[ensures(ret.steps.is_empty())]
    #[expensive_ensures(ret.get() == old(handle.clone()).get().core_word())]
    pub(crate) fn from_indicators(handle: WithIndicatorsHandle) -> Self {
        new!(WordLikeHandle {
            root: WordLikeRoot::Indicators { handle },
            steps: Vec::new(),
        })
    }

    #[requires(word_like_step_resolves(self.get(), step))]
    #[ensures(ret.steps.len() == self.steps.len() + 1)]
    fn child(&self, step: WordLikeStep) -> Self {
        let mut steps = self.steps.clone();
        steps.push(step);
        new!(WordLikeHandle {
            root: self.root.clone(),
            steps,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn get(&self) -> &WordLike {
        project_word_like(self.root.get(), &self.steps)
            .expect("word-like handle is valid by construction")
    }

    /// Return the exact owning token when this projection originated in syntax.
    #[requires(true)]
    #[ensures(ret.is_none() == self.root.root_token().is_none())]
    pub(crate) fn root_token(&self) -> Option<&TokenHandle> {
        self.root.root_token()
    }

    /// Materialize the projected Rust subtree for an owned parser input.
    ///
    /// Python values and their children otherwise retain the shared root and typed path. The
    /// syntax binding should use `root_token` when a token-backed value crosses back into Rust;
    /// this owned clone is only for independently constructed morphology values.
    #[requires(true)]
    #[ensures(ret == self.get().clone())]
    pub(crate) fn into_owned(self) -> WordLike {
        self.get().clone()
    }
}

#[requires(true)]
#[ensures(ret.is_some() == word_like_step_resolves(value, step))]
fn word_like_child(value: &WordLike, step: WordLikeStep) -> Option<&WordLike> {
    match (value.as_data(), step) {
        (data!(WordLike::LerfuWord { base, .. }), WordLikeStep::LerfuBase) => Some(base),
        (data!(WordLike::ZeiCompound { left, .. }), WordLikeStep::ZeiLeft) => Some(left),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_some() == word_like_path_resolves(root, steps))]
fn project_word_like<'a>(root: &'a WordLike, steps: &[WordLikeStep]) -> Option<&'a WordLike> {
    let mut current = root;
    for step in steps {
        current = word_like_child(current, *step)?;
    }
    Some(current)
}

#[requires(true)]
#[ensures(steps.is_empty() -> ret)]
fn word_like_path_resolves(root: &WordLike, steps: &[WordLikeStep]) -> bool {
    let mut current = root;
    for step in steps {
        let Some(child) = word_like_child(current, *step) else {
            return false;
        };
        current = child;
    }
    true
}

#[requires(true)]
#[ensures(
    ret
        == matches!(
            (value.as_data(), step),
            (data!(WordLike::LerfuWord { .. }), WordLikeStep::LerfuBase)
                | (data!(WordLike::ZeiCompound { .. }), WordLikeStep::ZeiLeft)
        )
)]
fn word_like_step_resolves(value: &WordLike, step: WordLikeStep) -> bool {
    matches!(
        (value.as_data(), step),
        (data!(WordLike::LerfuWord { .. }), WordLikeStep::LerfuBase)
            | (data!(WordLike::ZeiCompound { .. }), WordLikeStep::ZeiLeft)
    )
}

#[invariant(::Plain => true)]
#[invariant(::QuotedMarker => true)]
#[invariant(::QuotedWord => true)]
#[invariant(::SelmahoMarker => true)]
#[invariant(::SelmahoWord => true)]
#[invariant(::ZoiMarker => true)]
#[invariant(::ZoiOpeningDelimiter => true)]
#[invariant(::ZoiClosingDelimiter => true)]
#[invariant(::LohuMarker => true)]
#[invariant(::QuotedWordsWord { .. } => true)]
#[invariant(::LehuMarker => true)]
#[invariant(::DelimitedWordMarker => true)]
#[invariant(::BuSuffix => true)]
#[invariant(::ZeiLink => true)]
#[invariant(::ZeiRight => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WordSlot {
    Plain,
    QuotedMarker,
    QuotedWord,
    SelmahoMarker,
    SelmahoWord,
    ZoiMarker,
    ZoiOpeningDelimiter,
    ZoiClosingDelimiter,
    LohuMarker,
    QuotedWordsWord { index: usize },
    LehuMarker,
    DelimitedWordMarker,
    BuSuffix,
    ZeiLink,
    ZeiRight,
}

/// Typed locator for a `Word` stored directly in one indicator layer.
#[invariant(::EmphasisBahe => true)]
#[invariant(::ExtraEmphasisBahe { .. } => true, "the enclosing handle validates the index")]
#[invariant(::IndicatorBahe { .. } => true, "the enclosing handle validates the index")]
#[invariant(::Indicator => true)]
#[invariant(::NaiBahe { .. } => true, "the enclosing handle validates the index")]
#[invariant(::Nai => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WithIndicatorsWordSlot {
    EmphasisBahe,
    ExtraEmphasisBahe { index: usize },
    IndicatorBahe { index: usize },
    Indicator,
    NaiBahe { index: usize },
    Nai,
}

#[invariant(::WordLike { node, slot } => word_slot_resolves(node.get(), *slot))]
#[invariant(::Indicators { node, slot } => with_indicators_word_slot_resolves(node.get(), *slot))]
#[invariant(::Compiled { .. } => true, "compiled-word validity is carried by its typed handle")]
#[derive(Debug, Clone)]
enum WordHandleStorage {
    WordLike {
        node: WordLikeHandle,
        slot: WordSlot,
    },
    Indicators {
        node: WithIndicatorsHandle,
        slot: WithIndicatorsWordSlot,
    },
    Compiled {
        handle: CompiledWordHandle,
    },
}

#[invariant(true, "private typed storage enforces every word locator")]
#[derive(Debug, Clone)]
pub(crate) struct WordHandle {
    value: WordHandleStorage,
}

impl WordHandle {
    #[requires(true)]
    #[expensive_ensures(ret.get() == &old(word.clone()))]
    pub(crate) fn from_owned(word: Word) -> Self {
        WordHandle {
            value: new!(WordHandleStorage::WordLike {
                node: WordLikeHandle::root(WordLike::bare(word)),
                slot: WordSlot::Plain,
            }),
        }
    }

    #[requires(word_slot_resolves(node.get(), slot))]
    #[ensures(true)]
    fn new(node: WordLikeHandle, slot: WordSlot) -> Self {
        WordHandle {
            value: new!(WordHandleStorage::WordLike { node, slot }),
        }
    }

    /// Locate a word stored directly in an indicator layer without cloning it.
    #[requires(true)]
    #[ensures(
        ret.is_some()
            == with_indicators_word_slot_resolves(old(node.clone()).get(), slot)
    )]
    pub(crate) fn from_indicators(
        node: WithIndicatorsHandle,
        slot: WithIndicatorsWordSlot,
    ) -> Option<Self> {
        if !with_indicators_word_slot_resolves(node.get(), slot) {
            return None;
        }
        Some(WordHandle {
            value: new!(WordHandleStorage::Indicators { node, slot }),
        })
    }

    #[requires(true)]
    #[ensures(ret.get() == &old(handle.clone()).get().word)]
    fn from_compiled(handle: CompiledWordHandle) -> Self {
        WordHandle {
            value: new!(WordHandleStorage::Compiled { handle }),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn get(&self) -> &Word {
        match self.value.as_data() {
            data!(WordHandleStorage::WordLike { node, slot }) => {
                project_word(node.get(), *slot).expect("word handle is valid by construction")
            }
            data!(WordHandleStorage::Indicators { node, slot }) => {
                project_with_indicators_word(node.get(), *slot)
                    .expect("indicator-word handle is valid by construction")
            }
            data!(WordHandleStorage::Compiled { handle }) => &handle.get().word,
        }
    }

    /// Return the exact owning token when this word originated in syntax.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn root_token(&self) -> Option<&TokenHandle> {
        match self.value.as_data() {
            data!(WordHandleStorage::WordLike { node, .. }) => node.root_token(),
            data!(WordHandleStorage::Indicators { node, .. }) => node.root_token(),
            data!(WordHandleStorage::Compiled { .. }) => None,
        }
    }

    #[requires(true)]
    #[ensures(ret == self.get().clone())]
    pub(crate) fn clone_rust(&self) -> Word {
        self.get().clone()
    }
}

impl PartialEq for WordHandle {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for WordHandle {}

#[requires(true)]
#[ensures(
    ret.is_some()
        == match (value.as_data(), slot) {
            (data!(WordLike::PlainWord(_)), WordSlot::Plain)
            | (data!(WordLike::QuotedWord { .. }), WordSlot::QuotedMarker | WordSlot::QuotedWord)
            | (
                data!(WordLike::SelmahoQuotedWord { .. }),
                WordSlot::SelmahoMarker | WordSlot::SelmahoWord
            )
            | (
                data!(WordLike::DelimitedNonLojbanQuote { .. }),
                WordSlot::ZoiMarker
                    | WordSlot::ZoiOpeningDelimiter
                    | WordSlot::ZoiClosingDelimiter
            )
            | (data!(WordLike::QuotedWords { .. }), WordSlot::LohuMarker | WordSlot::LehuMarker)
            | (data!(WordLike::DelimitedWordQuote { .. }), WordSlot::DelimitedWordMarker)
            | (data!(WordLike::LerfuWord { .. }), WordSlot::BuSuffix)
            | (data!(WordLike::ZeiCompound { .. }), WordSlot::ZeiLink | WordSlot::ZeiRight) => true,
            (
                data!(WordLike::QuotedWords { quoted_words, .. }),
                WordSlot::QuotedWordsWord { index },
            ) => index < quoted_words.len(),
            _ => false,
        }
)]
fn word_at_slot(value: &WordLike, slot: WordSlot) -> Option<&Word> {
    match (value.as_data(), slot) {
        (data!(WordLike::PlainWord(word)), WordSlot::Plain) => Some(word),
        (data!(WordLike::QuotedWord { zo, .. }), WordSlot::QuotedMarker) => Some(zo),
        (data!(WordLike::QuotedWord { word, .. }), WordSlot::QuotedWord) => Some(word),
        (data!(WordLike::SelmahoQuotedWord { mahoi, .. }), WordSlot::SelmahoMarker) => Some(mahoi),
        (data!(WordLike::SelmahoQuotedWord { word, .. }), WordSlot::SelmahoWord) => Some(word),
        (data!(WordLike::DelimitedNonLojbanQuote { zoi, .. }), WordSlot::ZoiMarker) => Some(zoi),
        (
            data!(WordLike::DelimitedNonLojbanQuote {
                opening_delimiter,
                ..
            }),
            WordSlot::ZoiOpeningDelimiter,
        ) => Some(opening_delimiter),
        (
            data!(WordLike::DelimitedNonLojbanQuote {
                closing_delimiter,
                ..
            }),
            WordSlot::ZoiClosingDelimiter,
        ) => Some(closing_delimiter),
        (data!(WordLike::QuotedWords { lohu, .. }), WordSlot::LohuMarker) => Some(lohu),
        (
            data!(WordLike::QuotedWords { quoted_words, .. }),
            WordSlot::QuotedWordsWord { index },
        ) => quoted_words.get(index),
        (data!(WordLike::QuotedWords { lehu, .. }), WordSlot::LehuMarker) => Some(lehu),
        (data!(WordLike::DelimitedWordQuote { marker, .. }), WordSlot::DelimitedWordMarker) => {
            Some(marker)
        }
        (data!(WordLike::LerfuWord { bu, .. }), WordSlot::BuSuffix) => Some(bu),
        (data!(WordLike::ZeiCompound { zei, .. }), WordSlot::ZeiLink) => Some(zei),
        (data!(WordLike::ZeiCompound { right, .. }), WordSlot::ZeiRight) => Some(right),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_some() == word_slot_resolves(value, slot))]
fn project_word(value: &WordLike, slot: WordSlot) -> Option<&Word> {
    word_at_slot(value, slot)
}

#[requires(true)]
#[ensures(ret == word_at_slot(value, slot).is_some())]
fn word_slot_resolves(value: &WordLike, slot: WordSlot) -> bool {
    word_at_slot(value, slot).is_some()
}

#[requires(true)]
#[ensures(
    ret.is_some()
        == match (value.as_data(), slot) {
            (data!(WithIndicators::Emphasized { .. }), WithIndicatorsWordSlot::EmphasisBahe) => true,
            (
                data!(WithIndicators::Emphasized { extra_bahe, .. }),
                WithIndicatorsWordSlot::ExtraEmphasisBahe { index },
            ) => index < extra_bahe.len(),
            (
                data!(WithIndicators::WithIndicator { indicator_bahe, .. }),
                WithIndicatorsWordSlot::IndicatorBahe { index },
            ) => index < indicator_bahe.len(),
            (
                data!(WithIndicators::WithIndicator { .. }),
                WithIndicatorsWordSlot::Indicator,
            ) => true,
            (
                data!(WithIndicators::WithIndicator { nai_bahe, .. }),
                WithIndicatorsWordSlot::NaiBahe { index },
            ) => index < nai_bahe.len(),
            (
                data!(WithIndicators::WithIndicator { nai, .. }),
                WithIndicatorsWordSlot::Nai,
            ) => nai.is_some(),
            _ => false,
        }
)]
fn with_indicators_word_at_slot(
    value: &WithIndicators<WordLike>,
    slot: WithIndicatorsWordSlot,
) -> Option<&Word> {
    match (value.as_data(), slot) {
        (data!(WithIndicators::Emphasized { bahe, .. }), WithIndicatorsWordSlot::EmphasisBahe) => {
            Some(bahe)
        }
        (
            data!(WithIndicators::Emphasized { extra_bahe, .. }),
            WithIndicatorsWordSlot::ExtraEmphasisBahe { index },
        ) => extra_bahe.get(index),
        (
            data!(WithIndicators::WithIndicator { indicator_bahe, .. }),
            WithIndicatorsWordSlot::IndicatorBahe { index },
        ) => indicator_bahe.get(index),
        (
            data!(WithIndicators::WithIndicator { indicator, .. }),
            WithIndicatorsWordSlot::Indicator,
        ) => Some(indicator),
        (
            data!(WithIndicators::WithIndicator { nai_bahe, .. }),
            WithIndicatorsWordSlot::NaiBahe { index },
        ) => nai_bahe.get(index),
        (data!(WithIndicators::WithIndicator { nai, .. }), WithIndicatorsWordSlot::Nai) => {
            nai.as_ref()
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_some() == with_indicators_word_slot_resolves(value, slot))]
fn project_with_indicators_word(
    value: &WithIndicators<WordLike>,
    slot: WithIndicatorsWordSlot,
) -> Option<&Word> {
    with_indicators_word_at_slot(value, slot)
}

#[requires(true)]
#[ensures(ret == with_indicators_word_at_slot(value, slot).is_some())]
fn with_indicators_word_slot_resolves(
    value: &WithIndicators<WordLike>,
    slot: WithIndicatorsWordSlot,
) -> bool {
    with_indicators_word_at_slot(value, slot).is_some()
}

#[invariant(*index < word.get().lujvo_parts().map_or(0, |parts| parts.len()))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct LocatedLujvoPart {
    word: WordHandle,
    index: usize,
}

impl LocatedLujvoPart {
    #[requires(index < word.get().lujvo_parts().map_or(0, |parts| parts.len()))]
    #[ensures(ret.index == index)]
    fn new(word: WordHandle, index: usize) -> Self {
        new!(LocatedLujvoPart { word, index })
    }

    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &LujvoPart {
        &self
            .word
            .get()
            .lujvo_parts()
            .expect("located lujvo part belongs to a lujvo")[self.index]
    }
}

#[invariant(::Owned { .. } => true)]
#[invariant(::Located { .. } => true)]
#[derive(Debug, Clone)]
enum LujvoPartStorage {
    Owned { value: Arc<LujvoPart> },
    Located { handle: LocatedLujvoPart },
}

impl PartialEq for LujvoPartStorage {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for LujvoPartStorage {}

impl LujvoPartStorage {
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &LujvoPart {
        match self {
            LujvoPartStorage::Owned { value } => value.as_ref(),
            LujvoPartStorage::Located { handle } => handle.get(),
        }
    }

    #[requires(true)]
    #[ensures(ret == self.get().clone())]
    fn clone_rust(&self) -> LujvoPart {
        self.get().clone()
    }
}

/// Rafsi component of a parsed lujvo.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "LujvoRafsi",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyLujvoRafsi {
    value: LujvoPartStorage,
}

#[pymethods]
impl PyLujvoRafsi {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("phonemes",);

    /// Construct a rafsi lujvo part from canonical phonemes.
    #[requires(true)]
    #[ensures(true)]
    #[new]
    fn new(phonemes: PyRef<'_, PyPhonemes>) -> Self {
        PyLujvoRafsi {
            value: LujvoPartStorage::Owned {
                value: Arc::new(LujvoPart::rafsi(phonemes.clone_rust())),
            },
        }
    }

    /// Return the rafsi phonemes.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn phonemes(&self) -> PyPhonemes {
        PyPhonemes::from_rust(self.value.get().phonemes().clone())
    }
}

/// Hyphen component of a parsed lujvo.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "LujvoHyphen",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyLujvoHyphen {
    value: LujvoPartStorage,
}

#[pymethods]
impl PyLujvoHyphen {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("phonemes",);

    /// Construct a hyphen lujvo part from canonical phonemes.
    #[requires(true)]
    #[ensures(true)]
    #[new]
    fn new(phonemes: PyRef<'_, PyPhonemes>) -> Self {
        PyLujvoHyphen {
            value: LujvoPartStorage::Owned {
                value: Arc::new(LujvoPart::hyphen(phonemes.clone_rust())),
            },
        }
    }

    /// Return the hyphen phonemes.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn phonemes(&self) -> PyPhonemes {
        PyPhonemes::from_rust(self.value.get().phonemes().clone())
    }
}

#[requires(true)]
#[ensures(true)]
fn lujvo_part_from_python(value: &Bound<'_, PyAny>) -> PyResult<LujvoPart> {
    if let Ok(value) = value.extract::<PyRef<'_, PyLujvoRafsi>>() {
        return Ok(value.value.clone_rust());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyLujvoHyphen>>() {
        return Ok(value.value.clone_rust());
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "expected jbotci.morphology.LujvoRafsi or LujvoHyphen",
    ))
}

#[requires(true)]
#[ensures(true)]
fn lujvo_part_to_python(py: Python<'_>, handle: LocatedLujvoPart) -> PyResult<Py<PyAny>> {
    let value = LujvoPartStorage::Located { handle };
    match value.get() {
        LujvoPart::Rafsi(_) => Ok(Py::new(py, PyLujvoRafsi { value })?.into_any()),
        LujvoPart::Hyphen(_) => Ok(Py::new(py, PyLujvoHyphen { value })?.into_any()),
    }
}

#[invariant(::ZoiQuotedText => true)]
#[invariant(::DelimitedWordQuotedText => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VerbatimSlot {
    ZoiQuotedText,
    DelimitedWordQuotedText,
}

#[invariant(verbatim_slot_resolves(node.get(), *slot))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct LocatedVerbatim {
    node: WordLikeHandle,
    slot: VerbatimSlot,
}

impl LocatedVerbatim {
    #[requires(verbatim_slot_resolves(node.get(), slot))]
    #[ensures(true)]
    fn new(node: WordLikeHandle, slot: VerbatimSlot) -> Self {
        new!(LocatedVerbatim { node, slot })
    }

    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &Verbatim {
        project_verbatim(self.node.get(), self.slot)
            .expect("verbatim handle is valid by construction")
    }
}

#[requires(true)]
#[ensures(
    ret.is_some()
        == matches!(
            (value.as_data(), slot),
            (
                data!(WordLike::DelimitedNonLojbanQuote { .. }),
                VerbatimSlot::ZoiQuotedText,
            ) | (
                data!(WordLike::DelimitedWordQuote { .. }),
                VerbatimSlot::DelimitedWordQuotedText,
            )
        )
)]
fn verbatim_at_slot(value: &WordLike, slot: VerbatimSlot) -> Option<&Verbatim> {
    match (value.as_data(), slot) {
        (
            data!(WordLike::DelimitedNonLojbanQuote { quoted_text, .. }),
            VerbatimSlot::ZoiQuotedText,
        )
        | (
            data!(WordLike::DelimitedWordQuote { quoted_text, .. }),
            VerbatimSlot::DelimitedWordQuotedText,
        ) => Some(quoted_text),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_some() == verbatim_slot_resolves(value, slot))]
fn project_verbatim(value: &WordLike, slot: VerbatimSlot) -> Option<&Verbatim> {
    verbatim_at_slot(value, slot)
}

#[requires(true)]
#[ensures(ret == verbatim_at_slot(value, slot).is_some())]
fn verbatim_slot_resolves(value: &WordLike, slot: VerbatimSlot) -> bool {
    verbatim_at_slot(value, slot).is_some()
}

#[invariant(::Owned { value } => value.span.char_len() == value.text.chars().count())]
#[invariant(::Located { .. } => true)]
#[derive(Debug, Clone)]
enum VerbatimStorage {
    Owned { value: Arc<Verbatim> },
    Located { handle: LocatedVerbatim },
}

impl PartialEq for VerbatimStorage {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for VerbatimStorage {}

impl VerbatimStorage {
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &Verbatim {
        match self.as_data() {
            data!(VerbatimStorage::Owned { value }) => value.as_ref(),
            data!(VerbatimStorage::Located { handle }) => handle.get(),
        }
    }

    #[requires(true)]
    #[ensures(ret == self.get().clone())]
    fn clone_rust(&self) -> Verbatim {
        self.get().clone()
    }
}

/// Exact verbatim source text paired with its source span.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "Verbatim",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyVerbatim {
    value: VerbatimStorage,
}

impl PyVerbatim {
    #[requires(true)]
    #[ensures(true)]
    fn located(handle: LocatedVerbatim) -> Self {
        PyVerbatim {
            value: new!(VerbatimStorage::Located { handle }),
        }
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    pub(crate) fn rust(&self) -> &Verbatim {
        self.value.get()
    }
}

#[pymethods]
impl PyVerbatim {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("span", "text");

    /// Construct exact verbatim text whose scalar length matches its span.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(span: PyRef<'_, PySourceSpan>, text: String) -> PyResult<Self> {
        if span.rust().char_len() != text.chars().count() {
            return Err(InvalidInputError::new_err(
                "verbatim text character count must equal span.char_length",
            ));
        }
        Ok(PyVerbatim {
            value: new!(VerbatimStorage::Owned {
                value: Arc::new(Verbatim::new(span.clone_rust(), text)),
            }),
        })
    }

    /// Return the verbatim source span.
    #[requires(true)]
    #[ensures(ret.rust().char_start == self.value.get().span.char_start)]
    #[getter]
    fn span(&self) -> PySourceSpan {
        PySourceSpan::from_rust(self.value.get().span.as_ref().clone())
    }

    /// Return the exact verbatim source text.
    #[requires(true)]
    #[ensures(ret == self.value.get().text.as_str())]
    #[getter]
    fn text(&self) -> &str {
        &self.value.get().text
    }
}

#[requires(span.rust().char_len() > 0)]
#[ensures(ret.get().kind() == kind)]
fn plain_word(kind: WordKind, phonemes: &PyPhonemes, span: &PySourceSpan) -> WordHandle {
    WordHandle::from_owned(Word::from_kind(
        kind,
        phonemes.clone_rust(),
        span.clone_rust(),
    ))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_nonempty_word_span(span: &PySourceSpan) -> PyResult<()> {
    if span.rust().char_len() == 0 {
        Err(InvalidInputError::new_err(
            "word source span must contain at least one character",
        ))
    } else {
        Ok(())
    }
}

#[requires(true)]
#[ensures(true)]
fn word_kind_to_python(py: Python<'_>, handle: &WordHandle) -> PyResult<Py<PyAny>> {
    enum_to_python(py, handle.get().kind())
}

#[requires(true)]
#[ensures(true)]
fn word_phonemes(handle: &WordHandle) -> PyPhonemes {
    PyPhonemes::from_rust(handle.get().phonemes())
}

#[requires(true)]
#[ensures(true)]
fn word_span(handle: &WordHandle) -> PySourceSpan {
    PySourceSpan::from_rust(handle.get().span().clone())
}

#[requires(true)]
#[ensures(true)]
fn word_key(handle: &WordHandle) -> PyWordKey {
    PyWordKey::from_rust(handle.get().key())
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn word_canonical_phonemes(handle: &WordHandle) -> String {
    handle.get().canonical_phonemes()
}

#[requires(true)]
#[ensures(true)]
fn word_cmavo(py: Python<'_>, handle: &WordHandle) -> PyResult<Option<Py<PyAny>>> {
    handle
        .get()
        .cmavo()
        .map(|value| enum_to_python(py, value))
        .transpose()
}

#[requires(true)]
#[ensures(true)]
fn word_selmaho(py: Python<'_>, handle: &WordHandle) -> PyResult<Option<Py<PyAny>>> {
    handle
        .get()
        .selmaho_kind()
        .map(|value| enum_to_python(py, value))
        .transpose()
}

#[requires(true)]
#[ensures(true)]
fn word_is_cmavo(py: Python<'_>, handle: &WordHandle, cmavo: &Bound<'_, PyAny>) -> PyResult<bool> {
    Ok(handle.get().is_cmavo(enum_from_python(py, cmavo)?))
}

#[requires(true)]
#[ensures(true)]
fn word_is_selmaho(
    py: Python<'_>,
    handle: &WordHandle,
    selmaho: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    Ok(handle.get().is_selmaho(enum_from_python(py, selmaho)?))
}

/// Parsed cmavo word with canonical phonemes and provenance.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "CmavoWord",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyCmavoWord {
    handle: WordHandle,
}

#[pymethods]
impl PyCmavoWord {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("phonemes", "span");
    /// Construct a cmavo word with canonical phonemes and a non-empty span.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(phonemes: PyRef<'_, PyPhonemes>, span: PyRef<'_, PySourceSpan>) -> PyResult<Self> {
        validate_nonempty_word_span(&span)?;
        Ok(PyCmavoWord {
            handle: plain_word(WordKind::Cmavo, &phonemes, &span),
        })
    }
    /// Return the cmavo word kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_kind_to_python(py, &self.handle)
    }
    /// Return the canonical word phonemes.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn phonemes(&self) -> PyPhonemes {
        word_phonemes(&self.handle)
    }
    /// Return the source span.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn span(&self) -> PySourceSpan {
        word_span(&self.handle)
    }
    /// Return the syntax identity key.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn key(&self) -> PyWordKey {
        word_key(&self.handle)
    }
    /// Return canonical phoneme text.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn canonical_phonemes(&self) -> String {
        word_canonical_phonemes(&self.handle)
    }
    /// Return the exact cmavo identity when recognized.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn cmavo(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        word_cmavo(py, &self.handle)
    }
    /// Return the primary selma'o when defined.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn selmaho(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        word_selmaho(py, &self.handle)
    }
    /// Test exact cmavo identity.
    #[requires(true)]
    #[ensures(true)]
    fn is_cmavo(&self, py: Python<'_>, cmavo: &Bound<'_, PyAny>) -> PyResult<bool> {
        word_is_cmavo(py, &self.handle, cmavo)
    }
    /// Test selma'o membership.
    #[requires(true)]
    #[ensures(true)]
    fn is_selmaho(&self, py: Python<'_>, selmaho: &Bound<'_, PyAny>) -> PyResult<bool> {
        word_is_selmaho(py, &self.handle, selmaho)
    }
}

/// Parsed gismu word with canonical phonemes and provenance.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "GismuWord",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyGismuWord {
    handle: WordHandle,
}

#[pymethods]
impl PyGismuWord {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("phonemes", "span");
    /// Construct a gismu word with canonical phonemes and a non-empty span.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(phonemes: PyRef<'_, PyPhonemes>, span: PyRef<'_, PySourceSpan>) -> PyResult<Self> {
        validate_nonempty_word_span(&span)?;
        Ok(PyGismuWord {
            handle: plain_word(WordKind::Gismu, &phonemes, &span),
        })
    }
    /// Return the word kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_kind_to_python(py, &self.handle)
    }
    /// Return the canonical word phonemes.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn phonemes(&self) -> PyPhonemes {
        word_phonemes(&self.handle)
    }
    /// Return the source span.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn span(&self) -> PySourceSpan {
        word_span(&self.handle)
    }
    /// Return the syntax identity key.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn key(&self) -> PyWordKey {
        word_key(&self.handle)
    }
    /// Return canonical phoneme text.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn canonical_phonemes(&self) -> String {
        word_canonical_phonemes(&self.handle)
    }
    /// Return the exact cmavo identity when recognized.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn cmavo(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        word_cmavo(py, &self.handle)
    }
    /// Return the primary selma'o when defined.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn selmaho(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        word_selmaho(py, &self.handle)
    }
    /// Test exact cmavo identity.
    #[requires(true)]
    #[ensures(true)]
    fn is_cmavo(&self, py: Python<'_>, cmavo: &Bound<'_, PyAny>) -> PyResult<bool> {
        word_is_cmavo(py, &self.handle, cmavo)
    }
    /// Test selma'o membership.
    #[requires(true)]
    #[ensures(true)]
    fn is_selmaho(&self, py: Python<'_>, selmaho: &Bound<'_, PyAny>) -> PyResult<bool> {
        word_is_selmaho(py, &self.handle, selmaho)
    }
}

/// Parsed fu'ivla word with canonical phonemes and provenance.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "FuhivlaWord",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyFuhivlaWord {
    handle: WordHandle,
}

#[pymethods]
impl PyFuhivlaWord {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("phonemes", "span");
    /// Construct a fu'ivla word with canonical phonemes and a non-empty span.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(phonemes: PyRef<'_, PyPhonemes>, span: PyRef<'_, PySourceSpan>) -> PyResult<Self> {
        validate_nonempty_word_span(&span)?;
        Ok(PyFuhivlaWord {
            handle: plain_word(WordKind::Fuhivla, &phonemes, &span),
        })
    }
    /// Return the word kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_kind_to_python(py, &self.handle)
    }
    /// Return the canonical word phonemes.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn phonemes(&self) -> PyPhonemes {
        word_phonemes(&self.handle)
    }
    /// Return the source span.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn span(&self) -> PySourceSpan {
        word_span(&self.handle)
    }
    /// Return the syntax identity key.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn key(&self) -> PyWordKey {
        word_key(&self.handle)
    }
    /// Return canonical phoneme text.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn canonical_phonemes(&self) -> String {
        word_canonical_phonemes(&self.handle)
    }
    /// Return the exact cmavo identity when recognized.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn cmavo(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        word_cmavo(py, &self.handle)
    }
    /// Return the primary selma'o when defined.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn selmaho(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        word_selmaho(py, &self.handle)
    }
    /// Test exact cmavo identity.
    #[requires(true)]
    #[ensures(true)]
    fn is_cmavo(&self, py: Python<'_>, cmavo: &Bound<'_, PyAny>) -> PyResult<bool> {
        word_is_cmavo(py, &self.handle, cmavo)
    }
    /// Test selma'o membership.
    #[requires(true)]
    #[ensures(true)]
    fn is_selmaho(&self, py: Python<'_>, selmaho: &Bound<'_, PyAny>) -> PyResult<bool> {
        word_is_selmaho(py, &self.handle, selmaho)
    }
}

/// Parsed cmevla word with canonical phonemes and provenance.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "CmevlaWord",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyCmevlaWord {
    handle: WordHandle,
}

#[pymethods]
impl PyCmevlaWord {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("phonemes", "span");
    /// Construct a cmevla word with canonical phonemes and a non-empty span.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(phonemes: PyRef<'_, PyPhonemes>, span: PyRef<'_, PySourceSpan>) -> PyResult<Self> {
        validate_nonempty_word_span(&span)?;
        Ok(PyCmevlaWord {
            handle: plain_word(WordKind::Cmevla, &phonemes, &span),
        })
    }
    /// Return the word kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_kind_to_python(py, &self.handle)
    }
    /// Return the canonical word phonemes.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn phonemes(&self) -> PyPhonemes {
        word_phonemes(&self.handle)
    }
    /// Return the source span.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn span(&self) -> PySourceSpan {
        word_span(&self.handle)
    }
    /// Return the syntax identity key.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn key(&self) -> PyWordKey {
        word_key(&self.handle)
    }
    /// Return canonical phoneme text.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn canonical_phonemes(&self) -> String {
        word_canonical_phonemes(&self.handle)
    }
    /// Return the exact cmavo identity when recognized.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn cmavo(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        word_cmavo(py, &self.handle)
    }
    /// Return the primary selma'o when defined.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn selmaho(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        word_selmaho(py, &self.handle)
    }
    /// Test exact cmavo identity.
    #[requires(true)]
    #[ensures(true)]
    fn is_cmavo(&self, py: Python<'_>, cmavo: &Bound<'_, PyAny>) -> PyResult<bool> {
        word_is_cmavo(py, &self.handle, cmavo)
    }
    /// Test selma'o membership.
    #[requires(true)]
    #[ensures(true)]
    fn is_selmaho(&self, py: Python<'_>, selmaho: &Bound<'_, PyAny>) -> PyResult<bool> {
        word_is_selmaho(py, &self.handle, selmaho)
    }
}

/// Parsed lujvo word retaining its typed component sequence.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "LujvoWord",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyLujvoWord {
    handle: WordHandle,
}

#[pymethods]
impl PyLujvoWord {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("parts", "span");
    /// Construct a lujvo word from a non-empty typed part sequence and source span.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(parts: &Bound<'_, PyAny>, span: PyRef<'_, PySourceSpan>) -> PyResult<Self> {
        validate_nonempty_word_span(&span)?;
        let parts = extract_sequence(parts, "parts", lujvo_part_from_python)?;
        let parts = vec1::Vec1::try_from_vec(parts)
            .map_err(|_| InvalidInputError::new_err("lujvo parts must not be empty"))?;
        Ok(PyLujvoWord {
            handle: WordHandle::from_owned(Word::lujvo(parts, span.clone_rust())),
        })
    }
    /// Return the word kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_kind_to_python(py, &self.handle)
    }
    /// Return the canonical combined phonemes.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn phonemes(&self) -> PyPhonemes {
        word_phonemes(&self.handle)
    }
    /// Return the source span.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn span(&self) -> PySourceSpan {
        word_span(&self.handle)
    }
    /// Return the syntax identity key.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn key(&self) -> PyWordKey {
        word_key(&self.handle)
    }
    /// Return canonical phoneme text.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn canonical_phonemes(&self) -> String {
        word_canonical_phonemes(&self.handle)
    }
    /// Return the exact cmavo identity when recognized.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn cmavo(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        word_cmavo(py, &self.handle)
    }
    /// Return the primary selma'o when defined.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn selmaho(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        word_selmaho(py, &self.handle)
    }
    /// Test exact cmavo identity.
    #[requires(true)]
    #[ensures(true)]
    fn is_cmavo(&self, py: Python<'_>, cmavo: &Bound<'_, PyAny>) -> PyResult<bool> {
        word_is_cmavo(py, &self.handle, cmavo)
    }
    /// Test selma'o membership.
    #[requires(true)]
    #[ensures(true)]
    fn is_selmaho(&self, py: Python<'_>, selmaho: &Bound<'_, PyAny>) -> PyResult<bool> {
        word_is_selmaho(py, &self.handle, selmaho)
    }
    /// Return the immutable typed rafsi and hyphen part sequence.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn parts(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        let count = self
            .handle
            .get()
            .lujvo_parts()
            .expect("private construction and projection fix the word kind")
            .len();
        let values = (0..count)
            .map(|index| {
                lujvo_part_to_python(py, LocatedLujvoPart::new(self.handle.clone(), index))
            })
            .collect::<PyResult<Vec<_>>>()?;
        crate::support::sequence_to_tuple(py, values).map(Bound::unbind)
    }
}

/// Extract the canonical Rust-backed handle from any public Python `Word` variant.
#[requires(true)]
#[ensures(true)]
pub(crate) fn word_handle_from_python(value: &Bound<'_, PyAny>) -> PyResult<WordHandle> {
    if let Ok(value) = value.extract::<PyRef<'_, PyCmavoWord>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyGismuWord>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyLujvoWord>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyFuhivlaWord>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyCmevlaWord>>() {
        return Ok(value.handle.clone());
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "expected a jbotci.morphology Word variant",
    ))
}

/// Project a located Rust `Word` through the one public Python class family.
#[requires(true)]
#[ensures(true)]
pub(crate) fn word_to_python(py: Python<'_>, handle: WordHandle) -> PyResult<Py<PyAny>> {
    match handle.get().kind() {
        WordKind::Cmavo => Ok(Py::new(py, PyCmavoWord { handle })?.into_any()),
        WordKind::Gismu => Ok(Py::new(py, PyGismuWord { handle })?.into_any()),
        WordKind::Lujvo => Ok(Py::new(py, PyLujvoWord { handle })?.into_any()),
        WordKind::Fuhivla => Ok(Py::new(py, PyFuhivlaWord { handle })?.into_any()),
        WordKind::Cmevla => Ok(Py::new(py, PyCmevlaWord { handle })?.into_any()),
    }
}

#[requires(true)]
#[ensures(true)]
fn word_like_byte_range(handle: &WordLikeHandle) -> Option<(usize, usize)> {
    handle
        .get()
        .byte_range()
        .map(|range| (range.start, range.end))
}

#[requires(true)]
#[ensures(true)]
fn word_like_source_spans(
    py: Python<'_>,
    handle: &WordLikeHandle,
) -> PyResult<Py<pyo3::types::PyTuple>> {
    let values = handle
        .get()
        .source_spans()
        .into_iter()
        .cloned()
        .map(PySourceSpan::from_rust);
    crate::support::sequence_to_tuple(py, values).map(Bound::unbind)
}

#[requires(true)]
#[ensures(true)]
fn word_like_cmavo(py: Python<'_>, handle: &WordLikeHandle) -> PyResult<Option<Py<PyAny>>> {
    handle
        .get()
        .cmavo()
        .map(|value| enum_to_python(py, value))
        .transpose()
}

#[requires(true)]
#[ensures(true)]
fn word_like_is_cmavo(
    py: Python<'_>,
    handle: &WordLikeHandle,
    cmavo: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    Ok(handle.get().is_cmavo(enum_from_python(py, cmavo)?))
}

#[requires(true)]
#[ensures(true)]
fn word_like_is_selmaho(
    py: Python<'_>,
    handle: &WordLikeHandle,
    selmaho: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    Ok(handle.get().is_selmaho(enum_from_python(py, selmaho)?))
}

#[requires(true)]
#[ensures(true)]
fn word_like_str(handle: &WordLikeHandle) -> String {
    handle.get().to_string()
}

/// Plain unquoted morphology word.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "PlainWord",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyPlainWord {
    handle: WordLikeHandle,
}

#[pymethods]
impl PyPlainWord {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("word",);
    /// Construct a plain word-like variant from a parsed word.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(word: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(PyPlainWord {
            handle: WordLikeHandle::root(WordLike::bare(
                word_handle_from_python(word)?.clone_rust(),
            )),
        })
    }
    /// Return the contained parsed word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn word(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_to_python(py, WordHandle::new(self.handle.clone(), WordSlot::Plain))
    }
    /// Return the combined half-open byte range when available.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn byte_range(&self) -> Option<(usize, usize)> {
        word_like_byte_range(&self.handle)
    }
    /// Return every contributing source span in source order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn source_spans(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        word_like_source_spans(py, &self.handle)
    }
    /// Return the exact cmavo identity when this is a cmavo.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn cmavo(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        word_like_cmavo(py, &self.handle)
    }
    /// Test exact cmavo identity.
    #[requires(true)]
    #[ensures(true)]
    fn is_cmavo(&self, py: Python<'_>, cmavo: &Bound<'_, PyAny>) -> PyResult<bool> {
        word_like_is_cmavo(py, &self.handle, cmavo)
    }
    /// Test selma'o membership.
    #[requires(true)]
    #[ensures(true)]
    fn is_selmaho(&self, py: Python<'_>, selmaho: &Bound<'_, PyAny>) -> PyResult<bool> {
        word_like_is_selmaho(py, &self.handle, selmaho)
    }
    /// Report whether the contained word is a brivla.
    #[requires(true)]
    #[ensures(ret == self.handle.get().is_brivla())]
    fn is_brivla(&self) -> bool {
        self.handle.get().is_brivla()
    }
    /// Report whether the contained word is a cmevla.
    #[requires(true)]
    #[ensures(ret == self.handle.get().is_cmevla())]
    fn is_cmevla(&self) -> bool {
        self.handle.get().is_cmevla()
    }
    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        word_like_str(&self.handle)
    }
}

/// `zo` single-word quotation with marker and quoted word.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "QuotedWord",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyQuotedWord {
    handle: WordLikeHandle,
}

#[pymethods]
impl PyQuotedWord {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("zo", "word");
    /// Construct a validated `zo` quotation.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(zo: &Bound<'_, PyAny>, word: &Bound<'_, PyAny>) -> PyResult<Self> {
        let zo = word_handle_from_python(zo)?.clone_rust();
        if !zo.is_cmavo(Cmavo::Zo) {
            return Err(InvalidInputError::new_err(
                "QuotedWord.zo must be the cmavo zo",
            ));
        }
        Ok(PyQuotedWord {
            handle: WordLikeHandle::root(WordLike::zo_quote(
                zo,
                word_handle_from_python(word)?.clone_rust(),
            )),
        })
    }
    /// Return the `zo` marker word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn zo(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_to_python(
            py,
            WordHandle::new(self.handle.clone(), WordSlot::QuotedMarker),
        )
    }
    /// Return the quoted parsed word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn word(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_to_python(
            py,
            WordHandle::new(self.handle.clone(), WordSlot::QuotedWord),
        )
    }
    /// Return the combined half-open byte range when available.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn byte_range(&self) -> Option<(usize, usize)> {
        word_like_byte_range(&self.handle)
    }
    /// Return every contributing source span in source order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn source_spans(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        word_like_source_spans(py, &self.handle)
    }
    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        word_like_str(&self.handle)
    }
}

/// `ma'oi` selma'o quotation with marker and quoted word.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "SelmahoQuotedWord",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PySelmahoQuotedWord {
    handle: WordLikeHandle,
}

#[pymethods]
impl PySelmahoQuotedWord {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("mahoi", "word");
    /// Construct a validated `ma'oi` selma'o quotation.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(mahoi: &Bound<'_, PyAny>, word: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mahoi = word_handle_from_python(mahoi)?.clone_rust();
        if !mahoi.is_cmavo(Cmavo::Mahoi) {
            return Err(InvalidInputError::new_err(
                "SelmahoQuotedWord.mahoi must be the cmavo ma'oi",
            ));
        }
        Ok(PySelmahoQuotedWord {
            handle: WordLikeHandle::root(WordLike::mahoi_quote(
                mahoi,
                word_handle_from_python(word)?.clone_rust(),
            )),
        })
    }
    /// Return the `ma'oi` marker word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn mahoi(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_to_python(
            py,
            WordHandle::new(self.handle.clone(), WordSlot::SelmahoMarker),
        )
    }
    /// Return the quoted parsed word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn word(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_to_python(
            py,
            WordHandle::new(self.handle.clone(), WordSlot::SelmahoWord),
        )
    }
    /// Return the combined half-open byte range when available.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn byte_range(&self) -> Option<(usize, usize)> {
        word_like_byte_range(&self.handle)
    }
    /// Return every contributing source span in source order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn source_spans(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        word_like_source_spans(py, &self.handle)
    }
    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        word_like_str(&self.handle)
    }
}

/// Delimiter-based non-Lojban quotation with exact verbatim content.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "DelimitedNonLojbanQuote",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyDelimitedNonLojbanQuote {
    handle: WordLikeHandle,
}

#[pymethods]
impl PyDelimitedNonLojbanQuote {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str, &'static str) = (
        "zoi",
        "opening_delimiter",
        "quoted_text",
        "closing_delimiter",
    );
    /// Construct a validated delimiter-based non-Lojban quotation.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(
        zoi: &Bound<'_, PyAny>,
        opening_delimiter: &Bound<'_, PyAny>,
        quoted_text: PyRef<'_, PyVerbatim>,
        closing_delimiter: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let zoi = word_handle_from_python(zoi)?.clone_rust();
        let opening = word_handle_from_python(opening_delimiter)?.clone_rust();
        let closing = word_handle_from_python(closing_delimiter)?.clone_rust();
        let verbatim = quoted_text.value.clone_rust();
        if !zoi.is_selmaho(Selmaho::Zoi) {
            return Err(InvalidInputError::new_err(
                "zoi must be a delimiter-based non-Lojban quote opener",
            ));
        }
        if !jbotci_morphology::canonical_text_eq(
            opening.phonemes().as_str(),
            closing.phonemes().as_str(),
        ) {
            return Err(InvalidInputError::new_err(
                "opening and closing delimiters must be canonically equal",
            ));
        }
        if opening.span().byte_end > verbatim.span.byte_start
            || verbatim.span.byte_end > closing.span().byte_start
        {
            return Err(InvalidInputError::new_err(
                "quote source spans must occur in input order",
            ));
        }
        Ok(PyDelimitedNonLojbanQuote {
            handle: WordLikeHandle::root(WordLike::zoi_quote(zoi, opening, verbatim, closing)),
        })
    }
    /// Return the quotation marker word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn zoi(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_to_python(
            py,
            WordHandle::new(self.handle.clone(), WordSlot::ZoiMarker),
        )
    }
    /// Return the opening delimiter word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn opening_delimiter(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_to_python(
            py,
            WordHandle::new(self.handle.clone(), WordSlot::ZoiOpeningDelimiter),
        )
    }
    /// Return the exact verbatim quoted text.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn quoted_text(&self) -> PyVerbatim {
        PyVerbatim::located(LocatedVerbatim::new(
            self.handle.clone(),
            VerbatimSlot::ZoiQuotedText,
        ))
    }
    /// Return the closing delimiter word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn closing_delimiter(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_to_python(
            py,
            WordHandle::new(self.handle.clone(), WordSlot::ZoiClosingDelimiter),
        )
    }
    /// Return the combined half-open byte range when available.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn byte_range(&self) -> Option<(usize, usize)> {
        word_like_byte_range(&self.handle)
    }
    /// Return every contributing source span in source order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn source_spans(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        word_like_source_spans(py, &self.handle)
    }
    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        word_like_str(&self.handle)
    }
}

/// `lo'u`/`le'u` quotation containing parsed word tokens.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "QuotedWords",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyQuotedWords {
    handle: WordLikeHandle,
}

#[pymethods]
impl PyQuotedWords {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) =
        ("lohu", "quoted_words", "lehu");
    /// Construct a validated `lo'u`/`le'u` parsed-word quotation.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(
        lohu: &Bound<'_, PyAny>,
        quoted_words: &Bound<'_, PyAny>,
        lehu: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let lohu = word_handle_from_python(lohu)?.clone_rust();
        let lehu = word_handle_from_python(lehu)?.clone_rust();
        if !lohu.is_cmavo(Cmavo::Lohu) || !lehu.is_cmavo(Cmavo::Lehu) {
            return Err(InvalidInputError::new_err(
                "QuotedWords requires lo'u and le'u markers",
            ));
        }
        let words = extract_sequence(quoted_words, "quoted_words", |word| {
            word_handle_from_python(word).map(|word| word.clone_rust())
        })?;
        Ok(PyQuotedWords {
            handle: WordLikeHandle::root(WordLike::lohu_quote(lohu, words, lehu)),
        })
    }
    /// Return the `lo'u` marker word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn lohu(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_to_python(
            py,
            WordHandle::new(self.handle.clone(), WordSlot::LohuMarker),
        )
    }
    /// Return the immutable quoted parsed words.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn quoted_words(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        let data!(WordLike::QuotedWords { quoted_words, .. }) = self.handle.get().as_data() else {
            unreachable!("private construction fixes the word-like variant")
        };
        let values = (0..quoted_words.len())
            .map(|index| {
                word_to_python(
                    py,
                    WordHandle::new(self.handle.clone(), WordSlot::QuotedWordsWord { index }),
                )
            })
            .collect::<PyResult<Vec<_>>>()?;
        crate::support::sequence_to_tuple(py, values).map(Bound::unbind)
    }
    /// Return the `le'u` terminator word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn lehu(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_to_python(
            py,
            WordHandle::new(self.handle.clone(), WordSlot::LehuMarker),
        )
    }
    /// Return the combined half-open byte range when available.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn byte_range(&self) -> Option<(usize, usize)> {
        word_like_byte_range(&self.handle)
    }
    /// Return every contributing source span in source order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn source_spans(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        word_like_source_spans(py, &self.handle)
    }
    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        word_like_str(&self.handle)
    }
}

/// Single verbatim word quotation introduced by a quote marker.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "DelimitedWordQuote",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyDelimitedWordQuote {
    handle: WordLikeHandle,
}

#[pymethods]
impl PyDelimitedWordQuote {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("marker", "quoted_text");
    /// Construct a validated single verbatim word quotation.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(marker: &Bound<'_, PyAny>, quoted_text: PyRef<'_, PyVerbatim>) -> PyResult<Self> {
        let marker = word_handle_from_python(marker)?.clone_rust();
        if !marker
            .cmavo()
            .is_some_and(Cmavo::is_single_word_quote_opener)
        {
            return Err(InvalidInputError::new_err(
                "marker must be a single-word quote opener",
            ));
        }
        Ok(PyDelimitedWordQuote {
            handle: WordLikeHandle::root(WordLike::single_word_quote(
                marker,
                quoted_text.value.clone_rust(),
            )),
        })
    }
    /// Return the quotation marker word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn marker(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_to_python(
            py,
            WordHandle::new(self.handle.clone(), WordSlot::DelimitedWordMarker),
        )
    }
    /// Return the exact verbatim quoted word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn quoted_text(&self) -> PyVerbatim {
        PyVerbatim::located(LocatedVerbatim::new(
            self.handle.clone(),
            VerbatimSlot::DelimitedWordQuotedText,
        ))
    }
    /// Return the combined half-open byte range when available.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn byte_range(&self) -> Option<(usize, usize)> {
        word_like_byte_range(&self.handle)
    }
    /// Return every contributing source span in source order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn source_spans(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        word_like_source_spans(py, &self.handle)
    }
    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        word_like_str(&self.handle)
    }
}

/// `bu` letter word retaining its recursive base and suffix.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "LerfuWord",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyLerfuWord {
    handle: WordLikeHandle,
}

#[pymethods]
impl PyLerfuWord {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("base", "bu");
    /// Construct a validated recursive `bu` letter word.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(base: &Bound<'_, PyAny>, bu: &Bound<'_, PyAny>) -> PyResult<Self> {
        let base = extract_word_like(base)?.into_owned();
        let bu = word_handle_from_python(bu)?.clone_rust();
        if !bu.is_cmavo(Cmavo::Bu) {
            return Err(InvalidInputError::new_err(
                "LerfuWord.bu must be the cmavo bu",
            ));
        }
        Ok(PyLerfuWord {
            handle: WordLikeHandle::root(WordLike::letter(base, bu)),
        })
    }
    /// Return the recursive letter base.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn base(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_like_to_python(py, self.handle.child(WordLikeStep::LerfuBase))
    }
    /// Return the `bu` suffix word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn bu(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_to_python(py, WordHandle::new(self.handle.clone(), WordSlot::BuSuffix))
    }
    /// Return the combined half-open byte range when available.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn byte_range(&self) -> Option<(usize, usize)> {
        word_like_byte_range(&self.handle)
    }
    /// Return every contributing source span in source order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn source_spans(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        word_like_source_spans(py, &self.handle)
    }
    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        word_like_str(&self.handle)
    }
}

/// `zei` compound retaining its recursive left operand, link, and right word.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "ZeiCompound",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyZeiCompound {
    handle: WordLikeHandle,
}

#[pymethods]
impl PyZeiCompound {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) = ("left", "zei", "right");
    /// Construct a validated recursive `zei` compound.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(
        left: &Bound<'_, PyAny>,
        zei: &Bound<'_, PyAny>,
        right: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let left = extract_word_like(left)?.into_owned();
        let zei = word_handle_from_python(zei)?.clone_rust();
        if !zei.is_cmavo(Cmavo::Zei) {
            return Err(InvalidInputError::new_err(
                "ZeiCompound.zei must be the cmavo zei",
            ));
        }
        Ok(PyZeiCompound {
            handle: WordLikeHandle::root(WordLike::zei_lujvo(
                left,
                zei,
                word_handle_from_python(right)?.clone_rust(),
            )),
        })
    }
    /// Return the recursive left operand.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn left(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_like_to_python(py, self.handle.child(WordLikeStep::ZeiLeft))
    }
    /// Return the `zei` link word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn zei(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_to_python(py, WordHandle::new(self.handle.clone(), WordSlot::ZeiLink))
    }
    /// Return the right operand word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn right(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        word_to_python(py, WordHandle::new(self.handle.clone(), WordSlot::ZeiRight))
    }
    /// Return the combined half-open byte range when available.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn byte_range(&self) -> Option<(usize, usize)> {
        word_like_byte_range(&self.handle)
    }
    /// Return every contributing source span in source order.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn source_spans(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        word_like_source_spans(py, &self.handle)
    }
    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        word_like_str(&self.handle)
    }
}

/// Extract a direct typed projection from any public Python `WordLike` variant.
///
/// The returned handle owns the immutable root through `Arc` and retains a typed path to a nested
/// child, so it remains valid after the originating Python parent is deleted. This is the
/// crate-private handoff for the future syntax binding and deliberately performs no serde round
/// trip, field reconstruction, or recursive subtree clone.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub(crate) fn extract_word_like(value: &Bound<'_, PyAny>) -> PyResult<WordLikeHandle> {
    if let Ok(value) = value.extract::<PyRef<'_, PyPlainWord>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyQuotedWord>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PySelmahoQuotedWord>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyDelimitedNonLojbanQuote>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyQuotedWords>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyDelimitedWordQuote>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyLerfuWord>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyZeiCompound>>() {
        return Ok(value.handle.clone());
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "expected a jbotci.morphology WordLike variant",
    ))
}

/// Project a located Rust `WordLike` through the one public Python class family.
#[requires(true)]
#[ensures(true)]
pub(crate) fn word_like_to_python(py: Python<'_>, handle: WordLikeHandle) -> PyResult<Py<PyAny>> {
    match handle.get().as_data() {
        data!(WordLike::PlainWord(_)) => Ok(Py::new(py, PyPlainWord { handle })?.into_any()),
        data!(WordLike::QuotedWord { .. }) => Ok(Py::new(py, PyQuotedWord { handle })?.into_any()),
        data!(WordLike::SelmahoQuotedWord { .. }) => {
            Ok(Py::new(py, PySelmahoQuotedWord { handle })?.into_any())
        }
        data!(WordLike::DelimitedNonLojbanQuote { .. }) => {
            Ok(Py::new(py, PyDelimitedNonLojbanQuote { handle })?.into_any())
        }
        data!(WordLike::QuotedWords { .. }) => {
            Ok(Py::new(py, PyQuotedWords { handle })?.into_any())
        }
        data!(WordLike::DelimitedWordQuote { .. }) => {
            Ok(Py::new(py, PyDelimitedWordQuote { handle })?.into_any())
        }
        data!(WordLike::LerfuWord { .. }) => Ok(Py::new(py, PyLerfuWord { handle })?.into_any()),
        data!(WordLike::ZeiCompound { .. }) => {
            Ok(Py::new(py, PyZeiCompound { handle })?.into_any())
        }
    }
}

/// Project a syntax token's core word through the canonical `WordLike` classes.
#[requires(true)]
#[ensures(true)]
pub(crate) fn token_core_word_to_python(py: Python<'_>, token: TokenHandle) -> PyResult<Py<PyAny>> {
    word_like_to_python(py, token.core_word())
}

/// Project an indicator tree's core word through the canonical `WordLike` classes.
#[requires(true)]
#[ensures(true)]
pub(crate) fn with_indicators_core_word_to_python(
    py: Python<'_>,
    indicators: WithIndicatorsHandle,
) -> PyResult<Py<PyAny>> {
    word_like_to_python(py, indicators.core_word())
}

/// Project a directly stored indicator modifier through the canonical `Word` classes.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
pub(crate) fn with_indicators_word_to_python(
    py: Python<'_>,
    indicators: WithIndicatorsHandle,
    slot: WithIndicatorsWordSlot,
) -> PyResult<Option<Py<PyAny>>> {
    WordHandle::from_indicators(indicators, slot)
        .map(|word| word_to_python(py, word))
        .transpose()
}

#[cfg(test)]
mod syntax_leaf_projection_tests {
    use jbotci_source::SourceSpan;

    use super::*;

    #[requires(jbotci_morphology::cmavo_phonemes(text).is_some())]
    #[ensures(ret.kind() == WordKind::Cmavo)]
    fn cmavo_word(text: &str) -> Word {
        let phonemes = jbotci_morphology::cmavo_phonemes(text)
            .expect("test cmavo must have canonical phonemes");
        let scalar_len = text.chars().count();
        let span = SourceSpan::new(None, 0, text.len(), 0, scalar_len)
            .expect("test source span must be ordered");
        Word::from_kind(WordKind::Cmavo, phonemes, span)
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn token_backed_word_locators_preserve_exact_arc_identity() {
        let word_like = WordLike::bare(cmavo_word("coi"));
        let first = Token::bare(word_like.clone());
        let equal_span_sibling = Token::bare(word_like);
        assert!(!Token::ptr_eq(&first, &equal_span_sibling));

        let first_handle = TokenHandle::from_rust(first.clone());
        let sibling_handle = TokenHandle::from_rust(equal_span_sibling);
        assert_ne!(first_handle, sibling_handle);

        let core = first_handle.core_word();
        let located_word = WordHandle::new(core.clone(), WordSlot::Plain);
        let recovered_from_word_like = core
            .root_token()
            .expect("token-backed core must retain its token")
            .clone_rust();
        let recovered_from_word = located_word
            .root_token()
            .expect("token-backed word must retain its token")
            .clone_rust();
        assert!(Token::ptr_eq(&first, &recovered_from_word_like));
        assert!(Token::ptr_eq(&first, &recovered_from_word));
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn indicator_word_locators_retain_their_token_owner() {
        let token = Token::emphasized(cmavo_word("ba'e"), WordLike::bare(cmavo_word("coi")));
        let handle = TokenHandle::from_rust(token.clone());
        let indicators = handle.indicators();
        assert!(Token::ptr_eq(
            &token,
            indicators
                .exact_token()
                .expect("top-level indicators must recover their exact token")
                .get(),
        ));
        let bahe = WordHandle::from_indicators(indicators, WithIndicatorsWordSlot::EmphasisBahe)
            .expect("emphasized token must expose its ba'e modifier");
        assert!(bahe.get().is_cmavo(Cmavo::Bahe));
        assert!(Token::ptr_eq(
            &token,
            bahe.root_token()
                .expect("indicator word must retain its token")
                .get(),
        ));
    }
}

/// Morphological construct and character range active at a warning or error.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "MorphologyContext",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyMorphologyContext {
    value: MorphologyContext,
}

impl PyMorphologyContext {
    #[requires(value.char_start < value.char_end)]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from_rust(value: MorphologyContext) -> Self {
        PyMorphologyContext { value }
    }
}

#[pymethods]
impl PyMorphologyContext {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) =
        ("kind", "char_start", "char_end");
    /// Construct a typed morphology context over a non-empty character range.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(
        py: Python<'_>,
        kind: &Bound<'_, PyAny>,
        char_start: usize,
        char_end: usize,
    ) -> PyResult<Self> {
        if char_start >= char_end {
            return Err(InvalidInputError::new_err(
                "morphology context must cover a non-empty character range",
            ));
        }
        Ok(Self::from_rust(MorphologyContext::new(
            enum_from_python(py, kind)?,
            char_start,
            char_end,
        )))
    }
    /// Return the context kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.kind)
    }
    /// Return the inclusive character offset.
    #[requires(true)]
    #[ensures(ret == self.value.char_start)]
    #[getter]
    fn char_start(&self) -> usize {
        self.value.char_start
    }
    /// Return the exclusive character offset.
    #[requires(true)]
    #[ensures(ret == self.value.char_end)]
    #[getter]
    fn char_end(&self) -> usize {
        self.value.char_end
    }
    /// Return the human-readable context label.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn label(&self) -> &'static str {
        self.value.label()
    }
}

/// Invalid-lujvo detail carrying the parser expectation and parsed prefix.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "InvalidLujvoDetail",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyInvalidLujvoDetail {
    value: MorphologyErrorDetail,
}

#[pymethods]
impl PyInvalidLujvoDetail {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("parsed_prefix", "expected");
    /// Construct invalid-lujvo detail with an optional parsed prefix.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (expected, parsed_prefix=None))]
    fn new(
        py: Python<'_>,
        expected: &Bound<'_, PyAny>,
        parsed_prefix: Option<String>,
    ) -> PyResult<Self> {
        if parsed_prefix.as_ref().is_some_and(String::is_empty) {
            return Err(InvalidInputError::new_err(
                "parsed_prefix must be non-empty when present",
            ));
        }
        let expected = enum_from_python(py, expected)?;
        Ok(PyInvalidLujvoDetail {
            value: new!(MorphologyErrorDetail::InvalidLujvo {
                parsed_prefix,
                expected,
            }),
        })
    }
    /// Return the optional successfully parsed prefix.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn parsed_prefix(&self) -> Option<&str> {
        let data!(MorphologyErrorDetail::InvalidLujvo { parsed_prefix, .. }) = self.value.as_data()
        else {
            unreachable!("private construction fixes the detail variant")
        };
        parsed_prefix.as_deref()
    }
    /// Return the lujvo parser expectation.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn expected(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let data!(MorphologyErrorDetail::InvalidLujvo { expected, .. }) = self.value.as_data()
        else {
            unreachable!("private construction fixes the detail variant")
        };
        enum_to_python(py, *expected)
    }
}

/// Detail indicating that a fu'ivla contains forbidden `y`.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "FuhivlaContainsYDetail",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyFuhivlaContainsYDetail {
    value: MorphologyErrorDetail,
}

#[pymethods]
impl PyFuhivlaContainsYDetail {
    #[allow(non_upper_case_globals)]
    #[requires(true)]
    #[ensures(ret.bind(py).is_empty())]
    #[classattr]
    fn __match_args__(py: Python<'_>) -> Py<pyo3::types::PyTuple> {
        pyo3::types::PyTuple::empty(py).unbind()
    }

    /// Construct the fieldless fu'ivla-contains-y detail variant.
    #[requires(true)]
    #[ensures(matches!(ret.value.as_data(), data!(MorphologyErrorDetail::FuhivlaContainsY)))]
    #[new]
    fn new() -> Self {
        PyFuhivlaContainsYDetail {
            value: new!(MorphologyErrorDetail::FuhivlaContainsY),
        }
    }
}

/// Detail indicating a slinku'i morphology failure.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "SlinkuhiDetail",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PySlinkuhiDetail {
    value: MorphologyErrorDetail,
}

#[pymethods]
impl PySlinkuhiDetail {
    #[allow(non_upper_case_globals)]
    #[requires(true)]
    #[ensures(ret.bind(py).is_empty())]
    #[classattr]
    fn __match_args__(py: Python<'_>) -> Py<pyo3::types::PyTuple> {
        pyo3::types::PyTuple::empty(py).unbind()
    }

    /// Construct the fieldless slinku'i detail variant.
    #[requires(true)]
    #[ensures(matches!(ret.value.as_data(), data!(MorphologyErrorDetail::Slinkuhi)))]
    #[new]
    fn new() -> Self {
        PySlinkuhiDetail {
            value: new!(MorphologyErrorDetail::Slinkuhi),
        }
    }
}

/// Detail describing the word role expected by morphology.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "ExpectedWordDetail",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyExpectedWordDetail {
    value: MorphologyErrorDetail,
}

#[pymethods]
impl PyExpectedWordDetail {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("expected",);
    /// Construct detail describing the expected word role.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(py: Python<'_>, expected: &Bound<'_, PyAny>) -> PyResult<Self> {
        let expected = enum_from_python(py, expected)?;
        Ok(PyExpectedWordDetail {
            value: new!(MorphologyErrorDetail::ExpectedWord { expected }),
        })
    }
    /// Return the expected word role.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn expected(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let data!(MorphologyErrorDetail::ExpectedWord { expected }) = self.value.as_data() else {
            unreachable!("private construction fixes the detail variant")
        };
        enum_to_python(py, *expected)
    }
}

/// Detail describing why a ZOI delimiter is invalid.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "InvalidZoiDelimiterDetail",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyInvalidZoiDelimiterDetail {
    value: MorphologyErrorDetail,
}

#[pymethods]
impl PyInvalidZoiDelimiterDetail {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("reason",);
    /// Construct detail describing an invalid ZOI delimiter.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(py: Python<'_>, reason: &Bound<'_, PyAny>) -> PyResult<Self> {
        let reason = enum_from_python(py, reason)?;
        Ok(PyInvalidZoiDelimiterDetail {
            value: new!(MorphologyErrorDetail::InvalidZoiDelimiter { reason }),
        })
    }
    /// Return the invalid-delimiter reason.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn reason(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let data!(MorphologyErrorDetail::InvalidZoiDelimiter { reason }) = self.value.as_data()
        else {
            unreachable!("private construction fixes the detail variant")
        };
        enum_to_python(py, *reason)
    }
}

/// Detail identifying the violated phonotactic rule.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "PhonotacticDetail",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyPhonotacticDetail {
    value: MorphologyErrorDetail,
}

#[pymethods]
impl PyPhonotacticDetail {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("reason",);
    /// Construct detail describing a phonotactic violation.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(py: Python<'_>, reason: &Bound<'_, PyAny>) -> PyResult<Self> {
        let reason = enum_from_python(py, reason)?;
        Ok(PyPhonotacticDetail {
            value: new!(MorphologyErrorDetail::Phonotactic { reason }),
        })
    }
    /// Return the phonotactic violation reason.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn reason(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let data!(MorphologyErrorDetail::Phonotactic { reason }) = self.value.as_data() else {
            unreachable!("private construction fixes the detail variant")
        };
        enum_to_python(py, *reason)
    }
}

#[requires(true)]
#[ensures(true)]
fn morphology_detail_from_python(value: &Bound<'_, PyAny>) -> PyResult<MorphologyErrorDetail> {
    if let Ok(value) = value.extract::<PyRef<'_, PyInvalidLujvoDetail>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyFuhivlaContainsYDetail>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PySlinkuhiDetail>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyExpectedWordDetail>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyInvalidZoiDelimiterDetail>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyPhonotacticDetail>>() {
        return Ok(value.value.clone());
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "expected a jbotci.morphology MorphologyErrorDetail variant",
    ))
}

#[requires(true)]
#[ensures(true)]
fn morphology_detail_to_python(
    py: Python<'_>,
    value: MorphologyErrorDetail,
) -> PyResult<Py<PyAny>> {
    match value.as_data() {
        data!(MorphologyErrorDetail::InvalidLujvo { .. }) => {
            Ok(Py::new(py, PyInvalidLujvoDetail { value })?.into_any())
        }
        data!(MorphologyErrorDetail::FuhivlaContainsY) => {
            Ok(Py::new(py, PyFuhivlaContainsYDetail { value })?.into_any())
        }
        data!(MorphologyErrorDetail::Slinkuhi) => {
            Ok(Py::new(py, PySlinkuhiDetail { value })?.into_any())
        }
        data!(MorphologyErrorDetail::ExpectedWord { .. }) => {
            Ok(Py::new(py, PyExpectedWordDetail { value })?.into_any())
        }
        data!(MorphologyErrorDetail::InvalidZoiDelimiter { .. }) => {
            Ok(Py::new(py, PyInvalidZoiDelimiterDetail { value })?.into_any())
        }
        data!(MorphologyErrorDetail::Phonotactic { .. }) => {
            Ok(Py::new(py, PyPhonotacticDetail { value })?.into_any())
        }
    }
}

#[requires(!range_name.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|span| span.char_start == char_start && span.char_end == char_end) || ret.is_err())]
fn validate_diagnostic_char_range(
    source: &str,
    char_start: usize,
    char_end: usize,
    range_name: &str,
) -> PyResult<jbotci_source::SourceSpan> {
    source_span_from_char_offsets(None, source, char_start, char_end)
        .map_err(|error| InvalidInputError::new_err(format!("invalid {range_name}: {error}")))
}

#[requires(!range_name.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_diagnostic_source_text(
    source: &str,
    char_start: usize,
    char_end: usize,
    expected_text: &str,
    range_name: &str,
) -> PyResult<()> {
    let span = validate_diagnostic_char_range(source, char_start, char_end, range_name)?;
    let actual_text = &source[span.byte_start..span.byte_end];
    if actual_text != expected_text {
        return Err(InvalidInputError::new_err(format!(
            "{range_name} text {expected_text:?} does not match source text {actual_text:?} at character range {char_start}..{char_end}"
        )));
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn validate_diagnostic_context(source: &str, context: Option<&MorphologyContext>) -> PyResult<()> {
    if let Some(context) = context {
        validate_diagnostic_char_range(
            source,
            context.char_start,
            context.char_end,
            "morphology context character range",
        )?;
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|diagnostic| !diagnostic.code.is_empty()) || ret.is_err())]
fn warning_to_diagnostic_checked(
    warning: &MorphologyWarning,
    source_id: Option<jbotci_source::SourceId>,
    source: &str,
) -> PyResult<jbotci_diagnostics::Diagnostic> {
    validate_diagnostic_source_text(
        source,
        warning.char_start,
        warning.char_end,
        &warning.text,
        "morphology warning character range",
    )?;
    validate_diagnostic_context(source, warning.context.as_ref())?;
    warning
        .to_diagnostic(source_id, source)
        .map_err(|error| InvalidInputError::new_err(error.to_string()))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|diagnostic| !diagnostic.code.is_empty()) || ret.is_err())]
fn error_to_diagnostic_checked(
    error: &RustMorphologyError,
    source_id: Option<jbotci_source::SourceId>,
    source: &str,
) -> PyResult<jbotci_diagnostics::Diagnostic> {
    match error {
        RustMorphologyError::Invalid {
            char_start,
            char_end,
            text,
            context,
            ..
        } => {
            validate_diagnostic_source_text(
                source,
                *char_start,
                *char_end,
                text,
                "morphology error character range",
            )?;
            validate_diagnostic_context(source, context.as_ref())?;
        }
        RustMorphologyError::UnterminatedZoiQuote {
            char_offset,
            context,
            ..
        } => {
            validate_diagnostic_char_range(
                source,
                *char_offset,
                source.chars().count(),
                "unterminated quote character range",
            )?;
            validate_diagnostic_context(source, context.as_ref())?;
        }
        RustMorphologyError::SourceSpan(_) => {
            validate_diagnostic_char_range(source, 0, 0, "source-span error character range")?;
        }
    }
    error
        .to_diagnostic(source_id, source)
        .map_err(|error| InvalidInputError::new_err(error.to_string()))
}

/// Recoverable morphology warning with source offsets and typed context.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "MorphologyWarning",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyMorphologyWarning {
    value: MorphologyWarning,
}

impl PyMorphologyWarning {
    #[requires(value.char_start < value.char_end)]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from_rust(value: MorphologyWarning) -> Self {
        PyMorphologyWarning { value }
    }
}

#[pymethods]
impl PyMorphologyWarning {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) = (
        "kind",
        "char_start",
        "char_end",
        "text",
        "context",
        "ignored_character_count",
    );
    /// Construct a validated typed morphology warning.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (kind, char_start, char_end, text, *, context=None, ignored_character_count=None))]
    fn new(
        py: Python<'_>,
        kind: &Bound<'_, PyAny>,
        char_start: usize,
        char_end: usize,
        text: String,
        context: Option<PyRef<'_, PyMorphologyContext>>,
        ignored_character_count: Option<usize>,
    ) -> PyResult<Self> {
        let kind = enum_from_python(py, kind)?;
        if char_start >= char_end {
            return Err(InvalidInputError::new_err(
                "warning must cover a non-empty character range",
            ));
        }
        if text.is_empty() {
            return Err(InvalidInputError::new_err("warning text must not be empty"));
        }
        let value = if kind == MorphologyWarningKind::IgnoredCharacters {
            let count = ignored_character_count.ok_or_else(|| {
                InvalidInputError::new_err(
                    "ignored-character warnings require ignored_character_count",
                )
            })?;
            if context.is_some() {
                return Err(InvalidInputError::new_err(
                    "ignored-character warnings do not carry a morphology context",
                ));
            }
            if count == 0 {
                return Err(InvalidInputError::new_err(
                    "ignored_character_count must be greater than zero",
                ));
            }
            MorphologyWarning::ignored_characters(char_start, char_end, text, count)
        } else {
            if ignored_character_count.is_some() {
                return Err(InvalidInputError::new_err(
                    "ignored_character_count is only valid for ignored-character warnings",
                ));
            }
            MorphologyWarning::new(
                kind,
                char_start,
                char_end,
                text,
                context.map(|context| context.value.clone()),
            )
        };
        Ok(Self::from_rust(value))
    }
    /// Return the warning kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.kind)
    }
    /// Return the stable warning code.
    #[requires(true)]
    #[ensures(ret == self.value.kind.code())]
    #[getter]
    fn code(&self) -> &'static str {
        self.value.kind.code()
    }
    /// Return the human-readable warning message.
    #[requires(true)]
    #[ensures(ret == self.value.kind.message())]
    #[getter]
    fn message(&self) -> &'static str {
        self.value.kind.message()
    }
    /// Return the inclusive warning character offset.
    #[requires(true)]
    #[ensures(ret == self.value.char_start)]
    #[getter]
    fn char_start(&self) -> usize {
        self.value.char_start
    }
    /// Return the exclusive warning character offset.
    #[requires(true)]
    #[ensures(ret == self.value.char_end)]
    #[getter]
    fn char_end(&self) -> usize {
        self.value.char_end
    }
    /// Return the affected source text.
    #[requires(true)]
    #[ensures(ret == self.value.text.as_str())]
    #[getter]
    fn text(&self) -> &str {
        &self.value.text
    }
    /// Return the optional typed morphology context.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn context(&self) -> Option<PyMorphologyContext> {
        self.value
            .context
            .clone()
            .map(PyMorphologyContext::from_rust)
    }
    /// Return the ignored-character count when applicable.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn ignored_character_count(&self) -> Option<usize> {
        self.value.ignored_character_count.map(NonZeroUsize::get)
    }
    /// Convert this warning into a source-aware diagnostic.
    #[requires(true)]
    #[ensures(true)]
    #[pyo3(signature = (source, source_id=None))]
    fn to_diagnostic(
        &self,
        source: &str,
        source_id: Option<PyRef<'_, PySourceId>>,
    ) -> PyResult<PyDiagnostic> {
        warning_to_diagnostic_checked(
            &self.value,
            source_id.map(|value| value.clone_rust()),
            source,
        )
        .map(PyDiagnostic::from_rust)
    }
}

/// Structured ordinary morphology failure with typed detail.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "InvalidMorphology",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyInvalidMorphology {
    value: Arc<RustMorphologyError>,
}

#[pymethods]
impl PyInvalidMorphology {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) = (
        "kind",
        "char_start",
        "char_end",
        "text",
        "context",
        "detail",
    );
    /// Construct an ordinary structured morphology failure.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (kind, char_start, char_end, text, *, context=None, detail=None))]
    fn new(
        py: Python<'_>,
        kind: &Bound<'_, PyAny>,
        char_start: usize,
        char_end: usize,
        text: String,
        context: Option<PyRef<'_, PyMorphologyContext>>,
        detail: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let kind = enum_from_python(py, kind)?;
        let detail = detail.map(morphology_detail_from_python).transpose()?;
        Ok(PyInvalidMorphology {
            value: Arc::new(RustMorphologyError::Invalid {
                kind,
                char_start,
                char_end,
                text,
                context: context.map(|context| context.value.clone()),
                detail,
            }),
        })
    }
    /// Return the morphology error kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let RustMorphologyError::Invalid { kind, .. } = self.value.as_ref() else {
            unreachable!("private construction fixes the error variant")
        };
        enum_to_python(py, *kind)
    }
    /// Return the stable morphology error code.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn code(&self) -> &'static str {
        let RustMorphologyError::Invalid { kind, .. } = self.value.as_ref() else {
            unreachable!("private construction fixes the error variant")
        };
        kind.code()
    }
    /// Return the human-readable error message.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn message(&self) -> &'static str {
        let RustMorphologyError::Invalid { kind, .. } = self.value.as_ref() else {
            unreachable!("private construction fixes the error variant")
        };
        kind.message()
    }
    /// Return the inclusive failure character offset.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn char_start(&self) -> usize {
        let RustMorphologyError::Invalid { char_start, .. } = self.value.as_ref() else {
            unreachable!("private construction fixes the error variant")
        };
        *char_start
    }
    /// Return the exclusive failure character offset.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn char_end(&self) -> usize {
        let RustMorphologyError::Invalid { char_end, .. } = self.value.as_ref() else {
            unreachable!("private construction fixes the error variant")
        };
        *char_end
    }
    /// Return the affected source text.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn text(&self) -> &str {
        let RustMorphologyError::Invalid { text, .. } = self.value.as_ref() else {
            unreachable!("private construction fixes the error variant")
        };
        text
    }
    /// Return the optional typed morphology context.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn context(&self) -> Option<PyMorphologyContext> {
        let RustMorphologyError::Invalid { context, .. } = self.value.as_ref() else {
            unreachable!("private construction fixes the error variant")
        };
        context.clone().map(PyMorphologyContext::from_rust)
    }
    /// Return the optional typed variant-specific detail.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn detail(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        let RustMorphologyError::Invalid { detail, .. } = self.value.as_ref() else {
            unreachable!("private construction fixes the error variant")
        };
        detail
            .clone()
            .map(|detail| morphology_detail_to_python(py, detail))
            .transpose()
    }
    /// Convert this failure into a source-aware diagnostic.
    #[requires(true)]
    #[ensures(true)]
    #[pyo3(signature = (source, source_id=None))]
    fn to_diagnostic(
        &self,
        source: &str,
        source_id: Option<PyRef<'_, PySourceId>>,
    ) -> PyResult<PyDiagnostic> {
        error_to_diagnostic_checked(
            &self.value,
            source_id.map(|value| value.clone_rust()),
            source,
        )
        .map(PyDiagnostic::from_rust)
    }
    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        self.value.to_string()
    }
}

/// Structured failure for an unterminated delimiter-based quotation.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "UnterminatedZoiQuote",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyUnterminatedZoiQuote {
    value: Arc<RustMorphologyError>,
}

#[pymethods]
impl PyUnterminatedZoiQuote {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) =
        ("char_offset", "delimiter", "context");
    /// Construct an unterminated-quotation failure.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (char_offset, delimiter, *, context=None))]
    fn new(
        char_offset: usize,
        delimiter: String,
        context: Option<PyRef<'_, PyMorphologyContext>>,
    ) -> PyResult<Self> {
        Ok(PyUnterminatedZoiQuote {
            value: Arc::new(RustMorphologyError::UnterminatedZoiQuote {
                char_offset,
                delimiter,
                context: context.map(|context| context.value.clone()),
            }),
        })
    }
    /// Return the stable morphology error code.
    #[requires(true)]
    #[ensures(ret == "morphology.unterminated-zoi-quote")]
    #[getter]
    fn code(&self) -> &'static str {
        "morphology.unterminated-zoi-quote"
    }
    /// Return the character offset where the quotation remained open.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn char_offset(&self) -> usize {
        let RustMorphologyError::UnterminatedZoiQuote { char_offset, .. } = self.value.as_ref()
        else {
            unreachable!("private construction fixes the error variant")
        };
        *char_offset
    }
    /// Return the opening delimiter spelling.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn delimiter(&self) -> &str {
        let RustMorphologyError::UnterminatedZoiQuote { delimiter, .. } = self.value.as_ref()
        else {
            unreachable!("private construction fixes the error variant")
        };
        delimiter
    }
    /// Return the optional typed morphology context.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn context(&self) -> Option<PyMorphologyContext> {
        let RustMorphologyError::UnterminatedZoiQuote { context, .. } = self.value.as_ref() else {
            unreachable!("private construction fixes the error variant")
        };
        context.clone().map(PyMorphologyContext::from_rust)
    }
    /// Convert this failure into a source-aware diagnostic.
    #[requires(true)]
    #[ensures(true)]
    #[pyo3(signature = (source, source_id=None))]
    fn to_diagnostic(
        &self,
        source: &str,
        source_id: Option<PyRef<'_, PySourceId>>,
    ) -> PyResult<PyDiagnostic> {
        error_to_diagnostic_checked(
            &self.value,
            source_id.map(|value| value.clone_rust()),
            source,
        )
        .map(PyDiagnostic::from_rust)
    }
    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        self.value.to_string()
    }
}

/// Morphology failure caused by invalid source-location data.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "SourceSpanMorphologyError",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PySourceSpanMorphologyError {
    value: Arc<RustMorphologyError>,
}

#[pymethods]
impl PySourceSpanMorphologyError {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("error",);
    /// Construct a morphology error from a typed source-location failure.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(error: &Bound<'_, PyAny>) -> PyResult<Self> {
        let error = source_location_error_from_python(error)?;
        Ok(PySourceSpanMorphologyError {
            value: Arc::new(RustMorphologyError::SourceSpan(error)),
        })
    }
    /// Return the stable morphology error code.
    #[requires(true)]
    #[ensures(ret == "morphology.source-span")]
    #[getter]
    fn code(&self) -> &'static str {
        "morphology.source-span"
    }
    /// Return the typed source-location failure.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn error(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let RustMorphologyError::SourceSpan(error) = self.value.as_ref() else {
            unreachable!("private construction fixes the error variant")
        };
        source_location_error_to_python(py, error.clone())
    }
    /// Convert this failure into a source-aware diagnostic.
    #[requires(true)]
    #[ensures(true)]
    #[pyo3(signature = (source, source_id=None))]
    fn to_diagnostic(
        &self,
        source: &str,
        source_id: Option<PyRef<'_, PySourceId>>,
    ) -> PyResult<PyDiagnostic> {
        error_to_diagnostic_checked(
            &self.value,
            source_id.map(|value| value.clone_rust()),
            source,
        )
        .map(PyDiagnostic::from_rust)
    }
    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        self.value.to_string()
    }
}

#[requires(true)]
#[ensures(true)]
fn morphology_error_to_python(
    py: Python<'_>,
    value: Arc<RustMorphologyError>,
) -> PyResult<Py<PyAny>> {
    match value.as_ref() {
        RustMorphologyError::Invalid { .. } => {
            Ok(Py::new(py, PyInvalidMorphology { value })?.into_any())
        }
        RustMorphologyError::UnterminatedZoiQuote { .. } => {
            Ok(Py::new(py, PyUnterminatedZoiQuote { value })?.into_any())
        }
        RustMorphologyError::SourceSpan(_) => {
            Ok(Py::new(py, PySourceSpanMorphologyError { value })?.into_any())
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn morphology_error_arc_from_python(
    value: &Bound<'_, PyAny>,
) -> PyResult<Arc<RustMorphologyError>> {
    if let Ok(value) = value.extract::<PyRef<'_, PyInvalidMorphology>>() {
        return Ok(Arc::clone(&value.value));
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyUnterminatedZoiQuote>>() {
        return Ok(Arc::clone(&value.value));
    }
    if let Ok(value) = value.extract::<PyRef<'_, PySourceSpanMorphologyError>>() {
        return Ok(Arc::clone(&value.value));
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "expected a jbotci.morphology MorphologyErrorValue variant",
    ))
}

#[invariant(::Words { .. } => true)]
#[invariant(::Error { .. } => true)]
#[derive(Debug, Clone, PartialEq, Eq)]
enum SegmentOutcome {
    Words { values: Vec<Arc<WordLike>> },
    Error { value: Arc<RustMorphologyError> },
}

/// Strict segmentation outcome retaining warnings, traces, and source identity.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "MorphologySegmentAttempt",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyMorphologySegmentAttempt {
    source: Arc<str>,
    source_id: Option<jbotci_source::SourceId>,
    outcome: SegmentOutcome,
    warnings: Arc<[MorphologyWarning]>,
    trace: Option<jbotci_diagnostics::TraceReport>,
}

impl PyMorphologySegmentAttempt {
    #[requires(true)]
    #[ensures(ret.source.as_ref() == source)]
    fn from_rust(
        source: &str,
        source_id: Option<jbotci_source::SourceId>,
        value: MorphologySegmentAttempt,
    ) -> Self {
        let data = value.into_data();
        let outcome = match data.result {
            Ok(words) => SegmentOutcome::Words {
                values: words.into_iter().map(Arc::new).collect(),
            },
            Err(error) => SegmentOutcome::Error {
                value: Arc::new(error),
            },
        };
        Self {
            source: Arc::from(source),
            source_id,
            outcome,
            warnings: Arc::from(data.warnings),
            trace: data.trace,
        }
    }
}

#[pymethods]
impl PyMorphologySegmentAttempt {
    /// Return the original source text.
    #[requires(true)]
    #[ensures(ret == self.source.as_ref())]
    #[getter]
    fn source(&self) -> &str {
        self.source.as_ref()
    }
    /// Return the optional source identifier.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn source_id(&self) -> Option<PySourceId> {
        self.source_id.clone().map(PySourceId::from_rust)
    }
    /// Report whether strict segmentation succeeded.
    #[requires(true)]
    #[ensures(ret == matches!(&self.outcome, SegmentOutcome::Words { .. }))]
    #[getter]
    fn succeeded(&self) -> bool {
        matches!(&self.outcome, SegmentOutcome::Words { .. })
    }
    /// Return parsed words on success, otherwise `None`.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn words(&self, py: Python<'_>) -> PyResult<Option<Py<pyo3::types::PyTuple>>> {
        let SegmentOutcome::Words { values } = &self.outcome else {
            return Ok(None);
        };
        let words = values
            .iter()
            .cloned()
            .map(|word| word_like_to_python(py, WordLikeHandle::from_arc(word)))
            .collect::<PyResult<Vec<_>>>()?;
        crate::support::sequence_to_tuple(py, words)
            .map(Bound::unbind)
            .map(Some)
    }
    /// Return the typed failure on error, otherwise `None`.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn error(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        match &self.outcome {
            SegmentOutcome::Words { .. } => Ok(None),
            SegmentOutcome::Error { value } => {
                morphology_error_to_python(py, Arc::clone(value)).map(Some)
            }
        }
    }
    /// Return immutable warnings from the attempt.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn warnings(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        crate::support::sequence_to_tuple(
            py,
            self.warnings
                .iter()
                .cloned()
                .map(PyMorphologyWarning::from_rust),
        )
        .map(Bound::unbind)
    }
    /// Return the optional morphology trace report.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn trace(&self) -> Option<PyTraceReport> {
        self.trace.clone().map(PyTraceReport::from_rust)
    }
}

/// Recovered segmentation with typed errors paired to skipped source regions.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "RecoveredMorphologySegmentation",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyRecoveredMorphologySegmentation {
    words: Vec<Arc<WordLike>>,
    errors: Vec<Arc<RustMorphologyError>>,
    error_regions: Vec<jbotci_source::SourceSpan>,
    warnings: Vec<MorphologyWarning>,
}

impl PyRecoveredMorphologySegmentation {
    #[requires(true)]
    #[ensures(ret.errors.len() == ret.error_regions.len())]
    fn from_rust(value: RecoveredMorphologySegmentation) -> Self {
        let data = value.into_data();
        PyRecoveredMorphologySegmentation {
            words: data.words.into_iter().map(Arc::new).collect(),
            errors: data.errors.into_iter().map(Arc::new).collect(),
            error_regions: data.error_regions,
            warnings: data.warnings,
        }
    }
}

#[pymethods]
impl PyRecoveredMorphologySegmentation {
    /// Return the immutable recovered word sequence.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn words(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        let values = self
            .words
            .iter()
            .cloned()
            .map(|word| word_like_to_python(py, WordLikeHandle::from_arc(word)))
            .collect::<PyResult<Vec<_>>>()?;
        crate::support::sequence_to_tuple(py, values).map(Bound::unbind)
    }
    /// Return immutable typed recovery errors.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn errors(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        let values = self
            .errors
            .iter()
            .cloned()
            .map(|error| morphology_error_to_python(py, error))
            .collect::<PyResult<Vec<_>>>()?;
        crate::support::sequence_to_tuple(py, values).map(Bound::unbind)
    }
    /// Return source regions skipped for each corresponding error.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn error_regions(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        crate::support::sequence_to_tuple(
            py,
            self.error_regions
                .iter()
                .cloned()
                .map(PySourceSpan::from_rust),
        )
        .map(Bound::unbind)
    }
    /// Return immutable recovery warnings.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn warnings(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        crate::support::sequence_to_tuple(
            py,
            self.warnings
                .iter()
                .cloned()
                .map(PyMorphologyWarning::from_rust),
        )
        .map(Bound::unbind)
    }
}

/// Recovered segmentation attempt retaining source identity and trace output.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "RecoveredMorphologySegmentAttempt",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyRecoveredMorphologySegmentAttempt {
    source: Arc<str>,
    source_id: Option<jbotci_source::SourceId>,
    result: PyRecoveredMorphologySegmentation,
    trace: Option<jbotci_diagnostics::TraceReport>,
}

impl PyRecoveredMorphologySegmentAttempt {
    #[requires(true)]
    #[ensures(ret.source.as_ref() == source)]
    fn from_rust(
        source: &str,
        source_id: Option<jbotci_source::SourceId>,
        value: RecoveredMorphologySegmentAttempt,
    ) -> Self {
        let data = value.into_data();
        Self {
            source: Arc::from(source),
            source_id,
            result: PyRecoveredMorphologySegmentation::from_rust(data.result),
            trace: data.trace,
        }
    }
}

#[pymethods]
impl PyRecoveredMorphologySegmentAttempt {
    /// Return the original source text.
    #[requires(true)]
    #[ensures(ret == self.source.as_ref())]
    #[getter]
    fn source(&self) -> &str {
        self.source.as_ref()
    }
    /// Return the optional source identifier.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn source_id(&self) -> Option<PySourceId> {
        self.source_id.clone().map(PySourceId::from_rust)
    }
    /// Return the recovered segmentation result.
    #[requires(true)]
    #[ensures(ret == self.result)]
    #[getter]
    fn result(&self) -> PyRecoveredMorphologySegmentation {
        self.result.clone()
    }
    /// Return the optional morphology trace report.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn trace(&self) -> Option<PyTraceReport> {
        self.trace.clone().map(PyTraceReport::from_rust)
    }
}

#[requires(true)]
#[ensures(true)]
fn rust_options(options: Option<&PyMorphologyOptions>) -> MorphologyOptions {
    options.map_or_else(MorphologyOptions::default, |options| options.rust().clone())
}

/// Run strict morphology segmentation while retaining warnings and trace output.
#[requires(true)]
#[ensures(ret.source.as_ref() == source)]
#[pyfunction]
#[pyo3(name = "_morphology_segment_attempt", signature = (source, *, options=None, source_id=None))]
fn segment_attempt(
    py: Python<'_>,
    source: String,
    options: Option<PyRef<'_, PyMorphologyOptions>>,
    source_id: Option<PyRef<'_, PySourceId>>,
) -> PyMorphologySegmentAttempt {
    let options = rust_options(options.as_deref());
    let source_id = source_id.map(|value| value.clone_rust());
    let value = py.detach(|| {
        jbotci_morphology::segment_words_with_modifiers_with_options_and_source_id_attempt(
            &source,
            &options,
            source_id.clone(),
        )
    });
    PyMorphologySegmentAttempt::from_rust(&source, source_id, value)
}

/// Run recovered segmentation while retaining source metadata and trace output.
#[requires(true)]
#[ensures(ret.source.as_ref() == source)]
#[pyfunction]
#[pyo3(name = "_morphology_segment_recovered_attempt", signature = (source, *, options=None, source_id=None))]
fn segment_recovered_attempt(
    py: Python<'_>,
    source: String,
    options: Option<PyRef<'_, PyMorphologyOptions>>,
    source_id: Option<PyRef<'_, PySourceId>>,
) -> PyRecoveredMorphologySegmentAttempt {
    let options = rust_options(options.as_deref());
    let source_id = source_id.map(|value| value.clone_rust());
    let value = py.detach(|| {
        jbotci_morphology::segment_words_with_modifiers_recovered_with_options_and_source_id_attempt(
            &source,
            &options,
            source_id.clone(),
        )
    });
    PyRecoveredMorphologySegmentAttempt::from_rust(&source, source_id, value)
}

/// Run the display-oriented segmentation entry point as a strict attempt.
#[requires(true)]
#[ensures(ret.source.as_ref() == source)]
#[pyfunction]
#[pyo3(name = "_morphology_segment_for_display_attempt", signature = (source, *, options=None, source_id=None))]
fn segment_for_display_attempt(
    py: Python<'_>,
    source: String,
    options: Option<PyRef<'_, PyMorphologyOptions>>,
    source_id: Option<PyRef<'_, PySourceId>>,
) -> PyMorphologySegmentAttempt {
    let options = rust_options(options.as_deref());
    let source_id = source_id.map(|value| value.clone_rust());
    let value = py.detach(|| {
        jbotci_morphology::segment_words_for_display_with_options_and_source_id_attempt(
            &source,
            &options,
            source_id.clone(),
        )
    });
    PyMorphologySegmentAttempt::from_rust(&source, source_id, value)
}

/// Typed lujvo-analysis component with exact surface text.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "ValsiLujvoPart",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyValsiLujvoPart {
    value: ValsiLujvoPart,
}

impl PyValsiLujvoPart {
    #[requires(!value.text.is_empty())]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from_rust(value: ValsiLujvoPart) -> Self {
        PyValsiLujvoPart { value }
    }
}

#[pymethods]
impl PyValsiLujvoPart {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) =
        ("kind", "text", "rafsi_kind");
    /// Construct a validated typed lujvo-analysis part.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (kind, text, *, rafsi_kind=None))]
    fn new(
        py: Python<'_>,
        kind: &Bound<'_, PyAny>,
        text: String,
        rafsi_kind: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        if text.is_empty() {
            return Err(InvalidInputError::new_err(
                "lujvo analysis part text must not be empty",
            ));
        }
        let kind = enum_from_python(py, kind)?;
        let rafsi_kind = rafsi_kind
            .map(|value| enum_from_python(py, value))
            .transpose()?;
        if (kind == ValsiLujvoPartKind::Rafsi) != rafsi_kind.is_some() {
            return Err(InvalidInputError::new_err(
                "rafsi parts require rafsi_kind and hyphen parts forbid it",
            ));
        }
        Ok(Self::from_rust(new!(ValsiLujvoPart {
            kind,
            text,
            rafsi_kind
        })))
    }
    /// Return whether this part is a rafsi or hyphen.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.kind)
    }
    /// Return the exact part text.
    #[requires(true)]
    #[ensures(ret == self.value.text.as_str())]
    #[getter]
    fn text(&self) -> &str {
        &self.value.text
    }
    /// Return the rafsi subtype for rafsi parts.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn rafsi_kind(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.value
            .rafsi_kind
            .map(|value| enum_to_python(py, value))
            .transpose()
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClassificationStep {
    LerfuBase,
    ZeiLeft,
}

#[invariant(classification_path_resolves(root.as_ref(), steps))]
#[derive(Debug, Clone)]
struct ClassificationHandle {
    root: Arc<ValsiClassification>,
    steps: Vec<ClassificationStep>,
}

impl PartialEq for ClassificationHandle {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for ClassificationHandle {}

impl ClassificationHandle {
    #[requires(true)]
    #[ensures(ret.steps.is_empty())]
    fn root(value: ValsiClassification) -> Self {
        new!(ClassificationHandle {
            root: Arc::new(value),
            steps: Vec::new(),
        })
    }
    #[requires(true)]
    #[ensures(ret.steps.is_empty())]
    fn from_arc(root: Arc<ValsiClassification>) -> Self {
        new!(ClassificationHandle {
            root,
            steps: Vec::new(),
        })
    }
    #[requires(classification_step_resolves(self.get(), step))]
    #[ensures(ret.steps.len() == self.steps.len() + 1)]
    fn child(&self, step: ClassificationStep) -> Self {
        let mut steps = self.steps.clone();
        steps.push(step);
        new!(ClassificationHandle {
            root: Arc::clone(&self.root),
            steps,
        })
    }
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &ValsiClassification {
        project_classification(self.root.as_ref(), &self.steps)
            .expect("classification handle is valid by construction")
    }
}

#[requires(true)]
#[ensures(ret.is_some() == classification_step_resolves(value, step))]
fn classification_child(
    value: &ValsiClassification,
    step: ClassificationStep,
) -> Option<&ValsiClassification> {
    match (value.as_data(), step) {
        (data!(ValsiClassification::LerfuWord { base, .. }), ClassificationStep::LerfuBase) => {
            Some(base)
        }
        (data!(ValsiClassification::ZeiCompound { left, .. }), ClassificationStep::ZeiLeft) => {
            Some(left)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_some() == classification_path_resolves(root, steps))]
fn project_classification<'a>(
    root: &'a ValsiClassification,
    steps: &[ClassificationStep],
) -> Option<&'a ValsiClassification> {
    let mut current = root;
    for step in steps {
        current = classification_child(current, *step)?;
    }
    Some(current)
}

#[requires(true)]
#[ensures(steps.is_empty() -> ret)]
fn classification_path_resolves(root: &ValsiClassification, steps: &[ClassificationStep]) -> bool {
    let mut current = root;
    for step in steps {
        let Some(child) = classification_child(current, *step) else {
            return false;
        };
        current = child;
    }
    true
}

#[requires(true)]
#[ensures(
    ret
        == matches!(
            (value.as_data(), step),
            (
                data!(ValsiClassification::LerfuWord { .. }),
                ClassificationStep::LerfuBase
            ) | (
                data!(ValsiClassification::ZeiCompound { .. }),
                ClassificationStep::ZeiLeft
            )
        )
)]
fn classification_step_resolves(value: &ValsiClassification, step: ClassificationStep) -> bool {
    matches!(
        (value.as_data(), step),
        (
            data!(ValsiClassification::LerfuWord { .. }),
            ClassificationStep::LerfuBase
        ) | (
            data!(ValsiClassification::ZeiCompound { .. }),
            ClassificationStep::ZeiLeft
        )
    )
}

#[invariant(::PlainWord => true)]
#[invariant(::QuotedMarker => true)]
#[invariant(::QuotedTarget => true)]
#[invariant(::DelimitedMarker => true)]
#[invariant(::QuotedWordsMarker => true)]
#[invariant(::QuotedWordsTarget { .. } => true)]
#[invariant(::LerfuSuffix => true)]
#[invariant(::ZeiLink => true)]
#[invariant(::ZeiRight => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlainClassificationSlot {
    PlainWord,
    QuotedMarker,
    QuotedTarget,
    DelimitedMarker,
    QuotedWordsMarker,
    QuotedWordsTarget { index: usize },
    LerfuSuffix,
    ZeiLink,
    ZeiRight,
}

#[invariant(plain_classification_slot_resolves(owner.get(), *slot))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct LocatedPlainClassification {
    owner: ClassificationHandle,
    slot: PlainClassificationSlot,
}

impl LocatedPlainClassification {
    #[requires(plain_classification_slot_resolves(owner.get(), slot))]
    #[ensures(true)]
    fn new(owner: ClassificationHandle, slot: PlainClassificationSlot) -> Self {
        new!(LocatedPlainClassification { owner, slot })
    }
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &PlainWordClassification {
        project_plain_classification(self.owner.get(), self.slot)
            .expect("plain classification handle is valid by construction")
    }
}

#[requires(true)]
#[ensures(
    ret.is_some()
        == match (value.as_data(), slot) {
            (data!(ValsiClassification::PlainWord { .. }), PlainClassificationSlot::PlainWord)
            | (data!(ValsiClassification::QuotedWord { .. }), PlainClassificationSlot::QuotedMarker | PlainClassificationSlot::QuotedTarget)
            | (data!(ValsiClassification::DelimitedNonLojbanQuote { .. }), PlainClassificationSlot::DelimitedMarker)
            | (data!(ValsiClassification::QuotedWords { .. }), PlainClassificationSlot::QuotedWordsMarker)
            | (data!(ValsiClassification::LerfuWord { .. }), PlainClassificationSlot::LerfuSuffix)
            | (data!(ValsiClassification::ZeiCompound { .. }), PlainClassificationSlot::ZeiLink | PlainClassificationSlot::ZeiRight) => true,
            (
                data!(ValsiClassification::QuotedWords { quoted_words, .. }),
                PlainClassificationSlot::QuotedWordsTarget { index },
            ) => index < quoted_words.len(),
            _ => false,
        }
)]
fn plain_classification_at_slot(
    value: &ValsiClassification,
    slot: PlainClassificationSlot,
) -> Option<&PlainWordClassification> {
    match (value.as_data(), slot) {
        (data!(ValsiClassification::PlainWord { word }), PlainClassificationSlot::PlainWord) => {
            Some(word)
        }
        (
            data!(ValsiClassification::QuotedWord { marker, .. }),
            PlainClassificationSlot::QuotedMarker,
        ) => Some(marker),
        (
            data!(ValsiClassification::QuotedWord { quoted_word, .. }),
            PlainClassificationSlot::QuotedTarget,
        ) => Some(quoted_word),
        (
            data!(ValsiClassification::DelimitedNonLojbanQuote { marker, .. }),
            PlainClassificationSlot::DelimitedMarker,
        ) => Some(marker),
        (
            data!(ValsiClassification::QuotedWords { marker, .. }),
            PlainClassificationSlot::QuotedWordsMarker,
        ) => Some(marker),
        (
            data!(ValsiClassification::QuotedWords { quoted_words, .. }),
            PlainClassificationSlot::QuotedWordsTarget { index },
        ) => quoted_words.get(index),
        (
            data!(ValsiClassification::LerfuWord { suffix, .. }),
            PlainClassificationSlot::LerfuSuffix,
        ) => Some(suffix),
        (
            data!(ValsiClassification::ZeiCompound { link, .. }),
            PlainClassificationSlot::ZeiLink,
        ) => Some(link),
        (
            data!(ValsiClassification::ZeiCompound { right, .. }),
            PlainClassificationSlot::ZeiRight,
        ) => Some(right),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_some() == plain_classification_slot_resolves(value, slot))]
fn project_plain_classification(
    value: &ValsiClassification,
    slot: PlainClassificationSlot,
) -> Option<&PlainWordClassification> {
    plain_classification_at_slot(value, slot)
}

#[requires(true)]
#[ensures(ret == plain_classification_at_slot(value, slot).is_some())]
fn plain_classification_slot_resolves(
    value: &ValsiClassification,
    slot: PlainClassificationSlot,
) -> bool {
    plain_classification_at_slot(value, slot).is_some()
}

#[invariant(::Owned { value } => !value.phonemes.is_empty())]
#[invariant(::Located { .. } => true)]
#[derive(Debug, Clone)]
enum PlainClassificationStorage {
    Owned { value: Arc<PlainWordClassification> },
    Located { value: LocatedPlainClassification },
}

impl PartialEq for PlainClassificationStorage {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for PlainClassificationStorage {}

impl PlainClassificationStorage {
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &PlainWordClassification {
        match self.as_data() {
            data!(PlainClassificationStorage::Owned { value }) => value.as_ref(),
            data!(PlainClassificationStorage::Located { value }) => value.get(),
        }
    }
    #[requires(true)]
    #[ensures(ret == self.get().clone())]
    fn clone_rust(&self) -> PlainWordClassification {
        self.get().clone()
    }
}

/// Detailed classification of one plain parsed word.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "PlainWordClassification",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyPlainWordClassification {
    value: PlainClassificationStorage,
}

impl PyPlainWordClassification {
    #[requires(true)]
    #[ensures(true)]
    fn located(owner: ClassificationHandle, slot: PlainClassificationSlot) -> Self {
        PyPlainWordClassification {
            value: new!(PlainClassificationStorage::Located {
                value: LocatedPlainClassification::new(owner, slot),
            }),
        }
    }
}

#[pymethods]
impl PyPlainWordClassification {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) = ("category", "phonemes", "selmaho", "split", "parts", "stage");
    /// Construct a validated detailed plain-word classification.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (category, phonemes, *, selmaho=None, split=None, parts=Vec::new(), stage=None))]
    fn new(
        py: Python<'_>,
        category: &Bound<'_, PyAny>,
        phonemes: String,
        selmaho: Option<String>,
        split: Option<String>,
        parts: Vec<PyRef<'_, PyValsiLujvoPart>>,
        stage: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        if phonemes.is_empty() {
            return Err(InvalidInputError::new_err(
                "classification phonemes must not be empty",
            ));
        }
        let category = enum_from_python(py, category)?;
        let parts = parts
            .into_iter()
            .map(|part| part.value.clone())
            .collect::<Vec<_>>();
        let stage = stage.map(|value| enum_from_python(py, value)).transpose()?;
        if category != WordKind::Cmavo && selmaho.is_some() {
            return Err(InvalidInputError::new_err(
                "selmaho is only valid for cmavo classifications",
            ));
        }
        if category != WordKind::Lujvo && (split.is_some() || !parts.is_empty()) {
            return Err(InvalidInputError::new_err(
                "split and parts are only valid for lujvo classifications",
            ));
        }
        if category != WordKind::Fuhivla && stage.is_some() {
            return Err(InvalidInputError::new_err(
                "stage is only valid for fu'ivla classifications",
            ));
        }
        Ok(PyPlainWordClassification {
            value: new!(PlainClassificationStorage::Owned {
                value: Arc::new(new!(PlainWordClassification {
                    category,
                    phonemes,
                    selmaho,
                    split,
                    parts,
                    stage
                })),
            }),
        })
    }
    /// Return the morphology word category.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn category(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.get().category)
    }
    /// Return the canonical phoneme text.
    #[requires(true)]
    #[ensures(ret == self.value.get().phonemes.as_str())]
    #[getter]
    fn phonemes(&self) -> &str {
        &self.value.get().phonemes
    }
    /// Return the selma'o name for a cmavo classification.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn selmaho(&self) -> Option<&str> {
        self.value.get().selmaho.as_deref()
    }
    /// Return the rendered lujvo split when available.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn split(&self) -> Option<&str> {
        self.value.get().split.as_deref()
    }
    /// Return immutable lujvo analysis parts.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn parts(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        crate::support::sequence_to_tuple(
            py,
            self.value
                .get()
                .parts
                .iter()
                .cloned()
                .map(PyValsiLujvoPart::from_rust),
        )
        .map(Bound::unbind)
    }
    /// Return the fu'ivla stage when applicable.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn stage(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.value
            .get()
            .stage
            .map(|value| enum_to_python(py, value))
            .transpose()
    }
}

#[requires(true)]
#[ensures(true)]
fn classification_kind(py: Python<'_>, handle: &ClassificationHandle) -> PyResult<Py<PyAny>> {
    enum_to_python(py, handle.get().kind())
}

/// Valsi classification for one plain word.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "PlainWordValsiClassification",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyPlainWordValsiClassification {
    handle: ClassificationHandle,
}

#[pymethods]
impl PyPlainWordValsiClassification {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("word",);
    /// Construct a plain-word valsi classification.
    #[requires(true)]
    #[ensures(true)]
    #[new]
    fn new(word: PyRef<'_, PyPlainWordClassification>) -> Self {
        PyPlainWordValsiClassification {
            handle: ClassificationHandle::root(new!(ValsiClassification::PlainWord {
                word: word.value.clone_rust()
            })),
        }
    }
    /// Return the classification variant kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        classification_kind(py, &self.handle)
    }
    /// Return the detailed plain-word classification.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn word(&self) -> PyPlainWordClassification {
        PyPlainWordClassification::located(self.handle.clone(), PlainClassificationSlot::PlainWord)
    }
}

/// Valsi classification for a quoted word.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "QuotedWordValsiClassification",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyQuotedWordValsiClassification {
    handle: ClassificationHandle,
}

#[pymethods]
impl PyQuotedWordValsiClassification {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("marker", "quoted_word");
    /// Construct a quoted-word valsi classification.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(
        marker: PyRef<'_, PyPlainWordClassification>,
        quoted_word: PyRef<'_, PyPlainWordClassification>,
    ) -> PyResult<Self> {
        if marker.value.get().category != WordKind::Cmavo {
            return Err(InvalidInputError::new_err(
                "quoted-word marker classification must be cmavo",
            ));
        }
        Ok(PyQuotedWordValsiClassification {
            handle: ClassificationHandle::root(new!(ValsiClassification::QuotedWord {
                marker: marker.value.clone_rust(),
                quoted_word: quoted_word.value.clone_rust()
            })),
        })
    }
    /// Return the classification variant kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        classification_kind(py, &self.handle)
    }
    /// Return the quote marker classification.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn marker(&self) -> PyPlainWordClassification {
        PyPlainWordClassification::located(
            self.handle.clone(),
            PlainClassificationSlot::QuotedMarker,
        )
    }
    /// Return the quoted word classification.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn quoted_word(&self) -> PyPlainWordClassification {
        PyPlainWordClassification::located(
            self.handle.clone(),
            PlainClassificationSlot::QuotedTarget,
        )
    }
}

/// Valsi classification for a delimiter-based non-Lojban quote.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "DelimitedNonLojbanQuoteValsiClassification",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyDelimitedNonLojbanQuoteValsiClassification {
    handle: ClassificationHandle,
}

#[pymethods]
impl PyDelimitedNonLojbanQuoteValsiClassification {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("marker", "delimiter");
    /// Construct a delimiter-based quote classification.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(marker: PyRef<'_, PyPlainWordClassification>, delimiter: String) -> PyResult<Self> {
        if marker.value.get().category != WordKind::Cmavo {
            return Err(InvalidInputError::new_err(
                "delimited quote marker classification must be cmavo",
            ));
        }
        if delimiter.is_empty() {
            return Err(InvalidInputError::new_err("delimiter must not be empty"));
        }
        Ok(PyDelimitedNonLojbanQuoteValsiClassification {
            handle: ClassificationHandle::root(new!(
                ValsiClassification::DelimitedNonLojbanQuote {
                    marker: marker.value.clone_rust(),
                    delimiter
                }
            )),
        })
    }
    /// Return the classification variant kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        classification_kind(py, &self.handle)
    }
    /// Return the quotation marker classification.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn marker(&self) -> PyPlainWordClassification {
        PyPlainWordClassification::located(
            self.handle.clone(),
            PlainClassificationSlot::DelimitedMarker,
        )
    }
    /// Return the delimiter spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn delimiter(&self) -> &str {
        let data!(ValsiClassification::DelimitedNonLojbanQuote { delimiter, .. }) =
            self.handle.get().as_data()
        else {
            unreachable!("private construction fixes the classification variant")
        };
        delimiter
    }
}

/// Valsi classification for a quoted parsed-word sequence.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "QuotedWordsValsiClassification",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyQuotedWordsValsiClassification {
    handle: ClassificationHandle,
}

#[pymethods]
impl PyQuotedWordsValsiClassification {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("marker", "quoted_words");
    /// Construct a quoted-words valsi classification.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(
        marker: PyRef<'_, PyPlainWordClassification>,
        quoted_words: Vec<PyRef<'_, PyPlainWordClassification>>,
    ) -> PyResult<Self> {
        if marker.value.get().category != WordKind::Cmavo {
            return Err(InvalidInputError::new_err(
                "quoted-words marker classification must be cmavo",
            ));
        }
        Ok(PyQuotedWordsValsiClassification {
            handle: ClassificationHandle::root(new!(ValsiClassification::QuotedWords {
                marker: marker.value.clone_rust(),
                quoted_words: quoted_words
                    .into_iter()
                    .map(|word| word.value.clone_rust())
                    .collect()
            })),
        })
    }
    /// Return the classification variant kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        classification_kind(py, &self.handle)
    }
    /// Return the quotation marker classification.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn marker(&self) -> PyPlainWordClassification {
        PyPlainWordClassification::located(
            self.handle.clone(),
            PlainClassificationSlot::QuotedWordsMarker,
        )
    }
    /// Return immutable quoted word classifications.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn quoted_words(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        let data!(ValsiClassification::QuotedWords { quoted_words, .. }) =
            self.handle.get().as_data()
        else {
            unreachable!("private construction fixes the classification variant")
        };
        let values = (0..quoted_words.len()).map(|index| {
            PyPlainWordClassification::located(
                self.handle.clone(),
                PlainClassificationSlot::QuotedWordsTarget { index },
            )
        });
        crate::support::sequence_to_tuple(py, values).map(Bound::unbind)
    }
}

/// Valsi classification for a single verbatim word quote.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "DelimitedWordQuoteValsiClassification",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyDelimitedWordQuoteValsiClassification {
    handle: ClassificationHandle,
}

#[pymethods]
impl PyDelimitedWordQuoteValsiClassification {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("marker_text",);
    /// Construct a single delimited-word quote classification.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(marker_text: String) -> PyResult<Self> {
        if marker_text.is_empty() {
            return Err(InvalidInputError::new_err("marker_text must not be empty"));
        }
        Ok(PyDelimitedWordQuoteValsiClassification {
            handle: ClassificationHandle::root(new!(ValsiClassification::DelimitedWordQuote {
                marker_text
            })),
        })
    }
    /// Return the classification variant kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        classification_kind(py, &self.handle)
    }
    /// Return the marker spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn marker_text(&self) -> &str {
        let data!(ValsiClassification::DelimitedWordQuote { marker_text }) =
            self.handle.get().as_data()
        else {
            unreachable!("private construction fixes the classification variant")
        };
        marker_text
    }
}

#[requires(true)]
#[ensures(true)]
fn classification_handle_from_python(value: &Bound<'_, PyAny>) -> PyResult<ClassificationHandle> {
    if let Ok(value) = value.extract::<PyRef<'_, PyPlainWordValsiClassification>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyQuotedWordValsiClassification>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyDelimitedNonLojbanQuoteValsiClassification>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyQuotedWordsValsiClassification>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyDelimitedWordQuoteValsiClassification>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyLerfuWordValsiClassification>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyZeiCompoundValsiClassification>>() {
        return Ok(value.handle.clone());
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "expected a jbotci.morphology ValsiClassification variant",
    ))
}

/// Valsi classification for a recursive `bu` letter word.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "LerfuWordValsiClassification",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyLerfuWordValsiClassification {
    handle: ClassificationHandle,
}

#[pymethods]
impl PyLerfuWordValsiClassification {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("base", "suffix");
    /// Construct a recursive `bu` letter-word classification.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(
        base: &Bound<'_, PyAny>,
        suffix: PyRef<'_, PyPlainWordClassification>,
    ) -> PyResult<Self> {
        if suffix.value.get().category != WordKind::Cmavo {
            return Err(InvalidInputError::new_err(
                "lerfu suffix classification must be cmavo",
            ));
        }
        let base = classification_handle_from_python(base)?;
        Ok(PyLerfuWordValsiClassification {
            handle: ClassificationHandle::root(new!(ValsiClassification::LerfuWord {
                base: Box::new(base.get().clone()),
                suffix: suffix.value.clone_rust()
            })),
        })
    }
    /// Return the classification variant kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        classification_kind(py, &self.handle)
    }
    /// Return the recursive base classification.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn base(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        classification_to_python(py, self.handle.child(ClassificationStep::LerfuBase))
    }
    /// Return the `bu` suffix classification.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn suffix(&self) -> PyPlainWordClassification {
        PyPlainWordClassification::located(
            self.handle.clone(),
            PlainClassificationSlot::LerfuSuffix,
        )
    }
}

/// Valsi classification for a recursive `zei` compound.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "ZeiCompoundValsiClassification",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyZeiCompoundValsiClassification {
    handle: ClassificationHandle,
}

#[pymethods]
impl PyZeiCompoundValsiClassification {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) = ("left", "link", "right");
    /// Construct a recursive `zei` compound classification.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(
        left: &Bound<'_, PyAny>,
        link: PyRef<'_, PyPlainWordClassification>,
        right: PyRef<'_, PyPlainWordClassification>,
    ) -> PyResult<Self> {
        if link.value.get().category != WordKind::Cmavo {
            return Err(InvalidInputError::new_err(
                "ZEI link classification must be cmavo",
            ));
        }
        let left = classification_handle_from_python(left)?;
        Ok(PyZeiCompoundValsiClassification {
            handle: ClassificationHandle::root(new!(ValsiClassification::ZeiCompound {
                left: Box::new(left.get().clone()),
                link: link.value.clone_rust(),
                right: right.value.clone_rust()
            })),
        })
    }
    /// Return the classification variant kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        classification_kind(py, &self.handle)
    }
    /// Return the recursive left classification.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn left(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        classification_to_python(py, self.handle.child(ClassificationStep::ZeiLeft))
    }
    /// Return the `zei` link classification.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn link(&self) -> PyPlainWordClassification {
        PyPlainWordClassification::located(self.handle.clone(), PlainClassificationSlot::ZeiLink)
    }
    /// Return the right word classification.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn right(&self) -> PyPlainWordClassification {
        PyPlainWordClassification::located(self.handle.clone(), PlainClassificationSlot::ZeiRight)
    }
}

#[requires(true)]
#[ensures(true)]
fn classification_to_python(py: Python<'_>, handle: ClassificationHandle) -> PyResult<Py<PyAny>> {
    match handle.get().as_data() {
        data!(ValsiClassification::PlainWord { .. }) => {
            Ok(Py::new(py, PyPlainWordValsiClassification { handle })?.into_any())
        }
        data!(ValsiClassification::QuotedWord { .. }) => {
            Ok(Py::new(py, PyQuotedWordValsiClassification { handle })?.into_any())
        }
        data!(ValsiClassification::DelimitedNonLojbanQuote { .. }) => {
            Ok(Py::new(py, PyDelimitedNonLojbanQuoteValsiClassification { handle })?.into_any())
        }
        data!(ValsiClassification::QuotedWords { .. }) => {
            Ok(Py::new(py, PyQuotedWordsValsiClassification { handle })?.into_any())
        }
        data!(ValsiClassification::DelimitedWordQuote { .. }) => {
            Ok(Py::new(py, PyDelimitedWordQuoteValsiClassification { handle })?.into_any())
        }
        data!(ValsiClassification::LerfuWord { .. }) => {
            Ok(Py::new(py, PyLerfuWordValsiClassification { handle })?.into_any())
        }
        data!(ValsiClassification::ZeiCompound { .. }) => {
            Ok(Py::new(py, PyZeiCompoundValsiClassification { handle })?.into_any())
        }
    }
}

/// Status-dependent payload of single-valsi analysis.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "ValsiAnalysisResult",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyValsiAnalysisResult {
    status: ValsiAnalysisStatus,
    word: Option<Arc<WordLike>>,
    classification: Option<Arc<ValsiClassification>>,
    error: Option<Arc<RustMorphologyError>>,
    words: Vec<Arc<WordLike>>,
}

impl PyValsiAnalysisResult {
    #[requires(true)]
    #[ensures(ret.status == old(value.status))]
    fn from_rust(value: ValsiAnalysisResult) -> Self {
        let data = value.into_data();
        PyValsiAnalysisResult {
            status: data.status,
            word: data.word.map(Arc::new),
            classification: data.classification.map(Arc::new),
            error: data.error.map(Arc::new),
            words: data.words.into_iter().map(Arc::new).collect(),
        }
    }
}

#[pymethods]
impl PyValsiAnalysisResult {
    /// Construct a status-consistent valsi analysis payload.
    #[allow(clippy::too_many_arguments)]
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (status, *, word=None, classification=None, error=None, words=Vec::new()))]
    fn new(
        py: Python<'_>,
        status: &Bound<'_, PyAny>,
        word: Option<&Bound<'_, PyAny>>,
        classification: Option<&Bound<'_, PyAny>>,
        error: Option<&Bound<'_, PyAny>>,
        words: Vec<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let status = enum_from_python(py, status)?;
        let word = word
            .map(extract_word_like)
            .transpose()?
            .map(|value| Arc::new(value.into_owned()));
        let classification = classification
            .map(classification_handle_from_python)
            .transpose()?
            .map(|value| Arc::new(value.get().clone()));
        let error = error.map(morphology_error_arc_from_python).transpose()?;
        let words = words
            .iter()
            .map(extract_word_like)
            .map(|value| value.map(|value| Arc::new(value.into_owned())))
            .collect::<PyResult<Vec<_>>>()?;
        let valid_shape = match status {
            ValsiAnalysisStatus::Valid => {
                word.is_some() && classification.is_some() && error.is_none() && words.is_empty()
            }
            ValsiAnalysisStatus::Invalid => {
                word.is_none() && classification.is_none() && error.is_some() && words.is_empty()
            }
            ValsiAnalysisStatus::NotSingleWord => {
                word.is_none() && classification.is_none() && error.is_none()
            }
        };
        if !valid_shape {
            return Err(InvalidInputError::new_err(
                "valsi analysis fields do not match status",
            ));
        }
        Ok(PyValsiAnalysisResult {
            status,
            word,
            classification,
            error,
            words,
        })
    }
    /// Return the valsi analysis status.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn status(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.status)
    }
    /// Report whether analysis produced one valid valsi.
    #[requires(true)]
    #[ensures(ret == (self.status == ValsiAnalysisStatus::Valid))]
    #[getter]
    fn is_valid(&self) -> bool {
        self.status == ValsiAnalysisStatus::Valid
    }
    /// Return the parsed word-like value for a valid result.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn word(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.word
            .as_ref()
            .map(|word| word_like_to_python(py, WordLikeHandle::from_arc(Arc::clone(word))))
            .transpose()
    }
    /// Return the typed classification for a valid result.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn classification(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.classification
            .as_ref()
            .map(|value| {
                classification_to_python(py, ClassificationHandle::from_arc(Arc::clone(value)))
            })
            .transpose()
    }
    /// Return the typed error for an invalid result.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn error(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.error
            .as_ref()
            .map(|error| morphology_error_to_python(py, Arc::clone(error)))
            .transpose()
    }
    /// Return parsed words for a not-single-word result.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn words(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        let values = self
            .words
            .iter()
            .cloned()
            .map(|word| word_like_to_python(py, WordLikeHandle::from_arc(word)))
            .collect::<PyResult<Vec<_>>>()?;
        crate::support::sequence_to_tuple(py, values).map(Bound::unbind)
    }
}

/// Complete single-valsi analysis with input and warnings.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "ValsiAnalysis",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyValsiAnalysis {
    input: Arc<str>,
    warnings: Arc<[MorphologyWarning]>,
    result: PyValsiAnalysisResult,
}

impl PyValsiAnalysis {
    #[requires(true)]
    #[expensive_ensures(ret.input.as_ref() == old(value.input.clone()))]
    fn from_rust(value: ValsiAnalysis) -> Self {
        let data = value.into_data();
        Self {
            input: Arc::from(data.input),
            warnings: Arc::from(data.warnings),
            result: PyValsiAnalysisResult::from_rust(data.result),
        }
    }
}

#[pymethods]
impl PyValsiAnalysis {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) =
        ("input", "warnings", "result");
    /// Return the original analyzed input.
    #[requires(true)]
    #[ensures(ret == self.input.as_ref())]
    #[getter]
    fn input(&self) -> &str {
        self.input.as_ref()
    }
    /// Return immutable morphology warnings.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn warnings(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        crate::support::sequence_to_tuple(
            py,
            self.warnings
                .iter()
                .cloned()
                .map(PyMorphologyWarning::from_rust),
        )
        .map(Bound::unbind)
    }
    /// Return the status-dependent analysis result.
    #[requires(true)]
    #[ensures(ret == self.result)]
    #[getter]
    fn result(&self) -> PyValsiAnalysisResult {
        self.result.clone()
    }
}

/// Classify a single valsi using the real Rust morphology parser.
#[requires(true)]
#[ensures(ret.input.as_ref() == source)]
#[pyfunction]
#[pyo3(name = "_morphology_analyze_valsi", signature = (source, *, options=None, source_id=None))]
fn analyze_valsi(
    py: Python<'_>,
    source: String,
    options: Option<PyRef<'_, PyMorphologyOptions>>,
    source_id: Option<PyRef<'_, PySourceId>>,
) -> PyValsiAnalysis {
    let options = rust_options(options.as_deref());
    let source_id = source_id.map(|value| value.clone_rust());
    PyValsiAnalysis::from_rust(py.detach(|| {
        jbotci_morphology::analyze_valsi_with_options_and_source_id(&source, &options, source_id)
    }))
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| value.len_utf8() == text.len()) || ret.is_err())]
fn one_unicode_scalar(text: &str, parameter: &str) -> PyResult<char> {
    let mut values = text.chars();
    let Some(value) = values.next() else {
        return Err(InvalidInputError::new_err(format!(
            "{parameter} must be exactly one Unicode scalar"
        )));
    };
    if values.next().is_some() {
        return Err(InvalidInputError::new_err(format!(
            "{parameter} must be exactly one Unicode scalar"
        )));
    }
    Ok(value)
}

#[requires(true)]
#[ensures(true)]
fn owned_lujvo_part_to_python(py: Python<'_>, part: LujvoPart) -> PyResult<Py<PyAny>> {
    let value = LujvoPartStorage::Owned {
        value: Arc::new(part),
    };
    match value.get() {
        LujvoPart::Rafsi(_) => Ok(Py::new(py, PyLujvoRafsi { value })?.into_any()),
        LujvoPart::Hyphen(_) => Ok(Py::new(py, PyLujvoHyphen { value })?.into_any()),
    }
}

#[requires(true)]
#[ensures(true)]
fn lujvo_parts_to_tuple(
    py: Python<'_>,
    parts: Vec<LujvoPart>,
) -> PyResult<Py<pyo3::types::PyTuple>> {
    let values = parts
        .into_iter()
        .map(|part| owned_lujvo_part_to_python(py, part))
        .collect::<PyResult<Vec<_>>>()?;
    crate::support::sequence_to_tuple(py, values).map(Bound::unbind)
}

/// Normalize input text according to morphology options when all characters are accepted.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_normalize_input", signature = (text, *, options=None))]
fn normalize_input(text: &str, options: Option<PyRef<'_, PyMorphologyOptions>>) -> Option<String> {
    let options = rust_options(options.as_deref());
    jbotci_morphology::normalize_lojban_input_text_with_options(text, &options)
}

/// Canonicalize Lojban surface text for morphology identity comparisons.
#[requires(true)]
#[ensures(!ret.is_empty() || text.is_empty())]
#[pyfunction]
#[pyo3(name = "_morphology_canonicalize_text")]
fn canonicalize_text(text: &str) -> String {
    jbotci_morphology::canonicalize_text(text)
}

/// Compare two strings by canonical Lojban text identity.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_canonical_text_eq")]
fn canonical_text_eq(left: &str, right: &str) -> bool {
    jbotci_morphology::canonical_text_eq(left, right)
}

/// Test whether every canonical character equals one supplied Unicode scalar.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_canonical_text_is_all")]
fn canonical_text_is_all(text: &str, expected: &str) -> PyResult<bool> {
    Ok(jbotci_morphology::canonical_text_is_all(
        text,
        one_unicode_scalar(expected, "expected")?,
    ))
}

/// Normalize a valid cmavo spelling into its canonical form.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_normalize_cmavo_form")]
fn normalize_cmavo_form(text: &str) -> Option<String> {
    jbotci_morphology::normalize_cmavo_form(text)
}

/// Parse a valid cmavo spelling into canonical phonemes.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_cmavo_phonemes")]
fn cmavo_phonemes(text: &str) -> Option<PyPhonemes> {
    jbotci_morphology::cmavo_phonemes(text).map(PyPhonemes::from_rust)
}

/// Split canonical phonemes into pronunciation syllables.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_pronunciation_syllables")]
fn pronunciation_syllables(
    py: Python<'_>,
    phonemes: PyRef<'_, PyPhonemes>,
) -> PyResult<Py<pyo3::types::PyTuple>> {
    let values = jbotci_morphology::pronunciation_syllables(&phonemes.value)
        .map_err(InvalidInputError::new_err)?;
    crate::support::sequence_to_tuple(py, values).map(Bound::unbind)
}

/// Strip a Lojban diacritic from one Unicode scalar when defined.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_strip_lojban_diacritic")]
fn strip_lojban_diacritic(value: &str) -> PyResult<Option<String>> {
    Ok(
        jbotci_morphology::strip_lojban_diacritic(one_unicode_scalar(value, "value")?)
            .map(|value| value.to_string()),
    )
}

/// Fold a Lojban diacritic on one Unicode scalar when defined.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_fold_lojban_diacritic")]
fn fold_lojban_diacritic(value: &str) -> PyResult<Option<String>> {
    Ok(
        jbotci_morphology::fold_lojban_diacritic(one_unicode_scalar(value, "value")?)
            .map(|value| value.to_string()),
    )
}

/// Strip Lojban-specific diacritics from every character in text.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_strip_lojban_diacritics")]
fn strip_lojban_diacritics(text: &str) -> String {
    jbotci_morphology::strip_lojban_diacritics(text)
}
/// Fold Lojban-specific diacritics throughout text.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_fold_lojban_diacritics")]
fn fold_lojban_diacritics(text: &str) -> String {
    jbotci_morphology::fold_lojban_diacritics(text)
}
/// Compare strings after stripping Lojban-specific diacritics.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_stripped_lojban_diacritics_eq")]
fn stripped_lojban_diacritics_eq(left: &str, right: &str) -> bool {
    jbotci_morphology::stripped_lojban_diacritics_eq(left, right)
}
/// Compare strings after folding Lojban-specific diacritics.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_folded_lojban_diacritics_eq")]
fn folded_lojban_diacritics_eq(left: &str, right: &str) -> bool {
    jbotci_morphology::folded_lojban_diacritics_eq(left, right)
}
/// Strip all supported diacritics from text.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_strip_diacritics")]
fn strip_diacritics(text: &str) -> String {
    jbotci_morphology::strip_diacritics(text)
}
/// Compare strings after stripping all supported diacritics.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_strip_diacritics_eq")]
fn strip_diacritics_eq(left: &str, right: &str) -> bool {
    jbotci_morphology::strip_diacritics_eq(left, right)
}

/// Test whether one Unicode scalar is a valid canonical phoneme.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_is_valid_phoneme")]
fn is_valid_phoneme(value: &str) -> PyResult<bool> {
    Ok(jbotci_morphology::is_valid_phoneme(one_unicode_scalar(
        value, "value",
    )?))
}
/// Test whether one Unicode scalar forms morphology words under the options.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_is_word_forming_character", signature = (value, *, options=None))]
fn is_word_forming_character(
    value: &str,
    options: Option<PyRef<'_, PyMorphologyOptions>>,
) -> PyResult<bool> {
    let value = one_unicode_scalar(value, "value")?;
    let options = rust_options(options.as_deref());
    Ok(jbotci_morphology::is_word_forming_character_with_options(
        value, &options,
    ))
}
/// Test whether one Unicode scalar is a recognized period character.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_is_period_character")]
fn is_period_character(value: &str) -> PyResult<bool> {
    Ok(jbotci_morphology::is_period_character(one_unicode_scalar(
        value, "value",
    )?))
}
/// Test whether one Unicode scalar may be ignored by the permissive lexer.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_is_permissive_ignorable_character")]
fn is_permissive_ignorable_character(value: &str) -> PyResult<bool> {
    Ok(jbotci_morphology::is_permissive_ignorable_character(
        one_unicode_scalar(value, "value")?,
    ))
}

/// Parse a lujvo spelling into its typed rafsi and hyphen components.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_parse_lujvo_parts")]
fn parse_lujvo_parts(py: Python<'_>, word: String) -> PyResult<Option<Py<pyo3::types::PyTuple>>> {
    py.detach(|| jbotci_morphology::parse_lujvo_word_parts(&word))
        .map(|parts| lujvo_parts_to_tuple(py, parts))
        .transpose()
}
/// Parse a cmevla lujvo spelling into typed components.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_parse_cmevla_lujvo_parts")]
fn parse_cmevla_lujvo_parts(
    py: Python<'_>,
    word: String,
) -> PyResult<Option<Py<pyo3::types::PyTuple>>> {
    py.detach(|| jbotci_morphology::parse_cmevla_lujvo_word_parts(&word))
        .map(|parts| lujvo_parts_to_tuple(py, parts))
        .transpose()
}
/// Return every typed component parse candidate for a cmevla lujvo.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_parse_cmevla_lujvo_part_candidates")]
fn parse_cmevla_lujvo_part_candidates(
    py: Python<'_>,
    word: String,
) -> PyResult<Py<pyo3::types::PyTuple>> {
    let values = py
        .detach(|| jbotci_morphology::parse_cmevla_lujvo_word_part_candidates(&word))
        .into_iter()
        .map(|parts| lujvo_parts_to_tuple(py, parts))
        .collect::<PyResult<Vec<_>>>()?;
    crate::support::sequence_to_tuple(py, values).map(Bound::unbind)
}

/// Bond a rafsi sequence using the Rust lujvo morphology rules.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_bond_rafsis")]
fn bond_rafsis(py: Python<'_>, rafsis: Vec<String>) -> PyResult<Option<Py<pyo3::types::PyTuple>>> {
    py.detach(|| jbotci_morphology::bond_rafsis(&rafsis))
        .map(|values| crate::support::sequence_to_tuple(py, values).map(Bound::unbind))
        .transpose()
}
/// Test whether text is a valid constructed-lujvo candidate spelling.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_is_valid_lujvo_candidate_word")]
fn is_valid_lujvo_candidate_word(word: &str) -> bool {
    jbotci_morphology::is_valid_lujvo_candidate_word(word)
}
/// Add the leading pause required to make text a cmevla when needed.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_ensure_cmevla_word")]
fn ensure_cmevla_word(word: &str) -> String {
    jbotci_morphology::ensure_cmevla_word(word)
}
/// Test whether text ends with a Lojban consonant.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_ends_with_consonant")]
fn ends_with_consonant(word: &str) -> bool {
    jbotci_morphology::ends_with_consonant(word)
}
/// Test whether text ends with a Lojban vowel.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_ends_with_vowel")]
fn ends_with_vowel(word: &str) -> bool {
    jbotci_morphology::ends_with_vowel(word)
}
/// Test whether a lujvo component is a bonding hyphen.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_is_bonding_hyphen")]
fn is_bonding_hyphen(part: &str) -> bool {
    jbotci_morphology::is_bonding_hyphen(part)
}
/// Return the consonant/vowel syllable pattern for valid text.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_syllables_pattern")]
fn syllables_pattern(text: &str) -> Option<String> {
    jbotci_morphology::syllables_pattern(text)
}
/// Classify the structural shape of a rafsi spelling.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_rafsi_shape")]
fn rafsi_shape(py: Python<'_>, text: &str) -> PyResult<Py<PyAny>> {
    enum_to_python(py, jbotci_morphology::rafsi_shape(text))
}
/// Return the standard lujvo score contribution for a rafsi shape.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_rafsi_shape_score")]
fn rafsi_shape_score(py: Python<'_>, shape: &Bound<'_, PyAny>) -> PyResult<i32> {
    Ok(enum_from_python::<RafsiShape>(py, shape)?.score())
}
/// Test whether one Unicode scalar is a Lojban vowel.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_is_vowel")]
fn is_vowel(value: &str) -> PyResult<bool> {
    Ok(jbotci_morphology::is_vowel(one_unicode_scalar(
        value, "value",
    )?))
}
/// Test whether one Unicode scalar is a Lojban consonant.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_is_consonant")]
fn is_consonant(value: &str) -> PyResult<bool> {
    Ok(jbotci_morphology::is_consonant(one_unicode_scalar(
        value, "value",
    )?))
}
/// Test whether text has cmevla surface morphology.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_is_cmevla")]
fn is_cmevla(text: &str) -> bool {
    jbotci_morphology::is_cmevla(text)
}
/// Classify a pair of Unicode scalars by Lojban consonant-pair rules.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_consonant_pair_class")]
fn consonant_pair_class(py: Python<'_>, first: &str, second: &str) -> PyResult<Option<Py<PyAny>>> {
    let first = one_unicode_scalar(first, "first")?;
    let second = one_unicode_scalar(second, "second")?;
    jbotci_morphology::consonant_pair_class(first, second)
        .map(|value| enum_to_python(py, value))
        .transpose()
}
/// Test two Unicode scalars with the exact Rust permissible-pair predicate.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_permissible_consonant_pair")]
fn permissible_consonant_pair(first: &str, second: &str) -> PyResult<bool> {
    Ok(jbotci_morphology::permissible_consonant_pair(
        one_unicode_scalar(first, "first")?,
        one_unicode_scalar(second, "second")?,
    ))
}
/// Report whether a consonant-pair class is permissible.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_consonant_pair_is_permissible")]
fn consonant_pair_is_permissible(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<bool> {
    Ok(enum_from_python::<ConsonantPairClass>(py, value)?.is_permissible())
}
/// Report whether a consonant-pair class is valid word-initially.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_consonant_pair_is_initial")]
fn consonant_pair_is_initial(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<bool> {
    Ok(enum_from_python::<ConsonantPairClass>(py, value)?.is_initial())
}

/// Test whether a parsed word requires a rendered leading pause.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_word_needs_leading_pause")]
fn word_needs_leading_pause(
    py: Python<'_>,
    word: &Bound<'_, PyAny>,
    mode: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    Ok(jbotci_morphology::word_needs_leading_pause(
        word_handle_from_python(word)?.get(),
        enum_from_python::<LeadingPauseVowelMode>(py, mode)?,
    ))
}
/// Test leading-pause requirements in an explicit rendering context.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_word_needs_leading_pause_in_context")]
fn word_needs_leading_pause_in_context(
    py: Python<'_>,
    word: &Bound<'_, PyAny>,
    mode: &Bound<'_, PyAny>,
    context: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    Ok(jbotci_morphology::word_needs_leading_pause_in_context(
        word_handle_from_python(word)?.get(),
        enum_from_python::<LeadingPauseVowelMode>(py, mode)?,
        enum_from_python::<LeadingPauseContext>(py, context)?,
    ))
}
/// Compare two parsed words by syntax identity rather than source location.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_word_syntax_eq")]
fn word_syntax_eq(left: &Bound<'_, PyAny>, right: &Bound<'_, PyAny>) -> PyResult<bool> {
    Ok(jbotci_morphology::word_syntax_eq(
        word_handle_from_python(left)?.get(),
        word_handle_from_python(right)?.get(),
    ))
}
/// Compare two recursive word-like values by syntax identity.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_word_like_syntax_eq")]
fn word_like_syntax_eq(left: &Bound<'_, PyAny>, right: &Bound<'_, PyAny>) -> PyResult<bool> {
    Ok(jbotci_morphology::word_like_syntax_eq(
        extract_word_like(left)?.get(),
        extract_word_like(right)?.get(),
    ))
}

/// Look up the exact typed cmavo variant for canonical-equivalent text.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_cmavo_from_text")]
fn cmavo_from_text(py: Python<'_>, text: &str) -> PyResult<Option<Py<PyAny>>> {
    Cmavo::from_text(text)
        .map(|value| enum_to_python(py, value))
        .transpose()
}
/// Return the canonical spelling of a typed cmavo variant.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_cmavo_text")]
fn cmavo_text(py: Python<'_>, cmavo: &Bound<'_, PyAny>) -> PyResult<&'static str> {
    Ok(enum_from_python::<Cmavo>(py, cmavo)?.canonical_text())
}
/// Test whether a cmavo belongs to a selma'o.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_cmavo_is_selmaho")]
fn cmavo_is_selmaho(
    py: Python<'_>,
    cmavo: &Bound<'_, PyAny>,
    selmaho: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    Ok(enum_from_python::<Cmavo>(py, cmavo)?.is_selmaho(enum_from_python::<Selmaho>(py, selmaho)?))
}
/// Return the primary selma'o of a cmavo when it has one.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_cmavo_primary_selmaho")]
fn cmavo_primary_selmaho(py: Python<'_>, cmavo: &Bound<'_, PyAny>) -> PyResult<Option<Py<PyAny>>> {
    enum_from_python::<Cmavo>(py, cmavo)?
        .primary_selmaho()
        .map(|value| enum_to_python(py, value))
        .transpose()
}
/// Report whether a cmavo opens any morphology quotation form.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_cmavo_is_quote_opener")]
fn cmavo_is_quote_opener(py: Python<'_>, cmavo: &Bound<'_, PyAny>) -> PyResult<bool> {
    Ok(enum_from_python::<Cmavo>(py, cmavo)?.is_quote_opener())
}
/// Report whether a cmavo opens a single-word quotation form.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_cmavo_is_single_word_quote_opener")]
fn cmavo_is_single_word_quote_opener(py: Python<'_>, cmavo: &Bound<'_, PyAny>) -> PyResult<bool> {
    Ok(enum_from_python::<Cmavo>(py, cmavo)?.is_single_word_quote_opener())
}
/// Report whether a cmavo opens a delimiter-based non-Lojban quote.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_cmavo_is_delimited_non_lojban_quote_opener")]
fn cmavo_is_delimited_non_lojban_quote_opener(
    py: Python<'_>,
    cmavo: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    Ok(enum_from_python::<Cmavo>(py, cmavo)?.is_delimited_non_lojban_quote_opener())
}
/// Look up a typed selma'o by its exact Rust name.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(name = "_morphology_selmaho_from_name")]
fn selmaho_from_name(py: Python<'_>, name: &str) -> PyResult<Option<Py<PyAny>>> {
    if name.is_empty() {
        return Err(InvalidInputError::new_err("selma'o name must not be empty"));
    }
    Selmaho::from_name(name)
        .map(|value| enum_to_python(py, value))
        .transpose()
}
/// Return the exact Rust name of a typed selma'o.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_selmaho_name")]
fn selmaho_name(py: Python<'_>, selmaho: &Bound<'_, PyAny>) -> PyResult<&'static str> {
    Ok(enum_from_python::<Selmaho>(py, selmaho)?.name())
}
/// Test whether a selma'o contains a cmavo.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_selmaho_contains")]
fn selmaho_contains(
    py: Python<'_>,
    selmaho: &Bound<'_, PyAny>,
    cmavo: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    Ok(enum_from_python::<Selmaho>(py, selmaho)?.contains(enum_from_python::<Cmavo>(py, cmavo)?))
}

/// Rafsi input part for lujvo candidate construction.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "LujvoRafsiBuildPart",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyLujvoRafsiBuildPart {
    value: LujvoBuildPart,
}

#[pymethods]
impl PyLujvoRafsiBuildPart {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("text",);
    /// Construct a non-empty rafsi build part.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(text: String) -> PyResult<Self> {
        if text.is_empty() {
            return Err(InvalidInputError::new_err(
                "lujvo build part text must not be empty",
            ));
        }
        Ok(PyLujvoRafsiBuildPart {
            value: new!(LujvoBuildPart::Rafsi(text)),
        })
    }
    /// Return the build-part text.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn text(&self) -> &str {
        self.value.as_text()
    }
}

/// Full brivla-core input part for lujvo candidate construction.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "LujvoBrivlaCoreBuildPart",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyLujvoBrivlaCoreBuildPart {
    value: LujvoBuildPart,
}

#[pymethods]
impl PyLujvoBrivlaCoreBuildPart {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("text",);
    /// Construct a non-empty brivla-core build part.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(text: String) -> PyResult<Self> {
        if text.is_empty() {
            return Err(InvalidInputError::new_err(
                "lujvo build part text must not be empty",
            ));
        }
        Ok(PyLujvoBrivlaCoreBuildPart {
            value: new!(LujvoBuildPart::BrivlaCore(text)),
        })
    }
    /// Return the build-part text.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn text(&self) -> &str {
        self.value.as_text()
    }
}

#[requires(true)]
#[ensures(true)]
fn lujvo_build_part_from_python(value: &Bound<'_, PyAny>) -> PyResult<LujvoBuildPart> {
    if let Ok(value) = value.extract::<PyRef<'_, PyLujvoRafsiBuildPart>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyLujvoBrivlaCoreBuildPart>>() {
        return Ok(value.value.clone());
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "expected LujvoRafsiBuildPart or LujvoBrivlaCoreBuildPart",
    ))
}

/// Scored lujvo candidate with its selected surface parts.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "LujvoCandidate",
    frozen,
    eq,
    module = "jbotci.morphology",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyLujvoCandidate {
    value: LujvoCandidate,
}

impl PyLujvoCandidate {
    #[requires(!value.word.is_empty() && !value.parts.is_empty())]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from_rust(value: LujvoCandidate) -> Self {
        PyLujvoCandidate { value }
    }
}

#[pymethods]
impl PyLujvoCandidate {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) = ("word", "parts", "score");
    /// Construct a scored non-empty lujvo candidate.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(word: String, parts: Vec<String>, score: i32) -> PyResult<Self> {
        if word.is_empty() || parts.is_empty() {
            return Err(InvalidInputError::new_err(
                "lujvo candidate word and parts collection must be non-empty",
            ));
        }
        Ok(Self::from_rust(new!(LujvoCandidate { word, parts, score })))
    }
    /// Return the candidate word spelling.
    #[requires(true)]
    #[ensures(ret == self.value.word.as_str())]
    #[getter]
    fn word(&self) -> &str {
        &self.value.word
    }
    /// Return the immutable selected part spellings.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn parts(&self, py: Python<'_>) -> PyResult<Py<pyo3::types::PyTuple>> {
        crate::support::sequence_to_tuple(py, self.value.parts.iter().cloned()).map(Bound::unbind)
    }
    /// Return the standard lujvo score.
    #[requires(true)]
    #[ensures(ret == self.value.score)]
    #[getter]
    fn score(&self) -> i32 {
        self.value.score
    }
}

/// Choose the best scored lujvo candidate from textual rafsi choices.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_choose_best_lujvo_candidate")]
fn choose_best_lujvo_candidate(
    py: Python<'_>,
    mode: &Bound<'_, PyAny>,
    choices: Vec<Vec<String>>,
) -> PyResult<Option<PyLujvoCandidate>> {
    let mode = enum_from_python(py, mode)?;
    Ok(py
        .detach(|| jbotci_morphology::choose_best_lujvo_candidate(mode, &choices))
        .map(PyLujvoCandidate::from_rust))
}

/// Choose the best scored lujvo candidate from typed build-part choices.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_morphology_choose_best_lujvo_candidate_from_parts")]
fn choose_best_lujvo_candidate_from_parts(
    py: Python<'_>,
    mode: &Bound<'_, PyAny>,
    choices: &Bound<'_, PyAny>,
) -> PyResult<Option<PyLujvoCandidate>> {
    let mode = enum_from_python(py, mode)?;
    let choices = extract_sequence(choices, "choices", |choice| {
        extract_sequence(choice, "each lujvo choice", lujvo_build_part_from_python)
    })?;
    Ok(py
        .detach(|| jbotci_morphology::choose_best_lujvo_candidate_from_parts(mode, &choices))
        .map(PyLujvoCandidate::from_rust))
}

macro_rules! register_function {
    ($module:expr, $name:literal, $function:ident) => {
        register_private_object($module, $name, wrap_pyfunction!($function, $module)?)?;
    };
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    register_private_object(
        module,
        "_morphology_MORPHOLOGY_TRACE_FILTERS",
        sequence_to_tuple(module.py(), MORPHOLOGY_TRACE_FILTERS.iter().copied())?,
    )?;
    register_private_object(
        module,
        "_morphology_PERMISSIVE_IGNORABLE_RESERVED_CHARACTERS",
        sequence_to_tuple(
            module.py(),
            PERMISSIVE_IGNORABLE_RESERVED_CHARACTERS
                .iter()
                .map(|character| character.to_string()),
        )?,
    )?;
    register_string_enum::<WordKind>(module)?;
    register_string_enum::<ValsiAnalysisStatus>(module)?;
    register_string_enum::<ValsiClassificationKind>(module)?;
    register_string_enum::<ValsiLujvoPartKind>(module)?;
    register_string_enum::<ValsiLujvoRafsiKind>(module)?;
    register_string_enum::<ValsiFuhivlaStage>(module)?;
    register_string_enum::<StressMark>(module)?;
    register_string_enum::<GlideMark>(module)?;
    register_string_enum::<MorphologyErrorKind>(module)?;
    register_string_enum::<MorphologyWarningKind>(module)?;
    register_string_enum::<MorphologyContextKind>(module)?;
    register_string_enum::<LujvoParseExpectation>(module)?;
    register_string_enum::<ExpectedWordDetailKind>(module)?;
    register_string_enum::<ZoiDelimiterDetailKind>(module)?;
    register_string_enum::<PhonotacticDetailKind>(module)?;
    register_string_enum::<LujvoBuildMode>(module)?;
    register_string_enum::<RafsiShape>(module)?;
    register_string_enum::<ConsonantPairClass>(module)?;
    register_string_enum::<LeadingPauseVowelMode>(module)?;
    register_string_enum::<LeadingPauseContext>(module)?;
    register_string_enum::<Cmavo>(module)?;
    register_string_enum::<Selmaho>(module)?;

    register_type::<PyPhonemeRenderOptions>(module, "_morphology_PhonemeRenderOptions")?;
    register_type::<PyPhonemes>(module, "_morphology_Phonemes")?;
    register_type::<PyWordKey>(module, "_morphology_WordKey")?;
    register_type::<PyInvalidDialectWord>(module, "_morphology_InvalidDialectWord")?;
    register_type::<PyMorphologyOptions>(module, "_morphology_MorphologyOptions")?;
    register_type::<PyCompiledDialectDefinition>(module, "_morphology_CompiledDialectDefinition")?;
    register_type::<PyCompiledDialectSwap>(module, "_morphology_CompiledDialectSwap")?;
    register_type::<PyCompiledDialectExpansion>(module, "_morphology_CompiledDialectExpansion")?;
    register_type::<PyCompiledDialectWord>(module, "_morphology_CompiledDialectWord")?;
    register_type::<PyLujvoRafsi>(module, "_morphology_LujvoRafsi")?;
    register_type::<PyLujvoHyphen>(module, "_morphology_LujvoHyphen")?;
    register_type::<PyVerbatim>(module, "_morphology_Verbatim")?;
    register_type::<PyCmavoWord>(module, "_morphology_CmavoWord")?;
    register_type::<PyGismuWord>(module, "_morphology_GismuWord")?;
    register_type::<PyLujvoWord>(module, "_morphology_LujvoWord")?;
    register_type::<PyFuhivlaWord>(module, "_morphology_FuhivlaWord")?;
    register_type::<PyCmevlaWord>(module, "_morphology_CmevlaWord")?;
    register_type::<PyPlainWord>(module, "_morphology_PlainWord")?;
    register_type::<PyQuotedWord>(module, "_morphology_QuotedWord")?;
    register_type::<PySelmahoQuotedWord>(module, "_morphology_SelmahoQuotedWord")?;
    register_type::<PyDelimitedNonLojbanQuote>(module, "_morphology_DelimitedNonLojbanQuote")?;
    register_type::<PyQuotedWords>(module, "_morphology_QuotedWords")?;
    register_type::<PyDelimitedWordQuote>(module, "_morphology_DelimitedWordQuote")?;
    register_type::<PyLerfuWord>(module, "_morphology_LerfuWord")?;
    register_type::<PyZeiCompound>(module, "_morphology_ZeiCompound")?;
    register_type::<PyMorphologyContext>(module, "_morphology_MorphologyContext")?;
    register_type::<PyMorphologyWarning>(module, "_morphology_MorphologyWarning")?;
    register_type::<PyInvalidLujvoDetail>(module, "_morphology_InvalidLujvoDetail")?;
    register_type::<PyFuhivlaContainsYDetail>(module, "_morphology_FuhivlaContainsYDetail")?;
    register_type::<PySlinkuhiDetail>(module, "_morphology_SlinkuhiDetail")?;
    register_type::<PyExpectedWordDetail>(module, "_morphology_ExpectedWordDetail")?;
    register_type::<PyInvalidZoiDelimiterDetail>(module, "_morphology_InvalidZoiDelimiterDetail")?;
    register_type::<PyPhonotacticDetail>(module, "_morphology_PhonotacticDetail")?;
    register_type::<PyInvalidMorphology>(module, "_morphology_InvalidMorphology")?;
    register_type::<PyUnterminatedZoiQuote>(module, "_morphology_UnterminatedZoiQuote")?;
    register_type::<PySourceSpanMorphologyError>(module, "_morphology_SourceSpanMorphologyError")?;
    register_type::<PyMorphologySegmentAttempt>(module, "_morphology_MorphologySegmentAttempt")?;
    register_type::<PyRecoveredMorphologySegmentation>(
        module,
        "_morphology_RecoveredMorphologySegmentation",
    )?;
    register_type::<PyRecoveredMorphologySegmentAttempt>(
        module,
        "_morphology_RecoveredMorphologySegmentAttempt",
    )?;
    register_type::<PyValsiLujvoPart>(module, "_morphology_ValsiLujvoPart")?;
    register_type::<PyPlainWordClassification>(module, "_morphology_PlainWordClassification")?;
    register_type::<PyPlainWordValsiClassification>(
        module,
        "_morphology_PlainWordValsiClassification",
    )?;
    register_type::<PyQuotedWordValsiClassification>(
        module,
        "_morphology_QuotedWordValsiClassification",
    )?;
    register_type::<PyDelimitedNonLojbanQuoteValsiClassification>(
        module,
        "_morphology_DelimitedNonLojbanQuoteValsiClassification",
    )?;
    register_type::<PyQuotedWordsValsiClassification>(
        module,
        "_morphology_QuotedWordsValsiClassification",
    )?;
    register_type::<PyDelimitedWordQuoteValsiClassification>(
        module,
        "_morphology_DelimitedWordQuoteValsiClassification",
    )?;
    register_type::<PyLerfuWordValsiClassification>(
        module,
        "_morphology_LerfuWordValsiClassification",
    )?;
    register_type::<PyZeiCompoundValsiClassification>(
        module,
        "_morphology_ZeiCompoundValsiClassification",
    )?;
    register_type::<PyValsiAnalysisResult>(module, "_morphology_ValsiAnalysisResult")?;
    register_type::<PyValsiAnalysis>(module, "_morphology_ValsiAnalysis")?;
    register_type::<PyLujvoRafsiBuildPart>(module, "_morphology_LujvoRafsiBuildPart")?;
    register_type::<PyLujvoBrivlaCoreBuildPart>(module, "_morphology_LujvoBrivlaCoreBuildPart")?;
    register_type::<PyLujvoCandidate>(module, "_morphology_LujvoCandidate")?;

    register_function!(module, "_morphology_segment_attempt", segment_attempt);
    register_function!(
        module,
        "_morphology_segment_recovered_attempt",
        segment_recovered_attempt
    );
    register_function!(
        module,
        "_morphology_segment_for_display_attempt",
        segment_for_display_attempt
    );
    register_function!(module, "_morphology_analyze_valsi", analyze_valsi);
    register_function!(module, "_morphology_normalize_input", normalize_input);
    register_function!(module, "_morphology_canonicalize_text", canonicalize_text);
    register_function!(module, "_morphology_canonical_text_eq", canonical_text_eq);
    register_function!(
        module,
        "_morphology_canonical_text_is_all",
        canonical_text_is_all
    );
    register_function!(
        module,
        "_morphology_normalize_cmavo_form",
        normalize_cmavo_form
    );
    register_function!(module, "_morphology_cmavo_phonemes", cmavo_phonemes);
    register_function!(
        module,
        "_morphology_pronunciation_syllables",
        pronunciation_syllables
    );
    register_function!(
        module,
        "_morphology_strip_lojban_diacritic",
        strip_lojban_diacritic
    );
    register_function!(
        module,
        "_morphology_fold_lojban_diacritic",
        fold_lojban_diacritic
    );
    register_function!(
        module,
        "_morphology_strip_lojban_diacritics",
        strip_lojban_diacritics
    );
    register_function!(
        module,
        "_morphology_fold_lojban_diacritics",
        fold_lojban_diacritics
    );
    register_function!(
        module,
        "_morphology_stripped_lojban_diacritics_eq",
        stripped_lojban_diacritics_eq
    );
    register_function!(
        module,
        "_morphology_folded_lojban_diacritics_eq",
        folded_lojban_diacritics_eq
    );
    register_function!(module, "_morphology_strip_diacritics", strip_diacritics);
    register_function!(
        module,
        "_morphology_strip_diacritics_eq",
        strip_diacritics_eq
    );
    register_function!(module, "_morphology_is_valid_phoneme", is_valid_phoneme);
    register_function!(
        module,
        "_morphology_is_word_forming_character",
        is_word_forming_character
    );
    register_function!(
        module,
        "_morphology_is_period_character",
        is_period_character
    );
    register_function!(
        module,
        "_morphology_is_permissive_ignorable_character",
        is_permissive_ignorable_character
    );
    register_function!(module, "_morphology_parse_lujvo_parts", parse_lujvo_parts);
    register_function!(
        module,
        "_morphology_parse_cmevla_lujvo_parts",
        parse_cmevla_lujvo_parts
    );
    register_function!(
        module,
        "_morphology_parse_cmevla_lujvo_part_candidates",
        parse_cmevla_lujvo_part_candidates
    );
    register_function!(module, "_morphology_bond_rafsis", bond_rafsis);
    register_function!(
        module,
        "_morphology_is_valid_lujvo_candidate_word",
        is_valid_lujvo_candidate_word
    );
    register_function!(module, "_morphology_ensure_cmevla_word", ensure_cmevla_word);
    register_function!(
        module,
        "_morphology_ends_with_consonant",
        ends_with_consonant
    );
    register_function!(module, "_morphology_ends_with_vowel", ends_with_vowel);
    register_function!(module, "_morphology_is_bonding_hyphen", is_bonding_hyphen);
    register_function!(module, "_morphology_syllables_pattern", syllables_pattern);
    register_function!(module, "_morphology_rafsi_shape", rafsi_shape);
    register_function!(module, "_morphology_rafsi_shape_score", rafsi_shape_score);
    register_function!(module, "_morphology_is_vowel", is_vowel);
    register_function!(module, "_morphology_is_consonant", is_consonant);
    register_function!(module, "_morphology_is_cmevla", is_cmevla);
    register_function!(
        module,
        "_morphology_consonant_pair_class",
        consonant_pair_class
    );
    register_function!(
        module,
        "_morphology_permissible_consonant_pair",
        permissible_consonant_pair
    );
    register_function!(
        module,
        "_morphology_consonant_pair_is_permissible",
        consonant_pair_is_permissible
    );
    register_function!(
        module,
        "_morphology_consonant_pair_is_initial",
        consonant_pair_is_initial
    );
    register_function!(
        module,
        "_morphology_word_needs_leading_pause",
        word_needs_leading_pause
    );
    register_function!(
        module,
        "_morphology_word_needs_leading_pause_in_context",
        word_needs_leading_pause_in_context
    );
    register_function!(module, "_morphology_word_syntax_eq", word_syntax_eq);
    register_function!(
        module,
        "_morphology_word_like_syntax_eq",
        word_like_syntax_eq
    );
    register_function!(module, "_morphology_cmavo_from_text", cmavo_from_text);
    register_function!(module, "_morphology_cmavo_text", cmavo_text);
    register_function!(module, "_morphology_cmavo_is_selmaho", cmavo_is_selmaho);
    register_function!(
        module,
        "_morphology_cmavo_primary_selmaho",
        cmavo_primary_selmaho
    );
    register_function!(
        module,
        "_morphology_cmavo_is_quote_opener",
        cmavo_is_quote_opener
    );
    register_function!(
        module,
        "_morphology_cmavo_is_single_word_quote_opener",
        cmavo_is_single_word_quote_opener
    );
    register_function!(
        module,
        "_morphology_cmavo_is_delimited_non_lojban_quote_opener",
        cmavo_is_delimited_non_lojban_quote_opener
    );
    register_function!(module, "_morphology_selmaho_from_name", selmaho_from_name);
    register_function!(module, "_morphology_selmaho_name", selmaho_name);
    register_function!(module, "_morphology_selmaho_contains", selmaho_contains);
    register_function!(
        module,
        "_morphology_choose_best_lujvo_candidate",
        choose_best_lujvo_candidate
    );
    register_function!(
        module,
        "_morphology_choose_best_lujvo_candidate_from_parts",
        choose_best_lujvo_candidate_from_parts
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{diagnostics, dialect, source};
    use pyo3::types::PyDict;

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn compiled_dialect_storage_variants_retain_their_arc_roots() {
        let owned_root = Arc::new(CompiledDialectDefinition::default());
        let owned = CompiledDialectStorage::Owned {
            value: Arc::clone(&owned_root),
        };
        drop(owned_root);
        assert!(owned.get().entries.is_empty());

        let options_root = Arc::new(MorphologyOptions::default());
        let options = CompiledDialectStorage::Options {
            value: Arc::clone(&options_root),
        };
        drop(options_root);
        assert!(options.get().entries.is_empty());
    }

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn registered_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
        let module = PyModule::new(py, "jbotci._native")?;
        source::register(&module)?;
        diagnostics::register(&module)?;
        dialect::register(&module)?;
        register(&module)?;
        Ok(module)
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn bound_segmentation_projection_retains_literal_payloads() {
        Python::initialize();
        Python::attach(|py| -> PyResult<()> {
            let module = registered_module(py)?;
            let function = module.getattr("_morphology_segment_attempt")?;

            let projected = function.call1(("mimi",))?;
            let projected = projected.extract::<PyRef<'_, PyMorphologySegmentAttempt>>()?;
            assert_eq!(projected.source.as_ref(), "mimi");
            assert_eq!(projected.source_id, None);
            assert!(projected.warnings.is_empty());
            assert_eq!(projected.trace, None);
            let SegmentOutcome::Words { values } = &projected.outcome else {
                panic!("mimi must project as successful segmentation")
            };
            assert_eq!(values.len(), 2);
            for (word_like, expected_start) in values.iter().zip([0, 2]) {
                let word = word_like
                    .bare_word()
                    .expect("each projected mimi segment is a plain word");
                assert_eq!(word.kind(), WordKind::Cmavo);
                assert_eq!(word.phonemes().as_str(), "mi");
                assert_eq!(word.span().byte_start, expected_start);
                assert_eq!(word.span().byte_end, expected_start + 2);
                assert_eq!(word.span().char_start, expected_start);
                assert_eq!(word.span().char_end, expected_start + 2);
            }

            let projected = function.call1(("aa",))?;
            let projected = projected.extract::<PyRef<'_, PyMorphologySegmentAttempt>>()?;
            let SegmentOutcome::Error { value } = &projected.outcome else {
                panic!("aa must project as a morphology error")
            };
            let RustMorphologyError::Invalid {
                kind,
                char_start,
                char_end,
                text,
                context,
                detail,
            } = value.as_ref()
            else {
                panic!("aa must project as InvalidMorphology")
            };
            assert_eq!(*kind, MorphologyErrorKind::VowelHiatus);
            assert_eq!((*char_start, *char_end), (0, 2));
            assert_eq!(text, "aa");
            let data!(MorphologyContext {
                kind,
                char_start,
                char_end,
            }) = context
                .as_ref()
                .expect("vowel-hiatus projection must retain its fu'ivla context")
                .as_data();
            assert_eq!(*kind, MorphologyContextKind::Fuhivla);
            assert_eq!(*char_start, 0);
            assert_eq!(*char_end, 2);
            let data!(MorphologyErrorDetail::Phonotactic { reason }) = detail
                .as_ref()
                .expect("vowel-hiatus projection must retain phonotactic detail")
                .as_data()
            else {
                panic!("vowel-hiatus projection must retain phonotactic detail")
            };
            assert_eq!(*reason, PhonotacticDetailKind::VowelHiatus);
            Ok(())
        })
        .unwrap();
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn bound_recovery_projection_retains_literal_payloads() {
        Python::initialize();
        Python::attach(|py| -> PyResult<()> {
            let input = "mi @@@ do";
            let module = registered_module(py)?;
            let projected = module
                .getattr("_morphology_segment_recovered_attempt")?
                .call1((input,))?;
            let projected =
                projected.extract::<PyRef<'_, PyRecoveredMorphologySegmentAttempt>>()?;
            assert_eq!(projected.source.as_ref(), "mi @@@ do");
            assert_eq!(projected.source_id, None);
            assert_eq!(projected.trace, None);
            assert_eq!(projected.result.words.len(), 2);
            assert_eq!(projected.result.errors.len(), 1);
            assert_eq!(projected.result.error_regions.len(), 1);
            assert!(projected.result.warnings.is_empty());
            let region = &projected.result.error_regions[0];
            assert_eq!((region.byte_start, region.byte_end), (3, 7));
            assert_eq!((region.char_start, region.char_end), (3, 7));
            let RustMorphologyError::Invalid {
                kind,
                char_start,
                char_end,
                text,
                ..
            } = projected.result.errors[0].as_ref()
            else {
                panic!("recovery must retain an InvalidMorphology payload")
            };
            assert_eq!(*kind, MorphologyErrorKind::InvalidCharacter);
            assert_eq!((*char_start, *char_end), (3, 4));
            assert_eq!(text, "@");
            Ok(())
        })
        .unwrap();
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn bound_valsi_projection_retains_literal_payloads() {
        Python::initialize();
        Python::attach(|py| -> PyResult<()> {
            let module = registered_module(py)?;
            let function = module.getattr("_morphology_analyze_valsi")?;

            let projected = function.call1(("klama",))?;
            let projected = projected.extract::<PyRef<'_, PyValsiAnalysis>>()?;
            assert_eq!(projected.input.as_ref(), "klama");
            assert!(projected.warnings.is_empty());
            assert_eq!(projected.result.status, ValsiAnalysisStatus::Valid);
            assert_eq!(projected.result.error, None);
            assert!(projected.result.words.is_empty());
            let word_like = projected
                .result
                .word
                .as_ref()
                .expect("valid klama analysis must retain its word");
            let word = word_like
                .bare_word()
                .expect("klama analysis must project a plain word");
            assert_eq!(word.kind(), WordKind::Gismu);
            assert_eq!(word.phonemes().as_str(), "kláma");
            assert_eq!((word.span().byte_start, word.span().byte_end), (0, 5));
            assert_eq!((word.span().char_start, word.span().char_end), (0, 5));
            let classification = projected
                .result
                .classification
                .as_ref()
                .expect("valid klama analysis must retain its classification");
            assert_eq!(classification.kind(), ValsiClassificationKind::PlainWord);
            let classification = classification
                .word()
                .expect("klama classification must be a plain word");
            assert_eq!(classification.category, WordKind::Gismu);
            assert_eq!(classification.phonemes, "kláma");
            assert_eq!(classification.selmaho, None);
            assert_eq!(classification.split, None);
            assert!(classification.parts.is_empty());
            assert_eq!(classification.stage, None);
            Ok(())
        })
        .unwrap();
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn bound_display_attempt_projection_retains_warning_and_trace_literals() {
        Python::initialize();
        Python::attach(|py| -> PyResult<()> {
            let module = registered_module(py)?;
            let function = module.getattr("_morphology_segment_for_display_attempt")?;
            let options = MorphologyOptions {
                permissive_lexer: true,
                trace: jbotci_diagnostics::TraceOptions::enabled(
                    jbotci_diagnostics::TraceLevel::Top,
                    None,
                    jbotci_diagnostics::TracePhase::Morphology,
                    jbotci_diagnostics::DEFAULT_TRACE_LIMIT,
                ),
                ..MorphologyOptions::default()
            };
            let options = Py::new(py, PyMorphologyOptions::from_rust(options))?;
            let kwargs = PyDict::new(py);
            kwargs.set_item("options", options)?;

            let projected = function.call(("xu@no",), Some(&kwargs))?;
            let projected = projected.extract::<PyRef<'_, PyMorphologySegmentAttempt>>()?;
            assert_eq!(projected.source.as_ref(), "xu@no");
            let SegmentOutcome::Words { values } = &projected.outcome else {
                panic!("permissive display segmentation must succeed")
            };
            assert_eq!(values.len(), 2);
            assert_eq!(projected.warnings.len(), 1);
            let warning = &projected.warnings[0];
            assert_eq!(warning.kind, MorphologyWarningKind::IgnoredCharacters);
            assert_eq!((warning.char_start, warning.char_end), (2, 3));
            assert_eq!(warning.text, "@");
            assert_eq!(
                warning.ignored_character_count.map(NonZeroUsize::get),
                Some(1)
            );
            let trace = projected
                .trace
                .as_ref()
                .expect("trace-enabled display segmentation must retain a trace");
            assert_eq!(trace.phase, jbotci_diagnostics::TracePhase::Morphology);
            assert!(!trace.events.is_empty());
            assert!(
                trace
                    .events
                    .iter()
                    .all(|event| event.phase == jbotci_diagnostics::TracePhase::Morphology)
            );
            assert_eq!(trace.failure, None);

            let projected = function.call(("aa",), Some(&kwargs))?;
            let projected = projected.extract::<PyRef<'_, PyMorphologySegmentAttempt>>()?;
            assert!(matches!(
                &projected.outcome,
                SegmentOutcome::Error { value }
                    if matches!(value.as_ref(), RustMorphologyError::Invalid {
                        kind: MorphologyErrorKind::VowelHiatus,
                        char_start: 0,
                        char_end: 2,
                        text,
                        ..
                    } if text == "aa")
            ));
            let trace = projected
                .trace
                .as_ref()
                .expect("trace-enabled display failure must retain a trace");
            assert_eq!(trace.phase, jbotci_diagnostics::TracePhase::Morphology);
            assert!(trace.events.iter().any(|event| {
                event.kind == jbotci_diagnostics::TraceEventKind::MorphologyFailure
            }));
            Ok(())
        })
        .unwrap();
    }
}
