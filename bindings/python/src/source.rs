//! Python projection of source identifiers, locations, and text-offset helpers.

use std::sync::Arc;

use bityzba::{ensures, invariant, new, requires};
use jbotci_diagnostics::{
    DiagnosticSpanError, byte_offset_for_char_offset as rust_byte_offset_for_char_offset,
    char_offset_for_byte_offset as rust_char_offset_for_byte_offset,
    line_column_for_byte_offset as rust_line_column_for_byte_offset,
    source_span_from_byte_offsets as rust_source_span_from_byte_offsets,
    source_span_from_char_offsets as rust_source_span_from_char_offsets,
    source_text_for_span as rust_source_text_for_span,
};
use jbotci_source::{LineColumn, SourceId, SourceLocationError, SourceSpan};
use pyo3::prelude::*;

use crate::support::{
    public_exception_with_value, register_private_object, register_type, string_repr,
};

pub(crate) const NATIVE_EXPORTS: &[&str] = &[
    "_source_SourceId",
    "_source_LineColumn",
    "_source_SourceSpan",
    "_source_ZeroLine",
    "_source_ZeroColumn",
    "_source_ByteRangeInverted",
    "_source_CharRangeInverted",
    "_source_CharOffsetOutOfBounds",
    "_source_ByteOffsetOutOfBounds",
    "_source_ByteOffsetNotCharBoundary",
    "_source_SourceLocation",
    "_source_source_span_from_char_offsets",
    "_source_source_span_from_byte_offsets",
    "_source_byte_offset_for_char_offset",
    "_source_char_offset_for_byte_offset",
    "_source_line_column_for_byte_offset",
    "_source_source_text_for_span",
];

/// Source-location failure indicating a one-based line number of zero.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "ZeroLine",
    frozen,
    eq,
    module = "jbotci.source",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyZeroLine {
    value: SourceLocationError,
}

#[pymethods]
impl PyZeroLine {
    #[allow(non_upper_case_globals)]
    #[requires(true)]
    #[ensures(ret.bind(py).is_empty())]
    #[classattr]
    fn __match_args__(py: Python<'_>) -> Py<pyo3::types::PyTuple> {
        pyo3::types::PyTuple::empty(py).unbind()
    }

    /// Construct the zero-line error variant.
    #[requires(true)]
    #[ensures(ret.value == SourceLocationError::ZeroLine)]
    #[new]
    fn new() -> Self {
        PyZeroLine {
            value: SourceLocationError::ZeroLine,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        self.value.to_string()
    }

    #[requires(true)]
    #[ensures(ret == "jbotci.source.ZeroLine()")]
    fn __repr__(&self) -> &'static str {
        "jbotci.source.ZeroLine()"
    }
}

/// Source-location failure indicating a one-based column number of zero.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "ZeroColumn",
    frozen,
    eq,
    module = "jbotci.source",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyZeroColumn {
    value: SourceLocationError,
}

#[pymethods]
impl PyZeroColumn {
    #[allow(non_upper_case_globals)]
    #[requires(true)]
    #[ensures(ret.bind(py).is_empty())]
    #[classattr]
    fn __match_args__(py: Python<'_>) -> Py<pyo3::types::PyTuple> {
        pyo3::types::PyTuple::empty(py).unbind()
    }

    /// Construct the zero-column error variant.
    #[requires(true)]
    #[ensures(ret.value == SourceLocationError::ZeroColumn)]
    #[new]
    fn new() -> Self {
        PyZeroColumn {
            value: SourceLocationError::ZeroColumn,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        self.value.to_string()
    }

    #[requires(true)]
    #[ensures(ret == "jbotci.source.ZeroColumn()")]
    fn __repr__(&self) -> &'static str {
        "jbotci.source.ZeroColumn()"
    }
}

/// Source-location failure carrying an inverted byte range.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "ByteRangeInverted",
    frozen,
    eq,
    module = "jbotci.source",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyByteRangeInverted {
    value: SourceLocationError,
}

#[pymethods]
impl PyByteRangeInverted {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("start", "end");

