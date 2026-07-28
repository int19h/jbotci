from collections.abc import Sequence
from typing import ClassVar, Final, final

from .syntax.strict import TextSyntax as _StrictTextSyntax
from .syntax.recovered import TextSyntax as _RecoveredTextSyntax

_syntax_parser_SYNTAX_TRACE_FILTERS: Final[tuple[str, ...]]
_syntax_parser_ENUM_INVENTORY: Final[tuple[str, ...]]

@final
class _syntax_parser_SyntaxRecoveryErrorPolicy:
    DEFAULT_PER_STATEMENT: ClassVar[int]
    DEFAULT_GLOBAL_HARD_CAP: ClassVar[int]
    __match_args__: ClassVar[tuple[str, str]]
    def __new__(
        cls,
        *,
        per_statement: int = ...,
        global_hard_cap: int = ...,
    ) -> _syntax_parser_SyntaxRecoveryErrorPolicy: ...
    @property
    def per_statement(self) -> int: ...
    @property
    def global_hard_cap(self) -> int: ...
    def with_per_statement_limit(
        self, limit: int
    ) -> _syntax_parser_SyntaxRecoveryErrorPolicy: ...
    def with_global_hard_cap(
        self, limit: int
    ) -> _syntax_parser_SyntaxRecoveryErrorPolicy: ...
    def __repr__(self, /) -> str: ...

@final
class _syntax_parser_ParseOptions:
    def __new__(
        cls,
        *,
        dialect: _dialect_DialectDefinition | None = ...,
        trace: _diagnostics_TraceOptions | None = ...,
        error_context_depth: int | None = ...,
        recovery_error_policy: _syntax_parser_SyntaxRecoveryErrorPolicy | None = ...,
        max_recovery_errors: int | None = ...,
    ) -> _syntax_parser_ParseOptions: ...
    @staticmethod
    def default() -> _syntax_parser_ParseOptions: ...
    def with_dialect(
        self, dialect: _dialect_DialectDefinition
    ) -> _syntax_parser_ParseOptions: ...
    def with_trace(
        self, trace: _diagnostics_TraceOptions
    ) -> _syntax_parser_ParseOptions: ...
    def with_error_context_depth(self, depth: int) -> _syntax_parser_ParseOptions: ...
    def with_recovery_error_policy(
        self, policy: _syntax_parser_SyntaxRecoveryErrorPolicy
    ) -> _syntax_parser_ParseOptions: ...
    def with_max_recovery_errors(self, limit: int) -> _syntax_parser_ParseOptions: ...
    @property
    def dialect(self) -> _dialect_DialectDefinition: ...
    @property
    def trace(self) -> _diagnostics_TraceOptions: ...
    @property
    def error_context_depth(self) -> int: ...
    @property
    def recovery_error_policy(self) -> _syntax_parser_SyntaxRecoveryErrorPolicy: ...
    @property
    def max_recovery_errors(self) -> int: ...

@final
class _syntax_parser_SyntaxTextUnit:
    __match_args__: ClassVar[tuple[str, str]]
    def __new__(
        cls, token_start: int, token_end: int
    ) -> _syntax_parser_SyntaxTextUnit: ...
    @property
    def token_start(self) -> int: ...
    @property
    def token_end(self) -> int: ...

@final
class _syntax_parser_SyntaxTextStructureEventBoundary:
    __match_args__: ClassVar[tuple[str, str]]
    def __new__(
        cls, kind: _syntax_parser_SyntaxTextBoundaryKind, depth: int
    ) -> _syntax_parser_SyntaxTextStructureEventBoundary: ...
    @property
    def kind(self) -> _syntax_parser_SyntaxTextBoundaryKind: ...
    @property
    def depth(self) -> int: ...

@final
class _syntax_parser_SyntaxTextStructureEventContainerOpen:
    __match_args__: ClassVar[tuple[str, str]]
    def __new__(
        cls, opener: _morphology_Cmavo, depth: int
    ) -> _syntax_parser_SyntaxTextStructureEventContainerOpen: ...
    @property
    def opener(self) -> _morphology_Cmavo: ...
    @property
    def depth(self) -> int: ...

@final
class _syntax_parser_SyntaxTextStructureEventContainerClose:
    __match_args__: ClassVar[tuple[str, str, str]]
    def __new__(
        cls, closer: _morphology_Cmavo, depth: int, matched: bool
    ) -> _syntax_parser_SyntaxTextStructureEventContainerClose: ...
    @property
    def closer(self) -> _morphology_Cmavo: ...
    @property
    def depth(self) -> int: ...
    @property
    def matched(self) -> bool: ...

