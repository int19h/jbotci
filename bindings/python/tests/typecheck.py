"""Strict-type-check smoke coverage for packaged public declarations."""

from typing import assert_never, assert_type

from jbotci import (
    AnalyzedText,
    ParsedText,
    RecoveredParsedText,
    Sample,
    SampleMode,
    analyze,
    dictionary,
    diagnostics,
    dialect,
    jvozba,
    morphology,
    parse,
    parse_recovered,
    sample_mode,
    semantics,
    smoke,
    source,
    syntax,
)

references = semantics.references


def exhaustive_jvozba_input(value: jvozba.JvozbaInput) -> str:
    """Prove dictionary words and exact rafsi remain a closed input union."""

    if isinstance(value, jvozba.Word):
        assert_type(value.value, str)
        return value.value
    if isinstance(value, jvozba.FixedRafsi):
        assert_type(value.value, str)
        return value.value
    assert_never(value)


def exhaustive_jvozba_error(value: jvozba.JvozbaErrorValue) -> str:
    """Prove all eight structured Rust errors narrow without a fallback."""

    if isinstance(value, jvozba.RequiresAtLeastTwoInputs):
        return str(value)
    if isinstance(value, jvozba.FixedRafsiEmpty):
        return str(value)
    if isinstance(value, jvozba.NonFinalUniversalLongRafsi):
        assert_type(value.offending, str)
        return value.offending
    if isinstance(value, jvozba.FinalConsonant):
        assert_type(value.offending, str)
        assert_type(value.is_fixed_rafsi, bool)
        return value.offending
    if isinstance(value, jvozba.NoRafsiAvailable):
        assert_type(value.offending, str)
        return value.offending
    if isinstance(value, jvozba.NoDictionaryEntry):
        assert_type(value.offending, str)
        return value.offending
    if isinstance(value, jvozba.CouldNotBuildLujvo):
        return str(value)
    if isinstance(value, jvozba.CouldNotBuildCompound):
        return str(value)
    assert_never(value)


def typed_jvozba_surface() -> jvozba.JvozbaBuildResult:
    """Exercise composition, decomposition, provenance, and typed exceptions."""

    word = jvozba.Word("lojbo")
    fixed = jvozba.FixedRafsi("bau")
    inputs: tuple[jvozba.JvozbaInput, ...] = (word, fixed)
    result = jvozba.build(inputs)
    detailed = jvozba.build_best_jvozba_detailed(
        jvozba.JvozbaMode.LUJVO, dictionary.english, list(inputs)
    )
    assert_type(result, jvozba.JvozbaBuildResult)
    assert_type(detailed, jvozba.JvozbaBuildResult)
    assert_type(result.word, str)
    assert_type(result.segments, tuple[jvozba.JvozbaSegment, ...])
    for segment in result.segments:
        assert_type(segment.kind, jvozba.JvozbaSegmentKind)
        assert_type(segment.text, str)

    assert_type(
        jvozba.word_can_enter_jvozba_pane(dictionary.english, "lojbo"),
        bool,
    )
    assert_type(jvozba.can_use_word("lojbo"), bool)
    decomposition = jvozba.decompose_lujvo_like("jetcybolxada")
    assert_type(decomposition, jvozba.LujvoDecomposition | None)
    if decomposition is not None:
        assert_type(
            decomposition.segments, tuple[jvozba.LujvoSegmentInfo, ...]
        )
        assert_type(decomposition.source_words, tuple[str, ...])
        for info in decomposition.segments:
            assert_type(info.segment, morphology.LujvoPart)
            assert_type(info.source, str | None)

    try:
        jvozba.build([jvozba.Word("missing"), jvozba.Word("also-missing")])
    except jvozba.NoDictionaryEntryError as error:
        assert_type(error.value, jvozba.NoDictionaryEntry)
        assert_type(error.offending, str)

    assert exhaustive_jvozba_input(word)
    assert exhaustive_jvozba_error(jvozba.NoDictionaryEntry("missing"))
    return result


