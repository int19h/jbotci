"""Morphology parsing, word models, classification, and domain helpers."""

from __future__ import annotations

from collections.abc import Callable
from functools import wraps
from typing import ParamSpec, TypeAlias, TypeVar

from . import _native as _rust
from .diagnostics import Diagnostic, TraceReport
from .source import SourceId, SourceSpan

WordKind = _rust._morphology_WordKind
ValsiAnalysisStatus = _rust._morphology_ValsiAnalysisStatus
ValsiClassificationKind = _rust._morphology_ValsiClassificationKind
ValsiLujvoPartKind = _rust._morphology_ValsiLujvoPartKind
ValsiLujvoRafsiKind = _rust._morphology_ValsiLujvoRafsiKind
ValsiFuhivlaStage = _rust._morphology_ValsiFuhivlaStage
StressMark = _rust._morphology_StressMark
GlideMark = _rust._morphology_GlideMark
MorphologyErrorKind = _rust._morphology_MorphologyErrorKind
MorphologyWarningKind = _rust._morphology_MorphologyWarningKind
MorphologyContextKind = _rust._morphology_MorphologyContextKind
LujvoParseExpectation = _rust._morphology_LujvoParseExpectation
ExpectedWordDetailKind = _rust._morphology_ExpectedWordDetailKind
ZoiDelimiterDetailKind = _rust._morphology_ZoiDelimiterDetailKind
PhonotacticDetailKind = _rust._morphology_PhonotacticDetailKind
LujvoBuildMode = _rust._morphology_LujvoBuildMode
RafsiShape = _rust._morphology_RafsiShape
ConsonantPairClass = _rust._morphology_ConsonantPairClass
LeadingPauseVowelMode = _rust._morphology_LeadingPauseVowelMode
LeadingPauseContext = _rust._morphology_LeadingPauseContext
Cmavo = _rust._morphology_Cmavo
Selmaho = _rust._morphology_Selmaho

PhonemeRenderOptions = _rust._morphology_PhonemeRenderOptions
Phonemes = _rust._morphology_Phonemes
WordKey = _rust._morphology_WordKey
MorphologyOptions = _rust._morphology_MorphologyOptions
CompiledDialectDefinition = _rust._morphology_CompiledDialectDefinition
CompiledDialectSwap = _rust._morphology_CompiledDialectSwap
CompiledDialectExpansion = _rust._morphology_CompiledDialectExpansion
CompiledDialectWord = _rust._morphology_CompiledDialectWord
LujvoRafsi = _rust._morphology_LujvoRafsi
LujvoHyphen = _rust._morphology_LujvoHyphen
Verbatim = _rust._morphology_Verbatim
CmavoWord = _rust._morphology_CmavoWord
GismuWord = _rust._morphology_GismuWord
LujvoWord = _rust._morphology_LujvoWord
FuhivlaWord = _rust._morphology_FuhivlaWord
CmevlaWord = _rust._morphology_CmevlaWord
PlainWord = _rust._morphology_PlainWord
QuotedWord = _rust._morphology_QuotedWord
SelmahoQuotedWord = _rust._morphology_SelmahoQuotedWord
DelimitedNonLojbanQuote = _rust._morphology_DelimitedNonLojbanQuote
QuotedWords = _rust._morphology_QuotedWords
DelimitedWordQuote = _rust._morphology_DelimitedWordQuote
LerfuWord = _rust._morphology_LerfuWord
ZeiCompound = _rust._morphology_ZeiCompound
MorphologyContext = _rust._morphology_MorphologyContext
MorphologyWarning = _rust._morphology_MorphologyWarning
InvalidLujvoDetail = _rust._morphology_InvalidLujvoDetail
FuhivlaContainsYDetail = _rust._morphology_FuhivlaContainsYDetail
SlinkuhiDetail = _rust._morphology_SlinkuhiDetail
ExpectedWordDetail = _rust._morphology_ExpectedWordDetail
InvalidZoiDelimiterDetail = _rust._morphology_InvalidZoiDelimiterDetail
PhonotacticDetail = _rust._morphology_PhonotacticDetail
InvalidMorphology = _rust._morphology_InvalidMorphology
UnterminatedZoiQuote = _rust._morphology_UnterminatedZoiQuote
SourceSpanMorphologyError = _rust._morphology_SourceSpanMorphologyError
MorphologySegmentAttempt = _rust._morphology_MorphologySegmentAttempt
RecoveredMorphologySegmentation = _rust._morphology_RecoveredMorphologySegmentation
RecoveredMorphologySegmentAttempt = _rust._morphology_RecoveredMorphologySegmentAttempt
ValsiLujvoPart = _rust._morphology_ValsiLujvoPart
PlainWordClassification = _rust._morphology_PlainWordClassification
PlainWordValsiClassification = _rust._morphology_PlainWordValsiClassification
QuotedWordValsiClassification = _rust._morphology_QuotedWordValsiClassification
DelimitedNonLojbanQuoteValsiClassification = (
    _rust._morphology_DelimitedNonLojbanQuoteValsiClassification
)
QuotedWordsValsiClassification = _rust._morphology_QuotedWordsValsiClassification
DelimitedWordQuoteValsiClassification = (
    _rust._morphology_DelimitedWordQuoteValsiClassification
)
LerfuWordValsiClassification = _rust._morphology_LerfuWordValsiClassification
ZeiCompoundValsiClassification = _rust._morphology_ZeiCompoundValsiClassification
ValsiAnalysisResult = _rust._morphology_ValsiAnalysisResult
ValsiAnalysis = _rust._morphology_ValsiAnalysis
LujvoRafsiBuildPart = _rust._morphology_LujvoRafsiBuildPart
LujvoBrivlaCoreBuildPart = _rust._morphology_LujvoBrivlaCoreBuildPart
LujvoCandidate = _rust._morphology_LujvoCandidate

