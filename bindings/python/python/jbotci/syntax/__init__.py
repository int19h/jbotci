"""Strongly typed immutable syntax-tree values."""

from __future__ import annotations

from collections.abc import Sequence
from typing import Callable, Generic, Self, TypeAlias, TypeVar, cast, final

from jbotci import _native as _rust

_T = TypeVar("_T")
_F = TypeVar("_F")
_L = TypeVar("_L")


def _immutable_sequence(values: Sequence[_T], parameter: str) -> tuple[_T, ...]:
    """Copy a non-string ordered Python sequence into its immutable representation."""
    if isinstance(values, (str, bytes, bytearray)):
        raise TypeError(f"{parameter} must be a non-string sequence")
    return tuple(values)


class _ImmutableValue:
    """Shared immutable behavior for transparent syntax wrappers."""

    __slots__ = ("_identity",)

    _identity: _rust._syntax_Identity | None

    def _initialize_identity(self) -> None:
        object.__setattr__(self, "_identity", None)

    @classmethod
    def _from_projected(
        cls, arguments: tuple[object, ...], identity: _rust._syntax_Identity
    ) -> Self:
        constructor = cast(Callable[..., Self], cls)
        value = constructor(*arguments)
        object.__setattr__(value, "_identity", identity)
        return value

    def same_identity(self, other: object, /) -> bool:
        """Return whether two wrappers select the same generated owner field lens."""
        return (
            type(self) is type(other)
            and isinstance(other, _ImmutableValue)
            and self._identity is not None
            and other._identity is not None
            and self._identity._same_identity(other._identity)
        )

    def __setattr__(self, name: str, value: object) -> None:
        raise AttributeError(f"{type(self).__name__} is immutable")

    __hash__ = None


@final
class WithFreeModifiers(_ImmutableValue, Generic[_T, _F]):
    """A syntax value followed by its source-ordered free modifiers."""

    __slots__ = ("_value", "_free_modifiers")
    __match_args__ = ("value", "free_modifiers")
    _value: _T
    _free_modifiers: tuple[_F, ...]

    def __init__(self, value: _T, free_modifiers: Sequence[_F]) -> None:
        self._initialize_identity()
        object.__setattr__(self, "_value", value)
        object.__setattr__(
            self, "_free_modifiers", _immutable_sequence(free_modifiers, "free_modifiers")
        )

    @property
    def value(self) -> _T:
        return self._value

    @property
    def free_modifiers(self) -> tuple[_F, ...]:
        return self._free_modifiers

    def __repr__(self) -> str:
        return (
            "jbotci.syntax.WithFreeModifiers("
            f"value={self.value!r}, free_modifiers={self.free_modifiers!r})"
        )

    def __eq__(self, other: object, /) -> bool:
        return (
            isinstance(other, WithFreeModifiers)
            and self.value == other.value
            and self.free_modifiers == other.free_modifiers
        )

    def __init_subclass__(cls) -> None:
        raise TypeError("WithFreeModifiers is final")


@final
class Chain(_ImmutableValue, Generic[_T, _L]):
    """The first chain element and the immutable sequence of following links."""

    __slots__ = ("_first", "_links")
    __match_args__ = ("first", "links")
    _first: _T
    _links: tuple[_L, ...]

    def __init__(self, first: _T, links: Sequence[_L]) -> None:
        self._initialize_identity()
        object.__setattr__(self, "_first", first)
        object.__setattr__(self, "_links", _immutable_sequence(links, "links"))

    @property
    def first(self) -> _T:
        return self._first

    @property
    def links(self) -> tuple[_L, ...]:
        return self._links

    def __repr__(self) -> str:
        return f"jbotci.syntax.Chain(first={self.first!r}, links={self.links!r})"

    def __eq__(self, other: object, /) -> bool:
        return (
            isinstance(other, Chain)
            and self.first == other.first
            and self.links == other.links
        )

    def __init_subclass__(cls) -> None:
        raise TypeError("Chain is final")


