"""Strongly typed immutable syntax-tree values."""

from __future__ import annotations

from collections.abc import Sequence
from typing import Callable, Generic, Self, TypeAlias, TypeVar, cast, final

from jbotci import _native as _rust
from jbotci import diagnostics, dialect, morphology, source
from jbotci._native import InvalidInputError
from jbotci._errors import _StructuredError

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

    __hash__ = None  # type: ignore[assignment]


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
        """Return the wrapped syntax value."""
        return self._value

    @property
    def free_modifiers(self) -> tuple[_F, ...]:
        """Return following free modifiers in source order."""
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
        """Return the first chain element."""
        return self._first

    @property
    def links(self) -> tuple[_L, ...]:
        """Return following chain links in source order."""
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
        """Return the valid recovered-field value."""
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
        """Return the recovery item replacing the absent value."""
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
        """Return recovery items preceding the retained value."""
        return self._errors

    @property
    def value(self) -> _T:
        """Return the valid value following the recovery prefix."""
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

SYNTAX_TRACE_FILTERS: tuple[str, ...] = _rust._syntax_parser_SYNTAX_TRACE_FILTERS
PARSER_ENUM_INVENTORY: tuple[str, ...] = _rust._syntax_parser_ENUM_INVENTORY

SyntaxTextUnitGranularity = _rust._syntax_parser_SyntaxTextUnitGranularity
SyntaxTextBoundaryKind = _rust._syntax_parser_SyntaxTextBoundaryKind
SyntaxErrorKind = _rust._syntax_parser_SyntaxErrorKind
SyntaxWordCategory = _rust._syntax_parser_SyntaxWordCategory
ExperimentalConstruct = _rust._syntax_parser_ExperimentalConstruct
ParseOptions = _rust._syntax_parser_ParseOptions
SyntaxTextUnit = _rust._syntax_parser_SyntaxTextUnit
SyntaxTextStructureEventBoundary = (
    _rust._syntax_parser_SyntaxTextStructureEventBoundary
)
SyntaxTextStructureEventContainerOpen = (
    _rust._syntax_parser_SyntaxTextStructureEventContainerOpen
)
SyntaxTextStructureEventContainerClose = (
    _rust._syntax_parser_SyntaxTextStructureEventContainerClose
)
SyntaxConstructContext = _rust._syntax_parser_SyntaxConstructContext
SyntaxExpectedTokenCmavo = _rust._syntax_parser_SyntaxExpectedTokenCmavo
SyntaxExpectedTokenSelmaho = _rust._syntax_parser_SyntaxExpectedTokenSelmaho
SyntaxExpectedTokenWordCategory = _rust._syntax_parser_SyntaxExpectedTokenWordCategory
SyntaxExpectedTokenEndOfInput = _rust._syntax_parser_SyntaxExpectedTokenEndOfInput
SyntaxExpectedTokenNamed = _rust._syntax_parser_SyntaxExpectedTokenNamed
SyntaxExpectationReasonContinueCurrent = (
    _rust._syntax_parser_SyntaxExpectationReasonContinueCurrent
)
SyntaxExpectationReasonStartNested = (
    _rust._syntax_parser_SyntaxExpectationReasonStartNested
)
SyntaxExpectationReasonEndThenStart = (
    _rust._syntax_parser_SyntaxExpectationReasonEndThenStart
)
SyntaxExpectation = _rust._syntax_parser_SyntaxExpectation
SyntaxErrorNotImplemented = _rust._syntax_parser_SyntaxErrorNotImplemented
SyntaxErrorParse = _rust._syntax_parser_SyntaxErrorParse
SyntaxWarning = _rust._syntax_parser_SyntaxWarning
SyntaxWarningDisplay = _rust._syntax_parser_SyntaxWarningDisplay
SyntaxParse = _rust._syntax_parser_SyntaxParse
SyntaxParseAttempt = _rust._syntax_parser_SyntaxParseAttempt
RecoveredSyntaxParse = _rust._syntax_parser_RecoveredSyntaxParse
RecoveredSyntaxParseAttempt = _rust._syntax_parser_RecoveredSyntaxParseAttempt
SyntaxRecoveryParseValid = _rust._syntax_parser_SyntaxRecoveryParseValid
SyntaxRecoveryParseRecovered = _rust._syntax_parser_SyntaxRecoveryParseRecovered
SyntaxRecoveryParseAttempt = _rust._syntax_parser_SyntaxRecoveryParseAttempt