_SyntaxTextStructureEvent = (
    _syntax_parser_SyntaxTextStructureEventBoundary
    | _syntax_parser_SyntaxTextStructureEventContainerOpen
    | _syntax_parser_SyntaxTextStructureEventContainerClose
)

@final
class _syntax_parser_SyntaxConstructContext:
    __match_args__: ClassVar[tuple[str, str, str]]
    def __new__(
        cls, construct: str, byte_start: int, byte_end: int
    ) -> _syntax_parser_SyntaxConstructContext: ...
    @property
    def construct(self) -> str: ...
    @property
    def byte_start(self) -> int: ...
    @property
    def byte_end(self) -> int: ...

@final
class _syntax_parser_SyntaxExpectedTokenCmavo:
    __match_args__: ClassVar[tuple[str]]
    def __new__(
        cls, cmavo: _morphology_Cmavo
    ) -> _syntax_parser_SyntaxExpectedTokenCmavo: ...
    @property
    def cmavo(self) -> _morphology_Cmavo: ...
    def summary_text(self) -> str: ...

@final
class _syntax_parser_SyntaxExpectedTokenSelmaho:
    __match_args__: ClassVar[tuple[str]]
    def __new__(
        cls, selmaho: _morphology_Selmaho
    ) -> _syntax_parser_SyntaxExpectedTokenSelmaho: ...
    @property
    def selmaho(self) -> _morphology_Selmaho: ...
    def summary_text(self) -> str: ...

@final
class _syntax_parser_SyntaxExpectedTokenWordCategory:
    __match_args__: ClassVar[tuple[str]]
    def __new__(
        cls, category: _syntax_parser_SyntaxWordCategory
    ) -> _syntax_parser_SyntaxExpectedTokenWordCategory: ...
    @property
    def category(self) -> _syntax_parser_SyntaxWordCategory: ...
    def summary_text(self) -> str: ...

@final
class _syntax_parser_SyntaxExpectedTokenEndOfInput:
    __match_args__: ClassVar[tuple[()]]
    def __new__(cls) -> _syntax_parser_SyntaxExpectedTokenEndOfInput: ...
    def summary_text(self) -> str: ...

@final
class _syntax_parser_SyntaxExpectedTokenNamed:
    __match_args__: ClassVar[tuple[str]]
    def __new__(cls, name: str) -> _syntax_parser_SyntaxExpectedTokenNamed: ...
    @property
    def name(self) -> str: ...
    def summary_text(self) -> str: ...

_SyntaxExpectedToken = (
    _syntax_parser_SyntaxExpectedTokenCmavo
    | _syntax_parser_SyntaxExpectedTokenSelmaho
    | _syntax_parser_SyntaxExpectedTokenWordCategory
    | _syntax_parser_SyntaxExpectedTokenEndOfInput
    | _syntax_parser_SyntaxExpectedTokenNamed
)

@final
class _syntax_parser_SyntaxExpectationReasonContinueCurrent:
    __match_args__: ClassVar[tuple[str]]
    def __new__(
        cls, construct: str
    ) -> _syntax_parser_SyntaxExpectationReasonContinueCurrent: ...
    @property
    def construct(self) -> str: ...

@final
class _syntax_parser_SyntaxExpectationReasonStartNested:
    __match_args__: ClassVar[tuple[str]]
    def __new__(
        cls, construct: str
    ) -> _syntax_parser_SyntaxExpectationReasonStartNested: ...
    @property
    def construct(self) -> str: ...

@final
class _syntax_parser_SyntaxExpectationReasonEndThenStart:
    __match_args__: ClassVar[tuple[str, str]]
    def __new__(
        cls, starts: str, ends: Sequence[str]
    ) -> _syntax_parser_SyntaxExpectationReasonEndThenStart: ...
    @property
    def starts(self) -> str: ...
    @property
    def ends(self) -> tuple[str, ...]: ...

_SyntaxExpectationReason = (
    _syntax_parser_SyntaxExpectationReasonContinueCurrent
    | _syntax_parser_SyntaxExpectationReasonStartNested
    | _syntax_parser_SyntaxExpectationReasonEndThenStart
)

