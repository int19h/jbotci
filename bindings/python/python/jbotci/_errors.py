"""Shared implementation for immutable structured jbotci exceptions."""

from __future__ import annotations

from typing import Generic, TypeVar

from ._native import JbotciError

_Value = TypeVar("_Value")

_BASE_EXCEPTION_METADATA: frozenset[str] = frozenset(
    {
        "__traceback__",
        "__cause__",
        "__context__",
        "__suppress_context__",
        "__notes__",
    }
)


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

    def __setattr__(self, name: str, value: object) -> None:
        if name in _BASE_EXCEPTION_METADATA:
            super().__setattr__(name, value)
            return
        raise AttributeError(f"{type(self).__name__} is immutable")

    def __delattr__(self, name: str) -> None:
        if name in _BASE_EXCEPTION_METADATA:
            super().__delattr__(name)
            return
        raise AttributeError(f"{type(self).__name__} is immutable")