Word: TypeAlias = CmavoWord | GismuWord | LujvoWord | FuhivlaWord | CmevlaWord
LujvoPart: TypeAlias = LujvoRafsi | LujvoHyphen
WordLike: TypeAlias = (
    PlainWord
    | QuotedWord
    | SelmahoQuotedWord
    | DelimitedNonLojbanQuote
    | QuotedWords
    | DelimitedWordQuote
    | LerfuWord
    | ZeiCompound
)
MorphologyErrorDetail: TypeAlias = (
    InvalidLujvoDetail
    | FuhivlaContainsYDetail
    | SlinkuhiDetail
    | ExpectedWordDetail
    | InvalidZoiDelimiterDetail
    | PhonotacticDetail
)
MorphologyErrorValue: TypeAlias = (
    InvalidMorphology | UnterminatedZoiQuote | SourceSpanMorphologyError
)
ValsiClassification: TypeAlias = (
    PlainWordValsiClassification
    | QuotedWordValsiClassification
    | DelimitedNonLojbanQuoteValsiClassification
    | QuotedWordsValsiClassification
    | DelimitedWordQuoteValsiClassification
    | LerfuWordValsiClassification
    | ZeiCompoundValsiClassification
)
CompiledDialectEntry: TypeAlias = CompiledDialectSwap | CompiledDialectExpansion
LujvoBuildPart: TypeAlias = LujvoRafsiBuildPart | LujvoBrivlaCoreBuildPart

_P = ParamSpec("_P")
_R = TypeVar("_R")


def _public_native(
    name: str, function: Callable[_P, _R]
) -> Callable[_P, _R]:
    """Give a private native callable stable public introspection metadata."""

    @wraps(function)
    def wrapper(*args: _P.args, **kwargs: _P.kwargs) -> _R:
        return function(*args, **kwargs)

    wrapper.__name__ = name
    wrapper.__qualname__ = name
    wrapper.__module__ = __name__
    return wrapper


class MorphologyError(_rust.JbotciError):
    """Strict morphology failure with its complete typed Rust projection."""

    value: MorphologyErrorValue
    original_source: str
    source_id: SourceId | None
    code: str
    diagnostic: Diagnostic
    spans: tuple[SourceSpan, ...]
    context: MorphologyContext | None
    detail: MorphologyErrorDetail | None
    warnings: tuple[MorphologyWarning, ...]
    trace: TraceReport | None

    def __init__(
        self,
        value: MorphologyErrorValue,
        original_source: str,
        source_id: SourceId | None,
        warnings: tuple[MorphologyWarning, ...] = (),
        trace: TraceReport | None = None,
    ) -> None:
        super().__init__(str(value))
        self.value = value
        self.original_source = original_source
        self.source_id = source_id
        self.code = value.code
        self.diagnostic = value.to_diagnostic(original_source, source_id)
        self.spans = tuple(label.span for label in self.diagnostic.labels)
        self.context = getattr(value, "context", None)
        self.detail = getattr(value, "detail", None)
        self.warnings = warnings
        self.trace = trace