def exhaustive_syntax_expected_token(value: syntax.SyntaxExpectedToken) -> str:
    """Prove completion-token payloads narrow through the closed public union."""

    if isinstance(value, syntax.SyntaxExpectedTokenCmavo):
        assert_type(value.cmavo, morphology.Cmavo)
        return value.cmavo.value
    if isinstance(value, syntax.SyntaxExpectedTokenSelmaho):
        assert_type(value.selmaho, morphology.Selmaho)
        return value.selmaho.value
    if isinstance(value, syntax.SyntaxExpectedTokenWordCategory):
        assert_type(value.category, syntax.SyntaxWordCategory)
        return value.category.value
    if isinstance(value, syntax.SyntaxExpectedTokenEndOfInput):
        return "end"
    if isinstance(value, syntax.SyntaxExpectedTokenNamed):
        assert_type(value.name, str)
        return value.name
    assert_never(value)


def exhaustive_syntax_expectation_reason(
    value: syntax.SyntaxExpectationReason,
) -> str:
    """Prove all concrete expectation reasons retain their typed payloads."""

    if isinstance(value, syntax.SyntaxExpectationReasonContinueCurrent):
        assert_type(value.construct, str)
        return value.construct
    if isinstance(value, syntax.SyntaxExpectationReasonStartNested):
        assert_type(value.construct, str)
        return value.construct
    if isinstance(value, syntax.SyntaxExpectationReasonEndThenStart):
        assert_type(value.starts, str)
        assert_type(value.ends, tuple[str, ...])
        return value.starts
    assert_never(value)


def exhaustive_syntax_recovery_parse(value: syntax.SyntaxRecoveryParse) -> int:
    """Prove strict success and recovered success remain distinct variants."""

    match value:
        case syntax.SyntaxRecoveryParseValid(parse=result):
            assert_type(result, syntax.SyntaxParse)
            assert_type(result.parse_tree, syntax.strict.TextSyntax)
            return len(result.warnings)
        case syntax.SyntaxRecoveryParseRecovered(parse=result):
            assert_type(result, syntax.RecoveredSyntaxParse)
            assert_type(result.parse_tree, syntax.recovered.TextSyntax)
            return len(result.errors)
    assert_never(value)


