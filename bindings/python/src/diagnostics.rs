//! Immutable Python projection of diagnostics and parser trace products.

use std::borrow::Cow;

use bityzba::{contract_trait, ensures, expensive_ensures, invariant, new, requires};
use jbotci_diagnostics::{
    DEFAULT_TRACE_LIMIT, Diagnostic, DiagnosticDetailMode, DiagnosticLabel, DiagnosticNoteMode,
    DiagnosticPhase, DiagnosticSeverity, DiagnosticStyledNote, DiagnosticTextLink,
    DiagnosticTextRole, DiagnosticTextSegment, TraceContext, TraceEvent, TraceEventKind,
    TraceFailureBranch, TraceFailureSummary, TraceFilter, TraceLevel, TraceOptions, TracePhase,
    TraceReport, diagnostic_text_segments as rust_diagnostic_text_segments,
    diagnostic_text_segments_text as rust_diagnostic_text_segments_text,
};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyTuple};

use crate::InvalidInputError;
use crate::source::PySourceSpan;
use crate::support::{
    PythonStringEnum, extract_string_enum, register_private_object, register_string_enum,
    register_type, sequence_to_tuple, string_enum_member, string_repr,
};

const PUBLIC_MODULE: &str = "jbotci.diagnostics";

pub(crate) const NATIVE_EXPORTS: &[&str] = &[
    "_diagnostics_DiagnosticSeverity",
    "_diagnostics_DiagnosticPhase",
    "_diagnostics_TracePhase",
    "_diagnostics_TraceLevel",
    "_diagnostics_TraceEventKind",
    "_diagnostics_DiagnosticDetailMode",
    "_diagnostics_DiagnosticNoteMode",
    "_diagnostics_DiagnosticTextRole",
    "_diagnostics_TraceFilter",
    "_diagnostics_TraceOptions",
    "_diagnostics_TraceEvent",
    "_diagnostics_TraceContext",
    "_diagnostics_TraceFailureBranch",
    "_diagnostics_TraceFailureSummary",
    "_diagnostics_TraceReport",
    "_diagnostics_VlackuWordLink",
    "_diagnostics_CllSectionLink",
    "_diagnostics_EbnfRuleLink",
    "_diagnostics_DiagnosticTextSegment",
    "_diagnostics_DiagnosticStyledNote",
    "_diagnostics_DiagnosticLabel",
    "_diagnostics_Diagnostic",
    "_diagnostics_diagnostic_text_segments",
    "_diagnostics_diagnostic_text_segments_text",
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
                concat!("jbotci diagnostic enum ", $python_name, ".")
            }

            fn variants() -> &'static [Self] {
                Self::ALL
            }

            fn python_member_name(self) -> Cow<'static, str> {
                Cow::Owned(self.name().replace('-', "_").to_ascii_uppercase())
            }

            fn python_value(self) -> &'static str {
                self.name()
            }
        }
    };
}

impl_python_string_enum!(
    DiagnosticSeverity,
    "_diagnostics_DiagnosticSeverity",
    "DiagnosticSeverity"
);
impl_python_string_enum!(
    DiagnosticPhase,
    "_diagnostics_DiagnosticPhase",
    "DiagnosticPhase"
);
impl_python_string_enum!(TracePhase, "_diagnostics_TracePhase", "TracePhase");
impl_python_string_enum!(TraceLevel, "_diagnostics_TraceLevel", "TraceLevel");
impl_python_string_enum!(
    TraceEventKind,
    "_diagnostics_TraceEventKind",
    "TraceEventKind"
);
impl_python_string_enum!(
    DiagnosticDetailMode,
    "_diagnostics_DiagnosticDetailMode",
    "DiagnosticDetailMode"
);
impl_python_string_enum!(
    DiagnosticNoteMode,
    "_diagnostics_DiagnosticNoteMode",
    "DiagnosticNoteMode"
);
impl_python_string_enum!(
    DiagnosticTextRole,
    "_diagnostics_DiagnosticTextRole",
    "DiagnosticTextRole"
);

#[requires(true)]
#[ensures(true)]
fn native_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    py.import("jbotci._native")
}

#[requires(true)]
#[ensures(true)]
fn enum_from_python<E: PythonStringEnum>(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<E> {
    extract_string_enum(&native_module(py)?, value)
}

#[requires(true)]
#[ensures(true)]
fn enum_to_python<E: PythonStringEnum>(py: Python<'_>, value: E) -> PyResult<Py<PyAny>> {
    string_enum_member(&native_module(py)?, value).map(Bound::unbind)
}

/// Optional trace-event label filter.
#[invariant(!value.name.is_empty())]
#[pyclass(
    name = "TraceFilter",
    frozen,
    eq,
    hash,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PyTraceFilter {
    value: TraceFilter,
}

impl PyTraceFilter {
    #[requires(!value.name.is_empty())]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from_rust(value: TraceFilter) -> Self {
        new!(PyTraceFilter { value })
    }
}