@final
class _syntax_parser_SyntaxExpectation:
    __match_args__: ClassVar[tuple[str, str]]
    def __new__(
        cls,
        tokens: Sequence[_SyntaxExpectedToken],
        reason: _SyntaxExpectationReason,
    ) -> _syntax_parser_SyntaxExpectation: ...
    @property
    def tokens(self) -> tuple[_SyntaxExpectedToken, ...]: ...
    @property
    def reason(self) -> _SyntaxExpectationReason: ...

@final
class _syntax_parser_SyntaxErrorNotImplemented:
    __match_args__: ClassVar[tuple[()]]
    def __new__(cls) -> _syntax_parser_SyntaxErrorNotImplemented: ...
    @property
    def code(self) -> str: ...
    def to_diagnostic(
        self, source: str, source_id: _source_SourceId | None = ...
    ) -> _diagnostics_Diagnostic: ...
    def __str__(self) -> str: ...

@final
class _syntax_parser_SyntaxErrorParse:
    __match_args__: ClassVar[tuple[str, str, str, str, str, str, str]]
    def __new__(
        cls,
        kind: _syntax_parser_SyntaxErrorKind,
        byte_start: int,
        byte_end: int,
        reason: str,
        expected: Sequence[str],
        expectations: Sequence[_syntax_parser_SyntaxExpectation],
        contexts: Sequence[_syntax_parser_SyntaxConstructContext],
    ) -> _syntax_parser_SyntaxErrorParse: ...
    @property
    def kind(self) -> _syntax_parser_SyntaxErrorKind: ...
    @property
    def code(self) -> str: ...
    @property
    def byte_start(self) -> int: ...
    @property
    def byte_end(self) -> int: ...
    @property
    def reason(self) -> str: ...
    @property
    def expected(self) -> tuple[str, ...]: ...
    @property
    def expectations(self) -> tuple[_syntax_parser_SyntaxExpectation, ...]: ...
    @property
    def contexts(self) -> tuple[_syntax_parser_SyntaxConstructContext, ...]: ...
    def to_diagnostic(
        self, source: str, source_id: _source_SourceId | None = ...
    ) -> _diagnostics_Diagnostic: ...
    def __str__(self) -> str: ...

_SyntaxErrorValue = (
    _syntax_parser_SyntaxErrorNotImplemented | _syntax_parser_SyntaxErrorParse
)

@final
class _syntax_parser_SyntaxWarning:
    __match_args__: ClassVar[tuple[str, str, str]]
    @property
    def kind(self) -> _syntax_parser_ExperimentalConstruct: ...
    @property
    def anchor_index(self) -> int: ...
    @property
    def anchor(self) -> _syntax_Token: ...
    @property
    def code(self) -> str: ...
    @property
    def message(self) -> str: ...
    def to_diagnostic(
        self, source: str, source_id: _source_SourceId | None = ...
    ) -> _diagnostics_Diagnostic: ...

@final
class _syntax_parser_SyntaxWarningDisplay:
    @property
    def source_label(self) -> str: ...
    @property
    def kind(self) -> _syntax_parser_ExperimentalConstruct: ...
    @property
    def message(self) -> str: ...
    @property
    def line(self) -> int: ...
    @property
    def column(self) -> int: ...
    @property
    def selection_start(self) -> int: ...
    @property
    def selection_length(self) -> int: ...
    @property
    def experimental_cmavo(self) -> str | None: ...
    @property
    def context(self) -> str: ...

@final
class _syntax_parser_SyntaxParse:
    """@rust-doc:SyntaxParse"""
    @property
    def parse_tree(self) -> _StrictTextSyntax: ...
    @property
    def warnings(self) -> tuple[_syntax_parser_SyntaxWarning, ...]: ...

@final
class _syntax_parser_SyntaxParseAttempt:
    """@rust-doc:SyntaxParseAttempt"""
    @property
    def succeeded(self) -> bool: ...
    @property
    def result(self) -> _syntax_parser_SyntaxParse | None: ...
    @property
    def error(self) -> _SyntaxErrorValue | None: ...
    @property
    def trace(self) -> _diagnostics_TraceReport | None: ...

@final
class _syntax_parser_RecoveredSyntaxParse:
    """@rust-doc:RecoveredSyntaxParse"""
    @property
    def parse_tree(self) -> _RecoveredTextSyntax: ...
    @property
    def errors(self) -> tuple[_SyntaxErrorValue, ...]: ...
    @property
    def warnings(self) -> tuple[_syntax_parser_SyntaxWarning, ...]: ...

