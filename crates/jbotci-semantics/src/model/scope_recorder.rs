//! Builder-side recording of the structural scope model.
//!
//! The recorder has two jobs. While the builder walks syntax it keeps a region
//! stack, so every object is stamped with the region it is *introduced* in —
//! the one fact the finished object graph genuinely cannot recover, because the
//! graph is a DAG of shared identities and any traversal of it can only report
//! where a shared object was first *reached*.
//!
//! The builder does not assemble every scope-bearing construct in walk order:
//! quantifiers in particular are built by wrapping an already-finished body
//! formula (see `wrap_formula_with_generated_implicit_existential_ordered`), so
//! a push/pop around quantifier construction is not available. The regions those
//! constructs introduce are therefore declared by [`ScopeRecorder::finalize`],
//! one pass the recorder itself runs over the finished objects, which refines
//! the walk-recorded skeleton downwards: an object's home may move deeper into
//! the structure the walk already placed it in, and never sideways or outwards.
//! Regions the walk did record are reused by their owner locus rather than
//! duplicated.

use super::*;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};

/// One region under construction. Regions are addressed by their index here and
/// renumbered into [`ScopeRegionId`]s once the tree is final.
#[invariant(true)]
#[derive(Debug, Clone)]
struct ScopeRegionRecord {
    parent: Option<usize>,
    owner: Option<ScopeOwner>,
    boundary: ScopeBoundary,
    binders: Vec<SemanticObjectId>,
    source_order: SourceOrderKey,
}

/// One recorded reference occurrence, before region renumbering.
#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct ScopeOccurrenceRecord {
    owner: SemanticObjectId,
    target: SemanticObjectId,
    site: ScopeSite,
    region: usize,
}

/// The builder's scope recorder.
#[invariant(true)]
#[derive(Debug)]
pub(crate) struct ScopeRecorder {
    records: Vec<ScopeRegionRecord>,
    stack: Vec<usize>,
    origins: BTreeMap<SemanticObjectId, usize>,
    owned_loci: BTreeMap<(SemanticObjectId, ScopeSite), usize>,
    next_order: usize,
}

const DOCUMENT_REGION: usize = 0;

impl ScopeRecorder {
    #[requires(true)]
    #[ensures(ret.current() == DOCUMENT_REGION)]
    pub(crate) fn new() -> Self {
        let mut recorder = ScopeRecorder {
            records: Vec::new(),
            stack: Vec::new(),
            origins: BTreeMap::new(),
            owned_loci: BTreeMap::new(),
            next_order: 0,
        };
        let order = recorder.take_order();
        recorder.records.push(ScopeRegionRecord {
            parent: None,
            owner: Some(ScopeOwner::document()),
            boundary: new!(ScopeBoundary::Document),
            binders: Vec::new(),
            source_order: order,
        });
        recorder.stack.push(DOCUMENT_REGION);
        recorder
    }

    /// The region the builder is currently constructing objects in.
    #[requires(true)]
    #[ensures(ret < self.records.len())]
    pub(crate) fn current(&self) -> usize {
        *self
            .stack
            .last()
            .expect("the document region is never popped")
    }

    /// Open a child region and make it current.
    #[requires(true)]
    #[ensures(self.current() == ret)]
    pub(crate) fn enter(&mut self, boundary: ScopeBoundary) -> usize {
        let parent = self.current();
        let order = self.take_order();
        let region = self.records.len();
        self.records.push(ScopeRegionRecord {
            parent: Some(parent),
            owner: None,
            boundary,
            binders: Vec::new(),
            source_order: order,
        });
        self.stack.push(region);
        region
    }

    /// Close `region`, which must be the region the matching `enter` opened.
    ///
    /// Closing is tolerant of an already-unwound stack so an error path that
    /// abandons a construct cannot desynchronize the recorder.
    #[requires(true)]
    #[ensures(!self.stack.contains(&region))]
    pub(crate) fn leave(&mut self, region: usize) {
        while let Some(top) = self.stack.last().copied() {
            if self.stack.len() == 1 {
                break;
            }
            self.stack.pop();
            if top == region {
                break;
            }
        }
    }