#[pymethods]
impl PyTraceFilter {
    /// Construct a non-empty trace label filter.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|filter| filter.value.name == name) || ret.is_err())]
    #[new]
    fn new(name: String) -> PyResult<Self> {
        if name.is_empty() {
            return Err(InvalidInputError::new_err(
                "trace filter name must not be empty",
            ));
        }
        Ok(Self::from_rust(TraceFilter::new(name)))
    }

    /// Return the required event-label text.
    #[requires(true)]
    #[ensures(ret == self.value.name.as_str())]
    #[getter]
    fn name(&self) -> &str {
        &self.value.name
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        Ok(format!(
            "jbotci.diagnostics.TraceFilter(name={})",
            string_repr(py, &self.value.name)?
        ))
    }
}

/// Immutable parser-trace configuration.
#[invariant(value.limit > 0)]
#[pyclass(
    name = "TraceOptions",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyTraceOptions {
    value: TraceOptions,
}

impl PyTraceOptions {
    #[requires(value.limit > 0)]
    #[expensive_ensures(ret.value == old(value.clone()))]
    pub(crate) fn from_rust(value: TraceOptions) -> Self {
        new!(PyTraceOptions { value })
    }

    #[requires(true)]
    #[ensures(ret == &self.value)]
    pub(crate) fn rust(&self) -> &TraceOptions {
        &self.value
    }
}

#[pymethods]
impl PyTraceOptions {
    /// Construct trace options, validating the non-zero event limit.
    #[allow(clippy::too_many_arguments)]
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|options| options.value.limit == limit) || ret.is_err())]
    #[new]
    #[pyo3(signature = (*, enabled=false, level=None, filter=None, phase=None, limit=DEFAULT_TRACE_LIMIT))]
    fn new(
        py: Python<'_>,
        enabled: bool,
        level: Option<&Bound<'_, PyAny>>,
        filter: Option<PyRef<'_, PyTraceFilter>>,
        phase: Option<&Bound<'_, PyAny>>,
        limit: usize,
    ) -> PyResult<Self> {
        if limit == 0 {
            return Err(InvalidInputError::new_err(
                "trace event limit must be greater than zero",
            ));
        }
        let level = level.map_or(Ok(TraceLevel::Top), |value| enum_from_python(py, value))?;
        let phase = phase.map_or(Ok(TracePhase::All), |value| enum_from_python(py, value))?;
        let value = new!(TraceOptions {
            enabled,
            level,
            filter: filter.as_ref().map(|filter| filter.value.clone()),
            phase,
            limit,
        });
        Ok(Self::from_rust(value))
    }

    /// Report whether tracing is enabled.
    #[requires(true)]
    #[ensures(ret == self.value.enabled)]
    #[getter]
    fn enabled(&self) -> bool {
        self.value.enabled
    }

    /// Return the configured trace detail level.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn level(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.level)
    }

    /// Return the optional event-label filter.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn filter(&self) -> Option<PyTraceFilter> {
        self.value.filter.clone().map(PyTraceFilter::from_rust)
    }

    /// Return the phase selected for tracing.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn phase(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.phase)
    }

    /// Return the non-zero trace event limit.
    #[requires(true)]
    #[ensures(ret == self.value.limit)]
    #[getter]
    fn limit(&self) -> usize {
        self.value.limit
    }

    /// Return a copy configured for a different trace phase.
    #[requires(true)]
    #[ensures(ret.value.phase == phase)]
    fn with_phase(&self, py: Python<'_>, phase: &Bound<'_, PyAny>) -> PyResult<Self> {
        let phase = enum_from_python(py, phase)?;
        Ok(Self::from_rust(self.value.clone().with_phase(phase)))
    }

    /// Return a copy with a validated non-zero event limit.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|options| options.value.limit == limit) || ret.is_err())]
    fn with_limit(&self, limit: usize) -> PyResult<Self> {
        if limit == 0 {
            return Err(InvalidInputError::new_err(
                "trace event limit must be greater than zero",
            ));
        }
        Ok(Self::from_rust(self.value.clone().with_limit(limit)))
    }
}

/// One immutable event emitted by parser tracing.
#[invariant(value.byte_start <= value.byte_end)]
#[invariant(value.phase != TracePhase::All)]
#[invariant(!value.label.is_empty())]
#[pyclass(
    name = "TraceEvent",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyTraceEvent {
    value: TraceEvent,
}

