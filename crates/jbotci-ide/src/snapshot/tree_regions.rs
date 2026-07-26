use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::BTreeSet;

#[allow(unused_imports)]
use bityzba::{data, ensures, expensive_ensures, expensive_invariant, invariant, new, requires};
use jbotci_morphology::{WordLike, WordLikeData};
use jbotci_source::SourceSpan;
use jbotci_syntax::{
    ParseOptions, SyntaxRecoveryItem, SyntaxRecoveryParse, SyntaxRecoveryParseData,
    SyntaxTextUnitGranularity, Token, generated_model,
    parse_syntax_tokens_with_recovery_with_source_and_options_attempt, partition_syntax_text_units,
    syntax_tokens_with_options,
};
use jbotci_tree::{RecoveryItemState, TreeVisitor};

use super::DocumentSnapshot;
use crate::{LineIndex, MAX_POSITION_VALUE, Position, PositionEncoding, PositionRange};

/// A strict inner-to-outer source-span chain for one requested cursor offset.
#[invariant(!spans.is_empty(), "selection chains always contain at least the document span")]
#[expensive_invariant(spans.windows(2).all(|pair| {
    let inner = &pair[0];
    let outer = &pair[1];
    outer.byte_start <= inner.byte_start
        && inner.byte_end <= outer.byte_end
        && outer.char_start <= inner.char_start
        && inner.char_end <= outer.char_end
        && (outer.byte_start < inner.byte_start || inner.byte_end < outer.byte_end)
        && (outer.char_start < inner.char_start || inner.char_end < outer.char_end)
}), "selection parents must be strict source-coordinate supersets")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionRangeChain {
    pub spans: Vec<SourceSpan>,
}

/// Standard LSP folding categories available to transport adapters.
///
/// Lojban syntax blocks do not have a semantically correct standard category,
/// so the current tree projection deliberately returns `None` for every fold.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldingRangeKind {
    Comment,
    Imports,
    Region,
}

/// One transport-independent inclusive line range suitable for folding.
#[invariant(*start_line < *end_line, "folds must span at least two lines")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoldingRange {
    pub start_line: usize,
    pub end_line: usize,
    pub kind: Option<FoldingRangeKind>,
}

#[invariant(byte_start < byte_end)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ByteBounds {
    byte_start: usize,
    byte_end: usize,
}

impl ByteBounds {
    #[requires(byte_start < byte_end)]
    #[ensures(ret.byte_start == byte_start && ret.byte_end == byte_end)]
    fn new(byte_start: usize, byte_end: usize) -> Self {
        new!(ByteBounds {
            byte_start,
            byte_end,
        })
    }

    #[requires(other.byte_start < other.byte_end)]
    #[ensures(ret.byte_start <= self.byte_start && ret.byte_start <= other.byte_start)]
    #[ensures(ret.byte_end >= self.byte_end && ret.byte_end >= other.byte_end)]
    fn including(self, other: Self) -> Self {
        new!(ByteBounds {
            byte_start: self.byte_start.min(other.byte_start),
            byte_end: self.byte_end.max(other.byte_end),
        })
    }
}

#[invariant(span.byte_start < span.byte_end && span.char_start < span.char_end)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct TreeRegion {
    span: SourceSpan,
    foldable: bool,
}

#[invariant(regions.windows(2).all(|pair| {
    (pair[0].span.byte_start, pair[0].span.byte_end)
        < (pair[1].span.byte_start, pair[1].span.byte_end)
}), "tree regions must be source-ordered and deduplicated")]
#[derive(Debug, Clone)]
pub(super) struct TreeRegionProjection {
    regions: Vec<TreeRegion>,
}

