//! Canonical binding derivation for generated predication eventualities.

use super::*;

#[allow(unused_imports)]
use bityzba::{ensures, expensive_ensures, invariant, requires};

#[invariant(formulas.iter().all(|formula| formula.object_kind() == SemanticObjectKind::Formula))]
#[invariant(sequences.iter().all(|sequence| sequence.object_kind() == SemanticObjectKind::Sequence))]
#[derive(Debug, Default)]
struct GeneratedEventUses {
    formulas: BTreeSet<SemanticObjectId>,
    sequences: BTreeSet<SemanticObjectId>,
}

impl GeneratedEventUses {
    #[requires(formula.object_kind() == SemanticObjectKind::Formula)]
    #[ensures(self.formulas.contains(&formula))]
    fn insert_formula(&mut self, formula: SemanticObjectId) {
        let mut data = std::mem::take(self).into_data();
        data.formulas.insert(formula);
        *self = GeneratedEventUses::from_data(data);
    }

    #[requires(sequence.object_kind() == SemanticObjectKind::Sequence)]
    #[ensures(self.sequences.contains(&sequence))]
    fn insert_sequence(&mut self, sequence: SemanticObjectId) {
        let mut data = std::mem::take(self).into_data();
        data.sequences.insert(sequence);
        *self = GeneratedEventUses::from_data(data);
    }
}

/// Replaces all owner bindings with the unique canonical binding derived from semantic uses.
#[requires(objects.contains_key(&root))]
#[ensures(ret.is_err() || ret.is_ok())]
#[expensive_ensures(ret.is_err() || semantic_event_bindings_are_derived(root, objects))]
pub(crate) fn apply_semantic_event_bindings(
    root: SemanticObjectId,
    objects: &mut BTreeMap<SemanticObjectId, SemanticObject>,
) -> Result<(), String> {
    for object in objects.values_mut() {
        if matches!(
            object.object_kind(),
            SemanticObjectKind::Formula | SemanticObjectKind::Sequence
        ) {
            object.set_bound_eventualities(Vec::new());
        }
    }

    let bindings = derive_semantic_event_bindings(root, objects)?;
    let mut by_owner = BTreeMap::<SemanticObjectId, Vec<GeneratedEventualityId>>::new();
    for (eventuality, scope) in bindings {
        by_owner.entry(scope.owner()).or_default().push(eventuality);
    }
    for (owner, bound_eventualities) in by_owner {
        objects
            .get_mut(&owner)
            .expect("binding derivation returns defined owner IDs")
            .set_bound_eventualities(bound_eventualities);
    }
    Ok(())
}

/// Checks generated-event identity, uniqueness, domination, and completeness together.
#[requires(objects.contains_key(&root))]
#[ensures(true)]
pub fn semantic_event_bindings_are_derived(
    root: SemanticObjectId,
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    let Ok(bindings) = derive_semantic_event_bindings(root, objects) else {
        return false;
    };
    let mut expected = BTreeMap::<SemanticObjectId, Vec<GeneratedEventualityId>>::new();
    for (eventuality, scope) in bindings {
        expected.entry(scope.owner()).or_default().push(eventuality);
    }

    objects.iter().all(|(id, object)| {
        if matches!(
            object.object_kind(),
            SemanticObjectKind::Formula | SemanticObjectKind::Sequence
        ) {
            object.bound_eventualities()
                == expected
                    .get(id)
                    .map_or(&[] as &[GeneratedEventualityId], Vec::as_slice)
                && object.bound_eventualities().iter().all(|eventuality| {
                    objects
                        .get(&eventuality.object_id())
                        .is_some_and(SemanticObject::is_generated_eventuality)
                })
        } else {
            object.bound_eventualities().is_empty()
        }
    })
}

