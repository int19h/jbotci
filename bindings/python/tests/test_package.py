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

MORPHOLOGY_MATCH_ARGS: dict[str, tuple[str, ...]] = {
    "_morphology_CompiledDialectSwap": ("left", "right"),
    "_morphology_CompiledDialectExpansion": ("source", "replacement"),
    "_morphology_LujvoRafsi": ("phonemes",),
    "_morphology_LujvoHyphen": ("phonemes",),
    "_morphology_Verbatim": ("span", "text"),
    "_morphology_CmavoWord": ("phonemes", "span"),
    "_morphology_GismuWord": ("phonemes", "span"),
    "_morphology_FuhivlaWord": ("phonemes", "span"),
    "_morphology_CmevlaWord": ("phonemes", "span"),
    "_morphology_LujvoWord": ("parts", "span"),
    "_morphology_PlainWord": ("word",),
    "_morphology_QuotedWord": ("zo", "word"),
    "_morphology_SelmahoQuotedWord": ("mahoi", "word"),
    "_morphology_DelimitedNonLojbanQuote": (
        "zoi",
        "opening_delimiter",
        "quoted_text",
        "closing_delimiter",
    ),
    "_morphology_QuotedWords": ("lohu", "quoted_words", "lehu"),
    "_morphology_DelimitedWordQuote": ("marker", "quoted_text"),
    "_morphology_LerfuWord": ("base", "bu"),
    "_morphology_ZeiCompound": ("left", "zei", "right"),
    "_morphology_MorphologyContext": ("kind", "char_start", "char_end"),
    "_morphology_InvalidLujvoDetail": ("parsed_prefix", "expected"),
    "_morphology_FuhivlaContainsYDetail": (),
    "_morphology_SlinkuhiDetail": (),
    "_morphology_ExpectedWordDetail": ("expected",),
    "_morphology_InvalidZoiDelimiterDetail": ("reason",),
    "_morphology_PhonotacticDetail": ("reason",),
    "_morphology_MorphologyWarning": (
        "kind",
        "char_start",
        "char_end",
        "text",
        "context",
        "ignored_character_count",
    ),
    "_morphology_InvalidMorphology": (
        "kind",
        "char_start",
        "char_end",
        "text",
        "context",
        "detail",
    ),
    "_morphology_UnterminatedZoiQuote": (
        "char_offset",
        "delimiter",
        "context",
    ),
    "_morphology_SourceSpanMorphologyError": ("error",),
    "_morphology_ValsiLujvoPart": ("kind", "text", "rafsi_kind"),
    "_morphology_PlainWordClassification": (
        "category",
        "phonemes",
        "selmaho",
        "split",
        "parts",
        "stage",
    ),
    "_morphology_PlainWordValsiClassification": ("word",),
    "_morphology_QuotedWordValsiClassification": ("marker", "quoted_word"),
    "_morphology_DelimitedNonLojbanQuoteValsiClassification": (
        "marker",
        "delimiter",
    ),
    "_morphology_QuotedWordsValsiClassification": ("marker", "quoted_words"),
    "_morphology_DelimitedWordQuoteValsiClassification": ("marker_text",),
    "_morphology_LerfuWordValsiClassification": ("base", "suffix"),
    "_morphology_ZeiCompoundValsiClassification": ("left", "link", "right"),
    "_morphology_ValsiAnalysis": ("input", "warnings", "result"),
    "_morphology_LujvoRafsiBuildPart": ("text",),
    "_morphology_LujvoBrivlaCoreBuildPart": ("text",),
    "_morphology_LujvoCandidate": ("word", "parts", "score"),
}

