"""Declarative dialect formulas, feature sets, and settings."""

from collections.abc import Sequence
from typing import TypeAlias

from . import _native as _rust

from ._native import (
    _dialect_BuiltinDialect as BuiltinDialect,
    _dialect_CmavoExpansion as CmavoExpansion,
    _dialect_CmavoSwap as CmavoSwap,
    _dialect_CustomDialect as CustomDialect,
    _dialect_DialectDefinition as DialectDefinition,
    _dialect_DialectFeature as DialectFeature,
    _dialect_DialectSettings as DialectSettings,
)

CmavoDialectEntry: TypeAlias = CmavoSwap | CmavoExpansion


def _dialect_feature_atom_name(self: DialectFeature) -> str:
    """Return this feature's uppercase declarative atom name."""

    return _rust._dialect_dialect_feature_atom_name(self)


setattr(DialectFeature, "atom_name", _dialect_feature_atom_name)


def parse_dialect_definition(source: str) -> DialectDefinition:
    """Parse a standalone declarative dialect formula."""

    return _rust._dialect_parse_dialect_definition(source)


def builtin_dialects() -> tuple[BuiltinDialect, ...]:
    """Return every built-in dialect definition in stable order."""

    return _rust._dialect_builtin_dialects()


def builtin_dialect_names() -> tuple[str, ...]:
    """Return every built-in dialect name in stable order."""

    return _rust._dialect_builtin_dialect_names()


def find_builtin_dialect(name: str) -> BuiltinDialect | None:
    """Look up a built-in dialect by its exact name."""

    return _rust._dialect_find_builtin_dialect(name)


def parse_dialect_definition_with_custom_dialects(
    custom: Sequence[CustomDialect], source: str
) -> DialectDefinition:
    """Parse a dialect formula with a supplied custom-dialect environment."""

    return _rust._dialect_parse_dialect_definition_with_custom_dialects(custom, source)


def parse_dialect_selection_formula(
    settings: DialectSettings, source: str
) -> DialectDefinition:
    """Parse a saved dialect-selection formula against its settings."""

    return _rust._dialect_parse_dialect_selection_formula(settings, source)


def custom_dialect_is_valid(
    existing: Sequence[CustomDialect], custom: CustomDialect
) -> None:
    """Validate a custom dialect against the existing collection."""

    return _rust._dialect_custom_dialect_is_valid(existing, custom)


def dialect_name_shows_in_gentufa_picker(name: str) -> bool:
    """Return whether a dialect name is visible in the Gentufa picker."""

    return _rust._dialect_dialect_name_shows_in_gentufa_picker(name)


def dialect_formula_top_level_references(formula_text: str) -> tuple[str, ...]:
    """Return the top-level dialect references in a formula."""

    return _rust._dialect_dialect_formula_top_level_references(formula_text)


def add_dialect_formula_reference(dialect_name: str, formula_text: str) -> str:
    """Add a top-level dialect reference to a formula."""

    return _rust._dialect_add_dialect_formula_reference(dialect_name, formula_text)


def remove_dialect_formula_reference(dialect_name: str, formula_text: str) -> str:
    """Remove a top-level dialect reference from a formula."""

    return _rust._dialect_remove_dialect_formula_reference(dialect_name, formula_text)


def replace_dialect_formula_reference(
    previous_name: str, next_name: str, formula_text: str
) -> str:
    """Replace one top-level dialect reference with another."""

    return _rust._dialect_replace_dialect_formula_reference(
        previous_name, next_name, formula_text
    )


def dialect_definition_to_text(definition: DialectDefinition) -> str:
    """Render a declarative dialect definition as canonical text."""

    return _rust._dialect_dialect_definition_to_text(definition)


def cmavo_dialect_entries_to_definition(
    entries: tuple[CmavoDialectEntry, ...]
) -> str:
    """Render cmavo dialect entries as a definition formula."""

    return _rust._dialect_cmavo_dialect_entries_to_definition(entries)


def custom_dialect_definition_to_johau_uri(definition: str) -> str:
    """Encode a custom dialect definition as a ``johau`` URI."""

    return _rust._dialect_custom_dialect_definition_to_johau_uri(definition)


def custom_dialect_definition_to_johau_uri_with_custom_dialects(
    custom: Sequence[CustomDialect], definition: str
) -> str:
    """Encode a custom definition using a supplied dialect environment."""

    return _rust._dialect_custom_dialect_definition_to_johau_uri_with_custom_dialects(
        custom, definition
    )


def dialect_definition_to_johau_uri(definition: DialectDefinition) -> str:
    """Encode a parsed dialect definition as a ``johau`` URI."""

    return _rust._dialect_dialect_definition_to_johau_uri(definition)


def parse_johau_dialect_uri(raw_uri: str) -> DialectDefinition:
    """Parse a ``johau`` URI into its declarative definition."""

    return _rust._dialect_parse_johau_dialect_uri(raw_uri)


def import_johau_dialect_settings(
    raw_uri: str, settings: DialectSettings
) -> tuple[str, DialectSettings]:
    """Import a ``johau`` URI into immutable saved dialect settings."""

    return _rust._dialect_import_johau_dialect_settings(raw_uri, settings)

__all__: tuple[str, ...] = (
    "DialectFeature",
    "CmavoSwap",
    "CmavoExpansion",
    "CmavoDialectEntry",
    "DialectDefinition",
    "BuiltinDialect",
    "CustomDialect",
    "DialectSettings",
    "parse_dialect_definition",
    "builtin_dialects",
    "builtin_dialect_names",
    "find_builtin_dialect",
    "parse_dialect_definition_with_custom_dialects",
    "parse_dialect_selection_formula",
    "custom_dialect_is_valid",
    "dialect_name_shows_in_gentufa_picker",
    "dialect_formula_top_level_references",
    "add_dialect_formula_reference",
    "remove_dialect_formula_reference",
    "replace_dialect_formula_reference",
    "dialect_definition_to_text",
    "cmavo_dialect_entries_to_definition",
    "custom_dialect_definition_to_johau_uri",
    "custom_dialect_definition_to_johau_uri_with_custom_dialects",
    "dialect_definition_to_johau_uri",
    "parse_johau_dialect_uri",
    "import_johau_dialect_settings",
)