impl TreeRegionProjection {
    #[requires(true)]
    #[ensures(ret.ranges_are_within(line_index.byte_len(), line_index.char_len()))]
    pub(super) fn build(
        parse: &SyntaxRecoveryParse,
        words: &[WordLike],
        line_index: &LineIndex,
    ) -> Self {
        let mut regions = collect_parse_regions(parse, line_index);
        let tokens = syntax_tokens_with_options(words, &ParseOptions::default());
        append_paragraph_regions(&mut regions, &tokens, line_index);

        if let SyntaxRecoveryParseData::Recovered { parse } = parse.as_data() {
            let mut skipped = new!(SkippedTokenRunCollector {
                runs: RefCell::new(Vec::new()),
            });
            generated_model::recovered::TreeWalkable::walk_with(
                parse.parse_tree.as_ref(),
                &mut skipped,
            );
            let data!(SkippedTokenRunCollector { runs }) = skipped.into_data();
            for run in runs.into_inner() {
                append_recovered_run_regions(&mut regions, run, line_index);
            }
        }

        regions.sort_by_key(|region| (region.span.byte_start, region.span.byte_end));
        let mut deduplicated: Vec<TreeRegion> = Vec::with_capacity(regions.len());
        for region in regions {
            if let Some(previous) = deduplicated.last_mut()
                && previous.span.byte_start == region.span.byte_start
                && previous.span.byte_end == region.span.byte_end
            {
                if region.foldable && !previous.foldable {
                    *previous = previous.clone().with_data(data! {
                        foldable: true,
                    });
                }
                continue;
            }
            deduplicated.push(region);
        }
        new!(TreeRegionProjection {
            regions: deduplicated,
        })
    }

    #[requires(true)]
    #[ensures(true)]
    pub(super) fn ranges_are_within(&self, byte_len: usize, char_len: usize) -> bool {
        self.regions
            .iter()
            .all(|region| region.span.byte_end <= byte_len && region.span.char_end <= char_len)
    }
}

impl DocumentSnapshot {
    /// Return strict, encoding-independent selection chains in request order.
    #[requires(true)]
    #[ensures(ret.len() == char_offsets.len())]
    #[expensive_ensures(ret.iter().all(|chain| chain.spans.windows(2).all(|pair| {
        strict_span_container(&pair[1], &pair[0])
    })), "every returned parent must strictly contain its child")]
    pub fn selection_ranges(&self, char_offsets: &[usize]) -> Vec<SelectionRangeChain> {
        char_offsets
            .iter()
            .map(|offset| self.selection_range(*offset))
            .collect()
    }

    /// Return source-ordered, encoding-independent multiline folding regions.
    #[requires(true)]
    #[ensures(ret.iter().all(|range| range.start_line < range.end_line))]
    pub fn folding_ranges(&self) -> Vec<FoldingRange> {
        let mut lines = BTreeSet::new();
        for region in &self.tree_regions.regions {
            if !region.foldable {
                continue;
            }
            let positions = self
                .line_index
                .positions_for_span(&region.span, PositionEncoding::Utf32);
            if positions.start.line < positions.end.line {
                lines.insert((positions.start.line, positions.end.line));
            }
        }
        let mut lines = lines.into_iter().collect::<Vec<_>>();
        lines.sort_by_key(|(start, end)| (*start, Reverse(*end)));
        lines
            .into_iter()
            .map(|(start_line, end_line)| {
                new!(FoldingRange {
                    start_line,
                    end_line,
                    kind: None,
                })
            })
            .collect()
    }

    #[requires(true)]
    #[ensures(!ret.spans.is_empty())]
    fn selection_range(&self, char_offset: usize) -> SelectionRangeChain {
        let char_offset = char_offset.min(self.line_index.char_len());
        let document = document_span(&self.line_index);
        let mut spans = Vec::new();

        if let Some(word) = self.word_at(char_offset) {
            spans.push(word.span.clone());
            if let Some(sequence) = self.attested_cmavo_sequence_span(word.index)
                && strict_span_container(&sequence, spans.last().expect("word span was pushed"))
            {
                spans.push(sequence);
            }
        }

        let mut covering = self
            .tree_regions
            .regions
            .iter()
            .filter(|region| {
                span_covers_offset(&region.span, char_offset, self.line_index.char_len())
                    && !same_span(&region.span, &document)
            })
            .map(|region| &region.span)
            .collect::<Vec<_>>();
        covering.sort_by_key(|span| {
            (
                span.byte_end - span.byte_start,
                Reverse(span.byte_start),
                span.byte_end,
            )
        });

        let mut appended_structural_superset = false;
        for span in covering {
            if spans
                .last()
                .is_none_or(|inner| strict_span_container(span, inner))
            {
                appended_structural_superset |= !spans.is_empty();
                spans.push(span.clone());
            }
        }

        if !appended_structural_superset {
            let line = line_span(&self.line_index, char_offset);
            if spans
                .last()
                .is_none_or(|inner| strict_span_container(&line, inner))
                && !same_span(&line, &document)
            {
                spans.push(line);
            }
        }

        if spans
            .last()
            .is_none_or(|inner| strict_span_container(&document, inner))
        {
            spans.push(document);
        }
        debug_assert!(!spans.is_empty(), "the document always closes the chain");
        new!(SelectionRangeChain { spans })
    }
}

