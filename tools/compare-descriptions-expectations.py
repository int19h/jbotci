#!/usr/bin/env python3
"""Fail-closed classifier for the epoch-9 description/quantifier expectation regeneration.

The baseline archive is `ARCHIVE_COMMIT`, this epoch's implementation base, so every class
earlier epochs contributed is already applied in the baseline and has nothing left to find.

Usage (from the repository root):

    python3 tools/compare-descriptions-expectations.py tests/fixtures \\
        --baseline-root /build/jbotci/scratch/epoch09-impl/baseline-fixtures

`--baseline-root` is an archive of `tests/fixtures` taken at the epoch base; the positional
argument is the epoch's tree.  The classifier reads the *old* Rust-Debug tree and rewrites it
with exactly the mechanical shapes the epoch plan approves.  The rewritten old tree must then
equal the new tree structurally; anything else is emitted as manual residue for an individual
ledger disposition.  Nothing is inferred from the new tree, from fixture text, or from a
span-only comparison, so an ownership change can never be laundered as a re-typing.

Epoch 9 declares exactly two mechanical classes, both of them #634's:

Class `quantifier-retyping` (#634) — baseline quantifier surfaces return to the baseline route
    A completed priority raw-mex quantifier whose whole mex extent is exactly one of the two
    surfaces the baseline `quantifier` already owns is refused by the classifier, and strict
    ordered choice reparses it through the baseline alternative:

        ZantufaPriorityRawMeksoQuantifier(ZantufaPriorityRawMeksoQuantifierSyntax(
            InfixMekso(<one NumberMekso operand, every continuation slot empty>)))
          -> PaRunQuantifier(<the same PaRunQuantifierSyntax payload>)

        ZantufaPriorityRawMeksoQuantifier(ZantufaPriorityRawMeksoQuantifierSyntax(
            InfixMekso(<one ParenthesizedMeksoOperand, every continuation slot empty>)))
          -> MeksoQuantifier(MeksoQuantifierSyntax { vei, mekso, veho })

    Every continuation slot of the spine must be empty, because a non-empty one is a mex the
    baseline quantifier route cannot form; such a candidate is a genuine raw mex, keeps its
    priority ownership, and is left unrewritten so that any change to it becomes residue.

Class `quantifier-warning-removal` (#634) — the false warnings the re-typing removes
    The `experimental-zantufa-mex` diagnostics the re-typed positions anchored disappear with the
    ownership.  Accepted only when every OTHER diagnostic is identical and in order: a warning of
    any other kind appearing or disappearing takes the fixture out of the class, and so does a
    diagnostic moving span or severity.

What is deliberately NOT mechanical
    Every acceptance flip, every owner change and every other diagnostic change is manual residue
    with an individual ledger disposition: D4.1's deleted head-connective route, D1's and D3's
    new acceptances, and the two still-rejecting surfaces whose error frontier follows a route
    that no longer exists.  D0's tier restriction declares a comparer multiplicity of zero and
    that zero is STRUCTURAL only -- `description_leading_operand` produces `SumtiBaseSyntax` and
    adds no tree layer, so no serialized node can change SHAPE because of it -- while its
    behavioural effect is the separately measured C-a recovered-delta enumeration.  D3c's
    absent-field rewrite is zero because the frozen sibling-variant shape shipped: no existing
    quantifier node gains a field.

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

EPOCH_BASE = "9ec321d530"
# The commit whose `tests/fixtures` tree the baseline archive reproduces byte for byte.
ARCHIVE_COMMIT = "9ec321d530"

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



CLASS_QUANTIFIER_RETYPING = "quantifier-retyping"
CLASS_QUANTIFIER_WARNING_REMOVAL = "quantifier-warning-removal"
MECHANICAL_CLASSES = (
    CLASS_QUANTIFIER_RETYPING,
    CLASS_QUANTIFIER_WARNING_REMOVAL,
)

# --- the shapes, transcribed from the grammar ------------------------------------------
#
# Every entry below was read off `crates/jbotci-syntax/src/grammar/generated.rs` at HEAD and at
# `ARCHIVE_COMMIT`, resolved by production NAME rather than by line number, and the old field
# tuples are the exact ones the baseline macro rendered.

ZPRI_ARM = "ZantufaPriorityRawMeksoQuantifier"
ZPRI_STRUCT = "ZantufaPriorityRawMeksoQuantifierSyntax"
INFIX_ARM = "InfixMekso"
INFIX_STRUCT = "InfixMeksoSyntax"
INFIX_FIELDS = ("first_expression", "continuations")
PRECEDENCE_STRUCT = "MeksoPrecedenceSyntax"
PRECEDENCE_FIELDS = ("left_expression", "tail")
OPERAND_ARM = "MeksoOperand"
OPERAND_STRUCT = "MeksoOperandSyntax"
OPERAND_FIELDS = ("connected_expression", "grouped_continuation")
CHAIN_ARM = "AfterthoughtMeksoOperandSyntax"
CHAIN_STRUCT = "Chain"
CHAIN_FIELDS = ("first", "links")
SIMPLE_ARM = "SimpleMeksoOperand"
NUMBER_ARM = "NumberMekso"
NUMBER_STRUCT = "NumberMeksoSyntax"
PARENTHESIZED_ARM = "ParenthesizedMeksoOperand"
PARENTHESIZED_STRUCT = "ParenthesizedMeksoOperandSyntax"
PARENTHESIZED_FIELDS = ("vei", "inner_expression", "veho")
PA_RUN_ARM = "PaRunQuantifier"
MEKSO_QUANTIFIER_ARM = "MeksoQuantifier"
MEKSO_QUANTIFIER_STRUCT = "MeksoQuantifierSyntax"

ZANTUFA_MEX_CODE = "syntax.warning.experimental-zantufa-mex"

# Nodes this epoch retires.  A regenerated tree that still contains one means a population moved
# without being dispositioned.  The check runs on the regenerated TEXT, so it fires even where
# the baseline carried the same node and the leaf would otherwise compare equal.
RETIRED_SHAPES: tuple[str, ...] = (
    "DescriptionConnectionSumti",
    "DescriptionHeadConnective",
)

# The reviewed regeneration result.  A re-run against the same archive must reproduce it exactly;
# the `--expect-*` flags override the pins for an exploratory run.
EXPECTED_CHANGED = 8
EXPECTED_MECHANICAL: dict[str, int] = {
    # `corpus/alis/full-alice` plus the two epoch-5 #828 selbri witnesses: the only pre-epoch
    # fixtures whose trees hold a priority raw-mex quantifier over a baseline surface.
    CLASS_QUANTIFIER_RETYPING: 3,
    # `full-alice` pins no diagnostics at all, so only the two #828 witnesses record the warning.
    CLASS_QUANTIFIER_WARNING_REMOVAL: 2,
}
EXPECTED_MANUAL = 5
EXPECTED_PROSE = 0
EXPECTED_NEW_WITNESSES = 108

# The two manual residues that still REJECT at the head.  Their WINNING diagnostic moves, and a
# moved error frontier is exactly what a count cannot police: the signatures are pinned on both
# sides, so a different move at the same fixture is a comparer failure rather than unnamed
# residue.  Both moves were bisected across the epoch's commits and both land at C-d, on the
# DEFAULT axis, which is D3a.
#
# `corpus/camxes/21995` -- `noltroní'u pa la ce`.  The base reports TWO diagnostics: the
# description route's frontier at `ce` (`unexpected-cmavo` 29..31, whose expected set advertises
# the `quantifier` candidate D0 removes from the leading operand) and the mex route's
# `incomplete-mekso` at end of input.  The head reports the SECOND one, unchanged in code and in
# byte offset: no new frontier is invented, the shallower candidate simply stops being the
# reported one once C-d adds a sibling description-tail route at the same position.
#
# `corpus/camxes/5170` -- `... le tercru be me'e la bysydy .e la xypapa`.  The base stops at the
# `.e` connective (`unexpected-cmavo` 41..42) because the baseline leading operand is one
# `sumti_6` and cannot carry a connection.  D3a's `exp_full_sumti_description_tail` admits a FULL
# sumti there, so the connection is consumed and the frontier moves to end of input, where the
# description tail body is still missing (`incomplete-sumti` 52..52).  The new frontier is correct
# precisely because the extent that used to be unparsable now parses as far as the missing tail.
PINNED_DIAGNOSTIC_MOVES: dict[str, tuple[tuple[tuple[str, int, int], ...], tuple[tuple[str, int, int], ...]]] = {
    "tests/fixtures/corpus/camxes/21995.toml": (
        (("syntax.unexpected-cmavo", 29, 31),),
        (("syntax.incomplete-mekso", 31, 31),),
    ),
    "tests/fixtures/corpus/camxes/5170.toml": (
        (("syntax.unexpected-cmavo", 41, 42),),
        (("syntax.incomplete-sumti", 52, 52),),
    ),
}

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


def _single_simple_operand(mekso: Any) -> Any | None:
    """The one simple operand of a lone-operand `infix_mekso`, or `None`.

    Every continuation slot on the way down must be empty and every field tuple must be the exact
    one the macro renders, so a mex whose shape has moved falls out of the class rather than
    being rewritten on a partial match.
    """
    if not (isinstance(mekso, Form) and mekso.name == INFIX_ARM and mekso.args
            and len(mekso.args) == 1):
        return None
    body = mekso.args[0]
    if not (isinstance(body, Form) and body.name == INFIX_STRUCT and body.fields
            and field_names(body) == INFIX_FIELDS):
        return None
    values = dict(body.fields)
    if values["continuations"] != []:
        return None
    precedence = values["first_expression"]
    if not (isinstance(precedence, Form) and precedence.name == PRECEDENCE_STRUCT
            and precedence.fields and field_names(precedence) == PRECEDENCE_FIELDS):
        return None
    precedence_values = dict(precedence.fields)
    if not is_none(precedence_values["tail"]):
        return None
    left = precedence_values["left_expression"]
    if not (isinstance(left, Form) and left.name == OPERAND_ARM and left.args
            and len(left.args) == 1):
        return None
    operand = left.args[0]
    if not (isinstance(operand, Form) and operand.name == OPERAND_STRUCT and operand.fields
            and field_names(operand) == OPERAND_FIELDS):
        return None
    operand_values = dict(operand.fields)
    if not is_none(operand_values["grouped_continuation"]):
        return None
    chain = operand_values["connected_expression"]
    if not (isinstance(chain, Form) and chain.name == CHAIN_ARM and chain.args
            and len(chain.args) == 1):
        return None
    chain_body = chain.args[0]
    if not (isinstance(chain_body, Form) and chain_body.name == CHAIN_STRUCT and chain_body.fields
            and field_names(chain_body) == CHAIN_FIELDS):
        return None
    chain_values = dict(chain_body.fields)
    if chain_values["links"] != []:
        return None
    simple = chain_values["first"]
    if not (isinstance(simple, Form) and simple.name == SIMPLE_ARM and simple.args
            and len(simple.args) == 1):
        return None
    return simple.args[0]


def _retyped_quantifier(old: Any) -> Any | None:
    """The baseline quantifier a priority raw-mex candidate returns to, or `None`."""
    if not (isinstance(old, Form) and old.name == ZPRI_ARM and old.args and len(old.args) == 1):
        return None
    wrapper = old.args[0]
    if not (isinstance(wrapper, Form) and wrapper.name == ZPRI_STRUCT and wrapper.args
            and len(wrapper.args) == 1):
        return None
    operand = _single_simple_operand(wrapper.args[0])
    if operand is None:
        return None
    if (isinstance(operand, Form) and operand.name == NUMBER_ARM and operand.args
            and len(operand.args) == 1):
        number = operand.args[0]
        if (isinstance(number, Form) and number.name == NUMBER_STRUCT and number.args
                and len(number.args) == 1):
            return Form(name=PA_RUN_ARM, args=(number.args[0],))
        return None
    if (isinstance(operand, Form) and operand.name == PARENTHESIZED_ARM and operand.args
            and len(operand.args) == 1):
        payload = operand.args[0]
        if (isinstance(payload, Form) and payload.name == PARENTHESIZED_STRUCT and payload.fields
                and field_names(payload) == PARENTHESIZED_FIELDS):
            values = dict(payload.fields)
            return Form(
                name=MEKSO_QUANTIFIER_ARM,
                args=(Form(name=MEKSO_QUANTIFIER_STRUCT, fields=(
                    ("vei", values["vei"]),
                    ("mekso", values["inner_expression"]),
                    ("veho", values["veho"]),
                )),),
            )
        return None
    return None


def rewrite_node(old: Any, path: str, classes: set[str]) -> Any:
    """Apply the approved node-shape rewrites to one OLD node.

    The rewrite is keyed on the baseline node's own name and its exact baseline field tuples, so a
    node whose shape does not match the transcribed one falls through unchanged and is compared
    structurally -- which turns an unexpected shape into manual residue rather than a laundered
    rewrite.  A priority raw-mex quantifier over a GENUINE raw mex is left alone deliberately: it
    keeps its ownership, and any change to it is therefore residue.
    """
    if not isinstance(old, Form):
        return old
    if old.name == ZPRI_ARM:
        retyped = _retyped_quantifier(old)
        if retyped is None:
            return old
        classes.add(CLASS_QUANTIFIER_RETYPING)
        return retyped
    return old


def rewrite_position(old: Any, parent: Form, field: str, path: str, classes: set[str]) -> Any:
    """Epoch 9 has no position-keyed rewrite: the shape it moves is keyed on its own name."""
    return old


def quantifier_warnings_removed(old: Any, new: Any) -> bool:
    """True when the only diagnostic delta is the removal of `experimental-zantufa-mex` entries.

    Every other diagnostic must be identical and in order, so a warning of another kind moving, a
    span moving or a severity changing takes the fixture out of the class.
    """
    if not (isinstance(old, list) and isinstance(new, list)):
        return False
    kept = [entry for entry in old if entry.get("code") != ZANTUFA_MEX_CODE]
    return len(kept) != len(old) and kept == new

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
    # fixture whose only move is an arm split showing through its gentufa JSON would fall into
    # the "no mechanical shape found" backstop and read as residue.
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
        if path == ("expectations", "syntax", "diagnostics") and quantifier_warnings_removed(
            old_value, new_value
        ):
            classes.add(CLASS_QUANTIFIER_WARNING_REMOVAL)
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
        residue.append("no mechanical description or quantifier shape found")
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

    move_failures = pinned_diagnostic_move_failures(args.candidate, args.baseline_root)
    if move_failures:
        for failure in move_failures:
            print(f"error: {failure}")
        return 1
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


def syntax_diagnostic_signatures(document: dict[str, Any]) -> tuple[tuple[str, int, int], ...]:
    """The (code, byte-start, byte-end) signature of every pinned strict syntax diagnostic."""
    syntax = document.get("expectations", {}).get("syntax", {})
    return tuple(
        (item["code"], item["byte-span"][0], item["byte-span"][1])
        for item in syntax.get("diagnostics", ())
    )


def pinned_diagnostic_move_failures(candidate_root: Path, baseline_root: Path) -> list[str]:
    """Check every pinned diagnostic move against both trees, fail-closed in both directions."""
    failures: list[str] = []
    for repository_path, (expected_old, expected_new) in PINNED_DIAGNOSTIC_MOVES.items():
        relative = Path(repository_path).relative_to("tests/fixtures")
        for root, expected, side in (
            (baseline_root, expected_old, "baseline"),
            (candidate_root, expected_new, "candidate"),
        ):
            path = root / relative
            if not path.exists():
                failures.append(f"{repository_path}: absent from the {side} tree")
                continue
            actual = syntax_diagnostic_signatures(tomllib.loads(path.read_text()))
            if actual != expected:
                failures.append(
                    f"{repository_path}: {side} diagnostics are {actual}, "
                    f"pinned as {expected}"
                )
    return failures


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