#[requires(objects.contains_key(&root))]
#[ensures(ret.is_err() || ret.as_ref().is_ok_and(|bindings| bindings.keys().all(|eventuality| objects.get(&eventuality.object_id()).is_some_and(SemanticObject::is_generated_eventuality))))]
fn derive_semantic_event_bindings(
    root: SemanticObjectId,
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> Result<BTreeMap<GeneratedEventualityId, EventBindingScope>, String> {
    debug_assert!(objects.contains_key(&root));
    let mut uses = objects
        .iter()
        .filter_map(|(&id, object)| {
            object.is_generated_eventuality().then(|| {
                (
                    GeneratedEventualityId::new(id),
                    GeneratedEventUses::default(),
                )
            })
        })
        .collect::<BTreeMap<_, _>>();

    collect_formula_event_uses(objects, &mut uses);
    collect_event_content_uses(objects, &mut uses);

    let formula_ancestors = derive_formula_ancestors(objects);
    let sequence_reachability = derive_sequence_reachability(objects);
    let mut bindings = BTreeMap::new();
    for (eventuality, event_uses) in uses {
        if event_uses.formulas.is_empty() && event_uses.sequences.is_empty() {
            return Err(format!(
                "generated eventuality {} has no bindable semantic use",
                eventuality.object_id()
            ));
        }

        let scope = if event_uses.sequences.is_empty() {
            lowest_common_formula(&event_uses.formulas, &formula_ancestors)
                .map(EventBindingScope::formula)
        } else {
            None
        }
        .or_else(|| {
            lowest_common_sequence(
                &event_uses.formulas,
                &event_uses.sequences,
                &sequence_reachability,
            )
            .map(EventBindingScope::sequence)
        })
        .ok_or_else(|| {
            format!(
                "generated eventuality {} has no unique formula or sequence scope owner",
                eventuality.object_id()
            )
        })?;
        bindings.insert(eventuality, scope);
    }
    Ok(bindings)
}

#[requires(true)]
#[ensures(uses.values().all(|event_uses| event_uses.formulas.iter().all(|formula| formula.object_kind() == SemanticObjectKind::Formula)))]
fn collect_formula_event_uses(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    uses: &mut BTreeMap<GeneratedEventualityId, GeneratedEventUses>,
) {
    for (&formula_id, object) in objects {
        let Some(formula) = object.as_formula() else {
            continue;
        };
        match formula.as_data() {
            data!(FormulaNode::Atom(atom)) => {
                collect_predication_event_uses(
                    atom.predication,
                    formula_id,
                    objects,
                    uses,
                    &mut BTreeSet::new(),
                );
            }
            data!(FormulaNode::Connective(node)) => {
                record_generated_formula_use(node.eventuality, formula_id, objects, uses);
            }
            data!(FormulaNode::Quantified(_))
            | data!(FormulaNode::QuantifierBundle(_))
            | data!(FormulaNode::RespectivelyDistribution(_)) => {}
        }
    }
}

#[requires(predication.object_kind() == SemanticObjectKind::Predication)]
#[requires(formula.object_kind() == SemanticObjectKind::Formula)]
#[ensures(visited.contains(&predication))]
fn collect_predication_event_uses(
    predication: SemanticObjectId,
    formula: SemanticObjectId,
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    uses: &mut BTreeMap<GeneratedEventualityId, GeneratedEventUses>,
    visited: &mut BTreeSet<SemanticObjectId>,
) {
    if !visited.insert(predication) {
        return;
    }
    let Some(node) = objects
        .get(&predication)
        .and_then(SemanticObject::as_predication)
    else {
        return;
    };
    record_generated_formula_use(node.eventuality, formula, objects, uses);
    for argument in node.arguments.values() {
        record_generated_formula_use(argument.value, formula, objects, uses);
    }
    for question in &node.place_questions {
        record_generated_formula_use(question.argument.value, formula, objects, uses);
    }
    for modal in &node.modal_arguments {
        for argument in modal.arguments.values() {
            record_generated_formula_use(argument.value, formula, objects, uses);
        }
        record_generated_formula_use(modal.component, formula, objects, uses);
    }
    for exchange in &node.reciprocity {
        record_generated_formula_use(exchange.left.value, formula, objects, uses);
        record_generated_formula_use(exchange.right.value, formula, objects, uses);
    }
    if let Some(tanru_link) = &node.tanru_link {
        record_generated_formula_use(Some(tanru_link.modifier), formula, objects, uses);
        collect_predication_event_uses(tanru_link.head, formula, objects, uses, visited);
    }
}

#[requires(true)]
#[ensures(uses.values().all(|event_uses| event_uses.sequences.iter().all(|sequence| sequence.object_kind() == SemanticObjectKind::Sequence)))]
fn collect_event_content_uses(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    uses: &mut BTreeMap<GeneratedEventualityId, GeneratedEventUses>,
) {
    for (&id, object) in objects {
        let Some(eventuality) = object.as_eventuality() else {
            continue;
        };
        let generated = GeneratedEventualityId::new(id);
        let Some(event_uses) = uses.get_mut(&generated) else {
            continue;
        };
        match eventuality.content.map(SemanticObjectId::object_kind) {
            Some(SemanticObjectKind::Formula) => {
                event_uses.insert_formula(
                    eventuality
                        .content
                        .expect("matched an existing formula content ID"),
                );
            }
            Some(SemanticObjectKind::Sequence) => {
                event_uses.insert_sequence(
                    eventuality
                        .content
                        .expect("matched an existing sequence content ID"),
                );
            }
            Some(_) | None => {}
        }
    }
}

#[requires(formula.object_kind() == SemanticObjectKind::Formula)]
#[ensures(uses.len() >= old(uses.len()))]
fn record_generated_formula_use(
    eventuality: Option<SemanticObjectId>,
    formula: SemanticObjectId,
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    uses: &mut BTreeMap<GeneratedEventualityId, GeneratedEventUses>,
) {
    let Some(eventuality) = eventuality else {
        return;
    };
    if !objects
        .get(&eventuality)
        .is_some_and(SemanticObject::is_generated_eventuality)
    {
        return;
    }
    if let Some(event_uses) = uses.get_mut(&GeneratedEventualityId::new(eventuality)) {
        event_uses.insert_formula(formula);
    }
}

#[requires(true)]
#[ensures(ret.keys().all(|formula| formula.object_kind() == SemanticObjectKind::Formula))]
fn derive_formula_ancestors(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> {
    let mut parents = BTreeMap::<SemanticObjectId, BTreeSet<SemanticObjectId>>::new();
    for (&id, object) in objects {
        let Some(formula) = object.as_formula() else {
            continue;
        };
        parents.entry(id).or_default();
        let mut children = Vec::new();
        direct_formula_children(formula, &mut children);
        for child in children {
            parents.entry(child).or_default().insert(id);
        }
    }

    parents
        .keys()
        .copied()
        .map(|formula| {
            let mut ancestors = BTreeSet::from([formula]);
            let mut pending = vec![formula];
            while let Some(current) = pending.pop() {
                for parent in parents.get(&current).into_iter().flatten().copied() {
                    if ancestors.insert(parent) {
                        pending.push(parent);
                    }
                }
            }
            (formula, ancestors)
        })
        .collect()
}

#[requires(true)]
#[ensures(out.len() >= old(out.len()))]
fn direct_formula_children(formula: &FormulaNode, out: &mut Vec<SemanticObjectId>) {
    match formula.as_data() {
        data!(FormulaNode::Atom(_)) => {}
        data!(FormulaNode::Connective(node)) => out.extend(node.children.iter().copied()),
        data!(FormulaNode::Quantified(node)) => {
            out.extend(node.restriction);
            out.push(node.body);
        }
        data!(FormulaNode::QuantifierBundle(node)) => {
            out.extend(
                node.bindings
                    .iter()
                    .filter_map(|binding| binding.restriction),
            );
            out.push(node.body);
        }
        data!(FormulaNode::RespectivelyDistribution(node)) => {
            out.push(node.body);
            for stream in &node.streams {
                out.extend(
                    stream
                        .items
                        .iter()
                        .copied()
                        .filter(|item| item.object_kind() == SemanticObjectKind::Formula),
                );
                out.extend(stream.restriction);
            }
        }
    }
}

#[requires(formulas.iter().all(|formula| formula.object_kind() == SemanticObjectKind::Formula))]
#[ensures(ret.is_none_or(|formula| formula.object_kind() == SemanticObjectKind::Formula))]
fn lowest_common_formula(
    formulas: &BTreeSet<SemanticObjectId>,
    ancestors: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
) -> Option<SemanticObjectId> {
    let mut formulas = formulas.iter();
    let first = formulas.next()?;
    let mut common = ancestors.get(first)?.clone();
    for formula in formulas {
        let formula_ancestors = ancestors.get(formula)?;
        common.retain(|candidate| formula_ancestors.contains(candidate));
    }
    let lowest = common
        .iter()
        .copied()
        .filter(|candidate| {
            !common.iter().any(|other| {
                other != candidate
                    && ancestors
                        .get(other)
                        .is_some_and(|other_ancestors| other_ancestors.contains(candidate))
            })
        })
        .collect::<Vec<_>>();
    (lowest.len() == 1).then(|| lowest[0])
}

#[requires(true)]
#[ensures(ret.keys().all(|sequence| sequence.object_kind() == SemanticObjectKind::Sequence))]
fn derive_sequence_reachability(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> {
    objects
        .iter()
        .filter(|(_, object)| object.as_sequence().is_some())
        .map(|(&sequence, _)| {
            let mut reachable = BTreeSet::new();
            let mut pending = vec![sequence];
            while let Some(current) = pending.pop() {
                if !reachable.insert(current) {
                    continue;
                }
                if let Some(object) = objects.get(&current) {
                    let mut references = Vec::new();
                    object.references_without_event_bindings_into(&mut references);
                    pending.extend(references);
                }
            }
            (sequence, reachable)
        })
        .collect()
}

#[requires(formulas.iter().all(|formula| formula.object_kind() == SemanticObjectKind::Formula))]
#[requires(sequences.iter().all(|sequence| sequence.object_kind() == SemanticObjectKind::Sequence))]
#[ensures(ret.is_none_or(|sequence| sequence.object_kind() == SemanticObjectKind::Sequence))]
fn lowest_common_sequence(
    formulas: &BTreeSet<SemanticObjectId>,
    sequences: &BTreeSet<SemanticObjectId>,
    reachability: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
) -> Option<SemanticObjectId> {
    let targets = formulas
        .iter()
        .chain(sequences)
        .copied()
        .collect::<Vec<_>>();
    let candidates = reachability
        .iter()
        .filter_map(|(&sequence, reachable)| {
            targets
                .iter()
                .all(|target| reachable.contains(target))
                .then_some(sequence)
        })
        .collect::<BTreeSet<_>>();
    let lowest = candidates
        .iter()
        .copied()
        .filter(|candidate| {
            !candidates.iter().any(|other| {
                other != candidate
                    && reachability
                        .get(candidate)
                        .is_some_and(|reachable| reachable.contains(other))
            })
        })
        .collect::<Vec<_>>();
    (lowest.len() == 1).then(|| lowest[0])
}