#[invariant(true)]
struct NodeFrame {
    constructor: &'static str,
    bounds: Option<ByteBounds>,
    contains_lohu_quote: bool,
}

#[invariant(true)]
struct TreeRegionCollector<'index> {
    line_index: &'index LineIndex,
    nodes: Vec<NodeFrame>,
    regions: Vec<TreeRegion>,
}

impl<'index> TreeRegionCollector<'index> {
    #[requires(true)]
    #[ensures(ret.nodes.is_empty() && ret.regions.is_empty())]
    fn new(line_index: &'index LineIndex) -> Self {
        Self {
            line_index,
            nodes: Vec::new(),
            regions: Vec::new(),
        }
    }

    #[requires(!constructor.is_empty())]
    #[ensures(self.nodes.len() == old(self.nodes.len()) + 1)]
    fn enter_node(&mut self, constructor: &'static str) {
        self.nodes.push(NodeFrame {
            constructor,
            bounds: None,
            contains_lohu_quote: false,
        });
    }

    #[requires(!self.nodes.is_empty())]
    #[ensures(self.nodes.len() == old(self.nodes.len()) - 1)]
    fn exit_node(&mut self, constructor: &'static str) {
        let frame = self
            .nodes
            .pop()
            .expect("generated traversal exits every entered node");
        debug_assert_eq!(frame.constructor, constructor);
        let Some(bounds) = frame.bounds else {
            return;
        };
        let span = span_for_bounds(bounds, self.line_index);
        self.regions.push(new!(TreeRegion {
            span,
            foldable: foldable_constructor(constructor, frame.contains_lohu_quote),
        }));
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_token(&mut self, token: &Token) {
        if let Some(range) = token.core_word().byte_range()
            && range.start < range.end
        {
            self.include_bounds(ByteBounds::new(range.start, range.end));
        }
        if matches!(
            token.core_word().as_data(),
            data!(WordLike::QuotedWords { .. })
        ) {
            for node in &mut self.nodes {
                node.contains_lohu_quote = true;
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_recovered_error<E: RecoveryItemState>(&mut self, item: &E) {
        item.visit_source_spans(&mut |span| {
            if span.byte_start < span.byte_end {
                self.include_bounds(ByteBounds::new(span.byte_start, span.byte_end));
            }
        });
    }

    #[requires(bounds.byte_start < bounds.byte_end)]
    #[ensures(true)]
    fn include_bounds(&mut self, bounds: ByteBounds) {
        for node in &mut self.nodes {
            node.bounds = Some(
                node.bounds
                    .map_or(bounds, |existing| existing.including(bounds)),
            );
        }
    }
}

#[invariant(true)]
struct ValidTreeRegionVisitor<'index> {
    collector: TreeRegionCollector<'index>,
}

impl<'tree> TreeVisitor<'tree> for ValidTreeRegionVisitor<'_> {
    type Node = generated_model::NodeRef<'tree>;
    type Atom = generated_model::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        self.collector.enter_node(node.constructor_name());
    }

    #[requires(!self.collector.nodes.is_empty())]
    #[ensures(true)]
    fn exit_node(&mut self, node: Self::Node) {
        self.collector.exit_node(node.constructor_name());
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let generated_model::AtomRef::Token(token) = atom;
        self.collector.visit_token(token);
    }
}

#[invariant(true)]
struct RecoveredTreeRegionVisitor<'index> {
    collector: TreeRegionCollector<'index>,
}