def segment_attempt(
    text: str,
    *,
    options: MorphologyOptions | None = None,
    source_id: SourceId | None = None,
) -> MorphologySegmentAttempt:
    """Run strict segmentation without raising on a morphology failure."""

    return _rust._morphology_segment_attempt(text, options=options, source_id=source_id)


def segment(
    text: str,
    *,
    options: MorphologyOptions | None = None,
    source_id: SourceId | None = None,
) -> tuple[WordLike, ...]:
    """Segment text strictly, raising :class:`MorphologyError` on failure."""

    attempt = segment_attempt(text, options=options, source_id=source_id)
    if attempt.error is not None:
        raise MorphologyError(
            attempt.error,
            attempt.source,
            attempt.source_id,
            attempt.warnings,
            attempt.trace,
        )
    assert attempt.words is not None
    return attempt.words


def segment_recovered_attempt(
    text: str,
    *,
    options: MorphologyOptions | None = None,
    source_id: SourceId | None = None,
) -> RecoveredMorphologySegmentAttempt:
    """Run recovery segmentation and retain the optional parser trace."""

    return _rust._morphology_segment_recovered_attempt(
        text, options=options, source_id=source_id
    )


def segment_recovered(
    text: str,
    *,
    options: MorphologyOptions | None = None,
    source_id: SourceId | None = None,
) -> RecoveredMorphologySegmentation:
    """Segment text with recovery, preserving typed errors and skipped regions."""

    return segment_recovered_attempt(
        text, options=options, source_id=source_id
    ).result


def segment_for_display(
    text: str,
    *,
    options: MorphologyOptions | None = None,
    source_id: SourceId | None = None,
) -> tuple[WordLike, ...]:
    """Segment through the Rust display-oriented morphology entry point."""

    attempt = _rust._morphology_segment_for_display_attempt(
        text, options=options, source_id=source_id
    )
    if attempt.error is not None:
        raise MorphologyError(
            attempt.error,
            attempt.source,
            attempt.source_id,
            attempt.warnings,
            attempt.trace,
        )
    assert attempt.words is not None
    return attempt.words


def analyze_valsi(
    text: str,
    *,
    options: MorphologyOptions | None = None,
    source_id: SourceId | None = None,
) -> ValsiAnalysis:
    """Analyze whether text is one valsi and classify its morphology."""

    return _rust._morphology_analyze_valsi(
        text, options=options, source_id=source_id
    )


