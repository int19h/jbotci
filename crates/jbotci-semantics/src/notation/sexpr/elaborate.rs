//! Conservative semantic elaboration from typed graph objects to compact data.
//!
//! Every recognizer in this module is an exact structural proof. A branch that
//! cannot account for every semantic field returns the local typed fallback;
//! there are no spelling guesses or best-effort reconstructions.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, invariant, new, requires};

use super::datum::{Atom, Datum};
use super::planner::ReferencePlan;
use super::structural::{ProvenanceDisposition, object_datum_with_variables};
use crate::model::{
    AbstractionKind, ActualityKind, Adjunct, ArgumentValue, ArgumentValueKind, CompositionOperator,
    DeicticProximity, DescriptorKind, EventualityDenotationData, EventualityNode, EventualitySort,
    FormulaNodeData, FormulaOperator, IndexicalKind, MathExpressionNodeKindData, MathOperatorData,
    ParagraphTransition, ParameterRole, PlaceIndex, PredicationMode, PredicationNode,
    PredicationRelationData, QuantityForm, QuantityNode, QuantityScale, QuestionKind, QuestionMode,
    QuestionSlotData, QuestionSlotRole, ReferentCategory, ReferentNode, RelationLabelData,
    RelativeClauseKind, ScopeDependenceData, SemanticGraph, SemanticObjectData, SemanticObjectId,
    SemanticObjectKind, SemanticSort, SequenceNode, SequenceRelation, SignNode, UtteranceForce,
    UtteranceNode, semantic_scope_dependence_binder_universes,
};

/// Mutable counters kept separate from semantic rendering decisions.
#[invariant(true)]
#[derive(Debug, Default)]
struct ElaborationCounters {
    compact_objects: Cell<usize>,
    object_fallbacks: Cell<usize>,
    fallback_reasons: RefCell<BTreeMap<&'static str, usize>>,
    requires_typed_graph: Cell<bool>,
}

/// Predicate term plus its ordered application operands. Keeping the head
/// separate prevents a structured operator such as `(DropPlace p 2)` from
/// being mistaken for an already flattened application.
#[invariant(true)]
#[derive(Debug)]
struct PredicateApplication {
    head: Datum,
    operands: Vec<Datum>,
}

/// One fixed reference computation collected at its enclosing force segment.
/// Version 0 deliberately keeps these distinct from pure `Let` values.
#[invariant(true)]
#[derive(Debug)]
struct ReferenceBinding {
    id: SemanticObjectId,
    declared_type: Datum,
    computation: Datum,
}

/// Description constructors whose graph encodings are inverted into lexical
/// binders by one shared recognition proof.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DescriptionConstructor {
    Lo,
    Le,
}

/// Closed reasons for declining an old compact recognizer. Until a local
/// boundary has a proved v0 type, each cause selects a registered
/// whole-document fallback rather than emitting an untyped pseudo-form.
#[invariant(::UnrecognizedObjectFamily(_) => true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompactFallbackCause {
    UnrecognizedObjectFamily(SemanticObjectKind),
    UtteranceWithoutContent,
    ForceFieldsRequireRecord,
    SequenceFields,
    ConnectiveMetadata,
    UnrecognizedConnective,
    QuantifierVariableFields,
    QuantifierFields,
    RespectivelySlotFields,
    PredicationSideFields,
    PredicationModeUnrepresentable,
    NonAtomicRelation,
    CompositionPredication,
    ArgumentFields,
    AdjunctFields,
    CompositionFields,
    ConstantWithoutDependence,
    ReferentFields,
    UnboundGeneratedEvent,
    UnrepresentableRecursiveValue,
    EventualityFacets,
    QuestionSlotFields,
    MathSideFields,
    QuantityFields,
    MathLiteralDenotes,
    MathOperatorFields,
    SignFields,
}

impl CompactFallbackCause {
    /// Exact registered reason used by the current conservative boundary.
    #[requires(true)]
    #[ensures(ret.starts_with("smusni.fallback."))]
    fn reason_id(self) -> &'static str {
        match self {
            Self::UnrecognizedObjectFamily(kind) => match kind {
                SemanticObjectKind::DisplayedContent | SemanticObjectKind::Utterance => {
                    "smusni.fallback.force-reduction-unrepresentable"
                }
                SemanticObjectKind::Parameter => "smusni.fallback.higher-order-crossing-unlicensed",
                SemanticObjectKind::RelationMetadata => {
                    "smusni.fallback.lexical-signature-missing-or-stale"
                }
                _ => "smusni.fallback.relation-reduction-unregistered-or-inexact",
            },
            Self::UtteranceWithoutContent
            | Self::ForceFieldsRequireRecord
            | Self::SequenceFields => "smusni.fallback.force-reduction-unrepresentable",
            Self::ConnectiveMetadata
            | Self::UnrecognizedConnective
            | Self::PredicationSideFields
            | Self::PredicationModeUnrepresentable
            | Self::NonAtomicRelation => {
                "smusni.fallback.relation-reduction-unregistered-or-inexact"
            }
            Self::QuantifierVariableFields | Self::QuantifierFields => {
                "smusni.fallback.quantifier-effect-export-illegal"
            }
            Self::RespectivelySlotFields => "smusni.fallback.simultaneous-termset-unlicensed",
            Self::CompositionPredication | Self::CompositionFields => {
                "smusni.fallback.relation-former-reduction-unavailable"
            }
            Self::ArgumentFields => "smusni.fallback.predicate-fill-type-or-arity-mismatch",
            Self::AdjunctFields => "smusni.fallback.modal-tag-reduction-unregistered",
            Self::ConstantWithoutDependence | Self::ReferentFields => {
                "smusni.fallback.reference-description-unrepresentable"
            }
            Self::UnboundGeneratedEvent => "smusni.fallback.generated-eventuality-unbound",
            Self::UnrepresentableRecursiveValue => {
                "smusni.fallback.unguarded-or-unrepresentable-scc"
            }
            Self::EventualityFacets => "smusni.fallback.event-facet-reduction-unregistered",
            Self::QuestionSlotFields => "smusni.fallback.question-domain-or-answer-mismatch",
            Self::QuantityFields => "smusni.fallback.quantity-reduction-unregistered",
            Self::MathSideFields | Self::MathLiteralDenotes | Self::MathOperatorFields => {
                "smusni.fallback.math-reduction-unregistered"
            }
            Self::SignFields => "smusni.fallback.sign-identity-missing",
        }
    }
}

/// Complete result of exact descriptor recognition. Planning and rendering
/// consume this same value so support is never projected before the renderer
/// has proved the corresponding compact constructor.
#[invariant(::Property { constructor, property, parameter } => !property.is_empty() && match constructor {
    DescriptionConstructor::Lo => parameter.is_none(),
    DescriptionConstructor::Le => parameter.as_ref().is_some_and(|id| id.object_kind() == SemanticObjectKind::Parameter),
})]
#[invariant(::Name { name } => !name.is_empty())]
#[derive(Debug, Clone, Copy)]
enum DescriptionRecognition<'a> {
    Property {
        constructor: DescriptionConstructor,
        property: &'a str,
        parameter: Option<SemanticObjectId>,
    },
    Name {
        name: &'a str,
    },
}

impl PredicateApplication {
    /// Add one ordinary application operand in source-independent semantic order.
    #[requires(true)]
    #[ensures(self.operands.len() == old(self.operands.len()) + 1)]
    fn push(&mut self, operand: Datum) {
        self.operands.push(operand);
    }

    /// Convert the typed application to canonical datum shape.
    #[requires(true)]
    #[ensures(true)]
    fn into_datum(self) -> Datum {
        if self.operands.is_empty() {
            return self.head;
        }
        Datum::list(std::iter::once(self.head).chain(self.operands))
    }
}

/// Nest single-entry dynamic bindings in discovery order. The first reference
/// discovered is the outermost handler, matching left-to-right evaluation.
#[requires(true)]
#[ensures(true)]
fn wrap_reference_bindings(bindings: Vec<ReferenceBinding>, mut body: Datum) -> Datum {
    for binding in bindings.into_iter().rev() {
        body = Datum::form(
            "Bind",
            [
                Datum::list([Datum::list([
                    variable_datum(binding.id),
                    binding.declared_type,
                    binding.computation,
                ])]),
                body,
            ],
        );
    }
    body
}

/// Result of one compact elaboration pass.
#[invariant(true)]
#[derive(Debug, Clone)]
pub(super) struct CompactElaboration {
    pub(super) body: Datum,
    pub(super) compact_objects: usize,
    pub(super) object_fallbacks: usize,
    pub(super) fallback_reasons: BTreeMap<&'static str, usize>,
    pub(super) requires_typed_graph: bool,
}

/// Read-only elaborator over a validated semantic graph.
#[invariant(true)]
#[derive(Debug)]
struct Elaborator<'a> {
    graph: &'a SemanticGraph,
    plan: &'a ReferencePlan,
    definitions: BTreeSet<SemanticObjectId>,
    projected_descriptions: BTreeSet<SemanticObjectId>,
    binder_universes: BTreeMap<SemanticObjectId, BTreeSet<SemanticObjectId>>,
    needed_definitions: RefCell<BTreeSet<SemanticObjectId>>,
    placed_definitions: RefCell<BTreeSet<SemanticObjectId>>,
    absorbed_eventualities: RefCell<BTreeSet<SemanticObjectId>>,
    reference_binding_frames: RefCell<Vec<Vec<ReferenceBinding>>>,
    counters: ElaborationCounters,
}

/// Elaborate the compact document body, including deterministic shared-value
/// declarations when needed.
#[requires(graph.objects.contains_key(&graph.root))]
#[requires(plan.compact_is_eligible())]
#[ensures(true)]
pub(super) fn elaborate_compact(graph: &SemanticGraph, plan: &ReferencePlan) -> CompactElaboration {
    let (mut projected_description_support, mut description_values) =
        projected_description_objects(graph, plan);
    let (event_support, described_events) = projected_described_event_objects(graph, plan);
    projected_description_support.extend(event_support);
    description_values.extend(described_events);
    let projected_use_counts =
        reference_counts_excluding_sources(graph, &projected_description_support);
    let definitions = graph
        .objects
        .iter()
        .filter_map(|(id, object)| {
            if plan.binder_owner(*id).is_some()
                || is_conventional_atom(object)
                || object
                    .as_referent()
                    .is_some_and(|node| exact_deictic(node, graph).is_some())
                || projected_description_support.contains(id)
            {
                return None;
            }
            let needs_definition = if description_values.contains(id) {
                projected_use_counts.get(id).copied().unwrap_or(0) > 1
            } else {
                plan.use_count(*id) > 1 || plan.is_cyclic(*id)
            };
            needs_definition.then_some(*id)
        })
        .collect();
    let elaborator = Elaborator {
        graph,
        plan,
        definitions,
        projected_descriptions: description_values,
        binder_universes: semantic_scope_dependence_binder_universes(graph.root, &graph.objects),
        needed_definitions: RefCell::new(BTreeSet::new()),
        placed_definitions: RefCell::new(BTreeSet::new()),
        absorbed_eventualities: RefCell::new(BTreeSet::new()),
        reference_binding_frames: RefCell::new(Vec::new()),
        counters: ElaborationCounters::default(),
    };
    let body = elaborator.render_with_definitions();
    CompactElaboration {
        body,
        compact_objects: elaborator.counters.compact_objects.get(),
        object_fallbacks: elaborator.counters.object_fallbacks.get(),
        fallback_reasons: elaborator.counters.fallback_reasons.into_inner(),
        requires_typed_graph: elaborator.counters.requires_typed_graph.get(),
    }
}

/// Count edge multiplicities after removing identities consumed inside exact
/// projections. One reusable edge buffer keeps this O(V+E) without allocating
/// a temporary collection per description value or per source object.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.values().all(|count| *count > 0))]
fn reference_counts_excluding_sources(
    graph: &SemanticGraph,
    excluded_sources: &BTreeSet<SemanticObjectId>,
) -> BTreeMap<SemanticObjectId, usize> {
    let mut counts = BTreeMap::new();
    let mut references = Vec::new();
    for (source, object) in &graph.objects {
        if excluded_sources.contains(source) {
            continue;
        }
        references.clear();
        object.references_into(&mut references);
        for target in references.iter().copied() {
            *counts.entry(target).or_default() += 1;
        }
    }
    counts
}

