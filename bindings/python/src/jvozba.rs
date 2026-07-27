//! Typed Python projection of jvozba composition and decomposition.

use std::borrow::Cow;

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, new, requires, try_new};
use jbotci_jvozba::{
    JvozbaBuildResult as RustJvozbaBuildResult, JvozbaError as RustJvozbaError,
    JvozbaInput as RustJvozbaInput, JvozbaMode, JvozbaSegment as RustJvozbaSegment,
    JvozbaSegmentKind, LujvoDecomposition as RustLujvoDecomposition,
    LujvoSegmentInfo as RustLujvoSegmentInfo,
};
use jbotci_morphology::LujvoPart;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyTuple};

use crate::InvalidInputError;
use crate::dictionary::PyDictionary;
use crate::morphology::{clone_lujvo_part_from_python, owned_lujvo_part_to_python};
use crate::support::{
    PythonStringEnum, extract_sequence, extract_string_enum, public_exception_with_value,
    register_private_object, register_string_enum, register_type, sequence_to_tuple,
    string_enum_member, string_repr,
};

const PUBLIC_MODULE: &str = "jbotci.jvozba";

/// Ordered inventory of native names owned by this domain.
pub(crate) const NATIVE_EXPORTS: &[&str] = &[
    "_jvozba_JvozbaMode",
    "_jvozba_Word",
    "_jvozba_FixedRafsi",
    "_jvozba_JvozbaSegmentKind",
    "_jvozba_JvozbaSegment",
    "_jvozba_JvozbaBuildResult",
    "_jvozba_LujvoSegmentInfo",
    "_jvozba_LujvoDecomposition",
    "_jvozba_RequiresAtLeastTwoInputs",
    "_jvozba_FixedRafsiEmpty",
    "_jvozba_NonFinalUniversalLongRafsi",
    "_jvozba_FinalConsonant",
    "_jvozba_NoRafsiAvailable",
    "_jvozba_NoDictionaryEntry",
    "_jvozba_CouldNotBuildLujvo",
    "_jvozba_CouldNotBuildCompound",
    "_jvozba_build_best_jvozba_detailed",
    "_jvozba_word_can_enter_jvozba_pane",
    "_jvozba_decompose_lujvo_like",
];

macro_rules! define_jvozba_string_enum_binding {
    (
        $type:ty,
        $native_name:literal,
        $python_name:literal,
        $doc:literal,
        { $($variant:path => ($member:literal, $value:literal)),+ $(,)? }
    ) => {
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
                $doc
            }

            fn variants() -> &'static [Self] {
                const VALUES: &[$type] = &[$($variant),+];
                VALUES
            }

            fn python_member_name(self) -> Cow<'static, str> {
                match self {
                    $($variant => Cow::Borrowed($member)),+
                }
            }

            fn python_value(self) -> &'static str {
                match self {
                    $($variant => $value),+
                }
            }
        }
    };
}

define_jvozba_string_enum_binding!(
    JvozbaMode,
    "_jvozba_JvozbaMode",
    "JvozbaMode",
    "Composition mode for a brivla lujvo or cmevla-like compound.",
    {
        JvozbaMode::Lujvo => ("LUJVO", "lujvo"),
        JvozbaMode::Cmevla => ("CMEVLA", "cmevla"),
    }
);

define_jvozba_string_enum_binding!(
    JvozbaSegmentKind,
    "_jvozba_JvozbaSegmentKind",
    "JvozbaSegmentKind",
    "Surface role of one composed jvozba segment.",
    {
        JvozbaSegmentKind::Rafsi => ("RAFSI", "rafsi"),
        JvozbaSegmentKind::Hyphen => ("HYPHEN", "hyphen"),
    }
);

#[requires(true)]
#[ensures(true)]
fn native_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    py.import("jbotci._native")
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn enum_from_python<E: PythonStringEnum>(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<E> {
    extract_string_enum(&native_module(py)?, value)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn enum_to_python<E: PythonStringEnum>(py: Python<'_>, value: E) -> PyResult<Py<PyAny>> {
    string_enum_member(&native_module(py)?, value).map(Bound::unbind)
}

/// Dictionary word input whose rafsi are selected by Rust.
#[invariant(true, "every string is a representable jvozba word input")]
#[pyclass(
    name = "Word",
    frozen,
    eq,
    hash,
    module = "jbotci.jvozba",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PyJvozbaWord {
    value: String,
}

#[pymethods]
impl PyJvozbaWord {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("value",);

    /// Construct a dictionary-word jvozba input.
    #[requires(true)]
    #[ensures(ret.value == old(value.clone()))]
    #[new]
    fn new(value: String) -> Self {
        Self { value }
    }

    /// Return the supplied word text.
    #[requires(true)]
    #[ensures(ret == self.value.as_str())]
    #[getter]
    fn value(&self) -> &str {
        &self.value
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.Word({})",
            string_repr(py, &self.value)?
        ))
    }
}

