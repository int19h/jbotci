//! Builder-recorded structural scope: the region forest, object origins, and
//! the reference-occurrence table.
//!
//! The semantic object graph is a DAG of shared identities. It records *what*
//! refers to what, never *where* a reference is evaluated, so downstream
//! consumers used to reconstruct lexical enclosure from reverse reference
//! edges. That reconstruction is unsound: one owner object can hold two
//! references evaluated in different scopes (a quantified formula's restriction
//! versus its body; two operands of a connective), and a shared target can be
//! reached first along a path unrelated to its introduction.
//!
//! This module records the missing structure at the only place that has it —
//! the builder, which sits on the syntax tree — as ordinary additive model
//! data:
//!
//! * [`ScopeRegion`] — one node of a single rooted region tree, carrying a
//!   typed owner locus, a closed [`ScopeBoundary`] kind, and the binders the
//!   region introduces.
//! * `object_origins` — the region each object is introduced in.
//! * `uses` — one [`ScopeUseOccurrence`] per semantic reference edge, so two
//!   references held by one owner but evaluated in different regions are
//!   distinguishable.
//!
//! Nothing in this module decides scope by itself: the source-order keys order
//! already-placed definitions and effects and never participate in enclosure.

use super::*;

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_invariant, invariant, new, requires};

/// Identifier of one scope region, rendered as `scope:<index>`.
#[invariant(*index > 0, "region indices are one-based")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeRegionId {
    index: usize,
}

impl ScopeRegionId {
    #[requires(index > 0)]
    #[ensures(ret.index() == index)]
    pub fn new(index: usize) -> Self {
        new!(ScopeRegionId { index })
    }

    #[requires(true)]
    #[ensures(ret > 0)]
    pub fn index(self) -> usize {
        self.index
    }
}

impl fmt::Display for ScopeRegionId {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "scope:{}", self.index)
    }
}

impl Serialize for ScopeRegionId {
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

/// A total order over loci within one region.
///
/// Keys are allocated in construction order, which is syntax order for the
/// builder's syntax-directed walk. They order effects and definitions that are
/// already hosted at the same region; they never decide enclosure, and a
/// consumer that compares keys across regions is reading them wrong.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceOrderKey {
    key: usize,
}

impl Serialize for SourceOrderKey {
    #[requires(true)]
    #[ensures(ret.is_ok() || ret.is_err())]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.key as u64)
    }
}

impl SourceOrderKey {
    #[requires(true)]
    #[ensures(ret.get() == key)]
    pub fn new(key: usize) -> Self {
        SourceOrderKey { key }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn get(self) -> usize {
        self.key
    }
}

/// The closed semantic boundary vocabulary.
///
/// The boundary kind, not an object's spelling, decides what a later host
/// planner may cross. `Closure` and `NonlogicalJoin` are recorded explicitly
/// *because* they are transparent: transparency is a stated property of the
/// boundary table rather than something a consumer infers from absence.
#[invariant(::Document => true, "the synthetic document root has no payload")]
#[invariant(::Discourse => true, "a discourse continuation has no payload")]
#[invariant(::ForceSegment => true, "a performed force segment has no payload")]
#[invariant(::SequentialContinuation => true, "a sequential continuation has no payload")]
#[invariant(::ConnectiveBranch => true, "a connective operand has no payload")]
#[invariant(::Multiplicity => true, "a lambda or quantifier region has no payload")]
#[invariant(::Question => true, "a question region has no payload")]
#[invariant(::LexicalArgument { relation, original_place } =>
    !relation.is_empty() && original_place.get() > 0,
    "a lexical argument names the relation root and its original numbered place")]
#[invariant(::Reification => true, "a reification boundary has no payload")]
#[invariant(::ContentCrossing => true, "a content-to-object crossing has no payload")]
#[invariant(::TokenFacts => true, "a token-facts boundary has no payload")]
#[invariant(::Opaque => true, "an opaque or mention boundary has no payload")]
#[invariant(::Closure => true, "an expected-type closure has no payload")]
#[invariant(::NonlogicalJoin => true, "a nonlogical join has no payload")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ScopeBoundary {
    /// The synthetic document root every other region descends from.
    Document,
    /// A discourse whose items continue one another without a performed force.
    Discourse,
    /// One performed act. Dynamic sharing never crosses this edge.
    ForceSegment,
    /// One item of an ordered sequence.
    SequentialContinuation,
    /// One operand of a logical connective.
    ConnectiveBranch,
    /// A lambda or quantifier multiplicity region: the binders it introduces
    /// are live in it and in its descendants only.
    Multiplicity,
    /// A question region: its slots are answered, not hoisted.
    Question,
    /// A lexical argument place, carrying the verified `(relation root,
    /// original ordinal)` key its scope policy is resolved by.
    #[serde(rename_all = "camelCase")]
    LexicalArgument {
        relation: String,
        original_place: PlaceIndex,
    },
    /// A reification site: content is turned into an object here.
    Reification,
    /// A content-to-object crossing registered by the projection registry.
    ContentCrossing,
    /// Analyzer facts about tokens rather than about what they denote.
    TokenFacts,
    /// An opaque or mention-like barrier: quotation and the like.
    Opaque,
    /// A recorded-transparent expected-type closure.
    Closure,
    /// A recorded-transparent nonlogical (`joi`-family) join.
    NonlogicalJoin,
}

impl ScopeBoundary {
    /// Whether reference placement may move through this boundary.
    ///
    /// Only the two boundaries the design records *as* transparent report
    /// `true`; everything else is a barrier whose legality a consumer decides
    /// per effect family.
    #[requires(true)]
    #[ensures(ret == matches!(self.as_data(), data!(ScopeBoundary::Closure) | data!(ScopeBoundary::NonlogicalJoin)))]
    pub fn is_transparent(&self) -> bool {
        matches!(
            self.as_data(),
            data!(ScopeBoundary::Closure) | data!(ScopeBoundary::NonlogicalJoin)
        )
    }
}

