from __future__ import annotations

import gc
import json
import tomllib
from collections.abc import Sequence
from pathlib import Path
from typing import cast

import pytest

from jbotci import InvalidInputError, diagnostics, dialect, morphology, source


WORKSPACE_ROOT = Path(__file__).resolve().parents[3]


def word_fixture_value(word: morphology.Word) -> dict[str, object]:
    """Project public word fields into the independently authored fixture shape."""

    span = list(word.span.char_range)
    if isinstance(word, morphology.LujvoWord):
        parts = [
            {
                "Rafsi" if isinstance(part, morphology.LujvoRafsi) else "Hyphen": (
                    part.phonemes.text
                )
            }
            for part in word.parts
        ]
        payload: dict[str, object] = {"parts": parts, "span": span}
    else:
        payload = {"phonemes": word.phonemes.text, "span": span}
    names: tuple[tuple[type[object], str], ...] = (
        (morphology.CmavoWord, "Cmavo"),
        (morphology.GismuWord, "Gismu"),
        (morphology.LujvoWord, "Lujvo"),
        (morphology.FuhivlaWord, "Fuhivla"),
        (morphology.CmevlaWord, "Cmevla"),
    )
    for word_type, name in names:
        if isinstance(word, word_type):
            return {name: payload}
    raise AssertionError(f"unhandled word type: {type(word)!r}")


def word_like_fixture_value(value: morphology.WordLike) -> dict[str, object]:
    """Project every public word-like payload recursively into fixture JSON."""

    if isinstance(value, morphology.PlainWord):
        payload: object = word_fixture_value(value.word)
        name = "PlainWord"
    elif isinstance(value, morphology.QuotedWord):
        payload = {
            "zo": word_fixture_value(value.zo),
            "word": word_fixture_value(value.word),
        }
        name = "QuotedWord"
    elif isinstance(value, morphology.SelmahoQuotedWord):
        payload = {
            "mahoi": word_fixture_value(value.mahoi),
            "word": word_fixture_value(value.word),
        }
        name = "SelmahoQuotedWord"
    elif isinstance(value, morphology.DelimitedNonLojbanQuote):
        payload = {
            "zoi": word_fixture_value(value.zoi),
            "opening_delimiter": word_fixture_value(value.opening_delimiter),
            "quoted_text": {
                "span": list(value.quoted_text.span.char_range),
                "text": value.quoted_text.text,
            },
            "closing_delimiter": word_fixture_value(value.closing_delimiter),
        }
        name = "DelimitedNonLojbanQuote"
    elif isinstance(value, morphology.QuotedWords):
        payload = {
            "lohu": word_fixture_value(value.lohu),
            "quoted_words": [
                word_fixture_value(word) for word in value.quoted_words
            ],
            "lehu": word_fixture_value(value.lehu),
        }
        name = "QuotedWords"
    elif isinstance(value, morphology.DelimitedWordQuote):
        payload = {
            "marker": word_fixture_value(value.marker),
            "quoted_text": {
                "span": list(value.quoted_text.span.char_range),
                "text": value.quoted_text.text,
            },
        }
        name = "DelimitedWordQuote"
    elif isinstance(value, morphology.LerfuWord):
        payload = {
            "base": word_like_fixture_value(value.base),
            "bu": word_fixture_value(value.bu),
        }
        name = "LerfuWord"
    elif isinstance(value, morphology.ZeiCompound):
        payload = {
            "left": word_like_fixture_value(value.left),
            "zei": word_fixture_value(value.zei),
            "right": word_fixture_value(value.right),
        }
        name = "ZeiCompound"
    else:
        raise AssertionError(f"unhandled word-like type: {type(value)!r}")
    return {name: payload}


def plain_classification_value(
    value: morphology.PlainWordClassification,
) -> tuple[object, ...]:
    """Read every field of a plain classification into a literal oracle shape."""

    return (
        value.category.value,
        value.phonemes,
        value.selmaho,
        value.split,
        tuple(
            (
                part.kind.value,
                part.text,
                None if part.rafsi_kind is None else part.rafsi_kind.value,
            )
            for part in value.parts
        ),
        None if value.stage is None else value.stage.value,
    )


def classification_value(
    value: morphology.ValsiClassification,
) -> tuple[object, ...]:
    """Read every classification variant field recursively without converters."""

    if isinstance(value, morphology.PlainWordValsiClassification):
        payload: tuple[object, ...] = (plain_classification_value(value.word),)
    elif isinstance(value, morphology.QuotedWordValsiClassification):
        payload = (
            plain_classification_value(value.marker),
            plain_classification_value(value.quoted_word),
        )
    elif isinstance(
        value, morphology.DelimitedNonLojbanQuoteValsiClassification
    ):
        payload = (plain_classification_value(value.marker), value.delimiter)
    elif isinstance(value, morphology.QuotedWordsValsiClassification):
        payload = (
            plain_classification_value(value.marker),
            tuple(plain_classification_value(word) for word in value.quoted_words),
        )
    elif isinstance(value, morphology.DelimitedWordQuoteValsiClassification):
        payload = (value.marker_text,)
    elif isinstance(value, morphology.LerfuWordValsiClassification):
        payload = (
            classification_value(value.base),
            plain_classification_value(value.suffix),
        )
    elif isinstance(value, morphology.ZeiCompoundValsiClassification):
        payload = (
            classification_value(value.left),
            plain_classification_value(value.link),
            plain_classification_value(value.right),
        )
    else:
        raise AssertionError(f"unhandled classification type: {type(value)!r}")
    return (value.kind.value, *payload)


def expected_plain_classification(
    category: str, phonemes: str, selmaho: str | None = None
) -> tuple[object, ...]:
    """Build an independently authored expected plain-classification record."""

    return (category, phonemes, selmaho, None, (), None)


def plain_phonemes(words: tuple[morphology.WordLike, ...]) -> tuple[str, ...]:
    values: list[str] = []
    for value in words:
        assert isinstance(value, morphology.PlainWord)
        values.append(value.word.phonemes.text)
    return tuple(values)