/// Exact rafsi input used without dictionary selection.
#[invariant(
    true,
    "empty text remains representable so the Rust builder can return FixedRafsiEmpty"
)]
#[pyclass(
    name = "FixedRafsi",
    frozen,
    eq,
    hash,
    module = "jbotci.jvozba",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PyFixedRafsi {
    value: String,
}

#[pymethods]
impl PyFixedRafsi {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("value",);

    /// Construct an exact fixed-rafsi jvozba input.
    #[requires(true)]
    #[ensures(ret.value == old(value.clone()))]
    #[new]
    fn new(value: String) -> Self {
        Self { value }
    }

    /// Return the supplied rafsi text, which may be empty.
    #[requires(true)]
    #[ensures(ret == self.value.as_str())]
    #[getter]
    fn value(&self) -> &str {
        &self.value
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.FixedRafsi({})",
            string_repr(py, &self.value)?
        ))
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn input_from_python(value: &Bound<'_, PyAny>) -> PyResult<RustJvozbaInput> {
    if let Ok(value) = value.extract::<PyRef<'_, PyJvozbaWord>>() {
        return Ok(RustJvozbaInput::Word(value.value.clone()));
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyFixedRafsi>>() {
        return Ok(RustJvozbaInput::FixedRafsi(value.value.clone()));
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "expected jbotci.jvozba.Word or FixedRafsi",
    ))
}

/// One immutable surface segment of a composed jvozba result.
#[invariant(
    true,
    "the retained Rust value enforces the exact non-empty segment-text invariant"
)]
#[pyclass(
    name = "JvozbaSegment",
    frozen,
    eq,
    module = "jbotci.jvozba",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyJvozbaSegment {
    value: RustJvozbaSegment,
}

impl PyJvozbaSegment {
    #[requires(true)]
    #[ensures(ret.value == old(value.clone()))]
    fn from_rust(value: RustJvozbaSegment) -> Self {
        Self { value }
    }
}

#[pymethods]
impl PyJvozbaSegment {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("kind", "text");

    /// Construct a segment while enforcing the Rust non-empty text invariant.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(py: Python<'_>, kind: &Bound<'_, PyAny>, text: String) -> PyResult<Self> {
        let kind = enum_from_python(py, kind)?;
        let value = try_new!(RustJvozbaSegment { kind, text })
            .map_err(|_| InvalidInputError::new_err("JvozbaSegment text must not be empty"))?;
        Ok(Self::from_rust(value))
    }

    /// Return whether this surface piece is a rafsi or inserted hyphen.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.kind)
    }

    /// Return the exact composed surface text.
    #[requires(true)]
    #[ensures(ret == self.value.text.as_str())]
    #[getter]
    fn text(&self) -> &str {
        &self.value.text
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.JvozbaSegment(kind={PUBLIC_MODULE}.{}.{}, text={})",
            JvozbaSegmentKind::python_type_name(),
            JvozbaSegmentKind::python_member_name(self.value.kind),
            string_repr(py, &self.value.text)?
        ))
    }
}

/// Best composed word and its exact surface segmentation.
#[invariant(
    true,
    "the retained Rust value enforces the exact non-empty result-word invariant"
)]
#[pyclass(
    name = "JvozbaBuildResult",
    frozen,
    eq,
    module = "jbotci.jvozba",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyJvozbaBuildResult {
    value: RustJvozbaBuildResult,
}

impl PyJvozbaBuildResult {
    #[requires(true)]
    #[ensures(ret.value == old(value.clone()))]
    fn from_rust(value: RustJvozbaBuildResult) -> Self {
        Self { value }
    }
}

#[pymethods]
impl PyJvozbaBuildResult {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("word", "segments");