impl Elaborator<'_> {
    /// Render the root and place any definitions whose computed graph site was
    /// projected away at the outermost surviving legal scope.
    #[requires(true)]
    #[ensures(true)]
    fn render_with_definitions(&self) -> Datum {
        let mut active = BTreeSet::new();
        let bound = BTreeSet::new();
        let body = self.render_id(self.graph.root, &bound, &mut active, None);
        let remaining = self
            .needed_definitions
            .borrow()
            .difference(&self.placed_definitions.borrow())
            .copied()
            .collect::<BTreeSet<_>>();
        self.wrap_definitions(remaining, body, &bound, &mut active)
    }

    /// Place a deterministic definition group around one represented scope.
    #[requires(definitions.is_subset(&self.definitions))]
    #[ensures(true)]
    fn wrap_definitions(
        &self,
        definitions: BTreeSet<SemanticObjectId>,
        body: Datum,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Datum {
        let definitions = self.definition_dependency_closure(definitions);
        if definitions.is_empty() {
            return body;
        }
        self.placed_definitions
            .borrow_mut()
            .extend(definitions.iter().copied());
        let (ordered, recursive) = self.definition_order(&definitions);
        let bindings = ordered
            .into_iter()
            .map(|id| {
                let value = self.render_object(id, bound, active, None);
                Datum::list([
                    variable_datum(id),
                    definition_type_datum(&self.graph.objects[&id]),
                    value,
                ])
            })
            .collect::<Vec<_>>();
        if recursive
            && bindings.iter().any(|binding| {
                binding
                    .as_list()
                    .and_then(|fields| fields.get(2))
                    .and_then(Datum::form_head)
                    != Some("λ")
            })
        {
            self.counters.requires_typed_graph.set(true);
            *self
                .counters
                .fallback_reasons
                .borrow_mut()
                .entry(CompactFallbackCause::UnrepresentableRecursiveValue.reason_id())
                .or_default() += 1;
            return body;
        }
        Datum::form(
            if recursive { "LetRec" } else { "Let" },
            [Datum::list(bindings), body],
        )
    }

    /// Include shared definitions referenced by binding values. This is a
    /// graph-level over-approximation; the value renderer may consume some of
    /// those edges through a named compact projection, but retaining an extra
    /// typed binding never changes the denotation.
    #[requires(definitions.is_subset(&self.definitions))]
    #[ensures(ret.is_subset(&self.definitions))]
    fn definition_dependency_closure(
        &self,
        definitions: BTreeSet<SemanticObjectId>,
    ) -> BTreeSet<SemanticObjectId> {
        let mut closure = definitions;
        loop {
            let dependencies = closure
                .iter()
                .copied()
                .flat_map(|id| self.definition_dependencies(id))
                .collect::<BTreeSet<_>>();
            let previous_len = closure.len();
            closure.extend(dependencies);
            if closure.len() == previous_len {
                return closure;
            }
        }
    }

    /// Topologically order one local group. A remaining dependency cycle
    /// selects `LetRec` for the complete group.
    #[requires(definitions.is_subset(&self.definitions))]
    #[ensures(ret.0.len() == definitions.len())]
    fn definition_order(
        &self,
        definitions: &BTreeSet<SemanticObjectId>,
    ) -> (Vec<SemanticObjectId>, bool) {
        let mut emitted = BTreeSet::new();
        let mut ordered = Vec::new();
        loop {
            let next = definitions.iter().copied().find(|id| {
                !emitted.contains(id)
                    && self
                        .definition_dependencies(*id)
                        .into_iter()
                        .filter(|dependency| definitions.contains(dependency))
                        .all(|dependency| emitted.contains(&dependency))
            });
            let Some(next) = next else { break };
            emitted.insert(next);
            ordered.push(next);
        }
        let recursive = emitted.len() != definitions.len();
        if recursive {
            ordered.extend(
                definitions
                    .iter()
                    .copied()
                    .filter(|id| !emitted.contains(id)),
            );
        }
        (ordered, recursive)
    }

    /// Typed outgoing references for dependency ordering.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn object_references(&self, id: SemanticObjectId) -> Vec<SemanticObjectId> {
        let mut references = Vec::new();
        self.graph.objects[&id].references_into(&mut references);
        references
    }

    /// Shared definitions reached through inlined, single-use graph objects.
    /// This matches the dependencies of the eventual binding value without
    /// making compact recognition itself responsible for declaration order.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(ret.is_subset(&self.definitions))]
    fn definition_dependencies(&self, id: SemanticObjectId) -> BTreeSet<SemanticObjectId> {
        let mut dependencies = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut pending = self.object_references(id);
        while let Some(reference) = pending.pop() {
            if self.definitions.contains(&reference) {
                dependencies.insert(reference);
                continue;
            }
            if visited.insert(reference) {
                pending.extend(self.object_references(reference));
            }
        }
        dependencies
    }

    /// Render an identity as a bound/shared variable or inline semantic value.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_id(
        &self,
        id: SemanticObjectId,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
        expected_mode: Option<PredicationMode>,
    ) -> Datum {
        if bound.contains(&id) {
            return variable_datum(id);
        }
        if self.definitions.contains(&id) {
            self.needed_definitions.borrow_mut().insert(id);
            return variable_datum(id);
        }
        self.render_object(id, bound, active, expected_mode)
    }

    /// Dispatch one graph object through exact typed recognizers.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_object(
        &self,
        id: SemanticObjectId,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
        expected_mode: Option<PredicationMode>,
    ) -> Datum {
        if !active.insert(id) {
            return variable_datum(id);
        }
        let object = &self.graph.objects[&id];
        let value = match object.as_data() {
            data!(SemanticObject::Utterance(node)) => {
                self.render_utterance(id, node, bound, active)
            }
            data!(SemanticObject::Sequence(node)) => self.render_sequence(id, node, bound, active),
            data!(SemanticObject::Predication(node)) => {
                self.render_predication(id, node, bound, active, expected_mode)
            }
            data!(SemanticObject::Formula(node)) => {
                self.render_formula(id, node, bound, active, expected_mode)
            }
            data!(SemanticObject::Referent(node)) => self.render_referent(id, node, bound, active),
            data!(SemanticObject::Eventuality(node)) => {
                self.render_eventuality(id, node, bound, active)
            }
            data!(SemanticObject::Question(node)) => self.render_question(id, node, bound, active),
            data!(SemanticObject::Quantity(node)) => self.render_quantity(id, node, bound, active),
            data!(SemanticObject::MathExpression(node)) => {
                self.render_math(id, node, bound, active)
            }
            data!(SemanticObject::Sign(node)) => self.render_sign(id, node, bound, active),
            data!(SemanticObject::DisplayedContent(_))
            | data!(SemanticObject::Parameter(_))
            | data!(SemanticObject::RelationMetadata(_)) => self.fallback_object(
                id,
                bound,
                active,
                CompactFallbackCause::UnrecognizedObjectFamily(object.object_kind()),
            ),
        };
        active.remove(&id);
        let definitions = self
            .definitions
            .iter()
            .copied()
            .filter(|definition| {
                !self.placed_definitions.borrow().contains(definition)
                    && self.needed_definitions.borrow().contains(definition)
                    && self.plan.definition_site(*definition) == Some(id)
            })
            .collect();
        self.wrap_definitions(definitions, value, bound, active)
    }

    /// Complete object-local fallback with current lexical variables preserved.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn fallback_object(
        &self,
        id: SemanticObjectId,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
        cause: CompactFallbackCause,
    ) -> Datum {
        self.counters.requires_typed_graph.set(true);
        self.counters
            .object_fallbacks
            .set(self.counters.object_fallbacks.get() + 1);
        *self
            .counters
            .fallback_reasons
            .borrow_mut()
            .entry(cause.reason_id())
            .or_default() += 1;
        let object = &self.graph.objects[&id];
        let references = self.fallback_reference_map(id, bound, active);
        object_datum_with_variables(
            object.object_kind(),
            object,
            ProvenanceDisposition::Suppress,
            &references,
        )
    }

    /// Count one successful compact object recognition.
    #[requires(true)]
    #[ensures(self.counters.compact_objects.get() == old(self.counters.compact_objects.get()) + 1)]
    fn recognized(&self, datum: Datum) -> Datum {
        self.counters
            .compact_objects
            .set(self.counters.compact_objects.get() + 1);
        datum
    }

    /// Current structural-fallback mapping from typed IDs to lexical names.
    #[requires(true)]
    #[ensures(ret.len() <= bound.len())]
    fn variable_map(&self, bound: &BTreeSet<SemanticObjectId>) -> BTreeMap<String, Datum> {
        bound
            .iter()
            .map(|id| (id.to_string(), variable_datum(*id)))
            .collect()
    }

    /// Close every direct graph edge in a local typed object. Lexical binders
    /// and planned shared identities become `$` uses; each remaining
    /// single-use acyclic identity is recursively rendered at the exact field
    /// where its typed reference occurred. Consequently local `(Object ...)`
    /// forms never contain an undeclared `@` edge.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    #[expensive_ensures(self.object_references(id).iter().all(|reference| ret.contains_key(&reference.to_string())))]
    fn fallback_reference_map(
        &self,
        id: SemanticObjectId,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> BTreeMap<String, Datum> {
        let mut references = self.variable_map(bound);
        for reference in self.object_references(id) {
            let key = reference.to_string();
            if references.contains_key(&key) {
                continue;
            }
            assert!(
                self.graph.objects.contains_key(&reference),
                "validated semantic graphs close every object reference"
            );
            references.insert(key, self.render_id(reference, bound, active, None));
        }
        references
    }

    /// Render an utterance act, omitting the record only under the named default.
    #[requires(true)]
    #[ensures(true)]
    fn render_utterance(
        &self,
        id: SemanticObjectId,
        node: &UtteranceNode,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Datum {
        let Some(content) = node.content else {
            return self.fallback_object(
                id,
                bound,
                active,
                CompactFallbackCause::UtteranceWithoutContent,
            );
        };
        if matches!(
            node.force,
            UtteranceForce::Quote
                | UtteranceForce::Parenthetical
                | UtteranceForce::Subordinated
                | UtteranceForce::Command
                | UtteranceForce::Vocative
        ) {
            return self.fallback_object(
                id,
                bound,
                active,
                CompactFallbackCause::ForceFieldsRequireRecord,
            );
        }
        let expected_mode =
            (node.force == UtteranceForce::Assert).then_some(PredicationMode::Asserted);
        let content_id = content;
        self.reference_binding_frames.borrow_mut().push(Vec::new());
        let content = self.render_id(content_id, bound, active, expected_mode);
        let bindings = self
            .reference_binding_frames
            .borrow_mut()
            .pop()
            .expect("utterance rendering pushed one reference-binding frame");
        let act = match node.force {
            UtteranceForce::Assert => Datum::form("Assert", [content]),
            UtteranceForce::Ask if self.graph.objects[&content_id].as_question().is_some() => {
                content
            }
            UtteranceForce::Ask => Datum::form("Ask", [content]),
            UtteranceForce::Mention => Datum::form("Mention", [content]),
            UtteranceForce::Quote
            | UtteranceForce::Parenthetical
            | UtteranceForce::Subordinated
            | UtteranceForce::Command
            | UtteranceForce::Vocative => unreachable!(
                "unsupported forces return typed fallback before rendering their content"
            ),
        };
        let act = wrap_reference_bindings(bindings, act);
        if utterance_record_is_default(self.graph, self.plan, id, node) {
            return self.recognized(act);
        }
        if !node.asides.is_empty() || node.vocative_kind.is_some() {
            return self.fallback_object(
                id,
                bound,
                active,
                CompactFallbackCause::ForceFieldsRequireRecord,
            );
        }

        let utterance_variable = variable_datum(id);
        let mut record_bound = bound.clone();
        record_bound.insert(id);
        let mut fields = vec![Datum::list([Datum::list([
            utterance_variable.clone(),
            Datum::atom("UtteranceToken"),
        ])])];
        fields.extend([
            Datum::form(
                "SpeakerOf",
                [
                    utterance_variable.clone(),
                    self.render_id(node.speaker, &record_bound, active, None),
                ],
            ),
            Datum::form(
                "AudienceOf",
                [
                    utterance_variable.clone(),
                    self.render_id(node.audience, &record_bound, active, None),
                ],
            ),
        ]);
        if !default_locution_event(self.graph, node.eventuality)
            || self.plan.use_count(node.eventuality) > 1
        {
            fields.push(Datum::form(
                "LocutionOf",
                [
                    utterance_variable.clone(),
                    self.render_id(node.eventuality, &record_bound, active, None),
                ],
            ));
        }
        if !object_is_indexical(self.graph, node.deictic_ground.time, IndexicalKind::Now) {
            fields.push(Datum::form(
                "DeicticTimeOf",
                [
                    utterance_variable.clone(),
                    self.render_id(node.deictic_ground.time, &record_bound, active, None),
                ],
            ));
        }
        if !object_is_indexical(self.graph, node.deictic_ground.place, IndexicalKind::Here) {
            fields.push(Datum::form(
                "DeicticPlaceOf",
                [
                    utterance_variable.clone(),
                    self.render_id(node.deictic_ground.place, &record_bound, active, None),
                ],
            ));
        }
        fields.push(Datum::form("Realizes", [utterance_variable, act]));
        self.recognized(Datum::form("Utterance", fields))
    }

    /// Render ordinary ordered discourse as `Do`, or the complete typed
    /// sequence surface when relation or side fields make the concise form
    /// inexact.
    #[requires(true)]
    #[ensures(true)]
    fn render_sequence(
        &self,
        id: SemanticObjectId,
        node: &SequenceNode,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Datum {
        let ordinary_fields = node.force.is_none()
            && node.content.is_none()
            && node.connection_claims.is_empty()
            && node.bound_eventualities.is_empty()
            && node.ordinal_labels.is_empty()
            && node.nonlogical_connection.is_none()
            && node.elided_connection_operand.is_none();
        if ordinary_fields {
            match &node.relation {
                SequenceRelation::SameTopicContinuation => {
                    return self.recognized(Datum::form(
                        "Do",
                        node.items
                            .iter()
                            .map(|item| self.render_id(*item, bound, active, None)),
                    ));
                }
                SequenceRelation::ParagraphBoundary {
                    transition,
                    additional,
                } if additional.is_empty() && !node.items.is_empty() => {
                    let rendered = node
                        .items
                        .iter()
                        .map(|item| self.render_id(*item, bound, active, None))
                        .collect::<Vec<_>>();
                    let discourse = if rendered.len() == 1 {
                        let item = node.items[0];
                        match self.graph.objects[&item].object_kind() {
                            SemanticObjectKind::Utterance => {
                                let value = rendered.into_iter().next().expect("one item");
                                if value.form_head() == Some("Utterance") {
                                    Datum::form("PerformUtterance", [value])
                                } else {
                                    Datum::form("Perform", [value])
                                }
                            }
                            SemanticObjectKind::Sequence => {
                                rendered.into_iter().next().expect("one item")
                            }
                            _ => {
                                return self.fallback_object(
                                    id,
                                    bound,
                                    active,
                                    CompactFallbackCause::SequenceFields,
                                );
                            }
                        }
                    } else {
                        Datum::form("Do", rendered)
                    };
                    return self.recognized(Datum::form(
                        match transition {
                            ParagraphTransition::NewTopic => "NewTopic",
                            ParagraphTransition::ResumePriorTopic => "Resume",
                        },
                        [discourse],
                    ));
                }
                _ => {}
            }
        }
        self.fallback_object(id, bound, active, CompactFallbackCause::SequenceFields)
    }

    /// Render the typed formula families, declining whenever connective
    /// metadata is not consumed by an exact rule.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_formula(
        &self,
        id: SemanticObjectId,
        formula: &crate::model::FormulaNode,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
        expected_mode: Option<PredicationMode>,
    ) -> Datum {
        match formula.as_data() {
            data!(FormulaNode::Atom(node)) => {
                let generated = node
                    .bound_eventualities
                    .iter()
                    .map(|event| event.object_id())
                    .collect::<Vec<_>>();
                let mut scoped = bound.clone();
                scoped.extend(generated.iter().copied());
                let body = self.render_id(node.predication, &scoped, active, expected_mode);
                self.recognized(self.bind_generated_events(id, &generated, body, &scoped, active))
            }
            data!(FormulaNode::Connective(node)) => {
                if let Some(value) =
                    self.render_tanru_projection(id, node, bound, active, expected_mode)
                {
                    return self.recognized(value);
                }
                if node.eventuality.is_some() {
                    return self.fallback_object(
                        id,
                        bound,
                        active,
                        CompactFallbackCause::ConnectiveMetadata,
                    );
                }
                let Some((operator, children)) = exact_connective_projection(self.graph, node)
                else {
                    return self.fallback_object(
                        id,
                        bound,
                        active,
                        CompactFallbackCause::UnrecognizedConnective,
                    );
                };
                let generated = node
                    .bound_eventualities
                    .iter()
                    .map(|event| event.object_id())
                    .collect::<Vec<_>>();
                let mut scoped = bound.clone();
                scoped.extend(generated.iter().copied());
                let value = Datum::form(
                    operator,
                    children
                        .into_iter()
                        .map(|child| self.render_id(child, &scoped, active, expected_mode)),
                );
                self.recognized(self.bind_generated_events(id, &generated, value, &scoped, active))
            }
            data!(FormulaNode::Quantified(node)) => {
                let Some(variable_sort) = exact_plain_bound_variable(self.graph, node.variable)
                else {
                    return self.fallback_object(
                        id,
                        bound,
                        active,
                        CompactFallbackCause::QuantifierVariableFields,
                    );
                };
                let mut scoped = bound.clone();
                scoped.insert(node.variable);
                scoped.extend(
                    node.bound_eventualities
                        .iter()
                        .map(|event| event.object_id()),
                );
                let binding =
                    Datum::list([variable_datum(node.variable), sort_datum(variable_sort)]);
                let restriction = node.restriction.map(|restriction| {
                    self.render_id(
                        restriction,
                        &scoped,
                        active,
                        Some(PredicationMode::Restrictive),
                    )
                });
                let body = self.render_id(node.body, &scoped, active, expected_mode);
                let universal = exact_universal_quantity(self.graph, node.operator, node.quantity);
                let ordinary_exists =
                    node.operator == FormulaOperator::Exists && node.quantity.is_none();
                if node.source_variable.is_some()
                    || node.selection_source.is_some()
                    || !(universal || ordinary_exists)
                {
                    return self.fallback_object(
                        id,
                        bound,
                        active,
                        CompactFallbackCause::QuantifierFields,
                    );
                }
                let body_function = Datum::form("λ", [Datum::list([binding.clone()]), body]);
                let quantified = match restriction {
                    Some(restriction) => {
                        let restriction_function =
                            Datum::form("λ", [Datum::list([binding]), restriction]);
                        let constructor = if universal
                            && self.graph.objects[&id].formula_domain_import()
                                == Some(crate::model::DomainImport::Projective)
                        {
                            "Every"
                        } else if ordinary_exists {
                            "Some"
                        } else {
                            return self.fallback_object(
                                id,
                                bound,
                                active,
                                CompactFallbackCause::QuantifierFields,
                            );
                        };
                        Datum::list([
                            Datum::form(constructor, [restriction_function]),
                            body_function,
                        ])
                    }
                    None => Datum::form(if universal { "∀" } else { "∃" }, [body_function]),
                };
                let generated = node
                    .bound_eventualities
                    .iter()
                    .map(|event| event.object_id())
                    .collect::<Vec<_>>();
                self.recognized(
                    self.bind_generated_events(id, &generated, quantified, &scoped, active),
                )
            }
            data!(FormulaNode::QuantifierBundle(_)) => {
                self.fallback_object(id, bound, active, CompactFallbackCause::QuantifierFields)
            }
            data!(FormulaNode::RespectivelyDistribution(_)) => self.fallback_object(
                id,
                bound,
                active,
                CompactFallbackCause::RespectivelySlotFields,
            ),
        }
    }

    /// Project the canonical flat tanru graph to `(OfKind head modifier)` only
    /// after proving the link predication, head conjunct, property abstraction,
    /// and absorbed modifier event are private and otherwise default.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_tanru_projection(
        &self,
        id: SemanticObjectId,
        node: &crate::model::ConnectiveFormulaNode,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
        expected_mode: Option<PredicationMode>,
    ) -> Option<Datum> {
        let connector = node.connector.as_ref()?;
        if node.operator != FormulaOperator::And
            || node.children.len() != 2
            || node.eventuality.is_some()
            || connector.parameter.is_some()
            || connector.truth_table.is_some()
            || connector.locus != crate::model::ConnectorLocus::Predicate
            || !connector.source.is_implicit_juxtaposition()
        {
            return None;
        }
        let head_formula = node.children[0];
        let link_formula = node.children[1];
        let head_atom = formula_atom(self.graph, head_formula)?;
        let link_atom = formula_atom(self.graph, link_formula)?;
        if !head_atom.bound_eventualities.is_empty() || !link_atom.bound_eventualities.is_empty() {
            return None;
        }
        let head = self.graph.objects[&head_atom.predication].as_predication()?;
        let link = self.graph.objects[&link_atom.predication].as_predication()?;
        let data!(PredicationRelation::Named {
            relation: head_relation
        }) = head.relation.as_data()
        else {
            return None;
        };
        let data!(PredicationRelation::Composition) = link.relation.as_data() else {
            return None;
        };
        if expected_mode != Some(head.mode)
            || !predication_is_otherwise_plain(head)
            || link.eventuality.is_some()
            || link.place_questions.len() > 0
            || link.adjuncts.len() > 0
            || link.reciprocity.len() > 0
            || link.scalar_negation.is_some()
            || link.relation_metadata.is_some()
            || link.introduced_by.is_some()
            || link.arguments.len() != 2
            || link.mode != head.mode
        {
            return None;
        }
        let tanru = link.tanru_link.as_ref()?;
        if tanru.head != head_atom.predication
            || plain_argument_value(&link.arguments, 1) != plain_argument_value(&head.arguments, 1)
            || plain_argument_value(&link.arguments, 2) != Some(tanru.modifier)
        {
            return None;
        }
        let modifier = self.graph.objects[&tanru.modifier].as_referent()?;
        if modifier.category != ReferentCategory::Constant
            || !matches!(
                modifier
                    .scope_dependence
                    .as_ref()
                    .map(|value| value.as_data()),
                Some(data!(ScopeDependence::Fixed))
            )
            || modifier.sort != SemanticSort::Relation
            || modifier.parameters.len() != 1
            || modifier.arity != Some(1)
            || modifier.abstraction_kind != Some(crate::model::AbstractionKind::Property)
            || !referent_except_abstraction_is_default(modifier)
        {
            return None;
        }
        let modifier_formula = modifier.body?;
        let modifier_parameter = modifier.parameters[0];
        if !exact_parameter(
            self.graph,
            modifier_parameter,
            SemanticSort::Entity,
            ParameterRole::PropertySlot,
            "ce'u",
        ) {
            return None;
        }
        let (modifier_relation, modifier_predication, modifier_event) =
            recognize_tanru_modifier_property(
                self.graph,
                self.plan,
                modifier_formula,
                modifier_parameter,
            )?;
        let expected_label = format!("{modifier_relation}-{head_relation}");
        if !matches!(
            tanru.relation_label.as_data(),
            data!(RelationLabel::Constructed { text }) if text == &expected_label
        ) {
            return None;
        }

        let support = BTreeSet::from([
            head_formula,
            link_formula,
            link_atom.predication,
            tanru.modifier,
            modifier_formula,
            modifier_predication,
            modifier_parameter,
            modifier_event,
        ]);
        let allowed_sources = support
            .iter()
            .copied()
            .chain([id, head_atom.predication])
            .collect::<BTreeSet<_>>();
        if support.iter().any(|support_id| {
            !self.graph.objects[support_id].diagnostics().is_empty()
                || self
                    .plan
                    .uses_of(*support_id)
                    .is_some_and(|uses| !uses.is_subset(&allowed_sources))
        }) {
            return None;
        }
        let head_atom = Atom::try_new(head_relation.clone()).ok()?;
        let modifier_atom = Atom::try_new(modifier_relation.to_owned()).ok()?;
        let root = Datum::form(
            "Tanru",
            [Datum::Atom(modifier_atom), Datum::Atom(head_atom)],
        );
        let application =
            self.render_argument_map(root, &head.arguments, bound, active, head.eventuality)?;
        let mut conjuncts = vec![application.into_datum()];
        conjuncts.extend(
            head.adjuncts
                .iter()
                .map(|adjunct| self.render_modal(adjunct, bound, active))
                .collect::<Option<Vec<_>>>()?,
        );
        let value = if conjuncts.len() == 1 {
            conjuncts.pop().expect("one tanru application")
        } else {
            Datum::form("Joi", conjuncts)
        };
        let generated = node
            .bound_eventualities
            .iter()
            .map(|event| event.object_id())
            .collect::<Vec<_>>();
        Some(self.bind_generated_events(id, &generated, value, bound, active))
    }

    /// Add explicit generated-event binders only when sharing or facets make
    /// the graph-owned closure identity observable.
    #[requires(true)]
    #[ensures(true)]
    fn bind_generated_events(
        &self,
        owner: SemanticObjectId,
        generated: &[SemanticObjectId],
        body: Datum,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Datum {
        let visible = generated
            .iter()
            .copied()
            .filter(|event| {
                !generated_event_is_default(self.graph, self.plan, owner, *event)
                    || datum_contains_atom(&body, &variable_name(*event))
            })
            .collect::<Vec<_>>();
        if visible.is_empty() {
            return body;
        }
        let mut scoped = bound.clone();
        scoped.extend(visible.iter().copied());
        let facets = visible
            .iter()
            .flat_map(|event| self.generated_event_facets(*event, &scoped, active))
            .collect::<Vec<_>>();
        let body = if facets.is_empty() {
            body
        } else {
            Datum::form("Joi", std::iter::once(body).chain(facets))
        };
        Datum::form(
            "∃",
            [Datum::form(
                "λ",
                [
                    Datum::list(visible.iter().map(|event| {
                        let sort = self.graph.objects[event]
                            .as_eventuality()
                            .map(|node| SemanticSort::Eventuality(node.sort))
                            .unwrap_or(SemanticSort::eventuality());
                        Datum::list([variable_datum(*event), sort_datum(sort)])
                    })),
                    body,
                ],
            )],
        )
    }

    /// Every non-default generated-event coordinate becomes either a fixed
    /// exact intrinsic or one complete typed facet payload.
    #[requires(self.graph.objects.contains_key(&event))]
    #[ensures(true)]
    fn generated_event_facets(
        &self,
        event: SemanticObjectId,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Vec<Datum> {
        let Some(node) = self.graph.objects[&event].as_eventuality() else {
            return vec![self.typed_event_facet(event, bound, active)];
        };
        if generated_event_is_default_shape(node) {
            return Vec::new();
        }
        if let Some(time) = exact_generated_event_time_facet(node) {
            let relation = match time.relation.as_str() {
                "before" => "purci",
                "after" => "balvi",
                "at" => "cabna",
                _ => return vec![self.typed_event_facet(event, bound, active)],
            };
            return vec![Datum::form(
                relation,
                [
                    variable_datum(event),
                    self.render_id(time.anchor, bound, active, None),
                ],
            )];
        }
        vec![self.typed_event_facet(event, bound, active)]
    }

    /// Complete local structural payload for a generated event whose facets
    /// do not match one fixed intrinsic rule.
    #[requires(self.graph.objects.contains_key(&event))]
    #[ensures(matches!(ret, Datum::List(_)))]
    fn typed_event_facet(
        &self,
        event: SemanticObjectId,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Datum {
        self.counters.requires_typed_graph.set(true);
        self.counters
            .object_fallbacks
            .set(self.counters.object_fallbacks.get() + 1);
        *self
            .counters
            .fallback_reasons
            .borrow_mut()
            .entry(CompactFallbackCause::EventualityFacets.reason_id())
            .or_default() += 1;
        let object = &self.graph.objects[&event];
        let references = self.fallback_reference_map(event, bound, active);
        Datum::form(
            "Facet",
            [
                variable_datum(event),
                object_datum_with_variables(
                    object.object_kind(),
                    object,
                    ProvenanceDisposition::Suppress,
                    &references,
                ),
            ],
        )
    }

    /// Render graph-faithful predicate-term application, place deletion, event
    /// attachment, and canonical modal predicate terms.
    #[requires(true)]
    #[ensures(true)]
    fn render_predication(
        &self,
        id: SemanticObjectId,
        node: &PredicationNode,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
        expected_mode: Option<PredicationMode>,
    ) -> Datum {
        if expected_mode != Some(node.mode) {
            return self.fallback_object(
                id,
                bound,
                active,
                CompactFallbackCause::PredicationModeUnrepresentable,
            );
        }
        if node.tanru_link.is_some()
            || !node.place_questions.is_empty()
            || !node.reciprocity.is_empty()
            || node.scalar_negation.is_some()
            || node.relation_metadata.is_some()
            || node.introduced_by.is_some()
        {
            return self.fallback_object(
                id,
                bound,
                active,
                CompactFallbackCause::PredicationSideFields,
            );
        }
        let root = match node.relation.as_data() {
            data!(PredicationRelation::Named { relation }) => {
                let Ok(atom) = Atom::try_new(relation.clone()) else {
                    return self.fallback_object(
                        id,
                        bound,
                        active,
                        CompactFallbackCause::NonAtomicRelation,
                    );
                };
                Datum::Atom(atom)
            }
            data!(PredicationRelation::Parameter { parameter }) => {
                self.render_id(*parameter, bound, active, None)
            }
            data!(PredicationRelation::Composition) => {
                return self.fallback_object(
                    id,
                    bound,
                    active,
                    CompactFallbackCause::CompositionPredication,
                );
            }
        };
        let Some(application) =
            self.render_argument_map(root, &node.arguments, bound, active, node.eventuality)
        else {
            return self.fallback_object(id, bound, active, CompactFallbackCause::ArgumentFields);
        };
        let mut conjuncts = vec![application.into_datum()];
        for adjunct in &node.adjuncts {
            let Some(modal) = self.render_modal(adjunct, bound, active) else {
                return self.fallback_object(
                    id,
                    bound,
                    active,
                    CompactFallbackCause::AdjunctFields,
                );
            };
            conjuncts.push(modal);
        }
        let term = if conjuncts.len() == 1 {
            conjuncts.pop().expect("one predicate application")
        } else {
            Datum::form("Joi", conjuncts)
        };
        self.recognized(term)
    }

    /// Render numbered arguments with canonical `:n` and `:Eventuality`
    /// markers. `At` is reserved for computed place values.
    #[requires(true)]
    #[ensures(true)]
    fn render_argument_map(
        &self,
        mut term: Datum,
        arguments: &BTreeMap<PlaceIndex, ArgumentValue>,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
        eventuality: Option<SemanticObjectId>,
    ) -> Option<PredicateApplication> {
        let deleted = arguments
            .iter()
            .filter(|(_, argument)| argument.kind == ArgumentValueKind::Deleted)
            .map(|(place, _)| *place)
            .collect::<BTreeSet<_>>();
        for place in &deleted {
            term = Datum::form("DropPlace", [term, Datum::unsigned(place.get() as u128)]);
        }
        let mut next = 1usize;
        let mut application = PredicateApplication {
            head: term,
            operands: Vec::new(),
        };
        for (place, argument) in arguments {
            if argument.kind == ArgumentValueKind::Deleted {
                if argument.introduced_by.as_deref() != Some("zi'o") {
                    return None;
                }
                continue;
            }
            while deleted.contains(&PlaceIndex::new(next)) {
                next += 1;
            }
            if argument.quantity.is_some()
                || !argument.relative_clauses.is_empty()
                || argument.command_target.is_some()
                || (argument.kind == ArgumentValueKind::Elided
                    && argument.introduced_by.as_deref() != Some("zo'e"))
            {
                return None;
            }
            let value = argument.value?;
            if argument.kind == ArgumentValueKind::Elided
                && self.default_elided_is_silent(value, bound)
            {
                continue;
            }
            let value = self.render_id(value, bound, active, None);
            if place.get() != next {
                application.push(Datum::atom(format!(":{}", place.get())));
            }
            application.push(value);
            next = place.get() + 1;
        }
        if let Some(eventuality) = eventuality {
            let owner = self.plan.binder_owner(eventuality);
            let silent = self.absorbed_eventualities.borrow().contains(&eventuality)
                || owner.is_some_and(|owner| {
                    generated_event_is_default(self.graph, self.plan, owner, eventuality)
                });
            if !silent {
                application.push(Datum::atom(":Eventuality"));
                application.push(self.render_id(eventuality, bound, active, None));
            }
        }
        Some(application)
    }

    /// Named default for an unshared elided contextual referent.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn default_elided_is_silent(
        &self,
        id: SemanticObjectId,
        bound: &BTreeSet<SemanticObjectId>,
    ) -> bool {
        let Some(node) = self.graph.objects[&id].as_referent() else {
            return false;
        };
        if !default_elided_shape(node)
            || !self.graph.objects[&id].diagnostics().is_empty()
            || self.plan.use_count(id) != 1
        {
            return false;
        }
        self.scope_dependence_is_default(id, node.scope_dependence.as_ref(), bound)
    }

    /// Whether a stored dependence policy is exactly the default at this
    /// represented lexical site. `Fixed` is default only with no accessible
    /// binder; otherwise the complete derived binder universe must be named.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn scope_dependence_is_default(
        &self,
        id: SemanticObjectId,
        dependence: Option<&crate::model::ScopeDependence>,
        bound: &BTreeSet<SemanticObjectId>,
    ) -> bool {
        let Some(universe) = self.binder_universes.get(&id) else {
            return false;
        };
        if !universe.iter().all(|binder| bound.contains(binder)) {
            return false;
        }
        match dependence.map(|value| value.as_data()) {
            Some(data!(ScopeDependence::Fixed)) => universe.is_empty(),
            Some(data!(ScopeDependence::Underspecified { may_depend_on })) => {
                may_depend_on == universe
            }
            None => false,
        }
    }

    /// Render a canonical adjunct from its actual relation/place map. The
    /// surface `introduced_by` string is deliberately never inspected.
    #[requires(true)]
    #[ensures(true)]
    fn render_modal(
        &self,
        adjunct: &Adjunct,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Datum> {
        if adjunct.component.is_none()
            && adjunct.negation.is_none()
            && adjunct.scalar_negation.is_none()
            && adjunct.modifiers.is_empty()
        {
            if let Some(relation) = &adjunct.relation {
                if let Ok(root) = Atom::try_new(relation.clone()) {
                    if let Some(application) = self.render_argument_map(
                        Datum::Atom(root),
                        &adjunct.arguments,
                        bound,
                        active,
                        None,
                    ) {
                        return Some(application.into_datum());
                    }
                }
            }
        }
        None
    }

    /// Render reference constructors, indexicals, composition, abstractions,
    /// and explicit contextual constants.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_referent(
        &self,
        id: SemanticObjectId,
        node: &ReferentNode,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Datum {
        if let Some(indexical) = exact_referent_indexical(node) {
            return self.recognized(Datum::atom(indexical_name(indexical)));
        }
        if let Some(proximity) = exact_deictic(node, self.graph) {
            return self.recognized(Datum::atom(proximity));
        }
        if let Some(description) = self.render_description(id, node, bound, active) {
            return self.recognized(description);
        }
        if let Some(abstraction) = self.render_referent_abstraction(id, node, bound, active) {
            return self.recognized(abstraction);
        }
        if let Some(composition) = &node.composition {
            if referent_except_composition_is_default(node) {
                if !composition.excluded_members.is_empty()
                    || composition.collective.is_some()
                    || composition.scalar_negated.is_some()
                    || composition.complement.is_some()
                    || composition.endpoint_inclusion.is_some()
                    || composition.operator_parameter.is_some()
                {
                    return self.fallback_object(
                        id,
                        bound,
                        active,
                        CompactFallbackCause::CompositionFields,
                    );
                }
                let operator = composition_operator_name(composition.operator);
                let arguments = composition
                    .members
                    .iter()
                    .map(|member| self.render_id(*member, bound, active, None))
                    .collect::<Vec<_>>();
                return self.recognized(Datum::form(operator, arguments));
            }
        }
        if default_elided_shape(node) {
            let context =
                if self.scope_dependence_is_default(id, node.scope_dependence.as_ref(), bound) {
                    Datum::atom("Context")
                } else {
                    match node.scope_dependence.as_ref().map(|value| value.as_data()) {
                        Some(data!(ScopeDependence::Fixed)) => {
                            Datum::form("Context", [Datum::atom("Fixed")])
                        }
                        Some(data!(ScopeDependence::Underspecified { may_depend_on })) => {
                            Datum::form(
                                "Context",
                                [Datum::form(
                                    "MayDependOn",
                                    may_depend_on.iter().map(|binder| variable_datum(*binder)),
                                )],
                            )
                        }
                        None => {
                            return self.fallback_object(
                                id,
                                bound,
                                active,
                                CompactFallbackCause::ConstantWithoutDependence,
                            );
                        }
                    }
                };
            return self.recognized(context);
        }
        self.fallback_object(id, bound, active, CompactFallbackCause::ReferentFields)
    }

    /// Lower one exact fixed `lo`, `le`, or `la` description into a
    /// force-hosted `Refer` computation. The `le` branch retains the builder's
    /// explicit speaker/audience `skicu` property and never asserts the base
    /// classification. Unsupported description families remain typed fallback;
    /// incidental relatives are not turned into conjuncts.
    #[requires(true)]
    #[ensures(true)]
    fn render_description(
        &self,
        id: SemanticObjectId,
        node: &ReferentNode,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Datum> {
        if node.sort != SemanticSort::Entity
            || !self.projected_descriptions.contains(&id)
            || self.reference_binding_frames.borrow().is_empty()
        {
            return None;
        }
        let scope_default =
            self.scope_dependence_is_default(id, node.scope_dependence.as_ref(), bound);
        let recognition = recognize_description(self.graph, self.plan, id, node, scope_default)?;
        let mut scoped = bound.clone();
        scoped.insert(id);
        let variable = variable_datum(id);
        let property = match recognition.as_data() {
            data!(DescriptionRecognition::Property {
                constructor,
                property,
                parameter: None,
            }) if *constructor == DescriptionConstructor::Lo => {
                Datum::form(*property, [variable.clone()])
            }
            data!(DescriptionRecognition::Property {
                constructor,
                property,
                parameter: Some(parameter),
            }) if *constructor == DescriptionConstructor::Le => {
                let property_candidate = variable_datum(*parameter);
                Datum::form(
                    "skicu",
                    [
                        Datum::atom("Speaker"),
                        variable.clone(),
                        Datum::atom("Audience"),
                        Datum::form(
                            "λ",
                            [
                                Datum::list([Datum::list([
                                    property_candidate.clone(),
                                    referents_type_datum(node.sort),
                                ])]),
                                Datum::form(*property, [property_candidate]),
                            ],
                        ),
                    ],
                )
            }
            data!(DescriptionRecognition::Property { .. }) => {
                unreachable!("validated recognition pairs each constructor with its parameter")
            }
            data!(DescriptionRecognition::Name { name }) => {
                Datum::form("Named", [Datum::string(name), variable.clone()])
            }
        };
        let descriptor = node
            .descriptor
            .as_ref()
            .expect("recognized descriptions have a descriptor");
        let clauses = descriptor
            .relative_clauses
            .iter()
            .chain(node.relative_clauses.iter())
            .collect::<Vec<_>>();
        if clauses.iter().any(|clause| {
            clause.kind != RelativeClauseKind::Restrictive || clause.veridical == Some(false)
        }) {
            return None;
        }
        let frame_len = self
            .reference_binding_frames
            .borrow()
            .last()
            .expect("description requires an active force frame")
            .len();
        let mut conjuncts = vec![property];
        conjuncts.extend(clauses.into_iter().map(|clause| {
            self.render_id(
                clause.body,
                &scoped,
                active,
                Some(PredicationMode::Restrictive),
            )
        }));
        let mut frames = self.reference_binding_frames.borrow_mut();
        let frame = frames
            .last_mut()
            .expect("description requires an active force frame");
        if frame.len() != frame_len {
            // Dependency ordering belongs to the full dynamic planner. Do not
            // emit a guessed nesting for a description property that itself
            // performs another reference computation.
            frame.truncate(frame_len);
            return None;
        };
        let body = if conjuncts.len() == 1 {
            conjuncts.pop().expect("one description property")
        } else {
            Datum::form("∧", conjuncts)
        };
        let declared_type = referents_type_datum(node.sort);
        let computation = Datum::form(
            "Refer",
            [Datum::form(
                "λ",
                [
                    Datum::list([Datum::list([variable.clone(), declared_type.clone()])]),
                    body,
                ],
            )],
        );
        frame.push(ReferenceBinding {
            id,
            declared_type,
            computation,
        });
        Some(variable)
    }

    /// Render the two established pure abstraction crossings in this slice.
    /// Event-valued abstractions are reference computations and are therefore
    /// left to the force-hosted reference path rather than printed as `Nu`-like
    /// constructors.
    #[requires(true)]
    #[ensures(true)]
    fn render_referent_abstraction(
        &self,
        id: SemanticObjectId,
        node: &ReferentNode,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Datum> {
        let kind = node.abstraction_kind?;
        let body = node.body?;
        if !referent_except_abstraction_is_default(node)
            || !self.scope_dependence_is_default(id, node.scope_dependence.as_ref(), bound)
            || !node.parameters.iter().all(|parameter| {
                exact_parameter(
                    self.graph,
                    *parameter,
                    self.graph.objects[parameter]
                        .sort()
                        .unwrap_or(SemanticSort::Entity),
                    ParameterRole::PropertySlot,
                    "ce'u",
                )
            })
            || node
                .parameters
                .iter()
                .any(|parameter| self.graph.objects[parameter].sort() != Some(SemanticSort::Entity))
        {
            return None;
        }
        if !matches!(
            kind,
            AbstractionKind::Property | AbstractionKind::Proposition
        ) || (kind == AbstractionKind::Property && node.parameters.is_empty())
            || (kind == AbstractionKind::Proposition && !node.parameters.is_empty())
        {
            return None;
        }
        let mut scoped = bound.clone();
        scoped.extend(node.parameters.iter().copied());
        let body = self.render_id(
            body,
            &scoped,
            active,
            Some(if kind == AbstractionKind::Property {
                PredicationMode::Restrictive
            } else {
                PredicationMode::Inert
            }),
        );
        let value = if node.parameters.is_empty() {
            body
        } else {
            Datum::form(
                "λ",
                [
                    Datum::list(node.parameters.iter().map(|parameter| {
                        Datum::list([
                            variable_datum(*parameter),
                            referents_type_datum(
                                self.graph.objects[parameter]
                                    .sort()
                                    .unwrap_or(SemanticSort::Entity),
                            ),
                        ])
                    })),
                    body,
                ],
            )
        };
        Some(if kind == AbstractionKind::Proposition {
            Datum::form("Reify", [value])
        } else {
            value
        })
    }

    /// Eventualities use the same abstraction family and fixed indexicals;
    /// facet-bearing referents retain their typed structural object.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_eventuality(
        &self,
        id: SemanticObjectId,
        node: &EventualityNode,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Datum {
        if let Some(indexical) = exact_eventuality_indexical(node) {
            return self.recognized(Datum::atom(indexical_name(indexical)));
        }
        if node.denotation.is_generated_bound() {
            return if bound.contains(&id) {
                variable_datum(id)
            } else {
                self.fallback_object(
                    id,
                    bound,
                    active,
                    CompactFallbackCause::UnboundGeneratedEvent,
                )
            };
        }
        self.fallback_object(id, bound, active, CompactFallbackCause::EventualityFacets)
    }

    /// Render complete typed questions; use `Ask λ` only under its exact default.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_question(
        &self,
        id: SemanticObjectId,
        node: &crate::model::QuestionNode,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Datum {
        if !question_slots_are_exact(self.graph, node) {
            return self.fallback_object(
                id,
                bound,
                active,
                CompactFallbackCause::QuestionSlotFields,
            );
        }
        let parameters = node
            .slots
            .iter()
            .filter_map(|slot| slot.parameter())
            .collect::<Vec<_>>();
        let mut scoped = bound.clone();
        scoped.extend(parameters.iter().copied());
        let body = self.render_id(node.body, &scoped, active, Some(PredicationMode::Asserted));
        let ordinary_slots = node.slots.iter().all(|slot| {
            matches!(
                slot.as_data(),
                data!(QuestionSlot::Homogeneous {
                    role: QuestionSlotRole::Answer,
                    ..
                })
            )
        });
        if node.mode == QuestionMode::Direct
            && object_is_indexical(self.graph, node.asker, IndexicalKind::Speaker)
            && object_is_indexical(self.graph, node.respondent, IndexicalKind::Audience)
            && ordinary_slots
            && node.focus.is_none()
            && node.presupposed_answer.is_none()
        {
            if node.kind == QuestionKind::Truth && parameters.is_empty() {
                return self.recognized(Datum::form("Ask", [Datum::form("Polar", [body])]));
            }
            if node.kind == QuestionKind::Argument
                && !parameters.is_empty()
                && parameters.iter().all(|parameter| {
                    self.graph.objects[parameter].sort() == Some(SemanticSort::Entity)
                })
            {
                let lambda = Datum::form(
                    "λ",
                    [
                        Datum::list(parameters.iter().map(|parameter| {
                            Datum::list([
                                variable_datum(*parameter),
                                referents_type_datum(SemanticSort::Entity),
                            ])
                        })),
                        body,
                    ],
                );
                return self.recognized(Datum::form("Ask", [Datum::form("OpenQ", [lambda])]));
            }
        }
        self.fallback_object(id, bound, active, CompactFallbackCause::QuestionSlotFields)
    }

    /// Preserve quantity form, value, scale, comparison set, and question
    /// parameters. Numeric shorthands require an actual integer graph value.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_quantity(
        &self,
        id: SemanticObjectId,
        node: &QuantityNode,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Datum {
        if node.value.question_parameters.is_empty() && node.comparison_set.is_none() {
            if let Some(integer) = node.value.integer {
                if node.form == QuantityForm::Exact && node.scale == QuantityScale::Count {
                    return self.recognized(Datum::signed(i128::from(integer)));
                }
                if node.scale == QuantityScale::Count
                    && let Some(form) = numeric_quantity_form(node.form)
                {
                    return self
                        .recognized(Datum::form(form, [Datum::signed(i128::from(integer))]));
                }
            }
        }

        self.fallback_object(id, bound, active, CompactFallbackCause::QuantityFields)
    }

    /// Render fixed math operators through the declared Unicode table; named
    /// operators retain an explicit typed name.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_math(
        &self,
        id: SemanticObjectId,
        node: &crate::model::MathExpressionNode,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Datum {
        if node.scalar_negation.is_some() || node.subscript.is_some() {
            return self.fallback_object(id, bound, active, CompactFallbackCause::MathSideFields);
        }
        match node.kind.as_data() {
            data!(MathExpressionNodeKind::Literal { literal, denotes }) => {
                if denotes.is_some() {
                    return self.fallback_object(
                        id,
                        bound,
                        active,
                        CompactFallbackCause::MathLiteralDenotes,
                    );
                }
                if let data!(crate::model::MathLiteralValue::Integer(value)) =
                    literal.value.as_data()
                {
                    return self.recognized(Datum::signed(i128::from(*value)));
                }
                self.fallback_object(id, bound, active, CompactFallbackCause::MathOperatorFields)
            }
            data!(MathExpressionNodeKind::Operator {
                operator,
                operands,
                operator_denotes,
                endpoint_inclusion,
            }) => {
                if operator_denotes.is_some() || endpoint_inclusion.is_some() {
                    return self.fallback_object(
                        id,
                        bound,
                        active,
                        CompactFallbackCause::MathOperatorFields,
                    );
                }
                if operands.len() != 2 {
                    return self.fallback_object(
                        id,
                        bound,
                        active,
                        CompactFallbackCause::MathOperatorFields,
                    );
                }
                let head = match operator.as_data() {
                    data!(MathOperator::Add) => Datum::atom("+"),
                    data!(MathOperator::Multiply) => Datum::atom("×"),
                    data!(MathOperator::Subtract) => Datum::atom("−"),
                    data!(MathOperator::Divide) => Datum::atom("÷"),
                    _ => {
                        return self.fallback_object(
                            id,
                            bound,
                            active,
                            CompactFallbackCause::MathOperatorFields,
                        );
                    }
                };
                let mut application = vec![head];
                application.extend(
                    operands
                        .iter()
                        .map(|operand| self.render_id(*operand, bound, active, None)),
                );
                self.recognized(Datum::List(application))
            }
            data!(MathExpressionNodeKind::QuestionedOperator {
                operator_parameter,
                operands,
            }) => {
                let _ = (operator_parameter, operands);
                self.fallback_object(id, bound, active, CompactFallbackCause::MathOperatorFields)
            }
        }
    }

    /// Preserve quotation nesting and distinguish quotation text from sign text.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_sign(
        &self,
        id: SemanticObjectId,
        _node: &SignNode,
        bound: &BTreeSet<SemanticObjectId>,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Datum {
        self.fallback_object(id, bound, active, CompactFallbackCause::SignFields)
    }
}

/// Whether a rendered subtree contains one exact atom. This is used only to
/// keep a graph-owned generated-event binder when typed fallback makes the
/// otherwise-default identity explicit.
#[requires(!needle.is_empty())]
#[ensures(true)]
fn datum_contains_atom(datum: &Datum, needle: &str) -> bool {
    datum.as_atom() == Some(needle)
        || datum
            .as_list()
            .is_some_and(|items| items.iter().any(|item| datum_contains_atom(item, needle)))
}

/// Stable lexical variable datum for a typed graph identity.
#[requires(true)]
#[ensures(ret.as_atom().is_some_and(|atom| atom.starts_with('$')))]
fn variable_datum(id: SemanticObjectId) -> Datum {
    Datum::atom(variable_name(id))
}

/// Stable lexical variable spelling; the typed ID chooses the namespace and the
/// textual conversion merely replaces the separator with atom-safe `_`.
#[requires(true)]
#[ensures(ret.starts_with('$'))]
fn variable_name(id: SemanticObjectId) -> String {
    format!("${}", id.to_string().replace(':', "_"))
}

/// Sort spelling table used by every binder form.
#[requires(true)]
#[ensures(ret.as_atom().is_some())]
fn sort_datum(sort: SemanticSort) -> Datum {
    Datum::atom(match sort {
        SemanticSort::Entity => "Entity",
        SemanticSort::Mass => "Mass",
        SemanticSort::Set => "Set",
        SemanticSort::Sequence => "Sequence",
        SemanticSort::Time => "Time",
        SemanticSort::Eventuality(EventualitySort::General) => "Eventuality",
        SemanticSort::Eventuality(EventualitySort::State) => "Eventuality/State",
        SemanticSort::Eventuality(EventualitySort::Process) => "Eventuality/Process",
        SemanticSort::Eventuality(EventualitySort::Activity) => "Eventuality/Activity",
        SemanticSort::Eventuality(EventualitySort::Achievement) => "Eventuality/Achievement",
        SemanticSort::Eventuality(EventualitySort::Experience) => "Eventuality/Experience",
        SemanticSort::Eventuality(EventualitySort::Locution) => "Eventuality/Locution",
        SemanticSort::Predication => "Predication",
        SemanticSort::TruthValue => "TruthValue",
        SemanticSort::Proposition => "Proposition",
        SemanticSort::Concept => "Concept",
        SemanticSort::Amount => "Amount",
        SemanticSort::Quantity => "Quantity",
        SemanticSort::Number => "Number",
        SemanticSort::Scale => "Scale",
        SemanticSort::Text => "Text",
        SemanticSort::Sign => "Sign",
        SemanticSort::Relation => "Relation",
        SemanticSort::Place => "Place",
        SemanticSort::Connective => "Connective",
        SemanticSort::TenseModal => "TenseModal",
        SemanticSort::MathOperator => "MathOperator",
        SemanticSort::ArgumentBundle => "ArgumentBundle",
        SemanticSort::AbstractNature => "AbstractNature",
    })
}

/// Number-neutral reference type for one represented semantic sort.
#[requires(true)]
#[ensures(ret.form_head() == Some("Referents"))]
fn referents_type_datum(sort: SemanticSort) -> Datum {
    Datum::form("Referents", [sort_datum(sort)])
}

/// Notation-level type of a shared graph value. Formulae and open predicate
/// terms are not referents, so they must never inherit an unrelated fallback
/// semantic sort merely because `SemanticObject::sort` is intentionally absent.
#[requires(true)]
#[ensures(ret.as_atom().is_some())]
fn definition_type_datum(object: &crate::model::SemanticObject) -> Datum {
    Datum::atom(match object.object_kind() {
        crate::model::SemanticObjectKind::Formula => "Content",
        crate::model::SemanticObjectKind::Predication => "PredTerm",
        crate::model::SemanticObjectKind::Utterance => "Utterance",
        crate::model::SemanticObjectKind::Sequence => "Content",
        crate::model::SemanticObjectKind::DisplayedContent => "Content",
        crate::model::SemanticObjectKind::Question => "Question",
        crate::model::SemanticObjectKind::RelationMetadata => "RelationMetadata",
        _ => {
            return sort_datum(object.sort().unwrap_or(SemanticSort::AbstractNature));
        }
    })
}

/// Whether an object is represented losslessly by a declared context atom.
#[requires(true)]
#[ensures(true)]
fn is_conventional_atom(object: &crate::model::SemanticObject) -> bool {
    object
        .as_referent()
        .and_then(exact_referent_indexical)
        .is_some()
        || object
            .as_eventuality()
            .and_then(exact_eventuality_indexical)
            .is_some()
}

/// Objects consumed by an exact description-constructor projection, plus the
/// description values whose internal builder cycles have thereby disappeared.
/// A support component is projected only when none of its identities is used
/// outside that constructor.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.0.iter().all(|id| graph.objects.contains_key(id)))]
#[ensures(ret.1.iter().all(|id| graph.objects.contains_key(id)))]
pub(super) fn projected_description_objects(
    graph: &SemanticGraph,
    plan: &ReferencePlan,
) -> (BTreeSet<SemanticObjectId>, BTreeSet<SemanticObjectId>) {
    let mut projected = BTreeSet::new();
    let mut descriptions = BTreeSet::new();
    for (described, object) in &graph.objects {
        let Some(node) = object.as_referent() else {
            continue;
        };
        let scope_default = matches!(
            node.scope_dependence
                .as_ref()
                .map(|dependence| dependence.as_data()),
            Some(data!(ScopeDependence::Fixed))
        );
        let Some(recognition) = recognize_description(graph, plan, *described, node, scope_default)
        else {
            continue;
        };
        let data!(DescriptionRecognition::Property {
            constructor,
            property: _,
            parameter: _,
        }) = recognition.as_data()
        else {
            continue;
        };
        let descriptor = node
            .descriptor
            .as_ref()
            .expect("recognized property has a descriptor");
        let body = descriptor
            .body
            .expect("recognized property has a descriptor body");
        let mut support = BTreeSet::new();
        match *constructor {
            DescriptionConstructor::Lo => {
                collect_property_support(graph, body, *described, &mut support);
            }
            DescriptionConstructor::Le => {
                collect_speaker_description_support(graph, body, *described, &mut support);
            }
        }
        for clause in descriptor
            .relative_clauses
            .iter()
            .chain(node.relative_clauses.iter())
        {
            collect_inline_support(graph, clause.body, *described, &mut support);
        }
        let allowed_sources = support
            .iter()
            .copied()
            .chain([*described])
            .collect::<BTreeSet<_>>();
        if support.iter().all(|id| {
            plan.uses_of(*id)
                .is_none_or(|uses| uses.is_subset(&allowed_sources))
        }) {
            projected.extend(support);
            descriptions.insert(*described);
        }
    }
    (projected, descriptions)
}

/// Current described-event values and the content subgraphs that their exact
/// `Lo`/abstraction projection renders inline. Internal backedges to the event
/// are binder uses, not reasons to manufacture a recursive value definition.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.0.iter().all(|id| graph.objects.contains_key(id)))]
#[ensures(ret.1.iter().all(|id| graph.objects.contains_key(id)))]
fn projected_described_event_objects(
    graph: &SemanticGraph,
    plan: &ReferencePlan,
) -> (BTreeSet<SemanticObjectId>, BTreeSet<SemanticObjectId>) {
    let mut projected = BTreeSet::new();
    let mut values = BTreeSet::new();
    for (event, object) in &graph.objects {
        let Some(node) = object.as_eventuality() else {
            continue;
        };
        let (Some(descriptor), Some(kind), Some(content)) = (
            node.descriptor.as_ref(),
            node.abstraction_kind,
            node.content,
        ) else {
            continue;
        };
        if !described_eventuality_base_is_exact(node, descriptor, kind)
            || !matches!(
                node.denotation
                    .scope_dependence()
                    .map(|dependence| dependence.as_data()),
                Some(data!(ScopeDependence::Fixed))
            )
            || !descriptor
                .speaker
                .is_some_and(|speaker| object_is_indexical(graph, speaker, IndexicalKind::Speaker))
            || (node.time.is_some() && exact_described_event_time_facet(node).is_none())
        {
            continue;
        }
        let mut support = BTreeSet::new();
        collect_inline_support(graph, content, *event, &mut support);
        let allowed_sources = support
            .iter()
            .copied()
            .chain([*event])
            .collect::<BTreeSet<_>>();
        if support.iter().all(|id| {
            plan.uses_of(*id)
                .is_none_or(|uses| uses.is_subset(&allowed_sources))
        }) {
            projected.extend(support);
            values.insert(*event);
        }
    }
    (projected, values)
}

/// Collect an inline subgraph without crossing its owning self-reference or a
/// canonical atom that is safe to repeat by identity.
#[requires(graph.objects.contains_key(&root))]
#[ensures(out.contains(&root) || root == owner || is_conventional_atom(&graph.objects[&root]))]
fn collect_inline_support(
    graph: &SemanticGraph,
    root: SemanticObjectId,
    owner: SemanticObjectId,
    out: &mut BTreeSet<SemanticObjectId>,
) {
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        if id == owner || is_conventional_atom(&graph.objects[&id]) || !out.insert(id) {
            continue;
        }
        let mut references = Vec::new();
        graph.objects[&id].references_into(&mut references);
        pending.extend(references);
    }
}

/// Collect the atom formula, predication, and contextual arguments consumed by
/// one exact unary-property recognition.
#[requires(graph.objects.contains_key(&formula))]
#[ensures(out.contains(&formula))]
fn collect_property_support(
    graph: &SemanticGraph,
    formula: SemanticObjectId,
    subject: SemanticObjectId,
    out: &mut BTreeSet<SemanticObjectId>,
) {
    out.insert(formula);
    let Some(formula_node) = graph.objects[&formula].as_formula() else {
        return;
    };
    let data!(FormulaNode::Atom(atom)) = formula_node.as_data() else {
        return;
    };
    out.insert(atom.predication);
    let Some(predication) = graph.objects[&atom.predication].as_predication() else {
        return;
    };
    out.extend(
        predication
            .arguments
            .values()
            .filter_map(|argument| argument.value)
            .filter(|value| *value != subject && !is_conventional_atom(&graph.objects[value])),
    );
}

/// Collect both layers of the exact recursive `skicu` description encoding.
#[requires(graph.objects.contains_key(&formula))]
#[ensures(out.contains(&formula))]
fn collect_speaker_description_support(
    graph: &SemanticGraph,
    formula: SemanticObjectId,
    described: SemanticObjectId,
    out: &mut BTreeSet<SemanticObjectId>,
) {
    out.insert(formula);
    let Some(formula_node) = graph.objects[&formula].as_formula() else {
        return;
    };
    let data!(FormulaNode::Atom(atom)) = formula_node.as_data() else {
        return;
    };
    out.insert(atom.predication);
    let Some(predication) = graph.objects[&atom.predication].as_predication() else {
        return;
    };
    let Some(relation_value) = plain_argument_value(&predication.arguments, 4) else {
        return;
    };
    out.insert(relation_value);
    out.extend(
        predication
            .arguments
            .values()
            .filter_map(|argument| argument.value)
            .filter(|value| {
                *value != described
                    && *value != relation_value
                    && !is_conventional_atom(&graph.objects[value])
            }),
    );
    let Some(relation_node) = graph.objects[&relation_value].as_referent() else {
        return;
    };
    out.extend(relation_node.parameters.iter().copied());
    if let (Some(body), Some(parameter)) = (relation_node.body, relation_node.parameters.first()) {
        collect_property_support(graph, body, *parameter, out);
    }
}

/// Exact default allowing an utterance record to collapse to its act.
#[requires(graph.objects.contains_key(&id))]
#[ensures(true)]
fn utterance_record_is_default(
    graph: &SemanticGraph,
    plan: &ReferencePlan,
    id: SemanticObjectId,
    node: &UtteranceNode,
) -> bool {
    !utterance_identity_is_observed(graph, plan, id)
        && object_is_indexical(graph, node.speaker, IndexicalKind::Speaker)
        && object_is_indexical(graph, node.audience, IndexicalKind::Audience)
        && object_is_indexical(graph, node.deictic_ground.time, IndexicalKind::Now)
        && object_is_indexical(graph, node.deictic_ground.place, IndexicalKind::Here)
        && default_locution_event(graph, node.eventuality)
        && plan.use_count(node.eventuality) == 1
        && node.asides.is_empty()
        && node.vocative_kind.is_none()
}

/// A sole sequence-item containment edge selects a performing document child;
/// it does not observe the utterance token identity. Any other edge, repeated
/// item occurrence, quotation containment, or ordinal target keeps the record.
#[requires(graph.objects.contains_key(&id))]
#[ensures(!ret || plan.use_count(id) > 0)]
fn utterance_identity_is_observed(
    graph: &SemanticGraph,
    plan: &ReferencePlan,
    id: SemanticObjectId,
) -> bool {
    match plan.use_count(id) {
        0 => false,
        1 => {
            let Some(sources) = plan.uses_of(id) else {
                return true;
            };
            let Some(source) = sources
                .iter()
                .next()
                .copied()
                .filter(|_| sources.len() == 1)
            else {
                return true;
            };
            let Some(sequence) = graph.objects[&source].as_sequence() else {
                return true;
            };
            sequence.items.iter().filter(|item| **item == id).count() != 1
        }
        _ => true,
    }
}

/// Exact default locution-event shape suppressed with an omitted record.
#[requires(graph.objects.contains_key(&id))]
#[ensures(true)]
fn default_locution_event(graph: &SemanticGraph, id: SemanticObjectId) -> bool {
    let Some(node) = graph.objects[&id].as_eventuality() else {
        return false;
    };
    matches!(node.denotation.as_data(), data!(EventualityDenotation::Referential {
            category: ReferentCategory::Constant,
            scope_dependence: Some(dependence),
        }) if matches!(dependence.as_data(), data!(ScopeDependence::Fixed)))
        && node.sort == EventualitySort::Locution
        && node.class == Some(crate::model::EventualityClass::Locution)
        && node
            .actuality
            .is_some_and(|actuality| actuality.kind == ActualityKind::Actual)
        && eventuality_optional_fields_are_empty(node, false, true)
        && graph.objects[&id].diagnostics().is_empty()
}

/// Exact indexical lookup through either referent model variant.
#[requires(graph.objects.contains_key(&id))]
#[ensures(true)]
fn object_is_indexical(
    graph: &SemanticGraph,
    id: SemanticObjectId,
    expected: IndexicalKind,
) -> bool {
    graph.objects[&id]
        .as_referent()
        .and_then(exact_referent_indexical)
        .is_some_and(|indexical| indexical == expected)
        || graph.objects[&id]
            .as_eventuality()
            .and_then(exact_eventuality_indexical)
            .is_some_and(|indexical| indexical == expected)
}

/// Exact entity indexical shape.
#[requires(true)]
#[ensures(true)]
fn exact_referent_indexical(node: &ReferentNode) -> Option<IndexicalKind> {
    let indexical = node.indexical?;
    (matches!(
        indexical,
        IndexicalKind::Speaker | IndexicalKind::Audience | IndexicalKind::Here
    ) && node.category == ReferentCategory::Indexical
        && node.scope_dependence.is_none()
        && node.sort == SemanticSort::Entity
        && node.deictic_reference.is_none()
        && referent_payload_is_empty(node))
    .then_some(indexical)
}

/// Exact `ti`/`ta`/`tu` proximity atom grounded at declared `Here`.
#[requires(true)]
#[ensures(true)]
fn exact_deictic(node: &ReferentNode, graph: &SemanticGraph) -> Option<&'static str> {
    if node.category != ReferentCategory::Indexical
        || node.scope_dependence.is_some()
        || node.sort != SemanticSort::Entity
        || node.indexical.is_some()
        || !referent_payload_is_empty(node)
    {
        return None;
    }
    let deictic = node.deictic_reference.as_ref()?;
    if !object_is_indexical(graph, deictic.ground, IndexicalKind::Here) {
        return None;
    }
    Some(match deictic.proximity {
        DeicticProximity::Proximal => "This",
        DeicticProximity::Medial => "That",
        DeicticProximity::Distal => "Yonder",
    })
}

/// Referent fields absent from the indexical/deictic role itself.
#[requires(true)]
#[ensures(true)]
fn referent_payload_is_empty(node: &ReferentNode) -> bool {
    node.descriptor.is_none()
        && node.composition.is_none()
        && node.personal_mass_membership.is_none()
        && node.generated_referent.is_none()
        && node.relative_clauses.is_empty()
        && node.assigned_names.is_empty()
        && node.body.is_none()
        && node.parameters.is_empty()
        && node.arity.is_none()
        && node.embedded_questions.is_empty()
        && node.abstraction_kind.is_none()
        && node.abstracted.is_none()
        && node.experiencer.is_none()
        && node.target.is_none()
        && node.scale.is_none()
        && node.subscript.is_none()
}

/// Exact eventuality indexical shape.
#[requires(true)]
#[ensures(true)]
fn exact_eventuality_indexical(node: &EventualityNode) -> Option<IndexicalKind> {
    let category_is_indexical = matches!(
        node.denotation.as_data(),
        data!(EventualityDenotation::Referential {
            category: ReferentCategory::Indexical,
            scope_dependence: None,
        })
    );
    (node.indexical == Some(IndexicalKind::Now)
        && category_is_indexical
        && node.sort == EventualitySort::General
        && eventuality_optional_fields_are_empty(node, true, false))
    .then_some(IndexicalKind::Now)
}

/// Exhaustive eventuality field audit for defaults and abstractors.
#[requires(true)]
#[ensures(true)]
fn eventuality_optional_fields_are_empty(
    node: &EventualityNode,
    allow_indexical: bool,
    allow_class: bool,
) -> bool {
    (allow_class || node.class.is_none())
        && (allow_indexical || node.indexical.is_none())
        && node.descriptor.is_none()
        && node.composition.is_none()
        && node.relative_clauses.is_empty()
        && node.assigned_names.is_empty()
        && node.adjuncts.is_empty()
        && node.tense_modal.is_none()
        && node.time.is_none()
        && node.time_path.is_empty()
        && node.time_interval.is_none()
        && node.time_span.is_none()
        && node.aspect.is_none()
        && node.aspects.is_empty()
        && node.recurrence.is_empty()
        && node.interval_modifiers.is_empty()
        && node.space.is_none()
        && node.space_path.is_empty()
        && node.space_interval.is_none()
        && node.spatial_aspect.is_none()
        && node.spatial_aspects.is_empty()
        && node.spatial_recurrence.is_empty()
        && node.spatial_interval_modifiers.is_empty()
        && node.content.is_none()
        && node.body.is_none()
        && node.parameters.is_empty()
        && node.arity.is_none()
        && node.embedded_questions.is_empty()
        && node.abstraction_kind.is_none()
        && node.experiencer.is_none()
        && node.target.is_none()
        && node.scale.is_none()
        && node.subscript.is_none()
}

/// Generated event that may be suppressed at its graph-owned closure site.
#[requires(graph.objects.contains_key(&event))]
#[ensures(true)]
fn generated_event_is_default(
    graph: &SemanticGraph,
    plan: &ReferencePlan,
    owner: SemanticObjectId,
    event: SemanticObjectId,
) -> bool {
    let Some(node) = graph.objects[&event].as_eventuality() else {
        return false;
    };
    generated_event_is_default_shape(node)
        && graph.objects[&event].diagnostics().is_empty()
        && plan.binder_owner(event) == Some(owner)
        // A default event has exactly the closure-owner binding edge and the
        // predication's eventuality edge. Additional edges make its identity
        // observable even when they originate in the same object.
        && plan.use_count(event) == 2
}

/// Object-local part of the generated-event default. Sort and class are
/// audited because a completely silent event has no binder that could retain
/// a non-general subtype.
#[requires(true)]
#[ensures(true)]
fn generated_event_is_default_shape(node: &EventualityNode) -> bool {
    node.denotation.is_generated_bound()
        && node.sort == EventualitySort::General
        && node.actuality.is_none()
        && node
            .class
            .is_none_or(|class| class == crate::model::EventualityClass::Event)
        && eventuality_optional_fields_are_empty(node, false, true)
}

/// Fixed exact time-anchor facet shape. Every other eventuality coordinate is
/// checked before `Before`, `After`, or `AtTime` may consume this record.
#[requires(true)]
#[ensures(ret.is_none_or(|time| !time.relation.is_empty()))]
fn exact_generated_event_time_facet(
    node: &EventualityNode,
) -> Option<&crate::model::AnchorRelation> {
    if !node.denotation.is_generated_bound()
        || node.sort != EventualitySort::General
        || node
            .class
            .is_some_and(|class| class != crate::model::EventualityClass::Event)
        || node.indexical.is_some()
        || node.descriptor.is_some()
        || node.composition.is_some()
        || !node.relative_clauses.is_empty()
        || !node.assigned_names.is_empty()
        || !node.adjuncts.is_empty()
        || node.actuality.is_some()
        || node.tense_modal.is_some()
        || !node.time_path.is_empty()
        || node.time_interval.is_some()
        || node.time_span.is_some()
        || node.aspect.is_some()
        || !node.aspects.is_empty()
        || !node.recurrence.is_empty()
        || !node.interval_modifiers.is_empty()
        || node.space.is_some()
        || !node.space_path.is_empty()
        || node.space_interval.is_some()
        || node.spatial_aspect.is_some()
        || !node.spatial_aspects.is_empty()
        || !node.spatial_recurrence.is_empty()
        || !node.spatial_interval_modifiers.is_empty()
        || node.content.is_some()
        || node.body.is_some()
        || !node.parameters.is_empty()
        || node.arity.is_some()
        || !node.embedded_questions.is_empty()
        || node.abstraction_kind.is_some()
        || node.experiencer.is_some()
        || node.target.is_some()
        || node.scale.is_some()
        || node.subscript.is_some()
    {
        return None;
    }
    let time = node.time.as_ref()?;
    (!time.sticky
        && time.inherited.is_none()
        && time.distance.is_none()
        && time.magnitude.is_none()
        && time.scalar_negation.is_none()
        && time.motion.is_none())
    .then_some(time)
}

/// Exact default `zo'e` object shape before scope/use-site checks.
#[requires(true)]
#[ensures(true)]
fn default_elided_shape(node: &ReferentNode) -> bool {
    node.category == ReferentCategory::Constant
        && node.sort == SemanticSort::Entity
        && node.indexical.is_none()
        && node.deictic_reference.is_none()
        && node.descriptor.as_ref().is_some_and(|descriptor| {
            descriptor.kind == DescriptorKind::Elided
                && descriptor.word == "zo'e"
                && descriptor.speaker.is_none()
                && descriptor.body.is_none()
                && descriptor.veridical.is_none()
                && descriptor.relative_clauses.is_empty()
                && descriptor.quantity.is_none()
                && descriptor.name.is_none()
                && descriptor.scale.is_none()
                && descriptor.definiteness.is_none()
                && descriptor.operand.is_none()
        })
        && node.composition.is_none()
        && node.personal_mass_membership.is_none()
        && node.generated_referent.is_none()
        && node.relative_clauses.is_empty()
        && node.assigned_names.is_empty()
        && node.body.is_none()
        && node.parameters.is_empty()
        && node.arity.is_none()
        && node.embedded_questions.is_empty()
        && node.abstraction_kind.is_none()
        && node.abstracted.is_none()
        && node.experiencer.is_none()
        && node.target.is_none()
        && node.scale.is_none()
        && node.subscript.is_none()
}

/// Description referent has no content outside its descriptor/attached clauses.
#[requires(true)]
#[ensures(true)]
fn referent_except_descriptor_is_default(node: &ReferentNode) -> bool {
    node.category == ReferentCategory::Constant
        && node.scope_dependence.is_some()
        && node.sort == SemanticSort::Entity
        && node.indexical.is_none()
        && node.deictic_reference.is_none()
        && node.composition.is_none()
        && node.personal_mass_membership.is_none()
        && node.generated_referent.is_none()
        && node.assigned_names.is_empty()
        && node.body.is_none()
        && node.parameters.is_empty()
        && node.arity.is_none()
        && node.embedded_questions.is_empty()
        && node.abstraction_kind.is_none()
        && node.abstracted.is_none()
        && node.experiencer.is_none()
        && node.target.is_none()
        && node.scale.is_none()
        && node.subscript.is_none()
}

/// Composition referent has no second denotation mechanism or attachments.
#[requires(true)]
#[ensures(true)]
fn referent_except_composition_is_default(node: &ReferentNode) -> bool {
    node.category == ReferentCategory::Composite
        && node.scope_dependence.is_none()
        && node.sort == SemanticSort::Entity
        && node.indexical.is_none()
        && node.deictic_reference.is_none()
        && node.descriptor.is_none()
        && node.personal_mass_membership.is_none()
        && node.generated_referent.is_none()
        && node.relative_clauses.is_empty()
        && node.assigned_names.is_empty()
        && node.body.is_none()
        && node.parameters.is_empty()
        && node.arity.is_none()
        && node.embedded_questions.is_empty()
        && node.abstraction_kind.is_none()
        && node.abstracted.is_none()
        && node.experiencer.is_none()
        && node.target.is_none()
        && node.scale.is_none()
        && node.subscript.is_none()
}

/// Abstraction referent has no unrelated reference mechanism or attachments.
#[requires(true)]
#[ensures(true)]
fn referent_except_abstraction_is_default(node: &ReferentNode) -> bool {
    node.category == ReferentCategory::Constant
        && node.scope_dependence.is_some()
        && node.indexical.is_none()
        && node.deictic_reference.is_none()
        && node.descriptor.is_none()
        && node.composition.is_none()
        && node.personal_mass_membership.is_none()
        && node.generated_referent.is_none()
        && node.relative_clauses.is_empty()
        && node.assigned_names.is_empty()
        && node.embedded_questions.is_empty()
        && node.abstracted.is_none()
        && node.experiencer.is_none()
        && node.target.is_none()
        && node.scale.is_none()
        && node.subscript.is_none()
        && node.arity
            == (node.abstraction_kind == Some(AbstractionKind::Property))
                .then_some(node.parameters.len())
}

/// `lo`-described abstraction has no second denotation mechanism or attached
/// content beyond its descriptor and abstraction payload.
#[requires(true)]
#[ensures(true)]
fn referent_except_described_abstraction_is_default(node: &ReferentNode) -> bool {
    node.category == ReferentCategory::Constant
        && node.scope_dependence.is_some()
        && node.indexical.is_none()
        && node.deictic_reference.is_none()
        && node.composition.is_none()
        && node.personal_mass_membership.is_none()
        && node.generated_referent.is_none()
        && node.relative_clauses.is_empty()
        && node.assigned_names.is_empty()
        && node.embedded_questions.is_empty()
        && node.abstracted.is_none()
        && node.experiencer.is_none()
        && node.target.is_none()
        && node.scale.is_none()
        && node.subscript.is_none()
        && node.arity
            == (node.abstraction_kind == Some(AbstractionKind::Property))
                .then_some(node.parameters.len())
}

/// Eventuality abstraction has no facet or attachment that would need a
/// described-event constructor.
#[requires(true)]
#[ensures(true)]
fn eventuality_except_abstraction_is_default(node: &EventualityNode) -> bool {
    node.denotation.category() == Some(ReferentCategory::Constant)
        && node.denotation.scope_dependence().is_some()
        && node.class.is_none()
        && node.indexical.is_none()
        && node.descriptor.is_none()
        && node.composition.is_none()
        && node.relative_clauses.is_empty()
        && node.assigned_names.is_empty()
        && node.adjuncts.is_empty()
        && node.actuality.is_none()
        && node.tense_modal.is_none()
        && node.time.is_none()
        && node.time_path.is_empty()
        && node.time_interval.is_none()
        && node.time_span.is_none()
        && node.aspect.is_none()
        && node.aspects.is_empty()
        && node.recurrence.is_empty()
        && node.interval_modifiers.is_empty()
        && node.space.is_none()
        && node.space_path.is_empty()
        && node.space_interval.is_none()
        && node.spatial_aspect.is_none()
        && node.spatial_aspects.is_empty()
        && node.spatial_recurrence.is_empty()
        && node.spatial_interval_modifiers.is_empty()
        && node.content.is_none()
        && node.embedded_questions.is_empty()
        && node.experiencer.is_none()
        && node.target.is_none()
        && node.scale.is_none()
        && node.subscript.is_none()
}

/// Complete base shape for the builder's current described-event encoding.
/// The abstraction kind fixes both the eventuality subtype and class; only one
/// optional simple time facet is admitted by the compact rule below.
#[requires(true)]
#[ensures(true)]
fn described_eventuality_base_is_exact(
    node: &EventualityNode,
    descriptor: &crate::model::Descriptor,
    kind: AbstractionKind,
) -> bool {
    let expected = match kind {
        AbstractionKind::Event => (
            EventualitySort::General,
            crate::model::EventualityClass::Event,
        ),
        AbstractionKind::Achievement => (
            EventualitySort::Achievement,
            crate::model::EventualityClass::Achievement,
        ),
        AbstractionKind::Process => (
            EventualitySort::Process,
            crate::model::EventualityClass::Process,
        ),
        AbstractionKind::Activity => (
            EventualitySort::Activity,
            crate::model::EventualityClass::Activity,
        ),
        AbstractionKind::State => (
            EventualitySort::State,
            crate::model::EventualityClass::State,
        ),
        AbstractionKind::Experience => (
            EventualitySort::Experience,
            crate::model::EventualityClass::Event,
        ),
        _ => return false,
    };
    node.denotation.category() == Some(ReferentCategory::Constant)
        && node.denotation.scope_dependence().is_some()
        && node.sort == expected.0
        && node.class == Some(expected.1)
        && node.indexical.is_none()
        && descriptor.kind == DescriptorKind::VeridicalDescription
        && descriptor.word == "lo"
        && descriptor.speaker.is_some()
        && descriptor.body.is_none()
        && descriptor.veridical.is_none()
        && descriptor.relative_clauses.is_empty()
        && descriptor.quantity.is_none()
        && descriptor.name.is_none()
        && descriptor.scale.is_none()
        && descriptor.definiteness.is_none()
        && descriptor.operand.is_none()
        && node.composition.is_none()
        && node.relative_clauses.is_empty()
        && node.assigned_names.is_empty()
        && node.adjuncts.is_empty()
        && node.actuality.is_none()
        && node.tense_modal.is_none()
        && node.time_path.is_empty()
        && node.time_interval.is_none()
        && node.time_span.is_none()
        && node.aspect.is_none()
        && node.aspects.is_empty()
        && node.recurrence.is_empty()
        && node.interval_modifiers.is_empty()
        && node.space.is_none()
        && node.space_path.is_empty()
        && node.space_interval.is_none()
        && node.spatial_aspect.is_none()
        && node.spatial_aspects.is_empty()
        && node.spatial_recurrence.is_empty()
        && node.spatial_interval_modifiers.is_empty()
        && node.content.is_some()
        && node.body.is_none()
        && node.parameters.is_empty()
        && node.arity.is_none()
        && node.embedded_questions.is_empty()
        && node.abstraction_kind == Some(kind)
        && node.experiencer.is_none()
        && node.target.is_none()
        && node.scale.is_none()
        && node.subscript.is_none()
}

/// Simple anchor record consumed by a described-event time intrinsic.
#[requires(true)]
#[ensures(ret.is_none_or(|time| !time.relation.is_empty()))]
fn exact_described_event_time_facet(
    node: &EventualityNode,
) -> Option<&crate::model::AnchorRelation> {
    let time = node.time.as_ref()?;
    (!time.sticky
        && time.inherited.is_none()
        && time.distance.is_none()
        && time.magnitude.is_none()
        && time.scalar_negation.is_none()
        && time.motion.is_none()
        && matches!(time.relation.as_str(), "before" | "after" | "at"))
    .then_some(time)
}

/// Shared exact recognition for `lo`, `le`, and `la`. The caller supplies the
/// scope proof because rendering knows its lexical environment while planning
/// intentionally accepts only the independently provable fixed case. Every
/// other descriptor and attachment coordinate is audited here once.
#[requires(graph.objects.contains_key(&described))]
#[ensures(true)]
fn recognize_description<'a>(
    graph: &'a SemanticGraph,
    plan: &ReferencePlan,
    described: SemanticObjectId,
    node: &'a ReferentNode,
    scope_is_default: bool,
) -> Option<DescriptionRecognition<'a>> {
    let descriptor = node.descriptor.as_ref()?;
    if !scope_is_default
        || !referent_except_descriptor_is_default(node)
        || descriptor.quantity.is_some()
        || descriptor.scale.is_some()
        || descriptor.definiteness.is_some()
        || descriptor.operand.is_some()
        || descriptor.veridical.is_some()
        || !descriptor
            .speaker
            .is_some_and(|speaker| object_is_indexical(graph, speaker, IndexicalKind::Speaker))
    {
        return None;
    }

    if descriptor.kind == DescriptorKind::Name && descriptor.word == "la" {
        let name = descriptor.name.as_deref()?;
        return (descriptor.body.is_none()
            && descriptor.relative_clauses.is_empty()
            && node.relative_clauses.is_empty())
        .then(|| new!(DescriptionRecognition::Name { name }));
    }
    if descriptor.name.is_some()
        || descriptor
            .relative_clauses
            .iter()
            .chain(node.relative_clauses.iter())
            .any(|clause| {
                clause.kind == RelativeClauseKind::Incidental && clause.veridical.is_some()
            })
    {
        return None;
    }

    let body = descriptor.body?;
    let (constructor, property, parameter) = match (descriptor.kind, descriptor.word.as_str()) {
        (DescriptorKind::VeridicalDescription, "lo") => (
            DescriptionConstructor::Lo,
            recognize_direct_description_property(graph, plan, body, described)?,
            None,
        ),
        (DescriptorKind::SpeakerDescription, "le") => {
            let (property, parameter) =
                recognize_speaker_description_property(graph, plan, body, described)?;
            (DescriptionConstructor::Le, property, Some(parameter))
        }
        _ => return None,
    };
    Some(new!(DescriptionRecognition::Property {
        constructor,
        property,
        parameter,
    }))
}

