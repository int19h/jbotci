//! Conservative semantic elaboration from typed graph objects to typed kernel
//! values.
//!
//! Every recognizer in this module is an exact structural proof: a branch that
//! cannot account for every semantic field declines with its registered reason,
//! and there are no spelling guesses or best-effort reconstructions. What a
//! recognizer produces is a kernel value, so the kernel's constructors are the
//! type gate — a route that proves a graph shape but cannot construct a
//! well-typed value for it declines rather than emitting an untyped surface.
//!
//! Declining is silent when the reason is already recorded. `None` therefore
//! means "this position has no typed value, and whichever boundary was
//! responsible has already been logged"; a caller that receives it finishes
//! rendering its other children — so their boundaries are logged too — and then
//! declines in turn without inventing a second reason for one failure.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, invariant, new, requires};

use super::super::kernel::apply::PredicateSignature;
use super::super::kernel::binder::{
    Bind, Category, Declaration, Lambda, Let, LetRec, RecursiveDeclaration, TypedParameter,
    free_binders_of,
};
use super::super::kernel::content::{BinaryOp, Content, JunctionOp, QuantifierOp, Query};
use super::super::kernel::document::KernelDocument;
use super::super::kernel::intrinsic::Intrinsic;
use super::super::kernel::performable::{Act, Discourse, Performable, TranscriptEntry};
use super::super::kernel::predicate::{PlaceFill, PredTerm};
use super::super::kernel::types::{
    LexicalRoot, PlaceLabel, PositiveInteger, RelationRef, Row, RowSlot, TypeAtom, TypeExpr,
    Variable,
};
use super::super::kernel::value::{FnValue, Literal, Operand, RefComp, Value};
use super::identity::{crossing_default_variable, object_variable};
use super::planner::{GraphUsage, ProjectedIdentities, ReferencePlan, strong_components};
use super::purity::{intrinsic_requires_pure_property, is_pure_property};
use crate::model::{
    AbstractionKind, ActualityKind, Adjunct, ArgumentValue, ArgumentValueKind, DeicticProximity,
    DescriptorKind, EventualityDenotationData, EventualityNode, EventualitySort, FormulaNode,
    FormulaNodeData, FormulaOperator, IndexicalKind, MathExpressionNodeKindData, MathOperatorData,
    ParagraphTransition, ParameterRole, PlaceIndex, PredicationMode, PredicationNode,
    PredicationRelationData, QuantityForm, QuantityNode, QuantityScale, QuestionKind, QuestionMode,
    QuestionSlotData, QuestionSlotRole, ReferentCategory, ReferentNode, RelationLabelData,
    RelativeClauseKind, ScopeDependenceData, ScopeUseRole, SelectionSource, SemanticGraph,
    SemanticObjectData, SemanticObjectId, SemanticObjectKind, SemanticSort, SequenceNode,
    SequenceRelation, SignNode, UtteranceForce, UtteranceNode,
    semantic_scope_dependence_binder_universes,
};

/// Mutable counters kept separate from semantic rendering decisions.
#[invariant(true)]
#[derive(Debug, Default)]
struct ElaborationCounters {
    compact_objects: Cell<usize>,
    failures: CompactFallbackLog,
}

/// One failed compact projection edge.
///
/// The pair is the edge's identity, not merely a report about it. `owner` is
/// the graph object whose compact projection was attempted, and `cause` is the
/// exact typed boundary inside that projection which declined; the cause
/// vocabulary is closed and exhaustive, so it is a stable site identity rather
/// than free-form text. Two boundaries of one object are therefore two edges,
/// while re-reaching one boundary is the same edge.
///
/// Field order is also the channel's stable order: failures sort by owner, then
/// by declining boundary.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CompactFallback {
    pub(super) owner: SemanticObjectId,
    pub(super) cause: CompactFallbackCause,
}

/// Every distinct failed projection edge discovered by one elaboration pass.
///
/// One object's projection can be attempted more than once: a recognizer may
/// render children, decline, and route the whole object through
/// [`Elaborator::fallback_object`], which renders those same children again.
/// Keying by object alone answered that by keeping only the first boundary,
/// which silently discarded every later distinct boundary and so could not
/// carry specification section 16.1's one-record-per-failed-edge contract.
/// Keying by the whole edge keeps both laws instead: re-entering a boundary
/// that already failed adds nothing, so a declining wrapper never duplicates or
/// relabels a child's failure, while a genuinely different boundary on the same
/// owner survives as its own record.
#[invariant(true)]
#[derive(Debug, Default)]
struct CompactFallbackLog {
    edges: RefCell<BTreeSet<CompactFallback>>,
}

impl CompactFallbackLog {
    /// Record one failed projection edge, deduplicating exact re-entry.
    #[requires(true)]
    #[ensures(self.contains(edge), "a recorded edge is always retained")]
    #[ensures(
        self.len() <= old(self.len()) + 1,
        "re-entering one edge never adds a second record for it"
    )]
    fn record(&self, edge: CompactFallback) {
        self.edges.borrow_mut().insert(edge);
    }

    /// Whether this exact failed edge has already been recorded.
    #[requires(true)]
    #[ensures(true)]
    fn contains(&self, edge: CompactFallback) -> bool {
        self.edges.borrow().contains(&edge)
    }

    /// Number of distinct failed projection edges recorded so far.
    #[requires(true)]
    #[ensures(true)]
    fn len(&self) -> usize {
        self.edges.borrow().len()
    }

    /// Consume the log into the ordered per-edge diagnostic channel.
    #[requires(true)]
    #[ensures(
        ret.windows(2).all(|pair| pair[0] < pair[1]),
        "the channel is strictly increasing, so it is both ordered and deduplicated"
    )]
    fn into_ordered(self) -> Vec<CompactFallback> {
        self.edges.into_inner().into_iter().collect()
    }
}

/// One fixed reference computation collected at its selected host.
/// Version 0 deliberately keeps these distinct from pure `Let` values.
#[invariant(true)]
#[derive(Debug)]
struct ReferenceBinding {
    id: SemanticObjectId,
    declared_type: TypeExpr,
    computation: RefComp,
}

/// One position section 6.3 admits as the host of a raised `Bind`.
///
/// Raising ascends outwards until it meets a frame it may not leave. Three
/// kinds of frame exist, and the difference is exactly the specification's: a
/// `barrier` stops every computation — a performed force segment, a
/// reification, an opaque boundary, or the reference computation a nested one
/// runs inside per section 8.3 — while a binder frame stops only a computation
/// that depends on one of the binders it introduces, which is section 6.3's
/// lambda-dependency boundary. Everything the elaborator renders between two
/// frames is a transparent shell, so a computation with no dependency in scope
/// raises straight through it.
///
/// A `closure` frame stops nothing at all. It is not a raising boundary but an
/// *effect scope*: section 5.1 runs each omitted computation "at the dynamic
/// evaluation site of `Close`", left to right in numbered-place order and local
/// to that closure, so every `Close` is its own host position for the
/// computations it closes. A `Refer` that raises is unaffected — it passes
/// straight through — which is why this is a third kind rather than a binder
/// frame with an empty list.
///
/// A `placement` frame stops nothing either. It is the position one graph
/// object's rendered value occupies, which is where the declaration plan put
/// the identities it sited there — so a planned declaration whose value binds
/// itself rather than declaring itself has a host of its own instead of falling
/// back on whatever coarser scope is still open.
#[invariant(::Barrier => true, "a barrier stops every computation and carries no binder list")]
#[invariant(::Binders(_) => true, "any binder set is a legal lambda-dependency boundary")]
#[invariant(::Closure => true, "a closure scope is transparent to every raise")]
#[invariant(::Placement => true, "a placement position is transparent to every raise")]
#[derive(Debug)]
enum HostFrameKind {
    Barrier,
    Binders(BTreeSet<SemanticObjectId>),
    Closure,
    Placement,
}

impl HostFrameKind {
    /// Whether a computation with these dependencies must stop at this frame.
    #[requires(true)]
    #[ensures(true)]
    fn stops(&self, dependencies: &BTreeSet<SemanticObjectId>) -> bool {
        match self {
            Self::Barrier => true,
            Self::Binders(binders) => !binders.is_disjoint(dependencies),
            Self::Closure | Self::Placement => false,
        }
    }
}

/// Which `Close` of one graph object an effect scope is.
///
/// Section 5.1's evaluation site is a closure, and one graph object renders as
/// several: a predication is its own application plus one `Joi` operand per
/// adjunct, and each of those is its own `Close`. The plan's placement
/// coordinate is coarser than that — [`ReferencePlan::definition_site`] names
/// the containing *object* — so a shared default's schedule cannot be decided
/// on the object alone. Shared twice in the application, shared twice inside
/// one adjunct, and shared across the two are three different hosts: the first
/// two bind inside their own closure at the place that asks first, and only the
/// third has to stand at the common site above both.
#[invariant(::Main(_) => true, "an object's own application is one closure")]
#[invariant(::Adjunct(_, _) => true, "each adjunct is a `Joi` operand and so a closure of its own")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClosureSite {
    /// The object's own relation applied to its numbered places.
    Main(SemanticObjectId),
    /// One adjunct of that object, named by its position in the adjunct list.
    Adjunct(SemanticObjectId, usize),
}

#[invariant(
    true,
    "the kind already constrains itself and any binding list is hostable here"
)]
#[derive(Debug)]
struct HostFrame {
    kind: HostFrameKind,
    /// The `Close` this scope is the whole of, when it is one the notation can
    /// name. Only a closure scope carries one: section 5.1's rule 2 places a
    /// graph-shared default at the closure whose own places share it, and
    /// matching that closure against an open scope is what lets the shared
    /// `Bind` be scheduled with the closure's own numbered-place walk instead
    /// of appended after it.
    owner: Option<ClosureSite>,
    /// The lexical binders live at this position, so a computation placed here
    /// can be checked against the names its computation actually uses.
    live: BTreeSet<SemanticObjectId>,
    bindings: Vec<ReferenceBinding>,
}

impl HostFrame {
    /// Whether a computation depending on these binders may stand here.
    ///
    /// Section 6.3 rejects the smallest affected dynamic subgraph when the
    /// rules and the graph do not determine one legal host, and a host that
    /// does not have every dependency live is exactly that: the printed binder
    /// would name a variable no enclosing form binds.
    #[requires(true)]
    #[ensures(true)]
    fn admits(&self, dependencies: &BTreeSet<SemanticObjectId>) -> bool {
        dependencies.is_subset(&self.live)
    }

    /// Bind one computation here, or recognize that this identity is already
    /// bound here.
    ///
    /// One graph identity is one name. A second occurrence hosted at the same
    /// position is a *use* of the binder the first one introduced, not a second
    /// mint of the same reference — section 8.3's fixed reference is fixed for
    /// the whole scope of its binder. Pushing it twice would introduce one name
    /// inside a live binder of that name, which the document audit refuses.
    #[requires(true)]
    #[ensures(self.bindings.iter().any(|bound| bound.id == old(binding.id)))]
    fn bind(&mut self, binding: ReferenceBinding) {
        if self.bindings.iter().any(|bound| bound.id == binding.id) {
            return;
        }
        self.bindings.push(binding);
    }
}

/// What one live lexical binder was declared as.
///
/// A use has to be constructed at exactly the type its binder printed, or the
/// document audit rejects it, so the rendering environment carries the
/// declaration rather than only the identity.
#[invariant(::Value(_) => true, "a declared value type is whatever its binder printed")]
#[invariant(::Predicate(_) => true, "a declared row is whatever its binder printed")]
#[derive(Debug, Clone)]
enum BoundValue {
    /// An ordinary value binder: a hosted reference, a quantifier variable, a
    /// generated event, or a property parameter.
    Value(TypeExpr),
    /// The open-row relation variable of a `mo` question.
    Predicate(Row),
}

/// The lexical binders live at one rendering position.
type Bound = BTreeMap<SemanticObjectId, BoundValue>;

/// One host's whole declaration block, before it knows what it wraps.
///
/// The two blocks are kept apart because the kernel's binding forms are: a
/// `LetRec` group is a nonempty set of inert lambdas that see each other, and a
/// `Let` block is a sequence of ordinary declarations.
/// One block of a host's group, after exact SCC condensation.
///
/// `recursive` is a property of the group's dependency graph, not of how far
/// topological emission happened to get: it is set exactly for a component that
/// is a genuine cycle — two or more identities that see each other, or one that
/// sees itself.
#[invariant(!members.is_empty(), "a declaration block declares at least one name")]
#[invariant(!*recursive || !members.is_empty(), "a recursive block is a nonempty cycle")]
#[derive(Debug, Clone)]
struct DeclarationBlock {
    members: Vec<SemanticObjectId>,
    recursive: bool,
}

#[invariant(::Inert(_) => true, "each declaration validated its own initializer")]
#[invariant(::Recursive(_) => true, "each declaration validated its own inert lambda")]
#[derive(Debug)]
enum HostedGroup {
    Inert(Vec<Declaration>),
    Recursive(Vec<RecursiveDeclaration>),
}

impl HostedGroup {
    /// Bind this group over one hosted value, in the value's own category.
    #[requires(true)]
    #[ensures(true)]
    fn wrap(self, body: Elaborated) -> Option<Elaborated> {
        match body {
            Elaborated::Performable(body) => self
                .bind(body, Performable::Let, Performable::LetRec)
                .map(Elaborated::Performable),
            Elaborated::Operand(Operand::Value(body)) => self
                .bind(body, Value::let_form, Value::let_rec_form)
                .map(|value| Elaborated::Operand(Operand::Value(value))),
            Elaborated::Operand(Operand::Content(body)) => self
                .bind(body, Content::let_form, Content::let_rec_form)
                .map(|content| Elaborated::Operand(Operand::Content(content))),
            Elaborated::Operand(Operand::Predicate(body)) => self
                .bind(body, PredTerm::let_form, PredTerm::let_rec_form)
                .map(|term| Elaborated::Operand(Operand::Predicate(term))),
            Elaborated::Operand(Operand::Function(body)) => self
                .bind(body, FnValue::let_form, FnValue::let_rec_form)
                .map(|callable| Elaborated::Operand(Operand::Function(callable))),
            // Sections 2.2 and 3.1 give a query, act, discourse, or transcript
            // entry no binding form of its own, so a declaration group planned
            // at one of those positions has nowhere to stand.
            Elaborated::Operand(_) => None,
        }
    }

    /// Apply this group through one category's two binding constructors.
    #[requires(true)]
    #[ensures(true)]
    fn bind<C: Category, F, G>(self, body: C, inert: F, recursive: G) -> Option<C>
    where
        F: FnOnce(Let<C>) -> C,
        G: FnOnce(LetRec<C>) -> C,
    {
        match self {
            Self::Inert(declarations) => Let::new(declarations, body).ok().map(inert),
            Self::Recursive(declarations) => LetRec::new(declarations, body).ok().map(recursive),
        }
    }
}

/// One elaborated graph object, in whichever kernel category it inhabits.
#[invariant(::Operand(_) => true, "the kernel value inside already validated itself")]
#[invariant(::Performable(_) => true, "the kernel value inside already validated itself")]
#[derive(Debug, Clone)]
enum Elaborated {
    Operand(Operand),
    Performable(Performable),
}

impl Elaborated {
    /// Read this value as an operand, which is every category but a performable.
    #[requires(true)]
    #[ensures(true)]
    fn into_operand(self) -> Option<Operand> {
        match self {
            Self::Operand(operand) => Some(operand),
            Self::Performable(_) => None,
        }
    }

    /// Read this value as content.
    #[requires(true)]
    #[ensures(true)]
    fn into_content(self) -> Option<Content> {
        match self.into_operand()? {
            Operand::Content(content) => Some(content),
            _ => None,
        }
    }

    /// Whether this category has a binding form of its own.
    ///
    /// Sections 2.2 and 3.1 give a query, act, discourse or transcript entry no
    /// `Bind`, exactly as they give it no `Let`, so a computation planned at a
    /// position one of those occupies has nowhere to stand.
    #[requires(true)]
    #[ensures(true)]
    fn hosts_reference_bindings(&self) -> bool {
        match self {
            Self::Performable(_)
            | Self::Operand(
                Operand::Value(_)
                | Operand::Content(_)
                | Operand::Predicate(_)
                | Operand::Function(_),
            ) => true,
            Self::Operand(_) => false,
        }
    }

    /// Read this value as a performable, crossing an act or discourse operand
    /// onto the implicit performance spine.
    #[requires(true)]
    #[ensures(true)]
    fn into_performable(self) -> Option<Performable> {
        match self {
            Self::Performable(performable) => Some(performable),
            Self::Operand(Operand::Act(act)) => Some(Performable::Act(act)),
            Self::Operand(Operand::Entry(entry)) => Some(Performable::Entry(entry)),
            Self::Operand(Operand::Discourse(discourse)) => Some(Performable::Discourse(discourse)),
            Self::Operand(_) => None,
        }
    }
}

/// The predication modes one content boundary licenses.
///
/// Specification section 14.3 desugars mode away rather than representing it:
/// an asserted predication is an explicit `Assert` at its owning force
/// boundary, an inert one is a bare content value with no force, and a
/// restrictive one is the body of a reference property or a quantifier
/// restriction. So the printed bytes of the two non-asserted modes are
/// identical and only their *position* differs, which is why the boundary
/// table — not the kernel — is where mode is consumed.
///
/// The distinction the table must never blur is asserted against
/// non-asserted. An `asserted` predication inside a property, or a
/// non-asserted one at a force boundary, is a graph inconsistency and keeps
/// failing; admitting either would erase force.
#[invariant(::Unlicensed => true, "no boundary is responsible for this position")]
#[invariant(::Force => true, "a force boundary licenses assertion and nothing else")]
#[invariant(::NoForce => true, "a content boundary with no force of its own")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AcceptedModes {
    /// An ordinary argument position or a declined object's re-render: no
    /// content boundary stands here, so no predication mode is licensed.
    Unlicensed,
    /// An utterance's asserted content, or a question body.
    Force,
    /// A quantifier restriction, a description property or relative clause, or
    /// an abstraction's body or content.
    NoForce,
}

impl AcceptedModes {
    /// Whether this boundary licenses one predication's recorded mode.
    #[requires(true)]
    #[ensures(
        !ret || (self == AcceptedModes::Force) == (mode == PredicationMode::Asserted),
        "force is admitted at a force boundary and nowhere else"
    )]
    fn admits(self, mode: PredicationMode) -> bool {
        match self {
            Self::Unlicensed => false,
            Self::Force => mode == PredicationMode::Asserted,
            Self::NoForce => {
                matches!(mode, PredicationMode::Restrictive | PredicationMode::Inert)
            }
        }
    }
}

/// What one exact route did with the object it was offered.
///
/// The three cases are not interchangeable. A route whose graph shape does not
/// match has said nothing about the object, so the next route — and finally the
/// enclosing family's own boundary — still owns it. A route that *proved* its
/// shape owns the object outright: when its content declines, the boundary that
/// declined has already recorded why, and adding this object's family fallback
/// beside it would attribute one failure to two owners.
#[invariant(::Unmatched => true, "an unmatched route made no claim about the object")]
#[invariant(::Value(_) => true, "the kernel value inside already validated itself")]
#[invariant(::Declined => true, "a declining route has already recorded its boundary")]
#[derive(Debug)]
enum Routed {
    Unmatched,
    Value(Operand),
    Declined,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum CompactFallbackCause {
    UnrecognizedObjectFamily(SemanticObjectKind),
    UtteranceWithoutContent,
    ForceFieldsRequireRecord,
    SequenceFields,
    ConnectiveMetadata,
    UnrecognizedConnective,
    QuantifierVariableFields,
    QuantifierFields,
    /// The placed property failed a registry-only `PureProperty` requirement.
    PurityUnproven,
    /// A described `selectionSource` the restriction does not certify. The
    /// model states that a described domain restricts the candidate with
    /// `memberOf(candidate, description)`, but nothing in the model's
    /// invariants makes the two agree, so the exact renderer audits the
    /// correspondence and fails closed rather than printing a quantifier whose
    /// domain has quietly disappeared.
    SelectionSourceUncertified,
    RespectivelySlotFields,
    PredicationSideFields,
    PredicationModeUnrepresentable,
    NonAtomicRelation,
    CompositionPredication,
    ArgumentFields,
    /// A section-11.3 graph operand does not inhabit the crossing's declared
    /// trailing type. The graph is structurally recognized, but the typed
    /// kernel correctly refuses the application.
    CrossingOperandMismatch,
    AdjunctFields,
    CompositionFields,
    ConstantWithoutDependence,
    /// Section 6.3's closing rule: the placement rules and the graph together
    /// determine no legal host for a dynamic computation, because no open
    /// position has every binder it depends on live.
    DynamicHostIllegal,
    /// The same closing rule reached from the other side: the plan sited a
    /// dynamic computation at one object, and that object's own category has no
    /// binding form to write it at. Hosting it at some enclosing scope instead
    /// would place it where the plan did not, so the site is refused.
    PlacedHostWithoutBindingForm,
    ReferentFields,
    UnboundGeneratedEvent,
    UnrepresentableRecursiveValue,
    DefinitionTypeUnrepresentable,
    EventualityFacets,
    AbstractionAboutUnspecified,
    QuestionSlotFields,
    AskForceWithoutQuestion,
    MathSideFields,
    QuantityFields,
    MathLiteralDenotes,
    MathOperatorFields,
    SignFields,
    /// A route the registry declares carried proved its graph shape and then
    /// could not construct a well-typed kernel value for it. That is a defect in
    /// this renderer rather than a property of the graph, so it takes the one
    /// registered `ImplementationInvariant` reason instead of borrowing a
    /// route-unavailable one and hiding as backlog.
    TypedConstructionRejected,
}

impl CompactFallbackCause {
    /// Exact registered reason used by the current conservative boundary.
    #[requires(true)]
    #[ensures(ret.starts_with("smusni.projection."))]
    pub(super) fn reason_id(self) -> &'static str {
        match self {
            Self::UnrecognizedObjectFamily(kind) => match kind {
                SemanticObjectKind::DisplayedContent | SemanticObjectKind::Utterance => {
                    "smusni.projection.force-reduction-unrepresentable"
                }
                SemanticObjectKind::Parameter => {
                    "smusni.projection.higher-order-crossing-unlicensed"
                }
                SemanticObjectKind::RelationMetadata => {
                    "smusni.projection.lexical-signature-missing-or-stale"
                }
                _ => "smusni.projection.relation-reduction-unregistered-or-inexact",
            },
            Self::UtteranceWithoutContent
            | Self::ForceFieldsRequireRecord
            | Self::SequenceFields => "smusni.projection.force-reduction-unrepresentable",
            Self::ConnectiveMetadata
            | Self::UnrecognizedConnective
            | Self::PredicationSideFields
            | Self::PredicationModeUnrepresentable
            | Self::NonAtomicRelation => {
                "smusni.projection.relation-reduction-unregistered-or-inexact"
            }
            Self::QuantifierVariableFields
            | Self::QuantifierFields
            | Self::SelectionSourceUncertified => {
                "smusni.projection.quantifier-effect-export-illegal"
            }
            Self::PurityUnproven => "smusni.projection.prelude-reduction-unavailable",
            Self::RespectivelySlotFields => "smusni.projection.simultaneous-termset-unlicensed",
            Self::CompositionPredication | Self::CompositionFields => {
                "smusni.projection.relation-former-reduction-unavailable"
            }
            Self::ArgumentFields | Self::CrossingOperandMismatch => {
                "smusni.projection.predicate-fill-type-or-arity-mismatch"
            }
            Self::AdjunctFields => "smusni.projection.modal-tag-reduction-unregistered",
            Self::ConstantWithoutDependence | Self::ReferentFields => {
                "smusni.projection.reference-description-unrepresentable"
            }
            Self::DynamicHostIllegal | Self::PlacedHostWithoutBindingForm => {
                "smusni.projection.scope-dependency-without-binder"
            }
            Self::UnboundGeneratedEvent => "smusni.projection.generated-eventuality-unbound",
            Self::UnrepresentableRecursiveValue => {
                "smusni.projection.unguarded-or-unrepresentable-scc"
            }
            Self::DefinitionTypeUnrepresentable => {
                "smusni.projection.higher-order-crossing-unlicensed"
            }
            Self::EventualityFacets => "smusni.projection.event-facet-reduction-unregistered",
            Self::AbstractionAboutUnspecified => "smusni.projection.abstraction-about-unspecified",
            Self::QuestionSlotFields | Self::AskForceWithoutQuestion => {
                "smusni.projection.question-domain-or-answer-mismatch"
            }
            Self::QuantityFields => "smusni.projection.quantity-reduction-unregistered",
            Self::MathSideFields | Self::MathLiteralDenotes | Self::MathOperatorFields => {
                "smusni.projection.math-reduction-unregistered"
            }
            Self::SignFields => "smusni.projection.sign-identity-missing",
            Self::TypedConstructionRejected => "smusni.projection.unknown-registry-coordinate",
        }
    }