    /// Construct a build result while enforcing the Rust word invariant.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(word: String, segments: &Bound<'_, PyAny>) -> PyResult<Self> {
        let segments = extract_sequence(segments, "segments", |segment| {
            segment
                .extract::<PyRef<'_, PyJvozbaSegment>>()
                .map(|segment| segment.value.clone())
                .map_err(|_| {
                    pyo3::exceptions::PyTypeError::new_err(
                        "segments must contain only JvozbaSegment values",
                    )
                })
        })?;
        let value = try_new!(RustJvozbaBuildResult { word, segments })
            .map_err(|_| InvalidInputError::new_err("JvozbaBuildResult word must not be empty"))?;
        Ok(Self::from_rust(value))
    }

    /// Return the complete composed word.
    #[requires(true)]
    #[ensures(ret == self.value.word.as_str())]
    #[getter]
    fn word(&self) -> &str {
        &self.value.word
    }

    /// Return immutable surface segments in order.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|segments| {
        segments.bind(py).len() == self.value.segments.len()
    }) || ret.is_err())]
    #[getter]
    fn segments(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        Ok(sequence_to_tuple(
            py,
            self.value
                .segments
                .iter()
                .cloned()
                .map(PyJvozbaSegment::from_rust),
        )?
        .unbind())
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.JvozbaBuildResult(word={}, segments={})",
            string_repr(py, &self.value.word)?,
            self.segments(py)?.bind(py).repr()?.to_str()?
        ))
    }
}

#[invariant(!matches!(segment, LujvoPart::Hyphen(_)) || source.is_none())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnedLujvoSegmentInfo {
    segment: LujvoPart,
    source: Option<String>,
}

impl OwnedLujvoSegmentInfo {
    #[requires(true)]
    #[ensures(true)]
    fn from_rust(value: RustLujvoSegmentInfo<'_>) -> Self {
        let data = value.into_data();
        new!(OwnedLujvoSegmentInfo {
            segment: data.segment,
            source: data.source.map(str::to_owned),
        })
    }
}

/// One morphology lujvo part with optional dictionary provenance.
#[invariant(
    true,
    "the owned backing value enforces that a hyphen never has source provenance"
)]
#[pyclass(
    name = "LujvoSegmentInfo",
    frozen,
    eq,
    module = "jbotci.jvozba",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyLujvoSegmentInfo {
    value: OwnedLujvoSegmentInfo,
}

impl PyLujvoSegmentInfo {
    #[requires(true)]
    #[ensures(ret.value == old(value.clone()))]
    fn from_owned(value: OwnedLujvoSegmentInfo) -> Self {
        Self { value }
    }
}

#[pymethods]
impl PyLujvoSegmentInfo {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("segment", "source");

    /// Construct a segment/source pair with the Rust provenance invariant.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (segment, source=None))]
    fn new(segment: &Bound<'_, PyAny>, source: Option<String>) -> PyResult<Self> {
        let segment = clone_lujvo_part_from_python(segment)?;
        if matches!(segment, LujvoPart::Hyphen(_)) && source.is_some() {
            return Err(InvalidInputError::new_err(
                "a hyphen LujvoSegmentInfo must have source=None",
            ));
        }
        Ok(Self::from_owned(new!(OwnedLujvoSegmentInfo {
            segment,
            source,
        })))
    }

    /// Return the exact morphology `LujvoRafsi` or `LujvoHyphen`.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[getter]
    fn segment(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        owned_lujvo_part_to_python(py, self.value.segment.clone())
    }

    /// Return the dictionary source word for a rafsi, when one is known.
    #[requires(true)]
    #[ensures(ret == self.value.source.as_deref())]
    #[getter]
    fn source(&self) -> Option<&str> {
        self.value.source.as_deref()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let (segment_class, phonemes) = match &self.value.segment {
            LujvoPart::Rafsi(phonemes) => ("LujvoRafsi", phonemes.as_str()),
            LujvoPart::Hyphen(phonemes) => ("LujvoHyphen", phonemes.as_str()),
        };
        let source = match &self.value.source {
            Some(source) => string_repr(py, source)?,
            None => "None".to_owned(),
        };
        Ok(format!(
            "{PUBLIC_MODULE}.LujvoSegmentInfo(segment=jbotci.morphology.{segment_class}(jbotci.morphology.Phonemes({})), source={source})",
            string_repr(py, phonemes)?
        ))
    }
}