/// Exact direct `lo` property encoding: a restrictive atom whose x1 is the
/// described referent and whose remaining places are default contextual values.
#[requires(graph.objects.contains_key(&formula))]
#[ensures(true)]
fn recognize_direct_description_property<'a>(
    graph: &'a SemanticGraph,
    plan: &ReferencePlan,
    formula: SemanticObjectId,
    described: SemanticObjectId,
) -> Option<&'a str> {
    recognize_property_formula(graph, plan, formula, described)
}

/// Exact recursive `skicu` encoding for `le`, including its separately stored
/// unary property abstraction.
#[requires(graph.objects.contains_key(&formula))]
#[ensures(true)]
fn recognize_speaker_description_property<'a>(
    graph: &'a SemanticGraph,
    plan: &ReferencePlan,
    formula: SemanticObjectId,
    described: SemanticObjectId,
) -> Option<(&'a str, SemanticObjectId)> {
    let atom = graph.objects[&formula].as_formula()?.as_data();
    let data!(FormulaNode::Atom(atom)) = atom else {
        return None;
    };
    if !atom.bound_eventualities.is_empty() {
        return None;
    }
    let predication = graph.objects[&atom.predication].as_predication()?;
    let data!(PredicationRelation::Named { relation }) = predication.relation.as_data() else {
        return None;
    };
    if relation != "skicu"
        || predication.eventuality.is_some()
        || predication.mode != PredicationMode::Incidental
        || !predication_is_otherwise_plain(predication)
        || predication.arguments.len() != 4
        || !plain_argument_equals(&predication.arguments, 1, |id| {
            object_is_indexical(graph, id, IndexicalKind::Speaker)
        })
        || !plain_argument_equals(&predication.arguments, 2, |id| id == described)
        || !plain_argument_equals(&predication.arguments, 3, |id| {
            object_is_indexical(graph, id, IndexicalKind::Audience)
        })
    {
        return None;
    }
    let relation_value = plain_argument_value(&predication.arguments, 4)?;
    let relation_node = graph.objects[&relation_value].as_referent()?;
    if relation_node.sort != SemanticSort::Relation
        || !referent_except_abstraction_is_default(relation_node)
        || !matches!(
            relation_node
                .scope_dependence
                .as_ref()
                .map(|dependence| dependence.as_data()),
            Some(data!(ScopeDependence::Fixed))
        )
        || relation_node.parameters.len() != 1
        || relation_node.arity != Some(1)
        || relation_node.abstraction_kind != Some(AbstractionKind::Property)
        || !exact_parameter(
            graph,
            relation_node.parameters[0],
            SemanticSort::Entity,
            ParameterRole::PropertySlot,
            "ce'u",
        )
    {
        return None;
    }
    let parameter = relation_node.parameters[0];
    recognize_property_formula(graph, plan, relation_node.body?, parameter)
        .map(|property| (property, parameter))
}

