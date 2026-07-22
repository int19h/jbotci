//! Shared registration, ownership, and enum conventions for native domains.

use std::borrow::Cow;
use std::sync::Arc;

use bityzba::{contract_trait, invariant, requires};
use pyo3::PyClass;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule, PyString, PyTuple, PyType};

/// Supplies stable Python metadata for a fieldless Rust enum.
///
/// Native export names are private and domain-qualified because unrelated
/// public modules can legitimately contain classes with the same public name.
/// Public names and modules belong to the created Python class and are not
/// inferred from the private extension-module key.
#[contract_trait]
pub(crate) trait PythonStringEnum: Copy + 'static {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn native_export_name() -> &'static str;

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn python_type_name() -> &'static str;

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn python_module_name() -> &'static str;

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn python_doc() -> &'static str;

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn variants() -> &'static [Self];

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn python_member_name(self) -> Cow<'static, str>;

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn python_value(self) -> &'static str;
}

/// Retains an owner while projecting a stable reference into it.
///
/// The projection is rerun for every borrow, so this helper does not create a
/// Rust self-reference or erase a lifetime. Domain wrappers with multiple
/// possible projections should instead retain the same `Arc` plus typed
/// positions into the owner.
#[invariant(true)]
#[derive(Debug)]
#[allow(dead_code, reason = "shared convention for borrowed-data domains")]
pub(crate) struct OwnedReference<Owner, Target: ?Sized> {
    owner: Arc<Owner>,
    project: for<'a> fn(&'a Owner) -> &'a Target,
}

impl<Owner, Target: ?Sized> OwnedReference<Owner, Target> {
    #[requires(true)]
    #[ensures(true)]
    #[allow(dead_code, reason = "shared convention for borrowed-data domains")]
    pub(crate) fn new(owner: Arc<Owner>, project: for<'a> fn(&'a Owner) -> &'a Target) -> Self {
        Self { owner, project }
    }

    #[requires(true)]
    #[ensures(true)]
    #[allow(dead_code, reason = "shared convention for borrowed-data domains")]
    pub(crate) fn get(&self) -> &Target {
        (self.project)(&self.owner)
    }
}

/// Convert a Rust sequence into the package's immutable Python representation.
#[requires(true)]
#[ensures(true)]
pub(crate) fn sequence_to_tuple<'py, T, I>(
    py: Python<'py>,
    values: I,
) -> PyResult<Bound<'py, PyTuple>>
where
    T: IntoPyObject<'py>,
    I: IntoIterator<Item = T>,
    I::IntoIter: ExactSizeIterator,
{
    PyTuple::new(py, values)
}

/// Register a Python class under a unique private native export key.
#[requires(private_export_name.starts_with('_'))]
#[ensures(true)]
pub(crate) fn register_type<T: PyClass>(
    module: &Bound<'_, PyModule>,
    private_export_name: &str,
) -> PyResult<()> {
    ensure_private_export_available(module, private_export_name)?;
    module.add(private_export_name, module.py().get_type::<T>())
}

/// Register an already-created object under a private native export key.
#[requires(private_export_name.starts_with('_'))]
#[ensures(true)]
pub(crate) fn register_private_object<'py, T>(
    module: &Bound<'py, PyModule>,
    private_export_name: &str,
    value: T,
) -> PyResult<()>
where
    T: IntoPyObject<'py>,
{
    ensure_private_export_available(module, private_export_name)?;
    module.add(private_export_name, value)
}

#[requires(private_export_name.starts_with('_'))]
#[ensures(true)]
fn ensure_private_export_available(
    module: &Bound<'_, PyModule>,
    private_export_name: &str,
) -> PyResult<()> {
    if module.hasattr(private_export_name)? {
        Err(PyValueError::new_err(format!(
            "duplicate private native export: {private_export_name}"
        )))
    } else {
        Ok(())
    }
}

/// Register a genuine string-valued Python enum from stable Rust metadata.
#[requires(E::native_export_name().starts_with('_'))]
#[ensures(true)]
pub(crate) fn register_string_enum<E: PythonStringEnum>(
    module: &Bound<'_, PyModule>,
) -> PyResult<()> {
    ensure_private_export_available(module, E::native_export_name())?;
    let py = module.py();
    let members = PyDict::new(py);
    for variant in E::variants().iter().copied() {
        let member_name = variant.python_member_name();
        if members.contains(member_name.as_ref())? {
            return Err(PyValueError::new_err(format!(
                "duplicate Python enum member name: {member_name}"
            )));
        }
        members.set_item(member_name.as_ref(), variant.python_value())?;
    }

    let keyword_arguments = PyDict::new(py);
    keyword_arguments.set_item("module", E::python_module_name())?;
    let enum_module = py.import("enum")?;
    let enum_type = enum_module
        .getattr("StrEnum")?
        .call((E::python_type_name(), members), Some(&keyword_arguments))?;
    enum_type.setattr("__doc__", E::python_doc())?;
    enum_module.getattr("unique")?.call1((&enum_type,))?;
    module.add(E::native_export_name(), enum_type)
}

