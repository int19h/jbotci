#!/usr/bin/env python3
"""Generate the exhaustive Rust-to-Python API parity matrix."""

from __future__ import annotations

import argparse
import importlib
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

PACKAGE_ROOT = Path(__file__).resolve().parents[1]
WORKSPACE_ROOT = PACKAGE_ROOT.parents[1]
OUTPUT = PACKAGE_ROOT / "docs" / "api-parity.tsv"
HEADER = (
    "rust_path\tkind\trust_signature\tdisposition\tpython_path"
    "\texclusion_kind\trationale"
)
PUBLIC_MODULES = (
    "jbotci.semantics.references",
    "jbotci.syntax.recovered",
    "jbotci.syntax.strict",
    "jbotci.dictionary",
    "jbotci.morphology",
    "jbotci.diagnostics",
    "jbotci.jvozba",
    "jbotci.dialect",
    "jbotci.source",
    "jbotci.syntax",
    "jbotci",
)

PRIVATE_MORPHOLOGY_MODULES = frozenset(
    {"cmavo", "diacritics", "dialect", "lujvo", "surface", "syntax_eq", "tree"}
)

RUST_ONLY_CONCEPTS: dict[tuple[str, str], tuple[str, str]] = {
    ("jbotci_source", "Spanned"): (
        "generic-trait",
        "Generic Rust span/value carrier; each concrete Python result exposes its typed value and SourceSpan directly.",
    ),
    ("jbotci_diagnostics", "TraceRecorder"): (
        "implementation-representation",
        "Mutable parser instrumentation engine; Python receives its immutable TraceReport product instead.",
    ),
    ("jbotci_diagnostics", "TraceRecorderState"): (
        "implementation-representation",
        "Internal mutable storage for TraceRecorder; Python receives immutable trace events and reports.",
    ),
    ("jbotci_morphology", "StringEnumMetadata"): (
        "implementation-representation",
        "Compile-time metadata used to register exact Python StrEnum classes, not a consumer domain value.",
    ),
    ("jbotci_syntax", "RecoveryReachabilityKindTelemetry"): (
        "implementation-representation",
        "Test-harness counters for cross-checking internal recovery reachability branches under expensive contracts; they are not parser output or consumer configuration.",
    ),
    ("jbotci_syntax", "RecoveryReachabilityTelemetry"): (
        "implementation-representation",
        "Test-harness aggregate for comparing internal recovery reachability behavior under expensive contracts; it is not parser output or a consumer domain value.",
    ),
    ("jbotci_dictionary", "places"): (
        "implementation-representation",
        "Definition place-marker parsing and line segmentation are Rust-side definition-text rendering machinery shared by the output and semantics crates (moved from jbotci-output, which is outside the Python API scope); Python dictionary results carry the raw definition and notes text and no line-formatting internals.",
    ),
    ("jbotci_syntax", "with_recovery_reachability_instrumentation"): (
        "implementation-representation",
        "Feature-gated test-harness entry point that toggles an internal recovery filter and captures cross-check counters; it is not a consumer parser operation.",
    ),
}

PYTHON_CONCEPT_ALIASES: dict[tuple[str, str], str] = {
    ("jbotci_diagnostics", "DiagnosticSpanError"): "jbotci.source.DiagnosticSpanError",
    ("jbotci_dialect", "DialectError"): "jbotci.InvalidInputError",
    ("jbotci_dictionary", "DictionaryValidationError"): (
        "jbotci.dictionary.DictionaryValidationError"
    ),
    ("jbotci_jvozba", "JvozbaError"): "jbotci.jvozba.JvozbaErrorValue",
    ("jbotci_morphology", "LujvoFragmentError"): "jbotci.InvalidInputError",
    ("jbotci_morphology", "MorphologyError"): "jbotci.morphology.MorphologyErrorValue",
    ("jbotci_semantics", "GeneratedReferenceAnalysis"): (
        "jbotci.semantics.references.ReferenceAnalysis"
    ),
    ("jbotci_semantics", "ReferenceAnalysisError"): (
        "jbotci.semantics.references.ReferenceAnalysisErrorValue"
    ),
    ("jbotci_semantics", "SyntaxSpanKey"): (
        "jbotci.semantics.references.FixtureSpanKey"
    ),
    ("jbotci_syntax", "SyntaxError"): "jbotci.syntax.SyntaxErrorValue",
}

