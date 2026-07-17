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
            render_vlacku_output_with_options(
                &output,
                new!(VlackuRenderOptions {
                    color,
                    glyphs,
                    output_terminal_width,
                    sumti_places: input.sumti_places,
                    show_etymology: input.show_etymology,
                }),
            )
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
    let query = query.trim().to_owned();
    if query.is_empty() {
        bail!("vlacku query text must be non-empty.");
    }
    let dictionary = jbotci_dictionary_data::english();
    let hits = if let Some(context) = tool_context {
        if let Some(service) = context.embedding_search()? {
            service
                .semantic_vlacku_hits(&query, dictionary.entries().len())
                .map_err(|error| anyhow!(error.to_string()))?
        } else {
            semantic_vlacku_hits_with_new_backend(&query, dictionary.entries().len())?
        }
    } else {
        semantic_vlacku_hits_with_new_backend(&query, dictionary.entries().len())?
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
fn semantic_vlacku_hits_with_new_backend(
    query: &str,
    count: usize,
) -> Result<Vec<jbotci_embeddings::DictionarySemanticHit>> {
    let index_root = default_index_root().map_err(|error| anyhow!(error.to_string()))?;
    let mut backend = load_backend_for_search(DEFAULT_MODEL_KEY, None)
        .map_err(|error| anyhow!(error.to_string()))?;
    semantic_vlacku_hits(&mut backend, query, count, &index_root, DEFAULT_MODEL_KEY)
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
#[ensures(!ret.is_empty())]
pub(crate) fn render_vlacku_output_with_options(
    output: &VlackuSearchOutput,
    options: VlackuRenderOptions,
) -> String {
    if output.cards.is_empty() {
        return "No matches found.\n".to_owned();
    }
    let mut rendered = String::new();
    for (index, card) in output.cards.iter().enumerate() {
        rendered.push_str(&render_vlacku_card(index + 1, card, &options));
        rendered.push('\n');
    }
    rendered
}

#[requires(true)]
#[ensures(words.is_empty() -> ret.is_empty())]
pub(crate) fn render_dictionary_definitions_for_word_likes(
    words: &[WordLike],
    color: bool,
    glyphs: GlyphStyle,
) -> String {
    let cards = dictionary_cards_for_word_likes(jbotci_dictionary_data::english(), words);
    if cards.is_empty() {
        return String::new();
    }
    render_vlacku_output_with_options(
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
            show_etymology: false,
        }),
    )
}