impl<'tree> TreeVisitor<'tree> for RecoveredTreeRegionVisitor<'_> {
    type Node = generated_model::recovered::NodeRef<'tree>;
    type Atom = generated_model::recovered::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        self.collector.enter_node(node.constructor_name());
    }

    #[requires(!self.collector.nodes.is_empty())]
    #[ensures(true)]
    fn exit_node(&mut self, node: Self::Node) {
        self.collector.exit_node(node.constructor_name());
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let generated_model::recovered::AtomRef::Token(token) = atom;
        self.collector.visit_token(token);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_recovered_error<E: RecoveryItemState + serde::Serialize>(&mut self, item: &'tree E) {
        self.collector.visit_recovered_error(item);
    }
}

#[invariant(runs.borrow().iter().all(|run| !run.is_empty()), "syntax recovery never records empty skipped-token runs")]
struct SkippedTokenRunCollector<'tree> {
    runs: RefCell<Vec<&'tree [Token]>>,
}

impl<'tree> generated_model::recovered::TreeWalker<'tree> for SkippedTokenRunCollector<'tree> {
    #[requires(true)]
    #[ensures(true)]
    fn walk_recovered_error(&mut self, item: &'tree SyntaxRecoveryItem) {
        if let Some(tokens) = item.skipped_tokens() {
            self.runs.borrow_mut().push(tokens);
        }
    }
}

#[requires(true)]
#[ensures(ret.iter().all(|region| region.span.byte_end <= line_index.byte_len()))]
fn collect_parse_regions(parse: &SyntaxRecoveryParse, line_index: &LineIndex) -> Vec<TreeRegion> {
    match parse.as_data() {
        SyntaxRecoveryParseData::Valid { parse } => {
            let mut visitor = ValidTreeRegionVisitor {
                collector: TreeRegionCollector::new(line_index),
            };
            generated_model::TreeNode::visit_in_order(&parse.parse_tree, &mut visitor);
            visitor.collector.regions
        }
        SyntaxRecoveryParseData::Recovered { parse } => {
            let mut visitor = RecoveredTreeRegionVisitor {
                collector: TreeRegionCollector::new(line_index),
            };
            generated_model::recovered::TreeNode::visit_in_order(&parse.parse_tree, &mut visitor);
            visitor.collector.regions
        }
    }
}

#[requires(true)]
#[ensures(regions.len() >= old(regions.len()))]
fn append_paragraph_regions(
    regions: &mut Vec<TreeRegion>,
    tokens: &[Token],
    line_index: &LineIndex,
) {
    for unit in partition_syntax_text_units(tokens, SyntaxTextUnitGranularity::Paragraph) {
        if let Some(bounds) = token_slice_bounds(&tokens[unit.token_start..unit.token_end]) {
            regions.push(new!(TreeRegion {
                span: span_for_bounds(bounds, line_index),
                foldable: true,
            }));
        }
    }
}