impl PyTraceEvent {
    #[requires(value.byte_start <= value.byte_end)]
    #[requires(value.phase != TracePhase::All)]
    #[requires(!value.label.is_empty())]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from_rust(value: TraceEvent) -> Self {
        new!(PyTraceEvent { value })
    }
}

#[pymethods]
impl PyTraceEvent {
    /// Construct a validated trace event.
    #[allow(clippy::too_many_arguments)]
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (phase, level, depth, kind, label, byte_start, byte_end, detail=None))]
    fn new(
        py: Python<'_>,
        phase: &Bound<'_, PyAny>,
        level: &Bound<'_, PyAny>,
        depth: usize,
        kind: &Bound<'_, PyAny>,
        label: String,
        byte_start: usize,
        byte_end: usize,
        detail: Option<String>,
    ) -> PyResult<Self> {
        let phase = enum_from_python(py, phase)?;
        let level = enum_from_python(py, level)?;
        let kind = enum_from_python(py, kind)?;
        if phase == TracePhase::All {
            return Err(InvalidInputError::new_err(
                "trace event phase must be morphology or syntax, not all",
            ));
        }
        if label.is_empty() {
            return Err(InvalidInputError::new_err(
                "trace event label must not be empty",
            ));
        }
        if byte_start > byte_end {
            return Err(InvalidInputError::new_err(
                "trace event byte range must be ordered",
            ));
        }
        Ok(Self::from_rust(new!(TraceEvent {
            phase,
            level,
            depth,
            kind,
            label,
            byte_start,
            byte_end,
            detail,
        })))
    }

    /// Return the concrete parser phase.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn phase(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.phase)
    }

    /// Return the event detail level.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn level(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.level)
    }

    /// Return the parser nesting depth.
    #[requires(true)]
    #[ensures(ret == self.value.depth)]
    #[getter]
    fn depth(&self) -> usize {
        self.value.depth
    }

    /// Return the trace event kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.kind)
    }

    /// Return the event label.
    #[requires(true)]
    #[ensures(ret == self.value.label.as_str())]
    #[getter]
    fn label(&self) -> &str {
        &self.value.label
    }

    /// Return the inclusive event byte offset.
    #[requires(true)]
    #[ensures(ret == self.value.byte_start)]
    #[getter]
    fn byte_start(&self) -> usize {
        self.value.byte_start
    }

    /// Return the exclusive event byte offset.
    #[requires(true)]
    #[ensures(ret == self.value.byte_end)]
    #[getter]
    fn byte_end(&self) -> usize {
        self.value.byte_end
    }

    /// Return optional event-specific detail text.
    #[requires(true)]
    #[ensures(ret == self.value.detail.as_deref())]
    #[getter]
    fn detail(&self) -> Option<&str> {
        self.value.detail.as_deref()
    }
}

/// Grammar construct active at a trace location.
#[invariant(value.byte_start <= value.byte_end)]
#[invariant(!value.construct.is_empty())]
#[pyclass(
    name = "TraceContext",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyTraceContext {
    value: TraceContext,
}

impl PyTraceContext {
    #[requires(value.byte_start <= value.byte_end)]
    #[requires(!value.construct.is_empty())]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from_rust(value: TraceContext) -> Self {
        new!(PyTraceContext { value })
    }
}

#[pymethods]
impl PyTraceContext {
    /// Construct a validated grammar context and byte range.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(construct: String, byte_start: usize, byte_end: usize) -> PyResult<Self> {
        if construct.is_empty() {
            return Err(InvalidInputError::new_err(
                "trace context construct must not be empty",
            ));
        }
        if byte_start > byte_end {
            return Err(InvalidInputError::new_err(
                "trace context byte range must be ordered",
            ));
        }
        Ok(Self::from_rust(TraceContext::new(
            construct, byte_start, byte_end,
        )))
    }

    /// Return the grammar construct name.
    #[requires(true)]
    #[ensures(ret == self.value.construct.as_str())]
    #[getter]
    fn construct(&self) -> &str {
        &self.value.construct
    }

    /// Return the inclusive context byte offset.
    #[requires(true)]
    #[ensures(ret == self.value.byte_start)]
    #[getter]
    fn byte_start(&self) -> usize {
        self.value.byte_start
    }

    /// Return the exclusive context byte offset.
    #[requires(true)]
    #[ensures(ret == self.value.byte_end)]
    #[getter]
    fn byte_end(&self) -> usize {
        self.value.byte_end
    }
}