def typed_parser_surface(text: str, cursor: int) -> ParsedText | RecoveredParsedText:
    """Exercise high- and low-level strict/recovered/completion declarations."""

    recovery_policy = syntax.SyntaxRecoveryErrorPolicy()
    assert_type(
        syntax.SyntaxRecoveryErrorPolicy.DEFAULT_PER_STATEMENT,
        int,
    )
    assert_type(
        syntax.SyntaxRecoveryErrorPolicy.DEFAULT_GLOBAL_HARD_CAP,
        int,
    )
    assert_type(recovery_policy.per_statement, int)
    assert_type(recovery_policy.global_hard_cap, int)
    assert_type(
        recovery_policy.with_per_statement_limit(2),
        syntax.SyntaxRecoveryErrorPolicy,
    )
    assert_type(
        recovery_policy.with_global_hard_cap(8),
        syntax.SyntaxRecoveryErrorPolicy,
    )

    options = syntax.ParseOptions.default()
    assert_type(options.dialect, dialect.DialectDefinition)
    assert_type(options.trace, diagnostics.TraceOptions)
    assert_type(options.error_context_depth, int)
    assert_type(
        options.recovery_error_policy,
        syntax.SyntaxRecoveryErrorPolicy,
    )
    assert_type(options.max_recovery_errors, int)
    assert_type(options.with_dialect(dialect.DialectDefinition()), syntax.ParseOptions)
    assert_type(
        options.with_trace(diagnostics.TraceOptions()), syntax.ParseOptions
    )
    assert_type(options.with_error_context_depth(2), syntax.ParseOptions)
    assert_type(
        options.with_recovery_error_policy(recovery_policy),
        syntax.ParseOptions,
    )
    assert_type(options.with_max_recovery_errors(2), syntax.ParseOptions)

    strict_result = parse(text, parse_options=options, source_id="typed")
    assert_type(strict_result, ParsedText)
    assert_type(strict_result.parse_tree, syntax.strict.TextSyntax)
    assert_type(strict_result.syntax, syntax.SyntaxParse)
    assert_type(strict_result.warnings, tuple[syntax.SyntaxWarning, ...])
    recovered_result = parse_recovered(text, parse_options=options)
    assert_type(recovered_result, RecoveredParsedText)
    assert_type(recovered_result.parse_tree, syntax.recovered.TextSyntax)
    assert_type(recovered_result.syntax_errors, tuple[syntax.SyntaxErrorValue, ...])

    words = morphology.segment(text)
    tokens = syntax.normalize_syntax_tokens(words, options=options)
    assert_type(tokens, tuple[syntax.Token, ...])
    assert_type(
        syntax.partition_syntax_text_units(
            tokens, syntax.SyntaxTextUnitGranularity.PARAGRAPH
        ),
        tuple[syntax.SyntaxTextUnit, ...],
    )
    assert_type(
        syntax.syntax_text_structure(tokens),
        tuple[syntax.SyntaxTextStructureEvent, ...],
    )
    assert_type(syntax.parse_text(words, options=options), syntax.strict.TextSyntax)
    assert_type(
        syntax.parse_text_attempt(words, options=options),
        syntax.SyntaxParseAttempt,
    )
    assert_type(
        syntax.parse_syntax_tree(
            words, source_text=text, options=options, source_id=source.SourceId("typed")
        ),
        syntax.SyntaxParse,
    )
    assert_type(
        syntax.parse_syntax_tree_attempt(words, source_text=text, options=options),
        syntax.SyntaxParseAttempt,
    )
    assert_type(
        syntax.parse_syntax_tree_recovered(
            words, source_text=text, options=options
        ),
        syntax.RecoveredSyntaxParse,
    )
    assert_type(
        syntax.parse_syntax_tree_recovered_attempt(
            words, source_text=text, options=options
        ),
        syntax.RecoveredSyntaxParseAttempt,
    )
    assert_type(
        syntax.parse_syntax_tree_with_recovery(
            words, source_text=text, options=options
        ),
        syntax.SyntaxRecoveryParse,
    )
    assert_type(
        syntax.parse_syntax_tree_with_recovery_attempt(
            words, source_text=text, options=options
        ),
        syntax.SyntaxRecoveryParseAttempt,
    )
    assert_type(
        syntax.expected_continuations(words, options=options),
        tuple[syntax.SyntaxExpectation, ...],
    )
    assert_type(
        syntax.expected_continuations_with_time_limit(words, 0.1, options=options),
        tuple[syntax.SyntaxExpectation, ...],
    )
    assert_type(
        syntax.expected_continuations_at_cursor(
            text, cursor, parse_options=options
        ),
        tuple[syntax.SyntaxExpectation, ...],
    )
    assert_type(
        syntax.expected_continuations_for_text(text, parse_options=options),
        tuple[syntax.SyntaxExpectation, ...],
    )
    assert_type(
        syntax.syntax_tree_eq_ignoring_spans(
            strict_result.parse_tree, strict_result.parse_tree
        ),
        bool,
    )
    return strict_result if cursor <= len(text) else recovered_result


def exhaustive_linked_sumti(value: syntax.strict.LinkedSumtiSyntax) -> str:
    """Prove the packaged closed union supports exhaustive class matching."""

    match value:
        case syntax.strict.LinkedSumtiSyntaxPlaceTaggedLinkedSumti():
            return "place"
        case syntax.strict.LinkedSumtiSyntaxTenseTaggedLinkedSumti():
            return "tense"
        case syntax.strict.LinkedSumtiSyntaxPlainLinkedSumti():
            return "plain"
        case syntax.strict.LinkedSumtiSyntaxEmptyLinkedSumti():
            return "empty"
    assert_never(value)


