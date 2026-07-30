#!/usr/bin/env python3
"""Assertions shared by local tests and clean installed-wheel tests."""

from __future__ import annotations

import ast
import importlib
import importlib.metadata
import inspect
from pathlib import Path
from types import ModuleType

PUBLIC_MODULES: tuple[str, ...] = (
    "jbotci",
    "jbotci.diagnostics",
    "jbotci.dialect",
    "jbotci.dictionary",
    "jbotci.jvozba",
    "jbotci.morphology",
    "jbotci.semantics",
    "jbotci.semantics.references",
    "jbotci.source",
    "jbotci.syntax",
    "jbotci.syntax.recovered",
    "jbotci.syntax.strict",
)

INTENTIONAL_STUB_ONLY_NATIVE_DECLARATIONS: frozenset[str] = frozenset(
    {
        "_MorphologyErrorValueBase",
        "_MorphologyWordBase",
        "_MorphologyWordLikeBase",
        "_ValsiClassificationBase",
    }
)

REQUIRED_PACKAGE_FILES: tuple[str, ...] = (
    "py.typed",
    "_native.pyi",
    "syntax/strict.pyi",
    "syntax/recovered.pyi",
)


def _stub_declarations(path: Path) -> set[str]:
    """Return runtime-like declarations from one top-level stub module."""
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    declarations = {
        node.name
        for node in tree.body
        if isinstance(node, (ast.ClassDef, ast.FunctionDef))
    }
    declarations.update(
        node.target.id
        for node in tree.body
        if isinstance(node, ast.AnnAssign)
        and isinstance(node.target, ast.Name)
        and node.target.id != "__all__"
        and not (
            isinstance(node.annotation, ast.Name)
            and node.annotation.id == "TypeAlias"
        )
    )
    return declarations


def _generated_stub_inventory(path: Path) -> tuple[tuple[str, ...], tuple[str, ...]]:
    """Return all declarations and concrete classes in source order."""
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    declarations: list[str] = []
    classes: list[str] = []
    for statement in tree.body:
        if isinstance(statement, ast.ClassDef):
            declarations.append(statement.name)
            classes.append(statement.name)
        elif (
            isinstance(statement, ast.AnnAssign)
            and isinstance(statement.target, ast.Name)
            and isinstance(statement.annotation, ast.Name)
            and statement.annotation.id == "TypeAlias"
        ):
            declarations.append(statement.target.id)
    return (tuple(declarations), tuple(classes))


def import_public_modules() -> dict[str, ModuleType]:
    """Import every documented public module."""
    modules = {
        name: importlib.import_module(name)
        for name in PUBLIC_MODULES
    }
    for name, module in modules.items():
        assert module.__doc__ is not None and module.__doc__.strip(), name
        if name != "jbotci":
            assert getattr(module, "__all__"), name
    return modules


def assert_installed_package(
    *,
    source_package_root: Path,
    expected_version: str,
) -> None:
    """Check installed metadata, package data, and runtime/stub inventories."""
    modules = import_public_modules()
    jbotci = modules["jbotci"]
    native = importlib.import_module("jbotci._native")
    assert jbotci.__file__ is not None
    package_dir = Path(jbotci.__file__).resolve().parent
    source_package_root = source_package_root.resolve()
    assert not package_dir.is_relative_to(source_package_root), (
        package_dir,
        source_package_root,
    )

    distribution = importlib.metadata.distribution("jbotci")
    metadata = distribution.metadata
    assert metadata["Name"] == "jbotci"
    assert metadata["Version"] == expected_version
    assert getattr(jbotci, "__version__") == expected_version
    assert metadata["Summary"] == (
        "Pre-alpha Python bindings for the unstable jbotci Rust API."
    )
    assert metadata["License-Expression"] == "MIT"
    assert metadata["Description-Content-Type"].startswith("text/markdown")
    metadata_text = distribution.read_text("METADATA")
    assert metadata_text is not None
    _, separator, description = metadata_text.partition("\n\n")
    assert separator and description.startswith("# jbotci Python bindings\n")
    assert "Development Status :: 2 - Pre-Alpha" in metadata.get_all(
        "Classifier", []
    )
    assert "LICENSE.md" in metadata.get_all("License-File", [])

    for relative in REQUIRED_PACKAGE_FILES:
        path = package_dir / relative
        assert path.is_file(), path
    assert (package_dir / "py.typed").read_bytes() == b""

    native_declarations = _stub_declarations(package_dir / "_native.pyi")
    assert native_declarations == (
        set(native.__all__) | INTENTIONAL_STUB_ONLY_NATIVE_DECLARATIONS
    )
    assert all(hasattr(native, name) for name in native.__all__)
    assert all(
        not hasattr(native, name)
        for name in INTENTIONAL_STUB_ONLY_NATIVE_DECLARATIONS
    )

    for module_name, stub_name, inventory_name, concrete_inventory_name in (
        (
            "jbotci.syntax.strict",
            "strict.pyi",
            "_syntax_STRICT_INVENTORY",
            "_syntax_STRICT_CONCRETE_INVENTORY",
        ),
        (
            "jbotci.syntax.recovered",
            "recovered.pyi",
            "_syntax_RECOVERED_INVENTORY",
            "_syntax_RECOVERED_CONCRETE_INVENTORY",
        ),
    ):
        module = modules[module_name]
        declarations, classes = _generated_stub_inventory(
            package_dir / "syntax" / stub_name
        )
        runtime_inventory = getattr(native, inventory_name)
        concrete_inventory = getattr(native, concrete_inventory_name)
        assert module.__all__ == runtime_inventory == declarations
        assert concrete_inventory == classes
        for class_name in concrete_inventory:
            runtime_class = getattr(module, class_name)
            assert inspect.isclass(runtime_class)
            assert runtime_class.__module__ == module_name

    reference_inventory = native._references_RUNTIME_INVENTORY
    assert reference_inventory == tuple(
        name for name in native.__all__ if name.startswith("_references_")
    )
    assert set(reference_inventory) == {
        name
        for name in native_declarations
        if name.startswith("_references_")
    }

    assert jbotci.smoke() == "jbotci native bindings ready"
    dictionary = modules["jbotci.dictionary"]
    assert len(dictionary.english) == 17_536
    assert dictionary.english_metadata.entry_count == len(dictionary.english)
    assert dictionary.english.lookup_word("tavla") is not None