/// Exact unary property atom used by both recognized description encodings.
#[requires(graph.objects.contains_key(&formula))]
#[ensures(true)]
fn recognize_property_formula<'a>(
    graph: &'a SemanticGraph,
    plan: &ReferencePlan,
    formula: SemanticObjectId,
    subject: SemanticObjectId,
) -> Option<&'a str> {
    let formula = graph.objects[&formula].as_formula()?;
    let data!(FormulaNode::Atom(atom)) = formula.as_data() else {
        return None;
    };
    if !atom.bound_eventualities.is_empty() {
        return None;
    }
    let predication = graph.objects[&atom.predication].as_predication()?;
    let data!(PredicationRelation::Named { relation }) = predication.relation.as_data() else {
        return None;
    };
    if predication.eventuality.is_some()
        || predication.mode != PredicationMode::Restrictive
        || !predication_is_otherwise_plain(predication)
        || plain_argument_value(&predication.arguments, 1) != Some(subject)
    {
        return None;
    }
    for (place, argument) in &predication.arguments {
        if place.get() == 1 {
            continue;
        }
        let value = plain_elided_argument_value(argument)?;
        let referent = graph.objects[&value].as_referent()?;
        if !default_elided_shape(referent)
            || !graph.objects[&value].diagnostics().is_empty()
            || plan.use_count(value) != 1
        {
            return None;
        }
        let dependence_matches =
            if subject.object_kind() == crate::model::SemanticObjectKind::Parameter {
                referent
                    .scope_dependence
                    .as_ref()
                    .and_then(|dependence| dependence.may_depend_on())
                    == Some(&BTreeSet::from([subject]))
            } else {
                matches!(
                    referent
                        .scope_dependence
                        .as_ref()
                        .map(|value| value.as_data()),
                    Some(data!(ScopeDependence::Fixed))
                )
            };
        if !dependence_matches {
            return None;
        }
    }
    Atom::try_new(relation.clone()).ok()?;
    Some(relation)
}