/// One expected-path branch in a trace failure summary.
#[invariant(true)]
#[pyclass(
    name = "TraceFailureBranch",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyTraceFailureBranch {
    value: TraceFailureBranch,
}

impl PyTraceFailureBranch {
    #[requires(true)]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from_rust(value: TraceFailureBranch) -> Self {
        Self { value }
    }
}

#[pymethods]
impl PyTraceFailureBranch {
    /// Construct a failure branch from contexts and expected items.
    #[requires(true)]
    #[ensures(ret.value.contexts.len() == contexts.len())]
    #[ensures(ret.value.expected.len() == old(expected.len()))]
    #[new]
    #[pyo3(signature = (contexts=Vec::new(), expected=Vec::new()))]
    fn new(contexts: Vec<PyRef<'_, PyTraceContext>>, expected: Vec<String>) -> Self {
        Self::from_rust(new!(TraceFailureBranch {
            contexts: contexts
                .iter()
                .map(|context| context.value.clone())
                .collect(),
            expected,
        }))
    }

    /// Return the immutable context stack.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn contexts(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            self.value
                .contexts
                .iter()
                .cloned()
                .map(PyTraceContext::from_rust),
        )
        .map(Bound::unbind)
    }

    /// Return the immutable expected-item collection.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn expected(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(py, self.value.expected.iter().cloned()).map(Bound::unbind)
    }
}

/// Structured summary of the parser's furthest traced failure.
#[invariant(value.byte_start <= value.byte_end)]
#[invariant(!value.reason.is_empty())]
#[pyclass(
    name = "TraceFailureSummary",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyTraceFailureSummary {
    value: TraceFailureSummary,
}

impl PyTraceFailureSummary {
    #[requires(value.byte_start <= value.byte_end)]
    #[requires(!value.reason.is_empty())]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from_rust(value: TraceFailureSummary) -> Self {
        new!(PyTraceFailureSummary { value })
    }
}

#[pymethods]
impl PyTraceFailureSummary {
    /// Construct a validated trace failure summary.
    #[allow(clippy::too_many_arguments)]
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (byte_start, byte_end, reason, branches=Vec::new(), current_context=None))]
    fn new(
        byte_start: usize,
        byte_end: usize,
        reason: String,
        branches: Vec<PyRef<'_, PyTraceFailureBranch>>,
        current_context: Option<PyRef<'_, PyTraceContext>>,
    ) -> PyResult<Self> {
        if byte_start > byte_end {
            return Err(InvalidInputError::new_err(
                "trace failure byte range must be ordered",
            ));
        }
        if reason.is_empty() {
            return Err(InvalidInputError::new_err(
                "trace failure reason must not be empty",
            ));
        }
        Ok(Self::from_rust(new!(TraceFailureSummary {
            byte_start,
            byte_end,
            reason,
            branches: branches.iter().map(|branch| branch.value.clone()).collect(),
            current_context: current_context
                .as_ref()
                .map(|context| context.value.clone()),
        })))
    }

    /// Return the inclusive failure byte offset.
    #[requires(true)]
    #[ensures(ret == self.value.byte_start)]
    #[getter]
    fn byte_start(&self) -> usize {
        self.value.byte_start
    }

    /// Return the exclusive failure byte offset.
    #[requires(true)]
    #[ensures(ret == self.value.byte_end)]
    #[getter]
    fn byte_end(&self) -> usize {
        self.value.byte_end
    }

    /// Return the summarized failure reason.
    #[requires(true)]
    #[ensures(ret == self.value.reason.as_str())]
    #[getter]
    fn reason(&self) -> &str {
        &self.value.reason
    }

    /// Return the immutable alternative failure branches.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn branches(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            self.value
                .branches
                .iter()
                .cloned()
                .map(PyTraceFailureBranch::from_rust),
        )
        .map(Bound::unbind)
    }

    /// Return the active context at failure, when available.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn current_context(&self) -> Option<PyTraceContext> {
        self.value
            .current_context
            .clone()
            .map(PyTraceContext::from_rust)
    }
}

/// Immutable trace report for one concrete parser phase.
#[invariant(value.phase != TracePhase::All)]
#[pyclass(
    name = "TraceReport",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyTraceReport {
    value: TraceReport,
}

impl PyTraceReport {
    #[requires(value.phase != TracePhase::All)]
    #[expensive_ensures(ret.value == old(value.clone()))]
    pub(crate) fn from_rust(value: TraceReport) -> Self {
        new!(PyTraceReport { value })
    }
}

