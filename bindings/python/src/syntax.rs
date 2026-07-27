//! Generated strict and recovered syntax model bindings.

use std::sync::Arc;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires, try_new};
use jbotci_python_syntax_macros::generate_syntax_bindings;
use jbotci_syntax::tree::SyntaxRecoveryItemData;
use jbotci_syntax::{SyntaxRecoveryItem, Token, WithIndicators, WithIndicatorsData};
use jbotci_tree::TreePath;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyTuple};
use vec1::Vec1;

use crate::morphology::{
    TokenHandle, WithIndicatorsHandle, WithIndicatorsWordSlot, extract_word_like,
    token_core_word_to_python, with_indicators_core_word_to_python, with_indicators_word_to_python,
    word_handle_from_python,
};
use crate::source::PySourceSpan;
use crate::support::{register_private_object, register_type, sequence_to_tuple};

jbotci_syntax::__jbotci_syntax_binding_schema!(generate_syntax_bindings);

pub(crate) const NATIVE_EXPORTS: &[&str] = &[
    "_syntax_STRICT_SOURCE",
    "_syntax_RECOVERED_SOURCE",
    "_syntax_STRICT_STUB",
    "_syntax_RECOVERED_STUB",
    "_syntax_STRICT_INVENTORY",
    "_syntax_RECOVERED_INVENTORY",
    "_syntax_STRICT_CONCRETE_INVENTORY",
    "_syntax_RECOVERED_CONCRETE_INVENTORY",
    "_syntax_SCHEMA_MODEL_COUNT",
    "_syntax_SCHEMA_VARIANT_COUNT",
    "_syntax_SCHEMA_FIELD_COUNT",
    "_syntax_Value",
    "_syntax_Identity",
    "_syntax_Token",
    "_syntax_PlainWithIndicators",
    "_syntax_EmphasizedWithIndicators",
    "_syntax_IndicatorWithIndicators",
    "_syntax_SkippedTokens",
    "_syntax_MissingRequiredField",
    "_syntax_construct",
];

#[invariant(!lens.is_empty(), "projected wrapper identities retain a generated field lens")]
#[derive(Debug, Clone)]
struct SyntaxIdentity {
    owner: Arc<SyntaxOwner>,
    path: TreePath,
    lens: Vec<usize>,
}

#[invariant(
    true,
    "the backing identity enforces the non-empty generated field lens"
)]
#[pyclass(
    name = "_SyntaxIdentity",
    frozen,
    module = "jbotci._native",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PySyntaxIdentity {
    value: SyntaxIdentity,
}

#[pymethods]
impl PySyntaxIdentity {
    #[requires(true)]
    #[ensures(ret == (
        Arc::ptr_eq(&self.value.owner, &other.value.owner)
            && self.value.path == other.value.path
            && self.value.lens == other.value.lens
    ))]
    fn _same_identity(&self, other: &PySyntaxIdentity) -> bool {
        Arc::ptr_eq(&self.value.owner, &other.value.owner)
            && self.value.path == other.value.path
            && self.value.lens == other.value.lens
    }
}

/// One immutable source-backed syntax token with stable Arc identity.
#[invariant(true, "the canonical handle retains the exact Arc-backed syntax token")]
#[pyclass(name = "Token", frozen, module = "jbotci.syntax", skip_from_py_object)]
#[derive(Debug, Clone)]
struct PySyntaxToken {
    handle: TokenHandle,
}

#[pymethods]
impl PySyntaxToken {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("indicators",);

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(indicators: &Bound<'_, PyAny>) -> PyResult<Self> {
        let indicators = extract_with_indicators(indicators)?;
        Ok(Self {
            handle: TokenHandle::from_rust(Token::from_indicators(indicators.into_owned())),
        })
    }

