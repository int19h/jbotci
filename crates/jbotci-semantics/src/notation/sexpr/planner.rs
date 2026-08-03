//! Typed graph-reference and lexical-scope planning.
//!
//! The planner never repairs a graph. It derives binder ownership, graph SCCs,
//! dominators, use counts, and least common definition sites. Any mismatch
//! between graph ownership and the S-expression's lexical binder rules requests
//! the whole-document `TypedGraph` representation.

use std::collections::{BTreeMap, BTreeSet};

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};

use crate::model::{
    FormulaNodeData, SemanticGraph, SemanticObjectData, SemanticObjectId,
    semantic_scope_dependence_binder_universes,
};

/// Named compact-scope failure classes from the approved design.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScopeFailureKind {
    MultipleBinderOwners,
    BinderDoesNotEncloseUse,
    ScopeDependencyWithoutEnclosingBinder,
    UnrepresentableCycle,
    DefinitionSiteDoesNotDominateUse,
    DeclarationPlanningDidNotConverge,
}

impl ScopeFailureKind {
    /// Stable corpus-report spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn label(self) -> &'static str {
        match self {
            Self::MultipleBinderOwners => "multiple-binder-owners",
            Self::BinderDoesNotEncloseUse => "binder-does-not-enclose-use",
            Self::ScopeDependencyWithoutEnclosingBinder => {
                "scope-dependency-without-enclosing-binder"
            }
            Self::UnrepresentableCycle => "unrepresentable-cycle",
            Self::DefinitionSiteDoesNotDominateUse => "definition-site-does-not-dominate-use",
            Self::DeclarationPlanningDidNotConverge => "declaration-planning-did-not-converge",
        }
    }
}

/// Evidence for a scope failure. Optional IDs identify the affected binder and
/// use site without changing the failure class used for aggregate reporting.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScopeFailure {
    pub kind: ScopeFailureKind,
    pub binder: Option<SemanticObjectId>,
    pub use_site: Option<SemanticObjectId>,
}

/// Complete deterministic reference plan for one graph.
#[invariant(true)]
#[derive(Debug, Clone)]
pub struct ReferencePlan {
    failures: Vec<ScopeFailure>,
    use_counts: BTreeMap<SemanticObjectId, usize>,
    uses: BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    binder_owners: BTreeMap<SemanticObjectId, SemanticObjectId>,
    cyclic: BTreeSet<SemanticObjectId>,
    definition_sites: BTreeMap<SemanticObjectId, SemanticObjectId>,
}

impl ReferencePlan {
    /// Named failures that require a whole-document typed graph.
    #[requires(true)]
    #[ensures(true)]
    pub fn failures(&self) -> &[ScopeFailure] {
        &self.failures
    }

    /// Whether compact lexical rendering is honest for this graph.
    #[requires(true)]
    #[ensures(ret == self.failures.is_empty())]
    pub fn compact_is_eligible(&self) -> bool {
        self.failures.is_empty()
    }

    /// Number of graph edges that use an identity.
    #[requires(true)]
    #[ensures(true)]
    pub fn use_count(&self, id: SemanticObjectId) -> usize {
        self.use_counts.get(&id).copied().unwrap_or(0)
    }

    /// Source objects containing a reference to `id`.
    #[requires(true)]
    #[ensures(true)]
    pub fn uses_of(&self, id: SemanticObjectId) -> Option<&BTreeSet<SemanticObjectId>> {
        self.uses.get(&id)
    }

    /// The unique typed binder owner, when `id` is a lexical variable.
    #[requires(true)]
    #[ensures(true)]
    pub fn binder_owner(&self, id: SemanticObjectId) -> Option<SemanticObjectId> {
        self.binder_owners.get(&id).copied()
    }

    /// Whether `id` participates in a graph cycle and therefore requires
    /// recursive definition treatment when it is not a lexical binder.
    #[requires(true)]
    #[ensures(true)]
    pub fn is_cyclic(&self, id: SemanticObjectId) -> bool {
        self.cyclic.contains(&id)
    }

    /// Least common legal graph scope for a shared definition.
    #[requires(true)]
    #[ensures(true)]
    pub fn definition_site(&self, id: SemanticObjectId) -> Option<SemanticObjectId> {
        self.definition_sites.get(&id).copied()
    }

