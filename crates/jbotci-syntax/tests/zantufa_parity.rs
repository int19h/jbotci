#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_dialect::parse_dialect_definition;
use jbotci_morphology::{
    MorphologyOptions, WordLike, segment_words_with_modifiers_with_options_and_source_id,
};
use jbotci_syntax::{
    ParseOptions, SyntaxError, SyntaxParse,
    parse_syntax_tree_generated_model_with_source_and_options_attempt,
};
use serde_json::Value;

const CAPTURED_ZANTUFA_EXPECTATIONS: &str =
    include_str!("../../../tests/fixtures/zantufa/upstream-parity.json");

#[test]
#[requires(true)]
#[ensures(true)]
fn captured_zantufa_cases_match_parser_policy() {
    std::thread::Builder::new()
        .name("zantufa-parity".to_owned())
        .stack_size(32 * 1024 * 1024)
        .spawn(captured_zantufa_cases_match_parser_policy_on_large_stack)
        .expect("spawn zantufa parity test thread")
        .join()
        .expect("zantufa parity test thread should not panic");
}

#[requires(true)]
#[ensures(true)]
fn captured_zantufa_cases_match_parser_policy_on_large_stack() {
    let fixture: Value =
        serde_json::from_str(CAPTURED_ZANTUFA_EXPECTATIONS).expect("fixture JSON is valid");
    let cases = fixture
        .get("cases")
        .and_then(Value::as_array)
        .expect("fixture has cases array");

    for test_case in cases {
        let id = test_case
            .get("id")
            .and_then(Value::as_str)
            .expect("case has id");
        let source = test_case
            .get("source")
            .and_then(Value::as_str)
            .expect("case has source");
        let upstream_accept = test_case
            .get("upstreamAccept")
            .and_then(Value::as_bool)
            .expect("case has upstreamAccept");

        let default_parse = parse_generated(source, &ParseOptions::default());
        assert_acceptance(
            id,
            "default",
            &default_parse,
            test_case
                .get("defaultAccept")
                .and_then(Value::as_bool)
                .expect("case has defaultAccept"),
        );
        if let Ok(parse) = &default_parse {
            assert_expected_warnings(id, "default", parse, &test_case["defaultWarnings"]);
        }

        let zantufa_options = zantufa_options();
        let zantufa_parse = parse_generated(source, &zantufa_options);
        assert_acceptance(
            id,
            "zantufa",
            &zantufa_parse,
            test_case
                .get("zantufaAccept")
                .and_then(Value::as_bool)
                .expect("case has zantufaAccept"),
        );
        assert_eq!(
            test_case
                .get("zantufaAccept")
                .and_then(Value::as_bool)
                .expect("case has zantufaAccept"),
            upstream_accept,
            "{id} must preserve the pinned upstream acceptance result"
        );
        if let Ok(parse) = zantufa_parse {
            assert_expected_warnings(id, "zantufa", &parse, &test_case["zantufaWarnings"]);
            assert_shape_markers(id, &parse, &test_case["shapeMarkers"]);
        }
    }
}

#[requires(!source.is_empty())]
#[ensures(true)]
fn parse_generated(source: &str, options: &ParseOptions) -> Result<SyntaxParse, SyntaxError> {
    let words = segment_words(source);
    parse_syntax_tree_generated_model_with_source_and_options_attempt(&words, source, options)
        .result
}

#[requires(!source.is_empty())]
#[ensures(!ret.is_empty())]
fn segment_words(source: &str) -> Vec<WordLike> {
    segment_words_with_modifiers_with_options_and_source_id(
        source,
        &MorphologyOptions::default(),
        None,
    )
    .expect("fixture source should segment")
}

#[requires(true)]
#[ensures(true)]
fn zantufa_options() -> ParseOptions {
    let dialect = parse_dialect_definition("(zantufa)").expect("zantufa dialect is valid");
    ParseOptions::default().with_dialect_definition(&dialect)
}

#[requires(!id.is_empty())]
#[requires(!dialect_name.is_empty())]
#[ensures(true)]
fn assert_acceptance(
    id: &str,
    dialect_name: &str,
    parse: &Result<SyntaxParse, SyntaxError>,
    expected: bool,
) {
    assert_eq!(
        parse.is_ok(),
        expected,
        "{id} {dialect_name} acceptance mismatch: {parse:?}"
    );
}

#[requires(!id.is_empty())]
#[requires(!dialect_name.is_empty())]
#[ensures(true)]
fn assert_expected_warnings(
    id: &str,
    dialect_name: &str,
    parse: &SyntaxParse,
    expected_warnings: &Value,
) {
    let actual = parse
        .warnings
        .iter()
        .map(|warning| format!("{:?}", warning.kind))
        .collect::<Vec<_>>();
    for warning in expected_warning_names(expected_warnings) {
        assert!(
            actual.iter().any(|actual| actual == &warning),
            "{id} {dialect_name} missing warning {warning}; actual warnings: {actual:?}"
        );
    }
}

#[requires(true)]
#[ensures(true)]
fn assert_shape_markers(id: &str, parse: &SyntaxParse, expected_markers: &Value) {
    let tree_json = serde_json::to_string(&parse.parse_tree).expect("parse tree serializes");
    for marker in expected_warning_names(expected_markers) {
        assert!(
            tree_json.contains(&marker),
            "{id} parse tree missing marker {marker}; tree: {tree_json}"
        );
    }
}

#[requires(true)]
#[ensures(true)]
fn expected_warning_names(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("expected warning/marker list is an array")
        .iter()
        .map(|item| {
            item.as_str()
                .expect("expected warning/marker is a string")
                .to_owned()
        })
        .collect()
}
