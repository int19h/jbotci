from __future__ import annotations

import ast
import enum
import importlib
import importlib.metadata
import inspect
import pickle
import re
import subprocess
import sys
import tomllib
from collections import Counter
from pathlib import Path
from types import ModuleType
from typing import get_type_hints

import pytest

import jbotci
import jbotci._native as native
from jbotci import diagnostics, dialect, morphology, source
from tools.compose_stubs import FRAGMENTS, OUTPUT

PACKAGE_ROOT = Path(__file__).resolve().parents[1]
WORKSPACE_ROOT = PACKAGE_ROOT.parents[1]
_STANDALONE_ANY = re.compile(r"\bAny\b")

INTENTIONAL_STUB_ONLY_NATIVE_DECLARATIONS: frozenset[str] = frozenset(
    {
        "_MorphologyErrorValueBase",
        "_MorphologyWordBase",
        "_MorphologyWordLikeBase",
        "_ValsiClassificationBase",
    }
)

PYO3_EQ_COMPANION_DUNDERS: frozenset[str] = frozenset(
    {"__ne__", "__lt__", "__le__", "__gt__", "__ge__"}
)

MORPHOLOGY_MATCH_ARGS: dict[str, tuple[str, ...]] = {
    "_morphology_InvalidDialectWord": ("word",),
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

NON_MORPHOLOGY_MATCH_ARGS: dict[str, tuple[str, ...]] = {
    "_source_ZeroLine": (),
    "_source_ZeroColumn": (),
    "_source_ByteRangeInverted": ("start", "end"),
    "_source_CharRangeInverted": ("start", "end"),
    "_source_CharOffsetOutOfBounds": ("offset", "source_len"),
    "_source_ByteOffsetOutOfBounds": ("offset", "source_len"),
    "_source_ByteOffsetNotCharBoundary": ("offset",),
    "_source_SourceLocation": ("error",),
    "_diagnostics_VlackuWordLink": ("word",),
    "_diagnostics_CllSectionLink": ("section_id", "anchor"),
    "_diagnostics_EbnfRuleLink": ("rule_name",),
    "_diagnostics_InvalidTraceLevel": ("value",),
    "_dialect_CmavoSwap": ("left", "right"),
    "_dialect_CmavoExpansion": ("source", "replacement"),
}

_OPAQUE_PYO3_DEFAULT = object()

# These maps describe exact ``inspect.signature`` metadata. PyO3 renders
# nonliteral Rust default expressions as ``Ellipsis``; those opaque entries use
# a private sentinel here and require separate semantic witnesses. Literal and
# ``None`` defaults remain concrete.
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
        "parts": _OPAQUE_PYO3_DEFAULT,
        "stage": None,
    },
    "_morphology_ValsiAnalysisResult.__new__": {
        "word": None,
        "classification": None,
        "error": None,
        "words": _OPAQUE_PYO3_DEFAULT,
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

MANUAL_CALLABLE_DEFAULTS: dict[str, dict[str, object]] = {
    "_source_SourceSpan.__new__": {
        "source_id": None,
        "start": None,
        "end": None,
    },
    "_source_source_span_from_char_offsets": {"source_id": None},
    "_source_source_span_from_byte_offsets": {"source_id": None},
    "_diagnostics_TraceOptions.__new__": {
        "enabled": False,
        "level": None,
        "filter": None,
        "phase": None,
        "limit": _OPAQUE_PYO3_DEFAULT,
    },
    "_diagnostics_TraceEvent.__new__": {"detail": None},
    "_diagnostics_TraceFailureBranch.__new__": {
        "contexts": _OPAQUE_PYO3_DEFAULT,
        "expected": _OPAQUE_PYO3_DEFAULT,
    },
    "_diagnostics_TraceFailureSummary.__new__": {
        "branches": _OPAQUE_PYO3_DEFAULT,
        "current_context": None,
    },
    "_diagnostics_TraceReport.__new__": {
        "events": _OPAQUE_PYO3_DEFAULT,
        "truncated": False,
        "failure": None,
    },
    "_diagnostics_CllSectionLink.__new__": {"anchor": None},
    "_diagnostics_DiagnosticTextSegment.__new__": {"link": None},
    "_diagnostics_DiagnosticLabel.__new__": {"primary": False},
    "_diagnostics_Diagnostic.__new__": {
        "notes": _OPAQUE_PYO3_DEFAULT,
        "styled_notes": _OPAQUE_PYO3_DEFAULT,
        "word_index": None,
    },
    "_dialect_DialectDefinition.__new__": {
        "cmavo_entries": None,
        "features": None,
    },
    "_dialect_CustomDialect.__new__": {"show_in_gentufa": True},
    "_dialect_DialectSettings.__new__": {
        "custom_dialects": _OPAQUE_PYO3_DEFAULT,
        "hidden_builtin_gentufa_dialects": _OPAQUE_PYO3_DEFAULT,
    },
    **MORPHOLOGY_CALLABLE_DEFAULTS,
}

MANUAL_STUB_FRAGMENTS: dict[str, str] = {
    "_source_": "source.pyi",
    "_diagnostics_": "diagnostics.pyi",
    "_dialect_": "dialect.pyi",
    "_morphology_": "morphology.pyi",
}

PUBLIC_RUST_CONSTANTS: dict[str, dict[str, str]] = {
    "diagnostics.py": {
        "DEFAULT_TRACE_LIMIT": "_diagnostics_DEFAULT_TRACE_LIMIT",
    },
    "morphology.py": {
        "MORPHOLOGY_TRACE_FILTERS": "_morphology_MORPHOLOGY_TRACE_FILTERS",
        "PERMISSIVE_IGNORABLE_RESERVED_CHARACTERS": (
            "_morphology_PERMISSIVE_IGNORABLE_RESERVED_CHARACTERS"
        ),
    },
}

MORPHOLOGY_RETURNED_ONLY_CLASSES: frozenset[str] = frozenset(
    {
        "_morphology_CompiledDialectWord",
        "_morphology_CompiledDialectSwap",
        "_morphology_CompiledDialectExpansion",
        "_morphology_MorphologySegmentAttempt",
        "_morphology_RecoveredMorphologySegmentation",
        "_morphology_RecoveredMorphologySegmentAttempt",
        "_morphology_ValsiAnalysis",
    }
)

MANUAL_RETURNED_ONLY_CLASSES: frozenset[str] = (
    MORPHOLOGY_RETURNED_ONLY_CLASSES | {"_dialect_BuiltinDialect"}
)

# PyO3 currently exposes signatures for every manual morphology dunder. Keep an
# explicit empty allowlist so a future uninspectable slot requires a separately
# authored typing witness rather than silently losing signature coverage.
MORPHOLOGY_UNINSPECTABLE_DUNDERS: frozenset[str] = frozenset()

MORPHOLOGY_FUNCTION_TYPES: dict[
    str, tuple[tuple[tuple[str, str], ...], str]
] = {
    "_morphology_segment_attempt": (
        (
            ("source", "str"),
            ("options", "_morphology_MorphologyOptions | None"),
            ("source_id", "_source_SourceId | None"),
        ),
        "_morphology_MorphologySegmentAttempt",
    ),
    "_morphology_segment_recovered_attempt": (
        (
            ("source", "str"),
            ("options", "_morphology_MorphologyOptions | None"),
            ("source_id", "_source_SourceId | None"),
        ),
        "_morphology_RecoveredMorphologySegmentAttempt",
    ),
    "_morphology_segment_for_display_attempt": (
        (
            ("source", "str"),
            ("options", "_morphology_MorphologyOptions | None"),
            ("source_id", "_source_SourceId | None"),
        ),
        "_morphology_MorphologySegmentAttempt",
    ),
    "_morphology_analyze_valsi": (
        (
            ("source", "str"),
            ("options", "_morphology_MorphologyOptions | None"),
            ("source_id", "_source_SourceId | None"),
        ),
        "_morphology_ValsiAnalysis",
    ),
    "_morphology_normalize_input": (
        (("text", "str"), ("options", "_morphology_MorphologyOptions | None")),
        "str | None",
    ),
    "_morphology_canonicalize_text": ((("text", "str"),), "str"),
    "_morphology_canonical_text_eq": (
        (("left", "str"), ("right", "str")),
        "bool",
    ),
    "_morphology_canonical_text_is_all": (
        (("text", "str"), ("expected", "str")),
        "bool",
    ),
    "_morphology_normalize_cmavo_form": ((("text", "str"),), "str | None"),
    "_morphology_cmavo_phonemes": (
        (("text", "str"),),
        "_morphology_Phonemes | None",
    ),
    "_morphology_pronunciation_syllables": (
        (("phonemes", "_morphology_Phonemes"),),
        "tuple[str, ...]",
    ),
    "_morphology_strip_lojban_diacritic": ((("value", "str"),), "str | None"),
    "_morphology_fold_lojban_diacritic": ((("value", "str"),), "str | None"),
    "_morphology_strip_lojban_diacritics": ((("text", "str"),), "str"),
    "_morphology_fold_lojban_diacritics": ((("text", "str"),), "str"),
    "_morphology_stripped_lojban_diacritics_eq": (
        (("left", "str"), ("right", "str")),
        "bool",
    ),
    "_morphology_folded_lojban_diacritics_eq": (
        (("left", "str"), ("right", "str")),
        "bool",
    ),
    "_morphology_strip_diacritics": ((("text", "str"),), "str"),
    "_morphology_strip_diacritics_eq": (
        (("left", "str"), ("right", "str")),
        "bool",
    ),
    "_morphology_is_valid_phoneme": ((("value", "str"),), "bool"),
    "_morphology_is_word_forming_character": (
        (("value", "str"), ("options", "_morphology_MorphologyOptions | None")),
        "bool",
    ),
    "_morphology_is_period_character": ((("value", "str"),), "bool"),
    "_morphology_is_permissive_ignorable_character": (
        (("value", "str"),),
        "bool",
    ),
    "_morphology_parse_lujvo_parts": (
        (("word", "str"),),
        "tuple[_morphology_LujvoRafsi | _morphology_LujvoHyphen, ...] | None",
    ),
    "_morphology_parse_cmevla_lujvo_parts": (
        (("word", "str"),),
        "tuple[_morphology_LujvoRafsi | _morphology_LujvoHyphen, ...] | None",
    ),
    "_morphology_parse_cmevla_lujvo_part_candidates": (
        (("word", "str"),),
        "tuple[tuple[_morphology_LujvoRafsi | _morphology_LujvoHyphen, ...], ...]",
    ),
    "_morphology_bond_rafsis": (
        (("rafsis", "Sequence[str]"),),
        "tuple[str, ...] | None",
    ),
    "_morphology_is_valid_lujvo_candidate_word": (
        (("word", "str"),),
        "bool",
    ),
    "_morphology_ensure_cmevla_word": ((("word", "str"),), "str"),
    "_morphology_ends_with_consonant": ((("word", "str"),), "bool"),
    "_morphology_ends_with_vowel": ((("word", "str"),), "bool"),
    "_morphology_is_bonding_hyphen": ((("part", "str"),), "bool"),
    "_morphology_syllables_pattern": ((("text", "str"),), "str | None"),
    "_morphology_rafsi_shape": (
        (("text", "str"),),
        "_morphology_RafsiShape",
    ),
    "_morphology_rafsi_shape_score": (
        (("shape", "_morphology_RafsiShape"),),
        "int",
    ),
    "_morphology_is_vowel": ((("value", "str"),), "bool"),
    "_morphology_is_consonant": ((("value", "str"),), "bool"),
    "_morphology_is_cmevla": ((("text", "str"),), "bool"),
    "_morphology_consonant_pair_class": (
        (("first", "str"), ("second", "str")),
        "_morphology_ConsonantPairClass | None",
    ),
    "_morphology_permissible_consonant_pair": (
        (("first", "str"), ("second", "str")),
        "bool",
    ),
    "_morphology_consonant_pair_is_permissible": (
        (("value", "_morphology_ConsonantPairClass"),),
        "bool",
    ),
    "_morphology_consonant_pair_is_initial": (
        (("value", "_morphology_ConsonantPairClass"),),
        "bool",
    ),
    "_morphology_word_needs_leading_pause": (
        (
            ("word", "_MorphologyWord"),
            ("mode", "_morphology_LeadingPauseVowelMode"),
        ),
        "bool",
    ),
    "_morphology_word_needs_leading_pause_in_context": (
        (
            ("word", "_MorphologyWord"),
            ("mode", "_morphology_LeadingPauseVowelMode"),
            ("context", "_morphology_LeadingPauseContext"),
        ),
        "bool",
    ),
    "_morphology_word_syntax_eq": (
        (("left", "_MorphologyWord"), ("right", "_MorphologyWord")),
        "bool",
    ),
    "_morphology_word_like_syntax_eq": (
        (("left", "_MorphologyWordLike"), ("right", "_MorphologyWordLike")),
        "bool",
    ),
    "_morphology_cmavo_from_text": (
        (("text", "str"),),
        "_morphology_Cmavo | None",
    ),
    "_morphology_cmavo_text": ((("cmavo", "_morphology_Cmavo"),), "str"),
    "_morphology_cmavo_is_selmaho": (
        (
            ("cmavo", "_morphology_Cmavo"),
            ("selmaho", "_morphology_Selmaho"),
        ),
        "bool",
    ),
    "_morphology_cmavo_primary_selmaho": (
        (("cmavo", "_morphology_Cmavo"),),
        "_morphology_Selmaho | None",
    ),
    "_morphology_cmavo_is_quote_opener": (
        (("cmavo", "_morphology_Cmavo"),),
        "bool",
    ),
    "_morphology_cmavo_is_single_word_quote_opener": (
        (("cmavo", "_morphology_Cmavo"),),
        "bool",
    ),
    "_morphology_cmavo_is_delimited_non_lojban_quote_opener": (
        (("cmavo", "_morphology_Cmavo"),),
        "bool",
    ),
    "_morphology_selmaho_from_name": (
        (("name", "str"),),
        "_morphology_Selmaho | None",
    ),
    "_morphology_selmaho_name": (
        (("selmaho", "_morphology_Selmaho"),),
        "str",
    ),
    "_morphology_selmaho_contains": (
        (
            ("selmaho", "_morphology_Selmaho"),
            ("cmavo", "_morphology_Cmavo"),
        ),
        "bool",
    ),
    "_morphology_choose_best_lujvo_candidate": (
        (
            ("mode", "_morphology_LujvoBuildMode"),
            ("choices", "Sequence[Sequence[str]]"),
        ),
        "_morphology_LujvoCandidate | None",
    ),
    "_morphology_choose_best_lujvo_candidate_from_parts": (
        (
            ("mode", "_morphology_LujvoBuildMode"),
            (
                "choices",
                "Sequence[Sequence[_morphology_LujvoRafsiBuildPart | "
                "_morphology_LujvoBrivlaCoreBuildPart]]",
            ),
        ),
        "_morphology_LujvoCandidate | None",
    ),
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
    assert declaration_names == (
        set(native.__all__) | INTENTIONAL_STUB_ONLY_NATIVE_DECLARATIONS
    )
    assert all(hasattr(native, name) for name in native.__all__)
    assert all(
        not hasattr(native, name)
        for name in INTENTIONAL_STUB_ONLY_NATIVE_DECLARATIONS
    )


def test_public_constants_are_direct_rust_native_exports() -> None:
    """Prevent consumer constants from acquiring drift-prone Python values."""

    for filename, expected in PUBLIC_RUST_CONSTANTS.items():
        facade_path = PACKAGE_ROOT / "python" / "jbotci" / filename
        tree = ast.parse(
            facade_path.read_text(encoding="utf-8"), filename=str(facade_path)
        )
        declarations = {
            statement.target.id: statement
            for statement in tree.body
            if isinstance(statement, ast.AnnAssign)
            and isinstance(statement.target, ast.Name)
            and statement.target.id in expected
        }
        assert set(declarations) == set(expected), filename
        for public_name, native_name in expected.items():
            declaration = declarations[public_name]
            assert isinstance(declaration.annotation, ast.Subscript)
            assert isinstance(declaration.annotation.value, ast.Name)
            assert declaration.annotation.value.id == "Final"
            assert isinstance(declaration.value, ast.Attribute)
            assert isinstance(declaration.value.value, ast.Name)
            assert declaration.value.value.id == "_rust"
            assert declaration.value.attr == native_name


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


def _is_dunder(name: str) -> bool:
    return name.startswith("__") and name.endswith("__")


def _stub_function_type_shape(
    declaration: ast.FunctionDef,
) -> tuple[tuple[tuple[str, str], ...], str]:
    arguments = [
        *declaration.args.posonlyargs,
        *declaration.args.args,
        *declaration.args.kwonlyargs,
    ]
    if declaration.args.vararg is not None:
        arguments.append(declaration.args.vararg)
    if declaration.args.kwarg is not None:
        arguments.append(declaration.args.kwarg)
    parameter_types = tuple(
        (argument.arg, ast.unparse(argument.annotation))
        for argument in arguments
        if argument.annotation is not None
    )
    assert declaration.returns is not None
    return (parameter_types, ast.unparse(declaration.returns))


def _stub_is_returned_only_constructor(
    declaration: ast.FunctionDef, class_name: str
) -> bool:
    arguments = declaration.args
    if (
        declaration.name != "__new__"
        or arguments.posonlyargs
        or [argument.arg for argument in arguments.args]
        != ["cls", "_nonconstructible"]
        or arguments.vararg is not None
        or arguments.kwonlyargs
        or arguments.kwarg is not None
        or arguments.defaults
    ):
        return False
    sentinel = arguments.args[1].annotation
    return (
        isinstance(sentinel, ast.Name)
        and sentinel.id == "Never"
        and isinstance(declaration.returns, ast.Name)
        and declaration.returns.id == class_name
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


def _assert_runtime_parameter_defaults(
    actual: dict[str, object],
    expected: dict[str, object],
    *,
    callable_name: str,
) -> None:
    assert actual.keys() == expected.keys(), callable_name
    for parameter_name, expected_default in expected.items():
        actual_default = actual[parameter_name]
        if expected_default is _OPAQUE_PYO3_DEFAULT:
            assert actual_default is Ellipsis, (callable_name, parameter_name)
        else:
            assert actual_default == expected_default, (
                callable_name,
                parameter_name,
            )


def _generated_enum_runtime_type_name(annotation: object) -> str:
    if annotation is bool:
        return "bool"
    if annotation is int:
        return "int"
    if annotation is str:
        return "str"
    native_names = [
        name for name in native.__all__ if getattr(native, name) is annotation
    ]
    assert len(native_names) == 1, annotation
    return native_names[0]


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


def test_native_stub_sources_and_output_do_not_use_any() -> None:
    for stub_path in (*FRAGMENTS, OUTPUT):
        source_text = stub_path.read_text(encoding="utf-8")
        assert _STANDALONE_ANY.search(source_text) is None, stub_path.relative_to(
            PACKAGE_ROOT
        )


def test_opaque_pyo3_introspection_defaults_have_semantic_witnesses() -> None:
    witnessed: set[tuple[str, str]] = set()

    trace_options = diagnostics.TraceOptions()
    assert trace_options.limit == diagnostics.DEFAULT_TRACE_LIMIT == 10_000
    assert (
        diagnostics.TraceOptions(limit=diagnostics.DEFAULT_TRACE_LIMIT).limit
        == trace_options.limit
    )
    witnessed.add(("_diagnostics_TraceOptions.__new__", "limit"))

    failure_branch = diagnostics.TraceFailureBranch()
    explicit_failure_branch = diagnostics.TraceFailureBranch([], [])
    assert failure_branch.contexts == explicit_failure_branch.contexts == ()
    assert failure_branch.expected == explicit_failure_branch.expected == ()
    witnessed.update(
        {
            ("_diagnostics_TraceFailureBranch.__new__", "contexts"),
            ("_diagnostics_TraceFailureBranch.__new__", "expected"),
        }
    )

    failure_summary = diagnostics.TraceFailureSummary(0, 0, "expected word")
    explicit_failure_summary = diagnostics.TraceFailureSummary(
        0, 0, "expected word", []
    )
    assert failure_summary.branches == explicit_failure_summary.branches == ()
    witnessed.add(("_diagnostics_TraceFailureSummary.__new__", "branches"))

    trace_report = diagnostics.TraceReport(diagnostics.TracePhase.MORPHOLOGY)
    explicit_trace_report = diagnostics.TraceReport(
        diagnostics.TracePhase.MORPHOLOGY, []
    )
    assert trace_report.events == explicit_trace_report.events == ()
    witnessed.add(("_diagnostics_TraceReport.__new__", "events"))

    span = source.source_span_from_char_offsets("mi", 0, 2)
    label = diagnostics.DiagnosticLabel(span, "input", primary=True)
    diagnostic = diagnostics.Diagnostic(
        diagnostics.DiagnosticSeverity.ERROR,
        diagnostics.DiagnosticPhase.MORPHOLOGY,
        "test.default",
        "default collections",
        [label],
    )
    explicit_diagnostic = diagnostics.Diagnostic(
        diagnostics.DiagnosticSeverity.ERROR,
        diagnostics.DiagnosticPhase.MORPHOLOGY,
        "test.default",
        "default collections",
        [label],
        [],
        styled_notes=[],
    )
    assert diagnostic.notes == explicit_diagnostic.notes == ()
    assert diagnostic.styled_notes == explicit_diagnostic.styled_notes == ()
    witnessed.update(
        {
            ("_diagnostics_Diagnostic.__new__", "notes"),
            ("_diagnostics_Diagnostic.__new__", "styled_notes"),
        }
    )

    dialect_settings = dialect.DialectSettings()
    explicit_dialect_settings = dialect.DialectSettings([], [])
    assert (
        dialect_settings.custom_dialects
        == explicit_dialect_settings.custom_dialects
        == ()
    )
    assert (
        dialect_settings.hidden_builtin_gentufa_dialects
        == explicit_dialect_settings.hidden_builtin_gentufa_dialects
        == ()
    )
    witnessed.update(
        {
            ("_dialect_DialectSettings.__new__", "custom_dialects"),
            (
                "_dialect_DialectSettings.__new__",
                "hidden_builtin_gentufa_dialects",
            ),
        }
    )

    classification = morphology.PlainWordClassification(
        morphology.WordKind.GISMU, "broda"
    )
    explicit_classification = morphology.PlainWordClassification(
        morphology.WordKind.GISMU, "broda", parts=[]
    )
    assert classification.parts == explicit_classification.parts == ()
    witnessed.add(("_morphology_PlainWordClassification.__new__", "parts"))

    analysis_result = morphology.ValsiAnalysisResult(
        morphology.ValsiAnalysisStatus.NOT_SINGLE_WORD
    )
    explicit_analysis_result = morphology.ValsiAnalysisResult(
        morphology.ValsiAnalysisStatus.NOT_SINGLE_WORD, words=[]
    )
    assert analysis_result.words == explicit_analysis_result.words == ()
    witnessed.add(("_morphology_ValsiAnalysisResult.__new__", "words"))

    expected_witnesses = {
        (callable_name, parameter_name)
        for default_map in (MANUAL_CALLABLE_DEFAULTS, MORPHOLOGY_CALLABLE_DEFAULTS)
        for callable_name, defaults in default_map.items()
        for parameter_name, expected_default in defaults.items()
        if expected_default is _OPAQUE_PYO3_DEFAULT
    }
    assert witnessed == expected_witnesses


def test_manual_domain_stub_callable_surfaces_match_runtime() -> None:
    """Check every callable in the four hand-authored domain fragments."""

    generated_enum_names = set(
        _stub_classes(PACKAGE_ROOT / "stubs" / "_native" / "domain_enums.pyi")
    )
    checked_default_callables: set[str] = set()
    returned_only_stubs: set[str] = set()
    for prefix, filename in MANUAL_STUB_FRAGMENTS.items():
        stub_path = PACKAGE_ROOT / "stubs" / "_native" / filename
        stub_tree = ast.parse(
            stub_path.read_text(encoding="utf-8"), filename=str(stub_path)
        )
        classes = _stub_classes(stub_path)
        declarations = {
            name: declaration
            for name, declaration in classes.items()
            if name.startswith(prefix)
        }
        runtime_class_names = {
            name
            for name in native.__all__
            if name.startswith(prefix)
            and name not in generated_enum_names
            and isinstance(getattr(native, name), type)
        }
        assert set(declarations) == runtime_class_names, filename

        functions = {
            declaration.name: declaration
            for declaration in stub_tree.body
            if isinstance(declaration, ast.FunctionDef)
            and declaration.name.startswith(prefix)
        }
        runtime_function_names = {
            name
            for name in native.__all__
            if name.startswith(prefix)
            and not isinstance(getattr(native, name), type)
            and callable(getattr(native, name))
        }
        assert set(functions) == runtime_function_names, filename

        for name, declaration in declarations.items():
            runtime_class = getattr(native, name)
            class_functions = _stub_class_functions(classes, declaration)
            class_attributes = _stub_class_attributes(classes, declaration)
            stub_properties = {
                member_name
                for member_name, function in class_functions.items()
                if not member_name.startswith("_")
                and _stub_function_is_property(function)
            }
            stub_methods = {
                member_name
                for member_name, function in class_functions.items()
                if not member_name.startswith("_")
                and not _stub_function_is_property(function)
            }
            stub_attributes = {
                member_name
                for member_name in class_attributes
                if not member_name.startswith("_")
            }
            stub_dunders = {
                member_name
                for member_name in class_functions
                if _is_dunder(member_name)
            }

            runtime_properties: set[str] = set()
            runtime_methods: set[str] = set()
            runtime_attributes: set[str] = set()
            for member_name in dir(runtime_class):
                if member_name.startswith("_"):
                    continue
                descriptor = inspect.getattr_static(runtime_class, member_name)
                if inspect.isdatadescriptor(descriptor):
                    runtime_properties.add(member_name)
                elif callable(descriptor):
                    runtime_methods.add(member_name)
                else:
                    runtime_attributes.add(member_name)
            runtime_dunders = {
                member_name
                for member_name, descriptor in vars(runtime_class).items()
                if _is_dunder(member_name) and callable(descriptor)
            }
            returned_only_constructor = next(
                (
                    function
                    for member_name, function in class_functions.items()
                    if member_name == "__new__"
                    and _stub_is_returned_only_constructor(function, name)
                ),
                None,
            )
            expected_runtime_dunders = stub_dunders.copy()
            if returned_only_constructor is not None:
                expected_runtime_dunders.remove("__new__")
            if "__eq__" in stub_dunders:
                expected_runtime_dunders.update(PYO3_EQ_COMPANION_DUNDERS)
            assert runtime_properties == stub_properties, name
            assert runtime_methods == stub_methods, name
            assert runtime_attributes == stub_attributes, name
            assert runtime_dunders == expected_runtime_dunders, name

            for function_name, function in class_functions.items():
                if _stub_function_is_property(function):
                    continue
                if _stub_is_returned_only_constructor(function, name):
                    assert name in MANUAL_RETURNED_ONLY_CLASSES
                    returned_only_stubs.add(name)
                    continue
                if function_name == "__new__":
                    constructor = True
                    callable_value = runtime_class
                else:
                    constructor = False
                    callable_value = getattr(runtime_class, function_name)
                callable_name = f"{name}.{function_name}"
                runtime_signature = inspect.signature(callable_value)
                assert _runtime_parameter_shape(runtime_signature) == (
                    _stub_parameter_shape(function, constructor=constructor)
                ), callable_name
                runtime_defaults = _runtime_parameter_defaults(runtime_signature)
                if runtime_defaults:
                    _assert_runtime_parameter_defaults(
                        runtime_defaults,
                        MANUAL_CALLABLE_DEFAULTS[callable_name],
                        callable_name=callable_name,
                    )
                    checked_default_callables.add(callable_name)

        for name, declaration in functions.items():
            runtime_signature = inspect.signature(getattr(native, name))
            assert _runtime_parameter_shape(runtime_signature) == (
                _stub_parameter_shape(declaration, constructor=False)
            ), name
            runtime_defaults = _runtime_parameter_defaults(runtime_signature)
            if runtime_defaults:
                _assert_runtime_parameter_defaults(
                    runtime_defaults,
                    MANUAL_CALLABLE_DEFAULTS[name],
                    callable_name=name,
                )
                checked_default_callables.add(name)

    assert checked_default_callables == set(MANUAL_CALLABLE_DEFAULTS)
    assert returned_only_stubs == MANUAL_RETURNED_ONLY_CLASSES


def test_non_morphology_match_args_match_exact_runtime_and_stub_shape() -> None:
    classes: dict[str, ast.ClassDef] = {}
    for prefix, filename in MANUAL_STUB_FRAGMENTS.items():
        if prefix == "_morphology_":
            continue
        fragment_classes = _stub_classes(
            PACKAGE_ROOT / "stubs" / "_native" / filename
        )
        classes.update(
            (name, declaration)
            for name, declaration in fragment_classes.items()
            if name.startswith(prefix)
        )

    declared_match_args = {
        name
        for name, declaration in classes.items()
        if any(
            isinstance(statement, ast.AnnAssign)
            and isinstance(statement.target, ast.Name)
            and statement.target.id == "__match_args__"
            for statement in declaration.body
        )
    }
    runtime_match_args = {
        name for name in classes if hasattr(getattr(native, name), "__match_args__")
    }
    assert (
        runtime_match_args
        == declared_match_args
        == set(NON_MORPHOLOGY_MATCH_ARGS)
    )
    for name, expected in NON_MORPHOLOGY_MATCH_ARGS.items():
        declaration = classes[name]
        annotation = next(
            statement.annotation
            for statement in declaration.body
            if isinstance(statement, ast.AnnAssign)
            and isinstance(statement.target, ast.Name)
            and statement.target.id == "__match_args__"
        )
        assert _stub_match_args_arity(annotation) == len(expected), name
        assert getattr(native, name).__match_args__ == expected, name


def test_morphology_stub_class_members_signatures_and_match_args_match_runtime() -> None:
    """Check the complete manual morphology stub surface, not only exports."""

    stub_path = PACKAGE_ROOT / "stubs" / "_native" / "morphology.pyi"
    classes = _stub_classes(stub_path)
    declarations = {
        name: declaration
        for name, declaration in classes.items()
        if name.startswith("_morphology_")
    }
    assert len(declarations) == 53
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
    returned_only_stubs = {
        name
        for name, declaration in declarations.items()
        if any(
            isinstance(statement, ast.FunctionDef)
            and _stub_is_returned_only_constructor(statement, name)
            for statement in declaration.body
        )
    }
    assert returned_only_stubs == MORPHOLOGY_RETURNED_ONLY_CLASSES
    checked_default_callables: set[str] = set()
    uninspectable_dunders: set[str] = set()
    for name, declaration in declarations.items():
        runtime_class = getattr(native, name)
        functions = _stub_class_functions(classes, declaration)
        attributes = _stub_class_attributes(classes, declaration)
        stub_properties = {
            function_name
            for function_name, function in functions.items()
            if not function_name.startswith("_")
            and _stub_function_is_property(function)
        }
        stub_methods = {
            function_name
            for function_name, function in functions.items()
            if not function_name.startswith("_")
            and not _stub_function_is_property(function)
        }
        stub_attributes = {
            attribute_name
            for attribute_name in attributes
            if not attribute_name.startswith("_")
        }
        assert not (stub_properties & stub_methods)
        assert not (stub_properties & stub_attributes)
        assert not (stub_methods & stub_attributes)

        runtime_properties: set[str] = set()
        runtime_methods: set[str] = set()
        runtime_attributes: set[str] = set()
        for member_name in dir(runtime_class):
            if member_name.startswith("_"):
                continue
            descriptor = inspect.getattr_static(runtime_class, member_name)
            if inspect.isdatadescriptor(descriptor):
                runtime_properties.add(member_name)
            elif callable(descriptor):
                runtime_methods.add(member_name)
            else:
                runtime_attributes.add(member_name)
        assert runtime_properties == stub_properties, name
        assert runtime_methods == stub_methods, name
        assert runtime_attributes == stub_attributes, name

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
            if _stub_function_is_property(function):
                continue
            if _stub_is_returned_only_constructor(function, name):
                continue
            if function_name == "__new__":
                constructor = True
                callable_value = runtime_class
            else:
                constructor = False
                callable_value = getattr(runtime_class, function_name)
            callable_name = f"{name}.{function_name}"
            try:
                runtime_signature = inspect.signature(callable_value)
            except (TypeError, ValueError):
                assert callable_name in MORPHOLOGY_UNINSPECTABLE_DUNDERS
                uninspectable_dunders.add(callable_name)
                continue
            assert _runtime_parameter_shape(runtime_signature) == (
                _stub_parameter_shape(function, constructor=constructor)
            ), callable_name
            runtime_defaults = _runtime_parameter_defaults(runtime_signature)
            if runtime_defaults:
                _assert_runtime_parameter_defaults(
                    runtime_defaults,
                    MORPHOLOGY_CALLABLE_DEFAULTS[callable_name],
                    callable_name=callable_name,
                )
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
    assert len(functions) == 58
    assert set(functions) == set(MORPHOLOGY_FUNCTION_TYPES)
    for name, declaration in functions.items():
        assert _stub_function_type_shape(declaration) == MORPHOLOGY_FUNCTION_TYPES[
            name
        ], name
        runtime_signature = inspect.signature(getattr(native, name))
        assert _runtime_parameter_shape(runtime_signature) == _stub_parameter_shape(
            declaration, constructor=False
        ), name
        runtime_defaults = _runtime_parameter_defaults(runtime_signature)
        if runtime_defaults:
            _assert_runtime_parameter_defaults(
                runtime_defaults,
                MORPHOLOGY_CALLABLE_DEFAULTS[name],
                callable_name=name,
            )
            checked_default_callables.add(name)

    assert checked_default_callables == set(MORPHOLOGY_CALLABLE_DEFAULTS)
    assert uninspectable_dunders == MORPHOLOGY_UNINSPECTABLE_DUNDERS


def test_returned_only_morphology_classes_have_no_runtime_constructor() -> None:
    for name in MORPHOLOGY_RETURNED_ONLY_CLASSES:
        with pytest.raises(TypeError):
            getattr(native, name)()


def test_strict_typing_fixture_witnesses_every_public_morphology_function() -> None:
    typecheck_path = PACKAGE_ROOT / "tests" / "typecheck.py"
    tree = ast.parse(
        typecheck_path.read_text(encoding="utf-8"), filename=str(typecheck_path)
    )
    witness = next(
        declaration
        for declaration in tree.body
        if isinstance(declaration, ast.FunctionDef)
        and declaration.name == "typed_every_morphology_free_function"
    )
    witnessed_functions: Counter[str] = Counter()
    for assertion in ast.walk(witness):
        if (
            not isinstance(assertion, ast.Call)
            or not isinstance(assertion.func, ast.Name)
            or assertion.func.id != "assert_type"
            or not assertion.args
        ):
            continue
        value = assertion.args[0]
        if (
            isinstance(value, ast.Call)
            and isinstance(value.func, ast.Attribute)
            and isinstance(value.func.value, ast.Name)
            and value.func.value.id == "morphology"
        ):
            witnessed_functions[value.func.attr] += 1

    public_functions = {
        name
        for name in morphology.__all__
        if callable(value := getattr(morphology, name)) and not isinstance(value, type)
    }
    assert set(witnessed_functions) == public_functions
    assert set(witnessed_functions.values()) == {1}


def test_generated_domain_enum_members_match_runtime_rust_metadata() -> None:
    """Catch generated enum member, method, value, or signature drift."""

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

        stub_methods = {
            statement.name: statement
            for statement in declaration.body
            if isinstance(statement, ast.FunctionDef)
            and not statement.name.startswith("_")
        }
        runtime_methods = {
            name: value
            for name, value in vars(runtime_enum).items()
            if not name.startswith("_") and callable(value)
        }
        assert set(runtime_methods) == set(stub_methods), declaration.name
        for method_name, stub_method in stub_methods.items():
            qualified_name = f"{declaration.name}.{method_name}"
            runtime_method = runtime_methods[method_name]
            stub_is_static = any(
                isinstance(decorator, ast.Name) and decorator.id == "staticmethod"
                for decorator in stub_method.decorator_list
            )
            assert isinstance(runtime_method, staticmethod) is stub_is_static
            runtime_signature = inspect.signature(runtime_method)
            assert (
                _runtime_parameter_shape(runtime_signature)
                == _stub_parameter_shape(stub_method, constructor=False)
            ), qualified_name
            assert _runtime_parameter_defaults(runtime_signature) == {}, qualified_name

            runtime_function = (
                runtime_method.__func__
                if isinstance(runtime_method, staticmethod)
                else runtime_method
            )
            runtime_hints = get_type_hints(runtime_function)
            stub_parameter_types, stub_return_type = _stub_function_type_shape(
                stub_method
            )
            stub_types = dict(stub_parameter_types)
            stub_types["return"] = stub_return_type
            assert set(stub_types) <= set(runtime_hints), qualified_name
            assert set(runtime_hints) - set(stub_types) <= {"self"}, qualified_name
            normalized_runtime_hints = {
                name: _generated_enum_runtime_type_name(runtime_hints[name])
                for name in stub_types
            }
            assert normalized_runtime_hints == stub_types, qualified_name
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
