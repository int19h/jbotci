use super::super::*;

#[requires(diagnostic_terminal_width > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn run_tersmu<WOut: Write, WErr: Write>(
    input: TersmuInput,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_detail: DiagnosticDetailMode,
    glyphs: GlyphStyle,
    diagnostic_terminal_width: usize,
    stdin_text: Option<&str>,
) -> Result<CliStatus> {
    let rendered = render_tersmu(
        input,
        color_policy,
        diagnostic_detail,
        glyphs,
        diagnostic_terminal_width,
        stdin_text,
        false,
    )?;
    stderr.write_all(rendered.stderr.as_bytes())?;
    stdout.write_all(&rendered.stdout)?;
    Ok(rendered.status)
}

/// The tool-facing tersmu path (jbotci#723): identical rendering, plus the
/// declared compact-representation incompatibility records of the candidate's
/// semantic graph, each in its exact `<INCOMPATIBILITY .../>` declaration
/// form. Records exist only for candidates that reach a semantic graph.
#[requires(diagnostic_terminal_width > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn run_tersmu_with_incompatibilities<WOut: Write, WErr: Write>(
    input: TersmuInput,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_detail: DiagnosticDetailMode,
    glyphs: GlyphStyle,
    diagnostic_terminal_width: usize,
    stdin_text: Option<&str>,
) -> Result<(CliStatus, Vec<String>)> {
    let rendered = render_tersmu(
        input,
        color_policy,
        diagnostic_detail,
        glyphs,
        diagnostic_terminal_width,
        stdin_text,
        true,
    )?;
    stderr.write_all(rendered.stderr.as_bytes())?;
    stdout.write_all(&rendered.stdout)?;
    let data!(TersmuRendered {
        status,
        compact_incompatibilities,
        ..
    }) = rendered.into_data();
    Ok((status, compact_incompatibilities))
}

