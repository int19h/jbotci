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
