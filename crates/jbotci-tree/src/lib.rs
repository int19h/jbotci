//! Generic ordered tree traversal support.

extern crate self as jbotci_tree;

use bityzba::{contract_trait, invariant, requires};

pub use jbotci_tree_macros::tree_model;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[invariant(true)]
pub struct FieldRef {
    pub name: Option<&'static str>,
    pub primary: bool,
}

impl FieldRef {
    #[requires(true)]
    #[ensures(ret.name == name)]
    #[ensures(ret.primary == primary)]
    pub fn new(name: Option<&'static str>, primary: bool) -> Self {
        Self { name, primary }
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
    #[allow(unused_imports)]
    use bityzba::{ensures, invariant, requires};
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
                "field:{}:{}",
                field.name.unwrap_or("<tuple>"),
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
                "field:first:false",
                "enter:LeafNode",
                "field:text:false",
                "atom:first",
                "exit:LeafNode",
                "field:rest:true",
                "enter:LeafNode",
                "field:text:false",
                "atom:rest",
                "exit:LeafNode",
                "field:many:false",
                "enter:LeafNode",
                "field:text:false",
                "atom:many",
                "exit:LeafNode",
                "field:aliases:false",
                "enter:LeafNode",
                "field:text:false",
                "atom:aliases",
                "exit:LeafNode",
                "field:alias:false",
                "enter:LeafNode",
                "field:text:false",
                "atom:alias",
                "exit:LeafNode",
                "field:vec1:false",
                "enter:LeafNode",
                "field:text:false",
                "atom:vec1",
                "exit:LeafNode",
                "field:small:false",
                "enter:LeafNode",
                "field:text:false",
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
        assert!(visitor.events.contains(&"field:right:true".to_owned()));

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
}