    /// Preserve a byte-range error payload exactly, without revalidating endpoint order.
    #[requires(true)]
    #[ensures(matches!(ret.value, SourceLocationError::ByteRangeInverted { start: value_start, end: value_end } if value_start == start && value_end == end))]
    #[new]
    fn new(start: usize, end: usize) -> Self {
        PyByteRangeInverted {
            value: SourceLocationError::ByteRangeInverted { start, end },
        }
    }

    /// Return the supplied start byte offset.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn start(&self) -> usize {
        let SourceLocationError::ByteRangeInverted { start, .. } = &self.value else {
            unreachable!("private construction fixes the source error variant")
        };
        *start
    }

    /// Return the supplied end byte offset.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn end(&self) -> usize {
        let SourceLocationError::ByteRangeInverted { end, .. } = &self.value else {
            unreachable!("private construction fixes the source error variant")
        };
        *end
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        self.value.to_string()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!(
            "jbotci.source.ByteRangeInverted(start={}, end={})",
            self.start(),
            self.end()
        )
    }
}

/// Source-location failure carrying an inverted character range.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "CharRangeInverted",
    frozen,
    eq,
    module = "jbotci.source",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyCharRangeInverted {
    value: SourceLocationError,
}

#[pymethods]
impl PyCharRangeInverted {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("start", "end");

    /// Preserve a character-range error payload exactly, without revalidating endpoint order.
    #[requires(true)]
    #[ensures(matches!(ret.value, SourceLocationError::CharRangeInverted { start: value_start, end: value_end } if value_start == start && value_end == end))]
    #[new]
    fn new(start: usize, end: usize) -> Self {
        PyCharRangeInverted {
            value: SourceLocationError::CharRangeInverted { start, end },
        }
    }

    /// Return the supplied start character offset.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn start(&self) -> usize {
        let SourceLocationError::CharRangeInverted { start, .. } = &self.value else {
            unreachable!("private construction fixes the source error variant")
        };
        *start
    }

    /// Return the supplied end character offset.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn end(&self) -> usize {
        let SourceLocationError::CharRangeInverted { end, .. } = &self.value else {
            unreachable!("private construction fixes the source error variant")
        };
        *end
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        self.value.to_string()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!(
            "jbotci.source.CharRangeInverted(start={}, end={})",
            self.start(),
            self.end()
        )
    }
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn source_location_error_to_python(
    py: Python<'_>,
    error: SourceLocationError,
) -> PyResult<Py<PyAny>> {
    Ok(match error {
        SourceLocationError::ZeroLine => Py::new(
            py,
            PyZeroLine {
                value: SourceLocationError::ZeroLine,
            },
        )?
        .into_any(),
        SourceLocationError::ZeroColumn => Py::new(
            py,
            PyZeroColumn {
                value: SourceLocationError::ZeroColumn,
            },
        )?
        .into_any(),
        error @ SourceLocationError::ByteRangeInverted { .. } => {
            Py::new(py, PyByteRangeInverted { value: error })?.into_any()
        }
        error @ SourceLocationError::CharRangeInverted { .. } => {
            Py::new(py, PyCharRangeInverted { value: error })?.into_any()
        }
    })
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn source_location_error_from_python(
    value: &Bound<'_, PyAny>,
) -> PyResult<SourceLocationError> {
    if let Ok(value) = value.extract::<PyRef<'_, PyZeroLine>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyZeroColumn>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyByteRangeInverted>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyCharRangeInverted>>() {
        return Ok(value.value.clone());
    }
    Err(pyo3::exceptions::PyTypeError::new_err(
        "expected a jbotci.source SourceLocationError variant",
    ))
}

#[requires(true)]
#[ensures(true)]
fn source_location_error_to_exception(py: Python<'_>, error: SourceLocationError) -> PyErr {
    match source_location_error_to_python(py, error) {
        Ok(value) => {
            public_exception_with_value(py, "jbotci.source", "SourceLocationException", value)
        }
        Err(error) => error,
    }
}

/// Diagnostic-span failure carrying an out-of-bounds character offset.
#[invariant(true, "the Rust error preserves every supplied numeric payload")]
#[pyclass(
    name = "CharOffsetOutOfBounds",
    frozen,
    eq,
    module = "jbotci.source",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyCharOffsetOutOfBounds {
    offset: usize,
    source_len: usize,
}

#[pymethods]
impl PyCharOffsetOutOfBounds {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("offset", "source_len");

