//! Mechanical derivation of constant-denotation binder dependence.

use super::*;

#[allow(unused_imports)]
use bityzba::{ensures, expensive_ensures, requires};

/// Recomputes every constant referent's dependence from the rooted semantic graph.
#[requires(objects.contains_key(&root))]
#[ensures(objects.values().all(|object| object.referent_category() != Some(ReferentCategory::Constant) || object.scope_dependence().is_some()))]
#[expensive_ensures(semantic_object_scope_dependences_are_derived(root, objects))]
pub(crate) fn apply_semantic_scope_dependence(
    root: SemanticObjectId,
    objects: &mut BTreeMap<SemanticObjectId, SemanticObject>,
) {
    let derived = derive_semantic_scope_dependences(root, objects);
    for (id, dependence) in derived {
        objects
            .get_mut(&id)
            .expect("scope-dependence traversal only returns graph object IDs")
            .set_scope_dependence(dependence);
    }
}

/// Validates that every constant's stored dependence equals a fresh derivation.
#[requires(objects.contains_key(&root))]
#[ensures(!ret || objects.values().all(|object| object.referent_category() != Some(ReferentCategory::Constant) || object.scope_dependence().is_some()))]
pub fn semantic_object_scope_dependences_are_derived(
    root: SemanticObjectId,
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    let derived = derive_semantic_scope_dependences(root, objects);
    objects.iter().all(|(id, object)| {
        object.referent_category() != Some(ReferentCategory::Constant)
            || derived
                .get(id)
                .is_some_and(|dependence| object.scope_dependence() == Some(dependence))
    })
}