def exhaustive_place_slot(value: references.PlaceSlot) -> int | None:
    """Prove all native PlaceSlot payload variants narrow exhaustively."""

    if isinstance(value, references.NumberedPlaceSlot):
        assert_type(value.place, int)
        assert_type(value.numbered_index(), int)
        return value.place
    if isinstance(value, references.ModalPlaceSlot):
        assert_type(value.tag, references.RawSyntaxNodeId | None)
        assert_type(value.numbered_index(), None)
        return None
    if isinstance(value, references.PlaceQuestionPlaceSlot):
        assert_type(value.numbered_index(), None)
        return None
    if isinstance(value, references.FaiPlaceSlot):
        assert_type(value.numbered_index(), None)
        return None
    assert_never(value)


def exhaustive_place_propagation(
    value: references.PlaceFramePropagation,
) -> int:
    """Prove all native place-frame propagation variants remain distinct."""

    if isinstance(value, references.NoPlaceFramePropagation):
        return 0
    if isinstance(value, references.ForwardPlaceFramePropagation):
        assert_type(value.inner, references.SelbriPlaceFrameId)
        return value.inner.value
    if isinstance(value, references.ConversionPlaceFramePropagation):
        assert_type(value.inner, references.SelbriPlaceFrameId)
        assert_type(value.converted_place, int)
        return value.converted_place
    if isinstance(value, references.JaiPlaceFramePropagation):
        assert_type(value.inner, references.SelbriPlaceFrameId)
        return value.inner.value
    if isinstance(value, references.ConnectiveBranchesPlaceFramePropagation):
        assert_type(
            value.branches, tuple[references.SelbriPlaceFrameId, ...]
        )
        return len(value.branches)
    if isinstance(value, references.CompoundPlaceFramePropagation):
        assert_type(value.head, references.SelbriPlaceFrameId)
        assert_type(
            value.modifiers, tuple[references.SelbriPlaceFrameId, ...]
        )
        return value.head.value
    if isinstance(value, references.CoPlaceFramePropagation):
        assert_type(value.leading, references.SelbriPlaceFrameId)
        assert_type(value.trailing, references.SelbriPlaceFrameId)
        return value.leading.value
    assert_never(value)


def exhaustive_reference_target(value: references.ReferenceTarget) -> int:
    """Prove every native discourse-reference target retains its payload."""

    if isinstance(value, references.ResolvedNodeReferenceTarget):
        assert_type(value.node, references.RawSyntaxNodeId)
        return value.node.value
    if isinstance(value, references.ResolvedFrameReferenceTarget):
        assert_type(value.frame, references.SelbriPlaceFrameId)
        return value.frame.value
    if isinstance(value, references.AmbiguousNodesReferenceTarget):
        assert_type(value.nodes, tuple[references.RawSyntaxNodeId, ...])
        return len(value.nodes)
    if isinstance(value, references.UnresolvedReferenceTarget):
        assert_type(value.reason, str)
        return len(value.reason)
    if isinstance(value, references.VagueReferenceTarget):
        assert_type(value.kind, references.VagueReferenceKind)
        return len(value.kind.value)
    assert_never(value)


def exhaustive_reference_error(
    value: references.ReferenceAnalysisErrorValue,
) -> str:
    """Prove the structured core error union is closed."""

    if isinstance(value, references.MissingRootNode):
        return str(value)
    assert_never(value)


