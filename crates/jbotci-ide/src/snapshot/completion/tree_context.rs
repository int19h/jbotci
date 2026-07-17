use std::collections::BTreeSet;

#[allow(unused_imports)]
use bityzba::{ensures, invariant, new, requires};
use jbotci_morphology::Cmavo;
use jbotci_source::SourceSpan;
use jbotci_syntax::{
    SyntaxRecoveryItem, SyntaxRecoveryParse, SyntaxRecoveryParseData, Token, generated_model,
};
use jbotci_tree::{FieldRef, RecoveryItemState, TreeVisitor};
use serde::Serialize;

/// One legal closer contributed by a snapshot-tree construct covering the cut.
#[invariant(!construct.is_empty())]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct OpenConstructCandidate {
    pub(super) cmavo: Cmavo,
    pub(super) construct: String,
}

/// Tree-anchored facts for one completion interpretation.
#[invariant(restart_byte <= cut_byte)]
#[derive(Debug, Clone)]
pub(super) struct TreeCompletionContext {
    pub(super) cut_byte: usize,
    pub(super) restart_byte: usize,
    pub(super) suffix_consistent_cmavo: BTreeSet<Cmavo>,
    pub(super) open_constructs: Vec<OpenConstructCandidate>,
}

impl TreeCompletionContext {
    #[requires(cut_byte <= document_byte_len)]
    #[ensures(ret.cut_byte == cut_byte)]
    #[ensures(ret.restart_byte <= cut_byte)]
    pub(super) fn from_parse(
        parse: &SyntaxRecoveryParse,
        cut_byte: usize,
        document_byte_len: usize,
    ) -> Self {
        let collector = TreeContextCollector::new(cut_byte, document_byte_len);
        let collector = match parse.as_data() {
            SyntaxRecoveryParseData::Valid { parse } => {
                let mut visitor = ValidTreeContextVisitor { collector };
                generated_model::TreeNode::visit_in_order(&parse.parse_tree, &mut visitor);
                visitor.collector
            }
            SyntaxRecoveryParseData::Recovered { parse } => {
                let mut visitor = RecoveredTreeContextVisitor { collector };
                generated_model::recovered::TreeNode::visit_in_order(
                    &parse.parse_tree,
                    &mut visitor,
                );
                let mut skipped_visitor = RecoveredSkippedTokenVisitor {
                    cut_byte,
                    text_depth: 0,
                    anchors: Vec::new(),
                    suffix_consistent_cmavo: BTreeSet::new(),
                };
                generated_model::recovered::TreeWalkable::walk_with(
                    parse.parse_tree.as_ref(),
                    &mut skipped_visitor,
                );
                visitor.collector.skipped_anchors = skipped_visitor.anchors;
                visitor
                    .collector
                    .suffix_consistent_cmavo
                    .extend(skipped_visitor.suffix_consistent_cmavo);
                visitor.collector
            }
        };
        collector.finish()
    }
}

#[invariant(byte_start <= byte_end)]
#[derive(Debug, Clone, Copy)]
struct TreeBounds {
    byte_start: usize,
    byte_end: usize,
}

impl TreeBounds {
    #[requires(span.byte_start <= span.byte_end)]
    #[ensures(ret.byte_start == span.byte_start && ret.byte_end == span.byte_end)]
    fn from_span(span: &SourceSpan) -> Self {
        new!(TreeBounds {
            byte_start: span.byte_start,
            byte_end: span.byte_end,
        })
    }

    #[requires(span.byte_start <= span.byte_end)]
    #[ensures(ret.byte_start <= self.byte_start && ret.byte_start <= span.byte_start)]
    #[ensures(ret.byte_end >= self.byte_end && ret.byte_end >= span.byte_end)]
    fn including(&self, span: &SourceSpan) -> Self {
        new!(TreeBounds {
            byte_start: self.byte_start.min(span.byte_start),
            byte_end: self.byte_end.max(span.byte_end),
        })
    }