FUNCTION_ALIASES: dict[tuple[str, str], str] = {
    ("jbotci_diagnostics", "byte_offset_for_char_offset"): (
        "jbotci.source.byte_offset_for_char_offset"
    ),
    ("jbotci_diagnostics", "char_offset_for_byte_offset"): (
        "jbotci.source.char_offset_for_byte_offset"
    ),
    ("jbotci_diagnostics", "line_column_for_byte_offset"): (
        "jbotci.source.line_column_for_byte_offset"
    ),
    ("jbotci_diagnostics", "source_span_from_byte_offsets"): (
        "jbotci.source.source_span_from_byte_offsets"
    ),
    ("jbotci_diagnostics", "source_span_from_char_offsets"): (
        "jbotci.source.source_span_from_char_offsets"
    ),
    ("jbotci_diagnostics", "source_text_for_span"): (
        "jbotci.source.source_text_for_span"
    ),
    ("jbotci_morphology", "analyze_valsi_with_options"): (
        "jbotci.morphology.analyze_valsi"
    ),
    ("jbotci_morphology", "is_word_forming_character_with_options"): (
        "jbotci.morphology.is_word_forming_character"
    ),
    ("jbotci_morphology", "analyze_valsi_with_options_and_source_id"): (
        "jbotci.morphology.analyze_valsi"
    ),
    ("jbotci_morphology", "normalize_lojban_input_text"): (
        "jbotci.morphology.normalize_input"
    ),
    ("jbotci_morphology", "normalize_lojban_input_text_with_options"): (
        "jbotci.morphology.normalize_input"
    ),
    ("jbotci_morphology", "parse_cmevla_lujvo_word_part_candidates"): (
        "jbotci.morphology.parse_cmevla_lujvo_part_candidates"
    ),
    ("jbotci_morphology", "parse_cmevla_lujvo_word_parts"): (
        "jbotci.morphology.parse_cmevla_lujvo_parts"
    ),
    ("jbotci_morphology", "parse_lujvo_word_parts"): (
        "jbotci.morphology.parse_lujvo_parts"
    ),
    ("jbotci_morphology", "segment_words_for_display"): (
        "jbotci.morphology.segment_for_display"
    ),
    ("jbotci_morphology", "segment_words_for_display_with_options_and_source_id"): (
        "jbotci.morphology.segment_for_display"
    ),
    (
        "jbotci_morphology",
        "segment_words_for_display_with_options_and_source_id_attempt",
    ): "jbotci.morphology.segment_for_display_attempt",
    ("jbotci_morphology", "segment_words_with_modifiers"): (
        "jbotci.morphology.segment"
    ),
    ("jbotci_morphology", "segment_words_with_modifiers_recovered"): (
        "jbotci.morphology.segment_recovered"
    ),
    (
        "jbotci_morphology",
        "segment_words_with_modifiers_recovered_with_options",
    ): "jbotci.morphology.segment_recovered",
    (
        "jbotci_morphology",
        "segment_words_with_modifiers_recovered_with_options_and_source_id",
    ): "jbotci.morphology.segment_recovered",
    (
        "jbotci_morphology",
        "segment_words_with_modifiers_recovered_with_options_and_source_id_attempt",
    ): "jbotci.morphology.segment_recovered_attempt",
    (
        "jbotci_morphology",
        "segment_words_with_modifiers_with_options_and_source_id",
    ): "jbotci.morphology.segment",
    (
        "jbotci_morphology",
        "segment_words_with_modifiers_with_options_and_source_id_attempt",
    ): "jbotci.morphology.segment_attempt",
    ("jbotci_semantics", "analyze_generated_references"): (
        "jbotci.semantics.references.analyze_references"
    ),
    ("jbotci_syntax", "parse_syntax_tokens_with_recovery_with_source_and_options_attempt"): (
        "jbotci.syntax.parse_syntax_tree_with_recovery_attempt"
    ),
    ("jbotci_syntax", "parse_syntax_tree_generated_model_with_source_and_options"): (
        "jbotci.syntax.parse_syntax_tree"
    ),
    (
        "jbotci_syntax",
        "parse_syntax_tree_generated_model_with_source_and_options_attempt",
    ): "jbotci.syntax.parse_syntax_tree_attempt",
    ("jbotci_syntax", "parse_syntax_tree_recovered_with_source_and_options"): (
        "jbotci.syntax.parse_syntax_tree_recovered"
    ),
    (
        "jbotci_syntax",
        "parse_syntax_tree_recovered_with_source_and_options_attempt",
    ): "jbotci.syntax.parse_syntax_tree_recovered_attempt",
    ("jbotci_syntax", "parse_syntax_tree_with_options"): (
        "jbotci.syntax.parse_syntax_tree"
    ),
    (
        "jbotci_syntax",
        "parse_syntax_tree_with_recovery_with_source_and_options_attempt",
    ): "jbotci.syntax.parse_syntax_tree_with_recovery_attempt",
    ("jbotci_syntax", "parse_syntax_tree_with_source_and_options"): (
        "jbotci.syntax.parse_syntax_tree"
    ),
    ("jbotci_syntax", "parse_syntax_tree_with_source_and_options_attempt"): (
        "jbotci.syntax.parse_syntax_tree_attempt"
    ),
}