def typed_reference_surface(
    text: str,
    tree: syntax.strict.TextSyntax,
    syntax_parse: syntax.SyntaxParse,
) -> references.ReferenceAnalysis:
    """Exercise the high- and low-level owning reference-analysis APIs."""

    high_level = analyze(text)
    assert_type(high_level, AnalyzedText)
    assert_type(high_level.parsed, ParsedText)
    assert_type(high_level.reference_analysis, references.ReferenceAnalysis)
    assert_type(high_level.parse_tree, syntax.strict.TextSyntax)

    from_tree = references.analyze_references(tree)
    from_parse = references.analyze_references(syntax_parse)
    assert_type(from_tree, references.ReferenceAnalysis)
    assert_type(from_parse, references.ReferenceAnalysis)
    assert_type(from_tree.syntax, syntax.strict.TextSyntax)

    index = from_tree.syntax_index
    root = index.root()
    assert_type(root, references.TextNodeId)
    assert_type(root.value, int)
    assert_type(root.raw_id, references.RawSyntaxNodeId)
    assert_type(index.node_count(), int)
    assert_type(index.node(root.raw_id), references.SyntaxNode | None)
    assert_type(
        index.metadata(root.raw_id), references.SyntaxNodeMetadata | None
    )
    assert_type(index.id_of(tree), references.RawSyntaxNodeId | None)
    assert_type(index.text_node_id(tree), references.TextNodeId | None)

    places = from_tree.place_analysis
    assert_type(places.frames(), tuple[references.SelbriPlaceFrame, ...])
    assert_type(
        places.assignments(), tuple[references.SumtiPlaceAssignment, ...]
    )
    for frame in places.frames():
        assert_type(
            places.frame(frame.id), references.SelbriPlaceFrame | None
        )
        assert_type(
            places.frames_for_node(frame.node),
            tuple[references.SelbriPlaceFrameId, ...],
        )
        assert_type(
            places.assignments_for_frame(frame.id),
            tuple[references.SumtiPlaceAssignmentId, ...],
        )
    for assignment in places.assignments():
        assert_type(
            places.assignment(assignment.id),
            references.SumtiPlaceAssignment | None,
        )
        assert_type(
            places.assignments_for_sumti(assignment.sumti),
            tuple[references.SumtiPlaceAssignmentId, ...],
        )
        if assignment.term is not None:
            assert_type(
                places.assignments_for_term(assignment.term),
                tuple[references.SumtiPlaceAssignmentId, ...],
            )
        assert_type(
            places.assignments_for_frame_slot(
                assignment.frame, assignment.slot
            ),
            tuple[references.SumtiPlaceAssignmentId, ...],
        )
        assert_type(
            places.first_argument_for_place(
                assignment.frame, assignment.slot
            ),
            references.SumtiNodeId | None,
        )
    assert_type(
        from_tree.discourse_references.edges(),
        tuple[references.ReferenceEdge, ...],
    )
    assert_type(
        references.fixture_projection(from_tree),
        references.ReferenceFixtureProjection,
    )
    assert_type(references.fixture_projection_json(from_tree), str)
    return from_tree