/// The locus of a region inside its owning object.
///
/// `Object` names the object as a whole; every other variant names one
/// structural position, so a region can be identified without depending on the
/// order references happen to be enumerated in.
#[invariant(::Document => true, "the document locus carries no ordinal")]
#[invariant(::Object => true, "the whole-object locus carries no ordinal")]
#[invariant(::Content => true, "the content locus carries no ordinal")]
#[invariant(::Body => true, "the body locus carries no ordinal")]
#[invariant(::Binder { .. } => true, "every binder ordinal names a possible binder")]
#[invariant(::Denotation => true, "the denotation locus carries no ordinal")]
#[invariant(::Focus => true, "the focus locus carries no ordinal")]
#[invariant(::Property => true, "the property locus carries no ordinal")]
#[invariant(::Restriction { .. } => true, "every binding ordinal names a possible binding")]
#[invariant(::Operand { .. } => true, "every operand ordinal names a possible operand")]
#[invariant(::Item { .. } => true, "every item ordinal names a possible item")]
#[invariant(::Argument { place } => place.get() > 0, "argument places are one-based")]
#[invariant(::RelativeClause { .. } => true, "every clause ordinal names a possible clause")]
#[invariant(::Quotation => true, "the quotation locus carries no ordinal")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ScopeSite {
    Document,
    Object,
    Content,
    Body,
    /// The declaration edge by which a region introduces one of its binders.
    Binder {
        index: usize,
    },
    /// What an analyzed token denotes, as a token fact rather than a value use.
    Denotation,
    /// A question's focus or presupposed answer: a reference to one of the
    /// question's own slots, evaluated where that slot is introduced.
    Focus,
    /// A descriptor's property: the lambda a description is the referent of.
    Property,
    Restriction {
        binding: usize,
    },
    Operand {
        index: usize,
    },
    Item {
        index: usize,
    },
    Argument {
        place: PlaceIndex,
    },
    RelativeClause {
        index: usize,
    },
    Quotation,
}

/// The typed owner of a region: an object position, or the document itself.
#[invariant(object.is_none() == matches!(site.as_data(), data!(ScopeSite::Document)),
    "exactly the document locus is owned by no object")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeOwner {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<SemanticObjectId>,
    pub site: ScopeSite,
}

impl ScopeOwner {
    #[requires(true)]
    #[ensures(ret.object.is_none())]
    pub fn document() -> Self {
        new!(ScopeOwner {
            object: None,
            site: new!(ScopeSite::Document),
        })
    }

    #[requires(!matches!(site.as_data(), data!(ScopeSite::Document)))]
    #[ensures(ret.object == Some(object) && ret.site == site)]
    pub fn locus(object: SemanticObjectId, site: ScopeSite) -> Self {
        new!(ScopeOwner {
            object: Some(object),
            site,
        })
    }
}

/// One node of the region tree.
#[invariant(binders.iter().all(|binder| quantifier_variable_kind_is_allowed(binder.object_kind())),
    "only referent and parameter objects can be binders")]
#[invariant(binders.iter().collect::<BTreeSet<_>>().len() == binders.len(),
    "a region introduces each of its binders once")]
#[invariant(binders.is_empty() || matches!(boundary.as_data(),
    data!(ScopeBoundary::Multiplicity) | data!(ScopeBoundary::Question)),
    "binders are introduced only by multiplicity and question regions")]
#[invariant(parent.is_none() == matches!(boundary.as_data(), data!(ScopeBoundary::Document)),
    "exactly the document region is parentless")]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeRegion {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<ScopeRegionId>,
    pub owner: ScopeOwner,
    pub boundary: ScopeBoundary,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub binders: Vec<SemanticObjectId>,
    pub source_order: SourceOrderKey,
}

/// What one reference edge does with its target.
///
/// The distinction is recorded rather than recognized: a description's
/// self-reference inside its own defining property is a bound candidate, not a
/// use that a definition must be hoisted to cover, and no graph-shape
/// recognizer can tell the two apart reliably.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScopeUseRole {
    /// An ordinary value use.
    Value,
    /// The edge by which a region introduces one of its binders.
    BinderDeclaration,
    /// A use of an object some region introduces as a binder.
    BinderUse,
    /// A reference that occurs inside the target's own definition.
    DefinitionInternal,
}

/// One reference occurrence: an edge of the graph, placed.
#[invariant(*role != ScopeUseRole::BinderDeclaration || quantifier_variable_kind_is_allowed(target.object_kind()),
    "a binder declaration names a binder-capable object")]
#[invariant(*role != ScopeUseRole::BinderUse || quantifier_variable_kind_is_allowed(target.object_kind()),
    "a binder use names a binder-capable object")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeUseOccurrence {
    pub owner: SemanticObjectId,
    pub target: SemanticObjectId,
    pub role: ScopeUseRole,
    pub region: ScopeRegionId,
    pub source_order: SourceOrderKey,
}

/// The builder-recorded scope structure of one semantic graph.
#[invariant(regions.contains_key(root), "the root region exists")]
#[invariant(regions.get(root).is_some_and(|region| region.parent.is_none()),
    "the root region is parentless")]
#[expensive_invariant(scope_regions_form_one_tree(*root, regions),
    "regions form a single rooted tree with existing parents")]
#[expensive_invariant(regions.values().all(|region| region.parent.is_none_or(|parent| regions.contains_key(&parent))),
    "every parent link resolves")]
#[expensive_invariant(object_origins.values().all(|region| regions.contains_key(region)),
    "every origin names an existing region")]
#[expensive_invariant(uses.iter().all(|occurrence| regions.contains_key(&occurrence.region)),
    "every occurrence names an existing region")]