#[requires(diagnostic_terminal_width > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn render_tersmu(
    input: TersmuInput,
    color_policy: CliColorPolicy,
    diagnostic_detail: DiagnosticDetailMode,
    glyphs: GlyphStyle,
    diagnostic_terminal_width: usize,
    stdin_text: Option<&str>,
    collect_incompatibilities: bool,
) -> Result<TersmuRendered> {
    validate_tersmu_options(&input)?;
    let morphology_trace_options =
        trace_options(&input.trace, TracePhase::Syntax, DEFAULT_TRACE_LIMIT)?;
    let syntax_trace_options =
        trace_options(&input.trace, TracePhase::Syntax, DEFAULT_TRACE_LIMIT)?;
    let source_label = input_source_label(input.file.as_ref(), input.text.is_empty());
    let text = input.read_text_with_stdin(stdin_text)?;
    let dialect = input.dialect_definition()?;
    let mut morphology_options = MorphologyOptions::default()
        .with_dialect_definition(&dialect)
        .with_trace_options(morphology_trace_options);
    if let Some(max_errors) = input.max_errors {
        morphology_options = morphology_options.with_max_recovery_errors(max_errors.get());
    }
    let morphology_attempt =
        segment_words_with_modifiers_recovered_with_options_and_source_id_attempt(
            &text,
            &morphology_options,
            Some(SourceId(source_label.clone())),
        );
    let morphology_attempt = morphology_attempt.into_data();
    let morphology_trace_stderr =
        render_cli_trace(morphology_attempt.trace.as_ref(), color_policy.stderr);
    let morphology = morphology_attempt.result.into_data();
    let morphology_diagnostics = morphology_warning_diagnostics(
        &morphology.warnings,
        Some(SourceId(source_label.clone())),
        &text,
    );
    if !morphology.errors.is_empty() {
        let mut diagnostics = morphology_diagnostics;
        diagnostics.extend(morphology.errors.iter().map(|error| {
            error
                .to_diagnostic(Some(SourceId(source_label.clone())), &text)
                .expect("morphology error offsets belong to the parser source")
        }));
        let mut stderr = morphology_trace_stderr;
        stderr.push_str(&render_source_diagnostics(
            &source_label,
            &text,
            &diagnostics,
            color_policy.stderr,
            diagnostic_detail,
            glyphs,
            diagnostic_terminal_width,
        )?);
        return Ok(new!(TersmuRendered {
            status: CliStatus::Failure,
            stdout: Vec::new(),
            stderr,
            compact_incompatibilities: Vec::new(),
        }));
    }
    let words = morphology.words;
    let mut parse_options = ParseOptions::default()
        .with_dialect_definition(&dialect)
        .with_trace_options(syntax_trace_options);
    if let Some(max_errors) = input.max_errors {
        parse_options = parse_options.with_max_recovery_errors(max_errors.get());
    }
    let parsed = parse_syntax_tree_with_recovery_with_source_and_options_attempt(
        &words,
        &text,
        &parse_options,
    );
    let trace_stderr = render_cli_trace(parsed.trace.as_ref(), color_policy.stderr);
    let parsed =
        match parsed.result.into_data() {
            data!(SyntaxRecoveryParse::Valid { parse }) => parse.into_data(),
            data!(SyntaxRecoveryParse::Recovered { parse }) => {
                let parsed = parse.into_data();
                let mut diagnostics = morphology_diagnostics;
                diagnostics.extend(
                    parsed.errors.iter().map(|error| {
                        error.to_diagnostic(Some(SourceId(source_label.clone())), &text)
                    }),
                );
                diagnostics.extend(parsed.warnings.iter().map(|warning| {
                    warning.to_diagnostic(Some(SourceId(source_label.clone())), &text)
                }));
                let mut stderr = morphology_trace_stderr;
                stderr.push_str(&trace_stderr);
                stderr.push_str(&render_source_diagnostics(
                    &source_label,
                    &text,
                    &diagnostics,
                    color_policy.stderr,
                    diagnostic_detail,
                    glyphs,
                    diagnostic_terminal_width,
                )?);
                return Ok(new!(TersmuRendered {
                    status: CliStatus::Failure,
                    stdout: Vec::new(),
                    stderr,
                    compact_incompatibilities: Vec::new(),
                }));
            }
        };
    let mut diagnostics = morphology_diagnostics;
    diagnostics.extend(
        parsed
            .warnings
            .iter()
            .map(|warning| warning.to_diagnostic(Some(SourceId(source_label.clone())), &text)),
    );
    let mut stderr = morphology_trace_stderr;
    stderr.push_str(&trace_stderr);
    stderr.push_str(&render_source_diagnostics(
        &source_label,
        &text,
        &diagnostics,
        color_policy.stderr,
        diagnostic_detail,
        glyphs,
        diagnostic_terminal_width,
    )?);
    let graph = match build_generated_semantic_graph_with_dictionary_and_options(
        &parsed.parse_tree,
        SemanticBuildOptions {
            source_text: Some(&text),
            story_time: input.story_time,
        },
        jbotci_dictionary_data::english(),
    ) {
        Ok(graph) => graph,
        Err(error) => {
            stderr.push_str(&format!("semantic error: {error}\n"));
            return Ok(new!(TersmuRendered {
                status: CliStatus::Failure,
                stdout: Vec::new(),
                stderr,
                compact_incompatibilities: Vec::new(),
            }));
        }
    };
    let mut stdout = String::new();
    // Both human-readable formats keep structured cards inside their single
    // parseable document; JSON remains the pure interchange graph.
    let word_cards = if input.show_defs && input.format != TersmuFormat::Json {
        jbotci_semantics::notation::word_cards::build_word_cards(
            jbotci_dictionary_data::english(),
            words.as_slice(),
        )
    } else {
        Vec::new()
    };
    let compact_incompatibilities = if collect_incompatibilities {
        jbotci_semantics::analyze_compact_incompatibilities(&graph, !word_cards.is_empty())
            .iter()
            .map(jbotci_semantics::CompactIncompatibility::declaration)
            .collect()
    } else {
        Vec::new()
    };
    let rendered = match input.format {
        TersmuFormat::Json => json_string_with_options(
            &graph,
            JsonRenderOptions {
                indent: input.indent.unwrap_or(0),
                color: color_policy.stdout,
                ..JsonRenderOptions::default()
            },
        )?,
        TersmuFormat::Smusni => {
            let mut rendered = jbotci_semantics::render_smusni_with_word_cards(&graph, &word_cards);
            if rendered.ends_with('\n') {
                rendered.pop();
            }
            rendered
        }
        TersmuFormat::Xml => {
            // The command's input label is the document identity used by the
            // canonical XML root. As with smusni, normalize the renderer's
            // trailing newline before the shared output terminator below. An
            // empty card list renders exactly like `render_xml`, so the
            // show-defs distinction is already folded into `word_cards`.
            let mut rendered = render_xml_with_word_cards(&graph, &source_label, &word_cards)
                .into_data()
                .output;
            if rendered.ends_with('\n') {
                rendered.pop();
            }
            rendered
        }
    };
    stdout.push_str(&rendered);
    stdout.push('\n');
    Ok(new!(TersmuRendered {
        status: CliStatus::Success,
        stdout: stdout.into_bytes(),
        stderr,
        compact_incompatibilities,
    }))
}
