#!/usr/bin/env python3
"""Fail-closed classifier for the epoch-8 subsentence expectation regeneration.

The baseline archive is `ARCHIVE_COMMIT`, this epoch's implementation base, so every class
earlier epochs contributed is already applied in the baseline and has nothing left to find.

Usage (from the repository root, after the consolidated `fixture-rewrite` passes):

    python3 tools/compare-subsentence-expectations.py tests/fixtures \\
        --baseline-root /build/jbotci/scratch/epoch08/baseline-fixtures

`--baseline-root` is an archive of `tests/fixtures` taken *before* the rewrite; the positional
argument is the rewritten tree.  The classifier reads the *old* Rust-Debug tree and rewrites it
with exactly the mechanical shapes the epoch plan approves.  The rewritten old tree must then
equal the new tree structurally; anything else is emitted as manual residue for an individual
ledger disposition.  Nothing is inferred from the new tree, from fixture text, or from a
span-only comparison, so an ownership change can never be laundered as a re-typing.

Epoch 8 renames no production that a pre-epoch tree can contain.  What it does is add arms and
re-type two payloads, so its classes are position- and arm-level:

Class `zantufa-statement-relative-wrapper` (#818) — the site partition's sum
    The two rolling-Zantufa statement relative arms move inside one sum, `bridi_relative_clause`
    reaching them through the enclosing site's own entry rather than naming them:

        BridiRelativeClause(ZantufaRestrictiveStatementRelativeClause(S { .., statement, .. }))
          -> BridiRelativeClause(ZantufaStatementRelativeClause(
                 ZantufaRestrictiveStatementRelativeClause(S { .., statement', .. })))

    where `statement'` is `statement` under the body re-typing below.  Accepted at the
    `BridiRelativeClause` position only, and only for those two arms: a baseline arm at the same
    position is unchanged and compares equal, and anything else there is residue.

Class `relative-statement-body` (#818) — the tailored Zantufa relative body
    The arms' body moves off the shared `statement` node onto the tailored
    `zantufa_relative_statement` family, which is a re-typing wherever the body is a shape both
    can form:

        StatementBase(BridiStatement(BridiStatementSyntax(B)))
            -> ZantufaRelativeStatementBase(
                   ZantufaRelativeBridiStatement(ZantufaRelativeBridiStatementSyntax(B)))
        StatementBase(TextGroupStatement(T))
            -> ZantufaRelativeStatementBase(TextGroupStatement(T))
        StatementBase(PrenexStatement(P { prenex_terms, zohu, inner_statement }))
            -> ZantufaRelativePrenexStatement(
                   ZantufaRelativePrenexStatementSyntax { prenex_terms, zohu, inner_statement' })

    The prenex rewrite is accepted ONLY on a non-empty `prenex_terms`: Zantufa's prenex requires
    terms, so an empty one is a body its statement cannot form and the surface's owner moved.
    `IStatementConnection` and `PreposedIStatementConnection` are deliberately not rewritten --
    the first is an acceptance flip at the default profile, the second is the unadopted JACU
    tier -- so a baseline body carrying either is residue.

Class `soi-adverbial-arm-split` (#823) — the SOI adverbial's subsentence body
    The one SOI arm becomes three source-qualified ones, and the camxes-exp arm's body is a
    subsentence rather than a statement:

        SoiAdverbialTerm(SoiAdverbialTermSyntax { soi, statement, sehu })
            -> ExpSoiAdverbialTerm(ExpSoiAdverbialTermSyntax(
                   ExpSoiSubsentenceAdverbialSyntax { soi, subsentence, sehu }))

    with `statement` re-typed to a subbridi by the same two shapes as above --
    `BridiStatement` -> `BridiSubbridi`, non-empty `PrenexStatement` -> `PrenexSubbridi`.  A body
    the subsentence cannot form is an owner move to the Zantufa XOI arm and is residue; so is
    every empty prenex, which only camxes-exp admits and which therefore did not come from here.

Class `fihoi-adverbial-arm-split` (#823) — the FIhOI proposal arm
    The FIhOI arm keeps its FIhAU and takes the proposal grammar's subsentence body, and its
    terminator stops being optional because an explicit FIhAU is what selects the arm:

        FihoiAdverbialTerm(FihoiAdverbialTermSyntax { fihoi, statement, fihau: Some(F) })
            -> FihoiProposalAdverbialTerm(
                   FihoiProposalAdverbialTermSyntax { fihoi, subsentence, fihau: F })

    Accepted ONLY on an explicit FIhAU: an elided one is the shared extent R2 gives to the
    camxes-exp arm, which is an owner move and stays residue.

Class `rejection-diagnostic-reclassification` — the error frontier on surfaces that still reject
    A fixture whose syntax status is `failure` on both sides, whose only changed expectation
    leaves are its diagnostics, and every one of whose diagnostics is an `error` on both sides.
    The epoch adds arms at positions these surfaces reach, so the parser's best expectation at
    the failure point changes even though nothing about the surface's acceptance does.  The
    class proves that: a warning appearing or disappearing, an acceptance moving, or any other
    leaf changing takes the fixture out of it.

What is deliberately NOT mechanical
    Every acceptance flip, every warning change and every owner change is manual residue with an
    individual ledger disposition, however large its population: the `po'oi`/`voi'i` silent-to-
    warned flips, the `no'oi` revival, the statement-width bodies that flip reject-to-accept, the
    retired FIhAU hybrid, the SOI extents that move between the three adverbial arms, and the
    tanru-unit relative's own new node.

Epoch-new witnesses are not classified, so the only thing standing behind them is what they pin.
Every one must carry `expectations.syntax.diagnostics`, empty where the expectation is silence,
and one that does not is a hard error rather than a count.
"""