    /// Name the locus a walk-recorded region belongs to.
    ///
    /// The owning object usually only exists once its children are built, so
    /// the locus is attached on the way out. A region left unattached — an
    /// abandoned construct, or a nested lowering that opened a second region
    /// for a locus that already has one — is spliced out during finalization,
    /// so one locus always names one region.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn attach_owner(&mut self, region: usize, owner: ScopeOwner) {
        let Some(object) = owner.object else {
            return;
        };
        let key = (object, owner.site);
        if let Some(existing) = self.owned_loci.get(&key).copied() {
            // A nested lowering closes first, so the region that already claimed
            // the locus is the inner one. The outer region is the locus's real
            // extent: hand ownership up and let the inner one be spliced out.
            if existing == region || !self.region_encloses(region, existing) {
                return;
            }
            if let Some(inner) = self.records.get_mut(existing) {
                inner.owner = None;
            }
        }
        if let Some(record) = self.records.get_mut(region) {
            record.owner = Some(owner);
            self.owned_loci.insert(key, region);
        }
    }

    /// Whether `region` is a strict ancestor of `inner`.
    #[requires(true)]
    #[ensures(!ret || region != inner)]
    fn region_encloses(&self, region: usize, inner: usize) -> bool {
        let mut current = self.records.get(inner).and_then(|record| record.parent);
        let mut steps = 0;
        while let Some(next) = current {
            if next == region {
                return true;
            }
            current = self.records.get(next).and_then(|record| record.parent);
            steps += 1;
            if steps > self.records.len() {
                return false;
            }
        }
        false
    }

    /// Stamp an object with the region it is introduced in.
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn record_origin(&mut self, object: SemanticObjectId) {
        let region = self.current();
        self.origins.entry(object).or_insert(region);
    }

    #[requires(true)]
    #[ensures(self.next_order == old(self.next_order) + 1)]
    fn take_order(&mut self) -> SourceOrderKey {
        let key = SourceOrderKey::new(self.next_order);
        self.next_order += 1;
        key
    }

    /// Complete the region tree over the finished objects and materialize the
    /// occurrence table.
    #[requires(objects.contains_key(&root))]
    #[ensures(ret.object_origins.len() <= objects.len())]
    pub(crate) fn finalize(
        mut self,
        root: SemanticObjectId,
        objects: &BTreeMap<SemanticObjectId, SemanticObject>,
    ) -> SemanticScopeTree {
        self.compact_skeleton(objects);
        let mut references = Vec::new();
        let mut reference_counts: BTreeMap<SemanticObjectId, usize> = BTreeMap::new();
        for object in objects.values() {
            references.clear();
            object.references_into(&mut references);
            for reference in &references {
                *reference_counts.entry(*reference).or_default() += 1;
            }
        }
        let mut pass = ScopeFinalization {
            recorder: &mut self,
            objects,
            reference_counts,
            by_locus: BTreeMap::new(),
            home: BTreeMap::new(),
            visited: BTreeSet::new(),
            occurrences: Vec::new(),
        };
        pass.index_skeleton();
        pass.visit(root, DOCUMENT_REGION);
        for id in objects.keys().copied() {
            pass.visit(id, DOCUMENT_REGION);
        }
        let home = std::mem::take(&mut pass.home);
        let occurrences = std::mem::take(&mut pass.occurrences);
        self.emit(objects, home, occurrences)
    }

    /// Drop skeleton regions whose owner never materialized or was pruned,
    /// re-parenting their children and origins onto the nearest survivor.
    #[requires(true)]
    #[ensures(true)]
    fn compact_skeleton(&mut self, objects: &BTreeMap<SemanticObjectId, SemanticObject>) {
        let kept = (0..self.records.len())
            .map(|index| {
                index == DOCUMENT_REGION
                    || self.records[index]
                        .owner
                        .and_then(|owner| owner.object)
                        .is_some_and(|object| objects.contains_key(&object))
            })
            .collect::<Vec<_>>();
        let mut survivor = vec![DOCUMENT_REGION; self.records.len()];
        for index in 0..self.records.len() {
            let mut current = index;
            let mut steps = 0;
            while !kept[current] {
                match self.records[current].parent {
                    Some(parent) if steps <= self.records.len() => {
                        current = parent;
                        steps += 1;
                    }
                    _ => {
                        current = DOCUMENT_REGION;
                        break;
                    }
                }
            }
            survivor[index] = current;
        }
        for index in 0..self.records.len() {
            if let Some(parent) = self.records[index].parent {
                self.records[index].parent = Some(survivor[parent]);
            }
        }
        for region in self.origins.values_mut() {
            *region = survivor[*region];
        }
        for (index, keep) in kept.iter().enumerate() {
            if !keep {
                self.records[index].owner = None;
            }
        }
    }

    /// Renumber the surviving regions in pre-order and build the model value.
    #[requires(true)]
    #[ensures(ret.object_origins.len() <= objects.len())]
    fn emit(
        self,
        objects: &BTreeMap<SemanticObjectId, SemanticObject>,
        home: BTreeMap<SemanticObjectId, usize>,
        occurrences: Vec<ScopeOccurrenceRecord>,
    ) -> SemanticScopeTree {
        let mut children: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        let mut reachable = BTreeSet::from([DOCUMENT_REGION]);
        for (index, record) in self.records.iter().enumerate() {
            if index == DOCUMENT_REGION || record.owner.is_none() {
                continue;
            }
            if let Some(parent) = record.parent {
                children.entry(parent).or_default().push(index);
                reachable.insert(index);
            }
        }
        for list in children.values_mut() {
            list.sort_by_key(|index| self.records[*index].source_order);
        }
        let mut numbering = BTreeMap::new();
        let mut order = Vec::new();
        let mut stack = vec![DOCUMENT_REGION];
        while let Some(index) = stack.pop() {
            if !reachable.contains(&index) || numbering.contains_key(&index) {
                continue;
            }
            numbering.insert(index, ScopeRegionId::new(numbering.len() + 1));
            order.push(index);
            if let Some(list) = children.get(&index) {
                stack.extend(list.iter().rev().copied());
            }
        }

        let mut regions = BTreeMap::new();
        for index in order {
            let record = &self.records[index];
            let id = numbering[&index];
            let mut binders = record.binders.clone();
            binders.retain(|binder| objects.contains_key(binder));
            regions.insert(
                id,
                new!(ScopeRegion {
                    parent: record
                        .parent
                        .and_then(|parent| numbering.get(&parent).copied()),
                    owner: record.owner.expect("unowned regions are unreachable"),
                    boundary: record.boundary.clone(),
                    binders,
                    source_order: record.source_order,
                }),
            );
        }

        let resolve = |index: usize| {
            numbering
                .get(&index)
                .copied()
                .unwrap_or_else(|| ScopeRegionId::new(1))
        };
        let object_origins = home
            .into_iter()
            .filter(|(object, _)| objects.contains_key(object))
            .map(|(object, region)| (object, resolve(region)))
            .collect::<BTreeMap<_, _>>();

        let binder_regions = regions
            .iter()
            .flat_map(|(id, region)| region.binders.iter().map(move |binder| (*binder, *id)))
            .collect::<BTreeMap<_, _>>();
        let definition_regions = regions
            .iter()
            .filter_map(|(id, region)| {
                let object = region.owner.object?;
                matches!(
                    region.owner.site.as_data(),
                    data!(ScopeSite::Body)
                        | data!(ScopeSite::Content)
                        | data!(ScopeSite::RelativeClause { .. })
                )
                .then_some((*id, object))
            })
            .collect::<Vec<_>>();

        let mut orders: BTreeMap<ScopeRegionId, usize> = BTreeMap::new();
        let mut uses = Vec::new();
        for occurrence in occurrences {
            if !objects.contains_key(&occurrence.owner) || !objects.contains_key(&occurrence.target)
            {
                continue;
            }
            let region = resolve(occurrence.region);
            let role = if matches!(occurrence.site.as_data(), data!(ScopeSite::Binder { .. })) {
                ScopeUseRole::BinderDeclaration
            } else if binder_regions.contains_key(&occurrence.target) {
                ScopeUseRole::BinderUse
            } else if definition_regions.iter().any(|(definition, defined)| {
                *defined == occurrence.target && region_is_descendant(&regions, region, *definition)
            }) {
                ScopeUseRole::DefinitionInternal
            } else {
                ScopeUseRole::Value
            };
            let slot = orders.entry(region).or_default();
            let source_order = SourceOrderKey::new(*slot);
            *slot += 1;
            uses.push(ScopeUseOccurrence {
                owner: occurrence.owner,
                target: occurrence.target,
                role,
                region,
                source_order,
            });
        }

        new!(SemanticScopeTree {
            root: ScopeRegionId::new(1),
            regions,
            object_origins,
            uses,
        })
    }
}

