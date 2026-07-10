//! Generic ordered tree traversal support.
//!
//! `tree_model!` generates two traversal APIs for each model. `TreeNode` plus
//! [`TreeVisitor`] provides a flat in-order event stream with node, field,
//! sequence, chain, atom, and recovery events; use it for indexing, span
//! collection, rendering, and other scans that do not need grammar-directed
//! control flow. The macro also generates a model-specific recursive
//! `TreeWalker<'tree>` trait and `walk` module in the expanded model. That
//! walker follows the same child order as `TreeNode::visit_in_order`, but its
//! default-descent methods can be overridden and can call the public `walk::*`
//! free functions before, after, or between pass-specific logic.

extern crate self as jbotci_tree;

use std::{fmt, sync::Arc};

#[allow(unused_imports)]
use bityzba::{contract_trait, data, ensures, invariant, new, requires};
use serde::{Deserialize, Serialize};
use vec1::Vec1;

pub use jbotci_tree_macros::tree_model;

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chain<E, Links> {
    pub first: E,
    pub links: Links,
}

impl<E, Links> Chain<E, Links> {
    #[requires(true)]
    #[ensures(true)]
    pub fn new(first: E, links: Links) -> Self {
        Self { first, links }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecoveryItemKind {
    Missing,
    Invalid,
}

#[contract_trait]
pub trait RecoveredFieldState {
    #[requires(true)]
    #[ensures(true)]
    fn recovery_error_slots(&self) -> usize;

    #[requires(true)]
    #[ensures(true)]
    fn missing_error_slots(&self) -> usize {
        0
    }

    #[requires(true)]
    #[ensures(ret == self.recovery_error_slots().saturating_sub(self.missing_error_slots()))]
    fn invalid_error_slots(&self) -> usize {
        self.recovery_error_slots()
            .saturating_sub(self.missing_error_slots())
    }

    #[requires(true)]
    #[ensures(ret == self.missing_error_slots())]
    fn unconsumed_missing_error_slots(&self) -> usize {
        self.missing_error_slots()
    }

    #[requires(true)]
    #[ensures(ret == (self.recovery_error_slots() > 0 && self.recovery_error_slots() == self.missing_error_slots()))]
    fn is_unconsumed_missing_error(&self) -> bool {
        let error_slots = self.recovery_error_slots();
        error_slots > 0 && error_slots == self.missing_error_slots()
    }
}

#[contract_trait]
pub trait RecoveryItemState {
    #[requires(true)]
    #[ensures(true)]
    fn recovery_item_kind(&self) -> RecoveryItemKind;

    #[requires(true)]
    #[ensures(true)]
    fn visit_source_spans(&self, _visitor: &mut dyn FnMut(&jbotci_source::SourceSpan)) {}

    #[requires(true)]
    #[ensures(true)]
    fn recovery_error_index(&self) -> Option<usize> {
        None
    }

    #[requires(true)]
    #[ensures(ret == (self.recovery_item_kind() == RecoveryItemKind::Missing))]
    fn is_unconsumed_missing_error(&self) -> bool {
        self.recovery_item_kind() == RecoveryItemKind::Missing
    }
}

#[invariant(true)]
#[invariant(::Valid(_) => true)]
#[invariant(::Error(_) => true)]
#[invariant(::Prefix(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum Recovered<T, E> {
    Valid(Box<T>),
    Error(E),
    Prefix(RecoveredPrefix<T, E>),
}

/// A recovered value parsed after one or more prefix recovery items.
///
/// The non-empty error-list invariant is encoded by `Vec1`.
#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveredPrefix<T, E> {
    pub errors: Vec1<E>,
    pub value: Box<T>,
}

#[contract_trait]
impl<T, E> RecoveredFieldState for Recovered<T, E>
where
    T: RecoveredFieldState,
    E: RecoveryItemState,
{
    #[requires(true)]
    #[ensures(true)]
    fn recovery_error_slots(&self) -> usize {
        match self {
            Self::Valid(value) => value.recovery_error_slots(),
            Self::Error(_) => 1,
            Self::Prefix(prefix) => prefix.errors.len() + prefix.value.recovery_error_slots(),
        }
    }

    #[requires(true)]
    #[ensures(true)]
    fn missing_error_slots(&self) -> usize {
        match self {
            Self::Valid(value) => value.missing_error_slots(),
            Self::Error(item) => {
                usize::from(item.recovery_item_kind() == RecoveryItemKind::Missing)
            }
            Self::Prefix(prefix) => {
                prefix
                    .errors
                    .iter()
                    .filter(|item| item.recovery_item_kind() == RecoveryItemKind::Missing)
                    .count()
                    + prefix.value.missing_error_slots()
            }
        }
    }

    #[requires(true)]
    #[ensures(ret == self.recovery_error_slots().saturating_sub(self.missing_error_slots()))]
    fn invalid_error_slots(&self) -> usize {
        match self {
            Self::Valid(value) => value.invalid_error_slots(),
            Self::Error(item) => {
                usize::from(item.recovery_item_kind() == RecoveryItemKind::Invalid)
            }
            Self::Prefix(prefix) => {
                prefix
                    .errors
                    .iter()
                    .filter(|item| item.recovery_item_kind() == RecoveryItemKind::Invalid)
                    .count()
                    + prefix.value.invalid_error_slots()
            }
        }
    }
}