/// Borrow the atom payload of one exact formula object.
#[requires(graph.objects.contains_key(&formula))]
#[ensures(true)]
fn formula_atom(
    graph: &SemanticGraph,
    formula: SemanticObjectId,
) -> Option<&crate::model::AtomFormulaNode> {
    let formula = graph.objects[&formula].as_formula()?;
    let data!(FormulaNode::Atom(atom)) = formula.as_data() else {
        return None;
    };
    Some(atom)
}

/// Exact unary tanru modifier property with one private generated event.
#[requires(graph.objects.contains_key(&formula))]
#[ensures(ret.is_none_or(|(_, predication, event)| {
    graph.objects.contains_key(&predication) && graph.objects.contains_key(&event)
}))]
fn recognize_tanru_modifier_property<'a>(
    graph: &'a SemanticGraph,
    plan: &ReferencePlan,
    formula: SemanticObjectId,
    subject: SemanticObjectId,
) -> Option<(&'a str, SemanticObjectId, SemanticObjectId)> {
    let atom = formula_atom(graph, formula)?;
    if atom.bound_eventualities.len() != 1 {
        return None;
    }
    let event = atom.bound_eventualities[0].object_id();
    let predication = graph.objects[&atom.predication].as_predication()?;
    let data!(PredicationRelation::Named { relation }) = predication.relation.as_data() else {
        return None;
    };
    if predication.eventuality != Some(event)
        || predication.mode != PredicationMode::Restrictive
        || !predication_is_otherwise_plain(predication)
        || plain_argument_value(&predication.arguments, 1) != Some(subject)
        || !generated_event_is_default(graph, plan, formula, event)
    {
        return None;
    }
    for (place, argument) in &predication.arguments {
        if place.get() == 1 {
            continue;
        }
        let value = plain_elided_argument_value(argument)?;
        let referent = graph.objects[&value].as_referent()?;
        if !default_elided_shape(referent)
            || !graph.objects[&value].diagnostics().is_empty()
            || plan.use_count(value) != 1
            || referent
                .scope_dependence
                .as_ref()
                .and_then(|dependence| dependence.may_depend_on())
                != Some(&BTreeSet::from([subject]))
        {
            return None;
        }
    }
    Atom::try_new(relation.clone()).ok()?;
    Some((relation, atom.predication, event))
}