SUBSUMED_STRING_CARRIERS: dict[str, tuple[str, str]] = {
    "jbotci_morphology::MorphologyContextKind::label": (
        "jbotci.morphology.MorphologyContext.label",
        "MorphologyContext.label returns the exact human-readable label derived from its MorphologyContextKind.",
    ),
    "jbotci_morphology::MorphologyErrorKind::code": (
        "jbotci.morphology.InvalidMorphology.code",
        "InvalidMorphology.code returns the exact diagnostic code for the MorphologyErrorKind carried by that error value.",
    ),
    "jbotci_morphology::MorphologyErrorKind::message": (
        "jbotci.morphology.InvalidMorphology.message",
        "InvalidMorphology.message returns the exact human-readable message for the MorphologyErrorKind carried by that error value.",
    ),
    "jbotci_morphology::MorphologyWarningKind::code": (
        "jbotci.morphology.MorphologyWarning.code",
        "MorphologyWarning.code returns the exact diagnostic code for its MorphologyWarningKind.",
    ),
    "jbotci_morphology::MorphologyWarningKind::detail_reason": (
        "jbotci.diagnostics.Diagnostic.note_segments",
        "MorphologyWarning.to_diagnostic preserves the kind's exact detail reason in the diagnostic note segments.",
    ),
    "jbotci_morphology::MorphologyWarningKind::label": (
        "jbotci.diagnostics.DiagnosticLabel.message",
        "MorphologyWarning.to_diagnostic preserves the kind's exact source label in DiagnosticLabel.message.",
    ),
    "jbotci_morphology::MorphologyWarningKind::message": (
        "jbotci.morphology.MorphologyWarning.message",
        "MorphologyWarning.message returns the exact human-readable message for its MorphologyWarningKind.",
    ),
    "jbotci_syntax::ExperimentalConstruct::code": (
        "jbotci.syntax.SyntaxWarning.code",
        "SyntaxWarning.code returns the exact diagnostic code for its ExperimentalConstruct.",
    ),
    "jbotci_syntax::ExperimentalConstruct::message": (
        "jbotci.syntax.SyntaxWarning.message",
        "SyntaxWarning.message returns the exact human-readable message for its ExperimentalConstruct.",
    ),
    "jbotci_syntax::SyntaxErrorKind::code": (
        "jbotci.syntax.SyntaxErrorParse.code",
        "SyntaxErrorParse.code returns the exact diagnostic code for its SyntaxErrorKind.",
    ),
    "jbotci_syntax::SyntaxErrorKind::message": (
        "jbotci.diagnostics.Diagnostic.message",
        "SyntaxErrorParse.to_diagnostic preserves the SyntaxErrorKind's exact human-readable message in Diagnostic.message.",
    ),
    "jbotci_syntax::SyntaxWordCategory::display_name": (
        "jbotci.syntax.SyntaxExpectedTokenWordCategory.summary_text",
        "SyntaxExpectedTokenWordCategory.summary_text returns the exact display name for its SyntaxWordCategory.",
    ),
}


