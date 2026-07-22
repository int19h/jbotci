//! Private native implementation for the `jbotci` Python package.

use std::sync::Arc;

use bityzba::{contract_trait, invariant, requires};
use pyo3::PyClass;
use pyo3::exceptions::{PyException, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule, PyString, PyTuple};

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
    "SampleMode",
    "sample_mode",
];

/// Structured errors produced inside the binding layer.
#[invariant(::InvalidInput { .. } => true, "all error messages are representable")]
#[derive(Debug, Clone, PartialEq, Eq)]
enum BindingError {
    InvalidInput { message: String },
}

/// Supplies stable Python metadata for a fieldless Rust enum.
///
/// Generated bindings implement this trait from their Rust schema. Python
/// names and values are therefore explicit and never depend on Rust
/// discriminants or declaration order.
#[contract_trait]
trait PythonStringEnum: Copy {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn python_type_name() -> &'static str;

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn python_module_name() -> &'static str;

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn variants() -> &'static [Self];

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn python_member_name(self) -> &'static str;

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn python_value(self) -> &'static str;
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
    fn python_type_name() -> &'static str {
        "SampleMode"
    }

    fn python_module_name() -> &'static str {
        "jbotci"
    }

    fn variants() -> &'static [Self] {
        const VARIANTS: &[SampleMode] = &[SampleMode::Basic, SampleMode::Advanced];
        VARIANTS
    }

    fn python_member_name(self) -> &'static str {
        match self {
            Self::Basic => "BASIC",
            Self::Advanced => "ADVANCED",
        }
    }

    fn python_value(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Advanced => "advanced",
        }
    }
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
    #[ensures(true)]
    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let value_repr = PyString::new(py, &self.value).repr()?.to_str()?.to_owned();
        Ok(format!("jbotci.Sample(value={value_repr})"))
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

/// Register a genuine string-valued Python enum from stable Rust metadata.
#[requires(true)]
#[ensures(true)]
fn register_string_enum<E: PythonStringEnum>(module: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = module.py();
    let members = PyDict::new(py);
    for variant in E::variants().iter().copied() {
        let member_name = variant.python_member_name();
        if members.contains(member_name)? {
            return Err(PyValueError::new_err(format!(
                "duplicate Python enum member name: {member_name}"
            )));
        }
        members.set_item(member_name, variant.python_value())?;
    }

    let keyword_arguments = PyDict::new(py);
    keyword_arguments.set_item("module", E::python_module_name())?;
    let enum_module = py.import("enum")?;
    let enum_type = enum_module
        .getattr("StrEnum")?
        .call((E::python_type_name(), members), Some(&keyword_arguments))?;
    enum_module.getattr("unique")?.call1((&enum_type,))?;
    module.add(E::python_type_name(), enum_type)
}

/// Convert a Rust enum value using its stable string, not its discriminant.
#[requires(true)]
#[ensures(true)]
fn string_enum_member<'py, E: PythonStringEnum>(
    module: &Bound<'py, PyModule>,
    value: E,
) -> PyResult<Bound<'py, PyAny>> {
    module
        .getattr(E::python_type_name())?
        .call1((value.python_value(),))
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
    module.add_function(wrap_pyfunction!(sample_mode, module)?)?;
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
    register_type::<Sample>(module)?;
    register_string_enum::<SampleMode>(module)?;
    register_functions(module)?;
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
        Python::initialize();
        let sample = Sample::new("coi");
        assert_eq!(sample.value(), "coi");
        Python::attach(|py| {
            assert_eq!(sample.__repr__(py).unwrap(), "jbotci.Sample(value='coi')");
        });
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

    #[requires(true)]
    #[ensures(true)]
    #[test]
    fn string_enum_uses_explicit_names_and_values() {
        assert_eq!(
            SampleMode::variants(),
            &[SampleMode::Basic, SampleMode::Advanced]
        );
        assert_eq!(SampleMode::Basic.python_member_name(), "BASIC");
        assert_eq!(SampleMode::Advanced.python_value(), "advanced");
    }
}
