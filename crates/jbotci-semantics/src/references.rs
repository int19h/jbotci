//! Borrowed semantic reference overlay for syntax trees.

use std::collections::{HashMap, HashSet};
use std::num::NonZeroU8;

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, requires};
use jbotci_morphology::{Cmavo, WordLike};
use jbotci_source::{SourceId, SourceSpan};
use jbotci_syntax::ast::{
    ArgumentSyntax, ArgumentSyntaxData, AtomRef as SyntaxAtomRef, BeiLinkSyntax,
    GoiRelativeClauseSyntax, NodeRef as SyntaxNodeRef, ParagraphSyntax, PredicateSyntax,
    PredicateTail1Syntax, PredicateTail2Syntax, PredicateTail3Syntax, PredicateTail3SyntaxData,
    PredicateTailSyntax, RelationSyntax, RelationSyntaxData, RelationUnitSyntax,
    RelationUnitSyntaxData, StatementSyntax, StatementSyntaxData, SubsentenceSyntax,
    SubsentenceSyntaxData, TermSyntax, TermSyntaxData, TextSyntax, TreeNode, WithFreeModifiers,
    WithIndicators,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[invariant(true)]
pub enum ReferenceKind {
    GoiAssignment,
    CeiAssignment,
    Koha,
    Ri,
    Ra,
    Ru,
    Keha,
    VohaSeries,
    DaSeries,
    BrodaSeries,
    GohaSeries,
    Utterance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        for paragraph in &text.paragraphs {
            self.analyze_paragraph(paragraph);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_paragraph(&mut self, paragraph: &'tree ParagraphSyntax) {
        for statement in &paragraph.statements {
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
                fragment.visit_in_order(&mut NoopReferenceVisitor);
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
        let tail = self.analyze_predicate_tail(&predicate.predicate_tail);
        let predicate_raw = self.raw_for(SyntaxNodeRef::PredicateSyntax(predicate));
        let predicate_frame = self.add_frame(
            predicate_raw,
            PlaceFrameKind::Predicate,
            None,
            None,
            propagation_connected(tail.frames),
        );
        let mut cursors = vec![PlaceCursor::new(predicate_frame)];
        self.assign_terms(
            &mut cursors,
            &predicate.leading_terms,
            AssignmentSource::SequentialTerm,
        );
        self.assign_term_refs(&mut cursors, &tail.terms, AssignmentSource::SequentialTerm);
        predicate_frame
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_predicate_tail(
        &mut self,
        tail: &'tree PredicateTailSyntax,
    ) -> PredicateTailAnalysis<'tree> {
        let first = self.analyze_predicate_tail1(&tail.first);
        let mut branches = first.frames;
        let mut terms = first.terms;
        if let Some(ke_continuation) = tail.ke_continuation.as_deref() {
            let continuation = self.analyze_predicate_tail(&ke_continuation.predicate_tail);
            branches.extend(continuation.frames);
            terms.extend(continuation.terms);
            terms.extend(ke_continuation.tail_terms.iter());
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
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_predicate_tail1(
        &mut self,
        tail: &'tree PredicateTail1Syntax,
    ) -> PredicateTailAnalysis<'tree> {
        let mut analysis = self.analyze_predicate_tail2(&tail.first);
        for continuation in &tail.continuations {
            let next = self.analyze_predicate_tail2(&continuation.predicate_tail);
            analysis.frames.extend(next.frames);
            analysis.terms.extend(next.terms);
            analysis.terms.extend(continuation.tail_terms.iter());
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
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_predicate_tail2(
        &mut self,
        tail: &'tree PredicateTail2Syntax,
    ) -> PredicateTailAnalysis<'tree> {
        let mut analysis = self.analyze_predicate_tail3(&tail.first);
        if let Some(continuation) = tail.bo_continuation.as_deref() {
            let next = self.analyze_predicate_tail2(&continuation.predicate_tail);
            analysis.frames.extend(next.frames);
            analysis.terms.extend(next.terms);
            analysis.terms.extend(continuation.tail_terms.iter());
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
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_predicate_tail3(
        &mut self,
        tail: &'tree PredicateTail3Syntax,
    ) -> PredicateTailAnalysis<'tree> {
        match tail.as_data() {
            data!(PredicateTail3Syntax::Relation {
                relation,
                terms,
                ..
            }) => {
                let relation_frame = self.analyze_relation(relation);
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
                }
            }
            data!(PredicateTail3Syntax::GekSentence(gek)) => {
                let frames = self.analyze_gek_sentence(gek);
                PredicateTailAnalysis {
                    frames,
                    terms: Vec::new(),
                }
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_gek_sentence(
        &mut self,
        gek: &'tree jbotci_syntax::ast::GekSentenceSyntax,
    ) -> Vec<SelbriPlaceFrameId> {
        match gek.as_data() {
            data!(jbotci_syntax::ast::GekSentenceSyntax::Pair {
                first,
                second,
                tail_terms,
                ..
            }) => {
                let first_frame = self.analyze_subsentence_frame(first);
                let second_frame = self.analyze_subsentence_frame(second);
                let mut cursors = vec![
                    PlaceCursor::new(first_frame),
                    PlaceCursor::new(second_frame),
                ];
                self.assign_terms(&mut cursors, tail_terms, AssignmentSource::SequentialTerm);
                vec![first_frame, second_frame]
            }
            data!(jbotci_syntax::ast::GekSentenceSyntax::Ke { inner, .. })
            | data!(jbotci_syntax::ast::GekSentenceSyntax::Na { inner, .. }) => {
                self.analyze_gek_sentence(inner)
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_subsentence_frame(
        &mut self,
        subsentence: &'tree SubsentenceSyntax,
    ) -> SelbriPlaceFrameId {
        match subsentence.as_data() {
            data!(SubsentenceSyntax::Plain(predicate)) => self.analyze_predicate(predicate),
            data!(SubsentenceSyntax::Prenex {
                prenex_terms,
                inner_subsentence,
                ..
            }) => {
                self.analyze_terms_nested(prenex_terms);
                self.analyze_subsentence_frame(inner_subsentence)
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
            data!(RelationSyntax::Na { inner_relation, .. })
            | data!(RelationSyntax::Ke {
                relation: inner_relation,
                ..
            })
            | data!(RelationSyntax::TenseModal { inner_relation, .. }) => {
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
            })
            | data!(RelationSyntax::Bo {
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
                self.analyze_subsentence(&abstraction.subsentence);
                self.add_frame(
                    relation_raw,
                    PlaceFrameKind::Abstraction,
                    relation_id,
                    None,
                    propagation_none(),
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
            | data!(RelationUnitSyntax::Moi { .. })
            | data!(RelationUnitSyntax::Nuha { .. })
            | data!(RelationUnitSyntax::Xohi { .. })
            | data!(RelationUnitSyntax::Me { .. })
            | data!(RelationUnitSyntax::Luhei { .. }) => self.add_frame(
                unit_raw,
                PlaceFrameKind::RelationUnit,
                None,
                unit_id,
                propagation_none(),
            ),
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
            data!(RelationUnitSyntax::Ke { relation, .. })
            | data!(RelationUnitSyntax::Wrapped(relation)) => {
                let inner = self.analyze_relation(relation);
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::Forwarding,
                    None,
                    unit_id,
                    propagation_forward(inner),
                )
            }
            data!(RelationUnitSyntax::Nahe { inner_unit, .. })
            | data!(RelationUnitSyntax::SelbriRelativeClause {
                base: inner_unit,
                ..
            })
            | data!(RelationUnitSyntax::Cei {
                base: inner_unit,
                ..
            }) => {
                let inner = self.analyze_relation_unit(inner_unit);
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::Forwarding,
                    None,
                    unit_id,
                    propagation_forward(inner),
                )
            }
            data!(RelationUnitSyntax::Jai { inner_unit, .. }) => {
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
                trailing_unit,
                ..
            })
            | data!(RelationUnitSyntax::Connected {
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
                    fa.as_deref(),
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
                self.analyze_subsentence(&abstraction.subsentence);
                self.add_frame(
                    unit_raw,
                    PlaceFrameKind::Abstraction,
                    None,
                    unit_id,
                    propagation_none(),
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
            data!(TermSyntax::NoihaAdverbial { relation, .. })
            | data!(TermSyntax::PoihaBrigahi { relation, .. }) => {
                if let Some(relation) = relation.as_deref() {
                    self.analyze_relation(relation);
                }
            }
            data!(TermSyntax::JaiTagged { argument, .. })
            | data!(TermSyntax::Tagged { argument, .. }) => {
                self.analyze_argument_nested(argument);
            }
            data!(TermSyntax::NaKu { .. }) | data!(TermSyntax::BareNa(..)) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn analyze_argument_nested(&mut self, argument: &'tree ArgumentSyntax) {
        match argument.as_data() {
            data!(ArgumentSyntax::Quantified { inner_argument, .. })
            | data!(ArgumentSyntax::Tagged { inner_argument, .. })
            | data!(ArgumentSyntax::NaheBo { inner_argument, .. })
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
                    relative_clause.visit_in_order(&mut NoopReferenceVisitor);
                }
            }
            data!(ArgumentSyntax::Vuho {
                base_argument,
                connected_argument,
                ..
            }) => {
                self.analyze_argument_nested(base_argument);
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
                if let Some(relation) = descriptor.relation.as_deref() {
                    self.analyze_relation(relation);
                }
            }
            data!(ArgumentSyntax::ConnectedDescriptor(descriptor)) => {
                if let Some(relation) = descriptor.relation.as_deref() {
                    self.analyze_relation(relation);
                }
            }
            data!(ArgumentSyntax::RelationVocative { relation, .. }) => {
                self.analyze_relation(relation);
            }
            data!(ArgumentSyntax::Quote(..))
            | data!(ArgumentSyntax::MathExpression { .. })
            | data!(ArgumentSyntax::Letter { .. })
            | data!(ArgumentSyntax::NaKu { .. })
            | data!(ArgumentSyntax::Koha(..))
            | data!(ArgumentSyntax::Zohe { .. })
            | data!(ArgumentSyntax::Name { .. })
            | data!(ArgumentSyntax::Cmevla(..)) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn assign_terms(
        &mut self,
        cursors: &mut [PlaceCursor],
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
        cursors: &mut [PlaceCursor],
        terms: &[&'tree TermSyntax],
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
        cursors: &mut [PlaceCursor],
        term: &'tree TermSyntax,
        source: AssignmentSource,
    ) {
        match term.as_data() {
            data!(TermSyntax::Argument(argument)) => {
                self.assign_argument_to_cursors(cursors, term, argument, None, source);
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
            data!(TermSyntax::JaiTagged { argument, .. }) => {
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
                self.assign_terms(cursors, terms, AssignmentSource::TermsetBranch);
                self.assign_terms(cursors, gik_terms, AssignmentSource::TermsetBranch);
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
    fn assign_argument_to_cursors(
        &mut self,
        cursors: &mut [PlaceCursor],
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
        fa: Option<&'tree WithFreeModifiers<WithIndicators<WordLike>>>,
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
                let slot = link.fa.as_deref().and_then(fa_place_slot);
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
                    leading,
                    slot,
                    argument,
                    term,
                    AssignmentSource::Propagated,
                    visited,
                );
                if slot.numbered_index() == Some(1) {
                    self.add_assignment_recursive(
                        trailing,
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
}

#[derive(Debug, Default)]
#[invariant(true)]
struct NoopReferenceVisitor;

impl<'tree> TreeVisitor<'tree> for NoopReferenceVisitor {
    type Node = SyntaxNodeRef<'tree>;
    type Atom = SyntaxAtomRef<'tree>;
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
            SyntaxAtomRef::WithIndicatorsWordLike(word) => {
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
struct DiscourseReferenceBuilder<'index, 'tree> {
    index: &'index SyntaxIndex<'tree>,
    places: &'index PlaceAnalysis,
    edges: Vec<ReferenceEdge>,
    edge_ids_by_source: HashMap<RawSyntaxNodeId, Vec<ReferenceEdgeId>>,
    edge_ids_by_target_node: HashMap<RawSyntaxNodeId, Vec<ReferenceEdgeId>>,
    koha_bindings: HashMap<Cmavo, ArgumentNodeId>,
    cei_bindings: HashMap<String, RelationUnitNodeId>,
    da_bindings: HashMap<Cmavo, ArgumentNodeId>,
    last_argument: Option<ArgumentNodeId>,
    last_predicate: Option<PredicateNodeId>,
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
            cei_bindings: HashMap::new(),
            da_bindings: HashMap::new(),
            last_argument: None,
            last_predicate: None,
            current_predicate_frames: Vec::new(),
            relative_heads: Vec::new(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> DiscourseReferences {
        DiscourseReferences {
            edges: self.edges,
            edge_ids_by_source: self.edge_ids_by_source,
            edge_ids_by_target_node: self.edge_ids_by_target_node,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_text(&mut self, text: &'tree TextSyntax) {
        for paragraph in &text.paragraphs {
            self.visit_paragraph(paragraph);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_paragraph(&mut self, paragraph: &'tree ParagraphSyntax) {
        for statement in &paragraph.statements {
            if let Some(statement) = statement.statement.as_deref() {
                self.visit_statement(statement);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_statement(&mut self, statement: &'tree StatementSyntax) {
        match statement.as_data() {
            data!(StatementSyntax::Tuhe { text, .. }) => self.visit_text(text),
            data!(StatementSyntax::Prenex {
                prenex_terms,
                inner_statement,
                ..
            }) => {
                self.visit_terms(prenex_terms);
                self.visit_statement(inner_statement);
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
                self.visit_subsentence(&continuation.trailing_subsentence);
            }
            data!(StatementSyntax::Fragment(_)) => {}
        }
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
                self.visit_subsentence(inner_subsentence);
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
        self.visit_terms(&predicate.leading_terms);
        self.visit_predicate_tail(&predicate.predicate_tail);
        self.current_predicate_frames = previous_frames;
        self.last_predicate = Some(predicate_id);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_predicate_tail(&mut self, tail: &'tree PredicateTailSyntax) {
        self.visit_predicate_tail1(&tail.first);
        if let Some(continuation) = tail.ke_continuation.as_deref() {
            self.visit_predicate_tail(&continuation.predicate_tail);
            self.visit_terms(&continuation.tail_terms);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_predicate_tail1(&mut self, tail: &'tree PredicateTail1Syntax) {
        self.visit_predicate_tail2(&tail.first);
        for continuation in &tail.continuations {
            self.visit_predicate_tail2(&continuation.predicate_tail);
            self.visit_terms(&continuation.tail_terms);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_predicate_tail2(&mut self, tail: &'tree PredicateTail2Syntax) {
        self.visit_predicate_tail3(&tail.first);
        if let Some(continuation) = tail.bo_continuation.as_deref() {
            self.visit_predicate_tail2(&continuation.predicate_tail);
            self.visit_terms(&continuation.tail_terms);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_predicate_tail3(&mut self, tail: &'tree PredicateTail3Syntax) {
        match tail.as_data() {
            data!(PredicateTail3Syntax::Relation {
                relation,
                terms,
                ..
            }) => {
                self.visit_relation(relation);
                self.visit_terms(terms);
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
                ..
            }) => {
                self.visit_subsentence(first);
                self.visit_subsentence(second);
                self.visit_terms(tail_terms);
            }
            data!(jbotci_syntax::ast::GekSentenceSyntax::Ke { inner, .. })
            | data!(jbotci_syntax::ast::GekSentenceSyntax::Na { inner, .. }) => {
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
    fn visit_term(&mut self, term: &'tree TermSyntax) {
        match term.as_data() {
            data!(TermSyntax::Argument(argument))
            | data!(TermSyntax::Fa { argument, .. })
            | data!(TermSyntax::Tagged { argument, .. })
            | data!(TermSyntax::JaiTagged { argument, .. }) => self.visit_argument(argument),
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
            data!(TermSyntax::NoihaAdverbial { relation, .. })
            | data!(TermSyntax::PoihaBrigahi { relation, .. }) => {
                if let Some(relation) = relation.as_deref() {
                    self.visit_relation(relation);
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
                self.resolve_koha(argument_id, koha.cmavo());
                if !koha.cmavo().is_some_and(is_skipped_for_ri) {
                    self.last_argument = Some(argument_id);
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
                self.relative_heads.push(base_id);
                for relative_clause in relative_clauses {
                    self.visit_relative_clause(base_id, relative_clause);
                }
                self.relative_heads.pop();
                self.last_argument = Some(argument_id);
            }
            data!(ArgumentSyntax::Vuho {
                base_argument,
                connected_argument,
                ..
            }) => {
                self.visit_argument(base_argument);
                if let Some(connected) = connected_argument.as_deref() {
                    self.visit_argument(&connected.argument);
                }
                self.last_argument = Some(argument_id);
            }
            data!(ArgumentSyntax::Quantified { inner_argument, .. })
            | data!(ArgumentSyntax::Tagged { inner_argument, .. })
            | data!(ArgumentSyntax::NaheBo { inner_argument, .. })
            | data!(ArgumentSyntax::Nahe { inner_argument, .. })
            | data!(ArgumentSyntax::Lahe { inner_argument, .. })
            | data!(ArgumentSyntax::Ke { inner_argument, .. }) => {
                self.visit_argument(inner_argument);
                self.last_argument = Some(argument_id);
            }
            data!(ArgumentSyntax::BridiDescription { subsentence, .. }) => {
                self.visit_subsentence(subsentence);
                self.last_argument = Some(argument_id);
            }
            data!(ArgumentSyntax::TermWrapped { inner_term, .. }) => {
                self.visit_term(inner_term);
                self.last_argument = Some(argument_id);
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
                self.last_argument = Some(argument_id);
            }
            data!(ArgumentSyntax::Descriptor(descriptor)) => {
                if let Some(relation) = descriptor.relation.as_deref() {
                    self.visit_relation(relation);
                }
                for relative_clause in &descriptor.relative_clauses {
                    self.visit_relative_clause(argument_id, relative_clause);
                }
                self.last_argument = Some(argument_id);
            }
            data!(ArgumentSyntax::ConnectedDescriptor(descriptor)) => {
                if let Some(relation) = descriptor.relation.as_deref() {
                    self.visit_relation(relation);
                }
                for relative_clause in &descriptor.relative_clauses {
                    self.visit_relative_clause(argument_id, relative_clause);
                }
                self.last_argument = Some(argument_id);
            }
            data!(ArgumentSyntax::RelationVocative { relation, .. }) => {
                self.visit_relation(relation);
                self.last_argument = Some(argument_id);
            }
            data!(ArgumentSyntax::Quote(..))
            | data!(ArgumentSyntax::MathExpression { .. })
            | data!(ArgumentSyntax::Letter { .. })
            | data!(ArgumentSyntax::NaKu { .. })
            | data!(ArgumentSyntax::Zohe { .. })
            | data!(ArgumentSyntax::Name { .. })
            | data!(ArgumentSyntax::Cmevla(..)) => {
                self.last_argument = Some(argument_id);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_relative_clause(
        &mut self,
        base_id: ArgumentNodeId,
        relative_clause: &'tree jbotci_syntax::ast::RelativeClauseSyntax,
    ) {
        match relative_clause.as_data() {
            data!(jbotci_syntax::ast::RelativeClauseSyntax::Goi(goi)) => {
                self.visit_goi_clause(base_id, goi);
            }
            data!(jbotci_syntax::ast::RelativeClauseSyntax::Noi { subsentence, .. })
            | data!(jbotci_syntax::ast::RelativeClauseSyntax::Poi { subsentence, .. }) => {
                self.visit_subsentence(subsentence);
            }
            data!(jbotci_syntax::ast::RelativeClauseSyntax::Zihe { inner, .. })
            | data!(jbotci_syntax::ast::RelativeClauseSyntax::Connected { inner, .. }) => {
                self.visit_relative_clause(base_id, inner);
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
        }
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
            })
            | data!(RelationSyntax::Bo {
                leading_relation,
                trailing_relation,
                ..
            }) => {
                self.visit_relation(leading_relation);
                self.visit_relation(trailing_relation);
            }
            data!(RelationSyntax::Na { inner_relation, .. })
            | data!(RelationSyntax::Se { inner_relation, .. })
            | data!(RelationSyntax::Ke {
                relation: inner_relation,
                ..
            })
            | data!(RelationSyntax::TenseModal { inner_relation, .. }) => {
                self.visit_relation(inner_relation);
            }
            data!(RelationSyntax::Base(word)) => {
                if let Some(label) = broda_label(word.core_word()) {
                    self.resolve_broda_relation(relation, label);
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
                self.visit_subsentence(&abstraction.subsentence);
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
            data!(RelationUnitSyntax::Ke { relation, .. })
            | data!(RelationUnitSyntax::Wrapped(relation)) => self.visit_relation(relation),
            data!(RelationUnitSyntax::Bo {
                leading_unit,
                trailing_unit,
                ..
            })
            | data!(RelationUnitSyntax::Connected {
                leading_unit,
                trailing_unit,
                ..
            }) => {
                self.visit_relation_unit(leading_unit);
                self.visit_relation_unit(trailing_unit);
            }
            data!(RelationUnitSyntax::SelbriRelativeClause { base, .. }) => {
                self.visit_relation_unit(base);
            }
            data!(RelationUnitSyntax::Jai { inner_unit, .. }) => {
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
                self.visit_subsentence(&abstraction.subsentence);
            }
            data!(RelationUnitSyntax::Me { argument, .. }) => self.visit_argument(argument),
            data!(RelationUnitSyntax::Luhei { text, .. }) => self.visit_text(text),
            data!(RelationUnitSyntax::Cei { base, assignments }) => {
                self.visit_relation_unit(base);
                let base_id = self
                    .index
                    .relation_unit_node_id(base)
                    .expect("CEI base belongs to indexed syntax tree");
                for assignment in assignments {
                    self.visit_relation_unit(&assignment.relation_unit);
                    if let Some(label) = relation_unit_assignment_label(&assignment.relation_unit) {
                        self.cei_bindings.insert(label, base_id);
                    }
                    let assignment_id = self
                        .index
                        .relation_unit_node_id(&assignment.relation_unit)
                        .expect("CEI assignment belongs to indexed syntax tree");
                    self.add_edge(
                        ReferenceKind::CeiAssignment,
                        assignment_id.0,
                        target_resolved_node(base_id.0),
                        "CEI assigns a pro-selbri/relation word to the preceding relation unit",
                    );
                }
            }
            data!(RelationUnitSyntax::Mehoi(..))
            | data!(RelationUnitSyntax::Gohoi(..))
            | data!(RelationUnitSyntax::Muhoi(..))
            | data!(RelationUnitSyntax::Moi { .. })
            | data!(RelationUnitSyntax::Nuha { .. })
            | data!(RelationUnitSyntax::Xohi { .. }) => {}
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn resolve_koha(&mut self, source: ArgumentNodeId, cmavo: Option<Cmavo>) {
        let Some(cmavo) = cmavo else {
            return;
        };
        match cmavo {
            Cmavo::Ri => {
                let target = self
                    .last_argument
                    .map(|argument| target_resolved_node(argument.0))
                    .unwrap_or_else(|| target_unresolved("ri has no prior sumti"));
                self.add_edge(
                    ReferenceKind::Ri,
                    source.0,
                    target,
                    "ri repeats the previous complete sumti",
                );
            }
            Cmavo::Ra => self.add_edge(
                ReferenceKind::Ra,
                source.0,
                target_vague(VagueReferenceKind::DistantArgument),
                "ra is intentionally vague and is not resolved heuristically",
            ),
            Cmavo::Ru => self.add_edge(
                ReferenceKind::Ru,
                source.0,
                target_vague(VagueReferenceKind::DistantArgument),
                "ru is intentionally vague and is not resolved heuristically",
            ),
            Cmavo::Keha => {
                let target = self
                    .relative_heads
                    .last()
                    .copied()
                    .map(|argument| target_resolved_node(argument.0))
                    .unwrap_or_else(|| target_unresolved("ke'a is outside a relative clause"));
                self.add_edge(
                    ReferenceKind::Keha,
                    source.0,
                    target,
                    "ke'a refers to the current relative-clause head",
                );
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
            }
            Cmavo::Da | Cmavo::De | Cmavo::Di => {
                if let Some(target) = self.da_bindings.get(&cmavo).copied() {
                    self.add_edge(
                        ReferenceKind::DaSeries,
                        source.0,
                        target_resolved_node(target.0),
                        "later da/de/di mentions refer to the active variable binding",
                    );
                } else {
                    self.da_bindings.insert(cmavo, source);
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
        match cmavo {
            Cmavo::Gohi => {
                let target = self
                    .last_predicate
                    .map(|predicate| target_resolved_node(predicate.0))
                    .unwrap_or_else(|| target_unresolved("go'i has no prior bridi"));
                self.add_edge(
                    ReferenceKind::GohaSeries,
                    source.0,
                    target,
                    "go'i repeats the previous bridi",
                );
            }
            Cmavo::Goha | Cmavo::Gohu | Cmavo::Gohe | Cmavo::Goho => {
                self.add_edge(
                    ReferenceKind::GohaSeries,
                    source.0,
                    target_vague(VagueReferenceKind::Bridi),
                    "this GOhA form is context-sensitive and is not resolved heuristically",
                );
            }
            Cmavo::Nei => {
                let target = self
                    .current_predicate_frames
                    .first()
                    .copied()
                    .map(target_resolved_frame)
                    .unwrap_or_else(|| target_unresolved("nei is outside a current bridi"));
                self.add_edge(
                    ReferenceKind::GohaSeries,
                    source.0,
                    target,
                    "nei refers to the current bridi",
                );
            }
            Cmavo::Noha => {
                self.add_edge(
                    ReferenceKind::GohaSeries,
                    source.0,
                    target_unresolved("no'a outer-bridi stack is not represented yet"),
                    "no'a refers to an outer bridi",
                );
            }
            Cmavo::Buha | Cmavo::Buhe | Cmavo::Buhi => {
                let label = cmavo.canonical_text().to_owned();
                if let Some(target) = self.cei_bindings.get(&label).copied() {
                    self.add_edge(
                        ReferenceKind::BrodaSeries,
                        source.0,
                        target_resolved_node(target.0),
                        "CEI binding resolves this pro-relation word",
                    );
                }
            }
            _ => {}
        }
    }

    #[requires(!label.is_empty())]
    #[ensures(true)]
    fn resolve_broda_unit(&mut self, unit: &'tree RelationUnitSyntax, label: String) {
        let Some(target) = self.cei_bindings.get(&label).copied() else {
            return;
        };
        let source = self
            .index
            .relation_unit_node_id(unit)
            .expect("broda unit belongs to indexed syntax tree");
        self.add_edge(
            ReferenceKind::BrodaSeries,
            source.0,
            target_resolved_node(target.0),
            "CEI binding resolves this broda-series relation unit",
        );
    }

    #[requires(!label.is_empty())]
    #[ensures(true)]
    fn resolve_broda_relation(&mut self, relation: &'tree RelationSyntax, label: String) {
        let Some(target) = self.cei_bindings.get(&label).copied() else {
            return;
        };
        let source = self
            .index
            .relation_node_id(relation)
            .expect("broda relation belongs to indexed syntax tree");
        self.add_edge(
            ReferenceKind::BrodaSeries,
            source.0,
            target_resolved_node(target.0),
            "CEI binding resolves this broda-series relation",
        );
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
fn se_conversion_place(se: &WithFreeModifiers<WithIndicators<WordLike>>) -> Option<u8> {
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
fn fa_place_slot(fa: &WithFreeModifiers<WithIndicators<WordLike>>) -> Option<PlaceSlot> {
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
fn is_skipped_for_ri(cmavo: Cmavo) -> bool {
    matches!(
        cmavo,
        Cmavo::Ri
            | Cmavo::Ra
            | Cmavo::Ru
            | Cmavo::Koha
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
fn argument_koha_cmavo(argument: &ArgumentSyntax) -> Option<Cmavo> {
    match argument.as_data() {
        data!(ArgumentSyntax::Koha(koha)) => koha.cmavo(),
        data!(ArgumentSyntax::Tagged { inner_argument, .. })
        | data!(ArgumentSyntax::Quantified { inner_argument, .. })
        | data!(ArgumentSyntax::NaheBo { inner_argument, .. })
        | data!(ArgumentSyntax::Nahe { inner_argument, .. })
        | data!(ArgumentSyntax::Lahe { inner_argument, .. })
        | data!(ArgumentSyntax::Ke { inner_argument, .. }) => argument_koha_cmavo(inner_argument),
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
    fn run_reference_test(test: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(test)
            .expect("test thread starts")
            .join()
            .expect("test thread completes");
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
            data!(ArgumentSyntax::Descriptor(descriptor)) => descriptor
                .relation
                .as_deref()
                .and_then(|relation| relation_label(relation)),
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
            let syntax = parse_syntax("mi klama .i ri cadzu");
            let analysis = analyze_references(&syntax).expect("reference analysis succeeds");

            assert!(analysis.discourse_references.edges().iter().any(|edge| {
                edge.kind == ReferenceKind::Ri
                    && matches!(edge.target, ReferenceTarget::ResolvedNode(_))
            }));
        });
    }
}
