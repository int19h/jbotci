//! Structural graph-reference and lexical-scope planning.
//!
//! The planner never repairs a graph. It reads the scope structure the builder
//! recorded — the region forest, object origins, and the per-edge occurrence
//! table — and decides where each shared identity is declared. Any mismatch
//! between what the graph binds and what a lexical S-expression can express
//! requests the whole-document `TypedGraph` representation.
//!
//! Two records answer two different questions and are deliberately kept apart:
//!
//! * [`GraphUsage`] counts *value* occurrences. It is what recognizers ask when
//!   they need to know whether an identity is used more than once, and it is
//!   computed before any placement decision so that the projection pre-scan can
//!   run as an explicit pre-plan phase rather than a call back into the
//!   elaborator from inside planning.
//! * [`ReferencePlan`] places identities. Placement reads *occurrences*, not
//!   collapsed use-site sets: one owner object can hold two references
//!   evaluated in different regions, and a reference occurring inside its own
//!   target's definition is not a use that definition must be hosted to cover.
//!
//! Both read the same occurrence table, and for the same reason: a raw
//! reference edge does not say what kind of occurrence it is. A description's
//! body mentions the description it defines, and a binder-introducing node
//! mentions the variable it introduces; neither is a use that makes the
//! identity shared. The builder already classifies every edge, so counting
//! edges instead of occurrences reports both as sharing and turns an ordinary
//! description into an untypeable shared definition.
//!
//! Placement itself is one bottom-up pass over the SCC-condensed use DAG. The
//! host of an identity is the deepest position whose rendered value encloses
//! every occurrence that has to see it, and legality is decided against the
//! region forest: a host is legal only when every binder the hosted value
//! depends on is live on the host's own region path. There is no fixed point
//! and no reverse-reference dominance oracle.

use std::collections::{BTreeMap, BTreeSet};

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, expensive_invariant, invariant, new, requires};

use crate::model::{
    ScopeBoundaryData, ScopeRegionId, ScopeUseRole, SemanticGraph, SemanticObjectId,
    SemanticScopeTree, scope_binder_introduction_conflicts, scope_unenclosed_binder_uses,
    semantic_graph_generated_event_binders,
};

/// Named compact-scope failure classes from the approved design.
///
/// The placement classes the reverse-reference oracle used to need —
/// `definition-site-does-not-dominate-use` and
/// `declaration-planning-did-not-converge` — are gone. A region forest is a
/// tree, so a common host always exists, and placement is a single bottom-up
/// pass, so there is no fixed point to miss. What survives here are the
/// properties of the *graph* that no lexical tree can express, plus the one
/// representability gap the specification tracks.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScopeFailureKind {
    MultipleBinderOwners,
    BinderDoesNotEncloseUse,
    /// A recorded scope dependence names a binder that owns no scope anywhere
    /// in the graph. That is an unbound variable, not a placement question, so
    /// it takes the registered whole-graph reason rather than the
    /// binder-placement one.
    ScopeDependencyBinderUnowned,
    ScopeDependencyWithoutEnclosingBinder,
    UnrepresentableCycle,
    /// A dynamic reference computation whose uses lie in two performed force
    /// segments. Version 0 hosts an ordinary `Refer` inside its own segment, so
    /// sharing one across segments needs an explicit legal discourse host the
    /// graph does not supply, and no single host is determined.
    DynamicHostNotUnique,
}

impl ScopeFailureKind {
    /// Stable corpus-report spelling.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn label(self) -> &'static str {
        match self {
            Self::MultipleBinderOwners => "multiple-binder-owners",
            Self::BinderDoesNotEncloseUse => "binder-does-not-enclose-use",
            Self::ScopeDependencyBinderUnowned => "scope-dependency-binder-unowned",
            Self::ScopeDependencyWithoutEnclosingBinder => {
                "scope-dependency-without-enclosing-binder"
            }
            Self::UnrepresentableCycle => "unrepresentable-cycle",
            Self::DynamicHostNotUnique => "dynamic-host-not-unique",
        }
    }

    /// Stable registered whole-document fallback reason for this failure.
    #[requires(true)]
    #[ensures(ret.starts_with("smusni.projection."))]
    pub(super) fn reason_id(self) -> &'static str {
        match self {
            Self::MultipleBinderOwners => "smusni.projection.conflicting-binder-owners",
            Self::BinderDoesNotEncloseUse => "smusni.projection.binder-does-not-dominate-use",
            Self::ScopeDependencyBinderUnowned => "smusni.projection.graph.unbound-variable",
            Self::ScopeDependencyWithoutEnclosingBinder => {
                "smusni.projection.scope-dependency-without-binder"
            }
            Self::UnrepresentableCycle => "smusni.projection.unguarded-or-unrepresentable-scc",
            Self::DynamicHostNotUnique => "smusni.projection.dynamic-host-not-unique",
        }
    }

    /// Stable human-readable statement of the scope mismatch.
    ///
    /// Fixed per failure class so a per-edge diagnostic record's message is
    /// determined by its typed cause rather than composed at the failure site;
    /// the affected binder and use site travel as typed evidence instead.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(super) fn message(self) -> &'static str {
        match self {
            Self::MultipleBinderOwners => "this binder is owned by more than one graph scope",
            Self::BinderDoesNotEncloseUse => "the binder's scope does not enclose this use",
            Self::ScopeDependencyBinderUnowned => {
                "this variable has no binder anywhere in the graph"
            }
            Self::ScopeDependencyWithoutEnclosingBinder => {
                "a recorded scope dependence names a binder that does not enclose the constant"
            }
            Self::UnrepresentableCycle => "this graph cycle has no representable lexical form",
            Self::DynamicHostNotUnique => {
                "this reference computation is used in two force segments and the graph names no discourse host"
            }
        }
    }
}

