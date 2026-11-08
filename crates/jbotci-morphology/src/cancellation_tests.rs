//! The long brivla candidate the server measured, stopped by the shared flag.
//!
//! A few hundred copies of `zbasu` closed by `y` are one token. On a release
//! build that token keeps the memoized recognizers and the word-start search
//! busy for seconds. The flag is set only after that search has started, so
//! the test fails when those loops no longer load it.

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