#[invariant(
    segments
        .iter()
        .filter(|segment| matches!(segment.segment, LujvoPart::Rafsi(_)))
        .count()
        >= 2
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnedLujvoDecomposition {
    segments: Vec<OwnedLujvoSegmentInfo>,
    source_words: Vec<String>,
}

impl OwnedLujvoDecomposition {
    /// Own every dictionary-backed source string before leaving detached Rust work.
    #[requires(true)]
    #[ensures(
        ret.segments
            .iter()
            .filter(|segment| matches!(segment.segment, LujvoPart::Rafsi(_)))
            .count()
            >= 2
    )]
    fn from_rust(value: RustLujvoDecomposition<'_>) -> Self {
        let data = value.into_data();
        new!(OwnedLujvoDecomposition {
            segments: data
                .segments
                .into_iter()
                .map(OwnedLujvoSegmentInfo::from_rust)
                .collect(),
            source_words: data.source_words.into_iter().map(str::to_owned).collect(),
        })
    }
}

/// Immutable morphology-backed decomposition and dictionary provenance.
#[invariant(
    true,
    "the owned backing value enforces the Rust minimum of two rafsi segments"
)]
#[pyclass(
    name = "LujvoDecomposition",
    frozen,
    eq,
    module = "jbotci.jvozba",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyLujvoDecomposition {
    value: OwnedLujvoDecomposition,
}

impl PyLujvoDecomposition {
    #[requires(true)]
    #[ensures(ret.value == old(value.clone()))]
    fn from_owned(value: OwnedLujvoDecomposition) -> Self {
        Self { value }
    }
}

#[pymethods]
impl PyLujvoDecomposition {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("segments", "source_words");

    /// Construct a decomposition with at least two rafsi segments.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(segments: &Bound<'_, PyAny>, source_words: &Bound<'_, PyAny>) -> PyResult<Self> {
        let segments = extract_sequence(segments, "segments", |segment| {
            segment
                .extract::<PyRef<'_, PyLujvoSegmentInfo>>()
                .map(|segment| segment.value.clone())
                .map_err(|_| {
                    pyo3::exceptions::PyTypeError::new_err(
                        "segments must contain only LujvoSegmentInfo values",
                    )
                })
        })?;
        let source_words = extract_sequence(source_words, "source_words", |word| {
            word.extract::<String>().map_err(|_| {
                pyo3::exceptions::PyTypeError::new_err("source_words must contain only strings")
            })
        })?;
        let value = try_new!(OwnedLujvoDecomposition {
            segments,
            source_words,
        })
        .map_err(|_| {
            InvalidInputError::new_err(
                "LujvoDecomposition must contain at least two rafsi segments",
            )
        })?;
        Ok(Self::from_owned(value))
    }

    /// Return immutable segment/source records in surface order.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|segments| {
        segments.bind(py).len() == self.value.segments.len()
    }) || ret.is_err())]
    #[getter]
    fn segments(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        Ok(sequence_to_tuple(
            py,
            self.value
                .segments
                .iter()
                .cloned()
                .map(PyLujvoSegmentInfo::from_owned),
        )?
        .unbind())
    }

    /// Return known dictionary source words in Rust order.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|words| {
        words.bind(py).len() == self.value.source_words.len()
    }) || ret.is_err())]
    #[getter]
    fn source_words(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        Ok(sequence_to_tuple(py, self.value.source_words.iter().map(String::as_str))?.unbind())
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.LujvoDecomposition(segments={}, source_words={})",
            self.segments(py)?.bind(py).repr()?.to_str()?,
            self.source_words(py)?.bind(py).repr()?.to_str()?
        ))
    }
}

/// Failure caused by supplying fewer than two expanded inputs.
#[invariant(true, "private construction fixes the retained Rust error variant")]
#[pyclass(
    name = "RequiresAtLeastTwoInputs",
    frozen,
    eq,
    module = "jbotci.jvozba",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyRequiresAtLeastTwoInputs {
    value: RustJvozbaError,
}

#[pymethods]
impl PyRequiresAtLeastTwoInputs {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: () = ();

