from __future__ import annotations

import gc
from collections.abc import Sequence
from typing import cast

import pytest

from jbotci import InvalidInputError, diagnostics, dialect, morphology, source


def plain_phonemes(words: tuple[morphology.WordLike, ...]) -> tuple[str, ...]:
    values: list[str] = []
    for value in words:
        assert isinstance(value, morphology.PlainWord)
        values.append(value.word.phonemes.text)
    return tuple(values)


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
    assert isinstance(morphology.segment("ma'oi ba")[0], morphology.SelmahoQuotedWord)
    zoi = morphology.segment("zoi gy hello world gy")[0]
    assert isinstance(zoi, morphology.DelimitedNonLojbanQuote)
    assert zoi.quoted_text.text == "hello world"
    assert isinstance(morphology.segment("lo'u mi do le'u")[0], morphology.QuotedWords)
    assert isinstance(morphology.segment("zo'oi hello")[0], morphology.DelimitedWordQuote)
    assert isinstance(morphology.segment("a bu")[0], morphology.LerfuWord)
    assert isinstance(morphology.segment("broda zei brode")[0], morphology.ZeiCompound)


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


def test_analyze_valsi_status_and_variant_classification() -> None:
    valid = morphology.analyze_valsi("jetcybolxada")
    assert valid.result.status is morphology.ValsiAnalysisStatus.VALID
    assert isinstance(valid.result.classification, morphology.PlainWordValsiClassification)
    classification = valid.result.classification.word
    assert classification.category is morphology.WordKind.LUJVO
    assert classification.parts

    invalid = morphology.analyze_valsi("aa")
    assert invalid.result.status is morphology.ValsiAnalysisStatus.INVALID
    assert isinstance(invalid.result.error, morphology.InvalidMorphology)

    multiple = morphology.analyze_valsi("coibroda")
    assert multiple.result.status is morphology.ValsiAnalysisStatus.NOT_SINGLE_WORD
    assert len(multiple.result.words) == 2


@pytest.mark.parametrize(
    ("text", "classification_type"),
    (
        ("mi", morphology.PlainWordValsiClassification),
        ("zo broda", morphology.QuotedWordValsiClassification),
        (
            "zoi gy hello world gy",
            morphology.DelimitedNonLojbanQuoteValsiClassification,
        ),
        ("lo'u mi do le'u", morphology.QuotedWordsValsiClassification),
        ("zo'oi hello", morphology.DelimitedWordQuoteValsiClassification),
        ("a bu", morphology.LerfuWordValsiClassification),
        ("broda zei brode", morphology.ZeiCompoundValsiClassification),
    ),
)
def test_every_valsi_classification_payload_variant(
    text: str, classification_type: type[object]
) -> None:
    analysis = morphology.analyze_valsi(text)
    assert analysis.result.status is morphology.ValsiAnalysisStatus.VALID
    assert isinstance(analysis.result.classification, classification_type)


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