/// One failed projection edge found by scope planning.
///
/// The whole `(kind, binder, use_site)` triple is the edge's identity: one
/// binder can fail against several distinct use sites, and several binders can
/// fail for one reason, so only an exact repeat of the triple is a duplicate
/// discovery of one edge. This is the planner's counterpart to the elaborator's
/// `(owner, cause)` edge identity.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScopeFailure {
    pub kind: ScopeFailureKind,
    pub binder: Option<SemanticObjectId>,
    pub use_site: Option<SemanticObjectId>,
}

/// Whether one occurrence uses the identity it names as a value.
///
/// A binder declaration introduces the identity, and a reference inside the
/// target's own definition is a bound candidate of that definition rather than
/// a use the definition would have to be hosted to cover. Both are the record's
/// own classification, which is why every question about sharing asks it here
/// rather than counting reference edges.
#[requires(true)]
#[ensures(ret == matches!(role, ScopeUseRole::Value | ScopeUseRole::BinderUse))]
fn occurrence_is_value_use(role: ScopeUseRole) -> bool {
    match role {
        ScopeUseRole::Value | ScopeUseRole::BinderUse => true,
        ScopeUseRole::BinderDeclaration | ScopeUseRole::DefinitionInternal => false,
    }
}

/// Order one planner channel and drop only exact repeats of a single edge.
///
/// Sorting before deduplicating is what makes the order stable across runs and
/// what makes `dedup` see every repeat, since discovery order follows the
/// traversal rather than identity order.
#[requires(true)]
#[ensures(
    ret.windows(2).all(|pair| pair[0] < pair[1]),
    "the channel is strictly increasing, so it is both ordered and deduplicated"
)]
#[ensures(
    ret.len() <= old(failures.len()),
    "deduplication only removes repeats; it never invents an edge"
)]
fn stable_failure_channel(mut failures: Vec<ScopeFailure>) -> Vec<ScopeFailure> {
    failures.sort();
    failures.dedup();
    failures
}

/// Occurrence-level usage and binder ownership, independent of any placement.
///
/// This is the pre-plan record. It answers "how many value occurrences reach
/// this identity" and "which node binds this variable" — both decided by the
/// graph alone — so the projection pre-scan can consume it before hosts exist.
/// Splitting it out is what severs the planner's old back-reference into the
/// elaborator.
#[invariant(binder_owners.keys().all(|binder| !conflicting_binders.iter().any(|(other, _)| other == binder)),
    "a binder with more than one introducing region has no unique owner")]
#[derive(Debug, Clone)]
pub(super) struct GraphUsage {
    use_counts: BTreeMap<SemanticObjectId, usize>,
    uses: BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    binder_owners: BTreeMap<SemanticObjectId, SemanticObjectId>,
    conflicting_binders: Vec<(SemanticObjectId, Vec<ScopeRegionId>)>,
}

impl GraphUsage {
    /// Number of value occurrences of an identity.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn use_count(&self, id: SemanticObjectId) -> usize {
        self.use_counts.get(&id).copied().unwrap_or(0)
    }

    /// Source objects whose value occurrences reach `id`.
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

    /// Whether some region or node introduces `id` as a binder at all.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn is_binder(&self, id: SemanticObjectId) -> bool {
        self.binder_owners.contains_key(&id)
            || self
                .conflicting_binders
                .iter()
                .any(|(binder, _)| *binder == id)
    }
}