/// Fields that must be absent before a named predication is interpreted as a
/// concise property or description encoding.
#[requires(true)]
#[ensures(true)]
fn predication_is_otherwise_plain(node: &PredicationNode) -> bool {
    node.tanru_link.is_none()
        && node.place_questions.is_empty()
        && node.adjuncts.is_empty()
        && node.reciprocity.is_empty()
        && node.scalar_negation.is_none()
        && node.relation_metadata.is_none()
        && node.introduced_by.is_none()
}

/// Exact ordinary variable shape consumed by quantifier binders. The binding
/// prints the identity and sort; every other referent coordinate must therefore
/// be the unique plain-variable default. Provenance source is the ordinary
/// profile suppression, while diagnostics are kept out of compact binding so
/// their object attachment is retained by TypedGraph.
#[requires(graph.objects.contains_key(&variable))]
#[ensures(ret.is_none_or(|sort| graph.objects[&variable].sort() == Some(sort)))]
fn exact_plain_bound_variable(
    graph: &SemanticGraph,
    variable: SemanticObjectId,
) -> Option<SemanticSort> {
    let node = graph.objects[&variable].as_referent()?;
    (node.category == ReferentCategory::Variable
        && node.scope_dependence.is_none()
        && referent_payload_is_empty(node)
        && graph.objects[&variable].diagnostics().is_empty())
    .then_some(node.sort)
}

