#![recursion_limit = "1024"]

//! Private native implementation for the `jbotci` Python package.

// The Python syntax owner can retain any generated grammar root. Proving the normal PyO3
// `Send + Sync` boundary therefore walks the complete recursive model and needs more trait-solver
// depth than rustc's default; the bound itself remains fully enforced.

mod diagnostics;
mod dialect;
mod dictionary;
mod jvozba;
mod morphology;
mod parser;
mod source;
mod support;
mod syntax;

use bityzba::{contract_trait, invariant, requires};
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use crate::support::{
    PythonStringEnum, register_string_enum, register_type, sequence_to_tuple, string_enum_member,
    string_repr,
};

pyo3::create_exception!(
    jbotci,
    JbotciError,
    PyException,
    "Base exception for errors reported by jbotci."
);
pyo3::create_exception!(
    jbotci,
    InvalidInputError,
    JbotciError,
    "The supplied value is not valid input for an operation."
);

const SMOKE_MESSAGE: &str = "jbotci native bindings ready";
const ROOT_NATIVE_EXPORTS: &[&str] = &[
    "__version__",
    "smoke",
    "raise_sample_error",
    "sample_mode",
    "JbotciError",
    "InvalidInputError",
    "_root_Sample",
    "_root_SampleMode",
];
const NATIVE_EXPORT_GROUPS: &[&[&str]] = &[
    ROOT_NATIVE_EXPORTS,
    dictionary::NATIVE_EXPORTS,
    jvozba::NATIVE_EXPORTS,
    source::NATIVE_EXPORTS,
    diagnostics::NATIVE_EXPORTS,
    dialect::NATIVE_EXPORTS,
    morphology::NATIVE_EXPORTS,
    syntax::NATIVE_EXPORTS,
    parser::NATIVE_EXPORTS,
];

/// Structured errors produced inside the binding layer.
#[invariant(::InvalidInput { .. } => true, "all error messages are representable")]
#[derive(Debug, Clone, PartialEq, Eq)]
enum BindingError {
    InvalidInput { message: String },
}

impl BindingError {
    /// Convert a structured binding error to its stable Python exception class.
    #[requires(true)]
    #[ensures(true)]
    fn into_py_err(self) -> PyErr {
        match self {
            Self::InvalidInput { message } => InvalidInputError::new_err(message),
        }
    }
}

/// Temporary fieldless enum used to exercise the shared string-enum path.
///
/// The declaration order intentionally differs from the public member order;
/// neither that order nor the implicit Rust discriminants cross into Python.
#[invariant(::Advanced => true, "unit variant carries no invalid state")]
#[invariant(::Basic => true, "unit variant carries no invalid state")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SampleMode {
    Advanced,
    Basic,
}

#[contract_trait]
impl PythonStringEnum for SampleMode {
    fn native_export_name() -> &'static str {
        "_root_SampleMode"
    }

    fn python_type_name() -> &'static str {
        "SampleMode"
    }

    fn python_module_name() -> &'static str {
        "jbotci"
    }

    fn python_doc() -> &'static str {
        "Temporary fieldless enum used to test stable string registration."
    }

    fn variants() -> &'static [Self] {
        const VARIANTS: &[SampleMode] = &[SampleMode::Basic, SampleMode::Advanced];
        VARIANTS
    }

    fn python_member_name(self) -> std::borrow::Cow<'static, str> {
        match self {
            Self::Basic => std::borrow::Cow::Borrowed("BASIC"),
            Self::Advanced => std::borrow::Cow::Borrowed("ADVANCED"),
        }
    }

    fn python_value(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Advanced => "advanced",
        }
    }
}

/// Temporary value object used to exercise class binding conventions.
#[invariant(true)]
#[pyclass(
    name = "Sample",
    frozen,
    eq,
    hash,
    module = "jbotci",
    skip_from_py_object
)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PySample {
    value: String,
}

