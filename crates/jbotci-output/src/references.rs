//! Display model for semantic reference edges.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[allow(unused_imports)]
use bityzba::{contract_trait, data, ensures, invariant, new, requires};
use jbotci_morphology::canonicalize_text;
use jbotci_semantics::references::{
    DiscourseReferences, GeneratedReferenceAnalysis, GeneratedSyntaxIndex, PlaceAnalysis,
    PlaceFrameKind, PlaceSlot, RawSyntaxNodeId, ReferenceEdgeId, ReferenceKind, ReferenceTarget,
    SelbriPlaceFrame, SelbriPlaceFrameId, SumtiPlaceAssignment, SumtiPlaceAssignmentId,
    SyntaxNodeMetadata,
};
use jbotci_syntax::generated_model::{
    self as generated, AtomRef as GeneratedSyntaxAtomRef, NodeRef as GeneratedSyntaxNodeRef,
    SelbriSyntax as GeneratedSelbriSyntax, TreeNode as GeneratedSyntaxTreeNode,
};
use jbotci_syntax::tree::Token;
use jbotci_tree::TreeVisitor;

use crate::TreeRenderOptions;
use crate::tree::{TreeValue, morphology_tree_value};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[invariant(true)]
pub struct ReferenceName {
    pub stem: String,
    pub occurrence: Option<usize>,
    pub slot: Option<ReferenceSlotName>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[invariant(true)]
#[invariant(::Numbered(_) => true)]
#[invariant(::Modal(..) => true)]
#[invariant(::PlaceQuestion => true)]
#[invariant(::Fai => true)]
pub enum ReferenceSlotName {
    Numbered(u8),
    Modal(Vec<String>),
    PlaceQuestion,
    Fai,
}

impl ReferenceSlotName {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub fn text(&self) -> String {
        match self {
            Self::Numbered(place) => place.to_string(),
            Self::Modal(words) if words.is_empty() => "modal".to_owned(),
            Self::Modal(words) => words.join(" "),
            Self::PlaceQuestion => "place-question".to_owned(),
            Self::Fai => "fai".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[invariant(true)]
pub struct ReferenceAnnotations {
    pub incoming: Vec<ReferenceName>,
    pub outgoing: Vec<ReferenceName>,
}

#[invariant(self.incoming.iter().enumerate().all(|(index, annotation)| !self.incoming[..index].contains(annotation)))]
#[invariant(self.outgoing.iter().enumerate().all(|(index, annotation)| !self.outgoing[..index].contains(annotation)))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RichReferenceAnnotations {
    pub incoming: Vec<RichReferenceAnnotation>,
    pub outgoing: Vec<RichReferenceAnnotation>,
}

#[invariant(!self.name.stem.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RichReferenceAnnotation {
    pub name: ReferenceName,
    pub source: ReferenceAnnotationSource,
}

#[invariant(true)]
#[invariant(::PlaceFrame { display_word, lookup_word, .. } => !display_word.is_empty() && !lookup_word.is_empty())]
#[invariant(::PlaceAssignment { display_word, lookup_word, .. } => !display_word.is_empty() && !lookup_word.is_empty())]
#[invariant(::DiscourseEdge { display_word, lookup_word, .. } => !display_word.is_empty() && !lookup_word.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceAnnotationSource {
    PlaceFrame {
        frame: SelbriPlaceFrameId,
        source_node: RawSyntaxNodeId,
        display_word: String,
        lookup_word: String,
    },
    PlaceAssignment {
        frame: SelbriPlaceFrameId,
        assignment: SumtiPlaceAssignmentId,
        source_node: RawSyntaxNodeId,
        target_node: RawSyntaxNodeId,
        display_word: String,
        lookup_word: String,
    },
    DiscourseEdge {
        edge: ReferenceEdgeId,
        kind: ReferenceKind,
        source_node: RawSyntaxNodeId,
        target_node: RawSyntaxNodeId,
        display_word: String,
        lookup_word: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct ReferenceDisplayModel {
    incoming_by_node: BTreeMap<RawSyntaxNodeId, BTreeSet<ReferenceName>>,
    outgoing_by_node: BTreeMap<RawSyntaxNodeId, BTreeSet<ReferenceName>>,
    rich_incoming_by_node: BTreeMap<RawSyntaxNodeId, Vec<RichReferenceAnnotation>>,
    rich_outgoing_by_node: BTreeMap<RawSyntaxNodeId, Vec<RichReferenceAnnotation>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct TreeWordLabel {
    constructor: &'static str,
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
struct ReferenceSource {
    node: RawSyntaxNodeId,
    word: TreeWordLabel,
    pro_cmavo: bool,
    preorder: usize,
}

#[invariant(!self.name.stem.is_empty())]
#[invariant(!self.display_word.is_empty())]
#[invariant(!self.lookup_word.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReferenceSourceName {
    name: ReferenceName,
    display_word: String,
    lookup_word: String,
}

impl ReferenceDisplayModel {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn new_generated(
        analysis: &GeneratedReferenceAnalysis<'_>,
        tree: &TreeValue,
        source: &str,
        options: TreeRenderOptions,
    ) -> Self {
        Self::from_analysis_view(analysis, tree, source, options)
    }

    #[requires(true)]
    #[ensures(true)]
    fn from_analysis_view<'tree, A>(
        analysis: &A,
        tree: &TreeValue,
        source: &str,
        options: TreeRenderOptions,
    ) -> Self
    where
        A: ReferenceAnalysisView<'tree>,
    {
        let visible_words = visible_word_texts(tree);
        let sources = reference_sources(analysis, tree);
        let source_names = source_names(&sources, &visible_words);
        let mut model = Self {
            incoming_by_node: BTreeMap::new(),
            outgoing_by_node: BTreeMap::new(),
            rich_incoming_by_node: BTreeMap::new(),
            rich_outgoing_by_node: BTreeMap::new(),
        };
        model.add_place_annotations(analysis, source, options, &source_names);
        model.add_discourse_annotations(analysis, tree, &source_names);
        model
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn annotations_for_syntax_ids(
        &self,
        syntax_ids: &[RawSyntaxNodeId],
    ) -> ReferenceAnnotations {
        let mut incoming = BTreeSet::new();
        let mut outgoing = BTreeSet::new();
        for id in syntax_ids {
            if let Some(names) = self.incoming_by_node.get(id) {
                incoming.extend(names.iter().cloned());
            }
            if let Some(names) = self.outgoing_by_node.get(id) {
                outgoing.extend(names.iter().cloned());
            }
        }
        ReferenceAnnotations {
            incoming: incoming.into_iter().collect(),
            outgoing: outgoing.into_iter().collect(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn rich_annotations_for_syntax_ids(
        &self,
        syntax_ids: &[RawSyntaxNodeId],
    ) -> RichReferenceAnnotations {
        let mut incoming = Vec::new();
        let mut outgoing = Vec::new();
        for id in syntax_ids {
            if let Some(annotations) = self.rich_incoming_by_node.get(id) {
                extend_unique_rich_annotations(&mut incoming, annotations.iter().cloned());
            }
            if let Some(annotations) = self.rich_outgoing_by_node.get(id) {
                extend_unique_rich_annotations(&mut outgoing, annotations.iter().cloned());
            }
        }
        new!(RichReferenceAnnotations { incoming, outgoing })
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn translated_syntax_ids(
        &self,
        id_map: &HashMap<RawSyntaxNodeId, Vec<RawSyntaxNodeId>>,
    ) -> Self {
        Self {
            incoming_by_node: translate_reference_name_map(&self.incoming_by_node, id_map),
            outgoing_by_node: translate_reference_name_map(&self.outgoing_by_node, id_map),
            rich_incoming_by_node: translate_rich_reference_map(
                &self.rich_incoming_by_node,
                id_map,
            ),
            rich_outgoing_by_node: translate_rich_reference_map(
                &self.rich_outgoing_by_node,
                id_map,
            ),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_place_annotations<'tree, A>(
        &mut self,
        analysis: &A,
        source: &str,
        options: TreeRenderOptions,
        source_names: &HashMap<RawSyntaxNodeId, ReferenceSourceName>,
    ) where
        A: ReferenceAnalysisView<'tree>,
    {
        for frame in analysis.place_analysis().frames() {
            if analysis
                .place_analysis()
                .assignments_for_frame(frame.id)
                .is_empty()
            {
                continue;
            }
            let Some(source_node) = displayed_place_frame_source_from_names(frame, source_names)
            else {
                continue;
            };
            let Some(source_name) = source_names.get(&source_node).cloned() else {
                continue;
            };
            let base_name = source_name.name.clone();
            self.outgoing_by_node
                .entry(source_node)
                .or_default()
                .insert(base_name.clone());
            self.rich_outgoing_by_node
                .entry(source_node)
                .or_default()
                .push(new!(RichReferenceAnnotation {
                    name: base_name.clone(),
                    source: new!(ReferenceAnnotationSource::PlaceFrame {
                        frame: frame.id,
                        source_node,
                        display_word: source_name.display_word.clone(),
                        lookup_word: source_name.lookup_word.clone(),
                    }),
                }));
            for assignment_id in analysis.place_analysis().assignments_for_frame(frame.id) {
                let Some(assignment) = analysis.place_analysis().assignment(*assignment_id) else {
                    continue;
                };
                let mut place_name = base_name.clone();
                place_name.slot = Some(reference_slot_name_for_assignment(
                    analysis, assignment, source, options,
                ));
                self.incoming_by_node
                    .entry(assignment.sumti.0)
                    .or_default()
                    .insert(place_name.clone());
                self.rich_incoming_by_node
                    .entry(assignment.sumti.0)
                    .or_default()
                    .push(new!(RichReferenceAnnotation {
                        name: place_name,
                        source: new!(ReferenceAnnotationSource::PlaceAssignment {
                            frame: frame.id,
                            assignment: *assignment_id,
                            source_node,
                            target_node: assignment.sumti.0,
                            display_word: source_name.display_word.clone(),
                            lookup_word: source_name.lookup_word.clone(),
                        }),
                    }));
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_discourse_annotations<'tree, A>(
        &mut self,
        analysis: &A,
        tree: &TreeValue,
        source_names: &HashMap<RawSyntaxNodeId, ReferenceSourceName>,
    ) where
        A: ReferenceAnalysisView<'tree>,
    {
        for edge in analysis.discourse_references().edges() {
            let Some(target) = resolved_reference_target_node(analysis, &edge.target) else {
                continue;
            };
            if !contains_syntax_id(tree, target) {
                continue;
            }
            let Some(source_name) = source_names.get(&edge.source).cloned() else {
                continue;
            };
            let mut name = source_name.name.clone();
            name.slot = reference_slot_for_discourse_edge_kind(edge.kind.clone());
            self.outgoing_by_node
                .entry(edge.source)
                .or_default()
                .insert(name.clone());
            self.rich_outgoing_by_node
                .entry(edge.source)
                .or_default()
                .push(new!(RichReferenceAnnotation {
                    name: name.clone(),
                    source: new!(ReferenceAnnotationSource::DiscourseEdge {
                        edge: edge.id,
                        kind: edge.kind.clone(),
                        source_node: edge.source,
                        target_node: target,
                        display_word: source_name.display_word.clone(),
                        lookup_word: source_name.lookup_word.clone(),
                    }),
                }));
            self.incoming_by_node
                .entry(target)
                .or_default()
                .insert(name);
        }
    }
}

#[contract_trait]
trait ReferenceAnalysisView<'tree> {
    type Index: ReferenceIndexView<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn syntax_index(&self) -> &Self::Index;

    #[requires(true)]
    #[ensures(true)]
    fn place_analysis(&self) -> &PlaceAnalysis;

    #[requires(true)]
    #[ensures(true)]
    fn discourse_references(&self) -> &DiscourseReferences;
}

#[contract_trait]
impl<'tree> ReferenceAnalysisView<'tree> for GeneratedReferenceAnalysis<'tree> {
    type Index = GeneratedSyntaxIndex<'tree>;

    fn syntax_index(&self) -> &Self::Index {
        &self.syntax_index
    }

    fn place_analysis(&self) -> &PlaceAnalysis {
        &self.place_analysis
    }

    fn discourse_references(&self) -> &DiscourseReferences {
        &self.discourse_references
    }
}

#[contract_trait]
trait ReferenceIndexView<'tree> {
    #[requires(true)]
    #[ensures(true)]
    fn node_count(&self) -> usize;

    #[requires(true)]
    #[ensures(true)]
    fn metadata(&self, id: RawSyntaxNodeId) -> Option<&SyntaxNodeMetadata>;

    #[requires(true)]
    #[ensures(true)]
    fn termset_ancestor_for_term(&self, term: RawSyntaxNodeId) -> Option<RawSyntaxNodeId>;

    #[requires(true)]
    #[ensures(true)]
    fn sumti_is_omitted_placeholder(&self, sumti: RawSyntaxNodeId) -> bool;

    #[requires(true)]
    #[ensures(true)]
    fn modal_slot_words(
        &self,
        tag: RawSyntaxNodeId,
        source: &str,
        options: TreeRenderOptions,
    ) -> Vec<String>;

    #[requires(true)]
    #[ensures(true)]
    fn reference_slot_name_for_place_slot(
        &self,
        slot: PlaceSlot,
        source: &str,
        options: TreeRenderOptions,
    ) -> ReferenceSlotName {
        match slot {
            PlaceSlot::Numbered(place) => ReferenceSlotName::Numbered(place.get()),
            PlaceSlot::Modal(Some(tag)) => {
                ReferenceSlotName::Modal(self.modal_slot_words(tag, source, options))
            }
            PlaceSlot::Modal(None) => ReferenceSlotName::Modal(Vec::new()),
            PlaceSlot::PlaceQuestion => ReferenceSlotName::PlaceQuestion,
            PlaceSlot::Fai => ReferenceSlotName::Fai,
        }
    }
}

#[contract_trait]
impl<'tree> ReferenceIndexView<'tree> for GeneratedSyntaxIndex<'tree> {
    fn node_count(&self) -> usize {
        GeneratedSyntaxIndex::node_count(self)
    }

    fn metadata(&self, id: RawSyntaxNodeId) -> Option<&SyntaxNodeMetadata> {
        GeneratedSyntaxIndex::metadata(self, id)
    }

    fn termset_ancestor_for_term(&self, term: RawSyntaxNodeId) -> Option<RawSyntaxNodeId> {
        let mut current = Some(term);
        while let Some(node) = current {
            if self.node(node).is_some_and(|syntax| {
                matches!(
                    syntax,
                    GeneratedSyntaxNodeRef::TermSyntaxTermsetGroup(_)
                        | GeneratedSyntaxNodeRef::SimpleTermSyntaxNuhiTermset(_)
                        | GeneratedSyntaxNodeRef::SimpleTermSyntaxKeTermset(_)
                        | GeneratedSyntaxNodeRef::NuhiTermsetSyntax(_)
                        | GeneratedSyntaxNodeRef::KeTermsetSyntax(_)
                        | GeneratedSyntaxNodeRef::TermsetGroupSyntax(_)
                )
            }) {
                return Some(node);
            }
            current =
                GeneratedSyntaxIndex::metadata(self, node).and_then(|metadata| metadata.parent);
        }
        None
    }

    fn sumti_is_omitted_placeholder(&self, sumti: RawSyntaxNodeId) -> bool {
        self.node(sumti).is_some_and(|node| {
            matches!(
                node,
                GeneratedSyntaxNodeRef::TaggedElidedSumtiSyntax(_)
                    | GeneratedSyntaxNodeRef::TaggedOrElidedSumtiSyntaxTaggedElidedSumti(_)
            )
        })
    }

    fn modal_slot_words(
        &self,
        tag: RawSyntaxNodeId,
        source: &str,
        options: TreeRenderOptions,
    ) -> Vec<String> {
        generated_modal_slot_words(self, tag, source, options)
    }
}

#[requires(true)]
#[ensures(true)]
fn translate_reference_name_map(
    source: &BTreeMap<RawSyntaxNodeId, BTreeSet<ReferenceName>>,
    id_map: &HashMap<RawSyntaxNodeId, Vec<RawSyntaxNodeId>>,
) -> BTreeMap<RawSyntaxNodeId, BTreeSet<ReferenceName>> {
    let mut translated = BTreeMap::<RawSyntaxNodeId, BTreeSet<ReferenceName>>::new();
    for (source_id, names) in source {
        for target_id in translated_ids(*source_id, id_map) {
            translated
                .entry(target_id)
                .or_default()
                .extend(names.iter().cloned());
        }
    }
    translated
}

#[requires(true)]
#[ensures(true)]
fn translate_rich_reference_map(
    source: &BTreeMap<RawSyntaxNodeId, Vec<RichReferenceAnnotation>>,
    id_map: &HashMap<RawSyntaxNodeId, Vec<RawSyntaxNodeId>>,
) -> BTreeMap<RawSyntaxNodeId, Vec<RichReferenceAnnotation>> {
    let mut translated = BTreeMap::<RawSyntaxNodeId, Vec<RichReferenceAnnotation>>::new();
    for (source_id, annotations) in source {
        for target_id in translated_ids(*source_id, id_map) {
            let translated_annotations = annotations
                .iter()
                .cloned()
                .map(|annotation| translate_rich_reference_annotation(annotation, id_map));
            let entry = translated.entry(target_id).or_default();
            extend_unique_rich_annotations(entry, translated_annotations);
        }
    }
    translated
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn translated_ids(
    source_id: RawSyntaxNodeId,
    id_map: &HashMap<RawSyntaxNodeId, Vec<RawSyntaxNodeId>>,
) -> Vec<RawSyntaxNodeId> {
    id_map
        .get(&source_id)
        .cloned()
        .unwrap_or_else(|| vec![source_id])
}

#[requires(true)]
#[ensures(true)]
fn translate_rich_reference_annotation(
    annotation: RichReferenceAnnotation,
    id_map: &HashMap<RawSyntaxNodeId, Vec<RawSyntaxNodeId>>,
) -> RichReferenceAnnotation {
    let source = match annotation.source.as_data() {
        data!(ReferenceAnnotationSource::PlaceFrame {
            frame,
            source_node,
            display_word,
            lookup_word,
        }) => new!(ReferenceAnnotationSource::PlaceFrame {
            frame: *frame,
            source_node: first_translated_id(*source_node, id_map),
            display_word: display_word.clone(),
            lookup_word: lookup_word.clone(),
        }),
        data!(ReferenceAnnotationSource::PlaceAssignment {
            frame,
            assignment,
            source_node,
            target_node,
            display_word,
            lookup_word,
        }) => new!(ReferenceAnnotationSource::PlaceAssignment {
            frame: *frame,
            assignment: *assignment,
            source_node: first_translated_id(*source_node, id_map),
            target_node: first_translated_id(*target_node, id_map),
            display_word: display_word.clone(),
            lookup_word: lookup_word.clone(),
        }),
        data!(ReferenceAnnotationSource::DiscourseEdge {
            edge,
            kind,
            source_node,
            target_node,
            display_word,
            lookup_word,
        }) => new!(ReferenceAnnotationSource::DiscourseEdge {
            edge: *edge,
            kind: kind.clone(),
            source_node: first_translated_id(*source_node, id_map),
            target_node: first_translated_id(*target_node, id_map),
            display_word: display_word.clone(),
            lookup_word: lookup_word.clone(),
        }),
    };
    new!(RichReferenceAnnotation {
        name: annotation.name.clone(),
        source,
    })
}

#[requires(true)]
#[ensures(true)]
fn first_translated_id(
    source_id: RawSyntaxNodeId,
    id_map: &HashMap<RawSyntaxNodeId, Vec<RawSyntaxNodeId>>,
) -> RawSyntaxNodeId {
    id_map
        .get(&source_id)
        .and_then(|ids| ids.first().copied())
        .unwrap_or(source_id)
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|slot| matches!(slot, ReferenceSlotName::Numbered(1 | 2))))]
fn reference_slot_for_discourse_edge_kind(kind: ReferenceKind) -> Option<ReferenceSlotName> {
    match kind {
        ReferenceKind::RelativePhraseHead => Some(ReferenceSlotName::Numbered(1)),
        ReferenceKind::RelativePhraseArgument => Some(ReferenceSlotName::Numbered(2)),
        ReferenceKind::SumtiAssociation
        | ReferenceKind::ProBridiAssignment
        | ReferenceKind::Koha
        | ReferenceKind::Ri
        | ReferenceKind::Cehu
        | ReferenceKind::Letter
        | ReferenceKind::Ra
        | ReferenceKind::Ru
        | ReferenceKind::Keha
        | ReferenceKind::VohaSeries
        | ReferenceKind::DaSeries
        | ReferenceKind::BrodaSeries
        | ReferenceKind::GohaSeries
        | ReferenceKind::Utterance => None,
    }
}

#[requires(true)]
#[ensures(target.len() >= old(target.len()))]
fn extend_unique_rich_annotations(
    target: &mut Vec<RichReferenceAnnotation>,
    source: impl IntoIterator<Item = RichReferenceAnnotation>,
) {
    for item in source {
        if !target.iter().any(|existing| existing == &item) {
            target.push(item);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn reference_sources<'tree, A>(analysis: &A, tree: &TreeValue) -> Vec<ReferenceSource>
where
    A: ReferenceAnalysisView<'tree>,
{
    let mut nodes = BTreeMap::<RawSyntaxNodeId, bool>::new();
    for frame in analysis.place_analysis().frames().iter().filter(|frame| {
        !analysis
            .place_analysis()
            .assignments_for_frame(frame.id)
            .is_empty()
    }) {
        if let Some(source) = displayed_place_frame_source_in_tree(frame, tree) {
            nodes.entry(source).or_insert(false);
        }
    }
    for edge in analysis.discourse_references().edges() {
        let Some(target) = resolved_reference_target_node(analysis, &edge.target) else {
            continue;
        };
        if !contains_syntax_id(tree, target) {
            continue;
        }
        let is_pro_cmavo = edge.kind != ReferenceKind::BrodaSeries
            && word_for_syntax_id(tree, edge.source)
                .is_some_and(|word| word.constructor == "Cmavo");
        nodes
            .entry(edge.source)
            .and_modify(|pro_cmavo| *pro_cmavo |= is_pro_cmavo)
            .or_insert(is_pro_cmavo);
    }
    let mut sources = nodes
        .into_iter()
        .filter_map(|(node, pro_cmavo)| {
            let word = word_for_syntax_id(tree, node)?;
            let preorder = analysis
                .syntax_index()
                .metadata(node)
                .map(|metadata| metadata.preorder)
                .unwrap_or(node.0);
            Some(ReferenceSource {
                node,
                word,
                pro_cmavo,
                preorder,
            })
        })
        .collect::<Vec<_>>();
    sources.sort_by_key(|source| source.preorder);
    sources
}

#[requires(true)]
#[ensures(true)]
fn source_names(
    sources: &[ReferenceSource],
    visible_words: &HashSet<String>,
) -> HashMap<RawSyntaxNodeId, ReferenceSourceName> {
    let unique_words = sources
        .iter()
        .map(|source| source.word.text.clone())
        .collect::<BTreeSet<_>>();
    let pro_words = sources
        .iter()
        .filter(|source| source.pro_cmavo)
        .map(|source| source.word.text.clone())
        .collect::<HashSet<_>>();
    let stems = word_stems(&unique_words, &pro_words);
    let mut sources_by_word = HashMap::<String, Vec<&ReferenceSource>>::new();
    for source in sources {
        sources_by_word
            .entry(source.word.text.clone())
            .or_default()
            .push(source);
    }
    let mut names = HashMap::new();
    for (word, word_sources) in sources_by_word {
        let stem = stems.get(&word).cloned().unwrap_or_else(|| word.clone());
        let needs_occurrence = word_sources.len() > 1 || visible_words.contains(&stem);
        let mut word_sources = word_sources;
        word_sources.sort_by_key(|source| source.preorder);
        for (index, source) in word_sources.into_iter().enumerate() {
            names.insert(
                source.node,
                new!(ReferenceSourceName {
                    name: ReferenceName {
                        stem: stem.clone(),
                        occurrence: needs_occurrence.then_some(index + 1),
                        slot: None,
                    },
                    display_word: source.word.text.clone(),
                    lookup_word: canonicalize_text(&source.word.text),
                }),
            );
        }
    }
    names
}

#[requires(true)]
#[ensures(true)]
fn word_stems(words: &BTreeSet<String>, pro_words: &HashSet<String>) -> HashMap<String, String> {
    let mut stems = HashMap::new();
    let mut groups = BTreeMap::<char, Vec<String>>::new();
    for word in words {
        if pro_words.contains(word) {
            stems.insert(word.clone(), word.clone());
        } else if let Some(first) = word.chars().next() {
            groups.entry(first).or_default().push(word.clone());
        }
    }
    for group in groups.values() {
        let prefix_len = if group.len() == 1 {
            1
        } else {
            group
                .iter()
                .map(|word| shortest_unique_prefix_len(word, group))
                .max()
                .unwrap_or(1)
        };
        for word in group {
            stems.insert(word.clone(), prefix_chars(word, prefix_len));
        }
    }
    stems
}

#[requires(!word.is_empty())]
#[ensures(ret > 0)]
fn shortest_unique_prefix_len(word: &str, group: &[String]) -> usize {
    let word_len = word.chars().count();
    for len in 1..=word_len {
        let prefix = prefix_chars(word, len);
        if group
            .iter()
            .filter(|other| other.as_str() != word)
            .all(|other| !other.starts_with(&prefix))
        {
            return len;
        }
    }
    word_len
}

#[requires(len > 0)]
#[ensures(!ret.is_empty())]
fn prefix_chars(word: &str, len: usize) -> String {
    word.chars().take(len).collect()
}

#[requires(true)]
#[ensures(true)]
fn resolved_reference_target_node<'tree, A>(
    analysis: &A,
    target: &ReferenceTarget,
) -> Option<RawSyntaxNodeId>
where
    A: ReferenceAnalysisView<'tree>,
{
    match target {
        ReferenceTarget::ResolvedNode(node) => Some(*node),
        ReferenceTarget::ResolvedFrame(frame) => analysis
            .place_analysis()
            .frame(*frame)
            .map(|frame| frame.node),
        ReferenceTarget::AmbiguousNodes(_)
        | ReferenceTarget::Unresolved(_)
        | ReferenceTarget::Vague(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn displayed_place_frame_source_from_names(
    frame: &SelbriPlaceFrame,
    source_names: &HashMap<RawSyntaxNodeId, ReferenceSourceName>,
) -> Option<RawSyntaxNodeId> {
    displayed_place_frame_candidates(frame).find(|node| source_names.contains_key(node))
}

#[requires(true)]
#[ensures(true)]
fn displayed_place_frame_source_in_tree(
    frame: &SelbriPlaceFrame,
    tree: &TreeValue,
) -> Option<RawSyntaxNodeId> {
    displayed_place_frame_candidates(frame).find(|node| word_for_syntax_id(tree, *node).is_some())
}

#[requires(true)]
#[ensures(true)]
fn displayed_place_frame_candidates(
    frame: &SelbriPlaceFrame,
) -> impl Iterator<Item = RawSyntaxNodeId> + '_ {
    let displayable = matches!(
        frame.kind,
        PlaceFrameKind::BaseSelbri | PlaceFrameKind::TanruUnit
    );
    [
        displayable.then_some(frame.node),
        displayable
            .then_some(())
            .and_then(|_| frame.tanru_unit.map(|tanru_unit| tanru_unit.0)),
        displayable
            .then_some(())
            .and_then(|_| frame.selbri.map(|selbri| selbri.0)),
    ]
    .into_iter()
    .flatten()
}

#[requires(true)]
#[ensures(true)]
pub fn generated_reference_slot_name_for_place_slot(
    slot: PlaceSlot,
    index: &GeneratedSyntaxIndex<'_>,
    source: &str,
    options: TreeRenderOptions,
) -> ReferenceSlotName {
    ReferenceIndexView::reference_slot_name_for_place_slot(index, slot, source, options)
}

#[requires(true)]
#[ensures(true)]
fn reference_slot_name_for_assignment<'tree, A>(
    analysis: &A,
    assignment: &SumtiPlaceAssignment,
    source: &str,
    options: TreeRenderOptions,
) -> ReferenceSlotName
where
    A: ReferenceAnalysisView<'tree>,
{
    let slot = governed_termset_display_slot(analysis, assignment).unwrap_or(assignment.slot);
    analysis
        .syntax_index()
        .reference_slot_name_for_place_slot(slot, source, options)
}

#[requires(true)]
#[ensures(ret.is_none_or(|slot| matches!(slot, PlaceSlot::Modal(Some(_)))))]
fn governed_termset_display_slot<'tree, A>(
    analysis: &A,
    assignment: &SumtiPlaceAssignment,
) -> Option<PlaceSlot>
where
    A: ReferenceAnalysisView<'tree>,
{
    let PlaceSlot::Numbered(_) = assignment.slot else {
        return None;
    };
    let term = assignment.term?;
    let termset = analysis.syntax_index().termset_ancestor_for_term(term.0)?;
    let termset_order = source_order_for_node(analysis.syntax_index(), termset)?;
    analysis
        .place_analysis()
        .assignments_for_frame(assignment.frame)
        .iter()
        .filter_map(|assignment_id| {
            let modal_assignment = analysis.place_analysis().assignment(*assignment_id)?;
            let PlaceSlot::Modal(Some(tag)) = modal_assignment.slot else {
                return None;
            };
            if !analysis
                .syntax_index()
                .sumti_is_omitted_placeholder(modal_assignment.sumti.0)
            {
                return None;
            }
            let modal_order = source_order_for_node(analysis.syntax_index(), tag)?;
            if modal_order >= termset_order {
                return None;
            }
            let following =
                nearest_following_termset_for_assignment(analysis, modal_assignment, modal_order)?;
            (following == termset).then_some((modal_order, tag))
        })
        .max_by_key(|(order, _tag)| *order)
        .map(|(_order, tag)| PlaceSlot::Modal(Some(tag)))
}

#[requires(true)]
#[ensures(true)]
fn nearest_following_termset_for_assignment<'tree, A>(
    analysis: &A,
    assignment: &SumtiPlaceAssignment,
    assignment_order: usize,
) -> Option<RawSyntaxNodeId>
where
    A: ReferenceAnalysisView<'tree>,
{
    analysis
        .place_analysis()
        .assignments_for_frame(assignment.frame)
        .iter()
        .filter_map(|assignment_id| {
            let candidate = analysis.place_analysis().assignment(*assignment_id)?;
            let term = candidate.term?;
            let termset = analysis.syntax_index().termset_ancestor_for_term(term.0)?;
            let termset_order = source_order_for_node(analysis.syntax_index(), termset)?;
            (termset_order > assignment_order).then_some((termset_order, termset))
        })
        .min_by_key(|(order, _termset)| *order)
        .map(|(_order, termset)| termset)
}

#[requires(true)]
#[ensures(true)]
fn source_order_for_node<'tree, I>(index: &I, node: RawSyntaxNodeId) -> Option<usize>
where
    I: ReferenceIndexView<'tree>,
{
    index.metadata(node).map(|metadata| metadata.leaf_start)
}

#[requires(true)]
#[ensures(true)]
fn generated_modal_slot_words(
    index: &GeneratedSyntaxIndex<'_>,
    tag: RawSyntaxNodeId,
    source: &str,
    options: TreeRenderOptions,
) -> Vec<String> {
    match index.node(tag) {
        Some(GeneratedSyntaxNodeRef::FihoTenseSyntax(tense)) => {
            generated_words_for_relation(&tense.selbri, source, options)
        }
        Some(GeneratedSyntaxNodeRef::TenseModalSyntax(tense)) => {
            generated_words_for_tense_modal(tense, source, options)
        }
        Some(node) => {
            let words = generated_words_for_node(node, source, options);
            if words.is_empty() {
                generated_source_span_words(index, tag, source)
            } else {
                words
            }
        }
        None => Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_source_span_words(
    index: &GeneratedSyntaxIndex<'_>,
    node: RawSyntaxNodeId,
    source: &str,
) -> Vec<String> {
    index
        .metadata(node)
        .into_iter()
        .filter_map(|metadata| generated_metadata_source_text(metadata, source))
        .collect()
}

#[requires(true)]
#[ensures(ret.as_ref().is_none_or(|text| !text.is_empty()))]
fn generated_metadata_source_text(metadata: &SyntaxNodeMetadata, source: &str) -> Option<String> {
    let first = metadata.first_source_span.as_ref()?;
    let last = metadata.last_source_span.as_ref()?;
    // Reference labels preserve the user's raw spacing between boundary tokens.
    source
        .get(first.byte_start..last.byte_end)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

#[requires(true)]
#[ensures(true)]
fn generated_words_for_tense_modal(
    tense: &generated::TenseModalSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Vec<String> {
    match &tense.0 {
        generated::TenseModalBodySyntax::TenseModalAtom(atom) => {
            generated_words_for_tense_modal_atom(atom, source, options)
        }
        generated::TenseModalBodySyntax::ConnectedTenseModal(connected) => {
            let mut words = generated_words_for_tense_modal_atom(&connected.first, source, options);
            for continuation in &connected.continuations {
                words.extend(generated_words_for_node(
                    GeneratedSyntaxNodeRef::ConnectedTenseModalContinuationSyntax(continuation),
                    source,
                    options,
                ));
            }
            words
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_words_for_tense_modal_atom(
    atom: &generated::TenseModalAtomSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Vec<String> {
    match atom {
        generated::TenseModalAtomSyntax::FihoTense(tense) => {
            generated_words_for_relation(&tense.selbri, source, options)
        }
        _ => generated_words_for_node(generated_tense_modal_atom_node_ref(atom), source, options),
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_tense_modal_atom_node_ref(
    atom: &generated::TenseModalAtomSyntax,
) -> GeneratedSyntaxNodeRef<'_> {
    match atom {
        generated::TenseModalAtomSyntax::CompositeTense(_) => {
            GeneratedSyntaxNodeRef::TenseModalAtomSyntaxCompositeTense(atom)
        }
        generated::TenseModalAtomSyntax::FihoTense(_) => {
            GeneratedSyntaxNodeRef::TenseModalAtomSyntaxFihoTense(atom)
        }
        generated::TenseModalAtomSyntax::ModalTense(_) => {
            GeneratedSyntaxNodeRef::TenseModalAtomSyntaxModalTense(atom)
        }
        generated::TenseModalAtomSyntax::NaheSeFlatPrefixedTense(_) => {
            GeneratedSyntaxNodeRef::TenseModalAtomSyntaxNaheSeFlatPrefixedTense(atom)
        }
        generated::TenseModalAtomSyntax::SeFlatPrefixedTense(_) => {
            GeneratedSyntaxNodeRef::TenseModalAtomSyntaxSeFlatPrefixedTense(atom)
        }
        generated::TenseModalAtomSyntax::FaFlatTagTense(_) => {
            GeneratedSyntaxNodeRef::TenseModalAtomSyntaxFaFlatTagTense(atom)
        }
        generated::TenseModalAtomSyntax::ZantufaRecursiveTagTense(_) => {
            GeneratedSyntaxNodeRef::TenseModalAtomSyntaxZantufaRecursiveTagTense(atom)
        }
        generated::TenseModalAtomSyntax::StickyTense(_) => {
            GeneratedSyntaxNodeRef::TenseModalAtomSyntaxStickyTense(atom)
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn generated_words_for_relation(
    selbri: &GeneratedSelbriSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Vec<String> {
    let mut collector = GeneratedSyntaxWordCollector::new(source, options);
    selbri.visit_in_order(&mut collector);
    collector.words
}

#[requires(true)]
#[ensures(true)]
fn generated_words_for_node(
    node: GeneratedSyntaxNodeRef<'_>,
    source: &str,
    options: TreeRenderOptions,
) -> Vec<String> {
    let mut collector = GeneratedSyntaxWordCollector::new(source, options);
    match node {
        GeneratedSyntaxNodeRef::TenseModalSyntax(value) => value.visit_in_order(&mut collector),
        GeneratedSyntaxNodeRef::TenseModalBodySyntaxConnectedTenseModal(value)
        | GeneratedSyntaxNodeRef::TenseModalBodySyntaxTenseModalAtom(value) => {
            value.visit_in_order(&mut collector)
        }
        GeneratedSyntaxNodeRef::ConnectedTenseModalSyntax(value) => {
            value.visit_in_order(&mut collector)
        }
        GeneratedSyntaxNodeRef::ConnectedTenseModalContinuationSyntax(value) => {
            value.visit_in_order(&mut collector)
        }
        GeneratedSyntaxNodeRef::LeadingTermTagTenseModalSyntaxPuBeforeNaheLeadingTermTagTense(
            value,
        )
        | GeneratedSyntaxNodeRef::LeadingTermTagTenseModalSyntaxPuDistanceBeforeTagLeadingTermTagTense(
            value,
        )
        | GeneratedSyntaxNodeRef::LeadingTermTagTenseModalSyntaxZiBeforeZiLeadingTermTagTense(
            value,
        )
        | GeneratedSyntaxNodeRef::LeadingTermTagTenseModalSyntaxVaBeforeVaLeadingTermTagTense(
            value,
        )
        | GeneratedSyntaxNodeRef::LeadingTermTagTenseModalSyntaxMohiBeforeMohiLeadingTermTagTense(
            value,
        )
        | GeneratedSyntaxNodeRef::LeadingTermTagTenseModalSyntaxCahaBeforeTagLeadingTermTagTense(
            value,
        )
        | GeneratedSyntaxNodeRef::LeadingTermTagTenseModalSyntaxIntervalPropertyLeadingTermTagTense(
            value,
        )
        | GeneratedSyntaxNodeRef::LeadingTermTagTenseModalSyntaxTenseModal(value) => {
            value.visit_in_order(&mut collector)
        }
        GeneratedSyntaxNodeRef::TenseModalAtomSyntaxCompositeTense(value)
        | GeneratedSyntaxNodeRef::TenseModalAtomSyntaxFihoTense(value)
        | GeneratedSyntaxNodeRef::TenseModalAtomSyntaxModalTense(value)
        | GeneratedSyntaxNodeRef::TenseModalAtomSyntaxNaheSeFlatPrefixedTense(value)
        | GeneratedSyntaxNodeRef::TenseModalAtomSyntaxSeFlatPrefixedTense(value)
        | GeneratedSyntaxNodeRef::TenseModalAtomSyntaxFaFlatTagTense(value)
        | GeneratedSyntaxNodeRef::TenseModalAtomSyntaxZantufaRecursiveTagTense(value)
        | GeneratedSyntaxNodeRef::TenseModalAtomSyntaxStickyTense(value) => {
            value.visit_in_order(&mut collector)
        }
        GeneratedSyntaxNodeRef::FihoTenseSyntax(value) => value.visit_in_order(&mut collector),
        GeneratedSyntaxNodeRef::CompositeTenseSyntaxPrefixedTimeSpaceCahaTense(value)
        | GeneratedSyntaxNodeRef::CompositeTenseSyntaxTimeSpaceCahaKiTense(value)
        | GeneratedSyntaxNodeRef::CompositeTenseSyntaxCuheTense(value) => {
            value.visit_in_order(&mut collector)
        }
        GeneratedSyntaxNodeRef::ModalTenseSyntax(value) => value.visit_in_order(&mut collector),
        GeneratedSyntaxNodeRef::NaheSeFlatPrefixedTenseSyntax(value) => {
            value.visit_in_order(&mut collector)
        }
        GeneratedSyntaxNodeRef::SeFlatPrefixedTenseSyntax(value) => {
            value.visit_in_order(&mut collector)
        }
        GeneratedSyntaxNodeRef::FaFlatTagTenseSyntax(value) => value.visit_in_order(&mut collector),
        GeneratedSyntaxNodeRef::ZantufaRecursiveTagTenseSyntax(value) => {
            value.visit_in_order(&mut collector)
        }
        GeneratedSyntaxNodeRef::StickyTenseSyntax(value) => value.visit_in_order(&mut collector),
        _ => {}
    }
    collector.words
}

#[derive(Debug)]
#[invariant(true)]
struct GeneratedSyntaxWordCollector<'source> {
    source: &'source str,
    options: TreeRenderOptions,
    words: Vec<String>,
}

impl<'source> GeneratedSyntaxWordCollector<'source> {
    #[requires(true)]
    #[ensures(ret.source == source)]
    fn new(source: &'source str, options: TreeRenderOptions) -> Self {
        Self {
            source,
            options,
            words: Vec::new(),
        }
    }
}

impl<'tree> TreeVisitor<'tree> for GeneratedSyntaxWordCollector<'_> {
    type Node = GeneratedSyntaxNodeRef<'tree>;
    type Atom = GeneratedSyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let GeneratedSyntaxAtomRef::Token(token) = atom;
        self.words
            .push(token_word_text(token, self.source, self.options));
    }
}

#[requires(true)]
#[ensures(!ret.is_empty())]
fn token_word_text(token: &Token, source: &str, options: TreeRenderOptions) -> String {
    first_word_label(&morphology_tree_value(token.core_word(), source, options))
        .map(|word| word.text.clone())
        .unwrap_or_else(|| token.core_word().to_string())
}

#[requires(true)]
#[ensures(true)]
fn visible_word_texts(tree: &TreeValue) -> HashSet<String> {
    let mut words = HashSet::new();
    collect_visible_word_texts(tree, &mut words);
    words
}

#[requires(true)]
#[ensures(true)]
fn collect_visible_word_texts(tree: &TreeValue, words: &mut HashSet<String>) {
    match tree {
        TreeValue::Node(node) => {
            for entry in &node.entries {
                collect_visible_word_texts(&entry.value, words);
            }
        }
        TreeValue::Collection(items) => {
            for item in items {
                collect_visible_word_texts(item, words);
            }
        }
        TreeValue::Syntax { value, .. } => collect_visible_word_texts(value, words),
        TreeValue::Word { phonemes, .. } => {
            words.insert(phonemes.clone());
        }
        TreeValue::Verbatim { .. } | TreeValue::Text(_) | TreeValue::Span { .. } => {}
    }
}

#[requires(true)]
#[ensures(true)]
fn word_for_syntax_id(tree: &TreeValue, id: RawSyntaxNodeId) -> Option<TreeWordLabel> {
    match tree {
        TreeValue::Syntax { syntax_ids, value } if syntax_ids.contains(&id) => {
            first_word_label(value).or_else(|| word_for_syntax_id(value, id))
        }
        TreeValue::Syntax { value, .. } => word_for_syntax_id(value, id),
        TreeValue::Node(node) => node
            .entries
            .iter()
            .find_map(|entry| word_for_syntax_id(&entry.value, id)),
        TreeValue::Collection(items) => items.iter().find_map(|item| word_for_syntax_id(item, id)),
        TreeValue::Word { .. }
        | TreeValue::Verbatim { .. }
        | TreeValue::Text(_)
        | TreeValue::Span { .. } => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn contains_syntax_id(tree: &TreeValue, id: RawSyntaxNodeId) -> bool {
    match tree {
        TreeValue::Syntax { syntax_ids, value } => {
            syntax_ids.contains(&id) || contains_syntax_id(value, id)
        }
        TreeValue::Node(node) => node
            .entries
            .iter()
            .any(|entry| contains_syntax_id(&entry.value, id)),
        TreeValue::Collection(items) => items.iter().any(|item| contains_syntax_id(item, id)),
        TreeValue::Word { .. }
        | TreeValue::Verbatim { .. }
        | TreeValue::Text(_)
        | TreeValue::Span { .. } => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn first_word_label(tree: &TreeValue) -> Option<TreeWordLabel> {
    match tree {
        TreeValue::Node(node) => node
            .entries
            .iter()
            .find_map(|entry| first_word_label(&entry.value)),
        TreeValue::Collection(items) => items.iter().find_map(first_word_label),
        TreeValue::Syntax { value, .. } => first_word_label(value),
        TreeValue::Word { .. } => tree_word_label(tree),
        TreeValue::Verbatim { .. } | TreeValue::Text(_) | TreeValue::Span { .. } => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn tree_word_label(tree: &TreeValue) -> Option<TreeWordLabel> {
    match tree {
        TreeValue::Word {
            constructor,
            phonemes,
            ..
        } => Some(TreeWordLabel {
            constructor,
            text: phonemes.clone(),
        }),
        TreeValue::Syntax { value, .. } => tree_word_label(value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn disambiguates_prefix_group_with_shared_length() {
        let sources = vec![source(1, "kláma", false, 1), source(2, "kárce", false, 2)];
        let names = source_names(&sources, &HashSet::new());
        assert_eq!(names[&RawSyntaxNodeId(1)].name.stem, "kl");
        assert_eq!(names[&RawSyntaxNodeId(2)].name.stem, "ká");
        assert_eq!(names[&RawSyntaxNodeId(1)].name.occurrence, None);
        assert_eq!(names[&RawSyntaxNodeId(1)].lookup_word, "klama");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn numbers_duplicate_words_by_preorder() {
        let sources = vec![
            source(10, "kláma", false, 20),
            source(11, "kláma", false, 30),
        ];
        let names = source_names(&sources, &HashSet::new());
        assert_eq!(names[&RawSyntaxNodeId(10)].name.stem, "k");
        assert_eq!(names[&RawSyntaxNodeId(10)].name.occurrence, Some(1));
        assert_eq!(names[&RawSyntaxNodeId(11)].name.occurrence, Some(2));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn numbers_full_pro_cmavo_stems() {
        let sources = vec![source(1, "ri", true, 1)];
        let visible_words = HashSet::from(["ri".to_owned()]);
        let names = source_names(&sources, &visible_words);
        assert_eq!(names[&RawSyntaxNodeId(1)].name.stem, "ri");
        assert_eq!(names[&RawSyntaxNodeId(1)].name.occurrence, Some(1));
    }

    #[requires(!word.is_empty())]
    #[ensures(ret.word.text == word)]
    fn source(node: usize, word: &str, pro_cmavo: bool, preorder: usize) -> ReferenceSource {
        ReferenceSource {
            node: RawSyntaxNodeId(node),
            word: TreeWordLabel {
                constructor: if pro_cmavo { "Cmavo" } else { "Gismu" },
                text: word.to_owned(),
            },
            pro_cmavo,
            preorder,
        }
    }
}