    /// Preserve an exact character-offset diagnostic payload.
    #[requires(true)]
    #[ensures(ret.offset == offset && ret.source_len == source_len)]
    #[new]
    fn new(offset: usize, source_len: usize) -> Self {
        Self { offset, source_len }
    }

    /// Return the supplied character offset.
    #[requires(true)]
    #[ensures(ret == self.offset)]
    #[getter]
    fn offset(&self) -> usize {
        self.offset
    }

    /// Return the source length in Unicode scalar values.
    #[requires(true)]
    #[ensures(ret == self.source_len)]
    #[getter]
    fn source_len(&self) -> usize {
        self.source_len
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        format!(
            "character offset {} exceeds source character length {}",
            self.offset, self.source_len
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!(
            "jbotci.source.CharOffsetOutOfBounds(offset={}, source_len={})",
            self.offset, self.source_len
        )
    }
}

/// Diagnostic-span failure carrying an out-of-bounds byte offset.
#[invariant(true, "the Rust error preserves every supplied numeric payload")]
#[pyclass(
    name = "ByteOffsetOutOfBounds",
    frozen,
    eq,
    module = "jbotci.source",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyByteOffsetOutOfBounds {
    offset: usize,
    source_len: usize,
}

#[pymethods]
impl PyByteOffsetOutOfBounds {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("offset", "source_len");

    /// Preserve an exact byte-offset diagnostic payload.
    #[requires(true)]
    #[ensures(ret.offset == offset && ret.source_len == source_len)]
    #[new]
    fn new(offset: usize, source_len: usize) -> Self {
        Self { offset, source_len }
    }

    /// Return the supplied byte offset.
    #[requires(true)]
    #[ensures(ret == self.offset)]
    #[getter]
    fn offset(&self) -> usize {
        self.offset
    }

    /// Return the source length in UTF-8 bytes.
    #[requires(true)]
    #[ensures(ret == self.source_len)]
    #[getter]
    fn source_len(&self) -> usize {
        self.source_len
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        format!(
            "byte offset {} exceeds source byte length {}",
            self.offset, self.source_len
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!(
            "jbotci.source.ByteOffsetOutOfBounds(offset={}, source_len={})",
            self.offset, self.source_len
        )
    }
}

/// Diagnostic-span failure carrying a byte offset inside a UTF-8 scalar.
#[invariant(true, "the Rust error preserves every supplied numeric payload")]
#[pyclass(
    name = "ByteOffsetNotCharBoundary",
    frozen,
    eq,
    module = "jbotci.source",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyByteOffsetNotCharBoundary {
    offset: usize,
}

#[pymethods]
impl PyByteOffsetNotCharBoundary {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("offset",);

    /// Preserve an exact non-boundary byte offset.
    #[requires(true)]
    #[ensures(ret.offset == offset)]
    #[new]
    fn new(offset: usize) -> Self {
        Self { offset }
    }

    /// Return the supplied byte offset.
    #[requires(true)]
    #[ensures(ret == self.offset)]
    #[getter]
    fn offset(&self) -> usize {
        self.offset
    }

    #[requires(true)]
    #[ensures(true)]
    fn __str__(&self) -> String {
        format!(
            "byte offset {} is not a UTF-8 character boundary",
            self.offset
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!(
            "jbotci.source.ByteOffsetNotCharBoundary(offset={})",
            self.offset
        )
    }
}

/// Diagnostic-span failure wrapping a structured source-location error.
#[invariant(
    true,
    "PyO3 requires the declared class shape; SourceLocationError represents every valid shell state"
)]
#[pyclass(
    name = "SourceLocation",
    frozen,
    eq,
    module = "jbotci.source",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyDiagnosticSourceLocation {
    value: SourceLocationError,
}

impl PyDiagnosticSourceLocation {
    #[requires(true)]
    #[ensures(true)]
    fn source_error(&self) -> &SourceLocationError {
        &self.value
    }
}

#[pymethods]
impl PyDiagnosticSourceLocation {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("error",);

    /// Wrap one exact source-location error as a diagnostic-span detail.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(error: &Bound<'_, PyAny>) -> PyResult<Self> {
        let value = source_location_error_from_python(error)?;
        Ok(PyDiagnosticSourceLocation { value })
    }