    /// Return the complete emphasis-and-indicator tree for this token.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn indicators(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        with_indicators_to_python(py, self.handle.indicators())
    }

    /// Return the token's word-like value after stripping emphasis and indicators.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn core_word(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        token_core_word_to_python(py, self.handle.clone())
    }

    /// Return every exact source span attributed to this token.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn source_spans(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            self.handle
                .get()
                .source_spans()
                .into_iter()
                .map(|span| PySourceSpan::from_rust(span.clone())),
        )
        .map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(ret == self.handle.get().to_string())]
    fn __str__(&self) -> String {
        self.handle.get().to_string()
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!("jbotci.syntax.Token(indicators={:?})", self.handle.get())
    }

    #[requires(true)]
    #[ensures(ret == (self.handle.get() == other.handle.get()))]
    fn __eq__(&self, other: &PySyntaxToken) -> bool {
        self.handle.get() == other.handle.get()
    }

    /// Return whether both wrappers retain the same exact syntax token owner.
    #[requires(true)]
    #[ensures(ret == (self.handle == other.handle))]
    fn same_identity(&self, other: &PySyntaxToken) -> bool {
        self.handle == other.handle
    }
}

/// A word-like value without BAhE emphasis or attached indicators.
#[invariant(true, "the canonical handle selects a plain indicator tree")]
#[pyclass(
    name = "PlainWithIndicators",
    frozen,
    eq,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyPlainWithIndicators {
    handle: WithIndicatorsHandle,
}

#[pymethods]
impl PyPlainWithIndicators {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str,) = ("word_like",);

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(word_like: &Bound<'_, PyAny>) -> PyResult<Self> {
        let word_like = extract_word_like(word_like)?.into_owned();
        Ok(Self {
            handle: WithIndicatorsHandle::from_owned(WithIndicators::bare(word_like)),
        })
    }

    /// Return the plain word-like payload.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn word_like(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        with_indicators_core_word_to_python(py, self.handle.clone())
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!(
            "jbotci.syntax.PlainWithIndicators(word_like={:?})",
            self.handle.get().core_word()
        )
    }

    /// Return whether both wrappers select the same exact indicator tree.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn same_identity(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.handle.same_identity(&extract_with_indicators(other)?))
    }
}

/// A word-like value preceded by one or more BAhE-class modifiers.
#[invariant(true, "the canonical handle selects an emphasized indicator tree")]
#[pyclass(
    name = "EmphasizedWithIndicators",
    frozen,
    eq,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyEmphasizedWithIndicators {
    handle: WithIndicatorsHandle,
}

#[pymethods]
impl PyEmphasizedWithIndicators {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) =
        ("bahe", "extra_bahe", "word_like");

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(
        bahe: &Bound<'_, PyAny>,
        extra_bahe: &Bound<'_, PyAny>,
        word_like: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let bahe = word_handle_from_python(bahe)?.clone_rust();
        let extra_bahe = crate::support::extract_sequence(extra_bahe, "extra_bahe", |word| {
            Ok(word_handle_from_python(word)?.clone_rust())
        })?;
        let word_like = extract_word_like(word_like)?.into_owned();
        let value = try_new!(WithIndicators::Emphasized {
            bahe,
            extra_bahe,
            word_like,
        })
        .map_err(|_| {
            PyValueError::new_err("bahe and extra_bahe must contain only BAhE-class words")
        })?;
        Ok(Self {
            handle: WithIndicatorsHandle::from_owned(value),
        })
    }

    /// Return the required leading BAhE-class emphasis word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn bahe(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        indicator_word_required(py, &self.handle, WithIndicatorsWordSlot::EmphasisBahe)
    }

    /// Return the remaining source-ordered BAhE-class emphasis words.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn extra_bahe(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let count = match self.handle.get().as_data() {
            data!(WithIndicators::Emphasized { extra_bahe, .. }) => extra_bahe.len(),
            _ => unreachable!("private class fixes the indicator variant"),
        };
        indicator_word_sequence(py, &self.handle, count, |index| {
            WithIndicatorsWordSlot::ExtraEmphasisBahe { index }
        })
    }

    /// Return the emphasized word-like payload.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn word_like(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        with_indicators_core_word_to_python(py, self.handle.clone())
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!(
            "jbotci.syntax.EmphasizedWithIndicators({:?})",
            self.handle.get()
        )
    }

    /// Return whether both wrappers select the same exact indicator tree.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn same_identity(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.handle.same_identity(&extract_with_indicators(other)?))
    }
}

/// An indicator layer attached recursively to a base word-like value.
#[invariant(true, "the canonical handle selects an indicator-attached tree")]
#[pyclass(
    name = "IndicatorWithIndicators",
    frozen,
    eq,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyIndicatorWithIndicators {
    handle: WithIndicatorsHandle,
}

