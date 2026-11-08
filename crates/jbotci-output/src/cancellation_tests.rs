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
    pretty_generated_model_brackets_with_limit, pretty_generated_model_tree_with_limit,
    pretty_morphology_brackets_with_limit,
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

    let brackets = pretty_generated_model_brackets_with_limit(
        parse.parse_tree.as_ref(),
        &text,
        BracketRenderOptions::default(),
        limit,
        &clear,
    )
    .expect("clear flag renders brackets");
    assert!(!brackets.is_empty());
    let stopped = pretty_generated_model_brackets_with_limit(
        parse.parse_tree.as_ref(),
        &text,
        BracketRenderOptions::default(),
        limit,
        &token,
    );
    assert!(matches!(stopped, Err(OutputError::Interrupted)));

    let tree = pretty_generated_model_tree_with_limit(
        parse.parse_tree.as_ref(),
        &text,
        TreeRenderOptions::default(),
        limit,
        &clear,
    )
    .expect("clear flag renders a tree");
    assert!(!tree.is_empty());
    let stopped = pretty_generated_model_tree_with_limit(
        parse.parse_tree.as_ref(),
        &text,
        TreeRenderOptions::default(),
        limit,
        &token,
    );
    assert!(matches!(stopped, Err(OutputError::Interrupted)));

    let morphology = pretty_morphology_brackets_with_limit(
        &words,
        &text,
        BracketRenderOptions::default(),
        limit,
        &clear,
    )
    .expect("clear flag renders morphology brackets");
    assert!(!morphology.is_empty());
    let stopped = pretty_morphology_brackets_with_limit(
        &words,
        &text,
        BracketRenderOptions::default(),
        limit,
        &token,
    );
    assert!(matches!(stopped, Err(OutputError::Interrupted)));
}

/// Release measurements with `cancellation_test_hook` on the inputs below.
/// Each value is the last `trip_after_checks` argument that returns
/// [`OutputError::Interrupted`]. The next argument returns the document.
/// Counting again inside the test would hide a deleted load.
const MORPHOLOGY_BRACKETS_LATE_TRIP: usize = 10;
const GENERATED_BRACKETS_LATE_TRIP: usize = 222;
const GENERATED_TREE_LATE_TRIP: usize = 22;
/// The recovered sentence has errors, so this count includes the post-walk
/// load. A valid tree takes the generated-bracket path and never reaches it.
const RECOVERED_BRACKETS_LATE_TRIP: usize = 10;

#[test]
#[requires(true)]
#[ensures(true)]
fn a_late_trip_stops_inside_each_renderer() {
    let text = "mi klama le zarci";
    let words = segment_words_with_modifiers(text).expect("morphology");
    let parse = parse_syntax_tree(&words).expect("syntax");
    let bad = "mi klama le ???";
    let bad_words = segment_words_with_modifiers(bad).expect("bad morphology");
    let recovered = jbotci_syntax::parse_syntax_tree_recovered_with_source_and_options(
        &bad_words,
        bad,
        &jbotci_syntax::ParseOptions::default(),
    )
    .expect("an uninterrupted recovered parse returns a tree");
    assert!(
        !recovered.errors.is_empty(),
        "the recovered-bracket trip must use a tree the post-walk load sees"
    );
    let limit = ByteLimit::new(2_000_000).expect("positive limit");
    let stops = |trip: usize, render: &dyn Fn(&jbotci_interrupt::CancellationToken) -> bool| {
        let token = jbotci_interrupt::CancellationToken::trip_after_checks(trip);
        render(&token)
    };

    assert!(stops(MORPHOLOGY_BRACKETS_LATE_TRIP, &|token| {
        matches!(
            pretty_morphology_brackets_with_limit(
                &words,
                text,
                BracketRenderOptions::default(),
                limit,
                token,
            ),
            Err(OutputError::Interrupted)
        )
    }));
    assert!(!stops(MORPHOLOGY_BRACKETS_LATE_TRIP + 1, &|token| {
        matches!(
            pretty_morphology_brackets_with_limit(
                &words,
                text,
                BracketRenderOptions::default(),
                limit,
                token,
            ),
            Err(OutputError::Interrupted)
        )
    }));

    assert!(stops(GENERATED_BRACKETS_LATE_TRIP, &|token| {
        matches!(
            pretty_generated_model_brackets_with_limit(
                parse.parse_tree.as_ref(),
                text,
                BracketRenderOptions::default(),
                limit,
                token,
            ),
            Err(OutputError::Interrupted)
        )
    }));
    assert!(!stops(GENERATED_BRACKETS_LATE_TRIP + 1, &|token| {
        matches!(
            pretty_generated_model_brackets_with_limit(
                parse.parse_tree.as_ref(),
                text,
                BracketRenderOptions::default(),
                limit,
                token,
            ),
            Err(OutputError::Interrupted)
        )
    }));

    assert!(stops(GENERATED_TREE_LATE_TRIP, &|token| {
        matches!(
            pretty_generated_model_tree_with_limit(
                parse.parse_tree.as_ref(),
                text,
                TreeRenderOptions::default(),
                limit,
                token,
            ),
            Err(OutputError::Interrupted)
        )
    }));
    assert!(!stops(GENERATED_TREE_LATE_TRIP + 1, &|token| {
        matches!(
            pretty_generated_model_tree_with_limit(
                parse.parse_tree.as_ref(),
                text,
                TreeRenderOptions::default(),
                limit,
                token,
            ),
            Err(OutputError::Interrupted)
        )
    }));

    assert!(stops(RECOVERED_BRACKETS_LATE_TRIP, &|token| {
        matches!(
            crate::pretty_recovered_syntax_brackets_with_limit(
                &recovered,
                bad,
                BracketRenderOptions::default(),
                limit,
                token,
            ),
            Err(OutputError::Interrupted)
        )
    }));
    assert!(!stops(RECOVERED_BRACKETS_LATE_TRIP + 1, &|token| {
        matches!(
            crate::pretty_recovered_syntax_brackets_with_limit(
                &recovered,
                bad,
                BracketRenderOptions::default(),
                limit,
                token,
            ),
            Err(OutputError::Interrupted)
        )
    }));
}
