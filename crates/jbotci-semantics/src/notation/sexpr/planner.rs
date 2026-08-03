//! Typed graph-reference and lexical-scope planning.
//!
//! The planner never repairs a graph. It derives binder ownership, graph SCCs,
//! dominators, use counts, and least common definition sites. Any mismatch
//! between graph ownership and the S-expression's lexical binder rules requests
//! the whole-document `TypedGraph` representation.

use std::collections::{BTreeMap, BTreeSet};

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, expensive_invariant, invariant, new, requires};

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

/// Deterministic reference plan for one graph. Binder/use data is always
/// complete. SCC and definition-placement data is computed only while compact
/// rendering remains eligible and is therefore private to the renderer.
#[invariant(true)]
#[derive(Debug, Clone)]
pub(super) struct ReferencePlan {
    failures: Vec<ScopeFailure>,
    use_counts: BTreeMap<SemanticObjectId, usize>,
    uses: BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    binder_owners: BTreeMap<SemanticObjectId, SemanticObjectId>,
    cyclic: BTreeSet<SemanticObjectId>,
    definition_sites: BTreeMap<SemanticObjectId, SemanticObjectId>,
}

/// Linear-space dominance representation. Immediate dominators are computed
/// to a fixed point in reverse-postorder and answer both dominance and LCA
/// queries without retaining one transitive dominator set per graph node.
#[invariant(idom.get(root) == Some(root))]
#[invariant(depth.get(root) == Some(&0))]
#[expensive_invariant(idom.keys().eq(depth.keys()))]
#[expensive_invariant(idom.iter().all(|(node, parent)| {
    node == root || (node != parent && depth.get(parent).is_some_and(|parent_depth| depth[node] == parent_depth + 1))
}))]
#[derive(Debug, Clone)]
struct DominanceTree {
    root: SemanticObjectId,
    idom: BTreeMap<SemanticObjectId, SemanticObjectId>,
    depth: BTreeMap<SemanticObjectId, usize>,
}

impl DominanceTree {
    /// Whether `ancestor` dominates `node` in the rooted reference graph.
    #[requires(true)]
    #[ensures(ret -> self.idom.contains_key(&ancestor) && self.idom.contains_key(&node))]
    fn dominates(&self, ancestor: SemanticObjectId, node: SemanticObjectId) -> bool {
        let (Some(&ancestor_depth), Some(&node_depth)) =
            (self.depth.get(&ancestor), self.depth.get(&node))
        else {
            return false;
        };
        let mut candidate = node;
        let mut candidate_depth = node_depth;
        while candidate_depth > ancestor_depth {
            candidate = self.idom[&candidate];
            candidate_depth -= 1;
        }
        candidate == ancestor
    }

    /// Deepest common dominator of two reachable nodes.
    #[requires(self.idom.contains_key(&left) && self.idom.contains_key(&right))]
    #[ensures(self.dominates(ret, left) && self.dominates(ret, right))]
    fn common_dominator(
        &self,
        mut left: SemanticObjectId,
        mut right: SemanticObjectId,
    ) -> SemanticObjectId {
        let mut left_depth = self.depth[&left];
        let mut right_depth = self.depth[&right];
        while left_depth > right_depth {
            left = self.idom[&left];
            left_depth -= 1;
        }
        while right_depth > left_depth {
            right = self.idom[&right];
            right_depth -= 1;
        }
        while left != right {
            left = self.idom[&left];
            right = self.idom[&right];
        }
        left
    }
}

impl ReferencePlan {
    /// Named failures that require a whole-document typed graph.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn failures(&self) -> &[ScopeFailure] {
        &self.failures
    }

    /// Whether compact lexical rendering is honest for this graph.
    #[requires(true)]
    #[ensures(ret == self.failures.is_empty())]
    pub(super) fn compact_is_eligible(&self) -> bool {
        self.failures.is_empty()
    }

    /// Number of graph edges that use an identity.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn use_count(&self, id: SemanticObjectId) -> usize {
        self.use_counts.get(&id).copied().unwrap_or(0)
    }

    /// Source objects containing a reference to `id`.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn uses_of(&self, id: SemanticObjectId) -> Option<&BTreeSet<SemanticObjectId>> {
        self.uses.get(&id)
    }

    /// The unique typed binder owner, when `id` is a lexical variable.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn binder_owner(&self, id: SemanticObjectId) -> Option<SemanticObjectId> {
        self.binder_owners.get(&id).copied()
    }

    /// Whether `id` participates in a graph cycle and therefore requires
    /// recursive definition treatment when it is not a lexical binder.
    #[requires(self.compact_is_eligible())]
    #[ensures(true)]
    pub(super) fn is_cyclic(&self, id: SemanticObjectId) -> bool {
        self.cyclic.contains(&id)
    }

    /// Least common legal graph scope for a shared definition.
    #[requires(self.compact_is_eligible())]
    #[ensures(true)]
    pub(super) fn definition_site(&self, id: SemanticObjectId) -> Option<SemanticObjectId> {
        self.definition_sites.get(&id).copied()
    }

    /// Exact deterministic single-use inlining rule.
    #[requires(self.compact_is_eligible())]
    #[ensures(ret -> self.use_count(id) == 1 && !self.is_cyclic(id) && self.binder_owner(id).is_none())]
    pub(super) fn may_inline(&self, id: SemanticObjectId) -> bool {
        self.use_count(id) == 1 && !self.is_cyclic(id) && self.binder_owner(id).is_none()
    }
}