    /// Return the nested source-location error.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn error(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        source_location_error_to_python(py, self.source_error().clone())
    }

    #[requires(true)]
    #[ensures(ret.starts_with("invalid source span: "))]
    fn __str__(&self) -> String {
        DiagnosticSpanError::SourceLocation(self.value.clone()).to_string()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let error = source_location_error_to_python(py, self.source_error().clone())?;
        Ok(format!(
            "jbotci.source.SourceLocation(error={})",
            error.bind(py).repr()?.to_str()?
        ))
    }
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_span_error_to_exception(py: Python<'_>, error: DiagnosticSpanError) -> PyErr {
    let value = match error {
        DiagnosticSpanError::CharOffsetOutOfBounds { offset, source_len } => {
            Py::new(py, PyCharOffsetOutOfBounds { offset, source_len }).map(Py::into_any)
        }
        DiagnosticSpanError::ByteOffsetOutOfBounds { offset, source_len } => {
            Py::new(py, PyByteOffsetOutOfBounds { offset, source_len }).map(Py::into_any)
        }
        DiagnosticSpanError::ByteOffsetNotCharBoundary { offset } => {
            Py::new(py, PyByteOffsetNotCharBoundary { offset }).map(Py::into_any)
        }
        DiagnosticSpanError::SourceLocation(error) => {
            Py::new(py, PyDiagnosticSourceLocation { value: error }).map(Py::into_any)
        }
    };
    match value {
        Ok(value) => {
            public_exception_with_value(py, "jbotci.source", "DiagnosticSpanException", value)
        }
        Err(error) => error,
    }
}

/// Stable identifier for the source text associated with a span.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "SourceId",
    frozen,
    eq,
    hash,
    module = "jbotci.source",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PySourceId {
    value: SourceId,
}

impl PySourceId {
    #[requires(true)]
    #[ensures(ret.0 == self.value.0)]
    pub(crate) fn clone_rust(&self) -> SourceId {
        self.value.clone()
    }

    #[requires(true)]
    #[ensures(ret.value.0 == old(value.0.clone()))]
    pub(crate) fn from_rust(value: SourceId) -> Self {
        Self { value }
    }
}

#[pymethods]
impl PySourceId {
    /// Construct a source identifier from arbitrary text.
    #[requires(true)]
    #[ensures(ret.value.0 == value)]
    #[new]
    fn new(value: &str) -> Self {
        Self::from_rust(SourceId(value.to_owned()))
    }

    /// Return the identifier text.
    #[requires(true)]
    #[ensures(ret == self.value.0.as_str())]
    #[getter]
    fn value(&self) -> &str {
        &self.value.0
    }

    #[requires(true)]
    #[ensures(ret == self.value.0.as_str())]
    fn __str__(&self) -> &str {
        &self.value.0
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "jbotci.source.SourceId(value={})",
            string_repr(py, &self.value.0)?
        ))
    }
}

/// One-based line and column coordinates in source text.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "LineColumn",
    frozen,
    eq,
    hash,
    module = "jbotci.source",
    skip_from_py_object
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PyLineColumn {
    value: LineColumn,
}

impl PyLineColumn {
    #[requires(true)]
    #[ensures(ret == self.value)]
    pub(crate) fn rust(&self) -> LineColumn {
        self.value
    }

    #[requires(value.line > 0 && value.column > 0)]
    #[ensures(ret.value == value)]
    pub(crate) fn from_rust(value: LineColumn) -> Self {
        PyLineColumn { value }
    }
}

#[pymethods]
impl PyLineColumn {
    /// Construct validated one-based line and column coordinates.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|value| value.value.line == line && value.value.column == column) || ret.is_err())]
    #[new]
    fn new(py: Python<'_>, line: usize, column: usize) -> PyResult<Self> {
        LineColumn::new(line, column)
            .map(Self::from_rust)
            .map_err(|error| source_location_error_to_exception(py, error))
    }

    /// Return the one-based line number.
    #[requires(true)]
    #[ensures(ret == self.value.line)]
    #[getter]
    fn line(&self) -> usize {
        self.value.line
    }

