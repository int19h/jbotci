"""Shared implementation for immutable structured jbotci exceptions."""

from __future__ import annotations

from typing import Generic, Never, TypeVar

from ._native import JbotciError

_Value = TypeVar("_Value")


class _StructuredError(JbotciError, Generic[_Value]):
    """Retain one immutable typed value while keeping standard error args."""

    __slots__ = ("_value",)
    __match_args__ = ("value",)
    _value: _Value

    def __init__(self, value: _Value) -> None:
        super().__init__(str(value))
        object.__setattr__(self, "_value", value)

    @property
    def value(self) -> _Value:
        """Return the complete structured error value."""

        return self._value

    def __setattr__(self, name: str, value: object) -> Never:
        raise AttributeError(f"{type(self).__name__} is immutable")