normalize_input = _public_native("normalize_input", _rust._morphology_normalize_input)
canonicalize_text = _public_native(
    "canonicalize_text", _rust._morphology_canonicalize_text
)
canonical_text_eq = _public_native(
    "canonical_text_eq", _rust._morphology_canonical_text_eq
)
canonical_text_is_all = _public_native(
    "canonical_text_is_all", _rust._morphology_canonical_text_is_all
)
normalize_cmavo_form = _public_native(
    "normalize_cmavo_form", _rust._morphology_normalize_cmavo_form
)
cmavo_phonemes = _public_native("cmavo_phonemes", _rust._morphology_cmavo_phonemes)
pronunciation_syllables = _public_native(
    "pronunciation_syllables", _rust._morphology_pronunciation_syllables
)
strip_lojban_diacritic = _public_native(
    "strip_lojban_diacritic", _rust._morphology_strip_lojban_diacritic
)
fold_lojban_diacritic = _public_native(
    "fold_lojban_diacritic", _rust._morphology_fold_lojban_diacritic
)
strip_lojban_diacritics = _public_native(
    "strip_lojban_diacritics", _rust._morphology_strip_lojban_diacritics
)
fold_lojban_diacritics = _public_native(
    "fold_lojban_diacritics", _rust._morphology_fold_lojban_diacritics
)
stripped_lojban_diacritics_eq = _public_native(
    "stripped_lojban_diacritics_eq",
    _rust._morphology_stripped_lojban_diacritics_eq,
)
folded_lojban_diacritics_eq = _public_native(
    "folded_lojban_diacritics_eq", _rust._morphology_folded_lojban_diacritics_eq
)
strip_diacritics = _public_native("strip_diacritics", _rust._morphology_strip_diacritics)
strip_diacritics_eq = _public_native(
    "strip_diacritics_eq", _rust._morphology_strip_diacritics_eq
)
is_valid_phoneme = _public_native(
    "is_valid_phoneme", _rust._morphology_is_valid_phoneme
)
is_word_forming_character = _public_native(
    "is_word_forming_character", _rust._morphology_is_word_forming_character
)
is_period_character = _public_native(
    "is_period_character", _rust._morphology_is_period_character
)
is_permissive_ignorable_character = _public_native(
    "is_permissive_ignorable_character",
    _rust._morphology_is_permissive_ignorable_character,
)
parse_lujvo_parts = _public_native(
    "parse_lujvo_parts", _rust._morphology_parse_lujvo_parts
)
parse_cmevla_lujvo_parts = _public_native(
    "parse_cmevla_lujvo_parts", _rust._morphology_parse_cmevla_lujvo_parts
)
parse_cmevla_lujvo_part_candidates = _public_native(
    "parse_cmevla_lujvo_part_candidates",
    _rust._morphology_parse_cmevla_lujvo_part_candidates,
)
bond_rafsis = _public_native("bond_rafsis", _rust._morphology_bond_rafsis)
is_valid_lujvo_candidate_word = _public_native(
    "is_valid_lujvo_candidate_word", _rust._morphology_is_valid_lujvo_candidate_word
)
ensure_cmevla_word = _public_native(
    "ensure_cmevla_word", _rust._morphology_ensure_cmevla_word
)
ends_with_consonant = _public_native(
    "ends_with_consonant", _rust._morphology_ends_with_consonant
)
ends_with_vowel = _public_native("ends_with_vowel", _rust._morphology_ends_with_vowel)
is_bonding_hyphen = _public_native(
    "is_bonding_hyphen", _rust._morphology_is_bonding_hyphen
)
syllables_pattern = _public_native(
    "syllables_pattern", _rust._morphology_syllables_pattern
)
rafsi_shape = _public_native("rafsi_shape", _rust._morphology_rafsi_shape)
rafsi_shape_score = _public_native(
    "rafsi_shape_score", _rust._morphology_rafsi_shape_score
)
is_vowel = _public_native("is_vowel", _rust._morphology_is_vowel)
is_consonant = _public_native("is_consonant", _rust._morphology_is_consonant)
is_cmevla = _public_native("is_cmevla", _rust._morphology_is_cmevla)
consonant_pair_class = _public_native(
    "consonant_pair_class", _rust._morphology_consonant_pair_class
)
permissible_consonant_pair = _public_native(
    "permissible_consonant_pair", _rust._morphology_permissible_consonant_pair
)
consonant_pair_is_permissible = _public_native(
    "consonant_pair_is_permissible",
    _rust._morphology_consonant_pair_is_permissible,
)
consonant_pair_is_initial = _public_native(
    "consonant_pair_is_initial", _rust._morphology_consonant_pair_is_initial
)
word_needs_leading_pause = _public_native(
    "word_needs_leading_pause", _rust._morphology_word_needs_leading_pause
)
word_needs_leading_pause_in_context = _public_native(
    "word_needs_leading_pause_in_context",
    _rust._morphology_word_needs_leading_pause_in_context,
)
word_syntax_eq = _public_native("word_syntax_eq", _rust._morphology_word_syntax_eq)
word_like_syntax_eq = _public_native(
    "word_like_syntax_eq", _rust._morphology_word_like_syntax_eq
)
cmavo_from_text = _public_native("cmavo_from_text", _rust._morphology_cmavo_from_text)
cmavo_text = _public_native("cmavo_text", _rust._morphology_cmavo_text)
cmavo_is_selmaho = _public_native(
    "cmavo_is_selmaho", _rust._morphology_cmavo_is_selmaho
)
cmavo_primary_selmaho = _public_native(
    "cmavo_primary_selmaho", _rust._morphology_cmavo_primary_selmaho
)
cmavo_is_quote_opener = _public_native(
    "cmavo_is_quote_opener", _rust._morphology_cmavo_is_quote_opener
)
cmavo_is_single_word_quote_opener = _public_native(
    "cmavo_is_single_word_quote_opener",
    _rust._morphology_cmavo_is_single_word_quote_opener,
)
cmavo_is_delimited_non_lojban_quote_opener = _public_native(
    "cmavo_is_delimited_non_lojban_quote_opener",
    _rust._morphology_cmavo_is_delimited_non_lojban_quote_opener,
)
selmaho_from_name = _public_native(
    "selmaho_from_name", _rust._morphology_selmaho_from_name
)
selmaho_name = _public_native("selmaho_name", _rust._morphology_selmaho_name)
selmaho_contains = _public_native(
    "selmaho_contains", _rust._morphology_selmaho_contains
)
choose_best_lujvo_candidate = _public_native(
    "choose_best_lujvo_candidate", _rust._morphology_choose_best_lujvo_candidate
)
choose_best_lujvo_candidate_from_parts = _public_native(
    "choose_best_lujvo_candidate_from_parts",
    _rust._morphology_choose_best_lujvo_candidate_from_parts,
)