    /// Return the one-based column number.
    #[requires(true)]
    #[ensures(ret == self.value.column)]
    #[getter]
    fn column(&self) -> usize {
        self.value.column
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!(
            "jbotci.source.LineColumn(line={}, column={})",
            self.value.line, self.value.column
        )
    }
}

/// Immutable half-open byte and character ranges for a source region.
#[invariant(
    true,
    "PyO3 requires the declared class shape; checked constructors and validated Rust storage enforce projection constraints"
)]
#[pyclass(
    name = "SourceSpan",
    frozen,
    eq,
    hash,
    module = "jbotci.source",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PySourceSpan {
    value: Arc<SourceSpan>,
}

impl PySourceSpan {
    #[requires(true)]
    #[ensures(ret.byte_start == self.value.byte_start)]
    #[ensures(ret.byte_end == self.value.byte_end)]
    #[ensures(ret.char_start == self.value.char_start)]
    #[ensures(ret.char_end == self.value.char_end)]
    pub(crate) fn clone_rust(&self) -> SourceSpan {
        self.value.as_ref().clone()
    }

    #[requires(true)]
    #[ensures(ret.byte_start == self.value.byte_start)]
    #[ensures(ret.byte_end == self.value.byte_end)]
    #[ensures(ret.char_start == self.value.char_start)]
    #[ensures(ret.char_end == self.value.char_end)]
    pub(crate) fn rust(&self) -> &SourceSpan {
        self.value.as_ref()
    }

    #[requires(value.byte_start <= value.byte_end)]
    #[requires(value.char_start <= value.char_end)]
    #[ensures(ret.value.byte_start == old(value.byte_start))]
    #[ensures(ret.value.byte_end == old(value.byte_end))]
    #[ensures(ret.value.char_start == old(value.char_start))]
    #[ensures(ret.value.char_end == old(value.char_end))]
    pub(crate) fn from_rust(value: SourceSpan) -> Self {
        PySourceSpan {
            value: Arc::new(value),
        }
    }
}

#[pymethods]
impl PySourceSpan {
    /// Construct a span after validating each half-open range independently.
    #[allow(clippy::too_many_arguments)]
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|span| span.value.byte_start == byte_start) || ret.is_err())]
    #[ensures(ret.as_ref().is_ok_and(|span| span.value.byte_end == byte_end) || ret.is_err())]
    #[ensures(ret.as_ref().is_ok_and(|span| span.value.char_start == char_start) || ret.is_err())]
    #[ensures(ret.as_ref().is_ok_and(|span| span.value.char_end == char_end) || ret.is_err())]
    #[new]
    #[pyo3(signature = (byte_start, byte_end, char_start, char_end, *, source_id=None, start=None, end=None))]
    fn new(
        py: Python<'_>,
        byte_start: usize,
        byte_end: usize,
        char_start: usize,
        char_end: usize,
        source_id: Option<PyRef<'_, PySourceId>>,
        start: Option<PyRef<'_, PyLineColumn>>,
        end: Option<PyRef<'_, PyLineColumn>>,
    ) -> PyResult<Self> {
        let source_id = source_id.as_ref().map(|source_id| source_id.clone_rust());
        let start = start.as_ref().map(|start| start.rust());
        let end = end.as_ref().map(|end| end.rust());
        SourceSpan::new(source_id, byte_start, byte_end, char_start, char_end)
            .map(|span| span.with_line_columns(start, end))
            .map(Self::from_rust)
            .map_err(|error| source_location_error_to_exception(py, error))
    }

    /// Return the optional source identifier.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn source_id(&self) -> Option<PySourceId> {
        self.value
            .source_id
            .as_ref()
            .cloned()
            .map(PySourceId::from_rust)
    }

    /// Return the inclusive byte start offset.
    #[requires(true)]
    #[ensures(ret == self.value.byte_start)]
    #[getter]
    fn byte_start(&self) -> usize {
        self.value.byte_start
    }

    /// Return the exclusive byte end offset.
    #[requires(true)]
    #[ensures(ret == self.value.byte_end)]
    #[getter]
    fn byte_end(&self) -> usize {
        self.value.byte_end
    }

    /// Return the half-open byte range as `(start, end)`.
    #[requires(true)]
    #[ensures(ret == (self.value.byte_start, self.value.byte_end))]
    #[getter]
    fn byte_range(&self) -> (usize, usize) {
        (self.value.byte_start, self.value.byte_end)
    }

    /// Return the inclusive Unicode-scalar start offset.
    #[requires(true)]
    #[ensures(ret == self.value.char_start)]
    #[getter]
    fn char_start(&self) -> usize {
        self.value.char_start
    }

    /// Return the exclusive Unicode-scalar end offset.
    #[requires(true)]
    #[ensures(ret == self.value.char_end)]
    #[getter]
    fn char_end(&self) -> usize {
        self.value.char_end
    }

    /// Return the half-open character range as `(start, end)`.
    #[requires(true)]
    #[ensures(ret == (self.value.char_start, self.value.char_end))]
    #[getter]
    fn char_range(&self) -> (usize, usize) {
        (self.value.char_start, self.value.char_end)
    }

    /// Return the optional one-based start line and column.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn start(&self) -> Option<PyLineColumn> {
        self.value.start.map(PyLineColumn::from_rust)
    }

    /// Return the optional one-based end line and column.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn end(&self) -> Option<PyLineColumn> {
        self.value.end.map(PyLineColumn::from_rust)
    }

    /// Return the span length in UTF-8 bytes.
    #[requires(true)]
    #[ensures(ret == self.value.byte_len())]
    #[getter]
    fn byte_len(&self) -> usize {
        self.value.byte_len()
    }

    /// Return the span length in Unicode scalar values.
    #[requires(true)]
    #[ensures(ret == self.value.char_len())]
    #[getter]
    fn char_len(&self) -> usize {
        self.value.char_len()
    }

    /// Report whether both source ranges are empty.
    #[requires(true)]
    #[ensures(ret == self.value.is_empty())]
    fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let source_id = match self.value.source_id.as_ref() {
            None => "None".to_owned(),
            Some(source_id) => {
                format!(
                    "jbotci.source.SourceId(value={})",
                    string_repr(py, &source_id.0)?
                )
            }
        };
        let start = line_column_repr(self.value.start);
        let end = line_column_repr(self.value.end);
        Ok(format!(
            "jbotci.source.SourceSpan({}, {}, {}, {}, source_id={}, start={}, end={})",
            self.value.byte_start,
            self.value.byte_end,
            self.value.char_start,
            self.value.char_end,
            source_id,
            start,
            end
        ))
    }
}

