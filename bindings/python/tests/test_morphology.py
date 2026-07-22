from __future__ import annotations

import gc

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