@dataclass(frozen=True, slots=True)
class InventoryItem:
    """One mechanically extracted Rust declaration."""

    rust_path: str
    kind: str
    signature: str
    suggested_python: str


@dataclass(frozen=True, slots=True)
class Disposition:
    """One reviewed matrix classification."""

    name: str
    python_path: str = ""
    exclusion_kind: str = ""
    rationale: str = ""


def resolve(path: str) -> object | None:
    """Resolve a dotted path from the installed public package."""
    for module_name in PUBLIC_MODULES:
        if path != module_name and not path.startswith(f"{module_name}."):
            continue
        value: object = importlib.import_module(module_name)
        suffix = path.removeprefix(module_name).removeprefix(".")
        for part in suffix.split(".") if suffix else ():
            if not hasattr(value, part):
                return None
            value = getattr(value, part)
        return value
    return None


def rust_inventory() -> tuple[InventoryItem, ...]:
    """Run the syn/schema inventory tool and parse its deterministic output."""
    result = subprocess.run(
        [
            "cargo",
            "run",
            "--quiet",
            "-p",
            "jbotci-python-api-parity",
            "--",
            "inventory",
        ],
        cwd=WORKSPACE_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr or result.stdout)
    items = []
    for line in result.stdout.splitlines():
        rust_path, kind, signature, suggested_python = line.split("\t")
        items.append(InventoryItem(rust_path, kind, signature, suggested_python))
    return tuple(items)


def concept(item: InventoryItem) -> tuple[str, str]:
    """Return the crate and declaration concept for one inventory row."""
    parts = item.rust_path.split("::")
    crate_name = parts[0]
    if crate_name == "jbotci_semantics":
        return (crate_name, parts[2])
    if (
        crate_name == "jbotci_morphology"
        and len(parts) > 2
        and parts[1] in PRIVATE_MORPHOLOGY_MODULES
    ):
        return (crate_name, parts[2])
    if (
        crate_name == "jbotci_syntax"
        and len(parts) > 2
        and parts[1] in {"grammar", "tree"}
    ):
        return (crate_name, parts[2])
    return (crate_name, parts[1])


def rust_only(exclusion_kind: str, rationale: str) -> Disposition:
    """Build a concrete Rust-only classification."""
    return Disposition("rust-only", exclusion_kind=exclusion_kind, rationale=rationale)


def python_api(
    item: InventoryItem,
    name: str,
    python_path: str,
    *,
    rationale: str = "",
) -> Disposition:
    """Build a Python-facing classification only for a live public symbol."""
    if name not in {"direct", "python-equivalent", "subsumed"}:
        raise RuntimeError(
            f"invalid Python disposition {name!r} for {item.rust_path} [{item.kind}]"
        )
    if not python_path or resolve(python_path) is None:
        raise RuntimeError(
            f"non-resolving public Python path {python_path!r} for "
            f"{item.rust_path} [{item.kind}]; add an explicit classification"
        )
    return Disposition(name, python_path, rationale=rationale)