from __future__ import annotations

import argparse
from concurrent.futures import ProcessPoolExecutor
from dataclasses import dataclass
import json
from pathlib import Path
import re
import subprocess
import sys
import threading
import tomllib
from typing import Any, Iterator

EPOCH_BASE = "0d791fd35c"
# The commit whose `tests/fixtures` tree the baseline archive reproduces byte for byte.  Epoch 8
# baselines to its own implementation base, the #866 full-alice merge, so every class earlier
# epochs contributed is already applied in the baseline and has nothing left to find.
ARCHIVE_COMMIT = "0d791fd35c"


@dataclass(frozen=True)
class Form:
    name: str
    fields: tuple[tuple[str, Any], ...] | None = None
    args: tuple[Any, ...] | None = None


class DebugParseError(ValueError):
    pass


IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*")
NUMBER = re.compile(r"-?[0-9]+(?:\.[0-9]+)?")
STRING = re.compile(r'"(?:[^"\\]|\\.)*"')
SPACE = re.compile(r"\s*")


class DebugParser:
    """Parser for the Rust `Debug` rendering the fixtures pin.

    Every scan is anchored with `pattern.match(text, pos)` rather than slicing the
    remaining input: the long-text fixtures pin multi-megabyte trees, and re-slicing per
    token makes the parse quadratic.
    """

    def __init__(self, text: str) -> None:
        self.text = text
        self.pos = 0

    def parse(self) -> Any:
        value = self._value()
        self._space()
        if self.pos != len(self.text):
            raise DebugParseError(f"unexpected input at byte {self.pos}")
        return value

    def _space(self) -> None:
        self.pos = SPACE.match(self.text, self.pos).end()

    def _value(self) -> Any:
        self._space()
        if self.pos >= len(self.text):
            raise DebugParseError("unexpected end of input")
        char = self.text[self.pos]
        if char == '"':
            match = STRING.match(self.text, self.pos)
            if not match:
                raise DebugParseError(f"unterminated string at byte {self.pos}")
            self.pos = match.end()
            return json.loads(match.group(0))
        if char == "[":
            return self._sequence("[", "]", list)
        if char == "(":
            return tuple(self._sequence("(", ")", list))
        number = NUMBER.match(self.text, self.pos)
        if number:
            token = number.group(0)
            self.pos = number.end()
            return float(token) if "." in token else int(token)
        name = self._identifier()
        self._space()
        if self._take("{"):
            fields: list[tuple[str, Any]] = []
            self._space()
            while not self._take("}"):
                key = self._identifier()
                self._space()
                self._expect(":")
                fields.append((key, self._value()))
                self._space()
                if not self._take(","):
                    self._expect("}")
                    break
                self._space()
            return Form(name=name, fields=tuple(fields))
        if self._take("("):
            args = self._sequence_body(")")
            return Form(name=name, args=tuple(args))
        if name == "true":
            return True
        if name == "false":
            return False
        return Form(name=name)

    def _sequence(self, opening: str, closing: str, factory: Any) -> Any:
        self._expect(opening)
        return factory(self._sequence_body(closing))

    def _sequence_body(self, closing: str) -> list[Any]:
        values: list[Any] = []
        self._space()
        while not self._take(closing):
            values.append(self._value())
            self._space()
            if not self._take(","):
                self._expect(closing)
                break
            self._space()
        return values

    def _identifier(self) -> str:
        self._space()
        match = IDENTIFIER.match(self.text, self.pos)
        if not match:
            raise DebugParseError(f"expected identifier at byte {self.pos}")
        self.pos = match.end()
        return match.group(0)

    def _take(self, token: str) -> bool:
        if self.text.startswith(token, self.pos):
            self.pos += len(token)
            return True
        return False

    def _expect(self, token: str) -> None:
        if not self._take(token):
            raise DebugParseError(f"expected {token!r} at byte {self.pos}")



CLASS_RELATIVE_WRAPPER = "zantufa-statement-relative-wrapper"
CLASS_RELATIVE_BODY = "relative-statement-body"
CLASS_SOI_SPLIT = "soi-adverbial-arm-split"
CLASS_FIHOI_SPLIT = "fihoi-adverbial-arm-split"
CLASS_REJECTION_DIAGNOSTICS = "rejection-diagnostic-reclassification"
MECHANICAL_CLASSES = (
    CLASS_RELATIVE_WRAPPER,
    CLASS_RELATIVE_BODY,
    CLASS_SOI_SPLIT,
    CLASS_FIHOI_SPLIT,
    CLASS_REJECTION_DIAGNOSTICS,
)

# --- the shapes, transcribed from the grammar ------------------------------------------
#
# Every entry below was read off `crates/jbotci-syntax/src/grammar/generated.rs` at HEAD and at
# `ARCHIVE_COMMIT`, resolved by production NAME rather than by line number, and the old field
# tuples are the exact ones the baseline macro rendered.

