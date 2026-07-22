#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};
use jbotci_dialect::parse_dialect_definition;
use jbotci_morphology::{
    Cmavo, MorphologyOptions, segment_words_with_modifiers,
    segment_words_with_modifiers_with_options_and_source_id,
};
use jbotci_syntax::{
    ParseOptions, SyntaxRecoveryItem, Token, generated_model,
    parse_syntax_tree_recovered_with_source_and_options, parse_syntax_tree_with_source_and_options,
    syntax_tree_eq_ignoring_spans,
};

const JBOPONEI: &str = "(jboponei)";

#[test]
#[requires(true)]
#[ensures(true)]
fn jboponei_po_expansion_matches_explicit_strict_parse() {
    let dialect = parse_dialect_definition(JBOPONEI).expect("built-in jboponei dialect");
    let dialect_source = "mi cusku po do klama";
    let dialect_words = segment_words_with_modifiers_with_options_and_source_id(
        dialect_source,
        &MorphologyOptions::default().with_dialect_definition(&dialect),
        None,
    )
    .expect("valid jboponei morphology");
    let dialect_parse = parse_syntax_tree_with_source_and_options(
        &dialect_words,
        dialect_source,
        &ParseOptions::default().with_dialect_definition(&dialect),
    )
    .expect("the jboponei expansion is valid strict syntax");

    let explicit_source = "mi cusku lo su'u do klama";
    let explicit_words =
        segment_words_with_modifiers(explicit_source).expect("valid explicit morphology");
    let explicit_parse = parse_syntax_tree_with_source_and_options(
        &explicit_words,
        explicit_source,
        &ParseOptions::default(),
    )
    .expect("the explicit expansion is valid strict syntax");

    assert!(syntax_tree_eq_ignoring_spans(
        dialect_parse.parse_tree.as_ref(),
        explicit_parse.parse_tree.as_ref(),
    ));
}

#[invariant(skipped_runs.borrow().iter().all(|run| !run.is_empty()))]
#[derive(Default)]
struct SkippedTokenCollector {
    skipped_runs: RefCell<Vec<Vec<Token>>>,
}

impl<'tree> generated_model::recovered::TreeWalker<'tree> for SkippedTokenCollector {
    #[requires(true)]
    #[ensures(self.skipped_runs.borrow().len() == old(self.skipped_runs.borrow().len()) + usize::from(item.skipped_tokens().is_some()))]
    fn walk_recovered_error(&mut self, item: &'tree SyntaxRecoveryItem) {
        if let Some(tokens) = item.skipped_tokens() {
            self.skipped_runs.borrow_mut().push(tokens.to_vec());
        }
    }
}

#[test]
#[requires(true)]
#[ensures(true)]
fn recovery_retains_both_co_spanned_po_expansion_tokens() {
    let dialect = parse_dialect_definition(JBOPONEI).expect("built-in jboponei dialect");
    let source = "mi cusku po li";
    let words = segment_words_with_modifiers_with_options_and_source_id(
        source,
        &MorphologyOptions::default().with_dialect_definition(&dialect),
        None,
    )
    .expect("valid jboponei morphology");
    let recovered = parse_syntax_tree_recovered_with_source_and_options(
        &words,
        source,
        &ParseOptions::default().with_dialect_definition(&dialect),
    );

    assert_eq!(recovered.errors.len(), 1);
    let mut collector = SkippedTokenCollector::default();
    generated_model::recovered::TreeWalkable::walk_with(
        recovered.parse_tree.as_ref(),
        &mut collector,
    );
    let skipped_runs = collector.skipped_runs.borrow();
    let siblings = skipped_runs
        .iter()
        .flat_map(|run| run.windows(2))
        .find(|pair| pair[0].cmavo() == Some(Cmavo::Lo) && pair[1].cmavo() == Some(Cmavo::Suhu))
        .expect("recovery must retain adjacent lo and su'u expansion tokens");
    let lo_spans = siblings[0].source_spans();
    let suhu_spans = siblings[1].source_spans();
    assert_eq!(lo_spans, suhu_spans);
    assert_eq!((lo_spans[0].byte_start, lo_spans[0].byte_end), (9, 11),);
}
use std::cell::RefCell;