SyntaxTextStructureEvent: TypeAlias = (
    SyntaxTextStructureEventBoundary
    | SyntaxTextStructureEventContainerOpen
    | SyntaxTextStructureEventContainerClose
)
SyntaxExpectedToken: TypeAlias = (
    SyntaxExpectedTokenCmavo
    | SyntaxExpectedTokenSelmaho
    | SyntaxExpectedTokenWordCategory
    | SyntaxExpectedTokenEndOfInput
    | SyntaxExpectedTokenNamed
)
SyntaxExpectationReason: TypeAlias = (
    SyntaxExpectationReasonContinueCurrent
    | SyntaxExpectationReasonStartNested
    | SyntaxExpectationReasonEndThenStart
)
SyntaxErrorValue: TypeAlias = SyntaxErrorNotImplemented | SyntaxErrorParse
SyntaxRecoveryParse: TypeAlias = (
    SyntaxRecoveryParseValid | SyntaxRecoveryParseRecovered
)


@final
class SyntaxError(_StructuredError[SyntaxErrorValue]):
    """Strict syntax failure retaining the typed Rust error and parse context."""

    __slots__ = (
        "_original_source",
        "_source_id",
        "_diagnostic",
        "_spans",
        "_trace",
    )

    _original_source: str | None
    _source_id: source.SourceId | None
    _diagnostic: diagnostics.Diagnostic | None
    _spans: tuple[source.SourceSpan, ...]
    _trace: diagnostics.TraceReport | None

    def __init__(
        self,
        value: SyntaxErrorValue,
        original_source: str | None = None,
        source_id: source.SourceId | None = None,
        trace: diagnostics.TraceReport | None = None,
    ) -> None:
        if not isinstance(value, (SyntaxErrorNotImplemented, SyntaxErrorParse)):
            raise TypeError("value must be a SyntaxErrorValue variant")
        if original_source is not None and not isinstance(original_source, str):
            raise TypeError("original_source must be a str or None")
        if source_id is not None and not isinstance(source_id, source.SourceId):
            raise TypeError("source_id must be a SourceId or None")
        if trace is not None and not isinstance(trace, diagnostics.TraceReport):
            raise TypeError("trace must be a TraceReport or None")
        diagnostic = (
            value.to_diagnostic(original_source, source_id)
            if original_source is not None
            else None
        )
        super().__init__(value)
        object.__setattr__(self, "_original_source", original_source)
        object.__setattr__(self, "_source_id", source_id)
        object.__setattr__(self, "_diagnostic", diagnostic)
        object.__setattr__(
            self,
            "_spans",
            ()
            if diagnostic is None
            else tuple(label.span for label in diagnostic.labels),
        )
        object.__setattr__(self, "_trace", trace)

    @property
    def original_source(self) -> str | None:
        """Return the source text supplied when constructing the exception."""
        return self._original_source

    @property
    def source_id(self) -> source.SourceId | None:
        """Return the optional source identifier."""
        return self._source_id

    @property
    def code(self) -> str:
        """Return the stable diagnostic code."""
        return self.value.code

    @property
    def diagnostic(self) -> diagnostics.Diagnostic | None:
        """Return the rendered diagnostic when source text was supplied."""
        return self._diagnostic

    @property
    def spans(self) -> tuple[source.SourceSpan, ...]:
        """Return every diagnostic source span."""
        return self._spans

    @property
    def trace(self) -> diagnostics.TraceReport | None:
        """Return the optional parser trace."""
        return self._trace

    def __init_subclass__(cls) -> None:
        raise TypeError("SyntaxError is final")


def syntax_tokens_with_options(
    words: Sequence[morphology.WordLike],
    *,
    options: ParseOptions | None = None,
) -> tuple[Token, ...]:
    """Normalize morphology values into the exact Rust syntax-token stream."""

    return _rust._syntax_parser_syntax_tokens_with_options(words, options=options)


normalize_syntax_tokens = syntax_tokens_with_options


def partition_syntax_text_units(
    tokens: Sequence[Token],
    granularity: SyntaxTextUnitGranularity,
) -> tuple[SyntaxTextUnit, ...]:
    """Partition normalized tokens at formal top-level text boundaries."""

    return _rust._syntax_parser_partition_syntax_text_units(tokens, granularity)


def syntax_text_structure(
    tokens: Sequence[Token],
) -> tuple[SyntaxTextStructureEvent, ...]:
    """Return formal boundary and text-container events."""

    return _rust._syntax_parser_syntax_text_structure(tokens)


