#![recursion_limit = "1024"]

use std::sync::Arc;

use bityzba::{ensures, invariant, requires};
use jbotci_dialect::parse_dialect_definition;
use jbotci_morphology::segment_words_with_modifiers;
use jbotci_syntax::{
    ExperimentalConstruct, ParseOptions, generated_model as model,
    parse_syntax_tree_with_source_and_options,
};
use jbotci_tree::TreeVisitor;

// These are independent borrowed observations of validated nodes; no combination
// of entries is intrinsically invalid, including an empty traversal.
#[invariant(true)]
#[derive(Default)]
struct PlacementVisitor<'tree> {
    preposed: Vec<&'tree model::PreposedLinkargsTanruUnitSyntax>,
    units: Vec<&'tree model::TanruUnitSyntax>,
    jai: Vec<&'tree model::JaiModalTanruUnitSyntax>,
}

impl<'tree> TreeVisitor<'tree> for PlacementVisitor<'tree> {
    type Node = model::NodeRef<'tree>;
    type Atom = model::AtomRef<'tree>;

    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        match node {
            model::NodeRef::PreposedLinkargsTanruUnitSyntax(unit) => self.preposed.push(unit),
            model::NodeRef::TanruUnitSyntax(unit) => self.units.push(unit),
            model::NodeRef::JaiModalTanruUnitSyntax(unit) => self.jai.push(unit),
            _ => {}
        }
    }
}

#[invariant(true)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Placement {
    Postposed,
    Preposed,
    OuterCei,
    InnerPostlink,
    UnderJai,
}

impl Placement {
    #[requires(true)]
    #[ensures(!ret.is_empty())]
    fn source(self) -> &'static str {
        match self {
            Self::Postposed => "mi broda be ko'a be'o",
            Self::Preposed => "mi be ko'a be'o broda",
            Self::OuterCei => "mi be ko'a be'o broda cei brode",
            Self::InnerPostlink => "mi be ko'a be'o broda be ko'e be'o",
            Self::UnderJai => "mi jai be ko'a be'o broda",
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn preposed_links_warn_once_and_keep_cei_outside_the_linked_atom() {
    for dialect in ["()", "(zantufa)", "(+zantufa-terms)"] {
        let definition = parse_dialect_definition(dialect).expect("valid test dialect");
        let options = ParseOptions::default().with_dialect_definition(&definition);
        for case in [
            Placement::Postposed,
            Placement::Preposed,
            Placement::OuterCei,
            Placement::InnerPostlink,
            Placement::UnderJai,
        ] {
            let source = case.source();
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let parsed = parse_syntax_tree_with_source_and_options(&words, source, &options)
                .unwrap_or_else(|error| panic!("{dialect} {case:?}: {error:?}"));
            let mut visitor = PlacementVisitor::default();
            model::TreeNode::visit_in_order(parsed.parse_tree.as_ref(), &mut visitor);

            let preposed_count = usize::from(case != Placement::Postposed);
            assert_eq!(visitor.preposed.len(), preposed_count, "{dialect} {case:?}");
            assert_eq!(parsed.warnings.len(), preposed_count, "{dialect} {case:?}");
            for warning in &parsed.warnings {
                assert_eq!(
                    warning.kind,
                    ExperimentalConstruct::ExperimentalPreposedLinkargs
                );
                let range = warning.anchor.core_word().byte_range().expect("sourced BE");
                assert_eq!(&source[range], "be");
            }
            for preposed in &visitor.preposed {
                // Compile-time API pins: preposed links contain a linked atom,
                // while a JAI inner uses the very same ordinary atom family.
                let _: &Arc<model::LinkedTanruUnitSyntax> = &preposed.base;
                assert_eq!(
                    preposed.base.linkargs.is_some(),
                    case == Placement::InnerPostlink
                );
            }
            assert_eq!(visitor.jai.len(), usize::from(case == Placement::UnderJai));
            for jai in &visitor.jai {
                let _: &Arc<model::TanruUnitAtomSyntax> = &jai.inner_unit;
                assert!(matches!(
                    jai.inner_unit.base.as_ref(),
                    model::TanruUnitAtomBaseSyntax::PreposedLinkargsTanruUnit(_)
                ));
            }
            assert_eq!(
                visitor
                    .units
                    .iter()
                    .map(|unit| unit.assignments.len())
                    .sum::<usize>(),
                usize::from(case == Placement::OuterCei)
            );
            if case == Placement::OuterCei {
                let outer = visitor
                    .units
                    .iter()
                    .find(|unit| {
                        matches!(unit.base.base.base.as_ref(),
                    model::TanruUnitAtomBaseSyntax::PreposedLinkargsTanruUnit(preposed)
                        if std::ptr::eq(preposed, visitor.preposed[0]))
                    })
                    .expect("the outer tanru unit owns the preposed atom");
                assert_eq!(outer.assignments.len(), 1);
            }
        }
    }
}
