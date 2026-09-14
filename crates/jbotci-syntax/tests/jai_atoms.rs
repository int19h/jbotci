#![recursion_limit = "1024"]

use std::sync::Arc;

use bityzba::{ensures, invariant, requires};
use jbotci_dialect::parse_dialect_definition;
use jbotci_morphology::segment_words_with_modifiers;
use jbotci_syntax::{
    ExperimentalConstruct, ParseOptions, generated_model as model,
    generated_model::recovered as recovered_model,
    parse_syntax_tree_with_source_and_options,
    parse_syntax_tree_recovered_with_source_and_options,
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
    mehoi: Vec<&'tree model::MehoiTanruUnitSyntax>,
    quotes: Vec<&'tree model::QuotedSumtiSyntax>,
}

#[invariant(true)]
#[derive(Default)]
struct RecoveredJaiVisitor<'tree> {
    jai: Vec<&'tree recovered_model::JaiModalTanruUnitSyntax>,
}

impl<'tree> TreeVisitor<'tree> for RecoveredJaiVisitor<'tree> {
    type Node = recovered_model::NodeRef<'tree>;
    type Atom = recovered_model::AtomRef<'tree>;
    #[requires(true)]
    #[ensures(true)]
    fn enter_node(&mut self, node: Self::Node) {
        if let recovered_model::NodeRef::JaiModalTanruUnitSyntax(jai) = node { self.jai.push(jai); }
    }
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
            model::NodeRef::MehoiTanruUnitSyntax(unit) => self.mehoi.push(unit),
            model::NodeRef::QuotedSumtiSyntax(quote) => self.quotes.push(quote),
            _ => {}
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn mehoi_is_a_one_word_atom_in_both_runtime_dialects() {
    for dialect in ["()", "(zantufa)"] {
        let definition = parse_dialect_definition(dialect).expect("valid test dialect");
        let options = ParseOptions::default().with_dialect_definition(&definition);
        for source in [
            "mi me'oi broda",
            "mi me'oi broda cei brode",
            "mi broda cei me'oi brode",
            "mi me'oi broda bo brode",
            "mi me'oi broda be ko'a be'o",
            "mi se me'oi broda",
            "mi jai me'oi broda",
            "mi na'e me'oi broda",
            "mi me'oi broda brode",
        ] {
            let words = segment_words_with_modifiers(source).expect("valid morphology");
            let parsed = parse_syntax_tree_with_source_and_options(&words, source, &options)
                .unwrap_or_else(|error| panic!("{dialect} {source}: {error:?}"));
            let mut visitor = PlacementVisitor::default();
            model::TreeNode::visit_in_order(parsed.parse_tree.as_ref(), &mut visitor);
            assert_eq!(visitor.mehoi.len(), 1, "{dialect} {source}");
            assert!(
                visitor.quotes.is_empty(),
                "MEhOI must never own a quoted sumti"
            );
            assert_eq!(parsed.warnings.len(), 1, "{dialect} {source}");
            assert_eq!(
                parsed.warnings[0].kind,
                ExperimentalConstruct::ExperimentalMehOiSelbriUnit
            );
            // The completed morphology token is the warning anchor. Even with
            // an adjacent tanru word, the quote contains only its first word.
            let range = parsed.warnings[0]
                .anchor
                .core_word()
                .byte_range()
                .expect("sourced quote");
            assert_eq!(
                &source[range],
                if source == "mi broda cei me'oi brode" {
                    "me'oi brode"
                } else {
                    "me'oi broda"
                }
            );
            if source == "mi jai me'oi broda" {
                assert!(matches!(
                    visitor.jai[0].inner_unit.base.as_ref(),
                    model::TanruUnitAtomBaseSyntax::MehoiTanruUnit(_)
                ));
            }
            if source == "mi se me'oi broda" {
                let atom = &visitor.units[0].base.base;
                assert_eq!(atom.conversions.len(), 1);
                assert!(matches!(
                    atom.base.as_ref(),
                    model::TanruUnitAtomBaseSyntax::MehoiTanruUnit(_)
                ));
            }
            if source == "mi me'oi broda be ko'a be'o" {
                assert!(visitor.units[0].base.linkargs.is_some());
                assert!(matches!(
                    visitor.units[0].base.base.base.as_ref(),
                    model::TanruUnitAtomBaseSyntax::MehoiTanruUnit(_)
                ));
            }
        }
        let source = "mi me me'oi broda me'u";
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        assert!(parse_syntax_tree_with_source_and_options(&words, source, &options).is_err());
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn jai_enclosed_public_route_regression() {
    let source = "mi jai ga broda gi brode";
    let definition = parse_dialect_definition("(zantufa)").expect("valid dialect");
    let options = ParseOptions::default().with_dialect_definition(&definition);
    let words = segment_words_with_modifiers(source).expect("valid morphology");
    let parsed = parse_syntax_tree_with_source_and_options(&words, source, &options)
        .expect("public route parses");
    let mut visitor = PlacementVisitor::default();
    model::TreeNode::visit_in_order(parsed.parse_tree.as_ref(), &mut visitor);
    assert_eq!(visitor.jai.len(), 1, "expected no-tag enclosed JAI owner");
    assert!(matches!(
        visitor.jai[0].inner_unit.base.as_ref(),
        model::TanruUnitAtomBaseSyntax::ZantufaForethoughtTanruUnit(_)
    ));
    let nested_source = "mi jai jai ga broda gi brode";
    let nested_words = segment_words_with_modifiers(nested_source).expect("valid nested morphology");
    let nested = parse_syntax_tree_with_source_and_options(&nested_words, nested_source, &options)
        .expect("nested no-tag enclosed JAI parses");
    let mut nested_visitor = PlacementVisitor::default();
    model::TreeNode::visit_in_order(nested.parse_tree.as_ref(), &mut nested_visitor);
    assert_eq!(nested_visitor.jai.len(), 2, "nested JAI owners are retained");
    assert!(matches!(nested_visitor.jai[0].inner_unit.base.as_ref(), model::TanruUnitAtomBaseSyntax::JaiModalTanruUnit(_)));
    let nested_inner = match nested_visitor.jai[0].inner_unit.base.as_ref() {
        model::TanruUnitAtomBaseSyntax::JaiModalTanruUnit(inner) => &inner.inner_unit,
        _ => unreachable!(),
    };
    assert!(matches!(nested_inner.base.as_ref(), model::TanruUnitAtomBaseSyntax::ZantufaForethoughtTanruUnit(_)));
    for source in ["mi jai pu ga broda gi brode", "mi jai ko'a"] {
        let words = segment_words_with_modifiers(source).expect("valid morphology");
        let parsed = parse_syntax_tree_with_source_and_options(&words, source, &options)
            .expect("control parses");
        let mut visitor = PlacementVisitor::default();
        model::TreeNode::visit_in_order(parsed.parse_tree.as_ref(), &mut visitor);
        assert!(visitor.jai.is_empty(), "tag/sumti control must remain tag-term: {source}");
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn jai_enclosed_recovered_public_route_regression() {
    let source = "mi jai ga broda gi brode";
    let definition = parse_dialect_definition("(zantufa)").expect("valid dialect");
    let options = ParseOptions::default().with_dialect_definition(&definition);
    let words = segment_words_with_modifiers(source).expect("valid morphology");
    let recovered = parse_syntax_tree_recovered_with_source_and_options(&words, source, &options);
    let mut visitor = RecoveredJaiVisitor::default();
    recovered_model::TreeNode::visit_in_order(recovered.parse_tree.as_ref(), &mut visitor);
    assert_eq!(visitor.jai.len(), 1, "recovered public twin retains one JAI owner");
    let inner = match visitor.jai[0].inner_unit.as_ref() {
        jbotci_tree::Recovered::Valid(inner) => inner,
        jbotci_tree::Recovered::Prefix(prefix) => &prefix.value,
        jbotci_tree::Recovered::Error(_) => panic!("recovered JAI inner missing"),
    };
    let base = match inner.base.as_ref() {
        jbotci_tree::Recovered::Valid(base) => base,
        jbotci_tree::Recovered::Prefix(prefix) => &prefix.value,
        jbotci_tree::Recovered::Error(_) => panic!("recovered JAI base missing"),
    };
    assert!(matches!(base.as_ref(), recovered_model::TanruUnitAtomBaseSyntax::ZantufaForethoughtTanruUnit(_)));
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