#[expensive_invariant(
    scope_source_orders_are_unique_per_region(regions, uses),
    "source-order keys are unique within an ordered region"
)]
// Binder-introduction uniqueness and binder-use enclosure are deliberately not
// invariants here. They are properties of the *record graph*, which today binds
// one variable object at two quantifiers (a `da` shared across connective
// branches) and lets a use escape its binder (`ro do` quantifying the audience
// deictic other utterances reference). Asserting them would force a shape the
// graph does not have; they are certified and reported instead — see
// [`scope_binder_introduction_conflicts`] and [`scope_unenclosed_binder_uses`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticScopeTree {
    pub root: ScopeRegionId,
    pub regions: BTreeMap<ScopeRegionId, ScopeRegion>,
    pub object_origins: BTreeMap<SemanticObjectId, ScopeRegionId>,
    pub uses: Vec<ScopeUseOccurrence>,
}

impl SemanticScopeTree {
    /// The region an object is introduced in, when the object is recorded.
    #[requires(true)]
    #[ensures(ret.is_some() == self.object_origins.contains_key(&object))]
    pub fn origin(&self, object: SemanticObjectId) -> Option<ScopeRegionId> {
        self.object_origins.get(&object).copied()
    }

    /// One region, when it exists.
    #[requires(true)]
    #[ensures(ret.is_some() == self.regions.contains_key(&region))]
    pub fn region(&self, region: ScopeRegionId) -> Option<&ScopeRegion> {
        self.regions.get(&region)
    }

    /// The region path from `region` up to and including the document root.
    ///
    /// The path is empty exactly when `region` is not a recorded region, so a
    /// caller never silently reads a truncated ancestry.
    #[requires(true)]
    #[ensures(ret.is_empty() != self.regions.contains_key(&region))]
    pub fn ancestry(&self, region: ScopeRegionId) -> Vec<ScopeRegionId> {
        let mut path = Vec::new();
        let mut current = Some(region);
        while let Some(next) = current {
            let Some(node) = self.regions.get(&next) else {
                break;
            };
            path.push(next);
            current = node.parent;
            if path.len() > self.regions.len() {
                break;
            }
        }
        path
    }

    /// The binders live on `region`'s ancestor path, innermost region first.
    ///
    /// This is the permitted binder universe of anything introduced there: the
    /// value the recorded `scopeDependence` of a constant must agree with.
    #[requires(true)]
    #[ensures(ret.is_empty() || self.regions.contains_key(&region))]
    pub fn binder_universe(&self, region: ScopeRegionId) -> BTreeSet<SemanticObjectId> {
        let mut binders = BTreeSet::new();
        for ancestor in self.ancestry(region) {
            if let Some(node) = self.regions.get(&ancestor) {
                binders.extend(node.binders.iter().copied());
            }
        }
        binders
    }

    /// Whether `region` is `ancestor` or lies below it.
    #[requires(true)]
    #[ensures(ret == self.ancestry(region).contains(&ancestor))]
    pub fn is_descendant_of(&self, region: ScopeRegionId, ancestor: ScopeRegionId) -> bool {
        self.ancestry(region).contains(&ancestor)
    }

    /// Every occurrence, grouped by owner, in recorded order.
    #[requires(true)]
    #[ensures(true)]
    pub fn occurrences_by_owner(&self) -> BTreeMap<SemanticObjectId, Vec<&ScopeUseOccurrence>> {
        let mut grouped: BTreeMap<SemanticObjectId, Vec<&ScopeUseOccurrence>> = BTreeMap::new();
        for occurrence in &self.uses {
            grouped
                .entry(occurrence.owner)
                .or_default()
                .push(occurrence);
        }
        grouped
    }

    /// Record where an object introduced outside the builder belongs.
    ///
    /// Origins are total over the object map, and nothing about a bare object
    /// recovers the region it belongs in — that is the whole reason the record
    /// exists — so a caller that adds an object has to say where it goes.
    #[requires(self.regions.contains_key(&region))]
    #[ensures(ret.origin(object) == Some(region))]
    pub fn with_origin(self, object: SemanticObjectId, region: ScopeRegionId) -> Self {
        let mut data = self.into_data();
        data.object_origins.insert(object, region);
        Self::from_data(data)
    }

    /// Re-place one owner's occurrences after its references changed.
    ///
    /// The occurrence table is exactly the graph's reference edges, in
    /// enumeration order, so editing an object's references invalidates that
    /// owner's rows. Each edge inherits the region its old row in the same
    /// position had, and its role too when that row named the same target; an
    /// edge with no counterpart is placed at the owner's origin as a plain
    /// value, which is the weakest claim the table can make about it.
    ///
    /// This is repair, not derivation: it cannot recover a region the edit
    /// should have introduced, so a caller that edits an object into a new
    /// binder has to say so with [`Self::with_origin`].
    #[requires(true)]
    #[ensures(true)]
    pub fn with_owner_reindexed(self, owner: SemanticObjectId, object: &SemanticObject) -> Self {
        let fallback = self.origin(owner).unwrap_or(self.root);
        let mut data = self.into_data();
        let mut previous = Vec::new();
        data.uses.retain(|occurrence| {
            let mine = occurrence.owner == owner;
            if mine {
                previous.push(*occurrence);
            }
            !mine
        });
        let mut references = Vec::new();
        object.references_into(&mut references);
        for (index, target) in references.into_iter().enumerate() {
            let old = previous.get(index);
            let inherited = old.filter(|occurrence| occurrence.target == target);
            data.uses.push(new!(ScopeUseOccurrence {
                owner: owner,
                target: target,
                role: inherited.map_or(ScopeUseRole::Value, |occurrence| occurrence.role),
                region: old.map_or(fallback, |occurrence| occurrence.region),
                source_order: SourceOrderKey::new(0),
            }));
        }
        // Keys only order loci already placed in one region, so recording them
        // afresh in table order preserves every ordering they claimed.
        let mut next: BTreeMap<ScopeRegionId, usize> = BTreeMap::new();
        data.uses = data
            .uses
            .into_iter()
            .map(|occurrence| {
                let slot = next.entry(occurrence.region).or_default();
                let renumbered = new!(ScopeUseOccurrence {
                    owner: occurrence.owner,
                    target: occurrence.target,
                    role: occurrence.role,
                    region: occurrence.region,
                    source_order: SourceOrderKey::new(*slot),
                });
                *slot += 1;
                renumbered
            })
            .collect();
        Self::from_data(data)
    }
}