#[requires(true)]
#[ensures(value.is_none() == (ret == "None"))]
fn line_column_repr(value: Option<LineColumn>) -> String {
    value.map_or_else(
        || "None".to_owned(),
        |value| {
            format!(
                "jbotci.source.LineColumn(line={}, column={})",
                value.line, value.column
            )
        },
    )
}

#[requires(true)]
#[ensures(true)]
fn optional_source_id(value: Option<PyRef<'_, PySourceId>>) -> Option<SourceId> {
    value.as_ref().map(|source_id| source_id.clone_rust())
}

#[requires(true)]
#[ensures(ret.is_ok() -> byte_offset <= source.len() && source.is_char_boundary(byte_offset))]
fn validate_python_byte_offset(
    source: &str,
    byte_offset: usize,
) -> Result<(), DiagnosticSpanError> {
    if byte_offset > source.len() {
        return Err(DiagnosticSpanError::ByteOffsetOutOfBounds {
            offset: byte_offset,
            source_len: source.len(),
        });
    }
    if !source.is_char_boundary(byte_offset) {
        return Err(DiagnosticSpanError::ByteOffsetNotCharBoundary {
            offset: byte_offset,
        });
    }
    Ok(())
}

/// Build a source span from Unicode-scalar offsets into `source`.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(signature = (source, char_start, char_end, *, source_id=None))]
fn source_span_from_char_offsets(
    py: Python<'_>,
    source: &str,
    char_start: usize,
    char_end: usize,
    source_id: Option<PyRef<'_, PySourceId>>,
) -> PyResult<PySourceSpan> {
    rust_source_span_from_char_offsets(optional_source_id(source_id), source, char_start, char_end)
        .map(PySourceSpan::from_rust)
        .map_err(|error| diagnostic_span_error_to_exception(py, error))
}

