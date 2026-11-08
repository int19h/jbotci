//! Reference analysis stops when the flag is already set.
//!
//! The walk of one ordinary text is shorter than a delay, so a flag set on
//! another thread can miss it. The same tree is analyzed twice: a clear flag
//! returns the analysis, and a flag set before the call returns
//! [`ReferenceAnalysisError::Interrupted`] and no partial index. Removing the
//! loads makes the second call return that analysis.

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_interrupt::{CancellationSource, assert_exit_check};
use jbotci_syntax::parse_syntax_tree;

use crate::references::{ReferenceAnalysisError, analyze_generated_references};

#[test]
#[requires(true)]
#[ensures(true)]
fn cancellation_drops_reference_analysis_instead_of_returning_it() {
    let text = "lo prenu cu broda i ".repeat(40);
    let words = jbotci_morphology::segment_words_with_modifiers(&text)
        .expect("the repeated sentence is valid morphology");
    let parse = parse_syntax_tree(&words).expect("the repeated sentence is valid syntax");
    let analysis = analyze_generated_references(
        parse.parse_tree.as_ref(),
        &jbotci_interrupt::CancellationToken::never(),
    )
    .expect("clear flag analyzes");
    assert!(analysis.syntax_index.node_count() > 0);

    let source = CancellationSource::new();
    source.cancel();
    let error = analyze_generated_references(parse.parse_tree.as_ref(), &source.token())
        .expect_err("a set flag must not return an analysis");
    assert!(matches!(error, ReferenceAnalysisError::Interrupted));
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_late_trip_stops_inside_reference_analysis() {
    let text = "mi klama le zarci";
    let words = jbotci_morphology::segment_words_with_modifiers(text).expect("morphology");
    let parse = parse_syntax_tree(&words).expect("syntax");
    let tree = parse.parse_tree.as_ref();
    let analysis = assert_exit_check(
        |token| analyze_generated_references(tree, token),
        |error| matches!(error, ReferenceAnalysisError::Interrupted),
    );
    assert!(analysis.syntax_index.node_count() > 0);
}