#[requires(index > 0)]
#[requires(options.output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(!ret.is_empty())]
fn render_vlacku_card(index: usize, card: &VlackuCard, options: &VlackuRenderOptions) -> String {
    let place_map = DefinitionPlaceMap::from_definition(&card.definition);
    let mut lines = Vec::new();
    let mut header = String::new();
    header.push_str(&dark(&format!("{index}."), options.color));
    header.push(' ');
    header.push_str(&yellow_underlined(&card.word, options.color));
    if let Some(author) = &card.author {
        header.push_str(&dark(" | ", options.color));
        header.push_str(&dark("by: ", options.color));
        header.push_str(&author.username);
    }
    header.push_str(&dark(" | ", options.color));
    header.push_str(&blue(&vlacku_header_type(card), options.color));
    if let Some(similarity) = card.similarity {
        header.push_str(&dark(" | ", options.color));
        header.push_str(&dark("similarity: ", options.color));
        header.push_str(&magenta(
            &format_similarity_percent(similarity),
            options.color,
        ));
    }
    if let Some(votes) = card.votes {
        header.push_str(&dark(" | ", options.color));
        header.push_str(&dark("votes: ", options.color));
        header.push_str(&green(
            &format_vlacku_votes(votes, card.is_official, options.glyphs),
            options.color,
        ));
    }
    lines.push(header);

    if !card.rafsi.is_empty() {
        lines.push(format!(
            "  {}{}",
            dark("rafsi: ", options.color),
            card.rafsi
                .iter()
                .map(|rafsi| red(rafsi, options.color))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    if !card.decomposition.is_empty() {
        lines.push(format!(
            "  {}{}",
            dark("decomposition: ", options.color),
            render_vlacku_decomposition(&card.decomposition, options.color, options.glyphs)
        ));
    }
    if !card.glosses.is_empty() {
        lines.push(format!("  {}", dark("glosses:", options.color)));
        let glosses = card.glosses.join("; ");
        let rendered = vlacku_reference_text_for_sumti_places(&glosses, &place_map, options);
        push_rendered_vlacku_detail_lines(&mut lines, &rendered, options);
    }
    if !card.definition.trim().is_empty() {
        lines.push(format!("  {}", dark("definitions:", options.color)));
        for line in card.definition.lines() {
            let rendered = vlacku_definition_text_for_sumti_places(line, &place_map, options);
            push_rendered_vlacku_detail_lines(&mut lines, &rendered, options);
        }
    }
    if !card.notes.trim().is_empty() {
        lines.push(format!("  {}", dark("notes:", options.color)));
        for line in card.notes.lines() {
            let rendered = vlacku_reference_text_for_sumti_places(line, &place_map, options);
            push_rendered_vlacku_detail_lines(&mut lines, &rendered, options);
        }
    }
    if options.show_etymology {
        if let Some(etymology) = card
            .etymology
            .as_deref()
            .filter(|etymology| !etymology.trim().is_empty())
        {
            lines.push(format!("  {}", dark("etymology:", options.color)));
            for line in etymology.lines() {
                let rendered = vlacku_reference_text_for_sumti_places(line, &place_map, options);
                push_rendered_vlacku_detail_lines(&mut lines, &rendered, options);
            }
        }
    }
    lines.join("\n") + "\n"
}

#[requires(options.output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(true)]
fn push_rendered_vlacku_detail_lines(
    lines: &mut Vec<String>,
    rendered_text: &str,
    options: &VlackuRenderOptions,
) {
    for line in wrap_vlacku_detail_line(rendered_text, options.output_terminal_width) {
        lines.push(format!(
            "{VLACKU_DETAIL_INDENT}{}",
            render_vlacku_rich_text(&line, options)
        ));
    }
}

#[requires(true)]
#[ensures(true)]
fn vlacku_definition_text_for_sumti_places(
    text: &str,
    place_map: &DefinitionPlaceMap,
    options: &VlackuRenderOptions,
) -> String {
    match options.sumti_places {
        CliSumtiPlaces::Raw => text.to_owned(),
        CliSumtiPlaces::Index => {
            format_definition_line_with_indexed_places(text, place_map, options.glyphs)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn vlacku_reference_text_for_sumti_places(
    text: &str,
    place_map: &DefinitionPlaceMap,
    options: &VlackuRenderOptions,
) -> String {
    match options.sumti_places {
        CliSumtiPlaces::Raw => text.to_owned(),
        CliSumtiPlaces::Index => {
            format_notes_line_with_indexed_places(text, place_map, options.glyphs)
        }
    }
}

#[requires(output_terminal_width.is_none_or(|width| width > 0))]
#[ensures(!ret.is_empty())]
fn wrap_vlacku_detail_line(text: &str, output_terminal_width: Option<usize>) -> Vec<String> {
    let Some(output_terminal_width) = output_terminal_width else {
        return vec![text.to_owned()];
    };
    let wrap_width = output_terminal_width
        .saturating_sub(UnicodeWidthStr::width(VLACKU_DETAIL_INDENT))
        .max(1);
    if UnicodeWidthStr::width(text) <= wrap_width {
        return vec![text.to_owned()];
    }
    let atoms = vlacku_wrap_atoms(text);
    if atoms.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;
    for atom in atoms {
        let atom_width = UnicodeWidthStr::width(atom.as_str());
        if current.is_empty() {
            current_width = atom_width;
            current = atom;
        } else if current_width + 1 + atom_width <= wrap_width {
            current.push(' ');
            current.push_str(&atom);
            current_width += 1 + atom_width;
        } else {
            lines.push(current);
            current_width = atom_width;
            current = atom;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[requires(true)]
#[ensures(input.trim().is_empty() -> ret.is_empty())]
fn vlacku_wrap_atoms(input: &str) -> Vec<String> {
    let mut atoms = Vec::new();
    let mut remaining = input.trim();
    while !remaining.is_empty() {
        if let Some(after_open) = remaining.strip_prefix('$') {
            if let Some(close_index) = after_open.find('$') {
                let mut atom_end = close_index + 2;
                let trailing_text = &remaining[atom_end..];
                let trailing_end = trailing_text
                    .find(char::is_whitespace)
                    .unwrap_or(trailing_text.len());
                atom_end += trailing_end;
                atoms.push(remaining[..atom_end].to_owned());
                remaining = remaining[atom_end..].trim_start();
                continue;
            }
        }
        let atom_end = remaining
            .find(char::is_whitespace)
            .unwrap_or(remaining.len());
        atoms.push(remaining[..atom_end].to_owned());
        remaining = remaining[atom_end..].trim_start();
    }
    atoms
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn vlacku_header_type(card: &VlackuCard) -> String {
    let normalized = normalize_word_type_filter(&card.word_type);
    if normalized.starts_with("cmavo") {
        if let Some(selmaho) = &card.selmaho {
            if !selmaho.trim().is_empty() {
                return format!("cmavo: {selmaho}");
            }
        }
    }
    card.word_type.clone()
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
) -> String {
    let separator = dark(lujvo_separator(glyphs), color);
    pieces
        .iter()
        .map(|piece| render_vlacku_decomposition_piece(piece, color, glyphs))
        .collect::<Vec<_>>()
        .join(&separator)
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_decomposition_piece(
    piece: &VlackuCompositionPiece,
    color: bool,
    glyphs: GlyphStyle,
) -> String {
    let phoneme_options = phoneme_render_options(None, None, glyphs);
    let surface = Phonemes::from_canonical(piece.surface.clone())
        .map(|phonemes| phonemes.render(phoneme_options))
        .unwrap_or_else(|_| piece.surface.clone());
    match piece.kind {
        VlackuCompositionKind::Rafsi => red(&surface, color),
        VlackuCompositionKind::Hyphen => dark(&surface, color),
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
fn render_vlacku_rich_text(input: &str, options: &VlackuRenderOptions) -> String {
    let mut output = String::new();
    let mut remaining = input;
    while !remaining.is_empty() {
        let Some(open_index) = remaining.find('$') else {
            output.push_str(&render_vlacku_word_links(remaining, options));
            break;
        };
        let before = &remaining[..open_index];
        let after_open = &remaining[open_index + 1..];
        let Some(close_index) = after_open.find('$') else {
            output.push_str(&render_vlacku_word_links(remaining, options));
            break;
        };
        output.push_str(&render_vlacku_word_links(before, options));
        let math_body = &after_open[..close_index];
        output.push_str(&render_vlacku_raw_place_span(math_body, options.color));
        remaining = &after_open[close_index + 1..];
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_raw_place_span(input: &str, color: bool) -> String {
    let mut output = String::new();
    output.push_str(&dark("$", color));
    let mut remaining = input;
    while !remaining.is_empty() {
        let Some(equals_index) = remaining.find('=') else {
            output.push_str(&cyan(remaining, color));
            break;
        };
        output.push_str(&cyan(&remaining[..equals_index], color));
        output.push_str(&dark("=", color));
        remaining = &remaining[equals_index + 1..];
    }
    output.push_str(&dark("$", color));
    output
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_word_links(input: &str, options: &VlackuRenderOptions) -> String {
    let mut output = String::new();
    let mut remaining = input;
    while !remaining.is_empty() {
        let Some(open_index) = remaining.find('{') else {
            output.push_str(&render_vlacku_plain_or_indexed_places(remaining, options));
            break;
        };
        let before = &remaining[..open_index];
        let after_open = &remaining[open_index + 1..];
        let Some(close_index) = after_open.find('}') else {
            output.push_str(&render_vlacku_plain_or_indexed_places(remaining, options));
            break;
        };
        output.push_str(&render_vlacku_plain_or_indexed_places(before, options));
        let inside = &after_open[..close_index];
        let link_value = inside.trim();
        if is_vlacku_word_link(link_value) {
            output.push_str(&dark("{", options.color));
            output.push_str(&yellow(link_value, options.color));
            output.push_str(&dark("}", options.color));
        } else {
            output.push_str(&light(&format!("{{{inside}}}"), options.color));
        }
        remaining = &after_open[close_index + 1..];
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn render_vlacku_plain_or_indexed_places(input: &str, options: &VlackuRenderOptions) -> String {
    if options.sumti_places == CliSumtiPlaces::Raw {
        return light(input, options.color);
    }

    let mut output = String::new();
    let mut remaining = input;
    while !remaining.is_empty() {
        let Some(open_index) = remaining.find(options.glyphs.slot_open()) else {
            output.push_str(&light(remaining, options.color));
            break;
        };
        output.push_str(&light(&remaining[..open_index], options.color));
        let after_open = &remaining[open_index + options.glyphs.slot_open().len()..];
        let Some(close_index) = after_open.find(options.glyphs.slot_close()) else {
            output.push_str(&light(&remaining[open_index..], options.color));
            break;
        };
        let place_index = &after_open[..close_index];
        if !place_index.is_empty()
            && place_index
                .chars()
                .all(|character| character.is_ascii_digit())
        {
            output.push_str(&dark(options.glyphs.slot_open(), options.color));
            output.push_str(&cyan(place_index, options.color));
            output.push_str(&dark(options.glyphs.slot_close(), options.color));
            remaining = &after_open[close_index + options.glyphs.slot_close().len()..];
        } else {
            output.push_str(&light(options.glyphs.slot_open(), options.color));
            remaining = after_open;
        }
    }
    output
}

#[requires(true)]
#[ensures(true)]
fn is_vlacku_word_link(value: &str) -> bool {
    !value.is_empty() && !value.chars().any(char::is_whitespace)
}
