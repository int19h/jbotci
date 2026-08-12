//! Semantic regressions for the Zantufa MEX lowering paths.

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_dialect::parse_dialect_definition;
use jbotci_morphology::{
    MorphologyOptions, segment_words_with_modifiers_with_options_and_source_id,
};
use jbotci_semantics::{
    SemanticBuildOptions, SemanticGraph, build_generated_semantic_graph_with_dictionary_and_options,
};
use jbotci_source::SourceId;
use jbotci_syntax::{ParseOptions, parse_syntax_tree_generated_model_with_source_and_options};

#[requires(!source.is_empty())]
#[requires(!dialect_source.is_empty())]
#[ensures(ret.objects.contains_key(&ret.root))]
fn build_graph(source: &str, dialect_source: &str) -> SemanticGraph {
    let dialect = parse_dialect_definition(dialect_source).expect("test dialect");
    let words = segment_words_with_modifiers_with_options_and_source_id(
        source,
        &MorphologyOptions::default().with_dialect_definition(&dialect),
        Some(SourceId("<zantufa-mex-semantics>".to_owned())),
    )
    .expect("test morphology");
    let syntax = parse_syntax_tree_generated_model_with_source_and_options(
        &words,
        source,
        &ParseOptions::default().with_dialect_definition(&dialect),
    )
    .expect("test syntax");
    build_generated_semantic_graph_with_dictionary_and_options(
        &syntax,
        SemanticBuildOptions {
            source_text: Some(source),
            story_time: false,
        },
        jbotci_dictionary_data::english(),
    )
    .expect("test semantics")
}

#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(true)]
fn add_expression(graph: &SemanticGraph) -> (Vec<String>, bool) {
    let value = serde_json::to_value(graph).expect("serialize graph");
    value["objects"]
        .as_object()
        .expect("object map")
        .values()
        .find_map(|object| {
            (object["type"] == "mathExpression" && object["operator"] == "add").then(|| {
                let operands = object["operands"]
                    .as_array()
                    .expect("operator operands")
                    .iter()
                    .map(|operand| operand.as_str().expect("operand id").to_owned())
                    .collect();
                (operands, !object["scalarNegation"].is_null())
            })
        })
        .expect("typed add expression")
}

#[test]
#[requires(true)]
#[ensures(true)]
fn zantufa_bihe_operator_preserves_se_and_nahe_semantics() {
    let converted = build_graph("li ke pa ke'e bi'e se su'i re", "(zantufa)");
    let (converted_operands, converted_negated) = add_expression(&converted);
    assert!(!converted_negated);
    assert!(converted_operands[0] > converted_operands[1]);

    let negated = build_graph("li ke pa ke'e bi'e na'e su'i re", "(zantufa)");
    let (negated_operands, negated_scalar) = add_expression(&negated);
    assert!(negated_scalar);
    assert!(negated_operands[0] < negated_operands[1]);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn zantufa_reverse_polish_operator_preserves_se_and_nahe_semantics() {
    let dialect = "(zantufa +zantufa-mex-reinterpretation)";
    let converted = build_graph("li fu'a pa boi re boi se su'i ku'e", dialect);
    let (converted_operands, converted_negated) = add_expression(&converted);
    assert!(!converted_negated);
    assert!(converted_operands[0] > converted_operands[1]);

    let negated = build_graph("li fu'a pa boi re boi na'e su'i ku'e", dialect);
    let (negated_operands, negated_scalar) = add_expression(&negated);
    assert!(negated_scalar);
    assert!(negated_operands[0] < negated_operands[1]);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn se_transparent_scalar_negation_is_retained_on_standard_mex() {
    let graph = build_graph("li pa se na'e su'i re", "(zantufa)");
    let (operands, scalar_negated) = add_expression(&graph);
    assert!(scalar_negated);
    assert!(operands[0] > operands[1]);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn zantufa_bare_number_quantifier_remains_an_integer() {
    let graph = build_graph("pa mlatu cu blabi", "(zantufa)");
    let value = serde_json::to_value(graph).expect("serialize graph");
    assert!(
        value["objects"]
            .as_object()
            .expect("object map")
            .values()
            .any(|object| object["type"] == "quantity" && object["value"]["integer"] == 1)
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn zantufa_wide_qualified_quantifier_remains_an_opaque_math_expression() {
    for source in ["li la'e pa lu'u lo'o", "li na'e bo pa lu'u lo'o"] {
        let graph = build_graph(source, "(zantufa +zantufa-mex-reinterpretation)");
        let value = serde_json::to_value(graph).expect("serialize graph");
        assert!(
            value["objects"]
                .as_object()
                .expect("object map")
                .values()
                .filter(|object| object["type"] == "quantity")
                .all(|object| object["value"].get("integer").is_none()),
            "wide qualifier must prevent integer unwrapping: {source}"
        );
    }
}
