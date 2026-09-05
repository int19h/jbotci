use super::super::*;

const EMPTY_TEXT_RENDERING: &str = "Text {}";
const GENERATED_BLOCKS_TITLE: &str = "jbotci gentufa generated syntax";
const RECOVERED_BLOCKS_TITLE: &str = "jbotci gentufa recovered syntax";

#[requires(true)]
#[ensures(!ret.is_empty())]
#[ensures(!rendered.is_empty() -> ret == rendered)]
#[ensures(rendered.is_empty() -> ret == EMPTY_TEXT_RENDERING)]
fn visible_bracket_rendering(rendered: &str) -> &str {
    if rendered.is_empty() {
        EMPTY_TEXT_RENDERING
    } else {
        rendered
    }
}

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

/// Both parsing phases over one CLI input, kept apart from rendering so every
/// output format starts from the same analysis.
#[invariant(!source_label.is_empty())]
#[invariant(morphology_trace_stderr.is_empty() || morphology_trace_stderr.ends_with('\n'))]
#[invariant(
    !matches!(syntax, GentufaSyntaxStage::MorphologyFailed) || !diagnostics.is_empty(),
    "failed segmentation always reports its errors"
)]
struct GentufaAnalysis {
    text: String,
    source_label: String,
    morphology_trace_stderr: String,
    /// Morphology warnings, followed by the morphology errors when segmentation
    /// failed; syntax diagnostics are appended by the renderer.
    diagnostics: Vec<Diagnostic>,
    syntax: GentufaSyntaxStage,
}

#[invariant(::MorphologyFailed => true)]
#[invariant(::Parsed { .. } => true)]
enum GentufaSyntaxStage {
    /// Word segmentation failed, so there is no syntax parse to render.
    MorphologyFailed,
    Parsed {
        words: Vec<WordLike>,
        parse: SyntaxRecoveryParse,
    },
}