#[pymethods]
impl PyTraceReport {
    /// Construct a trace report for one concrete parser phase.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (phase, events=Vec::new(), *, truncated=false, failure=None))]
    fn new(
        py: Python<'_>,
        phase: &Bound<'_, PyAny>,
        events: Vec<PyRef<'_, PyTraceEvent>>,
        truncated: bool,
        failure: Option<PyRef<'_, PyTraceFailureSummary>>,
    ) -> PyResult<Self> {
        let phase = enum_from_python(py, phase)?;
        if phase == TracePhase::All {
            return Err(InvalidInputError::new_err(
                "trace report phase must be morphology or syntax, not all",
            ));
        }
        Ok(Self::from_rust(new!(TraceReport {
            phase,
            events: events.iter().map(|event| event.value.clone()).collect(),
            truncated,
            failure: failure.as_ref().map(|failure| failure.value.clone()),
        })))
    }

    /// Return the concrete parser phase.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn phase(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.phase)
    }

    /// Return the immutable trace events.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn events(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            self.value
                .events
                .iter()
                .cloned()
                .map(PyTraceEvent::from_rust),
        )
        .map(Bound::unbind)
    }

    /// Report whether the configured trace event limit was reached.
    #[requires(true)]
    #[ensures(ret == self.value.truncated)]
    #[getter]
    fn truncated(&self) -> bool {
        self.value.truncated
    }

    /// Return the optional failure summary.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn failure(&self) -> Option<PyTraceFailureSummary> {
        self.value
            .failure
            .clone()
            .map(PyTraceFailureSummary::from_rust)
    }
}

/// Diagnostic hyperlink targeting a valsi dictionary entry.
#[invariant(matches!(value.as_data(), bityzba::data!(DiagnosticTextLink::VlackuWord { .. })))]
#[pyclass(
    name = "VlackuWordLink",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyVlackuWordLink {
    value: DiagnosticTextLink,
}

#[pymethods]
impl PyVlackuWordLink {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("word",);

    /// Construct a link to a non-empty valsi spelling.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(word: String) -> PyResult<Self> {
        if word.is_empty() {
            return Err(InvalidInputError::new_err(
                "vlacku link word must not be empty",
            ));
        }
        Ok(new!(PyVlackuWordLink {
            value: new!(DiagnosticTextLink::VlackuWord { word }),
        }))
    }

    /// Return the linked valsi spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn word(&self) -> &str {
        self.value
            .vlacku_word()
            .expect("wrapper variant guarantees a vlacku word")
    }
}

/// Diagnostic hyperlink targeting a CLL section and optional anchor.
#[invariant(matches!(value.as_data(), bityzba::data!(DiagnosticTextLink::CllSection { .. })))]
#[pyclass(
    name = "CllSectionLink",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyCllSectionLink {
    value: DiagnosticTextLink,
}

#[pymethods]
impl PyCllSectionLink {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("section_id", "anchor");

    /// Construct a link to a CLL section and optional anchor.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (section_id, anchor=None))]
    fn new(section_id: String, anchor: Option<String>) -> PyResult<Self> {
        if section_id.is_empty() {
            return Err(InvalidInputError::new_err(
                "CLL section id must not be empty",
            ));
        }
        if anchor.as_ref().is_some_and(String::is_empty) {
            return Err(InvalidInputError::new_err(
                "CLL section anchor must not be empty when present",
            ));
        }
        Ok(new!(PyCllSectionLink {
            value: new!(DiagnosticTextLink::CllSection { section_id, anchor }),
        }))
    }

    /// Return the linked CLL section identifier.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn section_id(&self) -> &str {
        self.value
            .cll_section()
            .expect("wrapper variant guarantees a CLL section")
            .0
    }

    /// Return the optional within-section anchor.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn anchor(&self) -> Option<&str> {
        self.value
            .cll_section()
            .expect("wrapper variant guarantees a CLL section")
            .1
    }
}

/// Diagnostic hyperlink targeting a named EBNF rule.
#[invariant(matches!(value.as_data(), bityzba::data!(DiagnosticTextLink::EbnfRule { .. })))]
#[pyclass(
    name = "EbnfRuleLink",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyEbnfRuleLink {
    value: DiagnosticTextLink,
}

#[pymethods]
impl PyEbnfRuleLink {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("rule_name",);

