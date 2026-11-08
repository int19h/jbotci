//! Renderers return [`OutputError::Interrupted`] when the flag is set.
//!
//! Bracket, tree, and morphology-bracket rendering of a real sentence finish
//! too quickly for a delayed flag to be a reliable stop. Each format is
//! rendered twice. A clear flag returns a document. A flag set before the
//! call returns the interrupted error and no document. Removing the load
//! makes the second call return the document.

use std::sync::atomic::Ordering;

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_interrupt::CancellationSource;
use jbotci_morphology::segment_words_with_modifiers;
use jbotci_syntax::parse_syntax_tree;

use crate::budget::ByteLimit;
use crate::{
    BracketRenderOptions, OutputError, TreeRenderOptions,
    pretty_generated_model_brackets_with_limit, pretty_generated_model_tree_with_limit,
    pretty_morphology_brackets_with_limit, pretty_morphology_tree_with_limit,
    pretty_recovered_morphology_brackets_with_limit, pretty_recovered_morphology_raw_with_limit,
    pretty_recovered_morphology_tree_with_limit, pretty_recovered_syntax_brackets_with_limit,
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

#[requires(true)]
#[ensures(true)]
fn assert_exit_check<T: std::fmt::Debug, E: std::fmt::Debug>(
    run: impl Fn(&jbotci_interrupt::CancellationToken) -> Result<T, E>,
    interrupted: impl Fn(&E) -> bool,
) -> T {
    let (token, seen) = jbotci_interrupt::CancellationToken::counting();
    run(&token).expect("an untripped token finishes");
    let n = seen.load(Ordering::Relaxed);
    assert!(n >= 2, "the call never loaded the token");
    let error = run(&jbotci_interrupt::CancellationToken::trip_after_checks(
        n - 1,
    ))
    .expect_err("the last load stops the call");
    assert!(interrupted(&error));
    run(&jbotci_interrupt::CancellationToken::trip_after_checks(n))
        .expect("one more clear load finishes the call")
}

#[requires(true)]
#[ensures(true)]
fn is_interrupted(error: &OutputError) -> bool {
    matches!(error, OutputError::Interrupted)
}

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
        "the recovered-bracket path must use a tree the post-walk load sees"
    );
    let limit = ByteLimit::new(2_000_000).expect("positive limit");
    let morphology_words = segment_words_with_modifiers("mi klama").expect("morphology");
    let broken = "mi @@@ do";
    let broken_words = jbotci_morphology::segment_words_with_modifiers_recovered(broken);
    assert!(
        !broken_words.errors.is_empty(),
        "recovered morphology rendering must see an error, not the plain path"
    );

    let morphology = assert_exit_check(
        |token| {
            pretty_morphology_brackets_with_limit(
                &words,
                text,
                BracketRenderOptions::default(),
                limit,
                token,
            )
        },
        is_interrupted,
    );
    assert!(!morphology.is_empty());

    let brackets = assert_exit_check(
        |token| {
            pretty_generated_model_brackets_with_limit(
                parse.parse_tree.as_ref(),
                text,
                BracketRenderOptions::default(),
                limit,
                token,
            )
        },
        is_interrupted,
    );
    assert!(!brackets.is_empty());

    let tree = assert_exit_check(
        |token| {
            pretty_generated_model_tree_with_limit(
                parse.parse_tree.as_ref(),
                text,
                TreeRenderOptions::default(),
                limit,
                token,
            )
        },
        is_interrupted,
    );
    assert!(!tree.is_empty());

    let recovered_brackets = assert_exit_check(
        |token| {
            pretty_recovered_syntax_brackets_with_limit(
                &recovered,
                bad,
                BracketRenderOptions::default(),
                limit,
                token,
            )
        },
        is_interrupted,
    );
    assert!(!recovered_brackets.is_empty());

    let morphology_tree = assert_exit_check(
        |token| {
            pretty_morphology_tree_with_limit(
                &morphology_words,
                "mi klama",
                TreeRenderOptions::default(),
                limit,
                token,
            )
        },
        is_interrupted,
    );
    assert!(!morphology_tree.is_empty());

    let recovered_morphology_brackets = assert_exit_check(
        |token| {
            pretty_recovered_morphology_brackets_with_limit(
                &broken_words,
                broken,
                BracketRenderOptions::default(),
                limit,
                token,
            )
        },
        is_interrupted,
    );
    assert!(!recovered_morphology_brackets.is_empty());

    let recovered_morphology_tree = assert_exit_check(
        |token| {
            pretty_recovered_morphology_tree_with_limit(
                &broken_words,
                broken,
                TreeRenderOptions::default(),
                limit,
                token,
            )
        },
        is_interrupted,
    );
    assert!(!recovered_morphology_tree.is_empty());

    let recovered_morphology_raw = assert_exit_check(
        |token| {
            pretty_recovered_morphology_raw_with_limit(&broken_words, broken, None, limit, token)
        },
        is_interrupted,
    );
    assert!(!recovered_morphology_raw.is_empty());
}
