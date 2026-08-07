"""Immutable typed access to the embedded English Lensisku dictionary."""

from __future__ import annotations

from typing import Final, Sequence, TypeAlias, final

from ._native import (
    _dictionary_DefinitionId as DefinitionId,
    _dictionary_Dictionary as Dictionary,
    _dictionary_DictionaryEntries as DictionaryEntries,
    _dictionary_DictionaryEntry as DictionaryEntry,
    _dictionary_DictionaryLujvoEntry as DictionaryLujvoEntry,
    _dictionary_DictionaryLujvoSegment as DictionaryLujvoSegment,
    _dictionary_DictionaryLujvoSegmentKind as DictionaryLujvoSegmentKind,
    _dictionary_DictionaryPatternEntry as DictionaryPatternEntry,
    _dictionary_DictionarySnapshotMetadata as DictionarySnapshotMetadata,
    _dictionary_DictionarySoundEntry as DictionarySoundEntry,
    _dictionary_DictionaryUser as DictionaryUser,
    _dictionary_EntryIndex as EntryIndex,
    _dictionary_FreeRafsiAvailability as FreeRafsiAvailability,
    _dictionary_InvalidEntryValidationDetail as InvalidEntryValidationDetail,
    _dictionary_InvalidLujvoIndexEntryValidationDetail as InvalidLujvoIndexEntryValidationDetail,
    _dictionary_InvalidSoundIndexEntryValidationDetail as InvalidSoundIndexEntryValidationDetail,
    _dictionary_IpaSegmentId as IpaSegmentId,
    _dictionary_IpaTokenSequenceView as IpaTokenSequenceView,
    _dictionary_Keyword as Keyword,
    _dictionary_PatternIndexMismatchValidationDetail as PatternIndexMismatchValidationDetail,
    _dictionary_PronunciationTargetId as PronunciationTargetId,
    _dictionary_PronunciationTargetSequenceView as PronunciationTargetSequenceView,
    _dictionary_Rafsi as Rafsi,
    _dictionary_RafsiCandidate as RafsiCandidate,
    _dictionary_RafsiClaimKind as RafsiClaimKind,
    _dictionary_RafsiIndexMismatchValidationDetail as RafsiIndexMismatchValidationDetail,
    _dictionary_RafsiMatch as RafsiMatch,
    _dictionary_RafsiSource as RafsiSource,
    _dictionary_RawSelmaho as RawSelmaho,
    _dictionary_Score as Score,
    _dictionary_SelmahoIndexMismatchValidationDetail as SelmahoIndexMismatchValidationDetail,
    _dictionary_TakenRafsiAvailability as TakenRafsiAvailability,
    _dictionary_WordIndexMismatchValidationDetail as WordIndexMismatchValidationDetail,
    _dictionary_WordType as WordType,
    _dictionary_english,
    _dictionary_english_metadata,
    _dictionary_normalize_lookup_query,
    _dictionary_normalize_pattern_lookup_key,
    _dictionary_universal_gismu_rafsi_forms,
    _dictionary_word_type_is_gismu_like,
    _dictionary_word_type_is_lujvo_like,
    _dictionary_word_type_rafsi_claim_kind,
)
from ._native import JbotciError

RafsiAvailability: TypeAlias = FreeRafsiAvailability | TakenRafsiAvailability

DictionaryValidationDetail: TypeAlias = (
    InvalidEntryValidationDetail
    | WordIndexMismatchValidationDetail
    | RafsiIndexMismatchValidationDetail
    | SelmahoIndexMismatchValidationDetail
    | PatternIndexMismatchValidationDetail
    | InvalidSoundIndexEntryValidationDetail
    | InvalidLujvoIndexEntryValidationDetail
)

english: Final[Dictionary] = _dictionary_english
english_metadata: Final[DictionarySnapshotMetadata] = _dictionary_english_metadata


def _word_type_is_gismu_like(self: WordType) -> bool:
    """Return whether this Rust word type is gismu-like."""
    return _dictionary_word_type_is_gismu_like(self)