@pytest.mark.parametrize(
    "relative_path",
    (
        "adhoc/v0/morphology/basic/simple-cmavo.toml",
        "adhoc/v0/morphology/basic/simple-brivla.toml",
        "adhoc/v0/morphology/basic/smart-apostrophe-lujvo.toml",
        "adhoc/v0/morphology/streaming/unstressed-klamami-stays-one-word.toml",
        "adhoc/v0/morphology/basic/simple-cmevla.toml",
        "adhoc/v0/morphology/basic/zo-quotes-brivla.toml",
        "adhoc/v0/morphology/success/adjacent-zoi-corpus-shape-parses.toml",
        "adhoc/v0/morphology/basic/lohu-quotes-words.toml",
        "adhoc/v0/morphology/basic/bu-creates-lerfu.toml",
        "adhoc/v0/morphology/basic/zei-creates-brivla.toml",
    ),
)
def test_binding_projection_matches_independent_morphology_fixtures(
    relative_path: str,
) -> None:
    """Keep binding locators independent from the Rust-to-Python converters."""

    fixture_path = WORKSPACE_ROOT / "tests" / "fixtures" / relative_path
    fixture = tomllib.loads(fixture_path.read_text(encoding="utf-8"))
    fixture_source = fixture["lojban"]
    fixture_json = fixture["expectations"]["output"]["vlasei"]["json"]
    assert isinstance(fixture_source, str)
    assert isinstance(fixture_json, str)

    source_id = source.SourceId("<fixture>")
    projected = morphology.segment(fixture_source, source_id=source_id)
    assert [word_like_fixture_value(value) for value in projected] == json.loads(
        fixture_json
    )
    assert all(
        span.source_id == source_id
        for value in projected
        for span in value.source_spans
    )


def test_solid_text_and_unicode_spans_preserve_source_ids() -> None:
    source_id = source.SourceId("solid")
    words = morphology.segment("mimi", source_id=source_id)
    assert plain_phonemes(words) == ("mi", "mi")
    assert tuple(word.word.span.char_range for word in words) == ((0, 2), (2, 4))
    assert all(word.word.span.source_id == source_id for word in words)

    cyrillic = morphology.segment("ми", source_id=source_id)
    assert isinstance(cyrillic[0], morphology.PlainWord)
    assert cyrillic[0].word.span.byte_range == (0, 4)
    assert cyrillic[0].word.span.char_range == (0, 2)


@pytest.mark.parametrize(
    ("text", "word_type"),
    (
        ("mi", morphology.CmavoWord),
        ("klama", morphology.GismuWord),
        ("tci'ilykemcantutra", morphology.LujvoWord),
        ("spageti", morphology.FuhivlaWord),
        (".alis.", morphology.CmevlaWord),
    ),
)
def test_every_plain_word_kind(text: str, word_type: type[object]) -> None:
    (value,) = morphology.segment(text)
    assert isinstance(value, morphology.PlainWord)
    assert isinstance(value.word, word_type)
    assert value.word.phonemes.text
    assert value.word.key.kind == value.word.kind


def test_all_quote_and_modifier_word_like_variants() -> None:
    assert isinstance(morphology.segment("zo broda")[0], morphology.QuotedWord)
    source_id = source.SourceId("literal-quote-fields")
    mahoi = morphology.segment("ma'oi ba", source_id=source_id)[0]
    assert isinstance(mahoi, morphology.SelmahoQuotedWord)
    assert mahoi.mahoi.phonemes.text == "ma'oĭ"
    assert mahoi.mahoi.span.byte_range == (0, 5)
    assert mahoi.mahoi.span.char_range == (0, 5)
    assert mahoi.word.phonemes.text == "ba"
    assert mahoi.word.span.byte_range == (6, 8)
    assert mahoi.word.span.char_range == (6, 8)
    assert all(span.source_id == source_id for span in mahoi.source_spans)
    zoi = morphology.segment("zoi gy hello world gy")[0]
    assert isinstance(zoi, morphology.DelimitedNonLojbanQuote)
    assert zoi.quoted_text.text == "hello world"
    assert isinstance(morphology.segment("lo'u mi do le'u")[0], morphology.QuotedWords)
    zohoi = morphology.segment("zo'oi hello", source_id=source_id)[0]
    assert isinstance(zohoi, morphology.DelimitedWordQuote)
    assert zohoi.marker.phonemes.text == "zo'oĭ"
    assert zohoi.marker.span.byte_range == (0, 5)
    assert zohoi.marker.span.char_range == (0, 5)
    assert zohoi.quoted_text.text == "hello"
    assert zohoi.quoted_text.span.byte_range == (6, 11)
    assert zohoi.quoted_text.span.char_range == (6, 11)
    assert all(span.source_id == source_id for span in zohoi.source_spans)
    assert isinstance(morphology.segment("a bu")[0], morphology.LerfuWord)
    assert isinstance(morphology.segment("broda zei brode")[0], morphology.ZeiCompound)


@pytest.mark.parametrize(
    ("text", "word_type", "byte_range", "span_ranges"),
    (
        ("mi", morphology.PlainWord, (0, 2), (((0, 2), (0, 2)),)),
        (
            "zo broda",
            morphology.QuotedWord,
            (0, 8),
            (((0, 2), (0, 2)), ((3, 8), (3, 8))),
        ),
        (
            "ma'oi ba",
            morphology.SelmahoQuotedWord,
            (0, 8),
            (((0, 5), (0, 5)), ((6, 8), (6, 8))),
        ),
        (
            "zoi gy café gy",
            morphology.DelimitedNonLojbanQuote,
            (0, 15),
            (
                ((0, 3), (0, 3)),
                ((4, 6), (4, 6)),
                ((7, 12), (7, 11)),
                ((13, 15), (12, 14)),
            ),
        ),
        (
            "lo'u mi do le'u",
            morphology.QuotedWords,
            (0, 15),
            (
                ((0, 4), (0, 4)),
                ((5, 7), (5, 7)),
                ((8, 10), (8, 10)),
                ((11, 15), (11, 15)),
            ),
        ),
        (
            "zo'oi hello",
            morphology.DelimitedWordQuote,
            (0, 11),
            (((0, 5), (0, 5)), ((6, 11), (6, 11))),
        ),
        (
            "a bu",
            morphology.LerfuWord,
            (0, 4),
            (((0, 1), (0, 1)), ((2, 4), (2, 4))),
        ),
        (
            "broda zei brode",
            morphology.ZeiCompound,
            (0, 15),
            (((0, 5), (0, 5)), ((6, 9), (6, 9)), ((10, 15), (10, 15))),
        ),
    ),
)
def test_every_word_like_has_exact_ordered_source_spans(
    text: str,
    word_type: type[object],
    byte_range: tuple[int, int],
    span_ranges: tuple[tuple[tuple[int, int], tuple[int, int]], ...],
) -> None:
    source_id = source.SourceId(f"word-like:{text}")
    (value,) = morphology.segment(text, source_id=source_id)
    assert isinstance(value, word_type)
    assert value.byte_range == byte_range
    assert len(value.source_spans) == len(span_ranges)
    assert tuple(
        (span.byte_range, span.char_range, span.source_id)
        for span in value.source_spans
    ) == tuple(
        (expected_bytes, expected_chars, source_id)
        for expected_bytes, expected_chars in span_ranges
    )


def test_erasure_and_display_segmentation_preserve_their_rust_distinction() -> None:
    assert plain_phonemes(morphology.segment("mi si do")) == ("do",)
    assert plain_phonemes(morphology.segment_for_display("mi si do")) == (
        "mi",
        "si",
        "do",
    )