#[contract_trait]
impl<T> RecoveredFieldState for Box<T>
where
    T: RecoveredFieldState,
{
    #[requires(true)]
    #[ensures(true)]
    fn recovery_error_slots(&self) -> usize {
        self.as_ref().recovery_error_slots()
    }

    #[requires(true)]
    #[ensures(ret == self.as_ref().missing_error_slots())]
    fn missing_error_slots(&self) -> usize {
        self.as_ref().missing_error_slots()
    }

    #[requires(true)]
    #[ensures(ret == self.as_ref().invalid_error_slots())]
    fn invalid_error_slots(&self) -> usize {
        self.as_ref().invalid_error_slots()
    }
}

#[contract_trait]
impl<T> RecoveredFieldState for Arc<T>
where
    T: RecoveredFieldState,
{
    #[requires(true)]
    #[ensures(true)]
    fn recovery_error_slots(&self) -> usize {
        self.as_ref().recovery_error_slots()
    }

    #[requires(true)]
    #[ensures(ret == self.as_ref().missing_error_slots())]
    fn missing_error_slots(&self) -> usize {
        self.as_ref().missing_error_slots()
    }

    #[requires(true)]
    #[ensures(ret == self.as_ref().invalid_error_slots())]
    fn invalid_error_slots(&self) -> usize {
        self.as_ref().invalid_error_slots()
    }
}

#[contract_trait]
impl<T> RecoveredFieldState for Option<T>
where
    T: RecoveredFieldState,
{
    #[requires(true)]
    #[ensures(true)]
    fn recovery_error_slots(&self) -> usize {
        self.as_ref()
            .map_or(0, RecoveredFieldState::recovery_error_slots)
    }

    #[requires(true)]
    #[ensures(true)]
    fn missing_error_slots(&self) -> usize {
        self.as_ref()
            .map_or(0, RecoveredFieldState::missing_error_slots)
    }

    #[requires(true)]
    #[ensures(true)]
    fn invalid_error_slots(&self) -> usize {
        self.as_ref()
            .map_or(0, RecoveredFieldState::invalid_error_slots)
    }
}

#[contract_trait]
impl<T> RecoveredFieldState for Vec<T>
where
    T: RecoveredFieldState,
{
    #[requires(true)]
    #[ensures(true)]
    fn recovery_error_slots(&self) -> usize {
        self.iter()
            .map(RecoveredFieldState::recovery_error_slots)
            .sum()
    }

    #[requires(true)]
    #[ensures(true)]
    fn missing_error_slots(&self) -> usize {
        self.iter()
            .map(RecoveredFieldState::missing_error_slots)
            .sum()
    }

    #[requires(true)]
    #[ensures(true)]
    fn invalid_error_slots(&self) -> usize {
        self.iter()
            .map(RecoveredFieldState::invalid_error_slots)
            .sum()
    }
}

#[contract_trait]
impl<A> RecoveredFieldState for smallvec::SmallVec<A>
where
    A: smallvec::Array,
    A::Item: RecoveredFieldState,
{
    #[requires(true)]
    #[ensures(true)]
    fn recovery_error_slots(&self) -> usize {
        self.iter()
            .map(RecoveredFieldState::recovery_error_slots)
            .sum()
    }

    #[requires(true)]
    #[ensures(true)]
    fn missing_error_slots(&self) -> usize {
        self.iter()
            .map(RecoveredFieldState::missing_error_slots)
            .sum()
    }

    #[requires(true)]
    #[ensures(true)]
    fn invalid_error_slots(&self) -> usize {
        self.iter()
            .map(RecoveredFieldState::invalid_error_slots)
            .sum()
    }
}

#[contract_trait]
impl<T> RecoveredFieldState for Vec1<T>
where
    T: RecoveredFieldState,
{
    #[requires(true)]
    #[ensures(true)]
    fn recovery_error_slots(&self) -> usize {
        self.iter()
            .map(RecoveredFieldState::recovery_error_slots)
            .sum()
    }

    #[requires(true)]
    #[ensures(true)]
    fn missing_error_slots(&self) -> usize {
        self.iter()
            .map(RecoveredFieldState::missing_error_slots)
            .sum()
    }

    #[requires(true)]
    #[ensures(true)]
    fn invalid_error_slots(&self) -> usize {
        self.iter()
            .map(RecoveredFieldState::invalid_error_slots)
            .sum()
    }
}

#[contract_trait]
impl<E, Links> RecoveredFieldState for Chain<E, Links>
where
    E: RecoveredFieldState,
    Links: RecoveredFieldState,
{
    #[requires(true)]
    #[ensures(true)]
    fn recovery_error_slots(&self) -> usize {
        self.first.recovery_error_slots() + self.links.recovery_error_slots()
    }

    #[requires(true)]
    #[ensures(true)]
    fn missing_error_slots(&self) -> usize {
        self.first.missing_error_slots() + self.links.missing_error_slots()
    }

    #[requires(true)]
    #[ensures(true)]
    fn invalid_error_slots(&self) -> usize {
        self.first.invalid_error_slots() + self.links.invalid_error_slots()
    }
}

#[contract_trait]
impl RecoveredFieldState for String {
    #[requires(true)]
    #[ensures(ret == 0)]
    fn recovery_error_slots(&self) -> usize {
        0
    }
}

#[contract_trait]
impl RecoveredFieldState for jbotci_source::SourceSpan {
    #[requires(true)]
    #[ensures(ret == 0)]
    fn recovery_error_slots(&self) -> usize {
        0
    }
}

#[contract_trait]
impl RecoveredFieldState for () {
    #[requires(true)]
    #[ensures(ret == 0)]
    fn recovery_error_slots(&self) -> usize {
        0
    }
}

