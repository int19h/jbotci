"""Pre-alpha Python bindings for jbotci's unstable Rust API."""

from __future__ import annotations

from typing import final

from . import diagnostics, dialect, dictionary, jvozba, morphology, semantics, source, syntax
from ._native import (
    InvalidInputError,
    JbotciError,
    _root_Sample as Sample,
    _root_SampleMode as SampleMode,
    __version__,
    raise_sample_error,
    sample_mode,
    smoke,
)
from .diagnostics import TraceReport
from .morphology import (
    MorphologyErrorValue,
    MorphologyWarning,
    RecoveredMorphologySegmentation,
    WordLike,
)
from .source import SourceId, SourceSpan
from .syntax import (
    RecoveredSyntaxParse,
    SyntaxErrorValue,
    SyntaxParse,
    SyntaxWarning,
)
from .syntax.recovered import TextSyntax as RecoveredTextSyntax
from .syntax.strict import TextSyntax as StrictTextSyntax


def _source_id(value: source.SourceId | str | None) -> source.SourceId | None:
    if value is None or isinstance(value, source.SourceId):
        return value
    if isinstance(value, str):
        return source.SourceId(value)
    raise TypeError("source_id must be a SourceId, str, or None")


@final
class ParsedText:
    """Successful high-level morphology and strict syntax parse."""

    __slots__ = (
        "_source",
        "_source_id",
        "_words",
        "_morphology_warnings",
        "_morphology_trace",
        "_syntax",
        "_syntax_trace",
    )
    _source: str
    _source_id: SourceId | None
    _words: tuple[WordLike, ...]
    _morphology_warnings: tuple[MorphologyWarning, ...]
    _morphology_trace: TraceReport | None
    _syntax: SyntaxParse
    _syntax_trace: TraceReport | None

    def __init__(
        self,
        source_text: str,
        source_id: source.SourceId | None,
        words: tuple[morphology.WordLike, ...],
        morphology_warnings: tuple[morphology.MorphologyWarning, ...],
        morphology_trace: diagnostics.TraceReport | None,
        syntax_parse: syntax.SyntaxParse,
        syntax_trace: diagnostics.TraceReport | None,
    ) -> None:
        object.__setattr__(self, "_source", source_text)
        object.__setattr__(self, "_source_id", source_id)
        object.__setattr__(self, "_words", words)
        object.__setattr__(self, "_morphology_warnings", morphology_warnings)
        object.__setattr__(self, "_morphology_trace", morphology_trace)
        object.__setattr__(self, "_syntax", syntax_parse)
        object.__setattr__(self, "_syntax_trace", syntax_trace)

    @property
    def source(self) -> str:
        """Return the original source text."""
        return self._source

    @property
    def source_id(self) -> SourceId | None:
        """Return the optional caller-supplied source identifier."""
        return self._source_id

    @property
    def words(self) -> tuple[WordLike, ...]:
        """Return the strict morphology words consumed by syntax parsing."""
        return self._words

    @property
    def morphology_warnings(self) -> tuple[MorphologyWarning, ...]:
        """Return morphology warnings in source order."""
        return self._morphology_warnings

    @property
    def morphology_trace(self) -> TraceReport | None:
        """Return the optional morphology trace report."""
        return self._morphology_trace

    @property
    def syntax(self) -> SyntaxParse:
        """Return the complete strict syntax parse result."""
        return self._syntax

    @property
    def syntax_trace(self) -> TraceReport | None:
        """Return the optional syntax trace report."""
        return self._syntax_trace

    @property
    def parse_tree(self) -> StrictTextSyntax:
        """Return the typed strict syntax root."""
        return self._syntax.parse_tree

    @property
    def warnings(self) -> tuple[SyntaxWarning, ...]:
        """Return syntax warnings in source order."""
        return self._syntax.warnings

    def __setattr__(self, name: str, value: object) -> None:
        raise AttributeError("ParsedText is immutable")

    def __init_subclass__(cls) -> None:
        raise TypeError("ParsedText is final")


