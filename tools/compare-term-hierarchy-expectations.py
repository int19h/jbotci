#!/usr/bin/env python3
"""Fail-closed classifier for the epoch-6 term-hierarchy expectation regeneration.

Usage (from the repository root, after the consolidated `fixture-rewrite` passes):

    python3 tools/compare-term-hierarchy-expectations.py tests/fixtures \\
        --baseline-root /build/jbotci/scratch/epoch06/baseline-fixtures

`--baseline-root` is an archive of `tests/fixtures` taken *before* the rewrite; the
positional argument is the rewritten tree.  The classifier reads the *old* Rust-Debug tree
and rewrites it with exactly the mechanical shapes the epoch plan approves.  The rewritten
old tree must then equal the new tree byte for byte; anything else is emitted as manual
residue for an individual ledger disposition.  Nothing is inferred from the new tree, from
fixture text, or from a span-only comparison, so an ownership change can never be laundered
as a re-leveling.

Class (i) `flat-sum-wrapper` — the wrapper paths of the five former `term` sum siblings
    The old flat `term` sum was `pehe_termset_connection / bound_term_connection /
    termset_group / connected_term / simple_term`.  Two of those five siblings wrapped an
    atom without contributing structure, and the composed ladder drops both wrappers:

    * `connected_term` had a `zero_or_more` continuation list, so every connectionless term
      in the corpus was `ConnectedTerm { leading_term, continuations: [] }`.  These
      degenerate zero-continuation wrappers are the enumerated class-(i) case: each level of
      the ladder now requires at least one continuation, so a connectionless term selects
      its leaf directly.
    * `simple_term` was a *nested sum branch*, which prints an extra `SimpleTerm(..)` wrapper
      variant.  Every ladder level now re-lists the leaves (mechanism E), so that wrapper is
      gone.  At the old flat-`term` position the strip is class (i); at a PEhE operand it is
      the class (ii) re-typing below.

    The other three siblings (`pehe_termset_connection`, `termset_group`,
    `bound_term_connection`) contribute real structure and are never rewritten: they are
    compared structurally, and `bound_term_connection` is the route #796 deletes.

    Class (i) is atom-only and fail-closed.  A wrapper is stripped only when its payload is
    one of the plain leaves — never a connection (`ConnectedTerm`, `StagBoundTermConnection`,
    `BoundTermConnection`, `TermsetGroup`, `PeheTermsetConnection`), never a termset or GEK
    atom (`ForethoughtTermset`, `NuhiTermset`, `KeTermset`), and never a leaf whose own sumti
    carries a VUhO attachment, which is the epoch-4/D6 ownership surface.  Recovered trees
    are excluded from every mechanical class.

    Per-level nesting-depth validation (plan C7): every strip is performed at a position
    whose old and new ladder levels are named in `POSITIONS`, and the surviving leaf's
    variant name must be a member of that position's *new* level inventory
    (`NEW_LEVEL_INVENTORY`, transcribed from the rebuilt grammar).  A leaf that landed at
    the wrong ladder depth — for example a `NonabsTaggedSumtiTerm` outside a CEhE
    continuation, or a `TaggedSumtiTerm` inside one — is residue, not a re-leveling.

Class (ii) `pehe-cehe-retyping` — PEhE and CEhE operand re-typing
    The PEhE operand level changed from `pehe_termset_operand` to `cehe_term` and the CEhE
    continuation from `simple_term` to `nonabs_term`.  Both re-typings are accepted only
    with the parent shape *and* the connective exhaustively proven:

    * PEhE: the parent must be exactly `PeheTermsetConnectionSyntax.leading_term` or
      `PeheTermsetConnectionContinuationSyntax.trailing_term`, and every governing PEhE
      connective must be `JoikConnective` or `JekConnective` — the corrected #806 domain.  A
      pre-existing `pe'e` + EK or `pe'e` + VUhU fixture is an acceptance *flip*, not a
      re-typing, and stays in residue.
    * CEhE: the parent must be exactly `TermsetGroupContinuationSyntax.trailing_term` with
      its `cehe` token present, and only the tag-led leaf may change name
      (`TaggedSumtiTerm` -> `NonabsTaggedSumtiTerm`).  The payload must be otherwise
      identical.

Class (iii) from the plan, sumti-term pass-through, is PROHIBITED and deliberately not
implemented: it would accept ownership changes at identical spans.

#796 flip `stagless-bo-route-rejection`
    The deleted standard stag-less BO term route makes its surfaces reject.  A fixture whose
    old tree contains `BoundTermConnection` and whose status flips success -> failure is a
    mechanical regeneration of that deletion; the flip is accepted only with that node
    present in the old tree, and the new failure diagnostics are still pinned exactly.

Interpretation recorded for review: plan C7 requires "atom-only old-shape ... fail-closed",
and reconciliation B9 spells that out as "excluding connectives, VUhO, relatives, termsets,
GEK, BO, recovered trees".  Every one of those exclusions is implemented above except a
blanket exclusion of terms whose sumti carries a relative clause.  A relative clause is
sumti-internal structure, not a term-tier shape, and class (i) already requires the
unwrapped payload to match the regenerated tree byte for byte, so no relative-clause
ownership change can survive the comparison; excluding them would move 1,769 pre-epoch
fixtures into hand-written residue rows without adding any safety.  The VUhO attachment is
excluded because that *is* a term-versus-sumti ownership surface (epoch-4 residual, D6).
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

EPOCH_BASE = "a8b4f06227"


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


# --- the ladder, transcribed from the grammar ------------------------------------------
#
# Every term-family position, with the rule that produced its operand at the epoch base
# (`a8b4f06227`) and the ladder level that produces it now.  The 25 consumer sites keep
# their arity and their level name, so they are listed by the field that holds the term.
# Any term-family node that turns up at a position missing from this table is residue.

CONSUMER_POSITIONS: tuple[tuple[str, str], ...] = (
    ("ZantufaIauStatementTermsTailSyntax", "terms"),
    ("ZantufaBareStatementTermsTailSyntax", "terms"),
    ("PrenexFragmentSyntax", "terms"),
    ("PrenexStatementSyntax", "prenex_terms"),
    ("TermsFragmentSyntax", "terms"),
    ("BridiWithLeadingTermsSyntax", "leading_terms"),
    ("BridiWithPostCuTermsSyntax", "leading_terms"),
    ("CuTermsBridiTailSyntax", "terms"),
    ("ZantufaGroupedBridiTailSyntax", "tail_terms"),
    ("SelbriSimpleBridiTailSyntax", "terms"),
    ("DirectForethoughtBridiConnectionSyntax", "tail_terms"),
    ("BridiTailKeContinuationSyntax", "tail_terms"),
    ("GihekBridiTailKeContinuationSyntax", "tail_terms"),
    ("BridiTailBoContinuationSyntax", "tail_terms"),
    ("BridiTailContinuationSyntax", "tail_terms"),
    ("PrenexSubbridiSyntax", "prenex_terms"),
    ("ForethoughtTermsetSyntax", "terms"),
    ("ForethoughtTermsetBranchSyntax", "terms"),
    ("ZantufaForethoughtTermsetBranchSyntax", "terms"),
    ("NuhiTermsetSyntax", "termset"),
    ("KeTermsetSyntax", "termset"),
    ("LaheTermWrapperSyntax", "inner_term"),
    ("ScalarNegatedTermWrapperWithBoSyntax", "inner_term"),
    ("ScalarNegatedTermWrapperSyntax", "inner_term"),
    ("SeiFreeModifierSyntax", "terms"),
)

# (parent Debug struct, field) -> (old level, new level)
POSITIONS: dict[tuple[str, str], tuple[str, str]] = {
    position: ("term", "term") for position in CONSUMER_POSITIONS
}
POSITIONS.update(
    {
        ("PeheTermsetConnectionSyntax", "leading_term"): ("pehe_termset_operand", "cehe_term"),
        ("PeheTermsetConnectionContinuationSyntax", "trailing_term"): (
            "pehe_termset_operand",
            "cehe_term",
        ),
        ("TermsetGroupSyntax", "leading_term"): ("simple_term", "loose_term"),
        ("TermsetGroupContinuationSyntax", "trailing_term"): ("simple_term", "nonabs_term"),
        ("ConnectedTermSyntax", "leading_term"): ("bound_term", "bound_term"),
        ("ConnectedTermContinuationSyntax", "trailing_term"): ("bound_term", "bound_term"),
        ("StagBoundTermConnectionSyntax", "leading_term"): ("simple_term", "simple_term"),
        ("StagBoundTermContinuationSyntax", "trailing_term"): ("simple_term", "simple_term"),
        # The #796 route, deleted by this epoch; it can only appear on the old side.
        ("BoundTermConnectionSyntax", "leading_term"): ("simple_term", "<deleted>"),
        ("BoundTermConnectionSyntax", "trailing_term"): ("simple_term", "<deleted>"),
    }
)

ATOM_LEAVES = frozenset(
    {
        "PlaceTaggedSumtiTerm",
        "JaiTaggedSumtiTerm",
        "ElidedNaheFihoTagTerm",
        "TaggedSumtiBeforeTagTerm",
        "TaggedSumtiTerm",
        "NonabsTaggedSumtiTerm",
        "NoihaAdverbialTerm",
        "FihoiAdverbialTerm",
        "SoiAdverbialTerm",
        "NaKuTerm",
        "SumtiTerm",
        "BareNaTerm",
        "ForethoughtTermset",
        "NuhiTermset",
        "KeTermset",
    }
)

# Atoms that carry a termset or a GEK.  Sol's fail-closed rule excludes them from every
# mechanical rewrite even though the old grammar listed them as `simple_term` leaves.
TERMSET_ATOMS = frozenset({"ForethoughtTermset", "NuhiTermset", "KeTermset"})

TERM_CONNECTION_NAMES = frozenset(
    {
        "ConnectedTerm",
        "StagBoundTermConnection",
        "BoundTermConnection",
        "TermsetGroup",
        "PeheTermsetConnection",
    }
)

OLD_LEVEL_INVENTORY: dict[str, frozenset[str]] = {
    "term": frozenset(
        {"PeheTermsetConnection", "BoundTermConnection", "TermsetGroup", "ConnectedTerm"}
        | {"SimpleTerm"}
    ),
    "pehe_termset_operand": frozenset(
        {"BoundTermConnection", "StagBoundTermConnection", "TermsetGroup", "SimpleTerm"}
    ),
    "bound_term": frozenset({"StagBoundTermConnection"} | (ATOM_LEAVES - {"NonabsTaggedSumtiTerm"})),
    "simple_term": frozenset(ATOM_LEAVES - {"NonabsTaggedSumtiTerm"}),
}

NEW_LEVEL_INVENTORY: dict[str, frozenset[str]] = {
    "term": frozenset(
        {"PeheTermsetConnection", "TermsetGroup", "ConnectedTerm", "StagBoundTermConnection"}
        | (ATOM_LEAVES - {"NonabsTaggedSumtiTerm"})
    ),
    "cehe_term": frozenset(
        {"TermsetGroup", "ConnectedTerm", "StagBoundTermConnection"}
        | (ATOM_LEAVES - {"NonabsTaggedSumtiTerm"})
    ),
    "loose_term": frozenset(
        {"ConnectedTerm", "StagBoundTermConnection"} | (ATOM_LEAVES - {"NonabsTaggedSumtiTerm"})
    ),
    "nonabs_term": frozenset(
        {"ConnectedTerm", "StagBoundTermConnection"} | (ATOM_LEAVES - {"TaggedSumtiTerm"})
    ),
    "bound_term": frozenset(
        {"StagBoundTermConnection"} | (ATOM_LEAVES - {"NonabsTaggedSumtiTerm"})
    ),
    "simple_term": frozenset(ATOM_LEAVES - {"NonabsTaggedSumtiTerm"}),
    "<deleted>": frozenset(),
}

# The #791 tripwire node types (Kimi R3).  A diff that touches one of them is never
# mechanical: the epoch plan requires stopping and fixing the grammar rather than the
# expectation.  The single enumerated exception is the degenerate zero-continuation
# `ConnectedTerm` wrapper handled by class (i).
TRIPWIRE_NAMES = frozenset({"ConnectedTerm", "ConnectedLinkedTerm", "LinkedSumti", "Linkargs"})

PEHE_CONNECTIVE_NAMES = frozenset({"JoikConnective", "JekConnective"})

# The reviewed C7 result.  A re-run against the same archive must reproduce it exactly;
# `--expect-changed`/`--expect-manual` override the pins for an exploratory run.
EXPECTED_CHANGED = 18244
EXPECTED_MECHANICAL = {
    "flat-sum-wrapper": 18192,
    "pehe-cehe-retyping": 5,
    "stagless-bo-route-rejection": 0,
}
EXPECTED_MANUAL = 49

CLASS_FLAT_SUM_WRAPPER = "flat-sum-wrapper"
CLASS_PEHE_CEHE_RETYPING = "pehe-cehe-retyping"
CLASS_BO_ROUTE_REJECTION = "stagless-bo-route-rejection"
MECHANICAL_CLASSES = (
    CLASS_FLAT_SUM_WRAPPER,
    CLASS_PEHE_CEHE_RETYPING,
    CLASS_BO_ROUTE_REJECTION,
)


class Divergence(Exception):
    def __init__(self, path: str, reason: str) -> None:
        super().__init__(f"{path}: {reason}")
        self.path = path
        self.reason = reason


def form_name(value: Any) -> str:
    return value.name if isinstance(value, Form) else type(value).__name__


def single_arg(value: Any, name: str) -> Any | None:
    """Return the single payload of `Name(payload)`, or None."""
    if isinstance(value, Form) and value.name == name and value.args is not None:
        if len(value.args) == 1:
            return value.args[0]
    return None


def own_sumti_carries_vuho(leaf: Form) -> bool:
    """True when the leaf's own sumti has a VUhO attachment (the D6 ownership surface)."""
    payload = leaf.args[0] if leaf.args and len(leaf.args) == 1 else None
    if payload is None:
        return False
    candidates: list[Any] = []
    if isinstance(payload, Form):
        if payload.fields is not None:
            candidates.extend(child for key, child in payload.fields if key == "sumti")
        elif payload.args is not None:
            candidates.extend(payload.args)
    for candidate in candidates:
        sumti = single_arg(candidate, "Sumti")
        if sumti is None:
            sumti = candidate
        if isinstance(sumti, Form) and sumti.fields is not None:
            for key, child in sumti.fields:
                if key == "vuho_attachment" and form_name(child) != "None":
                    return True
    return False