/// Convert a Rust enum value to the singleton member of its registered class.
#[requires(true)]
#[ensures(true)]
pub(crate) fn string_enum_member<'py, E: PythonStringEnum>(
    module: &Bound<'py, PyModule>,
    value: E,
) -> PyResult<Bound<'py, PyAny>> {
    module
        .getattr(E::native_export_name())?
        .call1((value.python_value(),))
}

/// Extract an enum only from the exact scaffold-registered `StrEnum` class.
///
/// Extracting `str` first would silently accept an arbitrary string because a
/// `StrEnum` member is also a string. Identity-checking the registered class
/// before inspecting its value preserves the closed Rust enum boundary.
#[requires(true)]
#[ensures(true)]
#[allow(
    dead_code,
    reason = "shared input conversion for domains that accept fieldless enums"
)]
pub(crate) fn extract_string_enum<E: PythonStringEnum>(
    module: &Bound<'_, PyModule>,
    value: &Bound<'_, PyAny>,
) -> PyResult<E> {
    let registered = module.getattr(E::native_export_name())?;
    let registered_type = registered.cast::<PyType>()?;
    if !value.get_type().is(registered_type) {
        return Err(PyTypeError::new_err(format!(
            "expected {}.{}, not {}",
            E::python_module_name(),
            E::python_type_name(),
            value.get_type().qualname()?
        )));
    }

    let canonical_value = value.getattr("value")?.extract::<String>()?;
    E::variants()
        .iter()
        .copied()
        .find(|variant| variant.python_value() == canonical_value)
        .ok_or_else(|| {
            PyValueError::new_err(format!(
                "{}.{} has non-canonical value {canonical_value:?}",
                E::python_module_name(),
                E::python_type_name()
            ))
        })
}

/// Return Python's canonical repr for a string without reimplementing escapes.
#[requires(true)]
#[ensures(true)]
pub(crate) fn string_repr(py: Python<'_>, value: &str) -> PyResult<String> {
    Ok(PyString::new(py, value).repr()?.to_str()?.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[invariant(::First => true)]
    #[invariant(::Second => true)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestEnum {
        First,
        Second,
    }

    #[contract_trait]
    impl PythonStringEnum for TestEnum {
        fn native_export_name() -> &'static str {
            "_test_TestEnum"
        }

        fn python_type_name() -> &'static str {
            "TestEnum"
        }

        fn python_module_name() -> &'static str {
            "tests"
        }

        fn python_doc() -> &'static str {
            "Test enum."
        }

        fn variants() -> &'static [Self] {
            const VARIANTS: &[TestEnum] = &[TestEnum::First, TestEnum::Second];
            VARIANTS
        }

        fn python_member_name(self) -> Cow<'static, str> {
            match self {
                Self::First => Cow::Borrowed("FIRST"),
                Self::Second => Cow::Borrowed("SECOND"),
            }
        }

        fn python_value(self) -> &'static str {
            match self {
                Self::First => "first",
                Self::Second => "second",
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn project_text(owner: &String) -> &str {
        owner.as_str()
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn owned_reference_keeps_its_owner_alive() {
        let owner = Arc::new(String::from("coi"));
        let reference = OwnedReference::new(Arc::clone(&owner), project_text);
        drop(owner);
        assert_eq!(reference.get(), "coi");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn exact_enum_extraction_rejects_plain_strings() {
        Python::initialize();
        Python::attach(|py| {
            let module = PyModule::new(py, "test_native").unwrap();
            register_string_enum::<TestEnum>(&module).unwrap();
            assert!(register_string_enum::<TestEnum>(&module).is_err());
            let member = string_enum_member(&module, TestEnum::Second).unwrap();
            assert_eq!(
                extract_string_enum::<TestEnum>(&module, &member).unwrap(),
                TestEnum::Second
            );
            assert!(
                extract_string_enum::<TestEnum>(&module, PyString::new(py, "second").as_any())
                    .is_err()
            );
            let impostor_members = PyDict::new(py);
            impostor_members.set_item("SECOND", "second").unwrap();
            let impostor = py
                .import("enum")
                .unwrap()
                .getattr("StrEnum")
                .unwrap()
                .call1(("TestEnum", impostor_members))
                .unwrap()
                .call1(("second",))
                .unwrap();
            assert!(extract_string_enum::<TestEnum>(&module, &impostor).is_err());
        });
    }
}
