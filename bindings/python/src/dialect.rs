//! Declarative dialect formulas and settings exposed independently of morphology compilation.

use std::borrow::Cow;
use std::collections::BTreeSet;

use bityzba::{contract_trait, ensures, expensive_ensures, invariant, new, requires};
use jbotci_dialect::{
    BuiltinDialect, CmavoDialectEntry, CustomDialect, DialectDefinition, DialectFeature,
    DialectSettings, add_dialect_formula_reference as rust_add_dialect_formula_reference,
    builtin_dialect_names as rust_builtin_dialect_names, builtin_dialects as rust_builtin_dialects,
    cmavo_dialect_entries_to_definition as rust_cmavo_dialect_entries_to_definition,
    custom_dialect_definition_to_johau_uri as rust_custom_dialect_definition_to_johau_uri,
    custom_dialect_definition_to_johau_uri_with_custom_dialects as rust_custom_dialect_definition_to_johau_uri_with_custom_dialects,
    custom_dialect_is_valid as rust_custom_dialect_is_valid,
    dialect_definition_to_johau_uri as rust_dialect_definition_to_johau_uri,
    dialect_definition_to_text as rust_dialect_definition_to_text,
    dialect_formula_top_level_references as rust_dialect_formula_top_level_references,
    dialect_name_shows_in_gentufa_picker as rust_dialect_name_shows_in_gentufa_picker,
    find_builtin_dialect as rust_find_builtin_dialect,
    import_johau_dialect_settings as rust_import_johau_dialect_settings,
    parse_dialect_definition as rust_parse_dialect_definition,
    parse_dialect_definition_with_custom_dialects as rust_parse_dialect_definition_with_custom_dialects,
    parse_dialect_selection_formula as rust_parse_dialect_selection_formula,
    parse_johau_dialect_uri as rust_parse_johau_dialect_uri,
    remove_dialect_formula_reference as rust_remove_dialect_formula_reference,
    replace_dialect_formula_reference as rust_replace_dialect_formula_reference,
};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyTuple};

use crate::InvalidInputError;
use crate::support::{
    PythonStringEnum, extract_sequence, extract_string_enum, register_private_object,
    register_string_enum, register_type, sequence_to_tuple, string_enum_member, string_repr,
};

const PUBLIC_MODULE: &str = "jbotci.dialect";

pub(crate) const NATIVE_EXPORTS: &[&str] = &[
    "_dialect_DialectFeature",
    "_dialect_dialect_feature_atom_name",
    "_dialect_CmavoSwap",
    "_dialect_CmavoExpansion",
    "_dialect_DialectDefinition",
    "_dialect_BuiltinDialect",
    "_dialect_CustomDialect",
    "_dialect_DialectSettings",
    "_dialect_parse_dialect_definition",
    "_dialect_builtin_dialects",
    "_dialect_builtin_dialect_names",
    "_dialect_find_builtin_dialect",
    "_dialect_parse_dialect_definition_with_custom_dialects",
    "_dialect_parse_dialect_selection_formula",
    "_dialect_custom_dialect_is_valid",
    "_dialect_dialect_name_shows_in_gentufa_picker",
    "_dialect_dialect_formula_top_level_references",
    "_dialect_add_dialect_formula_reference",
    "_dialect_remove_dialect_formula_reference",
    "_dialect_replace_dialect_formula_reference",
    "_dialect_dialect_definition_to_text",
    "_dialect_cmavo_dialect_entries_to_definition",
    "_dialect_custom_dialect_definition_to_johau_uri",
    "_dialect_custom_dialect_definition_to_johau_uri_with_custom_dialects",
    "_dialect_dialect_definition_to_johau_uri",
    "_dialect_parse_johau_dialect_uri",
    "_dialect_import_johau_dialect_settings",
];

#[contract_trait]
impl PythonStringEnum for DialectFeature {
    fn native_export_name() -> &'static str {
        "_dialect_DialectFeature"
    }

    fn python_type_name() -> &'static str {
        "DialectFeature"
    }

    fn python_module_name() -> &'static str {
        PUBLIC_MODULE
    }

    fn python_doc() -> &'static str {
        "Declarative jbotci dialect feature."
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

#[requires(true)]
#[ensures(true)]
fn native_module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    py.import("jbotci._native")
}

