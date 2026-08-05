//! Structural validation for the experimental smusni S-expression renderer.
//!
//! These tests deliberately avoid byte-exact output expectations. They parse
//! every rendered document and validate the notation's binding, reference,
//! diagnostics, modal, and document-shape invariants instead.

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};

use std::collections::BTreeSet;
use std::path::PathBuf;

use jbotci_dialect::DialectDefinition;
use jbotci_morphology::{
    MorphologyOptions, WordLike, segment_words_with_modifiers_with_options_and_source_id,
};
use jbotci_semantics::completeness::corpus::CORPUS_DOCS;
use jbotci_semantics::model::{
    Actuality, ActualityKind, AnchorRelation, ArgumentValueKind, Aspect, EventualityNode,
    IndexicalKind, MathLiteral, ParameterRole, QuantityScale, QuantityValue, QuestionSlot,
    QuestionSlotRole, Recurrence, RecurrenceKind, ReferentCategory, ScopeDependence,
    SemanticGraphData, SemanticObject, SemanticObjectId, Subscript, UtteranceForce,
};
use jbotci_semantics::notation::sexpr::{Datum, parse_document, parse_v0_document};
use jbotci_semantics::notation::word_cards::build_word_cards;
use jbotci_semantics::{
    SemanticBuildOptions, SemanticGraph,
    build_generated_semantic_graph_with_dictionary_and_options, render_smusni,
    render_smusni_detailed, render_smusni_with_word_cards,
};
use jbotci_source::SourceId;
use jbotci_syntax::{ParseOptions, parse_syntax_tree_generated_model_with_source_and_options};

/// One successfully built graph together with its morphology result.
#[invariant(graph.objects.contains_key(&graph.root))]
#[derive(Debug)]
struct BuiltInput {
    graph: SemanticGraph,
    words: Vec<WordLike>,
}

/// Resolve a retained phase-B source fixture.
#[requires(!doc.is_empty())]
#[ensures(true)]
fn fixture(doc: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/phaseb_corpus");
    path.push(format!("{doc}.lojban"));
    path
}

/// Build the exact production morphology/syntax/semantics pipeline.
#[requires(!text.trim().is_empty())]
#[ensures(ret.graph.objects.contains_key(&ret.graph.root))]
fn build_input(text: &str, source_name: &str) -> BuiltInput {
    let text = text.trim();
    let dialect = DialectDefinition::default();
    let morphology_options = MorphologyOptions::default().with_dialect_definition(&dialect);
    let syntax_options = ParseOptions::default().with_dialect_definition(&dialect);
    let source_id = Some(SourceId(format!("<smusni-test:{source_name}>")));
    let words = segment_words_with_modifiers_with_options_and_source_id(
        text,
        &morphology_options,
        source_id,
    )
    .unwrap_or_else(|error| panic!("morphology {source_name}: {error}"));
    let parsed =
        parse_syntax_tree_generated_model_with_source_and_options(&words, text, &syntax_options)
            .unwrap_or_else(|error| panic!("syntax {source_name}: {error}"));
    let graph = build_generated_semantic_graph_with_dictionary_and_options(
        &parsed,
        SemanticBuildOptions {
            source_text: Some(text),
            story_time: false,
        },
        jbotci_dictionary_data::english(),
    )
    .unwrap_or_else(|error| panic!("semantics {source_name}: {error}"));
    new!(BuiltInput {
        graph: graph,
        words: words,
    })
}

/// Rebuild a retained structural corpus document from its Lojban source.
#[requires(!doc.is_empty())]
#[ensures(ret.graph.objects.contains_key(&ret.graph.root))]
fn corpus_input(doc: &str) -> BuiltInput {
    let text = std::fs::read_to_string(fixture(doc))
        .unwrap_or_else(|error| panic!("read {doc}.lojban: {error}"));
    build_input(&text, doc)
}

/// Replace one object while revalidating the graph wrapper.
#[requires(graph.objects.contains_key(&id))]
#[requires(object.object_kind() == id.object_kind())]
#[ensures(ret.objects.contains_key(&id))]
fn replace_object(
    graph: SemanticGraph,
    id: SemanticObjectId,
    object: SemanticObject,
) -> SemanticGraph {
    let data = graph.into_data();
    let mut objects = data.objects;
    objects.insert(id, object);
    SemanticGraph::from_data(data!(SemanticGraph { objects, ..data }))
}

/// Insert one otherwise unreachable support object for a mutation witness.
#[requires(!graph.objects.contains_key(&id))]
#[requires(object.object_kind() == id.object_kind())]
#[ensures(ret.objects.contains_key(&id))]
fn insert_object(
    graph: SemanticGraph,
    id: SemanticObjectId,
    object: SemanticObject,
) -> SemanticGraph {
    let data = graph.into_data();
    let mut objects = data.objects;
    objects.insert(id, object);
    SemanticGraph::from_data(data!(SemanticGraph { objects, ..data }))
}

