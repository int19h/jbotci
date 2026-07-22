//! Immutable Python projection of diagnostics and parser trace products.

use std::borrow::Cow;
use std::sync::Arc;

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
    "_diagnostics_trace_phase_includes",
    "_diagnostics_trace_level_number",
    "_diagnostics_trace_level_from_number",
    "_diagnostics_diagnostic_note_mode_visible_in",
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
    #[ensures(ret.is_ok() == old(!name.is_empty()))]
    #[expensive_ensures(ret.is_err() || ret.as_ref().ok().map(|filter| filter.value.name.clone()) == Some(old(name.clone())))]
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
        enum_to_python(py, self.rust().phase)
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

    /// Return whether tracing is enabled for the requested parser phase.
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|included| self.value.enabled || !*included) || ret.is_err())]
    fn includes(&self, py: Python<'_>, phase: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.value.includes(enum_from_python(py, phase)?))
    }
}

#[invariant(::Owned { .. } => true)]
#[invariant(::Report { root, index } => *index < root.events.len())]
#[derive(Debug, Clone)]
enum TraceEventStorage {
    Owned {
        value: Arc<TraceEvent>,
    },
    Report {
        root: Arc<TraceReport>,
        index: usize,
    },
}

impl TraceEventStorage {
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &TraceEvent {
        match self {
            Self::Owned { value } => value.as_ref(),
            Self::Report { root, index } => &root.events[*index],
        }
    }
}

impl PartialEq for TraceEventStorage {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for TraceEventStorage {}

#[invariant(::Owned { .. } => true)]
#[invariant(::Report { root } => root.failure.is_some())]
#[derive(Debug, Clone)]
enum TraceFailureSummaryStorage {
    Owned { value: Arc<TraceFailureSummary> },
    Report { root: Arc<TraceReport> },
}

impl TraceFailureSummaryStorage {
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &TraceFailureSummary {
        match self {
            Self::Owned { value } => value.as_ref(),
            Self::Report { root } => root
                .failure
                .as_ref()
                .expect("typed trace-summary locator requires a report failure"),
        }
    }
}

impl PartialEq for TraceFailureSummaryStorage {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for TraceFailureSummaryStorage {}

#[invariant(::Owned { .. } => true)]
#[invariant(::Summary { owner, index } => *index < owner.get().branches.len())]
#[derive(Debug, Clone)]
enum TraceFailureBranchStorage {
    Owned {
        value: Arc<TraceFailureBranch>,
    },
    Summary {
        owner: TraceFailureSummaryStorage,
        index: usize,
    },
}

impl TraceFailureBranchStorage {
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &TraceFailureBranch {
        match self {
            Self::Owned { value } => value.as_ref(),
            Self::Summary { owner, index } => &owner.get().branches[*index],
        }
    }
}

impl PartialEq for TraceFailureBranchStorage {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for TraceFailureBranchStorage {}

#[invariant(::Owned { .. } => true)]
#[invariant(::Branch { owner, index } => *index < owner.get().contexts.len())]
#[invariant(::SummaryCurrent { owner } => owner.get().current_context.is_some())]
#[derive(Debug, Clone)]
enum TraceContextStorage {
    Owned {
        value: Arc<TraceContext>,
    },
    Branch {
        owner: TraceFailureBranchStorage,
        index: usize,
    },
    SummaryCurrent {
        owner: TraceFailureSummaryStorage,
    },
}

impl TraceContextStorage {
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &TraceContext {
        match self {
            Self::Owned { value } => value.as_ref(),
            Self::Branch { owner, index } => &owner.get().contexts[*index],
            Self::SummaryCurrent { owner } => owner
                .get()
                .current_context
                .as_ref()
                .expect("typed context locator requires a current context"),
        }
    }
}

impl PartialEq for TraceContextStorage {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for TraceContextStorage {}

/// One immutable event emitted by parser tracing.
#[invariant(value.get().byte_start <= value.get().byte_end)]
#[invariant(value.get().phase != TracePhase::All)]
#[invariant(!value.get().label.is_empty())]
#[pyclass(
    name = "TraceEvent",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyTraceEvent {
    value: TraceEventStorage,
}

impl PyTraceEvent {
    #[requires(value.byte_start <= value.byte_end)]
    #[requires(value.phase != TracePhase::All)]
    #[requires(!value.label.is_empty())]
    #[expensive_ensures(ret.value.get() == &old(value.clone()))]
    fn from_rust(value: TraceEvent) -> Self {
        new!(PyTraceEvent {
            value: new!(TraceEventStorage::Owned {
                value: Arc::new(value),
            }),
        })
    }