#[pymethods]
impl PyIndicatorWithIndicators {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        &'static str,
    ) = ("base", "indicator_bahe", "indicator", "nai_bahe", "nai");

    #[allow(clippy::too_many_arguments)]
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(
        base: &Bound<'_, PyAny>,
        indicator_bahe: &Bound<'_, PyAny>,
        indicator: &Bound<'_, PyAny>,
        nai_bahe: &Bound<'_, PyAny>,
        nai: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let base = extract_with_indicators(base)?.into_owned();
        let indicator_bahe = extract_word_sequence(indicator_bahe, "indicator_bahe")?;
        let indicator = word_handle_from_python(indicator)?.clone_rust();
        let nai_bahe = extract_word_sequence(nai_bahe, "nai_bahe")?;
        let nai = nai
            .map(word_handle_from_python)
            .transpose()?
            .map(|word| word.clone_rust());
        let value = try_new!(WithIndicators::WithIndicator {
            base: Arc::new(base),
            indicator_bahe,
            indicator,
            nai_bahe,
            nai,
        })
        .map_err(|_| {
            PyValueError::new_err(
                "invalid indicator structure: modifiers must be BAhE-class words, the indicator must be UI/CAI/Y, and nai must be NAI",
            )
        })?;
        Ok(Self {
            handle: WithIndicatorsHandle::from_owned(value),
        })
    }

    /// Return the recursively decorated base value.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn base(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        with_indicators_to_python(
            py,
            self.handle
                .base()
                .expect("private class fixes the indicator variant"),
        )
    }

    /// Return the source-ordered BAhE-class words preceding the indicator.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn indicator_bahe(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let count = match self.handle.get().as_data() {
            data!(WithIndicators::WithIndicator { indicator_bahe, .. }) => indicator_bahe.len(),
            _ => unreachable!("private class fixes the indicator variant"),
        };
        indicator_word_sequence(py, &self.handle, count, |index| {
            WithIndicatorsWordSlot::IndicatorBahe { index }
        })
    }

    /// Return the required UI-, CAI-, or Y-class indicator word.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn indicator(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        indicator_word_required(py, &self.handle, WithIndicatorsWordSlot::Indicator)
    }

    /// Return the source-ordered BAhE-class words preceding NAI.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn nai_bahe(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let count = match self.handle.get().as_data() {
            data!(WithIndicators::WithIndicator { nai_bahe, .. }) => nai_bahe.len(),
            _ => unreachable!("private class fixes the indicator variant"),
        };
        indicator_word_sequence(py, &self.handle, count, |index| {
            WithIndicatorsWordSlot::NaiBahe { index }
        })
    }

    /// Return the optional NAI word attached to this indicator.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn nai(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        with_indicators_word_to_python(py, self.handle.clone(), WithIndicatorsWordSlot::Nai)
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!(
            "jbotci.syntax.IndicatorWithIndicators({:?})",
            self.handle.get()
        )
    }

    /// Return whether both wrappers select the same exact indicator tree.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn same_identity(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        Ok(self.handle.same_identity(&extract_with_indicators(other)?))
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn extract_with_indicators(value: &Bound<'_, PyAny>) -> PyResult<WithIndicatorsHandle> {
    if let Ok(value) = value.extract::<PyRef<'_, PyPlainWithIndicators>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyEmphasizedWithIndicators>>() {
        return Ok(value.handle.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyIndicatorWithIndicators>>() {
        return Ok(value.handle.clone());
    }
    Err(PyTypeError::new_err(
        "expected a jbotci.syntax WithIndicators variant",
    ))
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn with_indicators_to_python(py: Python<'_>, handle: WithIndicatorsHandle) -> PyResult<Py<PyAny>> {
    match handle.get().as_data() {
        data!(WithIndicators::Plain(_)) => {
            Ok(Py::new(py, PyPlainWithIndicators { handle })?.into_any())
        }
        data!(WithIndicators::Emphasized { .. }) => {
            Ok(Py::new(py, PyEmphasizedWithIndicators { handle })?.into_any())
        }
        data!(WithIndicators::WithIndicator { .. }) => {
            Ok(Py::new(py, PyIndicatorWithIndicators { handle })?.into_any())
        }
    }
}

#[requires(!parameter.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn extract_word_sequence(
    value: &Bound<'_, PyAny>,
    parameter: &str,
) -> PyResult<Vec<jbotci_morphology::Word>> {
    crate::support::extract_sequence(value, parameter, |word| {
        Ok(word_handle_from_python(word)?.clone_rust())
    })
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn indicator_word_required(
    py: Python<'_>,
    handle: &WithIndicatorsHandle,
    slot: WithIndicatorsWordSlot,
) -> PyResult<Py<PyAny>> {
    with_indicators_word_to_python(py, handle.clone(), slot)?.ok_or_else(|| {
        PyValueError::new_err("private indicator projection did not resolve its required word")
    })
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn indicator_word_sequence(
    py: Python<'_>,
    handle: &WithIndicatorsHandle,
    count: usize,
    slot: impl Fn(usize) -> WithIndicatorsWordSlot,
) -> PyResult<Py<PyTuple>> {
    let values = (0..count)
        .map(|index| indicator_word_required(py, handle, slot(index)))
        .collect::<PyResult<Vec<_>>>()?;
    sequence_to_tuple(py, values).map(Bound::unbind)
}

/// One non-empty source-ordered token sequence skipped during recovery.
#[invariant(matches!(value.as_data(), data!(SyntaxRecoveryItem::SkippedTokens { .. })))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct SkippedTokensValue {
    value: SyntaxRecoveryItem,
}

/// One non-empty source-ordered token sequence skipped during recovery.
#[invariant(true, "the validated backing value fixes the recovery-item variant")]
#[pyclass(
    name = "SkippedTokens",
    frozen,
    eq,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PySkippedTokens {
    value: SkippedTokensValue,
}

#[pymethods]
impl PySkippedTokens {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("error_index", "tokens");

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(error_index: usize, tokens: &Bound<'_, PyAny>) -> PyResult<Self> {
        let tokens = crate::support::extract_sequence(tokens, "tokens", |token| {
            token
                .extract::<PyRef<'_, PySyntaxToken>>()
                .map(|token| token.handle.clone_rust())
                .map_err(|_| PyTypeError::new_err("tokens must contain only syntax Token values"))
        })?;
        let tokens = Vec1::try_from_vec(tokens)
            .map_err(|_| PyValueError::new_err("tokens must contain at least one Token"))?;
        let value = try_new!(SyntaxRecoveryItem::SkippedTokens {
            error_index,
            tokens,
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
        Ok(Self {
            value: new!(SkippedTokensValue { value }),
        })
    }

    /// Return the stable recovery diagnostic index.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn error_index(&self) -> usize {
        match self.value.value.as_data() {
            data!(SyntaxRecoveryItem::SkippedTokens { error_index, .. }) => *error_index,
            _ => unreachable!("private class fixes the recovery-item variant"),
        }
    }

    /// Return the non-empty source-ordered skipped token sequence.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn tokens(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let tokens = match self.value.value.as_data() {
            data!(SyntaxRecoveryItem::SkippedTokens { tokens, .. }) => tokens,
            _ => unreachable!("private class fixes the recovery-item variant"),
        };
        let values = tokens
            .iter()
            .cloned()
            .map(|token| {
                Py::new(
                    py,
                    PySyntaxToken {
                        handle: TokenHandle::from_rust(token),
                    },
                )
            })
            .collect::<PyResult<Vec<_>>>()?;
        sequence_to_tuple(py, values).map(Bound::unbind)
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!("jbotci.syntax.SkippedTokens({:?})", self.value.value)
    }
}

/// One required syntax field inserted at an empty source position.
#[invariant(matches!(value.as_data(), data!(SyntaxRecoveryItem::MissingRequiredField { .. })))]
#[derive(Debug, Clone, PartialEq, Eq)]
struct MissingRequiredFieldValue {
    value: SyntaxRecoveryItem,
}

/// One required syntax field inserted at an empty source position.
#[invariant(true, "the validated backing value fixes the recovery-item variant")]
#[pyclass(
    name = "MissingRequiredField",
    frozen,
    eq,
    module = "jbotci.syntax",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct PyMissingRequiredField {
    value: MissingRequiredFieldValue,
}

#[pymethods]
impl PyMissingRequiredField {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str, &'static str) =
        ("error_index", "span", "expected");

    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(error_index: usize, span: PyRef<'_, PySourceSpan>, expected: &str) -> PyResult<Self> {
        if !span.rust().is_empty() {
            return Err(PyValueError::new_err(
                "span must be empty for a missing required field",
            ));
        }
        if expected.is_empty() {
            return Err(PyValueError::new_err("expected must not be empty"));
        }
        let value = try_new!(SyntaxRecoveryItem::MissingRequiredField {
            error_index,
            span: Arc::new(span.clone_rust()),
            expected: expected.to_owned(),
        })
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
        Ok(Self {
            value: new!(MissingRequiredFieldValue { value }),
        })
    }

    /// Return the stable recovery diagnostic index.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn error_index(&self) -> usize {
        match self.value.value.as_data() {
            data!(SyntaxRecoveryItem::MissingRequiredField { error_index, .. }) => *error_index,
            _ => unreachable!("private class fixes the recovery-item variant"),
        }
    }

    /// Return the empty source position at which the field was inserted.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn span(&self) -> PySourceSpan {
        match self.value.value.as_data() {
            data!(SyntaxRecoveryItem::MissingRequiredField { span, .. }) => {
                PySourceSpan::from_rust(span.as_ref().clone())
            }
            _ => unreachable!("private class fixes the recovery-item variant"),
        }
    }

    /// Return the canonical description of the required syntax field.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn expected(&self) -> &str {
        match self.value.value.as_data() {
            data!(SyntaxRecoveryItem::MissingRequiredField { expected, .. }) => expected,
            _ => unreachable!("private class fixes the recovery-item variant"),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn __repr__(&self) -> String {
        format!("jbotci.syntax.MissingRequiredField({:?})", self.value.value)
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn recovery_item_to_python(py: Python<'_>, value: SyntaxRecoveryItem) -> PyResult<Py<PyAny>> {
    match value.as_data() {
        data!(SyntaxRecoveryItem::SkippedTokens { .. }) => Ok(Py::new(
            py,
            PySkippedTokens {
                value: new!(SkippedTokensValue { value }),
            },
        )?
        .into_any()),
        data!(SyntaxRecoveryItem::MissingRequiredField { .. }) => Ok(Py::new(
            py,
            PyMissingRequiredField {
                value: new!(MissingRequiredFieldValue { value }),
            },
        )?
        .into_any()),
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn extract_recovery_item(value: &Bound<'_, PyAny>) -> PyResult<SyntaxRecoveryItem> {
    if let Ok(value) = value.extract::<PyRef<'_, PySkippedTokens>>() {
        return Ok(value.value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyMissingRequiredField>>() {
        return Ok(value.value.value.clone());
    }
    Err(PyTypeError::new_err(
        "expected a jbotci.syntax SyntaxRecoveryItem variant",
    ))
}

#[invariant(
    match &owner.root {
        SyntaxRoot::Strict { value } => value
            .node_at_path(path)
            .is_some_and(|node| strict_class_id(node) == *class_id),
        SyntaxRoot::Recovered { value } => value
            .node_at_path(path)
            .is_some_and(|node| recovered_class_id(node) == *class_id),
    },
    "every handle path resolves to its generated concrete class"
)]
#[derive(Debug, Clone)]
struct SyntaxHandle {
    owner: Arc<SyntaxOwner>,
    path: TreePath,
    class_id: usize,
}

impl SyntaxHandle {
    #[requires(true)]
    #[ensures(ret == self.class_id)]
    fn class_id(&self) -> usize {
        self.class_id
    }

    #[requires(true)]
    #[ensures(true)]
    fn class_name(&self) -> &'static str {
        match &self.owner.root {
            SyntaxRoot::Strict { .. } => SYNTAX_STRICT_CONCRETE_INVENTORY[self.class_id],
            SyntaxRoot::Recovered { .. } => SYNTAX_RECOVERED_CONCRETE_INVENTORY[self.class_id],
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn module_name(&self) -> &'static str {
        match &self.owner.root {
            SyntaxRoot::Strict { .. } => "jbotci.syntax.strict",
            SyntaxRoot::Recovered { .. } => "jbotci.syntax.recovered",
        }
    }

    #[requires(true)]
    #[ensures(ret == self.owner.projection_count())]
    fn projection_count(&self) -> usize {
        self.owner.projection_count()
    }

    #[requires(true)]
    #[ensures(ret == (Arc::ptr_eq(&self.owner, &other.owner) && self.path == other.path))]
    fn same_identity(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner, &other.owner) && self.path == other.path
    }

    #[requires(true)]
    #[ensures(true)]
    fn structural_eq(&self, other: &Self) -> bool {
        if self.class_id != other.class_id {
            return false;
        }
        match (&self.owner.root, &other.owner.root) {
            (SyntaxRoot::Strict { value: left }, SyntaxRoot::Strict { value: right }) => left
                .node_at_path(&self.path)
                .zip(right.node_at_path(&other.path))
                .is_some_and(|(left, right)| strict_nodes_equal(left, right)),
            (SyntaxRoot::Recovered { value: left }, SyntaxRoot::Recovered { value: right }) => left
                .node_at_path(&self.path)
                .zip(right.node_at_path(&other.path))
                .is_some_and(|(left, right)| recovered_nodes_equal(left, right)),
            _ => false,
        }
    }
}

#[invariant(true, "the validated handle owns and locates its generated Rust value")]
#[pyclass(
    name = "_SyntaxValue",
    frozen,
    module = "jbotci._native",
    skip_from_py_object
)]
#[derive(Debug, Clone)]
struct PySyntaxValue {
    handle: SyntaxHandle,
}

