//! Structural validation for the experimental smusni S-expression renderer.
//!
//! These tests deliberately avoid byte-exact output expectations. They parse
//! every rendered document and validate the notation's binding, reference,
//! diagnostics, modal, and document-shape invariants instead.

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use jbotci_dialect::DialectDefinition;
use jbotci_morphology::{
    MorphologyOptions, WordLike, segment_words_with_modifiers_with_options_and_source_id,
};
use jbotci_semantics::completeness::corpus::CORPUS_DOCS;
use jbotci_semantics::model::{
    Actuality, ActualityKind, AnchorRelation, ArgumentValueKind, Aspect, EventualityNode,
    EventualitySort, FormulaNode, FormulaNodeData, IndexicalKind, MathLiteral, ParameterRole,
    PlaceIndex, PredicationMode, PredicationRelationData, QuantityForm, QuantityScale,
    QuantityValue, QuestionSlot, QuestionSlotRole, Recurrence, RecurrenceKind, ReferentCategory,
    ScopeDependence, ScopeDependenceData, SelectionSource, SemanticGraphData, SemanticObject,
    SemanticObjectId, Subscript, UtteranceForce,
};
use jbotci_semantics::notation::kernel::types::Variable;
use jbotci_semantics::notation::sexpr::internal_raw::whole_graph_capture;
use jbotci_semantics::notation::sexpr::{Datum, parse_document, parse_v0_document};
use jbotci_semantics::notation::word_cards::build_word_cards;
use jbotci_semantics::{
    FailureClass, SemanticBuildOptions, SemanticGraph, SmusniProjectionFailed, SmusniRender,
    build_generated_semantic_graph_with_dictionary_and_options, render_smusni,
    render_smusni_with_word_cards,
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
///
/// A mutation that adds, drops or retargets a reference invalidates that
/// owner's rows in the scope occurrence table, which the graph requires to be
/// exactly its reference edges, so the table is repaired alongside the object.
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
    let scope = data.scope.with_owner_reindexed(id, &object);
    objects.insert(id, object);
    SemanticGraph::from_data(data!(SemanticGraph {
        objects,
        scope,
        ..data
    }))
}

/// Insert one otherwise unreachable support object for a mutation witness.
///
/// Origins are total over the object map, so the support object is recorded at
/// the document region: it is written by this test rather than by any act, and
/// the root region is the one place that claims no binders over it.
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
    let root = data.scope.root;
    let scope = data
        .scope
        .with_origin(id, root)
        .with_owner_reindexed(id, &object);
    objects.insert(id, object);
    SemanticGraph::from_data(data!(SemanticGraph {
        objects,
        scope,
        ..data
    }))
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

