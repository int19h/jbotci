from __future__ import annotations

import enum
import gc
import subprocess
import sys
from collections.abc import Sequence
from pathlib import Path

import pytest

import jbotci
import jbotci._native as native
from jbotci import dictionary, jvozba, morphology


@pytest.mark.parametrize(
    ("mode", "inputs", "expected_word", "expected_segments"),
    (
        (
            jvozba.JvozbaMode.LUJVO,
            [jvozba.Word("lojbo"), jvozba.Word("bangu")],
            "jbobau",
            (
                (jvozba.JvozbaSegmentKind.RAFSI, "jbo"),
                (jvozba.JvozbaSegmentKind.RAFSI, "bau"),
            ),
        ),
        (
            jvozba.JvozbaMode.CMEVLA,
            (jvozba.Word("lojbo"), jvozba.FixedRafsi("bau")),
            "jbobaus",
            (
                (jvozba.JvozbaSegmentKind.RAFSI, "jbo"),
                (jvozba.JvozbaSegmentKind.RAFSI, "bau"),
                (jvozba.JvozbaSegmentKind.HYPHEN, "s"),
            ),
        ),
        (
            jvozba.JvozbaMode.LUJVO,
            (jvozba.FixedRafsi("jbon"), jvozba.Word("bangu")),
            "jbonybau",
            (
                (jvozba.JvozbaSegmentKind.RAFSI, "jbon"),
                (jvozba.JvozbaSegmentKind.HYPHEN, "y"),
                (jvozba.JvozbaSegmentKind.RAFSI, "bau"),
            ),
        ),
        (
            jvozba.JvozbaMode.LUJVO,
            (jvozba.Word("fulta"), jvozba.Word("ismu")),
            "fuly'ismu",
            (
                (jvozba.JvozbaSegmentKind.RAFSI, "ful"),
                (jvozba.JvozbaSegmentKind.HYPHEN, "y'"),
                (jvozba.JvozbaSegmentKind.RAFSI, "ismu"),
            ),
        ),
        (
            jvozba.JvozbaMode.LUJVO,
            (
                jvozba.FixedRafsi("bau"),
                jvozba.FixedRafsi("gri"),
                jvozba.Word("klama"),
            ),
            "baurgrikla",
            (
                (jvozba.JvozbaSegmentKind.RAFSI, "bau"),
                (jvozba.JvozbaSegmentKind.HYPHEN, "r"),
                (jvozba.JvozbaSegmentKind.RAFSI, "gri"),
                (jvozba.JvozbaSegmentKind.RAFSI, "kla"),
            ),
        ),
        (
            jvozba.JvozbaMode.CMEVLA,
            (jvozba.FixedRafsi("bau"), jvozba.FixedRafsi("rok")),
            "baunrok",
            (
                (jvozba.JvozbaSegmentKind.RAFSI, "bau"),
                (jvozba.JvozbaSegmentKind.HYPHEN, "n"),
                (jvozba.JvozbaSegmentKind.RAFSI, "rok"),
            ),
        ),
        (
            jvozba.JvozbaMode.LUJVO,
            (jvozba.FixedRafsi("akt"), jvozba.Word("iismu")),
            "aktyiismu",
            (
                (jvozba.JvozbaSegmentKind.RAFSI, "akt"),
                (jvozba.JvozbaSegmentKind.HYPHEN, "y"),
                (jvozba.JvozbaSegmentKind.RAFSI, "iismu"),
            ),
        ),
        (
            jvozba.JvozbaMode.LUJVO,
            (jvozba.Word("gismu"), jvozba.Word("iismu")),
            "gimyiismu",
            (
                (jvozba.JvozbaSegmentKind.RAFSI, "gim"),
                (jvozba.JvozbaSegmentKind.HYPHEN, "y"),
                (jvozba.JvozbaSegmentKind.RAFSI, "iismu"),
            ),
        ),
        (
            jvozba.JvozbaMode.LUJVO,
            (
                jvozba.Word("mutce"),
                jvozba.Word("nelci"),
                jvozba.Word("iismu"),
            ),
            "tcenelyiismu",
            (
                (jvozba.JvozbaSegmentKind.RAFSI, "tce"),
                (jvozba.JvozbaSegmentKind.RAFSI, "nel"),
                (jvozba.JvozbaSegmentKind.HYPHEN, "y"),
                (jvozba.JvozbaSegmentKind.RAFSI, "iismu"),
            ),
        ),
        (
            jvozba.JvozbaMode.LUJVO,
            (jvozba.Word("jenjigu'ydi'e"),),
            "jenjigu'ydi'e",
            (
                (jvozba.JvozbaSegmentKind.RAFSI, "jenjigu"),
                (jvozba.JvozbaSegmentKind.HYPHEN, "'y"),
                (jvozba.JvozbaSegmentKind.RAFSI, "di'e"),
            ),
        ),
    ),
)
def test_composition_matches_rust_behavioral_witnesses(
    mode: jvozba.JvozbaMode,
    inputs: Sequence[jvozba.JvozbaInput],
    expected_word: str,
    expected_segments: tuple[
        tuple[jvozba.JvozbaSegmentKind, str], ...
    ],
) -> None:
    result = jvozba.build(inputs, mode=mode, dictionary=dictionary.english)

    assert result.word == expected_word
    assert tuple((segment.kind, segment.text) for segment in result.segments) == (
        expected_segments
    )
    assert isinstance(result.segments, tuple)