def test_children_keep_arc_root_alive_after_parent_deletion() -> None:
    parent = morphology.segment("zoi gy hello gy")[0]
    assert isinstance(parent, morphology.DelimitedNonLojbanQuote)
    verbatim = parent.quoted_text
    delimiter = parent.opening_delimiter
    del parent
    gc.collect()
    assert verbatim.text == "hello"
    assert delimiter.phonemes.text == "gy"

    lujvo_parent = morphology.segment("tci'ilykemcantutra")[0]
    assert isinstance(lujvo_parent, morphology.PlainWord)
    assert isinstance(lujvo_parent.word, morphology.LujvoWord)
    part = lujvo_parent.word.parts[0]
    del lujvo_parent
    gc.collect()
    assert part.phonemes.text

    compiled = morphology.CompiledDialectDefinition(
        dialect.DialectDefinition((dialect.CmavoSwap("ce'u", "ce"),))
    )
    compiled_word = compiled.entries[0].left.word
    del compiled
    gc.collect()
    assert compiled_word.phonemes.text == "ce'u"

    options = morphology.MorphologyOptions(
        dialect=dialect.DialectDefinition((dialect.CmavoSwap("ce'u", "ce"),))
    )
    options_compiled = options.compiled_dialect
    options_word = options_compiled.entries[0].left.word
    del options
    del options_compiled
    gc.collect()
    assert options_word == morphology.CmavoWord(
        options_word.phonemes, options_word.span
    )


def test_dialect_compilation_errors_retain_exact_rust_payloads() -> None:
    invalid_word = "aaa"
    definition = dialect.DialectDefinition(
        (dialect.CmavoSwap(invalid_word, "coi"),)
    )
    operations = (
        lambda: morphology.CompiledDialectDefinition(definition),
        lambda: morphology.MorphologyOptions(dialect=definition),
        lambda: morphology.MorphologyOptions().with_dialect(definition),
    )
    for operation in operations:
        with pytest.raises(morphology.DialectCompilationError) as caught:
            operation()
        detail = caught.value.value
        assert detail == morphology.InvalidDialectWord(invalid_word)
        assert hash(detail) == hash(morphology.InvalidDialectWord(invalid_word))
        assert detail.word == invalid_word
        assert caught.value.args == (str(detail),)
        assert str(detail) == (
            f"dialect word is not morphologically valid: {invalid_word}"
        )
        assert repr(detail) == (
            "jbotci.morphology.InvalidDialectWord(word='aaa')"
        )
        match detail:
            case morphology.InvalidDialectWord(word):
                assert word == invalid_word
            case _:
                pytest.fail("dialect error did not retain its exact word")
        match caught.value:
            case morphology.DialectCompilationError(
                morphology.InvalidDialectWord(word)
            ):
                assert word == invalid_word
            case _:
                pytest.fail("dialect exception did not expose its typed value")

    unicode_detail = morphology.InvalidDialectWord("café")
    unicode_error = morphology.DialectCompilationError(unicode_detail)
    assert unicode_error.value.word == "café"
    assert unicode_error.args == (str(unicode_detail),)
    assert morphology.InvalidDialectWord.__module__ == "jbotci.morphology"
    assert morphology.DialectCompilationError.__module__ == "jbotci.morphology"
    assert morphology.DialectCompilationError.__qualname__ == (
        "DialectCompilationError"
    )
    with pytest.raises(InvalidInputError):
        morphology.InvalidDialectWord("")
    with pytest.raises(AttributeError):
        unicode_error.value = morphology.InvalidDialectWord("coi")  # type: ignore[misc]
    with pytest.raises(AttributeError):
        unicode_error.args = ("changed",)
    with pytest.raises(TypeError):
        morphology.DialectCompilationError("invalid")  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        type(
            "DerivedDialectCompilationError",
            (morphology.DialectCompilationError,),
            {},
        )
    with pytest.raises(TypeError):
        type("DerivedInvalidDialectWord", (morphology.InvalidDialectWord,), {})


def test_locator_backed_children_compare_by_projected_rust_values() -> None:
    quote = morphology.segment("zo broda")[0]
    assert isinstance(quote, morphology.QuotedWord)
    reconstructed_word = morphology.GismuWord(quote.word.phonemes, quote.word.span)
    assert quote.word == reconstructed_word

    zoi = morphology.segment("zoi gy hello gy")[0]
    assert isinstance(zoi, morphology.DelimitedNonLojbanQuote)
    assert zoi.quoted_text == morphology.Verbatim(
        zoi.quoted_text.span, zoi.quoted_text.text
    )

    lujvo = morphology.segment("tci'ilykemcantutra")[0]
    assert isinstance(lujvo, morphology.PlainWord)
    assert isinstance(lujvo.word, morphology.LujvoWord)
    located_part = lujvo.word.parts[0]
    if isinstance(located_part, morphology.LujvoRafsi):
        rebuilt_part: morphology.LujvoPart = morphology.LujvoRafsi(
            located_part.phonemes
        )
    else:
        rebuilt_part = morphology.LujvoHyphen(located_part.phonemes)
    assert located_part == rebuilt_part

    analysis = morphology.analyze_valsi("broda zei brode")
    classification = analysis.result.classification
    assert isinstance(classification, morphology.ZeiCompoundValsiClassification)
    left = classification.left
    assert isinstance(left, morphology.PlainWordValsiClassification)
    assert left == morphology.PlainWordValsiClassification(left.word)


def test_attempt_warning_trace_and_recovery_are_distinct() -> None:
    trace = morphology.MorphologyOptions(trace=diagnostics.TraceOptions(enabled=True))
    attempt = morphology.segment_attempt("mi", options=trace)
    assert attempt.succeeded
    assert attempt.words is not None
    assert attempt.trace is not None

    permissive = morphology.MorphologyOptions(permissive_lexer=True)
    warned = morphology.segment_attempt("xu@no", options=permissive)
    assert plain_phonemes(warned.words or ()) == ("xu", "no")
    assert warned.warnings[0].kind is morphology.MorphologyWarningKind.IGNORED_CHARACTERS

    recovered = morphology.segment_recovered("mi @@@ do")
    assert plain_phonemes(recovered.words) == ("mi", "do")
    assert len(recovered.errors) == len(recovered.error_regions) == 1
    assert recovered.error_regions[0].char_range == (3, 7)