/// Locate the one matrix generated event in a simple focused graph.
#[requires(true)]
#[ensures(ret.is_none_or(|id| graph.objects.contains_key(&id)))]
fn first_generated_event(graph: &SemanticGraph) -> Option<SemanticObjectId> {
    graph.objects.iter().find_map(|(id, object)| {
        object
            .as_eventuality()
            .is_some_and(|node| node.denotation.is_generated_bound())
            .then_some(*id)
    })
}

/// Replace the first generated event with an invariant-preserving mutation.
#[requires(first_generated_event(&graph).is_some())]
#[ensures(ret.objects.len() == old(graph.objects.len()))]
fn mutate_generated_event(
    graph: SemanticGraph,
    update: impl FnOnce(EventualityNode) -> EventualityNode,
) -> SemanticGraph {
    let id = first_generated_event(&graph).expect("focused graph has a generated event");
    let mut object = graph.objects[&id].clone();
    object.update_eventuality(update);
    replace_object(graph, id, object)
}

/// Parse and validate the outer document shape and newline policy.
#[requires(text.ends_with('\n') && !text.ends_with("\n\n"))]
#[ensures(ret.as_list().is_some_and(|items| items.len() >= 3))]
fn parse_smusni(text: &str) -> Datum {
    assert_eq!(
        text.bytes().rev().take_while(|byte| *byte == b'\n').count(),
        1
    );
    let datum = parse_document(text).expect("renderer output must be one parseable datum");
    parse_v0_document(text).expect("renderer output must satisfy the current typed grammar");
    let items = datum.as_list().expect("Smusni document must be a list");
    assert_eq!(items.first().and_then(Datum::as_atom), Some("Smusni"));
    assert_eq!(items.get(1).and_then(Datum::as_integer), Some("0"));
    assert!(items.len() >= 3);
    datum
}

/// Read a binder entry's declared variable without treating it as a use.
#[requires(entry.as_list().is_some_and(|items| items.len() >= 2))]
#[ensures(ret.starts_with('$'))]
fn binding_name(entry: &Datum) -> String {
    let items = entry.as_list().expect("binding entry is a list");
    let name = items[0].as_atom().expect("binding name is an atom");
    assert!(name.starts_with('$'), "invalid binding name {name:?}");
    name.to_owned()
}

/// Validate ordinary lexical scope. Immutable environments make dynamic
/// operator siblings separate accessibility regions unless an enclosing
/// lexical binder explicitly covers them.
#[requires(true)]
#[ensures(true)]
fn validate_bindings(datum: &Datum, bound: &BTreeSet<String>) {
    let Some(items) = datum.as_list() else {
        if let Some(atom) = datum.as_atom() {
            if atom.starts_with('$') {
                assert!(bound.contains(atom), "unbound variable {atom}");
            }
        }
        return;
    };
    let Some(head) = items.first().and_then(Datum::as_atom) else {
        for item in items {
            validate_bindings(item, bound);
        }
        return;
    };
    match head {
        "Let" | "Bind" => validate_let(items, bound, false),
        "LetRec" => validate_let(items, bound, true),
        "λ" => validate_declaration_binder(items, bound),
        "Utterance" => validate_utterance(items, bound),
        _ => {
            for item in &items[1..] {
                validate_bindings(item, bound);
            }
        }
    }
}

/// Validate sequential `Let` or simultaneous `LetRec` entries.
#[requires(items.first().and_then(Datum::as_atom).is_some_and(|head| matches!(head, "Let" | "Bind" | "LetRec")))]
#[ensures(true)]
fn validate_let(items: &[Datum], outer: &BTreeSet<String>, recursive: bool) {
    assert_eq!(items.len(), 3, "Let forms have bindings and body");
    let bindings = items[1].as_list().expect("Let bindings are a list");
    let all_names = bindings.iter().map(binding_name).collect::<BTreeSet<_>>();
    assert_eq!(all_names.len(), bindings.len(), "duplicate Let binder");
    let mut available = outer.clone();
    if recursive {
        available.extend(all_names.iter().cloned());
    }
    for binding in bindings {
        let entry = binding.as_list().expect("Let entry is a list");
        assert!(entry.len() >= 3, "Let entry includes type and value");
        for value in &entry[2..] {
            validate_bindings(value, &available);
        }
        if !recursive {
            available.insert(binding_name(binding));
        }
    }
    let mut body_scope = outer.clone();
    body_scope.extend(all_names);
    validate_bindings(&items[2], &body_scope);
}