    #[requires(index < root.events.len())]
    #[expensive_ensures(ret.value.get() == &old(root.clone()).events[index])]
    fn from_report(root: Arc<TraceReport>, index: usize) -> Self {
        new!(PyTraceEvent {
            value: new!(TraceEventStorage::Report { root, index }),
        })
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    fn rust(&self) -> &TraceEvent {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(ret == self.value.get().clone())]
    fn clone_rust(&self) -> TraceEvent {
        self.value.get().clone()
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
        enum_to_python(py, self.rust().phase)
    }

    /// Return the event detail level.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn level(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.rust().level)
    }

    /// Return the parser nesting depth.
    #[requires(true)]
    #[ensures(ret == self.value.get().depth)]
    #[getter]
    fn depth(&self) -> usize {
        self.rust().depth
    }

    /// Return the trace event kind.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn kind(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.rust().kind)
    }

    /// Return the event label.
    #[requires(true)]
    #[ensures(ret == self.value.get().label.as_str())]
    #[getter]
    fn label(&self) -> &str {
        &self.rust().label
    }

    /// Return the inclusive event byte offset.
    #[requires(true)]
    #[ensures(ret == self.value.get().byte_start)]
    #[getter]
    fn byte_start(&self) -> usize {
        self.rust().byte_start
    }

    /// Return the exclusive event byte offset.
    #[requires(true)]
    #[ensures(ret == self.value.get().byte_end)]
    #[getter]
    fn byte_end(&self) -> usize {
        self.rust().byte_end
    }

    /// Return optional event-specific detail text.
    #[requires(true)]
    #[ensures(ret == self.value.get().detail.as_deref())]
    #[getter]
    fn detail(&self) -> Option<&str> {
        self.rust().detail.as_deref()
    }
}

/// Grammar construct active at a trace location.
#[invariant(value.get().byte_start <= value.get().byte_end)]
#[invariant(!value.get().construct.is_empty())]
#[pyclass(
    name = "TraceContext",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyTraceContext {
    value: TraceContextStorage,
}

impl PyTraceContext {
    #[requires(value.byte_start <= value.byte_end)]
    #[requires(!value.construct.is_empty())]
    #[expensive_ensures(ret.value.get() == &old(value.clone()))]
    fn from_rust(value: TraceContext) -> Self {
        new!(PyTraceContext {
            value: new!(TraceContextStorage::Owned {
                value: Arc::new(value),
            }),
        })
    }

    #[requires(index < owner.get().contexts.len())]
    #[expensive_ensures(ret.value.get() == &old(owner.clone()).get().contexts[index])]
    fn from_branch(owner: TraceFailureBranchStorage, index: usize) -> Self {
        new!(PyTraceContext {
            value: new!(TraceContextStorage::Branch { owner, index }),
        })
    }