#[requires(true)]
#[ensures(true)]
fn feature_from_python(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<DialectFeature> {
    extract_string_enum(&native_module(py)?, value)
}

#[requires(true)]
#[ensures(true)]
fn feature_to_python(py: Python<'_>, value: DialectFeature) -> PyResult<Py<PyAny>> {
    string_enum_member(&native_module(py)?, value).map(Bound::unbind)
}

/// Return the uppercase atom used for a declarative dialect feature.
#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|atom| !atom.is_empty()) || ret.is_err())]
#[pyfunction]
fn dialect_feature_atom_name(py: Python<'_>, feature: &Bound<'_, PyAny>) -> PyResult<String> {
    Ok(feature_from_python(py, feature)?.atom_name())
}

/// Declarative dialect entry that swaps two cmavo forms.
#[invariant(matches!(value.as_data(), bityzba::data!(CmavoDialectEntry::Swap { .. })))]
#[pyclass(
    name = "CmavoSwap",
    frozen,
    eq,
    module = "jbotci.dialect",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyCmavoSwap {
    value: CmavoDialectEntry,
}

#[pymethods]
impl PyCmavoSwap {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("left", "right");

    /// Construct a validated cmavo swap.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(left: String, right: String) -> PyResult<Self> {
        CmavoDialectEntry::swap(left, right)
            .map(|value| new!(PyCmavoSwap { value }))
            .map_err(invalid_input)
    }

    /// Return the first cmavo form in the swap.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn left(&self) -> &str {
        let bityzba::data!(CmavoDialectEntry::Swap { left, .. }) = self.value.as_data() else {
            unreachable!("wrapper invariant fixes the dialect entry variant")
        };
        left
    }

    /// Return the second cmavo form in the swap.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn right(&self) -> &str {
        let bityzba::data!(CmavoDialectEntry::Swap { right, .. }) = self.value.as_data() else {
            unreachable!("wrapper invariant fixes the dialect entry variant")
        };
        right
    }
}

/// Declarative dialect entry that expands one cmavo into a non-empty sequence.
#[invariant(matches!(value.as_data(), bityzba::data!(CmavoDialectEntry::Expansion { replacement, .. }) if !replacement.is_empty()))]
#[pyclass(
    name = "CmavoExpansion",
    frozen,
    eq,
    module = "jbotci.dialect",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyCmavoExpansion {
    value: CmavoDialectEntry,
}

#[pymethods]
impl PyCmavoExpansion {
    #[classattr]
    #[allow(non_upper_case_globals)]
    const __match_args__: (&'static str, &'static str) = ("source", "replacement");

    /// Construct a validated cmavo expansion.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    fn new(source: String, replacement: Vec<String>) -> PyResult<Self> {
        CmavoDialectEntry::expansion(source, replacement)
            .map(|value| new!(PyCmavoExpansion { value }))
            .map_err(invalid_input)
    }

    /// Return the source cmavo form.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    #[getter]
    fn source(&self) -> &str {
        let bityzba::data!(CmavoDialectEntry::Expansion { source, .. }) = self.value.as_data()
        else {
            unreachable!("wrapper invariant fixes the dialect entry variant")
        };
        source
    }

    /// Return the immutable replacement sequence.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn replacement(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let bityzba::data!(CmavoDialectEntry::Expansion { replacement, .. }) = self.value.as_data()
        else {
            unreachable!("wrapper invariant fixes the dialect entry variant")
        };
        sequence_to_tuple(py, replacement.iter().cloned()).map(Bound::unbind)
    }
}

#[requires(true)]
#[ensures(true)]
fn entry_from_python(value: &Bound<'_, PyAny>) -> PyResult<CmavoDialectEntry> {
    if let Ok(value) = value.extract::<PyRef<'_, PyCmavoSwap>>() {
        return Ok(value.value.clone());
    }
    if let Ok(value) = value.extract::<PyRef<'_, PyCmavoExpansion>>() {
        return Ok(value.value.clone());
    }
    Err(PyTypeError::new_err(
        "expected jbotci.dialect.CmavoSwap or CmavoExpansion",
    ))
}

#[requires(true)]
#[ensures(true)]
fn entry_to_python(py: Python<'_>, value: CmavoDialectEntry) -> PyResult<Py<PyAny>> {
    let value = match value.as_data() {
        bityzba::data!(CmavoDialectEntry::Swap { .. }) => {
            Py::new(py, new!(PyCmavoSwap { value }))?.into_any()
        }
        bityzba::data!(CmavoDialectEntry::Expansion { .. }) => {
            Py::new(py, new!(PyCmavoExpansion { value }))?.into_any()
        }
    };
    Ok(value)
}