/// Validate declaration-list binders used by functions and quantifiers.
#[requires(items.len() >= 3)]
#[ensures(true)]
fn validate_declaration_binder(items: &[Datum], outer: &BTreeSet<String>) {
    let bindings = items[1].as_list().expect("declarations are a list");
    let mut scoped = outer.clone();
    for binding in bindings {
        let entry = binding.as_list().expect("declaration is a list");
        assert_eq!(entry.len(), 2, "declaration has variable and sort");
        scoped.insert(binding_name(binding));
    }
    for body in &items[2..] {
        validate_bindings(body, &scoped);
    }
}

/// Validate the utterance-token binder.
#[requires(items.first().and_then(Datum::as_atom) == Some("Utterance"))]
#[ensures(true)]
fn validate_utterance(items: &[Datum], outer: &BTreeSet<String>) {
    assert!(items.len() >= 3);
    let declarations = items[1].as_list().expect("utterance binder is a list");
    assert_eq!(declarations.len(), 1);
    let name = binding_name(&declarations[0]);
    let mut scoped = outer.clone();
    scoped.insert(name);
    for field in &items[2..] {
        validate_bindings(field, &scoped);
    }
}

/// Compact output never leaks the retired escaped graph-reference spelling.
/// Whole-document `%id` identity closure is enforced by `parse_v0_document`.
#[requires(true)]
#[ensures(true)]
fn validate_no_legacy_graph_references(datum: &Datum) {
    if let Some(atom) = datum.as_atom() {
        assert!(
            !atom.starts_with("|@"),
            "compact output contains an undeclared identity edge {atom}"
        );
        return;
    }
    if let Some(items) = datum.as_list() {
        for item in items {
            validate_no_legacy_graph_references(item);
        }
    }
}

/// Count forms with a given typed head.
#[requires(!head.is_empty())]
#[ensures(true)]
fn count_forms(datum: &Datum, head: &str) -> usize {
    let own = usize::from(
        datum
            .as_list()
            .and_then(|items| items.first())
            .and_then(Datum::as_atom)
            == Some(head),
    );
    own + datum
        .as_list()
        .into_iter()
        .flat_map(|items| items.iter())
        .map(|item| count_forms(item, head))
        .sum::<usize>()
}

/// Recover numbered places from one canonical application. Plain operands
/// advance from the preceding `:n`; `:Eventuality` is a distinct row marker.
#[requires(datum.as_list().is_some_and(|items| items.len() >= 2))]
#[ensures(true)]
fn numbered_application_places(datum: &Datum) -> BTreeSet<usize> {
    let items = datum.as_list().expect("application is a list");
    let mut places = BTreeSet::new();
    let mut next = 1usize;
    let mut index = 1usize;
    while index < items.len() {
        if items[index].as_atom() == Some(":Eventuality") {
            index += 2;
            continue;
        }
        if let Some(place) = items[index]
            .as_atom()
            .and_then(|atom| atom.strip_prefix(':'))
            .and_then(|digits| digits.parse::<usize>().ok())
        {
            next = place;
            index += 1;
        }
        assert!(index < items.len(), "place marker has a value");
        places.insert(next);
        next += 1;
        index += 1;
    }
    places
}

/// Return every form with a requested head.
#[requires(!head.is_empty())]
#[ensures(true)]
fn collect_forms<'a>(datum: &'a Datum, head: &str, out: &mut Vec<&'a Datum>) {
    let Some(items) = datum.as_list() else {
        return;
    };
    if items.first().and_then(Datum::as_atom) == Some(head) {
        out.push(datum);
    }
    for item in items {
        collect_forms(item, head, out);
    }
}

/// Whether a parsed typed fallback contains one named field.
#[requires(!name.is_empty())]
#[ensures(true)]
fn contains_field(datum: &Datum, name: &str) -> bool {
    let Some(items) = datum.as_list() else {
        return false;
    };
    (items.first().and_then(Datum::as_atom) == Some("Field")
        && items
            .get(1)
            .and_then(|value| value.as_string().or_else(|| value.as_atom()))
            .is_some_and(|field| field.eq_ignore_ascii_case(name)))
        || items.iter().any(|item| contains_field(item, name))
}

/// Collect string data after parser round-trip for escaping integration checks.
#[requires(true)]
#[ensures(true)]
fn collect_strings<'a>(datum: &'a Datum, out: &mut Vec<&'a str>) {
    match datum {
        Datum::String(value) => out.push(value),
        Datum::List(items) => {
            for item in items {
                collect_strings(item, out);
            }
        }
        _ => {}
    }
}

/// Require a compact family when the graph passed compact-scope planning, or a
/// complete whole-document fallback when the live builder graph is scope-invalid.
#[requires(!head.is_empty())]
#[ensures(true)]
fn assert_compact_family_or_typed_graph(datum: &Datum, head: &str, witness: &str) {
    assert!(
        count_forms(datum, head) > 0 || count_forms(datum, "TypedGraph") == 1,
        "{witness} must render {head} or a complete TypedGraph"
    );
}

