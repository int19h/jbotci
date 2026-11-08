//! The unspaced `lo` text the server measured, stopped by the one parser check.
//!
//! Morphology of repeated `lo` is ordinary cmavo and finishes quickly. The
//! syntax error handling of that word list is what runs for seconds. The flag
//! is set after that parse has started, and the only interruption check in
//! the parser (strict attempt and recovery trials) must observe it. The
//! outcome is [`SyntaxRecoveryParse::Interrupted`], not a syntax error.

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

/// Clear loads of a successful parse of "mi klama" before the load that aborts.
/// Release measurement: 2318 clear loads, trip 2317. The post-parse `parse_stop`
/// is on this path. A flag set during the parse returns earlier and does not
/// show that this load is still there.
const STRICT_PARSE_LATE_TRIP: usize = 2317;
/// Recovered parse of "mi klama le ???": 9026 clear loads, trip 9025. The
/// sentence has errors, so the load after the failed strict attempt and the
/// load after the tracked attempt both run.
const RECOVERED_PARSE_LATE_TRIP: usize = 9025;

#[test]
#[requires(true)]
#[ensures(true)]
fn a_late_trip_stops_after_a_successful_parse() {
    let words = jbotci_morphology::segment_words_with_modifiers("mi klama").expect("morphology");
    let run = |trip: usize| {
        let token = jbotci_interrupt::CancellationToken::trip_after_checks(trip);
        super::parse_syntax_tree_with_options(
            &words,
            &ParseOptions::default().with_cancellation(&token),
        )
    };
    let error = run(STRICT_PARSE_LATE_TRIP).expect_err("the late trip stops the parse");
    assert!(matches!(error, super::SyntaxError::Interrupted));
    run(STRICT_PARSE_LATE_TRIP + 1).expect("the next index finishes the parse");
}

#[test]
#[requires(true)]
#[ensures(true)]
fn a_late_trip_stops_a_recovered_parse_that_has_errors() {
    let bad = "mi klama le ???";
    let words = jbotci_morphology::segment_words_with_modifiers(bad).expect("bad morphology");
    let run = |trip: usize| {
        let token = jbotci_interrupt::CancellationToken::trip_after_checks(trip);
        super::parse_syntax_tree_recovered_with_source_and_options(
            &words,
            bad,
            &ParseOptions::default().with_cancellation(&token),
        )
    };
    let error = run(RECOVERED_PARSE_LATE_TRIP).expect_err("the late trip stops recovery");
    assert!(matches!(error, super::SyntaxError::Interrupted));
    let finished = run(RECOVERED_PARSE_LATE_TRIP + 1).expect("the next index finishes recovery");
    assert!(
        !finished.errors.is_empty(),
        "this trip is the error path, where the second post-parse load runs"
    );
}
