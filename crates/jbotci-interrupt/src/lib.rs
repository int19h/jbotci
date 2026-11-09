//! Cooperative cancellation shared by morphology, syntax, semantics, and rendering.
//!
//! The value those crates load is an atomic flag. It does not read a clock:
//! `std::time::Instant` panics on `wasm32-unknown-unknown`. The owner sets the
//! flag when a deadline passes. A check in a hot loop is one acquire load.
//! Callers that have no deadline hold [`CancellationToken::never`], whose flag
//! stays clear for the life of the process.

use std::fmt;
#[cfg(feature = "cancellation_test_hook")]
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};

#[allow(unused_imports)]
use bityzba::{ensures, invariant, requires};

/// Flag shared by one computation and the owner that may stop it.
///
/// Equality is pointer equality. Two tokens are the same admission only when
/// they share one flag. Every [`CancellationToken::never`] value shares one
/// static flag, so default options stay equal.
#[invariant(true)]
#[derive(Debug, Clone)]
pub struct CancellationToken {
    flag: Arc<AtomicBool>,
    /// After this many clear loads, store the flag. The load that stores it
    /// still returns clear, so the following load is the one that stops.
    /// Absent on a production token. The field itself is absent unless the
    /// test feature is on, so a production load does not branch.
    #[cfg(feature = "cancellation_test_hook")]
    trip_after: Option<Arc<AtomicUsize>>,
    /// Counts clear loads. Tests read it to learn which load is last.
    #[cfg(feature = "cancellation_test_hook")]
    seen: Option<Arc<AtomicUsize>>,
}

impl PartialEq for CancellationToken {
    #[requires(true)]
    #[ensures(true)]
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.flag, &other.flag)
    }
}

impl Eq for CancellationToken {}

impl Default for CancellationToken {
    #[requires(true)]
    #[ensures(!ret.is_cancelled())]
    fn default() -> Self {
        Self::never()
    }
}

impl CancellationToken {
    /// A token that no owner can set.
    ///
    /// The CLI, the browser compute worker, and tests pass this value. Their
    /// results stay the results of an uninterrupted computation.
    #[requires(true)]
    #[ensures(!ret.is_cancelled())]
    pub fn never() -> Self {
        static FLAG: LazyLock<Arc<AtomicBool>> = LazyLock::new(|| Arc::new(AtomicBool::new(false)));
        Self {
            flag: Arc::clone(&FLAG),
            #[cfg(feature = "cancellation_test_hook")]
            trip_after: None,
            #[cfg(feature = "cancellation_test_hook")]
            seen: None,
        }
    }

    /// A token whose Nth clear load stores the flag after returning clear.
    ///
    /// The next load then reports cancellation. A test picks N so that load is
    /// the checkpoint under test. Removing that load leaves the flag clear for
    /// the return, and the test stops seeing an interrupted result.
    #[cfg(feature = "cancellation_test_hook")]
    #[requires(checks > 0)]
    #[ensures(!ret.is_cancelled())]
    pub fn trip_after_checks(checks: usize) -> Self {
        let mut token = CancellationSource::new().token();
        token.trip_after = Some(Arc::new(AtomicUsize::new(checks)));
        token
    }

    /// A clear token and a counter of its clear `is_cancelled` loads.
    #[cfg(feature = "cancellation_test_hook")]
    #[requires(true)]
    #[ensures(!ret.0.is_cancelled())]
    pub fn counting() -> (Self, Arc<AtomicUsize>) {
        let seen = Arc::new(AtomicUsize::new(0));
        let mut token = CancellationSource::new().token();
        token.seen = Some(Arc::clone(&seen));
        (token, seen)
    }

    /// One acquire load of the shared flag.
    ///
    /// Hot loops call this and nothing else. The load is acquire so it sees a
    /// release store from [`CancellationSource::cancel`]. With the test
    /// feature off, this function is that load and nothing more.
    #[requires(true)]
    #[ensures(true)]
    pub fn is_cancelled(&self) -> bool {
        let cancelled = self.flag.load(Ordering::Acquire);
        #[cfg(feature = "cancellation_test_hook")]
        if !cancelled {
            if let Some(seen) = &self.seen {
                seen.fetch_add(1, Ordering::Relaxed);
            }
            if let Some(remaining) = &self.trip_after {
                let previous = remaining.fetch_sub(1, Ordering::Relaxed);
                if previous == 1 {
                    self.flag.store(true, Ordering::Release);
                }
            }
        }
        cancelled
    }