    /// Construct a link to a non-empty EBNF rule name.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(rule_name: String) -> PyResult<Self> {
        if rule_name.is_empty() {
            return Err(InvalidInputError::new_err(
                "EBNF rule name must not be empty",
            ));
        }
        Ok(new!(PyEbnfRuleLink {
            value: new!(DiagnosticTextLink::EbnfRule { rule_name }),
        }))
    }

    /// Return the linked EBNF rule name.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn rule_name(&self) -> &str {
        self.value
            .ebnf_rule()
            .expect("wrapper variant guarantees an EBNF rule")
    }
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_link_from_python(value: &Bound<'_, PyAny>) -> PyResult<DiagnosticTextLink> {
    if let Ok(value) = value.extract::<PyRef<'_, PyVlackuWordLink>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyCllSectionLink>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyEbnfRuleLink>>() {
        return Ok(value.value.clone());
    }
    Err(PyTypeError::new_err(
        "expected a jbotci.diagnostics diagnostic text link variant",
    ))
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_link_to_python(py: Python<'_>, value: DiagnosticTextLink) -> PyResult<Py<PyAny>> {
    let value = match value.as_data() {
        bityzba::data!(DiagnosticTextLink::VlackuWord { .. }) => {
            Py::new(py, new!(PyVlackuWordLink { value }))?.into_any()
        }
        bityzba::data!(DiagnosticTextLink::CllSection { .. }) => {
            Py::new(py, new!(PyCllSectionLink { value }))?.into_any()
        }
        bityzba::data!(DiagnosticTextLink::EbnfRule { .. }) => {
            Py::new(py, new!(PyEbnfRuleLink { value }))?.into_any()
        }
    };
    Ok(value)
}

/// Styled, optionally linked segment of diagnostic text.
#[invariant(!value.text.is_empty())]
#[pyclass(
    name = "DiagnosticTextSegment",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyDiagnosticTextSegment {
    value: DiagnosticTextSegment,
}

impl PyDiagnosticTextSegment {
    #[requires(!value.text.is_empty())]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from_rust(value: DiagnosticTextSegment) -> Self {
        new!(PyDiagnosticTextSegment { value })
    }
}

#[pymethods]
impl PyDiagnosticTextSegment {
    /// Construct a non-empty styled text segment with an optional typed link.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (role, text, *, link=None))]
    fn new(
        py: Python<'_>,
        role: &Bound<'_, PyAny>,
        text: String,
        link: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        if text.is_empty() {
            return Err(InvalidInputError::new_err(
                "diagnostic text segment must not be empty",
            ));
        }
        let role = enum_from_python(py, role)?;
        let value = match link {
            Some(link) => {
                DiagnosticTextSegment::with_link(role, text, diagnostic_link_from_python(link)?)
            }
            None => DiagnosticTextSegment::new(role, text),
        };
        Ok(Self::from_rust(value))
    }

    /// Return the semantic styling role.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn role(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.role)
    }

    /// Return the segment text.
    #[requires(true)]
    #[ensures(ret == self.value.text.as_str())]
    #[getter]
    fn text(&self) -> &str {
        &self.value.text
    }

    /// Return the optional typed diagnostic hyperlink.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn link(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.value
            .link
            .clone()
            .map(|link| diagnostic_link_to_python(py, link))
            .transpose()
    }
}

/// Diagnostic note with explicit display mode and styled segments.
#[invariant(!value.segments.is_empty())]
#[pyclass(
    name = "DiagnosticStyledNote",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyDiagnosticStyledNote {
    value: DiagnosticStyledNote,
}

impl PyDiagnosticStyledNote {
    #[requires(!value.segments.is_empty())]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from_rust(value: DiagnosticStyledNote) -> Self {
        new!(PyDiagnosticStyledNote { value })
    }
}

#[pymethods]
impl PyDiagnosticStyledNote {
    /// Construct a styled note from a non-empty segment sequence.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(
        py: Python<'_>,
        mode: &Bound<'_, PyAny>,
        segments: Vec<PyRef<'_, PyDiagnosticTextSegment>>,
    ) -> PyResult<Self> {
        if segments.is_empty() {
            return Err(InvalidInputError::new_err(
                "styled diagnostic note must contain at least one segment",
            ));
        }
        Ok(Self::from_rust(DiagnosticStyledNote::new(
            enum_from_python(py, mode)?,
            segments
                .iter()
                .map(|segment| segment.value.clone())
                .collect(),
        )))
    }

    /// Return the note's display mode.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn mode(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.mode)
    }

    /// Return the immutable styled text segments.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn segments(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            self.value
                .segments
                .iter()
                .cloned()
                .map(PyDiagnosticTextSegment::from_rust),
        )
        .map(Bound::unbind)
    }
}

/// Source span and message attached to a diagnostic.
#[invariant(!value.message.is_empty())]
#[pyclass(
    name = "DiagnosticLabel",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyDiagnosticLabel {
    value: DiagnosticLabel,
}