def validate_disposition(item: InventoryItem, disposition: Disposition) -> None:
    """Enforce the resolution invariant at the final emission boundary."""
    if disposition.name in {"direct", "python-equivalent", "subsumed"}:
        if not disposition.python_path or resolve(disposition.python_path) is None:
            raise RuntimeError(
                f"non-resolving public Python path {disposition.python_path!r} for "
                f"{item.rust_path} [{item.kind}]; add an explicit classification"
            )
        return
    if disposition.name != "rust-only":
        raise RuntimeError(
            f"unknown disposition {disposition.name!r} for "
            f"{item.rust_path} [{item.kind}]"
        )


def classify_generated(item: InventoryItem) -> Disposition:
    """Classify generator-owned strict/recovered syntax declarations."""
    path = item.rust_path
    if item.kind == "generator-source":
        return rust_only(
            "implementation-representation",
            "SHA-256 drift sentinel for generator-owned public output; it is tooling metadata, not a runtime domain value.",
        )
    if path.endswith("::generated_model"):
        return python_api(
            item,
            "python-equivalent",
            "jbotci.syntax.strict",
            rationale="Python splits strict and recovered generated models into explicit public namespaces.",
        )
    if "::NodeRef::" in path or path.endswith("::AtomRef::Token"):
        return python_api(
            item,
            "subsumed",
            item.suggested_python,
            rationale="Python returns the concrete immutable typed node or Token directly; the Rust borrowed reference tag carries no additional information.",
        )
    if path.endswith("::NodeRef") or path.endswith("::AtomRef"):
        return rust_only(
            "ownership-lifetime",
            "Borrowed Rust dispatch enum whose alternatives are exposed as independently owned typed Python nodes.",
        )
    if any(
        path.endswith(suffix)
        for suffix in (
            "::TreeNode::as_node_ref",
            "::TreeNode::path_to_node",
            "::TreeNode::node_at_path",
            "::TreeNode::path_to_node_from",
            "::TreeNode::node_at_path_steps",
        )
    ):
        return rust_only(
            "ownership-lifetime",
            "Rust borrow-path identity adapter; Python preserves owner-plus-path identity through same_identity() without exposing raw paths.",
        )
    if path.endswith("::TreeNode"):
        return rust_only(
            "generic-trait",
            "Lifetime-parameterized Rust traversal trait; Python exposes immutable typed fields, structural matching, and same_identity().",
        )
    if path.endswith("::TreeWalker") or path.endswith("::TreeWalkable"):
        return rust_only(
            "generic-trait",
            "Rust generic callback-dispatch trait; Python traversal uses typed child properties and structural pattern matching.",
        )
    if path.endswith("::walk"):
        return rust_only(
            "implementation-representation",
            "Namespace for generated Rust trait descent functions; Python traverses the same typed child properties.",
        )
    if item.kind == "function" and item.signature == "generic generated descent":
        return rust_only(
            "generic-trait",
            "Generic descent adapter for Rust Box, Arc, Option, tuple, or sequence wrappers; Python erases those ownership wrappers.",
        )
    if item.kind in {"trait-method", "function"}:
        module = (
            "jbotci.syntax.recovered"
            if "::recovered::" in path
            else "jbotci.syntax.strict"
        )
        member = path.rsplit("::", 1)[-1]
        target = (
            module
            if not item.suggested_python
            or member in {"visit_in_order", "walk_with", "walk_atom"}
            or path
            == "jbotci_syntax::generated_model_text_syntax_leaf_spans_match_words"
            else item.suggested_python
        )
        return python_api(
            item,
            "subsumed",
            target,
            rationale="Typed Python fields and structural pattern matching provide the same grammar-directed child traversal.",
        )
    if path.endswith("::TextSyntax::visit_source_spans"):
        return python_api(item, "direct", "jbotci.syntax.source_spans")
    if item.kind == "method" and path.rsplit("::", 1)[-1] in {
        "from_valid",
        "from_valid_boxed",
        "try_into_valid",
    }:
        return python_api(
            item,
            "subsumed",
            "jbotci.syntax.parse_syntax_tree_with_recovery",
            rationale="The public strict-or-recovered parse operation performs the same validated boundary conversion without exposing Rust ownership conversions.",
        )
    if item.kind == "variant":
        return python_api(
            item,
            "python-equivalent",
            item.suggested_python,
            rationale="Rust enum alternative is an immutable final Python variant class in the generated closed union.",
        )
    if item.kind in {"type-alias"}:
        return python_api(
            item,
            "python-equivalent",
            item.suggested_python,
            rationale="Python's closed recovered-field union erases the Rust generic alias while preserving every alternative.",
        )
    return python_api(item, "direct", item.suggested_python)