def test_low_level_build_and_negative_morphology_fixture() -> None:
    result = jvozba.build_best_jvozba_detailed(
        jvozba.JvozbaMode.LUJVO,
        dictionary.english,
        [jvozba.Word("lojbo"), jvozba.Word("bangu")],
    )
    assert result.word == "jbobau"

    with pytest.raises(jvozba.CouldNotBuildLujvoError) as caught:
        jvozba.build(
            [
                jvozba.FixedRafsi("kerl"),
                jvozba.FixedRafsi("u'u"),
                jvozba.Word("kerlo"),
            ]
        )
    assert isinstance(caught.value.value, jvozba.CouldNotBuildLujvo)


@pytest.mark.parametrize(
    "raw_inputs",
    (
        "lojbo",
        ["lojbo", "bangu"],
        {jvozba.Word("lojbo"), jvozba.Word("bangu")},
        iter((jvozba.Word("lojbo"), jvozba.Word("bangu"))),
    ),
)
def test_composition_rejects_ambiguous_or_unordered_inputs(
    raw_inputs: object,
) -> None:
    with pytest.raises(TypeError):
        jvozba.build_best_jvozba_detailed(
            jvozba.JvozbaMode.LUJVO,
            dictionary.english,
            raw_inputs,  # type: ignore[arg-type]
        )


def test_composition_requires_exact_registered_enum_identity() -> None:
    class OtherMode(enum.StrEnum):
        LUJVO = "lujvo"

    inputs = [jvozba.Word("lojbo"), jvozba.Word("bangu")]
    for mode in ("lujvo", OtherMode.LUJVO):
        with pytest.raises(TypeError):
            jvozba.build_best_jvozba_detailed(
                mode,  # type: ignore[arg-type]
                dictionary.english,
                inputs,
            )


@pytest.mark.parametrize(
    ("inputs", "exception_type", "value_type", "offending"),
    (
        (
            (),
            jvozba.RequiresAtLeastTwoInputsError,
            jvozba.RequiresAtLeastTwoInputs,
            None,
        ),
        (
            (jvozba.Word("lojbo"), jvozba.FixedRafsi("")),
            jvozba.FixedRafsiEmptyError,
            jvozba.FixedRafsiEmpty,
            None,
        ),
        (
            (jvozba.FixedRafsi("klama"), jvozba.Word("bangu")),
            jvozba.NonFinalUniversalLongRafsiError,
            jvozba.NonFinalUniversalLongRafsi,
            "klama",
        ),
        (
            (jvozba.Word("klama"), jvozba.Word("a")),
            jvozba.FinalConsonantError,
            jvozba.FinalConsonant,
            "a",
        ),
        (
            (jvozba.Word("a"), jvozba.Word("klama")),
            jvozba.NoRafsiAvailableError,
            jvozba.NoRafsiAvailable,
            "a",
        ),
        (
            (jvozba.Word("lojbo"), jvozba.Word("notlojban")),
            jvozba.NoDictionaryEntryError,
            jvozba.NoDictionaryEntry,
            "notlojban",
        ),
        (
            (
                jvozba.FixedRafsi("kerl"),
                jvozba.FixedRafsi("u'u"),
                jvozba.Word("kerlo"),
            ),
            jvozba.CouldNotBuildLujvoError,
            jvozba.CouldNotBuildLujvo,
            None,
        ),
    ),
)
def test_every_reachable_rust_error_is_raised_end_to_end(
    inputs: Sequence[jvozba.JvozbaInput],
    exception_type: type[jvozba.JvozbaError],
    value_type: type[jvozba.JvozbaErrorValue],
    offending: str | None,
) -> None:
    with pytest.raises(exception_type) as caught:
        jvozba.build(inputs)

    exception = caught.value
    assert isinstance(exception.value, value_type)
    assert exception.args == (str(exception.value),)
    if offending is not None:
        assert isinstance(
            exception,
            (
                jvozba.NonFinalUniversalLongRafsiError,
                jvozba.FinalConsonantError,
                jvozba.NoRafsiAvailableError,
                jvozba.NoDictionaryEntryError,
            ),
        )
        assert exception.offending == offending
    if isinstance(exception, jvozba.FinalConsonantError):
        assert exception.is_fixed_rafsi is False


