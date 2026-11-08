//! Reference analysis stops when the flag is already set.
//!
//! The walk of one ordinary text is shorter than a delay, so a flag set on
//! another thread can miss it. The same tree is analyzed twice: a clear flag
//! returns the analysis, and a flag set before the call returns
//! [`ReferenceAnalysisError::Interrupted`] and no partial index. Removing the
//! loads makes the second call return that analysis.

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_interrupt::CancellationSource;
use jbotci_syntax::parse_syntax_tree;

use crate::references::{
    ReferenceAnalysisError, analyze_generated_references,
    analyze_generated_references_with_cancellation,
};

#[test]
#[requires(true)]
#[ensures(true)]
fn cancellation_drops_reference_analysis_instead_of_returning_it() {
    let text = "lo prenu cu broda i ".repeat(40);
    let words = jbotci_morphology::segment_words_with_modifiers(&text)
        .expect("the repeated sentence is valid morphology");
    let parse = parse_syntax_tree(&words).expect("the repeated sentence is valid syntax");
    let analysis =
        analyze_generated_references(parse.parse_tree.as_ref()).expect("clear flag analyzes");
    assert!(analysis.syntax_index.node_count() > 0);

    let source = CancellationSource::new();
    source.cancel();
    let error =
        analyze_generated_references_with_cancellation(parse.parse_tree.as_ref(), &source.token())
            .expect_err("a set flag must not return an analysis");
    assert!(matches!(error, ReferenceAnalysisError::Interrupted));
}