def test_every_morphology_option_round_trips_and_rejects_zero_caps() -> None:
    trace = diagnostics.TraceOptions(enabled=True)
    options = morphology.MorphologyOptions(
        accept_latin=False,
        accept_cyrillic=False,
        accept_zbalermorna=False,
        cmevla_as_relation_words=True,
        permissive_lexer=True,
        uppercase_marks_stress=False,
        max_recovery_errors=7,
        trace=trace,
    )
    assert not options.accept_latin
    assert not options.accept_cyrillic
    assert not options.accept_zbalermorna
    assert options.cmevla_as_relation_words
    assert options.permissive_lexer
    assert not options.uppercase_marks_stress
    assert options.max_recovery_errors == 7
    assert options.trace == trace
    assert options.compiled_dialect.entries == ()

    definition = dialect.DialectDefinition(
        [dialect.CmavoSwap("ce'u", "ce")]
    )
    compiled = morphology.CompiledDialectDefinition(definition)
    assert options.with_compiled_dialect(compiled).compiled_dialect == compiled
    assert options.with_dialect(definition).compiled_dialect == compiled
    assert options.with_trace(diagnostics.TraceOptions()).trace.enabled is False
    assert options.with_max_recovery_errors(3).max_recovery_errors == 3

    with pytest.raises(InvalidInputError, match="greater than zero"):
        morphology.MorphologyOptions(max_recovery_errors=0)
    with pytest.raises(InvalidInputError, match="greater than zero"):
        options.with_max_recovery_errors(0)


def test_each_morphology_option_changes_real_parser_behavior() -> None:
    latin_only = morphology.MorphologyOptions(
        accept_latin=True,
        accept_cyrillic=False,
        accept_zbalermorna=False,
    )
    cyrillic_only = morphology.MorphologyOptions(
        accept_latin=False,
        accept_cyrillic=True,
        accept_zbalermorna=False,
    )
    zbalermorna_only = morphology.MorphologyOptions(
        accept_latin=False,
        accept_cyrillic=False,
        accept_zbalermorna=True,
    )
    none = morphology.MorphologyOptions(
        accept_latin=False,
        accept_cyrillic=False,
        accept_zbalermorna=False,
    )
    for text, accepted, rejected_glyph in (
        ("mi", latin_only, "m"),
        ("ми", cyrillic_only, "м"),
        ("\ued87\ueda2", zbalermorna_only, "\ued87"),
    ):
        accepted_attempt = morphology.segment_attempt(text, options=accepted)
        assert accepted_attempt.succeeded
        assert plain_phonemes(accepted_attempt.words or ()) == ("mi",)
        rejected = morphology.segment_attempt(text, options=none)
        assert not rejected.succeeded
        assert isinstance(rejected.error, morphology.InvalidMorphology)
        assert (
            rejected.error.kind
            is morphology.MorphologyErrorKind.INVALID_CHARACTER
        )
        assert (rejected.error.char_start, rejected.error.char_end) == (0, 1)
        assert rejected.error.text == rejected_glyph

    cbm_source = "mi .alis. do sa broda"
    ordinary = morphology.segment(
        cbm_source,
        options=morphology.MorphologyOptions(cmevla_as_relation_words=False),
    )
    cbm = morphology.segment(
        cbm_source,
        options=morphology.MorphologyOptions(cmevla_as_relation_words=True),
    )
    assert plain_phonemes(ordinary) == ("bróda",)
    assert plain_phonemes(cbm) == ("mi", "bróda")

    stressed = morphology.segment(
        "finYks",
        options=morphology.MorphologyOptions(uppercase_marks_stress=True),
    )
    unstressed = morphology.segment(
        "finYks",
        options=morphology.MorphologyOptions(uppercase_marks_stress=False),
    )
    assert plain_phonemes(stressed) == ("finýks",)
    assert plain_phonemes(unstressed) == ("finyks",)

    recovery_source = "mi @@@ do ### mi"
    one = morphology.segment_recovered(
        recovery_source,
        options=morphology.MorphologyOptions(max_recovery_errors=1),
    )
    two = morphology.segment_recovered(
        recovery_source,
        options=morphology.MorphologyOptions(max_recovery_errors=2),
    )
    assert len(one.errors) == 1
    assert len(two.errors) == 2
    assert plain_phonemes(one.words) == ("mi",)
    assert plain_phonemes(two.words) == ("mi", "do", "mi")
    assert tuple(region.char_range for region in one.error_regions) == ((3, 7),)
    assert tuple(region.char_range for region in two.error_regions) == (
        (3, 7),
        (10, 14),
    )


def test_display_attempt_preserves_real_warnings_trace_and_failure() -> None:
    dialect_options = morphology.MorphologyOptions(
        dialect=dialect.DialectDefinition(
            [dialect.CmavoSwap("ce'u", "ce")]
        )
    )
    projected = morphology.segment_for_display_attempt(
        "ce'u", options=dialect_options
    )
    assert projected.succeeded
    assert plain_phonemes(projected.words or ()) == ("ce",)

    options = morphology.MorphologyOptions(
        permissive_lexer=True,
        trace=diagnostics.TraceOptions(enabled=True),
    )
    warned = morphology.segment_for_display_attempt("xu@no", options=options)
    assert warned.succeeded
    assert plain_phonemes(warned.words or ()) == ("xu", "no")
    assert tuple(warning.kind for warning in warned.warnings) == (
        morphology.MorphologyWarningKind.IGNORED_CHARACTERS,
    )
    assert warned.warnings[0].text == "@"
    assert warned.trace is not None
    assert warned.trace.phase is diagnostics.TracePhase.MORPHOLOGY
    assert warned.trace.events

    failed = morphology.segment_for_display_attempt("aa", options=options)
    assert not failed.succeeded
    assert isinstance(failed.error, morphology.InvalidMorphology)
    assert failed.error.kind is morphology.MorphologyErrorKind.VOWEL_HIATUS
    assert failed.trace is not None
    assert failed.trace.phase is diagnostics.TracePhase.MORPHOLOGY
    assert any(
        event.kind is diagnostics.TraceEventKind.MORPHOLOGY_FAILURE
        for event in failed.trace.events
    )


