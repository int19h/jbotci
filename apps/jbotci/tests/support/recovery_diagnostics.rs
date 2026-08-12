use super::*;

use clap::error::ErrorKind;
use jbotci_cli::{ToolGentufaFormat, ToolGentufaRequest, ToolStatus, run_tool_gentufa};
use jbotci_diagnostics::{
    Diagnostic, DiagnosticDetailMode, DiagnosticLabel, DiagnosticNoteMode, DiagnosticPhase,
    DiagnosticSeverity, DiagnosticStyledNote, DiagnosticTextRole, DiagnosticTextSegment,
};
use jbotci_output::{
    DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH, DiagnosticRenderOptions, GlyphStyle, render_diagnostics,
};
use jbotci_source::{SourceId, SourceSpan};

const SYNTAX_MULTI_ERROR_SOURCE: &str = "mi ku i do ku i mi klama";
const MORPHOLOGY_MULTI_ERROR_SOURCE: &str = "mi @@@ do ### mi";
const SYNTAX_EXPECTED_LABEL: &str = "expected: free modifier, space interval, sumti association phrase, time interval, space tense, time tense, joik, sumti relative phrase, termset connection continuation, termset connective, place tag, tag, paragraph statement, prenex, or paragraph";
const SYNTAX_DETAILED_NOTE: &str = "needs one of:\n- replacement phrase ({lo'ai})\n- space interval (VEhA)\n- sumti association phrase (GOI)\n- time interval (ZEhA)\n- space tense (FAhA or VA)\n- time tense (PU or ZI)\n- joik (JOI)\n- sumti relative phrase ({vu'o})\n- termset connection continuation ({pe'e})\n- termset connective ({ce'e})\n- place tag (FA)\n- tag ({fi'o})\n- term connection (NA, NAhE, SE, {cu}, {nu'i}, {pe'o}, or {vau})\n- paragraph statement ({i})\n- {zo'u} [continues prenex]\n- paragraph (NIhO)";

#[invariant(stderr.is_empty() || stderr.ends_with('\n'))]
struct CapturedCli {
    status: CliStatus,
    stdout: String,
    stderr: String,
}

#[requires(!args.is_empty())]
#[ensures(ret.stderr.is_empty() || ret.stderr.ends_with('\n'))]
fn capture_cli(args: &[&str]) -> CapturedCli {
    let cli = Cli::try_parse_from(args).expect("recovery diagnostic command should parse");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = run_cli(cli, &mut stdout, &mut stderr, false)
        .expect("recovery diagnostic command should run");
    new!(CapturedCli {
        status,
        stdout: String::from_utf8(stdout).expect("CLI stdout should be UTF-8"),
        stderr: String::from_utf8(stderr).expect("CLI stderr should be UTF-8"),
    })
}

#[requires(start < end)]
#[requires(end <= source.len())]
#[ensures(ret.byte_start == start)]
#[ensures(ret.byte_end == end)]
fn ascii_source_span(source: &str, start: usize, end: usize) -> SourceSpan {
    assert!(source.is_ascii(), "test diagnostic source must be ASCII");
    SourceSpan::new(Some(SourceId("<input>".to_owned())), start, end, start, end)
        .expect("ordered ASCII offsets should produce a source span")
}

#[requires(error_count <= 2)]
#[ensures(error_count == 0 -> ret.is_empty())]
#[ensures(error_count > 0 -> ret.contains("syntax.unexpected-cmavo"))]
fn expected_syntax_stderr(detail: DiagnosticDetailMode, error_count: usize) -> String {
    let locations = [(3, 5, 0, 5), (11, 13, 8, 13)];
    let diagnostics = locations[..error_count]
        .iter()
        .map(|&(error_start, error_end, context_start, context_end)| {
            let diagnostic = Diagnostic::new(
                DiagnosticSeverity::Error,
                DiagnosticPhase::Syntax,
                "syntax.unexpected-cmavo".to_owned(),
                "unexpected cmavo".to_owned(),
                vec![
                    DiagnosticLabel::new(
                        ascii_source_span(SYNTAX_MULTI_ERROR_SOURCE, error_start, error_end),
                        SYNTAX_EXPECTED_LABEL.to_owned(),
                        true,
                    ),
                    DiagnosticLabel::new(
                        ascii_source_span(SYNTAX_MULTI_ERROR_SOURCE, context_start, context_end),
                        "while parsing term connection".to_owned(),
                        false,
                    ),
                ],
                Vec::new(),
                None,
            );
            if detail == DiagnosticDetailMode::Detailed {
                diagnostic.with_styled_notes(vec![DiagnosticStyledNote::new(
                    DiagnosticNoteMode::Detailed,
                    vec![DiagnosticTextSegment::new(
                        DiagnosticTextRole::Plain,
                        SYNTAX_DETAILED_NOTE.to_owned(),
                    )],
                )])
            } else {
                diagnostic
            }
        })
        .collect::<Vec<_>>();
    render_diagnostics(
        "<input>",
        SYNTAX_MULTI_ERROR_SOURCE,
        &diagnostics,
        new!(DiagnosticRenderOptions {
            color: false,
            detail,
            glyphs: GlyphStyle::Unicode,
            terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
        }),
    )
    .expect("documented syntax diagnostics should render")
}

