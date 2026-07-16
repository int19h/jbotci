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
    )?;
    stderr.write_all(rendered.stderr.as_bytes())?;
    stdout.write_all(&rendered.stdout)?;
    Ok(rendered.status)
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
) -> Result<TersmuRendered> {
    validate_tersmu_options(&input)?;
    let morphology_trace_options =
        trace_options(&input.trace, TracePhase::Syntax, DEFAULT_TRACE_LIMIT)?;
    let syntax_trace_options =
        trace_options(&input.trace, TracePhase::Syntax, DEFAULT_TRACE_LIMIT)?;
    let source_label = input_source_label(input.file.as_ref(), input.text.is_empty());
    let text = input.read_text_with_stdin(stdin_text)?;
    let dialect = input.dialect_definition()?;
    let morphology_options = MorphologyOptions::default()
        .with_dialect_definition(&dialect)
        .with_max_recovery_errors(input.max_errors.get())
        .with_trace_options(morphology_trace_options);
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
        diagnostics.extend(
            morphology
                .errors
                .iter()
                .map(|error| error.to_diagnostic(Some(SourceId(source_label.clone())), &text)),
        );
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
        }));
    }
    let words = morphology.words;
    let parse_options = ParseOptions::default()
        .with_dialect_definition(&dialect)
        .with_trace_options(syntax_trace_options)
        .with_max_recovery_errors(input.max_errors.get());
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
            }));
        }
    };
    let mut stdout = String::new();
    if input.show_defs {
        stdout.push_str(&render_dictionary_definitions_for_word_likes(
            words.as_slice(),
            color_policy.stdout,
            glyphs,
        ));
    }
    let rendered = match input.format {
        TersmuFormat::Json => json_string_with_options(
            &graph,
            JsonRenderOptions {
                indent: input.indent.unwrap_or(0),
                color: color_policy.stdout,
                ..JsonRenderOptions::default()
            },
        )?,
        TersmuFormat::Tree => render_tree(&graph),
        TersmuFormat::TreeProj => render_tree_proj(&graph),
    };
    stdout.push_str(&rendered);
    stdout.push('\n');
    Ok(new!(TersmuRendered {
        status: CliStatus::Success,
        stdout: stdout.into_bytes(),
        stderr,
    }))
}