macro_rules! impl_recovered_field_state_for_tuple {
    ($($name:ident $index:tt),+) => {
        #[contract_trait]
        impl<$($name),+> RecoveredFieldState for ($($name,)+)
        where
            $($name: RecoveredFieldState),+
        {
            #[requires(true)]
            #[ensures(true)]
            fn recovery_error_slots(&self) -> usize {
                0usize $(+ self.$index.recovery_error_slots())+
            }

            #[requires(true)]
            #[ensures(true)]
            fn missing_error_slots(&self) -> usize {
                0usize $(+ self.$index.missing_error_slots())+
            }

            #[requires(true)]
            #[ensures(true)]
            fn unconsumed_missing_error_slots(&self) -> usize {
                0usize $(+ self.$index.unconsumed_missing_error_slots())+
            }
        }
    };
}

impl_recovered_field_state_for_tuple!(A 0);
impl_recovered_field_state_for_tuple!(A 0, B 1);
impl_recovered_field_state_for_tuple!(A 0, B 1, C 2);
impl_recovered_field_state_for_tuple!(A 0, B 1, C 2, D 3);
impl_recovered_field_state_for_tuple!(A 0, B 1, C 2, D 3, E 4);
impl_recovered_field_state_for_tuple!(A 0, B 1, C 2, D 3, E 4, F 5);
impl_recovered_field_state_for_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6);
impl_recovered_field_state_for_tuple!(A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7);

#[contract_trait]
impl RecoveryItemState for () {
    #[requires(true)]
    #[ensures(ret == RecoveryItemKind::Invalid)]
    fn recovery_item_kind(&self) -> RecoveryItemKind {
        RecoveryItemKind::Invalid
    }
}

impl<T, E> Recovered<T, E> {
    #[requires(true)]
    #[ensures(matches!(ret, Self::Valid(_)))]
    pub fn valid(value: T) -> Self {
        Self::Valid(Box::new(value))
    }

    #[requires(true)]
    #[ensures(matches!(ret, Self::Valid(_)))]
    pub fn valid_boxed(value: Box<T>) -> Self {
        Self::Valid(value)
    }

    #[requires(true)]
    #[ensures(matches!(ret, Self::Error(_)))]
    pub fn error(item: E) -> Self {
        Self::Error(item)
    }

    #[requires(!errors.is_empty())]
    #[ensures(matches!(ret, Self::Prefix(_)))]
    pub fn prefix(errors: Vec<E>, value: T) -> Self {
        let errors =
            Vec1::try_from_vec(errors).expect("precondition guarantees non-empty error list");
        Self::Prefix(RecoveredPrefix {
            errors,
            value: Box::new(value),
        })
    }

    #[requires(!errors.is_empty())]
    #[ensures(matches!(ret, Self::Prefix(_)))]
    pub fn prefix_boxed(errors: Vec<E>, value: Box<T>) -> Self {
        let errors =
            Vec1::try_from_vec(errors).expect("precondition guarantees non-empty error list");
        Self::Prefix(RecoveredPrefix { errors, value })
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn try_into_valid_with<V>(
        self,
        path: &mut TreePath,
        convert: impl FnOnce(T, &mut TreePath) -> Result<V, RecoveryError<E>>,
    ) -> Result<V, RecoveryError<E>> {
        match self {
            Self::Valid(value) => convert(*value, path),
            Self::Error(item) => Err(RecoveryError::new(path.clone(), item)),
            Self::Prefix(prefix) => {
                let mut errors = prefix.errors.into_vec();
                Err(RecoveryError::new(path.clone(), errors.remove(0)))
            }
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn try_into_valid_boxed_with<V>(
        self,
        path: &mut TreePath,
        convert: impl FnOnce(Box<T>, &mut TreePath) -> Result<V, RecoveryError<E>>,
    ) -> Result<V, RecoveryError<E>> {
        match self {
            Self::Valid(value) => convert(value, path),
            Self::Error(item) => Err(RecoveryError::new(path.clone(), item)),
            Self::Prefix(prefix) => {
                let mut errors = prefix.errors.into_vec();
                Err(RecoveryError::new(path.clone(), errors.remove(0)))
            }
        }
    }
}

#[invariant(true)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryError<E> {
    pub path: TreePath,
    pub item: E,
}

impl<E> RecoveryError<E> {
    #[requires(true)]
    #[ensures(ret.path == old(path.clone()))]
    pub fn new(path: TreePath, item: E) -> Self {
        Self { path, item }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn path(&self) -> &TreePath {
        &self.path
    }
}

impl<E> fmt::Display for RecoveryError<E> {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "recovered tree error item at {}", self.path)
    }
}

impl<E: fmt::Debug> std::error::Error for RecoveryError<E> {}

#[invariant(true)]
#[invariant(::Field => name.as_ref().is_none_or(|name| !name.is_empty()))]
#[invariant(::SequenceIndex(_) => true)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum TreePathStep {
    Field { name: Option<String>, index: usize },
    SequenceIndex(usize),
}

impl TreePathStep {
    #[requires(name.is_none_or(|name| !name.is_empty()))]
    #[ensures(ret.is_field(name, index))]
    pub fn field(name: Option<&str>, index: usize) -> Self {
        new!(TreePathStep::Field {
            name: name.map(ToOwned::to_owned),
            index,
        })
    }

    #[requires(true)]
    #[ensures(ret.as_sequence_index() == Some(index))]
    pub fn sequence_index(index: usize) -> Self {
        new!(TreePathStep::SequenceIndex(index))
    }