#[pymethods]
impl PySample {
    #[requires(true)]
    #[ensures(ret.value.as_str() == value)]
    #[new]
    fn new(value: &str) -> Self {
        Self {
            value: value.to_owned(),
        }
    }

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
            "jbotci.Sample(value={})",
            string_repr(py, &self.value)?
        ))
    }
}

/// Confirm that the native extension loaded and can execute Rust code.
#[requires(true)]
#[ensures(ret == SMOKE_MESSAGE)]
#[pyfunction]
fn smoke() -> &'static str {
    SMOKE_MESSAGE
}

/// Raise a sample structured error through the shared conversion path.
#[requires(true)]
#[ensures(ret.is_err())]
#[pyfunction]
fn raise_sample_error(message: &str) -> PyResult<()> {
    Err(BindingError::InvalidInput {
        message: message.to_owned(),
    }
    .into_py_err())
}

/// Return a sample enum through the stable string conversion path.
#[requires(true)]
#[ensures(true)]
#[pyfunction]
#[pyo3(signature = (advanced = false))]
fn sample_mode(py: Python<'_>, advanced: bool) -> PyResult<Py<PyAny>> {
    let module = py.import("jbotci._native")?;
    let value = if advanced {
        SampleMode::Advanced
    } else {
        SampleMode::Basic
    };
    string_enum_member(&module, value).map(Bound::unbind)
}

#[requires(true)]
#[ensures(true)]
fn register_exceptions(module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = module.py();
    module.add("JbotciError", py.get_type::<JbotciError>())?;
    module.add("InvalidInputError", py.get_type::<InvalidInputError>())?;
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn register_root(module: &Bound<'_, PyModule>) -> PyResult<()> {
    register_exceptions(module)?;
    register_type::<PySample>(module, "_root_Sample")?;
    register_string_enum::<SampleMode>(module)?;
    module.add_function(wrap_pyfunction!(smoke, module)?)?;
    module.add_function(wrap_pyfunction!(raise_sample_error, module)?)?;
    module.add_function(wrap_pyfunction!(sample_mode, module)?)?;
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn register_metadata(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    let exports = NATIVE_EXPORT_GROUPS
        .iter()
        .flat_map(|group| group.iter().copied())
        .collect::<Vec<_>>();
    module.add("__all__", sequence_to_tuple(module.py(), exports)?)?;
    Ok(())
}

/// Initialize the private `jbotci._native` extension module.
#[requires(true)]
#[ensures(true)]
#[pymodule]
#[pyo3(name = "_native")]
fn native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    register_root(module)?;
    dictionary::register(module)?;
    jvozba::register(module)?;
    source::register(module)?;
    diagnostics::register(module)?;
    dialect::register(module)?;
    morphology::register(module)?;
    syntax::register(module)?;
    parser::register(module)?;
    register_metadata(module)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn sample_is_a_value_object() {
        Python::initialize();
        let sample = PySample::new("coi");
        assert_eq!(sample.value(), "coi");
        Python::attach(|py| {
            assert_eq!(sample.__repr__(py).unwrap(), "jbotci.Sample(value='coi')");
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn smoke_message_is_stable() {
        assert_eq!(smoke(), SMOKE_MESSAGE);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn string_enum_uses_explicit_names_and_values() {
        assert_eq!(
            SampleMode::variants(),
            &[SampleMode::Basic, SampleMode::Advanced]
        );
        assert_eq!(SampleMode::Basic.python_member_name(), "BASIC");
        assert_eq!(SampleMode::Advanced.python_value(), "advanced");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn native_export_inventory_has_no_collisions() {
        let mut exports = NATIVE_EXPORT_GROUPS
            .iter()
            .flat_map(|group| group.iter().copied())
            .collect::<Vec<_>>();
        let original_len = exports.len();
        exports.sort_unstable();
        exports.dedup();
        assert_eq!(exports.len(), original_len);
    }
}
