"""Strict-type-check smoke coverage for packaged public declarations."""

from typing import Never, assert_type

from jbotci import (
    Sample,
    SampleMode,
    dictionary,
    diagnostics,
    dialect,
    morphology,
    sample_mode,
    semantics,
    smoke,
    source,
)


def sample_text(value: str | None) -> tuple[str, str | None]:
    """Exercise class, tuple, optional, and namespace annotations."""
    sample = Sample(value or "")
    mode: SampleMode = sample_mode(advanced=value is not None)
    assert semantics.references.__all__ == ()
    return (f"{smoke()}:{mode.value}", sample.value if value is not None else None)


def dictionary_values(word: str) -> tuple[str | None, int | None, str | None]:
    """Exercise typed dictionary records, enums, tuples, and optional results."""
    entry = dictionary.english.lookup_word(word)
    if entry is None:
        return (None, None, None)
    index = dictionary.english.entry_index_for_entry(entry)
    first_rafsi = entry.rafsi[0].value if entry.rafsi else None
    assert isinstance(entry.word_type, dictionary.WordType)
    _ = entry.word_type.is_gismu_like() or entry.word_type.is_lujvo_like()
    return (entry.word, index.value if index is not None else None, first_rafsi)


def dictionary_batch(words: tuple[str, ...]) -> tuple[str | None, ...]:
    """Exercise the strict batch and bounded-result declarations."""
    return dictionary.english.first_gloss_keywords_for_words(words)


def typed_source_and_diagnostics(text: str) -> diagnostics.Diagnostic:
    """Exercise source and immutable diagnostic result declarations."""

    source_id = source.SourceId("typecheck")
    location = source.LineColumn(1, 1)
    location_errors: tuple[source.SourceLocationError, ...] = (
        source.ZeroLine(),
        source.ZeroColumn(),
        source.ByteRangeInverted(2, 1),
        source.CharRangeInverted(2, 1),
    )
    span: source.SourceSpan = source.source_span_from_char_offsets(
        text, 0, len(text), source_id=source_id
    )
    enriched_span = source.SourceSpan(
        span.byte_start,
        span.byte_end,
        span.char_start,
        span.char_end,
        source_id=source_id,
        start=location,
    )
    label = diagnostics.DiagnosticLabel(span, "input", primary=True)
    diagnostic = diagnostics.Diagnostic(
        diagnostics.DiagnosticSeverity.ERROR,
        diagnostics.DiagnosticPhase.MORPHOLOGY,
        "typecheck.error",
        "typed diagnostic",
        [label],
    )
    labels: tuple[diagnostics.DiagnosticLabel, ...] = diagnostic.labels
    assert labels[0].span.source_id == source_id and enriched_span.start == location
    assert all(isinstance(error, object) for error in location_errors)
    return diagnostic


def typed_dialect() -> dialect.DialectDefinition:
    """Exercise payload-union and fieldless-enum dialect declarations."""

    entry: dialect.CmavoDialectEntry = dialect.CmavoSwap("ce'u", "ce")
    return dialect.DialectDefinition(
        (entry,), (dialect.DialectFeature.CASE_INSENSITIVE,)
    )


def returned_only_dialect_type() -> None:
    """Builtin dialects cannot be constructed by typed consumers."""

    assert_type(dialect.BuiltinDialect(), Never)


def typed_morphology(text: str) -> tuple[morphology.WordLike, ...]:
    """Exercise every recursive result union without falling back to Any."""

    options = morphology.MorphologyOptions(dialect=typed_dialect())
    attempt: morphology.MorphologySegmentAttempt = morphology.segment_attempt(
        text, options=options
    )
    warnings: tuple[morphology.MorphologyWarning, ...] = attempt.warnings
    if attempt.error is not None:
        error: morphology.MorphologyErrorValue = attempt.error
        code: str = error.code
        raise morphology.MorphologyError(
            error, attempt.source, attempt.source_id, warnings, attempt.trace
        )
    assert attempt.words is not None
    for value in attempt.words:
        match value:
            case morphology.PlainWord(word):
                kind: morphology.WordKind = word.kind
                assert kind.value
            case morphology.QuotedWord(marker, word):
                assert marker.span.char_end <= word.span.char_end
            case morphology.SelmahoQuotedWord(marker, word):
                assert marker.key != word.key
            case morphology.DelimitedNonLojbanQuote(_, opening, quoted, closing):
                assert opening.span.byte_end <= quoted.span.byte_start
                assert quoted.span.byte_end <= closing.span.byte_start
            case morphology.QuotedWords(_, quoted_words, _):
                words: tuple[morphology.Word, ...] = quoted_words
                assert isinstance(words, tuple)
            case morphology.DelimitedWordQuote(_, quoted):
                assert isinstance(quoted.text, str)
            case morphology.LerfuWord(base, suffix):
                assert base.source_spans and suffix.kind is morphology.WordKind.CMAVO
            case morphology.ZeiCompound(left, link, right):
                assert left.source_spans and link.key != right.key
    return attempt.words