# The sum the two Zantufa statement relative arms move inside, at the one position that holds
# them.
RELATIVE_CLAUSE_POSITIONS: tuple[tuple[str, str], ...] = (
    ("RelativeClauseAtomSyntax", "BridiRelativeClause"),
)
RELATIVE_WRAPPER_ARM = "ZantufaStatementRelativeClause"
ZANTUFA_RELATIVE_ARMS = (
    "ZantufaRestrictiveStatementRelativeClause",
    "ZantufaIncidentalStatementRelativeClause",
)
# arm -> (product struct, old field tuple, the body field)
ZANTUFA_RELATIVE_PRODUCTS: dict[str, tuple[str, tuple[str, ...], str]] = {
    "ZantufaRestrictiveStatementRelativeClause": (
        "ZantufaRestrictiveStatementRelativeClauseSyntax",
        ("poi", "statement", "kuho"),
        "statement",
    ),
    "ZantufaIncidentalStatementRelativeClause": (
        "ZantufaIncidentalStatementRelativeClauseSyntax",
        ("noi", "statement", "kuho"),
        "statement",
    ),
}

# The SOI and FIhOI adverbial arms, keyed on the leaf inventory position that holds them.  All
# nine leaf inventories carry both, and the term view carries the same names one level up.
ADVERBIAL_PARENTS: tuple[str, ...] = (
    "TermSyntax",
    "CeheTermSyntax",
    "LooseTermSyntax",
    "NonabsTermSyntax",
    "SimpleTermSyntax",
    "BoundTermSyntax",
    "NormalTermSyntax",
    "BoundNormalTermSyntax",
    "NormalTermAtomSyntax",
)
SOI_OLD_ARM = "SoiAdverbialTerm"
SOI_OLD_STRUCT = "SoiAdverbialTermSyntax"
SOI_OLD_FIELDS = ("soi", "statement", "sehu")
SOI_NEW_ARM = "ExpSoiAdverbialTerm"
SOI_NEW_WRAPPER = "ExpSoiAdverbialTermSyntax"
SOI_NEW_STRUCT = "ExpSoiSubsentenceAdverbialSyntax"
FIHOI_OLD_ARM = "FihoiAdverbialTerm"
FIHOI_OLD_STRUCT = "FihoiAdverbialTermSyntax"
FIHOI_OLD_FIELDS = ("fihoi", "statement", "fihau")
FIHOI_NEW_ARM = "FihoiProposalAdverbialTerm"
FIHOI_NEW_STRUCT = "FihoiProposalAdverbialTermSyntax"

# The statement bodies both re-typings accept, and the productions they become.
PRENEX_STATEMENT_ARM = "PrenexStatement"
PRENEX_STATEMENT_STRUCT = "PrenexStatementSyntax"
PRENEX_STATEMENT_FIELDS = ("prenex_terms", "zohu", "inner_statement")
BRIDI_STATEMENT_ARM = "BridiStatement"
BRIDI_STATEMENT_STRUCT = "BridiStatementSyntax"
TEXT_GROUP_STATEMENT_ARM = "TextGroupStatement"
STATEMENT_BASE_ARM = "StatementBase"

# Nodes this epoch retires.  A regenerated tree that still contains one means a population moved
# without being dispositioned.  The check runs on the regenerated TEXT, so it fires even where
# the baseline carried the same node and the leaf would otherwise compare equal.
RETIRED_SHAPES: tuple[str, ...] = (
    "SoiAdverbialTerm",
    "FihoiAdverbialTerm",
)

# The reviewed regeneration result.  A re-run against the same archive must reproduce it exactly;
# the `--expect-*` flags override the pins for an exploratory run.
EXPECTED_CHANGED = 122
EXPECTED_MECHANICAL: dict[str, int] = {
    # Zero, and the zero is the measurement: every pre-epoch fixture that reached a Zantufa
    # statement relative arm reached it over a baseline marker and a subbridi-shaped body, so
    # the site classifier returns all of them to a baseline arm and each is an owner change with
    # its own disposition rather than a re-typing.  Nothing in the pre-epoch corpus carried a
    # Zantufa-only body at one of these positions.  The two classes stay because they are what
    # the wrapper and the body re-typing WOULD be, and their unit tests prove they fire; a
    # fixture that acquires such a body later classifies instead of landing in residue.
    CLASS_RELATIVE_WRAPPER: 0,
    CLASS_RELATIVE_BODY: 0,
    CLASS_SOI_SPLIT: 6,
    CLASS_FIHOI_SPLIT: 1,
    CLASS_REJECTION_DIAGNOSTICS: 86,
}
EXPECTED_MANUAL = 29
EXPECTED_PROSE = 0
EXPECTED_NEW_WITNESSES = 89

class Divergence(Exception):
    def __init__(self, path: str, reason: str) -> None:
        super().__init__(f"{path}: {reason}")
        self.path = path
        self.reason = reason


def form_name(value: Any) -> str:
    return value.name if isinstance(value, Form) else type(value).__name__


def field_names(value: Form) -> tuple[str, ...]:
    return tuple(key for key, _ in value.fields or ())


def is_none(value: Any) -> bool:
    return isinstance(value, Form) and value.name == "None" and value.fields is None and (
        value.args is None
    )


NONE_FORM = Form(name="None")


class BodyRetypeError(Exception):
    """A statement body the epoch's re-typing does not produce."""


