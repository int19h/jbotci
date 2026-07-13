//! Human-readable projections of the canonical semantic graph.
//!
//! These renderers never construct semantic content. They walk the validated,
//! typed graph and expose two different views of the same objects: a claims
//! ledger for validation and a structural tree for scope inspection.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

#[allow(unused_imports)]
use bityzba::{contract_trait, data, ensures, invariant, new, requires};

use crate::model::{
    ArgumentValue, ArgumentValueKind, Descriptor, DescriptorKind, DisplayedContentAssertionEffect,
    DisplayedContentFamily, DisplayedContentNode, DisplayedContentPolarity, FormulaNode,
    FormulaNodeData, FormulaOperator, GeneratedEventualityId, IndexicalKind, PlaceIndex,
    PredicationMode, PredicationNode, PredicationRelationData, QuantifiedFormulaNode,
    ReferentCategory, RelativeClause, RelativeClauseKind, ScopeDependenceData, SemanticGraph,
    SemanticObject, SemanticObjectData, SemanticObjectId, SemanticObjectKind, SemanticSort,
    SequenceNode, UtteranceForce, UtteranceNode,
};

/// Render the graph as a flat, tiered claims ledger.
#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn render_claims(graph: &SemanticGraph) -> String {
    let mut visitor = ClaimsVisitor::new(graph);
    DerivedTraversal::new(graph).walk(&mut visitor);
    visitor.finish()
}