__all__: tuple[str, ...] = (
    "WordKind", "ValsiAnalysisStatus", "ValsiClassificationKind",
    "ValsiLujvoPartKind", "ValsiLujvoRafsiKind", "ValsiFuhivlaStage",
    "StressMark", "GlideMark", "MorphologyErrorKind", "MorphologyWarningKind",
    "MorphologyContextKind", "LujvoParseExpectation", "ExpectedWordDetailKind",
    "ZoiDelimiterDetailKind", "PhonotacticDetailKind", "LujvoBuildMode",
    "RafsiShape", "ConsonantPairClass", "LeadingPauseVowelMode",
    "LeadingPauseContext", "Cmavo", "Selmaho", "PhonemeRenderOptions",
    "Phonemes", "WordKey", "MorphologyOptions", "CompiledDialectDefinition",
    "CompiledDialectSwap", "CompiledDialectExpansion", "CompiledDialectWord",
    "CompiledDialectEntry", "LujvoRafsi", "LujvoHyphen", "LujvoPart",
    "Verbatim", "CmavoWord", "GismuWord", "LujvoWord", "FuhivlaWord",
    "CmevlaWord", "Word", "PlainWord", "QuotedWord", "SelmahoQuotedWord",
    "DelimitedNonLojbanQuote", "QuotedWords", "DelimitedWordQuote",
    "LerfuWord", "ZeiCompound", "WordLike", "MorphologyContext",
    "MorphologyWarning", "InvalidLujvoDetail", "FuhivlaContainsYDetail",
    "SlinkuhiDetail", "ExpectedWordDetail", "InvalidZoiDelimiterDetail",
    "PhonotacticDetail", "MorphologyErrorDetail", "InvalidMorphology",
    "UnterminatedZoiQuote", "SourceSpanMorphologyError", "MorphologyErrorValue",
    "MorphologyError", "MorphologySegmentAttempt", "RecoveredMorphologySegmentation",
    "RecoveredMorphologySegmentAttempt", "ValsiLujvoPart", "PlainWordClassification",
    "PlainWordValsiClassification", "QuotedWordValsiClassification",
    "DelimitedNonLojbanQuoteValsiClassification", "QuotedWordsValsiClassification",
    "DelimitedWordQuoteValsiClassification", "LerfuWordValsiClassification",
    "ZeiCompoundValsiClassification", "ValsiClassification", "ValsiAnalysisResult",
    "ValsiAnalysis", "LujvoRafsiBuildPart", "LujvoBrivlaCoreBuildPart",
    "LujvoBuildPart", "LujvoCandidate", "segment", "segment_attempt",
    "segment_recovered", "segment_recovered_attempt", "segment_for_display",
    "analyze_valsi", "normalize_input", "canonicalize_text", "canonical_text_eq",
    "canonical_text_is_all", "normalize_cmavo_form", "cmavo_phonemes",
    "pronunciation_syllables", "strip_lojban_diacritic", "fold_lojban_diacritic",
    "strip_lojban_diacritics", "fold_lojban_diacritics",
    "stripped_lojban_diacritics_eq", "folded_lojban_diacritics_eq",
    "strip_diacritics", "strip_diacritics_eq", "is_valid_phoneme",
    "is_word_forming_character", "is_period_character",
    "is_permissive_ignorable_character", "parse_lujvo_parts",
    "parse_cmevla_lujvo_parts", "parse_cmevla_lujvo_part_candidates",
    "bond_rafsis", "is_valid_lujvo_candidate_word", "ensure_cmevla_word",
    "ends_with_consonant", "ends_with_vowel", "is_bonding_hyphen",
    "syllables_pattern", "rafsi_shape", "rafsi_shape_score", "is_vowel",
    "is_consonant", "is_cmevla", "consonant_pair_class",
    "permissible_consonant_pair",
    "consonant_pair_is_permissible", "consonant_pair_is_initial",
    "word_needs_leading_pause", "word_needs_leading_pause_in_context",
    "word_syntax_eq", "word_like_syntax_eq", "cmavo_from_text", "cmavo_text",
    "cmavo_is_selmaho", "cmavo_primary_selmaho", "cmavo_is_quote_opener",
    "cmavo_is_single_word_quote_opener", "cmavo_is_delimited_non_lojban_quote_opener",
    "selmaho_from_name", "selmaho_name", "selmaho_contains",
    "choose_best_lujvo_candidate", "choose_best_lujvo_candidate_from_parts",
)