def test_morphology_diagnostic_conversion_checks_every_source_range() -> None:
    context = morphology.MorphologyContext(
        morphology.MorphologyContextKind.CMAVO, 0, 1
    )
    warning = morphology.MorphologyWarning(
        morphology.MorphologyWarningKind.EXPERIMENTAL_CGV,
        0,
        1,
        "é",
        context=context,
    )
    warning_diagnostic = warning.to_diagnostic("éx")
    assert warning_diagnostic.labels[0].span.byte_range == (0, 2)
    assert warning_diagnostic.labels[0].span.char_range == (0, 1)
    assert warning_diagnostic.labels[1].span.byte_range == (0, 2)
    assert morphology.MorphologyWarning(
        morphology.MorphologyWarningKind.EXPERIMENTAL_CGV, 1, 2, "x"
    ).to_diagnostic("éx").labels[0].span.byte_range == (2, 3)

    with pytest.raises(InvalidInputError, match="does not match source text"):
        warning.to_diagnostic("ax")
    with pytest.raises(InvalidInputError):
        morphology.MorphologyContext(
            morphology.MorphologyContextKind.CMAVO, 1, 0
        )
    with pytest.raises(InvalidInputError):
        morphology.MorphologyWarning(
            morphology.MorphologyWarningKind.EXPERIMENTAL_CGV, 1, 0, "é"
        )
    for out_of_bounds in (
        morphology.MorphologyWarning(
            morphology.MorphologyWarningKind.EXPERIMENTAL_CGV, 2, 3, "x"
        ),
        morphology.MorphologyWarning(
            morphology.MorphologyWarningKind.EXPERIMENTAL_CGV, 0, 2, "éx"
        ),
    ):
        with pytest.raises(InvalidInputError):
            out_of_bounds.to_diagnostic("é")
        with pytest.raises(InvalidInputError):
            out_of_bounds.to_diagnostic("")
    with pytest.raises(InvalidInputError, match="morphology context"):
        morphology.MorphologyWarning(
            morphology.MorphologyWarningKind.EXPERIMENTAL_CGV,
            0,
            1,
            "é",
            context=morphology.MorphologyContext(
                morphology.MorphologyContextKind.CMAVO, 1, 2
            ),
        ).to_diagnostic("é")

    invalid = morphology.InvalidMorphology(
        morphology.MorphologyErrorKind.INVALID_CHARACTER,
        0,
        1,
        "é",
        context=context,
    )
    invalid_diagnostic = invalid.to_diagnostic("éx")
    assert invalid_diagnostic.labels[0].span.byte_range == (0, 2)
    assert invalid_diagnostic.labels[0].span.char_range == (0, 1)
    assert invalid_diagnostic.labels[1].span.byte_range == (0, 2)
    assert morphology.InvalidMorphology(
        morphology.MorphologyErrorKind.EXPECTED_WORD, 2, 2, ""
    ).to_diagnostic("éx").labels[0].span.byte_range == (3, 3)

    invalid_inputs = (
        (
            morphology.InvalidMorphology(
                morphology.MorphologyErrorKind.INVALID_CHARACTER, 1, 0, ""
            ),
            "é",
        ),
        (
            morphology.InvalidMorphology(
                morphology.MorphologyErrorKind.INVALID_CHARACTER, 2, 2, ""
            ),
            "é",
        ),
        (
            morphology.InvalidMorphology(
                morphology.MorphologyErrorKind.INVALID_CHARACTER, 0, 2, "éx"
            ),
            "é",
        ),
        (
            morphology.InvalidMorphology(
                morphology.MorphologyErrorKind.INVALID_CHARACTER, 0, 1, "é"
            ),
            "",
        ),
        (
            morphology.InvalidMorphology(
                morphology.MorphologyErrorKind.INVALID_CHARACTER, 0, 1, "x"
            ),
            "é",
        ),
        (
            morphology.InvalidMorphology(
                morphology.MorphologyErrorKind.INVALID_CHARACTER,
                0,
                1,
                "é",
                context=morphology.MorphologyContext(
                    morphology.MorphologyContextKind.CMAVO, 1, 2
                ),
            ),
            "é",
        ),
    )
    for value, supplied_source in invalid_inputs:
        with pytest.raises(InvalidInputError):
            value.to_diagnostic(supplied_source)

    unterminated = morphology.UnterminatedZoiQuote(1, "gy", context=context)
    unterminated_diagnostic = unterminated.to_diagnostic("éx")
    assert unterminated_diagnostic.labels[0].span.byte_range == (2, 3)
    assert unterminated_diagnostic.labels[0].span.char_range == (1, 2)
    assert morphology.UnterminatedZoiQuote(2, "gy").to_diagnostic(
        "éx"
    ).labels[0].span.byte_range == (3, 3)
    assert morphology.UnterminatedZoiQuote(0, "gy").to_diagnostic(
        ""
    ).labels[0].span.byte_range == (0, 0)
    with pytest.raises(InvalidInputError):
        morphology.UnterminatedZoiQuote(2, "gy").to_diagnostic("é")
    with pytest.raises(InvalidInputError, match="morphology context"):
        morphology.UnterminatedZoiQuote(
            0,
            "gy",
            context=morphology.MorphologyContext(
                morphology.MorphologyContextKind.DELIMITED_NON_LOJBAN_QUOTE,
                1,
                2,
            ),
        ).to_diagnostic("")

    assert morphology.SourceSpanMorphologyError(source.ZeroLine()).to_diagnostic(
        ""
    ).labels[0].span.byte_range == (0, 0)
    assert morphology.SourceSpanMorphologyError(source.ZeroLine()).to_diagnostic(
        "é"
    ).labels[0].span.char_range == (0, 0)


def test_morphology_error_construction_uses_checked_diagnostic_conversion() -> None:
    value = morphology.InvalidMorphology(
        morphology.MorphologyErrorKind.INVALID_CHARACTER, 1, 0, ""
    )
    with pytest.raises(InvalidInputError):
        morphology.MorphologyError(value, "é", None)


def test_morphology_error_validates_and_copies_warning_sequences() -> None:
    value = morphology.InvalidMorphology(
        morphology.MorphologyErrorKind.INVALID_CHARACTER, 0, 1, "x"
    )
    warning = morphology.MorphologyWarning(
        morphology.MorphologyWarningKind.EXPERIMENTAL_CGV, 0, 1, "x"
    )
    trace = diagnostics.TraceReport(diagnostics.TracePhase.MORPHOLOGY)

    warning_list = [warning]
    from_list = morphology.MorphologyError(value, "x", None, warning_list, trace)
    warning_list.clear()
    assert from_list.warnings == (warning,)
    assert from_list.trace is trace

    from_tuple = morphology.MorphologyError(value, "x", None, (warning,), trace)
    assert from_tuple.warnings == (warning,)
    assert from_tuple.trace is trace


def test_morphology_error_rejects_non_sequence_warning_inputs() -> None:
    value = morphology.InvalidMorphology(
        morphology.MorphologyErrorKind.INVALID_CHARACTER, 0, 1, "x"
    )
    warning = morphology.MorphologyWarning(
        morphology.MorphologyWarningKind.EXPERIMENTAL_CGV, 0, 1, "x"
    )
    invalid_inputs: tuple[object, ...] = (
        "warning",
        b"warning",
        bytearray(b"warning"),
        set(),
        (item for item in (warning,)),
        object(),
    )

    for invalid in invalid_inputs:
        with pytest.raises(
            TypeError,
            match="^warnings must be an ordered Sequence of MorphologyWarning$",
        ):
            morphology.MorphologyError(
                value,
                "x",
                None,
                cast(Sequence[morphology.MorphologyWarning], invalid),
            )


def test_morphology_error_rejects_invalid_warning_elements_and_trace() -> None:
    value = morphology.InvalidMorphology(
        morphology.MorphologyErrorKind.INVALID_CHARACTER, 0, 1, "x"
    )
    warning = morphology.MorphologyWarning(
        morphology.MorphologyWarningKind.EXPERIMENTAL_CGV, 0, 1, "x"
    )
    invalid_sequences: tuple[object, ...] = (
        [object()],
        [warning, object()],
        ["warning"],
    )

    for invalid in invalid_sequences:
        with pytest.raises(
            TypeError,
            match=r"^warnings\[\d+\] must be a MorphologyWarning$",
        ):
            morphology.MorphologyError(
                value,
                "x",
                None,
                cast(Sequence[morphology.MorphologyWarning], invalid),
            )

    with pytest.raises(TypeError, match="^trace must be a TraceReport or None$"):
        morphology.MorphologyError(
            value,
            "x",
            None,
            (warning,),
            cast(diagnostics.TraceReport, object()),
        )