    #[requires(owner.get().current_context.is_some())]
    #[expensive_ensures(ret.value.get() == old(owner.clone()).get().current_context.as_ref().unwrap())]
    fn from_summary_current(owner: TraceFailureSummaryStorage) -> Self {
        new!(PyTraceContext {
            value: new!(TraceContextStorage::SummaryCurrent { owner }),
        })
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    fn rust(&self) -> &TraceContext {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(ret == self.value.get().clone())]
    fn clone_rust(&self) -> TraceContext {
        self.value.get().clone()
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
    #[ensures(ret == self.value.get().construct.as_str())]
    #[getter]
    fn construct(&self) -> &str {
        &self.rust().construct
    }

    /// Return the inclusive context byte offset.
    #[requires(true)]
    #[ensures(ret == self.value.get().byte_start)]
    #[getter]
    fn byte_start(&self) -> usize {
        self.rust().byte_start
    }

    /// Return the exclusive context byte offset.
    #[requires(true)]
    #[ensures(ret == self.value.get().byte_end)]
    #[getter]
    fn byte_end(&self) -> usize {
        self.rust().byte_end
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
    value: TraceFailureBranchStorage,
}

impl PyTraceFailureBranch {
    #[requires(true)]
    #[expensive_ensures(ret.value.get() == &old(value.clone()))]
    fn from_rust(value: TraceFailureBranch) -> Self {
        Self {
            value: new!(TraceFailureBranchStorage::Owned {
                value: Arc::new(value),
            }),
        }
    }

    #[requires(index < owner.get().branches.len())]
    #[expensive_ensures(ret.value.get() == &old(owner.clone()).get().branches[index])]
    fn from_summary(owner: TraceFailureSummaryStorage, index: usize) -> Self {
        Self {
            value: new!(TraceFailureBranchStorage::Summary { owner, index }),
        }
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    fn rust(&self) -> &TraceFailureBranch {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(ret == self.value.get().clone())]
    fn clone_rust(&self) -> TraceFailureBranch {
        self.value.get().clone()
    }
}

#[pymethods]
impl PyTraceFailureBranch {
    /// Construct a failure branch from contexts and expected items.
    #[requires(true)]
    #[ensures(ret.value.get().contexts.len() == contexts.len())]
    #[ensures(ret.value.get().expected.len() == old(expected.len()))]
    #[new]
    #[pyo3(signature = (contexts=Vec::new(), expected=Vec::new()))]
    fn new(contexts: Vec<PyRef<'_, PyTraceContext>>, expected: Vec<String>) -> Self {
        Self::from_rust(new!(TraceFailureBranch {
            contexts: contexts
                .iter()
                .map(|context| context.clone_rust())
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
            (0..self.rust().contexts.len())
                .map(|index| PyTraceContext::from_branch(self.value.clone(), index)),
        )
        .map(Bound::unbind)
    }

    /// Return the immutable expected-item collection.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn expected(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(py, self.rust().expected.iter().map(String::as_str)).map(Bound::unbind)
    }
}

/// Structured summary of the parser's furthest traced failure.
#[invariant(value.get().byte_start <= value.get().byte_end)]
#[invariant(!value.get().reason.is_empty())]
#[pyclass(
    name = "TraceFailureSummary",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyTraceFailureSummary {
    value: TraceFailureSummaryStorage,
}

impl PyTraceFailureSummary {
    #[requires(value.byte_start <= value.byte_end)]
    #[requires(!value.reason.is_empty())]
    #[expensive_ensures(ret.value.get() == &old(value.clone()))]
    fn from_rust(value: TraceFailureSummary) -> Self {
        new!(PyTraceFailureSummary {
            value: new!(TraceFailureSummaryStorage::Owned {
                value: Arc::new(value),
            }),
        })
    }

    #[requires(root.failure.is_some())]
    #[expensive_ensures(ret.value.get() == old(root.clone()).failure.as_ref().unwrap())]
    fn from_report(root: Arc<TraceReport>) -> Self {
        new!(PyTraceFailureSummary {
            value: new!(TraceFailureSummaryStorage::Report { root }),
        })
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    fn rust(&self) -> &TraceFailureSummary {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(ret == self.value.get().clone())]
    fn clone_rust(&self) -> TraceFailureSummary {
        self.value.get().clone()
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
            branches: branches.iter().map(|branch| branch.clone_rust()).collect(),
            current_context: current_context.as_ref().map(|context| context.clone_rust()),
        })))
    }

    /// Return the inclusive failure byte offset.
    #[requires(true)]
    #[ensures(ret == self.value.get().byte_start)]
    #[getter]
    fn byte_start(&self) -> usize {
        self.rust().byte_start
    }

    /// Return the exclusive failure byte offset.
    #[requires(true)]
    #[ensures(ret == self.value.get().byte_end)]
    #[getter]
    fn byte_end(&self) -> usize {
        self.rust().byte_end
    }

    /// Return the summarized failure reason.
    #[requires(true)]
    #[ensures(ret == self.value.get().reason.as_str())]
    #[getter]
    fn reason(&self) -> &str {
        &self.rust().reason
    }

    /// Return the immutable alternative failure branches.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn branches(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            (0..self.rust().branches.len())
                .map(|index| PyTraceFailureBranch::from_summary(self.value.clone(), index)),
        )
        .map(Bound::unbind)
    }

    /// Return the active context at failure, when available.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn current_context(&self) -> Option<PyTraceContext> {
        self.rust()
            .current_context
            .as_ref()
            .map(|_| PyTraceContext::from_summary_current(self.value.clone()))
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
    value: Arc<TraceReport>,
}

impl PyTraceReport {
    #[requires(value.phase != TracePhase::All)]
    #[expensive_ensures(ret.value.as_ref() == &old(value.clone()))]
    pub(crate) fn from_rust(value: TraceReport) -> Self {
        new!(PyTraceReport {
            value: Arc::new(value),
        })
    }