    /// Construct the fieldless structured error value.
    #[requires(true)]
    #[ensures(matches!(ret.value, RustJvozbaError::RequiresAtLeastTwoInputs))]
    #[new]
    fn new() -> Self {
        Self {
            value: RustJvozbaError::RequiresAtLeastTwoInputs,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        self.value.to_string()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> &'static str {
        "jbotci.jvozba.RequiresAtLeastTwoInputs()"
    }
}

/// Failure caused by an explicitly empty fixed rafsi.
#[invariant(true, "private construction fixes the retained Rust error variant")]
#[pyclass(
    name = "FixedRafsiEmpty",
    frozen,
    eq,
    module = "jbotci.jvozba",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyFixedRafsiEmpty {
    value: RustJvozbaError,
}

#[pymethods]
impl PyFixedRafsiEmpty {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: () = ();

    /// Construct the fieldless structured error value.
    #[requires(true)]
    #[ensures(matches!(ret.value, RustJvozbaError::FixedRafsiEmpty))]
    #[new]
    fn new() -> Self {
        Self {
            value: RustJvozbaError::FixedRafsiEmpty,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        self.value.to_string()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> &'static str {
        "jbotci.jvozba.FixedRafsiEmpty()"
    }
}

/// Failure caused by a non-final universal long rafsi.
#[invariant(true, "private construction fixes the retained Rust error variant")]
#[pyclass(
    name = "NonFinalUniversalLongRafsi",
    frozen,
    eq,
    module = "jbotci.jvozba",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyNonFinalUniversalLongRafsi {
    value: RustJvozbaError,
}

#[pymethods]
impl PyNonFinalUniversalLongRafsi {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("offending",);

    /// Construct the structured error with its exact offending text.
    #[requires(true)]
    #[ensures(matches!(ret.value, RustJvozbaError::NonFinalUniversalLongRafsi { .. }))]
    #[new]
    fn new(offending: String) -> Self {
        Self {
            value: RustJvozbaError::NonFinalUniversalLongRafsi { offending },
        }
    }

    /// Return the non-final fixed rafsi.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn offending(&self) -> &str {
        let RustJvozbaError::NonFinalUniversalLongRafsi { offending } = &self.value else {
            unreachable!("private class fixes the error variant")
        };
        offending
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        self.value.to_string()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.NonFinalUniversalLongRafsi(offending={})",
            string_repr(py, self.offending())?
        ))
    }
}

/// Failure caused by a final form that cannot end the selected mode.
#[invariant(true, "private construction fixes the retained Rust error variant")]
#[pyclass(
    name = "FinalConsonant",
    frozen,
    eq,
    module = "jbotci.jvozba",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyFinalConsonant {
    value: RustJvozbaError,
}

#[pymethods]
impl PyFinalConsonant {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("offending", "is_fixed_rafsi");

    /// Construct the structured error with exact origin information.
    #[requires(true)]
    #[ensures(matches!(ret.value, RustJvozbaError::FinalConsonant { .. }))]
    #[new]
    fn new(offending: String, is_fixed_rafsi: bool) -> Self {
        Self {
            value: RustJvozbaError::FinalConsonant {
                offending,
                is_fixed_rafsi,
            },
        }
    }

    /// Return the offending source value.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn offending(&self) -> &str {
        let RustJvozbaError::FinalConsonant { offending, .. } = &self.value else {
            unreachable!("private class fixes the error variant")
        };
        offending
    }

    /// Return whether the offending value was a fixed rafsi.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn is_fixed_rafsi(&self) -> bool {
        let RustJvozbaError::FinalConsonant { is_fixed_rafsi, .. } = &self.value else {
            unreachable!("private class fixes the error variant")
        };
        *is_fixed_rafsi
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        self.value.to_string()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let is_fixed_rafsi = if self.is_fixed_rafsi() {
            "True"
        } else {
            "False"
        };
        Ok(format!(
            "{PUBLIC_MODULE}.FinalConsonant(offending={}, is_fixed_rafsi={is_fixed_rafsi})",
            string_repr(py, self.offending())?
        ))
    }
}

/// Failure caused by a dictionary word with no usable rafsi.
#[invariant(true, "private construction fixes the retained Rust error variant")]
#[pyclass(
    name = "NoRafsiAvailable",
    frozen,
    eq,
    module = "jbotci.jvozba",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyNoRafsiAvailable {
    value: RustJvozbaError,
}

#[pymethods]
impl PyNoRafsiAvailable {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("offending",);