/// Declarative dialect definition containing cmavo rewrites and feature flags.
#[invariant(true, "entry validity is carried by CmavoDialectEntry")]
#[pyclass(
    name = "DialectDefinition",
    frozen,
    eq,
    module = "jbotci.dialect",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyDialectDefinition {
    value: DialectDefinition,
}

impl PyDialectDefinition {
    #[requires(true)]
    #[expensive_ensures(ret.value == old(value.clone()))]
    pub(crate) fn from_rust(value: DialectDefinition) -> Self {
        Self { value }
    }

    #[requires(true)]
    #[ensures(ret == &self.value)]
    pub(crate) fn rust(&self) -> &DialectDefinition {
        &self.value
    }
}

#[pymethods]
impl PyDialectDefinition {
    /// Construct a dialect definition from ordered entry and feature sequences.
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    #[new]
    #[pyo3(signature = (cmavo_entries=None, features=None))]
    fn new(
        py: Python<'_>,
        cmavo_entries: Option<&Bound<'_, PyAny>>,
        features: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let cmavo_entries = cmavo_entries
            .map(|entries| extract_sequence(entries, "cmavo_entries", entry_from_python))
            .transpose()?
            .unwrap_or_default();
        let features = features
            .map(|features| {
                extract_sequence(features, "features", |feature| {
                    feature_from_python(py, feature)
                })
                .map(|features| features.into_iter().collect::<BTreeSet<_>>())
            })
            .transpose()?
            .unwrap_or_default();
        Ok(Self::from_rust(DialectDefinition::new(
            cmavo_entries,
            features,
        )))
    }

    /// Construct the baseline dialect definition.
    #[staticmethod]
    #[requires(true)]
    #[ensures(ret.value.is_baseline())]
    fn baseline() -> Self {
        Self::from_rust(DialectDefinition::baseline())
    }

    /// Return the declarative cmavo entries.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn cmavo_entries(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let values = self
            .value
            .cmavo_entries
            .iter()
            .cloned()
            .map(|entry| entry_to_python(py, entry))
            .collect::<PyResult<Vec<_>>>()?;
        sequence_to_tuple(py, values).map(Bound::unbind)
    }

    /// Return the enabled dialect features.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn features(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        let values = self
            .value
            .features
            .iter()
            .copied()
            .map(|feature| feature_to_python(py, feature))
            .collect::<PyResult<Vec<_>>>()?;
        sequence_to_tuple(py, values).map(Bound::unbind)
    }

    /// Report whether this is the baseline dialect.
    #[requires(true)]
    #[ensures(ret == self.value.is_baseline())]
    fn is_baseline(&self) -> bool {
        self.value.is_baseline()
    }
}

/// Built-in named dialect and its declarative definition.
#[invariant(!name.is_empty())]
#[invariant(!definition.is_empty())]
#[pyclass(
    name = "BuiltinDialect",
    frozen,
    eq,
    module = "jbotci.dialect",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyBuiltinDialect {
    name: String,
    definition: String,
    dialect: DialectDefinition,
}

impl PyBuiltinDialect {
    #[requires(!value.name.is_empty())]
    #[requires(!value.definition.is_empty())]
    #[ensures(ret.name == value.name)]
    fn from_rust(value: &BuiltinDialect) -> Self {
        new!(PyBuiltinDialect {
            name: value.name.to_owned(),
            definition: value.definition.to_owned(),
            dialect: value.dialect.clone(),
        })
    }
}

#[pymethods]
impl PyBuiltinDialect {
    /// Return the stable built-in dialect name.
    #[requires(true)]
    #[ensures(ret == self.name.as_str())]
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    /// Return the textual dialect formula.
    #[requires(true)]
    #[ensures(ret == self.definition.as_str())]
    #[getter]
    fn definition(&self) -> &str {
        &self.definition
    }

    /// Return the parsed declarative dialect definition.
    #[requires(true)]
    #[ensures(ret.value == self.dialect)]
    #[getter]
    fn dialect(&self) -> PyDialectDefinition {
        PyDialectDefinition::from_rust(self.dialect.clone())
    }
}

/// User-defined named dialect formula.
#[invariant(true)]
#[pyclass(
    name = "CustomDialect",
    frozen,
    eq,
    module = "jbotci.dialect",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyCustomDialect {
    value: CustomDialect,
}

