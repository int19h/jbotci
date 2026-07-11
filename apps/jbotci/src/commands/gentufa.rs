use super::super::*;

#[allow(clippy::too_many_arguments)]
#[requires(diagnostic_terminal_width > 0)]
#[requires(trace.limit > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
pub(crate) fn run_gentufa<WOut: Write, WErr: Write>(
    input: GentufaInput,
    stdout: &mut WOut,
    stderr: &mut WErr,
    color_policy: CliColorPolicy,
    diagnostic_detail: DiagnosticDetailMode,
    glyphs: GlyphStyle,
    diagnostic_terminal_width: usize,
    trace: CliTraceConfig,
    stdin_text: Option<&str>,
) -> Result<CliStatus> {
    let output_file = input.output_file.clone();
    let rendered = render_gentufa(
        input,
        color_policy,
        diagnostic_detail,
        glyphs,
        diagnostic_terminal_width,
        trace,
        stdin_text,
    )?;
    stderr.write_all(rendered.stderr.as_bytes())?;
    if rendered.status == CliStatus::Success
        && let Some(path) = output_file.as_ref()
    {
        fs::write(path, &rendered.stdout)
            .with_context(|| format!("failed to write gentufa output to `{}`", path.display()))?;
    } else {
        stdout.write_all(&rendered.stdout)?;
    }
    Ok(rendered.status)
}

#[requires(diagnostic_terminal_width > 0)]
#[requires(trace.limit > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn render_gentufa(
    mut input: GentufaInput,
    color_policy: CliColorPolicy,
    diagnostic_detail: DiagnosticDetailMode,
    glyphs: GlyphStyle,
    diagnostic_terminal_width: usize,
    trace: CliTraceConfig,
    stdin_text: Option<&str>,
) -> Result<GentufaRendered> {
    normalize_trace_text_input(&mut input.trace, &input.file, &mut input.text);
    validate_gentufa_options(&input, glyphs)?;
    let morphology_trace_options = trace_options(&input.trace, trace.phase, trace.limit)?;
    let syntax_trace_options = trace_options(&input.trace, trace.phase, trace.limit)?;
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
        return Ok(new!(GentufaRendered {
            status: CliStatus::Failure,
            stdout: Vec::new(),
            stderr,
        }));
    }
    let words = morphology.words;
    let parse_options = ParseOptions::default()
        .with_dialect_definition(&dialect)
        .with_trace_options(syntax_trace_options)
        .with_error_context_depth(input.error_context)
        .with_max_recovery_errors(input.max_errors.get());
    let parsed = parse_syntax_tree_with_recovery_with_source_and_options_attempt(
        &words,
        &text,
        &parse_options,
    );
    let phoneme_options = phoneme_render_options(input.mark_stress, input.mark_glides, glyphs);
    let generated_model =
        match parsed.result.into_data() {
            data!(SyntaxRecoveryParse::Valid { parse }) => parse.into_data().parse_tree,
            data!(SyntaxRecoveryParse::Recovered { parse }) => {
                let stdout = render_recovered_gentufa_output(
                    &parse,
                    &text,
                    words.as_slice(),
                    &input,
                    color_policy.stdout,
                    glyphs,
                    phoneme_options,
                )?;
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
                stderr.push_str(&render_source_diagnostics(
                    &source_label,
                    &text,
                    &diagnostics,
                    color_policy.stderr,
                    diagnostic_detail,
                    glyphs,
                    diagnostic_terminal_width,
                )?);
                return Ok(new!(GentufaRendered {
                    status: CliStatus::Failure,
                    stdout,
                    stderr,
                }));
            }
        };
    let diagnostics = morphology_diagnostics;
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
    let mut stdout = String::new();
    if input.show_defs {
        let cards =
            dictionary_cards_for_word_likes(jbotci_dictionary_data::english(), words.as_slice());
        if !cards.is_empty() {
            stdout.push_str(&render_vlacku_output_with_options(
                &VlackuSearchOutput {
                    cards,
                    outcome: VlackuOutcome::Found,
                    diagnostics: Vec::new(),
                },
                new!(VlackuRenderOptions {
                    color: color_policy.stdout,
                    glyphs,
                    output_terminal_width: None,
                    sumti_places: CliSumtiPlaces::Index,
                    show_etymology: false,
                }),
            ));
        }
    }
    match input.format {
        GentufaFormat::Blocks => {
            let output_type = resolve_gentufa_blocks_output_type(&input)?;
            let stdout = render_gentufa_generated_blocks_output(
                &generated_model,
                &text,
                words.as_slice(),
                phoneme_options,
                output_type,
            )?;
            return Ok(new!(GentufaRendered {
                status: CliStatus::Success,
                stdout,
                stderr,
            }));
        }
        GentufaFormat::Brackets => {
            let rendered = pretty_generated_model_brackets_with_options(
                &generated_model,
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
            stdout.push_str(&rendered);
            stdout.push('\n');
        }
        GentufaFormat::Raw => {
            stdout.push_str(&debug_output_string(&generated_model, input.indent));
        }
        GentufaFormat::Tree => {
            let tree_options = TreeRenderOptions {
                color: color_policy.stdout,
                indent: input.indent.unwrap_or(2),
                phonemes: phoneme_options,
                glyphs,
                show_spans: input.show_spans,
                show_refs: input.show_refs,
                decompose_lujvo: input.decompose_lujvo,
                show_elided: false,
            };
            let rendered =
                pretty_generated_model_tree_with_options(&generated_model, &text, tree_options)?;
            stdout.push_str(&rendered);
            stdout.push('\n');
        }
        GentufaFormat::Json => {
            let rendered = compact_generated_model_json_string_with_options(
                &generated_model,
                JsonRenderOptions {
                    indent: input.indent.unwrap_or(2),
                    phonemes: phoneme_options,
                    show_elided: false,
                    color: color_policy.stdout,
                },
            )?;
            stdout.push_str(&rendered);
            stdout.push('\n');
        }
    }
    let stdout = stdout.into_bytes();
    Ok(new!(GentufaRendered {
        status: CliStatus::Success,
        stdout,
        stderr,
    }))
}