def test_strict_exception_retains_typed_details_and_provenance() -> None:
    source_id = source.SourceId("failure")
    with pytest.raises(morphology.MorphologyError) as caught:
        morphology.segment("aa", source_id=source_id)
    error = caught.value
    assert isinstance(error.value, morphology.InvalidMorphology)
    assert error.value.kind is morphology.MorphologyErrorKind.VOWEL_HIATUS
    assert isinstance(error.value.detail, morphology.PhonotacticDetail)
    assert error.code == "morphology.vowel-hiatus"
    assert error.original_source == "aa"
    assert error.source_id == source_id
    assert error.diagnostic.code == error.code
    assert error.spans[0].source_id == source_id


def test_unterminated_zoi_exception_retains_exact_payload_and_diagnostic() -> None:
    source_id = source.SourceId("unterminated-zoi")
    source_text = "zoi gy foo bar"
    with pytest.raises(morphology.MorphologyError) as caught:
        morphology.segment(source_text, source_id=source_id)

    error = caught.value
    assert isinstance(error.value, morphology.UnterminatedZoiQuote)
    assert error.value.char_offset == 7
    assert error.value.delimiter == "gy"
    assert error.value.context is not None
    assert (
        error.value.context.kind
        is morphology.MorphologyContextKind.DELIMITED_NON_LOJBAN_QUOTE
    )
    assert (error.value.context.char_start, error.value.context.char_end) == (3, 7)
    assert error.code == "morphology.unterminated-zoi-quote"
    assert error.original_source == source_text
    assert error.source_id == source_id
    assert error.diagnostic.code == error.code
    assert error.diagnostic.labels[0].span.byte_range == (7, 14)
    assert error.diagnostic.labels[0].span.char_range == (7, 14)
    assert error.diagnostic.labels[1].span.byte_range == (3, 7)
    assert error.diagnostic.labels[1].span.char_range == (3, 7)
    assert all(span.source_id == source_id for span in error.spans)


def test_unicode_recovery_retains_byte_char_regions_and_source_identity() -> None:
    source_id = source.SourceId("unicode-recovery")
    attempt = morphology.segment_recovered_attempt(
        "ми @@@ do", source_id=source_id
    )
    assert attempt.source_id == source_id
    assert len(attempt.result.errors) == len(attempt.result.error_regions) == 1
    region = attempt.result.error_regions[0]
    assert region.byte_range == (5, 9)
    assert region.char_range == (3, 7)
    assert region.source_id == source_id
    first = attempt.result.words[0]
    assert isinstance(first, morphology.PlainWord)
    assert first.word.span.byte_range == (0, 4)
    assert first.word.span.char_range == (0, 2)
    second = attempt.result.words[1]
    assert isinstance(second, morphology.PlainWord)
    assert second.word.span.byte_range == (9, 11)
    assert second.word.span.char_range == (7, 9)
    assert all(
        span.source_id == source_id
        for value in attempt.result.words
        for span in value.source_spans
    )


def test_recovered_attempt_retains_parser_warnings_and_trace() -> None:
    source_id = source.SourceId("recovered-warning-trace")
    options = morphology.MorphologyOptions(
        trace=diagnostics.TraceOptions(enabled=True)
    )
    attempt = morphology.segment_recovered_attempt(
        "namzi @@@ kamzifre", options=options, source_id=source_id
    )

    assert attempt.source == "namzi @@@ kamzifre"
    assert attempt.source_id == source_id
    assert attempt.trace is not None
    assert attempt.trace.phase is diagnostics.TracePhase.MORPHOLOGY
    assert attempt.trace.events
    assert plain_phonemes(attempt.result.words) == ("námzi", "kamzífre")
    assert len(attempt.result.errors) == len(attempt.result.error_regions) == 1
    assert attempt.result.error_regions[0].byte_range == (6, 10)
    assert attempt.result.error_regions[0].char_range == (6, 10)
    assert tuple(
        (
            warning.kind,
            warning.char_start,
            warning.char_end,
            warning.text,
            None
            if warning.context is None
            else (
                warning.context.kind,
                warning.context.char_start,
                warning.context.char_end,
            ),
            warning.ignored_character_count,
        )
        for warning in attempt.result.warnings
    ) == (
        (
            morphology.MorphologyWarningKind.EXPERIMENTAL_MZ,
            2,
            4,
            "mz",
            (morphology.MorphologyContextKind.GISMU, 0, 5),
            None,
        ),
        (
            morphology.MorphologyWarningKind.EXPERIMENTAL_MZ,
            12,
            14,
            "mz",
            (morphology.MorphologyContextKind.LUJVO, 10, 18),
            None,
        ),
    )


def test_valsi_analysis_retains_parser_warnings() -> None:
    analysis = morphology.analyze_valsi(
        "namzi", source_id=source.SourceId("valsi-warning")
    )
    assert analysis.input == "namzi"
    assert analysis.result.status is morphology.ValsiAnalysisStatus.VALID
    assert len(analysis.warnings) == 1
    (warning,) = analysis.warnings
    assert warning.kind is morphology.MorphologyWarningKind.EXPERIMENTAL_MZ
    assert (warning.char_start, warning.char_end, warning.text) == (2, 4, "mz")
    assert warning.context is not None
    assert warning.context.kind is morphology.MorphologyContextKind.GISMU
    assert (warning.context.char_start, warning.context.char_end) == (0, 5)
    assert warning.ignored_character_count is None


def test_nested_quotes_and_valsi_analysis_retain_all_source_ids() -> None:
    source_id = source.SourceId("nested-provenance")
    (quote,) = morphology.segment(
        "zoi gy café gy", source_id=source_id
    )
    assert isinstance(quote, morphology.DelimitedNonLojbanQuote)
    assert quote.quoted_text.text == "café"
    assert quote.quoted_text.span.byte_range == (7, 12)
    assert quote.quoted_text.span.char_range == (7, 11)
    assert all(span.source_id == source_id for span in quote.source_spans)

    analysis = morphology.analyze_valsi("broda zei brode", source_id=source_id)
    assert analysis.result.word is not None
    assert isinstance(analysis.result.word, morphology.ZeiCompound)
    assert all(
        span.source_id == source_id for span in analysis.result.word.source_spans
    )
    assert analysis.result.word.left.source_spans[0].byte_range == (0, 5)
    assert analysis.result.word.zei.span.byte_range == (6, 9)
    assert analysis.result.word.right.span.byte_range == (10, 15)