def typed_reference_node_lookups(
    index: references.GeneratedSyntaxIndex,
    paragraph: syntax.strict.ParagraphSyntax,
    statement: syntax.strict.StatementSyntax,
    bridi: syntax.strict.BridiSyntax,
    bridi_tail: syntax.strict.BridiTailSyntax,
    selbri: syntax.strict.SelbriSyntax,
    tanru_unit: syntax.strict.TanruUnitSyntax,
    term: syntax.strict.TermSyntax,
    sumti: syntax.strict.SumtiSyntax,
    free_modifier: syntax.strict.FreeModifierSyntax,
    abstraction: syntax.strict.AbstractionTanruUnitSyntax,
    mekso: syntax.strict.MeksoSyntax,
    mekso_operator: syntax.strict.MeksoOperatorSyntax,
) -> None:
    """Exercise every family-specific generated-syntax ID lookup."""

    assert_type(
        index.paragraph_node_id(paragraph), references.ParagraphNodeId | None
    )
    assert_type(
        index.statement_node_id(statement), references.StatementNodeId | None
    )
    assert_type(index.bridi_node_id(bridi), references.BridiNodeId | None)
    assert_type(
        index.bridi_tail_node_id(bridi_tail),
        references.BridiTailNodeId | None,
    )
    assert_type(index.selbri_node_id(selbri), references.SelbriNodeId | None)
    assert_type(
        index.tanru_unit_node_id(tanru_unit),
        references.TanruUnitNodeId | None,
    )
    assert_type(index.term_node_id(term), references.TermNodeId | None)
    assert_type(index.sumti_node_id(sumti), references.SumtiNodeId | None)
    assert_type(
        index.free_modifier_node_id(free_modifier),
        references.FreeModifierNodeId | None,
    )
    assert_type(
        index.abstraction_node_id(abstraction),
        references.AbstractionNodeId | None,
    )
    assert_type(index.mekso_node_id(mekso), references.MeksoNodeId | None)
    assert_type(
        index.mekso_operator_node_id(mekso_operator),
        references.MeksoOperatorNodeId | None,
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


def dictionary_pronunciation_targets(
    word: str,
) -> tuple[int, tuple[int, ...], float] | None:
    """Exercise target views, returned identifiers, and concrete realizations."""
    entry = dictionary.english.lookup_word(word)
    if entry is None:
        return None
    index = dictionary.english.entry_index_for_entry(entry)
    if index is None:
        return None
    sound = next(
        (value for value in dictionary.english.sound_index if value.entry_index == index),
        None,
    )
    if sound is None:
        return None
    sequence: dictionary.PronunciationTargetSequenceView = sound.pronunciation_targets
    target: dictionary.PronunciationTargetId = sequence.targets[0]
    realization: dictionary.IpaSegmentId | None = target.realization(0)
    values = tuple(value.value for value in target.realizations)
    if realization is not None:
        assert realization.value == values[0]
    assert target.realization(-1) is None
    return (target.value, values, sequence.self_similarity)


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
    location_exception = source.SourceLocationException(location_errors[0])
    assert_type(location_exception.value, source.SourceLocationError)
    span_errors: tuple[source.DiagnosticSpanError, ...] = (
        source.CharOffsetOutOfBounds(2, 1),
        source.ByteOffsetOutOfBounds(3, 2),
        source.ByteOffsetNotCharBoundary(1),
        source.SourceLocation(source.ZeroLine()),
    )
    assert_type(span_errors[0], source.DiagnosticSpanError)
    span_exception = source.DiagnosticSpanException(span_errors[0])
    assert_type(span_exception.value, source.DiagnosticSpanError)
    assert_type(source.CharOffsetOutOfBounds(2, 1).offset, int)
    assert_type(source.CharOffsetOutOfBounds(2, 1).source_len, int)
    assert_type(source.ByteOffsetOutOfBounds(3, 2).offset, int)
    assert_type(source.ByteOffsetOutOfBounds(3, 2).source_len, int)
    assert_type(source.ByteOffsetNotCharBoundary(1).offset, int)
    assert_type(source.SourceLocation(source.ZeroLine()).error, source.SourceLocationError)
    assert_type(diagnostics.DEFAULT_TRACE_LIMIT, int)
    trace_detail = diagnostics.InvalidTraceLevel(5)
    assert_type(trace_detail.value, int)
    trace_error = diagnostics.TraceOptionError(trace_detail)
    assert_type(trace_error.value, diagnostics.InvalidTraceLevel)
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

    assert_type(morphology.MORPHOLOGY_TRACE_FILTERS, tuple[str, ...])
    assert_type(
        morphology.PERMISSIVE_IGNORABLE_RESERVED_CHARACTERS,
        tuple[str, ...],
    )
    dialect_detail = morphology.InvalidDialectWord("aaa")
    assert_type(dialect_detail.word, str)
    dialect_error = morphology.DialectCompilationError(dialect_detail)
    assert_type(dialect_error.value, morphology.InvalidDialectWord)
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
    for valsi_part in plain_classification.parts:
        assert_type(valsi_part.kind, morphology.ValsiLujvoPartKind)
        assert_type(valsi_part.text, str)
        assert_type(
            valsi_part.rafsi_kind, morphology.ValsiLujvoRafsiKind | None
        )

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