def typed_valsi(text: str) -> morphology.ValsiAnalysisResult:
    """Exercise classification payload variants and lujvo part declarations."""

    result = morphology.analyze_valsi(text).result
    classification = result.classification
    if isinstance(classification, morphology.PlainWordValsiClassification):
        parts: tuple[morphology.ValsiLujvoPart, ...] = classification.word.parts
        for part in parts:
            kind: morphology.ValsiLujvoPartKind = part.kind
            assert kind.value
    elif isinstance(classification, morphology.LerfuWordValsiClassification):
        base: morphology.ValsiClassification = classification.base
        assert base.kind.value
    elif isinstance(classification, morphology.ZeiCompoundValsiClassification):
        left: morphology.ValsiClassification = classification.left
        assert left.kind.value
    return result


def typed_diagnostic_products(span: source.SourceSpan) -> diagnostics.TraceReport:
    """Exercise every immutable diagnostics payload and trace result class."""

    context = diagnostics.TraceContext("word", span.byte_start, span.byte_end)
    branch = diagnostics.TraceFailureBranch([context], ["cmavo"])
    failure = diagnostics.TraceFailureSummary(
        span.byte_start,
        span.byte_end,
        "expected a word",
        [branch],
        context,
    )
    event = diagnostics.TraceEvent(
        diagnostics.TracePhase.MORPHOLOGY,
        diagnostics.TraceLevel.DETAILED,
        0,
        diagnostics.TraceEventKind.MORPHOLOGY_STEP,
        "word",
        span.byte_start,
        span.byte_end,
        "accepted",
    )
    report = diagnostics.TraceReport(
        diagnostics.TracePhase.MORPHOLOGY,
        [event],
        failure=failure,
    )
    links: tuple[diagnostics.DiagnosticTextLink, ...] = (
        diagnostics.VlackuWordLink("klama"),
        diagnostics.CllSectionLink("chapter5", "section1"),
        diagnostics.EbnfRuleLink("sumti"),
    )
    segments = tuple(
        diagnostics.DiagnosticTextSegment(
            diagnostics.DiagnosticTextRole.PLAIN, "detail", link=link
        )
        for link in links
    )
    styled = diagnostics.DiagnosticStyledNote(
        diagnostics.DiagnosticNoteMode.DETAILED, list(segments)
    )
    label = diagnostics.DiagnosticLabel(span, "input", primary=True)
    diagnostic = diagnostics.Diagnostic(
        diagnostics.DiagnosticSeverity.WARNING,
        diagnostics.DiagnosticPhase.MORPHOLOGY,
        "typed.warning",
        "warning",
        [label],
        ["note"],
        styled_notes=[styled],
        word_index=0,
    )
    linked_text: str = diagnostics.diagnostic_text_segments_text(
        list(diagnostic.message_segments)
    )
    options = diagnostics.TraceOptions(
        enabled=True,
        level=diagnostics.TraceLevel.ALL,
        filter=diagnostics.TraceFilter("word"),
        phase=diagnostics.TracePhase.MORPHOLOGY,
        limit=100,
    ).with_phase(diagnostics.TracePhase.MORPHOLOGY).with_limit(50)
    level_number: int = diagnostics.TraceLevel.DETAILED.number()
    level: diagnostics.TraceLevel = diagnostics.TraceLevel.from_number(level_number)
    phase_included: bool = diagnostics.TracePhase.ALL.includes(
        diagnostics.TracePhase.SYNTAX
    )
    note_visible: bool = diagnostics.DiagnosticNoteMode.ALWAYS.visible_in(
        diagnostics.DiagnosticDetailMode.DETAILED
    )
    assert (
        options.enabled
        and options.includes(diagnostics.TracePhase.MORPHOLOGY)
        and linked_text == diagnostic.message
        and level is diagnostics.TraceLevel.DETAILED
        and phase_included
        and note_visible
    )
    return report