#[requires(!tokens.is_empty())]
#[ensures(regions.len() >= old(regions.len()))]
fn append_recovered_run_regions(
    regions: &mut Vec<TreeRegion>,
    tokens: &[Token],
    line_index: &LineIndex,
) {
    for paragraph in partition_syntax_text_units(tokens, SyntaxTextUnitGranularity::Paragraph) {
        let paragraph_tokens = &tokens[paragraph.token_start..paragraph.token_end];
        if let Some(bounds) = token_slice_bounds(paragraph_tokens) {
            regions.push(new!(TreeRegion {
                span: span_for_bounds(bounds, line_index),
                foldable: true,
            }));
        }
        for statement in
            partition_syntax_text_units(paragraph_tokens, SyntaxTextUnitGranularity::Statement)
        {
            let statement_tokens = &paragraph_tokens[statement.token_start..statement.token_end];
            let parse = parse_syntax_tokens_with_recovery_with_source_and_options_attempt(
                statement_tokens,
                line_index.text(),
                &ParseOptions::default(),
            )
            .result;
            regions.extend(collect_parse_regions(&parse, line_index));
        }
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|bounds| bounds.byte_start < bounds.byte_end))]
fn token_slice_bounds(tokens: &[Token]) -> Option<ByteBounds> {
    let mut bounds = None;
    for token in tokens {
        let Some(range) = token.core_word().byte_range() else {
            continue;
        };
        if range.start == range.end {
            continue;
        }
        let token = ByteBounds::new(range.start, range.end);
        bounds = Some(bounds.map_or(token, |existing: ByteBounds| existing.including(token)));
    }
    bounds
}

#[requires(bounds.byte_end <= line_index.byte_len())]
#[ensures(ret.byte_start == bounds.byte_start && ret.byte_end == bounds.byte_end)]
fn span_for_bounds(bounds: ByteBounds, line_index: &LineIndex) -> SourceSpan {
    let start = line_index.offsets_for_byte(bounds.byte_start);
    let end = line_index.offsets_for_byte(bounds.byte_end);
    SourceSpan::new(None, start.byte, end.byte, start.char, end.char)
        .expect("ordered byte bounds map to ordered source offsets")
}

#[requires(true)]
#[ensures(ret.byte_start == 0 && ret.byte_end == line_index.byte_len())]
fn document_span(line_index: &LineIndex) -> SourceSpan {
    SourceSpan::new(None, 0, line_index.byte_len(), 0, line_index.char_len())
        .expect("document offsets are ordered")
}

#[requires(true)]
#[ensures(ret.byte_end <= line_index.byte_len() && ret.char_end <= line_index.char_len())]
fn line_span(line_index: &LineIndex, char_offset: usize) -> SourceSpan {
    let line = line_index
        .position_for_char(
            char_offset.min(line_index.char_len()),
            PositionEncoding::Utf32,
        )
        .line;
    line_index.span_for_positions(
        &PositionRange::new(
            Position::new(line, 0),
            Position::new(line, MAX_POSITION_VALUE),
        ),
        PositionEncoding::Utf32,
        None,
    )
}

#[requires(true)]
#[ensures(true)]
fn same_span(left: &SourceSpan, right: &SourceSpan) -> bool {
    left.byte_start == right.byte_start
        && left.byte_end == right.byte_end
        && left.char_start == right.char_start
        && left.char_end == right.char_end
}

#[requires(true)]
#[ensures(ret -> !same_span(outer, inner))]
fn strict_span_container(outer: &SourceSpan, inner: &SourceSpan) -> bool {
    outer.byte_start <= inner.byte_start
        && inner.byte_end <= outer.byte_end
        && outer.char_start <= inner.char_start
        && inner.char_end <= outer.char_end
        && !same_span(outer, inner)
}

#[requires(offset <= document_char_len)]
#[ensures(true)]
fn span_covers_offset(span: &SourceSpan, offset: usize, document_char_len: usize) -> bool {
    span.char_start <= offset
        && (offset < span.char_end
            || (offset == document_char_len && span.char_end == document_char_len))
}

