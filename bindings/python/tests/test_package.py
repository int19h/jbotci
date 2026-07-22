from __future__ import annotations

import ast
import enum
import importlib
import importlib.metadata
import inspect
import pickle
import subprocess
import sys
import tomllib
from pathlib import Path
from types import ModuleType

import pytest

import jbotci
import jbotci._native as native
from jbotci import diagnostics, dialect, morphology, source

PACKAGE_ROOT = Path(__file__).resolve().parents[1]
WORKSPACE_ROOT = PACKAGE_ROOT.parents[1]


def test_version_comes_from_cargo_workspace() -> None:
    workspace_manifest = tomllib.loads(
        (WORKSPACE_ROOT / "Cargo.toml").read_text(encoding="utf-8")
    )
    cargo_version = workspace_manifest["workspace"]["package"]["version"]
    assert isinstance(cargo_version, str)
    assert jbotci.__version__ == cargo_version
    assert importlib.metadata.version("jbotci") == cargo_version


def test_native_module_and_smoke_function() -> None:
    assert native.__name__ == "jbotci._native"
    assert jbotci.smoke() == "jbotci native bindings ready"


def test_sample_is_immutable_non_subclassable_value() -> None:
    sample = jbotci.Sample("coi")
    assert sample.value == "coi"
    assert repr(sample) == "jbotci.Sample(value='coi')"
    assert sample == jbotci.Sample("coi")
    assert sample != jbotci.Sample("co'o")
    assert hash(sample) == hash(jbotci.Sample("coi"))

    with pytest.raises(AttributeError):
        sample.value = "co'o"  # type: ignore[misc]

    with pytest.raises(TypeError):
        type("Derived", (jbotci.Sample,), {})


def test_sample_repr_uses_python_string_escaping() -> None:
    value = "\x7f\ncafé'\\"
    sample = jbotci.Sample(value)
    assert repr(sample) == f"jbotci.Sample(value={value!r})"
    assert eval(repr(sample), {"jbotci": jbotci}) == sample


def test_string_enum_uses_stable_names_and_values() -> None:
    assert issubclass(jbotci.SampleMode, enum.Enum)
    assert issubclass(jbotci.SampleMode, str)
    assert set(jbotci.SampleMode) == {
        jbotci.SampleMode.BASIC,
        jbotci.SampleMode.ADVANCED,
    }
    assert jbotci.SampleMode.BASIC.value == "basic"
    assert jbotci.SampleMode.ADVANCED.value == "advanced"
    assert jbotci.sample_mode() is jbotci.SampleMode.BASIC
    assert jbotci.sample_mode(advanced=True) is jbotci.SampleMode.ADVANCED
    assert jbotci.SampleMode.__module__ == "jbotci"
    assert jbotci.SampleMode.__name__ == "SampleMode"
    assert native._root_SampleMode is jbotci.SampleMode
    assert native._root_Sample is jbotci.Sample
    with pytest.raises(ValueError):
        jbotci.SampleMode(0)  # type: ignore[arg-type]


def test_structured_error_conversion_uses_public_hierarchy() -> None:
    with pytest.raises(jbotci.InvalidInputError, match="invalid sample") as caught:
        jbotci.raise_sample_error("invalid sample")
    assert isinstance(caught.value, jbotci.JbotciError)
    assert jbotci.JbotciError.__module__ == "jbotci"
    assert jbotci.InvalidInputError.__module__ == "jbotci"


@pytest.mark.parametrize(
    ("module_name", "exports"),
    [
        ("source", None),
        ("diagnostics", None),
        ("dialect", None),
        ("morphology", None),
        ("syntax", ()),
        ("dictionary", None),
        ("jvozba", ()),
        ("semantics", ("references",)),
        ("semantics.references", ()),
    ],
)
def test_typed_namespace_is_importable(
    module_name: str, exports: tuple[str, ...] | None
) -> None:
    module = importlib.import_module(f"jbotci.{module_name}")
    assert isinstance(module, ModuleType)
    if exports is None:
        assert getattr(module, "__all__")
    else:
        assert getattr(module, "__all__") == exports


