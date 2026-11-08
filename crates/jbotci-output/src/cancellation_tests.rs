//! Renderers return [`OutputError::Interrupted`] when the flag is set.
//!
//! Bracket, tree, and morphology-bracket rendering of a real sentence finish
//! too quickly for a delayed flag to be a reliable stop. Each format is
//! rendered twice. A clear flag returns a document. A flag set before the
//! call returns the interrupted error and no document. Removing the load
//! makes the second call return the document.

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_interrupt::CancellationSource;
use jbotci_morphology::segment_words_with_modifiers;
use jbotci_syntax::parse_syntax_tree;

use crate::budget::ByteLimit;
use crate::{
    BracketRenderOptions, OutputError, TreeRenderOptions,
    pretty_generated_model_brackets_with_limit_and_cancellation,
    pretty_generated_model_tree_with_limit_and_cancellation,
    pretty_morphology_brackets_with_limit_and_cancellation,
};

#[test]
#[requires(true)]
#[ensures(true)]
fn cancellation_stops_bracket_tree_and_morphology_rendering() {
    let text = "lo prenu cu broda i ".repeat(12);
    let words = segment_words_with_modifiers(&text).expect("valid morphology");
    let parse = parse_syntax_tree(&words).expect("valid syntax");
    let limit = ByteLimit::new(2_000_000).expect("positive limit");
    let source = CancellationSource::new();
    source.cancel();
    let token = source.token();
    let clear = jbotci_interrupt::CancellationToken::never();

    let brackets = pretty_generated_model_brackets_with_limit_and_cancellation(
        parse.parse_tree.as_ref(),
        &text,
        BracketRenderOptions::default(),
        limit,
        &clear,
    )
    .expect("clear flag renders brackets");
    assert!(!brackets.is_empty());
    let stopped = pretty_generated_model_brackets_with_limit_and_cancellation(
        parse.parse_tree.as_ref(),
        &text,
        BracketRenderOptions::default(),
        limit,
        &token,
    );
    assert!(matches!(stopped, Err(OutputError::Interrupted)));

    let tree = pretty_generated_model_tree_with_limit_and_cancellation(
        parse.parse_tree.as_ref(),
        &text,
        TreeRenderOptions::default(),
        limit,
        &clear,
    )
    .expect("clear flag renders a tree");
    assert!(!tree.is_empty());
    let stopped = pretty_generated_model_tree_with_limit_and_cancellation(
        parse.parse_tree.as_ref(),
        &text,
        TreeRenderOptions::default(),
        limit,
        &token,
    );
    assert!(matches!(stopped, Err(OutputError::Interrupted)));

    let morphology = pretty_morphology_brackets_with_limit_and_cancellation(
        &words,
        &text,
        BracketRenderOptions::default(),
        limit,
        &clear,
    )
    .expect("clear flag renders morphology brackets");
    assert!(!morphology.is_empty());
    let stopped = pretty_morphology_brackets_with_limit_and_cancellation(
        &words,
        &text,
        BracketRenderOptions::default(),
        limit,
        &token,
    );
    assert!(matches!(stopped, Err(OutputError::Interrupted)));
}
