//! Private native implementation for the `jbotci` Python package.

use std::sync::Arc;

use bityzba::{invariant, requires};
use pyo3::PyClass;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyTuple};

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
const PUBLIC_EXPORTS: &[&str] = &[
    "__version__",
    "smoke",
    "raise_sample_error",
    "JbotciError",
    "InvalidInputError",
    "Sample",
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

/// Retains an owner while projecting a stable reference into it.
///
/// Python wrappers can own this helper instead of storing or erasing a Rust
/// lifetime. The projection is rerun for every borrow, so the value never
/// becomes a self-reference.
#[invariant(true)]
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "shared convention for upcoming borrowed-data wrappers"
)]
struct OwnedReference<Owner, Target: ?Sized> {
    owner: Arc<Owner>,
    project: for<'a> fn(&'a Owner) -> &'a Target,
}

impl<Owner, Target: ?Sized> OwnedReference<Owner, Target> {
    #[requires(true)]
    #[ensures(true)]
    #[allow(
        dead_code,
        reason = "shared convention for upcoming borrowed-data wrappers"
    )]
    fn new(owner: Arc<Owner>, project: for<'a> fn(&'a Owner) -> &'a Target) -> Self {
        Self { owner, project }
    }

    #[requires(true)]
    #[ensures(true)]
    #[allow(
        dead_code,
        reason = "shared convention for upcoming borrowed-data wrappers"
    )]
    fn get(&self) -> &Target {
        (self.project)(&self.owner)
    }
}

/// Temporary value object used to exercise class binding conventions.
#[invariant(true)]
#[pyclass(frozen, eq, hash, module = "jbotci", skip_from_py_object)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Sample {
    value: String,
}

#[pymethods]
impl Sample {
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
    #[ensures(ret.starts_with("jbotci.Sample(value="))]
    fn __repr__(&self) -> String {
        format!("jbotci.Sample(value={:?})", self.value)
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

/// Convert a Rust sequence into the package's immutable Python representation.
#[requires(true)]
#[ensures(true)]
fn sequence_to_tuple<'py, T, I>(py: Python<'py>, values: I) -> PyResult<Bound<'py, PyTuple>>
where
    T: IntoPyObject<'py>,
    I: IntoIterator<Item = T>,
    I::IntoIter: ExactSizeIterator,
{
    PyTuple::new(py, values)
}

/// Register a Python class through the common type-registration path.
#[requires(true)]
#[ensures(true)]
fn register_type<T: PyClass>(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<T>()
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
fn register_functions(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(smoke, module)?)?;
    module.add_function(wrap_pyfunction!(raise_sample_error, module)?)?;
    Ok(())
}

#[requires(true)]
#[ensures(true)]
fn register_metadata(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    module.add(
        "__all__",
        sequence_to_tuple(module.py(), PUBLIC_EXPORTS.iter().copied())?,
    )?;
    Ok(())
}

/// Initialize the private `jbotci._native` extension module.
#[requires(true)]
#[ensures(true)]
#[pymodule]
#[pyo3(name = "_native")]
fn native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    register_exceptions(module)?;
    register_functions(module)?;
    register_type::<Sample>(module)?;
    register_metadata(module)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[requires(true)]
    #[ensures(true)]
    fn project_text(owner: &String) -> &str {
        owner.as_str()
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn sample_is_a_value_object() {
        let sample = Sample::new("coi");
        assert_eq!(sample.value(), "coi");
        assert_eq!(sample.__repr__(), "jbotci.Sample(value=\"coi\")");
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn owned_reference_keeps_its_owner_alive() {
        let owner = Arc::new(String::from("coi"));
        let reference = OwnedReference::new(Arc::clone(&owner), project_text);
        drop(owner);
        assert_eq!(reference.get(), "coi");
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn smoke_message_is_stable() {
        assert_eq!(smoke(), SMOKE_MESSAGE);
    }

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn sequence_helper_builds_a_tuple() {
        Python::initialize();
        Python::attach(|py| {
            let tuple = sequence_to_tuple(py, [1_u8, 2, 3]).unwrap();
            assert_eq!(tuple.len(), 3);
        });
    }
}