def _retype_statement(old: Any, mapping: dict[str, Any]) -> Any:
    """Re-type a shared `statement` body onto one of the epoch's two narrower families.

    `mapping` names the target arms; every shape not in it raises, which is what keeps an
    acceptance flip or an owner move out of the class.  The prenex rewrite is refused on an
    empty term run, because neither target family admits one from this position.
    """
    if not isinstance(old, Form) or old.args is None or len(old.args) != 1:
        raise BodyRetypeError("statement body is not a sum arm")
    if old.name != STATEMENT_BASE_ARM:
        raise BodyRetypeError(f"statement body arm {old.name}")
    base = old.args[0]
    if not isinstance(base, Form) or base.args is None or len(base.args) != 1:
        raise BodyRetypeError("statement base is not a sum arm")
    if base.name == BRIDI_STATEMENT_ARM:
        payload = base.args[0]
        if not (
            isinstance(payload, Form)
            and payload.name == BRIDI_STATEMENT_STRUCT
            and payload.args is not None
            and len(payload.args) == 1
        ):
            raise BodyRetypeError("bridi statement is not the single-child product")
        return mapping["bridi"](payload.args[0])
    if base.name == TEXT_GROUP_STATEMENT_ARM:
        if "text_group" not in mapping:
            raise BodyRetypeError("this body family has no TUhE group")
        return mapping["text_group"](base.args[0])
    if base.name == PRENEX_STATEMENT_ARM:
        prenex = base.args[0]
        if not (
            isinstance(prenex, Form)
            and prenex.name == PRENEX_STATEMENT_STRUCT
            and prenex.fields is not None
            and field_names(prenex) == PRENEX_STATEMENT_FIELDS
        ):
            raise BodyRetypeError("prenex statement fields moved")
        values = dict(prenex.fields)
        if values["prenex_terms"] == []:
            raise BodyRetypeError(
                "the empty prenex is a body this family cannot form, so the owner moved"
            )
        inner = _retype_statement(values["inner_statement"], mapping)
        return mapping["prenex"](values["prenex_terms"], values["zohu"], inner)
    raise BodyRetypeError(f"statement base arm {base.name}")


def _relative_body(old: Any) -> Any:
    return _retype_statement(
        old,
        {
            "bridi": lambda payload: Form(
                name="ZantufaRelativeStatementBase",
                args=(
                    Form(
                        name="ZantufaRelativeBridiStatement",
                        args=(
                            Form(
                                name="ZantufaRelativeBridiStatementSyntax",
                                args=(payload,),
                            ),
                        ),
                    ),
                ),
            ),
            "text_group": lambda payload: Form(
                name="ZantufaRelativeStatementBase",
                args=(Form(name=TEXT_GROUP_STATEMENT_ARM, args=(payload,)),),
            ),
            "prenex": lambda terms, zohu, inner: Form(
                name="ZantufaRelativePrenexStatement",
                args=(
                    Form(
                        name="ZantufaRelativePrenexStatementSyntax",
                        fields=(
                            ("prenex_terms", terms),
                            ("zohu", zohu),
                            ("inner_statement", inner),
                        ),
                    ),
                ),
            ),
        },
    )


def _subsentence_body(old: Any) -> Any:
    return _retype_statement(
        old,
        {
            "bridi": lambda payload: Form(
                name="BridiSubbridi",
                args=(Form(name="BridiSubbridiSyntax", args=(payload,)),),
            ),
            "prenex": lambda terms, zohu, inner: Form(
                name="PrenexSubbridi",
                args=(
                    Form(
                        name="PrenexSubbridiSyntax",
                        fields=(
                            ("prenex_terms", terms),
                            ("zohu", zohu),
                            ("inner_subbridi", inner),
                        ),
                    ),
                ),
            ),
        },
    )


def rewrite_node(old: Any, path: str, classes: set[str]) -> Any:
    """Apply the approved node-shape rewrites to one OLD node.

    Every rewrite is keyed on the baseline node's own name and its exact baseline field tuple or
    arity, so a node whose shape does not match the transcribed one falls through unchanged and
    is compared structurally -- which turns an unexpected shape into manual residue rather than
    a laundered rewrite.
    """
    if not isinstance(old, Form):
        return old

    # The two Zantufa statement relative arms move inside one sum at the relative-clause atom.
    if old.name == "BridiRelativeClause" and old.args is not None and len(old.args) == 1:
        inner = old.args[0]
        if isinstance(inner, Form) and inner.name in ZANTUFA_RELATIVE_ARMS:
            classes.add(CLASS_RELATIVE_WRAPPER)
            return Form(
                name="BridiRelativeClause",
                args=(Form(name=RELATIVE_WRAPPER_ARM, args=(inner,)),),
            )
        return old

    # ...and their body moves onto the tailored family.
    for arm, (struct, fields, body_field) in ZANTUFA_RELATIVE_PRODUCTS.items():
        if old.name == struct and old.fields is not None and field_names(old) == fields:
            values = dict(old.fields)
            try:
                body = _relative_body(values[body_field])
            except BodyRetypeError as error:
                raise Divergence(path, f"{arm} body: {error}") from error
            classes.add(CLASS_RELATIVE_BODY)
            return Form(
                name=struct,
                fields=tuple(
                    (key, body if key == body_field else value) for key, value in old.fields
                ),
            )

    # The SOI adverbial arm splits and its body becomes a subsentence.
    if old.name == SOI_OLD_ARM and old.args is not None and len(old.args) == 1:
        payload = old.args[0]
        if not (
            isinstance(payload, Form)
            and payload.name == SOI_OLD_STRUCT
            and payload.fields is not None
            and field_names(payload) == SOI_OLD_FIELDS
        ):
            raise Divergence(path, f"unexpected {SOI_OLD_ARM} payload shape")
        values = dict(payload.fields)
        try:
            body = _subsentence_body(values["statement"])
        except BodyRetypeError as error:
            raise Divergence(path, f"{SOI_OLD_ARM} body: {error}") from error
        classes.add(CLASS_SOI_SPLIT)
        return Form(
            name=SOI_NEW_ARM,
            args=(
                Form(
                    name=SOI_NEW_WRAPPER,
                    args=(
                        Form(
                            name=SOI_NEW_STRUCT,
                            fields=(
                                ("soi", values["soi"]),
                                ("subsentence", body),
                                ("sehu", values["sehu"]),
                            ),
                        ),
                    ),
                ),
            ),
        )

    # The FIhOI arm keeps its FIhAU, which stops being optional, and takes a subsentence body.
    if old.name == FIHOI_OLD_ARM and old.args is not None and len(old.args) == 1:
        payload = old.args[0]
        if not (
            isinstance(payload, Form)
            and payload.name == FIHOI_OLD_STRUCT
            and payload.fields is not None
            and field_names(payload) == FIHOI_OLD_FIELDS
        ):
            raise Divergence(path, f"unexpected {FIHOI_OLD_ARM} payload shape")
        values = dict(payload.fields)
        fihau = values["fihau"]
        if not (isinstance(fihau, Form) and fihau.name == "Some" and fihau.args is not None):
            raise Divergence(
                path,
                "the FIhOI arm elided its FIhAU, so the shared extent moved to the camxes-exp "
                "arm rather than staying the proposal's",
            )
        try:
            body = _subsentence_body(values["statement"])
        except BodyRetypeError as error:
            raise Divergence(path, f"{FIHOI_OLD_ARM} body: {error}") from error
        classes.add(CLASS_FIHOI_SPLIT)
        return Form(
            name=FIHOI_NEW_ARM,
            args=(
                Form(
                    name=FIHOI_NEW_STRUCT,
                    fields=(
                        ("fihoi", values["fihoi"]),
                        ("subsentence", body),
                        ("fihau", fihau.args[0]),
                    ),
                ),
            ),
        )

    return old


