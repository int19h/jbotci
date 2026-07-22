from __future__ import annotations

import ast
import importlib
import importlib.metadata
import subprocess
import sys
import tomllib
from pathlib import Path
from types import ModuleType

import pytest

import jbotci
import jbotci._native as native

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
    assert repr(sample) == 'jbotci.Sample(value="coi")'
    assert sample == jbotci.Sample("coi")
    assert sample != jbotci.Sample("co'o")
    assert hash(sample) == hash(jbotci.Sample("coi"))

    with pytest.raises(AttributeError):
        sample.value = "co'o"  # type: ignore[misc]

    with pytest.raises(TypeError):
        type("Derived", (jbotci.Sample,), {})


def test_structured_error_conversion_uses_public_hierarchy() -> None:
    with pytest.raises(jbotci.InvalidInputError, match="invalid sample") as caught:
        jbotci.raise_sample_error("invalid sample")
    assert isinstance(caught.value, jbotci.JbotciError)
    assert jbotci.JbotciError.__module__ == "jbotci"
    assert jbotci.InvalidInputError.__module__ == "jbotci"


@pytest.mark.parametrize(
    ("module_name", "exports"),
    [
        ("source", ()),
        ("morphology", ()),
        ("syntax", ()),
        ("dictionary", ()),
        ("jvozba", ()),
        ("semantics", ("references",)),
        ("semantics.references", ()),
    ],
)
def test_typed_namespace_is_importable(
    module_name: str, exports: tuple[str, ...]
) -> None:
    module = importlib.import_module(f"jbotci.{module_name}")
    assert isinstance(module, ModuleType)
    assert getattr(module, "__all__") == exports


def test_stub_composition_is_current() -> None:
    result = subprocess.run(
        [sys.executable, str(PACKAGE_ROOT / "tools" / "compose_stubs.py"), "--check"],
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