    #[requires(name.is_none_or(|name| !name.is_empty()))]
    #[ensures(true)]
    pub fn is_field(&self, name: Option<&str>, index: usize) -> bool {
        match self.as_data() {
            data!(TreePathStep::Field {
                name: field_name,
                index: field_index,
            }) => field_name.as_deref() == name && *field_index == index,
            data!(TreePathStep::SequenceIndex(_)) => false,
        }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn as_sequence_index(&self) -> Option<usize> {
        match self.as_data() {
            data!(TreePathStep::Field { .. }) => None,
            data!(TreePathStep::SequenceIndex(index)) => Some(*index),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[invariant(true)]
pub struct TreePath {
    steps: Vec<TreePathStep>,
}

impl TreePath {
    #[requires(true)]
    #[ensures(ret.is_empty())]
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn from_steps(steps: Vec<TreePathStep>) -> Self {
        Self { steps }
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn steps(&self) -> &[TreePathStep] {
        &self.steps
    }

    #[requires(true)]
    #[ensures(ret == self.steps.len())]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    #[requires(true)]
    #[ensures(ret == self.steps.is_empty())]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    #[requires(true)]
    #[ensures(!self.steps.is_empty())]
    pub fn push(&mut self, step: TreePathStep) {
        self.steps.push(step);
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn pop(&mut self) -> Option<TreePathStep> {
        self.steps.pop()
    }
}

impl fmt::Display for TreePath {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.steps.is_empty() {
            return formatter.write_str("<root>");
        }

        let mut wrote_field = false;
        for step in &self.steps {
            match step.as_data() {
                data!(TreePathStep::Field {
                    name: Some(name),
                    ..
                }) => {
                    if wrote_field {
                        formatter.write_str(".")?;
                    }
                    formatter.write_str(name)?;
                    wrote_field = true;
                }
                data!(TreePathStep::Field { name: None, index }) => {
                    if wrote_field {
                        formatter.write_str(".")?;
                    }
                    write!(formatter, "<field:{index}>")?;
                    wrote_field = true;
                }
                data!(TreePathStep::SequenceIndex(index)) => write!(formatter, "[{index}]")?,
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub struct FieldRef {
    pub name: Option<&'static str>,
    pub index: usize,
    pub primary: bool,
}

impl FieldRef {
    #[requires(name.is_none_or(|name| !name.is_empty()))]
    #[ensures(ret.name == name)]
    #[ensures(ret.index == index)]
    #[ensures(ret.primary == primary)]
    pub fn new(name: Option<&'static str>, index: usize, primary: bool) -> Self {
        Self {
            name,
            index,
            primary,
        }
    }
}

#[contract_trait]
pub trait TreeVisitor<'tree> {
    type Node: Copy;
    type Atom: Copy;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, _node: Self::Node) {}

    #[requires(true)]
    #[ensures(true)]
    fn exit_node(&mut self, _node: Self::Node) {}

    #[requires(true)]
    #[ensures(true)]
    fn enter_field(&mut self, _field: FieldRef) {}

    #[requires(true)]
    #[ensures(true)]
    fn exit_field(&mut self, _field: FieldRef) {}

    #[requires(true)]
    #[ensures(true)]
    fn visit_absent_optional_field(&mut self, _field: FieldRef) {}

    #[requires(true)]
    #[ensures(true)]
    fn enter_sequence(&mut self) {}

    #[requires(true)]
    #[ensures(true)]
    fn exit_sequence(&mut self) {}

    #[requires(true)]
    #[ensures(true)]
    fn enter_chain(&mut self) {
        self.enter_sequence();
    }

    #[requires(true)]
    #[ensures(true)]
    fn exit_chain(&mut self) {
        self.exit_sequence();
    }

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, _atom: Self::Atom) {}

    #[requires(true)]
    #[ensures(true)]
    fn visit_recovered_error<E: RecoveryItemState + Serialize>(&mut self, _item: &'tree E) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashSet;
    use std::sync::Arc;

    #[allow(unused_imports)]
    use bityzba::{ensures, invariant, requires};
    use serde_json::json;
    use smallvec::SmallVec;
    use vec1::Vec1;

    #[invariant(true)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub(crate) enum RecoveryTreeItem {
        Missing,
        Invalid,
    }

    #[contract_trait]
    impl RecoveryItemState for RecoveryTreeItem {
        #[requires(true)]
        #[ensures(true)]
        fn recovery_item_kind(&self) -> RecoveryItemKind {
            match self {
                RecoveryTreeItem::Missing => RecoveryItemKind::Missing,
                RecoveryTreeItem::Invalid => RecoveryItemKind::Invalid,
            }
        }
    }

    tree_model! {
        #![tree_recovered]

        pub type LeafAlias = LeafNode;
        pub type LeafList = Vec<LeafNode>;

        #[derive(Debug, Clone, PartialEq, Eq)]
        #[invariant(true, "test fixture leaf nodes accept all field values")]
        pub struct LeafNode {
            pub text: String,
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        #[invariant(true)]
        pub struct PairNode {
            pub first: LeafNode,
            #[tree_child(false)]
            pub ignored: String,
            #[tree_child(primary)]
            pub rest: Option<Box<LeafNode>>,
            pub many: Vec<LeafNode>,
            pub aliases: LeafList,
            pub alias: Option<LeafAlias>,
            pub vec1: Vec1<LeafNode>,
            pub small: SmallVec<[LeafNode; 2]>,
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        #[allow(dead_code)]
        #[invariant(true)]
        #[invariant(::Tuple(_) => true)]
        #[invariant(::Named => true)]
        pub enum WrappedNode {
            Tuple(LeafNode),
            Named {
                left: LeafNode,
                #[tree_child(primary)]
                right: LeafNode,
            },
            Unit,
        }
    }

    #[derive(Debug, Default)]
    #[invariant(true)]
    struct RecordingVisitor {
        events: Vec<String>,
    }

    impl<'tree> TreeVisitor<'tree> for RecordingVisitor {
        type Node = NodeRef<'tree>;
        type Atom = AtomRef<'tree>;

        #[requires(true)]
        #[ensures(true)]
        fn enter_node(&mut self, node: Self::Node) {
            self.events
                .push(format!("enter:{}", node.constructor_name()));
        }

        #[requires(true)]
        #[ensures(true)]
        fn exit_node(&mut self, node: Self::Node) {
            self.events
                .push(format!("exit:{}", node.constructor_name()));
        }

        #[requires(true)]
        #[ensures(true)]
        fn enter_field(&mut self, field: FieldRef) {
            self.events.push(format!(
                "field:{}:{}:{}",
                field.name.unwrap_or("<tuple>"),
                field.index,
                field.primary
            ));
        }

        #[requires(true)]
        #[ensures(true)]
        fn visit_atom(&mut self, atom: Self::Atom) {
            match atom {
                AtomRef::String(text) => self.events.push(format!("atom:{text}")),
            }
        }
    }

    #[derive(Debug, Default)]
    #[invariant(true)]
    struct RecoveredRecordingVisitor {
        events: Vec<String>,
    }

    impl<'tree> TreeVisitor<'tree> for RecoveredRecordingVisitor {
        type Node = recovered::NodeRef<'tree>;
        type Atom = recovered::AtomRef<'tree>;

        #[requires(true)]
        #[ensures(true)]
        fn enter_node(&mut self, node: Self::Node) {
            self.events
                .push(format!("enter:{}", node.constructor_name()));
        }

        #[requires(true)]
        #[ensures(true)]
        fn exit_node(&mut self, node: Self::Node) {
            self.events
                .push(format!("exit:{}", node.constructor_name()));
        }

        #[requires(true)]
        #[ensures(true)]
        fn enter_field(&mut self, field: FieldRef) {
            self.events.push(format!(
                "field:{}:{}:{}",
                field.name.unwrap_or("<tuple>"),
                field.index,
                field.primary
            ));
        }

        #[requires(true)]
        #[ensures(true)]
        fn visit_atom(&mut self, atom: Self::Atom) {
            match atom {
                recovered::AtomRef::String(text) => self.events.push(format!("atom:{text}")),
            }
        }
    }

    #[requires(true)]
    #[ensures(ret.rest.is_some())]
    #[ensures(ret.many.len() == 1)]
    #[ensures(ret.aliases.len() == 1)]
    #[ensures(!ret.small.is_empty())]
    fn sample_pair_node() -> PairNode {
        PairNode {
            first: LeafNode {
                text: "first".to_owned(),
            },
            ignored: "ignored".to_owned(),
            rest: Some(Box::new(LeafNode {
                text: "rest".to_owned(),
            })),
            many: vec![LeafNode {
                text: "many".to_owned(),
            }],
            aliases: vec![LeafNode {
                text: "aliases".to_owned(),
            }],
            alias: Some(LeafNode {
                text: "alias".to_owned(),
            }),
            vec1: Vec1::new(LeafNode {
                text: "vec1".to_owned(),
            }),
            small: SmallVec::from_vec(vec![LeafNode {
                text: "small".to_owned(),
            }]),
        }
    }

    #[requires(true)]
    #[ensures(matches!(ret, recovered::Recovered::Valid(_)))]
    fn recovered_leaf(text: &str) -> recovered::Recovered<recovered::LeafNode> {
        recovered::Recovered::valid(recovered::LeafNode {
            text: recovered::Recovered::valid(text.to_owned()),
        })
    }

    #[invariant(true)]
    #[derive(Debug, Default)]
    struct NodeKindVisitor {
        nodes: Vec<(&'static str, bool)>,
    }

    impl<'tree> TreeVisitor<'tree> for NodeKindVisitor {
        type Node = NodeRef<'tree>;
        type Atom = AtomRef<'tree>;

        #[requires(true)]
        #[ensures(true)]
        fn enter_node(&mut self, node: Self::Node) {
            self.nodes
                .push((node.constructor_name(), node.is_variant()));
        }
    }

    #[invariant(events.borrow().iter().all(|event| !event.is_empty()))]
    #[derive(Debug, Default)]
    struct RecordingWalker {
        events: RefCell<Vec<String>>,
    }

    impl<'tree> TreeWalker<'tree> for RecordingWalker {
        #[requires(true)]
        #[ensures(true)]
        fn walk_pair_node(&mut self, node: &'tree PairNode) {
            self.events.borrow_mut().push("pair:before".to_owned());
            walk::pair_node(self, node);
            self.events.borrow_mut().push("pair:after".to_owned());
        }

        #[requires(true)]
        #[ensures(true)]
        fn walk_leaf_node(&mut self, node: &'tree LeafNode) {
            self.events.borrow_mut().push(format!("leaf:{}", node.text));
            walk::leaf_node(self, node);
        }

        #[requires(true)]
        #[ensures(true)]
        fn walk_wrapped_node_named(&mut self, _left: &'tree LeafNode, _right: &'tree LeafNode) {
            self.events.borrow_mut().push("named:cutoff".to_owned());
        }

        #[requires(true)]
        #[ensures(true)]
        fn walk_atom(&mut self, atom: AtomRef<'tree>) {
            match atom {
                AtomRef::String(text) => self.events.borrow_mut().push(format!("atom:{text}")),
            }
        }
    }

    #[invariant(events.borrow().iter().all(|event| !event.is_empty()))]
    #[derive(Debug, Default)]
    struct RecoveredRecordingWalker {
        events: RefCell<Vec<String>>,
    }

    impl<'tree> recovered::TreeWalker<'tree> for RecoveredRecordingWalker {
        #[requires(true)]
        #[ensures(true)]
        fn walk_recovered_error(&mut self, item: &'tree RecoveryTreeItem) {
            self.events.borrow_mut().push(format!("error:{item:?}"));
        }

        #[requires(true)]
        #[ensures(true)]
        fn walk_leaf_node(&mut self, node: &'tree recovered::LeafNode) {
            self.events.borrow_mut().push("leaf".to_owned());
            recovered::walk::leaf_node(self, node);
        }

        #[requires(true)]
        #[ensures(true)]
        fn walk_atom(&mut self, atom: recovered::AtomRef<'tree>) {
            match atom {
                recovered::AtomRef::String(text) => {
                    self.events.borrow_mut().push(format!("atom:{text}"))
                }
            }
        }
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn visits_fields_in_declaration_order_and_skips_false_fields() {
        let tree = PairNode {
            first: LeafNode {
                text: "first".to_owned(),
            },
            ignored: "ignored".to_owned(),
            rest: Some(Box::new(LeafNode {
                text: "rest".to_owned(),
            })),
            many: vec![LeafNode {
                text: "many".to_owned(),
            }],
            aliases: vec![LeafNode {
                text: "aliases".to_owned(),
            }],
            alias: Some(LeafNode {
                text: "alias".to_owned(),
            }),
            vec1: Vec1::new(LeafNode {
                text: "vec1".to_owned(),
            }),
            small: SmallVec::from_vec(vec![LeafNode {
                text: "small".to_owned(),
            }]),
        };
        let mut visitor = RecordingVisitor::default();
        tree.visit_in_order(&mut visitor);

        assert_eq!(
            visitor.events,
            vec![
                "enter:PairNode",
                "field:first:0:false",
                "enter:LeafNode",
                "field:text:0:false",
                "atom:first",
                "exit:LeafNode",
                "field:rest:2:true",
                "enter:LeafNode",
                "field:text:0:false",
                "atom:rest",
                "exit:LeafNode",
                "field:many:3:false",
                "enter:LeafNode",
                "field:text:0:false",
                "atom:many",
                "exit:LeafNode",
                "field:aliases:4:false",
                "enter:LeafNode",
                "field:text:0:false",
                "atom:aliases",
                "exit:LeafNode",
                "field:alias:5:false",
                "enter:LeafNode",
                "field:text:0:false",
                "atom:alias",
                "exit:LeafNode",
                "field:vec1:6:false",
                "enter:LeafNode",
                "field:text:0:false",
                "atom:vec1",
                "exit:LeafNode",
                "field:small:7:false",
                "enter:LeafNode",
                "field:text:0:false",
                "atom:small",
                "exit:LeafNode",
                "exit:PairNode",
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn distinguishes_enum_variants_as_node_refs() {
        let mut visitor = RecordingVisitor::default();
        WrappedNode::Named {
            left: LeafNode {
                text: "left".to_owned(),
            },
            right: LeafNode {
                text: "right".to_owned(),
            },
        }
        .visit_in_order(&mut visitor);

        assert_eq!(
            visitor.events.first().map(String::as_str),
            Some("enter:Named")
        );
        assert!(visitor.events.contains(&"field:right:1:true".to_owned()));

        let mut unit_visitor = RecordingVisitor::default();
        WrappedNode::Unit.visit_in_order(&mut unit_visitor);
        assert_eq!(unit_visitor.events, vec!["enter:Unit", "exit:Unit"]);

        let mut tuple_visitor = RecordingVisitor::default();
        WrappedNode::Tuple(LeafNode {
            text: "tuple".to_owned(),
        })
        .visit_in_order(&mut tuple_visitor);
        assert_eq!(
            tuple_visitor.events.first().map(String::as_str),
            Some("enter:Tuple")
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn marks_struct_nodes_and_enum_variant_nodes() {
        let mut visitor = NodeKindVisitor::default();
        PairNode {
            first: LeafNode {
                text: "first".to_owned(),
            },
            ignored: "ignored".to_owned(),
            rest: None,
            many: Vec::new(),
            aliases: Vec::new(),
            alias: None,
            vec1: Vec1::new(LeafNode {
                text: "vec1".to_owned(),
            }),
            small: SmallVec::new(),
        }
        .visit_in_order(&mut visitor);
        assert_eq!(visitor.nodes.first(), Some(&("PairNode", false)));

        let mut visitor = NodeKindVisitor::default();
        WrappedNode::Unit.visit_in_order(&mut visitor);
        assert_eq!(visitor.nodes, vec![("Unit", true)]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recursive_walker_defaults_descend_in_tree_order() {
        let tree = sample_pair_node();
        let mut walker = RecordingWalker::default();
        tree.walk_with(&mut walker);

        assert_eq!(
            walker.events.borrow().as_slice(),
            [
                "pair:before",
                "leaf:first",
                "atom:first",
                "leaf:rest",
                "atom:rest",
                "leaf:many",
                "atom:many",
                "leaf:aliases",
                "atom:aliases",
                "leaf:alias",
                "atom:alias",
                "leaf:vec1",
                "atom:vec1",
                "leaf:small",
                "atom:small",
                "pair:after",
            ]
            .as_slice()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recursive_walker_overrides_can_cut_off_variant_descent() {
        let tree = WrappedNode::Named {
            left: LeafNode {
                text: "left".to_owned(),
            },
            right: LeafNode {
                text: "right".to_owned(),
            },
        };
        let mut walker = RecordingWalker::default();
        tree.walk_with(&mut walker);

        assert_eq!(walker.events.borrow().as_slice(), ["named:cutoff"]);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recursive_walker_visits_recovered_prefix_errors_before_value() {
        let tree = recovered::Recovered::prefix(
            vec![RecoveryTreeItem::Missing, RecoveryTreeItem::Invalid],
            recovered::LeafNode {
                text: recovered::Recovered::valid("leaf".to_owned()),
            },
        );
        let mut walker = RecoveredRecordingWalker::default();
        recovered::TreeWalkable::walk_with(&tree, &mut walker);

        assert_eq!(
            walker.events.borrow().as_slice(),
            ["error:Missing", "error:Invalid", "leaf", "atom:leaf",].as_slice()
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn node_refs_use_identity_not_structural_equality() {
        let first = LeafNode {
            text: "same".to_owned(),
        };
        let second = LeafNode {
            text: "same".to_owned(),
        };
        let first_ref = NodeRef::LeafNode(&first);
        let repeated_first_ref = NodeRef::LeafNode(&first);
        let second_ref = NodeRef::LeafNode(&second);

        assert_eq!(first_ref, repeated_first_ref);
        assert_ne!(first_ref, second_ref);

        let mut set = HashSet::new();
        set.insert(first_ref);
        set.insert(repeated_first_ref);
        set.insert(second_ref);
        assert_eq!(set.len(), 2);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn node_refs_convert_and_delegate_through_single_node_wrappers() {
        let leaf = LeafNode {
            text: "leaf".to_owned(),
        };
        let leaf_ref: NodeRef<'_> = (&leaf).into();
        assert_eq!(leaf.as_node_ref(), Some(leaf_ref));

        let boxed = Box::new(LeafNode {
            text: "boxed".to_owned(),
        });
        assert_eq!(boxed.as_node_ref(), Some(NodeRef::LeafNode(boxed.as_ref())));

        let arc = Arc::new(LeafNode {
            text: "arc".to_owned(),
        });
        assert_eq!(arc.as_node_ref(), Some(NodeRef::LeafNode(arc.as_ref())));

        let optional = Some(LeafNode {
            text: "optional".to_owned(),
        });
        assert_eq!(
            optional.as_node_ref(),
            Some(NodeRef::LeafNode(optional.as_ref().unwrap()))
        );

        let wrapped = WrappedNode::Named {
            left: LeafNode {
                text: "left".to_owned(),
            },
            right: LeafNode {
                text: "right".to_owned(),
            },
        };
        let wrapped_ref: NodeRef<'_> = (&wrapped).into();
        assert_eq!(wrapped.as_node_ref(), Some(wrapped_ref));
        assert!(wrapped_ref.is_variant());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_from_valid_converts_back_to_valid_tree() {
        let tree = sample_pair_node();
        let recovered = recovered::PairNode::from_valid(tree.clone());

        assert_eq!(recovered.recovery_error_slots(), 0);
        assert_eq!(recovered.missing_error_slots(), 0);
        assert_eq!(recovered.invalid_error_slots(), 0);
        assert_eq!(recovered.try_into_valid(), Ok(tree));
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_conversion_reports_first_error_path() {
        let mut recovered = recovered::PairNode::from_valid(sample_pair_node());
        recovered
            .many
            .push(recovered::Recovered::error(RecoveryTreeItem::Missing));
        recovered
            .aliases
            .push(recovered::Recovered::error(RecoveryTreeItem::Invalid));

        assert_eq!(recovered.recovery_error_slots(), 2);
        assert_eq!(recovered.missing_error_slots(), 1);
        assert_eq!(recovered.invalid_error_slots(), 1);

        let error = recovered
            .try_into_valid()
            .expect_err("recovered errors block conversion to a valid tree");
        assert_eq!(error.item, RecoveryTreeItem::Missing);
        assert_eq!(
            error.path,
            TreePath::from_steps(vec![
                TreePathStep::field(Some("many"), 3),
                TreePathStep::sequence_index(1),
            ])
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn recovered_tree_traversal_visits_valid_fields_in_order() {
        let tree = recovered::PairNode {
            first: recovered_leaf("first"),
            ignored: recovered::Recovered::valid("ignored".to_owned()),
            rest: None,
            many: vec![recovered_leaf("many")],
            aliases: vec![recovered_leaf("aliases")],
            alias: Some(recovered_leaf("alias")),
            vec1: Vec1::new(recovered_leaf("vec1")),
            small: SmallVec::from_vec(vec![recovered_leaf("small")]),
        };
        let mut visitor = RecoveredRecordingVisitor::default();
        recovered::TreeNode::visit_in_order(&tree, &mut visitor);

        assert_eq!(
            visitor.events,
            vec![
                "enter:PairNode",
                "field:first:0:false",
                "enter:LeafNode",
                "field:text:0:false",
                "atom:first",
                "exit:LeafNode",
                "field:rest:2:true",
                "field:many:3:false",
                "enter:LeafNode",
                "field:text:0:false",
                "atom:many",
                "exit:LeafNode",
                "field:aliases:4:false",
                "enter:LeafNode",
                "field:text:0:false",
                "atom:aliases",
                "exit:LeafNode",
                "field:alias:5:false",
                "enter:LeafNode",
                "field:text:0:false",
                "atom:alias",
                "exit:LeafNode",
                "field:vec1:6:false",
                "enter:LeafNode",
                "field:text:0:false",
                "atom:vec1",
                "exit:LeafNode",
                "field:small:7:false",
                "enter:LeafNode",
                "field:text:0:false",
                "atom:small",
                "exit:LeafNode",
                "exit:PairNode",
            ]
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn paths_round_trip_named_fields_wrappers_and_sequences() {
        let tree = sample_pair_node();
        let cases = [
            (
                NodeRef::PairNode(&tree),
                TreePath::new(),
                "<root>".to_owned(),
            ),
            (
                NodeRef::LeafNode(&tree.first),
                TreePath::from_steps(vec![TreePathStep::field(Some("first"), 0)]),
                "first".to_owned(),
            ),
            (
                NodeRef::LeafNode(tree.rest.as_deref().expect("rest exists")),
                TreePath::from_steps(vec![TreePathStep::field(Some("rest"), 2)]),
                "rest".to_owned(),
            ),
            (
                NodeRef::LeafNode(&tree.many[0]),
                TreePath::from_steps(vec![
                    TreePathStep::field(Some("many"), 3),
                    TreePathStep::sequence_index(0),
                ]),
                "many[0]".to_owned(),
            ),
            (
                NodeRef::LeafNode(&tree.aliases[0]),
                TreePath::from_steps(vec![
                    TreePathStep::field(Some("aliases"), 4),
                    TreePathStep::sequence_index(0),
                ]),
                "aliases[0]".to_owned(),
            ),
            (
                NodeRef::LeafNode(tree.alias.as_ref().expect("alias exists")),
                TreePath::from_steps(vec![TreePathStep::field(Some("alias"), 5)]),
                "alias".to_owned(),
            ),
            (
                NodeRef::LeafNode(&tree.vec1[0]),
                TreePath::from_steps(vec![
                    TreePathStep::field(Some("vec1"), 6),
                    TreePathStep::sequence_index(0),
                ]),
                "vec1[0]".to_owned(),
            ),
            (
                NodeRef::LeafNode(&tree.small[0]),
                TreePath::from_steps(vec![
                    TreePathStep::field(Some("small"), 7),
                    TreePathStep::sequence_index(0),
                ]),
                "small[0]".to_owned(),
            ),
        ];

        for (target, expected_path, expected_display) in cases {
            let path = tree.path_to_node(target).expect("target is in tree");
            assert_eq!(path, expected_path);
            assert_eq!(path.to_string(), expected_display);
            assert_eq!(tree.node_at_path(&path), Some(target));
        }

        let skipped_path = TreePath::from_steps(vec![TreePathStep::field(Some("ignored"), 1)]);
        assert_eq!(tree.node_at_path(&skipped_path), None);
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn paths_round_trip_enum_named_tuple_and_unit_variants() {
        let named = WrappedNode::Named {
            left: LeafNode {
                text: "left".to_owned(),
            },
            right: LeafNode {
                text: "right".to_owned(),
            },
        };
        let WrappedNode::Named { left, right } = &named else {
            unreachable!("constructed as named variant");
        };
        for (target, expected_path) in [
            (NodeRef::WrappedNodeNamed(&named), TreePath::new()),
            (
                NodeRef::LeafNode(left),
                TreePath::from_steps(vec![TreePathStep::field(Some("left"), 0)]),
            ),
            (
                NodeRef::LeafNode(right),
                TreePath::from_steps(vec![TreePathStep::field(Some("right"), 1)]),
            ),
        ] {
            let path = named.path_to_node(target).expect("target is in tree");
            assert_eq!(path, expected_path);
            assert_eq!(named.node_at_path(&path), Some(target));
        }

        let tuple = WrappedNode::Tuple(LeafNode {
            text: "tuple".to_owned(),
        });
        let WrappedNode::Tuple(tuple_leaf) = &tuple else {
            unreachable!("constructed as tuple variant");
        };
        let tuple_path = TreePath::from_steps(vec![TreePathStep::field(None, 0)]);
        assert_eq!(
            tuple.path_to_node(NodeRef::LeafNode(tuple_leaf)),
            Some(tuple_path.clone())
        );
        assert_eq!(
            tuple.node_at_path(&tuple_path),
            Some(NodeRef::LeafNode(tuple_leaf))
        );
        assert_eq!(tuple_path.to_string(), "<field:0>");

        let unit = WrappedNode::Unit;
        assert_eq!(
            unit.node_at_path(&TreePath::new()),
            Some(NodeRef::WrappedNodeUnit(&unit))
        );
        assert_eq!(
            unit.node_at_path(&TreePath::from_steps(vec![TreePathStep::field(None, 0)])),
            None
        );
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn tree_path_serializes_as_structured_steps() {
        let path = TreePath::from_steps(vec![
            TreePathStep::field(Some("many"), 3),
            TreePathStep::sequence_index(0),
        ]);

        let value = serde_json::to_value(&path).expect("path serializes");
        assert_eq!(
            value,
            json!({
                "steps": [
                    {
                        "kind": "field",
                        "value": {
                            "name": "many",
                            "index": 3
                        }
                    },
                    {
                        "kind": "sequence-index",
                        "value": 0
                    }
                ]
            })
        );

        let round_trip: TreePath = serde_json::from_value(value).expect("path deserializes");
        assert_eq!(round_trip, path);
    }
}