#[requires(trace.limit > 0)]
#[ensures(ret.as_ref().err().is_none_or(|error| !error.to_string().is_empty()))]
fn analyze_gentufa_input(
    input: &GentufaInput,
    color_stderr: bool,
    trace: CliTraceConfig,
    stdin_text: Option<&str>,
) -> Result<GentufaAnalysis> {
    let morphology_trace_options = trace_options(&input.trace, trace.phase, trace.limit)?;
    let syntax_trace_options = trace_options(&input.trace, trace.phase, trace.limit)?;
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
        )
        .into_data();
    let morphology_trace_stderr = render_cli_trace(morphology_attempt.trace.as_ref(), color_stderr);
    let morphology = morphology_attempt.result.into_data();
    let mut diagnostics = morphology_warning_diagnostics(
        &morphology.warnings,
        Some(SourceId(source_label.clone())),
        &text,
    );
    if !morphology.errors.is_empty() {
        diagnostics.extend(morphology.errors.iter().map(|error| {
            error
                .to_diagnostic(Some(SourceId(source_label.clone())), &text)
                .expect("morphology error offsets belong to the parser source")
        }));
        return Ok(new!(GentufaAnalysis {
            text,
            source_label,
            morphology_trace_stderr,
            diagnostics,
            syntax: GentufaSyntaxStage::MorphologyFailed,
        }));
    }
    let words = morphology.words;
    let mut parse_options = ParseOptions::default()
        .with_dialect_definition(&dialect)
        .with_trace_options(syntax_trace_options)
        .with_error_context_depth(input.error_context);
    if let Some(max_errors) = input.max_errors {
        parse_options = parse_options.with_max_recovery_errors(max_errors.get());
    }
    let parse = parse_syntax_tree_with_recovery_with_source_and_options_attempt(
        &words,
        &text,
        &parse_options,
    )
    .result;
    Ok(new!(GentufaAnalysis {
        text,
        source_label,
        morphology_trace_stderr,
        diagnostics,
        syntax: GentufaSyntaxStage::Parsed { words, parse },
    }))
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
    let analysis = analyze_gentufa_input(&input, color_policy.stderr, trace, stdin_text)?;
    let data!(GentufaAnalysis {
        text,
        source_label,
        morphology_trace_stderr,
        diagnostics,
        syntax,
    }) = analysis.into_data();
    let mut diagnostics = diagnostics;
    let (words, parse) = match syntax {
        GentufaSyntaxStage::MorphologyFailed => {
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
        GentufaSyntaxStage::Parsed { words, parse } => (words, parse),
    };
    let phoneme_options = phoneme_render_options(input.mark_stress, input.mark_glides, glyphs);
    // A valid (non-recovered) parse can still carry warnings (e.g. experimental
    // syntax). They are advisory, not errors, so the command stays successful,
    // but the warnings must still reach stderr just like on the recovered path.
    // Both arms fold their syntax diagnostics into the same accumulator, seeded
    // with the morphology diagnostics.
    let generated_model =
        match parse.into_data() {
            data!(SyntaxRecoveryParse::Valid { parse }) => {
                let parsed = parse.into_data();
                diagnostics.extend(parsed.warnings.iter().map(|warning| {
                    warning.to_diagnostic(Some(SourceId(source_label.clone())), &text)
                }));
                parsed.parse_tree
            }
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
        stdout.push_str(&render_dictionary_definitions_for_word_likes(
            words.as_slice(),
            color_policy.stdout,
            glyphs,
        ));
    }
    match input.format {
        GentufaFormat::Blocks => {
            let output_type = resolve_gentufa_blocks_output_type(&input)?;
            let layout = generated_model_gentufa_blocks_projection(
                &generated_model,
                &text,
                words.as_slice(),
                &gentufa_blocks_projection_options(&input, phoneme_options),
            )?
            .into_blocks_layout();
            let stdout = render_gentufa_blocks_output(
                &layout,
                input.show_glosses,
                output_type,
                GENERATED_BLOCKS_TITLE,
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
                    show_elided: input.show_elided,
                },
            )?;
            stdout.push_str(visible_bracket_rendering(&rendered));
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
                show_elided: input.show_elided,
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
                    show_elided: input.show_elided,
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

#[requires(!recovered.errors.is_empty())]
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
            let layout = recovered_gentufa_blocks_projection(
                recovered,
                source,
                words,
                &gentufa_blocks_projection_options(input, phonemes),
            )
            .into_blocks_layout();
            return render_gentufa_blocks_output(
                &layout,
                input.show_glosses,
                output_type,
                RECOVERED_BLOCKS_TITLE,
            );
        }
        GentufaFormat::Brackets => {
            let rendered = pretty_recovered_syntax_brackets_with_options(
                recovered,
                source,
                BracketRenderOptions {
                    color,
                    phonemes,
                    script: LojbanScript::Latin,
                    glyphs,
                    decompose_lujvo: input.decompose_lujvo,
                    insert_hair_space: false,
                    show_elided: input.show_elided,
                },
            )?;
            let mut rendered = visible_bracket_rendering(&rendered).to_owned();
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
                    show_elided: input.show_elided,
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
                    show_elided: input.show_elided,
                    color,
                },
            )?;
            rendered.push('\n');
            rendered
        }
    };
    Ok(rendered.into_bytes())
}

/// The CLI's blocks share the web's dictionary-backed projection; only the
/// script is fixed (the CLI has no orthography selector) and the phonemes follow
/// the CLI's stress and glide flags.
#[requires(true)]
#[ensures(ret.blocks.show_elided == input.show_elided)]
#[ensures(ret.show_compounds == input.show_compounds)]
fn gentufa_blocks_projection_options(
    input: &GentufaInput,
    phonemes: PhonemeRenderOptions,
) -> GentufaBlocksProjectionOptions {
    GentufaBlocksProjectionOptions {
        blocks: GentufaBlockOptions {
            script: GentufaScript::Latin,
            show_elided: input.show_elided,
            phonemes,
        },
        show_compounds: input.show_compounds,
    }
}