    #[requires(true)]
    #[ensures(ret == self.value.as_ref())]
    fn rust(&self) -> &TraceReport {
        self.value.as_ref()
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
        if events.iter().any(|event| event.rust().phase != phase) {
            return Err(InvalidInputError::new_err(
                "every trace event must have the same phase as its report",
            ));
        }
        Ok(Self::from_rust(new!(TraceReport {
            phase,
            events: events.iter().map(|event| event.clone_rust()).collect(),
            truncated,
            failure: failure.as_ref().map(|failure| failure.clone_rust()),
        })))
    }

    /// Return the concrete parser phase.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn phase(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.rust().phase)
    }

    /// Return the immutable trace events.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn events(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            (0..self.rust().events.len())
                .map(|index| PyTraceEvent::from_report(Arc::clone(&self.value), index)),
        )
        .map(Bound::unbind)
    }

    /// Report whether the configured trace event limit was reached.
    #[requires(true)]
    #[ensures(ret == self.value.truncated)]
    #[getter]
    fn truncated(&self) -> bool {
        self.rust().truncated
    }

    /// Return the optional failure summary.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn failure(&self) -> Option<PyTraceFailureSummary> {
        self.rust()
            .failure
            .as_ref()
            .map(|_| PyTraceFailureSummary::from_report(Arc::clone(&self.value)))
    }
}

#[invariant(::Owned { .. } => true)]
#[invariant(::Diagnostic { root, index } => *index < root.styled_notes.len())]
#[derive(Debug, Clone)]
enum DiagnosticStyledNoteStorage {
    Owned { value: Arc<DiagnosticStyledNote> },
    Diagnostic { root: Arc<Diagnostic>, index: usize },
}

impl DiagnosticStyledNoteStorage {
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &DiagnosticStyledNote {
        match self {
            Self::Owned { value } => value.as_ref(),
            Self::Diagnostic { root, index } => &root.styled_notes[*index],
        }
    }
}

impl PartialEq for DiagnosticStyledNoteStorage {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for DiagnosticStyledNoteStorage {}

#[invariant(::Owned { .. } => true)]
#[invariant(::Diagnostic { root, index } => *index < root.labels.len())]
#[derive(Debug, Clone)]
enum DiagnosticLabelStorage {
    Owned { value: Arc<DiagnosticLabel> },
    Diagnostic { root: Arc<Diagnostic>, index: usize },
}

impl DiagnosticLabelStorage {
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &DiagnosticLabel {
        match self {
            Self::Owned { value } => value.as_ref(),
            Self::Diagnostic { root, index } => &root.labels[*index],
        }
    }
}

impl PartialEq for DiagnosticLabelStorage {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for DiagnosticLabelStorage {}

#[invariant(::Owned { .. } => true)]
#[invariant(::StyledNote { owner, index } => *index < owner.get().segments.len())]
#[invariant(::DiagnosticMessage { root, index } => *index < root.message_segments.len())]
#[invariant(::DiagnosticNote { root, note_index, segment_index } => *note_index < root.note_segments.len() && *segment_index < root.note_segments[*note_index].len())]
#[invariant(::LabelMessage { owner, index } => *index < owner.get().message_segments.len())]
#[derive(Debug, Clone)]
enum DiagnosticTextSegmentStorage {
    Owned {
        value: Arc<DiagnosticTextSegment>,
    },
    StyledNote {
        owner: DiagnosticStyledNoteStorage,
        index: usize,
    },
    DiagnosticMessage {
        root: Arc<Diagnostic>,
        index: usize,
    },
    DiagnosticNote {
        root: Arc<Diagnostic>,
        note_index: usize,
        segment_index: usize,
    },
    LabelMessage {
        owner: DiagnosticLabelStorage,
        index: usize,
    },
}

impl DiagnosticTextSegmentStorage {
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &DiagnosticTextSegment {
        match self {
            Self::Owned { value } => value.as_ref(),
            Self::StyledNote { owner, index } => &owner.get().segments[*index],
            Self::DiagnosticMessage { root, index } => &root.message_segments[*index],
            Self::DiagnosticNote {
                root,
                note_index,
                segment_index,
            } => &root.note_segments[*note_index][*segment_index],
            Self::LabelMessage { owner, index } => &owner.get().message_segments[*index],
        }
    }
}

impl PartialEq for DiagnosticTextSegmentStorage {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for DiagnosticTextSegmentStorage {}

#[invariant(::Owned { .. } => true)]
#[invariant(::Segment { owner } => owner.get().link.is_some())]
#[derive(Debug, Clone)]
enum DiagnosticLinkStorage {
    Owned { value: Arc<DiagnosticTextLink> },
    Segment { owner: DiagnosticTextSegmentStorage },
}

impl DiagnosticLinkStorage {
    #[requires(true)]
    #[ensures(true)]
    fn get(&self) -> &DiagnosticTextLink {
        match self {
            Self::Owned { value } => value.as_ref(),
            Self::Segment { owner } => owner
                .get()
                .link
                .as_ref()
                .expect("typed diagnostic-link locator requires a linked segment"),
        }
    }
}

impl PartialEq for DiagnosticLinkStorage {
    #[requires(true)]
    #[ensures(ret == (self.get() == other.get()))]
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for DiagnosticLinkStorage {}

/// Diagnostic hyperlink targeting a valsi dictionary entry.
#[invariant(matches!(value.get().as_data(), bityzba::data!(DiagnosticTextLink::VlackuWord { .. })))]
#[pyclass(
    name = "VlackuWordLink",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyVlackuWordLink {
    value: DiagnosticLinkStorage,
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
            value: new!(DiagnosticLinkStorage::Owned {
                value: Arc::new(new!(DiagnosticTextLink::VlackuWord { word })),
            }),
        }))
    }

