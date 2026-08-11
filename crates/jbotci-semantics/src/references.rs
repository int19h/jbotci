//! Borrowed semantic reference overlay for syntax trees.

use std::collections::{HashMap, HashSet};
use std::num::NonZeroU8;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, new, requires};
use jbotci_morphology::{Cmavo, Word, WordLike, WordLikeData};
use jbotci_source::{SourceId, SourceSpan};
use jbotci_syntax::generated_model::{
    self as generated, AtomRef as GeneratedSyntaxAtomRef, NodeRef as GeneratedSyntaxNodeRef,
    TextSyntax as GeneratedTextSyntax, TreeNode as GeneratedSyntaxTreeNode,
    TreeWalkable as GeneratedSyntaxTreeWalkable, TreeWalker as GeneratedSyntaxTreeWalker,
};

use jbotci_syntax::tree::{Token, WithFreeModifiers};
use jbotci_tree::TreeVisitor;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::generated_term_view::{GeneratedLinkedSumtiRef, GeneratedSimpleTermRef};

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
pub struct BridiNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct BridiTailNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct SelbriNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct TanruUnitNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct TermNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct SumtiNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct FreeModifierNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct AbstractionNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct MeksoNodeId(pub RawSyntaxNodeId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct MeksoOperatorNodeId(pub RawSyntaxNodeId);

#[invariant(leaf_start <= leaf_end)]
#[invariant(first_source_span.is_some() == last_source_span.is_some())]
#[invariant(first_source_span.as_ref().zip(last_source_span.as_ref()).is_none_or(|(first, last)| first.byte_start <= last.byte_start
    && first.char_start <= last.char_start))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxNodeMetadata {
    pub id: RawSyntaxNodeId,
    pub parent: Option<RawSyntaxNodeId>,
    pub preorder: usize,
    pub depth: usize,
    pub leaf_start: usize,
    pub leaf_end: usize,
    pub first_source_span: Option<SourceSpan>,
    pub last_source_span: Option<SourceSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct SelbriPlaceFrameId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct SumtiPlaceAssignmentId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct ReferenceEdgeId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
#[invariant(true)]
#[invariant(::Numbered(_) => true)]
#[invariant(::Modal(_) => true)]
#[invariant(::PlaceQuestion => true)]
pub enum PlaceSlot {
    Numbered(NonZeroU8),
    Modal(Option<RawSyntaxNodeId>),
    PlaceQuestion,
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
            PlaceSlot::Modal(_) | PlaceSlot::PlaceQuestion | PlaceSlot::Fai => None,
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
fn place_question_slot() -> PlaceSlot {
    PlaceSlot::PlaceQuestion
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
fn propagation_connective_branches(branches: Vec<SelbriPlaceFrameId>) -> PlaceFramePropagation {
    PlaceFramePropagation::ConnectiveBranches { branches }
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
    Bridi,
    BridiTail,
    BaseSelbri,
    TanruUnit,
    Converted,
    JaiConverted,
    LinkedUnit,
    ConnectiveBranching,
    Compound,
    CoInverted,
    Forwarding,
    Abstraction,
    ProBridi,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[invariant(true)]
#[invariant(::Forward => true)]
#[invariant(::Conversion => true)]
#[invariant(::Jai => true)]
#[invariant(::ConnectiveBranches => true)]
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
    ConnectiveBranches {
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

#[invariant(selbri.is_none() || tanru_unit.is_none())]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelbriPlaceFrame {
    pub id: SelbriPlaceFrameId,
    pub node: RawSyntaxNodeId,
    pub kind: PlaceFrameKind,
    pub selbri: Option<SelbriNodeId>,
    pub tanru_unit: Option<TanruUnitNodeId>,
    pub propagation: PlaceFramePropagation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum AssignmentSource {
    SequentialTerm,
    FaTerm,
    ModalTerm,
    LinkedSumti,
    CoSeltauTerm,
    TermsetBranch,
    SharedHeadTerm,
    SharedTailTerm,
    Propagated,
}

#[requires(true)]
#[ensures(matches!(
    ret,
    AssignmentSource::SharedHeadTerm | AssignmentSource::SharedTailTerm | AssignmentSource::Propagated
))]
fn propagated_assignment_source(source: AssignmentSource) -> AssignmentSource {
    match source {
        AssignmentSource::SharedHeadTerm | AssignmentSource::SharedTailTerm => source,
        AssignmentSource::SequentialTerm
        | AssignmentSource::FaTerm
        | AssignmentSource::ModalTerm
        | AssignmentSource::LinkedSumti
        | AssignmentSource::CoSeltauTerm
        | AssignmentSource::TermsetBranch
        | AssignmentSource::Propagated => AssignmentSource::Propagated,
    }
}

#[invariant(term.is_some() || matches!(source, AssignmentSource::LinkedSumti | AssignmentSource::Propagated))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SumtiPlaceAssignment {
    pub id: SumtiPlaceAssignmentId,
    pub frame: SelbriPlaceFrameId,
    pub slot: PlaceSlot,
    pub sumti: SumtiNodeId,
    pub term: Option<TermNodeId>,
    pub source: AssignmentSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum ReferenceKind {
    SumtiAssociation,
    RelativePhraseHead,
    RelativePhraseArgument,
    ProBridiAssignment,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum VagueReferenceKind {
    DistantSumti,
    RecentSumti,
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

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ReferenceRule {
    #[serde(rename = "di'e refers to the following utterance when one is present")]
    DiheFollowingWhenPresent,
    #[serde(rename = "di'e refers to the following utterance")]
    DiheFollowing,
    #[serde(rename = "prenex CEI assignment binds the following bridi")]
    PrenexCeiAssignment,
    #[serde(
        rename = "letteral pro-sumti resolves to the latest sumti with the same initial string"
    )]
    LetteralProSumtiLatestInitial,
    #[serde(rename = "GOI relative clause equates its sumti with the relative-clause head")]
    GoiEquatesHead,
    #[serde(rename = "GOI assigns the relative-clause head pro-sumti to its sumti")]
    GoiAssignsHeadProSumti,
    #[serde(rename = "GOI relative phrase marker relates x1 to the relative-clause head")]
    GoiX1RelativeHead,
    #[serde(rename = "GOI relative phrase marker relates x2 to the attached sumti")]
    GoiX2AttachedSumti,
    #[serde(rename = "CEI assigns a pro-bridi word to the enclosing bridi")]
    CeiAssignsEnclosingBridi,
    #[serde(rename = "wrapped ri exposes the complete sumti as a reference source")]
    WrappedRiReferenceSource,
    #[serde(rename = "wrapped ke'a exposes the complete sumti as a reference source")]
    WrappedKehaReferenceSource,
    #[serde(rename = "ri repeats the previous complete sumti")]
    RiPreviousSumti,
    #[serde(rename = "ce'u refers to the current abstraction")]
    CehuCurrentAbstraction,
    #[serde(rename = "ra is intentionally vague and is not resolved heuristically")]
    RaVague,
    #[serde(rename = "ru is intentionally vague and is not resolved heuristically")]
    RuVague,
    #[serde(rename = "ke'a refers to the current relative-clause head")]
    KehaCurrentRelativeHead,
    #[serde(
        rename = "utterance pro-sumti resolves to a neighboring utterance when determined by form"
    )]
    NeighborUtteranceByForm,
    #[serde(rename = "vo'a-series refers to a place of the current bridi")]
    VohaCurrentBridiPlace,
    #[serde(rename = "later da/de/di mentions refer to the active variable binding")]
    DaActiveVariableBinding,
    #[serde(rename = "KOhA resolves through an explicit GOI binding")]
    KohaGoiBinding,
    #[serde(rename = "go'i repeats the previous bridi")]
    GohiPreviousBridi,
    #[serde(rename = "go'e repeats the second-prior bridi")]
    GoheSecondPriorBridi,
    #[serde(rename = "this GOhA form is context-sensitive and is not resolved heuristically")]
    GohaUnresolvedContextSensitive,
    #[serde(rename = "nei refers to the current bridi")]
    NeiCurrentBridi,
    #[serde(rename = "no'a refers to an outer bridi")]
    NohaOuterBridi,
    #[serde(rename = "prenex binding resolves this pro-selbri word")]
    PrenexBindingProSelbri,
    #[serde(rename = "CEI binding resolves this pro-bridi word")]
    CeiBindingProBridi,
    #[serde(rename = "CEI binding resolves this broda-series bridi")]
    CeiBindingBroda,
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceEdge {
    pub id: ReferenceEdgeId,
    pub kind: ReferenceKind,
    pub source: RawSyntaxNodeId,
    pub target: ReferenceTarget,
    pub rule: ReferenceRule,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[invariant(true)]
pub enum ReferenceAnalysisError {
    #[error("syntax index did not contain the root text node")]
    MissingRootNode,
}

#[derive(Debug)]
#[invariant(true)]
pub struct GeneratedReferenceAnalysis<'tree> {
    pub syntax_index: GeneratedSyntaxIndex<'tree>,
    pub place_analysis: PlaceAnalysis,
    pub discourse_references: DiscourseReferences,
}

impl<'tree> GeneratedReferenceAnalysis<'tree> {
    #[requires(true)]
    #[ensures(true)]
    pub fn analyze(syntax: &'tree GeneratedTextSyntax) -> Result<Self, ReferenceAnalysisError> {
        analyze_generated_references(syntax)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn fixture_projection(&self) -> ReferenceFixtureProjection {
        ReferenceFixtureProjection::from_generated_analysis(self)
    }

    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|text| !text.is_empty()) || ret.is_err())]
    pub fn fixture_projection_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.fixture_projection())
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_ok_and(|analysis| analysis.syntax_index.node_count() > 0) || ret.is_err())]
pub fn analyze_generated_references<'tree>(
    syntax: &'tree GeneratedTextSyntax,
) -> Result<GeneratedReferenceAnalysis<'tree>, ReferenceAnalysisError> {
    let syntax_index = GeneratedSyntaxIndex::new(syntax)?;
    let place_analysis = PlaceAnalysis::analyze_generated(&syntax_index, syntax);
    let discourse_references =
        DiscourseReferences::analyze_generated(&syntax_index, &place_analysis, syntax);
    Ok(GeneratedReferenceAnalysis {
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
    assignments: Vec<SumtiPlaceAssignment>,
    assignment_ids_by_sumti: HashMap<SumtiNodeId, Vec<SumtiPlaceAssignmentId>>,
    assignment_ids_by_term: HashMap<TermNodeId, Vec<SumtiPlaceAssignmentId>>,
    assignment_ids_by_frame: HashMap<SelbriPlaceFrameId, Vec<SumtiPlaceAssignmentId>>,
    assignment_ids_by_frame_slot:
        HashMap<(SelbriPlaceFrameId, PlaceSlot), Vec<SumtiPlaceAssignmentId>>,
}

impl PlaceAnalysis {
    #[requires(true)]
    #[ensures(true)]
    fn analyze_generated<'tree>(
        index: &GeneratedSyntaxIndex<'tree>,
        syntax: &'tree GeneratedTextSyntax,
    ) -> Self {
        let mut builder = GeneratedPlaceAnalysisBuilder::new(index);
        GeneratedSyntaxTreeWalkable::walk_with(syntax, &mut builder);
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
    pub fn assignments(&self) -> &[SumtiPlaceAssignment] {
        &self.assignments
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn assignment(&self, id: SumtiPlaceAssignmentId) -> Option<&SumtiPlaceAssignment> {
        self.assignments.get(id.0)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn assignments_for_sumti(&self, sumti: SumtiNodeId) -> &[SumtiPlaceAssignmentId] {
        self.assignment_ids_by_sumti
            .get(&sumti)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn assignments_for_term(&self, term: TermNodeId) -> &[SumtiPlaceAssignmentId] {
        self.assignment_ids_by_term
            .get(&term)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn assignments_for_frame(&self, frame: SelbriPlaceFrameId) -> &[SumtiPlaceAssignmentId] {
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
    ) -> &[SumtiPlaceAssignmentId] {
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
    ) -> Option<SumtiNodeId> {
        self.assignments_for_frame_slot(frame, slot)
            .first()
            .and_then(|id| self.assignment(*id))
            .map(|assignment| assignment.sumti)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[invariant(true)]
pub struct DiscourseReferences {
    edges: Vec<ReferenceEdge>,
}

#[invariant(byte_start <= byte_end)]
#[invariant(char_start <= char_end)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SyntaxSpanKey {
    pub source_id: Option<SourceId>,
    pub byte_start: usize,
    pub byte_end: usize,
    pub char_start: usize,
    pub char_end: usize,
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
    pub assignments: Vec<FixtureSumtiAssignment>,
    pub selbri_places: Vec<FixtureSelbriPlace>,
    pub references: Vec<FixtureReferenceEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct FixturePlaceFrame {
    pub index: usize,
    pub node: FixtureSpanKey,
    pub kind: PlaceFrameKind,
    pub selbri: Option<FixtureSpanKey>,
    pub tanru_unit: Option<FixtureSpanKey>,
    pub propagation: FixturePlaceFramePropagation,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[invariant(true)]
#[invariant(::Forward => true)]
#[invariant(::Conversion => true)]
#[invariant(::Jai => true)]
#[invariant(::ConnectiveBranches => true)]
#[invariant(::Compound => true)]
#[invariant(::Co => true)]
pub enum FixturePlaceFramePropagation {
    None,
    Forward { inner: usize },
    Conversion { inner: usize, converted_place: u8 },
    Jai { inner: usize },
    ConnectiveBranches { branches: Vec<usize> },
    Compound { head: usize, modifiers: Vec<usize> },
    Co { leading: usize, trailing: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[invariant(true)]
#[invariant(::Numbered => true)]
#[invariant(::Modal => true)]
#[invariant(::PlaceQuestion => true)]
pub enum FixturePlaceSlot {
    Numbered { place: u8 },
    Modal { tag: Option<FixtureSpanKey> },
    PlaceQuestion,
    Fai,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct FixtureSumtiAssignment {
    pub frame: usize,
    pub frame_node: FixtureSpanKey,
    pub selbri: Option<FixtureSpanKey>,
    pub tanru_unit: Option<FixtureSpanKey>,
    pub slot: FixturePlaceSlot,
    pub sumti: FixtureSpanKey,
    pub term: Option<FixtureSpanKey>,
    pub source: AssignmentSource,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub struct FixtureSelbriPlace {
    pub frame: usize,
    pub selbri: FixtureSpanKey,
    pub place: u8,
    pub sumti: FixtureSpanKey,
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
    pub fn from_generated_analysis(analysis: &GeneratedReferenceAnalysis<'_>) -> Self {
        let mut frames = analysis
            .place_analysis
            .frames()
            .iter()
            .filter_map(|frame| generated_fixture_frame(analysis, frame))
            .collect::<Vec<_>>();
        frames.sort();

        let mut assignments = analysis
            .place_analysis
            .assignments()
            .iter()
            .filter_map(|assignment| generated_fixture_assignment(analysis, assignment))
            .collect::<Vec<_>>();
        assignments.sort();

        let mut selbri_places = analysis
            .place_analysis
            .assignments()
            .iter()
            .filter_map(|assignment| generated_fixture_relation_place(analysis, assignment))
            .collect::<Vec<_>>();
        selbri_places.sort();
        selbri_places.dedup();

        let mut references = analysis
            .discourse_references
            .edges()
            .iter()
            .filter_map(|edge| generated_fixture_reference_edge(analysis, edge))
            .collect::<Vec<_>>();
        references.sort();

        Self {
            frames,
            assignments,
            selbri_places,
            references,
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_fixture_frame(
    analysis: &GeneratedReferenceAnalysis<'_>,
    frame: &SelbriPlaceFrame,
) -> Option<FixturePlaceFrame> {
    Some(FixturePlaceFrame {
        index: frame.id.0,
        node: fixture_span_key_for_generated_node(&analysis.syntax_index, frame.node)?,
        kind: frame.kind,
        selbri: frame.selbri.and_then(|selbri| {
            fixture_span_key_for_generated_node(&analysis.syntax_index, selbri.0)
        }),
        tanru_unit: frame.tanru_unit.and_then(|tanru_unit| {
            fixture_span_key_for_generated_node(&analysis.syntax_index, tanru_unit.0)
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
        PlaceFramePropagation::ConnectiveBranches { branches } => {
            FixturePlaceFramePropagation::ConnectiveBranches {
                branches: branches.iter().map(|branch| branch.0).collect(),
            }
        }
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
fn generated_fixture_assignment(
    analysis: &GeneratedReferenceAnalysis<'_>,
    assignment: &SumtiPlaceAssignment,
) -> Option<FixtureSumtiAssignment> {
    let frame = analysis.place_analysis.frame(assignment.frame)?;
    Some(FixtureSumtiAssignment {
        frame: assignment.frame.0,
        frame_node: fixture_span_key_for_generated_node(&analysis.syntax_index, frame.node)?,
        selbri: frame.selbri.and_then(|selbri| {
            fixture_span_key_for_generated_node(&analysis.syntax_index, selbri.0)
        }),
        tanru_unit: frame.tanru_unit.and_then(|tanru_unit| {
            fixture_span_key_for_generated_node(&analysis.syntax_index, tanru_unit.0)
        }),
        slot: generated_fixture_place_slot(&analysis.syntax_index, assignment.slot),
        sumti: fixture_span_key_for_generated_node(&analysis.syntax_index, assignment.sumti.0)?,
        term: assignment
            .term
            .and_then(|term| fixture_span_key_for_generated_node(&analysis.syntax_index, term.0)),
        source: assignment.source,
    })
}

#[requires(true)]
#[ensures(true)]
fn generated_fixture_relation_place(
    analysis: &GeneratedReferenceAnalysis<'_>,
    assignment: &SumtiPlaceAssignment,
) -> Option<FixtureSelbriPlace> {
    let PlaceSlot::Numbered(place) = assignment.slot else {
        return None;
    };
    let frame = analysis.place_analysis.frame(assignment.frame)?;
    let selbri = frame.selbri.map(|selbri| selbri.0).unwrap_or(frame.node);
    Some(FixtureSelbriPlace {
        frame: assignment.frame.0,
        selbri: fixture_span_key_for_generated_node(&analysis.syntax_index, selbri)?,
        place: place.get(),
        sumti: fixture_span_key_for_generated_node(&analysis.syntax_index, assignment.sumti.0)?,
    })
}

#[requires(true)]
#[ensures(true)]
fn generated_fixture_reference_edge(
    analysis: &GeneratedReferenceAnalysis<'_>,
    edge: &ReferenceEdge,
) -> Option<FixtureReferenceEdge> {
    Some(FixtureReferenceEdge {
        kind: edge.kind,
        source: fixture_span_key_for_generated_node(&analysis.syntax_index, edge.source)?,
        target: generated_fixture_reference_target(analysis, &edge.target)?,
    })
}

#[requires(true)]
#[ensures(true)]
fn generated_fixture_reference_target(
    analysis: &GeneratedReferenceAnalysis<'_>,
    target: &ReferenceTarget,
) -> Option<FixtureReferenceTarget> {
    match target {
        ReferenceTarget::ResolvedNode(node) => Some(FixtureReferenceTarget::ResolvedNode {
            node: fixture_span_key_for_generated_node(&analysis.syntax_index, *node)?,
        }),
        ReferenceTarget::ResolvedFrame(frame) => {
            let frame_data = analysis.place_analysis.frame(*frame)?;
            Some(FixtureReferenceTarget::ResolvedFrame {
                frame: frame.0,
                frame_node: fixture_span_key_for_generated_node(
                    &analysis.syntax_index,
                    frame_data.node,
                )?,
            })
        }
        ReferenceTarget::AmbiguousNodes(nodes) => {
            let mut projected = nodes
                .iter()
                .filter_map(|node| {
                    fixture_span_key_for_generated_node(&analysis.syntax_index, *node)
                })
                .collect::<Vec<_>>();
            projected.sort();
            Some(FixtureReferenceTarget::AmbiguousNodes { nodes: projected })
        }
        ReferenceTarget::Unresolved(reason) => Some(FixtureReferenceTarget::Unresolved {
            reason: reason.clone(),
        }),
        ReferenceTarget::Vague(kind) => Some(FixtureReferenceTarget::Vague { vague_kind: *kind }),
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_fixture_place_slot(
    index: &GeneratedSyntaxIndex<'_>,
    slot: PlaceSlot,
) -> FixturePlaceSlot {
    match slot {
        PlaceSlot::Numbered(place) => FixturePlaceSlot::Numbered { place: place.get() },
        PlaceSlot::Modal(tag) => FixturePlaceSlot::Modal {
            tag: tag.and_then(|node| fixture_span_key_for_generated_node(index, node)),
        },
        PlaceSlot::PlaceQuestion => FixturePlaceSlot::PlaceQuestion,
        PlaceSlot::Fai => FixturePlaceSlot::Fai,
    }
}

#[requires(true)]
#[ensures(true)]
fn span_key_for_generated_node(
    index: &GeneratedSyntaxIndex<'_>,
    node: RawSyntaxNodeId,
) -> Option<SyntaxSpanKey> {
    let metadata = index.metadata(node)?;
    let first = metadata.first_source_span.as_ref()?;
    let last = metadata.last_source_span.as_ref()?;
    Some(new!(SyntaxSpanKey {
        source_id: first.source_id.clone(),
        byte_start: first.byte_start,
        byte_end: last.byte_end,
        char_start: first.char_start,
        char_end: last.char_end,
    }))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| key.length > 0))]
fn fixture_span_key_from_syntax_span(key: &SyntaxSpanKey) -> Option<FixtureSpanKey> {
    let length = key.byte_end.checked_sub(key.byte_start)?;
    if length == 0 {
        // Fixture projections are keyed by visible byte ranges; zero-width
        // generated nodes cannot be represented there, so their containing
        // projection record is omitted instead of inventing a synthetic range.
        return None;
    }
    Some(FixtureSpanKey {
        offset: key.byte_start,
        length,
    })
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| key.length > 0))]
fn fixture_span_key_for_generated_node(
    index: &GeneratedSyntaxIndex<'_>,
    node: RawSyntaxNodeId,
) -> Option<FixtureSpanKey> {
    let key = span_key_for_generated_node(index, node)?;
    fixture_span_key_from_syntax_span(&key)
}

impl DiscourseReferences {
    #[requires(true)]
    #[ensures(true)]
    fn analyze_generated<'tree>(
        index: &GeneratedSyntaxIndex<'tree>,
        places: &PlaceAnalysis,
        syntax: &'tree GeneratedTextSyntax,
    ) -> Self {
        let mut builder = GeneratedDiscourseReferenceBuilder::new(index, places);
        GeneratedSyntaxTreeWalkable::walk_with(syntax, &mut builder);
        builder.finish()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn edges(&self) -> &[ReferenceEdge] {
        &self.edges
    }
}

#[derive(Debug)]
#[invariant(true)]
struct GeneratedPlaceAnalysisBuilder<'index, 'tree> {
    index: &'index GeneratedSyntaxIndex<'tree>,
    frames: Vec<SelbriPlaceFrame>,
    frame_ids_by_node: HashMap<RawSyntaxNodeId, Vec<SelbriPlaceFrameId>>,
    assignments: Vec<SumtiPlaceAssignment>,
    assignment_ids_by_sumti: HashMap<SumtiNodeId, Vec<SumtiPlaceAssignmentId>>,
    assignment_ids_by_term: HashMap<TermNodeId, Vec<SumtiPlaceAssignmentId>>,
    assignment_ids_by_frame: HashMap<SelbriPlaceFrameId, Vec<SumtiPlaceAssignmentId>>,
    assignment_ids_by_frame_slot:
        HashMap<(SelbriPlaceFrameId, PlaceSlot), Vec<SumtiPlaceAssignmentId>>,
    max_numbered_place_by_frame: HashMap<SelbriPlaceFrameId, u8>,
    next_place_after_linked_arguments_by_frame: HashMap<SelbriPlaceFrameId, u8>,
    cursor_blocking_assignments: HashSet<SumtiPlaceAssignmentId>,
}

impl<'index, 'tree> GeneratedPlaceAnalysisBuilder<'index, 'tree> {
    #[requires(true)]
    #[ensures(ret.frames.is_empty())]
    fn new(index: &'index GeneratedSyntaxIndex<'tree>) -> Self {
        Self {
            index,
            frames: Vec::new(),
            frame_ids_by_node: HashMap::new(),
            assignments: Vec::new(),
            assignment_ids_by_sumti: HashMap::new(),
            assignment_ids_by_term: HashMap::new(),
            assignment_ids_by_frame: HashMap::new(),
            assignment_ids_by_frame_slot: HashMap::new(),
            max_numbered_place_by_frame: HashMap::new(),
            next_place_after_linked_arguments_by_frame: HashMap::new(),
            cursor_blocking_assignments: HashSet::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> PlaceAnalysis {
        PlaceAnalysis {
            frames: self.frames,
            frame_ids_by_node: self.frame_ids_by_node,
            assignments: self.assignments,
            assignment_ids_by_sumti: self.assignment_ids_by_sumti,
            assignment_ids_by_term: self.assignment_ids_by_term,
            assignment_ids_by_frame: self.assignment_ids_by_frame,
            assignment_ids_by_frame_slot: self.assignment_ids_by_frame_slot,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_predicate(&mut self, bridi: &'tree generated::BridiSyntax) -> SelbriPlaceFrameId {
        self.analyze_predicate_with_initial_place(bridi, 1)
    }

    #[requires(initial_place > 0)]
    #[ensures(true)]
    fn analyze_predicate_with_initial_place(
        &mut self,
        bridi: &'tree generated::BridiSyntax,
        initial_place: u8,
    ) -> SelbriPlaceFrameId {
        let leading_terms = generated_bridi_leading_terms(bridi);
        let tail = generated_bridi_tail(bridi);
        let branch_initial_place =
            next_generated_place_after_common_terms(initial_place, leading_terms);
        let tail = self.analyze_bridi_tail(tail, branch_initial_place);
        let predicate_raw = self.raw_for_node(bridi);
        let shared_branch_terms = tail.branch_cursors.is_some() || tail.frames.len() > 1;
        let predicate_frame = self.add_frame(
            predicate_raw,
            PlaceFrameKind::Bridi,
            None,
            None,
            propagation_connective_branches(tail.frames),
        );
        let mut cursors =
            vec![self.cursor_with_existing_assignments(predicate_frame, initial_place)];
        let leading_source = if shared_branch_terms {
            AssignmentSource::SharedHeadTerm
        } else {
            AssignmentSource::SequentialTerm
        };
        self.assign_terms(&mut cursors, leading_terms, leading_source);
        for cursor in &mut cursors {
            cursor.ensure_next_place_at_least(2);
            self.apply_linked_argument_cursor(cursor);
        }
        let tail_source = if shared_branch_terms {
            AssignmentSource::SharedTailTerm
        } else {
            AssignmentSource::SequentialTerm
        };
        self.assign_term_refs(&mut cursors, &tail.terms, tail_source);
        predicate_frame
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_bridi_tail(
        &mut self,
        tail: &'tree generated::BridiTailSyntax,
        gek_branch_initial_place: u8,
    ) -> GeneratedBridiTailAnalysis<'tree> {
        match tail {
            generated::BridiTailSyntax::ZantufaGroupedBridiTail(tail) => {
                let mut analysis =
                    self.analyze_bridi_tail(&tail.bridi_tail, gek_branch_initial_place);
                analysis.terms.extend(tail.tail_terms.iter());
                analysis
            }
            generated::BridiTailSyntax::BridiTailWithPossibleTailTerms(tail) => {
                self.analyze_bridi_tail_with_possible_tail_terms(tail, gek_branch_initial_place)
            }
            generated::BridiTailSyntax::BridiTailWithoutTailTerms(tail) => {
                self.analyze_bridi_tail_without_tail_terms(tail, gek_branch_initial_place)
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_bridi_tail_without_tail_terms(
        &mut self,
        tail: &'tree generated::BridiTailWithoutTailTermsSyntax,
        gek_branch_initial_place: u8,
    ) -> GeneratedBridiTailAnalysis<'tree> {
        let first = self.analyze_afterthought_bridi_tail_without_tail_terms(
            &tail.first,
            gek_branch_initial_place,
        );
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
                self.walk_node(tense_modal);
            }
            let mut continuation =
                self.analyze_bridi_tail(&ke_continuation.bridi_tail, gek_branch_initial_place);
            let continuation_cursors = self.consume_branch_tail_cursors(&mut continuation);
            branches.extend(continuation.frames);
            first_branch_cursors.extend(continuation_cursors);
            self.assign_terms(
                &mut first_branch_cursors,
                &ke_continuation.tail_terms,
                AssignmentSource::SharedTailTerm,
            );
            branch_cursors = Some(first_branch_cursors);
        }
        let frame = self.add_frame(
            self.raw_for_node(tail),
            PlaceFrameKind::BridiTail,
            None,
            None,
            propagation_connective_branches(branches),
        );
        GeneratedBridiTailAnalysis {
            frames: vec![frame],
            terms,
            branch_cursors,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_bridi_tail_with_possible_tail_terms(
        &mut self,
        tail: &'tree generated::BridiTailWithPossibleTailTermsSyntax,
        gek_branch_initial_place: u8,
    ) -> GeneratedBridiTailAnalysis<'tree> {
        let first = self.analyze_afterthought_bridi_tail(&tail.first, gek_branch_initial_place);
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
                self.walk_node(tense_modal);
            }
            let mut continuation =
                self.analyze_bridi_tail(&ke_continuation.bridi_tail, gek_branch_initial_place);
            let continuation_cursors = self.consume_branch_tail_cursors(&mut continuation);
            branches.extend(continuation.frames);
            first_branch_cursors.extend(continuation_cursors);
            self.assign_terms(
                &mut first_branch_cursors,
                &ke_continuation.tail_terms,
                AssignmentSource::SharedTailTerm,
            );
            branch_cursors = Some(first_branch_cursors);
        }
        let frame = self.add_frame(
            self.raw_for_node(tail),
            PlaceFrameKind::BridiTail,
            None,
            None,
            propagation_connective_branches(branches),
        );
        GeneratedBridiTailAnalysis {
            frames: vec![frame],
            terms,
            branch_cursors,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_afterthought_bridi_tail_without_tail_terms(
        &mut self,
        tail: &'tree generated::AfterthoughtBridiTailWithoutTailTermsSyntax,
        gek_branch_initial_place: u8,
    ) -> GeneratedBridiTailAnalysis<'tree> {
        let mut analysis = self.analyze_bo_grouped_bridi_tail_without_tail_terms(
            &tail.0.first,
            gek_branch_initial_place,
        );
        let mut branch_cursors = if tail.0.links.is_empty() {
            analysis.branch_cursors.take()
        } else {
            Some(self.consume_branch_tail_cursors(&mut analysis))
        };
        for continuation in &tail.0.links {
            let mut next = self.analyze_bo_grouped_bridi_tail_without_tail_terms(
                &continuation.bridi_tail,
                gek_branch_initial_place,
            );
            if let Some(cursors) = branch_cursors.as_mut() {
                let next_cursors = self.consume_branch_tail_cursors(&mut next);
                cursors.extend(next_cursors);
            }
            analysis.frames.extend(next.frames);
        }
        let frame = self.add_frame(
            self.raw_for_node(tail),
            PlaceFrameKind::BridiTail,
            None,
            None,
            propagation_connective_branches(analysis.frames),
        );
        GeneratedBridiTailAnalysis {
            frames: vec![frame],
            terms: analysis.terms,
            branch_cursors,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_afterthought_bridi_tail(
        &mut self,
        tail: &'tree generated::AfterthoughtBridiTailSyntax,
        gek_branch_initial_place: u8,
    ) -> GeneratedBridiTailAnalysis<'tree> {
        let mut analysis =
            self.analyze_bo_grouped_bridi_tail(&tail.0.first, gek_branch_initial_place);
        let mut branch_cursors = if tail.0.links.is_empty() {
            analysis.branch_cursors.take()
        } else {
            Some(self.consume_branch_tail_cursors(&mut analysis))
        };
        for continuation in &tail.0.links {
            let mut next = self
                .analyze_bo_grouped_bridi_tail(&continuation.bridi_tail, gek_branch_initial_place);
            if let Some(cursors) = branch_cursors.as_mut() {
                let next_cursors = self.consume_branch_tail_cursors(&mut next);
                cursors.extend(next_cursors);
                self.assign_terms(
                    cursors,
                    &continuation.tail_terms,
                    AssignmentSource::SharedTailTerm,
                );
            }
            analysis.frames.extend(next.frames);
        }
        let frame = self.add_frame(
            self.raw_for_node(tail),
            PlaceFrameKind::BridiTail,
            None,
            None,
            propagation_connective_branches(analysis.frames),
        );
        GeneratedBridiTailAnalysis {
            frames: vec![frame],
            terms: analysis.terms,
            branch_cursors,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_bo_grouped_bridi_tail_without_tail_terms(
        &mut self,
        tail: &'tree generated::BoGroupedBridiTailWithoutTailTermsSyntax,
        gek_branch_initial_place: u8,
    ) -> GeneratedBridiTailAnalysis<'tree> {
        let mut analysis = self
            .analyze_simple_bridi_tail_without_tail_terms(&tail.first, gek_branch_initial_place);
        let mut branch_cursors = analysis.branch_cursors.take();
        if let Some(continuation) = tail.bo_continuation.as_deref() {
            let mut active_cursors = if let Some(cursors) = branch_cursors.take() {
                cursors
            } else {
                self.consume_branch_tail_cursors(&mut analysis)
            };
            if let Some(tense_modal) = continuation.tense_modal.as_deref() {
                self.walk_node(tense_modal);
            }
            let mut next = self.analyze_bo_grouped_bridi_tail_without_tail_terms(
                &continuation.bridi_tail,
                gek_branch_initial_place,
            );
            let next_cursors = self.consume_branch_tail_cursors(&mut next);
            analysis.frames.extend(next.frames);
            active_cursors.extend(next_cursors);
            branch_cursors = Some(active_cursors);
        }
        let frame = self.add_frame(
            self.raw_for_node(tail),
            PlaceFrameKind::BridiTail,
            None,
            None,
            propagation_connective_branches(analysis.frames),
        );
        GeneratedBridiTailAnalysis {
            frames: vec![frame],
            terms: analysis.terms,
            branch_cursors,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_bo_grouped_bridi_tail(
        &mut self,
        tail: &'tree generated::BoGroupedBridiTailSyntax,
        gek_branch_initial_place: u8,
    ) -> GeneratedBridiTailAnalysis<'tree> {
        let mut analysis = self.analyze_simple_bridi_tail(&tail.first, gek_branch_initial_place);
        let mut branch_cursors = analysis.branch_cursors.take();
        if let Some(continuation) = tail.bo_continuation.as_deref() {
            let mut active_cursors = if let Some(cursors) = branch_cursors.take() {
                cursors
            } else {
                self.consume_branch_tail_cursors(&mut analysis)
            };
            if let Some(tense_modal) = continuation.tense_modal.as_deref() {
                self.walk_node(tense_modal);
            }
            let mut next = self
                .analyze_bo_grouped_bridi_tail(&continuation.bridi_tail, gek_branch_initial_place);
            let next_cursors = self.consume_branch_tail_cursors(&mut next);
            analysis.frames.extend(next.frames);
            active_cursors.extend(next_cursors);
            self.assign_terms(
                &mut active_cursors,
                &continuation.tail_terms,
                AssignmentSource::SharedTailTerm,
            );
            branch_cursors = Some(active_cursors);
        }
        let frame = self.add_frame(
            self.raw_for_node(tail),
            PlaceFrameKind::BridiTail,
            None,
            None,
            propagation_connective_branches(analysis.frames),
        );
        GeneratedBridiTailAnalysis {
            frames: vec![frame],
            terms: analysis.terms,
            branch_cursors,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_simple_bridi_tail_without_tail_terms(
        &mut self,
        tail: &'tree generated::SimpleBridiTailWithoutTailTermsSyntax,
        gek_branch_initial_place: u8,
    ) -> GeneratedBridiTailAnalysis<'tree> {
        match tail {
            generated::SimpleBridiTailWithoutTailTermsSyntax::SelbriSimpleBridiTailWithoutTailTerms(tail) => {
                let relation_frame = self.analyze_relation(&tail.selbri);
                let frame = self.add_frame(
                    self.raw_for_node(tail),
                    PlaceFrameKind::BridiTail,
                    None,
                    None,
                    propagation_forward(relation_frame),
                );
                GeneratedBridiTailAnalysis {
                    frames: vec![frame],
                    terms: Vec::new(),
                    branch_cursors: None,
                }
            }
            generated::SimpleBridiTailWithoutTailTermsSyntax::ForethoughtSimpleBridiTailWithoutTailTerms(tail) => {
                let frames = self.analyze_forethought_bridi_connection_without_tail_terms(
                    &tail.0,
                    gek_branch_initial_place,
                );
                GeneratedBridiTailAnalysis {
                    frames,
                    terms: Vec::new(),
                    branch_cursors: None,
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_simple_bridi_tail(
        &mut self,
        tail: &'tree generated::SimpleBridiTailSyntax,
        gek_branch_initial_place: u8,
    ) -> GeneratedBridiTailAnalysis<'tree> {
        match tail {
            generated::SimpleBridiTailSyntax::SelbriSimpleBridiTail(tail) => {
                let relation_frame = self.analyze_relation(&tail.selbri);
                let mut terms = tail.terms.iter().collect::<Vec<_>>();
                if let Some(seltau_frame) = self.co_seltau_term_frame(relation_frame) {
                    let mut cursors = vec![self.cursor_with_existing_assignments(seltau_frame, 2)];
                    self.assign_term_refs(&mut cursors, &terms, AssignmentSource::CoSeltauTerm);
                    terms.clear();
                }
                let frame = self.add_frame(
                    self.raw_for_node(tail),
                    PlaceFrameKind::BridiTail,
                    None,
                    None,
                    propagation_forward(relation_frame),
                );
                GeneratedBridiTailAnalysis {
                    frames: vec![frame],
                    terms,
                    branch_cursors: None,
                }
            }
            generated::SimpleBridiTailSyntax::ForethoughtSimpleBridiTail(tail) => {
                let frames =
                    self.analyze_forethought_bridi_connection(&tail.0, gek_branch_initial_place);
                GeneratedBridiTailAnalysis {
                    frames,
                    terms: Vec::new(),
                    branch_cursors: None,
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_forethought_bridi_connection(
        &mut self,
        connection: &'tree generated::ForethoughtBridiConnectionSyntax,
        branch_initial_place: u8,
    ) -> Vec<SelbriPlaceFrameId> {
        match connection {
            generated::ForethoughtBridiConnectionSyntax::DirectForethoughtBridiConnection(
                connection,
            ) => {
                let first_frame = self.analyze_subbridi_frame_with_initial_place(
                    &connection.first,
                    branch_initial_place,
                );
                let second_frame = self.analyze_subbridi_frame_with_initial_place(
                    &connection.first_branch.branch,
                    branch_initial_place,
                );
                let mut branch_frames = vec![first_frame, second_frame];
                for branch in &connection.additional_branches {
                    branch_frames.push(self.analyze_subbridi_frame_with_initial_place(
                        &branch.branch,
                        branch_initial_place,
                    ));
                }
                let mut cursors = branch_frames
                    .iter()
                    .map(|frame| {
                        self.cursor_with_existing_assignments(*frame, branch_initial_place)
                    })
                    .collect::<Vec<_>>();
                self.assign_terms(
                    &mut cursors,
                    &connection.tail_terms,
                    AssignmentSource::SharedTailTerm,
                );
                branch_frames
            }
            generated::ForethoughtBridiConnectionSyntax::GroupedForethoughtBridiConnection(
                connection,
            ) => {
                if let Some(tense_modal) = connection.tense_modal.as_deref() {
                    self.walk_node(tense_modal);
                }
                self.analyze_forethought_bridi_connection(&connection.inner, branch_initial_place)
            }
            generated::ForethoughtBridiConnectionSyntax::NegatedForethoughtBridiConnection(
                connection,
            ) => self.analyze_forethought_bridi_connection(&connection.inner, branch_initial_place),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_forethought_bridi_connection_without_tail_terms(
        &mut self,
        connection: &'tree generated::ForethoughtBridiConnectionWithoutTailTermsSyntax,
        branch_initial_place: u8,
    ) -> Vec<SelbriPlaceFrameId> {
        match connection {
            generated::ForethoughtBridiConnectionWithoutTailTermsSyntax::DirectForethoughtBridiConnectionWithoutTailTerms(connection) => {
                let first_frame =
                    self.analyze_subbridi_frame_with_initial_place(&connection.first, branch_initial_place);
                let second_frame =
                    self.analyze_subbridi_frame_with_initial_place(&connection.first_branch.branch, branch_initial_place);
                let mut branch_frames = vec![first_frame, second_frame];
                for branch in &connection.additional_branches {
                    branch_frames.push(self.analyze_subbridi_frame_with_initial_place(
                        &branch.branch,
                        branch_initial_place,
                    ));
                }
                branch_frames
            }
            generated::ForethoughtBridiConnectionWithoutTailTermsSyntax::GroupedForethoughtBridiConnectionWithoutTailTerms(connection) => {
                if let Some(tense_modal) = connection.tense_modal.as_deref() {
                    self.walk_node(tense_modal);
                }
                self.analyze_forethought_bridi_connection_without_tail_terms(&connection.inner, branch_initial_place)
            }
            generated::ForethoughtBridiConnectionWithoutTailTermsSyntax::NegatedForethoughtBridiConnectionWithoutTailTerms(connection) => {
                self.analyze_forethought_bridi_connection_without_tail_terms(&connection.inner, branch_initial_place)
            }
        }
    }

    #[requires(initial_place > 0)]
    #[ensures(true)]
    fn analyze_subbridi_frame_with_initial_place(
        &mut self,
        subbridi: &'tree generated::SubbridiSyntax,
        initial_place: u8,
    ) -> SelbriPlaceFrameId {
        match subbridi {
            generated::SubbridiSyntax::BridiSubbridi(subbridi) => {
                self.analyze_predicate_with_initial_place(&subbridi.0, initial_place)
            }
            generated::SubbridiSyntax::PrenexSubbridi(subbridi) => {
                for term in &subbridi.prenex_terms {
                    self.walk_node(term);
                }
                self.analyze_subbridi_frame_with_initial_place(
                    &subbridi.inner_subbridi,
                    initial_place,
                )
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_relation(&mut self, selbri: &'tree generated::SelbriSyntax) -> SelbriPlaceFrameId {
        match selbri {
            generated::SelbriSyntax::TaggedSelbri(selbri) => {
                self.walk_node(&selbri.tense_modal);
                let inner = self.analyze_untagged_relation(&selbri.inner_selbri);
                self.add_frame(
                    self.raw_for_node(selbri),
                    PlaceFrameKind::Forwarding,
                    Some(SelbriNodeId(self.raw_for_node(selbri))),
                    None,
                    propagation_forward(inner),
                )
            }
            generated::SelbriSyntax::UntaggedSelbri(selbri) => {
                self.analyze_untagged_relation(selbri)
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_untagged_relation(
        &mut self,
        selbri: &'tree generated::UntaggedSelbriSyntax,
    ) -> SelbriPlaceFrameId {
        match selbri {
            generated::UntaggedSelbriSyntax::NegatedSelbri(selbri) => {
                let inner = self.analyze_relation(&selbri.inner_selbri);
                self.add_frame(
                    self.raw_for_node(selbri),
                    PlaceFrameKind::Forwarding,
                    Some(SelbriNodeId(self.raw_for_node(selbri))),
                    None,
                    propagation_forward(inner),
                )
            }
            generated::UntaggedSelbriSyntax::CoSelbri(selbri) => self.analyze_co_selbri(selbri),
            generated::UntaggedSelbriSyntax::ForethoughtSelbriConnection(selbri) => {
                let leading = self.analyze_relation(&selbri.leading_selbri);
                let mut branches =
                    vec![leading, self.analyze_relation(&selbri.first_branch.selbri)];
                for branch in &selbri.additional_branches {
                    branches.push(self.analyze_relation(&branch.selbri));
                }
                self.add_frame(
                    self.raw_for_node(selbri),
                    PlaceFrameKind::ConnectiveBranching,
                    Some(SelbriNodeId(self.raw_for_node(selbri))),
                    None,
                    propagation_connective_branches(branches),
                )
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_co_selbri(
        &mut self,
        selbri: &'tree generated::CoSelbriSyntax,
    ) -> SelbriPlaceFrameId {
        let leading = self.analyze_connected_selbri(&selbri.leading_selbri);
        if let Some(tail) = &selbri.co_tail {
            let trailing = self.analyze_co_selbri(&tail.trailing_selbri);
            return self.add_frame(
                self.raw_for_node(selbri),
                PlaceFrameKind::CoInverted,
                Some(SelbriNodeId(self.raw_for_node(selbri))),
                None,
                propagation_co(leading, trailing),
            );
        }
        self.add_frame(
            self.raw_for_node(selbri),
            PlaceFrameKind::Forwarding,
            Some(SelbriNodeId(self.raw_for_node(selbri))),
            None,
            propagation_forward(leading),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_connected_selbri(
        &mut self,
        selbri: &'tree generated::ConnectedSelbriSyntax,
    ) -> SelbriPlaceFrameId {
        let leading = self.analyze_tanru_selbri(&selbri.leading_selbri);
        if selbri.continuations.is_empty() {
            return leading;
        }
        let mut branches = vec![leading];
        for continuation in &selbri.continuations {
            branches.push(self.analyze_tanru_selbri(&continuation.trailing_selbri));
        }
        self.add_frame(
            self.raw_for_node(selbri),
            PlaceFrameKind::ConnectiveBranching,
            Some(SelbriNodeId(self.raw_for_node(selbri))),
            None,
            propagation_connective_branches(branches),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_tanru_selbri(
        &mut self,
        selbri: &'tree generated::TanruSelbriSyntax,
    ) -> SelbriPlaceFrameId {
        let mut unit_frames = Vec::new();
        unit_frames.push(self.analyze_relation_unit(&selbri.first_unit));
        for unit in &selbri.additional_units {
            unit_frames.push(self.analyze_relation_unit(unit));
        }
        let head = *unit_frames
            .last()
            .expect("tanru selbri grammar always has a first unit");
        let modifiers = unit_frames[..unit_frames.len().saturating_sub(1)].to_vec();
        self.add_frame(
            self.raw_for_node(selbri),
            PlaceFrameKind::Compound,
            Some(SelbriNodeId(self.raw_for_node(selbri))),
            None,
            propagation_compound(head, modifiers),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_relation_unit(
        &mut self,
        unit: &'tree generated::TanruUnitSyntax,
    ) -> SelbriPlaceFrameId {
        let mut unit_frames = Vec::new();
        unit_frames.push(self.analyze_bo_or_linked_tanru_unit(&unit.0.first));
        for continuation in &unit.0.links {
            unit_frames.push(self.analyze_bo_or_linked_tanru_unit(&continuation.trailing_unit));
        }
        if unit_frames.len() == 1 {
            return unit_frames[0];
        }
        self.add_frame(
            self.raw_for_node(unit),
            PlaceFrameKind::ConnectiveBranching,
            None,
            Some(TanruUnitNodeId(self.raw_for_node(unit))),
            propagation_connective_branches(unit_frames),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_bo_or_linked_tanru_unit(
        &mut self,
        unit: &'tree generated::BoOrLinkedTanruUnitSyntax,
    ) -> SelbriPlaceFrameId {
        match unit {
            generated::BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => {
                self.analyze_linked_tanru_unit(unit)
            }
            generated::BoOrLinkedTanruUnitSyntax::BoundTanruUnit(unit) => {
                let leading = self.analyze_linked_tanru_unit(&unit.leading_unit);
                if let Some(tense_modal) = unit.bo_tense_modal.as_deref() {
                    self.walk_node(tense_modal);
                }
                let trailing = self.analyze_bo_or_linked_tanru_unit(&unit.trailing_unit);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::Compound,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_compound(trailing, vec![leading]),
                )
            }
            generated::BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => {
                let inner = self.analyze_linked_tanru_unit_for_cei(&unit.base);
                for assignment in &unit.assignments {
                    self.analyze_linked_tanru_unit_for_cei(&assignment.tanru_unit);
                }
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::Forwarding,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_forward(inner),
                )
            }
            generated::BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(unit) => {
                let leading = self.analyze_relation(&unit.leading_selbri);
                let mut branches = vec![
                    leading,
                    self.analyze_bo_or_linked_tanru_unit(&unit.first_branch.unit),
                ];
                for branch in &unit.additional_branches {
                    branches.push(self.analyze_bo_or_linked_tanru_unit(&branch.unit));
                }
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::ConnectiveBranching,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_connective_branches(branches),
                )
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_linked_tanru_unit(
        &mut self,
        unit: &'tree generated::LinkedTanruUnitSyntax,
    ) -> SelbriPlaceFrameId {
        let inner = self.analyze_tanru_unit_atom(&unit.base);
        if let Some(linkargs) = &unit.linkargs {
            self.assign_link_arguments(inner, linkargs);
        }
        self.add_frame(
            self.raw_for_node(unit),
            PlaceFrameKind::LinkedUnit,
            None,
            Some(TanruUnitNodeId(self.raw_for_node(unit))),
            propagation_forward(inner),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_linked_tanru_unit_for_cei(
        &mut self,
        unit: &'tree generated::LinkedTanruUnitForCeiSyntax,
    ) -> SelbriPlaceFrameId {
        let inner = self.analyze_tanru_unit_atom_for_cei(&unit.base);
        if let Some(linkargs) = &unit.linkargs {
            self.assign_link_arguments(inner, linkargs);
        }
        self.add_frame(
            self.raw_for_node(unit),
            PlaceFrameKind::LinkedUnit,
            None,
            Some(TanruUnitNodeId(self.raw_for_node(unit))),
            propagation_forward(inner),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_tanru_unit_atom(
        &mut self,
        unit: &'tree generated::TanruUnitAtomSyntax,
    ) -> SelbriPlaceFrameId {
        let inner = self.analyze_tanru_unit_atom_base(&unit.base);
        self.add_conversion_frames_for_tanru_unit_atom(inner, unit, &unit.conversions)
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_tanru_unit_atom_for_cei(
        &mut self,
        unit: &'tree generated::TanruUnitAtomForCeiSyntax,
    ) -> SelbriPlaceFrameId {
        let inner = self.analyze_tanru_unit_atom_base_for_cei(&unit.base);
        self.add_conversion_frames_for_tanru_unit_atom(inner, unit, &unit.conversions)
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_conversion_frames_for_tanru_unit_atom<N, F>(
        &mut self,
        inner: SelbriPlaceFrameId,
        unit: &'tree N,
        conversions: &[WithFreeModifiers<Token, F>],
    ) -> SelbriPlaceFrameId
    where
        N: GeneratedSyntaxTreeNode,
    {
        conversions
            .iter()
            .rev()
            .filter_map(generated_se_conversion_place)
            .filter_map(NonZeroU8::new)
            .fold(inner, |inner, converted_place| {
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::Converted,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_conversion(inner, converted_place),
                )
            })
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_tanru_unit_atom_base(
        &mut self,
        unit: &'tree generated::TanruUnitAtomBaseSyntax,
    ) -> SelbriPlaceFrameId {
        match unit {
            generated::TanruUnitAtomBaseSyntax::WordTanruUnit(_)
            | generated::TanruUnitAtomBaseSyntax::ProBridiTanruUnit(_)
            | generated::TanruUnitAtomBaseSyntax::GohaWordTanruUnit(_)
            | generated::TanruUnitAtomBaseSyntax::QuotedBridiSelbriTanruUnit(_)
            | generated::TanruUnitAtomBaseSyntax::QuotedTextSelbriTanruUnit(_)
            | generated::TanruUnitAtomBaseSyntax::OrdinalTanruUnit(_) => self.add_frame(
                self.raw_for_node(unit),
                PlaceFrameKind::TanruUnit,
                None,
                Some(TanruUnitNodeId(self.raw_for_node(unit))),
                propagation_none(),
            ),
            generated::TanruUnitAtomBaseSyntax::OperatorSelbriTanruUnit(unit) => {
                self.walk_node(&unit.mekso_operator);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::TanruUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
            generated::TanruUnitAtomBaseSyntax::ZantufaMeTanruUnit(unit) => {
                self.walk_node(unit);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::TanruUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
            generated::TanruUnitAtomBaseSyntax::ZantufaMexMoiTanruUnit(unit) => {
                self.walk_node(&unit.expression);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::TanruUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
            generated::TanruUnitAtomBaseSyntax::TagSelbriTanruUnit(unit) => {
                self.walk_node(&unit.tag);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::TanruUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
            generated::TanruUnitAtomBaseSyntax::SumtiSelbriTanruUnit(unit) => {
                self.walk_node(&unit.sumti);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::TanruUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
            generated::TanruUnitAtomBaseSyntax::TextSelbriTanruUnit(unit) => {
                self.walk_node(&unit.text);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::TanruUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
            generated::TanruUnitAtomBaseSyntax::GroupedTanruUnit(unit) => {
                let inner = self.analyze_connected_selbri(&unit.selbri);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::Forwarding,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_forward(inner),
                )
            }
            generated::TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit) => {
                let inner = self.analyze_scalar_negated_tanru_inner_unit(&unit.inner_unit);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::Forwarding,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_forward(inner),
                )
            }
            generated::TanruUnitAtomBaseSyntax::JaiModalTanruUnit(unit) => {
                if let Some(tense_modal) = unit.tense_modal.as_deref() {
                    self.walk_node(tense_modal);
                }
                let inner = self.analyze_jai_inner_tanru_unit(&unit.inner_unit);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::JaiConverted,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_jai(inner),
                )
            }
            generated::TanruUnitAtomBaseSyntax::PreposedLinkargsTanruUnit(unit) => {
                let inner = self.analyze_relation_unit(&unit.base);
                self.assign_link_arguments(inner, &unit.linkargs);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::LinkedUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_forward(inner),
                )
            }
            generated::TanruUnitAtomBaseSyntax::AbstractionTanruUnit(unit) => {
                let propagation = if generated_abstraction_is_property(unit) {
                    let inner = self.analyze_subbridi_frame_with_initial_place(&unit.subbridi, 1);
                    propagation_forward(inner)
                } else {
                    self.walk_node(&unit.subbridi);
                    propagation_none()
                };
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::Abstraction,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation,
                )
            }
            generated::TanruUnitAtomBaseSyntax::ZantufaStatementAbstractionTanruUnit(unit) => {
                self.walk_node(&unit.statement);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::Abstraction,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_tanru_unit_atom_base_for_cei(
        &mut self,
        unit: &'tree generated::TanruUnitAtomBaseForCeiSyntax,
    ) -> SelbriPlaceFrameId {
        match unit {
            generated::TanruUnitAtomBaseForCeiSyntax::ProBridiTanruUnit(_)
            | generated::TanruUnitAtomBaseForCeiSyntax::GohaWordTanruUnit(_)
            | generated::TanruUnitAtomBaseForCeiSyntax::WordTanruUnit(_)
            | generated::TanruUnitAtomBaseForCeiSyntax::QuotedBridiSelbriTanruUnit(_)
            | generated::TanruUnitAtomBaseForCeiSyntax::QuotedTextSelbriTanruUnit(_)
            | generated::TanruUnitAtomBaseForCeiSyntax::OrdinalTanruUnit(_) => self.add_frame(
                self.raw_for_node(unit),
                PlaceFrameKind::TanruUnit,
                None,
                Some(TanruUnitNodeId(self.raw_for_node(unit))),
                propagation_none(),
            ),
            generated::TanruUnitAtomBaseForCeiSyntax::OperatorSelbriTanruUnit(unit) => {
                self.walk_node(&unit.mekso_operator);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::TanruUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
            generated::TanruUnitAtomBaseForCeiSyntax::ZantufaMeTanruUnit(unit) => {
                self.walk_node(unit);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::TanruUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
            generated::TanruUnitAtomBaseForCeiSyntax::ZantufaMexMoiTanruUnit(unit) => {
                self.walk_node(&unit.expression);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::TanruUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
            generated::TanruUnitAtomBaseForCeiSyntax::TagSelbriTanruUnit(unit) => {
                self.walk_node(&unit.tag);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::TanruUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
            generated::TanruUnitAtomBaseForCeiSyntax::SumtiSelbriTanruUnit(unit) => {
                self.walk_node(&unit.sumti);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::TanruUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
            generated::TanruUnitAtomBaseForCeiSyntax::TextSelbriTanruUnit(unit) => {
                self.walk_node(&unit.text);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::TanruUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
            generated::TanruUnitAtomBaseForCeiSyntax::GroupedTanruUnit(unit) => {
                let inner = self.analyze_connected_selbri(&unit.selbri);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::Forwarding,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_forward(inner),
                )
            }
            generated::TanruUnitAtomBaseForCeiSyntax::ScalarNegatedTanruUnit(unit) => {
                let inner = self.analyze_scalar_negated_tanru_inner_unit(&unit.inner_unit);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::Forwarding,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_forward(inner),
                )
            }
            generated::TanruUnitAtomBaseForCeiSyntax::JaiModalTanruUnit(unit) => {
                if let Some(tense_modal) = unit.tense_modal.as_deref() {
                    self.walk_node(tense_modal);
                }
                let inner = self.analyze_jai_inner_tanru_unit(&unit.inner_unit);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::JaiConverted,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_jai(inner),
                )
            }
            generated::TanruUnitAtomBaseForCeiSyntax::PreposedLinkargsTanruUnit(unit) => {
                let inner = self.analyze_relation_unit(&unit.base);
                self.assign_link_arguments(inner, &unit.linkargs);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::LinkedUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_forward(inner),
                )
            }
            generated::TanruUnitAtomBaseForCeiSyntax::AbstractionTanruUnit(unit) => {
                let propagation = if generated_abstraction_is_property(unit) {
                    let inner = self.analyze_subbridi_frame_with_initial_place(&unit.subbridi, 1);
                    propagation_forward(inner)
                } else {
                    self.walk_node(&unit.subbridi);
                    propagation_none()
                };
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::Abstraction,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation,
                )
            }
            generated::TanruUnitAtomBaseForCeiSyntax::ZantufaStatementAbstractionTanruUnit(
                unit,
            ) => {
                self.walk_node(&unit.statement);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::Abstraction,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_scalar_negated_tanru_inner_unit(
        &mut self,
        unit: &'tree generated::ScalarNegatedTanruInnerUnitSyntax,
    ) -> SelbriPlaceFrameId {
        match unit {
            generated::ScalarNegatedTanruInnerUnitSyntax::TaggedSelbriGroupTanruUnit(unit) => {
                self.walk_node(&unit.tense_modal);
                self.analyze_connected_selbri(&unit.inner_selbri)
            }
            generated::ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(unit) => {
                self.analyze_tanru_unit_atom(unit)
            }
            generated::ScalarNegatedTanruInnerUnitSyntax::ProBridiTanruUnit(unit) => self
                .add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::TanruUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                ),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_jai_inner_tanru_unit(
        &mut self,
        unit: &'tree generated::JaiInnerTanruUnitSyntax,
    ) -> SelbriPlaceFrameId {
        match unit {
            generated::JaiInnerTanruUnitSyntax::ConvertedJaiInnerTanruUnit(unit) => {
                let inner = self.analyze_jai_inner_tanru_unit(&unit.inner_unit);
                let converted_place = generated_se_conversion_place(&unit.se)
                    .and_then(NonZeroU8::new)
                    .unwrap_or(NonZeroU8::new(2).expect("literal is non-zero"));
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::Converted,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_conversion(inner, converted_place),
                )
            }
            generated::JaiInnerTanruUnitSyntax::ScalarNegatedJaiInnerTanruUnit(unit) => {
                let inner = self.analyze_jai_inner_tanru_unit(&unit.inner_unit);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::Forwarding,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_forward(inner),
                )
            }
            generated::JaiInnerTanruUnitSyntax::GroupedJaiInnerTanruUnit(unit) => {
                let inner = self.analyze_connected_jai_inner_selbri(&unit.selbri);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::Forwarding,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_forward(inner),
                )
            }
            generated::JaiInnerTanruUnitSyntax::SumtiSelbriTanruUnit(unit) => {
                self.walk_node(&unit.sumti);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::TanruUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
            generated::JaiInnerTanruUnitSyntax::TextSelbriTanruUnit(unit) => {
                self.walk_node(&unit.text);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::TanruUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
            generated::JaiInnerTanruUnitSyntax::OperatorSelbriTanruUnit(unit) => {
                self.walk_node(&unit.mekso_operator);
                self.add_frame(
                    self.raw_for_node(unit),
                    PlaceFrameKind::TanruUnit,
                    None,
                    Some(TanruUnitNodeId(self.raw_for_node(unit))),
                    propagation_none(),
                )
            }
            generated::JaiInnerTanruUnitSyntax::QuotedBridiSelbriTanruUnit(_)
            | generated::JaiInnerTanruUnitSyntax::QuotedTextSelbriTanruUnit(_)
            | generated::JaiInnerTanruUnitSyntax::OrdinalTanruUnit(_)
            | generated::JaiInnerTanruUnitSyntax::ProBridiTanruUnit(_)
            | generated::JaiInnerTanruUnitSyntax::WordTanruUnit(_) => self.add_frame(
                self.raw_for_node(unit),
                PlaceFrameKind::TanruUnit,
                None,
                Some(TanruUnitNodeId(self.raw_for_node(unit))),
                propagation_none(),
            ),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_connected_jai_inner_selbri(
        &mut self,
        selbri: &'tree generated::ConnectedJaiInnerSelbriSyntax,
    ) -> SelbriPlaceFrameId {
        let leading = self.analyze_tanru_jai_inner_selbri(&selbri.leading_selbri);
        if selbri.continuations.is_empty() {
            return leading;
        }
        let mut branches = vec![leading];
        for continuation in &selbri.continuations {
            branches.push(self.analyze_tanru_jai_inner_selbri(&continuation.trailing_selbri));
        }
        self.add_frame(
            self.raw_for_node(selbri),
            PlaceFrameKind::ConnectiveBranching,
            Some(SelbriNodeId(self.raw_for_node(selbri))),
            None,
            propagation_connective_branches(branches),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_tanru_jai_inner_selbri(
        &mut self,
        selbri: &'tree generated::TanruJaiInnerSelbriSyntax,
    ) -> SelbriPlaceFrameId {
        let mut unit_frames = Vec::new();
        unit_frames.push(self.analyze_jai_inner_tanru_unit(&selbri.first_unit));
        for unit in &selbri.additional_units {
            unit_frames.push(self.analyze_jai_inner_tanru_unit(unit));
        }
        let head = *unit_frames
            .last()
            .expect("tanru jai-inner selbri grammar always has a first unit");
        let modifiers = unit_frames[..unit_frames.len().saturating_sub(1)].to_vec();
        self.add_frame(
            self.raw_for_node(selbri),
            PlaceFrameKind::Compound,
            Some(SelbriNodeId(self.raw_for_node(selbri))),
            None,
            propagation_compound(head, modifiers),
        )
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_terms(
        &mut self,
        cursors: &mut Vec<PlaceCursor>,
        terms: &'tree [generated::TermSyntax],
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
        terms: &[&'tree generated::TermSyntax],
        source: AssignmentSource,
    ) {
        for term in terms {
            self.assign_term(cursors, term, source);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_term(
        &mut self,
        cursors: &mut Vec<PlaceCursor>,
        term: &'tree generated::TermSyntax,
        source: AssignmentSource,
    ) {
        match term {
            generated::TermSyntax::SimpleTerm(simple) => {
                self.assign_simple_term(
                    cursors,
                    term,
                    GeneratedSimpleTermRef::from_simple(simple),
                    source,
                );
            }
            generated::TermSyntax::ConnectedTerm(connected) => {
                if let Some(leading) = GeneratedSimpleTermRef::from_bound(&connected.leading_term) {
                    self.assign_simple_term(cursors, term, leading, source);
                }
                for continuation in &connected.continuations {
                    if let Some(trailing) =
                        GeneratedSimpleTermRef::from_bound(&continuation.trailing_term)
                    {
                        self.assign_simple_term(cursors, term, trailing, source);
                    }
                }
            }
            generated::TermSyntax::BoundTermConnection(bound) => {
                self.assign_simple_term(
                    cursors,
                    term,
                    GeneratedSimpleTermRef::from_simple(&bound.leading_term),
                    AssignmentSource::TermsetBranch,
                );
                self.assign_simple_term(
                    cursors,
                    term,
                    GeneratedSimpleTermRef::from_simple(&bound.trailing_term),
                    AssignmentSource::TermsetBranch,
                );
            }
            generated::TermSyntax::TermsetGroup(group) => {
                self.assign_simple_term(
                    cursors,
                    term,
                    GeneratedSimpleTermRef::from_simple(&group.leading_term),
                    AssignmentSource::TermsetBranch,
                );
                for continuation in &group.continuations {
                    self.assign_simple_term(
                        cursors,
                        term,
                        GeneratedSimpleTermRef::from_simple(&continuation.trailing_term),
                        AssignmentSource::TermsetBranch,
                    );
                }
            }
            generated::TermSyntax::PeheTermsetConnection(connection) => {
                self.assign_pehe_termset_operand(cursors, term, &connection.leading_term);
                for continuation in &connection.continuations {
                    self.assign_pehe_termset_operand(cursors, term, &continuation.trailing_term);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_pehe_termset_operand(
        &mut self,
        cursors: &mut Vec<PlaceCursor>,
        outer_term: &'tree generated::TermSyntax,
        term: &'tree generated::PeheTermsetOperandSyntax,
    ) {
        match term {
            generated::PeheTermsetOperandSyntax::SimpleTerm(simple) => {
                self.assign_simple_term(
                    cursors,
                    outer_term,
                    GeneratedSimpleTermRef::from_simple(simple),
                    AssignmentSource::TermsetBranch,
                );
            }
            generated::PeheTermsetOperandSyntax::TermsetGroup(group) => {
                self.assign_simple_term(
                    cursors,
                    outer_term,
                    GeneratedSimpleTermRef::from_simple(&group.leading_term),
                    AssignmentSource::TermsetBranch,
                );
                for continuation in &group.continuations {
                    self.assign_simple_term(
                        cursors,
                        outer_term,
                        GeneratedSimpleTermRef::from_simple(&continuation.trailing_term),
                        AssignmentSource::TermsetBranch,
                    );
                }
            }
            generated::PeheTermsetOperandSyntax::BoundTermConnection(bound) => {
                self.assign_simple_term(
                    cursors,
                    outer_term,
                    GeneratedSimpleTermRef::from_simple(&bound.leading_term),
                    AssignmentSource::TermsetBranch,
                );
                self.assign_simple_term(
                    cursors,
                    outer_term,
                    GeneratedSimpleTermRef::from_simple(&bound.trailing_term),
                    AssignmentSource::TermsetBranch,
                );
            }
            generated::PeheTermsetOperandSyntax::StagBoundTermConnection(_) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_simple_term(
        &mut self,
        cursors: &mut Vec<PlaceCursor>,
        outer_term: &'tree generated::TermSyntax,
        term: GeneratedSimpleTermRef<'tree>,
        source: AssignmentSource,
    ) {
        match term {
            GeneratedSimpleTermRef::SumtiTerm(term) => {
                self.assign_argument_term_to_cursors(cursors, outer_term, &term.0, source);
            }
            GeneratedSimpleTermRef::PlaceTaggedSumtiTerm(term) => {
                let slot = generated_fa_place_slot(&term.fa);
                self.assign_tagged_or_elided_argument_to_cursors(
                    cursors,
                    outer_term,
                    &term.sumti,
                    slot,
                    AssignmentSource::FaTerm,
                );
            }
            GeneratedSimpleTermRef::TaggedSumtiTerm(term) => {
                self.walk_node(&term.tense_modal);
                let slot = Some(modal_slot(Some(
                    self.raw_for_node(term.tense_modal.as_ref()),
                )));
                self.assign_tagged_or_elided_argument_to_cursors(
                    cursors,
                    outer_term,
                    &term.sumti,
                    slot,
                    AssignmentSource::ModalTerm,
                );
            }
            GeneratedSimpleTermRef::JaiTaggedSumtiTerm(term) => {
                if let Some(tense_modal) = term.tag.as_deref() {
                    self.walk_node(tense_modal);
                }
                self.assign_argument_to_cursors(
                    cursors,
                    outer_term,
                    &term.sumti,
                    Some(fai_slot()),
                    AssignmentSource::FaTerm,
                );
            }
            GeneratedSimpleTermRef::NuhiTermset(term) => {
                for term in &term.termset {
                    self.assign_term(cursors, term, AssignmentSource::TermsetBranch);
                }
            }
            GeneratedSimpleTermRef::KeTermset(term) => {
                for term in &term.termset {
                    self.assign_term(cursors, term, AssignmentSource::TermsetBranch);
                }
            }
            GeneratedSimpleTermRef::TaggedSumtiBeforeTagTerm(term) => self.walk_node(term),
            GeneratedSimpleTermRef::NoihaAdverbialTerm(term) => self.walk_node(term),
            GeneratedSimpleTermRef::FihoiAdverbialTerm(term) => self.walk_node(term),
            GeneratedSimpleTermRef::SoiAdverbialTerm(term) => self.walk_node(term),
            GeneratedSimpleTermRef::NaKuTerm(term) => self.walk_node(term),
            GeneratedSimpleTermRef::BareNaTerm(term) => self.walk_node(term),
            GeneratedSimpleTermRef::ForethoughtTermset(term) => self.walk_node(term),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_argument_term_to_cursors(
        &mut self,
        cursors: &mut Vec<PlaceCursor>,
        term: &'tree generated::TermSyntax,
        sumti: &'tree generated::SumtiSyntax,
        source: AssignmentSource,
    ) {
        self.assign_argument_to_cursors(cursors, term, sumti, None, source);
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_tagged_or_elided_argument_to_cursors(
        &mut self,
        cursors: &mut Vec<PlaceCursor>,
        term: &'tree generated::TermSyntax,
        sumti: &'tree generated::TaggedOrElidedSumtiSyntax,
        explicit_slot: Option<PlaceSlot>,
        source: AssignmentSource,
    ) {
        match sumti {
            generated::TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                self.assign_argument_to_cursors(cursors, term, sumti, explicit_slot, source);
            }
            generated::TaggedOrElidedSumtiSyntax::TaggedElidedSumti(sumti) => {
                self.assign_elided_argument_to_cursors(cursors, term, sumti, explicit_slot, source);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_argument_to_cursors(
        &mut self,
        cursors: &mut Vec<PlaceCursor>,
        term: &'tree generated::TermSyntax,
        sumti: &'tree generated::SumtiSyntax,
        explicit_slot: Option<PlaceSlot>,
        source: AssignmentSource,
    ) {
        self.walk_node(sumti);
        let argument_id = SumtiNodeId(self.raw_for_node(sumti));
        let term_id = TermNodeId(self.raw_for_node(term));
        let blocks_cursor = generated_sumti_spine_cmavo(sumti) != Some(Cmavo::Cehu);
        for cursor in cursors {
            let slot = explicit_slot.unwrap_or_else(|| cursor.next_numbered_slot());
            self.add_assignment(
                cursor.frame,
                slot,
                argument_id,
                Some(term_id),
                source,
                blocks_cursor,
            );
            cursor.record_slot(slot);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_elided_argument_to_cursors(
        &mut self,
        cursors: &mut Vec<PlaceCursor>,
        term: &'tree generated::TermSyntax,
        sumti: &'tree generated::TaggedElidedSumtiSyntax,
        explicit_slot: Option<PlaceSlot>,
        source: AssignmentSource,
    ) {
        let argument_id = SumtiNodeId(self.raw_for_elided_sumti(sumti));
        let term_id = TermNodeId(self.raw_for_node(term));
        for cursor in cursors {
            let slot = explicit_slot.unwrap_or_else(|| cursor.next_numbered_slot());
            self.add_assignment(cursor.frame, slot, argument_id, Some(term_id), source, true);
            cursor.record_slot(slot);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_link_arguments(
        &mut self,
        frame: SelbriPlaceFrameId,
        linkargs: &'tree generated::LinkargsSyntax,
    ) {
        let mut cursor = PlaceCursor::new_at(frame, 2);
        let mut assigned_any = false;
        assigned_any |= GeneratedLinkedSumtiRef::from_linked_term(&linkargs.first_link)
            .is_some_and(|link| self.assign_linked_sumti(&mut cursor, link));
        for link in &linkargs.bei_links {
            assigned_any |= GeneratedLinkedSumtiRef::from_linked_term(&link.link)
                .is_some_and(|link| self.assign_linked_sumti(&mut cursor, link));
        }
        if assigned_any {
            self.next_place_after_linked_arguments_by_frame
                .insert(frame, cursor.next_place);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_linked_sumti(
        &mut self,
        cursor: &mut PlaceCursor,
        link: GeneratedLinkedSumtiRef<'tree>,
    ) -> bool {
        match link {
            GeneratedLinkedSumtiRef::Plain(link) => {
                self.assign_link_argument(cursor, &link.0, None);
                true
            }
            GeneratedLinkedSumtiRef::PlaceTagged(link) => self
                .assign_tagged_or_elided_link_argument(
                    cursor,
                    &link.sumti,
                    generated_fa_place_slot(&link.fa),
                ),
            GeneratedLinkedSumtiRef::TenseTagged(link) => {
                self.walk_node(&link.tense_modal);
                self.assign_tagged_or_elided_link_argument(
                    cursor,
                    &link.sumti,
                    Some(modal_slot(Some(
                        self.raw_for_node(link.tense_modal.as_ref()),
                    ))),
                )
            }
            GeneratedLinkedSumtiRef::Empty => false,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_tagged_or_elided_link_argument(
        &mut self,
        cursor: &mut PlaceCursor,
        sumti: &'tree generated::TaggedOrElidedSumtiSyntax,
        explicit_slot: Option<PlaceSlot>,
    ) -> bool {
        match sumti {
            generated::TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                self.assign_link_argument(cursor, sumti, explicit_slot);
            }
            generated::TaggedOrElidedSumtiSyntax::TaggedElidedSumti(sumti) => {
                self.assign_elided_link_argument(cursor, sumti, explicit_slot);
            }
        }
        true
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_link_argument(
        &mut self,
        cursor: &mut PlaceCursor,
        sumti: &'tree generated::SumtiSyntax,
        explicit_slot: Option<PlaceSlot>,
    ) {
        self.walk_node(sumti);
        let argument_id = SumtiNodeId(self.raw_for_node(sumti));
        let slot = explicit_slot.unwrap_or_else(|| cursor.next_numbered_slot());
        self.add_assignment(
            cursor.frame,
            slot,
            argument_id,
            None,
            AssignmentSource::LinkedSumti,
            generated_sumti_spine_cmavo(sumti) != Some(Cmavo::Cehu),
        );
        cursor.record_slot(slot);
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_elided_link_argument(
        &mut self,
        cursor: &mut PlaceCursor,
        sumti: &'tree generated::TaggedElidedSumtiSyntax,
        explicit_slot: Option<PlaceSlot>,
    ) {
        let slot = explicit_slot.unwrap_or_else(|| cursor.next_numbered_slot());
        self.add_assignment(
            cursor.frame,
            slot,
            SumtiNodeId(self.raw_for_elided_sumti(sumti)),
            None,
            AssignmentSource::LinkedSumti,
            true,
        );
        cursor.record_slot(slot);
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_frame(
        &mut self,
        node: RawSyntaxNodeId,
        kind: PlaceFrameKind,
        selbri: Option<SelbriNodeId>,
        tanru_unit: Option<TanruUnitNodeId>,
        propagation: PlaceFramePropagation,
    ) -> SelbriPlaceFrameId {
        let id = SelbriPlaceFrameId(self.frames.len());
        self.frames.push(new!(SelbriPlaceFrame {
            id: id,
            node: node,
            kind: kind,
            selbri: selbri,
            tanru_unit: tanru_unit,
            propagation: propagation,
        }));
        self.frame_ids_by_node.entry(node).or_default().push(id);
        id
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_assignment(
        &mut self,
        frame: SelbriPlaceFrameId,
        slot: PlaceSlot,
        sumti: SumtiNodeId,
        term: Option<TermNodeId>,
        source: AssignmentSource,
        blocks_cursor: bool,
    ) {
        let mut visited = HashSet::new();
        self.add_assignment_recursive(
            frame,
            slot,
            sumti,
            term,
            source,
            blocks_cursor,
            &mut visited,
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_assignment_recursive(
        &mut self,
        frame: SelbriPlaceFrameId,
        slot: PlaceSlot,
        sumti: SumtiNodeId,
        term: Option<TermNodeId>,
        source: AssignmentSource,
        blocks_cursor: bool,
        visited: &mut HashSet<(SelbriPlaceFrameId, PlaceSlot)>,
    ) {
        if !visited.insert((frame, slot)) {
            return;
        }
        let id = SumtiPlaceAssignmentId(self.assignments.len());
        if blocks_cursor {
            self.cursor_blocking_assignments.insert(id);
        }
        self.assignments.push(new!(SumtiPlaceAssignment {
            id: id,
            frame: frame,
            slot: slot,
            sumti: sumti,
            term: term,
            source: source,
        }));
        self.assignment_ids_by_sumti
            .entry(sumti)
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
        if let Some(place) = slot.numbered_index() {
            self.max_numbered_place_by_frame
                .entry(frame)
                .and_modify(|max_place| *max_place = (*max_place).max(place))
                .or_insert(place);
        }
        self.propagate_assignment(frame, slot, sumti, term, source, blocks_cursor, visited);
    }

    #[requires(true)]
    #[ensures(true)]
    fn propagate_assignment(
        &mut self,
        frame: SelbriPlaceFrameId,
        slot: PlaceSlot,
        sumti: SumtiNodeId,
        term: Option<TermNodeId>,
        source: AssignmentSource,
        blocks_cursor: bool,
        visited: &mut HashSet<(SelbriPlaceFrameId, PlaceSlot)>,
    ) {
        let Some(frame_data) = self.frames.get(frame.0).cloned() else {
            return;
        };
        match frame_data.into_data().propagation {
            PlaceFramePropagation::None => {}
            PlaceFramePropagation::Forward { inner } => self.add_assignment_recursive(
                inner,
                slot,
                sumti,
                term,
                propagated_assignment_source(source),
                blocks_cursor,
                visited,
            ),
            PlaceFramePropagation::Conversion {
                inner,
                converted_place,
            } => self.add_assignment_recursive(
                inner,
                convert_slot(slot, converted_place),
                sumti,
                term,
                propagated_assignment_source(source),
                blocks_cursor,
                visited,
            ),
            PlaceFramePropagation::Jai { inner } => match slot {
                PlaceSlot::Fai => self.add_assignment_recursive(
                    inner,
                    numbered_slot(NonZeroU8::new(1).expect("literal is non-zero")),
                    sumti,
                    term,
                    propagated_assignment_source(source),
                    blocks_cursor,
                    visited,
                ),
                PlaceSlot::Numbered(place) if place.get() > 1 => self.add_assignment_recursive(
                    inner,
                    numbered_slot(place),
                    sumti,
                    term,
                    propagated_assignment_source(source),
                    blocks_cursor,
                    visited,
                ),
                PlaceSlot::Numbered(_) | PlaceSlot::Modal(_) | PlaceSlot::PlaceQuestion => {}
            },
            PlaceFramePropagation::ConnectiveBranches { branches } => {
                for branch in branches {
                    self.add_assignment_recursive(
                        branch,
                        slot,
                        sumti,
                        term,
                        propagated_assignment_source(source),
                        blocks_cursor,
                        visited,
                    );
                }
            }
            PlaceFramePropagation::Compound { head, .. } => self.add_assignment_recursive(
                head,
                slot,
                sumti,
                term,
                propagated_assignment_source(source),
                blocks_cursor,
                visited,
            ),
            PlaceFramePropagation::Co { leading, .. } => self.add_assignment_recursive(
                leading,
                slot,
                sumti,
                term,
                propagated_assignment_source(source),
                blocks_cursor,
                visited,
            ),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn branch_tail_cursors(&self, frames: &[SelbriPlaceFrameId]) -> Vec<PlaceCursor> {
        frames
            .iter()
            .copied()
            .map(|frame| self.tail_cursor_with_existing_assignments(frame, 2))
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
        for place in 1..=self.max_existing_numbered_place_for_frame(frame) {
            let slot = numbered_slot(NonZeroU8::new(place).expect("range starts at one"));
            if self.frame_slot_has_existing_assignment(frame, slot) {
                cursor.mark_filled_slot(slot);
            }
        }
        cursor
    }

    #[requires(start > 0)]
    #[ensures(ret.frame == frame)]
    fn tail_cursor_with_existing_assignments(
        &self,
        frame: SelbriPlaceFrameId,
        start: u8,
    ) -> PlaceCursor {
        let mut cursor = self.cursor_with_existing_assignments(frame, start);
        self.apply_linked_argument_cursor(&mut cursor);
        cursor
    }

    #[requires(true)]
    #[ensures(cursor.frame == old(cursor.frame))]
    fn apply_linked_argument_cursor(&self, cursor: &mut PlaceCursor) {
        if let Some(next_place) = self.next_place_after_linked_arguments(cursor.frame) {
            cursor.reset_next_place(next_place);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_place_after_linked_arguments(&self, frame: SelbriPlaceFrameId) -> Option<u8> {
        let mut visited = HashSet::new();
        self.next_place_after_linked_arguments_recursive(frame, &mut visited)
    }

    #[requires(true)]
    #[ensures(true)]
    fn next_place_after_linked_arguments_recursive(
        &self,
        frame: SelbriPlaceFrameId,
        visited: &mut HashSet<SelbriPlaceFrameId>,
    ) -> Option<u8> {
        if let Some(next_place) = self.next_place_after_linked_arguments_by_frame.get(&frame) {
            return Some(*next_place);
        }
        if !visited.insert(frame) {
            return None;
        }
        let Some(frame_data) = self.frames.get(frame.0) else {
            return None;
        };
        match &frame_data.propagation {
            PlaceFramePropagation::None => None,
            PlaceFramePropagation::Forward { inner } => {
                self.next_place_after_linked_arguments_recursive(*inner, visited)
            }
            PlaceFramePropagation::Conversion { inner, .. } => {
                self.next_place_after_linked_arguments_recursive(*inner, visited)
            }
            PlaceFramePropagation::Jai { inner } => {
                self.next_place_after_linked_arguments_recursive(*inner, visited)
            }
            PlaceFramePropagation::ConnectiveBranches { branches } => branches
                .iter()
                .filter_map(|branch| {
                    self.next_place_after_linked_arguments_recursive(*branch, visited)
                })
                .max(),
            PlaceFramePropagation::Compound { head, .. } => {
                self.next_place_after_linked_arguments_recursive(*head, visited)
            }
            PlaceFramePropagation::Co { leading, .. } => {
                self.next_place_after_linked_arguments_recursive(*leading, visited)
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn max_existing_numbered_place_for_frame(&self, frame: SelbriPlaceFrameId) -> u8 {
        let mut visited = HashSet::new();
        self.max_existing_numbered_place_for_frame_recursive(frame, &mut visited)
    }

    #[requires(true)]
    #[ensures(true)]
    fn max_existing_numbered_place_for_frame_recursive(
        &self,
        frame: SelbriPlaceFrameId,
        visited: &mut HashSet<SelbriPlaceFrameId>,
    ) -> u8 {
        let mut max_place = self
            .max_numbered_place_by_frame
            .get(&frame)
            .copied()
            .unwrap_or(0);
        if !visited.insert(frame) {
            return max_place;
        }
        let Some(frame_data) = self.frames.get(frame.0) else {
            return max_place;
        };
        match &frame_data.propagation {
            PlaceFramePropagation::None => max_place,
            PlaceFramePropagation::Forward { inner }
            | PlaceFramePropagation::Compound { head: inner, .. }
            | PlaceFramePropagation::Co { leading: inner, .. } => {
                max_place.max(self.max_existing_numbered_place_for_frame_recursive(*inner, visited))
            }
            PlaceFramePropagation::Conversion {
                inner,
                converted_place,
            } => {
                let inner_max =
                    self.max_existing_numbered_place_for_frame_recursive(*inner, visited);
                max_place = max_place.max(inner_max);
                if inner_max > 0 {
                    max_place = max_place.max(converted_place.get());
                }
                max_place
            }
            PlaceFramePropagation::Jai { inner } => {
                let inner_max =
                    self.max_existing_numbered_place_for_frame_recursive(*inner, visited);
                if inner_max > 1 {
                    max_place.max(inner_max)
                } else {
                    max_place
                }
            }
            PlaceFramePropagation::ConnectiveBranches { branches } => {
                branches.iter().fold(max_place, |max_place, branch| {
                    max_place
                        .max(self.max_existing_numbered_place_for_frame_recursive(*branch, visited))
                })
            }
        }
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
            } => self.frame_slot_has_existing_assignment_recursive(
                *inner,
                convert_slot(slot, *converted_place),
                visited,
            ),
            PlaceFramePropagation::Jai { inner } => match slot {
                PlaceSlot::Fai => self.frame_slot_has_existing_assignment_recursive(
                    *inner,
                    numbered_slot(NonZeroU8::new(1).expect("literal is non-zero")),
                    visited,
                ),
                PlaceSlot::Numbered(place) if place.get() > 1 => {
                    self.frame_slot_has_existing_assignment_recursive(*inner, slot, visited)
                }
                PlaceSlot::Numbered(_) | PlaceSlot::Modal(_) | PlaceSlot::PlaceQuestion => false,
            },
            PlaceFramePropagation::ConnectiveBranches { branches } => {
                branches.iter().any(|branch| {
                    self.frame_slot_has_existing_assignment_recursive(*branch, slot, visited)
                })
            }
            PlaceFramePropagation::Compound { head, .. } => {
                self.frame_slot_has_existing_assignment_recursive(*head, slot, visited)
            }
            PlaceFramePropagation::Co { leading, .. } => {
                self.frame_slot_has_existing_assignment_recursive(*leading, slot, visited)
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
                    .any(|assignment| self.cursor_blocking_assignments.contains(assignment))
            })
    }

    #[requires(true)]
    #[ensures(analysis.branch_cursors.is_none())]
    #[ensures(analysis.terms.is_empty())]
    fn consume_branch_tail_cursors(
        &mut self,
        analysis: &mut GeneratedBridiTailAnalysis<'tree>,
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
    fn co_seltau_term_frame(&self, frame: SelbriPlaceFrameId) -> Option<SelbriPlaceFrameId> {
        let frame_data = self.frames.get(frame.0)?;
        match &frame_data.propagation {
            PlaceFramePropagation::Co { trailing, .. } => Some(*trailing),
            PlaceFramePropagation::Forward { inner } => self.co_seltau_term_frame(*inner),
            PlaceFramePropagation::None
            | PlaceFramePropagation::Conversion { .. }
            | PlaceFramePropagation::Jai { .. }
            | PlaceFramePropagation::ConnectiveBranches { .. }
            | PlaceFramePropagation::Compound { .. } => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn raw_for_node<N: GeneratedSyntaxTreeNode>(&self, node: &'tree N) -> RawSyntaxNodeId {
        self.index.id_for_tree_node(node).unwrap_or_else(|| {
            panic!(
                "generated syntax node belongs to indexed syntax tree: {:?}",
                node.as_node_ref().map(|node| node.constructor_name())
            )
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn raw_for_elided_sumti(
        &self,
        sumti: &'tree generated::TaggedElidedSumtiSyntax,
    ) -> RawSyntaxNodeId {
        self.index
            .id_of(GeneratedSyntaxNodeRef::TaggedElidedSumtiSyntax(sumti))
            .expect("elided generated sumti node belongs to indexed syntax tree")
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_node<N>(&mut self, node: &'tree N)
    where
        N: GeneratedSyntaxTreeWalkable<'tree> + ?Sized,
    {
        GeneratedSyntaxTreeWalkable::walk_with(node, self);
    }
}

impl<'index, 'tree> GeneratedSyntaxTreeWalker<'tree>
    for GeneratedPlaceAnalysisBuilder<'index, 'tree>
{
    #[requires(true)]
    #[ensures(true)]
    fn walk_text(&mut self, node: &'tree GeneratedTextSyntax) {
        match node {
            generated::TextSyntax::ExplicitXauhaLohoiText(text) => {
                self.walk_node(&text.0);
            }
            generated::TextSyntax::RegularText(text) => {
                for free_modifier in &text.leading_free_modifiers {
                    self.walk_node(free_modifier);
                }
                for statement in &text.leading_i_statements {
                    for free_modifier in &statement.free_modifiers {
                        self.walk_node(free_modifier);
                    }
                }
                if let Some(paragraphs) = text.paragraphs.as_deref() {
                    self.walk_node(paragraphs);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_text_paragraphs(&mut self, node: &'tree generated::TextParagraphsSyntax) {
        match node {
            generated::TextParagraphsSyntax::TextParagraphWithAdditionalNiho(paragraphs) => {
                self.walk_node(paragraphs);
            }
            generated::TextParagraphsSyntax::TextNihoParagraphs(paragraphs) => {
                for paragraph in &paragraphs.0 {
                    self.walk_node(paragraph);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_text_paragraph_with_additional_niho(
        &mut self,
        node: &'tree generated::TextParagraphWithAdditionalNihoSyntax,
    ) {
        self.walk_node(&node.first);
        for paragraph in &node.additional_niho {
            self.walk_node(paragraph);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_paragraph(&mut self, node: &'tree generated::ParagraphSyntax) {
        match node {
            generated::ParagraphSyntax::SimpleParagraph(paragraph) => {
                self.walk_node(&paragraph.0);
            }
            generated::ParagraphSyntax::INihoParagraph(paragraph) => {
                for free_modifier in &paragraph.free_modifiers {
                    self.walk_node(free_modifier);
                }
                if let Some(statements) = paragraph.statements.as_deref() {
                    self.walk_node(statements);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_niho_paragraph(&mut self, node: &'tree generated::NihoParagraphSyntax) {
        for free_modifier in &node.free_modifiers {
            self.walk_node(free_modifier);
        }
        if let Some(statements) = node.statements.as_deref() {
            self.walk_node(statements);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_paragraph_statement_sequence(
        &mut self,
        node: &'tree generated::ParagraphStatementSequenceSyntax,
    ) {
        self.walk_node(&node.initial.0);
        for statement in &node.following {
            for free_modifier in &statement.free_modifiers {
                self.walk_node(free_modifier);
            }
            if let Some(statement) = statement.statement.as_deref() {
                self.walk_node(statement);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_statement_or_fragment(&mut self, node: &'tree generated::StatementOrFragmentSyntax) {
        match node {
            generated::StatementOrFragmentSyntax::ZantufaStatementTermsStatement(statement) => {
                self.walk_node(&statement.statement);
                self.walk_node(&statement.tail);
            }
            generated::StatementOrFragmentSyntax::StatementOrFragmentStatement(statement) => {
                self.walk_node(&statement.0);
            }
            generated::StatementOrFragmentSyntax::FragmentStatement(fragment) => {
                self.walk_node(fragment);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_statement(&mut self, node: &'tree generated::StatementSyntax) {
        match node {
            generated::StatementSyntax::StatementBase(statement) => {
                self.walk_node(statement);
            }
            generated::StatementSyntax::IStatementConnection(connection) => {
                self.walk_node(&connection.leading_statement);
                for continuation in &connection.continuations {
                    self.walk_node(continuation);
                }
            }
            generated::StatementSyntax::PreposedIStatementConnection(connection) => {
                self.walk_node(&connection.leading_statement);
                self.walk_node(&connection.trailing_statement);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_i_statement_connection_tail(
        &mut self,
        node: &'tree generated::IStatementConnectionTailSyntax,
    ) {
        match node {
            generated::IStatementConnectionTailSyntax::ChainedIConnectiveStatementTail(tail) => {
                self.walk_node(&tail.trailing_statement);
            }
            generated::IStatementConnectionTailSyntax::SimpleIConnectiveStatementTail(tail) => {
                self.walk_node(&tail.trailing_statement);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_statement_after_i_connective(
        &mut self,
        node: &'tree generated::StatementAfterIConnectiveSyntax,
    ) {
        match node {
            generated::StatementAfterIConnectiveSyntax::BridiStatement(statement) => {
                self.walk_node(statement);
            }
            generated::StatementAfterIConnectiveSyntax::TextGroupStatement(statement) => {
                self.walk_node(statement);
            }
            generated::StatementAfterIConnectiveSyntax::ForethoughtStatement(statement) => {
                self.walk_node(statement);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_text_group_statement(&mut self, node: &'tree generated::TextGroupStatementSyntax) {
        if let Some(tense_modal) = node.tense_modal.as_deref() {
            self.walk_node(tense_modal);
        }
        self.walk_node(&node.text);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_bridi_statement(&mut self, node: &'tree generated::BridiStatementSyntax) {
        self.analyze_predicate(&node.bridi);
        for continuation in &node.continuations {
            match continuation {
                generated::BridiStatementContinuationSyntax::BoBridiStatementContinuation(
                    continuation,
                ) => {
                    if let Some(tense_modal) = continuation.tense_modal.as_deref() {
                        self.walk_node(tense_modal);
                    }
                    self.walk_node(&continuation.trailing_subbridi);
                }
                generated::BridiStatementContinuationSyntax::KeBridiStatementContinuation(
                    continuation,
                ) => {
                    if let Some(tense_modal) = continuation.tense_modal.as_deref() {
                        self.walk_node(tense_modal);
                    }
                    self.walk_node(&continuation.trailing_subbridi);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_statement_base(&mut self, node: &'tree generated::StatementBaseSyntax) {
        match node {
            generated::StatementBaseSyntax::PrenexStatement(statement) => {
                for term in &statement.prenex_terms {
                    self.walk_node(term);
                }
                self.walk_node(&statement.inner_statement);
            }
            generated::StatementBaseSyntax::BridiStatement(statement) => {
                self.walk_node(statement);
            }
            generated::StatementBaseSyntax::TextGroupStatement(statement) => {
                self.walk_node(statement);
            }
            generated::StatementBaseSyntax::ForethoughtStatement(statement) => {
                self.walk_node(statement);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_forethought_statement(&mut self, node: &'tree generated::ForethoughtStatementSyntax) {
        self.walk_node(&node.first);
        self.walk_node(&node.first_branch.statement);
        for branch in &node.additional_branches {
            self.walk_node(&branch.statement);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_subbridi(&mut self, node: &'tree generated::SubbridiSyntax) {
        match node {
            generated::SubbridiSyntax::BridiSubbridi(subbridi) => {
                self.analyze_predicate(&subbridi.0);
            }
            generated::SubbridiSyntax::PrenexSubbridi(subbridi) => {
                for term in &subbridi.prenex_terms {
                    self.walk_node(term);
                }
                self.walk_node(&subbridi.inner_subbridi);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_bridi(&mut self, node: &'tree generated::BridiSyntax) {
        self.analyze_predicate(node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_term(&mut self, node: &'tree generated::TermSyntax) {
        match node {
            generated::TermSyntax::SimpleTerm(term) => self.walk_node(term),
            generated::TermSyntax::ConnectedTerm(term) => {
                self.walk_node(&term.leading_term);
                for continuation in &term.continuations {
                    self.walk_node(&continuation.trailing_term);
                }
            }
            generated::TermSyntax::BoundTermConnection(term) => {
                self.walk_node(&term.leading_term);
                self.walk_node(&term.trailing_term);
            }
            generated::TermSyntax::TermsetGroup(term) => {
                self.walk_node(&term.leading_term);
                for continuation in &term.continuations {
                    self.walk_node(&continuation.trailing_term);
                }
            }
            generated::TermSyntax::PeheTermsetConnection(term) => {
                self.walk_node(&term.leading_term);
                for continuation in &term.continuations {
                    self.walk_node(&continuation.trailing_term);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_pehe_termset_operand(&mut self, node: &'tree generated::PeheTermsetOperandSyntax) {
        match node {
            generated::PeheTermsetOperandSyntax::BoundTermConnection(term) => {
                self.walk_node(&term.leading_term);
                self.walk_node(&term.trailing_term);
            }
            generated::PeheTermsetOperandSyntax::TermsetGroup(term) => {
                self.walk_node(&term.leading_term);
                for continuation in &term.continuations {
                    self.walk_node(&continuation.trailing_term);
                }
            }
            generated::PeheTermsetOperandSyntax::SimpleTerm(term) => self.walk_node(term),
            generated::PeheTermsetOperandSyntax::StagBoundTermConnection(term) => {
                self.walk_node(term)
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_simple_term(&mut self, node: &'tree generated::SimpleTermSyntax) {
        match node {
            generated::SimpleTermSyntax::SumtiTerm(term) => {
                self.walk_node(&term.0);
            }
            generated::SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => {
                self.walk_node(&term.sumti);
            }
            generated::SimpleTermSyntax::TaggedSumtiTerm(term) => {
                self.walk_node(&term.tense_modal);
                self.walk_node(&term.sumti);
            }
            generated::SimpleTermSyntax::JaiTaggedSumtiTerm(term) => {
                if let Some(tense_modal) = term.tag.as_deref() {
                    self.walk_node(tense_modal);
                }
                self.walk_node(&term.sumti);
            }
            generated::SimpleTermSyntax::FihoiAdverbialTerm(term) => {
                self.walk_node(&term.statement);
            }
            generated::SimpleTermSyntax::SoiAdverbialTerm(term) => {
                self.walk_node(&term.statement);
            }
            generated::SimpleTermSyntax::NoihaAdverbialTerm(term) => match term {
                generated::NoihaAdverbialTermSyntax::NoihaVariableAdverbialTerm(term) => {
                    for free_modifier in &term.free_modifiers {
                        self.walk_node(free_modifier);
                    }
                    self.analyze_relation(&term.selbri);
                }
                generated::NoihaAdverbialTermSyntax::NoihaRelativeAdverbialTerm(term) => {
                    self.analyze_relation(&term.selbri);
                }
            },
            generated::SimpleTermSyntax::ForethoughtTermset(term) => {
                for term in &term.terms {
                    self.walk_node(term);
                }
                for term in &term.first_branch.terms {
                    self.walk_node(term);
                }
                for branch in &term.additional_branches {
                    for term in &branch.terms {
                        self.walk_node(term);
                    }
                }
            }
            generated::SimpleTermSyntax::NuhiTermset(term) => {
                for term in &term.termset {
                    self.walk_node(term);
                }
            }
            generated::SimpleTermSyntax::KeTermset(term) => {
                for term in &term.termset {
                    self.walk_node(term);
                }
            }
            generated::SimpleTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                self.walk_node(&term.0);
            }
            generated::SimpleTermSyntax::NaKuTerm(_)
            | generated::SimpleTermSyntax::BareNaTerm(_) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_bound_term(&mut self, node: &'tree generated::BoundTermSyntax) {
        let Some(node) = GeneratedSimpleTermRef::from_bound(node) else {
            return;
        };
        match node {
            GeneratedSimpleTermRef::SumtiTerm(term) => self.walk_node(&term.0),
            GeneratedSimpleTermRef::PlaceTaggedSumtiTerm(term) => self.walk_node(&term.sumti),
            GeneratedSimpleTermRef::TaggedSumtiTerm(term) => {
                self.walk_node(&term.tense_modal);
                self.walk_node(&term.sumti);
            }
            GeneratedSimpleTermRef::JaiTaggedSumtiTerm(term) => {
                if let Some(tense_modal) = term.tag.as_deref() {
                    self.walk_node(tense_modal);
                }
                self.walk_node(&term.sumti);
            }
            GeneratedSimpleTermRef::FihoiAdverbialTerm(term) => {
                self.walk_node(&term.statement);
            }
            GeneratedSimpleTermRef::SoiAdverbialTerm(term) => {
                self.walk_node(&term.statement);
            }
            GeneratedSimpleTermRef::NoihaAdverbialTerm(term) => match term {
                generated::NoihaAdverbialTermSyntax::NoihaVariableAdverbialTerm(term) => {
                    for free_modifier in &term.free_modifiers {
                        self.walk_node(free_modifier);
                    }
                    self.analyze_relation(&term.selbri);
                }
                generated::NoihaAdverbialTermSyntax::NoihaRelativeAdverbialTerm(term) => {
                    self.analyze_relation(&term.selbri);
                }
            },
            GeneratedSimpleTermRef::ForethoughtTermset(term) => {
                for term in &term.terms {
                    self.walk_node(term);
                }
                for term in &term.first_branch.terms {
                    self.walk_node(term);
                }
                for branch in &term.additional_branches {
                    for term in &branch.terms {
                        self.walk_node(term);
                    }
                }
            }
            GeneratedSimpleTermRef::NuhiTermset(term) => {
                for term in &term.termset {
                    self.walk_node(term);
                }
            }
            GeneratedSimpleTermRef::KeTermset(term) => {
                for term in &term.termset {
                    self.walk_node(term);
                }
            }
            GeneratedSimpleTermRef::TaggedSumtiBeforeTagTerm(term) => self.walk_node(&term.0),
            GeneratedSimpleTermRef::NaKuTerm(_) | GeneratedSimpleTermRef::BareNaTerm(_) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_sumti(&mut self, node: &'tree generated::SumtiSyntax) {
        generated::walk::sumti(self, node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_selbri(&mut self, node: &'tree generated::SelbriSyntax) {
        self.analyze_relation(node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_relative_clause_list(&mut self, node: &'tree generated::RelativeClauseListSyntax) {
        generated::walk::relative_clause_list(self, node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_text_quote(&mut self, _node: &'tree generated::TextQuoteSyntax) {
        // Direct quote-as-paragraph nodes were not reached by the old
        // place-analysis recursion. Quoted sumti still descend through the
        // explicit `walk_quote` override below.
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_quote(&mut self, node: &'tree generated::QuoteSyntax) {
        if let generated::QuoteSyntax::TextQuote(quote) = node {
            self.walk_node(&quote.text);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_linkargs(&mut self, node: &'tree generated::LinkargsSyntax) {
        generated::walk::linkargs(self, node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_bei_link(&mut self, node: &'tree generated::BeiLinkSyntax) {
        self.walk_node(&node.link);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_quantifier(&mut self, node: &'tree generated::QuantifierSyntax) {
        match node {
            generated::QuantifierSyntax::MeksoQuantifier(quantifier) => {
                self.walk_node(&quantifier.mekso);
            }
            generated::QuantifierSyntax::ZantufaRawMeksoQuantifier(quantifier) => {
                self.walk_node(&quantifier.0);
            }
            generated::QuantifierSyntax::ZantufaPriorityRawMeksoQuantifier(quantifier) => {
                self.walk_node(&quantifier.0);
            }
            generated::QuantifierSyntax::PaRunQuantifier(_) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_mekso(&mut self, node: &'tree generated::MeksoSyntax) {
        match node {
            generated::MeksoSyntax::ZantufaReversePolishMekso(expression) => {
                for operand in &expression.operands {
                    self.walk_node(operand);
                }
                self.walk_node(&expression.operator);
                for tail in &expression.tails {
                    for operand in &tail.operands {
                        self.walk_node(operand);
                    }
                    self.walk_node(&tail.operator);
                }
            }
            generated::MeksoSyntax::ZantufaInfixMekso(expression) => {
                self.walk_node(&expression.first_expression);
                for continuation in &expression.continuations {
                    for operator in &continuation.operators {
                        self.walk_node(operator);
                    }
                    if let Some(right_expression) = &continuation.right_expression {
                        self.walk_node(right_expression);
                    }
                }
            }
            generated::MeksoSyntax::InfixMekso(expression) => {
                self.walk_node(&expression.first_expression);
                for continuation in &expression.continuations {
                    self.walk_node(&continuation.operator);
                    self.walk_node(&continuation.right_expression);
                }
            }
            generated::MeksoSyntax::ReversePolishMekso(expression) => {
                self.walk_node(&expression.parts);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_mekso_precedence(&mut self, node: &'tree generated::MeksoPrecedenceSyntax) {
        self.walk_node(&node.left_expression);
        if let Some(tail) = &node.tail {
            self.walk_node(&tail.operator);
            self.walk_node(&tail.right_expression);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_mekso_base(&mut self, node: &'tree generated::MeksoBaseSyntax) {
        match node {
            generated::MeksoBaseSyntax::MeksoOperand(operand) => {
                self.walk_node(operand);
            }
            generated::MeksoBaseSyntax::ForethoughtCallMekso(call) => {
                self.walk_node(&call.operator);
                for operand in &call.operands {
                    self.walk_node(operand);
                }
            }
            generated::MeksoBaseSyntax::ZantufaBoGroupedMeksoBase(group) => {
                self.walk_node(&group.first);
                for continuation in &group.continuations {
                    self.walk_node(&continuation.expression);
                }
            }
            generated::MeksoBaseSyntax::ZantufaGroupedMeksoOperandSequence(group) => {
                for operand in &group.operands {
                    self.walk_node(operand);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_mekso_operand(&mut self, node: &'tree generated::MeksoOperandSyntax) {
        let connected = &node.connected_expression.0;
        self.walk_node(&connected.first);
        for continuation in &connected.links {
            self.walk_node(&continuation.trailing_expression);
        }
        if let Some(group) = &node.grouped_continuation {
            if let Some(tense_modal) = group.tense_modal.as_deref() {
                self.walk_node(tense_modal);
            }
            self.walk_node(&group.inner_expression);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_bound_or_simple_mekso_operand(
        &mut self,
        node: &'tree generated::BoundOrSimpleMeksoOperandSyntax,
    ) {
        match node {
            generated::BoundOrSimpleMeksoOperandSyntax::BoundMeksoOperand(operand) => {
                self.walk_node(&operand.left_expression);
                if let Some(tense_modal) = operand.tense_modal.as_deref() {
                    self.walk_node(tense_modal);
                }
                self.walk_node(&operand.right_expression);
            }
            generated::BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(operand) => {
                self.walk_node(operand);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_simple_mekso_operand(&mut self, node: &'tree generated::SimpleMeksoOperandSyntax) {
        match node {
            generated::SimpleMeksoOperandSyntax::ForethoughtMeksoOperand(operand) => {
                self.walk_node(&operand.left_expression);
                self.walk_node(&operand.right_expression);
            }
            generated::SimpleMeksoOperandSyntax::QualifiedMeksoOperand(operand) => {
                self.walk_node(&operand.inner_expression);
            }
            generated::SimpleMeksoOperandSyntax::LaheQualifiedMeksoOperand(operand) => {
                self.walk_node(&operand.inner_expression);
            }
            generated::SimpleMeksoOperandSyntax::ZantufaScalarNegatedMeksoOperand(operand) => {
                self.walk_node(&operand.inner_expression);
            }
            generated::SimpleMeksoOperandSyntax::ParenthesizedMeksoOperand(operand) => {
                self.walk_node(&operand.inner_expression);
            }
            generated::SimpleMeksoOperandSyntax::SumtiMeksoOperand(operand) => {
                self.walk_node(&operand.sumti);
            }
            generated::SimpleMeksoOperandSyntax::SelbriMeksoOperand(operand) => {
                self.analyze_relation(&operand.selbri);
            }
            generated::SimpleMeksoOperandSyntax::ZantufaSelbriMoheMeksoOperand(operand) => {
                self.analyze_relation(&operand.selbri);
            }
            generated::SimpleMeksoOperandSyntax::ArrayMeksoOperand(operand) => {
                for expression in &operand.expressions {
                    self.walk_node(expression);
                }
            }
            generated::SimpleMeksoOperandSyntax::NumberMekso(_)
            | generated::SimpleMeksoOperandSyntax::LerfuStringMekso(_) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_reverse_polish_parts(&mut self, node: &'tree generated::ReversePolishPartsSyntax) {
        self.walk_node(&node.first_operand);
        for tail in &node.tails {
            self.walk_node(&tail.right_parts);
            self.walk_node(&tail.operator);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_mekso_operator(&mut self, node: &'tree generated::MeksoOperatorSyntax) {
        match node {
            generated::MeksoOperatorSyntax::AfterthoughtMeksoOperator(operator) => {
                self.walk_node(&operator.0.first);
                for continuation in &operator.0.links {
                    self.walk_node(&continuation.trailing_operator);
                }
            }
            generated::MeksoOperatorSyntax::BoundMeksoOperator(operator) => {
                self.walk_node(&operator.left_operator);
                self.walk_node(&operator.right_operator);
            }
            generated::MeksoOperatorSyntax::SimpleMeksoOperator(operator) => {
                self.walk_node(operator);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_bound_or_atom_mekso_operator(
        &mut self,
        node: &'tree generated::BoundOrAtomMeksoOperatorSyntax,
    ) {
        match node {
            generated::BoundOrAtomMeksoOperatorSyntax::BoundMeksoOperator(operator) => {
                self.walk_node(&operator.left_operator);
                self.walk_node(&operator.right_operator);
            }
            generated::BoundOrAtomMeksoOperatorSyntax::SimpleMeksoOperator(operator) => {
                self.walk_node(operator);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_simple_mekso_operator(&mut self, node: &'tree generated::SimpleMeksoOperatorSyntax) {
        match node {
            generated::SimpleMeksoOperatorSyntax::ConvertedMeksoOperator(operator) => {
                self.walk_node(&operator.inner_operator);
            }
            generated::SimpleMeksoOperatorSyntax::ScalarNegatedMeksoOperator(operator) => {
                self.walk_node(&operator.inner_operator);
            }
            generated::SimpleMeksoOperatorSyntax::ForethoughtMeksoOperator(operator) => {
                self.walk_node(&operator.left_operator);
                self.walk_node(&operator.right_operator);
            }
            generated::SimpleMeksoOperatorSyntax::GroupedMeksoOperator(operator) => {
                self.walk_node(&operator.inner_operator);
            }
            generated::SimpleMeksoOperatorSyntax::SelbriMeksoOperator(operator) => {
                self.analyze_relation(&operator.selbri);
            }
            generated::SimpleMeksoOperatorSyntax::OperandMeksoOperator(operator) => {
                self.walk_node(&operator.mekso);
            }
            generated::SimpleMeksoOperatorSyntax::ZantufaMahoSelbriMeksoOperator(operator) => {
                self.analyze_relation(&operator.selbri);
            }
            generated::SimpleMeksoOperatorSyntax::ZantufaMahoSumtiMeksoOperator(operator) => {
                self.walk_node(&operator.sumti);
            }
            generated::SimpleMeksoOperatorSyntax::ZantufaConnectiveMeksoOperator(_)
            | generated::SimpleMeksoOperatorSyntax::PrimitiveMeksoOperator(_) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_leading_term_tag_tense_modal(
        &mut self,
        node: &'tree generated::LeadingTermTagTenseModalSyntax,
    ) {
        if let generated::LeadingTermTagTenseModalSyntax::TenseModal(tense_modal) = node {
            self.walk_node(tense_modal);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_tense_modal(&mut self, node: &'tree generated::TenseModalSyntax) {
        self.walk_node(&node.0);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_tense_modal_body(&mut self, node: &'tree generated::TenseModalBodySyntax) {
        match node {
            generated::TenseModalBodySyntax::ConnectedTenseModal(tense_modal) => {
                self.walk_node(&tense_modal.first);
                for continuation in &tense_modal.continuations {
                    self.walk_node(&continuation.tense_modal);
                }
            }
            generated::TenseModalBodySyntax::TenseModalAtom(tense_modal) => {
                self.walk_node(tense_modal);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_tense_modal_atom(&mut self, node: &'tree generated::TenseModalAtomSyntax) {
        if let generated::TenseModalAtomSyntax::FihoTense(fiho) = node {
            self.analyze_relation(&fiho.selbri);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_free_modifier(&mut self, node: &'tree generated::FreeModifierSyntax) {
        match node {
            generated::FreeModifierSyntax::SeiFreeModifier(free_modifier) => {
                for term in &free_modifier.terms {
                    self.walk_node(term);
                }
                self.analyze_relation(&free_modifier.selbri);
            }
            generated::FreeModifierSyntax::ZantufaSeiStatementFreeModifier(free_modifier) => {
                self.walk_node(&free_modifier.statement);
            }
            generated::FreeModifierSyntax::ParentheticalText(free_modifier) => {
                self.walk_node(&free_modifier.text);
            }
            generated::FreeModifierSyntax::XiFreeModifier(free_modifier) => match free_modifier {
                generated::XiFreeModifierSyntax::XiParenthesizedFreeModifier(free_modifier) => {
                    self.walk_node(&free_modifier.expression.inner_expression);
                }
                generated::XiFreeModifierSyntax::XiNumberFreeModifier(_)
                | generated::XiFreeModifierSyntax::XiLerfuStringFreeModifier(_) => {}
            },
            generated::FreeModifierSyntax::ZantufaMeksoMaiFreeModifier(free_modifier) => {
                self.walk_node(&free_modifier.expression);
            }
            generated::FreeModifierSyntax::SoiFreeModifier(free_modifier) => {
                self.walk_node(&free_modifier.leading_sumti);
                if let Some(sumti) = free_modifier.trailing_sumti.as_deref() {
                    self.walk_node(sumti);
                }
            }
            generated::FreeModifierSyntax::VocativeFreeModifier(free_modifier) => {
                if let Some(sumti) = free_modifier.sumti.as_deref() {
                    self.walk_node(sumti);
                }
            }
            generated::FreeModifierSyntax::TextReplacementFreeModifier(_)
            | generated::FreeModifierSyntax::MaiFreeModifier(_) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_vocative_sumti(&mut self, node: &'tree generated::VocativeSumtiSyntax) {
        match node {
            generated::VocativeSumtiSyntax::SelbriVocativeSumti(sumti) => {
                if let Some(clauses) = &sumti.leading_relative_clauses {
                    self.walk_node(clauses);
                }
                self.analyze_relation(&sumti.selbri);
                if let Some(clauses) = &sumti.trailing_relative_clauses {
                    self.walk_node(clauses);
                }
            }
            generated::VocativeSumtiSyntax::CmevlaVocativeSumti(sumti) => {
                if let Some(clauses) = &sumti.leading_relative_clauses {
                    self.walk_node(clauses);
                }
                if let Some(clauses) = &sumti.trailing_relative_clauses {
                    self.walk_node(clauses);
                }
            }
            generated::VocativeSumtiSyntax::Sumti(sumti) => self.walk_node(sumti),
        }
    }

    // Boundary: these paragraph/statement connective nodes are not descended
    // for place analysis. The #219 TreeVisitor spike proved that descending
    // into the `i ... bo` connective `fi'o` selbri for corpus.camxes.157/.159
    // creates frames for connective material instead of only the main
    // statement. The leading-`i` wrapper itself remains traversed because it
    // can carry semantic free modifiers such as vocatives.
    #[requires(true)]
    #[ensures(true)]
    fn walk_trailing_ijek_paragraph_statement(
        &mut self,
        _node: &'tree generated::TrailingIjekParagraphStatementSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_pending_i_connective(&mut self, _node: &'tree generated::PendingIConnectiveSyntax) {}

    #[requires(true)]
    #[ensures(true)]
    fn walk_i_paragraph_statement_connective_i_standard_paragraph_statement_connective(
        &mut self,
        _node: &'tree generated::IStandardParagraphStatementConnectiveSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_i_paragraph_statement_connective_i_tag_bo_paragraph_statement_connective(
        &mut self,
        _node: &'tree generated::ITagBoParagraphStatementConnectiveSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_i_statement_connective_i_standard_statement_connective(
        &mut self,
        _node: &'tree generated::IStandardStatementConnectiveSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_i_statement_connective_i_tag_bo_statement_connective(
        &mut self,
        _node: &'tree generated::ITagBoStatementConnectiveSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_tagged_or_elided_sumti_tagged_elided_sumti(
        &mut self,
        _node: &'tree generated::TaggedElidedSumtiSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_sumti_base_pro_sumti(&mut self, _node: &'tree generated::ProSumtiSyntax) {}

    #[requires(true)]
    #[ensures(true)]
    fn walk_sumti_base_name_sumti(&mut self, node: &'tree generated::NameSumtiSyntax) {
        generated::walk::name_sumti(self, node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_sumti_base_lerfu_string_sumti(
        &mut self,
        _node: &'tree generated::LerfuStringSumtiSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_fragment_statement_ek_fragment(&mut self, _node: &'tree generated::EkFragmentSyntax) {}

    #[requires(true)]
    #[ensures(true)]
    fn walk_fragment_statement_gihek_fragment(
        &mut self,
        _node: &'tree generated::GihekFragmentSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_fragment_statement_multiple_na_fragment(
        &mut self,
        _node: &'tree generated::MultipleNaFragmentSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_fragment_statement_single_na_fragment(
        &mut self,
        _node: &'tree generated::SingleNaFragmentSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_linked_sumti_empty_linked_sumti(
        &mut self,
        _node: &'tree generated::EmptyLinkedSumtiSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_relative_sumti_na_ku_relative_sumti(
        &mut self,
        _node: &'tree generated::NaKuRelativeSumtiSyntax,
    ) {
    }
}

#[derive(Debug, Clone)]
#[invariant(true)]
struct GeneratedBridiTailAnalysis<'tree> {
    frames: Vec<SelbriPlaceFrameId>,
    terms: Vec<&'tree generated::TermSyntax>,
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
            PlaceSlot::PlaceQuestion => {
                self.next_place = self.next_place.saturating_add(1);
            }
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

    #[requires(next_place > 0)]
    #[ensures(true)]
    fn reset_next_place(&mut self, next_place: u8) {
        self.next_place = next_place;
        while self.filled_numbered.contains(&self.next_place) {
            self.next_place = self.next_place.saturating_add(1);
        }
    }

    #[requires(minimum > 0)]
    #[ensures(self.next_place >= minimum)]
    fn ensure_next_place_at_least(&mut self, minimum: u8) {
        self.next_place = self.next_place.max(minimum);
    }
}

#[derive(Debug)]
#[invariant(true)]
pub struct GeneratedSyntaxIndex<'tree> {
    nodes: Vec<GeneratedIndexedSyntaxNode<'tree>>,
    by_ref: HashMap<GeneratedSyntaxNodeRef<'tree>, RawSyntaxNodeId>,
    root: TextNodeId,
}

#[derive(Debug)]
#[invariant(true)]
struct GeneratedIndexedSyntaxNode<'tree> {
    node: GeneratedSyntaxNodeRef<'tree>,
    metadata: SyntaxNodeMetadata,
}

impl<'tree> GeneratedSyntaxIndex<'tree> {
    #[requires(true)]
    #[ensures(ret.as_ref().is_ok_and(|index| !index.nodes.is_empty()) || ret.is_err())]
    pub fn new(root: &'tree GeneratedTextSyntax) -> Result<Self, ReferenceAnalysisError> {
        let mut builder = GeneratedSyntaxIndexBuilder::new();
        root.visit_in_order(&mut builder);
        let root_ref = root
            .as_node_ref()
            .ok_or(ReferenceAnalysisError::MissingRootNode)?;
        let root_raw = builder
            .by_ref
            .get(&root_ref)
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
    pub fn node(&self, id: RawSyntaxNodeId) -> Option<GeneratedSyntaxNodeRef<'tree>> {
        self.nodes.get(id.0).map(|node| node.node)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn metadata(&self, id: RawSyntaxNodeId) -> Option<&SyntaxNodeMetadata> {
        self.nodes.get(id.0).map(|node| &node.metadata)
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn id_of(&self, node: GeneratedSyntaxNodeRef<'tree>) -> Option<RawSyntaxNodeId> {
        self.by_ref.get(&node).copied()
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn text_node_id(&self, node: &'tree GeneratedTextSyntax) -> Option<TextNodeId> {
        node.as_node_ref()
            .and_then(|node| self.id_of(node))
            .map(TextNodeId)
    }

    #[requires(true)]
    #[ensures(true)]
    fn id_for_tree_node<N: GeneratedSyntaxTreeNode>(
        &self,
        node: &'tree N,
    ) -> Option<RawSyntaxNodeId> {
        node.as_node_ref().and_then(|node| self.id_of(node))
    }
}

#[derive(Debug)]
#[invariant(true)]
struct GeneratedSyntaxIndexBuilder<'tree> {
    nodes: Vec<GeneratedIndexedSyntaxNode<'tree>>,
    by_ref: HashMap<GeneratedSyntaxNodeRef<'tree>, RawSyntaxNodeId>,
    stack: Vec<RawSyntaxNodeId>,
    leaf_index: usize,
}

impl<'tree> GeneratedSyntaxIndexBuilder<'tree> {
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
                let mut metadata = node.metadata.clone().into_data();
                if metadata.first_source_span.is_none() {
                    metadata.first_source_span = Some(span.clone());
                }
                metadata.last_source_span = Some(span.clone());
                node.metadata = SyntaxNodeMetadata::from_data(metadata);
            }
        }
        self.leaf_index += 1;
    }
}

impl<'tree> TreeVisitor<'tree> for GeneratedSyntaxIndexBuilder<'tree> {
    type Node = GeneratedSyntaxNodeRef<'tree>;
    type Atom = GeneratedSyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        let id = RawSyntaxNodeId(self.nodes.len());
        let parent = self.stack.last().copied();
        let metadata = new!(SyntaxNodeMetadata {
            id: id,
            parent: parent,
            preorder: id.0,
            depth: self.stack.len(),
            leaf_start: self.leaf_index,
            leaf_end: self.leaf_index,
            first_source_span: None,
            last_source_span: None,
        });
        self.nodes
            .push(GeneratedIndexedSyntaxNode { node, metadata });
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
        self.nodes[id.0].metadata = self.nodes[id.0].metadata.clone().with_data(data! {
            leaf_end: self.leaf_index,
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        match atom {
            GeneratedSyntaxAtomRef::Token(token) => {
                for span in token.source_spans() {
                    self.record_source_span(span);
                }
            }
        }
    }
}

#[derive(Debug)]
#[invariant(true)]
struct SumtiMention {
    source: SumtiNodeId,
    target: SumtiNodeId,
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

#[requires(true)]
#[ensures(ret.0 == mention.position)]
fn sumti_mention_sort_key(mention: &SumtiMention) -> (usize, usize) {
    (mention.position, mention.source.0.0)
}

#[requires(true)]
#[ensures(ret.0 == mention.position)]
fn node_mention_sort_key(mention: &NodeMention) -> (usize, usize) {
    (mention.position, mention.source.0)
}

#[requires(true)]
#[ensures(mentions.len() == old(mentions.len()) + 1)]
fn insert_sumti_mention_sorted(mentions: &mut Vec<SumtiMention>, mention: SumtiMention) {
    let key = sumti_mention_sort_key(&mention);
    if mentions
        .last()
        .is_none_or(|existing| sumti_mention_sort_key(existing) <= key)
    {
        mentions.push(mention);
        return;
    }
    let index = mentions.partition_point(|existing| sumti_mention_sort_key(existing) <= key);
    mentions.insert(index, mention);
}

#[requires(true)]
#[ensures(mentions.len() == old(mentions.len()) + 1)]
fn insert_node_mention_sorted(mentions: &mut Vec<NodeMention>, mention: NodeMention) {
    let key = node_mention_sort_key(&mention);
    if mentions
        .last()
        .is_none_or(|existing| node_mention_sort_key(existing) <= key)
    {
        mentions.push(mention);
        return;
    }
    let index = mentions.partition_point(|existing| node_mention_sort_key(existing) <= key);
    mentions.insert(index, mention);
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum CeiLabel {
    Broda,
    Brode,
    Brodi,
    Brodo,
    Brodu,
    Buha,
    Buhe,
    Buhi,
}

impl CeiLabel {
    #[requires(true)]
    #[ensures(true)]
    fn from_broda_word_like(word_like: &WordLike) -> Option<Self> {
        let word = word_like.bare_word()?;
        match word.canonical_phonemes().as_str() {
            "broda" => Some(Self::Broda),
            "brode" => Some(Self::Brode),
            "brodi" => Some(Self::Brodi),
            "brodo" => Some(Self::Brodo),
            "brodu" => Some(Self::Brodu),
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn from_buha_cmavo(cmavo: Cmavo) -> Option<Self> {
        match cmavo {
            Cmavo::Buha => Some(Self::Buha),
            Cmavo::Buhe => Some(Self::Buhe),
            Cmavo::Buhi => Some(Self::Buhi),
            _ => None,
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CeiAssignmentSource {
    label: CeiLabel,
    node: RawSyntaxNodeId,
}

#[derive(Debug)]
#[invariant(true)]
struct GeneratedPrenexCeiAssignmentSourceCollector<'index, 'tree> {
    index: &'index GeneratedSyntaxIndex<'tree>,
    skip_depth: usize,
    sources: Vec<CeiAssignmentSource>,
}

impl<'index, 'tree> GeneratedPrenexCeiAssignmentSourceCollector<'index, 'tree> {
    #[requires(true)]
    #[ensures(ret.skip_depth == 0)]
    #[ensures(ret.sources.is_empty())]
    fn new(index: &'index GeneratedSyntaxIndex<'tree>) -> Self {
        Self {
            index,
            skip_depth: 0,
            sources: Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn into_sources(self) -> Vec<CeiAssignmentSource> {
        self.sources
    }

    #[requires(true)]
    #[ensures(true)]
    fn raw_for_node<N: GeneratedSyntaxTreeNode>(&self, node: &'tree N) -> RawSyntaxNodeId {
        self.index.id_for_tree_node(node).unwrap_or_else(|| {
            panic!(
                "generated syntax node belongs to indexed syntax tree: {:?}",
                node.as_node_ref().map(|node| node.constructor_name())
            )
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn record_assignment(&mut self, unit: &'tree generated::LinkedTanruUnitForCeiSyntax) {
        if let Some(label) = generated_relation_unit_assignment_label(unit) {
            let node = self.raw_for_node(unit);
            self.sources.push(CeiAssignmentSource { label, node });
        }
    }
}

impl<'index, 'tree> TreeVisitor<'tree>
    for GeneratedPrenexCeiAssignmentSourceCollector<'index, 'tree>
{
    type Node = GeneratedSyntaxNodeRef<'tree>;
    type Atom = GeneratedSyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        if self.skip_depth > 0 {
            self.skip_depth += 1;
            return;
        }
        if generated_prenex_binding_should_skip_node(node) {
            self.skip_depth = 1;
            return;
        }
        if let GeneratedSyntaxNodeRef::AssignedProBridiTanruUnitSyntax(unit) = node {
            for assignment in &unit.assignments {
                self.record_assignment(assignment.tanru_unit.as_ref());
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        if self.skip_depth > 0 {
            self.skip_depth -= 1;
        }
    }
}

#[derive(Debug)]
#[invariant(true)]
struct GeneratedPrenexRelationVariableBindingCollector<'index, 'tree> {
    index: &'index GeneratedSyntaxIndex<'tree>,
    skip_depth: usize,
    bindings: Vec<(Cmavo, SelbriNodeId)>,
}

impl<'index, 'tree> GeneratedPrenexRelationVariableBindingCollector<'index, 'tree> {
    #[requires(true)]
    #[ensures(ret.skip_depth == 0)]
    #[ensures(ret.bindings.is_empty())]
    fn new(index: &'index GeneratedSyntaxIndex<'tree>) -> Self {
        Self {
            index,
            skip_depth: 0,
            bindings: Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn into_bindings(self) -> Vec<(Cmavo, SelbriNodeId)> {
        self.bindings
    }

    #[requires(true)]
    #[ensures(true)]
    fn raw_for_node<N: GeneratedSyntaxTreeNode>(&self, node: &'tree N) -> RawSyntaxNodeId {
        self.index.id_for_tree_node(node).unwrap_or_else(|| {
            panic!(
                "generated syntax node belongs to indexed syntax tree: {:?}",
                node.as_node_ref().map(|node| node.constructor_name())
            )
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn bind_relation(&mut self, selbri: &'tree generated::SelbriSyntax) {
        if let Some(cmavo @ (Cmavo::Buha | Cmavo::Buhe | Cmavo::Buhi)) =
            generated_relation_pro_bridi_cmavo(selbri)
        {
            let target = SelbriNodeId(self.raw_for_node(selbri));
            self.bindings.push((cmavo, target));
        }
    }
}

impl<'index, 'tree> TreeVisitor<'tree>
    for GeneratedPrenexRelationVariableBindingCollector<'index, 'tree>
{
    type Node = GeneratedSyntaxNodeRef<'tree>;
    type Atom = GeneratedSyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        if self.skip_depth > 0 {
            self.skip_depth += 1;
            return;
        }
        if generated_prenex_binding_should_skip_node(node) {
            self.skip_depth = 1;
            return;
        }
        match node {
            GeneratedSyntaxNodeRef::NoihaVariableAdverbialTermSyntax(term) => {
                self.bind_relation(&term.selbri);
            }
            GeneratedSyntaxNodeRef::NoihaRelativeAdverbialTermSyntax(term) => {
                self.bind_relation(&term.selbri);
            }
            GeneratedSyntaxNodeRef::DescriptorWithoutGadriSumtiSyntax(description) => {
                self.bind_relation(&description.selbri);
            }
            GeneratedSyntaxNodeRef::RelationDescriptionTailSyntax(tail) => {
                self.bind_relation(&tail.selbri);
            }
            GeneratedSyntaxNodeRef::QuantifierRelationDescriptionTailSyntax(tail) => {
                self.bind_relation(&tail.selbri);
            }
            GeneratedSyntaxNodeRef::SelbriFragmentSyntax(fragment) => {
                self.bind_relation(&fragment.0);
            }
            GeneratedSyntaxNodeRef::SelbriSimpleBridiTailSyntax(tail) => {
                self.bind_relation(&tail.selbri);
            }
            GeneratedSyntaxNodeRef::SelbriSimpleBridiTailWithoutTailTermsSyntax(tail) => {
                self.bind_relation(&tail.selbri);
            }
            _ => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        if self.skip_depth > 0 {
            self.skip_depth -= 1;
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_prenex_binding_should_skip_node(node: GeneratedSyntaxNodeRef<'_>) -> bool {
    matches!(
        node,
        GeneratedSyntaxNodeRef::SimpleTermSyntaxFihoiAdverbialTerm(_)
            | GeneratedSyntaxNodeRef::SimpleTermSyntaxSoiAdverbialTerm(_)
            | GeneratedSyntaxNodeRef::SimpleTermSyntaxTaggedSumtiBeforeTagTerm(_)
            | GeneratedSyntaxNodeRef::SimpleTermSyntaxNaKuTerm(_)
            | GeneratedSyntaxNodeRef::SimpleTermSyntaxBareNaTerm(_)
            | GeneratedSyntaxNodeRef::TaggedOrElidedSumtiSyntaxTaggedElidedSumti(_)
            | GeneratedSyntaxNodeRef::SumtiBaseSyntaxNumberSumti(_)
            | GeneratedSyntaxNodeRef::SumtiBaseSyntaxLerfuStringSumti(_)
            | GeneratedSyntaxNodeRef::SumtiBaseSyntaxQuotedSumti(_)
            | GeneratedSyntaxNodeRef::SumtiBaseSyntaxProSumti(_)
            | GeneratedSyntaxNodeRef::FragmentStatementSyntaxEkFragment(_)
            | GeneratedSyntaxNodeRef::FragmentStatementSyntaxGihekFragment(_)
            | GeneratedSyntaxNodeRef::FragmentStatementSyntaxMeksoFragment(_)
            | GeneratedSyntaxNodeRef::FragmentStatementSyntaxZantufaMeksoFragment(_)
            | GeneratedSyntaxNodeRef::FragmentStatementSyntaxMultipleNaFragment(_)
            | GeneratedSyntaxNodeRef::FragmentStatementSyntaxSingleNaFragment(_)
            | GeneratedSyntaxNodeRef::LinkedSumtiSyntaxEmptyLinkedSumti(_)
            | GeneratedSyntaxNodeRef::RelativeSumtiSyntaxNaKuRelativeSumti(_)
            | GeneratedSyntaxNodeRef::SimpleBridiTailSyntaxForethoughtSimpleBridiTail(_)
            | GeneratedSyntaxNodeRef::SimpleBridiTailWithoutTailTermsSyntaxForethoughtSimpleBridiTailWithoutTailTerms(_)
            | GeneratedSyntaxNodeRef::FreeModifierSyntaxTextReplacementFreeModifier(_)
            | GeneratedSyntaxNodeRef::FreeModifierSyntaxZantufaSeiStatementFreeModifier(_)
            | GeneratedSyntaxNodeRef::FreeModifierSyntaxSeiFreeModifier(_)
            | GeneratedSyntaxNodeRef::FreeModifierSyntaxXiFreeModifier(_)
            | GeneratedSyntaxNodeRef::FreeModifierSyntaxMaiFreeModifier(_)
            | GeneratedSyntaxNodeRef::FreeModifierSyntaxZantufaMeksoMaiFreeModifier(_)
            | GeneratedSyntaxNodeRef::FreeModifierSyntaxSoiFreeModifier(_)
            | GeneratedSyntaxNodeRef::FreeModifierSyntaxParentheticalText(_)
            | GeneratedSyntaxNodeRef::FreeModifierSyntaxVocativeFreeModifier(_)
    )
}

// Boundary inventory for the prenex-binding flat scan.
//
// Prenex binding deliberately uses `TreeVisitor` instead of the recursive
// walker because it is an in-order collection pass over a bounded term slice,
// not a stateful grammar-directed traversal. These skips keep that flat scan
// aligned with the semantic boundaries enforced by the recursive walkers.
//
// Provenance:
// - Statement-connective material is a deliberate scope boundary. The failed
//   #219 spike proved that descending into the `i ... bo` connective `fi'o`
//   selbri for `corpus.camxes.157`/`.159` creates place frames for connective
//   material instead of only the main statement. The leading-`i` wrapper itself
//   is not skipped because it can carry semantic free modifiers such as
//   vocatives; only its connective child is a boundary.
// - Empty/elided, NA, fragment-connective, quote, and replacement nodes are
//   wrapper or token-only leaves for these semantic reference families.
// - Free modifiers are handled only through explicit nested entry points so
//   quote/utterance and prenex scopes remain under caller control.
#[derive(Debug)]
#[invariant(true)]
struct GeneratedDiscourseReferenceBuilder<'index, 'tree> {
    index: &'index GeneratedSyntaxIndex<'tree>,
    places: &'index PlaceAnalysis,
    edges: Vec<ReferenceEdge>,
    koha_bindings: HashMap<Cmavo, SumtiNodeId>,
    cei_bridi_bindings: HashMap<CeiLabel, BridiNodeId>,
    selbri_variable_bindings: HashMap<Cmavo, SelbriNodeId>,
    da_bindings: HashMap<Cmavo, SumtiNodeId>,
    sumti_mentions: Vec<SumtiMention>,
    letter_sumti_mentions: HashMap<String, Vec<SumtiMention>>,
    predicate_mentions: Vec<NodeMention>,
    quote_sumti_mentions: Vec<SumtiMention>,
    quote_letter_sumti_mentions: HashMap<String, Vec<SumtiMention>>,
    quote_predicate_mentions: Vec<NodeMention>,
    quote_utterance_history: Vec<RawSyntaxNodeId>,
    quote_current_utterance: Option<RawSyntaxNodeId>,
    quote_pending_next_utterance_sources: Vec<RawSyntaxNodeId>,
    quote_depth: usize,
    last_bridi: Option<BridiNodeId>,
    current_bridi: Option<BridiNodeId>,
    predicate_stack: Vec<RawSyntaxNodeId>,
    discourse_predicate_stack: Vec<RawSyntaxNodeId>,
    abstraction_stack: Vec<RawSyntaxNodeId>,
    utterance_history: Vec<RawSyntaxNodeId>,
    current_utterance: Option<RawSyntaxNodeId>,
    pending_next_utterance_sources: Vec<RawSyntaxNodeId>,
    current_bridi_frames: Vec<SelbriPlaceFrameId>,
    relative_heads: Vec<SumtiNodeId>,
}

impl<'index, 'tree> GeneratedDiscourseReferenceBuilder<'index, 'tree> {
    #[requires(true)]
    #[ensures(ret.edges.is_empty())]
    fn new(index: &'index GeneratedSyntaxIndex<'tree>, places: &'index PlaceAnalysis) -> Self {
        Self {
            index,
            places,
            edges: Vec::new(),
            koha_bindings: HashMap::new(),
            cei_bridi_bindings: HashMap::new(),
            selbri_variable_bindings: HashMap::new(),
            da_bindings: HashMap::new(),
            sumti_mentions: Vec::new(),
            letter_sumti_mentions: HashMap::new(),
            predicate_mentions: Vec::new(),
            quote_sumti_mentions: Vec::new(),
            quote_letter_sumti_mentions: HashMap::new(),
            quote_predicate_mentions: Vec::new(),
            quote_utterance_history: Vec::new(),
            quote_current_utterance: None,
            quote_pending_next_utterance_sources: Vec::new(),
            quote_depth: 0,
            last_bridi: None,
            current_bridi: None,
            predicate_stack: Vec::new(),
            discourse_predicate_stack: Vec::new(),
            abstraction_stack: Vec::new(),
            utterance_history: Vec::new(),
            current_utterance: None,
            pending_next_utterance_sources: Vec::new(),
            current_bridi_frames: Vec::new(),
            relative_heads: Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(mut self) -> DiscourseReferences {
        self.flush_unresolved_pending_next_utterance_sources();
        DiscourseReferences { edges: self.edges }
    }

    #[requires(true)]
    #[ensures(self.pending_next_utterance_sources.is_empty())]
    fn flush_unresolved_pending_next_utterance_sources(&mut self) {
        for source in std::mem::take(&mut self.pending_next_utterance_sources) {
            self.add_edge(
                ReferenceKind::Utterance,
                source,
                target_unresolved("di'e has no following utterance"),
                ReferenceRule::DiheFollowingWhenPresent,
            );
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_statement(&mut self, statement: &'tree generated::StatementSyntax) {
        let statement_id = StatementNodeId(self.raw_for_node(statement));
        for source in std::mem::take(&mut self.pending_next_utterance_sources) {
            self.add_edge(
                ReferenceKind::Utterance,
                source,
                target_resolved_node(statement_id.0),
                ReferenceRule::DiheFollowing,
            );
        }
        let previous_utterance = self.current_utterance.replace(statement_id.0);
        match statement {
            generated::StatementSyntax::StatementBase(statement) => {
                self.visit_statement_base(statement);
            }
            generated::StatementSyntax::IStatementConnection(connection) => {
                self.visit_statement_base(&connection.leading_statement);
                for continuation in &connection.continuations {
                    self.walk_node(continuation);
                }
            }
            generated::StatementSyntax::PreposedIStatementConnection(connection) => {
                self.visit_statement_base(&connection.leading_statement);
                self.walk_node(&connection.trailing_statement);
            }
        }
        self.current_utterance = previous_utterance;
        self.utterance_history.push(statement_id.0);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_statement_base(&mut self, statement: &'tree generated::StatementBaseSyntax) {
        match statement {
            generated::StatementBaseSyntax::PrenexStatement(statement) => {
                let previous_da_bindings = self.da_bindings.clone();
                for term in &statement.prenex_terms {
                    self.walk_node(term);
                }
                let previous_selbri_variable_bindings = self.selbri_variable_bindings.clone();
                self.bind_prenex_relation_variables(&statement.prenex_terms);
                let previous_cei_bridi_bindings = self.cei_bridi_bindings.clone();
                self.bind_prenex_cei_predicate_targets_for_statement(
                    &statement.prenex_terms,
                    &statement.inner_statement,
                );
                self.visit_statement(&statement.inner_statement);
                self.cei_bridi_bindings = previous_cei_bridi_bindings;
                self.selbri_variable_bindings = previous_selbri_variable_bindings;
                self.da_bindings = previous_da_bindings;
            }
            generated::StatementBaseSyntax::BridiStatement(statement) => {
                self.walk_node(statement);
            }
            generated::StatementBaseSyntax::TextGroupStatement(statement) => {
                self.walk_node(statement);
            }
            generated::StatementBaseSyntax::ForethoughtStatement(statement) => {
                self.walk_node(statement);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_subbridi(&mut self, subbridi: &'tree generated::SubbridiSyntax) {
        match subbridi {
            generated::SubbridiSyntax::BridiSubbridi(subbridi) => self.visit_predicate(&subbridi.0),
            generated::SubbridiSyntax::PrenexSubbridi(subbridi) => {
                let previous_da_bindings = self.da_bindings.clone();
                for term in &subbridi.prenex_terms {
                    self.walk_node(term);
                }
                let previous_selbri_variable_bindings = self.selbri_variable_bindings.clone();
                self.bind_prenex_relation_variables(&subbridi.prenex_terms);
                let previous_cei_bridi_bindings = self.cei_bridi_bindings.clone();
                self.bind_prenex_cei_predicate_targets_for_subbridi(
                    &subbridi.prenex_terms,
                    &subbridi.inner_subbridi,
                );
                self.visit_subbridi(&subbridi.inner_subbridi);
                self.cei_bridi_bindings = previous_cei_bridi_bindings;
                self.selbri_variable_bindings = previous_selbri_variable_bindings;
                self.da_bindings = previous_da_bindings;
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_predicate(&mut self, bridi: &'tree generated::BridiSyntax) {
        let predicate_id = BridiNodeId(self.raw_for_node(bridi));
        let frames = self.places.frames_for_node(predicate_id.0).to_vec();
        let previous_frames = std::mem::replace(&mut self.current_bridi_frames, frames);
        let previous_predicate = self.current_bridi.replace(predicate_id);
        let was_top_predicate = self.predicate_stack.is_empty();
        let is_in_abstraction = !self.abstraction_stack.is_empty();
        self.predicate_stack.push(predicate_id.0);
        if !is_in_abstraction {
            self.discourse_predicate_stack.push(predicate_id.0);
        }
        match bridi {
            generated::BridiSyntax::BridiWithLeadingTerms(bridi) => {
                for term in &bridi.leading_terms {
                    self.walk_node(term);
                }
                self.visit_bridi_tail(&bridi.bridi_tail);
            }
            generated::BridiSyntax::BridiWithPostCuTerms(bridi) => {
                for term in &bridi.leading_terms {
                    self.walk_node(term);
                }
                self.visit_cu_terms_bridi_tail(&bridi.bridi_tail);
            }
            generated::BridiSyntax::BareCuBridi(bridi) => {
                self.visit_bridi_tail(&bridi.bridi_tail);
            }
            generated::BridiSyntax::BareCuTermsBridi(bridi) => {
                self.visit_cu_terms_bridi_tail(&bridi.bridi_tail);
            }
            generated::BridiSyntax::RelationOnlyBridi(bridi) => {
                self.visit_bridi_tail(&bridi.0);
            }
        }
        if !is_in_abstraction {
            self.discourse_predicate_stack.pop();
        }
        self.predicate_stack.pop();
        self.current_bridi_frames = previous_frames;
        self.current_bridi = previous_predicate;
        self.last_bridi = Some(predicate_id);
        if was_top_predicate {
            self.note_predicate_mention(predicate_id.0);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_cu_terms_bridi_tail(&mut self, tail: &'tree generated::CuTermsBridiTailSyntax) {
        for term in &tail.terms {
            self.walk_node(term);
        }
        self.visit_bridi_tail(&tail.bridi_tail);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_bridi_tail(&mut self, tail: &'tree generated::BridiTailSyntax) {
        match tail {
            generated::BridiTailSyntax::ZantufaGroupedBridiTail(tail) => {
                self.visit_bridi_tail(&tail.bridi_tail);
                for term in &tail.tail_terms {
                    self.walk_node(term);
                }
            }
            generated::BridiTailSyntax::BridiTailWithPossibleTailTerms(tail) => {
                self.visit_afterthought_bridi_tail(&tail.first);
                if let Some(continuation) = tail.ke_continuation.as_deref() {
                    if let Some(tense_modal) = continuation.tense_modal.as_deref() {
                        self.walk_node(tense_modal);
                    }
                    self.visit_bridi_tail(&continuation.bridi_tail);
                    for term in &continuation.tail_terms {
                        self.walk_node(term);
                    }
                }
            }
            generated::BridiTailSyntax::BridiTailWithoutTailTerms(tail) => {
                self.visit_afterthought_bridi_tail_without_tail_terms(&tail.first);
                if let Some(continuation) = tail.ke_continuation.as_deref() {
                    if let Some(tense_modal) = continuation.tense_modal.as_deref() {
                        self.walk_node(tense_modal);
                    }
                    self.visit_bridi_tail(&continuation.bridi_tail);
                    for term in &continuation.tail_terms {
                        self.walk_node(term);
                    }
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_afterthought_bridi_tail(
        &mut self,
        tail: &'tree generated::AfterthoughtBridiTailSyntax,
    ) {
        self.visit_bo_grouped_bridi_tail(&tail.0.first);
        for continuation in &tail.0.links {
            self.visit_bo_grouped_bridi_tail(&continuation.bridi_tail);
            for term in &continuation.tail_terms {
                self.walk_node(term);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_afterthought_bridi_tail_without_tail_terms(
        &mut self,
        tail: &'tree generated::AfterthoughtBridiTailWithoutTailTermsSyntax,
    ) {
        self.visit_bo_grouped_bridi_tail_without_tail_terms(&tail.0.first);
        for continuation in &tail.0.links {
            self.visit_bo_grouped_bridi_tail_without_tail_terms(&continuation.bridi_tail);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_bo_grouped_bridi_tail(&mut self, tail: &'tree generated::BoGroupedBridiTailSyntax) {
        self.visit_simple_bridi_tail(&tail.first);
        if let Some(continuation) = tail.bo_continuation.as_deref() {
            if let Some(tense_modal) = continuation.tense_modal.as_deref() {
                self.walk_node(tense_modal);
            }
            self.visit_bo_grouped_bridi_tail(&continuation.bridi_tail);
            for term in &continuation.tail_terms {
                self.walk_node(term);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_bo_grouped_bridi_tail_without_tail_terms(
        &mut self,
        tail: &'tree generated::BoGroupedBridiTailWithoutTailTermsSyntax,
    ) {
        self.visit_simple_bridi_tail_without_tail_terms(&tail.first);
        if let Some(continuation) = tail.bo_continuation.as_deref() {
            if let Some(tense_modal) = continuation.tense_modal.as_deref() {
                self.walk_node(tense_modal);
            }
            self.visit_bo_grouped_bridi_tail_without_tail_terms(&continuation.bridi_tail);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_simple_bridi_tail(&mut self, tail: &'tree generated::SimpleBridiTailSyntax) {
        match tail {
            generated::SimpleBridiTailSyntax::SelbriSimpleBridiTail(tail) => {
                self.visit_relation(&tail.selbri);
                for term in &tail.terms {
                    self.walk_node(term);
                }
            }
            generated::SimpleBridiTailSyntax::ForethoughtSimpleBridiTail(tail) => {
                self.visit_forethought_bridi_connection(&tail.0);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_simple_bridi_tail_without_tail_terms(
        &mut self,
        tail: &'tree generated::SimpleBridiTailWithoutTailTermsSyntax,
    ) {
        match tail {
            generated::SimpleBridiTailWithoutTailTermsSyntax::SelbriSimpleBridiTailWithoutTailTerms(tail) => {
                self.visit_relation(&tail.selbri);
            }
            generated::SimpleBridiTailWithoutTailTermsSyntax::ForethoughtSimpleBridiTailWithoutTailTerms(tail) => {
                self.visit_forethought_bridi_connection_without_tail_terms(&tail.0);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_forethought_bridi_connection(
        &mut self,
        connection: &'tree generated::ForethoughtBridiConnectionSyntax,
    ) {
        match connection {
            generated::ForethoughtBridiConnectionSyntax::DirectForethoughtBridiConnection(
                connection,
            ) => {
                self.visit_subbridi(&connection.first);
                self.visit_subbridi(&connection.first_branch.branch);
                for branch in &connection.additional_branches {
                    self.visit_subbridi(&branch.branch);
                }
                for term in &connection.tail_terms {
                    self.walk_node(term);
                }
            }
            generated::ForethoughtBridiConnectionSyntax::GroupedForethoughtBridiConnection(
                connection,
            ) => {
                if let Some(tense_modal) = connection.tense_modal.as_deref() {
                    self.walk_node(tense_modal);
                }
                self.visit_forethought_bridi_connection(&connection.inner);
            }
            generated::ForethoughtBridiConnectionSyntax::NegatedForethoughtBridiConnection(
                connection,
            ) => {
                self.visit_forethought_bridi_connection(&connection.inner);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_forethought_bridi_connection_without_tail_terms(
        &mut self,
        connection: &'tree generated::ForethoughtBridiConnectionWithoutTailTermsSyntax,
    ) {
        match connection {
            generated::ForethoughtBridiConnectionWithoutTailTermsSyntax::DirectForethoughtBridiConnectionWithoutTailTerms(
                connection,
            ) => {
                self.visit_subbridi(&connection.first);
                self.visit_subbridi(&connection.first_branch.branch);
                for branch in &connection.additional_branches {
                    self.visit_subbridi(&branch.branch);
                }
            }
            generated::ForethoughtBridiConnectionWithoutTailTermsSyntax::GroupedForethoughtBridiConnectionWithoutTailTerms(
                connection,
            ) => {
                if let Some(tense_modal) = connection.tense_modal.as_deref() {
                    self.walk_node(tense_modal);
                }
                self.visit_forethought_bridi_connection_without_tail_terms(&connection.inner);
            }
            generated::ForethoughtBridiConnectionWithoutTailTermsSyntax::NegatedForethoughtBridiConnectionWithoutTailTerms(
                connection,
            ) => {
                self.visit_forethought_bridi_connection_without_tail_terms(&connection.inner);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn bind_prenex_relation_variables(&mut self, terms: &'tree [generated::TermSyntax]) {
        let mut collector = GeneratedPrenexRelationVariableBindingCollector::new(self.index);
        for term in terms {
            term.visit_in_order(&mut collector);
        }
        for (cmavo, target) in collector.into_bindings() {
            self.selbri_variable_bindings.insert(cmavo, target);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn bind_prenex_cei_predicate_targets_for_statement(
        &mut self,
        terms: &'tree [generated::TermSyntax],
        statement: &'tree generated::StatementSyntax,
    ) {
        if let Some(bridi) = self.statement_main_predicate_id(statement) {
            self.bind_prenex_cei_predicate_targets(terms, bridi);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn bind_prenex_cei_predicate_targets_for_subbridi(
        &mut self,
        terms: &'tree [generated::TermSyntax],
        subbridi: &'tree generated::SubbridiSyntax,
    ) {
        if let Some(bridi) = self.subbridi_main_predicate_id(subbridi) {
            self.bind_prenex_cei_predicate_targets(terms, bridi);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn bind_prenex_cei_predicate_targets(
        &mut self,
        terms: &'tree [generated::TermSyntax],
        bridi: BridiNodeId,
    ) {
        for source in self.prenex_cei_assignment_sources(terms) {
            self.cei_bridi_bindings.insert(source.label, bridi);
            self.add_edge(
                ReferenceKind::ProBridiAssignment,
                source.node,
                target_resolved_node(bridi.0),
                ReferenceRule::PrenexCeiAssignment,
            );
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn statement_main_predicate_id(
        &self,
        statement: &'tree generated::StatementSyntax,
    ) -> Option<BridiNodeId> {
        match statement {
            generated::StatementSyntax::StatementBase(statement) => {
                self.statement_base_main_predicate_id(statement)
            }
            _ => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn statement_base_main_predicate_id(
        &self,
        statement: &'tree generated::StatementBaseSyntax,
    ) -> Option<BridiNodeId> {
        match statement {
            generated::StatementBaseSyntax::BridiStatement(statement) => {
                Some(BridiNodeId(self.raw_for_node(&statement.bridi)))
            }
            generated::StatementBaseSyntax::PrenexStatement(statement) => {
                self.statement_main_predicate_id(&statement.inner_statement)
            }
            generated::StatementBaseSyntax::TextGroupStatement(_)
            | generated::StatementBaseSyntax::ForethoughtStatement(_) => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn subbridi_main_predicate_id(
        &self,
        subbridi: &'tree generated::SubbridiSyntax,
    ) -> Option<BridiNodeId> {
        match subbridi {
            generated::SubbridiSyntax::BridiSubbridi(subbridi) => {
                Some(BridiNodeId(self.raw_for_node(&subbridi.0)))
            }
            generated::SubbridiSyntax::PrenexSubbridi(subbridi) => {
                self.subbridi_main_predicate_id(&subbridi.inner_subbridi)
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn prenex_cei_assignment_sources(
        &self,
        terms: &'tree [generated::TermSyntax],
    ) -> Vec<CeiAssignmentSource> {
        let mut collector = GeneratedPrenexCeiAssignmentSourceCollector::new(self.index);
        for term in terms {
            term.visit_in_order(&mut collector);
        }
        collector.into_sources()
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_argument(&mut self, sumti: &'tree generated::SumtiSyntax) {
        let argument_id = SumtiNodeId(self.raw_for_node(sumti));
        let handled_mention = self.visit_sumti_grouped(argument_id, &sumti.base_sumti);
        if let Some(attachment) = &sumti.vuho_attachment {
            match attachment {
                generated::VuhoSumtiAttachmentTailSyntax::VuhoRelativeSumtiAttachmentTail(
                    attachment,
                ) => {
                    self.visit_relative_clause_list(
                        argument_id,
                        argument_id,
                        &attachment.relative_clauses,
                    );
                    if let Some(connection) = attachment.sumti_connection.as_deref() {
                        self.visit_argument(&connection.sumti);
                    }
                }
                generated::VuhoSumtiAttachmentTailSyntax::VuhoConnectedSumtiAttachmentTail(
                    attachment,
                ) => self.visit_argument(&attachment.sumti_connection.sumti),
            }
        }
        if !handled_mention {
            self.note_self_sumti_mention_with_availability(
                argument_id,
                !generated_argument_wraps_ri(sumti),
            );
        }
        self.note_letter_sumti_antecedent(argument_id, sumti);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_sumti_grouped(
        &mut self,
        argument_id: SumtiNodeId,
        sumti: &'tree generated::SumtiGroupedSyntax,
    ) -> bool {
        let handled = self.visit_sumti_afterthought(argument_id, &sumti.leading_sumti);
        if let Some(tail) = sumti.grouped_tail.as_ref() {
            if let Some(tense_modal) = tail.tense_modal.as_deref() {
                self.walk_node(tense_modal);
            }
            self.visit_argument(&tail.inner_sumti);
            return false;
        }
        handled
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_sumti_afterthought(
        &mut self,
        argument_id: SumtiNodeId,
        sumti: &'tree generated::SumtiAfterthoughtSyntax,
    ) -> bool {
        let handled = self.visit_sumti_bound(argument_id, &sumti.leading_sumti);
        if sumti.continuations.is_empty() {
            return handled;
        }
        for continuation in &sumti.continuations {
            self.visit_sumti_bound(argument_id, &continuation.sumti);
        }
        false
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_sumti_bound(
        &mut self,
        argument_id: SumtiNodeId,
        sumti: &'tree generated::SumtiBoundSyntax,
    ) -> bool {
        let handled = self.visit_sumti_forethought(argument_id, &sumti.leading_sumti);
        if let Some(tail) = sumti.bound_tail.as_ref() {
            if let Some(tense_modal) = tail.tense_modal.as_deref() {
                self.walk_node(tense_modal);
            }
            self.visit_sumti_bound(argument_id, &tail.trailing_sumti);
            return false;
        }
        handled
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_sumti_forethought(
        &mut self,
        argument_id: SumtiNodeId,
        sumti: &'tree generated::SumtiForethoughtSyntax,
    ) -> bool {
        match sumti {
            generated::SumtiForethoughtSyntax::ForethoughtSumti(sumti) => {
                self.visit_argument(&sumti.leading_sumti);
                self.visit_sumti_forethought(argument_id, &sumti.first_branch.sumti);
                for branch in &sumti.additional_branches {
                    self.visit_sumti_forethought(argument_id, &branch.sumti);
                }
                false
            }
            generated::SumtiForethoughtSyntax::SimpleSumti(sumti) => {
                self.visit_simple_sumti(argument_id, sumti)
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_simple_sumti(
        &mut self,
        argument_id: SumtiNodeId,
        sumti: &'tree generated::SimpleSumtiSyntax,
    ) -> bool {
        let handled = self.visit_sumti_atom(argument_id, &sumti.base_sumti);
        if let Some(clauses) = &sumti.relative_clauses {
            self.record_wrapped_sumti_atom_koha_reference(argument_id, &sumti.base_sumti);
            self.visit_relative_clause_list(argument_id, argument_id, clauses);
            return false;
        }
        handled
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_sumti_atom(
        &mut self,
        argument_id: SumtiNodeId,
        sumti: &'tree generated::SumtiAtomSyntax,
    ) -> bool {
        match sumti {
            generated::SumtiAtomSyntax::SumtiBase(sumti) => {
                self.visit_sumti_base(argument_id, sumti)
            }
            generated::SumtiAtomSyntax::QuantifiedSumti(sumti) => {
                self.walk_node(&sumti.quantifier);
                let inner_id = SumtiNodeId(self.raw_for_node(sumti.inner_sumti.as_ref()));
                self.visit_sumti_base(inner_id, &sumti.inner_sumti);
                false
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_sumti_base(
        &mut self,
        argument_id: SumtiNodeId,
        sumti: &'tree generated::SumtiBaseSyntax,
    ) -> bool {
        match sumti {
            generated::SumtiBaseSyntax::ProSumti(koha) => {
                let cmavo = koha.0.value.cmavo();
                let resolved_target = self.resolve_koha(
                    argument_id,
                    cmavo,
                    generated_koha_subscript_index(&koha.0.free_modifiers),
                );
                for free_modifier in &koha.0.free_modifiers {
                    self.walk_node(free_modifier);
                }
                if let Some(target) = resolved_target {
                    self.note_sumti_mention_with_availability(
                        argument_id,
                        target,
                        cmavo.is_some_and(koha_mention_available_to_ri),
                    );
                } else if cmavo.is_some_and(koha_records_self_mention) {
                    self.note_self_sumti_mention_with_availability(
                        argument_id,
                        cmavo.is_some_and(koha_mention_available_to_ri),
                    );
                }
                true
            }
            generated::SumtiBaseSyntax::LerfuStringSumti(sumti) => {
                if let Some(initials) = generated_letter_string_initial_key(&sumti.words) {
                    if let Some(target) = self.resolve_letter_target(&initials) {
                        self.add_edge(
                            ReferenceKind::Letter,
                            argument_id.0,
                            target_resolved_node(target.0),
                            ReferenceRule::LetteralProSumtiLatestInitial,
                        );
                        self.note_sumti_mention_with_availability(argument_id, target, false);
                    } else {
                        self.note_self_sumti_mention_with_availability(argument_id, false);
                    }
                } else {
                    self.note_self_sumti_mention_with_availability(argument_id, false);
                }
                for free_modifier in &sumti.free_modifiers {
                    self.walk_node(free_modifier);
                }
                true
            }
            generated::SumtiBaseSyntax::NumberSumti(sumti) => {
                self.walk_node(&sumti.expression);
                false
            }
            generated::SumtiBaseSyntax::QuotedSumti(sumti) => {
                self.visit_quote(&sumti.0);
                false
            }
            generated::SumtiBaseSyntax::BridiDescriptionSumti(sumti) => {
                self.visit_statement(&sumti.statement);
                false
            }
            generated::SumtiBaseSyntax::LaheSumti(sumti) => {
                if let Some(clauses) = &sumti.relative_clauses {
                    self.visit_relative_clause_list(argument_id, argument_id, clauses);
                }
                self.visit_argument(&sumti.inner_sumti);
                false
            }
            generated::SumtiBaseSyntax::ScalarNegatedSumti(sumti) => {
                self.visit_argument(&sumti.inner_sumti);
                false
            }
            generated::SumtiBaseSyntax::ScalarNegatedSumtiWithBo(sumti) => {
                self.visit_argument(&sumti.inner_sumti);
                false
            }
            generated::SumtiBaseSyntax::LaheTermWrapper(sumti) => {
                self.walk_node(&sumti.inner_term);
                false
            }
            generated::SumtiBaseSyntax::ScalarNegatedTermWrapper(sumti) => {
                self.walk_node(&sumti.inner_term);
                false
            }
            generated::SumtiBaseSyntax::ScalarNegatedTermWrapperWithBo(sumti) => {
                self.walk_node(&sumti.inner_term);
                false
            }
            generated::SumtiBaseSyntax::DescriptorWithOuterQuantifierSumti(sumti) => {
                self.walk_node(&sumti.outer_quantifier);
                self.visit_description_tail(argument_id, &sumti.tail);
                false
            }
            generated::SumtiBaseSyntax::DescriptorWithGadriSumti(sumti) => {
                self.visit_description_tail(argument_id, &sumti.tail);
                false
            }
            generated::SumtiBaseSyntax::DescriptionConnectionSumti(sumti) => {
                self.visit_description_tail(argument_id, &sumti.tail);
                false
            }
            generated::SumtiBaseSyntax::DescriptorWithoutGadriSumti(sumti) => {
                self.walk_node(&sumti.quantifier);
                self.visit_relation(&sumti.selbri);
                if let Some(clauses) = &sumti.relative_clauses {
                    self.visit_relative_clause_list(argument_id, argument_id, clauses);
                }
                false
            }
            generated::SumtiBaseSyntax::NameSumti(name) => {
                if let Some(clauses) = &name.relative_clauses {
                    self.visit_relative_clause_list(argument_id, argument_id, clauses);
                }
                false
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_description_tail(
        &mut self,
        argument_id: SumtiNodeId,
        tail: &'tree generated::DescriptionTailSyntax,
    ) {
        let mut current_relative_head = None;
        if let Some(tail_sumti) = &tail.leading_tail_elements.tail_sumti {
            self.visit_sumti_base(argument_id, &tail_sumti.0);
            current_relative_head = Some(argument_id);
        }
        if let Some(clauses) = &tail.leading_tail_elements.relative_clauses {
            if let Some(head) = current_relative_head {
                self.visit_relative_clause_list(head, head, clauses);
            } else {
                self.visit_relative_clause_list_without_head(clauses);
            }
        }
        match tail.tail.as_ref() {
            generated::DescriptionTailBodySyntax::RelationDescriptionTail(tail) => {
                self.visit_relation(&tail.selbri);
                if let Some(clauses) = &tail.relative_clauses {
                    self.visit_relative_clause_list(argument_id, argument_id, clauses);
                }
            }
            generated::DescriptionTailBodySyntax::QuantifierRelationDescriptionTail(tail) => {
                self.walk_node(&tail.quantifier);
                self.visit_relation(&tail.selbri);
                if let Some(clauses) = &tail.relative_clauses {
                    self.visit_relative_clause_list(argument_id, argument_id, clauses);
                }
            }
            generated::DescriptionTailBodySyntax::QuantifierSumtiDescriptionTail(tail) => {
                self.walk_node(&tail.quantifier);
                self.visit_argument(&tail.sumti);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_quote(&mut self, quote: &'tree generated::QuoteSyntax) {
        if let generated::QuoteSyntax::TextQuote(quote) = quote {
            self.visit_text_with_quote_anaphora_context(&quote.text);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_text_with_quote_anaphora_context(&mut self, text: &'tree GeneratedTextSyntax) {
        if self.quote_depth > 0 {
            self.quote_depth += 1;
            self.walk_node(text);
            self.quote_depth -= 1;
            return;
        }

        self.quote_depth = 1;
        // CLL 7.4 defines the di'u-series as discourse-relative, and Example
        // 7.16 uses outer di'e to refer to a following quotation. Quoted text
        // therefore gets its own utterance history while outer pending di'e
        // references remain outside the quote context.
        std::mem::swap(&mut self.sumti_mentions, &mut self.quote_sumti_mentions);
        std::mem::swap(
            &mut self.letter_sumti_mentions,
            &mut self.quote_letter_sumti_mentions,
        );
        std::mem::swap(
            &mut self.predicate_mentions,
            &mut self.quote_predicate_mentions,
        );
        std::mem::swap(
            &mut self.utterance_history,
            &mut self.quote_utterance_history,
        );
        std::mem::swap(
            &mut self.current_utterance,
            &mut self.quote_current_utterance,
        );
        std::mem::swap(
            &mut self.pending_next_utterance_sources,
            &mut self.quote_pending_next_utterance_sources,
        );
        let outer_predicate_stack = std::mem::take(&mut self.predicate_stack);
        let outer_discourse_predicate_stack = std::mem::take(&mut self.discourse_predicate_stack);
        let outer_current_bridi = self.current_bridi.take();
        let outer_cei_bridi_bindings = std::mem::take(&mut self.cei_bridi_bindings);
        self.walk_node(text);
        self.flush_unresolved_pending_next_utterance_sources();
        self.cei_bridi_bindings = outer_cei_bridi_bindings;
        self.current_bridi = outer_current_bridi;
        self.discourse_predicate_stack = outer_discourse_predicate_stack;
        self.predicate_stack = outer_predicate_stack;
        std::mem::swap(
            &mut self.pending_next_utterance_sources,
            &mut self.quote_pending_next_utterance_sources,
        );
        std::mem::swap(
            &mut self.current_utterance,
            &mut self.quote_current_utterance,
        );
        std::mem::swap(
            &mut self.utterance_history,
            &mut self.quote_utterance_history,
        );
        std::mem::swap(
            &mut self.predicate_mentions,
            &mut self.quote_predicate_mentions,
        );
        std::mem::swap(
            &mut self.letter_sumti_mentions,
            &mut self.quote_letter_sumti_mentions,
        );
        std::mem::swap(&mut self.sumti_mentions, &mut self.quote_sumti_mentions);
        self.quote_depth = 0;
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_relative_clause_list_without_head(
        &mut self,
        clauses: &'tree generated::RelativeClauseListSyntax,
    ) {
        self.visit_relative_clause_without_head(&clauses.first);
        for tail in &clauses.additional {
            match tail {
                generated::RelativeClauseTailSyntax::JoinedRelativeClauseTail(tail) => {
                    self.visit_relative_clause_without_head(&tail.inner);
                }
                generated::RelativeClauseTailSyntax::ConnectedRelativeClauseTail(tail) => {
                    self.visit_relative_clause_without_head(&tail.inner);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_relative_clause_list(
        &mut self,
        assignment_head_id: SumtiNodeId,
        reference_head_id: SumtiNodeId,
        clauses: &'tree generated::RelativeClauseListSyntax,
    ) {
        self.visit_relative_clause(assignment_head_id, reference_head_id, &clauses.first);
        for tail in &clauses.additional {
            match tail {
                generated::RelativeClauseTailSyntax::JoinedRelativeClauseTail(tail) => {
                    self.visit_relative_clause(assignment_head_id, reference_head_id, &tail.inner);
                }
                generated::RelativeClauseTailSyntax::ConnectedRelativeClauseTail(tail) => {
                    self.visit_relative_clause(assignment_head_id, reference_head_id, &tail.inner);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_relative_clause_without_head(
        &mut self,
        clause: &'tree generated::RelativeClauseAtomSyntax,
    ) {
        match clause {
            generated::RelativeClauseAtomSyntax::SumtiAssociationRelativeClause(clause) => {
                self.visit_relative_sumti(&clause.sumti);
            }
            generated::RelativeClauseAtomSyntax::BridiRelativeClause(clause) => match clause {
                generated::BridiRelativeClauseSyntax::RestrictiveBridiRelativeClause(clause) => {
                    self.visit_subbridi(&clause.subbridi);
                }
                generated::BridiRelativeClauseSyntax::IncidentalBridiRelativeClause(clause) => {
                    self.visit_subbridi(&clause.subbridi);
                }
                generated::BridiRelativeClauseSyntax::ZantufaRestrictiveStatementRelativeClause(
                    clause,
                ) => {
                    self.visit_statement(&clause.statement);
                }
                generated::BridiRelativeClauseSyntax::ZantufaIncidentalStatementRelativeClause(
                    clause,
                ) => {
                    self.visit_statement(&clause.statement);
                }
            },
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_relative_clause(
        &mut self,
        assignment_head_id: SumtiNodeId,
        reference_head_id: SumtiNodeId,
        clause: &'tree generated::RelativeClauseAtomSyntax,
    ) {
        match clause {
            generated::RelativeClauseAtomSyntax::SumtiAssociationRelativeClause(clause) => {
                self.visit_sumti_association_relative_clause(assignment_head_id, clause);
            }
            generated::RelativeClauseAtomSyntax::BridiRelativeClause(clause) => match clause {
                generated::BridiRelativeClauseSyntax::RestrictiveBridiRelativeClause(clause) => {
                    self.relative_heads.push(reference_head_id);
                    self.visit_subbridi(&clause.subbridi);
                    self.relative_heads.pop();
                }
                generated::BridiRelativeClauseSyntax::IncidentalBridiRelativeClause(clause) => {
                    self.relative_heads.push(reference_head_id);
                    self.visit_subbridi(&clause.subbridi);
                    self.relative_heads.pop();
                }
                generated::BridiRelativeClauseSyntax::ZantufaRestrictiveStatementRelativeClause(
                    clause,
                ) => {
                    self.relative_heads.push(reference_head_id);
                    self.visit_statement(&clause.statement);
                    self.relative_heads.pop();
                }
                generated::BridiRelativeClauseSyntax::ZantufaIncidentalStatementRelativeClause(
                    clause,
                ) => {
                    self.relative_heads.push(reference_head_id);
                    self.visit_statement(&clause.statement);
                    self.relative_heads.pop();
                }
            },
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_sumti_association_relative_clause(
        &mut self,
        base_id: SumtiNodeId,
        clause: &'tree generated::SumtiAssociationRelativeClauseSyntax,
    ) {
        self.visit_relative_sumti(&clause.sumti);
        let Some(goi_argument_id) = self.relative_sumti_id(&clause.sumti) else {
            return;
        };
        let Some(marker) = clause.association_marker.value.cmavo() else {
            return;
        };
        if marker != Cmavo::Goi {
            self.add_relative_phrase_place_edges(base_id, clause, goi_argument_id);
            return;
        }
        let source = goi_argument_id.0;
        self.add_edge(
            ReferenceKind::SumtiAssociation,
            source,
            target_resolved_node(base_id.0),
            ReferenceRule::GoiEquatesHead,
        );
        if let Some(cmavo) = generated_koha_assignable_cmavo_from_relative_sumti(&clause.sumti) {
            self.koha_bindings.insert(cmavo, base_id);
        } else if let Some(cmavo) = generated_argument_koha_cmavo_from_index(self.index, base_id) {
            self.koha_bindings.insert(cmavo, goi_argument_id);
            self.add_edge(
                ReferenceKind::SumtiAssociation,
                base_id.0,
                target_resolved_node(goi_argument_id.0),
                ReferenceRule::GoiAssignsHeadProSumti,
            );
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_relative_phrase_place_edges(
        &mut self,
        base_id: SumtiNodeId,
        clause: &'tree generated::SumtiAssociationRelativeClauseSyntax,
        goi_argument_id: SumtiNodeId,
    ) {
        let Some(marker) = clause.association_marker.value.cmavo() else {
            return;
        };
        if !cmavo_is_relative_phrase_marker(marker) {
            return;
        }
        let source = self.raw_for_node(clause);
        self.add_edge(
            ReferenceKind::RelativePhraseHead,
            source,
            target_resolved_node(base_id.0),
            ReferenceRule::GoiX1RelativeHead,
        );
        self.add_edge(
            ReferenceKind::RelativePhraseArgument,
            source,
            target_resolved_node(goi_argument_id.0),
            ReferenceRule::GoiX2AttachedSumti,
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_relative_sumti(&mut self, sumti: &'tree generated::RelativeSumtiSyntax) {
        match sumti {
            generated::RelativeSumtiSyntax::PlainRelativeSumti(sumti) => {
                self.visit_argument(&sumti.0);
            }
            generated::RelativeSumtiSyntax::TenseTaggedRelativeSumti(sumti) => {
                self.walk_node(&sumti.tense_modal);
                self.walk_node(&sumti.sumti);
            }
            generated::RelativeSumtiSyntax::NaKuRelativeSumti(_) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn relative_sumti_id(
        &self,
        sumti: &'tree generated::RelativeSumtiSyntax,
    ) -> Option<SumtiNodeId> {
        match sumti {
            generated::RelativeSumtiSyntax::PlainRelativeSumti(sumti) => {
                Some(SumtiNodeId(self.raw_for_node(sumti.0.as_ref())))
            }
            generated::RelativeSumtiSyntax::TenseTaggedRelativeSumti(sumti) => {
                match sumti.sumti.as_ref() {
                    generated::TaggedOrElidedSumtiSyntax::Sumti(sumti) => {
                        Some(SumtiNodeId(self.raw_for_node(sumti)))
                    }
                    generated::TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_) => None,
                }
            }
            generated::RelativeSumtiSyntax::NaKuRelativeSumti(_) => None,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_relation(&mut self, selbri: &'tree generated::SelbriSyntax) {
        match selbri {
            generated::SelbriSyntax::TaggedSelbri(selbri) => {
                self.walk_node(&selbri.tense_modal);
                self.visit_untagged_relation(&selbri.inner_selbri);
            }
            generated::SelbriSyntax::UntaggedSelbri(selbri) => {
                self.visit_untagged_relation(selbri);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_untagged_relation(&mut self, selbri: &'tree generated::UntaggedSelbriSyntax) {
        match selbri {
            generated::UntaggedSelbriSyntax::NegatedSelbri(selbri) => {
                self.visit_relation(&selbri.inner_selbri);
            }
            generated::UntaggedSelbriSyntax::CoSelbri(selbri) => {
                self.visit_co_selbri(selbri);
            }
            generated::UntaggedSelbriSyntax::ForethoughtSelbriConnection(selbri) => {
                self.visit_relation(&selbri.leading_selbri);
                self.visit_relation(&selbri.first_branch.selbri);
                for branch in &selbri.additional_branches {
                    self.visit_relation(&branch.selbri);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_co_selbri(&mut self, selbri: &'tree generated::CoSelbriSyntax) {
        self.visit_connected_selbri(&selbri.leading_selbri);
        if let Some(tail) = selbri.co_tail.as_ref() {
            self.visit_co_selbri(&tail.trailing_selbri);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_connected_selbri(&mut self, selbri: &'tree generated::ConnectedSelbriSyntax) {
        self.visit_tanru_selbri(&selbri.leading_selbri);
        for continuation in &selbri.continuations {
            self.visit_tanru_selbri(&continuation.trailing_selbri);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_tanru_selbri(&mut self, selbri: &'tree generated::TanruSelbriSyntax) {
        self.visit_relation_unit(&selbri.first_unit);
        for unit in &selbri.additional_units {
            self.visit_relation_unit(unit);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_relation_unit(&mut self, unit: &'tree generated::TanruUnitSyntax) {
        self.visit_bo_or_linked_tanru_unit(&unit.0.first);
        for link in &unit.0.links {
            self.visit_bo_or_linked_tanru_unit(&link.trailing_unit);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_bo_or_linked_tanru_unit(&mut self, unit: &'tree generated::BoOrLinkedTanruUnitSyntax) {
        match unit {
            generated::BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(unit) => {
                self.visit_relation(&unit.leading_selbri);
                self.visit_bo_or_linked_tanru_unit(&unit.first_branch.unit);
                for branch in &unit.additional_branches {
                    self.visit_bo_or_linked_tanru_unit(&branch.unit);
                }
            }
            generated::BoOrLinkedTanruUnitSyntax::BoundTanruUnit(unit) => {
                self.visit_linked_tanru_unit(&unit.leading_unit);
                if let Some(tense_modal) = unit.bo_tense_modal.as_deref() {
                    self.walk_node(tense_modal);
                }
                self.visit_bo_or_linked_tanru_unit(&unit.trailing_unit);
            }
            generated::BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => {
                self.visit_linked_tanru_unit_for_cei(&unit.base);
                for assignment in &unit.assignments {
                    self.visit_linked_tanru_unit_for_cei(&assignment.tanru_unit);
                    if let Some(label) =
                        generated_relation_unit_assignment_label(&assignment.tanru_unit)
                        && let Some(predicate_id) = self.current_bridi
                    {
                        self.cei_bridi_bindings.insert(label, predicate_id);
                    }
                    if let Some(predicate_id) = self.current_bridi {
                        self.add_edge(
                            ReferenceKind::ProBridiAssignment,
                            self.raw_for_node(assignment.tanru_unit.as_ref()),
                            target_resolved_node(predicate_id.0),
                            ReferenceRule::CeiAssignsEnclosingBridi,
                        );
                    }
                }
            }
            generated::BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => {
                self.visit_linked_tanru_unit(unit);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_linked_tanru_unit(&mut self, unit: &'tree generated::LinkedTanruUnitSyntax) {
        self.visit_tanru_unit_atom(&unit.base);
        if let Some(linkargs) = &unit.linkargs {
            self.walk_node(linkargs);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_linked_tanru_unit_for_cei(
        &mut self,
        unit: &'tree generated::LinkedTanruUnitForCeiSyntax,
    ) {
        self.visit_tanru_unit_atom_for_cei(&unit.base);
        if let Some(linkargs) = &unit.linkargs {
            self.walk_node(linkargs);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_tanru_unit_atom_for_cei(&mut self, unit: &'tree generated::TanruUnitAtomForCeiSyntax) {
        self.visit_tanru_unit_atom_base_for_cei(&unit.base);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_tanru_unit_atom_base_for_cei(
        &mut self,
        unit: &'tree generated::TanruUnitAtomBaseForCeiSyntax,
    ) {
        match unit {
            generated::TanruUnitAtomBaseForCeiSyntax::ProBridiTanruUnit(unit) => {
                self.resolve_goha_source(self.raw_for_node(unit), unit.goha.value.cmavo());
            }
            generated::TanruUnitAtomBaseForCeiSyntax::GohaWordTanruUnit(unit) => {
                self.resolve_goha_source(self.raw_for_node(unit), unit.0.value.cmavo());
            }
            generated::TanruUnitAtomBaseForCeiSyntax::WordTanruUnit(unit) => {
                if let Some(label) = CeiLabel::from_broda_word_like(unit.0.value.core_word()) {
                    self.resolve_broda_source(self.raw_for_node(unit), label);
                }
            }
            generated::TanruUnitAtomBaseForCeiSyntax::ScalarNegatedTanruUnit(unit) => {
                self.visit_scalar_negated_tanru_inner_unit(&unit.inner_unit);
            }
            generated::TanruUnitAtomBaseForCeiSyntax::JaiModalTanruUnit(unit) => {
                if let Some(tense_modal) = unit.tense_modal.as_deref() {
                    self.walk_node(tense_modal);
                }
                self.visit_jai_inner_tanru_unit(&unit.inner_unit);
            }
            generated::TanruUnitAtomBaseForCeiSyntax::PreposedLinkargsTanruUnit(unit) => {
                self.walk_node(&unit.linkargs);
                self.visit_relation_unit(&unit.base);
            }
            generated::TanruUnitAtomBaseForCeiSyntax::AbstractionTanruUnit(unit) => {
                self.visit_abstraction(unit);
            }
            generated::TanruUnitAtomBaseForCeiSyntax::ZantufaStatementAbstractionTanruUnit(
                unit,
            ) => {
                self.visit_statement(&unit.statement);
            }
            generated::TanruUnitAtomBaseForCeiSyntax::SumtiSelbriTanruUnit(unit) => {
                self.walk_node(&unit.sumti);
            }
            generated::TanruUnitAtomBaseForCeiSyntax::OperatorSelbriTanruUnit(unit) => {
                self.walk_node(&unit.mekso_operator);
            }
            generated::TanruUnitAtomBaseForCeiSyntax::ZantufaMeTanruUnit(unit) => {
                self.walk_node(unit);
            }
            generated::TanruUnitAtomBaseForCeiSyntax::ZantufaMexMoiTanruUnit(unit) => {
                self.walk_node(&unit.expression);
            }
            generated::TanruUnitAtomBaseForCeiSyntax::TextSelbriTanruUnit(unit) => {
                self.walk_node(&unit.text);
            }
            generated::TanruUnitAtomBaseForCeiSyntax::TagSelbriTanruUnit(unit) => {
                self.walk_node(&unit.tag);
            }
            generated::TanruUnitAtomBaseForCeiSyntax::GroupedTanruUnit(unit) => {
                self.visit_connected_selbri(&unit.selbri);
            }
            generated::TanruUnitAtomBaseForCeiSyntax::OrdinalTanruUnit(_)
            | generated::TanruUnitAtomBaseForCeiSyntax::QuotedBridiSelbriTanruUnit(_)
            | generated::TanruUnitAtomBaseForCeiSyntax::QuotedTextSelbriTanruUnit(_) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_tanru_unit_atom(&mut self, unit: &'tree generated::TanruUnitAtomSyntax) {
        self.visit_tanru_unit_atom_base(&unit.base);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_tanru_unit_atom_base(&mut self, unit: &'tree generated::TanruUnitAtomBaseSyntax) {
        match unit {
            generated::TanruUnitAtomBaseSyntax::ProBridiTanruUnit(unit) => {
                self.resolve_goha_source(self.raw_for_node(unit), unit.goha.value.cmavo());
            }
            generated::TanruUnitAtomBaseSyntax::GohaWordTanruUnit(unit) => {
                self.resolve_goha_source(self.raw_for_node(unit), unit.0.value.cmavo());
            }
            generated::TanruUnitAtomBaseSyntax::WordTanruUnit(unit) => {
                if let Some(label) = CeiLabel::from_broda_word_like(unit.0.value.core_word()) {
                    self.resolve_broda_source(self.raw_for_node(unit), label);
                }
            }
            generated::TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit) => {
                self.visit_scalar_negated_tanru_inner_unit(&unit.inner_unit);
            }
            generated::TanruUnitAtomBaseSyntax::JaiModalTanruUnit(unit) => {
                if let Some(tense_modal) = unit.tense_modal.as_deref() {
                    self.walk_node(tense_modal);
                }
                self.visit_jai_inner_tanru_unit(&unit.inner_unit);
            }
            generated::TanruUnitAtomBaseSyntax::PreposedLinkargsTanruUnit(unit) => {
                self.walk_node(&unit.linkargs);
                self.visit_relation_unit(&unit.base);
            }
            generated::TanruUnitAtomBaseSyntax::AbstractionTanruUnit(unit) => {
                self.visit_abstraction(unit);
            }
            generated::TanruUnitAtomBaseSyntax::ZantufaStatementAbstractionTanruUnit(unit) => {
                self.visit_statement(&unit.statement);
            }
            generated::TanruUnitAtomBaseSyntax::SumtiSelbriTanruUnit(unit) => {
                self.walk_node(&unit.sumti);
            }
            generated::TanruUnitAtomBaseSyntax::OperatorSelbriTanruUnit(unit) => {
                self.walk_node(&unit.mekso_operator);
            }
            generated::TanruUnitAtomBaseSyntax::ZantufaMeTanruUnit(unit) => {
                self.walk_node(unit);
            }
            generated::TanruUnitAtomBaseSyntax::ZantufaMexMoiTanruUnit(unit) => {
                self.walk_node(&unit.expression);
            }
            generated::TanruUnitAtomBaseSyntax::TextSelbriTanruUnit(unit) => {
                self.walk_node(&unit.text);
            }
            generated::TanruUnitAtomBaseSyntax::TagSelbriTanruUnit(unit) => {
                self.walk_node(&unit.tag);
            }
            generated::TanruUnitAtomBaseSyntax::GroupedTanruUnit(unit) => {
                self.visit_connected_selbri(&unit.selbri);
            }
            generated::TanruUnitAtomBaseSyntax::OrdinalTanruUnit(_)
            | generated::TanruUnitAtomBaseSyntax::QuotedBridiSelbriTanruUnit(_)
            | generated::TanruUnitAtomBaseSyntax::QuotedTextSelbriTanruUnit(_) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_zantufa_me_tanru_unit(&mut self, unit: &'tree generated::ZantufaMeTanruUnitSyntax) {
        match unit.body.as_ref() {
            generated::ZantufaMeSelbriBodySyntax::ZantufaMeOperatorSelbriBody(body) => {
                for operator in &body.0 {
                    self.walk_node(operator);
                }
            }
            generated::ZantufaMeSelbriBodySyntax::ZantufaMeMeksoSelbriBody(body) => {
                self.walk_node(&body.0);
            }
            generated::ZantufaMeSelbriBodySyntax::ZantufaMeTagSelbriBody(body) => {
                self.walk_node(&body.0);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_scalar_negated_tanru_inner_unit(
        &mut self,
        unit: &'tree generated::ScalarNegatedTanruInnerUnitSyntax,
    ) {
        match unit {
            generated::ScalarNegatedTanruInnerUnitSyntax::TaggedSelbriGroupTanruUnit(unit) => {
                self.walk_node(&unit.tense_modal);
                self.visit_connected_selbri(&unit.inner_selbri);
            }
            generated::ScalarNegatedTanruInnerUnitSyntax::ProBridiTanruUnit(unit) => {
                self.resolve_goha_source(self.raw_for_node(unit), unit.goha.value.cmavo());
            }
            generated::ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(unit) => {
                self.visit_tanru_unit_atom(unit);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_jai_inner_tanru_unit(&mut self, unit: &'tree generated::JaiInnerTanruUnitSyntax) {
        match unit {
            generated::JaiInnerTanruUnitSyntax::ConvertedJaiInnerTanruUnit(unit) => {
                self.visit_jai_inner_tanru_unit(&unit.inner_unit);
            }
            generated::JaiInnerTanruUnitSyntax::ScalarNegatedJaiInnerTanruUnit(unit) => {
                self.visit_jai_inner_tanru_unit(&unit.inner_unit);
            }
            generated::JaiInnerTanruUnitSyntax::SumtiSelbriTanruUnit(unit) => {
                self.walk_node(&unit.sumti);
            }
            generated::JaiInnerTanruUnitSyntax::OperatorSelbriTanruUnit(unit) => {
                self.walk_node(&unit.mekso_operator);
            }
            generated::JaiInnerTanruUnitSyntax::TextSelbriTanruUnit(unit) => {
                self.walk_node(&unit.text);
            }
            generated::JaiInnerTanruUnitSyntax::GroupedJaiInnerTanruUnit(unit) => {
                self.visit_connected_jai_inner_selbri(&unit.selbri);
            }
            generated::JaiInnerTanruUnitSyntax::ProBridiTanruUnit(unit) => {
                self.resolve_goha_source(self.raw_for_node(unit), unit.goha.value.cmavo());
            }
            generated::JaiInnerTanruUnitSyntax::WordTanruUnit(unit) => {
                if let Some(label) = CeiLabel::from_broda_word_like(unit.0.value.core_word()) {
                    self.resolve_broda_source(self.raw_for_node(unit), label);
                }
            }
            generated::JaiInnerTanruUnitSyntax::OrdinalTanruUnit(_)
            | generated::JaiInnerTanruUnitSyntax::QuotedBridiSelbriTanruUnit(_)
            | generated::JaiInnerTanruUnitSyntax::QuotedTextSelbriTanruUnit(_) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_connected_jai_inner_selbri(
        &mut self,
        selbri: &'tree generated::ConnectedJaiInnerSelbriSyntax,
    ) {
        self.visit_tanru_jai_inner_selbri(&selbri.leading_selbri);
        for continuation in &selbri.continuations {
            self.visit_tanru_jai_inner_selbri(&continuation.trailing_selbri);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_tanru_jai_inner_selbri(
        &mut self,
        selbri: &'tree generated::TanruJaiInnerSelbriSyntax,
    ) {
        self.visit_jai_inner_tanru_unit(&selbri.first_unit);
        for unit in &selbri.additional_units {
            self.visit_jai_inner_tanru_unit(unit);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_sumti_selbri_sumti(&mut self, sumti: &'tree generated::SumtiSelbriSumtiSyntax) {
        match sumti {
            generated::SumtiSelbriSumtiSyntax::Sumti(sumti) => self.visit_argument(sumti),
            generated::SumtiSelbriSumtiSyntax::MeLerfuSumti(_) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_abstraction(&mut self, abstraction: &'tree generated::AbstractionTanruUnitSyntax) {
        let abstraction_id = self.raw_for_node(abstraction);
        self.abstraction_stack.push(abstraction_id);
        self.visit_subbridi(&abstraction.subbridi);
        self.abstraction_stack.pop();
    }

    #[requires(true)]
    #[ensures(self.predicate_mentions.len() == old(self.predicate_mentions.len()) + 1)]
    fn note_predicate_mention(&mut self, source: RawSyntaxNodeId) {
        let position = self
            .index
            .metadata(source)
            .and_then(|metadata| {
                metadata
                    .first_source_span
                    .as_ref()
                    .map(|span| span.byte_start)
                    .or(Some(metadata.preorder))
            })
            .unwrap_or(source.0);
        insert_node_mention_sorted(
            &mut self.predicate_mentions,
            NodeMention {
                source,
                target: source,
                position,
            },
        );
    }

    #[requires(true)]
    #[ensures(self.sumti_mentions.len() == old(self.sumti_mentions.len()) + 1)]
    fn note_self_sumti_mention_with_availability(
        &mut self,
        source: SumtiNodeId,
        available_to_ri: bool,
    ) {
        self.note_sumti_mention_with_availability(source, source, available_to_ri);
    }

    #[requires(true)]
    #[ensures(self.sumti_mentions.len() == old(self.sumti_mentions.len()) + 1)]
    fn note_sumti_mention_with_availability(
        &mut self,
        source: SumtiNodeId,
        target: SumtiNodeId,
        available_to_ri: bool,
    ) {
        let position = self.sumti_mention_position(source);
        insert_sumti_mention_sorted(
            &mut self.sumti_mentions,
            SumtiMention {
                source,
                target,
                position,
                available_to_ri,
            },
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn note_letter_sumti_antecedent(
        &mut self,
        source: SumtiNodeId,
        sumti: &'tree generated::SumtiSyntax,
    ) {
        let keys = generated_argument_letter_keys(sumti);
        let position = self.sumti_mention_position(source);
        for key in keys {
            self.letter_sumti_mentions
                .entry(key)
                .or_default()
                .push(SumtiMention {
                    source,
                    target: source,
                    position,
                    available_to_ri: false,
                });
        }
    }

    #[requires(!base_letter.is_empty())]
    #[ensures(true)]
    fn resolve_letter_target(&self, base_letter: &str) -> Option<SumtiNodeId> {
        self.letter_sumti_mentions
            .get(base_letter)
            .and_then(|mentions| mentions.last())
            .map(|mention| mention.target)
    }

    #[requires(true)]
    #[ensures(true)]
    fn record_wrapped_sumti_atom_koha_reference(
        &mut self,
        source: SumtiNodeId,
        base_sumti: &'tree generated::SumtiAtomSyntax,
    ) {
        self.record_wrapped_koha_reference_from_parts(
            source,
            generated_sumti_atom_koha_cmavo_with_subscript(base_sumti),
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn record_wrapped_koha_reference_from_parts(
        &mut self,
        source: SumtiNodeId,
        koha: Option<(Cmavo, Option<usize>)>,
    ) {
        let Some((cmavo, subscript)) = koha else {
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
                        ReferenceRule::WrappedRiReferenceSource,
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
                        ReferenceRule::WrappedKehaReferenceSource,
                    );
                }
            }
            _ => {}
        }
    }

    #[requires(recency_index > 0)]
    #[ensures(true)]
    fn predicate_mention_target_by_recency(&self, recency_index: usize) -> Option<RawSyntaxNodeId> {
        self.predicate_mentions
            .iter()
            .rev()
            .nth(recency_index - 1)
            .map(|mention| mention.target)
    }

    #[requires(true)]
    #[ensures(true)]
    fn sumti_mention_position(&self, source: SumtiNodeId) -> usize {
        self.index
            .metadata(source.0)
            .and_then(|metadata| {
                metadata
                    .first_source_span
                    .as_ref()
                    .map(|span| span.byte_start)
            })
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
        source: SumtiNodeId,
        recency_index: usize,
    ) -> Option<SumtiNodeId> {
        if recency_index == 0 {
            return None;
        }
        let source_position = self.sumti_mention_position(source);
        self.sumti_mentions
            .iter()
            .filter(|mention| mention.available_to_ri && mention.position < source_position)
            .rev()
            .nth(recency_index - 1)
            .map(|mention| mention.target)
    }

    #[requires(true)]
    #[ensures(true)]
    fn resolve_koha(
        &mut self,
        source: SumtiNodeId,
        cmavo: Option<Cmavo>,
        subscript: Option<usize>,
    ) -> Option<SumtiNodeId> {
        let Some(cmavo) = cmavo else {
            return None;
        };
        match cmavo {
            Cmavo::Ri => {
                let target_argument =
                    self.latest_argument_mention_target_before(source, subscript.unwrap_or(1));
                let target = target_argument
                    .map(|sumti| target_resolved_node(sumti.0))
                    .unwrap_or_else(|| target_unresolved("ri has no prior sumti"));
                self.add_edge(
                    ReferenceKind::Ri,
                    source.0,
                    target,
                    ReferenceRule::RiPreviousSumti,
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
                    ReferenceRule::CehuCurrentAbstraction,
                );
                None
            }
            Cmavo::Ra => {
                self.add_edge(
                    ReferenceKind::Ra,
                    source.0,
                    target_vague(VagueReferenceKind::DistantSumti),
                    ReferenceRule::RaVague,
                );
                None
            }
            Cmavo::Ru => {
                self.add_edge(
                    ReferenceKind::Ru,
                    source.0,
                    target_vague(VagueReferenceKind::DistantSumti),
                    ReferenceRule::RuVague,
                );
                None
            }
            Cmavo::Keha => {
                let target = subscript
                    .unwrap_or(1)
                    .checked_sub(1)
                    .and_then(|index| self.relative_heads.iter().rev().nth(index).copied())
                    .map(|sumti| target_resolved_node(sumti.0))
                    .unwrap_or_else(|| target_unresolved("ke'a is outside a relative clause"));
                self.add_edge(
                    ReferenceKind::Keha,
                    source.0,
                    target,
                    ReferenceRule::KehaCurrentRelativeHead,
                );
                None
            }
            Cmavo::Dihe => {
                self.pending_next_utterance_sources.push(source.0);
                None
            }
            Cmavo::Dei | Cmavo::Dihu => {
                let target_node = match cmavo {
                    Cmavo::Dei => self.current_utterance,
                    Cmavo::Dihu => self.utterance_history.last().copied(),
                    _ => unreachable!("utterance arm only handles dei and di'u"),
                };
                let target = target_node.map(target_resolved_node).unwrap_or_else(|| {
                    target_unresolved("utterance reference has no determinate target")
                });
                self.add_edge(
                    ReferenceKind::Utterance,
                    source.0,
                    target,
                    ReferenceRule::NeighborUtteranceByForm,
                );
                None
            }
            Cmavo::Voha | Cmavo::Vohe | Cmavo::Vohi | Cmavo::Voho | Cmavo::Vohu => {
                let slot = voha_slot(cmavo);
                let target = slot
                    .and_then(|slot| {
                        self.current_bridi_frames
                            .first()
                            .copied()
                            .map(|frame| (frame, slot))
                    })
                    .and_then(|(frame, slot)| self.places.first_argument_for_place(frame, slot))
                    .map(|sumti| target_resolved_node(sumti.0))
                    .unwrap_or_else(|| {
                        target_unresolved("vo'a-series place is not filled in the current bridi")
                    });
                self.add_edge(
                    ReferenceKind::VohaSeries,
                    source.0,
                    target,
                    ReferenceRule::VohaCurrentBridiPlace,
                );
                None
            }
            Cmavo::Da | Cmavo::De | Cmavo::Di => {
                if let Some(target) = self.da_bindings.get(&cmavo).copied() {
                    self.add_edge(
                        ReferenceKind::DaSeries,
                        source.0,
                        target_resolved_node(target.0),
                        ReferenceRule::DaActiveVariableBinding,
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
                        ReferenceRule::KohaGoiBinding,
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
    fn resolve_goha_source(&mut self, source: RawSyntaxNodeId, cmavo: Option<Cmavo>) {
        let Some(cmavo) = cmavo else {
            return;
        };
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
                    ReferenceRule::GohiPreviousBridi,
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
                    ReferenceRule::GoheSecondPriorBridi,
                );
            }
            Cmavo::Goha | Cmavo::Gohu | Cmavo::Goho => {
                self.add_edge(
                    ReferenceKind::GohaSeries,
                    source,
                    target_vague(VagueReferenceKind::Bridi),
                    ReferenceRule::GohaUnresolvedContextSensitive,
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
                    ReferenceRule::NeiCurrentBridi,
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
                    ReferenceRule::NohaOuterBridi,
                );
            }
            Cmavo::Buha | Cmavo::Buhe | Cmavo::Buhi => {
                if let Some(target) = self.selbri_variable_bindings.get(&cmavo).copied() {
                    self.add_edge(
                        ReferenceKind::BrodaSeries,
                        source,
                        target_resolved_node(target.0),
                        ReferenceRule::PrenexBindingProSelbri,
                    );
                }
                if let Some(label) = CeiLabel::from_buha_cmavo(cmavo)
                    && let Some(target) = self.cei_bridi_bindings.get(&label).copied()
                {
                    self.add_edge(
                        ReferenceKind::BrodaSeries,
                        source,
                        target_resolved_node(target.0),
                        ReferenceRule::CeiBindingProBridi,
                    );
                }
            }
            _ => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn resolve_broda_source(&mut self, source: RawSyntaxNodeId, label: CeiLabel) {
        if let Some(target) = self.cei_bridi_bindings.get(&label).copied() {
            self.add_edge(
                ReferenceKind::BrodaSeries,
                source,
                target_resolved_node(target.0),
                ReferenceRule::CeiBindingBroda,
            );
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_edge(
        &mut self,
        kind: ReferenceKind,
        source: RawSyntaxNodeId,
        target: ReferenceTarget,
        rule: ReferenceRule,
    ) {
        let id = ReferenceEdgeId(self.edges.len());
        self.edges.push(ReferenceEdge {
            id,
            kind,
            source,
            target,
            rule,
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn raw_for_node<N: GeneratedSyntaxTreeNode>(&self, node: &'tree N) -> RawSyntaxNodeId {
        self.index.id_for_tree_node(node).unwrap_or_else(|| {
            panic!(
                "generated syntax node belongs to indexed syntax tree: {:?}",
                node.as_node_ref().map(|node| node.constructor_name())
            )
        })
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_node<N>(&mut self, node: &'tree N)
    where
        N: GeneratedSyntaxTreeWalkable<'tree> + ?Sized,
    {
        GeneratedSyntaxTreeWalkable::walk_with(node, self);
    }
}

impl<'index, 'tree> GeneratedSyntaxTreeWalker<'tree>
    for GeneratedDiscourseReferenceBuilder<'index, 'tree>
{
    #[requires(true)]
    #[ensures(true)]
    fn walk_text(&mut self, node: &'tree GeneratedTextSyntax) {
        match node {
            generated::TextSyntax::ExplicitXauhaLohoiText(text) => {
                self.walk_node(&text.0);
            }
            generated::TextSyntax::RegularText(text) => {
                for free_modifier in &text.leading_free_modifiers {
                    self.walk_node(free_modifier);
                }
                for statement in &text.leading_i_statements {
                    for free_modifier in &statement.free_modifiers {
                        self.walk_node(free_modifier);
                    }
                }
                if let Some(paragraphs) = text.paragraphs.as_deref() {
                    self.walk_node(paragraphs);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_text_paragraphs(&mut self, node: &'tree generated::TextParagraphsSyntax) {
        match node {
            generated::TextParagraphsSyntax::TextParagraphWithAdditionalNiho(paragraphs) => {
                self.walk_node(paragraphs);
            }
            generated::TextParagraphsSyntax::TextNihoParagraphs(paragraphs) => {
                for paragraph in &paragraphs.0 {
                    self.walk_node(paragraph);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_text_paragraph_with_additional_niho(
        &mut self,
        node: &'tree generated::TextParagraphWithAdditionalNihoSyntax,
    ) {
        self.walk_node(&node.first);
        for paragraph in &node.additional_niho {
            self.walk_node(paragraph);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_paragraph(&mut self, node: &'tree generated::ParagraphSyntax) {
        match node {
            generated::ParagraphSyntax::SimpleParagraph(paragraph) => {
                self.walk_node(&paragraph.0);
            }
            generated::ParagraphSyntax::INihoParagraph(paragraph) => {
                for free_modifier in &paragraph.free_modifiers {
                    self.walk_node(free_modifier);
                }
                if let Some(statements) = paragraph.statements.as_deref() {
                    self.walk_node(statements);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_niho_paragraph(&mut self, node: &'tree generated::NihoParagraphSyntax) {
        for free_modifier in &node.free_modifiers {
            self.walk_node(free_modifier);
        }
        if let Some(statements) = node.statements.as_deref() {
            self.walk_node(statements);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_paragraph_statement_sequence(
        &mut self,
        node: &'tree generated::ParagraphStatementSequenceSyntax,
    ) {
        self.walk_node(&node.initial.0);
        for statement in &node.following {
            for free_modifier in &statement.free_modifiers {
                self.walk_node(free_modifier);
            }
            if let Some(statement) = statement.statement.as_deref() {
                self.walk_node(statement);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_statement_or_fragment(&mut self, node: &'tree generated::StatementOrFragmentSyntax) {
        match node {
            generated::StatementOrFragmentSyntax::ZantufaStatementTermsStatement(statement) => {
                self.visit_statement(&statement.statement);
                self.walk_node(&statement.tail);
            }
            generated::StatementOrFragmentSyntax::StatementOrFragmentStatement(statement) => {
                self.visit_statement(&statement.0);
            }
            generated::StatementOrFragmentSyntax::FragmentStatement(fragment) => {
                self.walk_node(fragment);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_statement(&mut self, node: &'tree generated::StatementSyntax) {
        self.visit_statement(node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_i_statement_connection_tail(
        &mut self,
        node: &'tree generated::IStatementConnectionTailSyntax,
    ) {
        match node {
            generated::IStatementConnectionTailSyntax::ChainedIConnectiveStatementTail(tail) => {
                self.walk_node(&tail.trailing_statement);
            }
            generated::IStatementConnectionTailSyntax::SimpleIConnectiveStatementTail(tail) => {
                self.walk_node(&tail.trailing_statement);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_statement_after_i_connective(
        &mut self,
        node: &'tree generated::StatementAfterIConnectiveSyntax,
    ) {
        match node {
            generated::StatementAfterIConnectiveSyntax::BridiStatement(statement) => {
                self.walk_node(statement);
            }
            generated::StatementAfterIConnectiveSyntax::TextGroupStatement(statement) => {
                self.walk_node(statement);
            }
            generated::StatementAfterIConnectiveSyntax::ForethoughtStatement(statement) => {
                self.walk_node(statement);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_forethought_statement(&mut self, node: &'tree generated::ForethoughtStatementSyntax) {
        self.visit_statement(&node.first);
        self.visit_statement(&node.first_branch.statement);
        for branch in &node.additional_branches {
            self.visit_statement(&branch.statement);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_text_group_statement(&mut self, node: &'tree generated::TextGroupStatementSyntax) {
        if let Some(tense_modal) = node.tense_modal.as_deref() {
            self.walk_node(tense_modal);
        }
        self.walk_node(&node.text);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_bridi_statement(&mut self, node: &'tree generated::BridiStatementSyntax) {
        self.visit_predicate(&node.bridi);
        for continuation in &node.continuations {
            match continuation {
                generated::BridiStatementContinuationSyntax::BoBridiStatementContinuation(
                    continuation,
                ) => {
                    if let Some(tense_modal) = continuation.tense_modal.as_deref() {
                        self.walk_node(tense_modal);
                    }
                    self.visit_subbridi(&continuation.trailing_subbridi);
                }
                generated::BridiStatementContinuationSyntax::KeBridiStatementContinuation(
                    continuation,
                ) => {
                    if let Some(tense_modal) = continuation.tense_modal.as_deref() {
                        self.walk_node(tense_modal);
                    }
                    self.visit_subbridi(&continuation.trailing_subbridi);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_bridi(&mut self, node: &'tree generated::BridiSyntax) {
        self.visit_predicate(node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_term(&mut self, node: &'tree generated::TermSyntax) {
        match node {
            generated::TermSyntax::SimpleTerm(term) => self.walk_node(term),
            generated::TermSyntax::ConnectedTerm(term) => {
                self.walk_node(&term.leading_term);
                for continuation in &term.continuations {
                    self.walk_node(&continuation.trailing_term);
                }
            }
            generated::TermSyntax::BoundTermConnection(term) => {
                self.walk_node(&term.leading_term);
                self.walk_node(&term.trailing_term);
            }
            generated::TermSyntax::TermsetGroup(term) => {
                self.walk_node(&term.leading_term);
                for continuation in &term.continuations {
                    self.walk_node(&continuation.trailing_term);
                }
            }
            generated::TermSyntax::PeheTermsetConnection(term) => {
                self.walk_node(&term.leading_term);
                for continuation in &term.continuations {
                    self.walk_node(&continuation.trailing_term);
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_pehe_termset_operand(&mut self, node: &'tree generated::PeheTermsetOperandSyntax) {
        match node {
            generated::PeheTermsetOperandSyntax::SimpleTerm(term) => self.walk_node(term),
            generated::PeheTermsetOperandSyntax::TermsetGroup(term) => {
                self.walk_node(&term.leading_term);
                for continuation in &term.continuations {
                    self.walk_node(&continuation.trailing_term);
                }
            }
            generated::PeheTermsetOperandSyntax::BoundTermConnection(term) => {
                self.walk_node(&term.leading_term);
                self.walk_node(&term.trailing_term);
            }
            generated::PeheTermsetOperandSyntax::StagBoundTermConnection(term) => {
                self.walk_node(term)
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_simple_term(&mut self, node: &'tree generated::SimpleTermSyntax) {
        match node {
            generated::SimpleTermSyntax::SumtiTerm(term) => self.visit_argument(&term.0),
            generated::SimpleTermSyntax::PlaceTaggedSumtiTerm(term) => {
                self.walk_node(&term.sumti);
            }
            generated::SimpleTermSyntax::TaggedSumtiTerm(term) => {
                self.walk_node(&term.tense_modal);
                self.walk_node(&term.sumti);
            }
            generated::SimpleTermSyntax::JaiTaggedSumtiTerm(term) => {
                if let Some(tense_modal) = term.tag.as_deref() {
                    self.walk_node(tense_modal);
                }
                self.visit_argument(&term.sumti);
            }
            generated::SimpleTermSyntax::ForethoughtTermset(term) => {
                for term in &term.terms {
                    self.walk_node(term.as_ref());
                }
                for term in &term.first_branch.terms {
                    self.walk_node(term.as_ref());
                }
                for branch in &term.additional_branches {
                    for term in &branch.terms {
                        self.walk_node(term.as_ref());
                    }
                }
            }
            generated::SimpleTermSyntax::NuhiTermset(term) => {
                for term in &term.termset {
                    self.walk_node(term.as_ref());
                }
            }
            generated::SimpleTermSyntax::KeTermset(term) => {
                for term in &term.termset {
                    self.walk_node(term.as_ref());
                }
            }
            generated::SimpleTermSyntax::NoihaAdverbialTerm(term) => match term {
                generated::NoihaAdverbialTermSyntax::NoihaVariableAdverbialTerm(term) => {
                    for free_modifier in &term.free_modifiers {
                        self.walk_node(free_modifier);
                    }
                    self.visit_relation(&term.selbri);
                }
                generated::NoihaAdverbialTermSyntax::NoihaRelativeAdverbialTerm(term) => {
                    self.visit_relation(&term.selbri);
                }
            },
            generated::SimpleTermSyntax::FihoiAdverbialTerm(term) => {
                self.visit_statement(&term.statement);
            }
            generated::SimpleTermSyntax::SoiAdverbialTerm(term) => {
                self.visit_statement(&term.statement);
            }
            generated::SimpleTermSyntax::TaggedSumtiBeforeTagTerm(term) => {
                self.walk_node(&term.0);
            }
            generated::SimpleTermSyntax::NaKuTerm(_)
            | generated::SimpleTermSyntax::BareNaTerm(_) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_bound_term(&mut self, node: &'tree generated::BoundTermSyntax) {
        let Some(node) = GeneratedSimpleTermRef::from_bound(node) else {
            return;
        };
        match node {
            GeneratedSimpleTermRef::SumtiTerm(term) => self.visit_argument(&term.0),
            GeneratedSimpleTermRef::PlaceTaggedSumtiTerm(term) => self.walk_node(&term.sumti),
            GeneratedSimpleTermRef::TaggedSumtiTerm(term) => {
                self.walk_node(&term.tense_modal);
                self.walk_node(&term.sumti);
            }
            GeneratedSimpleTermRef::JaiTaggedSumtiTerm(term) => {
                if let Some(tense_modal) = term.tag.as_deref() {
                    self.walk_node(tense_modal);
                }
                self.visit_argument(&term.sumti);
            }
            GeneratedSimpleTermRef::ForethoughtTermset(term) => {
                for term in &term.terms {
                    self.walk_node(term.as_ref());
                }
                for term in &term.first_branch.terms {
                    self.walk_node(term.as_ref());
                }
                for branch in &term.additional_branches {
                    for term in &branch.terms {
                        self.walk_node(term.as_ref());
                    }
                }
            }
            GeneratedSimpleTermRef::NuhiTermset(term) => {
                for term in &term.termset {
                    self.walk_node(term.as_ref());
                }
            }
            GeneratedSimpleTermRef::KeTermset(term) => {
                for term in &term.termset {
                    self.walk_node(term.as_ref());
                }
            }
            GeneratedSimpleTermRef::NoihaAdverbialTerm(term) => match term {
                generated::NoihaAdverbialTermSyntax::NoihaVariableAdverbialTerm(term) => {
                    for free_modifier in &term.free_modifiers {
                        self.walk_node(free_modifier);
                    }
                    self.visit_relation(&term.selbri);
                }
                generated::NoihaAdverbialTermSyntax::NoihaRelativeAdverbialTerm(term) => {
                    self.visit_relation(&term.selbri);
                }
            },
            GeneratedSimpleTermRef::FihoiAdverbialTerm(term) => {
                self.visit_statement(&term.statement);
            }
            GeneratedSimpleTermRef::SoiAdverbialTerm(term) => {
                self.visit_statement(&term.statement);
            }
            GeneratedSimpleTermRef::TaggedSumtiBeforeTagTerm(term) => self.walk_node(&term.0),
            GeneratedSimpleTermRef::NaKuTerm(_) | GeneratedSimpleTermRef::BareNaTerm(_) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_sumti(&mut self, node: &'tree generated::SumtiSyntax) {
        self.visit_argument(node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_selbri(&mut self, node: &'tree generated::SelbriSyntax) {
        self.visit_relation(node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_relative_clause_list(&mut self, node: &'tree generated::RelativeClauseListSyntax) {
        self.visit_relative_clause_list_without_head(node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_text_quote(&mut self, _node: &'tree generated::TextQuoteSyntax) {
        // Direct quote-as-paragraph nodes never entered the old quote-context
        // visitor. Quoted sumti still use `visit_quote`, which installs the
        // dedicated quote anaphora context.
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_linkargs(&mut self, node: &'tree generated::LinkargsSyntax) {
        generated::walk::linkargs(self, node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_bei_link(&mut self, node: &'tree generated::BeiLinkSyntax) {
        generated::walk::bei_link(self, node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_quantifier(&mut self, node: &'tree generated::QuantifierSyntax) {
        generated::walk::quantifier(self, node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_mekso(&mut self, node: &'tree generated::MeksoSyntax) {
        generated::walk::mekso(self, node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_mekso_operator(&mut self, node: &'tree generated::MeksoOperatorSyntax) {
        generated::walk::mekso_operator(self, node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_leading_term_tag_tense_modal(
        &mut self,
        node: &'tree generated::LeadingTermTagTenseModalSyntax,
    ) {
        if let generated::LeadingTermTagTenseModalSyntax::TenseModal(tense_modal) = node {
            self.walk_node(tense_modal);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_tense_modal(&mut self, node: &'tree generated::TenseModalSyntax) {
        self.walk_node(&node.0);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_tense_modal_body(&mut self, node: &'tree generated::TenseModalBodySyntax) {
        match node {
            generated::TenseModalBodySyntax::ConnectedTenseModal(tense_modal) => {
                self.walk_node(&tense_modal.first);
                for continuation in &tense_modal.continuations {
                    self.walk_node(&continuation.tense_modal);
                }
            }
            generated::TenseModalBodySyntax::TenseModalAtom(tense_modal) => {
                self.walk_node(tense_modal);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_tense_modal_atom(&mut self, node: &'tree generated::TenseModalAtomSyntax) {
        if let generated::TenseModalAtomSyntax::FihoTense(fiho) = node {
            self.visit_relation(&fiho.selbri);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_free_modifier(&mut self, node: &'tree generated::FreeModifierSyntax) {
        match node {
            generated::FreeModifierSyntax::SeiFreeModifier(free_modifier) => {
                for term in &free_modifier.terms {
                    self.walk_node(term);
                }
                self.visit_relation(&free_modifier.selbri);
            }
            generated::FreeModifierSyntax::ZantufaSeiStatementFreeModifier(free_modifier) => {
                self.visit_statement(&free_modifier.statement);
            }
            generated::FreeModifierSyntax::ParentheticalText(free_modifier) => {
                self.walk_node(&free_modifier.text);
            }
            generated::FreeModifierSyntax::XiFreeModifier(free_modifier) => match free_modifier {
                generated::XiFreeModifierSyntax::XiParenthesizedFreeModifier(free_modifier) => {
                    self.walk_node(&free_modifier.expression.inner_expression);
                }
                generated::XiFreeModifierSyntax::XiNumberFreeModifier(_)
                | generated::XiFreeModifierSyntax::XiLerfuStringFreeModifier(_) => {}
            },
            generated::FreeModifierSyntax::ZantufaMeksoMaiFreeModifier(free_modifier) => {
                self.walk_node(&free_modifier.expression);
            }
            generated::FreeModifierSyntax::SoiFreeModifier(free_modifier) => {
                self.visit_argument(&free_modifier.leading_sumti);
                if let Some(sumti) = free_modifier.trailing_sumti.as_deref() {
                    self.visit_argument(sumti);
                }
            }
            generated::FreeModifierSyntax::VocativeFreeModifier(free_modifier) => {
                if let Some(sumti) = free_modifier.sumti.as_deref() {
                    self.walk_node(sumti);
                }
            }
            generated::FreeModifierSyntax::TextReplacementFreeModifier(_)
            | generated::FreeModifierSyntax::MaiFreeModifier(_) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_vocative_sumti(&mut self, node: &'tree generated::VocativeSumtiSyntax) {
        match node {
            generated::VocativeSumtiSyntax::SelbriVocativeSumti(sumti) => {
                if let Some(clauses) = &sumti.leading_relative_clauses {
                    self.visit_relative_clause_list_without_head(clauses);
                }
                self.visit_relation(&sumti.selbri);
                if let Some(clauses) = &sumti.trailing_relative_clauses {
                    self.visit_relative_clause_list_without_head(clauses);
                }
            }
            generated::VocativeSumtiSyntax::CmevlaVocativeSumti(sumti) => {
                if let Some(clauses) = &sumti.leading_relative_clauses {
                    self.visit_relative_clause_list_without_head(clauses);
                }
                if let Some(clauses) = &sumti.trailing_relative_clauses {
                    self.visit_relative_clause_list_without_head(clauses);
                }
            }
            generated::VocativeSumtiSyntax::Sumti(sumti) => self.visit_argument(sumti),
        }
    }

    // Boundary: these paragraph/statement connective nodes are not descended
    // for discourse references. The #219 TreeVisitor spike proved that
    // descending into the `i ... bo` connective `fi'o` selbri for
    // corpus.camxes.157/.159 creates references for connective material
    // instead of only the main statement. The leading-`i` wrapper itself
    // remains traversed because it can carry semantic free modifiers.
    #[requires(true)]
    #[ensures(true)]
    fn walk_trailing_ijek_paragraph_statement(
        &mut self,
        _node: &'tree generated::TrailingIjekParagraphStatementSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_pending_i_connective(&mut self, _node: &'tree generated::PendingIConnectiveSyntax) {}

    #[requires(true)]
    #[ensures(true)]
    fn walk_i_paragraph_statement_connective_i_standard_paragraph_statement_connective(
        &mut self,
        _node: &'tree generated::IStandardParagraphStatementConnectiveSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_i_paragraph_statement_connective_i_tag_bo_paragraph_statement_connective(
        &mut self,
        _node: &'tree generated::ITagBoParagraphStatementConnectiveSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_i_statement_connective_i_standard_statement_connective(
        &mut self,
        _node: &'tree generated::IStandardStatementConnectiveSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_i_statement_connective_i_tag_bo_statement_connective(
        &mut self,
        _node: &'tree generated::ITagBoStatementConnectiveSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_tagged_or_elided_sumti_tagged_elided_sumti(
        &mut self,
        _node: &'tree generated::TaggedElidedSumtiSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_sumti_base_name_sumti(&mut self, node: &'tree generated::NameSumtiSyntax) {
        generated::walk::name_sumti(self, node);
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_fragment_statement_ek_fragment(&mut self, _node: &'tree generated::EkFragmentSyntax) {}

    #[requires(true)]
    #[ensures(true)]
    fn walk_fragment_statement_gihek_fragment(
        &mut self,
        _node: &'tree generated::GihekFragmentSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_fragment_statement_multiple_na_fragment(
        &mut self,
        _node: &'tree generated::MultipleNaFragmentSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_fragment_statement_single_na_fragment(
        &mut self,
        _node: &'tree generated::SingleNaFragmentSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_linked_sumti_empty_linked_sumti(
        &mut self,
        _node: &'tree generated::EmptyLinkedSumtiSyntax,
    ) {
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_relative_sumti_na_ku_relative_sumti(
        &mut self,
        _node: &'tree generated::NaKuRelativeSumtiSyntax,
    ) {
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_bridi_leading_terms(bridi: &generated::BridiSyntax) -> &[generated::TermSyntax] {
    match bridi {
        generated::BridiSyntax::BridiWithLeadingTerms(bridi) => &bridi.leading_terms,
        generated::BridiSyntax::BridiWithPostCuTerms(bridi) => &bridi.leading_terms,
        generated::BridiSyntax::BareCuBridi(_)
        | generated::BridiSyntax::BareCuTermsBridi(_)
        | generated::BridiSyntax::RelationOnlyBridi(_) => &[],
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_bridi_tail(bridi: &generated::BridiSyntax) -> &generated::BridiTailSyntax {
    match bridi {
        generated::BridiSyntax::BridiWithLeadingTerms(bridi) => &bridi.bridi_tail,
        generated::BridiSyntax::BridiWithPostCuTerms(bridi) => &bridi.bridi_tail.bridi_tail,
        generated::BridiSyntax::BareCuBridi(bridi) => &bridi.bridi_tail,
        generated::BridiSyntax::BareCuTermsBridi(bridi) => &bridi.bridi_tail.bridi_tail,
        generated::BridiSyntax::RelationOnlyBridi(bridi) => &bridi.0,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|place| (2..=5).contains(&place)))]
fn generated_se_conversion_place<F>(se: &WithFreeModifiers<Token, F>) -> Option<u8> {
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
fn generated_fa_place_slot<F>(fa: &WithFreeModifiers<Token, F>) -> Option<PlaceSlot> {
    match fa.value.cmavo() {
        Some(Cmavo::Fa) => PlaceSlot::numbered(1),
        Some(Cmavo::Fe) => PlaceSlot::numbered(2),
        Some(Cmavo::Fi) => PlaceSlot::numbered(3),
        Some(Cmavo::Fo) => PlaceSlot::numbered(4),
        Some(Cmavo::Fu) => PlaceSlot::numbered(5),
        Some(Cmavo::Fiha) => Some(place_question_slot()),
        Some(Cmavo::Fai) => Some(fai_slot()),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_abstraction_is_property(abstraction: &generated::AbstractionTanruUnitSyntax) -> bool {
    abstraction.nu.value.cmavo() == Some(Cmavo::Ka)
}

#[requires(start > 0)]
#[ensures(ret >= start)]
fn next_generated_place_after_common_terms(start: u8, terms: &[generated::TermSyntax]) -> u8 {
    let mut cursor = PlaceCursor::new_at(SelbriPlaceFrameId(usize::MAX), start);
    for term in terms {
        advance_cursor_for_generated_term_shape(&mut cursor, term);
    }
    cursor.next_place
}

#[requires(true)]
#[ensures(true)]
fn advance_cursor_for_generated_term_shape(cursor: &mut PlaceCursor, term: &generated::TermSyntax) {
    match term {
        generated::TermSyntax::SimpleTerm(simple) => {
            advance_cursor_for_generated_simple_term_shape(
                cursor,
                GeneratedSimpleTermRef::from_simple(simple),
            );
        }
        generated::TermSyntax::ConnectedTerm(connected) => {
            if let Some(leading) = GeneratedSimpleTermRef::from_bound(&connected.leading_term) {
                advance_cursor_for_generated_simple_term_shape(cursor, leading);
            }
            for continuation in &connected.continuations {
                if let Some(trailing) =
                    GeneratedSimpleTermRef::from_bound(&continuation.trailing_term)
                {
                    advance_cursor_for_generated_simple_term_shape(cursor, trailing);
                }
            }
        }
        generated::TermSyntax::BoundTermConnection(bound) => {
            advance_cursor_for_generated_simple_term_shape(
                cursor,
                GeneratedSimpleTermRef::from_simple(&bound.leading_term),
            );
            advance_cursor_for_generated_simple_term_shape(
                cursor,
                GeneratedSimpleTermRef::from_simple(&bound.trailing_term),
            );
        }
        generated::TermSyntax::TermsetGroup(group) => {
            advance_cursor_for_generated_simple_term_shape(
                cursor,
                GeneratedSimpleTermRef::from_simple(&group.leading_term),
            );
            for continuation in &group.continuations {
                advance_cursor_for_generated_simple_term_shape(
                    cursor,
                    GeneratedSimpleTermRef::from_simple(&continuation.trailing_term),
                );
            }
        }
        generated::TermSyntax::PeheTermsetConnection(connection) => {
            advance_cursor_for_generated_pehe_operand_shape(cursor, &connection.leading_term);
            for continuation in &connection.continuations {
                advance_cursor_for_generated_pehe_operand_shape(
                    cursor,
                    &continuation.trailing_term,
                );
            }
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn advance_cursor_for_generated_pehe_operand_shape(
    cursor: &mut PlaceCursor,
    term: &generated::PeheTermsetOperandSyntax,
) {
    match term {
        generated::PeheTermsetOperandSyntax::SimpleTerm(simple) => {
            advance_cursor_for_generated_simple_term_shape(
                cursor,
                GeneratedSimpleTermRef::from_simple(simple),
            );
        }
        generated::PeheTermsetOperandSyntax::TermsetGroup(group) => {
            advance_cursor_for_generated_simple_term_shape(
                cursor,
                GeneratedSimpleTermRef::from_simple(&group.leading_term),
            );
            for continuation in &group.continuations {
                advance_cursor_for_generated_simple_term_shape(
                    cursor,
                    GeneratedSimpleTermRef::from_simple(&continuation.trailing_term),
                );
            }
        }
        generated::PeheTermsetOperandSyntax::BoundTermConnection(bound) => {
            advance_cursor_for_generated_simple_term_shape(
                cursor,
                GeneratedSimpleTermRef::from_simple(&bound.leading_term),
            );
            advance_cursor_for_generated_simple_term_shape(
                cursor,
                GeneratedSimpleTermRef::from_simple(&bound.trailing_term),
            );
        }
        generated::PeheTermsetOperandSyntax::StagBoundTermConnection(_) => {}
    }
}

#[requires(true)]
#[ensures(true)]
fn advance_cursor_for_generated_simple_term_shape(
    cursor: &mut PlaceCursor,
    term: GeneratedSimpleTermRef<'_>,
) {
    match term {
        GeneratedSimpleTermRef::SumtiTerm(term) => {
            advance_cursor_for_generated_argument_term_shape(cursor, &term.0);
        }
        GeneratedSimpleTermRef::PlaceTaggedSumtiTerm(term) => {
            let slot =
                generated_fa_place_slot(&term.fa).unwrap_or_else(|| cursor.next_numbered_slot());
            cursor.record_slot(slot);
        }
        GeneratedSimpleTermRef::TaggedSumtiTerm(term) => {
            if matches!(
                term.sumti.as_ref(),
                generated::TaggedOrElidedSumtiSyntax::TaggedElidedSumti(_)
            ) {
                return;
            }
            cursor.record_slot(modal_slot(None));
        }
        GeneratedSimpleTermRef::JaiTaggedSumtiTerm(_) => {
            cursor.record_slot(fai_slot());
        }
        GeneratedSimpleTermRef::ForethoughtTermset(term) => {
            advance_cursor_for_generated_boxed_terms_shape(cursor, &term.terms);
            advance_cursor_for_generated_boxed_terms_shape(cursor, &term.first_branch.terms);
            for branch in &term.additional_branches {
                advance_cursor_for_generated_boxed_terms_shape(cursor, &branch.terms);
            }
        }
        GeneratedSimpleTermRef::NuhiTermset(term) => {
            advance_cursor_for_generated_boxed_terms_shape(cursor, &term.termset);
        }
        GeneratedSimpleTermRef::KeTermset(term) => {
            advance_cursor_for_generated_boxed_terms_shape(cursor, &term.termset);
        }
        _ => {}
    }
}

#[requires(true)]
#[ensures(true)]
fn advance_cursor_for_generated_boxed_terms_shape<'term, I, T>(cursor: &mut PlaceCursor, terms: I)
where
    I: IntoIterator<Item = &'term T>,
    T: AsRef<generated::TermSyntax> + 'term,
{
    for term in terms {
        advance_cursor_for_generated_term_shape(cursor, term.as_ref());
    }
}

#[requires(true)]
#[ensures(true)]
fn advance_cursor_for_generated_argument_term_shape(
    cursor: &mut PlaceCursor,
    _sumti: &generated::SumtiSyntax,
) {
    let slot = cursor.next_numbered_slot();
    cursor.record_slot(slot);
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_spine_cmavo(sumti: &generated::SumtiSyntax) -> Option<Cmavo> {
    if sumti.vuho_attachment.is_some() || sumti.base_sumti.grouped_tail.is_some() {
        return None;
    }
    generated_sumti_afterthought_spine_cmavo(&sumti.base_sumti.leading_sumti)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_afterthought_spine_cmavo(
    sumti: &generated::SumtiAfterthoughtSyntax,
) -> Option<Cmavo> {
    if !sumti.continuations.is_empty() {
        return None;
    }
    generated_sumti_bound_spine_cmavo(&sumti.leading_sumti)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_bound_spine_cmavo(sumti: &generated::SumtiBoundSyntax) -> Option<Cmavo> {
    if sumti.bound_tail.is_some() {
        return None;
    }
    generated_sumti_forethought_spine_cmavo(&sumti.leading_sumti)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_forethought_spine_cmavo(
    sumti: &generated::SumtiForethoughtSyntax,
) -> Option<Cmavo> {
    match sumti {
        generated::SumtiForethoughtSyntax::SimpleSumti(simple) => {
            generated_simple_sumti_spine_cmavo(simple)
        }
        generated::SumtiForethoughtSyntax::ForethoughtSumti(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_simple_sumti_spine_cmavo(sumti: &generated::SimpleSumtiSyntax) -> Option<Cmavo> {
    generated_sumti_atom_spine_cmavo(&sumti.base_sumti)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_atom_spine_cmavo(sumti: &generated::SumtiAtomSyntax) -> Option<Cmavo> {
    match sumti {
        generated::SumtiAtomSyntax::SumtiBase(base) => generated_sumti_base_spine_cmavo(base),
        generated::SumtiAtomSyntax::QuantifiedSumti(sumti) => {
            generated_sumti_base_spine_cmavo(&sumti.inner_sumti)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_base_spine_cmavo(sumti: &generated::SumtiBaseSyntax) -> Option<Cmavo> {
    match sumti {
        generated::SumtiBaseSyntax::ProSumti(pro_sumti) => pro_sumti.0.value.cmavo(),
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_argument_wraps_ri(sumti: &generated::SumtiSyntax) -> bool {
    generated_argument_koha_cmavo_with_subscript(sumti)
        .is_some_and(|(cmavo, _subscript)| cmavo == Cmavo::Ri)
}

#[requires(true)]
#[ensures(true)]
fn generated_argument_koha_cmavo_with_subscript(
    sumti: &generated::SumtiSyntax,
) -> Option<(Cmavo, Option<usize>)> {
    if sumti.base_sumti.grouped_tail.is_some() {
        return None;
    }
    generated_sumti_afterthought_koha_cmavo_with_subscript(&sumti.base_sumti.leading_sumti)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_afterthought_koha_cmavo_with_subscript(
    sumti: &generated::SumtiAfterthoughtSyntax,
) -> Option<(Cmavo, Option<usize>)> {
    if !sumti.continuations.is_empty() {
        return None;
    }
    generated_sumti_bound_koha_cmavo_with_subscript(&sumti.leading_sumti)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_bound_koha_cmavo_with_subscript(
    sumti: &generated::SumtiBoundSyntax,
) -> Option<(Cmavo, Option<usize>)> {
    if sumti.bound_tail.is_some() {
        return None;
    }
    generated_sumti_forethought_koha_cmavo_with_subscript(&sumti.leading_sumti)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_forethought_koha_cmavo_with_subscript(
    sumti: &generated::SumtiForethoughtSyntax,
) -> Option<(Cmavo, Option<usize>)> {
    match sumti {
        generated::SumtiForethoughtSyntax::SimpleSumti(simple) => {
            generated_simple_sumti_koha_cmavo_with_subscript(simple)
        }
        generated::SumtiForethoughtSyntax::ForethoughtSumti(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_simple_sumti_koha_cmavo_with_subscript(
    sumti: &generated::SimpleSumtiSyntax,
) -> Option<(Cmavo, Option<usize>)> {
    generated_sumti_atom_koha_cmavo_with_subscript(&sumti.base_sumti)
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_atom_koha_cmavo_with_subscript(
    sumti: &generated::SumtiAtomSyntax,
) -> Option<(Cmavo, Option<usize>)> {
    match sumti {
        generated::SumtiAtomSyntax::SumtiBase(base) => {
            generated_sumti_base_koha_cmavo_with_subscript(base)
        }
        generated::SumtiAtomSyntax::QuantifiedSumti(sumti) => {
            generated_sumti_base_koha_cmavo_with_subscript(&sumti.inner_sumti)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_sumti_base_koha_cmavo_with_subscript(
    sumti: &generated::SumtiBaseSyntax,
) -> Option<(Cmavo, Option<usize>)> {
    match sumti {
        generated::SumtiBaseSyntax::ProSumti(pro_sumti) => Some((
            pro_sumti.0.value.cmavo()?,
            generated_koha_subscript_index(&pro_sumti.0.free_modifiers),
        )),
        generated::SumtiBaseSyntax::LaheSumti(sumti) => {
            generated_argument_koha_cmavo_with_subscript(&sumti.inner_sumti)
        }
        generated::SumtiBaseSyntax::ScalarNegatedSumti(sumti) => {
            generated_argument_koha_cmavo_with_subscript(&sumti.inner_sumti)
        }
        generated::SumtiBaseSyntax::ScalarNegatedSumtiWithBo(sumti) => {
            generated_argument_koha_cmavo_with_subscript(&sumti.inner_sumti)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_koha_subscript_index(
    free_modifiers: &[generated::FreeModifierSyntax],
) -> Option<usize> {
    free_modifiers
        .iter()
        .find_map(|free_modifier| match free_modifier {
            generated::FreeModifierSyntax::XiFreeModifier(
                generated::XiFreeModifierSyntax::XiNumberFreeModifier(subscript),
            ) => generated_number_words_to_usize(&subscript.expression.0.number.value),
            generated::FreeModifierSyntax::XiFreeModifier(
                generated::XiFreeModifierSyntax::XiParenthesizedFreeModifier(subscript),
            ) => generated_math_expression_to_usize(&subscript.expression.inner_expression),
            _ => None,
        })
}

#[requires(true)]
#[ensures(true)]
fn generated_math_expression_to_usize(expression: &generated::MeksoSyntax) -> Option<usize> {
    match expression {
        generated::MeksoSyntax::InfixMekso(expression) if expression.continuations.is_empty() => {
            generated_mekso_precedence_to_usize(&expression.first_expression)
        }
        generated::MeksoSyntax::ZantufaInfixMekso(expression)
            if expression.continuations.is_empty() =>
        {
            generated_mekso_precedence_to_usize(&expression.first_expression)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_mekso_precedence_to_usize(
    expression: &generated::MeksoPrecedenceSyntax,
) -> Option<usize> {
    if expression.tail.is_some() {
        return None;
    }
    generated_mekso_base_to_usize(&expression.left_expression)
}

#[requires(true)]
#[ensures(true)]
fn generated_mekso_base_to_usize(expression: &generated::MeksoBaseSyntax) -> Option<usize> {
    match expression {
        generated::MeksoBaseSyntax::MeksoOperand(operand) => {
            generated_mekso_operand_to_usize(operand)
        }
        generated::MeksoBaseSyntax::ForethoughtCallMekso(_) => None,
        generated::MeksoBaseSyntax::ZantufaBoGroupedMeksoBase(_) => None,
        generated::MeksoBaseSyntax::ZantufaGroupedMeksoOperandSequence(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_mekso_operand_to_usize(operand: &generated::MeksoOperandSyntax) -> Option<usize> {
    if operand.grouped_continuation.is_some() {
        return None;
    }
    let connected = &operand.connected_expression.0;
    if !connected.links.is_empty() {
        return None;
    }
    match connected.first.as_ref() {
        generated::BoundOrSimpleMeksoOperandSyntax::SimpleMeksoOperand(operand) => {
            generated_simple_mekso_operand_to_usize(operand)
        }
        generated::BoundOrSimpleMeksoOperandSyntax::BoundMeksoOperand(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_simple_mekso_operand_to_usize(
    operand: &generated::SimpleMeksoOperandSyntax,
) -> Option<usize> {
    match operand {
        generated::SimpleMeksoOperandSyntax::NumberMekso(number) => {
            generated_number_words_to_usize(&number.0.number.value)
        }
        generated::SimpleMeksoOperandSyntax::ParenthesizedMeksoOperand(operand) => {
            generated_math_expression_to_usize(&operand.inner_expression)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_number_words_to_usize(words: &generated::NumberWordsSyntax) -> Option<usize> {
    let mut value = cmavo_digit(words.first_number.cmavo())?;
    for continuation in &words.continuations {
        let digit = match continuation {
            generated::NumberWordContinuationSyntax::NumberWordPaContinuation(continuation) => {
                cmavo_digit(continuation.0.cmavo())?
            }
            generated::NumberWordContinuationSyntax::NumberWordLerfuContinuation(_) => return None,
        };
        value = value.checked_mul(10)?.checked_add(digit)?;
    }
    Some(value)
}

#[requires(true)]
#[ensures(!ret.iter().any(|key| key.is_empty()))]
fn generated_argument_letter_keys(sumti: &generated::SumtiSyntax) -> Vec<String> {
    let mut keys = Vec::new();
    if let Some(base_letter) = generated_argument_letter_base(sumti) {
        keys.push(base_letter);
    }
    if let Some(initials) = generated_argument_name_initials(sumti)
        && !keys.iter().any(|key| key == &initials)
    {
        keys.push(initials);
    }
    keys
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
fn generated_argument_name_initials(sumti: &generated::SumtiSyntax) -> Option<String> {
    let simple = generated_simple_sumti_from_sumti(sumti)?;
    let generated::SumtiAtomSyntax::SumtiBase(base) = simple.base_sumti.as_ref() else {
        return None;
    };
    let generated::SumtiBaseSyntax::NameSumti(name) = base else {
        return None;
    };
    generated_word_run_initial_key(&name.names.value)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
fn generated_argument_letter_base(sumti: &generated::SumtiSyntax) -> Option<String> {
    let simple = generated_simple_sumti_from_sumti(sumti)?;
    match simple.base_sumti.as_ref() {
        generated::SumtiAtomSyntax::SumtiBase(base) => {
            generated_argument_letter_base_from_sumti_base(base)
        }
        generated::SumtiAtomSyntax::QuantifiedSumti(quantified) => {
            generated_argument_letter_base_from_sumti_base(&quantified.inner_sumti)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
fn generated_argument_letter_base_from_sumti_base(
    sumti: &generated::SumtiBaseSyntax,
) -> Option<String> {
    match sumti {
        generated::SumtiBaseSyntax::DescriptorWithGadriSumti(description) => {
            generated_description_tail_base_letter(&description.tail)
        }
        generated::SumtiBaseSyntax::DescriptorWithOuterQuantifierSumti(description) => {
            generated_description_tail_base_letter(&description.tail)
        }
        generated::SumtiBaseSyntax::DescriptionConnectionSumti(description) => {
            generated_description_tail_base_letter(&description.tail)
        }
        generated::SumtiBaseSyntax::DescriptorWithoutGadriSumti(description) => {
            generated_selbri_base_letter(&description.selbri)
        }
        generated::SumtiBaseSyntax::NameSumti(name) => {
            generated_token_base_letter(Some(name.names.value.first()))
        }
        generated::SumtiBaseSyntax::LaheSumti(sumti) => {
            generated_argument_letter_base(&sumti.inner_sumti)
        }
        generated::SumtiBaseSyntax::ScalarNegatedSumti(sumti) => {
            generated_argument_letter_base(&sumti.inner_sumti)
        }
        generated::SumtiBaseSyntax::ScalarNegatedSumtiWithBo(sumti) => {
            generated_argument_letter_base(&sumti.inner_sumti)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
fn generated_description_tail_base_letter(
    tail: &generated::DescriptionTailSyntax,
) -> Option<String> {
    if let Some(tail_sumti) = &tail.leading_tail_elements.tail_sumti
        && let Some(letter) = generated_argument_letter_base_from_sumti_base(tail_sumti.0.as_ref())
    {
        return Some(letter);
    }
    match tail.tail.as_ref() {
        generated::DescriptionTailBodySyntax::RelationDescriptionTail(tail) => {
            generated_selbri_base_letter(&tail.selbri)
        }
        generated::DescriptionTailBodySyntax::QuantifierRelationDescriptionTail(tail) => {
            generated_selbri_base_letter(&tail.selbri)
        }
        generated::DescriptionTailBodySyntax::QuantifierSumtiDescriptionTail(tail) => {
            generated_argument_letter_base(&tail.sumti)
        }
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
fn generated_selbri_base_letter(selbri: &generated::SelbriSyntax) -> Option<String> {
    generated_relation_first_token(selbri)
        .and_then(|token| generated_token_base_letter(Some(token)))
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
fn generated_letter_string_initial_key(letters: &generated::LetterStringSyntax) -> Option<String> {
    let tokens = generated_letter_string_tokens(letters);
    generated_token_run_initial_key(&tokens)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
fn generated_word_run_initial_key(tokens: &[Token]) -> Option<String> {
    if tokens.len() <= 1 {
        return None;
    }
    generated_token_run_initial_key(tokens)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
fn generated_token_run_initial_key(tokens: &[Token]) -> Option<String> {
    let initials = tokens
        .iter()
        .map(|token| generated_token_base_letter(Some(token)))
        .collect::<Option<Vec<_>>>()?
        .join("");
    (!initials.is_empty()).then_some(initials)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|key| !key.is_empty()))]
fn generated_token_base_letter(token: Option<&Token>) -> Option<String> {
    token.and_then(token_base_letter)
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn generated_letter_string_tokens(letters: &generated::LetterStringSyntax) -> Vec<Token> {
    let mut tokens = generated_letter_tokens(&letters.first_letter);
    for continuation in &letters.continuations {
        match continuation {
            generated::LetterStringContinuationSyntax::LetterStringPaContinuation(continuation) => {
                tokens.push(continuation.0.clone());
            }
            generated::LetterStringContinuationSyntax::LetterStringLerfuContinuation(
                continuation,
            ) => {
                tokens.extend(generated_letter_tokens(&continuation.0));
            }
        }
    }
    tokens
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn generated_letter_tokens(letter: &generated::LetterTokensSyntax) -> Vec<Token> {
    match letter {
        generated::LetterTokensSyntax::SimpleLerfuWord(word) => vec![word.0.clone()],
        generated::LetterTokensSyntax::LauLerfuWord(word) => {
            let mut tokens = vec![word.lau.clone()];
            tokens.extend(generated_letter_tokens(&word.letter));
            tokens
        }
        generated::LetterTokensSyntax::TeiLerfuWord(word) => {
            let mut tokens = vec![word.tei.clone()];
            tokens.extend(generated_letter_string_tokens(&word.letters));
            tokens.push(word.foi.clone());
            tokens
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_simple_sumti_from_sumti(
    sumti: &generated::SumtiSyntax,
) -> Option<&generated::SimpleSumtiSyntax> {
    if sumti.vuho_attachment.is_some() || sumti.base_sumti.grouped_tail.is_some() {
        return None;
    }
    let afterthought = sumti.base_sumti.leading_sumti.as_ref();
    if !afterthought.continuations.is_empty() || afterthought.leading_sumti.bound_tail.is_some() {
        return None;
    }
    let generated::SumtiForethoughtSyntax::SimpleSumti(simple) =
        afterthought.leading_sumti.leading_sumti.as_ref()
    else {
        return None;
    };
    Some(simple)
}

#[requires(true)]
#[ensures(true)]
fn generated_koha_assignable_cmavo_from_relative_sumti(
    sumti: &generated::RelativeSumtiSyntax,
) -> Option<Cmavo> {
    let cmavo = match sumti {
        generated::RelativeSumtiSyntax::PlainRelativeSumti(sumti) => {
            generated_argument_koha_cmavo_with_subscript(&sumti.0)?.0
        }
        generated::RelativeSumtiSyntax::TenseTaggedRelativeSumti(sumti) => {
            let generated::TaggedOrElidedSumtiSyntax::Sumti(sumti) = sumti.sumti.as_ref() else {
                return None;
            };
            generated_argument_koha_cmavo_with_subscript(sumti)?.0
        }
        generated::RelativeSumtiSyntax::NaKuRelativeSumti(_) => return None,
    };
    is_assignable_koha(cmavo).then_some(cmavo)
}

#[requires(true)]
#[ensures(true)]
fn generated_argument_koha_cmavo_from_index(
    index: &GeneratedSyntaxIndex<'_>,
    sumti: SumtiNodeId,
) -> Option<Cmavo> {
    let (cmavo, _subscript) = match index.node(sumti.0)? {
        GeneratedSyntaxNodeRef::SumtiSyntax(sumti) => {
            generated_argument_koha_cmavo_with_subscript(sumti)
        }
        GeneratedSyntaxNodeRef::SimpleSumtiSyntax(sumti) => {
            generated_simple_sumti_koha_cmavo_with_subscript(sumti)
        }
        _ => None,
    }?;
    is_assignable_koha(cmavo).then_some(cmavo)
}

#[requires(true)]
#[ensures(true)]
fn generated_relation_pro_bridi_cmavo(selbri: &generated::SelbriSyntax) -> Option<Cmavo> {
    let token = generated_relation_first_token(selbri)?;
    let cmavo = token.cmavo()?;
    matches!(cmavo, Cmavo::Buha | Cmavo::Buhe | Cmavo::Buhi).then_some(cmavo)
}

#[requires(true)]
#[ensures(true)]
fn generated_relation_first_token(selbri: &generated::SelbriSyntax) -> Option<&Token> {
    match selbri {
        generated::SelbriSyntax::TaggedSelbri(selbri) => {
            generated_untagged_relation_first_token(&selbri.inner_selbri)
        }
        generated::SelbriSyntax::UntaggedSelbri(selbri) => {
            generated_untagged_relation_first_token(selbri)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_untagged_relation_first_token(
    selbri: &generated::UntaggedSelbriSyntax,
) -> Option<&Token> {
    match selbri {
        generated::UntaggedSelbriSyntax::CoSelbri(selbri) => {
            generated_connected_selbri_first_token(&selbri.leading_selbri)
        }
        generated::UntaggedSelbriSyntax::NegatedSelbri(selbri) => {
            generated_relation_first_token(&selbri.inner_selbri)
        }
        generated::UntaggedSelbriSyntax::ForethoughtSelbriConnection(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_connected_selbri_first_token(
    selbri: &generated::ConnectedSelbriSyntax,
) -> Option<&Token> {
    generated_tanru_selbri_first_token(&selbri.leading_selbri)
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_selbri_first_token(selbri: &generated::TanruSelbriSyntax) -> Option<&Token> {
    generated_tanru_unit_first_token(&selbri.first_unit)
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_unit_first_token(unit: &generated::TanruUnitSyntax) -> Option<&Token> {
    generated_bo_or_linked_tanru_unit_first_token(&unit.0.first)
}

#[requires(true)]
#[ensures(true)]
fn generated_bo_or_linked_tanru_unit_first_token(
    unit: &generated::BoOrLinkedTanruUnitSyntax,
) -> Option<&Token> {
    match unit {
        generated::BoOrLinkedTanruUnitSyntax::LinkedTanruUnit(unit) => {
            generated_tanru_unit_atom_first_token(&unit.base)
        }
        generated::BoOrLinkedTanruUnitSyntax::AssignedProBridiTanruUnit(unit) => {
            generated_tanru_unit_atom_for_cei_first_token(&unit.base.base)
        }
        generated::BoOrLinkedTanruUnitSyntax::BoundTanruUnit(unit) => {
            generated_tanru_unit_atom_first_token(&unit.leading_unit.base)
        }
        generated::BoOrLinkedTanruUnitSyntax::ForethoughtSelbriGroupTanruUnit(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_unit_atom_first_token(unit: &generated::TanruUnitAtomSyntax) -> Option<&Token> {
    generated_tanru_unit_atom_base_first_token(&unit.base)
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_unit_atom_for_cei_first_token(
    unit: &generated::TanruUnitAtomForCeiSyntax,
) -> Option<&Token> {
    generated_tanru_unit_atom_base_for_cei_first_token(&unit.base)
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_unit_atom_base_first_token(
    unit: &generated::TanruUnitAtomBaseSyntax,
) -> Option<&Token> {
    match unit {
        generated::TanruUnitAtomBaseSyntax::WordTanruUnit(unit) => Some(&unit.0.value),
        generated::TanruUnitAtomBaseSyntax::GohaWordTanruUnit(unit) => Some(&unit.0.value),
        generated::TanruUnitAtomBaseSyntax::ProBridiTanruUnit(unit) => Some(&unit.goha.value),
        generated::TanruUnitAtomBaseSyntax::ScalarNegatedTanruUnit(unit) => {
            generated_scalar_negated_tanru_inner_unit_first_token(&unit.inner_unit)
        }
        generated::TanruUnitAtomBaseSyntax::GroupedTanruUnit(unit) => {
            generated_connected_selbri_first_token(&unit.selbri)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_tanru_unit_atom_base_for_cei_first_token(
    unit: &generated::TanruUnitAtomBaseForCeiSyntax,
) -> Option<&Token> {
    match unit {
        generated::TanruUnitAtomBaseForCeiSyntax::WordTanruUnit(unit) => Some(&unit.0.value),
        generated::TanruUnitAtomBaseForCeiSyntax::GohaWordTanruUnit(unit) => Some(&unit.0.value),
        generated::TanruUnitAtomBaseForCeiSyntax::ProBridiTanruUnit(unit) => Some(&unit.goha.value),
        generated::TanruUnitAtomBaseForCeiSyntax::ScalarNegatedTanruUnit(unit) => {
            generated_scalar_negated_tanru_inner_unit_first_token(&unit.inner_unit)
        }
        generated::TanruUnitAtomBaseForCeiSyntax::GroupedTanruUnit(unit) => {
            generated_connected_selbri_first_token(&unit.selbri)
        }
        _ => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_scalar_negated_tanru_inner_unit_first_token(
    unit: &generated::ScalarNegatedTanruInnerUnitSyntax,
) -> Option<&Token> {
    match unit {
        generated::ScalarNegatedTanruInnerUnitSyntax::TanruUnitAtom(unit) => {
            generated_tanru_unit_atom_base_first_token(&unit.base)
        }
        generated::ScalarNegatedTanruInnerUnitSyntax::ProBridiTanruUnit(unit) => {
            Some(&unit.goha.value)
        }
        generated::ScalarNegatedTanruInnerUnitSyntax::TaggedSelbriGroupTanruUnit(unit) => {
            generated_connected_selbri_first_token(&unit.inner_selbri)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_relation_unit_assignment_label(
    unit: &generated::LinkedTanruUnitForCeiSyntax,
) -> Option<CeiLabel> {
    generated_tanru_unit_atom_for_cei_first_token(&unit.base).and_then(|token| {
        CeiLabel::from_broda_word_like(token.core_word())
            .or_else(|| CeiLabel::from_buha_cmavo(token.cmavo()?))
    })
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
fn token_base_letter(word: &Token) -> Option<String> {
    word_like_base_letter(word.as_ref().core_word())
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|letter| !letter.is_empty()))]
fn word_like_base_letter(word_like: &WordLike) -> Option<String> {
    match word_like.as_data() {
        data!(WordLike::PlainWord(word)) => word_phoneme_base_letter(word),
        data!(WordLike::LerfuWord { base, .. }) => word_like_base_letter(base),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|letter| !letter.is_empty()))]
fn word_phoneme_base_letter(word: &Word) -> Option<String> {
    let text = word.canonical_phonemes();
    text.chars()
        .find(|character| character.is_alphabetic())
        .map(|character| character.to_string())
}

#[requires(true)]
#[ensures(true)]
fn cmavo_is_relative_phrase_marker(cmavo: Cmavo) -> bool {
    matches!(
        cmavo,
        Cmavo::Pe | Cmavo::Po | Cmavo::Pohe | Cmavo::Pohu | Cmavo::Ne | Cmavo::Nohu
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused_imports)]
    use bityzba::{ensures, requires};
    use jbotci_morphology::segment_words_with_modifiers;
    use jbotci_syntax::{ParseOptions, parse_syntax_tree_generated_model_with_source_and_options};

    #[requires(true)]
    #[ensures(true)]
    fn run_reference_test(test: impl FnOnce()) {
        test();
    }

    #[requires(true)]
    #[ensures(true)]
    fn parse_generated_syntax(input: &str) -> Box<GeneratedTextSyntax> {
        let words = segment_words_with_modifiers(input).expect("morphology succeeds");
        parse_syntax_tree_generated_model_with_source_and_options(
            &words,
            input,
            &ParseOptions::default(),
        )
        .expect("generated syntax succeeds")
    }

    #[requires(true)]
    #[ensures(ret.offset == offset && ret.length == length)]
    fn span_key(offset: usize, length: usize) -> FixtureSpanKey {
        FixtureSpanKey { offset, length }
    }

    #[requires(!needle.is_empty())]
    #[ensures(ret.length == needle.len())]
    fn nth_span_key(input: &str, needle: &str, occurrence: usize) -> FixtureSpanKey {
        let offset = input
            .match_indices(needle)
            .nth(occurrence)
            .map(|(offset, _)| offset)
            .expect("test input contains requested occurrence");
        span_key(offset, needle.len())
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_resolved_node(target: &FixtureReferenceTarget, expected: FixtureSpanKey) {
        assert!(
            matches!(target, FixtureReferenceTarget::ResolvedNode { node } if *node == expected),
            "expected resolved target {expected:?}, got {target:?}"
        );
    }

    #[requires(true)]
    #[ensures(true)]
    fn assert_unresolved(target: &FixtureReferenceTarget) {
        assert!(
            matches!(target, FixtureReferenceTarget::Unresolved { .. }),
            "expected unresolved target, got {target:?}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn fixture_span_key_from_syntax_span_rejects_zero_width_spans() {
        let zero_width = new!(SyntaxSpanKey {
            source_id: None,
            byte_start: 4,
            byte_end: 4,
            char_start: 4,
            char_end: 4,
        });
        let normal = new!(SyntaxSpanKey {
            source_id: None,
            byte_start: 4,
            byte_end: 9,
            char_start: 4,
            char_end: 9,
        });

        assert_eq!(fixture_span_key_from_syntax_span(&zero_width), None);
        assert_eq!(
            fixture_span_key_from_syntax_span(&normal),
            Some(span_key(4, 5))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_syntax_index_records_root_and_ordered_spans() {
        run_reference_test(|| {
            let syntax = parse_generated_syntax("mi tavla do");
            let index = GeneratedSyntaxIndex::new(&syntax).expect("generated index succeeds");
            assert!(index.node_count() > 0);
            assert_eq!(index.text_node_id(&syntax), Some(index.root()));
            let root = index
                .metadata(index.root().0)
                .expect("root metadata is present");
            assert_eq!(
                root.first_source_span
                    .as_ref()
                    .map(|span| (span.byte_start, span.byte_end)),
                Some((0, 2))
            );
            assert_eq!(
                root.last_source_span
                    .as_ref()
                    .map(|span| (span.byte_start, span.byte_end)),
                Some((9, 11))
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_reference_projection_records_basic_places() {
        run_reference_test(|| {
            let syntax = parse_generated_syntax("mi klama do");
            let analysis =
                analyze_generated_references(&syntax).expect("reference analysis succeeds");
            let projection = analysis.fixture_projection();

            assert!(!projection.frames.is_empty());
            assert!(projection.assignments.iter().any(|assignment| {
                assignment.sumti == span_key(0, 2)
                    && matches!(assignment.slot, FixturePlaceSlot::Numbered { place: 1 })
            }));
            assert!(projection.assignments.iter().any(|assignment| {
                assignment.sumti == span_key(9, 2)
                    && matches!(assignment.slot, FixturePlaceSlot::Numbered { place: 2 })
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_prenex_da_scope_stops_at_bare_i_boundary() {
        run_reference_test(|| {
            let syntax = parse_generated_syntax("su'oda zo'u mi prami da .i naku do prami da");
            let analysis =
                analyze_generated_references(&syntax).expect("reference analysis succeeds");
            let projection = analysis.fixture_projection();
            let da_edges = projection
                .references
                .iter()
                .filter(|edge| edge.kind == ReferenceKind::DaSeries)
                .collect::<Vec<_>>();

            assert_eq!(da_edges.len(), 1);
            assert_eq!(da_edges[0].source, span_key(21, 2));
            assert!(matches!(
                &da_edges[0].target,
                FixtureReferenceTarget::ResolvedNode { node } if *node == span_key(4, 2)
            ));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_se_conversion_chains_compose_all_conversions() {
        run_reference_test(|| {
            let input = "mi se te klama do";
            let syntax = parse_generated_syntax(input);
            let analysis =
                analyze_generated_references(&syntax).expect("reference analysis succeeds");
            let projection = analysis.fixture_projection();
            let klama = nth_span_key(input, "klama", 0);
            let mi = nth_span_key(input, "mi", 0);
            let do_sumti = nth_span_key(input, "do", 0);

            assert!(projection.assignments.iter().any(|assignment| {
                assignment.frame_node == klama
                    && assignment.sumti == mi
                    && matches!(assignment.slot, FixturePlaceSlot::Numbered { place: 2 })
            }));
            assert!(projection.assignments.iter().any(|assignment| {
                assignment.frame_node == klama
                    && assignment.sumti == do_sumti
                    && matches!(assignment.slot, FixturePlaceSlot::Numbered { place: 3 })
            }));
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_goi_can_assign_relative_clause_head_koha() {
        run_reference_test(|| {
            let input = "ko'a goi le broda cu klama .i ko'a cadzu";
            let syntax = parse_generated_syntax(input);
            let analysis =
                analyze_generated_references(&syntax).expect("reference analysis succeeds");
            let projection = analysis.fixture_projection();
            let later_koha = nth_span_key(input, "ko'a", 1);
            let description = nth_span_key(input, "le broda", 0);
            let koha_edge = projection
                .references
                .iter()
                .find(|edge| edge.kind == ReferenceKind::Koha && edge.source == later_koha)
                .expect("later ko'a resolves through GOI assignment");

            assert_resolved_node(&koha_edge.target, description);
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_cei_inside_grouped_tanru_unit_is_collected() {
        run_reference_test(|| {
            let input = "mi ke klama cei broda ke'e .i mi broda";
            let syntax = parse_generated_syntax(input);
            let analysis =
                analyze_generated_references(&syntax).expect("reference analysis succeeds");
            let projection = analysis.fixture_projection();
            let later_broda = nth_span_key(input, "broda", 1);
            let broda_edge = projection
                .references
                .iter()
                .find(|edge| edge.kind == ReferenceKind::BrodaSeries && edge.source == later_broda)
                .expect("later broda resolves through grouped CEI assignment");

            assert!(
                matches!(
                    broda_edge.target,
                    FixtureReferenceTarget::ResolvedNode { .. }
                ),
                "later broda should resolve through grouped CEI assignment"
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_cei_inside_quote_does_not_bind_outer_broda_series() {
        run_reference_test(|| {
            let input = "lu mi broda cei brode li'u zo'u mi brode";
            let syntax = parse_generated_syntax(input);
            let analysis =
                analyze_generated_references(&syntax).expect("reference analysis succeeds");
            let projection = analysis.fixture_projection();
            let outer_brode = nth_span_key(input, "brode", 1);

            assert!(
                !projection.references.iter().any(|edge| {
                    edge.kind == ReferenceKind::BrodaSeries && edge.source == outer_brode
                }),
                "CEI inside a quoted sumti must not bind the outer brode"
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_cei_inside_prenex_grouped_tanru_unit_is_collected() {
        run_reference_test(|| {
            let input = "lo ke broda cei brode ke'e ku zo'u mi brode";
            let syntax = parse_generated_syntax(input);
            let analysis =
                analyze_generated_references(&syntax).expect("reference analysis succeeds");
            let projection = analysis.fixture_projection();
            let later_brode = nth_span_key(input, "brode", 1);
            let brode_edge = projection
                .references
                .iter()
                .find(|edge| edge.kind == ReferenceKind::BrodaSeries && edge.source == later_brode)
                .expect("later brode resolves through grouped prenex CEI assignment");

            assert!(
                matches!(
                    brode_edge.target,
                    FixtureReferenceTarget::ResolvedNode { .. }
                ),
                "later brode should resolve through grouped prenex CEI assignment"
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_quote_history_does_not_leak_to_outer_dihu() {
        run_reference_test(|| {
            let input = "mi klama .i mi cusku lu do cadzu li'u .i di'u jitfa";
            let syntax = parse_generated_syntax(input);
            let analysis =
                analyze_generated_references(&syntax).expect("reference analysis succeeds");
            let projection = analysis.fixture_projection();
            let dihu = nth_span_key(input, "di'u", 0);
            let quote_statement = nth_span_key(input, "do cadzu", 0);
            let edge = projection
                .references
                .iter()
                .find(|edge| edge.kind == ReferenceKind::Utterance && edge.source == dihu)
                .expect("outer di'u has an utterance edge");

            assert!(
                !matches!(&edge.target, FixtureReferenceTarget::ResolvedNode { node } if *node == quote_statement),
                "outer di'u must not resolve to the quoted statement"
            );
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_quote_pending_dihe_does_not_resolve_to_outer_following_statement() {
        run_reference_test(|| {
            let input = "mi cusku lu di'e jitfa li'u .i do cadzu";
            let syntax = parse_generated_syntax(input);
            let analysis =
                analyze_generated_references(&syntax).expect("reference analysis succeeds");
            let projection = analysis.fixture_projection();
            let dihe = nth_span_key(input, "di'e", 0);
            let edge = projection
                .references
                .iter()
                .find(|edge| edge.kind == ReferenceKind::Utterance && edge.source == dihe)
                .expect("quoted di'e has an utterance edge");

            assert_unresolved(&edge.target);
        });
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn generated_fixture_projection_is_sorted_and_canonical_json() {
        run_reference_test(|| {
            let syntax = parse_generated_syntax("mi se klama do .i ri cadzu");
            let analysis =
                analyze_generated_references(&syntax).expect("reference analysis succeeds");
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
                    .selbri_places
                    .windows(2)
                    .all(|items| items[0] <= items[1])
            );
            assert!(
                projection
                    .references
                    .windows(2)
                    .all(|items| items[0] <= items[1])
            );
            assert!(!json.contains('\n'));
            assert!(json.contains("assignments"));
        });
    }
}