    /// Exact deterministic single-use inlining rule.
    #[requires(true)]
    #[ensures(ret -> self.use_count(id) == 1 && !self.is_cyclic(id) && self.binder_owner(id).is_none())]
    pub fn may_inline(&self, id: SemanticObjectId) -> bool {
        self.use_count(id) == 1 && !self.is_cyclic(id) && self.binder_owner(id).is_none()
    }
}

/// Build the reference/scope plan from typed graph edges and typed binders.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(true)]
pub fn plan_references(graph: &SemanticGraph) -> ReferencePlan {
    let edges = reference_edges(graph);
    let adjacency = edges
        .iter()
        .map(|(id, targets)| (*id, targets.iter().copied().collect()))
        .collect::<BTreeMap<_, BTreeSet<_>>>();
    let (uses, use_counts) = reverse_uses(&edges);
    let predecessors = reverse_adjacency(&adjacency);
    let reachable = reachable_from(graph.root, &adjacency);
    let dominators = compute_dominators(graph.root, &reachable, &predecessors);
    let (binder_owner_candidates, binder_scope_roots) = binder_ownership(graph);
    let mut failures = Vec::new();
    let mut binder_owners = BTreeMap::new();

    for (binder, owners) in &binder_owner_candidates {
        if owners.len() != 1 {
            failures.push(ScopeFailure {
                kind: ScopeFailureKind::MultipleBinderOwners,
                binder: Some(*binder),
                use_site: None,
            });
            continue;
        }
        let owner = *owners.first().expect("one-owner branch is nonempty");
        binder_owners.insert(*binder, owner);
        let permitted = binder_scope_roots
            .get(&(*binder, owner))
            .map(|roots| reachable_from_roots(roots, &adjacency))
            .unwrap_or_default();
        if let Some(use_sites) = uses.get(binder) {
            for use_site in use_sites
                .iter()
                .copied()
                .filter(|use_site| *use_site != owner)
            {
                let dominates = dominators
                    .get(&use_site)
                    .is_some_and(|set| set.contains(&owner));
                if !dominates || !permitted.contains(&use_site) {
                    failures.push(ScopeFailure {
                        kind: ScopeFailureKind::BinderDoesNotEncloseUse,
                        binder: Some(*binder),
                        use_site: Some(use_site),
                    });
                }
            }
        }
    }

    let universes = semantic_scope_dependence_binder_universes(graph.root, &graph.objects);
    for (constant, universe) in universes {
        for binder in universe {
            let Some(owner) = binder_owners.get(&binder).copied() else {
                failures.push(ScopeFailure {
                    kind: ScopeFailureKind::ScopeDependencyWithoutEnclosingBinder,
                    binder: Some(binder),
                    use_site: Some(constant),
                });
                continue;
            };
            if !dominators
                .get(&constant)
                .is_some_and(|set| set.contains(&owner))
            {
                failures.push(ScopeFailure {
                    kind: ScopeFailureKind::ScopeDependencyWithoutEnclosingBinder,
                    binder: Some(binder),
                    use_site: Some(constant),
                });
            }
        }
    }

    let cyclic = cyclic_nodes(&adjacency);
    let mut definition_sites = BTreeMap::new();
    for id in graph.objects.keys().copied() {
        if binder_owners.contains_key(&id) || use_counts.get(&id).copied().unwrap_or(0) <= 1 {
            continue;
        }
        let sites = uses.get(&id).cloned().unwrap_or_default();
        let Some(site) = least_common_dominator(&sites, &dominators) else {
            failures.push(ScopeFailure {
                kind: ScopeFailureKind::DefinitionSiteDoesNotDominateUse,
                binder: Some(id),
                use_site: None,
            });
            continue;
        };
        if sites.iter().any(|use_site| {
            !dominators
                .get(use_site)
                .is_some_and(|set| set.contains(&site))
        }) {
            failures.push(ScopeFailure {
                kind: ScopeFailureKind::DefinitionSiteDoesNotDominateUse,
                binder: Some(id),
                use_site: None,
            });
        } else {
            definition_sites.insert(id, site);
        }
    }

    // A definition that is not placed strictly inside its dependency's binder
    // is lexically outside that binder, even when graph cycles make the binder
    // dominate all original use edges. There is no faithful `Let` position at
    // or above the owner spanning its separate restriction/body operands.
    // Preserve identity and scope with TypedGraph.
    // A definition owner's own parameters are internal to its rendered value
    // and therefore do not constrain the definition's outer placement.
    // Generated events are excluded here because their binding decision is
    // made after compact elaboration: exact forms absorb a default event,
    // while a typed fallback leaves a `$` use that `bind_generated_events`
    // detects and binds at the graph-owned closure site.
    for (definition, site) in &definition_sites {
        let dependencies = reachable_from_roots(&BTreeSet::from([*definition]), &adjacency);
        for (binder, owner) in &binder_owners {
            let site_is_inside_binder = owner != site
                && dominators
                    .get(site)
                    .is_some_and(|dominators| dominators.contains(owner));
            if owner != definition
                && dependencies.contains(binder)
                && !graph.objects[binder].is_generated_eventuality()
                && !site_is_inside_binder
            {
                failures.push(ScopeFailure {
                    kind: ScopeFailureKind::DefinitionSiteDoesNotDominateUse,
                    binder: Some(*definition),
                    use_site: Some(*binder),
                });
            }
        }
    }

    failures.sort();
    failures.dedup();
    ReferencePlan {
        failures,
        use_counts,
        uses,
        binder_owners,
        cyclic,
        definition_sites,
    }
}