    /// Stable human-readable statement of why the compact projection declined.
    ///
    /// This is finer-grained than [`Self::reason_id`] on purpose: several
    /// distinct causes share one registered reason, and a per-edge diagnostic
    /// record should still say which boundary was actually reached. The text is
    /// `&'static str` so a record's message is fixed by its cause rather than
    /// composed at the failure site.
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(super) fn message(self) -> &'static str {
        match self {
            Self::UnrecognizedObjectFamily(kind) => match kind {
                SemanticObjectKind::DisplayedContent | SemanticObjectKind::Utterance => {
                    "no compact force reduction is registered for this object family"
                }
                SemanticObjectKind::Parameter => {
                    "this renderer does not yet project the higher-order crossing this parameter value requires"
                }
                SemanticObjectKind::RelationMetadata => {
                    "the lexical signature for this relation is missing or stale"
                }
                _ => "no compact relation reduction is registered for this object family",
            },
            Self::UtteranceWithoutContent => "the utterance act has no content to close",
            Self::ForceFieldsRequireRecord => {
                "utterance asides or vocative force require an utterance record"
            }
            Self::SequenceFields => "no compact discourse form covers these sequence fields",
            Self::ConnectiveMetadata => "connective metadata has no compact representation",
            Self::UnrecognizedConnective => {
                "the connective is not a registered ordinary truth function"
            }
            Self::QuantifierVariableFields => {
                "the quantifier's bound-variable fields have no compact representation"
            }
            Self::QuantifierFields => "the quantifier shape is not a registered compact reduction",
            Self::PurityUnproven => {
                "the placed property does not satisfy the registered purity precondition"
            }
            Self::SelectionSourceUncertified => {
                "the restriction does not state the described selection source's membership"
            }
            Self::RespectivelySlotFields => {
                "simultaneous termset slots are not licensed by version 0"
            }
            Self::PredicationSideFields => {
                "predication side fields have no compact projection at this boundary"
            }
            Self::PredicationModeUnrepresentable => {
                "the predication mode is not representable at this boundary"
            }
            Self::NonAtomicRelation => "the relation spelling is not a lexical atom",
            Self::CompositionPredication => {
                "a composition predication has no registered relation former"
            }
            Self::ArgumentFields => {
                "the argument map does not match the predicate's fill type or arity"
            }
            Self::CrossingOperandMismatch => {
                "the abstraction's trailing graph operand does not inhabit the crossing's declared type"
            }
            Self::AdjunctFields => "no registered modal-tag reduction covers this adjunct",
            Self::CompositionFields => {
                "these composition fields have no registered relation former"
            }
            Self::ConstantWithoutDependence => {
                "a constant referent carries no recorded scope dependence"
            }
            Self::DynamicHostIllegal => {
                "no legal host has every binder this dynamic computation depends on"
            }
            Self::PlacedHostWithoutBindingForm => {
                "the site this dynamic computation was placed at has no binding form"
            }
            Self::ReferentFields => {
                "these reference or description fields have no compact projection"
            }
            Self::UnboundGeneratedEvent => "a generated eventuality has no lexical binding site",
            Self::UnrepresentableRecursiveValue => {
                "this recursive value is unguarded or lexically unrepresentable"
            }
            Self::DefinitionTypeUnrepresentable => {
                "this renderer does not yet project a shared definition at this higher-order type"
            }
            Self::EventualityFacets => "these event facets have no registered compact reduction",
            Self::AbstractionAboutUnspecified => {
                "`tu'a` withholds which abstraction about the operand is meant, and version 0 \
                 specifies no faithful underspecified crossing for it (tracked spec gap, \
                 specification section 14.4)"
            }
            Self::QuestionSlotFields => {
                "the question's domain or answer slot does not match a compact form"
            }
            Self::AskForceWithoutQuestion => {
                "ask force carries content that is not a typed question, so no well-typed act exists"
            }
            Self::MathSideFields => "these math side fields have no registered reduction",
            Self::QuantityFields => "this quantity has no registered compact reduction",
            Self::MathLiteralDenotes => "this math literal's denotation is unregistered",
            Self::MathOperatorFields => "these math operator fields have no registered reduction",
            Self::SignFields => "the sign's graph-owned identity is missing",
            Self::TypedConstructionRejected => {
                "this renderer proved a compact route for this object and then failed to construct \
                 a well-typed kernel value for it"
            }
        }
    }
}

/// Complete result of exact descriptor recognition. Planning and rendering
/// consume this same value so support is never projected before the renderer
/// has proved the corresponding compact constructor.
#[invariant(::Property { constructor, property, parameter, .. } => !property.is_empty() && match constructor {
    DescriptionConstructor::Lo => parameter.is_none(),
    DescriptionConstructor::Le => parameter.as_ref().is_some_and(|id| id.object_kind() == SemanticObjectKind::Parameter),
})]
#[invariant(::Name { name } => !name.is_empty())]
#[derive(Debug, Clone, Copy)]
enum DescriptionRecognition<'a> {
    Property {
        constructor: DescriptionConstructor,
        property: &'a str,
        arguments: &'a BTreeMap<PlaceIndex, ArgumentValue>,
        parameter: Option<SemanticObjectId>,
    },
    Name {
        name: &'a str,
    },
}

/// The live binders one rendered property actually names.
///
/// Section 6.3 stops a raised reference at "lambda regions whose binders the
/// computation depends on", and that is a property of the computation, not a
/// permission granted to it. The recorded `mayDependOn` universe answers a
/// different question — which binders a `Context` lookup is *allowed* to
/// resolve against, which section 5.1 then prints verbatim — so using it as a
/// stop set anchors a description inside quantifiers its property never
/// mentions. The kernel already carries the exact answer: a lambda's free
/// binders, minus its own parameter.
///
/// Every free binder of a rendered property is one of the binders live where it
/// was rendered, so intersecting against `bound` recovers the graph identities
/// the frame stack is keyed by without inverting the variable spelling.
#[requires(true)]
#[ensures(ret.iter().all(|id| bound.contains_key(id)))]
fn property_dependencies(bound: &Bound, property: &Lambda<Content>) -> BTreeSet<SemanticObjectId> {
    let free = property.free_binders();
    bound
        .keys()
        .copied()
        .filter(|id| free.contains(&object_variable(*id)))
        .collect()
}

/// Nest single-entry dynamic bindings in discovery order. The first reference
/// discovered is the outermost handler, matching left-to-right evaluation.
///
/// The category is the host's own, so one rule serves every position a
/// section-6.3 host can occupy: a force segment binds over a `Performable`, a
/// quantifier scope or a description property binds over the `Content` it
/// abstracts.
#[requires(true)]
#[ensures(true)]
fn wrap_reference_bindings<C: Category, F>(
    bindings: Vec<ReferenceBinding>,
    body: C,
    form: F,
) -> Option<C>
where
    F: Fn(Bind<C>) -> C,
{
    bindings.into_iter().rev().try_fold(body, |body, binding| {
        Bind::new(
            object_variable(binding.id),
            binding.declared_type,
            binding.computation,
            body,
        )
        .ok()
        .map(&form)
    })
}

/// Bind one placement site's hosted computations over the value that stands
/// there, in that value's own category.
///
/// The categories are exactly the ones [`HostedGroup::wrap`] declares: a query,
/// act, discourse or transcript entry has no binding form at all, and reports
/// so rather than letting the computation drift to a position the plan did not
/// name.
#[requires(!bindings.is_empty())]
#[ensures(true)]
fn bind_at_site(bindings: Vec<ReferenceBinding>, body: Elaborated) -> Option<Elaborated> {
    match body {
        Elaborated::Performable(body) => {
            wrap_reference_bindings(bindings, body, Performable::Bind).map(Elaborated::Performable)
        }
        Elaborated::Operand(Operand::Value(body)) => {
            wrap_reference_bindings(bindings, body, Value::bind_form)
                .map(|value| Elaborated::Operand(Operand::Value(value)))
        }
        Elaborated::Operand(Operand::Content(body)) => {
            wrap_reference_bindings(bindings, body, Content::bind_form)
                .map(|content| Elaborated::Operand(Operand::Content(content)))
        }
        Elaborated::Operand(Operand::Predicate(body)) => {
            wrap_reference_bindings(bindings, body, PredTerm::bind_form)
                .map(|term| Elaborated::Operand(Operand::Predicate(term)))
        }
        Elaborated::Operand(Operand::Function(body)) => {
            wrap_reference_bindings(bindings, body, FnValue::bind_form)
                .map(|callable| Elaborated::Operand(Operand::Function(callable)))
        }
        Elaborated::Operand(_) => None,
    }
}

/// Why one place map produced no ordered fills.
///
/// The two cases take different routes: a map this notation cannot represent is
/// this predication's own declined boundary, while a value that declined
/// already recorded its own, and reporting both would attribute one failure to
/// two owners.
#[invariant(::Unrepresentable => true, "a unit discriminator carries nothing to constrain")]
#[invariant(::ValueDeclined => true, "a unit discriminator carries nothing to constrain")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FillFailure {
    Unrepresentable,
    ValueDeclined,
}

/// One rendered place fill, before it is applied to a row.
///
/// `place` is `None` for the distinguished event place, which is labelled by
/// name rather than by number; `labelled` says whether the cursor was already
/// standing on this place, which is what decides between a plain fill and a
/// `:n` one.
#[invariant(*labelled || place.is_some(), "the event place is always named")]
#[derive(Debug, Clone)]
struct RenderedFill {
    place: Option<u32>,
    labelled: bool,
    value: Operand,
}

/// Apply one ordered fill list to a predicate term.
///
/// An application with no fills at all is just the term: section 4.2's
/// application form fills at least one place.
#[requires(true)]
#[ensures(true)]
fn apply_fills(term: PredTerm, fills: Vec<RenderedFill>) -> Option<PredTerm> {
    if fills.is_empty() {
        return Some(term);
    }
    let fills = fills
        .into_iter()
        .map(|fill| {
            let fill = fill.into_data();
            match (fill.place, fill.labelled) {
                (None, _) => PlaceFill::eventuality(fill.value),
                (Some(place), true) => {
                    PlaceFill::numbered(PositiveInteger::from_u32(place), fill.value)
                }
                (Some(_), false) => PlaceFill::plain(fill.value),
            }
        })
        .collect();
    PredTerm::applied(term, fills).ok()
}

/// The places one argument map deletes.
#[requires(true)]
#[ensures(true)]
fn deleted_places(arguments: &BTreeMap<PlaceIndex, ArgumentValue>) -> BTreeSet<PlaceIndex> {
    arguments
        .iter()
        .filter(|(_, argument)| argument.kind == ArgumentValueKind::Deleted)
        .map(|(place, _)| *place)
        .collect()
}

/// How many places of one argument map are filled by exactly this identity.
///
/// Only the filled value counts. A place that names the identity some other
/// way — a quantity, a relative clause — is an occurrence this count does not
/// see, and the caller compares against the plan's own use count precisely so
/// that such a place makes the closure unnameable rather than miscounted.
#[requires(true)]
#[ensures(ret <= arguments.len())]
fn fills_naming(arguments: &BTreeMap<PlaceIndex, ArgumentValue>, id: SemanticObjectId) -> usize {
    arguments
        .values()
        .filter(|argument| argument.value == Some(id))
        .count()
}

/// The ordinary referential type an unfilled or deleted place accepts.
#[requires(true)]
#[ensures(matches!(ret, TypeExpr::Referents(_)))]
fn entity_referents() -> TypeExpr {
    TypeExpr::Referents(Box::new(TypeExpr::Atom(TypeAtom::Entity)))
}

/// Abstract one content body over one typed parameter.
#[requires(true)]
#[ensures(true)]
fn property_lambda(parameter: &TypedParameter, body: Option<Content>) -> Option<Lambda<Content>> {
    Lambda::new(vec![parameter.clone()], body?).ok()
}

/// Abstract one content body as a callable value.
///
/// A property in registered-operand position is a `Fn` value rather than a
/// `Content` binder, which is why it carries the joined operand category in its
/// body while a quantifier's own lambda does not.
#[requires(true)]
#[ensures(true)]
fn callable_property(parameter: &TypedParameter, body: Option<Content>) -> Option<FnValue> {
    Lambda::new(vec![parameter.clone()], Operand::Content(body?))
        .ok()
        .map(FnValue::lambda)
}

/// Outcome of constructing a generalized-quantifier reduction.
#[invariant(::Rendered(_) => true)]
#[invariant(::DeclinedOperand => true)]
#[invariant(::PurityUnproven => true)]
#[invariant(::ConstructionRejected => true)]
#[derive(Debug)]
enum GeneralizedQuantification {
    Rendered(Content),
    DeclinedOperand,
    PurityUnproven,
    ConstructionRejected,
}

/// Build section 9.4's application of one generalized quantifier to its scope.
///
/// The restriction is an operand of the registered constructor and the nuclear
/// scope is the argument of the generalized quantifier it returns; both are
/// properties of the same binder.
#[requires(matches!(constructor, Intrinsic::Some | Intrinsic::No | Intrinsic::Every))]
#[ensures(true)]
fn generalized_quantification(
    constructor: Intrinsic,
    parameter: &TypedParameter,
    restriction: Option<Content>,
    body: Option<Content>,
) -> GeneralizedQuantification {
    let (Some(restriction), Some(scope)) = (
        callable_property(parameter, restriction),
        callable_property(parameter, body),
    ) else {
        return GeneralizedQuantification::DeclinedOperand;
    };
    if intrinsic_requires_pure_property(constructor, 0) && !is_pure_property(&restriction) {
        return GeneralizedQuantification::PurityUnproven;
    }
    let Ok(quantifier) = FnValue::intrinsic(constructor, vec![Operand::Function(restriction)])
    else {
        return GeneralizedQuantification::ConstructionRejected;
    };
    match Content::apply(quantifier, vec![Operand::Function(scope)]) {
        Ok(content) => GeneralizedQuantification::Rendered(content),
        Err(_) => GeneralizedQuantification::ConstructionRejected,
    }
}

/// One `Bind` lifted off the value it used to wrap.
#[invariant(true)]
#[derive(Debug)]
struct RaisedBinding {
    variable: Variable,
    declared_type: TypeExpr,
    computation: RefComp,
}

/// Lift the reference computations a lone sequence item hosts out of it.
///
/// Section 6.3 raises a computation through transparent administrative shells,
/// and a paragraph transition over one item is exactly that: hosting the `Bind`
/// under the transition would trap it inside a `Discourse` operand instead. A
/// sequence of two or more items keeps every item's own hosts where they are,
/// because `Do` performs its operands directly.
#[requires(true)]
#[ensures(ret.1.len() == old(items.len()))]
fn raise_item_bindings(mut items: Vec<Performable>) -> (Vec<RaisedBinding>, Vec<Performable>) {
    let mut bindings = Vec::new();
    if items.len() != 1 {
        return (bindings, items);
    }
    let mut item = items.pop().expect("one item");
    while let Performable::Bind(form) = item {
        bindings.push(RaisedBinding {
            variable: form.variable().clone(),
            declared_type: form.declared_type().clone(),
            computation: form.computation().clone(),
        });
        item = form.body().clone();
    }
    (bindings, vec![item])
}

/// Reapply raised bindings around the value that now stands under them.
#[requires(true)]
#[ensures(true)]
fn rewrap_bindings(bindings: Vec<RaisedBinding>, body: Performable) -> Option<Performable> {
    bindings.into_iter().rev().try_fold(body, |body, binding| {
        Bind::new(
            binding.variable,
            binding.declared_type,
            binding.computation,
            body,
        )
        .ok()
        .map(Performable::Bind)
    })
}

/// Whether a rendered definition value is just a use of the name it declared.
///
/// A dynamic computation the renderer hosts returns a use of its own binder and
/// pushes the `Bind` that introduces it, so the identity is already declared;
/// declaring it a second time with a `Let` would make its name stop being an
/// identity.
#[requires(true)]
#[ensures(true)]
fn value_is_own_binder_use(id: SemanticObjectId, value: Option<&Operand>) -> bool {
    let Some(Operand::Value(value)) = value else {
        return false;
    };
    *value == Value::bound(object_variable(id), value.value_type())
}

/// Cross one performable onto an ordinary `Discourse` operand.
///
/// Section 7.1: away from the implicit performance spine an `Act` uses
/// `Perform`, a `TranscriptEntry` uses `PerformUtterance`, and an existing
/// `Discourse` remains as written.
#[requires(true)]
#[ensures(true)]
fn performed_discourse(item: Performable) -> Option<Discourse> {
    match item {
        Performable::Act(act) => Some(Discourse::perform(act)),
        Performable::Entry(entry) => Some(Discourse::perform_utterance(entry)),
        Performable::Discourse(discourse) => Some(discourse),
        Performable::Let(_) | Performable::Bind(_) | Performable::LetRec(_) => None,
    }
}

/// Result of one compact elaboration pass.
#[invariant(
    failures.windows(2).all(|pair| pair[0] < pair[1]),
    "each failed projection edge is recorded once, in stable order"
)]
#[invariant(document.is_some() != !failures.is_empty(),
    "a pass either closed one document or recorded why it could not")]
#[derive(Debug, Clone)]
pub(super) struct CompactElaboration {
    pub(super) document: Option<KernelDocument>,
    pub(super) compact_objects: usize,
    pub(super) failures: Vec<CompactFallback>,
}

impl CompactElaboration {
    /// Whether any boundary declined, which fails the projection as a whole:
    /// a failed elaboration yields failure records, never a document.
    #[requires(true)]
    #[ensures(ret == !self.failures.is_empty())]
    pub(super) fn has_failures(&self) -> bool {
        !self.failures.is_empty()
    }
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
    discharged_restrictions: BTreeMap<SemanticObjectId, SemanticObjectId>,
    needed_definitions: RefCell<BTreeSet<SemanticObjectId>>,
    placed_definitions: RefCell<BTreeSet<SemanticObjectId>>,
    /// Shared section-5.1 defaults already scheduled inside their own closure's
    /// effect scope. A second use of one of these names the binder the first
    /// use scheduled rather than asking for a declaration that is no longer
    /// coming.
    scheduled_defaults: RefCell<BTreeSet<SemanticObjectId>>,
    reference_binding_frames: RefCell<Vec<HostFrame>>,
    /// Event abstractions whose distinguished event fill is consumed by a
    /// section-11.3 crossing. This is a stack because crossing bodies may nest.
    absorbed_eventualities: RefCell<Vec<SemanticObjectId>>,
    counters: ElaborationCounters,
}

/// Recognize the projections whose binders the renderer owns.
///
/// This is the pre-plan phase. It reads only edge multiplicities and binder
/// ownership, both of which the graph fixes before any host is chosen, so
/// planning can consume the result instead of calling back into elaboration to
/// retract placement failures it had already made.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.support.iter().all(|id| graph.objects.contains_key(id)))]
#[ensures(ret.values.iter().all(|id| graph.objects.contains_key(id)))]
pub(super) fn prescan_projections(
    graph: &SemanticGraph,
    usage: &GraphUsage,
) -> ProjectedIdentities {
    let (mut support, mut values) = projected_description_objects(graph, usage);
    let (event_support, described_events) = projected_described_event_objects(graph, usage);
    support.extend(event_support);
    values.extend(described_events);
    let discharged_restrictions = discharged_quantifier_restrictions(graph);
    support.extend(discharged_restrictions.keys().copied());
    let atoms = graph
        .objects
        .iter()
        .filter(|(_, object)| {
            is_conventional_atom(object)
                || object
                    .as_referent()
                    .is_some_and(|node| exact_deictic(node, graph).is_some())
        })
        .map(|(id, _)| *id)
        .collect();
    ProjectedIdentities {
        support,
        values,
        atoms,
        discharged_restrictions,
    }
}

/// Quantifier restrictions the bound variable's argument sites merely repeat.
///
/// The builder records `ro da poi broda` twice: once as the quantifier's own
/// `restriction`, and once as a relative clause on every argument filled by the
/// bound variable. Section 8.4 puts it in exactly one place — "under the
/// generalized-quantifier restriction analysis, a restrictive clause attached
/// at that locus conjoins with the quantifier's restriction" — so the argument
/// occurrences are already discharged by the binder above them: they neither
/// print again nor make the restriction a shared declaration.
///
/// This is identity, not similarity. The argument's clause body must be the
/// very object the quantifier restricts, and the argument must be filled by the
/// very variable that quantifier binds. Any other occurrence of the restriction
/// disqualifies it, so a formula genuinely used elsewhere keeps the ordinary
/// placement it has today.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.keys().all(|restriction| graph.objects.contains_key(restriction)))]
fn discharged_quantifier_restrictions(
    graph: &SemanticGraph,
) -> BTreeMap<SemanticObjectId, SemanticObjectId> {
    let mut quantified: BTreeMap<SemanticObjectId, SemanticObjectId> = BTreeMap::new();
    let mut ambiguous = BTreeSet::new();
    for object in graph.objects.values() {
        let Some(data!(FormulaNode::Quantified(node))) =
            object.as_formula().map(FormulaNode::as_data)
        else {
            continue;
        };
        let Some(restriction) = node.restriction else {
            continue;
        };
        // Two quantifiers restricting one formula object give its repeats no
        // single binder to be discharged against.
        if quantified.insert(restriction, node.variable).is_some() {
            ambiguous.insert(restriction);
        }
    }
    let mut discharged: BTreeMap<SemanticObjectId, usize> = BTreeMap::new();
    for object in graph.objects.values() {
        let Some(arguments) = object.predication_arguments() else {
            continue;
        };
        for argument in arguments.values() {
            for clause in &argument.relative_clauses {
                let Some(variable) = quantified.get(&clause.body).copied() else {
                    continue;
                };
                if argument.value == Some(variable)
                    && clause.kind == RelativeClauseKind::Restrictive
                    && clause.veridical != Some(false)
                {
                    *discharged.entry(clause.body).or_default() += 1;
                } else {
                    ambiguous.insert(clause.body);
                }
            }
        }
    }
    let mut occurrences: BTreeMap<SemanticObjectId, usize> = BTreeMap::new();
    for occurrence in &graph.scope.uses {
        if occurrence.role == ScopeUseRole::Value {
            *occurrences.entry(occurrence.target).or_default() += 1;
        }
    }
    quantified
        .into_iter()
        .filter(|(restriction, _)| {
            let repeats = discharged.get(restriction).copied().unwrap_or(0);
            // One occurrence is the quantifier's own restriction edge. Every
            // other one has to be a discharged repeat, or the restriction is
            // shared with something this rule says nothing about.
            !ambiguous.contains(restriction)
                && repeats > 0
                && occurrences.get(restriction).copied().unwrap_or(0) == repeats + 1
        })
        .collect()
}