def test_strict_failure_retains_trace_and_preceding_warnings() -> None:
    options = morphology.MorphologyOptions(
        trace=diagnostics.TraceOptions(enabled=True),
    )
    with pytest.raises(morphology.MorphologyError) as caught:
        morphology.segment("namzi aa", options=options)
    error = caught.value
    assert isinstance(error.value, morphology.InvalidMorphology)
    assert error.value.kind is morphology.MorphologyErrorKind.VOWEL_HIATUS
    assert error.trace is not None
    assert error.trace.phase is diagnostics.TracePhase.MORPHOLOGY
    assert any(
        event.kind is diagnostics.TraceEventKind.MORPHOLOGY_FAILURE
        for event in error.trace.events
    )
    assert tuple(warning.kind for warning in error.warnings) == (
        morphology.MorphologyWarningKind.EXPERIMENTAL_MZ,
    )


def test_analyze_valsi_status_and_variant_classification() -> None:
    valid = morphology.analyze_valsi("jetcybolxada")
    assert valid.result.status is morphology.ValsiAnalysisStatus.VALID
    assert isinstance(valid.result.classification, morphology.PlainWordValsiClassification)
    classification = valid.result.classification.word
    assert plain_classification_value(classification) == (
        "lujvo",
        "jetcybolxáda",
        None,
        "jetc-y-bolxáda",
        (
            ("rafsi", "jetc", "long"),
            ("hyphen", "y", None),
            ("rafsi", "bolxáda", "fuivla"),
        ),
        None,
    )

    invalid = morphology.analyze_valsi("aa")
    assert invalid.result.status is morphology.ValsiAnalysisStatus.INVALID
    assert isinstance(invalid.result.error, morphology.InvalidMorphology)

    multiple = morphology.analyze_valsi("coibroda")
    assert multiple.result.status is morphology.ValsiAnalysisStatus.NOT_SINGLE_WORD
    assert len(multiple.result.words) == 2


@pytest.mark.parametrize(
    ("text", "expected_stage"),
    (
        ("cidjrspageti", morphology.ValsiFuhivlaStage.STAGE3),
        ("spageti", morphology.ValsiFuhivlaStage.STAGE4),
    ),
)
def test_valsi_analysis_retains_exact_fuhivla_stage(
    text: str, expected_stage: morphology.ValsiFuhivlaStage
) -> None:
    analysis = morphology.analyze_valsi(text)
    assert analysis.result.status is morphology.ValsiAnalysisStatus.VALID
    classification = analysis.result.classification
    assert isinstance(classification, morphology.PlainWordValsiClassification)
    assert classification.word.category is morphology.WordKind.FUHIVLA
    assert classification.word.selmaho is None
    assert classification.word.split is None
    assert classification.word.parts == ()
    assert classification.word.stage is expected_stage


@pytest.mark.parametrize(
    ("text", "classification_type", "expected"),
    (
        (
            "mi",
            morphology.PlainWordValsiClassification,
            (
                "plain-word",
                expected_plain_classification("cmavo", "mi", "KOhA"),
            ),
        ),
        (
            "zo broda",
            morphology.QuotedWordValsiClassification,
            (
                "quoted-word",
                expected_plain_classification("cmavo", "zo", "ZO"),
                expected_plain_classification("gismu", "bróda"),
            ),
        ),
        (
            "zoi gy hello world gy",
            morphology.DelimitedNonLojbanQuoteValsiClassification,
            (
                "delimited-non-lojban-quote",
                expected_plain_classification("cmavo", "zoĭ", "ZOI"),
                "gy",
            ),
        ),
        (
            "lo'u mi do le'u",
            morphology.QuotedWordsValsiClassification,
            (
                "quoted-words",
                expected_plain_classification("cmavo", "lo'u", "LOhU"),
                (
                    expected_plain_classification("cmavo", "mi", "KOhA"),
                    expected_plain_classification("cmavo", "do", "KOhA"),
                ),
            ),
        ),
        (
            "zo'oi hello",
            morphology.DelimitedWordQuoteValsiClassification,
            ("delimited-word-quote", "zo'oĭ"),
        ),
        (
            "a bu",
            morphology.LerfuWordValsiClassification,
            (
                "lerfu-word",
                (
                    "plain-word",
                    expected_plain_classification("cmavo", "a", "A"),
                ),
                expected_plain_classification("cmavo", "bu", "BU"),
            ),
        ),
        (
            "broda zei brode",
            morphology.ZeiCompoundValsiClassification,
            (
                "zei-compound",
                (
                    "plain-word",
                    expected_plain_classification("gismu", "bróda"),
                ),
                expected_plain_classification("cmavo", "zeĭ", "ZEI"),
                expected_plain_classification("gismu", "bróde"),
            ),
        ),
    ),
)
def test_every_valsi_classification_payload_variant(
    text: str, classification_type: type[object], expected: tuple[object, ...]
) -> None:
    analysis = morphology.analyze_valsi(text)
    assert analysis.result.status is morphology.ValsiAnalysisStatus.VALID
    assert isinstance(analysis.result.classification, classification_type)
    assert analysis.result.classification is not None
    assert classification_value(analysis.result.classification) == expected


def test_classification_children_keep_their_arc_root_alive() -> None:
    analysis = morphology.analyze_valsi("broda zei brode")
    classification = analysis.result.classification
    assert isinstance(classification, morphology.ZeiCompoundValsiClassification)
    left = classification.left
    link = classification.link
    del classification
    del analysis
    gc.collect()
    assert left.kind is morphology.ValsiClassificationKind.PLAIN_WORD
    assert link.category is morphology.WordKind.CMAVO


def test_python_created_words_share_the_parser_projection_and_syntax_identity() -> None:
    span = source.SourceSpan(0, 2, 0, 2)
    constructed_word = morphology.CmavoWord(morphology.Phonemes("mi"), span)
    constructed = morphology.PlainWord(constructed_word)
    (parsed,) = morphology.segment("mi")
    assert isinstance(parsed, morphology.PlainWord)
    assert morphology.word_syntax_eq(constructed_word, parsed.word)
    assert morphology.word_like_syntax_eq(constructed, parsed)


