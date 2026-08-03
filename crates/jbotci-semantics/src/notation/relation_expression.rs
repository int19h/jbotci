//! Recognition and projection of tanru (predicate-juxtaposition) graph patterns
//! into typed relation expressions for SFN-XML rendering (jbotci#719).
//!
//! The semantic graph lowers a tanru to a formula-level `and` whose children are
//! the head formula and a link predication carrying a `tanruLink` sidecar. The
//! canonical SFN-XML notation renders that pattern as one PREDICATION whose
//! relation slot holds a KIND-COMPOSITION relation expression; the head/link
//! decomposition is the notation's elaboration of that compact form, not
//! something a model-facing document should spell out per instance.
//!
//! This module is the recognition boundary. It walks the canonical graph JSON,
//! proves the canonical tanru pattern for each candidate (strict structural
//! guards — anything shared, decorated, questioned, or otherwise non-flat
//! rejects the compact form), rewrites the objects map (the AND formula becomes
//! the head predication's atom; the head predication gains a typed
//! `relationExpression` view; consumed scaffolding objects are removed), and
//! reports every consumed object so the XML renderer's omission accounting can
//! classify the removed surfaces (projected structure vs waived provenance).
//! Graphs that fail the guards keep the loud head-and-link form.

use std::collections::{BTreeMap, BTreeSet, HashSet};

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use serde::Serialize;
use serde_json::{Map, Value};

/// Grouping basis stated on a KIND-COMPOSITION node. Silence means
/// ASSUMED-LEFT (the deterministic grammar default); only edges whose grouping
/// the text itself determines (a bo/ke boundary, visible in the graph as
/// head-side nesting of the composed AND) are marked.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CompositionGrouping {
    Explicit,
}

/// One operand of a relation composition: the typed view attached to a
/// projected predication as its `relationExpression` field.
#[invariant(::Host { participant_place } => *participant_place > 0)]
#[invariant(::Lexical { predicate, participant_place, fixed_arguments, unfilled_places, .. } =>
    !predicate.is_empty()
        && *participant_place > 0
        && !fixed_arguments.contains_key(participant_place)
        && !unfilled_places.contains(participant_place)
        && fixed_arguments.keys().all(|place| *place > 0)
        && unfilled_places.iter().all(|place| *place > 0))]
#[invariant(::Composition { kind, .. } =>
    !matches!(kind.as_data(), data!(RelationOperand::Connective { .. }) | data!(RelationOperand::Reference { .. })),
    "the kind side of a composition is a head, leaf, or nested composition, never a connective or reference")]
#[invariant(::Connective { operator, operands } =>
    !operator.is_empty()
        && operands.len() >= 2
        && operands.iter().all(|operand| matches!(operand.as_data(), data!(RelationOperand::Lexical { .. }))),
    "a relation connective conjoins at least two compact lexical leaves")]
#[invariant(::Reference { relation } => !relation.is_empty())]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum RelationOperand {
    /// The enclosing predication's own predicate; the predicate name is read
    /// from the predication's `relation` field rather than duplicated here.
    Host { participant_place: usize },
    /// A compact lexical relation leaf: one lexical predicate, a flat place
    /// map, and (when present) a fresh locally bound eventuality.
    Lexical {
        predicate: String,
        participant_place: usize,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        fixed_arguments: BTreeMap<usize, String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        unfilled_places: Vec<usize>,
        has_event: bool,
    },
    /// A nested composition (a tanru inside a tanru): the KIND-COMPOSITION
    /// node of the typed relation-expression view.
    #[serde(rename = "kindComposition")]
    Composition {
        #[serde(skip_serializing_if = "Option::is_none")]
        grouping: Option<CompositionGrouping>,
        kind: Box<RelationOperand>,
        modifier: Box<RelationOperand>,
    },
    /// A logical connective over co-modifiers of the same head participant
    /// (a je-connected seltau lowered inside the property body).
    Connective {
        operator: String,
        operands: Vec<RelationOperand>,
    },
    /// A composite modifier that stays in the graph and renders through the
    /// existing RELATION-lambda / abstraction idioms (NU abstractions,
    /// be-linked arguments with non-default structure, joi composites, shared
    /// or decorated eventualities — anything the compact guards reject).
    Reference { relation: String },
}

/// One recognized and projected tanru instance. Its fields are the elaboration
/// contract of the acceptance test (re-expansion), which is `cfg(test)` — in
/// production builds they are unread, hence the targeted allow.
#[invariant(!anchor.is_empty() && !head_predication.is_empty())]
#[invariant(!consumed.contains(anchor) && !consumed.contains(head_predication), "the anchor and head predication survive projection")]
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct ProjectedInstance {
    /// The formula id whose AND was rewritten into the head atom (it survives).
    pub(crate) anchor: String,
    /// The surviving head predication carrying the `relationExpression` view.
    pub(crate) head_predication: String,
    /// Every consumed (removed) object id, including nested instances' ids but
    /// excluding the anchor itself.
    pub(crate) consumed: BTreeSet<String>,
    /// The typed relation-expression view attached to the head predication.
    pub(crate) composition: RelationOperand,
}

/// The result of projecting every proven tanru pattern in a graph.
#[invariant(consumed_objects.keys().all(|key| !rewritten_anchors.contains_key(key)), "consumed objects and rewritten anchors are disjoint")]
#[derive(Debug, Clone)]
pub(crate) struct TanruProjection {
    /// Per-instance projection data (the acceptance test re-expands these).
    #[allow(dead_code)]
    pub(crate) instances: Vec<ProjectedInstance>,
    /// The original JSON of every consumed object, for omission accounting.
    pub(crate) consumed_objects: Map<String, Value>,
    /// The original JSON of every anchor formula rewritten to the head atom
    /// (the anchor survives, but its `children`/`connector` fields do not —
    /// omission accounting must classify those surfaces too).
    pub(crate) rewritten_anchors: Map<String, Value>,
}

#[requires(true)]
#[ensures(true)]
fn object_of(value: &Value) -> Option<&Map<String, Value>> {
    value.as_object()
}

#[requires(true)]
#[ensures(true)]
fn string_of(value: &Value) -> Option<&str> {
    value.as_str()
}

#[requires(true)]
#[ensures(true)]
fn field<'a>(object: &'a Map<String, Value>, name: &str) -> Option<&'a Value> {
    object.get(name)
}

#[requires(true)]
#[ensures(true)]
fn string_field_of<'a>(object: &'a Map<String, Value>, name: &str) -> Option<&'a str> {
    object.get(name).and_then(Value::as_str)
}

#[requires(true)]
#[ensures(true)]
fn is_object_type(object: &Map<String, Value>, expected: &str) -> bool {
    string_field_of(object, "type") == Some(expected)
}