def test_stub_composition_is_current() -> None:
    result = subprocess.run(
        [sys.executable, str(PACKAGE_ROOT / "tools" / "compose_stubs.py"), "--check"],
        check=False,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr


def test_domain_enum_stubs_are_current() -> None:
    result = subprocess.run(
        [
            sys.executable,
            str(PACKAGE_ROOT / "tools" / "generate_domain_enum_stubs.py"),
            "--check",
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr


def test_native_stub_exports_match_runtime() -> None:
    stub_path = PACKAGE_ROOT / "python" / "jbotci" / "_native.pyi"
    tree = ast.parse(stub_path.read_text(encoding="utf-8"), filename=str(stub_path))
    declaration_names = {
        node.name
        for node in tree.body
        if isinstance(node, (ast.ClassDef, ast.FunctionDef))
    }
    declaration_names.update(
        node.target.id
        for node in tree.body
        if isinstance(node, ast.AnnAssign)
        and isinstance(node.target, ast.Name)
        and node.target.id != "__all__"
    )
    assert declaration_names == set(native.__all__)
    assert all(hasattr(native, name) for name in native.__all__)


def _stub_classes(path: Path) -> dict[str, ast.ClassDef]:
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    return {
        declaration.name: declaration
        for declaration in tree.body
        if isinstance(declaration, ast.ClassDef)
    }


def _stub_class_functions(
    classes: dict[str, ast.ClassDef], declaration: ast.ClassDef
) -> dict[str, ast.FunctionDef]:
    functions: dict[str, ast.FunctionDef] = {}
    for base in declaration.bases:
        if isinstance(base, ast.Name) and base.id in classes:
            functions.update(_stub_class_functions(classes, classes[base.id]))
    functions.update(
        {
            statement.name: statement
            for statement in declaration.body
            if isinstance(statement, ast.FunctionDef)
        }
    )
    return functions


def _stub_parameter_shape(
    declaration: ast.FunctionDef, *, constructor: bool
) -> tuple[tuple[str, bool, bool], ...]:
    positional = [*declaration.args.posonlyargs, *declaration.args.args]
    positional_defaults = [False] * (
        len(positional) - len(declaration.args.defaults)
    ) + [True] * len(declaration.args.defaults)
    parameters = [
        (argument.arg, False, has_default)
        for argument, has_default in zip(positional, positional_defaults, strict=True)
    ]
    if constructor:
        assert parameters and parameters[0][0] == "cls"
        parameters = parameters[1:]
    parameters.extend(
        (argument.arg, True, default is not None)
        for argument, default in zip(
            declaration.args.kwonlyargs, declaration.args.kw_defaults, strict=True
        )
    )
    if declaration.args.vararg is not None:
        parameters.append((declaration.args.vararg.arg, False, False))
    if declaration.args.kwarg is not None:
        parameters.append((declaration.args.kwarg.arg, True, False))
    return tuple(parameters)


def _runtime_parameter_shape(
    signature: inspect.Signature,
) -> tuple[tuple[str, bool, bool], ...]:
    return tuple(
        (
            parameter.name,
            parameter.kind is inspect.Parameter.KEYWORD_ONLY,
            parameter.default is not inspect.Parameter.empty,
        )
        for parameter in signature.parameters.values()
    )


def _annotation_mentions_any(annotation: ast.expr) -> bool:
    return any(
        isinstance(node, ast.Name) and node.id == "Any"
        for node in ast.walk(annotation)
    )


def test_morphology_stub_class_members_signatures_and_match_args_match_runtime() -> None:
    """Check the complete manual morphology stub surface, not only exports."""

    stub_path = PACKAGE_ROOT / "stubs" / "_native" / "morphology.pyi"
    classes = _stub_classes(stub_path)
    declarations = {
        name: declaration
        for name, declaration in classes.items()
        if name.startswith("_morphology_")
    }
    for name, declaration in declarations.items():
        runtime_class = getattr(native, name)
        functions = _stub_class_functions(classes, declaration)
        public_stub_members = {
            function_name
            for function_name in functions
            if not function_name.startswith("_")
        }
        public_runtime_members = {
            member_name
            for member_name in dir(runtime_class)
            if not member_name.startswith("_")
        }
        assert public_runtime_members == public_stub_members, name

        own_functions = {
            statement.name: statement
            for statement in declaration.body
            if isinstance(statement, ast.FunctionDef)
        }
        match_args_declaration = next(
            (
                statement
                for statement in declaration.body
                if isinstance(statement, ast.AnnAssign)
                and isinstance(statement.target, ast.Name)
                and statement.target.id == "__match_args__"
            ),
            None,
        )
        if match_args_declaration is not None:
            constructor = own_functions.get("__new__")
            if constructor is not None:
                expected_match_args = tuple(
                    parameter[0]
                    for parameter in _stub_parameter_shape(
                        constructor, constructor=True
                    )
                )
            else:
                expected_match_args = tuple(
                    statement.name
                    for statement in declaration.body
                    if isinstance(statement, ast.FunctionDef)
                    and any(
                        isinstance(decorator, ast.Name)
                        and decorator.id == "property"
                        for decorator in statement.decorator_list
                    )
                )
            assert runtime_class.__match_args__ == expected_match_args, name

        for function_name, function in functions.items():
            assert function.returns is not None, f"{name}.{function_name}"
            assert not _annotation_mentions_any(
                function.returns
            ), f"{name}.{function_name}"
            if function_name.startswith("__") and function_name != "__new__":
                continue
            if any(
                isinstance(decorator, ast.Name) and decorator.id == "property"
                for decorator in function.decorator_list
            ):
                continue
            if function_name == "__new__":
                runtime_signature = inspect.signature(runtime_class)
                constructor = True
            else:
                runtime_signature = inspect.signature(
                    getattr(runtime_class, function_name)
                )
                constructor = False
            assert _runtime_parameter_shape(runtime_signature) == (
                _stub_parameter_shape(function, constructor=constructor)
            ), f"{name}.{function_name}"


def test_generated_domain_enum_members_match_runtime_rust_metadata() -> None:
    """Catch per-variant omissions or value drift in generated enum stubs."""

    stub_path = PACKAGE_ROOT / "stubs" / "_native" / "domain_enums.pyi"
    tree = ast.parse(stub_path.read_text(encoding="utf-8"), filename=str(stub_path))
    declarations = tuple(node for node in tree.body if isinstance(node, ast.ClassDef))
    for declaration in declarations:
        expected = tuple(
            (statement.targets[0].id, ast.literal_eval(statement.value))
            for statement in declaration.body
            if isinstance(statement, ast.Assign)
            and len(statement.targets) == 1
            and isinstance(statement.targets[0], ast.Name)
        )
        runtime_enum = getattr(native, declaration.name)
        actual = tuple(
            (name, member.value) for name, member in runtime_enum.__members__.items()
        )
        assert actual == expected, declaration.name
    domain_prefixes = ("_diagnostics_", "_dialect_", "_morphology_")
    runtime_enums = {
        name
        for name in native.__all__
        if name.startswith(domain_prefixes)
        and isinstance((value := getattr(native, name)), type)
        and issubclass(value, enum.StrEnum)
    }
    assert runtime_enums == {declaration.name for declaration in declarations}


@pytest.mark.parametrize("module", (source, diagnostics, dialect, morphology))
def test_public_callables_have_stable_introspection_and_pickle_identity(
    module: ModuleType,
) -> None:
    for export_name in module.__all__:
        exported = getattr(module, export_name)
        if not callable(exported):
            continue
        assert exported.__module__ == module.__name__, export_name
        assert exported.__name__ == export_name
        assert getattr(module, export_name) is pickle.loads(pickle.dumps(exported))


@pytest.mark.parametrize("module", (source, diagnostics, dialect, morphology))
def test_domain_api_has_complete_runtime_docstrings(module: ModuleType) -> None:
    """Keep native documentation attached to every public consumer surface."""

    for export_name in module.__all__:
        exported = getattr(module, export_name)
        if not (inspect.isroutine(exported) or isinstance(exported, type)):
            continue
        assert inspect.getdoc(exported), f"{module.__name__}.{export_name}"
        if not isinstance(exported, type):
            continue
        for member_name, member in vars(exported).items():
            if member_name.startswith("_"):
                continue
            if inspect.isroutine(member) or inspect.isdatadescriptor(member):
                assert inspect.getdoc(member), (
                    f"{module.__name__}.{export_name}.{member_name}"
                )