    /// Return the linked valsi spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn word(&self) -> &str {
        self.value
            .get()
            .vlacku_word()
            .expect("wrapper variant guarantees a vlacku word")
    }
}

/// Diagnostic hyperlink targeting a CLL section and optional anchor.
#[invariant(matches!(value.get().as_data(), bityzba::data!(DiagnosticTextLink::CllSection { .. })))]
#[pyclass(
    name = "CllSectionLink",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyCllSectionLink {
    value: DiagnosticLinkStorage,
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
            value: new!(DiagnosticLinkStorage::Owned {
                value: Arc::new(new!(DiagnosticTextLink::CllSection { section_id, anchor })),
            }),
        }))
    }

    /// Return the linked CLL section identifier.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn section_id(&self) -> &str {
        self.value
            .get()
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
            .get()
            .cll_section()
            .expect("wrapper variant guarantees a CLL section")
            .1
    }
}

/// Diagnostic hyperlink targeting a named EBNF rule.
#[invariant(matches!(value.get().as_data(), bityzba::data!(DiagnosticTextLink::EbnfRule { .. })))]
#[pyclass(
    name = "EbnfRuleLink",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyEbnfRuleLink {
    value: DiagnosticLinkStorage,
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
            value: new!(DiagnosticLinkStorage::Owned {
                value: Arc::new(new!(DiagnosticTextLink::EbnfRule { rule_name })),
            }),
        }))
    }

    /// Return the linked EBNF rule name.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn rule_name(&self) -> &str {
        self.value
            .get()
            .ebnf_rule()
            .expect("wrapper variant guarantees an EBNF rule")
    }
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_link_from_python(value: &Bound<'_, PyAny>) -> PyResult<DiagnosticTextLink> {
    if let Ok(value) = value.extract::<PyRef<'_, PyVlackuWordLink>>() {
        return Ok(value.value.get().clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyCllSectionLink>>() {
        return Ok(value.value.get().clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyEbnfRuleLink>>() {
        return Ok(value.value.get().clone());
    }
    Err(PyTypeError::new_err(
        "expected a jbotci.diagnostics diagnostic text link variant",
    ))
}

#[requires(true)]
#[ensures(true)]
fn diagnostic_link_to_python(py: Python<'_>, value: DiagnosticLinkStorage) -> PyResult<Py<PyAny>> {
    let value = match value.get().as_data() {
        bityzba::data!(DiagnosticTextLink::VlackuWord { .. }) => Py::new(
            py,
            new!(PyVlackuWordLink {
                value: value.clone(),
            }),
        )?
        .into_any(),
        bityzba::data!(DiagnosticTextLink::CllSection { .. }) => Py::new(
            py,
            new!(PyCllSectionLink {
                value: value.clone(),
            }),
        )?
        .into_any(),
        bityzba::data!(DiagnosticTextLink::EbnfRule { .. }) => Py::new(
            py,
            new!(PyEbnfRuleLink {
                value: value.clone(),
            }),
        )?
        .into_any(),
    };
    Ok(value)
}

/// Styled, optionally linked segment of diagnostic text.
#[invariant(!value.get().text.is_empty())]
#[pyclass(
    name = "DiagnosticTextSegment",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyDiagnosticTextSegment {
    value: DiagnosticTextSegmentStorage,
}

impl PyDiagnosticTextSegment {
    #[requires(!value.text.is_empty())]
    #[expensive_ensures(ret.value.get() == &old(value.clone()))]
    fn from_rust(value: DiagnosticTextSegment) -> Self {
        new!(PyDiagnosticTextSegment {
            value: new!(DiagnosticTextSegmentStorage::Owned {
                value: Arc::new(value),
            }),
        })
    }

    #[requires(index < owner.get().segments.len())]
    #[expensive_ensures(ret.value.get() == &old(owner.clone()).get().segments[index])]
    fn from_styled_note(owner: DiagnosticStyledNoteStorage, index: usize) -> Self {
        new!(PyDiagnosticTextSegment {
            value: new!(DiagnosticTextSegmentStorage::StyledNote { owner, index }),
        })
    }

    #[requires(index < root.message_segments.len())]
    #[expensive_ensures(ret.value.get() == &old(root.clone()).message_segments[index])]
    fn from_diagnostic_message(root: Arc<Diagnostic>, index: usize) -> Self {
        new!(PyDiagnosticTextSegment {
            value: new!(DiagnosticTextSegmentStorage::DiagnosticMessage { root, index }),
        })
    }

    #[requires(note_index < root.note_segments.len())]
    #[requires(segment_index < root.note_segments[note_index].len())]
    #[expensive_ensures(ret.value.get() == &old(root.clone()).note_segments[note_index][segment_index])]
    fn from_diagnostic_note(
        root: Arc<Diagnostic>,
        note_index: usize,
        segment_index: usize,
    ) -> Self {
        new!(PyDiagnosticTextSegment {
            value: new!(DiagnosticTextSegmentStorage::DiagnosticNote {
                root,
                note_index,
                segment_index,
            }),
        })
    }

    #[requires(index < owner.get().message_segments.len())]
    #[expensive_ensures(ret.value.get() == &old(owner.clone()).get().message_segments[index])]
    fn from_label_message(owner: DiagnosticLabelStorage, index: usize) -> Self {
        new!(PyDiagnosticTextSegment {
            value: new!(DiagnosticTextSegmentStorage::LabelMessage { owner, index }),
        })
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    fn rust(&self) -> &DiagnosticTextSegment {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(ret == self.value.get().clone())]
    fn clone_rust(&self) -> DiagnosticTextSegment {
        self.value.get().clone()
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
        enum_to_python(py, self.rust().role)
    }

    /// Return the segment text.
    #[requires(true)]
    #[ensures(ret == self.value.get().text.as_str())]
    #[getter]
    fn text(&self) -> &str {
        &self.rust().text
    }

    /// Return the optional typed diagnostic hyperlink.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn link(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        self.rust()
            .link
            .as_ref()
            .map(|_| {
                diagnostic_link_to_python(
                    py,
                    new!(DiagnosticLinkStorage::Segment {
                        owner: self.value.clone(),
                    }),
                )
            })
            .transpose()
    }
}

/// Diagnostic note with explicit display mode and styled segments.
#[invariant(!value.get().segments.is_empty())]
#[pyclass(
    name = "DiagnosticStyledNote",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyDiagnosticStyledNote {
    value: DiagnosticStyledNoteStorage,
}

impl PyDiagnosticStyledNote {
    #[requires(!value.segments.is_empty())]
    #[expensive_ensures(ret.value.get() == &old(value.clone()))]
    fn from_rust(value: DiagnosticStyledNote) -> Self {
        new!(PyDiagnosticStyledNote {
            value: new!(DiagnosticStyledNoteStorage::Owned {
                value: Arc::new(value),
            }),
        })
    }

    #[requires(index < root.styled_notes.len())]
    #[expensive_ensures(ret.value.get() == &old(root.clone()).styled_notes[index])]
    fn from_diagnostic(root: Arc<Diagnostic>, index: usize) -> Self {
        new!(PyDiagnosticStyledNote {
            value: new!(DiagnosticStyledNoteStorage::Diagnostic { root, index }),
        })
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    fn rust(&self) -> &DiagnosticStyledNote {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(ret == self.value.get().clone())]
    fn clone_rust(&self) -> DiagnosticStyledNote {
        self.value.get().clone()
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
                .map(|segment| segment.clone_rust())
                .collect(),
        )))
    }

    /// Return the note's display mode.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn mode(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.rust().mode)
    }

    /// Return the immutable styled text segments.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn segments(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            (0..self.rust().segments.len())
                .map(|index| PyDiagnosticTextSegment::from_styled_note(self.value.clone(), index)),
        )
        .map(Bound::unbind)
    }
}

