"""Shared runtime validation for the public pure-Python wrappers."""

from __future__ import annotations

from collections.abc import Sequence
from typing import TypeVar, cast

_Element = TypeVar("_Element")


def immutable_typed_sequence(
    value: object,
    *,
    parameter: str,
    element_type: type[_Element],
) -> tuple[_Element, ...]:
    """Validate an ordered non-string sequence and return an immutable copy."""

    if isinstance(value, (str, bytes, bytearray)) or not isinstance(value, Sequence):
        raise TypeError(
            f"{parameter} must be an ordered Sequence of {element_type.__name__}"
        )

    sequence = cast(Sequence[object], value)
    result: list[_Element] = []
    for index, element in enumerate(sequence):
        if not isinstance(element, element_type):
            raise TypeError(
                f"{parameter}[{index}] must be a {element_type.__name__}"
            )
        result.append(element)
    return tuple(result)