def classify_import_build_item(item: InventoryItem) -> Disposition | None:
    """Recognize deliberate dictionary import/index-build drops."""
    path = item.rust_path
    if path.startswith("jbotci_dictionary::import"):
        return rust_only(
            "serialization-import",
            "Lensisku snapshot deserialization is repository data-build machinery; packaged Python exposes the validated embedded Dictionary and cannot produce these importer values.",
        )
    _, name = concept(item)
    if name == "OwnedDictionaryIndexes" or name.startswith("Owned"):
        return rust_only(
            "serialization-import",
            "Owned index-builder intermediate used to generate static dictionary data; Python queries the validated Dictionary indexes directly.",
        )
    if name in {"RafsiIndexEntry", "SelmahoIndexEntry", "WordIndexEntry"}:
        return rust_only(
            "ownership-lifetime",
            "Borrowed static-index row; Python exposes the corresponding typed Dictionary lookup operation and immutable results.",
        )
    if name == "RafsiIndexTarget":
        return python_api(
            item,
            "subsumed",
            "jbotci.dictionary.Dictionary.lookup_rafsi",
            rationale="lookup_rafsi returns RafsiMatch values containing the referenced DictionaryEntry and the same RafsiSource provenance, replacing the storage-only entry index without losing domain information.",
        )
    if name == "build_owned_indexes":
        return rust_only(
            "serialization-import",
            "Static dictionary index construction belongs to the repository data-generation pipeline, not installed-package lookup.",
        )
    if name == "Dictionary" and path.endswith("::from_static_slices"):
        return rust_only(
            "construction-validation",
            "Static-slice constructor is the embedded-data assembly boundary; Python receives validated Dictionary owners.",
        )
    return None


def classify_other_rust_only(item: InventoryItem) -> Disposition | None:
    """Recognize generic accumulator and ownership-only helpers."""
    key = concept(item)
    if key in RUST_ONLY_CONCEPTS:
        exclusion_kind, rationale = RUST_ONLY_CONCEPTS[key]
        return rust_only(exclusion_kind, rationale)
    path = item.rust_path
    if any(
        path.endswith(name)
        for name in (
            "::push_folded_lojban_diacritics_to",
            "::push_stripped_diacritics_to",
            "::push_stripped_lojban_diacritics_to",
        )
    ):
        return rust_only(
            "ownership-lifetime",
            "Rust caller-owned String accumulator adapter; Python exposes the returned-string equivalent.",
        )
    if key[0] == "jbotci_morphology" and key[1] in {
        "map_verbatim_span",
        "map_word_like_spans",
        "map_word_spans",
    }:
        return rust_only(
            "generic-trait",
            "Generic Rust closure-based ownership transformation; Python source spans are immutable and rebuilt through typed constructors.",
        )
    if path.endswith("::elidable_terminator_for_absent_field_ref"):
        return rust_only(
            "ownership-lifetime",
            "Borrowed parser-construction helper; generated Python fields expose the resulting optional token directly.",
        )
    return None


