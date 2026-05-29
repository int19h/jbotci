//! Borrowed semantic reference overlay for syntax trees.

use std::collections::{HashMap, HashSet};
use std::num::NonZeroU8;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, requires};
use jbotci_morphology::{Cmavo, Selmaho, WordLike};
use jbotci_source::{SourceId, SourceSpan};
use jbotci_syntax::ast::{
    AbstractionSyntax, ArgumentSyntax, ArgumentSyntaxData, ArgumentTagSyntax,
    ArgumentTagSyntaxData, ArgumentTailElementSyntax, ArgumentTailElementSyntaxData,
    AtomRef as SyntaxAtomRef, BeiLinkSyntax, CompositeTenseModalPartSyntaxData, ConnectiveSyntax,
    ConnectiveSyntaxData, DescriptorSyntax, FragmentSyntax, FragmentSyntaxData, FreeModifierSyntax,
    FreeModifierSyntaxData, GoiRelativeClauseSyntax, MathExpressionSyntax,
    MathExpressionSyntaxData, MathOperatorSyntax, MathOperatorSyntaxData, NodeRef as SyntaxNodeRef,
    ParagraphSyntax, PredicateSyntax, PredicateTail1Syntax, PredicateTail2Syntax,
    PredicateTail3Syntax, PredicateTail3SyntaxData, PredicateTailSyntax, QuantifierSyntax,
    QuantifierSyntaxData, QuoteSyntax, QuoteSyntaxData, RelationSyntax, RelationSyntaxData,
    RelationUnitSyntax, RelationUnitSyntaxData, RelativeClauseSyntax, RelativeClauseSyntaxData,
    StatementSyntax, StatementSyntaxData, SubsentenceSyntax, SubsentenceSyntaxData,
    TenseModalSyntax, TenseModalSyntaxData, TermSyntax, TermSyntaxData, TextSyntax, Token,
    TreeNode, WithFreeModifiers,
};
use jbotci_tree::TreeVisitor;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct RawSyntaxNodeId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct TextNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct ParagraphNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct StatementNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct PredicateNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct PredicateTailNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct RelationNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct RelationUnitNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct TermNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct ArgumentNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct FreeModifierNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct AbstractionNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct MathExpressionNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct MathOperatorNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct SyntaxNodeMetadata {
    pub id: RawSyntaxNodeId,
    pub parent: Option<RawSyntaxNodeId>,
    pub preorder: usize,
    pub depth: usize,
    pub leaf_start: usize,
    pub leaf_end: usize,
    pub source_spans: Vec<SourceSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct SelbriPlaceFrameId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct ArgumentPlaceAssignmentId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct ReferenceEdgeId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
#[invariant(true)]
#[invariant(::Numbered(_) => true)]
#[invariant(::Modal(_) => true)]
pub enum PlaceSlot {
    Numbered(NonZeroU8),
    Modal(Option<RawSyntaxNodeId>),
    Fai,
}

impl PlaceSlot {
    #[requires(place > 0)]
    #[ensures(ret.is_some())]
    pub fn numbered(place: u8) -> Option<Self> {
        NonZeroU8::new(place).map(PlaceSlot::Numbered)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn numbered_index(self) -> Option<u8> {
        match self {
            PlaceSlot::Numbered(place) => Some(place.get()),
            PlaceSlot::Modal(_) | PlaceSlot::Fai => None,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn numbered_slot(place: NonZeroU8) -> PlaceSlot {
    PlaceSlot::Numbered(place)
}

#[requires(true)]
#[ensures(true)]
fn modal_slot(tag: Option<RawSyntaxNodeId>) -> PlaceSlot {
    PlaceSlot::Modal(tag)
}

#[requires(true)]
#[ensures(true)]
fn fai_slot() -> PlaceSlot {
    PlaceSlot::Fai
}

#[requires(true)]
#[ensures(true)]
fn propagation_none() -> PlaceFramePropagation {
    PlaceFramePropagation::None
}

#[requires(true)]
#[ensures(true)]
fn propagation_forward(inner: SelbriPlaceFrameId) -> PlaceFramePropagation {
    PlaceFramePropagation::Forward { inner }
}

#[requires(true)]
#[ensures(true)]
fn propagation_conversion(
    inner: SelbriPlaceFrameId,
    converted_place: NonZeroU8,
) -> PlaceFramePropagation {
    PlaceFramePropagation::Conversion {
        inner,
        converted_place,
    }
}

#[requires(true)]
#[ensures(true)]
fn propagation_jai(inner: SelbriPlaceFrameId) -> PlaceFramePropagation {
    PlaceFramePropagation::Jai { inner }
}

#[requires(true)]
#[ensures(true)]
fn propagation_connected(branches: Vec<SelbriPlaceFrameId>) -> PlaceFramePropagation {
    PlaceFramePropagation::Connected { branches }
}

#[requires(true)]
#[ensures(true)]
fn propagation_compound(
    head: SelbriPlaceFrameId,
    modifiers: Vec<SelbriPlaceFrameId>,
) -> PlaceFramePropagation {
    PlaceFramePropagation::Compound { head, modifiers }
}

#[requires(true)]
#[ensures(true)]
fn propagation_co(
    leading: SelbriPlaceFrameId,
    trailing: SelbriPlaceFrameId,
) -> PlaceFramePropagation {
    PlaceFramePropagation::Co { leading, trailing }
}

#[requires(true)]
#[ensures(true)]
fn target_resolved_node(node: RawSyntaxNodeId) -> ReferenceTarget {
    ReferenceTarget::ResolvedNode(node)
}

#[requires(true)]
#[ensures(true)]
fn target_resolved_frame(frame: SelbriPlaceFrameId) -> ReferenceTarget {
    ReferenceTarget::ResolvedFrame(frame)
}

#[requires(!reason.is_empty())]
#[ensures(true)]
fn target_unresolved(reason: &str) -> ReferenceTarget {
    ReferenceTarget::Unresolved(reason.to_owned())
}

#[requires(true)]
#[ensures(true)]
fn target_vague(kind: VagueReferenceKind) -> ReferenceTarget {
    ReferenceTarget::Vague(kind)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum PlaceFrameKind {
    Predicate,
    PredicateTail,
    BaseRelation,
    RelationUnit,
    Converted,
    JaiConverted,
    LinkedUnit,
    Connected,
    Compound,
    CoInverted,
    Forwarding,
    Abstraction,
    ProRelation,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[invariant(true)]
#[invariant(::Forward => true)]
#[invariant(::Conversion => true)]
#[invariant(::Jai => true)]
#[invariant(::Connected => true)]
#[invariant(::Compound => true)]
#[invariant(::Co => true)]
pub enum PlaceFramePropagation {
    None,
    Forward {
        inner: SelbriPlaceFrameId,
    },
    Conversion {
        inner: SelbriPlaceFrameId,
        converted_place: NonZeroU8,
    },
    Jai {
        inner: SelbriPlaceFrameId,
    },
    Connected {
        branches: Vec<SelbriPlaceFrameId>,
    },
    Compound {
        head: SelbriPlaceFrameId,
        modifiers: Vec<SelbriPlaceFrameId>,
    },
    Co {
        leading: SelbriPlaceFrameId,
        trailing: SelbriPlaceFrameId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct SelbriPlaceFrame {
    pub id: SelbriPlaceFrameId,
    pub node: RawSyntaxNodeId,
    pub kind: PlaceFrameKind,
    pub relation: Option<RelationNodeId>,
    pub relation_unit: Option<RelationUnitNodeId>,
    pub propagation: PlaceFramePropagation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum AssignmentSource {
    SequentialTerm,
    FaTerm,
    ModalTerm,
    LinkArgument,
    TermsetBranch,
    Propagated,
    CompoundSharedX1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct ArgumentPlaceAssignment {
    pub id: ArgumentPlaceAssignmentId,
    pub frame: SelbriPlaceFrameId,
    pub slot: PlaceSlot,
    pub argument: ArgumentNodeId,
    pub term: Option<TermNodeId>,
    pub source: AssignmentSource,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum ReferenceKind {
    GoiAssignment,
    CeiAssignment,
    Koha,
    Ri,
    Cehu,
    Letter,
    Ra,
    Ru,
    Keha,
    VohaSeries,
    DaSeries,
    BrodaSeries,
    GohaSeries,
    Utterance,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum VagueReferenceKind {
    DistantArgument,
    RecentArgument,
    Bridi,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
#[invariant(true)]
#[invariant(::ResolvedNode(_) => true)]
#[invariant(::ResolvedFrame(_) => true)]
#[invariant(::AmbiguousNodes(_) => true)]
#[invariant(::Unresolved(_) => true)]
#[invariant(::Vague(_) => true)]
pub enum ReferenceTarget {
    ResolvedNode(RawSyntaxNodeId),
    ResolvedFrame(SelbriPlaceFrameId),
    AmbiguousNodes(Vec<RawSyntaxNodeId>),
    Unresolved(String),
    Vague(VagueReferenceKind),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct ReferenceEdge {
    pub id: ReferenceEdgeId,
    pub kind: ReferenceKind,
    pub source: RawSyntaxNodeId,
    pub target: ReferenceTarget,
    pub rule: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
pub enum ReferenceAnalysisError {
    #[error("syntax index did not contain the root text node")]
    MissingRootNode,
}

#[derive(Debug)]
#[invariant(true)]
pub struct ReferenceAnalysis<'tree> {
    pub syntax_index: SyntaxIndex<'tree>,
    pub place_analysis: PlaceAnalysis,
    pub discourse_references: DiscourseReferences,
}

impl<'tree> ReferenceAnalysis<'tree> {
    #[requires(true)]
    #[ensures(true)]
    pub fn analyze(syntax: &'tree TextSyntax) -> Result<Self, ReferenceAnalysisError> {
        analyze_references(syntax)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn v0_compatibility_projection(&self) -> V0CompatibilityProjection {
        V0CompatibilityProjection::from_analysis(self)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn fixture_projection(&self) -> ReferenceFixtureProjection {
        ReferenceFixtureProjection::from_analysis(self)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()))]
    pub fn fixture_projection_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.fixture_projection())
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|analysis| analysis.syntax_index.node_count() > 0))]
pub fn analyze_references<'tree>(
    syntax: &'tree TextSyntax,
) -> Result<ReferenceAnalysis<'tree>, ReferenceAnalysisError> {
    let syntax_index = SyntaxIndex::new(syntax)?;
    let place_analysis = PlaceAnalysis::analyze(&syntax_index, syntax);
    let discourse_references = DiscourseReferences::analyze(&syntax_index, &place_analysis, syntax);
    Ok(ReferenceAnalysis {
        syntax_index,
        place_analysis,
        discourse_references,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct PlaceAnalysis {
    frames: Vec<SelbriPlaceFrame>,
    frame_ids_by_node: HashMap<RawSyntaxNodeId, Vec<SelbriPlaceFrameId>>,
    assignments: Vec<ArgumentPlaceAssignment>,
    assignment_ids_by_argument: HashMap<ArgumentNodeId, Vec<ArgumentPlaceAssignmentId>>,
    assignment_ids_by_term: HashMap<TermNodeId, Vec<ArgumentPlaceAssignmentId>>,
    assignment_ids_by_frame: HashMap<SelbriPlaceFrameId, Vec<ArgumentPlaceAssignmentId>>,
    assignment_ids_by_frame_slot:
        HashMap<(SelbriPlaceFrameId, PlaceSlot), Vec<ArgumentPlaceAssignmentId>>,
}

impl PlaceAnalysis {
    #[requires(true)]
    #[ensures(true)]
    fn analyze<'tree>(index: &SyntaxIndex<'tree>, syntax: &'tree TextSyntax) -> Self {
        let mut builder = PlaceAnalysisBuilder::new(index);
        builder.analyze_text(syntax);
        builder.finish()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn frames(&self) -> &[SelbriPlaceFrame] {
        &self.frames
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn frame(&self, id: SelbriPlaceFrameId) -> Option<&SelbriPlaceFrame> {
        self.frames.get(id.0)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn frames_for_node(&self, node: RawSyntaxNodeId) -> &[SelbriPlaceFrameId] {
        self.frame_ids_by_node
            .get(&node)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn assignments(&self) -> &[ArgumentPlaceAssignment] {
        &self.assignments
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn assignment(&self, id: ArgumentPlaceAssignmentId) -> Option<&ArgumentPlaceAssignment> {
        self.assignments.get(id.0)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn assignments_for_argument(
        &self,
        argument: ArgumentNodeId,
    ) -> &[ArgumentPlaceAssignmentId] {
        self.assignment_ids_by_argument
            .get(&argument)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn assignments_for_term(&self, term: TermNodeId) -> &[ArgumentPlaceAssignmentId] {
        self.assignment_ids_by_term
            .get(&term)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn assignments_for_frame(&self, frame: SelbriPlaceFrameId) -> &[ArgumentPlaceAssignmentId] {
        self.assignment_ids_by_frame
            .get(&frame)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn assignments_for_frame_slot(
        &self,
        frame: SelbriPlaceFrameId,
        slot: PlaceSlot,
    ) -> &[ArgumentPlaceAssignmentId] {
        self.assignment_ids_by_frame_slot
            .get(&(frame, slot))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn first_argument_for_place(
        &self,
        frame: SelbriPlaceFrameId,
        slot: PlaceSlot,
    ) -> Option<ArgumentNodeId> {
        self.assignments_for_frame_slot(frame, slot)
            .first()
            .and_then(|id| self.assignment(*id))
            .map(|assignment| assignment.argument)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct DiscourseReferences {
    edges: Vec<ReferenceEdge>,
    edge_ids_by_source: HashMap<RawSyntaxNodeId, Vec<ReferenceEdgeId>>,
    edge_ids_by_target_node: HashMap<RawSyntaxNodeId, Vec<ReferenceEdgeId>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct SyntaxSpanKey {
    pub source_id: Option<SourceId>,
    pub byte_start: usize,
    pub byte_end: usize,
    pub char_start: usize,
    pub char_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct V0CompatibilityProjection {
    pub argument_assignments: Vec<V0ArgumentAssignment>,
    pub relation_places: Vec<V0RelationPlace>,
    pub reference_edges: Vec<V0ReferenceEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct V0ArgumentAssignment {
    pub argument: SyntaxSpanKey,
    pub relation: SyntaxSpanKey,
    pub slot: PlaceSlot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct V0RelationPlace {
    pub relation: SyntaxSpanKey,
    pub place: NonZeroU8,
    pub argument: SyntaxSpanKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct V0ReferenceEdge {
    pub source: SyntaxSpanKey,
    pub target: Option<SyntaxSpanKey>,
    pub kind: ReferenceKind,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct FixtureSpanKey {
    pub offset: usize,
    pub length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct ReferenceFixtureProjection {
    pub frames: Vec<FixturePlaceFrame>,
    pub assignments: Vec<FixtureArgumentAssignment>,
    pub relation_places: Vec<FixtureRelationPlace>,
    pub references: Vec<FixtureReferenceEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct FixturePlaceFrame {
    pub index: usize,
    pub node: FixtureSpanKey,
    pub kind: PlaceFrameKind,
    pub relation: Option<FixtureSpanKey>,
    pub relation_unit: Option<FixtureSpanKey>,
    pub propagation: FixturePlaceFramePropagation,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[invariant(true)]
#[invariant(::Forward => true)]
#[invariant(::Conversion => true)]
#[invariant(::Jai => true)]
#[invariant(::Connected => true)]
#[invariant(::Compound => true)]
#[invariant(::Co => true)]
pub enum FixturePlaceFramePropagation {
    None,
    Forward { inner: usize },
    Conversion { inner: usize, converted_place: u8 },
    Jai { inner: usize },
    Connected { branches: Vec<usize> },
    Compound { head: usize, modifiers: Vec<usize> },
    Co { leading: usize, trailing: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[invariant(true)]
#[invariant(::Numbered => true)]
#[invariant(::Modal => true)]
pub enum FixturePlaceSlot {
    Numbered { place: u8 },
    Modal { tag: Option<FixtureSpanKey> },
    Fai,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct FixtureArgumentAssignment {
    pub frame: usize,
    pub frame_node: FixtureSpanKey,
    pub relation: Option<FixtureSpanKey>,
    pub relation_unit: Option<FixtureSpanKey>,
    pub slot: FixturePlaceSlot,
    pub argument: FixtureSpanKey,
    pub term: Option<FixtureSpanKey>,
    pub source: AssignmentSource,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct FixtureRelationPlace {
    pub frame: usize,
    pub relation: FixtureSpanKey,
    pub place: u8,
    pub argument: FixtureSpanKey,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct FixtureReferenceEdge {
    pub kind: ReferenceKind,
    pub source: FixtureSpanKey,
    pub target: FixtureReferenceTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[invariant(true)]
#[invariant(::ResolvedNode => true)]
#[invariant(::ResolvedFrame => true)]
#[invariant(::AmbiguousNodes => true)]
#[invariant(::Unresolved => true)]
#[invariant(::Vague => true)]
pub enum FixtureReferenceTarget {
    ResolvedNode {
        node: FixtureSpanKey,
    },
    ResolvedFrame {
        frame: usize,
        frame_node: FixtureSpanKey,
    },
    AmbiguousNodes {
        nodes: Vec<FixtureSpanKey>,
    },
    Unresolved {
        reason: String,
    },
    Vague {
        vague_kind: VagueReferenceKind,
    },
}

impl ReferenceFixtureProjection {
    #[requires(true)]
    #[ensures(true)]
    pub fn from_analysis(analysis: &ReferenceAnalysis<'_>) -> Self {
        let mut frames = analysis
            .place_analysis
            .frames()
            .iter()
            .filter_map(|frame| fixture_frame(analysis, frame))
            .collect::<Vec<_>>();
        frames.sort();

        let mut assignments = analysis
            .place_analysis
            .assignments()
            .iter()
            .filter_map(|assignment| fixture_assignment(analysis, assignment))
            .collect::<Vec<_>>();
        assignments.sort();

        let mut relation_places = analysis
            .place_analysis
            .assignments()
            .iter()
            .filter_map(|assignment| fixture_relation_place(analysis, assignment))
            .collect::<Vec<_>>();
        relation_places.sort();
        relation_places.dedup();

        let mut references = analysis
            .discourse_references
            .edges()
            .iter()
            .filter_map(|edge| fixture_reference_edge(analysis, edge))
            .collect::<Vec<_>>();
        references.sort();

        Self {
            frames,
            assignments,
            relation_places,
            references,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn fixture_frame(
    analysis: &ReferenceAnalysis<'_>,
    frame: &SelbriPlaceFrame,
) -> Option<FixturePlaceFrame> {
    Some(FixturePlaceFrame {
        index: frame.id.0,
        node: fixture_span_key_for_node(&analysis.syntax_index, frame.node)?,
        kind: frame.kind,
        relation: frame
            .relation
            .and_then(|relation| fixture_span_key_for_node(&analysis.syntax_index, relation.0)),
        relation_unit: frame.relation_unit.and_then(|relation_unit| {
            fixture_span_key_for_node(&analysis.syntax_index, relation_unit.0)
        }),
        propagation: fixture_frame_propagation(&frame.propagation),
    })
}

#[requires(true)]
#[ensures(true)]
fn fixture_frame_propagation(propagation: &PlaceFramePropagation) -> FixturePlaceFramePropagation {
    match propagation {
        PlaceFramePropagation::None => FixturePlaceFramePropagation::None,
        PlaceFramePropagation::Forward { inner } => {
            FixturePlaceFramePropagation::Forward { inner: inner.0 }
        }
        PlaceFramePropagation::Conversion {
            inner,
            converted_place,
        } => FixturePlaceFramePropagation::Conversion {
            inner: inner.0,
            converted_place: converted_place.get(),
        },
        PlaceFramePropagation::Jai { inner } => {
            FixturePlaceFramePropagation::Jai { inner: inner.0 }
        }
        PlaceFramePropagation::Connected { branches } => FixturePlaceFramePropagation::Connected {
            branches: branches.iter().map(|branch| branch.0).collect(),
        },
        PlaceFramePropagation::Compound { head, modifiers } => {
            FixturePlaceFramePropagation::Compound {
                head: head.0,
                modifiers: modifiers.iter().map(|modifier| modifier.0).collect(),
            }
        }
        PlaceFramePropagation::Co { leading, trailing } => FixturePlaceFramePropagation::Co {
            leading: leading.0,
            trailing: trailing.0,
        },
    }
}

#[requires(true)]
#[ensures(true)]
fn fixture_assignment(
    analysis: &ReferenceAnalysis<'_>,
    assignment: &ArgumentPlaceAssignment,
) -> Option<FixtureArgumentAssignment> {
    let frame = analysis.place_analysis.frame(assignment.frame)?;
    Some(FixtureArgumentAssignment {
        frame: assignment.frame.0,
        frame_node: fixture_span_key_for_node(&analysis.syntax_index, frame.node)?,
        relation: frame
            .relation
            .and_then(|relation| fixture_span_key_for_node(&analysis.syntax_index, relation.0)),
        relation_unit: frame.relation_unit.and_then(|relation_unit| {
            fixture_span_key_for_node(&analysis.syntax_index, relation_unit.0)
        }),
        slot: fixture_place_slot(&analysis.syntax_index, assignment.slot),
        argument: fixture_span_key_for_node(&analysis.syntax_index, assignment.argument.0)?,
        term: assignment
            .term
            .and_then(|term| fixture_span_key_for_node(&analysis.syntax_index, term.0)),
        source: assignment.source,
    })
}

#[requires(true)]
#[ensures(true)]
fn fixture_relation_place(
    analysis: &ReferenceAnalysis<'_>,
    assignment: &ArgumentPlaceAssignment,
) -> Option<FixtureRelationPlace> {
    let PlaceSlot::Numbered(place) = assignment.slot else {
        return None;
    };
    let frame = analysis.place_analysis.frame(assignment.frame)?;
    let relation = frame
        .relation
        .map(|relation| relation.0)
        .unwrap_or(frame.node);
    Some(FixtureRelationPlace {
        frame: assignment.frame.0,
        relation: fixture_span_key_for_node(&analysis.syntax_index, relation)?,
        place: place.get(),
        argument: fixture_span_key_for_node(&analysis.syntax_index, assignment.argument.0)?,
    })
}

#[requires(true)]
#[ensures(true)]
fn fixture_reference_edge(
    analysis: &ReferenceAnalysis<'_>,
    edge: &ReferenceEdge,
) -> Option<FixtureReferenceEdge> {
    Some(FixtureReferenceEdge {
        kind: edge.kind.clone(),
        source: fixture_span_key_for_node(&analysis.syntax_index, edge.source)?,
        target: fixture_reference_target(analysis, &edge.target)?,
    })
}

#[requires(true)]
#[ensures(true)]
fn fixture_reference_target(
    analysis: &ReferenceAnalysis<'_>,
    target: &ReferenceTarget,
) -> Option<FixtureReferenceTarget> {
    match target {
        ReferenceTarget::ResolvedNode(node) => Some(FixtureReferenceTarget::ResolvedNode {
            node: fixture_span_key_for_node(&analysis.syntax_index, *node)?,
        }),
        ReferenceTarget::ResolvedFrame(frame) => {
            let frame_data = analysis.place_analysis.frame(*frame)?;
            Some(FixtureReferenceTarget::ResolvedFrame {
                frame: frame.0,
                frame_node: fixture_span_key_for_node(&analysis.syntax_index, frame_data.node)?,
            })
        }
        ReferenceTarget::AmbiguousNodes(nodes) => {
            let mut projected = nodes
                .iter()
                .filter_map(|node| fixture_span_key_for_node(&analysis.syntax_index, *node))
                .collect::<Vec<_>>();
            projected.sort();
            Some(FixtureReferenceTarget::AmbiguousNodes { nodes: projected })
        }
        ReferenceTarget::Unresolved(reason) => Some(FixtureReferenceTarget::Unresolved {
            reason: reason.clone(),
        }),
        ReferenceTarget::Vague(kind) => Some(FixtureReferenceTarget::Vague {
            vague_kind: kind.clone(),
        }),
    }
}

#[requires(true)]
#[ensures(true)]
fn fixture_place_slot(index: &SyntaxIndex<'_>, slot: PlaceSlot) -> FixturePlaceSlot {
    match slot {
        PlaceSlot::Numbered(place) => FixturePlaceSlot::Numbered { place: place.get() },
        PlaceSlot::Modal(tag) => FixturePlaceSlot::Modal {
            tag: tag.and_then(|node| fixture_span_key_for_node(index, node)),
        },
        PlaceSlot::Fai => FixturePlaceSlot::Fai,
    }
}

impl V0CompatibilityProjection {
    #[requires(true)]
    #[ensures(true)]
    fn from_analysis(analysis: &ReferenceAnalysis<'_>) -> Self {
        let mut argument_assignments = Vec::new();
        let mut relation_places = Vec::new();
        for assignment in analysis.place_analysis.assignments() {
            let Some(frame) = analysis.place_analysis.frame(assignment.frame) else {
                continue;
            };
            let Some(relation_key) = frame
                .relation
                .map(|relation| relation.0)
                .or(Some(frame.node))
                .and_then(|node| span_key_for_node(&analysis.syntax_index, node))
            else {
                continue;
            };
            let Some(argument_key) =
                span_key_for_node(&analysis.syntax_index, assignment.argument.0)
            else {
                continue;
            };
            argument_assignments.push(V0ArgumentAssignment {
                argument: argument_key.clone(),
                relation: relation_key.clone(),
                slot: assignment.slot,
            });
            if let PlaceSlot::Numbered(place) = assignment.slot {
                relation_places.push(V0RelationPlace {
                    relation: relation_key,
                    place,
                    argument: argument_key,
                });
            }
        }

        let reference_edges = analysis
            .discourse_references
            .edges()
            .iter()
            .filter_map(|edge| {
                let source = span_key_for_node(&analysis.syntax_index, edge.source)?;
                let target = match &edge.target {
                    ReferenceTarget::ResolvedNode(node) => {
                        span_key_for_node(&analysis.syntax_index, *node)
                    }
                    _ => None,
                };
                Some(V0ReferenceEdge {
                    source,
                    target,
                    kind: edge.kind.clone(),
                })
            })
            .collect();

        V0CompatibilityProjection {
            argument_assignments,
            relation_places,
            reference_edges,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn span_key_for_node(index: &SyntaxIndex<'_>, node: RawSyntaxNodeId) -> Option<SyntaxSpanKey> {
    let metadata = index.metadata(node)?;
    let first = metadata.source_spans.first()?;
    let last = metadata.source_spans.last()?;
    Some(SyntaxSpanKey {
        source_id: first.source_id.clone(),
        byte_start: first.byte_start,
        byte_end: last.byte_end,
        char_start: first.char_start,
        char_end: last.char_end,
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| key.length > 0))]
fn fixture_span_key_for_node(
    index: &SyntaxIndex<'_>,
    node: RawSyntaxNodeId,
) -> Option<FixtureSpanKey> {
    let key = span_key_for_node(index, node)?;
    Some(FixtureSpanKey {
        offset: key.byte_start,
        length: key.byte_end.saturating_sub(key.byte_start),
    })
}

impl DiscourseReferences {
    #[requires(true)]
    #[ensures(true)]
    fn analyze<'tree>(
        index: &SyntaxIndex<'tree>,
        places: &PlaceAnalysis,
        syntax: &'tree TextSyntax,
    ) -> Self {
        let mut builder = DiscourseReferenceBuilder::new(index, places);
        builder.visit_text(syntax);
        builder.finish()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn edges(&self) -> &[ReferenceEdge] {
        &self.edges
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn edge(&self, id: ReferenceEdgeId) -> Option<&ReferenceEdge> {
        self.edges.get(id.0)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn references_from_node(&self, node: RawSyntaxNodeId) -> &[ReferenceEdgeId] {
        self.edge_ids_by_source
            .get(&node)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn references_to_node(&self, node: RawSyntaxNodeId) -> &[ReferenceEdgeId] {
        self.edge_ids_by_target_node
            .get(&node)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

#[derive(Debug)]
#[invariant(true)]
struct PlaceAnalysisBuilder<'index, 'tree> {
    index: &'index SyntaxIndex<'tree>,
    frames: Vec<SelbriPlaceFrame>,
    frame_ids_by_node: HashMap<RawSyntaxNodeId, Vec<SelbriPlaceFrameId>>,
    assignments: Vec<ArgumentPlaceAssignment>,
    assignment_ids_by_argument: HashMap<ArgumentNodeId, Vec<ArgumentPlaceAssignmentId>>,
    assignment_ids_by_term: HashMap<TermNodeId, Vec<ArgumentPlaceAssignmentId>>,
    assignment_ids_by_frame: HashMap<SelbriPlaceFrameId, Vec<ArgumentPlaceAssignmentId>>,
    assignment_ids_by_frame_slot:
        HashMap<(SelbriPlaceFrameId, PlaceSlot), Vec<ArgumentPlaceAssignmentId>>,
}

impl<'index, 'tree> PlaceAnalysisBuilder<'index, 'tree> {
    #[requires(true)]
    #[ensures(ret.frames.is_empty())]
    fn new(index: &'index SyntaxIndex<'tree>) -> Self {
        Self {
            index,
            frames: Vec::new(),
            frame_ids_by_node: HashMap::new(),
            assignments: Vec::new(),
            assignment_ids_by_argument: HashMap::new(),
            assignment_ids_by_term: HashMap::new(),
            assignment_ids_by_frame: HashMap::new(),
            assignment_ids_by_frame_slot: HashMap::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> PlaceAnalysis {
        PlaceAnalysis {
            frames: self.frames,
            frame_ids_by_node: self.frame_ids_by_node,
            assignments: self.assignments,
            assignment_ids_by_argument: self.assignment_ids_by_argument,
            assignment_ids_by_term: self.assignment_ids_by_term,
            assignment_ids_by_frame: self.assignment_ids_by_frame,
            assignment_ids_by_frame_slot: self.assignment_ids_by_frame_slot,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_text(&mut self, text: &'tree TextSyntax) {
        self.analyze_free_modifiers_nested(&text.leading_free_modifiers);
        for paragraph in &text.paragraphs {
            self.analyze_paragraph(paragraph);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_paragraph(&mut self, paragraph: &'tree ParagraphSyntax) {
        self.analyze_free_modifiers_nested(&paragraph.free_modifiers);
        for statement in &paragraph.statements {
            self.analyze_free_modifiers_nested(&statement.free_modifiers);
            if let Some(statement) = statement.statement.as_deref() {
                self.analyze_statement(statement);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_statement(&mut self, statement: &'tree StatementSyntax) {
        match statement.as_data() {
            data!(StatementSyntax::Tuhe { text, .. }) => self.analyze_text(text),
            data!(StatementSyntax::Prenex {
                prenex_terms,
                inner_statement,
                ..
            }) => {
                self.analyze_terms_nested(prenex_terms);
                self.analyze_statement(inner_statement);
            }
            data!(StatementSyntax::Predicate(predicate)) => {
                self.analyze_predicate(predicate);
            }
            data!(StatementSyntax::Connected {
                leading_statement,
                trailing_statement,
                ..
            })
            | data!(StatementSyntax::PreIConnected {
                leading_statement,
                trailing_statement,
                ..
            }) => {
                self.analyze_statement(leading_statement);
                self.analyze_statement(trailing_statement);
            }
            data!(StatementSyntax::Iau {
                inner_statement,
                reset_terms,
                ..
            }) => {
                self.analyze_statement(inner_statement);
                self.analyze_terms_nested(reset_terms);
            }
            data!(StatementSyntax::ExperimentalPredicateContinuation {
                leading_statement,
                continuation,
            }) => {
                self.analyze_statement(leading_statement);
                self.analyze_subsentence(&continuation.trailing_subsentence);
            }
            data!(StatementSyntax::Fragment(fragment)) => {
                self.analyze_fragment(fragment);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_subsentence(&mut self, subsentence: &'tree SubsentenceSyntax) {
        match subsentence.as_data() {
            data!(SubsentenceSyntax::Plain(predicate)) => {
                self.analyze_predicate(predicate);
            }
            data!(SubsentenceSyntax::Prenex {
                prenex_terms,
                inner_subsentence,
                ..
            }) => {
                self.analyze_terms_nested(prenex_terms);
                self.analyze_subsentence(inner_subsentence);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_predicate(&mut self, predicate: &'tree PredicateSyntax) -> SelbriPlaceFrameId {
        self.analyze_predicate_with_initial_place(predicate, 1)
    }

    #[requires(initial_place > 0)]
    #[ensures(true)]
    fn analyze_predicate_with_initial_place(
        &mut self,
        predicate: &'tree PredicateSyntax,
        initial_place: u8,
    ) -> SelbriPlaceFrameId {
        let branch_initial_place =
            next_place_after_common_terms(initial_place, &predicate.leading_terms);
        let tail = self.analyze_predicate_tail(&predicate.predicate_tail, branch_initial_place);
        let predicate_raw = self.raw_for(SyntaxNodeRef::PredicateSyntax(predicate));
        let predicate_frame = self.add_frame(
            predicate_raw,
            PlaceFrameKind::Predicate,
            None,
            None,
            propagation_connected(tail.frames),
        );
        let mut cursors =
            vec![self.cursor_with_existing_assignments(predicate_frame, initial_place)];
        self.assign_terms(
            &mut cursors,
            &predicate.leading_terms,
            AssignmentSource::SequentialTerm,
        );
        for cursor in &mut cursors {
            cursor.ensure_next_place_at_least(2);
        }
        self.assign_term_refs(&mut cursors, &tail.terms, AssignmentSource::SequentialTerm);
        self.analyze_free_modifiers_nested(&predicate.free_modifiers);
        predicate_frame
    }

    #[requires(true)]
    #[ensures(true)]
    fn branch_tail_cursors(&self, frames: &[SelbriPlaceFrameId]) -> Vec<PlaceCursor> {
        frames
            .iter()
            .copied()
            .map(|frame| self.cursor_with_existing_assignments(frame, 2))
            .collect()
    }

    #[requires(start > 0)]
    #[ensures(ret.next_place >= start)]
    fn cursor_with_existing_assignments(
        &self,
        frame: SelbriPlaceFrameId,
        start: u8,
    ) -> PlaceCursor {
        let mut cursor = PlaceCursor::new_at(frame, start);
        for place in 1..=self.max_existing_numbered_place() {
            let slot = numbered_slot(NonZeroU8::new(place).expect("range starts at one"));
            if self.frame_slot_has_existing_assignment(frame, slot) {
                cursor.mark_filled_slot(slot);
            }
        }
        cursor
    }

    #[requires(true)]
    #[ensures(true)]
    fn max_existing_numbered_place(&self) -> u8 {
        self.assignments
            .iter()
            .filter_map(|assignment| assignment.slot.numbered_index())
            .max()
            .unwrap_or(0)
    }

    #[requires(true)]
    #[ensures(true)]
    fn frame_slot_has_existing_assignment(
        &self,
        frame: SelbriPlaceFrameId,
        slot: PlaceSlot,
    ) -> bool {
        let mut visited = HashSet::new();
        self.frame_slot_has_existing_assignment_recursive(frame, slot, &mut visited)
    }

    #[requires(true)]
    #[ensures(true)]
    fn frame_slot_has_existing_assignment_recursive(
        &self,
        frame: SelbriPlaceFrameId,
        slot: PlaceSlot,
        visited: &mut HashSet<(SelbriPlaceFrameId, PlaceSlot)>,
    ) -> bool {
        if self.frame_slot_has_blocking_assignment(frame, slot) {
            return true;
        }
        if !visited.insert((frame, slot)) {
            return false;
        }
        let Some(frame_data) = self.frames.get(frame.0) else {
            return false;
        };
        match &frame_data.propagation {
            PlaceFramePropagation::None => false,
            PlaceFramePropagation::Forward { inner } => {
                self.frame_slot_has_existing_assignment_recursive(*inner, slot, visited)
            }
            PlaceFramePropagation::Conversion {
                inner,
                converted_place,
            } => {
                let converted = convert_slot(slot, *converted_place);
                self.frame_slot_has_existing_assignment_recursive(*inner, converted, visited)
            }
            PlaceFramePropagation::Jai { inner } => match slot {
                PlaceSlot::Fai => self.frame_slot_has_existing_assignment_recursive(
                    *inner,
                    numbered_slot(NonZeroU8::new(1).expect("literal is non-zero")),
                    visited,
                ),
                PlaceSlot::Numbered(place) if place.get() > 1 => {
                    self.frame_slot_has_existing_assignment_recursive(*inner, slot, visited)
                }
                PlaceSlot::Numbered(_) | PlaceSlot::Modal(_) => false,
            },
            PlaceFramePropagation::Connected { branches } => branches.iter().any(|branch| {
                self.frame_slot_has_existing_assignment_recursive(*branch, slot, visited)
            }),
            PlaceFramePropagation::Compound { head, modifiers } => {
                self.frame_slot_has_existing_assignment_recursive(*head, slot, visited)
                    || (slot.numbered_index() == Some(1)
                        && modifiers.iter().any(|modifier| {
                            self.frame_slot_has_existing_assignment_recursive(
                                *modifier, slot, visited,
                            )
                        }))
            }
            PlaceFramePropagation::Co { leading, trailing } => {
                self.frame_slot_has_existing_assignment_recursive(*trailing, slot, visited)
                    || (slot.numbered_index() == Some(1)
                        && self
                            .frame_slot_has_existing_assignment_recursive(*leading, slot, visited))
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn frame_slot_has_blocking_assignment(
        &self,
        frame: SelbriPlaceFrameId,
        slot: PlaceSlot,
    ) -> bool {
        self.assignment_ids_by_frame_slot
            .get(&(frame, slot))
            .is_some_and(|assignments| {
                assignments
                    .iter()
                    .any(|assignment| self.assignment_blocks_cursor(*assignment))
            })
    }

    #[requires(true)]
    #[ensures(true)]
    fn assignment_blocks_cursor(&self, assignment: ArgumentPlaceAssignmentId) -> bool {
        let Some(assignment) = self.assignments.get(assignment.0) else {
            return false;
        };
        let Some(argument) = self.index.argument(assignment.argument) else {
            return false;
        };
        argument_koha_cmavo(argument) != Some(Cmavo::Cehu)
    }

    #[requires(true)]
    #[ensures(analysis.branch_cursors.is_none())]
    #[ensures(analysis.terms.is_empty())]
    fn consume_branch_tail_cursors(
        &mut self,
        analysis: &mut PredicateTailAnalysis<'tree>,
    ) -> Vec<PlaceCursor> {
        if let Some(cursors) = analysis.branch_cursors.take() {
            return cursors;
        }
        let mut cursors = self.branch_tail_cursors(&analysis.frames);
        self.assign_term_refs(
            &mut cursors,
            &analysis.terms,
            AssignmentSource::SequentialTerm,
        );
        analysis.terms.clear();
        cursors
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_predicate_tail(
        &mut self,
        tail: &'tree PredicateTailSyntax,
        gek_branch_initial_place: u8,
    ) -> PredicateTailAnalysis<'tree> {
        let first = self.analyze_predicate_tail1(&tail.first, gek_branch_initial_place);
        let mut branches = first.frames;
        let mut terms = first.terms;
        let mut branch_cursors = first.branch_cursors;
        if let Some(ke_continuation) = tail.ke_continuation.as_deref() {
            let mut first_branch_cursors = if let Some(cursors) = branch_cursors.take() {
                cursors
            } else {
                let mut cursors = self.branch_tail_cursors(&branches);
                self.assign_term_refs(&mut cursors, &terms, AssignmentSource::SequentialTerm);
                terms.clear();
                cursors
            };
            if let Some(tense_modal) = ke_continuation.tense_modal.as_deref() {
                self.analyze_tense_modal_nested(tense_modal);
            }
            let mut continuation = self
                .analyze_predicate_tail(&ke_continuation.predicate_tail, gek_branch_initial_place);
            let continuation_cursors = self.consume_branch_tail_cursors(&mut continuation);
            branches.extend(continuation.frames);
            first_branch_cursors.extend(continuation_cursors);
            self.assign_terms(
                &mut first_branch_cursors,
                &ke_continuation.tail_terms,
                AssignmentSource::SequentialTerm,
            );
            branch_cursors = Some(first_branch_cursors);
            self.analyze_free_modifiers_nested(&ke_continuation.free_modifiers);
        }
        let raw = self.raw_for(SyntaxNodeRef::PredicateTailSyntax(tail));
        let frame = self.add_frame(
            raw,
            PlaceFrameKind::PredicateTail,
            None,
            None,
            propagation_connected(branches),
        );
        PredicateTailAnalysis {
            frames: vec![frame],
            terms,
            branch_cursors,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_predicate_tail1(
        &mut self,
        tail: &'tree PredicateTail1Syntax,
        gek_branch_initial_place: u8,
    ) -> PredicateTailAnalysis<'tree> {
        let mut analysis = self.analyze_predicate_tail2(&tail.first, gek_branch_initial_place);
        let mut branch_cursors = if tail.continuations.is_empty() {
            analysis.branch_cursors.take()
        } else {
            Some(self.consume_branch_tail_cursors(&mut analysis))
        };
        for continuation in &tail.continuations {
            if let Some(tense_modal) = continuation.tense_modal.as_deref() {
                self.analyze_tense_modal_nested(tense_modal);
            }
            let mut next = self
                .analyze_predicate_tail2(&continuation.predicate_tail, gek_branch_initial_place);
            if let Some(cursors) = branch_cursors.as_mut() {
                let next_cursors = self.consume_branch_tail_cursors(&mut next);
                cursors.extend(next_cursors);
                self.assign_terms(
                    cursors,
                    &continuation.tail_terms,
                    AssignmentSource::SequentialTerm,
                );
            }
            analysis.frames.extend(next.frames);
            self.analyze_free_modifiers_nested(&continuation.free_modifiers);
        }
        let raw = self.raw_for(predicate_tail1_node_ref(tail));
        let frame = self.add_frame(
            raw,
            PlaceFrameKind::PredicateTail,
            None,
            None,
            propagation_connected(analysis.frames),
        );
        PredicateTailAnalysis {
            frames: vec![frame],
            terms: analysis.terms,
            branch_cursors,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_predicate_tail2(
        &mut self,
        tail: &'tree PredicateTail2Syntax,
        gek_branch_initial_place: u8,
    ) -> PredicateTailAnalysis<'tree> {
        let mut analysis = self.analyze_predicate_tail3(&tail.first, gek_branch_initial_place);
        let mut branch_cursors = analysis.branch_cursors.take();
        if let Some(continuation) = tail.bo_continuation.as_deref() {
            let mut active_cursors = if let Some(cursors) = branch_cursors.take() {
                cursors
            } else {
                self.consume_branch_tail_cursors(&mut analysis)
            };
            if let Some(tense_modal) = continuation.tense_modal.as_deref() {
                self.analyze_tense_modal_nested(tense_modal);
            }
            let mut next = self
                .analyze_predicate_tail2(&continuation.predicate_tail, gek_branch_initial_place);
            let next_cursors = self.consume_branch_tail_cursors(&mut next);
            analysis.frames.extend(next.frames);
            active_cursors.extend(next_cursors);
            self.assign_terms(
                &mut active_cursors,
                &continuation.tail_terms,
                AssignmentSource::SequentialTerm,
            );
            branch_cursors = Some(active_cursors);
            self.analyze_free_modifiers_nested(&continuation.free_modifiers);
        }
        let raw = self.raw_for(predicate_tail2_node_ref(tail));
        let frame = self.add_frame(
            raw,
            PlaceFrameKind::PredicateTail,
            None,
            None,
            propagation_connected(analysis.frames),
        );
        PredicateTailAnalysis {
            frames: vec![frame],
            terms: analysis.terms,
            branch_cursors,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_predicate_tail3(
        &mut self,
        tail: &'tree PredicateTail3Syntax,
        gek_branch_initial_place: u8,
    ) -> PredicateTailAnalysis<'tree> {
        match tail.as_data() {
            data!(PredicateTail3Syntax::Relation {
                relation,
                terms,
                free_modifiers,
                ..
            }) => {
                let relation_frame = self.analyze_relation(relation);
                self.analyze_free_modifiers_nested(free_modifiers);
                let raw = self.raw_for(predicate_tail3_node_ref(tail));
                let frame = self.add_frame(
                    raw,
                    PlaceFrameKind::PredicateTail,
                    None,
                    None,
                    propagation_forward(relation_frame),
                );
                PredicateTailAnalysis {
                    frames: vec![frame],
                    terms: terms.iter().collect(),
                    branch_cursors: None,
                }
            }
            data!(PredicateTail3Syntax::GekSentence(gek)) => {
                let frames = self.analyze_gek_sentence(gek, gek_branch_initial_place);
                PredicateTailAnalysis {
                    frames,
                    terms: Vec::new(),
                    branch_cursors: None,
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_gek_sentence(
        &mut self,
        gek: &'tree jbotci_syntax::ast::GekSentenceSyntax,
        branch_initial_place: u8,
    ) -> Vec<SelbriPlaceFrameId> {
        match gek.as_data() {
            data!(jbotci_syntax::ast::GekSentenceSyntax::Pair {
                first,
                second,
                tail_terms,
                free_modifiers,
                ..
            }) => {
                let first_frame =
                    self.analyze_subsentence_frame_with_initial_place(first, branch_initial_place);
                let second_frame =
                    self.analyze_subsentence_frame_with_initial_place(second, branch_initial_place);
                let mut cursors = vec![
                    self.cursor_with_existing_assignments(first_frame, branch_initial_place),
                    self.cursor_with_existing_assignments(second_frame, branch_initial_place),
                ];
                self.assign_terms(&mut cursors, tail_terms, AssignmentSource::SequentialTerm);
                self.analyze_free_modifiers_nested(free_modifiers);
                vec![first_frame, second_frame]
            }
            data!(jbotci_syntax::ast::GekSentenceSyntax::Ke {
                tense_modal,
                inner,
                ..
            }) => {
                if let Some(tense_modal) = tense_modal.as_deref() {
                    self.analyze_tense_modal_nested(tense_modal);
                }
                self.analyze_gek_sentence(inner, branch_initial_place)
            }
            data!(jbotci_syntax::ast::GekSentenceSyntax::Na { inner, .. }) => {
                self.analyze_gek_sentence(inner, branch_initial_place)
            }
        }
    }

    #[requires(initial_place > 0)]
    #[ensures(true)]
    fn analyze_subsentence_frame_with_initial_place(
        &mut self,
        subsentence: &'tree SubsentenceSyntax,
        initial_place: u8,
    ) -> SelbriPlaceFrameId {
        match subsentence.as_data() {
            data!(SubsentenceSyntax::Plain(predicate)) => {
                self.analyze_predicate_with_initial_place(predicate, initial_place)
            }
            data!(SubsentenceSyntax::Prenex {
                prenex_terms,
                inner_subsentence,
                ..
            }) => {
                self.analyze_terms_nested(prenex_terms);
                self.analyze_subsentence_frame_with_initial_place(inner_subsentence, initial_place)
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_relation(&mut self, relation: &'tree RelationSyntax) -> SelbriPlaceFrameId {
        let relation_id = self.index.relation_node_id(relation);
        let relation_raw = self.raw_for(relation_node_ref(relation));
        match relation.as_data() {
            data!(RelationSyntax::Base(..)) => self.add_frame(
                relation_raw,
                PlaceFrameKind::BaseRelation,
                relation_id,
                None,
                propagation_none(),
            ),
            data!(RelationSyntax::Se { se, inner_relation }) => {
                let inner = self.analyze_relation(inner_relation);
                let converted_place = se_conversion_place(se)
                    .and_then(NonZeroU8::new)
                    .unwrap_or(NonZeroU8::new(2).expect("literal is non-zero"));
                self.add_frame(
                    relation_raw,
                    PlaceFrameKind::Converted,
                    relation_id,
                    None,
                    propagation_conversion(inner, converted_place),
                )
            }
            data!(RelationSyntax::Na { inner_relation, .. }) => {
                let inner = self.analyze_relation(inner_relation);
                self.add_frame(
                    relation_raw,
                    PlaceFrameKind::Forwarding,
                    relation_id,
                    None,
                    propagation_forward(inner),
                )
            }
            data!(RelationSyntax::Ke {
                ke_tense_modal,
                relation: inner_relation,
                ..
            }) => {
                if let Some(tense_modal) = ke_tense_modal.as_deref() {
                    self.analyze_tense_modal_nested(tense_modal);
                }
                let inner = self.analyze_relation(inner_relation);
                self.add_frame(
                    relation_raw,
                    PlaceFrameKind::Forwarding,
                    relation_id,
                    None,
                    propagation_forward(inner),
                )
            }
            data!(RelationSyntax::TenseModal {
                tense_modal,
                inner_relation,
            }) => {
                self.analyze_tense_modal_nested(tense_modal);
                let inner = self.analyze_relation(inner_relation);
                self.add_frame(
                    relation_raw,
                    PlaceFrameKind::Forwarding,
                    relation_id,
                    None,
                    propagation_forward(inner),
                )
            }
            data!(RelationSyntax::Connected {
                leading_relation,
                trailing_relation,
                ..
            }) => {
                let leading = self.analyze_relation(leading_relation);
                let trailing = self.analyze_relation(trailing_relation);
                self.add_frame(
                    relation_raw,
                    PlaceFrameKind::Connected,
                    relation_id,
                    None,
                    propagation_connected(vec![leading, trailing]),
                )
            }
            data!(RelationSyntax::Bo {
                leading_relation,
                bo_tense_modal,
                trailing_relation,
                ..
            }) => {
                let leading = self.analyze_relation(leading_relation);
                if let Some(tense_modal) = bo_tense_modal.as_deref() {
                    self.analyze_tense_modal_nested(tense_modal);
                }
                let trailing = self.analyze_relation(trailing_relation);
                self.add_frame(
                    relation_raw,
                    PlaceFrameKind::Connected,
                    relation_id,
                    None,
                    propagation_connected(vec![leading, trailing]),
                )
            }
            data!(RelationSyntax::Co {
                leading_relation,
                trailing_relation,
                ..
            }) => {
                let leading = self.analyze_relation(leading_relation);
                let trailing = self.analyze_relation(trailing_relation);
                self.add_frame(
                    relation_raw,
                    PlaceFrameKind::CoInverted,
                    relation_id,
                    None,
                    propagation_co(leading, trailing),
                )
            }
            data!(RelationSyntax::Guha {
                leading_predicate,
                trailing_predicate,
                ..
            }) => {
                let leading = self.analyze_predicate(leading_predicate);
                let trailing = self.analyze_predicate(trailing_predicate);
                self.add_frame(
                    relation_raw,
                    PlaceFrameKind::Connected,
                    relation_id,
                    None,
                    propagation_connected(vec![leading, trailing]),
                )
            }
            data!(RelationSyntax::Abstraction(abstraction)) => {
                let propagation = if abstraction_is_property(abstraction) {
                    let inner = self
                        .analyze_subsentence_frame_with_initial_place(&abstraction.subsentence, 1);
                    propagation_forward(inner)
                } else {
                    self.analyze_subsentence(&abstraction.subsentence);
                    propagation_none()
                };
                self.add_frame(
                    relation_raw,
                    PlaceFrameKind::Abstraction,
                    relation_id,
                    None,
                    propagation,
                )
            }
            data!(RelationSyntax::Compound(units)) => {
                let mut unit_frames = Vec::new();
                for unit in units.iter() {
                    unit_frames.push(self.analyze_relation_unit(unit));
                }
                let head = *unit_frames
                    .last()
                    .expect("RelationUnitVec invariant ensures at least one unit");
                let modifiers = unit_frames[..unit_frames.len().saturating_sub(1)].to_vec();
                self.add_frame(
                    relation_raw,
                    PlaceFrameKind::Compound,
                    relation_id,
                    None,
                    propagation_compound(head, modifiers),
                )
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_relation_unit(&mut self, unit: &'tree RelationUnitSyntax) -> SelbriPlaceFrameId {
        let unit_id = self.index.relation_unit_node_id(unit);
        let unit_raw = self.raw_for(relation_unit_node_ref(unit));
        match unit.as_data() {
            data!(RelationUnitSyntax::Word(..))
            | data!(RelationUnitSyntax::Goha { .. })
            | data!(RelationUnitSyntax::Mehoi(..))
            | data!(RelationUnitSyntax::Gohoi(..))
            | data!(RelationUnitSyntax::Muhoi(..))
            | data!(RelationUnitSyntax::Moi { .. }) => self.add_frame(
                unit_raw,
                PlaceFrameKind::RelationUnit,
                None,
                unit_id,
                propagation_none(),
            ),
            data!(RelationUnitSyntax::Nuha { math_operator, .. }) => {
                self.analyze_math_operator_nested(math_operator);
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::RelationUnit,
                    None,
                    unit_id,
                    propagation_none(),
                )
            }
            data!(RelationUnitSyntax::Xohi { tag, .. }) => {
                self.analyze_tense_modal_nested(tag);
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::RelationUnit,
                    None,
                    unit_id,
                    propagation_none(),
                )
            }
            data!(RelationUnitSyntax::Me { argument, .. }) => {
                self.analyze_argument_nested(argument);
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::RelationUnit,
                    None,
                    unit_id,
                    propagation_none(),
                )
            }
            data!(RelationUnitSyntax::Luhei { text, .. }) => {
                self.analyze_text(text);
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::RelationUnit,
                    None,
                    unit_id,
                    propagation_none(),
                )
            }
            data!(RelationUnitSyntax::Se { se, inner_unit }) => {
                let inner = self.analyze_relation_unit(inner_unit);
                let converted_place = se_conversion_place(se)
                    .and_then(NonZeroU8::new)
                    .unwrap_or(NonZeroU8::new(2).expect("literal is non-zero"));
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::Converted,
                    None,
                    unit_id,
                    propagation_conversion(inner, converted_place),
                )
            }
            data!(RelationUnitSyntax::Ke {
                ke_tense_modal,
                relation,
                ..
            }) => {
                if let Some(tense_modal) = ke_tense_modal.as_deref() {
                    self.analyze_tense_modal_nested(tense_modal);
                }
                let inner = self.analyze_relation(relation);
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::Forwarding,
                    None,
                    unit_id,
                    propagation_forward(inner),
                )
            }
            data!(RelationUnitSyntax::Wrapped(relation)) => {
                let inner = self.analyze_relation(relation);
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::Forwarding,
                    None,
                    unit_id,
                    propagation_forward(inner),
                )
            }
            data!(RelationUnitSyntax::Nahe { inner_unit, .. }) => {
                let inner = self.analyze_relation_unit(inner_unit);
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::Forwarding,
                    None,
                    unit_id,
                    propagation_forward(inner),
                )
            }
            data!(RelationUnitSyntax::SelbriRelativeClause {
                base: inner_unit,
                selbri_relative_clauses,
            }) => {
                let inner = self.analyze_relation_unit(inner_unit);
                for relative_clause in selbri_relative_clauses {
                    self.analyze_relation(&relative_clause.relation);
                }
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::Forwarding,
                    None,
                    unit_id,
                    propagation_forward(inner),
                )
            }
            data!(RelationUnitSyntax::Cei {
                base: inner_unit,
                assignments,
            }) => {
                let inner = self.analyze_relation_unit(inner_unit);
                for assignment in assignments {
                    self.analyze_relation_unit(&assignment.relation_unit);
                }
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::Forwarding,
                    None,
                    unit_id,
                    propagation_forward(inner),
                )
            }
            data!(RelationUnitSyntax::Jai {
                tense_modal,
                inner_unit,
                ..
            }) => {
                if let Some(tense_modal) = tense_modal.as_deref() {
                    self.analyze_tense_modal_nested(tense_modal);
                }
                let inner = self.analyze_relation_unit(inner_unit);
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::JaiConverted,
                    None,
                    unit_id,
                    propagation_jai(inner),
                )
            }
            data!(RelationUnitSyntax::Bo {
                leading_unit,
                bo_tense_modal,
                trailing_unit,
                ..
            }) => {
                let leading = self.analyze_relation_unit(leading_unit);
                if let Some(tense_modal) = bo_tense_modal.as_deref() {
                    self.analyze_tense_modal_nested(tense_modal);
                }
                let trailing = self.analyze_relation_unit(trailing_unit);
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::Compound,
                    None,
                    unit_id,
                    propagation_compound(trailing, vec![leading]),
                )
            }
            data!(RelationUnitSyntax::Connected {
                leading_unit,
                trailing_unit,
                ..
            }) => {
                let leading = self.analyze_relation_unit(leading_unit);
                let trailing = self.analyze_relation_unit(trailing_unit);
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::Connected,
                    None,
                    unit_id,
                    propagation_connected(vec![leading, trailing]),
                )
            }
            data!(RelationUnitSyntax::Be {
                base,
                fa,
                first_argument,
                bei_links,
                ..
            })
            | data!(RelationUnitSyntax::PreposedBe {
                base,
                fa,
                first_argument,
                bei_links,
                ..
            }) => {
                let inner = self.analyze_relation_unit(base);
                self.assign_link_arguments(
                    inner,
                    fa.as_ref(),
                    first_argument.as_deref(),
                    bei_links,
                );
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::LinkedUnit,
                    None,
                    unit_id,
                    propagation_forward(inner),
                )
            }
            data!(RelationUnitSyntax::Abstraction(abstraction)) => {
                let propagation = if abstraction_is_property(abstraction) {
                    let inner = self
                        .analyze_subsentence_frame_with_initial_place(&abstraction.subsentence, 1);
                    propagation_forward(inner)
                } else {
                    self.analyze_subsentence(&abstraction.subsentence);
                    propagation_none()
                };
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::Abstraction,
                    None,
                    unit_id,
                    propagation,
                )
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_terms_nested(&mut self, terms: &'tree [TermSyntax]) {
        for term in terms {
            self.analyze_term_nested(term);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_term_nested(&mut self, term: &'tree TermSyntax) {
        match term.as_data() {
            data!(TermSyntax::Argument(argument)) | data!(TermSyntax::Fa { argument, .. }) => {
                self.analyze_argument_nested(argument);
            }
            data!(TermSyntax::NuhiTermset { termset, .. }) => self.analyze_terms_nested(termset),
            data!(TermSyntax::GekNuhiTermset {
                terms,
                gik_terms,
                ..
            }) => {
                self.analyze_terms_nested(terms);
                self.analyze_terms_nested(gik_terms);
            }
            data!(TermSyntax::Cehe {
                leading_terms,
                trailing_terms,
                ..
            })
            | data!(TermSyntax::Pehe {
                leading_terms,
                trailing_terms,
                ..
            })
            | data!(TermSyntax::Connected {
                leading_terms,
                trailing_terms,
                ..
            }) => {
                self.analyze_terms_nested(leading_terms);
                self.analyze_terms_nested(trailing_terms);
            }
            data!(TermSyntax::BoConnected {
                leading_terms,
                trailing_term,
                ..
            }) => {
                self.analyze_terms_nested(leading_terms);
                self.analyze_term_nested(trailing_term);
            }
            data!(TermSyntax::FihoiAdverbial { subsentence, .. })
            | data!(TermSyntax::SoiAdverbial { subsentence, .. }) => {
                self.analyze_subsentence(subsentence);
            }
            data!(TermSyntax::NoihaAdverbial {
                tail_elements,
                relation,
                relative_clauses,
                ..
            })
            | data!(TermSyntax::PoihaBrigahi {
                tail_elements,
                relation,
                relative_clauses,
                ..
            }) => {
                self.analyze_argument_tail_elements_nested(tail_elements);
                if let Some(relation) = relation.as_deref() {
                    self.analyze_relation(relation);
                }
                for relative_clause in relative_clauses {
                    self.analyze_relative_clause_nested(relative_clause);
                }
            }
            data!(TermSyntax::JaiTagged { tag, argument, .. }) => {
                if let Some(tense_modal) = tag.as_deref() {
                    self.analyze_tense_modal_nested(tense_modal);
                }
                self.analyze_argument_nested(argument);
            }
            data!(TermSyntax::Tagged {
                tense_modal,
                argument,
            }) => {
                if let Some(tense_modal) = tense_modal.as_deref() {
                    self.analyze_tense_modal_nested(tense_modal);
                }
                self.analyze_argument_nested(argument);
            }
            data!(TermSyntax::NaKu { .. }) | data!(TermSyntax::BareNa(..)) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_argument_nested(&mut self, argument: &'tree ArgumentSyntax) {
        match argument.as_data() {
            data!(ArgumentSyntax::Quantified {
                quantifier,
                inner_argument,
            }) => {
                self.analyze_quantifier_nested(quantifier);
                self.analyze_argument_nested(inner_argument);
            }
            data!(ArgumentSyntax::Tagged {
                tag,
                inner_argument,
            }) => {
                self.analyze_argument_tag_nested(tag);
                self.analyze_argument_nested(inner_argument);
            }
            data!(ArgumentSyntax::NaheBo { inner_argument, .. })
            | data!(ArgumentSyntax::Nahe { inner_argument, .. })
            | data!(ArgumentSyntax::Lahe { inner_argument, .. })
            | data!(ArgumentSyntax::Ke { inner_argument, .. }) => {
                self.analyze_argument_nested(inner_argument)
            }
            data!(ArgumentSyntax::RelativeClause {
                base_argument,
                relative_clauses,
                ..
            }) => {
                self.analyze_argument_nested(base_argument);
                for relative_clause in relative_clauses {
                    self.analyze_relative_clause_nested(relative_clause);
                }
            }
            data!(ArgumentSyntax::Vuho {
                base_argument,
                relative_clauses,
                connected_argument,
                ..
            }) => {
                self.analyze_argument_nested(base_argument);
                for relative_clause in relative_clauses {
                    self.analyze_relative_clause_nested(relative_clause);
                }
                if let Some(connected) = connected_argument.as_deref() {
                    self.analyze_argument_nested(&connected.argument);
                }
            }
            data!(ArgumentSyntax::BridiDescription { subsentence, .. }) => {
                self.analyze_subsentence(subsentence);
            }
            data!(ArgumentSyntax::TermWrapped { inner_term, .. }) => {
                self.analyze_term_nested(inner_term);
            }
            data!(ArgumentSyntax::Connected {
                leading_argument,
                trailing_argument,
                ..
            })
            | data!(ArgumentSyntax::Bo {
                leading_argument,
                trailing_argument,
                ..
            })
            | data!(ArgumentSyntax::Gek {
                leading_argument,
                trailing_argument,
                ..
            }) => {
                self.analyze_argument_nested(leading_argument);
                self.analyze_argument_nested(trailing_argument);
            }
            data!(ArgumentSyntax::Descriptor(descriptor)) => {
                if let Some(outer_quantifier) = descriptor.outer_quantifier.as_deref() {
                    self.analyze_quantifier_nested(outer_quantifier);
                }
                self.analyze_argument_tail_elements_nested(&descriptor.tail_elements);
                if let Some(relation) = descriptor.relation.as_deref() {
                    self.analyze_relation(relation);
                }
                for relative_clause in &descriptor.relative_clauses {
                    self.analyze_relative_clause_nested(relative_clause);
                }
            }
            data!(ArgumentSyntax::ConnectedDescriptor(descriptor)) => {
                self.analyze_argument_tail_elements_nested(&descriptor.tail_elements);
                if let Some(relation) = descriptor.relation.as_deref() {
                    self.analyze_relation(relation);
                }
                for relative_clause in &descriptor.relative_clauses {
                    self.analyze_relative_clause_nested(relative_clause);
                }
            }
            data!(ArgumentSyntax::RelationVocative {
                leading_relative_clauses,
                relation,
                trailing_relative_clauses,
            }) => {
                for relative_clause in leading_relative_clauses {
                    self.analyze_relative_clause_nested(relative_clause);
                }
                let frame = self.analyze_relation(relation);
                let argument_id = self
                    .index
                    .argument_node_id(argument)
                    .expect("argument belongs to indexed syntax tree");
                self.add_assignment(
                    frame,
                    numbered_slot(NonZeroU8::new(1).expect("literal is non-zero")),
                    argument_id,
                    None,
                    AssignmentSource::SequentialTerm,
                );
                for relative_clause in trailing_relative_clauses {
                    self.analyze_relative_clause_nested(relative_clause);
                }
            }
            data!(ArgumentSyntax::Quote(quote)) => self.analyze_quote_nested(quote),
            data!(ArgumentSyntax::MathExpression { expression, .. }) => {
                self.analyze_math_expression_nested(expression);
            }
            data!(ArgumentSyntax::Koha(koha)) => {
                self.analyze_free_modifiers_nested(&koha.free_modifiers);
            }
            data!(ArgumentSyntax::Letter { .. })
            | data!(ArgumentSyntax::NaKu { .. })
            | data!(ArgumentSyntax::Zohe { .. })
            | data!(ArgumentSyntax::Name { .. })
            | data!(ArgumentSyntax::Cmevla(..)) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_quote_nested(&mut self, quote: &'tree QuoteSyntax) {
        match quote.as_data() {
            data!(QuoteSyntax::Lu { text, .. }) => self.analyze_text(text),
            data!(QuoteSyntax::Zo(..))
            | data!(QuoteSyntax::ZohOi(..))
            | data!(QuoteSyntax::Zoi(..))
            | data!(QuoteSyntax::Lohu(..)) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_fragment(&mut self, fragment: &'tree FragmentSyntax) {
        match fragment.as_data() {
            data!(FragmentSyntax::Prenex { terms, .. })
            | data!(FragmentSyntax::Term { terms, .. }) => self.analyze_terms_nested(terms),
            data!(FragmentSyntax::BeLink {
                first_argument,
                bei_links,
                ..
            }) => {
                if let Some(argument) = first_argument.as_deref() {
                    self.analyze_argument_nested(argument);
                }
                self.analyze_bei_links_nested(bei_links);
            }
            data!(FragmentSyntax::BeiLink(bei_links)) => self.analyze_bei_links_nested(bei_links),
            data!(FragmentSyntax::RelativeClause(relative_clauses)) => {
                for relative_clause in relative_clauses {
                    self.analyze_relative_clause_nested(relative_clause);
                }
            }
            data!(FragmentSyntax::MathExpression(expression)) => {
                self.analyze_math_expression_nested(expression);
            }
            data!(FragmentSyntax::Relation(relation)) => {
                self.analyze_relation(relation);
            }
            data!(FragmentSyntax::Ek(..))
            | data!(FragmentSyntax::Gihek(..))
            | data!(FragmentSyntax::Other(..))
            | data!(FragmentSyntax::Ijek { .. }) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_bei_links_nested(&mut self, bei_links: &'tree [BeiLinkSyntax]) {
        for link in bei_links {
            if let Some(argument) = link.argument.as_deref() {
                self.analyze_argument_nested(argument);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_argument_tail_elements_nested(
        &mut self,
        tail_elements: &'tree [ArgumentTailElementSyntax],
    ) {
        for tail_element in tail_elements {
            self.analyze_argument_tail_element_nested(tail_element);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_argument_tail_element_nested(
        &mut self,
        tail_element: &'tree ArgumentTailElementSyntax,
    ) {
        match tail_element.as_data() {
            data!(ArgumentTailElementSyntax::Argument(argument)) => {
                self.analyze_argument_nested(argument);
            }
            data!(ArgumentTailElementSyntax::RelativeClauses(relative_clauses)) => {
                for relative_clause in relative_clauses {
                    self.analyze_relative_clause_nested(relative_clause);
                }
            }
            data!(ArgumentTailElementSyntax::Quantifier(quantifier)) => {
                self.analyze_quantifier_nested(quantifier);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_relative_clause_nested(&mut self, relative_clause: &'tree RelativeClauseSyntax) {
        match relative_clause.as_data() {
            data!(RelativeClauseSyntax::Goi(goi)) => {
                self.analyze_argument_nested(&goi.argument);
            }
            data!(RelativeClauseSyntax::Noi { subsentence, .. })
            | data!(RelativeClauseSyntax::Poi { subsentence, .. }) => {
                self.analyze_subsentence(subsentence);
            }
            data!(RelativeClauseSyntax::Zihe { inner, .. })
            | data!(RelativeClauseSyntax::Connected { inner, .. }) => {
                self.analyze_relative_clause_nested(inner);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_argument_tag_nested(&mut self, tag: &'tree ArgumentTagSyntax) {
        match tag.as_data() {
            data!(ArgumentTagSyntax::TenseModal(tense_modal)) => {
                self.analyze_tense_modal_nested(tense_modal);
            }
            data!(ArgumentTagSyntax::Fa(..)) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_quantifier_nested(&mut self, quantifier: &'tree QuantifierSyntax) {
        match quantifier.as_data() {
            data!(QuantifierSyntax::Vei {
                math_expression,
                ..
            }) => self.analyze_math_expression_nested(math_expression),
            data!(QuantifierSyntax::Number { .. }) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_math_expression_nested(&mut self, expression: &'tree MathExpressionSyntax) {
        match expression.as_data() {
            data!(MathExpressionSyntax::Number(quantifier)) => {
                self.analyze_quantifier_nested(quantifier);
            }
            data!(MathExpressionSyntax::Vei {
                inner_expression,
                ..
            })
            | data!(MathExpressionSyntax::Lahe {
                inner_expression,
                ..
            }) => self.analyze_math_expression_nested(inner_expression),
            data!(MathExpressionSyntax::Gek {
                left_expression,
                right_expression,
                ..
            })
            | data!(MathExpressionSyntax::Connected {
                left_expression,
                right_expression,
                ..
            }) => {
                self.analyze_math_expression_nested(left_expression);
                self.analyze_math_expression_nested(right_expression);
            }
            data!(MathExpressionSyntax::Forethought {
                operator,
                operands,
                ..
            }) => {
                self.analyze_math_operator_nested(operator);
                for operand in operands {
                    self.analyze_math_expression_nested(operand);
                }
            }
            data!(MathExpressionSyntax::ReversePolish {
                operands,
                operators,
                ..
            }) => {
                for operand in operands {
                    self.analyze_math_expression_nested(operand);
                }
                for operator in operators {
                    self.analyze_math_operator_nested(operator);
                }
            }
            data!(MathExpressionSyntax::Nihe { relation, .. }) => {
                self.analyze_relation(relation);
            }
            data!(MathExpressionSyntax::Mohe { argument, .. }) => {
                self.analyze_argument_nested(argument);
            }
            data!(MathExpressionSyntax::Johi { expressions, .. }) => {
                for expression in expressions.iter() {
                    self.analyze_math_expression_nested(expression);
                }
            }
            data!(MathExpressionSyntax::Binary {
                left_expression,
                operator,
                right_expression,
            })
            | data!(MathExpressionSyntax::Bihe {
                left_expression,
                operator,
                right_expression,
                ..
            }) => {
                self.analyze_math_expression_nested(left_expression);
                self.analyze_math_operator_nested(operator);
                self.analyze_math_expression_nested(right_expression);
            }
            data!(MathExpressionSyntax::Letter { .. }) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_math_operator_nested(&mut self, operator: &'tree MathOperatorSyntax) {
        match operator.as_data() {
            data!(MathOperatorSyntax::Maho {
                math_expression,
                ..
            }) => self.analyze_math_expression_nested(math_expression),
            data!(MathOperatorSyntax::Se { inner_operator, .. })
            | data!(MathOperatorSyntax::Nahe { inner_operator, .. })
            | data!(MathOperatorSyntax::Ke { inner_operator, .. }) => {
                self.analyze_math_operator_nested(inner_operator);
            }
            data!(MathOperatorSyntax::Nahu { relation, .. }) => {
                self.analyze_relation(relation);
            }
            data!(MathOperatorSyntax::Bo {
                left_operator,
                right_operator,
                ..
            })
            | data!(MathOperatorSyntax::Connected {
                left_operator,
                right_operator,
                ..
            }) => {
                self.analyze_math_operator_nested(left_operator);
                self.analyze_math_operator_nested(right_operator);
            }
            data!(MathOperatorSyntax::Vuhu(..)) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_tense_modal_nested(&mut self, tense_modal: &'tree TenseModalSyntax) {
        match tense_modal.as_data() {
            data!(TenseModalSyntax::Composite { parts }) => {
                for part in &parts.value {
                    if let data!(CompositeTenseModalPartSyntax::Fiho(fiho)) = part.as_data() {
                        self.analyze_relation(&fiho.relation);
                    }
                }
            }
            data!(TenseModalSyntax::Fiho { relation, .. }) => {
                self.analyze_relation(relation);
            }
            data!(TenseModalSyntax::Pu(..))
            | data!(TenseModalSyntax::PuDistance { .. })
            | data!(TenseModalSyntax::TimeInterval(..))
            | data!(TenseModalSyntax::PuCaha { .. })
            | data!(TenseModalSyntax::SpaceDistance(..))
            | data!(TenseModalSyntax::SpaceDirection(..))
            | data!(TenseModalSyntax::SpaceMovement { .. })
            | data!(TenseModalSyntax::Simple { .. })
            | data!(TenseModalSyntax::Ki(..))
            | data!(TenseModalSyntax::Caha(..))
            | data!(TenseModalSyntax::Zaho(..))
            | data!(TenseModalSyntax::Interval { .. }) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_free_modifiers_nested(&mut self, free_modifiers: &'tree [FreeModifierSyntax]) {
        for free_modifier in free_modifiers {
            self.analyze_free_modifier_nested(free_modifier);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_free_modifier_nested(&mut self, free_modifier: &'tree FreeModifierSyntax) {
        match free_modifier.as_data() {
            data!(FreeModifierSyntax::Sei {
                terms,
                relation,
                ..
            }) => {
                self.analyze_terms_nested(terms);
                self.analyze_relation(relation);
            }
            data!(FreeModifierSyntax::To { text, .. }) => self.analyze_text(text),
            data!(FreeModifierSyntax::Xi { expression, .. }) => {
                self.analyze_math_expression_nested(expression);
            }
            data!(FreeModifierSyntax::Soi {
                leading_argument,
                trailing_argument,
                ..
            }) => {
                self.analyze_argument_nested(leading_argument);
                if let Some(argument) = trailing_argument.as_deref() {
                    self.analyze_argument_nested(argument);
                }
            }
            data!(FreeModifierSyntax::Vocative { argument, .. }) => {
                if let Some(argument) = argument.as_deref() {
                    self.analyze_argument_nested(argument);
                }
            }
            data!(FreeModifierSyntax::Mai { .. })
            | data!(FreeModifierSyntax::Replacement { .. }) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_terms(
        &mut self,
        cursors: &mut Vec<PlaceCursor>,
        terms: &'tree [TermSyntax],
        source: AssignmentSource,
    ) {
        for term in terms {
            self.assign_term(cursors, term, source);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_term_refs(
        &mut self,
        cursors: &mut Vec<PlaceCursor>,
        terms: &[&'tree TermSyntax],
        source: AssignmentSource,
    ) {
        for term in terms {
            self.assign_term(cursors, term, source);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_alternative_term_branches(
        &mut self,
        cursors: &mut Vec<PlaceCursor>,
        leading_terms: &'tree [TermSyntax],
        trailing_terms: &'tree [TermSyntax],
    ) {
        let initial_cursors = std::mem::take(cursors);
        for initial_cursor in initial_cursors {
            let mut leading_cursors = vec![initial_cursor.clone()];
            self.assign_terms(
                &mut leading_cursors,
                leading_terms,
                AssignmentSource::TermsetBranch,
            );
            let mut trailing_cursors = vec![initial_cursor];
            self.assign_terms(
                &mut trailing_cursors,
                trailing_terms,
                AssignmentSource::TermsetBranch,
            );
            cursors.extend(leading_cursors);
            cursors.extend(trailing_cursors);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_term(
        &mut self,
        cursors: &mut Vec<PlaceCursor>,
        term: &'tree TermSyntax,
        source: AssignmentSource,
    ) {
        match term.as_data() {
            data!(TermSyntax::Argument(argument)) => {
                self.assign_argument_term_to_cursors(cursors, term, argument, source);
            }
            data!(TermSyntax::Fa { fa, argument, .. }) => {
                let slot = fa_place_slot(fa);
                self.assign_argument_to_cursors(
                    cursors,
                    term,
                    argument,
                    slot,
                    AssignmentSource::FaTerm,
                );
            }
            data!(TermSyntax::Tagged {
                tense_modal,
                argument,
            }) => {
                if let Some(tense_modal) = tense_modal.as_deref() {
                    self.analyze_tense_modal_nested(tense_modal);
                }
                let slot =
                    Some(modal_slot(tense_modal.as_deref().and_then(|tense| {
                        self.index.id_of(tense_modal_node_ref(tense))
                    })));
                self.assign_argument_to_cursors(
                    cursors,
                    term,
                    argument,
                    slot,
                    AssignmentSource::ModalTerm,
                );
            }
            data!(TermSyntax::JaiTagged { tag, argument, .. }) => {
                if let Some(tense_modal) = tag.as_deref() {
                    self.analyze_tense_modal_nested(tense_modal);
                }
                self.assign_argument_to_cursors(
                    cursors,
                    term,
                    argument,
                    Some(fai_slot()),
                    AssignmentSource::FaTerm,
                );
            }
            data!(TermSyntax::NuhiTermset { termset, .. }) => {
                self.assign_terms(cursors, termset, AssignmentSource::TermsetBranch);
            }
            data!(TermSyntax::GekNuhiTermset {
                terms,
                gik_terms,
                ..
            }) => {
                self.assign_alternative_term_branches(cursors, terms, gik_terms);
            }
            data!(TermSyntax::Cehe {
                leading_terms,
                trailing_terms,
                ..
            }) => {
                self.assign_terms(cursors, leading_terms, AssignmentSource::TermsetBranch);
                self.assign_terms(cursors, trailing_terms, AssignmentSource::TermsetBranch);
            }
            data!(TermSyntax::Pehe {
                leading_terms,
                trailing_terms,
                ..
            }) => {
                self.assign_alternative_term_branches(cursors, leading_terms, trailing_terms);
            }
            data!(TermSyntax::Connected {
                leading_terms,
                trailing_terms,
                ..
            }) => {
                self.assign_terms(cursors, leading_terms, AssignmentSource::TermsetBranch);
                self.assign_terms(cursors, trailing_terms, AssignmentSource::TermsetBranch);
            }
            data!(TermSyntax::BoConnected {
                leading_terms,
                trailing_term,
                ..
            }) => {
                self.assign_terms(cursors, leading_terms, AssignmentSource::TermsetBranch);
                self.assign_term(cursors, trailing_term, AssignmentSource::TermsetBranch);
            }
            _ => self.analyze_term_nested(term),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_argument_term_to_cursors(
        &mut self,
        cursors: &mut Vec<PlaceCursor>,
        term: &'tree TermSyntax,
        argument: &'tree ArgumentSyntax,
        source: AssignmentSource,
    ) {
        match argument.as_data() {
            data!(ArgumentSyntax::Connected {
                leading_argument,
                connective,
                trailing_argument,
            }) if connective_contains_cmavo(connective, Cmavo::Cehe) => {
                self.assign_argument_term_to_cursors(
                    cursors,
                    term,
                    leading_argument,
                    AssignmentSource::TermsetBranch,
                );
                self.assign_argument_term_to_cursors(
                    cursors,
                    term,
                    trailing_argument,
                    AssignmentSource::TermsetBranch,
                );
            }
            _ => self.assign_argument_to_cursors(cursors, term, argument, None, source),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_argument_to_cursors(
        &mut self,
        cursors: &mut Vec<PlaceCursor>,
        term: &'tree TermSyntax,
        argument: &'tree ArgumentSyntax,
        explicit_slot: Option<PlaceSlot>,
        source: AssignmentSource,
    ) {
        self.analyze_argument_nested(argument);
        let argument_id = self
            .index
            .argument_node_id(argument)
            .expect("argument belongs to indexed syntax tree");
        let term_id = self
            .index
            .term_node_id(term)
            .expect("term belongs to indexed syntax tree");
        for cursor in cursors {
            let slot = explicit_slot.unwrap_or_else(|| cursor.next_numbered_slot());
            self.add_assignment(cursor.frame, slot, argument_id, Some(term_id), source);
            cursor.record_slot(slot);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_link_arguments(
        &mut self,
        frame: SelbriPlaceFrameId,
        fa: Option<&'tree WithFreeModifiers<Token>>,
        first_argument: Option<&'tree ArgumentSyntax>,
        bei_links: &'tree [BeiLinkSyntax],
    ) {
        let mut cursor = PlaceCursor::new_at(frame, 2);
        if let Some(argument) = first_argument {
            let slot = fa.and_then(fa_place_slot);
            self.assign_link_argument(&mut cursor, argument, slot);
        }
        for link in bei_links {
            if let Some(argument) = link.argument.as_deref() {
                let slot = link.fa.as_ref().and_then(fa_place_slot);
                self.assign_link_argument(&mut cursor, argument, slot);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_link_argument(
        &mut self,
        cursor: &mut PlaceCursor,
        argument: &'tree ArgumentSyntax,
        explicit_slot: Option<PlaceSlot>,
    ) {
        self.analyze_argument_nested(argument);
        let argument_id = self
            .index
            .argument_node_id(argument)
            .expect("argument belongs to indexed syntax tree");
        let slot = explicit_slot
            .or_else(|| modal_slot_for_tagged_argument(argument, self.index))
            .unwrap_or_else(|| cursor.next_numbered_slot());
        self.add_assignment(
            cursor.frame,
            slot,
            argument_id,
            None,
            AssignmentSource::LinkArgument,
        );
        cursor.record_slot(slot);
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_frame(
        &mut self,
        node: RawSyntaxNodeId,
        kind: PlaceFrameKind,
        relation: Option<RelationNodeId>,
        relation_unit: Option<RelationUnitNodeId>,
        propagation: PlaceFramePropagation,
    ) -> SelbriPlaceFrameId {
        let id = SelbriPlaceFrameId(self.frames.len());
        self.frames.push(SelbriPlaceFrame {
            id,
            node,
            kind,
            relation,
            relation_unit,
            propagation,
        });
        self.frame_ids_by_node.entry(node).or_default().push(id);
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_assignment(
        &mut self,
        frame: SelbriPlaceFrameId,
        slot: PlaceSlot,
        argument: ArgumentNodeId,
        term: Option<TermNodeId>,
        source: AssignmentSource,
    ) {
        let mut visited = HashSet::new();
        self.add_assignment_recursive(frame, slot, argument, term, source, &mut visited);
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_assignment_recursive(
        &mut self,
        frame: SelbriPlaceFrameId,
        slot: PlaceSlot,
        argument: ArgumentNodeId,
        term: Option<TermNodeId>,
        source: AssignmentSource,
        visited: &mut HashSet<(SelbriPlaceFrameId, PlaceSlot)>,
    ) {
        if !visited.insert((frame, slot)) {
            return;
        }
        let id = ArgumentPlaceAssignmentId(self.assignments.len());
        self.assignments.push(ArgumentPlaceAssignment {
            id,
            frame,
            slot,
            argument,
            term,
            source,
        });
        self.assignment_ids_by_argument
            .entry(argument)
            .or_default()
            .push(id);
        if let Some(term) = term {
            self.assignment_ids_by_term
                .entry(term)
                .or_default()
                .push(id);
        }
        self.assignment_ids_by_frame
            .entry(frame)
            .or_default()
            .push(id);
        self.assignment_ids_by_frame_slot
            .entry((frame, slot))
            .or_default()
            .push(id);
        self.propagate_assignment(frame, slot, argument, term, source, visited);
    }

    #[requires(true)]
    #[ensures(true)]
    fn propagate_assignment(
        &mut self,
        frame: SelbriPlaceFrameId,
        slot: PlaceSlot,
        argument: ArgumentNodeId,
        term: Option<TermNodeId>,
        source: AssignmentSource,
        visited: &mut HashSet<(SelbriPlaceFrameId, PlaceSlot)>,
    ) {
        let Some(frame_data) = self.frames.get(frame.0).cloned() else {
            return;
        };
        match frame_data.propagation {
            PlaceFramePropagation::None => {}
            PlaceFramePropagation::Forward { inner } => {
                self.add_assignment_recursive(
                    inner,
                    slot,
                    argument,
                    term,
                    AssignmentSource::Propagated,
                    visited,
                );
            }
            PlaceFramePropagation::Conversion {
                inner,
                converted_place,
            } => {
                let mapped = convert_slot(slot, converted_place);
                self.add_assignment_recursive(
                    inner,
                    mapped,
                    argument,
                    term,
                    AssignmentSource::Propagated,
                    visited,
                );
            }
            PlaceFramePropagation::Jai { inner } => match slot {
                PlaceSlot::Fai => self.add_assignment_recursive(
                    inner,
                    numbered_slot(NonZeroU8::new(1).expect("literal is non-zero")),
                    argument,
                    term,
                    AssignmentSource::Propagated,
                    visited,
                ),
                PlaceSlot::Numbered(place) if place.get() > 1 => self.add_assignment_recursive(
                    inner,
                    numbered_slot(place),
                    argument,
                    term,
                    AssignmentSource::Propagated,
                    visited,
                ),
                PlaceSlot::Numbered(_) | PlaceSlot::Modal(_) => {}
            },
            PlaceFramePropagation::Connected { branches } => {
                for branch in branches {
                    self.add_assignment_recursive(
                        branch,
                        slot,
                        argument,
                        term,
                        AssignmentSource::Propagated,
                        visited,
                    );
                }
            }
            PlaceFramePropagation::Compound { head, modifiers } => {
                self.add_assignment_recursive(
                    head,
                    slot,
                    argument,
                    term,
                    AssignmentSource::Propagated,
                    visited,
                );
                if slot.numbered_index() == Some(1) {
                    for modifier in modifiers {
                        self.add_assignment_recursive(
                            modifier,
                            slot,
                            argument,
                            term,
                            AssignmentSource::CompoundSharedX1,
                            visited,
                        );
                    }
                }
            }
            PlaceFramePropagation::Co { leading, trailing } => {
                self.add_assignment_recursive(
                    trailing,
                    slot,
                    argument,
                    term,
                    AssignmentSource::Propagated,
                    visited,
                );
                if slot.numbered_index() == Some(1) {
                    self.add_assignment_recursive(
                        leading,
                        slot,
                        argument,
                        term,
                        AssignmentSource::CompoundSharedX1,
                        visited,
                    );
                }
            }
        }
        let _ = source;
    }

    #[requires(true)]
    #[ensures(true)]
    fn raw_for(&self, node: SyntaxNodeRef<'tree>) -> RawSyntaxNodeId {
        self.index
            .id_of(node)
            .expect("node belongs to indexed syntax tree")
    }
}

#[derive(Debug, Clone)]
#[invariant(true)]
struct PredicateTailAnalysis<'tree> {
    frames: Vec<SelbriPlaceFrameId>,
    terms: Vec<&'tree TermSyntax>,
    branch_cursors: Option<Vec<PlaceCursor>>,
}

#[derive(Debug, Clone)]
#[invariant(true)]
struct PlaceCursor {
    frame: SelbriPlaceFrameId,
    next_place: u8,
    filled_numbered: HashSet<u8>,
}

impl PlaceCursor {
    #[requires(true)]
    #[ensures(ret.next_place == 1)]
    fn new(frame: SelbriPlaceFrameId) -> Self {
        Self::new_at(frame, 1)
    }

    #[requires(start > 0)]
    #[ensures(ret.next_place == start)]
    fn new_at(frame: SelbriPlaceFrameId, start: u8) -> Self {
        Self {
            frame,
            next_place: start,
            filled_numbered: HashSet::new(),
        }
    }

    #[requires(true)]
    #[ensures(ret.numbered_index().is_some())]
    fn next_numbered_slot(&mut self) -> PlaceSlot {
        while self.filled_numbered.contains(&self.next_place) {
            self.next_place = self.next_place.saturating_add(1);
        }
        numbered_slot(NonZeroU8::new(self.next_place).expect("next place is non-zero"))
    }

    #[requires(true)]
    #[ensures(true)]
    fn record_slot(&mut self, slot: PlaceSlot) {
        match slot {
            PlaceSlot::Numbered(place) => {
                let place = place.get();
                self.filled_numbered.insert(place);
                self.next_place = place.saturating_add(1);
            }
            PlaceSlot::Modal(_) | PlaceSlot::Fai => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn mark_filled_slot(&mut self, slot: PlaceSlot) {
        if let PlaceSlot::Numbered(place) = slot {
            self.filled_numbered.insert(place.get());
            while self.filled_numbered.contains(&self.next_place) {
                self.next_place = self.next_place.saturating_add(1);
            }
        }
    }

    #[requires(minimum > 0)]
    #[ensures(self.next_place >= minimum)]
    fn ensure_next_place_at_least(&mut self, minimum: u8) {
        self.next_place = self.next_place.max(minimum);
    }

    #[requires(self.frame == branch.frame)]
    #[ensures(self.frame == old(self.frame))]
    #[ensures(self.next_place >= old(self.next_place))]
    fn merge_from_branch(&mut self, branch: &PlaceCursor) {
        self.next_place = self.next_place.max(branch.next_place);
        self.filled_numbered
            .extend(branch.filled_numbered.iter().copied());
        while self.filled_numbered.contains(&self.next_place) {
            self.next_place = self.next_place.saturating_add(1);
        }
    }
}

#[derive(Debug)]
#[invariant(true)]
pub struct SyntaxIndex<'tree> {
    nodes: Vec<IndexedSyntaxNode<'tree>>,
    by_ref: HashMap<SyntaxNodeRef<'tree>, RawSyntaxNodeId>,
    root: TextNodeId,
}

#[derive(Debug)]
#[invariant(true)]
struct IndexedSyntaxNode<'tree> {
    node: SyntaxNodeRef<'tree>,
    metadata: SyntaxNodeMetadata,
}

impl<'tree> SyntaxIndex<'tree> {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|index| !index.nodes.is_empty()))]
    pub fn new(root: &'tree TextSyntax) -> Result<Self, ReferenceAnalysisError> {
        let mut builder = SyntaxIndexBuilder::new();
        root.visit_in_order(&mut builder);
        let root_raw = builder
            .by_ref
            .get(&SyntaxNodeRef::TextSyntax(root))
            .copied()
            .ok_or(ReferenceAnalysisError::MissingRootNode)?;
        Ok(Self {
            nodes: builder.nodes,
            by_ref: builder.by_ref,
            root: TextNodeId(root_raw),
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn root(&self) -> TextNodeId {
        self.root
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn node(&self, id: RawSyntaxNodeId) -> Option<SyntaxNodeRef<'tree>> {
        self.nodes.get(id.0).map(|node| node.node)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn metadata(&self, id: RawSyntaxNodeId) -> Option<&SyntaxNodeMetadata> {
        self.nodes.get(id.0).map(|node| &node.metadata)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn id_of(&self, node: SyntaxNodeRef<'tree>) -> Option<RawSyntaxNodeId> {
        self.by_ref.get(&node).copied()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn text_node_id(&self, node: &'tree TextSyntax) -> Option<TextNodeId> {
        self.id_of(SyntaxNodeRef::TextSyntax(node)).map(TextNodeId)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn paragraph_node_id(&self, node: &'tree ParagraphSyntax) -> Option<ParagraphNodeId> {
        self.id_of(SyntaxNodeRef::ParagraphSyntax(node))
            .map(ParagraphNodeId)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn statement_node_id(&self, node: &'tree StatementSyntax) -> Option<StatementNodeId> {
        self.id_of(statement_node_ref(node)).map(StatementNodeId)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn predicate_node_id(&self, node: &'tree PredicateSyntax) -> Option<PredicateNodeId> {
        self.id_of(SyntaxNodeRef::PredicateSyntax(node))
            .map(PredicateNodeId)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn predicate_tail_node_id(
        &self,
        node: &'tree PredicateTailSyntax,
    ) -> Option<PredicateTailNodeId> {
        self.id_of(SyntaxNodeRef::PredicateTailSyntax(node))
            .map(PredicateTailNodeId)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn relation_node_id(&self, node: &'tree RelationSyntax) -> Option<RelationNodeId> {
        self.id_of(relation_node_ref(node)).map(RelationNodeId)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn relation_unit_node_id(
        &self,
        node: &'tree RelationUnitSyntax,
    ) -> Option<RelationUnitNodeId> {
        self.id_of(relation_unit_node_ref(node))
            .map(RelationUnitNodeId)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn term_node_id(&self, node: &'tree TermSyntax) -> Option<TermNodeId> {
        self.id_of(term_node_ref(node)).map(TermNodeId)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn argument_node_id(&self, node: &'tree ArgumentSyntax) -> Option<ArgumentNodeId> {
        self.id_of(argument_node_ref(node)).map(ArgumentNodeId)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn abstraction_node_id(&self, node: &'tree AbstractionSyntax) -> Option<AbstractionNodeId> {
        self.id_of(SyntaxNodeRef::AbstractionSyntax(node))
            .map(AbstractionNodeId)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn relation(&self, id: RelationNodeId) -> Option<&'tree RelationSyntax> {
        node_ref_as_relation(self.node(id.0)?)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn relation_unit(&self, id: RelationUnitNodeId) -> Option<&'tree RelationUnitSyntax> {
        node_ref_as_relation_unit(self.node(id.0)?)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn term(&self, id: TermNodeId) -> Option<&'tree TermSyntax> {
        node_ref_as_term(self.node(id.0)?)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn argument(&self, id: ArgumentNodeId) -> Option<&'tree ArgumentSyntax> {
        node_ref_as_argument(self.node(id.0)?)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn predicate(&self, id: PredicateNodeId) -> Option<&'tree PredicateSyntax> {
        match self.node(id.0)? {
            SyntaxNodeRef::PredicateSyntax(node) => Some(node),
            _ => None,
        }
    }
}

#[derive(Debug)]
#[invariant(true)]
struct SyntaxIndexBuilder<'tree> {
    nodes: Vec<IndexedSyntaxNode<'tree>>,
    by_ref: HashMap<SyntaxNodeRef<'tree>, RawSyntaxNodeId>,
    stack: Vec<RawSyntaxNodeId>,
    leaf_index: usize,
}

impl<'tree> SyntaxIndexBuilder<'tree> {
    #[requires(true)]
    #[ensures(ret.nodes.is_empty())]
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            by_ref: HashMap::new(),
            stack: Vec::new(),
            leaf_index: 0,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn record_source_span(&mut self, span: &SourceSpan) {
        for id in &self.stack {
            if let Some(node) = self.nodes.get_mut(id.0) {
                node.metadata.source_spans.push(span.clone());
            }
        }
        self.leaf_index += 1;
    }
}

impl<'tree> TreeVisitor<'tree> for SyntaxIndexBuilder<'tree> {
    type Node = SyntaxNodeRef<'tree>;
    type Atom = SyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        let id = RawSyntaxNodeId(self.nodes.len());
        let parent = self.stack.last().copied();
        let metadata = SyntaxNodeMetadata {
            id,
            parent,
            preorder: id.0,
            depth: self.stack.len(),
            leaf_start: self.leaf_index,
            leaf_end: self.leaf_index,
            source_spans: Vec::new(),
        };
        self.nodes.push(IndexedSyntaxNode { node, metadata });
        self.by_ref.insert(node, id);
        self.stack.push(id);
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, node: Self::Node) {
        let Some(id) = self.stack.pop() else {
            return;
        };
        debug_assert_eq!(self.nodes[id.0].node, node);
        self.nodes[id.0].metadata.leaf_end = self.leaf_index;
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        match atom {
            SyntaxAtomRef::Token(word) => {
                for span in word.source_spans() {
                    self.record_source_span(span);
                }
            }
            SyntaxAtomRef::Word(word) => self.record_source_span(word.span()),
        }
    }
}

#[derive(Debug)]
#[invariant(true)]
struct ArgumentMention {
    source: ArgumentNodeId,
    target: ArgumentNodeId,
    position: usize,
    available_to_ri: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
struct NodeMention {
    source: RawSyntaxNodeId,
    target: RawSyntaxNodeId,
    position: usize,
}

#[derive(Debug)]
#[invariant(true)]
struct DiscourseReferenceBuilder<'index, 'tree> {
    index: &'index SyntaxIndex<'tree>,
    places: &'index PlaceAnalysis,
    edges: Vec<ReferenceEdge>,
    edge_ids_by_source: HashMap<RawSyntaxNodeId, Vec<ReferenceEdgeId>>,
    edge_ids_by_target_node: HashMap<RawSyntaxNodeId, Vec<ReferenceEdgeId>>,
    koha_bindings: HashMap<Cmavo, ArgumentNodeId>,
    cei_predicate_bindings: HashMap<String, PredicateNodeId>,
    relation_variable_bindings: HashMap<Cmavo, RelationNodeId>,
    da_bindings: HashMap<Cmavo, ArgumentNodeId>,
    argument_mentions: Vec<ArgumentMention>,
    letter_mentions: HashMap<String, Vec<ArgumentMention>>,
    predicate_mentions: Vec<NodeMention>,
    last_predicate: Option<PredicateNodeId>,
    current_predicate: Option<PredicateNodeId>,
    predicate_stack: Vec<RawSyntaxNodeId>,
    discourse_predicate_stack: Vec<RawSyntaxNodeId>,
    abstraction_stack: Vec<RawSyntaxNodeId>,
    utterance_history: Vec<RawSyntaxNodeId>,
    current_utterance: Option<RawSyntaxNodeId>,
    pending_next_utterance_sources: Vec<RawSyntaxNodeId>,
    current_predicate_frames: Vec<SelbriPlaceFrameId>,
    relative_heads: Vec<ArgumentNodeId>,
}

impl<'index, 'tree> DiscourseReferenceBuilder<'index, 'tree> {
    #[requires(true)]
    #[ensures(ret.edges.is_empty())]
    fn new(index: &'index SyntaxIndex<'tree>, places: &'index PlaceAnalysis) -> Self {
        Self {
            index,
            places,
            edges: Vec::new(),
            edge_ids_by_source: HashMap::new(),
            edge_ids_by_target_node: HashMap::new(),
            koha_bindings: HashMap::new(),
            cei_predicate_bindings: HashMap::new(),
            relation_variable_bindings: HashMap::new(),
            da_bindings: HashMap::new(),
            argument_mentions: Vec::new(),
            letter_mentions: HashMap::new(),
            predicate_mentions: Vec::new(),
            last_predicate: None,
            current_predicate: None,
            predicate_stack: Vec::new(),
            discourse_predicate_stack: Vec::new(),
            abstraction_stack: Vec::new(),
            utterance_history: Vec::new(),
            current_utterance: None,
            pending_next_utterance_sources: Vec::new(),
            current_predicate_frames: Vec::new(),
            relative_heads: Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(mut self) -> DiscourseReferences {
        for source in std::mem::take(&mut self.pending_next_utterance_sources) {
            self.add_edge(
                ReferenceKind::Utterance,
                source,
                target_unresolved("di'e has no following utterance"),
                "di'e refers to the following utterance when one is present",
            );
        }
        DiscourseReferences {
            edges: self.edges,
            edge_ids_by_source: self.edge_ids_by_source,
            edge_ids_by_target_node: self.edge_ids_by_target_node,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_text(&mut self, text: &'tree TextSyntax) {
        self.visit_free_modifiers(&text.leading_free_modifiers);
        for paragraph in &text.paragraphs {
            self.visit_paragraph(paragraph);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_paragraph(&mut self, paragraph: &'tree ParagraphSyntax) {
        self.visit_free_modifiers(&paragraph.free_modifiers);
        for statement in &paragraph.statements {
            self.visit_free_modifiers(&statement.free_modifiers);
            if let Some(statement) = statement.statement.as_deref() {
                self.visit_statement(statement);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_statement(&mut self, statement: &'tree StatementSyntax) {
        let statement_id = self
            .index
            .statement_node_id(statement)
            .expect("statement belongs to indexed syntax tree");
        for source in std::mem::take(&mut self.pending_next_utterance_sources) {
            self.add_edge(
                ReferenceKind::Utterance,
                source,
                target_resolved_node(statement_id.0),
                "di'e refers to the following utterance",
            );
        }
        let previous_utterance = self.current_utterance.replace(statement_id.0);
        match statement.as_data() {
            data!(StatementSyntax::Tuhe { text, .. }) => self.visit_text(text),
            data!(StatementSyntax::Prenex {
                prenex_terms,
                inner_statement,
                ..
            }) => {
                self.visit_terms(prenex_terms);
                let previous_relation_variable_bindings = self.relation_variable_bindings.clone();
                self.bind_prenex_relation_variables(prenex_terms);
                let previous_cei_predicate_bindings = self.cei_predicate_bindings.clone();
                self.bind_prenex_cei_predicate_targets_for_statement(prenex_terms, inner_statement);
                self.visit_statement(inner_statement);
                self.cei_predicate_bindings = previous_cei_predicate_bindings;
                self.relation_variable_bindings = previous_relation_variable_bindings;
            }
            data!(StatementSyntax::Predicate(predicate)) => {
                self.visit_predicate(predicate);
            }
            data!(StatementSyntax::Connected {
                leading_statement,
                trailing_statement,
                ..
            })
            | data!(StatementSyntax::PreIConnected {
                leading_statement,
                trailing_statement,
                ..
            }) => {
                self.visit_statement(leading_statement);
                self.visit_statement(trailing_statement);
            }
            data!(StatementSyntax::Iau {
                inner_statement,
                reset_terms,
                ..
            }) => {
                self.visit_statement(inner_statement);
                self.visit_terms(reset_terms);
            }
            data!(StatementSyntax::ExperimentalPredicateContinuation {
                leading_statement,
                continuation,
            }) => {
                self.visit_statement(leading_statement);
                if let Some(tense_modal) = continuation.tense_modal.as_deref() {
                    self.visit_tense_modal(tense_modal);
                }
                self.visit_subsentence(&continuation.trailing_subsentence);
            }
            data!(StatementSyntax::Fragment(fragment)) => self.visit_fragment(fragment),
        }
        self.current_utterance = previous_utterance;
        self.utterance_history.push(statement_id.0);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_subsentence(&mut self, subsentence: &'tree SubsentenceSyntax) {
        match subsentence.as_data() {
            data!(SubsentenceSyntax::Plain(predicate)) => self.visit_predicate(predicate),
            data!(SubsentenceSyntax::Prenex {
                prenex_terms,
                inner_subsentence,
                ..
            }) => {
                self.visit_terms(prenex_terms);
                let previous_relation_variable_bindings = self.relation_variable_bindings.clone();
                self.bind_prenex_relation_variables(prenex_terms);
                let previous_cei_predicate_bindings = self.cei_predicate_bindings.clone();
                self.bind_prenex_cei_predicate_targets_for_subsentence(
                    prenex_terms,
                    inner_subsentence,
                );
                self.visit_subsentence(inner_subsentence);
                self.cei_predicate_bindings = previous_cei_predicate_bindings;
                self.relation_variable_bindings = previous_relation_variable_bindings;
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_predicate(&mut self, predicate: &'tree PredicateSyntax) {
        let predicate_id = self
            .index
            .predicate_node_id(predicate)
            .expect("predicate belongs to indexed syntax tree");
        let frames = self.places.frames_for_node(predicate_id.0).to_vec();
        let previous_frames = std::mem::replace(&mut self.current_predicate_frames, frames);
        let previous_predicate = self.current_predicate.replace(predicate_id);
        let was_top_predicate = self.predicate_stack.is_empty();
        let is_in_abstraction = !self.abstraction_stack.is_empty();
        self.predicate_stack.push(predicate_id.0);
        if !is_in_abstraction {
            self.discourse_predicate_stack.push(predicate_id.0);
        }
        self.visit_terms(&predicate.leading_terms);
        self.visit_predicate_tail(&predicate.predicate_tail);
        self.visit_free_modifiers(&predicate.free_modifiers);
        if !is_in_abstraction {
            self.discourse_predicate_stack.pop();
        }
        self.predicate_stack.pop();
        self.current_predicate_frames = previous_frames;
        self.current_predicate = previous_predicate;
        self.last_predicate = Some(predicate_id);
        if was_top_predicate {
            self.note_predicate_mention(predicate_id.0);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_predicate_tail(&mut self, tail: &'tree PredicateTailSyntax) {
        self.visit_predicate_tail1(&tail.first);
        if let Some(continuation) = tail.ke_continuation.as_deref() {
            if let Some(tense_modal) = continuation.tense_modal.as_deref() {
                self.visit_tense_modal(tense_modal);
            }
            self.visit_predicate_tail(&continuation.predicate_tail);
            self.visit_terms(&continuation.tail_terms);
            self.visit_free_modifiers(&continuation.free_modifiers);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_predicate_tail1(&mut self, tail: &'tree PredicateTail1Syntax) {
        self.visit_predicate_tail2(&tail.first);
        for continuation in &tail.continuations {
            if let Some(tense_modal) = continuation.tense_modal.as_deref() {
                self.visit_tense_modal(tense_modal);
            }
            self.visit_predicate_tail2(&continuation.predicate_tail);
            self.visit_terms(&continuation.tail_terms);
            self.visit_free_modifiers(&continuation.free_modifiers);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_predicate_tail2(&mut self, tail: &'tree PredicateTail2Syntax) {
        self.visit_predicate_tail3(&tail.first);
        if let Some(continuation) = tail.bo_continuation.as_deref() {
            if let Some(tense_modal) = continuation.tense_modal.as_deref() {
                self.visit_tense_modal(tense_modal);
            }
            self.visit_predicate_tail2(&continuation.predicate_tail);
            self.visit_terms(&continuation.tail_terms);
            self.visit_free_modifiers(&continuation.free_modifiers);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_predicate_tail3(&mut self, tail: &'tree PredicateTail3Syntax) {
        match tail.as_data() {
            data!(PredicateTail3Syntax::Relation {
                relation,
                terms,
                free_modifiers,
                ..
            }) => {
                self.visit_relation(relation);
                self.visit_terms(terms);
                self.visit_free_modifiers(free_modifiers);
            }
            data!(PredicateTail3Syntax::GekSentence(gek)) => self.visit_gek_sentence(gek),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_gek_sentence(&mut self, gek: &'tree jbotci_syntax::ast::GekSentenceSyntax) {
        match gek.as_data() {
            data!(jbotci_syntax::ast::GekSentenceSyntax::Pair {
                first,
                second,
                tail_terms,
                free_modifiers,
                ..
            }) => {
                self.visit_subsentence(first);
                self.visit_subsentence(second);
                self.visit_terms(tail_terms);
                self.visit_free_modifiers(free_modifiers);
            }
            data!(jbotci_syntax::ast::GekSentenceSyntax::Ke {
                tense_modal,
                inner,
                ..
            }) => {
                if let Some(tense_modal) = tense_modal.as_deref() {
                    self.visit_tense_modal(tense_modal);
                }
                self.visit_gek_sentence(inner);
            }
            data!(jbotci_syntax::ast::GekSentenceSyntax::Na { inner, .. }) => {
                self.visit_gek_sentence(inner);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_terms(&mut self, terms: &'tree [TermSyntax]) {
        for term in terms {
            self.visit_term(term);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn bind_prenex_relation_variables(&mut self, terms: &'tree [TermSyntax]) {
        for term in terms {
            self.bind_prenex_relation_variables_in_term(term);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn bind_prenex_cei_predicate_targets_for_statement(
        &mut self,
        terms: &'tree [TermSyntax],
        statement: &'tree StatementSyntax,
    ) {
        if let Some(predicate) = self.statement_main_predicate_id(statement) {
            self.bind_prenex_cei_predicate_targets(terms, predicate);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn bind_prenex_cei_predicate_targets_for_subsentence(
        &mut self,
        terms: &'tree [TermSyntax],
        subsentence: &'tree SubsentenceSyntax,
    ) {
        if let Some(predicate) = self.subsentence_main_predicate_id(subsentence) {
            self.bind_prenex_cei_predicate_targets(terms, predicate);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn bind_prenex_cei_predicate_targets(
        &mut self,
        terms: &'tree [TermSyntax],
        predicate: PredicateNodeId,
    ) {
        for (label, source) in self.prenex_cei_assignment_sources(terms) {
            self.cei_predicate_bindings.insert(label, predicate);
            self.add_edge(
                ReferenceKind::CeiAssignment,
                source,
                target_resolved_node(predicate.0),
                "prenex CEI assignment binds the following predicate",
            );
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn statement_main_predicate_id(
        &self,
        statement: &'tree StatementSyntax,
    ) -> Option<PredicateNodeId> {
        match statement.as_data() {
            data!(StatementSyntax::Predicate(predicate)) => self.index.predicate_node_id(predicate),
            data!(StatementSyntax::Prenex {
                inner_statement,
                ..
            }) => self.statement_main_predicate_id(inner_statement),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn subsentence_main_predicate_id(
        &self,
        subsentence: &'tree SubsentenceSyntax,
    ) -> Option<PredicateNodeId> {
        match subsentence.as_data() {
            data!(SubsentenceSyntax::Plain(predicate)) => self.index.predicate_node_id(predicate),
            data!(SubsentenceSyntax::Prenex {
                inner_subsentence,
                ..
            }) => self.subsentence_main_predicate_id(inner_subsentence),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn prenex_cei_assignment_sources(
        &self,
        terms: &'tree [TermSyntax],
    ) -> Vec<(String, RawSyntaxNodeId)> {
        let mut sources = Vec::new();
        for term in terms {
            self.collect_prenex_cei_assignment_sources_in_term(term, &mut sources);
        }
        sources
    }

    #[requires(true)]
    #[ensures(true)]
    fn collect_prenex_cei_assignment_sources_in_term(
        &self,
        term: &'tree TermSyntax,
        sources: &mut Vec<(String, RawSyntaxNodeId)>,
    ) {
        match term.as_data() {
            data!(TermSyntax::Argument(argument))
            | data!(TermSyntax::Fa { argument, .. })
            | data!(TermSyntax::Tagged { argument, .. })
            | data!(TermSyntax::JaiTagged { argument, .. }) => {
                self.collect_prenex_cei_assignment_sources_in_argument(argument, sources);
            }
            data!(TermSyntax::NuhiTermset { termset, .. }) => {
                self.collect_prenex_cei_assignment_sources(termset, sources);
            }
            data!(TermSyntax::GekNuhiTermset {
                terms,
                gik_terms,
                ..
            }) => {
                self.collect_prenex_cei_assignment_sources(terms, sources);
                self.collect_prenex_cei_assignment_sources(gik_terms, sources);
            }
            data!(TermSyntax::Cehe {
                leading_terms,
                trailing_terms,
                ..
            })
            | data!(TermSyntax::Pehe {
                leading_terms,
                trailing_terms,
                ..
            })
            | data!(TermSyntax::Connected {
                leading_terms,
                trailing_terms,
                ..
            }) => {
                self.collect_prenex_cei_assignment_sources(leading_terms, sources);
                self.collect_prenex_cei_assignment_sources(trailing_terms, sources);
            }
            data!(TermSyntax::BoConnected {
                leading_terms,
                trailing_term,
                ..
            }) => {
                self.collect_prenex_cei_assignment_sources(leading_terms, sources);
                self.collect_prenex_cei_assignment_sources_in_term(trailing_term, sources);
            }
            _ => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn collect_prenex_cei_assignment_sources(
        &self,
        terms: &'tree [TermSyntax],
        sources: &mut Vec<(String, RawSyntaxNodeId)>,
    ) {
        for term in terms {
            self.collect_prenex_cei_assignment_sources_in_term(term, sources);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn collect_prenex_cei_assignment_sources_in_argument(
        &self,
        argument: &'tree ArgumentSyntax,
        sources: &mut Vec<(String, RawSyntaxNodeId)>,
    ) {
        match argument.as_data() {
            data!(ArgumentSyntax::Descriptor(descriptor)) => {
                if let Some(relation) = descriptor.relation.as_deref() {
                    self.collect_prenex_cei_assignment_sources_in_relation(relation, sources);
                }
            }
            data!(ArgumentSyntax::ConnectedDescriptor(descriptor)) => {
                if let Some(relation) = descriptor.relation.as_deref() {
                    self.collect_prenex_cei_assignment_sources_in_relation(relation, sources);
                }
            }
            _ => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn collect_prenex_cei_assignment_sources_in_relation(
        &self,
        relation: &'tree RelationSyntax,
        sources: &mut Vec<(String, RawSyntaxNodeId)>,
    ) {
        match relation.as_data() {
            data!(RelationSyntax::Compound(units)) => {
                for unit in units.iter() {
                    self.collect_prenex_cei_assignment_sources_in_relation_unit(unit, sources);
                }
            }
            data!(RelationSyntax::Base(..)) => {}
            _ => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn collect_prenex_cei_assignment_sources_in_relation_unit(
        &self,
        unit: &'tree RelationUnitSyntax,
        sources: &mut Vec<(String, RawSyntaxNodeId)>,
    ) {
        if let data!(RelationUnitSyntax::Cei { assignments, .. }) = unit.as_data() {
            for assignment in assignments {
                if let Some(label) = relation_unit_assignment_label(&assignment.relation_unit) {
                    let source = self
                        .index
                        .relation_unit_node_id(&assignment.relation_unit)
                        .expect("prenex CEI assignment belongs to indexed syntax tree");
                    sources.push((label, source.0));
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn bind_prenex_relation_variables_in_term(&mut self, term: &'tree TermSyntax) {
        match term.as_data() {
            data!(TermSyntax::Argument(argument))
            | data!(TermSyntax::Fa { argument, .. })
            | data!(TermSyntax::Tagged { argument, .. })
            | data!(TermSyntax::JaiTagged { argument, .. }) => {
                self.bind_prenex_relation_variables_in_argument(argument);
            }
            data!(TermSyntax::NuhiTermset { termset, .. }) => {
                self.bind_prenex_relation_variables(termset);
            }
            data!(TermSyntax::GekNuhiTermset {
                terms,
                gik_terms,
                ..
            }) => {
                self.bind_prenex_relation_variables(terms);
                self.bind_prenex_relation_variables(gik_terms);
            }
            data!(TermSyntax::Cehe {
                leading_terms,
                trailing_terms,
                ..
            })
            | data!(TermSyntax::Pehe {
                leading_terms,
                trailing_terms,
                ..
            })
            | data!(TermSyntax::Connected {
                leading_terms,
                trailing_terms,
                ..
            }) => {
                self.bind_prenex_relation_variables(leading_terms);
                self.bind_prenex_relation_variables(trailing_terms);
            }
            data!(TermSyntax::BoConnected {
                leading_terms,
                trailing_term,
                ..
            }) => {
                self.bind_prenex_relation_variables(leading_terms);
                self.bind_prenex_relation_variables_in_term(trailing_term);
            }
            _ => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn bind_prenex_relation_variables_in_argument(&mut self, argument: &'tree ArgumentSyntax) {
        match argument.as_data() {
            data!(ArgumentSyntax::Descriptor(descriptor)) => {
                self.bind_prenex_relation_variables_in_descriptor(descriptor);
            }
            data!(ArgumentSyntax::ConnectedDescriptor(descriptor)) => {
                if let Some(relation) = descriptor.relation.as_deref() {
                    self.bind_prenex_relation_variable_relation(relation);
                }
            }
            data!(ArgumentSyntax::Quantified { inner_argument, .. })
            | data!(ArgumentSyntax::RelativeClause {
                base_argument: inner_argument,
                ..
            })
            | data!(ArgumentSyntax::Vuho {
                base_argument: inner_argument,
                ..
            })
            | data!(ArgumentSyntax::Tagged { inner_argument, .. })
            | data!(ArgumentSyntax::NaheBo { inner_argument, .. })
            | data!(ArgumentSyntax::Nahe { inner_argument, .. })
            | data!(ArgumentSyntax::Lahe { inner_argument, .. })
            | data!(ArgumentSyntax::Ke { inner_argument, .. }) => {
                self.bind_prenex_relation_variables_in_argument(inner_argument);
            }
            data!(ArgumentSyntax::TermWrapped { inner_term, .. }) => {
                self.bind_prenex_relation_variables_in_term(inner_term);
            }
            data!(ArgumentSyntax::Connected {
                leading_argument,
                trailing_argument,
                ..
            })
            | data!(ArgumentSyntax::Bo {
                leading_argument,
                trailing_argument,
                ..
            })
            | data!(ArgumentSyntax::Gek {
                leading_argument,
                trailing_argument,
                ..
            }) => {
                self.bind_prenex_relation_variables_in_argument(leading_argument);
                self.bind_prenex_relation_variables_in_argument(trailing_argument);
            }
            _ => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn bind_prenex_relation_variables_in_descriptor(
        &mut self,
        descriptor: &'tree DescriptorSyntax,
    ) {
        if let Some(relation) = descriptor.relation.as_deref() {
            self.bind_prenex_relation_variable_relation(relation);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn bind_prenex_relation_variable_relation(&mut self, relation: &'tree RelationSyntax) {
        if let data!(RelationSyntax::Base(word)) = relation.as_data()
            && let Some(cmavo @ (Cmavo::Buha | Cmavo::Buhe | Cmavo::Buhi)) = word.cmavo()
        {
            let target = self
                .index
                .relation_node_id(relation)
                .expect("prenex relation variable belongs to indexed syntax tree");
            self.relation_variable_bindings.insert(cmavo, target);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_term(&mut self, term: &'tree TermSyntax) {
        match term.as_data() {
            data!(TermSyntax::Argument(argument)) | data!(TermSyntax::Fa { argument, .. }) => {
                self.visit_argument(argument)
            }
            data!(TermSyntax::Tagged {
                tense_modal,
                argument,
            }) => {
                if let Some(tense_modal) = tense_modal.as_deref() {
                    self.visit_tense_modal(tense_modal);
                }
                self.visit_argument(argument);
            }
            data!(TermSyntax::JaiTagged { tag, argument, .. }) => {
                if let Some(tense_modal) = tag.as_deref() {
                    self.visit_tense_modal(tense_modal);
                }
                self.visit_argument(argument);
            }
            data!(TermSyntax::NuhiTermset { termset, .. }) => self.visit_terms(termset),
            data!(TermSyntax::GekNuhiTermset {
                terms,
                gik_terms,
                ..
            }) => {
                self.visit_terms(terms);
                self.visit_terms(gik_terms);
            }
            data!(TermSyntax::Cehe {
                leading_terms,
                trailing_terms,
                ..
            })
            | data!(TermSyntax::Pehe {
                leading_terms,
                trailing_terms,
                ..
            })
            | data!(TermSyntax::Connected {
                leading_terms,
                trailing_terms,
                ..
            }) => {
                self.visit_terms(leading_terms);
                self.visit_terms(trailing_terms);
            }
            data!(TermSyntax::BoConnected {
                leading_terms,
                trailing_term,
                ..
            }) => {
                self.visit_terms(leading_terms);
                self.visit_term(trailing_term);
            }
            data!(TermSyntax::FihoiAdverbial { subsentence, .. })
            | data!(TermSyntax::SoiAdverbial { subsentence, .. }) => {
                self.visit_subsentence(subsentence);
            }
            data!(TermSyntax::NoihaAdverbial {
                tail_elements,
                relation,
                relative_clauses,
                ..
            })
            | data!(TermSyntax::PoihaBrigahi {
                tail_elements,
                relation,
                relative_clauses,
                ..
            }) => {
                self.visit_argument_tail_elements(tail_elements, None);
                if let Some(relation) = relation.as_deref() {
                    self.visit_relation(relation);
                }
                for relative_clause in relative_clauses {
                    self.visit_relative_clause_without_head(relative_clause);
                }
            }
            data!(TermSyntax::NaKu { .. }) | data!(TermSyntax::BareNa(..)) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_argument(&mut self, argument: &'tree ArgumentSyntax) {
        let argument_id = self
            .index
            .argument_node_id(argument)
            .expect("argument belongs to indexed syntax tree");
        match argument.as_data() {
            data!(ArgumentSyntax::Koha(koha)) => {
                let cmavo = koha.cmavo();
                let resolved_target = self.resolve_koha(
                    argument_id,
                    cmavo,
                    koha_subscript_index(&koha.free_modifiers),
                );
                self.visit_free_modifiers(&koha.free_modifiers);
                if let Some(target) = resolved_target {
                    self.note_argument_mention_with_availability(argument_id, target, true);
                } else if cmavo.is_some_and(koha_records_self_mention) {
                    self.note_self_argument_mention_with_availability(
                        argument_id,
                        cmavo.is_some_and(koha_mention_available_to_ri),
                    );
                }
            }
            data!(ArgumentSyntax::Letter { letter, .. }) => {
                if let Some(base_letter) = letter_pro_sumti_base(letter) {
                    if let Some(target) = self.resolve_letter_target(&base_letter) {
                        self.add_edge(
                            ReferenceKind::Letter,
                            argument_id.0,
                            target_resolved_node(target.0),
                            "letteral pro-sumti resolves to the latest argument with the same initial letter",
                        );
                        self.note_argument_mention_with_availability(argument_id, target, false);
                    } else {
                        self.note_self_argument_mention_with_availability(argument_id, false);
                    }
                } else {
                    self.note_self_argument_mention_with_availability(argument_id, false);
                }
            }
            data!(ArgumentSyntax::RelativeClause {
                base_argument,
                relative_clauses,
                ..
            }) => {
                self.visit_argument(base_argument);
                let base_id = self
                    .index
                    .argument_node_id(base_argument)
                    .expect("base argument belongs to indexed syntax tree");
                self.record_wrapped_koha_reference(argument_id, base_argument);
                for relative_clause in relative_clauses {
                    self.visit_relative_clause(argument_id, base_id, relative_clause);
                }
                self.note_self_argument_mention(argument_id);
            }
            data!(ArgumentSyntax::Vuho {
                base_argument,
                relative_clauses,
                connected_argument,
                ..
            }) => {
                self.visit_argument(base_argument);
                for relative_clause in relative_clauses {
                    self.visit_relative_clause(argument_id, argument_id, relative_clause);
                }
                if let Some(connected) = connected_argument.as_deref() {
                    self.visit_argument(&connected.argument);
                }
                self.note_self_argument_mention(argument_id);
            }
            data!(ArgumentSyntax::Quantified {
                quantifier,
                inner_argument,
            }) => {
                self.visit_quantifier(quantifier);
                self.visit_argument(inner_argument);
                self.note_self_argument_mention(argument_id);
            }
            data!(ArgumentSyntax::Tagged {
                tag,
                inner_argument,
            }) => {
                self.visit_argument_tag(tag);
                self.visit_argument(inner_argument);
                self.note_self_argument_mention(argument_id);
            }
            data!(ArgumentSyntax::NaheBo { inner_argument, .. })
            | data!(ArgumentSyntax::Nahe { inner_argument, .. })
            | data!(ArgumentSyntax::Lahe { inner_argument, .. })
            | data!(ArgumentSyntax::Ke { inner_argument, .. }) => {
                self.visit_argument(inner_argument);
                self.note_self_argument_mention_with_availability(
                    argument_id,
                    !argument_wraps_ri(argument),
                );
            }
            data!(ArgumentSyntax::BridiDescription { subsentence, .. }) => {
                self.visit_subsentence(subsentence);
                self.note_self_argument_mention(argument_id);
            }
            data!(ArgumentSyntax::TermWrapped { inner_term, .. }) => {
                self.visit_term(inner_term);
                self.note_self_argument_mention(argument_id);
            }
            data!(ArgumentSyntax::Connected {
                leading_argument,
                trailing_argument,
                ..
            })
            | data!(ArgumentSyntax::Bo {
                leading_argument,
                trailing_argument,
                ..
            })
            | data!(ArgumentSyntax::Gek {
                leading_argument,
                trailing_argument,
                ..
            }) => {
                self.visit_argument(leading_argument);
                self.visit_argument(trailing_argument);
                self.note_self_argument_mention(argument_id);
            }
            data!(ArgumentSyntax::Descriptor(descriptor)) => {
                if let Some(outer_quantifier) = descriptor.outer_quantifier.as_deref() {
                    self.visit_quantifier(outer_quantifier);
                }
                self.visit_argument_tail_elements(&descriptor.tail_elements, None);
                if let Some(relation) = descriptor.relation.as_deref() {
                    self.visit_relation(relation);
                }
                for relative_clause in &descriptor.relative_clauses {
                    self.visit_relative_clause(argument_id, argument_id, relative_clause);
                }
                self.note_self_argument_mention(argument_id);
            }
            data!(ArgumentSyntax::ConnectedDescriptor(descriptor)) => {
                self.visit_argument_tail_elements(&descriptor.tail_elements, None);
                if let Some(relation) = descriptor.relation.as_deref() {
                    self.visit_relation(relation);
                }
                for relative_clause in &descriptor.relative_clauses {
                    self.visit_relative_clause(argument_id, argument_id, relative_clause);
                }
                self.note_self_argument_mention(argument_id);
            }
            data!(ArgumentSyntax::RelationVocative {
                leading_relative_clauses,
                relation,
                trailing_relative_clauses,
            }) => {
                for relative_clause in leading_relative_clauses {
                    self.visit_relative_clause(argument_id, argument_id, relative_clause);
                }
                self.visit_relation(relation);
                for relative_clause in trailing_relative_clauses {
                    self.visit_relative_clause(argument_id, argument_id, relative_clause);
                }
                self.note_self_argument_mention(argument_id);
            }
            data!(ArgumentSyntax::MathExpression { expression, .. }) => {
                self.visit_math_expression(expression);
                self.note_self_argument_mention(argument_id);
            }
            data!(ArgumentSyntax::Quote(quote)) => {
                self.visit_quote(quote);
                self.note_self_argument_mention(argument_id);
            }
            data!(ArgumentSyntax::NaKu { .. })
            | data!(ArgumentSyntax::Zohe { .. })
            | data!(ArgumentSyntax::Name { .. })
            | data!(ArgumentSyntax::Cmevla(..)) => {
                self.note_self_argument_mention(argument_id);
            }
        }
        self.note_letter_antecedent(argument_id, argument);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_quote(&mut self, quote: &'tree QuoteSyntax) {
        match quote.as_data() {
            data!(QuoteSyntax::Lu { text, .. }) => self.visit_text(text),
            data!(QuoteSyntax::Zo(..))
            | data!(QuoteSyntax::ZohOi(..))
            | data!(QuoteSyntax::Zoi(..))
            | data!(QuoteSyntax::Lohu(..)) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_fragment(&mut self, fragment: &'tree FragmentSyntax) {
        match fragment.as_data() {
            data!(FragmentSyntax::Prenex { terms, .. })
            | data!(FragmentSyntax::Term { terms, .. }) => self.visit_terms(terms),
            data!(FragmentSyntax::BeLink {
                first_argument,
                bei_links,
                ..
            }) => {
                if let Some(argument) = first_argument.as_deref() {
                    self.visit_argument(argument);
                }
                self.visit_bei_links(bei_links);
            }
            data!(FragmentSyntax::BeiLink(bei_links)) => self.visit_bei_links(bei_links),
            data!(FragmentSyntax::RelativeClause(relative_clauses)) => {
                for relative_clause in relative_clauses {
                    self.visit_relative_clause_without_head(relative_clause);
                }
            }
            data!(FragmentSyntax::MathExpression(expression)) => {
                self.visit_math_expression(expression);
            }
            data!(FragmentSyntax::Relation(relation)) => {
                self.visit_relation(relation);
            }
            data!(FragmentSyntax::Ek(..))
            | data!(FragmentSyntax::Gihek(..))
            | data!(FragmentSyntax::Other(..))
            | data!(FragmentSyntax::Ijek { .. }) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_bei_links(&mut self, bei_links: &'tree [BeiLinkSyntax]) {
        for link in bei_links {
            if let Some(argument) = link.argument.as_deref() {
                self.visit_argument(argument);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_argument_tail_elements(
        &mut self,
        tail_elements: &'tree [ArgumentTailElementSyntax],
        fallback_relative_head: Option<ArgumentNodeId>,
    ) {
        let mut current_relative_head = fallback_relative_head;
        for tail_element in tail_elements {
            match tail_element.as_data() {
                data!(ArgumentTailElementSyntax::Argument(argument)) => {
                    self.visit_argument(argument);
                    current_relative_head = self.index.argument_node_id(argument);
                }
                data!(ArgumentTailElementSyntax::RelativeClauses(relative_clauses)) => {
                    for relative_clause in relative_clauses {
                        if let Some(base_id) = current_relative_head {
                            self.visit_relative_clause(base_id, base_id, relative_clause);
                        } else {
                            self.visit_relative_clause_without_head(relative_clause);
                        }
                    }
                }
                data!(ArgumentTailElementSyntax::Quantifier(quantifier)) => {
                    self.visit_quantifier(quantifier);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_relative_clause_without_head(&mut self, relative_clause: &'tree RelativeClauseSyntax) {
        match relative_clause.as_data() {
            data!(RelativeClauseSyntax::Goi(goi)) => self.visit_argument(&goi.argument),
            data!(RelativeClauseSyntax::Noi { subsentence, .. })
            | data!(RelativeClauseSyntax::Poi { subsentence, .. }) => {
                self.visit_subsentence(subsentence);
            }
            data!(RelativeClauseSyntax::Zihe { inner, .. })
            | data!(RelativeClauseSyntax::Connected { inner, .. }) => {
                self.visit_relative_clause_without_head(inner);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_argument_tag(&mut self, tag: &'tree ArgumentTagSyntax) {
        match tag.as_data() {
            data!(ArgumentTagSyntax::TenseModal(tense_modal)) => {
                self.visit_tense_modal(tense_modal);
            }
            data!(ArgumentTagSyntax::Fa(..)) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_quantifier(&mut self, quantifier: &'tree QuantifierSyntax) {
        match quantifier.as_data() {
            data!(QuantifierSyntax::Vei {
                math_expression,
                ..
            }) => self.visit_math_expression(math_expression),
            data!(QuantifierSyntax::Number { .. }) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_math_expression(&mut self, expression: &'tree MathExpressionSyntax) {
        match expression.as_data() {
            data!(MathExpressionSyntax::Number(quantifier)) => self.visit_quantifier(quantifier),
            data!(MathExpressionSyntax::Vei {
                inner_expression,
                ..
            })
            | data!(MathExpressionSyntax::Lahe {
                inner_expression,
                ..
            }) => self.visit_math_expression(inner_expression),
            data!(MathExpressionSyntax::Gek {
                left_expression,
                right_expression,
                ..
            })
            | data!(MathExpressionSyntax::Connected {
                left_expression,
                right_expression,
                ..
            }) => {
                self.visit_math_expression(left_expression);
                self.visit_math_expression(right_expression);
            }
            data!(MathExpressionSyntax::Forethought {
                operator,
                operands,
                ..
            }) => {
                self.visit_math_operator(operator);
                for operand in operands {
                    self.visit_math_expression(operand);
                }
            }
            data!(MathExpressionSyntax::ReversePolish {
                operands,
                operators,
                ..
            }) => {
                for operand in operands {
                    self.visit_math_expression(operand);
                }
                for operator in operators {
                    self.visit_math_operator(operator);
                }
            }
            data!(MathExpressionSyntax::Nihe { relation, .. }) => {
                self.visit_relation(relation);
            }
            data!(MathExpressionSyntax::Mohe { argument, .. }) => {
                self.visit_argument(argument);
            }
            data!(MathExpressionSyntax::Johi { expressions, .. }) => {
                for expression in expressions.iter() {
                    self.visit_math_expression(expression);
                }
            }
            data!(MathExpressionSyntax::Binary {
                left_expression,
                operator,
                right_expression,
            })
            | data!(MathExpressionSyntax::Bihe {
                left_expression,
                operator,
                right_expression,
                ..
            }) => {
                self.visit_math_expression(left_expression);
                self.visit_math_operator(operator);
                self.visit_math_expression(right_expression);
            }
            data!(MathExpressionSyntax::Letter { .. }) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_math_operator(&mut self, operator: &'tree MathOperatorSyntax) {
        match operator.as_data() {
            data!(MathOperatorSyntax::Maho {
                math_expression,
                ..
            }) => self.visit_math_expression(math_expression),
            data!(MathOperatorSyntax::Se { inner_operator, .. })
            | data!(MathOperatorSyntax::Nahe { inner_operator, .. })
            | data!(MathOperatorSyntax::Ke { inner_operator, .. }) => {
                self.visit_math_operator(inner_operator);
            }
            data!(MathOperatorSyntax::Nahu { relation, .. }) => {
                self.visit_relation(relation);
            }
            data!(MathOperatorSyntax::Bo {
                left_operator,
                right_operator,
                ..
            })
            | data!(MathOperatorSyntax::Connected {
                left_operator,
                right_operator,
                ..
            }) => {
                self.visit_math_operator(left_operator);
                self.visit_math_operator(right_operator);
            }
            data!(MathOperatorSyntax::Vuhu(..)) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_tense_modal(&mut self, tense_modal: &'tree TenseModalSyntax) {
        match tense_modal.as_data() {
            data!(TenseModalSyntax::Composite { parts }) => {
                for part in &parts.value {
                    if let data!(CompositeTenseModalPartSyntax::Fiho(fiho)) = part.as_data() {
                        self.visit_relation(&fiho.relation);
                    }
                }
            }
            data!(TenseModalSyntax::Fiho { relation, .. }) => {
                self.visit_relation(relation);
            }
            data!(TenseModalSyntax::Pu(..))
            | data!(TenseModalSyntax::PuDistance { .. })
            | data!(TenseModalSyntax::TimeInterval(..))
            | data!(TenseModalSyntax::PuCaha { .. })
            | data!(TenseModalSyntax::SpaceDistance(..))
            | data!(TenseModalSyntax::SpaceDirection(..))
            | data!(TenseModalSyntax::SpaceMovement { .. })
            | data!(TenseModalSyntax::Simple { .. })
            | data!(TenseModalSyntax::Ki(..))
            | data!(TenseModalSyntax::Caha(..))
            | data!(TenseModalSyntax::Zaho(..))
            | data!(TenseModalSyntax::Interval { .. }) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_free_modifiers(&mut self, free_modifiers: &'tree [FreeModifierSyntax]) {
        for free_modifier in free_modifiers {
            self.visit_free_modifier(free_modifier);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_free_modifier(&mut self, free_modifier: &'tree FreeModifierSyntax) {
        match free_modifier.as_data() {
            data!(FreeModifierSyntax::Sei {
                terms,
                relation,
                ..
            }) => {
                self.visit_terms(terms);
                self.visit_relation(relation);
            }
            data!(FreeModifierSyntax::To { text, .. }) => self.visit_text(text),
            data!(FreeModifierSyntax::Xi { expression, .. }) => {
                self.visit_math_expression(expression);
            }
            data!(FreeModifierSyntax::Soi {
                leading_argument,
                trailing_argument,
                ..
            }) => {
                self.visit_argument(leading_argument);
                if let Some(argument) = trailing_argument.as_deref() {
                    self.visit_argument(argument);
                }
            }
            data!(FreeModifierSyntax::Vocative { argument, .. }) => {
                if let Some(argument) = argument.as_deref() {
                    self.visit_argument(argument);
                }
            }
            data!(FreeModifierSyntax::Mai { .. })
            | data!(FreeModifierSyntax::Replacement { .. }) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_relative_clause(
        &mut self,
        assignment_head_id: ArgumentNodeId,
        reference_head_id: ArgumentNodeId,
        relative_clause: &'tree jbotci_syntax::ast::RelativeClauseSyntax,
    ) {
        match relative_clause.as_data() {
            data!(jbotci_syntax::ast::RelativeClauseSyntax::Goi(goi)) => {
                self.visit_goi_clause(assignment_head_id, goi);
            }
            data!(jbotci_syntax::ast::RelativeClauseSyntax::Noi { subsentence, .. })
            | data!(jbotci_syntax::ast::RelativeClauseSyntax::Poi { subsentence, .. }) => {
                self.relative_heads.push(reference_head_id);
                self.visit_subsentence(subsentence);
                self.relative_heads.pop();
            }
            data!(jbotci_syntax::ast::RelativeClauseSyntax::Zihe { inner, .. })
            | data!(jbotci_syntax::ast::RelativeClauseSyntax::Connected { inner, .. }) => {
                self.visit_relative_clause(assignment_head_id, reference_head_id, inner);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_goi_clause(&mut self, base_id: ArgumentNodeId, goi: &'tree GoiRelativeClauseSyntax) {
        self.visit_argument(&goi.argument);
        let goi_argument_id = self
            .index
            .argument_node_id(&goi.argument)
            .expect("goi argument belongs to indexed syntax tree");
        let source = goi_argument_id.0;
        self.add_edge(
            ReferenceKind::GoiAssignment,
            source,
            target_resolved_node(base_id.0),
            "GOI relative clause equates its argument with the relative-clause head",
        );
        if let Some(cmavo) = koha_assignable_cmavo(&goi.argument) {
            self.koha_bindings.insert(cmavo, base_id);
        } else if let Some(cmavo) = argument_koha_cmavo(
            self.index
                .argument(base_id)
                .expect("base argument id resolves"),
        ) {
            self.koha_bindings.insert(cmavo, goi_argument_id);
            self.add_edge(
                ReferenceKind::GoiAssignment,
                base_id.0,
                target_resolved_node(goi_argument_id.0),
                "GOI assigns the relative-clause head pro-sumti to its argument",
            );
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_abstraction(&mut self, abstraction: &'tree AbstractionSyntax) {
        let abstraction_id = self
            .index
            .abstraction_node_id(abstraction)
            .expect("abstraction belongs to indexed syntax tree");
        self.abstraction_stack.push(abstraction_id.0);
        self.visit_subsentence(&abstraction.subsentence);
        self.abstraction_stack.pop();
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_relation(&mut self, relation: &'tree RelationSyntax) {
        match relation.as_data() {
            data!(RelationSyntax::Connected {
                leading_relation,
                trailing_relation,
                ..
            })
            | data!(RelationSyntax::Co {
                leading_relation,
                trailing_relation,
                ..
            }) => {
                self.visit_relation(leading_relation);
                self.visit_relation(trailing_relation);
            }
            data!(RelationSyntax::Bo {
                leading_relation,
                bo_tense_modal,
                trailing_relation,
                ..
            }) => {
                self.visit_relation(leading_relation);
                if let Some(tense_modal) = bo_tense_modal.as_deref() {
                    self.visit_tense_modal(tense_modal);
                }
                self.visit_relation(trailing_relation);
            }
            data!(RelationSyntax::Na { inner_relation, .. })
            | data!(RelationSyntax::Se { inner_relation, .. }) => {
                self.visit_relation(inner_relation);
            }
            data!(RelationSyntax::Ke {
                ke_tense_modal,
                relation: inner_relation,
                ..
            }) => {
                if let Some(tense_modal) = ke_tense_modal.as_deref() {
                    self.visit_tense_modal(tense_modal);
                }
                self.visit_relation(inner_relation);
            }
            data!(RelationSyntax::TenseModal {
                tense_modal,
                inner_relation,
            }) => {
                self.visit_tense_modal(tense_modal);
                self.visit_relation(inner_relation);
            }
            data!(RelationSyntax::Base(word)) => {
                if let Some(label) = broda_label(word.core_word()) {
                    self.resolve_broda_relation(relation, label);
                } else {
                    self.resolve_goha_relation(relation, word.cmavo());
                }
            }
            data!(RelationSyntax::Guha {
                leading_predicate,
                trailing_predicate,
                ..
            }) => {
                self.visit_predicate(leading_predicate);
                self.visit_predicate(trailing_predicate);
            }
            data!(RelationSyntax::Abstraction(abstraction)) => {
                self.visit_abstraction(abstraction);
            }
            data!(RelationSyntax::Compound(units)) => {
                for unit in units.iter() {
                    self.visit_relation_unit(unit);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_relation_unit(&mut self, unit: &'tree RelationUnitSyntax) {
        match unit.as_data() {
            data!(RelationUnitSyntax::Goha { goha, .. }) => {
                self.resolve_goha_unit(unit, goha.cmavo());
            }
            data!(RelationUnitSyntax::Word(word)) => {
                if let Some(label) = broda_label(word.core_word()) {
                    self.resolve_broda_unit(unit, label);
                }
            }
            data!(RelationUnitSyntax::Se { inner_unit, .. })
            | data!(RelationUnitSyntax::Nahe { inner_unit, .. }) => {
                self.visit_relation_unit(inner_unit);
            }
            data!(RelationUnitSyntax::Ke {
                ke_tense_modal,
                relation,
                ..
            }) => {
                if let Some(tense_modal) = ke_tense_modal.as_deref() {
                    self.visit_tense_modal(tense_modal);
                }
                self.visit_relation(relation);
            }
            data!(RelationUnitSyntax::Wrapped(relation)) => self.visit_relation(relation),
            data!(RelationUnitSyntax::Bo {
                leading_unit,
                bo_tense_modal,
                trailing_unit,
                ..
            }) => {
                self.visit_relation_unit(leading_unit);
                if let Some(tense_modal) = bo_tense_modal.as_deref() {
                    self.visit_tense_modal(tense_modal);
                }
                self.visit_relation_unit(trailing_unit);
            }
            data!(RelationUnitSyntax::Connected {
                leading_unit,
                trailing_unit,
                ..
            }) => {
                self.visit_relation_unit(leading_unit);
                self.visit_relation_unit(trailing_unit);
            }
            data!(RelationUnitSyntax::SelbriRelativeClause {
                base,
                selbri_relative_clauses,
            }) => {
                self.visit_relation_unit(base);
                for relative_clause in selbri_relative_clauses {
                    self.visit_relation(&relative_clause.relation);
                }
            }
            data!(RelationUnitSyntax::Jai {
                tense_modal,
                inner_unit,
                ..
            }) => {
                if let Some(tense_modal) = tense_modal.as_deref() {
                    self.visit_tense_modal(tense_modal);
                }
                self.visit_relation_unit(inner_unit)
            }
            data!(RelationUnitSyntax::Be {
                base,
                first_argument,
                bei_links,
                ..
            })
            | data!(RelationUnitSyntax::PreposedBe {
                base,
                first_argument,
                bei_links,
                ..
            }) => {
                self.visit_relation_unit(base);
                if let Some(argument) = first_argument.as_deref() {
                    self.visit_argument(argument);
                }
                for link in bei_links {
                    if let Some(argument) = link.argument.as_deref() {
                        self.visit_argument(argument);
                    }
                }
            }
            data!(RelationUnitSyntax::Abstraction(abstraction)) => {
                self.visit_abstraction(abstraction);
            }
            data!(RelationUnitSyntax::Me { argument, .. }) => self.visit_argument(argument),
            data!(RelationUnitSyntax::Luhei { text, .. }) => self.visit_text(text),
            data!(RelationUnitSyntax::Cei { base, assignments }) => {
                self.visit_relation_unit(base);
                for assignment in assignments {
                    self.visit_relation_unit(&assignment.relation_unit);
                    if let Some(label) = relation_unit_assignment_label(&assignment.relation_unit) {
                        if let Some(predicate_id) = self.current_predicate {
                            self.cei_predicate_bindings.insert(label, predicate_id);
                        }
                    }
                    if let Some(predicate_id) = self.current_predicate {
                        let assignment_id = self
                            .index
                            .relation_unit_node_id(&assignment.relation_unit)
                            .expect("CEI assignment belongs to indexed syntax tree");
                        self.add_edge(
                            ReferenceKind::CeiAssignment,
                            assignment_id.0,
                            target_resolved_node(predicate_id.0),
                            "CEI assigns a pro-bridi word to the enclosing predicate",
                        );
                    }
                }
            }
            data!(RelationUnitSyntax::Mehoi(..))
            | data!(RelationUnitSyntax::Gohoi(..))
            | data!(RelationUnitSyntax::Muhoi(..))
            | data!(RelationUnitSyntax::Moi { .. }) => {}
            data!(RelationUnitSyntax::Nuha { math_operator, .. }) => {
                self.visit_math_operator(math_operator);
            }
            data!(RelationUnitSyntax::Xohi { tag, .. }) => {
                self.visit_tense_modal(tag);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn note_self_argument_mention(&mut self, source: ArgumentNodeId) {
        self.note_self_argument_mention_with_availability(source, true);
    }

    #[requires(true)]
    #[ensures(true)]
    fn note_self_argument_mention_with_availability(
        &mut self,
        source: ArgumentNodeId,
        available_to_ri: bool,
    ) {
        self.note_argument_mention_with_availability(source, source, available_to_ri);
    }

    #[requires(true)]
    #[ensures(self.predicate_mentions.len() == old(self.predicate_mentions.len()) + 1)]
    fn note_predicate_mention(&mut self, source: RawSyntaxNodeId) {
        let position = self
            .index
            .metadata(source)
            .and_then(|metadata| {
                metadata
                    .source_spans
                    .first()
                    .map(|span| span.byte_start)
                    .or(Some(metadata.preorder))
            })
            .unwrap_or(source.0);
        self.predicate_mentions.push(NodeMention {
            source,
            target: source,
            position,
        });
    }

    #[requires(true)]
    #[ensures(self.argument_mentions.len() == old(self.argument_mentions.len()) + 1)]
    fn note_argument_mention(&mut self, source: ArgumentNodeId, target: ArgumentNodeId) {
        self.note_argument_mention_with_availability(source, target, true);
    }

    #[requires(true)]
    #[ensures(self.argument_mentions.len() == old(self.argument_mentions.len()) + 1)]
    fn note_argument_mention_with_availability(
        &mut self,
        source: ArgumentNodeId,
        target: ArgumentNodeId,
        available_to_ri: bool,
    ) {
        let position = self.argument_mention_position(source);
        self.argument_mentions.push(ArgumentMention {
            source,
            target,
            position,
            available_to_ri,
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn note_letter_antecedent(&mut self, source: ArgumentNodeId, argument: &'tree ArgumentSyntax) {
        let Some(base_letter) = argument_letter_base(argument) else {
            return;
        };
        let position = self.argument_mention_position(source);
        self.letter_mentions
            .entry(base_letter)
            .or_default()
            .push(ArgumentMention {
                source,
                target: source,
                position,
                available_to_ri: false,
            });
    }

    #[requires(!base_letter.is_empty())]
    #[ensures(true)]
    fn resolve_letter_target(&self, base_letter: &str) -> Option<ArgumentNodeId> {
        self.letter_mentions
            .get(base_letter)
            .and_then(|mentions| mentions.last())
            .map(|mention| mention.target)
    }

    #[requires(true)]
    #[ensures(true)]
    fn record_wrapped_koha_reference(
        &mut self,
        source: ArgumentNodeId,
        base_argument: &'tree ArgumentSyntax,
    ) {
        let Some((cmavo, subscript)) = argument_koha_cmavo_with_subscript(base_argument) else {
            return;
        };
        match cmavo {
            Cmavo::Ri => {
                if let Some(target) =
                    self.latest_argument_mention_target_before(source, subscript.unwrap_or(1))
                {
                    self.add_edge(
                        ReferenceKind::Ri,
                        source.0,
                        target_resolved_node(target.0),
                        "wrapped ri exposes the complete argument as a reference source",
                    );
                }
            }
            Cmavo::Keha => {
                if let Some(target) = subscript
                    .unwrap_or(1)
                    .checked_sub(1)
                    .and_then(|index| self.relative_heads.iter().rev().nth(index).copied())
                {
                    self.add_edge(
                        ReferenceKind::Keha,
                        source.0,
                        target_resolved_node(target.0),
                        "wrapped ke'a exposes the complete argument as a reference source",
                    );
                }
            }
            _ => {}
        }
    }

    #[requires(recency_index > 0)]
    #[ensures(true)]
    fn predicate_mention_target_by_recency(&self, recency_index: usize) -> Option<RawSyntaxNodeId> {
        let mut candidates: Vec<_> = self.predicate_mentions.iter().collect();
        candidates.sort_by_key(|mention| (mention.position, mention.source.0));
        candidates
            .into_iter()
            .rev()
            .nth(recency_index - 1)
            .map(|mention| mention.target)
    }

    #[requires(true)]
    #[ensures(true)]
    fn argument_mention_position(&self, source: ArgumentNodeId) -> usize {
        self.index
            .metadata(source.0)
            .and_then(|metadata| metadata.source_spans.first().map(|span| span.byte_start))
            .or_else(|| {
                self.index
                    .metadata(source.0)
                    .map(|metadata| metadata.preorder)
            })
            .unwrap_or(source.0.0)
    }

    #[requires(true)]
    #[ensures(true)]
    fn latest_argument_mention_target_before(
        &self,
        source: ArgumentNodeId,
        recency_index: usize,
    ) -> Option<ArgumentNodeId> {
        if recency_index == 0 {
            return None;
        }
        let source_position = self.argument_mention_position(source);
        let mut candidates: Vec<_> = self
            .argument_mentions
            .iter()
            .filter(|mention| mention.available_to_ri && mention.position < source_position)
            .collect();
        candidates.sort_by_key(|mention| (mention.position, mention.source.0.0));
        candidates
            .into_iter()
            .rev()
            .nth(recency_index - 1)
            .map(|mention| mention.target)
    }

    #[requires(true)]
    #[ensures(true)]
    fn resolve_koha(
        &mut self,
        source: ArgumentNodeId,
        cmavo: Option<Cmavo>,
        subscript: Option<usize>,
    ) -> Option<ArgumentNodeId> {
        let Some(cmavo) = cmavo else {
            return None;
        };
        match cmavo {
            Cmavo::Ri => {
                let target_argument =
                    self.latest_argument_mention_target_before(source, subscript.unwrap_or(1));
                let target = target_argument
                    .map(|argument| target_resolved_node(argument.0))
                    .unwrap_or_else(|| target_unresolved("ri has no prior sumti"));
                self.add_edge(
                    ReferenceKind::Ri,
                    source.0,
                    target,
                    "ri repeats the previous complete sumti",
                );
                target_argument
            }
            Cmavo::Cehu => {
                let target = subscript
                    .unwrap_or(1)
                    .checked_sub(1)
                    .and_then(|index| self.abstraction_stack.iter().rev().nth(index).copied())
                    .map(target_resolved_node)
                    .unwrap_or_else(|| target_unresolved("ce'u is outside an abstraction"));
                self.add_edge(
                    ReferenceKind::Cehu,
                    source.0,
                    target,
                    "ce'u refers to the current abstraction",
                );
                None
            }
            Cmavo::Ra => {
                self.add_edge(
                    ReferenceKind::Ra,
                    source.0,
                    target_vague(VagueReferenceKind::DistantArgument),
                    "ra is intentionally vague and is not resolved heuristically",
                );
                None
            }
            Cmavo::Ru => {
                self.add_edge(
                    ReferenceKind::Ru,
                    source.0,
                    target_vague(VagueReferenceKind::DistantArgument),
                    "ru is intentionally vague and is not resolved heuristically",
                );
                None
            }
            Cmavo::Keha => {
                let target = subscript
                    .unwrap_or(1)
                    .checked_sub(1)
                    .and_then(|index| self.relative_heads.iter().rev().nth(index).copied())
                    .map(|argument| target_resolved_node(argument.0))
                    .unwrap_or_else(|| target_unresolved("ke'a is outside a relative clause"));
                self.add_edge(
                    ReferenceKind::Keha,
                    source.0,
                    target,
                    "ke'a refers to the current relative-clause head",
                );
                None
            }
            Cmavo::Dei | Cmavo::Dihu | Cmavo::Dihe => {
                if cmavo == Cmavo::Dihe {
                    self.pending_next_utterance_sources.push(source.0);
                    return None;
                }
                let target_node = match cmavo {
                    Cmavo::Dei => self.current_utterance,
                    Cmavo::Dihu => self.utterance_history.last().copied(),
                    Cmavo::Dihe => None,
                    _ => None,
                };
                let target = target_node.map(target_resolved_node).unwrap_or_else(|| {
                    target_unresolved("utterance reference has no determinate target")
                });
                self.add_edge(
                    ReferenceKind::Utterance,
                    source.0,
                    target,
                    "utterance pro-sumti resolves to a neighboring utterance when determined by form",
                );
                None
            }
            Cmavo::Voha | Cmavo::Vohe | Cmavo::Vohi | Cmavo::Voho | Cmavo::Vohu => {
                let slot = voha_slot(cmavo);
                let target = slot
                    .and_then(|slot| {
                        self.current_predicate_frames
                            .first()
                            .copied()
                            .map(|frame| (frame, slot))
                    })
                    .and_then(|(frame, slot)| self.places.first_argument_for_place(frame, slot))
                    .map(|argument| target_resolved_node(argument.0))
                    .unwrap_or_else(|| {
                        target_unresolved(
                            "vo'a-series place is not filled in the current predicate",
                        )
                    });
                self.add_edge(
                    ReferenceKind::VohaSeries,
                    source.0,
                    target,
                    "vo'a-series refers to a place of the current bridi",
                );
                None
            }
            Cmavo::Da | Cmavo::De | Cmavo::Di => {
                if let Some(target) = self.da_bindings.get(&cmavo).copied() {
                    self.add_edge(
                        ReferenceKind::DaSeries,
                        source.0,
                        target_resolved_node(target.0),
                        "later da/de/di mentions refer to the active variable binding",
                    );
                    Some(target)
                } else {
                    self.da_bindings.insert(cmavo, source);
                    None
                }
            }
            _ => {
                if let Some(target) = self.koha_bindings.get(&cmavo).copied() {
                    self.add_edge(
                        ReferenceKind::Koha,
                        source.0,
                        target_resolved_node(target.0),
                        "KOhA resolves through an explicit GOI binding",
                    );
                    Some(target)
                } else {
                    None
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn resolve_goha_unit(&mut self, unit: &'tree RelationUnitSyntax, cmavo: Option<Cmavo>) {
        let Some(cmavo) = cmavo else {
            return;
        };
        let source = self
            .index
            .relation_unit_node_id(unit)
            .expect("GOhA unit belongs to indexed syntax tree");
        self.resolve_goha_source(source.0, cmavo);
    }

    #[requires(true)]
    #[ensures(true)]
    fn resolve_goha_relation(&mut self, relation: &'tree RelationSyntax, cmavo: Option<Cmavo>) {
        let Some(cmavo) = cmavo else {
            return;
        };
        let source = self
            .index
            .relation_node_id(relation)
            .expect("GOhA relation belongs to indexed syntax tree");
        self.resolve_goha_source(source.0, cmavo);
    }

    #[requires(true)]
    #[ensures(true)]
    fn resolve_goha_source(&mut self, source: RawSyntaxNodeId, cmavo: Cmavo) {
        match cmavo {
            Cmavo::Gohi => {
                let target = self
                    .predicate_mention_target_by_recency(1)
                    .map(target_resolved_node)
                    .unwrap_or_else(|| target_unresolved("go'i has no prior bridi"));
                self.add_edge(
                    ReferenceKind::GohaSeries,
                    source,
                    target,
                    "go'i repeats the previous bridi",
                );
            }
            Cmavo::Gohe => {
                let target = self
                    .predicate_mention_target_by_recency(2)
                    .map(target_resolved_node)
                    .unwrap_or_else(|| target_unresolved("go'e has no second-prior bridi"));
                self.add_edge(
                    ReferenceKind::GohaSeries,
                    source,
                    target,
                    "go'e repeats the second-prior bridi",
                );
            }
            Cmavo::Goha | Cmavo::Gohu | Cmavo::Goho => {
                self.add_edge(
                    ReferenceKind::GohaSeries,
                    source,
                    target_vague(VagueReferenceKind::Bridi),
                    "this GOhA form is context-sensitive and is not resolved heuristically",
                );
            }
            Cmavo::Nei => {
                let target = self
                    .discourse_predicate_stack
                    .last()
                    .copied()
                    .map(target_resolved_node)
                    .unwrap_or_else(|| target_unresolved("nei is outside a current bridi"));
                self.add_edge(
                    ReferenceKind::GohaSeries,
                    source,
                    target,
                    "nei refers to the current bridi",
                );
            }
            Cmavo::Noha => {
                let target = self
                    .predicate_stack
                    .iter()
                    .rev()
                    .nth(1)
                    .copied()
                    .map(target_resolved_node)
                    .unwrap_or_else(|| {
                        target_unresolved("no'a outer-bridi stack has no outer bridi")
                    });
                self.add_edge(
                    ReferenceKind::GohaSeries,
                    source,
                    target,
                    "no'a refers to an outer bridi",
                );
            }
            Cmavo::Buha | Cmavo::Buhe | Cmavo::Buhi => {
                if let Some(target) = self.relation_variable_bindings.get(&cmavo).copied() {
                    self.add_edge(
                        ReferenceKind::BrodaSeries,
                        source,
                        target_resolved_node(target.0),
                        "prenex binding resolves this pro-relation word",
                    );
                }
                let label = cmavo.canonical_text().to_owned();
                if let Some(target) = self.cei_predicate_bindings.get(&label).copied() {
                    self.add_edge(
                        ReferenceKind::BrodaSeries,
                        source,
                        target_resolved_node(target.0),
                        "CEI binding resolves this pro-predicate word",
                    );
                }
            }
            _ => {}
        }
    }

    #[requires(!label.is_empty())]
    #[ensures(true)]
    fn resolve_broda_unit(&mut self, unit: &'tree RelationUnitSyntax, label: String) {
        let source = self
            .index
            .relation_unit_node_id(unit)
            .expect("broda unit belongs to indexed syntax tree");
        if let Some(target) = self.cei_predicate_bindings.get(&label).copied() {
            self.add_edge(
                ReferenceKind::BrodaSeries,
                source.0,
                target_resolved_node(target.0),
                "CEI binding resolves this broda-series predicate",
            );
        }
    }

    #[requires(!label.is_empty())]
    #[ensures(true)]
    fn resolve_broda_relation(&mut self, relation: &'tree RelationSyntax, label: String) {
        let source = self
            .index
            .relation_node_id(relation)
            .expect("broda relation belongs to indexed syntax tree");
        if let Some(target) = self.cei_predicate_bindings.get(&label).copied() {
            self.add_edge(
                ReferenceKind::BrodaSeries,
                source.0,
                target_resolved_node(target.0),
                "CEI binding resolves this broda-series predicate",
            );
        }
    }

    #[requires(!rule.is_empty())]
    #[ensures(true)]
    fn add_edge(
        &mut self,
        kind: ReferenceKind,
        source: RawSyntaxNodeId,
        target: ReferenceTarget,
        rule: &str,
    ) {
        let id = ReferenceEdgeId(self.edges.len());
        if let ReferenceTarget::ResolvedNode(target_node) = target {
            self.edge_ids_by_target_node
                .entry(target_node)
                .or_default()
                .push(id);
        }
        self.edge_ids_by_source.entry(source).or_default().push(id);
        self.edges.push(ReferenceEdge {
            id,
            kind,
            source,
            target,
            rule: rule.to_owned(),
        });
    }
}

#[requires(true)]
#[ensures(true)]
fn statement_node_ref<'tree>(statement: &'tree StatementSyntax) -> SyntaxNodeRef<'tree> {
    match statement.as_data() {
        data!(StatementSyntax::Tuhe { .. }) => SyntaxNodeRef::StatementSyntaxTuhe(statement),
        data!(StatementSyntax::Prenex { .. }) => SyntaxNodeRef::StatementSyntaxPrenex(statement),
        data!(StatementSyntax::Predicate(..)) => SyntaxNodeRef::StatementSyntaxPredicate(statement),
        data!(StatementSyntax::Connected { .. }) => {
            SyntaxNodeRef::StatementSyntaxConnected(statement)
        }
        data!(StatementSyntax::PreIConnected { .. }) => {
            SyntaxNodeRef::StatementSyntaxPreIConnected(statement)
        }
        data!(StatementSyntax::Iau { .. }) => SyntaxNodeRef::StatementSyntaxIau(statement),
        data!(StatementSyntax::ExperimentalPredicateContinuation { .. }) => {
            SyntaxNodeRef::StatementSyntaxExperimentalPredicateContinuation(statement)
        }
        data!(StatementSyntax::Fragment(..)) => SyntaxNodeRef::StatementSyntaxFragment(statement),
    }
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail1_node_ref<'tree>(tail: &'tree PredicateTail1Syntax) -> SyntaxNodeRef<'tree> {
    SyntaxNodeRef::PredicateTail1Syntax(tail)
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail2_node_ref<'tree>(tail: &'tree PredicateTail2Syntax) -> SyntaxNodeRef<'tree> {
    SyntaxNodeRef::PredicateTail2Syntax(tail)
}

#[requires(true)]
#[ensures(true)]
fn predicate_tail3_node_ref<'tree>(tail: &'tree PredicateTail3Syntax) -> SyntaxNodeRef<'tree> {
    match tail.as_data() {
        data!(PredicateTail3Syntax::Relation { .. }) => {
            SyntaxNodeRef::PredicateTail3SyntaxRelation(tail)
        }
        data!(PredicateTail3Syntax::GekSentence(..)) => {
            SyntaxNodeRef::PredicateTail3SyntaxGekSentence(tail)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_node_ref<'tree>(relation: &'tree RelationSyntax) -> SyntaxNodeRef<'tree> {
    match relation.as_data() {
        data!(RelationSyntax::Connected { .. }) => SyntaxNodeRef::RelationSyntaxConnected(relation),
        data!(RelationSyntax::Co { .. }) => SyntaxNodeRef::RelationSyntaxCo(relation),
        data!(RelationSyntax::Bo { .. }) => SyntaxNodeRef::RelationSyntaxBo(relation),
        data!(RelationSyntax::Na { .. }) => SyntaxNodeRef::RelationSyntaxNa(relation),
        data!(RelationSyntax::Base(..)) => SyntaxNodeRef::RelationSyntaxBase(relation),
        data!(RelationSyntax::Se { .. }) => SyntaxNodeRef::RelationSyntaxSe(relation),
        data!(RelationSyntax::Ke { .. }) => SyntaxNodeRef::RelationSyntaxKe(relation),
        data!(RelationSyntax::TenseModal { .. }) => {
            SyntaxNodeRef::RelationSyntaxTenseModal(relation)
        }
        data!(RelationSyntax::Guha { .. }) => SyntaxNodeRef::RelationSyntaxGuha(relation),
        data!(RelationSyntax::Abstraction(..)) => {
            SyntaxNodeRef::RelationSyntaxAbstraction(relation)
        }
        data!(RelationSyntax::Compound(..)) => SyntaxNodeRef::RelationSyntaxCompound(relation),
    }
}

#[requires(true)]
#[ensures(true)]
fn relation_unit_node_ref<'tree>(unit: &'tree RelationUnitSyntax) -> SyntaxNodeRef<'tree> {
    match unit.as_data() {
        data!(RelationUnitSyntax::Word(..)) => SyntaxNodeRef::RelationUnitSyntaxWord(unit),
        data!(RelationUnitSyntax::Goha { .. }) => SyntaxNodeRef::RelationUnitSyntaxGoha(unit),
        data!(RelationUnitSyntax::Se { .. }) => SyntaxNodeRef::RelationUnitSyntaxSe(unit),
        data!(RelationUnitSyntax::Ke { .. }) => SyntaxNodeRef::RelationUnitSyntaxKe(unit),
        data!(RelationUnitSyntax::Nahe { .. }) => SyntaxNodeRef::RelationUnitSyntaxNahe(unit),
        data!(RelationUnitSyntax::Bo { .. }) => SyntaxNodeRef::RelationUnitSyntaxBo(unit),
        data!(RelationUnitSyntax::Connected { .. }) => {
            SyntaxNodeRef::RelationUnitSyntaxConnected(unit)
        }
        data!(RelationUnitSyntax::SelbriRelativeClause { .. }) => {
            SyntaxNodeRef::RelationUnitSyntaxSelbriRelativeClause(unit)
        }
        data!(RelationUnitSyntax::Wrapped(..)) => SyntaxNodeRef::RelationUnitSyntaxWrapped(unit),
        data!(RelationUnitSyntax::Jai { .. }) => SyntaxNodeRef::RelationUnitSyntaxJai(unit),
        data!(RelationUnitSyntax::Be { .. }) => SyntaxNodeRef::RelationUnitSyntaxBe(unit),
        data!(RelationUnitSyntax::PreposedBe { .. }) => {
            SyntaxNodeRef::RelationUnitSyntaxPreposedBe(unit)
        }
        data!(RelationUnitSyntax::Abstraction(..)) => {
            SyntaxNodeRef::RelationUnitSyntaxAbstraction(unit)
        }
        data!(RelationUnitSyntax::Me { .. }) => SyntaxNodeRef::RelationUnitSyntaxMe(unit),
        data!(RelationUnitSyntax::Mehoi(..)) => SyntaxNodeRef::RelationUnitSyntaxMehoi(unit),
        data!(RelationUnitSyntax::Gohoi(..)) => SyntaxNodeRef::RelationUnitSyntaxGohoi(unit),
        data!(RelationUnitSyntax::Muhoi(..)) => SyntaxNodeRef::RelationUnitSyntaxMuhoi(unit),
        data!(RelationUnitSyntax::Luhei { .. }) => SyntaxNodeRef::RelationUnitSyntaxLuhei(unit),
        data!(RelationUnitSyntax::Moi { .. }) => SyntaxNodeRef::RelationUnitSyntaxMoi(unit),
        data!(RelationUnitSyntax::Nuha { .. }) => SyntaxNodeRef::RelationUnitSyntaxNuha(unit),
        data!(RelationUnitSyntax::Xohi { .. }) => SyntaxNodeRef::RelationUnitSyntaxXohi(unit),
        data!(RelationUnitSyntax::Cei { .. }) => SyntaxNodeRef::RelationUnitSyntaxCei(unit),
    }
}

#[requires(true)]
#[ensures(true)]
fn term_node_ref<'tree>(term: &'tree TermSyntax) -> SyntaxNodeRef<'tree> {
    match term.as_data() {
        data!(TermSyntax::NuhiTermset { .. }) => SyntaxNodeRef::TermSyntaxNuhiTermset(term),
        data!(TermSyntax::GekNuhiTermset { .. }) => SyntaxNodeRef::TermSyntaxGekNuhiTermset(term),
        data!(TermSyntax::Cehe { .. }) => SyntaxNodeRef::TermSyntaxCehe(term),
        data!(TermSyntax::Pehe { .. }) => SyntaxNodeRef::TermSyntaxPehe(term),
        data!(TermSyntax::Argument(..)) => SyntaxNodeRef::TermSyntaxArgument(term),
        data!(TermSyntax::Fa { .. }) => SyntaxNodeRef::TermSyntaxFa(term),
        data!(TermSyntax::NaKu { .. }) => SyntaxNodeRef::TermSyntaxNaKu(term),
        data!(TermSyntax::BareNa(..)) => SyntaxNodeRef::TermSyntaxBareNa(term),
        data!(TermSyntax::NoihaAdverbial { .. }) => SyntaxNodeRef::TermSyntaxNoihaAdverbial(term),
        data!(TermSyntax::PoihaBrigahi { .. }) => SyntaxNodeRef::TermSyntaxPoihaBrigahi(term),
        data!(TermSyntax::FihoiAdverbial { .. }) => SyntaxNodeRef::TermSyntaxFihoiAdverbial(term),
        data!(TermSyntax::SoiAdverbial { .. }) => SyntaxNodeRef::TermSyntaxSoiAdverbial(term),
        data!(TermSyntax::JaiTagged { .. }) => SyntaxNodeRef::TermSyntaxJaiTagged(term),
        data!(TermSyntax::Tagged { .. }) => SyntaxNodeRef::TermSyntaxTagged(term),
        data!(TermSyntax::Connected { .. }) => SyntaxNodeRef::TermSyntaxConnected(term),
        data!(TermSyntax::BoConnected { .. }) => SyntaxNodeRef::TermSyntaxBoConnected(term),
    }
}

#[requires(true)]
#[ensures(true)]
fn argument_node_ref<'tree>(argument: &'tree ArgumentSyntax) -> SyntaxNodeRef<'tree> {
    match argument.as_data() {
        data!(ArgumentSyntax::Quote(..)) => SyntaxNodeRef::ArgumentSyntaxQuote(argument),
        data!(ArgumentSyntax::MathExpression { .. }) => {
            SyntaxNodeRef::ArgumentSyntaxMathExpression(argument)
        }
        data!(ArgumentSyntax::Letter { .. }) => SyntaxNodeRef::ArgumentSyntaxLetter(argument),
        data!(ArgumentSyntax::Quantified { .. }) => {
            SyntaxNodeRef::ArgumentSyntaxQuantified(argument)
        }
        data!(ArgumentSyntax::RelativeClause { .. }) => {
            SyntaxNodeRef::ArgumentSyntaxRelativeClause(argument)
        }
        data!(ArgumentSyntax::Vuho { .. }) => SyntaxNodeRef::ArgumentSyntaxVuho(argument),
        data!(ArgumentSyntax::BridiDescription { .. }) => {
            SyntaxNodeRef::ArgumentSyntaxBridiDescription(argument)
        }
        data!(ArgumentSyntax::NaKu { .. }) => SyntaxNodeRef::ArgumentSyntaxNaKu(argument),
        data!(ArgumentSyntax::Tagged { .. }) => SyntaxNodeRef::ArgumentSyntaxTagged(argument),
        data!(ArgumentSyntax::NaheBo { .. }) => SyntaxNodeRef::ArgumentSyntaxNaheBo(argument),
        data!(ArgumentSyntax::Nahe { .. }) => SyntaxNodeRef::ArgumentSyntaxNahe(argument),
        data!(ArgumentSyntax::TermWrapped { .. }) => {
            SyntaxNodeRef::ArgumentSyntaxTermWrapped(argument)
        }
        data!(ArgumentSyntax::Koha(..)) => SyntaxNodeRef::ArgumentSyntaxKoha(argument),
        data!(ArgumentSyntax::Zohe { .. }) => SyntaxNodeRef::ArgumentSyntaxZohe(argument),
        data!(ArgumentSyntax::Lahe { .. }) => SyntaxNodeRef::ArgumentSyntaxLahe(argument),
        data!(ArgumentSyntax::Connected { .. }) => SyntaxNodeRef::ArgumentSyntaxConnected(argument),
        data!(ArgumentSyntax::Ke { .. }) => SyntaxNodeRef::ArgumentSyntaxKe(argument),
        data!(ArgumentSyntax::Bo { .. }) => SyntaxNodeRef::ArgumentSyntaxBo(argument),
        data!(ArgumentSyntax::Gek { .. }) => SyntaxNodeRef::ArgumentSyntaxGek(argument),
        data!(ArgumentSyntax::Descriptor(..)) => SyntaxNodeRef::ArgumentSyntaxDescriptor(argument),
        data!(ArgumentSyntax::ConnectedDescriptor(..)) => {
            SyntaxNodeRef::ArgumentSyntaxConnectedDescriptor(argument)
        }
        data!(ArgumentSyntax::Name { .. }) => SyntaxNodeRef::ArgumentSyntaxName(argument),
        data!(ArgumentSyntax::Cmevla(..)) => SyntaxNodeRef::ArgumentSyntaxCmevla(argument),
        data!(ArgumentSyntax::RelationVocative { .. }) => {
            SyntaxNodeRef::ArgumentSyntaxRelationVocative(argument)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn node_ref_as_relation<'tree>(node: SyntaxNodeRef<'tree>) -> Option<&'tree RelationSyntax> {
    match node {
        SyntaxNodeRef::RelationSyntaxConnected(relation)
        | SyntaxNodeRef::RelationSyntaxCo(relation)
        | SyntaxNodeRef::RelationSyntaxBo(relation)
        | SyntaxNodeRef::RelationSyntaxNa(relation)
        | SyntaxNodeRef::RelationSyntaxBase(relation)
        | SyntaxNodeRef::RelationSyntaxSe(relation)
        | SyntaxNodeRef::RelationSyntaxKe(relation)
        | SyntaxNodeRef::RelationSyntaxTenseModal(relation)
        | SyntaxNodeRef::RelationSyntaxGuha(relation)
        | SyntaxNodeRef::RelationSyntaxAbstraction(relation)
        | SyntaxNodeRef::RelationSyntaxCompound(relation) => Some(relation),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn node_ref_as_relation_unit<'tree>(
    node: SyntaxNodeRef<'tree>,
) -> Option<&'tree RelationUnitSyntax> {
    match node {
        SyntaxNodeRef::RelationUnitSyntaxWord(unit)
        | SyntaxNodeRef::RelationUnitSyntaxGoha(unit)
        | SyntaxNodeRef::RelationUnitSyntaxSe(unit)
        | SyntaxNodeRef::RelationUnitSyntaxKe(unit)
        | SyntaxNodeRef::RelationUnitSyntaxNahe(unit)
        | SyntaxNodeRef::RelationUnitSyntaxBo(unit)
        | SyntaxNodeRef::RelationUnitSyntaxConnected(unit)
        | SyntaxNodeRef::RelationUnitSyntaxSelbriRelativeClause(unit)
        | SyntaxNodeRef::RelationUnitSyntaxWrapped(unit)
        | SyntaxNodeRef::RelationUnitSyntaxJai(unit)
        | SyntaxNodeRef::RelationUnitSyntaxBe(unit)
        | SyntaxNodeRef::RelationUnitSyntaxPreposedBe(unit)
        | SyntaxNodeRef::RelationUnitSyntaxAbstraction(unit)
        | SyntaxNodeRef::RelationUnitSyntaxMe(unit)
        | SyntaxNodeRef::RelationUnitSyntaxMehoi(unit)
        | SyntaxNodeRef::RelationUnitSyntaxGohoi(unit)
        | SyntaxNodeRef::RelationUnitSyntaxMuhoi(unit)
        | SyntaxNodeRef::RelationUnitSyntaxLuhei(unit)
        | SyntaxNodeRef::RelationUnitSyntaxMoi(unit)
        | SyntaxNodeRef::RelationUnitSyntaxNuha(unit)
        | SyntaxNodeRef::RelationUnitSyntaxXohi(unit)
        | SyntaxNodeRef::RelationUnitSyntaxCei(unit) => Some(unit),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn node_ref_as_term<'tree>(node: SyntaxNodeRef<'tree>) -> Option<&'tree TermSyntax> {
    match node {
        SyntaxNodeRef::TermSyntaxNuhiTermset(term)
        | SyntaxNodeRef::TermSyntaxGekNuhiTermset(term)
        | SyntaxNodeRef::TermSyntaxCehe(term)
        | SyntaxNodeRef::TermSyntaxPehe(term)
        | SyntaxNodeRef::TermSyntaxArgument(term)
        | SyntaxNodeRef::TermSyntaxFa(term)
        | SyntaxNodeRef::TermSyntaxNaKu(term)
        | SyntaxNodeRef::TermSyntaxBareNa(term)
        | SyntaxNodeRef::TermSyntaxNoihaAdverbial(term)
        | SyntaxNodeRef::TermSyntaxPoihaBrigahi(term)
        | SyntaxNodeRef::TermSyntaxFihoiAdverbial(term)
        | SyntaxNodeRef::TermSyntaxSoiAdverbial(term)
        | SyntaxNodeRef::TermSyntaxJaiTagged(term)
        | SyntaxNodeRef::TermSyntaxTagged(term)
        | SyntaxNodeRef::TermSyntaxConnected(term)
        | SyntaxNodeRef::TermSyntaxBoConnected(term) => Some(term),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn node_ref_as_argument<'tree>(node: SyntaxNodeRef<'tree>) -> Option<&'tree ArgumentSyntax> {
    match node {
        SyntaxNodeRef::ArgumentSyntaxQuote(argument)
        | SyntaxNodeRef::ArgumentSyntaxMathExpression(argument)
        | SyntaxNodeRef::ArgumentSyntaxLetter(argument)
        | SyntaxNodeRef::ArgumentSyntaxQuantified(argument)
        | SyntaxNodeRef::ArgumentSyntaxRelativeClause(argument)
        | SyntaxNodeRef::ArgumentSyntaxVuho(argument)
        | SyntaxNodeRef::ArgumentSyntaxBridiDescription(argument)
        | SyntaxNodeRef::ArgumentSyntaxNaKu(argument)
        | SyntaxNodeRef::ArgumentSyntaxTagged(argument)
        | SyntaxNodeRef::ArgumentSyntaxNaheBo(argument)
        | SyntaxNodeRef::ArgumentSyntaxNahe(argument)
        | SyntaxNodeRef::ArgumentSyntaxTermWrapped(argument)
        | SyntaxNodeRef::ArgumentSyntaxKoha(argument)
        | SyntaxNodeRef::ArgumentSyntaxZohe(argument)
        | SyntaxNodeRef::ArgumentSyntaxLahe(argument)
        | SyntaxNodeRef::ArgumentSyntaxConnected(argument)
        | SyntaxNodeRef::ArgumentSyntaxKe(argument)
        | SyntaxNodeRef::ArgumentSyntaxBo(argument)
        | SyntaxNodeRef::ArgumentSyntaxGek(argument)
        | SyntaxNodeRef::ArgumentSyntaxDescriptor(argument)
        | SyntaxNodeRef::ArgumentSyntaxConnectedDescriptor(argument)
        | SyntaxNodeRef::ArgumentSyntaxName(argument)
        | SyntaxNodeRef::ArgumentSyntaxCmevla(argument)
        | SyntaxNodeRef::ArgumentSyntaxRelationVocative(argument) => Some(argument),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn tense_modal_node_ref<'tree>(
    tense_modal: &'tree jbotci_syntax::ast::TenseModalSyntax,
) -> SyntaxNodeRef<'tree> {
    match tense_modal.as_data() {
        data!(jbotci_syntax::ast::TenseModalSyntax::Composite { .. }) => {
            SyntaxNodeRef::TenseModalSyntaxComposite(tense_modal)
        }
        data!(jbotci_syntax::ast::TenseModalSyntax::Pu(..)) => {
            SyntaxNodeRef::TenseModalSyntaxPu(tense_modal)
        }
        data!(jbotci_syntax::ast::TenseModalSyntax::PuDistance { .. }) => {
            SyntaxNodeRef::TenseModalSyntaxPuDistance(tense_modal)
        }
        data!(jbotci_syntax::ast::TenseModalSyntax::TimeInterval(..)) => {
            SyntaxNodeRef::TenseModalSyntaxTimeInterval(tense_modal)
        }
        data!(jbotci_syntax::ast::TenseModalSyntax::PuCaha { .. }) => {
            SyntaxNodeRef::TenseModalSyntaxPuCaha(tense_modal)
        }
        data!(jbotci_syntax::ast::TenseModalSyntax::SpaceDistance(..)) => {
            SyntaxNodeRef::TenseModalSyntaxSpaceDistance(tense_modal)
        }
        data!(jbotci_syntax::ast::TenseModalSyntax::SpaceDirection(..)) => {
            SyntaxNodeRef::TenseModalSyntaxSpaceDirection(tense_modal)
        }
        data!(jbotci_syntax::ast::TenseModalSyntax::SpaceMovement { .. }) => {
            SyntaxNodeRef::TenseModalSyntaxSpaceMovement(tense_modal)
        }
        data!(jbotci_syntax::ast::TenseModalSyntax::Simple { .. }) => {
            SyntaxNodeRef::TenseModalSyntaxSimple(tense_modal)
        }
        data!(jbotci_syntax::ast::TenseModalSyntax::Ki(..)) => {
            SyntaxNodeRef::TenseModalSyntaxKi(tense_modal)
        }
        data!(jbotci_syntax::ast::TenseModalSyntax::Fiho { .. }) => {
            SyntaxNodeRef::TenseModalSyntaxFiho(tense_modal)
        }
        data!(jbotci_syntax::ast::TenseModalSyntax::Caha(..)) => {
            SyntaxNodeRef::TenseModalSyntaxCaha(tense_modal)
        }
        data!(jbotci_syntax::ast::TenseModalSyntax::Zaho(..)) => {
            SyntaxNodeRef::TenseModalSyntaxZaho(tense_modal)
        }
        data!(jbotci_syntax::ast::TenseModalSyntax::Interval { .. }) => {
            SyntaxNodeRef::TenseModalSyntaxInterval(tense_modal)
        }
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|place| (2..=5).contains(&place)))]
fn se_conversion_place(se: &WithFreeModifiers<Token>) -> Option<u8> {
    match se.value.cmavo() {
        Some(Cmavo::Se) => Some(2),
        Some(Cmavo::Te) => Some(3),
        Some(Cmavo::Ve) => Some(4),
        Some(Cmavo::Xe) => Some(5),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn fa_place_slot(fa: &WithFreeModifiers<Token>) -> Option<PlaceSlot> {
    match fa.cmavo() {
        Some(Cmavo::Fa) => PlaceSlot::numbered(1),
        Some(Cmavo::Fe) => PlaceSlot::numbered(2),
        Some(Cmavo::Fi) => PlaceSlot::numbered(3),
        Some(Cmavo::Fo) => PlaceSlot::numbered(4),
        Some(Cmavo::Fu) => PlaceSlot::numbered(5),
        Some(Cmavo::Fai) => Some(fai_slot()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn modal_slot_for_tagged_argument<'tree>(
    argument: &'tree ArgumentSyntax,
    index: &SyntaxIndex<'tree>,
) -> Option<PlaceSlot> {
    match argument.as_data() {
        data!(ArgumentSyntax::Tagged { tag, .. }) => match tag.as_data() {
            data!(jbotci_syntax::ast::ArgumentTagSyntax::TenseModal(tense)) => {
                Some(modal_slot(index.id_of(tense_modal_node_ref(tense))))
            }
            data!(jbotci_syntax::ast::ArgumentTagSyntax::Fa(fa)) => fa_place_slot(fa),
        },
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn abstraction_is_property(abstraction: &AbstractionSyntax) -> bool {
    abstraction.nu.cmavo() == Some(Cmavo::Ka)
}

#[requires(start > 0)]
#[ensures(ret >= start)]
fn next_place_after_common_terms(start: u8, terms: &[TermSyntax]) -> u8 {
    let mut cursor = PlaceCursor::new_at(SelbriPlaceFrameId(usize::MAX), start);
    for term in terms {
        advance_cursor_for_term_shape(&mut cursor, term);
    }
    cursor.next_place
}

#[requires(true)]
#[ensures(true)]
fn advance_cursor_for_term_shape(cursor: &mut PlaceCursor, term: &TermSyntax) {
    match term.as_data() {
        data!(TermSyntax::Argument(argument)) => {
            advance_cursor_for_argument_term_shape(cursor, argument);
        }
        data!(TermSyntax::Fa { fa, .. }) => {
            let slot = fa_place_slot(fa).unwrap_or_else(|| cursor.next_numbered_slot());
            cursor.record_slot(slot);
        }
        data!(TermSyntax::Tagged {
            tense_modal: None,
            ..
        }) => {
            let slot = cursor.next_numbered_slot();
            cursor.record_slot(slot);
        }
        data!(TermSyntax::Tagged {
            tense_modal: Some(_),
            ..
        }) => {}
        data!(TermSyntax::JaiTagged { .. }) => {
            cursor.record_slot(fai_slot());
        }
        data!(TermSyntax::NuhiTermset { termset, .. }) => {
            advance_cursor_for_terms_shape(cursor, termset);
        }
        data!(TermSyntax::GekNuhiTermset {
            terms,
            gik_terms,
            ..
        }) => {
            advance_cursor_for_alternative_term_shapes(cursor, terms, gik_terms);
        }
        data!(TermSyntax::Cehe {
            leading_terms,
            trailing_terms,
            ..
        })
        | data!(TermSyntax::Connected {
            leading_terms,
            trailing_terms,
            ..
        }) => {
            advance_cursor_for_terms_shape(cursor, leading_terms);
            advance_cursor_for_terms_shape(cursor, trailing_terms);
        }
        data!(TermSyntax::Pehe {
            leading_terms,
            trailing_terms,
            ..
        }) => {
            advance_cursor_for_alternative_term_shapes(cursor, leading_terms, trailing_terms);
        }
        data!(TermSyntax::BoConnected {
            leading_terms,
            trailing_term,
            ..
        }) => {
            advance_cursor_for_terms_shape(cursor, leading_terms);
            advance_cursor_for_term_shape(cursor, trailing_term);
        }
        _ => {}
    }
}

#[requires(true)]
#[ensures(true)]
fn advance_cursor_for_terms_shape(cursor: &mut PlaceCursor, terms: &[TermSyntax]) {
    for term in terms {
        advance_cursor_for_term_shape(cursor, term);
    }
}

#[requires(true)]
#[ensures(true)]
fn advance_cursor_for_alternative_term_shapes(
    cursor: &mut PlaceCursor,
    leading_terms: &[TermSyntax],
    trailing_terms: &[TermSyntax],
) {
    let initial_cursor = cursor.clone();
    let mut leading_cursor = initial_cursor.clone();
    advance_cursor_for_terms_shape(&mut leading_cursor, leading_terms);
    let mut trailing_cursor = initial_cursor;
    advance_cursor_for_terms_shape(&mut trailing_cursor, trailing_terms);
    *cursor = leading_cursor;
    cursor.merge_from_branch(&trailing_cursor);
}

#[requires(true)]
#[ensures(true)]
fn advance_cursor_for_argument_term_shape(cursor: &mut PlaceCursor, argument: &ArgumentSyntax) {
    match argument.as_data() {
        data!(ArgumentSyntax::Connected {
            leading_argument,
            connective,
            trailing_argument,
        }) if connective_contains_cmavo(connective, Cmavo::Cehe) => {
            advance_cursor_for_argument_term_shape(cursor, leading_argument);
            advance_cursor_for_argument_term_shape(cursor, trailing_argument);
        }
        data!(ArgumentSyntax::Tagged { tag, .. }) => {
            let slot = match tag.as_data() {
                data!(ArgumentTagSyntax::Fa(fa)) => {
                    fa_place_slot(fa).unwrap_or_else(|| cursor.next_numbered_slot())
                }
                data!(ArgumentTagSyntax::TenseModal(..)) => modal_slot(None),
            };
            cursor.record_slot(slot);
        }
        _ => {
            let slot = cursor.next_numbered_slot();
            cursor.record_slot(slot);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn connective_contains_cmavo(connective: &ConnectiveSyntax, expected: Cmavo) -> bool {
    match connective.as_data() {
        data!(ConnectiveSyntax::Afterthought { cmavo, .. })
        | data!(ConnectiveSyntax::Relation { cmavo, .. })
        | data!(ConnectiveSyntax::PredicateTail { cmavo, .. })
        | data!(ConnectiveSyntax::Forethought { cmavo, .. })
        | data!(ConnectiveSyntax::NonLogical { cmavo, .. })
        | data!(ConnectiveSyntax::Interval { cmavo, .. }) => {
            cmavo.value.iter().any(|token| token.is_cmavo(expected))
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn convert_slot(slot: PlaceSlot, converted_place: NonZeroU8) -> PlaceSlot {
    match slot {
        PlaceSlot::Numbered(place) if place.get() == 1 => numbered_slot(converted_place),
        PlaceSlot::Numbered(place) if place == converted_place => {
            numbered_slot(NonZeroU8::new(1).expect("literal is non-zero"))
        }
        _ => slot,
    }
}

#[requires(true)]
#[ensures(true)]
fn koha_records_self_mention(cmavo: Cmavo) -> bool {
    matches!(
        cmavo,
        Cmavo::Da
            | Cmavo::De
            | Cmavo::Di
            | Cmavo::Do
            | Cmavo::Mi
            | Cmavo::Ta
            | Cmavo::Ti
            | Cmavo::Tu
    )
}

#[requires(true)]
#[ensures(ret == matches!(cmavo, Cmavo::Ri | Cmavo::Da | Cmavo::De | Cmavo::Di | Cmavo::Ta | Cmavo::Ti | Cmavo::Tu))]
fn koha_mention_available_to_ri(cmavo: Cmavo) -> bool {
    matches!(
        cmavo,
        Cmavo::Ri | Cmavo::Da | Cmavo::De | Cmavo::Di | Cmavo::Ta | Cmavo::Ti | Cmavo::Tu
    )
}

#[requires(true)]
#[ensures(true)]
fn argument_wraps_ri(argument: &ArgumentSyntax) -> bool {
    argument_koha_cmavo_with_subscript(argument)
        .is_some_and(|(cmavo, _subscript)| cmavo == Cmavo::Ri)
}

#[requires(true)]
#[ensures(true)]
fn koha_subscript_index(free_modifiers: &[FreeModifierSyntax]) -> Option<usize> {
    free_modifiers.iter().find_map(|free_modifier| {
        if let data!(FreeModifierSyntax::Xi { expression, .. }) = free_modifier.as_data() {
            math_expression_to_usize(expression)
        } else {
            None
        }
    })
}

#[requires(true)]
#[ensures(true)]
fn math_expression_to_usize(expression: &MathExpressionSyntax) -> Option<usize> {
    match expression.as_data() {
        data!(MathExpressionSyntax::Number(quantifier)) => quantifier_to_usize(quantifier),
        data!(MathExpressionSyntax::Vei {
            inner_expression,
            ..
        }) => math_expression_to_usize(inner_expression),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn quantifier_to_usize(quantifier: &QuantifierSyntax) -> Option<usize> {
    match quantifier.as_data() {
        data!(QuantifierSyntax::Number { number, .. }) => word_run_to_usize(&number.value),
        data!(QuantifierSyntax::Vei {
            math_expression,
            ..
        }) => math_expression_to_usize(math_expression),
    }
}

#[requires(true)]
#[ensures(true)]
fn word_run_to_usize(words: &jbotci_syntax::ast::WordRun) -> Option<usize> {
    let mut value = 0usize;
    let mut saw_digit = false;
    for word in words.iter() {
        let digit = cmavo_digit(word.cmavo())?;
        value = value.checked_mul(10)?.checked_add(digit)?;
        saw_digit = true;
    }
    saw_digit.then_some(value)
}

#[requires(true)]
#[ensures(ret.is_none_or(|digit| digit <= 9))]
fn cmavo_digit(cmavo: Option<Cmavo>) -> Option<usize> {
    match cmavo {
        Some(Cmavo::No) => Some(0),
        Some(Cmavo::Pa) => Some(1),
        Some(Cmavo::Re) => Some(2),
        Some(Cmavo::Ci) => Some(3),
        Some(Cmavo::Vo) => Some(4),
        Some(Cmavo::Mu) => Some(5),
        Some(Cmavo::Xa) => Some(6),
        Some(Cmavo::Ze) => Some(7),
        Some(Cmavo::Bi) => Some(8),
        Some(Cmavo::So) => Some(9),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|letter| !letter.is_empty()))]
fn letter_pro_sumti_base(
    letter: &WithFreeModifiers<jbotci_syntax::ast::WordRun>,
) -> Option<String> {
    let [word] = letter.value.as_slice() else {
        return None;
    };
    word.is_selmaho(Selmaho::By)
        .then(|| token_base_letter(word))
        .flatten()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|letter| !letter.is_empty()))]
fn argument_letter_base(argument: &ArgumentSyntax) -> Option<String> {
    match argument.as_data() {
        data!(ArgumentSyntax::Descriptor(descriptor)) => descriptor
            .relation
            .as_deref()
            .and_then(relation_base_letter)
            .or_else(|| {
                descriptor
                    .tail_elements
                    .iter()
                    .find_map(argument_tail_element_base_letter)
            }),
        data!(ArgumentSyntax::ConnectedDescriptor(descriptor)) => descriptor
            .relation
            .as_deref()
            .and_then(relation_base_letter)
            .or_else(|| {
                descriptor
                    .tail_elements
                    .iter()
                    .find_map(argument_tail_element_base_letter)
            }),
        data!(ArgumentSyntax::Name { names, .. }) | data!(ArgumentSyntax::Cmevla(names)) => {
            names.value.as_slice().first().and_then(token_base_letter)
        }
        data!(ArgumentSyntax::RelativeClause { base_argument, .. })
        | data!(ArgumentSyntax::Vuho { base_argument, .. })
        | data!(ArgumentSyntax::Lahe {
            inner_argument: base_argument,
            ..
        })
        | data!(ArgumentSyntax::NaheBo {
            inner_argument: base_argument,
            ..
        })
        | data!(ArgumentSyntax::Nahe {
            inner_argument: base_argument,
            ..
        })
        | data!(ArgumentSyntax::Ke {
            inner_argument: base_argument,
            ..
        })
        | data!(ArgumentSyntax::Tagged {
            inner_argument: base_argument,
            ..
        })
        | data!(ArgumentSyntax::Quantified {
            inner_argument: base_argument,
            ..
        }) => argument_letter_base(base_argument),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|letter| !letter.is_empty()))]
fn argument_tail_element_base_letter(element: &ArgumentTailElementSyntax) -> Option<String> {
    match element.as_data() {
        data!(ArgumentTailElementSyntax::Argument(argument)) => argument_letter_base(argument),
        data!(ArgumentTailElementSyntax::RelativeClauses(relative_clauses)) => relative_clauses
            .iter()
            .find_map(relative_clause_base_letter),
        data!(ArgumentTailElementSyntax::Quantifier(_)) => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|letter| !letter.is_empty()))]
fn relative_clause_base_letter(relative_clause: &RelativeClauseSyntax) -> Option<String> {
    match relative_clause.as_data() {
        data!(RelativeClauseSyntax::Goi(goi)) => argument_letter_base(&goi.argument),
        data!(RelativeClauseSyntax::Noi { subsentence, .. })
        | data!(RelativeClauseSyntax::Poi { subsentence, .. }) => {
            subsentence_base_letter(subsentence)
        }
        data!(RelativeClauseSyntax::Zihe { inner, .. })
        | data!(RelativeClauseSyntax::Connected { inner, .. }) => {
            relative_clause_base_letter(inner)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|letter| !letter.is_empty()))]
fn subsentence_base_letter(subsentence: &SubsentenceSyntax) -> Option<String> {
    match subsentence.as_data() {
        data!(SubsentenceSyntax::Plain(predicate)) => {
            predicate_tail_base_letter(&predicate.predicate_tail)
        }
        data!(SubsentenceSyntax::Prenex {
            inner_subsentence,
            ..
        }) => subsentence_base_letter(inner_subsentence),
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|letter| !letter.is_empty()))]
fn predicate_tail_base_letter(predicate_tail: &PredicateTailSyntax) -> Option<String> {
    predicate_tail1_base_letter(&predicate_tail.first)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|letter| !letter.is_empty()))]
fn predicate_tail1_base_letter(predicate_tail: &PredicateTail1Syntax) -> Option<String> {
    predicate_tail2_base_letter(&predicate_tail.first)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|letter| !letter.is_empty()))]
fn predicate_tail2_base_letter(predicate_tail: &PredicateTail2Syntax) -> Option<String> {
    predicate_tail3_base_letter(&predicate_tail.first)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|letter| !letter.is_empty()))]
fn predicate_tail3_base_letter(predicate_tail: &PredicateTail3Syntax) -> Option<String> {
    match predicate_tail.as_data() {
        data!(PredicateTail3Syntax::Relation { relation, .. }) => relation_base_letter(relation),
        data!(PredicateTail3Syntax::GekSentence(_)) => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|letter| !letter.is_empty()))]
fn relation_base_letter(relation: &RelationSyntax) -> Option<String> {
    match relation.as_data() {
        data!(RelationSyntax::Base(word)) => token_base_letter(word),
        data!(RelationSyntax::Se { inner_relation, .. })
        | data!(RelationSyntax::Na { inner_relation, .. })
        | data!(RelationSyntax::TenseModal { inner_relation, .. }) => {
            relation_base_letter(inner_relation)
        }
        data!(RelationSyntax::Ke { relation, .. }) => relation_base_letter(relation),
        data!(RelationSyntax::Connected {
            leading_relation,
            ..
        })
        | data!(RelationSyntax::Co {
            leading_relation,
            ..
        }) => relation_base_letter(leading_relation),
        data!(RelationSyntax::Bo {
            trailing_relation,
            ..
        }) => relation_base_letter(trailing_relation),
        data!(RelationSyntax::Compound(units)) => {
            units.as_slice().first().and_then(relation_unit_base_letter)
        }
        data!(RelationSyntax::Abstraction(abstraction)) => word_base_letter(&abstraction.nu),
        data!(RelationSyntax::Guha { .. }) => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|letter| !letter.is_empty()))]
fn relation_unit_base_letter(unit: &RelationUnitSyntax) -> Option<String> {
    match unit.as_data() {
        data!(RelationUnitSyntax::Word(word))
        | data!(RelationUnitSyntax::Goha { goha: word, .. }) => word_base_letter(word),
        data!(RelationUnitSyntax::Se { inner_unit, .. })
        | data!(RelationUnitSyntax::Nahe { inner_unit, .. })
        | data!(RelationUnitSyntax::Jai { inner_unit, .. }) => {
            relation_unit_base_letter(inner_unit)
        }
        data!(RelationUnitSyntax::Ke { relation, .. })
        | data!(RelationUnitSyntax::Wrapped(relation)) => relation_base_letter(relation),
        data!(RelationUnitSyntax::Bo { trailing_unit, .. }) => {
            relation_unit_base_letter(trailing_unit)
        }
        data!(RelationUnitSyntax::Connected { leading_unit, .. }) => {
            relation_unit_base_letter(leading_unit)
        }
        data!(RelationUnitSyntax::SelbriRelativeClause { base, .. })
        | data!(RelationUnitSyntax::Be { base, .. })
        | data!(RelationUnitSyntax::PreposedBe { base, .. })
        | data!(RelationUnitSyntax::Cei { base, .. }) => relation_unit_base_letter(base),
        data!(RelationUnitSyntax::Abstraction(abstraction)) => word_base_letter(&abstraction.nu),
        data!(RelationUnitSyntax::Me { argument, .. }) => argument_letter_base(argument),
        data!(RelationUnitSyntax::Luhei { .. })
        | data!(RelationUnitSyntax::Mehoi(..))
        | data!(RelationUnitSyntax::Gohoi(..))
        | data!(RelationUnitSyntax::Muhoi(..))
        | data!(RelationUnitSyntax::Moi { .. })
        | data!(RelationUnitSyntax::Nuha { .. })
        | data!(RelationUnitSyntax::Xohi { .. }) => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|letter| !letter.is_empty()))]
fn word_base_letter(word: &WithFreeModifiers<Token>) -> Option<String> {
    token_base_letter(&word.value)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|letter| !letter.is_empty()))]
fn token_base_letter(word: &Token) -> Option<String> {
    let text = word.as_ref().core_word().bare_word()?.canonical_phonemes();
    text.chars()
        .find(|character| character.is_alphabetic())
        .map(|character| character.to_string())
}

#[requires(true)]
#[ensures(true)]
fn argument_koha_cmavo(argument: &ArgumentSyntax) -> Option<Cmavo> {
    match argument.as_data() {
        data!(ArgumentSyntax::Koha(koha)) => koha.cmavo(),
        data!(ArgumentSyntax::Tagged { inner_argument, .. })
        | data!(ArgumentSyntax::Quantified { inner_argument, .. })
        | data!(ArgumentSyntax::NaheBo { inner_argument, .. })
        | data!(ArgumentSyntax::Nahe { inner_argument, .. })
        | data!(ArgumentSyntax::Lahe { inner_argument, .. })
        | data!(ArgumentSyntax::Ke { inner_argument, .. }) => argument_koha_cmavo(inner_argument),
        data!(ArgumentSyntax::RelativeClause { base_argument, .. })
        | data!(ArgumentSyntax::Vuho { base_argument, .. }) => argument_koha_cmavo(base_argument),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn argument_koha_cmavo_with_subscript(argument: &ArgumentSyntax) -> Option<(Cmavo, Option<usize>)> {
    match argument.as_data() {
        data!(ArgumentSyntax::Koha(koha)) => {
            Some((koha.cmavo()?, koha_subscript_index(&koha.free_modifiers)))
        }
        data!(ArgumentSyntax::Tagged { inner_argument, .. })
        | data!(ArgumentSyntax::Quantified { inner_argument, .. })
        | data!(ArgumentSyntax::NaheBo { inner_argument, .. })
        | data!(ArgumentSyntax::Nahe { inner_argument, .. })
        | data!(ArgumentSyntax::Lahe { inner_argument, .. })
        | data!(ArgumentSyntax::Ke { inner_argument, .. }) => {
            argument_koha_cmavo_with_subscript(inner_argument)
        }
        data!(ArgumentSyntax::RelativeClause { base_argument, .. })
        | data!(ArgumentSyntax::Vuho { base_argument, .. }) => {
            argument_koha_cmavo_with_subscript(base_argument)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn koha_assignable_cmavo(argument: &ArgumentSyntax) -> Option<Cmavo> {
    let cmavo = argument_koha_cmavo(argument)?;
    is_assignable_koha(cmavo).then_some(cmavo)
}

#[requires(true)]
#[ensures(true)]
fn is_assignable_koha(cmavo: Cmavo) -> bool {
    matches!(
        cmavo,
        Cmavo::Koha
            | Cmavo::Kohe
            | Cmavo::Kohi
            | Cmavo::Koho
            | Cmavo::Kohu
            | Cmavo::Foha
            | Cmavo::Fohe
            | Cmavo::Fohi
            | Cmavo::Foho
            | Cmavo::Fohu
    )
}

#[requires(true)]
#[ensures(true)]
fn voha_slot(cmavo: Cmavo) -> Option<PlaceSlot> {
    match cmavo {
        Cmavo::Voha => PlaceSlot::numbered(1),
        Cmavo::Vohe => PlaceSlot::numbered(2),
        Cmavo::Vohi => PlaceSlot::numbered(3),
        Cmavo::Voho => PlaceSlot::numbered(4),
        Cmavo::Vohu => PlaceSlot::numbered(5),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|label| !label.is_empty()))]
fn broda_label(word_like: &WordLike) -> Option<String> {
    let word = word_like.bare_word()?;
    let text = word.canonical_phonemes();
    matches!(
        text.as_str(),
        "broda" | "brode" | "brodi" | "brodo" | "brodu"
    )
    .then_some(text)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|label| !label.is_empty()))]
fn relation_unit_assignment_label(unit: &RelationUnitSyntax) -> Option<String> {
    match unit.as_data() {
        data!(RelationUnitSyntax::Word(word)) => broda_label(word.core_word()),
        data!(RelationUnitSyntax::Goha { goha, .. }) => {
            let cmavo = goha.cmavo()?;
            matches!(cmavo, Cmavo::Buha | Cmavo::Buhe | Cmavo::Buhi)
                .then(|| cmavo.canonical_text().to_owned())
        }
        data!(RelationUnitSyntax::Se { inner_unit, .. })
        | data!(RelationUnitSyntax::Nahe { inner_unit, .. }) => {
            relation_unit_assignment_label(inner_unit)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused_imports)]
    use bityzba::{data, ensures, requires};
    use jbotci_morphology::segment_words_with_modifiers;
    use jbotci_syntax::{ParseOptions, parse_text};

    #[requires(true)]
    #[ensures(true)]
    fn run_reference_test(test: impl FnOnce()) {
        test();
    }

    #[requires(true)]
    #[ensures(true)]
    fn parse_syntax(input: &str) -> TextSyntax {
        let words = segment_words_with_modifiers(input).expect("morphology succeeds");
        parse_text(&words, &ParseOptions::default()).expect("syntax succeeds")
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
    fn argument_label(index: &SyntaxIndex<'_>, argument: ArgumentNodeId) -> Option<String> {
        match index.argument(argument)?.as_data() {
            data!(ArgumentSyntax::Koha(koha)) => {
                Some(koha.core_word().bare_word()?.canonical_phonemes())
            }
            data!(ArgumentSyntax::Descriptor(descriptor)) => {
                descriptor.relation.as_deref().and_then(relation_label)
            }
            data!(ArgumentSyntax::Name { names, .. }) => names
                .value
                .first()
                .core_word()
                .bare_word()
                .map(|word| word.canonical_phonemes()),
            _ => index
                .metadata(argument.0)
                .and_then(|metadata| metadata.source_spans.first())
                .map(|span| format!("{}..{}", span.byte_start, span.byte_end)),
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
    fn relation_label(relation: &RelationSyntax) -> Option<String> {
        match relation.as_data() {
            data!(RelationSyntax::Base(word)) => {
                Some(word.core_word().bare_word()?.canonical_phonemes())
            }
            data!(RelationSyntax::Se { inner_relation, .. })
            | data!(RelationSyntax::Na { inner_relation, .. })
            | data!(RelationSyntax::TenseModal { inner_relation, .. }) => {
                relation_label(inner_relation)
            }
            data!(RelationSyntax::Ke { relation, .. }) => relation_label(relation),
            data!(RelationSyntax::Compound(units)) => relation_unit_label(units.last()),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
    fn relation_unit_label(unit: &RelationUnitSyntax) -> Option<String> {
        match unit.as_data() {
            data!(RelationUnitSyntax::Word(word)) => {
                Some(word.core_word().bare_word()?.canonical_phonemes())
            }
            data!(RelationUnitSyntax::Se { inner_unit, .. })
            | data!(RelationUnitSyntax::Nahe { inner_unit, .. }) => relation_unit_label(inner_unit),
            data!(RelationUnitSyntax::Ke { relation, .. })
            | data!(RelationUnitSyntax::Wrapped(relation)) => relation_label(relation),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn first_assignment_label(
        analysis: &ReferenceAnalysis<'_>,
        frame: SelbriPlaceFrameId,
        slot: u8,
    ) -> Option<String> {
        let slot = PlaceSlot::numbered(slot)?;
        let argument = analysis
            .place_analysis
            .first_argument_for_place(frame, slot)?;
        argument_label(&analysis.syntax_index, argument)
    }

    #[requires(true)]
    #[ensures(true)]
    fn frame_for_relation_label(
        analysis: &ReferenceAnalysis<'_>,
        label: &str,
        kind: PlaceFrameKind,
    ) -> Option<SelbriPlaceFrameId> {
        analysis
            .place_analysis
            .frames()
            .iter()
            .find(|frame| {
                frame.kind == kind
                    && frame.relation.is_some_and(|relation| {
                        analysis
                            .syntax_index
                            .relation(relation)
                            .and_then(relation_label)
                            .is_some_and(|actual| actual == label)
                    })
            })
            .map(|frame| frame.id)
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn syntax_index_round_trips_root_and_records_leaf_interval() {
        run_reference_test(|| {
            let syntax = parse_syntax("mi klama do");
            let analysis = analyze_references(&syntax).expect("reference analysis succeeds");
            let root = analysis.syntax_index.root();
            let root_metadata = analysis
                .syntax_index
                .metadata(root.0)
                .expect("root metadata exists");

            assert_eq!(
                analysis.syntax_index.node(root.0),
                Some(SyntaxNodeRef::TextSyntax(&syntax))
            );
            assert_eq!(root_metadata.leaf_start, 0);
            assert_eq!(root_metadata.leaf_end, 3);
            assert_eq!(root_metadata.source_spans.len(), 3);
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn simple_predicate_assigns_numbered_places() {
        run_reference_test(|| {
            let syntax = parse_syntax("mi klama do");
            let analysis = analyze_references(&syntax).expect("reference analysis succeeds");
            let klama = frame_for_relation_label(&analysis, "klama", PlaceFrameKind::BaseRelation)
                .expect("klama frame exists");

            assert_eq!(
                first_assignment_label(&analysis, klama, 1).as_deref(),
                Some("mi")
            );
            assert_eq!(
                first_assignment_label(&analysis, klama, 2).as_deref(),
                Some("do")
            );
            let projection = analysis.v0_compatibility_projection();
            assert!(!projection.argument_assignments.is_empty());
            assert!(!projection.relation_places.is_empty());
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn se_conversion_exposes_visible_and_base_place_frames() {
        run_reference_test(|| {
            let syntax = parse_syntax("mi se klama do");
            let analysis = analyze_references(&syntax).expect("reference analysis succeeds");
            let base = frame_for_relation_label(&analysis, "klama", PlaceFrameKind::BaseRelation)
                .expect("base klama frame exists");
            let converted = frame_for_relation_label(&analysis, "klama", PlaceFrameKind::Converted)
                .expect("converted se klama frame exists");

            assert_eq!(
                first_assignment_label(&analysis, converted, 1).as_deref(),
                Some("mi")
            );
            assert_eq!(
                first_assignment_label(&analysis, converted, 2).as_deref(),
                Some("do")
            );
            assert_eq!(
                first_assignment_label(&analysis, base, 1).as_deref(),
                Some("do")
            );
            assert_eq!(
                first_assignment_label(&analysis, base, 2).as_deref(),
                Some("mi")
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn goi_binding_resolves_later_koha() {
        run_reference_test(|| {
            let syntax = parse_syntax("le nanmu goi ko'a cu klama .i ko'a cadzu");
            let analysis = analyze_references(&syntax).expect("reference analysis succeeds");

            assert!(analysis.discourse_references.edges().iter().any(|edge| {
                edge.kind == ReferenceKind::Koha
                    && matches!(edge.target, ReferenceTarget::ResolvedNode(_))
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ri_resolves_previous_sumti_without_guessing_ra_ru() {
        run_reference_test(|| {
            let syntax = parse_syntax("la djan klama .i ri cadzu");
            let analysis = analyze_references(&syntax).expect("reference analysis succeeds");

            assert!(analysis.discourse_references.edges().iter().any(|edge| {
                edge.kind == ReferenceKind::Ri
                    && matches!(edge.target, ReferenceTarget::ResolvedNode(_))
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn ri_skips_most_koha_and_repeats_wrapped_ri_antecedent() {
        run_reference_test(|| {
            let syntax = parse_syntax("mi ce do girzu .i lu'o ri gunma .i vu'i ri porsi");
            let analysis = analyze_references(&syntax).expect("reference analysis succeeds");
            let projection = analysis.fixture_projection();
            let expected = FixtureSpanKey {
                offset: 0,
                length: 8,
            };

            let ri_targets: Vec<_> = projection
                .references
                .iter()
                .filter(|edge| edge.kind == ReferenceKind::Ri)
                .map(|edge| &edge.target)
                .collect();

            assert_eq!(ri_targets.len(), 2);
            assert!(ri_targets.iter().all(|target| {
                matches!(target, FixtureReferenceTarget::ResolvedNode { node } if *node == expected)
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fixture_projection_is_sorted_and_canonical_json() {
        run_reference_test(|| {
            let syntax = parse_syntax("mi se klama do .i ri cadzu");
            let analysis = analyze_references(&syntax).expect("reference analysis succeeds");
            let projection = analysis.fixture_projection();
            let json = analysis
                .fixture_projection_json()
                .expect("fixture projection serializes");

            assert!(
                projection
                    .frames
                    .windows(2)
                    .all(|items| items[0] <= items[1])
            );
            assert!(
                projection
                    .assignments
                    .windows(2)
                    .all(|items| items[0] <= items[1])
            );
            assert!(
                projection
                    .relation_places
                    .windows(2)
                    .all(|items| items[0] <= items[1])
            );
            assert!(
                projection
                    .references
                    .windows(2)
                    .all(|items| items[0] <= items[1])
            );
            assert_eq!(
                json,
                serde_json::to_string(&projection).expect("projection serializes")
            );
            assert!(json.contains("\"references\""));
        });
    }
}
