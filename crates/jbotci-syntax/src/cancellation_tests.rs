//! The unspaced `lo` text the server measured, stopped by the one parser check.
//!
//! Morphology of repeated `lo` is ordinary cmavo and finishes quickly. The
//! syntax error handling of that word list is what runs for seconds. The flag
//! is set after that parse has started, and the only interruption check in
//! the parser (strict attempt and recovery trials) must observe it. The
//! outcome is [`SyntaxRecoveryParse::Interrupted`], not a syntax error.

use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};

#[allow(unused_imports)]
use bityzba::{ensures, requires};
use jbotci_interrupt::CancellationSource;

use super::{
    ParseOptions, SyntaxRecoveryParseData,
    parse_syntax_tree_with_recovery_with_source_and_options_attempt,
};

/// See the morphology cancellation test for the sizing rule. This count is the
/// syntax-error family, not the brivla family.
const LO_COPIES: usize = 700;
const CANCEL_AFTER: Duration = Duration::from_millis(200);
const MUST_STOP_WITHIN: Duration = Duration::from_secs(3);

#[test]
#[requires(true)]
#[ensures(true)]
fn cancellation_stops_unspaced_lo_and_is_not_a_syntax_error() {
    let text = "lo".repeat(LO_COPIES);
    // libtest threads start at 2 MiB. A debug build of this recovery walk uses
    // more than that between `stacker::maybe_grow` points, and the process
    // aborts before the interruption check can return. The worker stack is
    // test-only room for those frames.
    let source = CancellationSource::new();
    let token = source.token();
    let canceller = thread::spawn(move || {
        thread::sleep(CANCEL_AFTER);
        source.cancel();
    });
    let worker = thread::Builder::new()
        .name("syntax-cancellation".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            let words = jbotci_morphology::segment_words_with_modifiers(&text)
                .expect("repeated lo is a run of cmavo, not an invalid word");
            let started = Instant::now();
            let attempt = parse_syntax_tree_with_recovery_with_source_and_options_attempt(
                &words,
                &text,
                &ParseOptions::default().with_cancellation(&token),
            );
            // Drop the tree on this thread. Moving the generated syntax type
            // back to the test thread overflows the compiler's Send check.
            let interrupted = matches!(
                attempt.result.as_data(),
                SyntaxRecoveryParseData::Interrupted
            );
            (started.elapsed(), interrupted)
        })
        .expect("syntax cancellation worker");
    let (elapsed, interrupted) = worker.join().expect("syntax cancellation worker");
    canceller.join().expect("canceller");
    assert!(
        elapsed < MUST_STOP_WITHIN,
        "syntax kept running for {elapsed:?} after the flag was set; elapsed includes the {CANCEL_AFTER:?} delay"
    );
    assert!(
        interrupted,
        "a stopped parse must not be reported as a recovered or failed syntax tree"
    );
}

/// Copies of unspaced `lo` in the slow-loop test.
///
/// Release time of the recovered parse, after morphology, with a clear
/// token is 76.52 s. The test segments first, then trips the parse at
/// check 1,000. It does not repeat the clear-token run.
const SLOW_LO_COPIES: usize = 320;
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
fn a_late_trip_stops_after_a_successful_parse() {
    let words = jbotci_morphology::segment_words_with_modifiers("mi klama").expect("morphology");
    assert_exit_check(
        |token| {
            super::parse_syntax_tree_with_options(
                &words,
                &ParseOptions::default().with_cancellation(token),
            )
        },
        |error| matches!(error, super::SyntaxError::Interrupted),
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_late_trip_stops_a_recovered_parse_that_has_errors() {
    let bad = "mi klama le ???";
    let words = jbotci_morphology::segment_words_with_modifiers(bad).expect("bad morphology");
    let finished = assert_exit_check(
        |token| {
            super::parse_syntax_tree_recovered_with_source_and_options(
                &words,
                bad,
                &ParseOptions::default().with_cancellation(token),
            )
        },
        |error| matches!(error, super::SyntaxError::Interrupted),
    );
    assert!(
        !finished.errors.is_empty(),
        "this path is the error path, where the second post-parse load runs"
    );
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_mid_loop_trip_stops_unspaced_lo_quickly() {
    let text = "lo".repeat(SLOW_LO_COPIES);
    let words = jbotci_morphology::segment_words_with_modifiers(&text)
        .expect("repeated lo is a run of cmavo, not an invalid word");
    let worker = thread::Builder::new()
        .name("syntax-slow-trip".to_owned())
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            let started = Instant::now();
            let token = jbotci_interrupt::CancellationToken::trip_after_checks(SLOW_LOOP_TRIP);
            let attempt = parse_syntax_tree_with_recovery_with_source_and_options_attempt(
                &words,
                &text,
                &ParseOptions::default().with_cancellation(&token),
            );
            let interrupted = matches!(
                attempt.result.as_data(),
                SyntaxRecoveryParseData::Interrupted
            );
            (started.elapsed(), interrupted)
        })
        .expect("syntax slow-trip worker");
    let (elapsed, interrupted) = worker.join().expect("syntax slow-trip worker");
    assert!(
        elapsed < SLOW_LOOP_BOUND,
        "syntax kept running for {elapsed:?}; a missing loop check parses the whole text"
    );
    assert!(
        interrupted,
        "the early trip must be an interrupt, not a recovered or failed tree"
    );
}
