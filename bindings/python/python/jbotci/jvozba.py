"""Typed jvozba composition and morphology-backed lujvo decomposition."""

from __future__ import annotations

from collections.abc import Sequence
from typing import Final, TypeAlias, cast, final

from . import _native as _rust
from ._errors import _StructuredError
from .dictionary import Dictionary, english
from .morphology import LujvoPart

JvozbaMode = _rust._jvozba_JvozbaMode
Word = _rust._jvozba_Word
FixedRafsi = _rust._jvozba_FixedRafsi
JvozbaSegmentKind = _rust._jvozba_JvozbaSegmentKind
JvozbaSegment = _rust._jvozba_JvozbaSegment
JvozbaBuildResult = _rust._jvozba_JvozbaBuildResult
LujvoSegmentInfo = _rust._jvozba_LujvoSegmentInfo
LujvoDecomposition = _rust._jvozba_LujvoDecomposition

RequiresAtLeastTwoInputs = _rust._jvozba_RequiresAtLeastTwoInputs
FixedRafsiEmpty = _rust._jvozba_FixedRafsiEmpty
NonFinalUniversalLongRafsi = _rust._jvozba_NonFinalUniversalLongRafsi
FinalConsonant = _rust._jvozba_FinalConsonant
NoRafsiAvailable = _rust._jvozba_NoRafsiAvailable
NoDictionaryEntry = _rust._jvozba_NoDictionaryEntry
CouldNotBuildLujvo = _rust._jvozba_CouldNotBuildLujvo
CouldNotBuildCompound = _rust._jvozba_CouldNotBuildCompound

JvozbaInput: TypeAlias = Word | FixedRafsi
JvozbaErrorValue: TypeAlias = (
    RequiresAtLeastTwoInputs
    | FixedRafsiEmpty
    | NonFinalUniversalLongRafsi
    | FinalConsonant
    | NoRafsiAvailable
    | NoDictionaryEntry
    | CouldNotBuildLujvo
    | CouldNotBuildCompound
)

_EXCEPTION_SUBCLASS_TOKEN: Final[object] = object()


class JvozbaError(_StructuredError[JvozbaErrorValue]):
    """Base class for the closed variant-specific jvozba exception hierarchy."""

    __slots__ = ()

    def __init__(self, value: JvozbaErrorValue) -> None:
        if type(self) is JvozbaError:
            raise TypeError("JvozbaError is an abstract variant base")
        super().__init__(value)

    def __init_subclass__(
        cls, *, _token: object | None = None
    ) -> None:
        if _token is not _EXCEPTION_SUBCLASS_TOKEN:
            raise TypeError("JvozbaError has a closed exception hierarchy")
        super().__init_subclass__()


@final
class RequiresAtLeastTwoInputsError(
    JvozbaError, _token=_EXCEPTION_SUBCLASS_TOKEN
):
    """Jvozba input expansion produced fewer than two inputs."""

    __slots__ = ()

    def __init__(self, value: RequiresAtLeastTwoInputs) -> None:
        if not isinstance(value, RequiresAtLeastTwoInputs):
            raise TypeError("value must be RequiresAtLeastTwoInputs")
        super().__init__(value)

    @property
    def value(self) -> RequiresAtLeastTwoInputs:
        """Return the exact structured Rust error value."""

        return cast(RequiresAtLeastTwoInputs, super().value)

    def __init_subclass__(cls) -> None:
        raise TypeError("RequiresAtLeastTwoInputsError is final")


@final
class FixedRafsiEmptyError(JvozbaError, _token=_EXCEPTION_SUBCLASS_TOKEN):
    """A fixed rafsi was explicitly empty."""

    __slots__ = ()

    def __init__(self, value: FixedRafsiEmpty) -> None:
        if not isinstance(value, FixedRafsiEmpty):
            raise TypeError("value must be FixedRafsiEmpty")
        super().__init__(value)

    @property
    def value(self) -> FixedRafsiEmpty:
        """Return the exact structured Rust error value."""

        return cast(FixedRafsiEmpty, super().value)

    def __init_subclass__(cls) -> None:
        raise TypeError("FixedRafsiEmptyError is final")


@final
class NonFinalUniversalLongRafsiError(
    JvozbaError, _token=_EXCEPTION_SUBCLASS_TOKEN
):
    """A universal long gismu rafsi appeared before the final position."""

    __slots__ = ()

    def __init__(self, value: NonFinalUniversalLongRafsi) -> None:
        if not isinstance(value, NonFinalUniversalLongRafsi):
            raise TypeError("value must be NonFinalUniversalLongRafsi")
        super().__init__(value)

    @property
    def value(self) -> NonFinalUniversalLongRafsi:
        """Return the exact structured Rust error value."""

        return cast(NonFinalUniversalLongRafsi, super().value)

    @property
    def offending(self) -> str:
        """Return the non-final fixed rafsi."""

        return self.value.offending

    def __init_subclass__(cls) -> None:
        raise TypeError("NonFinalUniversalLongRafsiError is final")


@final
class FinalConsonantError(JvozbaError, _token=_EXCEPTION_SUBCLASS_TOKEN):
    """No supplied final form can end the selected lujvo mode."""

    __slots__ = ()

    def __init__(self, value: FinalConsonant) -> None:
        if not isinstance(value, FinalConsonant):
            raise TypeError("value must be FinalConsonant")
        super().__init__(value)

    @property
    def value(self) -> FinalConsonant:
        """Return the exact structured Rust error value."""

        return cast(FinalConsonant, super().value)

    @property
    def offending(self) -> str:
        """Return the offending source value."""

        return self.value.offending

    @property
    def is_fixed_rafsi(self) -> bool:
        """Return whether the offending value was a fixed rafsi."""

        return self.value.is_fixed_rafsi

    def __init_subclass__(cls) -> None:
        raise TypeError("FinalConsonantError is final")


