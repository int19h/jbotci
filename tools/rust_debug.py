"""Structural reader shared by the parser-independent expectation comparers.

This recognizes Rust Debug syntax, not Lojban. Names, field order, strings and all
source-span fields are retained; no projection or grammar rewrite happens here.
Exact transcriptions opt into preserving numeric literals; the default retains
the older description comparer's numeric conversion behavior.
"""

from __future__ import annotations

from dataclasses import dataclass
import json
import re
from typing import Any


@dataclass(frozen=True)
class Form:
    name: str
    fields: tuple[tuple[str, Any], ...] | None = None
    args: tuple[Any, ...] | None = None


@dataclass(frozen=True)
class NumberLiteral:
    """An untouched Debug number, distinct from strings and Python numeric values."""

    text: str


def exact_equal(left: Any, right: Any) -> bool:
    """Compare Debug/fixture structures without coercing booleans or numbers.

    Python container/dataclass equality recursively equates True with 1 and 1
    with 1.0. Those are different pinned scalars. Debug fields and sequences
    also retain their order and shape; JSON/TOML objects have unordered string
    keys. Numeric Debug spelling is retained separately by NumberLiteral.
    """
    if type(left) is not type(right):
        return False
    if isinstance(left, Form):
        return (left.name == right.name and exact_equal(left.fields, right.fields)
                and exact_equal(left.args, right.args))
    if isinstance(left, (list, tuple)):
        return len(left) == len(right) and all(exact_equal(a, b) for a, b in zip(left, right))
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(
            isinstance(key, str) and exact_equal(value, right[key]) for key, value in left.items())
    if isinstance(left, float):
        # Signed zero is visible in pinned output; NaN must not match itself.
        return left == right and left.hex() == right.hex()
    return left == right


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

    def __init__(self, text: str, *, preserve_number_literals: bool = False) -> None:
        self.text = text
        self.pos = 0
        self.preserve_number_literals = preserve_number_literals

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
            if self.preserve_number_literals:
                return NumberLiteral(token)
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