/// Build the pre-plan usage record: value multiplicities and binder ownership.
///
/// Usage is read off the occurrence table, not swept out of the reference
/// edges, because only the table says what an edge *is*. A binder declaration
/// introduces the identity rather than using it, and a reference inside the
/// target's own definition is a bound candidate of that definition; counting
/// either as a use makes an ordinary description look shared and forces it into
/// a `Let` no reference type can spell.
///
/// Binder ownership is read the same way. Lexical binders are the ones the
/// region forest introduces, so their owner is the object owning the
/// introducing region; generated event binders are declared by the formula and
/// sequence nodes that carry them, which
/// [`ScopeRegion`](crate::model::ScopeRegion) cannot record because its binder
/// vocabulary is restricted to referent and parameter objects.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(true)]
pub(super) fn index_graph_usage(graph: &SemanticGraph) -> GraphUsage {
    let mut uses: BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> = BTreeMap::new();
    let mut use_counts = BTreeMap::new();
    for occurrence in &graph.scope.uses {
        if !occurrence_is_value_use(occurrence.role) {
            continue;
        }
        uses.entry(occurrence.target)
            .or_default()
            .insert(occurrence.owner);
        *use_counts.entry(occurrence.target).or_default() += 1;
    }

    let conflicting_binders = scope_binder_introduction_conflicts(&graph.scope.regions);
    let mut binder_owners = BTreeMap::new();
    for region in graph.scope.regions.values() {
        let Some(owner) = region.owner.object else {
            continue;
        };
        for binder in &region.binders {
            if conflicting_binders
                .iter()
                .any(|(conflicting, _)| conflicting == binder)
            {
                continue;
            }
            binder_owners.insert(*binder, owner);
        }
    }
    for (binder, owners) in semantic_graph_generated_event_binders(&graph.objects) {
        // A generated event bound by two nodes is the same conflict the region
        // forest reports for a lexical variable, and takes the same route.
        let mut owners = owners.into_iter();
        let Some(owner) = owners.next() else {
            continue;
        };
        if owners.next().is_some() {
            continue;
        }
        binder_owners.insert(binder, owner);
    }

    new!(GraphUsage {
        use_counts: use_counts,
        uses: uses,
        binder_owners: binder_owners,
        conflicting_binders: conflicting_binders
    })
}

/// The lexical containment tree the elaborator renders.
///
/// `parent(Y)` is the deepest object whose rendered value encloses every
/// occurrence of `Y` that has to see it, which is exactly where a declaration
/// of `Y` may be placed. The tree is built from the occurrence table in one
/// pass over the SCC-condensed use DAG: a referrer's own position is settled
/// before the referent's, so the ancestor computation never iterates.
#[invariant(parent.get(root) == Some(root))]
#[invariant(depth.get(root) == Some(&0))]
#[expensive_invariant(parent.keys().eq(depth.keys()))]
#[expensive_invariant(parent.iter().all(|(node, above)| {
    node == root || (node != above && depth.get(above).is_some_and(|above_depth| depth[node] == above_depth + 1))
}))]
#[derive(Debug, Clone)]
struct ContainmentTree {
    root: SemanticObjectId,
    parent: BTreeMap<SemanticObjectId, SemanticObjectId>,
    depth: BTreeMap<SemanticObjectId, usize>,
}

impl ContainmentTree {
    /// Whether `node` has a rendered position at all.
    #[requires(true)]
    #[ensures(ret == self.depth.contains_key(&node))]
    fn contains(&self, node: SemanticObjectId) -> bool {
        self.depth.contains_key(&node)
    }

    /// The planned position of `node`, when it has one.
    #[requires(true)]
    #[ensures(ret.is_some() == self.parent.contains_key(&node))]
    fn position(&self, node: SemanticObjectId) -> Option<SemanticObjectId> {
        self.parent.get(&node).copied()
    }
}

/// Deepest position enclosing both nodes, over a partially built tree.
///
/// The two fingers each move strictly toward the root, so this terminates on
/// any well-formed parent/depth pair and needs no ownership of the maps — which
/// is what lets the placement pass fold it over a tree it is still extending.
#[requires(depth.contains_key(&left) && depth.contains_key(&right))]
#[ensures(depth.contains_key(&ret))]
fn common_ancestor(
    parent: &BTreeMap<SemanticObjectId, SemanticObjectId>,
    depth: &BTreeMap<SemanticObjectId, usize>,
    mut left: SemanticObjectId,
    mut right: SemanticObjectId,
) -> SemanticObjectId {
    let mut left_depth = depth[&left];
    let mut right_depth = depth[&right];
    while left_depth > right_depth {
        left = parent[&left];
        left_depth -= 1;
    }
    while right_depth > left_depth {
        right = parent[&right];
        right_depth -= 1;
    }
    while left != right {
        left = parent[&left];
        right = parent[&right];
    }
    left
}

/// Deterministic reference plan for one graph. Usage and binder data is always
/// complete. Placement data is computed only while compact rendering remains
/// eligible and is therefore private to the renderer.
#[invariant(true)]
#[derive(Debug, Clone)]
pub(super) struct ReferencePlan {
    failures: Vec<ScopeFailure>,
    usage: GraphUsage,
    cyclic: BTreeSet<SemanticObjectId>,
    definition_sites: BTreeMap<SemanticObjectId, SemanticObjectId>,
    declaration_order: BTreeMap<SemanticObjectId, usize>,
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