/// Elaborate the compact document body, including deterministic shared-value
/// declarations when needed.
#[requires(graph.objects.contains_key(&graph.root))]
#[requires(plan.compact_is_eligible())]
#[ensures(true)]
pub(super) fn elaborate_compact(
    graph: &SemanticGraph,
    plan: &ReferencePlan,
    projected: &ProjectedIdentities,
) -> CompactElaboration {
    let projected_use_counts = reference_counts_excluding_sources(graph, &projected.support);
    let definitions = graph
        .objects
        .iter()
        .filter_map(|(id, _)| {
            if plan.binder_owner(*id).is_some()
                || projected.atoms.contains(id)
                || projected.support.contains(id)
            {
                return None;
            }
            let needs_definition = if projected.values.contains(id) {
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
        projected_descriptions: projected.values.clone(),
        binder_universes: semantic_scope_dependence_binder_universes(graph.root, &graph.objects),
        discharged_restrictions: projected.discharged_restrictions.clone(),
        needed_definitions: RefCell::new(BTreeSet::new()),
        placed_definitions: RefCell::new(BTreeSet::new()),
        scheduled_defaults: RefCell::new(BTreeSet::new()),
        reference_binding_frames: RefCell::new(Vec::new()),
        absorbed_eventualities: RefCell::new(Vec::new()),
        counters: ElaborationCounters::default(),
    };
    let document = elaborator
        .render_with_definitions()
        .and_then(|body| match KernelDocument::new(body) {
            Ok(document) => Some(document),
            Err(_) => {
                // Every local rule already held, so a whole-document rejection
                // is this renderer's own invariant failing — an unbound name or
                // a shadowed one — rather than anything the graph did.
                elaborator.record_object_fallback(
                    graph.root,
                    CompactFallbackCause::TypedConstructionRejected,
                );
                None
            }
        });
    // A route that declines silently because its reason is already recorded must
    // never leave the channel empty; section 16.1 gives a failed projection at
    // least one record, and an unexplained failure is itself the implementation
    // invariant this reason names.
    if document.is_none() && elaborator.counters.failures.len() == 0 {
        elaborator
            .record_object_fallback(graph.root, CompactFallbackCause::TypedConstructionRejected);
    }
    new!(CompactElaboration {
        document: document,
        compact_objects: elaborator.counters.compact_objects.get(),
        // `BTreeSet` iteration is the stable order required of the per-edge
        // diagnostic channel: one record per failed edge, ordered by owner
        // identity and then by declining boundary.
        failures: elaborator.counters.failures.into_ordered(),
    })
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
    fn render_with_definitions(&self) -> Option<Performable> {
        let mut active = BTreeSet::new();
        let bound = Bound::new();
        let body = self
            .render_id(
                self.graph.root,
                &bound,
                &mut active,
                AcceptedModes::Unlicensed,
            )
            .and_then(Elaborated::into_performable)
            .map(Elaborated::Performable);
        let remaining = self
            .needed_definitions
            .borrow()
            .difference(&self.placed_definitions.borrow())
            .copied()
            .collect::<BTreeSet<_>>();
        self.wrap_definitions(remaining, body, &bound, &mut active)?
            .into_performable()
    }

    /// Place a deterministic definition group around one represented scope.
    #[requires(definitions.is_subset(&self.definitions))]
    #[ensures(true)]
    fn wrap_definitions(
        &self,
        definitions: BTreeSet<SemanticObjectId>,
        body: Option<Elaborated>,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Elaborated> {
        let definitions = self.definition_dependency_closure(definitions);
        if definitions.is_empty() {
            return body;
        }
        self.placed_definitions
            .borrow_mut()
            .extend(definitions.iter().copied());
        let blocks = self.definition_blocks(&definitions);
        // An identity the value renderer hosts with its own `Bind` is already
        // declared by that binder: section 6.3's dynamic host *is* its
        // declaration site, and a `Let` beside it would introduce the same name
        // twice. Such a rendering returns exactly a use of the name it bound,
        // which is what identifies it here.
        let rendered = blocks
            .iter()
            .map(|block| {
                block
                    .members
                    .iter()
                    .copied()
                    .map(|id| {
                        let value = self
                            .render_object(id, bound, active, AcceptedModes::Unlicensed)
                            .and_then(Elaborated::into_operand);
                        (id, value)
                    })
                    .filter(|(id, value)| !value_is_own_binder_use(*id, value.as_ref()))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        // Only a declaration that survives to become a `Let` needs a type the
        // notation can spell. Placement and declaration are separate decisions:
        // the plan says *where* a shared identity is rendered, and a reference
        // computation rendered there declares itself, so demanding a
        // `Let`-bindable type of it would refuse the whole document over a
        // binding that is never written. Declining is still a per-object
        // failure, so the loop keeps the exact identity that could not be typed
        // rather than reporting the whole declaration group.
        let mut typed_blocks = Vec::with_capacity(rendered.len());
        for bindings in rendered {
            let mut typed = Vec::with_capacity(bindings.len());
            for (id, value) in bindings {
                let Some(declared_type) = definition_type_expr(&self.graph.objects[&id]) else {
                    self.record_object_fallback(
                        id,
                        CompactFallbackCause::DefinitionTypeUnrepresentable,
                    );
                    return None;
                };
                typed.push((id, declared_type, value));
            }
            typed_blocks.push(typed);
        }
        for (block, bindings) in blocks.iter().zip(&typed_blocks) {
            // Only a genuinely cyclic block is a `LetRec`, so only a cyclic
            // block's initializers have to be inert lambdas. An acyclic
            // neighbour that happens to share this host is an ordinary
            // declaration and is no longer dragged into the group's shape.
            if block.recursive
                && let Some((id, _, _)) = bindings.iter().find(|(_, _, value)| {
                    !matches!(value, Some(Operand::Function(callable)) if callable.is_lambda())
                })
            {
                self.record_object_fallback(
                    *id,
                    CompactFallbackCause::UnrepresentableRecursiveValue,
                );
                return None;
            }
        }
        // The first block is the outermost binder, so the blocks are applied
        // from the innermost outwards over the body they all wrap.
        let mut wrapped = body?;
        for (block, bindings) in blocks.into_iter().zip(typed_blocks).rev() {
            if bindings.is_empty() {
                continue;
            }
            wrapped = self.bind_definitions(bindings, block.recursive, wrapped)?;
        }
        Some(wrapped)
    }

    /// Bind one placed declaration group around the value it hosts.
    ///
    /// The kernel keeps a host's declarations together because that is the
    /// placement decision; nesting one `Let` per declaration is the printer's
    /// business. A category with no binding form of its own has no place to put
    /// the group, which is this renderer's invariant rather than the graph's.
    #[requires(true)]
    #[ensures(true)]
    fn bind_definitions(
        &self,
        bindings: Vec<(SemanticObjectId, TypeExpr, Option<Operand>)>,
        recursive: bool,
        body: Elaborated,
    ) -> Option<Elaborated> {
        let mut declarations = Vec::with_capacity(bindings.len());
        let mut recursive_declarations = Vec::with_capacity(bindings.len());
        for (id, declared_type, value) in bindings {
            let variable = object_variable(id);
            let value = value?;
            if recursive {
                let Operand::Function(initializer) = value else {
                    return None;
                };
                recursive_declarations
                    .push(RecursiveDeclaration::new(variable, declared_type, initializer).ok()?);
            } else {
                declarations.push(Declaration::new(variable, declared_type, value).ok()?);
            }
        }
        let hosted = if recursive {
            HostedGroup::Recursive(recursive_declarations)
        } else {
            HostedGroup::Inert(declarations)
        };
        hosted.wrap(body)
    }

    /// Open one section-6.3 host position.
    ///
    /// A barrier frame is a position no computation may be raised out of; a
    /// binder frame is one only a computation depending on one of its binders
    /// has to stay inside.
    #[requires(true)]
    #[ensures(self.reference_binding_frames.borrow().len() == old(self.reference_binding_frames.borrow().len()) + 1)]
    fn open_host_frame(&self, kind: HostFrameKind, live: &Bound) {
        self.reference_binding_frames.borrow_mut().push(HostFrame {
            kind,
            owner: None,
            live: live.keys().copied().collect(),
            bindings: Vec::new(),
        });
    }

    /// Open the effect scope of one `Close`.
    ///
    /// Section 5.1: the computations this closure omits run at the dynamic
    /// evaluation site of *this* `Close`, in current numbered-place order, and
    /// stay local to it unless graph identity has shared them. So a closure is
    /// its own host position — one per conjunct of a `∧`, one per operand of a
    /// `Joi`, one per closure in a question body — rather than a contributor to
    /// whatever lambda or force segment happens to enclose it.
    #[requires(true)]
    #[ensures(self.reference_binding_frames.borrow().len() == old(self.reference_binding_frames.borrow().len()) + 1)]
    fn open_closure_scope(&self, owner: Option<ClosureSite>, live: &Bound) {
        self.reference_binding_frames.borrow_mut().push(HostFrame {
            kind: HostFrameKind::Closure,
            owner,
            live: live.keys().copied().collect(),
            bindings: Vec::new(),
        });
    }

    /// Close one effect scope over the content it was opened for.
    ///
    /// The scope is always closed, whether or not the closure rendered: a
    /// declining route still has to leave the frame stack as it found it.
    #[requires(!self.reference_binding_frames.borrow().is_empty())]
    #[ensures(self.reference_binding_frames.borrow().len() == old(self.reference_binding_frames.borrow().len()) - 1)]
    fn close_closure_scope(&self, closed: Option<Content>) -> Option<Content> {
        let hosted = self.close_host_frame();
        wrap_reference_bindings(hosted, closed?, Content::bind_form)
    }

    /// Close the innermost host position and take what it hosts.
    #[requires(!self.reference_binding_frames.borrow().is_empty())]
    #[ensures(self.reference_binding_frames.borrow().len() == old(self.reference_binding_frames.borrow().len()) - 1)]
    fn close_host_frame(&self) -> Vec<ReferenceBinding> {
        self.reference_binding_frames
            .borrow_mut()
            .pop()
            .expect("a closed host frame was opened")
            .bindings
    }

    /// Place one reference computation at its section-6.3 host.
    ///
    /// The host is the innermost open frame the computation may not be raised
    /// out of: a barrier, or a frame introducing one of the binders the
    /// computation's property depends on. With no such frame the computation
    /// raises to the outermost open position, which is the enclosing force
    /// segment. Reporting `false` means there is no open position at all, and
    /// the caller declines rather than inventing one.
    #[requires(true)]
    #[ensures(true)]
    fn host_reference(
        &self,
        binding: ReferenceBinding,
        dependencies: &BTreeSet<SemanticObjectId>,
    ) -> bool {
        let mut frames = self.reference_binding_frames.borrow_mut();
        if frames.is_empty() {
            return false;
        }
        let host = frames
            .iter()
            .rposition(|frame| frame.kind.stops(dependencies))
            .unwrap_or(0);
        if !frames[host].admits(dependencies) {
            return false;
        }
        frames[host].bind(binding);
        true
    }

    /// Place one computation that does not raise at all.
    ///
    /// Section 6.3: `Context` stays at its represented evaluation site, so it
    /// takes the innermost open host rather than ascending. That position is
    /// still only legal when every binder the context depends on is live there.
    #[requires(true)]
    #[ensures(true)]
    fn host_locally(
        &self,
        binding: ReferenceBinding,
        dependencies: &BTreeSet<SemanticObjectId>,
    ) -> bool {
        let mut frames = self.reference_binding_frames.borrow_mut();
        let Some(frame) = frames.last_mut() else {
            return false;
        };
        if !frame.admits(dependencies) {
            return false;
        }
        frame.bind(binding);
        true
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

    /// Condense one host's group into the exact blocks the kernel admits.
    ///
    /// Specification section 3.4 gives `LetRec` one job: tie a group of inert
    /// lambdas that see each other. So the recursive shape belongs to a
    /// genuinely cyclic component of the group's dependency graph and to
    /// nothing else. Promoting the whole residual group when topological
    /// emission stalled — which is what this used to do — either rejected legal
    /// acyclic neighbours for not being lambdas or tied names that never see
    /// each other.
    ///
    /// Blocks are emitted in dependency order; ties, and the order inside one
    /// block, fall to the plan's declaration order, which is source order
    /// rather than the allocation order a set of identities iterates in.
    /// Consecutive acyclic components merge into one `Let`, because section
    /// 2.2's declarations are sequential and a later initializer may already
    /// use an earlier name — so a group with no cycle in it emits exactly the
    /// one block it emitted before.
    #[requires(definitions.is_subset(&self.definitions))]
    #[ensures(ret.iter().map(|block| block.members.len()).sum::<usize>() == definitions.len())]
    fn definition_blocks(&self, definitions: &BTreeSet<SemanticObjectId>) -> Vec<DeclarationBlock> {
        let adjacency = definitions
            .iter()
            .copied()
            .map(|id| {
                let dependencies = self
                    .definition_dependencies(id)
                    .into_iter()
                    .filter(|dependency| definitions.contains(dependency))
                    .collect::<BTreeSet<_>>();
                (id, dependencies)
            })
            .collect::<BTreeMap<_, _>>();
        let key = |id: &SemanticObjectId| self.plan.declaration_order(*id);
        let mut components = strong_components(&adjacency)
            .into_iter()
            .map(|component| {
                let mut members = component.into_iter().collect::<Vec<_>>();
                members.sort_by_key(key);
                members
            })
            .collect::<Vec<_>>();
        // A component's own least declaration order is what orders it against
        // its independent siblings, which reproduces the previous total order
        // exactly whenever every component is a single identity.
        let mut emitted = BTreeSet::new();
        let mut blocks: Vec<(Vec<SemanticObjectId>, bool)> = Vec::new();
        while !components.is_empty() {
            let next = components
                .iter()
                .enumerate()
                .filter(|(_, members)| {
                    members.iter().all(|id| {
                        adjacency[id].iter().all(|dependency| {
                            members.contains(dependency) || emitted.contains(dependency)
                        })
                    })
                })
                .min_by_key(|(_, members)| members.iter().map(key).min().expect("nonempty"))
                .map(|(index, _)| index)
                // A component whose dependencies are all inside the group and
                // already emitted always exists in a condensation, so this is
                // only a total-function guard, not a fixed point.
                .unwrap_or(0);
            let members = components.remove(next);
            emitted.extend(members.iter().copied());
            let recursive =
                members.len() > 1 || members.first().is_some_and(|id| adjacency[id].contains(id));
            match blocks.last_mut() {
                Some((last, false)) if !recursive => last.extend(members),
                _ => blocks.push((members, recursive)),
            }
        }
        blocks
            .into_iter()
            .map(|(members, recursive)| new!(DeclarationBlock { members, recursive }))
            .collect()
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
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
        modes: AcceptedModes,
    ) -> Option<Elaborated> {
        if let Some(declaration) = bound.get(&id) {
            return Some(Elaborated::Operand(bound_use(id, declaration)));
        }
        if self.definitions.contains(&id) {
            if let Some(scheduled) = self.schedule_shared_default(id, bound, active) {
                return Some(scheduled);
            }
            self.needed_definitions.borrow_mut().insert(id);
            // The declaration this use resolves to is placed by
            // `wrap_definitions`, which is also where an identity whose type the
            // notation cannot spell is refused; a use spells the same type.
            let declared_type = definition_use_type(&self.graph.objects[&id])?;
            return Some(Elaborated::Operand(bound_use(
                id,
                &BoundValue::Value(declared_type),
            )));
        }
        self.render_object(id, bound, active, modes)
    }

    /// Schedule one graph-shared section-5.1 default inside the closure that
    /// owns it, at the place that uses it first.
    ///
    /// Section 5.1's rule 2 keeps a shared default shared — one explicit `Bind`
    /// for one graph identity — but it does not exempt that `Bind` from rule
    /// 1's schedule: the omitted computations still "run left to right in
    /// current numbered-place order at the dynamic evaluation site of `Close`".
    /// Reaching the shared computation only through the deferred declaration
    /// pass breaks both halves of that: the declaration is emitted after the
    /// whole closure has been built, so an unshared default discovered at a
    /// *later* place is already bound inside it, and the closure's own scope
    /// has been left, so the `Bind` lands at a coarser lambda or force
    /// position. Rendering it here instead binds it at the first place that
    /// asks for it, in the scope that place is being rendered in.
    ///
    /// Only a default whose every use is a place of one single closure is
    /// eligible, which is what keeps the shared name in scope at every use. A
    /// default shared across two closures of one object — the main application
    /// and an adjunct, or two adjuncts — has no such closure, and section 5.1's
    /// rule 2 puts its one `Bind` at the common dynamic site above both, which
    /// is the declaration placement the plan already computed.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn schedule_shared_default(
        &self,
        id: SemanticObjectId,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Elaborated> {
        if !self.scheduled_defaults.borrow().contains(&id) {
            if !self.is_shared_closure_default(id) {
                return None;
            }
            let closure = self.shared_default_closure(id)?;
            // The scope that owns the schedule and the scope the `Bind` lands
            // in have to be the same one. Rendering hosts a contextual constant
            // at the *innermost* open scope, so an outward search that skipped
            // an intervening closure would schedule against one effect scope
            // and bind inside another, leaving every later use of the name
            // outside its binder.
            let innermost = self
                .reference_binding_frames
                .borrow()
                .last()
                .and_then(|frame| frame.owner);
            if innermost != Some(closure) {
                return None;
            }
            // The rendering itself is what appends the `Bind`: a contextual
            // constant hosts its own computation at the innermost open scope,
            // which is the closure this use stands in.
            let value = self.render_object(id, bound, active, AcceptedModes::Unlicensed)?;
            self.scheduled_defaults.borrow_mut().insert(id);
            self.placed_definitions.borrow_mut().insert(id);
            return Some(value);
        }
        let declared_type = definition_use_type(&self.graph.objects[&id])?;
        Some(Elaborated::Operand(bound_use(
            id,
            &BoundValue::Value(declared_type),
        )))
    }

    /// The one `Close` whose own numbered places share this default, when the
    /// graph names one.
    ///
    /// The plan places a shared identity at the object that contains every use
    /// of it, and that object may render as several closures. So the closure is
    /// read off the object's own place maps: the application's places are one
    /// `Close`, each adjunct's are another. One closure holding every use is a
    /// closure that can schedule the default itself; two are the cross-closure
    /// case, where a name bound inside the first would not be in scope in the
    /// second and the common site above both is the only legal host.
    ///
    /// Deliberately narrow in the other direction too: every use has to be a
    /// place of the site object itself. A use reached *through* some further
    /// object stands in whatever closure renders that object, which this
    /// coordinate cannot name, so such a default keeps its planned declaration.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn shared_default_closure(&self, id: SemanticObjectId) -> Option<ClosureSite> {
        let site = self.plan.definition_site(id)?;
        if self
            .plan
            .uses_of(id)
            .is_none_or(|uses| uses.len() != 1 || !uses.contains(&site))
        {
            return None;
        }
        let node = self.graph.objects[&site].as_predication()?;
        let mut closures = Vec::new();
        let main = fills_naming(&node.arguments, id) + usize::from(node.eventuality == Some(id));
        if main > 0 {
            closures.push((ClosureSite::Main(site), main));
        }
        for (index, adjunct) in node.adjuncts.iter().enumerate() {
            let places = fills_naming(&adjunct.arguments, id);
            if places > 0 {
                closures.push((ClosureSite::Adjunct(site, index), places));
            }
        }
        // Exactly one closure, and it accounts for every use the plan counted:
        // an occurrence this walk did not see is one whose closure it cannot
        // name, and guessing at it is what would bind a name out of scope.
        let [(closure, places)] = closures[..] else {
            return None;
        };
        (places == self.plan.use_count(id)).then_some(closure)
    }

    /// Whether one shared definition is a section-5.1 contextual default rather
    /// than an ordinary shared value.
    ///
    /// Only the default has a computation the closure schedules; every other
    /// shared identity is a value whose placement the declaration pass owns.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn is_shared_closure_default(&self, id: SemanticObjectId) -> bool {
        if self.projected_descriptions.contains(&id) {
            return false;
        }
        self.graph.objects[&id].as_referent().is_some_and(|node| {
            default_elided_shape(node)
                && node.scope_dependence.is_some()
                && self.graph.objects[&id].diagnostics().is_empty()
        })
    }

    /// Dispatch one graph object through exact typed recognizers.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_object(
        &self,
        id: SemanticObjectId,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
        modes: AcceptedModes,
    ) -> Option<Elaborated> {
        if !active.insert(id) {
            // Re-entering an identity means the graph has a cycle that no
            // declaration covers, so there is no binder to spell this use with.
            return None;
        }
        let object = &self.graph.objects[&id];
        let value = match object.as_data() {
            data!(SemanticObject::Utterance(node)) => self
                .render_utterance(id, node, bound, active)
                .map(Elaborated::Performable),
            data!(SemanticObject::Sequence(node)) => self
                .render_sequence(id, node, bound, active)
                .map(Elaborated::Performable),
            data!(SemanticObject::Predication(node)) => self
                .render_predication(id, node, bound, active, modes)
                .map(operand_content),
            data!(SemanticObject::Formula(node)) => self
                .render_formula(id, node, bound, active, modes)
                .map(operand_content),
            data!(SemanticObject::Referent(node)) => self
                .render_referent(id, node, bound, active)
                .map(Elaborated::Operand),
            data!(SemanticObject::Eventuality(node)) => self
                .render_eventuality(id, node, bound, active)
                .map(Elaborated::Operand),
            data!(SemanticObject::Question(node)) => self
                .render_question(id, node, bound, active)
                .map(|act| Elaborated::Operand(Operand::Act(act))),
            data!(SemanticObject::Quantity(node)) => self
                .render_quantity(id, node, bound, active)
                .map(operand_value),
            data!(SemanticObject::MathExpression(node)) => {
                self.render_math(id, node, bound, active).map(operand_value)
            }
            data!(SemanticObject::Sign(node)) => {
                self.render_sign(id, node, bound, active).map(operand_value)
            }
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
            .collect::<BTreeSet<_>>();
        self.place_definitions(id, definitions, value, bound, active)
    }

    /// Place the declarations the plan sited at one object, at that object's own
    /// position.
    ///
    /// The plan's placement coordinate is a graph object, and the position that
    /// object's rendered value occupies is a physical one this renderer owns:
    /// the wrapper it is about to build around that value. So the site is a host
    /// position, and a planned declaration whose value hosts its own computation
    /// — section 5.1's shared default, which is *bound* from `(Context ...)`
    /// rather than declared with a `Let` — belongs at it. Section 6.3 keeps that
    /// computation at its represented evaluation site, and for an identity no
    /// single closure closes, the site the graph determines is exactly the one
    /// the plan computed: the common position of every use. Without this frame
    /// the `Bind` would be appended to whichever force segment or lambda region
    /// is still open above the site, running the lookup ahead of content the
    /// site stands after and wrapping unrelated siblings in its binder.
    ///
    /// Only where the renderer already owns a host region: outside every force
    /// segment there is no host position at all — which is a placement failure
    /// of whatever could not place the enclosing value, not section 6.3's
    /// legality question — and opening one here would invent it.
    ///
    /// A site whose own category has no binding form is refused rather than
    /// hosted elsewhere. Sections 2.2 and 3.1 give a query, act, discourse or
    /// transcript entry no `Bind` of its own, so at such a site the plan's
    /// position is one nothing can be written at, and the rules and the graph
    /// together determine no host — the same closing clause of section 6.3 that
    /// an unlive dependency reaches, with a message of its own.
    #[requires(self.graph.objects.contains_key(&id))]
    #[requires(definitions.is_subset(&self.definitions))]
    #[ensures(true)]
    fn place_definitions(
        &self,
        id: SemanticObjectId,
        definitions: BTreeSet<SemanticObjectId>,
        value: Option<Elaborated>,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Elaborated> {
        if definitions.is_empty() || self.reference_binding_frames.borrow().is_empty() {
            return self.wrap_definitions(definitions, value, bound, active);
        }
        self.open_host_frame(HostFrameKind::Placement, bound);
        let wrapped = self.wrap_definitions(definitions, value, bound, active);
        let hosted = self.close_host_frame();
        let wrapped = wrapped?;
        if hosted.is_empty() {
            return Some(wrapped);
        }
        if !wrapped.hosts_reference_bindings() {
            self.record_object_fallback(id, CompactFallbackCause::PlacedHostWithoutBindingForm);
            return None;
        }
        let Some(bound_at_site) = bind_at_site(hosted, wrapped) else {
            // The category was proved to have a binding form, so a rejection
            // here is this renderer failing to construct one. Every boundary
            // below this object was already reached by the render that
            // succeeded, so only the new edge is recorded.
            self.record_object_fallback(id, CompactFallbackCause::TypedConstructionRejected);
            return None;
        };
        Some(bound_at_site)
    }

    /// Decline one object at a named boundary, having reached every boundary
    /// below it first.
    ///
    /// Rendering the children of a declined object is not wasted work: one
    /// document reports every failed projection edge it has, so a child that
    /// declines for its own reason must still be logged even though its parent
    /// has already declined.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(ret.is_none())]
    fn fallback_object<T>(
        &self,
        id: SemanticObjectId,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
        cause: CompactFallbackCause,
    ) -> Option<T> {
        self.record_object_fallback(id, cause);
        self.record_declined_children(id, bound, active);
        None
    }

    /// Record one failed projection edge, which fails the projection as a
    /// whole once elaboration completes.
    ///
    /// The edge is `(id, cause)`. Re-entering the same boundary from a
    /// declining wrapper's re-render is the same edge and adds nothing; a
    /// second, different boundary on the same object is a second failed edge
    /// and is retained.
    #[requires(true)]
    #[ensures(self.counters.failures.contains(CompactFallback { owner: id, cause }))]
    fn record_object_fallback(&self, id: SemanticObjectId, cause: CompactFallbackCause) {
        self.counters
            .failures
            .record(CompactFallback { owner: id, cause });
    }

    /// Count one compact object recognition, when the route actually produced
    /// a typed value for it.
    #[requires(true)]
    #[ensures(self.counters.compact_objects.get()
        == old(self.counters.compact_objects.get()) + usize::from(ret.is_some()))]
    fn recognized<T>(&self, value: Option<T>) -> Option<T> {
        if value.is_some() {
            self.counters
                .compact_objects
                .set(self.counters.compact_objects.get() + 1);
        }
        value
    }

    /// Reach every boundary below one declined object.
    ///
    /// A lexical binder resolves without recursion, and one identity reached
    /// twice reaches the same boundaries twice, so both are visited once. The
    /// rendered values are discarded: what this pass is for is the failed edges
    /// they record.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn record_declined_children(
        &self,
        id: SemanticObjectId,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) {
        let mut visited = BTreeSet::new();
        for reference in self.object_references(id) {
            if bound.contains_key(&reference) || !visited.insert(reference) {
                continue;
            }
            assert!(
                self.graph.objects.contains_key(&reference),
                "validated semantic graphs close every object reference"
            );
            let _ = self.render_id(reference, bound, active, AcceptedModes::Unlicensed);
        }
    }

    /// Render an utterance act, omitting the record only under the named default.
    #[requires(true)]
    #[ensures(true)]
    fn render_utterance(
        &self,
        id: SemanticObjectId,
        node: &UtteranceNode,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Performable> {
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
        // Spec section 12 types the interrogative act as
        // `Ask : Query<A> -> Act<Question>`. The question renderer is the only
        // site that can build that operand, so ask force over content that is
        // not a typed question has no well-typed compact act at all. Decline
        // before rendering rather than applying `Ask` at the wrong type.
        if node.force == UtteranceForce::Ask && self.graph.objects[&content].as_question().is_none()
        {
            return self.fallback_object(
                id,
                bound,
                active,
                CompactFallbackCause::AskForceWithoutQuestion,
            );
        }
        // An asserted utterance is the force boundary its content is asserted
        // at; every other force this route reaches carries its own act
        // constructor and leaves no predication boundary standing here.
        let modes = if node.force == UtteranceForce::Assert {
            AcceptedModes::Force
        } else {
            AcceptedModes::Unlicensed
        };
        let content_id = content;
        // The performed act is section 6.3's ceiling: no reference computation
        // raises out of its own force segment.
        self.open_host_frame(HostFrameKind::Barrier, bound);
        let content = self.render_id(content_id, bound, active, modes);
        let bindings = self.close_host_frame();
        let act = match node.force {
            UtteranceForce::Assert => content.and_then(Elaborated::into_content).map(Act::assert),
            // The question renderer already produced the complete `Ask` act,
            // and the guard above proved the content is a typed question.
            UtteranceForce::Ask => content.and_then(|value| match value {
                Elaborated::Operand(Operand::Act(act)) => Some(act),
                _ => None,
            }),
            UtteranceForce::Mention => content.and_then(Elaborated::into_operand).map(Act::mention),
            UtteranceForce::Quote
            | UtteranceForce::Parenthetical
            | UtteranceForce::Subordinated
            | UtteranceForce::Command
            | UtteranceForce::Vocative => unreachable!(
                "unsupported forces return typed fallback before rendering their content"
            ),
        };
        if utterance_record_is_default(self.graph, self.plan.usage(), id, node) {
            let act = act?;
            return self.recognized(wrap_reference_bindings(
                bindings,
                Performable::Act(act),
                Performable::Bind,
            ));
        }
        if !node.asides.is_empty() || node.vocative_kind.is_some() {
            return self.fallback_object(
                id,
                bound,
                active,
                CompactFallbackCause::ForceFieldsRequireRecord,
            );
        }

        let token = object_variable(id);
        let token_use = Operand::Value(Value::bound(
            token.clone(),
            TypeExpr::Atom(TypeAtom::UtteranceToken),
        ));
        let mut record_bound = bound.clone();
        record_bound.insert(
            id,
            BoundValue::Value(TypeExpr::Atom(TypeAtom::UtteranceToken)),
        );
        let mut facts = vec![
            self.utterance_fact(
                Intrinsic::SpeakerOf,
                &token_use,
                node.speaker,
                &record_bound,
                active,
            ),
            self.utterance_fact(
                Intrinsic::AudienceOf,
                &token_use,
                node.audience,
                &record_bound,
                active,
            ),
        ];
        if !default_locution_event(self.graph, node.eventuality)
            || self.plan.use_count(node.eventuality) > 1
        {
            facts.push(self.utterance_fact(
                Intrinsic::LocutionOf,
                &token_use,
                node.eventuality,
                &record_bound,
                active,
            ));
        }
        if !object_is_indexical(self.graph, node.deictic_ground.time, IndexicalKind::Now) {
            facts.push(self.utterance_fact(
                Intrinsic::DeicticTimeOf,
                &token_use,
                node.deictic_ground.time,
                &record_bound,
                active,
            ));
        }
        if !object_is_indexical(self.graph, node.deictic_ground.place, IndexicalKind::Here) {
            facts.push(self.utterance_fact(
                Intrinsic::DeicticPlaceOf,
                &token_use,
                node.deictic_ground.place,
                &record_bound,
                active,
            ));
        }
        facts.push(
            Content::intrinsic(Intrinsic::Realizes, vec![token_use, Operand::Act(act?)]).ok(),
        );
        let facts = facts.into_iter().collect::<Option<Vec<_>>>()?;
        let entry = TranscriptEntry::utterance(token, facts).ok()?;
        // A reference computation hosted inside this utterance stands outside
        // the record, not inside `Realizes`: section 6.3 raises a computation to
        // the outermost legal point, and the entry's own analyzer facts are one
        // administrative shell it may pass through.
        self.recognized(wrap_reference_bindings(
            bindings,
            Performable::Entry(entry),
            Performable::Bind,
        ))
    }

    /// Build one analyzer fact relating the utterance token to a rendered value.
    #[requires(self.graph.objects.contains_key(&value))]
    #[ensures(true)]
    fn utterance_fact(
        &self,
        intrinsic: Intrinsic,
        token: &Operand,
        value: SemanticObjectId,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Content> {
        let value = self
            .render_id(value, bound, active, AcceptedModes::Unlicensed)
            .and_then(Elaborated::into_operand)?;
        Content::intrinsic(intrinsic, vec![token.clone(), value]).ok()
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
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Performable> {
        let ordinary_fields = node.force.is_none()
            && node.content.is_none()
            && node.connection_claims.is_empty()
            && node.bound_eventualities.is_empty()
            && node.ordinal_labels.is_empty()
            && node.nonlogical_connection.is_none()
            && node.elided_connection_operand.is_none();
        if ordinary_fields {
            match &node.relation {
                // Section 7.1 types `Do : Performable^n -> Discourse, n >= 2`
                // and contracts a one-item sequence to that item. A sequence
                // with no items is neither: it denotes no discourse at all, so
                // there is nothing for the document body to perform.
                SequenceRelation::SameTopicContinuation if !node.items.is_empty() => {
                    let mut items = self.render_items(&node.items, bound, active)?;
                    let performable = if items.len() == 1 {
                        items
                            .pop()
                            .expect("a one-item sequence contracts to its item")
                    } else {
                        Performable::Discourse(Discourse::sequence(items).ok()?)
                    };
                    return self.recognized(Some(performable));
                }
                SequenceRelation::ParagraphBoundary {
                    transition,
                    additional,
                } if additional.is_empty() && !node.items.is_empty() => {
                    let items = self.render_items(&node.items, bound, active)?;
                    // A transition is an administrative shell, so a reference
                    // computation hosted by its single item stands outside it
                    // rather than trapped under a `Discourse` operand.
                    let (bindings, mut items) = raise_item_bindings(items);
                    let discourse = if items.len() == 1 {
                        performed_discourse(items.pop().expect("one item"))?
                    } else {
                        Discourse::sequence(items).ok()?
                    };
                    let transition = match transition {
                        ParagraphTransition::NewTopic => Discourse::new_topic(discourse),
                        ParagraphTransition::ResumePriorTopic => Discourse::resume(discourse),
                    }
                    .ok()?;
                    return self.recognized(rewrap_bindings(
                        bindings,
                        Performable::Discourse(transition),
                    ));
                }
                _ => {}
            }
        }
        self.fallback_object(id, bound, active, CompactFallbackCause::SequenceFields)
    }

    /// Render every item of a sequence, reaching each item's boundaries even
    /// after one of them has declined.
    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|items| items.len() == items_of.len()))]
    fn render_items(
        &self,
        items_of: &[SemanticObjectId],
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Vec<Performable>> {
        items_of
            .iter()
            .map(|item| {
                self.render_id(*item, bound, active, AcceptedModes::Unlicensed)
                    .and_then(Elaborated::into_performable)
            })
            .collect::<Vec<_>>()
            .into_iter()
            .collect()
    }

    /// Render the typed formula families, declining whenever connective
    /// metadata is not consumed by an exact rule.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_formula(
        &self,
        id: SemanticObjectId,
        formula: &crate::model::FormulaNode,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
        modes: AcceptedModes,
    ) -> Option<Content> {
        match formula.as_data() {
            data!(FormulaNode::Atom(node)) => {
                let generated = node
                    .bound_eventualities
                    .iter()
                    .map(|event| event.object_id())
                    .collect::<Vec<_>>();
                let scoped = self.scope_generated_events(bound, &generated);
                let body = self
                    .render_id(node.predication, &scoped, active, modes)
                    .and_then(Elaborated::into_content);
                self.recognized(self.bind_generated_events(id, &generated, body, &scoped, active))
            }
            data!(FormulaNode::Connective(node)) => {
                if let Some(value) = self.render_tanru_projection(id, node, bound, active, modes) {
                    return self.recognized(Some(value));
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
                let scoped = self.scope_generated_events(bound, &generated);
                let operands = children
                    .into_iter()
                    .map(|child| {
                        self.render_id(child, &scoped, active, modes)
                            .and_then(Elaborated::into_content)
                    })
                    .collect::<Vec<_>>();
                let value = operands
                    .into_iter()
                    .collect::<Option<Vec<_>>>()
                    .and_then(|operands| operator.apply(operands));
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
                let Some(variable_type) = sort_type_expr(variable_sort) else {
                    return self.fallback_object(
                        id,
                        bound,
                        active,
                        CompactFallbackCause::QuantifierVariableFields,
                    );
                };
                let generated = node
                    .bound_eventualities
                    .iter()
                    .map(|event| event.object_id())
                    .collect::<Vec<_>>();
                let mut scoped = self.scope_generated_events(bound, &generated);
                scoped.insert(node.variable, BoundValue::Value(variable_type.clone()));
                let binding = TypedParameter::new(object_variable(node.variable), variable_type);
                // Section 6.3's lambda-dependency boundary. The restriction and
                // the scope are two sibling properties of one binder, so each
                // opens its own host position: a computation depending on the
                // variable stays in the property it was written in, and one
                // that does not raises straight out of both.
                let quantified_binder = BTreeSet::from([node.variable]);
                self.open_host_frame(HostFrameKind::Binders(quantified_binder.clone()), &scoped);
                let restriction = node.restriction.map(|restriction| {
                    self.render_id(restriction, &scoped, active, AcceptedModes::NoForce)
                        .and_then(Elaborated::into_content)
                });
                let restriction_hosts = self.close_host_frame();
                let restriction = restriction.map(|restriction| {
                    restriction.and_then(|restriction| {
                        wrap_reference_bindings(restriction_hosts, restriction, Content::bind_form)
                    })
                });
                self.open_host_frame(HostFrameKind::Binders(quantified_binder), &scoped);
                let body = self
                    .render_id(node.body, &scoped, active, modes)
                    .and_then(Elaborated::into_content);
                let body_hosts = self.close_host_frame();
                let body = body
                    .and_then(|body| wrap_reference_bindings(body_hosts, body, Content::bind_form));
                let universal = exact_universal_quantity(self.graph, node.operator, node.quantity);
                let ordinary_exists =
                    node.operator == FormulaOperator::Exists && node.quantity.is_none();
                // A witness-set selection re-quantifies an established variable
                // and has no registered compact reduction.
                if node.source_variable.is_some()
                    || node
                        .selection_source
                        .as_ref()
                        .is_some_and(SelectionSource::is_witness_set)
                    || !(universal || ordinary_exists)
                {
                    return self.fallback_object(
                        id,
                        bound,
                        active,
                        CompactFallbackCause::QuantifierFields,
                    );
                }
                // A described domain adds no printing obligation of its own
                // only because the binding's own restriction already names it:
                // the model says a described source restricts the candidate
                // with `memberOf(candidate, description)`, and the reduction
                // renders that restriction. Nothing in the model's invariants
                // makes the two agree, though, so this is where the exact
                // renderer certifies the correspondence — and refuses when it
                // does not hold, rather than dropping the domain silently or
                // manufacturing the conjunct the graph did not write. A source
                // with no restriction at all is the same failure: the domain
                // would simply not be printed.
                if let Some(source) = node.selection_source.as_ref()
                    && !self.restriction_states_membership(
                        node.restriction,
                        node.variable,
                        source.variable,
                    )
                {
                    return self.fallback_object(
                        id,
                        bound,
                        active,
                        CompactFallbackCause::SelectionSourceUncertified,
                    );
                }
                let quantified = match restriction {
                    Some(restriction) => {
                        let constructor = if universal
                            && self.graph.objects[&id].formula_domain_import()
                                == Some(crate::model::DomainImport::Projective)
                        {
                            Intrinsic::Every
                        } else if ordinary_exists {
                            Intrinsic::Some
                        } else {
                            return self.fallback_object(
                                id,
                                bound,
                                active,
                                CompactFallbackCause::QuantifierFields,
                            );
                        };
                        match generalized_quantification(constructor, &binding, restriction, body) {
                            GeneralizedQuantification::Rendered(content) => Some(content),
                            GeneralizedQuantification::DeclinedOperand => None,
                            GeneralizedQuantification::PurityUnproven => {
                                return self.fallback_object(
                                    id,
                                    bound,
                                    active,
                                    CompactFallbackCause::PurityUnproven,
                                );
                            }
                            GeneralizedQuantification::ConstructionRejected => {
                                return self.fallback_object(
                                    id,
                                    bound,
                                    active,
                                    CompactFallbackCause::TypedConstructionRejected,
                                );
                            }
                        }
                    }
                    None => {
                        let operator = if universal {
                            QuantifierOp::ForAll
                        } else {
                            QuantifierOp::Exists
                        };
                        property_lambda(&binding, body)
                            .map(|lambda| Content::quantified(operator, lambda))
                    }
                };
                self.recognized(
                    self.bind_generated_events(id, &generated, quantified, &scoped, active),
                )
            }
            data!(FormulaNode::RespectivelyDistribution(_)) => self.fallback_object(
                id,
                bound,
                active,
                CompactFallbackCause::RespectivelySlotFields,
            ),
            data!(FormulaNode::QuantifierBundle(_)) => {
                self.fallback_object(id, bound, active, CompactFallbackCause::QuantifierFields)
            }
        }
    }

    /// Whether one quantifier's restriction states the membership its described
    /// selection source claims.
    ///
    /// The certified shape is the one the builder writes and the one section
    /// 9.2 describes: a `memberOf` conjunct over exactly this candidate and
    /// exactly this source, reached through the conjunction the restriction is
    /// built as. Every other conjunct is independently licensed content and is
    /// left alone — this proves a conjunct is *present*, never that it is the
    /// only one — and a restriction that is absent, or that names some other
    /// plurality, certifies nothing.
    #[requires(true)]
    #[ensures(true)]
    fn restriction_states_membership(
        &self,
        restriction: Option<SemanticObjectId>,
        candidate: SemanticObjectId,
        source: SemanticObjectId,
    ) -> bool {
        let Some(restriction) = restriction else {
            return false;
        };
        let mut visited = BTreeSet::new();
        let mut pending = vec![restriction];
        while let Some(formula) = pending.pop() {
            if !visited.insert(formula) || !self.graph.objects.contains_key(&formula) {
                continue;
            }
            let Some(node) = self.graph.objects[&formula].as_formula() else {
                continue;
            };
            match node.as_data() {
                data!(FormulaNode::Atom(atom)) => {
                    if self.predication_states_membership(atom.predication, candidate, source) {
                        return true;
                    }
                }
                // Only conjunction distributes: a conjunct of the restriction
                // is asserted of every candidate the domain keeps, which is
                // what makes the membership claim the domain itself. A branch
                // of a disjunction or the operand of a negation is not.
                data!(FormulaNode::Connective(connective))
                    if connective.operator == FormulaOperator::And =>
                {
                    pending.extend(connective.children.iter().copied());
                }
                _ => {}
            }
        }
        false
    }

    /// Whether one predication is exactly `memberOf(candidate, source)`.
    #[requires(true)]
    #[ensures(true)]
    fn predication_states_membership(
        &self,
        predication: SemanticObjectId,
        candidate: SemanticObjectId,
        source: SemanticObjectId,
    ) -> bool {
        let Some(node) = self
            .graph
            .objects
            .get(&predication)
            .and_then(|object| object.as_predication())
        else {
            return false;
        };
        let data!(PredicationRelation::Named { relation }) = node.relation.as_data() else {
            return false;
        };
        relation == "memberOf"
            && node.arguments.len() == 2
            && plain_argument_value(&node.arguments, 1) == Some(candidate)
            && plain_argument_value(&node.arguments, 2) == Some(source)
    }

    /// Extend a rendering environment with the events a formula generates.
    ///
    /// Each generated event is bound at the eventuality subtype its own node
    /// records, which is exactly the type `bind_generated_events` declares for
    /// it, so a use inside the body agrees with the binder that will appear
    /// above it.
    #[requires(true)]
    #[ensures(ret.len() >= bound.len())]
    fn scope_generated_events(&self, bound: &Bound, generated: &[SemanticObjectId]) -> Bound {
        let mut scoped = bound.clone();
        for event in generated.iter().copied() {
            if let Some(declared_type) = sort_type_expr(self.generated_event_sort(event)) {
                scoped.insert(event, BoundValue::Value(declared_type));
            }
        }
        scoped
    }

    /// The semantic sort one generated event binder is declared at.
    #[requires(true)]
    #[ensures(true)]
    fn generated_event_sort(&self, event: SemanticObjectId) -> SemanticSort {
        self.graph
            .objects
            .get(&event)
            .and_then(|object| object.as_eventuality())
            .map_or_else(SemanticSort::eventuality, |node| {
                SemanticSort::Eventuality(node.sort)
            })
    }

    /// Project the canonical flat tanru graph to the registered relation former
    /// `(Tanru modifier head)` only after proving the link predication, head
    /// conjunct, property abstraction, and absorbed modifier event are private
    /// and otherwise default.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_tanru_projection(
        &self,
        id: SemanticObjectId,
        node: &crate::model::ConnectiveFormulaNode,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
        modes: AcceptedModes,
    ) -> Option<Content> {
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
        if !modes.admits(head.mode)
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
                self.plan.usage(),
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
        // `Tanru` composes relation identities and preserves the tertau's row,
        // so the row this former is applied against is the head predication's.
        let former = RelationRef::Tanru {
            modifier: Box::new(RelationRef::Lexical(
                LexicalRoot::try_new(modifier_relation).ok()?,
            )),
            head: Box::new(RelationRef::Lexical(
                LexicalRoot::try_new(head_relation).ok()?,
            )),
        };
        // The projected `Tanru` application is one `Close` like any other, so it
        // owns the section-5.1 effect scope of the places it omits.
        self.open_closure_scope(Some(ClosureSite::Main(head_atom.predication)), bound);
        let closed = self
            .render_argument_map(former, &head.arguments, bound, active, head.eventuality)
            .ok()
            .and_then(|application| Content::close(application).ok());
        let mut conjuncts = vec![self.close_closure_scope(closed)?];
        conjuncts.extend(
            head.adjuncts
                .iter()
                .enumerate()
                .map(|(index, adjunct)| {
                    self.render_modal(
                        ClosureSite::Adjunct(head_atom.predication, index),
                        adjunct,
                        bound,
                        active,
                    )
                })
                .collect::<Option<Vec<_>>>()?,
        );
        let value = if conjuncts.len() == 1 {
            conjuncts.pop().expect("one tanru application")
        } else {
            Content::junction(JunctionOp::Joi, conjuncts).ok()?
        };
        let generated = node
            .bound_eventualities
            .iter()
            .map(|event| event.object_id())
            .collect::<Vec<_>>();
        self.bind_generated_events(id, &generated, Some(value), bound, active)
    }

    /// Add explicit generated-event binders only when sharing or facets make
    /// the graph-owned closure identity observable.
    #[requires(true)]
    #[ensures(true)]
    fn bind_generated_events(
        &self,
        owner: SemanticObjectId,
        generated: &[SemanticObjectId],
        body: Option<Content>,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Content> {
        // A default generated event has exactly two edges — the closure owner's
        // and its predication's — and its predication absorbs the second, so
        // nothing inside the body can name it and no binder is observable.
        let visible = generated
            .iter()
            .copied()
            .filter(|event| {
                !generated_event_is_default(self.graph, self.plan.usage(), owner, *event)
            })
            .collect::<Vec<_>>();
        if visible.is_empty() {
            return body;
        }
        let scoped = self.scope_generated_events(bound, &visible);
        let facets = visible
            .iter()
            .flat_map(|event| self.generated_event_facets(*event, &scoped, active))
            .collect::<Vec<_>>();
        let mut parameters = Vec::with_capacity(visible.len());
        for event in visible.iter().copied() {
            parameters.push(TypedParameter::new(
                object_variable(event),
                sort_type_expr(self.generated_event_sort(event))
                    .expect("every EventualitySort has a closed v0 subtype atom"),
            ));
        }
        let facets = facets.into_iter().collect::<Option<Vec<_>>>();
        let body = match facets {
            Some(facets) if facets.is_empty() => body?,
            Some(facets) => Content::junction(
                JunctionOp::Joi,
                std::iter::once(body?).chain(facets).collect(),
            )
            .ok()?,
            None => return None,
        };
        Lambda::new(parameters, body)
            .ok()
            .map(|lambda| Content::quantified(QuantifierOp::Exists, lambda))
    }

    /// Every non-default generated-event coordinate becomes either a fixed
    /// exact intrinsic or one complete typed facet payload.
    #[requires(self.graph.objects.contains_key(&event))]
    #[ensures(true)]
    fn generated_event_facets(
        &self,
        event: SemanticObjectId,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Vec<Option<Content>> {
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
            return vec![self.anchor_facet(relation, event, time.anchor, bound, active)];
        }
        vec![self.typed_event_facet(event, bound, active)]
    }

    /// Relate a generated event to its time anchor through one fixed relation.
    #[requires(self.graph.objects.contains_key(&anchor))]
    #[ensures(true)]
    fn anchor_facet(
        &self,
        relation: &str,
        event: SemanticObjectId,
        anchor: SemanticObjectId,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Content> {
        let event_use = bound_use(event, bound.get(&event)?);
        let anchor = self
            .render_id(anchor, bound, active, AcceptedModes::Unlicensed)
            .and_then(Elaborated::into_operand)?;
        let row = Row::new(
            vec![
                RowSlot::new(PlaceLabel::numbered(1), event_use.value_type()),
                RowSlot::new(PlaceLabel::numbered(2), anchor.value_type()),
            ],
            false,
        );
        let term = PredTerm::relation(PredicateSignature::new(
            RelationRef::Lexical(LexicalRoot::try_new(relation).ok()?),
            row,
        ));
        let applied = PredTerm::applied(
            term,
            vec![PlaceFill::plain(event_use), PlaceFill::plain(anchor)],
        )
        .ok()?;
        Content::close(applied).ok()
    }

    /// Decline one generated event whose facets match no fixed intrinsic rule.
    #[requires(self.graph.objects.contains_key(&event))]
    #[ensures(ret.is_none())]
    fn typed_event_facet(
        &self,
        event: SemanticObjectId,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Content> {
        self.fallback_object(
            event,
            bound,
            active,
            CompactFallbackCause::EventualityFacets,
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
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
        modes: AcceptedModes,
    ) -> Option<Content> {
        if !modes.admits(node.mode) {
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
        if matches!(
            node.relation.as_data(),
            data!(PredicationRelation::Composition)
        ) {
            return self.fallback_object(
                id,
                bound,
                active,
                CompactFallbackCause::CompositionPredication,
            );
        }
        if let data!(PredicationRelation::Named { relation }) = node.relation.as_data()
            && LexicalRoot::try_new(relation).is_err()
        {
            return self.fallback_object(
                id,
                bound,
                active,
                CompactFallbackCause::NonAtomicRelation,
            );
        }
        // Section 5.1's evaluation site. Everything the place walk below omits
        // is computed here, in this closure, in numbered-place order — not
        // pooled at the enclosing lambda or force segment, where a later
        // conjunct's lookup would run before the earlier conjunct it is
        // supposed to see the dynamic context of.
        self.open_closure_scope(Some(ClosureSite::Main(id)), bound);
        let application = match node.relation.as_data() {
            data!(PredicationRelation::Named { relation }) => {
                let root = LexicalRoot::try_new(relation).expect("the atomic spelling is proved");
                self.render_argument_map(
                    RelationRef::Lexical(root),
                    &node.arguments,
                    bound,
                    active,
                    node.eventuality,
                )
            }
            data!(PredicationRelation::Parameter { parameter }) => {
                // The only relation identity a graph binds is the open-row
                // variable of a `mo` question, and its row comes from that
                // binder rather than from this predication's place map.
                let head = self
                    .render_id(*parameter, bound, active, AcceptedModes::Unlicensed)
                    .and_then(Elaborated::into_operand)
                    .and_then(|operand| match operand {
                        Operand::Predicate(term) => Some(term),
                        _ => None,
                    });
                match head {
                    Some(head) => self.apply_argument_map(
                        head,
                        &node.arguments,
                        bound,
                        active,
                        node.eventuality,
                    ),
                    None => Err(FillFailure::ValueDeclined),
                }
            }
            data!(PredicationRelation::Composition) => {
                unreachable!("a composition predication declined before its scope was opened")
            }
        };
        // The scope is closed before any decline is reported, so a route that
        // gives up still leaves the frame stack as it found it.
        let (closed, failure) = match application {
            Ok(application) => (Content::close(application).ok(), None),
            Err(failure) => (None, Some(failure)),
        };
        let closed = self.close_closure_scope(closed);
        match failure {
            None => {}
            // A value that declined at its own boundary has already said so;
            // reporting the place map as well would attribute one failure
            // twice. The rest of this predication still has boundaries of its
            // own — its adjuncts and its event — so they are still reached.
            Some(FillFailure::ValueDeclined) => {
                self.record_declined_children(id, bound, active);
                return None;
            }
            Some(FillFailure::Unrepresentable) => {
                return self.fallback_object(
                    id,
                    bound,
                    active,
                    CompactFallbackCause::ArgumentFields,
                );
            }
        }
        let Some(closed) = closed else {
            return self.fallback_object(
                id,
                bound,
                active,
                CompactFallbackCause::TypedConstructionRejected,
            );
        };
        let mut conjuncts = vec![closed];
        for (index, adjunct) in node.adjuncts.iter().enumerate() {
            let Some(modal) =
                self.render_modal(ClosureSite::Adjunct(id, index), adjunct, bound, active)
            else {
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
            Content::junction(JunctionOp::Joi, conjuncts).ok()?
        };
        self.recognized(Some(term))
    }

    /// Render numbered arguments with canonical `:n` and `:Eventuality`
    /// markers, against the effective row the graph's place map attests.
    ///
    /// There is no lexical signature registry at render time — `render_smusni`
    /// takes no dictionary — so the row a lexical term is applied against is the
    /// one the graph itself records: the places its argument map names, the
    /// places it deletes, and the distinguished event place when it carries one.
    /// What the application kernel then checks is exactly what this notation can
    /// check without a signature table: the fill cursor, `:n` labelling,
    /// duplicate and already-filled places, and `DropPlace` legality. Lexical
    /// place *types* are not checked, because nothing at this layer attests
    /// them; that is the seat the registered
    /// `smusni.projection.lexical-signature-missing-or-stale` reason reserves.
    #[requires(true)]
    #[ensures(true)]
    fn render_argument_map(
        &self,
        relation: RelationRef,
        arguments: &BTreeMap<PlaceIndex, ArgumentValue>,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
        eventuality: Option<SemanticObjectId>,
    ) -> Result<PredTerm, FillFailure> {
        let eventuality = eventuality.filter(|id| !self.eventuality_is_absorbed(*id));
        let fills = self.render_fills(arguments, bound, active, eventuality)?;
        let mut slots = Vec::with_capacity(arguments.len() + 1);
        for place in arguments.keys() {
            let Ok(place) = u32::try_from(place.get()) else {
                return Err(FillFailure::Unrepresentable);
            };
            // A place this map fills is accepted at the type the graph puts
            // there; a place it deletes or silently defaults is an ordinary
            // referential place, which is what section 5.1 closes.
            let accepted = fills
                .iter()
                .find(|fill| fill.place == Some(place))
                .map_or_else(entity_referents, |fill| fill.value.value_type());
            slots.push(RowSlot::new(PlaceLabel::numbered(place), accepted));
        }
        if eventuality.is_some() {
            let Some(accepted) = referents_type_expr(SemanticSort::eventuality()) else {
                return Err(FillFailure::Unrepresentable);
            };
            slots.push(RowSlot::new(PlaceLabel::Eventuality, accepted));
        }
        let mut term =
            PredTerm::relation(PredicateSignature::new(relation, Row::new(slots, false)));
        for place in deleted_places(arguments) {
            let Ok(place) = u32::try_from(place.get()) else {
                return Err(FillFailure::Unrepresentable);
            };
            term = PredTerm::drop_place(&term, PositiveInteger::from_u32(place))
                .map_err(|_| FillFailure::Unrepresentable)?;
        }
        apply_fills(term, fills).ok_or(FillFailure::Unrepresentable)
    }

    /// Apply one already typed predicate term to the graph's place map.
    #[requires(true)]
    #[ensures(true)]
    fn apply_argument_map(
        &self,
        head: PredTerm,
        arguments: &BTreeMap<PlaceIndex, ArgumentValue>,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
        eventuality: Option<SemanticObjectId>,
    ) -> Result<PredTerm, FillFailure> {
        let eventuality = eventuality.filter(|id| !self.eventuality_is_absorbed(*id));
        let fills = self.render_fills(arguments, bound, active, eventuality)?;
        apply_fills(head, fills).ok_or(FillFailure::Unrepresentable)
    }

    /// Render the ordered fills one place map contributes.
    ///
    /// A fill is labelled exactly when the cursor is not already standing on its
    /// place, which is the same rule the application kernel advances by.
    #[requires(true)]
    #[ensures(true)]
    fn render_fills(
        &self,
        arguments: &BTreeMap<PlaceIndex, ArgumentValue>,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
        eventuality: Option<SemanticObjectId>,
    ) -> Result<Vec<RenderedFill>, FillFailure> {
        let deleted = deleted_places(arguments);
        let mut next = 1usize;
        let mut fills = Vec::new();
        // A value that declines does not end the walk: the remaining places
        // have boundaries of their own, and one document reports every failed
        // edge it has rather than only the first.
        let mut declined = false;
        for (place, argument) in arguments {
            if argument.kind == ArgumentValueKind::Deleted {
                if argument.introduced_by.as_deref() != Some("zi'o") {
                    return Err(FillFailure::Unrepresentable);
                }
                continue;
            }
            while deleted.contains(&PlaceIndex::new(next)) {
                next += 1;
            }
            if argument.quantity.is_some()
                || !self.argument_clauses_are_discharged(argument)
                || argument.command_target.is_some()
                || (argument.kind == ArgumentValueKind::Elided
                    && argument.introduced_by.as_deref() != Some("zo'e"))
            {
                return Err(FillFailure::Unrepresentable);
            }
            let Some(value) = argument.value else {
                return Err(FillFailure::Unrepresentable);
            };
            if argument.kind == ArgumentValueKind::Elided
                && self.default_elided_is_silent(value, bound)
            {
                continue;
            }
            let Some(value) = self
                .render_id(value, bound, active, AcceptedModes::Unlicensed)
                .and_then(Elaborated::into_operand)
            else {
                declined = true;
                next = place.get() + 1;
                continue;
            };
            let labelled = place.get() != next;
            let Ok(place) = u32::try_from(place.get()) else {
                return Err(FillFailure::Unrepresentable);
            };
            fills.push(new!(RenderedFill {
                place: Some(place),
                labelled: labelled,
                value: value
            }));
            next = place as usize + 1;
        }
        if let Some(eventuality) = eventuality {
            let owner = self.plan.binder_owner(eventuality);
            let silent = owner.is_some_and(|owner| {
                generated_event_is_default(self.graph, self.plan.usage(), owner, eventuality)
            });
            if !silent {
                let Some(value) = self
                    .render_id(eventuality, bound, active, AcceptedModes::Unlicensed)
                    .and_then(Elaborated::into_operand)
                else {
                    return Err(FillFailure::ValueDeclined);
                };
                fills.push(new!(RenderedFill {
                    place: None,
                    labelled: true,
                    value: value
                }));
            }
        }
        if declined {
            return Err(FillFailure::ValueDeclined);
        }
        Ok(fills)
    }

    /// Named default for an unshared elided contextual referent.
    ///
    /// Section 5.1 gives `Close` exactly one silent default: rule 1's fresh bare
    /// `Context` for a place whose graph dependence is `Fixed`. An
    /// `Underspecified { mayDependOn }` one "cannot be hidden by `Close`: it is
    /// bound explicitly from `(Context dependencies...)` and the same bound
    /// value fills the place. This preserves the exact permitted dependency set
    /// rather than replacing it with 'all accessible binders.'" So a recorded
    /// dependence that names the complete derived universe is still not silent:
    /// eliding it would leave re-elaboration to *rederive* that universe at the
    /// printed site rather than read the permission the graph recorded, and the
    /// two coincide only by accident of this site. Rule 2's shared default is
    /// already loud for a different reason — it is placed as a declaration — and
    /// the use-count test below is what keeps it out of this branch.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn default_elided_is_silent(&self, id: SemanticObjectId, bound: &Bound) -> bool {
        let Some(node) = self.graph.objects[&id].as_referent() else {
            return false;
        };
        let Some(dependence) = node.scope_dependence.as_ref() else {
            return false;
        };
        if !default_elided_shape(node)
            || !self.graph.objects[&id].diagnostics().is_empty()
            || self.plan.use_count(id) != 1
            // A recorded `mayDependOn` list is the loud case, whatever it names.
            || dependence.may_depend_on().is_some()
        {
            return false;
        }
        self.scope_dependence_is_default(id, Some(dependence), bound)
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
        bound: &Bound,
    ) -> bool {
        let Some(universe) = self.binder_universes.get(&id) else {
            return false;
        };
        if !universe.iter().all(|binder| bound.contains_key(binder)) {
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
    ///
    /// An adjunct is a `Joi` operand of its own, so it is its own `Close` and
    /// therefore its own section-5.1 effect scope: the defaults it omits belong
    /// inside its operand, evaluated at the recorded nonlogical locus in the
    /// same left-to-right flow section 6.2 gives conjunction, not hoisted above
    /// the junction beside the predication's.
    #[requires(true)]
    #[ensures(true)]
    fn render_modal(
        &self,
        site: ClosureSite,
        adjunct: &Adjunct,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Content> {
        if adjunct.component.is_some()
            || adjunct.negation.is_some()
            || adjunct.scalar_negation.is_some()
            || !adjunct.modifiers.is_empty()
        {
            return None;
        }
        let relation = adjunct.relation.as_ref()?;
        let root = LexicalRoot::try_new(relation).ok()?;
        self.open_closure_scope(Some(site), bound);
        let closed = self
            .render_argument_map(
                RelationRef::Lexical(root),
                &adjunct.arguments,
                bound,
                active,
                None,
            )
            .ok()
            .and_then(|application| Content::close(application).ok());
        self.close_closure_scope(closed)
    }

    /// Render reference constructors, indexicals, composition, abstractions,
    /// and explicit contextual constants.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_referent(
        &self,
        id: SemanticObjectId,
        node: &ReferentNode,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Operand> {
        if let Some(indexical) = exact_referent_indexical(node) {
            return self.recognized(registered_constant(indexical_constant(indexical)));
        }
        if let Some(proximity) = exact_deictic(node, self.graph) {
            return self.recognized(registered_constant(proximity));
        }
        if let Some(description) = self.render_description(id, node, bound, active) {
            return self.recognized(Some(description));
        }
        match self.render_referent_abstraction(id, node, bound, active) {
            Routed::Value(abstraction) => return self.recognized(Some(abstraction)),
            Routed::Declined => return None,
            Routed::Unmatched => {}
        }
        if node.composition.is_some() {
            return self.fallback_object(
                id,
                bound,
                active,
                CompactFallbackCause::CompositionFields,
            );
        }
        if default_elided_shape(node) {
            let Some(dependence) = node.scope_dependence.as_ref() else {
                return self.fallback_object(
                    id,
                    bound,
                    active,
                    CompactFallbackCause::ConstantWithoutDependence,
                );
            };
            if self.reference_binding_frames.borrow().is_empty() {
                // Outside every force segment there is no host position at all,
                // which is not section 6.3's legality question but the
                // enclosing value's own failure to be placed; that failure is
                // already recorded by whatever could not place it.
                return None;
            }
            let Some(hosted) = self.host_context(id, node.sort, dependence) else {
                // Every rule held except the one section 6.3 ends with: an open
                // position exists but none has this constant's dependencies
                // live, so the placement rules and the graph together determine
                // no legal host.
                return self.fallback_object(
                    id,
                    bound,
                    active,
                    CompactFallbackCause::DynamicHostIllegal,
                );
            };
            return self.recognized(Some(hosted));
        }
        self.fallback_object(id, bound, active, CompactFallbackCause::ReferentFields)
    }

    /// Build the builder's exact `le` encoding as a lexical `skicu` claim.
    #[requires(true)]
    #[ensures(true)]
    fn speaker_description(&self, subject: &Operand, described: FnValue) -> Option<Content> {
        let fills = vec![
            registered_constant(Intrinsic::Speaker)?,
            subject.clone(),
            registered_constant(Intrinsic::Audience)?,
            Operand::Function(described),
        ];
        let slots = fills
            .iter()
            .enumerate()
            .map(|(index, fill)| {
                RowSlot::new(
                    PlaceLabel::numbered(u32::try_from(index + 1).expect("four places fit")),
                    fill.value_type(),
                )
            })
            .collect();
        let term = PredTerm::relation(PredicateSignature::new(
            RelationRef::Lexical(LexicalRoot::try_new("skicu").ok()?),
            Row::new(slots, false),
        ));
        let applied =
            PredTerm::applied(term, fills.into_iter().map(PlaceFill::plain).collect()).ok()?;
        Content::close(applied).ok()
    }

    /// Host one contextual computation at the enclosing force segment.
    ///
    /// `Context` is a `RefComp`, not a value: section 5.1 says an
    /// `Underspecified { mayDependOn }` default "cannot be hidden by `Close`: it
    /// is bound explicitly from `(Context dependencies...)` and the same bound
    /// value fills the place", and a fixed one is the same computation with no
    /// dependencies. So a contextual constant introduces a binder exactly the
    /// way a description does, and the place is filled by the name it binds.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn host_context(
        &self,
        id: SemanticObjectId,
        sort: SemanticSort,
        dependence: &crate::model::ScopeDependence,
    ) -> Option<Operand> {
        let declared_type = referents_type_expr(sort)?;
        let dependencies = match dependence.as_data() {
            data!(ScopeDependence::Fixed) => Vec::new(),
            data!(ScopeDependence::Underspecified { may_depend_on }) => may_depend_on
                .iter()
                .map(|binder| object_variable(*binder))
                .collect(),
        };
        let computation = RefComp::context(dependencies, declared_type.clone()).ok()?;
        let binding = ReferenceBinding {
            id,
            declared_type: declared_type.clone(),
            computation,
        };
        self.host_locally(binding, self.hosted_dependencies(id))
            .then(|| Operand::Value(Value::bound(object_variable(id), declared_type)))
    }

    /// Lower one exact fixed `lo`, `le`, or `la` description into a hosted
    /// `Refer` computation. The `le` branch retains the builder's explicit
    /// speaker/audience `skicu` property and never asserts the base
    /// classification. Unsupported description families remain typed fallback;
    /// incidental relatives are not turned into conjuncts.
    #[requires(true)]
    #[ensures(true)]
    fn render_description(
        &self,
        id: SemanticObjectId,
        node: &ReferentNode,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Operand> {
        if node.sort != SemanticSort::Entity
            || !self.projected_descriptions.contains(&id)
            || self.reference_binding_frames.borrow().is_empty()
            // Section 5.1's default is a property of the *site*: a fixed
            // dependence is default only where no binder is accessible, and an
            // underspecified one only where it names the complete derived
            // universe. Anything else is a policy this position does not state,
            // and the description keeps its registered refusal.
            || !self.scope_dependence_is_default(id, node.scope_dependence.as_ref(), bound)
        {
            return None;
        }
        let recognition = recognize_description(self.graph, self.plan.usage(), id, node)?;
        let declared_type = referents_type_expr(node.sort)?;
        let variable = object_variable(id);
        let mut scoped = bound.clone();
        scoped.insert(id, BoundValue::Value(declared_type.clone()));
        // Section 8.3: a nested reference computation required while evaluating
        // this description's property runs *inside* this computation unless the
        // graph gives that nested effect its own legal outer host, which no
        // version-0 graph does. The property is therefore a barrier, and this
        // is what replaces the old refusal to emit any nesting at all.
        self.open_host_frame(HostFrameKind::Barrier, &scoped);
        let body =
            self.description_property(id, node, &recognition, &declared_type, &scoped, active);
        let nested = self.close_host_frame();
        let body = wrap_reference_bindings(nested, body?, Content::bind_form)?;
        let property = Lambda::new(
            vec![TypedParameter::new(variable.clone(), declared_type.clone())],
            body,
        )
        .ok()?;
        let dependencies = property_dependencies(bound, &property);
        let binding = ReferenceBinding {
            id,
            declared_type: declared_type.clone(),
            computation: RefComp::refer(property).ok()?,
        };
        // Section 6.3 raises this `Bind` to the outermost point that is still
        // inside every boundary it may not leave; the binders its property
        // actually names are what decide the lambda-dependency half of that.
        self.host_reference(binding, &dependencies)
            .then(|| Operand::Value(Value::bound(variable, declared_type)))
    }

    /// Whether every relative clause on this argument is already printed above
    /// it by the binder it restricts.
    ///
    /// The only clause an argument may carry and still fill an ordinary place
    /// is the repeat of its own quantifier's restriction: section 8.4 conjoins
    /// that clause with the restriction at the quantifier's locus, so printing
    /// it again here would state the restriction twice.
    #[requires(true)]
    #[ensures(ret == argument.relative_clauses.is_empty() || ret)]
    fn argument_clauses_are_discharged(&self, argument: &ArgumentValue) -> bool {
        argument
            .relative_clauses
            .iter()
            .all(|clause| self.discharged_restrictions.get(&clause.body) == argument.value.as_ref())
    }

    /// The binders one hosted computation's property depends on.
    #[requires(true)]
    #[ensures(true)]
    fn hosted_dependencies(&self, id: SemanticObjectId) -> &BTreeSet<SemanticObjectId> {
        static NONE: BTreeSet<SemanticObjectId> = BTreeSet::new();
        self.binder_universes.get(&id).unwrap_or(&NONE)
    }

    /// Build the descriptive property of one recognized description.
    ///
    /// This is the whole body of the `Refer` lambda: the base description
    /// property conjoined, in source order, with the restrictive clauses that
    /// modify the same reference.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn description_property(
        &self,
        id: SemanticObjectId,
        node: &ReferentNode,
        recognition: &DescriptionRecognition<'_>,
        declared_type: &TypeExpr,
        scoped: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Content> {
        let subject = Operand::Value(Value::bound(object_variable(id), declared_type.clone()));
        let property = match recognition.as_data() {
            data!(DescriptionRecognition::Property {
                constructor,
                property,
                arguments,
                parameter: None,
            }) if *constructor == DescriptionConstructor::Lo => {
                // The base property is its own `Close`, so a section-5.1
                // default inside it belongs to that conjunct rather than to the
                // whole `∧` a restrictive clause would build around it.
                let root = RelationRef::Lexical(LexicalRoot::try_new(property).ok()?);
                self.open_closure_scope(None, scoped);
                let closed = self
                    .render_argument_map(root, arguments, scoped, active, None)
                    .ok()
                    .and_then(|term| Content::close(term).ok());
                self.close_closure_scope(closed)?
            }
            data!(DescriptionRecognition::Property {
                constructor,
                property,
                arguments,
                parameter: Some(parameter),
            }) if *constructor == DescriptionConstructor::Le => {
                let root = RelationRef::Lexical(LexicalRoot::try_new(property).ok()?);
                let mut property_scope = scoped.clone();
                property_scope.insert(*parameter, BoundValue::Value(declared_type.clone()));
                // The described property is a lambda of its own, so it is a
                // section-6.3 host position: a computation naming the described
                // candidate — which is what a section-5.1 `Context` inside this
                // property depends on — may not be raised out of the binder
                // that introduces it. Without this frame the innermost open
                // position is the description's barrier, one binder too far out.
                self.open_host_frame(
                    HostFrameKind::Binders([*parameter].into_iter().collect()),
                    &property_scope,
                );
                let term = self.render_argument_map(root, arguments, &property_scope, active, None);
                let hosted = self.close_host_frame();
                let term = term.ok()?;
                let body = wrap_reference_bindings(
                    hosted,
                    Content::close(term).ok()?,
                    Content::bind_form,
                )?;
                let candidate =
                    TypedParameter::new(object_variable(*parameter), declared_type.clone());
                let described = callable_property(&candidate, Some(body))?;
                // The builder's `le` encoding is a `skicu` predication over the
                // speaker, the described referent, the audience, and the
                // property; it is a lexical relation like any other, so it is
                // applied against the row those four operands attest.
                self.speaker_description(&subject, described)?
            }
            data!(DescriptionRecognition::Property { .. }) => {
                unreachable!("validated recognition pairs each constructor with its parameter")
            }
            data!(DescriptionRecognition::Name { name }) => Content::intrinsic(
                Intrinsic::Named,
                vec![
                    Operand::Value(Value::literal(Literal::text(*name))),
                    subject.clone(),
                ],
            )
            .ok()?,
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
        let mut conjuncts = vec![property];
        let rendered = clauses
            .into_iter()
            .map(|clause| {
                self.render_id(clause.body, scoped, active, AcceptedModes::NoForce)
                    .and_then(Elaborated::into_content)
            })
            .collect::<Vec<_>>();
        conjuncts.extend(rendered.into_iter().collect::<Option<Vec<_>>>()?);
        if conjuncts.len() == 1 {
            return conjuncts.pop();
        }
        Content::junction(JunctionOp::And, conjuncts).ok()
    }

    /// Whether this event identity is currently consumed by its surrounding
    /// section-11.3 crossing instead of filling the content root's ordinary
    /// distinguished-event place.
    #[requires(true)]
    #[ensures(true)]
    fn eventuality_is_absorbed(&self, id: SemanticObjectId) -> bool {
        self.absorbed_eventualities.borrow().last() == Some(&id)
    }

    /// Render one exact section-11.3 two-operand abstraction crossing.
    ///
    /// An omitted operand is not a `Close` default and is not description
    /// raising: it is one fresh local `Bind` whose `Context` computation has the
    /// type declared for this crossing. An explicit graph operand is rendered
    /// at its own type and the kernel application is the final type gate.
    #[requires(self.graph.objects.contains_key(&id))]
    #[requires(self.graph.objects.contains_key(&body))]
    #[requires(trailing.is_none_or(|operand| self.graph.objects.contains_key(&operand)))]
    #[requires(matches!(
        intrinsic,
        Intrinsic::Measure
            | Intrinsic::TruthValue
            | Intrinsic::ExperienceOf
            | Intrinsic::ProcessOf
            | Intrinsic::ActivityOf
            | Intrinsic::Concept
            | Intrinsic::Abstract
    ))]
    #[requires(matches!(default_type, TypeExpr::Referents(_)))]
    #[ensures(true)]
    fn render_abstraction_crossing(
        &self,
        id: SemanticObjectId,
        body: SemanticObjectId,
        intrinsic: Intrinsic,
        trailing: Option<SemanticObjectId>,
        default_type: TypeExpr,
        absorbs_eventuality: bool,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Routed {
        // A crossing is a reification boundary. Computations discovered in its
        // content remain part of that content and cannot be raised around the
        // crossing itself.
        self.open_host_frame(HostFrameKind::Barrier, bound);
        if absorbs_eventuality {
            self.absorbed_eventualities.borrow_mut().push(id);
        }
        let rendered = self
            .render_id(body, bound, active, AcceptedModes::NoForce)
            .and_then(Elaborated::into_content);
        if absorbs_eventuality {
            let popped = self.absorbed_eventualities.borrow_mut().pop();
            debug_assert_eq!(popped, Some(id));
        }
        let nested = self.close_host_frame();
        let Some(rendered) = rendered else {
            return Routed::Declined;
        };
        let Some(content) = wrap_reference_bindings(nested, rendered, Content::bind_form) else {
            return self.construction_rejected(id);
        };

        if let Some(trailing) = trailing {
            let Some(operand) = self
                .render_id(trailing, bound, active, AcceptedModes::Unlicensed)
                .and_then(Elaborated::into_operand)
            else {
                return Routed::Declined;
            };
            return match Value::intrinsic(intrinsic, vec![Operand::Content(content), operand]) {
                Ok(value) => Routed::Value(Operand::Value(value)),
                Err(_) => {
                    self.record_object_fallback(id, CompactFallbackCause::CrossingOperandMismatch);
                    Routed::Declined
                }
            };
        }

        let variable = crossing_default_variable(id, intrinsic);
        let Ok(computation) = RefComp::context(Vec::new(), default_type.clone()) else {
            return self.construction_rejected(id);
        };
        let Ok(value) = Value::intrinsic(
            intrinsic,
            vec![
                Operand::Content(content),
                Operand::Value(Value::bound(variable.clone(), default_type.clone())),
            ],
        ) else {
            return self.construction_rejected(id);
        };
        let Ok(binding) = Bind::new(variable, default_type, computation, value) else {
            return self.construction_rejected(id);
        };
        Routed::Value(Operand::Value(Value::bind_form(binding)))
    }

    /// Render the property, proposition, and section-11.3 crossings in both
    /// encodings the builder emits for them.
    ///
    /// A bare abstraction (`abstraction_kind` with no descriptor) and a
    /// gadri-folded one (`lo ka …`, which folds the descriptor into the
    /// abstraction object itself) denote the same crossing, so one route proves
    /// both. Folding is not a description *of* the abstraction: the graph
    /// records no second body predicating over the abstraction referent, so
    /// there is no `Refer` to build — section 11.1's "no `Property` or
    /// `Relation` record is needed around the function" is what licenses the
    /// object to *be* the lambda.
    ///
    /// Event-valued abstractions are section 11.2 reference computations
    /// instead, and take [`Self::render_described_event`].
    #[requires(true)]
    #[ensures(true)]
    fn render_referent_abstraction(
        &self,
        id: SemanticObjectId,
        node: &ReferentNode,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Routed {
        let (Some(kind), Some(body)) = (node.abstraction_kind, node.body) else {
            return Routed::Unmatched;
        };
        let descriptor_is_exact = node.descriptor.as_ref().is_none_or(|descriptor| {
            folded_abstraction_descriptor_is_exact(self.graph, descriptor)
        });
        if !descriptor_is_exact
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
        {
            return Routed::Unmatched;
        }
        if let Some((intrinsic, trailing, default_sort)) =
            exact_referent_abstraction_crossing(node, kind)
        {
            let Some(default_type) = referents_type_expr(default_sort) else {
                return self.construction_rejected(id);
            };
            return self.render_abstraction_crossing(
                id,
                body,
                intrinsic,
                trailing,
                default_type,
                false,
                bound,
                active,
            );
        }
        let folded = match node.descriptor.as_ref() {
            None => referent_except_abstraction_is_default(node),
            Some(_) => referent_except_described_abstraction_is_default(node),
        };
        if !folded {
            return Routed::Unmatched;
        }
        if !matches!(
            kind,
            AbstractionKind::Property | AbstractionKind::Proposition
        ) || (kind == AbstractionKind::Property && node.parameters.is_empty())
            || (kind == AbstractionKind::Proposition && !node.parameters.is_empty())
        {
            return Routed::Unmatched;
        }
        // A `ce'u` place is typed by the sort the graph gave it. A sort with no
        // closed version-0 reference type — a relation-sorted parameter is the
        // only one the model can currently mint — has no lambda parameter to
        // declare, so the crossing stays unavailable rather than borrowing an
        // unrelated type.
        let mut parameters = Vec::with_capacity(node.parameters.len());
        let mut scoped = bound.clone();
        for parameter in node.parameters.iter().copied() {
            let Some(declared_type) = self.graph.objects[&parameter]
                .sort()
                .and_then(referents_type_expr)
            else {
                return Routed::Unmatched;
            };
            scoped.insert(parameter, BoundValue::Value(declared_type.clone()));
            parameters.push(TypedParameter::new(
                object_variable(parameter),
                declared_type,
            ));
        }
        // Section 6.3 stops raising at a reification, and an abstraction is
        // exactly that: a reference computation inside the abstracted content
        // is part of what the abstraction denotes, so hoisting it outside would
        // read the description de re where the graph wrote it de dicto. The
        // frame carries this crossing's own binders, so a computation that
        // depends on a `ce'u` is hostable inside it.
        self.open_host_frame(HostFrameKind::Barrier, &scoped);
        let rendered = self
            .render_id(body, &scoped, active, AcceptedModes::NoForce)
            .and_then(Elaborated::into_content);
        let abstracted = self.close_host_frame();
        // The shape is proved from here on, so a declining body has already
        // recorded its own boundary and this object adds none.
        let Some(rendered) = rendered else {
            return Routed::Declined;
        };
        let Some(body) = wrap_reference_bindings(abstracted, rendered, Content::bind_form) else {
            return self.construction_rejected(id);
        };
        if kind == AbstractionKind::Proposition {
            return match Value::intrinsic(Intrinsic::Reify, vec![Operand::Content(body)]) {
                Ok(value) => Routed::Value(Operand::Value(value)),
                Err(_) => self.construction_rejected(id),
            };
        }
        match Lambda::new(parameters, Operand::Content(body)) {
            Ok(lambda) => Routed::Value(Operand::Function(FnValue::lambda(lambda))),
            Err(_) => self.construction_rejected(id),
        }
    }

    /// Record that a proved route could not construct its typed kernel value.
    #[requires(true)]
    #[ensures(matches!(ret, Routed::Declined))]
    fn construction_rejected(&self, id: SemanticObjectId) -> Routed {
        self.record_object_fallback(id, CompactFallbackCause::TypedConstructionRejected);
        Routed::Declined
    }

    /// Project a two-place experience, process, or activity abstraction.
    ///
    /// One-place members of these families remain the ordinary section-11.2
    /// `Refer` route. The crossing is selected only by an independently
    /// represented trailing graph operand, and it consumes the root's
    /// distinguished event self-fill rather than manufacturing a recursive
    /// value for the abstraction result.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_described_event_crossing(
        &self,
        id: SemanticObjectId,
        node: &EventualityNode,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Routed {
        let Some((body, intrinsic, trailing, default_sort)) =
            exact_event_abstraction_crossing(self.graph, node)
        else {
            return Routed::Unmatched;
        };
        if !self.projected_descriptions.contains(&id)
            || !self.scope_dependence_is_default(id, node.denotation.scope_dependence(), bound)
        {
            return Routed::Unmatched;
        }
        let Some(default_type) = referents_type_expr(default_sort) else {
            return self.construction_rejected(id);
        };
        self.render_abstraction_crossing(
            id,
            body,
            intrinsic,
            Some(trailing),
            default_type,
            true,
            bound,
            active,
        )
    }

    /// Project one described eventuality as specification section 11.2's
    /// ordinary reference computation.
    ///
    /// `lo nu broda` is not a crossing constructor at all: it is a `Refer`
    /// whose property binds the event explicitly and predicates the abstracted
    /// content of it, hosted by the same `Bind` machinery every other
    /// `RefComp` uses. That is why `EventOf`, `AchievementOf`, and `StateOf`
    /// are not primitives — section 11.2 and section 14.3's forbidden list
    /// agree — and why a more specific eventuality subtype needs nothing but a
    /// narrower parameter type, since its one-way upcast already permits the
    /// reference at the root's event place.
    ///
    /// The property must actually *name* its parameter. Section 11.2 requires
    /// the abstracted event to fill the distinguished event place of the
    /// content's root, and a content root that reports no single event — a
    /// branching connective is the one shape that genuinely does not — would
    /// leave a vacuous property describing nothing about the event. That stays
    /// the registered event-facet refusal rather than an invented link.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_described_event(
        &self,
        id: SemanticObjectId,
        node: &EventualityNode,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Routed {
        let (Some(descriptor), Some(kind), Some(content)) = (
            node.descriptor.as_ref(),
            node.abstraction_kind,
            node.content,
        ) else {
            return Routed::Unmatched;
        };
        // The pre-scan already proved this object's shape and reserved its
        // binder for the renderer, so its content subgraph is consumed here and
        // declared nowhere. What planning could not decide — whether the
        // recorded dependence is the derived one at *this* lexical position —
        // is decided here, exactly as for an ordinary description.
        if !self.projected_descriptions.contains(&id)
            || !described_eventuality_base_is_exact(node, descriptor, kind)
            || !self.scope_dependence_is_default(id, node.denotation.scope_dependence(), bound)
            || self.reference_binding_frames.borrow().is_empty()
        {
            return Routed::Unmatched;
        }
        let Some(declared_type) = referents_type_expr(SemanticSort::Eventuality(node.sort)) else {
            return Routed::Unmatched;
        };
        let variable = object_variable(id);
        let mut scoped = bound.clone();
        scoped.insert(id, BoundValue::Value(declared_type.clone()));
        // The abstraction is a reification, so section 6.3 stops every raise
        // here, and the frame carries the bound event so the content's own
        // computations can depend on it.
        self.open_host_frame(HostFrameKind::Barrier, &scoped);
        let rendered = self
            .render_id(content, &scoped, active, AcceptedModes::NoForce)
            .and_then(Elaborated::into_content);
        let facets = self.described_event_facets(id, node, &scoped, active);
        let nested = self.close_host_frame();
        // The route is proved from here on: a declining content boundary has
        // already recorded itself, and this object adds no second reason.
        let (Some(rendered), Some(facets)) = (rendered, facets) else {
            return Routed::Declined;
        };
        let mut conjuncts = vec![rendered];
        conjuncts.extend(facets);
        let body = if conjuncts.len() == 1 {
            conjuncts.pop().expect("one abstracted content")
        } else {
            match Content::junction(JunctionOp::Joi, conjuncts) {
                Ok(body) => body,
                Err(_) => return self.construction_rejected(id),
            }
        };
        let Some(body) = wrap_reference_bindings(nested, body, Content::bind_form) else {
            return self.construction_rejected(id);
        };
        if !free_binders_of(&body).contains(&variable) {
            // Nothing in the content fills the abstracted event's place, so the
            // property would say nothing about the event it abstracts.
            self.record_object_fallback(id, CompactFallbackCause::EventualityFacets);
            return Routed::Declined;
        }
        let Ok(property) = Lambda::new(
            vec![TypedParameter::new(variable.clone(), declared_type.clone())],
            body,
        ) else {
            return self.construction_rejected(id);
        };
        let dependencies = property_dependencies(bound, &property);
        let Ok(computation) = RefComp::refer(property) else {
            return self.construction_rejected(id);
        };
        let binding = ReferenceBinding {
            id,
            declared_type: declared_type.clone(),
            computation,
        };
        if !self.host_reference(binding, &dependencies) {
            // Section 6.3's closing rule, reached by a proved route: an open
            // position exists but none has every binder this property names.
            self.record_object_fallback(id, CompactFallbackCause::DynamicHostIllegal);
            return Routed::Declined;
        }
        Routed::Value(Operand::Value(Value::bound(variable, declared_type)))
    }

    /// Section 10.3 facets of one described eventuality, as ordinary
    /// predications of the bound event variable.
    ///
    /// A facet is not a record beside the abstraction: it conjoins into the
    /// very property the content occupies, exactly like any other predication
    /// of that event. `None` means an unregistered facet has already recorded
    /// the refusal section 10.3 requires.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn described_event_facets(
        &self,
        id: SemanticObjectId,
        node: &EventualityNode,
        scoped: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Vec<Content>> {
        if node.time.is_none() {
            return Some(Vec::new());
        }
        let anchored = exact_described_event_time_facet(node).and_then(|time| {
            let relation = match time.relation.as_str() {
                "before" => "purci",
                "after" => "balvi",
                "at" => "cabna",
                _ => return None,
            };
            Some((relation, time.anchor))
        });
        let Some((relation, anchor)) = anchored else {
            return self.fallback_object(
                id,
                scoped,
                active,
                CompactFallbackCause::EventualityFacets,
            );
        };
        self.anchor_facet(relation, id, anchor, scoped, active)
            .map(|facet| vec![facet])
    }

    /// Eventualities use the same abstraction family and fixed indexicals;
    /// facet-bearing referents retain their typed structural object.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_eventuality(
        &self,
        id: SemanticObjectId,
        node: &EventualityNode,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Operand> {
        if let Some(indexical) = exact_eventuality_indexical(node) {
            return self.recognized(registered_constant(indexical_constant(indexical)));
        }
        if node.denotation.is_generated_bound() {
            return match bound.get(&id) {
                Some(declaration) => Some(bound_use(id, declaration)),
                None => self.fallback_object(
                    id,
                    bound,
                    active,
                    CompactFallbackCause::UnboundGeneratedEvent,
                ),
            };
        }
        match self.render_described_event_crossing(id, node, bound, active) {
            Routed::Value(crossing) => return self.recognized(Some(crossing)),
            Routed::Declined => return None,
            Routed::Unmatched => {}
        }
        match self.render_described_event(id, node, bound, active) {
            Routed::Value(reference) => return self.recognized(Some(reference)),
            Routed::Declined => return None,
            Routed::Unmatched => {}
        }
        // `tu'a` is the one construction specification section 14.4 names a
        // tracked spec gap: its descriptor deliberately withholds *which*
        // abstraction about the operand is meant, and version 0 specifies no
        // faithful underspecified crossing to carry that. Its decline is
        // therefore language-design backlog, not the renderer backlog the
        // ordinary event-facet boundary reports, so it takes its own reason.
        let cause = if node
            .descriptor
            .as_ref()
            .is_some_and(|descriptor| descriptor.kind == DescriptorKind::AbstractionAbout)
        {
            CompactFallbackCause::AbstractionAboutUnspecified
        } else {
            CompactFallbackCause::EventualityFacets
        };
        self.fallback_object(id, bound, active, cause)
    }

    /// Render complete typed questions; use `Ask λ` only under its exact default.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_question(
        &self,
        id: SemanticObjectId,
        node: &crate::model::QuestionNode,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Act> {
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
        // The answer domain declares each slot's type, and the body uses the
        // slot at exactly that type, so the declaration is settled before the
        // body is rendered rather than reconstructed from it.
        let relation_row = (node.kind == QuestionKind::Relation && parameters.len() == 1)
            .then(|| self.exact_open_relation_question_row(node, parameters[0]))
            .flatten();
        let mut scoped = bound.clone();
        for parameter in parameters.iter().copied() {
            let declaration = match &relation_row {
                Some(row) => BoundValue::Predicate(row.clone()),
                None => BoundValue::Value(referents_type_expr(SemanticSort::Entity)?),
            };
            scoped.insert(parameter, declaration);
        }
        // An answered slot is a binder the query's own lambda introduces, and
        // section 6.3 will not let a computation naming it stand outside that
        // lambda — which is where a section-5.1 `Context` inside the question's
        // body would otherwise be hosted. A polar question binds nothing and
        // keeps the enclosing position it always had.
        let query_binders = !parameters.is_empty();
        if query_binders {
            self.open_host_frame(
                HostFrameKind::Binders(parameters.iter().copied().collect()),
                &scoped,
            );
        }
        let body = self
            .render_id(node.body, &scoped, active, AcceptedModes::Force)
            .and_then(Elaborated::into_content);
        let body = if query_binders {
            let hosted = self.close_host_frame();
            body.and_then(|body| wrap_reference_bindings(hosted, body, Content::bind_form))
        } else {
            body
        };
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
                return self.recognized(body.map(|body| Act::ask(Query::polar(body))));
            }
            if node.kind == QuestionKind::Argument
                && !parameters.is_empty()
                && parameters.iter().all(|parameter| {
                    self.graph.objects[parameter].sort() == Some(SemanticSort::Entity)
                })
            {
                let declared_type = referents_type_expr(SemanticSort::Entity)?;
                let slots = parameters
                    .iter()
                    .map(|parameter| {
                        TypedParameter::new(object_variable(*parameter), declared_type.clone())
                    })
                    .collect();
                return self.recognized(
                    body.and_then(|body| Lambda::new(slots, body).ok())
                        .map(|lambda| Act::ask(Query::open(lambda))),
                );
            }
            if node.kind == QuestionKind::Relation
                && let Some(row) = relation_row
            {
                let slot =
                    TypedParameter::new(object_variable(parameters[0]), TypeExpr::Predicate(row));
                return self.recognized(
                    body.and_then(|body| Lambda::new(vec![slot], body).ok())
                        .map(|lambda| Act::ask(Query::open(lambda))),
                );
            }
        }
        self.fallback_object(id, bound, active, CompactFallbackCause::QuestionSlotFields)
    }

    /// Exact `mo` parameter row for one atomic relation question. The open
    /// numbered tail preserves the respondent's ability to supply a predicate
    /// with additional places; every place already filled by the question and
    /// the graph-owned event slot remains explicit in the row.
    #[requires(self.graph.objects.contains_key(&node.body))]
    #[requires(parameter.object_kind() == SemanticObjectKind::Parameter)]
    #[ensures(ret.as_ref().is_none_or(|row| row.has_open_numbered_tail()))]
    fn exact_open_relation_question_row(
        &self,
        node: &crate::model::QuestionNode,
        parameter: SemanticObjectId,
    ) -> Option<Row> {
        if node.domain != SemanticSort::Relation {
            return None;
        }
        let formula = self.graph.objects[&node.body].as_formula()?;
        let data!(FormulaNode::Atom(atom)) = formula.as_data() else {
            return None;
        };
        if atom.bound_eventualities.len() != 1 {
            return None;
        }
        let event = atom.bound_eventualities[0].object_id();
        let predication = self.graph.objects[&atom.predication].as_predication()?;
        if !matches!(
            predication.relation.as_data(),
            data!(PredicationRelation::Parameter { parameter: relation }) if *relation == parameter
        ) || predication.mode != PredicationMode::Asserted
            || !predication_is_otherwise_plain(predication)
            || predication.eventuality != Some(event)
            || !generated_event_is_default(self.graph, self.plan.usage(), node.body, event)
            || predication.arguments.is_empty()
        {
            return None;
        }
        let entity_referents = referents_type_expr(SemanticSort::Entity)?;
        let eventuality_referents = referents_type_expr(SemanticSort::eventuality())?;
        let mut slots = Vec::with_capacity(predication.arguments.len() + 1);
        for (place, argument) in &predication.arguments {
            let value = plain_argument_value(&predication.arguments, place.get())?;
            if argument.kind != ArgumentValueKind::Filled
                || self.graph.objects[&value].sort() != Some(SemanticSort::Entity)
            {
                return None;
            }
            let place = u32::try_from(place.get()).ok()?;
            slots.push(RowSlot::new(
                PlaceLabel::numbered(place),
                entity_referents.clone(),
            ));
        }
        slots.push(RowSlot::new(PlaceLabel::Eventuality, eventuality_referents));
        Some(Row::new(slots, true))
    }

    /// Preserve quantity form, value, scale, comparison set, and question
    /// parameters. Numeric shorthands require an actual integer graph value.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_quantity(
        &self,
        id: SemanticObjectId,
        node: &QuantityNode,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Operand> {
        if node.form == QuantityForm::Exact
            && node.scale == QuantityScale::Count
            && node.value.question_parameters.is_empty()
            && node.comparison_set.is_none()
            && let Some(integer) = node.value.integer
        {
            return self.recognized(Some(Operand::Value(Value::literal(Literal::integer(
                i64::from(integer),
            )))));
        }

        self.fallback_object(id, bound, active, CompactFallbackCause::QuantityFields)
    }

    /// Render fixed math operators through the declared registry; named
    /// operators retain an explicit typed name.
    #[requires(self.graph.objects.contains_key(&id))]
    #[ensures(true)]
    fn render_math(
        &self,
        id: SemanticObjectId,
        node: &crate::model::MathExpressionNode,
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Operand> {
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
                    return self.recognized(Some(Operand::Value(Value::literal(
                        Literal::integer(i64::from(*value)),
                    ))));
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
                    data!(MathOperator::Add) => Intrinsic::Add,
                    data!(MathOperator::Multiply) => Intrinsic::Multiply,
                    data!(MathOperator::Subtract) => Intrinsic::Subtract,
                    data!(MathOperator::Divide) => Intrinsic::Divide,
                    _ => {
                        return self.fallback_object(
                            id,
                            bound,
                            active,
                            CompactFallbackCause::MathOperatorFields,
                        );
                    }
                };
                let arguments = operands
                    .iter()
                    .map(|operand| {
                        self.render_id(*operand, bound, active, AcceptedModes::Unlicensed)
                            .and_then(Elaborated::into_operand)
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .collect::<Option<Vec<_>>>()?;
                self.recognized(Value::intrinsic(head, arguments).ok().map(Operand::Value))
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
        bound: &Bound,
        active: &mut BTreeSet<SemanticObjectId>,
    ) -> Option<Operand> {
        self.fallback_object(id, bound, active, CompactFallbackCause::SignFields)
    }
}

/// Closed version-0 type for model sorts that carry enough information.
///
/// Composite model sorts do not retain the component type version 0 requires,
/// so they fail closed rather than borrowing an unrelated atom.
#[requires(true)]
#[ensures(true)]
fn sort_type_expr(sort: SemanticSort) -> Option<TypeExpr> {
    let atom = match sort {
        SemanticSort::Entity => TypeAtom::Entity,
        SemanticSort::Eventuality(EventualitySort::General) => TypeAtom::Eventuality,
        SemanticSort::Eventuality(EventualitySort::State) => TypeAtom::State,
        SemanticSort::Eventuality(EventualitySort::Process) => TypeAtom::Process,
        SemanticSort::Eventuality(EventualitySort::Activity) => TypeAtom::Activity,
        SemanticSort::Eventuality(EventualitySort::Achievement) => TypeAtom::Achievement,
        SemanticSort::Eventuality(EventualitySort::Experience) => TypeAtom::Experience,
        SemanticSort::Eventuality(EventualitySort::Locution) => TypeAtom::Locution,
        SemanticSort::TruthValue => TypeAtom::TruthValue,
        SemanticSort::Epistemology => TypeAtom::Epistemology,
        SemanticSort::Proposition => TypeAtom::Proposition,
        SemanticSort::Concept => TypeAtom::Concept,
        SemanticSort::Amount => TypeAtom::Amount,
        SemanticSort::Number => TypeAtom::Number,
        SemanticSort::Scale => TypeAtom::Scale,
        SemanticSort::Text => TypeAtom::Text,
        SemanticSort::AbstractNature => TypeAtom::AbstractNature,
        SemanticSort::Mass
        | SemanticSort::Set
        | SemanticSort::Sequence
        | SemanticSort::Time
        | SemanticSort::Predication
        | SemanticSort::Quantity
        | SemanticSort::Sign
        | SemanticSort::Relation
        | SemanticSort::Place
        | SemanticSort::Connective
        | SemanticSort::TenseModal
        | SemanticSort::MathOperator
        | SemanticSort::ArgumentBundle => return None,
    };
    Some(TypeExpr::Atom(atom))
}

/// Number-neutral reference type, when the component sort is closed.
#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|value| matches!(value, TypeExpr::Referents(_))))]
fn referents_type_expr(sort: SemanticSort) -> Option<TypeExpr> {
    sort_type_expr(sort).map(|value| TypeExpr::Referents(Box::new(value)))
}

/// Notation-level type of a shared graph value. Formulae and open predicate
/// terms are not referents, so they must never inherit an unrelated fallback
/// semantic sort merely because `SemanticObject::sort` is intentionally absent.
#[requires(true)]
#[ensures(true)]
fn definition_type_expr(object: &crate::model::SemanticObject) -> Option<TypeExpr> {
    match object.as_data() {
        data!(SemanticObject::Formula(_)) => Some(TypeExpr::Atom(TypeAtom::Content)),
        data!(SemanticObject::MathExpression(_)) => sort_type_expr(object.sort()?),
        data!(SemanticObject::Quantity(node))
            if node.form == QuantityForm::Exact
                && node.scale == QuantityScale::Count
                && node.value.question_parameters.is_empty()
                && node.comparison_set.is_none()
                && node.value.integer.is_some() =>
        {
            Some(TypeExpr::Atom(
                if node.value.integer.is_some_and(|value| value >= 0) {
                    TypeAtom::Natural
                } else {
                    TypeAtom::Number
                },
            ))
        }
        _ => None,
    }
}

/// The type a *use* of a shared identity is spelled at.
///
/// This is deliberately wider than [`definition_type_expr`]: whether the
/// notation can spell a declaration of an identity is decided once, where that
/// declaration is placed, and refusing every use of it as well would attribute
/// one failure to every object that mentions it. A reference or eventuality is
/// used at its own number-neutral reference type even where no declaration of
/// it can be written.
#[requires(true)]
#[ensures(true)]
fn definition_use_type(object: &crate::model::SemanticObject) -> Option<TypeExpr> {
    definition_type_expr(object).or_else(|| match object.as_data() {
        data!(SemanticObject::Referent(_)) | data!(SemanticObject::Eventuality(_)) => {
            referents_type_expr(object.sort()?)
        }
        _ => None,
    })
}

/// Spell one use of a live binder at exactly the type its binder declared.
#[requires(true)]
#[ensures(true)]
fn bound_use(id: SemanticObjectId, declaration: &BoundValue) -> Operand {
    let variable = object_variable(id);
    match declaration {
        BoundValue::Value(declared_type) => {
            Operand::Value(Value::bound(variable, declared_type.clone()))
        }
        BoundValue::Predicate(row) => Operand::Predicate(PredTerm::bound(variable, row.clone())),
    }
}

/// Lift content into the joined operand category.
#[requires(true)]
#[ensures(true)]
fn operand_content(content: Content) -> Elaborated {
    Elaborated::Operand(Operand::Content(content))
}

/// Lift a first-order value into the joined operand category.
#[requires(true)]
#[ensures(true)]
fn operand_value(value: Operand) -> Elaborated {
    Elaborated::Operand(value)
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
/// outside that constructor. Name descriptions have no support component at
/// all, but they are description values on the same terms: the renderer owns
/// their binder, so no graph definition site does.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.0.iter().all(|id| graph.objects.contains_key(id)))]
#[ensures(ret.1.iter().all(|id| graph.objects.contains_key(id)))]
fn projected_description_objects(
    graph: &SemanticGraph,
    usage: &GraphUsage,
) -> (BTreeSet<SemanticObjectId>, BTreeSet<SemanticObjectId>) {
    let mut projected = BTreeSet::new();
    let mut descriptions = BTreeSet::new();
    for (described, object) in &graph.objects {
        let Some(node) = object.as_referent() else {
            continue;
        };
        // Which dependence policy a description records is not a pre-planning
        // question, exactly as for a described event: a dependent description is
        // representable — its `Bind` stands inside the binders its property
        // names — and whether the recorded universe *is* the derived one at that
        // lexical position is decided by the route, which knows the position.
        // Recognition already proves that some policy is recorded.
        let Some(recognition) = recognize_description(graph, usage, *described, node) else {
            continue;
        };
        let mut support = BTreeSet::new();
        match recognition.as_data() {
            data!(DescriptionRecognition::Property {
                constructor,
                property: _,
                arguments: _,
                parameter: _,
            }) => {
                let descriptor = node
                    .descriptor
                    .as_ref()
                    .expect("recognized property has a descriptor");
                let body = descriptor
                    .body
                    .expect("recognized property has a descriptor body");
                match *constructor {
                    DescriptionConstructor::Lo => {
                        collect_property_support(graph, usage, body, *described, &mut support);
                    }
                    DescriptionConstructor::Le => {
                        collect_speaker_description_support(
                            graph,
                            usage,
                            body,
                            *described,
                            &mut support,
                        );
                    }
                }
                for clause in descriptor
                    .relative_clauses
                    .iter()
                    .chain(node.relative_clauses.iter())
                {
                    collect_inline_support(graph, usage, clause.body, *described, &mut support);
                }
            }
            // A recognized name description has no descriptor body and no
            // relative clauses at all, so its exact `Named` projection consumes
            // no other graph identity: the support component is empty and the
            // value is admitted unconditionally by the check below. The
            // description value itself still belongs in `descriptions` so that
            // the renderer hosts its `Refer` binding exactly like `lo`/`le`,
            // and so that the planner drops the definition-site failure for a
            // value that description inversion turns into a lexical binder.
            data!(DescriptionRecognition::Name { name: _ }) => {}
        }
        let allowed_sources = support
            .iter()
            .copied()
            .chain([*described])
            .collect::<BTreeSet<_>>();
        if support.iter().all(|id| {
            usage
                .uses_of(*id)
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
    usage: &GraphUsage,
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
        // Which dependence policy a described event records is not a
        // pre-planning question: a dependent abstraction is representable —
        // its `Bind` simply stands inside the binders its property names — and
        // whether the recorded universe *is* the derived one is decided at the
        // lexical position, by the route. What planning needs here is only that
        // some policy is recorded at all, which the base shape already proves.
        let crossing_is_exact = exact_event_abstraction_crossing(graph, node).is_some();
        if !(described_eventuality_base_is_exact(node, descriptor, kind) || crossing_is_exact)
            || !descriptor
                .speaker
                .is_some_and(|speaker| object_is_indexical(graph, speaker, IndexicalKind::Speaker))
            || (node.time.is_some() && exact_described_event_time_facet(node).is_none())
        {
            continue;
        }
        let mut support = BTreeSet::new();
        collect_inline_support(graph, usage, content, *event, &mut support);
        let allowed_sources = support
            .iter()
            .copied()
            .chain([*event])
            .collect::<BTreeSet<_>>();
        if support.iter().all(|id| {
            usage
                .uses_of(*id)
                .is_none_or(|uses| uses.is_subset(&allowed_sources))
        }) {
            projected.extend(support);
            values.insert(*event);
        }
    }
    (projected, values)
}

/// Collect an inline subgraph without crossing its owning self-reference, a
/// canonical atom that is safe to repeat by identity, or a binder.
///
/// A binder is not something the projection *consumes*: it is a free binder of
/// the property being built, declared by whatever introduces it, and no
/// declaration of it is ever written. Collecting one would make an ordinary
/// mention outside — the quantifier that binds it, a sibling place filled by the
/// same variable — look like an escape of the inlined subgraph and refuse the
/// whole projection. This is section D5's stop-set distinction at the pre-scan:
/// what a value contains is not what it depends on.
#[requires(graph.objects.contains_key(&root))]
#[ensures(out.contains(&root)
    || root == owner
    || usage.is_binder(root)
    || is_conventional_atom(&graph.objects[&root]))]
fn collect_inline_support(
    graph: &SemanticGraph,
    usage: &GraphUsage,
    root: SemanticObjectId,
    owner: SemanticObjectId,
    out: &mut BTreeSet<SemanticObjectId>,
) {
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        if id == owner
            || usage.is_binder(id)
            || is_conventional_atom(&graph.objects[&id])
            || !out.insert(id)
        {
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
    usage: &GraphUsage,
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
            .filter(|value| {
                *value != subject
                    && !usage.is_binder(*value)
                    && !is_conventional_atom(&graph.objects[value])
            }),
    );
}

/// Collect both layers of the exact recursive `skicu` description encoding.
#[requires(graph.objects.contains_key(&formula))]
#[ensures(out.contains(&formula))]
fn collect_speaker_description_support(
    graph: &SemanticGraph,
    usage: &GraphUsage,
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
                    && !usage.is_binder(*value)
                    && !is_conventional_atom(&graph.objects[value])
            }),
    );
    let Some(relation_node) = graph.objects[&relation_value].as_referent() else {
        return;
    };
    if let (Some(body), Some(parameter)) = (relation_node.body, relation_node.parameters.first()) {
        collect_property_support(graph, usage, body, *parameter, out);
    }
}

/// Exact default allowing an utterance record to collapse to its act.
#[requires(graph.objects.contains_key(&id))]
#[ensures(true)]
fn utterance_record_is_default(
    graph: &SemanticGraph,
    usage: &GraphUsage,
    id: SemanticObjectId,
    node: &UtteranceNode,
) -> bool {
    !utterance_identity_is_observed(graph, usage, id)
        && object_is_indexical(graph, node.speaker, IndexicalKind::Speaker)
        && object_is_indexical(graph, node.audience, IndexicalKind::Audience)
        && object_is_indexical(graph, node.deictic_ground.time, IndexicalKind::Now)
        && object_is_indexical(graph, node.deictic_ground.place, IndexicalKind::Here)
        && default_locution_event(graph, node.eventuality)
        && usage.use_count(node.eventuality) == 1
        && node.asides.is_empty()
        && node.vocative_kind.is_none()
}

/// A sole sequence-item containment edge selects a performing document child;
/// it does not observe the utterance token identity. Any other edge, repeated
/// item occurrence, quotation containment, or ordinal target keeps the record.
#[requires(graph.objects.contains_key(&id))]
#[ensures(!ret || usage.use_count(id) > 0)]
fn utterance_identity_is_observed(
    graph: &SemanticGraph,
    usage: &GraphUsage,
    id: SemanticObjectId,
) -> bool {
    match usage.use_count(id) {
        0 => false,
        1 => {
            let Some(sources) = usage.uses_of(id) else {
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
fn exact_deictic(node: &ReferentNode, graph: &SemanticGraph) -> Option<Intrinsic> {
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
        DeicticProximity::Proximal => Intrinsic::This,
        DeicticProximity::Medial => Intrinsic::That,
        DeicticProximity::Distal => Intrinsic::Yonder,
    })
}

/// Referent fields absent from the indexical/deictic role itself, allowing the
/// name assignments that role may carry as provenance.
///
/// An indexical or deictic atom has no binder for a handle to name, and it needs
/// none: the atom prints by identity wherever it occurs, so `mi goi ko'a` makes
/// every `ko'a` print `Speaker` with nothing left to state. See
/// [`assigned_names_are_resolution_provenance`].
#[requires(true)]
#[ensures(ret == (assigned_names_are_resolution_provenance(node)
    && referent_payload_except_names_is_empty(node)))]
fn referent_payload_is_empty(node: &ReferentNode) -> bool {
    assigned_names_are_resolution_provenance(node) && referent_payload_except_names_is_empty(node)
}

/// Whether every name assignment on a referent is the resolution's provenance.
///
/// Section 8.4 lowers a `goi` assignment to "`Let`, `Bind`, or the represented
/// naming or association predicate, depending on its graph semantics". When the
/// handle is a pro-sumti or lerfu-string stand-in, the graph has already made
/// that choice: the builder resolves every use of the handle to the object the
/// assignment was written on (jbotci#779), so whatever prints that object — a
/// quantifier's λ, a description's hosted `Bind`, or a canonical atom — *is* the
/// assignment, and the record is the provenance of a resolution that already
/// happened. Nothing is dropped by not printing it, because no other object
/// carries the handle.
///
/// The builder writes exactly three assignment shapes, and only those two are
/// stand-ins: an assignable `KOhA` and a lerfu string each record the handle as
/// both `word` and `name`, while `X goi la djan` records the name-marking word
/// and the name separately. That third shape states something about the
/// referent — section 8.4's naming predicate — which no resolution edge
/// carries, so it is not provenance and keeps its registered refusal.
///
/// The marker audit is defence rather than discrimination: every builder path
/// that writes an `AssignedName` is gated on `goi` today, and the other `GOI`
/// members attach an association with content of its own through a relative
/// clause instead. Checking it here keeps a future marker from silently
/// inheriting a suppression that was only ever true of assignment.
#[requires(true)]
#[ensures(ret || !node.assigned_names.is_empty())]
fn assigned_names_are_resolution_provenance(node: &ReferentNode) -> bool {
    node.assigned_names
        .iter()
        .all(|assigned| assigned.introduced_by == "goi" && assigned.word == assigned.name)
}

/// The referent audit of [`referent_payload_is_empty`] without name
/// assignments, for the positions that carry them as provenance.
#[requires(true)]
#[ensures(true)]
fn referent_payload_except_names_is_empty(node: &ReferentNode) -> bool {
    node.descriptor.is_none()
        && node.composition.is_none()
        && node.personal_mass_membership.is_none()
        && node.generated_referent.is_none()
        && node.relative_clauses.is_empty()
        && node.body.is_none()
        && node.parameters.is_empty()
        && node.arity.is_none()
        && node.embedded_questions.is_empty()
        && node.abstraction_kind.is_none()
        && node.abstracted.is_none()
        && node.experiencer.is_none()
        && node.epistemology.is_none()
        && node.expressed_by.is_none()
        && node.mind.is_none()
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
    usage: &GraphUsage,
    owner: SemanticObjectId,
    event: SemanticObjectId,
) -> bool {
    let Some(node) = graph.objects[&event].as_eventuality() else {
        return false;
    };
    generated_event_is_default_shape(node)
        && graph.objects[&event].diagnostics().is_empty()
        && usage.binder_owner(event) == Some(owner)
        // A default event has exactly the closure-owner binding edge and the
        // predication's eventuality edge. Additional edges make its identity
        // observable even when they originate in the same object.
        && usage.use_count(event) == 2
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
        && node.epistemology.is_none()
        && node.expressed_by.is_none()
        && node.mind.is_none()
        && node.target.is_none()
        && node.scale.is_none()
        && node.subscript.is_none()
}

/// Description referent has no content outside its descriptor/attached clauses.
///
/// A `goi` handle assigned to the description is provenance on the same terms
/// as one assigned to a bound variable: section 8.4's menu selects `Bind` here,
/// and the description's own hosted `Refer` *is* that `Bind`, so every use of
/// the handle already prints as the name it introduced.
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
        && assigned_names_are_resolution_provenance(node)
        && node.body.is_none()
        && node.parameters.is_empty()
        && node.arity.is_none()
        && node.embedded_questions.is_empty()
        && node.abstraction_kind.is_none()
        && node.abstracted.is_none()
        && node.experiencer.is_none()
        && node.epistemology.is_none()
        && node.expressed_by.is_none()
        && node.mind.is_none()
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
        && node.epistemology.is_none()
        && node.expressed_by.is_none()
        && node.mind.is_none()
        && node.target.is_none()
        && node.scale.is_none()
        && node.subscript.is_none()
        && node.arity
            == (node.abstraction_kind == Some(AbstractionKind::Property))
                .then_some(node.parameters.len())
}

/// The exact `lo` descriptor a gadri-folded abstraction carries.
///
/// The builder writes `lo ka ce'u broda` as one object holding both the
/// descriptor and the abstraction payload rather than as a description over a
/// separate abstraction referent, so the descriptor here has no body of its own
/// and every other descriptor coordinate must be absent: a quantity, a scale, a
/// definiteness, a relative clause, or a non-veridical marking would each be a
/// second semantic commitment that the plain crossing does not carry.
#[requires(true)]
#[ensures(true)]
fn folded_abstraction_descriptor_is_exact(
    graph: &SemanticGraph,
    descriptor: &crate::model::Descriptor,
) -> bool {
    descriptor.kind == DescriptorKind::VeridicalDescription
        && descriptor.word == "lo"
        && descriptor
            .speaker
            .is_some_and(|speaker| object_is_indexical(graph, speaker, IndexicalKind::Speaker))
        && descriptor.body.is_none()
        && descriptor.veridical.is_none()
        && descriptor.relative_clauses.is_empty()
        && descriptor.quantity.is_none()
        && descriptor.name.is_none()
        && descriptor.scale.is_none()
        && descriptor.definiteness.is_none()
        && descriptor.operand.is_none()
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

/// Exact two-operand referent abstraction and its section-11.3 declaration.
///
/// The optional trailing identity is kept optional here because omission is
/// precisely the contextual sugar this route owns. Every unrelated coordinate
/// is rejected, so a missing non-defaultable semantic operand cannot be
/// mistaken for that sugar.
#[requires(true)]
#[ensures(ret.is_none_or(|(intrinsic, _, _)| matches!(
    intrinsic,
    Intrinsic::Measure | Intrinsic::TruthValue | Intrinsic::Concept | Intrinsic::Abstract
)))]
fn exact_referent_abstraction_crossing(
    node: &ReferentNode,
    kind: AbstractionKind,
) -> Option<(Intrinsic, Option<SemanticObjectId>, SemanticSort)> {
    let (intrinsic, trailing, default_sort) = match kind {
        AbstractionKind::Amount => (Intrinsic::Measure, node.scale, SemanticSort::Scale),
        AbstractionKind::TruthValue => (
            Intrinsic::TruthValue,
            node.epistemology,
            SemanticSort::Epistemology,
        ),
        AbstractionKind::Concept => (Intrinsic::Concept, node.mind, SemanticSort::Entity),
        AbstractionKind::Unspecified => (Intrinsic::Abstract, node.target, SemanticSort::Entity),
        _ => return None,
    };
    let trailing_fields_are_exact = match kind {
        AbstractionKind::Amount => {
            node.epistemology.is_none() && node.mind.is_none() && node.target.is_none()
        }
        AbstractionKind::TruthValue => {
            node.scale.is_none() && node.mind.is_none() && node.target.is_none()
        }
        AbstractionKind::Concept => {
            node.scale.is_none() && node.epistemology.is_none() && node.target.is_none()
        }
        AbstractionKind::Unspecified => {
            node.scale.is_none() && node.epistemology.is_none() && node.mind.is_none()
        }
        _ => unreachable!("the crossing family was selected above"),
    };
    (node.category == ReferentCategory::Constant
        && node.scope_dependence.is_some()
        && node.sort == kind.output_sort()
        && node.indexical.is_none()
        && node.deictic_reference.is_none()
        && node.composition.is_none()
        && node.personal_mass_membership.is_none()
        && node.generated_referent.is_none()
        && node.relative_clauses.is_empty()
        && node.assigned_names.is_empty()
        && node.body.is_some()
        && node.parameters.is_empty()
        && node.arity.is_none()
        && node.embedded_questions.is_empty()
        && node.abstraction_kind == Some(kind)
        && node.abstracted.is_none()
        && node.experiencer.is_none()
        && node.expressed_by.is_none()
        && node.subscript.is_none()
        && trailing_fields_are_exact)
        .then_some((intrinsic, trailing, default_sort))
}

/// Exact two-place event abstraction and its explicit trailing operand.
///
/// Section 11.2 keeps the corresponding one-place graph as an ordinary
/// described event. Consequently this recognizer requires the independent x2
/// object; it never manufactures a crossing merely from the abstraction kind.
#[requires(graph.objects.contains_key(&graph.root))]
#[ensures(ret.is_none_or(|(_, intrinsic, _, _)| matches!(
    intrinsic,
    Intrinsic::ExperienceOf | Intrinsic::ProcessOf | Intrinsic::ActivityOf
)))]
fn exact_event_abstraction_crossing(
    graph: &SemanticGraph,
    node: &EventualityNode,
) -> Option<(SemanticObjectId, Intrinsic, SemanticObjectId, SemanticSort)> {
    let descriptor = node.descriptor.as_ref()?;
    let kind = node.abstraction_kind?;
    let (expected_sort, expected_class, intrinsic, trailing, default_sort) = match kind {
        AbstractionKind::Experience => (
            EventualitySort::Experience,
            crate::model::EventualityClass::Event,
            Intrinsic::ExperienceOf,
            node.experiencer?,
            SemanticSort::Entity,
        ),
        AbstractionKind::Process => (
            EventualitySort::Process,
            crate::model::EventualityClass::Process,
            Intrinsic::ProcessOf,
            node.stages?,
            SemanticSort::eventuality(),
        ),
        AbstractionKind::Activity => (
            EventualitySort::Activity,
            crate::model::EventualityClass::Activity,
            Intrinsic::ActivityOf,
            node.actions?,
            SemanticSort::eventuality(),
        ),
        _ => return None,
    };
    let trailing_fields_are_exact = match kind {
        AbstractionKind::Experience => node.stages.is_none() && node.actions.is_none(),
        AbstractionKind::Process => node.experiencer.is_none() && node.actions.is_none(),
        AbstractionKind::Activity => node.experiencer.is_none() && node.stages.is_none(),
        _ => unreachable!("the crossing family was selected above"),
    };
    (node.denotation.category() == Some(ReferentCategory::Constant)
        && node.denotation.scope_dependence().is_some()
        && node.sort == expected_sort
        && node.class == Some(expected_class)
        && node.indexical.is_none()
        && descriptor.kind == DescriptorKind::VeridicalDescription
        && descriptor.word == "lo"
        && descriptor
            .speaker
            .is_some_and(|speaker| object_is_indexical(graph, speaker, IndexicalKind::Speaker))
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
        && node.content.is_some()
        && node.body.is_none()
        && node.parameters.is_empty()
        && node.arity.is_none()
        && node.embedded_questions.is_empty()
        && node.abstraction_kind == Some(kind)
        && node.target.is_none()
        && node.scale.is_none()
        && node.subscript.is_none()
        && trailing_fields_are_exact)
        .then_some((node.content?, intrinsic, trailing, default_sort))
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
        && node.stages.is_none()
        && node.actions.is_none()
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
        && node.stages.is_none()
        && node.actions.is_none()
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

/// Shared exact recognition for `lo`, `le`, and `la`. Every descriptor and
/// attachment coordinate is audited here once.
///
/// Whether the recorded dependence policy is the default *at a lexical
/// position* is deliberately not asked here: that is a question about a
/// position, so only the route that has one can answer it, and the pre-scan
/// that runs before any position exists would otherwise have to refuse every
/// dependent description outright.
#[requires(graph.objects.contains_key(&described))]
#[ensures(true)]
fn recognize_description<'a>(
    graph: &'a SemanticGraph,
    usage: &GraphUsage,
    described: SemanticObjectId,
    node: &'a ReferentNode,
) -> Option<DescriptionRecognition<'a>> {
    let descriptor = node.descriptor.as_ref()?;
    if !referent_except_descriptor_is_default(node)
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
    let (constructor, property, arguments, parameter) =
        match (descriptor.kind, descriptor.word.as_str()) {
            (DescriptorKind::VeridicalDescription, "lo") => {
                let (property, arguments) =
                    recognize_direct_description_property(graph, usage, body, described)?;
                (DescriptionConstructor::Lo, property, arguments, None)
            }
            (DescriptorKind::SpeakerDescription, "le") => {
                let (property, arguments, parameter) =
                    recognize_speaker_description_property(graph, usage, body, described)?;
                (
                    DescriptionConstructor::Le,
                    property,
                    arguments,
                    Some(parameter),
                )
            }
            _ => return None,
        };
    Some(new!(DescriptionRecognition::Property {
        constructor,
        property,
        arguments,
        parameter,
    }))
}