/// Every region reaches the root through parent links without revisiting one.
#[requires(true)]
#[ensures(!ret || regions.contains_key(&root))]
pub fn scope_regions_form_one_tree(
    root: ScopeRegionId,
    regions: &BTreeMap<ScopeRegionId, ScopeRegion>,
) -> bool {
    if !regions
        .get(&root)
        .is_some_and(|region| region.parent.is_none())
    {
        return false;
    }
    regions.iter().all(|(id, region)| {
        if region.parent.is_none() {
            return *id == root;
        }
        let mut seen = BTreeSet::from([*id]);
        let mut current = region.parent;
        while let Some(next) = current {
            if !seen.insert(next) {
                return false;
            }
            match regions.get(&next) {
                None => return false,
                Some(node) => current = node.parent,
            }
        }
        seen.contains(&root)
    })
}

/// No object is introduced as a binder by two regions.
///
/// A certification rather than an invariant: the record model does bind one
/// variable object at two quantifiers when a quantified argument distributes
/// over connective branches, and a prenex `da` shared by connected statements
/// binds in each. Callers that need one binder to have one home must consult
/// [`scope_binder_introduction_conflicts`] and decide.
#[requires(true)]
#[ensures(ret == scope_binder_introduction_conflicts(regions).is_empty())]
pub fn scope_binder_introduction_is_unique(regions: &BTreeMap<ScopeRegionId, ScopeRegion>) -> bool {
    scope_binder_introduction_conflicts(regions).is_empty()
}

/// Every binder more than one region introduces, with the regions introducing it.
#[requires(true)]
#[ensures(ret.iter().all(|(_, regions)| regions.len() > 1))]
pub fn scope_binder_introduction_conflicts(
    regions: &BTreeMap<ScopeRegionId, ScopeRegion>,
) -> Vec<(SemanticObjectId, Vec<ScopeRegionId>)> {
    let mut introductions: BTreeMap<SemanticObjectId, Vec<ScopeRegionId>> = BTreeMap::new();
    for (id, region) in regions {
        for binder in &region.binders {
            introductions.entry(*binder).or_default().push(*id);
        }
    }
    introductions
        .into_iter()
        .filter(|(_, regions)| regions.len() > 1)
        .collect()
}

/// Every binder use lies in the subtree of the region introducing that binder.
///
/// This is the structural form of the host planner's binder-does-not-enclose-use
/// failure. It is a certification rather than an invariant because the record
/// model still admits the failure: `ro do` quantifies the audience deictic in
/// place, so every other utterance's reference to the audience is a use outside
/// the binder's region. [`scope_unenclosed_binder_uses`] names the witnesses.
#[requires(true)]
#[ensures(ret == (regions.contains_key(&root) && scope_unenclosed_binder_uses(regions, uses).is_empty()))]
pub fn scope_binder_uses_are_enclosed(
    root: ScopeRegionId,
    regions: &BTreeMap<ScopeRegionId, ScopeRegion>,
    uses: &[ScopeUseOccurrence],
) -> bool {
    regions.contains_key(&root) && scope_unenclosed_binder_uses(regions, uses).is_empty()
}

/// Every binder-use occurrence evaluated outside its binder's region.
#[requires(true)]
#[ensures(ret.iter().all(|occurrence| occurrence.role == ScopeUseRole::BinderUse))]
pub fn scope_unenclosed_binder_uses(
    regions: &BTreeMap<ScopeRegionId, ScopeRegion>,
    uses: &[ScopeUseOccurrence],
) -> Vec<ScopeUseOccurrence> {
    let introductions = regions
        .iter()
        .flat_map(|(id, region)| region.binders.iter().map(move |binder| (*binder, *id)))
        .collect::<BTreeMap<_, _>>();
    uses.iter()
        .filter(|occurrence| occurrence.role == ScopeUseRole::BinderUse)
        .filter(|occurrence| {
            !introductions
                .get(&occurrence.target)
                .is_some_and(|introduction| {
                    scope_region_is_descendant(regions, occurrence.region, *introduction)
                })
        })
        .copied()
        .collect()
}

/// Source-order keys separate the loci ordered within one region.
#[requires(true)]
#[ensures(true)]
pub fn scope_source_orders_are_unique_per_region(
    regions: &BTreeMap<ScopeRegionId, ScopeRegion>,
    uses: &[ScopeUseOccurrence],
) -> bool {
    let mut seen: BTreeMap<ScopeRegionId, BTreeSet<SourceOrderKey>> = BTreeMap::new();
    for occurrence in uses {
        if !seen
            .entry(occurrence.region)
            .or_default()
            .insert(occurrence.source_order)
        {
            return false;
        }
    }
    let mut sibling_orders: BTreeMap<Option<ScopeRegionId>, BTreeSet<SourceOrderKey>> =
        BTreeMap::new();
    regions.values().all(|region| {
        sibling_orders
            .entry(region.parent)
            .or_default()
            .insert(region.source_order)
    })
}

/// Whether `region` is `ancestor` or lies below it in the region tree.
#[requires(true)]
#[ensures(!ret || regions.contains_key(&region))]
pub(crate) fn scope_region_is_descendant(
    regions: &BTreeMap<ScopeRegionId, ScopeRegion>,
    region: ScopeRegionId,
    ancestor: ScopeRegionId,
) -> bool {
    let mut current = Some(region);
    let mut steps = 0;
    while let Some(next) = current {
        if next == ancestor {
            return regions.contains_key(&region);
        }
        let Some(node) = regions.get(&next) else {
            return false;
        };
        current = node.parent;
        steps += 1;
        if steps > regions.len() {
            return false;
        }
    }
    false
}

