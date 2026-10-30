use super::super::*;

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn run_vlacku<WOut: Write, WErr: Write>(
    input: VlackuInput,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color: bool,
    glyphs: GlyphStyle,
    output_terminal_width: Option<usize>,
    tool_context: Option<&mut ToolExecutionContext<'_>>,
    limits: Option<&ToolExecutionLimits>,
) -> Result<CliStatus> {
    validate_vlacku_input(&input)?;
    let options = vlacku_search_options(&input)?;
    let output = if let Some(query) = semantic_vlacku_query(&input) {
        match run_semantic_vlacku(&query, &options, tool_context) {
            Ok(output) => output,
            Err(error) => {
                writeln!(stderr, "vlacku: {error}")?;
                return Ok(CliStatus::InvalidInput);
            }
        }
    } else {
        run_vlacku_requests(jbotci_dictionary_data::english(), &input.requests, &options)
    };
    for diagnostic in &output.diagnostics {
        writeln!(stderr, "vlacku: {diagnostic}")?;
    }
    if !output.cards.is_empty() || output.outcome != VlackuOutcome::Invalid {
        write!(
            stdout,
            "{}",
            render_vlacku_output_with_limit(
                &output,
                new!(VlackuRenderOptions {
                    color,
                    glyphs,
                    output_terminal_width,
                    sumti_places: input.sumti_places,
                    show_etymology: input.show_etymology,
                }),
                tool_render_limits(limits).text,
            )?
        )?;
    }
    Ok(cli_status_from_vlacku_outcome(output.outcome))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn run_semantic_vlacku(
    query: &str,
    options: &VlackuSearchOptions,
    tool_context: Option<&mut ToolExecutionContext<'_>>,
) -> Result<VlackuSearchOutput> {
    let query = query.to_owned();
    if query.trim().is_empty() {
        bail!("vlacku query text must be non-empty.");
    }
    let dictionary = jbotci_dictionary_data::english();
    // Entry filters apply while ranking, so the fetch is bounded by the
    // requested count instead of ranking the whole dictionary first; the
    // similarity threshold is applied below, on the scores.
    let mut entry_allowed = |entry_index: usize| {
        dictionary
            .entries()
            .get(entry_index)
            .is_some_and(|entry| dictionary_entry_passes_vlacku_entry_filters(entry, options))
    };
    let hits = if let Some(context) = tool_context {
        if let Some(service) = context.embedding_search()? {
            service
                .semantic_vlacku_hits_filtered(&query, options.count, &mut entry_allowed)
                .map_err(|error| anyhow!(error.to_string()))?
        } else {
            semantic_vlacku_hits_with_new_backend(&query, options.count, &mut entry_allowed)?
        }
    } else {
        semantic_vlacku_hits_with_new_backend(&query, options.count, &mut entry_allowed)?
    };
    let cards = hits
        .into_iter()
        .filter_map(|hit| {
            dictionary
                .entries()
                .get(hit.entry_index)
                .map(|entry| (hit.score, entry))
        })
        .filter(|(score, entry)| {
            dictionary_entry_passes_vlacku_filters(entry, options, Some(*score), true)
        })
        .take(options.count)
        .map(|(score, entry)| {
            dictionary_entry_card(dictionary, entry, Some(score), options.decompose_lujvo)
        })
        .collect::<Vec<_>>();
    Ok(VlackuSearchOutput {
        cards,
        outcome: VlackuOutcome::Found,
        diagnostics: Vec::new(),
    })
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn semantic_vlacku_hits_with_new_backend<F>(
    query: &str,
    count: usize,
    entry_allowed: F,
) -> Result<Vec<jbotci_embeddings::DictionarySemanticHit>>
where
    F: FnMut(usize) -> bool,
{
    let index_root = default_index_root().map_err(|error| anyhow!(error.to_string()))?;
    let mut backend = load_backend_for_search(DEFAULT_MODEL_KEY, None)
        .map_err(|error| anyhow!(error.to_string()))?;
    semantic_vlacku_hits_filtered(
        &mut backend,
        query,
        count,
        &index_root,
        DEFAULT_MODEL_KEY,
        entry_allowed,
    )
    .map_err(|error| anyhow!(error.to_string()))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|query| !query.is_empty()))]