impl PyDiagnosticLabel {
    #[requires(!value.message.is_empty())]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from_rust(value: DiagnosticLabel) -> Self {
        new!(PyDiagnosticLabel { value })
    }
}

#[pymethods]
impl PyDiagnosticLabel {
    /// Construct a non-empty diagnostic label on a source span.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (span, message, *, primary=false))]
    fn new(span: PyRef<'_, PySourceSpan>, message: String, primary: bool) -> PyResult<Self> {
        if message.is_empty() {
            return Err(InvalidInputError::new_err(
                "diagnostic label message must not be empty",
            ));
        }
        Ok(Self::from_rust(DiagnosticLabel::new(
            span.clone_rust(),
            message,
            primary,
        )))
    }

    /// Return the labeled source span.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn span(&self) -> PySourceSpan {
        PySourceSpan::from_rust(self.value.span.clone())
    }

    /// Return the plain label message.
    #[requires(true)]
    #[ensures(ret == self.value.message.as_str())]
    #[getter]
    fn message(&self) -> &str {
        &self.value.message
    }

    /// Return the immutable styled message segments.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn message_segments(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            self.value
                .message_segments
                .iter()
                .cloned()
                .map(PyDiagnosticTextSegment::from_rust),
        )
        .map(Bound::unbind)
    }

    /// Report whether this is a primary diagnostic label.
    #[requires(true)]
    #[ensures(ret == self.value.primary)]
    #[getter]
    fn primary(&self) -> bool {
        self.value.primary
    }
}

/// Complete immutable diagnostic emitted by a jbotci phase.
#[invariant(!value.code.is_empty())]
#[invariant(!value.message.is_empty())]
#[invariant(!value.labels.is_empty())]
#[invariant(value.labels.iter().any(|label| label.primary))]
#[pyclass(
    name = "Diagnostic",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyDiagnostic {
    value: Diagnostic,
}

impl PyDiagnostic {
    #[requires(!value.code.is_empty())]
    #[requires(!value.message.is_empty())]
    #[requires(!value.labels.is_empty())]
    #[requires(value.labels.iter().any(|label| label.primary))]
    #[expensive_ensures(ret.value == old(value.clone()))]
    pub(crate) fn from_rust(value: Diagnostic) -> Self {
        new!(PyDiagnostic { value })
    }
}

#[pymethods]
impl PyDiagnostic {
    /// Construct a validated diagnostic with at least one primary label.
    #[allow(clippy::too_many_arguments)]
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (severity, phase, code, message, labels, notes=Vec::new(), *, styled_notes=Vec::new(), word_index=None))]
    fn new(
        py: Python<'_>,
        severity: &Bound<'_, PyAny>,
        phase: &Bound<'_, PyAny>,
        code: String,
        message: String,
        labels: Vec<PyRef<'_, PyDiagnosticLabel>>,
        notes: Vec<String>,
        styled_notes: Vec<PyRef<'_, PyDiagnosticStyledNote>>,
        word_index: Option<usize>,
    ) -> PyResult<Self> {
        if code.is_empty() {
            return Err(InvalidInputError::new_err(
                "diagnostic code must not be empty",
            ));
        }
        if message.is_empty() {
            return Err(InvalidInputError::new_err(
                "diagnostic message must not be empty",
            ));
        }
        if labels.is_empty() {
            return Err(InvalidInputError::new_err(
                "diagnostic must contain at least one label",
            ));
        }
        if !labels.iter().any(|label| label.value.primary) {
            return Err(InvalidInputError::new_err(
                "diagnostic must contain a primary label",
            ));
        }
        let value = Diagnostic::new(
            enum_from_python(py, severity)?,
            enum_from_python(py, phase)?,
            code,
            message,
            labels.iter().map(|label| label.value.clone()).collect(),
            notes,
            word_index,
        )
        .with_styled_notes(styled_notes.iter().map(|note| note.value.clone()).collect());
        Ok(Self::from_rust(value))
    }

    /// Return the diagnostic severity.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn severity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.severity)
    }

    /// Return the producing analysis phase.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn phase(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.value.phase)
    }

    /// Return the stable machine-readable diagnostic code.
    #[requires(true)]
    #[ensures(ret == self.value.code.as_str())]
    #[getter]
    fn code(&self) -> &str {
        &self.value.code
    }

    /// Return the plain diagnostic message.
    #[requires(true)]
    #[ensures(ret == self.value.message.as_str())]
    #[getter]
    fn message(&self) -> &str {
        &self.value.message
    }

    /// Return the immutable styled message segments.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn message_segments(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            self.value
                .message_segments
                .iter()
                .cloned()
                .map(PyDiagnosticTextSegment::from_rust),
        )
        .map(Bound::unbind)
    }

    /// Return the immutable diagnostic labels.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn labels(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            self.value
                .labels
                .iter()
                .cloned()
                .map(PyDiagnosticLabel::from_rust),
        )
        .map(Bound::unbind)
    }

    /// Return the immutable plain notes.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn notes(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(py, self.value.notes.iter().cloned()).map(Bound::unbind)
    }

    /// Return the immutable styled segments for plain notes.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn note_segments(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let notes = self
            .value
            .note_segments
            .iter()
            .map(|segments| {
                sequence_to_tuple(
                    py,
                    segments
                        .iter()
                        .cloned()
                        .map(PyDiagnosticTextSegment::from_rust),
                )
                .map(Bound::unbind)
            })
            .collect::<PyResult<Vec<_>>>()?;
        sequence_to_tuple(py, notes).map(Bound::unbind)
    }

    /// Return the immutable explicitly styled notes.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn styled_notes(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            self.value
                .styled_notes
                .iter()
                .cloned()
                .map(PyDiagnosticStyledNote::from_rust),
        )
        .map(Bound::unbind)
    }

    /// Return the optional word index associated with the diagnostic.
    #[requires(true)]
    #[ensures(ret == self.value.word_index)]
    #[getter]
    fn word_index(&self) -> Option<usize> {
        self.value.word_index
    }

    /// Return the first primary label.
    #[requires(true)]
    #[ensures(ret.value.primary)]
    #[getter]
    fn primary_label(&self) -> PyDiagnosticLabel {
        PyDiagnosticLabel::from_rust(self.value.primary_label().clone())
    }
}