@final
class AnalyzedText:
    """Successful high-level parse plus owning reference analysis."""

    __slots__ = ("_parsed", "_references")
    _parsed: ParsedText
    _references: semantics.references.ReferenceAnalysis

    def __init__(
        self,
        parsed: ParsedText,
        references: semantics.references.ReferenceAnalysis,
    ) -> None:
        object.__setattr__(self, "_parsed", parsed)
        object.__setattr__(self, "_references", references)

    @property
    def parsed(self) -> ParsedText:
        """Return the complete morphology and syntax parse result."""

        return self._parsed

    @property
    def source(self) -> str:
        """Return the original source text."""
        return self._parsed.source

    @property
    def source_id(self) -> SourceId | None:
        """Return the optional caller-supplied source identifier."""
        return self._parsed.source_id

    @property
    def words(self) -> tuple[WordLike, ...]:
        """Return the strict morphology words consumed by syntax parsing."""
        return self._parsed.words

    @property
    def morphology_warnings(self) -> tuple[MorphologyWarning, ...]:
        """Return morphology warnings in source order."""
        return self._parsed.morphology_warnings

    @property
    def morphology_trace(self) -> TraceReport | None:
        """Return the optional morphology trace report."""
        return self._parsed.morphology_trace

    @property
    def syntax(self) -> SyntaxParse:
        """Return the complete strict syntax parse result."""
        return self._parsed.syntax

    @property
    def syntax_trace(self) -> TraceReport | None:
        """Return the optional syntax trace report."""
        return self._parsed.syntax_trace

    @property
    def parse_tree(self) -> StrictTextSyntax:
        """Return the typed strict syntax root retained by the analysis."""
        return self._parsed.parse_tree

    @property
    def warnings(self) -> tuple[SyntaxWarning, ...]:
        """Return syntax warnings in source order."""
        return self._parsed.warnings

    @property
    def reference_analysis(self) -> semantics.references.ReferenceAnalysis:
        """Return place assignment and discourse-reference results."""

        return self._references

    def __setattr__(self, name: str, value: object) -> None:
        raise AttributeError("AnalyzedText is immutable")

    def __init_subclass__(cls) -> None:
        raise TypeError("AnalyzedText is final")


@final
class RecoveredParsedText:
    """High-level recovered morphology and syntax result."""

    __slots__ = (
        "_source",
        "_source_id",
        "_morphology",
        "_morphology_trace",
        "_syntax",
        "_syntax_trace",
    )
    _source: str
    _source_id: SourceId | None
    _morphology: RecoveredMorphologySegmentation
    _morphology_trace: TraceReport | None
    _syntax: RecoveredSyntaxParse
    _syntax_trace: TraceReport | None

    def __init__(
        self,
        source_text: str,
        source_id: source.SourceId | None,
        morphology_parse: morphology.RecoveredMorphologySegmentation,
        morphology_trace: diagnostics.TraceReport | None,
        syntax_parse: syntax.RecoveredSyntaxParse,
        syntax_trace: diagnostics.TraceReport | None,
    ) -> None:
        object.__setattr__(self, "_source", source_text)
        object.__setattr__(self, "_source_id", source_id)
        object.__setattr__(self, "_morphology", morphology_parse)
        object.__setattr__(self, "_morphology_trace", morphology_trace)
        object.__setattr__(self, "_syntax", syntax_parse)
        object.__setattr__(self, "_syntax_trace", syntax_trace)

    @property
    def source(self) -> str:
        """Return the original source text."""
        return self._source

    @property
    def source_id(self) -> SourceId | None:
        """Return the optional caller-supplied source identifier."""
        return self._source_id

    @property
    def morphology(self) -> RecoveredMorphologySegmentation:
        """Return the recovered morphology result."""
        return self._morphology

    @property
    def morphology_trace(self) -> TraceReport | None:
        """Return the optional morphology trace report."""
        return self._morphology_trace

    @property
    def words(self) -> tuple[WordLike, ...]:
        """Return recovered words accepted by syntax parsing."""
        return self._morphology.words

    @property
    def morphology_errors(self) -> tuple[MorphologyErrorValue, ...]:
        """Return typed morphology recovery errors."""
        return self._morphology.errors

    @property
    def syntax(self) -> RecoveredSyntaxParse:
        """Return the complete recovered syntax parse result."""
        return self._syntax

    @property
    def syntax_trace(self) -> TraceReport | None:
        """Return the optional syntax trace report."""
        return self._syntax_trace

    @property
    def parse_tree(self) -> RecoveredTextSyntax:
        """Return the typed recovered syntax root."""
        return self._syntax.parse_tree

    @property
    def syntax_errors(self) -> tuple[SyntaxErrorValue, ...]:
        """Return typed syntax recovery errors."""
        return self._syntax.errors

    @property
    def warnings(self) -> tuple[SyntaxWarning, ...]:
        """Return syntax warnings in source order."""
        return self._syntax.warnings

    def __setattr__(self, name: str, value: object) -> None:
        raise AttributeError("RecoveredParsedText is immutable")

    def __init_subclass__(cls) -> None:
        raise TypeError("RecoveredParsedText is final")