def test_all_error_values_and_variant_exceptions_remain_distinct() -> None:
    values: tuple[jvozba.JvozbaErrorValue, ...] = (
        jvozba.RequiresAtLeastTwoInputs(),
        jvozba.FixedRafsiEmpty(),
        jvozba.NonFinalUniversalLongRafsi("klama"),
        jvozba.FinalConsonant("rok", True),
        jvozba.NoRafsiAvailable("a"),
        jvozba.NoDictionaryEntry("missing"),
        jvozba.CouldNotBuildLujvo(),
        jvozba.CouldNotBuildCompound(),
    )
    exception_types: tuple[type[jvozba.JvozbaError], ...] = (
        jvozba.RequiresAtLeastTwoInputsError,
        jvozba.FixedRafsiEmptyError,
        jvozba.NonFinalUniversalLongRafsiError,
        jvozba.FinalConsonantError,
        jvozba.NoRafsiAvailableError,
        jvozba.NoDictionaryEntryError,
        jvozba.CouldNotBuildLujvoError,
        jvozba.CouldNotBuildCompoundError,
    )

    exceptions = tuple(
        exception_type(value)
        for exception_type, value in zip(exception_types, values, strict=True)
    )
    assert tuple(exception.value for exception in exceptions) == values
    assert tuple(type(exception) for exception in exceptions) == exception_types
    assert tuple(type(value) for value in values) == tuple(
        getattr(
            native,
            f"_jvozba_{type(value).__name__}",
        )
        for value in values
    )


def test_values_results_and_exceptions_are_immutable_and_final() -> None:
    word = jvozba.Word("lojbo")
    result = jvozba.build([word, jvozba.Word("bangu")])
    error = jvozba.FinalConsonantError(jvozba.FinalConsonant("rok", True))

    with pytest.raises(AttributeError):
        word.value = "bangu"  # type: ignore[misc]
    with pytest.raises(AttributeError):
        result.word = "other"  # type: ignore[misc]
    with pytest.raises(AttributeError):
        error.offending = "other"  # type: ignore[misc]
    with pytest.raises(TypeError):
        type("DerivedWord", (jvozba.Word,), {})
    with pytest.raises(TypeError):
        type("DerivedError", (jvozba.FinalConsonantError,), {})
    with pytest.raises(TypeError):
        type("InventedJvozbaError", (jvozba.JvozbaError,), {})


def test_binding_constructors_mirror_rust_invariants_exactly() -> None:
    empty = jvozba.FixedRafsi("")
    assert empty.value == ""
    with pytest.raises(jvozba.FixedRafsiEmptyError):
        jvozba.build([jvozba.Word("lojbo"), empty])

    with pytest.raises(jbotci.InvalidInputError):
        jvozba.JvozbaSegment(jvozba.JvozbaSegmentKind.RAFSI, "")
    with pytest.raises(jbotci.InvalidInputError):
        jvozba.JvozbaBuildResult("", [])

    permissive_segment = jvozba.JvozbaSegment(
        jvozba.JvozbaSegmentKind.RAFSI, "."
    )
    permissive_result = jvozba.JvozbaBuildResult(".", [])
    assert permissive_segment.text == permissive_result.word

    rafsi = morphology.LujvoRafsi(morphology.Phonemes("jbo"))
    hyphen = morphology.LujvoHyphen(morphology.Phonemes("y"))
    rafsi_without_source = jvozba.LujvoSegmentInfo(rafsi)
    with pytest.raises(jbotci.InvalidInputError):
        jvozba.LujvoSegmentInfo(hyphen, "invented")
    with pytest.raises(jbotci.InvalidInputError):
        jvozba.LujvoDecomposition([rafsi_without_source], [])

    decomposition = jvozba.LujvoDecomposition(
        [rafsi_without_source, rafsi_without_source], ["", ""]
    )
    assert decomposition.source_words == ("", "")