/// Count one exact atom at every tree position.
#[requires(!atom.is_empty())]
#[ensures(true)]
fn count_atoms(datum: &Datum, atom: &str) -> usize {
    usize::from(datum.as_atom() == Some(atom))
        + datum
            .as_list()
            .into_iter()
            .flat_map(|items| items.iter())
            .map(|item| count_atoms(item, atom))
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

/// Require a compact family when the graph projects, or a registered
/// projection failure when the live builder graph is not projectable.
#[requires(!head.is_empty())]
#[ensures(true)]
fn assert_compact_family_or_projection_failure(graph: &SemanticGraph, head: &str, witness: &str) {
    match render_smusni(graph) {
        Ok(rendered) => {
            let datum = validate_render(graph, &rendered.text);
            assert!(
                count_forms(&datum, head) > 0,
                "{witness} projected a document without {head}"
            );
        }
        Err(_) => {
            project_failure(graph);
        }
    }
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

/// Project one graph and require the complete document result.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.text.ends_with('\n'))]
fn project_document(graph: &SemanticGraph) -> SmusniRender {
    match render_smusni(graph) {
        Ok(rendered) => rendered,
        Err(failed) => panic!(
            "expected one complete document; the projection failed with {:?}",
            failed
                .failures
                .iter()
                .map(|failure| failure.reason_id)
                .collect::<Vec<_>>()
        ),
    }
}

/// The document text of a graph that must project.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.ends_with('\n'))]
fn document_text(graph: &SemanticGraph) -> String {
    project_document(graph).into_data().text
}

/// Project one graph and require the failed result, checking every
/// specification section-17 law of a failed result on the way through.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(!ret.failures.is_empty())]
fn project_failure(graph: &SemanticGraph) -> SmusniProjectionFailed {
    let Err(failed) = render_smusni(graph) else {
        panic!("expected a projection failure; the graph produced a document");
    };
    assert!(
        !failed.failures.is_empty(),
        "a failed result reports at least one error"
    );
    for failure in &failed.failures {
        assert!(
            failure.reason_id.starts_with("smusni.projection."),
            "unregistered reason {}",
            failure.reason_id
        );
        assert!(!failure.message.is_empty());
        assert!(failure.span.byte_start <= failure.span.byte_end);
        assert!(FailureClass::ALL.contains(&failure.failure_class));
        assert_eq!(
            failure.severity(),
            jbotci_semantics::model::DiagnosticSeverity::Error
        );
    }
    assert_eq!(failed.stats.failed_projection_edges, failed.failures.len());
    assert_eq!(
        failed.stats.failure_reasons.values().sum::<usize>(),
        failed.failures.len()
    );
    // Deterministic for a given graph: the same graph fails identically.
    let Err(again) = render_smusni(graph) else {
        panic!("a failing projection must not become a document on a second run");
    };
    assert_eq!(
        again.failures, failed.failures,
        "failure order is deterministic"
    );
    failed
}

/// The internal debug capture of a graph whose projection failed.
///
/// The capture is **not** smusni: it is the unstable codec of
/// `docs/smusni/internal-raw.md`, used here as the losslessness oracle that a
/// mutated model field is still represented somewhere. The projection itself
/// produces no document, which `project_failure` checks first.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.1.form_head() == Some("TypedGraph"))]
fn failing_capture(graph: &SemanticGraph) -> (SmusniProjectionFailed, Datum) {
    let failed = project_failure(graph);
    let reason = failed.failures[0].reason_id;
    let capture = whole_graph_capture(graph, reason);
    (failed, capture)
}

/// The registered reason ids of one failed projection, in channel order.
#[requires(true)]
#[ensures(ret.len() == failed.failures.len())]
fn failure_reason_ids(failed: &SmusniProjectionFailed) -> Vec<&'static str> {
    failed
        .failures
        .iter()
        .map(|failure| failure.reason_id)
        .collect()
}

#[test]
#[requires(true)]
#[ensures(true)]
fn frozen_corpus_is_total_deterministic_and_structurally_valid() {
    for doc in CORPUS_DOCS {
        let input = corpus_input(doc);
        match render_smusni(&input.graph) {
            Ok(first) => {
                let second = project_document(&input.graph);
                assert_eq!(first.text, second.text, "nondeterministic document {doc}");
                validate_render(&input.graph, &first.text);
            }
            Err(_) => {
                // A corpus input that does not project is a product error with
                // no document at all; the law check is in `project_failure`.
                project_failure(&input.graph);
            }
        }
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
    let mut observed_failures = BTreeSet::new();
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
        // Families with no compact route are product projection errors, so they
        // are checked against the failure channel rather than a document.
        match name {
            "relation-question" => {
                assert_compact_family_or_projection_failure(&input.graph, "Ask", name);
                continue;
            }
            "math" | "displayed" | "abstraction" => {
                assert_compact_family_or_projection_failure(&input.graph, "Assert", name);
                continue;
            }
            "quotation"
            | "hostile-quotation"
            | "termset"
            | "respectively-distribution"
            | "respectively-values" => {
                // No quotation or termset family has a compact route, so these
                // are product projection errors with no document to inspect.
                observed_failures.insert(name);
                project_failure(&input.graph);
                continue;
            }
            _ => {}
        }
        // Every other family is checked as it actually behaves: a document is
        // validated structurally, and a projection failure is checked against
        // the section-17 laws of a failed result. The four required families
        // below are the ones this milestone claims a compact route for.
        let rendered = match render_smusni(&input.graph) {
            Ok(rendered) => rendered,
            Err(_) => {
                assert!(
                    !matches!(
                        name,
                        "deleted-place" | "paragraph" | "relative-clause" | "tanru"
                    ),
                    "{name} claims a compact route but did not project"
                );
                observed_failures.insert(name);
                project_failure(&input.graph);
                continue;
            }
        };
        let datum = validate_render(&input.graph, &rendered.text);
        match name {
            "deleted-place" => assert_eq!(count_forms(&datum, "DropPlace"), 1),
            "paragraph" => assert_eq!(count_forms(&datum, "NewTopic"), 1),
            "relative-clause" => {
                assert_eq!(count_forms(&datum, "Bind"), 1);
                assert_eq!(count_forms(&datum, "Refer"), 1);
                assert_eq!(count_forms(&datum, "∧"), 1);
            }
            "tanru" => assert_eq!(count_forms(&datum, "Tanru"), 1),
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
        ] {
            if count_forms(&datum, head) > 0 {
                observed_heads.insert(head);
            }
        }
    }
    for required in ["DropPlace", "NewTopic", "Bind", "Refer"] {
        assert!(
            observed_heads.contains(required),
            "missing structural family {required}"
        );
    }
    assert!(
        observed_failures.contains("quotation"),
        "the quotation witness must reach the projection-failure channel"
    );
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
        let failed = project_failure(&graph);
        assert!(
            failure_reason_ids(&failed)
                .contains(&"smusni.projection.force-reduction-unrepresentable"),
            "unsupported force {force:?} must use the registered force reason: {:?}",
            failure_reason_ids(&failed)
        );
    }

    // `Ask : Query<A> -> Act<Question>`, so ask force over content that is not
    // a typed question must fail closed rather than apply `Ask` at the wrong
    // type. Nothing in the model constrains an utterance's content object, so
    // this boundary is a contract the renderer has to enforce itself.
    let mut object = input.graph.objects[&utterance].clone();
    object.update_utterance(|node| node.with_data(data! { force: UtteranceForce::Ask }));
    let graph = replace_object(input.graph.clone(), utterance, object);
    let failed = project_failure(&graph);
    assert_eq!(
        failed
            .stats
            .failure_reasons
            .get("smusni.projection.question-domain-or-answer-mismatch"),
        Some(&1),
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn speaker_description_is_one_nonveridical_reference_computation() {
    let input = build_input("le mlatu cu gerku", "speaker-description");
    let datum = validate_render(&input.graph, &document_text(&input.graph));

    // Two binders: the description's own `Refer`, and — inside the described
    // property, where the candidate is live — section 5.1's explicit bind for
    // `mlatu`'s elided x2, whose recorded dependence is the described candidate
    // rather than `Fixed`.
    assert_eq!(count_forms(&datum, "Bind"), 2);
    assert_eq!(count_forms(&datum, "Refer"), 1);
    assert_eq!(count_forms(&datum, "skicu"), 1);
    assert_eq!(count_forms(&datum, "mlatu"), 1);
    assert_eq!(count_forms(&datum, "Le"), 0);

    let mut descriptions = Vec::new();
    collect_forms(&datum, "skicu", &mut descriptions);
    let fields = descriptions[0]
        .as_list()
        .expect("skicu description is an application");
    assert_eq!(fields.len(), 5);
    assert_eq!(fields[1].as_atom(), Some("Speaker"));
    assert!(
        fields[2]
            .as_atom()
            .is_some_and(|atom| atom.starts_with('$'))
    );
    assert_eq!(fields[3].as_atom(), Some("Audience"));
    assert_eq!(count_forms(&fields[4], "λ"), 1);
    assert_eq!(count_forms(&fields[4], "mlatu"), 1);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn description_property_retains_filled_conventional_places() {
    let input = build_input("lo penbi be mi cu barda", "description-filled-place");
    let datum = validate_render(&input.graph, &document_text(&input.graph));

    assert_eq!(count_forms(&datum, "Bind"), 1);
    assert_eq!(count_forms(&datum, "Refer"), 1);
    let mut properties = Vec::new();
    collect_forms(&datum, "penbi", &mut properties);
    assert_eq!(properties.len(), 1);
    assert_eq!(
        numbered_application_places(properties[0]),
        BTreeSet::from([1, 2])
    );

    let nested = build_input(
        "lo penbi be lo cukta cu barda",
        "description-nested-reference",
    );
    project_failure(&nested.graph);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn atomic_relation_question_uses_a_typed_open_predicate_row() {
    let input = build_input("ti mo", "atomic-relation-question");
    let rendered = project_document(&input.graph);
    assert!(rendered.stats.failure_reasons.is_empty());
    let datum = validate_render(&input.graph, &rendered.text);

    assert_eq!(count_forms(&datum, "Ask"), 1);
    assert_eq!(count_forms(&datum, "OpenQ"), 1);
    assert_eq!(count_forms(&datum, "PredTerm"), 1);
    assert_eq!(count_forms(&datum, "Row"), 1);
    assert_eq!(count_forms(&datum, "Close"), 1);

    let tanru = build_input("ti mo zdani", "relation-question-tanru");
    project_failure(&tanru.graph);
}

/// The canonical flat tanru graph is one of the compact families this milestone
/// claims, so pin the recognized route exactly. A disjunctive
/// compact-or-`TypedGraph` check would let a silent withdrawal of the
/// recognizer pass by falling back instead.
#[test]
#[requires(true)]
#[ensures(true)]
fn canonical_flat_tanru_projects_the_registered_relation_former() {
    let input = build_input("ti blanu zdani", "canonical-flat-tanru");
    let rendered = project_document(&input.graph);
    assert!(rendered.stats.failure_reasons.is_empty());
    let datum = validate_render(&input.graph, &rendered.text);

    let mut formers = Vec::new();
    collect_forms(&datum, "Tanru", &mut formers);
    assert_eq!(formers.len(), 1);
    // `(Tanru modifier head)`: the seltau precedes the tertau and neither root
    // leaves the lowercase content-root namespace.
    assert_eq!(
        formers[0]
            .as_list()
            .expect("relation former is a list")
            .iter()
            .map(|item| item.as_atom().expect("former operands are atoms"))
            .collect::<Vec<_>>(),
        ["Tanru", "blanu", "zdani"],
    );

    // The former is applied to the tertau's own numbered places, so the asserted
    // content is one application of it to the single deictic argument.
    let mut asserted = Vec::new();
    collect_forms(&datum, "Assert", &mut asserted);
    assert_eq!(asserted.len(), 1);
    let application = asserted[0].as_list().expect("Assert is a list")[1]
        .as_list()
        .expect("asserted content is an application");
    assert_eq!(application.len(), 2);
    assert_eq!(&application[0], formers[0]);
    assert_eq!(application[1].as_atom(), Some("This"));
}

/// The fixed `la` + cmevla name description is one of the compact families this
/// milestone claims, so pin the recognized route exactly for the same reason
/// `canonical_flat_tanru_projects_the_registered_relation_former` does: a
/// compact-or-`TypedGraph` disjunction would let a silent withdrawal of the
/// recognizer pass as a fallback. The second half pins the honest boundary —
/// a `la` descriptor over a selbri body carries no name and stays fallback.
#[test]
#[requires(true)]
#[ensures(true)]
fn fixed_name_description_projects_one_named_reference_computation() {
    let input = build_input("la .alis. cu bajra", "fixed-name-description");
    let rendered = project_document(&input.graph);
    assert!(rendered.stats.failure_reasons.is_empty());
    let datum = validate_render(&input.graph, &rendered.text);
    assert_eq!(count_forms(&datum, "Bind"), 1);
    assert_eq!(count_forms(&datum, "Refer"), 1);

    // `(Named "alis" $var)`: the name is a string operand, never an atom that
    // would collide with the content-root namespace, and it is applied to the
    // variable the hosting `Bind` introduces.
    let mut names = Vec::new();
    collect_forms(&datum, "Named", &mut names);
    assert_eq!(names.len(), 1);
    let operands = names[0].as_list().expect("Named is a list");
    assert_eq!(operands.len(), 3);
    assert_eq!(operands[1].as_string(), Some("alis"));
    let variable = operands[2].as_atom().expect("Named applies to a variable");

    let mut bindings = Vec::new();
    collect_forms(&datum, "Bind", &mut bindings);
    let entries = bindings[0].as_list().expect("Bind is a list")[1]
        .as_list()
        .expect("Bind entries are a list");
    assert_eq!(entries.len(), 1);
    assert_eq!(binding_name(&entries[0]), variable);

    // The predication applies to that same bound variable, so the name is the
    // runner rather than a second, unrelated referent.
    let mut asserted = Vec::new();
    collect_forms(&datum, "Assert", &mut asserted);
    assert_eq!(asserted.len(), 1);
    let application = asserted[0].as_list().expect("Assert is a list")[1]
        .as_list()
        .expect("asserted content is an application");
    assert_eq!(application.len(), 2);
    assert_eq!(application[0].as_atom(), Some("bajra"));
    assert_eq!(application[1].as_atom(), Some(variable));

    // `la` over a selbri body is a different descriptor: it has no name field,
    // so no compact constructor is recognized and the document stays typed.
    let body = build_input("la gerku cu bajra", "name-description-selbri-body");
    project_failure(&body.graph);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn fixed_context_uses_the_bare_primitive_atom() {
    let input = build_input("mi klama", "fixed-context-spelling");
    let selected = input.graph.objects.iter().find_map(|(id, object)| {
        let predication = object.as_predication()?;
        predication.arguments.iter().find_map(|(place, argument)| {
            if argument.kind != ArgumentValueKind::Elided {
                return None;
            }
            let value = argument.value?;
            let referent = input.graph.objects[&value].as_referent()?;
            matches!(
                referent.scope_dependence.as_ref()?.as_data(),
                data!(ScopeDependence::Fixed)
            )
            .then_some((*id, *place, value))
        })
    });
    let (predication, place, source_context) =
        selected.expect("witness has a fixed elided context");
    let context = SemanticObjectId::referent(999_999);
    assert!(!input.graph.objects.contains_key(&context));
    let graph = insert_object(
        input.graph.clone(),
        context,
        input.graph.objects[&source_context].clone(),
    );
    let mut object = graph.objects[&predication].clone();
    object.update_predication(|node| {
        let mut arguments = node.arguments.clone();
        let argument = arguments
            .remove(&place)
            .expect("selected argument remains present")
            .with_data(data! {
                kind: ArgumentValueKind::Filled,
                value: Some(context),
                introduced_by: None,
            });
        arguments.insert(place, argument);
        node.with_data(data! { arguments: arguments })
    });
    let graph = replace_object(graph, predication, object);
    let rendered = project_document(&graph);
    let datum = validate_render(&graph, &rendered.text);
    assert_eq!(count_atoms(&datum, "Fixed"), 0);
    assert_eq!(count_forms(&datum, "MayDependOn"), 0);
    assert_eq!(count_forms(&datum, "Context"), 0);
    assert!(count_atoms(&datum, "Context") > 0);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn generic_composition_and_nonexact_quantities_do_not_borrow_callable_heads() {
    let composition = build_input("mi joi do cu klama", "generic-composition");
    assert!(composition.graph.objects.values().any(|object| {
        object
            .as_referent()
            .is_some_and(|node| node.composition.is_some())
    }));
    // No generic-composition route exists, so this is a projection error; the
    // point of the test is that no callable head is borrowed, and a failed
    // projection emits no head at all.
    project_failure(&composition.graph);

    let input = build_input("ci mlatu cu jbena", "nonexact-quantity");
    let quantity = input
        .graph
        .objects
        .iter()
        .find_map(|(id, object)| object.as_quantity().is_some().then_some(*id))
        .expect("cardinality witness has a quantity");
    let mut object = input.graph.objects[&quantity].clone();
    object.update_quantity(|node| node.with_data(data! { form: QuantityForm::AtLeast }));
    let graph = replace_object(input.graph.clone(), quantity, object);
    let failed = project_failure(&graph);
    assert!(
        failure_reason_ids(&failed)
            .iter()
            .any(|reason| reason.starts_with("smusni.projection.quantity")),
        "a nonexact quantity fails under a registered quantity reason: {:?}",
        failure_reason_ids(&failed)
    );
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
        let datum = validate_render(&input.graph, &document_text(&input.graph));
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
    let tense = validate_render(&tense.graph, &document_text(&tense.graph));
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
        let (_failed, datum) = failing_capture(&graph);
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
    let (_failed, datum) = failing_capture(&graph);
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
    let (_failed, datum) = failing_capture(&graph);
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
    let (_failed, datum) = failing_capture(&graph);
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
    project_failure(&graph);

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
    let (_failed, datum) = failing_capture(&graph);
    assert!(contains_field(&datum, "Subscript"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn binder_and_projection_recognizers_require_complete_typed_shapes() {
    let input = build_input("ro mlatu cu jbena", "quantifier-domain-import");
    let ordinary = validate_render(&input.graph, &document_text(&input.graph));
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
    let (_failed, datum) = failing_capture(&graph);
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
    let (_failed, datum) = failing_capture(&graph);
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
    let (_failed, datum) = failing_capture(&graph);
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
    let (_failed, datum) = failing_capture(&graph);
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
    let (_failed, datum) = failing_capture(&graph);
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
    let (_failed, datum) = failing_capture(&graph);
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
        let (_failed, datum) = failing_capture(&graph);
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
    let (_failed, datum) = failing_capture(&graph);
    assert!(contains_field(&datum, "Indexical"));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn word_cards_are_inside_the_single_smusni_document() {
    let input = build_input("mi klama lo zarci", "word-cards");
    let cards = build_word_cards(jbotci_dictionary_data::english(), &input.words);
    assert!(!cards.is_empty());
    let rendered = render_smusni_with_word_cards(&input.graph, &cards)
        .expect("the word-card witness projects")
        .into_data()
        .text;
    let datum = validate_render(&input.graph, &rendered);
    assert_eq!(count_forms(&datum, "Words"), 1);
    assert_eq!(count_forms(&datum, "Word"), cards.len());
    assert_eq!(count_forms(&datum, "Smusni"), 1);

    // A defined zei-lujvo's card word is its exact multi-word dictionary
    // surface. The grammar admits that through its escaped lexical-root
    // spelling, so the card must survive with that identity instead of being
    // dropped from the reference section the XML rendering still carries.
    let zei = build_input("mi klama lo abu zei sance", "word-cards-zei-lujvo");
    let zei_cards = build_word_cards(jbotci_dictionary_data::english(), &zei.words);
    // The card's escaped surface is a property of the card production, so it is
    // checked on the card data itself: this witness's own graph has no compact
    // projection, and a failed projection carries no `Words` section at all.
    assert!(
        zei_cards
            .iter()
            .any(|card| card.word == "abu zei sance" && card.definition.is_some()),
        "witness must produce a defined zei-lujvo card",
    );
    let emitted = zei_cards
        .iter()
        .filter_map(jbotci_semantics::notation::sexpr::word_card_datum)
        .collect::<Vec<_>>();
    assert_eq!(
        emitted.len(),
        zei_cards
            .iter()
            .filter(|card| card.definition.is_some())
            .count()
    );
    assert!(
        emitted.iter().any(|card| {
            card.as_list()
                .and_then(|items| items.get(1))
                .and_then(Datum::as_atom)
                == Some("|abu zei sance|")
        }),
        "the zei-lujvo card must keep its exact surface as an escaped lexical root",
    );
    project_failure(&zei.graph);
}

#[test]
#[requires(true)]
#[ensures(true)]
fn render_stats_and_diagnostic_projection_are_complete() {
    for doc in CORPUS_DOCS {
        let input = corpus_input(doc);
        let expected_semantic = input
            .graph
            .objects
            .values()
            .map(|object| object.diagnostics().len())
            .sum::<usize>();
        match render_smusni(&input.graph) {
            Ok(rendered) => {
                validate_render(&input.graph, &rendered.text);
                assert_eq!(rendered.stats.semantic_diagnostic_count, expected_semantic);
                // Nonfatal semantic diagnostics travel alone on the success
                // path; there is no failure record to summarize.
                assert_eq!(rendered.diagnostics.len(), expected_semantic);
                assert_eq!(rendered.stats.failed_projection_edges, 0);
                assert!(rendered.stats.failure_reasons.is_empty());
                assert!(rendered.stats.compact_objects > 0);
            }
            Err(failed) => {
                assert_eq!(failed.stats.semantic_diagnostic_count, expected_semantic);
                assert_eq!(failed.diagnostics.len(), expected_semantic);
                // The aggregate reason counts are exactly a summary of the
                // per-edge records, never a substitute for them.
                assert_eq!(
                    failed.failures.len(),
                    failed.stats.failed_projection_edges,
                    "{doc}: the edge statistic must count the per-edge records",
                );
                assert_eq!(
                    failed.stats.failed_projection_edges,
                    failed.stats.failure_reasons.values().sum::<usize>(),
                    "{doc}: the reason aggregate must account for every failed edge",
                );
                assert_eq!(failed.stats.compact_objects, 0);
            }
        }
    }
}

/// One Lojban input per eventuality subtype the model can mint.
///
/// Subtype identities are spelled `eventuality/<subtype>` by the model, so
/// every one of them is a regression witness for variable spelling.
const EVENTUALITY_SUBTYPE_INPUTS: [(EventualitySort, &str); 7] = [
    (EventualitySort::General, "mi djuno lo nu do klama"),
    (EventualitySort::State, "le za'i mi jmive cu ckape do"),
    (EventualitySort::Process, "mi tatpi ri'a le pu'u mi plipe"),
    (EventualitySort::Activity, "mi tatpi ri'a le zu'o mi plipe"),
    (
        EventualitySort::Achievement,
        "le mu'e la .djan. catra la .djim. cu zekri",
    ),
    (EventualitySort::Experience, "mi morji le li'i mi verba"),
    (EventualitySort::Locution, "mi klama"),
];

/// The CLL inputs whose renders manufactured an invalid variable atom.
const CLL_VARIABLE_SPELLING_REGRESSIONS: [(&str, &str); 5] = [
    (
        "c11e12d2",
        "le mikce cu se cinri le pu'u jenai za'i mi sipna",
    ),
    ("c11e3d1", "le mu'e la .djan. catra la .djim. cu zekri"),
    ("c11e3d3", "mi tatpi ri'a le zu'o mi plipe"),
    ("c11e3d4", "le za'i mi jmive cu ckape do"),
    ("c11e9d1", "mi morji le li'i mi verba"),
];

/// Collect every `$` atom occurring in a rendered document.
#[requires(true)]
#[ensures(ret.iter().all(|name| name.starts_with('$')))]
fn variable_atoms(datum: &Datum) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    collect_variable_atoms(datum, &mut names);
    names
}

/// Accumulate `$` atoms into a caller-owned set.
#[requires(true)]
#[ensures(out.len() >= old(out.len()))]
fn collect_variable_atoms(datum: &Datum, out: &mut BTreeSet<String>) {
    match datum {
        Datum::Atom(_) => {
            if let Some(name) = datum.as_atom().filter(|name| name.starts_with('$')) {
                out.insert(name.to_owned());
            }
        }
        Datum::List(items) => {
            for item in items {
                collect_variable_atoms(item, out);
            }
        }
        Datum::String(_) | Datum::Integer(_) => {}
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn every_eventuality_subtype_renders_a_valid_variable_grammar() {
    for (sort, text) in EVENTUALITY_SUBTYPE_INPUTS {
        let input = build_input(text, &format!("eventuality-{sort:?}"));
        assert!(
            input.graph.objects.values().any(|object| object
                .as_eventuality()
                .is_some_and(|node| node.sort == sort)),
            "{text:?} was expected to mint an {sort:?} eventuality",
        );
        // Rendering used to panic while manufacturing `$eventuality/<subtype>`.
        // Whether the subtype projects or fails, no invalid atom may be
        // spelled; on the failure path the internal capture holds the atoms.
        let datum = match render_smusni(&input.graph) {
            Ok(rendered) => validate_render(&input.graph, &rendered.text),
            Err(_) => failing_capture(&input.graph).1,
        };
        for name in variable_atoms(&datum) {
            Variable::try_new(&name)
                .unwrap_or_else(|error| panic!("{text:?} spelled {name:?}: {error}"));
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn named_cll_regressions_render_without_manufacturing_invalid_atoms() {
    for (name, text) in CLL_VARIABLE_SPELLING_REGRESSIONS {
        let input = build_input(text, name);
        // These five inputs previously panicked while manufacturing a variable
        // atom. Whether they project or fail, no invalid atom may be spelled;
        // on the failure path the internal capture is where any atom lives.
        let datum = match render_smusni(&input.graph) {
            Ok(rendered) => validate_render(&input.graph, &rendered.text),
            Err(_) => failing_capture(&input.graph).1,
        };
        for variable in variable_atoms(&datum) {
            Variable::try_new(&variable)
                .unwrap_or_else(|error| panic!("{name} spelled {variable:?}: {error}"));
        }
    }
}

/// Borrow every per-edge failure record in channel order.
#[requires(true)]
#[ensures(ret.len() == failed.failures.len())]
fn failure_records(
    failed: &SmusniProjectionFailed,
) -> Vec<(
    &'static str,
    &'static str,
    Option<SemanticObjectId>,
    Option<SemanticObjectId>,
)> {
    failed
        .failures
        .iter()
        .map(|failure| {
            (
                failure.reason_id,
                failure.message,
                failure.owner,
                failure.use_site,
            )
        })
        .collect()
}

#[test]
#[requires(true)]
#[ensures(true)]
fn failure_channel_is_evidenced_and_reproducible_on_the_corpus() {
    // This is a well-formedness sweep, not the cardinality proof: on a corpus
    // it can only observe the channel the renderer produced. The laws that
    // distinguish one re-entered edge from two genuinely distinct edges are
    // proved directly against the channel representations in
    // `notation::sexpr::elaborate` and `notation::sexpr::planner`.
    //
    // Owner and use-site evidence is optional by design, so this only requires
    // that whatever identities a record does carry resolve in the graph it
    // describes. Ordering is deterministic from the typed channels, but the
    // internal sort key is not a public tuple contract, so this observes it
    // only as render-to-render equality.
    for doc in CORPUS_DOCS {
        let input = corpus_input(doc);
        let Err(failed) = render_smusni(&input.graph) else {
            continue;
        };
        let records = failure_records(&failed);
        for (reason_id, message, owner, use_site) in &records {
            assert!(reason_id.starts_with("smusni.projection."));
            assert!(!message.is_empty());
            for identity in [owner, use_site].into_iter().flatten() {
                assert!(
                    input.graph.objects.contains_key(identity),
                    "{doc}: {reason_id} names unresolvable evidence {identity}",
                );
            }
        }
        // The aggregate channel summarizes exactly these records.
        let mut counted = BTreeMap::new();
        for (reason_id, ..) in &records {
            *counted.entry(*reason_id).or_insert(0usize) += 1;
        }
        assert_eq!(counted, failed.stats.failure_reasons, "{doc}");
        // Projecting the same graph twice must yield the identical channel.
        assert_eq!(records, failure_records(&project_failure(&input.graph)));
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn scope_failures_carry_binder_and_use_evidence() {
    // The restriction repeat on `da`'s own argument is discharged, so planning
    // no longer refuses this graph outright. What still declines is section
    // 6.3's closing rule: the elided constants inside the quantifier depend on
    // `da`, and the leftover declaration group has no open host with `da` live,
    // so the registered scope-dependency route reports them per identity rather
    // than a placement being retried.
    let input = build_input("ro da poi gerku cu bajra", "scope-evidence");
    let failed = project_failure(&input.graph);
    let records = failure_records(&failed);
    let definition_site = records
        .iter()
        .filter(|(reason_id, ..)| *reason_id == "smusni.projection.scope-dependency-without-binder")
        .collect::<Vec<_>>();
    assert!(
        !definition_site.is_empty(),
        "planner failures must reach the channel: {records:?}",
    );
    // Evidence is optional in general; what this input demonstrates is that the
    // planner does attach it, and that whatever it attaches resolves in the
    // graph the record describes.
    assert!(
        definition_site
            .iter()
            .any(|(_, _, owner, _)| owner.is_some()),
        "planner failures must name the affected identity: {records:?}",
    );
    for (reason_id, message, owner, use_site) in &definition_site {
        assert!(!message.is_empty());
        for identity in [owner, use_site].into_iter().flatten() {
            assert!(
                input.graph.objects.contains_key(identity),
                "{reason_id} named unresolvable evidence {identity}",
            );
        }
    }
}

/// Whether a form with this head occurs anywhere in a datum.
#[requires(!head.is_empty())]
#[ensures(true)]
fn contains_form(datum: &Datum, head: &str) -> bool {
    datum.form_head() == Some(head)
        || datum
            .as_list()
            .is_some_and(|items| items.iter().any(|item| contains_form(item, head)))
}

/// Whether any `inner` form occurs strictly inside some `outer` form.
#[requires(!outer.is_empty() && !inner.is_empty())]
#[ensures(true)]
fn nests_inside(datum: &Datum, outer: &str, inner: &str) -> bool {
    if datum.form_head() == Some(outer)
        && datum
            .as_list()
            .is_some_and(|items| items.iter().skip(1).any(|item| contains_form(item, inner)))
    {
        return true;
    }
    datum
        .as_list()
        .is_some_and(|items| items.iter().any(|item| nests_inside(item, outer, inner)))
}

/// A reference computation required while evaluating a description's property
/// runs inside that computation, and the outer one still raises to the top of
/// its force segment.
///
/// Specification section 8.3 states the nesting: "A nested `RefComp` required
/// while evaluating `$P` runs inside this reference computation unless the
/// graph assigns that nested effect its own legal outer host", and no version-0
/// graph assigns one. Section 6.3 states the ascent: the outer `Bind` raises to
/// the outermost legal point inside its force segment, so its binder encloses
/// the act constructor rather than standing under `Assert`.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_nested_description_is_hosted_inside_the_description_that_needs_it() {
    let input = build_input(
        "le prenu poi zvati le kumfa poi blanu cu masno",
        "nested-refer",
    );
    let rendered = project_document(&input.graph);
    let datum = parse_document(&rendered.text).expect("a rendered document parses");
    assert!(
        nests_inside(&datum, "Refer", "Bind"),
        "the inner reference computation must run inside the outer one:\n{}",
        rendered.text,
    );
    assert!(
        nests_inside(&datum, "Bind", "Assert"),
        "the outer binder must enclose the act constructor:\n{}",
        rendered.text,
    );
    assert!(
        !nests_inside(&datum, "Assert", "Bind"),
        "no reference computation stays under Assert once it may raise:\n{}",
        rendered.text,
    );
}

/// The same nesting holds when the nested computation fills an argument place
/// of the outer description's own property rather than a relative clause.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_description_inside_a_description_property_nests_rather_than_raising() {
    let input = build_input("le le nanmu ku karce cu blanu", "nested-refer-argument");
    let rendered = project_document(&input.graph);
    let datum = parse_document(&rendered.text).expect("a rendered document parses");
    assert!(
        nests_inside(&datum, "Refer", "Bind"),
        "the argument's reference computation runs inside the property that needs it:\n{}",
        rendered.text,
    );
}

/// A gadri-folded property abstraction is section 11.1's bare lambda.
///
/// The builder writes `lo ka ce'u broda` as one relation-sorted referent
/// carrying the descriptor *and* the abstraction payload, so the object is the
/// property. Section 11.1 licenses exactly that — "no `Property` or `Relation`
/// record is needed around the function" — and there is no second body
/// predicating over the abstraction, so no `Refer` may appear either.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_gadri_folded_property_crossing_is_one_lambda() {
    let input = build_input("mi nelci lo ka ce'u melbi", "folded-ka");
    assert!(
        input
            .graph
            .objects
            .values()
            .any(|object| object.as_parameter().is_some()),
        "a ka abstraction has a ce'u parameter",
    );
    let rendered = project_document(&input.graph);
    let datum = parse_document(&rendered.text).expect("a rendered document parses");
    assert_eq!(
        count_forms(&datum, "λ"),
        1,
        "the crossing is one lambda:\n{}",
        rendered.text,
    );
    assert_eq!(
        count_forms(&datum, "Refer"),
        0,
        "folding the gadri introduces no description:\n{}",
        rendered.text,
    );
    // The lambda declares the parameter, `melbi` fills its first place with it,
    // and each of the three remaining elided places records the parameter as
    // its permitted dependence, so section 5.1 names it once per explicit bind.
    assert_eq!(
        count_atoms(&datum, "$parameterNode_8"),
        5,
        "the lambda binds and uses the graph-owned parameter:\n{}",
        rendered.text,
    );
}

/// A gadri-folded proposition abstraction is section 11.3's `Reify`.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_gadri_folded_proposition_crossing_is_one_reify() {
    let input = build_input("mi djuno lo du'u do klama", "folded-duhu");
    let rendered = project_document(&input.graph);
    let datum = parse_document(&rendered.text).expect("a rendered document parses");
    assert_eq!(
        count_forms(&datum, "Reify"),
        1,
        "the proposition crossing is one `Reify`:\n{}",
        rendered.text,
    );
    assert_eq!(
        count_forms(&datum, "Close"),
        0,
        "section 5.2 elides `Close` at a registered `Content` operand:\n{}",
        rendered.text,
    );
}

/// An event abstraction is section 11.2's ordinary reference computation, and
/// its property binds the abstracted event at the content root's own event
/// place rather than through an `EventOf`-style primitive.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_described_event_binds_its_own_event_place() {
    let input = build_input("lo nu mi klama cu se cinri mi", "described-event");
    let rendered = project_document(&input.graph);
    let datum = parse_document(&rendered.text).expect("a rendered document parses");
    assert!(
        nests_inside(&datum, "Bind", "Refer"),
        "the abstraction is hosted by an ordinary `Bind`:\n{}",
        rendered.text,
    );
    assert!(
        nests_inside(&datum, "Refer", "λ"),
        "the reference computation carries a property:\n{}",
        rendered.text,
    );
    assert!(
        rendered.text.contains(":Eventuality $eventuality_7"),
        "the property fills the content root's distinguished event place:\n{}",
        rendered.text,
    );
    for forbidden in ["EventOf", "AchievementOf", "StateOf"] {
        assert_eq!(
            count_atoms(&datum, forbidden),
            0,
            "section 14.3 forbids {forbidden}:\n{}",
            rendered.text,
        );
    }
}

/// A quantified content root names the abstraction's own collective event, and
/// its `memberOf` restriction is recorded `inert` rather than `restrictive`.
///
/// The builder stamps an abstraction body's mode over everything under it,
/// including a quantifier restriction written inside that body, so the two
/// non-asserted modes occur at one and the same locus across a corpus. Both
/// denote content without force and print identically, so the boundary accepts
/// the set rather than one member.
#[test]
#[requires(true)]
#[ensures(true)]
fn an_inert_restriction_inside_abstracted_content_projects() {
    let input = build_input(
        "lo nu ro lo prenu cu troci kei cu cadga",
        "inert-restriction",
    );
    let restriction = input
        .graph
        .objects
        .values()
        .filter_map(|object| object.as_predication())
        .find(|node| {
            matches!(
                node.relation.as_data(),
                data!(PredicationRelation::Named { relation }) if relation == "memberOf"
            )
        })
        .expect("a described domain records a membership restriction");
    assert_eq!(
        restriction.mode,
        PredicationMode::Inert,
        "the witness for the accepted-set boundary is the builder's own mode",
    );
    let rendered = project_document(&input.graph);
    assert!(
        rendered.text.contains(":Eventuality $eventuality_7"),
        "the quantified content root fills the abstraction's own event place:\n{}",
        rendered.text,
    );
}

/// Assertion stays positional: an asserted predication inside abstracted
/// content is a graph inconsistency and keeps failing.
#[test]
#[requires(true)]
#[ensures(true)]
fn an_asserted_predication_inside_abstracted_content_is_refused() {
    let input = build_input("lo nu mi klama cu se cinri mi", "asserted-inside-content");
    let abstracted = input
        .graph
        .objects
        .iter()
        .find_map(|(id, object)| {
            object
                .as_predication()
                .is_some_and(|node| node.mode == PredicationMode::Inert)
                .then_some(*id)
        })
        .expect("abstracted content carries an inert predication");
    let mut object = input.graph.objects[&abstracted].clone();
    object.update_predication(|node| node.with_data(data! { mode: PredicationMode::Asserted }));
    let graph = replace_object(input.graph.clone(), abstracted, object);
    let failed = project_failure(&graph);
    assert!(
        failure_reason_ids(&failed)
            .contains(&"smusni.projection.relation-reduction-unregistered-or-inexact"),
        "force inside a property is not licensed by the no-force boundary: {:?}",
        failure_reason_ids(&failed),
    );
}

/// Section 6.3 stops a raised reference at the binders its property actually
/// names, not at every binder its recorded dependence permits it to name.
///
/// `lo nu mi dunda ti do` inside a universal records `mayDependOn` on the
/// quantified variable, because the builder writes the permission the position
/// allows. Its property mentions no binder at all — the base predication's
/// places are all filled, so it has no elided place whose own section-5.1 bind
/// would name one — and the computation raises past the quantifier to the
/// enclosing abstraction's barrier; anchoring it by the permission instead
/// would trap it inside a quantifier it never mentions.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_raised_reference_stops_at_the_binders_its_property_names() {
    let input = build_input(
        "lo nu ro lo prenu cu troci lo nu mi dunda ti do kei kei cu cadga",
        "permission-wider-than-use",
    );
    assert!(
        input.graph.objects.values().any(|object| {
            object.as_eventuality().is_some_and(|node| {
                node.content.is_some()
                    && node
                        .denotation
                        .scope_dependence()
                        .and_then(|dependence| dependence.may_depend_on().cloned())
                        .is_some_and(|universe| !universe.is_empty())
            })
        }),
        "the inner abstraction records a dependence permission it does not use",
    );
    let rendered = project_document(&input.graph);
    let datum = parse_document(&rendered.text).expect("a rendered document parses");
    let hosts = collect_bind_hosts(&datum, "$eventuality_9");
    assert_eq!(
        hosts.len(),
        1,
        "one identity is one binder:\n{}",
        rendered.text
    );
    assert!(
        contains_form(&hosts[0], "Every"),
        "the binder encloses the quantifier its property never names:\n{}",
        rendered.text,
    );
}

/// Every `Bind` form in a document that introduces one named binder.
#[requires(!binder.is_empty())]
#[ensures(true)]
fn collect_bind_hosts<'a>(datum: &'a Datum, binder: &str) -> Vec<&'a Datum> {
    let mut hosts = Vec::new();
    collect_bind_hosts_into(datum, binder, &mut hosts);
    hosts
}

/// Recursive half of [`collect_bind_hosts`].
#[requires(!binder.is_empty())]
#[ensures(true)]
fn collect_bind_hosts_into<'a>(datum: &'a Datum, binder: &str, out: &mut Vec<&'a Datum>) {
    if datum.form_head() == Some("Bind")
        && datum.as_list().is_some_and(|items| {
            items
                .get(1)
                .and_then(Datum::as_list)
                .is_some_and(|entries| entries.iter().any(|entry| binding_name(entry) == binder))
        })
    {
        out.push(datum);
    }
    if let Some(items) = datum.as_list() {
        for item in items {
            collect_bind_hosts_into(item, binder, out);
        }
    }
}

/// A description's own body mentions the description it defines, and that
/// occurrence is not a use: the builder records it as `definitionInternal`
/// exactly so that counting it as sharing cannot turn every ordinary `lo broda`
/// into a shared definition no reference type can spell.
///
/// This is the specification's membership shape end to end (sections 8.4, 9.2):
/// `ro lo P` is an `Every` whose restriction is `memberOf` of the bound variable
/// against the description referent, and the description is hosted *outside* the
/// quantifier, so the restriction is a plain predicate over one bound variable
/// and one outer binding.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_description_reached_once_is_not_a_shared_definition() {
    let input = build_input("ro lo prenu cu prami", "membership-restriction");
    let rendered = project_document(&input.graph);
    let datum = parse_document(&rendered.text).expect("a rendered document parses");
    let hosts = collect_bind_hosts(&datum, "$entity_11");
    assert_eq!(
        hosts.len(),
        1,
        "the description is bound exactly once:\n{}",
        rendered.text
    );
    assert!(
        contains_form(hosts[0], "Every"),
        "the description referent is hosted outside its own quantifier:\n{}",
        rendered.text
    );
    let restrictions = collect_forms_owned(&datum, "Every");
    assert_eq!(restrictions.len(), 1);
    assert!(
        contains_atom(restrictions[0], "memberOf"),
        "section 9.2 restricts the candidate by membership in the description:\n{}",
        rendered.text
    );
    // No `Let` is written, because section 6.3's dynamic host is already the
    // declaration site; the failure this used to take was the demand for a
    // `Let`-bindable type of a value that never becomes a `Let`.
    assert_eq!(count_forms(&datum, "Let"), 0, "{}", rendered.text);
}

/// Section 8.4's `goi` menu, in the configuration the builder resolves
/// structurally: the quantifier's own λ *is* the assignment, so the handle names
/// a binder that is already printed and contributes nothing of its own.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_quantified_goi_handle_is_the_binder_it_names() {
    let input = build_input("ro lo prenu goi ko'a cu prami ko'a", "quantified-goi");
    assert!(
        input.graph.objects.values().any(|object| object
            .as_referent()
            .is_some_and(|node| node.category == ReferentCategory::Variable
                && !node.assigned_names.is_empty())),
        "the bound variable carries the assignment the builder resolved onto it",
    );
    let rendered = project_document(&input.graph);
    let datum = parse_document(&rendered.text).expect("a rendered document parses");
    assert!(
        !rendered.text.contains("ko'a"),
        "the handle is provenance and prints nothing:\n{}",
        rendered.text
    );
    let applications = collect_forms_owned(&datum, "prami");
    assert_eq!(applications.len(), 1, "{}", rendered.text);
    let places = applications[0]
        .as_list()
        .expect("an application is a list")
        .iter()
        .skip(1)
        .map(|item| item.as_atom().map(str::to_owned))
        .collect::<Vec<_>>();
    assert_eq!(
        places,
        vec![Some("$entity_7".to_owned()), Some("$entity_7".to_owned())],
        "both places are the one variable the assignment resolved to:\n{}",
        rendered.text
    );
}

/// The same handle written outside a quantifier selects the other item on
/// section 8.4's menu: the description's own hosted `Refer` is the `Bind`, and
/// both occurrences are uses of the name it introduced. No `Let` alias is
/// needed, because there is no second identity to alias.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_same_scope_goi_handle_is_the_description_binder() {
    let input = build_input("lo prenu goi ko'a cu prami ko'a", "same-scope-goi");
    let rendered = project_document(&input.graph);
    let datum = parse_document(&rendered.text).expect("a rendered document parses");
    assert!(!rendered.text.contains("ko'a"), "{}", rendered.text);
    assert_eq!(count_forms(&datum, "Let"), 0, "{}", rendered.text);
    assert_eq!(
        collect_bind_hosts(&datum, "$entity_7").len(),
        1,
        "one identity is one binder:\n{}",
        rendered.text
    );
}

/// The third assignment shape the builder writes is not provenance. `X goi la
/// djan` states that the referent bears a name, which is section 8.4's naming
/// predicate; no resolution edge carries it, so suppressing it would drop
/// content and the description keeps its registered refusal.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_goi_name_assignment_is_not_resolution_provenance() {
    let input = build_input("lo prenu goi la djan cu prami", "goi-name-assignment");
    assert!(
        input
            .graph
            .objects
            .values()
            .any(|object| object
                .as_referent()
                .is_some_and(|node| node.assigned_names.iter().any(|assigned| {
                    assigned.introduced_by == "goi" && assigned.name != assigned.word
                }))),
        "the record separates the name-marking word from the name",
    );
    let failed = project_failure(&input.graph);
    assert!(
        failure_reason_ids(&failed)
            .contains(&"smusni.projection.reference-description-unrepresentable"),
        "{:?}",
        failure_reason_ids(&failed)
    );
}

/// A binder is a free binder of the value being projected, not something the
/// projection consumes. Collecting one into an inlined subgraph's support makes
/// the quantifier that binds it — and every sibling place it fills — look like
/// an escape, which refused every abstraction whose content mentions a variable
/// bound outside it.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_binder_named_inside_abstracted_content_is_not_support() {
    let input = build_input(
        "ro lo prenu goi ko'a cu troci lo nu ko'a klama",
        "binder-inside-content",
    );
    let rendered = project_document(&input.graph);
    let datum = parse_document(&rendered.text).expect("a rendered document parses");
    // `validate_render` proves the whole document is well scoped, so an
    // abstraction whose binding names the quantified variable necessarily
    // stands inside that quantifier's scope.
    validate_render(&input.graph, &rendered.text);
    let hosts = collect_bind_hosts(&datum, "$eventuality_8");
    assert_eq!(hosts.len(), 1, "{}", rendered.text);
    assert!(
        contains_atom(hosts[0], "$entity_7"),
        "the abstraction's content names the variable bound outside it:\n{}",
        rendered.text
    );
    assert!(contains_atom(&datum, "klama"), "{}", rendered.text);
}

/// An interior `zo'e` of a description property may record any binders live at
/// the description's host position. It projects as the section-5.1 `Context` its
/// dependence names, bound inside the property, and section 6.3 then hosts the
/// description where its property's free binders are all live — so a dependent
/// description is representable rather than a shape to refuse before planning.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_dependent_description_projects_inside_the_binders_it_names() {
    let input = build_input(
        "ro lo prenu cu troci lo nu lo pendo cu klama",
        "dependent-description",
    );
    assert!(
        input.graph.objects.values().any(|object| {
            object.as_referent().is_some_and(|node| {
                node.descriptor
                    .as_ref()
                    .is_some_and(|descriptor| descriptor.body.is_some())
                    && node
                        .scope_dependence
                        .as_ref()
                        .and_then(|dependence| dependence.may_depend_on())
                        .is_some_and(|universe| !universe.is_empty())
            })
        }),
        "the inner description records a dependence on the quantified variable",
    );
    let rendered = project_document(&input.graph);
    let datum = parse_document(&rendered.text).expect("a rendered document parses");
    assert!(contains_atom(&datum, "pendo"), "{}", rendered.text);
}

/// The issue's named acceptance witness. Every claim below is a specification
/// claim about the document's shape rather than a byte expectation: two `nu`
/// reference computations hosted where their properties' binders are live, a
/// `ka` lambda over its graph-owned `ce'u`, two membership restrictions, and the
/// `ta'i` modal conjoined into the inner abstraction's property.
#[test]
#[requires(true)]
#[ensures(true)]
fn the_membership_path_witness_renders_a_canonical_document() {
    let input = build_input(
        "cadga fa lonu ro lo prenu goi ko'a cu troci lonu ko'a tarti loka ce'u xendo je cnikansa \
         ro lo jmive kei ta'i lo racli",
        "issue-778-witness",
    );
    let rendered = project_document(&input.graph);
    let datum = parse_document(&rendered.text).expect("a rendered document parses");
    validate_render(&input.graph, &rendered.text);
    let quantifiers = collect_forms_owned(&datum, "Every");
    assert_eq!(quantifiers.len(), 2, "{}", rendered.text);
    for quantifier in &quantifiers {
        assert!(
            contains_atom(quantifier, "memberOf"),
            "each domain is a membership restriction:\n{}",
            rendered.text
        );
    }
    // The `ka` crossing is a bare lambda over the graph's own parameter, and its
    // body is the `je` conjunction of both relations.
    assert!(
        rendered.text.contains("$parameterNode_11"),
        "{}",
        rendered.text
    );
    for relation in ["cadga", "troci", "tarti", "xendo", "cnikansa", "tadji"] {
        assert!(
            contains_atom(&datum, relation),
            "{relation} reaches the document:\n{}",
            rendered.text
        );
    }
    // The outer `nu` is fixed and binds at the top; the inner one names the
    // quantified variable and binds inside the quantifier.
    let outer = collect_bind_hosts(&datum, "$eventuality_7");
    assert_eq!(outer.len(), 1, "{}", rendered.text);
    assert!(
        contains_form(outer[0], "Assert"),
        "the fixed abstraction binds above the act that mentions it:\n{}",
        rendered.text
    );
    let inner = collect_bind_hosts(&datum, "$eventuality_9");
    assert_eq!(inner.len(), 1, "{}", rendered.text);
    // The document is well scoped, so a binding that names `$entity_8` stands
    // inside the quantifier that introduces it.
    assert!(
        contains_atom(inner[0], "$entity_8"),
        "the dependent abstraction names the quantified variable:\n{}",
        rendered.text
    );
    assert!(!rendered.text.contains("ko'a"), "{}", rendered.text);
}

/// Every `head` form in one document, outermost first.
#[requires(!head.is_empty())]
#[ensures(true)]
fn collect_forms_owned<'a>(datum: &'a Datum, head: &str) -> Vec<&'a Datum> {
    let mut out = Vec::new();
    collect_forms(datum, head, &mut out);
    out
}