def parse(
    text: str,
    morphology_options: morphology.MorphologyOptions | None = None,
    parse_options: syntax.ParseOptions | None = None,
    source_id: source.SourceId | str | None = None,
) -> ParsedText:
    """Run strict morphology and syntax parsing without a serialization boundary."""

    checked_source_id = _source_id(source_id)
    morphology_attempt = morphology.segment_attempt(
        text, options=morphology_options, source_id=checked_source_id
    )
    if morphology_attempt.error is not None:
        raise morphology.MorphologyError(
            morphology_attempt.error,
            morphology_attempt.source,
            morphology_attempt.source_id,
            morphology_attempt.warnings,
            morphology_attempt.trace,
        )
    assert morphology_attempt.words is not None
    syntax_attempt = syntax.parse_syntax_tree_attempt(
        morphology_attempt.words,
        source_text=text,
        options=parse_options,
    )
    if syntax_attempt.error is not None:
        raise syntax.SyntaxError(
            syntax_attempt.error,
            text,
            checked_source_id,
            syntax_attempt.trace,
        )
    assert syntax_attempt.result is not None
    return ParsedText(
        text,
        checked_source_id,
        morphology_attempt.words,
        morphology_attempt.warnings,
        morphology_attempt.trace,
        syntax_attempt.result,
        syntax_attempt.trace,
    )


def parse_recovered(
    text: str,
    morphology_options: morphology.MorphologyOptions | None = None,
    parse_options: syntax.ParseOptions | None = None,
    source_id: source.SourceId | str | None = None,
) -> RecoveredParsedText:
    """Run recovered morphology and syntax parsing through their real Rust values."""

    checked_source_id = _source_id(source_id)
    morphology_attempt = morphology.segment_recovered_attempt(
        text, options=morphology_options, source_id=checked_source_id
    )
    syntax_attempt = syntax.parse_syntax_tree_recovered_attempt(
        morphology_attempt.result.words,
        source_text=text,
        options=parse_options,
    )
    return RecoveredParsedText(
        text,
        checked_source_id,
        morphology_attempt.result,
        morphology_attempt.trace,
        syntax_attempt.result,
        syntax_attempt.trace,
    )


def analyze(
    text: str,
    morphology_options: morphology.MorphologyOptions | None = None,
    parse_options: syntax.ParseOptions | None = None,
    source_id: source.SourceId | str | None = None,
) -> AnalyzedText:
    """Run the real strict parse pipeline and reference analysis."""

    parsed = parse(
        text,
        morphology_options=morphology_options,
        parse_options=parse_options,
        source_id=source_id,
    )
    references = semantics.references.analyze_references(parsed.syntax)
    return AnalyzedText(parsed, references)

__all__: tuple[str, ...] = (
    "__version__",
    "dictionary",
    "diagnostics",
    "dialect",
    "jvozba",
    "morphology",
    "semantics",
    "source",
    "syntax",
    "InvalidInputError",
    "JbotciError",
    "ParsedText",
    "AnalyzedText",
    "RecoveredParsedText",
    "Sample",
    "SampleMode",
    "raise_sample_error",
    "sample_mode",
    "smoke",
    "parse",
    "parse_recovered",
    "analyze",
)
