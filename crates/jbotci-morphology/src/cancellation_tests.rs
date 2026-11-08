//! The long brivla candidate the server measured, stopped by the shared flag.
//!
//! A few hundred copies of `zbasu` closed by `y` are one token. On a release
//! build that token keeps the memoized recognizers and the word-start search
//! busy for seconds. The flag is set only after that search has started, so
//! the test fails when those loops no longer load it.

use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_interrupt::CancellationSource;

use super::{
    MorphologyError, MorphologyOptions,
    segment_words_with_modifiers_recovered_with_options_and_source_id,
};

/// Large enough that an uninterrupted release run takes seconds, small enough
/// that a missing check fails this test in a few seconds rather than minutes.
const ZBASU_COPIES: usize = 220;
const CANCEL_AFTER: Duration = Duration::from_millis(200);
const MUST_STOP_WITHIN: Duration = Duration::from_secs(3);

#[test]
#[requires(true)]
#[ensures(true)]
fn cancellation_stops_a_long_brivla_and_is_not_an_invalid_word() {
    let input = format!("{}y", "zbasu".repeat(ZBASU_COPIES));
    let source = CancellationSource::new();
    let token = source.token();
    let canceller = thread::spawn(move || {
        thread::sleep(CANCEL_AFTER);
        source.cancel();
    });
    let started = Instant::now();
    let options = MorphologyOptions::default().with_cancellation(&token);
    let segmentation =
        segment_words_with_modifiers_recovered_with_options_and_source_id(&input, &options, None);
    let elapsed = started.elapsed();
    canceller.join().expect("canceller");
    assert!(
        elapsed < MUST_STOP_WITHIN,
        "morphology kept running for {elapsed:?} after the flag was set; elapsed includes the {CANCEL_AFTER:?} delay"
    );
    assert!(
        segmentation.was_interrupted(),
        "a stopped segmentation must not look like a finished invalid word"
    );
    assert!(
        segmentation.words.is_empty(),
        "a stopped segmentation must not keep a partial word list"
    );
    assert!(
        segmentation
            .errors
            .iter()
            .all(MorphologyError::is_interrupted)
    );
}

/// Copies of `zbasu` before the closing `y` in the slow-loop test.
///
/// Release time of strict segmentation of this input with a clear token
/// is 87.67 s. The test trips at check 1,000, inside the word-start
/// search, and does not repeat the clear-token run.
const SLOW_ZBASU_COPIES: usize = 700;
const SLOW_LOOP_TRIP: usize = 1_000;
const SLOW_LOOP_BOUND: Duration = Duration::from_secs(10);

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

#[test]
#[requires(true)]
#[ensures(true)]
fn a_late_trip_stops_strict_segmentation_after_the_last_word() {
    assert_exit_check(
        |token| {
            let options = MorphologyOptions::default().with_cancellation(token);
            super::segment_words_with_modifiers_with_options_and_source_id(
                "mi klama", &options, None,
            )
        },
        MorphologyError::is_interrupted,
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_late_trip_stops_display_segmentation_after_the_last_word() {
    assert_exit_check(
        |token| {
            let options = MorphologyOptions::default().with_cancellation(token);
            super::segment_words_for_display_with_options_and_source_id("mi klama", &options, None)
        },
        MorphologyError::is_interrupted,
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_mid_loop_trip_stops_a_long_brivla_quickly() {
    let input = format!("{}y", "zbasu".repeat(SLOW_ZBASU_COPIES));
    let started = Instant::now();
    let token = jbotci_interrupt::CancellationToken::trip_after_checks(SLOW_LOOP_TRIP);
    let options = MorphologyOptions::default().with_cancellation(&token);
    let segmentation =
        super::segment_words_with_modifiers_with_options_and_source_id(&input, &options, None);
    let elapsed = started.elapsed();
    assert!(
        elapsed < SLOW_LOOP_BOUND,
        "morphology kept running for {elapsed:?}; a missing loop check runs the whole word"
    );
    assert!(
        segmentation.is_err_and(|error| error.is_interrupted()),
        "the early trip must be an interrupt, not an invalid word"
    );
}