/// Source span and message attached to a diagnostic.
#[invariant(!value.get().message.is_empty())]
#[pyclass(
    name = "DiagnosticLabel",
    frozen,
    eq,
    module = "jbotci.diagnostics",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyDiagnosticLabel {
    value: DiagnosticLabelStorage,
}

impl PyDiagnosticLabel {
    #[requires(!value.message.is_empty())]
    #[expensive_ensures(ret.value.get() == &old(value.clone()))]
    fn from_rust(value: DiagnosticLabel) -> Self {
        new!(PyDiagnosticLabel {
            value: new!(DiagnosticLabelStorage::Owned {
                value: Arc::new(value),
            }),
        })
    }

    #[requires(index < root.labels.len())]
    #[expensive_ensures(ret.value.get() == &old(root.clone()).labels[index])]
    fn from_diagnostic(root: Arc<Diagnostic>, index: usize) -> Self {
        new!(PyDiagnosticLabel {
            value: new!(DiagnosticLabelStorage::Diagnostic { root, index }),
        })
    }

    #[requires(true)]
    #[ensures(ret == self.value.get())]
    fn rust(&self) -> &DiagnosticLabel {
        self.value.get()
    }

    #[requires(true)]
    #[ensures(ret == self.value.get().clone())]
    fn clone_rust(&self) -> DiagnosticLabel {
        self.value.get().clone()
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
        PySourceSpan::from_rust(self.rust().span.clone())
    }