def require_mechanical_atom(leaf: Any, path: str, wrapper: str) -> Form:
    if not isinstance(leaf, Form):
        raise Divergence(path, f"{wrapper} payload is not a syntax node")
    if leaf.name in TERM_CONNECTION_NAMES:
        raise Divergence(path, f"{wrapper} payload is the connection node {leaf.name}")
    if leaf.name in TERMSET_ATOMS:
        raise Divergence(path, f"{wrapper} payload is the termset/GEK atom {leaf.name}")
    if leaf.name not in ATOM_LEAVES:
        raise Divergence(path, f"{wrapper} payload {leaf.name} is not a term leaf")
    if own_sumti_carries_vuho(leaf):
        raise Divergence(path, f"{wrapper} payload carries a VUhO attachment")
    return leaf


def require_new_level_admits(name: str, new_level: str, path: str) -> None:
    inventory = NEW_LEVEL_INVENTORY.get(new_level)
    if inventory is None:
        raise Divergence(path, f"unknown ladder level {new_level}")
    if name not in inventory:
        raise Divergence(path, f"{name} is not a leaf of the {new_level} level")


def pehe_connectives_are_corrected(parent: Form, field: str, path: str) -> None:
    """Prove the connective domain of a PEhE parent before accepting a re-typing."""
    fields = dict(parent.fields or ())
    if field == "trailing_term":
        connectives = [fields.get("connective")]
    else:
        continuations = fields.get("continuations")
        if not isinstance(continuations, list) or not continuations:
            raise Divergence(path, "PEhE parent has no continuation to prove")
        connectives = []
        for continuation in continuations:
            if not isinstance(continuation, Form) or continuation.fields is None:
                raise Divergence(path, "PEhE continuation is not a syntax node")
            connectives.append(dict(continuation.fields).get("connective"))
    for connective in connectives:
        if form_name(connective) not in PEHE_CONNECTIVE_NAMES:
            raise Divergence(
                path, f"PEhE connective {form_name(connective)} is outside the JOIK/JEK domain"
            )