@pytest.mark.parametrize(
    ("raw_word", "expected_segments", "expected_source_words"),
    (
        (
            "jetcybolxada",
            (
                (morphology.LujvoRafsi, "jetc", "jetce"),
                (morphology.LujvoHyphen, "y", None),
                (morphology.LujvoRafsi, "bolxáda", "bolxada"),
            ),
            ("jetce", "bolxada"),
        ),
        (
            "jenjigu'ydi'e",
            (
                (morphology.LujvoRafsi, "jenjigu", "jenjigu"),
                (morphology.LujvoHyphen, "'y", None),
                (morphology.LujvoRafsi, "dí'e", "dirce"),
            ),
            ("jenjigu", "dirce"),
        ),
        (
            "ci'artai",
            (
                (morphology.LujvoRafsi, "ci'á", "ciska"),
                (morphology.LujvoHyphen, "r", None),
                (morphology.LujvoRafsi, "taĭ", "tarmi"),
            ),
            ("ciska", "tarmi"),
        ),
        (
            "jetrok",
            (
                (morphology.LujvoRafsi, "jet", "jetnu"),
                (morphology.LujvoRafsi, "rok", "rokci"),
            ),
            ("jetnu", "rokci"),
        ),
        (
            "baunrok",
            (
                (morphology.LujvoRafsi, "bau", "bangu"),
                (morphology.LujvoHyphen, "n", None),
                (morphology.LujvoRafsi, "rok", "rokci"),
            ),
            ("bangu", "rokci"),
        ),
    ),
)
def test_decomposition_preserves_exact_morphology_parts_and_sources(
    raw_word: str,
    expected_segments: tuple[
        tuple[type[morphology.LujvoPart], str, str | None], ...
    ],
    expected_source_words: tuple[str, ...],
) -> None:
    decomposition = jvozba.decompose_lujvo_like(
        raw_word, dictionary=dictionary.english
    )
    assert decomposition is not None
    assert tuple(
        (type(info.segment), info.segment.phonemes.text, info.source)
        for info in decomposition.segments
    ) == expected_segments
    assert decomposition.source_words == expected_source_words
    assert all(
        info.source is None
        for info in decomposition.segments
        if isinstance(info.segment, morphology.LujvoHyphen)
    )


def test_decomposition_normalization_and_nondecomposable_result() -> None:
    expected = jvozba.decompose_lujvo_like("jenjigu'ydi'e")
    assert expected is not None
    for variant in (
        ".JENJIGUHYDIHE.",
        "jenjigu’ydi’e",
        "  jenjigu'ydi'e  ",
    ):
        assert jvozba.decompose_lujvo_like(variant) == expected
    assert jvozba.decompose_lujvo_like("klama") is None


def test_decomposition_owns_dictionary_source_strings() -> None:
    decomposition = jvozba.decompose_lujvo_like(
        "jetcybolxada", dictionary=dictionary.english
    )
    assert decomposition is not None
    segments = decomposition.segments
    source_words = decomposition.source_words
    del decomposition
    gc.collect()

    assert tuple(segment.source for segment in segments) == (
        "jetce",
        None,
        "bolxada",
    )
    assert source_words == ("jetce", "bolxada")


def test_can_use_word_direct_and_ergonomic_forms() -> None:
    assert jvozba.word_can_enter_jvozba_pane(
        dictionary.english, "klama"
    )
    assert jvozba.can_use_word("klama", dictionary=dictionary.english)
    assert not jvozba.word_can_enter_jvozba_pane(
        dictionary.english, "notlojban"
    )
    assert not jvozba.can_use_word(
        "notlojban", dictionary=dictionary.english
    )


def test_constructor_shaped_reprs_round_trip() -> None:
    result = jvozba.build([jvozba.Word("lojbo"), jvozba.Word("bangu")])
    decomposition = jvozba.decompose_lujvo_like("jetcybolxada")
    assert decomposition is not None

    namespace = {"jbotci": jbotci}
    assert eval(repr(result), namespace) == result
    assert eval(repr(decomposition), namespace) == decomposition
    error_value = jvozba.FinalConsonant("r'ok", True)
    assert eval(repr(error_value), namespace) == error_value


def test_installed_jvozba_import_works_outside_repository(
    tmp_path: Path,
) -> None:
    result = subprocess.run(
        [
            sys.executable,
            "-c",
            (
                "from jbotci import jvozba; "
                "result = jvozba.build(["
                "jvozba.Word('lojbo'), jvozba.Word('bangu')]); "
                "assert result.word == 'jbobau'; "
                "assert jvozba.decompose_lujvo_like('jetcybolxada')"
            ),
        ],
        cwd=tmp_path,
        check=False,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stdout + result.stderr