#[requires(error_count <= 2)]
#[ensures(error_count == 0 -> ret.is_empty())]
#[ensures(error_count > 0 -> ret.contains("morphology.invalid-character"))]
fn expected_morphology_stderr(error_count: usize) -> String {
    let diagnostics = [(3, 4), (10, 11)][..error_count]
        .iter()
        .map(|&(start, end)| {
            Diagnostic::new(
                DiagnosticSeverity::Error,
                DiagnosticPhase::Morphology,
                "morphology.invalid-character".to_owned(),
                "invalid character in Lojban word".to_owned(),
                vec![DiagnosticLabel::new(
                    ascii_source_span(MORPHOLOGY_MULTI_ERROR_SOURCE, start, end),
                    "invalid character in Lojban word".to_owned(),
                    true,
                )],
                Vec::new(),
                None,
            )
        })
        .collect::<Vec<_>>();
    render_diagnostics(
        "<input>",
        MORPHOLOGY_MULTI_ERROR_SOURCE,
        &diagnostics,
        new!(DiagnosticRenderOptions {
            color: false,
            detail: DiagnosticDetailMode::Summary,
            glyphs: GlyphStyle::Unicode,
            terminal_width: DEFAULT_DIAGNOSTIC_TERMINAL_WIDTH,
        }),
    )
    .expect("documented morphology diagnostics should render")
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_renders_both_syntax_errors_exactly() {
    let run = capture_cli(&["jbotci", "gentufa", SYNTAX_MULTI_ERROR_SOURCE]);

    assert_eq!(run.status, CliStatus::Failure);
    assert_eq!(run.stdout, "([mi ‼ku‼] [{.i do} ‼ku‼ {.i (mi kláma)}])\n");
    assert_eq!(
        run.stderr,
        expected_syntax_stderr(DiagnosticDetailMode::Summary, 2)
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn morphology_errors_suppress_syntax_in_every_syntax_command() {
    for command in ["gentufa", "tersmu"] {
        let run = capture_cli(&["jbotci", command, MORPHOLOGY_MULTI_ERROR_SOURCE]);

        assert_eq!(run.status, CliStatus::Failure, "{command}");
        assert!(run.stdout.is_empty(), "{command}");
        assert_eq!(run.stderr, expected_morphology_stderr(2), "{command}");
        assert!(!run.stderr.contains("syntax."), "{command}");
    }

    let blocks = capture_cli(&[
        "jbotci",
        "gentufa",
        "--turtai",
        "blocks",
        "--output-type",
        "svg",
        MORPHOLOGY_MULTI_ERROR_SOURCE,
    ]);
    assert_eq!(blocks.status, CliStatus::Failure);
    assert!(blocks.stdout.is_empty());
    assert_eq!(blocks.stderr, expected_morphology_stderr(2));
    assert!(!blocks.stderr.contains("syntax."));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn gentufa_blocks_svg_renders_recovered_regions_without_changing_diagnostics() {
    let run = capture_cli(&[
        "jbotci",
        "gentufa",
        "--turtai",
        "blocks",
        "--output-type",
        "svg",
        SYNTAX_MULTI_ERROR_SOURCE,
    ]);

    assert_eq!(run.status, CliStatus::Failure);
    assert_recovered_blocks_svg(&run.stdout);
    assert_eq!(
        run.stderr,
        expected_syntax_stderr(DiagnosticDetailMode::Summary, 2)
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn tersmu_renders_all_syntax_errors_without_semantic_output() {
    let run = capture_cli(&["jbotci", "tersmu", SYNTAX_MULTI_ERROR_SOURCE]);

    assert_eq!(run.status, CliStatus::Failure);
    assert!(run.stdout.is_empty());
    assert_eq!(
        run.stderr,
        expected_syntax_stderr(DiagnosticDetailMode::Summary, 2)
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn vlasei_preserves_valid_output_and_reports_all_failure_diagnostics() {
    let valid = capture_cli(&["jbotci", "vlasei", "mi klama"]);
    assert_eq!(valid.status, CliStatus::Success);
    assert_eq!(valid.stdout, "(mi kláma)\n");
    assert!(valid.stderr.is_empty());

    let invalid = capture_cli(&["jbotci", "vlasei", MORPHOLOGY_MULTI_ERROR_SOURCE]);
    assert_eq!(invalid.status, CliStatus::Failure);
    assert_eq!(invalid.stdout, "(mi ‼@@@ ‼ do ‼### ‼ mi)\n");
    assert_eq!(invalid.stderr, expected_morphology_stderr(2));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn max_errors_one_caps_both_recovery_phases_at_the_first_diagnostic() {
    let syntax = capture_cli(&[
        "jbotci",
        "gentufa",
        "--max-errors",
        "1",
        SYNTAX_MULTI_ERROR_SOURCE,
    ]);
    assert_eq!(syntax.status, CliStatus::Failure);
    assert!(!syntax.stdout.is_empty());
    assert_eq!(syntax.stdout.matches('‼').count(), 2);
    assert_eq!(
        syntax.stderr,
        expected_syntax_stderr(DiagnosticDetailMode::Summary, 1)
    );

    let morphology = capture_cli(&[
        "jbotci",
        "vlasei",
        "--max-errors",
        "1",
        MORPHOLOGY_MULTI_ERROR_SOURCE,
    ]);
    assert_eq!(morphology.status, CliStatus::Failure);
    assert!(!morphology.stdout.is_empty());
    assert_eq!(morphology.stdout.matches('‼').count(), 2);
    assert_eq!(morphology.stderr, expected_morphology_stderr(1));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn max_errors_uses_parser_defaults_and_rejects_zero_for_every_parsing_command() {
    for command in ["gentufa", "tersmu", "vlasei"] {
        let error = Cli::try_parse_from(["jbotci", command, "--max-errors", "0", "mi"])
            .expect_err("zero recovery error cap must be rejected");
        assert_eq!(error.kind(), ErrorKind::ValueValidation, "{command}");
        let rendered = error.to_string();
        assert!(
            rendered.contains("invalid value '0'"),
            "{command}: {rendered}"
        );
        assert!(rendered.contains("--max-errors"), "{command}: {rendered}");
        assert!(
            rendered.contains("number would be zero for non-zero type"),
            "{command}: {rendered}"
        );
    }

    let gentufa = Cli::try_parse_from(["jbotci", "gentufa", "mi"])
        .expect("default gentufa arguments should parse");
    let Command::Gentufa(gentufa) = gentufa.command else {
        panic!("gentufa command should parse as gentufa");
    };
    assert_eq!(gentufa.max_errors, None);

    let tersmu = Cli::try_parse_from(["jbotci", "tersmu", "mi"])
        .expect("default tersmu arguments should parse");
    let Command::Tersmu(tersmu) = tersmu.command else {
        panic!("tersmu command should parse as tersmu");
    };
    assert_eq!(tersmu.max_errors, None);

    let vlasei = Cli::try_parse_from(["jbotci", "vlasei", "mi"])
        .expect("default vlasei arguments should parse");
    let Command::Vlasei(vlasei) = vlasei.command else {
        panic!("vlasei command should parse as vlasei");
    };
    assert_eq!(vlasei.max_errors.get(), 20);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn run_tool_gentufa_returns_partial_stdout_and_full_stderr_for_structural_formats() {
    let expected = expected_syntax_stderr(DiagnosticDetailMode::Detailed, 2);
    for format in [
        ToolGentufaFormat::Tree,
        ToolGentufaFormat::Brackets,
        ToolGentufaFormat::Raw,
        ToolGentufaFormat::Json,
    ] {
        let output = run_tool_gentufa(ToolGentufaRequest {
            text: SYNTAX_MULTI_ERROR_SOURCE.to_owned(),
            format,
            dialect: None,
            show_defs: false,
            show_spans: false,
            show_refs: Some(false),
            show_elided: false,
            decompose_lujvo: false,
            indent: (format == ToolGentufaFormat::Raw).then_some(0),
        })
        .expect("gentufa tool call should run");

        assert_eq!(output.status, ToolStatus::Failure, "{format:?}");
        let stdout = output.stdout_text().expect("structural output is UTF-8");
        assert!(!stdout.is_empty(), "{format:?}");
        assert_recovered_tool_stdout(format, stdout);
        assert_eq!(output.stderr, expected, "{format:?}");
    }

    let svg = run_tool_gentufa(ToolGentufaRequest {
        text: SYNTAX_MULTI_ERROR_SOURCE.to_owned(),
        format: ToolGentufaFormat::Svg,
        dialect: None,
        show_defs: false,
        show_spans: false,
        show_refs: Some(false),
        show_elided: false,
        decompose_lujvo: false,
        indent: None,
    })
    .expect("gentufa SVG tool call should run");
    assert_eq!(svg.status, ToolStatus::Failure);
    assert_recovered_blocks_svg(svg.stdout_text().expect("SVG output is UTF-8"));
    assert_eq!(svg.stderr, expected);

    let png = run_tool_gentufa(ToolGentufaRequest {
        text: SYNTAX_MULTI_ERROR_SOURCE.to_owned(),
        format: ToolGentufaFormat::Png,
        dialect: None,
        show_defs: false,
        show_spans: false,
        show_refs: Some(false),
        show_elided: false,
        decompose_lujvo: false,
        indent: None,
    })
    .expect("gentufa PNG tool call should run");
    assert_eq!(png.status, ToolStatus::Failure);
    assert!(png.stdout.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(png.stderr, expected);
}

#[requires(!svg.is_empty())]
#[ensures(true)]
fn assert_recovered_blocks_svg(svg: &str) {
    assert!(svg.starts_with("<svg "), "{svg}");
    assert_eq!(svg.matches("data-role=\"error\"").count(), 2, "{svg}");
    assert!(
        svg.contains(
            "data-role=\"error\" data-error-index=\"0\" data-byte-start=\"3\" data-byte-end=\"5\""
        ),
        "{svg}"
    );
    assert!(
        svg.contains(
            "data-role=\"error\" data-error-index=\"1\" data-byte-start=\"11\" data-byte-end=\"13\""
        ),
        "{svg}"
    );
    assert_eq!(svg.matches(">ku</text>").count(), 2, "{svg}");
    assert!(svg.contains(">do</text>"), "{svg}");
    assert!(svg.contains(">kláma</text>"), "{svg}");
    assert!(svg.contains("fill=\"#fee2e2\""), "{svg}");
    assert!(svg.contains("text-decoration=\"line-through\""), "{svg}");
}

#[requires(true)]
#[ensures(true)]
fn assert_recovered_tool_stdout(format: ToolGentufaFormat, stdout: &str) {
    match format {
        ToolGentufaFormat::Brackets => {
            assert_eq!(stdout, "([mi ‼ku‼] [{.i do} ‼ku‼ {.i (mi kláma)}])\n")
        }
        ToolGentufaFormat::Tree => {
            assert!(stdout.starts_with("ParagraphStatementSequence"), "{stdout}");
            assert_eq!(stdout.matches("Error \"ku\"").count(), 2, "{stdout}");
            assert!(stdout.contains("Cmavo \"do\""), "{stdout}");
            assert!(stdout.contains("Gismu \"kláma\""), "{stdout}");
        }
        ToolGentufaFormat::Raw => {
            assert!(
                stdout.contains("SkippedTokens { error_index: 0"),
                "{stdout}"
            );
            assert!(
                stdout.contains("SkippedTokens { error_index: 1"),
                "{stdout}"
            );
            assert!(stdout.contains("text: \"do\""), "{stdout}");
            assert!(stdout.contains("text: \"kláma\""), "{stdout}");
        }
        ToolGentufaFormat::Json => {
            let value: serde_json::Value = serde_json::from_str(stdout).expect("tool JSON");
            let mut errors = Vec::new();
            collect_json_errors(&value, &mut errors);
            assert_eq!(errors.len(), 2);
            assert_eq!(errors[0]["error_index"], 0);
            assert_eq!(errors[0]["span"], serde_json::json!([3, 5]));
            assert_eq!(errors[0]["diagnostic_code"], "syntax.unexpected-cmavo");
            assert_eq!(errors[1]["error_index"], 1);
            assert_eq!(errors[1]["span"], serde_json::json!([11, 13]));
            assert_eq!(errors[1]["diagnostic_code"], "syntax.unexpected-cmavo");
            assert_eq!(
                value["ParagraphStatementSequence"]["following"][2]["ParagraphStatement"]["value"]
                    ["BridiWithLeadingTerms"]["bridi_tail"]["Gismu"]["phonemes"],
                "kláma"
            );
        }
        ToolGentufaFormat::Svg | ToolGentufaFormat::Png => {
            panic!("image formats have no recovered structural output")
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn collect_json_errors<'value>(
    value: &'value serde_json::Value,
    errors: &mut Vec<&'value serde_json::Map<String, serde_json::Value>>,
) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(serde_json::Value::Object(error)) = object.get("Error") {
                errors.push(error);
            } else {
                for child in object.values() {
                    collect_json_errors(child, errors);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_json_errors(item, errors);
            }
        }
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {}
    }
}
