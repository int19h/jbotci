#![recursion_limit = "1024"]

use std::{
    cell::{Cell, RefCell},
    ops::Range,
};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_morphology::WordLike;
use jbotci_syntax::{
    ParseOptions, generated_model, parse_syntax_tree_recovered_with_source_and_options,
    parse_syntax_tree_with_source_and_options,
};
use jbotci_tree::{RecoveredFieldState, RecoveryItemKind, RecoveryItemState, TreeVisitor};

#[invariant(active_paragraph_statement_starts.borrow().iter().all(|start| *start <= event_index.get()))]
#[invariant(paragraph_statement_event_ranges.borrow().iter().all(|range| range.start <= range.end && range.end <= event_index.get()))]
#[invariant(invalid_recovery_event_indices.borrow().iter().all(|index| *index < event_index.get()))]
#[invariant(invalid_recovery_spans.borrow().iter().all(|(start, end)| start <= end))]
#[derive(Default)]
struct RecoveryStructureVisitor {
    event_index: Cell<usize>,
    active_paragraph_statement_starts: RefCell<Vec<usize>>,
    paragraph_statement_event_ranges: RefCell<Vec<Range<usize>>>,
    invalid_recovery_event_indices: RefCell<Vec<usize>>,
    invalid_recovery_spans: RefCell<Vec<(usize, usize)>>,
}

impl<'tree> TreeVisitor<'tree> for RecoveryStructureVisitor {
    type Node = generated_model::recovered::NodeRef<'tree>;
    type Atom = generated_model::recovered::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        if matches!(
            node,
            generated_model::recovered::NodeRef::InitialParagraphStatementSyntax(_)
                | generated_model::recovered::NodeRef::FollowingParagraphStatementSyntax(_)
                | generated_model::recovered::NodeRef::StatementBaseSyntaxBridiStatement(_)
                | generated_model::recovered::NodeRef::SimpleIConnectiveStatementTailSyntax(_)
        ) {
            self.active_paragraph_statement_starts
                .borrow_mut()
                .push(self.event_index.get());
            self.event_index.set(self.event_index.get() + 1);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, node: Self::Node) {
        if matches!(
            node,
            generated_model::recovered::NodeRef::InitialParagraphStatementSyntax(_)
                | generated_model::recovered::NodeRef::FollowingParagraphStatementSyntax(_)
                | generated_model::recovered::NodeRef::StatementBaseSyntaxBridiStatement(_)
                | generated_model::recovered::NodeRef::SimpleIConnectiveStatementTailSyntax(_)
        ) {
            let start = self
                .active_paragraph_statement_starts
                .borrow_mut()
                .pop()
                .expect("bridi statement exits after its matching entry");
            self.paragraph_statement_event_ranges
                .borrow_mut()
                .push(start..self.event_index.get());
            self.event_index.set(self.event_index.get() + 1);
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, _atom: Self::Atom) {
        self.event_index.set(self.event_index.get() + 1);
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_recovered_error<E>(&mut self, item: &'tree E)
    where
        E: RecoveryItemState + serde::Serialize,
    {
        let invalid = item.recovery_item_kind() == RecoveryItemKind::Invalid;
        if invalid {
            self.invalid_recovery_event_indices
                .borrow_mut()
                .push(self.event_index.get());
        }
        self.event_index.set(self.event_index.get() + 1);
        if invalid {
            item.visit_source_spans(&mut |span| {
                self.invalid_recovery_spans
                    .borrow_mut()
                    .push((span.byte_start, span.byte_end));
            });
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn natural_stop_preserves_statements_around_i_anchor() {
    for (source, skipped) in [
        ("mi tarti li ka broda i do", "li ka broda"),
        (
            "cadga fa lo nu ro lo prenu goi ko'a cu troci lo nu ko'a tarti li ka ce'u xendo ije cnikansa ro lo jmive ta'i lo racli",
            "li ka ce'u xendo",
        ),
    ] {
        assert_natural_stop_recovery(source, skipped, true);
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn natural_stop_preserves_statement_before_eof_skip() {
    for (source, skipped) in [
        ("mi tarti li ka broda", "ka broda"),
        (
            "cadga fa lo nu ro lo prenu goi ko'a cu troci lo nu ko'a tarti li ka ce'u xendo je cnikansa ro lo jmive ta'i lo racli",
            "li ka ce'u xendo",
        ),
    ] {
        assert_natural_stop_recovery(source, skipped, false);
    }
}

#[requires(!source.is_empty())]
#[requires(!skipped_text.is_empty())]
#[requires(source.contains(skipped_text))]
#[ensures(true)]
fn assert_natural_stop_recovery(
    source: &str,
    skipped_text: &str,
    expect_following_statement: bool,
) {
    let words = jbotci_morphology::segment_words_with_modifiers(source).expect("valid morphology");
    let options = ParseOptions::default();
    let strict_error = parse_syntax_tree_with_source_and_options(&words, source, &options)
        .expect_err("strict syntax rejects li followed by ka");
    let recovered = parse_syntax_tree_recovered_with_source_and_options(&words, source, &options);

    assert_eq!(recovered.errors, vec![strict_error]);
    assert!(recovered.parse_tree.recovery_error_slots() >= 1);

    let skip_start = source
        .find(skipped_text)
        .expect("required skip text occurs in source");
    let skip_end = skip_start + skipped_text.len();
    let expected_recovery_spans = words
        .iter()
        .filter_map(WordLike::byte_range)
        .filter(|range| range.start >= skip_start && range.end <= skip_end)
        .map(|range| (range.start, range.end))
        .collect::<Vec<_>>();

    let mut visitor = RecoveryStructureVisitor::default();
    generated_model::recovered::TreeNode::visit_in_order(
        recovered.parse_tree.as_ref(),
        &mut visitor,
    );
    assert_eq!(
        *visitor.invalid_recovery_spans.borrow(),
        expected_recovery_spans
    );
    assert_eq!(visitor.invalid_recovery_event_indices.borrow().len(), 1);
    let recovery_event = visitor.invalid_recovery_event_indices.borrow()[0];
    assert!(
        visitor
            .paragraph_statement_event_ranges
            .borrow()
            .iter()
            .any(|range| range.start < recovery_event),
        "the initial statement must be a structural tree node spanning content before the skip for {source:?}"
    );
    assert_eq!(
        visitor
            .paragraph_statement_event_ranges
            .borrow()
            .iter()
            .any(|range| range.start > recovery_event),
        expect_following_statement,
        "following-statement placement mismatch for {source:?}"
    );
}
