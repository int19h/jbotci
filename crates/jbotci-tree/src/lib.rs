//! Generic ordered tree traversal support.

extern crate self as jbotci_tree;

use std::fmt;

#[allow(unused_imports)]
use bityzba::{contract_trait, data, ensures, invariant, new, requires};
use serde::{Deserialize, Serialize};

pub use jbotci_tree_macros::tree_model;

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
    fn enter_sequence(&mut self) {}

    #[requires(true)]
    #[ensures(true)]
    fn exit_sequence(&mut self) {}

    #[requires(true)]
    #[ensures(true)]
    fn visit_atom(&mut self, _atom: Self::Atom) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[allow(unused_imports)]
    use bityzba::{ensures, invariant, requires};
    use serde_json::json;
    use smallvec::SmallVec;
    use vec1::Vec1;

    tree_model! {
        pub type LeafAlias = LeafNode;
        pub type LeafList = Vec<LeafNode>;

        #[derive(Debug, Clone, PartialEq, Eq)]
        #[invariant(true)]
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

    #[derive(Debug, Default)]
    #[invariant(true)]
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