/// Whether one atom occurs anywhere in a document.
#[requires(!atom.is_empty())]
#[ensures(ret == (count_atoms(datum, atom) > 0))]
fn contains_atom(datum: &Datum, atom: &str) -> bool {
    count_atoms(datum, atom) > 0
}

/// A handle resolved onto an indexical needs no binder at all: the atom prints
/// by identity wherever it occurs, so every use of the handle prints that atom
/// and the assignment has nothing left to state. This is the third position
/// section 8.4's "depending on its graph semantics" reaches, and it is why an
/// assignment on a canonical atom is provenance rather than a refusal.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_handle_resolved_onto_an_atom_prints_that_atom() {
    let input = build_input("mi goi ko'a cu prami ko'a", "indexical-goi");
    let rendered = project_document(&input.graph);
    let datum = parse_document(&rendered.text).expect("a rendered document parses");
    assert!(!rendered.text.contains("ko'a"), "{}", rendered.text);
    assert_eq!(
        count_atoms(&datum, "Speaker"),
        2,
        "both places print the atom the handle resolved to:\n{}",
        rendered.text
    );
    assert_eq!(count_forms(&datum, "Bind"), 0, "{}", rendered.text);
    assert_eq!(count_forms(&datum, "Let"), 0, "{}", rendered.text);
}