/// Build a source span from UTF-8 byte offsets into `source`.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(signature = (source, byte_start, byte_end, *, source_id=None))]
fn source_span_from_byte_offsets(
    py: Python<'_>,
    source: &str,
    byte_start: usize,
    byte_end: usize,
    source_id: Option<PyRef<'_, PySourceId>>,
) -> PyResult<PySourceSpan> {
    rust_source_span_from_byte_offsets(optional_source_id(source_id), source, byte_start, byte_end)
        .map(PySourceSpan::from_rust)
        .map_err(|error| diagnostic_span_error_to_exception(py, error))
}

/// Convert a Unicode-scalar offset into a UTF-8 byte offset.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|offset| *offset <= source.len()) || ret.is_err())]
#[pyfunction]
fn byte_offset_for_char_offset(
    py: Python<'_>,
    source: &str,
    char_offset: usize,
) -> PyResult<usize> {
    rust_byte_offset_for_char_offset(source, char_offset)
        .map_err(|error| diagnostic_span_error_to_exception(py, error))
}

/// Convert a UTF-8 byte boundary into a Unicode-scalar offset.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
fn char_offset_for_byte_offset(
    py: Python<'_>,
    source: &str,
    byte_offset: usize,
) -> PyResult<usize> {
    validate_python_byte_offset(source, byte_offset)
        .map_err(|error| diagnostic_span_error_to_exception(py, error))?;
    rust_char_offset_for_byte_offset(source, byte_offset)
        .map_err(|error| diagnostic_span_error_to_exception(py, error))
}

/// Compute the one-based line and column at a UTF-8 byte boundary.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|value| value.value.line > 0 && value.value.column > 0) || ret.is_err())]
#[pyfunction]
fn line_column_for_byte_offset(
    py: Python<'_>,
    source: &str,
    byte_offset: usize,
) -> PyResult<PyLineColumn> {
    rust_line_column_for_byte_offset(source, byte_offset)
        .map(PyLineColumn::from_rust)
        .map_err(|error| diagnostic_span_error_to_exception(py, error))
}

/// Extract the exact source substring covered by a validated span.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
fn source_text_for_span(
    py: Python<'_>,
    source: &str,
    span: PyRef<'_, PySourceSpan>,
) -> PyResult<String> {
    rust_source_text_for_span(source, span.rust())
        .map_err(|error| diagnostic_span_error_to_exception(py, error))
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    register_type::<PySourceId>(module, "_source_SourceId")?;
    register_type::<PyLineColumn>(module, "_source_LineColumn")?;
    register_type::<PySourceSpan>(module, "_source_SourceSpan")?;
    register_type::<PyZeroLine>(module, "_source_ZeroLine")?;
    register_type::<PyZeroColumn>(module, "_source_ZeroColumn")?;
    register_type::<PyByteRangeInverted>(module, "_source_ByteRangeInverted")?;
    register_type::<PyCharRangeInverted>(module, "_source_CharRangeInverted")?;
    register_type::<PyCharOffsetOutOfBounds>(module, "_source_CharOffsetOutOfBounds")?;
    register_type::<PyByteOffsetOutOfBounds>(module, "_source_ByteOffsetOutOfBounds")?;
    register_type::<PyByteOffsetNotCharBoundary>(module, "_source_ByteOffsetNotCharBoundary")?;
    register_type::<PyDiagnosticSourceLocation>(module, "_source_SourceLocation")?;
    register_private_object(
        module,
        "_source_source_span_from_char_offsets",
        wrap_pyfunction!(source_span_from_char_offsets, module)?,
    )?;
    register_private_object(
        module,
        "_source_source_span_from_byte_offsets",
        wrap_pyfunction!(source_span_from_byte_offsets, module)?,
    )?;
    register_private_object(
        module,
        "_source_byte_offset_for_char_offset",
        wrap_pyfunction!(byte_offset_for_char_offset, module)?,
    )?;
    register_private_object(
        module,
        "_source_char_offset_for_byte_offset",
        wrap_pyfunction!(char_offset_for_byte_offset, module)?,
    )?;
    register_private_object(
        module,
        "_source_line_column_for_byte_offset",
        wrap_pyfunction!(line_column_for_byte_offset, module)?,
    )?;
    register_private_object(
        module,
        "_source_source_text_for_span",
        wrap_pyfunction!(source_text_for_span, module)?,
    )?;
    Ok(())
}