def typed_dialect_products() -> dialect.DialectSettings:
    """Exercise both dialect-entry variants and all declarative result records."""

    swap = dialect.CmavoSwap("ce'u", "ce")
    expansion = dialect.CmavoExpansion("coi", ["coi", "ro", "do"])
    entries: tuple[dialect.CmavoDialectEntry, ...] = (swap, expansion)
    definition = dialect.DialectDefinition(
        entries, (dialect.DialectFeature.CASE_INSENSITIVE,)
    )
    feature_atom: str = dialect.DialectFeature.CASE_INSENSITIVE.atom_name()
    custom = dialect.CustomDialect("typed", dialect.dialect_definition_to_text(definition))
    settings = dialect.DialectSettings([custom], ["zantufa"])
    builtins: tuple[dialect.BuiltinDialect, ...] = dialect.builtin_dialects()
    if builtins:
        parsed: dialect.DialectDefinition = builtins[0].dialect
        assert isinstance(parsed.cmavo_entries, tuple)
    assert feature_atom == "CASE-INSENSITIVE"
    return settings


def typed_constructed_morphology_products() -> morphology.ValsiAnalysisResult:
    """Construct every exposed morphology payload variant with closed unions."""

    span = source.SourceSpan(0, 2, 0, 2)
    longer_span = source.SourceSpan(0, 5, 0, 5)
    phonemes = morphology.Phonemes("mi")
    rendered: str = phonemes.render(
        morphology.PhonemeRenderOptions(
            mark_stress=morphology.StressMark.ACUTE,
            mark_glides=morphology.GlideMark.BREVE,
        )
    )
    cmavo = morphology.CmavoWord(phonemes, span)
    gismu = morphology.GismuWord(morphology.Phonemes("klama"), longer_span)
    fuhivla = morphology.FuhivlaWord(morphology.Phonemes("spageti"), longer_span)
    cmevla = morphology.CmevlaWord(morphology.Phonemes("alis"), longer_span)
    rafsi = morphology.LujvoRafsi(morphology.Phonemes("jbo"))
    hyphen = morphology.LujvoHyphen(morphology.Phonemes("y"))
    parts: tuple[morphology.LujvoPart, ...] = (rafsi, hyphen, rafsi)
    lujvo = morphology.LujvoWord(parts, longer_span)
    words: tuple[morphology.Word, ...] = (cmavo, gismu, lujvo, fuhivla, cmevla)
    key: morphology.WordKey = cmavo.key
    assert key.phonemes.text and rendered

    plain = morphology.PlainWord(cmavo)
    quoted = morphology.QuotedWord(cmavo, gismu)
    selmaho_quoted = morphology.SelmahoQuotedWord(cmavo, gismu)
    verbatim = morphology.Verbatim(span, "mi")
    zoi = morphology.DelimitedNonLojbanQuote(cmavo, cmavo, verbatim, cmavo)
    quoted_words = morphology.QuotedWords(cmavo, words, cmavo)
    delimited_word = morphology.DelimitedWordQuote(cmavo, verbatim)
    lerfu = morphology.LerfuWord(plain, cmavo)
    zei = morphology.ZeiCompound(plain, cmavo, gismu)
    word_likes: tuple[morphology.WordLike, ...] = (
        plain,
        quoted,
        selmaho_quoted,
        zoi,
        quoted_words,
        delimited_word,
        lerfu,
        zei,
    )

    context = morphology.MorphologyContext(
        morphology.MorphologyContextKind.CMAVO, 0, 2
    )
    details: tuple[morphology.MorphologyErrorDetail, ...] = (
        morphology.InvalidLujvoDetail(
            morphology.LujvoParseExpectation.FINAL_OR_INITIAL_RAFSI, "jbo"
        ),
        morphology.FuhivlaContainsYDetail(),
        morphology.SlinkuhiDetail(),
        morphology.ExpectedWordDetail(
            morphology.ExpectedWordDetailKind.PLAIN_WORD
        ),
        morphology.InvalidZoiDelimiterDetail(
            morphology.ZoiDelimiterDetailKind.NOT_SINGLE_WORD
        ),
        morphology.PhonotacticDetail(
            morphology.PhonotacticDetailKind.VOWEL_HIATUS
        ),
    )
    warning = morphology.MorphologyWarning(
        morphology.MorphologyWarningKind.IGNORED_CHARACTERS,
        0,
        1,
        "@",
        context=context,
        ignored_character_count=1,
    )
    invalid = morphology.InvalidMorphology(
        morphology.MorphologyErrorKind.VOWEL_HIATUS,
        0,
        2,
        "aa",
        context=context,
        detail=details[-1],
    )
    unterminated = morphology.UnterminatedZoiQuote(2, "gy", context=context)
    span_error = morphology.SourceSpanMorphologyError(source.ZeroLine())
    errors: tuple[morphology.MorphologyErrorValue, ...] = (
        invalid,
        unterminated,
        span_error,
    )
    assert warning.to_diagnostic("@").code and errors[0].code

    lujvo_part = morphology.ValsiLujvoPart(
        morphology.ValsiLujvoPartKind.RAFSI,
        "jbo",
        rafsi_kind=morphology.ValsiLujvoRafsiKind.CCV,
    )
    plain_classification = morphology.PlainWordClassification(
        morphology.WordKind.LUJVO,
        "jbolu",
        split="jbo+lu",
        parts=[lujvo_part],
    )
    cmavo_classification = morphology.PlainWordClassification(
        morphology.WordKind.CMAVO, "mi", selmaho="KOhA"
    )
    classifications: tuple[morphology.ValsiClassification, ...] = (
        morphology.PlainWordValsiClassification(plain_classification),
        morphology.QuotedWordValsiClassification(
            cmavo_classification, plain_classification
        ),
        morphology.DelimitedNonLojbanQuoteValsiClassification(
            cmavo_classification, "gy"
        ),
        morphology.QuotedWordsValsiClassification(
            cmavo_classification, [plain_classification]
        ),
        morphology.DelimitedWordQuoteValsiClassification("zo'oi"),
        morphology.LerfuWordValsiClassification(
            morphology.PlainWordValsiClassification(cmavo_classification),
            cmavo_classification,
        ),
        morphology.ZeiCompoundValsiClassification(
            morphology.PlainWordValsiClassification(plain_classification),
            cmavo_classification,
            plain_classification,
        ),
    )
    result = morphology.ValsiAnalysisResult(
        morphology.ValsiAnalysisStatus.VALID,
        word=plain,
        classification=classifications[0],
    )
    invalid_result = morphology.ValsiAnalysisResult(
        morphology.ValsiAnalysisStatus.INVALID, error=invalid
    )
    not_single = morphology.ValsiAnalysisResult(
        morphology.ValsiAnalysisStatus.NOT_SINGLE_WORD, words=list(word_likes)
    )
    assert invalid_result.error is invalid and not_single.words == word_likes

    options = morphology.MorphologyOptions(
        accept_latin=True,
        accept_cyrillic=True,
        accept_zbalermorna=True,
        dialect=typed_dialect(),
        cmevla_as_relation_words=True,
        permissive_lexer=True,
        uppercase_marks_stress=True,
        max_recovery_errors=3,
        trace=diagnostics.TraceOptions(enabled=True),
    )
    compiled = morphology.CompiledDialectDefinition(typed_dialect())
    options = (
        options.with_dialect(typed_dialect())
        .with_compiled_dialect(compiled)
        .with_max_recovery_errors(2)
    )
    compiled_entries: tuple[morphology.CompiledDialectEntry, ...] = compiled.entries
    for entry in compiled_entries:
        if isinstance(entry, morphology.CompiledDialectSwap):
            compiled_word: morphology.CompiledDialectWord = entry.left
        else:
            compiled_word = entry.source
        assert compiled_word.key.phonemes.text

    build_parts: tuple[morphology.LujvoBuildPart, ...] = (
        morphology.LujvoRafsiBuildPart("jbo"),
        morphology.LujvoBrivlaCoreBuildPart("klama"),
    )
    candidate: morphology.LujvoCandidate | None = (
        morphology.choose_best_lujvo_candidate_from_parts(
            morphology.LujvoBuildMode.LUJVO, (build_parts,)
        )
    )
    assert candidate is None or candidate.parts
    assert options.trace.enabled
    recovered_attempt: morphology.RecoveredMorphologySegmentAttempt = (
        morphology.segment_recovered_attempt("mi", options=options)
    )
    recovered: morphology.RecoveredMorphologySegmentation = recovered_attempt.result
    analysis: morphology.ValsiAnalysis = morphology.analyze_valsi("mi", options=options)
    assert recovered.words and analysis.result.status.value
    return result