/// Derives the canonical dependence value at each constant's introduction path.
///
/// Generated graphs are connected after pruning. For hand-built graphs that retain
/// disconnected objects, each remaining component is visited in object-ID order at
/// empty scope so validation remains total and deterministic.
#[requires(objects.contains_key(&root))]
#[ensures(ret.keys().all(|id| objects.get(id).is_some_and(|object| object.referent_category() == Some(ReferentCategory::Constant))))]
fn derive_semantic_scope_dependences(
    root: SemanticObjectId,
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> BTreeMap<SemanticObjectId, ScopeDependence> {
    let mut expanded = BTreeSet::new();
    let mut derived = BTreeMap::new();
    visit_scope_object(root, objects, &BTreeSet::new(), &mut expanded, &mut derived);
    for id in objects.keys().copied() {
        if !expanded.contains(&id) {
            visit_scope_object(id, objects, &BTreeSet::new(), &mut expanded, &mut derived);
        }
    }
    derived
}

#[requires(objects.contains_key(&id))]
#[requires(binders.iter().all(|binder| quantifier_variable_kind_is_allowed(binder.object_kind())))]
#[ensures(derived.len() >= old(derived.len()))]
fn visit_scope_object(
    id: SemanticObjectId,
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    binders: &BTreeSet<SemanticObjectId>,
    expanded: &mut BTreeSet<SemanticObjectId>,
    derived: &mut BTreeMap<SemanticObjectId, ScopeDependence>,
) {
    let object = objects
        .get(&id)
        .expect("scope traversal requires graph references to be defined");
    if object.referent_category() == Some(ReferentCategory::Constant) {
        derived.entry(id).or_insert_with(|| {
            if binders.is_empty() {
                ScopeDependence::fixed()
            } else {
                ScopeDependence::underspecified(binders.clone())
            }
        });
    }
    if !expanded.insert(id) {
        return;
    }

    match object.as_data() {
        data!(SemanticObject::Utterance(node)) => {
            // Utterance boundaries, including quoted utterances, are scope-opaque.
            let unbound = BTreeSet::new();
            visit_ids(
                [node.speaker, node.audience, node.eventuality]
                    .into_iter()
                    .chain(node.content)
                    .chain([node.deictic_ground.time, node.deictic_ground.place])
                    .chain(node.asides.iter().copied()),
                objects,
                &unbound,
                expanded,
                derived,
            );
        }
        data!(SemanticObject::Formula(formula)) => {
            visit_formula_scope(formula, objects, binders, expanded, derived);
        }
        data!(SemanticObject::Sequence(node)) => {
            visit_ids(
                node.content
                    .into_iter()
                    .chain(node.connection_claims.iter().copied())
                    .chain(node.items.iter().copied()),
                objects,
                binders,
                expanded,
                derived,
            );
            let mut references = Vec::new();
            object.references_into(&mut references);
            let suffix_start = node.items.len()
                + usize::from(node.content.is_some())
                + node.connection_claims.len();
            visit_ids(
                references[suffix_start..].iter().copied(),
                objects,
                binders,
                expanded,
                derived,
            );
        }
        data!(SemanticObject::Eventuality(node)) => {
            visit_abstraction_scope(
                object,
                node.body,
                &node.parameters,
                node.tense_modal.is_some() as usize + node.content.is_some() as usize,
                objects,
                binders,
                expanded,
                derived,
            );
        }
        data!(SemanticObject::Referent(node)) => {
            visit_abstraction_scope(
                object,
                node.body,
                &node.parameters,
                0,
                objects,
                binders,
                expanded,
                derived,
            );
        }
        data!(SemanticObject::Question(node)) => {
            visit_ids(
                [node.asker, node.respondent]
                    .into_iter()
                    .chain(node.slots.iter().map(|slot| slot.parameter))
                    .chain(node.focus)
                    .chain(node.presupposed_answer),
                objects,
                binders,
                expanded,
                derived,
            );
            let scoped = binders_with(binders, node.slots.iter().map(|slot| slot.parameter));
            visit_scope_object(node.body, objects, &scoped, expanded, derived);
        }
        _ => {
            let mut references = Vec::new();
            object.references_into(&mut references);
            visit_ids(references, objects, binders, expanded, derived);
        }
    }
}

#[requires(references.clone().into_iter().all(|id| objects.contains_key(&id)))]
#[requires(binders.iter().all(|binder| quantifier_variable_kind_is_allowed(binder.object_kind())))]
#[ensures(derived.len() >= old(derived.len()))]
fn visit_ids(
    references: impl IntoIterator<Item = SemanticObjectId> + Clone,
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    binders: &BTreeSet<SemanticObjectId>,
    expanded: &mut BTreeSet<SemanticObjectId>,
    derived: &mut BTreeMap<SemanticObjectId, ScopeDependence>,
) {
    for reference in references {
        visit_scope_object(reference, objects, binders, expanded, derived);
    }
}

#[requires(parameters.iter().all(|parameter| parameter.object_kind() == SemanticObjectKind::Parameter))]
#[requires(body.is_none_or(|body| body.object_kind() == SemanticObjectKind::Formula))]
#[ensures(derived.len() >= old(derived.len()))]
#[allow(clippy::too_many_arguments)]
fn visit_abstraction_scope(
    object: &SemanticObject,
    body: Option<SemanticObjectId>,
    parameters: &[SemanticObjectId],
    prefix_before_body: usize,
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    binders: &BTreeSet<SemanticObjectId>,
    expanded: &mut BTreeSet<SemanticObjectId>,
    derived: &mut BTreeMap<SemanticObjectId, ScopeDependence>,
) {
    let mut references = Vec::new();
    object.references_into(&mut references);
    let body_width = usize::from(body.is_some());
    let suffix_start = prefix_before_body + body_width + parameters.len();

    visit_ids(
        references[..prefix_before_body].iter().copied(),
        objects,
        binders,
        expanded,
        derived,
    );
    visit_ids(
        parameters.iter().copied(),
        objects,
        binders,
        expanded,
        derived,
    );
    visit_ids(
        references[suffix_start..].iter().copied(),
        objects,
        binders,
        expanded,
        derived,
    );
    if let Some(body) = body {
        let scoped = binders_with(binders, parameters.iter().copied());
        visit_scope_object(body, objects, &scoped, expanded, derived);
    }
}

#[requires(binders.iter().all(|binder| quantifier_variable_kind_is_allowed(binder.object_kind())))]
#[requires(additional.clone().into_iter().all(|binder| quantifier_variable_kind_is_allowed(binder.object_kind())))]
#[ensures(ret.is_superset(binders))]
fn binders_with(
    binders: &BTreeSet<SemanticObjectId>,
    additional: impl IntoIterator<Item = SemanticObjectId> + Clone,
) -> BTreeSet<SemanticObjectId> {
    let mut scoped = binders.clone();
    scoped.extend(additional);
    scoped
}

#[requires(binders.iter().all(|binder| quantifier_variable_kind_is_allowed(binder.object_kind())))]
#[ensures(derived.len() >= old(derived.len()))]
fn visit_formula_scope(
    formula: &FormulaNode,
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    binders: &BTreeSet<SemanticObjectId>,
    expanded: &mut BTreeSet<SemanticObjectId>,
    derived: &mut BTreeMap<SemanticObjectId, ScopeDependence>,
) {
    match formula.as_data() {
        data!(FormulaNode::Atom(node)) => {
            visit_scope_object(node.predication, objects, binders, expanded, derived);
        }
        data!(FormulaNode::Connective(node)) => {
            visit_ids(
                node.eventuality
                    .into_iter()
                    .chain(node.children.iter().copied())
                    .chain(
                        node.connector
                            .as_ref()
                            .and_then(|connector| connector.parameter),
                    ),
                objects,
                binders,
                expanded,
                derived,
            );
        }
        data!(FormulaNode::Quantified(node)) => {
            visit_ids(
                [node.variable]
                    .into_iter()
                    .chain(node.source_variable)
                    .chain(node.selection_source.as_ref().map(|source| source.variable))
                    .chain(node.quantity),
                objects,
                binders,
                expanded,
                derived,
            );
            let scoped = binders_with(binders, [node.variable]);
            visit_ids(
                node.restriction.into_iter().chain([node.body]),
                objects,
                &scoped,
                expanded,
                derived,
            );
        }
        data!(FormulaNode::QuantifierBundle(node)) => {
            let scoped = binders_with(
                binders,
                node.bindings.iter().map(|binding| binding.variable),
            );
            for binding in &node.bindings {
                visit_ids(
                    [binding.variable]
                        .into_iter()
                        .chain(binding.source_variable)
                        .chain(
                            binding
                                .selection_source
                                .as_ref()
                                .map(|source| source.variable),
                        )
                        .chain(binding.quantity),
                    objects,
                    binders,
                    expanded,
                    derived,
                );
                visit_ids(binding.restriction, objects, &scoped, expanded, derived);
            }
            visit_scope_object(node.body, objects, &scoped, expanded, derived);
        }
        data!(FormulaNode::RespectivelyDistribution(node)) => {
            let scoped = binders_with(binders, node.streams.iter().map(|stream| stream.slot));
            for stream in &node.streams {
                visit_ids(
                    [stream.slot]
                        .into_iter()
                        .chain(stream.items.iter().copied())
                        .chain(stream.quantity),
                    objects,
                    binders,
                    expanded,
                    derived,
                );
                visit_ids(stream.restriction, objects, &scoped, expanded, derived);
            }
            visit_scope_object(node.body, objects, &scoped, expanded, derived);
        }
    }
}