/// Exact parameter shape consumed by a binder declaration. The declaration
/// prints the sort; its fixed recognizer position entails role and introducer.
/// Subscripts and diagnostics are never silently absorbed.
#[requires(graph.objects.contains_key(&parameter))]
#[ensures(true)]
fn exact_parameter(
    graph: &SemanticGraph,
    parameter: SemanticObjectId,
    sort: SemanticSort,
    role: ParameterRole,
    introduced_by: &str,
) -> bool {
    let Some(node) = graph.objects[&parameter].as_parameter() else {
        return false;
    };
    node.sort == sort
        && node.role == role
        && node.introduced_by == introduced_by
        && node.subscript.is_none()
        && graph.objects[&parameter].diagnostics().is_empty()
}

/// Fixed question-word/parameter-role table used by exact question binding.
#[requires(true)]
#[ensures(ret.is_none_or(|(_, introduced_by)| !introduced_by.is_empty()))]
fn question_parameter_shape(kind: QuestionKind) -> Option<(ParameterRole, &'static str)> {
    Some(match kind {
        QuestionKind::Argument => (ParameterRole::ArgumentQuestion, "ma"),
        QuestionKind::Relation => (ParameterRole::RelationQuestion, "mo"),
        QuestionKind::Place => (ParameterRole::PlaceQuestion, "fi'a"),
        QuestionKind::Connective => (ParameterRole::ConnectiveQuestion, "ji"),
        QuestionKind::Tense => (ParameterRole::TenseQuestion, "cu'e"),
        QuestionKind::MathOperator => (ParameterRole::MathOperatorQuestion, "ma'o"),
        QuestionKind::Attitude => (ParameterRole::AttitudeQuestion, "pei"),
        QuestionKind::Quantity => (ParameterRole::QuantityQuestion, "xo"),
        QuestionKind::Truth | QuestionKind::Multiple => return None,
    })
}