    #[requires(other.byte_start <= other.byte_end)]
    #[ensures(ret.byte_start <= self.byte_start && ret.byte_start <= other.byte_start)]
    #[ensures(ret.byte_end >= self.byte_end && ret.byte_end >= other.byte_end)]
    fn including_bounds(&self, other: Self) -> Self {
        new!(TreeBounds {
            byte_start: self.byte_start.min(other.byte_start),
            byte_end: self.byte_end.max(other.byte_end),
        })
    }

    #[requires(true)]
    #[ensures(ret == (self.byte_start <= cut && cut <= self.byte_end))]
    fn covers(self, cut: usize) -> bool {
        self.byte_start <= cut && cut <= self.byte_end
    }
}

#[invariant(true)]
#[derive(Debug)]
struct NodeFrame {
    constructor: &'static str,
    bounds: Option<TreeBounds>,
    text: Option<TextFrame>,
    elidable_terminators: BTreeSet<Cmavo>,
}

#[invariant(true)]
#[derive(Debug)]
struct TextFrame {
    depth: usize,
    leading_i_start: Option<usize>,
    statement_starts: Vec<usize>,
}

#[invariant(true)]
#[derive(Debug)]
struct TextRecord {
    depth: usize,
    bounds: TreeBounds,
    statement_starts: Vec<usize>,
}

#[invariant(true)]
#[derive(Debug)]
struct SkippedAnchor {
    depth: usize,
    bounds: TreeBounds,
    restart_byte: usize,
}

#[invariant(true)]
#[derive(Debug)]
struct SequenceFrame {
    boundary_byte: usize,
    bounds: Option<TreeBounds>,
    first_suffix_token_seen: bool,
    suffix_cmavo: Option<Cmavo>,
}

#[invariant(true)]
#[derive(Debug)]
struct TreeContextCollector {
    cut_byte: usize,
    document_byte_len: usize,
    nodes: Vec<NodeFrame>,
    fields: Vec<FieldRef>,
    sequences: Vec<SequenceFrame>,
    texts: Vec<TextRecord>,
    skipped_anchors: Vec<SkippedAnchor>,
    suffix_consistent_cmavo: BTreeSet<Cmavo>,
    open_constructs: BTreeSet<OpenConstructCandidate>,
}

impl TreeContextCollector {
    #[requires(cut_byte <= document_byte_len)]
    #[ensures(ret.cut_byte == cut_byte && ret.document_byte_len == document_byte_len)]
    fn new(cut_byte: usize, document_byte_len: usize) -> Self {
        Self {
            cut_byte,
            document_byte_len,
            nodes: Vec::new(),
            fields: Vec::new(),
            sequences: Vec::new(),
            texts: Vec::new(),
            skipped_anchors: Vec::new(),
            suffix_consistent_cmavo: BTreeSet::new(),
            open_constructs: BTreeSet::new(),
        }
    }

    #[requires(!constructor.is_empty())]
    #[ensures(self.nodes.len() == old(self.nodes.len()) + 1)]
    fn enter_node(&mut self, constructor: &'static str) {
        let text = is_text_root_constructor(constructor).then(|| TextFrame {
            depth: self
                .nodes
                .iter()
                .filter(|frame| frame.text.is_some())
                .count()
                + 1,
            leading_i_start: None,
            statement_starts: Vec::new(),
        });
        self.nodes.push(NodeFrame {
            constructor,
            bounds: None,
            text,
            elidable_terminators: BTreeSet::new(),
        });
    }