def rewrite_position(old: Any, parent: Form, field: str, path: str, classes: set[str]) -> Any:
    """Epoch 8 has no position-keyed rewrite: every shape it moves is keyed on its own name."""
    return old


def compare_tree(
    old: Any,
    new: Any,
    classes: set[str],
    path: str = "",
) -> None:
    """Structural comparison of the rewritten old tree against the new tree."""
    old = rewrite_node(old, path, classes)

    if isinstance(old, Form) or isinstance(new, Form):
        if not (isinstance(old, Form) and isinstance(new, Form)):
            raise Divergence(path, f"{form_name(old)} became {form_name(new)}")
        if old.name != new.name:
            raise Divergence(path, f"{old.name} became {new.name}")
        if (old.fields is None) != (new.fields is None) or (old.args is None) != (
            new.args is None
        ):
            raise Divergence(path, f"{old.name} changed shape")
        if old.fields is not None:
            old_keys = field_names(old)
            new_keys = field_names(new)
            if old_keys != new_keys:
                raise Divergence(path, f"{old.name} fields {old_keys} became {new_keys}")
            for (key, old_child), (_, new_child) in zip(old.fields, new.fields or ()):
                child_path = f"{path}.{key}" if path else key
                compare_tree(
                    rewrite_position(old_child, old, key, child_path, classes),
                    new_child,
                    classes,
                    child_path,
                )
        if old.args is not None:
            if len(old.args) != len(new.args or ()):
                raise Divergence(path, f"{old.name} arity changed")
            for index, (old_child, new_child) in enumerate(zip(old.args, new.args or ())):
                compare_tree(old_child, new_child, classes, f"{path}({index})")
        return

    if isinstance(old, list) or isinstance(new, list):
        if not (isinstance(old, list) and isinstance(new, list)):
            raise Divergence(path, f"{form_name(old)} became {form_name(new)}")
        if len(old) != len(new):
            raise Divergence(path, f"sequence length {len(old)} became {len(new)}")
        for index, (old_child, new_child) in enumerate(zip(old, new)):
            compare_tree(old_child, new_child, classes, f"{path}[{index}]")
        return

    if isinstance(old, tuple) or isinstance(new, tuple):
        if not (isinstance(old, tuple) and isinstance(new, tuple)) or len(old) != len(new):
            raise Divergence(path, "tuple shape changed")
        for index, (old_child, new_child) in enumerate(zip(old, new)):
            compare_tree(old_child, new_child, classes, f"{path}({index})")
        return

    if old != new:
        raise Divergence(path, f"{old!r} became {new!r}")


# Matched on a whole identifier, in both renderings the macro emits.  A sum arm appears as
# `Name(NameSyntax { .. })` and a product reached directly as `NameSyntax { .. }`, so the
# optional suffix covers a list that names some entries by arm and some by struct.  The leading
# boundary is what makes the check usable at all: `GihekBridiTailKeContinuationSyntax`, the node
# the KE narrowing PRODUCES, ends with a retired name, and a substring test reports the
# narrowing's own output as the shape it retires -- hiding the class and inventing residue.
RETIRED_SHAPE_PATTERNS: tuple[tuple[str, re.Pattern[str]], ...] = tuple(
    (name, re.compile(rf"(?<![A-Za-z0-9_]){re.escape(name)}(?:Syntax)?\s*[({{]"))
    for name in RETIRED_SHAPES
)