/// Every question slot must account for both its slot role and the complete
/// underlying parameter record before the lambda projection may consume it.
#[requires(true)]
#[ensures(true)]
fn question_slots_are_exact(graph: &SemanticGraph, node: &crate::model::QuestionNode) -> bool {
    node.slots.iter().all(|slot| match slot.as_data() {
        data!(QuestionSlot::Homogeneous { parameter, role }) => match role {
            QuestionSlotRole::Answer => question_parameter_shape(node.kind).is_some_and(
                |(parameter_role, introduced_by)| {
                    exact_parameter(
                        graph,
                        *parameter,
                        node.domain,
                        parameter_role,
                        introduced_by,
                    )
                },
            ),
            QuestionSlotRole::RespectiveSlot => false,
        },
        data!(QuestionSlot::Typed {
            parameter,
            role,
            kind,
            domain,
        }) => {
            *role == QuestionSlotRole::Answer
                && *kind == node.kind
                && *domain == node.domain
                && match parameter {
                    None => *kind == QuestionKind::Truth,
                    Some(parameter) => question_parameter_shape(*kind).is_some_and(
                        |(parameter_role, introduced_by)| {
                            exact_parameter(
                                graph,
                                *parameter,
                                *domain,
                                parameter_role,
                                introduced_by,
                            )
                        },
                    ),
                }
        }
    })
}

/// Extract one plain filled argument.
#[requires(place > 0)]
#[ensures(true)]
fn plain_argument_value(
    arguments: &BTreeMap<PlaceIndex, ArgumentValue>,
    place: usize,
) -> Option<SemanticObjectId> {
    let argument = arguments.get(&PlaceIndex::new(place))?;
    (argument.kind == ArgumentValueKind::Filled
        && argument.quantity.is_none()
        && argument.relative_clauses.is_empty()
        && argument.command_target.is_none())
    .then_some(argument.value?)
}

/// Extract one ordinary `zo'e` argument whose surface distinction is exactly
/// the named provenance default. All semantic side fields must be absent.
#[requires(true)]
#[ensures(true)]
fn plain_elided_argument_value(argument: &ArgumentValue) -> Option<SemanticObjectId> {
    (argument.kind == ArgumentValueKind::Elided
        && argument.introduced_by.as_deref() == Some("zo'e")
        && argument.quantity.is_none()
        && argument.relative_clauses.is_empty()
        && argument.command_target.is_none())
    .then_some(argument.value?)
}

/// Test one plain argument with a typed identity predicate.
#[requires(place > 0)]
#[ensures(true)]
fn plain_argument_equals(
    arguments: &BTreeMap<PlaceIndex, ArgumentValue>,
    place: usize,
    predicate: impl FnOnce(SemanticObjectId) -> bool,
) -> bool {
    plain_argument_value(arguments, place).is_some_and(predicate)
}

/// Exact universal glyph rule: typed operator plus the complete ordinary `ro`
/// all/count quantity and no semantically relevant quantity side fields.
#[requires(true)]
#[ensures(true)]
fn exact_universal_quantity(
    graph: &SemanticGraph,
    operator: FormulaOperator,
    quantity: Option<SemanticObjectId>,
) -> bool {
    if operator != FormulaOperator::Forall {
        return false;
    }
    let Some(quantity) = quantity else {
        return false;
    };
    let Some(node) = graph.objects[&quantity].as_quantity() else {
        return false;
    };
    node.form == QuantityForm::All
        && node.scale == QuantityScale::Count
        && node.value.integer.is_none()
        && node.value.text.as_deref() == Some("ro")
        && node.value.math_expression.is_none()
        && node.value.question_parameters.is_empty()
        && node.comparison_set.is_none()
        && graph.objects[&quantity].diagnostics().is_empty()
}

/// Exact logical connective projection. Connector surface and locus are typed
/// provenance; a stored truth table must agree with the graph operator or
/// license the declared `∨(¬P,Q)` implication normalization.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|(_, children)| !children.is_empty()))]
fn exact_connective_projection(
    graph: &SemanticGraph,
    node: &crate::model::ConnectiveFormulaNode,
) -> Option<(&'static str, Vec<SemanticObjectId>)> {
    if let Some(connector) = &node.connector {
        if connector.parameter.is_some() {
            return None;
        }
        match connector.source.as_data() {
            data!(crate::model::ConnectorSource::SurfaceWord { .. })
            | data!(crate::model::ConnectorSource::ImplicitJuxtaposition) => {}
        }
        match connector.locus {
            crate::model::ConnectorLocus::Statement
            | crate::model::ConnectorLocus::Argument
            | crate::model::ConnectorLocus::Term
            | crate::model::ConnectorLocus::TermSet
            | crate::model::ConnectorLocus::Tense
            | crate::model::ConnectorLocus::Tag
            | crate::model::ConnectorLocus::Operand
            | crate::model::ConnectorLocus::Clause
            | crate::model::ConnectorLocus::PredicatePhrase
            | crate::model::ConnectorLocus::Predicate
            | crate::model::ConnectorLocus::PredicateInversion
            | crate::model::ConnectorLocus::PredicateUnit
            | crate::model::ConnectorLocus::PropertyAbstraction
            | crate::model::ConnectorLocus::PropertyInversion
            | crate::model::ConnectorLocus::Abstraction
            | crate::model::ConnectorLocus::Description
            | crate::model::ConnectorLocus::MathOperator
            | crate::model::ConnectorLocus::BareRaisedParticipant => {}
        }
        if connector.truth_table.as_deref() == Some("TFTT")
            && node.operator == FormulaOperator::Or
            && node.children.len() == 2
        {
            let negated = graph.objects[&node.children[0]].as_formula()?;
            let data!(FormulaNode::Connective(negation)) = negated.as_data() else {
                return None;
            };
            if negation.operator == FormulaOperator::Not
                && negation.children.len() == 1
                && negation.connector.is_none()
                && negation.eventuality.is_none()
                && negation.bound_eventualities.is_empty()
            {
                return Some(("→", vec![negation.children[0], node.children[1]]));
            }
            return None;
        }
        if connector
            .truth_table
            .as_deref()
            .is_some_and(|table| canonical_connective_truth_table(node.operator) != Some(table))
        {
            return None;
        }
    }
    let operator = match (node.operator, node.children.len()) {
        (FormulaOperator::Not, 1) => "¬",
        (FormulaOperator::And, 2..) => "∧",
        (FormulaOperator::Or, 2..) => "∨",
        (FormulaOperator::Implies, 2) => "→",
        (FormulaOperator::Iff, 2) => "↔",
        (FormulaOperator::ExclusiveOr, 2) => "⊕",
        _ => return None,
    };
    Some((operator, node.children.clone()))
}

/// Closed truth-table registry in builder row order `(TT, TF, FT, FF)`.
#[requires(true)]
#[ensures(ret.is_none_or(|table| table.len() == 4))]
fn canonical_connective_truth_table(operator: FormulaOperator) -> Option<&'static str> {
    match operator {
        FormulaOperator::And => Some("TFFF"),
        FormulaOperator::Or => Some("TTTF"),
        FormulaOperator::Implies => Some("TFTT"),
        FormulaOperator::Iff => Some("TFFT"),
        FormulaOperator::ExclusiveOr => Some("FTTF"),
        FormulaOperator::WhetherOrNot => Some("TTFF"),
        _ => None,
    }
}

/// Fixed composition constructor table.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn composition_operator_name(operator: CompositionOperator) -> &'static str {
    match operator {
        CompositionOperator::ConnectiveQuestion => "ConnectiveQuestion",
        CompositionOperator::Joint => "Joint",
        CompositionOperator::Mass => "Mass",
        CompositionOperator::Set => "Set",
        CompositionOperator::Sequence => "SequenceValue",
        CompositionOperator::Respectively => "RespectivelyValue",
        CompositionOperator::Union => "Union",
        CompositionOperator::Intersection => "Intersection",
        CompositionOperator::CrossProduct => "CrossProduct",
        CompositionOperator::UnorderedInterval => "UnorderedInterval",
        CompositionOperator::OrderedInterval => "OrderedInterval",
        CompositionOperator::CenteredInterval => "CenteredInterval",
    }
}

/// Fixed indexical atom table.
#[requires(true)]
#[ensures(!ret.is_empty())]
fn indexical_name(indexical: IndexicalKind) -> &'static str {
    match indexical {
        IndexicalKind::Speaker => "Speaker",
        IndexicalKind::Audience => "Audience",
        IndexicalKind::Now => "Now",
        IndexicalKind::Here => "Here",
    }
}

/// Numeric quantity shorthand table; text values never enter this function.
#[requires(true)]
#[ensures(true)]
fn numeric_quantity_form(form: QuantityForm) -> Option<&'static str> {
    match form {
        QuantityForm::AtLeast => Some("AtLeast"),
        QuantityForm::AtMost => Some("AtMost"),
        QuantityForm::MoreThan => Some("MoreThan"),
        QuantityForm::LessThan => Some("LessThan"),
        QuantityForm::Approximate => Some("Approximate"),
        QuantityForm::Enough => Some("Enough"),
        QuantityForm::TooMany => Some("TooMany"),
        QuantityForm::TooFew => Some("TooFew"),
        QuantityForm::Exact | QuantityForm::All | QuantityForm::Indefinite => None,
    }
}