MORPHOLOGY_CALLABLE_DEFAULTS: dict[str, dict[str, object]] = {
    "_morphology_PhonemeRenderOptions.__new__": {
        "mark_stress": None,
        "mark_glides": None,
    },
    "_morphology_CompiledDialectDefinition.__new__": {"definition": None},
    "_morphology_MorphologyOptions.__new__": {
        "accept_latin": True,
        "accept_cyrillic": True,
        "accept_zbalermorna": True,
        "dialect": None,
        "cmevla_as_relation_words": False,
        "permissive_lexer": False,
        "uppercase_marks_stress": True,
        "max_recovery_errors": 20,
        "trace": None,
    },
    "_morphology_InvalidLujvoDetail.__new__": {"parsed_prefix": None},
    "_morphology_MorphologyWarning.__new__": {
        "context": None,
        "ignored_character_count": None,
    },
    "_morphology_MorphologyWarning.to_diagnostic": {"source_id": None},
    "_morphology_InvalidMorphology.__new__": {
        "context": None,
        "detail": None,
    },
    "_morphology_InvalidMorphology.to_diagnostic": {"source_id": None},
    "_morphology_UnterminatedZoiQuote.__new__": {"context": None},
    "_morphology_UnterminatedZoiQuote.to_diagnostic": {"source_id": None},
    "_morphology_SourceSpanMorphologyError.to_diagnostic": {"source_id": None},
    "_morphology_ValsiLujvoPart.__new__": {"rafsi_kind": None},
    "_morphology_PlainWordClassification.__new__": {
        "selmaho": None,
        "split": None,
        "parts": [],
        "stage": None,
    },
    "_morphology_ValsiAnalysisResult.__new__": {
        "word": None,
        "classification": None,
        "error": None,
        "words": [],
    },
    "_morphology_segment_attempt": {"options": None, "source_id": None},
    "_morphology_segment_recovered_attempt": {
        "options": None,
        "source_id": None,
    },
    "_morphology_segment_for_display_attempt": {
        "options": None,
        "source_id": None,
    },
    "_morphology_analyze_valsi": {"options": None, "source_id": None},
    "_morphology_normalize_input": {"options": None},
    "_morphology_is_word_forming_character": {"options": None},
}


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


def _stub_class_attributes(
    classes: dict[str, ast.ClassDef], declaration: ast.ClassDef
) -> dict[str, ast.AnnAssign]:
    attributes: dict[str, ast.AnnAssign] = {}
    for base in declaration.bases:
        if isinstance(base, ast.Name) and base.id in classes:
            attributes.update(_stub_class_attributes(classes, classes[base.id]))
    attributes.update(
        {
            statement.target.id: statement
            for statement in declaration.body
            if isinstance(statement, ast.AnnAssign)
            and isinstance(statement.target, ast.Name)
        }
    )
    return attributes


def _stub_function_is_property(declaration: ast.FunctionDef) -> bool:
    return any(
        isinstance(decorator, ast.Name) and decorator.id == "property"
        for decorator in declaration.decorator_list
    )


def _stub_parameter_shape(
    declaration: ast.FunctionDef, *, constructor: bool
) -> tuple[tuple[str, str, bool], ...]:
    positional = [*declaration.args.posonlyargs, *declaration.args.args]
    positional_defaults = [False] * (
        len(positional) - len(declaration.args.defaults)
    ) + [True] * len(declaration.args.defaults)
    parameters = [
        (argument.arg, kind, has_default)
        for argument, kind, has_default in zip(
            positional,
            ["POSITIONAL_ONLY"] * len(declaration.args.posonlyargs)
            + ["POSITIONAL_OR_KEYWORD"] * len(declaration.args.args),
            positional_defaults,
            strict=True,
        )
    ]
    if constructor:
        assert parameters and parameters[0][0] == "cls"
        parameters = parameters[1:]
    if declaration.args.vararg is not None:
        parameters.append((declaration.args.vararg.arg, "VAR_POSITIONAL", False))
    parameters.extend(
        (argument.arg, "KEYWORD_ONLY", default is not None)
        for argument, default in zip(
            declaration.args.kwonlyargs, declaration.args.kw_defaults, strict=True
        )
    )
    if declaration.args.kwarg is not None:
        parameters.append((declaration.args.kwarg.arg, "VAR_KEYWORD", False))
    return tuple(parameters)


def _runtime_parameter_shape(
    signature: inspect.Signature,
) -> tuple[tuple[str, str, bool], ...]:
    return tuple(
        (
            parameter.name,
            parameter.kind.name,
            parameter.default is not inspect.Parameter.empty,
        )
        for parameter in signature.parameters.values()
    )