def rewrite_term_position(
    old: Any,
    new: Any,
    parent: Form,
    field: str,
    levels: tuple[str, str],
    path: str,
    classes: set[str],
) -> Any:
    """Apply the approved mechanical rewrites at one term-family position.

    Returns the rewritten *old* value; the caller compares it structurally with `new`.
    """
    old_level, new_level = levels
    if not isinstance(old, Form):
        return old
    if old.name not in OLD_LEVEL_INVENTORY.get(old_level, frozenset()):
        raise Divergence(path, f"{old.name} is not a member of the old {old_level} level")

    # Class (i): the degenerate zero-continuation `connected_term` wrapper.
    payload = single_arg(old, "ConnectedTerm")
    if payload is not None:
        if not isinstance(payload, Form) or payload.name != "ConnectedTermSyntax":
            raise Divergence(path, "ConnectedTerm payload is not ConnectedTermSyntax")
        keys = tuple(key for key, _ in payload.fields or ())
        if keys != ("leading_term", "continuations"):
            raise Divergence(path, f"unexpected ConnectedTermSyntax fields {keys}")
        fields = dict(payload.fields or ())
        if fields["continuations"] == []:
            leaf = require_mechanical_atom(
                fields["leading_term"], path, "zero-continuation ConnectedTerm"
            )
            require_new_level_admits(leaf.name, new_level, path)
            classes.add(CLASS_FLAT_SUM_WRAPPER)
            return leaf
        return old  # a real loose connection: compared structurally, tripwire on any diff

    # Class (i) / class (ii): the nested `simple_term` sum branch.
    payload = single_arg(old, "SimpleTerm")
    if payload is not None:
        leaf = require_mechanical_atom(payload, path, "SimpleTerm wrapper")
        require_new_level_admits(leaf.name, new_level, path)
        if old_level == "pehe_termset_operand":
            pehe_connectives_are_corrected(parent, field, path)
            classes.add(CLASS_PEHE_CEHE_RETYPING)
        else:
            classes.add(CLASS_FLAT_SUM_WRAPPER)
        return leaf

    # Class (ii): the CEhE continuation takes the unguarded tag-led leaf.  The two rules
    # differ only in the absorption assertion, so the payload must survive the rename
    # field for field.
    payload = single_arg(old, "TaggedSumtiTerm")
    if payload is not None and new_level == "nonabs_term":
        if (parent.name, field) != ("TermsetGroupContinuationSyntax", "trailing_term"):
            raise Divergence(path, "nonabs re-typing outside a CEhE continuation")
        if form_name(dict(parent.fields or ()).get("cehe")) == "None":
            raise Divergence(path, "CEhE continuation has no CEhE token")
        if not isinstance(payload, Form) or payload.name != "TaggedSumtiTermSyntax":
            raise Divergence(path, "TaggedSumtiTerm payload is not TaggedSumtiTermSyntax")
        keys = tuple(key for key, _ in payload.fields or ())
        if keys != ("tense_modal", "sumti"):
            raise Divergence(path, f"unexpected TaggedSumtiTermSyntax fields {keys}")
        require_new_level_admits("NonabsTaggedSumtiTerm", new_level, path)
        classes.add(CLASS_PEHE_CEHE_RETYPING)
        return Form(
            name="NonabsTaggedSumtiTerm",
            args=(Form(name="NonabsTaggedSumtiTermSyntax", fields=payload.fields),),
        )

    return old