/// Collect graph edges in stable object-ID order.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.len() == graph.objects.len())]
fn reference_edges(graph: &SemanticGraph) -> BTreeMap<SemanticObjectId, Vec<SemanticObjectId>> {
    graph
        .objects
        .iter()
        .map(|(id, object)| {
            let mut references = Vec::new();
            object.references_into(&mut references);
            (*id, references)
        })
        .collect()
}

/// Invert edges and retain both source sets and edge multiplicities.
#[requires(true)]
#[ensures(true)]
fn reverse_uses(
    adjacency: &BTreeMap<SemanticObjectId, Vec<SemanticObjectId>>,
) -> (
    BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    BTreeMap<SemanticObjectId, usize>,
) {
    let mut uses: BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> = BTreeMap::new();
    let mut counts = BTreeMap::new();
    for (source, targets) in adjacency {
        for target in targets {
            uses.entry(*target).or_default().insert(*source);
            *counts.entry(*target).or_default() += 1;
        }
    }
    (uses, counts)
}

/// Invert adjacency for the dominator fixed point.
#[requires(true)]
#[ensures(ret.len() == adjacency.len())]
fn reverse_adjacency(
    adjacency: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
) -> BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> {
    let mut reverse = adjacency
        .keys()
        .copied()
        .map(|id| (id, BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for (source, targets) in adjacency {
        for target in targets {
            reverse.entry(*target).or_default().insert(*source);
        }
    }
    reverse
}

/// Reachability from one root.
#[requires(adjacency.contains_key(&root))]
#[ensures(ret.contains(&root))]
fn reachable_from(
    root: SemanticObjectId,
    adjacency: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
) -> BTreeSet<SemanticObjectId> {
    reachable_from_roots(&BTreeSet::from([root]), adjacency)
}

/// Reachability from a stable set of roots.
#[requires(true)]
#[ensures(ret.is_superset(roots))]
fn reachable_from_roots(
    roots: &BTreeSet<SemanticObjectId>,
    adjacency: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
) -> BTreeSet<SemanticObjectId> {
    let mut reached = BTreeSet::new();
    let mut pending = roots.iter().copied().collect::<Vec<_>>();
    while let Some(id) = pending.pop() {
        if !reached.insert(id) {
            continue;
        }
        if let Some(next) = adjacency.get(&id) {
            pending.extend(next.iter().rev().copied());
        }
    }
    reached
}

/// Classic iterative dominators over the rooted reference graph.
#[requires(reachable.contains(&root))]
#[ensures(ret.get(&root).is_some_and(|set| set == &BTreeSet::from([root])))]
fn compute_dominators(
    root: SemanticObjectId,
    reachable: &BTreeSet<SemanticObjectId>,
    predecessors: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
) -> BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> {
    let mut dominators = reachable
        .iter()
        .copied()
        .map(|id| {
            let initial = if id == root {
                BTreeSet::from([root])
            } else {
                reachable.clone()
            };
            (id, initial)
        })
        .collect::<BTreeMap<_, _>>();

    loop {
        let mut changed = false;
        for id in reachable.iter().copied().filter(|id| *id != root) {
            let incoming = predecessors
                .get(&id)
                .into_iter()
                .flat_map(|set| set.iter())
                .filter(|predecessor| reachable.contains(predecessor))
                .copied()
                .collect::<Vec<_>>();
            let mut next = if let Some(first) = incoming.first() {
                dominators[first].clone()
            } else {
                BTreeSet::new()
            };
            for predecessor in incoming.iter().skip(1) {
                next.retain(|candidate| dominators[predecessor].contains(candidate));
            }
            next.insert(id);
            if dominators.get(&id) != Some(&next) {
                dominators.insert(id, next);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    dominators
}

/// Discover every typed binder owner and its lexical scope roots.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(true)]
#[allow(clippy::type_complexity)]
fn binder_ownership(
    graph: &SemanticGraph,
) -> (
    BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    BTreeMap<(SemanticObjectId, SemanticObjectId), BTreeSet<SemanticObjectId>>,
) {
    let mut owners: BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> = BTreeMap::new();
    let mut scope_roots = BTreeMap::new();
    for (owner, object) in &graph.objects {
        match object.as_data() {
            data!(SemanticObject::Formula(formula)) => match formula.as_data() {
                data!(FormulaNode::Atom(node)) => {
                    register_generated_binders(
                        *owner,
                        node.bound_eventualities.iter().map(|id| id.object_id()),
                        BTreeSet::from([node.predication]),
                        &mut owners,
                        &mut scope_roots,
                    );
                }
                data!(FormulaNode::Connective(node)) => {
                    register_generated_binders(
                        *owner,
                        node.bound_eventualities.iter().map(|id| id.object_id()),
                        node.children.iter().copied().collect(),
                        &mut owners,
                        &mut scope_roots,
                    );
                }
                data!(FormulaNode::Quantified(node)) => {
                    register_binder(
                        *owner,
                        node.variable,
                        node.restriction.into_iter().chain([node.body]).collect(),
                        &mut owners,
                        &mut scope_roots,
                    );
                    register_generated_binders(
                        *owner,
                        node.bound_eventualities.iter().map(|id| id.object_id()),
                        BTreeSet::from([node.body]),
                        &mut owners,
                        &mut scope_roots,
                    );
                }
                data!(FormulaNode::QuantifierBundle(node)) => {
                    let roots = node
                        .bindings
                        .iter()
                        .filter_map(|binding| binding.restriction)
                        .chain([node.body])
                        .collect::<BTreeSet<_>>();
                    for binding in &node.bindings {
                        register_binder(
                            *owner,
                            binding.variable,
                            roots.clone(),
                            &mut owners,
                            &mut scope_roots,
                        );
                    }
                    register_generated_binders(
                        *owner,
                        node.bound_eventualities.iter().map(|id| id.object_id()),
                        roots,
                        &mut owners,
                        &mut scope_roots,
                    );
                }
                data!(FormulaNode::RespectivelyDistribution(node)) => {
                    let roots = node
                        .streams
                        .iter()
                        .filter_map(|stream| stream.restriction)
                        .chain([node.body])
                        .collect::<BTreeSet<_>>();
                    for stream in &node.streams {
                        register_binder(
                            *owner,
                            stream.slot,
                            roots.clone(),
                            &mut owners,
                            &mut scope_roots,
                        );
                    }
                    register_generated_binders(
                        *owner,
                        node.bound_eventualities.iter().map(|id| id.object_id()),
                        roots,
                        &mut owners,
                        &mut scope_roots,
                    );
                }
            },
            data!(SemanticObject::Sequence(node)) => register_generated_binders(
                *owner,
                node.bound_eventualities.iter().map(|id| id.object_id()),
                node.content
                    .into_iter()
                    .chain(node.connection_claims.iter().copied())
                    .chain(node.items.iter().copied())
                    .collect(),
                &mut owners,
                &mut scope_roots,
            ),
            data!(SemanticObject::Eventuality(node)) => {
                if let Some(body) = node.body {
                    for parameter in &node.parameters {
                        register_binder(
                            *owner,
                            *parameter,
                            BTreeSet::from([body]),
                            &mut owners,
                            &mut scope_roots,
                        );
                    }
                }
            }
            data!(SemanticObject::Referent(node)) => {
                if let Some(body) = node.body {
                    for parameter in &node.parameters {
                        register_binder(
                            *owner,
                            *parameter,
                            BTreeSet::from([body]),
                            &mut owners,
                            &mut scope_roots,
                        );
                    }
                }
            }
            data!(SemanticObject::Question(node)) => {
                for parameter in node.slots.iter().filter_map(|slot| slot.parameter()) {
                    register_binder(
                        *owner,
                        parameter,
                        BTreeSet::from([node.body]),
                        &mut owners,
                        &mut scope_roots,
                    );
                }
            }
            _ => {}
        }
    }
    (owners, scope_roots)
}

/// Register one binder owner and its complete lexical roots.
#[requires(true)]
#[ensures(owners.get(&binder).is_some_and(|set| set.contains(&owner)))]
fn register_binder(
    owner: SemanticObjectId,
    binder: SemanticObjectId,
    roots: BTreeSet<SemanticObjectId>,
    owners: &mut BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    scope_roots: &mut BTreeMap<(SemanticObjectId, SemanticObjectId), BTreeSet<SemanticObjectId>>,
) {
    owners.entry(binder).or_default().insert(owner);
    scope_roots
        .entry((binder, owner))
        .or_default()
        .extend(roots);
}

/// Register a sequence of graph-owned generated-event binders.
#[requires(true)]
#[ensures(true)]
fn register_generated_binders(
    owner: SemanticObjectId,
    binders: impl IntoIterator<Item = SemanticObjectId>,
    roots: BTreeSet<SemanticObjectId>,
    owners: &mut BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    scope_roots: &mut BTreeMap<(SemanticObjectId, SemanticObjectId), BTreeSet<SemanticObjectId>>,
) {
    for binder in binders {
        register_binder(owner, binder, roots.clone(), owners, scope_roots);
    }
}

/// Compute graph nodes belonging to cyclic SCCs using deterministic Kosaraju
/// passes over the reference graph.
#[requires(true)]
#[ensures(ret.iter().all(|id| adjacency.contains_key(id)))]
fn cyclic_nodes(
    adjacency: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
) -> BTreeSet<SemanticObjectId> {
    let mut visited = BTreeSet::new();
    let mut order = Vec::new();
    for id in adjacency.keys().copied() {
        finish_order(id, adjacency, &mut visited, &mut order);
    }
    let reverse = reverse_adjacency(adjacency);
    visited.clear();
    let mut cyclic = BTreeSet::new();
    for id in order.into_iter().rev() {
        if visited.contains(&id) {
            continue;
        }
        let mut component = BTreeSet::new();
        collect_component(id, &reverse, &mut visited, &mut component);
        let self_edge = component.len() == 1
            && adjacency
                .get(&id)
                .is_some_and(|targets| targets.contains(&id));
        if component.len() > 1 || self_edge {
            cyclic.extend(component);
        }
    }
    cyclic
}

/// DFS postorder helper for SCC discovery.
#[requires(adjacency.contains_key(&id))]
#[ensures(visited.contains(&id))]
fn finish_order(
    id: SemanticObjectId,
    adjacency: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    visited: &mut BTreeSet<SemanticObjectId>,
    order: &mut Vec<SemanticObjectId>,
) {
    if !visited.insert(id) {
        return;
    }
    if let Some(targets) = adjacency.get(&id) {
        for target in targets {
            if adjacency.contains_key(target) {
                finish_order(*target, adjacency, visited, order);
            }
        }
    }
    order.push(id);
}

/// Reverse-graph DFS helper for SCC discovery.
#[requires(adjacency.contains_key(&id))]
#[ensures(visited.contains(&id))]
fn collect_component(
    id: SemanticObjectId,
    adjacency: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    visited: &mut BTreeSet<SemanticObjectId>,
    component: &mut BTreeSet<SemanticObjectId>,
) {
    if !visited.insert(id) {
        return;
    }
    component.insert(id);
    if let Some(targets) = adjacency.get(&id) {
        for target in targets {
            collect_component(*target, adjacency, visited, component);
        }
    }
}

/// Deepest common dominator of all use sites; depth is the number of strict
/// dominators and therefore independent of graph traversal order.
#[requires(true)]
#[ensures(ret.is_none() || sites.iter().all(|site| dominators.get(site).is_some_and(|set| set.contains(&ret.unwrap()))))]
fn least_common_dominator(
    sites: &BTreeSet<SemanticObjectId>,
    dominators: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
) -> Option<SemanticObjectId> {
    let mut iter = sites.iter();
    let first = iter.next()?;
    let mut common = dominators.get(first)?.clone();
    for site in iter {
        let site_dominators = dominators.get(site)?;
        common.retain(|candidate| site_dominators.contains(candidate));
    }
    common
        .into_iter()
        .max_by_key(|candidate| dominators.get(candidate).map_or(0, BTreeSet::len))
}