/// Section 5.1 rule 1: a defaultable ordinary referential place whose graph
/// dependence is `Fixed` "receives a fresh bare `Context` computation" as part
/// of closure, and section 5.2 lets that closure itself be omitted. Nothing of
/// it is written, because a fresh site-stable lookup is fully determined by the
/// place it fills — there is no permission to preserve.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_fixed_default_place_is_hidden_by_close() {
    let input = build_input("mi klama", "fixed-default-place");
    let elided = input
        .graph
        .objects
        .values()
        .filter_map(SemanticObject::as_referent)
        .filter(|node| {
            node.scope_dependence
                .as_ref()
                .is_some_and(|dependence| dependence.may_depend_on().is_none())
        })
        .count();
    assert!(
        elided > 0,
        "the graph records `Fixed` defaults for `klama`'s unstated places",
    );
    let rendered = project_document(&input.graph);
    let datum = parse_document(&rendered.text).expect("a rendered document parses");
    assert_eq!(count_forms(&datum, "Bind"), 0, "{}", rendered.text);
    assert_eq!(count_atoms(&datum, "Context"), 0, "{}", rendered.text);
    let applications = collect_forms_owned(&datum, "klama");
    assert_eq!(applications.len(), 1, "{}", rendered.text);
    assert_eq!(
        applications[0]
            .as_list()
            .expect("an application is a list")
            .len(),
        2,
        "only the stated place is written:\n{}",
        rendered.text
    );
}