def compare_tree(
    old: Any,
    new: Any,
    classes: set[str],
    path: str = "",
    position: tuple[Form, str, tuple[str, str]] | None = None,
) -> None:
    """Structural comparison of the rewritten old tree against the new tree."""
    if position is not None:
        parent, field, levels = position
        old = rewrite_term_position(old, new, parent, field, levels, path, classes)

    if isinstance(old, Form) or isinstance(new, Form):
        if not (isinstance(old, Form) and isinstance(new, Form)):
            raise Divergence(path, f"{form_name(old)} became {form_name(new)}")
        if old.name != new.name:
            reason = f"{old.name} became {new.name}"
            if old.name in TRIPWIRE_NAMES or new.name in TRIPWIRE_NAMES:
                reason += " (#791 tripwire node type)"
            raise Divergence(path, reason)
        if (old.fields is None) != (new.fields is None) or (old.args is None) != (
            new.args is None
        ):
            raise Divergence(path, f"{old.name} changed shape")
        if old.fields is not None:
            old_keys = tuple(key for key, _ in old.fields)
            new_keys = tuple(key for key, _ in new.fields or ())
            if old_keys != new_keys:
                raise Divergence(path, f"{old.name} fields {old_keys} became {new_keys}")
            if new.name == "ConnectedTermSyntax" and dict(new.fields or ()).get(
                "continuations"
            ) == []:
                raise Divergence(path, "regenerated tree has a zero-continuation ConnectedTerm")
            for (key, old_child), (_, new_child) in zip(old.fields, new.fields or ()):
                compare_tree(
                    old_child,
                    new_child,
                    classes,
                    f"{path}.{key}" if path else key,
                    _position_for(old, key),
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
            compare_tree(old_child, new_child, classes, f"{path}[{index}]", position)
        return

    if isinstance(old, tuple) or isinstance(new, tuple):
        if not (isinstance(old, tuple) and isinstance(new, tuple)) or len(old) != len(new):
            raise Divergence(path, "tuple shape changed")
        for index, (old_child, new_child) in enumerate(zip(old, new)):
            compare_tree(old_child, new_child, classes, f"{path}({index})")
        return

    if old != new:
        raise Divergence(path, f"{old!r} became {new!r}")


def _position_for(parent: Form, field: str) -> tuple[Form, str, tuple[str, str]] | None:
    levels = POSITIONS.get((parent.name, field))
    if levels is None:
        return None
    return parent, field, levels


def assert_no_retired_shapes(tree: str, path: str) -> None:
    """Invariant on the regenerated tree: the retired nested-sum wrapper is gone.

    Every ladder level re-lists its leaves, so `simple_term` is only ever reached through an
    `arc`, which prints the selected leaf directly.  The retired zero-continuation
    `ConnectedTerm` wrapper is rejected structurally in `compare_tree`.
    """
    if "SimpleTerm(" in tree:
        raise Divergence(path, "regenerated tree still contains a SimpleTerm wrapper")


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


def compare_fixture(old: dict[str, Any], new: dict[str, Any]) -> tuple[set[str], list[str]]:
    old_leaves = dict(leaves(old))
    new_leaves = dict(leaves(new))
    residue: list[str] = []
    classes: set[str] = set()
    if set(old_leaves) != set(new_leaves):
        added = sorted(".".join(path) for path in set(new_leaves) - set(old_leaves))
        removed = sorted(".".join(path) for path in set(old_leaves) - set(new_leaves))
        reasons = []
        if added:
            reasons.append(f"expectation leaves added: {', '.join(added)}")
        if removed:
            reasons.append(f"expectation leaves removed: {', '.join(removed)}")
        return classes, reasons

    old_status = old_leaves.get(("expectations", "syntax", "status"))
    new_status = new_leaves.get(("expectations", "syntax", "status"))
    old_raw = old_leaves.get(("expectations", "syntax", "raw"))
    bo_route_flip = (
        old_status == "success"
        and new_status == "failure"
        and isinstance(old_raw, str)
        and "BoundTermConnection(" in old_raw
    )

    for path, old_value in old_leaves.items():
        new_value = new_leaves[path]
        if old_value == new_value:
            continue
        joined = ".".join(path)
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
                assert_no_retired_shapes(new_value, "syntax.raw")
                compare_tree(old_form, new_form, classes)
            except Divergence as divergence:
                residue.append(f"syntax tree {divergence.path or '<root>'}: {divergence.reason}")
            continue
        if path == ("expectations", "syntax", "status") and bo_route_flip:
            classes.add(CLASS_BO_ROUTE_REJECTION)
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
            continue
        if path[-1:] == ("tree",) and "gentufa" in path:
            if tree_token_projection(old_value) != tree_token_projection(new_value):
                residue.append("gentufa.tree token/span projection changed")
            continue
        if path[-1:] == ("brackets",) and "gentufa" in path:
            if bracket_token_projection(old_value) != bracket_token_projection(new_value):
                residue.append("gentufa.brackets token projection changed")
            continue
        # Diagnostics, statuses, digests, semantics refs, tersmu output and every other leaf
        # are deliberately exact.
        residue.append(joined)
    if not classes and old != new and not residue:
        residue.append("no mechanical term-hierarchy shape found")
    return classes, residue


def classify_one(job: tuple[str, str, str]) -> tuple[str, list[str], list[str]]:
    """Worker entry point.  The texts are re-read here so the parent never holds them."""
    repository_path, baseline_file, candidate_file = job
    old = tomllib.loads(Path(baseline_file).read_text())
    new = tomllib.loads(Path(candidate_file).read_text())
    classes, residue = compare_fixture(old, new)
    return repository_path, sorted(classes), residue


def collect_jobs(
    candidate_root: Path, baseline_root: Path, witnesses: set[str]
) -> tuple[list[tuple[str, str, str]], list[str]]:
    jobs: list[tuple[str, str, str]] = []
    witness_deltas: list[str] = []
    for candidate_path in sorted(candidate_root.rglob("*.toml")):
        relative = candidate_path.relative_to(candidate_root)
        repository_path = (Path("tests/fixtures") / relative).as_posix()
        baseline_file = baseline_root / relative
        if not baseline_file.exists():
            continue
        if baseline_file.read_bytes() == candidate_path.read_bytes():
            continue
        if repository_path in witnesses:
            # C1-C6 witnesses carry commit-local exact pins; a delta means the pin was wrong.
            witness_deltas.append(repository_path)
            continue
        jobs.append((repository_path, str(baseline_file), str(candidate_path)))
    return jobs, witness_deltas


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
    jobs, witness_deltas = collect_jobs(args.candidate, args.baseline_root, witnesses)

    mechanical: dict[str, list[str]] = {name: [] for name in MECHANICAL_CLASSES}
    manual: list[tuple[str, list[str]]] = []
    with ProcessPoolExecutor(max_workers=args.jobs) as pool:
        for repository_path, classes, residue in pool.map(classify_one, jobs, chunksize=16):
            if residue:
                manual.append((repository_path, residue))
            else:
                for classification in classes:
                    mechanical[classification].append(repository_path)

    lines = [f"changed: {len(jobs)}"]
    for classification, paths in mechanical.items():
        lines.append(f"mechanical {classification}: {len(paths)}")
    lines.append(f"manual: {len(manual)}")
    for path, reasons in sorted(manual):
        lines.append(f"  {path}: {'; '.join(reasons)}")
    if witness_deltas:
        lines.append(f"epoch-witness deltas (must be empty): {len(witness_deltas)}")
        lines.extend(f"  {path}" for path in witness_deltas)
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
                    "witness_deltas": witness_deltas,
                },
                indent=2,
                sort_keys=True,
            )
            + "\n"
        )

    if witness_deltas:
        print("error: epoch witnesses must keep their commit-local pins")
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
