#!/usr/bin/env python3
"""Fail-closed classifier for epoch-5 selbri expectation regeneration.

The classifier deliberately reads the *old* Rust-Debug tree instead of
inferring a class from the new tree or from fixture text.  Only the four
mechanical shapes approved by the epoch plan are accepted.  Every other delta
is emitted as manual residue for an individual ledger disposition.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
from pathlib import Path
import re
import subprocess
import tomllib
from typing import Any, Iterator


EPOCH_BASE = "9fafb66d4a"
C6_WITNESS_REFRESH = {
    "tests/fixtures/adhoc/syntax/selbri/issue-829-cei-extension-reach-zantufa.toml"
}
EXPECTED_CHANGED = 20729
EXPECTED_MECHANICAL_INCIDENCES = {
    "single-unit-wrapper": 15699,
    "pure-adjacency": 4001,
    "pure-joik-jek": 6,
    "pure-plain-bo": 5,
}
EXPECTED_MANUAL = 3334


@dataclass(frozen=True)
class Form:
    name: str
    fields: tuple[tuple[str, Any], ...] | None = None
    args: tuple[Any, ...] | None = None


class DebugParseError(ValueError):
    pass


class DebugParser:
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
        while self.pos < len(self.text) and self.text[self.pos].isspace():
            self.pos += 1

    def _value(self) -> Any:
        self._space()
        if self.pos >= len(self.text):
            raise DebugParseError("unexpected end of input")
        char = self.text[self.pos]
        if char == '"':
            value, end = json.JSONDecoder().raw_decode(self.text[self.pos :])
            self.pos += end
            return value
        if char == "[":
            return self._sequence("[", "]", list)
        if char == "(":
            return tuple(self._sequence("(", ")", list))
        number = re.match(r"-?[0-9]+(?:\.[0-9]+)?", self.text[self.pos :])
        if number:
            token = number.group(0)
            self.pos += len(token)
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
        match = re.match(
            r"[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*",
            self.text[self.pos :],
        )
        if not match:
            raise DebugParseError(f"expected identifier at byte {self.pos}")
        value = match.group(0)
        self.pos += len(value)
        return value

    def _take(self, token: str) -> bool:
        if self.text.startswith(token, self.pos):
            self.pos += len(token)
            return True
        return False

    def _expect(self, token: str) -> None:
        if not self._take(token):
            raise DebugParseError(f"expected {token!r} at byte {self.pos}")


NONE = Form("None")


def field(form: Any, name: str) -> Any:
    if not isinstance(form, Form) or form.fields is None:
        raise KeyError(name)
    return dict(form.fields)[name]


def arg(form: Any, name: str | None = None) -> Any:
    if not isinstance(form, Form) or form.args is None or len(form.args) != 1:
        raise ValueError("not a unary form")
    if name is not None and form.name != name:
        raise ValueError(f"expected {name}, got {form.name}")
    return form.args[0]


def is_none(value: Any) -> bool:
    return value == NONE


def some(value: Any) -> Any | None:
    if isinstance(value, Form) and value.name == "Some" and value.args is not None:
        return arg(value)
    return None


def named(value: Any, name: str) -> bool:
    return isinstance(value, Form) and value.name == name


def unwrap_variant(value: Any, names: tuple[str, ...]) -> Any:
    while isinstance(value, Form) and value.name in names and value.args is not None:
        value = arg(value)
    return value


BANNED_SIMPLE_NAMES = (
    "Cei",
    "Forethought",
    "GroupedTanruUnit",
    "KeTanruUnit",
    "Nahe",
    "ScalarNegated",
    "Tagged",
    "Zantufa",
    "Experimental",
    "Relative",
)


def contains_banned_simple_shape(value: Any) -> bool:
    if isinstance(value, Form):
        if any(part in value.name for part in BANNED_SIMPLE_NAMES):
            return True
        if value.fields is not None:
            for key, child in value.fields:
                if key == "linkargs" and not is_none(child):
                    return True
                if "relative" in key.lower() and not is_none(child):
                    return True
                if contains_banned_simple_shape(child):
                    return True
        if value.args is not None:
            return any(contains_banned_simple_shape(child) for child in value.args)
    if isinstance(value, (list, tuple)):
        return any(contains_banned_simple_shape(child) for child in value)
    return False


def old_linked_payload(value: Any) -> Any | None:
    value = unwrap_variant(value, ("LinkedTanruUnit",))
    if not named(value, "LinkedTanruUnitSyntax"):
        return None
    if not is_none(field(value, "linkargs")) or contains_banned_simple_shape(value):
        return None
    return value


def old_simple_unit(value: Any) -> Any | None:
    try:
        chain = arg(value, "TanruUnitSyntax")
        if not named(chain, "Chain") or field(chain, "links") != []:
            return None
        return old_linked_payload(field(chain, "first"))
    except (KeyError, ValueError):
        return None


def connector_kind(value: Any) -> str | None:
    names: set[str] = set()

    def visit(node: Any) -> None:
        if isinstance(node, Form):
            names.add(node.name)
            if node.fields is not None:
                for _, child in node.fields:
                    visit(child)
            if node.args is not None:
                for child in node.args:
                    visit(child)
        elif isinstance(node, (list, tuple)):
            for child in node:
                visit(child)

    visit(value)
    if any("JekConnective" in name for name in names):
        return "jek"
    if any("JoikConnective" in name or "JoiConnective" in name for name in names):
        return "joik"
    return None


@dataclass(frozen=True)
class CanonicalSelbri:
    classification: str
    operands: tuple[Any, ...]
    connectives: tuple[Any, ...] = ()
    bo_words: tuple[Any, ...] = ()
    elided_terminators: tuple[tuple[str, int], ...] = ()


def old_plain_bo(value: Any) -> tuple[tuple[Any, ...], tuple[Any, ...]] | None:
    """Extract the old connectiveless, stagless BoundTanruUnit chain."""
    try:
        chain = arg(value, "TanruUnitSyntax")
        if not named(chain, "Chain") or field(chain, "links") != []:
            return None

        def extract(item: Any) -> tuple[list[Any], list[Any]] | None:
            linked = old_linked_payload(item)
            if linked is not None:
                return [linked], []
            bound = unwrap_variant(item, ("BoundTanruUnit",))
            if not named(bound, "BoundTanruUnitSyntax"):
                return None
            if not is_none(field(bound, "bo_connective")):
                return None
            if not is_none(field(bound, "bo_tense_modal")):
                return None
            leading = old_linked_payload(field(bound, "leading_unit"))
            trailing = extract(field(bound, "trailing_unit"))
            if leading is None or trailing is None:
                return None
            operands, bo_words = trailing
            return [leading, *operands], [field(bound, "bo"), *bo_words]

        extracted = extract(field(chain, "first"))
        if extracted is None or len(extracted[0]) < 2:
            return None
        return tuple(extracted[0]), tuple(extracted[1])
    except (KeyError, ValueError):
        return None


def old_tanru_parts(value: Any) -> tuple[Any, list[Any]]:
    if not named(value, "TanruSelbriSyntax"):
        raise ValueError("not old TanruSelbriSyntax")
    return field(value, "first_unit"), field(value, "additional_units")


def classify_old_selbri(value: Any) -> CanonicalSelbri | None:
    try:
        if not named(value, "CoSelbriSyntax") or not is_none(field(value, "co_tail")):
            return None
        connected = field(value, "leading_selbri")
        if not named(connected, "ConnectedSelbriSyntax"):
            return None
        tanru = field(connected, "leading_selbri")
        first, additional = old_tanru_parts(tanru)
        outer_continuations = field(connected, "continuations")

        simple = old_simple_unit(first)
        if simple is not None and not additional and not outer_continuations:
            return CanonicalSelbri("single-unit-wrapper", (simple,))

        adjacency = tuple(old_simple_unit(unit) for unit in (first, *additional))
        if (
            len(adjacency) > 1
            and all(unit is not None for unit in adjacency)
            and not outer_continuations
        ):
            return CanonicalSelbri("pure-adjacency", adjacency)

        plain_bo = old_plain_bo(first)
        if plain_bo is not None and not additional and not outer_continuations:
            operands, bo_words = plain_bo
            return CanonicalSelbri("pure-plain-bo", operands, bo_words=bo_words)

        # Legacy level one: jek/joik continuations inside TanruUnitSyntax.
        chain = arg(first, "TanruUnitSyntax")
        chain_links = field(chain, "links")
        leading = old_linked_payload(field(chain, "first"))
        if leading is not None and chain_links and not additional and not outer_continuations:
            operands = [leading]
            connectives = []
            for continuation in chain_links:
                continuation = unwrap_variant(continuation, ("TanruUnitContinuation",))
                if not named(continuation, "TanruUnitContinuationSyntax"):
                    return None
                connective = field(continuation, "connective")
                if connector_kind(connective) is None:
                    return None
                trailing = old_linked_payload(field(continuation, "trailing_unit"))
                if trailing is None:
                    return None
                operands.append(trailing)
                connectives.append(connective)
            return CanonicalSelbri(
                "pure-joik-jek", tuple(operands), tuple(connectives)
            )

        # Legacy level two: a ConnectedSelbriSyntax continuation chain.
        if simple is not None and not additional and outer_continuations:
            operands = [simple]
            connectives = []
            for continuation in outer_continuations:
                if not isinstance(continuation, Form):
                    return None
                connective = field(continuation, "connective")
                if connector_kind(connective) is None:
                    return None
                trailing = field(continuation, "trailing_selbri")
                trailing_first, trailing_additional = old_tanru_parts(trailing)
                trailing_simple = old_simple_unit(trailing_first)
                if trailing_simple is None or trailing_additional:
                    return None
                operands.append(trailing_simple)
                connectives.append(connective)
            return CanonicalSelbri(
                "pure-joik-jek", tuple(operands), tuple(connectives)
            )
    except (KeyError, ValueError, TypeError):
        return None
    return None


def new_tanru_unit_payload(value: Any) -> Any | None:
    if not named(value, "TanruUnitSyntax"):
        return None
    if field(value, "assignments") != []:
        return None
    payload = field(value, "base")
    if not named(payload, "LinkedTanruUnitSyntax"):
        return None
    if not is_none(field(payload, "linkargs")) or contains_banned_simple_shape(payload):
        return None
    return payload


def new_plain_bo(value: Any) -> tuple[tuple[Any, ...], tuple[Any, ...]] | None:
    operands: list[Any] = []
    bo_words: list[Any] = []
    while True:
        value = unwrap_variant(value, ("PlainBoTanruUnit",))
        if not named(value, "PlainBoTanruUnitSyntax"):
            return None
        payload = new_tanru_unit_payload(field(value, "leading_unit"))
        if payload is None:
            return None
        operands.append(payload)
        tail = some(field(value, "bo_tail"))
        if tail is None:
            return (tuple(operands), tuple(bo_words)) if len(operands) > 1 else None
        if not named(tail, "PlainBoSelbriTailSyntax"):
            return None
        bo_words.append(field(tail, "bo"))
        value = field(tail, "trailing_selbri")


def new_simple_bound(value: Any) -> Any | None:
    if not named(value, "BoundSelbriSyntax") or not is_none(field(value, "bo_tail")):
        return None
    plain = unwrap_variant(field(value, "leading_selbri"), ("PlainBoTanruUnit",))
    if not named(plain, "PlainBoTanruUnitSyntax") or not is_none(field(plain, "bo_tail")):
        return None
    return new_tanru_unit_payload(field(plain, "leading_unit"))


def classify_new_selbri(value: Any) -> CanonicalSelbri | None:
    try:
        if not named(value, "CoSelbriSyntax") or not is_none(field(value, "co_tail")):
            return None
        tanru = field(value, "leading_selbri")
        if not named(tanru, "TanruSelbriSyntax"):
            return None
        connected_values = (field(tanru, "first_selbri"), *field(tanru, "additional_selbri"))

        def simple_connected(connected: Any) -> Any | None:
            if not named(connected, "ConnectedSelbriSyntax"):
                return None
            if field(connected, "continuations") != []:
                return None
            return new_simple_bound(field(connected, "leading_selbri"))

        simple_values = tuple(simple_connected(value) for value in connected_values)
        if len(simple_values) == 1 and simple_values[0] is not None:
            return CanonicalSelbri("single-unit-wrapper", simple_values)
        if len(simple_values) > 1 and all(value is not None for value in simple_values):
            return CanonicalSelbri("pure-adjacency", simple_values)

        if len(connected_values) == 1:
            connected = connected_values[0]
            leading = new_simple_bound(field(connected, "leading_selbri"))
            continuations = field(connected, "continuations")
            if leading is not None and continuations:
                operands = [leading]
                connectives = []
                for continuation in continuations:
                    continuation = unwrap_variant(
                        continuation, ("SimpleConnectedSelbriContinuation",)
                    )
                    connective = field(continuation, "connective")
                    if connector_kind(connective) is None:
                        return None
                    trailing = new_simple_bound(field(continuation, "trailing_selbri"))
                    if trailing is None:
                        return None
                    operands.append(trailing)
                    connectives.append(connective)
                return CanonicalSelbri(
                    "pure-joik-jek", tuple(operands), tuple(connectives)
                )

            if named(connected, "ConnectedSelbriSyntax") and field(connected, "continuations") == []:
                bound = field(connected, "leading_selbri")
                if named(bound, "BoundSelbriSyntax") and is_none(field(bound, "bo_tail")):
                    bo = new_plain_bo(field(bound, "leading_selbri"))
                    if bo is not None:
                        operands, bo_words = bo
                        return CanonicalSelbri(
                            "pure-plain-bo", operands, bo_words=bo_words
                        )
    except (KeyError, ValueError, TypeError):
        return None
    return None


def source_spans(value: Any) -> tuple[tuple[Any, ...], ...]:
    spans: list[tuple[Any, ...]] = []

    def visit(node: Any) -> None:
        if named(node, "SourceSpan"):
            spans.append(tuple(child for _, child in node.fields or ()))
        if isinstance(node, Form):
            if node.fields is not None:
                for _, child in node.fields:
                    visit(child)
            if node.args is not None:
                for child in node.args:
                    visit(child)
        elif isinstance(node, (list, tuple)):
            for child in node:
                visit(child)

    visit(value)
    return tuple(spans)


def elided_terminator_positions(value: Any) -> tuple[tuple[str, int], ...]:
    """Model absent KEhE/FEhU/KEI tokens as zero-width spans at owner end."""
    result: list[tuple[str, int]] = []

    def visit(node: Any) -> None:
        if isinstance(node, Form) and node.fields is not None:
            spans = source_spans(node)
            end = max((span[2] for span in spans if len(span) > 2), default=None)
            for key, child in node.fields:
                if key in {"kehe", "fehu", "kei"} and is_none(child) and end is not None:
                    result.append((key, int(end)))
                visit(child)
        elif isinstance(node, Form) and node.args is not None:
            for child in node.args:
                visit(child)
        elif isinstance(node, (list, tuple)):
            for child in node:
                visit(child)

    visit(value)
    return tuple(result)


def equivalent_forms(old: Any, new: Any, classes: set[str]) -> bool:
    if old == new:
        return True
    if named(old, "CoSelbriSyntax") and named(new, "CoSelbriSyntax"):
        old_class = classify_old_selbri(old)
        new_class = classify_new_selbri(new)
        if old_class is None or new_class is None:
            return False
        if old_class.classification != new_class.classification:
            return False
        if len(old_class.operands) != len(new_class.operands):
            return False
        if len(old_class.connectives) != len(new_class.connectives):
            return False
        if len(old_class.bo_words) != len(new_class.bo_words):
            return False
        pairs = (
            zip(old_class.operands, new_class.operands),
            zip(old_class.connectives, new_class.connectives),
            zip(old_class.bo_words, new_class.bo_words),
        )
        if not all(equivalent_forms(left, right, classes) for group in pairs for left, right in group):
            return False
        # This is both the global span-exact check and the explicit
        # zero-width validation for elided KEhE/FEhU/KEI owners.
        if source_spans(old) != source_spans(new):
            return False
        if elided_terminator_positions(old) != elided_terminator_positions(new):
            return False
        classes.add(old_class.classification)
        return True
    if type(old) is not type(new):
        return False
    if isinstance(old, Form):
        if old.name != new.name:
            return False
        if (old.fields is None) != (new.fields is None):
            return False
        if old.fields is not None:
            if tuple(key for key, _ in old.fields) != tuple(key for key, _ in new.fields or ()):
                return False
            return all(
                equivalent_forms(left, right, classes)
                for (_, left), (_, right) in zip(old.fields, new.fields or ())
            )
        if (old.args is None) != (new.args is None):
            return False
        return all(
            equivalent_forms(left, right, classes)
            for left, right in zip(old.args or (), new.args or ())
        ) and len(old.args or ()) == len(new.args or ())
    if isinstance(old, (list, tuple)):
        return len(old) == len(new) and all(
            equivalent_forms(left, right, classes) for left, right in zip(old, new)
        )
    return False


def manual_shape_features(value: Any) -> tuple[str, ...]:
    """Name the excluded old-tree features without promoting them to classes."""
    features: set[str] = set()

    def visit(node: Any) -> None:
        if isinstance(node, Form):
            name = node.name
            if "Cei" in name:
                features.add("CEI")
            if "Forethought" in name:
                features.add("forethought")
            if "GroupedTanruUnit" in name or "KeTanruUnit" in name:
                features.add("KE")
            if "ScalarNegated" in name or "Nahe" in name:
                features.add("NAhE")
            if "Relative" in name:
                features.add("relative")
            if "Tagged" in name:
                features.add("tagged")
            if "Zantufa" in name or "Experimental" in name:
                features.add("warning-gated")
            if node.fields is not None:
                fields = dict(node.fields)
                if "co_tail" in fields and not is_none(fields["co_tail"]):
                    features.add("CO")
                if "linkargs" in fields and not is_none(fields["linkargs"]):
                    features.add("linkargs")
                for key, child in node.fields:
                    if "relative" in key.lower() and not is_none(child):
                        features.add("relative")
                    visit(child)
            if node.args is not None:
                for child in node.args:
                    visit(child)
        elif isinstance(node, (list, tuple)):
            for child in node:
                visit(child)

    visit(value)
    if not features:
        features.add("mixed-or-non-simple")
    return tuple(sorted(features))


def leaves(value: Any, path: tuple[str, ...] = ()) -> Iterator[tuple[tuple[str, ...], Any]]:
    if isinstance(value, dict):
        for key, child in value.items():
            yield from leaves(child, (*path, key))
    else:
        yield path, value


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
    for path, old_value in old_leaves.items():
        new_value = new_leaves[path]
        if old_value == new_value:
            continue
        if path[-2:] == ("syntax", "raw"):
            try:
                old_form = DebugParser(old_value).parse()
                new_form = DebugParser(new_value).parse()
            except (DebugParseError, TypeError) as error:
                residue.append(f"syntax.raw parser: {error}")
                continue
            if not equivalent_forms(old_form, new_form, classes):
                features = ",".join(manual_shape_features(old_form))
                residue.append(
                    "syntax.raw is outside the four mechanical classes "
                    f"({features})"
                )
            continue
        # Diagnostics (and every other non-tree leaf) are deliberately exact.
        # Gentufa's serialized/tree views are checked after syntax classification
        # by token/span equality and a narrow wrapper-name projection.
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
        residue.append(".".join(path))
    if not classes and old != new:
        residue.append("no mechanical selbri reconstruction found")
    return classes, residue


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


TREE_TOKEN = re.compile(
    r'^\s*(?:\w+:\s*)?\w+ @\[(?P<span>.*?)\) "(?P<text>.*)"[,]?$'
)


def tree_token_projection(value: str) -> tuple[tuple[str, str], ...]:
    result: list[tuple[str, str]] = []
    for line in value.splitlines():
        match = TREE_TOKEN.match(line)
        if match:
            result.append((match.group("span"), match.group("text")))
    return tuple(result)


BRACKET_TOKEN = re.compile(r"[A-Za-z][A-Za-z',\.]*")


def bracket_token_projection(value: str) -> tuple[str, ...]:
    return tuple(BRACKET_TOKEN.findall(value))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("candidate", type=Path)
    parser.add_argument("--baseline-root", type=Path, default=Path("tests/fixtures"))
    args = parser.parse_args()

    baseline_root = args.baseline_root
    candidate_root = args.candidate
    epoch_witnesses = set(
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
    mechanical: dict[str, list[str]] = {
        "single-unit-wrapper": [],
        "pure-adjacency": [],
        "pure-joik-jek": [],
        "pure-plain-bo": [],
    }
    manual: list[tuple[str, list[str]]] = []
    changed = 0
    for candidate_path in sorted(candidate_root.rglob("*.toml")):
        relative = candidate_path.relative_to(candidate_root)
        repository_path = (Path("tests/fixtures") / relative).as_posix()
        if repository_path in epoch_witnesses and repository_path not in C6_WITNESS_REFRESH:
            continue  # C1-C5 witnesses retain their commit-local exact pins.
        baseline_file = baseline_root / relative
        if not baseline_file.exists():
            continue
        old_text = baseline_file.read_text()
        new_text = candidate_path.read_text()
        if old_text == new_text:
            continue
        changed += 1
        classes, residue = compare_fixture(tomllib.loads(old_text), tomllib.loads(new_text))
        path_text = repository_path
        if residue:
            manual.append((path_text, residue))
        else:
            for classification in sorted(classes):
                mechanical[classification].append(path_text)

    print(f"changed: {changed}")
    for classification, paths in mechanical.items():
        print(f"mechanical {classification}: {len(paths)}")
        for path in paths:
            print(f"  {path}")
    print(f"manual: {len(manual)}")
    for path, reasons in manual:
        print(f"  {path}: {'; '.join(reasons)}")
    actual_incidences = {
        classification: len(paths) for classification, paths in mechanical.items()
    }
    if changed != EXPECTED_CHANGED:
        print(f"error: expected {EXPECTED_CHANGED} changed pre-epoch fixtures")
        return 1
    if actual_incidences != EXPECTED_MECHANICAL_INCIDENCES:
        print("error: mechanical class incidences differ from the reviewed C6 set")
        return 1
    if len(manual) != EXPECTED_MANUAL:
        print(f"error: expected {EXPECTED_MANUAL} manual fixtures")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