def test_all_error_detail_and_error_value_variants_are_typed() -> None:
    context = morphology.MorphologyContext(
        morphology.MorphologyContextKind.LUJVO, 0, 2
    )
    details: tuple[morphology.MorphologyErrorDetail, ...] = (
        morphology.InvalidLujvoDetail(
            morphology.LujvoParseExpectation.FINAL_OR_INITIAL_RAFSI, "jbo"
        ),
        morphology.FuhivlaContainsYDetail(),
        morphology.SlinkuhiDetail(),
        morphology.ExpectedWordDetail(
            morphology.ExpectedWordDetailKind.ZEI_OPERAND
        ),
        morphology.InvalidZoiDelimiterDetail(
            morphology.ZoiDelimiterDetailKind.NOT_SINGLE_WORD
        ),
        morphology.PhonotacticDetail(
            morphology.PhonotacticDetailKind.FORBIDDEN_CONSONANT_PAIR
        ),
    )
    assert len({type(detail) for detail in details}) == 6

    errors: tuple[morphology.MorphologyErrorValue, ...] = (
        morphology.InvalidMorphology(
            morphology.MorphologyErrorKind.INVALID_LUJVO,
            0,
            2,
            "xx",
            context=context,
            detail=details[0],
        ),
        morphology.UnterminatedZoiQuote(2, "gy", context=context),
        morphology.SourceSpanMorphologyError(source.ZeroLine()),
    )
    assert tuple(error.code for error in errors) == (
        "morphology.invalid-lujvo",
        "morphology.unterminated-zoi-quote",
        "morphology.source-span",
    )


def test_nondefault_dialect_is_applied_by_real_parser() -> None:
    definition = dialect.DialectDefinition(
        features=(dialect.DialectFeature.CASE_INSENSITIVE,)
    )
    options = morphology.MorphologyOptions().with_dialect(definition)
    (value,) = morphology.segment("NALSELTRO", options=options)
    assert isinstance(value, morphology.PlainWord)
    assert value.word.phonemes.text == "nalséltro"


def test_fieldless_payload_variants_support_positional_pattern_matching() -> None:
    source_error: source.SourceLocationError = source.ZeroLine()
    detail: morphology.MorphologyErrorDetail = morphology.FuhivlaContainsYDetail()

    match source_error:
        case source.ZeroLine():
            pass
        case _:
            pytest.fail("ZeroLine did not match its exact variant class")

    match detail:
        case morphology.FuhivlaContainsYDetail():
            pass
        case _:
            pytest.fail("FuhivlaContainsYDetail did not match its exact variant class")


def test_cmavo_selmaho_round_trips_are_exact() -> None:
    for cmavo in morphology.Cmavo:
        assert morphology.cmavo_from_text(cmavo.value) is cmavo
        assert morphology.cmavo_text(cmavo) == cmavo.value
    for selmaho in morphology.Selmaho:
        assert morphology.selmaho_from_name(selmaho.value) is selmaho
        assert morphology.selmaho_name(selmaho) == selmaho.value
    assert morphology.cmavo_is_selmaho(morphology.Cmavo.ZO, morphology.Selmaho.ZO)
    with pytest.raises(InvalidInputError):
        morphology.selmaho_from_name("")
    with pytest.raises(TypeError):
        morphology.cmavo_text("zo")  # type: ignore[arg-type]


def test_lujvo_parts_and_domain_helpers_are_typed() -> None:
    parts = morphology.parse_lujvo_parts("jetcybolxada")
    assert parts is not None
    assert any(isinstance(part, morphology.LujvoHyphen) for part in parts)
    assert morphology.rafsi_shape("jbo") is morphology.RafsiShape.CCV
    assert morphology.rafsi_shape_score(morphology.RafsiShape.CCV) == 7
    assert morphology.consonant_pair_class("b", "l") is not None
    assert morphology.permissible_consonant_pair("b", "l")
    assert morphology.canonical_text_eq("COI", "coi")
    assert morphology.strip_lojban_diacritic("á") == "a"
    with pytest.raises(InvalidInputError):
        morphology.is_vowel("aa")


def test_empty_textual_lujvo_parts_return_none_without_panicking() -> None:
    assert morphology.bond_rafsis(["", "jbo"]) is None
    assert (
        morphology.choose_best_lujvo_candidate(
            morphology.LujvoBuildMode.LUJVO,
            [["", "jbo"]],
        )
        is None
    )

    with pytest.raises(InvalidInputError):
        morphology.LujvoRafsiBuildPart("")
    with pytest.raises(InvalidInputError):
        morphology.LujvoBrivlaCoreBuildPart("")


def test_copied_morphology_inputs_accept_lists_and_tuples_but_return_tuples() -> None:
    parsed_lujvo = morphology.segment("jetcybolxada")[0]
    assert isinstance(parsed_lujvo, morphology.PlainWord)
    assert isinstance(parsed_lujvo.word, morphology.LujvoWord)
    lujvo_from_list = morphology.LujvoWord(
        list(parsed_lujvo.word.parts), parsed_lujvo.word.span
    )
    lujvo_from_tuple = morphology.LujvoWord(
        tuple(parsed_lujvo.word.parts), parsed_lujvo.word.span
    )
    assert lujvo_from_list == lujvo_from_tuple
    assert isinstance(lujvo_from_list.parts, tuple)

    parsed_quote = morphology.segment("lo'u mi do le'u")[0]
    assert isinstance(parsed_quote, morphology.QuotedWords)
    quote_from_list = morphology.QuotedWords(
        parsed_quote.lohu, list(parsed_quote.quoted_words), parsed_quote.lehu
    )
    quote_from_tuple = morphology.QuotedWords(
        parsed_quote.lohu, tuple(parsed_quote.quoted_words), parsed_quote.lehu
    )
    assert quote_from_list == quote_from_tuple
    assert isinstance(quote_from_list.quoted_words, tuple)

    choices_list = [
        [morphology.LujvoRafsiBuildPart("jbo")],
        [morphology.LujvoBrivlaCoreBuildPart("klama")],
    ]
    choices_tuple = tuple(tuple(choice) for choice in choices_list)
    list_candidate = morphology.choose_best_lujvo_candidate_from_parts(
        morphology.LujvoBuildMode.LUJVO, choices_list
    )
    tuple_candidate = morphology.choose_best_lujvo_candidate_from_parts(
        morphology.LujvoBuildMode.LUJVO, choices_tuple
    )
    assert list_candidate == tuple_candidate
    if list_candidate is not None:
        assert isinstance(list_candidate.parts, tuple)

    for invalid_call in (
        lambda: morphology.LujvoWord(
            set(),  # type: ignore[arg-type]
            parsed_lujvo.word.span,
        ),
        lambda: morphology.LujvoWord(
            [object()],  # type: ignore[list-item]
            parsed_lujvo.word.span,
        ),
        lambda: morphology.QuotedWords(
            parsed_quote.lohu,
            set(),  # type: ignore[arg-type]
            parsed_quote.lehu,
        ),
        lambda: morphology.QuotedWords(
            parsed_quote.lohu,
            [object()],  # type: ignore[list-item]
            parsed_quote.lehu,
        ),
        lambda: morphology.choose_best_lujvo_candidate_from_parts(
            morphology.LujvoBuildMode.LUJVO,
            set(),  # type: ignore[arg-type]
        ),
        lambda: morphology.choose_best_lujvo_candidate_from_parts(
            morphology.LujvoBuildMode.LUJVO,
            [[object()]],  # type: ignore[list-item]
        ),
    ):
        with pytest.raises(TypeError):
            invalid_call()