def _word_type_is_lujvo_like(self: WordType) -> bool:
    """Return whether this Rust word type is lujvo-like."""
    return _dictionary_word_type_is_lujvo_like(self)


def _word_type_rafsi_claim_kind(self: WordType) -> RafsiClaimKind:
    """Return the standing of a rafsi claim made by this Rust word type."""
    return _dictionary_word_type_rafsi_claim_kind(self)


# Functional `StrEnum` construction is what lets Rust register the exact class
# per interpreter. Attach the Rust word-type methods after construction; their
# implementations delegate through exact native enum extraction to Rust.
setattr(WordType, "is_gismu_like", _word_type_is_gismu_like)
setattr(WordType, "is_lujvo_like", _word_type_is_lujvo_like)
setattr(WordType, "rafsi_claim_kind", _word_type_rafsi_claim_kind)


@final
class DictionaryValidationError(JbotciError):
    """Dictionary validation failed with a concrete structural detail."""

    def __init__(self, detail: DictionaryValidationDetail) -> None:
        self.detail: Final[DictionaryValidationDetail] = detail
        super().__init__(str(detail))


def normalize_lookup_query(raw: str) -> str:
    """Return the normalized key used for exact and prefix lookup."""
    return _dictionary_normalize_lookup_query(raw)


def normalize_pattern_lookup_key(raw: str) -> str:
    """Return the normalized key used by dictionary pattern matching."""
    return _dictionary_normalize_pattern_lookup_key(raw)


def universal_gismu_rafsi_forms(
    word: str,
) -> tuple[tuple[Rafsi, RafsiSource], ...]:
    """Return universal short/long rafsi forms with exact provenance."""
    return _dictionary_universal_gismu_rafsi_forms(word)


def short_rafsi_candidates(gismu: str) -> tuple[RafsiCandidate, ...]:
    """Return every short rafsi a gismu could claim, with its availability."""
    return english.short_rafsi_candidates(gismu)


def rafsi_claimants(rafsi: str) -> tuple[tuple[str, WordType], ...]:
    """Return the word and type of every entry claiming a rafsi."""
    return english.rafsi_claimants(rafsi)


def first_gloss_keywords_for_words(
    words: Sequence[str],
) -> tuple[str | None, ...]:
    """Batch first-gloss lookup against the embedded English dictionary."""
    return english.first_gloss_keywords_for_words(words)


__all__: tuple[str, ...] = (
    "DefinitionId",
    "Dictionary",
    "DictionaryEntries",
    "DictionaryEntry",
    "DictionaryLujvoEntry",
    "DictionaryLujvoSegment",
    "DictionaryLujvoSegmentKind",
    "DictionaryPatternEntry",
    "DictionarySnapshotMetadata",
    "DictionarySoundEntry",
    "DictionaryUser",
    "DictionaryValidationDetail",
    "DictionaryValidationError",
    "EntryIndex",
    "FreeRafsiAvailability",
    "InvalidEntryValidationDetail",
    "InvalidLujvoIndexEntryValidationDetail",
    "InvalidSoundIndexEntryValidationDetail",
    "IpaSegmentId",
    "IpaTokenSequenceView",
    "Keyword",
    "PatternIndexMismatchValidationDetail",
    "PronunciationTargetId",
    "PronunciationTargetSequenceView",
    "Rafsi",
    "RafsiAvailability",
    "RafsiCandidate",
    "RafsiClaimKind",
    "RafsiIndexMismatchValidationDetail",
    "RafsiMatch",
    "RafsiSource",
    "RawSelmaho",
    "Score",
    "SelmahoIndexMismatchValidationDetail",
    "TakenRafsiAvailability",
    "WordIndexMismatchValidationDetail",
    "WordType",
    "english",
    "english_metadata",
    "first_gloss_keywords_for_words",
    "normalize_lookup_query",
    "normalize_pattern_lookup_key",
    "rafsi_claimants",
    "short_rafsi_candidates",
    "universal_gismu_rafsi_forms",
)