#[requires(!title.is_empty())]
#[ensures(ret.as_ref().is_ok_and(|output| !output.is_empty()) || ret.is_err())]
fn render_gentufa_blocks_output<Tooltip, ReferenceTooltip>(
    layout: &jbotci_gentufa::GentufaBlocksLayout<Tooltip, ReferenceTooltip>,
    show_glosses: bool,
    output_type: GentufaImageOutputType,
    title: &str,
) -> Result<Vec<u8>> {
    let svg_options = GentufaSvgOptions {
        show_glosses,
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

#[cfg(test)]
mod tests {
    use super::*;
    use jbotci_web_core::{
        GentufaBlocksLayout, GentufaWebOptions, GentufaWebRequest, GentufaWebResult,
        parse_gentufa_for_web,
    };

    /// The layout the CLI renders for `args`, produced by the CLI's own parse
    /// pipeline and option resolution exactly as `render_gentufa` does it.
    #[requires(!args.is_empty())]
    #[ensures(true)]
    fn cli_blocks_layout(args: &[&str]) -> GentufaBlocksLayout {
        let cli = Cli::try_parse_from(args).expect("CLI args parse");
        let Command::Gentufa(mut input) = cli.command else {
            panic!("gentufa command");
        };
        normalize_trace_text_input(&mut input.trace, &input.file, &mut input.text);
        validate_gentufa_options(&input, GlyphStyle::Unicode).expect("blocks options validate");
        let trace = CliTraceConfig {
            phase: TracePhase::Syntax,
            limit: DEFAULT_TRACE_LIMIT,
        };
        let analysis = analyze_gentufa_input(&input, false, trace, None).expect("analysis");
        let data!(GentufaAnalysis { text, syntax, .. }) = analysis.into_data();
        let GentufaSyntaxStage::Parsed { words, parse } = syntax else {
            panic!("morphology succeeds for {args:?}");
        };
        let phonemes =
            phoneme_render_options(input.mark_stress, input.mark_glides, GlyphStyle::Unicode);
        let options = gentufa_blocks_projection_options(&input, phonemes);
        match parse.into_data() {
            data!(SyntaxRecoveryParse::Valid { parse }) => {
                generated_model_gentufa_blocks_projection(
                    &parse.parse_tree,
                    &text,
                    &words,
                    &options,
                )
                .expect("valid projection")
                .into_blocks_layout()
            }
            data!(SyntaxRecoveryParse::Recovered { parse }) => {
                recovered_gentufa_blocks_projection(&parse, &text, &words, &options)
                    .into_blocks_layout()
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn blocks_layout_matches_the_web_projection_for_identical_options() {
        // Valid inputs with and without attested compounds, then recovered inputs
        // whose error regions must stay compound barriers on both surfaces.
        for (source, has_compound) in [
            ("mi pa moi klama", true),
            ("mi klama", false),
            ("la pa da cu klama .i ri tavla", true),
            ("ba pu mi ku i do", true),
            ("mi ku i do ku i mi klama", false),
        ] {
            for show_elided in [false, true] {
                for show_compounds in [false, true] {
                    let mut args = vec!["jbotci", "gentufa", "--turtai", "blocks"];
                    if show_elided {
                        args.push("--show-elided");
                    }
                    if !show_compounds {
                        args.push("--no-compounds");
                    }
                    args.push(source);
                    let cli = cli_blocks_layout(&args);
                    let request = GentufaWebRequest {
                        text: source.to_owned(),
                        options: GentufaWebOptions {
                            show_elided,
                            show_compounds,
                            ..GentufaWebOptions::default()
                        },
                    };
                    let GentufaWebResult::Success(web) = parse_gentufa_for_web(&request) else {
                        panic!("web projection succeeds for {source}");
                    };
                    let context =
                        format!("{source} elided={show_elided} compounds={show_compounds}");
                    assert_eq!(cli, web.blocks_layout, "{context}");
                    assert_eq!(
                        cli.blocks.iter().any(|block| block.compound_kind.is_some()),
                        has_compound && show_compounds,
                        "{context}"
                    );
                    assert_eq!(
                        cli.blocks.iter().any(|block| block.role.is_elided()),
                        show_elided,
                        "{context}"
                    );
                }
            }
        }
    }
}