def assert_no_retired_shapes(tree: str, path: str) -> None:
    """Invariant on the regenerated tree: no node this epoch retires survives in it."""
    for name, pattern in RETIRED_SHAPE_PATTERNS:
        if pattern.search(tree):
            raise Divergence(path, f"regenerated tree still contains the retired node {name}")


def leaves(value: Any, path: tuple[str, ...] = ()) -> Iterator[tuple[tuple[str, ...], Any]]:
    if isinstance(value, dict):
        for key, child in value.items():
            yield from leaves(child, (*path, key))
    else:
        yield path, value


TREE_TOKEN = re.compile(r'^\s*(?:\w+:\s*)?\w+ @\[(?P<span>.*?)\) "(?P<text>.*)"[,]?$')
BRACKET_TOKEN = re.compile(r"[A-Za-z][A-Za-z',\.]*")


def tree_token_projection(value: str) -> tuple[tuple[str, str], ...]:
    result: list[tuple[str, str]] = []
    for line in value.splitlines():
        match = TREE_TOKEN.match(line)
        if match:
            result.append((match.group("span"), match.group("text")))
    return tuple(result)


def json_token_projection(value: Any) -> tuple[tuple[str, Any], ...]:
    result: list[tuple[str, Any]] = []

    def visit(node: Any) -> None:
        if isinstance(node, dict):
            if set(node) >= {"phonemes", "span"}:
                result.append((node["phonemes"], node["span"]))
            for child in node.values():
                visit(child)
        elif isinstance(node, list):
            for child in node:
                visit(child)

    visit(value)
    return tuple(result)


def bracket_token_projection(value: str) -> tuple[str, ...]:
    return tuple(BRACKET_TOKEN.findall(value))


def provenance_prose_only(old_value: Any, new_value: Any) -> bool:
    """True when a `provenance` array moved in its `description` prose and nowhere else."""
    if not isinstance(old_value, list) or not isinstance(new_value, list):
        return False
    if len(old_value) != len(new_value):
        return False
    for old_entry, new_entry in zip(old_value, new_value):
        if not isinstance(old_entry, dict) or not isinstance(new_entry, dict):
            return False
        if set(old_entry) != set(new_entry):
            return False
        if any(old_entry[key] != new_entry[key] for key in old_entry if key != "description"):
            return False
    return True


DIAGNOSTIC_LEAVES = frozenset(
    {
        ("expectations", "syntax", "diagnostics"),
        ("expectations", "syntax", "recovered", "diagnostics"),
    }
)


def is_rejection_diagnostic_reclassification(
    old: dict[str, Any], new: dict[str, Any], old_leaves: dict, new_leaves: dict
) -> bool:
    """A surface that rejected before and rejects now, whose error frontier moved.

    The epoch adds arms at positions these surfaces reach, so the parser's best expectation at
    the failure point changes without anything about the surface's acceptance changing.  The
    class proves exactly that and nothing more: both statuses must be `failure`, the only leaves
    that may move are the two diagnostics lists, and every diagnostic on both sides must be an
    `error` -- a warning appearing or disappearing takes the fixture out of the class, as does
    any other leaf moving at all.
    """
    if set(old_leaves) != set(new_leaves):
        return False
    changed = {path for path, value in old_leaves.items() if new_leaves[path] != value}
    if not changed or not changed <= DIAGNOSTIC_LEAVES:
        return False
    for document in (old, new):
        syntax = document.get("expectations", {}).get("syntax")
        if not isinstance(syntax, dict) or syntax.get("status") != "failure":
            return False
    for path in changed:
        sides = []
        for document in (old_leaves, new_leaves):
            entries = document[path]
            if not isinstance(entries, list):
                return False
            errors = [entry for entry in entries if entry.get("severity") == "error"]
            if not errors:
                return False
            sides.append((errors, [entry for entry in entries if entry not in errors]))
        # Everything that is not an error must be identical on both sides: a fixture whose
        # morphology warning rides in the same list stays in the class, and one whose syntax
        # warnings moved does not.
        if sides[0][1] != sides[1][1]:
            return False
    return True


