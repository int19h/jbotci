use super::super::*;

#[allow(clippy::too_many_arguments)]
#[requires(diagnostic_terminal_width > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn run_vlasei<WOut: Write, WErr: Write>(
    mut input: VlaseiInput,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_terminal_width: usize,
    stdin_text: Option<&str>,
) -> Result<CliStatus> {
    let glyphs = cli_glyph_style(input.ascii);
    let diagnostic_detail = cli_diagnostic_detail(input.detailed_errors);
    let trace_limit = input.trace_limit.unwrap_or(DEFAULT_TRACE_LIMIT);
    let trace_limit_present = input.trace_limit.is_some();
    if trace_limit == 0 {
        bail!("--trace-limit must be greater than 0");
    }
    let requested_trace_phase = input.trace_phase.map(TracePhase::from);
    normalize_trace_text_input(&mut input.trace, &input.file, &mut input.text);
    validate_vlasei_options(&input, glyphs)?;
    validate_trace_controls(
        &input.trace,
        new!(CliTraceValidation {
            command_name: "vlasei",
            trace_phase: requested_trace_phase,
            trace_limit_present,
            trace_list: input.trace_list,
            supports_morphology: true,
            supports_syntax: false,
        }),
    )?;
    if input.trace_list {
        write_trace_filter_list(
            stdout,
            requested_trace_phase.unwrap_or(TracePhase::Morphology),
            true,
            false,
        )?;
        return Ok(CliStatus::Success);
    }
    let morphology_trace_options = trace_options(
        &input.trace,
        requested_trace_phase.unwrap_or(TracePhase::Morphology),
        trace_limit,
    )?;
    let source_label = input_source_label(input.file.as_ref(), input.text.is_empty());
    let text = input.read_text_with_stdin(stdin_text)?;
    let dialect = input.dialect_definition()?;
    let morphology_options = MorphologyOptions::default()
        .with_dialect_definition(&dialect)
        .with_max_recovery_errors(input.max_errors.get())
        .with_trace_options(morphology_trace_options);
    let attempt = segment_words_with_modifiers_recovered_with_options_and_source_id_attempt(
        &text,
        &morphology_options,
        Some(SourceId(source_label.clone())),
    );
    let attempt = attempt.into_data();
    let trace_stderr = render_cli_trace(attempt.trace.as_ref(), color_policy.stderr);
    let morphology = attempt.result.into_data();
    if !morphology.errors.is_empty() {
        stderr.write_all(trace_stderr.as_bytes())?;
        let mut diagnostics = morphology_warning_diagnostics(
            &morphology.warnings,
            Some(SourceId(source_label.clone())),
            &text,
        );
        diagnostics.extend(
            morphology
                .errors
                .iter()
                .map(|error| error.to_diagnostic(Some(SourceId(source_label.clone())), &text)),
        );
        write_source_diagnostics(
            stderr,
            &source_label,
            &text,
            &diagnostics,
            color_policy.stderr,
            diagnostic_detail,
            glyphs,
            diagnostic_terminal_width,
        )?;
        return Ok(CliStatus::Failure);
    }
    let words = morphology.words;
    stderr.write_all(trace_stderr.as_bytes())?;
    let diagnostics = morphology_warning_diagnostics(
        &morphology.warnings,
        Some(SourceId(source_label.clone())),
        &text,
    );
    write_source_diagnostics(
        stderr,
        &source_label,
        &text,
        &diagnostics,
        color_policy.stderr,
        diagnostic_detail,
        glyphs,
        diagnostic_terminal_width,
    )?;
    let phoneme_options = phoneme_render_options(input.mark_stress, input.mark_glides, glyphs);
    match input.format {
        VlaseiFormat::Json => {
            let rendered = compact_morphology_json_string_with_options(
                &words,
                JsonRenderOptions {
                    indent: input.indent.unwrap_or(2),
                    phonemes: phoneme_options,
                    show_elided: false,
                    color: color_policy.stdout,
                },
            )?;
            writeln!(stdout, "{rendered}")?;
        }
        VlaseiFormat::Brackets => {
            let rendered = pretty_morphology_brackets_with_options(
                &words,
                &text,
                BracketRenderOptions {
                    color: color_policy.stdout,
                    phonemes: phoneme_options,
                    script: LojbanScript::Latin,
                    glyphs,
                    decompose_lujvo: input.decompose_lujvo,
                    insert_hair_space: false,
                    show_elided: false,
                },
            )?;
            writeln!(stdout, "{rendered}")?;
        }
        VlaseiFormat::Tree => {
            let rendered = pretty_morphology_tree_with_options(
                &words,
                &text,
                TreeRenderOptions {
                    color: color_policy.stdout,
                    indent: input.indent.unwrap_or(2),
                    phonemes: phoneme_options,
                    glyphs,
                    show_spans: input.show_spans,
                    // Morphology has no place-structure references.
                    show_refs: false,
                    decompose_lujvo: input.decompose_lujvo,
                    show_elided: false,
                },
            )?;
            writeln!(stdout, "{rendered}")?;
        }
        VlaseiFormat::Ipa => {
            let rendered = ipa_morphology_text(&words, &text)?;
            writeln!(stdout, "{rendered}")?;
        }
        VlaseiFormat::Raw => write_debug_output(stdout, &words, input.indent)?,
    }
    Ok(CliStatus::Success)
}