def _runtime_parameter_defaults(signature: inspect.Signature) -> dict[str, object]:
    return {
        parameter.name: parameter.default
        for parameter in signature.parameters.values()
        if parameter.default is not inspect.Parameter.empty
    }


def _stub_match_args_arity(annotation: ast.expr) -> int:
    assert isinstance(annotation, ast.Subscript)
    assert isinstance(annotation.value, ast.Name)
    assert annotation.value.id == "tuple"
    elements = (
        annotation.slice.elts
        if isinstance(annotation.slice, ast.Tuple)
        else [annotation.slice]
    )
    assert all(
        isinstance(element, ast.Name) and element.id == "str" for element in elements
    )
    return len(elements)


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
    declared_match_args = {
        name
        for name, declaration in declarations.items()
        if any(
            isinstance(statement, ast.AnnAssign)
            and isinstance(statement.target, ast.Name)
            and statement.target.id == "__match_args__"
            for statement in declaration.body
        )
    }
    runtime_match_args = {
        name
        for name in declarations
        if hasattr(getattr(native, name), "__match_args__")
    }
    assert runtime_match_args == declared_match_args == set(MORPHOLOGY_MATCH_ARGS)
    checked_default_callables: set[str] = set()
    for name, declaration in declarations.items():
        runtime_class = getattr(native, name)
        functions = _stub_class_functions(classes, declaration)
        attributes = _stub_class_attributes(classes, declaration)
        public_stub_members = {
            function_name
            for function_name in functions
            if not function_name.startswith("_")
        }
        public_stub_members.update(
            attribute_name
            for attribute_name in attributes
            if not attribute_name.startswith("_")
        )
        public_runtime_members = {
            member_name
            for member_name in dir(runtime_class)
            if not member_name.startswith("_")
        }
        assert public_runtime_members == public_stub_members, name
        for property_name, function in functions.items():
            if not _stub_function_is_property(function):
                continue
            descriptor = inspect.getattr_static(runtime_class, property_name)
            assert inspect.isdatadescriptor(descriptor), f"{name}.{property_name}"
            assert not callable(descriptor), f"{name}.{property_name}"

        if name in MORPHOLOGY_MATCH_ARGS:
            match_args_declaration = next(
                statement
                for statement in declaration.body
                if isinstance(statement, ast.AnnAssign)
                and isinstance(statement.target, ast.Name)
                and statement.target.id == "__match_args__"
            )
            assert _stub_match_args_arity(match_args_declaration.annotation) == len(
                MORPHOLOGY_MATCH_ARGS[name]
            ), name
            assert runtime_class.__match_args__ == MORPHOLOGY_MATCH_ARGS[name], name

        for function_name, function in functions.items():
            assert function.returns is not None, f"{name}.{function_name}"
            assert not _annotation_mentions_any(
                function.returns
            ), f"{name}.{function_name}"
            if function_name.startswith("__") and function_name != "__new__":
                continue
            if _stub_function_is_property(function):
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
            runtime_defaults = _runtime_parameter_defaults(runtime_signature)
            if runtime_defaults:
                callable_name = f"{name}.{function_name}"
                assert runtime_defaults == MORPHOLOGY_CALLABLE_DEFAULTS[callable_name]
                checked_default_callables.add(callable_name)

    stub_tree = ast.parse(
        stub_path.read_text(encoding="utf-8"), filename=str(stub_path)
    )
    functions = {
        declaration.name: declaration
        for declaration in stub_tree.body
        if isinstance(declaration, ast.FunctionDef)
        and declaration.name.startswith("_morphology_")
    }
    for name, declaration in functions.items():
        assert declaration.returns is not None, name
        assert not _annotation_mentions_any(declaration.returns), name
        runtime_signature = inspect.signature(getattr(native, name))
        assert _runtime_parameter_shape(runtime_signature) == _stub_parameter_shape(
            declaration, constructor=False
        ), name
        runtime_defaults = _runtime_parameter_defaults(runtime_signature)
        if runtime_defaults:
            assert runtime_defaults == MORPHOLOGY_CALLABLE_DEFAULTS[name]
            checked_default_callables.add(name)

    assert checked_default_callables == set(MORPHOLOGY_CALLABLE_DEFAULTS)


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