#[requires(true)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn render_recovered_gentufa_output(
    recovered: &jbotci_syntax::RecoveredSyntaxParse,
    source: &str,
    words: &[WordLike],
    input: &GentufaInput,
    color: bool,
    glyphs: GlyphStyle,
    phonemes: PhonemeRenderOptions,
) -> Result<Vec<u8>> {
    let rendered = match input.format {
        GentufaFormat::Blocks => {
            let output_type = resolve_gentufa_blocks_output_type(input)?;
            return render_gentufa_recovered_blocks_output(
                recovered,
                source,
                words,
                phonemes,
                output_type,
            );
        }
        GentufaFormat::Brackets => {
            let mut rendered = pretty_recovered_syntax_brackets_with_options(
                recovered,
                source,
                BracketRenderOptions {
                    color,
                    phonemes,
                    script: LojbanScript::Latin,
                    glyphs,
                    decompose_lujvo: input.decompose_lujvo,
                    insert_hair_space: false,
                    show_elided: false,
                },
            )?;
            rendered.push('\n');
            rendered
        }
        GentufaFormat::Raw => pretty_recovered_syntax_raw(recovered, input.indent),
        GentufaFormat::Tree => {
            let mut rendered = pretty_recovered_syntax_tree_with_options(
                recovered,
                source,
                TreeRenderOptions {
                    color,
                    indent: input.indent.unwrap_or(2),
                    phonemes,
                    glyphs,
                    show_spans: input.show_spans,
                    show_refs: input.show_refs,
                    decompose_lujvo: input.decompose_lujvo,
                    show_elided: false,
                },
            )?;
            rendered.push('\n');
            rendered
        }
        GentufaFormat::Json => {
            let mut rendered = compact_recovered_syntax_json_string_with_options(
                recovered,
                source,
                JsonRenderOptions {
                    indent: input.indent.unwrap_or(2),
                    phonemes,
                    show_elided: false,
                    color,
                },
            )?;
            rendered.push('\n');
            rendered
        }
    };
    Ok(rendered.into_bytes())
}