#[requires(!constructor.is_empty())]
#[ensures(true)]
fn foldable_constructor(constructor: &str, contains_lohu_quote: bool) -> bool {
    matches!(
        constructor,
        "SimpleParagraphSyntax"
            | "INihoParagraphSyntax"
            | "NihoParagraphSyntax"
            | "BridiStatementSyntax"
            | "BridiTailSyntax"
            | "RelativeClauseListSyntax"
            | "SumtiAssociationRelativeClauseSyntax"
            | "ZantufaRestrictiveStatementRelativeClauseSyntax"
            | "ZantufaIncidentalStatementRelativeClauseSyntax"
            | "RestrictiveBridiRelativeClauseSyntax"
            | "IncidentalBridiRelativeClauseSyntax"
            | "AbstractionTanruUnitSyntax"
            | "ZantufaStatementAbstractionTanruUnitSyntax"
            | "TextQuoteSyntax"
            | "ParentheticalTextSyntax"
            | "TermsetGroupSyntax"
            | "ForethoughtTermsetSyntax"
            | "NuhiTermsetSyntax"
            | "KeTermsetSyntax"
            | "GroupedTanruUnitSyntax"
            | "GroupedJaiInnerTanruUnitSyntax"
    ) || (constructor == "GenericCompoundQuoteSyntax" && contains_lohu_quote)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use bityzba::{ensures, requires};

    #[requires(true)]
    #[ensures(ret.len() == chain.spans.len())]
    fn char_ranges(chain: &SelectionRangeChain) -> Vec<(usize, usize)> {
        chain
            .spans
            .iter()
            .map(|span| (span.char_start, span.char_end))
            .collect()
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn nested_tanru_selection_chain_is_strict_and_reaches_the_document() {
        let source = "le ke melbi xunre ke'e rozgu cu se viska";
        let snapshot = DocumentSnapshot::new(source.to_owned(), 1);
        let offset = source.find("xunre").expect("fixture word") + 2;
        let chain = &snapshot.selection_ranges(&[offset])[0];
        let ranges = char_ranges(chain);

        assert_eq!(ranges.first(), Some(&(12, 17)));
        assert_eq!(ranges.last(), Some(&(0, source.len())));
        assert!(
            ranges.contains(&(3, 22)),
            "grouped tanru must be a selection ancestor: {ranges:?}"
        );
        assert!(
            ranges.contains(&(3, 28)),
            "description selbri must be a selection ancestor: {ranges:?}"
        );
        assert!(
            chain
                .spans
                .windows(2)
                .all(|pair| strict_span_container(&pair[1], &pair[0]))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn subordinate_quotation_selection_climbs_through_inner_and_outer_bridi() {
        let source = "mi cusku lu do poi ke'a tavla cu klama li'u";
        let snapshot = DocumentSnapshot::new(source.to_owned(), 1);
        let offset = source.find("tavla").expect("fixture word") + 1;
        let ranges = char_ranges(&snapshot.selection_ranges(&[offset])[0]);

        assert_eq!(ranges.first(), Some(&(24, 29)));
        assert!(
            ranges.iter().any(|(start, end)| {
                *start <= source.find("do poi").expect("inner bridi")
                    && source.find("klama").expect("inner selbri") + 5 <= *end
            }),
            "inner quotation bridi must be represented: {ranges:?}"
        );
        assert_eq!(ranges.last(), Some(&(0, source.len())));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn solid_cmavo_run_starts_at_word_then_attested_sequence() {
        let snapshot = DocumentSnapshot::new("ije mi klama".to_owned(), 1);
        let ranges = char_ranges(&snapshot.selection_ranges(&[1])[0]);

        assert_eq!(ranges[0], (1, 3), "selection starts at morphology word je");
        assert_eq!(ranges[1], (0, 3), "attested ije is the next expansion");
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn whole_skip_recovery_falls_back_through_line_to_document() {
        let source = "mi klama\ncu ku\nzo'e";
        let snapshot = DocumentSnapshot::new(source.to_owned(), 1);
        let offset = source.find("ku").expect("recovered word");
        let ranges = char_ranges(&snapshot.selection_ranges(&[offset])[0]);

        assert_eq!(ranges.first(), Some(&(12, 14)));
        assert!(
            ranges.contains(&(9, 14)),
            "recovered selection must include its source line: {ranges:?}"
        );
        assert_eq!(ranges.last(), Some(&(0, source.len())));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn selection_ranges_preserve_request_order_and_clamp_offsets() {
        let snapshot = DocumentSnapshot::new("mi klama".to_owned(), 1);
        let chains = snapshot.selection_ranges(&[4, 0, usize::MAX]);

        assert_eq!(chains.len(), 3);
        assert_eq!(char_ranges(&chains[0]).first(), Some(&(3, 8)));
        assert_eq!(char_ranges(&chains[1]).first(), Some(&(0, 2)));
        assert_eq!(
            char_ranges(&chains[2]).last(),
            Some(&(0, snapshot.line_index.char_len()))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn folding_covers_multiline_structures_and_omits_single_line_nodes() {
        let source = concat!(
            "ni'o mi djuno lo du'u\n",
            "do poi ke'a tavla\n",
            "cu klama kei\n",
            ".i mi cusku lu\n",
            "do tavla\n",
            "li'u\n",
            "ni'o mi klama",
        );
        let snapshot = DocumentSnapshot::new(source.to_owned(), 1);
        let folds = snapshot.folding_ranges();

        assert!(folds.iter().all(|fold| fold.start_line < fold.end_line));
        assert!(folds.iter().all(|fold| fold.kind.is_none()));
        assert!(
            folds
                .iter()
                .any(|fold| fold.start_line == 0 && fold.end_line >= 2),
            "first paragraph/abstraction must fold: {folds:?}"
        );
        assert!(
            folds
                .iter()
                .any(|fold| fold.start_line == 3 && fold.end_line == 5),
            "LU quotation must fold: {folds:?}"
        );
        assert!(
            folds.iter().all(|fold| fold.start_line != 6),
            "single-line final paragraph must not fold: {folds:?}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn folding_handles_lohu_parentheticals_termsets_and_grouped_tanru() {
        let source = concat!(
            "lo'u coi\n",
            "ni'o li'u\n",
            "le'u cu se cusku\n",
            "to do\n",
            "tavla toi\n",
            "nu'i mi\n",
            "ce'e do nu'u cu klama\n",
            "le ke melbi\n",
            "xunre ke'e rozgu cu se viska",
        );
        let folds = DocumentSnapshot::new(source.to_owned(), 1).folding_ranges();

        for expected in [(0, 2), (3, 4), (5, 6), (7, 8)] {
            assert!(
                folds
                    .iter()
                    .any(|fold| (fold.start_line, fold.end_line) == expected),
                "missing fold {expected:?} in {folds:?}"
            );
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn quoted_boundaries_do_not_create_false_paragraph_or_quote_folds() {
        let source = concat!(
            "mi cusku lu\n",
            "do cusku lo'u ni'o li'u le'u\n",
            "li'u\n",
            ".i mi klama",
        );
        let folds = DocumentSnapshot::new(source.to_owned(), 1).folding_ranges();

        assert!(
            folds
                .iter()
                .any(|fold| fold.start_line == 0 && fold.end_line == 2),
            "real LU quote must fold: {folds:?}"
        );
        assert!(
            folds.iter().all(|fold| fold.start_line != 1),
            "quoted NIhO/LIhU must not create top-level folds: {folds:?}"
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn folding_line_ranges_are_encoding_independent_and_survive_recovery() {
        let ascii = "mi cusku lu\ndo tavla\nli'u\n.i ku cu klama";
        let multibyte = "mí cusku lu\ndo tavla\nli'u\n.i ku cu klama";
        let ascii_folds = DocumentSnapshot::new(ascii.to_owned(), 1).folding_ranges();
        let multibyte_folds = DocumentSnapshot::new(multibyte.to_owned(), 1).folding_ranges();

        assert_eq!(ascii_folds, multibyte_folds);
        assert!(
            ascii_folds
                .iter()
                .any(|fold| fold.start_line == 0 && fold.end_line == 2),
            "quotation fold must survive the later recovery error: {ascii_folds:?}"
        );
    }
}