@final
class NoRafsiAvailableError(JvozbaError, _token=_EXCEPTION_SUBCLASS_TOKEN):
    """A dictionary word has no rafsi usable in its position."""

    __slots__ = ()

    def __init__(self, value: NoRafsiAvailable) -> None:
        if not isinstance(value, NoRafsiAvailable):
            raise TypeError("value must be NoRafsiAvailable")
        super().__init__(value)

    @property
    def value(self) -> NoRafsiAvailable:
        """Return the exact structured Rust error value."""

        return cast(NoRafsiAvailable, super().value)

    @property
    def offending(self) -> str:
        """Return the dictionary word with no usable rafsi."""

        return self.value.offending

    def __init_subclass__(cls) -> None:
        raise TypeError("NoRafsiAvailableError is final")


@final
class NoDictionaryEntryError(JvozbaError, _token=_EXCEPTION_SUBCLASS_TOKEN):
    """A word is absent from the selected dictionary."""

    __slots__ = ()

    def __init__(self, value: NoDictionaryEntry) -> None:
        if not isinstance(value, NoDictionaryEntry):
            raise TypeError("value must be NoDictionaryEntry")
        super().__init__(value)

    @property
    def value(self) -> NoDictionaryEntry:
        """Return the exact structured Rust error value."""

        return cast(NoDictionaryEntry, super().value)

    @property
    def offending(self) -> str:
        """Return the word absent from the dictionary."""

        return self.value.offending

    def __init_subclass__(cls) -> None:
        raise TypeError("NoDictionaryEntryError is final")


@final
class CouldNotBuildLujvoError(
    JvozbaError, _token=_EXCEPTION_SUBCLASS_TOKEN
):
    """Every candidate was rejected for lujvo output."""

    __slots__ = ()

    def __init__(self, value: CouldNotBuildLujvo) -> None:
        if not isinstance(value, CouldNotBuildLujvo):
            raise TypeError("value must be CouldNotBuildLujvo")
        super().__init__(value)

    @property
    def value(self) -> CouldNotBuildLujvo:
        """Return the exact structured Rust error value."""

        return cast(CouldNotBuildLujvo, super().value)

    def __init_subclass__(cls) -> None:
        raise TypeError("CouldNotBuildLujvoError is final")


@final
class CouldNotBuildCompoundError(
    JvozbaError, _token=_EXCEPTION_SUBCLASS_TOKEN
):
    """Every candidate was rejected for cmevla-like output."""

    __slots__ = ()

    def __init__(self, value: CouldNotBuildCompound) -> None:
        if not isinstance(value, CouldNotBuildCompound):
            raise TypeError("value must be CouldNotBuildCompound")
        super().__init__(value)

    @property
    def value(self) -> CouldNotBuildCompound:
        """Return the exact structured Rust error value."""

        return cast(CouldNotBuildCompound, super().value)

    def __init_subclass__(cls) -> None:
        raise TypeError("CouldNotBuildCompoundError is final")


def build_best_jvozba_detailed(
    mode: JvozbaMode,
    dictionary: Dictionary,
    raw_inputs: Sequence[JvozbaInput],
) -> JvozbaBuildResult:
    """Build through the exact low-level Rust signature and error hierarchy."""

    return _rust._jvozba_build_best_jvozba_detailed(
        mode, dictionary, raw_inputs
    )


def build(
    inputs: Sequence[JvozbaInput],
    *,
    mode: JvozbaMode = JvozbaMode.LUJVO,
    dictionary: Dictionary = english,
) -> JvozbaBuildResult:
    """Build the best jvozba with ergonomic Python argument order and defaults."""

    return build_best_jvozba_detailed(mode, dictionary, inputs)


def word_can_enter_jvozba_pane(
    dictionary: Dictionary, word_text: str
) -> bool:
    """Call the direct Rust predicate with an explicit dictionary."""

    return _rust._jvozba_word_can_enter_jvozba_pane(dictionary, word_text)


def can_use_word(
    word_text: str, *, dictionary: Dictionary = english
) -> bool:
    """Return whether a word can contribute to a lujvo build."""

    return word_can_enter_jvozba_pane(dictionary, word_text)


def decompose_lujvo_like(
    raw_word: str, *, dictionary: Dictionary = english
) -> LujvoDecomposition | None:
    """Decompose a lujvo or cmevla-like compound with owned source provenance."""

    return _rust._jvozba_decompose_lujvo_like(dictionary, raw_word)


__all__: tuple[str, ...] = (
    "JvozbaMode",
    "Word",
    "FixedRafsi",
    "JvozbaInput",
    "JvozbaSegmentKind",
    "JvozbaSegment",
    "JvozbaBuildResult",
    "LujvoSegmentInfo",
    "LujvoDecomposition",
    "RequiresAtLeastTwoInputs",
    "FixedRafsiEmpty",
    "NonFinalUniversalLongRafsi",
    "FinalConsonant",
    "NoRafsiAvailable",
    "NoDictionaryEntry",
    "CouldNotBuildLujvo",
    "CouldNotBuildCompound",
    "JvozbaErrorValue",
    "JvozbaError",
    "RequiresAtLeastTwoInputsError",
    "FixedRafsiEmptyError",
    "NonFinalUniversalLongRafsiError",
    "FinalConsonantError",
    "NoRafsiAvailableError",
    "NoDictionaryEntryError",
    "CouldNotBuildLujvoError",
    "CouldNotBuildCompoundError",
    "build_best_jvozba_detailed",
    "build",
    "word_can_enter_jvozba_pane",
    "can_use_word",
    "decompose_lujvo_like",
)