/// One reference edge of an object, with the structural locus it is evaluated
/// at.
///
/// The enumeration order is exactly [`SemanticObject::references_into`]'s, so a
/// site list and a reference list of the same object index the same edges. That
/// agreement is not left to inspection: it is an expensive invariant of
/// [`SemanticGraph`], because an occurrence table that silently drops an edge
/// would authorize a host the dropped use forbids.
#[requires(true)]
#[ensures(out.len() >= old(out.len()))]
pub(crate) fn reference_sites_into(
    object: &SemanticObject,
    out: &mut Vec<(SemanticObjectId, ScopeSite)>,
) {
    let mut scratch = Vec::new();
    match object.as_data() {
        data!(SemanticObject::Utterance(node)) => {
            push_sites(
                out,
                [node.speaker, node.audience, node.eventuality],
                site_object(),
            );
            push_optional_site(out, node.content, new!(ScopeSite::Content));
            push_sites(
                out,
                [node.deictic_ground.time, node.deictic_ground.place],
                site_object(),
            );
            push_sites(out, node.asides.iter().copied(), site_object());
        }
        data!(SemanticObject::Sequence(node)) => {
            for (index, item) in node.items.iter().enumerate() {
                out.push((*item, new!(ScopeSite::Item { index })));
            }
            push_optional_site(out, node.content, new!(ScopeSite::Content));
            push_sites(out, node.connection_claims.iter().copied(), site_object());
            for label in &node.ordinal_labels {
                label.references_into(&mut scratch);
            }
            if let Some(connection) = &node.nonlogical_connection {
                connection.references_into(&mut scratch);
            }
            drain_sites(out, &mut scratch, site_object());
            push_sites(
                out,
                node.bound_eventualities
                    .iter()
                    .map(|eventuality| eventuality.object_id()),
                site_object(),
            );
        }
        data!(SemanticObject::Eventuality(node)) => {
            push_optional_site(out, node.tense_modal, site_object());
            push_optional_site(out, node.content, new!(ScopeSite::Content));
            push_optional_site(out, node.body, new!(ScopeSite::Body));
            push_binder_sites(out, node.parameters.iter().copied());
            push_sites(out, node.embedded_questions.iter().copied(), site_object());
            push_optional_site(out, node.experiencer, site_object());
            push_optional_site(out, node.scale, site_object());
            push_optional_site(out, node.target, site_object());
            for adjunct in &node.adjuncts {
                adjunct.references_into(&mut scratch);
            }
            if let Some(time_interval) = &node.time_interval {
                time_interval.references_into(&mut scratch);
            }
            if let Some(time_span) = &node.time_span {
                time_span.references_into(&mut scratch);
            }
            if let Some(aspect) = &node.aspect {
                aspect.references_into(&mut scratch);
            }
            for aspect in &node.aspects {
                aspect.references_into(&mut scratch);
            }
            for recurrence in &node.recurrence {
                recurrence.references_into(&mut scratch);
            }
            for modifier in &node.interval_modifiers {
                modifier.references_into(&mut scratch);
            }
            if let Some(space_interval) = &node.space_interval {
                space_interval.references_into(&mut scratch);
            }
            if let Some(aspect) = &node.spatial_aspect {
                aspect.references_into(&mut scratch);
            }
            for aspect in &node.spatial_aspects {
                aspect.references_into(&mut scratch);
            }
            for recurrence in &node.spatial_recurrence {
                recurrence.references_into(&mut scratch);
            }
            for modifier in &node.spatial_interval_modifiers {
                modifier.references_into(&mut scratch);
            }
            if let Some(subscript) = &node.subscript {
                subscript.references_into(&mut scratch);
            }
            drain_sites(out, &mut scratch, site_object());
            push_referent_sites(
                out,
                &mut scratch,
                node.descriptor.as_ref(),
                node.composition.as_ref(),
                &node.relative_clauses,
            );
            if let Some(time) = &node.time {
                time.references_into(&mut scratch);
            }
            for step in &node.time_path {
                step.references_into(&mut scratch);
            }
            if let Some(space) = &node.space {
                space.references_into(&mut scratch);
            }
            for step in &node.space_path {
                step.references_into(&mut scratch);
            }
            drain_sites(out, &mut scratch, site_object());
        }
        data!(SemanticObject::Referent(node)) => {
            push_optional_site(out, node.body, new!(ScopeSite::Body));
            push_binder_sites(out, node.parameters.iter().copied());
            push_sites(out, node.embedded_questions.iter().copied(), site_object());
            push_optional_site(out, node.abstracted, site_object());
            push_optional_site(out, node.experiencer, site_object());
            push_optional_site(out, node.scale, site_object());
            push_optional_site(out, node.target, site_object());
            if let Some(deictic_reference) = node.deictic_reference {
                out.push((deictic_reference.ground, site_object()));
            }
            if let Some(membership) = &node.personal_mass_membership {
                membership.references_into(&mut scratch);
            }
            drain_sites(out, &mut scratch, site_object());
            push_referent_sites(
                out,
                &mut scratch,
                node.descriptor.as_ref(),
                node.composition.as_ref(),
                &node.relative_clauses,
            );
            if let Some(subscript) = &node.subscript {
                subscript.references_into(&mut scratch);
            }
            drain_sites(out, &mut scratch, site_object());
        }
        data!(SemanticObject::Parameter(node)) => {
            if let Some(subscript) = &node.subscript {
                subscript.references_into(&mut scratch);
            }
            drain_sites(out, &mut scratch, site_object());
        }
        data!(SemanticObject::Predication(node)) => {
            if let data!(PredicationRelation::Parameter { parameter }) = node.relation.as_data() {
                out.push((*parameter, site_object()));
            }
            push_optional_site(out, node.eventuality, site_object());
            if let Some(tanru) = &node.tanru_link {
                tanru.references_into(&mut scratch);
            }
            drain_sites(out, &mut scratch, site_object());
            for (place, argument) in &node.arguments {
                argument.references_into(&mut scratch);
                drain_sites(
                    out,
                    &mut scratch,
                    new!(ScopeSite::Argument { place: *place }),
                );
            }
            for question in &node.place_questions {
                question.references_into(&mut scratch);
            }
            for adjunct in &node.adjuncts {
                adjunct.references_into(&mut scratch);
            }
            for exchange in &node.reciprocity {
                exchange.references_into(&mut scratch);
            }
            if let Some(scalar) = &node.scalar_negation {
                scalar.references_into(&mut scratch);
            }
            drain_sites(out, &mut scratch, site_object());
            push_optional_site(out, node.relation_metadata, site_object());
        }
        data!(SemanticObject::Formula(node)) => push_formula_sites(out, &mut scratch, node),
        data!(SemanticObject::Sign(node)) => {
            let described = push_descriptor_sites(out, &mut scratch, node.descriptor.as_ref());
            if let Some(quotation) = &node.quotation {
                quotation.references_into(&mut scratch);
            }
            drain_sites(out, &mut scratch, new!(ScopeSite::Quotation));
            push_optional_site(out, node.denotes, new!(ScopeSite::Denotation));
            for (index, clause) in node.relative_clauses.iter().enumerate() {
                out.push((
                    clause.body,
                    new!(ScopeSite::RelativeClause {
                        index: described + index
                    }),
                ));
            }
            push_optional_site(out, node.target, site_object());
            if let Some(subscript) = &node.subscript {
                subscript.references_into(&mut scratch);
            }
            drain_sites(out, &mut scratch, site_object());
        }
        data!(SemanticObject::DisplayedContent(node)) => {
            push_sites(
                out,
                [node.experiencer, node.target, node.anchor],
                site_object(),
            );
        }
        data!(SemanticObject::MathExpression(node)) => {
            match node.kind.as_data() {
                data!(MathExpressionNodeKind::Literal { denotes, .. }) => {
                    push_optional_site(out, *denotes, site_object());
                }
                data!(MathExpressionNodeKind::Operator {
                    operands,
                    operator_denotes,
                    ..
                }) => {
                    push_sites(out, operands.iter().copied(), site_object());
                    push_optional_site(out, *operator_denotes, site_object());
                }
                data!(MathExpressionNodeKind::QuestionedOperator {
                    operator_parameter,
                    operands
                }) => {
                    out.push((*operator_parameter, site_object()));
                    push_sites(out, operands.iter().copied(), site_object());
                }
            }
            if let Some(scalar) = &node.scalar_negation {
                scalar.references_into(&mut scratch);
            }
            if let Some(subscript) = &node.subscript {
                subscript.references_into(&mut scratch);
            }
            drain_sites(out, &mut scratch, site_object());
        }
        data!(SemanticObject::Quantity(node)) => {
            node.value.references_into(&mut scratch);
            drain_sites(out, &mut scratch, site_object());
            push_optional_site(out, node.comparison_set, site_object());
        }
        data!(SemanticObject::RelationMetadata(node)) => {
            if let Some(expansion) = &node.expansion {
                expansion.references_into(&mut scratch);
            }
            drain_sites(out, &mut scratch, site_object());
        }
        data!(SemanticObject::Question(node)) => {
            push_sites(out, [node.asker, node.respondent], site_object());
            out.push((node.body, new!(ScopeSite::Body)));
            push_binder_sites(out, node.slots.iter().filter_map(QuestionSlot::parameter));
            push_optional_site(out, node.focus, new!(ScopeSite::Focus));
            push_optional_site(out, node.presupposed_answer, new!(ScopeSite::Focus));
        }
    }
}