    #[requires(!self.nodes.is_empty())]
    #[ensures(self.nodes.len() == old(self.nodes.len()) - 1)]
    fn exit_node(&mut self, constructor: &'static str) {
        let frame = self
            .nodes
            .pop()
            .expect("tree traversal exits every entered node");
        debug_assert_eq!(frame.constructor, constructor);

        if let Some(bounds) = frame.bounds {
            if constructor == "LeadingIStatementSyntax" {
                if let Some(text) = self.enclosing_text_mut() {
                    text.leading_i_start = Some(
                        text.leading_i_start
                            .map_or(bounds.byte_start, |start| start.min(bounds.byte_start)),
                    );
                }
            } else if is_initial_statement_constructor(constructor) {
                let paragraph_start = self
                    .nodes
                    .iter()
                    .rev()
                    .take_while(|ancestor| ancestor.text.is_none())
                    .filter(|ancestor| is_leading_paragraph_constructor(ancestor.constructor))
                    .filter_map(|ancestor| ancestor.bounds)
                    .map(|bounds| bounds.byte_start)
                    .min();
                if let Some(text) = self.enclosing_text_mut() {
                    let restart = paragraph_start
                        .into_iter()
                        .chain(text.leading_i_start)
                        .min()
                        .unwrap_or(bounds.byte_start);
                    text.statement_starts.push(restart);
                }
            } else if is_following_statement_constructor(constructor)
                && let Some(text) = self.enclosing_text_mut()
            {
                text.statement_starts.push(bounds.byte_start);
            }

            if bounds.covers(self.cut_byte) {
                let construct = constructor_label(constructor);
                for cmavo in frame.elidable_terminators {
                    self.open_constructs.insert(new!(OpenConstructCandidate {
                        cmavo,
                        construct: construct.to_owned(),
                    }));
                }
            }

            if let Some(text) = frame.text {
                let bounds = if text.depth == 1 {
                    new!(TreeBounds {
                        byte_start: 0,
                        byte_end: self.document_byte_len,
                    })
                } else {
                    bounds
                };
                self.texts.push(TextRecord {
                    depth: text.depth,
                    bounds,
                    statement_starts: text.statement_starts,
                });
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, field: FieldRef) {
        self.fields.push(field);
    }

    #[requires(!self.fields.is_empty())]
    #[ensures(self.fields.len() == old(self.fields.len()) - 1)]
    fn exit_field(&mut self, field: FieldRef) {
        let entered = self
            .fields
            .pop()
            .expect("tree traversal exits every entered field");
        debug_assert_eq!(entered, field);
    }

    #[requires(!self.nodes.is_empty())]
    #[ensures(true)]
    fn visit_absent_optional_field(&mut self, field: FieldRef) {
        if let Some(cmavo) = elidable_terminator_for_field(field) {
            self.nodes
                .last_mut()
                .expect("absent fields belong to an entered node")
                .elidable_terminators
                .insert(cmavo);
        }
    }

    #[requires(true)]
    #[ensures(self.sequences.len() == old(self.sequences.len()) + 1)]
    fn enter_sequence(&mut self) {
        let boundary_byte = self
            .nodes
            .last()
            .and_then(|frame| frame.bounds)
            .map_or(0, |bounds| bounds.byte_end);
        self.sequences.push(SequenceFrame {
            boundary_byte,
            bounds: None,
            first_suffix_token_seen: false,
            suffix_cmavo: None,
        });
    }

    #[requires(!self.sequences.is_empty())]
    #[ensures(self.sequences.len() == old(self.sequences.len()) - 1)]
    fn exit_sequence(&mut self) {
        let sequence = self
            .sequences
            .pop()
            .expect("tree traversal exits every entered sequence");
        if sequence.boundary_byte <= self.cut_byte
            && sequence
                .bounds
                .is_some_and(|bounds| self.cut_byte <= bounds.byte_end)
            && let Some(cmavo) = sequence.suffix_cmavo
        {
            self.suffix_consistent_cmavo.insert(cmavo);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_token(&mut self, token: &Token) {
        let Some(token_bounds) = token_bounds(token) else {
            return;
        };
        self.include_bounds(token_bounds);

        if token_bounds.byte_end > self.cut_byte {
            for sequence in &mut self.sequences {
                if !sequence.first_suffix_token_seen {
                    sequence.first_suffix_token_seen = true;
                    sequence.suffix_cmavo = token.cmavo();
                }
            }
        }

        let Some(field) = self.fields.last().copied() else {
            return;
        };
        let Some(terminator) = elidable_terminator_for_field(field) else {
            return;
        };
        if token.cmavo() == Some(terminator)
            && token_bounds.byte_end > self.cut_byte
            && let Some(node) = self.nodes.last_mut()
        {
            node.elidable_terminators.insert(terminator);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_recovered_error<E: RecoveryItemState + Serialize>(&mut self, item: &E) {
        let mut saw_suffix_span = false;
        item.visit_source_spans(&mut |span| {
            self.include_bounds(TreeBounds::from_span(span));
            saw_suffix_span |= span.byte_end > self.cut_byte;
        });
        if saw_suffix_span {
            for sequence in &mut self.sequences {
                if !sequence.first_suffix_token_seen {
                    sequence.first_suffix_token_seen = true;
                }
            }
        }
    }

    #[requires(bounds.byte_start <= bounds.byte_end)]
    #[ensures(true)]
    fn include_bounds(&mut self, bounds: TreeBounds) {
        for node in &mut self.nodes {
            if let Some(existing) = &mut node.bounds {
                *existing = existing.including_bounds(bounds);
            } else {
                node.bounds = Some(bounds);
            }
        }
        for sequence in &mut self.sequences {
            if let Some(existing) = &mut sequence.bounds {
                *existing = existing.including_bounds(bounds);
            } else {
                sequence.bounds = Some(bounds);
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn enclosing_text_mut(&mut self) -> Option<&mut TextFrame> {
        self.nodes
            .iter_mut()
            .rev()
            .find_map(|frame| frame.text.as_mut())
    }

    #[requires(true)]
    #[ensures(ret.cut_byte == self.cut_byte && ret.restart_byte <= ret.cut_byte)]
    fn finish(mut self) -> TreeCompletionContext {
        let structured = self
            .texts
            .iter_mut()
            .filter(|text| text.bounds.covers(self.cut_byte))
            .max_by_key(|text| text.depth)
            .map(|text| {
                text.statement_starts.sort_unstable();
                let restart = text
                    .statement_starts
                    .iter()
                    .copied()
                    .take_while(|start| *start <= self.cut_byte)
                    .last()
                    .unwrap_or(text.bounds.byte_start);
                (text.depth, restart)
            });
        let skipped = self
            .skipped_anchors
            .iter()
            .filter(|anchor| anchor.bounds.covers(self.cut_byte))
            .map(|anchor| (anchor.depth, anchor.restart_byte))
            .max();
        let restart_byte = structured
            .into_iter()
            .chain(skipped)
            .max()
            .map(|(_, restart)| restart)
            .unwrap_or(0)
            .min(self.cut_byte);
        new!(TreeCompletionContext {
            cut_byte: self.cut_byte,
            restart_byte,
            suffix_consistent_cmavo: self.suffix_consistent_cmavo,
            open_constructs: self.open_constructs.into_iter().collect(),
        })
    }
}

#[invariant(true)]
struct RecoveredSkippedTokenVisitor {
    cut_byte: usize,
    text_depth: usize,
    anchors: Vec<SkippedAnchor>,
    suffix_consistent_cmavo: BTreeSet<Cmavo>,
}

#[invariant(true)]
struct SkippedTextFrame {
    closer: Option<Cmavo>,
    restart_byte: Option<usize>,
    boundary_group_start: Option<usize>,
}

impl RecoveredSkippedTokenVisitor {
    #[requires(true)]
    #[ensures(self.text_depth == old(self.text_depth))]
    fn walk_text(&mut self, descend: impl FnOnce(&mut Self)) {
        self.text_depth += 1;
        descend(self);
        self.text_depth -= 1;
    }

    #[requires(true)]
    #[ensures(true)]
    fn record_skipped_tokens(&mut self, tokens: &[Token]) {
        let mut bounds: Option<TreeBounds> = None;
        let mut texts = vec![SkippedTextFrame {
            closer: None,
            restart_byte: None,
            boundary_group_start: None,
        }];
        let mut saw_suffix_token = false;
        for token in tokens {
            let Some(token_bounds) = token_bounds(token) else {
                continue;
            };
            bounds = Some(match bounds {
                Some(existing) => existing.including_bounds(token_bounds),
                None => token_bounds,
            });
            if token_bounds.byte_end > self.cut_byte && !saw_suffix_token {
                saw_suffix_token = true;
                if let Some(cmavo) = token.cmavo() {
                    self.suffix_consistent_cmavo.insert(cmavo);
                }
            }
            if token_bounds.byte_start >= self.cut_byte {
                continue;
            }
            if texts
                .last()
                .and_then(|text| text.closer)
                .is_some_and(|closer| token.is_cmavo(closer))
            {
                texts.pop();
                continue;
            }
            if let Some(closer) = token.cmavo().and_then(text_closer_for_opener) {
                texts.push(SkippedTextFrame {
                    closer: Some(closer),
                    restart_byte: Some(token_bounds.byte_end),
                    boundary_group_start: None,
                });
                continue;
            }
            let text = texts
                .last_mut()
                .expect("the root skipped-token text frame is never popped");
            if token.is_cmavo(Cmavo::I) {
                text.boundary_group_start = Some(token_bounds.byte_start);
                text.restart_byte = text.boundary_group_start;
            } else if token.is_selmaho(jbotci_morphology::Selmaho::Niho) {
                let start = text
                    .boundary_group_start
                    .get_or_insert(token_bounds.byte_start);
                text.restart_byte = Some(*start);
            } else {
                text.boundary_group_start = None;
            }
        }
        let text = texts
            .last()
            .expect("the root skipped-token text frame is never popped");
        if let (Some(bounds), Some(restart_byte)) = (bounds, text.restart_byte) {
            self.anchors.push(SkippedAnchor {
                depth: self.text_depth.max(1) + texts.len() - 1,
                bounds,
                restart_byte,
            });
        }
    }
}

impl<'tree> generated_model::recovered::TreeWalker<'tree> for RecoveredSkippedTokenVisitor {
    #[requires(true)]
    #[ensures(true)]
    fn walk_regular_text(&mut self, node: &'tree generated_model::recovered::RegularTextSyntax) {
        self.walk_text(|visitor| {
            generated_model::recovered::walk::regular_text(visitor, node);
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_explicit_xauha_lohoi_text(
        &mut self,
        node: &'tree generated_model::recovered::ExplicitXauhaLohoiTextSyntax,
    ) {
        self.walk_text(|visitor| {
            generated_model::recovered::walk::explicit_xauha_lohoi_text(visitor, node);
        });
    }

    #[requires(true)]
    #[ensures(true)]
    fn walk_recovered_error(&mut self, item: &'tree SyntaxRecoveryItem) {
        if let Some(tokens) = item.skipped_tokens() {
            self.record_skipped_tokens(tokens);
        }
    }
}

#[invariant(true)]
struct ValidTreeContextVisitor {
    collector: TreeContextCollector,
}

impl<'tree> TreeVisitor<'tree> for ValidTreeContextVisitor {
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
    fn enter_field(&mut self, field: FieldRef) {
        self.collector.enter_field(field);
    }

    #[requires(!self.collector.fields.is_empty())]
    #[ensures(true)]
    fn exit_field(&mut self, field: FieldRef) {
        self.collector.exit_field(field);
    }

    #[requires(!self.collector.nodes.is_empty())]
    #[ensures(true)]
    fn visit_absent_optional_field(&mut self, field: FieldRef) {
        self.collector.visit_absent_optional_field(field);
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {
        self.collector.enter_sequence();
    }

    #[requires(!self.collector.sequences.is_empty())]
    #[ensures(true)]
    fn exit_sequence(&mut self) {
        self.collector.exit_sequence();
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let generated_model::AtomRef::Token(token) = atom;
        self.collector.visit_token(token);
    }
}

#[invariant(true)]
struct RecoveredTreeContextVisitor {
    collector: TreeContextCollector,
}

impl<'tree> TreeVisitor<'tree> for RecoveredTreeContextVisitor {
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
    fn enter_field(&mut self, field: FieldRef) {
        self.collector.enter_field(field);
    }

    #[requires(!self.collector.fields.is_empty())]
    #[ensures(true)]
    fn exit_field(&mut self, field: FieldRef) {
        self.collector.exit_field(field);
    }

    #[requires(!self.collector.nodes.is_empty())]
    #[ensures(true)]
    fn visit_absent_optional_field(&mut self, field: FieldRef) {
        self.collector.visit_absent_optional_field(field);
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {
        self.collector.enter_sequence();
    }

    #[requires(!self.collector.sequences.is_empty())]
    #[ensures(true)]
    fn exit_sequence(&mut self) {
        self.collector.exit_sequence();
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        let generated_model::recovered::AtomRef::Token(token) = atom;
        self.collector.visit_token(token);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_recovered_error<E: RecoveryItemState + Serialize>(&mut self, item: &'tree E) {
        self.collector.visit_recovered_error(item);
    }
}

#[requires(true)]
#[ensures(true)]
fn is_text_root_constructor(constructor: &str) -> bool {
    matches!(constructor, "RegularText" | "ExplicitXauhaLohoiText")
}

#[requires(true)]
#[ensures(true)]
fn is_initial_statement_constructor(constructor: &str) -> bool {
    constructor == "InitialParagraphStatementSyntax"
}

#[requires(true)]
#[ensures(true)]
fn is_following_statement_constructor(constructor: &str) -> bool {
    constructor == "FollowingParagraphStatementSyntax"
}

#[requires(true)]
#[ensures(true)]
fn is_leading_paragraph_constructor(constructor: &str) -> bool {
    matches!(
        constructor,
        "INihoParagraph" | "INihoParagraphSyntax" | "NihoParagraphSyntax"
    )
}

#[requires(true)]
#[ensures(ret.is_none_or(|cmavo| generated_model::GENERATED_MODEL_ELIDABLE_TERMINATORS.iter().any(|terminator| terminator.field == field.name.unwrap_or_default() && terminator.cmavo == cmavo)))]
fn elidable_terminator_for_field(field: FieldRef) -> Option<Cmavo> {
    let name = field.name?;
    generated_model::GENERATED_MODEL_ELIDABLE_TERMINATORS
        .iter()
        .find(|terminator| terminator.field == name)
        .map(|terminator| terminator.cmavo)
}

#[requires(!constructor.is_empty())]
#[ensures(!ret.is_empty())]
fn constructor_label(constructor: &str) -> &str {
    let constructor = constructor.strip_suffix("Syntax").unwrap_or(constructor);
    generated_model::GENERATED_MODEL_CONSTRUCTOR_LABELS
        .iter()
        .find_map(|(candidate, label)| (*candidate == constructor).then_some(*label))
        .unwrap_or("syntax construct")
}

#[requires(true)]
#[ensures(ret == match opener {
    Cmavo::Lu => Some(Cmavo::Lihu),
    Cmavo::Tuhe => Some(Cmavo::Tuhu),
    Cmavo::To => Some(Cmavo::Toi),
    _ => None,
})]
fn text_closer_for_opener(opener: Cmavo) -> Option<Cmavo> {
    match opener {
        Cmavo::Lu => Some(Cmavo::Lihu),
        Cmavo::Tuhe => Some(Cmavo::Tuhu),
        Cmavo::To => Some(Cmavo::Toi),
        _ => None,
    }
}

#[requires(true)]
#[ensures(ret.is_none_or(|bounds| bounds.byte_start <= bounds.byte_end))]
fn token_bounds(token: &Token) -> Option<TreeBounds> {
    let range = token.core_word().byte_range()?;
    Some(new!(TreeBounds {
        byte_start: range.start,
        byte_end: range.end,
    }))
}