def classify_syntax_tree_item(item: InventoryItem) -> Disposition | None:
    """Classify hand-written source-backed syntax token helpers."""
    path = item.rust_path
    if path == "jbotci_syntax::tree":
        return python_api(
            item,
            "python-equivalent",
            "jbotci.syntax",
            rationale="Python exposes the hand-written tree helpers in its public syntax namespace.",
        )
    if not path.startswith("jbotci_syntax::tree::"):
        return None
    parts = path.split("::")
    name = parts[2]
    member = parts[-1]
    if name == "SyntaxRecoveryItem":
        if item.kind == "enum":
            return python_api(
                item,
                "python-equivalent",
                "jbotci.syntax.SyntaxRecoveryItem",
                rationale="The Rust enum is the named closed Python union of its two immutable variants.",
            )
        if member == "skipped_tokens":
            return python_api(
                item,
                "subsumed",
                "jbotci.syntax.SkippedTokens.tokens",
                rationale="The concrete Python recovery variant exposes the same optional payload as its immutable tokens field.",
            )
        variant = parts[3]
        target = {
            "SkippedTokens": "jbotci.syntax.SkippedTokens",
            "MissingRequiredField": "jbotci.syntax.MissingRequiredField",
        }[variant]
        if item.kind == "field":
            target = f"{target}.{member}"
        return python_api(
            item,
            "subsumed" if item.kind == "method" else "python-equivalent",
            target,
            rationale="Python's immutable recovery variant and typed fields preserve the exact alternative and all result data.",
        )
    if name == "Token":
        if item.kind == "struct":
            return python_api(item, "direct", "jbotci.syntax.Token")
        equivalent_target = {
            "0": "jbotci.syntax.Token.indicators",
            "as_indicators": "jbotci.syntax.Token.indicators",
            "core_word": "jbotci.syntax.Token.core_word",
            "source_spans": "jbotci.syntax.Token.source_spans",
            "ptr_eq": "jbotci.syntax.Token.same_identity",
            "from_indicators": "jbotci.syntax.Token",
        }.get(member)
        if equivalent_target is not None:
            return python_api(
                item,
                "python-equivalent",
                equivalent_target,
                rationale="Python uses immutable properties, its constructor, and same_identity for the corresponding Rust token operation.",
            )
        target = (
            "jbotci.syntax.Token.source_spans"
            if member == "source_spans_into"
            else "jbotci.syntax.Token.core_word"
        )
        return python_api(
            item,
            "subsumed",
            target,
            rationale="The immutable core_word or source_spans projection preserves the information used by this Rust convenience constructor, predicate, or caller-owned accumulator helper.",
        )
    if name == "WithFreeModifiers":
        target = (
            f"jbotci.syntax.WithFreeModifiers.{member}"
            if member in {"free_modifiers", "value"}
            else "jbotci.syntax.WithFreeModifiers"
        )
        return python_api(
            item,
            "subsumed",
            target,
            rationale="The immutable generic Python wrapper exposes the same value and free_modifiers fields; its value projection subsumes token convenience predicates.",
        )
    if name == "WithIndicators":
        variants = {
            "Plain": "jbotci.syntax.PlainWithIndicators",
            "Emphasized": "jbotci.syntax.EmphasizedWithIndicators",
            "WithIndicator": "jbotci.syntax.IndicatorWithIndicators",
        }
        if item.kind == "enum":
            return python_api(
                item,
                "python-equivalent",
                "jbotci.syntax.WithIndicators",
                rationale="The generic Rust enum is a named closed Python union of immutable final variant classes.",
            )
        if item.kind in {"variant", "field"}:
            variant = parts[3]
            target = variants[variant]
            if item.kind == "field":
                field_alias = "word_like" if member == "0" else member
                target = f"{target}.{field_alias}"
            return python_api(
                item,
                "python-equivalent",
                target,
                rationale="The Rust alternative and its fields are represented by the named immutable Python variant class.",
            )
        return python_api(
            item,
            "subsumed",
            "jbotci.syntax.WithIndicators",
            rationale="Named variant constructors preserve the validated structure; their typed fields recursively expose core words, cmavo classification, quote markers, modifiers, and source spans without losing information.",
        )
    return None