fn semantic_vlacku_query(input: &VlackuInput) -> Option<String> {
    if input.requests.is_empty() {
        let query = joined_query_text(&input.query);
        return (!query.is_empty()).then_some(query);
    }
    match input.requests.as_slice() {
        [request] => match request.as_data() {
            data!(VlackuRequest::Meaning(query)) => Some(query.clone()),
            _ => None,
        },
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_vlacku_input(input: &VlackuInput) -> Result<()> {
    if input.count == Some(0) {
        bail!("`--count` must be greater than 0");
    }
    if let Some(min_similarity) = input.min_similarity {
        if !(0.0..=100.0).contains(&min_similarity) {
            bail!("`--min-similarity` must be between 0 and 100");
        }
    }
    if input.requests.is_empty() {
        if input.query.is_empty() {
            bail!("No query provided for vlacku. Use --valsi, --rafsi, --lujvo, or --sound.");
        }
    }
    if !input.query.is_empty() && !input.requests.is_empty() {
        bail!(
            "Do not pass positional query text when using --valsi, --rafsi, --lujvo, or --sound."
        );
    }
    let sound_count = input
        .requests
        .iter()
        .filter(|request| matches!(request.as_data(), data!(VlackuRequest::Sound(_))))
        .count();
    if sound_count > 1 {
        bail!("`--sound` may be specified only once");
    }
    if sound_count == 1 && input.requests.len() > 1 {
        bail!("`--sound` cannot be combined with --valsi, --rafsi, or --lujvo");
    }
    let meaning_count = input
        .requests
        .iter()
        .filter(|request| matches!(request.as_data(), data!(VlackuRequest::Meaning(_))))
        .count();
    if meaning_count > 1 {
        bail!("semantic vlacku query may be specified only once");
    }
    if meaning_count == 1 && input.requests.len() > 1 {
        bail!(
            "semantic vlacku query cannot be combined with --valsi, --rafsi, --lujvo, or --sound"
        );
    }
    if input.min_similarity.is_some()
        && sound_count != 1
        && meaning_count != 1
        && !input.requests.is_empty()
    {
        bail!("`--min-similarity` is only valid with `--sound` or semantic search");
    }
    for request in &input.requests {
        validate_vlacku_request_value(request)?;
    }
    let _ = parse_vlacku_word_types(&input.word_types)?;
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn validate_vlacku_request_value(request: &VlackuRequest) -> Result<()> {
    let (flag, value) = match request.as_data() {
        data!(VlackuRequest::Valsi(value)) => ("--valsi", value),
        data!(VlackuRequest::Rafsi(value)) => ("--rafsi", value),
        data!(VlackuRequest::Lujvo(value)) => ("--lujvo", value),
        data!(VlackuRequest::Sound(value)) => ("--sound", value),
        data!(VlackuRequest::Meaning(value)) => ("semantic query", value),
    };
    if value.trim().is_empty() {
        bail!("{flag} requires a non-empty value");
    }
    Ok(())
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn vlacku_search_options(input: &VlackuInput) -> Result<VlackuSearchOptions> {
    let word_types = parse_vlacku_word_types(&input.word_types)?;
    Ok(new!(VlackuSearchOptions {
        skip: 0,
        count: input.count.unwrap_or(DEFAULT_VLACKU_RESULT_COUNT),
        word_types,
        min_votes: input.min_votes,
        min_similarity: input.min_similarity,
        decompose_lujvo: input.decompose_lujvo,
    }))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn parse_vlacku_word_types(raw_values: &[String]) -> Result<Vec<WordTypeFilter>> {
    let mut values = Vec::new();
    for raw_value in raw_values {
        for piece in raw_value.split(',') {
            let normalized = normalize_word_type_filter(piece);
            if normalized.is_empty() {
                continue;
            }
            let Some(filter) = parse_word_type_filter(&normalized) else {
                bail!(
                    "Unknown `--word-type` value: {normalized}. Use gismu, lujvo, cmavo, cmevla, fu'ivla, or brivla."
                );
            };
            if !is_valid_vlacku_word_type_filter(filter) {
                bail!(
                    "Unknown `--word-type` value: {normalized}. Use gismu, lujvo, cmavo, cmevla, fu'ivla, or brivla."
                );
            }
            if !values.contains(&filter) {
                values.push(filter);
            }
        }
    }
    if !raw_values.is_empty() && values.is_empty() {
        bail!("`--word-type` requires at least one non-empty type");
    }
    Ok(values)
}

#[requires(true)]
#[ensures(true)]
fn is_valid_vlacku_word_type_filter(value: WordTypeFilter) -> bool {
    matches!(
        value,
        WordTypeFilter::Gismu
            | WordTypeFilter::Lujvo
            | WordTypeFilter::Cmavo
            | WordTypeFilter::Cmevla
            | WordTypeFilter::Fuivla
            | WordTypeFilter::Brivla
    )
}

#[requires(true)]
#[ensures(true)]
fn cli_status_from_vlacku_outcome(outcome: VlackuOutcome) -> CliStatus {
    match outcome {
        VlackuOutcome::Found => CliStatus::Success,
        VlackuOutcome::ValidMissing => CliStatus::ValidMissing,
        VlackuOutcome::Invalid => CliStatus::InvalidInput,
    }
}

#[invariant(self.output_terminal_width.is_none_or(|width| width > 0))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VlackuRenderOptions {
    pub color: bool,
    pub glyphs: GlyphStyle,
    pub output_terminal_width: Option<usize>,
    pub sumti_places: CliSumtiPlaces,
    pub show_etymology: bool,
}

#[requires(true)]
#[ensures(!ret.is_empty())]
pub(crate) fn render_vlacku_output(
    output: &VlackuSearchOutput,
    color: bool,
    glyphs: GlyphStyle,
) -> String {
    render_vlacku_output_with_options(
        output,
        new!(VlackuRenderOptions {
            color,
            glyphs,
            output_terminal_width: None,
            sumti_places: CliSumtiPlaces::Index,
            show_etymology: false,
        }),
    )
}

#[requires(output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(!ret.is_empty())]
pub(crate) fn render_vlacku_output_with_width(
    output: &VlackuSearchOutput,
    color: bool,
    glyphs: GlyphStyle,
    output_terminal_width: Option<usize>,
) -> String {
    render_vlacku_output_with_options(
        output,
        new!(VlackuRenderOptions {
            color,
            glyphs,
            output_terminal_width,
            sumti_places: CliSumtiPlaces::Index,
            show_etymology: false,
        }),
    )
}

#[requires(options.output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(scope.is_exceeded() || !ret.is_empty())]
pub(crate) fn render_vlacku_output_in_scope(
    output: &VlackuSearchOutput,
    options: VlackuRenderOptions,
    scope: &jbotci_output::budget::TextScope,
) -> String {
    if output.cards.is_empty() {
        return scope.copy("No matches found.\n");
    }
    let mut rendered = scope.buffer();
    for (index, card) in output.cards.iter().enumerate() {
        if scope.is_exceeded() {
            break;
        }
        rendered.push_str(&render_vlacku_card(index + 1, card, &options, scope));
        rendered.push('\n');
    }
    rendered.into_piece()
}

#[requires(true)]
#[ensures(true)]
pub(crate) fn render_dictionary_definitions_with_limits(
    words: &[WordLike],
    color: bool,
    glyphs: GlyphStyle,
    limits: Option<&ToolExecutionLimits>,
) -> Result<String> {
    let count = limits.map_or(std::num::NonZeroUsize::MAX, |limits| limits.entries);
    let cards = jbotci_search::vlacku::dictionary_cards_for_word_likes_with_limit(
        jbotci_dictionary_data::english(),
        words,
        count,
    )?;
    if cards.is_empty() {
        return Ok(String::new());
    }
    render_vlacku_output_with_limit(
        &VlackuSearchOutput {
            cards,
            outcome: VlackuOutcome::Found,
            diagnostics: Vec::new(),
        },
        new!(VlackuRenderOptions {
            color,
            glyphs,
            output_terminal_width: None,
            sumti_places: CliSumtiPlaces::Index,
            show_etymology: false
        }),
        tool_render_limits(limits).text,
    )
}

#[requires(index > 0)]
#[requires(options.output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(scope.is_exceeded() || !ret.is_empty())]
fn render_vlacku_card(
    index: usize,
    card: &VlackuCard,
    options: &VlackuRenderOptions,
    scope: &jbotci_output::budget::TextScope,
) -> String {
    let place_map = DefinitionPlaceMap::from_definition(&card.definition);
    let mut lines = scope.buffer();
    let mut header = scope.buffer();
    header.push_str(&dark_in_scope(
        &scope.format(format_args!("{index}.")),
        options.color,
        &scope,
    ));
    header.push(' ');
    header.push_str(&yellow_underlined_in_scope(
        &card.word,
        options.color,
        &scope,
    ));
    if let Some(author) = &card.author {
        header.push_str(&dark_in_scope(" | ", options.color, &scope));
        header.push_str(&dark_in_scope("by: ", options.color, &scope));
        header.push_str(&author.username);
    }
    header.push_str(&dark_in_scope(" | ", options.color, &scope));
    header.push_str(&blue_in_scope(
        &vlacku_header_type(card, &scope),
        options.color,
        &scope,
    ));
    if let Some(similarity) = card.similarity {
        header.push_str(&dark_in_scope(" | ", options.color, &scope));
        header.push_str(&dark_in_scope("similarity: ", options.color, &scope));
        header.push_str(&magenta_in_scope(
            &format_similarity_percent(similarity),
            options.color,
            &scope,
        ));
    }
    if let Some(votes) = card.votes {
        header.push_str(&dark_in_scope(" | ", options.color, &scope));
        header.push_str(&dark_in_scope("votes: ", options.color, &scope));
        header.push_str(&green_in_scope(
            &format_vlacku_votes(votes, card.is_official, options.glyphs),
            options.color,
            &scope,
        ));
    }
    {
        lines.push_str(&(header));
        lines.push('\n');
    };

    if !card.rafsi.is_empty() {
        {
            lines.push_str(
                &(scope.format(format_args!(
                    "  {}{}",
                    dark_in_scope("rafsi: ", options.color, &scope),
                    scope.join(
                        card.rafsi
                            .iter()
                            .map(|rafsi| red_in_scope(rafsi, options.color, scope)),
                        " "
                    )
                ))),
            );
            lines.push('\n');
        };
    }
    if !card.decomposition.is_empty() {
        {
            lines.push_str(
                &(scope.format(format_args!(
                    "  {}{}",
                    dark_in_scope("decomposition: ", options.color, &scope),
                    render_vlacku_decomposition(
                        &card.decomposition,
                        options.color,
                        options.glyphs,
                        &scope
                    )
                ))),
            );
            lines.push('\n');
        };
    }
    if !card.glosses.is_empty() {
        {
            lines.push_str(
                &(scope.format(format_args!(
                    "  {}",
                    dark_in_scope("glosses:", options.color, &scope)
                ))),
            );
            lines.push('\n');
        };
        let glosses = scope.join(&card.glosses, "; ");
        let rendered =
            vlacku_reference_text_for_sumti_places(&glosses, &place_map, options, &scope);
        push_rendered_vlacku_detail_lines(&mut lines, &rendered, options);
    }
    if !card.definition.trim().is_empty() {
        {
            lines.push_str(
                &(scope.format(format_args!(
                    "  {}",
                    dark_in_scope("definitions:", options.color, &scope)
                ))),
            );
            lines.push('\n');
        };
        for line in card.definition.lines() {
            let rendered =
                vlacku_definition_text_for_sumti_places(line, &place_map, options, &scope);
            push_rendered_vlacku_detail_lines(&mut lines, &rendered, options);
        }
    } else if !card.known {
        {
            lines.push_str(
                &(scope.format(format_args!(
                    "  {}",
                    dark_in_scope("definitions:", options.color, &scope)
                ))),
            );
            lines.push('\n');
        };
        push_rendered_vlacku_detail_lines(&mut lines, "not in dictionary", options);
    }
    if !card.notes.trim().is_empty() {
        {
            lines.push_str(
                &(scope.format(format_args!(
                    "  {}",
                    dark_in_scope("notes:", options.color, &scope)
                ))),
            );
            lines.push('\n');
        };
        for line in card.notes.lines() {
            let rendered =
                vlacku_reference_text_for_sumti_places(line, &place_map, options, &scope);
            push_rendered_vlacku_detail_lines(&mut lines, &rendered, options);
        }
    }
    if options.show_etymology {
        if let Some(etymology) = card
            .etymology
            .as_deref()
            .filter(|etymology| !etymology.trim().is_empty())
        {
            {
                lines.push_str(
                    &(scope.format(format_args!(
                        "  {}",
                        dark_in_scope("etymology:", options.color, &scope)
                    ))),
                );
                lines.push('\n');
            };
            for line in etymology.lines() {
                let rendered =
                    vlacku_reference_text_for_sumti_places(line, &place_map, options, &scope);
                push_rendered_vlacku_detail_lines(&mut lines, &rendered, options);
            }
        }
    }
    lines.into_piece()
}

#[requires(options.output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(true)]
fn push_rendered_vlacku_detail_lines(
    lines: &mut jbotci_output::budget::TextBuffer,
    rendered_text: &str,
    options: &VlackuRenderOptions,
) {
    let scope = lines.scope();
    for line in wrap_vlacku_detail_line(rendered_text, options.output_terminal_width, &scope) {
        {
            lines.push_str(
                &(scope.format(format_args!(
                    "{VLACKU_DETAIL_INDENT}{}",
                    render_vlacku_rich_text(&line, options, &scope)
                ))),
            );
            lines.push('\n');
        };
    }
}

#[requires(true)]
#[ensures(true)]
fn vlacku_definition_text_for_sumti_places(
    text: &str,
    place_map: &DefinitionPlaceMap,
    options: &VlackuRenderOptions,
    scope: &jbotci_output::budget::TextScope,
) -> String {
    match options.sumti_places {
        CliSumtiPlaces::Raw => scope.copy(text),
        CliSumtiPlaces::Index => {
            jbotci_output::format_definition_line_with_indexed_places_in_scope(
                text,
                place_map,
                options.glyphs,
                scope,
            )
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn vlacku_reference_text_for_sumti_places(
    text: &str,
    place_map: &DefinitionPlaceMap,
    options: &VlackuRenderOptions,
    scope: &jbotci_output::budget::TextScope,
) -> String {
    match options.sumti_places {
        CliSumtiPlaces::Raw => scope.copy(text),
        CliSumtiPlaces::Index => jbotci_output::format_notes_line_with_indexed_places_in_scope(
            text,
            place_map,
            options.glyphs,
            scope,
        ),
    }
}

#[requires(output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(scope.is_exceeded() || !ret.is_empty())]
fn wrap_vlacku_detail_line(
    text: &str,
    output_terminal_width: Option<usize>,
    scope: &jbotci_output::budget::TextScope,
) -> Vec<String> {
    let Some(output_terminal_width) = output_terminal_width else {
        return vec![scope.copy(text)];
    };
    let wrap_width = output_terminal_width
        .saturating_sub(UnicodeWidthStr::width(VLACKU_DETAIL_INDENT))
        .max(1);
    if UnicodeWidthStr::width(text) <= wrap_width {
        return vec![scope.copy(text)];
    }
    let atoms = vlacku_wrap_atoms(text, &scope);
    if atoms.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = scope.buffer();
    let mut current_width = 0;
    for atom in atoms {
        if scope.is_exceeded() {
            break;
        }
        let atom_width = UnicodeWidthStr::width(atom.as_str());
        if current.is_empty() {
            current_width = atom_width;
            current = scope.buffer_with(&atom);
        } else if current_width + 1 + atom_width <= wrap_width {
            current.push(' ');
            current.push_str(&atom);
            current_width += 1 + atom_width;
        } else {
            lines.push(current.into_piece());
            current_width = atom_width;
            current = scope.buffer_with(&atom);
        }
    }
    if !current.is_empty() {
        lines.push(current.into_piece());
    }
    lines
}

#[requires(true)]
#[ensures(input.trim().is_empty() -> ret.is_empty())]
fn vlacku_wrap_atoms(input: &str, scope: &jbotci_output::budget::TextScope) -> Vec<String> {
    let mut atoms = Vec::new();
    let mut remaining = input.trim();
    while !remaining.is_empty() && !scope.is_exceeded() {
        if let Some(after_open) = remaining.strip_prefix('$') {
            if let Some(close_index) = after_open.find('$') {
                let mut atom_end = close_index + 2;
                let trailing_text = &remaining[atom_end..];
                let trailing_end = trailing_text
                    .find(char::is_whitespace)
                    .unwrap_or(trailing_text.len());
                atom_end += trailing_end;
                atoms.push(scope.copy(&remaining[..atom_end]));
                remaining = remaining[atom_end..].trim_start();
                continue;
            }
        }
        let atom_end = remaining
            .find(char::is_whitespace)
            .unwrap_or(remaining.len());
        atoms.push(scope.copy(&remaining[..atom_end]));
        remaining = remaining[atom_end..].trim_start();
    }
    atoms
}

#[requires(true)]
#[ensures(scope.is_exceeded() || !ret.is_empty())]
fn vlacku_header_type(card: &VlackuCard, scope: &jbotci_output::budget::TextScope) -> String {
    let normalized = normalize_word_type_filter(&card.word_type);
    if normalized.starts_with("cmavo") {
        if let Some(selmaho) = &card.selmaho {
            if !selmaho.trim().is_empty() {
                return scope.format(format_args!("cmavo: {selmaho}"));
            }
        }
    }
    scope.copy(&card.word_type)
}

#[requires(true)]
#[ensures(ret.ends_with('%'))]
fn format_similarity_percent(value: f32) -> String {
    format!("{}%", (value * 100.0).round() as i32)
}

#[requires(true)]
#[ensures(glyphs == GlyphStyle::Ascii && is_official -> ret == "official")]
fn format_vlacku_votes(value: i32, is_official: bool, glyphs: GlyphStyle) -> String {
    if glyphs == GlyphStyle::Ascii && is_official {
        "official".to_owned()
    } else {
        format_vote_display(value, is_official)
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_decomposition(
    pieces: &[VlackuCompositionPiece],
    color: bool,
    glyphs: GlyphStyle,
    scope: &jbotci_output::budget::TextScope,
) -> String {
    let separator = dark_in_scope(lujvo_separator(glyphs), color, &scope);
    scope.join(
        pieces
            .iter()
            .map(|piece| render_vlacku_decomposition_piece(piece, color, glyphs, scope)),
        &separator,
    )
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_decomposition_piece(
    piece: &VlackuCompositionPiece,
    color: bool,
    glyphs: GlyphStyle,
    scope: &jbotci_output::budget::TextScope,
) -> String {
    let phoneme_options = phoneme_render_options(None, None, glyphs);
    let surface = Phonemes::from_canonical(piece.surface.clone())
        .map(|phonemes| phonemes.render(phoneme_options))
        .unwrap_or_else(|_| piece.surface.clone());
    match piece.kind {
        VlackuCompositionKind::Rafsi => red_in_scope(&surface, color, &scope),
        VlackuCompositionKind::Hyphen => dark_in_scope(&surface, color, &scope),
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn lujvo_separator(glyphs: GlyphStyle) -> &'static str {
    match glyphs {
        GlyphStyle::Unicode => "·",
        GlyphStyle::Ascii => "~",
    }
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_rich_text(
    input: &str,
    options: &VlackuRenderOptions,
    scope: &jbotci_output::budget::TextScope,
) -> String {
    let mut output = scope.buffer();
    let mut remaining = input;
    while !remaining.is_empty() && !scope.is_exceeded() {
        let Some(open_index) = remaining.find('$') else {
            output.push_str(&render_vlacku_word_links(remaining, options, &scope));
            break;
        };
        let before = &remaining[..open_index];
        let after_open = &remaining[open_index + 1..];
        let Some(close_index) = after_open.find('$') else {
            output.push_str(&render_vlacku_word_links(remaining, options, &scope));
            break;
        };
        output.push_str(&render_vlacku_word_links(before, options, &scope));
        let math_body = &after_open[..close_index];
        output.push_str(&render_vlacku_raw_place_span(
            math_body,
            options.color,
            &scope,
        ));
        remaining = &after_open[close_index + 1..];
    }
    output.into_piece()
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_raw_place_span(
    input: &str,
    color: bool,
    scope: &jbotci_output::budget::TextScope,
) -> String {
    let mut output = scope.buffer();
    output.push_str(&dark_in_scope("$", color, &scope));
    let mut remaining = input;
    while !remaining.is_empty() && !scope.is_exceeded() {
        let Some(equals_index) = remaining.find('=') else {
            output.push_str(&cyan_in_scope(remaining, color, &scope));
            break;
        };
        output.push_str(&cyan_in_scope(&remaining[..equals_index], color, &scope));
        output.push_str(&dark_in_scope("=", color, &scope));
        remaining = &remaining[equals_index + 1..];
    }
    output.push_str(&dark_in_scope("$", color, &scope));
    output.into_piece()
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_word_links(
    input: &str,
    options: &VlackuRenderOptions,
    scope: &jbotci_output::budget::TextScope,
) -> String {
    let mut output = scope.buffer();
    let mut remaining = input;
    while !remaining.is_empty() && !scope.is_exceeded() {
        let Some(open_index) = remaining.find('{') else {
            output.push_str(&render_vlacku_plain_or_indexed_places(
                remaining, options, &scope,
            ));
            break;
        };
        let before = &remaining[..open_index];
        let after_open = &remaining[open_index + 1..];
        let Some(close_index) = after_open.find('}') else {
            output.push_str(&render_vlacku_plain_or_indexed_places(
                remaining, options, &scope,
            ));
            break;
        };
        output.push_str(&render_vlacku_plain_or_indexed_places(
            before, options, &scope,
        ));
        let inside = &after_open[..close_index];
        let link_value = inside.trim();
        if is_vlacku_word_link(link_value) {
            output.push_str(&dark_in_scope("{", options.color, &scope));
            output.push_str(&yellow_in_scope(link_value, options.color, &scope));
            output.push_str(&dark_in_scope("}", options.color, &scope));
        } else {
            output.push_str(&light_in_scope(
                &scope.format(format_args!("{{{inside}}}")),
                options.color,
                &scope,
            ));
        }
        remaining = &after_open[close_index + 1..];
    }
    output.into_piece()
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_plain_or_indexed_places(
    input: &str,
    options: &VlackuRenderOptions,
    scope: &jbotci_output::budget::TextScope,
) -> String {
    if options.sumti_places == CliSumtiPlaces::Raw {
        return light_in_scope(input, options.color, &scope);
    }

    let mut output = scope.buffer();
    let mut remaining = input;
    while !remaining.is_empty() && !scope.is_exceeded() {
        let Some(open_index) = remaining.find(options.glyphs.slot_open()) else {
            output.push_str(&light_in_scope(remaining, options.color, &scope));
            break;
        };
        output.push_str(&light_in_scope(
            &remaining[..open_index],
            options.color,
            &scope,
        ));
        let after_open = &remaining[open_index + options.glyphs.slot_open().len()..];
        let Some(close_index) = after_open.find(options.glyphs.slot_close()) else {
            output.push_str(&light_in_scope(
                &remaining[open_index..],
                options.color,
                &scope,
            ));
            break;
        };
        let place_index = &after_open[..close_index];
        if !place_index.is_empty()
            && place_index
                .chars()
                .all(|character| character.is_ascii_digit())
        {
            output.push_str(&dark_in_scope(
                options.glyphs.slot_open(),
                options.color,
                &scope,
            ));
            output.push_str(&cyan_in_scope(place_index, options.color, &scope));
            output.push_str(&dark_in_scope(
                options.glyphs.slot_close(),
                options.color,
                &scope,
            ));
            remaining = &after_open[close_index + options.glyphs.slot_close().len()..];
        } else {
            output.push_str(&light_in_scope(
                options.glyphs.slot_open(),
                options.color,
                &scope,
            ));
            remaining = after_open;
        }
    }
    output.into_piece()
}

#[requires(true)]
#[ensures(true)]
fn is_vlacku_word_link(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_whitespace)
}

#[requires(options.output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(!ret.is_empty())]
pub(crate) fn render_vlacku_output_with_options(
    output: &VlackuSearchOutput,
    options: VlackuRenderOptions,
) -> String {
    render_vlacku_output_with_limit(
        output,
        options,
        ByteLimit::new(usize::MAX).expect("positive unlimited limit"),
    )
    .expect("unrestricted dictionary rendering")
}
#[requires(options.output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(ret.as_ref().is_ok_and(|text| text.len() <= limit.get()) || ret.is_err())]
fn render_vlacku_output_with_limit(
    output: &VlackuSearchOutput,
    options: VlackuRenderOptions,
    limit: ByteLimit,
) -> Result<String> {
    let scope = jbotci_output::budget::TextScope::new(limit);
    Ok(scope.finish(render_vlacku_output_in_scope(output, options, &scope))?)
}