/// Build the reference/scope plan from typed graph edges and typed binders.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(true)]
pub(super) fn plan_references(graph: &SemanticGraph) -> ReferencePlan {
    let edges = reference_edges(graph);
    let adjacency = edges
        .iter()
        .map(|(id, targets)| (*id, targets.iter().copied().collect()))
        .collect::<BTreeMap<_, BTreeSet<_>>>();
    let (uses, use_counts) = reverse_uses(&edges);
    let predecessors = reverse_adjacency(&adjacency);
    let reachable = reachable_from(graph.root, &adjacency);
    let dominance = compute_dominance_tree(graph.root, &adjacency, &reachable, &predecessors);
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
                let dominates = dominance.dominates(owner, use_site);
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
            if !dominance.dominates(owner, constant) {
                failures.push(ScopeFailure {
                    kind: ScopeFailureKind::ScopeDependencyWithoutEnclosingBinder,
                    binder: Some(binder),
                    use_site: Some(constant),
                });
            }
        }
    }

    // Binder ownership and scope-universe failures are already sufficient,
    // typed proof that no lexical compact tree can represent this graph. The
    // whole-document TypedGraph path uses only the failure inventory, so SCCs,
    // definition sites, and definition-to-binder dependency closure would be
    // dead work. In particular, avoiding that definition × binder sweep keeps
    // this path linear-space for very large documents such as the Alice graph.
    if !failures.is_empty() {
        failures.sort();
        failures.dedup();
        return ReferencePlan {
            failures,
            use_counts,
            uses,
            binder_owners,
            cyclic: BTreeSet::new(),
            definition_sites: BTreeMap::new(),
        };
    }

    let cyclic = cyclic_nodes(&adjacency);
    let mut definition_sites = BTreeMap::new();
    for id in graph.objects.keys().copied() {
        if binder_owners.contains_key(&id) || use_counts.get(&id).copied().unwrap_or(0) <= 1 {
            continue;
        }
        let sites = uses.get(&id).cloned().unwrap_or_default();
        let Some(site) = least_common_dominator(&sites, &dominance) else {
            failures.push(ScopeFailure {
                kind: ScopeFailureKind::DefinitionSiteDoesNotDominateUse,
                binder: Some(id),
                use_site: None,
            });
            continue;
        };
        if sites
            .iter()
            .any(|use_site| !dominance.dominates(site, *use_site))
        {
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
            let site_is_inside_binder = owner != site && dominance.dominates(*owner, *site);
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
    let mut plan = ReferencePlan {
        failures,
        use_counts,
        uses,
        binder_owners,
        cyclic,
        definition_sites,
    };

    // Exact description inversion turns the graph's recursive self-reference
    // into a lexical `Lo`/`Le` binder. A graph definition site for that same
    // description value is therefore not part of the post-elaboration tree.
    // Run the recognizer against the complete provisional usage plan, then
    // remove only failures whose definition identity is proven consumed by
    // that projection; ordinary binder failures remain untouched.
    let (_, projected_descriptions) = super::elaborate::projected_description_objects(graph, &plan);
    plan.failures.retain(|failure| {
        failure.kind != ScopeFailureKind::DefinitionSiteDoesNotDominateUse
            || failure
                .binder
                .is_none_or(|binder| !projected_descriptions.contains(&binder))
    });
    plan
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

/// Deterministic reverse postorder without recursion proportional to graph
/// depth. Mark-on-entry preserves ordinary DFS postorder for cross edges.
#[requires(adjacency.contains_key(&root))]
#[ensures(ret.first() == Some(&root))]
#[expensive_ensures(
    ret.iter().copied().collect::<BTreeSet<_>>() == reachable_from(root, adjacency)
)]
fn reverse_postorder(
    root: SemanticObjectId,
    adjacency: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
) -> Vec<SemanticObjectId> {
    let mut visited = BTreeSet::new();
    let mut postorder = Vec::new();
    let mut pending = vec![(root, false)];
    while let Some((id, expanded)) = pending.pop() {
        if expanded {
            postorder.push(id);
            continue;
        }
        if !visited.insert(id) {
            continue;
        }
        pending.push((id, true));
        if let Some(targets) = adjacency.get(&id) {
            pending.extend(
                targets
                    .iter()
                    .rev()
                    .filter(|target| adjacency.contains_key(target))
                    .map(|target| (*target, false)),
            );
        }
    }
    postorder.reverse();
    postorder
}