#[requires(true)]
#[ensures(matches!(ret.as_data(), data!(ScopeSite::Object)))]
fn site_object() -> ScopeSite {
    new!(ScopeSite::Object)
}

#[requires(true)]
#[ensures(out.len() >= old(out.len()))]
fn push_sites(
    out: &mut Vec<(SemanticObjectId, ScopeSite)>,
    references: impl IntoIterator<Item = SemanticObjectId>,
    site: ScopeSite,
) {
    out.extend(references.into_iter().map(|target| (target, site)));
}

/// Binder declarations are indexed so each names its own introducing locus.
#[requires(true)]
#[ensures(out.len() >= old(out.len()))]
fn push_binder_sites(
    out: &mut Vec<(SemanticObjectId, ScopeSite)>,
    binders: impl IntoIterator<Item = SemanticObjectId>,
) {
    for (index, binder) in binders.into_iter().enumerate() {
        out.push((binder, new!(ScopeSite::Binder { index })));
    }
}

#[requires(true)]
#[ensures(out.len() == old(out.len()) + usize::from(reference.is_some()))]
fn push_optional_site(
    out: &mut Vec<(SemanticObjectId, ScopeSite)>,
    reference: Option<SemanticObjectId>,
    site: ScopeSite,
) {
    if let Some(reference) = reference {
        out.push((reference, site));
    }
}

/// Move everything a nested value collector produced to one locus.
#[requires(true)]
#[ensures(scratch.is_empty())]
fn drain_sites(
    out: &mut Vec<(SemanticObjectId, ScopeSite)>,
    scratch: &mut Vec<SemanticObjectId>,
    site: ScopeSite,
) {
    out.extend(scratch.drain(..).map(|target| (target, site)));
}

/// Mirrors `collect_referent_references`.
///
/// A description's property and every relative clause restricting it are their
/// own regions: they are lambdas over the described referent, and a reference
/// evaluated inside one is not evaluated where the description sits. Clause
/// ordinals run through the descriptor's clauses first and then the node's, so
/// one referent never gives two clauses the same locus.
#[requires(true)]
#[ensures(scratch.is_empty())]
fn push_referent_sites(
    out: &mut Vec<(SemanticObjectId, ScopeSite)>,
    scratch: &mut Vec<SemanticObjectId>,
    descriptor: Option<&Descriptor>,
    composition: Option<&Composition>,
    relative_clauses: &[RelativeClause],
) {
    let described = push_descriptor_sites(out, scratch, descriptor);
    if let Some(composition) = composition {
        scratch.extend(composition.members.iter().copied());
        scratch.extend(composition.excluded_members.iter().copied());
        extend_optional(scratch, composition.operator_parameter);
    }
    drain_sites(out, scratch, site_object());
    for (index, clause) in relative_clauses.iter().enumerate() {
        out.push((
            clause.body,
            new!(ScopeSite::RelativeClause {
                index: described + index
            }),
        ));
    }
}

