#!/usr/bin/env python3
"""Fail-closed classifier for the epoch-7 bridi-tail expectation regeneration.

The baseline archive is `ARCHIVE_COMMIT`, this epoch's implementation base, so every class
earlier epochs contributed is already applied in the baseline and has nothing left to find.  Epoch 7 contributes five classes of its own, and one population that
is listed rather than classified: fixtures this epoch ADDED, which have no baseline entry to
compare against.

Usage (from the repository root, after the consolidated `fixture-rewrite` passes):

    python3 tools/compare-bridi-tail-expectations.py tests/fixtures \\
        --baseline-root /build/jbotci/scratch/07/baseline-fixtures

`--baseline-root` is an archive of `tests/fixtures` taken *before* the rewrite; the positional
argument is the rewritten tree.  The classifier reads the *old* Rust-Debug tree and rewrites
it with exactly the mechanical shapes the epoch plan approves.  The rewritten old tree must
then equal the new tree structurally; anything else is emitted as manual residue for an
individual ledger disposition.  Nothing is inferred from the new tree, from fixture text, or
from a span-only comparison, so an ownership change can never be laundered as a re-typing.

The epoch's shape moves are all field- and arm-level inside productions that keep their names
-- the plan's C7 same-name table resolves 1:1 against the base by construction -- so the five
classes below are field- and arm-level too.  There is no path-level class at all, and a node
that changed its position in the tree is residue by definition.

Class `statement-continuation-collapse` (#805) — the retired I-less statement envelopes
    `bridi_statement` kept its name and its one child when D1 deleted `bridi_statement_continuation`,
    and the macro renders a single-child product transparently, so the node goes from the
    two-field struct to the tuple form:

        BridiStatementSyntax { bridi: X, continuations: [] }  ->  BridiStatementSyntax(X)

    Fail-closed on the list being EMPTY and on `X` surviving verbatim.  A non-empty
    `continuations` list is a D1 acceptance FLIP -- one of the two envelopes actually parsed
    there -- and has no collapsed tree for this rewrite to produce, so it stays residue and
    takes an individual disposition.  This is the epoch's one bulk class: nearly every fixture
    in the corpus has a statement.

Class `bt2-leading-cu-field` (#815) — camxes-exp's `bridi_tail_2` leading CU
    `bo_grouped_bridi_tail` and its tail-terms-free mirror gain the leading-CU field at the
    head of their field list (camxes-exp.peg:107):

        BoGroupedBridiTailSyntax { first, bo_continuation }
            -> BoGroupedBridiTailSyntax { cu: None, first, bo_continuation }

    The field is INSERTED, never matched: the baseline grammar has no CU at this level, so a
    baseline tree can only ever have carried its absence.  A regenerated `cu` that is anything
    but `None` is therefore a parse the baseline could not have produced, and diverges rather
    than classifying -- which is what keeps the class from absorbing an adoption.

Class `tail-joint-cu-drop` (#805/#815) — the joints' own CU slots
    D1 deletes the CU field from the flat and BO joints, both families, because camxes-exp's
    CU at that surface position is the right operand's own leading `bridi_tail_2` CU, which
    the class above adds:

        BridiTailContinuationSyntax { connective, cu, bridi_tail, tail_terms, vau }
            -> BridiTailContinuationSyntax { connective, bridi_tail, tail_terms, vau }

    and the same drop on `BridiTailContinuationWithoutTailTermsSyntax`,
    `BridiTailBoContinuationSyntax` and `BridiTailBoContinuationWithoutTailTermsSyntax`.

    Accepted ONLY where the baseline value is `None`.  A baseline tree that actually parsed a
    joint CU is a surface whose ownership moved -- the CU is now the operand's, one level down
    -- and dropping the field would erase the evidence, so it is residue.

Class `bo-joint-sum-wrapper` (#826) — the BO joint that became a sum
    Rolling Zantufa admits a connectiveless `tag BO` joint, which no sourced grammar does, so
    the BO continuation position now holds a two-arm sum: the sourced product and the Zantufa
    arm.  Both families move, and the rewrite is a pure one-to-one wrap:

        BoGroupedBridiTailSyntax.bo_continuation:
            Some(BridiTailBoContinuationSyntax {..})
                -> Some(BridiTailBoContinuation(BridiTailBoContinuationSyntax {..}))
        BoGroupedBridiTailWithoutTailTermsSyntax.bo_continuation:
            Some(BridiTailBoContinuationWithoutTailTermsSyntax {..})
                -> Some(BridiTailBoContinuationWithoutTailTerms(...))

    Only the sourced product wraps.  The Zantufa arm cannot appear on the baseline side at all
    -- the baseline grammar has no such node -- so an old value that is already wrapped, or
    wrapped in the Zantufa arm, is residue.  This is the shape epoch 6c's own table blessed one
    tier down, under the same name.

Class `ke-join-gihek-narrowing` (#805) — the KE join is GIhA's alone
    D1 narrows the tail-terms-free family's KE join from the widened shared connective to the
    GIhA-only one its with-tail-terms sibling already had at the base.  Rolling Zantufa spells
    no KE join at this level at all, so nothing is lost:

        BridiTailWithoutTailTermsSyntax.ke_continuation:
            Some(BridiTailKeContinuationSyntax { connective: GihekConnective(G), .. })
                -> Some(GihekBridiTailKeContinuationSyntax { connective: G, .. })

    Two things move together and both are proven: the node is renamed, and the connective loses
    its sum-arm wrapper because the narrowed production names `gihek_connective` directly.
    Accepted ONLY where the baseline selected the `GihekConnective` arm, with every other field
    carried across verbatim; a baseline JOIK-, JEK- or EK-led KE join is a D1 acceptance flip
    and stays residue.

Retired shapes — refused outright on the regenerated side
    The classifier refuses any regenerated tree that still contains a node this epoch retires:
    `BridiStatementContinuation` and its two arms, `RelationConnectiveAsBridiTail`,
    `RelationAfterthoughtConnective`, `BridiTailKeContinuationSyntax`, `BridiWithPostCuTerms`,
    `BareCuTermsBridi` and `CuTermsBridiTail`.  A surviving instance would mean a population
    moved without being dispositioned.  The check is on the regenerated text, so it fires even
    where the baseline had the same node and the leaf would otherwise compare equal.

What is deliberately NOT mechanical
    Every acceptance flip, every warning change and every owner change is manual residue with
    an individual ledger disposition, however large its population:

    * the four D1-deleted routes and their flipped surfaces (`relation_connective_as_bridi_tail`
      at every tail joint; the two I-less statement envelopes; the widened KE join);
    * the retired single-outer-group camxes-exp family (`bridi_with_post_cu_terms`,
      `bare_cu_terms_bridi`, `cu_terms_bridi_tail`), which the plan names as a dedicated
      manual-reviewed class;
    * the `bare_cu_bridi` CU warning, the C-e stop-and-ask resolution: a warning change on a
      pre-existing node, so its whole population is manual by construction;
    * the KE-tail ownership guard, which returns baseline-owned KE bodies to the baseline group;
    * the `zantufa_priority_grouped_bridi_tail` wrapper, which is an ownership *filter* rather
      than a rendering change and is therefore never laundered as one.

The baseline archive is exactly `git archive 67cc7e4b5a tests/fixtures` (`ARCHIVE_COMMIT`), the
fixture tree at the #870 tersmu-retirement merge this epoch was rebased onto, so it is
reproducible rather than hand-assembled.  Every
candidate fixture must have a baseline entry and every baseline entry a candidate: an unpaired
fixture on either side is a hard error, never a skip.  The one exception is a fixture this epoch
ADDED, which is identified from `git diff --diff-filter=A EPOCH_BASE..HEAD` rather than from its
mere absence, is reported in its own pinned list, and is never classified.

The one value the classifier does not compare exactly is the `description` prose of a
`provenance` entry, which carries no expectation.  Every other provenance field stays exact, and
a fixture whose prose moved is listed in the report with its number pinned like the mechanical
classes, so an unreviewed prose edit still fails the run.

Epoch-new witnesses are not classified, so the only thing standing behind them is what they pin.
A witness that omits `expectations.syntax.diagnostics` pins its tree and leaves its warning
stream unspecified, which is how a construct can quietly stop warning -- or start -- without any
expectation moving.  This is the epoch that adds a warning to a pre-existing node, so the check
matters more here than where 6c ported it from: every epoch-new witness must carry the key, empty
where the expectation is silence, and one that does not is a hard error rather than a count.
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

EPOCH_BASE = "67cc7e4b5a"
# The commit whose `tests/fixtures` tree the baseline archive reproduces byte for byte.  Epoch 7
# baselines to its own implementation base, which round 2 moved from the #862 merge to the #870
# tersmu-retirement merge, so every class earlier epochs contributed is already applied in the
# baseline and has nothing left to find.  Baselining to the post-retirement tree is also what
# keeps `[expectations.output.tersmu]` out of the comparison entirely: the block is absent on
# both sides, so its removal is not an expectation change and never reaches a class.
ARCHIVE_COMMIT = "67cc7e4b5a"


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


CLASS_STATEMENT_COLLAPSE = "statement-continuation-collapse"
CLASS_BT2_LEADING_CU = "bt2-leading-cu-field"
CLASS_TAIL_JOINT_CU_DROP = "tail-joint-cu-drop"
CLASS_BO_JOINT_SUM_WRAPPER = "bo-joint-sum-wrapper"
CLASS_KE_JOIN_NARROWING = "ke-join-gihek-narrowing"
MECHANICAL_CLASSES = (
    CLASS_STATEMENT_COLLAPSE,
    CLASS_BT2_LEADING_CU,
    CLASS_TAIL_JOINT_CU_DROP,
    CLASS_BO_JOINT_SUM_WRAPPER,
    CLASS_KE_JOIN_NARROWING,
)

# --- the shapes, transcribed from the grammar ------------------------------------------
#
# Every entry below was read off `crates/jbotci-syntax/src/grammar/generated.rs` at HEAD and at
# `ARCHIVE_COMMIT`, resolved by production NAME rather than by line number, and the old field
# tuples are the exact ones the baseline macro rendered.

# The single-child collapse: (Debug struct, old field tuple, the field the tuple form keeps).
COLLAPSE_STRUCT = "BridiStatementSyntax"
COLLAPSE_OLD_FIELDS = ("bridi", "continuations")
COLLAPSE_PAYLOAD_FIELD = "bridi"
COLLAPSE_EMPTY_FIELD = "continuations"

# The `bridi_tail_2` leading CU: struct -> the old field tuple it is inserted in front of.
LEADING_CU_STRUCTS: dict[str, tuple[str, ...]] = {
    "BoGroupedBridiTailSyntax": ("first", "bo_continuation"),
    "BoGroupedBridiTailWithoutTailTermsSyntax": ("first", "bo_continuation"),
}

# The joints that lose their own CU: struct -> the old field tuple, `cu` included.
JOINT_CU_DROP_STRUCTS: dict[str, tuple[str, ...]] = {
    "BridiTailContinuationSyntax": ("connective", "cu", "bridi_tail", "tail_terms", "vau"),
    "BridiTailContinuationWithoutTailTermsSyntax": ("connective", "cu", "bridi_tail"),
    "BridiTailBoContinuationSyntax": (
        "connective",
        "tense_modal",
        "bo",
        "cu",
        "bridi_tail",
        "tail_terms",
        "vau",
    ),
    "BridiTailBoContinuationWithoutTailTermsSyntax": (
        "connective",
        "tense_modal",
        "bo",
        "cu",
        "bridi_tail",
    ),
}

# The BO-joint sum wrap: (parent Debug struct, field) -> (sum arm, sourced product).
BO_JOINT_POSITIONS: dict[tuple[str, str], tuple[str, str]] = {
    ("BoGroupedBridiTailSyntax", "bo_continuation"): (
        "BridiTailBoContinuation",
        "BridiTailBoContinuationSyntax",
    ),
    ("BoGroupedBridiTailWithoutTailTermsSyntax", "bo_continuation"): (
        "BridiTailBoContinuationWithoutTailTerms",
        "BridiTailBoContinuationWithoutTailTermsSyntax",
    ),
}

# The KE-join narrowing.  `bridi_tail_ke_continuation` had exactly ONE consumer in the baseline
# grammar -- the tail-terms-free family's `ke_continuation` -- and the with-tail-terms family
# already named the GIhA-only production there, so this is the only position that moves.
KE_JOIN_POSITION = ("BridiTailWithoutTailTermsSyntax", "ke_continuation")
KE_JOIN_OLD_STRUCT = "BridiTailKeContinuationSyntax"
KE_JOIN_NEW_STRUCT = "GihekBridiTailKeContinuationSyntax"
KE_JOIN_FIELDS = ("connective", "tense_modal", "ke", "bridi_tail", "kehe", "tail_terms", "vau")
GIHEK_CONNECTIVE_ARM = "GihekConnective"

# Nodes this epoch retires.  A regenerated tree that still contains one means a population moved
# without being dispositioned, so the check runs on the regenerated TEXT and fires even where the
# baseline carried the same node and the leaf would otherwise compare equal.
RETIRED_SHAPES: tuple[str, ...] = (
    "BridiStatementContinuation",
    "BoBridiStatementContinuation",
    "KeBridiStatementContinuation",
    "RelationConnectiveAsBridiTail",
    "RelationAfterthoughtConnective",
    "BridiTailKeContinuationSyntax",
    "BridiWithPostCuTerms",
    "BareCuTermsBridi",
    "CuTermsBridiTail",
)

# The reviewed regeneration result.  A re-run against the same archive must reproduce it exactly;
# the `--expect-*` flags override the pins for an exploratory run.
EXPECTED_CHANGED = 19769
EXPECTED_MECHANICAL: dict[str, int] = {
    CLASS_STATEMENT_COLLAPSE: 19286,
    CLASS_BT2_LEADING_CU: 19695,
    CLASS_TAIL_JOINT_CU_DROP: 456,
    CLASS_BO_JOINT_SUM_WRAPPER: 33,
    # Zero, and the zero is the measurement: the only baseline tree that reached the
    # tail-terms-free family's KE join is `issue-840-jek-tag-ke-bridi-tail-residual`, whose join
    # is JEK-led and therefore an acceptance flip.  The narrowing itself is witnessed by
    # `adhoc/syntax/bridi-tail/baseline-gihek-ke-join.toml` and by the retired-shape invariant.
    CLASS_KE_JOIN_NARROWING: 0,
}
EXPECTED_MANUAL = 73
EXPECTED_PROSE = 0
EXPECTED_NEW_WITNESSES = 36


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


def rewrite_node(old: Any, path: str, classes: set[str]) -> Any:
    """Apply the approved node-shape rewrites to one OLD node.

    Every rewrite is keyed on the baseline node's own name and its exact baseline field tuple,
    so a node whose fields do not match the transcribed shape falls through unchanged and is
    compared structurally -- which turns an unexpected shape into manual residue rather than a
    laundered rewrite.
    """
    if not isinstance(old, Form) or old.fields is None:
        return old

    if old.name == COLLAPSE_STRUCT and field_names(old) == COLLAPSE_OLD_FIELDS:
        values = dict(old.fields)
        if values[COLLAPSE_EMPTY_FIELD] != []:
            raise Divergence(
                path,
                "bridi_statement carries a continuation, so the D1 deletion is an acceptance "
                "flip rather than the single-child collapse",
            )
        classes.add(CLASS_STATEMENT_COLLAPSE)
        return Form(name=COLLAPSE_STRUCT, args=(values[COLLAPSE_PAYLOAD_FIELD],))

    expected = LEADING_CU_STRUCTS.get(old.name)
    if expected is not None and field_names(old) == expected:
        classes.add(CLASS_BT2_LEADING_CU)
        return Form(name=old.name, fields=(("cu", NONE_FORM), *old.fields))

    expected = JOINT_CU_DROP_STRUCTS.get(old.name)
    if expected is not None and field_names(old) == expected:
        values = dict(old.fields)
        if not is_none(values["cu"]):
            raise Divergence(
                path,
                f"{old.name} parsed a joint CU, so the D1 deletion moves its ownership to the "
                "operand rather than dropping an absent field",
            )
        classes.add(CLASS_TAIL_JOINT_CU_DROP)
        return Form(
            name=old.name, fields=tuple((key, value) for key, value in old.fields if key != "cu")
        )

    return old


def rewrite_position(old: Any, parent: Form, field: str, path: str, classes: set[str]) -> Any:
    """Apply the approved position-keyed rewrites at one child position.

    `Some` is unwrapped and re-wrapped so an optional field is treated exactly as its payload;
    every other value falls through unchanged.
    """
    if not isinstance(old, Form):
        return old
    if old.name == "Some" and old.args is not None and len(old.args) == 1:
        inner = rewrite_position(old.args[0], parent, field, path, classes)
        return old if inner is old.args[0] else Form(name="Some", args=(inner,))

    wrap = BO_JOINT_POSITIONS.get((parent.name, field))
    if wrap is not None:
        arm, product = wrap
        if old.name == product:
            classes.add(CLASS_BO_JOINT_SUM_WRAPPER)
            return Form(name=arm, args=(old,))
        return old

    if (parent.name, field) == KE_JOIN_POSITION and old.name == KE_JOIN_OLD_STRUCT:
        if field_names(old) != KE_JOIN_FIELDS:
            raise Divergence(path, f"unexpected {KE_JOIN_OLD_STRUCT} fields {field_names(old)}")
        values = dict(old.fields or ())
        connective = values["connective"]
        if not (
            isinstance(connective, Form)
            and connective.name == GIHEK_CONNECTIVE_ARM
            and connective.args is not None
            and len(connective.args) == 1
        ):
            raise Divergence(
                path,
                f"the baseline KE join selected {form_name(connective)}, which the GIhA-only "
                "join cannot hold, so the narrowing is an acceptance flip",
            )
        classes.add(CLASS_KE_JOIN_NARROWING)
        return Form(
            name=KE_JOIN_NEW_STRUCT,
            fields=tuple(
                (key, connective.args[0] if key == "connective" else value)
                for key, value in old.fields or ()
            ),
        )

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


def compare_fixture(old: dict[str, Any], new: dict[str, Any]) -> tuple[set[str], list[str], bool]:
    old_leaves = dict(leaves(old))
    new_leaves = dict(leaves(new))
    residue: list[str] = []
    classes: set[str] = set()
    prose = False
    # Gentufa renderings are compared by token/span projection rather than by shape, because
    # they are a rendering of the tree and not an expectation about it.  A projection that
    # holds is an ACCEPTED delta, not an unexplained one, so it is recorded here: without it a
    # fixture whose only move is the statement collapse showing through its gentufa JSON would
    # fall into the "no mechanical shape found" backstop and read as residue.
    projected = False
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
        residue.append("no mechanical bridi-tail shape found")
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