#[requires(!recovered.errors.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|output| !output.is_empty()) || ret.is_err())]
fn render_gentufa_recovered_blocks_output(
    recovered: &jbotci_syntax::RecoveredSyntaxParse,
    source: &str,
    words: &[WordLike],
    phoneme_options: PhonemeRenderOptions,
    output_type: GentufaImageOutputType,
) -> Result<Vec<u8>> {
    let block_options = GentufaBlockOptions {
        script: GentufaScript::Latin,
        show_elided: false,
        phonemes: phoneme_options,
    };
    let annotations = gentufa_block_annotations(words);
    let layout = recovered_generated_model_blocks_layout(
        recovered.parse_tree.as_ref(),
        source,
        recovered.errors.len(),
        &annotations,
        &block_options,
    );
    render_gentufa_blocks_output(&layout, output_type, "jbotci gentufa recovered syntax")
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|output| !output.is_empty()) || ret.is_err())]
fn render_gentufa_generated_blocks_output(
    syntax: &jbotci_syntax::generated_model::TextSyntax,
    source: &str,
    words: &[WordLike],
    phoneme_options: PhonemeRenderOptions,
    output_type: GentufaImageOutputType,
) -> Result<Vec<u8>> {
    let block_options = GentufaBlockOptions {
        script: GentufaScript::Latin,
        show_elided: false,
        phonemes: phoneme_options,
    };
    let annotations = gentufa_block_annotations(words);
    let reference_display = generated_reference_display(
        syntax,
        source,
        TreeRenderOptions {
            color: false,
            indent: 2,
            phonemes: phoneme_options,
            glyphs: GlyphStyle::Unicode,
            show_spans: false,
            show_refs: true,
            decompose_lujvo: false,
            show_elided: false,
        },
    )?;
    let layout = generated_model_blocks_layout_with_references(
        syntax,
        source,
        Some(&reference_display.analysis.syntax_index),
        Some(&reference_display.references),
        &annotations,
        &block_options,
    );
    render_gentufa_blocks_output(&layout, output_type, "jbotci gentufa generated syntax")
}

#[requires(!title.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|output| !output.is_empty()) || ret.is_err())]
fn render_gentufa_blocks_output<Tooltip, ReferenceTooltip>(
    layout: &jbotci_gentufa::GentufaBlocksLayout<Tooltip, ReferenceTooltip>,
    output_type: GentufaImageOutputType,
    title: &str,
) -> Result<Vec<u8>> {
    let svg_options = GentufaSvgOptions {
        show_glosses: false,
        script: GentufaScript::Latin,
        title: title.to_owned(),
    };
    let fonts = EmbeddedGentufaFonts::get();
    match output_type {
        GentufaImageOutputType::Svg => {
            Ok(render_gentufa_blocks_svg(&layout, &svg_options, fonts)?.into_bytes())
        }
        GentufaImageOutputType::Png => Ok(render_gentufa_blocks_png(
            &layout,
            &GentufaPngOptions::default().with_data(data! { svg: svg_options }),
            fonts,
        )?),
    }
}

#[requires(true)]
#[ensures(true)]
fn gentufa_block_annotations(words: &[WordLike]) -> Vec<GentufaBlockAnnotation<()>> {
    dictionary_matches_for_word_likes(jbotci_dictionary_data::english(), words)
        .into_iter()
        .map(|parsed_match| {
            let parsed_match = parsed_match.into_data();
            let first = parsed_match.cards.first();
            GentufaBlockAnnotation {
                range: new!(WebSourceRange {
                    byte_start: parsed_match.byte_start,
                    byte_end: parsed_match.byte_end,
                    char_start: parsed_match.char_start,
                    char_end: parsed_match.char_end,
                }),
                text: Some(parsed_match.lookup_text),
                glosses: first.map(|card| card.glosses.clone()).unwrap_or_default(),
                definition: first
                    .map(|card| card.definition.trim().to_owned())
                    .filter(|definition| !definition.is_empty()),
                tooltip: None,
            }
        })
        .collect()
}