impl PyCustomDialect {
    #[requires(true)]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from_rust(value: CustomDialect) -> Self {
        Self { value }
    }
}

#[pymethods]
impl PyCustomDialect {
    /// Construct a custom dialect descriptor.
    #[requires(true)]
    #[ensures(ret.value.name.is_empty() == old(name.is_empty()))]
    #[ensures(ret.value.definition.is_empty() == old(definition.is_empty()))]
    #[ensures(ret.value.show_in_gentufa == show_in_gentufa)]
    #[expensive_ensures((ret.value.name.clone(), ret.value.definition.clone()) == old((name.clone(), definition.clone())))]
    #[new]
    #[pyo3(signature = (name, definition, *, show_in_gentufa=true))]
    fn new(name: String, definition: String, show_in_gentufa: bool) -> Self {
        Self::from_rust(CustomDialect {
            name,
            definition,
            show_in_gentufa,
        })
    }

    /// Return the custom dialect name.
    #[requires(true)]
    #[ensures(ret == self.value.name.as_str())]
    #[getter]
    fn name(&self) -> &str {
        &self.value.name
    }

    /// Return the custom dialect formula text.
    #[requires(true)]
    #[ensures(ret == self.value.definition.as_str())]
    #[getter]
    fn definition(&self) -> &str {
        &self.value.definition
    }

    /// Report whether the dialect is shown in the Gentufa picker.
    #[requires(true)]
    #[ensures(ret == self.value.show_in_gentufa)]
    #[getter]
    fn show_in_gentufa(&self) -> bool {
        self.value.show_in_gentufa
    }
}

/// Saved custom dialects and hidden built-in-picker entries.
#[invariant(true)]
#[pyclass(
    name = "DialectSettings",
    frozen,
    eq,
    module = "jbotci.dialect",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PyDialectSettings {
    value: DialectSettings,
}

impl PyDialectSettings {
    #[requires(true)]
    #[expensive_ensures(ret.value == old(value.clone()))]
    fn from_rust(value: DialectSettings) -> Self {
        Self { value }
    }
}

#[pymethods]
impl PyDialectSettings {
    /// Construct dialect settings from custom and hidden dialect collections.
    #[requires(true)]
    #[ensures(ret.value.custom_dialects.len() == custom_dialects.len())]
    #[new]
    #[pyo3(signature = (custom_dialects=Vec::new(), hidden_builtin_gentufa_dialects=Vec::new()))]
    fn new(
        custom_dialects: Vec<PyRef<'_, PyCustomDialect>>,
        hidden_builtin_gentufa_dialects: Vec<String>,
    ) -> Self {
        Self::from_rust(DialectSettings {
            custom_dialects: custom_dialects
                .iter()
                .map(|dialect| dialect.value.clone())
                .collect(),
            hidden_builtin_gentufa_dialects: hidden_builtin_gentufa_dialects.into_iter().collect(),
        })
    }

    /// Return the immutable custom dialect collection.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn custom_dialects(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            self.value
                .custom_dialects
                .iter()
                .cloned()
                .map(PyCustomDialect::from_rust),
        )
        .map(Bound::unbind)
    }

    /// Return the immutable hidden built-in dialect names.
    #[requires(true)]
    #[ensures(true)]
    #[getter]
    fn hidden_builtin_gentufa_dialects(&self, py: Python<'_>) -> PyResult<Py<PyTuple>> {
        sequence_to_tuple(
            py,
            self.value.hidden_builtin_gentufa_dialects.iter().cloned(),
        )
        .map(Bound::unbind)
    }
}