def compare_fixture(old: dict[str, Any], new: dict[str, Any]) -> tuple[set[str], list[str], bool]:
    old_leaves = dict(leaves(old))
    new_leaves = dict(leaves(new))
    residue: list[str] = []
    classes: set[str] = set()
    prose = False
    # Gentufa renderings are compared by token/span projection rather than by shape, because
    # they are a rendering of the tree and not an expectation about it.  A projection that
    # holds is an ACCEPTED delta, not an unexplained one, so it is recorded here: without it a
    # fixture whose only move is an arm split showing through its gentufa JSON would fall into
    # the "no mechanical shape found" backstop and read as residue.
    projected = False
    if is_rejection_diagnostic_reclassification(old, new, old_leaves, new_leaves):
        classes.add(CLASS_REJECTION_DIAGNOSTICS)
        return classes, residue, prose
    if set(old_leaves) != set(new_leaves):
        added = sorted(".".join(path) for path in set(new_leaves) - set(old_leaves))
        removed = sorted(".".join(path) for path in set(old_leaves) - set(new_leaves))
        reasons = []
        if added:
            reasons.append(f"expectation leaves added: {', '.join(added)}")
        if removed:
            reasons.append(f"expectation leaves removed: {', '.join(removed)}")
        return classes, reasons, prose

    for path, old_value in old_leaves.items():
        new_value = new_leaves[path]
        # The retired-shape invariant runs on every regenerated tree, changed or not: a node this
        # epoch deletes must not survive anywhere, including where the leaf compares equal.
        if path[-1:] == ("raw",) and path[:2] == ("expectations", "syntax"):
            try:
                assert_no_retired_shapes(new_value, ".".join(path))
            except Divergence as divergence:
                residue.append(f"{divergence.path}: {divergence.reason}")
                continue
        if old_value == new_value:
            continue
        joined = ".".join(path)
        # Provenance prose, not an expectation: recorded and counted rather than compared.
        if path == ("provenance",) and provenance_prose_only(old_value, new_value):
            prose = True
            continue
        if path[:2] == ("expectations", "syntax") and "recovered" in path:
            residue.append(f"recovered-tree leaf {joined} (excluded from mechanical classes)")
            continue
        if path == ("expectations", "syntax", "raw"):
            try:
                old_form = DebugParser(old_value).parse()
                new_form = DebugParser(new_value).parse()
            except (DebugParseError, TypeError) as error:
                residue.append(f"syntax tree parser: {error}")
                continue
            try:
                compare_tree(old_form, new_form, classes)
            except Divergence as divergence:
                residue.append(f"syntax tree {divergence.path or '<root>'}: {divergence.reason}")
            continue
        if path[-1:] == ("json",) and "gentufa" in path:
            try:
                old_json = json.loads(old_value)
                new_json = json.loads(new_value)
            except json.JSONDecodeError as error:
                residue.append(f"gentufa.json parser: {error}")
                continue
            if json_token_projection(old_json) != json_token_projection(new_json):
                residue.append("gentufa.json token/span projection changed")
            else:
                projected = True
            continue
        if path[-1:] == ("tree",) and "gentufa" in path:
            if tree_token_projection(old_value) != tree_token_projection(new_value):
                residue.append("gentufa.tree token/span projection changed")
            else:
                projected = True
            continue
        if path[-1:] == ("brackets",) and "gentufa" in path:
            if bracket_token_projection(old_value) != bracket_token_projection(new_value):
                residue.append("gentufa.brackets token projection changed")
            else:
                projected = True
            continue
        # Diagnostics, statuses, digests, semantics refs, tersmu output and every other leaf are
        # deliberately exact.  In this epoch that is what puts the whole `bare_cu_bridi` warning
        # population, and every acceptance flip, into individually dispositioned residue.
        residue.append(joined)
    if not classes and old != new and not residue and not prose and not projected:
        residue.append("no mechanical subsentence shape found")
    return classes, residue, prose


def classify_one(job: tuple[str, str, str]) -> tuple[str, list[str], list[str], bool]:
    """Worker entry point.  The texts are re-read here so the parent never holds them."""
    repository_path, baseline_file, candidate_file = job
    old = tomllib.loads(Path(baseline_file).read_text())
    new = tomllib.loads(Path(candidate_file).read_text())
    classes, residue, prose = compare_fixture(old, new)
    return repository_path, sorted(classes), residue, prose


def collect_jobs(
    candidate_root: Path, baseline_root: Path, witnesses: set[str]
) -> tuple[list[tuple[str, str, str]], list[tuple[str, str, str]], list[str], list[str]]:
    jobs: list[tuple[str, str, str]] = []
    witness_jobs: list[tuple[str, str, str]] = []
    epoch_new: list[str] = []
    unpaired: list[str] = []
    seen: set[Path] = set()
    for candidate_path in sorted(candidate_root.rglob("*.toml")):
        relative = candidate_path.relative_to(candidate_root)
        seen.add(relative)
        repository_path = (Path("tests/fixtures") / relative).as_posix()
        baseline_file = baseline_root / relative
        if not baseline_file.exists():
            if repository_path in witnesses:
                epoch_new.append(repository_path)
            else:
                unpaired.append(f"{repository_path}: absent from the baseline archive")
            continue
        if baseline_file.read_bytes() == candidate_path.read_bytes():
            continue
        if repository_path in witnesses:
            # Epoch witnesses carry commit-local exact pins, authored from the frozen decision
            # function rather than from the writer.  Any delta at all means the pin was wrong.
            witness_jobs.append((repository_path, str(baseline_file), str(candidate_path)))
            continue
        jobs.append((repository_path, str(baseline_file), str(candidate_path)))
    for baseline_path in sorted(baseline_root.rglob("*.toml")):
        relative = baseline_path.relative_to(baseline_root)
        if relative not in seen:
            repository_path = (Path("tests/fixtures") / relative).as_posix()
            unpaired.append(f"{repository_path}: absent from the candidate tree")
    return jobs, witness_jobs, unpaired, epoch_new


def epoch_new_missing_diagnostics(candidate_root: Path, epoch_new: list[str]) -> list[str]:
    """Epoch-new witnesses that pin no `expectations.syntax.diagnostics` list."""
    offenders: list[str] = []
    for repository_path in epoch_new:
        relative = Path(repository_path).relative_to("tests/fixtures")
        document = tomllib.loads((candidate_root / relative).read_text(encoding="utf-8"))
        syntax = document.get("expectations", {}).get("syntax")
        if syntax is None:
            offenders.append(f"{repository_path}: no expectations.syntax to pin diagnostics on")
        elif "diagnostics" not in syntax:
            offenders.append(f"{repository_path}: expectations.syntax pins no diagnostics list")
    return sorted(offenders)