    /// Construct the structured error with its exact offending word.
    #[requires(true)]
    #[ensures(matches!(ret.value, RustJvozbaError::NoRafsiAvailable { .. }))]
    #[new]
    fn new(offending: String) -> Self {
        Self {
            value: RustJvozbaError::NoRafsiAvailable { offending },
        }
    }

    /// Return the dictionary word with no available rafsi.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn offending(&self) -> &str {
        let RustJvozbaError::NoRafsiAvailable { offending } = &self.value else {
            unreachable!("private class fixes the error variant")
        };
        offending
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        self.value.to_string()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.NoRafsiAvailable(offending={})",
            string_repr(py, self.offending())?
        ))
    }
}

/// Failure caused by a word absent from the selected dictionary.
#[invariant(true, "private construction fixes the retained Rust error variant")]
#[pyclass(
    name = "NoDictionaryEntry",
    frozen,
    eq,
    module = "jbotci.jvozba",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyNoDictionaryEntry {
    value: RustJvozbaError,
}

#[pymethods]
impl PyNoDictionaryEntry {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("offending",);

    /// Construct the structured error with its exact offending word.
    #[requires(true)]
    #[ensures(matches!(ret.value, RustJvozbaError::NoDictionaryEntry { .. }))]
    #[new]
    fn new(offending: String) -> Self {
        Self {
            value: RustJvozbaError::NoDictionaryEntry { offending },
        }
    }

    /// Return the word absent from the dictionary.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn offending(&self) -> &str {
        let RustJvozbaError::NoDictionaryEntry { offending } = &self.value else {
            unreachable!("private class fixes the error variant")
        };
        offending
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        self.value.to_string()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "{PUBLIC_MODULE}.NoDictionaryEntry(offending={})",
            string_repr(py, self.offending())?
        ))
    }
}

/// Failure after every lujvo candidate was rejected by Rust morphology.
#[invariant(true, "private construction fixes the retained Rust error variant")]
#[pyclass(
    name = "CouldNotBuildLujvo",
    frozen,
    eq,
    module = "jbotci.jvozba",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyCouldNotBuildLujvo {
    value: RustJvozbaError,
}

#[pymethods]
impl PyCouldNotBuildLujvo {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: () = ();

    /// Construct the fieldless structured error value.
    #[requires(true)]
    #[ensures(matches!(ret.value, RustJvozbaError::CouldNotBuildLujvo))]
    #[new]
    fn new() -> Self {
        Self {
            value: RustJvozbaError::CouldNotBuildLujvo,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        self.value.to_string()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> &'static str {
        "jbotci.jvozba.CouldNotBuildLujvo()"
    }
}

/// Failure after every cmevla-like candidate was rejected by Rust morphology.
#[invariant(true, "private construction fixes the retained Rust error variant")]
#[pyclass(
    name = "CouldNotBuildCompound",
    frozen,
    eq,
    module = "jbotci.jvozba",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyCouldNotBuildCompound {
    value: RustJvozbaError,
}

#[pymethods]
impl PyCouldNotBuildCompound {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: () = ();

    /// Construct the fieldless structured error value.
    #[requires(true)]
    #[ensures(matches!(ret.value, RustJvozbaError::CouldNotBuildCompound))]
    #[new]
    fn new() -> Self {
        Self {
            value: RustJvozbaError::CouldNotBuildCompound,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        self.value.to_string()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> &'static str {
        "jbotci.jvozba.CouldNotBuildCompound()"
    }
}

#[invariant(
    !exception_name.is_empty(),
    "every projected variant names one public exception class"
)]
struct PythonErrorProjection {
    exception_name: &'static str,
    value: Py<PyAny>,
}