/// Parse inline diagnostic markup into immutable styled segments.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
fn diagnostic_text_segments(text: &str, py: Python<'_>) -> PyResult<Py<PyTuple>> {
    sequence_to_tuple(
        py,
        rust_diagnostic_text_segments(text)
            .into_iter()
            .map(PyDiagnosticTextSegment::from_rust),
    )
    .map(Bound::unbind)
}

/// Concatenate styled diagnostic segments into their plain text.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
fn diagnostic_text_segments_text(segments: Vec<PyRef<'_, PyDiagnosticTextSegment>>) -> String {
    let segments = segments
        .iter()
        .map(|segment| segment.value.clone())
        .collect::<Vec<_>>();
    rust_diagnostic_text_segments_text(&segments)
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    register_string_enum::<DiagnosticSeverity>(module)?;
    register_string_enum::<DiagnosticPhase>(module)?;
    register_string_enum::<TracePhase>(module)?;
    register_string_enum::<TraceLevel>(module)?;
    register_string_enum::<TraceEventKind>(module)?;
    register_string_enum::<DiagnosticDetailMode>(module)?;
    register_string_enum::<DiagnosticNoteMode>(module)?;
    register_string_enum::<DiagnosticTextRole>(module)?;
    register_type::<PyTraceFilter>(module, "_diagnostics_TraceFilter")?;
    register_type::<PyTraceOptions>(module, "_diagnostics_TraceOptions")?;
    register_type::<PyTraceEvent>(module, "_diagnostics_TraceEvent")?;
    register_type::<PyTraceContext>(module, "_diagnostics_TraceContext")?;
    register_type::<PyTraceFailureBranch>(module, "_diagnostics_TraceFailureBranch")?;
    register_type::<PyTraceFailureSummary>(module, "_diagnostics_TraceFailureSummary")?;
    register_type::<PyTraceReport>(module, "_diagnostics_TraceReport")?;
    register_type::<PyVlackuWordLink>(module, "_diagnostics_VlackuWordLink")?;
    register_type::<PyCllSectionLink>(module, "_diagnostics_CllSectionLink")?;
    register_type::<PyEbnfRuleLink>(module, "_diagnostics_EbnfRuleLink")?;
    register_type::<PyDiagnosticTextSegment>(module, "_diagnostics_DiagnosticTextSegment")?;
    register_type::<PyDiagnosticStyledNote>(module, "_diagnostics_DiagnosticStyledNote")?;
    register_type::<PyDiagnosticLabel>(module, "_diagnostics_DiagnosticLabel")?;
    register_type::<PyDiagnostic>(module, "_diagnostics_Diagnostic")?;
    register_private_object(
        module,
        "_diagnostics_diagnostic_text_segments",
        wrap_pyfunction!(diagnostic_text_segments, module)?,
    )?;
    register_private_object(
        module,
        "_diagnostics_diagnostic_text_segments_text",
        wrap_pyfunction!(diagnostic_text_segments_text, module)?,
    )?;
    Ok(())
}