/// Run every graph-independent structural check for one render.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(true)]
fn validate_render(graph: &SemanticGraph, text: &str) -> Datum {
    assert!(!text.contains("SEMANTIC DOCUMENT"));
    assert!(!text.contains("DECLARATION"));
    assert!(!text.contains("ID PREFIXES"));
    assert!(!text.contains("NOT COMPUTED"));
    let datum = parse_smusni(text);
    validate_bindings(&datum, &BTreeSet::new());
    validate_no_legacy_graph_references(&datum);
    for forbidden in ["WithWarnings", "Warnings", "Warning"] {
        assert_eq!(count_forms(&datum, forbidden), 0);
    }
    datum
}

#[test]
#[requires(true)]
#[ensures(true)]
fn frozen_corpus_is_total_deterministic_and_structurally_valid() {
    for doc in CORPUS_DOCS {
        let input = corpus_input(doc);
        let first = render_smusni(&input.graph);
        let second = render_smusni(&input.graph);
        assert_eq!(first, second, "nondeterministic document {doc}");
        validate_render(&input.graph, &first);
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn focused_semantic_families_are_exercised_without_output_goldens() {
    let cases = [
        ("deleted-place", "mi dunda zi'o ti"),
        ("paragraph", "ni'o mi klama"),
        ("relation-question", "ti mo zdani"),
        ("quotation", "mi cusku lu mi prami do li'u"),
        ("hostile-quotation", r#"mi cusku zoi gy. a"b\c(d);e .gy"#),
        ("relative-clause", "mi klama lo zarci poi do nelci ke'a"),
        ("connective", "ganai broda gi brode .a brodi"),
        ("abstraction", "mi djuno lo du'u do klama"),
        (
            "termset",
            "ci gerku ce'e re nanmu cu batci .i nu'i ci gerku re nanmu nu'u cu batci",
        ),
        (
            "respectively-distribution",
            "la .djan. fa'u la .frank. cusku nu'i fa'ugi bau la .lojban. nu'u gi bai tu'a la .djordj.",
        ),
        (
            "respectively-values",
            "la djeimyz. fa'u la djordj. prami la meris. fa'u la martas.",
        ),
        ("math", "li pa su'i re du li ci"),
        (
            "displayed",
            "mi pu klama lo zarci .i le nanmu poi mi viska ke'a cu dunda lo cukta mi .i mi jinvi lo du'u le cukta cu melbi .i ku'i mi na djica lo nu mi cilre",
        ),
        ("tanru", "ti blanu zdani"),
    ];
    let mut observed_heads = BTreeSet::new();
    for (name, text) in cases {
        let input = build_input(text, name);
        match name {
            "relation-question" => assert!(
                input
                    .graph
                    .objects
                    .values()
                    .any(|object| object.as_question().is_some()),
                "question witness must construct a typed question",
            ),
            "quotation" | "hostile-quotation" => assert!(
                input
                    .graph
                    .objects
                    .values()
                    .any(|object| object.as_sign().is_some()),
                "quotation witness must construct a typed sign",
            ),
            "math" => assert!(
                input
                    .graph
                    .objects
                    .values()
                    .any(|object| object.as_math_expression().is_some()),
                "math witness must construct typed math",
            ),
            "displayed" => assert!(
                input
                    .graph
                    .objects
                    .values()
                    .any(|object| object.as_displayed_content().is_some()),
                "display witness must construct displayed content",
            ),
            _ => {}
        }
        let rendered = render_smusni(&input.graph);
        let datum = validate_render(&input.graph, &rendered);
        match name {
            "deleted-place" => assert_eq!(count_forms(&datum, "DropPlace"), 1),
            "paragraph" => assert_eq!(count_forms(&datum, "NewTopic"), 1),
            "relation-question" => assert_compact_family_or_typed_graph(&datum, "Ask", name),
            "quotation" => assert_eq!(count_forms(&datum, "TypedGraph"), 1),
            "hostile-quotation" => {
                let mut strings = Vec::new();
                collect_strings(&datum, &mut strings);
                assert!(strings.iter().any(|value| {
                    value.contains('"')
                        && value.contains('\\')
                        && value.contains('(')
                        && value.contains(';')
                }));
            }
            "relative-clause" => {
                assert_eq!(count_forms(&datum, "Bind"), 1);
                assert_eq!(count_forms(&datum, "Refer"), 1);
                assert_eq!(count_forms(&datum, "∧"), 1);
            }
            "termset" | "respectively-distribution" | "respectively-values" => {
                assert_eq!(count_forms(&datum, "TypedGraph"), 1)
            }
            "math" | "displayed" | "abstraction" => {
                assert_compact_family_or_typed_graph(&datum, "Assert", name)
            }
            "tanru" => assert_compact_family_or_typed_graph(&datum, "Tanru", name),
            _ => {}
        }
        for retired in [
            "WithWarnings",
            "Warnings",
            "Warning",
            "Modal",
            "Mode",
            "Lo",
            "Le",
            "La",
            "Relative",
            "Quantify",
            "Respectively",
            "RespectivelyValue",
            "OfKind",
            "Sequence",
            "ParagraphBoundary",
            "Quote",
            "Parenthetical",
            "Subordinated",
        ] {
            assert_eq!(
                count_forms(&datum, retired),
                0,
                "retired form {retired} in {name}"
            );
        }
        for head in [
            "DropPlace",
            "Ask",
            "NewTopic",
            "Bind",
            "Refer",
            "→",
            "Tanru",
            "TypedGraph",
        ] {
            if count_forms(&datum, head) > 0 {
                observed_heads.insert(head);
            }
        }
    }
    for required in ["DropPlace", "NewTopic", "Bind", "Refer", "TypedGraph"] {
        assert!(
            observed_heads.contains(required),
            "missing structural family {required}"
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn unsupported_utterance_forces_use_typed_fallback_not_retired_forms() {
    let input = build_input("mi klama", "unsupported-utterance-forces");
    let utterance = input.graph.root;
    assert!(
        input.graph.objects[&utterance].as_utterance().is_some(),
        "simple text root is an utterance",
    );
    for force in [
        UtteranceForce::Quote,
        UtteranceForce::Parenthetical,
        UtteranceForce::Subordinated,
    ] {
        let mut object = input.graph.objects[&utterance].clone();
        object.update_utterance(|node| node.with_data(data! { force: force }));
        let graph = replace_object(input.graph.clone(), utterance, object);
        let datum = validate_render(&graph, &render_smusni(&graph));
        assert_eq!(count_forms(&datum, "TypedGraph"), 1);
        for retired in ["Quote", "Parenthetical", "Subordinated"] {
            assert_eq!(count_forms(&datum, retired), 0);
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn modal_place_labels_match_the_actual_graph_maps() {
    for (name, text) in [
        ("converted-modal", "mi klama sepi'o lo karce"),
        ("direct-modal", "mi klama fi'o pilno lo karce"),
    ] {
        let input = build_input(text, name);
        let expected = input
            .graph
            .objects
            .values()
            .filter_map(SemanticObject::as_predication)
            .flat_map(|predication| predication.adjuncts.iter())
            .filter(|adjunct| adjunct.relation.is_some())
            .map(|adjunct| {
                let relation = adjunct.relation.clone().expect("filtered relation");
                let places = adjunct
                    .arguments
                    .iter()
                    .filter(|(_, argument)| {
                        argument.kind == ArgumentValueKind::Filled && argument.value.is_some()
                    })
                    .map(|(place, _)| place.get())
                    .collect::<BTreeSet<_>>();
                (relation, places)
            })
            .collect::<Vec<_>>();
        assert!(
            !expected.is_empty(),
            "modal witness must have an adjunct map"
        );
        let datum = validate_render(&input.graph, &render_smusni(&input.graph));
        assert_eq!(count_forms(&datum, "Modal"), 0);
        assert_eq!(count_forms(&datum, "Joi"), 1);
        assert_eq!(count_forms(&datum, "At"), 0);
        for (relation, expected_places) in expected {
            let mut applications = Vec::new();
            collect_forms(&datum, &relation, &mut applications);
            assert_eq!(applications.len(), 1, "one {relation} modal application");
            assert_eq!(
                numbered_application_places(applications[0]),
                expected_places
            );
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn generated_event_facet_families_have_structural_witnesses() {
    let tense = build_input("mi pu klama lo zarci", "facet-time");
    let tense = validate_render(&tense.graph, &render_smusni(&tense.graph));
    assert_eq!(count_forms(&tense, "Joi"), 1);
    assert_eq!(count_forms(&tense, "purci"), 1);
    assert_eq!(count_forms(&tense, "Before"), 0);

    let cases = [
        (
            "actuality",
            Box::new(|node: EventualityNode| {
                node.with_data(data! {
                    actuality: Some(Actuality { kind: ActualityKind::Capable })
                })
            }) as Box<dyn FnOnce(EventualityNode) -> EventualityNode>,
            "Actuality",
        ),
        (
            "aspect",
            Box::new(|node: EventualityNode| {
                node.with_data(data! {
                    aspect: Some(Aspect::new("ongoing".to_owned(), None))
                })
            }),
            "Aspect",
        ),
        (
            "recurrence",
            Box::new(|node: EventualityNode| {
                node.with_data(data! {
                    recurrence: vec![Recurrence::new(
                        RecurrenceKind::Regular,
                        "roi".to_owned(),
                        None,
                        Some(QuantityValue::integer(2)),
                        None,
                        None,
                        None,
                    )]
                })
            }),
            "Recurrence",
        ),
        (
            "space",
            Box::new(|node: EventualityNode| {
                node.with_data(data! {
                    space: Some(new!(AnchorRelation {
                        relation: "at".to_owned(),
                        anchor: SemanticObjectId::referent(4),
                        sticky: false,
                        inherited: None,
                        distance: None,
                        magnitude: None,
                        scalar_negation: None,
                        motion: None,
                    }))
                })
            }),
            "Space",
        ),
    ];
    for (name, update, field) in cases {
        let input = build_input("mi klama", name);
        let graph = mutate_generated_event(input.graph.clone(), update);
        let datum = validate_render(&graph, &render_smusni(&graph));
        assert_eq!(count_forms(&datum, "TypedGraph"), 1);
        assert!(
            contains_field(&datum, field),
            "missing {field} field for {name}"
        );
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn optional_semantic_side_fields_force_representation_or_typed_graph() {
    let input = build_input("mi klama", "predication-introducer");
    let predication = input
        .graph
        .objects
        .iter()
        .find_map(|(id, object)| object.as_predication().map(|_| *id))
        .expect("simple assertion has a predication");
    let mut object = input.graph.objects[&predication].clone();
    object.update_predication(|node| {
        node.with_data(data! { introduced_by: Some("mutated".to_owned()) })
    });
    let graph = replace_object(input.graph.clone(), predication, object);
    let datum = validate_render(&graph, &render_smusni(&graph));
    assert!(contains_field(&datum, "IntroducedBy"));

    let input = build_input("ci mlatu cu jbena", "quantity-scale");
    let quantity = input
        .graph
        .objects
        .iter()
        .find_map(|(id, object)| object.as_quantity().map(|_| *id))
        .expect("cardinality has a quantity");
    let mut object = input.graph.objects[&quantity].clone();
    object.update_quantity(|node| node.with_data(data! { scale: QuantityScale::Fraction }));
    let graph = replace_object(input.graph.clone(), quantity, object);
    let datum = validate_render(&graph, &render_smusni(&graph));
    assert_eq!(count_forms(&datum, "TypedGraph"), 1);
    assert!(contains_field(&datum, "Scale"));

    let input = build_input("li pa su'i re du li ci", "math-denotes");
    let denotation = input
        .graph
        .objects
        .iter()
        .find_map(|(id, object)| {
            object
                .as_referent()
                .is_some_and(|node| {
                    node.indexical == Some(jbotci_semantics::model::IndexicalKind::Speaker)
                })
                .then_some(*id)
        })
        .expect("utterance graph has Speaker");
    let math = input
        .graph
        .objects
        .iter()
        .find_map(|(id, object)| {
            object
                .as_math_expression()
                .is_some_and(|node| {
                    matches!(
                        node.kind.as_data(),
                        data!(jbotci_semantics::model::MathExpressionNodeKind::Literal { .. })
                    )
                })
                .then_some(*id)
        })
        .expect("mekso has a math literal");
    let mut object = input.graph.objects[&math].clone();
    object.update_math_expression(|node| {
        let literal = match node.kind.as_data() {
            data!(jbotci_semantics::model::MathExpressionNodeKind::Literal { literal, .. }) => {
                literal.clone()
            }
            _ => return node,
        };
        node.with_data(data! {
            kind: new!(jbotci_semantics::model::MathExpressionNodeKind::Literal {
                literal,
                denotes: Some(denotation),
            })
        })
    });
    let graph = replace_object(input.graph.clone(), math, object);
    let datum = validate_render(&graph, &render_smusni(&graph));
    assert!(contains_field(&datum, "Denotes"));

    let input = build_input("ti mo zdani", "question-role");
    let question = input
        .graph
        .objects
        .iter()
        .find_map(|(id, object)| object.as_question().map(|_| *id))
        .expect("relation question has a Question object");
    let mut object = input.graph.objects[&question].clone();
    object.update_question(|node| {
        let parameter = node.slots[0].parameter().expect("relation slot parameter");
        node.with_data(data! {
            slots: vec![QuestionSlot::homogeneous(
                parameter,
                QuestionSlotRole::RespectiveSlot,
            )]
        })
    });
    let graph = replace_object(input.graph.clone(), question, object);
    let datum = validate_render(&graph, &render_smusni(&graph));
    assert_eq!(count_forms(&datum, "TypedGraph"), 1);

    let input = build_input("ti mo zdani", "parameter-subscript");
    let parameter = input
        .graph
        .objects
        .iter()
        .find_map(|(id, object)| {
            object
                .as_parameter()
                .is_some_and(|node| node.introduced_by == "mo")
                .then_some(*id)
        })
        .expect("relation question has a mo parameter");
    let math = SemanticObjectId::math_expression(10_000);
    let graph = insert_object(
        input.graph.clone(),
        math,
        SemanticObject::math_expression(
            None,
            Vec::new(),
            Some(MathLiteral::integer(1)),
            None,
            Vec::new(),
        ),
    );
    let mut object = graph.objects[&parameter].clone();
    object.update_parameter(|node| {
        node.with_data(data! {
            subscript: Some(Subscript::new(math, "xi".to_owned(), None))
        })
    });
    let graph = replace_object(graph, parameter, object);
    let datum = validate_render(&graph, &render_smusni(&graph));
    assert_eq!(count_forms(&datum, "TypedGraph"), 1);
    assert!(contains_field(&datum, "Subscript"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn binder_and_projection_recognizers_require_complete_typed_shapes() {
    let input = build_input("ro mlatu cu jbena", "quantifier-domain-import");
    let ordinary = validate_render(&input.graph, &render_smusni(&input.graph));
    assert_eq!(count_forms(&ordinary, "Every"), 1);
    assert_eq!(count_forms(&ordinary, "Import"), 0);
    let variable = input
        .graph
        .objects
        .values()
        .find_map(|object| {
            let formula = object.as_formula()?;
            match formula.as_data() {
                data!(jbotci_semantics::model::FormulaNode::Quantified(node)) => {
                    Some(node.variable)
                }
                _ => None,
            }
        })
        .expect("ordinary universal has a bound variable");
    let mut object = input.graph.objects[&variable].clone();
    object.update_referent(|node| {
        node.with_data(data! {
            category: ReferentCategory::Constant,
            scope_dependence: Some(ScopeDependence::fixed()),
        })
    });
    let graph = replace_object(input.graph.clone(), variable, object);
    let datum = validate_render(&graph, &render_smusni(&graph));
    assert_eq!(count_forms(&datum, "TypedGraph"), 1);
    assert!(contains_field(&datum, "Category"));

    let input = build_input("ti mo zdani", "question-parameter-fields");
    let parameter = input
        .graph
        .objects
        .iter()
        .find_map(|(id, object)| {
            object
                .as_parameter()
                .is_some_and(|node| node.introduced_by == "mo")
                .then_some(*id)
        })
        .expect("relation question has a mo parameter");
    let mut object = input.graph.objects[&parameter].clone();
    object.update_parameter(|node| node.with_data(data! { introduced_by: "mutated".to_owned() }));
    let graph = replace_object(input.graph.clone(), parameter, object);
    let datum = validate_render(&graph, &render_smusni(&graph));
    assert_eq!(count_forms(&datum, "TypedGraph"), 1);
    assert!(contains_field(&datum, "IntroducedBy"));

    // Question parameters have a graph-level role invariant, so exercise role
    // fidelity on an abstraction parameter whose alternate entity role remains
    // a valid SemanticGraph while invalidating exact abstraction recognition.
    let input = build_input("lo ka ce'u gleki", "abstraction-parameter-role");
    let parameter = input
        .graph
        .objects
        .iter()
        .find_map(|(id, object)| {
            object
                .as_parameter()
                .is_some_and(|node| node.role == ParameterRole::PropertySlot)
                .then_some(*id)
        })
        .expect("ka abstraction has a property-slot parameter");
    let mut object = input.graph.objects[&parameter].clone();
    object
        .update_parameter(|node| node.with_data(data! { role: ParameterRole::RelativeClauseHead }));
    let graph = replace_object(input.graph.clone(), parameter, object);
    let datum = validate_render(&graph, &render_smusni(&graph));
    assert_compact_family_or_typed_graph(&datum, "Object", "abstraction parameter role");
    assert!(contains_field(&datum, "Role"));

    let input = build_input(
        "la .djan. fa'u la .frank. cusku nu'i fa'ugi bau la .lojban. nu'u gi bai tu'a la .djordj.",
        "respectively-parameter-fields",
    );
    let parameter = input
        .graph
        .objects
        .iter()
        .find_map(|(id, object)| {
            object
                .as_parameter()
                .is_some_and(|node| node.introduced_by == "fa'u")
                .then_some(*id)
        })
        .expect("respectively distribution has a fa'u slot");
    let mut object = input.graph.objects[&parameter].clone();
    object.update_parameter(|node| node.with_data(data! { introduced_by: "mutated".to_owned() }));
    let graph = replace_object(input.graph.clone(), parameter, object);
    let datum = validate_render(&graph, &render_smusni(&graph));
    assert_eq!(count_forms(&datum, "TypedGraph"), 1);
    assert!(contains_field(&datum, "IntroducedBy"));

    let input = build_input(
        "la djeimyz. fa'u la djordj. prami la meris. fa'u la martas.",
        "composition-fields",
    );
    let composition = input
        .graph
        .objects
        .iter()
        .find_map(|(id, object)| {
            object
                .as_referent()
                .is_some_and(|node| node.composition.is_some())
                .then_some(*id)
        })
        .expect("respectively values have a composition referent");
    let mut object = input.graph.objects[&composition].clone();
    object.update_referent(|node| {
        let composition = node
            .composition
            .clone()
            .expect("selected referent has composition")
            .with_data(data! { collective: Some(true) });
        node.with_data(data! { composition: Some(composition) })
    });
    let graph = replace_object(input.graph.clone(), composition, object);
    let datum = validate_render(&graph, &render_smusni(&graph));
    assert!(contains_field(&datum, "Collective"));

    let input = build_input("lo mlatu cu jbena", "description-fields");
    let description = input
        .graph
        .objects
        .iter()
        .find_map(|(id, object)| {
            object
                .as_referent()
                .is_some_and(|node| {
                    node.descriptor
                        .as_ref()
                        .is_some_and(|descriptor| descriptor.word == "lo")
                })
                .then_some(*id)
        })
        .expect("description witness has lo referent");
    let mut object = input.graph.objects[&description].clone();
    object.update_referent(|node| {
        let descriptor = node
            .descriptor
            .clone()
            .expect("lo has a descriptor")
            .with_data(data! { name: Some("mutated".to_owned()) });
        node.with_data(data! { descriptor: Some(descriptor) })
    });
    let graph = replace_object(input.graph.clone(), description, object);
    let datum = validate_render(&graph, &render_smusni(&graph));
    assert!(contains_field(&datum, "Name"));

    let input = build_input("mi pu klama", "eventuality-indexicals");
    let now = input
        .graph
        .objects
        .iter()
        .find_map(|(id, object)| {
            object
                .as_eventuality()
                .is_some_and(|node| node.indexical == Some(IndexicalKind::Now))
                .then_some(*id)
        })
        .expect("tense witness has an eventuality-valued Now");
    for kind in [
        IndexicalKind::Speaker,
        IndexicalKind::Audience,
        IndexicalKind::Here,
    ] {
        let mut object = input.graph.objects[&now].clone();
        object.update_eventuality(|node| node.with_data(data! { indexical: Some(kind) }));
        let graph = replace_object(input.graph.clone(), now, object);
        let datum = validate_render(&graph, &render_smusni(&graph));
        assert_compact_family_or_typed_graph(&datum, "Object", "cross-sort eventuality indexical");
        assert!(contains_field(&datum, "Indexical"));
    }

    let input = build_input("mi klama", "referent-now-indexical");
    let speaker = input
        .graph
        .objects
        .iter()
        .find_map(|(id, object)| {
            object
                .as_referent()
                .is_some_and(|node| node.indexical == Some(IndexicalKind::Speaker))
                .then_some(*id)
        })
        .expect("utterance has an entity-valued Speaker");
    let mut object = input.graph.objects[&speaker].clone();
    object.update_referent(|node| node.with_data(data! { indexical: Some(IndexicalKind::Now) }));
    let graph = replace_object(input.graph.clone(), speaker, object);
    let datum = validate_render(&graph, &render_smusni(&graph));
    assert_compact_family_or_typed_graph(&datum, "Object", "cross-sort referent indexical");
    assert!(contains_field(&datum, "Indexical"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn word_cards_are_inside_the_single_smusni_document() {
    let input = build_input("mi klama lo zarci", "word-cards");
    let cards = build_word_cards(jbotci_dictionary_data::english(), &input.words);
    assert!(!cards.is_empty());
    let rendered = render_smusni_with_word_cards(&input.graph, &cards);
    let datum = validate_render(&input.graph, &rendered);
    assert_eq!(count_forms(&datum, "Words"), 1);
    assert_eq!(count_forms(&datum, "Word"), cards.len());
    assert_eq!(count_forms(&datum, "Smusni"), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn render_stats_and_diagnostic_projection_are_complete() {
    for doc in CORPUS_DOCS {
        let input = corpus_input(doc);
        let rendered = render_smusni_detailed(&input.graph, &[]);
        validate_render(&input.graph, &rendered.text);
        let expected_warnings = input
            .graph
            .objects
            .values()
            .map(|object| object.diagnostics().len())
            .sum::<usize>();
        assert_eq!(rendered.stats.warning_count, expected_warnings);
        assert_eq!(
            rendered.diagnostics.len(),
            expected_warnings + rendered.stats.fallback_reasons.len(),
        );
        assert!(rendered.stats.compact_objects + rendered.stats.object_fallbacks > 0);
    }
}