#[requires(true)]
#[ensures(!ret || regions.contains_key(&region))]
fn region_is_descendant(
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

/// Whether a locus is evaluated under the binders its owner introduces.
#[requires(true)]
#[ensures(true)]
fn scope_site_is_scoped(site: ScopeSite) -> bool {
    matches!(
        site.as_data(),
        data!(ScopeSite::Body) | data!(ScopeSite::Restriction { .. })
    )
}

/// The structural completion pass.
#[invariant(true)]
struct ScopeFinalization<'a> {
    recorder: &'a mut ScopeRecorder,
    objects: &'a BTreeMap<SemanticObjectId, SemanticObject>,
    reference_counts: BTreeMap<SemanticObjectId, usize>,
    by_locus: BTreeMap<(SemanticObjectId, ScopeSite), usize>,
    home: BTreeMap<SemanticObjectId, usize>,
    visited: BTreeSet<SemanticObjectId>,
    occurrences: Vec<ScopeOccurrenceRecord>,
}

impl ScopeFinalization<'_> {
    /// Make walk-recorded regions findable by the locus they were attached to.
    #[requires(true)]
    #[ensures(true)]
    fn index_skeleton(&mut self) {
        for (index, record) in self.recorder.records.iter().enumerate() {
            if let Some(owner) = record.owner
                && let Some(object) = owner.object
            {
                self.by_locus.entry((object, owner.site)).or_insert(index);
            }
        }
    }

    /// Descend into `id`, reached by a reference evaluated in `region`.
    #[requires(true)]
    #[ensures(true)]
    fn visit(&mut self, id: SemanticObjectId, region: usize) {
        if !self.visited.insert(id) {
            return;
        }
        let Some(object) = self.objects.get(&id) else {
            return;
        };
        let home = self.refine(id, region);
        self.home.insert(id, home);
        self.declare_site_regions(id, object, home);
        let mut sites = Vec::new();
        reference_sites_into(object, &mut sites);
        let mut placed = Vec::with_capacity(sites.len());
        for (target, site) in sites {
            let region = self.by_locus.get(&(id, site)).copied().unwrap_or(home);
            self.occurrences.push(ScopeOccurrenceRecord {
                owner: id,
                target,
                site,
                region,
            });
            placed.push((target, site, region));
        }
        // Occurrences are recorded in canonical reference order, but descent
        // enters the scoped positions last. A binder's own region is the
        // innermost home for anything reachable only through it, and an object
        // both a lambda body and an embedded question reach must be placed by
        // the binder that actually scopes it, not by whichever field the
        // reference enumeration happens to list first.
        for scoped in [false, true] {
            for (target, site, region) in &placed {
                if scope_site_is_scoped(*site) == scoped {
                    self.visit(*target, *region);
                }
            }
        }
    }

    /// The introduction region.
    ///
    /// The introduction region.
    ///
    /// The two records disagree in exactly one interesting way. The walk knows
    /// which *segment* — which performed act, which quotation, or the document
    /// itself — a value was written in, and no traversal of the finished graph
    /// can recover that for a value two acts share: it can only report which
    /// act happened to reach it first. Inside one segment the opposite holds:
    /// the builder assembles quantifiers, questions and connectives around
    /// already-finished operands, so its stamp predates the construct that
    /// encloses them and the structure is the better witness.
    ///
    /// So the walk decides the segment and the structure decides the position
    /// within it.
    #[requires(true)]
    #[ensures(true)]
    fn refine(&self, id: SemanticObjectId, region: usize) -> usize {
        match self.recorder.origins.get(&id).copied() {
            Some(recorded) if self.segment(recorded) != self.segment(region) => recorded,
            _ => region,
        }
    }

    /// The nearest enclosing segment: a performed act, an opaque mention, or
    /// the document.
    #[requires(true)]
    #[ensures(true)]
    fn segment(&self, region: usize) -> usize {
        let mut current = region;
        let mut steps = 0;
        loop {
            let Some(record) = self.recorder.records.get(current) else {
                return DOCUMENT_REGION;
            };
            if matches!(
                record.boundary.as_data(),
                data!(ScopeBoundary::Document)
                    | data!(ScopeBoundary::ForceSegment)
                    | data!(ScopeBoundary::Opaque)
            ) {
                return current;
            }
            match record.parent {
                Some(parent) if steps <= self.recorder.records.len() => {
                    current = parent;
                    steps += 1;
                }
                _ => return DOCUMENT_REGION,
            }
        }
    }

    #[requires(true)]
    #[ensures(!ret || region < self.recorder.records.len())]
    fn is_descendant(&self, region: usize, ancestor: usize) -> bool {
        let mut current = Some(region);
        let mut steps = 0;
        while let Some(next) = current {
            if next == ancestor {
                return region < self.recorder.records.len();
            }
            let Some(record) = self.recorder.records.get(next) else {
                return false;
            };
            current = record.parent;
            steps += 1;
            if steps > self.recorder.records.len() {
                return false;
            }
        }
        false
    }

    /// Find or create the region one locus of `object` is evaluated in.
    #[requires(true)]
    #[ensures(true)]
    fn region_for(
        &mut self,
        object: SemanticObjectId,
        site: ScopeSite,
        parent: usize,
        boundary: ScopeBoundary,
    ) -> usize {
        if let Some(existing) = self.by_locus.get(&(object, site)).copied() {
            // A walk-recorded region keeps its recorded context, but may be
            // pushed deeper when the structure places it inside that context.
            if self.is_descendant(parent, existing) {
                return existing;
            }
            if let Some(recorded_parent) = self.recorder.records[existing].parent
                && self.is_descendant(parent, recorded_parent)
                && !self.is_descendant(parent, existing)
            {
                self.recorder.records[existing].parent = Some(parent);
            }
            return existing;
        }
        let order = self.recorder.take_order();
        let region = self.recorder.records.len();
        self.recorder.records.push(ScopeRegionRecord {
            parent: Some(parent),
            owner: Some(ScopeOwner::locus(object, site)),
            boundary,
            binders: Vec::new(),
            source_order: order,
        });
        self.by_locus.insert((object, site), region);
        region
    }

    #[requires(true)]
    #[ensures(true)]
    fn alias_site(&mut self, object: SemanticObjectId, site: ScopeSite, region: usize) {
        self.by_locus.insert((object, site), region);
    }

    #[requires(true)]
    #[ensures(true)]
    fn set_binders(&mut self, region: usize, binders: Vec<SemanticObjectId>) {
        let mut unique = Vec::new();
        for binder in binders {
            if !unique.contains(&binder) {
                unique.push(binder);
            }
        }
        self.recorder.records[region].binders = unique;
    }

    /// Declare the regions the shape of one object introduces.
    #[requires(true)]
    #[ensures(true)]
    fn declare_site_regions(&mut self, id: SemanticObjectId, object: &SemanticObject, home: usize) {
        match object.as_data() {
            data!(SemanticObject::Utterance(node)) => {
                if node.content.is_some() {
                    let boundary = match node.force {
                        UtteranceForce::Assert
                        | UtteranceForce::Ask
                        | UtteranceForce::Command
                        | UtteranceForce::Parenthetical
                        | UtteranceForce::Vocative => new!(ScopeBoundary::ForceSegment),
                        UtteranceForce::Mention | UtteranceForce::Quote => {
                            new!(ScopeBoundary::Opaque)
                        }
                        UtteranceForce::Subordinated => new!(ScopeBoundary::Closure),
                    };
                    self.region_for(id, new!(ScopeSite::Content), home, boundary);
                }
            }
            data!(SemanticObject::Sequence(node)) => {
                let boundary = if node.nonlogical_connection.is_some() {
                    new!(ScopeBoundary::NonlogicalJoin)
                } else {
                    new!(ScopeBoundary::SequentialContinuation)
                };
                for index in 0..node.items.len() {
                    self.region_for(id, new!(ScopeSite::Item { index }), home, boundary.clone());
                }
            }
            data!(SemanticObject::Eventuality(node)) => {
                self.declare_abstraction_regions(
                    id,
                    home,
                    node.content.is_some(),
                    node.body.is_some(),
                    &node.parameters,
                    node.relative_clauses.len(),
                );
            }
            data!(SemanticObject::Referent(node)) => {
                // A referent has no `content` locus; its abstraction, when it
                // has one, is reached through `abstracted` at its own locus.
                self.declare_abstraction_regions(
                    id,
                    home,
                    false,
                    node.body.is_some(),
                    &node.parameters,
                    node.relative_clauses.len(),
                );
            }
            data!(SemanticObject::Predication(node)) => {
                let data!(PredicationRelation::Named { relation }) = node.relation.as_data() else {
                    return;
                };
                for place in node.arguments.keys().copied() {
                    self.region_for(
                        id,
                        new!(ScopeSite::Argument { place }),
                        home,
                        new!(ScopeBoundary::LexicalArgument {
                            relation: relation.clone(),
                            original_place: place,
                        }),
                    );
                }
            }
            data!(SemanticObject::Formula(node)) => self.declare_formula_regions(id, node, home),
            data!(SemanticObject::Sign(node)) => {
                if node.quotation.is_some() {
                    let boundary = if node.sign_kind == Some(SignKind::Quotation) {
                        new!(ScopeBoundary::Opaque)
                    } else {
                        new!(ScopeBoundary::TokenFacts)
                    };
                    self.region_for(id, new!(ScopeSite::Quotation), home, boundary);
                }
                if node.denotes.is_some() {
                    self.region_for(
                        id,
                        new!(ScopeSite::Denotation),
                        home,
                        new!(ScopeBoundary::TokenFacts),
                    );
                }
                for index in 0..node.relative_clauses.len() {
                    self.region_for(
                        id,
                        new!(ScopeSite::RelativeClause { index }),
                        home,
                        new!(ScopeBoundary::Multiplicity),
                    );
                }
            }
            data!(SemanticObject::Question(node)) => {
                let region = self.region_for(
                    id,
                    new!(ScopeSite::Body),
                    home,
                    new!(ScopeBoundary::Question),
                );
                let slots = node
                    .slots
                    .iter()
                    .filter_map(QuestionSlot::parameter)
                    .collect::<Vec<_>>();
                for index in 0..slots.len() {
                    self.alias_site(id, new!(ScopeSite::Binder { index }), region);
                }
                self.set_binders(region, slots);
            }
            _ => {}
        }
    }

    /// A description or abstraction: its body is a lambda region over the
    /// parameters it declares, its content is a reification, and each relative
    /// clause is its own property region.
    #[requires(true)]
    #[ensures(true)]
    fn declare_abstraction_regions(
        &mut self,
        id: SemanticObjectId,
        home: usize,
        has_content: bool,
        has_body: bool,
        parameters: &[SemanticObjectId],
        relative_clauses: usize,
    ) {
        if has_content {
            self.region_for(
                id,
                new!(ScopeSite::Content),
                home,
                new!(ScopeBoundary::Reification),
            );
        }
        if has_body || !parameters.is_empty() {
            let region = self.region_for(
                id,
                new!(ScopeSite::Body),
                home,
                new!(ScopeBoundary::Multiplicity),
            );
            for index in 0..parameters.len() {
                self.alias_site(id, new!(ScopeSite::Binder { index }), region);
            }
            self.set_binders(region, parameters.to_vec());
        }
        for index in 0..relative_clauses {
            self.region_for(
                id,
                new!(ScopeSite::RelativeClause { index }),
                home,
                new!(ScopeBoundary::Multiplicity),
            );
        }
    }

    /// Formula-shaped regions.
    ///
    /// Quantifier bundles nest cumulatively left to right (CLL 16.7,
    /// ex. 16.42–16.44): binding *i*'s restriction is evaluated under bindings
    /// `0..=i` and the body under all of them, so each binding gets its own
    /// region rather than one flat region carrying every binder.
    #[requires(true)]
    #[ensures(true)]
    fn declare_formula_regions(&mut self, id: SemanticObjectId, node: &FormulaNode, home: usize) {
        match node.as_data() {
            data!(FormulaNode::Connective(node)) => {
                for index in 0..node.children.len() {
                    self.region_for(
                        id,
                        new!(ScopeSite::Operand { index }),
                        home,
                        new!(ScopeBoundary::ConnectiveBranch),
                    );
                }
            }
            data!(FormulaNode::Quantified(node)) => {
                let region = self.region_for(
                    id,
                    new!(ScopeSite::Restriction { binding: 0 }),
                    home,
                    new!(ScopeBoundary::Multiplicity),
                );
                self.alias_site(id, new!(ScopeSite::Binder { index: 0 }), region);
                self.alias_site(id, new!(ScopeSite::Body), region);
                self.set_binders(region, vec![node.variable]);
            }
            data!(FormulaNode::QuantifierBundle(node)) => {
                let mut parent = home;
                for (binding, entry) in node.bindings.iter().enumerate() {
                    let region = self.region_for(
                        id,
                        new!(ScopeSite::Restriction { binding }),
                        parent,
                        new!(ScopeBoundary::Multiplicity),
                    );
                    self.alias_site(id, new!(ScopeSite::Binder { index: binding }), region);
                    self.set_binders(region, vec![entry.variable]);
                    parent = region;
                }
                self.alias_site(id, new!(ScopeSite::Body), parent);
            }
            data!(FormulaNode::RespectivelyDistribution(node)) => {
                let mut parent = home;
                for (binding, stream) in node.streams.iter().enumerate() {
                    let region = self.region_for(
                        id,
                        new!(ScopeSite::Restriction { binding }),
                        parent,
                        new!(ScopeBoundary::Multiplicity),
                    );
                    self.alias_site(id, new!(ScopeSite::Binder { index: binding }), region);
                    self.set_binders(region, vec![stream.slot]);
                    parent = region;
                }
                self.alias_site(id, new!(ScopeSite::Body), parent);
            }
            _ => {}
        }
    }
}