/// The bound-eventuality id list of a formula object (empty when absent).
#[requires(is_object_type(object, "formula"))]
#[ensures(true)]
fn bound_eventualities(object: &Map<String, Value>) -> Vec<&str> {
    object
        .get("boundEventualities")
        .and_then(Value::as_array)
        .map(|bound| bound.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

/// The place number of an argument key (`x2` → 2); zero marks a malformed key.
#[requires(true)]
#[ensures(true)]
fn argument_place(key: &str) -> usize {
    key.strip_prefix('x')
        .and_then(|digits| digits.parse::<usize>().ok())
        .unwrap_or(0)
}

/// Whether a connector record is the implicit tanru-juxtaposition connective.
#[requires(true)]
#[ensures(true)]
fn is_implicit_connector(object: &Map<String, Value>) -> bool {
    object
        .get("connector")
        .and_then(Value::as_object)
        .and_then(|connector| connector.get("source"))
        .and_then(Value::as_object)
        .and_then(|source| string_field_of(source, "kind"))
        == Some("implicitJuxtaposition")
}

/// The truth table a binary logical operator already determines (row order TT,
/// TF, FT, FF), mirroring the XML renderer's `canonical_truth_table`.
#[requires(true)]
#[ensures(ret.is_none_or(|table| table.len() == 4))]
fn canonical_truth_table(operator: &str) -> Option<&'static str> {
    match operator {
        "and" => Some("TFFF"),
        "or" => Some("TTTF"),
        "iff" => Some("TFFT"),
        "whetherOrNot" => Some("TTFF"),
        _ => None,
    }
}

/// The formula ids reachable from a tanru AND through tanru-internal nesting:
/// the modifier-property region (link predication → tanruLink.modifier →
/// property body → recurse) plus head-side nested implicit ANDs (the bo/ke
/// right-grouping case). Used to order candidates outermost-first: a nested
/// candidate must be consumed by its parent's projection, never projected
/// standalone ahead of it.
#[requires(true)]
#[ensures(true)]
fn tanru_region_walk(objects: &Map<String, Value>, anchor: &str) -> BTreeSet<String> {
    let mut reached = BTreeSet::new();
    let mut frontier = vec![anchor.to_owned()];
    while let Some(formula_id) = frontier.pop() {
        let Some(formula) = objects.get(&formula_id).and_then(Value::as_object) else {
            continue;
        };
        let Some(children) = formula.get("children").and_then(Value::as_array) else {
            continue;
        };
        // Head-side nested implicit AND (bo/ke right-grouping).
        if let Some(head_id) = children.first().and_then(Value::as_str) {
            let is_nested_and =
                objects
                    .get(head_id)
                    .and_then(Value::as_object)
                    .is_some_and(|head| {
                        string_field_of(head, "operator") == Some("and")
                            && is_implicit_connector(head)
                    });
            if is_nested_and && reached.insert(head_id.to_owned()) {
                frontier.push(head_id.to_owned());
            }
        }
        // Modifier-property nesting.
        let Some(link_atom) = children.get(1).and_then(Value::as_str) else {
            continue;
        };
        let Some(link_formula) = objects.get(link_atom).and_then(Value::as_object) else {
            continue;
        };
        let Some(link_predication_id) = string_field_of(link_formula, "predication") else {
            continue;
        };
        let Some(link) = objects.get(link_predication_id).and_then(Value::as_object) else {
            continue;
        };
        let Some(modifier) = link
            .get("tanruLink")
            .and_then(Value::as_object)
            .and_then(|tanru_link| string_field_of(tanru_link, "modifier"))
        else {
            continue;
        };
        let Some(relation) = objects.get(modifier).and_then(Value::as_object) else {
            continue;
        };
        let Some(body) = string_field_of(relation, "body") else {
            continue;
        };
        if reached.insert(body.to_owned()) {
            frontier.push(body.to_owned());
        }
    }
    reached
}

/// One recognized tanru AND formula before the edits are applied.
#[invariant(!anchor.is_empty() && !head_predication.is_empty() && !participant.is_empty())]
#[invariant(!consumed.contains(anchor), "the anchor formula is rewritten, not consumed")]
#[invariant(consumed.contains(head_predication) == matches!(composition.as_data(), data!(RelationOperand::Composition { kind, .. }) if matches!(kind.as_data(), data!(RelationOperand::Lexical { .. }))), "the head predication is consumed exactly when it projects as a lexical leaf (nested in a property body); host and nested-composition heads survive")]
#[derive(Debug)]
struct TanruMatch {
    anchor: String,
    head_predication: String,
    /// The link predication's x1 (the composition participant).
    participant: String,
    /// Bound eventualities the rewritten anchor formula must carry (the AND's
    /// own bindings plus, for a plain head, the head atom's).
    rewritten_bound: Vec<String>,
    composition: RelationOperand,
    /// Consumed object ids, excluding the anchor.
    consumed: BTreeSet<String>,
}

/// Whether a relation is a lexical Lojban predicate: morphologically a single
/// brivla (gismu, lujvo, or fu'ivla, zei compounds included) per the
/// morphology parser — the typed provenance for the compact lexical form, as
/// opposed to synthesized structural relation names.
#[requires(!relation.is_empty())]
#[ensures(true)]
fn relation_is_lexical_brivla(relation: &str) -> bool {
    let Ok(words) = jbotci_morphology::segment_words_with_modifiers(relation) else {
        return false;
    };
    let [word] = words.as_slice() else {
        return false;
    };
    word.is_brivla()
}

/// Fields a predication inside a projected modifier/head must not carry: any of
/// them makes the relation expression non-flat.
const NON_FLAT_PREDICATION_FIELDS: &[&str] = &[
    "placeQuestions",
    "adjuncts",
    "reciprocity",
    "scalarNegation",
    "relationMetadata",
    "introducedBy",
];

#[requires(true)]
#[ensures(true)]
fn has_non_flat_fields(predication: &Map<String, Value>) -> bool {
    NON_FLAT_PREDICATION_FIELDS
        .iter()
        .any(|field| predication.contains_key(*field))
}

/// The recognized shape of a lexical relation leaf (a bare-brivla modifier or a
/// nested composition's head): one named predicate over its property parameter
/// with a flat place map and at most a fresh locally bound eventuality.
#[requires(is_object_type(predication, "predication"))]
#[requires(string_field_of(predication, "relation").is_some())]
#[ensures(ret.as_ref().is_none_or(|(_, _, consumed)| consumed.contains(predication_id)))]
fn recognize_lexical_leaf(
    objects: &Map<String, Value>,
    predication_id: &str,
    predication: &Map<String, Value>,
    parameter: &str,
    locally_bound: &HashSet<String>,
) -> Option<(RelationOperand, Vec<String>, BTreeSet<String>)> {
    if has_non_flat_fields(predication) || predication.contains_key("tanruLink") {
        return None;
    }
    if string_field_of(predication, "mode") != Some("restrictive") {
        return None;
    }
    // A lexical predicate is morphologically a brivla (gismu, lujvo, or
    // fu'ivla — including zei compounds); anything else (synthesized
    // structural relations such as eventOf/memberOf, abstraction links, tense
    // relations) belongs to the composite abstraction idioms, not the compact
    // form. Classification goes through the morphology parser, never string
    // shape (AGENTS.md).
    let predicate = string_field_of(predication, "relation")?;
    if !relation_is_lexical_brivla(predicate) {
        return None;
    }
    let arguments = field(predication, "arguments")?.as_object()?;
    let mut participant_place = None;
    let mut fixed_arguments = BTreeMap::new();
    let mut unfilled_places = Vec::new();
    let mut consumed: BTreeSet<String> = [predication_id.to_owned()].into_iter().collect();
    for (key, argument) in arguments {
        let place = argument_place(key);
        let argument = argument.as_object()?;
        match string_field_of(argument, "kind")? {
            "filled" => {
                let value = string_field_of(argument, "value")?;
                if value == parameter {
                    if participant_place.is_some() || place == 0 {
                        return None;
                    }
                    participant_place = Some(place);
                } else {
                    if place == 0 || argument.contains_key("introducedBy") {
                        return None;
                    }
                    fixed_arguments.insert(place, value.to_owned());
                }
            }
            "elided" => {
                if place == 0 || string_field_of(argument, "introducedBy") != Some("zo'e") {
                    return None;
                }
                let value = string_field_of(argument, "value")?;
                let referent = object_of(objects.get(value)?)?;
                if !is_object_type(referent, "referent")
                    || string_field_of(referent, "category") != Some("constant")
                    || string_field_of(referent, "sort") != Some("entity")
                {
                    return None;
                }
                let descriptor = field(referent, "descriptor")?.as_object()?;
                if string_field_of(descriptor, "kind") != Some("elided")
                    || string_field_of(descriptor, "word") != Some("zo'e")
                {
                    return None;
                }
                for field_name in referent.keys() {
                    if !matches!(
                        field_name.as_str(),
                        "type" | "category" | "scopeDependence" | "sort" | "descriptor" | "source"
                    ) {
                        return None;
                    }
                }
                unfilled_places.push(place);
                consumed.insert(value.to_owned());
            }
            _ => return None,
        }
    }
    let participant_place = participant_place?;
    unfilled_places.sort_unstable();
    let event = string_field_of(predication, "eventuality");
    if let Some(event) = event {
        if !locally_bound.contains(event) {
            return None;
        }
        let referent = object_of(objects.get(event)?)?;
        if !is_object_type(referent, "referent")
            || string_field_of(referent, "denotation") != Some("generated-bound")
            || string_field_of(referent, "sort") != Some("eventuality")
        {
            return None;
        }
        // A decorated (tense/aspect/anything-else) modifier event is not the
        // derivable fresh-local eventuality of the compact form.
        for field_name in referent.keys() {
            if !matches!(
                field_name.as_str(),
                "type" | "denotation" | "sort" | "scopeDependence" | "source"
            ) {
                return None;
            }
        }
        consumed.insert(event.to_owned());
    }
    Some((
        new!(RelationOperand::Lexical {
            predicate: string_field_of(predication, "relation")?.to_owned(),
            participant_place,
            fixed_arguments,
            unfilled_places,
            has_event: event.is_some(),
        }),
        event.into_iter().map(str::to_owned).collect(),
        consumed,
    ))
}

/// Project one modifier property body formula into a relation operand. `parameter`
/// is the enclosing property's slot; every leaf's participant must fill it.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(_, consumed)| consumed.contains(body_id)))]
fn project_modifier_body(
    objects: &Map<String, Value>,
    body_id: &str,
    parameter: &str,
) -> Option<(RelationOperand, BTreeSet<String>)> {
    let body = object_of(objects.get(body_id)?)?;
    let operator = string_field_of(body, "operator")?;
    let mut consumed: BTreeSet<String> = [body_id.to_owned()].into_iter().collect();
    if operator == "atom" {
        let predication_id = string_field_of(body, "predication")?;
        let predication = object_of(objects.get(predication_id)?)?;
        if !is_object_type(predication, "predication")
            || string_field_of(predication, "relation").is_none()
        {
            return None;
        }
        let locally_bound: HashSet<String> = bound_eventualities(body)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let (operand, events, leaf_consumed) = recognize_lexical_leaf(
            objects,
            predication_id,
            predication,
            parameter,
            &locally_bound,
        )?;
        // Every eventuality this atom binds must be the leaf's own event.
        if locally_bound.len() != events.len() {
            return None;
        }
        consumed.extend(leaf_consumed);
        return Some((operand, consumed));
    }
    if operator == "and" && is_implicit_connector(body) {
        // A nested tanru inside the property body: default left grouping, so
        // the nested composition carries no grouping mark.
        let nested = recognize_tanru_and(objects, body_id, Some(parameter))?;
        let nested_data = nested.into_data();
        consumed.extend(nested_data.consumed);
        consumed.insert(nested_data.anchor.clone());
        return Some((nested_data.composition, consumed));
    }
    // A je-style connected modifier: a formula connective over co-modifier
    // leaves sharing the same property parameter.
    let connector = field(body, "connector")?.as_object()?;
    let source = field(connector, "source")?.as_object()?;
    if string_field_of(source, "kind") != Some("surfaceWord")
        || !bound_eventualities(body).is_empty()
    {
        return None;
    }
    if let Some(table) = string_field_of(connector, "truthTable") {
        if canonical_truth_table(operator) != Some(table) {
            return None;
        }
    }
    if connector.contains_key("parameter") {
        return None;
    }
    let children = field(body, "children")?.as_array()?;
    if children.len() < 2 {
        return None;
    }
    let mut operands = Vec::with_capacity(children.len());
    for child in children {
        let child_id = string_of(child)?;
        let child_formula = object_of(objects.get(child_id)?)?;
        if !is_object_type(child_formula, "formula")
            || string_field_of(child_formula, "operator") != Some("atom")
        {
            return None;
        }
        let predication_id = string_field_of(child_formula, "predication")?;
        let predication = object_of(objects.get(predication_id)?)?;
        if !is_object_type(predication, "predication")
            || string_field_of(predication, "relation").is_none()
        {
            return None;
        }
        let locally_bound: HashSet<String> = bound_eventualities(child_formula)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let (operand, events, leaf_consumed) = recognize_lexical_leaf(
            objects,
            predication_id,
            predication,
            parameter,
            &locally_bound,
        )?;
        if locally_bound.len() != events.len() {
            return None;
        }
        consumed.insert(child_id.to_owned());
        consumed.extend(leaf_consumed);
        operands.push(operand);
    }
    Some((
        new!(RelationOperand::Connective {
            operator: operator.to_owned(),
            operands,
        }),
        consumed,
    ))
}

/// Project the modifier side of a proven tanru link. Falls back to a reference
/// to the surviving relation referent whenever the compact guards reject the
/// property body (NU abstractions, be-link structure, shared state, ...).
/// `allowed_referrers` names the link-side objects whose references to the
/// modifier relation are expected (they are consumed by the enclosing match).
#[requires(true)]
#[ensures(true)]
fn project_modifier(
    objects: &Map<String, Value>,
    modifier: &str,
    allowed_referrers: &HashSet<String>,
) -> (RelationOperand, BTreeSet<String>) {
    let compact = (|| {
        let relation = object_of(objects.get(modifier)?)?;
        if !is_object_type(relation, "referent")
            || string_field_of(relation, "sort") != Some("relation")
            || field(relation, "arity")? != &Value::from(1)
        {
            return None;
        }
        let parameters = field(relation, "parameters")?.as_array()?;
        let [parameter] = parameters.as_slice() else {
            return None;
        };
        let parameter = string_of(parameter)?;
        let parameter_object = object_of(objects.get(parameter)?)?;
        if !is_object_type(parameter_object, "parameter")
            || string_field_of(parameter_object, "role") != Some("propertySlot")
        {
            return None;
        }
        for field_name in parameter_object.keys() {
            if !matches!(
                field_name.as_str(),
                "type" | "sort" | "role" | "introducedBy" | "source"
            ) {
                return None;
            }
        }
        let body_id = string_field_of(relation, "body")?;
        let (operand, mut consumed) = project_modifier_body(objects, body_id, parameter)?;
        consumed.insert(modifier.to_owned());
        consumed.insert(parameter.to_owned());
        Some((operand, consumed))
    })();
    match compact {
        Some((operand, consumed)) => {
            // The compact form consumes the whole modifier subtree, so nothing
            // outside it may reference any of its objects, and no formula it
            // consumes may bind an eventuality that survives.
            let closed = objects.iter().all(|(key, value)| {
                if consumed.contains(key) || allowed_referrers.contains(key) {
                    return true;
                }
                let serialized = value.to_string();
                !consumed
                    .iter()
                    .any(|id| serialized.contains(format!("\"{id}\"").as_str()))
            }) && consumed.iter().all(|id| {
                objects
                    .get(id)
                    .and_then(Value::as_object)
                    .filter(|object| is_object_type(object, "formula"))
                    .is_none_or(|formula| {
                        bound_eventualities(formula)
                            .into_iter()
                            .all(|event| consumed.contains(event))
                    })
            });
            if closed {
                (operand, consumed)
            } else {
                (
                    new!(RelationOperand::Reference {
                        relation: modifier.to_owned(),
                    }),
                    BTreeSet::new(),
                )
            }
        }
        None => (
            new!(RelationOperand::Reference {
                relation: modifier.to_owned(),
            }),
            BTreeSet::new(),
        ),
    }
}

/// Recognize one tanru AND formula. With `nested_parameter = Some(p)` the match
/// is nested inside a property body: its head projects as a lexical leaf over
/// `p` and its own bound eventualities must be exactly its head event. With
/// `None` (or as the head side of a larger tanru) the head is the surviving
/// predication and the AND's bindings are preserved on the rewritten formula.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|m| !m.consumed.contains(&m.anchor)))]
fn recognize_tanru_and(
    objects: &Map<String, Value>,
    anchor: &str,
    nested_parameter: Option<&str>,
) -> Option<TanruMatch> {
    let formula = object_of(objects.get(anchor)?)?;
    if !is_object_type(formula, "formula") || string_field_of(formula, "operator") != Some("and") {
        return None;
    }
    if !is_implicit_connector(formula) {
        return None;
    }
    let connector = field(formula, "connector")?.as_object()?;
    if connector.contains_key("truthTable") || connector.contains_key("parameter") {
        return None;
    }
    let children = field(formula, "children")?.as_array()?;
    let [head_id, link_id] = children.as_slice() else {
        return None;
    };
    let head_id = string_of(head_id)?;
    let link_id = string_of(link_id)?;

    // The link side: an atom of a composition predication whose tanruLink
    // points back at the head predication and at the modifier relation.
    let link_formula = object_of(objects.get(link_id)?)?;
    if !is_object_type(link_formula, "formula")
        || string_field_of(link_formula, "operator") != Some("atom")
        || !bound_eventualities(link_formula).is_empty()
    {
        return None;
    }
    let link_predication_id = string_field_of(link_formula, "predication")?;
    let link = object_of(objects.get(link_predication_id)?)?;
    if !is_object_type(link, "predication")
        || link.contains_key("relation")
        || link.contains_key("relationParameter")
        || link.contains_key("eventuality")
        || has_non_flat_fields(link)
    {
        return None;
    }
    let tanru_link = field(link, "tanruLink")?.as_object()?;
    let link_head = string_field_of(tanru_link, "head")?;
    let modifier = string_field_of(tanru_link, "modifier")?;
    let arguments = field(link, "arguments")?.as_object()?;
    if arguments.len() != 2 {
        return None;
    }
    let x1 = field(arguments, "x1")?.as_object()?;
    let x2 = field(arguments, "x2")?.as_object()?;
    if string_field_of(x1, "kind") != Some("filled")
        || string_field_of(x2, "kind") != Some("filled")
        || string_field_of(x2, "value") != Some(modifier)
        || x1.contains_key("introducedBy")
        || x2.contains_key("introducedBy")
    {
        return None;
    }
    let participant = string_field_of(x1, "value")?;
    // The link predication's mode must equal the head predication's mode: the
    // compact form states one mode on the composed predication, so unequal
    // modes cannot compact (re-expansion synthesizes the link mode from the
    // head).
    let link_mode = string_field_of(link, "mode")?;

    let mut consumed: BTreeSet<String> = [link_id.to_owned(), link_predication_id.to_owned()]
        .into_iter()
        .collect();

    // The head side: either the surviving predication (possibly itself a
    // nested tanru AND, the bo/ke right-grouping case), or — when this match
    // is nested inside a property body — a lexical leaf over the parameter.
    let (kind, head_predication, head_bound) = if nested_parameter.is_some() {
        let head_formula = object_of(objects.get(head_id)?)?;
        if !is_object_type(head_formula, "formula")
            || string_field_of(head_formula, "operator") != Some("atom")
        {
            return None;
        }
        let head_predication_id = string_field_of(head_formula, "predication")?;
        if head_predication_id != link_head {
            return None;
        }
        let head_predication = object_of(objects.get(head_predication_id)?)?;
        if !is_object_type(head_predication, "predication")
            || string_field_of(head_predication, "relation").is_none()
            || string_field_of(head_predication, "mode") != Some("restrictive")
            || link_mode != "restrictive"
        {
            return None;
        }
        // The nested AND's own bindings must be exactly the head leaf's event.
        let and_bound: HashSet<String> = bound_eventualities(formula)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let (operand, events, head_consumed) = recognize_lexical_leaf(
            objects,
            head_predication_id,
            head_predication,
            nested_parameter?,
            &and_bound,
        )?;
        if and_bound.len() != events.len() {
            return None;
        }
        consumed.insert(head_id.to_owned());
        consumed.extend(head_consumed);
        (operand, head_predication_id.to_owned(), Vec::new())
    } else {
        let head_formula = object_of(objects.get(head_id)?)?;
        if is_object_type(head_formula, "formula")
            && string_field_of(head_formula, "operator") == Some("atom")
        {
            let head_predication_id = string_field_of(head_formula, "predication")?;
            if head_predication_id != link_head {
                return None;
            }
            let head_predication = object_of(objects.get(head_predication_id)?)?;
            if !is_object_type(head_predication, "predication")
                || string_field_of(head_predication, "relation").is_none()
                || head_predication.contains_key("tanruLink")
                || head_predication.contains_key("relationExpression")
                || string_field_of(head_predication, "mode") != Some(link_mode)
            {
                return None;
            }
            // The participant place is the unique head place whose value is
            // the link's x1; ambiguity rejects the projection.
            let head_arguments = field(head_predication, "arguments")?.as_object()?;
            let mut participant_place = None;
            for (key, argument) in head_arguments {
                let argument = argument.as_object()?;
                if string_field_of(argument, "kind") == Some("filled")
                    && string_field_of(argument, "value") == Some(participant)
                {
                    if participant_place.is_some() {
                        return None;
                    }
                    participant_place = Some(argument_place(key));
                }
            }
            let participant_place = participant_place.filter(|place| *place > 0)?;
            consumed.insert(head_id.to_owned());
            let head_bound = bound_eventualities(head_formula)
                .into_iter()
                .map(str::to_owned)
                .collect();
            (
                new!(RelationOperand::Host { participant_place }),
                head_predication_id.to_owned(),
                head_bound,
            )
        } else if is_object_type(head_formula, "formula")
            && string_field_of(head_formula, "operator") == Some("and")
            && is_implicit_connector(head_formula)
        {
            // bo/ke right-grouping: the nested composition sits on the head
            // side and shares this tanru's head predication; its edge is the
            // explicit one.
            let nested = recognize_tanru_and(objects, head_id, None)?;
            if nested.head_predication != link_head || nested.participant != participant {
                return None;
            }
            // The outer link's mode must equal the shared head predication's
            // mode, just like a plain head's link.
            let shared_head = object_of(objects.get(link_head)?)?;
            if string_field_of(shared_head, "mode") != Some(link_mode) {
                return None;
            }
            // The nested AND's bindings must not escape: they may reference
            // only objects the nested match itself consumes.
            for event in bound_eventualities(head_formula) {
                if !nested.consumed.contains(event) {
                    return None;
                }
            }
            let nested_data = nested.into_data();
            let head_predication = nested_data.head_predication.clone();
            consumed.extend(nested_data.consumed);
            consumed.insert(head_id.to_owned());
            let data!(RelationOperand::Composition { kind, modifier, .. }) =
                nested_data.composition.into_data()
            else {
                unreachable!("a recognized tanru always produces a composition");
            };
            (
                new!(RelationOperand::Composition {
                    grouping: Some(CompositionGrouping::Explicit),
                    kind,
                    modifier,
                }),
                head_predication,
                Vec::new(),
            )
        } else {
            return None;
        }
    };

    let allowed_referrers: HashSet<String> = [link_id.to_owned(), link_predication_id.to_owned()]
        .into_iter()
        .collect();
    let (modifier_operand, modifier_consumed) =
        project_modifier(objects, modifier, &allowed_referrers);
    // The compact modifier form is consumed here, so nothing it consumes may
    // be referenced by objects outside this whole match (the anchor may
    // reference its own children only).
    for (key, value) in objects {
        if key == anchor || consumed.contains(key) || modifier_consumed.contains(key) {
            continue;
        }
        let serialized = value.to_string();
        if consumed
            .iter()
            .chain(modifier_consumed.iter())
            .any(|id| serialized.contains(format!("\"{id}\"").as_str()))
        {
            return None;
        }
    }
    consumed.extend(modifier_consumed);

    let composition = normalize_grouping(new!(RelationOperand::Composition {
        grouping: None,
        kind: Box::new(kind),
        modifier: Box::new(modifier_operand),
    }));

    // The rewritten anchor carries the AND's own bindings plus the plain head
    // atom's (nested matches already consumed theirs).
    let mut rewritten_bound: Vec<String> = bound_eventualities(formula)
        .into_iter()
        .map(str::to_owned)
        .collect();
    rewritten_bound.extend(head_bound);

    Some(new!(TanruMatch {
        anchor: anchor.to_owned(),
        head_predication,
        participant: participant.to_owned(),
        rewritten_bound,
        composition,
        consumed,
    }))
}

/// Tree-level grouping rollup: a uniformly explicit tree states EXPLICIT at the
/// root only; a mixed tree states EXPLICIT per affected node (silence is
/// ASSUMED-LEFT everywhere else).
#[requires(true)]
#[ensures(true)]
fn normalize_grouping(composition: RelationOperand) -> RelationOperand {
    #[requires(true)]
    #[ensures(true)]
    fn all_explicit(operand: &RelationOperand) -> bool {
        match operand.as_data() {
            data!(RelationOperand::Composition {
                grouping,
                kind,
                modifier
            }) => {
                *grouping == Some(CompositionGrouping::Explicit)
                    && all_explicit(kind)
                    && all_explicit(modifier)
            }
            data!(RelationOperand::Connective { operands, .. }) => {
                operands.iter().all(all_explicit)
            }
            _ => true,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn clear_grouping(operand: RelationOperand) -> RelationOperand {
        match operand.into_data() {
            data!(RelationOperand::Composition { kind, modifier, .. }) => {
                new!(RelationOperand::Composition {
                    grouping: None,
                    kind: Box::new(clear_grouping(*kind)),
                    modifier: Box::new(clear_grouping(*modifier)),
                })
            }
            data => RelationOperand::from_data(data),
        }
    }

    if all_explicit(&composition) {
        let cleared = clear_grouping(composition);
        let data!(RelationOperand::Composition { kind, modifier, .. }) = cleared.into_data() else {
            unreachable!("the root of a projected view is always a composition");
        };
        new!(RelationOperand::Composition {
            grouping: Some(CompositionGrouping::Explicit),
            kind,
            modifier,
        })
    } else {
        composition
    }
}

/// Project every proven tanru pattern in `objects`. Returns the transformed
/// objects map (AND formulas rewritten to head atoms, head predications
/// carrying their `relationExpression` view, consumed scaffolding removed) and
/// the projection report for omission accounting and the acceptance test.
#[requires(true)]
#[ensures(ret.0.len() <= objects.len())]
pub(crate) fn project_tanru_compositions(
    objects: &Map<String, Value>,
) -> (Map<String, Value>, TanruProjection) {
    let mut candidates: Vec<&str> = objects
        .iter()
        .filter(|(_, value)| {
            value.as_object().is_some_and(|object| {
                is_object_type(object, "formula")
                    && string_field_of(object, "operator") == Some("and")
                    && is_implicit_connector(object)
            })
        })
        .map(|(key, _)| key.as_str())
        .collect();
    candidates.sort_unstable();

    let mut consumed_global: HashSet<String> = HashSet::new();
    let mut matches = Vec::new();
    // Process outermost candidates first: an implicit AND nested inside another
    // candidate's modifier property must be consumed by that parent's
    // projection, never projected standalone ahead of it. Depth = how many
    // other candidates' modifier walks contain the anchor.
    let depth_of = |anchor: &str| -> usize {
        candidates
            .iter()
            .filter(|other| **other != anchor)
            .filter(|other| tanru_region_walk(objects, other).contains(anchor))
            .count()
    };
    let mut ordered = candidates.clone();
    ordered.sort_by_key(|anchor| depth_of(anchor));
    for candidate in ordered {
        if consumed_global.contains(candidate) {
            continue;
        }
        // A candidate nested inside an already-projected match is consumed by
        // its parent; overlapping independent matches cannot occur by
        // construction (the consumed sets are disjoint graph regions).
        if let Some(result) = recognize_tanru_and(objects, candidate, None) {
            if result
                .consumed
                .iter()
                .any(|id| consumed_global.contains(id))
            {
                continue;
            }
            consumed_global.extend(result.consumed.iter().cloned());
            matches.push(result);
        }
    }

    let mut transformed = objects.clone();
    let mut instances = Vec::with_capacity(matches.len());
    let mut consumed_objects = Map::new();
    let mut rewritten_anchors = Map::new();
    for result in matches {
        for id in &result.consumed {
            if let Some(original) = transformed.remove(id) {
                consumed_objects.insert(id.clone(), original);
            }
        }
        rewritten_anchors.insert(result.anchor.clone(), transformed[&result.anchor].clone());
        let anchor_object = transformed
            .get_mut(&result.anchor)
            .and_then(Value::as_object_mut)
            .unwrap_or_else(|| panic!("tanru anchor must survive: {}", result.anchor));
        let source = anchor_object.get("source").cloned();
        let mut rewritten = Map::new();
        rewritten.insert("type".to_owned(), Value::from("formula"));
        rewritten.insert("operator".to_owned(), Value::from("atom"));
        rewritten.insert(
            "predication".to_owned(),
            Value::from(result.head_predication.as_str()),
        );
        if !result.rewritten_bound.is_empty() {
            rewritten.insert(
                "boundEventualities".to_owned(),
                Value::Array(
                    result
                        .rewritten_bound
                        .iter()
                        .map(|event| Value::from(event.as_str()))
                        .collect(),
                ),
            );
        }
        if let Some(source) = source {
            rewritten.insert("source".to_owned(), source);
        }
        *anchor_object = rewritten;
        let head_object = transformed
            .get_mut(&result.head_predication)
            .and_then(Value::as_object_mut)
            .unwrap_or_else(|| {
                panic!(
                    "tanru head predication must survive: {}",
                    result.head_predication
                )
            });
        let view = serde_json::to_value(&result.composition)
            .expect("relation-expression view serialization cannot fail");
        head_object.insert("relationExpression".to_owned(), view);
        let result = result.into_data();
        instances.push(new!(ProjectedInstance {
            anchor: result.anchor,
            head_predication: result.head_predication,
            consumed: result.consumed,
            composition: result.composition,
        }));
    }

    (
        transformed,
        new!(TanruProjection {
            instances,
            consumed_objects,
            rewritten_anchors,
        }),
    )
}

/// The re-expansion elaboration contract and its acceptance test (jbotci#719,
/// mandated by the round-14 decision): re-expanding the *rendered* relation
/// expression — the KIND-COMPOSITION/BODY/RELATION/PARTICIPANT-PLACE surface
/// of the emitted SFN-XML — must reproduce the recognized subgraph modulo
/// opaque ids and waived provenance. The re-expander is a second, independent
/// implementation of the tanru graph shape; the comparer pairs object ids by
/// BFS encounter order so only structure, field values, and reference topology
/// are compared. Negative fixtures prove the loud/composite fallbacks fire
/// for unequal link/head modes, shared or decorated modifier eventualities,
/// and non-flat place maps.
#[cfg(test)]
pub(crate) mod reexpansion {
    use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

    use serde_json::{Map, Value};

    #[allow(unused_imports)]
    use bityzba::{data, ensures, invariant, new, requires};
    use jbotci_dictionary::Dictionary;

    use super::{
        ProjectedInstance, RelationOperand, RelationOperandData, canonical_truth_table,
        project_tanru_compositions,
    };

    /// Deterministic fresh id source for regenerated scaffolding objects.
    #[requires(true)]
    #[ensures(ret.starts_with(prefix))]
    fn fresh_id(next: &mut usize, prefix: &str) -> String {
        *next += 1;
        format!("{prefix}:{}", 900_000 + *next)
    }

    #[requires(true)]
    #[ensures(true)]
    fn object(fields: &[(&str, Value)]) -> Value {
        Value::Object(
            fields
                .iter()
                .map(|(name, value)| ((*name).to_owned(), value.clone()))
                .collect(),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn id_list(ids: &[String]) -> Value {
        Value::Array(ids.iter().map(|id| Value::from(id.as_str())).collect())
    }

    /// The scope-dependence record of a generated relation referent or elided
    /// referent: `fixed` when no property parameters enclose it, otherwise an
    /// underspecified dependence on exactly the enclosing parameter stack.
    #[requires(true)]
    #[ensures(true)]
    fn scope_dependence(enclosing_parameters: &[String]) -> Value {
        if enclosing_parameters.is_empty() {
            object(&[("kind", Value::from("fixed"))])
        } else {
            object(&[
                ("kind", Value::from("underspecified")),
                ("mayDependOn", id_list(enclosing_parameters)),
            ])
        }
    }

    /// Regenerate one lexical leaf predication and its atom formula: the
    /// participant fills `parameter`, fixed places keep their targets, every
    /// unfilled place elaborates to a distinct ordinary elided (zo'e) node,
    /// and the leaf eventuality is fresh and locally bound.
    #[requires(!parameter.is_empty() && !predicate.is_empty() && participant_place > 0)]
    #[ensures(true)]
    fn expand_lexical_leaf(
        next: &mut usize,
        predicate: &str,
        participant_place: usize,
        fixed_arguments: &BTreeMap<usize, String>,
        unfilled_places: &[usize],
        has_event: bool,
        parameter: &str,
        enclosing_parameters: &[String],
        bind_event_at_atom: bool,
        out: &mut Map<String, Value>,
    ) -> String {
        let mut arguments = Map::new();
        arguments.insert(
            format!("x{participant_place}"),
            object(&[
                ("kind", Value::from("filled")),
                ("value", Value::from(parameter)),
            ]),
        );
        for (place, target) in fixed_arguments {
            arguments.insert(
                format!("x{place}"),
                object(&[
                    ("kind", Value::from("filled")),
                    ("value", Value::from(target.as_str())),
                ]),
            );
        }
        for place in unfilled_places {
            let referent = fresh_id(next, "entity");
            out.insert(
                referent.clone(),
                object(&[
                    ("type", Value::from("referent")),
                    ("category", Value::from("constant")),
                    ("scopeDependence", scope_dependence(enclosing_parameters)),
                    ("sort", Value::from("entity")),
                    (
                        "descriptor",
                        object(&[
                            ("kind", Value::from("elided")),
                            ("word", Value::from("zo'e")),
                        ]),
                    ),
                ]),
            );
            arguments.insert(
                format!("x{place}"),
                object(&[
                    ("kind", Value::from("elided")),
                    ("value", Value::from(referent.as_str())),
                    ("introducedBy", Value::from("zo'e")),
                ]),
            );
        }
        let eventuality = has_event.then(|| fresh_id(next, "eventuality"));
        if let Some(eventuality) = &eventuality {
            out.insert(
                eventuality.clone(),
                object(&[
                    ("type", Value::from("referent")),
                    ("denotation", Value::from("generated-bound")),
                    ("sort", Value::from("eventuality")),
                ]),
            );
        }
        let predication = fresh_id(next, "predication");
        let mut fields = Vec::from([
            ("type", Value::from("predication")),
            ("relation", Value::from(predicate)),
            ("arguments", Value::Object(arguments)),
            ("mode", Value::from("restrictive")),
        ]);
        if let Some(eventuality) = &eventuality {
            fields.insert(1, ("eventuality", Value::from(eventuality.as_str())));
        }
        out.insert(predication.clone(), object(&fields));
        let atom = fresh_id(next, "formula");
        let mut atom_fields = Vec::from([
            ("type", Value::from("formula")),
            ("operator", Value::from("atom")),
            ("predication", Value::from(predication.as_str())),
        ]);
        if bind_event_at_atom && let Some(eventuality) = &eventuality {
            atom_fields.push(("boundEventualities", id_list(&[eventuality.clone()])));
        }
        out.insert(atom.clone(), object(&atom_fields));
        atom
    }

    /// Decompose one operand into the flat pieces the leaf expander needs,
    /// panicking on non-lexical operands.
    #[requires(true)]
    #[ensures(true)]
    fn expand_lexical_operand(
        next: &mut usize,
        operand: &RelationOperand,
        parameter: &str,
        enclosing_parameters: &[String],
        bind_event_at_atom: bool,
        out: &mut Map<String, Value>,
    ) -> String {
        let data!(RelationOperand::Lexical {
            predicate,
            participant_place,
            fixed_arguments,
            unfilled_places,
            has_event,
        }) = operand.as_data()
        else {
            panic!("expand_lexical_operand requires a lexical operand");
        };
        expand_lexical_leaf(
            next,
            predicate,
            *participant_place,
            fixed_arguments,
            unfilled_places,
            *has_event,
            parameter,
            enclosing_parameters,
            bind_event_at_atom,
            out,
        )
    }

    /// Regenerate the modifier property abstraction for one operand, returning
    /// the relation referent id. A `Reference` operand survives in the graph,
    /// so its id is returned unchanged with no new objects.
    #[requires(true)]
    #[ensures(true)]
    fn expand_modifier(
        next: &mut usize,
        operand: &RelationOperand,
        enclosing_parameters: &[String],
        out: &mut Map<String, Value>,
    ) -> String {
        if let data!(RelationOperand::Reference { relation }) = operand.as_data() {
            return relation.clone();
        }
        let parameter = fresh_id(next, "parameter");
        out.insert(
            parameter.clone(),
            object(&[
                ("type", Value::from("parameter")),
                ("sort", Value::from("entity")),
                ("role", Value::from("propertySlot")),
                ("introducedBy", Value::from("ce'u")),
            ]),
        );
        let mut nested_parameters = enclosing_parameters.to_vec();
        nested_parameters.push(parameter.clone());
        let body = match operand.as_data() {
            data!(RelationOperand::Lexical { .. }) => {
                expand_lexical_operand(next, operand, &parameter, &nested_parameters, true, out)
            }
            data!(RelationOperand::Composition { .. }) => expand_composition(
                next,
                operand,
                None,
                &parameter,
                "restrictive",
                &nested_parameters,
                out,
            ),
            data!(RelationOperand::Connective { operator, operands }) => {
                let children: Vec<String> = operands
                    .iter()
                    .map(|operand| {
                        expand_lexical_operand(
                            next,
                            operand,
                            &parameter,
                            &nested_parameters,
                            true,
                            out,
                        )
                    })
                    .collect();
                let formula = fresh_id(next, "formula");
                let mut connector = Map::new();
                if let Some(table) = canonical_truth_table(operator) {
                    connector.insert("truthTable".to_owned(), Value::from(table));
                }
                out.insert(
                    formula.clone(),
                    object(&[
                        ("type", Value::from("formula")),
                        ("operator", Value::from(operator.as_str())),
                        ("children", id_list(&children)),
                        ("connector", Value::Object(connector)),
                    ]),
                );
                formula
            }
            data!(RelationOperand::Host { .. }) => {
                panic!("a host operand cannot head a modifier property")
            }
            data!(RelationOperand::Reference { .. }) => unreachable!("handled above"),
        };
        let relation = fresh_id(next, "relation");
        out.insert(
            relation.clone(),
            object(&[
                ("type", Value::from("referent")),
                ("category", Value::from("constant")),
                ("scopeDependence", scope_dependence(enclosing_parameters)),
                ("sort", Value::from("relation")),
                ("body", Value::from(body.as_str())),
                ("parameters", id_list(&[parameter.clone()])),
                ("arity", Value::from(1)),
            ]),
        );
        relation
    }

    /// Regenerate the tanru AND formula for one composition node. `surviving_head`
    /// is the head predication id for `Host` operands (present at the root and
    /// at bo/ke head-side nesting); `participant` is the link x1 value.
    #[requires(!participant.is_empty())]
    #[ensures(true)]
    fn expand_composition(
        next: &mut usize,
        composition: &RelationOperand,
        surviving_head: Option<&str>,
        participant: &str,
        mode: &str,
        enclosing_parameters: &[String],
        out: &mut Map<String, Value>,
    ) -> String {
        let data!(RelationOperand::Composition { kind, modifier, .. }) = composition.as_data()
        else {
            panic!("expand_composition requires a composition operand");
        };
        let (head_child, head_predication, head_event) = match kind.as_data() {
            data!(RelationOperand::Host { .. }) => {
                let head_predication =
                    surviving_head.expect("a host operand requires the surviving head predication");
                let atom = fresh_id(next, "formula");
                out.insert(
                    atom.clone(),
                    object(&[
                        ("type", Value::from("formula")),
                        ("operator", Value::from("atom")),
                        ("predication", Value::from(head_predication)),
                    ]),
                );
                (atom, head_predication.to_owned(), None)
            }
            data!(RelationOperand::Lexical { has_event, .. }) => {
                // A composition head's eventuality is bound by its tanru AND,
                // not by its own atom (unlike a modifier leaf's).
                let atom = expand_lexical_operand(
                    next,
                    kind,
                    participant,
                    enclosing_parameters,
                    false,
                    out,
                );
                let predication = out[&atom]["predication"]
                    .as_str()
                    .expect("atom predication")
                    .to_owned();
                let event = has_event.then(|| {
                    out[&predication]["eventuality"]
                        .as_str()
                        .expect("leaf event")
                        .to_owned()
                });
                (atom, predication, event)
            }
            data!(RelationOperand::Composition { .. }) => {
                let and = expand_composition(
                    next,
                    kind,
                    surviving_head,
                    participant,
                    mode,
                    enclosing_parameters,
                    out,
                );
                (
                    and,
                    surviving_head
                        .expect("bo nesting shares the head predication")
                        .to_owned(),
                    None,
                )
            }
            data!(RelationOperand::Connective { .. })
            | data!(RelationOperand::Reference { .. }) => {
                panic!("the kind side of a composition is never a connective or reference")
            }
        };
        let modifier_relation = expand_modifier(next, modifier, enclosing_parameters, out);
        let link_predication = fresh_id(next, "predication");
        out.insert(
            link_predication.clone(),
            object(&[
                ("type", Value::from("predication")),
                (
                    "tanruLink",
                    object(&[
                        ("head", Value::from(head_predication.as_str())),
                        ("modifier", Value::from(modifier_relation.as_str())),
                    ]),
                ),
                (
                    "arguments",
                    object(&[
                        (
                            "x1",
                            object(&[
                                ("kind", Value::from("filled")),
                                ("value", Value::from(participant)),
                            ]),
                        ),
                        (
                            "x2",
                            object(&[
                                ("kind", Value::from("filled")),
                                ("value", Value::from(modifier_relation.as_str())),
                            ]),
                        ),
                    ]),
                ),
                ("mode", Value::from(mode)),
            ]),
        );
        let link_atom = fresh_id(next, "formula");
        out.insert(
            link_atom.clone(),
            object(&[
                ("type", Value::from("formula")),
                ("operator", Value::from("atom")),
                ("predication", Value::from(link_predication.as_str())),
            ]),
        );
        let and = fresh_id(next, "formula");
        let mut and_fields = Vec::from([
            ("type", Value::from("formula")),
            ("operator", Value::from("and")),
            ("children", id_list(&[head_child, link_atom])),
            ("connector", Value::Object(Map::new())),
        ]);
        // A nested composition's AND binds its head leaf's eventuality; the
        // root AND's bindings are restored verbatim from the rewritten atom.
        if let Some(event) = head_event {
            and_fields.push(("boundEventualities", id_list(&[event])));
        }
        out.insert(and.clone(), object(&and_fields));
        and
    }

    /// The property/abstraction parameter stack enclosing a formula in the
    /// surviving graph (outermost first), found by walking body→parent
    /// links from the anchor. Needed when a nested tanru projects standalone
    /// while its enclosing property stays loud: the regenerated subtree's
    /// scope-dependence records name those surviving enclosing parameters.
    #[requires(true)]
    #[ensures(true)]
    fn surviving_enclosing_parameters(objects: &Map<String, Value>, anchor: &str) -> Vec<String> {
        let mut stack = Vec::new();
        let mut current = anchor.to_owned();
        loop {
            let parent = objects.iter().find(|(_, value)| {
                value
                    .as_object()
                    .and_then(|object| object.get("body"))
                    .and_then(Value::as_str)
                    == Some(current.as_str())
                    && value
                        .as_object()
                        .and_then(|object| object.get("parameters"))
                        .and_then(Value::as_array)
                        .is_some()
            });
            let Some((parent_id, parent)) = parent else {
                break;
            };
            let parameters: Vec<String> = parent
                .get("parameters")
                .and_then(Value::as_array)
                .expect("parent parameters")
                .iter()
                .filter_map(|parameter| parameter.as_str().map(str::to_owned))
                .collect();
            stack = parameters.into_iter().chain(stack).collect();
            current = parent_id.clone();
        }
        stack
    }

    /// The host participant place of a projected view (the kind side's
    /// innermost Host operand).
    #[requires(true)]
    #[ensures(true)]
    fn host_participant_place(composition: &RelationOperand) -> usize {
        let mut kind = composition;
        loop {
            match kind.as_data() {
                data!(RelationOperand::Composition { kind: nested, .. }) => kind = nested,
                data!(RelationOperand::Host { participant_place }) => return *participant_place,
                _ => panic!("a projected instance always bottoms out at a host operand"),
            }
        }
    }

    /// Re-expand one projected instance with the given relation-expression
    /// view: the inverse elaboration of the compact relation expression,
    /// regenerating scaffolding with fresh (opaque) ids.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn reexpand_instance(
        objects: &Map<String, Value>,
        instance: &ProjectedInstance,
        view: &RelationOperand,
        next: &mut usize,
    ) -> Map<String, Value> {
        let mut expanded = objects.clone();
        let bound = expanded[&instance.anchor]
            .get("boundEventualities")
            .and_then(Value::as_array)
            .cloned();
        let head_object = expanded
            .get_mut(&instance.head_predication)
            .and_then(Value::as_object_mut)
            .expect("head predication survives");
        head_object.remove("relationExpression");
        let mode = head_object
            .get("mode")
            .and_then(Value::as_str)
            .expect("head predication mode")
            .to_owned();
        let participant_place = host_participant_place(view);
        let participant = head_object["arguments"]
            .get(format!("x{participant_place}"))
            .and_then(|argument| argument.get("value"))
            .and_then(Value::as_str)
            .expect("participant argument")
            .to_owned();
        let enclosing = surviving_enclosing_parameters(&expanded, &instance.anchor);
        let mut regenerated = Map::new();
        let and = expand_composition(
            next,
            view,
            Some(&instance.head_predication),
            &participant,
            &mode,
            &enclosing,
            &mut regenerated,
        );
        // The regenerated AND takes the anchor's id (the anchor survives);
        // its bindings move back from the rewritten atom verbatim.
        let mut and_object = match regenerated.remove(&and) {
            Some(Value::Object(and_object)) => and_object,
            _ => panic!("regenerated AND must be an object"),
        };
        if let Some(bound) = bound {
            and_object.insert("boundEventualities".to_owned(), Value::Array(bound));
        }
        expanded.insert(instance.anchor.clone(), Value::Object(and_object));
        for (key, value) in regenerated {
            expanded.insert(key, value);
        }
        expanded
    }

    /// Re-expand every projected instance with its recognizer-produced view
    /// (used by the negative/guard fixtures; the acceptance driver re-expands
    /// the *rendered* view instead).
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn reexpand_instances(
        objects: &Map<String, Value>,
        instances: &[ProjectedInstance],
    ) -> Map<String, Value> {
        let mut expanded = objects.clone();
        let mut next = 0usize;
        for instance in instances {
            let view = instance.composition.clone();
            expanded = reexpand_instance(&expanded, instance, &view, &mut next);
        }
        expanded
    }

    /// Replicate the renderer's id assignment (`make_id` in notation::xml):
    /// a type/sort prefix plus the graph key's numeric suffix.
    #[requires(!key.is_empty())]
    #[ensures(!ret.is_empty())]
    fn make_id(key: &str, object: &Map<String, Value>) -> String {
        let prefix = if object.get("type").and_then(Value::as_str) == Some("referent") {
            if object
                .get("sort")
                .and_then(Value::as_str)
                .is_some_and(|sort| sort.starts_with("eventuality"))
            {
                "e"
            } else {
                "r"
            }
        } else {
            match object.get("type").and_then(Value::as_str) {
                Some("utterance") => "u",
                Some("predication") => "p",
                Some("formula") => "f",
                Some("quantity") => "q",
                Some("parameter") => "v",
                Some("sequence") => "s",
                Some("displayedContent") => "d",
                Some("mathExpression") => "m",
                Some("question") => "x",
                _ => "o",
            }
        };
        let suffix = key
            .rsplit_once(':')
            .map(|(_, suffix)| suffix)
            .filter(|suffix| suffix.chars().all(|character| character.is_ascii_digit()))
            .unwrap_or_else(|| panic!("graph key lacks JSON-aligned suffix: {key:?}"));
        format!("{prefix}{suffix}")
    }

    /// Rendered id (r7, e12, p9) → graph key (entity:7, eventuality:12,
    /// predication:9), for every object in the graph.
    #[requires(true)]
    #[ensures(true)]
    fn rendered_id_to_key_map(objects: &Map<String, Value>) -> HashMap<String, String> {
        objects
            .iter()
            .map(|(key, value)| {
                let object = value.as_object().expect("graph object");
                (make_id(key, object), key.clone())
            })
            .collect()
    }

    /// The elided-place list of a compact lexical leaf per the KEY's
    /// elaboration contract: every lexical place of the predicate (per the
    /// dictionary place structure) that is neither the participant nor a fixed
    /// argument elaborates to a distinct ordinary elided-place node.
    #[requires(!predicate.is_empty() && participant_place > 0)]
    #[ensures(true)]
    fn lexical_unfilled_places(
        dictionary: &Dictionary<'_>,
        predicate: &str,
        participant_place: usize,
        fixed_arguments: &BTreeMap<usize, String>,
    ) -> Vec<usize> {
        let arity = crate::dictionary_relation_place_count(dictionary, predicate)
            .unwrap_or_else(|| panic!("dictionary must know the place count of {predicate}"));
        (1..=arity)
            .filter(|place| *place != participant_place && !fixed_arguments.contains_key(place))
            .collect()
    }

    /// Parse one operand element (KIND, MODIFIER, or RELATION) of a rendered
    /// KIND-COMPOSITION back into the typed view. `host_slot` is true exactly
    /// on the root composition's kind chain (the bo/ke head side); only there
    /// does a bare PREDICATE= leaf mean the enclosing predication's own
    /// predicate.
    #[requires(true)]
    #[ensures(true)]
    fn operand_from_xml(
        element: roxmltree::Node<'_, '_>,
        host_slot: bool,
        id_to_key: &HashMap<String, String>,
        dictionary: &Dictionary<'_>,
        objects: &Map<String, Value>,
        host_relation: &str,
    ) -> RelationOperand {
        if let Some(reference) = element.attribute("REF") {
            return new!(RelationOperand::Reference {
                relation: id_to_key
                    .get(reference)
                    .unwrap_or_else(|| panic!("unknown rendered id {reference}"))
                    .clone(),
            });
        }
        if let Some(predicate) = element.attribute("PREDICATE") {
            let participant_place = element
                .attribute("PARTICIPANT-PLACE")
                .map(|place| {
                    place
                        .parse::<usize>()
                        .expect("PARTICIPANT-PLACE is numeric")
                })
                .unwrap_or(1);
            if host_slot {
                // The host leaf names the surviving head predication's own
                // relation: validate the rendered value instead of discarding
                // it, so a swapped host predicate fails the acceptance check.
                assert_eq!(
                    predicate, host_relation,
                    "rendered host predicate does not match the surviving head predication"
                );
                return new!(RelationOperand::Host { participant_place });
            }
            let mut fixed_arguments = BTreeMap::new();
            for child in element.children().filter(|child| child.is_element()) {
                assert_eq!(child.tag_name().name(), "ARG");
                let place: usize = child
                    .attribute("INDEX")
                    .expect("operand ARG INDEX")
                    .parse()
                    .expect("operand ARG INDEX is numeric");
                let target = child.attribute("REF").expect("operand ARG REF");
                fixed_arguments.insert(
                    place,
                    id_to_key
                        .get(target)
                        .unwrap_or_else(|| panic!("unknown rendered id {target}"))
                        .clone(),
                );
            }
            let unfilled_places =
                lexical_unfilled_places(dictionary, predicate, participant_place, &fixed_arguments);
            return new!(RelationOperand::Lexical {
                predicate: predicate.to_owned(),
                participant_place,
                fixed_arguments,
                unfilled_places,
                has_event: true,
            });
        }
        let body = element
            .children()
            .find(|child| child.is_element() && child.tag_name().name() == "BODY")
            .unwrap_or_else(|| {
                panic!(
                    "operand {} has no PREDICATE=, REF=, or BODY",
                    element.tag_name().name()
                )
            });
        let content = body
            .children()
            .find(roxmltree::Node::is_element)
            .expect("BODY wraps one relation-expression subtree");
        match content.tag_name().name() {
            "KIND-COMPOSITION" => composition_from_xml(
                content,
                host_slot,
                id_to_key,
                dictionary,
                objects,
                host_relation,
            ),
            "CONNECTIVE" => {
                let operator = match content.attribute("OPERATOR").expect("CONNECTIVE OPERATOR") {
                    "AND" => "and",
                    "OR" => "or",
                    other => panic!("unknown relation connective operator {other}"),
                };
                let operands: Vec<RelationOperand> = content
                    .children()
                    .filter(|child| child.is_element() && child.tag_name().name() == "RELATION")
                    .map(|leaf| {
                        operand_from_xml(leaf, false, id_to_key, dictionary, objects, host_relation)
                    })
                    .collect();
                new!(RelationOperand::Connective {
                    operator: operator.to_owned(),
                    operands,
                })
            }
            // A composite RELATION-lambda/abstraction subtree rendered inline:
            // the relation it defines survives in the graph.
            _ => {
                let id = content.attribute("ID").map(str::to_owned);
                let relation = match id {
                    Some(id) => id_to_key
                        .get(id.as_str())
                        .unwrap_or_else(|| panic!("unknown rendered id {id}"))
                        .clone(),
                    // A single-use relation renders inline without an ID:
                    // identify it by its parameter list (the rendered
                    // PARAMETERS= are the property parameters' ids).
                    None => {
                        assert_eq!(content.tag_name().name(), "RELATION");
                        let parameters: Vec<&str> = content
                            .attribute("PARAMETERS")
                            .expect("inline RELATION carries PARAMETERS")
                            .split(' ')
                            .collect();
                        let parameter_keys: Vec<&str> = parameters
                            .iter()
                            .map(|rendered| {
                                id_to_key
                                    .get(*rendered)
                                    .map(String::as_str)
                                    .unwrap_or_else(|| panic!("unknown rendered id {rendered}"))
                            })
                            .collect();
                        objects
                            .iter()
                            .find(|(_, value)| {
                                let Some(object) = value.as_object() else {
                                    return false;
                                };
                                object.get("sort").and_then(Value::as_str) == Some("relation")
                                    && object
                                        .get("parameters")
                                        .and_then(Value::as_array)
                                        .is_some_and(|object_parameters| {
                                            object_parameters.len() == parameter_keys.len()
                                                && object_parameters
                                                    .iter()
                                                    .zip(parameter_keys.iter())
                                                    .all(|(object_parameter, parameter_key)| {
                                                        object_parameter.as_str()
                                                            == Some(*parameter_key)
                                                    })
                                        })
                            })
                            .map(|(key, _)| key.clone())
                            .expect("inline composite relation matches a graph relation")
                    }
                };
                new!(RelationOperand::Reference { relation })
            }
        }
    }

    /// Parse a rendered KIND-COMPOSITION element back into the typed
    /// relation-expression view.
    #[requires(element.tag_name().name() == "KIND-COMPOSITION")]
    #[ensures(true)]
    fn composition_from_xml(
        element: roxmltree::Node<'_, '_>,
        host_slot: bool,
        id_to_key: &HashMap<String, String>,
        dictionary: &Dictionary<'_>,
        objects: &Map<String, Value>,
        host_relation: &str,
    ) -> RelationOperand {
        let grouping = match element.attribute("GROUPING") {
            None => None,
            Some("EXPLICIT") => Some(super::CompositionGrouping::Explicit),
            Some(other) => panic!("unknown GROUPING basis {other}"),
        };
        let kind = element
            .children()
            .find(|child| child.is_element() && child.tag_name().name() == "KIND")
            .expect("KIND-COMPOSITION has a KIND child");
        let modifier = element
            .children()
            .find(|child| child.is_element() && child.tag_name().name() == "MODIFIER")
            .expect("KIND-COMPOSITION has a MODIFIER child");
        new!(RelationOperand::Composition {
            grouping,
            kind: Box::new(operand_from_xml(
                kind,
                host_slot,
                id_to_key,
                dictionary,
                objects,
                host_relation
            )),
            modifier: Box::new(operand_from_xml(
                modifier,
                false,
                id_to_key,
                dictionary,
                objects,
                host_relation
            )),
        })
    }

    /// Locate the PREDICATION element a projected instance rendered as: by its
    /// rendered ID when defined, otherwise by exact MODE/ARG surface match
    /// against the surviving head predication.
    #[requires(true)]
    #[ensures(true)]
    fn find_projected_predication<'a>(
        document: &'a roxmltree::Document<'a>,
        instance: &ProjectedInstance,
        objects: &Map<String, Value>,
        id_to_key: &HashMap<String, String>,
    ) -> roxmltree::Node<'a, 'a> {
        let head_object = objects[&instance.head_predication]
            .as_object()
            .expect("head predication object");
        let expected_id = make_id(&instance.head_predication, head_object);
        let mut candidates: Vec<roxmltree::Node<'a, 'a>> = document
            .descendants()
            .filter(|node| {
                node.is_element()
                    && node.tag_name().name() == "PREDICATION"
                    && node
                        .children()
                        .find(roxmltree::Node::is_element)
                        .is_some_and(|first| first.tag_name().name() == "KIND-COMPOSITION")
            })
            .collect();
        if let Some(exact) = candidates
            .iter()
            .find(|node| node.attribute("ID") == Some(expected_id.as_str()))
        {
            return *exact;
        }
        let head_mode = head_object
            .get("mode")
            .and_then(Value::as_str)
            .expect("head predication mode")
            .to_uppercase();
        let head_arguments: BTreeMap<usize, String> = head_object["arguments"]
            .as_object()
            .expect("head arguments")
            .iter()
            .map(|(place, argument)| {
                let place: usize = place
                    .strip_prefix('x')
                    .and_then(|digits| digits.parse().ok())
                    .expect("numeric argument place");
                let argument = argument.as_object().expect("argument object");
                let rendered = match argument.get("kind").and_then(Value::as_str) {
                    Some("filled") => id_to_key
                        .iter()
                        .find(|(_, key)| {
                            **key == argument["value"].as_str().expect("argument value")
                        })
                        .map(|(rendered, _)| rendered.clone())
                        .expect("argument target renders"),
                    Some("elided") => "SOME".to_owned(),
                    other => panic!("unexpected head argument kind {other:?}"),
                };
                (place, rendered)
            })
            .collect();
        candidates.retain(|node| {
            if node.attribute("MODE") != Some(head_mode.as_str()) {
                return false;
            }
            let rendered_arguments: BTreeMap<usize, String> = node
                .children()
                .filter(|child| child.is_element() && child.tag_name().name() == "ARG")
                .filter_map(|argument| {
                    let place: usize = argument
                        .attribute("INDEX")
                        .expect("ARG INDEX")
                        .parse()
                        .expect("ARG INDEX numeric");
                    // Inline arguments (a single-use referent rendered in
                    // place) carry no REF=; they cannot disambiguate, so only
                    // reference-form arguments constrain the match.
                    let reference = argument.attribute("REF")?.to_owned();
                    Some((place, reference))
                })
                .collect();
            rendered_arguments
                .iter()
                .all(|(place, reference)| head_arguments.get(place) == Some(reference))
        });
        match candidates.as_slice() {
            [exact] => *exact,
            [] => panic!(
                "no PREDICATION element matches projected head {}",
                instance.head_predication
            ),
            _ => panic!(
                "ambiguous PREDICATION elements for projected head {}",
                instance.head_predication
            ),
        }
    }

    /// Provenance fields dropped before equivalence comparison: source records,
    /// introducer markers, the synthesized relation label, connector surface
    /// word and locus, and the relation-expression view itself.
    const PROVENANCE_FIELDS: &[&str] = &[
        "source",
        "introducedBy",
        "relationLabel",
        "relationExpression",
        "locus",
    ];

    /// Canonical structural form of one objects map for equivalence comparison:
    /// provenance fields dropped, then every id renamed by BFS encounter order
    /// from `root` (unreached stragglers in sorted-id order), so opaque
    /// generated ids align by reference topology alone.
    #[requires(!root.is_empty())]
    #[ensures(true)]
    pub(crate) fn normalize_objects(
        objects: &Map<String, Value>,
        root: &str,
    ) -> BTreeMap<String, String> {
        let mut canonical: HashMap<String, String> = HashMap::new();
        let mut next = 0usize;
        let mut queue: VecDeque<String> = VecDeque::from([root.to_owned()]);
        let rename = |id: &str,
                      canonical: &mut HashMap<String, String>,
                      next: &mut usize,
                      queue: &mut VecDeque<String>| {
            if !canonical.contains_key(id) {
                *next += 1;
                canonical.insert(id.to_owned(), format!("n{next}"));
                queue.push_back(id.to_owned());
            }
        };
        rename(root, &mut canonical, &mut next, &mut queue);
        while let Some(id) = queue.pop_front() {
            let Some(object) = objects.get(&id).and_then(Value::as_object) else {
                continue;
            };
            for value in object.values() {
                collect_ids(value, objects, &mut |id| {
                    rename(id, &mut canonical, &mut next, &mut queue)
                });
            }
        }
        // Unreached stragglers (the graph should not have any, but never hide
        // one): continue the walk in sorted-id order.
        let stragglers: BTreeSet<&String> = objects
            .keys()
            .filter(|key| !canonical.contains_key(*key))
            .collect();
        for straggler in stragglers {
            rename(straggler, &mut canonical, &mut next, &mut queue);
            while let Some(id) = queue.pop_front() {
                let Some(object) = objects.get(&id).and_then(Value::as_object) else {
                    continue;
                };
                for value in object.values() {
                    collect_ids(value, objects, &mut |id| {
                        rename(id, &mut canonical, &mut next, &mut queue)
                    });
                }
            }
        }
        objects
            .iter()
            .map(|(key, value)| {
                (
                    canonical[key].clone(),
                    canonical_json(value, objects, &canonical),
                )
            })
            .collect()
    }

    /// Invoke `visit` on every string value that names an object in the map.
    #[requires(true)]
    #[ensures(true)]
    fn collect_ids(value: &Value, objects: &Map<String, Value>, visit: &mut impl FnMut(&str)) {
        match value {
            Value::String(text) => {
                if objects.contains_key(text.as_str()) {
                    visit(text);
                }
            }
            Value::Array(items) => {
                for item in items {
                    collect_ids(item, objects, visit);
                }
            }
            Value::Object(map) => {
                for (field, item) in map {
                    if PROVENANCE_FIELDS.contains(&field.as_str()) {
                        continue;
                    }
                    collect_ids(item, objects, visit);
                }
            }
            _ => {}
        }
    }

    /// Serialize one value with provenance fields dropped and ids renamed.
    #[requires(true)]
    #[ensures(true)]
    fn canonical_json(
        value: &Value,
        objects: &Map<String, Value>,
        canonical: &HashMap<String, String>,
    ) -> String {
        match value {
            Value::String(text) => format!(
                "{:?}",
                canonical
                    .get(text.as_str())
                    .map(String::as_str)
                    .unwrap_or(text)
            ),
            Value::Array(items) => format!(
                "[{}]",
                items
                    .iter()
                    .map(|item| canonical_json(item, objects, canonical))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Value::Object(map) => {
                let mut fields: Vec<String> = map
                    .iter()
                    .filter(|(field, _)| !PROVENANCE_FIELDS.contains(&field.as_str()))
                    .map(|(field, item)| {
                        format!("{field:?}:{}", canonical_json(item, objects, canonical))
                    })
                    .collect();
                fields.sort();
                format!("{{{}}}", fields.join(","))
            }
            other => format!("{other:?}"),
        }
    }

    /// The acceptance driver mandated by the round-14 decision: parse the
    /// *rendered* SFN-XML relation expressions back into compositions,
    /// re-expand them, and require the normalized forms of the pre-projection
    /// graph and the regenerated graph to be identical. `objects` is the
    /// original (unprojected) graph; `xml` is the product renderer's actual
    /// output for it.
    #[requires(!root.is_empty())]
    #[ensures(true)]
    pub(crate) fn assert_rendered_reexpansion_equivalent(
        objects: &Map<String, Value>,
        root: &str,
        xml: &str,
        dictionary: &Dictionary<'_>,
    ) {
        let (transformed, projection) = project_tanru_compositions(objects);
        let id_to_key = rendered_id_to_key_map(objects);
        let document = roxmltree::Document::parse(xml).expect("rendered XML parses");
        let mut expanded = transformed.clone();
        let mut next = 0usize;
        for instance in &projection.instances {
            let predication_element =
                find_projected_predication(&document, instance, objects, &id_to_key);
            let host_relation = objects[&instance.head_predication]
                .get("relation")
                .and_then(Value::as_str)
                .expect("surviving head predication has its lexical relation");
            let view = composition_from_xml(
                predication_element
                    .children()
                    .find(|child| {
                        child.is_element() && child.tag_name().name() == "KIND-COMPOSITION"
                    })
                    .expect("projected predication renders KIND-COMPOSITION"),
                true,
                &id_to_key,
                dictionary,
                objects,
                host_relation,
            );
            expanded = reexpand_instance(&expanded, instance, &view, &mut next);
        }
        let original_normalized = normalize_objects(objects, root);
        let expanded_normalized = normalize_objects(&expanded, root);
        if original_normalized != expanded_normalized {
            for (key, value) in &original_normalized {
                if expanded_normalized.get(key) != Some(value) {
                    eprintln!("ORIGINAL {key}: {value}");
                    eprintln!("EXPANDED {key}: {:?}", expanded_normalized.get(key));
                }
            }
            for (key, value) in &expanded_normalized {
                if !original_normalized.contains_key(key) {
                    eprintln!("EXPANDED-ONLY {key}: {value}");
                }
            }
            panic!(
                "re-expanding the rendered relation expression must reproduce the recognized subgraph modulo opaque ids and provenance"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{data, ensures, invariant, new, requires};

    use serde_json::{Map, Value};

    use super::reexpansion::{
        assert_rendered_reexpansion_equivalent, normalize_objects, reexpand_instances,
    };
    use super::{RelationOperandData, project_tanru_compositions};
    use crate::SemanticGraph;
    use crate::notation::render_xml_value_for_tooling;

    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn graph_for_text(text: &str) -> SemanticGraph {
        let words = jbotci_morphology::segment_words_with_modifiers(text).expect("morphology");
        let parsed = jbotci_syntax::parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            text,
            &jbotci_syntax::ParseOptions::default(),
        )
        .expect("syntax");
        crate::build_generated_semantic_graph_with_dictionary_and_options(
            &parsed,
            crate::SemanticBuildOptions {
                source_text: Some(text),
                story_time: false,
            },
            jbotci_dictionary_data::english(),
        )
        .expect("semantics")
    }

    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn graph_value(text: &str) -> Value {
        serde_json::to_value(graph_for_text(text)).expect("graph serializes")
    }

    #[requires(true)]
    #[ensures(true)]
    fn objects_of(graph: &Value) -> Map<String, Value> {
        graph["objects"].as_object().expect("objects map").clone()
    }

    /// Render the graph exactly as the product does, then run the
    /// rendered-surface re-expansion equivalence acceptance check.
    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn assert_text_reexpands(text: &str) {
        let graph = graph_for_text(text);
        let root = graph.root.to_string();
        let value = serde_json::to_value(&graph).expect("graph serializes");
        let objects = objects_of(&value);
        let xml = crate::render_xml(&graph, "<acceptance>").into_data().output;
        assert_rendered_reexpansion_equivalent(
            &objects,
            &root,
            &xml,
            jbotci_dictionary_data::english(),
        );
    }

    /// The round-14 witness set plus the tricky shapes: asserted main-selbri
    /// tanru, co with following terms (fixed modifier args), se conversion
    /// (non-first participant place), be-linked and NU-abstracted modifiers
    /// (reference fallback), and a connected tertau (loud fallback).
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reexpansion_equivalence_holds_for_tanru_witnesses() {
        for text in [
            "lo blanu zdani cu barda",
            "lo mutce blanu zdani cu barda",
            "lo mutce blanu bo zdani cu barda",
            "lo sutra je xekri mlatu cu barda",
            "lo zdani co blanu cu barda",
            "ti blanu zdani",
            "ti zdani co blanu mi",
            "lo se xekri mlatu cu barda",
            "lo xamgu be lo gerku zdani cu barda",
            "lo xamgu be lo gerku be'o zdani cu barda",
            "lo nu bajra kei zdani cu barda",
            "mi nelci lo melbi cmalu nixli je ckule",
        ] {
            assert_text_reexpands(text);
        }
    }

    /// Every frozen-corpus graph must satisfy the same rendered-surface
    /// equivalence (the corpus migration activates the tanru instances there).
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn reexpansion_equivalence_holds_for_the_frozen_corpus() {
        let corpus = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/xml_corpus");
        let mut documents: Vec<_> = std::fs::read_dir(&corpus)
            .expect("corpus directory")
            .filter_map(|entry| {
                let path = entry.expect("corpus entry").path();
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .filter(|name| name.ends_with(".frozen.json"))
                    .map(|_| path)
            })
            .collect();
        documents.sort();
        assert!(
            documents.len() >= 48,
            "the frozen corpus must stay complete"
        );
        for path in documents {
            let mut graph: Value = serde_json::from_str(
                &std::fs::read_to_string(&path).expect("corpus document reads"),
            )
            .expect("corpus document parses");
            let binder_universes: Value = serde_json::from_slice(include_bytes!(
                "../../tests/xml_corpus/BINDER_UNIVERSES.json"
            ))
            .expect("binder universes parse");
            let document_name = path
                .file_name()
                .expect("document name")
                .to_string_lossy()
                .replace(".frozen.json", "");
            graph["scopeDependenceBinderUniverses"] =
                binder_universes[document_name.as_str()].clone();
            let root = graph["root"].as_str().expect("graph root").to_owned();
            let objects = objects_of(&graph);
            let xml = render_xml_value_for_tooling(graph, &document_name)
                .into_data()
                .output;
            assert_rendered_reexpansion_equivalent(
                &objects,
                &root,
                &xml,
                jbotci_dictionary_data::english(),
            );
        }
    }

    /// Build a witness graph and mutate its objects JSON before projection.
    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn mutated_witness(
        text: &str,
        mutate: impl FnOnce(&mut Map<String, Value>),
    ) -> (Value, Map<String, Value>) {
        let graph = graph_value(text);
        let mut objects = objects_of(&graph);
        mutate(&mut objects);
        (graph, objects)
    }

    #[requires(true)]
    #[ensures(true)]
    fn find_predication_with_relation(objects: &Map<String, Value>, relation: &str) -> String {
        objects
            .iter()
            .find(|(_, object)| {
                object.get("type").and_then(Value::as_str) == Some("predication")
                    && object.get("relation").and_then(Value::as_str) == Some(relation)
            })
            .map(|(key, _)| key.clone())
            .unwrap_or_else(|| panic!("no predication with relation {relation}"))
    }

    #[requires(true)]
    #[ensures(true)]
    fn find_tanru_link_predication(objects: &Map<String, Value>) -> String {
        objects
            .iter()
            .find(|(_, object)| object.get("tanruLink").is_some())
            .map(|(key, _)| key.clone())
            .expect("a tanru-link predication")
    }

    /// Negative fixture: unequal head/link modes must NOT compact (the
    /// recognizer derives the link mode from the head, so a graph with unequal
    /// modes could not be reconstructed losslessly).
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unequal_head_and_link_modes_stay_loud() {
        let (_graph, objects) = mutated_witness("lo blanu zdani cu barda", |objects| {
            let link = find_tanru_link_predication(objects);
            objects
                .get_mut(&link)
                .and_then(Value::as_object_mut)
                .expect("link object")
                .insert("mode".to_owned(), Value::from("asserted"));
        });
        let (transformed, projection) = project_tanru_compositions(&objects);
        assert!(
            projection.instances.is_empty(),
            "unequal head/link modes must reject the compact form"
        );
        assert_eq!(transformed.len(), objects.len(), "nothing was consumed");
        // Sanity: the original graph itself re-expands trivially (no instances).
        assert_eq!(
            normalize_objects(&objects, "utterance:5"),
            normalize_objects(
                &reexpand_instances(&transformed, &projection.instances),
                "utterance:5"
            )
        );
    }

    /// Negative fixture: a corrupted rendered host predicate (a swap of the
    /// surviving head's own PREDICATE= in the emitted XML) must FAIL the
    /// rendered-surface acceptance check — the host leaf's rendered value is
    /// validated against the surviving head predication, not discarded.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn corrupted_rendered_host_predicate_fails() {
        let graph = graph_for_text("lo blanu zdani cu barda");
        let root = graph.root.to_string();
        let value = serde_json::to_value(&graph).expect("graph serializes");
        let objects = objects_of(&value);
        let xml = crate::render_xml(&graph, "<acceptance>").into_data().output;
        assert!(
            xml.contains("<KIND PREDICATE=\"zdani\"/>"),
            "witness must render the host predicate"
        );
        let corrupted = xml.replacen(
            "<KIND PREDICATE=\"zdani\"/>",
            "<KIND PREDICATE=\"mlatu\"/>",
            1,
        );
        let result = std::panic::catch_unwind(|| {
            assert_rendered_reexpansion_equivalent(
                &objects,
                &root,
                &corrupted,
                jbotci_dictionary_data::english(),
            );
        });
        assert!(
            result.is_err(),
            "a swapped host predicate in the rendered XML must fail the acceptance check"
        );
    }

    /// Negative fixture: a modifier eventuality shared with an outside
    /// predication is not the derivable fresh-local event — the modifier must
    /// fall back to the composite reference form, never the compact leaf.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn shared_modifier_event_forces_composite_fallback() {
        let (_graph, objects) = mutated_witness("lo blanu zdani cu barda", |objects| {
            let blanu = find_predication_with_relation(objects, "blanu");
            let barda = find_predication_with_relation(objects, "barda");
            let event = objects[&blanu]["eventuality"].clone();
            objects
                .get_mut(&barda)
                .and_then(Value::as_object_mut)
                .expect("barda object")
                .insert("eventuality".to_owned(), event);
        });
        assert_modifier_falls_back_to_reference(objects);
    }

    /// Negative fixture: a decorated (tense-marked) modifier eventuality is
    /// not the derivable fresh-local event.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn decorated_modifier_event_forces_composite_fallback() {
        let (_graph, objects) = mutated_witness("lo blanu zdani cu barda", |objects| {
            let blanu = find_predication_with_relation(objects, "blanu");
            let event = objects[&blanu]["eventuality"]
                .as_str()
                .expect("modifier event")
                .to_owned();
            objects
                .get_mut(&event)
                .and_then(Value::as_object_mut)
                .expect("event object")
                .insert(
                    "time".to_owned(),
                    serde_json::json!({"kind": "offset", "direction": "past"}),
                );
        });
        assert_modifier_falls_back_to_reference(objects);
    }

    /// Negative fixture: a place question on the modifier predication makes
    /// its place map non-flat.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn non_flat_place_map_forces_composite_fallback() {
        let (_graph, objects) = mutated_witness("lo blanu zdani cu barda", |objects| {
            let blanu = find_predication_with_relation(objects, "blanu");
            objects
                .get_mut(&blanu)
                .and_then(Value::as_object_mut)
                .expect("blanu object")
                .insert("placeQuestions".to_owned(), serde_json::json!([]));
        });
        assert_modifier_falls_back_to_reference(objects);
    }

    /// A structural predication without a lexical relation name is outside
    /// the compact-leaf contract. The caller must decline the lexical
    /// projection before invoking that recognizer.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn unnamed_modifier_declines_lexical_projection() {
        let (_graph, objects) = mutated_witness("lo blanu zdani cu barda", |objects| {
            let blanu = find_predication_with_relation(objects, "blanu");
            objects
                .get_mut(&blanu)
                .and_then(Value::as_object_mut)
                .expect("blanu object")
                .remove("relation");
        });
        assert_modifier_falls_back_to_reference(objects);
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_modifier_falls_back_to_reference(objects: Map<String, Value>) {
        let (_transformed, projection) = project_tanru_compositions(&objects);
        assert_eq!(
            projection.instances.len(),
            1,
            "the tanru itself is still recognized"
        );
        let instance = &projection.instances[0];
        let data!(RelationOperand::Composition { modifier, .. }) = instance.composition.as_data()
        else {
            panic!("the projected view root is a composition");
        };
        assert!(
            matches!(modifier.as_data(), data!(RelationOperand::Reference { .. })),
            "a shared/decorated/non-flat modifier must stay in the composite reference form: {:?}",
            modifier.as_data()
        );
    }
}