#[pymethods]
impl PySyntaxValue {
    #[requires(true)]
    #[ensures(true)]
    fn _field(&self, py: Python<'_>, index: usize) -> PyResult<Py<PyAny>> {
        project_syntax_field(py, &self.handle, index)
    }

    #[requires(true)]
    #[ensures(ret == self.handle.same_identity(&other.handle))]
    fn _same_identity(&self, other: &PySyntaxValue) -> bool {
        self.handle.same_identity(&other.handle)
    }

    #[requires(true)]
    #[ensures(ret == self.handle.structural_eq(&other.handle))]
    fn _structural_eq(&self, other: &PySyntaxValue) -> bool {
        self.handle.structural_eq(&other.handle)
    }

    #[requires(true)]
    #[ensures(ret == self.handle.projection_count())]
    fn _projection_count(&self) -> usize {
        self.handle.projection_count()
    }
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn wrap_syntax_value(py: Python<'_>, handle: SyntaxHandle) -> PyResult<Py<PyAny>> {
    let module = py.import(handle.module_name())?;
    let class = module.getattr(handle.class_name())?;
    let native = Py::new(py, PySyntaxValue { handle })?;
    class
        .call_method1("_from_native", (native,))
        .map(Bound::unbind)
}

#[requires(!class_name.is_empty() && !lens.is_empty())]
#[ensures(ret.is_ok() || ret.is_err())]
fn call_projected_syntax_wrapper(
    py: Python<'_>,
    class_name: &str,
    arguments: Vec<Py<PyAny>>,
    owner: &Arc<SyntaxOwner>,
    path: &TreePath,
    lens: Vec<usize>,
) -> PyResult<Py<PyAny>> {
    let class = py.import("jbotci.syntax")?.getattr(class_name)?;
    let arguments = sequence_to_tuple(py, arguments)?;
    let identity = Py::new(
        py,
        PySyntaxIdentity {
            value: new!(SyntaxIdentity {
                owner: Arc::clone(owner),
                path: path.clone(),
                lens,
            }),
        },
    )?;
    class
        .call_method1("_from_projected", (arguments, identity))
        .map(Bound::unbind)
}

#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
fn extract_syntax_value(value: &Bound<'_, PyAny>) -> PyResult<SyntaxHandle> {
    let native = value
        .getattr("_native")
        .map_err(|_| PyTypeError::new_err("expected a generated jbotci syntax value"))?;
    let native = native
        .extract::<PyRef<'_, PySyntaxValue>>()
        .map_err(|_| PyTypeError::new_err("expected a generated jbotci syntax value"))?;
    Ok(native.handle.clone())
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    register_type::<PySyntaxValue>(module, "_syntax_Value")?;
    register_type::<PySyntaxIdentity>(module, "_syntax_Identity")?;
    register_type::<PySyntaxToken>(module, "_syntax_Token")?;
    register_type::<PyPlainWithIndicators>(module, "_syntax_PlainWithIndicators")?;
    register_type::<PyEmphasizedWithIndicators>(module, "_syntax_EmphasizedWithIndicators")?;
    register_type::<PyIndicatorWithIndicators>(module, "_syntax_IndicatorWithIndicators")?;
    register_type::<PySkippedTokens>(module, "_syntax_SkippedTokens")?;
    register_type::<PyMissingRequiredField>(module, "_syntax_MissingRequiredField")?;
    module.add_function(wrap_pyfunction!(syntax_construct, module)?)?;
    register_private_object(
        module,
        "_syntax_STRICT_SOURCE",
        SYNTAX_STRICT_RUNTIME_SOURCE,
    )?;
    register_private_object(
        module,
        "_syntax_RECOVERED_SOURCE",
        SYNTAX_RECOVERED_RUNTIME_SOURCE,
    )?;
    register_private_object(module, "_syntax_STRICT_STUB", SYNTAX_STRICT_STUB)?;
    register_private_object(module, "_syntax_RECOVERED_STUB", SYNTAX_RECOVERED_STUB)?;
    register_private_object(
        module,
        "_syntax_STRICT_INVENTORY",
        sequence_to_tuple(module.py(), SYNTAX_STRICT_INVENTORY.iter().copied())?,
    )?;
    register_private_object(
        module,
        "_syntax_RECOVERED_INVENTORY",
        sequence_to_tuple(module.py(), SYNTAX_RECOVERED_INVENTORY.iter().copied())?,
    )?;
    register_private_object(
        module,
        "_syntax_STRICT_CONCRETE_INVENTORY",
        sequence_to_tuple(
            module.py(),
            SYNTAX_STRICT_CONCRETE_INVENTORY.iter().copied(),
        )?,
    )?;
    register_private_object(
        module,
        "_syntax_RECOVERED_CONCRETE_INVENTORY",
        sequence_to_tuple(
            module.py(),
            SYNTAX_RECOVERED_CONCRETE_INVENTORY.iter().copied(),
        )?,
    )?;
    register_private_object(
        module,
        "_syntax_SCHEMA_MODEL_COUNT",
        SYNTAX_SCHEMA_MODEL_COUNT,
    )?;
    register_private_object(
        module,
        "_syntax_SCHEMA_VARIANT_COUNT",
        SYNTAX_SCHEMA_VARIANT_COUNT,
    )?;
    register_private_object(
        module,
        "_syntax_SCHEMA_FIELD_COUNT",
        SYNTAX_SCHEMA_FIELD_COUNT,
    )?;
    debug_assert!(SYNTAX_SCHEMA_MODEL_COUNT > 0);
    debug_assert!(SYNTAX_SCHEMA_VARIANT_COUNT > 0);
    debug_assert!(SYNTAX_SCHEMA_FIELD_COUNT > 0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[requires(true)]
    #[ensures(ret.path.is_empty())]
    fn linked_sumti_factory() -> SyntaxHandle {
        let empty = jbotci_syntax::generated_model::EmptyLinkedSumtiSyntax {};
        let linked = jbotci_syntax::generated_model::LinkedSumtiSyntax::EmptyLinkedSumti(empty);
        let owner = Arc::new(SyntaxOwner {
            root: SyntaxRoot::Strict {
                value: StrictSyntaxRoot::LinkedSumtiSyntax(Arc::new(linked)),
            },
            projections: std::sync::atomic::AtomicUsize::new(0),
        });
        let node = match &owner.root {
            SyntaxRoot::Strict { value } => value
                .node_at_path(&TreePath::new())
                .expect("the factory root resolves"),
            SyntaxRoot::Recovered { .. } => unreachable!("the factory is strict"),
        };
        let class_id = strict_class_id(node);
        new!(SyntaxHandle {
            owner,
            path: TreePath::new(),
            class_id,
        })
    }

    #[requires(true)]
    #[ensures(ret.path.is_empty())]
    fn word_tanru_unit_factory() -> SyntaxHandle {
        let mut words = jbotci_morphology::segment_words_with_modifiers("melbi ui")
            .expect("the binding-internal syntax fixture is valid morphology");
        assert_eq!(words.len(), 2);
        let indicator = words
            .pop()
            .and_then(|word_like| word_like.bare_word().cloned())
            .expect("the fixture indicator is a plain morphology word");
        let base = words.pop().expect("the fixture relation word is present");
        let indicators = try_new!(WithIndicators::WithIndicator {
            base: Arc::new(WithIndicators::bare(base)),
            indicator_bahe: Vec::new(),
            indicator,
            nai_bahe: Vec::new(),
            nai: None,
        })
        .expect("the fixture indicator tree is valid");
        let value = jbotci_syntax::generated_model::WordTanruUnitSyntax(
            jbotci_syntax::tree::WithFreeModifiers::new(
                Token::from_indicators(indicators),
                Vec::new(),
            ),
        );
        let owner = Arc::new(SyntaxOwner {
            root: SyntaxRoot::Strict {
                value: StrictSyntaxRoot::WordTanruUnitSyntax(Arc::new(value)),
            },
            projections: std::sync::atomic::AtomicUsize::new(0),
        });
        let node = match &owner.root {
            SyntaxRoot::Strict { value } => value
                .node_at_path(&TreePath::new())
                .expect("the fixture root resolves"),
            SyntaxRoot::Recovered { .. } => unreachable!("the fixture is strict"),
        };
        let class_id = strict_class_id(node);
        new!(SyntaxHandle {
            owner,
            path: TreePath::new(),
            class_id,
        })
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn internal_factory_retains_owner_and_typed_tree_paths() {
        let root = linked_sumti_factory();
        assert_eq!(root.class_name(), "LinkedSumtiSyntaxEmptyLinkedSumti");

        let mut child_path = root.path.clone();
        child_path.push(jbotci_tree::TreePathStep::field(None, 0));
        let child = match &root.owner.root {
            SyntaxRoot::Strict { value } => value
                .node_at_path(&child_path)
                .expect("the generated payload path resolves"),
            SyntaxRoot::Recovered { .. } => unreachable!("the factory is strict"),
        };
        assert!(matches!(
            child,
            jbotci_syntax::generated_model::NodeRef::EmptyLinkedSumtiSyntax(_)
        ));
        assert!(root.same_identity(&root.clone()));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn token_indicator_projection_retains_exact_leaf_identity_and_lifetime() {
        let root = word_tanru_unit_factory();
        assert_eq!(root.class_name(), "WordTanruUnitSyntax");
        let token = match &root.owner.root {
            SyntaxRoot::Strict { value } => {
                let node = value
                    .node_at_path(&root.path)
                    .expect("the generated fixture root resolves");
                let jbotci_syntax::generated_model::NodeRef::WordTanruUnitSyntax(word) = node
                else {
                    panic!("the fixture root retains its word tanru unit type");
                };
                word.0.value.clone()
            }
            SyntaxRoot::Recovered { .. } => unreachable!("the fixture is strict"),
        };
        let first = TokenHandle::from_rust(token.clone()).indicators();
        let second = TokenHandle::from_rust(token).indicators();
        assert!(first.same_identity(&second));
        assert!(std::ptr::eq(first.get(), second.get()));

        drop(root);
        assert!(first.get().core_word().is_brivla());
        assert!(first.base().is_some());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn direct_with_indicators_projection_covers_resolution_identity_and_rejection() {
        let root = word_tanru_unit_factory();
        let lens = vec![
            root.class_id(),
            0,
            16, // SYNTAX_LENS_WITH_FREE_VALUE
            15, // SYNTAX_LENS_WITH_INDICATORS
        ];
        let projected = root
            .owner
            .with_indicators_at(&root.path, &lens)
            .expect("the generated fixture lens resolves");
        let first = WithIndicatorsHandle::from_projection(
            Arc::clone(&root.owner),
            root.path.clone(),
            lens.clone(),
        )
        .expect("a resolving owner lens constructs a handle");
        let second = WithIndicatorsHandle::from_projection(
            Arc::clone(&root.owner),
            root.path.clone(),
            lens.clone(),
        )
        .expect("the repeated owner lens constructs a handle");

        assert!(first.has_empty_steps());
        assert!(second.has_empty_steps());
        assert!(first.same_identity(&second));
        assert!(std::ptr::eq(first.get(), projected));
        assert!(std::ptr::eq(first.get(), second.get()));

        let non_resolving_lens = vec![
            root.class_id(),
            0,
            16, // SYNTAX_LENS_WITH_FREE_VALUE
            15, // SYNTAX_LENS_WITH_INDICATORS
            15, // an indicator tree has no nested projection at this lens
        ];
        assert!(
            root.owner
                .with_indicators_at(&root.path, &non_resolving_lens)
                .is_none()
        );
        assert!(
            WithIndicatorsHandle::from_projection(
                Arc::clone(&root.owner),
                root.path.clone(),
                non_resolving_lens,
            )
            .is_none()
        );
    }
}