def public_concept_path(item: InventoryItem) -> str:
    """Return the public Python concept or operation that carries this item."""
    key = concept(item)
    if item.kind == "function":
        alias = FUNCTION_ALIASES.get(key)
        if alias is not None:
            return alias
    alias = PYTHON_CONCEPT_ALIASES.get(key)
    if alias is not None:
        return alias
    module = {
        "jbotci_source": "jbotci.source",
        "jbotci_diagnostics": "jbotci.diagnostics",
        "jbotci_dialect": "jbotci.dialect",
        "jbotci_dictionary": "jbotci.dictionary",
        "jbotci_dictionary_data": "jbotci.dictionary",
        "jbotci_jvozba": "jbotci.jvozba",
        "jbotci_morphology": "jbotci.morphology",
        "jbotci_semantics": "jbotci.semantics.references",
        "jbotci_syntax": "jbotci.syntax",
    }[key[0]]
    return f"{module}.{key[1]}"


def classify(item: InventoryItem) -> Disposition:
    """Return exactly one reviewed disposition for an inventory item."""
    if item.rust_path.startswith("jbotci_syntax::generated_model"):
        return classify_generated(item)
    syntax_tree = classify_syntax_tree_item(item)
    if syntax_tree is not None:
        return syntax_tree
    if item.rust_path == "jbotci_morphology::tree":
        return python_api(
            item,
            "python-equivalent",
            "jbotci.morphology",
            rationale="Python exposes the source-backed morphology tree values in the public morphology namespace.",
        )
    excluded = classify_import_build_item(item) or classify_other_rust_only(item)
    if excluded is not None:
        return excluded
    string_carrier = SUBSUMED_STRING_CARRIERS.get(item.rust_path)
    if string_carrier is not None:
        target, rationale = string_carrier
        return python_api(item, "subsumed", target, rationale=rationale)
    if item.suggested_python and resolve(item.suggested_python) is not None:
        if item.kind == "variant":
            return python_api(
                item,
                "python-equivalent",
                item.suggested_python,
                rationale="Rust enum alternative is represented by the named closed-union variant or exact StrEnum member.",
            )
        return python_api(item, "direct", item.suggested_python)
    target = public_concept_path(item)
    if resolve(target) is None:
        raise RuntimeError(
            f"no public Python operation classified for {item.rust_path} [{item.kind}]; "
            f"candidate {target}"
        )
    rationale = (
        "Python exposes the same information through this named immutable class, "
        "closed union, constructor, property, or normalized operation."
    )
    if concept(item) in {
        ("jbotci_dialect", "DialectError"),
        ("jbotci_morphology", "LujvoFragmentError"),
    }:
        rationale = (
            "The Rust error contains only its rendered message; the Python operation "
            "raises InvalidInputError with that exact message and no domain field is lost."
        )
    return python_api(item, "subsumed", target, rationale=rationale)


def render(items: tuple[InventoryItem, ...]) -> str:
    """Render a deterministic fully classified TSV matrix."""
    lines = [HEADER]
    unresolved = []
    for item in items:
        try:
            disposition = classify(item)
            validate_disposition(item, disposition)
        except RuntimeError as error:
            unresolved.append(str(error))
            continue
        fields = (
            item.rust_path,
            item.kind,
            item.signature,
            disposition.name,
            disposition.python_path,
            disposition.exclusion_kind,
            disposition.rationale,
        )
        if any("\t" in field or "\n" in field for field in fields):
            raise RuntimeError(f"matrix field contains TSV control text: {item.rust_path}")
        lines.append("\t".join(fields))
    if unresolved:
        raise RuntimeError(
            f"{len(unresolved)} Rust API items remain unclassified:\n"
            + "\n".join(unresolved)
        )
    return "\n".join(lines) + "\n"


def parse_args() -> argparse.Namespace:
    """Parse matrix generator arguments."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail instead of writing when the checked-in matrix is stale",
    )
    return parser.parse_args()


def main() -> int:
    """Generate or validate the exhaustive parity matrix."""
    expected = render(rust_inventory())
    if parse_args().check:
        if OUTPUT.read_text(encoding="utf-8") == expected:
            return 0
        print(f"{OUTPUT} is stale; run {Path(__file__).name}", file=sys.stderr)
        return 1
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(expected, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