/// Mirrors `Descriptor::references_into`, returning the clause count it placed.
#[requires(true)]
#[ensures(scratch.is_empty())]
#[ensures(ret == descriptor.map_or(0, |descriptor| descriptor.relative_clauses.len()))]
fn push_descriptor_sites(
    out: &mut Vec<(SemanticObjectId, ScopeSite)>,
    scratch: &mut Vec<SemanticObjectId>,
    descriptor: Option<&Descriptor>,
) -> usize {
    let Some(descriptor) = descriptor else {
        drain_sites(out, scratch, site_object());
        return 0;
    };
    push_optional_site(out, descriptor.speaker, site_object());
    push_optional_site(out, descriptor.body, new!(ScopeSite::Property));
    for (index, clause) in descriptor.relative_clauses.iter().enumerate() {
        out.push((clause.body, new!(ScopeSite::RelativeClause { index })));
    }
    push_optional_site(out, descriptor.quantity, site_object());
    push_optional_site(out, descriptor.scale, site_object());
    push_optional_site(out, descriptor.operand, site_object());
    drain_sites(out, scratch, site_object());
    descriptor.relative_clauses.len()
}

/// Mirrors `collect_formula_references`.
///
/// Each binding's restriction names its own ordinal so the sites stay
/// distinguishable; whether those ordinals nest or share one locus is the
/// recorder's decision, not the enumeration's.
#[requires(true)]
#[ensures(scratch.is_empty())]
fn push_formula_sites(
    out: &mut Vec<(SemanticObjectId, ScopeSite)>,
    scratch: &mut Vec<SemanticObjectId>,
    node: &FormulaNode,
) {
    match node.as_data() {
        data!(FormulaNode::Atom(node)) => out.push((node.predication, site_object())),
        data!(FormulaNode::Connective(node)) => {
            push_optional_site(out, node.eventuality, site_object());
            for (index, child) in node.children.iter().enumerate() {
                out.push((*child, new!(ScopeSite::Operand { index })));
            }
            if let Some(connector) = &node.connector {
                connector.references_into(scratch);
            }
            drain_sites(out, scratch, site_object());
        }
        data!(FormulaNode::Quantified(node)) => {
            out.push((node.variable, new!(ScopeSite::Binder { index: 0 })));
            push_optional_site(out, node.source_variable, site_object());
            if let Some(selection) = &node.selection_source {
                selection.references_into(scratch);
            }
            drain_sites(out, scratch, site_object());
            push_optional_site(
                out,
                node.restriction,
                new!(ScopeSite::Restriction { binding: 0 }),
            );
            out.push((node.body, new!(ScopeSite::Body)));
            push_optional_site(out, node.quantity, site_object());
        }
        data!(FormulaNode::QuantifierBundle(node)) => {
            for (binding, entry) in node.bindings.iter().enumerate() {
                out.push((entry.variable, new!(ScopeSite::Binder { index: binding })));
                push_optional_site(out, entry.source_variable, site_object());
                if let Some(selection) = &entry.selection_source {
                    selection.references_into(scratch);
                }
                drain_sites(out, scratch, site_object());
                push_optional_site(
                    out,
                    entry.restriction,
                    new!(ScopeSite::Restriction { binding }),
                );
                push_optional_site(out, entry.quantity, site_object());
            }
            out.push((node.body, new!(ScopeSite::Body)));
        }
        data!(FormulaNode::RespectivelyDistribution(node)) => {
            out.push((node.body, new!(ScopeSite::Body)));
            for (binding, stream) in node.streams.iter().enumerate() {
                out.push((stream.slot, new!(ScopeSite::Binder { index: binding })));
                push_sites(out, stream.items.iter().copied(), site_object());
                push_optional_site(
                    out,
                    stream.restriction,
                    new!(ScopeSite::Restriction { binding }),
                );
                push_optional_site(out, stream.quantity, site_object());
            }
        }
    }
    push_sites(
        out,
        match node.as_data() {
            data!(FormulaNode::Atom(node)) => &node.bound_eventualities,
            data!(FormulaNode::Connective(node)) => &node.bound_eventualities,
            data!(FormulaNode::Quantified(node)) => &node.bound_eventualities,
            data!(FormulaNode::QuantifierBundle(node)) => &node.bound_eventualities,
            data!(FormulaNode::RespectivelyDistribution(node)) => &node.bound_eventualities,
        }
        .iter()
        .map(|eventuality| eventuality.object_id()),
        site_object(),
    );
}

/// Site enumeration and reference enumeration index the same edges.
#[requires(true)]
#[ensures(true)]
pub fn semantic_reference_sites_match_references(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    let mut references = Vec::new();
    let mut sites = Vec::new();
    objects.values().all(|object| {
        references.clear();
        sites.clear();
        object.references_into(&mut references);
        reference_sites_into(object, &mut sites);
        sites.len() == references.len()
            && sites
                .iter()
                .zip(references.iter())
                .all(|((target, _), reference)| target == reference)
    })
}

/// The occurrence table is exactly the graph's reference edges.
///
/// Equality, not containment: an occurrence the table omits would let a later
/// host planner place a definition where the omitted use is not covered, and a
/// spurious occurrence would forbid a legal host. Order is compared too, so an
/// occurrence cannot borrow a sibling edge's region.
#[requires(true)]
#[ensures(true)]
pub fn semantic_scope_occurrences_match_references(
    scope: &SemanticScopeTree,
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    let grouped = scope.occurrences_by_owner();
    if grouped.keys().any(|owner| !objects.contains_key(owner)) {
        return false;
    }
    let mut references = Vec::new();
    objects.iter().all(|(id, object)| {
        references.clear();
        object.references_into(&mut references);
        let empty = Vec::new();
        let occurrences = grouped.get(id).unwrap_or(&empty);
        occurrences.len() == references.len()
            && occurrences
                .iter()
                .zip(references.iter())
                .all(|(occurrence, reference)| occurrence.target == *reference)
    })
}