def run(args: argparse.Namespace) -> int:
    witnesses = set(
        subprocess.run(
            [
                "git",
                "diff",
                "--diff-filter=A",
                "--name-only",
                f"{EPOCH_BASE}..HEAD",
                "--",
                "tests/fixtures",
            ],
            check=True,
            text=True,
            stdout=subprocess.PIPE,
        ).stdout.splitlines()
    )
    jobs, witness_jobs, unpaired, epoch_new = collect_jobs(
        args.candidate, args.baseline_root, witnesses
    )

    mechanical: dict[str, list[str]] = {name: [] for name in MECHANICAL_CLASSES}
    manual: list[tuple[str, list[str]]] = []
    prose_edits: list[str] = []
    witness_deltas: list[str] = []
    with ProcessPoolExecutor(max_workers=args.jobs) as pool:
        for repository_path, classes, residue, prose in pool.map(
            classify_one, jobs, chunksize=16
        ):
            if prose:
                prose_edits.append(repository_path)
            if residue:
                manual.append((repository_path, residue))
            else:
                for classification in classes:
                    mechanical[classification].append(repository_path)
        for repository_path, classes, residue, prose in pool.map(
            classify_one, witness_jobs, chunksize=4
        ):
            witness_deltas.append(repository_path)

    lines = [f"changed: {len(jobs)}"]
    for classification, paths in mechanical.items():
        lines.append(f"mechanical {classification}: {len(paths)}")
    lines.append(f"manual: {len(manual)}")
    for path, reasons in sorted(manual):
        lines.append(f"  {path}: {'; '.join(reasons)}")
    lines.append(f"prose-only provenance edits: {len(prose_edits)}")
    lines.extend(f"  {path}" for path in sorted(prose_edits))
    lines.append(f"epoch-new witnesses (authored, unclassifiable): {len(epoch_new)}")
    lines.extend(f"  {path}" for path in sorted(epoch_new))
    unpinned_diagnostics = epoch_new_missing_diagnostics(args.candidate, epoch_new)
    if unpinned_diagnostics:
        lines.append(
            "epoch-new witnesses without a diagnostics pin (must be empty): "
            f"{len(unpinned_diagnostics)}"
        )
        lines.extend(f"  {entry}" for entry in unpinned_diagnostics)
    if unpaired:
        lines.append(f"unpaired fixtures (must be empty): {len(unpaired)}")
        lines.extend(f"  {entry}" for entry in sorted(unpaired))
    if witness_deltas:
        lines.append(f"epoch-witness deltas (must be empty): {len(witness_deltas)}")
        lines.extend(f"  {path}" for path in sorted(witness_deltas))
    report = "\n".join(lines)
    print(report)
    if args.report:
        args.report.write_text(report + "\n")
    if args.report_json:
        args.report_json.write_text(
            json.dumps(
                {
                    "changed": len(jobs),
                    "mechanical": mechanical,
                    "manual": [
                        {"fixture": path, "reasons": reasons} for path, reasons in sorted(manual)
                    ],
                    "prose_edits": sorted(prose_edits),
                    "unpaired": sorted(unpaired),
                    "witness_deltas": sorted(witness_deltas),
                    "epoch_new": sorted(epoch_new),
                    "epoch_new_missing_diagnostics": unpinned_diagnostics,
                },
                indent=2,
                sort_keys=True,
            )
            + "\n"
        )

    if unpaired:
        print(
            "error: every fixture must be paired with the baseline archive "
            f"({ARCHIVE_COMMIT}); an unpaired one is never skipped"
        )
        return 1
    if witness_deltas:
        print("error: epoch witnesses are authored pins and may take no regeneration delta")
        return 1
    if unpinned_diagnostics:
        print(
            "error: every epoch-new witness must pin expectations.syntax.diagnostics, "
            "empty where the expectation is silence"
        )
        return 1
    if len(jobs) != args.expect_changed:
        print(f"error: expected {args.expect_changed} changed pre-epoch fixtures")
        return 1
    if {name: len(paths) for name, paths in mechanical.items()} != EXPECTED_MECHANICAL:
        print("error: mechanical class incidences differ from the reviewed C7 set")
        return 1
    if len(manual) != args.expect_manual:
        print(f"error: expected {args.expect_manual} manual-residue fixtures")
        return 1
    if len(prose_edits) != args.expect_prose:
        print(f"error: expected {args.expect_prose} prose-only provenance edits")
        return 1
    if len(epoch_new) != args.expect_new_witnesses:
        print(f"error: expected {args.expect_new_witnesses} epoch-new witness fixtures")
        return 1
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--baseline-root", type=Path, required=True)
    parser.add_argument("--report", type=Path)
    parser.add_argument("--report-json", type=Path)
    parser.add_argument("--jobs", type=int, default=15)
    parser.add_argument("--expect-changed", type=int, default=EXPECTED_CHANGED)
    parser.add_argument("--expect-manual", type=int, default=EXPECTED_MANUAL)
    parser.add_argument("--expect-prose", type=int, default=EXPECTED_PROSE)
    parser.add_argument("--expect-new-witnesses", type=int, default=EXPECTED_NEW_WITNESSES)
    args = parser.parse_args()

    # The pinned trees nest deeply enough to exhaust the default interpreter stack.
    sys.setrecursionlimit(200_000)
    threading.stack_size(512 * 1024 * 1024)
    result: list[int] = []
    worker = threading.Thread(target=lambda: result.append(run(args)))
    worker.start()
    worker.join()
    return result[0] if result else 1


if __name__ == "__main__":
    raise SystemExit(main())