def _raise_strict_attempt(
    attempt: SyntaxParseAttempt,
    source_text: str | None,
    source_id: source.SourceId | None,
) -> SyntaxParse:
    if attempt.error is not None:
        raise SyntaxError(attempt.error, source_text, source_id, attempt.trace)
    assert attempt.result is not None
    return attempt.result


def parse_text_attempt(
    words: Sequence[morphology.WordLike],
    *,
    options: ParseOptions | None = None,
) -> SyntaxParseAttempt:
    """Attempt direct strict text parsing without raising."""

    return _rust._syntax_parser_parse_text_attempt(words, options=options)


def parse_text(
    words: Sequence[morphology.WordLike],
    *,
    options: ParseOptions | None = None,
) -> strict.TextSyntax:
    """Run the direct strict Rust text parser and return its typed root."""

    return _raise_strict_attempt(parse_text_attempt(words, options=options), None, None).parse_tree


def parse_syntax_tree_attempt(
    words: Sequence[morphology.WordLike],
    *,
    source_text: str | None = None,
    options: ParseOptions | None = None,
) -> SyntaxParseAttempt:
    """Attempt strict parsing while retaining its structured error and trace."""

    return _rust._syntax_parser_parse_syntax_tree_attempt(
        words, source=source_text, options=options
    )


def parse_syntax_tree(
    words: Sequence[morphology.WordLike],
    *,
    source_text: str | None = None,
    options: ParseOptions | None = None,
    source_id: source.SourceId | None = None,
) -> SyntaxParse:
    """Parse presegmented morphology values strictly."""

    return _raise_strict_attempt(
        parse_syntax_tree_attempt(words, source_text=source_text, options=options),
        source_text,
        source_id,
    )


def parse_syntax_tree_recovered_attempt(
    words: Sequence[morphology.WordLike],
    *,
    source_text: str,
    options: ParseOptions | None = None,
) -> RecoveredSyntaxParseAttempt:
    """Attempt recovered parsing, retaining exact error-slot indices and trace."""

    return _rust._syntax_parser_parse_syntax_tree_recovered_attempt(
        words, source=source_text, options=options
    )


def parse_syntax_tree_recovered(
    words: Sequence[morphology.WordLike],
    *,
    source_text: str,
    options: ParseOptions | None = None,
) -> RecoveredSyntaxParse:
    """Parse presegmented values with syntax recovery."""

    return parse_syntax_tree_recovered_attempt(
        words, source_text=source_text, options=options
    ).result


def parse_syntax_tree_with_recovery_attempt(
    words: Sequence[morphology.WordLike],
    *,
    source_text: str,
    options: ParseOptions | None = None,
) -> SyntaxRecoveryParseAttempt:
    """Attempt strict-or-recovered parsing without converting valid strict trees."""

    return _rust._syntax_parser_parse_syntax_tree_with_recovery_attempt(
        words, source=source_text, options=options
    )


def parse_syntax_tree_with_recovery(
    words: Sequence[morphology.WordLike],
    *,
    source_text: str,
    options: ParseOptions | None = None,
) -> SyntaxRecoveryParse:
    """Return the exact strict-success or recovered-result variant."""

    return parse_syntax_tree_with_recovery_attempt(
        words, source_text=source_text, options=options
    ).result


def expected_continuations(
    words: Sequence[morphology.WordLike],
    *,
    options: ParseOptions | None = None,
) -> tuple[SyntaxExpectation, ...]:
    """Return typed grammar expectations after a morphology word prefix."""

    return _rust._syntax_parser_expected_continuations(words, options=options)


def expected_continuations_with_time_limit(
    words: Sequence[morphology.WordLike],
    time_limit: float,
    *,
    options: ParseOptions | None = None,
) -> tuple[SyntaxExpectation, ...]:
    """Return typed grammar expectations under a finite nonnegative time limit."""

    return _rust._syntax_parser_expected_continuations_with_time_limit(
        words, time_limit, options=options
    )


def expected_continuations_at_cursor(
    text: str,
    cursor: int,
    *,
    morphology_options: morphology.MorphologyOptions | None = None,
    parse_options: ParseOptions | None = None,
    source_id: source.SourceId | None = None,
    time_limit: float | None = None,
) -> tuple[SyntaxExpectation, ...]:
    """Segment exactly ``text[:cursor]`` and return its grammar expectations."""

    if cursor < 0 or cursor > len(text):
        raise InvalidInputError("cursor must satisfy 0 <= cursor <= len(text)")
    prefix = text[:cursor]
    words = morphology.segment(
        prefix, options=morphology_options, source_id=source_id
    )
    if time_limit is None:
        return expected_continuations(words, options=parse_options)
    return expected_continuations_with_time_limit(
        words, time_limit, options=parse_options
    )