    /// The pre-plan usage record this plan was built over.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn usage(&self) -> &GraphUsage {
        &self.usage
    }

    /// Number of value occurrences of an identity.
    #[requires(true)]
    #[ensures(ret == self.usage.use_count(id))]
    pub(super) fn use_count(&self, id: SemanticObjectId) -> usize {
        self.usage.use_count(id)
    }

    /// Source objects whose value occurrences reach `id`.
    #[requires(true)]
    #[ensures(true)]
    pub(super) fn uses_of(&self, id: SemanticObjectId) -> Option<&BTreeSet<SemanticObjectId>> {
        self.usage.uses_of(id)
    }

    /// The unique typed binder owner, when `id` is a lexical variable.
    #[requires(true)]
    #[ensures(ret == self.usage.binder_owner(id))]
    pub(super) fn binder_owner(&self, id: SemanticObjectId) -> Option<SemanticObjectId> {
        self.usage.binder_owner(id)
    }

    /// Whether `id` participates in a value-use cycle and therefore requires
    /// recursive definition treatment when it is not a lexical binder.
    #[requires(self.compact_is_eligible())]
    #[ensures(true)]
    pub(super) fn is_cyclic(&self, id: SemanticObjectId) -> bool {
        self.cyclic.contains(&id)
    }

    /// Planned host of a shared declaration: the object whose rendered value
    /// the declaration wraps.
    #[requires(self.compact_is_eligible())]
    #[ensures(true)]
    pub(super) fn definition_site(&self, id: SemanticObjectId) -> Option<SemanticObjectId> {
        self.definition_sites.get(&id).copied()
    }

    /// Total order over declarations sharing one host, after their dependency
    /// constraints.
    ///
    /// Source-order keys are unique only inside one region and are meaningless
    /// across regions, so what is compared here is the position of an
    /// identity's first value occurrence in the occurrence table. That table is
    /// enumerated in the graph's canonical reference order, and the keys inside
    /// each region are handed out in exactly that order, so the table position
    /// is the total refinement of the per-region source order — and, for the
    /// builder's syntax-directed walk, source order itself. Ties fall to the
    /// object id.
    #[requires(self.compact_is_eligible())]
    #[ensures(true)]
    pub(super) fn declaration_order(&self, id: SemanticObjectId) -> (usize, SemanticObjectId) {
        (
            self.declaration_order
                .get(&id)
                .copied()
                .unwrap_or(usize::MAX),
            id,
        )
    }

    /// Exact deterministic single-use inlining rule.
    #[requires(self.compact_is_eligible())]
    #[ensures(ret -> self.use_count(id) == 1 && !self.is_cyclic(id) && self.binder_owner(id).is_none())]
    pub(super) fn may_inline(&self, id: SemanticObjectId) -> bool {
        self.use_count(id) == 1 && !self.is_cyclic(id) && self.binder_owner(id).is_none()
    }
}

/// Identities whose declaration the renderer owns rather than the graph.
///
/// An exact description projection turns the graph's recursive self-reference
/// into a lexical `Lo`/`Le` binder, so no graph host declares that value, and
/// the support subgraph the constructor consumes is never declared at all. The
/// pre-scan that recognizes them runs before planning and hands the result in
/// here, which is why planning no longer calls back into the elaborator.
#[invariant(true)]
#[derive(Debug, Clone, Default)]
pub(super) struct ProjectedIdentities {
    /// Objects consumed inside an exact projection and therefore never declared.
    pub(super) support: BTreeSet<SemanticObjectId>,
    /// Values whose binder the renderer owns, so no graph placement can fail.
    pub(super) values: BTreeSet<SemanticObjectId>,
    /// Identities that print as a canonical atom wherever they occur. Repeating
    /// one costs nothing and shares no state, so the notation never declares
    /// them however many edges reach them.
    pub(super) atoms: BTreeSet<SemanticObjectId>,
    /// Quantifier restrictions whose only other occurrences are the bound
    /// variable's own argument-site repeats, mapped to the variable that
    /// discharges them. Those repeats print nothing: section 8.4 conjoins the
    /// clause with the quantifier's restriction at that one locus.
    pub(super) discharged_restrictions: BTreeMap<SemanticObjectId, SemanticObjectId>,
}

impl ProjectedIdentities {
    /// Whether a declaration of `id` can exist at all.
    #[requires(true)]
    #[ensures(true)]
    fn is_declarable(&self, id: SemanticObjectId) -> bool {
        !self.support.contains(&id) && !self.atoms.contains(&id)
    }
}