@final
class _syntax_parser_RecoveredSyntaxParseAttempt:
    """@rust-doc:RecoveredSyntaxParseAttempt"""
    @property
    def result(self) -> _syntax_parser_RecoveredSyntaxParse: ...
    @property
    def trace(self) -> _diagnostics_TraceReport | None: ...

@final
class _syntax_parser_SyntaxRecoveryParseValid:
    """@rust-doc:SyntaxRecoveryParseValid"""
    __match_args__: ClassVar[tuple[str]]
    def __new__(
        cls, parse: _syntax_parser_SyntaxParse
    ) -> _syntax_parser_SyntaxRecoveryParseValid: ...
    @property
    def parse(self) -> _syntax_parser_SyntaxParse: ...

@final
class _syntax_parser_SyntaxRecoveryParseRecovered:
    """@rust-doc:SyntaxRecoveryParseRecovered"""
    __match_args__: ClassVar[tuple[str]]
    def __new__(
        cls, parse: _syntax_parser_RecoveredSyntaxParse
    ) -> _syntax_parser_SyntaxRecoveryParseRecovered: ...
    @property
    def parse(self) -> _syntax_parser_RecoveredSyntaxParse: ...

_SyntaxRecoveryParse = (
    _syntax_parser_SyntaxRecoveryParseValid
    | _syntax_parser_SyntaxRecoveryParseRecovered
)

@final
class _syntax_parser_SyntaxRecoveryParseAttempt:
    """@rust-doc:SyntaxRecoveryParseAttempt"""
    @property
    def result(self) -> _SyntaxRecoveryParse: ...
    @property
    def trace(self) -> _diagnostics_TraceReport | None: ...

def _syntax_parser_syntax_tokens_with_options(
    words: Sequence[_MorphologyWordLike],
    *,
    options: _syntax_parser_ParseOptions | None = ...,
) -> tuple[_syntax_Token, ...]: ...
def _syntax_parser_partition_syntax_text_units(
    tokens: Sequence[_syntax_Token],
    granularity: _syntax_parser_SyntaxTextUnitGranularity,
) -> tuple[_syntax_parser_SyntaxTextUnit, ...]: ...
def _syntax_parser_syntax_text_structure(
    tokens: Sequence[_syntax_Token],
) -> tuple[_SyntaxTextStructureEvent, ...]: ...
def _syntax_parser_parse_text_attempt(
    words: Sequence[_MorphologyWordLike],
    *,
    options: _syntax_parser_ParseOptions | None = ...,
) -> _syntax_parser_SyntaxParseAttempt: ...
def _syntax_parser_parse_syntax_tree_attempt(
    words: Sequence[_MorphologyWordLike],
    *,
    source: str | None = ...,
    options: _syntax_parser_ParseOptions | None = ...,
) -> _syntax_parser_SyntaxParseAttempt: ...
def _syntax_parser_parse_syntax_tree_recovered_attempt(
    words: Sequence[_MorphologyWordLike],
    *,
    source: str,
    options: _syntax_parser_ParseOptions | None = ...,
) -> _syntax_parser_RecoveredSyntaxParseAttempt: ...
def _syntax_parser_parse_syntax_tree_with_recovery_attempt(
    words: Sequence[_MorphologyWordLike],
    *,
    source: str,
    options: _syntax_parser_ParseOptions | None = ...,
) -> _syntax_parser_SyntaxRecoveryParseAttempt: ...
def _syntax_parser_expected_continuations(
    words: Sequence[_MorphologyWordLike],
    *,
    options: _syntax_parser_ParseOptions | None = ...,
) -> tuple[_syntax_parser_SyntaxExpectation, ...]: ...
def _syntax_parser_expected_continuations_with_time_limit(
    words: Sequence[_MorphologyWordLike],
    time_limit: float,
    *,
    options: _syntax_parser_ParseOptions | None = ...,
) -> tuple[_syntax_parser_SyntaxExpectation, ...]: ...
def _syntax_parser_syntax_warning_display(
    source_label: str,
    source: str,
    tokens: Sequence[_syntax_Token],
    warning: _syntax_parser_SyntaxWarning,
) -> _syntax_parser_SyntaxWarningDisplay: ...
def _syntax_parser_syntax_warning_displays(
    source_label: str,
    source: str,
    tokens: Sequence[_syntax_Token],
    warnings: Sequence[_syntax_parser_SyntaxWarning],
) -> tuple[_syntax_parser_SyntaxWarningDisplay, ...]: ...