    /// Return the plain label message.
    #[requires(true)]
    #[ensures(ret == self.value.get().message.as_str())]
    #[getter]
    fn message(&self) -> &str {
        &self.rust().message
    }

    /// Return the immutable styled message segments.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn message_segments(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            (0..self.rust().message_segments.len()).map(|index| {
                PyDiagnosticTextSegment::from_label_message(self.value.clone(), index)
            }),
        )
        .map(Bound::unbind)
    }

    /// Report whether this is a primary diagnostic label.
    #[requires(true)]
    #[ensures(ret == self.value.get().primary)]
    #[getter]
    fn primary(&self) -> bool {
        self.rust().primary
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
    value: Arc<Diagnostic>,
}

impl PyDiagnostic {
    #[requires(!value.code.is_empty())]
    #[requires(!value.message.is_empty())]
    #[requires(!value.labels.is_empty())]
    #[requires(value.labels.iter().any(|label| label.primary))]
    #[expensive_ensures(ret.value.as_ref() == &old(value.clone()))]
    pub(crate) fn from_rust(value: Diagnostic) -> Self {
        new!(PyDiagnostic {
            value: Arc::new(value),
        })
    }

    #[requires(true)]
    #[ensures(ret == self.value.as_ref())]
    fn rust(&self) -> &Diagnostic {
        self.value.as_ref()
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
        if !labels.iter().any(|label| label.rust().primary) {
            return Err(InvalidInputError::new_err(
                "diagnostic must contain a primary label",
            ));
        }
        let value = Diagnostic::new(
            enum_from_python(py, severity)?,
            enum_from_python(py, phase)?,
            code,
            message,
            labels.iter().map(|label| label.clone_rust()).collect(),
            notes,
            word_index,
        )
        .with_styled_notes(styled_notes.iter().map(|note| note.clone_rust()).collect());
        Ok(Self::from_rust(value))
    }

