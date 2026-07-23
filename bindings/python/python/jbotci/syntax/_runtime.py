"""Private shared runtime for generated strict and recovered syntax classes."""

from __future__ import annotations

from collections.abc import Sequence
from typing import ClassVar, Self

from jbotci import _native as _rust


class _SyntaxNode:
    """Immutable public facade over one native owner-and-path handle."""

    __slots__ = ("_native",)

    _schema_id: ClassVar[int]
    __match_args__: ClassVar[tuple[str, ...]]
    _native: _rust._syntax_Value

    def __setattr__(self, name: str, value: object) -> None:
        raise AttributeError(f"{type(self).__name__} is immutable")

    @classmethod
    def _from_native(cls, native: _rust._syntax_Value) -> Self:
        value = object.__new__(cls)
        object.__setattr__(value, "_native", native)
        return value

    @classmethod
    def _from_fields(cls, fields: Sequence[object]) -> Self:
        native = _rust._syntax_construct(cls.__module__, cls._schema_id, tuple(fields))
        return cls._from_native(native)

    def _field(self, index: int) -> object:
        return self._native._field(index)

    def same_identity(self, other: object, /) -> bool:
        """Return whether two wrappers locate the same node in the same Rust owner."""
        return isinstance(other, _SyntaxNode) and self._native._same_identity(other._native)

    def _debug_projection_count(self) -> int:
        """Return the deterministic owner projection count used by regression tests."""
        return self._native._projection_count()

    def __repr__(self) -> str:
        fields = ", ".join(
            f"{name}={getattr(self, name)!r}" for name in self.__match_args__
        )
        return f"{self.__module__}.{type(self).__name__}({fields})"

    def __eq__(self, other: object, /) -> bool:
        if type(self) is not type(other):
            return False
        assert isinstance(other, _SyntaxNode)
        return self._native._structural_eq(other._native)

    __hash__ = None