/// Build the reference/scope plan from the recorded scope structure.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(true)]
pub(super) fn plan_references(
    graph: &SemanticGraph,
    usage: GraphUsage,
    projected: &ProjectedIdentities,
) -> ReferencePlan {
    let scope = &graph.scope;
    let mut failures = Vec::new();

    for (binder, _) in &usage.conflicting_binders {
        failures.push(ScopeFailure {
            kind: ScopeFailureKind::MultipleBinderOwners,
            binder: Some(*binder),
            use_site: None,
        });
    }

    // The occurrence table already says which region each binder use is
    // evaluated in, so an escaping use is read off rather than reconstructed.
    for occurrence in scope_unenclosed_binder_uses(&scope.regions, &scope.uses) {
        failures.push(ScopeFailure {
            kind: ScopeFailureKind::BinderDoesNotEncloseUse,
            binder: Some(occurrence.target),
            use_site: Some(occurrence.owner),
        });
    }

    for (id, object) in &graph.objects {
        let Some(dependence) = object.scope_dependence() else {
            continue;
        };
        let Some(universe) = dependence.may_depend_on() else {
            continue;
        };
        let live = scope
            .origin(*id)
            .map(|origin| scope.binder_universe(origin))
            .unwrap_or_default();
        for binder in universe {
            if !usage.is_binder(*binder) {
                // Nothing binds this variable anywhere, so the dependence names
                // a free variable rather than a placement question.
                failures.push(ScopeFailure {
                    kind: ScopeFailureKind::ScopeDependencyBinderUnowned,
                    binder: Some(*binder),
                    use_site: Some(*id),
                });
            } else if !live.contains(binder)
                && !graph.objects[binder].is_generated_eventuality()
                && usage.binder_owner(*binder) != Some(*id)
            {
                failures.push(ScopeFailure {
                    kind: ScopeFailureKind::ScopeDependencyWithoutEnclosingBinder,
                    binder: Some(*binder),
                    use_site: Some(*id),
                });
            }
        }
    }

    // Binder ownership and dependence failures are already sufficient, typed
    // proof that no lexical compact tree can represent this graph. The
    // whole-document TypedGraph path uses only the failure inventory, so
    // placement would be dead work — and skipping it keeps this path
    // linear-space for very large documents such as the Alice graph.
    if !failures.is_empty() {
        return ReferencePlan {
            failures: stable_failure_channel(failures),
            usage,
            cyclic: BTreeSet::new(),
            definition_sites: BTreeMap::new(),
            declaration_order: BTreeMap::new(),
        };
    }

    let edges = reference_edges(graph);
    let adjacency = edges
        .iter()
        .map(|(id, targets)| (*id, targets.iter().copied().collect()))
        .collect::<BTreeMap<_, BTreeSet<_>>>();

    let (declaration_users, hosted_by) = placement_users(graph);
    let declarations = placement_adjacency(&graph.objects, &declaration_users);
    // A cycle is a property of the *value* graph. A description whose body
    // mentions the description it defines is not recursive — that occurrence is
    // the bound candidate the definition introduces — so reading cyclicity off
    // reference edges would demand a guarded recursive form for every ordinary
    // `lo broda`.
    let cyclic = cyclic_nodes(&declarations);
    let placement = placement_adjacency(&graph.objects, &hosted_by);
    let components = strong_components(&placement);
    let containment = build_containment(graph.root, &components, &hosted_by);

    let mut definition_sites = BTreeMap::new();
    for id in graph.objects.keys().copied() {
        if usage.is_binder(id)
            || !projected.is_declarable(id)
            || usage.use_count(id) <= 1
            || id == graph.root
        {
            continue;
        }
        let Some(site) = containment.position(id) else {
            continue;
        };
        definition_sites.insert(id, site);
    }

    // A declaration is legal at its host only if every binder its value depends
    // on is live on the host's own region path. Moving the host outward can
    // only lose binders, and moving it inward would stop covering a use, so a
    // violation is a graph the lexical notation cannot bind at all rather than
    // a placement to retry: it takes the registered unbound-variable route.
    //
    // A declaration owner's own parameters are internal to its rendered value
    // and do not constrain its outer placement. Generated events are excluded
    // because their binding decision is made after compact elaboration: exact
    // forms absorb a default event, while a typed fallback leaves a `$` use
    // that `bind_generated_events` binds at the graph-owned closure site.
    for (definition, site) in &definition_sites {
        if projected.values.contains(definition) {
            continue;
        }
        let live = scope
            .origin(*site)
            .map(|origin| scope.binder_universe(origin))
            .unwrap_or_default();
        for binder in free_binders(graph, &adjacency, &usage, *definition) {
            if live.contains(&binder) {
                continue;
            }
            failures.push(ScopeFailure {
                kind: ScopeFailureKind::ScopeDependencyWithoutEnclosingBinder,
                binder: Some(binder),
                use_site: Some(*definition),
            });
        }
    }

    // One `LetRec` per cyclic component, and only when every initializer is an
    // inert lambda. A component touching a value that prints no binder has no
    // guarded lexical form, which is the specification's tracked gap rather
    // than a placement question.
    let lambda_shaped = objects_introducing_binders(graph);
    for component in strong_components(&declarations) {
        let Some(entry) = component.iter().copied().next() else {
            continue;
        };
        // Mutual recursion between two declarations is the planner's to decide:
        // it is a property of the graph's value structure and is settled once
        // hosts exist. A lone self-reference is not, because whether it survives
        // into a declaration at all depends on how the object is recognized —
        // an indexical's own deictic ground, for instance, disappears into the
        // printed atom — so that case stays with the recognizer that knows.
        //
        // Only a cycle that becomes one declaration group has no lexical form:
        // a component the renderer's own projection consumes end to end, or
        // that prints as a canonical atom wherever it occurs, never reaches
        // `LetRec`. A group of inert lambdas is exactly what `LetRec` is for.
        if component.len() < 2
            || !component
                .iter()
                .any(|id| definition_sites.contains_key(id) && projected.is_declarable(*id))
            || component.iter().all(|id| lambda_shaped.contains(id))
        {
            continue;
        }
        failures.push(ScopeFailure {
            kind: ScopeFailureKind::UnrepresentableCycle,
            binder: Some(entry),
            use_site: None,
        });
    }

    // Section 6.3 hosts an ordinary `Refer` inside its own force segment, and
    // the renderer owns the binder of every projected description, so a
    // description shared by two performed acts has no host the notation can
    // print: the binder would stand in one act and the variable in the other.
    // Deciding this needs the boundary *kind*, which is why the planner reads
    // the region forest's boundary table rather than only its shape.
    for value in &projected.values {
        if segments_of_value_uses(scope, &declaration_users, *value).len() > 1 {
            failures.push(ScopeFailure {
                kind: ScopeFailureKind::DynamicHostNotUnique,
                binder: Some(*value),
                use_site: None,
            });
        }
    }

    let declaration_order = declaration_order_keys(graph);
    ReferencePlan {
        failures: stable_failure_channel(failures),
        usage,
        cyclic,
        definition_sites,
        declaration_order,
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

/// The occurrences a declaration of each identity has to cover.
///
/// Ordinary value uses decide placement. A reference occurring inside its own
/// target's definition is a bound candidate rather than a use — the role the
/// builder records exists for exactly this — and a binder declaration is not a
/// use at all. An identity whose only remaining occurrences are
/// definition-internal still has to be printed somewhere, so those occurrences
/// are its placement constraint when nothing else is.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.0.keys().all(|target| ret.1.contains_key(target)))]
#[allow(clippy::type_complexity)]
fn placement_users(
    graph: &SemanticGraph,
) -> (
    BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
) {
    let mut value: BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> = BTreeMap::new();
    let mut internal: BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> = BTreeMap::new();
    for occurrence in &graph.scope.uses {
        if occurrence_is_value_use(occurrence.role) {
            value
                .entry(occurrence.target)
                .or_default()
                .insert(occurrence.owner);
            continue;
        }
        // Exhaustive on purpose: a role added later has to be dispositioned
        // here as well as in `occurrence_is_value_use`, rather than silently
        // becoming neither a use nor a placement constraint.
        match occurrence.role {
            ScopeUseRole::DefinitionInternal => {
                internal
                    .entry(occurrence.target)
                    .or_default()
                    .insert(occurrence.owner);
            }
            ScopeUseRole::BinderDeclaration => {}
            ScopeUseRole::Value | ScopeUseRole::BinderUse => {
                unreachable!("a value occurrence is handled above")
            }
        }
    }
    let mut placement = value.clone();
    for (target, owners) in internal {
        // An identity whose ordinary uses are gone still has to be printed
        // somewhere, and inside its own definition is the only position left.
        // That position is a placement constraint, never a mutual dependency
        // between two declarations, which is why the two maps stay separate.
        placement.entry(target).or_default().extend(owners);
    }
    (value, placement)
}

/// Invert the placement occurrences into owner-to-target adjacency.
#[requires(true)]
#[ensures(true)]
fn placement_adjacency(
    objects: &BTreeMap<SemanticObjectId, crate::model::SemanticObject>,
    hosted_by: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
) -> BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> {
    let mut adjacency = objects
        .keys()
        .copied()
        .map(|id| (id, BTreeSet::new()))
        .collect::<BTreeMap<_, BTreeSet<_>>>();
    for (target, owners) in hosted_by {
        for owner in owners {
            adjacency.entry(*owner).or_default().insert(*target);
        }
    }
    adjacency
}

/// Place every reachable identity in one bottom-up pass.
///
/// Components arrive in topological order, so every external user of a
/// component is already placed when the component is reached and its host is
/// one fold of [`ContainmentTree::common_ancestor`]. Members of one cyclic
/// component share a host, which is what makes their declarations one group.
#[requires(true)]
#[ensures(ret.contains(root))]
fn build_containment(
    root: SemanticObjectId,
    components: &[BTreeSet<SemanticObjectId>],
    hosted_by: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
) -> ContainmentTree {
    let mut parent = BTreeMap::from([(root, root)]);
    let mut depth = BTreeMap::from([(root, 0)]);
    for component in components {
        if component.contains(&root) {
            for member in component.iter().copied().filter(|member| *member != root) {
                parent.insert(member, root);
                depth.insert(member, 1);
            }
            continue;
        }
        let external = component
            .iter()
            .filter_map(|member| hosted_by.get(member))
            .flatten()
            .copied()
            .filter(|user| !component.contains(user) && depth.contains_key(user))
            .collect::<BTreeSet<_>>();
        let mut users = external.iter().copied();
        let Some(mut host) = users.next() else {
            continue;
        };
        for user in users {
            host = common_ancestor(&parent, &depth, host, user);
        }
        let host_depth = depth[&host] + 1;
        for member in component {
            parent.insert(*member, host);
            depth.insert(*member, host_depth);
        }
    }
    new!(ContainmentTree {
        root: root,
        parent: parent,
        depth: depth
    })
}

/// Position of each identity's first value occurrence in the occurrence table.
#[requires(true)]
#[ensures(true)]
fn declaration_order_keys(graph: &SemanticGraph) -> BTreeMap<SemanticObjectId, usize> {
    let mut keys = BTreeMap::new();
    for (index, occurrence) in graph.scope.uses.iter().enumerate() {
        if occurrence.role == ScopeUseRole::Value {
            keys.entry(occurrence.target).or_insert(index);
        }
    }
    keys
}

/// Binders a declaration's own value leaves free.
///
/// A binder reached through the value is only a constraint on where that value
/// may be declared when the value does not itself introduce it: a definition
/// containing a quantifier binds that quantifier's variable inside its own
/// printed form, so no enclosing host has to supply it. Generated event binders
/// are excluded outright — their binding decision is made after compact
/// elaboration, where exact forms absorb a default event and a typed fallback
/// leaves a `$` use that `bind_generated_events` binds at the graph-owned
/// closure site.
#[requires(graph.objects.contains_key(&definition))]
#[ensures(ret.iter().all(|binder| usage.is_binder(*binder)))]
fn free_binders(
    graph: &SemanticGraph,
    adjacency: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    usage: &GraphUsage,
    definition: SemanticObjectId,
) -> BTreeSet<SemanticObjectId> {
    let value = reachable_from_roots(&BTreeSet::from([definition]), adjacency);
    value
        .iter()
        .copied()
        .filter(|id| usage.is_binder(*id))
        .filter(|id| !graph.objects[id].is_generated_eventuality())
        .filter(|id| {
            usage
                .binder_owner(*id)
                .is_none_or(|owner| !value.contains(&owner))
        })
        .collect()
}

/// The force segments one identity's value uses are evaluated in.
///
/// A segment is the nearest enclosing performed act, opaque mention, or the
/// document itself — the same three boundaries the recorder uses to decide
/// which act a value was written in. Nothing between them stops a lexical
/// binder, so two uses in one segment always have a printable common host and
/// two uses in different segments never do.
#[requires(true)]
#[ensures(true)]
fn segments_of_value_uses(
    scope: &SemanticScopeTree,
    declaration_users: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    value: SemanticObjectId,
) -> BTreeSet<ScopeRegionId> {
    let mut segments = BTreeSet::new();
    for occurrence in &scope.uses {
        if occurrence.target != value
            || occurrence.role != ScopeUseRole::Value
            || !declaration_users
                .get(&value)
                .is_some_and(|users| users.contains(&occurrence.owner))
        {
            continue;
        }
        segments.insert(force_segment(scope, occurrence.region));
    }
    segments
}

/// The nearest enclosing force segment, opaque mention, or document region.
#[requires(true)]
#[ensures(true)]
fn force_segment(scope: &SemanticScopeTree, region: ScopeRegionId) -> ScopeRegionId {
    for ancestor in scope.ancestry(region) {
        if scope.region(ancestor).is_some_and(|node| {
            matches!(
                node.boundary.as_data(),
                data!(ScopeBoundary::Document)
                    | data!(ScopeBoundary::ForceSegment)
                    | data!(ScopeBoundary::Opaque)
            )
        }) {
            return ancestor;
        }
    }
    scope.root
}

/// Objects that print a binder of their own, which is what a recursive
/// declaration group needs of every one of its initializers.
#[requires(true)]
#[ensures(true)]
fn objects_introducing_binders(graph: &SemanticGraph) -> BTreeSet<SemanticObjectId> {
    graph
        .scope
        .regions
        .values()
        .filter(|region| !region.binders.is_empty())
        .filter_map(|region| region.owner.object)
        .collect()
}

/// Invert adjacency.
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

/// Deterministic Kosaraju condensation, emitted in topological order.
#[requires(true)]
#[ensures(ret.iter().flatten().all(|id| adjacency.contains_key(id)))]
pub(super) fn strong_components(
    adjacency: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
) -> Vec<BTreeSet<SemanticObjectId>> {
    let mut visited = BTreeSet::new();
    let mut order = Vec::new();
    for id in adjacency.keys().copied() {
        finish_order_iterative(id, adjacency, &mut visited, &mut order);
    }
    let reverse = reverse_adjacency(adjacency);
    visited.clear();
    let mut components = Vec::new();
    for id in order.into_iter().rev() {
        if visited.contains(&id) {
            continue;
        }
        let mut component = BTreeSet::new();
        collect_component_iterative(id, &reverse, &mut visited, &mut component);
        components.push(component);
    }
    components
}

/// Compute graph nodes belonging to cyclic SCCs of the value-use graph.
#[requires(true)]
#[ensures(ret.iter().all(|id| adjacency.contains_key(id)))]
fn cyclic_nodes(
    adjacency: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
) -> BTreeSet<SemanticObjectId> {
    let mut cyclic = BTreeSet::new();
    for component in strong_components(adjacency) {
        let self_edge = component.len() == 1
            && component.iter().next().is_some_and(|id| {
                adjacency
                    .get(id)
                    .is_some_and(|targets| targets.contains(id))
            });
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

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    use super::*;

    /// Compose one scope-failure edge for the channel laws.
    #[requires(true)]
    #[ensures(ret.kind == kind)]
    fn scope_edge(
        kind: ScopeFailureKind,
        binder: Option<usize>,
        use_site: Option<usize>,
    ) -> ScopeFailure {
        ScopeFailure {
            kind,
            binder: binder.map(SemanticObjectId::referent),
            use_site: use_site.map(SemanticObjectId::referent),
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn rediscovering_one_scope_edge_records_it_once() {
        let repeated = scope_edge(ScopeFailureKind::BinderDoesNotEncloseUse, Some(1), Some(2));
        assert_eq!(
            stable_failure_channel(vec![repeated, repeated, repeated]),
            vec![repeated],
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn one_binder_failing_against_two_use_sites_is_two_scope_edges() {
        // Collapsing to the failure class alone would report one record here,
        // which is exactly what the per-edge contract forbids.
        let first = scope_edge(ScopeFailureKind::BinderDoesNotEncloseUse, Some(1), Some(2));
        let second = scope_edge(ScopeFailureKind::BinderDoesNotEncloseUse, Some(1), Some(3));
        let channel = stable_failure_channel(vec![second, first, second]);
        assert_eq!(channel, vec![first, second]);
        assert_eq!(channel[0].kind, channel[1].kind);
        assert_eq!(channel[0].binder, channel[1].binder);
        assert_ne!(channel[0].use_site, channel[1].use_site);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn two_binders_failing_for_one_reason_are_two_scope_edges() {
        let first = scope_edge(ScopeFailureKind::MultipleBinderOwners, Some(1), None);
        let second = scope_edge(ScopeFailureKind::MultipleBinderOwners, Some(2), None);
        let channel = stable_failure_channel(vec![second, first]);
        assert_eq!(channel, vec![first, second]);
        assert_eq!(first.kind.reason_id(), second.kind.reason_id());
        assert_eq!(first.kind.message(), second.kind.message());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn the_scope_edge_channel_is_stably_ordered_whatever_the_discovery_order() {
        let edges = [
            scope_edge(ScopeFailureKind::UnrepresentableCycle, Some(2), None),
            scope_edge(ScopeFailureKind::MultipleBinderOwners, Some(3), None),
            scope_edge(ScopeFailureKind::BinderDoesNotEncloseUse, Some(1), Some(4)),
            scope_edge(ScopeFailureKind::MultipleBinderOwners, Some(1), None),
        ];
        let mut expected = edges;
        expected.sort();
        for rotation in 0..edges.len() {
            let mut discovered = edges.to_vec();
            discovered.rotate_left(rotation);
            assert_eq!(stable_failure_channel(discovered), expected.to_vec());
        }
        assert!(expected.windows(2).all(|pair| pair[0] < pair[1]));
    }

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

    /// Invert adjacency into the placement-user form `build_containment` reads.
    #[requires(true)]
    #[ensures(true)]
    fn users_of(
        adjacency: &BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    ) -> BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> {
        let mut users: BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> = BTreeMap::new();
        for (source, targets) in adjacency {
            for target in targets {
                users.entry(*target).or_default().insert(*source);
            }
        }
        users
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

    /// The containment tree is the graph's dominator tree, computed from
    /// occurrences in one pass instead of from a reverse-edge fixed point.
    ///
    /// This is the property that lets the placement rewrite be judged as a
    /// change of *inputs* — occurrences with roles, and region legality —
    /// rather than a change of what "encloses every use" means.
    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn containment_placement_matches_the_bounded_dominator_oracle() {
        let root = nodes()[0];
        for mask in 0..(1 << 9) {
            let adjacency = adjacency(mask);
            let hosted_by = users_of(&adjacency);
            let reachable = reachable_from_roots(&BTreeSet::from([root]), &adjacency);
            let predecessors = reverse_adjacency(&adjacency);
            let components = strong_components(&adjacency);
            let tree = build_containment(root, &components, &hosted_by);
            let oracle = dominator_set_oracle(root, &reachable, &predecessors);
            for node in reachable.iter().copied().filter(|node| *node != root) {
                // One cyclic component shares a host, so the oracle's immediate
                // dominator of a component member can be another member. Both
                // records agree on where the *component* is placed.
                let component = components
                    .iter()
                    .find(|component| component.contains(&node))
                    .expect("every node is in one component");
                let expected = oracle[&node]
                    .iter()
                    .copied()
                    .filter(|candidate| *candidate != node && !component.contains(candidate))
                    .max_by_key(|candidate| oracle[candidate].len());
                assert_eq!(
                    tree.parent.get(&node).copied(),
                    expected.or(Some(root)),
                    "mask={mask:#011b}, node={node}"
                );
            }
            for node in adjacency.keys().copied() {
                assert_eq!(
                    tree.contains(node),
                    reachable.contains(&node),
                    "mask={mask:#011b}, node={node}"
                );
            }
        }
    }
}