/// Intersect two already-known immediate-dominator chains using reverse-
/// postorder indices. Every step moves at least one finger toward the root.
#[requires(idom.contains_key(&left) && idom.contains_key(&right))]
#[requires(indices.contains_key(&left) && indices.contains_key(&right))]
#[ensures(idom.contains_key(&ret))]
fn intersect_immediate_dominators(
    mut left: SemanticObjectId,
    mut right: SemanticObjectId,
    idom: &BTreeMap<SemanticObjectId, SemanticObjectId>,
    indices: &BTreeMap<SemanticObjectId, usize>,
) -> SemanticObjectId {
    while left != right {
        while indices[&left] > indices[&right] {
            left = idom[&left];
        }
        while indices[&right] > indices[&left] {
            right = idom[&right];
        }
    }
    left
}

/// Exact Cooper-Harvey-Kennedy immediate-dominator fixed point. This computes
/// the same dominance relation as the classic transitive-set algorithm while
/// retaining O(V) state rather than O(V²) sets.
#[requires(reachable.contains(&root))]
#[requires(adjacency.contains_key(&root))]
#[ensures(ret.idom.len() == reachable.len())]
fn compute_dominance_tree(
    root: SemanticObjectId,
    adjacency: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    reachable: &BTreeSet<SemanticObjectId>,
    predecessors: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
) -> DominanceTree {
    let order = reverse_postorder(root, adjacency);
    let indices = order
        .iter()
        .copied()
        .enumerate()
        .map(|(index, id)| (id, index))
        .collect::<BTreeMap<_, _>>();
    let mut idom = BTreeMap::from([(root, root)]);

    loop {
        let mut changed = false;
        for id in order.iter().copied().skip(1) {
            let mut incoming = predecessors
                .get(&id)
                .into_iter()
                .flat_map(|set| set.iter())
                .filter(|predecessor| idom.contains_key(predecessor))
                .copied()
                .collect::<Vec<_>>();
            incoming.sort_by_key(|predecessor| indices[predecessor]);
            let Some(mut next) = incoming.first().copied() else {
                continue;
            };
            for predecessor in incoming.iter().copied().skip(1) {
                next = intersect_immediate_dominators(next, predecessor, &idom, &indices);
            }
            if idom.get(&id) != Some(&next) {
                idom.insert(id, next);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    assert_eq!(
        idom.len(),
        reachable.len(),
        "every reachable non-root has a reachable predecessor"
    );
    let mut depth = BTreeMap::from([(root, 0)]);
    for id in order.iter().copied().skip(1) {
        let parent = idom[&id];
        depth.insert(id, depth[&parent] + 1);
    }
    new!(DominanceTree {
        root: root,
        idom: idom,
        depth: depth
    })
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
        finish_order_iterative(id, adjacency, &mut visited, &mut order);
    }
    let reverse = reverse_adjacency(adjacency);
    visited.clear();
    let mut cyclic = BTreeSet::new();
    for id in order.into_iter().rev() {
        if visited.contains(&id) {
            continue;
        }
        let mut component = BTreeSet::new();
        collect_component_iterative(id, &reverse, &mut visited, &mut component);
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

/// Iterative DFS postorder helper for SCC discovery. An explicit expansion
/// marker avoids consuming call-stack space proportional to document depth.
#[requires(adjacency.contains_key(&id))]
#[ensures(visited.contains(&id))]
fn finish_order_iterative(
    id: SemanticObjectId,
    adjacency: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    visited: &mut BTreeSet<SemanticObjectId>,
    order: &mut Vec<SemanticObjectId>,
) {
    let mut pending = vec![(id, false)];
    while let Some((candidate, expanded)) = pending.pop() {
        if expanded {
            order.push(candidate);
            continue;
        }
        if !visited.insert(candidate) {
            continue;
        }
        pending.push((candidate, true));
        if let Some(targets) = adjacency.get(&candidate) {
            pending.extend(
                targets
                    .iter()
                    .rev()
                    .filter(|target| adjacency.contains_key(target))
                    .map(|target| (*target, false)),
            );
        }
    }
}

/// Iterative reverse-graph traversal for SCC collection.
#[requires(adjacency.contains_key(&id))]
#[ensures(visited.contains(&id))]
fn collect_component_iterative(
    id: SemanticObjectId,
    adjacency: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    visited: &mut BTreeSet<SemanticObjectId>,
    component: &mut BTreeSet<SemanticObjectId>,
) {
    let mut pending = vec![id];
    while let Some(candidate) = pending.pop() {
        if !visited.insert(candidate) {
            continue;
        }
        component.insert(candidate);
        if let Some(targets) = adjacency.get(&candidate) {
            for target in targets.iter().rev() {
                if adjacency.contains_key(target) {
                    pending.push(*target);
                }
            }
        }
    }
}

/// Deepest common dominator of all use sites, independent of traversal order.
#[requires(true)]
#[ensures(ret.is_none() || sites.iter().all(|site| dominance.dominates(ret.unwrap(), *site)))]
fn least_common_dominator(
    sites: &BTreeSet<SemanticObjectId>,
    dominance: &DominanceTree,
) -> Option<SemanticObjectId> {
    let mut iter = sites.iter();
    let mut common = *iter.next()?;
    if !dominance.idom.contains_key(&common) {
        return None;
    }
    for site in iter {
        if !dominance.idom.contains_key(site) {
            return None;
        }
        common = dominance.common_dominator(common, *site);
    }
    Some(common)
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    use super::*;

    /// Three stable identities used by the bounded directed-graph oracle.
    #[requires(true)]
    #[ensures(ret.len() == 3)]
    fn nodes() -> [SemanticObjectId; 3] {
        [
            SemanticObjectId::formula(1),
            SemanticObjectId::formula(2),
            SemanticObjectId::formula(3),
        ]
    }

    /// Decode one complete directed graph, including self edges, from a mask.
    #[requires(mask < (1 << 9))]
    #[ensures(ret.len() == 3)]
    fn adjacency(mask: usize) -> BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> {
        let nodes = nodes();
        nodes
            .iter()
            .copied()
            .enumerate()
            .map(|(source_index, source)| {
                let targets = nodes
                    .iter()
                    .copied()
                    .enumerate()
                    .filter_map(|(target_index, target)| {
                        let bit = source_index * nodes.len() + target_index;
                        (mask & (1 << bit) != 0).then_some(target)
                    })
                    .collect();
                (source, targets)
            })
            .collect()
    }

    /// Small transitive-set oracle retained only for exhaustive tests.
    #[requires(reachable.contains(&root))]
    #[ensures(ret.len() == reachable.len())]
    fn dominator_set_oracle(
        root: SemanticObjectId,
        reachable: &BTreeSet<SemanticObjectId>,
        predecessors: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    ) -> BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> {
        let mut dominators = reachable
            .iter()
            .copied()
            .map(|id| {
                (
                    id,
                    if id == root {
                        BTreeSet::from([root])
                    } else {
                        reachable.clone()
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        loop {
            let mut changed = false;
            for id in reachable.iter().copied().filter(|id| *id != root) {
                let incoming = predecessors[&id]
                    .iter()
                    .filter(|predecessor| reachable.contains(predecessor))
                    .copied()
                    .collect::<Vec<_>>();
                let mut next = dominators[&incoming[0]].clone();
                for predecessor in &incoming[1..] {
                    next.retain(|candidate| dominators[predecessor].contains(candidate));
                }
                next.insert(id);
                if dominators[&id] != next {
                    dominators.insert(id, next);
                    changed = true;
                }
            }
            if !changed {
                return dominators;
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn immediate_dominator_tree_matches_bounded_set_oracle() {
        let root = nodes()[0];
        for mask in 0..(1 << 9) {
            let adjacency = adjacency(mask);
            let reachable = reachable_from(root, &adjacency);
            let predecessors = reverse_adjacency(&adjacency);
            let tree = compute_dominance_tree(root, &adjacency, &reachable, &predecessors);
            let oracle = dominator_set_oracle(root, &reachable, &predecessors);
            for node in &reachable {
                for candidate in &reachable {
                    assert_eq!(
                        tree.dominates(*candidate, *node),
                        oracle[node].contains(candidate),
                        "mask={mask:#011b}, candidate={candidate}, node={node}"
                    );
                }
            }
            for left in &reachable {
                for right in &reachable {
                    let common = tree.common_dominator(*left, *right);
                    let expected = oracle[left]
                        .intersection(&oracle[right])
                        .copied()
                        .max_by_key(|candidate| oracle[candidate].len())
                        .expect("root is a common dominator");
                    assert_eq!(common, expected, "mask={mask:#011b}");
                }
            }
        }
    }
}