/// Convert every Rust error variant through one exhaustive structured match.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn jvozba_error_value_to_python(
    py: Python<'_>,
    error: RustJvozbaError,
) -> PyResult<PythonErrorProjection> {
    let (exception_name, value) = match error {
        value @ RustJvozbaError::RequiresAtLeastTwoInputs => (
            "RequiresAtLeastTwoInputsError",
            Py::new(py, PyRequiresAtLeastTwoInputs { value })?.into_any(),
        ),
        value @ RustJvozbaError::FixedRafsiEmpty => (
            "FixedRafsiEmptyError",
            Py::new(py, PyFixedRafsiEmpty { value })?.into_any(),
        ),
        value @ RustJvozbaError::NonFinalUniversalLongRafsi { .. } => (
            "NonFinalUniversalLongRafsiError",
            Py::new(py, PyNonFinalUniversalLongRafsi { value })?.into_any(),
        ),
        value @ RustJvozbaError::FinalConsonant { .. } => (
            "FinalConsonantError",
            Py::new(py, PyFinalConsonant { value })?.into_any(),
        ),
        value @ RustJvozbaError::NoRafsiAvailable { .. } => (
            "NoRafsiAvailableError",
            Py::new(py, PyNoRafsiAvailable { value })?.into_any(),
        ),
        value @ RustJvozbaError::NoDictionaryEntry { .. } => (
            "NoDictionaryEntryError",
            Py::new(py, PyNoDictionaryEntry { value })?.into_any(),
        ),
        value @ RustJvozbaError::CouldNotBuildLujvo => (
            "CouldNotBuildLujvoError",
            Py::new(py, PyCouldNotBuildLujvo { value })?.into_any(),
        ),
        value @ RustJvozbaError::CouldNotBuildCompound => (
            "CouldNotBuildCompoundError",
            Py::new(py, PyCouldNotBuildCompound { value })?.into_any(),
        ),
    };
    Ok(new!(PythonErrorProjection {
        exception_name,
        value,
    }))
}

#[requires(true)]
#[ensures(true)]
fn jvozba_error_to_python(py: Python<'_>, error: RustJvozbaError) -> PyErr {
    match jvozba_error_value_to_python(py, error) {
        Ok(projection) => {
            let projection = projection.into_data();
            public_exception_with_value(
                py,
                PUBLIC_MODULE,
                projection.exception_name,
                projection.value,
            )
        }
        Err(error) => error,
    }
}

/// Build the best jvozba using an explicit mode, dictionary, and typed inputs.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_jvozba_build_best_jvozba_detailed")]
fn build_best_jvozba_detailed(
    py: Python<'_>,
    mode: &Bound<'_, PyAny>,
    dictionary: PyRef<'_, PyDictionary>,
    raw_inputs: &Bound<'_, PyAny>,
) -> PyResult<PyJvozbaBuildResult> {
    let mode = enum_from_python(py, mode)?;
    let dictionary = dictionary.dictionary();
    let raw_inputs = extract_sequence(raw_inputs, "raw_inputs", input_from_python)?;
    py.detach(move || jbotci_jvozba::build_best_jvozba_detailed(mode, dictionary, &raw_inputs))
        .map(PyJvozbaBuildResult::from_rust)
        .map_err(|error| jvozba_error_to_python(py, error))
}

/// Return whether one word can contribute a jvozba input under an explicit dictionary.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_jvozba_word_can_enter_jvozba_pane")]
fn word_can_enter_jvozba_pane(
    py: Python<'_>,
    dictionary: PyRef<'_, PyDictionary>,
    word_text: String,
) -> PyResult<bool> {
    let dictionary = dictionary.dictionary();
    Ok(py.detach(move || jbotci_jvozba::word_can_enter_jvozba_pane(dictionary, &word_text)))
}

/// Decompose one lujvo-like word and own all dictionary provenance.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
#[pyo3(name = "_jvozba_decompose_lujvo_like")]
fn decompose_lujvo_like(
    py: Python<'_>,
    dictionary: PyRef<'_, PyDictionary>,
    raw_word: String,
) -> PyResult<Option<PyLujvoDecomposition>> {
    let dictionary = dictionary.dictionary();
    Ok(py.detach(move || {
        jbotci_jvozba::decompose_lujvo_like(dictionary, &raw_word)
            .map(OwnedLujvoDecomposition::from_rust)
            .map(PyLujvoDecomposition::from_owned)
    }))
}