/// Exact direct `lo` property encoding: a restrictive atom whose x1 is the
/// described referent and whose remaining places are default contextual values.
#[requires(graph.objects.contains_key(&formula))]
#[ensures(true)]
fn recognize_direct_description_property<'a>(
    graph: &'a SemanticGraph,
    usage: &GraphUsage,
    formula: SemanticObjectId,
    described: SemanticObjectId,
) -> Option<(&'a str, &'a BTreeMap<PlaceIndex, ArgumentValue>)> {
    recognize_property_formula(graph, usage, formula, described)
}

/// Exact recursive `skicu` encoding for `le`, including its separately stored
/// unary property abstraction.
#[requires(graph.objects.contains_key(&formula))]
#[ensures(true)]
fn recognize_speaker_description_property<'a>(
    graph: &'a SemanticGraph,
    usage: &GraphUsage,
    formula: SemanticObjectId,
    described: SemanticObjectId,
) -> Option<(
    &'a str,
    &'a BTreeMap<PlaceIndex, ArgumentValue>,
    SemanticObjectId,
)> {
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
    recognize_property_formula(graph, usage, relation_node.body?, parameter)
        .map(|(property, arguments)| (property, arguments, parameter))
}

/// Exact unary property atom used by both recognized description encodings.
#[requires(graph.objects.contains_key(&formula))]
#[ensures(true)]
fn recognize_property_formula<'a>(
    graph: &'a SemanticGraph,
    usage: &GraphUsage,
    formula: SemanticObjectId,
    subject: SemanticObjectId,
) -> Option<(&'a str, &'a BTreeMap<PlaceIndex, ArgumentValue>)> {
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
        if argument.kind == ArgumentValueKind::Filled {
            let value = plain_argument_value(&predication.arguments, place.get())?;
            if !is_conventional_atom(&graph.objects[&value]) {
                return None;
            }
            continue;
        }
        let value = plain_elided_argument_value(argument)?;
        let referent = graph.objects[&value].as_referent()?;
        // An interior `zo'e` records whichever binders the graph says its value
        // may be resolved against, and any such record is representable: it
        // projects as the section-5.1 `Context` its dependence names, bound
        // inside this property, and section 6.3 then hosts the description at
        // the innermost position where its property's free binders — those
        // dependences included — are all live. What this recognizer owes is
        // therefore only that a policy is recorded at all; the site the policy
        // is default at is the route's question, and an unsupplied binder is a
        // hosting failure with its own registered reason rather than a reason to
        // refuse the description shape here.
        if !default_elided_shape(referent)
            || referent.scope_dependence.is_none()
            || !graph.objects[&value].diagnostics().is_empty()
            || usage.use_count(value) != 1
        {
            return None;
        }
    }
    LexicalRoot::try_new(relation).ok()?;
    Some((relation, &predication.arguments))
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
    usage: &GraphUsage,
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
        || !generated_event_is_default(graph, usage, formula, event)
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
            || usage.use_count(value) != 1
            || referent
                .scope_dependence
                .as_ref()
                .and_then(|dependence| dependence.may_depend_on())
                != Some(&BTreeSet::from([subject]))
        {
            return None;
        }
    }
    LexicalRoot::try_new(relation).ok()?;
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
///
/// A `goi` handle assigned to the variable is one of those provenance
/// coordinates rather than a second commitment: `ro lo prenu goi ko'a` resolves
/// every `ko'a` to this very variable, so the quantifier's own λ is the
/// assignment section 8.4 asks for and the recorded handle names the binder
/// that is already printed. See
/// [`assigned_names_are_resolution_provenance`] for the assignment shapes that
/// qualify.
#[requires(graph.objects.contains_key(&variable))]
#[ensures(ret.is_none_or(|sort| graph.objects[&variable].sort() == Some(sort)))]
fn exact_plain_bound_variable(
    graph: &SemanticGraph,
    variable: SemanticObjectId,
) -> Option<SemanticSort> {
    let node = graph.objects[&variable].as_referent()?;
    (node.category == ReferentCategory::Variable
        && node.scope_dependence.is_none()
        && referent_payload_except_names_is_empty(node)
        && assigned_names_are_resolution_provenance(node)
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
) -> Option<(ConnectiveForm, Vec<SemanticObjectId>)> {
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
                return Some((
                    ConnectiveForm::Binary(BinaryOp::Implies),
                    vec![negation.children[0], node.children[1]],
                ));
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
        (FormulaOperator::Not, 1) => ConnectiveForm::Not,
        (FormulaOperator::And, 2..) => ConnectiveForm::Junction(JunctionOp::And),
        (FormulaOperator::Or, 2..) => ConnectiveForm::Junction(JunctionOp::Or),
        (FormulaOperator::Implies, 2) => ConnectiveForm::Binary(BinaryOp::Implies),
        (FormulaOperator::Iff, 2) => ConnectiveForm::Binary(BinaryOp::Equivalent),
        (FormulaOperator::ExclusiveOr, 2) => ConnectiveForm::Binary(BinaryOp::ExclusiveOr),
        _ => return None,
    };
    Some((operator, node.children.clone()))
}