    /// The interrupted outcome when the flag is set, otherwise success.
    ///
    /// A hot loop that only needs the load calls [`Self::is_cancelled`]. This
    /// helper is for a checkpoint that leaves the function.
    #[requires(true)]
    #[ensures(ret.is_ok() || self.is_cancelled())]
    pub fn check(&self) -> Result<(), Interrupted> {
        if self.is_cancelled() {
            Err(Interrupted)
        } else {
            Ok(())
        }
    }
}

/// Owner of a cancellation flag.
///
/// The server creates one source for each admitted computation and sets it
/// when that computation's deadline passes. Clones share the flag. Dropping a
/// source does not set the flag: a finished owner must not cancel a token that
/// a caller still holds from an earlier clone that outlived this value.
#[invariant(true)]
#[derive(Debug, Clone)]
pub struct CancellationSource {
    flag: Arc<AtomicBool>,
}

impl CancellationSource {
    #[requires(true)]
    #[ensures(!ret.is_cancelled())]
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    #[requires(true)]
    #[ensures(!ret.is_cancelled() || self.is_cancelled())]
    pub fn token(&self) -> CancellationToken {
        CancellationToken {
            flag: Arc::clone(&self.flag),
            #[cfg(feature = "cancellation_test_hook")]
            trip_after: None,
            #[cfg(feature = "cancellation_test_hook")]
            seen: None,
        }
    }

    #[requires(true)]
    #[ensures(self.is_cancelled())]
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Release);
    }

    #[requires(true)]
    #[ensures(true)]
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }
}

impl Default for CancellationSource {
    #[requires(true)]
    #[ensures(!ret.is_cancelled())]
    fn default() -> Self {
        Self::new()
    }
}

/// A computation stopped because its cancellation flag was set.
///
/// This is not a morphology error and not a syntax error. Callers match it by
/// type. Layers that were cancelled return this value instead of a partial
/// word, parse, or rendering.
#[invariant(true)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interrupted;

impl fmt::Display for Interrupted {
    #[requires(true)]
    #[ensures(true)]
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("computation interrupted")
    }
}

impl std::error::Error for Interrupted {}

/// Prove that the last clear load turns a late flag into an interrupt.
///
/// `counting` records N on a clear run. `trip_after_checks(N - 1)` must
/// return the interrupted error. `trip_after_checks(N)` must succeed.
/// N includes the constructor load, so a call that never loads the token
/// fails the `N >= 2` assert.
#[cfg(feature = "cancellation_test_hook")]
#[requires(true)]
#[ensures(true)]
pub fn assert_exit_check<T: fmt::Debug, E: fmt::Debug>(
    run: impl Fn(&CancellationToken) -> Result<T, E>,
    interrupted: impl Fn(&E) -> bool,
) -> T {
    let (token, seen) = CancellationToken::counting();
    run(&token).expect("an untripped token finishes");
    let n = seen.load(Ordering::Relaxed);
    assert!(n >= 2, "the call never loaded the token");
    let error = run(&CancellationToken::trip_after_checks(n - 1))
        .expect_err("the last load stops the call");
    assert!(interrupted(&error));
    run(&CancellationToken::trip_after_checks(n)).expect("one more clear load finishes the call")
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use super::*;

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn never_stays_clear_and_a_source_sets_only_its_own_flag() {
        let never = CancellationToken::never();
        assert!(!never.is_cancelled());
        let source = CancellationSource::new();
        let token = source.token();
        assert!(!token.is_cancelled());
        assert_ne!(token, never);
        source.cancel();
        assert!(token.is_cancelled());
        assert!(source.token().is_cancelled());
        assert!(!CancellationToken::never().is_cancelled());
        assert_eq!(CancellationToken::never(), CancellationToken::default());
    }

    #[test]
    #[requires(true)]
    #[ensures(true)]
    fn a_flag_set_on_another_thread_is_visible_to_the_token() {
        let source = CancellationSource::new();
        let token = source.token();
        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            source.cancel();
        });
        let started = std::time::Instant::now();
        while !token.is_cancelled() {
            assert!(
                started.elapsed() < Duration::from_secs(2),
                "the token did not observe the other thread"
            );
            thread::sleep(Duration::from_millis(5));
        }
        handle.join().expect("canceller thread");
    }
}