def expected_continuations_for_text(
    text: str,
    *,
    morphology_options: morphology.MorphologyOptions | None = None,
    parse_options: ParseOptions | None = None,
    source_id: source.SourceId | None = None,
    time_limit: float | None = None,
) -> tuple[SyntaxExpectation, ...]:
    """Return grammar expectations at the Unicode-character end of ``text``."""

    return expected_continuations_at_cursor(
        text,
        len(text),
        morphology_options=morphology_options,
        parse_options=parse_options,
        source_id=source_id,
        time_limit=time_limit,
    )


def syntax_tree_eq_ignoring_spans(
    left: strict.TextSyntax,
    right: strict.TextSyntax,
) -> bool:
    """Compare strict generated trees while ignoring source-span fields."""

    return _rust._syntax_tree_eq_ignoring_spans(left, right)


def syntax_warning_display(
    source_label: str,
    source_text: str,
    tokens: Sequence[Token],
    warning: SyntaxWarning,
) -> SyntaxWarningDisplay:
    """Render one typed syntax warning for terminal-oriented display."""

    return _rust._syntax_parser_syntax_warning_display(
        source_label, source_text, tokens, warning
    )


def syntax_warning_displays(
    source_label: str,
    source_text: str,
    tokens: Sequence[Token],
    warnings: Sequence[SyntaxWarning],
) -> tuple[SyntaxWarningDisplay, ...]:
    """Render an immutable sequence of typed syntax warnings."""

    return _rust._syntax_parser_syntax_warning_displays(
        source_label, source_text, tokens, warnings
    )

__all__: tuple[str, ...] = (
    "SYNTAX_TRACE_FILTERS",
    "PARSER_ENUM_INVENTORY",
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
    "ExperimentalConstruct",
    "ParseOptions",
    "RecoveredSyntaxParse",
    "RecoveredSyntaxParseAttempt",
    "SyntaxConstructContext",
    "SyntaxError",
    "SyntaxErrorKind",
    "SyntaxErrorNotImplemented",
    "SyntaxErrorParse",
    "SyntaxErrorValue",
    "SyntaxExpectation",
    "SyntaxExpectationReason",
    "SyntaxExpectationReasonContinueCurrent",
    "SyntaxExpectationReasonEndThenStart",
    "SyntaxExpectationReasonStartNested",
    "SyntaxExpectedToken",
    "SyntaxExpectedTokenCmavo",
    "SyntaxExpectedTokenEndOfInput",
    "SyntaxExpectedTokenNamed",
    "SyntaxExpectedTokenSelmaho",
    "SyntaxExpectedTokenWordCategory",
    "SyntaxParse",
    "SyntaxParseAttempt",
    "SyntaxRecoveryParse",
    "SyntaxRecoveryParseAttempt",
    "SyntaxRecoveryParseRecovered",
    "SyntaxRecoveryParseValid",
    "SyntaxTextBoundaryKind",
    "SyntaxTextStructureEvent",
    "SyntaxTextStructureEventBoundary",
    "SyntaxTextStructureEventContainerClose",
    "SyntaxTextStructureEventContainerOpen",
    "SyntaxTextUnit",
    "SyntaxTextUnitGranularity",
    "SyntaxWarning",
    "SyntaxWarningDisplay",
    "SyntaxWordCategory",
    "expected_continuations",
    "expected_continuations_at_cursor",
    "expected_continuations_for_text",
    "expected_continuations_with_time_limit",
    "normalize_syntax_tokens",
    "parse_syntax_tree",
    "parse_syntax_tree_attempt",
    "parse_syntax_tree_recovered",
    "parse_syntax_tree_recovered_attempt",
    "parse_syntax_tree_with_recovery",
    "parse_syntax_tree_with_recovery_attempt",
    "parse_text",
    "parse_text_attempt",
    "partition_syntax_text_units",
    "recovered",
    "syntax_text_structure",
    "syntax_tokens_with_options",
    "syntax_tree_eq_ignoring_spans",
    "syntax_warning_display",
    "syntax_warning_displays",
    "strict",
)
