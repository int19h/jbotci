"""Strict-type-check smoke coverage for packaged public declarations."""

from typing import assert_type

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
        [entry], [dialect.DialectFeature.CASE_INSENSITIVE]
    )


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


def typed_every_morphology_free_function(
    options: morphology.MorphologyOptions,
    source_id: source.SourceId,
    phonemes: morphology.Phonemes,
    word: morphology.Word,
    word_like: morphology.WordLike,
    cmavo: morphology.Cmavo,
    selmaho: morphology.Selmaho,
    pair_class: morphology.ConsonantPairClass,
    rafsi_shape: morphology.RafsiShape,
    build_part: morphology.LujvoBuildPart,
) -> None:
    """Independently pin every public morphology function's input/output types."""

    assert_type(
        morphology.segment("mi", options=options, source_id=source_id),
        tuple[morphology.WordLike, ...],
    )
    assert_type(
        morphology.segment_attempt("mi", options=options, source_id=source_id),
        morphology.MorphologySegmentAttempt,
    )
    assert_type(
        morphology.segment_recovered("mi", options=options, source_id=source_id),
        morphology.RecoveredMorphologySegmentation,
    )
    assert_type(
        morphology.segment_recovered_attempt(
            "mi", options=options, source_id=source_id
        ),
        morphology.RecoveredMorphologySegmentAttempt,
    )
    assert_type(
        morphology.segment_for_display(
            "mi", options=options, source_id=source_id
        ),
        tuple[morphology.WordLike, ...],
    )
    assert_type(
        morphology.segment_for_display_attempt(
            "mi", options=options, source_id=source_id
        ),
        morphology.MorphologySegmentAttempt,
    )
    assert_type(
        morphology.analyze_valsi("mi", options=options, source_id=source_id),
        morphology.ValsiAnalysis,
    )
    assert_type(morphology.normalize_input("mi", options=options), str | None)
    assert_type(morphology.canonicalize_text("MI"), str)
    assert_type(morphology.canonical_text_eq("MI", "mi"), bool)
    assert_type(morphology.canonical_text_is_all("mi", "m"), bool)
    assert_type(morphology.normalize_cmavo_form("coi"), str | None)
    assert_type(morphology.cmavo_phonemes("coi"), morphology.Phonemes | None)
    assert_type(morphology.pronunciation_syllables(phonemes), tuple[str, ...])
    assert_type(morphology.strip_lojban_diacritic("á"), str | None)
    assert_type(morphology.fold_lojban_diacritic("á"), str | None)
    assert_type(morphology.strip_lojban_diacritics("á"), str)
    assert_type(morphology.fold_lojban_diacritics("á"), str)
    assert_type(morphology.stripped_lojban_diacritics_eq("á", "a"), bool)
    assert_type(morphology.folded_lojban_diacritics_eq("á", "a"), bool)
    assert_type(morphology.strip_diacritics("á"), str)
    assert_type(morphology.strip_diacritics_eq("á", "a"), bool)
    assert_type(morphology.is_valid_phoneme("a"), bool)
    assert_type(
        morphology.is_word_forming_character("a", options=options), bool
    )
    assert_type(morphology.is_period_character("."), bool)
    assert_type(morphology.is_permissive_ignorable_character("@"), bool)
    assert_type(
        morphology.parse_lujvo_parts("jbogri"),
        tuple[morphology.LujvoPart, ...] | None,
    )
    assert_type(
        morphology.parse_cmevla_lujvo_parts("jbogris"),
        tuple[morphology.LujvoPart, ...] | None,
    )
    assert_type(
        morphology.parse_cmevla_lujvo_part_candidates("jbogris"),
        tuple[tuple[morphology.LujvoPart, ...], ...],
    )
    assert_type(morphology.bond_rafsis(["jbo", "gri"]), tuple[str, ...] | None)
    assert_type(morphology.is_valid_lujvo_candidate_word("jbogri"), bool)
    assert_type(morphology.ensure_cmevla_word("alis"), str)
    assert_type(morphology.ends_with_consonant("alis"), bool)
    assert_type(morphology.ends_with_vowel("klama"), bool)
    assert_type(morphology.is_bonding_hyphen("y"), bool)
    assert_type(morphology.syllables_pattern("klama"), str | None)
    assert_type(morphology.rafsi_shape("jbo"), morphology.RafsiShape)
    assert_type(morphology.rafsi_shape_score(rafsi_shape), int)
    assert_type(morphology.is_vowel("a"), bool)
    assert_type(morphology.is_consonant("b"), bool)
    assert_type(morphology.is_cmevla("alis"), bool)
    assert_type(
        morphology.consonant_pair_class("b", "l"),
        morphology.ConsonantPairClass | None,
    )
    assert_type(morphology.permissible_consonant_pair("b", "l"), bool)
    assert_type(morphology.consonant_pair_is_permissible(pair_class), bool)
    assert_type(morphology.consonant_pair_is_initial(pair_class), bool)
    assert_type(
        morphology.word_needs_leading_pause(
            word, morphology.LeadingPauseVowelMode.FOLDED_VOWELS
        ),
        bool,
    )
    assert_type(
        morphology.word_needs_leading_pause_in_context(
            word,
            morphology.LeadingPauseVowelMode.FOLDED_VOWELS,
            morphology.LeadingPauseContext.INDEPENDENT_WORD,
        ),
        bool,
    )
    assert_type(morphology.word_syntax_eq(word, word), bool)
    assert_type(morphology.word_like_syntax_eq(word_like, word_like), bool)
    assert_type(morphology.cmavo_from_text("zo"), morphology.Cmavo | None)
    assert_type(morphology.cmavo_text(cmavo), str)
    assert_type(morphology.cmavo_is_selmaho(cmavo, selmaho), bool)
    assert_type(
        morphology.cmavo_primary_selmaho(cmavo), morphology.Selmaho | None
    )
    assert_type(morphology.cmavo_is_quote_opener(cmavo), bool)
    assert_type(morphology.cmavo_is_single_word_quote_opener(cmavo), bool)
    assert_type(
        morphology.cmavo_is_delimited_non_lojban_quote_opener(cmavo), bool
    )
    assert_type(morphology.selmaho_from_name("ZO"), morphology.Selmaho | None)
    assert_type(morphology.selmaho_name(selmaho), str)
    assert_type(morphology.selmaho_contains(selmaho, cmavo), bool)
    assert_type(
        morphology.choose_best_lujvo_candidate(
            morphology.LujvoBuildMode.LUJVO, [["jbo"], ["gri"]]
        ),
        morphology.LujvoCandidate | None,
    )
    assert_type(
        morphology.choose_best_lujvo_candidate_from_parts(
            morphology.LujvoBuildMode.LUJVO, [[build_part]]
        ),
        morphology.LujvoCandidate | None,
    )


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
    lujvo = morphology.LujvoWord(list(parts), longer_span)
    words: tuple[morphology.Word, ...] = (cmavo, gismu, lujvo, fuhivla, cmevla)
    key: morphology.WordKey = cmavo.key
    assert key.phonemes.text and rendered

    plain = morphology.PlainWord(cmavo)
    quoted = morphology.QuotedWord(cmavo, gismu)
    selmaho_quoted = morphology.SelmahoQuotedWord(cmavo, gismu)
    verbatim = morphology.Verbatim(span, "mi")
    zoi = morphology.DelimitedNonLojbanQuote(cmavo, cmavo, verbatim, cmavo)
    quoted_words = morphology.QuotedWords(cmavo, list(words), cmavo)
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
            morphology.LujvoBuildMode.LUJVO, [list(build_parts)]
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