#[requires(true)]
#[ensures(ret.len() == values.len())]
fn custom_dialects(values: &[PyRef<'_, PyCustomDialect>]) -> Vec<CustomDialect> {
    values.iter().map(|value| value.value.clone()).collect()
}

/// Parse a standalone dialect formula into its declarative definition.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
fn parse_dialect_definition(source: &str) -> PyResult<PyDialectDefinition> {
    rust_parse_dialect_definition(source)
        .map(PyDialectDefinition::from_rust)
        .map_err(invalid_input)
}

/// Return every built-in named dialect.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
fn builtin_dialects(py: Python<'_>) -> PyResult<Py<PyTuple>> {
    sequence_to_tuple(
        py,
        rust_builtin_dialects()
            .iter()
            .map(PyBuiltinDialect::from_rust),
    )
    .map(Bound::unbind)
}

/// Return the stable names of all built-in dialects.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
fn builtin_dialect_names(py: Python<'_>) -> PyResult<Py<PyTuple>> {
    sequence_to_tuple(py, rust_builtin_dialect_names()).map(Bound::unbind)
}

/// Find a built-in dialect by its stable name.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
fn find_builtin_dialect(name: &str) -> Option<PyBuiltinDialect> {
    rust_find_builtin_dialect(name).map(PyBuiltinDialect::from_rust)
}

/// Parse a dialect formula with additional named custom dialects in scope.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
fn parse_dialect_definition_with_custom_dialects(
    custom: Vec<PyRef<'_, PyCustomDialect>>,
    source: &str,
) -> PyResult<PyDialectDefinition> {
    rust_parse_dialect_definition_with_custom_dialects(&custom_dialects(&custom), source)
        .map(PyDialectDefinition::from_rust)
        .map_err(invalid_input)
}

/// Parse a saved dialect-selection formula using the supplied settings.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
fn parse_dialect_selection_formula(
    settings: PyRef<'_, PyDialectSettings>,
    source: &str,
) -> PyResult<PyDialectDefinition> {
    rust_parse_dialect_selection_formula(&settings.value, source)
        .map(PyDialectDefinition::from_rust)
        .map_err(invalid_input)
}

/// Validate a custom dialect against the existing custom dialect collection.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
fn custom_dialect_is_valid(
    existing: Vec<PyRef<'_, PyCustomDialect>>,
    custom: PyRef<'_, PyCustomDialect>,
) -> PyResult<()> {
    rust_custom_dialect_is_valid(&custom_dialects(&existing), &custom.value).map_err(invalid_input)
}

/// Report whether a dialect name is eligible for the Gentufa picker.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
fn dialect_name_shows_in_gentufa_picker(name: &str) -> bool {
    rust_dialect_name_shows_in_gentufa_picker(name)
}

/// Return the top-level named dialect references in a formula.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
fn dialect_formula_top_level_references(
    py: Python<'_>,
    formula_text: &str,
) -> PyResult<Py<PyTuple>> {
    sequence_to_tuple(py, rust_dialect_formula_top_level_references(formula_text))
        .map(Bound::unbind)
}

/// Add a named dialect reference to a formula.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
fn add_dialect_formula_reference(dialect_name: &str, formula_text: &str) -> String {
    rust_add_dialect_formula_reference(dialect_name, formula_text)
}

/// Remove a named dialect reference from a formula.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
fn remove_dialect_formula_reference(dialect_name: &str, formula_text: &str) -> String {
    rust_remove_dialect_formula_reference(dialect_name, formula_text)
}

/// Replace one named dialect reference with another in a formula.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
fn replace_dialect_formula_reference(
    previous_name: &str,
    next_name: &str,
    formula_text: &str,
) -> String {
    rust_replace_dialect_formula_reference(previous_name, next_name, formula_text)
}

/// Render a declarative dialect definition as formula text.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
fn dialect_definition_to_text(definition: PyRef<'_, PyDialectDefinition>) -> String {
    rust_dialect_definition_to_text(&definition.value)
}

/// Render an ordered sequence of declarative cmavo entries as a dialect formula.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
fn cmavo_dialect_entries_to_definition(entries: &Bound<'_, PyAny>) -> PyResult<String> {
    let entries = extract_sequence(entries, "entries", entry_from_python)?;
    Ok(rust_cmavo_dialect_entries_to_definition(&entries))
}

/// Encode a custom dialect formula as a `johau` URI.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
fn custom_dialect_definition_to_johau_uri(definition: &str) -> PyResult<String> {
    rust_custom_dialect_definition_to_johau_uri(definition).map_err(invalid_input)
}

/// Encode a formula as a `johau` URI with custom dialects in scope.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
fn custom_dialect_definition_to_johau_uri_with_custom_dialects(
    custom: Vec<PyRef<'_, PyCustomDialect>>,
    definition: &str,
) -> PyResult<String> {
    rust_custom_dialect_definition_to_johau_uri_with_custom_dialects(
        &custom_dialects(&custom),
        definition,
    )
    .map_err(invalid_input)
}

/// Encode a parsed dialect definition as a `johau` URI.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
fn dialect_definition_to_johau_uri(definition: PyRef<'_, PyDialectDefinition>) -> PyResult<String> {
    rust_dialect_definition_to_johau_uri(&definition.value).map_err(invalid_input)
}

/// Decode a `johau` URI into a declarative dialect definition.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
fn parse_johau_dialect_uri(raw_uri: &str) -> PyResult<PyDialectDefinition> {
    rust_parse_johau_dialect_uri(raw_uri)
        .map(PyDialectDefinition::from_rust)
        .map_err(invalid_input)
}

/// Import a `johau` URI into saved dialect settings.
#[requires(true)]
#[ensures(ret.is_ok() || ret.is_err())]
#[pyfunction]
fn import_johau_dialect_settings(
    raw_uri: &str,
    settings: PyRef<'_, PyDialectSettings>,
) -> PyResult<(String, PyDialectSettings)> {
    rust_import_johau_dialect_settings(raw_uri, &settings.value)
        .map(|(name, settings)| (name, PyDialectSettings::from_rust(settings)))
        .map_err(invalid_input)
}

#[requires(true)]
#[ensures(true)]
fn invalid_input(error: impl std::fmt::Display) -> PyErr {
    InvalidInputError::new_err(error.to_string())
}

macro_rules! register_function {
    ($module:expr, $name:literal, $function:ident) => {
        register_private_object($module, $name, wrap_pyfunction!($function, $module)?)?;
    };
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    register_string_enum::<DialectFeature>(module)?;
    register_function!(
        module,
        "_dialect_dialect_feature_atom_name",
        dialect_feature_atom_name
    );
    register_type::<PyCmavoSwap>(module, "_dialect_CmavoSwap")?;
    register_type::<PyCmavoExpansion>(module, "_dialect_CmavoExpansion")?;
    register_type::<PyDialectDefinition>(module, "_dialect_DialectDefinition")?;
    register_type::<PyBuiltinDialect>(module, "_dialect_BuiltinDialect")?;
    register_type::<PyCustomDialect>(module, "_dialect_CustomDialect")?;
    register_type::<PyDialectSettings>(module, "_dialect_DialectSettings")?;
    register_function!(
        module,
        "_dialect_parse_dialect_definition",
        parse_dialect_definition
    );
    register_function!(module, "_dialect_builtin_dialects", builtin_dialects);
    register_function!(
        module,
        "_dialect_builtin_dialect_names",
        builtin_dialect_names
    );
    register_function!(
        module,
        "_dialect_find_builtin_dialect",
        find_builtin_dialect
    );
    register_function!(
        module,
        "_dialect_parse_dialect_definition_with_custom_dialects",
        parse_dialect_definition_with_custom_dialects
    );
    register_function!(
        module,
        "_dialect_parse_dialect_selection_formula",
        parse_dialect_selection_formula
    );
    register_function!(
        module,
        "_dialect_custom_dialect_is_valid",
        custom_dialect_is_valid
    );
    register_function!(
        module,
        "_dialect_dialect_name_shows_in_gentufa_picker",
        dialect_name_shows_in_gentufa_picker
    );
    register_function!(
        module,
        "_dialect_dialect_formula_top_level_references",
        dialect_formula_top_level_references
    );
    register_function!(
        module,
        "_dialect_add_dialect_formula_reference",
        add_dialect_formula_reference
    );
    register_function!(
        module,
        "_dialect_remove_dialect_formula_reference",
        remove_dialect_formula_reference
    );
    register_function!(
        module,
        "_dialect_replace_dialect_formula_reference",
        replace_dialect_formula_reference
    );
    register_function!(
        module,
        "_dialect_dialect_definition_to_text",
        dialect_definition_to_text
    );
    register_function!(
        module,
        "_dialect_cmavo_dialect_entries_to_definition",
        cmavo_dialect_entries_to_definition
    );
    register_function!(
        module,
        "_dialect_custom_dialect_definition_to_johau_uri",
        custom_dialect_definition_to_johau_uri
    );
    register_function!(
        module,
        "_dialect_custom_dialect_definition_to_johau_uri_with_custom_dialects",
        custom_dialect_definition_to_johau_uri_with_custom_dialects
    );
    register_function!(
        module,
        "_dialect_dialect_definition_to_johau_uri",
        dialect_definition_to_johau_uri
    );
    register_function!(
        module,
        "_dialect_parse_johau_dialect_uri",
        parse_johau_dialect_uri
    );
    register_function!(
        module,
        "_dialect_import_johau_dialect_settings",
        import_johau_dialect_settings
    );
    Ok(())
}