/// Section 5.1's other half: an `Underspecified { mayDependOn }` default
/// "cannot be hidden by `Close`: it is bound explicitly from
/// `(Context dependencies...)` and the same bound value fills the place. This
/// preserves the exact permitted dependency set rather than replacing it with
/// 'all accessible binders.'"
///
/// This is the one position where the recorded universe *is* the printed
/// content. Eliding the place instead would leave re-elaboration to rederive a
/// dependence at the printed site, and the two coincide only by accident of
/// this site: nothing in the document would still say which binders the graph
/// actually permitted this lookup to resolve against.
#[test]
#[requires(true)]
#[ensures(true)]
fn an_underspecified_default_place_is_bound_explicitly_from_its_recorded_dependence() {
    let input = build_input("ro lo prenu cu prami", "underspecified-default-place");
    let permitted = input
        .graph
        .objects
        .values()
        .filter_map(SemanticObject::as_referent)
        .filter_map(|node| {
            node.scope_dependence
                .as_ref()
                .and_then(|dependence| dependence.may_depend_on().cloned())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        permitted.len(),
        1,
        "one elided place records a permission naming the quantified variable",
    );
    assert_eq!(permitted[0].len(), 1);

    let rendered = project_document(&input.graph);
    let datum = parse_document(&rendered.text).expect("a rendered document parses");
    let contexts = collect_forms_owned(&datum, "Context");
    assert_eq!(
        contexts.len(),
        1,
        "the recorded permission is written once:\n{}",
        rendered.text
    );
    let dependencies = contexts[0]
        .as_list()
        .expect("a context computation is a list")
        .iter()
        .skip(1)
        .map(|item| item.as_atom().map(str::to_owned))
        .collect::<Vec<_>>();
    assert_eq!(
        dependencies,
        vec![Some("$entity_7".to_owned())],
        "the dependency list is the recorded one, not every accessible binder:\n{}",
        rendered.text
    );
    let hosts = collect_bind_hosts(&datum, "$entity_8");
    assert_eq!(
        hosts.len(),
        1,
        "the explicit bind stands at the closure it belongs to:\n{}",
        rendered.text
    );
    let applications = collect_forms_owned(&datum, "prami");
    assert_eq!(applications.len(), 1, "{}", rendered.text);
    let places = applications[0]
        .as_list()
        .expect("an application is a list")
        .iter()
        .skip(1)
        .map(|item| item.as_atom().map(str::to_owned))
        .collect::<Vec<_>>();
    assert_eq!(
        places,
        vec![Some("$entity_7".to_owned()), Some("$entity_8".to_owned())],
        "the same bound value fills the place it was bound for:\n{}",
        rendered.text
    );
}

/// Section 5.1 orders the closure's own computations: they "run left to right
/// in current numbered-place order". The explicit binds are that order made
/// visible, so the binder nesting at one closure follows the places rather than
/// graph-identity allocation order.
#[test]
#[requires(true)]
#[ensures(true)]
fn underspecified_default_binds_run_left_to_right_in_numbered_place_order() {
    let input = build_input("mi nelci lo ka ce'u melbi", "default-bind-order");
    let rendered = project_document(&input.graph);
    let datum = parse_document(&rendered.text).expect("a rendered document parses");
    let binders = collect_forms_owned(&datum, "Bind")
        .into_iter()
        .map(|host| {
            binding_name(
                &host.as_list().expect("a bind is a list")[1]
                    .as_list()
                    .expect("a bind declares a list of entries")[0],
            )
        })
        .collect::<Vec<_>>();
    let applications = collect_forms_owned(&datum, "melbi");
    assert_eq!(applications.len(), 1, "{}", rendered.text);
    let filled = applications[0]
        .as_list()
        .expect("an application is a list")
        .iter()
        .skip(2)
        .map(|item| item.as_atom().unwrap_or_default().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(binders.len(), 3, "{}", rendered.text);
    assert_eq!(
        binders, filled,
        "the binders nest in the order their places are numbered:\n{}",
        rendered.text
    );
}

/// The operands of one rendered form, which for a logical junction are its
/// conjuncts and for a `Bind` are its binding list and its body.
#[requires(datum.as_list().is_some())]
#[ensures(true)]
fn form_operands(datum: &Datum) -> &[Datum] {
    &datum.as_list().expect("a form is a list")[1..]
}

/// Peel one `Bind` into the single name it introduces and the value it wraps.
#[requires(datum.form_head() == Some("Bind"))]
#[ensures(true)]
fn peel_bind(datum: &Datum) -> (String, &Datum) {
    let operands = form_operands(datum);
    assert_eq!(operands.len(), 2, "a bind has entries and a body");
    let entries = operands[0].as_list().expect("bind entries are a list");
    assert_eq!(entries.len(), 1, "each hosted computation binds one name");
    (binding_name(&entries[0]), &operands[1])
}

/// Peel a nest of `Bind`s into the names they introduce, outermost first, and
/// the innermost value they all wrap.
#[requires(true)]
#[ensures(true)]
fn peel_binds(datum: &Datum) -> (Vec<String>, &Datum) {
    let mut names = Vec::new();
    let mut body = datum;
    while body.form_head() == Some("Bind") {
        let (name, inner) = peel_bind(body);
        names.push(name);
        body = inner;
    }
    (names, body)
}

/// Section 5.1's locality rule, asserted as topology rather than presence: each
/// omitted computation runs "at the dynamic evaluation site of `Close` ... local
/// to that closure", and section 6.2 evaluates `∧` and `Joi` operands left to
/// right with each operand seeing the preceding one's context. So a `Context`
/// belonging to the second conjunct must stand *inside* that conjunct: pooling
/// both above the junction resolves the second lookup before the first conjunct
/// has run, which is a different document.
///
/// The witness is the sharpest case in the corpus, carrying both junctions: the
/// `je` conjunction of `xendo` and `cnikansa`, and the `Joi` of the inner
/// abstraction's content with its `ta'i` adjunct.
#[test]
#[requires(true)]
#[ensures(true)]
fn each_closure_binds_its_own_defaults_inside_its_own_operand() {
    let input = build_input(
        "cadga fa lonu ro lo prenu goi ko'a cu troci lonu ko'a tarti loka ce'u xendo je cnikansa \
         ro lo jmive kei ta'i lo racli",
        "issue-778-witness-topology",
    );
    let rendered = project_document(&input.graph);
    let datum = validate_render(&input.graph, &rendered.text);

    let conjunctions = collect_forms_owned(&datum, "∧");
    assert_eq!(conjunctions.len(), 1, "{}", rendered.text);
    let conjuncts = form_operands(conjunctions[0]);
    assert_eq!(conjuncts.len(), 2, "{}", rendered.text);
    for (conjunct, relation) in conjuncts.iter().zip(["xendo", "cnikansa"]) {
        let (names, body) = peel_binds(conjunct);
        assert_eq!(
            names.len(),
            1,
            "{relation}'s own default binds inside its own conjunct:\n{}",
            rendered.text
        );
        assert_eq!(
            body.form_head(),
            Some(relation),
            "the bind wraps exactly its own closure:\n{}",
            rendered.text
        );
        assert!(
            body.as_list()
                .expect("an application is a list")
                .iter()
                .any(|item| item.as_atom() == Some(names[0].as_str())),
            "{relation} fills its place with the value bound for it:\n{}",
            rendered.text
        );
    }
    // Nothing hoists either conjunct's computation above the junction.
    assert_eq!(
        collect_bind_hosts(&datum, "$entity_13").len(),
        1,
        "{}",
        rendered.text
    );
    assert!(
        !contains_atom(&peel_binds(&conjuncts[0]).1, "cnikansa"),
        "the first conjunct's bind does not enclose the second conjunct:\n{}",
        rendered.text
    );

    let junctions = collect_forms_owned(&datum, "Joi");
    assert_eq!(junctions.len(), 1, "{}", rendered.text);
    let operands = form_operands(junctions[0]);
    assert_eq!(operands.len(), 2, "{}", rendered.text);
    let (tarti_binds, tarti_body) = peel_binds(&operands[0]);
    assert_eq!(tarti_binds.len(), 1, "{}", rendered.text);
    assert_eq!(tarti_body.form_head(), Some("tarti"), "{}", rendered.text);
    let (tadji_binds, tadji_body) = peel_binds(&operands[1]);
    assert_eq!(
        tadji_binds.len(),
        2,
        "the adjunct's two defaults bind inside the adjunct's own operand:\n{}",
        rendered.text
    );
    assert_eq!(tadji_body.form_head(), Some("tadji"), "{}", rendered.text);
    let filled = form_operands(tadji_body)
        .iter()
        .skip(1)
        .map(|item| item.as_atom().unwrap_or_default().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        tadji_binds, filled,
        "and in the order their places are numbered:\n{}",
        rendered.text
    );
}

/// A question body with two sibling closures is the same rule one level up: the
/// answered slot's lambda is a section-6.3 host position, but it is not the
/// section-5.1 evaluation site of anything, so each conjunct's own default
/// stays inside that conjunct and only the identity the graph shares between
/// them is bound above both.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_question_body_binds_each_closure_default_in_its_own_closure() {
    let input = build_input("do jai se smuni ma", "question-body-closures");
    let rendered = project_document(&input.graph);
    let datum = validate_render(&input.graph, &rendered.text);
    let queries = collect_forms_owned(&datum, "OpenQ");
    assert_eq!(queries.len(), 1, "{}", rendered.text);
    let lambda = &form_operands(queries[0])[0];
    let (shared, junction) = peel_binds(&form_operands(lambda)[1]);
    assert_eq!(
        shared.len(),
        1,
        "only the identity both closures use is bound above them:\n{}",
        rendered.text
    );
    assert_eq!(junction.form_head(), Some("∧"), "{}", rendered.text);
    let conjuncts = form_operands(junction);
    assert_eq!(conjuncts.len(), 2, "{}", rendered.text);
    let (local, smuni) = peel_binds(&conjuncts[0]);
    assert_eq!(
        local.len(),
        1,
        "the first closure's own default binds inside it:\n{}",
        rendered.text
    );
    assert_eq!(smuni.form_head(), Some("smuni"), "{}", rendered.text);
    assert_eq!(
        conjuncts[1].form_head(),
        Some("involves"),
        "the second closure omits nothing and binds nothing:\n{}",
        rendered.text
    );
    assert!(
        contains_atom(&conjuncts[1], &shared[0]),
        "the shared value is used in the second closure, which is why it is \
         bound above both:\n{}",
        rendered.text
    );
}

/// The one predication in a focused graph that names the given relation.
#[requires(!relation.is_empty())]
#[ensures(graph.objects.contains_key(&ret))]
fn named_predication(graph: &SemanticGraph, relation: &str) -> SemanticObjectId {
    graph
        .objects
        .iter()
        .find_map(|(id, object)| {
            matches!(
                object.as_predication()?.relation.as_data(),
                data!(PredicationRelation::Named { relation: named }) if named == relation
            )
            .then_some(*id)
        })
        .expect("the named predication is present")
}

/// Retarget one place of a named predication onto another value already in the
/// graph.
///
/// This is model surgery, not a builder shape: the audit under test exists
/// because the model's invariants permit a described selection source and its
/// restriction to disagree, and only a hand-written graph reaches that.
#[requires(graph.objects.contains_key(&target))]
#[ensures(ret.objects.len() == old(graph.objects.len()))]
fn retarget_place(
    graph: SemanticGraph,
    relation: &str,
    place: usize,
    target: SemanticObjectId,
) -> SemanticGraph {
    let predication = named_predication(&graph, relation);
    let key = PlaceIndex::new(place);
    let mut object = graph.objects[&predication].clone();
    object.update_predication(|node| {
        let mut arguments = node.arguments.clone();
        let argument = arguments
            .remove(&key)
            .expect("the retargeted place is present")
            .with_data(data! { value: Some(target) });
        arguments.insert(key, argument);
        node.with_data(data! { arguments: arguments })
    });
    replace_object(graph, predication, object)
}

/// Point one place of a predication at the identity another place already
/// names, in one revalidation.
///
/// The two halves have to land together. Retargeting alone leaves the displaced
/// identity unreached, and the derived-dependence invariant is total over the
/// object map: an unreached object is its own component, which the derivation
/// visits at empty scope. So the displaced identity is re-homed at the document
/// region with a `Fixed` dependence in the same edit, and the graph is legal at
/// every point a validator sees it.
///
/// This is model surgery, not a builder shape: the model permits one graph
/// identity to fill two places of one predication, and the renderer has to
/// schedule the shared computation correctly whether or not this builder ever
/// writes that.
#[requires(true)]
#[ensures(ret.0.objects.len() == old(graph.objects.len()))]
fn share_place_with(
    graph: SemanticGraph,
    relation: &str,
    place: usize,
    with_place: usize,
) -> (SemanticGraph, SemanticObjectId) {
    let predication = named_predication(&graph, relation);
    let arguments = graph.objects[&predication]
        .predication_arguments()
        .expect("a predication carries an argument map");
    let key = PlaceIndex::new(place);
    let shared = arguments[&PlaceIndex::new(with_place)]
        .value
        .expect("the retained place is filled");
    let displaced = arguments[&key]
        .value
        .expect("the retargeted place is filled");
    let mut owner = graph.objects[&predication].clone();
    owner.update_predication(|node| {
        let mut arguments = node.arguments.clone();
        let argument = arguments
            .remove(&key)
            .expect("the retargeted place is present")
            .with_data(data! { value: Some(shared) });
        arguments.insert(key, argument);
        node.with_data(data! { arguments: arguments })
    });
    let mut orphan = graph.objects[&displaced].clone();
    orphan.update_referent(|node| {
        node.with_data(data! { scope_dependence: Some(ScopeDependence::fixed()) })
    });
    let data = graph.into_data();
    let mut objects = data.objects;
    let root = data.scope.root;
    let scope = data
        .scope
        .with_owner_reindexed(predication, &owner)
        .with_origin(displaced, root)
        .with_owner_reindexed(displaced, &orphan);
    objects.insert(predication, owner);
    objects.insert(displaced, orphan);
    let graph = SemanticGraph::from_data(data!(SemanticGraph {
        objects,
        scope,
        ..data
    }));
    (graph, displaced)
}

/// Section 5.1 rule 2 keeps a graph-shared default shared — one explicit
/// binder for one identity — but it does not lift that binder out of rule 1's
/// schedule: the omitted computations still "run left to right in current
/// numbered-place order at the dynamic evaluation site of `Close`". So a
/// default shared between x2 and x4 is bound *before* an unshared one at x3,
/// and both stand at the closure that omits them.
///
/// Reaching the shared computation through the deferred declaration pass
/// instead gets both halves wrong: it is emitted after the whole closure has
/// been built, so the later place's binder is already inside it, and it is
/// placed at whatever coarser position the closure has already been left for.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_shared_default_binds_at_its_first_place_in_numbered_order() {
    let input = build_input("mi nelci lo ka ce'u klama je bajra", "shared-default-order");
    let shared = SemanticObjectId::referent(9);
    let unshared = SemanticObjectId::referent(10);
    let displaced = SemanticObjectId::referent(11);
    let last = SemanticObjectId::referent(12);
    for id in [shared, unshared, displaced, last] {
        assert!(
            input.graph.objects[&id]
                .scope_dependence()
                .is_some_and(|dependence| dependence.may_depend_on().is_some()),
            "each of `klama`'s omitted places records a permission",
        );
    }
    // x4 now names the same identity x2 does, which is what makes that identity
    // shared; x3 and x5 keep their own. The identity x4 used to name is left
    // reachable from nothing, so it is re-homed at the document region, where a
    // disconnected object's derived dependence is `Fixed`.
    let (graph, orphaned) = share_place_with(input.graph.clone(), "klama", 4, 2);
    assert_eq!(orphaned, displaced);
    let rendered = project_document(&graph);
    let datum = validate_render(&graph, &rendered.text);

    let applications = collect_forms_owned(&datum, "klama");
    assert_eq!(applications.len(), 1, "{}", rendered.text);
    let filled = form_operands(applications[0])
        .iter()
        .map(|item| item.as_atom().unwrap_or_default().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        filled[1], filled[3],
        "one identity fills both places:\n{}",
        rendered.text
    );
    assert_eq!(
        count_forms(&datum, "Let"),
        0,
        "the shared default is one `Bind`, not a declaration beside it:\n{}",
        rendered.text
    );
    assert_eq!(
        collect_bind_hosts(&datum, &filled[1]).len(),
        1,
        "and it is bound exactly once:\n{}",
        rendered.text
    );

    // The conjunction is where the difference shows: a declaration placed after
    // the closure was built stands above the whole `∧`, where the sibling
    // conjunct — which never mentions this identity — would run inside it.
    let conjunctions = collect_forms_owned(&datum, "∧");
    assert_eq!(conjunctions.len(), 1, "{}", rendered.text);
    let conjuncts = form_operands(conjunctions[0]);
    assert_eq!(conjuncts.len(), 2, "{}", rendered.text);
    let (binders, body) = peel_binds(&conjuncts[0]);
    assert_eq!(body.form_head(), Some("klama"), "{}", rendered.text);
    assert_eq!(
        binders,
        vec![filled[1].clone(), filled[2].clone(), filled[4].clone()],
        "the shared default is scheduled at its first place, inside its own \
         closure, ahead of the unshared one at the next place:\n{}",
        rendered.text
    );
    assert!(
        !contains_atom(&conjuncts[1], &filled[1]),
        "the sibling conjunct does not use it, and is not inside its binder:\n{}",
        rendered.text
    );
}