/// Render the graph as an indented formula/utterance tree.
#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn render_tree(graph: &SemanticGraph) -> String {
    let mut visitor = TreeVisitor::new(graph);
    DerivedTraversal::new(graph).walk(&mut visitor);
    visitor.finish()
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClaimTier {
    Asserted,
    Projected,
    Displayed,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClaimStatus {
    Commitment,
    NonClaim,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TraversalRole {
    Root,
    Content,
    SequenceContent,
    SequenceItem,
    ConnectionClaim,
    Aside,
    Child,
    Restriction,
    Body,
    DescriptorBody,
    RelationBody,
    AbstractionBody,
    RestrictiveRelativeClause,
    IncidentalRelativeClause,
    NonveridicalRelativeClause,
    ModalBody,
    DetachedIncidental,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy)]
struct TraversalLocation {
    tier: ClaimTier,
    claim_status: ClaimStatus,
    role: TraversalRole,
    depth: usize,
}

#[contract_trait]
trait DerivedVisitor<'graph> {
    #[requires(true)]
    #[ensures(true)]
    fn enter_utterance(
        &mut self,
        _id: SemanticObjectId,
        _node: &'graph UtteranceNode,
        _location: TraversalLocation,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_utterance(&mut self, _id: SemanticObjectId, _location: TraversalLocation) {}

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(
        &mut self,
        _id: SemanticObjectId,
        _node: &'graph SequenceNode,
        _location: TraversalLocation,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_sequence(&mut self, _id: SemanticObjectId, _location: TraversalLocation) {}

    #[requires(true)]
    #[ensures(true)]
    fn enter_formula(
        &mut self,
        _id: SemanticObjectId,
        _node: &'graph FormulaNode,
        _location: TraversalLocation,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_formula(&mut self, _id: SemanticObjectId, _location: TraversalLocation) {}

    #[requires(true)]
    #[ensures(true)]
    fn predication(
        &mut self,
        _id: SemanticObjectId,
        _node: &'graph PredicationNode,
        _location: TraversalLocation,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn referent(
        &mut self,
        _id: SemanticObjectId,
        _object: &'graph SemanticObject,
        _location: TraversalLocation,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn domain_import(
        &mut self,
        _formula: SemanticObjectId,
        _node: &'graph QuantifiedFormulaNode,
        _location: TraversalLocation,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn displayed(
        &mut self,
        _id: SemanticObjectId,
        _node: &'graph DisplayedContentNode,
        _location: TraversalLocation,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn cycle(&mut self, _id: SemanticObjectId, _location: TraversalLocation) {}
}

#[invariant(graph.objects.contains_key(&graph.root))]
struct DerivedTraversal<'graph> {
    graph: &'graph SemanticGraph,
}

#[invariant(true)]
struct TraversalState {
    active: BTreeSet<SemanticObjectId>,
    expanded_referents: BTreeSet<SemanticObjectId>,
    structural_restrictions: BTreeSet<SemanticObjectId>,
    visited_predications: BTreeSet<SemanticObjectId>,
    visited_displayed: BTreeSet<SemanticObjectId>,
}

impl TraversalState {
    #[requires(true)]
    #[ensures(ret.active.is_empty())]
    fn new() -> Self {
        Self {
            active: BTreeSet::new(),
            expanded_referents: BTreeSet::new(),
            structural_restrictions: BTreeSet::new(),
            visited_predications: BTreeSet::new(),
            visited_displayed: BTreeSet::new(),
        }
    }
}

impl<'graph> DerivedTraversal<'graph> {
    #[requires(graph.objects.contains_key(&graph.root))]
    #[ensures(ret.graph.root == graph.root)]
    fn new(graph: &'graph SemanticGraph) -> Self {
        new!(DerivedTraversal { graph })
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk<V: DerivedVisitor<'graph>>(&self, visitor: &mut V) {
        let mut state = TraversalState::new();
        self.walk_object(
            self.graph.root,
            TraversalLocation {
                tier: ClaimTier::Asserted,
                claim_status: ClaimStatus::Commitment,
                role: TraversalRole::Root,
                depth: 0,
            },
            &mut state,
            visitor,
        );

        // Incidental predications normally arrive through a typed descriptor or
        // relative-clause edge. Keep the renderer total for valid graphs that
        // share or expose such a formula through another typed route.
        let atom_formulas: BTreeMap<_, _> = self
            .graph
            .objects
            .iter()
            .filter_map(|(&id, object)| object.formula_predication().map(|prd| (prd, id)))
            .collect();
        for (&id, object) in &self.graph.objects {
            let Some(node) = object.as_predication() else {
                continue;
            };
            if node.mode != PredicationMode::Incidental || state.visited_predications.contains(&id)
            {
                continue;
            }
            let location = TraversalLocation {
                tier: ClaimTier::Projected,
                claim_status: ClaimStatus::Commitment,
                role: TraversalRole::DetachedIncidental,
                depth: 0,
            };
            if let Some(formula) = atom_formulas.get(&id) {
                self.walk_formula(*formula, location, &mut state, visitor);
            } else {
                self.walk_predication(id, location, &mut state, visitor);
            }
        }

        for (&id, object) in &self.graph.objects {
            if object.as_displayed_content().is_some() && !state.visited_displayed.contains(&id) {
                self.walk_displayed(
                    id,
                    TraversalLocation {
                        tier: ClaimTier::Displayed,
                        claim_status: ClaimStatus::Commitment,
                        role: TraversalRole::Aside,
                        depth: 0,
                    },
                    &mut state,
                    visitor,
                );
            }
        }
    }

    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn walk_object<V: DerivedVisitor<'graph>>(
        &self,
        id: SemanticObjectId,
        location: TraversalLocation,
        state: &mut TraversalState,
        visitor: &mut V,
    ) {
        if id.object_kind() == SemanticObjectKind::Formula {
            self.walk_formula(id, location, state, visitor);
            return;
        }
        if matches!(
            id.object_kind(),
            SemanticObjectKind::Referent | SemanticObjectKind::Parameter
        ) {
            self.visit_referent(id, location, state, visitor);
            return;
        }
        if !state.active.insert(id) {
            visitor.cycle(id, location);
            return;
        }
        let object = self.object(id);
        match object.as_data() {
            data!(SemanticObject::Utterance(node)) => {
                visitor.enter_utterance(id, node, location);
                let child_location = TraversalLocation {
                    depth: location.depth + 1,
                    ..location
                };
                self.visit_referent(node.speaker, child_location, state, visitor);
                self.visit_referent(node.audience, child_location, state, visitor);
                self.visit_referent(node.eventuality, child_location, state, visitor);
                self.visit_referent(node.deictic_ground.time, child_location, state, visitor);
                self.visit_referent(node.deictic_ground.place, child_location, state, visitor);
                if let Some(content) = node.content {
                    self.walk_object(
                        content,
                        TraversalLocation {
                            role: TraversalRole::Content,
                            depth: location.depth + 1,
                            ..location
                        },
                        state,
                        visitor,
                    );
                }
                for &aside in &node.asides {
                    self.walk_object(
                        aside,
                        TraversalLocation {
                            tier: if aside.object_kind() == SemanticObjectKind::DisplayedContent {
                                ClaimTier::Displayed
                            } else {
                                ClaimTier::Projected
                            },
                            role: TraversalRole::Aside,
                            depth: location.depth + 1,
                            ..location
                        },
                        state,
                        visitor,
                    );
                }
                visitor.exit_utterance(id, location);
            }
            data!(SemanticObject::Sequence(node)) => {
                visitor.enter_sequence(id, node, location);
                if let Some(content) = node.content {
                    self.walk_object(
                        content,
                        TraversalLocation {
                            role: TraversalRole::SequenceContent,
                            depth: location.depth + 1,
                            ..location
                        },
                        state,
                        visitor,
                    );
                }
                for &claim in &node.connection_claims {
                    self.walk_object(
                        claim,
                        TraversalLocation {
                            role: TraversalRole::ConnectionClaim,
                            depth: location.depth + 1,
                            ..location
                        },
                        state,
                        visitor,
                    );
                }
                for &item in &node.items {
                    self.walk_object(
                        item,
                        TraversalLocation {
                            role: TraversalRole::SequenceItem,
                            depth: location.depth + 1,
                            ..location
                        },
                        state,
                        visitor,
                    );
                }
                visitor.exit_sequence(id, location);
            }
            data!(SemanticObject::Formula(_)) => unreachable!("formula handled before object walk"),
            data!(SemanticObject::Predication(_)) => {
                self.walk_predication(id, location, state, visitor)
            }
            data!(SemanticObject::DisplayedContent(_)) => {
                self.walk_displayed(id, location, state, visitor)
            }
            _ => {}
        }
        state.active.remove(&id);
    }

    #[requires(id.object_kind() == SemanticObjectKind::Formula)]
    #[ensures(true)]
    fn walk_formula<V: DerivedVisitor<'graph>>(
        &self,
        id: SemanticObjectId,
        location: TraversalLocation,
        state: &mut TraversalState,
        visitor: &mut V,
    ) {
        if !state.active.insert(id) {
            visitor.cycle(id, location);
            return;
        }
        let formula = self
            .object(id)
            .as_formula()
            .expect("formula id has formula object");
        visitor.enter_formula(id, formula, location);
        match formula.as_data() {
            data!(FormulaNode::Atom(node)) => {
                self.walk_predication(
                    node.predication,
                    TraversalLocation {
                        depth: location.depth + 1,
                        ..location
                    },
                    state,
                    visitor,
                );
            }
            data!(FormulaNode::Connective(node)) => {
                for &child in &node.children {
                    self.walk_formula(
                        child,
                        TraversalLocation {
                            role: TraversalRole::Child,
                            depth: location.depth + 1,
                            ..location
                        },
                        state,
                        visitor,
                    );
                }
            }
            data!(FormulaNode::Quantified(node)) => {
                self.visit_referent(node.variable, location, state, visitor);
                if self.object(id).formula_domain_import().is_some() {
                    visitor.domain_import(
                        id,
                        node,
                        TraversalLocation {
                            tier: ClaimTier::Projected,
                            ..location
                        },
                    );
                }
                if let Some(restriction) = node.restriction {
                    self.walk_formula(
                        restriction,
                        TraversalLocation {
                            role: TraversalRole::Restriction,
                            depth: location.depth + 1,
                            ..location
                        },
                        state,
                        visitor,
                    );
                    // The same formula is also retained on the scoped argument's
                    // relative-clause edge. Its quantifier restriction branch is
                    // the structural owner; do not expand that shared edge again
                    // from the body or count it as a second projected claim.
                    state.structural_restrictions.insert(restriction);
                }
                self.walk_formula(
                    node.body,
                    TraversalLocation {
                        role: TraversalRole::Body,
                        depth: location.depth + 1,
                        ..location
                    },
                    state,
                    visitor,
                );
            }
            data!(FormulaNode::QuantifierBundle(node)) => {
                for binding in &node.bindings {
                    self.visit_referent(binding.variable, location, state, visitor);
                    if let Some(restriction) = binding.restriction {
                        self.walk_formula(
                            restriction,
                            TraversalLocation {
                                role: TraversalRole::Restriction,
                                depth: location.depth + 1,
                                ..location
                            },
                            state,
                            visitor,
                        );
                        state.structural_restrictions.insert(restriction);
                    }
                }
                self.walk_formula(
                    node.body,
                    TraversalLocation {
                        role: TraversalRole::Body,
                        depth: location.depth + 1,
                        ..location
                    },
                    state,
                    visitor,
                );
            }
            data!(FormulaNode::RespectivelyDistribution(node)) => {
                for stream in &node.streams {
                    self.visit_referent(stream.slot, location, state, visitor);
                    for &item in &stream.items {
                        if item.object_kind() == SemanticObjectKind::Formula {
                            self.walk_formula(
                                item,
                                TraversalLocation {
                                    role: TraversalRole::Child,
                                    depth: location.depth + 1,
                                    ..location
                                },
                                state,
                                visitor,
                            );
                        } else if matches!(
                            item.object_kind(),
                            SemanticObjectKind::Referent | SemanticObjectKind::Parameter
                        ) {
                            self.visit_referent(item, location, state, visitor);
                        }
                    }
                    if let Some(restriction) = stream.restriction {
                        self.walk_formula(
                            restriction,
                            TraversalLocation {
                                role: TraversalRole::Restriction,
                                depth: location.depth + 1,
                                ..location
                            },
                            state,
                            visitor,
                        );
                        state.structural_restrictions.insert(restriction);
                    }
                }
                self.walk_formula(
                    node.body,
                    TraversalLocation {
                        role: TraversalRole::Body,
                        depth: location.depth + 1,
                        ..location
                    },
                    state,
                    visitor,
                );
            }
        }
        visitor.exit_formula(id, location);
        state.active.remove(&id);
    }

    #[requires(id.object_kind() == SemanticObjectKind::Predication)]
    #[ensures(true)]
    fn walk_predication<V: DerivedVisitor<'graph>>(
        &self,
        id: SemanticObjectId,
        location: TraversalLocation,
        state: &mut TraversalState,
        visitor: &mut V,
    ) {
        state.visited_predications.insert(id);
        let node = self
            .object(id)
            .as_predication()
            .expect("predication id has predication object");
        let tier = match node.mode {
            PredicationMode::Incidental => ClaimTier::Projected,
            PredicationMode::Displayed => ClaimTier::Displayed,
            _ => location.tier,
        };
        let location = TraversalLocation { tier, ..location };
        visitor.predication(id, node, location);
        if let Some(eventuality) = node.eventuality {
            self.visit_referent(eventuality, location, state, visitor);
        }
        for argument in node.arguments.values() {
            self.visit_argument(argument, location, state, visitor);
        }
        for modal in &node.modal_arguments {
            for argument in modal.arguments.values() {
                self.visit_argument(argument, location, state, visitor);
            }
            if let Some(body) = modal.body {
                self.walk_formula(
                    body,
                    TraversalLocation {
                        role: TraversalRole::ModalBody,
                        depth: location.depth + 1,
                        ..location
                    },
                    state,
                    visitor,
                );
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_argument<V: DerivedVisitor<'graph>>(
        &self,
        argument: &'graph ArgumentValue,
        location: TraversalLocation,
        state: &mut TraversalState,
        visitor: &mut V,
    ) {
        if let Some(value) = argument.value {
            self.visit_referent(value, location, state, visitor);
        }
        for clause in &argument.relative_clauses {
            if state.structural_restrictions.contains(&clause.body) {
                continue;
            }
            self.walk_relative_clause(
                clause,
                TraversalLocation {
                    depth: location.depth + 1,
                    ..location
                },
                state,
                visitor,
            );
        }
    }

    #[requires(matches!(id.object_kind(), SemanticObjectKind::Referent | SemanticObjectKind::Parameter))]
    #[ensures(true)]
    fn visit_referent<V: DerivedVisitor<'graph>>(
        &self,
        id: SemanticObjectId,
        location: TraversalLocation,
        state: &mut TraversalState,
        visitor: &mut V,
    ) {
        let object = self.object(id);
        visitor.referent(id, object, location);
        if id.object_kind() != SemanticObjectKind::Referent {
            return;
        }
        if state.active.contains(&id) {
            return;
        }
        if !state.expanded_referents.insert(id) {
            return;
        }
        let inserted = state.active.insert(id);
        debug_assert!(inserted, "referent active-set check and insertion agree");
        // Referent expansion is stable typed-field order: descriptor content,
        // intensional body, then referent-level clauses. Formula children and
        // predication arguments retain their own stored/BTreeMap order.
        if let Some(descriptor) = object.descriptor() {
            self.walk_descriptor(
                descriptor,
                TraversalLocation {
                    depth: location.depth + 1,
                    ..location
                },
                state,
                visitor,
            );
        }
        let (body, clauses): (Option<SemanticObjectId>, &[RelativeClause]) = match object.as_data()
        {
            data!(SemanticObject::Eventuality(node)) => (node.body, &node.relative_clauses),
            data!(SemanticObject::Referent(node)) => (node.body, &node.relative_clauses),
            _ => (None, &[]),
        };
        if let Some(body) = body {
            self.walk_formula(
                body,
                TraversalLocation {
                    claim_status: ClaimStatus::NonClaim,
                    role: if object.sort() == Some(SemanticSort::Relation) {
                        TraversalRole::RelationBody
                    } else {
                        TraversalRole::AbstractionBody
                    },
                    depth: location.depth + 1,
                    ..location
                },
                state,
                visitor,
            );
        }
        for clause in clauses {
            self.walk_relative_clause(
                clause,
                TraversalLocation {
                    depth: location.depth + 1,
                    ..location
                },
                state,
                visitor,
            );
        }
        state.active.remove(&id);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_descriptor<V: DerivedVisitor<'graph>>(
        &self,
        descriptor: &'graph Descriptor,
        location: TraversalLocation,
        state: &mut TraversalState,
        visitor: &mut V,
    ) {
        if let Some(body) = descriptor.body {
            let claim_status = if location.claim_status == ClaimStatus::NonClaim
                || descriptor.veridical == Some(false)
            {
                ClaimStatus::NonClaim
            } else {
                ClaimStatus::Commitment
            };
            self.walk_formula(
                body,
                TraversalLocation {
                    tier: if claim_status == ClaimStatus::Commitment {
                        ClaimTier::Projected
                    } else {
                        location.tier
                    },
                    claim_status,
                    role: TraversalRole::DescriptorBody,
                    ..location
                },
                state,
                visitor,
            );
        }
        for clause in &descriptor.relative_clauses {
            self.walk_relative_clause(clause, location, state, visitor);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_relative_clause<V: DerivedVisitor<'graph>>(
        &self,
        clause: &'graph RelativeClause,
        location: TraversalLocation,
        state: &mut TraversalState,
        visitor: &mut V,
    ) {
        let claim_status =
            if location.claim_status == ClaimStatus::NonClaim || clause.veridical == Some(false) {
                ClaimStatus::NonClaim
            } else {
                ClaimStatus::Commitment
            };
        self.walk_formula(
            clause.body,
            TraversalLocation {
                tier: if claim_status == ClaimStatus::Commitment {
                    ClaimTier::Projected
                } else {
                    location.tier
                },
                claim_status,
                role: if clause.veridical == Some(false) {
                    TraversalRole::NonveridicalRelativeClause
                } else {
                    match clause.kind {
                        RelativeClauseKind::Restrictive => TraversalRole::RestrictiveRelativeClause,
                        RelativeClauseKind::Incidental => TraversalRole::IncidentalRelativeClause,
                    }
                },
                ..location
            },
            state,
            visitor,
        );
    }

    #[requires(id.object_kind() == SemanticObjectKind::DisplayedContent)]
    #[ensures(true)]
    fn walk_displayed<V: DerivedVisitor<'graph>>(
        &self,
        id: SemanticObjectId,
        location: TraversalLocation,
        state: &mut TraversalState,
        visitor: &mut V,
    ) {
        state.visited_displayed.insert(id);
        let node = self
            .object(id)
            .as_displayed_content()
            .expect("display id has displayed-content object");
        let location = TraversalLocation {
            tier: ClaimTier::Displayed,
            ..location
        };
        visitor.displayed(id, node, location);
        self.visit_referent(node.experiencer, location, state, visitor);
        if matches!(
            node.target.object_kind(),
            SemanticObjectKind::Referent | SemanticObjectKind::Parameter
        ) {
            self.visit_referent(node.target, location, state, visitor);
        }
    }

    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn object(&self, id: SemanticObjectId) -> &'graph SemanticObject {
        self.graph
            .objects
            .get(&id)
            .expect("validated semantic graph references are defined")
    }
}

#[invariant(!text.is_empty())]
struct ClaimLine {
    text: String,
}

#[invariant(true)]
struct ClaimsVisitor<'graph> {
    graph: &'graph SemanticGraph,
    asserted: Vec<ClaimLine>,
    projected: Vec<ClaimLine>,
    displayed: Vec<ClaimLine>,
    context: Vec<String>,
    seen_predications: BTreeSet<(ClaimTierKey, SemanticObjectId)>,
    seen_constants: BTreeSet<SemanticObjectId>,
    seen_imports: BTreeSet<SemanticObjectId>,
    seen_displayed: BTreeSet<SemanticObjectId>,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ClaimTierKey {
    Asserted,
    Projected,
    Displayed,
}

impl From<ClaimTier> for ClaimTierKey {
    #[requires(true)]
    #[ensures(true)]
    fn from(value: ClaimTier) -> Self {
        match value {
            ClaimTier::Asserted => Self::Asserted,
            ClaimTier::Projected => Self::Projected,
            ClaimTier::Displayed => Self::Displayed,
        }
    }
}

impl<'graph> ClaimsVisitor<'graph> {
    #[requires(graph.objects.contains_key(&graph.root))]
    #[ensures(ret.graph.root == graph.root)]
    fn new(graph: &'graph SemanticGraph) -> Self {
        Self {
            graph,
            asserted: Vec::new(),
            projected: Vec::new(),
            displayed: Vec::new(),
            context: Vec::new(),
            seen_predications: BTreeSet::new(),
            seen_constants: BTreeSet::new(),
            seen_imports: BTreeSet::new(),
            seen_displayed: BTreeSet::new(),
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn finish(self) -> String {
        let mut output = String::new();
        push_claim_tier(&mut output, "asserted", &self.asserted);
        output.push('\n');
        push_claim_tier(&mut output, "presupposed/projected", &self.projected);
        output.push('\n');
        push_claim_tier(&mut output, "displayed", &self.displayed);
        output
    }

    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn push(&mut self, tier: ClaimTier, text: String) {
        let line = new!(ClaimLine { text });
        match tier {
            ClaimTier::Asserted => self.asserted.push(line),
            ClaimTier::Projected => self.projected.push(line),
            ClaimTier::Displayed => self.displayed.push(line),
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn context_label(&self) -> String {
        if self.context.is_empty() {
            "graph".to_owned()
        } else {
            self.context.join(" > ")
        }
    }
}

#[contract_trait]
impl<'graph> DerivedVisitor<'graph> for ClaimsVisitor<'graph> {
    #[requires(true)]
    #[ensures(true)]
    fn enter_utterance(
        &mut self,
        id: SemanticObjectId,
        node: &'graph UtteranceNode,
        _location: TraversalLocation,
    ) {
        self.context
            .push(format!("{id} {}", utterance_force_label(node.force)));
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_utterance(&mut self, _id: SemanticObjectId, _location: TraversalLocation) {
        self.context.pop();
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(
        &mut self,
        id: SemanticObjectId,
        node: &'graph SequenceNode,
        _location: TraversalLocation,
    ) {
        let binding = event_binding_label(self.graph, &node.bound_eventualities);
        self.context.push(if binding.is_empty() {
            format!("sequence {id}")
        } else {
            format!("sequence {id} {binding}")
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_sequence(&mut self, _id: SemanticObjectId, _location: TraversalLocation) {
        self.context.pop();
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_formula(
        &mut self,
        id: SemanticObjectId,
        node: &'graph FormulaNode,
        location: TraversalLocation,
    ) {
        self.context
            .push(formula_context_label(self.graph, id, node, location.role));
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_formula(&mut self, _id: SemanticObjectId, _location: TraversalLocation) {
        self.context.pop();
    }

    #[requires(true)]
    #[ensures(true)]
    fn predication(
        &mut self,
        id: SemanticObjectId,
        node: &'graph PredicationNode,
        location: TraversalLocation,
    ) {
        if location.claim_status == ClaimStatus::NonClaim {
            return;
        }
        let tier = location.tier;
        if self.seen_predications.insert((tier.into(), id)) {
            let predication = format_predication(self.graph, id, node);
            self.push(
                tier,
                format!(
                    "{predication} [mode={}; context={}]",
                    predication_mode_label(node.mode),
                    self.context_label()
                ),
            );
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn referent(
        &mut self,
        id: SemanticObjectId,
        object: &'graph SemanticObject,
        location: TraversalLocation,
    ) {
        if location.claim_status == ClaimStatus::NonClaim {
            return;
        }
        if (object.referent_category() == Some(ReferentCategory::Constant)
            || object.referent_category() == Some(ReferentCategory::Indexical))
            && self.seen_constants.insert(id)
        {
            let nonclaim_context = referent_nonclaim_context(self.graph, object);
            let category = if object.referent_category() == Some(ReferentCategory::Constant) {
                "constant"
            } else {
                "indexical"
            };
            self.push(
                ClaimTier::Projected,
                format!(
                    "denotes {} [{}; {category};{} context={}]",
                    referent_label(self.graph, id),
                    binder_dependence_context(self.graph, object),
                    nonclaim_context,
                    self.context_label()
                ),
            );
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn domain_import(
        &mut self,
        formula: SemanticObjectId,
        node: &'graph QuantifiedFormulaNode,
        location: TraversalLocation,
    ) {
        if location.claim_status == ClaimStatus::NonClaim {
            return;
        }
        if self.seen_imports.insert(formula) {
            let restriction = node
                .restriction
                .expect("domain import requires a restriction formula");
            self.push(
                ClaimTier::Projected,
                format!(
                    "exists {} satisfying {} [restriction={restriction}; projective domain import of {formula}]",
                    referent_label(self.graph, node.variable),
                    formula_reference_label(self.graph, restriction)
                ),
            );
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn displayed(
        &mut self,
        id: SemanticObjectId,
        node: &'graph DisplayedContentNode,
        _location: TraversalLocation,
    ) {
        if self.seen_displayed.insert(id) {
            self.push(ClaimTier::Displayed, format_displayed(self.graph, id, node));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn cycle(&mut self, id: SemanticObjectId, location: TraversalLocation) {
        if location.claim_status == ClaimStatus::NonClaim {
            return;
        }
        self.push(
            location.tier,
            format!(
                "shared/cyclic reference to {id} [context={}]",
                self.context_label()
            ),
        );
    }
}

#[requires(object.referent_category().is_some_and(|category| matches!(category, ReferentCategory::Constant | ReferentCategory::Indexical)))]
#[ensures(ret.starts_with("binder-dependence="))]
fn binder_dependence_context(graph: &SemanticGraph, object: &SemanticObject) -> String {
    let Some(scope_dependence) = object.scope_dependence() else {
        // Indexicals are rigidly fixed by their category and do not carry the
        // constant-only wire field.
        return "binder-dependence=fixed".to_owned();
    };
    match scope_dependence.as_data() {
        data!(ScopeDependence::Fixed) => "binder-dependence=fixed".to_owned(),
        data!(ScopeDependence::Underspecified { may_depend_on }) => {
            let mut binders = String::new();
            for binder in may_depend_on {
                if !binders.is_empty() {
                    binders.push_str(", ");
                }
                binders.push_str(&referent_label(graph, *binder));
            }
            format!("binder-dependence=underspecified; may-depend-on={binders}")
        }
    }
}

#[invariant(true)]
struct TreeVisitor<'graph> {
    graph: &'graph SemanticGraph,
    output: String,
}

impl<'graph> TreeVisitor<'graph> {
    #[requires(graph.objects.contains_key(&graph.root))]
    #[ensures(ret.graph.root == graph.root)]
    fn new(graph: &'graph SemanticGraph) -> Self {
        Self {
            graph,
            output: String::new(),
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn finish(mut self) -> String {
        while self.output.ends_with('\n') {
            self.output.pop();
        }
        if self.output.is_empty() {
            "(empty semantic tree)".to_owned()
        } else {
            self.output
        }
    }

    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn line(&mut self, depth: usize, text: &str) {
        for _ in 0..depth {
            self.output.push_str("  ");
        }
        self.output.push_str(text);
        self.output.push('\n');
    }
}

#[contract_trait]
impl<'graph> DerivedVisitor<'graph> for TreeVisitor<'graph> {
    #[requires(true)]
    #[ensures(true)]
    fn enter_utterance(
        &mut self,
        id: SemanticObjectId,
        node: &'graph UtteranceNode,
        location: TraversalLocation,
    ) {
        self.line(
            location.depth,
            &format!(
                "{}utterance {} [{id}]",
                tree_role_prefix(location.role),
                utterance_force_label(node.force)
            ),
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(
        &mut self,
        id: SemanticObjectId,
        node: &'graph SequenceNode,
        location: TraversalLocation,
    ) {
        let binding = event_binding_label(self.graph, &node.bound_eventualities);
        self.line(
            location.depth,
            &format!(
                "{}sequence{} [{id}]",
                tree_role_prefix(location.role),
                if binding.is_empty() {
                    String::new()
                } else {
                    format!(" {binding}")
                }
            ),
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_formula(
        &mut self,
        id: SemanticObjectId,
        node: &'graph FormulaNode,
        location: TraversalLocation,
    ) {
        self.line(
            location.depth,
            &format!(
                "{}{} [{id}]",
                tree_role_prefix(location.role),
                formula_tree_label(self.graph, id, node)
            ),
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn predication(
        &mut self,
        id: SemanticObjectId,
        node: &'graph PredicationNode,
        location: TraversalLocation,
    ) {
        self.line(location.depth, &format_predication(self.graph, id, node));
    }

    #[requires(true)]
    #[ensures(true)]
    fn displayed(
        &mut self,
        id: SemanticObjectId,
        node: &'graph DisplayedContentNode,
        location: TraversalLocation,
    ) {
        self.line(
            location.depth,
            &format!(
                "{}{}",
                tree_role_prefix(location.role),
                format_displayed(self.graph, id, node)
            ),
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn cycle(&mut self, id: SemanticObjectId, location: TraversalLocation) {
        self.line(location.depth, &format!("shared reference [{id}]"));
    }
}

#[requires(!title.is_empty())]
#[ensures(true)]
fn push_claim_tier(output: &mut String, title: &str, lines: &[ClaimLine]) {
    output.push_str(title);
    output.push(':');
    output.push('\n');
    if lines.is_empty() {
        output.push_str("- (none)\n");
    } else {
        for line in lines {
            output.push_str("- ");
            output.push_str(&line.text);
            output.push('\n');
        }
    }
    output.pop();
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_predication(
    graph: &SemanticGraph,
    id: SemanticObjectId,
    node: &PredicationNode,
) -> String {
    let relation = match node.relation.as_data() {
        data!(PredicationRelation::Named { relation }) => relation.clone(),
        data!(PredicationRelation::Parameter { parameter }) => {
            format!("relation?{parameter}")
        }
    };
    let mut output = String::new();
    if let Some(scalar) = &node.scalar_negation {
        let _ = write!(output, "{}:", scalar_negation_label(scalar.kind));
    }
    output.push_str(&relation);
    output.push('(');
    for (index, (place, argument)) in node.arguments.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        let _ = write!(output, "x{}=", place.get());
        format_argument_to(graph, argument, &mut output);
    }
    output.push(')');
    if let Some(eventuality) = node.eventuality {
        let _ = write!(output, " {{event={}}}", referent_label(graph, eventuality));
    }
    if let Some(tanru_link) = &node.tanru_link
        && let Some(head_eventuality) = graph
            .objects
            .get(&tanru_link.head)
            .and_then(SemanticObject::predication_eventuality)
    {
        let _ = write!(
            output,
            " {{tanru-head-event={}}}",
            referent_label(graph, head_eventuality)
        );
    }
    if !node.modal_arguments.is_empty() {
        output.push_str(" {modal=");
        for (modal_index, modal) in node.modal_arguments.iter().enumerate() {
            if modal_index > 0 {
                output.push_str(", ");
            }
            if let Some(relation) = &modal.relation {
                output.push_str(relation);
                output.push('(');
                for (argument_index, (place, argument)) in modal.arguments.iter().enumerate() {
                    if argument_index > 0 {
                        output.push_str(", ");
                    }
                    let _ = write!(output, "x{}=", place.get());
                    format_argument_to(graph, argument, &mut output);
                }
                output.push(')');
            } else if let Some(body) = modal.body {
                let _ = write!(output, "formula={body}");
            }
        }
        output.push('}');
    }
    let _ = write!(output, " [{id}]");
    output
}

#[requires(true)]
#[ensures(true)]
fn format_argument_to(graph: &SemanticGraph, argument: &ArgumentValue, output: &mut String) {
    match argument.kind {
        ArgumentValueKind::Deleted => {
            let _ = write!(
                output,
                "deleted({})",
                argument.introduced_by.as_deref().unwrap_or("zi'o")
            );
        }
        ArgumentValueKind::Filled | ArgumentValueKind::Elided => {
            if let Some(value) = argument.value {
                output.push_str(&referent_label(graph, value));
            } else {
                output.push_str("missing-value");
            }
        }
    }
}

#[requires(graph.objects.contains_key(&id))]
#[ensures(!ret.is_empty())]
fn referent_label(graph: &SemanticGraph, id: SemanticObjectId) -> String {
    let object = graph
        .objects
        .get(&id)
        .expect("validated semantic graph references are defined");
    let label = match object.as_data() {
        data!(SemanticObject::Eventuality(node)) => node
            .indexical
            .map(indexical_label)
            .map(str::to_owned)
            .or_else(|| descriptor_label(graph, node.descriptor.as_ref(), None))
            .unwrap_or_else(|| node.sort.label().to_owned()),
        data!(SemanticObject::Referent(node)) => {
            let content_relation = node
                .body
                .and_then(|body| single_atom_relation(graph, body))
                .or_else(|| {
                    node.descriptor.as_ref().and_then(|descriptor| {
                        speaker_description_content_relation(graph, descriptor)
                    })
                });
            node.indexical
                .map(indexical_label)
                .map(str::to_owned)
                .or_else(|| {
                    descriptor_label(graph, node.descriptor.as_ref(), content_relation.as_deref())
                })
                .or_else(|| {
                    content_relation.map(|relation| format!("{} {relation}", node.sort.label()))
                })
                .unwrap_or_else(|| node.sort.label().to_owned())
        }
        data!(SemanticObject::Sign(node)) => node
            .text
            .as_ref()
            .map(|text| format!("sign {text:?}"))
            .or_else(|| descriptor_label(graph, node.descriptor.as_ref(), None))
            .unwrap_or_else(|| "sign".to_owned()),
        data!(SemanticObject::Parameter(node)) => {
            format!("{}?{}", node.sort.label(), node.introduced_by)
        }
        data!(SemanticObject::Formula(_)) => "formula".to_owned(),
        _ => format!("{:?}", object.object_kind()),
    };
    format!("{label}[{id}]")
}

#[requires(true)]
#[ensures(true)]
fn descriptor_label(
    graph: &SemanticGraph,
    descriptor: Option<&Descriptor>,
    preferred_relation: Option<&str>,
) -> Option<String> {
    let descriptor = descriptor?;
    if descriptor.word.is_empty() {
        return Some(match preferred_relation {
            Some(relation) => format!("description {relation}"),
            None => "description".to_owned(),
        });
    }
    let relation = preferred_relation.map(str::to_owned).or_else(|| {
        descriptor
            .body
            .and_then(|body| single_atom_relation(graph, body))
    });
    Some(match relation {
        Some(relation) => format!("{} {relation}", descriptor.word),
        None => descriptor.word.clone(),
    })
}

#[requires(true)]
#[ensures(true)]
fn speaker_description_content_relation(
    graph: &SemanticGraph,
    descriptor: &Descriptor,
) -> Option<String> {
    if descriptor.kind != DescriptorKind::SpeakerDescription {
        return None;
    }
    let predication = graph
        .objects
        .get(&descriptor.body?)?
        .formula_predication()?;
    let property = graph
        .objects
        .get(&predication)?
        .as_predication()?
        .arguments
        .get(&PlaceIndex::new(4))?
        .value?;
    if property.referent_sort() != Some(SemanticSort::Relation) {
        return None;
    }
    referent_body(graph.objects.get(&property)?).and_then(|body| single_atom_relation(graph, body))
}

#[requires(true)]
#[ensures(ret.is_none_or(|body| body.object_kind() == SemanticObjectKind::Formula))]
fn referent_body(object: &SemanticObject) -> Option<SemanticObjectId> {
    match object.as_data() {
        data!(SemanticObject::Eventuality(node)) => node.body,
        data!(SemanticObject::Referent(node)) => node.body,
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn referent_nonclaim_context(graph: &SemanticGraph, object: &SemanticObject) -> String {
    let mut output = String::new();
    if let Some(body) = referent_body(object) {
        let label = if object.sort() == Some(SemanticSort::Relation) {
            "relation-body"
        } else {
            "abstraction-body"
        };
        let _ = write!(output, " {label}={};", formula_reference_label(graph, body));
    }
    if let Some(descriptor) = object.descriptor() {
        if descriptor.veridical == Some(false) {
            if let Some(body) = descriptor.body {
                let _ = write!(
                    output,
                    " non-claim-descriptor-body={};",
                    formula_reference_label(graph, body)
                );
            }
        }
        append_nonclaim_relative_clause_context(graph, &descriptor.relative_clauses, &mut output);
    }
    let clauses: &[RelativeClause] = match object.as_data() {
        data!(SemanticObject::Eventuality(node)) => &node.relative_clauses,
        data!(SemanticObject::Referent(node)) => &node.relative_clauses,
        _ => &[],
    };
    append_nonclaim_relative_clause_context(graph, clauses, &mut output);
    output
}

#[requires(true)]
#[ensures(true)]
fn append_nonclaim_relative_clause_context(
    graph: &SemanticGraph,
    clauses: &[RelativeClause],
    output: &mut String,
) {
    for clause in clauses {
        if clause.veridical == Some(false) {
            let _ = write!(
                output,
                " non-claim-restrictive-clause={};",
                formula_reference_label(graph, clause.body)
            );
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn single_atom_relation(graph: &SemanticGraph, formula: SemanticObjectId) -> Option<String> {
    let object = graph.objects.get(&formula)?;
    let predication = object.formula_predication()?;
    match graph
        .objects
        .get(&predication)?
        .as_predication()?
        .relation
        .as_data()
    {
        data!(PredicationRelation::Named { relation }) => Some(relation.clone()),
        data!(PredicationRelation::Parameter { .. }) => None,
    }
}

#[requires(formula.object_kind() == SemanticObjectKind::Formula)]
#[ensures(!ret.is_empty())]
fn formula_reference_label(graph: &SemanticGraph, formula: SemanticObjectId) -> String {
    let Some(predication_id) = graph
        .objects
        .get(&formula)
        .and_then(SemanticObject::formula_predication)
    else {
        return formula.to_string();
    };
    let Some(predication) = graph
        .objects
        .get(&predication_id)
        .and_then(SemanticObject::as_predication)
    else {
        return formula.to_string();
    };
    format_predication(graph, predication_id, predication)
}

#[requires(id.object_kind() == SemanticObjectKind::Formula)]
#[requires(graph.objects.contains_key(&id))]
#[ensures(!ret.is_empty())]
fn formula_tree_label(graph: &SemanticGraph, id: SemanticObjectId, node: &FormulaNode) -> String {
    let base = match node.as_data() {
        data!(FormulaNode::Atom(_)) => "atom".to_owned(),
        data!(FormulaNode::Connective(node)) => formula_operator_label(node.operator).to_owned(),
        data!(FormulaNode::Quantified(node)) => format!(
            "{} variable={}{}",
            formula_operator_label(node.operator),
            referent_label(graph, node.variable),
            if graph
                .objects
                .get(&id)
                .is_some_and(|object| object.formula_domain_import().is_some())
            {
                " domain-import=projective"
            } else {
                ""
            }
        ),
        data!(FormulaNode::QuantifierBundle(node)) => {
            format!("quantifier-bundle bindings={}", node.bindings.len())
        }
        data!(FormulaNode::RespectivelyDistribution(node)) => {
            format!("respectively-distribution streams={}", node.streams.len())
        }
    };
    let binding = event_binding_label(
        graph,
        graph
            .objects
            .get(&id)
            .expect("formula label requires a defined formula")
            .bound_eventualities(),
    );
    if binding.is_empty() {
        base
    } else {
        format!("{base} {binding}")
    }
}

#[requires(eventualities.iter().all(|eventuality| graph.objects.get(&eventuality.object_id()).is_some_and(SemanticObject::is_generated_eventuality)))]
#[ensures(ret.is_empty() == eventualities.is_empty())]
fn event_binding_label(graph: &SemanticGraph, eventualities: &[GeneratedEventualityId]) -> String {
    let mut output = String::new();
    for eventuality in eventualities {
        if !output.is_empty() {
            output.push_str(", ");
        }
        output.push_str(&referent_label(graph, eventuality.object_id()));
    }
    if output.is_empty() {
        output
    } else {
        format!("binds=exists {output}")
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn formula_context_label(
    graph: &SemanticGraph,
    id: SemanticObjectId,
    node: &FormulaNode,
    role: TraversalRole,
) -> String {
    format!(
        "{} {} [{id}]",
        traversal_role_label(role),
        formula_tree_label(graph, id, node)
    )
}

#[requires(!node.relation.is_empty())]
#[ensures(!ret.is_empty())]
fn format_displayed(
    graph: &SemanticGraph,
    id: SemanticObjectId,
    node: &DisplayedContentNode,
) -> String {
    format!(
        "{} [display={id}; family={}; polarity={}; assertion-effect={}; experiencer={}; target={}]",
        node.relation,
        displayed_family_label(node.family),
        displayed_polarity_label(node.polarity),
        assertion_effect_label(node.assertion_effect),
        referent_label(graph, node.experiencer),
        referent_label(graph, node.target)
    )
}

#[requires(true)]
#[ensures(ret.is_empty() == (role == TraversalRole::Root))]
fn tree_role_prefix(role: TraversalRole) -> &'static str {
    match role {
        TraversalRole::Root => "",
        TraversalRole::Content => "content: ",
        TraversalRole::SequenceContent => "combined content: ",
        TraversalRole::SequenceItem => "item: ",
        TraversalRole::ConnectionClaim => "connection claim: ",
        TraversalRole::Aside => "aside: ",
        TraversalRole::Child => "child: ",
        TraversalRole::Restriction => "restriction: ",
        TraversalRole::Body => "body: ",
        TraversalRole::DescriptorBody => "descriptor body: ",
        TraversalRole::RelationBody => "relation body: ",
        TraversalRole::AbstractionBody => "abstraction body: ",
        TraversalRole::RestrictiveRelativeClause => "restrictive relative clause: ",
        TraversalRole::IncidentalRelativeClause => "incidental relative clause: ",
        TraversalRole::NonveridicalRelativeClause => "non-claim restrictive relative clause: ",
        TraversalRole::ModalBody => "modal body: ",
        TraversalRole::DetachedIncidental => "incidental: ",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn traversal_role_label(role: TraversalRole) -> &'static str {
    match role {
        TraversalRole::Root => "root",
        TraversalRole::Content => "content",
        TraversalRole::SequenceContent => "sequence-content",
        TraversalRole::SequenceItem => "sequence-item",
        TraversalRole::ConnectionClaim => "connection-claim",
        TraversalRole::Aside => "aside",
        TraversalRole::Child => "child",
        TraversalRole::Restriction => "restriction",
        TraversalRole::Body => "body",
        TraversalRole::DescriptorBody => "descriptor-body",
        TraversalRole::RelationBody => "relation-body",
        TraversalRole::AbstractionBody => "abstraction-body",
        TraversalRole::RestrictiveRelativeClause => "restrictive-relative-clause",
        TraversalRole::IncidentalRelativeClause => "incidental-relative-clause",
        TraversalRole::NonveridicalRelativeClause => "non-claim-restrictive-relative-clause",
        TraversalRole::ModalBody => "modal-body",
        TraversalRole::DetachedIncidental => "incidental",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn formula_operator_label(operator: FormulaOperator) -> &'static str {
    match operator {
        FormulaOperator::Atom => "atom",
        FormulaOperator::Affirmed => "affirmed",
        FormulaOperator::Not => "not",
        FormulaOperator::Scoped => "scoped",
        FormulaOperator::And => "and",
        FormulaOperator::Or => "or",
        FormulaOperator::Implies => "implies",
        FormulaOperator::Iff => "iff",
        FormulaOperator::ExclusiveOr => "exclusive-or",
        FormulaOperator::WhetherOrNot => "whether-or-not",
        FormulaOperator::ConnectiveQuestion => "connective-question",
        FormulaOperator::Exists => "exists",
        FormulaOperator::Forall => "forall",
        FormulaOperator::None => "none",
        FormulaOperator::Cardinality => "cardinality",
        FormulaOperator::PluralExists => "plural-exists",
        FormulaOperator::PluralForall => "plural-forall",
        FormulaOperator::QuantifierBundle => "quantifier-bundle",
        FormulaOperator::RespectivelyDistribution => "respectively-distribution",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn utterance_force_label(force: UtteranceForce) -> &'static str {
    match force {
        UtteranceForce::Assert => "assert",
        UtteranceForce::Ask => "ask",
        UtteranceForce::Command => "command",
        UtteranceForce::Mention => "mention",
        UtteranceForce::Quote => "quote",
        UtteranceForce::Parenthetical => "parenthetical",
        UtteranceForce::Subordinated => "subordinated",
        UtteranceForce::Vocative => "vocative",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn predication_mode_label(mode: PredicationMode) -> &'static str {
    match mode {
        PredicationMode::Asserted => "asserted",
        PredicationMode::Definitional => "definitional",
        PredicationMode::Restrictive => "restrictive",
        PredicationMode::Incidental => "incidental",
        PredicationMode::Displayed => "displayed",
        PredicationMode::Inert => "inert",
        PredicationMode::Performative => "performative",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn indexical_label(indexical: IndexicalKind) -> &'static str {
    match indexical {
        IndexicalKind::Speaker => "speaker",
        IndexicalKind::Audience => "audience",
        IndexicalKind::Now => "now",
        IndexicalKind::Here => "here",
        IndexicalKind::ProximalDemonstrative => "proximal-demonstrative",
        IndexicalKind::MedialDemonstrative => "medial-demonstrative",
        IndexicalKind::DistalDemonstrative => "distal-demonstrative",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn displayed_family_label(family: DisplayedContentFamily) -> &'static str {
    match family {
        DisplayedContentFamily::Emotion => "emotion",
        DisplayedContentFamily::AttitudeModifier => "attitude-modifier",
        DisplayedContentFamily::PropositionalAttitude => "propositional-attitude",
        DisplayedContentFamily::Evidential => "evidential",
        DisplayedContentFamily::Discursive => "discursive",
        DisplayedContentFamily::Metalinguistic => "metalinguistic",
        DisplayedContentFamily::Emphasis => "emphasis",
        DisplayedContentFamily::QuestionPrompt => "question-prompt",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn displayed_polarity_label(polarity: DisplayedContentPolarity) -> &'static str {
    match polarity {
        DisplayedContentPolarity::Positive => "positive",
        DisplayedContentPolarity::Neutral => "neutral",
        DisplayedContentPolarity::Negative => "negative",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn assertion_effect_label(effect: DisplayedContentAssertionEffect) -> &'static str {
    match effect {
        DisplayedContentAssertionEffect::None => "none",
        DisplayedContentAssertionEffect::HostAsserted => "host-asserted",
        DisplayedContentAssertionEffect::HostSubordinated => "host-subordinated",
        DisplayedContentAssertionEffect::MetalinguisticallyVoided => "metalinguistically-voided",
        DisplayedContentAssertionEffect::Performative => "performative",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn scalar_negation_label(kind: crate::model::ScalarNegationKind) -> &'static str {
    match kind {
        crate::model::ScalarNegationKind::OtherThan => "other-than",
        crate::model::ScalarNegationKind::Opposite => "opposite",
        crate::model::ScalarNegationKind::Neutral => "neutral",
        crate::model::ScalarNegationKind::Affirmed => "affirmed",
    }
}
