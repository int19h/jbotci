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
    DisplayedContentFamily, DisplayedContentNode, DisplayedContentPolarity, EventualityClass,
    FormulaNode, FormulaNodeData, FormulaOperator, GeneratedEventualityId, IndexicalKind,
    PlaceIndex, PredicationMode, PredicationNode, PredicationRelationData, QuantifiedFormulaNode,
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
    let mut visitor = TreeVisitor::new(graph, TreeEventConditionPolicy::EverySite);
    DerivedTraversal::new(graph).walk(&mut visitor);
    visitor.finish()
}

/// Render the structural tree followed by only commitments displaced from it.
#[requires(true)]
#[ensures(!ret.is_empty())]
pub fn render_combined(graph: &SemanticGraph) -> String {
    let mut tree_visitor = TreeVisitor::new(graph, TreeEventConditionPolicy::StructuralSiteOnly);
    DerivedTraversal::new(graph).walk(&mut tree_visitor);
    let tree = tree_visitor.finish();

    let mut projected_visitor = CombinedProjectedVisitor::new(graph);
    DerivedTraversal::new(graph).walk(&mut projected_visitor);
    let projected = projected_visitor.finish();
    format!("{tree}\n\n{projected}")
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
                if let Some(eventuality) = node.eventuality {
                    self.visit_referent(eventuality, location, state, visitor);
                }
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

#[invariant(!label.is_empty())]
#[invariant(scope_operator.as_ref().is_none_or(|operator| !operator.is_empty()))]
struct ClaimsContextFrame {
    label: String,
    scope_operator: Option<String>,
}

#[invariant(true)]
struct ClaimsVisitor<'graph> {
    graph: &'graph SemanticGraph,
    asserted: Vec<ClaimLine>,
    projected: Vec<ClaimLine>,
    displayed: Vec<ClaimLine>,
    context: Vec<ClaimsContextFrame>,
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
        push_claim_tier(&mut output, "at-issue commitments", &self.asserted);
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
            let mut output = String::new();
            for frame in &self.context {
                if !output.is_empty() {
                    output.push_str(" > ");
                }
                output.push_str(&frame.label);
            }
            output
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn scope_label(&self) -> String {
        let mut output = String::new();
        for frame in &self.context {
            let Some(operator) = &frame.scope_operator else {
                continue;
            };
            if !output.is_empty() {
                output.push_str(" > ");
            }
            output.push_str(operator);
        }
        if output.is_empty() {
            "top-level".to_owned()
        } else {
            output
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
        self.context.push(new!(ClaimsContextFrame {
            label: format!("{id} {}", utterance_force_label(node.force)),
            scope_operator: None,
        }));
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
        self.context.push(new!(ClaimsContextFrame {
            label: if binding.is_empty() {
                format!("sequence {id}")
            } else {
                format!("sequence {id} {binding}")
            },
            scope_operator: None,
        }));
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
        self.context.push(new!(ClaimsContextFrame {
            label: formula_context_label(self.graph, id, node, location.role),
            scope_operator: formula_scope_operator_label(self.graph, node),
        }));
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
            let line = if tier == ClaimTier::Asserted {
                format!(
                    "{predication} [mode={}; scope={}; context={}]",
                    predication_mode_label(node.mode),
                    self.scope_label(),
                    self.context_label()
                )
            } else {
                format!(
                    "{predication} [mode={}; context={}]",
                    predication_mode_label(node.mode),
                    self.context_label()
                )
            };
            self.push(tier, line);
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
            let mut line = format!("denotes {} [", referent_label(self.graph, id));
            if object.as_eventuality().is_some() {
                format_eventuality_conditions_to(self.graph, id, &mut line);
                line.push_str("; ");
            }
            let _ = write!(
                line,
                "{}; {category};{} context={}]",
                binder_dependence_context(self.graph, object),
                nonclaim_context,
                self.context_label()
            );
            self.push(ClaimTier::Projected, line);
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CombinedImplicitConstantKind {
    Elided,
    TypicalPlaceValue,
}

#[invariant(::Fixed => true)]
#[invariant(::Underspecified { may_depend_on } => !may_depend_on.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum CombinedScopeDependence {
    Fixed,
    Underspecified {
        may_depend_on: BTreeSet<SemanticObjectId>,
    },
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CombinedImplicitConstantGroupKey {
    kind: CombinedImplicitConstantKind,
    scope_dependence: CombinedScopeDependence,
}

#[invariant(true)]
struct CombinedProjectedVisitor<'graph> {
    graph: &'graph SemanticGraph,
    displaced: Vec<ClaimLine>,
    frame_indexicals: Vec<SemanticObjectId>,
    frame_locutions: Vec<SemanticObjectId>,
    constants: Vec<SemanticObjectId>,
    implicit_constant_groups: BTreeMap<CombinedImplicitConstantGroupKey, Vec<SemanticObjectId>>,
    seen_predications: BTreeSet<SemanticObjectId>,
    seen_constants: BTreeSet<SemanticObjectId>,
    seen_imports: BTreeSet<SemanticObjectId>,
}

impl<'graph> CombinedProjectedVisitor<'graph> {
    #[requires(graph.objects.contains_key(&graph.root))]
    #[ensures(ret.graph.root == graph.root)]
    fn new(graph: &'graph SemanticGraph) -> Self {
        Self {
            graph,
            displaced: Vec::new(),
            frame_indexicals: Vec::new(),
            frame_locutions: Vec::new(),
            constants: Vec::new(),
            implicit_constant_groups: BTreeMap::new(),
            seen_predications: BTreeSet::new(),
            seen_constants: BTreeSet::new(),
            seen_imports: BTreeSet::new(),
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn finish(self) -> String {
        let mut output = String::from("projected:\n");
        let mut line_count = 0;

        if !self.frame_indexicals.is_empty() || !self.frame_locutions.is_empty() {
            push_combined_frame_line(
                self.graph,
                &self.frame_indexicals,
                &self.frame_locutions,
                &mut output,
            );
            line_count += 1;
        }
        for line in &self.displaced {
            push_combined_projected_line(&line.text, &mut output);
            line_count += 1;
        }
        for id in self.constants {
            let mut line = String::new();
            format_combined_constant_denotation_to(self.graph, id, &mut line);
            push_combined_projected_line(&line, &mut output);
            line_count += 1;
        }
        for (key, ids) in self.implicit_constant_groups {
            let mut line = String::new();
            format_combined_implicit_group_to(self.graph, &key, &ids, &mut line);
            push_combined_projected_line(&line, &mut output);
            line_count += 1;
        }

        if line_count == 0 {
            output.push_str("- (none)\n");
        }
        output.pop();
        output
    }

    #[requires(!text.is_empty())]
    #[ensures(true)]
    fn push_displaced(&mut self, text: String) {
        self.displaced.push(new!(ClaimLine { text }));
    }
}

#[contract_trait]
impl<'graph> DerivedVisitor<'graph> for CombinedProjectedVisitor<'graph> {
    #[requires(true)]
    #[ensures(true)]
    fn predication(
        &mut self,
        id: SemanticObjectId,
        node: &'graph PredicationNode,
        location: TraversalLocation,
    ) {
        if location.claim_status != ClaimStatus::Commitment
            || location.tier != ClaimTier::Projected
            || !self.seen_predications.insert(id)
        {
            return;
        }
        let predication = format_predication_with_event_conditions(self.graph, id, node, false);
        self.push_displaced(format!(
            "{predication} [mode={}]",
            predication_mode_label(node.mode)
        ));
    }

    #[requires(true)]
    #[ensures(true)]
    fn referent(
        &mut self,
        id: SemanticObjectId,
        object: &'graph SemanticObject,
        location: TraversalLocation,
    ) {
        if location.claim_status != ClaimStatus::Commitment || !self.seen_constants.insert(id) {
            return;
        }
        match object.referent_category() {
            Some(ReferentCategory::Indexical) => self.frame_indexicals.push(id),
            Some(ReferentCategory::Constant) => {
                if object.as_eventuality().is_some_and(|eventuality| {
                    eventuality.class == Some(EventualityClass::Locution)
                }) {
                    self.frame_locutions.push(id);
                } else if let Some(kind) = combined_implicit_constant_kind(object) {
                    let key = CombinedImplicitConstantGroupKey {
                        kind,
                        scope_dependence: combined_scope_dependence(object),
                    };
                    self.implicit_constant_groups
                        .entry(key)
                        .or_default()
                        .push(id);
                } else {
                    self.constants.push(id);
                }
            }
            _ => {}
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
        if location.claim_status != ClaimStatus::Commitment || !self.seen_imports.insert(formula) {
            return;
        }
        let restriction = node
            .restriction
            .expect("domain import requires a restriction formula");
        self.push_displaced(format!(
            "exists {} satisfying {} [restriction={restriction}; projective domain import of {formula}]",
            referent_label(self.graph, node.variable),
            formula_reference_label_with_event_conditions(self.graph, restriction, false)
        ));
    }
}

#[requires(!text.is_empty())]
#[ensures(true)]
fn push_combined_projected_line(text: &str, output: &mut String) {
    output.push_str("- ");
    output.push_str(text);
    output.push('\n');
}

#[requires(indexicals.iter().all(|id| graph.objects.get(id).is_some_and(|object| object.referent_category() == Some(ReferentCategory::Indexical))))]
#[requires(locutions.iter().all(|id| graph.objects.get(id).is_some_and(|object| object.as_eventuality().is_some_and(|eventuality| eventuality.class == Some(EventualityClass::Locution)))))]
#[requires(!indexicals.is_empty() || !locutions.is_empty())]
#[ensures(true)]
fn push_combined_frame_line(
    graph: &SemanticGraph,
    indexicals: &[SemanticObjectId],
    locutions: &[SemanticObjectId],
    output: &mut String,
) {
    output.push_str("- frame: indexicals=[");
    for (index, id) in indexicals.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        output.push_str(&referent_label(graph, *id));
        if graph
            .objects
            .get(id)
            .is_some_and(|object| object.as_eventuality().is_some())
        {
            output.push_str(" {");
            format_eventuality_conditions_to(graph, *id, output);
            output.push('}');
        }
    }
    output.push_str("] [binder-dependence=fixed]; locutions=[");
    for (index, id) in locutions.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        output.push_str(&referent_label(graph, *id));
    }
    output.push_str("] [binder-dependence=fixed]\n");
}

#[requires(object.referent_category() == Some(ReferentCategory::Constant))]
#[ensures(true)]
fn combined_implicit_constant_kind(
    object: &SemanticObject,
) -> Option<CombinedImplicitConstantKind> {
    match object.descriptor().map(|descriptor| descriptor.kind) {
        Some(DescriptorKind::Elided) => Some(CombinedImplicitConstantKind::Elided),
        Some(DescriptorKind::TypicalPlaceValue) => {
            Some(CombinedImplicitConstantKind::TypicalPlaceValue)
        }
        _ => None,
    }
}

#[requires(object.referent_category() == Some(ReferentCategory::Constant))]
#[ensures(true)]
fn combined_scope_dependence(object: &SemanticObject) -> CombinedScopeDependence {
    match object
        .scope_dependence()
        .expect("constant objects carry scope dependence")
        .as_data()
    {
        data!(ScopeDependence::Fixed) => new!(CombinedScopeDependence::Fixed),
        data!(ScopeDependence::Underspecified { may_depend_on }) => {
            new!(CombinedScopeDependence::Underspecified {
                may_depend_on: may_depend_on.clone(),
            })
        }
    }
}

#[requires(graph.objects.get(&id).is_some_and(|object| object.referent_category() == Some(ReferentCategory::Constant)))]
#[ensures(true)]
fn format_combined_constant_denotation_to(
    graph: &SemanticGraph,
    id: SemanticObjectId,
    output: &mut String,
) {
    let object = graph.objects.get(&id).expect("precondition checked");
    let _ = write!(output, "denotes {} [", referent_label(graph, id));
    if object.as_eventuality().is_some() {
        format_eventuality_conditions_to(graph, id, output);
        output.push_str("; ");
    }
    let _ = write!(
        output,
        "{}; constant]",
        binder_dependence_context(graph, object)
    );
}

#[requires(!ids.is_empty())]
#[requires(ids.iter().all(|id| graph.objects.get(id).is_some_and(|object| object.referent_category() == Some(ReferentCategory::Constant))))]
#[ensures(true)]
fn format_combined_implicit_group_to(
    graph: &SemanticGraph,
    key: &CombinedImplicitConstantGroupKey,
    ids: &[SemanticObjectId],
    output: &mut String,
) {
    output.push_str("denotes [");
    for (index, id) in ids.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        output.push_str(&referent_label(graph, *id));
    }
    output.push_str("] [");
    format_combined_scope_dependence_to(graph, &key.scope_dependence, output);
    let _ = write!(
        output,
        "; constant; descriptor-kind={}]",
        combined_implicit_constant_kind_label(key.kind)
    );
}

#[requires(true)]
#[ensures(true)]
fn format_combined_scope_dependence_to(
    graph: &SemanticGraph,
    scope_dependence: &CombinedScopeDependence,
    output: &mut String,
) {
    match scope_dependence.as_data() {
        data!(CombinedScopeDependence::Fixed) => output.push_str("binder-dependence=fixed"),
        data!(CombinedScopeDependence::Underspecified { may_depend_on }) => {
            output.push_str("binder-dependence=underspecified; may-depend-on=");
            for (index, binder) in may_depend_on.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                output.push_str(&referent_label(graph, *binder));
            }
        }
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn combined_implicit_constant_kind_label(kind: CombinedImplicitConstantKind) -> &'static str {
    match kind {
        CombinedImplicitConstantKind::Elided => "elided",
        CombinedImplicitConstantKind::TypicalPlaceValue => "typical-place-value",
    }
}

#[invariant(true)]
struct TreeVisitor<'graph> {
    graph: &'graph SemanticGraph,
    output: String,
    event_condition_policy: TreeEventConditionPolicy,
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TreeEventConditionPolicy {
    EverySite,
    StructuralSiteOnly,
}

impl<'graph> TreeVisitor<'graph> {
    #[requires(graph.objects.contains_key(&graph.root))]
    #[ensures(ret.graph.root == graph.root)]
    fn new(graph: &'graph SemanticGraph, event_condition_policy: TreeEventConditionPolicy) -> Self {
        Self {
            graph,
            output: String::new(),
            event_condition_policy,
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
        let mut line = format!(
            "{}utterance {}",
            tree_role_prefix(location.role),
            utterance_force_label(node.force)
        );
        format_eventuality_site_to(self.graph, node.eventuality, "event", &mut line);
        let _ = write!(line, " [{id}]");
        self.line(location.depth, &line);
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
                formula_tree_label_with_event_conditions(
                    self.graph,
                    id,
                    node,
                    self.event_condition_policy == TreeEventConditionPolicy::EverySite,
                    true,
                )
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
        self.line(
            location.depth,
            &format_predication_with_event_conditions(
                self.graph,
                id,
                node,
                self.event_condition_policy == TreeEventConditionPolicy::EverySite,
            ),
        );
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
    format_predication_with_event_conditions(graph, id, node, true)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn format_predication_with_event_conditions(
    graph: &SemanticGraph,
    id: SemanticObjectId,
    node: &PredicationNode,
    include_event_conditions: bool,
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
        format_eventuality_site_with_conditions_to(
            graph,
            eventuality,
            "event",
            include_event_conditions,
            &mut output,
        );
    }
    if let Some(tanru_link) = &node.tanru_link
        && let Some(head_eventuality) = graph
            .objects
            .get(&tanru_link.head)
            .and_then(SemanticObject::predication_eventuality)
    {
        format_eventuality_site_with_conditions_to(
            graph,
            head_eventuality,
            "tanru-head-event",
            include_event_conditions,
            &mut output,
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

#[requires(eventuality.object_kind() == SemanticObjectKind::Referent)]
#[requires(graph.objects.get(&eventuality).is_some_and(|object| object.as_eventuality().is_some()))]
#[requires(!site.is_empty())]
#[ensures(true)]
fn format_eventuality_site_to(
    graph: &SemanticGraph,
    eventuality: SemanticObjectId,
    site: &str,
    output: &mut String,
) {
    format_eventuality_site_with_conditions_to(graph, eventuality, site, true, output);
}

#[requires(eventuality.object_kind() == SemanticObjectKind::Referent)]
#[requires(graph.objects.get(&eventuality).is_some_and(|object| object.as_eventuality().is_some()))]
#[requires(!site.is_empty())]
#[ensures(true)]
fn format_eventuality_site_with_conditions_to(
    graph: &SemanticGraph,
    eventuality: SemanticObjectId,
    site: &str,
    include_conditions: bool,
    output: &mut String,
) {
    let _ = write!(output, " {{{site}={}", referent_label(graph, eventuality));
    if include_conditions {
        output.push_str("; ");
        format_eventuality_conditions_to(graph, eventuality, output);
    }
    output.push('}');
}

#[requires(eventuality.object_kind() == SemanticObjectKind::Referent)]
#[requires(graph.objects.get(&eventuality).is_some_and(|object| object.as_eventuality().is_some()))]
#[ensures(true)]
fn format_eventuality_conditions_to(
    graph: &SemanticGraph,
    eventuality: SemanticObjectId,
    output: &mut String,
) {
    let node = graph
        .objects
        .get(&eventuality)
        .and_then(SemanticObject::as_eventuality)
        .expect("validated eventuality reference has an eventuality object");

    // Keep this projection aligned with serialize_eventuality's complete
    // event-condition block: content, actuality, tense-modal, temporal and
    // spatial placement, intervals, aspect, recurrence, and modifier stacks.
    // Generic referent identity/body fields retain their established typed
    // labels and traversal sites instead of being duplicated in this suffix.
    output.push_str("time=");
    format_anchor_dimension_to(graph, node.time.as_ref(), &node.time_path, output);

    output.push_str("; actuality=");
    if let Some(actuality) = node.actuality {
        output.push_str(actuality_kind_label(actuality.kind));
    } else {
        output.push_str("unspecified");
    }

    output.push_str("; aspect=");
    format_aspect_dimension_to(graph, node.aspect.as_ref(), &node.aspects, output);

    output.push_str("; recurrence=");
    if node.recurrence.is_empty() {
        output.push_str("unspecified");
    } else {
        format_recurrences_to(graph, &node.recurrence, output);
    }

    output.push_str("; space=");
    format_anchor_dimension_to(graph, node.space.as_ref(), &node.space_path, output);

    output.push_str("; spatial-aspect=");
    format_aspect_dimension_to(
        graph,
        node.spatial_aspect.as_ref(),
        &node.spatial_aspects,
        output,
    );

    output.push_str("; spatial-recurrence=");
    if node.spatial_recurrence.is_empty() {
        output.push_str("unspecified");
    } else {
        format_recurrences_to(graph, &node.spatial_recurrence, output);
    }

    format_eventuality_details_to(graph, node, output);
}

#[requires(primary.is_none_or(|relation| !relation.relation.is_empty()))]
#[requires(path.iter().all(|step| !step.relation.is_empty()))]
#[ensures(true)]
fn format_anchor_dimension_to(
    graph: &SemanticGraph,
    primary: Option<&crate::model::AnchorRelation>,
    path: &[crate::model::TemporalPathStep],
    output: &mut String,
) {
    match (primary, path.is_empty()) {
        (None, true) => output.push_str("unspecified"),
        (Some(primary), true) => format_anchor_relation_to(graph, primary, output),
        (None, false) => format_path_to(graph, path, output),
        (Some(primary), false) => {
            output.push_str("combined(primary=");
            format_anchor_relation_to(graph, primary, output);
            output.push_str("; path=");
            format_path_to(graph, path, output);
            output.push(')');
        }
    }
}

#[requires(primary.is_none_or(|aspect| !aspect.contour.is_empty()))]
#[requires(aspects.iter().all(|aspect| !aspect.contour.is_empty()))]
#[ensures(true)]
fn format_aspect_dimension_to(
    graph: &SemanticGraph,
    primary: Option<&crate::model::Aspect>,
    aspects: &[crate::model::Aspect],
    output: &mut String,
) {
    match (primary, aspects.is_empty()) {
        (None, true) => output.push_str("unspecified"),
        (Some(primary), true) => format_aspect_to(graph, primary, output),
        (None, false) => format_aspects_to(graph, aspects, output),
        (Some(primary), false) => {
            output.push_str("combined(primary=");
            format_aspect_to(graph, primary, output);
            output.push_str("; aspects=");
            format_aspects_to(graph, aspects, output);
            output.push(')');
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn format_eventuality_details_to(
    graph: &SemanticGraph,
    node: &crate::model::EventualityNode,
    output: &mut String,
) {
    let has_details = node.tense_modal.is_some()
        || node.time_interval.is_some()
        || node.time_span.is_some()
        || !node.interval_modifiers.is_empty()
        || node.space_interval.is_some()
        || !node.spatial_interval_modifiers.is_empty()
        || node.content.is_some();
    output.push_str("; details=");
    if !has_details {
        output.push_str("unspecified");
        return;
    }

    output.push('{');
    let mut first = true;
    if let Some(tense_modal) = node.tense_modal {
        format_detail_separator(&mut first, output);
        let _ = write!(output, "tense-modal={}", referent_label(graph, tense_modal));
    }
    if let Some(interval) = &node.time_interval {
        format_detail_separator(&mut first, output);
        output.push_str("time-interval=");
        format_time_interval_to(graph, interval, output);
    }
    if let Some(span) = &node.time_span {
        format_detail_separator(&mut first, output);
        output.push_str("time-span=");
        format_time_span_to(graph, span, output);
    }
    if !node.interval_modifiers.is_empty() {
        format_detail_separator(&mut first, output);
        output.push_str("interval-modifiers=");
        format_interval_modifiers_to(graph, &node.interval_modifiers, output);
    }
    if let Some(interval) = &node.space_interval {
        format_detail_separator(&mut first, output);
        output.push_str("space-interval=");
        format_space_interval_to(graph, interval, output);
    }
    if !node.spatial_interval_modifiers.is_empty() {
        format_detail_separator(&mut first, output);
        output.push_str("spatial-interval-modifiers=");
        format_interval_modifiers_to(graph, &node.spatial_interval_modifiers, output);
    }
    if let Some(content) = node.content {
        format_detail_separator(&mut first, output);
        let _ = write!(output, "content={content}");
    }
    format_detail_separator(&mut first, output);
    output.push_str("otherwise=unspecified}");
}

#[requires(true)]
#[ensures(!*first)]
fn format_detail_separator(first: &mut bool, output: &mut String) {
    if *first {
        *first = false;
    } else {
        output.push_str("; ");
    }
}

#[requires(!relation.relation.is_empty())]
#[ensures(true)]
fn format_anchor_relation_to(
    graph: &SemanticGraph,
    relation: &crate::model::AnchorRelation,
    output: &mut String,
) {
    let _ = write!(
        output,
        "{}(anchor={}; sticky={}",
        relation.relation,
        referent_label(graph, relation.anchor),
        relation.sticky
    );
    format_relation_details_to(
        graph,
        relation.inherited,
        relation.distance.as_deref(),
        relation.magnitude.as_ref(),
        relation.scalar_negation.as_ref(),
        relation.motion.as_ref(),
        output,
    );
    output.push(')');
}

#[requires(!magnitude.introduced_by.is_empty())]
#[ensures(true)]
fn format_anchor_magnitude_to(
    graph: &SemanticGraph,
    magnitude: &crate::model::AnchorMagnitude,
    output: &mut String,
) {
    let _ = write!(
        output,
        "{}(introduced-by={})",
        referent_label(graph, magnitude.value),
        magnitude.introduced_by
    );
}

#[requires(!negation.introduced_by.is_empty())]
#[ensures(true)]
fn format_scalar_negation_to(
    graph: &SemanticGraph,
    negation: &crate::model::ScalarNegation,
    output: &mut String,
) {
    let _ = write!(
        output,
        "{}(introduced-by={}",
        scalar_negation_label(negation.kind),
        negation.introduced_by
    );
    output.push_str("; scale=");
    if let Some(scale) = negation.scale {
        output.push_str(&referent_label(graph, scale));
    } else {
        output.push_str("unspecified");
    }
    output.push_str("; argument-scope=");
    if negation.argument_scope.is_empty() {
        output.push_str("unspecified");
    } else {
        output.push('[');
        for (index, place) in negation.argument_scope.iter().enumerate() {
            if index > 0 {
                output.push_str(", ");
            }
            let _ = write!(output, "x{}", place.get());
        }
        output.push(']');
    }
    output.push(')');
}

#[requires(!path.is_empty())]
#[ensures(true)]
fn format_path_to(
    graph: &SemanticGraph,
    path: &[crate::model::TemporalPathStep],
    output: &mut String,
) {
    output.push_str("path[");
    for (index, step) in path.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        format_path_step_to(graph, step, output);
    }
    output.push(']');
}

#[requires(!step.relation.is_empty())]
#[requires(!step.introduced_by.is_empty())]
#[ensures(true)]
fn format_path_step_to(
    graph: &SemanticGraph,
    step: &crate::model::TemporalPathStep,
    output: &mut String,
) {
    let _ = write!(output, "{}(anchor=", step.relation);
    match step.anchor.kind {
        crate::model::TemporalPathAnchorKind::Object => output.push_str(&referent_label(
            graph,
            step.anchor
                .value
                .expect("object temporal path anchors have values"),
        )),
        crate::model::TemporalPathAnchorKind::Previous => output.push_str("previous"),
    }
    let _ = write!(
        output,
        "; introduced-by={}; sticky={}",
        step.introduced_by, step.sticky
    );
    format_relation_details_to(
        graph,
        step.inherited,
        step.distance.as_deref(),
        step.magnitude.as_ref(),
        step.scalar_negation.as_ref(),
        step.motion.as_ref(),
        output,
    );
    output.push(')');
}

#[requires(true)]
#[ensures(true)]
fn format_relation_details_to(
    graph: &SemanticGraph,
    inherited: Option<bool>,
    distance: Option<&str>,
    magnitude: Option<&crate::model::AnchorMagnitude>,
    scalar_negation: Option<&crate::model::ScalarNegation>,
    motion: Option<&crate::model::SpatialMotion>,
    output: &mut String,
) {
    let has_details = inherited.is_some()
        || distance.is_some()
        || magnitude.is_some()
        || scalar_negation.is_some()
        || motion.is_some();
    output.push_str("; details=");
    if !has_details {
        output.push_str("unspecified");
        return;
    }
    output.push('{');
    let mut first = true;
    if let Some(inherited) = inherited {
        format_detail_separator(&mut first, output);
        let _ = write!(output, "inherited={inherited}");
    }
    if let Some(distance) = distance {
        format_detail_separator(&mut first, output);
        let _ = write!(output, "distance={distance}");
    }
    if let Some(magnitude) = magnitude {
        format_detail_separator(&mut first, output);
        output.push_str("magnitude=");
        format_anchor_magnitude_to(graph, magnitude, output);
    }
    if let Some(negation) = scalar_negation {
        format_detail_separator(&mut first, output);
        output.push_str("scalar-negation=");
        format_scalar_negation_to(graph, negation, output);
    }
    if let Some(motion) = motion {
        format_detail_separator(&mut first, output);
        let _ = write!(
            output,
            "motion={}(introduced-by={})",
            spatial_motion_label(motion.kind),
            motion.introduced_by
        );
    }
    format_detail_separator(&mut first, output);
    output.push_str("otherwise=unspecified}");
}

#[requires(!interval.extent.is_empty())]
#[ensures(true)]
fn format_time_interval_to(
    graph: &SemanticGraph,
    interval: &crate::model::TimeInterval,
    output: &mut String,
) {
    let _ = write!(output, "{}(anchor=", interval.extent);
    if let Some(anchor) = interval.anchor {
        output.push_str(&referent_label(graph, anchor));
    } else {
        output.push_str("unspecified");
    }
    output.push(')');
}

#[requires(!span.introduced_by.is_empty())]
#[ensures(true)]
fn format_time_span_to(graph: &SemanticGraph, span: &crate::model::TimeSpan, output: &mut String) {
    let _ = write!(output, "span(introduced-by={}; start=", span.introduced_by);
    format_time_span_endpoint_to(graph, &span.start, output);
    output.push_str("; end=");
    format_time_span_endpoint_to(graph, &span.end, output);
    output.push(')');
}

#[requires(!endpoint.relation.is_empty())]
#[requires(!endpoint.introduced_by.is_empty())]
#[ensures(true)]
fn format_time_span_endpoint_to(
    graph: &SemanticGraph,
    endpoint: &crate::model::TimeSpanEndpoint,
    output: &mut String,
) {
    let _ = write!(output, "{}(anchor=", endpoint.relation);
    if let Some(anchor) = endpoint.anchor {
        output.push_str(&referent_label(graph, anchor));
    } else {
        output.push_str("unspecified");
    }
    let _ = write!(
        output,
        "; introduced-by={}; details=",
        endpoint.introduced_by
    );
    if endpoint.distance.is_none() && endpoint.scalar_negation.is_none() {
        output.push_str("unspecified)");
        return;
    }
    output.push('{');
    let mut first = true;
    if let Some(distance) = &endpoint.distance {
        format_detail_separator(&mut first, output);
        let _ = write!(output, "distance={distance}");
    }
    if let Some(negation) = &endpoint.scalar_negation {
        format_detail_separator(&mut first, output);
        output.push_str("scalar-negation=");
        format_scalar_negation_to(graph, negation, output);
    }
    format_detail_separator(&mut first, output);
    output.push_str("otherwise=unspecified})");
}

#[requires(!aspect.contour.is_empty())]
#[ensures(true)]
fn format_aspect_to(graph: &SemanticGraph, aspect: &crate::model::Aspect, output: &mut String) {
    let _ = write!(output, "{}(anchor=", aspect.contour);
    if let Some(anchor) = aspect.anchor {
        output.push_str(&referent_label(graph, anchor));
    } else {
        output.push_str("unspecified");
    }
    output.push_str("; scalar-negation=");
    if let Some(negation) = &aspect.scalar_negation {
        format_scalar_negation_to(graph, negation, output);
    } else {
        output.push_str("unspecified");
    }
    output.push(')');
}

#[requires(!aspects.is_empty())]
#[ensures(true)]
fn format_aspects_to(graph: &SemanticGraph, aspects: &[crate::model::Aspect], output: &mut String) {
    output.push('[');
    for (index, aspect) in aspects.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        format_aspect_to(graph, aspect, output);
    }
    output.push(']');
}

#[requires(!recurrences.is_empty())]
#[ensures(true)]
fn format_recurrences_to(
    graph: &SemanticGraph,
    recurrences: &[crate::model::Recurrence],
    output: &mut String,
) {
    output.push('[');
    for (index, recurrence) in recurrences.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        format_recurrence_to(graph, recurrence, output);
    }
    output.push(']');
}

#[requires(!recurrence.introduced_by.is_empty())]
#[ensures(true)]
fn format_recurrence_to(
    graph: &SemanticGraph,
    recurrence: &crate::model::Recurrence,
    output: &mut String,
) {
    let _ = write!(
        output,
        "{}(introduced-by={}; details=",
        recurrence_kind_label(recurrence.kind),
        recurrence.introduced_by
    );
    let has_details = recurrence.connection.is_some()
        || recurrence.quantity.is_some()
        || recurrence.value.is_some()
        || recurrence.interval.is_some()
        || recurrence.negation.is_some();
    if !has_details {
        output.push_str("unspecified)");
        return;
    }
    output.push('{');
    let mut first = true;
    if let Some(connection) = &recurrence.connection {
        format_detail_separator(&mut first, output);
        let _ = write!(
            output,
            "connection={}(introduced-by={})",
            recurrence_connection_label(connection.kind),
            connection.introduced_by
        );
    }
    if let Some(quantity) = recurrence.quantity {
        format_detail_separator(&mut first, output);
        let _ = write!(output, "quantity={quantity}");
    }
    if let Some(value) = &recurrence.value {
        format_detail_separator(&mut first, output);
        output.push_str("value=");
        format_quantity_value_to(value, output);
    }
    if let Some(interval) = recurrence.interval {
        format_detail_separator(&mut first, output);
        let _ = write!(output, "interval={}", referent_label(graph, interval));
    }
    if let Some(negation) = &recurrence.negation {
        format_detail_separator(&mut first, output);
        let _ = write!(
            output,
            "negation={}(introduced-by={})",
            modal_negation_label(negation.kind),
            negation.introduced_by
        );
    }
    format_detail_separator(&mut first, output);
    output.push_str("otherwise=unspecified})");
}

#[requires(true)]
#[ensures(true)]
fn format_quantity_value_to(value: &crate::model::QuantityValue, output: &mut String) {
    if let Some(integer) = value.integer {
        let _ = write!(output, "integer({integer})");
    } else if let Some(text) = &value.text {
        let _ = write!(output, "text({text:?})");
    } else if let Some(expression) = value.math_expression {
        let _ = write!(output, "math-expression({expression})");
    } else {
        unreachable!("quantity values have exactly one representation");
    }
}

#[requires(!modifiers.is_empty())]
#[ensures(true)]
fn format_interval_modifiers_to(
    graph: &SemanticGraph,
    modifiers: &[crate::model::IntervalModifier],
    output: &mut String,
) {
    output.push('[');
    for (index, modifier) in modifiers.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        match modifier.as_data() {
            data!(crate::model::IntervalModifier::Aspect(aspect)) => {
                output.push_str("aspect(");
                format_aspect_to(graph, aspect, output);
                output.push(')');
            }
            data!(crate::model::IntervalModifier::Recurrence(recurrence)) => {
                output.push_str("recurrence(");
                format_recurrence_to(graph, recurrence, output);
                output.push(')');
            }
        }
    }
    output.push(']');
}

#[requires(interval.extent.is_some() || !interval.directions.is_empty() || !interval.dimensions.is_empty())]
#[ensures(true)]
fn format_space_interval_to(
    graph: &SemanticGraph,
    interval: &crate::model::SpaceInterval,
    output: &mut String,
) {
    output.push_str("interval(extent=");
    if let Some(extent) = &interval.extent {
        output.push_str(extent);
    } else {
        output.push_str("unspecified");
    }
    output.push_str("; directions=");
    format_string_list_to(&interval.directions, output);
    output.push_str("; dimensions=");
    format_string_list_to(&interval.dimensions, output);
    output.push_str("; anchor=");
    if let Some(anchor) = interval.anchor {
        output.push_str(&referent_label(graph, anchor));
    } else {
        output.push_str("unspecified");
    }
    output.push(')');
}

#[requires(values.iter().all(|value| !value.is_empty()))]
#[ensures(true)]
fn format_string_list_to(values: &[String], output: &mut String) {
    if values.is_empty() {
        output.push_str("unspecified");
        return;
    }
    output.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            output.push_str(", ");
        }
        output.push_str(value);
    }
    output.push(']');
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
    formula_reference_label_with_event_conditions(graph, formula, true)
}

#[requires(formula.object_kind() == SemanticObjectKind::Formula)]
#[ensures(!ret.is_empty())]
fn formula_reference_label_with_event_conditions(
    graph: &SemanticGraph,
    formula: SemanticObjectId,
    include_event_conditions: bool,
) -> String {
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
    format_predication_with_event_conditions(
        graph,
        predication_id,
        predication,
        include_event_conditions,
    )
}

#[requires(id.object_kind() == SemanticObjectKind::Formula)]
#[requires(graph.objects.contains_key(&id))]
#[ensures(!ret.is_empty())]
fn formula_tree_label(graph: &SemanticGraph, id: SemanticObjectId, node: &FormulaNode) -> String {
    formula_tree_label_with_event_conditions(graph, id, node, true, true)
}

#[requires(id.object_kind() == SemanticObjectKind::Formula)]
#[requires(graph.objects.contains_key(&id))]
#[ensures(!ret.is_empty())]
fn formula_tree_label_with_event_conditions(
    graph: &SemanticGraph,
    id: SemanticObjectId,
    node: &FormulaNode,
    include_event_use_conditions: bool,
    include_binding_conditions: bool,
) -> String {
    let mut base = match node.as_data() {
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
    if let data!(FormulaNode::Connective(node)) = node.as_data()
        && let Some(eventuality) = node.eventuality
    {
        format_eventuality_site_with_conditions_to(
            graph,
            eventuality,
            "event",
            include_event_use_conditions,
            &mut base,
        );
    }
    let binding = event_binding_label_with_conditions(
        graph,
        graph
            .objects
            .get(&id)
            .expect("formula label requires a defined formula")
            .bound_eventualities(),
        include_binding_conditions,
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
    event_binding_label_with_conditions(graph, eventualities, true)
}

#[requires(eventualities.iter().all(|eventuality| graph.objects.get(&eventuality.object_id()).is_some_and(SemanticObject::is_generated_eventuality)))]
#[ensures(ret.is_empty() == eventualities.is_empty())]
fn event_binding_label_with_conditions(
    graph: &SemanticGraph,
    eventualities: &[GeneratedEventualityId],
    include_conditions: bool,
) -> String {
    let mut output = String::new();
    for eventuality in eventualities {
        if !output.is_empty() {
            output.push_str(", ");
        }
        let eventuality = eventuality.object_id();
        output.push_str(&referent_label(graph, eventuality));
        if include_conditions {
            output.push_str(" {");
            format_eventuality_conditions_to(graph, eventuality, &mut output);
            output.push('}');
        }
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

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|label| !label.is_empty()))]
fn formula_scope_operator_label(graph: &SemanticGraph, node: &FormulaNode) -> Option<String> {
    match node.as_data() {
        data!(FormulaNode::Atom(_)) => None,
        data!(FormulaNode::Connective(node)) => {
            Some(formula_operator_label(node.operator).to_owned())
        }
        data!(FormulaNode::Quantified(node)) => Some(format!(
            "{} {}",
            formula_operator_label(node.operator),
            referent_label(graph, node.variable)
        )),
        data!(FormulaNode::QuantifierBundle(node)) => {
            let mut label = "quantifier-bundle".to_owned();
            for (index, binding) in node.bindings.iter().enumerate() {
                if index == 0 {
                    label.push(' ');
                } else {
                    label.push_str(", ");
                }
                label.push_str(&referent_label(graph, binding.variable));
            }
            Some(label)
        }
        data!(FormulaNode::RespectivelyDistribution(node)) => {
            let mut label = "respectively-distribution".to_owned();
            for (index, stream) in node.streams.iter().enumerate() {
                if index == 0 {
                    label.push(' ');
                } else {
                    label.push_str(", ");
                }
                label.push_str(&referent_label(graph, stream.slot));
            }
            Some(label)
        }
    }
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

#[requires(true)]
#[ensures(!ret.is_empty())]
fn actuality_kind_label(kind: crate::model::ActualityKind) -> &'static str {
    match kind {
        crate::model::ActualityKind::Actual => "actual",
        crate::model::ActualityKind::Capable => "capable",
        crate::model::ActualityKind::Potential => "potential",
        crate::model::ActualityKind::Demonstrated => "demonstrated",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn spatial_motion_label(kind: crate::model::SpatialMotionKind) -> &'static str {
    match kind {
        crate::model::SpatialMotionKind::Toward => "toward",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn recurrence_kind_label(kind: crate::model::RecurrenceKind) -> &'static str {
    match kind {
        crate::model::RecurrenceKind::OccurrenceCount => "occurrence-count",
        crate::model::RecurrenceKind::OrdinalOccurrence => "ordinal-occurrence",
        crate::model::RecurrenceKind::Regular => "regular",
        crate::model::RecurrenceKind::Typically => "typically",
        crate::model::RecurrenceKind::Continuously => "continuously",
        crate::model::RecurrenceKind::Habitually => "habitually",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn recurrence_connection_label(kind: crate::model::RecurrenceConnectionKind) -> &'static str {
    match kind {
        crate::model::RecurrenceConnectionKind::Product => "product",
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn modal_negation_label(kind: crate::model::ModalNegationKind) -> &'static str {
    match kind {
        crate::model::ModalNegationKind::Contradictory => "contradictory",
        crate::model::ModalNegationKind::OtherThan => "other-than",
    }
}