@final
class RecoveredValid(_ImmutableValue, Generic[_T]):
    """A recovered field that contains its valid syntax value."""

    __slots__ = ("_value",)
    __match_args__ = ("value",)
    _value: _T

    def __init__(self, value: _T) -> None:
        self._initialize_identity()
        object.__setattr__(self, "_value", value)

    @property
    def value(self) -> _T:
        return self._value

    def __repr__(self) -> str:
        return f"jbotci.syntax.RecoveredValid(value={self.value!r})"

    def __eq__(self, other: object, /) -> bool:
        return isinstance(other, RecoveredValid) and self.value == other.value

    def __init_subclass__(cls) -> None:
        raise TypeError("RecoveredValid is final")


@final
class RecoveredError(_ImmutableValue):
    """A recovered field replaced entirely by one typed recovery item."""

    __slots__ = ("_error",)
    __match_args__ = ("error",)
    _error: SyntaxRecoveryItem

    def __init__(self, error: SyntaxRecoveryItem) -> None:
        self._initialize_identity()
        if not isinstance(error, (SkippedTokens, MissingRequiredField)):
            raise TypeError("error must be a SyntaxRecoveryItem value")
        object.__setattr__(self, "_error", error)

    @property
    def error(self) -> SyntaxRecoveryItem:
        return self._error

    def __repr__(self) -> str:
        return f"jbotci.syntax.RecoveredError(error={self.error!r})"

    def __eq__(self, other: object, /) -> bool:
        return isinstance(other, RecoveredError) and self.error == other.error

    def __init_subclass__(cls) -> None:
        raise TypeError("RecoveredError is final")


@final
class RecoveredPrefix(_ImmutableValue, Generic[_T]):
    """A recovered value preceded by one or more typed recovery items."""

    __slots__ = ("_errors", "_value")
    __match_args__ = ("errors", "value")
    _errors: tuple[SyntaxRecoveryItem, ...]
    _value: _T

    def __init__(self, errors: Sequence[SyntaxRecoveryItem], value: _T) -> None:
        self._initialize_identity()
        checked = _immutable_sequence(errors, "errors")
        if not checked:
            raise ValueError("errors must contain at least one recovery item")
        if not all(
            isinstance(error, (SkippedTokens, MissingRequiredField))
            for error in checked
        ):
            raise TypeError("errors must contain only SyntaxRecoveryItem values")
        object.__setattr__(self, "_errors", checked)
        object.__setattr__(self, "_value", value)

    @property
    def errors(self) -> tuple[SyntaxRecoveryItem, ...]:
        return self._errors

    @property
    def value(self) -> _T:
        return self._value

    def __repr__(self) -> str:
        return f"jbotci.syntax.RecoveredPrefix(errors={self.errors!r}, value={self.value!r})"

    def __eq__(self, other: object, /) -> bool:
        return (
            isinstance(other, RecoveredPrefix)
            and self.errors == other.errors
            and self.value == other.value
        )

    def __init_subclass__(cls) -> None:
        raise TypeError("RecoveredPrefix is final")


Token = _rust._syntax_Token
PlainWithIndicators = _rust._syntax_PlainWithIndicators
EmphasizedWithIndicators = _rust._syntax_EmphasizedWithIndicators
IndicatorWithIndicators = _rust._syntax_IndicatorWithIndicators
SkippedTokens = _rust._syntax_SkippedTokens
MissingRequiredField = _rust._syntax_MissingRequiredField

WithIndicators: TypeAlias = (
    PlainWithIndicators | EmphasizedWithIndicators | IndicatorWithIndicators
)
SyntaxRecoveryItem: TypeAlias = SkippedTokens | MissingRequiredField
RecoveredField: TypeAlias = (
    RecoveredValid[_T] | RecoveredError | RecoveredPrefix[_T]
)

from . import recovered, strict

__all__: tuple[str, ...] = (
    "Chain",
    "EmphasizedWithIndicators",
    "IndicatorWithIndicators",
    "MissingRequiredField",
    "PlainWithIndicators",
    "RecoveredError",
    "RecoveredField",
    "RecoveredPrefix",
    "RecoveredValid",
    "SkippedTokens",
    "SyntaxRecoveryItem",
    "Token",
    "WithFreeModifiers",
    "WithIndicators",
    "recovered",
    "strict",
)