/// The kernel form one recognized connective builds.
///
/// The three shapes are kept apart because their arities are: negation takes
/// one operand, a junction two or more at one locus, and a truth-functional
/// binary connective exactly two.
#[invariant(::Not => true, "negation takes one operand; the arity is checked where it applies")]
#[invariant(::Junction(_) => true, "every junction operator is legal; operand arity is a Content invariant")]
#[invariant(::Binary(_) => true, "every binary connective is legal; operand arity is checked where it applies")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectiveForm {
    Not,
    Junction(JunctionOp),
    Binary(BinaryOp),
}

impl ConnectiveForm {
    /// Combine the recognized operands at this connective's locus.
    #[requires(true)]
    #[ensures(true)]
    fn apply(self, operands: Vec<Content>) -> Option<Content> {
        match self {
            Self::Not => {
                let [inner] = <[Content; 1]>::try_from(operands).ok()?;
                Some(Content::not(inner))
            }
            Self::Junction(operator) => Content::junction(operator, operands).ok(),
            Self::Binary(operator) => {
                let [left, right] = <[Content; 2]>::try_from(operands).ok()?;
                Some(Content::binary(operator, left, right))
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use bityzba::requires;

    use super::*;
    use crate::notation::kernel::content::ContentData;
    use crate::notation::kernel::value::RefCompData;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_declaration_group_binds_in_its_hosted_value_s_own_category() {
        let declaration = Declaration::new(
            Variable::try_new("$first").expect("a valid variable"),
            entity_referents(),
            registered_constant(Intrinsic::This).expect("This is a registered constant"),
        )
        .expect("the initializer inhabits its declared type");
        let body = Content::intrinsic(
            Intrinsic::Named,
            vec![
                Operand::Value(Value::literal(Literal::text("alis"))),
                Operand::Value(Value::bound(
                    Variable::try_new("$first").expect("a valid variable"),
                    entity_referents(),
                )),
            ],
        )
        .expect("Named relates a text to a reference");

        let hosted = HostedGroup::Inert(vec![declaration.clone()])
            .wrap(Elaborated::Operand(Operand::Content(body.clone())))
            .expect("content has a binding form of its own");
        let Elaborated::Operand(Operand::Content(hosted)) = hosted else {
            panic!("a content body stays content when a group binds over it");
        };
        assert!(matches!(hosted.as_data(), data!(Content::Let(_))));

        // A query, act, discourse, or transcript entry has no binding form, so
        // a group planned at one of those positions has nowhere to stand.
        assert!(
            HostedGroup::Inert(vec![declaration])
                .wrap(Elaborated::Operand(Operand::Act(Act::assert(body))))
                .is_none(),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn model_event_subtypes_use_closed_v0_atoms_and_erased_composites_fail() {
        for (sort, expected) in [
            (SemanticSort::eventuality(), TypeAtom::Eventuality),
            (
                SemanticSort::Eventuality(EventualitySort::State),
                TypeAtom::State,
            ),
            (
                SemanticSort::Eventuality(EventualitySort::Process),
                TypeAtom::Process,
            ),
            (
                SemanticSort::Eventuality(EventualitySort::Activity),
                TypeAtom::Activity,
            ),
            (
                SemanticSort::Eventuality(EventualitySort::Achievement),
                TypeAtom::Achievement,
            ),
            (
                SemanticSort::Eventuality(EventualitySort::Experience),
                TypeAtom::Experience,
            ),
            (
                SemanticSort::Eventuality(EventualitySort::Locution),
                TypeAtom::Locution,
            ),
        ] {
            assert_eq!(sort_type_expr(sort), Some(TypeExpr::Atom(expected)));
        }
        for erased in [
            SemanticSort::Mass,
            SemanticSort::Set,
            SemanticSort::Sequence,
            SemanticSort::Sign,
            SemanticSort::Relation,
        ] {
            assert_eq!(sort_type_expr(erased), None);
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn universal_quantification_refuses_an_impure_restriction_once_at_its_operand() {
        let variable = Variable::try_new("$candidate").expect("a valid variable");
        let parameter = TypedParameter::new(variable.clone(), entity_referents());
        let candidate = Operand::Value(Value::bound(variable, entity_referents()));
        let pure = Content::intrinsic(Intrinsic::Among, vec![candidate.clone(), candidate])
            .expect("Among produces content over two entity references");
        let impure = Content::presuppose(pure.clone(), pure.clone());

        assert!(matches!(
            generalized_quantification(
                Intrinsic::Every,
                &parameter,
                Some(impure.clone()),
                Some(pure.clone()),
            ),
            GeneralizedQuantification::PurityUnproven
        ));
        assert!(matches!(
            generalized_quantification(Intrinsic::Some, &parameter, Some(impure), Some(pure),),
            GeneralizedQuantification::Rendered(_)
        ));
    }

    /// Compose one failed projection edge without needing a whole graph.
    #[requires(index > 0)]
    #[ensures(ret.owner.index() == index && ret.cause == cause)]
    fn edge(index: usize, cause: CompactFallbackCause) -> CompactFallback {
        CompactFallback {
            owner: SemanticObjectId::predication(index),
            cause,
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn re_entering_one_failed_edge_records_it_once() {
        let log = CompactFallbackLog::default();
        let repeated = edge(1, CompactFallbackCause::ArgumentFields);
        log.record(repeated);
        log.record(repeated);
        log.record(repeated);
        assert_eq!(log.into_ordered(), vec![repeated]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn two_boundaries_of_one_owner_are_two_failed_edges() {
        // Keying by owner alone kept whichever boundary declined first and
        // dropped the other, so this is the law that regression must not lose.
        let log = CompactFallbackLog::default();
        let arguments = edge(1, CompactFallbackCause::ArgumentFields);
        let relation = edge(1, CompactFallbackCause::NonAtomicRelation);
        log.record(relation);
        log.record(arguments);
        log.record(relation);
        let recorded = log.into_ordered();
        assert_eq!(recorded.len(), 2, "{recorded:?}");
        assert!(
            recorded
                .iter()
                .all(|failure| failure.owner == arguments.owner)
        );
        assert_eq!(
            recorded
                .iter()
                .map(|failure| failure.cause)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                CompactFallbackCause::ArgumentFields,
                CompactFallbackCause::NonAtomicRelation,
            ]),
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn two_owners_declining_for_one_reason_are_two_failed_edges() {
        let log = CompactFallbackLog::default();
        let first = edge(1, CompactFallbackCause::SignFields);
        let second = edge(2, CompactFallbackCause::SignFields);
        log.record(second);
        log.record(first);
        let recorded = log.into_ordered();
        assert_eq!(recorded, vec![first, second]);
        assert_eq!(first.cause.reason_id(), second.cause.reason_id());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn the_failed_edge_channel_is_ordered_by_owner_then_boundary() {
        let log = CompactFallbackLog::default();
        // Recorded in an order that is neither owner order nor boundary order.
        let edges = [
            edge(2, CompactFallbackCause::ArgumentFields),
            edge(1, CompactFallbackCause::SignFields),
            edge(2, CompactFallbackCause::SignFields),
            edge(1, CompactFallbackCause::ArgumentFields),
        ];
        for failure in edges {
            log.record(failure);
        }
        let recorded = log.into_ordered();
        let mut expected = edges;
        expected.sort();
        assert_eq!(recorded, expected.to_vec());
        assert!(recorded.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_boundary_has_one_stable_code_and_message_at_every_occurrence() {
        // The code and message are functions of the typed boundary alone, so
        // two occurrences of one boundary cannot drift apart, and a boundary
        // that shares a registered reason with another still says which
        // boundary was actually reached.
        let first = edge(1, CompactFallbackCause::SignFields);
        let second = edge(2, CompactFallbackCause::SignFields);
        assert_eq!(first.cause.reason_id(), second.cause.reason_id());
        assert_eq!(first.cause.message(), second.cause.message());

        let shared_reason = [
            CompactFallbackCause::MathSideFields,
            CompactFallbackCause::MathLiteralDenotes,
            CompactFallbackCause::MathOperatorFields,
        ];
        assert_eq!(
            shared_reason
                .iter()
                .map(|cause| cause.reason_id())
                .collect::<BTreeSet<_>>()
                .len(),
            1,
            "these boundaries deliberately share one registered reason",
        );
        assert_eq!(
            shared_reason
                .iter()
                .map(|cause| cause.message())
                .collect::<BTreeSet<_>>()
                .len(),
            shared_reason.len(),
            "a shared reason must not erase which boundary declined",
        );
        for cause in shared_reason {
            assert!(cause.reason_id().starts_with("smusni.projection."));
            assert!(!cause.message().is_empty());
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_contextual_constant_is_a_computation_rather_than_a_value() {
        // Section 3.5 types `Context` as a `RefComp`, so it inhabits a `Bind`
        // initializer and nothing else. A fixed context carries no
        // dependencies; an underspecified one carries exactly the permitted
        // ones, which is what section 5.1 refuses to let `Close` hide.
        let fixed = RefComp::context(Vec::new(), entity_referents())
            .expect("a contextual computation selects a reference");
        assert!(matches!(fixed.as_data(), data!(RefComp::Context { .. })));

        let dependencies = [
            SemanticObjectId::referent(7),
            SemanticObjectId::referent(11),
        ];
        let underspecified = RefComp::context(
            dependencies.iter().map(|id| object_variable(*id)).collect(),
            entity_referents(),
        )
        .expect("a contextual computation selects a reference");
        let data!(RefComp::Context {
            dependencies: named,
            ..
        }) = underspecified.as_data()
        else {
            panic!("a contextual computation is a Context");
        };
        assert_eq!(
            named,
            &dependencies
                .iter()
                .map(|id| object_variable(*id))
                .collect::<Vec<_>>(),
        );

        // The declared reference type is the computation's result, so a
        // computation that produced something else could not be bound at it.
        assert!(
            Bind::new(
                object_variable(SemanticObjectId::referent(3)),
                TypeExpr::Atom(TypeAtom::Entity),
                fixed,
                Content::bound(Variable::try_new("$body").expect("a valid variable")),
            )
            .is_err(),
        );
    }
}

/// Fixed indexical constant table.
#[requires(true)]
#[ensures(true)]
fn indexical_constant(indexical: IndexicalKind) -> Intrinsic {
    match indexical {
        IndexicalKind::Speaker => Intrinsic::Speaker,
        IndexicalKind::Audience => Intrinsic::Audience,
        IndexicalKind::Now => Intrinsic::Now,
        IndexicalKind::Here => Intrinsic::Here,
    }
}

/// Apply one nullary registered constant.
#[requires(true)]
#[ensures(true)]
fn registered_constant(intrinsic: Intrinsic) -> Option<Operand> {
    Value::intrinsic(intrinsic, Vec::new())
        .ok()
        .map(Operand::Value)
}
