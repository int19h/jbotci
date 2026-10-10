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
// Both constants omit `selbri`. The error selector discards every alternative that offers it.
// Issue #926 tracks this defect. PEhO remains a valid expectation through mex before MOI.
const SYNTAX_EXPECTED_LABEL: &str = "expected: free modifier, forethought operator connective, LAhE-qualified operand, VUhU operator, converted operator, grouped operator, mekso array, number, operand-to-operator, parenthesized mex, qualified operand, scalar-negated operand, selbri operand, selbri-to-operator, space interval property, sumti operand, ek, forethought mex, reverse Polish mex, space interval, sumti association phrase, time interval, descriptor, interval, interval property, lerfu word, modal tag, quantifier, relative clause, space tense, sumti connective, text quote, time tense, FIhO modal, abstraction, bridi description, converted sumti, converted term, grouped tanru, jek, joik, linked arguments, modal conversion, name, number sumti, operator-to-selbri, pro-bridi, quote, scalar-negated sumti, scalar-negated tanru unit, scalar-negated term, sumti relative phrase, sumti-to-selbri, termset connection continuation, FIhOI adverbial, NA KU term, NA term, SOI adverbial, forethought selbri connective, place tag, sumti, tag, tanru unit, termset continuation, forethought connective, negated selbri, paragraph statement, termset, forethought bridi connection, prenex, paragraph, or end of input";
const SYNTAX_DETAILED_NOTE: &str = "needs one of:\n- vocative marker (COI or {doi})\n- metalinguistic comment (SEI)\n- parenthetical text (TO)\n- reciprocal ({soi})\n- replacement phrase ({le'ai}, {lo'ai}, or {sa'ai})\n- subscript (XI)\n- forethought operator connective (GUhA)\n- LAhE-qualified operand (LAhE)\n- VUhU operator (VUhU)\n- converted operator (NAhE or SE)\n- grouped operator ({ke})\n- mekso array ({jo'i})\n- number (PA)\n- operand-to-operator ({ma'o})\n- parenthesized mex ({vei})\n- qualified operand (NAhE)\n- scalar-negated operand (NAhE)\n- selbri operand ({ni'e})\n- selbri-to-operator ({na'u})\n- space interval property ({fe'e})\n- sumti operand ({mo'e})\n- ek (A or JEhI)\n- forethought mex ({pe'o})\n- reverse Polish mex ({fu'a})\n- space interval (VEhA or VIhA)\n- sumti association phrase (GOI)\n- time interval (ZEhA)\n- descriptor (LA or LE)\n- interval (BIhI or GAhO)\n- interval property (TAhE or ZAhO)\n- lerfu word (LERFU, LAU, or {tei})\n- modal tag (BAI)\n- quantifier ({vei})\n- relative clause ({noi}, {poi}, or {voi})\n- space tense (FAhA, MOhI, or VA)\n- sumti connective (VUhU)\n- text quote ({lu})\n- time tense (PU or ZI)\n- FIhO modal ({fi'o})\n- abstraction (NU)\n- bridi description ({lo'oi})\n- converted sumti (LAhE)\n- converted term (LAhE)\n- grouped tanru ({ke})\n- jek (JA)\n- joik (JOI)\n- linked arguments ({be})\n- modal conversion ({jai})\n- name (LA)\n- number sumti (LI)\n- operator-to-selbri ({nu'a})\n- pro-bridi (GOhA)\n- quote (QUOTE, {la'oi}, {ra'oi}, or {zo'oi})\n- scalar-negated sumti (NAhE)\n- scalar-negated tanru unit (NAhE)\n- scalar-negated term (NAhE)\n- sumti relative phrase ({vu'o})\n- sumti-to-selbri ({me})\n- termset connection continuation ({pe'e})\n- FIhOI adverbial ({fi'oi})\n- NA KU term (NA)\n- NA term (NA)\n- SOI adverbial ({fi'oi}, {soi}, or {xoi})\n- forethought selbri connective (GUhA)\n- place tag (FA)\n- sumti (PRO-SUMTI)\n- tag (BAI, CAhA, CUhE, FA, FAhA, MOhI, NAhE, PA, PU, SE, TAhE, VA, VEhA, VIhA, ZAhO, ZEhA, ZI, {fe'e}, {fi'o}, {ki}, {mo'e}, {ni'e}, or {vei})\n- tanru unit (SELBRI WORD, GOhA, or {me'oi})\n- term connection (NA, NAhE, SE, {cu}, {pe'o}, or {vau})\n- termset continuation ({ce'e})\n- forethought connective (GA or JA)\n- negated selbri (NA)\n- paragraph statement ({i})\n- termset ({nu'i})\n- forethought bridi connection (NA or {ke})\n- {zo'u} [continues prenex]\n- paragraph (NIhO)\n- end of input [ends term connection, termset connection, prenex, or bridi tail]";

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
    let run = capture_cli(&["jbotci", "gentufa", MORPHOLOGY_MULTI_ERROR_SOURCE]);

    assert_eq!(run.status, CliStatus::Failure);
    assert!(run.stdout.is_empty());
    assert_eq!(run.stderr, expected_morphology_stderr(2));
    assert!(!run.stderr.contains("syntax."));

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
    for command in ["gentufa", "vlasei"] {
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
            show_glosses: false,
            show_compounds: true,
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
        show_glosses: false,
        show_compounds: true,
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
        show_glosses: false,
        show_compounds: true,
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
