//! Display model for semantic reference edges.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[allow(unused_imports)]
use bityzba::{data, ensures, invariant, requires};
use jbotci_semantics::references::{
    PlaceFrameKind, PlaceSlot, RawSyntaxNodeId, ReferenceAnalysis, ReferenceKind, ReferenceTarget,
    SelbriPlaceFrame, SyntaxIndex,
};
use jbotci_syntax::ast::{
    AtomRef as SyntaxAtomRef, NodeRef as SyntaxNodeRef, RelationSyntax, TenseModalSyntax,
    TenseModalSyntaxData, Token, TreeNode as SyntaxTreeNode,
};
use jbotci_tree::TreeVisitor;

use crate::TreeRenderOptions;
use crate::tree::{TreeValue, morphology_tree_value};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[invariant(true)]
pub(crate) struct ReferenceName {
    pub(crate) stem: String,
    pub(crate) occurrence: Option<usize>,
    pub(crate) slot: Option<ReferenceSlotName>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[invariant(true)]
#[invariant(::Numbered(_) => true)]
#[invariant(::Modal(..) => true)]
#[invariant(::Fai => true)]
pub(crate) enum ReferenceSlotName {
    Numbered(u8),
    Modal(Vec<String>),
    Fai,
}

impl ReferenceSlotName {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    pub(crate) fn text(&self) -> String {
        match self {
            Self::Numbered(place) => place.to_string(),
            Self::Modal(words) if words.is_empty() => "modal".to_owned(),
            Self::Modal(words) => words.join(" "),
            Self::Fai => "fai".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[invariant(true)]
pub(crate) struct ReferenceAnnotations {
    pub(crate) incoming: Vec<ReferenceName>,
    pub(crate) outgoing: Vec<ReferenceName>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub(crate) struct ReferenceDisplayModel {
    incoming_by_node: BTreeMap<RawSyntaxNodeId, BTreeSet<ReferenceName>>,
    outgoing_by_node: BTreeMap<RawSyntaxNodeId, BTreeSet<ReferenceName>>,
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

impl ReferenceDisplayModel {
    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn new(
        analysis: &ReferenceAnalysis<'_>,
        tree: &TreeValue,
        source: &str,
        options: TreeRenderOptions,
    ) -> Self {
        let visible_words = visible_word_texts(tree);
        let sources = reference_sources(analysis, tree);
        let source_names = source_names(&sources, &visible_words);
        let mut model = Self {
            incoming_by_node: BTreeMap::new(),
            outgoing_by_node: BTreeMap::new(),
        };
        model.add_place_annotations(analysis, source, options, &source_names);
        model.add_discourse_annotations(analysis, tree, &source_names);
        model
    }

    #[requires(true)]
    #[ensures(true)]
    pub(crate) fn annotations_for_syntax_ids(
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
    fn add_place_annotations(
        &mut self,
        analysis: &ReferenceAnalysis<'_>,
        source: &str,
        options: TreeRenderOptions,
        source_names: &HashMap<RawSyntaxNodeId, ReferenceName>,
    ) {
        for frame in analysis
            .place_analysis
            .frames()
            .iter()
            .filter(|frame| is_displayed_place_frame(analysis, frame))
        {
            if analysis
                .place_analysis
                .assignments_for_frame(frame.id)
                .is_empty()
            {
                continue;
            }
            let Some(base_name) = source_names.get(&frame.node).cloned() else {
                continue;
            };
            self.outgoing_by_node
                .entry(frame.node)
                .or_default()
                .insert(base_name.clone());
            for assignment_id in analysis.place_analysis.assignments_for_frame(frame.id) {
                let Some(assignment) = analysis.place_analysis.assignment(*assignment_id) else {
                    continue;
                };
                let mut place_name = base_name.clone();
                place_name.slot = Some(slot_name(
                    assignment.slot,
                    &analysis.syntax_index,
                    source,
                    options,
                ));
                self.incoming_by_node
                    .entry(assignment.argument.0)
                    .or_default()
                    .insert(place_name);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn add_discourse_annotations(
        &mut self,
        analysis: &ReferenceAnalysis<'_>,
        tree: &TreeValue,
        source_names: &HashMap<RawSyntaxNodeId, ReferenceName>,
    ) {
        for edge in analysis.discourse_references.edges() {
            let Some(target) = resolved_reference_target_node(analysis, &edge.target) else {
                continue;
            };
            if !contains_syntax_id(tree, target) {
                continue;
            }
            let Some(name) = source_names.get(&edge.source).cloned() else {
                continue;
            };
            self.outgoing_by_node
                .entry(edge.source)
                .or_default()
                .insert(name.clone());
            self.incoming_by_node
                .entry(target)
                .or_default()
                .insert(name);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn reference_sources(analysis: &ReferenceAnalysis<'_>, tree: &TreeValue) -> Vec<ReferenceSource> {
    let mut nodes = BTreeMap::<RawSyntaxNodeId, bool>::new();
    for frame in analysis
        .place_analysis
        .frames()
        .iter()
        .filter(|frame| is_displayed_place_frame(analysis, frame))
        .filter(|frame| {
            !analysis
                .place_analysis
                .assignments_for_frame(frame.id)
                .is_empty()
        })
    {
        nodes.entry(frame.node).or_insert(false);
    }
    for edge in analysis.discourse_references.edges() {
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
                .syntax_index
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
) -> HashMap<RawSyntaxNodeId, ReferenceName> {
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
                ReferenceName {
                    stem: stem.clone(),
                    occurrence: needs_occurrence.then_some(index + 1),
                    slot: None,
                },
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
fn resolved_reference_target_node(
    analysis: &ReferenceAnalysis<'_>,
    target: &ReferenceTarget,
) -> Option<RawSyntaxNodeId> {
    match target {
        ReferenceTarget::ResolvedNode(node) => Some(*node),
        ReferenceTarget::ResolvedFrame(frame) => analysis
            .place_analysis
            .frame(*frame)
            .map(|frame| frame.node),
        ReferenceTarget::AmbiguousNodes(_)
        | ReferenceTarget::Unresolved(_)
        | ReferenceTarget::Vague(_) => None,
    }
}

#[requires(true)]
#[ensures(true)]
fn is_displayed_place_frame(analysis: &ReferenceAnalysis<'_>, frame: &SelbriPlaceFrame) -> bool {
    match frame.kind {
        PlaceFrameKind::BaseRelation => analysis
            .syntax_index
            .node(frame.node)
            .is_some_and(|node| matches!(node, SyntaxNodeRef::RelationSyntaxBase(_))),
        PlaceFrameKind::RelationUnit => analysis
            .syntax_index
            .node(frame.node)
            .is_some_and(|node| matches!(node, SyntaxNodeRef::RelationUnitSyntaxWord(_))),
        _ => false,
    }
}

#[requires(true)]
#[ensures(true)]
fn slot_name(
    slot: PlaceSlot,
    index: &SyntaxIndex<'_>,
    source: &str,
    options: TreeRenderOptions,
) -> ReferenceSlotName {
    match slot {
        PlaceSlot::Numbered(place) => ReferenceSlotName::Numbered(place.get()),
        PlaceSlot::Modal(Some(tag)) => {
            ReferenceSlotName::Modal(modal_slot_words(index, tag, source, options))
        }
        PlaceSlot::Modal(None) => ReferenceSlotName::Modal(Vec::new()),
        PlaceSlot::Fai => ReferenceSlotName::Fai,
    }
}

#[requires(true)]
#[ensures(true)]
fn modal_slot_words(
    index: &SyntaxIndex<'_>,
    tag: RawSyntaxNodeId,
    source: &str,
    options: TreeRenderOptions,
) -> Vec<String> {
    match index.node(tag) {
        Some(SyntaxNodeRef::TenseModalSyntaxFiho(tense)) => {
            fiho_tense_modal_words(tense, source, options)
        }
        Some(SyntaxNodeRef::TenseModalSyntaxComposite(tense)) => {
            composite_tense_modal_words(tense, source, options)
        }
        Some(node) => words_for_node(node, source, options),
        None => Vec::new(),
    }
}

#[requires(true)]
#[ensures(true)]
fn fiho_tense_modal_words(
    tense: &TenseModalSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Vec<String> {
    match tense.as_data() {
        data!(TenseModalSyntax::Fiho { relation, .. }) => {
            words_for_relation(relation, source, options)
        }
        _ => words_for_node(SyntaxNodeRef::TenseModalSyntaxFiho(tense), source, options),
    }
}

#[requires(true)]
#[ensures(true)]
fn composite_tense_modal_words(
    tense: &TenseModalSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Vec<String> {
    let data!(TenseModalSyntax::Composite { parts }) = tense.as_data() else {
        return words_for_node(
            SyntaxNodeRef::TenseModalSyntaxComposite(tense),
            source,
            options,
        );
    };
    let mut words = Vec::new();
    for part in &parts.value {
        match part.as_data() {
            data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::Word(
                word
            )) => {
                words.push(token_word_text(word, source, options));
            }
            data!(jbotci_syntax::ast::CompositeTenseModalPartSyntax::Fiho(
                fiho
            )) => {
                if let Some(nahe) = &fiho.nahe {
                    words.push(token_word_text(nahe, source, options));
                }
                words.extend(words_for_relation(&fiho.relation, source, options));
            }
        }
    }
    words
}

#[requires(true)]
#[ensures(true)]
fn words_for_relation(
    relation: &RelationSyntax,
    source: &str,
    options: TreeRenderOptions,
) -> Vec<String> {
    let mut collector = SyntaxWordCollector::new(source, options);
    relation.visit_in_order(&mut collector);
    collector.words
}

#[requires(true)]
#[ensures(true)]
fn words_for_node(
    node: SyntaxNodeRef<'_>,
    source: &str,
    options: TreeRenderOptions,
) -> Vec<String> {
    let mut collector = SyntaxWordCollector::new(source, options);
    match node {
        SyntaxNodeRef::TenseModalSyntaxComposite(tense)
        | SyntaxNodeRef::TenseModalSyntaxPu(tense)
        | SyntaxNodeRef::TenseModalSyntaxPuDistance(tense)
        | SyntaxNodeRef::TenseModalSyntaxTimeInterval(tense)
        | SyntaxNodeRef::TenseModalSyntaxPuCaha(tense)
        | SyntaxNodeRef::TenseModalSyntaxSpaceDistance(tense)
        | SyntaxNodeRef::TenseModalSyntaxSpaceDirection(tense)
        | SyntaxNodeRef::TenseModalSyntaxSpaceMovement(tense)
        | SyntaxNodeRef::TenseModalSyntaxSimple(tense)
        | SyntaxNodeRef::TenseModalSyntaxKi(tense)
        | SyntaxNodeRef::TenseModalSyntaxFiho(tense)
        | SyntaxNodeRef::TenseModalSyntaxCaha(tense)
        | SyntaxNodeRef::TenseModalSyntaxZaho(tense)
        | SyntaxNodeRef::TenseModalSyntaxInterval(tense) => tense.visit_in_order(&mut collector),
        _ => {}
    }
    collector.words
}

#[derive(Debug)]
#[invariant(true)]
struct SyntaxWordCollector<'source> {
    source: &'source str,
    options: TreeRenderOptions,
    words: Vec<String>,
}

impl<'source> SyntaxWordCollector<'source> {
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

impl<'tree> TreeVisitor<'tree> for SyntaxWordCollector<'_> {
    type Node = SyntaxNodeRef<'tree>;
    type Atom = SyntaxAtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        match atom {
            SyntaxAtomRef::Token(token) => {
                self.words
                    .push(token_word_text(token, self.source, self.options));
            }
            SyntaxAtomRef::Word(word) => {
                if let Some(word) = first_word_label(&morphology_tree_value(
                    &jbotci_morphology::WordLike::bare(word.clone()),
                    self.source,
                    self.options,
                )) {
                    self.words.push(word.text.clone());
                }
            }
        }
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
        assert_eq!(names[&RawSyntaxNodeId(1)].stem, "kl");
        assert_eq!(names[&RawSyntaxNodeId(2)].stem, "ká");
        assert_eq!(names[&RawSyntaxNodeId(1)].occurrence, None);
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
        assert_eq!(names[&RawSyntaxNodeId(10)].stem, "k");
        assert_eq!(names[&RawSyntaxNodeId(10)].occurrence, Some(1));
        assert_eq!(names[&RawSyntaxNodeId(11)].occurrence, Some(2));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn numbers_full_pro_cmavo_stems() {
        let sources = vec![source(1, "ri", true, 1)];
        let visible_words = HashSet::from(["ri".to_owned()]);
        let names = source_names(&sources, &visible_words);
        assert_eq!(names[&RawSyntaxNodeId(1)].stem, "ri");
        assert_eq!(names[&RawSyntaxNodeId(1)].occurrence, Some(1));
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
