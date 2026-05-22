//! Generic syntax-tree metadata for output formats.

#[allow(unused_imports)]
use bityzba::{contract_trait, ensures, invariant, requires};
use jbotci_morphology::{Word, WordLike};
use smallvec::SmallVec;
use vec1::{Vec1, smallvec_v1::SmallVec1};

use crate::ast::{AtomRef, NodeRef, TextSyntax, TreeNode as AstTreeNode};
use crate::{Indicator, WithIndicators};
use jbotci_tree::{FieldRef, TreeVisitor};

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct SyntaxTreeEntry {
    pub label: Option<&'static str>,
    pub value: SyntaxTreeValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub struct SyntaxTreeNode {
    pub constructor: &'static str,
    pub entries: Vec<SyntaxTreeEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[invariant(true)]
pub enum SyntaxTreeValue {
    Node(SyntaxTreeNode),
    Collection(Vec<SyntaxTreeValue>),
    Word(WithIndicators<WordLike>),
    Text(String),
}

impl SyntaxTreeValue {
    #[requires(!constructor.is_empty())]
    #[ensures(true)]
    pub fn node(constructor: &'static str, entries: Vec<SyntaxTreeEntry>) -> Self {
        Self::Node(SyntaxTreeNode {
            constructor,
            entries,
        })
    }

    #[requires(!constructor.is_empty())]
    #[ensures(true)]
    pub fn unit(constructor: &'static str) -> Self {
        Self::node(constructor, Vec::new())
    }
}

#[contract_trait]
pub trait SyntaxTree {
    #[requires(true)]
    #[ensures(true)]
    fn syntax_tree_value(&self) -> Option<SyntaxTreeValue>;
}

#[requires(true)]
#[ensures(true)]
pub fn push_labelled_entry<T: SyntaxTree + ?Sized>(
    entries: &mut Vec<SyntaxTreeEntry>,
    label: &'static str,
    value: &T,
) {
    if let Some(value) = value.syntax_tree_value() {
        entries.push(SyntaxTreeEntry {
            label: Some(label),
            value,
        });
    }
}

#[requires(true)]
#[ensures(true)]
pub fn push_primary_entry<T: SyntaxTree + ?Sized>(entries: &mut Vec<SyntaxTreeEntry>, value: &T) {
    let Some(value) = value.syntax_tree_value() else {
        return;
    };
    match value {
        SyntaxTreeValue::Collection(items) => {
            entries.extend(
                items
                    .into_iter()
                    .map(|value| SyntaxTreeEntry { label: None, value }),
            );
        }
        value => entries.push(SyntaxTreeEntry { label: None, value }),
    }
}

#[contract_trait]
impl SyntaxTree for Word {
    #[requires(true)]
    #[ensures(true)]
    fn syntax_tree_value(&self) -> Option<SyntaxTreeValue> {
        Some(SyntaxTreeValue::Word(WithIndicators::bare(WordLike::bare(
            self.clone(),
        ))))
    }
}

#[contract_trait]
impl SyntaxTree for WordLike {
    #[requires(true)]
    #[ensures(true)]
    fn syntax_tree_value(&self) -> Option<SyntaxTreeValue> {
        Some(SyntaxTreeValue::Word(WithIndicators::bare(self.clone())))
    }
}

#[contract_trait]
impl SyntaxTree for WithIndicators<WordLike> {
    #[requires(true)]
    #[ensures(true)]
    fn syntax_tree_value(&self) -> Option<SyntaxTreeValue> {
        match self {
            WithIndicators::Bare(word_like) => word_like.syntax_tree_value(),
            WithIndicators::Emphasized { bahe, word_like } => {
                let mut entries = Vec::new();
                push_labelled_entry(&mut entries, "bahe", bahe.as_ref());
                push_primary_entry(&mut entries, word_like.as_ref());
                Some(SyntaxTreeValue::node("Emphasized", entries))
            }
            WithIndicators::WithIndicator {
                base,
                indicator,
                nai,
            } => {
                let mut entries = Vec::new();
                push_primary_entry(&mut entries, base.as_ref());
                push_labelled_entry(&mut entries, "indicator", indicator.as_ref());
                push_labelled_entry(&mut entries, "nai", nai);
                Some(SyntaxTreeValue::node("WithIndicator", entries))
            }
        }
    }
}

#[contract_trait]
impl SyntaxTree for Indicator {
    #[requires(true)]
    #[ensures(true)]
    fn syntax_tree_value(&self) -> Option<SyntaxTreeValue> {
        let mut entries = Vec::new();
        push_primary_entry(&mut entries, self.indicator.as_ref());
        push_labelled_entry(&mut entries, "nai", &self.nai);
        Some(SyntaxTreeValue::node("Indicator", entries))
    }
}

#[contract_trait]
impl<T: SyntaxTree + ?Sized> SyntaxTree for Box<T> {
    #[requires(true)]
    #[ensures(true)]
    fn syntax_tree_value(&self) -> Option<SyntaxTreeValue> {
        (**self).syntax_tree_value()
    }
}

#[contract_trait]
impl<T: SyntaxTree> SyntaxTree for Option<T> {
    #[requires(true)]
    #[ensures(true)]
    fn syntax_tree_value(&self) -> Option<SyntaxTreeValue> {
        self.as_ref().and_then(SyntaxTree::syntax_tree_value)
    }
}

#[contract_trait]
impl<T: SyntaxTree> SyntaxTree for Vec<T> {
    #[requires(true)]
    #[ensures(true)]
    fn syntax_tree_value(&self) -> Option<SyntaxTreeValue> {
        collection_value(self.iter())
    }
}

#[contract_trait]
impl<T: SyntaxTree> SyntaxTree for Vec1<T> {
    #[requires(true)]
    #[ensures(true)]
    fn syntax_tree_value(&self) -> Option<SyntaxTreeValue> {
        collection_value(self.iter())
    }
}

#[contract_trait]
impl<A> SyntaxTree for SmallVec<A>
where
    A: smallvec::Array,
    A::Item: SyntaxTree,
{
    #[requires(true)]
    #[ensures(true)]
    fn syntax_tree_value(&self) -> Option<SyntaxTreeValue> {
        collection_value(self.iter())
    }
}

#[contract_trait]
impl<A> SyntaxTree for SmallVec1<A>
where
    A: smallvec::Array,
    A::Item: SyntaxTree,
{
    #[requires(true)]
    #[ensures(true)]
    fn syntax_tree_value(&self) -> Option<SyntaxTreeValue> {
        collection_value(self.iter())
    }
}

#[requires(true)]
#[ensures(true)]
fn collection_value<'a, T: SyntaxTree + 'a>(
    items: impl Iterator<Item = &'a T>,
) -> Option<SyntaxTreeValue> {
    let items = items
        .filter_map(SyntaxTree::syntax_tree_value)
        .collect::<Vec<_>>();
    (!items.is_empty()).then_some(SyntaxTreeValue::Collection(items))
}

#[contract_trait]
impl SyntaxTree for String {
    #[requires(true)]
    #[ensures(true)]
    fn syntax_tree_value(&self) -> Option<SyntaxTreeValue> {
        Some(SyntaxTreeValue::Text(self.clone()))
    }
}

#[contract_trait]
impl SyntaxTree for TextSyntax {
    #[requires(true)]
    #[ensures(true)]
    fn syntax_tree_value(&self) -> Option<SyntaxTreeValue> {
        let mut builder = SyntaxTreeBuilder::default();
        self.visit_in_order(&mut builder);
        builder.finish()
    }
}

#[derive(Debug, Default)]
#[invariant(true)]
struct SyntaxTreeBuilder {
    stack: Vec<SyntaxTreeFrame>,
    fields: Vec<FieldRef>,
    root: Option<SyntaxTreeValue>,
}

impl SyntaxTreeBuilder {
    #[requires(true)]
    #[ensures(true)]
    fn finish(self) -> Option<SyntaxTreeValue> {
        self.root
    }

    #[requires(true)]
    #[ensures(true)]
    fn push_value(&mut self, value: SyntaxTreeValue) {
        let Some(frame) = self.stack.last_mut() else {
            self.root = Some(value);
            return;
        };
        if let SyntaxTreeFrame::Collection { items } = frame {
            items.push(value);
            return;
        }
        let field = self.fields.last().copied();
        if field.is_some_and(|field| field.primary || field.name.is_none()) {
            match value {
                SyntaxTreeValue::Collection(items) => {
                    frame.entries_mut().extend(
                        items
                            .into_iter()
                            .map(|value| SyntaxTreeEntry { label: None, value }),
                    );
                }
                value => frame
                    .entries_mut()
                    .push(SyntaxTreeEntry { label: None, value }),
            }
        } else {
            frame.entries_mut().push(SyntaxTreeEntry {
                label: field.and_then(|field| field.name),
                value,
            });
        }
    }
}

#[derive(Debug)]
#[invariant(true)]
enum SyntaxTreeFrame {
    Node {
        constructor: &'static str,
        entries: Vec<SyntaxTreeEntry>,
    },
    Collection {
        items: Vec<SyntaxTreeValue>,
    },
}

impl SyntaxTreeFrame {
    #[requires(matches!(self, Self::Node { .. }))]
    #[ensures(true)]
    fn entries_mut(&mut self) -> &mut Vec<SyntaxTreeEntry> {
        match self {
            Self::Node { entries, .. } => entries,
            Self::Collection { .. } => unreachable!("precondition rejects collection frames"),
        }
    }
}

#[contract_trait]
impl<'tree> TreeVisitor<'tree> for SyntaxTreeBuilder {
    type Node = NodeRef<'tree>;
    type Atom = AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        self.stack.push(SyntaxTreeFrame::Node {
            constructor: node.constructor_name(),
            entries: Vec::new(),
        });
    }

    #[requires(!self.stack.is_empty())]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {
        let frame = self.stack.pop().expect("precondition checked above");
        let SyntaxTreeFrame::Node {
            constructor,
            entries,
        } = frame
        else {
            unreachable!("node exit must close a node frame");
        };
        self.push_value(SyntaxTreeValue::node(
            syntax_constructor_name(constructor),
            entries,
        ));
    }

    #[requires(true)]
    #[ensures(self.fields.len() == old(self.fields.len()) + 1)]
    fn enter_field(&mut self, field: FieldRef) {
        self.fields.push(field);
    }

    #[requires(!self.fields.is_empty())]
    #[ensures(self.fields.len() == old(self.fields.len()) - 1)]
    fn exit_field(&mut self, _field: FieldRef) {
        self.fields.pop();
    }

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {
        self.stack
            .push(SyntaxTreeFrame::Collection { items: Vec::new() });
    }

    #[requires(matches!(self.stack.last(), Some(SyntaxTreeFrame::Collection { .. })))]
    #[ensures(true)]
    fn exit_sequence(&mut self) {
        let frame = self.stack.pop().expect("precondition checked above");
        let SyntaxTreeFrame::Collection { items } = frame else {
            unreachable!("precondition checked above");
        };
        if !items.is_empty() {
            self.push_value(SyntaxTreeValue::Collection(items));
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, atom: Self::Atom) {
        if let Some(value) = atom_syntax_tree_value(atom) {
            self.push_value(value);
        }
    }
}

#[requires(true)]
#[ensures(true)]
fn atom_syntax_tree_value(atom: AtomRef<'_>) -> Option<SyntaxTreeValue> {
    match atom {
        AtomRef::WithIndicatorsWordLike(word) => word.syntax_tree_value(),
        AtomRef::Word(word) => word.syntax_tree_value(),
    }
}

#[requires(true)]
#[ensures(!ret.ends_with("Syntax"))]
fn syntax_constructor_name(constructor: &'static str) -> &'static str {
    constructor.strip_suffix("Syntax").unwrap_or(constructor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SyntaxTree;
    #[allow(unused_imports)]
    use bityzba::{ensures, invariant, requires};

    #[derive(SyntaxTree)]
    #[invariant(true)]
    #[allow(dead_code)]
    struct TestNode<T> {
        marker: Option<T>,
        #[tree(primary)]
        payload: Vec<T>,
        #[tree(skip)]
        ignored: T,
    }

    #[derive(SyntaxTree)]
    #[invariant(true)]
    enum TestEnum<T> {
        Tuple(T),
        Named {
            marker: T,
            #[tree(primary)]
            payload: T,
        },
        Unit,
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn derive_marks_primary_and_skipped_struct_fields() {
        let node = TestNode {
            marker: Some("marker".to_owned()),
            payload: vec!["left".to_owned(), "right".to_owned()],
            ignored: "ignored".to_owned(),
        };

        let Some(SyntaxTreeValue::Node(node)) = node.syntax_tree_value() else {
            panic!("expected node");
        };
        assert_eq!(node.constructor, "TestNode");
        assert_eq!(node.entries.len(), 3);
        assert_eq!(node.entries[0].label, Some("marker"));
        assert_eq!(node.entries[1].label, None);
        assert_eq!(node.entries[2].label, None);
        assert!(
            !node
                .entries
                .iter()
                .any(|entry| entry.label == Some("ignored"))
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn derive_handles_tuple_named_and_unit_variants() {
        let Some(SyntaxTreeValue::Node(tuple)) =
            TestEnum::Tuple("payload".to_owned()).syntax_tree_value()
        else {
            panic!("expected tuple node");
        };
        assert_eq!(tuple.constructor, "Tuple");
        assert_eq!(tuple.entries.len(), 1);
        assert_eq!(tuple.entries[0].label, None);

        let Some(SyntaxTreeValue::Node(named)) = (TestEnum::Named {
            marker: "marker".to_owned(),
            payload: "payload".to_owned(),
        })
        .syntax_tree_value() else {
            panic!("expected named node");
        };
        assert_eq!(named.entries[0].label, Some("marker"));
        assert_eq!(named.entries[1].label, None);

        let Some(SyntaxTreeValue::Node(unit)) = TestEnum::<String>::Unit.syntax_tree_value() else {
            panic!("expected unit node");
        };
        assert_eq!(unit.constructor, "Unit");
        assert!(unit.entries.is_empty());
    }
}