/// The described selection source is semantic data: it names the plurality the
/// candidate is drawn from, and the model states that the candidate is
/// restricted with `memberOf(candidate, description)`. The reduction prints the
/// restriction and nothing else for the source, which is only sound while the
/// restriction really does contain that conjunct. Nothing in the model's
/// invariants requires it, so the exact renderer audits it — and a restriction
/// naming some other object leaves the recorded domain unprinted, which is a
/// refusal rather than a document.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_selection_source_the_restriction_does_not_name_is_refused() {
    let input = build_input("ro lo prenu cu prami", "selection-source-mismatch");
    let described = SemanticObjectId::referent(11);
    let elsewhere = SemanticObjectId::referent(1);
    assert_eq!(
        input.graph.objects[&SemanticObjectId::formula(17)]
            .as_formula()
            .and_then(|node| match node.as_data() {
                data!(FormulaNode::Quantified(binding)) => binding.selection_source.clone(),
                _ => None,
            })
            .map(|source| source.variable),
        Some(described),
        "the built graph selects from the description its restriction names",
    );
    // The binding keeps naming the description; the restriction stops agreeing,
    // which is the shape the model permits and nothing else audits.
    let graph = retarget_place(input.graph.clone(), "memberOf", 2, elsewhere);
    let failed = project_failure(&graph);
    assert!(
        failure_reason_ids(&failed).contains(&"smusni.projection.quantifier-effect-export-illegal"),
        "the uncertified domain is refused with its registered reason: {:?}",
        failure_reason_ids(&failed)
    );
    assert!(
        failed
            .failures
            .iter()
            .any(|failure| failure.message.contains("selection source")),
        "the per-edge record names the boundary that declined: {:?}",
        failed
            .failures
            .iter()
            .map(|failure| failure.message)
            .collect::<Vec<_>>()
    );
}

/// The same obligation, in the shape that loses the domain most quietly: a
/// described source with no restriction at all would print a plain unrestricted
/// quantifier, and the plurality the graph selected from would appear nowhere.
/// Fail closed; the missing conjunct is never synthesized from the source.
#[test]
#[requires(true)]
#[ensures(true)]
fn a_selection_source_with_no_restriction_is_refused() {
    let input = build_input("ro lo prenu cu prami", "selection-source-unrestricted");
    let quantifier = SemanticObjectId::formula(17);
    let mut object = input.graph.objects[&quantifier].clone();
    object.update_formula(|node| match node.into_data() {
        data!(FormulaNode::Quantified(binding)) => new!(FormulaNode::Quantified(
            binding.with_data(data! { restriction: None })
        )),
        data => FormulaNode::from_data(data),
    });
    let graph = replace_object(input.graph.clone(), quantifier, object);
    let failed = project_failure(&graph);
    assert!(
        failure_reason_ids(&failed).contains(&"smusni.projection.quantifier-effect-export-illegal"),
        "an unrestricted sourced quantifier is refused: {:?}",
        failure_reason_ids(&failed)
    );
}