/// Every surviving object records the region it was introduced in.
#[requires(objects.contains_key(&root))]
#[ensures(!ret || scope.origin(root).is_some())]
pub fn semantic_scope_origins_are_total(
    root: SemanticObjectId,
    scope: &SemanticScopeTree,
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    let _ = root;
    objects.keys().all(|id| scope.origin(*id).is_some())
}

/// The cross-certification: recorded dependence against recorded structure.
///
/// A constant's `scopeDependence` is derived by an independent traversal of the
/// object graph, so every binder that derivation attributes must lie on the
/// constant's origin path in the region tree — restricted to binders the
/// *graph itself declares*, since a binder only the builder knows is invisible
/// to the derivation and cannot be held against it.
///
/// This is containment, not equality, and the direction is the load-bearing
/// one: the region tree may never attribute *fewer* binders than the
/// derivation, because a host placed outside a binder the value depends on is
/// exactly the illegal hoist this model exists to prevent. The converse gap is
/// real but benign — the derivation blesses a shared object at the first path
/// its traversal happens to take, which can be shallower than where the object
/// is actually written. See [`semantic_scope_dependence_divergences`], which
/// reports those cases as witnesses rather than hiding them.
#[requires(true)]
#[ensures(true)]
pub fn semantic_scope_dependences_agree_with_regions(
    scope: &SemanticScopeTree,
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> bool {
    semantic_scope_dependence_divergences(scope, objects)
        .into_iter()
        .all(|(_, derived, structural)| derived.is_subset(&structural))
}

/// Every constant whose derived dependence differs from its recorded origin
/// path, as `(constant, derived universe, structural universe)`.
///
/// Corpus-wide certification of the new structure against the existing
/// derivation is the deliverable of this step as much as the data is, so the
/// disagreements are enumerable rather than merely counted.
#[requires(true)]
#[ensures(true)]
pub fn semantic_scope_dependence_divergences(
    scope: &SemanticScopeTree,
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> Vec<(
    SemanticObjectId,
    BTreeSet<SemanticObjectId>,
    BTreeSet<SemanticObjectId>,
)> {
    let declared = semantic_graph_declared_binders(objects);
    let mut divergences = Vec::new();
    for (id, object) in objects {
        let Some(dependence) = object.scope_dependence() else {
            continue;
        };
        let derived = dependence
            .may_depend_on()
            .cloned()
            .unwrap_or_else(BTreeSet::new);
        let structural = scope
            .origin(*id)
            .map(|origin| {
                scope
                    .binder_universe(origin)
                    .into_iter()
                    .filter(|binder| declared.contains(binder))
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        if derived != structural {
            divergences.push((*id, derived, structural));
        }
    }
    divergences
}

/// Every generated event binder the graph declares, with the nodes declaring it.
///
/// Generated event binders are not region binders and cannot become ones:
/// [`ScopeRegion`] admits only referent and parameter objects, because a
/// generated eventuality is bound by the formula or sequence node carrying it
/// rather than by a lexical multiplicity region. The declaration is nonetheless
/// explicit typed model data — the `boundEventualities` list — so a consumer
/// that needs binder ownership reads it here instead of reconstructing
/// ownership from reference edges.
#[requires(true)]
#[ensures(ret.values().all(|owners| !owners.is_empty()))]
pub fn semantic_graph_generated_event_binders(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> {
    let mut declared: BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>> = BTreeMap::new();
    for (owner, object) in objects {
        let bound = match object.as_data() {
            data!(SemanticObject::Sequence(node)) => &node.bound_eventualities,
            data!(SemanticObject::Formula(node)) => match node.as_data() {
                data!(FormulaNode::Atom(node)) => &node.bound_eventualities,
                data!(FormulaNode::Connective(node)) => &node.bound_eventualities,
                data!(FormulaNode::Quantified(node)) => &node.bound_eventualities,
                data!(FormulaNode::QuantifierBundle(node)) => &node.bound_eventualities,
                data!(FormulaNode::RespectivelyDistribution(node)) => &node.bound_eventualities,
            },
            _ => continue,
        };
        for eventuality in bound {
            declared
                .entry(eventuality.object_id())
                .or_default()
                .insert(*owner);
        }
    }
    declared
}

/// Every object some node declares as a binder.
///
/// This is what the scope-dependence derivation can see, and therefore the part
/// of the region tree's binder record it can be held to.
#[requires(true)]
#[ensures(ret.iter().all(|binder| quantifier_variable_kind_is_allowed(binder.object_kind())))]
pub fn semantic_graph_declared_binders(
    objects: &BTreeMap<SemanticObjectId, SemanticObject>,
) -> BTreeSet<SemanticObjectId> {
    let mut declared = BTreeSet::new();
    for object in objects.values() {
        match object.as_data() {
            data!(SemanticObject::Eventuality(node)) => declared.extend(node.parameters.iter()),
            data!(SemanticObject::Referent(node)) => declared.extend(node.parameters.iter()),
            data!(SemanticObject::Question(node)) => {
                declared.extend(node.slots.iter().filter_map(QuestionSlot::parameter));
            }
            data!(SemanticObject::Formula(node)) => match node.as_data() {
                data!(FormulaNode::Quantified(node)) => {
                    declared.insert(node.variable);
                }
                data!(FormulaNode::QuantifierBundle(node)) => {
                    declared.extend(node.bindings.iter().map(|binding| binding.variable));
                }
                data!(FormulaNode::RespectivelyDistribution(node)) => {
                    declared.extend(node.streams.iter().map(|stream| stream.slot));
                }
                _ => {}
            },
            _ => {}
        }
    }
    declared
}