def typed_every_morphology_result_property(
    render_options: morphology.PhonemeRenderOptions,
    phonemes: morphology.Phonemes,
    key: morphology.WordKey,
    options: morphology.MorphologyOptions,
    compiled: morphology.CompiledDialectDefinition,
    word: morphology.Word,
    word_like: morphology.WordLike,
    context: morphology.MorphologyContext,
    detail: morphology.MorphologyErrorDetail,
    warning: morphology.MorphologyWarning,
    error: morphology.MorphologyErrorValue,
    exception: morphology.MorphologyError,
    attempt: morphology.MorphologySegmentAttempt,
    recovered: morphology.RecoveredMorphologySegmentation,
    recovered_attempt: morphology.RecoveredMorphologySegmentAttempt,
    plain_classification: morphology.PlainWordClassification,
    classification: morphology.ValsiClassification,
    result: morphology.ValsiAnalysisResult,
    analysis: morphology.ValsiAnalysis,
    build_part: morphology.LujvoBuildPart,
    candidate: morphology.LujvoCandidate,
) -> None:
    """Assert every property type on morphology's immutable result graph."""

    assert_type(render_options.mark_stress, morphology.StressMark)
    assert_type(render_options.mark_glides, morphology.GlideMark)
    assert_type(phonemes.text, str)
    assert_type(phonemes.render(render_options), str)
    assert_type(key.kind, morphology.WordKind)
    assert_type(key.phonemes, morphology.Phonemes)

    assert_type(options.accept_latin, bool)
    assert_type(options.accept_cyrillic, bool)
    assert_type(options.accept_zbalermorna, bool)
    assert_type(options.compiled_dialect, morphology.CompiledDialectDefinition)
    assert_type(options.cmevla_as_relation_words, bool)
    assert_type(options.permissive_lexer, bool)
    assert_type(options.uppercase_marks_stress, bool)
    assert_type(options.max_recovery_errors, int)
    assert_type(options.trace, diagnostics.TraceOptions)
    assert_type(
        morphology.MorphologyOptions.default(), morphology.MorphologyOptions
    )
    assert_type(
        options.with_compiled_dialect(compiled), morphology.MorphologyOptions
    )
    assert_type(options.with_dialect(typed_dialect()), morphology.MorphologyOptions)
    assert_type(
        options.with_trace(diagnostics.TraceOptions()), morphology.MorphologyOptions
    )
    assert_type(options.with_max_recovery_errors(1), morphology.MorphologyOptions)

    assert_type(compiled.entries, tuple[morphology.CompiledDialectEntry, ...])
    for entry in compiled.entries:
        if isinstance(entry, morphology.CompiledDialectSwap):
            assert_type(entry.left, morphology.CompiledDialectWord)
            assert_type(entry.right, morphology.CompiledDialectWord)
            compiled_word = entry.left
        else:
            assert_type(entry.source, morphology.CompiledDialectWord)
            assert_type(
                entry.replacement, tuple[morphology.CompiledDialectWord, ...]
            )
            compiled_word = entry.source
        assert_type(compiled_word.word, morphology.Word)
        assert_type(compiled_word.key, morphology.WordKey)

    assert_type(word.kind, morphology.WordKind)
    assert_type(word.phonemes, morphology.Phonemes)
    assert_type(word.span, source.SourceSpan)
    assert_type(word.key, morphology.WordKey)
    assert_type(word.canonical_phonemes, str)
    assert_type(word.cmavo, morphology.Cmavo | None)
    assert_type(word.selmaho, morphology.Selmaho | None)
    assert_type(word.is_cmavo(morphology.Cmavo.ZO), bool)
    assert_type(word.is_selmaho(morphology.Selmaho.ZO), bool)
    if isinstance(word, morphology.LujvoWord):
        assert_type(word.parts, tuple[morphology.LujvoPart, ...])
        for part in word.parts:
            assert_type(part.phonemes, morphology.Phonemes)

    assert_type(word_like.byte_range, tuple[int, int] | None)
    assert_type(word_like.source_spans, tuple[source.SourceSpan, ...])
    if isinstance(word_like, morphology.PlainWord):
        assert_type(word_like.word, morphology.Word)
        assert_type(word_like.cmavo, morphology.Cmavo | None)
        assert_type(word_like.is_cmavo(morphology.Cmavo.ZO), bool)
        assert_type(word_like.is_selmaho(morphology.Selmaho.ZO), bool)
        assert_type(word_like.is_brivla(), bool)
        assert_type(word_like.is_cmevla(), bool)
    elif isinstance(word_like, morphology.QuotedWord):
        assert_type(word_like.zo, morphology.Word)
        assert_type(word_like.word, morphology.Word)
    elif isinstance(word_like, morphology.SelmahoQuotedWord):
        assert_type(word_like.mahoi, morphology.Word)
        assert_type(word_like.word, morphology.Word)
    elif isinstance(word_like, morphology.DelimitedNonLojbanQuote):
        assert_type(word_like.zoi, morphology.Word)
        assert_type(word_like.opening_delimiter, morphology.Word)
        assert_type(word_like.quoted_text, morphology.Verbatim)
        assert_type(word_like.closing_delimiter, morphology.Word)
        assert_type(word_like.quoted_text.span, source.SourceSpan)
        assert_type(word_like.quoted_text.text, str)
    elif isinstance(word_like, morphology.QuotedWords):
        assert_type(word_like.lohu, morphology.Word)
        assert_type(word_like.quoted_words, tuple[morphology.Word, ...])
        assert_type(word_like.lehu, morphology.Word)
    elif isinstance(word_like, morphology.DelimitedWordQuote):
        assert_type(word_like.marker, morphology.Word)
        assert_type(word_like.quoted_text, morphology.Verbatim)
    elif isinstance(word_like, morphology.LerfuWord):
        assert_type(word_like.base, morphology.WordLike)
        assert_type(word_like.bu, morphology.Word)
    else:
        assert_type(word_like, morphology.ZeiCompound)
        assert_type(word_like.left, morphology.WordLike)
        assert_type(word_like.zei, morphology.Word)
        assert_type(word_like.right, morphology.Word)

    assert_type(context.kind, morphology.MorphologyContextKind)
    assert_type(context.char_start, int)
    assert_type(context.char_end, int)
    assert_type(context.label, str)
    if isinstance(detail, morphology.InvalidLujvoDetail):
        assert_type(detail.parsed_prefix, str | None)
        assert_type(detail.expected, morphology.LujvoParseExpectation)
    elif isinstance(detail, morphology.ExpectedWordDetail):
        assert_type(detail.expected, morphology.ExpectedWordDetailKind)
    elif isinstance(detail, morphology.InvalidZoiDelimiterDetail):
        assert_type(detail.reason, morphology.ZoiDelimiterDetailKind)
    elif isinstance(detail, morphology.PhonotacticDetail):
        assert_type(detail.reason, morphology.PhonotacticDetailKind)
    else:
        assert_type(
            detail,
            morphology.FuhivlaContainsYDetail | morphology.SlinkuhiDetail,
        )

    assert_type(warning.kind, morphology.MorphologyWarningKind)
    assert_type(warning.code, str)
    assert_type(warning.message, str)
    assert_type(warning.char_start, int)
    assert_type(warning.char_end, int)
    assert_type(warning.text, str)
    assert_type(warning.context, morphology.MorphologyContext | None)
    assert_type(warning.ignored_character_count, int | None)
    assert_type(warning.to_diagnostic(""), diagnostics.Diagnostic)

    assert_type(error.code, str)
    assert_type(error.to_diagnostic(""), diagnostics.Diagnostic)
    if isinstance(error, morphology.InvalidMorphology):
        assert_type(error.kind, morphology.MorphologyErrorKind)
        assert_type(error.message, str)
        assert_type(error.char_start, int)
        assert_type(error.char_end, int)
        assert_type(error.text, str)
        assert_type(error.context, morphology.MorphologyContext | None)
        assert_type(error.detail, morphology.MorphologyErrorDetail | None)
    elif isinstance(error, morphology.UnterminatedZoiQuote):
        assert_type(error.char_offset, int)
        assert_type(error.delimiter, str)
        assert_type(error.context, morphology.MorphologyContext | None)
    else:
        assert_type(error, morphology.SourceSpanMorphologyError)
        assert_type(error.error, source.SourceLocationError)

    assert_type(exception.value, morphology.MorphologyErrorValue)
    assert_type(exception.original_source, str)
    assert_type(exception.source_id, source.SourceId | None)
    assert_type(exception.code, str)
    assert_type(exception.diagnostic, diagnostics.Diagnostic)
    assert_type(exception.spans, tuple[source.SourceSpan, ...])
    assert_type(exception.context, morphology.MorphologyContext | None)
    assert_type(exception.detail, morphology.MorphologyErrorDetail | None)
    assert_type(exception.warnings, tuple[morphology.MorphologyWarning, ...])
    assert_type(exception.trace, diagnostics.TraceReport | None)

    assert_type(attempt.source, str)
    assert_type(attempt.source_id, source.SourceId | None)
    assert_type(attempt.succeeded, bool)
    assert_type(attempt.words, tuple[morphology.WordLike, ...] | None)
    assert_type(attempt.error, morphology.MorphologyErrorValue | None)
    assert_type(attempt.warnings, tuple[morphology.MorphologyWarning, ...])
    assert_type(attempt.trace, diagnostics.TraceReport | None)

    assert_type(recovered.words, tuple[morphology.WordLike, ...])
    assert_type(recovered.errors, tuple[morphology.MorphologyErrorValue, ...])
    assert_type(recovered.error_regions, tuple[source.SourceSpan, ...])
    assert_type(recovered.warnings, tuple[morphology.MorphologyWarning, ...])
    assert_type(recovered_attempt.source, str)
    assert_type(recovered_attempt.source_id, source.SourceId | None)
    assert_type(
        recovered_attempt.result, morphology.RecoveredMorphologySegmentation
    )
    assert_type(recovered_attempt.trace, diagnostics.TraceReport | None)

    assert_type(plain_classification.category, morphology.WordKind)
    assert_type(plain_classification.phonemes, str)
    assert_type(plain_classification.selmaho, str | None)
    assert_type(plain_classification.split, str | None)
    assert_type(
        plain_classification.parts, tuple[morphology.ValsiLujvoPart, ...]
    )
    assert_type(plain_classification.stage, morphology.ValsiFuhivlaStage | None)
    for part in plain_classification.parts:
        assert_type(part.kind, morphology.ValsiLujvoPartKind)
        assert_type(part.text, str)
        assert_type(part.rafsi_kind, morphology.ValsiLujvoRafsiKind | None)

    assert_type(classification.kind, morphology.ValsiClassificationKind)
    if isinstance(classification, morphology.PlainWordValsiClassification):
        assert_type(classification.word, morphology.PlainWordClassification)
    elif isinstance(classification, morphology.QuotedWordValsiClassification):
        assert_type(classification.marker, morphology.PlainWordClassification)
        assert_type(
            classification.quoted_word, morphology.PlainWordClassification
        )
    elif isinstance(
        classification, morphology.DelimitedNonLojbanQuoteValsiClassification
    ):
        assert_type(classification.marker, morphology.PlainWordClassification)
        assert_type(classification.delimiter, str)
    elif isinstance(classification, morphology.QuotedWordsValsiClassification):
        assert_type(classification.marker, morphology.PlainWordClassification)
        assert_type(
            classification.quoted_words,
            tuple[morphology.PlainWordClassification, ...],
        )
    elif isinstance(
        classification, morphology.DelimitedWordQuoteValsiClassification
    ):
        assert_type(classification.marker_text, str)
    elif isinstance(classification, morphology.LerfuWordValsiClassification):
        assert_type(classification.base, morphology.ValsiClassification)
        assert_type(classification.suffix, morphology.PlainWordClassification)
    else:
        assert_type(classification, morphology.ZeiCompoundValsiClassification)
        assert_type(classification.left, morphology.ValsiClassification)
        assert_type(classification.link, morphology.PlainWordClassification)
        assert_type(classification.right, morphology.PlainWordClassification)

    assert_type(result.status, morphology.ValsiAnalysisStatus)
    assert_type(result.is_valid, bool)
    assert_type(result.word, morphology.WordLike | None)
    assert_type(result.classification, morphology.ValsiClassification | None)
    assert_type(result.error, morphology.MorphologyErrorValue | None)
    assert_type(result.words, tuple[morphology.WordLike, ...])
    assert_type(analysis.input, str)
    assert_type(analysis.warnings, tuple[morphology.MorphologyWarning, ...])
    assert_type(analysis.result, morphology.ValsiAnalysisResult)

    if isinstance(build_part, morphology.LujvoRafsiBuildPart):
        assert_type(build_part.text, str)
    else:
        assert_type(build_part, morphology.LujvoBrivlaCoreBuildPart)
        assert_type(build_part.text, str)
    assert_type(candidate.word, str)
    assert_type(candidate.parts, tuple[str, ...])
    assert_type(candidate.score, int)