    /// Return the diagnostic severity.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn severity(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.rust().severity)
    }

    /// Return the producing analysis phase.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn phase(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        enum_to_python(py, self.rust().phase)
    }

    /// Return the stable machine-readable diagnostic code.
    #[requires(true)]
    #[ensures(ret == self.value.code.as_str())]
    #[getter]
    fn code(&self) -> &str {
        &self.rust().code
    }

    /// Return the plain diagnostic message.
    #[requires(true)]
    #[ensures(ret == self.value.message.as_str())]
    #[getter]
    fn message(&self) -> &str {
        &self.rust().message
    }

    /// Return the immutable styled message segments.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn message_segments(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            (0..self.rust().message_segments.len()).map(|index| {
                PyDiagnosticTextSegment::from_diagnostic_message(Arc::clone(&self.value), index)
            }),
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
            (0..self.rust().labels.len())
                .map(|index| PyDiagnosticLabel::from_diagnostic(Arc::clone(&self.value), index)),
        )
        .map(Bound::unbind)
    }

    /// Return the immutable plain notes.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn notes(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(py, self.rust().notes.iter().map(String::as_str)).map(Bound::unbind)
    }

    /// Return the immutable styled segments for plain notes.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn note_segments(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let notes = self
            .rust()
            .note_segments
            .iter()
            .enumerate()
            .map(|(note_index, segments)| {
                sequence_to_tuple(
                    py,
                    (0..segments.len()).map(|segment_index| {
                        PyDiagnosticTextSegment::from_diagnostic_note(
                            Arc::clone(&self.value),
                            note_index,
                            segment_index,
                        )
                    }),
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
            (0..self.rust().styled_notes.len()).map(|index| {
                PyDiagnosticStyledNote::from_diagnostic(Arc::clone(&self.value), index)
            }),
        )
        .map(Bound::unbind)
    }

    /// Return the optional word index associated with the diagnostic.
    #[requires(true)]
    #[ensures(ret == self.value.word_index)]
    #[getter]
    fn word_index(&self) -> Option<usize> {
        self.rust().word_index
    }

    /// Return the first primary label.
    #[requires(true)]
    #[ensures(ret.value.get().primary)]
    #[getter]
    fn primary_label(&self) -> PyDiagnosticLabel {
        let index = self
            .rust()
            .labels
            .iter()
            .position(|label| label.primary)
            .expect("diagnostic invariant requires a primary label");
        PyDiagnosticLabel::from_diagnostic(Arc::clone(&self.value), index)
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
        .map(|segment| segment.clone_rust())
        .collect::<Vec<_>>();
    rust_diagnostic_text_segments_text(&segments)
}

/// Return whether one trace-phase selector includes another phase.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
fn trace_phase_includes(
    py: Python<'_>,
    selector: &Bound<'_, PyAny>,
    phase: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    Ok(enum_from_python::<TracePhase>(py, selector)?.includes(enum_from_python(py, phase)?))
}

/// Return the stable one-based numeric trace-detail level.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|number| (1..=4).contains(number)) || ret.is_err())]
#[pyfunction]
fn trace_level_number(py: Python<'_>, level: &Bound<'_, PyAny>) -> PyResult<u8> {
    Ok(enum_from_python::<TraceLevel>(py, level)?.number())
}

/// Convert a stable one-based number to its exact trace-detail level.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
fn trace_level_from_number(py: Python<'_>, value: i64) -> PyResult<Py<PyAny>> {
    let value = u8::try_from(value)
        .ok()
        .filter(|value| (1..=4).contains(value))
        .ok_or_else(|| {
            InvalidInputError::new_err("trace level number must be one of 1, 2, 3, or 4")
        })?;
    let level = TraceLevel::from_number(value)
        .map_err(|error| InvalidInputError::new_err(error.to_string()))?;
    enum_to_python(py, level)
}

/// Return whether a diagnostic note is visible in the selected detail mode.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
fn diagnostic_note_mode_visible_in(
    py: Python<'_>,
    note_mode: &Bound<'_, PyAny>,
    detail_mode: &Bound<'_, PyAny>,
) -> PyResult<bool> {
    Ok(enum_from_python::<DiagnosticNoteMode>(py, note_mode)?
        .visible_in(enum_from_python(py, detail_mode)?))
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
    register_private_object(
        module,
        "_diagnostics_trace_phase_includes",
        wrap_pyfunction!(trace_phase_includes, module)?,
    )?;
    register_private_object(
        module,
        "_diagnostics_trace_level_number",
        wrap_pyfunction!(trace_level_number, module)?,
    )?;
    register_private_object(
        module,
        "_diagnostics_trace_level_from_number",
        wrap_pyfunction!(trace_level_from_number, module)?,
    )?;
    register_private_object(
        module,
        "_diagnostics_diagnostic_note_mode_visible_in",
        wrap_pyfunction!(diagnostic_note_mode_visible_in, module)?,
    )?;
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