macro_rules! register_function {
    ($module:expr, $name:literal, $function:ident) => {
        register_private_object($module, $name, wrap_pyfunction!($function, $module)?)?;
    };
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    register_string_enum::<JvozbaMode>(module)?;
    register_type::<PyJvozbaWord>(module, "_jvozba_Word")?;
    register_type::<PyFixedRafsi>(module, "_jvozba_FixedRafsi")?;
    register_string_enum::<JvozbaSegmentKind>(module)?;
    register_type::<PyJvozbaSegment>(module, "_jvozba_JvozbaSegment")?;
    register_type::<PyJvozbaBuildResult>(module, "_jvozba_JvozbaBuildResult")?;
    register_type::<PyLujvoSegmentInfo>(module, "_jvozba_LujvoSegmentInfo")?;
    register_type::<PyLujvoDecomposition>(module, "_jvozba_LujvoDecomposition")?;
    register_type::<PyRequiresAtLeastTwoInputs>(module, "_jvozba_RequiresAtLeastTwoInputs")?;
    register_type::<PyFixedRafsiEmpty>(module, "_jvozba_FixedRafsiEmpty")?;
    register_type::<PyNonFinalUniversalLongRafsi>(module, "_jvozba_NonFinalUniversalLongRafsi")?;
    register_type::<PyFinalConsonant>(module, "_jvozba_FinalConsonant")?;
    register_type::<PyNoRafsiAvailable>(module, "_jvozba_NoRafsiAvailable")?;
    register_type::<PyNoDictionaryEntry>(module, "_jvozba_NoDictionaryEntry")?;
    register_type::<PyCouldNotBuildLujvo>(module, "_jvozba_CouldNotBuildLujvo")?;
    register_type::<PyCouldNotBuildCompound>(module, "_jvozba_CouldNotBuildCompound")?;
    register_function!(
        module,
        "_jvozba_build_best_jvozba_detailed",
        build_best_jvozba_detailed
    );
    register_function!(
        module,
        "_jvozba_word_can_enter_jvozba_pane",
        word_can_enter_jvozba_pane
    );
    register_function!(module, "_jvozba_decompose_lujvo_like", decompose_lujvo_like);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[invariant(true)]
    struct ErrorProjectionCase {
        error: RustJvozbaError,
        exception_name: &'static str,
        value_class_name: &'static str,
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fixed_rafsi_keeps_empty_text_representable() {
        assert_eq!(PyFixedRafsi::new(String::new()).value(), "");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exhaustive_error_projection_preserves_every_rust_variant() {
        Python::initialize();
        Python::attach(|py| -> PyResult<()> {
            let cases = vec![
                ErrorProjectionCase {
                    error: RustJvozbaError::RequiresAtLeastTwoInputs,
                    exception_name: "RequiresAtLeastTwoInputsError",
                    value_class_name: "RequiresAtLeastTwoInputs",
                },
                ErrorProjectionCase {
                    error: RustJvozbaError::FixedRafsiEmpty,
                    exception_name: "FixedRafsiEmptyError",
                    value_class_name: "FixedRafsiEmpty",
                },
                ErrorProjectionCase {
                    error: RustJvozbaError::NonFinalUniversalLongRafsi {
                        offending: "klama".to_owned(),
                    },
                    exception_name: "NonFinalUniversalLongRafsiError",
                    value_class_name: "NonFinalUniversalLongRafsi",
                },
                ErrorProjectionCase {
                    error: RustJvozbaError::FinalConsonant {
                        offending: "rok".to_owned(),
                        is_fixed_rafsi: true,
                    },
                    exception_name: "FinalConsonantError",
                    value_class_name: "FinalConsonant",
                },
                ErrorProjectionCase {
                    error: RustJvozbaError::NoRafsiAvailable {
                        offending: "mi".to_owned(),
                    },
                    exception_name: "NoRafsiAvailableError",
                    value_class_name: "NoRafsiAvailable",
                },
                ErrorProjectionCase {
                    error: RustJvozbaError::NoDictionaryEntry {
                        offending: "notlojban".to_owned(),
                    },
                    exception_name: "NoDictionaryEntryError",
                    value_class_name: "NoDictionaryEntry",
                },
                ErrorProjectionCase {
                    error: RustJvozbaError::CouldNotBuildLujvo,
                    exception_name: "CouldNotBuildLujvoError",
                    value_class_name: "CouldNotBuildLujvo",
                },
                ErrorProjectionCase {
                    error: RustJvozbaError::CouldNotBuildCompound,
                    exception_name: "CouldNotBuildCompoundError",
                    value_class_name: "CouldNotBuildCompound",
                },
            ];

            for case in cases {
                let projection = jvozba_error_value_to_python(py, case.error)?;
                assert_eq!(projection.exception_name, case.exception_name);
                assert_eq!(
                    projection.value.bind(py).get_type().name()?,
                    case.value_class_name
                );
            }
            Ok(())
        })
        .unwrap();
    }
}
